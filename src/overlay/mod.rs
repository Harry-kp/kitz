pub mod command_palette;
pub mod confirm;
pub mod help;

pub use command_palette::{CommandPaletteOverlay, PaletteCommand};
pub use confirm::ConfirmOverlay;
pub use help::HelpOverlay;

use std::fmt::Debug;

use crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::theme::Theme;

/// Result of an overlay processing an event.
pub enum OverlayResult<M> {
    /// Close the overlay without sending a message.
    Close,
    /// Close the overlay and dispatch this message to `Application::update`.
    CloseWithMessage(M),
    /// The overlay consumed the event — don't propagate.
    Consumed,
    /// The overlay didn't handle it (rare for overlays, but possible).
    Ignored,
}

/// A modal dialog rendered on top of the main UI.
///
/// Overlays capture all input while open. Press Esc to pop (unless the
/// overlay overrides that).
pub trait Overlay<M: Debug + Send + 'static> {
    /// Display title rendered in the overlay border.
    fn title(&self) -> &str;
    /// Render the overlay content within the given area.
    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    /// Process an input event, returning how it was handled.
    fn handle_event(&mut self, event: &Event) -> OverlayResult<M>;
}

/// A stack of overlays. The topmost overlay receives all input.
pub struct OverlayStack<M: Debug + Send + 'static> {
    stack: Vec<Box<dyn Overlay<M>>>,
}

impl<M: Debug + Send + 'static> OverlayStack<M> {
    /// Create an empty overlay stack.
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push an overlay onto the stack.
    pub fn push(&mut self, overlay: Box<dyn Overlay<M>>) {
        self.stack.push(overlay);
    }

    /// Pop the topmost overlay.
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    /// Borrow the topmost overlay, if any.
    pub fn top(&self) -> Option<&dyn Overlay<M>> {
        self.stack.last().map(|o| o.as_ref())
    }

    /// Mutably borrow the topmost overlay, if any.
    pub fn top_mut(&mut self) -> Option<&mut Box<dyn Overlay<M>>> {
        self.stack.last_mut()
    }

    /// Returns `true` if no overlays are open.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

impl<M: Debug + Send + 'static> Default for OverlayStack<M> {
    fn default() -> Self {
        Self::new()
    }
}
