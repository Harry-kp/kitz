//! Application state + input handling.
//!
//! The UI thread NEVER touches Kafka directly. It sends [`Cmd`]s to the
//! [`Worker`] thread and applies [`Evt`]s it drains each tick, so rendering
//! stays smooth no matter how slow the cluster is. Cheap reads (topic list,
//! partition structure) come from a locally-cached `meta`, so navigation is
//! instant; expensive reads (watermarks, groups, peek) are requested lazily
//! and land asynchronously with a loading indicator in the meantime.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::widgets::{ListState, TableState};

use crate::config::{Config, EnvProfile};
use crate::kafka::{EventRecord, GroupSummary, PartitionLag, TopicDetail, TopicMeta};
use crate::worker::{Cmd, Evt, Worker};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Screen {
    EnvSelect,
    Main,
}

/// The two top-level views of a connected cluster. Each is a list on the left
/// and a detail view of the selection on the right; `⇥` switches.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum View {
    Topics,
    Groups,
}

/// How often the selected topic's offsets are re-polled (counts + rate).
const POLL_EVERY: Duration = Duration::from_secs(3);
/// Selection must settle this long before offsets load, so scrolling through
/// a long list doesn't fire a request per row.
const SETTLE: Duration = Duration::from_millis(250);

/// Connection in progress - drives the spinner overlay.
pub struct Connecting {
    pub profile: EnvProfile,
    pub started: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
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
    /// Latest messages of a topic, newest first (Enter on a topic).
    Peek {
        topic: String,
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
    /// Activity log overlay (`L`); the value is lines scrolled back.
    Logs(u16),
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

    pub view: View,

    /// Cached cluster metadata - the source for the topic list + detail. Free
    /// to read (no network), so navigation never blocks.
    pub meta: Vec<TopicMeta>,
    pub topic_state: ListState,
    /// Filter for the active tab's list (cleared when switching tabs).
    pub filter: String,
    pub filtering: bool,
    pub detail: Option<TopicDetail>,
    /// Lines scrolled in the right-hand detail view (PgUp/PgDn).
    pub detail_scroll: u16,
    pub loading_watermarks: bool,
    /// When the current topic got selected (offset loads wait for `SETTLE`).
    selected_at: Instant,
    last_poll: Instant,
    /// Message totals for topics whose offsets have been loaded this session -
    /// shown in the topic list.
    pub counts: HashMap<String, i64>,

    /// Config of the currently selected topic: (topic, [(key,value)]).
    pub topic_config: Option<(String, Vec<(String, String)>)>,

    /// Messages/second for the selected topic, one sample per poll.
    pub rate: Vec<u64>,
    /// (topic, total, when) of the previous poll, to turn deltas into a rate.
    rate_last: Option<(String, i64, Instant)>,

    pub groups: Vec<GroupSummary>,
    pub group_state: TableState,
    pub groups_loaded: bool,
    pub loading_groups: bool,
    /// Committed offsets per group, filled in one group at a time.
    pub lags: HashMap<String, Vec<PartitionLag>>,
    lag_queue: Vec<String>,

    pub peeking: bool,

    /// Activity/debug log. Newest last; capped.
    pub logs: Vec<String>,

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
            view: View::Topics,
            meta: Vec::new(),
            topic_state: ListState::default(),
            filter: String::new(),
            filtering: false,
            detail: None,
            detail_scroll: 0,
            loading_watermarks: false,
            selected_at: Instant::now(),
            last_poll: Instant::now(),
            counts: HashMap::new(),
            topic_config: None,
            rate: Vec::new(),
            rate_last: None,
            groups: Vec::new(),
            group_state: TableState::default(),
            groups_loaded: false,
            loading_groups: false,
            lags: HashMap::new(),
            lag_queue: Vec::new(),
            peeking: false,
            logs: Vec::new(),
            toast: None,
            modal: Modal::None,
            status: String::new(),
            should_quit: false,
            brokers: 0,
        }
    }

    /// Connect straight away (single env, or `kitz <env>`), skipping the picker.
    pub fn connect_to(&mut self, idx: usize) {
        self.env_state.select(Some(idx));
        self.start_connect();
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

    /// Called each tick: expire the toast and keep the selected topic's
    /// counts/rate fresh.
    pub fn tick(&mut self) {
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

        // Offsets for the selected topic load by themselves once the
        // selection settles, then re-poll so counts and rate stay live.
        if self.screen != Screen::Main || self.view != View::Topics || self.loading_watermarks {
            return;
        }
        let Some(d) = &self.detail else { return };
        let due = if d.watermarks_loaded {
            self.last_poll.elapsed() >= POLL_EVERY
        } else {
            self.selected_at.elapsed() >= SETTLE
        };
        if due {
            self.loading_watermarks = true;
            self.last_poll = Instant::now();
            self.worker.send(Cmd::Watermarks(d.name.clone()));
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

    /// Indices into `self.groups` matching the current filter.
    pub fn filtered_groups(&self) -> Vec<usize> {
        let f = self.filter.to_lowercase();
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, g)| f.is_empty() || g.name.to_lowercase().contains(&f))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected_group(&self) -> Option<&GroupSummary> {
        let i = *self.filtered_groups().get(self.group_state.selected()?)?;
        self.groups.get(i)
    }

    /// Total lag of a group, once its offsets have loaded.
    pub fn group_lag(&self, group: &str) -> Option<i64> {
        self.lags
            .get(group)
            .map(|ps| ps.iter().map(PartitionLag::lag).sum())
    }

    /// Topics a group reads: from live members' subscriptions plus anything it
    /// has committed offsets on (so idle groups still show their topics).
    pub fn group_topics(&self, g: &GroupSummary) -> Vec<String> {
        let mut t = g.topics.clone();
        for p in self.lags.get(&g.name).into_iter().flatten() {
            if !t.contains(&p.topic) {
                t.push(p.topic.clone());
            }
        }
        t.sort();
        t
    }

    /// Groups reading `topic`, with their lag on it (None = not loaded yet,
    /// or a live member with no committed offset).
    pub fn consumers_of(&self, topic: &str) -> Vec<(&GroupSummary, Option<i64>)> {
        self.groups
            .iter()
            .filter_map(|g| {
                let parts: Vec<_> = self
                    .lags
                    .get(&g.name)
                    .into_iter()
                    .flatten()
                    .filter(|p| p.topic == topic)
                    .collect();
                if parts.is_empty() {
                    g.topics.iter().any(|t| t == topic).then_some((g, None))
                } else {
                    Some((g, Some(parts.iter().map(|p| p.lag()).sum())))
                }
            })
            .collect()
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
                    "connected to {} · {} topics · {} broker(s)",
                    profile.name,
                    meta.len(),
                    brokers
                ));
                self.connected = Some(profile);
                self.connecting = None;
                self.meta = meta;
                self.topic_state
                    .select((!self.meta.is_empty()).then_some(0));
                self.screen = Screen::Main;
                self.status.clear();
                self.rebuild_detail();
                // Groups (and then their lag) load in the background so the
                // topic view can show who consumes it.
                self.load_groups();
            }
            Evt::ConnectFailed(e) => {
                self.connecting = None;
                self.log(format!("connect failed: {e}"));
                self.modal = Modal::Error(format!("connect failed: {e}"));
            }
            Evt::Topics(meta) => {
                self.meta = meta;
                let n = self.filtered_topics().len();
                if self.topic_state.selected().is_none_or(|s| s >= n) {
                    self.topic_state.select((n > 0).then_some(0));
                }
                self.rebuild_detail();
            }
            Evt::Watermarks { topic, marks } => {
                self.loading_watermarks = false;
                let total: i64 = marks.iter().map(|(_, low, high)| high - low).sum();
                let end: i64 = marks.iter().map(|(_, _, high)| high).sum();
                self.counts.insert(topic.clone(), total);
                let Some(d) = &mut self.detail else { return };
                if d.name != topic {
                    return;
                }
                for (id, low, high) in marks {
                    if let Some(p) = d.partitions.iter_mut().find(|p| p.id == id) {
                        p.low = low;
                        p.high = high;
                    }
                }
                d.watermarks_loaded = true;
                // Rate = new messages since the last poll / measured gap.
                if let Some((t, prev, at)) = &self.rate_last {
                    if *t == topic {
                        let secs = at.elapsed().as_secs_f64().max(0.001);
                        self.rate
                            .push(((end - prev).max(0) as f64 / secs).round() as u64);
                        if self.rate.len() > 60 {
                            self.rate.remove(0);
                        }
                    }
                }
                self.rate_last = Some((topic, end, Instant::now()));
            }
            Evt::Groups(groups) => {
                self.groups = groups;
                self.groups_loaded = true;
                self.loading_groups = false;
                let n = self.filtered_groups().len();
                if self.group_state.selected().is_none_or(|s| s >= n) {
                    self.group_state.select((n > 0).then_some(0));
                }
                // Lag loads one group at a time so other requests (offsets
                // for the selected topic, peek) interleave between groups.
                self.lag_queue = self.groups.iter().rev().map(|g| g.name.clone()).collect();
                self.next_lag();
            }
            Evt::GroupLag { group, parts } => {
                self.lags.insert(group, parts);
                self.next_lag();
            }
            Evt::TopicConfig { topic, entries } => {
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
            Evt::Peek { topic, records } => {
                self.peeking = false;
                self.status.clear();
                self.log(format!("{topic}: loaded {} messages", records.len()));
                self.modal = Modal::Peek {
                    topic,
                    records,
                    sel: 0,
                    scroll: 0,
                };
            }
            Evt::Ok(msg) => {
                self.status.clear();
                self.toast(ToastLevel::Success, msg);
            }
            Evt::Failed(e) => {
                self.loading_watermarks = false;
                self.loading_groups = false;
                self.peeking = false;
                // Non-blocking: operation failures pop a toast, not a modal.
                self.status.clear();
                self.toast(ToastLevel::Error, e);
            }
        }
    }

    fn next_lag(&mut self) {
        if let Some(g) = self.lag_queue.pop() {
            self.worker.send(Cmd::GroupLag(g));
        }
    }

    fn load_groups(&mut self) {
        if self.loading_groups {
            return;
        }
        self.loading_groups = true;
        self.lag_queue.clear();
        self.worker.send(Cmd::Groups);
    }

    fn rebuild_detail(&mut self) {
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
        // Same topic, same shape (e.g. a refresh): keep loaded data and rate.
        if self
            .detail
            .as_ref()
            .is_some_and(|d| d.name == name && d.partitions.len() == t.partitions.len())
        {
            return;
        }
        self.detail_scroll = 0;
        self.loading_watermarks = false;
        self.selected_at = Instant::now();
        self.rate.clear();
        self.detail = Some(TopicDetail {
            name: name.clone(),
            partitions: t.partitions.clone(),
            watermarks_loaded: false,
        });
        self.topic_config = None;
        self.worker.send(Cmd::TopicConfig(name));
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
        self.topic_config = None;
        self.detail_scroll = 0;
        self.loading_watermarks = false;
        self.counts.clear();
        self.rate.clear();
        self.rate_last = None;
        self.groups.clear();
        self.group_state.select(None);
        self.groups_loaded = false;
        self.loading_groups = false;
        self.lags.clear();
        self.lag_queue.clear();
        self.filter.clear();
        self.filtering = false;
        self.view = View::Topics;
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
                self.toast(ToastLevel::Info, format!("already on {name}"));
            }
            return;
        }
        self.env_state.select(Some(idx));
        self.start_connect();
    }

    fn refresh(&mut self) {
        self.status = "refreshing…".into();
        self.worker.send(Cmd::RefreshTopics);
        self.load_groups();
    }

    fn peek(&mut self) {
        if self.peeking {
            return;
        }
        let Some(name) = self.selected_topic_name() else {
            return;
        };
        self.peeking = true;
        self.status = format!("loading messages from {name}…");
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

    /// Move the active tab's selection by `delta` (clamped).
    fn nav(&mut self, delta: isize) {
        match self.view {
            View::Topics => {
                let len = self.filtered_topics().len();
                let n = Self::next_index(self.topic_state.selected(), len, delta);
                self.topic_state.select(n);
                self.rebuild_detail(); // instant - from cache, no network
            }
            View::Groups => {
                let len = self.filtered_groups().len();
                let n = Self::next_index(self.group_state.selected(), len, delta);
                self.group_state.select(n);
                self.detail_scroll = 0;
            }
        }
    }

    fn set_view(&mut self, view: View) {
        if self.view == view {
            return;
        }
        self.view = view;
        self.filter.clear();
        self.filtering = false;
        self.detail_scroll = 0;
        if view == View::Topics {
            let n = self.filtered_topics().len();
            if self.topic_state.selected().is_none_or(|s| s >= n) {
                self.topic_state.select((n > 0).then_some(0));
            }
            self.rebuild_detail();
        } else if self.group_state.selected().is_none() && !self.groups.is_empty() {
            self.group_state.select(Some(0));
        }
    }

    /// Change the filter text, keeping the selection on the same item while
    /// it still matches (else the first match).
    fn edit_filter(&mut self, edit: impl FnOnce(&mut String)) {
        match self.view {
            View::Topics => {
                let prev = self.selected_topic_name();
                edit(&mut self.filter);
                let visible = self.filtered_topics();
                let i = visible
                    .iter()
                    .position(|&m| Some(&self.meta[m].name) == prev.as_ref());
                self.topic_state
                    .select(i.or((!visible.is_empty()).then_some(0)));
                self.rebuild_detail();
            }
            View::Groups => {
                let prev = self.selected_group().map(|g| g.name.clone());
                edit(&mut self.filter);
                let visible = self.filtered_groups();
                let i = visible
                    .iter()
                    .position(|&g| Some(&self.groups[g].name) == prev.as_ref());
                self.group_state
                    .select(i.or((!visible.is_empty()).then_some(0)));
            }
        }
    }

    // ── Input ──────────────────────────────────────────────────────────

    pub fn on_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> Result<()> {
        use ratatui::crossterm::event::KeyCode::*;

        // Ctrl-C always quits: raw mode swallows SIGINT, and in the filter it
        // would otherwise type a 'c'.
        if key.code == Char('c')
            && key
                .modifiers
                .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
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
                    self.edit_filter(String::clear);
                }
                Enter | Down | Up => {
                    self.filtering = false;
                    // ↑↓ leave the filter and move in one go.
                    if key.code != Enter {
                        self.nav(if key.code == Down { 1 } else { -1 });
                    }
                }
                Backspace => self.edit_filter(|f| {
                    f.pop();
                }),
                Char(c) => self.edit_filter(|f| f.push(c)),
                _ => {}
            }
            return Ok(());
        }

        if self.screen == Screen::EnvSelect {
            match key.code {
                Char('q') => self.should_quit = true,
                Up | Char('k') => {
                    let n = Self::next_index(self.env_state.selected(), self.config.envs.len(), -1);
                    self.env_state.select(n);
                }
                Down | Char('j') => {
                    let n = Self::next_index(self.env_state.selected(), self.config.envs.len(), 1);
                    self.env_state.select(n);
                }
                Enter => match self.env_state.selected() {
                    Some(i) if self.connected.is_some() => self.switch_env(i),
                    _ => self.start_connect(),
                },
                Esc if self.connected.is_some() => self.screen = Screen::Main,
                Char(c @ '1'..='9') => self.switch_env((c as u8 - b'1') as usize),
                Char('?') => self.modal = Modal::Help,
                _ => {}
            }
            return Ok(());
        }

        let page = 10;
        match (self.view, key.code) {
            (_, Char('q')) => self.should_quit = true,
            (_, Char('?')) => self.modal = Modal::Help,
            (_, Char('x')) => self.open_actions(),
            (_, Char('L')) => self.modal = Modal::Logs(0),

            (_, Tab_ | BackTab) => self.set_view(match self.view {
                View::Topics => View::Groups,
                View::Groups => View::Topics,
            }),
            (_, Char('t')) => self.set_view(View::Topics),
            (_, Char('G')) => self.set_view(View::Groups),

            (_, Up | Char('k')) => self.nav(-1),
            (_, Down | Char('j')) => self.nav(1),
            (_, Home | Char('g')) => self.nav(isize::MIN / 2),
            (_, End) => self.nav(isize::MAX / 2),
            (_, PageDown | Char(' ')) => {
                self.detail_scroll = self.detail_scroll.saturating_add(page)
            }
            (_, PageUp) => self.detail_scroll = self.detail_scroll.saturating_sub(page),

            (_, Char('/')) => {
                self.filtering = true;
                self.edit_filter(String::clear);
            }
            (_, Esc) if !self.filter.is_empty() => self.edit_filter(String::clear),
            (_, Char('r')) => self.refresh(),
            (_, Char('e')) => {
                self.env_state
                    .select(Some(self.current_env_index().unwrap_or(0)));
                self.screen = Screen::EnvSelect;
            }
            (_, Char(c @ '1'..='9')) => self.switch_env((c as u8 - b'1') as usize),

            // ── Topics ──
            (View::Topics, Enter | Char('m') | Char('p')) => self.peek(),
            (View::Topics, Char('c')) => {
                self.modal = Modal::Create(CreateForm {
                    partitions: "1".into(),
                    // 3 is the usual default, but it fails outright on smaller
                    // clusters (MSK stag is often 2 brokers).
                    replication: self.brokers.clamp(1, 3).to_string(),
                    ..Default::default()
                });
            }
            (View::Topics, Char('d')) => self.open_delete(),
            (View::Topics, Char('a')) => self.open_add_partitions(),
            (View::Topics, Char('y')) => {
                if let Some(name) = self.selected_topic_name() {
                    self.copy(&name, "topic name");
                }
            }

            // ── Consumer groups ──
            (View::Groups, Char('d')) => self.open_delete_group(),
            (View::Groups, Char('y')) => {
                if let Some(name) = self.selected_group().map(|g| g.name.clone()) {
                    self.copy(&name, "group name");
                }
            }
            // Jump to the topic this group reads (first one).
            (View::Groups, Enter) => {
                let topic = self
                    .selected_group()
                    .and_then(|g| self.group_topics(g).into_iter().next());
                if let Some(t) = topic {
                    self.set_view(View::Topics);
                    if let Some(i) = self
                        .filtered_topics()
                        .iter()
                        .position(|&m| self.meta[m].name == t)
                    {
                        self.topic_state.select(Some(i));
                        self.rebuild_detail();
                    }
                }
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
        if let Some(name) = self.selected_group().map(|g| g.name.clone()) {
            self.modal = Modal::Delete(DeleteForm {
                kind: DeleteKind::Group,
                target: name,
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
        let items: Vec<(char, &'static str)> = match self.view {
            View::Groups => vec![
                ('d', "delete selected group"),
                ('y', "copy group name"),
                ('/', "filter groups"),
                ('t', "go to topics"),
                ('r', "refresh"),
                ('e', "switch environment"),
                ('L', "activity log"),
            ],
            View::Topics => vec![
                ('m', "view latest messages"),
                ('/', "filter topics"),
                ('c', "create topic"),
                ('a', "add partitions"),
                ('d', "delete topic"),
                ('y', "copy topic name"),
                ('G', "go to consumer groups"),
                ('r', "refresh"),
                ('e', "switch environment"),
                ('L', "activity log"),
            ],
        };
        self.modal = Modal::Actions { items, sel: 0 };
    }

    fn on_modal_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        use ratatui::crossterm::event::KeyCode::*;

        let modal = std::mem::replace(&mut self.modal, Modal::None);
        match modal {
            Modal::Error(_) | Modal::Help | Modal::None => { /* any key dismisses */ }

            Modal::Logs(mut back) => {
                let max = self.logs.len() as u16;
                match key.code {
                    Up | Char('k') => back = (back + 1).min(max),
                    Down | Char('j') => back = back.saturating_sub(1),
                    PageUp => back = (back + 10).min(max),
                    PageDown => back = back.saturating_sub(10),
                    _ => return, // anything else closes
                }
                self.modal = Modal::Logs(back);
            }

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
                topic,
                records,
                mut sel,
                mut scroll,
            } => {
                match key.code {
                    Esc | Char('q') => return, // modal already cleared → closes
                    Char('r') => {
                        // Reload: close and fetch again (the new list replaces it).
                        self.peeking = true;
                        self.status = format!("loading messages from {topic}…");
                        self.worker.send(Cmd::Peek(topic.clone()));
                    }
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
                    topic,
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
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

/// Clipboard: `pbcopy` on macOS; elsewhere the OSC 52 terminal escape, which
/// also works over SSH (iTerm2, kitty, WezTerm, Windows Terminal, tmux with
/// `set-clipboard on`) - the usual way kitz is run on a bastion.
fn copy_to_clipboard(text: &str) -> std::result::Result<(), String> {
    use std::io::Write;
    if cfg!(target_os = "macos") {
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("pbcopy: {e}"))?;
        child
            .stdin
            .take()
            .ok_or("pbcopy: no stdin")?
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
        return match child.wait() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("pbcopy exited with {s}")),
            Err(e) => Err(e.to_string()),
        };
    }
    let mut out = std::io::stdout();
    write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()))
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())
}

/// Standard base64 (for OSC 52); std has none and it's 15 lines.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            s.push(if i <= c.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

// crossterm's KeyCode::Tab collides with our `Tab_` usage in match arms after
// the `use KeyCode::*` glob; alias it. (BackTab comes from the glob.)
use ratatui::crossterm::event::KeyCode::Tab as Tab_;

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (i, o) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
        ] {
            assert_eq!(super::base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn local_time_formats_epoch_ms() {
        let s = super::local_time(1_791_020_372_134, true);
        assert!(s.starts_with("2026-10-0"), "{s}");
        assert_eq!(s.len(), "2026-10-03 09:39:32".len());
    }
}
