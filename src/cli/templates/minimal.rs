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
            content: r#"use rataframe::prelude::*;
use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

struct App {
    counter: i32,
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
            Msg::Increment => self.counter += 1,
            Msg::Decrement => self.counter -= 1,
            Msg::Reset => self.counter = 0,
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let lines = vec![
            Line::raw(""),
            Line::raw(format!("  Count: {}", self.counter)),
            Line::raw(""),
            Line::styled(
                "  j: increment  k: decrement  r: reset  q: quit",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
        ];
        let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
        frame.render_widget(paragraph, area);
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') => return EventResult::Message(Msg::Increment),
                KeyCode::Char('k') => return EventResult::Message(Msg::Decrement),
                KeyCode::Char('r') => return EventResult::Message(Msg::Reset),
                _ => {}
            }
        }
        EventResult::Ignored
    }

    fn title(&self) -> &str {
        "{{project-name}}"
    }
}

fn main() -> Result<()> {
    rataframe::run(App { counter: 0 })
}
"#,
        },
    ]
}
