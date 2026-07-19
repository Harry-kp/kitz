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

/// Connection in progress — drives the spinner overlay.
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

/// A transient top-right notification (vortix-style). Auto-expires.
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
    },
    /// Context action menu (vortix-style): every action for the current
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
}

pub struct DeleteForm {
    pub kind: DeleteKind,
    /// Topic or group name being deleted.
    pub target: String,
    pub confirm: String,
    pub is_prod: bool,
}

#[derive(Default)]
pub struct PartForm {
    pub topic: String,
    pub total: String,
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

    /// Cached cluster metadata — the source for the topic list + detail. Free
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
    rate_last_total: Option<i64>,
    rate_last_at: Instant,

    pub groups: Vec<GroupSummary>,
    pub group_state: TableState,
    pub groups_loaded: bool,
    pub loading_groups: bool,

    pub peeking: bool,

    /// Activity/debug log (vortix-style). Newest last; capped.
    pub logs: Vec<String>,
    /// Lines scrolled back from the newest (0 = pinned to newest).
    pub logs_scroll: u16,

    pub toast: Option<Toast>,

    pub modal: Modal,
    pub status: String,
    pub should_quit: bool,
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
            if t.born.elapsed().as_millis() > 3600 {
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
            Evt::Connected { profile, meta } => {
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
                    if let Some(prev) = self.rate_last_total {
                        let delta = (total - prev).max(0) as u64;
                        self.rate.push(delta);
                        if self.rate.len() > 120 {
                            self.rate.remove(0);
                        }
                    }
                    self.rate_last_total = Some(total);
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
                self.log(format!("peeked {} events", records.len()));
                self.modal = Modal::Peek { records, sel: 0 };
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
                    leader: p.leader,
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
            let name = env.name.clone();
            self.toast(ToastLevel::Warning, format!("already on {name}"));
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
            format!("tracking {} — graph is live", name.0),
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

    /// Clamp at the ends — no wrap-around (per user: no circular looping).
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
                self.rebuild_detail(); // instant — from cache, no network
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
            (Screen::EnvSelect, Enter) => self.start_connect(),

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
            (Screen::Main, Char(c)) if c.is_ascii_digit() && c != '0' => {
                self.switch_env((c as u8 - b'1') as usize);
            }

            (Screen::Main, Char('/')) => {
                self.focus = Panel::Topics;
                self.filtering = true;
                self.filter.clear();
            }
            (Screen::Main, Char('c')) => {
                self.modal = Modal::Create(CreateForm {
                    partitions: "1".into(),
                    replication: "3".into(),
                    ..Default::default()
                });
            }
            (Screen::Main, Char('d')) => self.open_delete(),
            (Screen::Main, Char('a')) => self.open_add_partitions(),
            (Screen::Main, Char('p')) => self.peek(),

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
            });
        }
    }

    fn open_add_partitions(&mut self) {
        if let Some(topic) = self.selected_topic_name() {
            self.modal = Modal::AddPartitions(PartForm {
                topic,
                total: String::new(),
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

            Modal::Peek { records, mut sel } => match key.code {
                Esc | Char('q') => {} // modal already cleared → closes
                Up | Char('k') => {
                    sel = sel.saturating_sub(1);
                    self.modal = Modal::Peek { records, sel };
                }
                Down | Char('j') => {
                    sel = (sel + 1).min(records.len().saturating_sub(1));
                    self.modal = Modal::Peek { records, sel };
                }
                Char('y') => {
                    if let Some(r) = records.get(sel) {
                        let payload = r.payload.clone();
                        self.copy(&payload, "payload");
                    }
                    self.modal = Modal::Peek { records, sel };
                }
                Char('Y') => {
                    if let Some(r) = records.get(sel) {
                        let key = r.key.clone();
                        self.copy(&key, "key");
                    }
                    self.modal = Modal::Peek { records, sel };
                }
                _ => self.modal = Modal::Peek { records, sel },
            },

            Modal::Create(mut f) => match key.code {
                Esc => {}
                Tab_ | Char('\t') => {
                    f.focus = (f.focus + 1) % 3;
                    self.modal = Modal::Create(f);
                }
                Enter => self.submit_create(&f),
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
                Enter => self.submit_add_partitions(&f),
                Backspace => {
                    f.total.pop();
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
                Enter => self.submit_delete(&f),
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

    fn submit_create(&mut self, f: &CreateForm) {
        if f.name.trim().is_empty() {
            self.modal = Modal::Error("topic name required".into());
            return;
        }
        let partitions: i32 = f.partitions.trim().parse().unwrap_or(1);
        let replication: i32 = f.replication.trim().parse().unwrap_or(3);
        self.status = format!("creating {}…", f.name.trim());
        self.worker.send(Cmd::Create {
            name: f.name.trim().to_string(),
            partitions,
            replication,
        });
    }

    fn submit_add_partitions(&mut self, f: &PartForm) {
        let Ok(total) = f.total.trim().parse::<usize>() else {
            self.modal = Modal::Error("partition count must be a number".into());
            return;
        };
        self.status = format!("adding partitions to {}…", f.topic);
        self.worker.send(Cmd::AddPartitions {
            name: f.topic.clone(),
            total,
        });
    }

    fn submit_delete(&mut self, f: &DeleteForm) {
        // Prod guardrail applies to both topics and groups: the typed
        // confirmation must match the target name.
        if f.is_prod && f.confirm.trim() != f.target {
            self.modal = Modal::Error("confirmation text did not match the name".into());
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

/// UTC HH:MM:SS for activity-log timestamps (no chrono dependency).
fn now_hms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:02}:{:02}:{:02}", (s / 3600) % 24, (s / 60) % 60, s % 60)
}

fn copy_to_clipboard(text: &str) -> std::result::Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text.to_string()).map_err(|e| e.to_string())
}

// crossterm's KeyCode::Tab collides with our `Tab_` usage in match arms after
// the `use KeyCode::*` glob; alias it. (BackTab comes from the glob.)
use crossterm::event::KeyCode::Tab as Tab_;
