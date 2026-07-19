//! franz - an IAM-auth-native, multi-environment terminal UI for AWS MSK.
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
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event};

use crate::app::App;
use crate::config::Config;

/// franz — your Kafka desk clerk.
///
/// A terminal UI for AWS MSK with IAM auth, multi-environment switching, and
/// live topic / consumer-group inspection. Run with no arguments to launch the
/// TUI. Config is read from ./franz.toml or ~/.config/franz/config.toml.
#[derive(Parser)]
#[command(name = "franz", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Diagnose connectivity for an environment (TCP → IAM token → SASL_SSL
    /// handshake), with verbose librdkafka logs. No TUI.
    Doctor {
        /// Environment name from your config (defaults to the first).
        env: Option<String>,
    },
}

fn main() -> Result<()> {
    // clap handles --help/--version/bad-args and exits before we touch config.
    let cli = Cli::parse();

    let config = Config::load()?;

    match cli.command {
        Some(Command::Doctor { env }) => {
            let env = match env {
                Some(name) => config
                    .envs
                    .iter()
                    .find(|e| e.name == name)
                    .with_context(|| format!("no env named '{name}' in config"))?,
                None => &config.envs[0],
            };
            kafka::doctor(env);
            Ok(())
        }
        None => {
            let mut app = App::new(config);
            let mut terminal = ratatui::init();
            let result = run(&mut terminal, &mut app);
            ratatui::restore();
            result
        }
    }
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
