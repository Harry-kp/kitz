pub mod error_boundary;
pub mod layout;
pub mod manager;

pub use error_boundary::ErrorBoundaryState;
pub use layout::PanelLayout;
pub use manager::PanelManager;

/// Unique identifier for a panel — a static string known at compile time.
pub type PanelId = &'static str;

/// A key-binding hint displayed in the footer and the Help overlay.
#[derive(Debug, Clone)]
pub struct KeyHint {
    pub key: &'static str,
    pub desc: &'static str,
}

impl KeyHint {
    pub fn new(key: &'static str, desc: &'static str) -> Self {
        Self { key, desc }
    }
}
