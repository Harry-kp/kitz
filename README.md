# rataframe

**The Next.js of terminal user interfaces.**

A batteries-included Rust framework for building TUI applications with
[ratatui](https://ratatui.rs). Strong conventions as the default happy path,
with clean escape hatches when you need full control.

> Convention over Configuration — but never Configuration over Freedom.

## Status

**v0.1.0** — All core features implemented. API is stabilising.

## Why rataframe?

Building a TUI app with raw ratatui means wiring up the event loop, terminal
management, focus cycling, overlays, theming, error recovery, and testing from
scratch — every time.

rataframe gives you all of that out of the box:

| Feature | Status |
|---------|--------|
| TEA architecture (Model → Message → Update → View) | ✅ |
| Panel system with auto-focus, zoom, borders | ✅ |
| Overlay stack (Confirm, Help, Command Palette) | ✅ |
| Auto-generated footer with key hints | ✅ |
| 4 built-in themes (Nord, Tokyo Night, Catppuccin, Dracula) | ✅ |
| Async commands via `Command::perform` | ✅ |
| Declarative subscriptions (timers, background streams) | ✅ |
| Toast notifications with severity levels | ✅ |
| Screen navigation stack (push/pop) | ✅ |
| Error boundaries (per-panel panic recovery) | ✅ |
| TUI-safe file logging | ✅ |
| TextInput widget (UTF-8, cursor, insert/delete) | ✅ |
| TestHarness (simulate keys, assert state) | ✅ |
| Mouse click-to-focus | ✅ |
| Terminal resize handling + minimum size check | ✅ |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rataframe = { git = "https://github.com/Harry-kp/rataframe" }
```

The simplest app:

```rust
use rataframe::prelude::*;

struct App;

impl Application for App {
    type Message = ();
    fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
        Command::quit()
    }
    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        frame.render_widget(
            Paragraph::new("Hello, rataframe! Press any key to quit."),
            frame.area(),
        );
    }
    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<()> {
        EventResult::Message(())
    }
}

fn main() -> Result<()> { rataframe::run(App) }
```

## Convention Ladder

rataframe grows with your app. Start minimal, add conventions as needed:

### Level 1: Custom View (escape hatch)

Override `view()` for full rendering control. No panels, no auto-footer.
Good for: editors, games, single-screen tools.

```rust
fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
    // You own the entire frame
}
```

### Level 2: Panels

Implement `panels()` and the framework handles borders, focus cycling,
Tab/Shift+Tab, zoom (z), auto-footer, auto-help (?).

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(30)),
        ("main", Constraint::Percentage(70)),
    ])
}
```

### Level 3: Full Framework

Add overlays, subscriptions, toasts, themes, screen navigation,
command palette — all through the same Application trait.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   Runtime                        │
│  ┌─────────┐  ┌──────────┐  ┌────────────────┐ │
│  │Terminal  │  │Event Loop│  │ Subscription   │ │
│  │  Init    │──│  Poll    │──│   Manager      │ │
│  │  Restore │  │  Dispatch│  │   (background) │ │
│  └─────────┘  └──────────┘  └────────────────┘ │
│                     │                            │
│  Event Flow:        ▼                            │
│  Overlay ──→ App ──→ Panel ──→ Convention Keys   │
│                     │                            │
│  ┌──────────────────┴───────────────────────┐   │
│  │              Application                  │   │
│  │  update() ──→ Command  (side-effects)     │   │
│  │  view()   ──→ Frame    (rendering)        │   │
│  │  panels() ──→ Layout   (convention path)  │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

## Examples

| Example | What it proves |
|---------|----------------|
| `hello.rs` | Minimal app — 10 lines |
| `counter.rs` | TEA message dispatch, Command::message |
| `todo.rs` | Panel system, overlays, confirm dialog, help |
| `async_fetch.rs` | Command::perform, background tasks |
| `editor.rs` | Escape hatch, modal editing, TextInput |
| `dashboard.rs` | Subscriptions, toasts, theme cycling |
| `theme_showcase.rs` | All 4 themes side by side |

Run any example:

```bash
cargo run --example counter
cargo run --example todo
cargo run --example dashboard
```

## Design Philosophy

See [docs/DESIGN.md](docs/DESIGN.md) for the full architectural blueprint and
[docs/DECISIONS.md](docs/DECISIONS.md) for the reasoning behind every major
design choice.

## License

MIT — see [LICENSE](LICENSE) for details.
