use crate::config::{Config, RegisteredProject};
use crate::session_detector::ClaudeSession;
use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
            // Written out rather than defaulted: registering a project lets the
            // dashboard read it, and driving it is a separate deliberate act.
            // `auto_register_from_sessions` reaches this line, so this explicit
            // `None` is what proves discovery can never enrol a project into
            // being driven (D-14, D-15).
            driver_opt_in: None,
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
            // See `add_project`: never set by any registration path (D-14).
            driver_opt_in: None,
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

/// Canonicalize a path, falling back to the raw path if canonicalization fails
/// (e.g. broken symlink, permission denied). Equivalent canonical paths are
/// what we use to deduplicate against the registry.
fn canon_or_raw(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// Derive an alias from a path's file name (lowercased). Falls back to "project".
fn derive_alias(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "project".to_string())
}

/// Pick an alias that doesn't collide with existing keys. If `base` is free,
/// returns it as-is; otherwise tries `base-2`, `base-3`, … up to `base-999`.
fn unique_alias(config: &Config, base: &str) -> String {
    if !config.projects.contains_key(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{}-{}", base, n);
        if !config.projects.contains_key(&candidate) {
            return candidate;
        }
    }
    // Pathological fallback — should never happen in practice
    format!("{}-{}", base, chrono::Utc::now().timestamp())
}

/// Scan active Claude sessions and auto-register any whose `working_dir` is a
/// GSD project (contains `.planning/`) and is not already in the registry.
///
/// Returns the list of `(alias, canonical_path)` pairs that were newly added.
/// Callers are responsible for persisting the config and starting file
/// watchers on the new entries.
///
/// Dedup is canonical-path based — paths that resolve to the same target
/// (e.g. symlinked variants, trailing slashes) register at most once per call
/// and won't double-register if already present in the config.
pub fn auto_register_from_sessions(
    config: &mut Config,
    sessions: &[ClaudeSession],
) -> Vec<(String, PathBuf)> {
    let registered: HashSet<PathBuf> = config
        .projects
        .values()
        .map(|p| canon_or_raw(&p.path))
        .collect();

    let mut seen_this_pass: HashSet<PathBuf> = HashSet::new();
    let mut added: Vec<(String, PathBuf)> = Vec::new();

    for session in sessions {
        let canonical = canon_or_raw(&session.working_dir);

        if !canonical.join(".planning").is_dir() {
            continue;
        }
        if registered.contains(&canonical) || !seen_this_pass.insert(canonical.clone()) {
            continue;
        }

        let base = derive_alias(&canonical);
        let alias = unique_alias(config, &base);

        if let Err(e) = add_project(config, &alias, &canonical) {
            tracing::warn!(
                alias = %alias,
                path = %canonical.display(),
                error = %e,
                "auto-register: add_project failed",
            );
            continue;
        }
        added.push((alias, canonical));
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn make_session(working_dir: PathBuf) -> ClaudeSession {
        ClaudeSession {
            pid: 1,
            session_id: None,
            working_dir,
            start_time: None,
            tty: None,
        }
    }

    fn empty_config() -> Config {
        Config::new()
    }

    #[test]
    fn auto_register_adds_new_gsd_project() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(config.projects.len(), 1);
        let alias = &added[0].0;
        assert!(config.projects.contains_key(alias));
    }

    #[test]
    fn auto_register_skips_existing_canonical_path() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let mut config = empty_config();
        add_project(&mut config, "existing", &canonical).unwrap();
        let sessions = vec![make_session(canonical.clone())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert!(added.is_empty());
        assert_eq!(config.projects.len(), 1);
    }

    #[test]
    fn auto_register_skips_non_gsd_project() {
        let dir = tempdir().unwrap();
        // Intentionally NO .planning/ directory
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert!(added.is_empty());
        assert!(config.projects.is_empty());
    }

    #[test]
    fn auto_register_dedupes_within_single_pass() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![
            make_session(dir.path().to_path_buf()),
            make_session(dir.path().to_path_buf()),
        ];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(config.projects.len(), 1);
    }

    #[test]
    fn auto_register_uses_suffix_on_alias_collision() {
        // Two different directories sharing the same basename.
        let parent_a = tempdir().unwrap();
        let parent_b = tempdir().unwrap();
        std::fs::create_dir_all(parent_a.path().join("myproj/.planning")).unwrap();
        std::fs::create_dir_all(parent_b.path().join("myproj/.planning")).unwrap();
        let path_a = parent_a.path().join("myproj");
        let path_b = parent_b.path().join("myproj");

        let mut config = empty_config();
        // Pre-register the first one under its natural alias.
        add_project(&mut config, "myproj", &path_a.canonicalize().unwrap()).unwrap();

        let sessions = vec![make_session(path_b)];
        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(added[0].0, "myproj-2");
        assert!(config.projects.contains_key("myproj"));
        assert!(config.projects.contains_key("myproj-2"));
    }

    #[test]
    fn auto_register_returns_canonical_paths() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        let canonical = dir.path().canonicalize().unwrap();
        assert_eq!(added[0].1, canonical);
    }
}
