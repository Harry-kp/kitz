use super::TemplateFile;

pub fn files() -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "Cargo.toml",
            content: r#"[package]
name = "{{project-name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
rataframe = { version = "0.1", default-features = false }
ratatui = "0.30"
crossterm = "0.29"
color-eyre = "0.6"
"#,
        },
        TemplateFile {
            path: ".gitignore",
            content: "/target\nCargo.lock\n",
        },
        TemplateFile {
            path: "src/main.rs",
            content: r#"mod app;
mod messages;
mod panels;

use color_eyre::Result;

fn main() -> Result<()> {
    rataframe::run(app::App::new())
}
"#,
        },
        TemplateFile {
            path: "src/messages.rs",
            content: r#"#[derive(Debug, Clone)]
pub enum Msg {
    StatsRefresh(Vec<u64>),
    LogAppend(String),
    ChartTick,
    CycleTheme,
    // rataframe:messages
}
"#,
        },
        TemplateFile {
            path: "src/app.rs",
            content: r#"use std::time::Duration;

use rataframe::prelude::*;
use rataframe::toast::ToastLevel;

use crate::messages::Msg;
use crate::panels;

pub struct App {
    pub stats: panels::stats::StatsPanel,
    pub chart: panels::chart::ChartPanel,
    pub log: panels::log::LogPanel,
    // rataframe:app-fields
    pub theme: rataframe::theme::Theme,
}

impl App {
    pub fn new() -> Self {
        Self {
            stats: panels::stats::StatsPanel::new(),
            chart: panels::chart::ChartPanel::new(),
            log: panels::log::LogPanel::new(),
            // rataframe:app-init
            theme: rataframe::theme::Theme::default(),
        }
    }
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::StatsRefresh(values) => {
                self.stats.update_values(&values);
                if values.iter().any(|v| *v > 90) {
                    ctx.toast("High value detected!", ToastLevel::Warning);
                }
            }
            Msg::LogAppend(line) => self.log.append(line),
            Msg::ChartTick => self.chart.tick(),
            Msg::CycleTheme => {
                self.theme = self.theme.next();
                ctx.toast(format!("Theme: {}", self.theme.name), ToastLevel::Success);
            }
            // rataframe:update
        }
        Command::none()
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("stats", Constraint::Percentage(25)),
            ("chart", Constraint::Percentage(50)),
            ("log", Constraint::Percentage(25)),
            // rataframe:layout
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "stats" => panels::stats::PANEL_TITLE,
            "chart" => panels::chart::PANEL_TITLE,
            "log" => panels::log::PANEL_TITLE,
            // rataframe:panel-title
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
        let theme = &self.theme;
        match id {
            "stats" => self.stats.view(frame, area, focused, theme),
            "chart" => self.chart.view(frame, area, focused, theme),
            "log" => self.log.view(frame, area, focused, theme),
            // rataframe:panel-view
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "stats" => panels::stats::StatsPanel::key_hints(),
            "chart" => panels::chart::ChartPanel::key_hints(),
            "log" => panels::log::LogPanel::key_hints(),
            // rataframe:panel-hints
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<Msg> {
        match id {
            // rataframe:panel-keys
            _ => EventResult::Ignored,
        }
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('t') {
                return EventResult::Message(Msg::CycleTheme);
            }
        }
        EventResult::Ignored
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::every("chart-tick", Duration::from_secs(1), || Msg::ChartTick),
        ]
    }

    fn theme(&self) -> rataframe::theme::Theme {
        self.theme.clone()
    }

    fn title(&self) -> &str {
        "{{project-name}}"
    }
}
"#,
        },
        TemplateFile {
            path: "src/panels/mod.rs",
            content: r#"pub mod stats;
pub mod chart;
pub mod log;
// rataframe:panel-mods
"#,
        },
        TemplateFile {
            path: "src/panels/stats.rs",
            content: r#"use rataframe::prelude::*;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};

pub const PANEL_ID: PanelId = "stats";
pub const PANEL_TITLE: &str = "Stats";

pub struct StatsPanel {
    pub values: Vec<(String, u64)>,
}

impl StatsPanel {
    pub fn new() -> Self {
        Self {
            values: vec![
                ("CPU".into(), 42),
                ("Memory".into(), 67),
                ("Disk".into(), 23),
                ("Network".into(), 81),
            ],
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &rataframe::theme::Theme) {
        let items: Vec<ListItem> = self
            .values
            .iter()
            .map(|(name, val)| {
                let color = if *val > 80 {
                    theme.error
                } else if *val > 60 {
                    theme.warning
                } else {
                    theme.success
                };
                ListItem::new(Line::from(vec![
                    ratatui::text::Span::styled(
                        format!(" {:>8} ", name),
                        Style::default().fg(theme.text),
                    ),
                    ratatui::text::Span::styled(
                        format!("{}%", val),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ]))
            })
            .collect();
        frame.render_widget(List::new(items), area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![]
    }

    pub fn update_values(&mut self, new_vals: &[u64]) {
        for (i, val) in new_vals.iter().enumerate() {
            if let Some(entry) = self.values.get_mut(i) {
                entry.1 = *val;
            }
        }
    }
}
"#,
        },
        TemplateFile {
            path: "src/panels/chart.rs",
            content: r#"use rataframe::prelude::*;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub const PANEL_ID: PanelId = "chart";
pub const PANEL_TITLE: &str = "Chart";

pub struct ChartPanel {
    pub data_points: Vec<u64>,
    pub tick_count: u64,
}

impl ChartPanel {
    pub fn new() -> Self {
        Self {
            data_points: vec![20, 35, 50, 42, 60, 45, 70, 55, 80, 65],
            tick_count: 0,
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &rataframe::theme::Theme) {
        let max = *self.data_points.iter().max().unwrap_or(&1);
        let height = area.height.saturating_sub(1) as u64;

        let mut lines = Vec::new();
        for row in 0..area.height.saturating_sub(1) {
            let threshold = max - (row as u64 * max / height.max(1));
            let mut spans = Vec::new();
            for &val in &self.data_points {
                let ch = if val >= threshold { "█" } else { " " };
                spans.push(ratatui::text::Span::styled(
                    format!("{} ", ch),
                    Style::default().fg(theme.accent),
                ));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::styled(
            format!(" tick #{}", self.tick_count),
            Style::default().fg(theme.text_muted),
        ));

        frame.render_widget(Paragraph::new(lines), area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![]
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.data_points.rotate_left(1);
        if let Some(last) = self.data_points.last_mut() {
            *last = (*last + 7 + self.tick_count * 3) % 100;
        }
    }
}
"#,
        },
        TemplateFile {
            path: "src/panels/log.rs",
            content: r#"use rataframe::prelude::*;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub const PANEL_ID: PanelId = "log";
pub const PANEL_TITLE: &str = "Log";

pub struct LogPanel {
    pub entries: Vec<String>,
}

impl LogPanel {
    pub fn new() -> Self {
        Self {
            entries: vec![
                "App started".into(),
                "Loaded config".into(),
                "Ready".into(),
            ],
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &rataframe::theme::Theme) {
        let visible = area.height as usize;
        let start = self.entries.len().saturating_sub(visible);
        let lines: Vec<Line> = self.entries[start..]
            .iter()
            .map(|e| Line::styled(format!(" {}", e), Style::default().fg(theme.text_muted)))
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![]
    }

    pub fn append(&mut self, line: String) {
        self.entries.push(line);
        if self.entries.len() > 100 {
            self.entries.remove(0);
        }
    }
}
"#,
        },
        TemplateFile {
            path: "tests/app_test.rs",
            content: r#"// TestHarness-based tests. Run with: rataframe test
// rataframe:tests
"#,
        },
    ]
}
