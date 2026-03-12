# Testing

Kitz provides `TestHarness`, a headless test runner that lets you simulate key presses, send messages, and assert on application state without a real terminal. No screen rendering occurs during tests, so they run fast and can be used in CI environments.

## Setting Up TestHarness

`TestHarness` wraps any type that implements `Application`:

```rust
use kitz::prelude::*;

let mut harness = TestHarness::new(MyApp::new());
```

When constructed, the harness automatically processes the `init()` command returned by your application, so any initialization logic runs before your first assertion.

## Core API

### `press_key(code: KeyCode)`

Simulates a key press with no modifiers. The harness routes the event through `handle_event()`. If the result is `EventResult::Message(msg)`, the message is dispatched to `update()`.

```rust
harness.press_key(KeyCode::Char('j'));
harness.press_key(KeyCode::Enter);
harness.press_key(KeyCode::Esc);
```

### `send_key(code: KeyCode, modifiers: KeyModifiers)`

Like `press_key`, but with explicit modifiers:

```rust
harness.send_key(KeyCode::Char('s'), KeyModifiers::CONTROL);
```

### `press_panel_key(panel_id: &'static str, code: KeyCode)`

Sends a key event directly to a specific panel's `panel_handle_key` handler, bypassing `handle_event()`. Useful for testing panel-specific key bindings in isolation:

```rust
harness.press_panel_key("sidebar", KeyCode::Char('j'));
assert_eq!(harness.app().sidebar_selected, 1);
```

### `send_message(msg: A::Message)`

Dispatches a message directly to `update()`, skipping event handling entirely. This is the primary way to test your update logic and to simulate the results of async commands:

```rust
harness.send_message(Msg::Increment);
harness.send_message(Msg::FetchDone(Ok("data".into())));
```

### `app() -> &A`

Returns a shared reference to the application state for assertions:

```rust
assert_eq!(harness.app().count, 5);
assert!(harness.app().items.contains(&"hello".to_string()));
```

### `app_mut() -> &mut A`

Returns a mutable reference, allowing you to set up specific states before testing:

```rust
harness.app_mut().items = vec!["a".into(), "b".into(), "c".into()];
harness.press_key(KeyCode::Char('j'));
assert_eq!(harness.app().selected, 1);
```

### `quit_requested() -> bool`

Returns `true` if any processed command returned `Command::quit()`:

```rust
harness.send_message(Msg::ConfirmQuit);
assert!(harness.quit_requested());
```

## Command::perform Is Skipped in Tests

When `TestHarness` encounters a `Command::perform()` action, it silently discards it. No background threads are spawned. This is intentional:

- Tests should be deterministic and fast.
- Network calls, file I/O, and other side-effects should not run during unit tests.
- You simulate async results by calling `send_message()` with the expected result message.

For example, if your `update` spawns an HTTP fetch:

```rust
Msg::Fetch => {
    self.loading = true;
    Command::perform(|| http_get("/api/data"), Msg::FetchDone)
}
```

Test it like this:

```rust
harness.send_message(Msg::Fetch);
assert!(harness.app().loading);

// The Command::perform was skipped. Simulate its result:
harness.send_message(Msg::FetchDone(Ok("response body".into())));
assert!(!harness.app().loading);
assert_eq!(harness.app().data, Some("response body".into()));
```

`Command::message()` and `Command::batch()` are fully processed. `Command::quit()` sets the `quit_requested` flag.

## Complete Test Example: Counter App

```rust
#[cfg(test)]
mod tests {
    use kitz::prelude::*;

    struct Counter {
        count: i32,
    }

    #[derive(Debug, Clone)]
    enum Msg {
        Increment,
        Decrement,
        Reset,
        Quit,
    }

    impl Application for Counter {
        type Message = Msg;

        fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
            match msg {
                Msg::Increment => self.count += 1,
                Msg::Decrement => self.count -= 1,
                Msg::Reset => self.count = 0,
                Msg::Quit => return Command::quit(),
            }
            Command::none()
        }

        fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('j') => return EventResult::Message(Msg::Increment),
                    KeyCode::Char('k') => return EventResult::Message(Msg::Decrement),
                    KeyCode::Char('r') => return EventResult::Message(Msg::Reset),
                    _ => {}
                }
            }
            EventResult::Ignored
        }
    }

    #[test]
    fn initial_state() {
        let harness = TestHarness::new(Counter { count: 0 });
        assert_eq!(harness.app().count, 0);
        assert!(!harness.quit_requested());
    }

    #[test]
    fn key_presses_dispatch_messages() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('j'));
        assert_eq!(harness.app().count, 3);

        harness.press_key(KeyCode::Char('k'));
        assert_eq!(harness.app().count, 2);
    }

    #[test]
    fn direct_message_dispatch() {
        let mut harness = TestHarness::new(Counter { count: 10 });
        harness.send_message(Msg::Decrement);
        harness.send_message(Msg::Decrement);
        assert_eq!(harness.app().count, 8);
    }

    #[test]
    fn reset_works() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        harness.send_message(Msg::Increment);
        harness.send_message(Msg::Increment);
        harness.press_key(KeyCode::Char('r'));
        assert_eq!(harness.app().count, 0);
    }

    #[test]
    fn quit_sets_flag() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        harness.send_message(Msg::Quit);
        assert!(harness.quit_requested());
    }

    #[test]
    fn unhandled_keys_are_ignored() {
        let mut harness = TestHarness::new(Counter { count: 5 });
        harness.press_key(KeyCode::Char('x'));
        assert_eq!(harness.app().count, 5);
    }
}
```

## Testing Panel Applications

For apps that use the panel system, `press_panel_key` lets you target specific panels:

```rust
#[cfg(test)]
mod tests {
    use kitz::prelude::*;

    struct PanelApp {
        sidebar_selected: usize,
        main_scroll: usize,
    }

    #[derive(Debug, Clone)]
    enum Msg {
        SidebarNext,
        SidebarPrev,
        MainScrollDown,
    }

    impl Application for PanelApp {
        type Message = Msg;

        fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
            match msg {
                Msg::SidebarNext => self.sidebar_selected += 1,
                Msg::SidebarPrev => {
                    self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
                }
                Msg::MainScrollDown => self.main_scroll += 1,
            }
            Command::none()
        }

        fn panels(&self) -> PanelLayout {
            PanelLayout::horizontal(vec![
                ("sidebar", Constraint::Percentage(30)),
                ("main", Constraint::Percentage(70)),
            ])
        }

        fn panel_handle_key(
            &mut self,
            panel: PanelId,
            key: &KeyEvent,
        ) -> EventResult<Msg> {
            match panel {
                "sidebar" => match key.code {
                    KeyCode::Char('j') => EventResult::Message(Msg::SidebarNext),
                    KeyCode::Char('k') => EventResult::Message(Msg::SidebarPrev),
                    _ => EventResult::Ignored,
                },
                "main" => match key.code {
                    KeyCode::Char('j') => EventResult::Message(Msg::MainScrollDown),
                    _ => EventResult::Ignored,
                },
                _ => EventResult::Ignored,
            }
        }
    }

    #[test]
    fn panel_keys_are_independent() {
        let mut harness = TestHarness::new(PanelApp {
            sidebar_selected: 0,
            main_scroll: 0,
        });

        harness.press_panel_key("sidebar", KeyCode::Char('j'));
        harness.press_panel_key("sidebar", KeyCode::Char('j'));
        assert_eq!(harness.app().sidebar_selected, 2);
        assert_eq!(harness.app().main_scroll, 0);

        harness.press_panel_key("main", KeyCode::Char('j'));
        assert_eq!(harness.app().main_scroll, 1);
        assert_eq!(harness.app().sidebar_selected, 2);
    }
}
```

## Testing Command Chains

`Command::message()` is fully processed, so you can test chains where one update triggers another:

```rust
#[test]
fn command_message_chains() {
    let mut harness = TestHarness::new(ChainApp { log: vec![] });
    harness.send_message(Msg::Start);
    // Start dispatches Command::message(Msg::Step("chained"))
    assert_eq!(harness.app().log, vec!["start", "chained"]);
}
```

## Best Practices

- Test your `update` logic through `send_message()` for direct, deterministic coverage.
- Test key bindings through `press_key()` and `press_panel_key()` to verify the full event-to-message-to-state pipeline.
- Simulate async results by calling `send_message()` with both success and error variants.
- Check `quit_requested()` to verify quit flows without actually terminating a process.
- Keep test apps minimal. You do not need to implement `view()` for tests -- the harness never renders.
