use crate::state_reader::git_ops::{GitDiffStat, GitLogEntry};
use crate::state_reader::ProjectState;
use crossterm::event::KeyEvent;

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
    /// Every field is plain data — `String`, `Vec`, and a cursor of `u64` —
    /// so `Action` stays `Clone` and no file handle or join handle leaks into
    /// a message type (D-20).
    DriverJournalAppended {
        alias: String,
        run_id: String,
        records: Vec<crate::journal::reader::JournalRecord>,
        cursor: crate::journal::reader::TailCursor,
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
    /// A stop finished, with its outcome already rendered.
    ///
    /// The outcome is a `String` rather than the
    /// [`StopOutcome`](crate::driver::kill::StopOutcome) itself for one reason:
    /// that type is `#[cfg(unix)]`, and `Action` is a cross-platform message
    /// type this file keeps free of platform attributes. Rendering happens at
    /// the seam that produced it, where the state is still in hand.
    DriverStopped {
        alias: String,
        run_id: String,
        outcome: String,
    },
}
