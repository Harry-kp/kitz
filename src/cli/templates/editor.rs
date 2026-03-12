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
            content: r#"use kitz::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
}

struct App {
    lines: Vec<TextInputState>,
    cursor_line: usize,
    mode: Mode,
}

#[derive(Debug, Clone)]
enum Msg {
    EnterInsert,
    ExitInsert,
    InsertChar(char),
    Backspace,
    CursorUp,
    CursorDown,
    NewLine,
}

impl App {
    fn new() -> Self {
        Self {
            lines: vec![
                TextInputState::with_content("Welcome to the editor."),
                TextInputState::with_content("Press 'i' to enter insert mode."),
                TextInputState::with_content("Press Esc to return to normal mode."),
                TextInputState::new(),
            ],
            cursor_line: 0,
            mode: Mode::Normal,
        }
    }

    fn current_line(&self) -> &TextInputState {
        &self.lines[self.cursor_line]
    }

    fn current_line_mut(&mut self) -> &mut TextInputState {
        &mut self.lines[self.cursor_line]
    }
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::EnterInsert => self.mode = Mode::Insert,
            Msg::ExitInsert => self.mode = Mode::Normal,
            Msg::InsertChar(c) => self.current_line_mut().insert_char(c),
            Msg::Backspace => self.current_line_mut().delete_char_before(),
            Msg::CursorUp => {
                self.cursor_line = self.cursor_line.saturating_sub(1);
            }
            Msg::CursorDown => {
                if self.cursor_line < self.lines.len() - 1 {
                    self.cursor_line += 1;
                }
            }
            Msg::NewLine => {
                self.cursor_line += 1;
                self.lines.insert(self.cursor_line, TextInputState::new());
            }
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let mut text_lines: Vec<Line> = Vec::new();

        for (i, line) in self.lines.iter().enumerate() {
            let is_current = i == self.cursor_line;
            let number = Span::styled(
                format!("{:>3} ", i + 1),
                Style::default().fg(if is_current {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            );
            let content = Span::styled(
                line.content().to_string(),
                Style::default().fg(Color::White),
            );
            text_lines.push(Line::from(vec![number, content]));
        }

        let mode_indicator = match self.mode {
            Mode::Normal => Span::styled(
                " NORMAL ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Mode::Insert => Span::styled(
                " INSERT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        };
        text_lines.push(Line::raw(""));
        text_lines.push(Line::from(mode_indicator));

        frame.render_widget(Paragraph::new(text_lines), area);

        if self.mode == Mode::Insert {
            let x = area.x + 4 + self.current_line().content().len() as u16;
            let y = area.y + self.cursor_line as u16;
            frame.set_cursor_position(ratatui::layout::Position::new(x, y));
        }
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match self.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('i') => return EventResult::Message(Msg::EnterInsert),
                    KeyCode::Char('j') | KeyCode::Down => {
                        return EventResult::Message(Msg::CursorDown)
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        return EventResult::Message(Msg::CursorUp)
                    }
                    KeyCode::Char('o') => return EventResult::Message(Msg::NewLine),
                    _ => {}
                },
                Mode::Insert => match key.code {
                    KeyCode::Esc => return EventResult::Message(Msg::ExitInsert),
                    KeyCode::Char(c) => return EventResult::Message(Msg::InsertChar(c)),
                    KeyCode::Backspace => return EventResult::Message(Msg::Backspace),
                    KeyCode::Enter => return EventResult::Message(Msg::NewLine),
                    _ => {}
                },
            }
        }
        if self.mode == Mode::Insert {
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }

    fn title(&self) -> &str {
        "{{project-name}}"
    }
}

fn main() -> Result<()> {
    kitz::run(App::new())
}
"#,
        },
    ]
}
