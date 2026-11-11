//! `gsd-meta-manager status --json [project...]`: one schema-versioned JSON
//! document describing every registered project, or only the named ones (quick
//! 261110-qap). The query a script or a model uses instead of opening the TUI.
//!
//! # Numbered facts
//!
//! 1. **It writes nothing.** `main` dispatches `Commands::Status` before
//!    `init_tracing` (which creates a log file), and this module takes no lock,
//!    appends to no journal and creates no directory. The inputs are the projects'
//!    `.planning/` trees (read by the one shared state reader), the registry file
//!    (read by plain `config::load_config`, which returns an empty config for a
//!    missing file and never writes; the pruning loader `list` uses SAVES, so it
//!    is deliberately not used here) and the process table (read-only).
//! 2. **One compact JSON document, one stdout line** (D1): `{schema,
//!    schema_version, sessions_supported, projects}`. No timestamp, so the output
//!    is deterministic. The schema v1 contract (every key path, its type and its
//!    meaning, and the stability rule) lives in README.md's "Scripting and agents"
//!    section; `tests/status_cli.rs` pins the emitted key paths against it.
//! 3. **Exit codes.** `0` the document was printed, including per-project error
//!    entries; `1` the registry could not be loaded or parsed, or the working
//!    directory is unreadable (nothing on stdout, exactly one stderr line starting
//!    `status: `); `2` is clap's usage error, including a missing `--json`.
//! 4. **Escaping (D5).** Every project-authored string (milestone, phase name, raw
//!    verification and UAT values, workstream names, session ids, the echoed
//!    target) goes through `text::render_for_terminal` and is capped at
//!    [`MAX_TEXT_CHARS`]. Filesystem paths (`path`, `sessions[].cwd`) are emitted
//!    raw: they are identifiers a script will `cd` into, and JSON encoding still
//!    escapes C0 controls. Artifact prose (phase goals, descriptions, CONTEXT.md,
//!    HANDOFF `next_action`) never reaches the output.
//! 5. **`projects[].next` is `next --json`.** It is built by
//!    [`crate::next_cli::hint_for`], the same function `next` calls, so the two
//!    cannot drift.

use std::io::Write;
use std::path::{Path, PathBuf};

use gsd_meta_manager::driver::router;
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
use gsd_meta_manager::text::{render_for_terminal, Rendered};
use serde_json::{json, Value};

use crate::next_cli;

/// The `schema` field.
const SCHEMA: &str = "gsd-meta-manager/status";

/// The `schema_version` field. Bumped on any rename, removal or type change; new
/// keys may be added without a bump.
const SCHEMA_VERSION: u64 = 1;

/// The longest project-authored string, in characters, before it is cut.
const MAX_TEXT_CHARS: usize = 200;

/// Write one line to stdout, ignoring failure (a closed pipe must not panic).
fn out_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

/// Cut `value` to [`MAX_TEXT_CHARS`] characters, appending one `…` when it cut.
fn cap_string(value: String) -> String {
    if value.chars().count() <= MAX_TEXT_CHARS {
        return value;
    }
    let mut cut: String = value.chars().take(MAX_TEXT_CHARS).collect();
    cut.push('…');
    cut
}

/// An escaped value, capped.
fn cap(rendered: Rendered) -> String {
    cap_string(String::from(rendered))
}

/// Project-authored text: escaped for a terminal, then capped.
fn text(value: &str) -> String {
    cap(render_for_terminal(value))
}

/// [`text`], or `None` for a blank value.
fn opt_text(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| text(value))
}

fn milestone_json(state: &ProjectState) -> Value {
    json!({
        "version": opt_text(&state.milestone),
        "name": state
            .milestone_name
            .as_ref()
            .filter(|name| !name.as_raw_for_logic_only().trim().is_empty())
            .map(|name| cap(name.shown())),
        "status": opt_text(&state.status),
    })
}

/// The active phase (D7), or `None` when neither STATE.md nor the disk frontier
/// names one, so the `completed_phases + 1` fallback is never reported as real.
fn phase_json(state: &ProjectState) -> Option<Value> {
    if state.state_md_phase_number.is_none() && state.current_phase_number.is_none() {
        return None;
    }
    let id = state.active_phase_number().to_string();
    let name = state
        .roadmap_phase(&id)
        .and_then(|phase| opt_text(&phase.name))
        .or_else(|| opt_text(&state.current_phase_name));
    let disk = state.disk_status_for(&id);
    Some(json!({
        "id": text(&id),
        "name": name,
        "status": disk.map(|d| router::status_token(d.status)),
        "verification": disk.map(|d| text(d.verification_status.as_str())),
        "uat": disk.map(|d| text(d.uat_status.as_str())),
    }))
}

fn progress_json(state: &ProjectState) -> Value {
    let progress = state.phase_progress();
    json!({
        "phases_done": progress.done,
        "phases_total": progress.total,
    })
}

fn warnings_json(state: &ProjectState) -> Vec<&'static str> {
    let mut warnings = Vec::new();
    if state.state_md_unreadable {
        warnings.push("state_md_unreadable");
    }
    if state.state_md_recovered {
        warnings.push("state_md_recovered");
    }
    warnings
}

/// The entry for a readable project: all eleven keys of the schema.
fn project_entry(target: Option<&str>, alias: Option<&str>, root: &Path) -> Value {
    let state = parse_project_state(&root.join(".planning"));
    json!({
        "target": target.map(text),
        "alias": alias.map(text),
        "path": root.display().to_string(),
        "error": Value::Null,
        "milestone": milestone_json(&state),
        "phase": phase_json(&state),
        "progress": progress_json(&state),
        "attention": Vec::<Value>::new(),
        "sessions": Vec::<Value>::new(),
        "next": next_cli::hint_for(root, &state),
        "warnings": warnings_json(&state),
    })
}

/// The entry for a project that could not be read (D8): the same eleven keys,
/// null data keys and empty arrays.
fn error_entry(
    target: Option<&str>,
    alias: Option<&str>,
    path: Option<&Path>,
    kind: &str,
    message: &str,
) -> Value {
    json!({
        "target": target.map(text),
        "alias": alias.map(text),
        "path": path.map(|p| p.display().to_string()),
        "error": {"kind": kind, "message": message},
        "milestone": Value::Null,
        "phase": Value::Null,
        "progress": Value::Null,
        "attention": Vec::<Value>::new(),
        "sessions": Vec::<Value>::new(),
        "next": Value::Null,
        "warnings": Vec::<String>::new(),
    })
}

/// Resolve one argv target to an entry.
fn target_entry(target: &str) -> Value {
    let path = Path::new(target);
    if path.join(".planning").is_dir() {
        let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        return project_entry(Some(target), None, &root);
    }
    error_entry(
        Some(target),
        None,
        None,
        "unresolved",
        &format!(
            "{} is neither a directory containing .planning/ nor a registered project",
            text(target)
        ),
    )
}

/// Run `status` and return the process exit code.
pub fn run(targets: &[String], _config: Option<PathBuf>) -> i32 {
    let projects: Vec<Value> = targets.iter().map(|t| target_entry(t)).collect();
    let document = json!({
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "sessions_supported": cfg!(target_os = "linux"),
        "projects": projects,
    });
    out_line(&document.to_string());
    0
}
