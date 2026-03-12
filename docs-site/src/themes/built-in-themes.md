# Built-in Themes

Kitz ships with four dark themes. All are available through `kitz::theme::palettes` and are returned by `palettes::all()`. The default theme is Nord.

## Nord

A calm, blue-gray palette inspired by the [Nord](https://www.nordtheme.com/) color scheme.

```rust
Theme {
    name: "Nord",
    bg:             Color::Rgb(46, 52, 64),     // #2E3440
    surface:        Color::Rgb(59, 66, 82),     // #3B4252
    text:           Color::Rgb(216, 222, 233),  // #D8DEE9
    text_muted:     Color::Rgb(76, 86, 106),    // #4C566A
    border:         Color::Rgb(76, 86, 106),    // #4C566A
    border_focused: Color::Rgb(136, 192, 208),  // #88C0D0
    accent:         Color::Rgb(136, 192, 208),  // #88C0D0
    success:        Color::Rgb(163, 190, 140),  // #A3BE8C
    warning:        Color::Rgb(235, 203, 139),  // #EBCB8B
    error:          Color::Rgb(191, 97, 106),   // #BF616A
}
```

| Role | Color | Hex |
|---|---|---|
| Background | Dark blue-gray | `#2E3440` |
| Surface | Slightly lighter blue-gray | `#3B4252` |
| Text | Light gray | `#D8DEE9` |
| Text muted | Medium gray-blue | `#4C566A` |
| Accent | Frost blue | `#88C0D0` |
| Success | Muted green | `#A3BE8C` |
| Warning | Warm yellow | `#EBCB8B` |
| Error | Muted red | `#BF616A` |

## Tokyo Night

A rich, deep-blue palette from the [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) family.

```rust
Theme {
    name: "Tokyo Night",
    bg:             Color::Rgb(26, 27, 38),     // #1A1B26
    surface:        Color::Rgb(36, 40, 59),     // #24283B
    text:           Color::Rgb(169, 177, 214),  // #A9B1D6
    text_muted:     Color::Rgb(86, 95, 137),    // #565F89
    border:         Color::Rgb(86, 95, 137),    // #565F89
    border_focused: Color::Rgb(125, 174, 163),  // #7DAE A3
    accent:         Color::Rgb(125, 174, 163),  // #7DAEA3
    success:        Color::Rgb(158, 206, 106),  // #9ECE6A
    warning:        Color::Rgb(224, 175, 104),  // #E0AF68
    error:          Color::Rgb(247, 118, 142),  // #F7768E
}
```

| Role | Color | Hex |
|---|---|---|
| Background | Very dark blue | `#1A1B26` |
| Surface | Dark indigo | `#24283B` |
| Text | Lavender gray | `#A9B1D6` |
| Text muted | Slate blue | `#565F89` |
| Accent | Teal green | `#7DAEA3` |
| Success | Bright green | `#9ECE6A` |
| Warning | Amber | `#E0AF68` |
| Error | Coral pink | `#F7768E` |

## Catppuccin Mocha

The darkest flavor of [Catppuccin](https://github.com/catppuccin/catppuccin), with pastel accents on a deep base.

```rust
Theme {
    name: "Catppuccin",
    bg:             Color::Rgb(30, 30, 46),     // #1E1E2E
    surface:        Color::Rgb(49, 50, 68),     // #313244
    text:           Color::Rgb(205, 214, 244),  // #CDD6F4
    text_muted:     Color::Rgb(108, 112, 134),  // #6C7086
    border:         Color::Rgb(108, 112, 134),  // #6C7086
    border_focused: Color::Rgb(137, 180, 250),  // #89B4FA
    accent:         Color::Rgb(137, 180, 250),  // #89B4FA
    success:        Color::Rgb(166, 218, 149),  // #A6DA95
    warning:        Color::Rgb(249, 226, 175),  // #F9E2AF
    error:          Color::Rgb(243, 139, 168),  // #F38BA8
}
```

| Role | Color | Hex |
|---|---|---|
| Background | Deep purple-black | `#1E1E2E` |
| Surface | Dark purple-gray | `#313244` |
| Text | Light lavender | `#CDD6F4` |
| Text muted | Gray-purple | `#6C7086` |
| Accent | Pastel blue | `#89B4FA` |
| Success | Pastel green | `#A6DA95` |
| Warning | Pastel yellow | `#F9E2AF` |
| Error | Pastel pink | `#F38BA8` |

## Dracula

The classic [Dracula](https://draculatheme.com/) palette with vivid, high-contrast accents.

```rust
Theme {
    name: "Dracula",
    bg:             Color::Rgb(40, 42, 54),     // #282A36
    surface:        Color::Rgb(68, 71, 90),     // #44475A
    text:           Color::Rgb(248, 248, 242),  // #F8F8F2
    text_muted:     Color::Rgb(98, 114, 164),   // #6272A4
    border:         Color::Rgb(98, 114, 164),   // #6272A4
    border_focused: Color::Rgb(139, 233, 253),  // #8BE9FD
    accent:         Color::Rgb(139, 233, 253),  // #8BE9FD
    success:        Color::Rgb(80, 250, 123),   // #50FA7B
    warning:        Color::Rgb(241, 250, 140),  // #F1FA8C
    error:          Color::Rgb(255, 85, 85),    // #FF5555
}
```

| Role | Color | Hex |
|---|---|---|
| Background | Dark charcoal | `#282A36` |
| Surface | Medium gray-purple | `#44475A` |
| Text | Near-white | `#F8F8F2` |
| Text muted | Blue-gray | `#6272A4` |
| Accent | Cyan | `#8BE9FD` |
| Success | Bright green | `#50FA7B` |
| Warning | Bright yellow | `#F1FA8C` |
| Error | Bright red | `#FF5555` |

## Cycling themes at runtime

Store the theme in your application state and use `theme.next()` to cycle:

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
        match msg {
            Msg::NextTheme => {
                self.theme = self.theme.next();
            }
            _ => {}
        }
        Command::none()
    }

    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        vec![KeyHint::new("t", "Cycle theme")]
    }

    fn panel_handle_key(&mut self, _id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        match key.code {
            KeyCode::Char('t') => EventResult::Message(Msg::NextTheme),
            _ => EventResult::Ignored,
        }
    }
}
```

`next()` cycles through Nord, Tokyo Night, Catppuccin, and Dracula in order, wrapping back to Nord after Dracula. The change takes effect on the very next frame since `theme()` is called every render cycle.

## Accessing all themes

To get a `Vec<Theme>` of every built-in theme:

```rust
use kitz::theme::palettes;

let all_themes = palettes::all();
// [Nord, Tokyo Night, Catppuccin, Dracula]
```

You can use this to build a theme picker overlay or to select a theme by index.
