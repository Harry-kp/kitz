use kitz::prelude::*;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

struct App {
    count: i32,
}

#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
    Reset,
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
            Msg::Reset => self.count = 0,
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let block = Block::default()
            .title(" Counter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let text = vec![
            Line::raw(""),
            Line::raw(format!("  Count: {}", self.count)),
            Line::raw(""),
            Line::styled(
                "  j/k: inc/dec  r: reset  q: quit",
                Style::default().fg(Color::DarkGray),
            ),
        ];

        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
        frame.render_widget(paragraph, area);
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return EventResult::Message(Msg::Increment),
                KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::Decrement),
                KeyCode::Char('r') => return EventResult::Message(Msg::Reset),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

fn main() -> Result<()> {
    kitz::run(App { count: 0 })
}
