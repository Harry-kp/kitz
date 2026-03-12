# Application Trait

The `Application` trait is the single entry point for every kitz app. It defines your message type, your update logic, and how your UI is rendered. Only one method is required --- `update()`. Everything else has a sensible default, so you can start minimal and opt in to more framework features as your application grows.

## Complete Trait Definition

```rust
pub trait Application: Sized + 'static {
    type Message: Debug + Send + 'static;

    // Required
    fn update(&mut self, msg: Self::Message, ctx: &mut Context<Self::Message>) -> Command<Self::Message>;

    // Rendering (custom path)
    fn view(&self, frame: &mut Frame, ctx: &ViewContext) { /* default placeholder */ }

    // Rendering (convention path: panels)
    fn panels(&self) -> PanelLayout { PanelLayout::None }
    fn panel_title(&self, id: PanelId) -> &str { "" }
    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {}
    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> { vec![] }
    fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Self::Message> { EventResult::Ignored }
    fn panel_on_focus(&mut self, id: PanelId) {}
    fn panel_on_blur(&mut self, id: PanelId) {}

    // Event handling
    fn handle_event(&self, event: &Event, ctx: &EventContext) -> EventResult<Self::Message> { EventResult::Ignored }

    // Lifecycle
    fn init(&self) -> Command<Self::Message> { Command::none() }

    // Metadata
    fn title(&self) -> &str { "kitz app" }
    fn tick_rate(&self) -> Duration { Duration::from_millis(250) }
    fn theme(&self) -> Theme { Theme::default() }
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> { Subscription::none() }
}
```

## Associated Type

### `type Message`

Your application's message enum. Every state change in the app originates as a `Message` value. The type must implement `Debug` (for logging), `Send` (messages may arrive from background threads), and `'static` (messages are owned values, not references).

```rust
#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
    Reset,
    FetchComplete(Result<String, String>),
}
```

Define one variant per meaningful event in your application. Messages should describe **what happened**, not what to do about it. Prefer `UserClickedSave` over `WriteFileToDisk`.

## Required Method

### `update(&mut self, msg: M, ctx: &mut Context<M>) -> Command<M>`

The heart of TEA. Called every time a message is dispatched. Mutate `self` to reflect the new state, use `ctx` for framework operations (overlays, focus, toasts), and return a `Command` for side effects.

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Increment => {
            self.count += 1;
            Command::none()
        }
        Msg::SaveRequested => {
            let data = self.data.clone();
            Command::perform(move || save(data), Msg::SaveComplete)
        }
        Msg::SaveComplete(Ok(_)) => {
            ctx.toast("Saved", ToastLevel::Info);
            Command::none()
        }
        Msg::SaveComplete(Err(e)) => {
            ctx.toast(format!("Error: {}", e), ToastLevel::Error);
            Command::none()
        }
    }
}
```

The runtime guarantees that `update()` is called synchronously and sequentially. You will never have two concurrent calls to `update()`. This means `&mut self` is always exclusive and you never need interior mutability for your model.

## Rendering Methods

Kitz offers two rendering paths. Choose one; do not mix them.

### Custom Path: `view()`

#### `view(&self, frame: &mut Frame, ctx: &ViewContext)`

Full control over rendering. You receive the entire terminal frame and draw whatever you like using ratatui widgets. The `ViewContext` provides read-only access to panel focus state (if relevant).

```rust
fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
    let area = frame.area();
    let block = Block::bordered().title(" My App ");
    let paragraph = Paragraph::new(format!("Count: {}", self.count)).block(block);
    frame.render_widget(paragraph, area);
}
```

Use the custom path when your layout does not fit a panel-based model --- single-screen apps, full-screen editors, splash screens, or anything with a highly custom layout.

### Convention Path: Panels

The convention path gives you automatic borders, focus indicators, a footer with key hints, a help overlay, a command palette, Tab/Shift+Tab navigation, and zoom --- all without writing layout code.

#### `panels(&self) -> PanelLayout`

Return the panel layout. If this returns anything other than `PanelLayout::None`, the runtime uses the convention path and ignores `view()`.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(30)),
        ("content", Constraint::Percentage(70)),
    ])
}
```

#### `panel_title(&self, id: PanelId) -> &str`

The title displayed in the panel's border. Called for each panel every frame.

```rust
fn panel_title(&self, id: PanelId) -> &str {
    match id {
        "sidebar" => "Files",
        "content" => "Editor",
        _ => "",
    }
}
```

#### `panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool)`

Render a single panel's content within its allocated area. The `area` is the inner rectangle (inside the border). The `focused` flag indicates whether this panel currently has focus, which you can use to adjust styling.

```rust
fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
    match id {
        "sidebar" => self.render_file_list(frame, area, focused),
        "content" => self.render_editor(frame, area, focused),
        _ => {}
    }
}
```

#### `panel_key_hints(&self, id: PanelId) -> Vec<KeyHint>`

Key binding hints for a panel. These are displayed in the footer when the panel is focused and included in the `?` help overlay.

```rust
fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "sidebar" => vec![
            KeyHint::new("j/k", "Navigate"),
            KeyHint::new("Enter", "Open"),
            KeyHint::new("d", "Delete"),
        ],
        _ => vec![],
    }
}
```

#### `panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<M>`

Handle a key event for the focused panel. Called only when `handle_event()` returned `Ignored` and a panel layout is active. Return `EventResult::Message(msg)` to dispatch a message, `EventResult::Consumed` to swallow the event, or `EventResult::Ignored` to let it fall through to convention keys.

```rust
fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
    if id == "sidebar" {
        match key.code {
            KeyCode::Char('j') => return EventResult::Message(Msg::SelectNext),
            KeyCode::Char('k') => return EventResult::Message(Msg::SelectPrev),
            KeyCode::Enter => return EventResult::Message(Msg::OpenSelected),
            _ => {}
        }
    }
    EventResult::Ignored
}
```

#### `panel_on_focus(&mut self, id: PanelId)`

Called when a panel gains focus. Use this for setup work --- loading data for the panel, resetting scroll position, or starting an animation.

#### `panel_on_blur(&mut self, id: PanelId)`

Called when a panel loses focus. Use this for cleanup --- saving draft state, stopping timers, or clearing transient selections.

## Event Handling

### `handle_event(&self, event: &Event, ctx: &EventContext) -> EventResult<M>`

Map raw terminal events to messages. This is called before panel-level handling, so it is the right place for global key bindings that should work regardless of which panel is focused.

The `EventContext` provides read-only state: which panel is focused and whether an overlay is open (though overlays consume events before this method is reached, so `has_overlay()` is primarily informational).

```rust
fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return EventResult::Message(Msg::Save);
            }
            _ => {}
        }
    }
    EventResult::Ignored
}
```

Return `EventResult::Ignored` for events you do not handle. This lets them fall through to the focused panel and then to convention keys.

## Lifecycle

### `init(&self) -> Command<M>`

Called once after the terminal is initialized and before the first render. Use this to kick off initial data loading, start background tasks, or dispatch a setup message.

```rust
fn init(&self) -> Command<Msg> {
    Command::perform(|| load_config(), Msg::ConfigLoaded)
}
```

## Metadata

### `title(&self) -> &str`

The window title displayed in the terminal emulator's title bar. Defaults to `"kitz app"`.

### `tick_rate(&self) -> Duration`

How frequently the runtime polls for events. Defaults to 250 milliseconds. Lower values give snappier response at the cost of higher CPU usage. For most applications, the default is appropriate. If you have subscriptions or animations that need fast updates, consider lowering it to 50--100 ms.

### `theme(&self) -> Theme`

The color theme used for panel borders, overlays, toasts, and the footer. You can return different themes based on application state to support runtime theme switching.

```rust
fn theme(&self) -> Theme {
    self.current_theme.clone()
}
```

### `subscriptions(&self) -> Vec<Subscription<M>>`

Declarative background tasks managed by the runtime. Each subscription has a unique string ID. The runtime diffs the returned set against the currently active subscriptions each frame: new IDs are started, removed IDs are stopped.

```rust
fn subscriptions(&self) -> Vec<Subscription<Msg>> {
    if self.polling_enabled {
        vec![Subscription::every("poll", Duration::from_secs(5), || Msg::Poll)]
    } else {
        Subscription::none()
    }
}
```

## Choosing a Path: Custom vs. Convention

| | Custom (`view()`) | Convention (`panels()`) |
|---|---|---|
| **Layout** | You control everything | Framework-managed split layout |
| **Borders & focus** | Manual | Automatic |
| **Footer** | Manual | Automatic from `panel_key_hints()` |
| **Help overlay (`?`)** | Not available | Automatic |
| **Command palette (`:`)** | Not available | Automatic |
| **Tab navigation** | Manual | Automatic |
| **Zoom (`z`)** | Manual | Automatic |
| **Best for** | Single-pane apps, editors, splash screens | Multi-panel dashboards, list-detail views |

If your application has distinct regions that the user switches between, use the convention path. If your application is a single cohesive surface (like a text editor or a game), use the custom path.

You can start with the custom path and migrate to the convention path later --- they are not mutually exclusive across the lifetime of a project, just within a single render frame. The runtime checks whether `panels()` returns `PanelLayout::None` each frame. If it does, `view()` is called. Otherwise, the panel system takes over.
