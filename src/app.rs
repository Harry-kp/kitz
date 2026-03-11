use std::fmt::Debug;
use std::time::Duration;

use crossterm::event::{Event, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::command::Command;
use crate::context::{Context, EventContext, ViewContext};
use crate::panel::{KeyHint, PanelId, PanelLayout};
use crate::subscription::Subscription;
use crate::theme::Theme;

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

    // === Rendering (escape hatch: override view for custom layouts) ==========

    /// Render the UI. The default is a placeholder — override this for the
    /// custom path, or use `panels()` for the convention path (the runtime
    /// renders panels automatically when `panels()` returns a layout).
    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        use ratatui::widgets::Paragraph;
        frame.render_widget(
            Paragraph::new("rataframe app — override view() or implement panels()"),
            frame.area(),
        );
    }

    // === Convention path: Panel system =======================================

    /// Return the panel layout. The runtime uses this to render panels with
    /// borders, focus indicators, and the auto-footer.
    fn panels(&self) -> PanelLayout {
        PanelLayout::None
    }

    /// Title for a specific panel — shown in the panel's border.
    fn panel_title(&self, _id: PanelId) -> &str {
        ""
    }

    /// Render a panel's content inside its allocated area.
    fn panel_view(&self, _id: PanelId, _frame: &mut Frame, _area: Rect, _focused: bool) {}

    /// Key hints for a panel — shown in the footer and help overlay.
    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        vec![]
    }

    /// Handle a key event for the focused panel. Called when `handle_event`
    /// returns `Ignored` and a panel layout is active.
    fn panel_handle_key(&mut self, _id: PanelId, _key: &KeyEvent) -> EventResult<Self::Message> {
        EventResult::Ignored
    }

    /// Called when a panel gains focus.
    fn panel_on_focus(&mut self, _id: PanelId) {}

    /// Called when a panel loses focus.
    fn panel_on_blur(&mut self, _id: PanelId) {}

    // === Event handling ======================================================

    /// Map a raw terminal event to a message.
    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<Self::Message> {
        EventResult::Ignored
    }

    // === Lifecycle ===========================================================

    /// Called once after the terminal is initialised.
    fn init(&self) -> Command<Self::Message> {
        Command::none()
    }

    // === Metadata ============================================================

    /// Display title for the terminal window.
    fn title(&self) -> &str {
        "rataframe app"
    }

    /// How often the runtime polls for events.
    fn tick_rate(&self) -> Duration {
        Duration::from_millis(250)
    }

    /// The color theme used for panels, overlays, and toasts.
    fn theme(&self) -> Theme {
        Theme::default()
    }

    /// Declarative subscriptions. The runtime starts/stops background tasks
    /// based on the set returned here each frame.
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        Subscription::none()
    }
}
