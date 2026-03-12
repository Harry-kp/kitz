use colored::Colorize;

struct ThemeInfo {
    name: &'static str,
    description: &'static str,
    background: (u8, u8, u8),
    surface: (u8, u8, u8),
    text: (u8, u8, u8),
    text_muted: (u8, u8, u8),
    accent: (u8, u8, u8),
    success: (u8, u8, u8),
    warning: (u8, u8, u8),
    error: (u8, u8, u8),
    border: (u8, u8, u8),
}

fn themes() -> Vec<ThemeInfo> {
    vec![
        ThemeInfo {
            name: "Nord",
            description: "Calm, arctic-inspired palette",
            background: (46, 52, 64),
            surface: (59, 66, 82),
            text: (216, 222, 233),
            text_muted: (76, 86, 106),
            accent: (136, 192, 208),
            success: (163, 190, 140),
            warning: (235, 203, 139),
            error: (191, 97, 106),
            border: (67, 76, 94),
        },
        ThemeInfo {
            name: "Tokyo Night",
            description: "Vibrant dark theme inspired by Tokyo city lights",
            background: (26, 27, 38),
            surface: (36, 40, 59),
            text: (169, 177, 214),
            text_muted: (65, 72, 104),
            accent: (122, 162, 247),
            success: (115, 218, 202),
            warning: (224, 175, 104),
            error: (247, 118, 142),
            border: (41, 46, 66),
        },
        ThemeInfo {
            name: "Catppuccin Mocha",
            description: "Warm, cozy, pastel dark theme",
            background: (30, 30, 46),
            surface: (49, 50, 68),
            text: (205, 214, 244),
            text_muted: (108, 112, 134),
            accent: (137, 180, 250),
            success: (166, 227, 161),
            warning: (249, 226, 175),
            error: (243, 139, 168),
            border: (69, 71, 90),
        },
        ThemeInfo {
            name: "Dracula",
            description: "Iconic dark theme with vivid highlights",
            background: (40, 42, 54),
            surface: (68, 71, 90),
            text: (248, 248, 242),
            text_muted: (98, 114, 164),
            accent: (139, 233, 253),
            success: (80, 250, 123),
            warning: (241, 250, 140),
            error: (255, 85, 85),
            border: (68, 71, 90),
        },
    ]
}

fn color_swatch(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[48;2;{};{};{}m   \x1b[0m", r, g, b)
}

fn color_text(text: &str, r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

pub fn list() -> Result<(), String> {
    println!("\n{}  Built-in Themes\n", "✦".cyan().bold());

    for theme in themes() {
        println!("  {}  {}", theme.name.bold(), theme.description.dimmed());
        println!(
            "    bg       {}  surface  {}  border   {}",
            color_swatch(theme.background.0, theme.background.1, theme.background.2),
            color_swatch(theme.surface.0, theme.surface.1, theme.surface.2),
            color_swatch(theme.border.0, theme.border.1, theme.border.2),
        );
        println!(
            "    text     {}  muted    {}  accent   {}",
            color_swatch(theme.text.0, theme.text.1, theme.text.2),
            color_swatch(theme.text_muted.0, theme.text_muted.1, theme.text_muted.2),
            color_swatch(theme.accent.0, theme.accent.1, theme.accent.2),
        );
        println!(
            "    success  {}  warning  {}  error    {}",
            color_swatch(theme.success.0, theme.success.1, theme.success.2),
            color_swatch(theme.warning.0, theme.warning.1, theme.warning.2),
            color_swatch(theme.error.0, theme.error.1, theme.error.2),
        );
        println!();
    }

    println!(
        "  {}",
        "Set theme: self.theme = Theme::nord() (or tokyo_night, catppuccin_mocha, dracula)"
            .dimmed()
    );
    println!();

    Ok(())
}

pub fn preview() -> Result<(), String> {
    println!("\n{}  Theme Previews\n", "✦".cyan().bold());

    for theme in themes() {
        let _bg = theme.background;
        let border = theme.border;
        let text = theme.text;
        let muted = theme.text_muted;
        let accent = theme.accent;
        let success = theme.success;
        let warning = theme.warning;
        let error = theme.error;

        // Top border
        println!(
            "  {}{}{}",
            color_text("┌", border.0, border.1, border.2),
            color_text(&"─".repeat(48), border.0, border.1, border.2),
            color_text("┐", border.0, border.1, border.2),
        );

        // Title bar
        println!(
            "  {}  {}{} {}",
            color_text("│", border.0, border.1, border.2),
            color_text(theme.name, accent.0, accent.1, accent.2),
            " ".repeat(48 - theme.name.len() - 1),
            color_text("│", border.0, border.1, border.2),
        );

        // Separator
        println!(
            "  {}{}{}",
            color_text("├", border.0, border.1, border.2),
            color_text(&"─".repeat(48), border.0, border.1, border.2),
            color_text("┤", border.0, border.1, border.2),
        );

        // Content lines
        let content_lines = [
            ("  Normal text here".to_string(), text),
            ("  Muted secondary text".to_string(), muted),
            ("  ● Accent / Focus element".to_string(), accent),
            ("  ✓ Success    ⚠ Warning    ✗ Error".to_string(), text),
        ];

        for (line, color) in &content_lines {
            let padding = 48_i32 - line.len() as i32;
            let pad = if padding > 0 {
                " ".repeat(padding as usize)
            } else {
                String::new()
            };
            println!(
                "  {} {}{}{}",
                color_text("│", border.0, border.1, border.2),
                color_text(line, color.0, color.1, color.2),
                pad,
                color_text("│", border.0, border.1, border.2),
            );
        }

        // Special line for success/warning/error colors
        let status_line = format!(
            "    {}  {}  {}",
            color_text("✓ OK", success.0, success.1, success.2),
            color_text("⚠ WARN", warning.0, warning.1, warning.2),
            color_text("✗ ERR", error.0, error.1, error.2),
        );
        println!(
            "  {}  {}{}",
            color_text("│", border.0, border.1, border.2),
            status_line,
            color_text("│", border.0, border.1, border.2),
        );

        // Bottom border
        println!(
            "  {}{}{}",
            color_text("└", border.0, border.1, border.2),
            color_text(&"─".repeat(48), border.0, border.1, border.2),
            color_text("┘", border.0, border.1, border.2),
        );

        println!();
    }

    Ok(())
}
