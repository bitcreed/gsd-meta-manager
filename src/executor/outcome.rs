//! Run-outcome derivation, and the before/after project snapshot it reads.
//!
//! The outcome comes from four sources and **never** from the agent's prose:
//! the `result` envelope, the process exit code, the `.planning/` artifact
//! fingerprint, and git (D-10). The exit code is a liveness/crash signal only —
//! the reproduced hang's exit code came from an external `timeout`, never from
//! Claude.
//!
//! `subtype` and `terminal_reason` are matched as `&str` with an explicit
//! fallback arm, never as typed enums: this CLI shipped three new values on one
//! version line, and a typed enum would need a catch-all on each and would
//! still lose the actual string.
//!
//! Plan 15-05 expands [`derive_run_outcome`] into the full D-26 matrix and
//! fills the two git fields from new helpers beside the existing ones in
//! `state_reader/git_ops.rs`. Both are functionality gaps fillable without any
//! signature or architecture change — which is why the four-argument shape is
//! fixed here rather than grown later.

use std::path::Path;
use std::process::ExitStatus;

use crate::executor::{RunOutcome, TurnOutcome};
use crate::state_reader::{self, ProjectState};

/// A point-in-time fingerprint of a project, captured before and after a run.
///
/// The artifact half is deliberately **not** a new reader:
/// [`state_reader::parse_project_state`] is already synchronous, idempotent,
/// never panics, and returns a type deriving `PartialEq`, so the delta is a
/// value comparison. Writing a parallel reader is exactly what D-11 forbids.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSnapshot {
    /// Git `HEAD` sha. `None` means **not yet captured**, not "no commits":
    /// plan 15-05 fills this from a new `git_ops` helper.
    pub head_sha: Option<String>,
    /// Whether the working tree was dirty. `None` means not yet captured.
    pub dirty: Option<bool>,
    /// The `.planning/` artifact fingerprint.
    pub project_state: ProjectState,
}

impl RunSnapshot {
    /// Capture a project's current state.
    ///
    /// Does full-tree file I/O, so callers must run it on a blocking thread —
    /// never on the async reactor (the precedent is the project re-parse in
    /// `src/app.rs`). Never panics: a missing or malformed `.planning/` yields
    /// a default fingerprint rather than an error.
    pub fn capture(project_root: &Path) -> Self {
        Self {
            head_sha: None,
            dirty: None,
            project_state: state_reader::parse_project_state(&project_root.join(".planning")),
        }
    }

    /// Whether anything observable changed between two snapshots.
    ///
    /// Git fields participate only once they are actually captured, so an
    /// uncaptured pair never fabricates a change.
    pub fn changed_since(&self, before: &RunSnapshot) -> bool {
        if self.project_state != before.project_state {
            return true;
        }
        if let (Some(before_sha), Some(after_sha)) = (&before.head_sha, &self.head_sha) {
            if before_sha != after_sha {
                return true;
            }
        }
        if let (Some(before_dirty), Some(after_dirty)) = (before.dirty, self.dirty) {
            if before_dirty != after_dirty {
                return true;
            }
        }
        false
    }
}

/// Derive the run-level outcome from the four corroboration sources.
///
/// `turns` is every `result` envelope observed, in stream order — a run that
/// uses `send` emits one per turn (D-29). The run-level verdict comes from the
/// **last** of them.
///
/// An **empty** `turns` means no terminal envelope was ever observed. That
/// yields a failure naming the missing envelope and is never reported as a
/// success, however the process exited: a stream that closed with nothing to
/// corroborate is the one case where the exit code alone would lie.
pub fn derive_run_outcome(
    turns: &[TurnOutcome],
    exit: Option<ExitStatus>,
    before: &RunSnapshot,
    after: &RunSnapshot,
) -> RunOutcome {
    let exit_code = exit.and_then(|status| status.code());

    let Some(last) = turns.last() else {
        return RunOutcome::Failed {
            reason: "the stream ended without a terminal `result` envelope".to_string(),
            subtype: None,
            terminal_reason: None,
            exit_code,
        };
    };

    let terminal_reason = last.terminal_reason.as_deref();

    match (last.subtype.as_str(), last.is_error, terminal_reason) {
        ("success", false, Some("completed")) | ("success", false, None) => {
            if after.changed_since(before) {
                RunOutcome::SucceededWithChanges {
                    turns: turns.to_vec(),
                    total_cost_usd: last.total_cost_usd,
                }
            } else {
                RunOutcome::SucceededNoChanges {
                    turns: turns.to_vec(),
                    total_cost_usd: last.total_cost_usd,
                }
            }
        }
        // Explicit fallback arm. New `subtype` and `terminal_reason` values ship
        // at patch level, so an unrecognised pair must classify as a failure
        // carrying the observed strings, never panic and never be dropped.
        (subtype, _, reason) => RunOutcome::Failed {
            reason: describe_failure(subtype, reason),
            subtype: Some(subtype.to_string()),
            terminal_reason: reason.map(str::to_string),
            exit_code,
        },
    }
}

/// A human-readable classification for a non-success terminal envelope.
fn describe_failure(subtype: &str, terminal_reason: Option<&str>) -> String {
    match (subtype, terminal_reason) {
        ("error_max_budget_usd", _) => "the run stopped at its budget ceiling".to_string(),
        ("error_max_turns", _) => "the run stopped at its turn ceiling".to_string(),
        (_, Some("aborted_tools")) => {
            "the run aborted during tool use — the classic hook-hang signature".to_string()
        }
        (_, Some("aborted_streaming")) => {
            "the run was interrupted while streaming".to_string()
        }
        (subtype, Some(reason)) => format!("the run failed: {subtype} / {reason}"),
        (subtype, None) => format!("the run failed: {subtype}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RunSnapshot {
        RunSnapshot {
            head_sha: None,
            dirty: None,
            project_state: ProjectState::default(),
        }
    }

    fn turn(subtype: &str, is_error: bool, terminal_reason: Option<&str>) -> TurnOutcome {
        TurnOutcome {
            subtype: subtype.to_string(),
            is_error,
            terminal_reason: terminal_reason.map(str::to_string),
            num_turns: Some(1),
            total_cost_usd: Some(0.5),
            session_id: Some("s".to_string()),
        }
    }

    #[test]
    fn a_stream_with_no_terminal_envelope_is_never_a_success() {
        let outcome = derive_run_outcome(&[], None, &snapshot(), &snapshot());
        match outcome {
            RunOutcome::Failed { reason, .. } => assert!(
                reason.contains("terminal `result` envelope"),
                "the failure must name the missing envelope, got: {reason}"
            ),
            other => panic!("expected a failure, got: {other:?}"),
        }
    }

    #[test]
    fn success_with_no_disk_delta_is_its_own_outcome() {
        let outcome = derive_run_outcome(
            &[turn("success", false, Some("completed"))],
            None,
            &snapshot(),
            &snapshot(),
        );
        assert!(
            matches!(outcome, RunOutcome::SucceededNoChanges { .. }),
            "an envelope saying success while nothing moved is a no-op, got: {outcome:?}"
        );
    }

    #[test]
    fn success_with_a_disk_delta_is_a_success_with_changes() {
        let before = snapshot();
        let mut after = snapshot();
        after.project_state.current_phase = "15".to_string();
        let outcome = derive_run_outcome(
            &[turn("success", false, Some("completed"))],
            None,
            &before,
            &after,
        );
        assert!(
            matches!(outcome, RunOutcome::SucceededWithChanges { .. }),
            "expected a success with changes, got: {outcome:?}"
        );
    }

    #[test]
    fn an_unrecognised_subtype_falls_back_to_a_failure_carrying_the_strings() {
        let outcome = derive_run_outcome(
            &[turn("error_from_a_future_version", true, Some("who_knows"))],
            None,
            &snapshot(),
            &snapshot(),
        );
        match outcome {
            RunOutcome::Failed {
                subtype,
                terminal_reason,
                ..
            } => {
                assert_eq!(subtype.as_deref(), Some("error_from_a_future_version"));
                assert_eq!(terminal_reason.as_deref(), Some("who_knows"));
            }
            other => panic!("expected a failure, got: {other:?}"),
        }
    }

    #[test]
    fn the_last_envelope_decides_a_multi_turn_run() {
        let outcome = derive_run_outcome(
            &[
                turn("success", false, Some("completed")),
                turn("error_during_execution", true, Some("aborted_streaming")),
            ],
            None,
            &snapshot(),
            &snapshot(),
        );
        assert!(
            matches!(outcome, RunOutcome::Failed { .. }),
            "the run verdict comes from the LAST result envelope, got: {outcome:?}"
        );
    }

    #[test]
    fn uncaptured_git_fields_never_fabricate_a_change() {
        assert!(
            !snapshot().changed_since(&snapshot()),
            "None means not-yet-captured, not a delta"
        );
    }
}
