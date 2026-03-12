# Changelog

All notable changes to kitz are documented here.

---

## v0.2.0

The CLI tooling release. Adds the `kitz` command-line tool for scaffolding projects, generating components, and streamlining the development workflow.

### Added

- **`kitz new` command.** Scaffold a new project from one of four templates: `panels` (default), `minimal`, `dashboard`, and `editor`. Generates a complete project with `Cargo.toml`, application code, message enum, panels, and tests. Automatically runs `git init` and `cargo check`.
- **`kitz generate` command.** Generate panels, screens, and overlays and wire them into an existing project. The generator inserts module declarations, message variants, update match arms, layout entries, and test stubs using marker comments in the source code.
- **`kitz dev` command.** Start development with auto-reload using `cargo-watch`. Automatically installs `cargo-watch` if not present.
- **`kitz run` command.** Build and run the application, with optional `--release` flag for optimized builds.
- **`kitz test` command.** Run the test suite with `cargo test`. Supports `--watch` for continuous test execution during development.
- **`kitz theme list` command.** Display all built-in themes with ANSI true-color swatches showing every semantic color slot.
- **`kitz theme preview` command.** Render a sample UI panel in each built-in theme to preview how colors work together.
- **Template variable substitution.** Project templates support `{{project_name}}` (snake_case), `{{ProjectName}}` (PascalCase), and `{{project-name}}` (kebab-case) placeholders.
- **Marker comment system.** Generated projects include `// kitz:*` marker comments that the generator uses as insertion points. Markers are preserved across multiple generator runs.
- **Layout rebalancing.** When a new panel is generated, `Constraint::Percentage` values are recalculated to distribute space evenly.

---

## v0.1.0

The initial release of the kitz framework. Provides the core architecture for building terminal user interfaces with The Elm Architecture.

### Added

- **`Application` trait.** The central trait with `update`, `view`, `handle_event`, `panels`, and lifecycle hooks. Only `update` is required; all other methods have sensible defaults.
- **Command system.** `Command::none()`, `Command::quit()`, `Command::message()`, `Command::perform()`, and `Command::batch()` for describing side-effects as values.
- **Panel system.** `PanelLayout` with `Single`, `Horizontal`, `Vertical`, and `Nested` variants. Automatic border rendering, focus indicators, and focus cycling with Tab/Shift+Tab.
- **Overlay system.** `Overlay` trait with built-in `ConfirmOverlay`, `HelpOverlay`, and `CommandPaletteOverlay`. Overlays render on top of the main content and capture input.
- **Screen navigation.** `Screen` trait and navigation stack with `push_screen` and `pop_screen` for multi-screen applications.
- **Subscription system.** `Subscription::every()` for periodic messages with automatic lifecycle management. The runtime starts and stops subscription threads based on the set returned from `subscriptions()` each frame.
- **Theme system.** Semantic `Theme` struct with four built-in palettes: Nord (default), Tokyo Night, Catppuccin Mocha, and Dracula. `Theme::next()` for runtime cycling.
- **Toast notifications.** `ToastLevel::Info`, `Success`, `Warning`, `Error` with auto-dismiss rendering in the top-right corner.
- **Convention keys.** `q` to quit, `Esc` to dismiss/back, `Tab`/`Shift+Tab` for panel focus, `z` for panel zoom, `?` for help overlay, `:` for command palette.
- **Auto-footer.** Renders key hints from the focused panel's `panel_key_hints()`.
- **Error boundaries.** Panel rendering errors are caught and displayed inline without crashing the application.
- **`Context` API.** `push_overlay`, `pop_overlay`, `focus_panel`, `toggle_zoom`, `toast`, `push_screen`, `pop_screen`.
- **`TestHarness`.** Headless test runner with `press_key`, `send_key`, `press_panel_key`, `send_message`, `app`, `app_mut`, `quit_requested`. Skips `Command::perform` actions for deterministic tests.
- **File-based logging.** `kitz::logging::init_logging()` with `tracing` integration. Daily log rotation. Writes to `~/.local/share/kitz/<app>/app.log`.
- **Widgets.** `TextInput` / `TextInputState` for text input, `Footer` for key hint bars, `centered_rect` helper.
- **`kitz::prelude`** module re-exporting all commonly used types.
- **Mouse support.** Click-to-focus on panels.
- **Minimum terminal size check.** Displays a message when the terminal is too small.
- **Panic recovery.** Uses `color_eyre` for clean terminal restoration on panic.
