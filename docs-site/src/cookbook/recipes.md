# Recipes

A collection of self-contained patterns for common kitz tasks. Each recipe includes a brief explanation and a complete code snippet you can adapt to your project.

---

## 1. Custom Key Bindings

Override `handle_event` to map any key combination to a message. Return `EventResult::Consumed` to prevent the event from reaching panel handlers or convention keys.

```rust
fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                EventResult::Message(Msg::Save)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                EventResult::Message(Msg::NewItem)
            }
            (KeyModifiers::ALT, KeyCode::Char('1')) => {
                EventResult::Message(Msg::FocusSidebar)
            }
            (KeyModifiers::ALT, KeyCode::Char('2')) => {
                EventResult::Message(Msg::FocusMain)
            }
            _ => EventResult::Ignored,
        }
    } else {
        EventResult::Ignored
    }
}
```

Handle the focus messages in `update`:

```rust
Msg::FocusSidebar => {
    ctx.focus_panel("sidebar");
    Command::none()
}
Msg::FocusMain => {
    ctx.focus_panel("main");
    Command::none()
}
```

---

## 2. Background Tasks

Use `Command::perform` to run work on a background thread. The UI stays responsive while the task executes.

```rust
#[derive(Debug, Clone)]
enum Msg {
    LoadFile(String),
    FileLoaded(Result<String, String>),
}

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::LoadFile(path) => {
            self.loading = true;
            Command::perform(
                move || std::fs::read_to_string(&path).map_err(|e| e.to_string()),
                Msg::FileLoaded,
            )
        }
        Msg::FileLoaded(result) => {
            self.loading = false;
            match result {
                Ok(content) => self.content = content,
                Err(e) => self.error = Some(e),
            }
            Command::none()
        }
    }
}
```

---

## 3. Periodic Updates

Use `Subscription::every` to emit a message at a fixed interval. The runtime manages the background thread.

```rust
use std::time::Duration;

fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
    vec![
        Subscription::every("clock", Duration::from_secs(1), || Msg::Tick),
    ]
}

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Tick => {
            self.elapsed += 1;
            Command::none()
        }
        // ...
    }
}
```

Make subscriptions conditional on state to auto-start and auto-stop them:

```rust
fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
    if self.timer_running {
        vec![Subscription::every("timer", Duration::from_millis(100), || Msg::TimerTick)]
    } else {
        Subscription::none()
    }
}
```

---

## 4. Modal Dialogs

Use `ConfirmOverlay` for yes/no confirmations. Push it from `update` via the context.

```rust
use kitz::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    AskDelete,
    ConfirmDelete,
    // ...
}

fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::AskDelete => {
            ctx.push_overlay(ConfirmOverlay::new(
                "Delete this item? This cannot be undone.",
                Msg::ConfirmDelete,
            ));
            Command::none()
        }
        Msg::ConfirmDelete => {
            self.items.remove(self.selected);
            Command::none()
        }
        // ...
    }
}
```

The overlay renders centered on screen, handles Enter (confirm) and Esc (cancel), and dispatches the confirmation message when accepted.

---

## 5. Toast Notifications

Show temporary messages that auto-dismiss using the context's `toast` method.

```rust
use kitz::prelude::*;

fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Save => {
            self.save_to_disk();
            ctx.toast("File saved successfully", ToastLevel::Success);
            Command::none()
        }
        Msg::SaveFailed(e) => {
            ctx.toast(format!("Save failed: {}", e), ToastLevel::Error);
            Command::none()
        }
        Msg::Warning(text) => {
            ctx.toast(text, ToastLevel::Warning);
            Command::none()
        }
        // ...
    }
}
```

Toast levels: `ToastLevel::Info`, `ToastLevel::Success`, `ToastLevel::Warning`, `ToastLevel::Error`. Toasts render in the top-right corner and fade out automatically.

---

## 6. Theme Switching

Store the theme in your app state and cycle through built-in palettes at runtime.

```rust
struct App {
    theme: Theme,
    // ...
}

#[derive(Debug, Clone)]
enum Msg {
    CycleTheme,
    // ...
}

impl Application for App {
    type Message = Msg;

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::CycleTheme => {
                self.theme = self.theme.next();
                ctx.toast(
                    format!("Theme: {}", self.theme.name),
                    ToastLevel::Info,
                );
                Command::none()
            }
            // ...
        }
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('t') {
                return EventResult::Message(Msg::CycleTheme);
            }
        }
        EventResult::Ignored
    }
}
```

---

## 7. Multi-Screen Navigation

Use the navigation stack to push and pop full-screen views.

```rust
use kitz::prelude::*;

struct SettingsScreen {
    volume: u8,
}

impl Screen<Msg> for SettingsScreen {
    fn update(&mut self, msg: &Msg) -> Command<Msg> {
        Command::none()
    }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = format!("Settings\n\nVolume: {}\n\nPress Esc to go back", self.volume);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(theme.text)),
            area,
        );
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::None
    }
}

fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::OpenSettings => {
            ctx.push_screen(SettingsScreen { volume: self.volume });
            Command::none()
        }
        // Esc automatically pops the screen via convention keys
        // ...
    }
}
```

---

## 8. Testing

Use `TestHarness` to test key bindings, messages, and state transitions without a terminal.

```rust
#[cfg(test)]
mod tests {
    use kitz::prelude::*;
    use super::*;

    #[test]
    fn navigation_wraps() {
        let mut harness = TestHarness::new(App::new_with_items(vec![
            "a".into(), "b".into(), "c".into(),
        ]));

        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('j'));
        assert_eq!(harness.app().selected, 2);

        // At the end, pressing j should not go past the last item
        harness.press_key(KeyCode::Char('j'));
        assert_eq!(harness.app().selected, 2);
    }

    #[test]
    fn delete_removes_item() {
        let mut harness = TestHarness::new(App::new_with_items(vec![
            "a".into(), "b".into(), "c".into(),
        ]));

        harness.send_message(Msg::ConfirmDelete);
        assert_eq!(harness.app().items.len(), 2);
    }

    #[test]
    fn async_result_updates_state() {
        let mut harness = TestHarness::new(App::new());
        harness.send_message(Msg::LoadFile("test.txt".into()));
        assert!(harness.app().loading);

        // Simulate the Command::perform result
        harness.send_message(Msg::FileLoaded(Ok("file contents".into())));
        assert!(!harness.app().loading);
        assert_eq!(harness.app().content, "file contents");
    }
}
```

---

## 9. Custom Panels with Nested Layouts

Use `PanelLayout::nested` for complex multi-direction arrangements.

```rust
use ratatui::layout::{Constraint, Direction};

fn panels(&self) -> PanelLayout {
    PanelLayout::nested(Direction::Horizontal, vec![
        // Left sidebar
        (PanelLayout::single("sidebar"), Constraint::Percentage(25)),

        // Right side: two panels stacked vertically
        (PanelLayout::vertical(vec![
            ("editor", Constraint::Percentage(70)),
            ("terminal", Constraint::Percentage(30)),
        ]), Constraint::Percentage(75)),
    ])
}

fn panel_title(&self, id: PanelId) -> &str {
    match id {
        "sidebar" => "Files",
        "editor" => "Editor",
        "terminal" => "Terminal",
        _ => "",
    }
}

fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    let theme = self.theme();
    match id {
        "sidebar" => self.render_file_tree(frame, area, &theme),
        "editor" => self.render_editor(frame, area, &theme),
        "terminal" => self.render_terminal(frame, area, &theme),
        _ => {}
    }
}
```

---

## 10. Logging

Set up file-based logging to debug without corrupting the TUI.

```rust
use tracing::{info, debug, error};

fn main() -> kitz::prelude::Result<()> {
    let _guard = kitz::logging::init_logging("my-app");
    info!("Application starting");
    kitz::run(App::new())
}

// In your update function:
fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::ItemSelected(idx) => {
            debug!(index = idx, total = self.items.len(), "Item selected");
            self.selected = idx;
            Command::none()
        }
        Msg::FetchDone(Err(e)) => {
            error!(error = %e, "Fetch failed");
            self.error = Some(e);
            Command::none()
        }
        // ...
    }
}
```

View logs in a separate terminal:

```bash
tail -f ~/.local/share/kitz/my-app/app.log
```

---

## 11. Skip Conventions Entirely

For full control, return `PanelLayout::None` from `panels()` and handle everything in `view()` and `handle_event()`. Return `EventResult::Consumed` to suppress convention keys.

```rust
impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Quit => Command::quit(),
            Msg::Input(c) => {
                self.buffer.push(c);
                Command::none()
            }
            // ...
        }
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        // Complete custom rendering -- no panels, no footer
        let area = frame.area();
        frame.render_widget(
            Paragraph::new(self.buffer.as_str()),
            area,
        );
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => EventResult::Message(Msg::Quit),
                KeyCode::Char(c) => EventResult::Message(Msg::Input(c)),
                _ => EventResult::Consumed, // Suppress all convention keys
            }
        } else {
            EventResult::Ignored
        }
    }
}
```

---

## 12. Mode Switching (Normal/Insert)

Implement vim-style modal input by tracking the current mode and switching key handling behavior.

```rust
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Insert,
}

struct App {
    mode: Mode,
    buffer: String,
    // ...
}

#[derive(Debug, Clone)]
enum Msg {
    EnterInsert,
    ExitInsert,
    InsertChar(char),
    InsertBackspace,
    Navigate(Direction),
    // ...
}

impl Application for App {
    type Message = Msg;

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match self.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('i') => EventResult::Message(Msg::EnterInsert),
                    KeyCode::Char('j') => EventResult::Message(Msg::Navigate(Direction::Down)),
                    KeyCode::Char('k') => EventResult::Message(Msg::Navigate(Direction::Up)),
                    _ => EventResult::Ignored, // Let convention keys work in Normal mode
                },
                Mode::Insert => match key.code {
                    KeyCode::Esc => EventResult::Message(Msg::ExitInsert),
                    KeyCode::Backspace => EventResult::Message(Msg::InsertBackspace),
                    KeyCode::Char(c) => EventResult::Message(Msg::InsertChar(c)),
                    _ => EventResult::Consumed, // Suppress convention keys in Insert mode
                },
            }
        } else {
            EventResult::Ignored
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::EnterInsert => {
                self.mode = Mode::Insert;
                Command::none()
            }
            Msg::ExitInsert => {
                self.mode = Mode::Normal;
                Command::none()
            }
            Msg::InsertChar(c) => {
                self.buffer.push(c);
                Command::none()
            }
            Msg::InsertBackspace => {
                self.buffer.pop();
                Command::none()
            }
            Msg::Navigate(dir) => {
                // Handle navigation...
                Command::none()
            }
            // ...
        }
    }

    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        match self.mode {
            Mode::Normal => vec![
                KeyHint::new("i", "Insert mode"),
                KeyHint::new("j/k", "Navigate"),
            ],
            Mode::Insert => vec![
                KeyHint::new("Esc", "Normal mode"),
                KeyHint::new("Type", "Insert text"),
            ],
        }
    }
}
```

The key detail is returning `EventResult::Consumed` in Insert mode to prevent convention keys like `q` (quit) and `?` (help) from firing while the user is typing.
