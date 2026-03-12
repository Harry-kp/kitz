# Widgets

Kitz provides a small set of purpose-built widgets that integrate with the framework's theme and panel systems. These complement ratatui's own widget library rather than replacing it -- you continue to use `Paragraph`, `List`, `Table`, `Block`, and other ratatui widgets directly in your `panel_view` implementations.

## Available widgets

### TextInput

A single-line text input widget with full UTF-8 support. It manages cursor position, insertion, deletion, and movement through a `TextInputState` struct. The widget renders the text with a visible cursor.

See [TextInput](text-input.md) for the full API and usage examples.

### Footer

An auto-generated footer bar that displays key hints from the focused panel alongside global convention keys. The runtime renders this automatically at the bottom of the terminal when a panel layout is active. No manual setup is required.

See [Footer](footer.md) for details on how it works and how to customize it.

### centered_rect

A utility function that computes a centered `Rect` within a given area. Used extensively by overlays to position dialog boxes.

```rust
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect
```

**Parameters:**
- `percent_x` -- width of the centered rectangle as a percentage of the area width.
- `percent_y` -- height of the centered rectangle as a percentage of the area height.
- `area` -- the outer bounding rectangle.

**Example:**

```rust
use kitz::widgets::centered_rect;

let dialog = centered_rect(50, 30, frame.area());
frame.render_widget(Clear, dialog);
frame.render_widget(my_widget, dialog);
```

This creates a rectangle that is 50% of the terminal width and 30% of the terminal height, centered both horizontally and vertically.

### Toast notifications

A non-blocking notification system for displaying transient messages. Toasts appear in the top-right corner, stack vertically, and auto-dismiss after a configurable duration.

See [Toast Notifications](toast-notifications.md) for the full API.

## Using ratatui widgets

The Kitz widget set is intentionally small. For everything else, use ratatui directly. Inside `panel_view`, you have full access to `Frame` and `Rect`:

```rust
fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
    match id {
        "logs" => {
            let items: Vec<ListItem> = self.logs
                .iter()
                .map(|l| ListItem::new(l.as_str()))
                .collect();
            let list = List::new(items)
                .highlight_style(Style::default().fg(self.theme.accent));
            frame.render_widget(list, area);
        }
        _ => {}
    }
}
```

Kitz adds structure (panels, focus, overlays) around your ratatui rendering code. It does not abstract away ratatui's widget system.
