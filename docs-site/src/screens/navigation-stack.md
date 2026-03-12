# Navigation Stack

Screens are managed by a `NavigationStack` -- a simple stack data structure where the topmost screen drives the current UI. Pushing a screen transitions to a new view. Popping returns to the previous one. The stack can grow to arbitrary depth.

## NavigationStack API

```rust
pub struct NavigationStack<M: Debug + Send + 'static> {
    screens: Vec<Box<dyn Screen<M>>>,
}
```

| Method | Behavior |
|---|---|
| `push(screen)` | Push a screen onto the stack, calling `on_enter()` |
| `pop()` | Pop the topmost screen, calling `on_leave()` |
| `top()` | Borrow the topmost screen |
| `top_mut()` | Mutably borrow the topmost screen |
| `depth()` | Number of screens on the stack |
| `is_empty()` | Whether the stack is empty |

You do not interact with `NavigationStack` directly. The runtime manages it internally. Use `Context` methods to push and pop screens from your `update()` function.

## Pushing screens

From `Application::update()`:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::ViewDetail(id) => {
            ctx.push_screen(DetailScreen::new(id));
        }
        _ => {}
    }
    Command::none()
}
```

When a screen is pushed:

1. `on_enter()` is called on the new screen.
2. The runtime calls `sync_layout()` on the `PanelManager` with the new screen's panel IDs.
3. The new screen's panels become the active layout, and focus moves to the first panel.

## Popping screens

Screens can be popped in two ways:

### Automatic: Esc key

The runtime's `Esc` chain handles popping automatically:

1. If an overlay is open, close it.
2. If a screen is on the navigation stack, pop it.
3. If the stack is empty, quit the application.

This gives every screen a built-in "back" behavior without any code.

### Programmatic: `ctx.pop_screen()`

```rust
Msg::GoBack => {
    ctx.pop_screen();
}
```

When a screen is popped:

1. `on_leave()` is called on the departing screen.
2. The runtime syncs the panel manager with the layout of the screen now on top (or the `Application::panels()` layout if the stack is empty).
3. Focus is restored to the first panel of the restored layout.

## Lifecycle hooks

### `on_enter()`

Called immediately when the screen is pushed onto the stack. Use this for:

- Initializing or resetting screen state.
- Starting a data fetch.
- Setting a default selection.

```rust
fn on_enter(&mut self) {
    self.scroll_position = 0;
    self.loaded = false;
}
```

### `on_leave()`

Called immediately when the screen is popped off the stack. Use this for:

- Cleaning up resources.
- Saving draft state.
- Cancelling pending operations.

```rust
fn on_leave(&mut self) {
    self.draft.clear();
}
```

Both hooks are called synchronously during the push/pop operation, before the next frame is rendered.

## Stack depth

The navigation stack has no built-in depth limit. A common pattern is a list-to-detail flow:

```
Application (main dashboard)
  -> DetailScreen (item detail)
    -> EditScreen (edit form)
```

The user presses `Esc` at each level to walk back through the stack. If you need to pop multiple levels at once (e.g., "save and return to the main view"), call `ctx.pop_screen()` multiple times:

```rust
Msg::SaveAndReturn => {
    self.save();
    ctx.pop_screen(); // pop EditScreen
    ctx.pop_screen(); // pop DetailScreen
}
```

The intents are processed in order after `update()` returns, so both pops occur before the next render.

## Interaction with overlays

Overlays and screens are independent stacks. Overlays always take priority over screen navigation in the `Esc` chain. If a confirmation dialog is open on a detail screen, pressing `Esc` closes the dialog first, not the screen.

Screens pushed while an overlay is open work correctly -- the overlay remains on top until dismissed.

## Panel focus across screen transitions

When a screen is pushed, the `PanelManager` is synced with the new screen's panel IDs, and focus moves to the first panel. When a screen is popped, the panel manager syncs with the underlying layout. If the previously focused panel still exists in the restored layout, focus returns to it. Otherwise, focus defaults to the first panel.
