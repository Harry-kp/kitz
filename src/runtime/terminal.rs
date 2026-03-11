use std::io::{self, stdout};
use std::panic;

use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialise the terminal: raw mode, alternate screen, and a panic hook that
/// guarantees restoration.
pub fn init() -> Result<Tui> {
    install_panic_hook();
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state.
pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing the panic
/// message. Without this, a panic inside a TUI app leaves the terminal in raw
/// mode — unusable until `reset`.
fn install_panic_hook() {
    let original = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = restore();
        original(info);
    }));
}
