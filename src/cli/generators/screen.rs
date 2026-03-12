use std::fs;
use std::path::Path;

use colored::Colorize;

use super::helpers::*;

/// Generate a new screen and wire it into the project.
pub fn generate(name: &str, project_root: &Path) -> Result<(), String> {
    let pascal = to_pascal(name);

    // 1. Create screens directory if needed
    let screens_dir = project_root.join("src/screens");
    ensure_dir(&screens_dir)?;

    // Create or update mod.rs
    let mod_path = screens_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(
            &mod_path,
            format!("pub mod {};\n// rataframe:screen-mods\n", name),
        )
        .map_err(|e| format!("Cannot write screens/mod.rs: {}", e))?;
        print_generated("src/screens/mod.rs", "screen registry");
    } else {
        let inserted = insert_above_marker(
            &mod_path,
            "rataframe:screen-mods",
            &format!("pub mod {};", name),
        )?;
        if !inserted {
            let mut content =
                fs::read_to_string(&mod_path).map_err(|e| format!("Cannot read mod.rs: {}", e))?;
            content.push_str(&format!("pub mod {};\n", name));
            fs::write(&mod_path, content).map_err(|e| format!("Cannot write mod.rs: {}", e))?;
        }
        print_modified("src/screens/mod.rs", &format!("added: pub mod {}", name));
    }

    // 2. Create the screen file
    let screen_path = screens_dir.join(format!("{}.rs", name));
    if screen_path.exists() {
        return Err(format!(
            "Screen '{}' already exists at {}",
            name,
            screen_path.display()
        ));
    }

    let screen_content = generate_screen_file(name, &pascal);
    fs::write(&screen_path, screen_content)
        .map_err(|e| format!("Cannot write screen file: {}", e))?;
    print_generated(
        &format!("src/screens/{}.rs", name),
        "Screen impl with panel and lifecycle",
    );

    // 3. Ensure `mod screens;` is in main.rs
    let main_path = project_root.join("src/main.rs");
    if main_path.exists() {
        let content =
            fs::read_to_string(&main_path).map_err(|e| format!("Cannot read main.rs: {}", e))?;
        if !content.contains("mod screens;") {
            let new_content = format!("mod screens;\n{}", content);
            fs::write(&main_path, new_content)
                .map_err(|e| format!("Cannot write main.rs: {}", e))?;
            print_modified("src/main.rs", "added: mod screens");
        }
    }

    // 4. Add push_screen message to messages.rs
    let messages_path = project_root.join("src/messages.rs");
    if messages_path.exists() {
        let msg = format!("Push{}Screen,", pascal);
        insert_above_marker(&messages_path, "rataframe:messages", &msg)?;
        print_modified("src/messages.rs", &format!("added: Push{}Screen", pascal));
    }

    // 5. Add update match arm in app.rs + ensure `use crate::screens;`
    let app_path = project_root.join("src/app.rs");
    if app_path.exists() {
        let app_content =
            fs::read_to_string(&app_path).map_err(|e| format!("Cannot read app.rs: {}", e))?;
        if !app_content.contains("use crate::screens") {
            let new_app = app_content.replacen(
                "use crate::panels;",
                "use crate::panels;\nuse crate::screens;",
                1,
            );
            fs::write(&app_path, new_app).map_err(|e| format!("Cannot write app.rs: {}", e))?;
            print_modified("src/app.rs", "added: use crate::screens");
        }

        insert_above_marker(
            &app_path,
            "rataframe:update",
            &format!(
                "Msg::Push{}Screen => ctx.push_screen(screens::{}::{}Screen::new()),",
                pascal, name, pascal
            ),
        )?;
        print_modified(
            "src/app.rs",
            &format!("added: Push{}Screen update arm", pascal),
        );
    }

    println!(
        "\n  {}",
        format!(
            "Use ctx.push_screen(screens::{}::{}Screen::new()) to navigate to this screen.",
            name, pascal
        )
        .dimmed()
    );

    Ok(())
}

fn generate_screen_file(name: &str, pascal: &str) -> String {
    format!(
        r#"use rataframe::prelude::*;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub struct {pascal}Screen {{
    pub title: String,
}}

impl {pascal}Screen {{
    pub fn new() -> Self {{
        Self {{
            title: "{pascal}".into(),
        }}
    }}
}}

impl<M: std::fmt::Debug + Send + 'static> Screen<M> for {pascal}Screen {{
    fn id(&self) -> &str {{
        "{name}"
    }}

    fn panels(&self) -> PanelLayout {{
        PanelLayout::single("main")
    }}

    fn panel_title(&self, _id: PanelId) -> &str {{
        &self.title
    }}

    fn panel_view(&self, _id: PanelId, frame: &mut Frame, area: Rect, _focused: bool) {{
        let text = format!(" {{}} screen — press Esc to go back", self.title);
        frame.render_widget(
            Paragraph::new(text).style(Style::default()),
            area,
        );
    }}

    fn on_enter(&mut self) {{
        // Called when this screen becomes active
    }}

    fn on_leave(&mut self) {{
        // Called when navigating away from this screen
    }}
}}
"#,
        pascal = pascal,
        name = name,
    )
}
