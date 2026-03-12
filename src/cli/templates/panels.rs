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
kitz = { version = "0.1", default-features = false }
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
    kitz::run(app::App::new())
}
"#,
        },
        TemplateFile {
            path: "src/messages.rs",
            content: r#"#[derive(Debug, Clone)]
pub enum Msg {
    SidebarNext,
    SidebarPrev,
    DetailUpdate(String),
    // kitz:messages
}
"#,
        },
        TemplateFile {
            path: "src/app.rs",
            content: r#"use kitz::prelude::*;

use crate::messages::Msg;
use crate::panels;

pub struct App {
    pub sidebar: panels::sidebar::SidebarPanel,
    pub detail: panels::detail::DetailPanel,
    // kitz:app-fields
    pub theme: kitz::theme::Theme,
}

impl App {
    pub fn new() -> Self {
        Self {
            sidebar: panels::sidebar::SidebarPanel::new(),
            detail: panels::detail::DetailPanel::new(),
            // kitz:app-init
            theme: kitz::theme::Theme::default(),
        }
    }
}

impl Application for App {
    type Message = Msg;

    #[allow(unused_variables)]
    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SidebarNext => self.sidebar.select_next(),
            Msg::SidebarPrev => self.sidebar.select_prev(),
            Msg::DetailUpdate(text) => self.detail.set_content(text),
            // kitz:update
        }
        Command::none()
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("detail", Constraint::Percentage(70)),
            // kitz:layout
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => panels::sidebar::PANEL_TITLE,
            "detail" => panels::detail::PANEL_TITLE,
            // kitz:panel-title
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
        let theme = &self.theme;
        match id {
            "sidebar" => self.sidebar.view(frame, area, focused, theme),
            "detail" => self.detail.view(frame, area, focused, theme),
            // kitz:panel-view
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => panels::sidebar::SidebarPanel::key_hints(),
            "detail" => panels::detail::DetailPanel::key_hints(),
            // kitz:panel-hints
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<Msg> {
        match id {
            "sidebar" => match key.code {
                KeyCode::Char('j') | KeyCode::Down => EventResult::Message(Msg::SidebarNext),
                KeyCode::Char('k') | KeyCode::Up => EventResult::Message(Msg::SidebarPrev),
                _ => EventResult::Ignored,
            },
            // kitz:panel-keys
            _ => EventResult::Ignored,
        }
    }

    fn theme(&self) -> kitz::theme::Theme {
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
            content: r#"pub mod sidebar;
pub mod detail;
// kitz:panel-mods
"#,
        },
        TemplateFile {
            path: "src/panels/sidebar.rs",
            content: r#"use kitz::prelude::*;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};

pub const PANEL_ID: PanelId = "sidebar";
pub const PANEL_TITLE: &str = "Sidebar";

pub struct SidebarPanel {
    pub items: Vec<String>,
    pub selected: usize,
}

impl SidebarPanel {
    pub fn new() -> Self {
        Self {
            items: vec![
                "Item One".into(),
                "Item Two".into(),
                "Item Three".into(),
            ],
            selected: 0,
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &kitz::theme::Theme) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::styled(format!(" {} ", item), style))
            })
            .collect();
        frame.render_widget(List::new(items), area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![
            KeyHint::new("j/k", "Navigate"),
        ]
    }

    pub fn select_next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}
"#,
        },
        TemplateFile {
            path: "src/panels/detail.rs",
            content: r#"use kitz::prelude::*;
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Wrap};

pub const PANEL_ID: PanelId = "detail";
pub const PANEL_TITLE: &str = "Detail";

pub struct DetailPanel {
    pub content: String,
}

impl DetailPanel {
    pub fn new() -> Self {
        Self {
            content: "Select an item from the sidebar.".into(),
        }
    }

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &kitz::theme::Theme) {
        let para = Paragraph::new(format!(" {}", self.content))
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![]
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
}
"#,
        },
        TemplateFile {
            path: "tests/app_test.rs",
            content: r#"use kitz::prelude::*;

#[path = "../src/app.rs"]
#[allow(dead_code)]
mod app_mod;

#[path = "../src/messages.rs"]
#[allow(dead_code)]
mod messages_mod;

#[path = "../src/panels/mod.rs"]
#[allow(dead_code, unused_imports)]
mod panels_mod;

// Note: TestHarness-based tests for your app.
// Run with: kitz test

// kitz:tests
"#,
        },
    ]
}
