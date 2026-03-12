# Focus and Navigation

When an application uses the panel system, Kitz provides built-in focus management. One panel is always the "focused" panel, and the runtime visually distinguishes it with a different border color, routes key events to it, and displays its key hints in the footer.

## Focus indicators

The focused panel's border uses `theme.border_focused` (typically the accent color). Unfocused panels use `theme.border`. The panel title is always displayed in the border, so users can identify panels at a glance.

When a panel is zoomed, its border title includes a `[zoomed]` suffix to make the state obvious.

## Keyboard navigation

### Tab / Shift+Tab

Pressing `Tab` moves focus to the next panel in display order (left-to-right for horizontal layouts, top-to-bottom for vertical layouts). `Shift+Tab` moves to the previous panel. Both wrap around -- tabbing past the last panel returns to the first.

These keys are handled by the runtime before reaching your application code. They work whenever a panel layout is active and no overlay is open.

### Zoom with `z`

Pressing `z` toggles zoom on the focused panel. When zoomed, the focused panel expands to fill the entire main area. All other panels are hidden. Press `z` again to return to the normal layout.

Zoom is useful for temporarily maximizing an editor, log viewer, or any panel that benefits from extra space.

### Esc chain

`Esc` follows a priority chain:

1. If an overlay is open, close it.
2. If a screen is on the navigation stack, pop it.
3. Otherwise, quit the application.

This means `Esc` naturally "backs out" of nested UI layers without any custom handling.

## Mouse click-to-focus

When mouse support is enabled (the default), clicking inside a panel's region sets focus to that panel. The runtime hit-tests the click coordinates against the computed `Rect` for each panel and calls `focus_panel()` on the first match.

Mouse click-to-focus only activates when no overlay is open. If an overlay is displayed, mouse events are consumed by the overlay instead.

## The PanelManager

Focus state is tracked internally by `PanelManager`:

```rust
pub struct PanelManager {
    panel_ids: Vec<PanelId>,
    focused_idx: usize,
    zoomed: bool,
}
```

Key methods:

| Method | Behavior |
|---|---|
| `focus_next()` | Advance focus to the next panel (wraps around) |
| `focus_prev()` | Move focus to the previous panel (wraps around) |
| `focus_panel(id)` | Set focus to a specific panel by ID |
| `toggle_zoom()` | Toggle the zoomed state |
| `focused_id()` | Return the currently focused panel's ID |
| `is_focused(id)` | Check if a specific panel is focused |
| `is_zoomed()` | Check if the focused panel is zoomed |
| `sync_layout(ids)` | Update the panel ID list (preserves focus if possible) |

You do not interact with `PanelManager` directly. The runtime creates and manages it. To change focus programmatically from your `update()` function, use the `Context`:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::OpenEditor => {
            ctx.focus_panel("editor");
        }
        Msg::ToggleZoom => {
            ctx.toggle_zoom();
        }
        _ => {}
    }
    Command::none()
}
```

## Focus lifecycle hooks

The `Application` trait provides two hooks for reacting to focus changes:

```rust
fn panel_on_focus(&mut self, id: PanelId) {
    // Called when `id` gains focus
}

fn panel_on_blur(&mut self, id: PanelId) {
    // Called when `id` loses focus
}
```

These hooks fire in sequence: `panel_on_blur` is called on the old panel first, then `panel_on_focus` on the new panel. They are triggered by:

- Tab / Shift+Tab cycling
- Mouse click-to-focus
- Programmatic focus via `ctx.focus_panel()`

Common uses:

- Pausing a timer or animation when a panel loses focus.
- Loading data lazily when a panel first receives focus.
- Updating a status indicator or breadcrumb trail.

## Event routing

Key events flow through the following chain:

1. **Overlays** -- if an overlay is open, it gets the event first. If the overlay returns `Consumed` or `Close`/`CloseWithMessage`, processing stops.
2. **`Application::handle_event()`** -- your global event handler. Return `EventResult::Message(msg)` to dispatch a message, or `EventResult::Ignored` to pass through.
3. **Focused panel** -- if the global handler returned `Ignored`, the runtime calls `panel_handle_key(focused_id, key)` on the focused panel.
4. **Convention keys** -- if the panel also returned `Ignored`, the runtime checks for built-in convention keys: `q` (quit), `Tab`/`Shift+Tab` (focus), `z` (zoom), `?` (help), `:` (command palette).

This chain ensures that overlays always take priority, your application can intercept any key globally, panels get their own scoped handling, and convention keys serve as a sensible fallback.
