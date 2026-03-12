# Architecture

Kitz is built on **The Elm Architecture** (TEA), a pattern that originated in the Elm programming language and has since been adopted by frameworks across many ecosystems --- Iced in Rust, SwiftUI's unidirectional data flow, and Redux in JavaScript. TEA provides a strict unidirectional data flow that makes state changes predictable, testable, and easy to reason about.

This chapter explains how TEA works in kitz, how events flow through the system, and why this architecture is particularly well-suited to terminal user interfaces.

## The TEA Cycle

Every kitz application revolves around four concepts:

- **Model** -- your application struct, holding all state.
- **Message** -- an enum describing something that happened.
- **Update** -- a function that takes the current model and a message, mutates the model, and returns a command describing side effects.
- **View** -- a function that renders the current model to the terminal.

The cycle looks like this:

```
              ┌──────────────────────────────────────────┐
              │                                          │
              ▼                                          │
        ┌───────────┐    ┌───────────┐    ┌──────────┐  │
        │           │    │           │    │          │  │
        │   Model   │───▶│   View    │───▶│ Terminal │  │
        │           │    │           │    │          │  │
        └───────────┘    └───────────┘    └──────────┘  │
              ▲                                │        │
              │                                │        │
        ┌───────────┐                    ┌──────────┐   │
        │           │                    │          │   │
        │  Update   │◀───── Message ◀────│  Event   │   │
        │           │                    │          │   │
        └───────────┘                    └──────────┘   │
              │                                         │
              │                                         │
              └─── Command (side effects) ──────────────┘
```

1. The **runtime** renders the current model by calling `view()` (or the panel system).
2. The terminal produces an **event** (key press, mouse click, resize).
3. The application maps the event to a **message** via `handle_event()`.
4. The runtime calls `update()` with that message and a mutable `Context`.
5. `update()` mutates the model and returns a **Command** describing any side effects.
6. The runtime executes the command. If it produces a new message, the cycle repeats.

There is no other way for state to change. Every mutation flows through `update()`, every side effect is described by a `Command`, and the view is always a pure function of the current model.

## Event Flow in Detail

When a raw terminal event arrives, it passes through a precise chain of handlers. Understanding this chain is essential for knowing where to put your event-handling logic.

```
Raw terminal event
        │
        ▼
   ┌─────────────────┐
   │  Ctrl+C check   │──── hard quit (always works, even if app is stuck)
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │ Mouse click-to- │──── if click lands on a panel, focus it
   │  focus (panels) │
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │ Active overlay?  │──── overlay.handle_event() consumes all input
   └────────┬────────┘
            │ no overlay
            ▼
   ┌─────────────────┐
   │ App.handle_event │──── returns Message, Consumed, or Ignored
   └────────┬────────┘
            │ Ignored
            ▼
   ┌─────────────────┐
   │  Focused panel   │──── panel_handle_key() for the focused panel
   └────────┬────────┘
            │ Ignored
            ▼
   ┌─────────────────┐
   │ Convention keys  │──── q/Esc quit, Tab/Shift+Tab cycle panels,
   └─────────────────┘      z zoom, ? help, : command palette
```

Each stage can either handle the event (returning `Message` or `Consumed`) or pass it along (returning `Ignored`). This cascade ensures that:

- **Overlays are modal.** When an overlay is open, it captures all input. The app and panels never see the event.
- **App-level bindings take priority** over panel-specific ones, so global shortcuts work regardless of which panel is focused.
- **Panel-specific bindings** only fire when the panel is focused and neither the overlay nor the app claimed the event.
- **Convention keys** are the fallback, providing consistent behavior (quit, navigate, help) that the user never has to wire up manually.

## Commands and Context

The `update()` method has two channels for producing effects:

### Commands

A `Command<M>` is a **value** describing a side effect. It is returned from `update()` and executed by the runtime after `update()` returns. Commands are composable, serializable descriptions of work --- they never execute inline.

```rust
fn update(&mut self, msg: Msg, ctx: &mut Context<Msg>) -> Command<Msg> {
    match msg {
        Msg::Save => Command::perform(
            || save_to_disk(),
            |result| Msg::SaveComplete(result),
        ),
        Msg::Quit => Command::quit(),
        _ => Command::none(),
    }
}
```

Commands express things like "quit the app," "re-dispatch this message," "run this closure on a background thread and send back a message when it's done," or "do all of these things at once."

### Context

`Context<M>` is a mutable reference passed into `update()`. It collects **intents** --- operations that need to mutate framework-owned state that the application does not own directly.

```rust
Msg::ShowHelp => {
    ctx.push_overlay(HelpOverlay::new(sections));
    Command::none()
}
```

Context handles overlay management, panel focus changes, zoom toggling, toast notifications, and screen navigation. These operations cannot be commands because they require immediate, synchronous mutation of framework internals (the overlay stack, the panel manager, the navigation stack) rather than deferred execution.

The split is deliberate:

| Mechanism | When to use | Execution |
|-----------|-------------|-----------|
| `Command` | Quit, async work, re-dispatch messages | Deferred, after `update()` returns |
| `Context` | Overlays, focus, zoom, toasts, screens | Collected during `update()`, applied immediately after |

Both are processed after `update()` returns, but intents from Context are applied first (so an overlay push is visible before a command's message is dispatched).

## Why TEA for TUI Apps

Terminal user interfaces have properties that make TEA an exceptionally good fit:

**State is inherently synchronous.** A terminal redraws the entire screen each frame. There is no partial DOM diffing, no retained widget tree. The view function receives the full model and produces the full frame. TEA's "view is a function of state" maps directly to this reality.

**Events arrive sequentially.** The terminal event loop processes one event at a time. There is no concurrent event handling to worry about. TEA's single `update()` entry point matches this perfectly --- each message is processed in order, and the model is always in a consistent state between messages.

**Testability comes free.** Because `update()` is a pure function from `(Model, Message)` to `(Model, Command)`, you can unit test every state transition without a terminal, without a runtime, and without mocking. Construct a model, send a message, assert on the result.

```rust
#[test]
fn increment_increases_count() {
    let mut app = App { count: 0 };
    let mut ctx = Context::new();
    let _ = app.update(Msg::Increment, &mut ctx);
    assert_eq!(app.count, 1);
}
```

**Side effects are explicit.** In a typical TUI app, it is tempting to spawn threads, write files, or make network calls inline inside an event handler. TEA forces you to describe these effects as commands. The result is code where every side effect is visible at the call site and executed under the runtime's control --- making it straightforward to reason about ordering, cancellation, and error handling.

**Debugging is straightforward.** Because every state change goes through a message, you can log the message stream to get a complete history of what happened and in what order. Reproducing a bug becomes a matter of replaying messages against an initial model.

TEA is not the only viable architecture for TUI apps, but its constraints align remarkably well with the terminal's execution model. Kitz embraces these constraints fully, providing the `Application` trait, the `Command` type, and the `Context` struct as the three pillars of every application built with the framework.
