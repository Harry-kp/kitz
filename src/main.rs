//! kitz - an IAM-auth-native, multi-environment terminal UI for AWS MSK.
//!
//! Launch → pick an environment → inspect topics, partitions, consumer groups,
//! peek events, and do topic admin (create / add partitions / delete) with a
//! prod guardrail. Auth is MSK IAM (SASL OAUTHBEARER) using your ~/.aws creds.

mod app;
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

/// kitz — your Kafka desk clerk.
///
/// A terminal UI for AWS MSK with IAM auth, multi-environment switching, and
/// live topic / consumer-group inspection. Run with no arguments to launch the
/// TUI. Config is read from ./kitz.toml or ~/.config/kitz/config.toml.
#[derive(Parser)]
#[command(name = "kitz", version, about, long_about = None)]
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
                    .with_context(|| {
                        let names: Vec<_> = config.envs.iter().map(|e| e.name.as_str()).collect();
                        format!(
                            "no env named '{name}' in config (have: {})",
                            names.join(", ")
                        )
                    })?,
                None => &config.envs[0],
            };
            anyhow::ensure!(kafka::doctor(env), "doctor found problems (see above)");
            Ok(())
        }
        None => {
            let restore_stderr = stderr_to_log_file();
            let mut app = App::new(config);
            let mut terminal = ratatui::init();
            let result = run(&mut terminal, &mut app);
            ratatui::restore();
            restore_stderr();
            result
        }
    }
}

/// librdkafka logs straight to stderr (rdkafka never enables its log events),
/// which would paint over the TUI. Point fd 2 at kitz.log for the session and
/// return a closure that puts it back. A panic also restores it first so the
/// message stays visible after ratatui's hook leaves the alt screen.
fn stderr_to_log_file() -> impl Fn() {
    use std::os::fd::AsRawFd;
    // SAFETY: plain fd juggling on fd 2; `saved` is a fresh dup we own.
    let saved = unsafe { libc::dup(2) };
    if let Some(log) = kafka::open_log_file() {
        unsafe { libc::dup2(log.as_raw_fd(), 2) };
    }
    let restore = move || {
        if saved >= 0 {
            unsafe { libc::dup2(saved, 2) };
        }
    };
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        prev(info);
    }));
    restore
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
