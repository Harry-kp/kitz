# Toast Notifications

Toasts are non-blocking, auto-dismissing notifications that appear in the top-right corner of the terminal. They are used for transient feedback: successful saves, warnings, errors, or informational messages that do not require user interaction.

## ToastLevel

Each toast has a severity level that controls its icon and color:

```rust
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}
```

| Level | Icon | Color source |
|---|---|---|
| `Info` | `i` | `theme.accent` |
| `Success` | checkmark | `theme.success` |
| `Warning` | warning sign | `theme.warning` |
| `Error` | `x` | `theme.error` |

## Showing a toast

From your `update()` function, call `ctx.toast()`:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::SaveComplete => {
            ctx.toast("File saved successfully", ToastLevel::Success);
        }
        Msg::NetworkError(e) => {
            ctx.toast(format!("Connection failed: {e}"), ToastLevel::Error);
        }
        Msg::LowDisk => {
            ctx.toast("Disk space is running low", ToastLevel::Warning);
        }
        Msg::NewVersion(v) => {
            ctx.toast(format!("Version {v} available"), ToastLevel::Info);
        }
        _ => {}
    }
    Command::none()
}
```

`ctx.toast()` takes any type that implements `Into<String>` and a `ToastLevel`. The toast is created with a default time-to-live of 3 seconds.

## Auto-dismiss

Each toast has a TTL (time-to-live). The `ToastManager` calls `tick()` every frame, which removes expired toasts. The default TTL is 3 seconds.

If you need a longer or shorter duration, construct a `Toast` directly and push it through the `ToastManager` (advanced usage):

```rust
use kitz::toast::{Toast, ToastLevel};
use std::time::Duration;

let toast = Toast::new("Processing complete", ToastLevel::Success)
    .with_ttl(Duration::from_secs(5));
```

In most cases, `ctx.toast()` with the default TTL is sufficient.

## Stacking

Multiple toasts stack vertically from the top-right corner, each occupying one line. The most recent toast appears below the previous ones. If the stack grows beyond the available terminal height, excess toasts are clipped.

The toast area width is capped to one-third of the terminal width, clamped between 20 and 50 columns. Long messages are truncated with an ellipsis.

## Rendering

The runtime renders toasts after all panels and overlays, so they appear on top of everything. Each toast line consists of:

1. An icon span (colored by severity, bold).
2. The message text (in `theme.text`).
3. A background fill using `theme.surface`.

Toasts do not block input. Key events and mouse events pass through to the panels and overlays underneath.

## ToastManager

Internally, the runtime manages toasts through a `ToastManager`:

```rust
pub struct ToastManager {
    toasts: Vec<Toast>,
}
```

| Method | Behavior |
|---|---|
| `push(toast)` | Add a toast to the queue |
| `tick()` | Remove expired toasts (called every frame) |
| `is_empty()` | Check if any toasts are active |
| `toasts()` | Borrow the current toast list |

You do not interact with `ToastManager` directly in normal usage. The `ctx.toast()` method creates a `Toast` and enqueues it through the runtime's intent system.

## Toast struct

```rust
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub(crate) created: Instant,
    pub(crate) ttl: Duration,
}
```

| Method | Behavior |
|---|---|
| `new(message, level)` | Create a toast with a 3-second TTL |
| `with_ttl(duration)` | Override the default TTL |
| `is_expired()` | Check if the toast has outlived its TTL |

## Example: feedback on every action

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::ItemCreated(name) => {
            self.items.push(name.clone());
            ctx.toast(format!("Created '{name}'"), ToastLevel::Success);
        }
        Msg::ItemDeleted(name) => {
            self.items.retain(|i| i != &name);
            ctx.toast(format!("Deleted '{name}'"), ToastLevel::Info);
        }
        Msg::ValidationFailed(reason) => {
            ctx.toast(reason, ToastLevel::Warning);
        }
        _ => {}
    }
    Command::none()
}
```
