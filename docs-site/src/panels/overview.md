# Panels

Panels are the primary building block for structuring a Kitz application's UI. A panel is a named, bordered region of the terminal that receives focus, handles key events, and reports its own key hints. The runtime manages rendering, focus indicators, and layout splitting automatically.

## Why panels exist

In a typical ratatui application, layout code sprawls quickly. You manually call `Layout::default().split(area)`, track which region is "focused," wire up key routing to the right handler, and draw borders with the correct style depending on focus state. This boilerplate repeats in every application and is a common source of bugs.

Kitz panels eliminate this ceremony. You declare a layout, give each panel an ID, and implement a small set of trait methods. The runtime handles the rest:

- Splitting the terminal area according to your layout and constraints.
- Drawing titled borders with distinct styles for focused vs. unfocused panels.
- Routing key events to the focused panel.
- Rendering an auto-generated footer from each panel's key hints.
- Wrapping each panel in an error boundary so a panic in one panel does not crash the application.

## The panel contract

Panels are defined through methods on the `Application` trait. There is no separate `Panel` trait to implement. Instead, you return a `PanelLayout` from `panels()` and match on `PanelId` in the other methods:

| Method | Purpose |
|---|---|
| `panels()` | Return the layout describing how panels are arranged |
| `panel_title(id)` | Title string displayed in the panel's border |
| `panel_view(id, frame, area, focused)` | Render the panel's content into the given area |
| `panel_key_hints(id)` | Key hints shown in the footer and help overlay |
| `panel_handle_key(id, key)` | Handle a key event for the focused panel |
| `panel_on_focus(id)` | Called when a panel gains focus |
| `panel_on_blur(id)` | Called when a panel loses focus |

Every method except `panels()` receives a `PanelId` (`&'static str`) so you can distinguish between panels with a simple `match`.

## A simple example

```rust
use kitz::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

const SIDEBAR: PanelId = "sidebar";
const MAIN: PanelId = "main";

struct App {
    items: Vec<String>,
    selected: usize,
}

#[derive(Debug)]
enum Msg {
    SelectNext,
    SelectPrev,
}

impl Application for App {
    type Message = Msg;

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            (SIDEBAR, Constraint::Percentage(30)),
            (MAIN, Constraint::Percentage(70)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            SIDEBAR => "Items",
            MAIN => "Detail",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        match id {
            SIDEBAR => {
                let text: String = self
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        if i == self.selected { format!("> {item}\n") }
                        else { format!("  {item}\n") }
                    })
                    .collect();
                frame.render_widget(Paragraph::new(text), area);
            }
            MAIN => {
                let detail = self.items.get(self.selected)
                    .map(|s| s.as_str())
                    .unwrap_or("Nothing selected");
                frame.render_widget(Paragraph::new(detail), area);
            }
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            SIDEBAR => vec![
                KeyHint::new("j", "Next"),
                KeyHint::new("k", "Prev"),
            ],
            _ => vec![],
        }
    }

    fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        match (id, key.code) {
            (SIDEBAR, KeyCode::Char('j')) => EventResult::Message(Msg::SelectNext),
            (SIDEBAR, KeyCode::Char('k')) => EventResult::Message(Msg::SelectPrev),
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SelectNext => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                }
            }
            Msg::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        Command::none()
    }
}
```

This produces a two-panel layout with a 30/70 split. The sidebar panel lists items with j/k navigation. The detail panel shows the selected item. Focus, borders, footer hints, and the help overlay all work automatically.

## Panels vs. custom `view()`

If `panels()` returns `PanelLayout::None` (the default), the runtime falls back to calling `view()` directly, giving you full control over rendering. Use this escape hatch for splash screens, login flows, or any UI that does not fit a panel grid.

Most applications benefit from the panel path. It reduces boilerplate, gives you focus management and error boundaries for free, and keeps the UI consistent as the application grows.
