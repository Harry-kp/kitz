use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::panel::KeyHint;
use crate::theme::Theme;

/// Auto-generated footer bar showing key hints for the focused panel plus
/// global framework shortcuts.
pub struct Footer<'a> {
    panel_hints: &'a [KeyHint],
    theme: &'a Theme,
}

impl<'a> Footer<'a> {
    pub fn new(panel_hints: &'a [KeyHint], theme: &'a Theme) -> Self {
        Self { panel_hints, theme }
    }

    fn global_hints() -> Vec<KeyHint> {
        vec![
            KeyHint::new("Tab", "Switch panel"),
            KeyHint::new("z", "Zoom"),
            KeyHint::new("?", "Help"),
            KeyHint::new("q", "Quit"),
        ]
    }
}

impl Widget for Footer<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut spans = Vec::new();

        for hint in self.panel_hints {
            spans.push(Span::styled(
                format!(" {} ", hint.key),
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("{} ", hint.desc),
                Style::default().fg(self.theme.text_muted),
            ));
        }

        if !self.panel_hints.is_empty() {
            spans.push(Span::styled("│ ", Style::default().fg(self.theme.border)));
        }

        for hint in Self::global_hints() {
            spans.push(Span::styled(
                format!(" {} ", hint.key),
                Style::default()
                    .fg(self.theme.text_muted)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("{} ", hint.desc),
                Style::default().fg(self.theme.text_muted),
            ));
        }

        let line = Line::from(spans);
        let paragraph = ratatui::widgets::Paragraph::new(line);
        paragraph.render(area, buf);
    }
}
