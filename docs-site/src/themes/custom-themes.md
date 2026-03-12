# Custom Themes

If the built-in themes do not match your application's identity, you can create a custom `Theme` by constructing the struct directly. Every field is public, so no builder or macro is needed.

## Creating a custom theme

```rust
use kitz::theme::Theme;
use ratatui::style::Color;

fn solarized_dark() -> Theme {
    Theme {
        name: "Solarized Dark",
        bg: Color::Rgb(0, 43, 54),
        surface: Color::Rgb(7, 54, 66),
        text: Color::Rgb(131, 148, 150),
        text_muted: Color::Rgb(88, 110, 117),
        border: Color::Rgb(88, 110, 117),
        border_focused: Color::Rgb(38, 139, 210),
        accent: Color::Rgb(38, 139, 210),
        success: Color::Rgb(133, 153, 0),
        warning: Color::Rgb(181, 137, 0),
        error: Color::Rgb(220, 50, 47),
    }
}
```

## Using a custom theme

Return it from `Application::theme()`:

```rust
struct App {
    theme: Theme,
}

impl Application for App {
    type Message = Msg;

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        Command::none()
    }
}
```

Initialize the application with the custom theme:

```rust
let app = App {
    theme: solarized_dark(),
};
kitz::run(app)?;
```

## Integrating with built-in themes

If you want your custom theme to participate in the `next()` cycling alongside the built-in themes, you can manage the cycle yourself:

```rust
fn all_themes() -> Vec<Theme> {
    let mut themes = kitz::theme::palettes::all();
    themes.push(solarized_dark());
    themes.push(gruvbox());
    themes
}

fn next_theme(current: &Theme) -> Theme {
    let themes = all_themes();
    let idx = themes
        .iter()
        .position(|t| t.name == current.name)
        .map(|i| (i + 1) % themes.len())
        .unwrap_or(0);
    themes.into_iter().nth(idx).unwrap()
}
```

Then in your update handler:

```rust
Msg::CycleTheme => {
    self.theme = next_theme(&self.theme);
}
```

## Design guidelines

### Use semantic roles, not visual names

The theme fields are semantic: `accent`, `success`, `warning`, `error`. Map your palette to these roles based on meaning, not just what looks good in isolation. The accent color is used for focused borders, selected items, overlay highlights, and the command palette prompt. Choose a color that works in all of those contexts.

### Test contrast

Terminal emulators vary. Test your theme in at least two or three different terminals (e.g., iTerm2, Alacritty, Windows Terminal) to verify that text is legible and borders are visible.

### Keep `bg` and `surface` close

`surface` is used for elevated elements like toast backgrounds and selected rows. It should be distinguishable from `bg` but not jarring. A 10-20 point difference in lightness works well.

### Ensure `border` and `border_focused` are distinct

The focused panel border must be visually distinguishable from unfocused borders at a glance. A common approach is to use the muted text color for `border` and the accent color for `border_focused`.

## Color types

Kitz themes use ratatui's `Color` enum. The most common variants for custom themes:

```rust
Color::Rgb(r, g, b)   // True color (24-bit)
Color::Indexed(n)      // 256-color palette
Color::Red             // Named ANSI color
Color::Reset           // Terminal default
```

`Color::Rgb` provides the best results on modern terminals. For maximum compatibility with older terminals, use `Color::Indexed` or named ANSI colors, though the visual consistency will be reduced.
