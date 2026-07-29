use crate::config::{Config, DriverOptIn, RegisteredProject};
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
            //
            // Written out at the site rather than with a struct-update
            // shorthand on purpose: `..Default::default()` would absorb the next
            // field silently, and the compile error that forced this line to
            // exist is the whole mechanism.
            driver_opt_in: None,
            // A fresh entry carries no unknown fields. Also explicit, for the
            // same reason.
            extra: Default::default(),
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
            extra: Default::default(),
        },
    );

    Ok(())
}

/// Record the user's deliberate opt-in for `alias`.
///
/// **This is the only function outside tests that constructs a [`DriverOptIn`],
/// and that uniqueness is the point (D-14).** Because a record can come into
/// existence in exactly one place, a `Some(record)` sitting in a `config.json` is
/// proof of a deliberate user action rather than a bit that could have been
/// flipped from anywhere. `tests/spawn_seam_guard.rs`-style greps can check
/// "constructed once"; they cannot check "every branch remembered to ask".
///
/// The timestamp is
/// `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` — the same
/// call `JournalRun::finish` makes, deliberately not this module's older bare
/// `to_rfc3339()`, so an opt-in stamp and a run's `ended_at` compare directly
/// without normalising.
///
/// **Does not persist.** The caller calls `save_config`, consistently with every
/// other function in this module.
pub fn record_opt_in(config: &mut Config, alias: &str) -> Result<()> {
    let Some(entry) = config.projects.get_mut(alias) else {
        bail!("Project not found: {}", alias);
    };

    let digest = claude_md_digest(&entry.path);
    entry.driver_opt_in = Some(DriverOptIn {
        opted_in_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        claude_md_digest: digest,
    });

    Ok(())
}

/// A digest of `<project_root>/CLAUDE.md` as it stands right now.
///
/// Reuses [`crate::journal::argv_digest`], which adds no hashing code and no
/// dependency. Two facts about that function are the reason it is the right one
/// here, and both are already stated on it: it is FNV-1a based, and it is
/// **explicitly not a security control**. That is exactly what this needs — a
/// cheap identity fingerprint for *drift detection*, never an integrity check.
///
/// **Phase 17 records the digest and acts on nothing.** Phase 21 re-confirms the
/// opt-in when the file drifts; recording it now is what saves that phase a
/// second migration of a user-owned file.
///
/// `None` when `CLAUDE.md` is absent or unreadable — both are ordinary states for
/// a project, not failures.
fn claude_md_digest(project_root: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(project_root.join("CLAUDE.md")).ok()?;
    Some(crate::journal::argv_digest(&[contents]))
}

/// Withdraw the opt-in for `alias`, removing the record entirely.
///
/// Setting the field back to `None` rather than storing a "revoked" marker: the
/// gate asks one question — is there a record? — and a second representation of
/// "no" is a second thing that can be got wrong. Like [`record_opt_in`], this
/// does not persist.
pub fn clear_opt_in(config: &mut Config, alias: &str) -> Result<()> {
    let Some(entry) = config.projects.get_mut(alias) else {
        bail!("Project not found: {}", alias);
    };
    entry.driver_opt_in = None;
    Ok(())
}

/// Whether `alias` carries an opt-in record.
///
/// A cheap read for the UI (plan 17-07). **Not a gate**: the gate is
/// `DrivableProject::from_registry`, which lives in the driver process so a
/// hand-typed `drive` is refused by the same code as a TUI-initiated one (D-16).
/// An unregistered alias answers `false`.
pub fn is_opted_in(config: &Config, alias: &str) -> bool {
    config
        .projects
        .get(alias)
        .is_some_and(|entry| entry.driver_opt_in.is_some())
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
    fn record_opt_in_stamps_a_record_with_a_second_precision_timestamp() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Project\n").unwrap();
        let mut config = empty_config();
        add_project(&mut config, "opted", dir.path()).unwrap();

        assert!(
            !is_opted_in(&config, "opted"),
            "registration alone never opts a project in (D-14)"
        );

        record_opt_in(&mut config, "opted").unwrap();

        let record = config.projects["opted"]
            .driver_opt_in
            .as_ref()
            .expect("record_opt_in constructs the record");

        // Second precision, RFC3339, `Z` — the shape `JournalRun::finish` writes,
        // so an opt-in stamp and a run's `ended_at` compare without normalising.
        assert!(
            record.opted_in_at.ends_with('Z'),
            "the stamp is UTC with a Z suffix, got: {}",
            record.opted_in_at
        );
        assert!(
            !record.opted_in_at.contains('.'),
            "second precision means no fractional part, got: {}",
            record.opted_in_at
        );
        assert_eq!(
            record.opted_in_at.len(),
            20,
            "`YYYY-MM-DDTHH:MM:SSZ` is 20 characters, got: {}",
            record.opted_in_at
        );

        let digest = record
            .claude_md_digest
            .as_ref()
            .expect("a project with a CLAUDE.md records its digest");
        assert!(
            digest.starts_with("fnv1a64:"),
            "the digest reuses `journal::argv_digest`, got: {digest}"
        );

        assert!(is_opted_in(&config, "opted"));
    }

    #[test]
    fn clear_opt_in_removes_the_record_entirely() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        add_project(&mut config, "opted", dir.path()).unwrap();
        record_opt_in(&mut config, "opted").unwrap();

        clear_opt_in(&mut config, "opted").unwrap();

        assert!(
            config.projects["opted"].driver_opt_in.is_none(),
            "withdrawal removes the record; there is no second representation of 'no'"
        );
        assert!(!is_opted_in(&config, "opted"));
        assert!(
            config.projects.contains_key("opted"),
            "withdrawing the opt-in never unregisters the project"
        );
    }

    #[test]
    fn auto_registration_leaves_every_discovered_project_not_opted_in() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        std::fs::create_dir(dir_a.path().join(".planning")).unwrap();
        std::fs::create_dir(dir_b.path().join(".planning")).unwrap();
        // A CLAUDE.md present at discovery time must make no difference at all.
        std::fs::write(dir_a.path().join("CLAUDE.md"), "# A\n").unwrap();

        let mut config = empty_config();
        let sessions = vec![
            make_session(dir_a.path().to_path_buf()),
            make_session(dir_b.path().to_path_buf()),
        ];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 2, "both discovered projects register");
        for (alias, entry) in &config.projects {
            assert!(
                entry.driver_opt_in.is_none(),
                "discovery may REGISTER; driving requires a separate, explicit, \
                 persisted opt-in — but '{alias}' came back opted in (D-15)"
            );
        }
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
