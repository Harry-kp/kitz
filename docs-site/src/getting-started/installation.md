# Installation

There are two ways to use kitz: as a **CLI tool** that scaffolds and manages projects, or as a **library-only dependency** that you add to an existing Cargo project. Most users will want both.

---

## Prerequisites

- **Rust 1.80 or later.** Install or update via [rustup](https://rustup.rs):

  ```
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  ```

- **A terminal emulator.** Any modern terminal works: iTerm2, Alacritty, WezTerm, kitty, Windows Terminal, the default macOS Terminal, or any xterm-compatible emulator. Kitz uses crossterm for terminal I/O, so it runs on macOS, Linux, and Windows out of the box.

---

## Option A: Install the CLI

```
cargo install kitz
```

This compiles the `kitz` binary and places it in `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

Verify the installation:

```
kitz --version
```

The CLI gives you:

| Command | Purpose |
|---------|---------|
| `kitz new <name>` | Scaffold a new project with working panels, messages, and tests |
| `kitz generate panel <name>` | Generate a panel module and wire it into the project |
| `kitz generate screen <name>` | Generate a screen module and wire it into the project |
| `kitz generate overlay <name>` | Generate an overlay module and wire it into the project |
| `kitz dev` | Start auto-reload development (installs `cargo-watch` if needed) |
| `kitz run` | Build and run the application |
| `kitz run --release` | Build in release mode and run |
| `kitz test` | Run the project's test suite |
| `kitz test --watch` | Re-run tests on every file change |
| `kitz theme list` | Display all built-in themes with color swatches |
| `kitz theme preview` | Render a sample UI in each theme |

After installing the CLI, scaffold your first project:

```
kitz new my-app
cd my-app
cargo run
```

The generated project depends on kitz as a library (with `default-features = false` so the CLI code is not included in your binary).

---

## Option B: Library-Only Dependency

If you already have a Cargo project and want to add kitz without the CLI binary, add it to your `Cargo.toml` with default features disabled:

```toml
[dependencies]
kitz = { version = "0.1", default-features = false }
ratatui = "0.30"
crossterm = "0.29"
color-eyre = "0.6"
```

Setting `default-features = false` excludes the `cli` feature (and its dependencies on `clap` and `colored`), keeping your binary lean.

You can still use the CLI commands like `kitz dev` and `kitz generate` from outside the project if you have the CLI installed globally. The CLI operates on source files and does not need to be a dependency of your project.

### Optional Features

| Feature | What it enables |
|---------|----------------|
| `cli` | The `kitz` binary and its dependencies (clap, colored). Enabled by default. |
| `tokio` | Tokio runtime integration for async commands. Add when you need `async`/`await` in background tasks. |

Enable the tokio feature when needed:

```toml
[dependencies]
kitz = { version = "0.1", default-features = false, features = ["tokio"] }
```

---

## Verifying Your Setup

The fastest way to confirm everything works:

```
cargo install kitz
kitz new hello-test
cd hello-test
cargo run
```

You should see a two-panel TUI with a sidebar and detail pane. Press `?` to open the help overlay, Tab to switch panels, and `q` to quit.

If `cargo run` fails, check:

1. Your Rust version: `rustc --version` should print 1.80.0 or higher.
2. Your terminal supports alternate screen mode (virtually all modern terminals do).
3. On Windows, you are using Windows Terminal or another terminal that supports ANSI escape codes.

---

## Updating

To update the CLI to the latest version:

```
cargo install kitz --force
```

To update the library dependency in an existing project, change the version in `Cargo.toml` and run `cargo update -p kitz`.

---

## Next Steps

- [Your First App](your-first-app.md) — build a hello world and a full panel app step by step.
- [Project Structure](project-structure.md) — understand what `kitz new` generates and why.
- [Development Workflow](development-workflow.md) — learn `kitz dev`, testing, and logging.
