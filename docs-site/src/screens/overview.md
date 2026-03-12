# Screens

A screen is a distinct "page" in your application with its own panel layout, titles, key hints, and event handling. While panels divide the terminal into regions within a single view, screens represent entirely different views that the user navigates between.

## The Screen trait

```rust
pub trait Screen<M: Debug + Send + 'static> {
    fn id(&self) -> &str;
    fn panels(&self) -> PanelLayout;
    fn panel_title(&self, id: PanelId) -> &str;
    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool);
    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> { vec![] }
    fn panel_handle_key(&mut self, _id: PanelId, _key: &KeyEvent) -> EventResult<M> {
        EventResult::Ignored
    }
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
}
```

A `Screen` mirrors the panel-related methods of the `Application` trait, plus lifecycle hooks:

| Method | Purpose |
|---|---|
| `id()` | Unique identifier for this screen |
| `panels()` | The panel layout for this screen |
| `panel_title(id)` | Title for a panel on this screen |
| `panel_view(id, frame, area, focused)` | Render a panel's content |
| `panel_key_hints(id)` | Key hints for a panel on this screen |
| `panel_handle_key(id, key)` | Handle a key event for the focused panel |
| `on_enter()` | Called when pushed onto the navigation stack |
| `on_leave()` | Called when popped off the navigation stack |

## When to use screens vs. panels

Use **panels** when the regions are part of the same logical view and should be visible simultaneously. A dashboard with a sidebar, a chart area, and a status bar is a single screen with multiple panels.

Use **screens** when you need entirely different views that the user moves between. A file manager that opens a detail page, a settings page nested inside a main view, or a wizard flow with multiple steps -- these are separate screens.

The key distinction: panels coexist, screens replace each other.

| Scenario | Use |
|---|---|
| Sidebar + main content | Panels (horizontal layout) |
| Editor + terminal + file tree | Panels (nested layout) |
| Main view to settings page | Screens (push settings, Esc to go back) |
| List view to detail view | Screens (push detail, Esc to go back) |
| Multi-step form wizard | Screens (push each step) |

## Screens and the Application trait

When a screen is on the navigation stack, the runtime delegates panel methods to the topmost screen instead of the `Application`. This means:

- `Screen::panels()` replaces `Application::panels()` for layout.
- `Screen::panel_view()` replaces `Application::panel_view()` for rendering.
- `Screen::panel_handle_key()` replaces `Application::panel_handle_key()` for input.

The `Application::handle_event()` method is still called first (it always has priority), so global key bindings continue to work regardless of which screen is active.

## A minimal screen

```rust
struct SettingsScreen {
    options: Vec<String>,
    selected: usize,
}

impl Screen<Msg> for SettingsScreen {
    fn id(&self) -> &str {
        "settings"
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::single("settings_panel")
    }

    fn panel_title(&self, _id: PanelId) -> &str {
        "Settings"
    }

    fn panel_view(&self, _id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        let items: String = self
            .options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                if i == self.selected { format!("> {opt}\n") }
                else { format!("  {opt}\n") }
            })
            .collect();
        frame.render_widget(Paragraph::new(items), area);
    }

    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        vec![
            KeyHint::new("j/k", "Navigate"),
            KeyHint::new("Enter", "Toggle"),
        ]
    }

    fn on_enter(&mut self) {
        self.selected = 0;
    }
}
```

Push this screen from your `update()`:

```rust
Msg::OpenSettings => {
    ctx.push_screen(SettingsScreen {
        options: vec!["Dark mode".into(), "Notifications".into()],
        selected: 0,
    });
}
```

The user presses `Esc` to return to the previous view. See [Navigation Stack](navigation-stack.md) for details on the push/pop lifecycle.
