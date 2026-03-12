# Footer

The footer is a single-row widget rendered at the bottom of the terminal when a panel layout is active. It displays key hints from the focused panel alongside global convention keys, giving the user a persistent reference for available actions.

## How it works

The runtime renders the footer automatically. You do not create or position it yourself. The process is:

1. The runtime splits the terminal into two vertical regions: the main area (`Constraint::Min(1)`) and a footer row (`Constraint::Length(1)`).
2. It calls `panel_key_hints(focused_id)` on the focused panel to get the panel-specific hints.
3. It combines those with a fixed set of global hints.
4. It renders the `Footer` widget into the bottom row.

### Global hints

The footer always includes these global hints on the right side:

| Key | Description |
|---|---|
| `Tab` | Switch panel |
| `z` | Zoom |
| `?` | Help |
| `q` | Quit |

These are hardcoded in the `Footer` widget and cannot be removed. They represent the convention keys that are always available.

### Layout

The footer renders in a single line with this structure:

```
[panel hints...]  |  [global hints...]
```

Panel-specific hints appear on the left, formatted in the theme's accent color with bold text. A vertical bar separator (`|`) divides them from the global hints, which are formatted in the theme's muted text color with bold text.

Each hint is rendered as `key description` with a space between hints.

## Providing key hints

Key hints come from `panel_key_hints()` on your `Application` (or `Screen`) trait:

```rust
fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "editor" => vec![
            KeyHint::new("Ctrl+s", "Save"),
            KeyHint::new("Ctrl+z", "Undo"),
            KeyHint::new("Ctrl+f", "Find"),
        ],
        "files" => vec![
            KeyHint::new("Enter", "Open"),
            KeyHint::new("d", "Delete"),
            KeyHint::new("n", "New file"),
        ],
        _ => vec![],
    }
}
```

The `KeyHint` struct has two fields:

```rust
pub struct KeyHint {
    pub key: &'static str,
    pub desc: &'static str,
}
```

The footer updates immediately when focus changes, because the runtime calls `panel_key_hints` with the current focused panel ID each frame.

## Customization

### Dynamic hints

Since `panel_key_hints` is called every frame, you can return different hints based on application state:

```rust
fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
    match id {
        "editor" => {
            let mut hints = vec![
                KeyHint::new("Ctrl+s", "Save"),
            ];
            if self.has_selection {
                hints.push(KeyHint::new("Ctrl+c", "Copy"));
                hints.push(KeyHint::new("Ctrl+x", "Cut"));
            }
            if self.can_undo {
                hints.push(KeyHint::new("Ctrl+z", "Undo"));
            }
            hints
        }
        _ => vec![],
    }
}
```

### Hints flow to other systems

Key hints are not just for the footer. The same `panel_key_hints` return values are used to:

- Populate the **Help overlay** (opened with `?`), grouped by panel title.
- Populate the **Command palette** (opened with `:`), as display-only entries.

This means a single source of truth -- your `panel_key_hints` implementation -- feeds three different discovery mechanisms.

## When the footer is hidden

The footer is only rendered when a panel layout is active (`panels()` returns something other than `PanelLayout::None`). If you use the custom `view()` path without panels, no footer is shown. You can render your own status bar in that case using the full terminal area.

When a panel is zoomed, the footer still appears. It shows the zoomed panel's hints.
