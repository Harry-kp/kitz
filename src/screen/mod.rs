use std::fmt::Debug;

use crossterm::event::KeyEvent;

use crate::app::EventResult;
use crate::panel::{KeyHint, PanelId, PanelLayout};
use ratatui::layout::Rect;
use ratatui::Frame;

/// A screen is a distinct "page" in your application with its own panel
/// layout. Screens form a navigation stack: push to go deeper, pop (Esc)
/// to go back.
pub trait Screen<M: Debug + Send + 'static> {
    /// Unique identifier for this screen.
    fn id(&self) -> &str;

    /// The panel layout for this screen.
    fn panels(&self) -> PanelLayout;

    /// Title for a panel on this screen.
    fn panel_title(&self, id: PanelId) -> &str;

    /// Render a panel's content.
    fn panel_view(&self, id: PanelId, frame: &mut Frame, area: Rect, focused: bool);

    /// Key hints for a panel on this screen.
    fn panel_key_hints(&self, _id: PanelId) -> Vec<KeyHint> {
        vec![]
    }

    /// Handle a key event for the focused panel on this screen.
    fn panel_handle_key(&mut self, _id: PanelId, _key: &KeyEvent) -> EventResult<M> {
        EventResult::Ignored
    }

    /// Called when this screen is pushed onto the stack.
    fn on_enter(&mut self) {}

    /// Called when this screen is popped off the stack.
    fn on_leave(&mut self) {}
}

/// A stack of screens. The topmost screen drives the current layout.
pub struct NavigationStack<M: Debug + Send + 'static> {
    screens: Vec<Box<dyn Screen<M>>>,
}

impl<M: Debug + Send + 'static> NavigationStack<M> {
    /// Create an empty navigation stack.
    pub fn new() -> Self {
        Self {
            screens: Vec::new(),
        }
    }

    /// Push a screen onto the stack, calling its `on_enter` hook.
    pub fn push(&mut self, mut screen: Box<dyn Screen<M>>) {
        screen.on_enter();
        self.screens.push(screen);
    }

    /// Pop the top screen, calling its `on_leave` hook.
    pub fn pop(&mut self) -> Option<Box<dyn Screen<M>>> {
        if let Some(mut screen) = self.screens.pop() {
            screen.on_leave();
            Some(screen)
        } else {
            None
        }
    }

    /// Borrow the topmost screen, if any.
    pub fn top(&self) -> Option<&dyn Screen<M>> {
        self.screens.last().map(|s| s.as_ref())
    }

    /// Mutably borrow the topmost screen, if any.
    pub fn top_mut(&mut self) -> Option<&mut Box<dyn Screen<M>>> {
        self.screens.last_mut()
    }

    /// Number of screens on the stack.
    pub fn depth(&self) -> usize {
        self.screens.len()
    }

    /// Returns `true` if no screens are on the stack.
    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }
}

impl<M: Debug + Send + 'static> Default for NavigationStack<M> {
    fn default() -> Self {
        Self::new()
    }
}
