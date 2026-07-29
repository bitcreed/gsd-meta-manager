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
//! [`derive_run_outcome_from_envelopes`] is the **only** run-outcome entry
//! point, and it deliberately takes the full `result` envelopes rather than the
//! [`TurnOutcome`] projection of them. An envelope-discarding sibling used to
//! sit beside it and hard-coded an empty denials source; the production
//! coordinator called that one, so a `--permission-mode dontAsk` run that was
//! blocked from doing anything reported as a plain success (CR-04). Two entry
//! points onto one matrix is what made that possible, so there is now one.

use std::path::Path;
use std::process::ExitStatus;
use std::time::Duration;

use crate::executor::stream_json::ResultMessage;
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

/// The run-level turn count: the **sum** of every envelope's per-turn count.
///
/// That field resets on each envelope — fixture 05 reports `1` on both of its
/// turns and the A3 sub-probe reported `1` then `2` — so reading it off the
/// last envelope understates every steered run (D-29). An envelope that omits
/// the field contributes nothing rather than a guessed `1`.
pub fn run_turn_count(turns: &[TurnOutcome]) -> u64 {
    turns
        .iter()
        .map(|turn| turn.num_turns.unwrap_or(0))
        .sum()
}

/// The run-level cumulative cost: the **last** envelope's value.
///
/// That field accumulates across turns, so the last envelope already carries
/// the run total and summing would double-count.
///
/// **This is a notional figure.** Under subscription auth it prices work that
/// is not billed per call, so it must never be presented as what the run cost
/// the user (D-16, T-15-23).
pub fn run_cost_usd(turns: &[TurnOutcome]) -> Option<f64> {
    turns.last().and_then(|turn| turn.total_cost_usd)
}

/// Derive the run-level outcome from the **full** terminal envelopes.
///
/// The complete four-source derivation. D-10 names `permission_denials[]` as
/// part of the envelope source, and that array survives only on the envelope,
/// so this is the entry point that can report a permission refusal.
///
/// `envelopes` is every `result` envelope observed, in stream order — a run
/// that uses `send` emits one per turn (D-29). The run-level verdict comes from
/// the **last** of them; denials are collected from all of them.
///
/// This is the **only** run-outcome entry point, and it reads the denials
/// array. A sibling that took the [`TurnOutcome`] projection — which cannot
/// carry `permission_denials[]` — is what let a blocked run be reported as a
/// success, so no second entry point onto this matrix exists.
///
/// The envelopes' prose summaries are read by no branch of the derivation.
pub fn derive_run_outcome_from_envelopes(
    envelopes: &[ResultMessage],
    exit: Option<ExitStatus>,
    before: &RunSnapshot,
    after: &RunSnapshot,
) -> RunOutcome {
    let turns: Vec<TurnOutcome> = envelopes.iter().map(TurnOutcome::from_result).collect();
    // Denials from ANY turn, not merely the last: a refusal three turns back
    // still explains why the run produced nothing.
    let denials: Vec<serde_json::Value> = envelopes
        .iter()
        .flat_map(|envelope| envelope.permission_denials.iter().cloned())
        .collect();

    derive(&turns, denials, exit, before, after)
}

/// The derivation matrix proper (D-10, D-26, D-29, D-32).
///
/// Source precedence, in the order the arms are tried:
///
/// 1. **No terminal envelope at all** — a failure naming the missing envelope,
///    however the process exited. A stream that closed with nothing to
///    corroborate is the one case where the exit code alone would lie.
/// 2. **A populated denials array** — the most actionable classification
///    available, and the tell for an untrusted workspace silently voiding the
///    project allow-list, which under `dontAsk` denies every write while
///    looking like a capability failure. It outranks the envelope's own
///    verdict, because a `success` envelope alongside denials describes a run
///    that was blocked from doing what it was asked.
/// 3. **The last envelope's `subtype` / `is_error` / `terminal_reason`**,
///    matched as string slices with an explicit fallback arm that carries the
///    observed values verbatim. This CLI shipped three new values for those
///    fields on one version line; a typed classification would need a catch-all
///    on each and would still discard the actual string a support report needs
///    (D-32).
/// 4. **The exit status**, read as a liveness and crash signal only. Where it
///    disagrees with a success envelope the disagreement is *surfaced*, never
///    silently resolved in either direction (D-10).
/// 5. **The disk and git delta**, as the corroboration half: a success envelope
///    with no signal is the distinct no-op outcome, not a success (D-11).
fn derive(
    turns: &[TurnOutcome],
    denials: Vec<serde_json::Value>,
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

    if !denials.is_empty() {
        return RunOutcome::PermissionDenied { denials };
    }

    let terminal_reason = last.terminal_reason.as_deref();

    match (last.subtype.as_str(), last.is_error, terminal_reason) {
        ("success", false, Some("completed") | None) => {
            if let Some(disagreement) = describe_exit_disagreement(exit) {
                return RunOutcome::Failed {
                    reason: disagreement,
                    subtype: Some(last.subtype.clone()),
                    terminal_reason: terminal_reason.map(str::to_string),
                    exit_code,
                };
            }

            if DiskDelta::between(before, after).made_changes() {
                RunOutcome::SucceededWithChanges {
                    turns: turns.to_vec(),
                    total_cost_usd: run_cost_usd(turns),
                }
            } else {
                RunOutcome::SucceededNoChanges {
                    turns: turns.to_vec(),
                    total_cost_usd: run_cost_usd(turns),
                }
            }
        }
        // The classic hook-hang signature: the turn aborted mid-tool-use and the
        // process was reaped by an EXTERNAL bound. The terminal reason is what
        // classifies this, not the exit code — the reproduced hang exited 124
        // because `timeout` fired, and Claude never chose that status (D-10).
        (_, _, Some("aborted_tools")) => RunOutcome::TimedOut {
            // The cap that was breached is not knowable from the four
            // derivation sources; the supervisor that *enforces* a deadline
            // constructs this variant with its real cap. Zero here means
            // "externally bounded, duration unknown to the derivation".
            after: Duration::ZERO,
        },
        // An interrupt landed while the turn was streaming. This is the real
        // confirmation of a cancellation — a `control_response` of `success`
        // means only that the request was accepted (D-31, Pitfall D).
        (_, _, Some("aborted_streaming")) => RunOutcome::Killed {
            turns: turns.to_vec(),
        },
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

/// Describe a disagreement between a success envelope and the exit status.
///
/// `None` when there is nothing to report: either the status was never observed
/// (a failed `wait()` leaves the liveness signal *unknown*, which is not the
/// same as a contradiction) or the process exited cleanly.
fn describe_exit_disagreement(exit: Option<ExitStatus>) -> Option<String> {
    let status = exit?;
    if status.success() {
        return None;
    }

    let observed = match status.code() {
        Some(code) => format!("code {code}"),
        // No code on Unix means the process was terminated by a signal.
        None => "a signal".to_string(),
    };

    Some(format!(
        "the last turn reported success but the process exited with {observed} — the \
         envelope and the exit status disagree, and a run is never reported as \
         succeeded on the strength of one source alone"
    ))
}

/// A human-readable classification for a non-success terminal envelope.
///
/// The two aborted terminal reasons are intercepted by their own arms above and
/// never reach here.
fn describe_failure(subtype: &str, terminal_reason: Option<&str>) -> String {
    match (subtype, terminal_reason) {
        ("error_max_budget_usd", _) => "the run stopped at its budget ceiling".to_string(),
        ("error_max_turns", _) => "the run stopped at its turn ceiling".to_string(),
        (subtype, Some(reason)) => format!("the run failed: {subtype} / {reason}"),
        (subtype, None) => format!("the run failed: {subtype}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::executor::stream_json::{parse_line, Envelope, StreamMessage};

    // Compile-time fixture loading: the matrix does no filesystem I/O. Every
    // one of these is a real 2.1.220 capture, so a failure here means the
    // derivation is wrong, not that the test is wrong.
    const T01: &str = include_str!("../../tests/fixtures/transcripts/01-success-textonly.ndjson");
    const T02: &str = include_str!("../../tests/fixtures/transcripts/02-budget-exhausted.ndjson");
    const T04: &str =
        include_str!("../../tests/fixtures/transcripts/04-hookhang-aborted-tools.ndjson");
    const T05: &str =
        include_str!("../../tests/fixtures/transcripts/05-queued-injection-two-turns.ndjson");
    const T06: &str =
        include_str!("../../tests/fixtures/transcripts/06-interrupt-aborted-streaming.ndjson");
    const T08: &str =
        include_str!("../../tests/fixtures/transcripts/08-tooluse-queued-two-turns.ndjson");

    /// Every `result` envelope of a transcript, in stream order.
    fn envelopes(transcript: &str) -> Vec<ResultMessage> {
        transcript
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| match parse_line(line) {
                Envelope::Parsed {
                    msg: StreamMessage::Result(result),
                    ..
                } => Some(*result),
                _ => None,
            })
            .collect()
    }

    /// One `result` envelope parsed from a raw line.
    fn envelope_from(raw: &str) -> ResultMessage {
        match parse_line(raw) {
            Envelope::Parsed {
                msg: StreamMessage::Result(result),
                ..
            } => *result,
            other => panic!("expected a result envelope, got: {other:?}"),
        }
    }

    /// The single `result` line of a transcript, as raw text — so a test can
    /// mutate one field of a real capture rather than invent a whole envelope.
    fn result_line(transcript: &str) -> &str {
        transcript
            .lines()
            .find(|line| line.contains(r#""type":"result""#))
            .expect("the transcript carries a result envelope")
    }

    fn turns_of(transcript: &str) -> Vec<TurnOutcome> {
        envelopes(transcript)
            .iter()
            .map(TurnOutcome::from_result)
            .collect()
    }

    /// A process exit status carrying `code`. `std::process::ExitStatus` has no
    /// portable constructor, so both platform extensions are used.
    #[cfg(unix)]
    fn exit_status(code: i32) -> Option<ExitStatus> {
        use std::os::unix::process::ExitStatusExt;
        // The raw wait status packs the exit code into the high byte.
        Some(ExitStatus::from_raw(code << 8))
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> Option<ExitStatus> {
        use std::os::windows::process::ExitStatusExt;
        Some(ExitStatus::from_raw(code as u32))
    }

    /// A snapshot pair where nothing moved.
    fn no_delta() -> (RunSnapshot, RunSnapshot) {
        (snapshot(), snapshot())
    }

    /// A snapshot pair where a planning artifact moved.
    fn artifact_delta() -> (RunSnapshot, RunSnapshot) {
        let before = snapshot();
        let mut after = snapshot();
        after.project_state.current_phase = "15".to_string();
        (before, after)
    }

    fn snapshot() -> RunSnapshot {
        RunSnapshot {
            head_sha: None,
            dirty: None,
            project_state: ProjectState::default(),
        }
    }

    /// A synthetic `result` envelope carrying exactly these verdict fields, and
    /// an empty denials array.
    ///
    /// Formatted as a raw line and parsed through [`envelope_from`], never
    /// built by struct update: `ResultMessage` derives no `Default`, and
    /// building a test envelope from a line is the house idiom here because it
    /// is the same path the wire takes. `terminal_reason` is emitted only when
    /// `Some`, so an absent reason is genuinely absent rather than null.
    fn envelope(subtype: &str, is_error: bool, terminal_reason: Option<&str>) -> ResultMessage {
        let reason = match terminal_reason {
            Some(reason) => format!(r#""terminal_reason":"{reason}","#),
            None => String::new(),
        };
        envelope_from(&format!(
            r#"{{"type":"result","subtype":"{subtype}","is_error":{is_error},{reason}"session_id":"s","num_turns":1,"total_cost_usd":0.5,"permission_denials":[]}}"#
        ))
    }

    #[test]
    fn a_stream_with_no_terminal_envelope_is_never_a_success() {
        let outcome = derive_run_outcome_from_envelopes(&[], None, &snapshot(), &snapshot());
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
        let outcome = derive_run_outcome_from_envelopes(
            &[envelope("success", false, Some("completed"))],
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
        let outcome = derive_run_outcome_from_envelopes(
            &[envelope("success", false, Some("completed"))],
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
        let outcome = derive_run_outcome_from_envelopes(
            &[envelope(
                "error_from_a_future_version",
                true,
                Some("who_knows"),
            )],
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
        let outcome = derive_run_outcome_from_envelopes(
            &[
                envelope("success", false, Some("completed")),
                envelope("error_during_execution", true, Some("aborted_streaming")),
            ],
            None,
            &snapshot(),
            &snapshot(),
        );
        // The last envelope was interrupted while streaming, so the run reads as
        // killed — not as the success the FIRST envelope reported.
        assert!(
            matches!(outcome, RunOutcome::Killed { .. }),
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

    // ========================================================================
    // The derivation matrix (D-26): one test per distinguishable combination of
    // subtype x is_error x terminal_reason x exit code x disk-changed.
    // ========================================================================

    #[test]
    fn fixture_01_success_completed_exit_zero_with_changes_is_a_success_with_changes() {
        let (before, after) = artifact_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T01), exit_status(0), &before, &after);
        assert!(
            matches!(outcome, RunOutcome::SucceededWithChanges { .. }),
            "the clean baseline plus a real delta is a success with changes, got: {outcome:?}"
        );
    }

    #[test]
    fn success_without_changes_is_noop() {
        let (before, after) = no_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T01), exit_status(0), &before, &after);
        assert!(
            matches!(outcome, RunOutcome::SucceededNoChanges { .. }),
            "an envelope saying success while nothing moved on disk or in git is a no-op, \
             not a success — this is ROADMAP success criterion 2, got: {outcome:?}"
        );
    }

    #[test]
    fn a_moved_head_alone_is_enough_for_a_success_with_changes() {
        let mut before = snapshot();
        before.head_sha = Some("aaaaaaa".to_string());
        before.dirty = Some(false);
        let mut after = before.clone();
        after.head_sha = Some("bbbbbbb".to_string());

        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T01), exit_status(0), &before, &after);
        assert!(
            matches!(outcome, RunOutcome::SucceededWithChanges { .. }),
            "artifacts unchanged but HEAD moved: any one of the three signals is \
             sufficient, got: {outcome:?}"
        );
    }

    #[test]
    fn fixture_02_budget_exhausted_is_a_failure_classified_as_the_budget_ceiling() {
        let (before, after) = no_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T02), exit_status(1), &before, &after);
        match outcome {
            RunOutcome::Failed {
                reason,
                subtype,
                terminal_reason,
                exit_code,
            } => {
                assert_eq!(subtype.as_deref(), Some("error_max_budget_usd"));
                assert_eq!(terminal_reason.as_deref(), Some("budget_exhausted"));
                assert_eq!(exit_code, Some(1));
                assert!(
                    reason.contains("budget"),
                    "the classification must name the budget ceiling, got: {reason}"
                );
            }
            other => panic!("expected a budget failure, got: {other:?}"),
        }
    }

    #[test]
    fn fixture_02_absent_result_field_never_aborts_the_derivation() {
        let envelope = envelopes(T02);
        let envelope = envelope.first().expect("the budget transcript has a result");
        assert!(
            envelope.result.is_none(),
            "precondition: the budget envelope omits the prose field entirely (D-32)"
        );

        let (before, after) = no_delta();
        let outcome = derive_run_outcome_from_envelopes(
            std::slice::from_ref(envelope),
            exit_status(1),
            &before,
            &after,
        );
        assert!(
            matches!(outcome, RunOutcome::Failed { .. }),
            "an absent prose field must classify normally, never panic, got: {outcome:?}"
        );
    }

    #[test]
    fn fixture_04_aborted_tools_with_exit_124_is_a_timeout_not_a_claude_verdict() {
        let (before, after) = no_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T04), exit_status(124), &before, &after);
        assert!(
            matches!(outcome, RunOutcome::TimedOut { .. }),
            "exit 124 came from an EXTERNAL timeout, never from Claude, and the \
             aborted-tools terminal reason is what classifies it (D-10), got: {outcome:?}"
        );
    }

    #[test]
    fn fixture_06_aborted_streaming_with_exit_1_is_a_kill_classified_as_interrupted() {
        let (before, after) = no_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T06), exit_status(1), &before, &after);
        match outcome {
            RunOutcome::Killed { turns } => assert_eq!(
                turns.len(),
                1,
                "the interrupt transcript closes exactly one turn"
            ),
            other => panic!("an interrupted stream is a kill, got: {other:?}"),
        }
    }

    #[test]
    fn a_max_turns_envelope_is_a_failure_classified_as_the_turn_ceiling() {
        let raw = result_line(T01).replace(
            r#""subtype":"success""#,
            r#""subtype":"error_max_turns""#,
        );
        let (before, after) = no_delta();
        let outcome = derive_run_outcome_from_envelopes(
            &[envelope_from(&raw)],
            exit_status(1),
            &before,
            &after,
        );
        match outcome {
            RunOutcome::Failed {
                reason, subtype, ..
            } => {
                assert_eq!(subtype.as_deref(), Some("error_max_turns"));
                assert!(
                    reason.contains("turn ceiling"),
                    "the classification must name the turn ceiling, got: {reason}"
                );
            }
            other => panic!("expected a turn-limit failure, got: {other:?}"),
        }
    }

    #[test]
    fn a_non_empty_permission_denials_array_is_its_own_outcome_and_carries_the_denials() {
        let raw = result_line(T01).replace(
            r#""permission_denials":[]"#,
            r#""permission_denials":[{"tool_name":"Write","rule":"workspace not trusted"}]"#,
        );
        let (before, after) = no_delta();
        let outcome = derive_run_outcome_from_envelopes(
            &[envelope_from(&raw)],
            exit_status(0),
            &before,
            &after,
        );
        match outcome {
            RunOutcome::PermissionDenied { denials } => {
                assert_eq!(denials.len(), 1, "the denial records must be carried");
                assert_eq!(
                    denials[0].get("tool_name").and_then(|v| v.as_str()),
                    Some("Write"),
                    "the driver must be able to say WHICH tool was denied (D-10)"
                );
            }
            other => panic!(
                "a populated denials array outranks a success envelope — it is the \
                 untrusted-workspace tell the spike found (P2), got: {other:?}"
            ),
        }
    }

    #[test]
    fn a_success_envelope_with_a_non_zero_exit_code_surfaces_the_disagreement() {
        let (before, after) = artifact_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T01), exit_status(1), &before, &after);
        match outcome {
            RunOutcome::Failed {
                reason,
                subtype,
                exit_code,
                ..
            } => {
                assert_eq!(subtype.as_deref(), Some("success"));
                assert_eq!(exit_code, Some(1));
                assert!(
                    reason.contains("disagree"),
                    "the disagreement between the envelope and the exit status must be \
                     surfaced, not silently resolved, got: {reason}"
                );
            }
            other => panic!(
                "a success envelope must not silently become a success when the process \
                 exited non-zero, got: {other:?}"
            ),
        }
    }

    #[test]
    fn a_failure_envelope_with_exit_zero_is_still_a_failure() {
        let (before, after) = artifact_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes(T02), exit_status(0), &before, &after);
        assert!(
            matches!(outcome, RunOutcome::Failed { .. }),
            "the exit code is a liveness signal, never the authoritative verdict (D-10), \
             got: {outcome:?}"
        );
    }

    #[test]
    fn an_unobserved_exit_status_is_not_read_as_a_disagreement() {
        let (before, after) = artifact_delta();
        let outcome = derive_run_outcome_from_envelopes(&envelopes(T01), None, &before, &after);
        assert!(
            matches!(outcome, RunOutcome::SucceededWithChanges { .. }),
            "a failed wait() means the exit status is unknown, not that it disagreed, \
             got: {outcome:?}"
        );
    }

    #[test]
    fn zero_terminal_envelopes_is_a_failure_even_on_a_clean_exit() {
        for exit in [exit_status(0), None] {
            let (before, after) = artifact_delta();
            let outcome = derive_run_outcome_from_envelopes(&[], exit, &before, &after);
            match outcome {
                RunOutcome::Failed { reason, .. } => assert!(
                    reason.contains("terminal `result` envelope"),
                    "the failure must name the missing envelope, got: {reason}"
                ),
                other => panic!(
                    "a stream that closed with nothing to corroborate is never a success, \
                     got: {other:?}"
                ),
            }
        }
    }

    // ========================================================================
    // Multi-turn: result closes a TURN, not the run (D-29)
    // ========================================================================

    #[test]
    fn multi_turn_yields_exactly_one_run_outcome_derived_from_the_last_envelope() {
        let envelopes = envelopes(T05);
        assert_eq!(
            envelopes.len(),
            2,
            "precondition: fixture 05 carries two terminal envelopes in one process"
        );
        let last_cost = envelopes[1].total_cost_usd.expect("the last cost");

        let (before, after) = artifact_delta();
        let outcome =
            derive_run_outcome_from_envelopes(&envelopes, exit_status(0), &before, &after);

        match outcome {
            RunOutcome::SucceededWithChanges {
                turns,
                total_cost_usd,
            } => {
                assert_eq!(
                    turns.len(),
                    2,
                    "both turns are carried on the single run outcome"
                );
                assert_eq!(
                    total_cost_usd,
                    Some(last_cost),
                    "the run verdict and its cost come from the LAST envelope (D-29)"
                );
            }
            other => panic!("two envelopes must yield ONE run outcome, got: {other:?}"),
        }
    }

    #[test]
    fn the_run_turn_count_is_summed_while_the_run_cost_comes_from_the_last_envelope() {
        // Fixture 05: num_turns resets to 1 on both envelopes, cost accumulates.
        let turns = turns_of(T05);
        assert_eq!(
            run_turn_count(&turns),
            2,
            "num_turns is per-turn and resets, so a run-level count must be SUMMED (D-29)"
        );
        let envelopes = envelopes(T05);
        assert_eq!(
            run_cost_usd(&turns),
            envelopes[1].total_cost_usd,
            "total_cost_usd accumulates, so the run cost is the LAST envelope's value"
        );
        assert!(
            run_cost_usd(&turns) > envelopes[0].total_cost_usd,
            "and it is strictly greater than the first envelope's"
        );

        // Fixture 08: the tool round-trip makes turn 2 report 2 internal turns.
        let turns = turns_of(T08);
        assert_eq!(
            run_turn_count(&turns),
            3,
            "1 on turn 1 plus 2 on turn 2 — the A3 sub-probe's observed counts"
        );
    }

    #[test]
    fn a_later_failing_turn_overrides_an_earlier_succeeding_one() {
        let (before, after) = artifact_delta();
        let outcome = derive_run_outcome_from_envelopes(
            &[
                envelope("success", false, Some("completed")),
                envelope("error_max_budget_usd", true, Some("budget_exhausted")),
            ],
            exit_status(1),
            &before,
            &after,
        );
        assert!(
            matches!(outcome, RunOutcome::Failed { .. }),
            "an executor that returns on the FIRST envelope truncates every steered run \
             while reporting success (Pitfall A), got: {outcome:?}"
        );
    }

    // ========================================================================
    // The prose is never a derivation source (T-15-20, TRANS-02)
    // ========================================================================

    #[test]
    fn the_agents_prose_summary_changes_nothing_about_the_outcome() {
        let honest = envelope_from(result_line(T01));
        let lying = envelope_from(
            &result_line(T01).replace(
                r#""result":"PONG""#,
                r#""result":"I rewrote every file in the repository""#,
            ),
        );
        assert_ne!(
            honest.result, lying.result,
            "precondition: the two envelopes differ only in their prose"
        );

        let (before, after) = no_delta();
        assert_eq!(
            derive_run_outcome_from_envelopes(&[honest], exit_status(0), &before, &after),
            derive_run_outcome_from_envelopes(&[lying], exit_status(0), &before, &after),
            "no branch of the derivation may read the model-authored summary (D-10)"
        );
    }
}
