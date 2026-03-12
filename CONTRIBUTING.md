# Contributing to kitz

Thank you for considering contributing! Here's how to get started.

## Setup

```bash
git clone https://github.com/Harry-kp/kitz.git
cd kitz
cargo build
cargo test
cargo run --example counter  # verify everything works
```

## Architecture Overview

```
src/
├── app.rs           # Application trait — the core TEA interface
├── command.rs       # Command<M> — side-effect descriptions
├── context.rs       # Context, ViewContext, EventContext, Intent
├── runtime/
│   ├── mod.rs       # Main event loop, rendering, dispatch
│   └── terminal.rs  # Terminal init/restore, panic hook
├── panel/
│   ├── mod.rs       # PanelId, KeyHint types
│   ├── layout.rs    # PanelLayout enum, rect computation
│   ├── manager.rs   # Focus cycling, zoom state
│   └── error_boundary.rs  # catch_unwind per panel
├── overlay/
│   ├── mod.rs       # Overlay trait, OverlayStack
│   ├── confirm.rs   # ConfirmOverlay
│   ├── help.rs      # HelpOverlay
│   └── command_palette.rs  # CommandPaletteOverlay + fuzzy search
├── screen/mod.rs    # Screen trait, NavigationStack
├── subscription.rs  # Declarative background subscriptions
├── theme/
│   ├── mod.rs       # Theme struct
│   └── palettes.rs  # Nord, Tokyo Night, Catppuccin, Dracula
├── toast/mod.rs     # ToastManager, ToastWidget
├── widgets/
│   ├── footer.rs    # Auto-generated footer bar
│   ├── text_input.rs # TextInput widget
│   └── centered_rect.rs  # Layout utility
├── testing.rs       # TestHarness for unit tests
├── logging.rs       # TUI-safe file logging via tracing
├── prelude.rs       # Convenience re-exports
└── lib.rs           # Crate root, run() entry point
```

## Key Concepts

- **TEA (The Elm Architecture)**: All state changes flow through `update()`.
  Events become Messages, Messages produce Commands (side-effects).
- **Convention Path vs Escape Hatch**: Implement `panels()` for the
  convention path. Override `view()` for full control.
- **Intents**: Side-effects on framework state (overlays, focus, toasts)
  go through `Context` intents, not Commands.

## Making Changes

1. **Fork and branch** from `main`
2. **Run checks** before submitting:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets
   cargo test
   cargo build --examples
   ```
3. **Write an example** if adding a new feature
4. **Update docs** if changing public API

## Pull Request Guidelines

- Keep PRs focused — one feature or fix per PR
- Include a clear description of what and why
- All CI checks must pass
- Prefer small, incremental changes

## Code Style

- Follow `cargo fmt` output
- No `unwrap()` in library code — use `Result` or `Option`
- Public API must have doc comments
- Comments explain *why*, not *what*

## License

By contributing, you agree that your contributions will be licensed under the
MIT License.
