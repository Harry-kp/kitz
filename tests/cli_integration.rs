#![cfg(feature = "cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn rataframe_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("rataframe");
    path
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rataframe_test_{}", name));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_rataframe(args: &[&str], cwd: &Path) -> (bool, String, String) {
    let output = Command::new(rataframe_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute rataframe");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

// ── rataframe new ──────────────────────────────────────────

#[test]
fn new_creates_project_with_default_template() {
    let dir = temp_dir("new_default");
    let (ok, stdout, _) = run_rataframe(&["new", "testproject"], &dir);

    assert!(ok, "rataframe new should succeed");
    assert!(stdout.contains("Creating new rataframe project"));

    let project = dir.join("testproject");
    assert!(project.join("Cargo.toml").exists());
    assert!(project.join("src/main.rs").exists());
    assert!(project.join("src/app.rs").exists());
    assert!(project.join("src/messages.rs").exists());
    assert!(project.join("src/panels/mod.rs").exists());
    assert!(project.join("src/panels/sidebar.rs").exists());
    assert!(project.join("src/panels/detail.rs").exists());
    assert!(project.join("tests/app_test.rs").exists());
    assert!(project.join(".gitignore").exists());
}

#[test]
fn new_creates_minimal_project() {
    let dir = temp_dir("new_minimal");
    let (ok, _, _) = run_rataframe(&["new", "minapp", "--template", "minimal"], &dir);

    assert!(ok);

    let project = dir.join("minapp");
    assert!(project.join("Cargo.toml").exists());
    assert!(project.join("src/main.rs").exists());
    assert!(!project.join("src/app.rs").exists());
    assert!(!project.join("src/panels").exists());
}

#[test]
fn new_creates_dashboard_project() {
    let dir = temp_dir("new_dashboard");
    let (ok, _, _) = run_rataframe(&["new", "dashapp", "--template", "dashboard"], &dir);

    assert!(ok);

    let project = dir.join("dashapp");
    assert!(project.join("src/panels/stats.rs").exists());
    assert!(project.join("src/panels/chart.rs").exists());
    assert!(project.join("src/panels/log.rs").exists());
}

#[test]
fn new_creates_editor_project() {
    let dir = temp_dir("new_editor");
    let (ok, _, _) = run_rataframe(&["new", "edapp", "--template", "editor"], &dir);

    assert!(ok);

    let project = dir.join("edapp");
    assert!(project.join("src/main.rs").exists());

    let main = fs::read_to_string(project.join("src/main.rs")).unwrap();
    assert!(main.contains("Mode"));
    assert!(main.contains("Insert"));
    assert!(main.contains("Normal"));
}

#[test]
fn new_fails_on_existing_directory() {
    let dir = temp_dir("new_exists");
    fs::create_dir_all(dir.join("existing")).unwrap();

    let (ok, _, _) = run_rataframe(&["new", "existing"], &dir);
    assert!(!ok);
}

#[test]
fn new_fails_on_unknown_template() {
    let dir = temp_dir("new_unknown_tpl");
    let (ok, _, _) = run_rataframe(&["new", "proj", "--template", "nonexistent"], &dir);
    assert!(!ok);
}

#[test]
fn new_applies_name_substitution() {
    let dir = temp_dir("new_substitution");
    let (ok, _, _) = run_rataframe(&["new", "my-cool-app", "--template", "panels"], &dir);

    assert!(ok);

    let project = dir.join("my-cool-app");
    let cargo = fs::read_to_string(project.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"my-cool-app\""));

    let app = fs::read_to_string(project.join("src/app.rs")).unwrap();
    assert!(app.contains("\"my-cool-app\""));
}

// ── rataframe generate panel ────────────────────────────────

fn setup_panels_project(test_name: &str) -> PathBuf {
    let dir = temp_dir(test_name);
    let (ok, _, _) = run_rataframe(&["new", "gentest", "--template", "panels"], &dir);
    assert!(ok, "project setup failed");
    dir.join("gentest")
}

#[test]
fn generate_panel_creates_files() {
    let project = setup_panels_project("gen_panel_files");

    let (ok, stdout, _) = run_rataframe(&["generate", "panel", "stats"], &project);
    assert!(ok, "generate panel should succeed");
    assert!(stdout.contains("Panel 'stats' generated"));

    assert!(project.join("src/panels/stats.rs").exists());

    let panel = fs::read_to_string(project.join("src/panels/stats.rs")).unwrap();
    assert!(panel.contains("StatsPanel"));
    assert!(panel.contains("PANEL_TITLE"));
}

#[test]
fn generate_panel_wires_into_mod() {
    let project = setup_panels_project("gen_panel_mod");

    run_rataframe(&["generate", "panel", "network"], &project);

    let mod_rs = fs::read_to_string(project.join("src/panels/mod.rs")).unwrap();
    assert!(mod_rs.contains("pub mod network;"));
}

#[test]
fn generate_panel_adds_message_variants() {
    let project = setup_panels_project("gen_panel_msgs");

    run_rataframe(&["generate", "panel", "activity"], &project);

    let messages = fs::read_to_string(project.join("src/messages.rs")).unwrap();
    assert!(messages.contains("ActivityNext,"));
    assert!(messages.contains("ActivityPrev,"));
}

#[test]
fn generate_panel_wires_into_app() {
    let project = setup_panels_project("gen_panel_app");

    run_rataframe(&["generate", "panel", "files"], &project);

    let app = fs::read_to_string(project.join("src/app.rs")).unwrap();
    assert!(app.contains("pub files: panels::files::FilesPanel,"));
    assert!(app.contains("Msg::FilesNext"));
    assert!(app.contains("\"files\" => panels::files::PANEL_TITLE"));
    assert!(app.contains("\"files\" => self.files.view"));
}

#[test]
fn generate_panel_fails_on_duplicate() {
    let project = setup_panels_project("gen_panel_dup");

    let (ok1, _, _) = run_rataframe(&["generate", "panel", "logs"], &project);
    assert!(ok1);

    let (ok2, _, _) = run_rataframe(&["generate", "panel", "logs"], &project);
    assert!(!ok2, "duplicate panel should fail");
}

#[test]
fn generate_panel_validates_name() {
    let project = setup_panels_project("gen_panel_name");

    let (ok, _, _) = run_rataframe(&["generate", "panel", "BadName"], &project);
    assert!(!ok, "PascalCase name should be rejected");

    let (ok2, _, _) = run_rataframe(&["generate", "panel", "has-dash"], &project);
    assert!(!ok2, "kebab-case name should be rejected");
}

// ── rataframe generate screen ────────────────────────────────

#[test]
fn generate_screen_creates_files() {
    let project = setup_panels_project("gen_screen");

    let (ok, stdout, _) = run_rataframe(&["generate", "screen", "settings"], &project);
    assert!(ok, "generate screen should succeed");
    assert!(stdout.contains("Screen 'settings' generated"));

    assert!(project.join("src/screens/mod.rs").exists());
    assert!(project.join("src/screens/settings.rs").exists());

    let screen = fs::read_to_string(project.join("src/screens/settings.rs")).unwrap();
    assert!(screen.contains("SettingsScreen"));
    assert!(screen.contains("impl<M:"));
}

#[test]
fn generate_screen_adds_mod_to_main() {
    let project = setup_panels_project("gen_screen_main");

    run_rataframe(&["generate", "screen", "detail_view"], &project);

    let main = fs::read_to_string(project.join("src/main.rs")).unwrap();
    assert!(main.contains("mod screens;"));
}

#[test]
fn generate_screen_adds_message_and_update() {
    let project = setup_panels_project("gen_screen_msg");

    run_rataframe(&["generate", "screen", "profile"], &project);

    let messages = fs::read_to_string(project.join("src/messages.rs")).unwrap();
    assert!(messages.contains("PushProfileScreen,"));

    let app = fs::read_to_string(project.join("src/app.rs")).unwrap();
    assert!(app.contains("Msg::PushProfileScreen"));
    assert!(app.contains("screens::profile::ProfileScreen::new()"));
}

// ── rataframe generate overlay ────────────────────────────────

#[test]
fn generate_overlay_creates_files() {
    let project = setup_panels_project("gen_overlay");

    let (ok, stdout, _) = run_rataframe(&["generate", "overlay", "confirm_delete"], &project);
    assert!(ok, "generate overlay should succeed");
    assert!(stdout.contains("Overlay 'confirm_delete' generated"));

    assert!(project.join("src/overlays/mod.rs").exists());
    assert!(project.join("src/overlays/confirm_delete.rs").exists());

    let overlay = fs::read_to_string(project.join("src/overlays/confirm_delete.rs")).unwrap();
    assert!(overlay.contains("ConfirmDeleteOverlay"));
    assert!(overlay.contains("impl<M:"));
}

#[test]
fn generate_overlay_adds_mod_to_main() {
    let project = setup_panels_project("gen_overlay_main");

    run_rataframe(&["generate", "overlay", "search"], &project);

    let main = fs::read_to_string(project.join("src/main.rs")).unwrap();
    assert!(main.contains("mod overlays;"));
}

// ── Template substitution ────────────────────────────────────

#[test]
fn template_substitution_handles_underscored_names() {
    let dir = temp_dir("subst_underscore");
    let (ok, _, _) = run_rataframe(&["new", "my_project", "--template", "panels"], &dir);

    assert!(ok);

    let project = dir.join("my_project");
    let cargo = fs::read_to_string(project.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"my-project\""));
}

// ── CLI help and version ────────────────────────────────────

#[test]
fn cli_shows_help() {
    let dir = temp_dir("help");
    let (ok, stdout, _) = run_rataframe(&["--help"], &dir);

    assert!(ok);
    assert!(stdout.contains("terminal user interfaces"));
    assert!(stdout.contains("new"));
    assert!(stdout.contains("generate"));
    assert!(stdout.contains("dev"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("theme"));
}

#[test]
fn cli_shows_version() {
    let dir = temp_dir("version");
    let (ok, stdout, _) = run_rataframe(&["--version"], &dir);

    assert!(ok);
    assert!(stdout.contains("rataframe"));
}

// ── Generator helpers unit tests ────────────────────────────

#[test]
fn template_to_snake_case() {
    use rataframe::cli::templates::to_snake_case;

    assert_eq!(to_snake_case("my-project"), "my_project");
    assert_eq!(to_snake_case("MyProject"), "myproject");
    assert_eq!(to_snake_case("hello world"), "hello_world");
}

#[test]
fn template_to_pascal_case() {
    use rataframe::cli::templates::to_pascal_case;

    assert_eq!(to_pascal_case("my_project"), "MyProject");
    assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
    assert_eq!(to_pascal_case("single"), "Single");
}

#[test]
fn template_to_kebab_case() {
    use rataframe::cli::templates::to_kebab_case;

    assert_eq!(to_kebab_case("my_project"), "my-project");
    assert_eq!(to_kebab_case("hello world"), "hello-world");
}
