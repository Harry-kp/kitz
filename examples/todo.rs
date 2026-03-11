use rataframe::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

struct TodoApp {
    todos: Vec<Todo>,
    selected: usize,
}

struct Todo {
    title: String,
    done: bool,
}

#[derive(Debug, Clone)]
enum Msg {
    SelectNext,
    SelectPrev,
    ToggleDone,
    AddSample,
    ConfirmDelete,
    DeleteSelected,
}

impl TodoApp {
    fn new() -> Self {
        Self {
            todos: vec![
                Todo {
                    title: "Learn rataframe".into(),
                    done: false,
                },
                Todo {
                    title: "Build a TUI app".into(),
                    done: false,
                },
                Todo {
                    title: "Ship to crates.io".into(),
                    done: false,
                },
            ],
            selected: 0,
        }
    }

    fn selected_todo(&self) -> Option<&Todo> {
        self.todos.get(self.selected)
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect, _focused: bool) {
        let items: Vec<ListItem> = self
            .todos
            .iter()
            .enumerate()
            .map(|(i, todo)| {
                let marker = if todo.done { "[x]" } else { "[ ]" };
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if todo.done {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::styled(format!(" {} {}", marker, todo.title), style))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, area);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, _focused: bool) {
        if let Some(todo) = self.selected_todo() {
            let status = if todo.done { "Done" } else { "Pending" };
            let status_color = if todo.done {
                Color::Green
            } else {
                Color::Yellow
            };
            let lines = vec![
                Line::raw(""),
                Line::from(vec![
                    Span::styled("  Title: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &todo.title,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(status, Style::default().fg(status_color)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("  Index: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{} of {}", self.selected + 1, self.todos.len()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), area);
        } else {
            frame.render_widget(
                Paragraph::new("  No todos yet. Press 'a' to add one."),
                area,
            );
        }
    }
}

impl Application for TodoApp {
    type Message = Msg;

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(35)),
            ("detail", Constraint::Percentage(65)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Todos",
            "detail" => "Details",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
        match id {
            "sidebar" => self.render_sidebar(frame, area, focused),
            "detail" => self.render_detail(frame, area, focused),
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => vec![
                KeyHint::new("j/k", "Navigate"),
                KeyHint::new("Space", "Toggle done"),
                KeyHint::new("a", "Add sample"),
                KeyHint::new("d", "Delete"),
            ],
            "detail" => vec![],
            _ => vec![],
        }
    }

    fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        if id == "sidebar" {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return EventResult::Message(Msg::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::SelectPrev),
                KeyCode::Char(' ') => return EventResult::Message(Msg::ToggleDone),
                KeyCode::Char('a') => return EventResult::Message(Msg::AddSample),
                KeyCode::Char('d') => return EventResult::Message(Msg::ConfirmDelete),
                _ => {}
            }
        }
        EventResult::Ignored
    }

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SelectNext => {
                if !self.todos.is_empty() {
                    self.selected = (self.selected + 1).min(self.todos.len() - 1);
                }
            }
            Msg::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
            }
            Msg::ToggleDone => {
                if let Some(todo) = self.todos.get_mut(self.selected) {
                    todo.done = !todo.done;
                }
            }
            Msg::AddSample => {
                let n = self.todos.len() + 1;
                self.todos.push(Todo {
                    title: format!("New todo #{}", n),
                    done: false,
                });
            }
            Msg::ConfirmDelete => {
                if let Some(todo) = self.selected_todo() {
                    let title = todo.title.clone();
                    ctx.push_overlay(ConfirmOverlay::new(
                        "Delete Todo",
                        format!("Delete '{}'?", title),
                        Msg::DeleteSelected,
                    ));
                }
            }
            Msg::DeleteSelected => {
                if !self.todos.is_empty() {
                    self.todos.remove(self.selected);
                    if self.selected >= self.todos.len() && self.selected > 0 {
                        self.selected -= 1;
                    }
                }
            }
        }
        Command::none()
    }
}

fn main() -> Result<()> {
    rataframe::run(TodoApp::new())
}
