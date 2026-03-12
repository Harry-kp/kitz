# Logging

Terminal user interfaces own stdout and stderr. Calling `println!` or `eprintln!` while the TUI is running corrupts the screen because ratatui uses an alternate screen buffer that expects exclusive control of terminal output. Kitz provides a file-based logging setup that integrates with the `tracing` ecosystem to give you structured, inspectable logs without interfering with your UI.

## Why `println!` Corrupts the TUI

When a ratatui application enters the alternate screen, all terminal output is expected to come through the `Frame` rendering API. A stray `println!` writes bytes directly to stdout, which lands in the middle of the terminal control sequences that ratatui uses to draw widgets. The result is garbled output, misplaced text, or a completely broken display.

The solution is to redirect all log output to a file. Kitz makes this easy with a single function call.

## Setting Up Logging

Call `kitz::logging::init_logging()` before starting the runtime:

```rust
fn main() -> kitz::prelude::Result<()> {
    let _guard = kitz::logging::init_logging("my-app");

    kitz::run(App::new())
}
```

The function returns an `Option<WorkerGuard>`. The guard must be held for the lifetime of the application -- dropping it flushes any pending log writes. Bind it to a variable with a leading underscore (`_guard`) so the compiler does not warn about an unused value, but the variable stays alive until `main` returns.

The argument is your application's name. Logs are written to:

```
~/.local/share/kitz/<app-name>/app.log
```

On systems where `XDG_DATA_HOME` is set, that directory is used instead of `~/.local/share`. If neither `XDG_DATA_HOME` nor `HOME` is available, logs fall back to `.kitz-logs/` in the current working directory.

## Log File Rotation

`init_logging` uses `tracing_appender::rolling::daily`, which creates a new log file each day. Old log files are not automatically deleted, so you can review historical logs when debugging issues that span multiple sessions.

## Using `tracing` Macros

Once logging is initialized, use the standard `tracing` macros anywhere in your application:

```rust
use tracing::{info, debug, warn, error, trace};

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::ItemSelected(idx) => {
            info!(index = idx, "Item selected");
            self.selected = idx;
            Command::none()
        }
        Msg::FetchDone(Err(e)) => {
            error!(error = %e, "API fetch failed");
            self.error = Some(e);
            Command::none()
        }
        Msg::Tick => {
            trace!("Tick received");
            Command::none()
        }
    }
}
```

Log output includes timestamps, thread IDs, and target modules by default. A typical log line looks like:

```
2026-03-13T10:42:15.123456Z  INFO ThreadId(01) my_app::app: Item selected index=3
```

## Structured Fields

`tracing` supports structured key-value pairs. Use them to add context without formatting strings:

```rust
debug!(
    panel = "sidebar",
    items = self.items.len(),
    "Panel refreshed"
);

warn!(
    threshold = 100,
    current = self.queue.len(),
    "Queue approaching capacity"
);
```

## Complete Setup Example

```rust
use kitz::prelude::*;
use tracing::{info, debug};

struct App {
    count: i32,
}

#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Increment => {
                self.count += 1;
                debug!(count = self.count, "Incremented");
            }
            Msg::Decrement => {
                self.count -= 1;
                debug!(count = self.count, "Decremented");
            }
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        frame.render_widget(
            Paragraph::new(format!("Count: {}", self.count)),
            frame.area(),
        );
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') => return EventResult::Message(Msg::Increment),
                KeyCode::Char('k') => return EventResult::Message(Msg::Decrement),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

fn main() -> Result<()> {
    let _guard = kitz::logging::init_logging("counter");
    info!("Starting counter app");
    kitz::run(App { count: 0 })
}
```

## Viewing Logs

While the application is running (or after it exits), tail the log file in a separate terminal:

```bash
tail -f ~/.local/share/kitz/counter/app.log
```

This is the primary debugging technique for kitz applications. Instead of scattering `println!` calls and corrupting the UI, write structured log statements and monitor them from a second terminal.

## Logging Is Opt-In

The runtime does not call `init_logging` automatically. If you do not set it up, no log files are created. This keeps simple examples and quick prototypes clean. Add logging when you need it for debugging or production monitoring.
