use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::PanelId;

/// Tracks which panels have panicked, so they show an error state instead
/// of crashing the entire application.
#[derive(Default)]
pub struct ErrorBoundaryState {
    errors: HashMap<&'static str, String>,
}

impl ErrorBoundaryState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_error(&self, id: PanelId) -> bool {
        self.errors.contains_key(id)
    }

    pub fn clear(&mut self, id: PanelId) {
        self.errors.remove(id);
    }

    /// Run a panel's view function inside a catch_unwind boundary.
    /// If the panel panics, record the error and render a fallback.
    pub fn guarded_view(
        &mut self,
        id: PanelId,
        frame: &mut Frame,
        area: Rect,
        view_fn: impl FnOnce(&mut Frame, Rect),
    ) {
        if let Some(err) = self.errors.get(id) {
            render_error(frame, area, id, err);
            return;
        }

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            view_fn(frame, area);
        }));

        if let Err(panic_info) = result {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            self.errors.insert(id, msg);
            render_error(frame, area, id, self.errors.get(id).unwrap());
        }
    }
}

fn render_error(frame: &mut Frame, area: Rect, id: PanelId, error: &str) {
    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!("  Panel '{}' crashed", id),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::styled(format!("  {}", error), Style::default().fg(Color::Yellow)),
        Line::raw(""),
        Line::styled(
            "  The rest of the application continues to work.",
            Style::default().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}
