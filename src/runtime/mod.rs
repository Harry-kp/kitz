pub mod terminal;

use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Application, EventResult};
use crate::command::Action;
use crate::context::{Context, EventContext, ViewContext};

/// The main entry-point called by [`crate::run`]. Initialises the terminal,
/// enters the event-loop, and restores the terminal on exit.
pub fn run<A: Application>(mut app: A) -> Result<()> {
    color_eyre::install()?;
    let mut terminal = terminal::init()?;

    let mut should_quit = false;

    // Process the init command
    let init_cmd = app.init();
    process_command(init_cmd, &mut app, &mut should_quit);

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
                    process_command(cmd, &mut app, &mut should_quit);
                }
                EventResult::Consumed => {}
                EventResult::Ignored => {
                    handle_convention_keys(&ev, &mut should_quit);
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

/// Process all actions in a command. Handles re-dispatch of
/// `Command::message()` by calling `app.update()` again in a loop.
fn process_command<A: Application>(
    cmd: crate::command::Command<A::Message>,
    app: &mut A,
    should_quit: &mut bool,
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
                    pending.extend(next.actions);
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

/// Convention keys handled by the framework.
fn handle_convention_keys(ev: &Event, should_quit: &mut bool) {
    if let Event::Key(key) = ev {
        if key.code == KeyCode::Char('q') {
            *should_quit = true;
        }
    }
}
