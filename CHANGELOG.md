# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-13

### Added

- **CLI tooling**: `cargo install kitz` provides the `kitz` CLI binary.
- **`kitz new <name>`**: Scaffold new projects from 4 built-in templates (minimal, panels, dashboard, editor).
- **`kitz generate panel <name>`**: Generate a panel and auto-wire it into mod.rs, messages.rs, app.rs, and tests.
- **`kitz generate screen <name>`**: Generate a screen with navigation wiring.
- **`kitz generate overlay <name>`**: Generate an overlay with full `Overlay` trait implementation.
- **`kitz dev`**: Auto-reload development mode via `cargo-watch`.
- **`kitz run`** / **`kitz test`**: Convenience wrappers for cargo commands.
- **`kitz theme list`** / **`kitz theme preview`**: Browse built-in themes with ANSI color swatches.
- **Marker comment system**: Generated projects use `// kitz:X` markers as stable injection points for code generation.
- **Convention structure**: Opinionated project layout (src/app.rs, src/messages.rs, src/panels/, src/screens/, src/overlays/).
- **24 CLI integration tests** covering project scaffolding, code generation, and template substitution.

### Changed

- CLI is feature-gated (`cli` feature, default-enabled). Library users can opt out with `default-features = false`.
- `clap` and `colored` added as optional dependencies for CLI.
- README rewritten to lead with CLI workflow as the primary getting-started path.
- Getting-started guide updated for CLI-first workflow.

## [0.1.0] - 2026-03-05

### Added

- **Core runtime**: Terminal lifecycle, panic safety, event loop with configurable tick rate.
- **Application trait**: TEA (The Elm Architecture) with `update()`, `view()`, `handle_event()`, `init()`.
- **Command system**: `Command::none`, `quit`, `message`, `batch`, `perform` (background tasks via threads).
- **Panel system**: `PanelLayout` (Single, Horizontal, Vertical, Nested), focus cycling (Tab/Shift+Tab), zoom (z), mouse click-to-focus.
- **Auto-footer**: Generated from focused panel's `key_hints()` plus global shortcuts.
- **Auto-help overlay**: `?` opens help grouped by panel with all key hints.
- **Command palette**: `:` opens fuzzy-searchable palette auto-populated from key hints.
- **Convention keys**: `q` quit, `?` help, `:` palette, `Tab` focus, `Esc` back chain, `z` zoom, `Ctrl+C` hard quit.
- **Overlay system**: `Overlay` trait, `OverlayStack`, built-in `ConfirmOverlay`, `HelpOverlay`, `CommandPaletteOverlay`.
- **Screen navigation**: `Screen` trait, `NavigationStack` with push/pop and Esc-to-go-back.
- **Theme system**: Semantic colors with 4 built-in palettes (Nord, Tokyo Night, Catppuccin Mocha, Dracula).
- **Toast system**: Queue-based notifications with auto-dismiss, severity levels (Info, Success, Warning, Error).
- **TextInput widget**: Single-line input with cursor, UTF-8 support.
- **Subscription system**: Declarative background tasks (`Subscription::every`).
- **Error boundaries**: Per-panel `catch_unwind` with graceful degradation.
- **TUI-safe logging**: `tracing` integration with rolling file appender.
- **TestHarness**: Simulate key presses and messages, assert on app state — no terminal needed.
- **Context system**: `ctx.push_overlay()`, `ctx.toast()`, `ctx.push_screen()`, `ctx.focus_panel()`, `ctx.toggle_zoom()`.
- 8 examples: hello, counter, todo, async_fetch, editor, dashboard, theme_showcase, file_manager.
- GitHub Actions CI.
