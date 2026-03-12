# Messages and Commands

Kitz applications communicate through two complementary mechanisms: **messages** describe what happened, and **commands** describe what should happen next. Together, they form the backbone of the TEA data flow --- events become messages, messages drive state transitions in `update()`, and `update()` returns commands that produce more messages.

## Messages

A message is a value from your application's `Message` enum. You define this type yourself; kitz imposes only three trait bounds:

- `Debug` --- so messages can be logged for debugging.
- `Send` --- so messages can arrive from background threads.
- `'static` --- messages are owned values, not borrowed references.

```rust
#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
    Reset,
    TimerTick,
    FetchStarted,
    FetchComplete(Result<String, String>),
    ConfirmQuit,
    UserConfirmedQuit,
}
```

### Design guidelines

**Name messages after what happened, not what to do.** `UserPressedSave` is better than `WriteFile` because it keeps the decision logic in `update()` where it belongs. The message describes the trigger; `update()` decides the response.

**Carry data in variants.** Messages can hold arbitrary data. Use this to pass results from async operations, user input, or any value that `update()` needs to process the event.

**Keep the enum flat when possible.** Nested enums (e.g., `Msg::Panel(PanelMsg)`) are valid Rust, but they add indirection. A flat enum with clear variant names is usually easier to follow in `update()`.

## EventResult

When mapping raw terminal events to messages, kitz uses `EventResult<M>` rather than `Option<M>`. This three-variant enum gives the event system enough information to decide whether to continue propagating the event.

```rust
pub enum EventResult<M> {
    Message(M),
    Ignored,
    Consumed,
}
```

### `EventResult::Message(M)`

The event was handled and produced a message. The runtime will dispatch this message to `update()`. The event is considered fully handled and will not propagate further.

### `EventResult::Ignored`

The handler did not recognize or care about this event. The runtime will continue propagating it to the next handler in the chain (focused panel, then convention keys).

### `EventResult::Consumed`

The handler recognized the event and dealt with it, but there is no message to dispatch. The event is considered fully handled. Use this when you want to swallow an event without triggering a state transition --- for example, suppressing a key press that is not relevant in the current mode.

The distinction between `Ignored` and `Consumed` is critical for the event cascade. If `handle_event()` returns `Ignored`, the focused panel gets a chance to handle the key. If it returns `Consumed`, the panel never sees it. Choose deliberately.

## Commands

A `Command<M>` is a **value** that describes a side effect. It is returned from `update()` and executed by the runtime after `update()` returns. Your application never executes side effects inline --- it describes them, and the runtime carries them out.

This is a deliberate design choice inspired by Elm and Iced. It keeps `update()` deterministic and testable: given the same model and message, `update()` always returns the same command, regardless of external state.

### `Command::none()`

No side effects. This is the most common return value. Most messages simply mutate the model and need nothing further.

```rust
Msg::Increment => {
    self.count += 1;
    Command::none()
}
```

### `Command::quit()`

Tell the runtime to shut down the application. The terminal is restored and the process exits cleanly.

```rust
Msg::UserConfirmedQuit => {
    Command::quit()
}
```

### `Command::message(msg)`

Immediately re-dispatch a message through `update()`. This is useful when one message should trigger another, or when you want to decompose a complex update into smaller steps.

```rust
Msg::FormSubmitted => {
    self.form_visible = false;
    Command::message(Msg::SaveData)
}
```

The re-dispatched message goes through `update()` in the same event-loop iteration, before the next render. This is synchronous and deterministic.

### `Command::batch(cmds)`

Combine multiple commands into one. All actions from all commands execute. Use this when a single message needs to trigger several effects.

```rust
Msg::AppStarted => {
    Command::batch([
        Command::perform(|| load_config(), Msg::ConfigLoaded),
        Command::perform(|| load_user_data(), Msg::UserDataLoaded),
        Command::message(Msg::ShowSplash),
    ])
}
```

The actions within a batch are processed in order, but `perform` tasks are spawned onto threads and run concurrently. Message and quit actions are processed synchronously.

### `Command::perform(task, mapper)`

Spawn a blocking closure on a background thread. When the closure completes, the `mapper` function converts its return value into a message, which is sent back to `update()`.

```rust
Msg::FetchStarted => {
    self.state = LoadingState::InProgress;
    Command::perform(
        || {
            let resp = reqwest::blocking::get("https://api.example.com/data");
            resp.and_then(|r| r.text()).map_err(|e| e.to_string())
        },
        |result| Msg::FetchComplete(result),
    )
}
```

The type signature is:

```rust
pub fn perform<T, F, Map>(task: F, mapper: Map) -> Command<M>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    Map: FnOnce(T) -> M + Send + 'static,
```

Key points:

- `task` runs on a new thread. It can block for as long as it needs.
- `mapper` runs on the same background thread, immediately after `task` completes.
- The resulting message is sent through a channel and picked up by the runtime on the next event-loop iteration.
- The UI remains responsive while the task runs. The runtime continues rendering and processing events.

### How the runtime executes commands

After `update()` returns, the runtime iterates over the command's actions:

1. **Quit** --- sets the quit flag and stops processing.
2. **Message** --- calls `update()` again immediately (recursively draining any commands the new call produces).
3. **Perform** --- spawns a thread. The message arrives asynchronously on a future iteration.

This means `Command::message` chains are fully resolved before the next render, while `Command::perform` results arrive later. Design your loading states accordingly.

## Putting It Together

Here is a complete example showing messages, event results, and commands working together:

```rust
struct App {
    query: String,
    results: Vec<String>,
    loading: bool,
}

#[derive(Debug, Clone)]
enum Msg {
    UpdateQuery(String),
    Search,
    SearchComplete(Result<Vec<String>, String>),
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::UpdateQuery(q) => {
                self.query = q;
                Command::none()
            }
            Msg::Search => {
                self.loading = true;
                let query = self.query.clone();
                Command::perform(
                    move || search_api(&query),
                    Msg::SearchComplete,
                )
            }
            Msg::SearchComplete(Ok(results)) => {
                self.loading = false;
                self.results = results;
                ctx.toast("Search complete", ToastLevel::Info);
                Command::none()
            }
            Msg::SearchComplete(Err(e)) => {
                self.loading = false;
                ctx.toast(format!("Error: {}", e), ToastLevel::Error);
                Command::none()
            }
        }
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => return EventResult::Message(Msg::Search),
                _ => {}
            }
        }
        EventResult::Ignored
    }

    // ... view methods omitted for brevity
}
```

The flow: the user types a query (producing `UpdateQuery` messages), presses Enter (mapped to `Msg::Search` by `handle_event`), `update()` sets the loading flag and spawns a background search, and when the search completes, `SearchComplete` arrives with the results or an error. Every step is explicit, every side effect is declared, and the entire sequence is testable by constructing messages directly.
