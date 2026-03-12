use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

/// Find the project root by looking for Cargo.toml with a kitz dependency.
pub fn find_project_root() -> Result<PathBuf, String> {
    let mut dir = std::env::current_dir().map_err(|e| format!("Cannot get current dir: {}", e))?;

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)
                .map_err(|e| format!("Cannot read Cargo.toml: {}", e))?;
            if content.contains("kitz") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            return Err("Not inside a kitz project. Run `kitz new <name>` first.".into());
        }
    }
}

/// Insert text above a marker comment in a file.
/// Returns true if the marker was found and insertion was made.
pub fn insert_above_marker(
    file_path: &Path,
    marker: &str,
    new_content: &str,
) -> Result<bool, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Cannot read {}: {}", file_path.display(), e))?;

    let marker_line = format!("// {}", marker);
    if let Some(pos) = content.find(&marker_line) {
        let indent = detect_indent(&content, pos);
        let indented_content = new_content
            .lines()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("{}{}", indent, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let new_file = format!(
            "{}{}\n{}{}",
            &content[..pos],
            indented_content,
            &indent,
            &content[pos..]
        );

        fs::write(file_path, new_file)
            .map_err(|e| format!("Cannot write {}: {}", file_path.display(), e))?;

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Detect the indentation at a given byte position by looking backwards
/// to the start of the line.
fn detect_indent(content: &str, pos: usize) -> String {
    let before = &content[..pos];
    if let Some(newline_pos) = before.rfind('\n') {
        let line_start = &before[newline_pos + 1..];
        let indent_len = line_start.len() - line_start.trim_start().len();
        line_start[..indent_len].to_string()
    } else {
        String::new()
    }
}

/// Convert a snake_case name to PascalCase.
pub fn to_pascal(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

/// Print a generated file entry.
pub fn print_generated(path: &str, description: &str) {
    println!("  {}  {}", path.green(), description.dimmed());
}

/// Print a modified file entry.
pub fn print_modified(path: &str, description: &str) {
    println!("  {}  {}", path.yellow(), description.dimmed());
}

/// Ensure a directory exists, creating it if needed.
pub fn ensure_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create directory {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Recalculate panel layout percentages to distribute evenly.
pub fn even_percentage(panel_count: usize) -> u16 {
    if panel_count == 0 {
        100
    } else {
        (100 / panel_count as u16).max(10)
    }
}

/// Count existing panels by counting lines matching a pattern in panels/mod.rs.
pub fn count_existing_panels(project_root: &Path) -> usize {
    let mod_path = project_root.join("src/panels/mod.rs");
    if let Ok(content) = fs::read_to_string(mod_path) {
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("pub mod ") && !trimmed.contains("kitz:")
            })
            .count()
    } else {
        0
    }
}
