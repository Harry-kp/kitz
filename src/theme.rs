//! Color palette borrowed from vortix (Synthwave): slate panels, cyan accents,
//! coral/emerald status. Kept as a flat set of consts - the app is small enough
//! not to need runtime theme switching yet.

use ratatui::style::Color;

// Backgrounds. Dashboard panels are transparent outlines on APP_BG (vortix
// look); PANEL_BG is only for opaque overlays (modals, toast, connecting).
pub const APP_BG: Color = Color::Rgb(15, 23, 36);
pub const PANEL_BG: Color = Color::Rgb(30, 41, 59);

// Separator between header/footer segments (vortix nord polar-night-4).
pub const SEPARATOR: Color = Color::Rgb(76, 86, 106);

// Accents
pub const ACCENT: Color = Color::Rgb(6, 182, 212); // cyan
pub const ACCENT_LIGHT: Color = Color::Rgb(34, 211, 238);

// Status
pub const SUCCESS: Color = Color::Rgb(16, 185, 129); // emerald
pub const WARNING: Color = Color::Rgb(245, 158, 11); // amber
pub const ERROR: Color = Color::Rgb(239, 68, 68); // coral
pub const INACTIVE: Color = Color::Gray;

// Text
pub const TEXT: Color = Color::Rgb(248, 250, 252);
pub const TEXT_MUTED: Color = Color::Rgb(148, 163, 184);

// Borders / rows
pub const BORDER: Color = Color::Rgb(71, 85, 105);
pub const BORDER_FOCUSED: Color = ACCENT;
pub const ROW_SELECTED_BG: Color = Color::Rgb(40, 55, 75);
pub const ROW_SELECTED_FG: Color = ACCENT_LIGHT;

// Footer hints
pub const KEY_HINT: Color = ACCENT;

/// Coral for a prod-tagged env, emerald otherwise. The one guardrail that
/// matters: prod always reads red on screen.
pub fn env_color(is_prod: bool) -> Color {
    if is_prod {
        ERROR
    } else {
        SUCCESS
    }
}
