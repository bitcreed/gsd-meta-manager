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

use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::driver::router::{self, Decision};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::launch_target::resolve_launch_target;
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
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
///
/// An existing directory with a `.planning/` subdirectory is used as it is: that
/// is the sandbox case, where there is no registry to consult. Anything else is
/// looked up as a registered alias (or a path inside a registered root) through
/// the same read-only resolver the launcher uses. Neither branch creates a file.
fn resolve_root(target: &str, config: Option<&Path>) -> Result<PathBuf, String> {
    let shown = || render_for_terminal(target);
    let path = Path::new(target);
    if path.join(".planning").is_dir() {
        return path
            .canonicalize()
            .map_err(|e| format!("cannot resolve {}: {e}", shown()));
    }
    let config_path = config.map_or_else(Config::default_path, Path::to_path_buf);
    let config = load_config(&config_path).map_err(|e| format!("{e:#}"))?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("cannot read the working directory: {e}"))?;
    let alias = resolve_launch_target(&config, target, &cwd).map_err(|_| {
        format!(
            "{} is neither a directory containing .planning/ nor a registered project",
            shown()
        )
    })?;
    let root = config
        .projects
        .get(&alias)
        .map(|project| project.path.clone())
        .ok_or_else(|| format!("project {} is not in the registry", shown()))?;
    if !root.join(".planning").is_dir() {
        return Err(format!("{} has no .planning/ directory", shown()));
    }
    Ok(root.canonicalize().unwrap_or(root))
}

/// The first current-milestone roadmap phase whose verification is not goal-met.
///
/// Goal-met is [`router::is_goal_met`], the router's own predicate, and NOT
/// `phase_is_done`: that counts `Executed` as done, so a phase stuck at
/// `gaps_found` would be skipped and never reported. A phase with no disk
/// inference at all is not goal-met either. Roadmap order, no re-sorting.
fn default_target(state: &ProjectState) -> Option<String> {
    state
        .roadmap_entries()
        .into_iter()
        .map(|(phase, _planned)| phase)
        .filter(|phase| state.in_current_milestone(&phase.number))
        .find(|phase| {
            !state
                .disk_status_for(&phase.number)
                .is_some_and(router::is_goal_met)
        })
        .map(|phase| phase.number.clone())
}

/// The D-20 object. `decision: None` is the "nothing left to route" answer.
///
/// One arm per `Decision` variant and no wildcard, following the router's own
/// discipline: a new arm is a compile error here, not a silent fallthrough.
fn hint_json(root: &Path, target: Option<&str>, decision: Option<&Decision>) -> Value {
    let (kind, command, reason, detail): (&str, Option<&str>, &str, Option<&str>) = match decision {
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
pub fn run(project: &str, target_phase: Option<&str>, config: Option<PathBuf>) -> i32 {
    if let Some(target) = target_phase {
        if !gsd_meta_manager::journal::is_plain_path_component(target) {
            let refusal = DriveError::TargetPhaseInvalid {
                target_phase: Untrusted::from_untrusted_source(target.to_string()),
            };
            err_line(&format!("next: {refusal}"));
            return 1;
        }
    }
    let root = match resolve_root(project, config.as_deref()) {
        Ok(root) => root,
        Err(message) => {
            err_line(&format!("next: {message}"));
            return 1;
        }
    };
    let state = parse_project_state(&root.join(".planning"));
    let target = target_phase
        .map(str::to_owned)
        .or_else(|| default_target(&state));
    // No target left (every current-milestone phase is goal-met, or the roadmap
    // names none): an honest "nothing to route", not an error (D-20).
    let decision = target.as_deref().map(|phase| router::decide(&state, phase));
    out_line(&hint_json(&root, target.as_deref(), decision.as_ref()).to_string());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// Phase 1 belongs to a shipped milestone; 2 and 3 to the active one.
    fn banded_project(verdicts: &[(&str, &str)], context_only: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let planning = dir.path().join(".planning");
        write(
            &planning.join("ROADMAP.md"),
            "# Roadmap\n\n## Milestones\n\n\
             - \u{2705} **v1.0 First** -- Phases 1-1 (shipped 2026-01-01)\n\
             - \u{1f6a7} **v1.1 Second** -- Phases 2-3 (in progress)\n\n\
             ## Phases\n\n\
             - [x] **Phase 1: Old** - shipped\n\
             - [ ] **Phase 2: Mid** - active\n\
             - [ ] **Phase 3: Last** - active\n\n\
             ### Phase 1: Old\n\n**Goal**: g\n**Depends on**: Nothing\n\n\
             ### Phase 2: Mid\n\n**Goal**: g\n**Depends on**: Nothing\n\n\
             ### Phase 3: Last\n\n**Goal**: g\n**Depends on**: Nothing\n",
        );
        write(
            &planning.join("STATE.md"),
            "---\nmilestone: v1.1\nmilestone_name: Second\nstatus: in_progress\n---\n\n# State\n",
        );
        for (phase, status) in verdicts {
            let dir = planning.join(format!("phases/0{phase}-p{phase}"));
            write(
                &dir.join(format!("0{phase}-01-PLAN.md")),
                "---\nphase: x\n---\n",
            );
            write(
                &dir.join(format!("0{phase}-01-SUMMARY.md")),
                "---\nstatus: complete\n---\n",
            );
            write(
                &dir.join(format!("0{phase}-VERIFICATION.md")),
                &format!("---\nstatus: {status}\n---\n"),
            );
        }
        for phase in context_only {
            write(
                &planning.join(format!("phases/0{phase}-p{phase}/0{phase}-CONTEXT.md")),
                "# Context\n",
            );
        }
        dir
    }

    fn target_of(dir: &tempfile::TempDir) -> Option<String> {
        default_target(&parse_project_state(&dir.path().join(".planning")))
    }

    #[test]
    fn the_default_target_skips_a_passed_phase_and_stops_at_a_gaps_found_one() {
        let dir = banded_project(&[("2", "passed"), ("3", "gaps_found")], &[]);
        assert_eq!(target_of(&dir).as_deref(), Some("3"));
    }

    #[test]
    fn the_default_target_does_not_skip_a_phase_that_is_executed_but_not_verified() {
        // `phase_is_done` counts Executed as done; goal-met does not.
        let dir = banded_project(&[("2", "gaps_found")], &["3"]);
        assert_eq!(target_of(&dir).as_deref(), Some("2"));
    }

    #[test]
    fn the_default_target_is_none_when_every_current_milestone_phase_passed() {
        // Phase 1 is NOT passed, but it is outside the current milestone.
        let dir = banded_project(&[("2", "passed"), ("3", "passed")], &["1"]);
        assert_eq!(target_of(&dir), None);
    }

    #[test]
    fn the_default_target_counts_a_phase_with_no_artifacts_as_unfinished() {
        let dir = banded_project(&[("2", "passed")], &[]);
        assert_eq!(target_of(&dir).as_deref(), Some("3"));
    }
}
