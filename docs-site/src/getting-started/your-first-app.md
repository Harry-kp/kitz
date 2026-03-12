# Your First App

This page walks through two tutorials. The first is a minimal hello world that shows the bare mechanics of the `Application` trait. The second is a full panel-based app with a sidebar, a detail pane, a confirm dialog, and toast notifications — built the convention way so you see everything the framework provides for free.

---

## Tutorial 1: Hello World (Custom Path)

This is the smallest possible kitz application. It renders a single line of text and quits on any key press.

### Setup

Create a new Cargo project (or use `kitz new` and replace the generated code):

```
cargo new hello-kitz
cd hello-kitz
```

Set your `Cargo.toml`:

```toml
[package]
name = "hello-kitz"
version = "0.1.0"
edition = "2021"

[dependencies]
kitz = { version = "0.1", default-features = false }
ratatui = "0.30"
crossterm = "0.29"
color-eyre = "0.6"
```

### The Code

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

Run it:

```
cargo run
```

You should see "Hello, kitz! Press any key to quit." rendered in your terminal. Press any key and the application exits cleanly.

### What Just Happened

Let's walk through each piece.

**`type Message = ()`** — Every kitz app has a message type. Messages are the only way to change state. Since this app has no meaningful state, we use the unit type.

**`update`** — Called whenever a message arrives. We receive `()` and return `Command::quit()` to tell the runtime to shut down.

**`view`** — Called every frame. We get a `Frame` (from ratatui) and render a `Paragraph` into the full terminal area. This is the custom path: you have complete control over what gets drawn.

**`handle_event`** — Called for every terminal event (key press, mouse event, resize). We return `EventResult::Message(())` for every event, which dispatches the unit message to `update`, which quits. Returning `EventResult::Ignored` would let the framework handle the event (convention keys like `q` to quit).

**`kitz::run(App)`** — Initializes the terminal (alternate screen, raw mode, mouse capture), enters the event loop, and guarantees terminal restoration on exit — even if your code panics.

---

## Tutorial 2: Panel App with Overlays and Toasts (Convention Path)

This tutorial builds a contacts app with a sidebar listing names and a detail pane showing the selected contact. It demonstrates the convention path: panels, focus management, the help overlay, the confirm dialog, and toast notifications.

### Setup

The fastest way:

```
kitz new contacts && cd contacts
```

Or create a Cargo project manually with the same dependencies as Tutorial 1.

### The Code

We will structure this as a single file for clarity. In a real project, you would split panels into separate modules (as `kitz new` does).

Replace `src/main.rs` with:

```rust
use kitz::prelude::*;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

// -- State -------------------------------------------------------------------

struct App {
    contacts: Vec<Contact>,
    selected: usize,
}

struct Contact {
    name: String,
    email: String,
    notes: String,
}

// -- Messages ----------------------------------------------------------------

#[derive(Debug, Clone)]
enum Msg {
    SelectNext,
    SelectPrev,
    ConfirmDelete,
    DeleteConfirmed,
}

// -- Application trait -------------------------------------------------------

impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
        match msg {
            Msg::SelectNext => {
                if self.selected < self.contacts.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            Msg::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
            }
            Msg::ConfirmDelete => {
                if !self.contacts.is_empty() {
                    let name = self.contacts[self.selected].name.clone();
                    ctx.push_overlay(ConfirmOverlay::new(
                        format!("Delete {}?", name),
                        "This action cannot be undone.",
                        Msg::DeleteConfirmed,
                    ));
                }
            }
            Msg::DeleteConfirmed => {
                if !self.contacts.is_empty() {
                    let removed = self.contacts.remove(self.selected);
                    if self.selected >= self.contacts.len() && self.selected > 0 {
                        self.selected -= 1;
                    }
                    ctx.toast(
                        format!("Deleted {}", removed.name),
                        ToastLevel::Warning,
                    );
                }
            }
        }
        Command::none()
    }

    // -- Panel layout --------------------------------------------------------

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("detail", Constraint::Percentage(70)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Contacts",
            "detail" => "Details",
            _ => "",
        }
    }

    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool) {
        match id {
            "sidebar" => self.render_sidebar(frame, area, focused),
            "detail" => self.render_detail(frame, area),
            _ => {}
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => vec![
                KeyHint::new("j/k", "Navigate"),
                KeyHint::new("d", "Delete"),
            ],
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<Msg> {
        if id == "sidebar" {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    return EventResult::Message(Msg::SelectNext);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    return EventResult::Message(Msg::SelectPrev);
                }
                KeyCode::Char('d') => {
                    return EventResult::Message(Msg::ConfirmDelete);
                }
                _ => {}
            }
        }
        EventResult::Ignored
    }

    fn title(&self) -> &str {
        "Contacts"
    }
}

// -- Rendering ---------------------------------------------------------------

impl App {
    fn new() -> Self {
        Self {
            contacts: vec![
                Contact {
                    name: "Alice Chen".into(),
                    email: "alice@example.com".into(),
                    notes: "Met at RustConf 2025.".into(),
                },
                Contact {
                    name: "Bob Nakamura".into(),
                    email: "bob@example.com".into(),
                    notes: "Collaborating on the TUI toolkit.".into(),
                },
                Contact {
                    name: "Carol Rivera".into(),
                    email: "carol@example.com".into(),
                    notes: "Organizes the local Rust meetup.".into(),
                },
            ],
            selected: 0,
        }
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect, _focused: bool) {
        let items: Vec<ListItem> = self
            .contacts
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                ListItem::new(Line::styled(format!(" {} ", c.name), style))
            })
            .collect();
        frame.render_widget(List::new(items), area);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        if self.contacts.is_empty() {
            frame.render_widget(Paragraph::new(" No contacts."), area);
            return;
        }
        let contact = &self.contacts[self.selected];
        let text = vec![
            Line::raw(format!(" Name:  {}", contact.name)),
            Line::raw(format!(" Email: {}", contact.email)),
            Line::raw(""),
            Line::raw(format!(" {}", contact.notes)),
        ];
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }),
            area,
        );
    }
}

// -- Entry point -------------------------------------------------------------

fn main() -> color_eyre::Result<()> {
    kitz::run(App::new())
}
```

### Run It

```
cargo run
```

### What You Get for Free

Without writing any extra code, the convention path gives you all of these key bindings:

| Key | Action |
|-----|--------|
| **Tab** | Move focus to the next panel |
| **Shift+Tab** | Move focus to the previous panel |
| **z** | Toggle zoom on the focused panel (full-screen / restore) |
| **?** | Open the help overlay (built from all `panel_key_hints` plus globals) |
| **:** | Open the command palette (fuzzy search over every registered action) |
| **Esc** | Pop the current overlay, or pop the current screen, or quit |
| **q** | Quit the application |
| **Ctrl+C** | Hard quit (always works, even if an overlay is open) |
| **Mouse click** | Click a panel to focus it |

On top of that, the framework renders:

- **Bordered panels** with titles, highlighted when focused.
- **A footer bar** at the bottom showing the key hints for the currently focused panel.
- **Toast notifications** that appear in the bottom-right corner and auto-dismiss after a few seconds.
- **The confirm overlay** centered on screen with Yes/No navigation.

### Walking Through the Key Methods

**`panels()`** — Returns a `PanelLayout::horizontal` with two panels. The framework uses this to compute rects, render borders, manage focus order, and generate the help overlay. If you return `PanelLayout::None`, the framework falls back to calling `view()` (the custom path).

**`panel_title(id)`** — Returns the display title for each panel. Rendered in the top border.

**`panel_view(id, frame, area, focused)`** — Called once per panel per frame. You render inside the `area` rect. The `focused` flag lets you highlight the active panel differently if you want (the border is already highlighted by the framework).

**`panel_key_hints(id)`** — Returns a list of `KeyHint { key, desc }` pairs for a panel. These appear in the footer when the panel is focused, and in the help overlay and command palette at all times.

**`panel_handle_key(id, key)`** — Called when a key event reaches the focused panel. Return `EventResult::Message(msg)` to dispatch a message, `EventResult::Consumed` to swallow the event, or `EventResult::Ignored` to let it fall through to the framework's convention keys.

**`update(msg, ctx)`** — The heart of your application. Matches on the message, mutates state, and uses the `Context` to push overlays or show toasts. Returns a `Command` for side-effects. In this example, `Msg::ConfirmDelete` pushes a `ConfirmOverlay` that dispatches `Msg::DeleteConfirmed` when the user selects "Yes". The overlay handles its own rendering, key input, and dismissal — you just specify the question and the message to send on confirmation.

---

## Next Steps

- [Project Structure](project-structure.md) — understand the multi-file layout that `kitz new` generates.
- [Development Workflow](development-workflow.md) — use `kitz dev` for auto-reload and `kitz test` for testing.
- [Architecture](../core-concepts/architecture.md) — deeper dive into the TEA runtime, commands, and the event pipeline.
