use crate::config::{Config, DriverOptIn, PromptInput, RegisteredProject};
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

    let prompt_inputs = current_prompt_inputs(&entry.path);
    entry.driver_opt_in = Some(DriverOptIn {
        opted_in_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        // Deliberately `None`: this build does not write the legacy key. It is
        // kept on the struct so an older binary that loads and re-saves this
        // config still finds the field it expects, which is what makes the
        // upgrade migration-free. `prompt_inputs` is where the real digest
        // lives, and it covers `CLAUDE.md` along with everything else that
        // reaches a prompt.
        claude_md_digest: None,
        // Named explicitly rather than absorbed through `..Default::default()`,
        // for the reason the comment below records: the next field added must
        // break this line.
        prompt_inputs,
        // A fresh record carries no fields this build does not model.
        extra: Default::default(),
        // Every envelope field is named explicitly as `None` rather than
        // reached for through `..Default::default()`, for the reason
        // `config.rs:41-45` records about the opt-in field itself: the next
        // field added must break this line, not be silently absorbed by it.
        // `None` here means "the envelope's own defaults apply", resolved in
        // the one place they are decided — `EnvelopePolicy::resolve` (D-30).
        branch_namespace: None,
        credential: None,
        pr_cap_per_24h: None,
        pr_cap_per_run: None,
    });

    Ok(())
}

/// Which spawn profile reads a disclosed file.
///
/// A **disclosure label**, deliberately not [`crate::executor::SpawnProfile`]:
/// that type carries a JSON schema and exists to shape an argv, while this one
/// exists to tell a user which of the two spawns will read a file. Keeping them
/// separate stops the disclosure surface from acquiring a dependency on the
/// seam's payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptProfile {
    /// The bounded model seam: an empty tool set and a suppressed `CLAUDE.md`.
    /// Only the enumerated third-party *strings* from these files reach it, and
    /// only inside the labelled untrusted boundary — never the whole file.
    ModelSeam,
    /// The spawn that actually runs a GSD command. It loads the target
    /// repository's `CLAUDE.md` itself, and this phase does **not** change that.
    Executor,
}

/// Every file whose bytes can reach a model prompt, and which profile reads it.
///
/// **This list is the disclosure, and its membership is a correctness property.**
/// The plan's own prohibition rates an under-broad list as exactly as dishonest
/// as an over-broad one, and under-broad is the more dangerous direction: an
/// over-broad list annoys, an under-broad one misleads a user about what reaches
/// the model.
///
/// `HANDOFF` is here in **both** its spellings because `state_reader` reads
/// either (`HANDOFF.json` first, then `HANDOFF.md`), and a list naming only one
/// would be under-broad by exactly one file. `ProjectState` is parsed from
/// `STATE.md` *and* the `HANDOFF` file; `RoadmapPhase` from `ROADMAP.md` — see
/// `crate::driver::untrusted`'s census, which is authoritative for that.
///
/// Absent files are still listed. A file that does not exist at opt-in is
/// recorded with a `None` digest rather than omitted, so that its later
/// appearance is a mismatch — see [`crate::config::PromptInput::digest`].
pub const DISCLOSED_PROMPT_INPUTS: &[(&str, PromptProfile)] = &[
    ("CLAUDE.md", PromptProfile::Executor),
    (".planning/STATE.md", PromptProfile::ModelSeam),
    (".planning/ROADMAP.md", PromptProfile::ModelSeam),
    (".planning/HANDOFF.json", PromptProfile::ModelSeam),
    (".planning/HANDOFF.md", PromptProfile::ModelSeam),
];

/// A SHA-256 digest of one file under `project_root`, or `None` if it is absent
/// or unreadable.
///
/// Absent and unreadable are ordinary states for a project, not failures — and
/// they are **not** silently equivalent to "unchanged": `None` is recorded and
/// compared like any other value, so a file appearing or vanishing after opt-in
/// is drift.
fn file_digest(project_root: &Path, relative: &str) -> Option<String> {
    let bytes = std::fs::read(project_root.join(relative)).ok()?;
    Some(crate::journal::sha256_digest(&bytes))
}

/// The disclosed file set with each file's digest as it stands right now.
///
/// Reads bytes rather than a `String` so a file that is not valid UTF-8 still
/// digests instead of silently reading as absent — "unreadable" should mean the
/// filesystem refused, not that the content surprised us.
pub fn current_prompt_inputs(project_root: &Path) -> Vec<PromptInput> {
    DISCLOSED_PROMPT_INPUTS
        .iter()
        .map(|(relative, _profile)| PromptInput {
            path: (*relative).to_string(),
            digest: file_digest(project_root, relative),
            extra: Default::default(),
        })
        .collect()
}

/// Why a recorded opt-in no longer covers what would reach a prompt.
///
/// Every variant means the same thing to the gate — **re-confirm** — and the
/// variants exist only so the reason can be shown to the user. A gate with one
/// answer for every unclear state has no unclear state left to get wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptInDrift {
    /// The record predates the disclosure entirely (an empty `prompt_inputs`).
    /// A record written before the field existed never showed the user a list,
    /// so it cannot stand as an informed approval of what reaches a prompt.
    NothingDisclosed,
    /// A recorded digest does not carry the `sha256:` prefix.
    ///
    /// Matched on "is not `sha256:`" rather than on "is `fnv1a64:`", which is
    /// load-bearing: the pre-Phase-19 fixture in `config.rs` carries `fnv1a:`,
    /// so an exact legacy-prefix match would sail past a real legacy value
    /// already present in this tree.
    LegacyDigest { path: String, digest: String },
    /// A file this build would read has no entry in the record at all — the
    /// user never saw it, so they never approved it.
    Undisclosed { path: String },
    /// The bytes changed, appeared, or vanished since opt-in.
    Changed { path: String },
}

impl OptInDrift {
    /// A single line naming the file and what happened, for the surface that
    /// tells the user why they are being asked again.
    pub fn describe(&self) -> String {
        match self {
            OptInDrift::NothingDisclosed => "this opt-in predates the prompt-input disclosure, \
                 so it never listed which files' bytes reach a prompt"
                .to_string(),
            OptInDrift::LegacyDigest { path, .. } => format!(
                "`{path}` was recorded with a legacy digest that is not a security control, \
                 so it cannot be compared across hash families"
            ),
            OptInDrift::Undisclosed { path } => format!(
                "`{path}` can reach a prompt but was not disclosed when this opt-in was recorded"
            ),
            OptInDrift::Changed { path } => {
                format!("`{path}` no longer matches the bytes recorded at opt-in")
            }
        }
    }
}

/// Whether `opt_in` still covers the bytes that would reach a prompt right now.
///
/// `None` means the recorded approval is still good. `Some(drift)` means
/// re-confirm.
///
/// **Called at the spawn gate, not at render time.** A confirmation screen left
/// open while a `git pull` rewrites `CLAUDE.md` must not be able to approve
/// bytes that changed underneath it, so the question is asked where the opt-in
/// is consulted before a spawn — `crate::executor::DrivableProject::from_registry`.
pub fn check_prompt_input_drift(project_root: &Path, opt_in: &DriverOptIn) -> Option<OptInDrift> {
    if opt_in.prompt_inputs.is_empty() {
        return Some(OptInDrift::NothingDisclosed);
    }

    // A legacy digest anywhere in the record is checked before any comparison,
    // so a value from a hash family this binary does not compute can never
    // reach the equality test and read as "matches".
    for recorded in &opt_in.prompt_inputs {
        if let Some(digest) = &recorded.digest {
            if !digest.starts_with("sha256:") {
                return Some(OptInDrift::LegacyDigest {
                    path: recorded.path.clone(),
                    digest: digest.clone(),
                });
            }
        }
    }

    for (relative, _profile) in DISCLOSED_PROMPT_INPUTS {
        let Some(recorded) = opt_in
            .prompt_inputs
            .iter()
            .find(|entry| entry.path == *relative)
        else {
            return Some(OptInDrift::Undisclosed {
                path: (*relative).to_string(),
            });
        };

        if recorded.digest != file_digest(project_root, relative) {
            return Some(OptInDrift::Changed {
                path: (*relative).to_string(),
            });
        }
    }

    None
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
///
/// **It deliberately touches no UI-side map, and the omission is structural
/// rather than an oversight** (D-27): this module is free functions over a
/// `&mut Config` and has no access to `AppContext`, where the driver state lives
/// — `run_states`, `observed_runs` and `journal_cursors`. Those are cleaned in
/// exactly two places, and this sentence exists so the next reader finds them
/// instead of concluding the leak is still open:
///
/// * `ui::screens::delete_confirm::do_remove_project` — the interactive path,
///   which drops them in the same block as `project_states` and `last_refresh`.
/// * `App::prune_driver_maps` — the backstop, on the existing 20-tick block, for
///   every removal that does not go through that screen.
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

        // **Rewritten in the commit that falsified it** (the `dry_run.rs:78-83`
        // precedent). This previously asserted `claude_md_digest` was `Some`
        // with an `fnv1a64:` prefix. Phase 21 stopped writing that key — the
        // digest moved to `prompt_inputs` and became SHA-256, because a
        // re-confirmation prompt is a security affordance and FNV-1a says in
        // its own documentation that it is not a security control (C-4).
        assert_eq!(
            record.claude_md_digest, None,
            "this build must not write the legacy key; it is kept on the struct \
             only so an older binary's load-and-save stays non-destructive"
        );

        let claude_md = record
            .prompt_inputs
            .iter()
            .find(|entry| entry.path == "CLAUDE.md")
            .expect("`CLAUDE.md` is one of the disclosed prompt inputs");
        let digest = claude_md
            .digest
            .as_ref()
            .expect("a project that has a CLAUDE.md records its digest");
        assert!(
            digest.starts_with("sha256:"),
            "the re-confirmation digest must be collision resistant, got: {digest}"
        );

        // The whole disclosed set is recorded, not just the files that exist —
        // an absent file is `None`, so its later appearance is drift.
        assert_eq!(
            record.prompt_inputs.len(),
            DISCLOSED_PROMPT_INPUTS.len(),
            "every disclosed file gets an entry, present or not"
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
