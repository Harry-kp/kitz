pub mod terminal;

use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::{Context, EventContext, ViewContext};

/// The main entry-point called by [`crate::run`]. Initialises the terminal,
/// enters the event-loop, and restores the terminal on exit.
pub fn run<A: Application>(mut app: A) -> Result<()> {
    color_eyre::install()?;
    let mut terminal = terminal::init()?;

    // Process the init command
    let init_cmd = app.init();
    let mut should_quit = process_command(init_cmd);

    let tick_rate = app.tick_rate();
    let mut last_tick = Instant::now();

    while !should_quit {
        // --- Render --------------------------------------------------------
        let view_ctx = ViewContext::new();
        terminal.draw(|frame| {
            app.view(frame, &view_ctx);
        })?;

        // --- Poll events ---------------------------------------------------
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;

            // Ctrl+C is non-overridable — always quits
            if is_hard_quit(&ev) {
                should_quit = true;
                continue;
            }

            // Let the app handle the event
            let event_ctx = EventContext::new();
            match app.handle_event(&ev, &event_ctx) {
                EventResult::Message(msg) => {
                    let mut ctx = Context::new();
                    let cmd = app.update(msg, &mut ctx);
                    should_quit = process_command(cmd);
                }
                EventResult::Consumed => {
                    // App handled it — nothing more to do
                }
                EventResult::Ignored => {
                    // Fall through to convention keys (Phase 2 will add 'q',
                    // Phase 3 will add Tab, ?, etc.)
                    should_quit = handle_convention_keys(&ev);
                }
            }
        }

        // --- Tick ----------------------------------------------------------
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            // Future: call app.on_tick or fire a tick subscription
        }
    }

    terminal::restore()?;
    Ok(())
}

/// Process a command returned by `update()`. Returns `true` if the app should
/// quit.
fn process_command<M: std::fmt::Debug + Send + 'static>(cmd: crate::command::Command<M>) -> bool {
    for action in cmd.actions {
        match action {
            Action::Quit => return true,
            Action::Message(_msg) => {
                // Re-dispatch will be wired in Phase 2 when we have a proper
                // message channel. For now this is a no-op.
            }
        }
    }
    false
}

fn is_hard_quit(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(key) if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
    )
}

/// Convention key handling. Phase 1 only has `q` to quit. Later phases add
/// Tab, ?, :, Esc, z, mouse, etc.
fn handle_convention_keys(ev: &Event) -> bool {
    if let Event::Key(key) = ev {
        if key.code == KeyCode::Char('q') {
            return true;
        }
    }
    false
}
