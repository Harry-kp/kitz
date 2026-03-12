# TextInput

`TextInput` is a single-line text input widget. It handles UTF-8 text correctly, including multi-byte characters, and provides cursor movement, insertion, and deletion operations.

The widget is split into two parts:

- **`TextInputState`** -- owns the text content and cursor position. Lives in your application state.
- **`TextInput`** -- a stateless ratatui `Widget` that renders a `TextInputState`. Created each frame during rendering.

## TextInputState

```rust
pub struct TextInputState {
    content: String,
    cursor: usize, // byte offset into content
}
```

### Constructors

```rust
let state = TextInputState::new();                     // empty
let state = TextInputState::with_content("hello");     // pre-filled
```

### Content access

```rust
state.content()                   // &str - the current text
state.set_content("new value");   // replace the content
state.clear();                    // clear to empty string
```

### Editing operations

| Method | Behavior |
|---|---|
| `insert_char(ch)` | Insert a character at the cursor position and advance the cursor |
| `delete_char_before()` | Delete the character before the cursor (Backspace) |
| `delete_char_after()` | Delete the character after the cursor (Delete) |
| `delete_to_end()` | Delete from the cursor to the end of the line |

### Cursor movement

| Method | Behavior |
|---|---|
| `move_left()` | Move the cursor one character to the left |
| `move_right()` | Move the cursor one character to the right |
| `move_home()` | Move the cursor to the beginning |
| `move_end()` | Move the cursor to the end |

All movement operations respect UTF-8 character boundaries. Moving left from the beginning or right from the end is a no-op.

### UTF-8 correctness

The cursor is stored as a byte offset, but all operations navigate by character boundaries. This means multi-byte characters (CJK, emoji, accented letters) are handled correctly:

- `insert_char` advances the cursor by `ch.len_utf8()` bytes.
- `move_left` walks backward to the previous character boundary.
- `move_right` walks forward to the next character boundary.
- `delete_char_before` removes all bytes of the preceding character.

You never need to worry about splitting a multi-byte character.

## TextInput widget

The `TextInput` widget renders a `TextInputState` into a ratatui `Buffer`.

### Construction and styling

```rust
let widget = TextInput::new(&state)
    .style(Style::default().fg(theme.text))
    .cursor_style(Style::default().fg(theme.bg).bg(theme.accent))
    .show_cursor(true);
```

| Method | Purpose |
|---|---|
| `style(style)` | Style for the text content |
| `cursor_style(style)` | Style for the cursor character |
| `show_cursor(bool)` | Whether to render the cursor (default: true) |

The cursor is rendered by applying `cursor_style` to the character at the cursor position. If the cursor is at the end of the text, a space character with the cursor style is rendered.

### Rendering

```rust
frame.render_widget(
    TextInput::new(&self.search_state)
        .style(Style::default().fg(theme.text))
        .cursor_style(Style::default().fg(theme.bg).bg(theme.accent))
        .show_cursor(true),
    input_area,
);
```

The widget uses `unicode_width::UnicodeWidthChar` to correctly compute display widths for wide characters.

## Complete usage example

```rust
use kitz::prelude::*;
use ratatui::widgets::Paragraph;

const SEARCH: PanelId = "search";

struct App {
    input: TextInputState,
    results: Vec<String>,
}

#[derive(Debug)]
enum Msg {
    Search(String),
}

impl Application for App {
    type Message = Msg;

    fn panels(&self) -> PanelLayout {
        PanelLayout::single(SEARCH)
    }

    fn panel_title(&self, _id: PanelId) -> &str {
        "Search"
    }

    fn panel_view(&self, _id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let input_widget = TextInput::new(&self.input)
            .style(Style::default().fg(ratatui::style::Color::White))
            .cursor_style(Style::default().bg(ratatui::style::Color::Cyan))
            .show_cursor(true);
        frame.render_widget(input_widget, chunks[0]);

        let results_text: String = self.results.iter()
            .map(|r| format!("  {r}\n"))
            .collect();
        frame.render_widget(Paragraph::new(results_text), chunks[1]);
    }

    fn panel_handle_key(&mut self, _id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        match key.code {
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                EventResult::Message(Msg::Search(self.input.content().to_string()))
            }
            KeyCode::Backspace => {
                self.input.delete_char_before();
                EventResult::Message(Msg::Search(self.input.content().to_string()))
            }
            KeyCode::Left => {
                self.input.move_left();
                EventResult::Consumed
            }
            KeyCode::Right => {
                self.input.move_right();
                EventResult::Consumed
            }
            KeyCode::Home => {
                self.input.move_home();
                EventResult::Consumed
            }
            KeyCode::End => {
                self.input.move_end();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Search(query) => {
                self.results = self.all_items()
                    .filter(|item| item.contains(&query))
                    .cloned()
                    .collect();
            }
        }
        Command::none()
    }
}
```

Note that `TextInputState` mutation happens in `panel_handle_key`, not in `update`. This is because the input state needs to respond immediately to each keystroke. The `Msg::Search` message is dispatched to trigger a re-filter of results through the standard TEA update cycle.
