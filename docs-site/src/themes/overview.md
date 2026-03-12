# Themes

Kitz uses a `Theme` struct with semantic color fields to style every part of the UI consistently. Panels, overlays, the footer, toasts, and all built-in widgets read from the current theme. By changing the theme, you change the entire application's appearance in one step.

## The Theme struct

```rust
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub text_muted: Color,
    pub border: Color,
    pub border_focused: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}
```

### Field semantics

| Field | Purpose |
|---|---|
| `name` | Human-readable name (e.g., "Nord", "Dracula") |
| `bg` | Primary background color |
| `surface` | Elevated surface color (toast backgrounds, selected rows) |
| `text` | Primary text color |
| `text_muted` | Secondary/dimmed text (descriptions, inactive hints) |
| `border` | Default panel border color (unfocused) |
| `border_focused` | Focused panel border color |
| `accent` | Highlight color (selected items, active elements, overlay borders) |
| `success` | Success indicators and success-level toasts |
| `warning` | Warning indicators and warning-level toasts |
| `error` | Error indicators and error-level toasts |

## How themes flow through the application

The theme is provided by the `Application::theme()` method:

```rust
fn theme(&self) -> Theme {
    Theme::default() // Returns Nord
}
```

The runtime calls `theme()` every frame and passes the result to:

- **Panel borders** -- `theme.border` for unfocused, `theme.border_focused` for focused panels.
- **Footer widget** -- accent color for panel key hints, muted text for global hints.
- **Overlay rendering** -- accent color for borders and highlights, text colors for content.
- **Toast rendering** -- `theme.accent` for info, `theme.success` for success, `theme.warning` for warning, `theme.error` for error toasts. Toast backgrounds use `theme.surface`.
- **Help overlay** -- accent for section headers, bold text for keys, muted text for descriptions.
- **Command palette** -- accent for the search prompt and selected entry, muted for display-only entries.

Your own `panel_view()` implementations receive the theme indirectly through the application state. A common pattern is to store the current theme in your application struct:

```rust
struct App {
    theme: Theme,
    // ...
}

impl Application for App {
    type Message = Msg;

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        let style = Style::default()
            .fg(self.theme.text)
            .bg(self.theme.bg);
        frame.render_widget(Paragraph::new("Hello").style(style), area);
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::CycleTheme => {
                self.theme = self.theme.next();
            }
            _ => {}
        }
        Command::none()
    }
}
```

## Runtime theme cycling

The `Theme::next()` method cycles through all built-in themes in order:

```rust
pub fn next(&self) -> Self
```

It matches the current theme by name, advances to the next one in the list, and wraps around. This makes it trivial to add a "cycle theme" key binding.

## Default theme

`Theme::default()` returns the Nord theme. This is a comfortable dark palette with blue-gray tones that works well in most terminals.

## Using themes in custom widgets

If you build custom widgets or overlays, accept a `&Theme` parameter so they integrate with the rest of the application:

```rust
fn render_status_bar(frame: &mut Frame, area: Rect, theme: &Theme) {
    let style = Style::default().fg(theme.text_muted).bg(theme.surface);
    frame.render_widget(Paragraph::new("Ready").style(style), area);
}
```

See [Built-in Themes](built-in-themes.md) for the full color palettes and [Custom Themes](custom-themes.md) for how to define your own.
