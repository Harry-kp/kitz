use colored::Colorize;

use crate::cli::generators;

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
    {
        return Err(format!(
            "Name '{}' must be snake_case (lowercase, underscores, digits)",
            name
        ));
    }
    if name.starts_with('_') || name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(format!("Name '{}' cannot start with '_' or a digit", name));
    }
    Ok(())
}

pub fn panel(name: &str) -> Result<(), String> {
    validate_name(name)?;

    let root = generators::helpers::find_project_root()?;

    println!(
        "\n{}  Generating panel: {}\n",
        "✦".cyan().bold(),
        name.bold()
    );

    generators::panel::generate(name, &root)?;

    println!(
        "\n  {}  Panel '{}' generated and wired into the project.\n",
        "✓".green().bold(),
        name
    );

    Ok(())
}

pub fn screen(name: &str) -> Result<(), String> {
    validate_name(name)?;

    let root = generators::helpers::find_project_root()?;

    println!(
        "\n{}  Generating screen: {}\n",
        "✦".cyan().bold(),
        name.bold()
    );

    generators::screen::generate(name, &root)?;

    println!(
        "\n  {}  Screen '{}' generated and wired into the project.\n",
        "✓".green().bold(),
        name
    );

    Ok(())
}

pub fn overlay(name: &str) -> Result<(), String> {
    validate_name(name)?;

    let root = generators::helpers::find_project_root()?;

    println!(
        "\n{}  Generating overlay: {}\n",
        "✦".cyan().bold(),
        name.bold()
    );

    generators::overlay::generate(name, &root)?;

    println!(
        "\n  {}  Overlay '{}' generated and wired into the project.\n",
        "✓".green().bold(),
        name
    );

    Ok(())
}
