use std::fmt::Debug;

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{Overlay, OverlayResult};
use crate::theme::Theme;

/// A built-in confirmation dialog.
///
/// Renders a centered box with a question, Yes/No buttons.
/// - Enter confirms, Esc / n cancels.
/// - Tab / h / l toggles the selection.
pub struct ConfirmOverlay<M: Debug + Send + 'static> {
    title: String,
    message: String,
    on_confirm: Option<M>,
    selected_yes: bool,
}

impl<M: Debug + Send + 'static> ConfirmOverlay<M> {
    pub fn new(title: impl Into<String>, message: impl Into<String>, on_confirm: M) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            on_confirm: Some(on_confirm),
            selected_yes: false,
        }
    }
}

impl<M: Debug + Send + 'static> Overlay<M> for ConfirmOverlay<M> {
    fn title(&self) -> &str {
        &self.title
    }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog = centered_rect(50, 30, area);
        frame.render_widget(Clear, dialog);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog);
        frame.render_widget(block, dialog);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let msg = Paragraph::new(self.message.as_str())
            .style(Style::default().fg(theme.text))
            .alignment(Alignment::Center);
        frame.render_widget(msg, chunks[1]);

        let yes_style = if self.selected_yes {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_muted)
        };
        let no_style = if !self.selected_yes {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_muted)
        };

        let buttons = Line::from(vec![
            Span::raw("  "),
            Span::styled(" Yes ", yes_style),
            Span::raw("   "),
            Span::styled(" No ", no_style),
            Span::raw("  "),
        ]);
        let btn_paragraph = Paragraph::new(buttons).alignment(Alignment::Center);
        frame.render_widget(btn_paragraph, chunks[2]);
    }

    fn handle_event(&mut self, event: &Event) -> OverlayResult<M> {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Tab
                | KeyCode::Char('h')
                | KeyCode::Char('l')
                | KeyCode::Left
                | KeyCode::Right => {
                    self.selected_yes = !self.selected_yes;
                    OverlayResult::Consumed
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    if self.selected_yes {
                        if let Some(msg) = self.on_confirm.take() {
                            OverlayResult::CloseWithMessage(msg)
                        } else {
                            OverlayResult::Close
                        }
                    } else {
                        OverlayResult::Close
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') => OverlayResult::Close,
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
