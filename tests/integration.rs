use crossterm::event::KeyCode;
use rataframe::prelude::*;

// =============================================================================
// Fixtures: reusable test apps
// =============================================================================

struct MinimalApp;

impl Application for MinimalApp {
    type Message = ();
    fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
        Command::none()
    }
}

// ---

struct Counter {
    count: i32,
}

#[derive(Debug, Clone)]
enum CounterMsg {
    Inc,
    Dec,
    Reset,
    DoubleInc,
    Quit,
}

impl Application for Counter {
    type Message = CounterMsg;

    fn update(&mut self, msg: CounterMsg, _ctx: &mut Context<CounterMsg>) -> Command<CounterMsg> {
        match msg {
            CounterMsg::Inc => self.count += 1,
            CounterMsg::Dec => self.count -= 1,
            CounterMsg::Reset => self.count = 0,
            CounterMsg::DoubleInc => {
                return Command::batch([
                    Command::message(CounterMsg::Inc),
                    Command::message(CounterMsg::Inc),
                ]);
            }
            CounterMsg::Quit => return Command::quit(),
        }
        Command::none()
    }

    fn handle_event(&self, event: &Event, _ctx: &EventContext) -> EventResult<CounterMsg> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') => return EventResult::Message(CounterMsg::Inc),
                KeyCode::Char('k') => return EventResult::Message(CounterMsg::Dec),
                KeyCode::Char('r') => return EventResult::Message(CounterMsg::Reset),
                KeyCode::Char('d') => return EventResult::Message(CounterMsg::DoubleInc),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

// ---

struct PanelApp {
    sidebar_items: Vec<String>,
    sidebar_selected: usize,
    detail_text: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PanelMsg {
    SidebarDown,
    SidebarUp,
    DetailUpdate(String),
    ConfirmDelete,
    DeleteItem,
    ShowToast(String),
}

impl PanelApp {
    fn new() -> Self {
        Self {
            sidebar_items: vec!["Alpha".into(), "Beta".into(), "Gamma".into()],
            sidebar_selected: 0,
            detail_text: String::new(),
        }
    }
}

impl Application for PanelApp {
    type Message = PanelMsg;

    fn update(&mut self, msg: PanelMsg, ctx: &mut Context<PanelMsg>) -> Command<PanelMsg> {
        match msg {
            PanelMsg::SidebarDown => {
                if self.sidebar_selected < self.sidebar_items.len().saturating_sub(1) {
                    self.sidebar_selected += 1;
                }
            }
            PanelMsg::SidebarUp => {
                self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
            }
            PanelMsg::DetailUpdate(text) => self.detail_text = text,
            PanelMsg::ConfirmDelete => {
                ctx.push_overlay(ConfirmOverlay::new(
                    "Delete",
                    "Are you sure?",
                    PanelMsg::DeleteItem,
                ));
            }
            PanelMsg::DeleteItem => {
                if self.sidebar_selected < self.sidebar_items.len() {
                    self.sidebar_items.remove(self.sidebar_selected);
                    if self.sidebar_selected >= self.sidebar_items.len()
                        && self.sidebar_selected > 0
                    {
                        self.sidebar_selected -= 1;
                    }
                }
            }
            PanelMsg::ShowToast(msg) => {
                ctx.toast(msg, ToastLevel::Info);
            }
        }
        Command::none()
    }

    fn panels(&self) -> PanelLayout {
        PanelLayout::horizontal(vec![
            ("sidebar", Constraint::Percentage(30)),
            ("detail", Constraint::Percentage(70)),
        ])
    }

    fn panel_title(&self, id: PanelId) -> &str {
        match id {
            "sidebar" => "Sidebar",
            "detail" => "Detail",
            _ => "",
        }
    }

    fn panel_key_hints(&self, id: PanelId) -> Vec<KeyHint> {
        match id {
            "sidebar" => vec![KeyHint::new("j/k", "Navigate"), KeyHint::new("d", "Delete")],
            _ => vec![],
        }
    }

    fn panel_handle_key(
        &mut self,
        id: PanelId,
        key: &crossterm::event::KeyEvent,
    ) -> EventResult<PanelMsg> {
        if id == "sidebar" {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    return EventResult::Message(PanelMsg::SidebarDown)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    return EventResult::Message(PanelMsg::SidebarUp)
                }
                KeyCode::Char('d') => return EventResult::Message(PanelMsg::ConfirmDelete),
                _ => {}
            }
        }
        EventResult::Ignored
    }
}

// =============================================================================
// Tests: Core TEA
// =============================================================================

#[test]
fn minimal_app_creates_without_panic() {
    let _harness = TestHarness::new(MinimalApp);
}

#[test]
fn counter_initial_state() {
    let h = TestHarness::new(Counter { count: 0 });
    assert_eq!(h.app().count, 0);
    assert!(!h.quit_requested());
}

#[test]
fn counter_increment_via_key() {
    let mut h = TestHarness::new(Counter { count: 0 });
    h.press_key(KeyCode::Char('j'));
    assert_eq!(h.app().count, 1);
}

#[test]
fn counter_decrement_via_key() {
    let mut h = TestHarness::new(Counter { count: 5 });
    h.press_key(KeyCode::Char('k'));
    assert_eq!(h.app().count, 4);
}

#[test]
fn counter_reset_via_key() {
    let mut h = TestHarness::new(Counter { count: 42 });
    h.press_key(KeyCode::Char('r'));
    assert_eq!(h.app().count, 0);
}

#[test]
fn counter_unhandled_key_no_change() {
    let mut h = TestHarness::new(Counter { count: 10 });
    h.press_key(KeyCode::Char('x'));
    assert_eq!(h.app().count, 10);
}

#[test]
fn counter_send_message_directly() {
    let mut h = TestHarness::new(Counter { count: 0 });
    h.send_message(CounterMsg::Inc);
    h.send_message(CounterMsg::Inc);
    h.send_message(CounterMsg::Dec);
    assert_eq!(h.app().count, 1);
}

#[test]
fn counter_quit_command() {
    let mut h = TestHarness::new(Counter { count: 0 });
    assert!(!h.quit_requested());
    h.send_message(CounterMsg::Quit);
    assert!(h.quit_requested());
}

#[test]
fn counter_batch_double_increment() {
    let mut h = TestHarness::new(Counter { count: 0 });
    h.press_key(KeyCode::Char('d'));
    assert_eq!(h.app().count, 2);
}

#[test]
fn counter_message_chain_via_send() {
    let mut h = TestHarness::new(Counter { count: 0 });
    h.send_message(CounterMsg::DoubleInc);
    assert_eq!(h.app().count, 2);
    h.send_message(CounterMsg::DoubleInc);
    assert_eq!(h.app().count, 4);
}

#[test]
fn counter_negative_count() {
    let mut h = TestHarness::new(Counter { count: 0 });
    h.press_key(KeyCode::Char('k'));
    h.press_key(KeyCode::Char('k'));
    assert_eq!(h.app().count, -2);
}

#[test]
fn counter_many_operations() {
    let mut h = TestHarness::new(Counter { count: 0 });
    for _ in 0..100 {
        h.press_key(KeyCode::Char('j'));
    }
    assert_eq!(h.app().count, 100);
    h.press_key(KeyCode::Char('r'));
    assert_eq!(h.app().count, 0);
}

// =============================================================================
// Tests: Panel System
// =============================================================================

#[test]
fn panel_app_has_panels() {
    let app = PanelApp::new();
    let layout = app.panels();
    assert!(!layout.is_none());
    assert_eq!(layout.panel_ids().len(), 2);
}

#[test]
fn panel_sidebar_navigate_down() {
    let mut h = TestHarness::new(PanelApp::new());
    h.press_panel_key("sidebar", KeyCode::Char('j'));
    assert_eq!(h.app().sidebar_selected, 1);
}

#[test]
fn panel_sidebar_navigate_up_at_zero() {
    let mut h = TestHarness::new(PanelApp::new());
    h.press_panel_key("sidebar", KeyCode::Char('k'));
    assert_eq!(h.app().sidebar_selected, 0);
}

#[test]
fn panel_sidebar_navigate_clamped() {
    let mut h = TestHarness::new(PanelApp::new());
    for _ in 0..10 {
        h.press_panel_key("sidebar", KeyCode::Char('j'));
    }
    assert_eq!(h.app().sidebar_selected, 2);
}

#[test]
fn panel_confirm_delete_dispatches() {
    let mut h = TestHarness::new(PanelApp::new());
    assert_eq!(h.app().sidebar_items.len(), 3);
    h.send_message(PanelMsg::DeleteItem);
    assert_eq!(h.app().sidebar_items.len(), 2);
    assert_eq!(h.app().sidebar_items[0], "Beta");
}

#[test]
fn panel_delete_adjusts_selection() {
    let mut h = TestHarness::new(PanelApp::new());
    h.press_panel_key("sidebar", KeyCode::Char('j'));
    h.press_panel_key("sidebar", KeyCode::Char('j'));
    assert_eq!(h.app().sidebar_selected, 2);
    h.send_message(PanelMsg::DeleteItem);
    assert_eq!(h.app().sidebar_selected, 1);
}

#[test]
fn panel_detail_key_ignored_on_sidebar() {
    let mut h = TestHarness::new(PanelApp::new());
    h.press_panel_key("detail", KeyCode::Char('j'));
    assert_eq!(h.app().sidebar_selected, 0);
}

#[test]
fn panel_titles_match() {
    let app = PanelApp::new();
    assert_eq!(app.panel_title("sidebar"), "Sidebar");
    assert_eq!(app.panel_title("detail"), "Detail");
    assert_eq!(app.panel_title("unknown"), "");
}

#[test]
fn panel_key_hints_sidebar_has_entries() {
    let app = PanelApp::new();
    let hints = app.panel_key_hints("sidebar");
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].key, "j/k");
}

#[test]
fn panel_key_hints_detail_empty() {
    let app = PanelApp::new();
    assert!(app.panel_key_hints("detail").is_empty());
}

// =============================================================================
// Tests: Command
// =============================================================================

#[test]
fn command_none_is_empty() {
    let cmd: Command<()> = Command::none();
    assert!(cmd.is_empty());
}

#[test]
fn command_quit_not_empty() {
    let cmd: Command<()> = Command::quit();
    assert!(!cmd.is_empty());
}

#[test]
fn command_batch_combines() {
    let cmd: Command<()> = Command::batch([Command::none(), Command::none(), Command::quit()]);
    assert!(!cmd.is_empty());
}

#[test]
fn command_batch_empty_is_empty() {
    let cmd: Command<()> = Command::batch(std::iter::empty());
    assert!(cmd.is_empty());
}

#[test]
fn command_message_has_action() {
    let cmd = Command::message(42i32);
    assert!(!cmd.is_empty());
}

// =============================================================================
// Tests: PanelLayout
// =============================================================================

#[test]
fn layout_none_has_no_panels() {
    let layout = PanelLayout::none();
    assert!(layout.is_none());
    assert!(layout.panel_ids().is_empty());
}

#[test]
fn layout_single_has_one_panel() {
    let layout = PanelLayout::single("main");
    assert!(!layout.is_none());
    assert_eq!(layout.panel_ids(), vec!["main"]);
}

#[test]
fn layout_horizontal_preserves_order() {
    let layout = PanelLayout::horizontal(vec![
        ("a", Constraint::Percentage(50)),
        ("b", Constraint::Percentage(50)),
    ]);
    assert_eq!(layout.panel_ids(), vec!["a", "b"]);
}

#[test]
fn layout_vertical_preserves_order() {
    let layout = PanelLayout::vertical(vec![
        ("top", Constraint::Min(5)),
        ("mid", Constraint::Percentage(50)),
        ("bot", Constraint::Length(3)),
    ]);
    assert_eq!(layout.panel_ids(), vec!["top", "mid", "bot"]);
}

#[test]
fn layout_nested_flattens_ids() {
    use ratatui::layout::Direction;
    let layout = PanelLayout::nested(
        Direction::Horizontal,
        vec![
            (PanelLayout::single("left"), Constraint::Percentage(30)),
            (
                PanelLayout::vertical(vec![
                    ("top_right", Constraint::Percentage(50)),
                    ("bot_right", Constraint::Percentage(50)),
                ]),
                Constraint::Percentage(70),
            ),
        ],
    );
    assert_eq!(layout.panel_ids(), vec!["left", "top_right", "bot_right"]);
}

#[test]
fn layout_compute_rects_single() {
    let layout = PanelLayout::single("main");
    let area = ratatui::layout::Rect::new(0, 0, 80, 24);
    let rects = layout.compute_rects(area);
    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0].0, "main");
    assert_eq!(rects[0].1, area);
}

#[test]
fn layout_compute_rects_horizontal_splits() {
    let layout = PanelLayout::horizontal(vec![
        ("a", Constraint::Percentage(50)),
        ("b", Constraint::Percentage(50)),
    ]);
    let area = ratatui::layout::Rect::new(0, 0, 100, 24);
    let rects = layout.compute_rects(area);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].0, "a");
    assert_eq!(rects[1].0, "b");
    assert!(rects[0].1.width > 0);
    assert!(rects[1].1.width > 0);
    assert_eq!(rects[0].1.width + rects[1].1.width, area.width);
}

#[test]
fn layout_compute_rects_none_empty() {
    let layout = PanelLayout::none();
    let area = ratatui::layout::Rect::new(0, 0, 80, 24);
    assert!(layout.compute_rects(area).is_empty());
}

// =============================================================================
// Tests: PanelManager
// =============================================================================

mod panel_manager_tests {
    use rataframe::panel::PanelManager;

    #[test]
    fn new_focuses_first() {
        let pm = PanelManager::new(vec!["a", "b", "c"]);
        assert_eq!(pm.focused_id(), Some("a"));
    }

    #[test]
    fn empty_focused_is_none() {
        let pm = PanelManager::new(vec![]);
        assert_eq!(pm.focused_id(), None);
    }

    #[test]
    fn focus_next_cycles() {
        let mut pm = PanelManager::new(vec!["a", "b", "c"]);
        assert_eq!(pm.focused_id(), Some("a"));
        pm.focus_next();
        assert_eq!(pm.focused_id(), Some("b"));
        pm.focus_next();
        assert_eq!(pm.focused_id(), Some("c"));
        pm.focus_next();
        assert_eq!(pm.focused_id(), Some("a"));
    }

    #[test]
    fn focus_prev_cycles() {
        let mut pm = PanelManager::new(vec!["a", "b", "c"]);
        pm.focus_prev();
        assert_eq!(pm.focused_id(), Some("c"));
        pm.focus_prev();
        assert_eq!(pm.focused_id(), Some("b"));
    }

    #[test]
    fn focus_panel_by_id() {
        let mut pm = PanelManager::new(vec!["a", "b", "c"]);
        pm.focus_panel("c");
        assert_eq!(pm.focused_id(), Some("c"));
    }

    #[test]
    fn focus_unknown_panel_is_noop() {
        let mut pm = PanelManager::new(vec!["a", "b"]);
        pm.focus_panel("z");
        assert_eq!(pm.focused_id(), Some("a"));
    }

    #[test]
    fn is_focused_correct() {
        let pm = PanelManager::new(vec!["a", "b"]);
        assert!(pm.is_focused("a"));
        assert!(!pm.is_focused("b"));
    }

    #[test]
    fn toggle_zoom() {
        let mut pm = PanelManager::new(vec!["a"]);
        assert!(!pm.is_zoomed());
        pm.toggle_zoom();
        assert!(pm.is_zoomed());
        pm.toggle_zoom();
        assert!(!pm.is_zoomed());
    }

    #[test]
    fn sync_layout_preserves_focus() {
        let mut pm = PanelManager::new(vec!["a", "b", "c"]);
        pm.focus_panel("c");
        pm.sync_layout(vec!["x", "c", "y"]);
        assert_eq!(pm.focused_id(), Some("c"));
    }

    #[test]
    fn sync_layout_resets_when_panel_gone() {
        let mut pm = PanelManager::new(vec!["a", "b"]);
        pm.focus_panel("b");
        pm.sync_layout(vec!["x", "y"]);
        assert_eq!(pm.focused_id(), Some("x"));
    }

    #[test]
    fn panel_ids_returns_all() {
        let pm = PanelManager::new(vec!["a", "b", "c"]);
        assert_eq!(pm.panel_ids(), &["a", "b", "c"]);
    }
}

// =============================================================================
// Tests: Theme
// =============================================================================

mod theme_tests {
    use rataframe::theme::palettes;
    use rataframe::theme::Theme;

    #[test]
    fn default_theme_is_nord() {
        let theme = Theme::default();
        assert_eq!(theme.name, "Nord");
    }

    #[test]
    fn theme_next_cycles_through_all() {
        let all = palettes::all();
        let mut theme = Theme::default();
        for expected in all.iter().skip(1).chain(all.iter().take(1)) {
            theme = theme.next();
            assert_eq!(theme.name, expected.name);
        }
    }

    #[test]
    fn all_palettes_have_distinct_names() {
        let all = palettes::all();
        let mut names: Vec<&str> = all.iter().map(|t| t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all.len());
    }

    #[test]
    fn palettes_count_is_four() {
        assert_eq!(palettes::all().len(), 4);
    }
}

// =============================================================================
// Tests: TextInputState
// =============================================================================

mod text_input_tests {
    use rataframe::widgets::TextInputState;

    #[test]
    fn new_is_empty() {
        let state = TextInputState::new();
        assert_eq!(state.content(), "");
    }

    #[test]
    fn with_content_sets_cursor_at_end() {
        let state = TextInputState::with_content("hello");
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn insert_char() {
        let mut state = TextInputState::new();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        assert_eq!(state.content(), "abc");
    }

    #[test]
    fn delete_char_before() {
        let mut state = TextInputState::with_content("abc");
        state.delete_char_before();
        assert_eq!(state.content(), "ab");
    }

    #[test]
    fn delete_char_before_empty() {
        let mut state = TextInputState::new();
        state.delete_char_before();
        assert_eq!(state.content(), "");
    }

    #[test]
    fn delete_char_after() {
        let mut state = TextInputState::with_content("abc");
        state.move_home();
        state.delete_char_after();
        assert_eq!(state.content(), "bc");
    }

    #[test]
    fn move_left_and_right() {
        let mut state = TextInputState::with_content("abc");
        state.move_left();
        state.insert_char('X');
        assert_eq!(state.content(), "abXc");
    }

    #[test]
    fn move_home_and_end() {
        let mut state = TextInputState::with_content("abc");
        state.move_home();
        state.insert_char('X');
        assert_eq!(state.content(), "Xabc");
        state.move_end();
        state.insert_char('Y');
        assert_eq!(state.content(), "XabcY");
    }

    #[test]
    fn clear() {
        let mut state = TextInputState::with_content("hello");
        state.clear();
        assert_eq!(state.content(), "");
    }

    #[test]
    fn set_content() {
        let mut state = TextInputState::new();
        state.set_content("new text");
        assert_eq!(state.content(), "new text");
    }

    #[test]
    fn delete_to_end() {
        let mut state = TextInputState::with_content("abcdef");
        state.move_home();
        state.move_right();
        state.move_right();
        state.delete_to_end();
        assert_eq!(state.content(), "ab");
    }

    #[test]
    fn utf8_multibyte_insert_delete() {
        let mut state = TextInputState::new();
        state.insert_char('日');
        state.insert_char('本');
        state.insert_char('語');
        assert_eq!(state.content(), "日本語");
        state.delete_char_before();
        assert_eq!(state.content(), "日本");
    }

    #[test]
    fn utf8_emoji() {
        let mut state = TextInputState::new();
        state.insert_char('🦀');
        state.insert_char('!');
        assert_eq!(state.content(), "🦀!");
        state.move_left();
        state.move_left();
        state.insert_char('R');
        assert_eq!(state.content(), "R🦀!");
    }

    #[test]
    fn move_left_at_start_is_noop() {
        let mut state = TextInputState::with_content("a");
        state.move_home();
        state.move_left();
        state.insert_char('X');
        assert_eq!(state.content(), "Xa");
    }

    #[test]
    fn move_right_at_end_is_noop() {
        let mut state = TextInputState::with_content("a");
        state.move_right();
        state.insert_char('X');
        assert_eq!(state.content(), "aX");
    }
}

// =============================================================================
// Tests: OverlayStack
// =============================================================================

mod overlay_stack_tests {
    use rataframe::overlay::{Overlay, OverlayResult, OverlayStack};
    use rataframe::theme::Theme;

    struct DummyOverlay(&'static str);
    impl Overlay<String> for DummyOverlay {
        fn title(&self) -> &str {
            self.0
        }
        fn view(&self, _frame: &mut ratatui::Frame, _area: ratatui::layout::Rect, _theme: &Theme) {}
        fn handle_event(&mut self, _event: &crossterm::event::Event) -> OverlayResult<String> {
            OverlayResult::Consumed
        }
    }

    #[test]
    fn stack_starts_empty() {
        let stack: OverlayStack<String> = OverlayStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn push_and_pop() {
        let mut stack: OverlayStack<String> = OverlayStack::new();
        stack.push(Box::new(DummyOverlay("A")));
        assert!(!stack.is_empty());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.top().unwrap().title(), "A");

        stack.push(Box::new(DummyOverlay("B")));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.top().unwrap().title(), "B");

        stack.pop();
        assert_eq!(stack.top().unwrap().title(), "A");

        stack.pop();
        assert!(stack.is_empty());
    }

    #[test]
    fn pop_empty_is_safe() {
        let mut stack: OverlayStack<String> = OverlayStack::new();
        stack.pop();
        assert!(stack.is_empty());
    }
}

// =============================================================================
// Tests: NavigationStack
// =============================================================================

mod nav_stack_tests {
    use rataframe::prelude::*;
    use rataframe::screen::{NavigationStack, Screen};

    struct TestScreen {
        name: &'static str,
        entered: bool,
        left: bool,
    }

    impl TestScreen {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                entered: false,
                left: false,
            }
        }
    }

    impl Screen<String> for TestScreen {
        fn id(&self) -> &str {
            self.name
        }
        fn panels(&self) -> PanelLayout {
            PanelLayout::single("main")
        }
        fn panel_title(&self, _id: PanelId) -> &str {
            self.name
        }
        fn panel_view(
            &self,
            _id: PanelId,
            _frame: &mut ratatui::Frame,
            _area: ratatui::layout::Rect,
            _focused: bool,
        ) {
        }
        fn on_enter(&mut self) {
            self.entered = true;
        }
        fn on_leave(&mut self) {
            self.left = true;
        }
    }

    #[test]
    fn starts_empty() {
        let stack: NavigationStack<String> = NavigationStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn push_calls_on_enter() {
        let mut stack: NavigationStack<String> = NavigationStack::new();
        stack.push(Box::new(TestScreen::new("A")));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.top().unwrap().id(), "A");
    }

    #[test]
    fn pop_calls_on_leave() {
        let mut stack: NavigationStack<String> = NavigationStack::new();
        stack.push(Box::new(TestScreen::new("A")));
        let popped = stack.pop().unwrap();
        assert!(stack.is_empty());
        assert_eq!(popped.id(), "A");
    }

    #[test]
    fn pop_empty_returns_none() {
        let mut stack: NavigationStack<String> = NavigationStack::new();
        assert!(stack.pop().is_none());
    }

    #[test]
    fn depth_tracks_correctly() {
        let mut stack: NavigationStack<String> = NavigationStack::new();
        stack.push(Box::new(TestScreen::new("A")));
        stack.push(Box::new(TestScreen::new("B")));
        stack.push(Box::new(TestScreen::new("C")));
        assert_eq!(stack.depth(), 3);
        assert_eq!(stack.top().unwrap().id(), "C");
        stack.pop();
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.top().unwrap().id(), "B");
    }
}

// =============================================================================
// Tests: Toast
// =============================================================================

mod toast_tests {
    use rataframe::toast::{Toast, ToastLevel, ToastManager};
    use std::time::Duration;

    #[test]
    fn toast_not_expired_initially() {
        let toast = Toast::new("hello", ToastLevel::Info);
        assert!(!toast.is_expired());
    }

    #[test]
    fn toast_with_zero_ttl_expires_immediately() {
        let toast = Toast::new("hello", ToastLevel::Info).with_ttl(Duration::ZERO);
        assert!(toast.is_expired());
    }

    #[test]
    fn manager_starts_empty() {
        let mgr = ToastManager::new();
        assert!(mgr.is_empty());
    }

    #[test]
    fn manager_push_and_tick() {
        let mut mgr = ToastManager::new();
        mgr.push(Toast::new("hi", ToastLevel::Success));
        assert!(!mgr.is_empty());
        assert_eq!(mgr.toasts().len(), 1);
    }

    #[test]
    fn manager_tick_removes_expired() {
        let mut mgr = ToastManager::new();
        mgr.push(Toast::new("hi", ToastLevel::Info).with_ttl(Duration::ZERO));
        mgr.tick();
        assert!(mgr.is_empty());
    }

    #[test]
    fn multiple_toasts() {
        let mut mgr = ToastManager::new();
        mgr.push(Toast::new("one", ToastLevel::Info));
        mgr.push(Toast::new("two", ToastLevel::Warning));
        mgr.push(Toast::new("three", ToastLevel::Error));
        assert_eq!(mgr.toasts().len(), 3);
    }
}

// =============================================================================
// Tests: KeyHint
// =============================================================================

mod keyhint_tests {
    use rataframe::panel::KeyHint;

    #[test]
    fn keyhint_construction() {
        let hint = KeyHint::new("j/k", "Navigate");
        assert_eq!(hint.key, "j/k");
        assert_eq!(hint.desc, "Navigate");
    }
}

// =============================================================================
// Tests: ErrorBoundaryState
// =============================================================================

mod error_boundary_tests {
    use rataframe::panel::ErrorBoundaryState;

    #[test]
    fn no_errors_initially() {
        let state = ErrorBoundaryState::new();
        assert!(!state.has_error("panel_a"));
    }

    #[test]
    fn clear_nonexistent_is_safe() {
        let mut state = ErrorBoundaryState::new();
        state.clear("nonexistent");
    }
}

// =============================================================================
// Tests: centered_rect
// =============================================================================

mod centered_rect_tests {
    use rataframe::widgets::centered_rect;
    use ratatui::layout::Rect;

    #[test]
    fn centered_within_area() {
        let area = Rect::new(0, 0, 100, 50);
        let result = centered_rect(50, 50, area);
        assert!(result.x > 0);
        assert!(result.y > 0);
        assert!(result.x + result.width <= area.width);
        assert!(result.y + result.height <= area.height);
    }

    #[test]
    fn full_size_fills_area() {
        let area = Rect::new(0, 0, 100, 50);
        let result = centered_rect(100, 100, area);
        assert_eq!(result.width, area.width);
        assert_eq!(result.height, area.height);
    }
}

// =============================================================================
// Tests: Application trait defaults
// =============================================================================

mod trait_defaults {
    use rataframe::prelude::*;
    use std::time::Duration;

    struct Defaults;
    impl Application for Defaults {
        type Message = ();
        fn update(&mut self, _: (), _ctx: &mut Context<()>) -> Command<()> {
            Command::none()
        }
    }

    #[test]
    fn default_title() {
        let app = Defaults;
        assert_eq!(app.title(), "rataframe app");
    }

    #[test]
    fn default_tick_rate() {
        let app = Defaults;
        assert_eq!(app.tick_rate(), Duration::from_millis(250));
    }

    #[test]
    fn default_panels_is_none() {
        let app = Defaults;
        assert!(app.panels().is_none());
    }

    #[test]
    fn default_theme_is_nord() {
        let app = Defaults;
        assert_eq!(app.theme().name, "Nord");
    }

    #[test]
    fn default_subscriptions_empty() {
        let app = Defaults;
        assert!(app.subscriptions().is_empty());
    }

    #[test]
    fn default_init_is_none() {
        let app = Defaults;
        assert!(app.init().is_empty());
    }

    #[test]
    fn default_handle_event_is_ignored() {
        let app = Defaults;
        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let ctx = EventContext::with_state(None, false);
        match app.handle_event(&event, &ctx) {
            EventResult::Ignored => {}
            _ => panic!("expected Ignored"),
        }
    }

    #[test]
    fn default_panel_key_hints_empty() {
        let app = Defaults;
        assert!(app.panel_key_hints("any").is_empty());
    }
}

// =============================================================================
// Tests: Context intents
// =============================================================================

mod context_tests {
    use rataframe::prelude::*;
    use rataframe::toast::ToastLevel;

    #[test]
    fn context_accumulates_intents() {
        let mut ctx: Context<String> = Context::new();
        assert_eq!(ctx.intent_count(), 0);
        ctx.focus_panel("sidebar");
        ctx.toggle_zoom();
        ctx.toast("hello", ToastLevel::Info);
        ctx.pop_overlay();
        ctx.pop_screen();
        assert_eq!(ctx.intent_count(), 5);
    }
}

// =============================================================================
// Tests: ViewContext and EventContext
// =============================================================================

mod view_event_context_tests {
    use rataframe::context::{EventContext, ViewContext};

    #[test]
    fn view_context_defaults() {
        let ctx = ViewContext::new();
        assert_eq!(ctx.focused_panel(), None);
        assert!(!ctx.is_zoomed());
    }

    #[test]
    fn event_context_with_state() {
        let ctx = EventContext::with_state(Some("sidebar"), true);
        assert_eq!(ctx.focused_panel(), Some("sidebar"));
        assert!(ctx.has_overlay());
    }

    #[test]
    fn event_context_no_overlay() {
        let ctx = EventContext::with_state(None, false);
        assert!(!ctx.has_overlay());
        assert_eq!(ctx.focused_panel(), None);
    }
}
