# Custom Overlays

When the built-in overlays are not sufficient, you can implement the `Overlay` trait to create any modal UI: input forms, file pickers, progress indicators, or multi-step wizards.

## Implementing the Overlay trait

The trait requires three methods:

```rust
pub trait Overlay<M: Debug + Send + 'static> {
    fn title(&self) -> &str;
    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_event(&mut self, event: &Event) -> OverlayResult<M>;
}
```

- `title()` returns a display string for the overlay's border.
- `view()` renders the overlay. The `area` is the full terminal area, so you control positioning entirely.
- `handle_event()` processes input and returns an `OverlayResult` to tell the runtime what to do next.

## Complete example: text input overlay

This example builds an overlay that prompts the user to type a name. When they press Enter, it dispatches a message containing the entered text. Pressing Esc cancels.

```rust
use std::fmt::Debug;

use crossterm::event::{Event, KeyCode, KeyEvent};
use kitz::prelude::*;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use kitz::overlay::{Overlay, OverlayResult};
use kitz::theme::Theme;
use kitz::widgets::{TextInput, TextInputState};

pub struct InputOverlay<M: Debug + Send + 'static> {
    title: String,
    prompt: String,
    input: TextInputState,
    on_submit: Box<dyn Fn(String) -> M + Send>,
}

impl<M: Debug + Send + 'static> InputOverlay<M> {
    pub fn new(
        title: impl Into<String>,
        prompt: impl Into<String>,
        on_submit: impl Fn(String) -> M + Send + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            input: TextInputState::new(),
            on_submit: Box::new(on_submit),
        }
    }
}

impl<M: Debug + Send + 'static> Overlay<M> for InputOverlay<M> {
    fn title(&self) -> &str {
        &self.title
    }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog = centered_rect(50, 20, area);
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
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let prompt = Paragraph::new(self.prompt.as_str())
            .style(Style::default().fg(theme.text));
        frame.render_widget(prompt, chunks[0]);

        let input_widget = TextInput::new(&self.input)
            .style(Style::default().fg(theme.text))
            .cursor_style(
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent),
            )
            .show_cursor(true);
        frame.render_widget(input_widget, chunks[2]);
    }

    fn handle_event(&mut self, event: &Event) -> OverlayResult<M> {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Esc => return OverlayResult::Close,
                KeyCode::Enter => {
                    let value = self.input.content().to_string();
                    if !value.is_empty() {
                        let msg = (self.on_submit)(value);
                        return OverlayResult::CloseWithMessage(msg);
                    }
                    return OverlayResult::Consumed;
                }
                KeyCode::Char(c) => self.input.insert_char(*c),
                KeyCode::Backspace => self.input.delete_char_before(),
                KeyCode::Delete => self.input.delete_char_after(),
                KeyCode::Left => self.input.move_left(),
                KeyCode::Right => self.input.move_right(),
                KeyCode::Home => self.input.move_home(),
                KeyCode::End => self.input.move_end(),
                _ => {}
            }
        }
        OverlayResult::Consumed
    }
}
```

### Using the overlay

Push it from your `update()` function:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::AskForName => {
            ctx.push_overlay(InputOverlay::new(
                "New Item",
                "Enter a name:",
                |name| Msg::CreateItem(name),
            ));
        }
        Msg::CreateItem(name) => {
            self.items.push(name);
        }
        _ => {}
    }
    Command::none()
}
```

## Design guidelines

### Consume all events

An overlay should return `OverlayResult::Consumed` for any event it does not explicitly handle. Returning `Ignored` causes the event to propagate to the application and panels underneath, which is rarely the intended behavior for a modal.

### Use `Clear` before rendering

Call `frame.render_widget(Clear, dialog_rect)` before drawing your overlay content. This erases the underlying panel content in the overlay's region, preventing visual artifacts.

### Use `centered_rect` for positioning

The `centered_rect(percent_x, percent_y, area)` utility computes a centered `Rect` within the given area. It is available in the prelude.

### Communicate results through messages

Overlays should not mutate application state directly. Instead, return `OverlayResult::CloseWithMessage(msg)` to send a message to `update()`. This keeps the data flow unidirectional and makes the interaction testable.

### Keep overlays focused

Each overlay should do one thing. If you need a multi-step wizard, consider pushing a sequence of overlays or using a screen instead.
