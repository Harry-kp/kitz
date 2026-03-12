# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- 7 examples: hello, counter, todo, async_fetch, editor, dashboard, theme_showcase.
- GitHub Actions CI.
