# Migrating from Raw Ratatui

This guide walks through migrating an existing ratatui application to kitz. Each section shows a common raw ratatui pattern alongside its kitz equivalent, so you can convert your code incrementally.

## Migration Strategy

You do not need to rewrite everything at once. The recommended approach:

1. Wrap your existing app in the `Application` trait using the minimal path (override `view()`, skip panels).
2. Move your event loop logic into `handle_event()` and `update()`.
3. Extract side-effects into `Command` values.
4. Optionally adopt panels, overlays, and other conventions.

## Pattern 1: Terminal Initialization

### Before (raw ratatui)

```rust
use std::io;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
```

### After (kitz)

```rust
use kitz::prelude::*;

fn main() -> Result<()> {
    kitz::run(App::new())
}
```

Kitz handles terminal initialization, raw mode, the alternate screen, cursor visibility, and cleanup (including on panic via `color_eyre`).

## Pattern 2: Event Loop

### Before (raw ratatui)

```rust
fn run_app(terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') => app.next(),
                    KeyCode::Char('k') => app.prev(),
                    _ => {}
                }
            }
        }
    }
}
```

### After (kitz)

```rust
impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Next => { self.next(); Command::none() }
            Msg::Prev => { self.prev(); Command::none() }
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        ui(frame, self);
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') => return EventResult::Message(Msg::Next),
                KeyCode::Char('k') => return EventResult::Message(Msg::Prev),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}
```

The `q` to quit convention is handled automatically by the runtime. Your existing `ui()` rendering function works unchanged inside `view()`.

## Pattern 3: Layout::split

### Before (raw ratatui)

```rust
fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(frame.area());

    render_sidebar(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
}
```

### After (kitz panels)

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(30)),
        ("main", Constraint::Percentage(70)),
    ])
}

fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    match id {
        "sidebar" => render_sidebar(frame, self, area),
        "main" => render_main(frame, self, area),
        _ => {}
    }
}
```

The framework adds borders, focus indicators, and the footer automatically. Your rendering functions receive the inner area (inside the border).

## Pattern 4: Key Hints / Status Bar

### Before (raw ratatui)

```rust
fn render_status_bar(frame: &mut Frame, area: Rect) {
    let hints = Paragraph::new("j/k: Navigate | Enter: Select | q: Quit")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hints, area);
}
```

### After (kitz)

```rust
fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "sidebar" => vec![
            KeyHint::new("j/k", "Navigate"),
            KeyHint::new("Enter", "Select"),
        ],
        _ => vec![],
    }
}
```

The framework renders the footer automatically from the focused panel's key hints. Global hints (quit, tab, zoom, help) are added by the runtime.

## Pattern 5: Confirm Dialogs

### Before (raw ratatui)

```rust
if app.show_confirm {
    let popup = Paragraph::new("Delete item? (y/n)")
        .block(Block::default().borders(Borders::ALL));
    let area = centered_rect(40, 5, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

// In event handling:
if app.show_confirm {
    match key.code {
        KeyCode::Char('y') => { app.delete_item(); app.show_confirm = false; }
        KeyCode::Char('n') | KeyCode::Esc => { app.show_confirm = false; }
        _ => {}
    }
}
```

### After (kitz)

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::AskDelete => {
            ctx.push_overlay(ConfirmOverlay::new(
                "Delete item?",
                Msg::ConfirmDelete,
            ));
            Command::none()
        }
        Msg::ConfirmDelete => {
            self.delete_item();
            Command::none()
        }
    }
}
```

`ConfirmOverlay` handles rendering, key input (Enter to confirm, Esc to cancel), and cleanup. The overlay renders on top of your UI with proper centering and theming.

## Pattern 6: Background Work with thread::spawn

### Before (raw ratatui)

```rust
let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    let data = fetch_data();
    tx.send(data).unwrap();
});

// In event loop:
if let Ok(data) = rx.try_recv() {
    app.data = Some(data);
}
```

### After (kitz)

```rust
fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::StartFetch => {
            self.loading = true;
            Command::perform(|| fetch_data(), Msg::DataLoaded)
        }
        Msg::DataLoaded(data) => {
            self.loading = false;
            self.data = Some(data);
            Command::none()
        }
    }
}
```

No manual channel management. The runtime owns the background thread and routes the result back through `update()`.

## Pattern 7: Theme Colors

### Before (raw ratatui)

```rust
let border_style = if focused {
    Style::default().fg(Color::Cyan)
} else {
    Style::default().fg(Color::DarkGray)
};
let block = Block::default()
    .borders(Borders::ALL)
    .border_style(border_style);
```

### After (kitz)

Panel borders are themed automatically by the framework. For custom rendering inside panels, use the theme:

```rust
fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    let theme = self.theme();
    let style = Style::default().fg(theme.text);
    let accent = Style::default().fg(theme.accent);
    // Use style and accent in your widgets...
}
```

Or override `theme()` to use a different palette:

```rust
fn theme(&self) -> Theme {
    kitz::theme::palettes::tokyo_night()
}
```

## Pattern 8: Testing

### Before (raw ratatui)

Testing raw ratatui applications typically involves ad-hoc state manipulation or integration tests that require a terminal (or a mock backend):

```rust
#[test]
fn test_next() {
    let mut app = App::new();
    app.next();
    assert_eq!(app.selected, 1);
}
```

### After (kitz)

```rust
#[test]
fn test_navigation() {
    let mut harness = TestHarness::new(App::new());
    harness.press_key(KeyCode::Char('j'));
    assert_eq!(harness.app().selected, 1);

    harness.press_key(KeyCode::Char('k'));
    assert_eq!(harness.app().selected, 0);
}

#[test]
fn test_quit_flow() {
    let mut harness = TestHarness::new(App::new());
    harness.send_message(Msg::ConfirmQuit);
    assert!(harness.quit_requested());
}
```

`TestHarness` tests the full event-to-message-to-state pipeline without a terminal.

## Step-by-Step Migration Checklist

1. **Add kitz to Cargo.toml.** Replace your direct `ratatui`, `crossterm`, and event loop dependencies with `kitz` (which re-exports what you need through `kitz::prelude`).

2. **Create a message enum.** Identify every state transition in your event loop and create a `#[derive(Debug, Clone)]` enum with one variant per transition.

3. **Implement `Application`.** Start with `update()` to handle messages and `view()` to render (reusing your existing render function).

4. **Move event handling to `handle_event()`.** Map key events to messages instead of mutating state directly.

5. **Replace `main()`.** Remove terminal init/cleanup code. Replace with `kitz::run(App::new())`.

6. **Extract side-effects.** Replace `thread::spawn` with `Command::perform()`. Replace direct channel management with messages.

7. **Adopt panels (optional).** If your app has a multi-pane layout, implement `panels()`, `panel_view()`, `panel_title()`, `panel_key_hints()`, and `panel_handle_key()`. Remove your manual `Layout::split` and border rendering.

8. **Replace custom dialogs with overlays (optional).** Use `ConfirmOverlay`, `HelpOverlay`, or custom overlays instead of manual popup rendering.

9. **Add theme support (optional).** Replace hardcoded colors with `self.theme()` values.

10. **Write tests.** Create `TestHarness`-based tests to cover your key bindings and update logic.

Each step can be done independently and tested before moving to the next. The application should compile and run correctly after each step.
