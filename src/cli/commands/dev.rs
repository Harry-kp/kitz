use std::process::Command;

use colored::Colorize;

pub fn dev() -> Result<(), String> {
    // Check if cargo-watch is installed
    let which = Command::new("cargo").args(["watch", "--version"]).output();

    match which {
        Ok(output) if output.status.success() => {}
        _ => {
            println!(
                "  {}  cargo-watch is not installed. Installing...",
                "●".dimmed()
            );
            let install = Command::new("cargo")
                .args(["install", "cargo-watch"])
                .status()
                .map_err(|e| format!("Failed to install cargo-watch: {}", e))?;

            if !install.success() {
                return Err(
                    "Failed to install cargo-watch. Install manually: cargo install cargo-watch"
                        .into(),
                );
            }
            println!("  {}  cargo-watch installed.", "✓".green().bold());
        }
    }

    println!(
        "\n{}  Starting dev mode (auto-reload on changes)...\n",
        "✦".cyan().bold()
    );

    let status = Command::new("cargo")
        .args(["watch", "-x", "run", "-c"])
        .status()
        .map_err(|e| format!("Failed to run cargo-watch: {}", e))?;

    if !status.success() {
        return Err("cargo-watch exited with an error".into());
    }

    Ok(())
}

pub fn run(release: bool) -> Result<(), String> {
    println!("\n{}  Building and running...\n", "✦".cyan().bold());

    let mut args = vec!["run"];
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !status.success() {
        return Err("cargo run exited with an error".into());
    }

    Ok(())
}

pub fn test(watch: bool) -> Result<(), String> {
    if watch {
        // Check for cargo-watch
        let which = Command::new("cargo").args(["watch", "--version"]).output();

        if !matches!(which, Ok(ref o) if o.status.success()) {
            println!(
                "  {}  Installing cargo-watch for test watching...",
                "●".dimmed()
            );
            Command::new("cargo")
                .args(["install", "cargo-watch"])
                .status()
                .map_err(|e| format!("Failed to install cargo-watch: {}", e))?;
        }

        println!("\n{}  Running tests in watch mode...\n", "✦".cyan().bold());

        let status = Command::new("cargo")
            .args(["watch", "-x", "test", "-c"])
            .status()
            .map_err(|e| format!("Failed to run cargo-watch: {}", e))?;

        if !status.success() {
            return Err("cargo-watch exited with an error".into());
        }
    } else {
        println!("\n{}  Running tests...\n", "✦".cyan().bold());

        let status = Command::new("cargo")
            .args(["test"])
            .status()
            .map_err(|e| format!("Failed to run cargo: {}", e))?;

        if !status.success() {
            return Err("Tests failed".into());
        }

        println!("\n  {}  All tests passed!\n", "✓".green().bold());
    }

    Ok(())
}
