use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use kitz::prelude::*;
use kitz::toast::ToastLevel;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

const SIDEBAR: PanelId = "files";
const PREVIEW: PanelId = "preview";

// -- State -------------------------------------------------------------------

struct App {
    cwd: PathBuf,
    entries: Vec<DirEntry>,
    selected: usize,
    preview: String,
    current_theme: kitz::theme::Theme,
    show_hidden: bool,
}

struct DirEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

impl App {
    fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
            preview: String::new(),
            current_theme: kitz::theme::Theme::default(),
            show_hidden: false,
        };
        app.reload_entries();
        app
    }

    fn reload_entries(&mut self) {
        let mut entries = Vec::new();

        if let Ok(read_dir) = fs::read_dir(&self.cwd) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                let meta = entry.metadata().ok();
                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                entries.push(DirEntry { name, is_dir, size });
            }
        }

        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

        self.entries = entries;
        self.selected = 0;
        self.update_preview();
    }

    fn update_preview(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let path = self.cwd.join(&entry.name);
            if entry.is_dir {
                match fs::read_dir(&path) {
                    Ok(rd) => {
                        let count = rd.count();
                        self.preview = format!(
                            "Directory: {}\n{} items\n\nPress Enter to open",
                            path.display(),
                            count
                        );
                    }
                    Err(e) => self.preview = format!("Cannot read: {}", e),
                }
            } else {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        self.preview = content.chars().take(2000).collect();
                    }
                    Err(_) => {
                        self.preview = format!(
                            "Binary file: {}\nSize: {} bytes",
                            entry.name,
                            format_size(entry.size)
                        );
                    }
                }
            }
        } else {
            self.preview = "Empty directory".into();
        }
    }

    fn selected_entry(&self) -> Option<&DirEntry> {
        self.entries.get(self.selected)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// -- Messages ----------------------------------------------------------------

#[derive(Debug, Clone)]
enum Msg {
    SelectNext,
    SelectPrev,
    Enter,
    GoUp,
    ToggleHidden,
    CycleTheme,
    ConfirmDelete,
    DoDelete,
    Tick,
}

// -- Application -------------------------------------------------------------

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SelectNext => {
                if self.selected < self.entries.len().saturating_sub(1) {
                    self.selected += 1;
                    self.update_preview();
                }
            }
            Msg::SelectPrev => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.update_preview();
                }
            }
            Msg::Enter => {
                if let Some(entry) = self.selected_entry() {
                    if entry.is_dir {
                        self.cwd = self.cwd.join(&entry.name);
                        self.reload_entries();
                    }
                }
            }
            Msg::GoUp => {
                if let Some(parent) = self.cwd.parent() {
                    self.cwd = parent.to_path_buf();
                    self.reload_entries();
                }
            }
            Msg::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.reload_entries();
                ctx.toast(
                    if self.show_hidden {
                        "Showing hidden files"
                    } else {
                        "Hiding hidden files"
                    },
                    ToastLevel::Info,
                );
            }
            Msg::CycleTheme => {
                self.current_theme = self.current_theme.next();
                ctx.toast(
                    format!("Theme: {}", self.current_theme.name),
                    ToastLevel::Success,
                );
            }
            Msg::ConfirmDelete => {
                if let Some(entry) = self.selected_entry() {
                    ctx.push_overlay(ConfirmOverlay::new(
                        "Delete",
                        format!("Delete '{}'?", entry.name),
                        Msg::DoDelete,
                    ));
                }
            }
            Msg::DoDelete => {
                if let Some(entry) = self.selected_entry() {
                    let path = self.cwd.join(&entry.name);
                    let result = if entry.is_dir {
                        fs::remove_dir_all(&path)
                    } else {
                        fs::remove_file(&path)
                    };
                    match result {
                        Ok(_) => {
                            ctx.toast("Deleted", ToastLevel::Success);
                            self.reload_entries();
                        }
                        Err(e) => {
                            ctx.toast(format!("Failed: {}", e), ToastLevel::Error);
                        }
                    }
                }
            }
            Msg::Tick => {
                // Silently refresh entries in case files changed
            }
        }
        Command::none()
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            (SIDEBAR, Constraint::Percentage(40)),
            (PREVIEW, Constraint::Percentage(60)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            SIDEBAR => "Files",
            PREVIEW => "Preview",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        let theme = &self.current_theme;
        match id {
            SIDEBAR => {
                // Path header
                let cwd_display = self.cwd.display().to_string();
                let path_line = Line::styled(
                    format!(" {} ", cwd_display),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                );

                let items: Vec<ListItem> = self
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let icon = if entry.is_dir { "📁" } else { "  " };
                        let size_str = if entry.is_dir {
                            String::new()
                        } else {
                            format!("  {}", format_size(entry.size))
                        };
                        let style = if i == self.selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(theme.accent)
                                .add_modifier(Modifier::BOLD)
                        } else if entry.is_dir {
                            Style::default().fg(theme.accent)
                        } else {
                            Style::default().fg(theme.text)
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(format!(" {} {} ", icon, entry.name), style),
                            Span::styled(size_str, Style::default().fg(theme.text_muted)),
                        ]))
                    })
                    .collect();

                let inner = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        ratatui::layout::Constraint::Length(1),
                        ratatui::layout::Constraint::Min(1),
                    ])
                    .split(area);

                frame.render_widget(Paragraph::new(path_line), inner[0]);
                frame.render_widget(List::new(items), inner[1]);
            }
            PREVIEW => {
                let para = Paragraph::new(self.preview.as_str())
                    .style(Style::default().fg(theme.text))
                    .wrap(Wrap { trim: false });
                frame.render_widget(para, area);
            }
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            SIDEBAR => vec![
                KeyHint::new("j/k", "Navigate"),
                KeyHint::new("Enter", "Open"),
                KeyHint::new("Backspace", "Parent"),
                KeyHint::new(".", "Toggle hidden"),
                KeyHint::new("d", "Delete"),
                KeyHint::new("t", "Theme"),
            ],
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<Msg> {
        if id == SIDEBAR {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return EventResult::Message(Msg::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::SelectPrev),
                KeyCode::Enter => return EventResult::Message(Msg::Enter),
                KeyCode::Backspace => return EventResult::Message(Msg::GoUp),
                KeyCode::Char('.') => return EventResult::Message(Msg::ToggleHidden),
                KeyCode::Char('d') => return EventResult::Message(Msg::ConfirmDelete),
                KeyCode::Char('t') => return EventResult::Message(Msg::CycleTheme),
                _ => {}
            }
        }
        EventResult::Ignored
    }

    fn theme(&self) -> kitz::theme::Theme {
        self.current_theme.clone()
    }

    fn title(&self) -> &str {
        "kitz File Manager"
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![Subscription::every(
            "refresh",
            Duration::from_secs(5),
            || Msg::Tick,
        )]
    }
}

fn main() -> Result<()> {
    kitz::run(App::new())
}
