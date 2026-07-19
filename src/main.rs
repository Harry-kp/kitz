//! mskui — an IAM-auth-native, multi-environment terminal UI for AWS MSK.
//!
//! Launch → pick an environment → inspect topics, partitions, consumer groups,
//! peek events, and do topic admin (create / add partitions / delete) with a
//! prod guardrail. Auth is MSK IAM (SASL OAUTHBEARER) using your ~/.aws creds.

mod app;
mod brand;
mod config;
mod kafka;
mod theme;
mod ui;
mod worker;

use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};

use crate::app::App;
use crate::config::Config;

fn main() -> Result<()> {
    let config = Config::load()?;

    // `mskui doctor [env]` — plain-text connectivity diagnosis, no TUI.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("doctor") {
        let env = match args.get(1) {
            Some(name) => config
                .envs
                .iter()
                .find(|e| &e.name == name)
                .with_context(|| format!("no env named '{name}' in config"))?,
            None => &config.envs[0],
        };
        kafka::doctor(env);
        return Ok(());
    }

    let mut app = App::new(config);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        // Non-blocking: apply whatever the worker has produced since last tick.
        app.drain_events();
        app.tick();

        terminal.draw(|frame| ui::render(frame, app))?;

        // Tighten the frame budget while the flip animation is mid-flight so the
        // card-flip is smooth (~60fps); otherwise a relaxed 100ms tick.
        let budget = if app.animating() { 16 } else { 100 };
        if event::poll(Duration::from_millis(budget))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    app.on_key(key)?;
                }
            }
        }
    }
    app.shutdown();
    Ok(())
}
