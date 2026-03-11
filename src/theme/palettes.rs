use ratatui::style::Color;

use super::Theme;

pub fn all() -> Vec<Theme> {
    vec![nord(), tokyo_night(), catppuccin_mocha(), dracula()]
}

pub fn nord() -> Theme {
    Theme {
        name: "Nord",
        bg: Color::Rgb(46, 52, 64),
        surface: Color::Rgb(59, 66, 82),
        text: Color::Rgb(216, 222, 233),
        text_muted: Color::Rgb(76, 86, 106),
        border: Color::Rgb(76, 86, 106),
        border_focused: Color::Rgb(136, 192, 208),
        accent: Color::Rgb(136, 192, 208),
        success: Color::Rgb(163, 190, 140),
        warning: Color::Rgb(235, 203, 139),
        error: Color::Rgb(191, 97, 106),
    }
}

pub fn tokyo_night() -> Theme {
    Theme {
        name: "Tokyo Night",
        bg: Color::Rgb(26, 27, 38),
        surface: Color::Rgb(36, 40, 59),
        text: Color::Rgb(169, 177, 214),
        text_muted: Color::Rgb(86, 95, 137),
        border: Color::Rgb(86, 95, 137),
        border_focused: Color::Rgb(125, 174, 163),
        accent: Color::Rgb(125, 174, 163),
        success: Color::Rgb(158, 206, 106),
        warning: Color::Rgb(224, 175, 104),
        error: Color::Rgb(247, 118, 142),
    }
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin",
        bg: Color::Rgb(30, 30, 46),
        surface: Color::Rgb(49, 50, 68),
        text: Color::Rgb(205, 214, 244),
        text_muted: Color::Rgb(108, 112, 134),
        border: Color::Rgb(108, 112, 134),
        border_focused: Color::Rgb(137, 180, 250),
        accent: Color::Rgb(137, 180, 250),
        success: Color::Rgb(166, 218, 149),
        warning: Color::Rgb(249, 226, 175),
        error: Color::Rgb(243, 139, 168),
    }
}

pub fn dracula() -> Theme {
    Theme {
        name: "Dracula",
        bg: Color::Rgb(40, 42, 54),
        surface: Color::Rgb(68, 71, 90),
        text: Color::Rgb(248, 248, 242),
        text_muted: Color::Rgb(98, 114, 164),
        border: Color::Rgb(98, 114, 164),
        border_focused: Color::Rgb(139, 233, 253),
        accent: Color::Rgb(139, 233, 253),
        success: Color::Rgb(80, 250, 123),
        warning: Color::Rgb(241, 250, 140),
        error: Color::Rgb(255, 85, 85),
    }
}
