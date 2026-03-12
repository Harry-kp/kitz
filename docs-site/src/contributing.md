# Contributing

Contributions to kitz are welcome. This guide covers the development setup, project architecture, and guidelines for submitting changes.

## Development Setup

### Prerequisites

- Rust stable (latest) via [rustup](https://rustup.rs)
- A terminal that supports ANSI colors (for theme development and testing)

### Clone and Build

```bash
git clone https://github.com/your-org/kitz.git
cd kitz
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Examples

The `examples/` directory contains complete applications that exercise various framework features:

```bash
cargo run --example counter
cargo run --example todo
cargo run --example dashboard
cargo run --example editor
cargo run --example async_fetch
cargo run --example theme_showcase
cargo run --example hello
cargo run --example file_manager
```

## Project Architecture

The codebase is organized into focused modules:

```
src/
  lib.rs              # Public API and kitz::run() entry point
  app.rs              # Application trait and EventResult enum
  command.rs          # Command type and side-effect descriptions
  context.rs          # Context, ViewContext, EventContext
  subscription.rs     # Subscription type and SubscriptionManager
  error.rs            # Error types
  prelude.rs          # Re-exports for use kitz::prelude::*

  runtime/
    mod.rs            # Main event loop, command processing, convention keys
    terminal.rs       # Terminal init/restore, panic hooks

  panel/
    mod.rs            # Panel trait types, PanelId
    layout.rs         # PanelLayout enum and rect computation
    manager.rs        # PanelManager (focus tracking, zoom state)
    error_boundary.rs # Error boundary for panel rendering

  overlay/
    mod.rs            # Overlay trait, OverlayStack
    confirm.rs        # ConfirmOverlay
    help.rs           # HelpOverlay
    command_palette.rs # CommandPaletteOverlay

  screen/
    mod.rs            # Screen trait and NavigationStack

  theme/
    mod.rs            # Theme struct
    palettes.rs       # Built-in theme definitions

  toast/
    mod.rs            # ToastManager, Toast, ToastWidget

  widgets/
    mod.rs            # Widget re-exports
    text_input.rs     # TextInput and TextInputState
    footer.rs         # Footer widget
    centered_rect.rs  # Layout helper

  logging.rs          # init_logging() for TUI-safe file logging
  testing.rs          # TestHarness

  cli/
    main.rs           # CLI binary entry point
    mod.rs            # Clap command definitions
    commands/         # Command implementations (new, generate, dev, theme)
    generators/       # Code generators (panel, screen, overlay, helpers)
    templates/        # Project templates (minimal, panels, dashboard, editor)
```

### Key Design Principles

- **The Elm Architecture.** State is updated through messages. Side-effects are described as `Command` values, not executed directly. This keeps `update` pure and testable.
- **Convention over configuration.** The panel system, auto-footer, help overlay, and convention keys work out of the box. Escape hatches allow opting out.
- **Values, not callbacks.** `Command` and `Subscription` are data types. The runtime interprets them. This makes the framework easier to reason about and test.
- **Progressive adoption.** Only `update` is required. Every other `Application` method has a default. Features are adopted incrementally.

## Pull Request Guidelines

### Before Submitting

1. **Run `cargo test`** and ensure all tests pass.
2. **Run `cargo clippy`** and resolve any warnings.
3. **Run `cargo fmt`** to ensure consistent formatting.
4. **Add tests** for new functionality. Use `TestHarness` for application-level behavior. Use unit tests for isolated modules.
5. **Update examples** if your change affects the public API or adds a notable feature.

### Commit Messages

Write clear, descriptive commit messages. Use the imperative mood:

- "Add subscription cancellation on drop"
- "Fix panel focus cycling when panels are removed"
- "Update ConfirmOverlay to support custom button labels"

### PR Description

Include:

- A summary of the change and its motivation.
- Any breaking changes to the public API.
- How to test the change (e.g., which example to run, which test to look at).

### Scope

- Keep PRs focused. One feature or fix per PR.
- If a PR touches many files, explain why in the description.
- Large refactors should be discussed in an issue first.

## Code Style

- Follow standard Rust conventions and `rustfmt` defaults.
- Public types and functions must have doc comments.
- Use `pub(crate)` for internal types that should not be part of the public API.
- Avoid unnecessary allocations in the render path (the `view` and `panel_view` methods are called every frame).
- Prefer returning `Command` values over mutating external state.
- Test with `TestHarness` when possible. Reserve integration tests in `tests/` for CLI and end-to-end scenarios.

## Adding a Built-in Theme

1. Add the palette function to `src/theme/palettes.rs`.
2. Add the theme to the `all()` function so it appears in `Theme::next()` cycling.
3. Add the theme info to `src/cli/commands/theme.rs` so it appears in `kitz theme list` and `kitz theme preview`.
4. Add a test that verifies the palette has distinct colors for all semantic slots.

## Adding a Built-in Overlay

1. Create a new file in `src/overlay/`.
2. Implement the `Overlay<M>` trait.
3. Re-export it from `src/overlay/mod.rs`.
4. Add it to `src/prelude.rs` if it is expected to be commonly used.
5. Add an example or extend an existing example to demonstrate it.

## Reporting Issues

When filing a bug report, include:

- The kitz version (`kitz --version` or check `Cargo.toml`).
- Your terminal emulator and OS.
- A minimal reproduction case if possible.
- The full error output or a screenshot of the visual issue.
