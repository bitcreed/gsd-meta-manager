use crate::state_reader::git_ops::{GitDiffStat, GitLogEntry};
use crate::state_reader::ProjectState;
use crossterm::event::KeyEvent;

/// Whether a stopped run is actually gone (WR-15, D-29).
///
/// A **state**, not a message — the same register as
/// [`StopOutcome`](crate::driver::kill::StopOutcome), whose doc records the
/// house rule from `src/error.rs`'s module header: a driver UI needs something
/// it can render rather than a string it must parse. The rendered text travels
/// **alongside** this value in [`Action::DriverStopped::outcome`] for the status
/// line, and is never parsed back out to recover the state.
///
/// Deliberately **portable**, and that is the whole reason it exists rather than
/// `StopOutcome` travelling in the message: `StopOutcome` is `#[cfg(unix)]` and
/// `Action` is a cross-platform message type this file keeps free of platform
/// attributes.
///
/// The bug it closes: `app.rs`'s handler unconditionally dropped the run from
/// `observed_runs` **and** `session_spawned_runs`, so a stop whose signal was
/// never delivered left the dashboard showing no run for up to five seconds and
/// removed the session-spawned record **permanently** — after which a later stop
/// took the `Adopted` reaping arm for a run this session did spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopDisposition {
    /// The run is gone. Mutating the observed maps is safe.
    RunGone,
    /// The run may still be live, so nothing may be dropped on this report.
    ///
    /// Covers both halves of the WR-15 reproduction: `SignalFailed` means the
    /// signal was **never delivered**, and `AlreadyGone` means **nothing was
    /// signalled** — in neither case has anything established that the run
    /// ended (D-29).
    MayStillBeLive,
}

#[cfg(unix)]
impl From<&crate::driver::kill::StopOutcome> for StopDisposition {
    fn from(outcome: &crate::driver::kill::StopOutcome) -> Self {
        use crate::driver::kill::StopOutcome;
        match outcome {
            StopOutcome::ExitedOnTerminate | StopOutcome::ExitedAfterKill => Self::RunGone,
            StopOutcome::AlreadyGone | StopOutcome::SignalFailed { .. } => Self::MayStillBeLive,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    RawKey(KeyEvent),
    Resize,
    /// A watched path under a project's `.planning/` changed.
    ///
    /// Both fields are needed and neither is redundant (D-09):
    ///
    /// * `project_path` is the directory containing `.planning/`, and is what
    ///   the alias lookup in `app.rs` keys on.
    /// * `changed_path` is the individual path that changed, and is the input
    ///   to `journal::classify_change`. Without it the handler cannot tell a
    ///   driver journal append from a `STATE.md` write, and every append pays
    ///   for a full `parse_project_state` (OBS-06).
    FileChanged {
        project_path: std::path::PathBuf,
        changed_path: std::path::PathBuf,
    },
    CreateProjectResult {
        alias: String,
        path: std::path::PathBuf,
        success: bool,
        error: Option<String>,
    },
    ProjectStateLoaded {
        alias: String,
        // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
        state: Box<ProjectState>,
    },
    GitLogLoaded {
        alias: String,
        entries: Vec<GitLogEntry>,
        planning_only: bool,
    },
    GitDiffStatLoaded {
        alias: String,
        stat: GitDiffStat,
    },
    SessionsDetected {
        sessions: Vec<crate::session_detector::ClaudeSession>,
    },
    ArchiveMilestonesDiscovered {
        alias: String,
        milestones: Vec<String>,
    },
    ArchiveLoaded {
        alias: String,
        milestone: String,
        data: crate::archive::MilestoneArchive,
    },
    /// One byte-offset tail of a run journal completed (D-12).
    ///
    /// The records travel in a `Vec` rather than by value, and that is a
    /// deliberate sizing choice rather than a habit: a `Vec` is a fixed 24
    /// bytes regardless of what it holds, so this variant is 24 + 24 + 24 + 8
    /// = 80 bytes. RESEARCH §8.3 measured `Action` at 120 bytes and measured
    /// `clippy::large_enum_variant` as firing on a 200-byte *difference*
    /// between the largest and second-largest variants — so **nothing here
    /// needs boxing** and nothing should be boxed reflexively. (The one
    /// `Box` in this file, on `ProjectStateLoaded`, is there because
    /// `ProjectState` really is 360 bytes.)
    ///
    /// Every field is plain data — `String`, `Vec`, and a `Copy` cursor of two
    /// `u64`s — so `Action` stays `Clone` and no file handle or join handle
    /// leaks into a message type (D-20). The cursor gained its second `u64` in
    /// plan 17-07 and the variant is 88 bytes rather than 80; the sizing
    /// reasoning above is unchanged by eight bytes.
    ///
    /// The cursor carries the last observed `seq` alongside the byte offset
    /// because a gap that straddles two tail reads is invisible to a check that
    /// only compares within one batch — see
    /// [`JournalCursor`](crate::journal::reader::JournalCursor) (D-28).
    DriverJournalAppended {
        alias: String,
        run_id: String,
        records: Vec<crate::journal::reader::JournalRecord>,
        cursor: crate::journal::reader::JournalCursor,
    },
    /// One reconciliation scan completed (D-13).
    ///
    /// The payload is the **whole** result, not a delta, because the scan is
    /// authoritative: a project whose run ended, or which was unregistered
    /// between two scans, is expressed by its absence and by nothing else. The
    /// handler replaces rather than merges for exactly that reason.
    ///
    /// Every field of every `ObservedRun` is plain data — ids, counts and
    /// strings — so `Action` stays `Clone` and no file handle or join handle
    /// leaks into a message type (D-20). Reconciliation is read-only, so there
    /// is no handle to leak in the first place (D-12).
    RunsReconciled {
        runs: Vec<crate::driver::reconcile::ObservedRun>,
        /// The outcome label of each project's most recent **ended** run, keyed
        /// by alias (WR-02, D-14).
        ///
        /// `reconcile_all` deliberately returns nothing for an ended run —
        /// there is nothing left to *observe* — and this is the fact that gets
        /// dropped with it. D-14 names "a finished run whose outcome is
        /// `PermissionDenied` / `Failed` / `Stalled` / `TimedOut`" as one of the
        /// four sources of the needs-a-human badge, and with no reader for it
        /// that arm could never fire in production. A badge that can never light
        /// is worse than no badge, because it teaches the user to ignore it.
        ///
        /// A **label**, not a `RunOutcome`: what is on disk is the string
        /// `run.json` recorded, and reconstructing a typed variant from it would
        /// require inventing the payload fields the label does not carry. The
        /// render layer's `TerminalState::from_label` is the one mapping.
        ///
        /// An alias whose newest run has not ended, or which has never been
        /// driven, is simply absent — the same authoritative-by-absence rule
        /// `runs` follows.
        last_outcomes: std::collections::HashMap<String, String>,
    },
    /// The user asked for a run to be started on `alias` (CTRL-03, D-25).
    ///
    /// The sibling of [`Action::DriverStopRequested`], and it exists for the
    /// same structural reason: the spawn seam is `App::start_driver_run`, a
    /// `Screen` only ever receives `&mut AppContext`, and this file's existing
    /// route from one to the other is a message. Adding the sibling rather than
    /// a second mechanism is deliberate.
    ///
    /// `command` travels with the request because the seam takes it — a driver
    /// run executes **exactly one** GSD command supplied by its caller, since
    /// the decision router is Phase 20's. Phase 18's command picker (D-23) is
    /// what makes the field carry more than one value: it seeds from
    /// `queue_md::suggest_next_commands` with
    /// `driver_confirm::DEFAULT_DRIVE_COMMAND` as the default *selection*
    /// rather than the only value.
    ///
    /// `goal` is the originating prompt, **stored verbatim and never
    /// paraphrased** (D-23, OBS-03). It is `Option<String>` rather than
    /// `String` because "the user gave no goal" and "the user gave an empty
    /// goal" are the same fact and the display renders it `(none given)` — a
    /// fabricated summary in its place is the failure D-13 exists to prevent.
    /// It flows into the `goal: Option<&str>` parameter `App::start_driver_run`
    /// already accepts, so OBS-03 needs no new plumbing below the UI.
    ///
    /// Both fields are plain data — `String` and `Option<String>` — so `Action`
    /// stays `Clone` and no file handle or join handle leaks into a message type
    /// (D-20). A `String` is 24 bytes and an `Option<String>` is 24, so this
    /// variant is 72 bytes: far under the ~200-byte *difference* RESEARCH §8.3
    /// measured `clippy::large_enum_variant` firing on, so nothing here needs
    /// boxing.
    ///
    /// **The opt-in gate is not here and is not in the handler.** It lives in
    /// the driver process, at `DrivableProject::from_registry`, so a hand-typed
    /// `gsd-meta-manager drive foo` is refused by the same code as this (D-16).
    DriverStartRequested {
        alias: String,
        command: String,
        goal: Option<String>,
    },
    /// The user asked for the live run on `alias` to be stopped (CTRL-01).
    ///
    /// Carries the alias and nothing else: the pid and the process group are
    /// read from the reconciliation scan's own observation at the moment the
    /// stop is dispatched, never carried in the message. A pgid captured when a
    /// key was pressed and used some milliseconds later is a pgid that may
    /// already belong to a different run — and this is the one value in the
    /// codebase where being stale means signalling a stranger's process group.
    DriverStopRequested {
        alias: String,
    },
    /// A stop finished, with its outcome already rendered **and** its
    /// disposition carried as a value (WR-15, D-29).
    ///
    /// The outcome is a `String` rather than the
    /// [`StopOutcome`](crate::driver::kill::StopOutcome) itself for one reason:
    /// that type is `#[cfg(unix)]`, and `Action` is a cross-platform message
    /// type this file keeps free of platform attributes. Rendering happens at
    /// the seam that produced it, where the state is still in hand.
    ///
    /// `disposition` is the half the rendered string cannot supply. **A handler
    /// that must decide whether the run is gone can only get that from a value**
    /// — recovering it by matching on the rendered text would be screen-scraping
    /// this project's own output. See [`StopDisposition`] for the bug.
    DriverStopped {
        alias: String,
        run_id: String,
        outcome: String,
        disposition: StopDisposition,
    },
    /// The user asked for `text` to be queued into the live run on `alias`
    /// (STEER-01, D-06).
    ///
    /// The message travels to the driver **through the filesystem and nothing
    /// else** (D-03): the TUI holds no handle on the run, so this action's
    /// handler appends one line to `runs/<run-id>/inbox.jsonl` on
    /// `spawn_blocking` and the driver tails it. There is no shortcut, and any
    /// design in which the TUI "sends" anywhere other than to a file cannot
    /// survive a TUI restart and therefore cannot satisfy STEER-03.
    ///
    /// `id` is generated by the TUI **at queue time**, before any other process
    /// has seen the line, which is what makes the `queued` state addressable at
    /// all. Text alone is not a correlation key: a user may legitimately send
    /// the same sentence twice, and STEER-02's states are tracked per message
    /// rather than per string (D-05).
    ///
    /// Every field is a `String` — 24 bytes each, 96 for the variant, well under
    /// the ~200-byte difference `clippy::large_enum_variant` fires on — so
    /// nothing needs boxing, `Action` stays `Clone`, and **no file handle or
    /// join handle leaks into a message type** (D-20).
    DriverInjectRequested {
        alias: String,
        run_id: String,
        id: String,
        text: String,
    },
    /// The durable append for one injected message finished (STEER-03, D-06).
    ///
    /// `error` is `Option<String>` rather than a `Result` deliberately: a
    /// `Result` in a message type invites a caller to `?` it into an unrelated
    /// error domain, where the failure loses the alias, the run and the message
    /// id that make it actionable. The option **is** the disposition and the
    /// string is the text the UI shows.
    ///
    /// The status shown for the message is not set to `queued` until this
    /// arrives with `error: None`, because STEER-03's criterion is survival of
    /// the writing process — a buffered write that dies with the TUI satisfies
    /// the UI and fails the criterion. The append pays `sync_data()` before this
    /// is sent.
    ///
    /// Plain data throughout (three `String`s and an `Option<String>`), so
    /// `Action` stays `Clone` and no handle enters the message (D-20).
    DriverInjectWritten {
        alias: String,
        run_id: String,
        id: String,
        error: Option<String>,
    },
    /// One alias's runs on disk, and the inbox for its selected run (OBS-05,
    /// STEER-02).
    ///
    /// Both payloads are **whole** rather than deltas, for the same reason
    /// [`Action::RunsReconciled`] carries the whole scan: each read is
    /// authoritative, and a run directory or a message that is no longer on
    /// disk is expressed by its absence and by nothing else.
    ///
    /// `runs` comes from [`crate::journal::list_runs`], already sorted newest
    /// first. Each [`RunSummary`](crate::journal::RunSummary) is read from that
    /// run's small committed `run.json` and **never from the journal beside
    /// it**, so listing a project holding the full retention history costs the
    /// same as listing one with a single run.
    ///
    /// A `Vec` is a fixed 24 bytes regardless of what it holds, so this variant
    /// is 72 bytes and needs no boxing (RESEARCH §8.3 measured
    /// `clippy::large_enum_variant` firing on a ~200-byte *difference*). Every
    /// field is plain data, so `Action` stays `Clone` and **no file handle or
    /// join handle leaks into a message type** (D-20) — which matters
    /// especially here, because the producer is a `spawn_blocking` task that
    /// really does hold an open file (D-28) and must drop it before sending.
    /// `journal` is the selected run's whole journal, projected into pane lines
    /// through the same bounded ring the live tail uses, plus its interjection
    /// records kept **as records**. It is what makes a finished run reviewable
    /// after the fact (OBS-05) and what makes an injected message's state a pure
    /// function of disk across a TUI restart (STEER-03).
    ///
    /// It is `Box`ed for the sizing reason the whole enum is documented by: it
    /// is the largest driver payload, and boxing keeps this variant from
    /// becoming the `clippy::large_enum_variant` outlier the file's other
    /// variants were sized to avoid. `None` when the selected run's directory
    /// could not be resolved — the fallible join on an id that came off disk
    /// (D-27).
    DriverRunsListed {
        alias: String,
        runs: Vec<crate::journal::RunSummary>,
        inbox: Vec<crate::journal::inbox::InboxMessage>,
        journal: Option<Box<crate::ui::screens::DriverRunJournal>>,
    },
    /// A dry-run report was built for `alias` and is ready to show (D-26).
    ///
    /// `report` is the already-rendered text from `dry_run::render`, not the
    /// structured report, because the pane shows it verbatim and the section
    /// headers are pinned by test. Building it shells out to `git` twice, which
    /// is why it happens on `spawn_blocking` and returns through a message
    /// rather than on the render thread — **this is one of the WR-10 call
    /// sites** the Phase 17 review named (D-28).
    ///
    /// Surfacing the report is TRANS-05-adjacent rather than required: Phase 18
    /// *surfaces* it and does not police it, and git blast-radius enforcement is
    /// Phase 19's.
    ///
    /// Three `String`s, 72 bytes, plain data — `Action` stays `Clone` and no
    /// handle enters the message (D-20).
    DriverDryRunLoaded {
        alias: String,
        command: String,
        report: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_signal_that_was_never_delivered_never_reports_the_run_as_gone() {
        use crate::driver::kill::StopOutcome;

        // The two that establish the run ended.
        for outcome in [StopOutcome::ExitedOnTerminate, StopOutcome::ExitedAfterKill] {
            assert_eq!(
                StopDisposition::from(&outcome),
                StopDisposition::RunGone,
                "{outcome:?} means the driver was observed to be gone"
            );
        }

        // The two that do not. `SignalFailed` means the signal was never
        // delivered and `AlreadyGone` means nothing was signalled — mapping
        // either to `RunGone` is precisely the WR-15 defect.
        for outcome in [
            StopOutcome::AlreadyGone,
            StopOutcome::SignalFailed {
                detail: "PermissionDenied".to_string(),
            },
        ] {
            assert_eq!(
                StopDisposition::from(&outcome),
                StopDisposition::MayStillBeLive,
                "{outcome:?} establishes nothing about whether the run ended, so \
                 the observed maps must not be mutated on it"
            );
        }
    }

    #[test]
    fn every_new_variant_is_plain_data_so_action_stays_clone() {
        // The compiler proves the claim; the test names it so a later field
        // addition that breaks `Clone` fails here with the reason attached.
        let action = Action::DriverInjectWritten {
            alias: "proj".to_string(),
            run_id: "2026-07-29T12-00-00Z-abcd".to_string(),
            id: "3f2a".to_string(),
            error: None,
        };
        let copy = action.clone();
        assert!(matches!(copy, Action::DriverInjectWritten { .. }));

        let listed = Action::DriverRunsListed {
            alias: "proj".to_string(),
            runs: Vec::new(),
            inbox: Vec::new(),
            journal: None,
        };
        assert!(matches!(listed.clone(), Action::DriverRunsListed { .. }));

        let stopped = Action::DriverStopped {
            alias: "proj".to_string(),
            run_id: "2026-07-29T12-00-00Z-abcd".to_string(),
            outcome: "stopped".to_string(),
            disposition: StopDisposition::RunGone,
        };
        assert!(matches!(stopped.clone(), Action::DriverStopped { .. }));
    }
}
