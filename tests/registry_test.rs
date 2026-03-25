use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::path::PathBuf;

// Import the library's modules
use gsd_manager::config::{load_config, save_config, Config};
use gsd_manager::registry::{add_project, list_projects, remove_project};

#[test]
fn add_project_with_valid_alias_and_planning_dir_succeeds() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    let result = add_project(&mut config, "myapp", temp.path());
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    assert!(config.projects.contains_key("myapp"));
    assert_eq!(config.projects["myapp"].path, temp.path());
}

#[test]
fn add_project_with_duplicate_alias_returns_error() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, "myapp", temp.path()).unwrap();

    let result = add_project(&mut config, "myapp", temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("already exists"),
        "Expected 'already exists' in error, got: {}",
        err_msg
    );
}

#[test]
fn add_project_with_empty_alias_returns_error() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    let result = add_project(&mut config, "", temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("cannot be empty"),
        "Expected 'cannot be empty' in error, got: {}",
        err_msg
    );
}

#[test]
fn add_project_with_whitespace_alias_returns_error() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    let result = add_project(&mut config, "my app", temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("whitespace"),
        "Expected 'whitespace' in error, got: {}",
        err_msg
    );
}

#[test]
fn add_project_with_path_missing_planning_returns_error() {
    let temp = TempDir::new().unwrap();
    // No .planning/ directory created

    let mut config = Config::new();
    let result = add_project(&mut config, "myapp", temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No .planning/ directory found"),
        "Expected 'No .planning/ directory found' in error, got: {}",
        err_msg
    );
}

#[test]
fn add_project_with_nonexistent_path_returns_error() {
    let mut config = Config::new();
    let fake_path = PathBuf::from("/nonexistent/path/to/project");
    let result = add_project(&mut config, "myapp", &fake_path);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "Expected 'does not exist' in error, got: {}",
        err_msg
    );
}

#[test]
fn remove_project_with_existing_alias_succeeds() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, "myapp", temp.path()).unwrap();

    let result = remove_project(&mut config, "myapp");
    assert!(result.is_ok());
    assert!(!config.projects.contains_key("myapp"));
}

#[test]
fn remove_project_with_nonexistent_alias_returns_error() {
    let mut config = Config::new();
    let result = remove_project(&mut config, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn save_config_then_load_config_roundtrips() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.json");

    // Create a config with a project
    let project_temp = TempDir::new().unwrap();
    project_temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, "roundtrip", project_temp.path()).unwrap();

    // Save and reload
    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    assert_eq!(loaded.version, 1);
    assert!(loaded.projects.contains_key("roundtrip"));
    assert_eq!(loaded.projects["roundtrip"].path, project_temp.path());
}

#[test]
fn load_config_on_nonexistent_file_returns_default() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("does_not_exist.json");

    let config = load_config(&config_path).unwrap();
    assert_eq!(config.version, 1);
    assert!(config.projects.is_empty());
}

#[test]
fn list_projects_returns_sorted_by_alias() {
    let temp_a = TempDir::new().unwrap();
    temp_a.child(".planning").create_dir_all().unwrap();
    let temp_b = TempDir::new().unwrap();
    temp_b.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, "zulu", temp_a.path()).unwrap();
    add_project(&mut config, "alpha", temp_b.path()).unwrap();

    let projects = list_projects(&config);
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].0, "alpha");
    assert_eq!(projects[1].0, "zulu");
}

#[test]
fn end_to_end_add_then_list_via_cli() {
    let project_temp = TempDir::new().unwrap();
    project_temp.child(".planning").create_dir_all().unwrap();
    let config_temp = TempDir::new().unwrap();
    let config_path = config_temp.path().join("config.json");

    // Run `add`
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "add",
            "testalias",
            project_temp.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run cargo");

    assert!(
        output.status.success(),
        "Add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let add_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        add_stdout.contains("testalias"),
        "Expected 'testalias' in add output: {}",
        add_stdout
    );

    // Run `list`
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "list",
        ])
        .output()
        .expect("Failed to run cargo");

    assert!(
        output.status.success(),
        "List failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let list_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        list_stdout.contains("testalias"),
        "Expected 'testalias' in list output: {}",
        list_stdout
    );
}
