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

use anyhow::Result;
use clap::{Parser, Subcommand};
use ratatui::crossterm::event::{self, Event};

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
    /// Config file to use instead of ./kitz.toml / ~/.config/kitz/config.toml.
    #[arg(long, short, global = true, value_name = "PATH")]
    config: Option<std::path::PathBuf>,

    /// Environment to open straight away (skips the picker).
    env: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Create a starter config at ~/.config/kitz/config.toml.
    Init {
        /// Overwrite an existing config.
        #[arg(long)]
        force: bool,
    },
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

    if let Some(Command::Init { force }) = cli.command {
        let path = Config::init(force)?;
        println!("created {}", path.display());
        println!("next: edit it to add your clusters, then run `kitz`");
        return Ok(());
    }

    let config = Config::load(cli.config.as_deref())?;

    match cli.command {
        Some(Command::Init { .. }) => unreachable!("handled above"),
        Some(Command::Doctor { env }) => {
            let idx = env.map_or(Ok(0), |name| config.env_index(&name))?;
            anyhow::ensure!(
                kafka::doctor(&config.envs[idx]),
                "doctor found problems (see above)"
            );
            Ok(())
        }
        None => {
            // `kitz stag` opens stag; a single-env config needs no picker.
            let start = match &cli.env {
                Some(name) => Some(config.env_index(name)?),
                None => (config.envs.len() == 1).then_some(0),
            };
            use std::io::IsTerminal;
            anyhow::ensure!(
                std::io::stdout().is_terminal(),
                "kitz needs an interactive terminal (for scripts and CI use `kitz doctor`)"
            );
            let restore_stderr = stderr_to_log_file();
            let mut app = App::new(config);
            if let Some(i) = start {
                app.connect_to(i);
            }
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

        if event::poll(Duration::from_millis(100))? {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("kitz").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn env_argument_and_subcommands_coexist_with_global_config() {
        let c = parse(&["-c", "k.toml", "doctor", "stag"]);
        assert!(matches!(c.command, Some(Command::Doctor { env: Some(ref e) }) if e == "stag"));
        assert_eq!(c.config.as_deref(), Some(std::path::Path::new("k.toml")));
        let c = parse(&["-c", "k.toml", "stag"]);
        assert_eq!(c.env.as_deref(), Some("stag"));
        assert!(c.command.is_none());
        assert!(matches!(
            parse(&["init"]).command,
            Some(Command::Init { force: false })
        ));
    }
}
