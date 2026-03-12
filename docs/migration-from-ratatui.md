# Migrating from raw ratatui to rataframe

This guide walks through replacing hand-rolled ratatui boilerplate with rataframe equivalents. Each section shows a **before** (raw ratatui) and **after** (rataframe) snippet.

---

## What you get for free

When you adopt rataframe, you can delete code that handles:

- Terminal init/restore with panic-safe cleanup
- The event loop (poll, read, dispatch, render)
- Convention keys (`q`, `?`, `:`, `Tab`, `Shift+Tab`, `Esc`, `z`, `Ctrl+C`)
- Panel borders, focus indicators, and layout splitting
- Auto-generated footer from `panel_key_hints()`
- Auto-generated help overlay from all panels' hints
- Command palette with fuzzy search over all actions
- Confirm dialogs and overlay stacking
- Theme system (Nord, Tokyo Night, Catppuccin, Dracula)
- Toast notifications
- Per-panel error boundaries (`catch_unwind`)
- TUI-safe file logging via `tracing`
- `TestHarness` for headless unit testing

---

## Choose your migration path

### Path 1 — Minimal

Implement `update()` + `view()`. Replace your manual terminal setup and event loop with `rataframe::run(app)`. You keep full rendering control.

### Path 2 — Convention (recommended for most apps)

Implement `panels()` returning a `PanelLayout`, plus `panel_view()` and `panel_key_hints()` per panel. Replace manual `Layout::split` calls. You get auto-footer, help, command palette, focus cycling, and zoom for free.

### Path 3 — Custom

Override `view()` entirely — you render with `Frame` directly, but still get the runtime, commands, overlays, themes, toasts, and subscriptions.

---

## Migration patterns

### 1. Terminal init/restore → `rataframe::run()`

**Before** — you manage raw mode, alternate screen, panic hooks:

```rust
fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(e) = res {
        eprintln!("{e:?}");
    }
    Ok(())
}
```

**After** — one line:

```rust
fn main() -> color_eyre::Result<()> {
    rataframe::run(MyApp::new())
}
```

Panic hooks, raw mode, alternate screen, mouse capture — all handled internally.

### 2. Event loop → `Application` trait

**Before** — you write the poll/read/dispatch loop yourself:

```rust
fn run_app(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') => app.next(),
                    KeyCode::Char('k') => app.previous(),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
```

**After** — the runtime owns the loop; you declare behavior:

```rust
impl Application for MyApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Next => self.next(),
            Msg::Previous => self.previous(),
        }
        Command::none()
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') => return EventResult::Message(Msg::Next),
                KeyCode::Char('k') => return EventResult::Message(Msg::Previous),
                _ => {}
            }
        }
        EventResult::Ignored // falls through to convention keys (q, Esc, etc.)
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        // your existing ui() function body goes here
    }
}
```

`q` to quit, `Ctrl+C` to hard-quit, `Esc` to quit — all free. No more match arms for those.

### 3. Manual `Layout::split` → `PanelLayout`

**Before** — imperative layout splitting:

```rust
fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(f.area());

    let sidebar_block = Block::default()
        .title(" Sidebar ")
        .borders(Borders::ALL)
        .border_style(if app.focus == Focus::Sidebar {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    f.render_widget(sidebar_block, chunks[0]);
    render_sidebar(f, app, sidebar_block.inner(chunks[0]));

    let main_block = Block::default()
        .title(" Main ")
        .borders(Borders::ALL)
        .border_style(if app.focus == Focus::Main {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    f.render_widget(main_block, chunks[1]);
    render_main(f, app, main_block.inner(chunks[1]));
}
```

**After** — declarative layout; borders, focus styling, and splitting handled by the framework:

```rust
impl Application for MyApp {
    type Message = Msg;

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("main", Constraint::Percentage(70)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Sidebar",
            "main" => "Main",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
        match id {
            "sidebar" => render_sidebar(frame, self, area),
            "main" => render_main(frame, self, area),
            _ => {}
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        // ...
        Command::none()
    }
}
```

`Tab`/`Shift+Tab` to cycle focus, `z` to zoom, mouse click-to-focus — all free.

For complex nested layouts:

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::nested(Direction::Horizontal, vec![
        (PanelLayout::single("sidebar"), Constraint::Percentage(30)),
        (PanelLayout::vertical(vec![
            ("editor", Constraint::Percentage(60)),
            ("output", Constraint::Percentage(40)),
        ]), Constraint::Percentage(70)),
    ])
}
```

### 4. Manual key hint rendering → `panel_key_hints()` + auto-footer

**Before** — you render a footer bar yourself:

```rust
fn render_footer(f: &mut Frame, area: Rect, focus: Focus) {
    let hints = match focus {
        Focus::Sidebar => "j/k: Navigate  Enter: Select  /: Search",
        Focus::Main => "e: Edit  d: Delete  n: New",
    };
    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}
```

**After** — declare hints per panel; footer and help overlay are auto-generated:

```rust
fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "sidebar" => vec![
            KeyHint::new("j/k", "Navigate"),
            KeyHint::new("Enter", "Select"),
            KeyHint::new("/", "Search"),
        ],
        "main" => vec![
            KeyHint::new("e", "Edit"),
            KeyHint::new("d", "Delete"),
            KeyHint::new("n", "New"),
        ],
        _ => vec![],
    }
}
```

The footer shows the focused panel's hints automatically. Press `?` to see all hints grouped by panel. Press `:` to fuzzy-search all actions.

### 5. Manual confirm dialogs → `ConfirmOverlay`

**Before** — you track dialog state, render it yourself, handle yes/no:

```rust
struct App {
    show_confirm: bool,
    confirm_selection: bool,
    pending_delete: Option<usize>,
    // ...
}

// In event handling:
if self.show_confirm {
    match key.code {
        KeyCode::Tab => self.confirm_selection = !self.confirm_selection,
        KeyCode::Enter => {
            if self.confirm_selection {
                if let Some(idx) = self.pending_delete.take() {
                    self.items.remove(idx);
                }
            }
            self.show_confirm = false;
        }
        KeyCode::Esc => self.show_confirm = false,
        _ => {}
    }
}

// In rendering:
if self.show_confirm {
    render_confirm_dialog(f, "Delete this item?");
}
```

**After** — push a `ConfirmOverlay`; the framework handles rendering and key dispatch:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::RequestDelete(idx) => {
            self.pending_delete = Some(idx);
            ctx.push_overlay(ConfirmOverlay::new(
                "Confirm",
                "Delete this item?",
                Msg::ConfirmDelete,
            ));
            Command::none()
        }
        Msg::ConfirmDelete => {
            if let Some(idx) = self.pending_delete.take() {
                self.items.remove(idx);
            }
            Command::none()
        }
        // ...
    }
}
```

No dialog state, no rendering code, no key handling for the dialog itself.

### 6. Manual `thread::spawn` / `tokio::spawn` → `Command::perform`

**Before** — you spawn threads and send results back manually:

```rust
let (tx, rx) = mpsc::channel();
thread::spawn(move || {
    let result = reqwest::blocking::get("https://api.example.com/data");
    tx.send(result).unwrap();
});

// Later, in event loop:
if let Ok(result) = rx.try_recv() {
    match result {
        Ok(resp) => app.data = resp.text().unwrap_or_default(),
        Err(e) => app.error = Some(e.to_string()),
    }
}
```

**After** — return a `Command::perform`; the runtime manages the thread and dispatches the result:

```rust
fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::FetchData => {
            Command::perform(
                || reqwest::blocking::get("https://api.example.com/data")
                    .and_then(|r| r.text()),
                |result| match result {
                    Ok(body) => Msg::DataLoaded(body),
                    Err(e) => Msg::FetchFailed(e.to_string()),
                },
            )
        }
        Msg::DataLoaded(body) => {
            self.data = body;
            Command::none()
        }
        Msg::FetchFailed(err) => {
            self.error = Some(err);
            Command::none()
        }
        // ...
    }
}
```

No channel management, no manual thread tracking, no `try_recv` in your loop.

### 7. Theme — no more hardcoded colors

**Before** — colors scattered across your rendering functions:

```rust
let border = Style::default().fg(Color::Cyan);
let text = Style::default().fg(Color::White);
let muted = Style::default().fg(Color::DarkGray);
```

**After** — use semantic theme colors:

```rust
fn theme(&self) -> Theme {
    Theme::default() // Nord, or: palettes::tokyo_night(), etc.
}

fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    let theme = self.theme(); // or receive it from ViewContext
    let style = Style::default().fg(theme.text);
    let muted = Style::default().fg(theme.text_muted);
    // ...
}
```

Built-in palettes: `nord()`, `tokyo_night()`, `catppuccin()`, `dracula()`. Switch at runtime with `theme.next()`.

### 8. Testing — no terminal needed

**Before** — testing a TUI app usually means no tests, or painful integration tests with a pseudo-terminal.

**After** — `TestHarness` simulates keys and messages without any terminal:

```rust
#[test]
fn test_navigation() {
    let mut h = TestHarness::new(MyApp::new());
    h.press_key(KeyCode::Char('j'));
    assert_eq!(h.app().selected, 1);

    h.press_key(KeyCode::Char('k'));
    assert_eq!(h.app().selected, 0);
}

#[test]
fn test_delete_flow() {
    let mut h = TestHarness::new(MyApp::with_items(vec!["a", "b", "c"]));
    h.send_message(Msg::ConfirmDelete);
    assert_eq!(h.app().items.len(), 2);
}

#[test]
fn test_panel_keys() {
    let mut h = TestHarness::new(MyApp::new());
    h.press_panel_key("sidebar", KeyCode::Char('j'));
    assert_eq!(h.app().sidebar_selected, 1);
}

#[test]
fn test_quit() {
    let mut h = TestHarness::new(MyApp::new());
    h.send_message(Msg::QuitNow);
    assert!(h.quit_requested());
}
```

---

## What changes conceptually

| Raw ratatui | rataframe |
|---|---|
| You call `enable_raw_mode()` / `disable_raw_mode()` | `rataframe::run()` handles terminal lifecycle |
| You write the event loop | The runtime owns the loop; you declare behavior |
| State mutations happen inline in key handlers | `handle_event` returns `EventResult::Message`, `update` handles state changes |
| Side effects (HTTP, file I/O) happen directly | Return `Command::perform` — the runtime executes it and dispatches the result |
| Layout is imperative (`Layout::split`) | Layout is declarative (`PanelLayout::horizontal/vertical/nested`) |
| You render borders and focus indicators manually | Framework renders themed borders and tracks focus |
| You build a footer widget | `panel_key_hints()` → auto-footer |
| You build help screens | `?` auto-generates a help overlay from all panels' hints |
| You build a command palette | `:` opens a fuzzy-search palette auto-populated from hints |
| You handle `q`, `Esc`, `Tab` yourself | Convention keys work out of the box (overridable via `EventResult::Consumed`) |
| Confirm dialogs are manual state + rendering | `ConfirmOverlay` — push it, get a message back |
| Colors are hardcoded | Semantic `Theme` with 4 built-in palettes |
| Testing requires a terminal | `TestHarness` — simulate keys, assert state |

---

## Step-by-step checklist

1. **Add the dependency**: `cargo add rataframe`
2. **Define your `Message` enum**: every action your app can take becomes a variant
3. **Implement `Application`**: move rendering into `view()` or `panel_view()`, move state changes into `update()`
4. **Delete terminal setup**: remove `enable_raw_mode`, `EnterAlternateScreen`, panic hooks, `disable_raw_mode`, `LeaveAlternateScreen`
5. **Delete your event loop**: replace with `rataframe::run(app)` in `main()`
6. **Delete convention key handling**: `q`, `Esc`, `Ctrl+C` are free; remove your match arms for them
7. **Replace `Layout::split`** with `PanelLayout` (if using convention path)
8. **Add `panel_key_hints()`**: the footer and help overlay auto-populate
9. **Replace `thread::spawn`** with `Command::perform`
10. **Replace inline confirm dialogs** with `ConfirmOverlay`
11. **Add tests** using `TestHarness`

---

## Minimal complete example

```rust
use rataframe::prelude::*;

struct Counter {
    count: i32,
}

#[derive(Debug)]
enum Msg {
    Increment,
    Decrement,
}

impl Application for Counter {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
        }
        Command::none()
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

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let text = format!("Count: {} (j/k to change, q to quit)", self.count);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(text),
            frame.area(),
        );
    }
}

fn main() -> color_eyre::Result<()> {
    rataframe::run(Counter { count: 0 })
}
```

That's it. Terminal safety, panic handling, convention keys, and the event loop — all handled.
