//! Kafka worker thread. Owns the `KafkaClient` and runs *every* blocking Kafka
//! call here, off the UI thread. The UI sends [`Cmd`]s and receives [`Evt`]s
//! through channels, so rendering never blocks — no matter how slow the
//! cluster is.
//!
//! rdkafka's client types aren't `Sync`, but they never leave this thread:
//! only plain data (`Cmd`/`Evt`) crosses the channels.

use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::EnvProfile;
use crate::kafka::{EventRecord, GroupSummary, KafkaClient, TopicMeta};

/// Requests from the UI to the worker.
pub enum Cmd {
    Connect(EnvProfile),
    RefreshTopics,
    Watermarks(String),
    TopicConfig(String),
    Groups,
    Peek(String),
    Create {
        name: String,
        partitions: i32,
        replication: i32,
    },
    Delete(String),
    AddPartitions {
        name: String,
        total: usize,
    },
    DeleteGroup(String),
    Shutdown,
}

/// Results from the worker back to the UI.
pub enum Evt {
    Connected {
        profile: EnvProfile,
        meta: Vec<TopicMeta>,
    },
    ConnectFailed(String),
    Topics(Vec<TopicMeta>),
    Watermarks {
        topic: String,
        marks: Vec<(i32, i64, i64)>,
    },
    TopicConfig {
        topic: String,
        entries: Vec<(String, String)>,
    },
    Groups(Vec<GroupSummary>),
    Peek {
        records: Vec<EventRecord>,
    },
    /// A mutation (create/delete/+partitions) or refresh succeeded.
    Ok(String),
    /// Any operation failed — carries a human-readable message.
    Failed(String),
}

/// Handle the UI keeps: send commands, receive events.
pub struct Worker {
    pub tx: Sender<Cmd>,
    pub rx: Receiver<Evt>,
}

impl Worker {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (evt_tx, evt_rx) = mpsc::channel::<Evt>();

        std::thread::spawn(move || run(&cmd_rx, &evt_tx));

        Worker {
            tx: cmd_tx,
            rx: evt_rx,
        }
    }

    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }
}

fn run(cmd_rx: &Receiver<Cmd>, evt: &Sender<Evt>) {
    let mut client: Option<KafkaClient> = None;

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            Cmd::Connect(profile) => match KafkaClient::connect(&profile) {
                Ok(c) => {
                    let meta = c.metadata();
                    client = Some(c);
                    send(evt, Evt::Connected { profile, meta });
                }
                Err(e) => send(evt, Evt::ConnectFailed(format!("{e:#}"))),
            },

            Cmd::RefreshTopics => with_client_mut(&mut client, evt, |c| {
                c.reload_meta()?;
                Ok(Evt::Topics(c.metadata()))
            }),

            Cmd::Watermarks(topic) => with_client(&client, evt, |c| {
                let marks = c.watermarks(&topic)?;
                Ok(Evt::Watermarks { topic, marks })
            }),

            // Config-load errors surface in the pane (no toast), so scrolling
            // through topics on a cluster without DescribeConfigs ACL is quiet.
            Cmd::TopicConfig(topic) => {
                if let Some(c) = &client {
                    let entries = c
                        .topic_config(&topic)
                        .unwrap_or_else(|e| vec![("(unavailable)".into(), format!("{e:#}"))]);
                    send(evt, Evt::TopicConfig { topic, entries });
                }
            }

            Cmd::Groups => with_client(&client, evt, |c| Ok(Evt::Groups(c.consumer_groups()?))),

            Cmd::Peek(topic) => with_client(&client, evt, |c| {
                let records = c.peek(&topic, 50)?;
                Ok(Evt::Peek { records })
            }),

            Cmd::Create {
                name,
                partitions,
                replication,
            } => mutate(&mut client, evt, format!("created {name}"), |c| {
                c.create_topic(&name, partitions, replication)
            }),

            Cmd::Delete(name) => mutate(&mut client, evt, format!("deleted {name}"), |c| {
                c.delete_topic(&name)
            }),

            Cmd::AddPartitions { name, total } => mutate(
                &mut client,
                evt,
                format!("{name}: partitions → {total}"),
                |c| c.add_partitions(&name, total),
            ),

            Cmd::DeleteGroup(name) => with_client(&client, evt, |c| {
                c.delete_group(&name)?;
                // Re-list groups so the view reflects the deletion.
                send(evt, Evt::Ok(format!("deleted group {name}")));
                Ok(Evt::Groups(c.consumer_groups()?))
            }),

            Cmd::Shutdown => break,
        }
    }
}

fn send(evt: &Sender<Evt>, e: Evt) {
    let _ = evt.send(e);
}

fn with_client(
    client: &Option<KafkaClient>,
    evt: &Sender<Evt>,
    f: impl FnOnce(&KafkaClient) -> anyhow::Result<Evt>,
) {
    let Some(c) = client else {
        return send(evt, Evt::Failed("not connected".into()));
    };
    match f(c) {
        Ok(e) => send(evt, e),
        Err(e) => send(evt, Evt::Failed(format!("{e:#}"))),
    }
}

fn with_client_mut(
    client: &mut Option<KafkaClient>,
    evt: &Sender<Evt>,
    f: impl FnOnce(&mut KafkaClient) -> anyhow::Result<Evt>,
) {
    let Some(c) = client.as_mut() else {
        return send(evt, Evt::Failed("not connected".into()));
    };
    match f(c) {
        Ok(e) => send(evt, e),
        Err(e) => send(evt, Evt::Failed(format!("{e:#}"))),
    }
}

/// A mutation: run it, and only on success reload metadata + emit `Ok` and a
/// fresh topic list. On failure, emit `Failed` and leave state untouched.
fn mutate(
    client: &mut Option<KafkaClient>,
    evt: &Sender<Evt>,
    ok_msg: String,
    op: impl FnOnce(&KafkaClient) -> anyhow::Result<()>,
) {
    let Some(c) = client.as_mut() else {
        return send(evt, Evt::Failed("not connected".into()));
    };
    match op(c).and_then(|()| c.reload_meta()) {
        Ok(()) => {
            send(evt, Evt::Ok(ok_msg));
            send(evt, Evt::Topics(c.metadata()));
        }
        Err(e) => send(evt, Evt::Failed(format!("{e:#}"))),
    }
}
