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
//! 2. **One compact JSON document, one stdout line** (D1; every entry has twelve keys): `{schema,
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
//!    [`gsd_meta_manager::text::MAX_TEXT_CHARS`]. Filesystem paths (`path`, `sessions[].cwd`) are emitted
//!    raw: they are identifiers a script will `cd` into, and JSON encoding still
//!    escapes C0 controls. Artifact prose (phase goals, descriptions, CONTEXT.md,
//!    HANDOFF `next_action`) never reaches the output.
//! 5. **`projects[].owner` is counts only.** It is built from the same owner digest
//!    `inbox` prints (`read_owner_digest`, read once per project), so the two cannot
//!    drift; a project that could not be read has `owner: null`.
//! 6. **`projects[].next` is `next --json`.** It is built by
//!    [`crate::next_cli::hint_for`], the same function `next` calls, so the two
//!    cannot drift.

use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::driver::router;
use gsd_meta_manager::session_detector::{detect_sessions, ClaudeSession, SessionKind};
use gsd_meta_manager::state_reader::disk_status::VerificationStatus;
use gsd_meta_manager::state_reader::owner_items::{read_owner_digest, OwnerDigest};
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
use gsd_meta_manager::text::{cap_rendered as cap, terminal_capped as text};
use serde_json::{json, Value};

use crate::cli_targets::{self, fail, out_line, slots, Slot};
use crate::next_cli;

/// The `schema` field.
const SCHEMA: &str = "gsd-meta-manager/status";

/// The `schema_version` field. Bumped on any rename, removal or type change; new
/// keys may be added without a bump.
const SCHEMA_VERSION: u64 = 1;

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

/// One attention item (D6). All four keys are always present.
fn attention_item(
    kind: &str,
    phase: Option<&str>,
    workstream: Option<&str>,
    status: Option<String>,
) -> Value {
    json!({
        "kind": kind,
        "phase": phase.map(text),
        "workstream": workstream.map(text),
        "status": status,
    })
}

/// The pending human/UAT items of the root and of every workstream (D6).
///
/// Per scope, in order: the scope-wide flags, then each roadmap phase's disk
/// verdicts in roadmap order, then the deferred-verification rows. Prose
/// (`pause_context`, goals, descriptions) is never read.
///
/// `uat_outstanding` fires at most once per phase: when the UAT status is
/// owner-outstanding (`testing` included), or when the owner digest holds an open
/// check in a UAT whose status is not terminal (OWN-09).
fn attention_json(state: &ProjectState, digest: &OwnerDigest) -> Vec<Value> {
    let mut items = Vec::new();
    for (workstream, scope) in state.scopes() {
        let ws = workstream.map(|w| w.name.as_str());
        if scope.paused {
            items.push(attention_item("paused", None, ws, None));
        }
        if scope.external_job_waiting {
            items.push(attention_item("external_job_waiting", None, ws, None));
        }
        if scope.continue_here_present {
            items.push(attention_item("continue_here", None, ws, None));
        }
        for phase in &scope.phases {
            let Some(disk) = scope.disk_status_for(&phase.number) else {
                continue;
            };
            let id = Some(phase.number.as_str());
            if disk.verification_status == VerificationStatus::HumanNeeded {
                items.push(attention_item("verification_human_needed", id, ws, None));
            }
            if disk.uat_status.is_owner_outstanding()
                || digest.has_open_uat_items(ws, &phase.number)
            {
                items.push(attention_item(
                    "uat_outstanding",
                    id,
                    ws,
                    Some(text(disk.uat_status.as_str())),
                ));
            }
            if disk.continue_here_blocking {
                items.push(attention_item("continue_here_blocking", id, ws, None));
            }
        }
        for row in &scope.deferred_verification_phases {
            items.push(attention_item("deferred_verification", Some(row), ws, None));
        }
    }
    items
}

/// One live agent process (D4, D5).
fn session_json(session: &ClaudeSession) -> Value {
    let agent = match session.kind {
        SessionKind::Claude => "claude",
        SessionKind::Codex => "codex",
    };
    json!({
        "agent": agent,
        "pid": session.pid,
        "session_id": session.session_id.as_ref().map(|id| cap(id.shown())),
        "cwd": session.working_dir.display().to_string(),
        "tty": session.tty,
    })
}

/// The deepest of `roots` that `cwd` is, or lies under (D4).
///
/// `Path::starts_with` compares whole components, never a string prefix, so
/// `/x/foo` can never claim `/x/foobar`. When roots nest, the deepest wins, the
/// rule `resolve_launch_target` applies.
fn owning_root<'a>(cwd: &Path, roots: &'a [PathBuf]) -> Option<&'a Path> {
    roots
        .iter()
        .filter(|root| cwd.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(PathBuf::as_path)
}

/// The `owner` counts of one project: numbers only, current milestone of the root
/// and every workstream, after the owner rules. `decide` stays 0 until the decision
/// signals land.
fn owner_json(digest: &OwnerDigest) -> Value {
    json!({
        "test": digest.test,
        "decide": 0,
        "fix_phases": digest.fix_phases,
        "blocked": digest.blocked,
        "closeable_phases": digest.closeable_phases,
    })
}

/// A readable project's entry: all twelve keys of the schema.
fn project_entry(
    target: Option<&str>,
    alias: Option<&str>,
    root: &Path,
    sessions: Vec<Value>,
    stale_days: u32,
) -> Value {
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, chrono::Utc::now(), stale_days);
    json!({
        "target": target.map(text),
        "alias": alias.map(text),
        "path": root.display().to_string(),
        "error": Value::Null,
        "milestone": milestone_json(&state),
        "phase": phase_json(&state),
        "progress": progress_json(&state),
        "attention": attention_json(&state, &digest),
        "sessions": sessions,
        "next": next_cli::hint_for(root, &state),
        "warnings": warnings_json(&state),
        "owner": owner_json(&digest),
    })
}

/// The entry for a project that could not be read (D8): the same twelve keys,
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
        "owner": Value::Null,
    })
}

/// Run `status` and return the process exit code.
pub fn run(targets: &[String], config: Option<PathBuf>) -> i32 {
    let config_path = config.unwrap_or_else(Config::default_path);
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(error) => return fail("status", &format!("{error:#}")),
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            return fail(
                "status",
                &format!("cannot read the working directory: {error}"),
            )
        }
    };

    let stale_days = config.preferences.owner_stale_days();
    let slots: Vec<Slot> = slots(&config, &cwd, targets);

    // One detection per run, and only when some project can own a session. A
    // session goes to the deepest root among every registered and listed one.
    let any_project = slots
        .iter()
        .any(|slot| matches!(slot, Slot::Project { .. }));
    let sessions = if any_project {
        detect_sessions()
    } else {
        Vec::new()
    };
    let mut roots: Vec<PathBuf> = config
        .projects
        .values()
        .map(|project| cli_targets::canon_or_raw(&project.path))
        .collect();
    roots.extend(slots.iter().filter_map(|slot| match slot {
        Slot::Project { root, .. } => Some(root.clone()),
        Slot::Failed { .. } => None,
    }));

    let projects: Vec<Value> = slots
        .into_iter()
        .map(|slot| match slot {
            Slot::Failed {
                target,
                alias,
                path,
                kind,
                message,
            } => error_entry(
                target.as_deref(),
                alias.as_deref(),
                path.as_deref(),
                kind,
                &message,
            ),
            Slot::Project {
                target,
                alias,
                root,
            } => {
                let mut mine: Vec<&ClaudeSession> = sessions
                    .iter()
                    .filter(|s| owning_root(&s.working_dir, &roots) == Some(root.as_path()))
                    .collect();
                mine.sort_by_key(|s| s.pid);
                let mine = mine.into_iter().map(session_json).collect();
                project_entry(target.as_deref(), alias.as_deref(), &root, mine, stale_days)
            }
        })
        .collect();

    let document = json!({
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "sessions_supported": cfg!(target_os = "linux"),
        "projects": projects,
    });
    out_line(&document.to_string());
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsd_meta_manager::text::Untrusted;

    fn roots(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn owning_root_matches_an_exact_root_and_a_descendant() {
        let roots = roots(&["/work/app"]);
        assert_eq!(
            owning_root(Path::new("/work/app"), &roots),
            Some(Path::new("/work/app"))
        );
        assert_eq!(
            owning_root(Path::new("/work/app/src/deep"), &roots),
            Some(Path::new("/work/app"))
        );
    }

    #[test]
    fn owning_root_prefers_the_deepest_of_nested_roots() {
        let roots = roots(&["/work", "/work/app", "/work/app/sub"]);
        assert_eq!(
            owning_root(Path::new("/work/app/sub/x"), &roots),
            Some(Path::new("/work/app/sub"))
        );
        assert_eq!(
            owning_root(Path::new("/work/app/y"), &roots),
            Some(Path::new("/work/app"))
        );
    }

    #[test]
    fn owning_root_never_claims_a_string_prefix_sibling_or_a_stranger() {
        let roots = roots(&["/x/foo"]);
        assert_eq!(owning_root(Path::new("/x/foobar"), &roots), None);
        assert_eq!(owning_root(Path::new("/elsewhere"), &roots), None);
    }

    fn session(kind: SessionKind, id: Option<&str>) -> ClaudeSession {
        ClaudeSession {
            pid: 4242,
            kind,
            session_id: id.map(|id| Untrusted::from_untrusted_source(id.to_string())),
            working_dir: PathBuf::from("/work/app"),
            start_time: Some(1),
            tty: Some("pts/3".to_string()),
        }
    }

    #[test]
    fn session_json_has_exactly_the_documented_keys() {
        let value = session_json(&session(SessionKind::Claude, Some("abc")));
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["agent", "cwd", "pid", "session_id", "tty"]);
        assert_eq!(value["agent"], "claude");
        assert_eq!(value["pid"], 4242);
        assert_eq!(value["cwd"], "/work/app");
        assert_eq!(value["tty"], "pts/3");
    }

    #[test]
    fn session_json_maps_codex_and_a_missing_id() {
        let value = session_json(&session(SessionKind::Codex, None));
        assert_eq!(value["agent"], "codex");
        assert!(value["session_id"].is_null());
    }

    #[test]
    fn session_json_escapes_a_hostile_session_id() {
        let value = session_json(&session(SessionKind::Claude, Some("a\u{202e}b\u{1b}[0m")));
        let id = value["session_id"].as_str().unwrap();
        assert!(!id.contains('\u{202e}') && !id.contains('\u{1b}'), "{id:?}");
    }
}
