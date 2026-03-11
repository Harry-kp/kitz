use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

/// A single-line text input with cursor movement, insert, and delete.
///
/// UTF-8 aware — supports multi-byte characters correctly.
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    content: String,
    cursor: usize,
}

impl TextInputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content(content: impl Into<String>) -> Self {
        let content = content.into();
        let cursor = content.len();
        Self { content, cursor }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, s: impl Into<String>) {
        self.content = s.into();
        self.cursor = self.content.len();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.content.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor > 0 {
            let prev = self.prev_char_boundary();
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor < self.content.len() {
            let next = self.next_char_boundary();
            self.content.drain(self.cursor..next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_char_boundary();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = self.next_char_boundary();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn delete_to_end(&mut self) {
        self.content.truncate(self.cursor);
    }

    fn prev_char_boundary(&self) -> usize {
        let mut idx = self.cursor.saturating_sub(1);
        while idx > 0 && !self.content.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    fn next_char_boundary(&self) -> usize {
        let mut idx = self.cursor + 1;
        while idx < self.content.len() && !self.content.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    }

    fn char_position(&self) -> usize {
        self.content[..self.cursor].chars().count()
    }
}

/// Renders a `TextInputState` with a visible cursor.
pub struct TextInput<'a> {
    state: &'a TextInputState,
    style: Style,
    cursor_style: Style,
    show_cursor: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(state: &'a TextInputState) -> Self {
        Self {
            state,
            style: Style::default(),
            cursor_style: Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
            show_cursor: true,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[allow(dead_code)]
    pub fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }

    pub fn show_cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }
}

impl Widget for TextInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let content = self.state.content();
        let cursor_pos = self.state.char_position();

        // Render the text content
        let mut x = area.x;
        for (i, ch) in content.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
            if width == 0 {
                continue;
            }
            let style = if self.show_cursor && i == cursor_pos {
                self.cursor_style
            } else {
                self.style
            };
            buf.set_string(x, area.y, ch.to_string(), style);
            x += width;
        }

        // If cursor is at the end, render a space with cursor style
        if self.show_cursor && cursor_pos >= content.chars().count() && x < area.x + area.width {
            buf.set_string(x, area.y, " ", self.cursor_style);
        }
    }
}
