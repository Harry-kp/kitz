# Getting Started with kitz

kitz is an application framework for [ratatui](https://ratatui.rs).
It handles the terminal lifecycle, event loop, panel layout, overlays, toasts,
and convention key-bindings so you can focus on your app's state and views.

This guide takes you from zero to a working two-panel app in three steps.

---

## Step 1 — Project Setup

The fastest way to start is with the kitz CLI:

```bash
cargo install kitz
kitz new my-tui
cd my-tui
cargo run
```

This scaffolds a fully working two-panel app with sidebar, detail view, and
all framework conventions wired up.

**Alternative (manual setup):** If you prefer to set up a project yourself,
add kitz to your `Cargo.toml`:

```toml
[dependencies]
kitz = { version = "0.1", default-features = false }
```

kitz re-exports `ratatui`, `crossterm`, and `color_eyre` through its
prelude — you don't need to add them yourself.

---

## Step 2 — Hello World

Replace `src/main.rs` with:

```rust
use kitz::prelude::*;

struct App;

impl Application for App {
    type Message = ();

    fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
        Command::quit()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        frame.render_widget(
            Paragraph::new("Hello, kitz! Press any key to quit."),
            frame.area(),
        );
    }

    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<()> {
        EventResult::Message(())
    }
}

fn main() -> Result<()> {
    kitz::run(App)
}
```

`cargo run` and you'll see the message. Press any key to quit.

**What's happening:**

- `use kitz::prelude::*` imports `Application`, `Command`, `Context`,
  `Frame`, `Paragraph`, `Event`, `EventResult`, `Result`, and everything else
  you'll need.
- `Application` is the core trait. Only `update()` is required — `view()`,
  `handle_event()`, and everything else have defaults.
- `kitz::run(App)` initialises the terminal, enters the event loop, and
  restores the terminal on exit (even on panic).
- `handle_event` maps raw terminal events to messages. Returning
  `EventResult::Message(())` dispatches `()` to `update()`, which returns
  `Command::quit()` to shut down.

This is the **custom path** — you own the full `view()`. For most apps you'll
want the **convention path** instead.

---

## Step 3 — Panel App

The convention path: return a `PanelLayout` from `panels()` and implement
per-panel methods. The framework renders borders, focus indicators, a footer
bar, a help overlay, and a command palette — all for free.

Here's a sidebar + detail app with navigation, a confirm dialog, and toasts:

```rust
use kitz::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

// -- State -------------------------------------------------------------------

struct App {
    items: Vec<String>,
    selected: usize,
}

impl App {
    fn new() -> Self {
        Self {
            items: vec!["Alpha".into(), "Bravo".into(), "Charlie".into()],
            selected: 0,
        }
    }
}

// -- Messages ----------------------------------------------------------------

#[derive(Debug)]
enum Msg {
    Next,
    Prev,
    ConfirmDelete,
    Delete,
}

// -- Application trait -------------------------------------------------------

impl Application for App {
    type Message = Msg;

    // 1. Layout: two horizontal panels
    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(35)),
            ("detail", Constraint::Percentage(65)),
        ])
    }

    // 2. Titles shown in panel borders
    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Items",
            "detail" => "Detail",
            _ => "",
        }
    }

    // 3. Render each panel's content
    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {
        match id {
            "sidebar" => {
                let items: Vec<ListItem> = self
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let style = if i == self.selected {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        ListItem::new(Line::styled(format!(" {s}"), style))
                    })
                    .collect();
                frame.render_widget(List::new(items), area);
            }
            "detail" => {
                let text = self
                    .items
                    .get(self.selected)
                    .map(|s| format!("  Selected: {s}"))
                    .unwrap_or_else(|| "  (empty)".into());
                frame.render_widget(Paragraph::new(text), area);
            }
            _ => {}
        }
    }

    // 4. Key hints — shown in the footer and the ? help overlay
    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => vec![
                KeyHint::new("j/k", "Navigate"),
                KeyHint::new("d", "Delete"),
            ],
            _ => vec![],
        }
    }

    // 5. Per-panel key handling
    fn panel_handle_key(&mut self, id: PanelId, key: &KeyEvent) -> EventResult<Msg> {
        if id == "sidebar" {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return EventResult::Message(Msg::Next),
                KeyCode::Char('k') | KeyCode::Up => return EventResult::Message(Msg::Prev),
                KeyCode::Char('d') => return EventResult::Message(Msg::ConfirmDelete),
                _ => {}
            }
        }
        EventResult::Ignored
    }

    // 6. State transitions — the only required method
    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::Next => {
                if !self.items.is_empty() {
                    self.selected = (self.selected + 1).min(self.items.len() - 1);
                }
            }
            Msg::Prev => {
                self.selected = self.selected.saturating_sub(1);
            }
            Msg::ConfirmDelete => {
                if let Some(name) = self.items.get(self.selected) {
                    ctx.push_overlay(ConfirmOverlay::new(
                        "Delete",
                        format!("Delete '{name}'?"),
                        Msg::Delete,
                    ));
                }
            }
            Msg::Delete => {
                if !self.items.is_empty() {
                    let removed = self.items.remove(self.selected);
                    if self.selected >= self.items.len() && self.selected > 0 {
                        self.selected -= 1;
                    }
                    ctx.toast(format!("Deleted '{removed}'"), ToastLevel::Info);
                }
            }
        }
        Command::none()
    }
}

fn main() -> Result<()> {
    kitz::run(App::new())
}
```

Run it:

```bash
cargo run
```

**What you get without writing any extra code:**

| Key | Behaviour |
|---|---|
| `Tab` / `Shift+Tab` | Cycle focus between panels |
| `?` | Help overlay (built from your `panel_key_hints`) |
| `:` | Fuzzy command palette (same source) |
| `z` | Zoom the focused panel full-screen |
| `Esc` | Back chain: dismiss overlay → pop screen → quit |
| `q` | Quit |
| `Ctrl+C` | Hard quit (always works, even during overlays) |

Borders highlight the focused panel. The footer shows key hints for the
focused panel. All of this comes from the convention path — you only wrote
domain logic.

---

## Architecture at a Glance

kitz follows **TEA (The Elm Architecture)**:

```
Terminal Event
    │
    ▼
handle_event() / panel_handle_key()    ← map events to messages
    │
    ▼
update(msg, ctx)                        ← pure state transition
    │
    ├──▶ Command  (side-effects for the runtime)
    │       • Command::none()           – no-op
    │       • Command::quit()           – shut down
    │       • Command::message(msg)     – re-dispatch immediately
    │       • Command::batch([...])     – combine multiple commands
    │       • Command::perform(task, f) – spawn background work
    │
    └──▶ Context  (intents processed after update returns)
            • ctx.push_overlay(...)     – modal dialog
            • ctx.pop_overlay()
            • ctx.focus_panel(id)       – move focus
            • ctx.toggle_zoom()
            • ctx.toast(msg, level)     – notification
            • ctx.push_screen(...)      – navigation stack
            • ctx.pop_screen()
```

Your app never touches the terminal directly. It returns **values** (`Command`)
and records **intents** (`Context`) — the runtime does the rest.

---

## Two Paths

| | Custom path | Convention path |
|---|---|---|
| **You implement** | `view()` | `panels()` + `panel_*` methods |
| **You get** | Full control over rendering | Borders, focus, footer, help, command palette, zoom |
| **Best for** | Single-screen tools, splash screens | Multi-panel dashboards, CRUD apps |

Start with the convention path. Drop down to `view()` only when you need pixel-perfect control.

---

## Next Steps

- Browse the [examples/](../examples/) directory — `counter.rs`, `todo.rs`,
  `dashboard.rs`, `editor.rs`, and `async_fetch.rs` cover progressively more
  features.
- Read `DESIGN.md` for the full rationale behind the framework's decisions.
- Check the API docs: `cargo doc --open`.
