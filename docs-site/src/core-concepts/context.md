# Context

Kitz provides three context types, each tailored to a specific phase of the application lifecycle. `Context<M>` is the mutable context passed to `update()` for producing framework-level side effects. `ViewContext` and `EventContext` are read-only snapshots of runtime state passed to `view()` and `handle_event()`, respectively.

## Context\<M\> --- The Update Context

`Context<M>` is passed as `&mut Context<Self::Message>` to every call to `update()`. It is the mechanism for requesting operations that affect framework-owned state: the overlay stack, the panel manager, the toast system, and the navigation stack.

### Why Context exists

Some operations cannot be expressed as commands. A `Command` is a value returned from `update()` and processed afterward --- it works well for spawning threads, quitting, or re-dispatching messages. But pushing an overlay requires mutable access to the overlay stack, which the application does not own. Changing panel focus requires mutating the panel manager. These are **synchronous mutations of framework internals** that must happen atomically as part of the update cycle.

Context solves this by collecting **intents** --- descriptions of framework mutations --- during `update()`. After `update()` returns, the runtime drains these intents and applies them before processing the returned command. This means an overlay pushed via `ctx.push_overlay()` is active before any `Command::message()` in the returned command is dispatched.

### Methods

#### `push_overlay(overlay)`

Push a modal overlay onto the overlay stack. The overlay immediately captures all input on the next event-loop iteration. Overlays are rendered on top of the main UI and receive all terminal events until they are closed.

```rust
Msg::ShowConfirm => {
    ctx.push_overlay(ConfirmOverlay::new(
        "Confirm Delete",
        "Are you sure you want to delete this item?",
        Msg::DeleteConfirmed,
    ));
    Command::none()
}
```

The argument must implement the `Overlay<M>` trait. Kitz provides several built-in overlays (`ConfirmOverlay`, `HelpOverlay`, `CommandPaletteOverlay`), and you can implement custom overlays by implementing the trait yourself.

#### `pop_overlay()`

Pop the topmost overlay from the stack. If no overlay is open, this is a no-op.

```rust
Msg::CancelDialog => {
    ctx.pop_overlay();
    Command::none()
}
```

In most cases, overlays close themselves by returning `OverlayResult::Close` or `OverlayResult::CloseWithMessage` from their `handle_event` method. Use `ctx.pop_overlay()` when your update logic needs to programmatically dismiss an overlay --- for example, closing a progress dialog after an async operation completes.

#### `focus_panel(id)`

Move focus to a specific panel by its `PanelId`. The runtime calls `panel_on_blur` for the previously focused panel and `panel_on_focus` for the newly focused panel.

```rust
Msg::OpenFile(path) => {
    self.editor.open(path);
    ctx.focus_panel("editor");
    Command::none()
}
```

The `id` must match one of the panel IDs returned by `panels()`. If the ID does not match any panel, the focus does not change.

#### `toggle_zoom()`

Toggle zoom on the currently focused panel. When zoomed, the focused panel occupies the entire main area and all other panels are hidden. The panel's border title shows `[zoomed]` as a visual indicator.

```rust
Msg::ToggleZoom => {
    ctx.toggle_zoom();
    Command::none()
}
```

This is the programmatic equivalent of the user pressing `z` (the convention key for zoom). Use it when you want to zoom in response to a specific application event rather than a direct key press.

#### `toast(message, level)`

Display a toast notification. Toasts appear in the top-right corner of the terminal, above the main content but below any open overlay. They auto-dismiss after a short duration.

```rust
Msg::FileSaved => {
    ctx.toast("File saved successfully", ToastLevel::Info);
    Command::none()
}

Msg::NetworkError(e) => {
    ctx.toast(format!("Connection failed: {}", e), ToastLevel::Error);
    Command::none()
}
```

The `ToastLevel` enum controls the toast's color:

| Level | Typical Color | Use Case |
|-------|---------------|----------|
| `ToastLevel::Info` | Theme accent | Success confirmations, status updates |
| `ToastLevel::Warning` | Theme warning | Non-critical issues, degraded states |
| `ToastLevel::Error` | Theme error | Failures, unrecoverable problems |

Multiple toasts can be visible simultaneously. They stack vertically and each dismisses independently.

#### `push_screen(screen)`

Push a new screen onto the navigation stack. The screen's panel layout replaces the current one, and its `on_enter` hook is called. The previous screen is preserved on the stack.

```rust
Msg::OpenSettings => {
    ctx.push_screen(SettingsScreen::new(self.config.clone()));
    Command::none()
}
```

Screens are a navigation abstraction for applications with multiple distinct views --- a main screen, a settings screen, a detail screen. Each screen defines its own `panels()`, `panel_view()`, and `panel_handle_key()`. The user can press Esc to pop the current screen and return to the previous one.

#### `pop_screen()`

Pop the topmost screen from the navigation stack, calling its `on_leave` hook. The previous screen's panel layout is restored.

```rust
Msg::SettingsSaved => {
    self.apply_settings();
    ctx.pop_screen();
    ctx.toast("Settings applied", ToastLevel::Info);
    Command::none()
}
```

If the navigation stack is empty, this is a no-op.

### Using Context in tests

`Context::new()` creates an empty context suitable for unit tests. After calling `update()`, you can inspect the number of pending intents with `intent_count()` to verify that your update logic requested the expected framework operations.

```rust
#[test]
fn delete_shows_confirm_overlay() {
    let mut app = App::new();
    let mut ctx = Context::new();
    let _ = app.update(Msg::DeleteRequested, &mut ctx);
    assert_eq!(ctx.intent_count(), 1);
}
```

For more fine-grained assertions, the intent types are `pub(crate)` and not directly accessible from application code. This is intentional --- tests should verify observable behavior (the intent count, the model state after update) rather than the specific intent variant.

## ViewContext --- The Render Context

`ViewContext` is passed to `view()` when using the custom rendering path. It provides read-only access to the panel focus state maintained by the runtime.

### Methods

#### `focused_panel() -> Option<PanelId>`

Returns the currently focused panel ID, if any. This is `None` when no panel layout is active.

#### `is_zoomed() -> bool`

Returns whether the focused panel is currently zoomed.

### When to use

`ViewContext` is primarily useful in the custom rendering path when you still want to be aware of panel state. In the convention path, the runtime passes focus information directly to `panel_view()` via the `focused` parameter, so you rarely need to query `ViewContext` yourself.

```rust
fn view(&self, frame: &mut Frame, ctx: &ViewContext) {
    if ctx.is_zoomed() {
        self.render_zoomed(frame);
    } else {
        self.render_normal(frame);
    }
}
```

### Creating in tests

```rust
let ctx = ViewContext::new();
let ctx = ViewContext::with_panels(Some("sidebar"), true);
```

## EventContext --- The Event-Handling Context

`EventContext` is passed to `handle_event()` and provides read-only information about the current runtime state at the time an event arrives.

### Methods

#### `focused_panel() -> Option<PanelId>`

Returns the currently focused panel ID. Useful when a global key binding should behave differently depending on which panel is focused.

```rust
fn handle_event(&self, event: &Event, ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        if key.code == KeyCode::Enter {
            return match ctx.focused_panel() {
                Some("search") => EventResult::Message(Msg::ExecuteSearch),
                Some("results") => EventResult::Message(Msg::OpenSelected),
                _ => EventResult::Ignored,
            };
        }
    }
    EventResult::Ignored
}
```

#### `has_overlay() -> bool`

Returns whether an overlay is currently open. In practice, this is always `false` inside `handle_event()` because overlays consume events before `handle_event()` is reached. The method exists for informational completeness and for edge cases where you check event context outside the normal event cascade.

### Creating in tests

```rust
let ctx = EventContext::new();
let ctx = EventContext::with_state(Some("sidebar"), false);
```

## Summary of Context Types

| Type | Passed to | Mutability | Purpose |
|------|-----------|------------|---------|
| `Context<M>` | `update()` | `&mut` | Collect framework intents (overlays, focus, toasts, screens) |
| `ViewContext` | `view()` | `&` (read-only) | Query panel focus and zoom state during rendering |
| `EventContext` | `handle_event()` | `&` (read-only) | Query panel focus and overlay state during event mapping |

The separation ensures that only `update()` can request state changes, while `view()` and `handle_event()` remain pure observers of runtime state. This aligns with the TEA principle that all mutations flow through a single entry point.
