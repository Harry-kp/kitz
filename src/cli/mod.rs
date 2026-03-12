mod commands;
mod generators;
pub mod templates;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "rataframe",
    about = "The application framework for terminal user interfaces",
    version,
    after_help = "Run `rataframe <command> --help` for more information on a command."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new rataframe project
    New {
        /// Project name (also used as the directory name)
        name: String,

        /// Project template
        #[arg(short, long, default_value = "panels")]
        template: String,
    },

    /// Generate a component and wire it into the project
    Generate {
        #[command(subcommand)]
        what: GenerateType,
    },

    /// Start development with auto-reload on file changes
    Dev,

    /// Build and run the application
    Run {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },

    /// Run tests
    Test {
        /// Re-run tests on file changes
        #[arg(long)]
        watch: bool,
    },

    /// Manage themes
    Theme {
        #[command(subcommand)]
        action: ThemeAction,
    },
}

#[derive(Subcommand)]
enum GenerateType {
    /// Generate a panel and wire it into the project
    Panel {
        /// Panel name (snake_case, e.g. "stats" or "file_list")
        name: String,
    },
    /// Generate a screen and wire it into the project
    Screen {
        /// Screen name (snake_case, e.g. "settings" or "detail_view")
        name: String,
    },
    /// Generate an overlay and wire it into the project
    Overlay {
        /// Overlay name (snake_case, e.g. "confirm_delete" or "search")
        name: String,
    },
}

#[derive(Subcommand)]
enum ThemeAction {
    /// Show all built-in themes with color swatches
    List,
    /// Render a sample UI in each theme
    Preview,
}

pub fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name, template } => commands::new::execute(&name, &template),
        Commands::Generate { what } => match what {
            GenerateType::Panel { name } => commands::generate::panel(&name),
            GenerateType::Screen { name } => commands::generate::screen(&name),
            GenerateType::Overlay { name } => commands::generate::overlay(&name),
        },
        Commands::Dev => commands::dev::dev(),
        Commands::Run { release } => commands::dev::run(release),
        Commands::Test { watch } => commands::dev::test(watch),
        Commands::Theme { action } => match action {
            ThemeAction::List => commands::theme::list(),
            ThemeAction::Preview => commands::theme::preview(),
        },
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}
