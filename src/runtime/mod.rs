pub mod terminal;

use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::{Context, EventContext, Intent, ViewContext};
use crate::overlay::{HelpOverlay, OverlayResult, OverlayStack};
use crate::panel::{KeyHint, PanelManager};
use crate::widgets::Footer;

pub fn run<A: Application>(mut app: A) -> Result<()> {
    color_eyre::install()?;
    let mut terminal = terminal::init()?;

    let mut should_quit = false;

    // Initialise panel manager from the app's layout
    let layout = app.panels();
    let has_panels = !layout.is_none();
    let mut panel_manager = PanelManager::new(layout.panel_ids());
    let mut overlay_stack: OverlayStack<A::Message> = OverlayStack::new();

    // Fire the app's initial focus callback
    if let Some(id) = panel_manager.focused_id() {
        app.panel_on_focus(id);
    }

    // Process the init command
    let init_cmd = app.init();
    process_command(
        init_cmd,
        &mut app,
        &mut should_quit,
        &mut overlay_stack,
        &mut panel_manager,
    );

    let tick_rate = app.tick_rate();
    let mut last_tick = Instant::now();

    while !should_quit {
        // --- Render --------------------------------------------------------
        let theme = app.theme();
        terminal.draw(|frame| {
            let full_area = frame.area();

            if has_panels {
                // Convention path: framework renders panels + footer
                let main_and_footer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(full_area);

                let main_area = main_and_footer[0];
                let footer_area = main_and_footer[1];

                // Re-compute the layout each frame (app state might change it)
                let current_layout = app.panels();

                if panel_manager.is_zoomed() {
                    // Zoomed: render only the focused panel
                    if let Some(focused_id) = panel_manager.focused_id() {
                        let title = app.panel_title(focused_id);
                        let block = Block::default()
                            .title(format!(" {} [zoomed] ", title))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(theme.border_focused));
                        let inner = block.inner(main_area);
                        frame.render_widget(block, main_area);
                        app.panel_view(focused_id, frame, inner, true);
                    }
                } else {
                    // Normal: render all panels
                    let panel_rects = current_layout.compute_rects(main_area);
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
                        app.panel_view(id, frame, inner, focused);
                    }
                }

                // Footer
                let focused_hints = panel_manager
                    .focused_id()
                    .map(|id| app.panel_key_hints(id))
                    .unwrap_or_default();
                let footer = Footer::new(&focused_hints, &theme);
                frame.render_widget(footer, footer_area);
            } else {
                // Custom path: let the app render everything
                let view_ctx = ViewContext::new();
                app.view(frame, &view_ctx);
            }

            // Overlays render on top of everything
            if let Some(overlay) = overlay_stack.top() {
                overlay.view(frame, full_area, &theme);
            }
        })?;

        // --- Poll events ---------------------------------------------------
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;

            if is_hard_quit(&ev) {
                should_quit = true;
                continue;
            }

            // Event dispatch chain:
            // 1. Top overlay (if any)
            // 2. App handle_event
            // 3. Focused panel handle_key (if panels exist)
            // 4. Convention keys

            if !overlay_stack.is_empty() {
                // Route to overlay
                if let Some(overlay) = overlay_stack.top_mut() {
                    match overlay.handle_event(&ev) {
                        OverlayResult::Close => {
                            overlay_stack.pop();
                        }
                        OverlayResult::CloseWithMessage(msg) => {
                            overlay_stack.pop();
                            let mut ctx = Context::new();
                            let cmd = app.update(msg, &mut ctx);
                            process_intents(
                                &mut ctx,
                                &mut overlay_stack,
                                &mut panel_manager,
                                &mut app,
                            );
                            process_command(
                                cmd,
                                &mut app,
                                &mut should_quit,
                                &mut overlay_stack,
                                &mut panel_manager,
                            );
                        }
                        OverlayResult::Consumed | OverlayResult::Ignored => {}
                    }
                }
            } else {
                // No overlay — normal dispatch
                let event_ctx =
                    EventContext::with_state(panel_manager.focused_id(), !overlay_stack.is_empty());
                match app.handle_event(&ev, &event_ctx) {
                    EventResult::Message(msg) => {
                        let mut ctx = Context::new();
                        let cmd = app.update(msg, &mut ctx);
                        process_intents(&mut ctx, &mut overlay_stack, &mut panel_manager, &mut app);
                        process_command(
                            cmd,
                            &mut app,
                            &mut should_quit,
                            &mut overlay_stack,
                            &mut panel_manager,
                        );
                    }
                    EventResult::Consumed => {}
                    EventResult::Ignored => {
                        // Try focused panel
                        let mut handled = false;
                        if has_panels {
                            if let Some(focused_id) = panel_manager.focused_id() {
                                if let Event::Key(key) = &ev {
                                    match app.panel_handle_key(focused_id, key) {
                                        EventResult::Message(msg) => {
                                            let mut ctx = Context::new();
                                            let cmd = app.update(msg, &mut ctx);
                                            process_intents(
                                                &mut ctx,
                                                &mut overlay_stack,
                                                &mut panel_manager,
                                                &mut app,
                                            );
                                            process_command(
                                                cmd,
                                                &mut app,
                                                &mut should_quit,
                                                &mut overlay_stack,
                                                &mut panel_manager,
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
                                &app,
                                has_panels,
                            );
                        }
                    }
                }
            }
        }

        // --- Tick ----------------------------------------------------------
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    terminal::restore()?;
    Ok(())
}

fn process_command<A: Application>(
    cmd: crate::command::Command<A::Message>,
    app: &mut A,
    should_quit: &mut bool,
    overlay_stack: &mut OverlayStack<A::Message>,
    panel_manager: &mut PanelManager,
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
                    process_intents(&mut ctx, overlay_stack, panel_manager, app);
                    pending.extend(next.actions);
                }
            }
        }
    }
}

fn process_intents<A: Application>(
    ctx: &mut Context<A::Message>,
    overlay_stack: &mut OverlayStack<A::Message>,
    panel_manager: &mut PanelManager,
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

/// Convention keys handled by the framework when no panel/app handles the event.
fn handle_convention_keys<A: Application>(
    ev: &Event,
    should_quit: &mut bool,
    panel_manager: &mut PanelManager,
    overlay_stack: &mut OverlayStack<A::Message>,
    app: &A,
    has_panels: bool,
) {
    if let Event::Key(key) = ev {
        match key.code {
            KeyCode::Char('q') => *should_quit = true,
            KeyCode::Tab if has_panels => {
                let old = panel_manager.focused_id();
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    panel_manager.focus_prev();
                } else {
                    panel_manager.focus_next();
                }
                // Focus callbacks are handled via the &dyn trait — we need
                // &mut for on_focus/on_blur, so we skip them here in the
                // convention-key path. They fire in process_intents when
                // triggered via ctx.focus_panel().
                let _ = (old, app);
            }
            KeyCode::Char('z') if has_panels => {
                panel_manager.toggle_zoom();
            }
            KeyCode::Char('?') if has_panels => {
                // Build help overlay from all panels' key hints
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
                        KeyHint::new("?", "This help"),
                        KeyHint::new("q", "Quit"),
                    ],
                ));
                overlay_stack.push(Box::new(HelpOverlay::new(sections)));
            }
            _ => {}
        }
    }
}
