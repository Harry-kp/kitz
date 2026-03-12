use std::fs;
use std::path::Path;

use super::helpers::*;

/// Generate a new panel and wire it into the project.
pub fn generate(name: &str, project_root: &Path) -> Result<(), String> {
    let pascal = to_pascal(name);

    // 1. Create the panel file
    let panels_dir = project_root.join("src/panels");
    ensure_dir(&panels_dir)?;

    let panel_path = panels_dir.join(format!("{}.rs", name));
    if panel_path.exists() {
        return Err(format!(
            "Panel '{}' already exists at {}",
            name,
            panel_path.display()
        ));
    }

    let panel_content = generate_panel_file(name, &pascal);
    fs::write(&panel_path, panel_content).map_err(|e| format!("Cannot write panel file: {}", e))?;
    print_generated(
        &format!("src/panels/{}.rs", name),
        "panel view + key hints + handle_key",
    );

    // 2. Wire into panels/mod.rs
    let mod_path = panels_dir.join("mod.rs");
    if mod_path.exists() {
        let inserted = insert_above_marker(
            &mod_path,
            "rataframe:panel-mods",
            &format!("pub mod {};", name),
        )?;
        if inserted {
            print_modified("src/panels/mod.rs", &format!("added: pub mod {}", name));
        } else {
            // Fallback: append to file
            let mut content =
                fs::read_to_string(&mod_path).map_err(|e| format!("Cannot read mod.rs: {}", e))?;
            content.push_str(&format!("pub mod {};\n", name));
            fs::write(&mod_path, content).map_err(|e| format!("Cannot write mod.rs: {}", e))?;
            print_modified("src/panels/mod.rs", &format!("appended: pub mod {}", name));
        }
    }

    // 3. Add message variants to messages.rs
    let messages_path = project_root.join("src/messages.rs");
    if messages_path.exists() {
        let msg_lines = format!("{}Next,\n{}Prev,", pascal, pascal);
        let inserted = insert_above_marker(&messages_path, "rataframe:messages", &msg_lines)?;
        if inserted {
            print_modified(
                "src/messages.rs",
                &format!("added: {}Next, {}Prev", pascal, pascal),
            );
        }
    }

    // 4. Wire into app.rs — fields, init, update, panels, title, view, hints, keys
    let app_path = project_root.join("src/app.rs");
    if app_path.exists() {
        // App field
        insert_above_marker(
            &app_path,
            "rataframe:app-fields",
            &format!("pub {}: panels::{}::{}Panel,", name, name, pascal),
        )?;
        print_modified("src/app.rs", &format!("added: {} field", name));

        // App init
        insert_above_marker(
            &app_path,
            "rataframe:app-init",
            &format!("{}: panels::{}::{}Panel::new(),", name, name, pascal),
        )?;

        // Update match arm
        insert_above_marker(
            &app_path,
            "rataframe:update",
            &format!(
                "Msg::{}Next => self.{}.select_next(),\nMsg::{}Prev => self.{}.select_prev(),",
                pascal, name, pascal, name
            ),
        )?;
        print_modified(
            "src/app.rs",
            &format!("added: {}Next/Prev update arms", pascal),
        );

        // Layout
        let existing_count = count_existing_panels(project_root) + 1; // +1 for the new one
        let pct = even_percentage(existing_count);
        insert_above_marker(
            &app_path,
            "rataframe:layout",
            &format!("(\"{}\", Constraint::Percentage({})),", name, pct),
        )?;
        print_modified("src/app.rs", &format!("added: \"{}\" to PanelLayout", name));

        // Panel title
        insert_above_marker(
            &app_path,
            "rataframe:panel-title",
            &format!("\"{}\" => panels::{}::PANEL_TITLE,", name, name),
        )?;

        // Panel view
        insert_above_marker(
            &app_path,
            "rataframe:panel-view",
            &format!(
                "\"{}\" => self.{}.view(frame, area, focused, theme),",
                name, name
            ),
        )?;

        // Panel hints
        insert_above_marker(
            &app_path,
            "rataframe:panel-hints",
            &format!(
                "\"{}\" => panels::{}::{}Panel::key_hints(),",
                name, name, pascal
            ),
        )?;

        // Panel keys
        insert_above_marker(
            &app_path,
            "rataframe:panel-keys",
            &format!(
                "\"{}\" => match key.code {{\n    KeyCode::Char('j') | KeyCode::Down => EventResult::Message(Msg::{}Next),\n    KeyCode::Char('k') | KeyCode::Up => EventResult::Message(Msg::{}Prev),\n    _ => EventResult::Ignored,\n}},",
                name, pascal, pascal
            ),
        )?;
        print_modified(
            "src/app.rs",
            "added: panel_title/view/hints/handle_key arms",
        );
    }

    // 5. Add test
    let test_path = project_root.join("tests/app_test.rs");
    if test_path.exists() {
        insert_above_marker(
            &test_path,
            "rataframe:tests",
            &format!(
                "#[test]\nfn test_{}_exists() {{\n    // TODO: Add TestHarness tests for the {} panel\n    assert!(true);\n}}",
                name, name
            ),
        )?;
        print_modified("tests/app_test.rs", &format!("added: test_{}", name));
    }

    Ok(())
}

fn generate_panel_file(name: &str, pascal: &str) -> String {
    format!(
        r#"use rataframe::prelude::*;
use ratatui::style::{{Modifier, Style}};
use ratatui::text::Line;
use ratatui::widgets::{{List, ListItem}};

pub const PANEL_ID: PanelId = "{}";
pub const PANEL_TITLE: &str = "{}";

pub struct {}Panel {{
    pub items: Vec<String>,
    pub selected: usize,
}}

impl {}Panel {{
    pub fn new() -> Self {{
        Self {{
            items: vec![
                "Item 1".into(),
                "Item 2".into(),
                "Item 3".into(),
            ],
            selected: 0,
        }}
    }}

    pub fn view(&self, frame: &mut Frame, area: Rect, _focused: bool, theme: &rataframe::theme::Theme) {{
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {{
                let style = if i == self.selected {{
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::REVERSED)
                }} else {{
                    Style::default().fg(theme.text)
                }};
                ListItem::new(Line::styled(format!(" {{}} ", item), style))
            }})
            .collect();
        frame.render_widget(List::new(items), area);
    }}

    pub fn key_hints() -> Vec<KeyHint> {{
        vec![
            KeyHint::new("j/k", "Navigate"),
        ]
    }}

    pub fn select_next(&mut self) {{
        if self.selected < self.items.len().saturating_sub(1) {{
            self.selected += 1;
        }}
    }}

    pub fn select_prev(&mut self) {{
        self.selected = self.selected.saturating_sub(1);
    }}
}}
"#,
        name, pascal, pascal, pascal
    )
}
