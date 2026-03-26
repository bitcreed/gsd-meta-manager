use crate::config::{Config, RegisteredProject};
use anyhow::{bail, Result};
use std::path::Path;

/// Add a project to the registry with the given alias and path.
/// Validates that:
/// - alias is non-empty and contains no whitespace
/// - alias is unique in the config
/// - path exists on disk
/// - path contains a `.planning/` directory
pub fn add_project(config: &mut Config, alias: &str, path: &Path) -> Result<()> {
    if alias.is_empty() {
        bail!("Alias cannot be empty");
    }

    if alias.contains(char::is_whitespace) {
        bail!("Alias cannot contain whitespace");
    }

    if config.projects.contains_key(alias) {
        bail!("Alias already exists");
    }

    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    let planning_dir = path.join(".planning");
    if !planning_dir.is_dir() {
        bail!("No .planning/ directory found at: {}", path.display());
    }

    let now = chrono::Utc::now().to_rfc3339();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: path.to_path_buf(),
            added: now,
        },
    );

    Ok(())
}

/// Add a project to the registry without checking for `.planning/` directory.
/// Used for freshly created projects that don't yet have a `.planning/` folder.
/// Validates alias is non-empty, has no whitespace, and is unique.
pub fn add_project_unchecked(config: &mut Config, alias: &str, path: &Path) -> Result<()> {
    if alias.is_empty() {
        bail!("Alias cannot be empty");
    }

    if alias.contains(char::is_whitespace) {
        bail!("Alias cannot contain whitespace");
    }

    if config.projects.contains_key(alias) {
        bail!("Alias already exists");
    }

    let now = chrono::Utc::now().to_rfc3339();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: path.to_path_buf(),
            added: now,
        },
    );

    Ok(())
}

/// Remove a project from the registry by alias.
pub fn remove_project(config: &mut Config, alias: &str) -> Result<()> {
    if config.projects.remove(alias).is_none() {
        bail!("Project not found: {}", alias);
    }
    Ok(())
}

/// List all registered projects sorted by alias.
pub fn list_projects(config: &Config) -> Vec<(&String, &RegisteredProject)> {
    let mut projects: Vec<_> = config.projects.iter().collect();
    projects.sort_by_key(|(alias, _)| alias.to_lowercase());
    projects
}
