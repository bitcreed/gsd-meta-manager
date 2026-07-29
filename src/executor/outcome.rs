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
use crate::state_reader::{self, git_ops, ProjectState};

/// A point-in-time fingerprint of a project, captured before and after a run.
///
/// The artifact half is deliberately **not** a new reader:
/// [`state_reader::parse_project_state`] is already synchronous, idempotent,
/// never panics, and returns a type deriving `PartialEq`, so the delta is a
/// value comparison. Writing a parallel reader is exactly what D-11 forbids.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSnapshot {
    /// Git `HEAD` sha, from [`git_ops::head_sha`]. `None` means **unknown** —
    /// the project is not a git repository, has no commits, or the field was
    /// never captured. It never means "no commits" specifically, and it is
    /// never read as "the sha did not move".
    pub head_sha: Option<String>,
    /// Whether the working tree was dirty, from [`git_ops::is_dirty`].
    /// `None` means unknown, on the same terms as [`RunSnapshot::head_sha`].
    pub dirty: Option<bool>,
    /// The `.planning/` artifact fingerprint.
    pub project_state: ProjectState,
}

impl RunSnapshot {
    /// Capture a project's current state.
    ///
    /// Does full-tree file I/O **and shells out to git twice**, so callers must
    /// run the whole capture on a blocking thread — never on the async reactor
    /// (the precedent is the project re-parse in `src/app.rs`). Deliberately
    /// synchronous end to end for that reason: all three reads belong in one
    /// blocking closure. Never panics: a missing or malformed `.planning/`
    /// yields a default fingerprint, and a non-repository yields `None` for
    /// both git fields, rather than an error.
    pub fn capture(project_root: &Path) -> Self {
        Self {
            head_sha: git_ops::head_sha(project_root),
            dirty: git_ops::is_dirty(project_root),
            project_state: state_reader::parse_project_state(&project_root.join(".planning")),
        }
    }

    /// Whether anything observable changed between two snapshots.
    ///
    /// A thin roll-up over [`DiskDelta`], kept because it reads well at the
    /// call site; the three independent signals live on the delta.
    pub fn changed_since(&self, before: &RunSnapshot) -> bool {
        DiskDelta::between(before, self).made_changes()
    }
}

/// The disk-and-git half of the four-source outcome derivation (D-11).
///
/// Three **independent** signals, deliberately not collapsed into one boolean
/// at capture time: which one fired is what tells a reader whether an agent
/// edited planning artifacts, committed, or left uncommitted work behind.
///
/// **An unknown git half is inconclusive, not false.** When a project is not a
/// git repository, `head_sha` and `dirty` are `None` on both snapshots and the
/// two git signals stay `false` — they contribute nothing rather than asserting
/// "nothing moved". A project with no git therefore still reports its artifact
/// changes honestly, and a half-captured pair never fabricates a delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiskDelta {
    /// The `.planning/` artifact fingerprint changed.
    ///
    /// A value comparison on `ProjectState`, which already derives `PartialEq`
    /// — reusing the existing reader rather than hand-rolling a field-by-field
    /// diff is the whole point of D-11's "do not write a parallel one".
    pub artifacts_changed: bool,
    /// Git `HEAD` moved: the run committed something.
    pub head_moved: bool,
    /// The working tree's dirty flag flipped in either direction: the run left
    /// uncommitted work behind, or committed work that was pending.
    pub dirty_flipped: bool,
}

impl DiskDelta {
    /// Compute the delta between a before and an after snapshot.
    pub fn between(before: &RunSnapshot, after: &RunSnapshot) -> Self {
        let head_moved = match (&before.head_sha, &after.head_sha) {
            (Some(before_sha), Some(after_sha)) => before_sha != after_sha,
            // Unknown on either side: inconclusive, so the signal stays quiet.
            _ => false,
        };
        let dirty_flipped = match (before.dirty, after.dirty) {
            (Some(before_dirty), Some(after_dirty)) => before_dirty != after_dirty,
            _ => false,
        };

        Self {
            artifacts_changed: before.project_state != after.project_state,
            head_moved,
            dirty_flipped,
        }
    }

    /// Whether **any** of the three signals fired.
    ///
    /// Any one is sufficient: a run that only moved `HEAD` changed the project
    /// just as truly as one that only rewrote `STATE.md`.
    pub fn made_changes(&self) -> bool {
        self.artifacts_changed || self.head_moved || self.dirty_flipped
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

    // ========================================================================
    // The disk and git delta (D-11)
    // ========================================================================

    #[test]
    fn two_identical_snapshots_produce_a_delta_with_no_signals() {
        let delta = DiskDelta::between(&snapshot(), &snapshot());
        assert_eq!(
            delta,
            DiskDelta::default(),
            "nothing moved, so no signal may fire: {delta:?}"
        );
        assert!(!delta.made_changes(), "the roll-up must agree: {delta:?}");
    }

    #[test]
    fn a_moved_head_fires_only_the_head_signal() {
        let mut before = snapshot();
        before.head_sha = Some("aaaaaaa".to_string());
        before.dirty = Some(false);
        let mut after = before.clone();
        after.head_sha = Some("bbbbbbb".to_string());

        let delta = DiskDelta::between(&before, &after);
        assert!(delta.head_moved, "the sha moved: {delta:?}");
        assert!(!delta.artifacts_changed, "no artifact moved: {delta:?}");
        assert!(!delta.dirty_flipped, "the tree stayed clean: {delta:?}");
        assert!(
            delta.made_changes(),
            "any single signal is sufficient: {delta:?}"
        );
    }

    #[test]
    fn a_flipped_dirty_flag_fires_only_the_dirty_signal() {
        let mut before = snapshot();
        before.head_sha = Some("aaaaaaa".to_string());
        before.dirty = Some(false);
        let mut after = before.clone();
        after.dirty = Some(true);

        let delta = DiskDelta::between(&before, &after);
        assert!(delta.dirty_flipped, "the dirty flag flipped: {delta:?}");
        assert!(!delta.head_moved, "the sha did not move: {delta:?}");
        assert!(delta.made_changes(), "uncommitted work is work: {delta:?}");
    }

    #[test]
    fn an_unknown_git_half_leaves_the_git_signals_inconclusive_not_false() {
        // A non-repository project: both git fields are None on both sides.
        let before = snapshot();
        let mut after = snapshot();
        after.project_state.current_phase = "16".to_string();

        let delta = DiskDelta::between(&before, &after);
        assert!(
            delta.artifacts_changed,
            "a project with no git still reports artifact changes honestly: {delta:?}"
        );
        assert!(
            !delta.head_moved && !delta.dirty_flipped,
            "an unknown git half contributes nothing rather than asserting no-change: {delta:?}"
        );
    }

    #[test]
    fn a_half_captured_git_pair_never_fabricates_a_signal() {
        let before = snapshot();
        let mut after = snapshot();
        after.head_sha = Some("aaaaaaa".to_string());
        after.dirty = Some(true);

        let delta = DiskDelta::between(&before, &after);
        assert!(
            !delta.head_moved && !delta.dirty_flipped,
            "an unknown before-side makes the comparison inconclusive: {delta:?}"
        );
    }

    #[test]
    fn changing_one_artifact_file_between_captures_fires_the_artifact_signal() {
        let tmp = std::env::temp_dir().join(format!(
            "gsd_outcome_test_artifacts_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let planning = tmp.join(".planning");
        std::fs::create_dir_all(&planning).expect("create .planning");
        std::fs::write(
            planning.join("STATE.md"),
            "---\nstatus: planning\ncurrent_phase: 15\n---\n",
        )
        .expect("write STATE.md");

        let before = RunSnapshot::capture(&tmp);

        std::fs::write(
            planning.join("STATE.md"),
            "---\nstatus: executing\ncurrent_phase: 16\n---\n",
        )
        .expect("rewrite STATE.md");

        let after = RunSnapshot::capture(&tmp);
        let delta = DiskDelta::between(&before, &after);

        assert!(
            delta.artifacts_changed,
            "the fingerprint must notice a rewritten STATE.md: {:?} then {:?}",
            before.project_state.current_phase, after.project_state.current_phase
        );
        assert!(delta.made_changes(), "so the roll-up fires too: {delta:?}");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn capture_fills_the_git_half_in_a_real_repository() {
        let tmp = std::env::temp_dir()
            .join(format!("gsd_outcome_test_gitcapture_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create the scratch project");

        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&tmp)
                .args(args)
                .output()
                .ok()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        if !git(&["init"]) {
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test User"]);
        std::fs::write(tmp.join("seed.txt"), "seed").expect("write seed");
        if !git(&["add", "seed.txt"]) || !git(&["commit", "-m", "seed"]) {
            // Sandbox forbids committing — skip gracefully, as git_ops does.
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let snapshot = RunSnapshot::capture(&tmp);
        assert!(
            snapshot.head_sha.is_some(),
            "capture must fill the sha in a real repository"
        );
        assert_eq!(
            snapshot.dirty,
            Some(false),
            "a freshly committed scratch project is clean"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
