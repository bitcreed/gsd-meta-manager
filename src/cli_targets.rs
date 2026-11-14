//! Target resolution shared by the read-only query commands `status` and `inbox`
//! (Phase 34, D-18).
//!
//! Both commands take the same positional (`[project...]`: a directory holding
//! `.planning/`, or a registered alias, or a path inside a registered root), list
//! every registered project when none is named, and report a project they cannot
//! read as a per-project error entry instead of failing the run. This module is
//! that one resolver; each command maps [`Slot::Failed`] to its own error entry.
//! Nothing here writes: the registry is read by plain `config::load_config`.

use std::io::Write;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::Config;
use gsd_meta_manager::launch_target::resolve_launch_target;
use gsd_meta_manager::registry::list_projects;
use gsd_meta_manager::text::{render_for_terminal, terminal_capped};

/// What one requested project resolved to, before its state is read.
pub enum Slot {
    /// A root that exists and holds `.planning/`.
    Project {
        target: Option<String>,
        alias: Option<String>,
        root: PathBuf,
    },
    /// Resolution failed. `kind` is `unresolved`, `folder_missing` or
    /// `no_planning`; `message` is already escaped for a terminal.
    Failed {
        target: Option<String>,
        alias: Option<String>,
        path: Option<PathBuf>,
        kind: &'static str,
        message: String,
    },
}

/// Write one line to stdout, ignoring failure (a closed pipe must not panic).
pub fn out_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

pub fn canon_or_raw(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// A registered (or resolved) root, checked before its state is parsed.
pub fn checked_slot(target: Option<&str>, alias: Option<&str>, root: PathBuf) -> Slot {
    let failed = |kind: &'static str, message: String, root: PathBuf| Slot::Failed {
        target: target.map(str::to_owned),
        alias: alias.map(str::to_owned),
        path: Some(root),
        kind,
        message,
    };
    if !root.is_dir() {
        let message = format!(
            "the folder {} does not exist",
            terminal_capped(&root.display().to_string())
        );
        return failed("folder_missing", message, root);
    }
    if !root.join(".planning").is_dir() {
        let message = format!(
            "{} has no .planning/ directory",
            terminal_capped(&root.display().to_string())
        );
        return failed("no_planning", message, root);
    }
    Slot::Project {
        target: target.map(str::to_owned),
        alias: alias.map(str::to_owned),
        root,
    }
}

/// The registry alias whose canonical root is `root`; ties go to the
/// lexicographically smallest alias, because `projects` is a `HashMap`.
pub fn alias_of_root(config: &Config, root: &Path) -> Option<String> {
    config
        .projects
        .iter()
        .filter(|(_, project)| canon_or_raw(&project.path) == root)
        .map(|(alias, _)| alias.clone())
        .min()
}

/// Resolve one argv target (D2, D3): a directory holding `.planning/`, else a
/// registered alias or a path inside a registered root, else unresolved.
pub fn resolve_target(config: &Config, cwd: &Path, target: &str) -> Slot {
    let path = Path::new(target);
    if path.join(".planning").is_dir() {
        let root = canon_or_raw(path);
        let alias = alias_of_root(config, &root);
        return checked_slot(Some(target), alias.as_deref(), root);
    }
    if let Ok(alias) = resolve_launch_target(config, target, cwd) {
        if let Some(project) = config.projects.get(&alias) {
            return checked_slot(Some(target), Some(&alias), canon_or_raw(&project.path));
        }
    }
    let message = format!(
        "{} is neither a directory containing .planning/ nor a registered project",
        terminal_capped(target)
    );
    Slot::Failed {
        target: Some(target.to_owned()),
        alias: None,
        path: None,
        kind: "unresolved",
        message,
    }
}

/// The requested projects in output order: registry order (as `list`) when no
/// target is named, argv order otherwise. A name given twice appears twice.
pub fn slots(config: &Config, cwd: &Path, targets: &[String]) -> Vec<Slot> {
    if targets.is_empty() {
        list_projects(config)
            .into_iter()
            .map(|(alias, project)| checked_slot(None, Some(alias), canon_or_raw(&project.path)))
            .collect()
    } else {
        targets
            .iter()
            .map(|target| resolve_target(config, cwd, target))
            .collect()
    }
}

/// Write the one `<prefix>: ...` stderr line of a failed run and return exit code 1.
pub fn fail(prefix: &str, message: &str) -> i32 {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{prefix}: {}", render_for_terminal(message));
    1
}
