use kitz::prelude::*;
use kitz::theme::palettes;
use ratatui::layout::{Alignment, Direction};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

struct App;

#[derive(Debug, Clone)]
enum Msg {}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, _msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let themes = palettes::all();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                themes
                    .iter()
                    .map(|_| Constraint::Ratio(1, themes.len() as u32))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        for (i, theme) in themes.iter().enumerate() {
            render_theme_column(frame, chunks[i], theme);
        }
    }

    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        EventResult::Ignored
    }
}

fn render_theme_column(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let block = Block::default()
        .title(format!(" {} ", theme.name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let entries: Vec<(&str, ratatui::style::Color)> = vec![
        ("text", theme.text),
        ("text_muted", theme.text_muted),
        ("border", theme.border),
        ("focused", theme.border_focused),
        ("accent", theme.accent),
        ("success", theme.success),
        ("warning", theme.warning),
        ("error", theme.error),
    ];

    for (i, (label, color)) in entries.iter().enumerate() {
        let line = Line::from(vec![
            Span::styled(
                format!(" {:>10} ", label),
                Style::default().fg(theme.text_muted),
            ),
            Span::styled(
                " ████ ",
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(line)
                .alignment(Alignment::Left)
                .style(Style::default().bg(theme.bg)),
            rows[i],
        );
    }

    frame.render_widget(
        Paragraph::new(Line::styled(
            " Press q to quit",
            Style::default().fg(theme.text_muted),
        ))
        .style(Style::default().bg(theme.bg)),
        rows[9],
    );
}

fn main() -> Result<()> {
    kitz::run(App)
}
