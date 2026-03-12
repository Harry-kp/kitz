# kitz new

The `kitz new` command scaffolds a complete kitz project with working code, proper dependencies, and a ready-to-run structure.

## Usage

```bash
kitz new <name> [--template <template>]
```

- **`<name>`** -- The project name. This becomes the directory name, the Cargo package name (snake_case), and the binary name.
- **`--template`** (or `-t`) -- The project template to use. Defaults to `panels`.

## Templates

Four templates are available, each targeting a different starting point:

### `panels` (default)

A multi-panel application with a sidebar and main content area. Includes panel focus, key hints, the auto-footer, and the help overlay. This is the recommended starting point for most applications.

```bash
kitz new my-app
kitz new my-app --template panels
```

### `minimal`

The simplest possible kitz application. A single `view()` override with no panels. Good for learning the framework basics or for applications that do not need the panel system.

```bash
kitz new my-app --template minimal
```

### `dashboard`

A multi-panel layout designed for monitoring or status display. Includes a horizontal split with multiple information panels and sample data rendering.

```bash
kitz new my-app --template dashboard
```

### `editor`

A text-editor-style layout with a file tree sidebar, main editing area, and status bar. Demonstrates nested panel layouts, input handling, and mode switching.

```bash
kitz new my-app --template editor
```

## What Gets Generated

Regardless of template, the generated project includes:

```
my-app/
  Cargo.toml          # Dependencies: kitz, ratatui, crossterm, color-eyre
  src/
    main.rs            # Entry point with kitz::run()
    app.rs             # Application struct implementing the Application trait
    messages.rs        # Message enum with marker comments for code generation
    panels/            # Panel modules (for panels/dashboard/editor templates)
      mod.rs
      sidebar.rs
      main_panel.rs
  tests/
    app_test.rs        # TestHarness-based test scaffold
```

The exact files vary by template. The `minimal` template omits the `panels/` directory and `messages.rs`, keeping everything in a single `main.rs`.

## Post-Scaffolding Steps

After generating the project, `kitz new` automatically:

1. **Initializes a git repository** -- Runs `git init` in the new directory. If git is not installed, this step is skipped with a warning.
2. **Runs `cargo check`** -- Verifies that the generated project compiles. This catches dependency resolution issues early.

## Marker Comments

The generated code contains marker comments like `// kitz:panel-mods`, `// kitz:messages`, `// kitz:update`, and so on. These markers are used by `kitz generate` to know where to insert new code. Do not remove them if you plan to use the generator. If you prefer to manage code manually, the markers are plain comments and can be safely deleted.

## Examples

Create a project and start coding immediately:

```bash
kitz new todo-app
cd todo-app
cargo run
```

Create a dashboard project:

```bash
kitz new system-monitor --template dashboard
cd system-monitor
kitz dev
```
