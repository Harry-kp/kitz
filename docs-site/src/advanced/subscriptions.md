# Subscriptions

Subscriptions let your application receive periodic messages without manually managing threads or timers. You declare what subscriptions you want, and the runtime handles starting, stopping, and restarting them as your application state changes.

## How Subscriptions Work

The runtime calls `Application::subscriptions()` on every frame. It compares the returned set of subscription IDs against the currently active set. If a new ID appears, the runtime spawns a background thread for it. If an ID disappears, the runtime cancels the corresponding thread. This declarative model means you never have to manually manage background thread lifecycles.

## `Subscription::every()`

The primary subscription type is `Subscription::every()`, which emits a message at a fixed interval:

```rust
use std::time::Duration;
use kitz::prelude::*;

Subscription::every("clock-tick", Duration::from_secs(1), || Msg::Tick)
```

The three arguments are:

1. **`id`** -- A unique `&'static str` identifier. The runtime uses this to track which subscriptions are active.
2. **`interval`** -- A `Duration` between each message emission.
3. **`msg_fn`** -- A closure that produces the message to dispatch. This closure is called on each interval tick.

## Declaring Subscriptions

Override the `subscriptions()` method on your `Application` implementation:

```rust
fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
    vec![
        Subscription::every("clock-tick", Duration::from_secs(1), || Msg::Tick),
    ]
}
```

If you have no subscriptions, you can omit the method entirely -- the default returns an empty vector.

## State-Dependent Subscriptions

The real power of subscriptions is that they are re-evaluated every frame. You can conditionally include or exclude subscriptions based on application state, and the runtime will start or stop the corresponding background threads automatically.

```rust
fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
    let mut subs = Vec::new();

    if self.polling_enabled {
        subs.push(Subscription::every(
            "api-poll",
            Duration::from_secs(5),
            || Msg::PollApi,
        ));
    }

    if self.show_clock {
        subs.push(Subscription::every(
            "clock",
            Duration::from_millis(500),
            || Msg::ClockTick,
        ));
    }

    subs
}
```

When `self.polling_enabled` changes from `true` to `false`, the runtime automatically cancels the `"api-poll"` subscription thread. When it flips back, the runtime spawns a new one. You never write thread management code.

## Complete Example: Periodic Polling

Below is a full application that polls an API every ten seconds, displaying the result and allowing the user to toggle polling on and off.

```rust
use std::time::Duration;
use kitz::prelude::*;

struct App {
    polling: bool,
    last_result: Option<String>,
    poll_count: usize,
}

#[derive(Debug, Clone)]
enum Msg {
    TogglePolling,
    PollNow,
    PollResult(String),
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::TogglePolling => {
                self.polling = !self.polling;
                Command::none()
            }
            Msg::PollNow => {
                self.poll_count += 1;
                let count = self.poll_count;
                Command::perform(
                    move || format!("Poll #{} at {:?}", count, std::time::Instant::now()),
                    |result| Msg::PollResult(result),
                )
            }
            Msg::PollResult(data) => {
                self.last_result = Some(data);
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let status = if self.polling { "ON" } else { "OFF" };
        let data = self
            .last_result
            .as_deref()
            .unwrap_or("No data yet");
        let text = format!(
            "Polling: {} (press 'p' to toggle)\nLast result: {}",
            status, data
        );
        frame.render_widget(Paragraph::new(text), frame.area());
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('p') {
                return EventResult::Message(Msg::TogglePolling);
            }
        }
        EventResult::Ignored
    }

    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        if self.polling {
            vec![Subscription::every(
                "api-poll",
                Duration::from_secs(10),
                || Msg::PollNow,
            )]
        } else {
            Subscription::none()
        }
    }
}

fn main() -> Result<()> {
    kitz::run(App {
        polling: true,
        last_result: None,
        poll_count: 0,
    })
}
```

## Subscription IDs Must Be Unique

Each subscription must have a distinct ID string. If two subscriptions share the same ID, the runtime treats them as a single subscription and will not start a duplicate. Use descriptive, static strings:

```rust
Subscription::every("heartbeat", Duration::from_secs(30), || Msg::Heartbeat)
Subscription::every("progress-refresh", Duration::from_secs(2), || Msg::RefreshProgress)
```

## Lifecycle Summary

| Event | Runtime behavior |
|---|---|
| New ID appears in returned `Vec` | Spawns a background thread that emits messages at the given interval |
| Existing ID still present | No change -- thread continues running |
| ID disappears from returned `Vec` | Sends a cancellation signal; thread exits on its next wake |
| Application quits | All subscription threads are shut down and joined |

Subscriptions are one of the safest ways to introduce periodic behavior into a kitz application. The runtime owns the threads, handles cancellation, and cleans up on exit. Your application code stays purely declarative.
