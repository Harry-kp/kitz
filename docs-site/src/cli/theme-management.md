# Theme Management

The kitz CLI includes commands for browsing and previewing the built-in color themes without writing any code. This helps you choose a theme before wiring it into your application.

## `kitz theme list`

Displays all built-in themes with their color swatches rendered using ANSI true-color escape sequences:

```bash
kitz theme list
```

For each theme, you see the theme name, a brief description, and color swatches for every semantic color slot:

- **bg** -- Background color
- **surface** -- Elevated surface color (panel interiors, overlays)
- **border** -- Unfocused panel border color
- **text** -- Primary text color
- **muted** -- Secondary/dimmed text color
- **accent** -- Focus indicators, highlights, active elements
- **success** -- Success states and confirmations
- **warning** -- Warning indicators
- **error** -- Error states and destructive actions

Each swatch is rendered as a small colored block in your terminal, so you see the actual colors rather than hex codes.

## `kitz theme preview`

Renders a sample UI panel in each theme, showing how text, borders, accents, and status colors look together:

```bash
kitz theme preview
```

The preview draws a bordered panel with:

- The theme name as the title (in the accent color)
- Normal text
- Muted secondary text
- An accent/focus element
- Success, warning, and error status indicators

This gives you a realistic impression of how each theme will look in a running application.

## Built-in Themes

Kitz ships with four themes:

| Theme | Description |
|---|---|
| **Nord** | Calm, arctic-inspired palette with cool blues and muted tones |
| **Tokyo Night** | Vibrant dark theme inspired by Tokyo city lights |
| **Catppuccin Mocha** | Warm, cozy, pastel dark theme |
| **Dracula** | Iconic dark theme with vivid highlights |

The default theme is Nord.

## Using a Theme in Your Application

After choosing a theme from the CLI previews, set it in your application by overriding the `theme()` method:

```rust
use kitz::prelude::*;
use kitz::theme::palettes;

impl Application for App {
    type Message = Msg;

    fn theme(&self) -> Theme {
        palettes::tokyo_night()
    }

    // ...
}
```

Available palette functions:

- `palettes::nord()`
- `palettes::tokyo_night()`
- `palettes::catppuccin_mocha()`
- `palettes::dracula()`

## Dynamic Theme Switching

You can store the theme in your application state and switch it at runtime:

```rust
struct App {
    theme: Theme,
    // ...
}

#[derive(Debug, Clone)]
enum Msg {
    CycleTheme,
    // ...
}

impl Application for App {
    type Message = Msg;

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::CycleTheme => {
                self.theme = self.theme.next();
                Command::none()
            }
            // ...
        }
    }
}
```

The `Theme::next()` method cycles through all built-in palettes in order.

## Terminal Requirements

The color swatches and previews use ANSI true-color (24-bit) escape sequences. They display correctly in modern terminals such as iTerm2, WezTerm, Alacritty, Kitty, Windows Terminal, and most recent versions of GNOME Terminal. If your terminal does not support true-color, the swatches may appear as approximated colors.
