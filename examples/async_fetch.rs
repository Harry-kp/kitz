use std::io::Read as _;
use std::net::TcpStream;

use rataframe::prelude::*;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

struct App {
    state: FetchState,
}

#[derive(Debug, Clone)]
enum FetchState {
    Idle,
    Loading,
    Success(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum Msg {
    StartFetch,
    FetchDone(Result<String, String>),
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::StartFetch => {
                self.state = FetchState::Loading;
                Command::perform(fetch_httpbin, Msg::FetchDone)
            }
            Msg::FetchDone(result) => {
                self.state = match result {
                    Ok(body) => FetchState::Success(body),
                    Err(e) => FetchState::Error(e),
                };
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let block = Block::bordered()
            .title(" Async Fetch Demo ")
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = match &self.state {
            FetchState::Idle => vec![
                Line::raw(""),
                Line::from(Span::styled(
                    "  Press Enter to fetch data from httpbin.org",
                    Style::default().fg(Color::White),
                )),
                Line::raw(""),
                Line::styled("  Press q to quit", Style::default().fg(Color::DarkGray)),
            ],
            FetchState::Loading => vec![
                Line::raw(""),
                Line::from(Span::styled(
                    "  Loading...",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
            ],
            FetchState::Success(body) => {
                let mut lines = vec![
                    Line::raw(""),
                    Line::from(Span::styled(
                        "  Success! Response (first 500 chars):",
                        Style::default().fg(Color::Green),
                    )),
                    Line::raw(""),
                ];
                let preview: String = body.chars().take(500).collect();
                for line in preview.lines() {
                    lines.push(Line::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White),
                    ));
                }
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "  Press Enter to fetch again, q to quit",
                    Style::default().fg(Color::DarkGray),
                ));
                lines
            }
            FetchState::Error(e) => vec![
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  Error: {}", e),
                    Style::default().fg(Color::Red),
                )),
                Line::raw(""),
                Line::styled(
                    "  Press Enter to retry, q to quit",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
        };

        let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
        frame.render_widget(paragraph, inner);
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Enter {
                return EventResult::Message(Msg::StartFetch);
            }
        }
        EventResult::Ignored
    }
}

/// Simple HTTP GET using raw TCP (no external HTTP crate needed for the demo).
fn fetch_httpbin() -> Result<String, String> {
    let mut stream = TcpStream::connect("httpbin.org:80").map_err(|e| format!("connect: {}", e))?;
    std::io::Write::write_all(
        &mut stream,
        b"GET /get HTTP/1.1\r\nHost: httpbin.org\r\nConnection: close\r\n\r\n",
    )
    .map_err(|e| format!("write: {}", e))?;

    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .map_err(|e| format!("read: {}", e))?;

    // Strip HTTP headers — return only the body
    if let Some(idx) = buf.find("\r\n\r\n") {
        Ok(buf[idx + 4..].to_string())
    } else {
        Ok(buf)
    }
}

fn main() -> Result<()> {
    rataframe::run(App {
        state: FetchState::Idle,
    })
}
