# rataframe

**The application framework for [ratatui](https://github.com/ratatui/ratatui).**

> Panels, overlays, themes, and TEA architecture — with convention-driven productivity.

---

> **Status: Under active development.** The framework is being built phase by phase. See [`docs/DESIGN.md`](docs/DESIGN.md) for the full architectural plan and [`docs/DECISIONS.md`](docs/DECISIONS.md) for the design decision log.

## What is rataframe?

rataframe is to ratatui what Next.js is to React. It provides:

- **Terminal lifecycle** — init, restore, panic safety. You never think about it.
- **TEA architecture** — Application trait, Messages, Commands. Proven by Elm and Iced.
- **Convention-driven panels** — implement `Panel`, get auto-footer, auto-help, command palette, focus cycling, and themed borders for free.
- **Overlays, toasts, themes, navigation** — built-in, composable, opt-in.
- **10 lines to hello world. Same trait scales to 500-line dashboards.**

## Quick Start

```rust
use rataframe::prelude::*;

struct App;

impl Application for App {
    type Message = ();
    fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
        Command::quit()
    }
    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        frame.render_widget(
            Paragraph::new("Hello, rataframe! Press any key to quit."),
            frame.area(),
        );
    }
    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<()> {
        EventResult::Message(())
    }
}

fn main() -> Result<()> {
    rataframe::run(App)
}
```

## License

MIT
