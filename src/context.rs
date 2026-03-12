use std::fmt::Debug;

use crate::overlay::Overlay;
use crate::panel::PanelId;
use crate::screen::Screen;
use crate::toast::{Toast, ToastLevel};

/// Runtime intents accumulated during `update()` and processed afterwards.
pub(crate) enum Intent<M: Debug + Send + 'static> {
    PushOverlay(Box<dyn Overlay<M>>),
    PopOverlay,
    FocusPanel(PanelId),
    ToggleZoom,
    ShowToast(Toast),
    PushScreen(Box<dyn Screen<M>>),
    PopScreen,
}

/// Mutable context passed to [`Application::update`](crate::app::Application::update).
///
/// Use this to push overlays, change panel focus, or toggle zoom from within
/// your update logic.
pub struct Context<M: Debug + Send + 'static> {
    pub(crate) intents: Vec<Intent<M>>,
}

impl<M: Debug + Send + 'static> Default for Context<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Debug + Send + 'static> Context<M> {
    /// Create an empty context. Useful in tests.
    pub fn new() -> Self {
        Self {
            intents: Vec::new(),
        }
    }

    /// Number of pending intents (useful in tests).
    pub fn intent_count(&self) -> usize {
        self.intents.len()
    }

    /// Push a modal overlay (confirm dialog, help screen, etc.).
    pub fn push_overlay(&mut self, overlay: impl Overlay<M> + 'static) {
        self.intents.push(Intent::PushOverlay(Box::new(overlay)));
    }

    /// Pop the topmost overlay.
    pub fn pop_overlay(&mut self) {
        self.intents.push(Intent::PopOverlay);
    }

    /// Move focus to a specific panel.
    pub fn focus_panel(&mut self, id: PanelId) {
        self.intents.push(Intent::FocusPanel(id));
    }

    /// Toggle zoom on the focused panel.
    pub fn toggle_zoom(&mut self) {
        self.intents.push(Intent::ToggleZoom);
    }

    /// Show a toast notification.
    pub fn toast(&mut self, message: impl Into<String>, level: ToastLevel) {
        self.intents
            .push(Intent::ShowToast(Toast::new(message, level)));
    }

    /// Push a new screen onto the navigation stack.
    pub fn push_screen(&mut self, screen: impl Screen<M> + 'static) {
        self.intents.push(Intent::PushScreen(Box::new(screen)));
    }

    /// Pop the current screen, returning to the previous one.
    pub fn pop_screen(&mut self) {
        self.intents.push(Intent::PopScreen);
    }
}

/// Read-only context passed to [`Application::view`](crate::app::Application::view).
pub struct ViewContext {
    pub(crate) focused_panel: Option<PanelId>,
    pub(crate) zoomed: bool,
}

impl Default for ViewContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewContext {
    /// Create a default view context with no focus state.
    pub fn new() -> Self {
        Self {
            focused_panel: None,
            zoomed: false,
        }
    }

    /// Create a view context with specific panel focus state.
    pub fn with_panels(focused: Option<PanelId>, zoomed: bool) -> Self {
        Self {
            focused_panel: focused,
            zoomed,
        }
    }

    pub fn focused_panel(&self) -> Option<PanelId> {
        self.focused_panel
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoomed
    }
}

/// Read-only context passed to
/// [`Application::handle_event`](crate::app::Application::handle_event).
pub struct EventContext {
    pub(crate) focused_panel: Option<PanelId>,
    pub(crate) has_overlay: bool,
}

impl Default for EventContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EventContext {
    /// Create a default event context.
    pub fn new() -> Self {
        Self {
            focused_panel: None,
            has_overlay: false,
        }
    }

    /// Create an event context with specific state.
    pub fn with_state(focused: Option<PanelId>, has_overlay: bool) -> Self {
        Self {
            focused_panel: focused,
            has_overlay,
        }
    }

    pub fn focused_panel(&self) -> Option<PanelId> {
        self.focused_panel
    }

    pub fn has_overlay(&self) -> bool {
        self.has_overlay
    }
}
