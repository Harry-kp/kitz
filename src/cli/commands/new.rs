use std::fs;
use std::path::Path;
use std::process::Command;

use colored::Colorize;

use crate::cli::templates;

pub fn execute(name: &str, template: &str) -> Result<(), String> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    println!(
        "\n{}  Creating new kitz project: {}",
        "✦".cyan().bold(),
        name.bold()
    );
    println!("   template: {}\n", template.cyan());

    let files = templates::get_template(template)?;

    for file in &files {
        let content = templates::substitute(file.content, name);
        let full_path = project_dir.join(file.path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
        }

        fs::write(&full_path, content)
            .map_err(|e| format!("Cannot write {}: {}", full_path.display(), e))?;

        println!("  {}  {}", "create".green(), file.path);
    }

    // git init
    let git_result = Command::new("git")
        .arg("init")
        .current_dir(project_dir)
        .output();

    match git_result {
        Ok(output) if output.status.success() => {
            println!("  {}  git repository", "init".green());
        }
        _ => {
            println!("  {}  git init (git not found, skipping)", "skip".yellow());
        }
    }

    // cargo check
    println!("\n  {} Running cargo check...", "●".dimmed());
    let check_result = Command::new("cargo")
        .arg("check")
        .current_dir(project_dir)
        .output();

    match check_result {
        Ok(output) if output.status.success() => {
            println!("  {}  Project compiles successfully!", "✓".green().bold());
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "  {}  Project created but cargo check had warnings:\n{}",
                "⚠".yellow().bold(),
                stderr.dimmed()
            );
        }
        Err(_) => {
            println!(
                "  {}  cargo not found — install Rust from https://rustup.rs",
                "✗".red().bold()
            );
        }
    }

    // Print getting-started instructions
    println!("\n{}", "━".repeat(50).dimmed());
    println!("\n  {}  Your project is ready.\n", "Done!".green().bold());
    println!("  Get started:");
    println!("    {}  cd {}", "→".cyan(), name);
    println!("    {}  cargo run", "→".cyan());
    println!();
    println!("  Or use the kitz CLI:");
    println!(
        "    {}  kitz dev          {}",
        "→".cyan(),
        "(auto-reload)".dimmed()
    );
    println!("    {}  kitz generate panel <name>", "→".cyan());
    println!("    {}  kitz generate screen <name>", "→".cyan());
    println!("    {}  kitz generate overlay <name>", "→".cyan());
    println!(
        "    {}  kitz test         {}",
        "→".cyan(),
        "(run tests)".dimmed()
    );
    println!(
        "    {}  kitz theme list   {}",
        "→".cyan(),
        "(see themes)".dimmed()
    );
    println!();

    Ok(())
}
