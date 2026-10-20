//! `gsd-meta-manager next <project> --json`: the Phase 20 router as an advisory,
//! strictly read-only hint (Phase 29, PRMT-05; D-20, D-21).
//!
//! The driver's master prompt cites this command as "what the deterministic
//! router would do next". It is a hint, never a binding: the driver stays free to
//! disagree, and nothing here stops it.
//!
//! # Numbered facts
//!
//! 1. **It writes nothing.** `main` dispatches `Commands::Next` before
//!    `init_tracing` (which creates a log file), and this module takes no lock,
//!    appends to no journal and creates no directory. The inputs are the project's
//!    `.planning/` tree (read by the one shared state reader) and, for an alias,
//!    the registry file (read by `config::load_config`, which returns an empty
//!    config for a missing file and never writes). That is what lets it run in
//!    the driver's sandbox, where the run directory and `$HOME` are read-only.
//! 2. **Every value in the output is a typed token or a closed-set constant.**
//!    `Decision` payloads, `RouterReason::as_str()` and the router's rationale
//!    constants — never text read out of a project artifact. The `project` field
//!    is the resolved root the caller named; `target_phase` was validated as a
//!    single plain path component before anything was read (T-29-10, T-29-13).
//! 3. **Exit codes.** `0` a hint was printed; `1` the project could not be
//!    resolved, has no `.planning/`, or `--target-phase` is not a plain
//!    component (one `next: ...` line on stderr, nothing on stdout); `2` is clap's
//!    usage error, including a missing `--json`.

use std::io::Write;
use std::path::{Path, PathBuf};

use gsd_meta_manager::driver::router::{self, Decision};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::state_reader::parse_project_state;
use gsd_meta_manager::text::{render_for_terminal, Untrusted};
use serde_json::{json, Value};

/// The `source` field: which component produced the hint.
const SOURCE: &str = "phase-20-router";

/// The `reason` for a goal-met decision.
const REASON_GOAL_MET: &str = "goal-met";

/// The `reason` for a rule-table miss.
const REASON_NO_RULE: &str = "no-rule";

/// The `reason` when there is no phase left to route.
const REASON_NO_TARGET: &str = "no-target";

/// Write one line to stderr, ignoring failure (a closed pipe must not panic).
fn err_line(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{text}");
}

/// Write one line to stdout, ignoring failure.
fn out_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

/// Resolve the project argument to a root that contains `.planning/`.
fn resolve_root(target: &str) -> Result<PathBuf, String> {
    let path = Path::new(target);
    if path.join(".planning").is_dir() {
        return path.canonicalize().map_err(|e| {
            format!(
                "cannot resolve {}: {e}",
                render_for_terminal(&path.display().to_string())
            )
        });
    }
    Err(format!(
        "{} is not a directory containing .planning/",
        render_for_terminal(target)
    ))
}

/// The D-20 object. `decision: None` is the "nothing left to route" answer.
///
/// One arm per `Decision` variant and no wildcard, following the router's own
/// discipline: a new arm is a compile error here, not a silent fallthrough.
fn hint_json(root: &Path, target: Option<&str>, decision: Option<&Decision>) -> Value {
    let (kind, command, reason, detail): (&str, Option<&str>, &str, Option<&str>) = match decision
    {
        Some(Decision::Run { command, rationale }) => {
            ("run", Some(command.as_str()), rationale, None)
        }
        Some(Decision::Park { reason, detail }) => {
            ("park", None, reason.as_str(), Some(detail.as_str()))
        }
        Some(Decision::GoalMet) => ("goal_met", None, REASON_GOAL_MET, None),
        Some(Decision::NoRule { observed }) => {
            ("no_rule", None, REASON_NO_RULE, Some(observed.as_str()))
        }
        None => ("none", None, REASON_NO_TARGET, None),
    };
    json!({
        "advisory": true,
        "source": SOURCE,
        "project": root.display().to_string(),
        "target_phase": target,
        "decision": kind,
        "command": command,
        "reason": reason,
        "detail": detail,
    })
}

/// Run `next` and return the process exit code.
pub fn run(project: &str, target_phase: Option<&str>, _config: Option<PathBuf>) -> i32 {
    if let Some(target) = target_phase {
        if !gsd_meta_manager::journal::is_plain_path_component(target) {
            let refusal = DriveError::TargetPhaseInvalid {
                target_phase: Untrusted::from_untrusted_source(target.to_string()),
            };
            err_line(&format!("next: {refusal}"));
            return 1;
        }
    }
    let root = match resolve_root(project) {
        Ok(root) => root,
        Err(message) => {
            err_line(&format!("next: {message}"));
            return 1;
        }
    };
    let Some(target) = target_phase else {
        err_line("next: --target-phase is required");
        return 1;
    };
    let state = parse_project_state(&root.join(".planning"));
    let decision = router::decide(&state, target);
    out_line(&hint_json(&root, Some(target), Some(&decision)).to_string());
    0
}
