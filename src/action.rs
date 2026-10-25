use crate::state_reader::git_ops::{GitCommitDetail, GitLogEntry};
use crate::state_reader::ProjectState;
use crossterm::event::KeyEvent;

/// Why creating a new run produced no run directory (30-05, D-18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewRunFailure {
    /// The run id (carried) already exists; nothing was written.
    Exists(String),
    /// Anything else, already worded for the status line.
    Other(String),
}

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    RawKey(KeyEvent),
    /// A mouse event that passed the reader filter (quick 260926-dyf).
    ///
    /// Only a left press and vertical wheel steps are ever sent:
    /// `ui::mouse::routed` drops motion, drag, release, the other buttons and
    /// horizontal scroll in the reader, before the Action FIFO (I-5, T-dyf-01).
    Mouse(crossterm::event::MouseEvent),
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
    /// One commit's message AND file stat, loaded together (260916-vr1).
    ///
    /// **`hash` is the hash the load was ASKED for**, carried back so the
    /// handler can drop a payload that arrived for a commit the user has since
    /// scrolled off (QD-07). Without it, a slow `git show` re-opens the pane
    /// for whatever row happens to be selected when it lands.
    ///
    /// One action rather than the two this replaced: the pane wants both halves
    /// of the same commit, and two independent loads can arrive in either order
    /// under two different selections.
    GitCommitDetailLoaded {
        alias: String,
        hash: String,
        detail: GitCommitDetail,
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
    /// One launcher-marker scan completed (30-03, D-12, D-13; renamed in Phase 31).
    ///
    /// The payload is the **whole** result, not a delta, because the scan is
    /// authoritative: a project whose launcher runs ended, or which was
    /// unregistered between two scans, is expressed by its absence and by nothing
    /// else. The handler replaces rather than merges for exactly that reason.
    ///
    /// Every field is plain data, so `Action` stays `Clone` and no file handle or
    /// join handle leaks into a message type (D-20). The scan is read-only, so
    /// there is no handle to leak in the first place (D-12).
    LauncherMarkersRescanned {
        /// Each registered project's launcher-run marker (30-03, D-12, D-13), keyed by
        /// alias: the highest of crashed, parked, driven, stale and unknown across the
        /// project's launcher runs, and whether any holds its run lock.
        ///
        /// Computed in a `spawn_blocking` task on the 20-tick counter, so a launcher
        /// killed with `SIGKILL` (no file event) turns into a crashed badge within one
        /// slow tick. Replaced wholesale: a project absent from the map has no marker.
        /// **Empty with the experimental flag off**, since no launcher surface is shown
        /// then and the scan's file reads are skipped.
        launcher_markers:
            std::collections::HashMap<String, crate::ui::screens::driver_run::LauncherMarker>,
    },
    /// One running-agents scan completed (AGENT-05, D-C01).
    ///
    /// The payload is the **whole** scan over the registered projects, not a
    /// delta, because the scan is authoritative: a project whose agents all
    /// ended, or which was unregistered between two scans, is expressed by its
    /// absence and by nothing else. The handler replaces
    /// `AppContext.agent_views` rather than merging into it for exactly that
    /// reason — the rule [`Action::LauncherMarkersRescanned`] follows.
    ///
    /// Every field of every `ProjectAgents` is plain data — paths, counts,
    /// strings and times — never a file handle, a child process or a join
    /// handle, so `Action` stays `Clone` (D-20). The scan is read-only, so
    /// there is no handle to leak in the first place.
    ///
    /// No `Box`: a `Vec` is a fixed 24 bytes whatever it holds, so this variant
    /// is far below the `clippy::large_enum_variant` threshold (RESEARCH §8.3
    /// measured it firing on a ~200-byte difference between variants).
    AgentsScanned {
        per_project: Vec<(String, crate::agents::ProjectAgents)>,
    },
    /// The project's launcher runs were listed off the render thread (30-01, D-13).
    ///
    /// The whole result, **replaced and never merged** into
    /// `ProjectViewCache::launcher_runs`: a run directory no longer on disk is
    /// expressed by its absence. Each row carries the status `read_driver_run`
    /// derived from the run lock, so a lock released by `SIGKILL` (which emits no
    /// file event) reads as crashed on the next listing. Produced only by
    /// `AppContext::schedule_launcher_refresh`, the single producer of launcher
    /// rows.
    LauncherRunsRefreshed {
        alias: String,
        launcher_runs: Vec<crate::state_reader::driver_frontmatter::DriverRunEntry>,
        /// The selected run's `DIRECTION.md` read in the same blocking task (30-02,
        /// D-06, D-25): the run the pinned selection resolves to once the listing is
        /// known. `None` when that run could not be read (its row is then an `Err`).
        /// Applied only when `detail.run_id` is still the selected run's.
        detail: Option<Box<crate::ui::screens::driver_run::RunDetail>>,
        /// What was appended to that run's `LOG.md` since the cursor the cache held
        /// when the task was scheduled (30-02, D-23). Applied only if the cache's
        /// cursor is still where the read started.
        log: Option<Box<crate::ui::screens::driver_run::LogRead>>,
    },
    /// One launcher run was re-read off the render thread (30-02, D-24, D-25).
    ///
    /// Sent by `AppContext::schedule_selected_run_refresh` for a `DriverRun` watcher
    /// event that names the *selected* run: one `DIRECTION.md` read with the lock
    /// probe replaces that run's list row and the cached detail, instead of
    /// re-listing all twenty. `Err` carries the reader's display text; the handler
    /// answers it with a full re-listing, which produces the typed unreadable row.
    LauncherRunRefreshed {
        alias: String,
        run_id: String,
        detail: Result<Box<crate::ui::screens::driver_run::RunDetail>, String>,
        /// The run's `LOG.md` delta, read in the same task (30-02, D-23).
        log: Option<Box<crate::ui::screens::driver_run::LogRead>>,
    },
    /// One project's launcher-run marker was re-scanned off the render thread
    /// (30-03, D-13).
    ///
    /// Sent by `AppContext::schedule_launcher_marker` for a `ChangeKind::DriverRun`
    /// watcher event on a project whose Driver tab is NOT on top (when it is, the
    /// launcher-run refresh handlers recompute the marker from the cached rows). One
    /// `list_driver_runs` for one project, reduced by `launcher_marker`; the 20-tick
    /// scan still replaces every marker wholesale. Plain data, so `Action` stays
    /// `Clone`.
    LauncherMarkerScanned {
        alias: String,
        marker: crate::ui::screens::driver_run::LauncherMarker,
    },
    /// The user confirmed stopping launcher run `run_id` of `alias` from the Driver
    /// tab (28-07, D-11).
    ///
    /// Two `String`s, plain data. The handler runs `launcher::stop::stop_run` on
    /// `spawn_blocking` (it waits on a lock and on `systemctl`, so never on the
    /// render thread) and answers with [`Action::LauncherStopped`]. The run id
    /// travels from the confirmation screen, which is the id it named to the user
    /// (T-28-18).
    LauncherStopRequested {
        alias: String,
        run_id: String,
    },
    /// A launcher stop finished. `report` is the already-rendered outcome: the
    /// `StopReport` display, or the error's.
    LauncherStopped {
        alias: String,
        run_id: String,
        report: String,
    },
    /// A Driver-tab steering key asked for one edit to a launcher run's
    /// `DIRECTION.md` body (30-04, D-14, D-15).
    ///
    /// Sent by a chooser or input screen, which touched no file. The handler resolves
    /// the run directory from the registered project and the listed run id, runs
    /// `driver::steer::steer_run` (a `write_body` closure under `DIRECTION.lock`) on
    /// `spawn_blocking`, and answers [`Action::DriverSteered`]. Plain data, so
    /// `Action` stays `Clone`.
    DriverSteerRequested {
        alias: String,
        run_id: String,
        op: crate::driver::steer::SteerOp,
    },
    /// A steering write finished (30-04). `outcome` says what happened to the file;
    /// the handler words it for the status line and schedules a re-read of the run.
    DriverSteered {
        alias: String,
        run_id: String,
        op: crate::driver::steer::SteerOp,
        outcome: crate::driver::steer::SteerOutcome,
    },
    /// `e` on a readable launcher run (30-04, D-17). The handler makes the body-only
    /// edit temp on `spawn_blocking` and answers [`Action::DriverEditReady`],
    /// [`Action::DriverEditStale`] or [`Action::DriverEditFailed`].
    DriverEditRequested {
        alias: String,
        run_id: String,
    },
    /// The edit temp exists. `base` is the body it was written from (or, after a
    /// re-open, the live body read just now): the apply is checked against it. The
    /// handler records the session and asks the main loop to open `temp` in the
    /// user's editor.
    DriverEditReady {
        alias: String,
        run_id: String,
        temp: std::path::PathBuf,
        base: String,
    },
    /// A temp already exists (an edit a crashed TUI never applied): nothing was
    /// written or overwritten; the chooser offers re-open, discard or keep.
    DriverEditStale {
        alias: String,
        run_id: String,
        temp: std::path::PathBuf,
    },
    /// The temp could not be made; `DIRECTION.md` was not changed.
    DriverEditFailed {
        alias: String,
        run_id: String,
        error: String,
    },
    /// `r` in the edit chooser: open the kept text again, against the live body as it
    /// is now. The handler reads that body on `spawn_blocking` and answers
    /// [`Action::DriverEditReady`] with it as the new base.
    DriverEditReopen {
        alias: String,
        run_id: String,
        temp: std::path::PathBuf,
    },
    /// `y` in the lint confirmation: apply the recorded session's edit although
    /// `driver check` reported problems.
    DriverEditApply {
        temp: std::path::PathBuf,
        accept_lint: bool,
    },
    /// An apply finished. `outcome` decides the status line or the screen shown.
    DriverEditDone {
        alias: String,
        run_id: String,
        temp: std::path::PathBuf,
        outcome: crate::driver::steer::EditOutcome,
    },
    /// `d` in the edit chooser: delete the kept temp. The path removed is built from
    /// the run directory, not taken from `temp`.
    DriverEditDiscard {
        alias: String,
        run_id: String,
        temp: std::path::PathBuf,
    },
    /// The discard finished: `error` is `None` when the temp is gone.
    DriverEditDiscarded {
        alias: String,
        run_id: String,
        error: Option<String>,
    },
    /// `R` opened the launch form for `run_id` (30-05, D-20): run the host check and the
    /// `driver check` lint on `spawn_blocking` and answer
    /// [`Action::DriverLaunchPreflighted`]. Until that answer arrives the form shows
    /// `Sandbox: Loading...` and `y` does nothing.
    DriverLaunchPreflightRequested {
        alias: String,
        run_id: String,
    },
    /// The preflight finished. Plain data, stored on the project's view cache under the
    /// run id it was computed for.
    DriverLaunchPreflighted {
        alias: String,
        run_id: String,
        preflight: crate::ui::screens::driver_launch::LaunchPreflight,
    },
    /// `y` on the launch form (30-05, D-19). `args` is the confirmed `driver launch`
    /// argument set (plain data; `unsandboxed` is true only from the separate
    /// confirmation). The handler builds the launcher options and calls the launcher
    /// library's detached launch on `spawn_blocking`, then answers
    /// [`Action::DriverLaunched`]. The run it starts outlives this process.
    DriverLaunchRequested {
        alias: String,
        run_id: String,
        args: crate::cli::LaunchArgs,
    },
    /// Enter on a valid slug in the new-run input (30-05, D-18). The handler mints the
    /// run id from today's UTC date and creates the run directory with the same
    /// `driver_templates::create_run_dir` that `driver new` calls, on `spawn_blocking`,
    /// and answers [`Action::DriverNewRunCreated`].
    DriverNewRunRequested {
        alias: String,
        slug: String,
    },
    /// The run was created (`Ok(run id)`) or was not. An id that already exists is its
    /// own failure so the input can stay open with the buffer kept.
    DriverNewRunCreated {
        alias: String,
        slug: String,
        result: Result<String, NewRunFailure>,
    },
    /// The new run's DIRECTION.md was edited and `driver check` ran over it: the
    /// problems block the launch, the warnings do not.
    DriverNewRunChecked {
        alias: String,
        run_id: String,
        problems: Vec<String>,
        warnings: Vec<String>,
    },
    /// `n` on the `Check DIRECTION.md` popup: the new-run flow ends with the run created
    /// and not launched.
    DriverNewRunLeft {
        alias: String,
        run_id: String,
    },
    /// A launch finished. `Ok` carries the daemon's pid; `Err` is the already-worded
    /// failure (`launch_error_copy`), every one of which says nothing was started.
    DriverLaunched {
        alias: String,
        run_id: String,
        result: Result<u32, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_new_variant_is_plain_data_so_action_stays_clone() {
        // The compiler proves the claim; the test names it so a later field
        // addition that breaks `Clone` fails here with the reason attached.
        let refreshed = Action::LauncherRunsRefreshed {
            alias: "proj".to_string(),
            launcher_runs: Vec::new(),
            detail: None,
            log: None,
        };
        assert!(matches!(
            refreshed.clone(),
            Action::LauncherRunsRefreshed { .. }
        ));

        let one = Action::LauncherRunRefreshed {
            alias: "proj".to_string(),
            run_id: "20261020-a".to_string(),
            detail: Err("no frontmatter block".to_string()),
            log: None,
        };
        assert!(matches!(one.clone(), Action::LauncherRunRefreshed { .. }));

        let scanned = Action::LauncherMarkerScanned {
            alias: "proj".to_string(),
            marker: crate::ui::screens::driver_run::LauncherMarker::default(),
        };
        assert!(matches!(
            scanned.clone(),
            Action::LauncherMarkerScanned { .. }
        ));

        let steered = Action::DriverSteered {
            alias: "proj".to_string(),
            run_id: "20261020-a".to_string(),
            op: crate::driver::steer::SteerOp::SetControl("pause".to_string()),
            outcome: crate::driver::steer::SteerOutcome::Written {
                lock_timed_out: false,
            },
        };
        assert!(matches!(steered.clone(), Action::DriverSteered { .. }));
    }
}
