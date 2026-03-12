# Overlays

Overlays are modal UI elements that float above the main panel layout. They capture all input until dismissed, making them ideal for confirmation dialogs, help screens, command palettes, and custom forms. Kitz manages overlays through a stack, so multiple overlays can be layered.

## The Overlay trait

Every overlay implements the `Overlay` trait:

```rust
pub trait Overlay<M: Debug + Send + 'static> {
    fn title(&self) -> &str;
    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_event(&mut self, event: &Event) -> OverlayResult<M>;
}
```

| Method | Purpose |
|---|---|
| `title()` | Display title rendered in the overlay's border |
| `view()` | Render the overlay content within the given area |
| `handle_event()` | Process an input event, returning how it was handled |

The `area` passed to `view()` is the full terminal area, giving the overlay complete freedom over positioning. Most overlays use the `centered_rect` utility to render a centered dialog box.

## OverlayResult

`handle_event` returns an `OverlayResult` that tells the runtime what to do next:

```rust
pub enum OverlayResult<M> {
    Close,
    CloseWithMessage(M),
    Consumed,
    Ignored,
}
```

| Variant | Behavior |
|---|---|
| `Close` | Pop this overlay off the stack without sending a message |
| `CloseWithMessage(msg)` | Pop this overlay and dispatch `msg` to `Application::update()` |
| `Consumed` | The overlay handled the event; do not propagate further |
| `Ignored` | The overlay did not handle the event (rare for overlays) |

The distinction between `Close` and `CloseWithMessage` is central to the overlay design. A confirmation dialog, for example, returns `Close` when the user cancels and `CloseWithMessage(some_action)` when the user confirms.

## OverlayStack

Overlays are managed by an `OverlayStack`:

```rust
pub struct OverlayStack<M: Debug + Send + 'static> {
    stack: Vec<Box<dyn Overlay<M>>>,
}
```

Key methods:

| Method | Behavior |
|---|---|
| `push(overlay)` | Push a new overlay onto the stack |
| `pop()` | Remove the topmost overlay |
| `top()` | Borrow the topmost overlay (if any) |
| `top_mut()` | Mutably borrow the topmost overlay |
| `is_empty()` | Check whether any overlays are active |
| `len()` | Number of overlays on the stack |

Only the topmost overlay receives events and is rendered. Lower overlays in the stack are hidden until the ones above them are dismissed.

## Event priority

Overlays sit at the top of the event processing chain. When an overlay is open:

1. The overlay's `handle_event()` is called first.
2. If the result is `Close` or `CloseWithMessage`, the overlay is popped and (if applicable) the message is dispatched.
3. If the result is `Consumed`, no further processing occurs.
4. If the result is `Ignored`, the event falls through to `Application::handle_event()` and then the focused panel.

In practice, overlays almost always return `Consumed` or `Close`/`CloseWithMessage`. Returning `Ignored` is unusual because an open overlay is expected to own all keyboard input.

Mouse events do not reach the panel system while an overlay is active. Click-to-focus on panels is suppressed.

## Pushing overlays

From your `update()` function, push an overlay through the `Context`:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::ConfirmDelete => {
            ctx.push_overlay(ConfirmOverlay::new(
                "Delete Item",
                "Are you sure you want to delete this item?",
                Msg::DoDelete,
            ));
        }
        Msg::DoDelete => {
            self.items.remove(self.selected);
        }
        _ => {}
    }
    Command::none()
}
```

You can also pop the topmost overlay programmatically with `ctx.pop_overlay()`.

## Built-in overlays

Kitz ships with three overlays:

- **`ConfirmOverlay`** -- a Yes/No confirmation dialog.
- **`HelpOverlay`** -- a scrollable help screen auto-generated from panel key hints.
- **`CommandPaletteOverlay`** -- a fuzzy-searchable command palette.

These are covered in detail in the [Built-in Overlays](built-in-overlays.md) section.

## Custom overlays

You can implement the `Overlay` trait for any custom modal UI. See [Custom Overlays](custom-overlays.md) for a complete walkthrough.
