//! Branding: name, tagline, wordmark. One place to change identity.

pub const NAME: &str = "kitz";
pub const TAGLINE: &str = "your Kafka desk clerk";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ASCII wordmark for the landing screen.
pub const WORDMARK: &[&str] = &["  █▄▀ █ ▀█▀ ▀█", "  █ █ █  █  █▄"];
