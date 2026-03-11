use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Initialise TUI-safe file logging.
///
/// Returns a guard that must be held for the lifetime of the application
/// (dropping it flushes pending log writes). Logs go to `~/.local/share/rataframe/`
/// or a custom directory.
///
/// Call this before `rataframe::run()` if you want structured logging.
/// The runtime does NOT call this automatically — it's opt-in to avoid
/// creating files for simple examples.
pub fn init_logging(app_name: &str) -> Option<WorkerGuard> {
    let log_dir = log_directory(app_name);
    std::fs::create_dir_all(&log_dir).ok()?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true),
        )
        .init();

    Some(guard)
}

fn log_directory(app_name: &str) -> PathBuf {
    if let Some(data_dir) = dirs_fallback() {
        data_dir.join("rataframe").join(app_name)
    } else {
        PathBuf::from(".").join(".rataframe-logs")
    }
}

fn dirs_fallback() -> Option<PathBuf> {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
}
