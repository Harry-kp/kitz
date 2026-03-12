use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::Context;

/// A test harness for kitz applications.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    // -- Minimal counter app for testing --

    struct Counter {
        count: i32,
    }

    #[derive(Debug, Clone)]
    enum CounterMsg {
        Increment,
        Decrement,
        Reset,
        QuitNow,
    }

    impl Application for Counter {
        type Message = CounterMsg;

        fn update(
            &mut self,
            msg: CounterMsg,
            _ctx: &mut Context<CounterMsg>,
        ) -> Command<CounterMsg> {
            match msg {
                CounterMsg::Increment => self.count += 1,
                CounterMsg::Decrement => self.count -= 1,
                CounterMsg::Reset => self.count = 0,
                CounterMsg::QuitNow => return Command::quit(),
            }
            Command::none()
        }

        fn view(&self, _frame: &mut Frame, _ctx: &ViewContext) {}

        fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<CounterMsg> {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('j') => return EventResult::Message(CounterMsg::Increment),
                    KeyCode::Char('k') => return EventResult::Message(CounterMsg::Decrement),
                    KeyCode::Char('r') => return EventResult::Message(CounterMsg::Reset),
                    _ => {}
                }
            }
            EventResult::Ignored
        }
    }

    #[test]
    fn test_harness_initial_state() {
        let harness = TestHarness::new(Counter { count: 0 });
        assert_eq!(harness.app().count, 0);
        assert!(!harness.quit_requested());
    }

    #[test]
    fn test_harness_press_key_dispatches_message() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        harness.press_key(KeyCode::Char('j'));
        assert_eq!(harness.app().count, 1);

        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('j'));
        assert_eq!(harness.app().count, 3);
    }

    #[test]
    fn test_harness_decrement_and_reset() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('j'));
        harness.press_key(KeyCode::Char('k'));
        assert_eq!(harness.app().count, 1);

        harness.press_key(KeyCode::Char('r'));
        assert_eq!(harness.app().count, 0);
    }

    #[test]
    fn test_harness_send_message_directly() {
        let mut harness = TestHarness::new(Counter { count: 10 });
        harness.send_message(CounterMsg::Decrement);
        assert_eq!(harness.app().count, 9);
    }

    #[test]
    fn test_harness_quit_command() {
        let mut harness = TestHarness::new(Counter { count: 0 });
        assert!(!harness.quit_requested());
        harness.send_message(CounterMsg::QuitNow);
        assert!(harness.quit_requested());
    }

    #[test]
    fn test_harness_unhandled_key_is_ignored() {
        let mut harness = TestHarness::new(Counter { count: 5 });
        harness.press_key(KeyCode::Char('x')); // not handled
        assert_eq!(harness.app().count, 5);
    }

    // -- Panel app for panel_handle_key testing --

    struct PanelApp {
        sidebar_selected: usize,
    }

    #[derive(Debug, Clone)]
    enum PanelMsg {
        SidebarDown,
        SidebarUp,
    }

    impl Application for PanelApp {
        type Message = PanelMsg;

        fn update(&mut self, msg: PanelMsg, _ctx: &mut Context<PanelMsg>) -> Command<PanelMsg> {
            match msg {
                PanelMsg::SidebarDown => self.sidebar_selected += 1,
                PanelMsg::SidebarUp => {
                    self.sidebar_selected = self.sidebar_selected.saturating_sub(1)
                }
            }
            Command::none()
        }

        fn view(&self, _frame: &mut Frame, _ctx: &ViewContext) {}

        fn panels(&self) -> PanelLayout {
            PanelLayout::horizontal(vec![
                ("sidebar", ratatui::layout::Constraint::Percentage(30)),
                ("main", ratatui::layout::Constraint::Percentage(70)),
            ])
        }

        fn panel_handle_key(
            &mut self,
            panel: PanelId,
            key: &crossterm::event::KeyEvent,
        ) -> EventResult<PanelMsg> {
            if panel == "sidebar" {
                match key.code {
                    KeyCode::Char('j') => return EventResult::Message(PanelMsg::SidebarDown),
                    KeyCode::Char('k') => return EventResult::Message(PanelMsg::SidebarUp),
                    _ => {}
                }
            }
            EventResult::Ignored
        }
    }

    #[test]
    fn test_harness_panel_key() {
        let mut harness = TestHarness::new(PanelApp {
            sidebar_selected: 0,
        });
        harness.press_panel_key("sidebar", KeyCode::Char('j'));
        assert_eq!(harness.app().sidebar_selected, 1);

        harness.press_panel_key("sidebar", KeyCode::Char('j'));
        harness.press_panel_key("sidebar", KeyCode::Char('k'));
        assert_eq!(harness.app().sidebar_selected, 1);
    }

    // -- Command::message re-dispatch test --

    struct ChainApp {
        log: Vec<String>,
    }

    #[derive(Debug, Clone)]
    enum ChainMsg {
        Start,
        Step(String),
    }

    impl Application for ChainApp {
        type Message = ChainMsg;

        fn update(&mut self, msg: ChainMsg, _ctx: &mut Context<ChainMsg>) -> Command<ChainMsg> {
            match msg {
                ChainMsg::Start => {
                    self.log.push("start".into());
                    Command::message(ChainMsg::Step("chained".into()))
                }
                ChainMsg::Step(s) => {
                    self.log.push(s);
                    Command::none()
                }
            }
        }

        fn view(&self, _frame: &mut Frame, _ctx: &ViewContext) {}
    }

    #[test]
    fn test_harness_message_chain() {
        let mut harness = TestHarness::new(ChainApp { log: vec![] });
        harness.send_message(ChainMsg::Start);
        assert_eq!(harness.app().log, vec!["start", "chained"]);
    }

    #[test]
    fn test_harness_batch_commands() {
        let mut harness = TestHarness::new(ChainApp { log: vec![] });
        harness.send_message(ChainMsg::Start);
        harness.send_message(ChainMsg::Step("extra".into()));
        assert_eq!(harness.app().log, vec!["start", "chained", "extra"]);
    }
}
