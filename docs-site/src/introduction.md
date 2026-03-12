# Introduction

**Kitz is the application framework for ratatui.** It gives you the architecture, conventions, and tooling to build terminal user interfaces that scale from a weekend hack to a production-grade dashboard — without fighting the terminal.

Think of it as the Next.js of terminal user interfaces: opinionated where it matters, flexible where it counts, and zero boilerplate to get started.

```
cargo install kitz
kitz new my-app && cd my-app && cargo run
```

That one command gives you a working multi-panel TUI with a sidebar, a detail pane, keyboard navigation, a help overlay, a command palette, toast notifications, theming, and a test harness. All before you write a single line of application logic.

---

## Why Kitz Exists

Ratatui is an excellent rendering library, but building a real application on top of it means inventing the same things over and over: an event loop, a message bus, panel focus management, modal overlays, error boundaries, a footer, a help screen, logging that doesn't destroy your terminal, and tests that don't need a terminal at all.

Kitz extracts those patterns into a cohesive framework so you can focus on what makes your application different.

**The premise is simple.** You describe your state. You describe how messages change that state. Kitz handles everything else: the terminal lifecycle, rendering, event routing, overlays, toasts, subscriptions, and graceful shutdown — even on panic.

---

## Architecture at a Glance

Kitz follows The Elm Architecture (TEA), adapted for Rust and terminal I/O:

```
               ┌──────────────────────────────────────────┐
               │              kitz runtime                 │
               │                                           │
  Terminal ──► │  Event ──► handle_event() ──► Message     │
  Events       │                                  │        │
               │                                  ▼        │
               │                             update()      │
               │                              │    │       │
               │                    ┌─────────┘    └───┐   │
               │                    ▼                   ▼   │
               │               State change         Command │
               │                    │            (side-effects)
               │                    ▼                       │
               │               view() / panels()            │
               │                    │                       │
               │                    ▼                       │
               │              Terminal output               │
               └──────────────────────────────────────────┘
```

1. **Events** arrive from the terminal (key presses, mouse clicks, resize).
2. Your `handle_event` method maps them to **messages** — plain enum variants.
3. Your `update` method receives each message, mutates state, and returns a **Command** describing side-effects (quit, spawn a background task, re-dispatch another message).
4. The runtime calls `view()` (custom path) or `panels()` (convention path) to render the next frame.

Commands are values, not callbacks. Your application never touches the terminal directly and never spawns threads itself. This makes the entire update cycle deterministic and testable.

---

## Feature Highlights

Kitz ships 21 features out of the box, all opt-in through the `Application` trait:

| # | Feature | What it does |
|---|---------|-------------|
| 1 | **TEA runtime** | Event loop, message dispatch, command processing |
| 2 | **Panel system** | Declarative multi-panel layouts with borders and focus |
| 3 | **Panel focus** | Tab / Shift+Tab cycling, click-to-focus, programmatic focus |
| 4 | **Panel zoom** | `z` to zoom any panel to full screen, `z` again to restore |
| 5 | **Footer** | Auto-generated from your `panel_key_hints()` |
| 6 | **Help overlay** | `?` opens a help screen built from all panel hints |
| 7 | **Command palette** | `:` opens a fuzzy-searchable palette of every action |
| 8 | **Confirm dialog** | `ConfirmOverlay` with Yes/No dispatch |
| 9 | **Custom overlays** | Implement the `Overlay` trait for any modal dialog |
| 10 | **Overlay stack** | Multiple overlays can be layered; Esc pops one at a time |
| 11 | **Toast notifications** | `ctx.toast("message", ToastLevel::Info)` — auto-dismiss |
| 12 | **Screen navigation** | Push/pop screens onto a navigation stack |
| 13 | **Themes** | 6 built-in palettes; swap at runtime with a single method |
| 14 | **Subscriptions** | Declarative background timers (tick, polling, animation) |
| 15 | **Async commands** | `Command::perform` spawns background work and maps results back to messages |
| 16 | **Error boundaries** | Panel rendering panics are caught and displayed inline |
| 17 | **Terminal safety** | Restore on panic, Ctrl+C hard quit, minimum-size guard |
| 18 | **Test harness** | `TestHarness` — simulate keys and messages without a terminal |
| 19 | **File logging** | `init_logging()` writes to `~/.local/share/kitz/` with daily rotation |
| 20 | **CLI scaffolding** | `kitz new`, `kitz generate panel|screen|overlay`, `kitz dev` |
| 21 | **Code generation** | Marker comments (`// kitz:update`, etc.) let the CLI wire new components automatically |

---

## The Convention Ladder

Kitz offers three levels of structure. Start wherever you are comfortable and move up when you need more.

### Level 1: Custom Path

Override `view()` and `handle_event()`. You get the TEA runtime, terminal safety, and nothing else. This is raw ratatui with a better event loop.

```rust
impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        // handle state changes
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        // full control over rendering
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
        // full control over input
        EventResult::Ignored
    }
}
```

### Level 2: Convention Path

Implement `panels()` instead of `view()`. Kitz renders borders, manages focus, generates the footer and help overlay, and routes keys to the focused panel. You still own the rendering inside each panel.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(30)),
        ("detail",  Constraint::Percentage(70)),
    ])
}
```

With this single method, you get Tab/Shift+Tab focus cycling, `z` for zoom, `?` for help, `:` for the command palette, and `q` to quit — all for free.

### Level 3: Full Framework

Add overlays, screens, subscriptions, toasts, themes, and async commands. Use the CLI to scaffold new components. Use `TestHarness` to test without a terminal. Use `kitz dev` for auto-reload. This is the full framework experience.

---

## Quick Start

Install the CLI:

```
cargo install kitz
```

Create a project:

```
kitz new my-app && cd my-app && cargo run
```

You now have a multi-panel TUI with a sidebar and detail pane. Press Tab to switch panels, `?` for help, `:` for the command palette, `z` to zoom, and `q` to quit.

See [Installation](getting-started/installation.md) for prerequisites and alternative setups, or jump straight to [Your First App](getting-started/your-first-app.md) for a hands-on tutorial.

---

## Who Is This For

- **Ratatui users** who are tired of reimplementing the same scaffolding for every project.
- **CLI tool authors** who want their dashboards and interactive UIs to feel polished.
- **Elm/Iced/TEA fans** who want that architecture in the terminal.
- **Teams** who need testable, maintainable TUI applications with consistent structure.

---

## Links

- [GitHub](https://github.com/Harry-kp/kitz)
- [API Reference (docs.rs)](https://docs.rs/kitz)
- [Examples](https://github.com/Harry-kp/kitz/tree/main/examples) — hello, counter, todo, dashboard, editor, file manager, async fetch, theme showcase
