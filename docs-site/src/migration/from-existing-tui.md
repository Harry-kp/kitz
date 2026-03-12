# Migrating from an Existing TUI App

If you have an existing terminal application -- whether built with raw ratatui, tui-rs, cursive, or hand-rolled crossterm -- this guide shows how to migrate it to kitz incrementally. The key principle is to start with the minimal path, get your app running under the kitz runtime, and then progressively adopt framework features.

## Phase 1: Wrap in the Application Trait

The first step is the smallest possible change: implement `Application` with just `update()` and `view()`, keeping your existing rendering code intact.

### Step 1: Add kitz as a dependency

```toml
[dependencies]
kitz = "0.2"
```

You can keep your existing `ratatui` and `crossterm` dependencies, or use kitz's re-exports through `kitz::prelude::*`.

### Step 2: Create a message enum

Start with a minimal message type. You can expand it later:

```rust
#[derive(Debug, Clone)]
enum Msg {
    Noop,
}
```

### Step 3: Implement Application

Move your application struct into the `Application` trait. Your existing render function goes into `view()`:

```rust
use kitz::prelude::*;

struct MyApp {
    // Your existing fields...
}

impl Application for MyApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Noop => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        // Call your existing render function
        self.render(frame, frame.area());
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        // For now, handle everything as Consumed to prevent
        // convention keys from interfering
        if let Event::Key(key) = event {
            // Keep your existing key handling for now
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }
}
```

### Step 4: Replace your main function

```rust
fn main() -> kitz::prelude::Result<()> {
    kitz::run(MyApp::new())
}
```

Remove all terminal initialization, the event loop, and cleanup code. Kitz handles all of that.

At this point, your app should compile and run under the kitz runtime. The UI looks the same as before. You have gained panic recovery (via `color_eyre`), automatic terminal cleanup, and the ability to use kitz features incrementally.

## Phase 2: Extract Messages

Now convert your direct state mutations into a message-based architecture.

### Before

```rust
// Inside your event loop
match key.code {
    KeyCode::Char('j') => self.selected += 1,
    KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
    KeyCode::Enter => self.activate_item(),
    KeyCode::Char('d') => self.delete_selected(),
    _ => {}
}
```

### After

```rust
#[derive(Debug, Clone)]
enum Msg {
    SelectNext,
    SelectPrev,
    ActivateItem,
    DeleteSelected,
}

fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('j') => EventResult::Message(Msg::SelectNext),
            KeyCode::Char('k') => EventResult::Message(Msg::SelectPrev),
            KeyCode::Enter => EventResult::Message(Msg::ActivateItem),
            KeyCode::Char('d') => EventResult::Message(Msg::DeleteSelected),
            _ => EventResult::Ignored,
        }
    } else {
        EventResult::Ignored
    }
}

fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::SelectNext => self.selected += 1,
        Msg::SelectPrev => self.selected = self.selected.saturating_sub(1),
        Msg::ActivateItem => self.activate_item(),
        Msg::DeleteSelected => self.delete_selected(),
    }
    Command::none()
}
```

Once you return `EventResult::Ignored` for unhandled keys instead of `EventResult::Consumed`, the kitz convention keys (`q` to quit, `Esc` to go back) start working automatically.

## Phase 3: Extract Side-Effects

If your app spawns threads, makes network calls, or performs file I/O, convert those to `Command::perform()`:

### Before

```rust
// Somewhere in your event handler
let data = std::fs::read_to_string("config.toml").unwrap();
self.config = parse_config(&data);
```

### After

```rust
Msg::LoadConfig => {
    Command::perform(
        || std::fs::read_to_string("config.toml").map_err(|e| e.to_string()),
        Msg::ConfigLoaded,
    )
}
Msg::ConfigLoaded(result) => {
    match result {
        Ok(data) => self.config = parse_config(&data),
        Err(e) => self.error = Some(e),
    }
    Command::none()
}
```

## Phase 4: Adopt Panels (Optional)

If your app has a multi-pane layout with `Layout::split`, consider adopting the panel system. This gives you borders, focus management, the auto-footer, and the help overlay for free.

### Identify your panels

Look at your `view()` function. If it splits the screen into named regions (sidebar, main, preview, status), each region is a candidate panel.

### Implement the panel methods

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(25)),
        ("main", Constraint::Percentage(75)),
    ])
}

fn panel_title(&self, id: PanelId) -> &str {
    match id {
        "sidebar" => "Files",
        "main" => "Content",
        _ => "",
    }
}

fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    match id {
        "sidebar" => self.render_sidebar(frame, area, focused),
        "main" => self.render_main(frame, area, focused),
        _ => {}
    }
}

fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "sidebar" => vec![
            KeyHint::new("j/k", "Navigate"),
            KeyHint::new("Enter", "Open"),
        ],
        "main" => vec![
            KeyHint::new("j/k", "Scroll"),
        ],
        _ => vec![],
    }
}

fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
    match id {
        "sidebar" => match key.code {
            KeyCode::Char('j') => EventResult::Message(Msg::SidebarNext),
            KeyCode::Char('k') => EventResult::Message(Msg::SidebarPrev),
            KeyCode::Enter => EventResult::Message(Msg::SidebarSelect),
            _ => EventResult::Ignored,
        },
        "main" => match key.code {
            KeyCode::Char('j') => EventResult::Message(Msg::MainScrollDown),
            KeyCode::Char('k') => EventResult::Message(Msg::MainScrollUp),
            _ => EventResult::Ignored,
        },
        _ => EventResult::Ignored,
    }
}
```

Remove the `view()` override -- once `panels()` returns a layout, the runtime handles rendering. You can also remove your custom border drawing, focus indicator logic, and status bar rendering.

## Phase 5: Adopt Other Features (Optional)

With the core migration complete, you can adopt additional kitz features as needed:

- **Overlays** -- Replace custom popup dialogs with `ConfirmOverlay` or custom overlays via `ctx.push_overlay()`.
- **Screens** -- If your app has multiple full-screen views, use the navigation stack via `ctx.push_screen()` and `ctx.pop_screen()`.
- **Toasts** -- Replace status messages with `ctx.toast("message", ToastLevel::Info)`.
- **Subscriptions** -- Replace manual timer threads with `Subscription::every()`.
- **Themes** -- Replace hardcoded colors with `self.theme()` values.
- **Testing** -- Add `TestHarness` tests for your key bindings and update logic.
- **Logging** -- Replace `println!` debugging with `kitz::logging::init_logging()` and `tracing` macros.

## Migration Checklist

- [ ] Add `kitz` to `Cargo.toml`
- [ ] Create a message enum
- [ ] Implement `Application` with `update()` and `view()`
- [ ] Move key handling to `handle_event()` returning messages
- [ ] Replace `main()` with `kitz::run()`
- [ ] Verify the app compiles and runs
- [ ] Convert direct state mutations to messages in `update()`
- [ ] Replace `thread::spawn` / blocking I/O with `Command::perform()`
- [ ] (Optional) Adopt panels for multi-pane layouts
- [ ] (Optional) Replace popups with overlays
- [ ] (Optional) Add subscriptions for periodic tasks
- [ ] (Optional) Switch to semantic theme colors
- [ ] (Optional) Add `TestHarness` tests
- [ ] (Optional) Set up file-based logging
- [ ] Remove unused direct dependencies (`crossterm`, terminal init code)

Each step is independently testable. You can stop at any point and have a working application that benefits from whatever level of kitz integration you have adopted.
