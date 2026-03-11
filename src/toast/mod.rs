use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// Severity level controls the toast color.
#[derive(Debug, Clone, Copy)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub(crate) created: Instant,
    pub(crate) ttl: Duration,
}

impl Toast {
    /// Create a toast with a 3-second default TTL.
    pub fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            created: Instant::now(),
            ttl: Duration::from_secs(3),
        }
    }

    /// Override the default time-to-live.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Returns `true` if this toast has outlived its TTL.
    pub fn is_expired(&self) -> bool {
        self.created.elapsed() >= self.ttl
    }
}

/// Manages a queue of toast notifications. Auto-dismisses expired ones.
#[derive(Default)]
pub struct ToastManager {
    toasts: Vec<Toast>,
}

impl ToastManager {
    /// Create an empty toast manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a toast for display.
    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Remove expired toasts. Call once per frame.
    pub fn tick(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Returns `true` if there are no active toasts.
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Borrow the current toast queue.
    pub fn toasts(&self) -> &[Toast] {
        &self.toasts
    }
}

/// Widget that renders toasts in the top-right corner, stacked vertically.
pub struct ToastWidget<'a> {
    manager: &'a ToastManager,
    theme: &'a Theme,
}

impl<'a> ToastWidget<'a> {
    pub fn new(manager: &'a ToastManager, theme: &'a Theme) -> Self {
        Self { manager, theme }
    }
}

impl Widget for ToastWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.manager.is_empty() || area.width < 20 || area.height < 1 {
            return;
        }

        let max_width = (area.width / 3).clamp(20, 50);
        let x = area.x + area.width - max_width - 1;
        let mut y = area.y + 1;

        for toast in self.manager.toasts() {
            if y >= area.y + area.height - 1 {
                break;
            }

            let color = match toast.level {
                ToastLevel::Info => self.theme.accent,
                ToastLevel::Success => self.theme.success,
                ToastLevel::Warning => self.theme.warning,
                ToastLevel::Error => self.theme.error,
            };

            let icon = match toast.level {
                ToastLevel::Info => "ℹ",
                ToastLevel::Success => "✓",
                ToastLevel::Warning => "⚠",
                ToastLevel::Error => "✗",
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", icon),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate_str(&toast.message, (max_width - 4) as usize),
                    Style::default().fg(self.theme.text),
                ),
                Span::raw(" "),
            ]);

            let toast_rect = Rect::new(x, y, max_width, 1);
            // Background
            for col in toast_rect.x..toast_rect.x + toast_rect.width {
                if col < buf.area().width + buf.area().x && y < buf.area().height + buf.area().y {
                    buf[(col, y)]
                        .set_bg(self.theme.surface)
                        .set_fg(self.theme.text);
                }
            }
            buf.set_line(x, y, &line, max_width);

            y += 1;
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    }
}
