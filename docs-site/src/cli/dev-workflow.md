# Dev Workflow

The kitz CLI provides three commands for the development cycle: `kitz dev` for auto-reloading development, `kitz run` for one-shot builds, and `kitz test` for running your test suite.

## `kitz dev`

Starts the application with automatic rebuilding and restarting on file changes. Under the hood, this uses `cargo-watch`:

```bash
kitz dev
```

If `cargo-watch` is not installed, the CLI installs it automatically before starting. The underlying command is:

```bash
cargo watch -x run -c
```

The `-c` flag clears the terminal between restarts, giving you a clean view each time the app relaunches.

### When to Use `kitz dev`

Use `kitz dev` during active development. Every time you save a `.rs` file, the project recompiles and the application restarts. This gives you a tight feedback loop without manually stopping and restarting the process.

### Stopping Dev Mode

Press `Ctrl+C` to stop the watcher and return to your shell.

## `kitz run`

Builds and runs the application once, without file watching:

```bash
kitz run
```

For release builds with optimizations:

```bash
kitz run --release
```

This is equivalent to `cargo run` and `cargo run --release` respectively. Use `kitz run` when you want to test the application without the overhead of the file watcher, or when doing a final check before distributing.

## `kitz test`

Runs the project's test suite:

```bash
kitz test
```

This executes `cargo test` and reports the results. Kitz tests typically use `TestHarness` to verify application logic without a real terminal.

### Watch Mode

For continuous test execution during development:

```bash
kitz test --watch
```

This uses `cargo-watch` (installed automatically if missing) to re-run the test suite whenever a file changes. The underlying command is:

```bash
cargo watch -x test -c
```

Watch mode is useful when writing tests alongside new features. Save your code, and the tests run within seconds.

## Recommended Development Setup

A productive kitz development setup uses two terminals side by side:

**Terminal 1** -- Running the application:
```bash
kitz dev
```

**Terminal 2** -- Tailing logs (if logging is enabled):
```bash
tail -f ~/.local/share/kitz/my-app/app.log
```

Alternatively, use one terminal for auto-reloading tests:

**Terminal 1** -- Writing code in your editor.

**Terminal 2** -- Running tests continuously:
```bash
kitz test --watch
```

## Command Summary

| Command | What it does | Underlying tool |
|---|---|---|
| `kitz dev` | Build + run with auto-reload on changes | `cargo watch -x run -c` |
| `kitz run` | Build + run once | `cargo run` |
| `kitz run --release` | Build + run once with optimizations | `cargo run --release` |
| `kitz test` | Run the test suite once | `cargo test` |
| `kitz test --watch` | Run tests on every file change | `cargo watch -x test -c` |
