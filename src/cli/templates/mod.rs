pub mod dashboard;
pub mod editor;
pub mod minimal;
pub mod panels;

/// A single file in a project template.
pub struct TemplateFile {
    pub path: &'static str,
    pub content: &'static str,
}

/// Apply variable substitution to template content.
pub fn substitute(content: &str, project_name: &str) -> String {
    let snake = to_snake_case(project_name);
    let pascal = to_pascal_case(project_name);
    let kebab = to_kebab_case(project_name);

    content
        .replace("{{project_name}}", &snake)
        .replace("{{ProjectName}}", &pascal)
        .replace("{{project-name}}", &kebab)
}

pub fn to_snake_case(s: &str) -> String {
    s.replace(['-', ' '], "_").to_lowercase()
}

pub fn to_pascal_case(s: &str) -> String {
    s.replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

pub fn to_kebab_case(s: &str) -> String {
    s.replace(['_', ' '], "-").to_lowercase()
}

pub fn get_template(name: &str) -> Result<Vec<TemplateFile>, String> {
    match name {
        "minimal" => Ok(minimal::files()),
        "panels" => Ok(panels::files()),
        "dashboard" => Ok(dashboard::files()),
        "editor" => Ok(editor::files()),
        _ => Err(format!(
            "Unknown template '{}'. Available: minimal, panels, dashboard, editor",
            name
        )),
    }
}
