pub mod terminal;

use std::sync::mpsc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::{Context, EventContext, Intent, ViewContext};
use crate::overlay::{CommandPaletteOverlay, HelpOverlay, OverlayResult, OverlayStack};
use crate::panel::{ErrorBoundaryState, KeyHint, PanelManager};
use crate::screen::NavigationStack;
use crate::subscription::SubscriptionManager;
use crate::toast::{ToastManager, ToastWidget};
use crate::widgets::Footer;

pub fn run<A: Application>(mut app: A) -> Result<()> {
    color_eyre::install()?;
    let mut terminal = terminal::init()?;

    let mut should_quit = false;

    let (bg_tx, bg_rx) = mpsc::channel::<A::Message>();

    let layout = app.panels();
    let has_panels = !layout.is_none();
    let mut panel_manager = PanelManager::new(layout.panel_ids());
    let mut overlay_stack: OverlayStack<A::Message> = OverlayStack::new();
    let mut toast_manager = ToastManager::new();
    let mut sub_manager = SubscriptionManager::<A::Message>::new();
    let mut error_boundaries = ErrorBoundaryState::new();
    let mut nav_stack = NavigationStack::<A::Message>::new();
    let mut last_panel_rects: Vec<(&str, Rect)> = Vec::new();

    if let Some(id) = panel_manager.focused_id() {
        app.panel_on_focus(id);
    }

    let init_cmd = app.init();
    process_command(
        init_cmd,
        &mut app,
        &mut should_quit,
        &mut overlay_stack,
        &mut panel_manager,
        &mut toast_manager,
        &mut nav_stack,
        &bg_tx,
    );

    let tick_rate = app.tick_rate();
    let mut last_tick = Instant::now();

    while !should_quit {
        // --- Drain background messages ------------------------------------
        while let Ok(msg) = bg_rx.try_recv() {
            dispatch_message(
                msg,
                &mut app,
                &mut should_quit,
                &mut overlay_stack,
                &mut panel_manager,
                &mut toast_manager,
                &mut nav_stack,
                &bg_tx,
            );
        }

        if should_quit {
            break;
        }

        // --- Sync subscriptions -------------------------------------------
        let subs = app.subscriptions();
        sub_manager.sync(subs, &bg_tx);

        // --- Tick toasts --------------------------------------------------
        toast_manager.tick();

        // --- Render -------------------------------------------------------
        let theme = app.theme();
        terminal.draw(|frame| {
            let full_area = frame.area();

            // Minimum terminal size check
            if full_area.width < terminal::MIN_WIDTH || full_area.height < terminal::MIN_HEIGHT {
                let msg = format!(
                    "Terminal too small ({}x{}). Minimum: {}x{}",
                    full_area.width,
                    full_area.height,
                    terminal::MIN_WIDTH,
                    terminal::MIN_HEIGHT,
                );
                frame.render_widget(
                    Paragraph::new(msg).style(Style::default().fg(theme.warning)),
                    full_area,
                );
                return;
            }

            if has_panels {
                let main_and_footer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(full_area);

                let main_area = main_and_footer[0];
                let footer_area = main_and_footer[1];

                let current_layout = app.panels();

                if panel_manager.is_zoomed() {
                    if let Some(focused_id) = panel_manager.focused_id() {
                        let title = app.panel_title(focused_id);
                        let block = Block::default()
                            .title(format!(" {} [zoomed] ", title))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(theme.border_focused));
                        let inner = block.inner(main_area);
                        frame.render_widget(block, main_area);
                        error_boundaries.guarded_view(focused_id, frame, inner, |f, a| {
                            app.panel_view(focused_id, f, a, true);
                        });
                    }
                } else {
                    let panel_rects = current_layout.compute_rects(main_area);
                    last_panel_rects = panel_rects.clone();
                    for (id, rect) in &panel_rects {
                        let focused = panel_manager.is_focused(id);
                        let title = app.panel_title(id);
                        let border_color = if focused {
                            theme.border_focused
                        } else {
                            theme.border
                        };
                        let block = Block::default()
                            .title(format!(" {} ", title))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(border_color));
                        let inner = block.inner(*rect);
                        frame.render_widget(block, *rect);
                        error_boundaries.guarded_view(id, frame, inner, |f, a| {
                            app.panel_view(id, f, a, focused);
                        });
                    }
                }

                let focused_hints = panel_manager
                    .focused_id()
                    .map(|id| app.panel_key_hints(id))
                    .unwrap_or_default();
                let footer = Footer::new(&focused_hints, &theme);
                frame.render_widget(footer, footer_area);
            } else {
                let view_ctx = ViewContext::new();
                app.view(frame, &view_ctx);
            }

            // Toasts render on top of content but below overlays
            if !toast_manager.is_empty() {
                let toast_widget = ToastWidget::new(&toast_manager, &theme);
                frame.render_widget(toast_widget, full_area);
            }

            // Overlays render on top of everything
            if let Some(overlay) = overlay_stack.top() {
                overlay.view(frame, full_area, &theme);
            }
        })?;

        // --- Poll events --------------------------------------------------
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;

            if is_hard_quit(&ev) {
                should_quit = true;
                continue;
            }

            // Mouse click-to-focus: check if click lands on a panel
            if let Event::Mouse(mouse) = &ev {
                if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    && has_panels
                    && overlay_stack.is_empty()
                {
                    for (id, rect) in &last_panel_rects {
                        if rect.x <= mouse.column
                            && mouse.column < rect.x + rect.width
                            && rect.y <= mouse.row
                            && mouse.row < rect.y + rect.height
                        {
                            panel_manager.focus_panel(id);
                            break;
                        }
                    }
                }
            }

            if !overlay_stack.is_empty() {
                if let Some(overlay) = overlay_stack.top_mut() {
                    match overlay.handle_event(&ev) {
                        OverlayResult::Close => {
                            overlay_stack.pop();
                        }
                        OverlayResult::CloseWithMessage(msg) => {
                            overlay_stack.pop();
                            dispatch_message(
                                msg,
                                &mut app,
                                &mut should_quit,
                                &mut overlay_stack,
                                &mut panel_manager,
                                &mut toast_manager,
                                &mut nav_stack,
                                &bg_tx,
                            );
                        }
                        OverlayResult::Consumed | OverlayResult::Ignored => {}
                    }
                }
            } else {
                let event_ctx =
                    EventContext::with_state(panel_manager.focused_id(), !overlay_stack.is_empty());
                match app.handle_event(&ev, &event_ctx) {
                    EventResult::Message(msg) => {
                        dispatch_message(
                            msg,
                            &mut app,
                            &mut should_quit,
                            &mut overlay_stack,
                            &mut panel_manager,
                            &mut toast_manager,
                            &mut nav_stack,
                            &bg_tx,
                        );
                    }
                    EventResult::Consumed => {}
                    EventResult::Ignored => {
                        let mut handled = false;
                        if has_panels {
                            if let Some(focused_id) = panel_manager.focused_id() {
                                if let Event::Key(key) = &ev {
                                    match app.panel_handle_key(focused_id, key) {
                                        EventResult::Message(msg) => {
                                            dispatch_message(
                                                msg,
                                                &mut app,
                                                &mut should_quit,
                                                &mut overlay_stack,
                                                &mut panel_manager,
                                                &mut toast_manager,
                                                &mut nav_stack,
                                                &bg_tx,
                                            );
                                            handled = true;
                                        }
                                        EventResult::Consumed => {
                                            handled = true;
                                        }
                                        EventResult::Ignored => {}
                                    }
                                }
                            }
                        }

                        if !handled {
                            handle_convention_keys(
                                &ev,
                                &mut should_quit,
                                &mut panel_manager,
                                &mut overlay_stack,
                                &mut nav_stack,
                                &app,
                                has_panels,
                            );
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    sub_manager.shutdown();
    terminal::restore()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn dispatch_message<A: Application>(
    msg: A::Message,
    app: &mut A,
    should_quit: &mut bool,
    overlay_stack: &mut OverlayStack<A::Message>,
    panel_manager: &mut PanelManager,
    toast_manager: &mut ToastManager,
    nav_stack: &mut NavigationStack<A::Message>,
    bg_tx: &mpsc::Sender<A::Message>,
) {
    let mut ctx = Context::new();
    let cmd = app.update(msg, &mut ctx);
    process_intents(
        &mut ctx,
        overlay_stack,
        panel_manager,
        toast_manager,
        nav_stack,
        app,
    );
    process_command(
        cmd,
        app,
        should_quit,
        overlay_stack,
        panel_manager,
        toast_manager,
        nav_stack,
        bg_tx,
    );
}

#[allow(clippy::too_many_arguments)]
fn process_command<A: Application>(
    cmd: crate::command::Command<A::Message>,
    app: &mut A,
    should_quit: &mut bool,
    overlay_stack: &mut OverlayStack<A::Message>,
    panel_manager: &mut PanelManager,
    toast_manager: &mut ToastManager,
    nav_stack: &mut NavigationStack<A::Message>,
    bg_tx: &mpsc::Sender<A::Message>,
) {
    let mut pending: Vec<Action<A::Message>> = cmd.actions;

    while !pending.is_empty() {
        let batch = std::mem::take(&mut pending);
        for action in batch {
            match action {
                Action::Quit => {
                    *should_quit = true;
                    return;
                }
                Action::Message(msg) => {
                    let mut ctx = Context::new();
                    let next = app.update(msg, &mut ctx);
                    process_intents(
                        &mut ctx,
                        overlay_stack,
                        panel_manager,
                        toast_manager,
                        nav_stack,
                        app,
                    );
                    pending.extend(next.actions);
                }
                Action::Perform(task) => {
                    let tx = bg_tx.clone();
                    std::thread::spawn(move || task(tx));
                }
            }
        }
    }
}

fn process_intents<A: Application>(
    ctx: &mut Context<A::Message>,
    overlay_stack: &mut OverlayStack<A::Message>,
    panel_manager: &mut PanelManager,
    toast_manager: &mut ToastManager,
    nav_stack: &mut NavigationStack<A::Message>,
    app: &mut A,
) {
    let intents = std::mem::take(&mut ctx.intents);
    for intent in intents {
        match intent {
            Intent::PushOverlay(overlay) => overlay_stack.push(overlay),
            Intent::PopOverlay => overlay_stack.pop(),
            Intent::FocusPanel(id) => {
                if let Some(old) = panel_manager.focused_id() {
                    app.panel_on_blur(old);
                }
                panel_manager.focus_panel(id);
                if let Some(new) = panel_manager.focused_id() {
                    app.panel_on_focus(new);
                }
            }
            Intent::ToggleZoom => panel_manager.toggle_zoom(),
            Intent::ShowToast(toast) => toast_manager.push(toast),
            Intent::PushScreen(screen) => {
                nav_stack.push(screen);
                // Sync panel manager to the new screen's layout
                if let Some(top) = nav_stack.top() {
                    let ids = top.panels().panel_ids();
                    panel_manager.sync_layout(ids);
                }
            }
            Intent::PopScreen => {
                nav_stack.pop();
                // Restore panel manager to the previous screen or app
                if let Some(top) = nav_stack.top() {
                    let ids = top.panels().panel_ids();
                    panel_manager.sync_layout(ids);
                } else {
                    let ids = app.panels().panel_ids();
                    panel_manager.sync_layout(ids);
                }
            }
        }
    }
}

fn is_hard_quit(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers, .. })
            if modifiers.contains(KeyModifiers::CONTROL)
    )
}

fn handle_convention_keys<A: Application>(
    ev: &Event,
    should_quit: &mut bool,
    panel_manager: &mut PanelManager,
    overlay_stack: &mut OverlayStack<A::Message>,
    nav_stack: &mut NavigationStack<A::Message>,
    app: &A,
    has_panels: bool,
) {
    if let Event::Key(key) = ev {
        match key.code {
            KeyCode::Char('q') => *should_quit = true,
            KeyCode::Esc => {
                // Esc chain: pop overlay → pop screen → quit
                if !overlay_stack.is_empty() {
                    overlay_stack.pop();
                } else if !nav_stack.is_empty() {
                    nav_stack.pop();
                    if let Some(top) = nav_stack.top() {
                        panel_manager.sync_layout(top.panels().panel_ids());
                    } else {
                        panel_manager.sync_layout(app.panels().panel_ids());
                    }
                } else {
                    *should_quit = true;
                }
            }
            KeyCode::Tab if has_panels => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    panel_manager.focus_prev();
                } else {
                    panel_manager.focus_next();
                }
            }
            KeyCode::Char('z') if has_panels => {
                panel_manager.toggle_zoom();
            }
            KeyCode::Char('?') if has_panels => {
                let mut sections = Vec::new();
                for &id in panel_manager.panel_ids() {
                    let title = app.panel_title(id).to_string();
                    let hints = app.panel_key_hints(id);
                    if !hints.is_empty() {
                        sections.push((title, hints));
                    }
                }
                sections.push((
                    "Global".to_string(),
                    vec![
                        KeyHint::new("Tab", "Switch panel"),
                        KeyHint::new("Shift+Tab", "Previous panel"),
                        KeyHint::new("z", "Zoom toggle"),
                        KeyHint::new(":", "Command palette"),
                        KeyHint::new("?", "This help"),
                        KeyHint::new("q", "Quit"),
                    ],
                ));
                overlay_stack.push(Box::new(HelpOverlay::new(sections)));
            }
            KeyCode::Char(':') if has_panels => {
                let mut hints: Vec<(String, String)> = Vec::new();

                for &id in panel_manager.panel_ids() {
                    let title = app.panel_title(id);
                    for hint in app.panel_key_hints(id) {
                        hints.push((format!("[{}] {}", title, hint.desc), hint.key.to_string()));
                    }
                }

                // Global convention hints
                hints.push(("Toggle help".into(), "?".into()));
                hints.push(("Next panel".into(), "Tab".into()));
                hints.push(("Toggle zoom".into(), "z".into()));
                hints.push(("Quit".into(), "q".into()));

                overlay_stack.push(Box::new(CommandPaletteOverlay::with_hints(
                    hints,
                    Vec::new(),
                )));
            }
            _ => {}
        }
    }
}
