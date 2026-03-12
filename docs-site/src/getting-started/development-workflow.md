# Development Workflow

Kitz provides CLI commands that wrap common Cargo workflows with sensible defaults. This page covers the development loop, auto-reload, testing, and logging.

---

## The Development Loop

The typical cycle when building a kitz application:

1. Edit your code (messages, update logic, panel views).
2. See the result in the terminal.
3. Check for correctness with tests.
4. Read logs when you need to trace behavior.

Kitz optimizes each step.

---

## `kitz dev` — Auto-Reload

```
kitz dev
```

This starts your application with automatic recompilation and restart on every file change. Under the hood, it runs `cargo watch -x run -c`:

- **`cargo watch`** monitors your `src/` directory for changes.
- On any change, it recompiles and reruns your application.
- The `-c` flag clears the terminal between runs for a clean view.

If `cargo-watch` is not installed, `kitz dev` installs it automatically via `cargo install cargo-watch`.

This is the recommended way to develop. Edit a panel's `view` method, save, and see the updated UI within seconds.

### When to Use `kitz dev` vs `cargo run`

Use `kitz dev` during active development when you are iterating on the UI or behavior. Use `cargo run` (or `kitz run`) when you want a single build-and-run cycle, for example to demo the app or verify a specific state.

---

## `kitz run` — Build and Run

```
kitz run
```

Compiles and runs your application in debug mode. Equivalent to `cargo run`, but printed with kitz's formatted output.

For a release build:

```
kitz run --release
```

This compiles with optimizations. Use it for performance testing or distribution.

---

## `kitz test` — Run Tests

```
kitz test
```

Runs your project's test suite via `cargo test`. Kitz applications are tested with `TestHarness`, which simulates the event loop without a terminal.

A typical test:

```rust
use kitz::prelude::*;

#[test]
fn sidebar_navigation() {
    let mut harness = TestHarness::new(App::new());

    harness.press_panel_key("sidebar", KeyCode::Char('j'));
    assert_eq!(harness.app().sidebar.selected, 1);

    harness.press_panel_key("sidebar", KeyCode::Char('j'));
    harness.press_panel_key("sidebar", KeyCode::Char('k'));
    assert_eq!(harness.app().sidebar.selected, 1);
}

#[test]
fn delete_shows_confirmation() {
    let mut harness = TestHarness::new(App::new());
    harness.send_message(Msg::ConfirmDelete);
    // The overlay was pushed via ctx.push_overlay — verify state
    assert_eq!(harness.app().contacts.len(), 3); // not yet deleted

    harness.send_message(Msg::DeleteConfirmed);
    assert_eq!(harness.app().contacts.len(), 2);
}

#[test]
fn quit_command_works() {
    let mut harness = TestHarness::new(App::new());
    assert!(!harness.quit_requested());
    harness.send_message(Msg::Quit);
    assert!(harness.quit_requested());
}
```

`TestHarness` provides:

| Method | Purpose |
|--------|---------|
| `TestHarness::new(app)` | Create a harness, process the `init()` command |
| `harness.app()` | Borrow the application state for assertions |
| `harness.app_mut()` | Mutably borrow the application state |
| `harness.press_key(KeyCode)` | Simulate a key press routed through `handle_event` |
| `harness.send_key(KeyCode, KeyModifiers)` | Simulate a key press with modifiers |
| `harness.press_panel_key(panel_id, KeyCode)` | Simulate a key press routed through `panel_handle_key` |
| `harness.send_message(msg)` | Dispatch a message directly to `update` |
| `harness.quit_requested()` | Check if `Command::quit()` was returned |

Background tasks (`Command::perform`) are not executed in the test harness. To test async workflows, call `send_message` with the message your background task would produce.

### Watch Mode

```
kitz test --watch
```

Re-runs the full test suite on every file change. Uses `cargo watch -x test -c` under the hood, with the same auto-install behavior as `kitz dev`.

This pairs well with a split-terminal setup: your app in one pane, tests in another.

---

## Logging

TUI applications cannot write to stdout or stderr because the terminal is in alternate screen mode. Kitz provides file-based logging via the `tracing` ecosystem.

### Enabling Logging

Call `kitz::logging::init_logging` before `kitz::run`:

```rust
use kitz::prelude::*;

fn main() -> Result<()> {
    let _guard = kitz::logging::init_logging("my-app");
    kitz::run(App::new())
}
```

The returned guard must be held for the lifetime of the application. When it drops, pending log writes are flushed. Bind it to a variable with a leading underscore to suppress the unused-variable warning while keeping the guard alive.

### Where Logs Go

Logs are written to `~/.local/share/kitz/<app-name>/app.log` with daily rotation. On systems where `$XDG_DATA_HOME` is set, that directory is used instead of `~/.local/share`.

If neither `$XDG_DATA_HOME` nor `$HOME` is set (rare), logs fall back to `./.kitz-logs/` in the current working directory.

### Tailing Logs

In a separate terminal, run:

```
tail -f ~/.local/share/kitz/my-app/app.log
```

This gives you a live stream of log output while your application runs in another terminal.

### Writing Log Statements

Kitz uses the `tracing` crate. Add log statements anywhere in your code:

```rust
use tracing::{info, warn, debug, error};

fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::SelectNext => {
            self.selected += 1;
            debug!(selected = self.selected, "moved to next item");
        }
        Msg::DeleteConfirmed => {
            let name = &self.contacts[self.selected].name;
            info!(name = %name, "contact deleted");
        }
        Msg::FetchError(e) => {
            error!(error = %e, "failed to fetch data");
        }
    }
    Command::none()
}
```

Log output includes timestamps, thread IDs, and target modules. A typical log line:

```
2026-03-13T10:42:07.123456Z  INFO my_app::app: contact deleted name="Alice Chen"
```

### Logging Is Opt-In

If you do not call `init_logging`, no log files are created and no log infrastructure is initialized. The `tracing` macros become no-ops with zero runtime cost. This is intentional — simple examples and demos should not leave files on disk.

---

## Putting It All Together

A recommended terminal layout during development:

```
┌─────────────────────────────────────┬─────────────────────────────┐
│                                     │                             │
│  Terminal 1: kitz dev               │  Terminal 2: kitz test      │
│  (your running app, auto-reloads)   │  --watch                    │
│                                     │  (tests re-run on save)     │
│                                     │                             │
├─────────────────────────────────────┴─────────────────────────────┤
│                                                                   │
│  Terminal 3: tail -f ~/.local/share/kitz/my-app/app.log           │
│  (live log stream)                                                │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

Save a file. Watch the app rebuild in terminal 1, tests re-run in terminal 2, and log output stream in terminal 3.

---

## Next Steps

- [Testing](../advanced/testing.md) — advanced test patterns, testing overlays and subscriptions.
- [Logging](../advanced/logging.md) — configuring log levels, filtering, and custom appenders.
- [kitz generate](../cli/generate-command.md) — adding panels, screens, and overlays with the CLI.
