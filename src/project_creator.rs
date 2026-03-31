use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::HooksConfig;

/// Expand tilde in a path string to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// Resolve a user-provided path string: expand tilde, then resolve relative paths
/// against the current directory. Does NOT canonicalize (directory may not exist yet).
pub fn resolve_path(input: &str) -> PathBuf {
    let expanded = expand_tilde(input.trim());
    if expanded.is_relative() {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(expanded)
    } else {
        expanded
    }
}

/// Create a new GSD project: run pre-create hook, create directory, git init, run post-create hook.
pub fn create_project(name: &str, path: &Path, hooks: &HooksConfig) -> anyhow::Result<()> {
    let alias = name.to_lowercase().replace(' ', "-");

    // Run pre_create hook if configured
    if let Some(ref hook_cmd) = hooks.pre_create {
        execute_hook(hook_cmd, name, path, &alias).context("Pre-create hook failed")?;
    }

    // Create the project directory
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;

    // Initialize git repository
    let status = Command::new("git")
        .arg("init")
        .current_dir(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to run git init")?;

    if !status.success() {
        bail!("git init failed with exit code: {:?}", status.code());
    }

    // Run post_create hook if configured
    if let Some(ref hook_cmd) = hooks.post_create {
        execute_hook(hook_cmd, name, path, &alias).context("Post-create hook failed")?;
    }

    Ok(())
}

/// Execute a shell hook command with GSD environment variables set.
pub fn execute_hook(hook_cmd: &str, name: &str, path: &Path, alias: &str) -> anyhow::Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(hook_cmd)
        .env("GSD_PROJECT_NAME", name)
        .env("GSD_PROJECT_PATH", path.as_os_str())
        .env("GSD_PROJECT_ALIAS", alias)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("Failed to execute hook: {}", hook_cmd))?;

    if !status.success() {
        bail!(
            "Hook command failed with exit code {:?}: {}",
            status.code(),
            hook_cmd
        );
    }

    Ok(())
}

/// Tab-complete a partial path input. Returns matching filesystem entries.
/// Expands tilde, lists children of the parent directory that match the prefix.
/// Returns at most 20 results. Silently returns empty vec on any IO error.
pub fn tab_complete_path(partial: &str) -> Vec<String> {
    let expanded = expand_tilde(partial);

    let (search_dir, prefix) = if partial.ends_with('/') || expanded.is_dir() {
        // List children of this directory
        (expanded.clone(), String::new())
    } else {
        // List children of parent that start with the filename prefix
        let parent = expanded.parent().unwrap_or(Path::new(".")).to_path_buf();
        let prefix = expanded
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, prefix)
    };

    let entries = match std::fs::read_dir(&search_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            if prefix.is_empty() {
                true
            } else {
                entry.file_name().to_string_lossy().starts_with(&prefix)
            }
        })
        .map(|entry| {
            let path = entry.path();
            let mut s = path.to_string_lossy().to_string();
            if path.is_dir() {
                s.push('/');
            }
            s
        })
        .collect();

    results.sort();
    results.truncate(20);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_home() {
        let result = expand_tilde("~");
        assert!(result != PathBuf::from("~") || dirs::home_dir().is_none());
    }

    #[test]
    fn test_expand_tilde_subpath() {
        let result = expand_tilde("~/projects/test");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(result, home.join("projects/test"));
        }
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_resolve_path_absolute() {
        let result = resolve_path("/tmp/test");
        assert_eq!(result, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_resolve_path_relative() {
        let result = resolve_path("relative/path");
        assert!(result.is_absolute());
    }

    #[test]
    fn test_tab_complete_nonexistent_returns_empty() {
        let results = tab_complete_path("/nonexistent_dir_abc123/");
        assert!(results.is_empty());
    }
}
