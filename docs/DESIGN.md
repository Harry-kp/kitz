# rataframe — Architecture & Design Plan

> The application framework for ratatui.

## Design Philosophy

**"rataframe is to ratatui what Next.js is to React"**

Strong conventions as the happy path. Clean escape hatches when conventions don't fit.

### Three Paths, One Framework

1. **Minimal path** (simple tools): Just `update()` + `view()`. 10 lines. Terminal safety, event loop, panic handling — all free.
2. **Convention path** (80% of apps): Implement `Panel` trait, return `PanelLayout` from `panels()`, define `key_hints()`. Framework auto-generates footer, help screen, command palette, focus cycling, key conventions.
3. **Custom path** (editors, exotic layouts): Skip `panels()`, implement `view()` directly. Lose auto-footer and auto-help, but keep runtime, commands, overlays, themes, toasts, subscriptions, navigation.

### Convention Keys

These work out of the box, overridable via `EventResult::Consumed`:

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Toggle help overlay |
| `:` | Open command palette |
| `Tab` / `Shift+Tab` | Cycle panel focus |
| `Esc` | Back chain: pop overlay → pop screen → quit |
| `z` | Toggle zoom on focused panel |
| `Ctrl+C` | Hard quit (non-overridable) |

---

## Core API

### Application Trait

```rust
pub trait Application: Sized + 'static {
    type Message: Debug + Send + 'static;

    // Required
    fn update(&mut self, msg: Self::Message, ctx: &mut Context<Self::Message>) -> Command<Self::Message>;

    // Escape hatch: full rendering control
    fn view(&self, frame: &mut Frame, ctx: &ViewContext) { /* default placeholder */ }

    // Convention path: declare panels via methods
    fn panels(&self) -> PanelLayout { PanelLayout::None }
    fn panel_title(&self, id: PanelId) -> &str { "" }
    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {}
    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> { vec![] }
    fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Self::Message> { EventResult::Ignored }
    fn panel_on_focus(&mut self, id: PanelId) {}
    fn panel_on_blur(&mut self, id: PanelId) {}

    // Event handling
    fn handle_event(&self, event: &Event, ctx: &EventContext) -> EventResult<Self::Message> {
        EventResult::Ignored
    }

    // Lifecycle
    fn init(&self) -> Command<Self::Message> { Command::none() }

    // Subscriptions
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> { vec![] }

    // Metadata
    fn title(&self) -> &str { "rataframe app" }
    fn tick_rate(&self) -> Duration { Duration::from_millis(250) }
    fn theme(&self) -> Theme { Theme::default() }
}
```

> **Design note:** Panel behavior is expressed as methods on the Application trait
> rather than as a separate Panel trait. This avoids borrow-checker friction when
> panels need to read/write shared application state — a common pattern in TUIs.

Framework auto-behaviors when `panels()` returns a layout:
1. Renders panels in the declared layout with themed borders
2. Draws focused border color on the focused panel
3. Tab/Shift+Tab cycles focus between panels
4. Mouse click on a panel focuses it
5. Footer shows focused panel's `panel_key_hints()` + global hints
6. `?` opens help overlay — all panels' hints grouped by title
7. `:` opens command palette — all hints fuzzy-searchable
8. Focused panel's `panel_handle_key()` is called before convention keys
9. `panel_on_focus()` / `panel_on_blur()` fire on focus changes
10. `z` toggles focused panel fullscreen

### PanelLayout

```rust
pub enum PanelLayout {
    None,
    Single(PanelId),
    Horizontal(Vec<(PanelId, Constraint)>),
    Vertical(Vec<(PanelId, Constraint)>),
    Nested(Direction, Vec<(Box<PanelLayout>, Constraint)>),
}
```

### EventResult

```rust
pub enum EventResult<M> {
    Message(M),   // dispatch to update()
    Ignored,      // fall through to panel → convention keys
    Consumed,     // stop processing
}
```

### Command (side effects)

```rust
impl<M: Send + 'static> Command<M> {
    pub fn none() -> Self;
    pub fn quit() -> Self;
    pub fn batch(cmds: impl IntoIterator<Item = Command<M>>) -> Self;
    pub fn message(msg: M) -> Self;
    pub fn perform<T, F, Map>(task: F, mapper: Map) -> Self;  // background thread
}
```

### Context (framework intents)

Framework-level actions are requested via `Context`, which is passed mutably to `update()`:

```rust
impl<M> Context<M> {
    pub fn push_overlay(&mut self, overlay: impl Overlay<M> + 'static);
    pub fn pop_overlay(&mut self);
    pub fn focus_panel(&mut self, id: PanelId);
    pub fn toggle_zoom(&mut self);
    pub fn toast(&mut self, message: impl Into<String>, level: ToastLevel);
    pub fn push_screen(&mut self, screen: impl Screen<M> + 'static);
    pub fn pop_screen(&mut self);
}
```

> **Design note:** Overlay/screen/toast operations live on Context rather than
> Command because they require mutable framework state. Command remains a pure
> description of app-level side effects.

### Screen Trait (navigation)

```rust
pub trait Screen<M> {
    fn panels(&self) -> PanelLayout<M>;
    fn title(&self) -> &str;
    fn handle_event(&self, event: &Event) -> EventResult<M> { EventResult::Ignored }
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
}
```

### Overlay Trait

```rust
pub trait Overlay<M> {
    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_event(&self, event: &Event) -> EventResult<M>;
    fn title(&self) -> &str { "" }
}
```

Built-in overlays: Confirm, Help (auto-generated), CommandPalette (fuzzy search).

### Subscription

```rust
impl<M> Subscription<M> {
    pub fn none() -> Vec<Self>;
    pub fn every(id: &'static str, interval: Duration, msg_fn: impl Fn() -> M) -> Self;
}
```

> `watch_file()` is planned for a future release.

---

## Event Flow

```
Raw terminal event
  │
  ▼
Ctrl+C? ──► hard quit (non-overridable)
  │
  ▼
Active overlay? ──► overlay.handle_event()
  │                   ├── Message(msg) ──► dispatch to update(), stop
  │                   ├── Consumed ──► stop
  │                   └── Ignored ──► continue
  │
  ▼
App.handle_event()
  ├── Message(msg) ──► dispatch to update(), stop
  ├── Consumed ──► stop
  └── Ignored ──► continue
         │
         ▼
Focused panel.handle_key()
  ├── Some(msg) ──► dispatch to update(), stop
  └── None ──► continue
         │
         ▼
Convention keys:
  q         ──► quit
  ?         ──► toggle Help overlay
  :         ──► open CommandPalette
  Tab       ──► focus next panel
  Shift+Tab ──► focus prev panel
  z         ──► toggle zoom
  Esc       ──► pop overlay → pop screen → quit
  Mouse     ──► panel hit-test + focus
```

---

## Feature Summary (21 features)

### Core Runtime
1. Terminal lifecycle — init, restore, alternate screen
2. Panic safety — always restores terminal, shows friendly error
3. Event loop — poll events, dispatch, render (tick + events built-in)

### TEA Architecture
4. Application trait — single required method (update), progressive defaults
5. Command system — async side effects via futures, batch composition
6. Subscription system — declarative, state-dependent, auto-managed

### Convention Path
7. Panel trait — id, title, view, key_hints, handle_key, lifecycle hooks
8. PanelLayout — declarative layout tree (horizontal, vertical, nested, dynamic)
9. Auto-footer — generated from focused panel's key_hints() + global hints
10. Auto-help overlay — generated from ALL panels' key_hints(), grouped by title
11. Command palette — fuzzy-searchable actions auto-discovered from key_hints()
12. Convention keys — q, ?, :, Tab, Esc, z, Ctrl+C

### Navigation
13. Screen stack — push/pop screens, on_enter/on_leave lifecycle, Esc-to-go-back

### Overlays
14. Overlay system — Overlay trait, OverlayStack, Esc chain, built-in Confirm

### Visual
15. Theme system — semantic colors, 4 palettes (Nord, Tokyo Night, Catppuccin, Dracula)
16. Toast system — queue, auto-dismiss, severity levels

### Widgets
17. TextInput — cursor, UTF-8 aware, selection

### Safety
18. Error boundaries — per-panel catch_unwind, graceful degradation

### Developer Experience
19. Logging integration — tracing to file, TUI-safe
20. TestHarness — simulate keys, assert state, check panel focus
21. Project template — `template/` directory for scaffolding new apps

---

## Convention Ladder (Progressive Disclosure)

```
Level 0: Minimal
  impl Application { update + view }
  You get: terminal safety, event loop, panic handling

Level 1: TEA
  impl Application { update + view + handle_event }
  You get: + message dispatch, convention keys (q, Ctrl+C)

Level 2: Panels (the happy path)
  impl Application { update + panels }
  impl Panel for each panel { view + key_hints + handle_key }
  You get: + auto-layout, focus cycling, auto-footer, auto-help, command palette, zoom

Level 3: Navigation
  impl Screen for sub-screens { panels + on_enter/on_leave }
  Command::push_screen / pop_screen
  You get: + screen stack, Esc-to-go-back

Level 4: Full framework
  + Subscriptions, Toasts, Themes, Overlays, Async Commands
  You get: everything

Custom: Escape hatch
  Override view() at any level to take full rendering control
  Override handle_event() to suppress any convention
```

---

## File Structure

```
src/
  lib.rs                         -- pub mod + prelude + run()
  prelude.rs                     -- re-exports
  app.rs                         -- Application trait, EventResult
  command.rs                     -- Command struct, Action variants
  context.rs                     -- Context, ViewContext, EventContext
  subscription.rs                -- Subscription, every(), watch_file()
  runtime/
    mod.rs                       -- main loop: event → dispatch → render
    terminal.rs                  -- init / restore / panic hook
    subscription_manager.rs      -- diff subscriptions, manage lifecycle
  panel/
    mod.rs                       -- Panel trait, PanelId, KeyHint
    layout.rs                    -- PanelLayout enum + constructors
    manager.rs                   -- PanelManager: focus, zoom, mouse
    error_boundary.rs            -- catch_unwind wrapper
  screen/
    mod.rs                       -- Screen trait
    navigator.rs                 -- NavigationStack
  overlay/
    mod.rs                       -- Overlay trait, OverlayStack
    confirm.rs                   -- built-in Confirm dialog
    help.rs                      -- auto-generated Help
    command_palette.rs           -- fuzzy-search action palette
  toast/
    mod.rs                       -- ToastManager, Toast, ToastLevel
    widget.rs                    -- toast render widget
  theme/
    mod.rs                       -- Theme struct
    palettes.rs                  -- Nord, Tokyo Night, Catppuccin, Dracula
  widgets/
    mod.rs                       -- re-exports
    footer.rs                    -- auto-footer renderer
    text_input.rs                -- TextInput widget
    centered_rect.rs             -- layout utility
  logging.rs                     -- tracing file appender setup
  testing.rs                     -- TestHarness
examples/
  hello.rs                       -- ~10 lines, proves runtime
  counter.rs                     -- TEA basics
  todo.rs                        -- convention path: panels + overlays
  async_fetch.rs                 -- Command::perform
  editor.rs                      -- modes (normal/insert)
  dashboard.rs                   -- full framework showcase
  theme_showcase.rs              -- all themes side by side
```

---

## Implementation Phases

### Phase 1: Hello World
Runtime, Application trait, Command (none/quit), event loop, `hello.rs`.

### Phase 2: Counter
Full TEA dispatch, EventResult, convention key `q`, `counter.rs`.

### Phase 3: Todo App
Panel trait, PanelLayout, PanelManager, Overlay system, Confirm, auto-footer, auto-help, `todo.rs`.

### Phase 4: Async Fetch
Command::perform, Command::batch, `async_fetch.rs`.

### Phase 5: Editor
TextInput widget, mode handling via handle_event, `editor.rs`.

### Phase 6: Dashboard
Subscriptions, Toasts, Themes, Command palette, `dashboard.rs`.

### Phase 7: Navigation + Safety
Screen trait, NavigationStack, error boundaries, logging.

### Phase 8: Testing + Polish
TestHarness, mouse, resize, `theme_showcase.rs`.

### Phase 9: Docs + CI + Marketing
README, rustdoc, CONTRIBUTING, CI, cargo-generate, GIF.

---

## Dependencies

```toml
[dependencies]
ratatui = "0.30"
crossterm = "0.29"
color-eyre = "0.6"
unicode-width = "0.2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
nucleo-matcher = "0.3"
```
