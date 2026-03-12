# rataframe

**The Next.js of terminal user interfaces.**

A batteries-included Rust framework for building TUI applications with
[ratatui](https://ratatui.rs). Strong conventions as the default happy path,
with clean escape hatches when you need full control.

> Convention over Configuration — but never Configuration over Freedom.

## Status

**v0.1.0** — All core features implemented. CLI tooling for scaffolding and
code generation is ready. API is stabilising.

## Install

```bash
cargo install rataframe
```

This gives you the `rataframe` CLI — your single entry point for creating
projects, generating components, running, testing, and managing themes.

## Quick Start

```bash
# Create a new project (default: panels template)
rataframe new my-app
cd my-app
cargo run
```

That's it. You get a working panel app with sidebar, detail view, key hints,
and all convention keys (Tab, ?, :, z, Esc, q) wired up.

### Available Templates

| Template | Description |
|----------|-------------|
| `panels` (default) | Two-panel app with sidebar + detail view |
| `minimal` | Single-file counter app — simplest possible starting point |
| `dashboard` | Three-panel dashboard with stats, chart, and log |
| `editor` | Modal text editor with Normal/Insert modes |

```bash
rataframe new my-app --template dashboard
rataframe new my-app --template minimal
rataframe new my-app --template editor
```

## Code Generation

Generate components and have them automatically wired into your project:

```bash
# Generate a panel — creates file, wires into mod.rs, messages.rs, app.rs, tests
rataframe generate panel stats

# Generate a screen — creates file, wires into messages, update, main
rataframe generate screen settings

# Generate an overlay — creates file, wires into main
rataframe generate overlay confirm_delete
```

Every `generate` command:
1. Creates the component file with a working implementation
2. Registers it in the module tree
3. Adds message variants where needed
4. Wires it into the application's update/view/layout logic
5. Adds a test stub

## Development Workflow

```bash
rataframe dev       # Auto-reload on file changes (uses cargo-watch)
rataframe run       # Build and run
rataframe run --release  # Build and run in release mode
rataframe test      # Run tests
rataframe test --watch   # Re-run tests on changes
```

## Themes

```bash
rataframe theme list     # Show all 4 themes with color swatches
rataframe theme preview  # Render a sample UI in each theme
```

Built-in themes: **Nord**, **Tokyo Night**, **Catppuccin Mocha**, **Dracula**.

Cycle themes at runtime with `self.theme = self.theme.next()`.

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
| CLI scaffolding and code generation | ✅ |

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

## Project Structure (generated by `rataframe new`)

```
my-app/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point
│   ├── app.rs           # Application trait impl + panel wiring
│   ├── messages.rs      # All message variants
│   └── panels/
│       ├── mod.rs       # Panel module registry
│       ├── sidebar.rs   # Sidebar panel
│       └── detail.rs    # Detail panel
└── tests/
    └── app_test.rs      # Test stubs
```

Marker comments (`// rataframe:messages`, `// rataframe:update`, etc.) serve
as injection points for the `generate` command. Don't remove them.

## Library Usage (without CLI)

If you prefer to use rataframe as a pure library without the CLI binary:

```toml
[dependencies]
rataframe = { version = "0.1", default-features = false }
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
| `file_manager.rs` | **Real-world app**: panels, overlays, toasts, themes, subscriptions |

Run any example:

```bash
cargo run --example counter
cargo run --example todo
cargo run --example dashboard
```

## Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting-started.md) | Zero to working app in 3 steps |
| [Migration Guide](docs/migration-from-ratatui.md) | Coming from raw ratatui? Start here |
| [Cookbook](docs/cookbook.md) | 12 recipes for common patterns |
| [Design](docs/DESIGN.md) | Architecture and API blueprint |
| [Decisions](docs/DECISIONS.md) | Why we made each major choice |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Changelog](CHANGELOG.md) | Release history |

## Design Philosophy

See [docs/DESIGN.md](docs/DESIGN.md) for the full architectural blueprint and
[docs/DECISIONS.md](docs/DECISIONS.md) for the reasoning behind every major
design choice.

## License

MIT — see [LICENSE](LICENSE) for details.
