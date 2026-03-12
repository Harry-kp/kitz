use rataframe::prelude::*;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

// -- State -------------------------------------------------------------------

struct App {
    items: Vec<String>,
    selected: usize,
}

impl App {
    fn new() -> Self {
        Self {
            items: vec![
                "Item One".into(),
                "Item Two".into(),
                "Item Three".into(),
            ],
            selected: 0,
        }
    }
}

// -- Messages ----------------------------------------------------------------

#[derive(Debug, Clone)]
enum Msg {
    SelectNext,
    SelectPrev,
}

// -- Application -------------------------------------------------------------

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SelectNext => {
                if self.selected < self.items.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            Msg::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        Command::none()
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("detail", Constraint::Percentage(70)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Items",
            "detail" => "Detail",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        match id {
            "sidebar" => {
                let items: Vec<ListItem> = self
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let style = if i == self.selected {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default()
                        };
                        ListItem::new(Line::styled(format!(" {} ", item), style))
                    })
                    .collect();
                frame.render_widget(List::new(items), area);
            }
            "detail" => {
                let text = self
                    .items
                    .get(self.selected)
                    .map(|s| s.as_str())
                    .unwrap_or("Nothing selected");
                frame.render_widget(Paragraph::new(format!(" Selected: {}", text)), area);
            }
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => vec![
                KeyHint::new("j/↓", "Next"),
                KeyHint::new("k/↑", "Prev"),
            ],
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<Msg> {
        if id == "sidebar" {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return EventResult::Message(Msg::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::SelectPrev),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

// -- Entry point -------------------------------------------------------------

fn main() -> Result<()> {
    rataframe::run(App::new())
}
