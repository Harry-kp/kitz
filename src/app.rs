//! Application state + input handling.
//!
//! The UI thread NEVER touches Kafka directly. It sends [`Cmd`]s to the
//! [`Worker`] thread and applies [`Evt`]s it drains each tick, so rendering
//! stays smooth no matter how slow the cluster is. Cheap reads (topic list,
//! partition structure) come from a locally-cached `meta`, so navigation is
//! instant; expensive reads (watermarks, groups, peek) are requested lazily
//! and land asynchronously with a loading indicator in the meantime.

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::widgets::{ListState, TableState};
use ratatui_flip_panel::FlipState;

use crate::config::{Config, EnvProfile};
use crate::kafka::{EventRecord, GroupSummary, PartitionInfo, TopicDetail, TopicMeta};
use crate::worker::{Cmd, Evt, Worker};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Screen {
    EnvSelect,
    Main,
    /// Full-screen cluster-wide consumer groups list (toggled with `G`).
    Groups,
}

/// Dashboard panels. Right column is now topic-scoped (Detail) + global Logs;
/// consumer groups moved to their own full-screen view (they're cluster-wide,
/// so pinning them next to a highlighted topic was confusing).
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Panel {
    Topics,
    Graph,
    /// Bottom-left pane; flips between Detail (front) and Config (back) with `f`.
    Detail,
    Logs,
}

/// Connection in progress - drives the spinner overlay.
pub struct Connecting {
    pub profile: EnvProfile,
    pub started: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A transient top-right notification. Auto-expires.
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub born: Instant,
}

pub enum Modal {
    None,
    Create(CreateForm),
    Delete(DeleteForm),
    AddPartitions(PartForm),
    Peek {
        records: Vec<EventRecord>,
        sel: usize,
        /// Lines scrolled in the payload pane (reset when `sel` changes).
        scroll: u16,
    },
    /// Context action menu: every action for the current
    /// screen/pane. Keeps the footer to essentials.
    Actions {
        items: Vec<(char, &'static str)>,
        sel: usize,
    },
    Help,
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeleteKind {
    Topic,
    Group,
}

#[derive(Default)]
pub struct CreateForm {
    pub name: String,
    pub partitions: String,
    pub replication: String,
    pub focus: usize,
    /// Validation message shown inside the form (the form stays open).
    pub error: Option<String>,
}

pub struct DeleteForm {
    pub kind: DeleteKind,
    /// Topic or group name being deleted.
    pub target: String,
    pub confirm: String,
    pub is_prod: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct PartForm {
    pub topic: String,
    pub current: usize,
    pub total: String,
    /// Prod only: typed topic name, same guardrail as delete (irreversible).
    pub is_prod: bool,
    pub confirm: String,
    /// 0 = total, 1 = confirm.
    pub focus: usize,
    pub error: Option<String>,
}

pub struct App {
    pub config: Config,
    pub worker: Worker,
    pub screen: Screen,
    pub env_state: ListState,

    pub connected: Option<EnvProfile>,
    pub connecting: Option<Connecting>,

    pub focus: Panel,
    pub zoom: bool,

    /// Cached cluster metadata - the source for the topic list + detail. Free
    /// to read (no network), so navigation never blocks.
    pub meta: Vec<TopicMeta>,
    pub topic_state: ListState,
    pub filter: String,
    pub filtering: bool,
    pub detail: Option<TopicDetail>,
    pub detail_scroll: u16,
    pub loading_watermarks: bool,

    /// Config of the currently selected topic: (topic, [(key,value)]).
    pub topic_config: Option<(String, Vec<(String, String)>)>,
    pub loading_config: bool,
    /// Bottom-left pane flip animation (Detail front ⟷ Config back).
    pub flip: FlipState,

    // Live incoming-events graph (top-right). Sampling is opt-in per topic via `w`.
    pub rate: Vec<u64>,
    pub rate_topic: Option<String>,
    /// Last total + when it arrived, to turn deltas into events/second.
    rate_last_total: Option<(i64, Instant)>,
    rate_last_at: Instant,

    pub groups: Vec<GroupSummary>,
    pub group_state: TableState,
    pub groups_loaded: bool,
    pub loading_groups: bool,

    pub peeking: bool,

    /// Activity/debug log. Newest last; capped.
    pub logs: Vec<String>,
    /// Lines scrolled back from the newest (0 = pinned to newest).
    pub logs_scroll: u16,

    pub toast: Option<Toast>,

    pub modal: Modal,
    pub status: String,
    pub should_quit: bool,
    /// Brokers in the connected cluster (caps the default replication factor).
    pub brokers: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut env_state = ListState::default();
        env_state.select(Some(0));
        Self {
            config,
            worker: Worker::spawn(),
            screen: Screen::EnvSelect,
            env_state,
            connected: None,
            connecting: None,
            focus: Panel::Topics,
            zoom: false,
            meta: Vec::new(),
            topic_state: ListState::default(),
            filter: String::new(),
            filtering: false,
            detail: None,
            detail_scroll: 0,
            loading_watermarks: false,
            topic_config: None,
            loading_config: false,
            flip: FlipState::new(Duration::from_millis(280)),
            rate: Vec::new(),
            rate_topic: None,
            rate_last_total: None,
            rate_last_at: Instant::now(),
            groups: Vec::new(),
            group_state: TableState::default(),
            groups_loaded: false,
            loading_groups: false,
            peeking: false,
            logs: Vec::new(),
            logs_scroll: 0,
            toast: None,
            modal: Modal::None,
            status: "↑↓ select env · Enter connect · q quit".into(),
            should_quit: false,
            brokers: 0,
        }
    }

    /// Append a timestamped line to the activity log (capped at 500).
    fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(format!("{}  {}", now_hms(), msg.into()));
        if self.logs.len() > 500 {
            self.logs.drain(0..self.logs.len() - 500);
        }
    }

    /// Raise a transient notification (also mirrored into the activity log).
    fn toast(&mut self, level: ToastLevel, msg: impl Into<String>) {
        let msg = msg.into();
        self.log(&msg);
        self.toast = Some(Toast {
            message: msg,
            level,
            born: Instant::now(),
        });
    }

    /// Whether the flip animation is mid-flight (drives faster redraws).
    pub fn animating(&self) -> bool {
        self.flip.is_animating()
    }

    /// Called each tick: advance the flip animation, expire the toast, drive
    /// live rate sampling.
    pub fn tick(&mut self) {
        self.flip.tick();

        if let Some(t) = &self.toast {
            // Errors stay up long enough to actually read.
            let ttl = if matches!(t.level, ToastLevel::Error) {
                8000
            } else {
                3600
            };
            if t.born.elapsed().as_millis() > ttl {
                self.toast = None;
            }
        }

        // Incoming-events graph: while a topic is opted-in (via `w`) and still
        // selected, re-request its watermarks every few seconds; the delta is
        // the events produced in that window.
        if let Some(rt) = self.rate_topic.clone() {
            if self.selected_topic_name().as_deref() != Some(rt.as_str()) {
                self.rate_topic = None;
                self.rate.clear();
                self.rate_last_total = None;
            } else if self.rate_last_at.elapsed().as_millis() > 3500 {
                self.rate_last_at = Instant::now();
                self.worker.send(Cmd::Watermarks(rt));
            }
        }
    }

    // ── Derived views ──────────────────────────────────────────────────

    pub fn is_prod(&self) -> bool {
        self.connected.as_ref().map(|p| p.prod).unwrap_or(false)
    }

    pub fn topic_count(&self) -> usize {
        self.meta.len()
    }

    /// Indices into `self.meta` matching the current filter.
    pub fn filtered_topics(&self) -> Vec<usize> {
        let f = self.filter.to_lowercase();
        self.meta
            .iter()
            .enumerate()
            .filter(|(_, t)| f.is_empty() || t.name.to_lowercase().contains(&f))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn topic_row(&self, meta_idx: usize) -> (&str, usize) {
        let t = &self.meta[meta_idx];
        (&t.name, t.partitions.len())
    }

    fn selected_topic_name(&self) -> Option<String> {
        let visible = self.filtered_topics();
        let sel = self.topic_state.selected()?;
        visible.get(sel).map(|&i| self.meta[i].name.clone())
    }

    // ── Async event handling (drained each tick, never blocks) ───────────

    pub fn drain_events(&mut self) {
        while let Ok(evt) = self.worker.rx.try_recv() {
            self.apply(evt);
        }
    }

    /// Ask the worker thread to exit (called on quit).
    pub fn shutdown(&self) {
        self.worker.send(Cmd::Shutdown);
    }

    fn apply(&mut self, evt: Evt) {
        match evt {
            Evt::Connected {
                profile,
                meta,
                brokers,
            } => {
                self.brokers = brokers;
                self.log(format!(
                    "connected to {} · {} topics",
                    profile.name,
                    meta.len()
                ));
                self.connected = Some(profile);
                self.connecting = None;
                self.meta = meta;
                self.topic_state
                    .select((!self.meta.is_empty()).then_some(0));
                self.screen = Screen::Main;
                self.status = format!("{} topics", self.meta.len());
                self.rebuild_detail();
                // Load groups in the background so Detail can show which groups
                // consume the selected topic (no blocking on connect).
                self.loading_groups = true;
                self.worker.send(Cmd::Groups);
            }
            Evt::ConnectFailed(e) => {
                self.connecting = None;
                self.log(format!("connect failed: {e}"));
                self.modal = Modal::Error(format!("connect failed: {e}"));
            }
            Evt::Topics(meta) => {
                self.log(format!("topics refreshed · {}", meta.len()));
                self.meta = meta;
                let n = self.filtered_topics().len();
                if self.topic_state.selected().is_none_or(|s| s >= n) {
                    self.topic_state.select((n > 0).then_some(0));
                }
                self.status = format!("{} topics", self.meta.len());
                self.rebuild_detail();
            }
            Evt::Watermarks { topic, marks } => {
                self.loading_watermarks = false;
                let total: i64 = marks.iter().map(|(_, _, high)| *high).sum();
                if let Some(d) = &mut self.detail {
                    if d.name == topic {
                        for (id, low, high) in marks {
                            if let Some(p) = d.partitions.iter_mut().find(|p| p.id == id) {
                                p.low = low;
                                p.high = high;
                            }
                        }
                        d.watermarks_loaded = true;
                    }
                }
                // Feed the incoming-events graph.
                if self.rate_topic.as_deref() == Some(topic.as_str()) {
                    if let Some((prev, at)) = self.rate_last_total {
                        // Polls are ~3.5s apart plus round-trip, so divide by
                        // the measured gap - the graph is labelled per second.
                        let secs = at.elapsed().as_secs_f64().max(0.001);
                        let per_sec = ((total - prev).max(0) as f64 / secs).round() as u64;
                        self.rate.push(per_sec);
                        if self.rate.len() > 120 {
                            self.rate.remove(0);
                        }
                    }
                    self.rate_last_total = Some((total, Instant::now()));
                } else {
                    self.log(format!("loaded event counts for {topic}"));
                }
            }
            Evt::Groups(groups) => {
                self.log(format!("loaded {} consumer groups", groups.len()));
                self.groups = groups;
                self.groups_loaded = true;
                self.loading_groups = false;
                self.group_state
                    .select((!self.groups.is_empty()).then_some(0));
                self.status = format!("{} consumer groups", self.groups.len());
            }
            Evt::TopicConfig { topic, entries } => {
                self.loading_config = false;
                // Keep only if it's still the selected topic.
                if self
                    .detail
                    .as_ref()
                    .map(|d| d.name == topic)
                    .unwrap_or(false)
                {
                    self.topic_config = Some((topic, entries));
                }
            }
            Evt::Peek { records } => {
                self.peeking = false;
                self.status = format!("peeked {} events", records.len());
                self.log(format!("peeked {} events", records.len()));
                self.modal = Modal::Peek {
                    records,
                    sel: 0,
                    scroll: 0,
                };
            }
            Evt::Ok(msg) => {
                self.status = msg.clone();
                self.toast(ToastLevel::Success, msg);
            }
            Evt::Failed(e) => {
                self.loading_watermarks = false;
                self.loading_groups = false;
                self.peeking = false;
                // Non-blocking: operation failures pop a toast, not a modal.
                // Clear the "…ing" status so it doesn't read as still running.
                self.status = "last action failed - see Logs".into();
                self.toast(ToastLevel::Error, e);
            }
        }
    }

    fn rebuild_detail(&mut self) {
        self.detail_scroll = 0;
        self.loading_watermarks = false;
        // Selection changed → stop the previous topic's live graph.
        self.rate_topic = None;
        self.rate.clear();
        self.rate_last_total = None;
        let Some(name) = self.selected_topic_name() else {
            self.detail = None;
            self.topic_config = None;
            return;
        };
        let Some(t) = self.meta.iter().find(|t| t.name == name) else {
            self.detail = None;
            self.topic_config = None;
            return;
        };
        self.detail = Some(TopicDetail {
            name: name.clone(),
            partitions: t
                .partitions
                .iter()
                .map(|p| PartitionInfo {
                    id: p.id,
                    replicas: p.replicas,
                    isr: p.isr,
                    low: -1,
                    high: -1,
                })
                .collect(),
            watermarks_loaded: false,
        });
        // Fetch this topic's config for the top-right pane (async, non-blocking).
        self.topic_config = None;
        self.loading_config = true;
        self.worker.send(Cmd::TopicConfig(name));
    }

    /// Names of consumer groups subscribed to `topic` (from the group list).
    pub fn groups_for_topic(&self, topic: &str) -> Vec<&str> {
        self.groups
            .iter()
            .filter(|g| g.topics.iter().any(|t| t == topic))
            .map(|g| g.name.as_str())
            .collect()
    }

    // ── Commands to the worker ───────────────────────────────────────────

    fn start_connect(&mut self) {
        if self.connecting.is_some() {
            return;
        }
        let Some(i) = self.env_state.selected() else {
            return;
        };
        let profile = self.config.envs[i].clone();
        // Clear any state from a previous environment before reconnecting.
        self.reset_dashboard();
        self.status = format!("connecting to {}…", profile.name);
        self.worker.send(Cmd::Connect(profile.clone()));
        self.connecting = Some(Connecting {
            profile,
            started: Instant::now(),
        });
    }

    /// Wipe per-cluster state so switching environments never shows stale data.
    fn reset_dashboard(&mut self) {
        self.meta.clear();
        self.topic_state.select(None);
        self.detail = None;
        self.detail_scroll = 0;
        self.loading_watermarks = false;
        self.groups.clear();
        self.group_state.select(None);
        self.groups_loaded = false;
        self.loading_groups = false;
        self.filter.clear();
        self.filtering = false;
    }

    /// Index of the currently-connected env in the config (for the picker).
    pub fn current_env_index(&self) -> Option<usize> {
        let name = &self.connected.as_ref()?.name;
        self.config.envs.iter().position(|e| &e.name == name)
    }

    /// Hot-switch to env `idx` (number keys). No-op if already there.
    fn switch_env(&mut self, idx: usize) {
        let Some(env) = self.config.envs.get(idx) else {
            return;
        };
        if self.connecting.is_none() && self.current_env_index() == Some(idx) {
            if self.screen == Screen::EnvSelect {
                self.screen = Screen::Main; // picked the env we're on: just go back
            } else {
                let name = env.name.clone();
                self.toast(ToastLevel::Warning, format!("already on {name}"));
            }
            return;
        }
        self.env_state.select(Some(idx));
        self.start_connect();
    }

    /// Jump to the top/bottom of the focused list.
    fn jump(&mut self, top: bool) {
        match self.focus {
            Panel::Topics => {
                let len = self.filtered_topics().len();
                if len > 0 {
                    self.topic_state.select(Some(if top { 0 } else { len - 1 }));
                    self.rebuild_detail();
                }
            }
            Panel::Detail => {
                let max = self
                    .detail
                    .as_ref()
                    .map(|d| d.partitions.len() as u16)
                    .unwrap_or(0);
                self.detail_scroll = if top { 0 } else { max };
            }
            Panel::Logs => {
                self.logs_scroll = if top { self.logs.len() as u16 } else { 0 };
            }
            Panel::Graph => {}
        }
    }

    fn load_watermarks(&mut self) {
        if self.loading_watermarks {
            return;
        }
        let Some(name) = self
            .detail
            .as_ref()
            .map(|d| (d.name.clone(), d.watermarks_loaded))
        else {
            return;
        };
        if name.1 && self.rate_topic.as_deref() == Some(name.0.as_str()) {
            self.toast(ToastLevel::Info, "already tracking this topic");
            return;
        }
        // `w` loads counts AND starts the live incoming-events graph.
        self.loading_watermarks = true;
        self.rate_topic = Some(name.0.clone());
        self.rate.clear();
        self.rate_last_total = None;
        self.rate_last_at = Instant::now();
        self.toast(
            ToastLevel::Info,
            format!("tracking {} - graph is live", name.0),
        );
        self.worker.send(Cmd::Watermarks(name.0));
    }

    fn ensure_groups(&mut self) {
        if self.groups_loaded || self.loading_groups {
            return;
        }
        self.loading_groups = true;
        self.status = "loading consumer groups…".into();
        self.worker.send(Cmd::Groups);
    }

    fn refresh(&mut self) {
        self.status = "refreshing…".into();
        self.worker.send(Cmd::RefreshTopics);
        if self.groups_loaded {
            self.loading_groups = true;
            self.worker.send(Cmd::Groups);
        }
    }

    fn peek(&mut self) {
        if self.peeking {
            return;
        }
        let Some(name) = self.selected_topic_name() else {
            return;
        };
        self.peeking = true;
        self.status = format!("peeking {name}…");
        self.worker.send(Cmd::Peek(name));
    }

    // ── Navigation ────────────────────────────────────────────────────

    /// Clamp at the ends - no wrap-around (per user: no circular looping).
    fn next_index(cur: Option<usize>, len: usize, delta: isize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let cur = cur.unwrap_or(0) as isize;
        Some((cur + delta).clamp(0, len as isize - 1) as usize)
    }

    fn cycle_focus(&mut self, forward: bool) {
        // Visual order: Topics (TL) → Graph (TR) → Detail (BL) → Logs (BR).
        self.focus = match (self.focus, forward) {
            (Panel::Topics, true) => Panel::Graph,
            (Panel::Graph, true) => Panel::Detail,
            (Panel::Detail, true) => Panel::Logs,
            (Panel::Logs, true) => Panel::Topics,
            (Panel::Topics, false) => Panel::Logs,
            (Panel::Logs, false) => Panel::Detail,
            (Panel::Detail, false) => Panel::Graph,
            (Panel::Graph, false) => Panel::Topics,
        };
    }

    fn nav(&mut self, delta: isize) {
        match self.focus {
            Panel::Topics => {
                let len = self.filtered_topics().len();
                let n = Self::next_index(self.topic_state.selected(), len, delta);
                self.topic_state.select(n);
                self.rebuild_detail(); // instant - from cache, no network
            }
            Panel::Detail => {
                let max = self
                    .detail
                    .as_ref()
                    .map(|d| d.partitions.len() as u16)
                    .unwrap_or(0);
                self.detail_scroll =
                    (self.detail_scroll as isize + delta).clamp(0, max as isize) as u16;
            }
            Panel::Logs => {
                // logs_scroll counts lines back from newest.
                let max = self.logs.len() as isize;
                self.logs_scroll = (self.logs_scroll as isize + delta).clamp(0, max) as u16;
            }
            Panel::Graph => {}
        }
    }

    /// Navigate the fullscreen groups view.
    fn nav_groups(&mut self, delta: isize) {
        let n = Self::next_index(self.group_state.selected(), self.groups.len(), delta);
        self.group_state.select(n);
    }

    // ── Input ──────────────────────────────────────────────────────────

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode::*;

        // Ctrl-C always quits: raw mode swallows SIGINT, and in the filter it
        // would otherwise type a 'c'.
        if key.code == Char('c')
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            self.should_quit = true;
            return Ok(());
        }

        if !matches!(self.modal, Modal::None) {
            self.on_modal_key(key);
            return Ok(());
        }

        if self.connecting.is_some() {
            if key.code == Char('q') {
                self.should_quit = true;
            }
            return Ok(());
        }

        if self.filtering {
            match key.code {
                Esc => {
                    self.filtering = false;
                    self.filter.clear();
                    self.topic_state.select(Some(0));
                    self.rebuild_detail();
                }
                Enter => self.filtering = false,
                Backspace => {
                    self.filter.pop();
                    self.topic_state.select(Some(0));
                    self.rebuild_detail();
                }
                Char(c) => {
                    self.filter.push(c);
                    self.topic_state.select(Some(0));
                    self.rebuild_detail();
                }
                _ => {}
            }
            return Ok(());
        }

        match (self.screen, key.code) {
            (_, Char('q')) => self.should_quit = true,

            (Screen::EnvSelect, Up | Char('k')) => {
                let n = Self::next_index(self.env_state.selected(), self.config.envs.len(), -1);
                self.env_state.select(n);
            }
            (Screen::EnvSelect, Down | Char('j')) => {
                let n = Self::next_index(self.env_state.selected(), self.config.envs.len(), 1);
                self.env_state.select(n);
            }
            (Screen::EnvSelect, Enter) => match self.env_state.selected() {
                Some(i) if self.connected.is_some() => self.switch_env(i),
                _ => self.start_connect(),
            },
            (Screen::EnvSelect, Esc) if self.connected.is_some() => self.screen = Screen::Main,
            (Screen::EnvSelect | Screen::Main, Char(c)) if c.is_ascii_digit() && c != '0' => {
                self.switch_env((c as u8 - b'1') as usize);
            }

            (_, Char('?')) => self.modal = Modal::Help,
            (Screen::Main | Screen::Groups, Char('x')) => self.open_actions(),

            (Screen::Main, Tab_ | Char('l') | Right) => self.cycle_focus(true),
            (Screen::Main, BackTab | Char('h') | Left) => self.cycle_focus(false),
            (Screen::Main, Char('z')) => self.zoom = !self.zoom,
            (Screen::Main, Char('f')) => {
                self.flip.flip();
            }

            (Screen::Main, Char('g')) => self.jump(true),
            (Screen::Main, Up | Char('k')) => self.nav(-1),
            (Screen::Main, Down | Char('j')) => self.nav(1),

            (Screen::Main, Char('r')) => self.refresh(),
            (Screen::Main, Char('w')) => self.load_watermarks(),

            // Full-screen consumer groups view.
            (Screen::Main, Char('G')) => {
                self.ensure_groups();
                self.screen = Screen::Groups;
            }

            // ── Environment switching ──
            (Screen::Main, Char('e')) => {
                self.env_state
                    .select(Some(self.current_env_index().unwrap_or(0)));
                self.screen = Screen::EnvSelect;
            }
            (Screen::Main, Esc) if !self.filter.is_empty() => {
                self.filter.clear();
                self.topic_state.select(Some(0));
                self.rebuild_detail();
            }
            (Screen::Main, Char('/')) => {
                self.focus = Panel::Topics;
                self.filtering = true;
                self.filter.clear();
            }
            (Screen::Main, Char('c')) => {
                self.modal = Modal::Create(CreateForm {
                    partitions: "1".into(),
                    // 3 is the usual default, but it fails outright on smaller
                    // clusters (MSK stag is often 2 brokers).
                    replication: self.brokers.clamp(1, 3).to_string(),
                    ..Default::default()
                });
            }
            (Screen::Main, Char('d')) => self.open_delete(),
            (Screen::Main, Char('a')) => self.open_add_partitions(),
            (Screen::Main, Char('p')) => self.peek(),
            (Screen::Main, Char('y')) => {
                if let Some(name) = self.selected_topic_name() {
                    self.copy(&name, "topic name");
                }
            }

            // ── Full-screen groups view ──
            (Screen::Groups, Esc | Char('G')) => self.screen = Screen::Main,
            (Screen::Groups, Up | Char('k')) => self.nav_groups(-1),
            (Screen::Groups, Down | Char('j')) => self.nav_groups(1),
            (Screen::Groups, Char('g')) => {
                self.group_state
                    .select((!self.groups.is_empty()).then_some(0));
            }
            (Screen::Groups, Char('d')) => self.open_delete_group(),
            (Screen::Groups, Char('r')) => {
                self.loading_groups = true;
                self.status = "refreshing groups…".into();
                self.worker.send(Cmd::Groups);
            }

            _ => {}
        }
        Ok(())
    }

    fn open_delete(&mut self) {
        if let Some(topic) = self.selected_topic_name() {
            self.modal = Modal::Delete(DeleteForm {
                kind: DeleteKind::Topic,
                target: topic,
                confirm: String::new(),
                is_prod: self.is_prod(),
                error: None,
            });
        }
    }

    fn open_delete_group(&mut self) {
        let Some(i) = self.group_state.selected() else {
            return;
        };
        if let Some(g) = self.groups.get(i) {
            self.modal = Modal::Delete(DeleteForm {
                kind: DeleteKind::Group,
                target: g.name.clone(),
                confirm: String::new(),
                is_prod: self.is_prod(),
                error: None,
            });
        }
    }

    fn open_add_partitions(&mut self) {
        if let Some(topic) = self.selected_topic_name() {
            let current = self
                .meta
                .iter()
                .find(|t| t.name == topic)
                .map_or(0, |t| t.partitions.len());
            self.modal = Modal::AddPartitions(PartForm {
                topic,
                current,
                total: String::new(),
                is_prod: self.is_prod(),
                confirm: String::new(),
                focus: 0,
                error: None,
            });
        }
    }

    /// Build the context action menu for the current screen/pane.
    fn open_actions(&mut self) {
        let items: Vec<(char, &'static str)> = match self.screen {
            Screen::Groups => vec![
                ('d', "delete selected group"),
                ('r', "refresh groups"),
                ('e', "switch environment"),
            ],
            _ => vec![
                ('w', "load event counts"),
                ('p', "peek events  (y copy payload)"),
                ('/', "find topics"),
                ('c', "create topic"),
                ('a', "add partitions"),
                ('d', "delete topic"),
                ('r', "refresh"),
                ('G', "consumer groups"),
                ('e', "switch environment"),
                ('z', "zoom focused pane"),
            ],
        };
        self.modal = Modal::Actions { items, sel: 0 };
    }

    fn on_modal_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode::*;

        let modal = std::mem::replace(&mut self.modal, Modal::None);
        match modal {
            Modal::Error(_) | Modal::Help | Modal::None => { /* any key dismisses */ }

            Modal::Actions { items, mut sel } => match key.code {
                Esc | Char('x') | Char('q') => {}
                Up | Char('k') => {
                    sel = sel.saturating_sub(1);
                    self.modal = Modal::Actions { items, sel };
                }
                Down | Char('j') => {
                    sel = (sel + 1).min(items.len().saturating_sub(1));
                    self.modal = Modal::Actions { items, sel };
                }
                Enter => {
                    let ch = items.get(sel).map(|(c, _)| *c);
                    if let Some(ch) = ch {
                        self.run_key(ch); // modal already cleared
                    }
                }
                Char(c) => {
                    // Pressing an item's key runs it directly.
                    if items.iter().any(|(k, _)| *k == c) {
                        self.run_key(c);
                    } else {
                        self.modal = Modal::Actions { items, sel };
                    }
                }
                _ => self.modal = Modal::Actions { items, sel },
            },

            Modal::Peek {
                records,
                mut sel,
                mut scroll,
            } => {
                match key.code {
                    Esc | Char('q') => return, // modal already cleared → closes
                    Up | Char('k') => (sel, scroll) = (sel.saturating_sub(1), 0),
                    Down | Char('j') => {
                        (sel, scroll) = ((sel + 1).min(records.len().saturating_sub(1)), 0)
                    }
                    // ponytail: renderer clamps scroll (it knows the wrapped height);
                    // overshooting PgDn makes PgUp lag. Store max_scroll if it bites.
                    PageDown | Char(' ') => scroll = scroll.saturating_add(10),
                    PageUp => scroll = scroll.saturating_sub(10),
                    Char('y') => {
                        if let Some(r) = records.get(sel) {
                            let payload = r.payload.clone();
                            self.copy(&payload, "payload");
                        }
                    }
                    Char('Y') => {
                        if let Some(r) = records.get(sel) {
                            let key = r.key.clone();
                            self.copy(&key, "key");
                        }
                    }
                    _ => {}
                }
                self.modal = Modal::Peek {
                    records,
                    sel,
                    scroll,
                };
            }

            Modal::Create(mut f) => match key.code {
                Esc => {}
                Tab_ | Char('\t') => {
                    f.focus = (f.focus + 1) % 3;
                    self.modal = Modal::Create(f);
                }
                Enter => self.submit_create(f),
                Backspace => {
                    Self::field_mut(&mut f).pop();
                    self.modal = Modal::Create(f);
                }
                Char(c) => {
                    Self::field_mut(&mut f).push(c);
                    self.modal = Modal::Create(f);
                }
                _ => self.modal = Modal::Create(f),
            },

            Modal::AddPartitions(mut f) => match key.code {
                Esc => {}
                Enter => self.submit_add_partitions(f),
                Tab_ | BackTab if f.is_prod => {
                    f.focus = 1 - f.focus;
                    self.modal = Modal::AddPartitions(f);
                }
                Backspace => {
                    if f.focus == 0 {
                        f.total.pop()
                    } else {
                        f.confirm.pop()
                    };
                    self.modal = Modal::AddPartitions(f);
                }
                Char(c) if f.focus == 1 => {
                    f.confirm.push(c);
                    self.modal = Modal::AddPartitions(f);
                }
                Char(c) if c.is_ascii_digit() => {
                    f.total.push(c);
                    self.modal = Modal::AddPartitions(f);
                }
                _ => self.modal = Modal::AddPartitions(f),
            },

            Modal::Delete(mut f) => match key.code {
                Esc => {}
                Enter => self.submit_delete(f),
                Backspace => {
                    f.confirm.pop();
                    self.modal = Modal::Delete(f);
                }
                Char(c) => {
                    f.confirm.push(c);
                    self.modal = Modal::Delete(f);
                }
                _ => self.modal = Modal::Delete(f),
            },
        }
    }

    /// Re-dispatch a character as if the user typed it (used by the action
    /// menu so menu items and hotkeys share one code path).
    fn run_key(&mut self, c: char) {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let _ = self.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }

    /// Copy text to the system clipboard, reporting the outcome in the status
    /// line + activity log.
    fn copy(&mut self, text: &str, what: &str) {
        match copy_to_clipboard(text) {
            Ok(()) => self.toast(
                ToastLevel::Success,
                format!("copied {what} ({} bytes)", text.len()),
            ),
            Err(e) => self.toast(ToastLevel::Error, format!("copy failed: {e}")),
        }
    }

    fn field_mut(f: &mut CreateForm) -> &mut String {
        match f.focus {
            0 => &mut f.name,
            1 => &mut f.partitions,
            _ => &mut f.replication,
        }
    }

    // Validation failures keep the form open with an inline message: a
    // dismiss-with-any-key popup would swallow the next keystroke and drop the
    // following ones onto the dashboard (on prod, `a` = add partitions).

    fn submit_create(&mut self, mut f: CreateForm) {
        let partitions = f.partitions.trim().parse::<i32>();
        let replication = f.replication.trim().parse::<i32>();
        let err = if f.name.trim().is_empty() {
            Some("topic name required".to_string())
        } else if !matches!(partitions, Ok(1..)) {
            Some("partitions must be a number ≥ 1".to_string())
        } else if !matches!(replication, Ok(1..)) {
            Some("replication must be a number ≥ 1".to_string())
        } else if self.brokers > 0
            && replication
                .as_ref()
                .is_ok_and(|r| *r as usize > self.brokers)
        {
            Some(format!(
                "replication can't exceed the {} broker(s) in this cluster",
                self.brokers
            ))
        } else {
            None
        };
        if err.is_some() {
            f.error = err;
            self.modal = Modal::Create(f);
            return;
        }
        let (partitions, replication) = (partitions.unwrap_or(1), replication.unwrap_or(1));
        self.status = format!("creating {}…", f.name.trim());
        self.worker.send(Cmd::Create {
            name: f.name.trim().to_string(),
            partitions,
            replication,
        });
    }

    fn submit_add_partitions(&mut self, mut f: PartForm) {
        let total = f.total.trim().parse::<usize>().unwrap_or(0);
        let err = if total <= f.current {
            Some(format!(
                "enter a total above the current {} (partitions can only increase)",
                f.current
            ))
        } else if f.is_prod && f.confirm.trim() != f.topic {
            f.focus = 1;
            Some("PROD: type the topic name to confirm".to_string())
        } else {
            None
        };
        if err.is_some() {
            f.error = err;
            self.modal = Modal::AddPartitions(f);
            return;
        }
        self.status = format!("adding partitions to {}…", f.topic);
        self.worker.send(Cmd::AddPartitions {
            name: f.topic.clone(),
            total,
        });
    }

    fn submit_delete(&mut self, mut f: DeleteForm) {
        // Prod guardrail applies to both topics and groups: the typed
        // confirmation must match the target name.
        if f.is_prod && f.confirm.trim() != f.target {
            f.error = Some("confirmation text did not match the name".into());
            self.modal = Modal::Delete(f);
            return;
        }
        match f.kind {
            DeleteKind::Topic => {
                self.status = format!("deleting topic {}…", f.target);
                self.worker.send(Cmd::Delete(f.target.clone()));
            }
            DeleteKind::Group => {
                self.status = format!("deleting group {}…", f.target);
                self.worker.send(Cmd::DeleteGroup(f.target.clone()));
            }
        }
    }
}

/// Local-time HH:MM:SS for activity-log timestamps.
fn now_hms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    local_time(ms, false)
}

/// Epoch milliseconds → local `HH:MM:SS`, or `YYYY-MM-DD HH:MM:SS` with
/// `date` (no chrono dependency; libc is already linked).
pub fn local_time(epoch_ms: i64, date: bool) -> String {
    let t = epoch_ms.div_euclid(1000) as libc::time_t;
    // SAFETY: localtime_r only writes into the zeroed `tm` we own.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    let hms = format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec);
    if date {
        format!(
            "{:04}-{:02}-{:02} {hms}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday
        )
    } else {
        hms
    }
}

fn copy_to_clipboard(text: &str) -> std::result::Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text.to_string()).map_err(|e| e.to_string())
}

// crossterm's KeyCode::Tab collides with our `Tab_` usage in match arms after
// the `use KeyCode::*` glob; alias it. (BackTab comes from the glob.)
use crossterm::event::KeyCode::Tab as Tab_;

#[cfg(test)]
mod tests {
    #[test]
    fn local_time_formats_epoch_ms() {
        let s = super::local_time(1_791_020_372_134, true);
        assert!(s.starts_with("2026-10-0"), "{s}");
        assert_eq!(s.len(), "2026-10-03 09:39:32".len());
    }
}
