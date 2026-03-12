use std::fmt::Debug;

use crossterm::event::{Event, KeyCode, KeyEvent};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{Overlay, OverlayResult};
use crate::theme::Theme;

/// A command in the palette, carrying a message to dispatch when selected.
pub struct PaletteCommand<M> {
    pub label: String,
    pub key_hint: String,
    pub message: M,
}

/// An entry in the palette — either actionable (dispatches a message) or
/// display-only (shows a key hint for discoverability).
pub(crate) enum PaletteEntry<M> {
    Actionable(PaletteCommand<M>),
    DisplayOnly { label: String, key_hint: String },
}

impl<M> PaletteEntry<M> {
    fn label(&self) -> &str {
        match self {
            PaletteEntry::Actionable(cmd) => &cmd.label,
            PaletteEntry::DisplayOnly { label, .. } => label,
        }
    }

    fn key_hint(&self) -> &str {
        match self {
            PaletteEntry::Actionable(cmd) => &cmd.key_hint,
            PaletteEntry::DisplayOnly { key_hint, .. } => key_hint,
        }
    }

    fn is_actionable(&self) -> bool {
        matches!(self, PaletteEntry::Actionable(_))
    }
}

/// Fuzzy-searchable command palette overlay (`:` key).
///
/// Auto-populated from panel key_hints for discoverability. Apps can also
/// add actionable commands via `PaletteCommand`.
pub struct CommandPaletteOverlay<M: Debug + Send + 'static> {
    entries: Vec<PaletteEntry<M>>,
    query: String,
    filtered: Vec<usize>,
    selected: usize,
}

impl<M: Debug + Send + 'static> CommandPaletteOverlay<M> {
    /// Create from actionable commands only.
    pub fn new(commands: Vec<PaletteCommand<M>>) -> Self {
        let entries: Vec<PaletteEntry<M>> =
            commands.into_iter().map(PaletteEntry::Actionable).collect();
        let filtered: Vec<usize> = (0..entries.len()).collect();
        Self {
            entries,
            query: String::new(),
            filtered,
            selected: 0,
        }
    }

    /// Create with display-only key hint entries plus optional actionable commands.
    pub(crate) fn with_hints(
        hints: Vec<(String, String)>,
        commands: Vec<PaletteCommand<M>>,
    ) -> Self {
        let mut entries: Vec<PaletteEntry<M>> = hints
            .into_iter()
            .map(|(label, key_hint)| PaletteEntry::DisplayOnly { label, key_hint })
            .collect();
        entries.extend(commands.into_iter().map(PaletteEntry::Actionable));
        let filtered: Vec<usize> = (0..entries.len()).collect();
        Self {
            entries,
            query: String::new(),
            filtered,
            selected: 0,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
            let pattern = Pattern::new(
                &self.query,
                CaseMatching::Ignore,
                Normalization::Smart,
                AtomKind::Fuzzy,
            );

            let mut scored: Vec<(usize, u32)> = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    let mut buf = Vec::new();
                    let haystack = Utf32Str::new(entry.label(), &mut buf);
                    pattern
                        .score(haystack, &mut matcher)
                        .map(|score| (i, score))
                })
                .collect();

            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        self.selected = 0;
    }
}

impl<M: Debug + Send + 'static> Overlay<M> for CommandPaletteOverlay<M> {
    fn title(&self) -> &str {
        "Command Palette"
    }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog = centered_wide(70, 60, area);
        frame.render_widget(Clear, dialog);

        let block = Block::default()
            .title(" Command Palette ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog);
        frame.render_widget(block, dialog);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        // Search input
        let prompt = Line::from(vec![
            Span::styled("> ", Style::default().fg(theme.accent)),
            Span::styled(&self.query, Style::default().fg(theme.text)),
            Span::styled("█", Style::default().fg(theme.accent)),
        ]);
        frame.render_widget(Paragraph::new(prompt), chunks[0]);

        // Separator
        let sep = "─".repeat(chunks[1].width as usize);
        frame.render_widget(
            Paragraph::new(sep).style(Style::default().fg(theme.border)),
            chunks[1],
        );

        // Results list
        let results_area = chunks[2];
        let visible = results_area.height as usize;
        let start = if self.selected >= visible {
            self.selected - visible + 1
        } else {
            0
        };

        let mut lines = Vec::new();
        for (display_idx, &entry_idx) in self.filtered.iter().enumerate().skip(start).take(visible)
        {
            let entry = &self.entries[entry_idx];
            let is_selected = display_idx == self.selected;
            let is_action = entry.is_actionable();

            let prefix = if is_selected { "▸ " } else { "  " };
            let label_style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_action {
                Style::default().fg(theme.text)
            } else {
                Style::default().fg(theme.text_muted)
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(entry.label(), label_style),
                Span::raw("  "),
                Span::styled(
                    entry.key_hint(),
                    Style::default()
                        .fg(theme.text_muted)
                        .add_modifier(Modifier::DIM),
                ),
            ]));
        }

        if self.filtered.is_empty() {
            lines.push(Line::styled(
                "  No matching commands",
                Style::default().fg(theme.text_muted),
            ));
        }

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Left),
            results_area,
        );
    }

    fn handle_event(&mut self, event: &Event) -> OverlayResult<M> {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Esc => return OverlayResult::Close,
                KeyCode::Enter => {
                    if let Some(&idx) = self.filtered.get(self.selected) {
                        if self.entries[idx].is_actionable() {
                            let entry = self.entries.remove(idx);
                            if let PaletteEntry::Actionable(cmd) = entry {
                                return OverlayResult::CloseWithMessage(cmd.message);
                            }
                        }
                        // Display-only entries just close the palette
                        return OverlayResult::Close;
                    }
                    return OverlayResult::Close;
                }
                KeyCode::Up | KeyCode::BackTab => {
                    self.selected = self.selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Tab => {
                    if !self.filtered.is_empty() {
                        self.selected = (self.selected + 1).min(self.filtered.len() - 1);
                    }
                }
                KeyCode::Char(c) => {
                    self.query.push(*c);
                    self.refilter();
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    self.refilter();
                }
                _ => {}
            }
            OverlayResult::Consumed
        } else {
            OverlayResult::Consumed
        }
    }
}

fn centered_wide(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
