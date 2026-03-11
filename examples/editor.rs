use rataframe::prelude::*;
use ratatui::layout::{Alignment, Direction};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

/// Proves the escape hatch: a modal editor using custom view(), no panels.
/// - Normal mode: hjkl navigation, i to insert, x to delete char
/// - Insert mode: type freely, Esc returns to normal
/// - EventResult::Consumed suppresses convention keys (q) in insert mode
struct App {
    lines: Vec<TextInputState>,
    cursor_row: usize,
    mode: Mode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
}

#[derive(Debug, Clone)]
enum Msg {
    EnterInsert,
    ExitInsert,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    InsertChar(char),
    Backspace,
    DeleteChar,
    NewLine,
    MoveHome,
    MoveEnd,
}

impl App {
    fn new() -> Self {
        Self {
            lines: vec![
                TextInputState::with_content("Welcome to the rataframe editor!"),
                TextInputState::with_content(""),
                TextInputState::with_content("This example proves the escape hatch:"),
                TextInputState::with_content("  - No panels, full custom view()"),
                TextInputState::with_content("  - Normal/Insert modes via app state"),
                TextInputState::with_content(
                    "  - EventResult::Consumed blocks 'q' quit in insert mode",
                ),
                TextInputState::with_content(""),
                TextInputState::with_content(
                    "Press i to enter insert mode, Esc to return to normal.",
                ),
            ],
            cursor_row: 0,
            mode: Mode::Normal,
        }
    }

    fn current_line(&self) -> &TextInputState {
        &self.lines[self.cursor_row]
    }
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::EnterInsert => self.mode = Mode::Insert,
            Msg::ExitInsert => self.mode = Mode::Normal,
            Msg::MoveUp => {
                self.cursor_row = self.cursor_row.saturating_sub(1);
            }
            Msg::MoveDown => {
                if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                }
            }
            Msg::MoveLeft => self.lines[self.cursor_row].move_left(),
            Msg::MoveRight => self.lines[self.cursor_row].move_right(),
            Msg::InsertChar(ch) => self.lines[self.cursor_row].insert_char(ch),
            Msg::Backspace => self.lines[self.cursor_row].delete_char_before(),
            Msg::DeleteChar => self.lines[self.cursor_row].delete_char_after(),
            Msg::NewLine => {
                self.lines
                    .insert(self.cursor_row + 1, TextInputState::new());
                self.cursor_row += 1;
            }
            Msg::MoveHome => self.lines[self.cursor_row].move_home(),
            Msg::MoveEnd => self.lines[self.cursor_row].move_end(),
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        };
        let mode_color = match self.mode {
            Mode::Normal => Color::Cyan,
            Mode::Insert => Color::Green,
        };

        let block = Block::bordered()
            .title(format!(" Editor — {} ", mode_str))
            .border_style(Style::default().fg(mode_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner into editor area and status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let editor_area = chunks[0];
        let status_area = chunks[1];

        // Render lines
        for (i, line_state) in self.lines.iter().enumerate() {
            if i as u16 >= editor_area.height {
                break;
            }
            let line_area = ratatui::layout::Rect {
                x: editor_area.x,
                y: editor_area.y + i as u16,
                width: editor_area.width,
                height: 1,
            };

            let is_current = i == self.cursor_row;
            let show_cursor = is_current && self.mode == Mode::Insert;

            // Line number
            let line_num = format!("{:>3} ", i + 1);
            let num_style = if is_current {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            frame.render_widget(
                Paragraph::new(line_num).style(num_style),
                ratatui::layout::Rect {
                    x: line_area.x,
                    y: line_area.y,
                    width: 4,
                    height: 1,
                },
            );

            // Content
            let content_area = ratatui::layout::Rect {
                x: line_area.x + 4,
                y: line_area.y,
                width: line_area.width.saturating_sub(4),
                height: 1,
            };

            let text_style = if is_current {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let widget = TextInput::new(line_state)
                .style(text_style)
                .show_cursor(show_cursor);
            frame.render_widget(widget, content_area);

            // In normal mode, show a block cursor on the current line
            if is_current && self.mode == Mode::Normal {
                let content = line_state.content();
                let char_count = content.chars().count();
                let cursor_x = content_area.x
                    + content[..line_state.content().len().min(
                        content
                            .char_indices()
                            .nth(char_count.min(content.chars().count()))
                            .map(|(i, _)| i)
                            .unwrap_or(content.len()),
                    )]
                        .chars()
                        .count() as u16;
                // Simplified: just highlight at approximate position
                // The TextInput widget handles cursor rendering in insert mode
                let _ = cursor_x;
            }
        }

        // Status bar
        let hints = match self.mode {
            Mode::Normal => "h/j/k/l: move  i: insert  x: delete  q: quit",
            Mode::Insert => "Type to edit  Enter: new line  Esc: normal mode",
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", mode_str),
                Style::default()
                    .fg(Color::Black)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(
                    "Ln {}, Col {} ",
                    self.cursor_row + 1,
                    self.current_line().content().chars().count()
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("  "),
            Span::styled(hints, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(
            Paragraph::new(status_line).alignment(Alignment::Left),
            status_area,
        );
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match self.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('i') => return EventResult::Message(Msg::EnterInsert),
                    KeyCode::Char('h') | KeyCode::Left => {
                        return EventResult::Message(Msg::MoveLeft)
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        return EventResult::Message(Msg::MoveDown)
                    }
                    KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::MoveUp),
                    KeyCode::Char('l') | KeyCode::Right => {
                        return EventResult::Message(Msg::MoveRight)
                    }
                    KeyCode::Char('x') => return EventResult::Message(Msg::DeleteChar),
                    KeyCode::Char('0') | KeyCode::Home => {
                        return EventResult::Message(Msg::MoveHome)
                    }
                    KeyCode::Char('$') | KeyCode::End => return EventResult::Message(Msg::MoveEnd),
                    _ => return EventResult::Ignored,
                },
                Mode::Insert => {
                    // In insert mode, CONSUME everything to prevent convention
                    // keys (q, Tab, etc.) from triggering.
                    match key.code {
                        KeyCode::Esc => return EventResult::Message(Msg::ExitInsert),
                        KeyCode::Char(c) => return EventResult::Message(Msg::InsertChar(c)),
                        KeyCode::Backspace => return EventResult::Message(Msg::Backspace),
                        KeyCode::Delete => return EventResult::Message(Msg::DeleteChar),
                        KeyCode::Enter => return EventResult::Message(Msg::NewLine),
                        KeyCode::Left => return EventResult::Message(Msg::MoveLeft),
                        KeyCode::Right => return EventResult::Message(Msg::MoveRight),
                        KeyCode::Up => return EventResult::Message(Msg::MoveUp),
                        KeyCode::Down => return EventResult::Message(Msg::MoveDown),
                        KeyCode::Home => return EventResult::Message(Msg::MoveHome),
                        KeyCode::End => return EventResult::Message(Msg::MoveEnd),
                        _ => return EventResult::Consumed,
                    }
                }
            }
        }
        EventResult::Ignored
    }
}

fn main() -> Result<()> {
    rataframe::run(App::new())
}
