use std::fmt::Debug;

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{Overlay, OverlayResult};
use crate::panel::KeyHint;
use crate::theme::Theme;

/// Built-in Help overlay that displays all key hints grouped by section.
pub struct HelpOverlay {
    sections: Vec<(String, Vec<KeyHint>)>,
    scroll: u16,
}

impl HelpOverlay {
    pub fn new(sections: Vec<(String, Vec<KeyHint>)>) -> Self {
        Self {
            sections,
            scroll: 0,
        }
    }
}

impl<M: Debug + Send + 'static> Overlay<M> for HelpOverlay {
    fn title(&self) -> &str {
        "Help"
    }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog = centered_rect(60, 70, area);
        frame.render_widget(Clear, dialog);

        let block = Block::default()
            .title(" Help (? or Esc to close) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog);
        frame.render_widget(block, dialog);

        let mut lines: Vec<Line> = Vec::new();
        for (section_title, hints) in &self.sections {
            lines.push(Line::from(Span::styled(
                format!("── {} ──", section_title),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            for hint in hints {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:>10}", hint.key),
                        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(hint.desc, Style::default().fg(theme.text_muted)),
                ]));
            }
            lines.push(Line::raw(""));
        }

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .scroll((self.scroll, 0));
        frame.render_widget(paragraph, inner);
    }

    fn handle_event(&mut self, event: &Event) -> OverlayResult<M> {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => OverlayResult::Close,
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll = self.scroll.saturating_add(1);
                    OverlayResult::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll = self.scroll.saturating_sub(1);
                    OverlayResult::Consumed
                }
                _ => OverlayResult::Consumed,
            }
        } else {
            OverlayResult::Consumed
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
