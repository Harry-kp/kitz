# Built-in Overlays

Kitz provides three overlays out of the box. They cover the most common modal interactions in TUI applications and are available through the prelude.

## ConfirmOverlay

A centered confirmation dialog with a message and Yes/No buttons.

### Constructor

```rust
pub fn new(
    title: impl Into<String>,
    message: impl Into<String>,
    on_confirm: M,
) -> Self
```

The `on_confirm` message is dispatched to `Application::update()` only if the user selects "Yes" and presses Enter. If the user cancels, the overlay closes without sending a message.

### Key bindings

| Key | Action |
|---|---|
| `Tab`, `h`, `l`, Left, Right | Toggle between Yes and No |
| `Enter`, `y` | Confirm the selected option |
| `Esc`, `n` | Cancel and close |

The selection defaults to "No" for safety. The user must explicitly move to "Yes" before confirming.

### Usage

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::RequestQuit => {
            ctx.push_overlay(ConfirmOverlay::new(
                "Quit",
                "You have unsaved changes. Quit anyway?",
                Msg::ForceQuit,
            ));
        }
        Msg::ForceQuit => {
            return Command::quit();
        }
        _ => {}
    }
    Command::none()
}
```

The overlay renders a bordered box at 50% width and 30% height of the terminal, centered. The title appears in the border. The message is centered inside, with Yes/No buttons below it. The selected button is highlighted with the theme's accent color, bold and underlined.

## HelpOverlay

A scrollable help screen that displays all registered key hints, grouped by panel. It is opened automatically when the user presses `?`.

### How it is built

The runtime constructs the `HelpOverlay` by collecting `panel_key_hints()` from every panel in the current layout, plus a "Global" section with the built-in convention keys:

```
── Sidebar ──
         j  Next item
         k  Previous item

── Editor ──
     Ctrl+s  Save
     Ctrl+z  Undo

── Global ──
       Tab  Switch panel
         z  Zoom
         ?  Help
         q  Quit
```

### Constructor

```rust
pub fn new(sections: Vec<(String, Vec<KeyHint>)>) -> Self
```

Each section is a `(title, hints)` pair. You rarely need to construct this manually -- the runtime builds it for you.

### Key bindings

| Key | Action |
|---|---|
| `j`, Down | Scroll down |
| `k`, Up | Scroll up |
| `Esc`, `?`, `q` | Close |

The overlay renders at 60% width and 70% height of the terminal. Keys are right-aligned and bolded, descriptions are muted. Section headers use the accent color.

### Generic over M

`HelpOverlay` implements `Overlay<M>` for any message type `M`. It never sends a message -- it always returns `Close` or `Consumed`. This means it can be pushed onto any application's overlay stack regardless of the message type.

## CommandPaletteOverlay

A fuzzy-searchable command palette, opened with the `:` key. It displays all discoverable actions from the current panel layout and optionally includes actionable commands defined by the application.

### How it is built

The runtime auto-populates the palette with display-only entries derived from every panel's `panel_key_hints()`, formatted as `[Panel Title] Description`. It also includes global convention keys. The result is a searchable index of everything the user can do.

Applications can additionally register actionable commands via `PaletteCommand`:

```rust
pub struct PaletteCommand<M> {
    pub label: String,
    pub key_hint: String,
    pub message: M,
}
```

Actionable commands dispatch a message when selected. Display-only entries simply close the palette (they exist for discoverability).

### Fuzzy search

The palette uses [nucleo-matcher](https://crates.io/crates/nucleo-matcher) for fast, typo-tolerant fuzzy matching. As the user types, entries are scored and sorted by relevance. The matching is case-insensitive with smart Unicode normalization.

For example, typing "svf" would match "Save file" because `s`, `v`, and `f` appear in order in the haystack.

### Key bindings

| Key | Action |
|---|---|
| Any character | Append to the search query |
| `Backspace` | Delete the last character from the query |
| `Down`, `Tab` | Move selection down |
| `Up`, `Shift+Tab` | Move selection up |
| `Enter` | Execute the selected entry |
| `Esc` | Close the palette |

### Rendering

The palette renders at 70% width and 60% height, centered. It has three zones:

1. **Search input** -- a `> ` prompt followed by the query text and a block cursor.
2. **Separator** -- a horizontal rule.
3. **Results list** -- filtered entries with the selected entry marked by `▸` and highlighted in the accent color. Actionable entries appear in the normal text color; display-only entries appear muted. Each entry shows its key hint on the right in dim text.

When no entries match the query, the message "No matching commands" is displayed.

### Adding custom commands

To register actionable commands, construct the palette manually and push it as an overlay:

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::OpenPalette => {
            let commands = vec![
                PaletteCommand {
                    label: "New file".into(),
                    key_hint: "Ctrl+n".into(),
                    message: Msg::NewFile,
                },
                PaletteCommand {
                    label: "Open file".into(),
                    key_hint: "Ctrl+o".into(),
                    message: Msg::OpenFile,
                },
                PaletteCommand {
                    label: "Toggle dark mode".into(),
                    key_hint: "".into(),
                    message: Msg::ToggleTheme,
                },
            ];
            ctx.push_overlay(CommandPaletteOverlay::new(commands));
        }
        _ => {}
    }
    Command::none()
}
```
