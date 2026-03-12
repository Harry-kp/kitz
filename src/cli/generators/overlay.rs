use std::fs;
use std::path::Path;

use colored::Colorize;

use super::helpers::*;

/// Generate a new overlay and wire it into the project.
pub fn generate(name: &str, project_root: &Path) -> Result<(), String> {
    let pascal = to_pascal(name);

    // 1. Create overlays directory if needed
    let overlays_dir = project_root.join("src/overlays");
    ensure_dir(&overlays_dir)?;

    let mod_path = overlays_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(
            &mod_path,
            format!("pub mod {};\n// kitz:overlay-mods\n", name),
        )
        .map_err(|e| format!("Cannot write overlays/mod.rs: {}", e))?;
        print_generated("src/overlays/mod.rs", "overlay registry");
    } else {
        let inserted = insert_above_marker(
            &mod_path,
            "kitz:overlay-mods",
            &format!("pub mod {};", name),
        )?;
        if !inserted {
            let mut content =
                fs::read_to_string(&mod_path).map_err(|e| format!("Cannot read mod.rs: {}", e))?;
            content.push_str(&format!("pub mod {};\n", name));
            fs::write(&mod_path, content).map_err(|e| format!("Cannot write mod.rs: {}", e))?;
        }
        print_modified("src/overlays/mod.rs", &format!("added: pub mod {}", name));
    }

    // 2. Create the overlay file
    let overlay_path = overlays_dir.join(format!("{}.rs", name));
    if overlay_path.exists() {
        return Err(format!(
            "Overlay '{}' already exists at {}",
            name,
            overlay_path.display()
        ));
    }

    let overlay_content = generate_overlay_file(name, &pascal);
    fs::write(&overlay_path, overlay_content)
        .map_err(|e| format!("Cannot write overlay file: {}", e))?;
    print_generated(
        &format!("src/overlays/{}.rs", name),
        "Overlay impl with view + handle_event",
    );

    // 3. Ensure `mod overlays;` is in main.rs
    let main_path = project_root.join("src/main.rs");
    if main_path.exists() {
        let content =
            fs::read_to_string(&main_path).map_err(|e| format!("Cannot read main.rs: {}", e))?;
        if !content.contains("mod overlays;") {
            let new_content = format!("mod overlays;\n{}", content);
            fs::write(&main_path, new_content)
                .map_err(|e| format!("Cannot write main.rs: {}", e))?;
            print_modified("src/main.rs", "added: mod overlays");
        }
    }

    println!(
        "\n  {}",
        format!(
            "Use ctx.push_overlay(overlays::{}::{}Overlay::new()) from your update() function.",
            name, pascal
        )
        .dimmed()
    );

    Ok(())
}

fn generate_overlay_file(_name: &str, pascal: &str) -> String {
    format!(
        r#"use std::fmt::Debug;

use crossterm::event::{{Event, KeyCode, KeyEvent}};
use kitz::overlay::{{Overlay, OverlayResult}};
use kitz::theme::Theme;
use kitz::widgets::centered_rect;
use ratatui::layout::Rect;
use ratatui::style::{{Modifier, Style}};
use ratatui::text::Line;
use ratatui::widgets::{{Block, Borders, Clear, Paragraph}};
use ratatui::Frame;

pub struct {pascal}Overlay {{
    pub message: String,
}}

impl {pascal}Overlay {{
    pub fn new() -> Self {{
        Self {{
            message: "{pascal} overlay — press Esc to close".into(),
        }}
    }}
}}

impl<M: Debug + Send + 'static> Overlay<M> for {pascal}Overlay {{
    fn title(&self) -> &str {{
        "{pascal}"
    }}

    fn view(&self, frame: &mut Frame, area: Rect, theme: &Theme) {{
        let dialog = centered_rect(50, 30, area);
        frame.render_widget(Clear, dialog);

        let block = Block::default()
            .title(" {pascal} ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(dialog);
        frame.render_widget(block, dialog);

        let lines = vec![
            Line::raw(""),
            Line::styled(
                format!("  {{}}", self.message),
                Style::default().fg(theme.text),
            ),
            Line::raw(""),
            Line::styled(
                "  Esc to close",
                Style::default().fg(theme.text_muted).add_modifier(Modifier::DIM),
            ),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }}

    fn handle_event(&mut self, event: &Event) -> OverlayResult<M> {{
        if let Event::Key(KeyEvent {{ code, .. }}) = event {{
            match code {{
                KeyCode::Esc => return OverlayResult::Close,
                _ => {{}}
            }}
            OverlayResult::Consumed
        }} else {{
            OverlayResult::Consumed
        }}
    }}
}}
"#,
        pascal = pascal,
    )
}
