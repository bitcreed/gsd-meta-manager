//! `gsd-meta-manager inbox [project...] [--json] [--lane ...] [--include ...]`: the
//! checks the owner has to run, across every registered project or only the named
//! ones (Phase 34, D-18).
//!
//! # Numbered facts
//!
//! 1. **It writes nothing.** `main` dispatches `Commands::Inbox` before
//!    `init_tracing` (which creates a log file), and this module takes no lock,
//!    appends to no journal and creates no directory. The inputs are the projects'
//!    `.planning/` trees (read by the one shared state reader and the owner-items
//!    reader) and the registry file (read by plain `config::load_config`, which
//!    never writes). No process table is read.
//! 2. **Views (D-02).** `--json` prints one compact JSON document on one stdout
//!    line, built by [`gsd_meta_manager::inbox::build_document`] (no timestamp, so
//!    the output is deterministic). Without it, [`view_of`] picks the text: `--all`
//!    prints the full checklist of `inbox::render_text` (lane sections, `- [ ]`
//!    items, folds), exactly one project argument prints that project, any
//!    `--lane` prints the lanes, `--include` alone prints the full checklist, and
//!    everything else prints the digest of `inbox::render_digest`.
//! 3. **Exit codes.** `0` the document was printed, including per-project error
//!    entries; `1` the registry could not be loaded or parsed, or the working
//!    directory is unreadable (nothing on stdout, exactly one stderr line starting
//!    `inbox: `); `2` is clap's usage error.
//! 4. **Escaping.** Every project-authored string goes through
//!    `text::terminal_capped` inside `build_document`. Filesystem paths are emitted
//!    raw.
//! 5. **Target resolution is shared with `status`** (`crate::cli_targets`), so the
//!    two commands cannot drift on which project a name means.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use gsd_meta_manager::cli::{InboxInclude, InboxLane};
use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::inbox::{
    build_document, render_digest, render_project, render_text, EntryError, Filters, ProjectEntry,
};
use gsd_meta_manager::oneoff::todos::list_pending;
use gsd_meta_manager::state_reader::owner_items::{read_owner_digest, Lane};
use gsd_meta_manager::state_reader::parse_project_state;

use crate::cli_targets::{fail, out_line, slots, Slot};

fn lane_of(lane: InboxLane) -> Lane {
    match lane {
        InboxLane::Test => Lane::Test,
        InboxLane::Decide => Lane::Decide,
        InboxLane::Fix => Lane::Fix,
        InboxLane::Blocked => Lane::Blocked,
    }
}

/// Which output `inbox` prints (D-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Json,
    /// The full checklist of the releases before the digest.
    Legacy,
    Project,
    Lanes,
    Digest,
}

/// The view for the arguments, in the D-02 order: `--json`, then `--all`, then
/// exactly one project argument, then any `--lane`, then `--include` alone (the
/// full checklist, as before the digest), else the digest.
fn view_of(
    json: bool,
    all: bool,
    targets: &[String],
    lanes: &[InboxLane],
    includes: &[InboxInclude],
) -> View {
    if json {
        View::Json
    } else if all {
        View::Legacy
    } else if targets.len() == 1 {
        View::Project
    } else if !lanes.is_empty() {
        View::Lanes
    } else if !includes.is_empty() {
        View::Legacy
    } else {
        View::Digest
    }
}

fn filters_of(lanes: &[InboxLane], includes: &[InboxInclude]) -> Filters {
    Filters {
        lanes: lanes.iter().copied().map(lane_of).collect(),
        archived: includes.contains(&InboxInclude::Archived),
        deferred: includes.contains(&InboxInclude::Deferred),
        acknowledged: includes.contains(&InboxInclude::Acknowledged),
        todos: includes.contains(&InboxInclude::Todos),
    }
}

fn read_entry(
    target: Option<String>,
    alias: Option<String>,
    root: PathBuf,
    config: &Config,
    filters: &Filters,
    now: DateTime<Utc>,
) -> ProjectEntry {
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(
        &planning,
        &state,
        now,
        config.preferences.owner_stale_days(),
    );
    let pending = if filters.todos {
        list_pending(&root).unwrap_or_default()
    } else {
        Vec::new()
    };
    let progress = state.phase_progress();
    ProjectEntry {
        target,
        alias,
        path: Some(root),
        error: None,
        milestone_version: Some(state.milestone.clone()),
        phases_done: progress.done,
        phases_total: progress.total,
        digest,
        todos: pending,
    }
}

/// Run `inbox` and return the process exit code.
pub fn run(
    targets: &[String],
    json: bool,
    all: bool,
    lanes: &[InboxLane],
    includes: &[InboxInclude],
    config: Option<PathBuf>,
) -> i32 {
    let config_path = config.unwrap_or_else(Config::default_path);
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(error) => return fail("inbox", &format!("{error:#}")),
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            return fail(
                "inbox",
                &format!("cannot read the working directory: {error}"),
            )
        }
    };
    let filters = filters_of(lanes, includes);
    let fold_days = config.preferences.owner_fold_days();
    // One clock reading per run, so every age in the document shares one instant.
    let now = Utc::now();
    let entries: Vec<ProjectEntry> = slots(&config, &cwd, targets)
        .into_iter()
        .map(|slot| match slot {
            Slot::Project {
                target,
                alias,
                root,
            } => read_entry(target, alias, root, &config, &filters, now),
            Slot::Failed {
                target,
                alias,
                path,
                kind,
                message,
            } => ProjectEntry {
                target,
                alias,
                path,
                error: Some(EntryError { kind, message }),
                ..Default::default()
            },
        })
        .collect();
    let text = match view_of(json, all, targets, lanes, includes) {
        View::Json => build_document(&entries, &filters).to_string(),
        View::Project => render_project(&entries[0], &filters, fold_days),
        // The lane renderer arrives with plan 38-03; until then that invocation
        // prints what it printed before this phase.
        View::Legacy | View::Lanes => render_text(&entries, &filters),
        View::Digest => render_digest(&entries, fold_days),
    };
    for line in text.lines() {
        out_line(line);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_of_follows_the_d02_order() {
        let none: Vec<String> = Vec::new();
        let one = vec!["a".to_owned()];
        let two = vec!["a".to_owned(), "b".to_owned()];
        let lane = [InboxLane::Test];
        let inc = [InboxInclude::Archived];
        assert_eq!(view_of(true, true, &one, &lane, &inc), View::Json);
        assert_eq!(view_of(false, true, &one, &lane, &inc), View::Legacy);
        assert_eq!(view_of(false, true, &none, &[], &[]), View::Legacy);
        assert_eq!(view_of(false, false, &one, &[], &[]), View::Project);
        assert_eq!(view_of(false, false, &one, &[], &inc), View::Project);
        assert_eq!(view_of(false, false, &one, &lane, &[]), View::Project);
        assert_eq!(view_of(false, false, &none, &lane, &[]), View::Lanes);
        assert_eq!(view_of(false, false, &two, &lane, &inc), View::Lanes);
        assert_eq!(view_of(false, false, &none, &[], &inc), View::Legacy);
        assert_eq!(view_of(false, false, &two, &[], &inc), View::Legacy);
        assert_eq!(view_of(false, false, &none, &[], &[]), View::Digest);
        assert_eq!(view_of(false, false, &two, &[], &[]), View::Digest);
    }
}
