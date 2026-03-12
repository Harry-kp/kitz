use std::fmt;
use std::io;

/// All errors produced by the rataframe runtime.
#[derive(Debug)]
pub enum RataframeError {
    /// Terminal initialization or restoration failed.
    Terminal(io::Error),
    /// Event polling or reading failed.
    Event(io::Error),
    /// Rendering to the terminal failed.
    Render(io::Error),
    /// color-eyre installation failed (usually means it was called twice).
    Setup(String),
}

impl fmt::Display for RataframeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminal(e) => write!(
                f,
                "Failed to initialize terminal. Is this running in a real terminal? ({})",
                e
            ),
            Self::Event(e) => write!(f, "Failed to read terminal event: {}", e),
            Self::Render(e) => write!(f, "Failed to render frame: {}", e),
            Self::Setup(msg) => write!(f, "Framework setup error: {}", msg),
        }
    }
}

impl std::error::Error for RataframeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Terminal(e) | Self::Event(e) | Self::Render(e) => Some(e),
            Self::Setup(_) => None,
        }
    }
}

impl From<io::Error> for RataframeError {
    fn from(e: io::Error) -> Self {
        Self::Terminal(e)
    }
}

/// Convenience type alias for rataframe results.
pub type Result<T> = std::result::Result<T, RataframeError>;
