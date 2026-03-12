pub mod app;
pub mod command;
pub mod context;
pub mod error;
pub mod logging;
pub mod overlay;
pub mod panel;
pub mod prelude;
pub mod runtime;
pub mod screen;
pub mod subscription;
pub mod testing;
pub mod theme;
pub mod toast;
pub mod widgets;

use app::Application;
use color_eyre::Result;

/// Run a rataframe application.
///
/// This is the main entry point. It initialises the terminal, enters the
/// event loop, and guarantees terminal restoration on exit — even on panic.
///
/// # Example
///
/// ```no_run
/// use rataframe::prelude::*;
///
/// struct App;
///
/// impl Application for App {
///     type Message = ();
///     fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
///         Command::quit()
///     }
/// }
///
/// fn main() -> color_eyre::Result<()> {
///     rataframe::run(App)
/// }
/// ```
pub fn run<A: Application>(app: A) -> Result<()> {
    runtime::run(app)
}
