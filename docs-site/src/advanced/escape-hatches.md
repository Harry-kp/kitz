# Escape Hatches

Kitz's convention path -- panels, auto-footer, help overlay, convention keys -- handles the majority of TUI layouts. But some applications need full control over rendering or event handling. Kitz provides deliberate escape hatches so you can opt out of any convention without fighting the framework.

## Override `view()` for Full Rendering Control

The simplest escape hatch is to override `view()` and skip panels entirely. When `panels()` returns `PanelLayout::None` (the default), the runtime delegates all rendering to your `view()` method:

```rust
impl Application for App {
    type Message = Msg;

    fn update(&mut self, msg: Msg, _ctx: &mut Context<Msg>) -> Command<Msg> {
        // ...
        Command::none()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        let area = frame.area();

        // Full control: lay out whatever you want
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        frame.render_widget(my_header_widget(), chunks[0]);
        frame.render_widget(my_content_widget(), chunks[1]);
        frame.render_widget(my_status_bar(), chunks[2]);
    }
}
```

When using the `view()` path, the runtime does not render panel borders, the auto-footer, or the help overlay. You handle all layout and drawing yourself. The convention keys (`q` to quit, `Tab` to switch panels, `?` for help) still work if you return `EventResult::Ignored` from `handle_event`. To suppress them, return `EventResult::Consumed`.

## Return `EventResult::Consumed` to Suppress Convention Keys

The runtime processes events in a cascade:

1. If an overlay is active, the overlay handles the event.
2. Otherwise, `handle_event()` is called.
3. If `handle_event` returns `Ignored` and panels are active, `panel_handle_key` is called for the focused panel.
4. If the event is still unhandled, convention keys are processed (`q`, `Esc`, `Tab`, `z`, `?`, `:`).

Return `EventResult::Consumed` from `handle_event` to stop the cascade at step 2. The event will not reach panel handlers or convention keys:

```rust
fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<Msg> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('q') => {
                // Override the convention: 'q' does something else here
                return EventResult::Message(Msg::EnterSearchMode);
            }
            KeyCode::Esc => {
                // Suppress Esc from triggering quit
                return EventResult::Consumed;
            }
            _ => {}
        }
    }
    EventResult::Ignored
}
```

This is useful when your application has modes (like a text editor with insert mode) where convention keys should not fire, or when you want `q` to do something other than quit.

## Skip Panels Entirely

If you never implement `panels()`, or explicitly return `PanelLayout::None`, the entire panel system is inactive:

```rust
fn panels(&self) -> PanelLayout {
    PanelLayout::None
}
```

With panels disabled:

- No panel borders are drawn.
- No auto-footer appears.
- `Tab` / `Shift+Tab` do not cycle focus.
- `?` does not open the help overlay.
- `:` does not open the command palette.
- `z` does not toggle zoom.

Your application receives all events through `handle_event()` and renders everything through `view()`. This is the "minimal path" and gives you the same level of control as a raw ratatui application, while still benefiting from kitz's runtime loop, command system, subscriptions, overlays, and toast notifications.

## Mixing Convention and Custom Rendering

You can use panels for most of your UI but still intercept specific events or draw additional elements. For example, render a custom header above the panel area by overriding `view()` while still delegating panel rendering to the framework:

```rust
fn view(&self, frame: &mut Frame, ctx: &ViewContext) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Custom header
    frame.render_widget(
        Paragraph::new("My Custom Header")
            .style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );

    // The rest is handled by panels -- but note that when you override
    // view(), the runtime no longer auto-renders panels. You would need
    // to manually invoke panel rendering or stick to the full custom path.
}
```

In practice, if you need a custom header or footer alongside panels, it is simpler to make the header itself a panel (using `PanelLayout::nested`) than to override `view()`. Reserve the `view()` escape hatch for layouts that genuinely do not fit the panel model.

## Conditional Escape Hatches

You can switch between the convention path and the custom path based on state:

```rust
fn panels(&self) -> PanelLayout {
    if self.fullscreen_mode {
        PanelLayout::None
    } else {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("main", Constraint::Percentage(70)),
        ])
    }
}

fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
    if self.fullscreen_mode {
        // Custom fullscreen rendering
        frame.render_widget(
            Paragraph::new(&*self.fullscreen_content),
            frame.area(),
        );
    }
    // When not in fullscreen mode, the runtime renders panels automatically
    // because panels() returned a layout.
}
```

## When to Use Escape Hatches

Use the `view()` override when:

- Your layout does not map to a set of named rectangular panels.
- You are building a game, animation, or canvas-style application.
- You are migrating an existing ratatui app and want to wrap it incrementally.

Use `EventResult::Consumed` when:

- You have modal input modes where convention keys should not fire.
- A specific key has a domain-specific meaning that conflicts with conventions.

Use `PanelLayout::None` when:

- You want the runtime, commands, subscriptions, overlays, and toasts but not the panel system.
- You are building a simple single-screen application that does not need multiple panels.

The escape hatches exist so that kitz's conventions help when they apply and stay out of the way when they do not. You can always start with the convention path and drop down to escape hatches for the parts of your application that need custom behavior.
