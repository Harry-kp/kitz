# Error Boundaries

Kitz wraps every panel's `panel_view()` call in an error boundary. If a panel's rendering code panics, the panic is caught and the panel displays a fallback error message instead of crashing the entire application. The remaining panels, overlays, and the footer continue to render normally.

## Motivation

Terminal applications run in a single process. An unhandled panic tears down the whole application, often leaving the terminal in a broken state. In a multi-panel layout, a bug in one panel -- an out-of-bounds index, an unwrap on `None`, a formatting error -- should not take down the sidebar, the status bar, or the user's unsaved work in another panel.

This is the same principle behind React's `ErrorBoundary` component. In React, a component tree crash is isolated to the boundary that wraps it, and a fallback UI is rendered in its place. Kitz applies this idea at the panel level: each panel is its own boundary.

## How it works

The runtime maintains an `ErrorBoundaryState` that maps panel IDs to error messages:

```rust
pub struct ErrorBoundaryState {
    errors: HashMap<&'static str, String>,
}
```

Every time the runtime renders a panel, it calls `guarded_view()` instead of invoking `panel_view()` directly:

```rust
error_boundaries.guarded_view(id, frame, area, |f, a| {
    app.panel_view(id, f, a, focused);
});
```

Inside `guarded_view`, the view function runs inside `std::panic::catch_unwind`. If the closure completes normally, the panel renders as expected. If it panics, the panic payload is captured (either a `String` or `&str`), stored in the error map, and a fallback is rendered immediately.

On subsequent frames, if the panel ID already has a recorded error, the fallback is rendered without attempting to call the view function again.

## The fallback display

When a panel has panicked, the framework renders three lines in its area:

1. **"Panel '{id}' crashed"** -- in red, bold.
2. The panic message -- in yellow.
3. **"The rest of the application continues to work."** -- in dark gray.

This gives the user immediate feedback about what went wrong and reassurance that the application has not fully failed.

## Debugging a panicked panel

When you see the error boundary fallback during development:

1. **Read the panic message** displayed in the panel. It contains the same string that would appear in a normal panic backtrace.

2. **Check your `panel_view` implementation** for the panel ID shown. Common causes:
   - Indexing a `Vec` without bounds checking.
   - Calling `.unwrap()` on an `Option` or `Result` that can be `None`/`Err`.
   - Integer overflow or underflow in layout calculations.
   - Passing invalid coordinates to ratatui widgets.

3. **Enable logging** with `RUST_LOG=debug` to see the panic in the log output. Kitz uses `tracing` for structured logging when available.

4. **Run with `RUST_BACKTRACE=1`** for a full stack trace:
   ```bash
   RUST_BACKTRACE=1 cargo run
   ```

5. **Note that `catch_unwind` only catches panics, not aborts.** If your code calls `std::process::abort()` or triggers a signal (e.g., stack overflow), the error boundary cannot help.

## Recovery

Once an error is recorded for a panel, it stays in the error state for the rest of the session. The `ErrorBoundaryState` provides a `clear(id)` method, but this is primarily for internal use. In practice, a panic in a view function is a bug to fix rather than a transient error to retry.

If you need resilient rendering (e.g., displaying data that might be temporarily unavailable), handle that with `Option`/`Result` in your view logic rather than relying on the error boundary:

```rust
fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
    match id {
        "data" => {
            let text = match &self.data {
                Some(d) => format!("{d}"),
                None => "Loading...".to_string(),
            };
            frame.render_widget(Paragraph::new(text), area);
        }
        _ => {}
    }
}
```

## Why per-panel granularity

Kitz could place a single error boundary around the entire `view()` call. Per-panel boundaries are more useful because:

- **Isolation** -- a crash in a secondary panel (e.g., a log viewer) does not blank out the primary panel (e.g., an editor).
- **Diagnostics** -- the error message names the specific panel that failed, making the bug easier to locate.
- **User experience** -- the user can continue working in the healthy panels while the developer investigates the crash.
