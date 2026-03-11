use ratatui::style::Color;

/// Semantic color theme for all framework widgets and panels.
///
/// Every built-in overlay, footer, and panel border respects these colors.
/// Override `Application::theme()` to customise. Full palette support (Nord,
/// Tokyo Night, etc.) lands in Phase 6.
#[derive(Debug, Clone)]
pub struct Theme {
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
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            text: Color::White,
            text_muted: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            accent: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
        }
    }
}
