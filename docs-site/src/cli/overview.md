# CLI Overview

The `kitz` command-line tool accelerates development by scaffolding projects, generating components, running builds, and previewing themes. It is a standalone binary that wraps common Cargo workflows with kitz-specific conventions.

## Installation

Install the CLI from crates.io:

```bash
cargo install kitz
```

This installs the `kitz` binary into your Cargo bin directory. Verify the installation:

```bash
kitz --version
```

## Commands at a Glance

| Command | Description |
|---|---|
| `kitz new <name>` | Create a new kitz project from a template |
| `kitz generate panel <name>` | Generate a panel and wire it into the project |
| `kitz generate screen <name>` | Generate a screen and wire it into the project |
| `kitz generate overlay <name>` | Generate an overlay and wire it into the project |
| `kitz dev` | Start development with auto-reload on file changes |
| `kitz run` | Build and run the application |
| `kitz test` | Run the project's tests |
| `kitz theme list` | Display all built-in themes with ANSI color swatches |
| `kitz theme preview` | Render a sample UI panel in each theme |

## Typical Workflow

A common development session looks like this:

```bash
# Create a new project
kitz new my-app --template panels

# Enter the project
cd my-app

# Start auto-reloading dev mode
kitz dev

# (In another terminal) Add a new panel
kitz generate panel stats

# Run tests
kitz test

# Preview available themes
kitz theme list
```

## Getting Help

Every command supports `--help`:

```bash
kitz --help
kitz new --help
kitz generate --help
kitz generate panel --help
kitz theme --help
```

The following pages cover each command in detail:

- [kitz new](new-command.md) -- Project scaffolding
- [kitz generate](generate-command.md) -- Component generation
- [Dev Workflow](dev-workflow.md) -- Running, testing, and auto-reload
- [Theme Management](theme-management.md) -- Browsing and previewing themes
