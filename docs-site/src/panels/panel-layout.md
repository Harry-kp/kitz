# Panel Layout

`PanelLayout` describes how panels are arranged within the main area of the terminal. The runtime uses this layout to compute a `Rect` for each panel, draw borders, and route focus. Layouts are returned from `Application::panels()` (or `Screen::panels()` when using the screen stack).

## The enum

```rust
pub enum PanelLayout {
    None,
    Single(PanelId),
    Horizontal(Vec<(PanelId, Constraint)>),
    Vertical(Vec<(PanelId, Constraint)>),
    Nested(Direction, Vec<(Box<PanelLayout>, Constraint)>),
}
```

### None

No panel layout. The runtime calls `Application::view()` instead, giving you full rendering control. This is the default.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::None
}
```

### Single

A single panel filling the entire main area.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::single("editor")
}
```

The panel gets the full width and height (minus the footer row). Focus management still works -- Tab and Shift+Tab are no-ops with one panel, but zoom (`z`) still toggles the border title indicator.

### Horizontal

Two or more panels arranged side by side, left to right. Each panel is paired with a `Constraint` controlling its width.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::horizontal(vec![
        ("sidebar", Constraint::Percentage(25)),
        ("content", Constraint::Percentage(50)),
        ("preview", Constraint::Percentage(25)),
    ])
}
```

### Vertical

Two or more panels stacked top to bottom. Each panel is paired with a `Constraint` controlling its height.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::vertical(vec![
        ("toolbar", Constraint::Length(3)),
        ("editor", Constraint::Min(1)),
        ("output", Constraint::Length(10)),
    ])
}
```

### Nested

A recursive layout tree for complex multi-directional arrangements. The outer `Direction` controls how the top-level children are split. Each child is itself a `PanelLayout`, so you can nest horizontal splits inside vertical splits and vice versa.

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::nested(
        Direction::Horizontal,
        vec![
            (
                PanelLayout::single("sidebar"),
                Constraint::Percentage(25),
            ),
            (
                PanelLayout::vertical(vec![
                    ("editor", Constraint::Min(1)),
                    ("terminal", Constraint::Length(12)),
                ]),
                Constraint::Percentage(75),
            ),
        ],
    )
}
```

This produces a sidebar on the left (25% width) and a vertical split on the right (75% width) containing an editor on top and a terminal on the bottom.

## Constraints

Constraints come from ratatui's `ratatui::layout::Constraint` enum. The most commonly used variants in panel layouts:

| Constraint | Behavior |
|---|---|
| `Constraint::Percentage(n)` | Allocate `n`% of the available space |
| `Constraint::Length(n)` | Allocate exactly `n` rows or columns |
| `Constraint::Min(n)` | At least `n`, but expand to fill remaining space |
| `Constraint::Max(n)` | At most `n`, but shrink if space is limited |
| `Constraint::Ratio(num, den)` | Allocate `num/den` of the available space |

Common patterns:

```rust
// Fixed sidebar + flexible main
vec![
    ("sidebar", Constraint::Length(30)),
    ("main", Constraint::Min(1)),
]

// Equal thirds
vec![
    ("a", Constraint::Ratio(1, 3)),
    ("b", Constraint::Ratio(1, 3)),
    ("c", Constraint::Ratio(1, 3)),
]

// Fixed header/footer with flexible body
vec![
    ("header", Constraint::Length(3)),
    ("body", Constraint::Min(1)),
    ("status", Constraint::Length(1)),
]
```

## A complex nested layout

Here is an IDE-style layout with a file tree, editor, and split bottom pane:

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::nested(
        Direction::Horizontal,
        vec![
            // Left column: file tree
            (
                PanelLayout::single("files"),
                Constraint::Length(28),
            ),
            // Right column: editor + bottom split
            (
                PanelLayout::nested(
                    Direction::Vertical,
                    vec![
                        // Top: editor
                        (
                            PanelLayout::single("editor"),
                            Constraint::Min(1),
                        ),
                        // Bottom: terminal + problems side-by-side
                        (
                            PanelLayout::horizontal(vec![
                                ("terminal", Constraint::Percentage(50)),
                                ("problems", Constraint::Percentage(50)),
                            ]),
                            Constraint::Length(14),
                        ),
                    ],
                ),
                Constraint::Min(1),
            ),
        ],
    )
}
```

This yields four panels:

```
+----------+-------------------------------+
|          |           editor              |
|  files   |                               |
|          +---------------+---------------+
|          |   terminal    |   problems    |
+----------+---------------+---------------+
```

## How `compute_rects` works

When the runtime renders a frame, it calls `PanelLayout::compute_rects(area)` to produce a `Vec<(PanelId, Rect)>`. For `Horizontal` and `Vertical` variants, this delegates to ratatui's `Layout::default().direction(...).constraints(...).split(area)`. For `Nested`, it recurses: first splitting the outer area, then passing each child's `Rect` into the child layout's own `compute_rects`.

Panel IDs are collected in display order via `panel_ids()`. This ordering determines the Tab-cycling sequence for focus navigation.

## Dynamic layouts

`panels()` is called every frame, so you can change the layout at any time based on application state. For example, toggling a sidebar:

```rust
fn panels(&self) -> PanelLayout {
    if self.show_sidebar {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Length(30)),
            ("main", Constraint::Min(1)),
        ])
    } else {
        PanelLayout::single("main")
    }
}
```

The runtime's `PanelManager` calls `sync_layout()` each frame to detect changes and preserve focus on the previously focused panel when possible.
