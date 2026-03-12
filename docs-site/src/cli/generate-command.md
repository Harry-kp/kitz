# kitz generate

The `kitz generate` command creates a new component and wires it into your existing project. It writes the component file, updates module declarations, adds message variants, and inserts the necessary match arms into your application code.

## Usage

```bash
kitz generate panel <name>
kitz generate screen <name>
kitz generate overlay <name>
```

The `<name>` must be snake_case: lowercase letters, digits, and underscores. It cannot start with an underscore or digit.

## Generating a Panel

```bash
kitz generate panel stats
```

This creates and modifies the following files:

| File | Change |
|---|---|
| `src/panels/stats.rs` | New file: `StatsPanel` struct with `view()`, `key_hints()`, `select_next()`, `select_prev()` |
| `src/panels/mod.rs` | Adds `pub mod stats;` |
| `src/messages.rs` | Adds `StatsNext` and `StatsPrev` variants to the message enum |
| `src/app.rs` | Adds the panel field, initializer, update match arms, layout entry, panel title, view, hints, and key handler |
| `tests/app_test.rs` | Adds a placeholder test function |

The generated panel includes a list with items, arrow-key navigation, and themed rendering. It compiles and works immediately -- you can run the app and see the new panel alongside existing ones.

### Generated Panel Code

The panel file follows a standard structure:

```rust
use kitz::prelude::*;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};

pub const PANEL_ID: PanelId = "stats";
pub const PANEL_TITLE: &str = "Stats";

pub struct StatsPanel {
    pub items: Vec<String>,
    pub selected: usize,
}

impl StatsPanel {
    pub fn new() -> Self {
        Self {
            items: vec!["Item 1".into(), "Item 2".into(), "Item 3".into()],
            selected: 0,
        }
    }

    pub fn view(
        &self,
        frame: &mut Frame,
        area: Rect,
        _focused: bool,
        theme: &kitz::theme::Theme,
    ) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::styled(format!(" {} ", item), style))
            })
            .collect();
        frame.render_widget(List::new(items), area);
    }

    pub fn key_hints() -> Vec<KeyHint> {
        vec![KeyHint::new("j/k", "Navigate")]
    }

    pub fn select_next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}
```

## Generating a Screen

```bash
kitz generate screen settings
```

Creates a screen component that can be pushed onto the navigation stack:

| File | Change |
|---|---|
| `src/screens/settings.rs` | New file: `SettingsScreen` implementing the `Screen` trait |
| `src/screens/mod.rs` | Adds `pub mod settings;` |
| `src/messages.rs` | Adds screen-related message variants |
| `src/app.rs` | Adds the navigation trigger in update match arms |

Push the screen from your update logic:

```rust
Msg::OpenSettings => {
    ctx.push_screen(screens::settings::SettingsScreen::new());
    Command::none()
}
```

## Generating an Overlay

```bash
kitz generate overlay confirm_delete
```

Creates a custom overlay component:

| File | Change |
|---|---|
| `src/overlays/confirm_delete.rs` | New file: struct implementing the `Overlay` trait |
| `src/overlays/mod.rs` | Adds `pub mod confirm_delete;` |
| `src/messages.rs` | Adds overlay-related message variants |
| `src/app.rs` | Adds the overlay trigger in update match arms |

Push the overlay from your update logic:

```rust
Msg::AskDelete => {
    ctx.push_overlay(overlays::confirm_delete::ConfirmDeleteOverlay::new());
    Command::none()
}
```

## How Wiring Works: Marker Comments

The generator locates insertion points using marker comments in your source files. These are plain Rust comments with a `kitz:` prefix:

```rust
// kitz:panel-mods     -- in panels/mod.rs
// kitz:messages       -- in messages.rs
// kitz:app-fields     -- in app.rs (struct fields)
// kitz:app-init       -- in app.rs (constructor)
// kitz:update         -- in app.rs (update match arms)
// kitz:layout         -- in app.rs (panel layout)
// kitz:panel-title    -- in app.rs (panel_title match)
// kitz:panel-view     -- in app.rs (panel_view match)
// kitz:panel-hints    -- in app.rs (panel_key_hints match)
// kitz:panel-keys     -- in app.rs (panel_handle_key match)
// kitz:tests          -- in tests/app_test.rs
```

New code is inserted above the marker comment, preserving the marker for future generations. If a marker is missing (because you deleted it or restructured your code), the generator falls back to appending to the file or prints a warning.

## Layout Rebalancing

When a new panel is added, the generator recalculates `Constraint::Percentage` values to distribute space evenly across all panels. If you have custom percentage splits, you may want to adjust them after generation.

## Running After Generation

The generated code compiles immediately. Run the project to see the new component:

```bash
cargo run
```

Or use the dev workflow for auto-reload:

```bash
kitz dev
```
