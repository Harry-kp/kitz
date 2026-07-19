//! MSK client wrapper. Everything that talks to Kafka lives here so the UI
//! never sees an rdkafka type. Auth is MSK IAM (SASL OAUTHBEARER) - the whole
//! reason this tool exists.

use std::error::Error;
use std::fs::File;
use std::io::Write as _;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use aws_msk_iam_sasl_signer::generate_auth_token;
use aws_types::region::Region;
use rdkafka::admin::{AdminClient, AdminOptions, NewPartitions, NewTopic, TopicReplication};
use rdkafka::client::OAuthToken;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{BaseConsumer, Consumer, ConsumerContext};
use rdkafka::{ClientContext, Message, Offset, TopicPartitionList};

use crate::config::EnvProfile;

const TIMEOUT: Duration = Duration::from_secs(15);

// ── Plain data returned to the UI ────────────────────────────────────────

/// Cheap per-topic metadata from the single cluster-wide fetch. No watermarks -
/// those are the expensive per-partition round-trips, loaded on demand. Sent to
/// the UI so it can build topic lists + detail views with zero latency.
#[derive(Clone)]
pub struct PartMeta {
    pub id: i32,
    pub leader: i32,
    pub replicas: usize,
    pub isr: usize,
}

#[derive(Clone)]
pub struct TopicMeta {
    pub name: String,
    pub partitions: Vec<PartMeta>,
}

pub struct PartitionInfo {
    pub id: i32,
    /// Leader broker id - kept for a future leader-skew view (not shown in the
    /// compact narrow Detail table).
    #[allow(dead_code)]
    pub leader: i32,
    pub replicas: usize,
    pub isr: usize,
    /// -1 until watermarks are loaded on demand.
    pub low: i64,
    pub high: i64,
}

pub struct TopicDetail {
    pub name: String,
    pub partitions: Vec<PartitionInfo>,
    /// False until `load_watermarks` fills low/high + event counts.
    pub watermarks_loaded: bool,
}

impl TopicDetail {
    /// Rough message count = sum of (high - low) across partitions.
    pub fn total_messages(&self) -> i64 {
        self.partitions.iter().map(|p| p.high - p.low).sum()
    }
}

pub struct GroupSummary {
    pub name: String,
    pub state: String,
    pub members: usize,
    pub protocol: String,
    /// Topics this group's members are subscribed to (parsed from member
    /// metadata - no extra network calls).
    pub topics: Vec<String>,
}

pub struct EventRecord {
    pub partition: i32,
    pub offset: i64,
    pub key: String,
    pub payload: String,
    /// Kept for a future timestamp column in the peek view.
    #[allow(dead_code)]
    pub timestamp: Option<i64>,
}

// ── Auth + logging context ──────────────────────────────────────────────

#[derive(Clone)]
pub struct MskContext {
    region: String,
    /// Where librdkafka log lines go. `Some(file)` in the TUI (so logs can't
    /// corrupt the screen); `None` in `doctor` mode (straight to stderr).
    log_file: Option<Arc<Mutex<File>>>,
}

impl ClientContext for MskContext {
    const ENABLE_REFRESH_OAUTH_TOKEN: bool = true;

    fn generate_oauth_token(&self, _config: Option<&str>) -> Result<OAuthToken, Box<dyn Error>> {
        // Callback runs on a librdkafka thread; spin a tiny runtime to await
        // the async signer.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (token, expiry_ms) =
            rt.block_on(generate_auth_token(Region::new(self.region.clone())))?;
        Ok(OAuthToken {
            token,
            principal_name: String::new(),
            lifetime_ms: expiry_ms,
        })
    }

    fn log(&self, level: RDKafkaLogLevel, fact: &str, log_message: &str) {
        let line = format!("[rdkafka {level:?}] {fact}: {log_message}");
        match &self.log_file {
            Some(f) => {
                if let Ok(mut g) = f.lock() {
                    let _ = writeln!(g, "{line}");
                }
            }
            None => eprintln!("{line}"),
        }
    }
}

impl ConsumerContext for MskContext {}

// ── Client ──────────────────────────────────────────────────────────────

pub struct KafkaClient {
    profile: EnvProfile,
    ctx: MskContext,
    consumer: BaseConsumer<MskContext>,
    admin: AdminClient<MskContext>,
    rt: tokio::runtime::Runtime,
    /// Cluster-wide topic metadata, fetched once at connect and after admin
    /// ops. Reading it is free (no network) - that's what keeps navigation
    /// instant.
    meta: Vec<TopicMeta>,
}

impl KafkaClient {
    pub fn connect(profile: &EnvProfile) -> Result<Self> {
        if let Some(p) = &profile.aws_profile {
            std::env::set_var("AWS_PROFILE", p);
        }
        // Debug logging to a file is opt-in via KITZ_DEBUG=1 so a normal run
        // stays quiet; the file always exists so warnings/errors are captured.
        let debug = std::env::var("KITZ_DEBUG").is_ok();
        let ctx = MskContext {
            region: profile.region.clone(),
            log_file: open_log_file(),
        };

        let consumer: BaseConsumer<MskContext> = base_config(profile, debug)
            .create_with_context(ctx.clone())
            .context("creating consumer")?;
        // Poll once so the OAuth callback fires and the connection warms up.
        consumer.poll(Duration::from_secs(5));

        let admin: AdminClient<MskContext> = base_config(profile, debug)
            .create_with_context(ctx.clone())
            .context("creating admin client")?;

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .context("building admin runtime")?;

        let meta = fetch_meta(&consumer)?;

        Ok(Self {
            profile: profile.clone(),
            ctx,
            consumer,
            admin,
            rt,
            meta,
        })
    }

    /// Re-fetch cluster metadata (after create/delete/add-partitions).
    pub fn reload_meta(&mut self) -> Result<()> {
        self.meta = fetch_meta(&self.consumer)?;
        Ok(())
    }

    /// Cached topic/partition metadata - free (no network). The UI builds its
    /// topic list and detail views from this.
    pub fn metadata(&self) -> Vec<TopicMeta> {
        self.meta.clone()
    }

    /// The expensive part: one `fetch_watermarks` round-trip per partition.
    /// Called only on demand (the `w` key), never during navigation.
    pub fn watermarks(&self, name: &str) -> Result<Vec<(i32, i64, i64)>> {
        let t = self
            .meta
            .iter()
            .find(|t| t.name == name)
            .context("unknown topic")?;
        let mut out = Vec::with_capacity(t.partitions.len());
        for p in &t.partitions {
            let (low, high) = self
                .consumer
                .fetch_watermarks(name, p.id, TIMEOUT)
                .unwrap_or((0, 0));
            out.push((p.id, low, high));
        }
        Ok(out)
    }

    pub fn consumer_groups(&self) -> Result<Vec<GroupSummary>> {
        let list = self
            .consumer
            .fetch_group_list(None, TIMEOUT)
            .context("fetching consumer groups")?;
        let mut groups: Vec<GroupSummary> = list
            .groups()
            .iter()
            .map(|g| {
                // rdkafka 0.36 UB: an Empty/Dead group has a null members
                // pointer, and `members()` does `from_raw_parts(null, 0)`,
                // which aborts on modern Rust. Only read members for states
                // that actually have them.
                let has_members = matches!(
                    g.state(),
                    "Stable"
                        | "PreparingRebalance"
                        | "CompletingRebalance"
                        | "Assigning"
                        | "Reconciling"
                );
                let (members, topics) = if has_members {
                    let ms = g.members();
                    let mut topics = Vec::new();
                    let mut add = |t: String| {
                        if !t.is_empty() && !topics.contains(&t) {
                            topics.push(t);
                        }
                    };
                    for m in ms {
                        // Subscription (what they asked for) + assignment (what
                        // they actually hold) - union covers more real cases.
                        if let Some(meta) = m.metadata() {
                            parse_topic_strings(meta, false)
                                .into_iter()
                                .for_each(&mut add);
                        }
                        if let Some(asg) = m.assignment() {
                            parse_topic_strings(asg, true)
                                .into_iter()
                                .for_each(&mut add);
                        }
                    }
                    topics.sort();
                    (ms.len(), topics)
                } else {
                    (0, Vec::new())
                };
                GroupSummary {
                    name: g.name().to_string(),
                    state: g.state().to_string(),
                    protocol: g.protocol().to_string(),
                    members,
                    topics,
                }
            })
            .collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(groups)
    }

    /// Topic configuration (retention, cleanup policy, etc.) via describe_configs.
    /// Returns a curated set of the keys that matter day-to-day.
    pub fn topic_config(&self, name: &str) -> Result<Vec<(String, String)>> {
        use rdkafka::admin::ResourceSpecifier;
        let res = self
            .rt
            .block_on(
                self.admin
                    .describe_configs([&ResourceSpecifier::Topic(name)], &AdminOptions::new()),
            )
            .context("describe_configs call failed")?;
        let resource = res
            .into_iter()
            .next()
            .context("no config returned")?
            .map_err(|e| anyhow::anyhow!("describe_configs error: {e}"))?;

        const KEYS: &[&str] = &[
            "cleanup.policy",
            "retention.ms",
            "retention.bytes",
            "max.message.bytes",
            "min.insync.replicas",
            "segment.ms",
            "compression.type",
        ];
        let map = resource.entry_map();
        let out = KEYS
            .iter()
            .map(|k| {
                let v = map
                    .get(*k)
                    .and_then(|e| e.value.clone())
                    .unwrap_or_else(|| "-".to_string());
                ((*k).to_string(), v)
            })
            .collect();
        Ok(out)
    }

    pub fn create_topic(&self, name: &str, partitions: i32, replication: i32) -> Result<()> {
        let new = NewTopic::new(name, partitions, TopicReplication::Fixed(replication));
        let res = self
            .rt
            .block_on(self.admin.create_topics(&[new], &AdminOptions::new()))
            .context("create_topics call failed")?;
        check_results(res)
    }

    pub fn delete_topic(&self, name: &str) -> Result<()> {
        let res = self
            .rt
            .block_on(self.admin.delete_topics(&[name], &AdminOptions::new()))
            .context("delete_topics call failed")?;
        check_results(res)
    }

    pub fn add_partitions(&self, name: &str, total: usize) -> Result<()> {
        let np = NewPartitions::new(name, total);
        let res = self
            .rt
            .block_on(self.admin.create_partitions(&[np], &AdminOptions::new()))
            .context("create_partitions call failed")?;
        check_results(res)
    }

    pub fn delete_group(&self, name: &str) -> Result<()> {
        let res = self
            .rt
            .block_on(self.admin.delete_groups(&[name], &AdminOptions::new()))
            .context("delete_groups call failed")?;
        check_results(res)
    }

    /// Peek the last `limit` events across a topic's partitions.
    pub fn peek(&self, topic: &str, limit: usize) -> Result<Vec<EventRecord>> {
        let debug = std::env::var("KITZ_DEBUG").is_ok();
        let peeker: BaseConsumer<MskContext> = base_config(&self.profile, debug)
            .set("group.id", "kitz-peek")
            .set("enable.auto.commit", "false")
            .create_with_context(self.ctx.clone())
            .context("creating peek consumer")?;
        peeker.poll(Duration::from_secs(5));

        let md = self.consumer.fetch_metadata(Some(topic), TIMEOUT)?;
        let topic_md = md.topics().first().context("topic not found")?;
        let per_partition = (limit / topic_md.partitions().len().max(1)).max(1) as i64;

        let mut tpl = TopicPartitionList::new();
        for p in topic_md.partitions() {
            let (low, high) = self.consumer.fetch_watermarks(topic, p.id(), TIMEOUT)?;
            let start = (high - per_partition).max(low);
            tpl.add_partition_offset(topic, p.id(), Offset::Offset(start))?;
        }
        peeker.assign(&tpl)?;

        let mut out = Vec::with_capacity(limit);
        let mut empty_polls = 0;
        while out.len() < limit && empty_polls < 5 {
            match peeker.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    empty_polls = 0;
                    out.push(EventRecord {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg
                            .key()
                            .map(|k| String::from_utf8_lossy(k).into_owned())
                            .unwrap_or_default(),
                        payload: msg
                            .payload()
                            .map(|p| String::from_utf8_lossy(p).into_owned())
                            .unwrap_or_default(),
                        timestamp: msg.timestamp().to_millis(),
                    });
                }
                Some(Err(e)) => return Err(anyhow::anyhow!("peek error: {e}")),
                None => empty_polls += 1,
            }
        }
        out.sort_by(|a, b| a.partition.cmp(&b.partition).then(a.offset.cmp(&b.offset)));
        Ok(out)
    }
}

/// Parse topic names out of a Kafka consumer-protocol blob.
/// Both share: int16 version · int32 topic-count · [int16 len + utf8 …].
/// Assignment additionally has an int32 partition array after each topic
/// (`with_partitions = true`), which we skip.
fn parse_topic_strings(bytes: &[u8], with_partitions: bool) -> Vec<String> {
    let mut out = Vec::new();
    if bytes.len() < 6 {
        return out;
    }
    let rd_i32 = |b: &[u8], i: usize| i32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let mut i = 2; // skip version
    let count = rd_i32(bytes, i);
    i += 4;
    if !(0..=100_000).contains(&count) {
        return out;
    }
    for _ in 0..count {
        if i + 2 > bytes.len() {
            break;
        }
        let len = i16::from_be_bytes([bytes[i], bytes[i + 1]]);
        i += 2;
        if len < 0 {
            continue;
        }
        let len = len as usize;
        if i + len > bytes.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&bytes[i..i + len]) {
            out.push(s.to_string());
        }
        i += len;
        if with_partitions {
            if i + 4 > bytes.len() {
                break;
            }
            let pcount = rd_i32(bytes, i);
            i += 4;
            if pcount < 0 {
                break;
            }
            i += (pcount as usize) * 4; // skip partition ids
        }
    }
    out
}

/// One cluster-wide metadata fetch → owned, Send-safe topic/partition structs.
fn fetch_meta(consumer: &BaseConsumer<MskContext>) -> Result<Vec<TopicMeta>> {
    let md = consumer
        .fetch_metadata(None, TIMEOUT)
        .context("fetching metadata")?;
    let mut topics: Vec<TopicMeta> = md
        .topics()
        .iter()
        .filter(|t| !t.name().starts_with("__")) // hide internal topics
        .map(|t| {
            let mut partitions: Vec<PartMeta> = t
                .partitions()
                .iter()
                .map(|p| PartMeta {
                    id: p.id(),
                    leader: p.leader(),
                    replicas: p.replicas().len(),
                    isr: p.isr().len(),
                })
                .collect();
            partitions.sort_by_key(|p| p.id);
            TopicMeta {
                name: t.name().to_string(),
                partitions,
            }
        })
        .collect();
    topics.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(topics)
}

/// Client config for the env's declared wire protocol.
fn base_config(profile: &EnvProfile, debug: bool) -> ClientConfig {
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &profile.bootstrap);

    match profile.auth.as_str() {
        "plaintext" => {
            cfg.set("security.protocol", "PLAINTEXT");
        }
        "tls" | "ssl" => {
            cfg.set("security.protocol", "SSL");
            if let Some(ca) = ca_bundle() {
                cfg.set("ssl.ca.location", ca);
            }
        }
        // "iam" (default): SASL_SSL + MSK IAM via the OAUTHBEARER token callback.
        _ => {
            cfg.set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", "OAUTHBEARER");
            // librdkafka with vendored OpenSSL on macOS often can't find the
            // system CA store, which shows up as BrokerTransportFailure.
            if let Some(ca) = ca_bundle() {
                cfg.set("ssl.ca.location", ca);
            }
        }
    }

    if debug {
        cfg.set("debug", "broker,security,protocol,fetch");
    }
    cfg
}

/// First existing CA bundle on this machine, if any.
fn ca_bundle() -> Option<String> {
    const CANDIDATES: &[&str] = &[
        "/etc/ssl/cert.pem",                    // macOS system bundle
        "/opt/homebrew/etc/openssl@3/cert.pem", // brew openssl (arm)
        "/usr/local/etc/openssl@3/cert.pem",    // brew openssl (intel)
        "/etc/ssl/certs/ca-certificates.crt",   // debian/ubuntu
        "/etc/pki/tls/certs/ca-bundle.crt",     // rhel/fedora
    ];
    CANDIDATES
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| (*s).to_string())
}

fn log_path() -> Option<PathBuf> {
    let dir = dirs::cache_dir()?.join("kitz");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("mskui.log"))
}

fn open_log_file() -> Option<Arc<Mutex<File>>> {
    let path = log_path()?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
        .map(|f| Arc::new(Mutex::new(f)))
}

fn check_results<T>(results: Vec<Result<T, (T, rdkafka::types::RDKafkaErrorCode)>>) -> Result<()> {
    for r in results {
        if let Err((_, code)) = r {
            return Err(anyhow::anyhow!("kafka admin error: {code:?}"));
        }
    }
    Ok(())
}

// ── Doctor: layer-by-layer connectivity diagnosis ─────────────────────────

/// Prints a step-by-step diagnosis to stdout/stderr (no TUI). Isolates the
/// three failure layers: TCP reachability, IAM token generation, and the full
/// librdkafka SASL_SSL handshake with verbose debug logs.
pub fn doctor(profile: &EnvProfile) {
    println!("mskui doctor");
    println!("  env       : {}", profile.name);
    println!("  auth      : {}", profile.auth);
    println!("  region    : {}", profile.region);
    println!("  bootstrap : {}", profile.bootstrap);
    println!("  aws_profile: {:?}", profile.aws_profile);
    println!("  ca bundle : {:?}", ca_bundle());
    if let Some(p) = &profile.aws_profile {
        std::env::set_var("AWS_PROFILE", p);
    }

    println!("\n[1/3] TCP reachability (are the brokers routable from here?)");
    for hostport in profile.bootstrap.split(',') {
        let hp = hostport.trim();
        print!("      {hp} … ");
        let _ = std::io::stdout().flush();
        match tcp_check(hp) {
            Ok(ms) => println!("OK ({ms} ms)"),
            Err(e) => println!("FAIL: {e}"),
        }
    }

    println!("\n[2/3] AWS IAM token generation (are your ~/.aws creds usable?)");
    if profile.auth == "iam" {
        print!("      generate_auth_token({}) … ", profile.region);
        let _ = std::io::stdout().flush();
        match gen_token(&profile.region) {
            Ok(len) => println!("OK ({len} byte token)"),
            Err(e) => println!("FAIL: {e}"),
        }
    } else {
        println!("      skipped (auth = {}, not IAM)", profile.auth);
    }

    println!("\n[3/3] Full SASL_SSL handshake + metadata (verbose librdkafka log below)");
    let ctx = MskContext {
        region: profile.region.clone(),
        log_file: None, // → stderr, so you see the handshake live
    };
    let consumer: BaseConsumer<MskContext> = match base_config(profile, true)
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(ctx)
    {
        Ok(c) => c,
        Err(e) => {
            println!("      client create FAIL: {e}");
            return;
        }
    };
    consumer.poll(Duration::from_secs(3));
    match consumer.fetch_metadata(None, Duration::from_secs(15)) {
        Ok(md) => println!("\n  ✓ metadata OK - {} topics", md.topics().len()),
        Err(e) => println!("\n  ✗ metadata FAIL: {e}"),
    }
}

fn tcp_check(hostport: &str) -> std::result::Result<u128, String> {
    let (host, port) = hostport
        .rsplit_once(':')
        .ok_or_else(|| "no :port in broker string".to_string())?;
    let port: u16 = port.parse().map_err(|_| format!("bad port: {port}"))?;
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolve failed: {e}"))?
        .next()
        .ok_or_else(|| "DNS returned no addresses".to_string())?;
    let start = std::time::Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_secs(4)).map_err(|e| format!("{e}"))?;
    Ok(start.elapsed().as_millis())
}

fn gen_token(region: &str) -> std::result::Result<usize, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    rt.block_on(generate_auth_token(Region::new(region.to_string())))
        .map(|(t, _)| t.len())
        .map_err(|e| format!("{e}"))
}
