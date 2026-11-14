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
//! 2. **One compact JSON document, one stdout line.** Built by
//!    [`gsd_meta_manager::inbox::build_document`]; no timestamp, so the output is
//!    deterministic. Without `--json`, the plain checklist of `inbox::render_text`
//!    is printed instead (lane sections, `- [ ]` items, folds).
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

use chrono::Utc;
use gsd_meta_manager::cli::{InboxInclude, InboxLane};
use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::inbox::{build_document, render_text, EntryError, Filters, ProjectEntry};
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
) -> ProjectEntry {
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(
        &planning,
        &state,
        Utc::now(),
        config.preferences.owner_stale_days(),
    );
    let pending = list_pending(&root).unwrap_or_default();
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
        todos_count: pending.len(),
        todos: if filters.todos { pending } else { Vec::new() },
    }
}

/// Run `inbox` and return the process exit code.
pub fn run(
    targets: &[String],
    json: bool,
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
    let entries: Vec<ProjectEntry> = slots(&config, &cwd, targets)
        .into_iter()
        .map(|slot| match slot {
            Slot::Project {
                target,
                alias,
                root,
            } => read_entry(target, alias, root, &config, &filters),
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
    if json {
        out_line(&build_document(&entries, &filters).to_string());
    } else {
        for line in render_text(&entries, &filters).lines() {
            out_line(line);
        }
    }
    0
}
