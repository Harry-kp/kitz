use std::fmt::Debug;
use std::time::Duration;

use crossterm::event::Event;
use ratatui::Frame;

use crate::command::Command;
use crate::context::{Context, EventContext, ViewContext};

/// Result of processing a terminal event.
///
/// Returned by [`Application::handle_event`] to tell the runtime what to do
/// with the event.
pub enum EventResult<M> {
    /// Convert the event into a message and dispatch it to [`Application::update`].
    Message(M),
    /// The event was not handled — fall through to focused panel, then
    /// convention keys.
    Ignored,
    /// The event was fully handled — do not process further.
    Consumed,
}

/// The core trait every rataframe application implements.
///
/// Only [`update`](Application::update) is required. Every other method has a
/// sensible default so you can progressively opt in to more framework features.
pub trait Application: Sized + 'static {
    /// The message type that drives state transitions via TEA.
    type Message: Debug + Send + 'static;

    // === Required ============================================================

    /// Handle a message and return a [`Command`] describing side-effects.
    fn update(
        &mut self,
        msg: Self::Message,
        ctx: &mut Context<Self::Message>,
    ) -> Command<Self::Message>;

    // === Rendering (escape hatch: override for custom layouts) ===============

    /// Render the UI. The default implementation calls
    /// `ctx.render_panels(frame)` which renders the panel layout returned by
    /// [`panels`](Application::panels).
    ///
    /// Override this for full rendering control (the "custom path").
    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        // Default: render centered placeholder when no panels are defined.
        // Once the panel system lands (Phase 3) this will delegate to
        // ctx.render_panels(frame).
        use ratatui::widgets::Paragraph;
        frame.render_widget(
            Paragraph::new("rataframe app — override view() or implement panels()"),
            frame.area(),
        );
    }

    // === Event handling ======================================================

    /// Map a raw terminal event to a message.
    ///
    /// Returning [`EventResult::Ignored`] lets the event fall through to the
    /// focused panel's `handle_key`, then to the framework's convention keys
    /// (q, Tab, Esc, …).
    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<Self::Message> {
        EventResult::Ignored
    }

    // === Lifecycle ===========================================================

    /// Called once after the terminal is initialised. Return a [`Command`] to
    /// perform startup side-effects (e.g. fetch initial data).
    fn init(&self) -> Command<Self::Message> {
        Command::none()
    }

    // === Metadata ============================================================

    /// Application title — shown in the terminal window title.
    fn title(&self) -> &str {
        "rataframe app"
    }

    /// How often the runtime fires a tick event (used for animations,
    /// polling, etc.). The tick is built-in; you do not need a subscription.
    fn tick_rate(&self) -> Duration {
        Duration::from_millis(250)
    }
}
