# Kitz Cookbook

Practical recipes for common patterns. Each snippet is self-contained — add
`use kitz::prelude::*;` and you're ready to go.

---

## 1. Custom Key Bindings

Override a convention key (e.g. `?` normally opens Help) by returning
`EventResult::Consumed` from `handle_event`. The runtime stops processing the
event and never reaches the built-in handler.

```rust
fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        match key.code {
            // '?' is a convention key (opens Help) — steal it here
            KeyCode::Char('?') => {
                return EventResult::Message(Msg::ShowCustomHelp);
            }
            // Swallow F1 entirely so nothing else sees it
            KeyCode::F(1) => return EventResult::Consumed,
            _ => {}
        }
    }
    EventResult::Ignored
}
```

`EventResult::Ignored` lets the event fall through to the focused panel and
then to convention keys. `Consumed` eats it. `Message(msg)` dispatches to
`update`.

---

## 2. Background Tasks

`Command::perform` spawns a thread. The first closure does the work; the second
maps the result into a message.

```rust
#[derive(Debug)]
enum Msg {
    FetchStart,
    FetchDone(String),
    FetchFailed(String),
}

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::FetchStart => {
            self.loading = true;
            Command::perform(
                || {
                    let resp = reqwest::blocking::get("https://api.example.com/data")
                        .and_then(|r| r.text());
                    resp
                },
                |result| match result {
                    Ok(body) => Msg::FetchDone(body),
                    Err(e) => Msg::FetchFailed(e.to_string()),
                },
            )
        }
        Msg::FetchDone(body) => {
            self.loading = false;
            self.data = body;
            Command::none()
        }
        Msg::FetchFailed(err) => {
            self.loading = false;
            self.error = Some(err);
            Command::none()
        }
    }
}
```

The `perform` closure runs off the main thread, so the UI stays responsive.
The mapper closure runs when the thread finishes and feeds back into `update`.

---

## 3. Periodic Updates

`Subscription::every` emits a message at a fixed interval. Return it from
`subscriptions()` and the runtime manages the background thread for you.

```rust
use std::time::Duration;

fn subscriptions(&self) -> Vec<Subscription<Msg>> {
    if self.polling_enabled {
        vec![Subscription::every("refresh", Duration::from_secs(5), || Msg::Refresh)]
    } else {
        Subscription::none()
    }
}

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Refresh => {
            self.last_tick = std::time::Instant::now();
            Command::perform(|| fetch_latest_metrics(), Msg::MetricsLoaded)
        }
        // ...
    }
}
```

When `polling_enabled` flips to `false`, the runtime automatically stops the
background thread — no manual cleanup needed.

---

## 4. Modal Dialogs

Push a `ConfirmOverlay` from `update` via the context. The overlay captures all
input until the user confirms or cancels.

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::RequestDelete => {
            ctx.push_overlay(ConfirmOverlay::new(
                "Delete Item",
                "Are you sure? This cannot be undone.",
                Msg::ConfirmDelete,
            ));
            Command::none()
        }
        Msg::ConfirmDelete => {
            self.items.remove(self.selected);
            ctx.toast("Item deleted", ToastLevel::Success);
            Command::none()
        }
        // ...
    }
}
```

Build custom overlays by implementing the `Overlay` trait:

```rust
use kitz::overlay::{Overlay, OverlayResult};
use kitz::theme::Theme;

struct InputOverlay {
    input: String,
}

impl Overlay<Msg> for InputOverlay {
    fn title(&self) -> &str { "Enter Name" }

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog = centered_rect(40, 20, area);
        frame.render_widget(Clear, dialog);
        let p = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(theme.text));
        frame.render_widget(p, dialog);
    }

    fn handle_event(&mut self, event: &Event) -> OverlayResult<Msg> {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Enter => {
                    OverlayResult::CloseWithMessage(Msg::NameEntered(self.input.clone()))
                }
                KeyCode::Esc => OverlayResult::Close,
                KeyCode::Char(c) => { self.input.push(*c); OverlayResult::Consumed }
                KeyCode::Backspace => { self.input.pop(); OverlayResult::Consumed }
                _ => OverlayResult::Consumed,
            }
        } else {
            OverlayResult::Consumed
        }
    }
}
```

---

## 5. Toast Notifications

Call `ctx.toast()` anywhere inside `update` for non-blocking user feedback.

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::SaveSuccess => {
            ctx.toast("File saved", ToastLevel::Success);
            Command::none()
        }
        Msg::NetworkError(e) => {
            ctx.toast(format!("Connection failed: {e}"), ToastLevel::Error);
            Command::none()
        }
        Msg::Warning(w) => {
            ctx.toast(w, ToastLevel::Warning);
            Command::none()
        }
        // ...
    }
}
```

Toasts auto-dismiss after 3 seconds. They stack vertically in the top-right
corner and are colored by the current theme.

---

## 6. Theme Switching

Store a `Theme` in your app and return it from `theme()`. Cycle with
`Theme::next()`.

```rust
struct MyApp {
    theme: Theme,
    // ...
}

impl Application for MyApp {
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
            if key.code == KeyCode::F(2) {
                return EventResult::Message(Msg::CycleTheme);
            }
        }
        EventResult::Ignored
    }
}
```

Built-in palettes include Nord, Dracula, Solarized, and more. All framework
widgets — panels, overlays, toasts, footer — respect the returned theme
automatically.

---

## 7. Multi-Screen Navigation

Implement the `Screen` trait for each page. Push and pop screens through the
context.

```rust
struct SettingsScreen {
    items: Vec<String>,
    selected: usize,
}

impl Screen<Msg> for SettingsScreen {
    fn id(&self) -> &str { "settings" }

    fn panels(&self) -> PanelLayout {
        PanelLayout::single("settings-main")
    }

    fn panel_title(&self, _id: PanelId) -> &str { "Settings" }

    fn panel_view(&self, _id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        let text: Vec<Line> = self.items.iter().enumerate().map(|(i, item)| {
            if i == self.selected {
                Line::from(format!("> {item}"))
            } else {
                Line::from(format!("  {item}"))
            }
        }).collect();
        frame.render_widget(Paragraph::new(text), area);
    }

    fn panel_handle_key(&mut self, _id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        match key.code {
            KeyCode::Char('j') => { self.selected += 1; EventResult::Consumed }
            KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    fn on_enter(&mut self) { self.selected = 0; }
}
```

Push from `update`:

```rust
Msg::OpenSettings => {
    ctx.push_screen(SettingsScreen {
        items: vec!["Audio".into(), "Video".into(), "Controls".into()],
        selected: 0,
    });
    Command::none()
}
```

Press Esc to pop back (built-in convention), or call `ctx.pop_screen()`
explicitly.

---

## 8. Testing Your App

`TestHarness` drives your app without a real terminal. Simulate keys, send
messages directly, and assert on state.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kitz::prelude::*;

    #[test]
    fn increment_on_j() {
        let mut h = TestHarness::new(MyApp::new());
        h.press_key(KeyCode::Char('j'));
        h.press_key(KeyCode::Char('j'));
        assert_eq!(h.app().selected, 2);
    }

    #[test]
    fn delete_confirms() {
        let mut h = TestHarness::new(MyApp::new());
        // simulate the result of the confirm dialog
        h.send_message(Msg::ConfirmDelete);
        assert!(h.app().items.is_empty());
    }

    #[test]
    fn quit_on_q() {
        let mut h = TestHarness::new(MyApp::new());
        h.press_key(KeyCode::Char('q'));
        assert!(h.quit_requested());
    }

    #[test]
    fn panel_key_targets_sidebar() {
        let mut h = TestHarness::new(MyApp::new());
        h.press_panel_key("sidebar", KeyCode::Char('j'));
        assert_eq!(h.app().sidebar_selected, 1);
    }
}
```

Background tasks (`Command::perform`) are skipped in the harness. Use
`send_message` to inject the message that the task *would* produce.

---

## 9. Custom Panels

Use `PanelLayout::nested` for layouts that mix horizontal and vertical splits.

```rust
use ratatui::layout::{Constraint, Direction};

fn panels(&self) -> PanelLayout {
    PanelLayout::nested(Direction::Horizontal, vec![
        // Left sidebar, 25% width
        (PanelLayout::single("sidebar"), Constraint::Percentage(25)),
        // Right side: two panels stacked vertically
        (PanelLayout::vertical(vec![
            ("editor", Constraint::Min(10)),
            ("output", Constraint::Length(8)),
        ]), Constraint::Percentage(75)),
    ])
}

fn panel_title(&self, id: PanelId) -> &str {
    match id {
        "sidebar" => "Files",
        "editor"  => "Editor",
        "output"  => "Output",
        _ => "",
    }
}

fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "sidebar" => vec![
            KeyHint::new("j/k", "navigate"),
            KeyHint::new("Enter", "open"),
        ],
        "editor" => vec![
            KeyHint::new("i", "insert mode"),
            KeyHint::new("Ctrl+s", "save"),
        ],
        _ => vec![],
    }
}
```

The framework renders borders, tracks focus with Tab/Shift-Tab, supports
zooming the focused panel with `z`, and auto-generates the footer from your
`panel_key_hints`.

---

## 10. Logging

Kitz apps own stdout, so `println!` corrupts the display. Use
`kitz::logging::init_logging` to write to a file via `tracing`.

```rust
use tracing::info;

fn main() -> color_eyre::Result<()> {
    // Hold the guard — dropping it flushes pending writes
    let _log_guard = kitz::logging::init_logging("my-app");

    info!("application starting");
    kitz::run(MyApp::new())?;
    Ok(())
}
```

Then in your app code:

```rust
fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    tracing::debug!(?msg, "handling message");
    // ...
}
```

Logs go to `~/.local/share/kitz/my-app/app.log` (daily rotation). Tail
them in a separate terminal with `tail -f`.

---

## 11. Skip Conventions Entirely

Override `view()` and return `PanelLayout::None` (the default). The framework
gets out of the way — no panels, no auto-footer, no convention keys.

```rust
impl Application for MyApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        // ...
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        frame.render_widget(
            Paragraph::new("Your fully custom layout here"),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(" q: quit "),
            chunks[1],
        );
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('q') {
                return EventResult::Message(Msg::Quit);
            }
        }
        EventResult::Ignored
    }
}
```

You still get overlays, toasts, screens, commands, and subscriptions — you're
only opting out of the panel layout and built-in keybindings.

---

## 12. Mode Switching

Model modes as an enum in your app state. Switch between them in `update` and
branch on mode in `panel_handle_key`.

```rust
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Insert,
}

struct Editor {
    mode: Mode,
    content: String,
    cursor: usize,
}

impl Application for Editor {
    type Message = Msg;

    fn panel_handle_key(&mut self, _id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Char('i') => {
                    self.mode = Mode::Insert;
                    EventResult::Consumed
                }
                KeyCode::Char('j') => EventResult::Message(Msg::CursorDown),
                KeyCode::Char('k') => EventResult::Message(Msg::CursorUp),
                _ => EventResult::Ignored,
            },
            Mode::Insert => match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    EventResult::Consumed
                }
                KeyCode::Char(c) => EventResult::Message(Msg::InsertChar(c)),
                KeyCode::Backspace => EventResult::Message(Msg::DeleteChar),
                _ => EventResult::Ignored,
            },
        }
    }

    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        match self.mode {
            Mode::Normal => vec![
                KeyHint::new("i", "insert"),
                KeyHint::new("j/k", "move"),
            ],
            Mode::Insert => vec![
                KeyHint::new("Esc", "normal mode"),
            ],
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::InsertChar(c) => { self.content.insert(self.cursor, c); self.cursor += 1; }
            Msg::DeleteChar => { if self.cursor > 0 { self.cursor -= 1; self.content.remove(self.cursor); } }
            Msg::CursorDown => { self.cursor = (self.cursor + 1).min(self.content.len()); }
            Msg::CursorUp => { self.cursor = self.cursor.saturating_sub(1); }
            // ...
        }
        Command::none()
    }

    // ...
}
```

The footer updates live because `panel_key_hints` is called every frame —
users always see the hints for the current mode.
