# Async Commands

Kitz applications follow The Elm Architecture: your `update` function is synchronous and returns a `Command` value describing side-effects. For operations that take time -- network requests, file I/O, heavy computation -- `Command::perform()` spawns work on a background thread and maps the result back into a message. The UI remains responsive throughout.

## `Command::perform()` Signature

```rust
pub fn perform<T, F, Map>(task: F, mapper: Map) -> Command<M>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    Map: FnOnce(T) -> M + Send + 'static,
```

- **`task`** -- A closure that runs on a new thread. It can perform blocking I/O, sleep, or do anything that would freeze the UI if run on the main thread.
- **`mapper`** -- A closure that converts the task's return value into a message. The resulting message is sent back to `Application::update()` through the runtime's message channel.

The generic parameter `T` is the intermediate result type. It must be `Send + 'static` because it crosses a thread boundary.

## How It Works Internally

When the runtime encounters a `Command::perform()` action, it:

1. Clones the message sender channel.
2. Spawns a new `std::thread`.
3. Inside the thread, calls `task()` to get the result.
4. Passes the result through `mapper()` to produce a message.
5. Sends the message through the channel.
6. On the next event loop iteration, the runtime picks up the message and calls `update()`.

Because the task runs on a separate thread, the main event loop continues drawing frames and responding to key presses while the work happens in the background.

## Basic Example: HTTP Fetch

```rust
use kitz::prelude::*;

struct App {
    body: Option<String>,
    loading: bool,
}

#[derive(Debug, Clone)]
enum Msg {
    Fetch,
    FetchDone(Result<String, String>),
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Fetch => {
                self.loading = true;
                Command::perform(
                    || {
                        let resp = reqwest::blocking::get("https://httpbin.org/get")
                            .and_then(|r| r.text())
                            .map_err(|e| e.to_string());
                        resp
                    },
                    Msg::FetchDone,
                )
            }
            Msg::FetchDone(result) => {
                self.loading = false;
                match result {
                    Ok(text) => self.body = Some(text),
                    Err(e) => self.body = Some(format!("Error: {}", e)),
                }
                Command::none()
            }
        }
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Enter {
                return EventResult::Message(Msg::Fetch);
            }
        }
        EventResult::Ignored
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let text = if self.loading {
            "Loading...".to_string()
        } else {
            self.body.clone().unwrap_or_else(|| "Press Enter to fetch".into())
        };
        frame.render_widget(Paragraph::new(text), frame.area());
    }
}
```

The key insight is that `update` returns immediately with a `Command::perform(...)`. The runtime spawns the HTTP request on a background thread. While the request is in flight, the UI keeps rendering -- showing "Loading..." -- and the user can still press keys. When the request completes, `Msg::FetchDone` arrives and `update` is called again with the result.

## Using the Mapper Function

The mapper converts the task's raw result into a message. You can use it to transform data, handle errors, or wrap results:

```rust
Command::perform(
    || std::fs::read_to_string("/etc/hostname"),
    |result| match result {
        Ok(content) => Msg::FileLoaded(content.trim().to_string()),
        Err(e) => Msg::FileError(e.to_string()),
    },
)
```

If your message variant already matches the task's return type, you can pass it directly as a function pointer:

```rust
Command::perform(
    || compute_expensive_thing(),
    Msg::ComputeDone,  // Msg::ComputeDone(ComputeResult)
)
```

## Multiple Concurrent Tasks

`Command::batch()` lets you fire multiple background tasks at once:

```rust
Command::batch([
    Command::perform(|| fetch_users(), Msg::UsersLoaded),
    Command::perform(|| fetch_config(), Msg::ConfigLoaded),
])
```

Both tasks run concurrently on separate threads. Their result messages arrive independently, in whatever order they finish.

## Chaining Commands

A common pattern is to kick off a follow-up task when the first one completes:

```rust
fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Start => Command::perform(|| step_one(), Msg::StepOneDone),
        Msg::StepOneDone(data) => {
            self.intermediate = data.clone();
            Command::perform(move || step_two(data), Msg::StepTwoDone)
        }
        Msg::StepTwoDone(final_result) => {
            self.result = Some(final_result);
            Command::none()
        }
    }
}
```

Each step returns a `Command::perform` that triggers the next stage. The application state is updated between steps, and the UI can reflect progress.

## Why the UI Stays Responsive

Traditional TUI applications often block the main thread during I/O, freezing the entire interface. Kitz avoids this through its command model:

1. `update()` never blocks. It returns a `Command` immediately.
2. Background work runs on a dedicated `std::thread`.
3. Results arrive through an `mpsc` channel that the runtime drains on each frame.
4. The event loop continues polling terminal events and redrawing the screen.

This means that even during a long-running network request or file operation, the user can scroll, switch panels, open overlays, or quit the application.

## Testing Async Commands

`TestHarness` deliberately skips `Command::perform()` actions. Background threads are not spawned during tests. To test the logic that depends on an async result, send the result message directly:

```rust
let mut harness = TestHarness::new(App::new());
harness.send_message(Msg::Fetch);
assert!(harness.app().loading);

// Simulate the background task completing
harness.send_message(Msg::FetchDone(Ok("mock data".into())));
assert!(!harness.app().loading);
assert_eq!(harness.app().body, Some("mock data".into()));
```

See [Testing](testing.md) for more details on the test harness.

## Other Command Types

`Command::perform()` is the async workhorse, but kitz provides other command constructors for common cases:

| Constructor | Purpose |
|---|---|
| `Command::none()` | No side-effect |
| `Command::quit()` | Tell the runtime to shut down |
| `Command::message(msg)` | Immediately re-dispatch a message through `update()` |
| `Command::batch(cmds)` | Combine multiple commands into one |
| `Command::perform(task, mapper)` | Spawn a background thread |

All commands are values, not callbacks. Your `update` function stays pure and testable, while the runtime handles execution.
