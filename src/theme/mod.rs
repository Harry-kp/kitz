pub mod palettes;

use ratatui::style::Color;

/// Semantic color theme for all framework widgets and panels.
///
/// Every built-in overlay, footer, panel border, and toast respects these
/// colors. Override `Application::theme()` to customise.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub text_muted: Color,
    pub border: Color,
    pub border_focused: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl Default for Theme {
    fn default() -> Self {
        palettes::nord()
    }
}

impl Theme {
    /// Cycle to the next built-in palette.
    pub fn next(&self) -> Self {
        let all = palettes::all();
        let idx = all
            .iter()
            .position(|t| t.name == self.name)
            .map(|i| (i + 1) % all.len())
            .unwrap_or(0);
        all.into_iter().nth(idx).unwrap()
    }
}
