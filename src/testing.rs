use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::Context;

/// A test harness for rataframe applications.
///
/// Allows simulating key presses and messages, then asserting on app state.
/// No actual terminal is needed — all rendering is skipped.
///
/// # Example
///
/// ```ignore
/// let mut harness = TestHarness::new(MyApp::new());
/// harness.press_key(KeyCode::Char('j'));
/// assert_eq!(harness.app().selected, 1);
/// ```
pub struct TestHarness<A: Application> {
    app: A,
    quit_requested: bool,
}

impl<A: Application> TestHarness<A> {
    /// Create a new test harness wrapping the given application.
    pub fn new(app: A) -> Self {
        let mut harness = Self {
            app,
            quit_requested: false,
        };
        // Process init command
        let cmd = harness.app.init();
        harness.process_cmd(cmd);
        harness
    }

    /// Access the application state for assertions.
    pub fn app(&self) -> &A {
        &self.app
    }

    /// Mutably access the application state.
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// Whether the app has requested to quit.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// Simulate a key press.
    pub fn press_key(&mut self, code: KeyCode) {
        self.send_key(code, KeyModifiers::NONE);
    }

    /// Simulate a key press with modifiers.
    pub fn send_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let event = Event::Key(KeyEvent::new(code, modifiers));
        let event_ctx = crate::context::EventContext::with_state(None, false);

        match self.app.handle_event(&event, &event_ctx) {
            EventResult::Message(msg) => {
                self.dispatch(msg);
            }
            EventResult::Consumed | EventResult::Ignored => {}
        }
    }

    /// Simulate a key press targeted at a specific panel.
    pub fn press_panel_key(&mut self, panel_id: &'static str, code: KeyCode) {
        let key = KeyEvent::new(code, KeyModifiers::NONE);
        match self.app.panel_handle_key(panel_id, &key) {
            EventResult::Message(msg) => {
                self.dispatch(msg);
            }
            EventResult::Consumed | EventResult::Ignored => {}
        }
    }

    /// Dispatch a message directly to the app's update function.
    pub fn send_message(&mut self, msg: A::Message) {
        self.dispatch(msg);
    }

    fn dispatch(&mut self, msg: A::Message) {
        let mut ctx = Context::new();
        let cmd = self.app.update(msg, &mut ctx);
        self.process_cmd(cmd);
    }

    fn process_cmd(&mut self, cmd: crate::command::Command<A::Message>) {
        let mut pending = cmd.actions;
        while !pending.is_empty() {
            let batch = std::mem::take(&mut pending);
            for action in batch {
                match action {
                    Action::Quit => {
                        self.quit_requested = true;
                        return;
                    }
                    Action::Message(msg) => {
                        let mut ctx = Context::new();
                        let next = self.app.update(msg, &mut ctx);
                        pending.extend(next.actions);
                    }
                    Action::Perform(_) => {
                        // Background tasks are not executed in tests.
                        // Use send_message to simulate their results.
                    }
                }
            }
        }
    }
}
