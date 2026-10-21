use crate::action::Action;
use crate::change_tracker::ChangeTracker;
use crate::config::{save_config, Config};
use crate::registry;
use crate::session_detector::ClaudeSession;
use crate::state_reader::{self, ProjectPresence, ProjectState};
use crate::ui::screens::normal::NormalScreen;
use crate::ui::screens::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DetailSubView {
    /// The Roadmap tab — index 0 and the default the detail view opens on
    /// (D-B07).
    #[default]
    RoadmapViz,
    /// The tab labelled `Phases` (index 1, D-B03). The variant keeps its old
    /// name; only the label and block titles changed.
    Pipeline,
    Backlog,
    GitHistory,
    Queue,
    Sessions,
    /// The running agents: the Sessions tab's `Agents` sub-view (D-C15), not a
    /// tab of its own. It shares Sessions' index; inside the Sessions tab it
    /// is reached with `→`, `]` or the `m` alias, and the tab's digit (or the
    /// tab-bar arrows) re-open it when it was the tab's last-used sub-tab
    /// (quick 260926-1t1). It observes only — no key on it acts on an agent.
    /// Per-project view state is in memory only, so a stored `Agents` needs no
    /// migration: it renders as Sessions with Agents active.
    Agents,
    /// The milestone archive: the Docs tab's `Milestones` sub-view (D-B04),
    /// not a tab of its own. It shares Docs' index with [`Self::Browse`];
    /// inside the Docs tab it is reached with `→`, `]` or the `m` alias, or by
    /// `Enter` on the Roadmap's collapsed shipped-milestones row, and the
    /// tab's digit re-opens it when it was the tab's last-used sub-tab (quick
    /// 260926-1t1). Per-project view state is in memory only, so a stored
    /// `Archive` needs no migration: it renders as Docs with Milestones active.
    Archive,
    Defaults,
    /// The Docs tab's `Files` sub-view (index 7) — the `.planning/` browser,
    /// and what Docs' index resolves to.
    Browse,
    /// The Driver tab: run list, run detail, live output (D-15, OBS-04).
    ///
    /// **Index 8, and fully reachable.** `TAB_COUNT` is 9, `TAB_LABELS_FULL`
    /// has nine entries, `Left`/`Right` reach it, `Shift+D` jumps to it, the
    /// footer tiers carry its hints, and both render dispatches call
    /// [`super::ui::screens::driver::render_driver_tab`].
    ///
    /// This doc said the opposite until WR-04 of plan 18's review — plan 18-09
    /// landed all five of those and the paragraph describing the staged gap was
    /// not updated with them. It is the definitional site for the enum, so a
    /// reader starting here was told the tab does not exist.
    Driver,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterColumn {
    All,
    Name,
    Phase,
    Status,
    /// Rows satisfying [`crate::ui::screens::needs_human`] (OBS-07, D-25).
    ///
    /// Unlike the three column filters this one is a **predicate**, not a
    /// column: the term still matches across every column and this narrows what
    /// survives. `/h` alone (an empty term) is therefore "every project waiting
    /// on a human" with no special case in the grammar.
    NeedsHuman,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusCategory {
    Active,
    Idle,
    Blocked,
    Complete,
    Unknown,
}

/// Split a filter string into its term and the column or predicate it narrows.
///
/// The suffix grammar is `term/x`. `h` joins `n`, `p` and `s` for "needs a
/// human" (OBS-07): the other three letters were taken and `h` is both free and
/// mnemonic. On screen the search prompt supplies its own leading `/`, so the
/// all-rows form renders as `//h` while the stored `filter_text` is `/h`.
pub fn parse_filter(input: &str) -> (String, FilterColumn) {
    if let Some(term) = input.strip_suffix("/p") {
        (term.to_string(), FilterColumn::Phase)
    } else if let Some(term) = input.strip_suffix("/n") {
        (term.to_string(), FilterColumn::Name)
    } else if let Some(term) = input.strip_suffix("/s") {
        (term.to_string(), FilterColumn::Status)
    } else if let Some(term) = input.strip_suffix("/h") {
        (term.to_string(), FilterColumn::NeedsHuman)
    } else {
        (input.to_string(), FilterColumn::All)
    }
}

/// Map a goal buffer to the `Option<&str>` `App::start_driver_run` takes, so an
/// empty one arrives as `None` and never as `Some("")` (D-23, OBS-03).
///
/// **The trap is live rather than theoretical.** The start flow's goal field is
/// optional and Step B accepts an empty buffer on `Enter`, so the common case is
/// a user who typed nothing. `drive_argv` omits `--goal` entirely for `None`,
/// but for `Some("")` it pushes the flag with an empty operand — and the driver
/// then records an empty goal into `RunRecord.goal` **as though one had been
/// given**. Those are different facts, and the display distinguishes them: no
/// goal renders `(none given)`, while a recorded empty goal renders as a blank
/// line that looks like a goal the reader simply cannot see.
///
/// Emptiness is judged by [`crate::text::carries_visible_content`] — the one
/// production spelling — while the value returned is the **untrimmed** original.
/// A goal of three spaces is no goal, and neither is a goal of zero-width
/// characters: this used to test `!text.trim().is_empty()`, which accepts
/// `U+200B` and `U+FEFF`, so a goal read back from an OLD record made only of
/// those rendered as a blank line wearing a goal's label rather than as
/// `(none given)`. A goal that WAS given is stored verbatim and never
/// paraphrased, so nothing here rewrites what the user typed (FEATURES table
/// stakes, D-23).
///
/// Pure, which is what makes the branch that matters testable without spawning a
/// driver (S6).
pub fn goal_or_none(goal: Option<&str>) -> Option<&str> {
    goal.filter(|text| crate::text::carries_visible_content(text))
}

/// Project one journal record into a live-output line, or `None` if it belongs
/// somewhere other than the output pane (D-20, OBS-04).
///
/// The five classes are the UI-SPEC's event-kind rendering table, and the
/// mapping is on `record.kind` — a plain `String`, because the reader
/// deliberately does not go through the typed `JournalEvent`, so a kind this
/// build has never seen still arrives with its payload intact (D-30).
///
/// **What is deliberately not buffered, and why that is not "losing lines".**
/// `run_started`, `exec_started` and `cost` return `None`: they are the run
/// header's data — the goal, the command, the started time, the cumulative cost
/// — and they are rendered there rather than in the scrolling pane. Putting them
/// in both would make the pane's first rows a duplicate of the header above it.
/// The reserved kinds (`observed`, `decided`, `parked`) return `None` too, for a
/// different reason: nothing emits them before Phase 20, and their surface is
/// that phase's step timeline rather than this buffer.
///
/// **Every kind that can carry a diagnostic IS buffered**, because a pane that
/// silently loses a `journal_truncated` or an `events_dropped` is exactly the
/// "looks done but isn't" failure this phase enumerates by name.
///
/// **The text is composed, never `Debug`-rendered**, and it is not sanitised
/// here: `DriverOutput::push_record` is the one append-time gate every string
/// from disk passes through, and stripping twice would be two places to keep in
/// agreement.
///
/// Pure, which is what makes the whole table assertable without a terminal (S6).
pub fn driver_line_for_record(
    record: &crate::journal::reader::JournalRecord,
) -> Option<(crate::ui::screens::DriverLineKind, String)> {
    use crate::ui::screens::DriverLineKind;

    let text = |key: &str| -> String {
        record
            .rest
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let number = |key: &str| -> u64 {
        record
            .rest
            .get(key)
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
    };

    match record.kind.as_str() {
        "exec_event" => {
            // `stderr` is present but secondary; every other stream — assistant,
            // user, turn_completed, unknown, unparseable — is the common case
            // and gets no marker, so injections and diagnostics stand out.
            let kind = if text("stream") == "stderr" {
                DriverLineKind::Stderr
            } else {
                DriverLineKind::Output
            };
            Some((kind, text("text")))
        }
        // The human's own words, echoed back into the stream they were steering.
        "interjected" => Some((DriverLineKind::Injection, text("text"))),
        // These two carry an id rather than the text — the driver correlates,
        // the TUI does not (D-08) — so the line names the transition and the
        // renderer pairs it with the message body it already holds.
        "interjection_acted_on" => Some((
            DriverLineKind::Injection,
            format!("interjection acted on: {}", text("id")),
        )),
        "interjection_missed" => Some((
            DriverLineKind::Injection,
            format!("interjection missed: {} ({})", text("id"), text("reason")),
        )),
        "diagnostic" => Some((
            DriverLineKind::Diagnostic,
            format!("{}: {}", text("code"), text("detail")),
        )),
        "events_dropped" => Some((
            DriverLineKind::Diagnostic,
            format!("{} stream events were dropped", number("count")),
        )),
        "journal_truncated" => Some((
            DriverLineKind::Diagnostic,
            format!(
                "journal truncated at {} bytes (cap {})",
                number("bytes_written"),
                number("cap")
            ),
        )),
        // The visual full stop. `RunEnded.outcome` is the four-source
        // derivation's answer and never the agent's prose (D-13).
        "run_ended" => Some((
            DriverLineKind::Terminal,
            format!("run ended: {}", text("outcome")),
        )),
        "exec_finished" => {
            let exit = record
                .rest
                .get("exit")
                .and_then(|value| value.as_i64())
                .map_or_else(
                    || "no exit status".to_string(),
                    |code| format!("exit {code}"),
                );
            Some((
                DriverLineKind::Terminal,
                format!("agent finished: {exit} after {}s", number("duration_s")),
            ))
        }
        _ => None,
    }
}

/// Fold one batch of journal records into the Driver tab's running tally.
///
/// The two facts here are the ones the run header and the step timeline need
/// that the committed `run.json` does not carry — the cumulative cost and the
/// number of turn boundaries — so they are read from the records as they arrive
/// rather than by re-parsing a journal at render time.
///
/// **The tally is reset when the run id changes**, which is what stops one run's
/// cost being displayed against another's. Both facts are evidence: `cost`
/// records carry `cumulative_usd`, and a turn boundary is an `exec_event` whose
/// stream is `turn_completed`. Neither is inferred from the agent's prose (D-13).
///
/// Pure, so the whole fold is assertable without a terminal (S6).
pub fn update_driver_tally(
    cache: &mut crate::ui::screens::ProjectViewCache,
    run_id: &str,
    records: &[crate::journal::reader::JournalRecord],
) {
    use crate::ui::screens::DriverRunTally;

    let tally = match cache.driver_tally.as_mut() {
        Some(tally) if tally.run_id == run_id => tally,
        _ => {
            cache.driver_tally = Some(DriverRunTally {
                run_id: run_id.to_string(),
                ..DriverRunTally::default()
            });
            cache.driver_tally.as_mut().expect("just assigned")
        }
    };

    for record in records {
        match record.kind.as_str() {
            "cost" => {
                if let Some(usd) = record
                    .rest
                    .get("cumulative_usd")
                    .and_then(|value| value.as_f64())
                {
                    tally.cumulative_cost_usd = Some(usd);
                }
            }
            "exec_event" => {
                let is_boundary = record
                    .rest
                    .get("stream")
                    .and_then(|value| value.as_str())
                    .is_some_and(|stream| stream == "turn_completed");
                if is_boundary {
                    tally.turn_boundaries = tally.turn_boundaries.saturating_add(1);
                }
            }
            _ => {}
        }
    }
}

/// The exact copy for an observed journal sequence gap (Copywriting Contract).
///
/// A gap means records the writer emitted were never read — the pane is missing
/// lines and cannot recover them. **Surfacing it is the point.** Before this it
/// reached a `tracing::warn!` and nothing else, i.e. a log file the user of a
/// TUI never opens, while the pane presented itself as complete.
pub fn journal_gap_line(gaps: usize) -> String {
    format!("journal gap: {gaps} record(s) not read")
}

/// Whether this tick should repaint for the elapsed-time counter (D-21).
///
/// **All three conditions are required, and the gate is the whole point.**
/// Elapsed time is `now − started_at`, so it changes every tick forever; an
/// unconditional per-tick redraw would make an idle fleet dashboard burn CPU for
/// the life of the process, which is the exact opposite of this tool's pitch.
/// The counter is only *visible* when a detail view is on top, its active
/// sub-view is the Driver tab, and the selected project actually has a live run
/// — a finished run's elapsed figure is frozen and needs no frame at all.
///
/// **No second timer.** This rides the existing 250 ms `Action::Tick`, which
/// `App::update`'s 20-tick block forbids duplicating in as many words: two
/// timers at slightly different phases would make "how stale can the dashboard
/// be?" a question with two answers.
///
/// Pure, so the truth table is assertable without a terminal (S6).
pub fn driver_elapsed_redraw_wanted(
    top_screen_name: &str,
    sub_view: Option<&DetailSubView>,
    run: Option<&crate::driver::reconcile::ObservedRun>,
) -> bool {
    driver_tab_is_on_top(top_screen_name, sub_view) && run.is_some_and(|run| run.is_live())
}

/// Whether the selected launcher run's heartbeat age needs a frame this tick (D-08).
///
/// The age is `now − heartbeat`, recomputed at render time, so it changes every
/// second a live run is on screen and a frame only on a file event would freeze it
/// (and with it the `stale` flag). Same shape as [`driver_elapsed_redraw_wanted`]:
/// the Driver tab is on top **and** the highlighted launcher run is live, so an idle
/// fleet, a finished run and every other tab repaint nothing. It rides the existing
/// 250 ms tick (no new timer) and reads only the cache (no file I/O per tick).
///
/// Pure, so its truth table is assertable without a terminal.
pub fn launcher_heartbeat_redraw_wanted(
    top_screen_name: &str,
    sub_view: Option<&DetailSubView>,
    cache: Option<&crate::ui::screens::ProjectViewCache>,
) -> bool {
    driver_tab_is_on_top(top_screen_name, sub_view)
        && crate::ui::screens::driver_run::selected_run_is_live(cache)
}

/// Whether the Driver tab is the surface the user is actually looking at.
///
/// Shared with [`driver_elapsed_redraw_wanted`] rather than written twice: the
/// two questions ("should the elapsed counter repaint" and "should the run list
/// be rescanned") have the same visibility half and differ only in whether they
/// also require a live run.
///
/// **The rescan must NOT require a live run** (CR-04), and that difference is
/// the whole reason this is its own function. `reconcile_one` returns `None` for
/// an ended run, so the alias drops out of `observed_runs` the moment the run
/// finishes — which is precisely the moment the run list's glyph, the outcome
/// and the journal snapshot all become stale and most need re-reading. A rescan
/// gated on liveness would stop firing exactly when it started mattering.
///
/// Pure, so its truth table is assertable without a terminal.
pub fn driver_tab_is_on_top(top_screen_name: &str, sub_view: Option<&DetailSubView>) -> bool {
    top_screen_name == crate::ui::screens::detail::DetailScreen::NAME
        && matches!(sub_view, Some(DetailSubView::Driver))
}

/// Every registered project's launcher-run marker (30-03, D-12, D-13).
///
/// Per project: [`list_driver_runs`](crate::state_reader::driver_frontmatter::list_driver_runs)
/// (at most [`LAUNCHER_RUNS_LISTED`](crate::ui::screens::LAUNCHER_RUNS_LISTED) runs, each
/// `DIRECTION.md` read capped at 1 MiB, one lock probe per run) reduced by
/// [`launcher_marker`](crate::ui::screens::driver_run::launcher_marker). **Blocking**: it
/// runs inside the reconciliation `spawn_blocking` task on the existing 20-tick counter,
/// never on its own timer, and never on the render thread. A project with no launcher
/// runs still gets an entry (no class), so the handler's wholesale replacement is the
/// only place a marker is cleared.
pub fn launcher_markers_for(
    projects: &HashMap<String, crate::config::RegisteredProject>,
    now: std::time::SystemTime,
) -> HashMap<String, crate::ui::screens::driver_run::LauncherMarker> {
    projects
        .iter()
        .map(|(alias, project)| {
            let entries = crate::state_reader::driver_frontmatter::list_driver_runs(
                &project.path,
                crate::ui::screens::LAUNCHER_RUNS_LISTED,
            );
            (
                alias.clone(),
                crate::ui::screens::driver_run::launcher_marker(&entries, now),
            )
        })
        .collect()
}

pub fn classify_status(status: &str) -> StatusCategory {
    // ADR-2207 status vocabulary takes precedence over the generic keyword
    // matching below. Milestone-terminal statuses are truly Complete; the
    // intermediate `All phases complete` is NOT (the milestone awaits
    // `/gsd:complete-milestone`) — classify it as Idle so it doesn't fall
    // through to the `contains("complete")` branch.
    if state_reader::state_md::is_milestone_terminal(status) {
        return StatusCategory::Complete;
    }
    if state_reader::state_md::is_all_phases_complete(status) {
        return StatusCategory::Idle;
    }
    let s = status.to_lowercase();
    if s.contains("executing") || s.contains("active") || s.contains("in progress") {
        StatusCategory::Active
    } else if s.contains("blocked") {
        StatusCategory::Blocked
    } else if s.contains("ready for verification") || s.contains("verifying") {
        // gsd-core's awaiting-verification status, in both forms: the upstream
        // body wording (`Phase complete — ready for verification`) and the
        // normalized frontmatter token (`verifying`). Neither may fall through
        // to the `complete` branch below (or to Unknown). Ranks below
        // Active/Blocked and above Complete (upstream: "verif outranks
        // complete"). Idle = a user step is next, as for `ready to plan`.
        StatusCategory::Idle
    } else if s.contains("complete") || s.contains("done") {
        StatusCategory::Complete
    } else if s.contains("idle") || s.contains("ready") || s.contains("plan") {
        StatusCategory::Idle
    } else {
        StatusCategory::Unknown
    }
}

/// The cell shown when STATE.md is present but its frontmatter is unreadable.
///
/// Deliberately not a phase. A wrong number that looks right is worse than an
/// admission — this is the whole point of tracking `state_md_unreadable`
/// separately from an absent STATE.md.
pub const UNREADABLE_STATE_LABEL: &str = "! STATE.md unreadable";

/// Prefixed onto the phase cell when STATE.md parsed only after this tool
/// repaired it in memory (`ProjectState::state_md_recovered`).
///
/// **Why a marker rather than a silent success.** The label behind it is
/// derived from a document this tool rewrote before believing; a strict reader
/// would still refuse the file, and the value shown may not be the value a
/// fixed file would show. A reader who cannot tell has been told something
/// slightly false about the repository — exactly the argument
/// [`crate::driver::untrusted`]'s truncation marker makes about a string, here
/// applied to a document.
pub const RECOVERED_STATE_MARKER: &str = "~ ";

/// The phase cell of a registered project whose folder no longer exists
/// ([`ProjectPresence::FolderMissing`]).
///
/// **Why a marker rather than a count-derived phase.** Every read of a missing
/// folder fails silently, so the counts behind a phase label are all defaults;
/// "P1: Unknown" would be a guess that looks like a fact, and would read exactly
/// like an unreadable STATE.md. The literal text keeps the state readable
/// without color, and because the dashboard filter matches against
/// [`format_phase_display`], typing `/missing` lists every such project.
pub const MISSING_FOLDER_LABEL: &str = "(missing)";

/// The phase cell of a registered project whose folder exists but has no
/// `.planning/` directory ([`ProjectPresence::NoPlanning`]).
///
/// **Distinct from [`MISSING_FOLDER_LABEL`] on purpose.** "The folder is gone"
/// and "the folder is no longer a GSD project" are different fixes for a user,
/// so they must not read the same. Still drawn red: either way the entry is not
/// a working GSD project.
///
/// **Known transient.** Right after create-project the folder exists before
/// GSD writes `.planning/`, so the row reads this for up to about 60s, until the
/// existing post-create poll sees the directory and triggers a reparse.
pub const NO_PLANNING_LABEL: &str = "(no .planning)";

/// The unreadable-state cell, naming the fault's FILE line when one is known.
///
/// The bare constant is unchanged and is still what a location-less fault
/// renders: an invented line number would be worse than none.
///
/// **The column is deliberately absent here.** The cell is width-constrained
/// and a line is what a user acts on — they open the file and go to it. The
/// column rides on the state and is rendered in the detail pane, which has the
/// room.
pub fn unreadable_state_label(
    position: Option<crate::state_reader::state_md::FrontmatterFaultPosition>,
) -> String {
    match position {
        Some(p) => format!("{UNREADABLE_STATE_LABEL} (line {})", p.line),
        None => UNREADABLE_STATE_LABEL.to_string(),
    }
}

pub fn format_phase_display(state: &ProjectState) -> String {
    // A project that is no longer on disk (or no longer has `.planning/`) has
    // no phase to show; every value behind a label would be a default.
    // Outranks the unreadable and recovered branches, which both describe a
    // file that is not there.
    match state.presence {
        ProjectPresence::FolderMissing => return MISSING_FOLDER_LABEL.to_string(),
        ProjectPresence::NoPlanning => return NO_PLANNING_LABEL.to_string(),
        ProjectPresence::Present => {}
    }
    // A phase label derived from counts while the file that names the phase is
    // unreadable is a guess wearing a fact's clothes. Say so instead.
    if state.state_md_unreadable {
        return unreadable_state_label(state.state_md_fault_position);
    }
    let label = phase_display_label(state);
    if state.state_md_recovered {
        return format!("{RECOVERED_STATE_MARKER}{label}");
    }
    label
}

/// The phase label itself, before the recovered-state marker is considered.
fn phase_display_label(state: &ProjectState) -> String {
    // ADR-2207: an explicit `current_phase_name` from STATE.md frontmatter is
    // the most accurate label; prefer it over the count-derived fallback.
    if !state.current_phase_name.is_empty() {
        return state.current_phase_name.clone();
    }
    if state.completed_phases >= state.total_phases && state.total_phases > 0 {
        if !state.milestone.is_empty() {
            return format!("{} Complete", state.milestone);
        }
        return "Complete".to_string();
    }
    let phase_num = state.active_phase_number();
    let phase_name = state
        .phases
        .iter()
        .find(|p| {
            crate::state_reader::phase_num::PhaseNum::parse(&p.number).as_ref() == Some(&phase_num)
        })
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown");
    format!("P{}: {}", phase_num, phase_name)
}

/// How the user's editor ended, as the main loop saw it (30-04, D-17).
///
/// Built by `src/main.rs` from the child's `ExitStatus` or spawn error and handed to
/// [`App::after_editor`], so the library, not the loop, decides what a failed editor
/// means for an edit temp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorExit {
    /// The editor exited with status 0.
    Success,
    /// The editor exited unsuccessfully; the string is the status as displayed.
    Failed(String),
    /// The editor could not be started; the string is the program and the error.
    SpawnError(String),
}

/// One open editor session on a run's DIRECTION.md body (30-04, D-17).
///
/// Recorded when the temp is ready and keyed by the temp's path in
/// [`App::edit_sessions`]. `base` is the body the editor session started from, which
/// the apply checks the live body against; `run_dir` is where the temp lives and where
/// the write goes. Plan data, so a later plan's new-run flow chains the same session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSession {
    pub alias: String,
    pub run_id: String,
    pub run_dir: PathBuf,
    pub base: String,
    /// The edit belongs to the new-run flow (30-05, D-18): once it is applied or found
    /// unchanged, `driver check` runs over the body and the launch form follows.
    pub then_launch: bool,
}

/// The two process-facing calls the Driver tab's launch makes, injectable so a test can
/// replace them with fakes (30-05, D-26).
///
/// `launch` is the launcher library's *detached* start: it re-executes `current_exe`, so
/// calling it from a unit test would start the test binary again (RESEARCH Pitfall 5).
/// The production value is the real function; the tests' value records the options it
/// is given and returns a pid, and the real path is proven through the real binary with
/// the same argv in `tests/driver_tab_launch_e2e.rs`. `detect` is the host probe, which
/// runs `bwrap`, `systemd-run`, `systemctl` and `claude --version`.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
pub struct LaunchSeams {
    pub launch: fn(
        &crate::launcher::run::LaunchOptions,
        Option<&Path>,
    ) -> Result<u32, crate::launcher::LauncherError>,
    pub detect: fn() -> crate::launcher::host::HostCaps,
}

#[cfg(target_os = "linux")]
impl Default for LaunchSeams {
    fn default() -> Self {
        Self {
            launch: crate::launcher::run::launch_detached,
            detect: crate::launcher::host::detect,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub needs_redraw: bool,
    pub ctx: AppContext,
    pub screen_stack: Vec<Box<dyn Screen>>,
    pub active_sessions: Vec<ClaudeSession>,
    pub session_poll_counter: u32,
    /// When set, the main loop should suspend the TUI and open this file in
    /// $EDITOR, at the 1-based line when one is carried (see [`editor_args`]).
    pub pending_editor: Option<(PathBuf, Option<usize>)>,
    /// The DIRECTION.md edit sessions awaiting their editor's exit, keyed by the temp's
    /// path (30-04, D-17). A path here is what makes [`App::after_editor`] apply it.
    pub edit_sessions: HashMap<PathBuf, EditSession>,
    /// The runs `(alias, run id)` whose next editor session is the new-run flow's
    /// (30-05, D-18): inserted when `s` creates the run, read when the edit temp is
    /// ready (it becomes [`EditSession::then_launch`]), and removed when the flow ends:
    /// the check is clean, the user leaves the run unlaunched, the editor fails, the
    /// kept text is discarded, or the run is launched.
    pub new_run_flow: std::collections::HashSet<(String, String)>,
    /// The host check and the launch call the launch form reaches (30-05).
    #[cfg(target_os = "linux")]
    pub launch_seams: LaunchSeams,
    /// The DESIRED terminal mouse-capture state (quick 260926-dyf, D-06).
    ///
    /// Initialised from `preferences.mouse` and flipped by `M`; the binary's
    /// main loop applies it to the terminal each iteration (I-14). While it is
    /// false every mouse action is ignored.
    pub mouse_capture: bool,
    /// Double-click detection (I-6); reset by every key press.
    click_tracker: crate::ui::mouse::ClickTracker,
}

/// The argv (after the program name) that opens `path` in `editor`, positioned
/// at the 1-based `line` when the editor is one whose line syntax is known
/// (quick-260924-drx).
///
/// Keyed on the editor's file name, so `/usr/bin/nvim` and `nvim` agree.
/// **An unknown editor gets the path alone** (INFERRED): a guessed `+N` handed
/// to an editor that does not understand it would open a file literally named
/// `+N` — worse than opening at the top.
pub fn editor_args(
    editor: &str,
    path: &std::path::Path,
    line: Option<usize>,
) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;
    let Some(line) = line.filter(|n| *n > 0) else {
        return vec![path.as_os_str().to_owned()];
    };
    let name = std::path::Path::new(editor)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let with_suffix = || {
        let mut arg = path.as_os_str().to_owned();
        arg.push(format!(":{line}"));
        arg
    };
    match name.as_str() {
        "vi" | "vim" | "nvim" | "view" | "gvim" | "vis" | "nano" | "pico" | "emacs"
        | "emacsclient" | "kak" | "micro" | "joe" | "jed" | "ne" | "mg" => {
            vec![
                OsString::from(format!("+{line}")),
                path.as_os_str().to_owned(),
            ]
        }
        "code" | "code-insiders" | "codium" | "vscodium" | "cursor" => {
            vec![OsString::from("-g"), with_suffix()]
        }
        "hx" | "helix" | "subl" | "zed" => vec![with_suffix()],
        _ => vec![path.as_os_str().to_owned()],
    }
}

/// The program and argv that open `path` in the user's `editor` string (WR-06, D-17).
///
/// `$VISUAL` / `$EDITOR` may carry arguments (`code -w`, `emacs -nw`,
/// `nvim -u NONE`), so the string is not always one program name. On unix this
/// follows git's `prepare_shell_cmd` rule:
///
/// - a string that names **one existing file** (`Path::is_file`, e.g. an
///   unquoted `/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl`)
///   is the program, exactly as it was spawned before;
/// - otherwise a string with no shell-special character (space, tab, newline,
///   `|&;<>()$` backtick, backslash, quotes, `*?[#~=%`) is the program too;
/// - anything else runs as `sh -c '<editor> "$@"' '<editor>' <args...>`.
///
/// **The path is never interpolated into the script.** It (and the line
/// argument) ride as separate argv elements after `$0`, so a run directory whose
/// name holds `$(...)`, `;` or quotes reaches the editor as one literal
/// argument and runs nothing (T-30-27). Line positioning is keyed on the file
/// name of the string's first word, so `nvim -u NONE` still opens at the line.
///
/// A leading `~` (`~/bin/nvim`) never spawned directly (exec does no tilde
/// expansion); it now goes through `sh` and expands. An empty string is `vi`.
///
/// Spawn-free: it builds no `Command` (the library stays off the spawn
/// allowlist; the binary builds the process from this pair) and its only system
/// call is the one `is_file` stat. Off unix the shipped pair is returned.
pub fn editor_command(
    editor: &str,
    path: &std::path::Path,
    line: Option<usize>,
) -> (std::ffi::OsString, Vec<std::ffi::OsString>) {
    use std::ffi::OsString;
    let editor = match editor.trim() {
        "" => "vi",
        trimmed => trimmed,
    };
    #[cfg(unix)]
    if !std::path::Path::new(editor).is_file() && editor.contains(is_shell_special) {
        let mut argv = vec![
            OsString::from("-c"),
            OsString::from(format!("{editor} \"$@\"")),
            OsString::from(editor),
        ];
        argv.extend(editor_args(first_word(editor), path, line));
        return (OsString::from("sh"), argv);
    }
    (OsString::from(editor), editor_args(editor, path, line))
}

/// Git's shell-special set: a string holding any of these is handed to `sh`.
#[cfg(unix)]
fn is_shell_special(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '|' | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '$'
                | '`'
                | '\\'
                | '"'
                | '\''
                | '*'
                | '?'
                | '['
                | '#'
                | '~'
                | '='
                | '%'
        )
}

/// The first word of an editor string, surrounding quotes removed: `code -w` is
/// `code`, `"/opt/My Editors/nvim" -u NONE` is `/opt/My Editors/nvim`.
#[cfg(unix)]
fn first_word(editor: &str) -> &str {
    let editor = editor.trim_start();
    for quote in ['"', '\''] {
        if let Some(rest) = editor.strip_prefix(quote) {
            return rest.split(quote).next().unwrap_or(rest);
        }
    }
    editor.split_whitespace().next().unwrap_or(editor)
}

impl App {
    /// Called when `$EDITOR` returns: re-read any cached browser or archive file that
    /// was just edited so the viewer does not show pre-edit text.
    pub fn refresh_after_editor(&mut self, edited: &std::path::Path) {
        for cache in self.ctx.view_cache.values_mut() {
            cache.reload_browser_file_if(edited);
            cache.reload_archive_file_if(edited);
        }
        self.needs_redraw = true;
    }

    /// Called by the main loop when the editor returns (30-04, D-17): the shipped
    /// refresh of any cached file, then, for a path that is an open DIRECTION.md edit
    /// session, the next step.
    ///
    /// A clean exit dispatches the apply on `spawn_blocking` (the library compares the
    /// base, lints and writes under the lock; nothing blocking happens here). A failed
    /// editor or a spawn error writes nothing, keeps the temp and says where the text
    /// is. A GUI editor that returns at once looks like a clean exit with an unchanged
    /// file, which the apply answers as "no changes" (RESEARCH Pitfall 7).
    pub fn after_editor(&mut self, path: &Path, exit: EditorExit) {
        self.refresh_after_editor(path);
        let Some(session) = self.edit_sessions.get(path).cloned() else {
            return;
        };
        match exit {
            EditorExit::Success => {
                self.start_apply(path.to_path_buf(), &session, false);
            }
            EditorExit::Failed(status) | EditorExit::SpawnError(status) => {
                self.edit_sessions.remove(path);
                // An editor failure ends the new-run flow with the run left unlaunched.
                self.new_run_flow
                    .remove(&(session.alias.clone(), session.run_id.clone()));
                self.ctx.error_message = Some(
                    crate::ui::screens::driver_steer::editor_failed_message(&status, path),
                );
                self.needs_redraw = true;
            }
        }
    }

    /// Apply the edit of `session` on `spawn_blocking` and answer
    /// [`Action::DriverEditDone`]. The temp stays on disk until the library says it was
    /// applied or found unchanged.
    fn start_apply(&mut self, temp: PathBuf, session: &EditSession, accept_lint: bool) {
        // No session exists on a build that cannot steer (the arms that open one refuse
        // first), but the apply is a DIRECTION.md write and keeps its own refusal.
        if !crate::ui::screens::driver_steer::can_steer() {
            self.refuse_steering(
                crate::ui::screens::driver_steer::STEERING_NEEDS_LINUX.to_string(),
            );
            return;
        }
        let Some(tx) = self.ctx.event_tx.clone() else {
            self.refuse_steering(format!(
                "Could not write DIRECTION.md for '{}': this session has no event channel. Nothing was written. Your text is kept at {}.",
                crate::text::render_for_terminal(&session.alias),
                crate::text::render_for_terminal(&temp.display().to_string())
            ));
            return;
        };
        let session = session.clone();
        tokio::task::spawn_blocking(move || {
            let outcome = crate::driver::steer::apply_edit(
                &session.run_dir,
                &temp,
                &session.base,
                accept_lint,
            );
            let _ = tx.send(Action::DriverEditDone {
                alias: session.alias,
                run_id: session.run_id,
                temp,
                outcome,
            });
        });
    }

    pub fn new(config_path: PathBuf) -> anyhow::Result<Self> {
        // The pruning loader, not plain `load_config`: TUI launch is where stale
        // git-worktree entries (e.g. a finished agent's `.claude/worktrees/agent-*`)
        // are removed from `config.json` (quick 260925-x0v). Log-only — no banner
        // [inferred].
        let (config, _pruned) = registry::load_config_pruning_worktrees(&config_path)?;
        Ok(Self::from_config(config, config_path))
    }

    /// Build an `App` with an empty config and no disk access.
    ///
    /// `App::new` reads `config.json` from disk, which makes it useless in a
    /// unit test: the result depends on whatever the developer or the CI runner
    /// happens to have registered. This constructor is what makes the
    /// `main_loop` property tests possible without a config file.
    pub fn new_for_test() -> Self {
        let mut app = Self::from_config(Config::new(), PathBuf::from("/nonexistent/config.json"));
        // The experimental surfaces are ON in this fixture, matching every
        // `AppContext` fixture in the tree (260917-fko). Two reasons, and the
        // second is the load-bearing one:
        //
        // 1. The driver tests written before the flag existed keep asserting
        //    exactly what they always asserted.
        // 2. `from_config` resolves the flag from the PROCESS ENVIRONMENT, so
        //    without this line every driver test's verdict would depend on
        //    whether the developer or the CI runner happened to export
        //    `GSDMM_EXPERIMENTAL_FEATURES` — the same "depends on the machine"
        //    defect this constructor exists to remove for `config.json`.
        //
        // The flag-off tests set `app.ctx.experimental = false` explicitly.
        app.ctx.experimental = true;
        app
    }

    fn from_config(config: Config, config_path: PathBuf) -> Self {
        let mouse_capture = config.preferences.mouse;
        let mut table_state = TableState::default();
        if !config.projects.is_empty() {
            table_state.select(Some(0));
        }

        let mut ctx = AppContext {
            config,
            config_path,
            project_states: HashMap::new(),
            table_state,
            filtered_aliases: Vec::new(),
            filter_text: String::new(),
            change_tracker: ChangeTracker::new(),
            detail_sub_view_per_project: HashMap::new(),
            view_cache: HashMap::new(),
            status_message: None,
            error_message: None,
            event_tx: None,
            exec_tx: None,
            run_states: HashMap::new(),
            reparse_dispatches: 0,
            journal_cursors: HashMap::new(),
            observed_runs: HashMap::new(),
            last_outcomes: HashMap::new(),
            session_spawned_runs: std::collections::HashSet::new(),
            driver_output: HashMap::new(),
            agent_views: HashMap::new(),
            agents_scan_in_flight: false,
            sort_mode: crate::ui::screens::SortMode::default(),
            watcher: None,
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            suggestion_index: 0,
            input_buffer: String::new(),
            needs_redraw: true,
            active_sessions: Vec::new(),
            archive_cache: crate::archive::ArchiveCache::default(),
            // The ONE production read of `GSDMM_EXPERIMENTAL_FEATURES`, at
            // startup, resolved into the one field every gate point reads
            // (260917-fko D1, threat T-fko-02).
            experimental: crate::experimental::experimental_features_enabled(),
        };
        ctx.filtered_aliases = ctx.sorted_aliases();

        App {
            should_quit: false,
            needs_redraw: true,
            ctx,
            screen_stack: vec![Box::new(NormalScreen::new())],
            active_sessions: Vec::new(),
            session_poll_counter: 0,
            pending_editor: None,
            edit_sessions: HashMap::new(),
            new_run_flow: std::collections::HashSet::new(),
            #[cfg(target_os = "linux")]
            launch_seams: LaunchSeams::default(),
            mouse_capture,
            click_tracker: crate::ui::mouse::ClickTracker::default(),
        }
    }

    /// Apply one executor event to the per-alias driver state (D-17, D-19).
    ///
    /// This is the executor arm's whole handler. It touches only the sibling
    /// map on `AppContext` and the redraw flag — never `ProjectState`, whose
    /// derived equality suppresses status-bar spam.
    ///
    /// Note what it deliberately does *not* do: `ExecutionEvent::Exited` moves
    /// the alias to `Stopping`, **not** to `Finished(outcome)`. A `RunOutcome`
    /// cannot be derived from an exit status alone — D-26's matrix needs the
    /// collected turns and the disk delta as well — so fabricating one here
    /// would be a lie the UI then renders. The driver that owns the run
    /// (Phase 17) is what closes the state out.
    ///
    /// Nor does `ExecutionEvent::EventsDropped` move the state at all. The
    /// count is carried for the journal (plan 16-06) and for a future Phase 18
    /// surface, and it is deliberately not allowed to move a run's state: it is
    /// emitted in the terminal path, after the process has stopped producing,
    /// so letting it reach the catch-all below would let a diagnostic
    /// manufacture liveness (D-33).
    pub fn apply_exec_event(&mut self, event: crate::main_loop::ExecEvent) {
        use crate::executor::{ExecutionEvent, RunState};

        let crate::main_loop::ExecEvent { alias, event } = event;
        let state = self.ctx.run_states.entry(alias).or_default();

        match event {
            ExecutionEvent::SessionStarted { .. } => *state = RunState::Running,
            ExecutionEvent::Exited(_) => *state = RunState::Stopping,
            // A diagnostic about what the stream *lost*, not evidence that
            // anything is still running. Explicit rather than left to the
            // catch-all, which would promote an idle alias on reasoning that
            // does not apply to a terminal-path event (D-19, D-33).
            ExecutionEvent::EventsDropped { .. } => {}
            // Any other observed line means the process is alive and talking.
            // If we somehow never saw the gated `system/init` (a torn first
            // line, say), record that a run is at least under way rather than
            // leaving the alias reading `Idle` while output streams.
            _ => {
                if *state == RunState::Idle {
                    *state = RunState::Starting;
                }
            }
        }

        self.needs_redraw = true;
    }

    /// **The only place a full project re-parse is scheduled.**
    ///
    /// That claim is not decoration — the OBS-06 test asserts
    /// `ctx.reparse_dispatches` does not move across five hundred journal
    /// appends, and the assertion is only as strong as this being the sole
    /// writer of that counter. If a second dispatch site ever appears, route it
    /// through here rather than duplicating the body.
    ///
    /// `parse_project_state` reads STATE.md, ROADMAP.md, QUEUE.md, HANDOFF,
    /// every phase directory's disk inference and every workstream, and shells
    /// out to git for last-activity. It is the bill "free watching" was quietly
    /// running up.
    ///
    /// The counter is incremented **before** the `spawn_blocking` so the count
    /// is synchronous and cannot race the blocking task (RESEARCH §7.3).
    fn schedule_reparse(&mut self, alias: &str, project_path: &Path) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let planning_dir = project_path.join(".planning");

        self.ctx.reparse_dispatches += 1;
        tokio::task::spawn_blocking(move || {
            let state = state_reader::parse_project_state(&planning_dir);
            let _ = tx.send(Action::ProjectStateLoaded {
                alias: alias_for_task,
                state: Box::new(state),
            });
        });
        self.ctx
            .last_refresh
            .insert(alias.to_string(), std::time::Instant::now());
    }

    /// Schedule a byte-offset tail of one run's journal (D-12, D-16).
    ///
    /// Note the three things this deliberately does **not** do, because they
    /// are the literal content of OBS-06:
    ///
    /// * It does **not** call `parse_project_state`. A tail costs
    ///   bytes-appended-since-the-last-read, so its cost is proportional to
    ///   actual new data rather than to project size — that proportionality is
    ///   the whole difference (D-15).
    /// * It does **not** touch `last_refresh`. Sharing the 500 ms dedup map
    ///   would let a journal append suppress a genuine `STATE.md` re-parse for
    ///   half a second, trading a performance bug for a correctness bug (D-14).
    /// * It does **not** increment `reparse_dispatches`.
    ///
    /// The read runs on `spawn_blocking` with its result returned as an
    /// `Action` on a cloned sender, following the idiom this file already uses
    /// for the re-parse and for session detection. No file I/O on the render
    /// thread (D-16).
    fn schedule_journal_tail(&mut self, alias: &str, project_path: &Path, run_id: &str) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();

        // `.copied()` — and `JournalCursor` staying `Copy` is what keeps this
        // line unchanged now that the value carries a `seq` as well as an offset
        // (D-28, PATTERNS note 4).
        let key = (alias.to_string(), run_id.to_string());
        let stored = self
            .ctx
            .journal_cursors
            .get(&key)
            .copied()
            .unwrap_or_default();
        let (alias_for_task, run_id_for_task) = key;

        let planning_dir = project_path.join(".planning");
        // No tail is scheduled for a run id that is not a plain path component
        // (D-27, WR-02). The id reaches here from the driven project's own
        // `active` file, so it is attacker-controlled for threat-modelling
        // purposes, and without this guard a traversing id would have the TUI
        // tailing an arbitrary file on the user's disk into a render surface.
        // The refusal is logged by kind only — never the path, which is the
        // untrusted value itself (D-28, PATTERNS §S3).
        let Some(paths) = crate::journal::run_paths(&planning_dir, run_id) else {
            tracing::warn!(
                alias = %alias,
                kind = "run_id_not_a_plain_component",
                "journal tail refused: the run id does not name a single directory component",
            );
            return;
        };
        let journal = paths.journal;

        tokio::task::spawn_blocking(move || {
            let read = match crate::journal::reader::tail_lines(&journal, stored.cursor) {
                Ok(read) => read,
                Err(e) => {
                    // The error KIND only. Neither the path nor the message
                    // body is logged (D-28).
                    tracing::warn!(
                        alias = %alias_for_task,
                        run_id = %run_id_for_task,
                        kind = ?e.kind(),
                        "journal tail failed",
                    );
                    return;
                }
            };

            if read.restarted {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    restarted = true,
                    "journal tail: the file shrank and the cursor was reset",
                );
            }
            if read.skipped_oversize {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    skipped_oversize = true,
                    "journal tail: stepped over a line that exceeded the read bound",
                );
            }

            let mut records = Vec::with_capacity(read.lines.len());
            let mut unparseable: u64 = 0;
            for line in &read.lines {
                match crate::journal::reader::parse_line(line) {
                    crate::journal::reader::ParsedLine::Record(record) => records.push(record),
                    // A count, never the line (D-28).
                    crate::journal::reader::ParsedLine::Unparseable { .. } => unparseable += 1,
                }
            }
            if unparseable > 0 {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    count = unparseable,
                    "journal tail: lines did not parse",
                );
            }

            // An EMPTY batch retains the previous `last_seq` rather than
            // resetting it. A tail that finds nothing new is the common case —
            // the watcher fires on a directory before the first append, and a
            // torn trailing line yields no complete record — and resetting to
            // zero on any of those would silently disarm the boundary check for
            // the next batch that does carry records (D-28).
            let last_seq = records.last().map_or(stored.last_seq, |record| record.seq);

            let _ = tx.send(Action::DriverJournalAppended {
                alias: alias_for_task,
                run_id: run_id_for_task,
                records,
                cursor: crate::journal::reader::JournalCursor {
                    cursor: read.cursor,
                    last_seq,
                },
            });
        });
    }

    // ── The Driver surface's three schedulers (D-28) ──────────────────
    //
    // All three are `pub` for the reason `start_driver_run` and
    // `stop_driver_run` are: they are the seams the UI layer reaches, and a
    // `Screen` only ever receives `&mut AppContext`, so the route from a key to
    // one of these is an `Action` this file dispatches. Two of them — the run
    // list and the dry-run preview — have no in-file caller yet because their
    // callers are the Driver tab's own key handling, which is a later plan's.
    //
    // All three follow the shipped `spawn_blocking` → `Action` shape: a
    // `let … else` guard on `event_tx`, a cloned sender, owned data moved in.
    // **No file I/O on the render thread** — that phrase is the searchable
    // marker this file already uses for the boundary (S1).

    /// Gather the three facts [`driver_elapsed_redraw_wanted`] decides on.
    ///
    /// The alias comes from `ctx.selected_alias()` rather than from the screen,
    /// because the stack holds `Box<dyn Screen>` and cannot hand back a
    /// `DetailScreen`'s field. That is not a workaround: `DetailScreen::new` is
    /// constructed from exactly this value at `normal.rs:279`, and nothing moves
    /// the dashboard's selection while a detail view is on top of the stack, so
    /// the two are the same alias by construction.
    fn driver_elapsed_redraw_due(&self) -> bool {
        let Some(top) = self.screen_stack.last() else {
            return false;
        };
        let Some(alias) = self.ctx.selected_alias() else {
            return false;
        };
        driver_elapsed_redraw_wanted(
            top.name(),
            self.ctx.detail_sub_view_per_project.get(&alias),
            self.ctx.observed_runs.get(&alias),
        )
    }

    /// Gather the facts [`launcher_heartbeat_redraw_wanted`] decides on (30-02, D-08).
    fn launcher_heartbeat_redraw_due(&self) -> bool {
        let Some(top) = self.screen_stack.last() else {
            return false;
        };
        let Some(alias) = self.ctx.selected_alias() else {
            return false;
        };
        launcher_heartbeat_redraw_wanted(
            top.name(),
            self.ctx.detail_sub_view_per_project.get(&alias),
            self.ctx.view_cache.get(&alias),
        )
    }

    /// Rescan the selected project's run list, inbox and journal **if the
    /// Driver tab is the surface on screen** (CR-04).
    ///
    /// `schedule_run_list_scan` used to be reachable from exactly two places —
    /// `switch_to_tab`'s Driver arm and `move_driver_selection` — so
    /// `driver_runs`, `driver_inbox` and `driver_journal` were frozen at the
    /// instant the tab was entered and nothing thawed them. A watcher event
    /// under `runs/<id>/` classifies as `ChangeKind::DriverJournal` and routes
    /// to `schedule_journal_tail`, which reads `journal.jsonl` and nothing else.
    /// Three ordinary-use failures followed:
    ///
    /// 1. **A run started from the tab was invisible.** The wizard hands off
    ///    without re-entering the tab, so `driver_runs` was still empty and the
    ///    pane painted *"No runs yet … Press [s] to start one"* over a live run
    ///    whose ring buffer was filling behind it.
    /// 2. **The `queued` state never rendered.** `derive_injection_states`
    ///    iterates `cache.driver_inbox`; a message queued through `i` was not in
    ///    it until a rescan, so the four-state widget did not render at all.
    /// 3. **A run that ended while you watched lost its output.** The alias
    ///    drops out of `observed_runs`, the pane switches to
    ///    `cache.driver_journal` — the snapshot taken on tab entry — and every
    ///    line since then vanished, at the exact moment an after-the-fact review
    ///    is wanted.
    ///
    /// **Gated on visibility, and deliberately so.** The scan is three blocking
    /// reads; running it for every registered project on every tick would pay a
    /// fleet-sized cost for one pane's freshness.
    fn rescan_driver_tab_if_open(&mut self) {
        let Some(top) = self.screen_stack.last() else {
            return;
        };
        let Some(alias) = self.ctx.selected_alias() else {
            return;
        };
        if !driver_tab_is_on_top(top.name(), self.ctx.detail_sub_view_per_project.get(&alias)) {
            return;
        }
        let Some(path) = self.ctx.config.projects.get(&alias).map(|p| p.path.clone()) else {
            return;
        };
        self.ctx.schedule_run_list_scan(&alias, &path);
        // The launcher rows ride the same slow tick (D-13): a lock released by
        // SIGKILL emits no file event, so this is what turns that run into
        // `crashed` within one tick while the tab is open.
        self.ctx.schedule_launcher_refresh(&alias, &path);
    }

    /// Resolve one alias's `.planning/` directory, or refuse visibly (S4).
    ///
    /// The schedulers below begin with this lookup, and they all refuse the
    /// same way: a message in `ctx.error_message` and a redraw, never a bare
    /// `return`. Sharing it is what stops the refusals drifting into several
    /// different wordings for one condition.
    fn planning_dir_for(&mut self, alias: &str) -> Option<PathBuf> {
        self.project_path_for(alias)
            .map(|path| path.join(".planning"))
    }

    /// Resolve one alias's project root, or refuse visibly (S4). The lookup
    /// [`Self::planning_dir_for`] is built on, shared so the refusal has one wording.
    fn project_path_for(&mut self, alias: &str) -> Option<PathBuf> {
        match self.ctx.config.projects.get(alias) {
            Some(project) => Some(project.path.clone()),
            None => {
                self.ctx.error_message = Some(format!("No registered project named '{alias}'"));
                self.needs_redraw = true;
                None
            }
        }
    }

    /// Stop launcher run `run_id` of `alias` (28-07, D-11).
    ///
    /// The same `launcher::stop::stop_run` the `driver stop` CLI calls, on
    /// `spawn_blocking`: it probes the run lock, signals the launcher and waits up to
    /// a minute for the lock to clear, so it must never run on the render thread
    /// (WR-10). The outcome returns as [`Action::LauncherStopped`]. Gated on the
    /// experimental surfaces like `stop_driver_run`: the key that reaches this exists
    /// only on the Driver tab.
    #[cfg(target_os = "linux")]
    fn stop_launcher_run(&mut self, alias: &str, run_id: &str) {
        if !self.ctx.experimental {
            return;
        }
        let Some(path) = self.project_path_for(alias) else {
            return;
        };
        // Refuse visibly, never silently (S4): a stop whose result cannot be
        // reported is a stop the user cannot tell happened.
        let Some(tx) = &self.ctx.event_tx else {
            self.ctx.error_message = Some(format!(
                "Could not stop the launcher run on '{alias}': this session has no event channel."
            ));
            self.needs_redraw = true;
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let run_id_for_task = run_id.to_string();

        tokio::task::spawn_blocking(move || {
            let report = match crate::launcher::stop::stop_run(
                &path,
                &run_id_for_task,
                std::time::Duration::from_secs(60),
            ) {
                Ok(report) => report.to_string(),
                Err(error) => error.to_string(),
            };
            let _ = tx.send(Action::LauncherStopped {
                alias: alias_for_task,
                run_id: run_id_for_task,
                report,
            });
        });

        self.ctx.status_message = Some((
            crate::text::render_for_terminal(&format!("Stopping launcher run {run_id} on {alias}"))
                .to_string(),
            std::time::Instant::now(),
        ));
        self.needs_redraw = true;
    }

    /// What a launch needs before it can start: the experimental gate, a registered
    /// project, a run id that names a plain directory, and a channel to report on. A
    /// failure is refused visibly (S4) and says that nothing was started.
    #[cfg(target_os = "linux")]
    fn launch_target(
        &mut self,
        alias: &str,
        run_id: &str,
    ) -> Option<(PathBuf, PathBuf, UnboundedSender<Action>)> {
        if !self.ctx.experimental {
            return None;
        }
        let project = self.project_path_for(alias)?;
        let Some(run_dir) = Self::launcher_run_dir(&project, run_id) else {
            self.refuse_steering(format!(
                "Could not launch '{}': it is not a run directory name. Nothing was started.",
                crate::text::render_for_terminal(run_id)
            ));
            return None;
        };
        let Some(tx) = self.ctx.event_tx.clone() else {
            self.refuse_steering(format!(
                "Could not launch {}: this session has no event channel. Nothing was started.",
                crate::text::render_for_terminal(run_id)
            ));
            return None;
        };
        Some((project, run_dir, tx))
    }

    /// The launch form opened: run the host check and the `driver check` lint off the
    /// render thread and answer [`Action::DriverLaunchPreflighted`]. The host probes go
    /// through the `detect` seam.
    #[cfg(target_os = "linux")]
    fn request_launch_preflight(&mut self, alias: &str, run_id: &str) {
        let Some((_project, run_dir, tx)) = self.launch_target(alias, run_id) else {
            return;
        };
        let detect = self.launch_seams.detect;
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            let caps = detect();
            let preflight = crate::ui::screens::driver_launch::launch_preflight(&run_dir, &caps);
            let _ = tx.send(Action::DriverLaunchPreflighted {
                alias,
                run_id,
                preflight,
            });
        });
    }

    /// `y` on the launch form (30-05, D-19): build the launcher options exactly as
    /// `driver launch` does and call the launcher library's detached start through the
    /// `launch` seam, on `spawn_blocking` (it waits up to the daemon's budget for the
    /// run lock). The registry path travels with it so the daemon resolves the same
    /// registry.
    ///
    /// The registry reads (`open_pr`, the project's opt-in) happen BEFORE the spawn, on
    /// this thread; the closure owns plain data. This process starts a daemon and holds
    /// nothing of the run afterwards, so closing the TUI never stops a launched run.
    #[cfg(target_os = "linux")]
    fn request_launch(&mut self, alias: &str, run_id: &str, mut args: crate::cli::LaunchArgs) {
        let Some((project, _run_dir, tx)) = self.launch_target(alias, run_id) else {
            return;
        };
        // The id and project are the ones the form named, whatever the arguments carry.
        args.run_id = run_id.to_string();
        args.project = alias.to_string();
        let open_pr = self.ctx.config.preferences.driver_open_pr();
        let opt_in = self
            .ctx
            .config
            .projects
            .get(alias)
            .and_then(|project| project.driver_opt_in.clone());
        let config_path = self.ctx.config_path.clone();
        let launch = self.launch_seams.launch;
        let (alias_for_task, run_id_for_task) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            let options = crate::ui::screens::driver_launch::launch_options_for(
                &alias_for_task,
                &project,
                &args,
                open_pr,
                opt_in.as_ref(),
            );
            let result = launch(&options, Some(&config_path)).map_err(|error| {
                crate::ui::screens::driver_launch::launch_error_copy(&run_id_for_task, &error)
            });
            let _ = tx.send(Action::DriverLaunched {
                alias: alias_for_task,
                run_id: run_id_for_task,
                result,
            });
        });
        self.ctx.status_message = Some((
            format!(
                "Launching {} on {}",
                crate::text::render_for_terminal(run_id),
                crate::text::render_for_terminal(alias)
            ),
            std::time::Instant::now(),
        ));
        self.needs_redraw = true;
    }

    /// `Enter` on a usable slug (30-05, D-18): create the run directory with the same
    /// `driver_templates::create_run_dir` `driver new` calls, on `spawn_blocking`, and
    /// answer [`Action::DriverNewRunCreated`]. `Exists` is its own failure so the input
    /// can stay open. A build that cannot write refuses with the read-only copy.
    fn request_new_run(&mut self, alias: &str, slug: &str) {
        if !self.ctx.experimental {
            return;
        }
        if !crate::ui::screens::driver_steer::can_steer() {
            self.refuse_steering(
                crate::ui::screens::driver_steer::STEERING_NEEDS_LINUX.to_string(),
            );
            return;
        }
        #[cfg(target_os = "linux")]
        {
            use crate::action::NewRunFailure;
            // A request refused before it starts goes through the one answer path, so
            // the slug input closes and says why instead of waiting for a creation that
            // never began (IN-02).
            let Some(project) = self.ctx.config.projects.get(alias).map(|p| p.path.clone()) else {
                self.finish_new_run(
                    alias,
                    Err(NewRunFailure::Other(
                        "this project is no longer registered. Nothing was created.".to_string(),
                    )),
                );
                return;
            };
            let Some(tx) = self.ctx.event_tx.clone() else {
                self.finish_new_run(
                    alias,
                    Err(NewRunFailure::Other(
                        "this session has no event channel. Nothing was created.".to_string(),
                    )),
                );
                return;
            };
            let (alias, slug) = (alias.to_string(), slug.to_string());
            tokio::task::spawn_blocking(move || {
                use crate::action::NewRunFailure;
                use crate::driver_templates::{create_run_dir, mint_run_id, NewRunError};
                let now = chrono::Utc::now();
                let result = match create_run_dir(&project, &slug, now) {
                    Ok(run) => Ok(run.run_id),
                    Err(NewRunError::Exists(_)) => Err(NewRunFailure::Exists(
                        mint_run_id(&slug, now).unwrap_or_else(|_| slug.clone()),
                    )),
                    Err(other) => Err(NewRunFailure::Other(other.to_string())),
                };
                let _ = tx.send(Action::DriverNewRunCreated {
                    alias,
                    slug,
                    result,
                });
            });
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (alias, slug);
    }

    /// A run was created (30-05, D-18): leave the slug input, select the new run, say so,
    /// mark the run's next editor session as the flow's, and open the editor on its body
    /// through the same temp-and-apply path `e` uses. An id that exists keeps the input
    /// open with the buffer; any other failure closes it and says why.
    fn finish_new_run(
        &mut self,
        alias: &str,
        result: Result<String, crate::action::NewRunFailure>,
    ) {
        use crate::action::NewRunFailure;
        use crate::ui::screens::driver_launch::{create_failed_copy, created_copy, NewRunStatus};
        let slug_on_top = self
            .screen_stack
            .last()
            .is_some_and(|screen| screen.name() == "driver_slug_input");
        match result {
            Err(NewRunFailure::Exists(run_id)) => {
                self.ctx
                    .view_cache
                    .entry(alias.to_string())
                    .or_default()
                    .new_run_status = Some(NewRunStatus::Exists(run_id));
            }
            Err(NewRunFailure::Other(error)) => {
                if slug_on_top {
                    self.screen_stack.pop();
                }
                self.ctx.error_message = Some(create_failed_copy(&error));
            }
            Ok(run_id) => {
                if slug_on_top {
                    self.screen_stack.pop();
                }
                self.new_run_flow
                    .insert((alias.to_string(), run_id.clone()));
                let cache = self.ctx.view_cache.entry(alias.to_string()).or_default();
                cache.new_run_status = None;
                cache.legacy_row_selected = false;
                cache.launcher_selected_id = Some(run_id.clone());
                if let Some(path) = self.ctx.config.projects.get(alias).map(|p| p.path.clone()) {
                    self.ctx.schedule_launcher_refresh(alias, &path);
                }
                self.ctx.status_message = Some((created_copy(&run_id), std::time::Instant::now()));
                self.request_edit(alias, &run_id);
            }
        }
        self.needs_redraw = true;
    }

    /// `driver check` over the new run's edited DIRECTION.md, on `spawn_blocking`
    /// (30-05, D-18). Answers [`Action::DriverNewRunChecked`].
    #[cfg(target_os = "linux")]
    fn request_new_run_check(&mut self, alias: &str, run_id: &str) {
        let Some((_project, run_dir, tx)) = self.launch_target(alias, run_id) else {
            return;
        };
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            use crate::driver_templates::lint::lint_direction;
            let (problems, warnings) = match crate::driver::steer::read_direction(&run_dir) {
                Ok(text) => {
                    // The editor shows the body alone, so the findings are numbered
                    // from its first line (WR-04), as `apply_edit` reports them.
                    let report =
                        lint_direction(crate::driver::direction::split_frontmatter(&text).1);
                    (
                        report.problems.iter().map(ToString::to_string).collect(),
                        report.warnings.iter().map(|f| f.as_warning()).collect(),
                    )
                }
                Err(error) => (
                    vec![format!("DIRECTION.md could not be read: {error}")],
                    Vec::new(),
                ),
            };
            let _ = tx.send(Action::DriverNewRunChecked {
                alias,
                run_id,
                problems,
                warnings,
            });
        });
    }

    #[cfg(not(target_os = "linux"))]
    fn request_new_run_check(&mut self, _alias: &str, _run_id: &str) {
        self.refuse_steering(crate::ui::screens::driver_steer::STEERING_NEEDS_LINUX.to_string());
    }

    /// The check finished: problems open the `Check DIRECTION.md` popup (launch stays
    /// blocked), a clean body opens the launch form with the CLI defaults and asks for
    /// its host check. The warnings are listed by the form's own preflight.
    fn finish_new_run_check(&mut self, alias: &str, run_id: &str, problems: Vec<String>) {
        use crate::ui::screens::driver_launch::{
            LaunchForm, LaunchFormScreen, LaunchKind, LintPopupScreen,
        };
        if !problems.is_empty() {
            self.screen_stack.push(Box::new(LintPopupScreen::new(
                alias.to_string(),
                run_id.to_string(),
                problems,
            )));
            self.needs_redraw = true;
            return;
        }
        // The flow's editing is over: a later `e` on this run is an ordinary edit.
        self.new_run_flow
            .remove(&(alias.to_string(), run_id.to_string()));
        self.ctx
            .view_cache
            .entry(alias.to_string())
            .or_default()
            .launch_preflight = None;
        self.screen_stack.push(Box::new(LaunchFormScreen::new(
            alias.to_string(),
            run_id.to_string(),
            LaunchKind::Launch,
            LaunchForm::fresh(),
        )));
        self.update(Action::DriverLaunchPreflightRequested {
            alias: alias.to_string(),
            run_id: run_id.to_string(),
        });
        self.needs_redraw = true;
    }

    /// Show a finished launch and re-list the runs so the row's status catches up.
    fn finish_launch(&mut self, alias: &str, run_id: &str, result: Result<u32, String>) {
        match result {
            Ok(_pid) => {
                self.new_run_flow
                    .remove(&(alias.to_string(), run_id.to_string()));
                self.ctx.status_message = Some((
                    format!(
                        "{}: launched {}. It runs detached and keeps running if you close this TUI.",
                        crate::text::render_for_terminal(alias),
                        crate::text::render_for_terminal(run_id)
                    ),
                    std::time::Instant::now(),
                ));
            }
            Err(copy) => self.ctx.error_message = Some(copy),
        }
        if let Some(path) = self.ctx.config.projects.get(alias).map(|p| p.path.clone()) {
            self.ctx.schedule_launcher_refresh(alias, &path);
        }
        self.needs_redraw = true;
    }

    /// The run directory of launcher run `run_id` of project `project`, or `None` when
    /// the id is not a plain directory name. The id came from the listing or from a
    /// watcher path, so it is validated before it names a directory (30-02).
    fn launcher_run_dir(project: &std::path::Path, run_id: &str) -> Option<PathBuf> {
        crate::state_reader::driver_frontmatter::is_plain_run_id(run_id).then(|| {
            project
                .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
                .join(run_id)
        })
    }

    /// Refuse a steering request visibly (S4): a write whose result cannot be
    /// reported, or that this build cannot make, says so and writes nothing.
    fn refuse_steering(&mut self, message: String) {
        self.ctx.error_message = Some(message);
        self.needs_redraw = true;
    }

    /// A launch request reached a build that cannot launch (not Linux): say so and
    /// start nothing.
    #[cfg(not(target_os = "linux"))]
    fn refuse_unsupported_launch(&mut self) {
        self.ctx.error_message = Some(format!(
            "{} Nothing was started.",
            crate::launcher::LauncherError::UnsupportedPlatform
        ));
        self.needs_redraw = true;
    }

    /// What every Driver-tab DIRECTION.md write needs before it can start: the
    /// experimental gate, a build that can write (Linux), a registered project, a run
    /// id that names a plain directory, and a channel to report back on. Each failure
    /// is refused visibly (S4) except the gate, which is silent like the stop path.
    fn steering_target(
        &mut self,
        alias: &str,
        run_id: &str,
    ) -> Option<(PathBuf, UnboundedSender<Action>)> {
        if !self.ctx.experimental {
            return None;
        }
        if !crate::ui::screens::driver_steer::can_steer() {
            self.refuse_steering(
                crate::ui::screens::driver_steer::STEERING_NEEDS_LINUX.to_string(),
            );
            return None;
        }
        let path = self.project_path_for(alias)?;
        let Some(run_dir) = Self::launcher_run_dir(&path, run_id) else {
            self.refuse_steering(format!(
                "Could not write DIRECTION.md: '{}' is not a run directory name. Nothing was written.",
                crate::text::render_for_terminal(run_id)
            ));
            return None;
        };
        let Some(tx) = self.ctx.event_tx.clone() else {
            self.refuse_steering(format!(
                "Could not write DIRECTION.md for '{}': this session has no event channel. Nothing was written.",
                crate::text::render_for_terminal(alias)
            ));
            return None;
        };
        Some((run_dir, tx))
    }

    /// Run one steering edit (30-04, D-14, D-15, D-25).
    ///
    /// **One of two DIRECTION.md writes the Driver tab makes** (the other is the editor
    /// apply): `steer::steer_run`, a `write_body` closure under `DIRECTION.lock` that
    /// keeps the `---` tool block byte-identical, on `spawn_blocking`. The key handler
    /// that asked for it touched no file. On a build that cannot steer (not Linux) the
    /// request is refused here with the read-only copy, whatever reached this arm.
    fn request_steer(&mut self, alias: &str, run_id: &str, op: crate::driver::steer::SteerOp) {
        let Some((run_dir, tx)) = self.steering_target(alias, run_id) else {
            return;
        };
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            let outcome = crate::driver::steer::steer_run(&run_dir, &op);
            let _ = tx.send(Action::DriverSteered {
                alias,
                run_id,
                op,
                outcome,
            });
        });
    }

    /// `e`: make the body-only edit temp (30-04, D-17) on `spawn_blocking`. The result
    /// is [`Action::DriverEditReady`] (the editor opens), [`Action::DriverEditStale`]
    /// (an earlier temp is there; it is never overwritten) or
    /// [`Action::DriverEditFailed`]. `DIRECTION.lock` is not held: the lock exists for
    /// single writes, not for an editor session.
    fn request_edit(&mut self, alias: &str, run_id: &str) {
        let Some((run_dir, tx)) = self.steering_target(alias, run_id) else {
            return;
        };
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            use crate::driver::steer::{prepare_edit, PrepareError};
            let action = match prepare_edit(&run_dir) {
                Ok(prepared) => Action::DriverEditReady {
                    alias,
                    run_id,
                    temp: prepared.temp,
                    base: prepared.base,
                },
                Err(PrepareError::StaleTemp(temp)) => Action::DriverEditStale {
                    alias,
                    run_id,
                    temp,
                },
                Err(PrepareError::Read(error) | PrepareError::Create(error)) => {
                    Action::DriverEditFailed {
                        alias,
                        run_id,
                        error,
                    }
                }
            };
            let _ = tx.send(action);
        });
    }

    /// `r` in the edit chooser: read the live body NOW and answer
    /// [`Action::DriverEditReady`] with it as the new base, over the kept temp. The
    /// user's text is untouched; saving it will replace what changed meanwhile.
    fn request_reopen(&mut self, alias: &str, run_id: &str) {
        let Some((run_dir, tx)) = self.steering_target(alias, run_id) else {
            return;
        };
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            let temp = run_dir.join(crate::driver::steer::EDIT_TEMP_NAME);
            let kept = std::fs::symlink_metadata(&temp).is_ok_and(|m| m.file_type().is_file());
            let action = if !kept {
                Action::DriverEditFailed {
                    alias,
                    run_id,
                    error: "the kept text is no longer there".to_string(),
                }
            } else {
                match crate::driver::steer::live_body(&run_dir) {
                    Ok(base) => Action::DriverEditReady {
                        alias,
                        run_id,
                        temp,
                        base,
                    },
                    Err(error) => Action::DriverEditFailed {
                        alias,
                        run_id,
                        error,
                    },
                }
            };
            let _ = tx.send(action);
        });
    }

    /// `d` in the edit chooser: remove the kept temp. The path comes from the run
    /// directory, never from the action, so only `<run>/.DIRECTION.md.edit` can go.
    fn request_discard(&mut self, alias: &str, run_id: &str) {
        let Some((run_dir, tx)) = self.steering_target(alias, run_id) else {
            return;
        };
        self.edit_sessions
            .retain(|_, session| !(session.alias == alias && session.run_id == run_id));
        self.new_run_flow
            .remove(&(alias.to_string(), run_id.to_string()));
        let (alias, run_id) = (alias.to_string(), run_id.to_string());
        tokio::task::spawn_blocking(move || {
            let error = crate::driver::steer::discard_edit(&run_dir).err();
            let _ = tx.send(Action::DriverEditDiscarded {
                alias,
                run_id,
                error,
            });
        });
    }

    /// Show a finished edit apply (30-04, D-17): a status line for the outcomes that
    /// need no screen, the chooser for a conflict or an insane body, and the lint
    /// confirmation for `driver check` problems. Only an applied or unchanged edit ends
    /// the session and re-reads the run; the others keep the temp.
    fn finish_edit(
        &mut self,
        alias: &str,
        run_id: &str,
        temp: PathBuf,
        outcome: crate::driver::steer::EditOutcome,
    ) {
        use crate::driver::steer::EditOutcome;
        use crate::ui::screens::driver_steer::{
            edit_done_message, ChooserReason, EditChooserScreen, EditLintConfirmScreen,
        };
        // Read before the session is dropped: the new-run flow checks the body once the
        // edit is applied or found unchanged.
        let then_launch = self
            .edit_sessions
            .get(&temp)
            .is_some_and(|session| session.then_launch);
        match &outcome {
            EditOutcome::Conflict | EditOutcome::Insane => {
                self.edit_sessions.remove(&temp);
                let reason = if matches!(outcome, EditOutcome::Conflict) {
                    ChooserReason::Conflict
                } else {
                    ChooserReason::Insane
                };
                self.screen_stack.push(Box::new(EditChooserScreen::new(
                    alias.to_string(),
                    run_id.to_string(),
                    temp,
                    reason,
                )));
            }
            EditOutcome::LintProblems(findings) => {
                self.screen_stack.push(Box::new(EditLintConfirmScreen::new(
                    alias.to_string(),
                    run_id.to_string(),
                    temp,
                    findings.clone(),
                )));
            }
            other => {
                self.edit_sessions.remove(&temp);
                if let Some((text, is_error)) = edit_done_message(other, &temp) {
                    if is_error {
                        self.ctx.error_message = Some(text);
                    } else {
                        self.ctx.status_message = Some((text, std::time::Instant::now()));
                    }
                }
                if !matches!(other, EditOutcome::Failed(_)) {
                    if let Some(path) = self.ctx.config.projects.get(alias).map(|p| p.path.clone())
                    {
                        self.ctx.schedule_selected_run_refresh(alias, &path, run_id);
                    }
                    if then_launch {
                        self.request_new_run_check(alias, run_id);
                    }
                }
            }
        }
        self.needs_redraw = true;
    }

    /// Show a finished steering write and schedule the re-read that makes the header
    /// show it (30-04, D-05, D-28).
    ///
    /// The wording comes from `driver_steer::steered_message`: it says the file was
    /// written, never that the driver applied it. The selected run is re-read at once
    /// so the control line reads `{word} (pending)` without waiting for the watcher.
    fn finish_steer(
        &mut self,
        alias: &str,
        run_id: &str,
        op: &crate::driver::steer::SteerOp,
        outcome: &crate::driver::steer::SteerOutcome,
    ) {
        let live = self
            .ctx
            .view_cache
            .get(alias)
            .and_then(|cache| {
                cache
                    .launcher_runs
                    .iter()
                    .find(|(listed, _)| listed == run_id)
            })
            .is_some_and(|(_, entry)| {
                entry
                    .as_ref()
                    .is_ok_and(|view| crate::ui::screens::driver_run::is_live(&view.derived))
            });
        // The rule the LOG recorded for an answered item, valid only while the cached
        // tail is for this very run (D-16).
        let park_rule = match op {
            crate::driver::steer::SteerOp::Answer { item, .. } => self
                .ctx
                .view_cache
                .get(alias)
                .filter(|cache| cache.launcher_log.run_id == run_id)
                .and_then(|cache| cache.launcher_log.park_rules.get(item))
                .cloned(),
            _ => None,
        };
        let (text, is_error) = crate::ui::screens::driver_steer::steered_message(
            op,
            outcome,
            live,
            park_rule.as_deref(),
        );
        if is_error {
            self.ctx.error_message = Some(text);
        } else {
            self.ctx.status_message = Some((text, std::time::Instant::now()));
        }
        if let Some(path) = self.ctx.config.projects.get(alias).map(|p| p.path.clone()) {
            self.ctx.schedule_selected_run_refresh(alias, &path, run_id);
        }
        self.needs_redraw = true;
    }

    /// Schedule the durable append of one injected message (STEER-01, STEER-03).
    ///
    /// **The key handler never touches the filesystem, and that is the whole
    /// shape of this function.** `journal::inbox::append` pays a `sync_data()`
    /// — deliberately, because STEER-03's criterion is that the bytes were on
    /// disk before any reader existed — and a `sync_data()` on the render thread
    /// is the WR-10 failure mode whose symptom is a frozen frame rather than an
    /// error (D-06, D-28). So the write runs on `spawn_blocking` with its result
    /// returned as an `Action` on a cloned sender, following the idiom this file
    /// already uses for the re-parse, for session detection and for the journal
    /// tail. **No file I/O on the render thread.**
    ///
    /// `id` is the caller's, minted at queue time before any other process saw
    /// the line, which is what makes the `queued` state addressable at all
    /// (D-05). It is **not** re-minted here: `InboxMessage::new` would generate
    /// a fresh one, and the message the UI is tracking would then have an id no
    /// journal record ever references.
    ///
    /// The status the UI shows is set by the `DriverInjectWritten` handler and
    /// never here — nothing may read `queued` before the write returns `Ok`.
    pub fn schedule_inbox_append(&mut self, alias: &str, run_id: &str, id: String, text: String) {
        let Some(planning_dir) = self.planning_dir_for(alias) else {
            return;
        };

        // The same fallible join `schedule_journal_tail` uses, and for the same
        // reason: the run id reaches the TUI from the driven project's own
        // `active` file, so it is attacker-controlled for threat-modelling
        // purposes (D-27, WR-02). Without this a traversing id would have the
        // TUI appending an agent-authored line to an arbitrary file on the
        // user's disk. Logged by kind only — never the path, which IS the
        // untrusted value (S3).
        let Some(paths) = crate::journal::run_paths(&planning_dir, run_id) else {
            tracing::warn!(
                alias = %alias,
                kind = "run_id_not_a_plain_component",
                "inbox append refused: the run id does not name a single directory component",
            );
            self.ctx.error_message = Some(
                "Could not queue message: that run id does not name a single directory \
                 component. Nothing was written — try again."
                    .to_string(),
            );
            self.needs_redraw = true;
            return;
        };

        let Some(tx) = &self.ctx.event_tx else {
            self.ctx.error_message = Some(
                "Could not queue message: this session has no event channel. Nothing was \
                 written — try again."
                    .to_string(),
            );
            self.needs_redraw = true;
            return;
        };
        let tx = tx.clone();
        let inbox = paths.inbox;
        let alias_for_task = alias.to_string();
        let run_id_for_task = run_id.to_string();

        tokio::task::spawn_blocking(move || {
            // Built by hand rather than through `InboxMessage::new` so the
            // caller's id survives; `cap_chars` is applied explicitly because
            // that is the half of `new` this path still needs.
            let message = crate::journal::inbox::InboxMessage {
                id: id.clone(),
                ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                text: crate::journal::inbox::cap_chars(
                    &text,
                    crate::journal::inbox::MAX_INBOX_TEXT_CHARS,
                ),
            };

            // The error **kind** and nothing else. An `io::Error`'s `Display`
            // can carry an OS path, and this path's operand is a run directory
            // under a project the agent writes to (S3, D-28).
            let error = crate::journal::inbox::append(&inbox, &message)
                .err()
                .map(|e| format!("{:?}", e.kind()));
            if let Some(kind) = &error {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    kind = %kind,
                    "inbox append failed",
                );
            }

            let _ = tx.send(Action::DriverInjectWritten {
                alias: alias_for_task,
                run_id: run_id_for_task,
                id,
                error,
            });
        });
    }

    // The two one-line `App::schedule_run_list_scan` / `App::schedule_dry_run_report`
    // wrappers that stood here are gone (WR-09). Both delegated to the identically
    // named `AppContext` methods and neither had a caller anywhere in `src/` or
    // `tests/`: every real call goes to `ctx.schedule_*` directly, because a
    // `Screen` is handed an `&mut AppContext` and never an `&mut App`. They were
    // `pub`, so no `dead_code` warning fired and the duplication was invisible to
    // the compiler. `App`'s own callers — the tick rescan, the injection handler
    // and `start_driver_run` — go through `self.ctx` for the same reason.

    /// Load project states for all registered projects.
    /// Uses spawn_blocking for async-safe file I/O when event_tx is available,
    /// falls back to synchronous loading for initial startup.
    pub fn load_project_states(&mut self) {
        for (alias, project) in &self.ctx.config.projects {
            let planning_dir = project.path.join(".planning");
            let state = state_reader::parse_project_state(&planning_dir);
            self.ctx.project_states.insert(alias.clone(), state);
        }
        self.ctx.recompute_filtered_aliases();
    }

    /// Record initial snapshots for all loaded project states (for change detection).
    pub fn init_change_tracker(&mut self) {
        for (alias, state) in &self.ctx.project_states {
            self.ctx.change_tracker.record_initial(alias, state);
        }
    }

    /// Set the one startup status line naming the effective global gsd-core
    /// install against the synced version (quick 260926-j0a). Called once,
    /// synchronously, right after the initial [`Self::load_project_states`].
    pub fn announce_gsd_install(&mut self) {
        self.announce_gsd_install_with(&state_reader::gsd_install::InstallRoots::from_env());
    }

    /// [`Self::announce_gsd_install`] with the install roots injected.
    ///
    /// Sets exactly one status message. The winning `gsd-core` path goes to
    /// the log file only, never the TUI (T-j0a-05). A rarer "Auto-registered"
    /// message set later in startup may supersede it (inferred I-9); the
    /// Config tab and dashboard border keep carrying the information.
    pub fn announce_gsd_install_with(&mut self, roots: &state_reader::gsd_install::InstallRoots) {
        use state_reader::gsd_install::{detect_gsd_install, startup_summary, GsdInstallStatus};
        let global = detect_gsd_install(None, roots);
        let text = startup_summary(
            &global,
            self.ctx.project_states.values().map(|s| &s.gsd_install),
        );
        match &global {
            GsdInstallStatus::Found(install) => tracing::info!(
                gsd_core_dir = %install.gsd_core_dir.display(),
                summary = %text,
                "installed gsd-core detected",
            ),
            GsdInstallStatus::NotFound => {
                tracing::info!(summary = %text, "no global gsd-core install found")
            }
        }
        self.ctx.status_message = Some((text, std::time::Instant::now()));
        self.needs_redraw = true;
    }

    /// Auto-register any active agent sessions (Claude or Codex) whose working_dir is an
    /// unregistered GSD project. Persists config, starts the file watcher,
    /// loads initial project state, and surfaces a status message per
    /// newly-added project.
    pub fn auto_register_new_sessions(&mut self) {
        let added =
            registry::auto_register_from_sessions(&mut self.ctx.config, &self.active_sessions);

        if added.is_empty() {
            return;
        }

        for (alias, path) in &added {
            if let Err(e) = save_config(&self.ctx.config, &self.ctx.config_path) {
                tracing::warn!(
                    alias = %alias,
                    error = %e,
                    "auto-register: save_config failed",
                );
                // Continue; the in-memory registration still helps this session.
            }

            let planning_dir = path.join(".planning");
            if let Some(ref mut watcher) = self.ctx.watcher {
                if let Err(e) = watcher.watch(&planning_dir) {
                    tracing::warn!(
                        alias = %alias,
                        path = %planning_dir.display(),
                        error = %e,
                        "auto-register: watcher.watch failed",
                    );
                }
            }

            let state = state_reader::parse_project_state(&planning_dir);
            self.ctx.change_tracker.record_initial(alias, &state);
            self.ctx.project_states.insert(alias.clone(), state);

            tracing::info!(
                alias = %alias,
                path = %path.display(),
                "Auto-registered GSD project from active agent session",
            );
            self.ctx.status_message = Some((
                format!("Auto-registered: {}", alias),
                std::time::Instant::now(),
            ));
        }

        self.ctx.recompute_filtered_aliases();
        self.needs_redraw = true;
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {
                // Expire status messages after 3 seconds
                if let Some((_, instant)) = &self.ctx.status_message {
                    if instant.elapsed() > std::time::Duration::from_secs(3) {
                        self.ctx.status_message = None;
                        self.needs_redraw = true;
                    }
                }

                // The elapsed-time counter (D-21). It rides THIS tick and gets
                // no interval of its own, for the same reason the reconciliation
                // probe below does not — see the comment on that block, which
                // forbids a second timer in as many words.
                //
                // **Gated, and the gate is not an optimisation.** Elapsed time
                // is `now − started_at`, so it differs every tick forever; an
                // unconditional redraw here would repaint an idle fleet
                // dashboard four times a second for the life of the process.
                // The predicate is a pure function so its truth table is
                // assertable without a terminal.
                if self.driver_elapsed_redraw_due() {
                    self.needs_redraw = true;
                }
                // The selected launcher run's heartbeat age (30-02, D-08): the same
                // gate shape, the same tick, no timer of its own.
                if self.launcher_heartbeat_redraw_due() {
                    self.needs_redraw = true;
                }

                // Poll for Claude and Codex sessions every 20 ticks (~5s at 250ms interval)
                self.session_poll_counter += 1;
                if self.session_poll_counter >= 20 {
                    self.session_poll_counter = 0;
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let sessions = crate::session_detector::detect_sessions();
                            let _ = tx.send(Action::SessionsDetected { sessions });
                        });
                    }

                    // The driver reconciliation probe rides THIS counter and
                    // must never get one of its own (D-13, ARCHITECTURE §4.4(c)
                    // are both explicit). Two timers polling `/proc` and
                    // `run.json` at slightly different phases would double the
                    // syscall load for no extra freshness and would make "how
                    // stale can the dashboard be?" a question with two answers.
                    // Do not tidy this into its own interval.
                    //
                    // `spawn_blocking` for the same reason as the line above:
                    // `/proc` reads and `run.json` reads are synchronous fs work
                    // and this file's idiom for that is unambiguous.
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        let projects = self.ctx.config.projects.clone();
                        // The launcher-run markers (30-03, D-13) ride this same task:
                        // one more `read_dir` plus bounded reads per project, on the
                        // same counter, never a timer of their own. With the
                        // experimental flag off no launcher surface is shown, so the
                        // reads are skipped altogether.
                        let experimental = self.ctx.experimental;
                        tokio::task::spawn_blocking(move || {
                            let runs = crate::driver::reconcile::reconcile_all(&projects);
                            // Read in the SAME blocking task as the probe above,
                            // not in one of its own (WR-02). Both answers
                            // describe the same instant and both are `read_dir`
                            // plus small reads, so a second task would double
                            // the scheduling for no extra freshness — the same
                            // reasoning that forbids a second timer.
                            let last_outcomes =
                                crate::driver::reconcile::last_ended_outcomes(&projects);
                            let launcher_markers = if experimental {
                                launcher_markers_for(&projects, std::time::SystemTime::now())
                            } else {
                                HashMap::new()
                            };
                            let _ = tx.send(Action::RunsReconciled {
                                runs,
                                last_outcomes,
                                launcher_markers,
                            });
                        });
                    }

                    // The Driver tab's run list, inbox and journal ride the SAME
                    // counter, for the same reason everything above does — and
                    // the comment at the head of this block forbidding a second
                    // timer governs this too (CR-04). Without it the tab is
                    // frozen at the moment it was entered: a run started from it
                    // stays invisible, a queued injection never reaches the
                    // four-state widget, and a run that ends while you watch
                    // swaps its live output for a stale snapshot.
                    //
                    // Gated on the tab being on screen, because unlike the probe
                    // above this is three blocking reads for ONE project rather
                    // than a `/proc` glance across the fleet.
                    self.rescan_driver_tab_if_open();

                    // The running-agents scan rides THIS counter too and must
                    // never get a timer of its own, for the reason the comment
                    // on the reconciliation probe above gives (D-C01). Nor does
                    // it add a `notify` watcher on `~/.claude` or on any agent
                    // worktree: this poll already bounds staleness at about
                    // five seconds, and a watcher per worktree would turn a
                    // thirteen-agent wave into thirteen more inotify trees for
                    // no extra freshness (D-B04).
                    //
                    // `spawn_blocking`, because the scan runs `git worktree
                    // list`, `git status` and small reads per project. The
                    // in-flight flag is set BEFORE the spawn and cleared by the
                    // `AgentsScanned` handler, so a slow scan can never stack
                    // up behind itself; the closure ALWAYS sends, an empty
                    // result after a panic, so the flag cannot stick
                    // (RESEARCH Pitfall 6).
                    //
                    // The process probe rides this same task, once per poll
                    // (quick 260926-06g): it is built INSIDE the catch_unwind,
                    // so a panic while building it still yields the empty
                    // result and the send. Off Linux it answers "unknown" to
                    // everything and the scan is mtime-only.
                    if !self.ctx.agents_scan_in_flight {
                        if let Some(ref tx) = self.ctx.event_tx {
                            let tx: UnboundedSender<Action> = tx.clone();
                            let projects: Vec<(String, PathBuf)> = self
                                .ctx
                                .config
                                .projects
                                .iter()
                                .map(|(alias, project)| (alias.clone(), project.path.clone()))
                                .collect();
                            self.ctx.agents_scan_in_flight = true;
                            tokio::task::spawn_blocking(move || {
                                let now = std::time::SystemTime::now();
                                let per_project =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        let probe = crate::session_detector::process_probe();
                                        crate::agents::scan_projects_guarded(
                                            &projects,
                                            probe.as_ref(),
                                            now,
                                        )
                                    }))
                                    .unwrap_or_default();
                                let _ = tx.send(Action::AgentsScanned { per_project });
                            });
                        }
                    }

                    // The prune rides the SAME counter, for the same reason the
                    // reconciliation probe above does (D-27 says so explicitly).
                    // It is pure in-memory map work — no syscall, no file read —
                    // so it runs inline rather than on a blocking task.
                    self.prune_driver_maps();
                }
            }
            Action::RawKey(key_event) => {
                self.handle_key(key_event.code, key_event.modifiers);
            }
            Action::Mouse(mouse_event) => {
                self.handle_mouse(mouse_event, std::time::Instant::now());
            }
            Action::Resize => {
                self.needs_redraw = true;
            }
            Action::FileChanged {
                project_path,
                changed_path,
            } => {
                // Find the alias matching this project path
                let alias = self
                    .ctx
                    .config
                    .projects
                    .iter()
                    .find(|(_, proj)| proj.path == project_path)
                    .map(|(alias, _)| alias.clone());

                if let Some(alias) = alias {
                    // Classify BEFORE the dedup check. D-14 is explicit that
                    // the two routes must not share `last_refresh`: if the
                    // driver route went through the 500ms map, one journal
                    // append could suppress a genuine STATE.md re-parse for
                    // half a second — a performance bug traded for a
                    // correctness bug.
                    match crate::journal::classify_change(&project_path, &changed_path) {
                        crate::journal::ChangeKind::DriverJournal { run_id } => {
                            // No extra throttle here, deliberately (D-15). A
                            // tail read costs bytes-appended-since-the-last-
                            // read, so its cost tracks actual new data rather
                            // than project size — which is the whole
                            // difference from `parse_project_state`, and the
                            // 200ms watcher debounce already floors the rate.
                            self.schedule_journal_tail(&alias, &project_path, &run_id);
                        }
                        // A launcher run's own files (30-01, D-24). Refreshes the
                        // launcher list for the alias whose Driver tab is open and
                        // does NOTHING ELSE: no re-parse (OBS-06), no `last_refresh`
                        // entry, so a heartbeat-rate stream of these cannot starve
                        // a genuine STATE.md re-parse of its 500 ms window.
                        crate::journal::ChangeKind::DriverRun { run_id } => {
                            let on_top = self.screen_stack.last().is_some_and(|top| {
                                driver_tab_is_on_top(
                                    top.name(),
                                    self.ctx.detail_sub_view_per_project.get(&alias),
                                )
                            });
                            if on_top && self.ctx.selected_alias().as_deref() == Some(&alias) {
                                // The selected run: one DIRECTION.md read and one lock
                                // probe replace just that run (D-24, D-25). Any other
                                // run changes only its list row, so the whole listing
                                // is refreshed instead.
                                let selected = self
                                    .ctx
                                    .view_cache
                                    .get(&alias)
                                    .and_then(|cache| cache.launcher_selected_id.as_deref());
                                if selected == Some(run_id.as_str()) {
                                    self.ctx.schedule_selected_run_refresh(
                                        &alias,
                                        &project_path,
                                        &run_id,
                                    );
                                } else {
                                    self.ctx.schedule_launcher_refresh(&alias, &project_path);
                                }
                            } else {
                                // The Driver tab is not open for this project: its
                                // dashboard marker is refreshed by a one-project
                                // listing instead (30-03, D-13). When the tab IS on
                                // top the refresh handlers recompute the marker from
                                // the rows they just stored.
                                self.ctx.schedule_launcher_marker(&alias, &project_path);
                            }
                        }
                        // The splice temps, `state.json`, the locks, editor swap
                        // files: nothing reads them, so nothing refreshes (T-30-03).
                        crate::journal::ChangeKind::Ignored => {}
                        crate::journal::ChangeKind::Planning => {
                            // The Archive tab reloads in place, and AHEAD of the
                            // dedup below rather than behind it: the dedup drops
                            // the last write of a burst that lands inside 500 ms
                            // of the previous refresh, and for a milestone being
                            // archived that last write is the one that matters.
                            // The reload is a few `read_dir`s bounded by the
                            // archives this alias has open — nothing at all for
                            // an alias whose Archive tab was never visited — so
                            // it does not need the throttle `parse_project_state`
                            // does.
                            self.ctx.schedule_archive_refresh(&alias, &project_path);

                            // Dedup: skip if last refresh was less than 500ms ago
                            let now = std::time::Instant::now();
                            if let Some(last) = self.ctx.last_refresh.get(&alias) {
                                if now.duration_since(*last) < std::time::Duration::from_millis(500)
                                {
                                    return;
                                }
                            }

                            self.schedule_reparse(&alias, &project_path);

                            // Auto-start watcher if not yet watching
                            let planning_dir_check = project_path.join(".planning");
                            if planning_dir_check.is_dir() {
                                if let Some(ref mut watcher) = self.ctx.watcher {
                                    let _ = watcher.watch(&planning_dir_check);
                                }
                            }
                        }
                    }
                }
            }
            Action::ProjectStateLoaded { alias, state } => {
                // Detect changes before replacing the old state.
                // `changed` is true if there was no prior state (first load) or
                // the freshly-parsed snapshot differs from the cached one.
                // Watcher events that don't actually move state (e.g. unrelated
                // .planning/ writes) skip the user-facing notification.
                let changed = match self.ctx.project_states.get(&alias) {
                    Some(old_state) => {
                        self.ctx
                            .change_tracker
                            .detect_changes(&alias, old_state, &state);
                        *old_state != *state
                    }
                    None => true,
                };
                self.ctx.project_states.insert(alias.clone(), *state);
                if changed {
                    self.ctx.recompute_filtered_aliases();
                    self.ctx.status_message =
                        Some((format!("Updated: {}", alias), std::time::Instant::now()));
                    self.needs_redraw = true;
                }
            }
            Action::GitLogLoaded {
                alias,
                entries,
                planning_only,
            } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                cache.git_entries = entries;
                cache.git_planning_only = planning_only;
                cache.loading_git = false;
                self.needs_redraw = true;
            }
            Action::GitCommitDetailLoaded {
                alias,
                hash,
                detail,
            } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                // The hash guard lives on the cache, beside the three fields it
                // touches, so the pane's state has exactly one writer (QD-07,
                // QD-09). A detail for an unselected commit is dropped; the
                // loading flag clears either way.
                cache.apply_loaded_commit_detail(&hash, detail);
                self.needs_redraw = true;
            }
            Action::SessionsDetected { sessions } => {
                self.active_sessions = sessions.clone();
                self.ctx.active_sessions = sessions;
                self.auto_register_new_sessions();
                self.needs_redraw = true;
            }
            Action::CreateProjectResult {
                alias,
                path,
                success,
                error,
            } => {
                if success {
                    // Register the new project — through the same alias
                    // judgment as every other registration route (D-17-2).
                    let judged = match registry::Alias::new(&alias) {
                        Ok(judged) => judged,
                        Err(refusal) => {
                            self.ctx.error_message =
                                Some(format!("Failed to register: {}", refusal));
                            self.needs_redraw = true;
                            return;
                        }
                    };
                    if let Err(e) =
                        registry::add_project_unchecked(&mut self.ctx.config, &judged, &path)
                    {
                        self.ctx.error_message = Some(format!("Failed to register: {}", e));
                        self.needs_redraw = true;
                        return;
                    }
                    if let Err(e) = save_config(&self.ctx.config, &self.ctx.config_path) {
                        self.ctx.error_message = Some(format!("Failed to save config: {}", e));
                        self.needs_redraw = true;
                        return;
                    }

                    // Start file watcher on the new project
                    let planning_dir = path.join(".planning");
                    if planning_dir.is_dir() {
                        if let Some(ref mut watcher) = self.ctx.watcher {
                            let _ = watcher.watch(&planning_dir);
                        }
                    } else {
                        if let Some(ref tx) = self.ctx.event_tx {
                            let tx = tx.clone();
                            let planning_poll = planning_dir.clone();
                            tokio::spawn(async move {
                                for _ in 0..30 {
                                    if planning_poll.is_dir() {
                                        let _ = tx.send(Action::FileChanged {
                                            project_path: planning_poll
                                                .parent()
                                                .unwrap()
                                                .to_path_buf(),
                                            // The `.planning` directory itself
                                            // is what was just observed. It
                                            // classifies as `Planning`, which
                                            // is the re-parse this poll exists
                                            // to trigger.
                                            changed_path: planning_poll.clone(),
                                        });
                                        break;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                }
                            });
                        }
                    }

                    // Load project state for the new alias
                    let state = state_reader::parse_project_state(&planning_dir);
                    self.ctx.project_states.insert(alias.clone(), state);

                    self.ctx.status_message = Some((
                        format!("Created project \"{}\"", alias),
                        std::time::Instant::now(),
                    ));
                    // Pop back to normal screen if we're on the create screen
                    // The CreateProjectScreen already popped itself via ScreenAction::Pop
                    self.ctx.input_buffer.clear();
                    self.ctx.error_message = None;
                    self.ctx.recompute_filtered_aliases();
                    if let Some(pos) = self.ctx.filtered_aliases.iter().position(|a| a == &alias) {
                        self.ctx.table_state.select(Some(pos));
                    }
                    self.needs_redraw = true;
                } else {
                    self.ctx.error_message =
                        Some(error.unwrap_or_else(|| "Unknown error creating project".to_string()));
                    self.ctx.input_buffer.clear();
                    self.needs_redraw = true;
                }
            }
            Action::ArchiveMilestonesDiscovered { alias, milestones } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                // The producer for `archive_milestones` (D-21-20): these are
                // directory-derived names scanned out of `.planning/archive/`,
                // and this is the one place they reach the cache.
                cache.archive_milestones = milestones
                    .into_iter()
                    .map(crate::text::Untrusted::from_untrusted_source)
                    .collect();
                // An in-place refresh (`schedule_archive_refresh`) can shrink
                // the list under the cursor; keep the cursor on a real row.
                let last = cache.archive_milestones.len().saturating_sub(1);
                cache.archive_selected[0] = cache.archive_selected[0].min(last);
                cache.archive_loading = false;
                self.needs_redraw = true;
            }
            Action::ArchiveLoaded {
                alias,
                milestone,
                data,
            } => {
                let cache = self.ctx.view_cache.entry(alias.clone()).or_default();
                // Same clamp as the milestone list above, for the listing this
                // archive feeds if it is the one on screen.
                match &cache.archive_depth {
                    crate::archive::ArchiveDepth::PhaseList { milestone: open }
                        if *open == milestone =>
                    {
                        let rows = data.top_level_files.len() + data.phases.len();
                        cache.archive_selected[1] =
                            cache.archive_selected[1].min(rows.saturating_sub(1));
                    }
                    crate::archive::ArchiveDepth::FileList {
                        milestone: open,
                        phase_idx,
                    } if *open == milestone => {
                        let rows = data.phases.get(*phase_idx).map_or(0, |p| p.files.len());
                        cache.archive_selected[2] =
                            cache.archive_selected[2].min(rows.saturating_sub(1));
                    }
                    _ => {}
                }
                cache.archive_loading = false;
                self.ctx.archive_cache.insert(alias, milestone, data);
                self.needs_redraw = true;
            }
            // One tail read landed. **This is the seam Phase 16 left open, and
            // this handler is what closes it** (D-20).
            //
            // Until now it stored its cursor, counted `seq` gaps into a
            // `tracing::warn!`, and dropped `records` on the floor — deliberately,
            // because there was no surface to render them onto and a redraw would
            // have scheduled a frame that could not differ from the one on
            // screen. The comment here said so, and named Phase 18 as what adds
            // the surface and the flag together. It does, so that comment is
            // gone rather than left standing above code it no longer describes.
            //
            // What it does now:
            //
            // * projects each record into a `DriverLineKind` and pushes it into
            //   the alias's bounded ring buffer, where `push_record` sanitises
            //   it and enforces the cap;
            // * turns an observed sequence gap into a **visible** diagnostic
            //   line as well as a log line, because a pane that silently loses
            //   records is the named "looks done but isn't" failure; and
            // * sets `needs_redraw`, because the frame genuinely can differ now.
            //
            // What it still deliberately does **not** do, and the omissions are
            // as load-bearing as they were before:
            //
            // * It does not touch `ProjectState` (D-18). That type derives
            //   `PartialEq` and `app.rs` uses the derived equality to suppress
            //   "Updated: {alias}" status spam; live output arrives every few
            //   seconds, so a field there would flood the status bar for an
            //   entire multi-hour run.
            // * It does not touch `last_refresh` (D-14). Sharing the 500 ms
            //   dedup map would let a journal append suppress a genuine
            //   `STATE.md` re-parse, trading a performance bug for a correctness
            //   one.
            // * It does not schedule a re-parse (OBS-06).
            Action::DriverJournalAppended {
                alias,
                run_id,
                records,
                cursor,
            } => {
                // `seq` is monotonic from 1 and exists so a tailing reader can
                // detect gaps (D-03). A gap is reported as a COUNT and never as
                // a parse failure, and never with a record body (D-28, D-30).
                //
                // This calls the shared reader function rather than
                // re-implementing its `windows(2)` filter, which is what it used
                // to do. Two copies of a comparison are two things to keep in
                // agreement, and they had already diverged in the way that
                // matters: the inline copy could not be seeded, so it could only
                // ever see a gap that fell inside one batch.
                let key = (alias, run_id);
                let previous_last_seq = self
                    .ctx
                    .journal_cursors
                    .get(&key)
                    .map_or(0, |stored| stored.last_seq);
                let gaps = crate::journal::reader::seq_gaps_from(previous_last_seq, &records).len();
                if gaps > 0 {
                    tracing::warn!(
                        alias = %key.0,
                        run_id = %key.1,
                        count = gaps,
                        "journal tail: sequence gaps observed",
                    );
                }

                // The buffer for this alias, created on first append. The map is
                // pruned in `prune_driver_maps` — see the negative
                // carry-forward the phase discharges there.
                //
                // **Retargeted before the first push** (CR-02). The map is keyed
                // by alias, so without this the ring outlives the run that
                // filled it and run B's pane paints run A's output — including
                // A's terminal record, which renders last and in B's colour, so
                // a live run shows a previous run's success as its own visual
                // full stop. This is the same guard `update_driver_tally`
                // applies to the tally, at the same point in the same handler.
                let output = self.ctx.driver_output.entry(key.0.clone()).or_default();
                output.retarget(&key.1);

                // The gap goes in FIRST and as a line the user can see. The
                // records that follow are the ones that arrived; the gap is what
                // came before them and never will, so it belongs above them.
                if gaps > 0 {
                    output.push_record(
                        crate::ui::screens::DriverLineKind::Diagnostic,
                        &journal_gap_line(gaps),
                    );
                }

                for record in &records {
                    if let Some((line_kind, text)) = driver_line_for_record(record) {
                        output.push_record(line_kind, &text);
                    }
                }

                // The two header facts that live in journal records rather than
                // in the committed `run.json`: the cumulative cost and the turn
                // count. `driver_line_for_record` deliberately returns `None`
                // for `cost` because it is header data and not a pane line —
                // this is the header reading it (D-12).
                let cache = self.ctx.view_cache.entry(key.0.clone()).or_default();
                update_driver_tally(cache, &key.1, &records);

                // An injected message's state is a set intersection over disk
                // records, and the newest of those records arrive here. They are
                // kept **as records** rather than as rendered lines because the
                // correlation id lives in a typed field, and reconstructing a
                // protocol state by parsing a rendered string is the
                // screen-scraping D-01 forbids in another guise (D-08).
                //
                // Guarded by run id for the reason the tally is: appending the
                // tailed run's transitions to a journal the user is reviewing
                // from last week would report one run's steering as another's.
                if let Some(journal) = cache
                    .driver_journal
                    .as_mut()
                    .filter(|journal| journal.run_id == key.1)
                {
                    // **Deduped on `seq`** (WR-08). The scan reads the journal
                    // whole from offset zero and replaces `driver_journal`,
                    // while the tail keeps its own independent cursor in
                    // `journal_cursors`. When a scan lands after a tail has
                    // already consumed records, the scan's snapshot contains
                    // them and the next tail batch re-appends everything the
                    // scan read past — with nothing to stop it, the vector grew
                    // with duplicates for the life of the selection. The state
                    // derivation is idempotent under duplication, so this was
                    // growth and clarity rather than a wrong render; but the
                    // field's own doc claims it is "naturally bounded: one
                    // record per state transition of one message a human typed",
                    // and that has to stay true.
                    //
                    // `seq` is the key because the journal already guarantees it
                    // is monotonic within a run, which an id does not: one id
                    // legitimately carries several records.
                    let known: std::collections::HashSet<u64> =
                        journal.injections.iter().map(|record| record.seq).collect();
                    journal.injections.extend(
                        records
                            .iter()
                            .filter(|record| {
                                crate::ui::screens::driver::INJECTION_KINDS
                                    .contains(&record.kind.as_str())
                                    && !known.contains(&record.seq)
                            })
                            .cloned(),
                    );
                }

                self.ctx.journal_cursors.insert(key, cursor);
                // The pane can now show a frame that differs from the one on
                // screen, which is precisely the condition that was absent
                // before (D-20).
                self.needs_redraw = true;
            }
            // The scan is authoritative, so the map is **replaced** and never
            // merged (D-12, D-13). A merge would keep a run in the map after its
            // project was unregistered, or after it ended — the absence of an
            // entry is how the scan says both of those things, and a merge
            // discards exactly that information.
            Action::RunsReconciled {
                runs,
                last_outcomes,
                launcher_markers,
            } => {
                // The launcher-run markers (30-03, D-13), replaced wholesale: a project
                // absent from the scan has no marker, and one never scanned has none
                // either (no guessed class). The filter and the attention sort read the
                // marker, so they are recomputed when any marker moved.
                if self.ctx.apply_launcher_markers(&launcher_markers) {
                    self.ctx.recompute_filtered_aliases();
                    self.needs_redraw = true;
                }
                let observed: HashMap<String, crate::driver::reconcile::ObservedRun> = runs
                    .into_iter()
                    .map(|run| (run.alias.clone(), run))
                    .collect();

                // Replaced wholesale for the reason `observed_runs` is: the scan
                // is authoritative, and a project whose newest run is no longer
                // an ended one says so by its absence.
                if self.ctx.last_outcomes != last_outcomes {
                    self.ctx.last_outcomes = last_outcomes;
                    self.needs_redraw = true;
                }

                // Equality-guarded redraw, following this file's existing
                // discipline: a scan lands every ~5s for the whole life of the
                // process, and an unconditional redraw here would repaint the
                // frame twelve times a minute forever with nothing changed.
                if self.ctx.observed_runs != observed {
                    // Counts only. A goal string is user content and a run id is
                    // not worth a line every five seconds (D-28).
                    tracing::debug!(
                        observed = observed.len(),
                        live = observed.values().filter(|run| run.is_live()).count(),
                        "driver reconciliation scan applied",
                    );
                    self.ctx.observed_runs = observed;
                    self.needs_redraw = true;
                }
            }
            // The agents scan is authoritative, so `agent_views` is **replaced**
            // and never merged — the `RunsReconciled` rule above (D-C01).
            //
            // The wave model is derived HERE, once per scan, against the
            // already-parsed `ProjectState`, never at render time. `AgentView`
            // carries the scan instant, so a project with agents compares
            // unequal on every scan and redraws once per scan: that is what
            // advances the age column. A fleet with no agents never redraws
            // from this arm, because its map stays empty.
            Action::AgentsScanned { per_project } => {
                // First and unconditionally: whatever the payload holds, the
                // scan that was in flight has ended (RESEARCH Pitfall 6).
                self.ctx.agents_scan_in_flight = false;

                let default_state = crate::state_reader::ProjectState::default();
                let mut views: HashMap<String, crate::agents::waves::AgentView> = HashMap::new();
                for (alias, agents) in per_project {
                    // Registered aliases only: a project unregistered while the
                    // scan ran must not come back through its result.
                    if !self.ctx.config.projects.contains_key(&alias) {
                        continue;
                    }
                    // No entry for a project with nothing to show, so its
                    // dashboard row stays byte-identical to today's (D-C14).
                    // Unmerged worktree work alone is something to show: its
                    // `◐N` is a pending action (quick 260929-szq, I-7).
                    // Worktree-held plans are something to show too, in the
                    // Phases pane and on the Roadmap (quick 261014-0jv); the
                    // dashboard cell is unchanged because `summary_forms`
                    // yields no form for them (ID-10).
                    if agents.rows.is_empty()
                        && agents.worktreeless.is_empty()
                        && agents.unmerged.is_empty()
                        && agents.worktree_plans.is_empty()
                    {
                        continue;
                    }
                    let state = self
                        .ctx
                        .project_states
                        .get(&alias)
                        .unwrap_or(&default_state);
                    let view = crate::agents::waves::derive(&agents, state);
                    views.insert(alias, view);
                }

                if self.ctx.agent_views != views {
                    // Counts only. Descriptions, agent types and branch names
                    // are agent-authored text and never reach a log (D-C13).
                    tracing::debug!(
                        projects = views.len(),
                        rows = views.values().map(|v| v.agents.len()).sum::<usize>(),
                        "agents scan applied",
                    );
                    self.ctx.agent_views = views;
                    self.needs_redraw = true;
                }
            }
            // The goal travels from the sender and is recorded into
            // `RunRecord.goal` verbatim, interpreting nothing (Phase 21 owns
            // goal decomposition). `None` and `Some("")` are deliberately kept
            // distinct all the way down: `drive_argv` omits the flag entirely
            // for `None`, where an empty string would record an empty goal as
            // though one had been given.
            //
            // That reasoning was written when the handler passed a hard-coded
            // `None`, because there was no screen to type a goal into — Phase
            // 18's start flow (D-23) is what changes. The handler now passes the
            // variant's own `goal` through, and the reasoning it replaces is not
            // obsolete but **load-bearing**: the goal picker's Step B accepts an
            // empty buffer, so the empty-string trap is live rather than
            // theoretical, and [`goal_or_none`] is what keeps it shut.
            Action::DriverStartRequested {
                alias,
                command,
                goal,
            } => {
                #[cfg(unix)]
                self.start_driver_run(&alias, &command, goal_or_none(goal.as_deref()));
                // Off Unix there is no detached spawn to reach, so the request
                // has nowhere to go. The bindings are consumed explicitly
                // rather than left to an `unused_variables` allow.
                #[cfg(not(unix))]
                let _ = (alias, command, goal);
            }
            Action::DriverStopRequested { alias } => {
                #[cfg(unix)]
                self.stop_driver_run(&alias);
                // Off Unix there is no way to have started a run in the first
                // place, so there is nothing to stop. The binding is consumed
                // explicitly rather than left to an `unused_variables` allow.
                #[cfg(not(unix))]
                let _ = alias;
            }
            // The stop already happened; this is the report, and **the maps are
            // mutated only when the run is actually gone** (WR-15, D-29).
            //
            // When the run IS gone the entry is dropped rather than edited,
            // because `observed_runs` is a projection of the scan and a
            // hand-edited entry would be overwritten by the next one anyway.
            // Dropping it is what the scan itself will do five seconds later,
            // done now so the dashboard does not show a stopped run as live in
            // the meantime.
            //
            // When it may still be live, nothing is dropped. `SignalFailed`
            // means the signal was **never delivered** and `AlreadyGone` means
            // **nothing was signalled** — in neither case has anything
            // established that the run ended, and dropping on either cost two
            // distinct things:
            //
            // * the dashboard showed **no run** for up to five seconds, until
            //   the next reconciliation scan put back a run that never left; and
            // * the `session_spawned_runs` entry was gone **permanently** —
            //   nothing ever re-inserts it, because the disk cannot say who a
            //   driver's parent was — so a later stop took the `Adopted` reaping
            //   arm for a run this session did spawn, waiting on a `/proc`
            //   re-probe instead of the reaping task that actually owns the
            //   `wait()`.
            //
            // **Phase 18 is what makes the contradiction visible:** the Driver
            // tab renders a "no run" pane directly beside a status line reading
            // *"the stop signal could not be delivered"*. The status message is
            // still set on both arms — the user is told what happened either
            // way; what changes is that the dashboard no longer forgets a run on
            // the strength of a stop that stopped nothing.
            Action::DriverStopped {
                alias,
                run_id,
                outcome,
                disposition,
            } => {
                if disposition == crate::action::StopDisposition::RunGone {
                    self.ctx.observed_runs.remove(&alias);
                    self.ctx.session_spawned_runs.remove(&run_id);
                }
                self.ctx.status_message =
                    Some((format!("{alias}: {outcome}"), std::time::Instant::now()));
                self.needs_redraw = true;
            }
            // 28-07, D-11: stop a launcher run through the library function the
            // `driver stop` CLI calls. Linux-only like the launcher itself.
            Action::LauncherStopRequested { alias, run_id } => {
                #[cfg(target_os = "linux")]
                self.stop_launcher_run(&alias, &run_id);
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = (alias, run_id);
                    self.ctx.error_message =
                        Some(crate::launcher::LauncherError::UnsupportedPlatform.to_string());
                    self.needs_redraw = true;
                }
            }
            // The stop already happened; `report` is its outcome. It is shown on the
            // status line, and the Driver runs are re-listed so the row's derived
            // status catches up without waiting for the next event.
            Action::LauncherStopped {
                alias,
                run_id: _,
                report,
            } => {
                // `render_for_terminal`: the report embeds the final status word the
                // launcher read back out of `state.json`, which is run-dir content.
                let line =
                    crate::text::render_for_terminal(&format!("{alias}: {report}")).to_string();
                self.ctx.status_message = Some((line, std::time::Instant::now()));
                if let Some(path) = self.ctx.config.projects.get(&alias).map(|p| p.path.clone()) {
                    self.ctx.schedule_run_list_scan(&alias, &path);
                    self.ctx.schedule_launcher_refresh(&alias, &path);
                }
                self.needs_redraw = true;
            }
            // 30-04, D-14, D-15: a steering key asked for one edit to the selected
            // launcher run's DIRECTION.md body. The key handler touched no file; the
            // write runs on `spawn_blocking` and answers `DriverSteered`.
            Action::DriverSteerRequested { alias, run_id, op } => {
                self.request_steer(&alias, &run_id, op);
            }
            Action::DriverSteered {
                alias,
                run_id,
                op,
                outcome,
            } => {
                self.finish_steer(&alias, &run_id, &op, &outcome);
            }
            // 30-04, D-17: edit DIRECTION.md's body in the user's editor. The temp is
            // made and applied off the render thread; the library holds the no-clobber
            // rule. Every arm that reaches a write has its non-Linux refusal in
            // `steering_target`.
            Action::DriverEditRequested { alias, run_id } => {
                self.request_edit(&alias, &run_id);
            }
            Action::DriverEditReady {
                alias,
                run_id,
                temp,
                base,
            } => {
                let run_dir = self
                    .ctx
                    .config
                    .projects
                    .get(&alias)
                    .and_then(|project| Self::launcher_run_dir(&project.path, &run_id));
                if let Some(run_dir) = run_dir {
                    let then_launch = self.new_run_flow.contains(&(alias.clone(), run_id.clone()));
                    self.edit_sessions.insert(
                        temp.clone(),
                        EditSession {
                            alias,
                            run_id,
                            run_dir,
                            base,
                            then_launch,
                        },
                    );
                    self.pending_editor = Some((temp, None));
                    self.needs_redraw = true;
                }
            }
            Action::DriverEditStale {
                alias,
                run_id,
                temp,
            } => {
                self.screen_stack.push(Box::new(
                    crate::ui::screens::driver_steer::EditChooserScreen::new(
                        alias,
                        run_id,
                        temp,
                        crate::ui::screens::driver_steer::ChooserReason::Stale,
                    ),
                ));
                self.needs_redraw = true;
            }
            Action::DriverEditFailed {
                alias: _,
                run_id: _,
                error,
            } => {
                self.ctx.error_message = Some(format!(
                    "Could not start the edit: {}. DIRECTION.md was not changed.",
                    crate::text::render_for_terminal(&error)
                ));
                self.needs_redraw = true;
            }
            Action::DriverEditReopen {
                alias,
                run_id,
                temp: _,
            } => {
                self.request_reopen(&alias, &run_id);
            }
            Action::DriverEditApply { temp, accept_lint } => {
                match self.edit_sessions.get(&temp).cloned() {
                    Some(session) => self.start_apply(temp, &session, accept_lint),
                    None => {
                        self.ctx.error_message = Some(
                            "That edit is no longer open, so nothing was applied. Press [e] to start again."
                                .to_string(),
                        );
                        self.needs_redraw = true;
                    }
                }
            }
            Action::DriverEditDone {
                alias,
                run_id,
                temp,
                outcome,
            } => {
                self.finish_edit(&alias, &run_id, temp, outcome);
            }
            Action::DriverEditDiscard {
                alias,
                run_id,
                temp: _,
            } => {
                self.request_discard(&alias, &run_id);
            }
            Action::DriverEditDiscarded {
                alias: _,
                run_id: _,
                error,
            } => {
                match error {
                    None => {
                        self.ctx.status_message = Some((
                            "Discarded your edit. DIRECTION.md was not changed.".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                    Some(error) => {
                        self.ctx.error_message = Some(format!(
                            "Could not discard your edit: {}. DIRECTION.md was not changed.",
                            crate::text::render_for_terminal(&error)
                        ));
                    }
                }
                self.needs_redraw = true;
            }
            // 30-05, D-19, D-20: the launch form's host check, and the launch itself
            // through the launcher library's detached start. Linux-only, like the
            // launcher; the other arms say so and start nothing.
            Action::DriverLaunchPreflightRequested { alias, run_id } => {
                #[cfg(target_os = "linux")]
                self.request_launch_preflight(&alias, &run_id);
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = (alias, run_id);
                    self.refuse_unsupported_launch();
                }
            }
            Action::DriverLaunchPreflighted {
                alias,
                run_id,
                preflight,
            } => {
                self.ctx
                    .view_cache
                    .entry(alias)
                    .or_default()
                    .launch_preflight = Some((run_id, preflight));
                self.needs_redraw = true;
            }
            Action::DriverLaunchRequested {
                alias,
                run_id,
                args,
            } => {
                #[cfg(target_os = "linux")]
                self.request_launch(&alias, &run_id, args);
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = (alias, run_id, args);
                    self.refuse_unsupported_launch();
                }
            }
            Action::DriverLaunched {
                alias,
                run_id,
                result,
            } => {
                self.finish_launch(&alias, &run_id, result);
            }
            // 30-05, D-18: `s` minted a slug; create the run, edit it, check it, then
            // offer the launch.
            Action::DriverNewRunRequested { alias, slug } => {
                self.request_new_run(&alias, &slug);
            }
            Action::DriverNewRunCreated {
                alias,
                slug: _,
                result,
            } => {
                self.finish_new_run(&alias, result);
            }
            Action::DriverNewRunChecked {
                alias,
                run_id,
                problems,
                warnings: _,
            } => {
                self.finish_new_run_check(&alias, &run_id, problems);
            }
            Action::DriverNewRunLeft { alias, run_id } => {
                self.new_run_flow.remove(&(alias, run_id.clone()));
                self.ctx.status_message = Some((
                    crate::ui::screens::driver_launch::left_unlaunched_copy(&run_id),
                    std::time::Instant::now(),
                ));
                self.needs_redraw = true;
            }
            // ── The Driver surface's four messages ────────────────────
            //
            // Four explicit arms rather than a catch-all: a `_ =>` would let a
            // later variant be added and silently ignored, which is the
            // opposite of what this match's exhaustiveness is for.
            //
            // Nothing is logged from `text`, `report` or any other message
            // body: log lines in this subsystem carry the error **kind** and
            // counts only, because a body can carry agent output (S3).

            // The request arrives from a key handler that touched no file, and
            // it leaves this arm still having touched none (D-22, D-28).
            Action::DriverInjectRequested {
                alias,
                run_id,
                id,
                text,
            } => {
                self.schedule_inbox_append(&alias, &run_id, id, text);
            }
            // The durable append reported. **`queued` is set here and nowhere
            // earlier**, because STEER-03's criterion is survival of the
            // writing process: a buffered write that dies with the TUI
            // satisfies the UI and fails the criterion, so nothing may claim
            // the message is queued before the `sync_data()`-backed write
            // returned `Ok` (D-06).
            //
            // On failure the error copy states that **nothing was written** —
            // the honest thing to say, and the thing that tells the user
            // retrying is safe rather than a way to send the same steer twice.
            Action::DriverInjectWritten {
                alias,
                run_id,
                id,
                error,
            } => {
                match error {
                    Some(kind) => {
                        tracing::warn!(
                            %alias,
                            %run_id,
                            %id,
                            kind = %kind,
                            "injection append failed",
                        );
                        self.ctx.error_message = Some(format!(
                            "Could not queue message: {kind}. Nothing was written — try again."
                        ));
                    }
                    None => {
                        // Deliberately promises nothing about timing. The
                        // dequeue ack was measured at 55 seconds, so any word
                        // implying imminence would be a lie about the protocol
                        // (D-07).
                        self.ctx.status_message = Some((
                            "Queued — waiting for the driver to pick it up.".to_string(),
                            std::time::Instant::now(),
                        ));
                        // **The `queued` state is not observable until the inbox
                        // is re-read** (CR-04). `derive_injection_states`
                        // iterates `cache.driver_inbox`, which the run-list scan
                        // fills; without this the status line above was the only
                        // trace of the message until the driver's `interjected`
                        // record arrived through the journal tail — at which
                        // point the pane showed the message with no state label
                        // at all, because `entries` was still empty. STEER-02's
                        // entire deliverable did not render.
                        //
                        // On the success arm only: a failed append wrote
                        // nothing, so there is nothing new to read.
                        self.rescan_driver_tab_if_open();
                    }
                }
                self.needs_redraw = true;
            }
            // Both payloads replace rather than merge, for the reason
            // `RunsReconciled` does: each read is authoritative, and a run
            // directory or a message no longer on disk is expressed by its
            // absence and by nothing else.
            Action::DriverRunsListed {
                alias,
                runs,
                inbox,
                journal,
            } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                // Clamped rather than reset: a scan that lands while the user is
                // on a run must not move the selection off it, and a selection
                // past the end of a shortened list must not dangle.
                if !runs.is_empty() {
                    cache.driver_selected_run = cache.driver_selected_run.min(runs.len() - 1);
                }
                cache.driver_runs = runs;
                cache.driver_inbox = inbox;
                // The journal carries its own run id, so the renderer can refuse
                // to show it under a run it is not about.
                cache.driver_journal = journal;
                self.needs_redraw = true;
            }
            // The launcher rows have exactly one producer
            // (`AppContext::schedule_launcher_refresh`) and are replaced, never
            // merged: a run directory no longer on disk is expressed by its absence.
            Action::LauncherRunsRefreshed {
                alias,
                launcher_runs,
                detail,
                log,
            } => {
                let alias_for_marker = alias.clone();
                let cache = self.ctx.view_cache.entry(alias).or_default();
                // Pinned to the run id, not the index: a newer run landing above the
                // selected one must not move the selection onto a different run.
                // The run being gone clamps the index into range instead. The same
                // rule chose which run's detail the task read.
                let before = cache.launcher_selected_id.clone();
                cache.launcher_selected = crate::ui::screens::driver_run::pinned_index(
                    &launcher_runs,
                    before.as_deref(),
                    cache.launcher_selected,
                );
                cache.launcher_selected_id = launcher_runs
                    .get(cache.launcher_selected)
                    .map(|(run_id, _)| run_id.clone());
                if cache.launcher_selected_id != before {
                    // The selected run is another one: its cursor starts over.
                    cache.launcher_item_cursor = None;
                }
                cache.launcher_runs = launcher_runs;
                cache.launcher_listed = true;
                // Stored only for the run now selected; a stale result is dropped.
                crate::ui::screens::driver_run::store_detail(cache, detail);
                // The log delta: applied only if it started where the tail stands
                // (`LogRead::from_offset` equals the tail's cursor), so a duplicated
                // or stale read can never show a line twice.
                if let Some(log) = log {
                    crate::ui::screens::driver_run::apply_log_read(cache, *log);
                }
                // The dashboard marker follows the rows just stored (30-03, D-13).
                if self
                    .ctx
                    .refresh_launcher_marker_from_cache(&alias_for_marker)
                {
                    self.ctx.recompute_filtered_aliases();
                }
                self.needs_redraw = true;
            }
            // One launcher run was re-read (30-02, D-24): its row and the cached
            // detail are replaced, nothing else is touched. A read that failed, or a
            // run that is not in the listing yet, is answered by a full re-listing,
            // which yields the typed unreadable row (or the new row).
            Action::LauncherRunRefreshed {
                alias,
                run_id,
                detail,
                log,
            } => {
                let cache = self.ctx.view_cache.entry(alias.clone()).or_default();
                // The log first: a re-listing scheduled below must start from the
                // cursor this read advanced.
                if let Some(log) = log {
                    crate::ui::screens::driver_run::apply_log_read(cache, *log);
                }
                let row = cache
                    .launcher_runs
                    .iter()
                    .position(|(listed, _)| *listed == run_id);
                match (detail, row) {
                    (Ok(detail), Some(row)) => {
                        cache.launcher_runs[row].1 = Ok(detail.view.clone());
                        crate::ui::screens::driver_run::store_detail(cache, Some(detail));
                    }
                    _ => {
                        if let Some(path) =
                            self.ctx.config.projects.get(&alias).map(|p| p.path.clone())
                        {
                            self.ctx.schedule_launcher_refresh(&alias, &path);
                        }
                    }
                }
                // The dashboard marker follows the row just replaced (30-03, D-13).
                if self.ctx.refresh_launcher_marker_from_cache(&alias) {
                    self.ctx.recompute_filtered_aliases();
                }
                self.needs_redraw = true;
            }
            // A one-project marker scan landed (30-03, D-13). Only a registered alias
            // is accepted, so a scan answered after the project was removed leaves no
            // cache entry behind.
            Action::LauncherMarkerScanned { alias, marker } => {
                if self.ctx.experimental && self.ctx.config.projects.contains_key(&alias) {
                    let cache = self.ctx.view_cache.entry(alias).or_default();
                    if cache.launcher_marker != Some(marker) {
                        cache.launcher_marker = Some(marker);
                        self.ctx.recompute_filtered_aliases();
                        self.needs_redraw = true;
                    }
                }
            }
            // The report was built off the render thread and arrived (D-26).
            //
            // The load-bearing half of D-26 is the *scheduling*:
            // `build_report` shells out to `git` twice and is one of the WR-10
            // call sites the Phase 17 review named, so it must never run on the
            // render thread. `AppContext::schedule_dry_run_report` discharges
            // that in full, and it is the half with a threat-register row
            // (T-18-25, T-18-62).
            //
            // This arm is the other half, landed by plan 18-11: the string is
            // parked in `ProjectViewCache::driver_dry_run`, which the start
            // screen created when it reached Step B.
            //
            // **Guarded twice, and neither guard is belt-and-braces.** A report
            // is stored only when a preview is actually open — a report that
            // arrives after the user pressed Esc has nowhere to go, and
            // recreating the preview for it would repaint a pane the user has
            // left — and only when the command it was built for is still the
            // command the preview is about. Without the second guard, a user who
            // went back to Step A and retyped would see the previous command's
            // blast radius under the new command's name, which is the display
            // disagreeing with the disk in the direction that flatters the run.
            //
            // Logged by count, never by body: the report interpolates a working
            // tree's file names (S3).
            Action::DriverDryRunLoaded {
                alias,
                command,
                report,
            } => {
                tracing::debug!(
                    %alias,
                    %command,
                    bytes = report.len(),
                    "dry-run report built off the render thread",
                );
                if let Some(preview) = self
                    .ctx
                    .view_cache
                    .get_mut(&alias)
                    .and_then(|cache| cache.driver_dry_run.as_mut())
                    .filter(|preview| preview.command == command)
                {
                    preview.report = Some(report);
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Drop driver state the registry no longer justifies, and bound what stays
    /// (D-27).
    ///
    /// **The leak this closes.** `journal_cursors` is keyed `(alias, run_id)`,
    /// inserted in exactly one place — `Action::DriverJournalAppended` — and
    /// removed in none, so it grew for the whole life of the process: one entry
    /// per run of every project ever tailed, forever. `run_states` and
    /// `observed_runs` had the same shape for an unregistered alias, because
    /// `registry::remove_project` touches neither (it has no access to
    /// `AppContext`; see its doc, which names both cleanup sites).
    ///
    /// Two passes:
    ///
    /// 1. **Unregistered aliases go.** A project that is no longer a key in
    ///    `config.projects` cannot produce another journal append, another run
    ///    state, or another observation, so every entry it owns is dead weight.
    /// 2. **At most [`RETAIN_RUNS`](crate::journal::RETAIN_RUNS) run ids survive
    ///    per alias**, newest kept. Aligning to that constant rather than
    ///    picking a second number is deliberate: it is the on-disk retention
    ///    bound, so the in-memory bound and the disk bound move together instead
    ///    of drifting into two unrelated budgets.
    ///
    /// **"Newest" is a sort on the run-id string and needs no file read.** Run
    /// ids sort lexicographically in chronological order by construction — that
    /// is `new_run_id`'s documented load-bearing property, and the same one
    /// on-disk pruning relies on. A reader might otherwise reach for `run.json`
    /// timestamps, which would turn a map operation into O(runs) file reads.
    ///
    /// Called from the **existing** 20-tick block, never from a timer of its own
    /// (D-13, D-27).
    fn prune_driver_maps(&mut self) {
        let registered = &self.ctx.config.projects;
        self.ctx
            .journal_cursors
            .retain(|(alias, _), _| registered.contains_key(alias));
        self.ctx
            .run_states
            .retain(|alias, _| registered.contains_key(alias));
        self.ctx
            .observed_runs
            .retain(|alias, _| registered.contains_key(alias));
        // Its sibling joins the pass, and for the same reason (WR-02): keyed by
        // alias, filled by every scan, and otherwise growing for the life of the
        // process one entry per project ever driven and then unregistered.
        self.ctx
            .last_outcomes
            .retain(|alias, _| registered.contains_key(alias));
        // The ring buffer joins the pass, and this line is the phase's only
        // remaining Phase 16 carry-forward — a negative one. `driver_output` is
        // keyed by alias and inserted on every journal append; without this it
        // would grow for the life of the process, holding up to
        // `DRIVER_OUTPUT_RING_LINES` lines for every project ever driven and
        // then unregistered. That is the Phase 16 leak reintroduced under a new
        // name, which is exactly what the carry-forward exists to prevent.
        //
        // No per-run second pass, because the map has no run dimension: one
        // alias holds one buffer, already bounded by its own cap.
        self.ctx
            .driver_output
            .retain(|alias, _| registered.contains_key(alias));
        // `view_cache` joins the pass too, and plan 18-09 is what makes that
        // necessary rather than tidy. It is keyed by alias and was never pruned,
        // which cost little while it held browse listings and git logs for
        // projects the user had merely visited — but this phase moved three
        // driver payloads into it: `driver_runs` (a `RunSummary` per run on
        // disk), `driver_inbox` (every queued message for the selected run) and
        // `driver_tally`. Those are exactly the per-alias driver state the
        // phase's carry-forward is about, so leaving them in an unpruned map
        // would satisfy the obligation for the maps it names while breaking it
        // for the one it does not.
        //
        // Dropping the whole entry for an unregistered alias is correct rather
        // than merely convenient: the project is gone, so every view state it
        // held — driver and otherwise — describes something the user can no
        // longer open.
        self.ctx
            .view_cache
            .retain(|alias, _| registered.contains_key(alias));
        // The last two alias-keyed maps on `AppContext`, added by plan 18-11 so
        // that "every per-alias map is pruned" is literally true rather than
        // true of the driver maps and quietly false of two neighbours.
        //
        // Neither predates this pass by accident: `last_refresh` holds one
        // `Instant` per alias ever watched and `archive_cache` holds a whole
        // parsed milestone archive per alias ever browsed, and neither has a
        // removal site. They are small and slow-growing rather than harmless —
        // "small leak" is how the Phase 16 leak was described before it was
        // measured. Both are caches, so dropping an entry costs at most one
        // re-derivation for a project that no longer exists.
        self.ctx
            .last_refresh
            .retain(|alias, _| registered.contains_key(alias));
        // `archive_cache` was keyed by MILESTONE when this line first retained
        // it by alias, so every archive went on every pass and the Archive
        // tab's drill-in flipped to `Loading...` five seconds after it loaded
        // (debug session `archive-milestone-view-loading`). It is now an
        // `ArchiveCache`, keyed alias first, whose only prune names aliases.
        self.ctx
            .archive_cache
            .retain_aliases(|alias| registered.contains_key(alias));

        let mut runs_by_alias: HashMap<&str, Vec<&str>> = HashMap::new();
        for (alias, run_id) in self.ctx.journal_cursors.keys() {
            runs_by_alias
                .entry(alias.as_str())
                .or_default()
                .push(run_id.as_str());
        }

        let mut evict: Vec<(String, String)> = Vec::new();
        for (alias, mut run_ids) in runs_by_alias {
            if run_ids.len() <= crate::journal::RETAIN_RUNS {
                continue;
            }
            run_ids.sort_unstable();
            let excess = run_ids.len() - crate::journal::RETAIN_RUNS;
            for run_id in run_ids.into_iter().take(excess) {
                evict.push((alias.to_string(), run_id.to_string()));
            }
        }
        for key in evict {
            self.ctx.journal_cursors.remove(&key);
        }
    }

    /// Start a driver run against `alias`, or refuse visibly (D-03, D-18).
    ///
    /// The order of the body is the decision:
    ///
    /// 1. Look the alias up, refusing an unknown one through `ctx.error_message`.
    /// 2. [`admit`](crate::driver::spawn::admit) against
    ///    `preferences.driver_max_concurrent`, counting the live entries of the
    ///    reconciliation scan's own result. Refuse through `ctx.error_message`.
    /// 3. Generate the run id **here** — the TUI owns the id so it knows what to
    ///    look for afterwards, and the driver owns the record because `run.json`
    ///    carries the driver's own pid and pgid, which only the driver knows
    ///    (D-03).
    /// 4. Build the argv, spawn detached, report either way.
    ///
    /// **There is deliberately no opt-in check here, and the absence is the
    /// point** (D-16). The gate lives in the child, at `driver::drive`, which is
    /// what makes CTRL-03's "never" literally true for both entry points: a user
    /// typing `gsd-meta-manager drive foo` by hand is refused by the same code
    /// path as this one. A second check here would put the gate at two call
    /// sites, and "exactly one call site" is a property
    /// `tests/spawn_seam_guard.rs` can check while "every branch remembered to
    /// gate" is not.
    ///
    /// Unix-only, like the rest of the driver (D-05). The TUI itself stays
    /// cross-platform; a Windows build simply has no way to start a run.
    #[cfg(unix)]
    pub fn start_driver_run(&mut self, alias: &str, command: &str, goal: Option<&str>) {
        use crate::driver::spawn::{admit, drive_argv, spawn_detached};

        // The TUI must not be able to start a run while the surface that offers
        // it does not exist (260917-fko D2). Defensive: every key that reaches
        // here is already gated, and this is the backstop for the one that is
        // ever added without one. It is silent on purpose — an error message
        // would itself disclose that a driver exists.
        if !self.ctx.experimental {
            return;
        }

        let Some(project) = self.ctx.config.projects.get(alias) else {
            self.ctx.error_message = Some(format!("No registered project named '{alias}'"));
            self.needs_redraw = true;
            return;
        };
        let project_root = project.path.clone();
        // Cloned beside `project_root` and for the same borrow-checker reason:
        // `project` borrows `self.ctx.config`, and the `admit` call below needs
        // `self.ctx` again. It is the config the TUI itself is using, and handing
        // it to the child is what stops the spawned driver resolving this alias in
        // a different registry (CR-03).
        let config_path = self.ctx.config_path.clone();

        let live = self
            .ctx
            .observed_runs
            .values()
            .filter(|run| run.is_live())
            .count();
        if let Err(refusal) = admit(live, self.ctx.config.preferences.driver_max_concurrent) {
            self.ctx.error_message = Some(refusal.to_string());
            self.needs_redraw = true;
            return;
        }

        let run_id = crate::journal::new_run_id(chrono::Utc::now(), &uuid::Uuid::new_v4());
        let argv = drive_argv(&config_path, alias, command, &run_id, goal);

        match spawn_detached(&project_root, &argv) {
            Ok(pid) => {
                // This session is the driver's parent, so its `wait()` belongs
                // to the reaping task `spawn_detached` just created. Recording
                // the run id here is what lets a later stop pick D-07's parent
                // arm; a run absent from this set was adopted after a restart
                // and can only be confirmed dead through `/proc`.
                self.ctx.session_spawned_runs.insert(run_id.clone());
                self.ctx.status_message = Some((
                    format!("Driving {alias} — run {run_id}"),
                    std::time::Instant::now(),
                ));
                // An optimistic entry so the dashboard does not wait up to five
                // seconds for the first scan to notice. The next scan replaces
                // it wholesale from disk, which is what corrects it if the
                // driver refused at its own gate and exited immediately.
                self.ctx.observed_runs.insert(
                    alias.to_string(),
                    crate::driver::reconcile::ObservedRun {
                        alias: alias.to_string(),
                        run_id,
                        pid,
                        // The child is its own group leader, so pgid == pid
                        // (D-04). The scan reads the driver's own record a few
                        // seconds later and would correct this if it were wrong.
                        pgid: pid,
                        started_at: chrono::Utc::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        goal: goal.unwrap_or_default().to_string(),
                        gsd_command: command.to_string(),
                        // `Alive` and not the probe's answer: the spawn just
                        // succeeded, so this entry asserts what this process
                        // knows first-hand rather than what /proc could say
                        // about a pid that is microseconds old.
                        liveness: crate::driver::liveness::Liveness::Alive,
                    },
                );

                // **The new run must reach the run list without a tab
                // round-trip** (CR-04). The wizard hands off and pops; it does
                // not re-enter the Driver tab, so nothing else would schedule
                // this and `render_driver_tab` would take its `runs.is_empty()`
                // early return — painting "No runs yet … Press [s] to start
                // one" over a run that is live and whose ring buffer is filling
                // behind it. That is the first thing a new user does with the
                // tab.
                //
                // **Ungated**, unlike the tick's rescan: the confirmation screen
                // is still on top of the stack at this moment, so a visibility
                // gate would refuse the one scan the user explicitly asked for.
                // The child may not have created its run directory yet — this is
                // a spawn, microseconds old — in which case the scan reads what
                // is there and the tick's rescan picks the run up within the
                // same five seconds the reconciliation probe already runs on.
                self.ctx.schedule_run_list_scan(alias, &project_root);
            }
            Err(e) => {
                // A spawn failure is synchronous and leaves genuinely nothing on
                // disk, which is the correct state rather than a lost run (D-03).
                self.ctx.error_message =
                    Some(format!("Could not start a driver for '{alias}': {e}"));
            }
        }
        self.needs_redraw = true;
    }

    /// Stop the observed run on `alias`, or refuse visibly (CTRL-01, D-06, D-07).
    ///
    /// The counterpart to [`App::start_driver_run`], and the order of the body is
    /// the decision:
    ///
    /// 1. Read the `ObservedRun` for the alias **out of the scan's own result**.
    ///    The pid and the pgid are taken here, at dispatch, and never carried in
    ///    the key event that asked for the stop: a pgid captured earlier is a
    ///    pgid that may already name a different run's group, and this is the one
    ///    value in the codebase where being stale means signalling a stranger.
    ///    No entry is a visible refusal through `ctx.error_message`.
    /// 2. Choose D-07's arm from `ctx.session_spawned_runs` — `Parent` if this
    ///    session spawned it, `Adopted` otherwise. That set exists rather than a
    ///    flag on `ObservedRun` because the observed map is replaced wholesale by
    ///    every scan and the disk cannot say who a driver's parent was.
    /// 3. Dispatch on a task and take the result back as
    ///    [`Action::DriverStopped`]. **Nothing here blocks the render thread**:
    ///    the grace alone is twelve seconds, and a loop that stopped painting for
    ///    twelve seconds while stopping a run would look exactly like the hang
    ///    the stop exists to end (TRANS-03).
    ///
    /// Unix-only, like the rest of the driver (D-05).
    #[cfg(unix)]
    pub fn stop_driver_run(&mut self, alias: &str) {
        use crate::driver::kill::ReapArm;

        // The sibling of the guard in `start_driver_run`, and for the same
        // reason: the TUI must not be able to stop a run while the surface that
        // offers it does not exist (260917-fko D2). A run started by the `drive`
        // CLI, which stays ungated (D5), is still stoppable from the CLI side.
        if !self.ctx.experimental {
            return;
        }

        let Some(run) = self.ctx.observed_runs.get(alias) else {
            self.ctx.error_message = Some(format!("No run is being observed for '{alias}'"));
            self.needs_redraw = true;
            return;
        };
        let pid = run.pid;
        let pgid = run.pgid;
        let run_id = run.run_id.clone();

        let arm = if self.ctx.session_spawned_runs.contains(&run_id) {
            ReapArm::Parent
        } else {
            ReapArm::Adopted
        };

        // The same guard `schedule_journal_tail` uses: without a channel there
        // is nowhere to return the outcome, and a stop whose result cannot be
        // reported is a stop the user cannot tell happened.
        //
        // **It refuses visibly** (WR-11, S4). A bare `return` here left the user
        // pressing the stop key against a live run and being told nothing at
        // all, which reads as "the key does not work" — and the run keeps
        // driving their repository meanwhile. The condition is not reachable in
        // production (the channel is installed at startup and held for the
        // process lifetime), which is exactly why the silent arm survived: it
        // only fires in a test or after a wiring regression, and both are cases
        // where silence costs the most.
        let Some(tx) = &self.ctx.event_tx else {
            self.ctx.error_message = Some(format!(
                "Could not stop the run on '{alias}': this session has no event channel."
            ));
            self.needs_redraw = true;
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let run_id_for_task = run_id.clone();

        tokio::spawn(async move {
            let outcome = crate::driver::kill::stop_run(pid, pgid, &run_id_for_task, arm).await;
            let _ = tx.send(Action::DriverStopped {
                alias: alias_for_task,
                run_id: run_id_for_task,
                // The disposition is derived here, at the seam that still holds
                // the typed `StopOutcome`, and travels beside the rendered
                // text. Recovering it downstream by matching on that text would
                // be screen-scraping this project's own output (D-29).
                disposition: crate::action::StopDisposition::from(&outcome),
                outcome: outcome.to_string(),
            });
        });

        self.ctx.status_message = Some((
            format!("Stopping {alias} — run {run_id}"),
            std::time::Instant::now(),
        ));
        self.needs_redraw = true;
    }

    /// One mouse event from the reader (quick 260926-dyf).
    ///
    /// Ignored while capture is off. A left press goes through the click tracker
    /// to decide `double` (I-6); the result reaches the TOP screen only, whose
    /// default `handle_mouse` ignores it (I-13).
    pub fn handle_mouse(&mut self, event: crossterm::event::MouseEvent, now: std::time::Instant) {
        use crate::ui::mouse::{routed, MouseInput, Routed};
        if !self.mouse_capture {
            return;
        }
        let input = match routed(&event) {
            Some(Routed::Click { column, row }) => MouseInput::Click {
                column,
                row,
                double: self.click_tracker.register(now, column, row),
            },
            Some(Routed::Wheel { column, row, down }) => MouseInput::Wheel { column, row, down },
            None => return,
        };
        if let Some(screen) = self.screen_stack.last_mut() {
            let action = screen.handle_mouse(input, &mut self.ctx);
            self.process_screen_action(action);
        }
        self.needs_redraw = true;
    }

    /// Open `alias`'s project view at launch, exactly as if the user had
    /// selected its overview row and pressed Enter (quick 260930-vvk, D-05).
    ///
    /// Returns whether the project view is now on top. Does nothing (and
    /// returns `false`) unless the root overview is the only screen, or when
    /// `alias` is not among the overview's rows.
    ///
    /// It drives the overview's OWN Enter handler rather than pushing a
    /// `DetailScreen` itself: a parallel push would drift from Enter the first
    /// time Enter gains behaviour, and Esc would stop being identical to a
    /// drilled-in view. Selecting the row first is what makes Esc land on the
    /// overview with this project selected.
    pub fn open_project_view(&mut self, alias: &str) -> bool {
        if self.screen_stack.len() != 1 {
            return false;
        }
        let Some(index) = self.ctx.filtered_aliases.iter().position(|a| a == alias) else {
            return false;
        };
        self.ctx.table_state.select(Some(index));
        self.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        self.screen_stack
            .last()
            .is_some_and(|top| top.name() == crate::ui::screens::detail::DetailScreen::NAME)
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // A key between two presses means they are not a double-click (I-6).
        self.click_tracker.reset();
        // Ctrl+C always quits regardless of mode
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        if let Some(screen) = self.screen_stack.last_mut() {
            let action = screen.handle_key(code, modifiers, &mut self.ctx);
            self.process_screen_action(action);
        }
    }

    fn process_screen_action(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Push(screen) => {
                self.screen_stack.push(screen);
                self.needs_redraw = true;
            }
            // Pop-then-push, and the guard is the same one `Pop` carries: the
            // root screen is never removed, so a `Replace` from it degrades to a
            // plain `Push` rather than emptying the stack.
            ScreenAction::Replace(screen) => {
                if self.screen_stack.len() > 1 {
                    self.screen_stack.pop();
                }
                self.screen_stack.push(screen);
                self.needs_redraw = true;
            }
            ScreenAction::Pop => {
                if self.screen_stack.len() > 1 {
                    self.screen_stack.pop();
                    self.needs_redraw = true;
                }
            }
            ScreenAction::Quit => {
                self.should_quit = true;
            }
            ScreenAction::SetStatusMessage(msg) => {
                self.ctx.status_message = Some((msg, std::time::Instant::now()));
                self.needs_redraw = true;
            }
            ScreenAction::SuspendAndEdit(path, line) => {
                self.pending_editor = Some((path, line));
            }
            ScreenAction::DispatchAction(action) => {
                if let Some(tx) = &self.ctx.event_tx {
                    let _ = tx.send(*action);
                }
            }
            // Runtime only: nothing is written to `config.json` (D-06, I-2).
            ScreenAction::ToggleMouseCapture => {
                self.mouse_capture = !self.mouse_capture;
                self.ctx.status_message = Some((
                    crate::ui::mouse::mouse_status_text(self.mouse_capture).to_string(),
                    std::time::Instant::now(),
                ));
                self.needs_redraw = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    /// Todo 2026-07-29: after `$EDITOR` exits the browser viewer re-reads the
    /// edited file; a different file's cache is left alone.
    #[test]
    fn editor_exit_reloads_browser_file_cache() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.md");
        std::fs::write(&f, "old").unwrap();
        let mut app = App::new_for_test();
        let cache = app.ctx.view_cache.entry("p".into()).or_default();
        cache.browser_file_content = Some("old".into());
        cache.browser_file_path = Some(f.clone());
        std::fs::write(&f, "new").unwrap();
        app.refresh_after_editor(&dir.path().join("other.md"));
        assert_eq!(
            app.ctx.view_cache["p"].browser_file_content.as_deref(),
            Some("old")
        );
        app.refresh_after_editor(&f);
        assert_eq!(
            app.ctx.view_cache["p"].browser_file_content.as_deref(),
            Some("new")
        );
        assert!(app.needs_redraw);
    }

    /// After `$EDITOR` exits the archive viewer re-reads the edited file; a
    /// different file's cache is left alone.
    #[test]
    fn editor_exit_reloads_archive_file_cache() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.md");
        std::fs::write(&f, "old").unwrap();
        let mut app = App::new_for_test();
        let cache = app.ctx.view_cache.entry("p".into()).or_default();
        cache.archive_file_content = Some("old".into());
        cache.archive_file_path = Some(f.clone());
        std::fs::write(&f, "new").unwrap();
        app.refresh_after_editor(&dir.path().join("other.md"));
        assert_eq!(
            app.ctx.view_cache["p"].archive_file_content.as_deref(),
            Some("old")
        );
        app.refresh_after_editor(&f);
        assert_eq!(
            app.ctx.view_cache["p"].archive_file_content.as_deref(),
            Some("new")
        );
        assert!(app.needs_redraw);
    }

    /// quick 260926-j0a: the startup line is exactly `startup_summary`'s text
    /// over the injected global install and the loaded project states.
    #[test]
    fn announce_gsd_install_sets_the_startup_summary() {
        use crate::state_reader::config_json::GSD_CORE_SYNCED_TREE_VERSION;
        use crate::state_reader::gsd_install::{detect_gsd_install, startup_summary, InstallRoots};
        let home = tempfile::tempdir().unwrap();
        let core = home.path().join(".claude/gsd-core");
        std::fs::create_dir_all(&core).unwrap();
        std::fs::write(core.join("VERSION"), GSD_CORE_SYNCED_TREE_VERSION).unwrap();
        let project = tempfile::tempdir().unwrap();
        let local = project.path().join(".codex/gsd-core");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(local.join("VERSION"), "0.1.0").unwrap();
        let roots = InstallRoots {
            home: Some(home.path().to_path_buf()),
            ..Default::default()
        };

        let mut app = App::new_for_test();
        app.ctx.project_states.insert(
            "local".to_string(),
            ProjectState {
                gsd_install: detect_gsd_install(Some(project.path()), &roots),
                ..Default::default()
            },
        );
        app.ctx
            .project_states
            .insert("plain".to_string(), ProjectState::default());
        app.ctx.status_message = None;
        app.needs_redraw = false;

        app.announce_gsd_install_with(&roots);

        let expected = startup_summary(
            &detect_gsd_install(None, &roots),
            app.ctx.project_states.values().map(|s| &s.gsd_install),
        );
        let (text, _) = app
            .ctx
            .status_message
            .clone()
            .expect("a status message is set");
        assert_eq!(text, expected);
        assert!(
            text.starts_with(&format!(
                "GSD {GSD_CORE_SYNCED_TREE_VERSION} (global Claude)"
            )),
            "{text}"
        );
        assert!(
            text.ends_with("1 project uses a project-local GSD"),
            "{text}"
        );
        assert!(app.needs_redraw);
    }

    /// quick-260924-drx: the line reaches the editor in the syntax it reads.
    #[test]
    fn editor_args_positions_known_editors_and_leaves_unknown_ones_alone() {
        use std::ffi::OsString;
        let p = std::path::Path::new("/r/.planning/ROADMAP.md");
        let os = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            editor_args("nvim", p, Some(42)),
            os(&["+42", "/r/.planning/ROADMAP.md"])
        );
        assert_eq!(
            editor_args("/usr/bin/vim", p, Some(7)),
            os(&["+7", "/r/.planning/ROADMAP.md"])
        );
        assert_eq!(
            editor_args("nano", p, Some(3)),
            os(&["+3", "/r/.planning/ROADMAP.md"])
        );
        assert_eq!(
            editor_args("code", p, Some(9)),
            os(&["-g", "/r/.planning/ROADMAP.md:9"])
        );
        assert_eq!(
            editor_args("hx", p, Some(5)),
            os(&["/r/.planning/ROADMAP.md:5"])
        );
        // Unknown editor: path only, never a guessed flag.
        assert_eq!(
            editor_args("ed", p, Some(5)),
            os(&["/r/.planning/ROADMAP.md"])
        );
        // No line (or a nonsensical 0): path only, whatever the editor.
        assert_eq!(
            editor_args("nvim", p, None),
            os(&["/r/.planning/ROADMAP.md"])
        );
        assert_eq!(
            editor_args("nvim", p, Some(0)),
            os(&["/r/.planning/ROADMAP.md"])
        );
    }

    /// WR-06: a bare program keeps the shipped direct spawn, argv unchanged.
    #[test]
    fn editor_command_runs_a_bare_program_directly() {
        use std::ffi::OsString;
        let p = std::path::Path::new("/r/.planning/DIRECTION.md");
        assert_eq!(
            editor_command("nvim", p, Some(42)),
            (
                OsString::from("nvim"),
                vec![
                    OsString::from("+42"),
                    OsString::from("/r/.planning/DIRECTION.md")
                ]
            )
        );
        assert_eq!(
            editor_command("", p, None),
            (
                OsString::from("vi"),
                vec![OsString::from("/r/.planning/DIRECTION.md")]
            ),
            "an empty editor string is `vi`"
        );
    }

    /// WR-06 / git's `prepare_shell_cmd` rule: an editor string with arguments runs as
    /// `sh -c '<editor> "$@"' '<editor>' <args...>`.
    #[cfg(unix)]
    #[test]
    fn editor_command_wraps_an_editor_with_arguments_in_sh() {
        use std::ffi::OsString;
        let p = std::path::Path::new("/r/.planning/DIRECTION.md");
        let os = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            editor_command("code -w", p, Some(7)),
            (
                OsString::from("sh"),
                os(&[
                    "-c",
                    "code -w \"$@\"",
                    "code -w",
                    "-g",
                    "/r/.planning/DIRECTION.md:7"
                ])
            )
        );
        assert_eq!(
            editor_command("emacs -nw", p, None),
            (
                OsString::from("sh"),
                os(&[
                    "-c",
                    "emacs -nw \"$@\"",
                    "emacs -nw",
                    "/r/.planning/DIRECTION.md"
                ])
            )
        );
    }

    /// T-30-27: the path (inside a run directory a cloned repository or a sibling sandbox
    /// names) is only ever a separate argv element after `$0`, never part of the script.
    #[cfg(unix)]
    #[test]
    fn editor_command_never_puts_the_path_in_the_shell_script() {
        let hostile = "/runs/$(touch PWNED);`id`'\"x/DIRECTION.md";
        let p = std::path::Path::new(hostile);
        for editor in [
            "code -w",
            "nvim -u NONE",
            "'/opt/My Ed/ed' --wait",
            "emacs -nw",
        ] {
            for line in [None, Some(3)] {
                let (program, argv) = editor_command(editor, p, line);
                assert_eq!(program, std::ffi::OsString::from("sh"), "{editor}");
                assert_eq!(argv[0], std::ffi::OsString::from("-c"), "{editor}");
                let script = argv[1].to_string_lossy().into_owned();
                assert_eq!(script, format!("{editor} \"$@\""), "{editor}");
                assert!(!script.contains("PWNED"), "{script}");
                assert_eq!(
                    argv[2],
                    std::ffi::OsString::from(editor),
                    "$0 is the editor"
                );
                assert!(
                    argv[3..]
                        .iter()
                        .any(|a| a.to_string_lossy().contains(hostile)),
                    "the path rides as its own argv element: {argv:?}"
                );
            }
        }
    }

    /// WR-06: line positioning is keyed on the file name of the editor string's first word.
    #[cfg(unix)]
    #[test]
    fn editor_command_positions_the_line_for_the_first_word() {
        use std::ffi::OsString;
        let p = std::path::Path::new("/r/DIRECTION.md");
        let tail = |editor: &str, line: usize| {
            let (_, argv) = editor_command(editor, p, Some(line));
            argv[3..].to_vec()
        };
        let os = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(tail("nvim -u NONE", 3), os(&["+3", "/r/DIRECTION.md"]));
        assert_eq!(tail("code -w", 9), os(&["-g", "/r/DIRECTION.md:9"]));
        assert_eq!(
            tail("/usr/local/bin/code --wait", 4),
            os(&["-g", "/r/DIRECTION.md:4"])
        );
        assert_eq!(
            tail("\"/opt/My Editors/nvim\" -u NONE", 2),
            os(&["+2", "/r/DIRECTION.md"]),
            "a quoted first word is unquoted before it is keyed"
        );
        assert_eq!(
            tail("ed -s", 5),
            os(&["/r/DIRECTION.md"]),
            "an unknown editor never gets a guessed flag"
        );
    }

    #[test]
    fn an_unreadable_state_md_is_admitted_not_guessed_around() {
        // The exact shape the bug produced: STATE.md on disk, frontmatter lost,
        // counts arriving from ROADMAP's Progress table. The old code printed a
        // confident "P3: ..." here.
        let state = ProjectState {
            state_md_unreadable: true,
            state_md_fault: Some(crate::state_reader::state_md::FrontmatterFault::NotAMapping),
            completed_phases: 2,
            total_phases: 8,
            phases: vec![RoadmapPhase {
                number: "3".to_string(),
                name: "Vertical Slice".to_string(),
                description: String::new(),
                completed: false,
                total_plans: 0,
                completed_plans: 0,
                depends_on: Vec::new(),
            }],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), UNREADABLE_STATE_LABEL);
    }

    /// A folder that exists without `.planning/` is not "gone": it gets its own
    /// marker so the two fixes a user needs are not confused (quick-260922-hdh).
    #[test]
    fn folder_presence_no_planning_shows_its_own_marker() {
        let state = ProjectState {
            presence: ProjectPresence::NoPlanning,
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "(no .planning)");
    }

    /// A missing folder outranks the unreadable and recovered branches: both
    /// describe a file that is not there any more.
    #[test]
    fn folder_presence_marker_outranks_unreadable_and_recovered() {
        let unreadable = ProjectState {
            presence: ProjectPresence::FolderMissing,
            state_md_unreadable: true,
            ..Default::default()
        };
        assert_eq!(format_phase_display(&unreadable), MISSING_FOLDER_LABEL);

        let recovered = ProjectState {
            presence: ProjectPresence::FolderMissing,
            state_md_recovered: true,
            ..Default::default()
        };
        let cell = format_phase_display(&recovered);
        assert_eq!(cell, MISSING_FOLDER_LABEL);
        assert!(!cell.starts_with(RECOVERED_STATE_MARKER));

        let present = format_phase_display(&ProjectState::default());
        assert!(!present.contains(MISSING_FOLDER_LABEL));
        assert!(!present.contains(NO_PLANNING_LABEL));
    }

    /// "Unreadable" tells a user the file is broken; the line tells them where
    /// to look. The number is the only thing the parser knew that crosses into
    /// the cell — no parser vocabulary comes with it.
    #[test]
    fn an_unreadable_state_md_with_a_located_fault_names_its_file_line() {
        let state = ProjectState {
            state_md_unreadable: true,
            state_md_fault: Some(crate::state_reader::state_md::FrontmatterFault::InvalidYaml),
            state_md_fault_position: Some(
                crate::state_reader::state_md::FrontmatterFaultPosition {
                    line: 7,
                    column: 218,
                },
            ),
            ..Default::default()
        };
        let cell = format_phase_display(&state);
        assert_eq!(cell, "! STATE.md unreadable (line 7)");
        for parser_word in ["mapping values", "serde", "yaml", "YAML", "context"] {
            assert!(
                !cell.contains(parser_word),
                "the parser's vocabulary must not reach the cell: {cell:?}"
            );
        }
        // The column is carried on the state for the detail pane, not spent on
        // a width-constrained cell.
        assert!(!cell.contains("218"));
    }

    /// A fault the parser could not locate renders the bare constant. An
    /// invented line number would be worse than none.
    #[test]
    fn an_unlocated_fault_renders_the_bare_unreadable_label() {
        assert_eq!(unreadable_state_label(None), UNREADABLE_STATE_LABEL);
    }

    /// A repaired STATE.md is a third state: the phase IS known, so the cell
    /// must not read as unreadable — but the document was rewritten to get it,
    /// so the cell must not read as a clean one either.
    #[test]
    fn a_recovered_state_md_renders_neither_the_plain_label_nor_the_unreadable_one() {
        let state = ProjectState {
            state_md_recovered: true,
            current_phase_name: "Routine Event Logging".to_string(),
            ..Default::default()
        };
        let cell = format_phase_display(&state);
        assert_ne!(cell, UNREADABLE_STATE_LABEL);
        assert_ne!(
            cell, "Routine Event Logging",
            "a repaired document must not present as a clean one"
        );
        assert_eq!(cell, "~ Routine Event Logging");
        // And the clean read of the same label is the bare form.
        let clean = ProjectState {
            current_phase_name: "Routine Event Logging".to_string(),
            ..Default::default()
        };
        assert_eq!(format_phase_display(&clean), "Routine Event Logging");
    }

    #[test]
    fn an_absent_state_md_still_renders_a_phase() {
        // Same defaulted fields, but nothing failed to be read — so the
        // count-derived label is a legitimate inference, not a cover-up.
        let state = ProjectState {
            state_md_unreadable: false,
            completed_phases: 2,
            total_phases: 8,
            phases: vec![RoadmapPhase {
                number: "3".to_string(),
                name: "Vertical Slice".to_string(),
                description: String::new(),
                completed: false,
                total_plans: 0,
                completed_plans: 0,
                depends_on: Vec::new(),
            }],
            ..Default::default()
        };
        assert_ne!(format_phase_display(&state), UNREADABLE_STATE_LABEL);
    }

    #[test]
    fn test_format_phase_display_completed_milestone() {
        let state = ProjectState {
            completed_phases: 4,
            total_phases: 4,
            milestone: "v1.0".to_string(),
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "v1.0 Complete");
    }

    #[test]
    fn test_format_phase_display_completed_no_milestone() {
        let state = ProjectState {
            completed_phases: 4,
            total_phases: 4,
            milestone: "".to_string(),
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "Complete");
    }

    #[test]
    fn test_format_phase_display_in_progress() {
        let state = ProjectState {
            completed_phases: 1,
            total_phases: 4,
            milestone: "v1.0".to_string(),
            phases: vec![
                RoadmapPhase {
                    number: "1".to_string(),
                    name: "Foundation".to_string(),
                    description: String::new(),
                    completed: true,
                    total_plans: 0,
                    completed_plans: 0,
                    depends_on: Vec::new(),
                },
                RoadmapPhase {
                    number: "2".to_string(),
                    name: "Dashboard".to_string(),
                    description: String::new(),
                    completed: false,
                    total_plans: 0,
                    completed_plans: 0,
                    depends_on: Vec::new(),
                },
            ],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "P2: Dashboard");
    }

    #[test]
    fn test_classify_status_milestone_terminal_is_complete() {
        assert_eq!(
            classify_status("v1.5.0 milestone complete"),
            StatusCategory::Complete
        );
        assert_eq!(
            classify_status("Awaiting next milestone"),
            StatusCategory::Complete
        );
    }

    #[test]
    fn test_classify_status_all_phases_complete_is_idle() {
        // Intermediate ADR-2207 state — NOT Complete.
        assert_eq!(classify_status("All phases complete"), StatusCategory::Idle);
    }

    #[test]
    fn test_classify_status_ordinary_unchanged() {
        assert_eq!(classify_status("executing"), StatusCategory::Active);
        assert_eq!(classify_status("blocked"), StatusCategory::Blocked);
        assert_eq!(classify_status("ready to plan"), StatusCategory::Idle);
        assert_eq!(classify_status("shipped"), StatusCategory::Unknown);
        // A plain phase-completion status still classifies as Complete.
        assert_eq!(classify_status("phase complete"), StatusCategory::Complete);
    }

    #[test]
    fn test_classify_status_awaiting_verification_is_idle() {
        // Body wording written by gsd-core fde7d4e80 `nextPhaseWording`.
        assert_eq!(
            classify_status("Phase complete — ready for verification"),
            StatusCategory::Idle
        );
        // The value actually read from disk: `ProjectState.status` is the
        // frontmatter `status:` (state_md.rs:683) and upstream
        // `STATUS_EXACT_TOKENS` normalizes the wording above to `verifying`.
        assert_eq!(classify_status("verifying"), StatusCategory::Idle);
        // Upstream's anchored `verifying phase N` form.
        assert_eq!(classify_status("Verifying phase 5"), StatusCategory::Idle);
        // ASCII-hyphen variant passes through the exact-token normalizer verbatim.
        assert_eq!(
            classify_status("Phase complete - ready for verification"),
            StatusCategory::Idle
        );
        // Blocked keeps precedence over awaiting-verification.
        assert_eq!(
            classify_status("blocked — ready for verification"),
            StatusCategory::Blocked
        );
    }

    #[test]
    fn test_classify_status_ready_to_execute_is_idle() {
        assert_eq!(classify_status("Ready to execute"), StatusCategory::Idle);
    }

    #[test]
    fn test_format_phase_display_prefers_current_phase_name() {
        let state = ProjectState {
            current_phase_name: "Live State".to_string(),
            completed_phases: 2,
            total_phases: 4,
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "Live State");
    }

    #[test]
    fn a_drop_report_does_not_move_a_runs_state() {
        use crate::executor::{ExecutionEvent, RunState};
        use crate::main_loop::ExecEvent;

        let mut app = App::new_for_test();
        app.ctx
            .run_states
            .insert("busy".to_string(), RunState::Running);

        app.apply_exec_event(ExecEvent::new(
            "busy",
            ExecutionEvent::EventsDropped { count: 40 },
        ));

        assert_eq!(
            app.ctx.run_states.get("busy"),
            Some(&RunState::Running),
            "a drop report is a diagnostic; it must not disturb a state the \
             stream already established"
        );

        // The load-bearing half. The catch-all arm promotes an `Idle` alias to
        // `Starting` on the reasoning that any observed line means the process
        // is alive and talking — but this report is emitted in the terminal
        // path, *after* the process stopped producing. If this assertion ever
        // reads `Starting`, the catch-all has reclaimed the variant and a
        // diagnostic is manufacturing liveness again (D-19, D-33).
        app.apply_exec_event(ExecEvent::new(
            "quiet",
            ExecutionEvent::EventsDropped { count: 40 },
        ));

        assert_eq!(
            app.ctx.run_states.get("quiet"),
            Some(&RunState::Idle),
            "a drop report must not promote an idle alias to Starting"
        );
    }

    // ── OBS-06: counting the re-parses ───────────────────────────────────
    //
    // The seam is a plain `u64` on `AppContext`, bumped in `schedule_reparse`.
    // A `static AtomicUsize` inside `parse_project_state` would be more literal
    // and is deliberately REJECTED: cargo runs a crate's tests as threads
    // inside one process, so every other test that constructs project state
    // would increment the same static concurrently, and the failure would be
    // intermittent. Per-`App` state is the right scope — please do not
    // "improve" this seam back into a flaky one (RESEARCH §7.4).
    //
    // Deliberately NOT written: any wall-clock timing assertion. `main_loop.rs`
    // already records the reasoning for the same repository (its tests module,
    // "it passes on a developer laptop, fails on a loaded runner, gets
    // `#[ignore]`d within a month, and at that point the requirement has no
    // verification at all"). D-17 permits a generous comparative timing number
    // as corroboration, but the counter is the load-bearing evidence and a
    // timing number must never be the only evidence. Read this absence as a
    // choice.
    //
    // `#[tokio::test]` is required throughout: both schedulers call
    // `spawn_blocking`, which panics without a runtime.

    const OBS_ALIAS: &str = "proj";
    const OBS_RUN: &str = "2026-07-29T09-00-00Z-a3f9";

    /// The two liveness answers these tests build fixtures from.
    ///
    /// Named locally so the assertions below read as "a live run" and "a crashed
    /// run" rather than as a path repeated forty times, and so the third answer
    /// — `Liveness::Unknown`, which no `App`-level assertion here depends on — is
    /// conspicuously absent rather than silently unused.
    const ALIVE: crate::driver::liveness::Liveness = crate::driver::liveness::Liveness::Alive;
    const DEAD: crate::driver::liveness::Liveness = crate::driver::liveness::Liveness::Dead;

    /// One `App` with one registered project at `root`, and a live `event_tx`.
    ///
    /// Every OBS-06 test below builds through this one helper, so a handler
    /// change cannot satisfy the zero-arm by breaking the control arm.
    /// `App::new_for_test` leaves `event_tx` as `None`, and `schedule_reparse`
    /// returns early without a sender — so without this the counter would read
    /// zero for the wrong reason.
    fn obs_app(root: &std::path::Path) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        use crate::config::RegisteredProject;

        let mut app = App::new_for_test();
        app.ctx.config.projects.insert(
            OBS_ALIAS.to_string(),
            RegisteredProject {
                path: root.to_path_buf(),
                added: "2026-07-29".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        app.ctx.event_tx = Some(tx);
        (app, rx)
    }

    /// A scan that observed `runs` and found no ended run anywhere.
    ///
    /// The `last_outcomes` half is exercised by its own tests below; every
    /// assertion about `observed_runs` wants it empty, and spelling that out at
    /// seven call sites would say nothing seven times.
    fn reconciled(runs: Vec<crate::driver::reconcile::ObservedRun>) -> Action {
        Action::RunsReconciled {
            runs,
            last_outcomes: HashMap::new(),
            launcher_markers: HashMap::new(),
        }
    }

    fn journal_change(root: &std::path::Path) -> Action {
        Action::FileChanged {
            project_path: root.to_path_buf(),
            changed_path: root.join(format!(
                ".planning/meta-manager/runs/{OBS_RUN}/journal.jsonl"
            )),
        }
    }

    fn planning_change(root: &std::path::Path) -> Action {
        Action::FileChanged {
            project_path: root.to_path_buf(),
            changed_path: root.join(".planning/STATE.md"),
        }
    }

    #[tokio::test]
    async fn journal_appends_never_trigger_a_full_reparse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        let before = app.ctx.reparse_dispatches;
        for _ in 0..500 {
            app.update(journal_change(root));
        }

        assert_eq!(
            app.ctx.reparse_dispatches,
            before,
            "OBS-06: five hundred journal appends leaked {} full re-parses",
            app.ctx.reparse_dispatches - before
        );
    }

    // The control arm, and it is NOT optional. A test that only asserts zero
    // would pass against a handler that dropped `FileChanged` entirely — which
    // would break the dashboard outright while satisfying OBS-06 (RESEARCH
    // §7.3).
    #[tokio::test]
    async fn a_planning_write_still_triggers_a_reparse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        let before = app.ctx.reparse_dispatches;
        app.update(planning_change(root));

        assert_eq!(
            app.ctx.reparse_dispatches,
            before + 1,
            "a planning write must still schedule exactly one re-parse, got {}",
            app.ctx.reparse_dispatches
        );
    }

    #[tokio::test]
    async fn the_driver_route_leaves_the_refresh_dedup_map_untouched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        for _ in 0..10 {
            app.update(journal_change(root));
        }
        assert!(
            app.ctx.last_refresh.is_empty(),
            "D-14: the driver route must not touch the 500ms dedup map, or a \
             journal append could suppress a genuine state refresh"
        );

        // And the planning route still records into it, so the assertion above
        // is not passing because the map went unused everywhere.
        app.update(planning_change(root));
        assert!(app.ctx.last_refresh.contains_key(OBS_ALIAS));
    }

    /// A `JournalCursor` with both of its fields varied, so an assertion cannot
    /// pass by comparing only the byte offset.
    fn cursor_at(offset: u64, last_seq: u64) -> crate::journal::reader::JournalCursor {
        crate::journal::reader::JournalCursor {
            cursor: crate::journal::reader::TailCursor { offset },
            last_seq,
        }
    }

    /// A `RunSummary` with only the field these assertions read varied.
    fn run_summary(run_id: &str) -> crate::journal::RunSummary {
        crate::journal::RunSummary {
            run_id: run_id.to_string(),
            started_at: "2026-07-29T12:02:00Z".to_string(),
            ended_at: None,
            goal: "ship it".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            outcome: None,
        }
    }

    /// An `ObservedRun` with only the fields these assertions read varied.
    fn observed(
        alias: &str,
        run_id: &str,
        liveness: crate::driver::liveness::Liveness,
    ) -> crate::driver::reconcile::ObservedRun {
        crate::driver::reconcile::ObservedRun {
            alias: alias.to_string(),
            run_id: run_id.to_string(),
            pid: 4242,
            pgid: 4242,
            started_at: "2026-07-29T09:00:00Z".to_string(),
            goal: "ship it".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            liveness,
        }
    }

    #[tokio::test]
    async fn a_reconciliation_result_replaces_the_observed_map_rather_than_merging_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // A run the previous scan saw, whose project has since been
        // unregistered — or whose run has since ended. Either way the new scan
        // expresses that by NOT returning it, and a merge would keep it forever.
        app.ctx
            .observed_runs
            .insert("gone".to_string(), observed("gone", "run-old", ALIVE));

        app.update(reconciled(vec![observed(OBS_ALIAS, "run-new", ALIVE)]));

        assert_eq!(
            app.ctx.observed_runs.len(),
            1,
            "the scan is authoritative: a stale entry must be dropped, not merged"
        );
        assert!(!app.ctx.observed_runs.contains_key("gone"));
        assert_eq!(app.ctx.observed_runs[OBS_ALIAS].run_id, "run-new");

        // An empty scan clears the map outright, which is how "every run ended"
        // is expressed.
        app.update(reconciled(Vec::new()));
        assert!(app.ctx.observed_runs.is_empty());
    }

    #[tokio::test]
    async fn an_unchanged_reconciliation_result_does_not_request_a_redraw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(reconciled(vec![observed(OBS_ALIAS, "run-a", ALIVE)]));

        // The scan lands every ~5s for the whole life of the process. Without
        // the equality guard this would repaint twelve times a minute forever
        // with nothing on screen changed.
        app.needs_redraw = false;
        app.update(reconciled(vec![observed(OBS_ALIAS, "run-a", ALIVE)]));
        assert!(
            !app.needs_redraw,
            "an identical scan result must not request a redraw"
        );

        // The control arm: a real change still does.
        app.update(reconciled(vec![observed(OBS_ALIAS, "run-a", DEAD)]));
        assert!(
            app.needs_redraw,
            "a run going from live to crashed must request a redraw, or the guard \
             above is suppressing genuine updates too"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_driver_run_is_refused_when_the_live_count_already_meets_the_cap() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        assert_eq!(app.ctx.config.preferences.driver_max_concurrent, 1);
        app.ctx
            .observed_runs
            .insert("busy".to_string(), observed("busy", "run-live", ALIVE));

        app.start_driver_run(OBS_ALIAS, "/gsd-progress", None);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("the refusal must be visible to the user, not silent");
        assert!(
            refusal.contains("driver_max_concurrent"),
            "the refusal must name the setting that governs it, got: {refusal}"
        );
        assert!(
            !app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "a refused spawn must add no optimistic entry"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_driver_run_against_an_unknown_alias_is_refused_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.start_driver_run("nosuchalias", "/gsd-progress", None);

        let refusal = app.ctx.error_message.as_deref().expect("a visible refusal");
        assert!(refusal.contains("nosuchalias"), "got: {refusal}");
        assert!(app.ctx.observed_runs.is_empty());
    }

    /// A crashed run does NOT count against the cap.
    ///
    /// It is still surfaced — a run that died without an ending is exactly what
    /// a user needs to be told about — but it consumes no quota, so counting it
    /// would leave a project permanently unstartable after one crash, with no
    /// way out but editing `config.json`.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_crashed_run_does_not_consume_a_concurrency_slot() {
        use crate::driver::spawn::admit;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .observed_runs
            .insert("dead".to_string(), observed("dead", "run-crashed", DEAD));

        let live = app
            .ctx
            .observed_runs
            .values()
            .filter(|run| run.is_live())
            .count();
        assert_eq!(live, 0);
        assert!(admit(live, app.ctx.config.preferences.driver_max_concurrent).is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stopping_an_alias_with_no_observed_run_is_refused_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.stop_driver_run(OBS_ALIAS);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a stop with nothing to stop must say so, not fail silently");
        assert!(refusal.contains(OBS_ALIAS), "got: {refusal}");
    }

    /// A run the scan found but this session did not start takes D-07's
    /// **adopted** arm.
    ///
    /// The distinction cannot be read off disk — `run.json` records the driver's
    /// own pid and never who its parent was — so it lives in a set that survives
    /// the scan replacing `observed_runs` wholesale every five seconds. This
    /// asserts the set stays empty for a run that arrived through a scan, which
    /// is what makes the arm `Adopted` and the `/proc` re-probe the only way to
    /// confirm death.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_this_session_did_not_spawn_is_never_recorded_as_our_child() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(reconciled(vec![observed(OBS_ALIAS, "run-adopted", ALIVE)]));

        assert!(
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "the scan's own entry must be there, or the assertion below is vacuous"
        );
        assert!(
            app.ctx.session_spawned_runs.is_empty(),
            "a run rediscovered by the scan was reparented to init when its \
             original parent exited; claiming it as our child would make a stop \
             wait on a `wait()` that returns ECHILD (D-07)"
        );
    }

    /// The stop is dispatched off the render thread and its result comes back as
    /// an `Action`.
    ///
    /// The twelve-second grace is why this shape is not optional: a stop that
    /// blocked the loop would freeze the frame for twelve seconds while stopping
    /// a run, which looks exactly like the hang the stop exists to end
    /// (TRANS-03). The pid here belongs to no driver, so the dispatched task
    /// takes the `AlreadyGone` path and returns immediately — what is being
    /// asserted is the seam, not the teardown.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_returns_its_outcome_as_an_action_rather_than_blocking() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));

        app.stop_driver_run(OBS_ALIAS);
        assert!(
            app.ctx.error_message.is_none(),
            "a stop against an observed run is not a refusal"
        );

        let action = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("the stop must report back promptly, not block the loop")
            .expect("the channel is open");

        let Action::DriverStopped {
            alias,
            run_id,
            outcome,
            disposition,
        } = action
        else {
            panic!("the dispatched stop must report through DriverStopped, got {action:?}");
        };
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(run_id, "run-x");
        assert!(!outcome.is_empty(), "the outcome must render as something");
        // The pid belongs to no driver, so the dispatched task took the
        // `AlreadyGone` path — which establishes nothing about whether a run
        // ended, and therefore reports `MayStillBeLive` (WR-15/D-29).
        assert_eq!(disposition, crate::action::StopDisposition::MayStillBeLive);

        // And on a report that the run really IS gone, the handler drops the
        // entry rather than editing it, so the dashboard does not show a
        // stopped run as live until the next scan. The disposition is supplied
        // by hand rather than reused from the dispatch above, because that
        // dispatch legitimately produced the other one — reusing it would make
        // this assertion silently test nothing.
        app.update(Action::DriverStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-x".to_string(),
            outcome,
            disposition: crate::action::StopDisposition::RunGone,
        });
        assert!(!app.ctx.observed_runs.contains_key(OBS_ALIAS));
        assert!(!app.ctx.session_spawned_runs.contains("run-x"));
    }

    // ── WR-15 / D-29: a stop that stopped nothing must forget nothing ────
    //
    // One test per `StopOutcome` variant, driven through the real
    // `StopDisposition::from` conversion rather than through a hand-picked
    // disposition — otherwise the test would assert the handler's behaviour
    // for a mapping the send site might not actually produce, and the two
    // halves of the fix could drift apart silently.
    //
    // **The load-bearing half of each is the second assertion**, following the
    // discipline `driver_confirm.rs:367-372` records: an implementation that
    // set the status message and mutated the maps anyway would pass a test that
    // only checked the message, and the user would be shown the truth beside a
    // dashboard that had already forgotten the run.

    /// One `App` holding a live observed run this session spawned, ready to
    /// receive a stop report for it.
    #[cfg(unix)]
    fn app_with_a_spawned_run(
        root: &std::path::Path,
    ) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (mut app, rx) = obs_app(root);
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));
        app.ctx.session_spawned_runs.insert("run-x".to_string());
        (app, rx)
    }

    /// Apply a stop report derived from `outcome` and return the two facts the
    /// assertions below care about: whether each map kept its entry.
    #[cfg(unix)]
    fn report_stop(
        app: &mut App,
        outcome: crate::driver::kill::StopOutcome,
    ) -> (bool, bool, Option<String>) {
        app.update(Action::DriverStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-x".to_string(),
            disposition: crate::action::StopDisposition::from(&outcome),
            outcome: outcome.to_string(),
        });
        (
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            app.ctx.session_spawned_runs.contains("run-x"),
            app.ctx
                .status_message
                .as_ref()
                .map(|(text, _)| text.clone()),
        )
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_that_exited_on_terminate_is_dropped_from_both_maps() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::ExitedOnTerminate);

        assert!(
            !observed_kept,
            "the driver was observed gone, so the dashboard must not keep showing \
             its run as live until the next scan"
        );
        assert!(
            !spawned_kept,
            "a run that is gone needs no reaping arm, so its session record goes too"
        );
        let message = message.expect("the outcome must still reach the status line");
        assert!(message.contains(OBS_ALIAS), "got: {message}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_that_exited_after_the_uncatchable_signal_is_dropped_from_both_maps() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::ExitedAfterKill);

        assert!(
            !observed_kept,
            "an escalated stop is still a stop that was observed to work"
        );
        assert!(!spawned_kept);
        let message = message.expect("the outcome must still reach the status line");
        assert!(
            message.contains("orphaned"),
            "an escalation must still tell the user its `claude` group may have \
             been orphaned, got: {message}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_that_signalled_nothing_keeps_both_map_entries() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::AlreadyGone);

        // The user is told, either way. That half was never broken.
        let message = message.expect("the outcome must reach the status line");
        assert!(message.contains(OBS_ALIAS), "got: {message}");

        // The half that was. `AlreadyGone` means NOTHING WAS SIGNALLED — the
        // probe simply did not recognise the pid as this run's driver — so it
        // establishes nothing about whether the run ended.
        assert!(
            observed_kept,
            "WR-15: dropping the observed run here shows 'no run' for up to five \
             seconds beside a status line saying nothing was signalled"
        );
        assert!(
            spawned_kept,
            "WR-15: this entry is never re-inserted — the disk cannot say who a \
             driver's parent was — so dropping it makes a later stop take the \
             `Adopted` arm for a run this session did spawn"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_whose_signal_was_never_delivered_keeps_both_map_entries() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) = report_stop(
            &mut app,
            StopOutcome::SignalFailed {
                detail: "PermissionDenied".to_string(),
            },
        );

        let message = message.expect("the outcome must reach the status line");
        assert!(
            message.contains("could not be delivered"),
            "the user must be told the signal never landed, got: {message}"
        );

        // This is the arm the Driver tab makes impossible to miss: a "no run"
        // pane beside a status line saying the signal could not be delivered.
        assert!(
            observed_kept,
            "WR-15: the signal was never delivered, so the run is most likely \
             still driving the user's repository"
        );
        assert!(spawned_kept);
    }

    // ── D-20: the seam, the gap diagnostic, the tick gate, the prune ─────

    /// One journal record of `kind`, carrying `fields` as its payload.
    fn journal_record(
        seq: u64,
        kind: &str,
        fields: &[(&str, serde_json::Value)],
    ) -> crate::journal::reader::JournalRecord {
        let mut rest = serde_json::Map::new();
        for (key, value) in fields {
            rest.insert((*key).to_string(), value.clone());
        }
        crate::journal::reader::JournalRecord {
            ts: "2026-07-29T12:00:00Z".to_string(),
            seq,
            kind: kind.to_string(),
            rest,
        }
    }

    /// An `exec_event` on `stream` carrying `text`.
    fn exec_event(seq: u64, stream: &str, text: &str) -> crate::journal::reader::JournalRecord {
        journal_record(
            seq,
            "exec_event",
            &[("stream", stream.into()), ("text", text.into())],
        )
    }

    /// Every buffered line for `alias`, as `(kind, text)` pairs.
    fn buffered(app: &App, alias: &str) -> Vec<(crate::ui::screens::DriverLineKind, String)> {
        app.ctx
            .driver_output
            .get(alias)
            .map(|output| {
                output
                    .lines()
                    // `as_raw_for_logic_only`: these assertions ask what the
                    // buffer STORED, which is the append-time question. `shown()`
                    // would answer the render-time one and make every expected
                    // string in this module a post-escape spelling.
                    .map(|line| (line.kind, line.text.as_raw_for_logic_only().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The header's two record-borne facts — the cumulative cost and the turn
    /// count — are read from the tail as it arrives, and both are evidence:
    /// `cost.cumulative_usd` and an `exec_event` on the `turn_completed` stream.
    #[tokio::test]
    async fn a_journal_batch_tallies_the_cumulative_cost_and_the_turn_boundaries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![
                exec_event(1, "assistant", "working"),
                exec_event(2, "turn_completed", "turn ended: success"),
                journal_record(3, "cost", &[("cumulative_usd", 0.42.into())]),
                exec_event(4, "turn_completed", "turn ended: success"),
                journal_record(5, "cost", &[("cumulative_usd", 1.83.into())]),
            ],
            cursor: cursor_at(120, 5),
        });

        let tally = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .and_then(|cache| cache.driver_tally.clone())
            .expect("a tally after a batch");
        assert_eq!(tally.run_id, OBS_RUN);
        // The LAST cumulative value wins; it is a running total, not a delta.
        assert_eq!(tally.cumulative_cost_usd, Some(1.83));
        assert_eq!(tally.turn_boundaries, 2);

        // A batch from a DIFFERENT run resets the tally rather than adding to
        // it — one run's cost shown against another's is a figure the evidence
        // does not support.
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: "2026-07-29T22-00-00Z-beef".to_string(),
            records: vec![exec_event(1, "assistant", "a different run")],
            cursor: cursor_at(20, 1),
        });
        let tally = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .and_then(|cache| cache.driver_tally.clone())
            .expect("a tally after the second batch");
        assert_eq!(tally.run_id, "2026-07-29T22-00-00Z-beef");
        assert_eq!(
            tally.cumulative_cost_usd, None,
            "the previous run's cost must not carry over"
        );
        assert_eq!(tally.turn_boundaries, 0);
    }

    /// CR-02, at the handler that fills the ring.
    ///
    /// `driver_output` is keyed by **alias**, so one buffer serves every run a
    /// project ever has. Without a retarget before the first push of each batch,
    /// run A's lines — its `run ended:` terminal record above all, which the
    /// pane deliberately renders LAST and in the *currently selected* run's
    /// terminal colour — appear under run B's header as run B's own visual full
    /// stop. That is the display disagreeing with the disk in the direction that
    /// flatters the run.
    #[tokio::test]
    async fn a_batch_from_a_new_run_clears_the_previous_runs_lines_from_the_ring() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![
                exec_event(1, "assistant", "run A was working"),
                journal_record(2, "run_ended", &[("outcome", "succeeded".into())]),
            ],
            cursor: cursor_at(120, 2),
        });
        assert_eq!(buffered(&app, OBS_ALIAS).len(), 2);

        const NEXT: &str = "2026-07-29T22-00-00Z-beef";
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: NEXT.to_string(),
            records: vec![exec_event(1, "assistant", "run B is working")],
            cursor: cursor_at(20, 1),
        });

        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(
            lines,
            vec![(
                crate::ui::screens::DriverLineKind::Output,
                "run B is working".to_string()
            )],
            "run B's ring holds run B's lines and nothing else; a surviving \
             `run ended: succeeded` would render as run B's own ending"
        );
        assert_eq!(
            app.ctx
                .driver_output
                .get(OBS_ALIAS)
                .map(|output| output.run_id()),
            Some(NEXT),
            "and the buffer says which run it now speaks for"
        );
    }

    #[tokio::test]
    async fn a_journal_batch_reaches_the_ring_buffer_and_requests_a_redraw() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.needs_redraw = false;
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![
                exec_event(1, "assistant", "reading src/driver/run.rs"),
                exec_event(2, "stderr", "a warning from the tool"),
                journal_record(3, "run_ended", &[("outcome", "succeeded".into())]),
            ],
            cursor: cursor_at(120, 3),
        });

        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(
            lines.len(),
            3,
            "every record with an output-pane class must be buffered, got: {lines:?}"
        );
        assert_eq!(
            lines[0],
            (
                DriverLineKind::Output,
                "reading src/driver/run.rs".to_string()
            )
        );
        assert_eq!(
            lines[1].0,
            DriverLineKind::Stderr,
            "stderr is its own class"
        );
        assert_eq!(
            lines[2].0,
            DriverLineKind::Terminal,
            "the ending is the visual full stop"
        );
        assert!(
            lines[2].1.contains("succeeded"),
            "the terminal line carries the derived outcome, got: {}",
            lines[2].1
        );

        assert!(
            app.needs_redraw,
            "D-20: the handler that adds the surface adds the flag with it — \
             without this the Driver tab renders a frame that can never change"
        );

        // And the omissions the rewritten comment claims are still real.
        assert!(
            app.ctx.last_refresh.is_empty(),
            "the driver route must still not touch the 500ms dedup map (D-14)"
        );
        assert_eq!(
            app.ctx.reparse_dispatches, 0,
            "a journal append must still schedule no full re-parse (OBS-06)"
        );
    }

    #[tokio::test]
    async fn a_sequence_gap_becomes_a_line_the_user_can_see() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Seed a cursor at seq 4, then deliver a batch starting at 9. Five
        // records the writer emitted were never read, and the pane cannot
        // recover them — so it has to say so.
        let key = (OBS_ALIAS.to_string(), OBS_RUN.to_string());
        app.ctx.journal_cursors.insert(key, cursor_at(10, 4));

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![exec_event(9, "assistant", "still working")],
            cursor: cursor_at(80, 9),
        });

        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(lines.len(), 2, "the gap line plus the record: {lines:?}");
        assert_eq!(
            lines[0].0,
            DriverLineKind::Diagnostic,
            "a gap is a diagnostic, never silently swallowed"
        );
        assert!(
            lines[0].1.contains("journal gap") && lines[0].1.contains('1'),
            "the diagnostic must name the count, got: {}",
            lines[0].1
        );
        assert_eq!(
            lines[1].0,
            DriverLineKind::Output,
            "the gap goes ABOVE the records that did arrive, because it is what \
             came before them"
        );

        // The control arm: a contiguous batch adds no diagnostic, so the
        // assertion above is not passing because every batch gets one.
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![exec_event(10, "assistant", "carrying on")],
            cursor: cursor_at(120, 10),
        });
        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(lines.len(), 3, "no second gap line: {lines:?}");
    }

    /// The record → line projection, across every class the pane renders.
    #[test]
    fn every_diagnostic_bearing_record_kind_reaches_the_pane() {
        use crate::ui::screens::DriverLineKind;

        let cases: Vec<(crate::journal::reader::JournalRecord, DriverLineKind, &str)> = vec![
            (
                journal_record(1, "events_dropped", &[("count", 40.into())]),
                DriverLineKind::Diagnostic,
                "40 stream events were dropped",
            ),
            (
                journal_record(
                    2,
                    "journal_truncated",
                    &[("bytes_written", 1024.into()), ("cap", 2048.into())],
                ),
                DriverLineKind::Diagnostic,
                "2048",
            ),
            (
                journal_record(
                    3,
                    "diagnostic",
                    &[("code", "seq_gap".into()), ("detail", "one record".into())],
                ),
                DriverLineKind::Diagnostic,
                "seq_gap",
            ),
            (
                journal_record(
                    4,
                    "interjected",
                    &[("id", "3f2a".into()), ("text", "skip the UI review".into())],
                ),
                DriverLineKind::Injection,
                "skip the UI review",
            ),
            (
                journal_record(5, "interjection_acted_on", &[("id", "3f2a".into())]),
                DriverLineKind::Injection,
                "3f2a",
            ),
            (
                journal_record(
                    6,
                    "interjection_missed",
                    &[("id", "3f2a".into()), ("reason", "stdin closed".into())],
                ),
                DriverLineKind::Injection,
                "stdin closed",
            ),
            (
                journal_record(
                    7,
                    "exec_finished",
                    &[("exit", 0.into()), ("duration_s", 12.into())],
                ),
                DriverLineKind::Terminal,
                "exit 0",
            ),
        ];

        for (record, expected_kind, expected_substring) in cases {
            let (kind, text) = driver_line_for_record(&record)
                .unwrap_or_else(|| panic!("`{}` must reach the pane", record.kind));
            assert_eq!(kind, expected_kind, "wrong class for `{}`", record.kind);
            assert!(
                text.contains(expected_substring),
                "`{}` rendered as {text:?}, which does not name {expected_substring:?}",
                record.kind
            );
            assert!(
                !text.contains('{'),
                "`{}` must compose a real string, never a Debug rendering: {text:?}",
                record.kind
            );
        }

        // The header's own data is deliberately NOT buffered — it renders above
        // the pane, and duplicating it would make the first rows a copy of the
        // header. This is not "losing lines"; the assertions above are what
        // guarantee that every diagnostic-bearing kind does arrive.
        for kind in ["run_started", "exec_started", "cost", "parked"] {
            assert!(
                driver_line_for_record(&journal_record(1, kind, &[])).is_none(),
                "`{kind}` belongs to the header or to a later phase's timeline"
            );
        }
    }

    /// D-21's truth table: three conditions, and all three are required.
    #[test]
    fn the_elapsed_redraw_gate_is_true_for_exactly_one_combination() {
        use crate::ui::screens::detail::DetailScreen;

        let live = observed(OBS_ALIAS, "run-x", ALIVE);
        let dead = observed(OBS_ALIAS, "run-x", DEAD);

        // The one combination that repaints.
        assert!(
            driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Driver),
                Some(&live)
            ),
            "a live run on the Driver tab is the case the counter exists for"
        );

        // The dashboard. This is the one that matters most: an idle fleet
        // dashboard repainting four times a second forever is the exact
        // opposite of this tool's pitch.
        assert!(
            !driver_elapsed_redraw_wanted("normal", Some(&DetailSubView::Driver), Some(&live)),
            "the counter is not on screen from the dashboard"
        );

        // A detail view on another tab. The counter is not rendered there
        // either.
        assert!(
            !driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Pipeline),
                Some(&live)
            ),
            "no other tab shows an elapsed counter"
        );

        // The Driver tab with nothing running. A finished run's elapsed figure
        // is frozen, so a frame would be identical to the one already up.
        assert!(
            !driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Driver),
                Some(&dead)
            ),
            "a run that is not live has a frozen elapsed figure"
        );
        assert!(
            !driver_elapsed_redraw_wanted(DetailScreen::NAME, Some(&DetailSubView::Driver), None),
            "a project with no observed run at all has nothing to count"
        );
    }

    /// The gate wired through a real `Action::Tick`, not just the predicate.
    #[tokio::test]
    async fn a_tick_repaints_only_when_the_driver_tab_is_watching_a_live_run() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));

        // The dashboard is on top and no tab is selected: an idle tick.
        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(
            !app.needs_redraw,
            "an idle tick must not repaint — this is the CPU burn D-21 names"
        );

        // Now a detail view on the Driver tab, over the same live run.
        app.screen_stack
            .push(Box::new(crate::ui::screens::detail::DetailScreen::new(
                OBS_ALIAS.to_string(),
            )));
        app.ctx
            .detail_sub_view_per_project
            .insert(OBS_ALIAS.to_string(), DetailSubView::Driver);

        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(
            app.needs_redraw,
            "the elapsed counter must advance where it can actually be seen"
        );
    }

    // ── The Driver tab must not freeze while it is open (CR-04) ────────────

    /// An app whose Driver tab for `OBS_ALIAS` is the surface on screen.
    fn app_on_the_driver_tab(
        root: &std::path::Path,
    ) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (mut app, rx) = obs_app(root);
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));
        app.screen_stack
            .push(Box::new(crate::ui::screens::detail::DetailScreen::new(
                OBS_ALIAS.to_string(),
            )));
        app.ctx
            .detail_sub_view_per_project
            .insert(OBS_ALIAS.to_string(), DetailSubView::Driver);
        (app, rx)
    }

    /// Drain `rx` and report whether the run-list scan reported back.
    ///
    /// The scan runs on `spawn_blocking` and answers with
    /// `Action::DriverRunsListed`, so its arrival on the channel is the only
    /// observation that says the scan actually happened — as opposed to a
    /// counter this file incremented itself.
    async fn scan_reported(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) -> bool {
        for _ in 0..200 {
            while let Ok(action) = rx.try_recv() {
                if matches!(action, Action::DriverRunsListed { .. }) {
                    return true;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    /// The rescan gate is **visibility, not liveness**, and the difference is
    /// the whole finding.
    ///
    /// `reconcile_one` returns `None` for an ended run, so the alias drops out
    /// of `observed_runs` the instant the run finishes — which is exactly when
    /// the run list's glyph, its outcome and the journal snapshot go stale and
    /// most need re-reading. Gating the rescan on `driver_elapsed_redraw_wanted`
    /// would stop it firing at the moment it started mattering.
    #[test]
    fn the_rescan_gate_is_visibility_and_deliberately_not_liveness() {
        use crate::ui::screens::detail::DetailScreen;

        assert!(driver_tab_is_on_top(
            DetailScreen::NAME,
            Some(&DetailSubView::Driver)
        ));
        assert!(!driver_tab_is_on_top(
            "normal",
            Some(&DetailSubView::Driver)
        ));
        assert!(!driver_tab_is_on_top(
            DetailScreen::NAME,
            Some(&DetailSubView::Pipeline)
        ));
        assert!(!driver_tab_is_on_top(DetailScreen::NAME, None));

        // And a run that has ENDED still satisfies it, where the elapsed-redraw
        // gate does not.
        let dead = observed(OBS_ALIAS, "run-x", crate::driver::liveness::Liveness::Dead);
        assert!(!driver_elapsed_redraw_wanted(
            DetailScreen::NAME,
            Some(&DetailSubView::Driver),
            Some(&dead)
        ));
        assert!(driver_tab_is_on_top(
            DetailScreen::NAME,
            Some(&DetailSubView::Driver)
        ));
    }

    /// D-08: the heartbeat age is recomputed at render time, so a frame is wanted every
    /// tick while the Driver tab shows a live launcher run, and in no other case.
    #[test]
    fn the_launcher_heartbeat_gate_truth_table() {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, DerivedStatus, DriverRunView, RunStatus,
        };
        use crate::ui::screens::detail::DetailScreen;
        use crate::ui::screens::ProjectViewCache;

        let cache_with = |derived: DerivedStatus| {
            let frontmatter = parse_driver_frontmatter(include_str!(
                "../tests/fixtures/driverctl/direction/s003-run1b.md"
            ))
            .expect("the spike fixture parses");
            ProjectViewCache {
                launcher_runs: vec![(
                    "20261020-a".to_string(),
                    Ok(DriverRunView {
                        frontmatter,
                        derived,
                    }),
                )],
                launcher_listed: true,
                ..ProjectViewCache::default()
            }
        };
        let live = cache_with(DerivedStatus::AsWritten(RunStatus::Running));
        let starting = cache_with(DerivedStatus::AsWritten(RunStatus::Starting));
        let driver = Some(&DetailSubView::Driver);

        assert!(launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            driver,
            Some(&live)
        ));
        assert!(launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            driver,
            Some(&starting)
        ));
        // Not on the Driver tab, or not on the detail screen.
        assert!(!launcher_heartbeat_redraw_wanted(
            "normal",
            driver,
            Some(&live)
        ));
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            Some(&DetailSubView::Pipeline),
            Some(&live)
        ));
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            None,
            Some(&live)
        ));
        // A run that is not live has a frozen age: no frame.
        for derived in [
            DerivedStatus::Crashed {
                written: RunStatus::Running,
            },
            DerivedStatus::NotLaunched,
            DerivedStatus::LivenessUnknown {
                written: RunStatus::Running,
            },
            DerivedStatus::AsWritten(RunStatus::Done),
            DerivedStatus::AsWritten(RunStatus::Parked),
        ] {
            let cache = cache_with(derived.clone());
            assert!(
                !launcher_heartbeat_redraw_wanted(DetailScreen::NAME, driver, Some(&cache)),
                "{derived:?}"
            );
        }
        // No cache, an empty one, or a legacy row selected: nothing to age.
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            driver,
            None
        ));
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            driver,
            Some(&ProjectViewCache::default())
        ));
        let mut legacy_selected = cache_with(DerivedStatus::AsWritten(RunStatus::Running));
        legacy_selected.driver_runs = vec![crate::journal::RunSummary {
            run_id: "2026-07-29T21-40-00Z-3f2a".to_string(),
            started_at: "2026-07-29T21:40:00Z".to_string(),
            ended_at: None,
            goal: "g".to_string(),
            gsd_command: "/gsd:execute-phase 18".to_string(),
            outcome: None,
        }];
        legacy_selected.legacy_expanded = true;
        legacy_selected.legacy_row_selected = true;
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            driver,
            Some(&legacy_selected)
        ));
    }

    /// The tick turns the gate into a redraw request, and only for a live selected run.
    #[tokio::test]
    async fn launcher_view_a_tick_redraws_for_a_live_selected_run_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        // `write_items_run` writes a paused run: not live, so a tick draws nothing.
        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(!app.needs_redraw, "a paused run has a frozen age");

        // Make the selected row live and tick again.
        let cache = app.ctx.view_cache.get_mut(OBS_ALIAS).expect("the cache");
        if let Ok(view) = &mut cache.launcher_runs[0].1 {
            view.derived = crate::state_reader::driver_frontmatter::DerivedStatus::AsWritten(
                crate::state_reader::driver_frontmatter::RunStatus::Running,
            );
        }
        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(app.needs_redraw, "a live run's age advances every tick");
    }

    /// CR-04's first failure: the tab froze at the moment it was entered.
    ///
    /// `schedule_run_list_scan` had exactly two call sites — tab entry and
    /// selection move — so `driver_runs`, `driver_inbox` and `driver_journal`
    /// never changed while the user watched. A run that ended lost its output to
    /// a stale snapshot; a run that started stayed invisible.
    #[tokio::test]
    async fn the_driver_tab_rescans_on_the_existing_twenty_tick_block() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        // The 20-tick block, and no second timer: nothing fires before it.
        for _ in 0..19 {
            app.update(Action::Tick);
        }
        while let Ok(action) = rx.try_recv() {
            assert!(
                !matches!(action, Action::DriverRunsListed { .. }),
                "the rescan must ride the EXISTING counter — a scan before the \
                 twentieth tick means it got a timer of its own"
            );
        }

        app.update(Action::Tick);
        assert!(
            scan_reported(&mut rx).await,
            "the twentieth tick must rescan the run list, inbox and journal \
             while the Driver tab is open"
        );
    }

    /// Drain `rx` until the launcher listing reports back, and return its rows.
    async fn launcher_listing_reported(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) -> Option<Vec<crate::state_reader::driver_frontmatter::DriverRunEntry>> {
        for _ in 0..200 {
            while let Ok(action) = rx.try_recv() {
                if let Action::LauncherRunsRefreshed { launcher_runs, .. } = action {
                    return Some(launcher_runs);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        None
    }

    /// D-13: a lock released by SIGKILL emits no file event, so the slow tick is the
    /// only thing that can turn that run into `crashed` while the tab is open.
    #[tokio::test]
    async fn launcher_view_the_slow_tick_schedules_the_launcher_refresh() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        for _ in 0..19 {
            app.update(Action::Tick);
        }
        while let Ok(action) = rx.try_recv() {
            assert!(
                !matches!(action, Action::LauncherRunsRefreshed { .. }),
                "the launcher refresh must ride the existing 20-tick block"
            );
        }
        app.update(Action::Tick);
        let rows = launcher_listing_reported(&mut rx)
            .await
            .expect("the twentieth tick must re-list the launcher runs");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, LAUNCHER_RUN);
    }

    /// S4: no event channel means a visible refusal, never a silent return.
    #[tokio::test]
    async fn launcher_view_a_refresh_without_an_event_channel_refuses_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.event_tx = None;
        app.needs_redraw = false;
        app.ctx.schedule_launcher_refresh(OBS_ALIAS, dir.path());
        let message = app.ctx.error_message.clone().expect("a visible refusal");
        assert!(message.contains(OBS_ALIAS), "{message}");
    }

    fn run_file_change(root: &std::path::Path, file: &str) -> Action {
        Action::FileChanged {
            project_path: root.to_path_buf(),
            changed_path: root
                .join(".planning/meta-manager/drivers")
                .join(LAUNCHER_RUN)
                .join(file),
        }
    }

    /// D-24 / OBS-06: a launcher run's own files refresh the launcher list and never
    /// re-parse the project.
    #[tokio::test]
    async fn launcher_view_a_driver_run_event_refreshes_without_a_reparse() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        app.update(run_file_change(dir.path(), "DIRECTION.md"));

        let rows = launcher_listing_reported(&mut rx)
            .await
            .expect("an event under the run directory must re-list the launcher runs");
        assert_eq!(rows[0].0, LAUNCHER_RUN);
        assert_eq!(app.ctx.reparse_dispatches, 0, "no full project re-parse");
        assert!(
            !app.ctx.last_refresh.contains_key(OBS_ALIAS),
            "the driver route must not share the planning route's 500 ms dedup"
        );
    }

    /// The same event for a tab that is not open schedules no launcher listing, no
    /// run read and no re-parse. Since 30-03 it does schedule the one-project marker
    /// scan that keeps the dashboard badge current (D-13), and nothing else.
    #[tokio::test]
    async fn launcher_view_a_driver_run_event_with_the_tab_closed_schedules_only_the_marker_scan() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));

        app.update(run_file_change(dir.path(), "LOG.md"));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        while let Ok(action) = rx.try_recv() {
            assert!(
                matches!(action, Action::LauncherMarkerScanned { .. }),
                "the dashboard is on top: only the marker scan may follow, got {action:?}"
            );
        }
        assert_eq!(app.ctx.reparse_dispatches, 0);
    }

    /// The splice rewrites `state.json` on every heartbeat; none of it may cost a
    /// listing, let alone a re-parse (T-30-03).
    #[tokio::test]
    async fn launcher_view_a_state_json_event_schedules_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        for file in ["state.json", ".DIRECTION.md.fm.abc", "launcher.lock"] {
            app.update(run_file_change(dir.path(), file));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err(), "an Ignored path schedules no work");
        assert_eq!(app.ctx.reparse_dispatches, 0);
        assert!(app.ctx.last_refresh.is_empty());
    }

    /// Wait for the launcher listing the last event scheduled and feed it to the app,
    /// as the event loop does.
    async fn deliver_launcher_refresh(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) {
        loop {
            match next_action(rx).await {
                Some(
                    action @ (Action::LauncherRunsRefreshed { .. }
                    | Action::LauncherRunRefreshed { .. }),
                ) => {
                    app.update(action);
                    return;
                }
                Some(_) => {}
                None => panic!("the launcher listing never reported"),
            }
        }
    }

    /// SC2 end to end (D-04, D-09, D-11): a run whose frontmatter says `running`
    /// reads `crashed` while its run lock is free, and `running` once a lock holder
    /// exists. The frontmatter never changes between the two reads; only the lock
    /// does.
    #[cfg(unix)]
    #[tokio::test]
    async fn launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running() {
        use crate::driver::lock::{acquire_at, run_lock_path};
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, RunStatus, DRIVER_RUNS_REL,
        };

        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir.path().join(DRIVER_RUNS_REL).join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).expect("run dir");
        let mut frontmatter = parse_driver_frontmatter(include_str!(
            "../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        frontmatter.run_id = LAUNCHER_RUN.to_string();
        frontmatter.status = RunStatus::Running;
        frontmatter.status_reason = None;
        frontmatter.launches = 1;
        // A held lock must read `running`, not stale: the fixture's own heartbeat
        // stamps are fixed dates.
        crate::ui::screens::driver_run::refresh_heartbeats(&mut frontmatter);
        std::fs::write(
            run_dir.join("DIRECTION.md"),
            format!(
                "{}{}",
                frontmatter.emit(),
                crate::driver_templates::DIRECTION_TEMPLATE
            ),
        )
        .expect("DIRECTION.md");

        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.ctx.experimental = true;

        // 1. The lock is free: the file says running, the lock says nobody is.
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("\u{2717} crashed"), "{text}");
        assert!(!text.contains("\u{25C6} running"), "{text}");

        // 2. The lock is held by this test: the same bytes now read running.
        let _lock = acquire_at(&run_lock_path(&run_dir), LAUNCHER_RUN, std::process::id())
            .expect("the test takes the run lock");
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("\u{25C6} running"), "{text}");
        assert!(!text.contains("\u{2717} crashed"), "{text}");
        assert_eq!(app.ctx.reparse_dispatches, 0, "no project re-parse");
    }

    /// A run dir whose body lists I-01 and I-02 and whose frontmatter records I-01 as
    /// parked and I-09 as done (an item the body no longer lists).
    fn write_items_run(root: &std::path::Path, extra_body: &str) {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, ItemState, ItemStateEntry, ItemStates, RunStatus,
            DRIVER_RUNS_REL,
        };
        let run_dir = root.join(DRIVER_RUNS_REL).join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).expect("run dir");
        let mut frontmatter = parse_driver_frontmatter(include_str!(
            "../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        frontmatter.run_id = LAUNCHER_RUN.to_string();
        frontmatter.status = RunStatus::Paused;
        frontmatter.item_states = ItemStates(vec![
            (
                "I-01".to_string(),
                ItemStateEntry {
                    state: ItemState::Parked,
                    dispatches: 2,
                    outcome: Some("needs a socket path".to_string()),
                },
            ),
            (
                "I-09".to_string(),
                ItemStateEntry {
                    state: ItemState::Done,
                    dispatches: 1,
                    outcome: None,
                },
            ),
        ]);
        let body = format!(
            "\n# Driver direction: x\n\n## Direction\n\n- control: run\n\n## Items\n\n\
             ### I-01 first thing\n\n- hold: false\n\n### I-02 second thing\n\n- hold: false\n{extra_body}"
        );
        std::fs::write(
            run_dir.join("DIRECTION.md"),
            format!("{}{body}", frontmatter.emit()),
        )
        .expect("DIRECTION.md");
    }

    /// D-06 end to end: a body with I-01 and I-02 and a frontmatter with I-01 parked
    /// and I-09 done give one merged list; the cursor starts on the parked item; and a
    /// later edit to DIRECTION.md re-reads only the selected run.
    #[tokio::test]
    async fn launcher_view_items_merge_body_and_frontmatter() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.ctx.experimental = true;

        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        let row = |needle: &str| {
            text.lines()
                .find(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("no row contains {needle:?}:\n{text}"))
                .to_string()
        };
        assert!(
            row("I-01 \u{2691} parked").contains("> I-01"),
            "the cursor starts on the parked item:\n{text}"
        );
        assert!(row("I-02 \u{B7} unseen").contains("second thing"), "{text}");
        let nine = row("I-09 \u{25CF} done");
        assert!(nine.contains("[not in DIRECTION]"), "{nine}");
        assert!(row("I-01 parked").contains("needs a socket path"), "{text}");
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .launcher_item_cursor
                .as_deref(),
            Some("I-01")
        );

        // The next event names the SELECTED run: one run is re-read, no re-listing.
        write_items_run(dir.path(), "\n### I-03 third thing\n\n- hold: true\n");
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        let refreshed = next_action(&mut rx).await;
        match &refreshed {
            Some(Action::LauncherRunRefreshed { run_id, detail, .. }) => {
                assert_eq!(run_id, LAUNCHER_RUN);
                assert_eq!(detail.as_ref().expect("the run reads").items.len(), 3);
            }
            other => panic!("expected the single-run refresh, got {other:?}"),
        }
        app.update(refreshed.expect("checked above"));
        let text = render_top_screen(&app);
        let three = text
            .lines()
            .find(|l| l.contains("I-03"))
            .expect("the new item");
        assert!(
            three.contains("hold") && three.contains("third thing"),
            "{text}"
        );
        assert_eq!(app.ctx.reparse_dispatches, 0, "no project re-parse");
    }

    /// D-23 end to end: appended LOG.md bytes arrive through the refresh actions, only
    /// the new bytes are read, and a read whose `from_offset` is not the tail's cursor
    /// is discarded when it is delivered as an action.
    #[tokio::test]
    async fn launcher_log_appends_arrive_once_through_the_refresh_actions() {
        use std::io::Write as _;
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let log_path = dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(LAUNCHER_RUN)
            .join("LOG.md");
        let preamble = crate::driver_templates::LOG_TEMPLATE
            .replace(crate::driver_templates::RUN_ID_TOKEN, LAUNCHER_RUN);
        let event = |n: u32| format!("- 2026-10-20T14:02:1{n}Z | I-01 | tick | n={n}\n");
        std::fs::write(&log_path, format!("{preamble}{}{}", event(1), event(2))).unwrap();

        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(run_file_change(dir.path(), "LOG.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let log_lines = |app: &App| app.ctx.view_cache[OBS_ALIAS].launcher_log.ring.len();
        assert_eq!(log_lines(&app), 2, "the preamble is not an event");
        let first_end = app.ctx.view_cache[OBS_ALIAS].launcher_log.cursor.offset;

        // One more event, delivered by the single-run refresh a LOG.md event causes.
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .unwrap()
            .write_all(event(3).as_bytes())
            .unwrap();
        app.update(run_file_change(dir.path(), "LOG.md"));
        let refreshed = next_action(&mut rx).await.expect("a single-run refresh");
        let Action::LauncherRunRefreshed {
            log: Some(read), ..
        } = &refreshed
        else {
            panic!("expected the single-run refresh with a log read, got {refreshed:?}");
        };
        assert_eq!(
            read.from_offset, first_end,
            "it started where the tail stood"
        );
        assert_eq!(read.lines.len(), 1, "only the new bytes were read");
        let duplicate = refreshed.clone();
        app.update(refreshed);
        assert_eq!(log_lines(&app), 3);

        // The same read delivered again started at the OLD cursor: discarded.
        app.update(duplicate);
        assert_eq!(log_lines(&app), 3, "no line is shown twice");
    }

    /// A single-run refresh for a run that is not the selected one is dropped, and a
    /// failed read is answered by a re-listing.
    #[tokio::test]
    async fn launcher_view_a_single_run_result_for_another_run_is_not_stored() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let stored = app.ctx.view_cache[OBS_ALIAS]
            .launcher_detail
            .clone()
            .expect("the selected run's detail was stored");

        let mut other = (*stored).clone();
        other.run_id = "20261018-other".to_string();
        let row_before = app.ctx.view_cache[OBS_ALIAS].launcher_runs.len();
        app.update(Action::LauncherRunRefreshed {
            alias: OBS_ALIAS.to_string(),
            run_id: "20261018-other".to_string(),
            detail: Ok(Box::new(other)),
            log: None,
        });
        let cache = &app.ctx.view_cache[OBS_ALIAS];
        assert_eq!(cache.launcher_runs.len(), row_before);
        assert_eq!(
            cache.launcher_detail.as_deref().map(|d| d.run_id.as_str()),
            Some(LAUNCHER_RUN),
            "a result for another run must not replace the selected run's detail"
        );
        drop(rx);
    }

    /// The gate really refuses: a dashboard with no Driver tab open pays nothing
    /// for it. The scan is three blocking reads, so an ungated rescan would cost
    /// the whole fleet for one pane's freshness.
    #[tokio::test]
    async fn a_tick_with_no_driver_tab_open_schedules_no_scan() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));

        for _ in 0..40 {
            app.update(Action::Tick);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        while let Ok(action) = rx.try_recv() {
            assert!(
                !matches!(action, Action::DriverRunsListed { .. }),
                "the dashboard is on top; nothing is looking at a run list"
            );
        }
    }

    /// CR-04's second failure: STEER-02's whole deliverable did not render.
    ///
    /// `derive_injection_states` iterates `cache.driver_inbox`, which only the
    /// run-list scan fills. Without a rescan on the append's success arm the
    /// message existed on disk and nowhere on screen — the four-state widget did
    /// not render at all until the driver's own `interjected` record arrived
    /// through the journal tail, at which point the pane showed the message with
    /// no state label beside it.
    #[tokio::test]
    async fn a_successful_injection_rescans_so_the_queued_state_can_render() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "abc".to_string(),
            error: None,
        });
        assert!(
            scan_reported(&mut rx).await,
            "a queued message is not observable until the inbox is re-read"
        );
    }

    /// A failed append wrote nothing, so there is nothing new to read.
    #[tokio::test]
    async fn a_failed_injection_schedules_no_scan() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "abc".to_string(),
            error: Some("PermissionDenied".to_string()),
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        while let Ok(action) = rx.try_recv() {
            assert!(
                !matches!(action, Action::DriverRunsListed { .. }),
                "nothing was written, so nothing changed on disk"
            );
        }
    }

    /// WR-03: confirming a start must not leave the wizard under the user's
    /// fingers, because a screen they land back on is a screen they can
    /// re-submit.
    ///
    /// The wizard used to `Push` the confirmation and stay on the stack, so `y`
    /// dispatched a start and popped the user back onto Step B — with an empty
    /// input buffer and the preview closed, looking exactly as it had before
    /// they confirmed. `Enter` there pushed a second confirmation for the same
    /// command and `y` started a **second run**. Nothing downstream catches it:
    /// `admit` counts live runs across all projects rather than per project, the
    /// child's `flock` refusal is invisible because its stdio is `/dev/null`,
    /// and `start_driver_run` has already overwritten `observed_runs` with the
    /// second driver's pid — so for up to five seconds `x` would signal the
    /// wrong, already-dead pid while the real run kept going.
    ///
    /// Driven through `App` rather than the screen, because the property is
    /// about the **stack** and a screen cannot see its own position on it.
    #[tokio::test]
    async fn confirming_a_start_pops_the_wizard_so_a_second_enter_cannot_start_a_duplicate() {
        use crossterm::event::{KeyCode, KeyModifiers};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        crate::registry::record_opt_in(&mut app.ctx.config, OBS_ALIAS).expect("opt in");
        app.screen_stack.push(Box::new(
            crate::ui::screens::driver_start::DriverStartScreen::new(OBS_ALIAS.to_string()),
        ));

        // Step A commits the default command; Step B hands off.
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack.last().map(|s| s.name()),
            Some("driver_confirm"),
            "Step B hands off to the confirmation"
        );
        assert!(
            !app.screen_stack.iter().any(|s| s.name() == "driver_start"),
            "and REPLACES itself doing so — the wizard must not be waiting \
             underneath: {:?}",
            app.screen_stack
                .iter()
                .map(|s| s.name())
                .collect::<Vec<_>>()
        );

        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack.last().map(|s| s.name()),
            Some(crate::ui::screens::detail::DetailScreen::NAME),
            "confirming lands the user back on the run detail, not on a wizard \
             primed to repeat itself"
        );

        // The keystroke that used to start the second run.
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

        let starts = std::iter::from_fn(|| rx.try_recv().ok())
            .filter(|action| matches!(action, Action::DriverStartRequested { .. }))
            .count();
        assert_eq!(
            starts, 1,
            "exactly one start request. Two means the wizard was still on the \
             stack and the user paid for one confirmation twice"
        );
    }

    /// WR-08: a scan landing after a tail must not double the injection
    /// records.
    ///
    /// The scan reads the journal whole from offset zero and replaces
    /// `driver_journal`; the tail keeps its own independent cursor in
    /// `journal_cursors`. When a scan lands after a tail has already consumed
    /// records, the scan's snapshot contains them and the next tail batch
    /// re-appends everything the scan read past. The state derivation is
    /// idempotent under duplication — same id, same rank, same timestamp — so
    /// this was growth and clarity rather than a wrong render; but the field's
    /// own doc claims it is "naturally bounded: one record per state transition
    /// of one message a human typed", and a vector that grows for the life of
    /// the selection is not that.
    #[tokio::test]
    async fn a_scan_and_tail_overlap_does_not_duplicate_the_injection_records() {
        use crate::ui::screens::DriverRunJournal;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        let interjected = journal_record(
            7,
            "interjected",
            &[
                ("id", "abc".into()),
                ("text", "steer".into()),
                ("delivered", true.into()),
            ],
        );
        let acted_on = journal_record(8, "interjection_acted_on", &[("id", "abc".into())]);

        // The scan's snapshot, which already read past seq 7.
        let cache = app.ctx.view_cache.entry(OBS_ALIAS.to_string()).or_default();
        cache.driver_journal = Some(Box::new(DriverRunJournal {
            run_id: OBS_RUN.to_string(),
            output: Default::default(),
            injections: vec![interjected.clone()],
        }));

        // The tail's own cursor is behind the scan's, so its batch repeats seq 7.
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![interjected, acted_on],
            cursor: cursor_at(200, 8),
        });

        let injections = &app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .and_then(|cache| cache.driver_journal.as_deref())
            .expect("the journal survives the batch")
            .injections;
        let seqs: Vec<u64> = injections.iter().map(|record| record.seq).collect();
        assert_eq!(
            seqs,
            vec![7, 8],
            "one record per transition, deduped on the seq the journal already \
             guarantees is monotonic"
        );

        // An id is NOT the key: one message legitimately carries several
        // records, and deduping on it would swallow the acted-on transition.
        assert_eq!(injections.len(), 2);
    }

    /// D-27's negative carry-forward, discharged for `driver_output`.
    #[tokio::test]
    async fn pruning_drops_the_output_buffer_for_an_unregistered_alias() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // OBS_ALIAS is registered by the fixture; "gone" never was.
        app.ctx
            .driver_output
            .entry("gone".to_string())
            .or_default()
            .push_record(DriverLineKind::Output, "output from a project that left");
        app.ctx
            .driver_output
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .push_record(DriverLineKind::Output, "output from a project that stayed");

        app.prune_driver_maps();

        assert!(
            !app.ctx.driver_output.contains_key("gone"),
            "a buffer for an unregistered alias holds up to the full ring cap and \
             nothing can ever append to it again — leaving it reintroduces the \
             Phase 16 leak under a new name"
        );

        // The control arm: a registered alias keeps its buffer AND its
        // contents, so the assertion above is not passing because the prune
        // emptied the map.
        let kept = app
            .ctx
            .driver_output
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its buffer");
        assert_eq!(
            kept.len(),
            1,
            "the prune must not clear a live buffer's contents"
        );
    }

    /// Plan 18-09 moved three driver payloads onto `ProjectViewCache`, so the
    /// map that holds it joins the prune pass. Without this, `driver_runs`,
    /// `driver_inbox` and `driver_tally` for a project the user unregistered
    /// would sit in memory for the life of the process — the same leak
    /// `driver_output` was added to the pass to prevent, under a third name.
    #[tokio::test]
    async fn pruning_drops_the_view_cache_for_an_unregistered_alias() {
        use crate::ui::screens::DriverRunTally;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        for alias in ["gone", OBS_ALIAS] {
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_selected_run = 2;
            cache.driver_tally = Some(DriverRunTally {
                run_id: OBS_RUN.to_string(),
                cumulative_cost_usd: Some(1.83),
                turn_boundaries: 3,
            });
        }

        app.prune_driver_maps();

        assert!(
            !app.ctx.view_cache.contains_key("gone"),
            "view state for an unregistered project describes something the user \
             can no longer open"
        );

        // The control arm: a registered alias keeps its cache AND its contents,
        // so the assertion above is not passing because the prune emptied the
        // map.
        let kept = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its view cache");
        assert_eq!(kept.driver_selected_run, 2);
        assert_eq!(
            kept.driver_tally
                .as_ref()
                .and_then(|t| t.cumulative_cost_usd),
            Some(1.83),
            "the prune must not clear a live cache's contents"
        );
    }

    // ── The three schedulers, the goal, and the sub-view ─────────────────

    /// A dispatch with no `event_tx` refuses visibly and sends nothing.
    ///
    /// **The second assertion is the load-bearing one.** A scheduler that set
    /// the message and dispatched anyway would pass a test that only checked
    /// `error_message`, and the write would happen regardless of what the user
    /// was told — the `driver_confirm.rs:367-372` discipline, applied here.
    #[tokio::test]
    async fn an_injection_with_no_event_channel_refuses_visibly_and_writes_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.event_tx = None;

        app.schedule_inbox_append(
            OBS_ALIAS,
            OBS_RUN,
            "3f2a".to_string(),
            "skip the UI review".to_string(),
        );

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a queue that cannot report its result must say so, not fail silently");
        assert!(
            refusal.contains("Nothing was written"),
            "the copy must state that nothing reached disk, so the user knows a \
             retry is safe rather than a way to steer twice, got: {refusal}"
        );
        assert!(
            rx.try_recv().is_err(),
            "a refused injection must dispatch NOTHING"
        );

        // And the file was never created, which is the claim the copy makes.
        let inbox = crate::journal::run_paths(&dir.path().join(".planning"), OBS_RUN)
            .expect("a plain run id resolves")
            .inbox;
        assert!(
            !inbox.exists(),
            "nothing was written must mean nothing was written"
        );
    }

    /// A traversing run id is refused before any path is joined (T-18-27).
    #[tokio::test]
    async fn a_traversing_run_id_never_reaches_the_inbox_append() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());

        app.schedule_inbox_append(
            OBS_ALIAS,
            "../../../../escaped",
            "3f2a".to_string(),
            "steer".to_string(),
        );

        assert!(
            app.ctx.error_message.is_some(),
            "the refusal must be visible, not a silent no-op"
        );
        assert!(rx.try_recv().is_err(), "nothing may be dispatched");

        // The positive control: a plain id does reach the scheduler, so the
        // assertion above is not passing because the function refuses
        // everything.
        app.ctx.error_message = None;
        app.schedule_inbox_append(OBS_ALIAS, OBS_RUN, "b1".to_string(), "steer".to_string());
        assert!(
            app.ctx.error_message.is_none(),
            "a plain run id must not be refused: {:?}",
            app.ctx.error_message
        );
    }

    /// The whole injection round trip, off the render thread (STEER-01/03).
    #[tokio::test]
    async fn an_injection_is_written_off_the_render_thread_and_reports_back() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());

        let paths = crate::journal::run_paths(&dir.path().join(".planning"), OBS_RUN)
            .expect("a plain run id resolves");
        std::fs::create_dir_all(&paths.dir).expect("the run dir must exist for an append");

        app.schedule_inbox_append(
            OBS_ALIAS,
            OBS_RUN,
            "3f2a".to_string(),
            "skip the UI review".to_string(),
        );

        let action = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("the append must report back promptly, not block the loop")
            .expect("the channel is open");
        let Action::DriverInjectWritten {
            alias, id, error, ..
        } = action
        else {
            panic!("the append must report through DriverInjectWritten, got {action:?}");
        };
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(id, "3f2a", "the caller's id must survive, not be re-minted");
        assert!(error.is_none(), "unexpected append failure: {error:?}");

        // The bytes are on disk with the caller's id, which is what makes the
        // `queued` state addressable and STEER-03 hold across a restart.
        let written = std::fs::read_to_string(&paths.inbox).expect("the inbox must exist");
        assert!(written.contains("\"id\":\"3f2a\""), "got: {written}");
        assert!(written.contains("skip the UI review"), "got: {written}");
    }

    /// A failed append sets `error_message` and **not** `status_message`.
    ///
    /// The load-bearing half is the second assertion: showing `queued` beside an
    /// error is the exact failure D-06 exists to prevent — the UI satisfied, the
    /// criterion failed.
    #[tokio::test]
    async fn a_failed_injection_write_never_reports_the_message_as_queued() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "3f2a".to_string(),
            error: Some("PermissionDenied".to_string()),
        });

        let failure = app
            .ctx
            .error_message
            .as_deref()
            .expect("a failed append must be visible");
        assert!(failure.contains("PermissionDenied"), "got: {failure}");
        assert!(
            failure.contains("Nothing was written"),
            "the copy must state that nothing reached disk, got: {failure}"
        );
        assert!(
            app.ctx.status_message.is_none(),
            "a message that was never written must never be shown as queued"
        );

        // The control arm: a successful write does set the queued status, and
        // it promises nothing about timing — the dequeue ack was measured at 55
        // seconds, so a word implying imminence would lie about the protocol.
        app.ctx.error_message = None;
        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "3f2a".to_string(),
            error: None,
        });
        let queued = app
            .ctx
            .status_message
            .as_ref()
            .map(|(text, _)| text.clone())
            .expect("a successful write must confirm");
        assert!(queued.contains("Queued"), "got: {queued}");
        assert!(
            !queued.to_lowercase().contains("sent"),
            "the word 'sent' is forbidden for an injected message, got: {queued}"
        );
    }

    /// `DriverRunsListed` populates the view cache and requests a redraw.
    #[tokio::test]
    async fn a_run_list_result_populates_the_cache_and_requests_a_redraw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.needs_redraw = false;
        app.update(Action::DriverRunsListed {
            alias: OBS_ALIAS.to_string(),
            runs: vec![run_summary("2026-07-29T12-02-00Z-a1b2")],
            inbox: vec![crate::journal::inbox::InboxMessage {
                id: "3f2a".to_string(),
                ts: "2026-07-29T21:40:02Z".to_string(),
                text: "steer".to_string(),
            }],
            journal: None,
        });

        let cache = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .expect("the handler must create the cache entry");
        assert_eq!(cache.driver_runs.len(), 1);
        assert_eq!(cache.driver_runs[0].run_id, "2026-07-29T12-02-00Z-a1b2");
        assert_eq!(cache.driver_inbox.len(), 1);
        assert!(
            app.needs_redraw,
            "a run list that changed under the pane must be painted"
        );
    }

    /// An unreadable launcher run row: enough to be listed, no file needed.
    fn launcher_entry(id: &str) -> crate::state_reader::driver_frontmatter::DriverRunEntry {
        (
            id.to_string(),
            Err(
                crate::state_reader::driver_frontmatter::FrontmatterError::Io {
                    detail: "x".to_string(),
                },
            ),
        )
    }

    fn launcher_refreshed(ids: &[&str]) -> Action {
        Action::LauncherRunsRefreshed {
            alias: OBS_ALIAS.to_string(),
            launcher_runs: ids.iter().map(|id| launcher_entry(id)).collect(),
            detail: None,
            log: None,
        }
    }

    /// `LauncherRunsRefreshed` replaces the launcher rows wholesale (28-07, D-30).
    #[tokio::test]
    async fn a_run_list_result_replaces_the_launcher_runs_never_merges() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(launcher_refreshed(&["20261019-a", "20261018-b"]));
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].launcher_runs.len(), 2);
        assert!(
            app.ctx.view_cache[OBS_ALIAS].launcher_listed,
            "the first listing must clear the Loading state"
        );
        app.update(launcher_refreshed(&["20261019-a"]));
        let ids: Vec<&str> = app.ctx.view_cache[OBS_ALIAS]
            .launcher_runs
            .iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(ids, ["20261019-a"], "a run gone from disk must disappear");
    }

    /// Selection is pinned to the run id: a newer run landing above the selected
    /// one must not move the selection onto a different run.
    #[tokio::test]
    async fn launcher_view_a_newer_run_above_the_selection_keeps_the_same_run_selected() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(launcher_refreshed(&["20261019-a", "20261018-b"]));
        {
            let cache = app.ctx.view_cache.get_mut(OBS_ALIAS).unwrap();
            cache.launcher_selected = 1;
            cache.launcher_selected_id = Some("20261018-b".to_string());
        }

        app.update(launcher_refreshed(&[
            "20261020-new",
            "20261019-a",
            "20261018-b",
        ]));
        let cache = &app.ctx.view_cache[OBS_ALIAS];
        assert_eq!(cache.launcher_selected, 2, "still on 20261018-b");
        assert_eq!(cache.launcher_selected_id.as_deref(), Some("20261018-b"));

        // The selected run vanishing clamps the index into range and re-pins.
        app.update(launcher_refreshed(&["20261020-new"]));
        let cache = &app.ctx.view_cache[OBS_ALIAS];
        assert_eq!(cache.launcher_selected, 0);
        assert_eq!(cache.launcher_selected_id.as_deref(), Some("20261020-new"));
    }

    // ── 28-07: stopping a launcher run from the Driver tab ───────────────

    const LAUNCHER_RUN: &str = "20261019-0042-abcd";

    /// A project with a launcher run dir `LAUNCHER_RUN` holding nothing, so the lock
    /// probe reads it as free and `stop_run` reports `NotRunning` without touching
    /// systemd.
    fn launcher_project(root: &std::path::Path) {
        std::fs::create_dir_all(
            root.join(".planning/meta-manager/drivers")
                .join(LAUNCHER_RUN),
        )
        .expect("run dir");
    }

    async fn next_action(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) -> Option<Action> {
        tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
            .await
            .ok()
            .flatten()
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_stop_requested_runs_the_shared_stop_and_reports_through_launcher_stopped() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = obs_app(dir.path());

        app.update(Action::LauncherStopRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });

        let Some(Action::LauncherStopped {
            alias,
            run_id,
            report,
        }) = next_action(&mut rx).await
        else {
            panic!("the stop must report through LauncherStopped");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (OBS_ALIAS, LAUNCHER_RUN));
        assert_eq!(report, "the run is not running");
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_stop_of_a_run_that_is_not_there_reports_the_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = obs_app(dir.path());

        app.update(Action::LauncherStopRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: "20261019-missing".to_string(),
        });

        let Some(Action::LauncherStopped { report, .. }) = next_action(&mut rx).await else {
            panic!("a failed stop is still reported");
        };
        assert!(
            report.starts_with("invalid run") || report.contains("run"),
            "{report}"
        );
        assert_ne!(report, "the run is not running");
    }

    #[tokio::test]
    async fn launcher_stop_requested_for_an_unregistered_alias_refuses_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.needs_redraw = false;

        app.update(Action::LauncherStopRequested {
            alias: "nobody".to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });

        let message = app.ctx.error_message.clone().expect("a visible refusal");
        assert!(message.contains("nobody"), "{message}");
        assert!(app.needs_redraw);
        assert!(rx.try_recv().is_err(), "nothing is dispatched");
    }

    #[tokio::test]
    async fn launcher_stop_requested_without_an_event_channel_refuses_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.event_tx = None;

        app.update(Action::LauncherStopRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });

        let message = app.ctx.error_message.clone().expect("a visible refusal");
        assert!(message.contains("event channel"), "{message}");
    }

    #[tokio::test]
    async fn launcher_stop_does_nothing_where_the_driver_tab_does_not_exist() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.experimental = false;

        app.update(Action::LauncherStopRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });

        assert!(app.ctx.error_message.is_none());
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err(), "no stop was started");
    }

    #[tokio::test]
    async fn launcher_stop_stopped_sets_the_status_line_and_relists_the_driver_runs() {
        let dir = tempfile::tempdir().expect("temp dir");
        launcher_project(dir.path());
        let (mut app, mut rx) = obs_app(dir.path());
        app.needs_redraw = false;

        app.update(Action::LauncherStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            report: "stopped the launcher (pid 7); the run is now stopped".to_string(),
        });

        let (text, _) = app.ctx.status_message.clone().expect("status line");
        assert_eq!(
            text,
            format!("{OBS_ALIAS}: stopped the launcher (pid 7); the run is now stopped")
        );
        assert!(app.needs_redraw);

        // Both listings follow: the journal runs, and the launcher rows whose
        // derived status is what the stop just changed.
        let mut launcher = None;
        let mut journal_listed = false;
        while launcher.is_none() || !journal_listed {
            match next_action(&mut rx).await {
                Some(Action::LauncherRunsRefreshed {
                    alias,
                    launcher_runs,
                    ..
                }) => launcher = Some((alias, launcher_runs)),
                Some(Action::DriverRunsListed { .. }) => journal_listed = true,
                Some(_) => {}
                None => panic!("a re-list of the Driver runs must follow"),
            }
        }
        let (alias, launcher_runs) = launcher.unwrap();
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(launcher_runs.len(), 1);
        assert_eq!(launcher_runs[0].0, LAUNCHER_RUN);
    }

    #[test]
    fn launcher_stop_stopped_strips_terminal_controls_from_the_report() {
        let mut app = App::new_for_test();
        app.update(Action::LauncherStopped {
            alias: "p".to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            report: "bad\u{1b}[2Jreport".to_string(),
        });
        let (text, _) = app.ctx.status_message.clone().expect("status line");
        assert!(!text.chars().any(char::is_control), "{text:?}");
    }

    #[tokio::test]
    async fn launcher_stop_a_shorter_launcher_list_clamps_the_highlight() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .launcher_selected = 5;

        app.update(launcher_refreshed(&["20261019-a"]));
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].launcher_selected, 0);

        app.update(launcher_refreshed(&[]));
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].launcher_selected, 0);
    }

    /// A shortened list clamps the selection rather than leaving it dangling.
    #[tokio::test]
    async fn a_shorter_run_list_clamps_the_selection_instead_of_dangling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .driver_selected_run = 4;

        app.update(Action::DriverRunsListed {
            alias: OBS_ALIAS.to_string(),
            runs: vec![run_summary("2026-07-29T12-02-00Z-a1b2")],
            inbox: Vec::new(),
            journal: None,
        });

        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS].driver_selected_run, 0,
            "a selection past the end of a shortened list must be clamped, or the \
             detail pane indexes a run that is not there"
        );
    }

    /// D-23 / OBS-03: an empty goal buffer reaches `start_driver_run` as `None`.
    ///
    /// Asserted at two levels, because the pure mapping alone would not prove
    /// the consequence: the second half drives the real `drive_argv` and shows
    /// that `None` omits the flag entirely while `Some("")` would push it with
    /// an empty operand — which is what makes the driver record an empty goal
    /// **as though one had been given**.
    #[test]
    fn an_empty_goal_buffer_reaches_start_driver_run_as_none() {
        assert_eq!(goal_or_none(None), None);
        assert_eq!(goal_or_none(Some("")), None, "an empty buffer is no goal");
        assert_eq!(
            goal_or_none(Some("   ")),
            None,
            "a buffer of spaces is no goal either"
        );
        // **The shapes `str::trim` cannot see**, which is what this predicate
        // used to be written against. A goal of one zero-width space rendered as
        // a blank line wearing a goal's label — which reads as a goal the user
        // cannot see, rather than as no goal. The whole shared set, so a seventh
        // blank shape arrives here without anybody copying it.
        for blank in crate::test_support::DEGENERATE {
            assert_eq!(
                goal_or_none(Some(blank)),
                None,
                "{blank:?} carries no visible goal"
            );
        }
        assert_eq!(
            goal_or_none(Some("ship the driver tab")),
            Some("ship the driver tab"),
            "a goal that was given is returned verbatim and never paraphrased"
        );
        assert_eq!(
            goal_or_none(Some("  ship it  ")),
            Some("  ship it  "),
            "emptiness is tested on the trimmed text; the value returned is the \
             untrimmed original, because the goal is stored verbatim"
        );

        #[cfg(unix)]
        {
            use crate::driver::spawn::drive_argv;
            let config = std::path::Path::new("/tmp/config.json");

            let empty = drive_argv(
                config,
                "proj",
                "/gsd-progress",
                "run-1",
                goal_or_none(Some("")),
            );
            assert!(
                !empty.iter().any(|arg| arg == "--goal"),
                "an empty goal must omit the flag entirely, got: {empty:?}"
            );

            // The control: a real goal still reaches the child.
            let given = drive_argv(
                config,
                "proj",
                "/gsd-progress",
                "run-1",
                goal_or_none(Some("ship it")),
            );
            let position = given
                .iter()
                .position(|arg| arg == "--goal")
                .expect("a goal that was given must reach the child");
            assert_eq!(given[position + 1], "ship it");
        }
    }

    /// The `DriverStartRequested` handler threads the variant's own goal.
    ///
    /// The refusal is what is observed: `start_driver_run` refuses an unknown
    /// alias before it spawns anything, so this asserts the handler reached the
    /// seam at all rather than dropping the request — the goal's own mapping is
    /// pinned by the test above.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_start_request_carries_its_goal_to_the_spawn_seam() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverStartRequested {
            alias: "nosuchalias".to_string(),
            command: "/gsd-progress".to_string(),
            goal: Some("ship the driver tab".to_string()),
        });

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("the request must reach `start_driver_run`, which refuses visibly");
        assert!(refusal.contains("nosuchalias"), "got: {refusal}");
    }

    /// The Driver sub-view round-trips through both `detail.rs` mappings.
    #[test]
    fn the_driver_sub_view_is_the_last_tab_index_in_both_directions() {
        use crate::ui::screens::detail::{sub_view_from_index, tab_index};

        // Index 8 (the last tab, after the Phase 24 consolidation to eight
        // tabs plus the Driver), and the two mappings must agree — a tab whose
        // index does not round-trip lands on a different tab than the one the
        // user asked for. The literal is the point: it pins the index the key
        // map and the tab bar both assume.
        assert_eq!(tab_index(&DetailSubView::Driver), 8);
        assert_eq!(sub_view_from_index(8, true), DetailSubView::Driver);
        // The fallback: an out-of-range index lands on the default Roadmap tab
        // and never on the newest one.
        assert_eq!(sub_view_from_index(9, true), DetailSubView::RoadmapViz);
    }

    /// A stop with nowhere to report its outcome refuses **visibly** (WR-11).
    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_with_no_event_channel_refuses_visibly_rather_than_silently() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());
        app.ctx.event_tx = None;

        app.stop_driver_run(OBS_ALIAS);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a stop that cannot report its outcome must say so, not fail silently");
        assert!(refusal.contains(OBS_ALIAS), "got: {refusal}");
        assert!(
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "a stop that never dispatched must not have disturbed the observed map"
        );
    }

    #[tokio::test]
    async fn a_tail_result_advances_only_its_own_cursor() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        let mine = (OBS_ALIAS.to_string(), "run-a".to_string());
        let theirs = (OBS_ALIAS.to_string(), "run-b".to_string());
        app.ctx
            .journal_cursors
            .insert(mine.clone(), cursor_at(10, 1));
        app.ctx
            .journal_cursors
            .insert(theirs.clone(), cursor_at(20, 7));

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-a".to_string(),
            records: Vec::new(),
            cursor: cursor_at(99, 4),
        });

        assert_eq!(
            app.ctx.journal_cursors.get(&mine),
            Some(&cursor_at(99, 4)),
            "the tail's own cursor must advance, seq and all"
        );
        assert_eq!(
            app.ctx.journal_cursors.get(&theirs),
            Some(&cursor_at(20, 7)),
            "a sibling run's cursor must not move"
        );
    }

    /// A run id whose lexicographic order equals its chronological order, which
    /// is `new_run_id`'s documented property and what the retention sort relies
    /// on. Built by hand rather than through `new_run_id` so the ordering under
    /// test is visible in the test itself.
    fn run_id_at(minute: u32) -> String {
        format!("2026-07-29T12-{minute:02}-00Z-a1b2")
    }

    /// D-27, first pass: an alias the registry no longer knows about.
    #[tokio::test]
    async fn pruning_drops_cursors_for_an_unregistered_alias() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // OBS_ALIAS is registered by the fixture; "gone" never was.
        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(1)), cursor_at(5, 1));
        app.ctx
            .journal_cursors
            .insert(("gone".to_string(), run_id_at(2)), cursor_at(6, 2));
        app.ctx
            .run_states
            .insert("gone".to_string(), crate::executor::RunState::Running);
        app.ctx
            .run_states
            .insert(OBS_ALIAS.to_string(), crate::executor::RunState::Running);
        app.ctx
            .observed_runs
            .insert("gone".to_string(), observed("gone", "run-gone", ALIVE));
        app.ctx.observed_runs.insert(
            OBS_ALIAS.to_string(),
            observed(OBS_ALIAS, "run-here", ALIVE),
        );

        app.prune_driver_maps();

        assert!(
            !app.ctx
                .journal_cursors
                .contains_key(&("gone".to_string(), run_id_at(2))),
            "a cursor for an unregistered alias is dead weight for the whole \
             life of the process — nothing can ever append to it again"
        );
        assert!(!app.ctx.run_states.contains_key("gone"));
        assert!(!app.ctx.observed_runs.contains_key("gone"));

        // The control arm: a registered alias keeps everything, so the
        // assertions above are not passing because the prune emptied the maps.
        assert!(app
            .ctx
            .journal_cursors
            .contains_key(&(OBS_ALIAS.to_string(), run_id_at(1))));
        assert!(app.ctx.run_states.contains_key(OBS_ALIAS));
        assert!(app.ctx.observed_runs.contains_key(OBS_ALIAS));
    }

    /// D-27, second pass: the per-alias retention bound.
    ///
    /// Asserts the count **and** which ids survived. A prune that kept the right
    /// number of the wrong runs — the oldest — would satisfy a count-only
    /// assertion while discarding exactly the cursors a live run needs.
    #[tokio::test]
    async fn pruning_retains_at_most_the_newest_runs_per_alias() {
        use crate::journal::RETAIN_RUNS;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        let total = RETAIN_RUNS + 5;
        for minute in 0..total {
            app.ctx.journal_cursors.insert(
                (OBS_ALIAS.to_string(), run_id_at(minute as u32)),
                cursor_at(minute as u64, minute as u64),
            );
        }
        assert_eq!(app.ctx.journal_cursors.len(), total);

        app.prune_driver_maps();

        assert_eq!(
            app.ctx.journal_cursors.len(),
            RETAIN_RUNS,
            "the in-memory bound must match the on-disk one rather than being a \
             second unrelated number"
        );

        let mut survivors: Vec<String> = app
            .ctx
            .journal_cursors
            .keys()
            .map(|(_, run_id)| run_id.clone())
            .collect();
        survivors.sort();
        let expected: Vec<String> = (total - RETAIN_RUNS..total)
            .map(|minute| run_id_at(minute as u32))
            .collect();
        assert_eq!(
            survivors, expected,
            "the NEWEST ids must survive; run ids sort lexicographically in \
             chronological order by construction, so this is a string sort and \
             needs no file read"
        );

        // Under the bound, nothing is evicted.
        app.ctx.journal_cursors.clear();
        for minute in 0..RETAIN_RUNS {
            app.ctx.journal_cursors.insert(
                (OBS_ALIAS.to_string(), run_id_at(minute as u32)),
                cursor_at(0, 0),
            );
        }
        app.prune_driver_maps();
        assert_eq!(app.ctx.journal_cursors.len(), RETAIN_RUNS);
    }

    /// The interactive removal path cleans the driver maps immediately, rather
    /// than leaving them to the periodic backstop.
    #[tokio::test]
    async fn removing_a_project_interactively_clears_its_driver_maps() {
        use crate::ui::screens::delete_confirm::DeleteConfirmScreen;
        use crate::ui::screens::Screen;
        use crossterm::event::{KeyCode, KeyModifiers};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        // `save_config` must land somewhere real, or the removal reports a save
        // failure and returns before it reaches the cleanup block.
        app.ctx.config_path = dir.path().join("config.json");

        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(1)), cursor_at(5, 1));
        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(2)), cursor_at(9, 3));
        app.ctx
            .run_states
            .insert(OBS_ALIAS.to_string(), crate::executor::RunState::Running);
        // `DEAD`, and the choice is load-bearing since plan 17-08: the removal
        // path now REFUSES a project whose observed run is not known to be
        // finished (CR-06), so a live run here would make this test assert the
        // refusal rather than the D-27 cleanup it is about. A crashed run is
        // nothing left to abandon, so it is removable — and it still exercises
        // the sibling-map cleanup, which is this test's actual subject.
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-here", DEAD));

        let mut screen = DeleteConfirmScreen::new(OBS_ALIAS.to_string());
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut app.ctx);

        assert!(
            !app.ctx.config.projects.contains_key(OBS_ALIAS),
            "the removal itself must have happened, or the rest is vacuous"
        );
        assert!(
            app.ctx.error_message.is_none(),
            "unexpected failure: {:?}",
            app.ctx.error_message
        );
        assert!(
            !app.ctx.run_states.contains_key(OBS_ALIAS),
            "run_states leaked past an interactive removal"
        );
        assert!(
            !app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "observed_runs leaked past an interactive removal"
        );
        assert!(
            app.ctx
                .journal_cursors
                .keys()
                .all(|(alias, _)| alias != OBS_ALIAS),
            "every cursor for the removed alias must go, not just one — the map \
             is keyed (alias, run_id) and a project may have several"
        );
    }

    // ── The phase's negative carry-forward, discharged mechanically ────────

    /// Every per-alias and per-run map on `AppContext` is covered by
    /// `prune_driver_maps` (D-27, the Phase 16 carry-forward).
    ///
    /// **This is the durable form of a promise that was previously kept by
    /// remembering.** The sibling tests above each prove one map is pruned; this
    /// one proves the *set* is complete, and the exhaustive destructuring below
    /// is what makes it stay complete: adding a field to `AppContext` fails to
    /// compile here until someone has looked at this list and decided whether
    /// the new field is alias-keyed. A new map that is not pruned reintroduces
    /// the Phase 16 leak under a new name, which is precisely what the
    /// carry-forward exists to prevent.
    #[tokio::test]
    async fn every_per_alias_driver_map_is_pruned() {
        use crate::executor::RunState;
        use crate::journal::reader::JournalCursor;
        use crate::ui::screens::{DriverLineKind, DryRunPreview};

        const GONE: &str = "gone";

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Populate every alias-keyed map for BOTH a registered alias and an
        // alias the registry has never heard of, so each assertion below has a
        // control arm and none of them can pass by emptying the map.
        for alias in [GONE, OBS_ALIAS] {
            app.ctx.journal_cursors.insert(
                (alias.to_string(), OBS_RUN.to_string()),
                JournalCursor::default(),
            );
            app.ctx
                .run_states
                .insert(alias.to_string(), RunState::Running);
            app.ctx.observed_runs.insert(
                alias.to_string(),
                observed(alias, OBS_RUN, crate::driver::liveness::Liveness::Dead),
            );
            app.ctx
                .last_outcomes
                .insert(alias.to_string(), "failed".to_string());
            app.ctx
                .driver_output
                .entry(alias.to_string())
                .or_default()
                .push_record(DriverLineKind::Output, "a line");
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_selected_run = 1;
            // The field plan 18-11 added. It is inside `view_cache`, so it
            // inherits that map's prune rather than needing its own — and this
            // test is where that inheritance is checked rather than assumed.
            cache.driver_dry_run = Some(DryRunPreview {
                command: "/gsd:progress".to_string(),
                report: None,
            });
            app.ctx
                .last_refresh
                .insert(alias.to_string(), std::time::Instant::now());
            // Keyed alias THEN milestone. This fixture used to insert the alias
            // as the map's only key, which is how it passed while the real map
            // was keyed by milestone and the prune emptied it every pass.
            app.ctx.archive_cache.insert(
                alias.to_string(),
                "v1.0".to_string(),
                crate::archive::MilestoneArchive {
                    version: "v1.0".to_string(),
                    top_level_files: Vec::new(),
                    phases: Vec::new(),
                },
            );
        }

        app.prune_driver_maps();

        // ── The enumeration. Destructured exhaustively so a new `AppContext`
        // field is a compile error here until it has been classified.
        let crate::ui::screens::AppContext {
            // Alias-keyed or (alias, run_id)-keyed: every one of these must be
            // pruned, and each is asserted below.
            journal_cursors,
            run_states,
            observed_runs,
            last_outcomes,
            driver_output,
            view_cache,
            last_refresh,
            archive_cache,

            // Alias-keyed but deliberately NOT pruned, with the reason on the
            // field: `project_states` is the registry's own mirror and is
            // rebuilt wholesale by `load_project_states`;
            // `detail_sub_view_per_project` holds one enum per alias and is
            // rewritten on every tab switch.
            project_states: _,
            detail_sub_view_per_project: _,
            // `agent_views` is replaced wholesale by every agents scan, which
            // covers the registered projects only and drops any alias the
            // registry does not hold — the replacement is its prune, so an
            // unregistered alias cannot outlive the next scan (AGENT-05).
            agent_views: _,
            agents_scan_in_flight: _,

            // Not alias-keyed at all. `session_spawned_runs` is a set of run
            // ids that grows by one per run this session starts, with
            // `driver_max_concurrent` defaulting to one; the rest are scalars,
            // handles and single values. `experimental` is the startup-resolved
            // experimental-features flag (260917-fko D1): one scalar bool, not
            // alias-keyed, never mutated after `from_config`, and therefore
            // nothing for the prune to reach.
            session_spawned_runs: _,
            experimental: _,
            config: _,
            config_path: _,
            table_state: _,
            filtered_aliases: _,
            filter_text: _,
            change_tracker: _,
            status_message: _,
            error_message: _,
            event_tx: _,
            exec_tx: _,
            reparse_dispatches: _,
            sort_mode: _,
            watcher: _,
            detail_scroll_offset: _,
            suggestion_index: _,
            input_buffer: _,
            needs_redraw: _,
            active_sessions: _,
        } = &app.ctx;

        assert!(
            journal_cursors.keys().all(|(alias, _)| alias != GONE),
            "journal_cursors is keyed (alias, run_id) and must lose every entry \
             for an unregistered alias"
        );
        assert!(!run_states.contains_key(GONE), "run_states leaked");
        assert!(!observed_runs.contains_key(GONE), "observed_runs leaked");
        assert!(!last_outcomes.contains_key(GONE), "last_outcomes leaked");
        assert!(!driver_output.contains_key(GONE), "driver_output leaked");
        assert!(!view_cache.contains_key(GONE), "view_cache leaked");
        assert!(!last_refresh.contains_key(GONE), "last_refresh leaked");
        assert!(!archive_cache.has_alias(GONE), "archive_cache leaked");

        // The control arm for all seven at once: the registered alias kept
        // everything, so none of the assertions above passed because the prune
        // cleared the map.
        assert!(journal_cursors.keys().any(|(alias, _)| alias == OBS_ALIAS));
        assert!(run_states.contains_key(OBS_ALIAS));
        assert!(observed_runs.contains_key(OBS_ALIAS));
        assert!(last_outcomes.contains_key(OBS_ALIAS));
        assert!(driver_output.contains_key(OBS_ALIAS));
        assert!(last_refresh.contains_key(OBS_ALIAS));
        assert!(archive_cache.get(OBS_ALIAS, "v1.0").is_some());
        let kept = view_cache
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its view cache");
        assert_eq!(kept.driver_selected_run, 1);
        assert!(
            kept.driver_dry_run.is_some(),
            "and keeps the preview it had open"
        );
    }

    /// A dry-run report is parked only under the preview it was built for
    /// (D-26).
    #[tokio::test]
    async fn a_dry_run_report_is_stored_only_under_the_preview_it_was_built_for() {
        use crate::ui::screens::DryRunPreview;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Nothing open: a report that arrives after the user pressed Esc has
        // nowhere to go, and recreating the preview for it would repaint a pane
        // the user has left.
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:progress".to_string(),
            report: "orphaned".to_string(),
        });
        assert!(
            app.ctx
                .view_cache
                .get(OBS_ALIAS)
                .and_then(|cache| cache.driver_dry_run.as_ref())
                .is_none(),
            "a report with no open preview must not create one"
        );

        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .driver_dry_run = Some(DryRunPreview {
            command: "/gsd:progress".to_string(),
            report: None,
        });

        // Stale: built for a command the user has since retyped. Storing it
        // would show one command's blast radius under another command's name —
        // the display disagreeing with the disk in the direction that flatters
        // the run.
        app.needs_redraw = false;
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:execute-phase 99".to_string(),
            report: "the wrong report".to_string(),
        });
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .driver_dry_run
                .as_ref()
                .and_then(|p| p.report.as_deref()),
            None,
            "a report for a different command must be discarded"
        );
        assert!(!app.needs_redraw, "and must not repaint the frame");

        // Matching: stored, and the pane is repainted.
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:progress".to_string(),
            report: "== Push refspecs this state would produce ==".to_string(),
        });
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .driver_dry_run
                .as_ref()
                .and_then(|p| p.report.as_deref()),
            Some("== Push refspecs this state would produce =="),
        );
        assert!(app.needs_redraw);
    }

    // ── The Archive tab's drill-in view across refreshes ───────────────────
    //
    // Debug session `archive-milestone-view-loading`. `Archive > v1.2` showed
    // its content and then, a few seconds later, flipped back to `Loading...`
    // and stayed there until ESC + re-enter. The few seconds was the 20-tick
    // block: `prune_driver_maps` retained `archive_cache` entries whose key was
    // a registered ALIAS, but the map was keyed by MILESTONE, so every entry
    // went on every pass. These tests drive the real `App` — key events,
    // channel actions, ticks and a `TestBackend` render — because the defect
    // lived in the seam between the drill-in, the tick and the render, and a
    // test of any one of them alone passed.

    /// Archive fixture: `v1.1` and `v1.2` archived, `v1.2` carrying one
    /// top-level file and one phase directory named `phase_dir`.
    fn archive_fixture(root: &std::path::Path, phase_dir: &str) {
        let milestones = root.join(".planning/milestones");
        let phase = milestones.join("v1.2-phases").join(phase_dir);
        std::fs::create_dir_all(&phase).expect("phase dir");
        std::fs::write(milestones.join("v1.1-ROADMAP.md"), "# v1.1\n").expect("v1.1 roadmap");
        std::fs::write(milestones.join("v1.2-ROADMAP.md"), "# v1.2\n").expect("v1.2 roadmap");
        std::fs::write(phase.join("01-PLAN.md"), "plan\n").expect("phase plan");
    }

    /// Open the milestone archive the way a user does: `8` for Docs, then `m`
    /// for its Milestones sub-tab (D-B04). The detail screen is pushed already
    /// on Docs › Files, and a real `m` key goes through `App::update`, so the
    /// milestone discovery is scheduled on the real channel by the Docs sub-tab
    /// switch itself.
    fn open_docs_milestones(app: &mut App, alias: &str) {
        let screen = crate::ui::screens::detail::DetailScreen::opened_on(
            alias.to_string(),
            DetailSubView::Browse,
            &mut app.ctx,
        );
        app.screen_stack.push(Box::new(screen));
        press(app, KeyCode::Char('m'));
    }

    fn press(app: &mut App, code: KeyCode) {
        app.update(Action::RawKey(crossterm::event::KeyEvent::new(
            code,
            KeyModifiers::NONE,
        )));
    }

    /// Apply archive actions from the channel until one satisfies `wanted`.
    ///
    /// Only the two archive actions are applied. Everything else (session
    /// detection and reconciliation results scheduled by the tick) is dropped,
    /// because `SessionsDetected` auto-registers whatever agent sessions happen
    /// to be running on the test machine — a verdict that depends on the
    /// machine. Returns `false` if nothing wanted arrives within two seconds.
    async fn pump_archive_until(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        wanted: impl Fn(&Action) -> bool,
    ) -> bool {
        loop {
            let next = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
            let Ok(Some(action)) = next else {
                return false;
            };
            let is_wanted = wanted(&action);
            if matches!(
                action,
                Action::ArchiveLoaded { .. } | Action::ArchiveMilestonesDiscovered { .. }
            ) {
                app.update(action);
            }
            if is_wanted {
                return true;
            }
        }
    }

    fn is_discovery_for(alias: &'static str) -> impl Fn(&Action) -> bool {
        move |action| matches!(action, Action::ArchiveMilestonesDiscovered { alias: a, .. } if a == alias)
    }

    fn is_load_for(alias: &'static str, milestone: &'static str) -> impl Fn(&Action) -> bool {
        move |action| {
            matches!(
                action,
                Action::ArchiveLoaded { alias: a, milestone: m, .. } if a == alias && m == milestone
            )
        }
    }

    /// Draw the top of the screen stack into a `TestBackend` and join the cells.
    fn render_top_screen(app: &App) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let (width, height) = (120u16, 30u16);
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| {
                app.screen_stack
                    .last()
                    .expect("a screen on the stack")
                    .render(frame, frame.area(), &app.ctx)
            })
            .expect("draw the top screen");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol())
                            .unwrap_or(" ")
                            .to_string()
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Open Docs › Milestones for `alias` and drill into `v1.2` (second in the
    /// list), exactly as a user does: `8`, `m`, `Down`, `Enter`.
    async fn drill_into_v1_2(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        alias: &'static str,
    ) -> bool {
        open_docs_milestones(app, alias);
        assert!(
            pump_archive_until(app, rx, is_discovery_for(alias)).await,
            "milestone discovery for {alias} never arrived"
        );
        press(app, KeyCode::Down);
        press(app, KeyCode::Enter);
        pump_archive_until(app, rx, is_load_for(alias, "v1.2")).await
    }

    /// The reported symptom, reproduced end to end: the drill-in view keeps its
    /// content across the 20-tick block (~5 s) that runs detection,
    /// reconciliation and the map prune.
    #[tokio::test]
    async fn archive_milestone_view_keeps_its_content_across_the_periodic_prune() {
        let dir = tempfile::tempdir().expect("temp dir");
        archive_fixture(dir.path(), "01-core-flow");
        let (mut app, mut rx) = obs_app(dir.path());

        assert!(
            drill_into_v1_2(&mut app, &mut rx, OBS_ALIAS).await,
            "the v1.2 load never arrived"
        );
        let before = render_top_screen(&app);
        assert!(before.contains("Archive > v1.2"), "{before}");
        assert!(before.contains("Phase 01: Core Flow"), "{before}");
        assert!(!before.contains("Loading..."), "{before}");

        // Twenty ticks is one pass of the 20-tick block at 250 ms: the
        // "after a few seconds" in the report. Two passes, so a fix that only
        // survives the first prune does not pass.
        for pass in 1..=2 {
            for _ in 0..20 {
                app.update(Action::Tick);
            }
            let after = render_top_screen(&app);
            assert!(
                !after.contains("Loading..."),
                "Archive > v1.2 flipped back to Loading... after 20-tick pass {pass}:\n{after}"
            );
            assert!(after.contains("Phase 01: Core Flow"), "{after}");
        }
    }

    /// Two projects that both archived a `v1.2` each see their OWN archive.
    /// Keyed by milestone alone, the second drill-in found the first project's
    /// entry, scheduled no load, and drew the other project's phases.
    #[tokio::test]
    async fn the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry() {
        use crate::config::RegisteredProject;
        const OTHER: &str = "other";

        let first = tempfile::tempdir().expect("temp dir");
        let second = tempfile::tempdir().expect("temp dir");
        archive_fixture(first.path(), "01-alpha-work");
        archive_fixture(second.path(), "01-bravo-work");
        let (mut app, mut rx) = obs_app(first.path());
        app.ctx.config.projects.insert(
            OTHER.to_string(),
            RegisteredProject {
                path: second.path().to_path_buf(),
                added: "2026-09-23".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );

        assert!(drill_into_v1_2(&mut app, &mut rx, OBS_ALIAS).await);
        let first_view = render_top_screen(&app);
        assert!(first_view.contains("Phase 01: Alpha Work"), "{first_view}");

        app.screen_stack.pop();
        let loaded = drill_into_v1_2(&mut app, &mut rx, OTHER).await;
        let second_view = render_top_screen(&app);
        assert!(
            second_view.contains("Phase 01: Bravo Work") && !second_view.contains("Alpha Work"),
            "project `{OTHER}`'s Archive > v1.2 must show its own phases \
             (its load arrived: {loaded}):\n{second_view}"
        );
    }

    /// A change under `.planning/` reloads the open archive IN PLACE: the old
    /// content stays on screen until the new listing lands, and nothing in
    /// between is `Loading...`.
    ///
    /// The change is reported as `STATE.md`, not as a path under
    /// `milestones/`, on purpose: the watcher sends one `Planning` event per
    /// project per debounce batch carrying the FIRST path in the batch, so a
    /// reload gated on the path naming `milestones/` would miss a
    /// complete-milestone write that happened to share a batch with STATE.md.
    #[tokio::test]
    async fn archive_milestone_view_reloads_in_place_when_planning_files_change() {
        let dir = tempfile::tempdir().expect("temp dir");
        archive_fixture(dir.path(), "01-core-flow");
        let (mut app, mut rx) = obs_app(dir.path());
        assert!(drill_into_v1_2(&mut app, &mut rx, OBS_ALIAS).await);

        let late = dir
            .path()
            .join(".planning/milestones/v1.2-phases/02-late-addition");
        std::fs::create_dir_all(&late).expect("late phase dir");
        std::fs::write(late.join("02-PLAN.md"), "plan\n").expect("late plan");

        app.update(Action::FileChanged {
            project_path: dir.path().to_path_buf(),
            changed_path: dir.path().join(".planning/STATE.md"),
        });

        let during = render_top_screen(&app);
        assert!(
            during.contains("Phase 01: Core Flow") && !during.contains("Loading..."),
            "the open archive must keep its content while the reload is in flight:\n{during}"
        );

        assert!(
            pump_archive_until(&mut app, &mut rx, is_load_for(OBS_ALIAS, "v1.2")).await,
            "a planning change scheduled no reload of the open archive"
        );
        let after = render_top_screen(&app);
        assert!(after.contains("Phase 02: Late Addition"), "{after}");
        assert!(after.contains("Phase 01: Core Flow"), "{after}");
        assert!(!after.contains("Loading..."), "{after}");
    }

    /// The milestone list (the Archive tab's top level) reloads in place too:
    /// a milestone archived while the list is on screen appears without
    /// leaving the tab.
    #[tokio::test]
    async fn the_archive_milestone_list_picks_up_a_newly_archived_milestone_in_place() {
        let dir = tempfile::tempdir().expect("temp dir");
        archive_fixture(dir.path(), "01-core-flow");
        let (mut app, mut rx) = obs_app(dir.path());
        open_docs_milestones(&mut app, OBS_ALIAS);
        assert!(pump_archive_until(&mut app, &mut rx, is_discovery_for(OBS_ALIAS)).await);
        assert!(!render_top_screen(&app).contains("v1.3"));

        std::fs::write(
            dir.path().join(".planning/milestones/v1.3-ROADMAP.md"),
            "# v1.3\n",
        )
        .expect("archive v1.3");
        app.update(Action::FileChanged {
            project_path: dir.path().to_path_buf(),
            changed_path: dir.path().join(".planning/STATE.md"),
        });
        assert!(
            pump_archive_until(&mut app, &mut rx, is_discovery_for(OBS_ALIAS)).await,
            "a planning change scheduled no re-discovery of the listed milestones"
        );
        let after = render_top_screen(&app);
        assert!(after.contains("v1.3"), "{after}");
        assert!(!after.contains("Loading..."), "{after}");
    }

    /// An in-place reload that SHRINKS the listing under the cursor leaves the
    /// cursor on a row that exists, so `Enter` still opens something. The
    /// neighbour of the reload test above: there the listing grew.
    #[tokio::test]
    async fn an_in_place_reload_that_shrinks_the_listing_keeps_the_cursor_on_a_row() {
        let dir = tempfile::tempdir().expect("temp dir");
        archive_fixture(dir.path(), "01-core-flow");
        let (mut app, mut rx) = obs_app(dir.path());
        assert!(drill_into_v1_2(&mut app, &mut rx, OBS_ALIAS).await);

        // Rows: [v1.2-ROADMAP.md, Phase 01]. Put the cursor on the phase, the
        // last row, then archive-edit the phase away.
        press(&mut app, KeyCode::Down);
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].archive_selected[1], 1);
        std::fs::remove_dir_all(
            dir.path()
                .join(".planning/milestones/v1.2-phases/01-core-flow"),
        )
        .expect("remove the phase");

        app.update(Action::FileChanged {
            project_path: dir.path().to_path_buf(),
            changed_path: dir.path().join(".planning/STATE.md"),
        });
        assert!(pump_archive_until(&mut app, &mut rx, is_load_for(OBS_ALIAS, "v1.2")).await);

        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS].archive_selected[1], 0,
            "one row is left, so the cursor must be on row 0"
        );
        let after = render_top_screen(&app);
        assert!(after.contains(">   v1.2-ROADMAP.md"), "{after}");
        assert!(!after.contains("Core Flow"), "{after}");
    }

    /// `Enter` on the Roadmap's collapsed shipped-milestones row lands on Docs
    /// › Milestones with milestone discovery scheduled on the real channel
    /// (D-B04) — driven through `App::update`, like the archive tests above.
    #[tokio::test]
    async fn enter_on_the_shipped_milestones_row_opens_docs_milestones() {
        let dir = tempfile::tempdir().expect("temp dir");
        archive_fixture(dir.path(), "01-core-flow");
        let planning = dir.path().join(".planning");
        std::fs::write(
            planning.join("ROADMAP.md"),
            "# Roadmap\n\n\
             ## Milestones\n\n\
             - \u{2705} **v1.1 First** -- Phases 1-2 (shipped 2026-01-10)\n\
             - \u{2705} **v1.2 Second** -- Phases 3-4 (shipped 2026-02-10)\n\
             - \u{1F6A7} **v1.3 Third** -- Phases 5-6 (in progress)\n\n\
             ## Phases\n\n\
             - [x] **Phase 1: One** - first\n\
             - [x] **Phase 2: Two** - second\n\
             - [x] **Phase 3: Three** - third\n\
             - [x] **Phase 4: Four** - fourth\n\
             - [ ] **Phase 5: Five** - fifth\n\
             - [ ] **Phase 6: Six** - sixth\n\n\
             ### Phase 5: Five\n**Goal**: the fifth\n\n\
             ### Phase 6: Six\n**Goal**: the sixth\n**Depends on**: Phase 5\n",
        )
        .expect("ROADMAP.md");
        std::fs::write(
            planning.join("STATE.md"),
            "---\nmilestone: v1.3\nmilestone_name: Third\ncurrent_phase: 5\nstatus: planning\n---\n\n# Project State\n",
        )
        .expect("STATE.md");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.project_states.insert(
            OBS_ALIAS.to_string(),
            crate::state_reader::parse_project_state(&planning),
        );

        let screen = crate::ui::screens::detail::DetailScreen::opened_on(
            OBS_ALIAS.to_string(),
            DetailSubView::RoadmapViz,
            &mut app.ctx,
        );
        app.screen_stack.push(Box::new(screen));
        // `g` puts the cursor on the first row: the shipped summary.
        press(&mut app, KeyCode::Char('g'));
        press(&mut app, KeyCode::Enter);

        assert_eq!(
            app.ctx.detail_sub_view_per_project.get(OBS_ALIAS),
            Some(&DetailSubView::Archive)
        );
        assert!(
            pump_archive_until(&mut app, &mut rx, is_discovery_for(OBS_ALIAS)).await,
            "milestone discovery for {OBS_ALIAS} never arrived"
        );
        let after = render_top_screen(&app);
        assert!(after.contains("[Milestones]"), "{after}");
        assert!(after.contains("v1.2"), "{after}");
    }

    // ── Running agents on the dashboard (AGENT-05, plan 25-04) ─────────────

    /// A `ProjectState` whose phase 13 has three plans in two waves — w1:
    /// 13-01, w2: 13-02 and 13-03 — with 13-01's SUMMARY paired in main.
    fn wave_state() -> ProjectState {
        use crate::state_reader::disk_status::DiskInference;
        use crate::state_reader::plan_waves::PlanWave;

        let mut state = ProjectState::default();
        state.phase_disk_statuses.insert(
            "13".to_string(),
            DiskInference {
                plan_waves: vec![
                    PlanWave {
                        wave: Some(1),
                        plans: vec!["13-01".to_string()],
                    },
                    PlanWave {
                        wave: Some(2),
                        plans: vec!["13-02".to_string(), "13-03".to_string()],
                    },
                ],
                summarized_plans: vec!["13-01".to_string()],
                plan_count: 3,
                summary_count: 1,
                ..Default::default()
            },
        );
        state
    }

    /// One scan holding one `Live` executor worktree on plan 13-02.
    fn live_executor_scan(alias: &str) -> Action {
        use crate::agents::{waves::PlanRef, AgentLiveness, AgentRow, ProjectAgents};

        let row = AgentRow {
            path: PathBuf::from("/nonexistent/.claude/worktrees/agent-a1b2c3d4"),
            liveness: AgentLiveness::Live,
            plan: PlanRef::from_id("13-02"),
            ..Default::default()
        };
        Action::AgentsScanned {
            per_project: vec![(
                alias.to_string(),
                ProjectAgents {
                    rows: vec![row],
                    scanned_at: Some(std::time::SystemTime::UNIX_EPOCH),
                    ..Default::default()
                },
            )],
        }
    }

    /// The tracer: a scan result for a live executor turns into a derived wave
    /// view and a fitted summary in that project's dashboard Status cell.
    #[tokio::test]
    async fn an_agents_scan_reaches_the_dashboard_status_cell() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .project_states
            .insert(OBS_ALIAS.to_string(), wave_state());

        app.update(live_executor_scan(OBS_ALIAS));
        app.ctx.recompute_filtered_aliases();

        assert!(
            app.ctx.agent_views.contains_key(OBS_ALIAS),
            "the handler must derive a view for a registered alias with a row"
        );
        let screen = render_top_screen(&app);
        // 120 columns: the Status column is 13 cells, so the ladder settles on
        // its fourth form (12 cells); the 17-cell third form does not fit.
        assert!(
            screen.contains("w2/2 \u{b7} 1 run"),
            "the Status cell must carry the fitted agent summary:\n{screen}"
        );
        assert!(
            !screen.contains("w2/2 \u{b7} 1 run \u{b7}"),
            "a wider form must never be clipped into the cell:\n{screen}"
        );
    }

    /// Quick 260929-szq: a project whose scan holds unmerged worktree work but
    /// no agent row still gets a view, and its Status cell shows `◐N`.
    #[tokio::test]
    async fn an_unmerged_only_scan_reaches_the_dashboard_status_cell() {
        use crate::agents::unmerged::{UnmergedItem, UnmergedKey, UnmergedState};
        use crate::agents::{waves::PlanRef, ProjectAgents};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .project_states
            .insert(OBS_ALIAS.to_string(), wave_state());

        let item = UnmergedItem {
            key: UnmergedKey::Plan(PlanRef::from_id("13-02").expect("valid id")),
            worktree: PathBuf::from("/nonexistent/wt"),
            branch: None,
            short_ref: crate::text::Untrusted::from_untrusted_source("a1b2c3d".into()),
            commits_ahead: Some(1),
            dirty: Some(0),
            state: UnmergedState::AwaitingMerge,
        };
        app.update(Action::AgentsScanned {
            per_project: vec![(
                OBS_ALIAS.to_string(),
                ProjectAgents {
                    unmerged: vec![item],
                    scanned_at: Some(std::time::SystemTime::UNIX_EPOCH),
                    ..Default::default()
                },
            )],
        });
        app.ctx.recompute_filtered_aliases();

        assert!(
            app.ctx.agent_views.contains_key(OBS_ALIAS),
            "unmerged work alone must give the alias a view"
        );
        let screen = render_top_screen(&app);
        assert!(
            screen.contains("\u{25D0}1"),
            "the Status cell must carry the unmerged count:\n{screen}"
        );
    }

    /// Quick 261014-0jv: a scan holding only worktree-held plans still gets a
    /// view (the Phases pane reads it), but the dashboard Status cell stays
    /// exactly as it was.
    #[tokio::test]
    async fn a_worktree_plans_only_scan_gets_a_view_but_leaves_the_status_cell_alone() {
        use crate::agents::worktree_plans::WorktreePlans;
        use crate::agents::ProjectAgents;
        use crate::text::Untrusted;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .project_states
            .insert(OBS_ALIAS.to_string(), wave_state());
        app.ctx.recompute_filtered_aliases();
        let before = render_top_screen(&app);

        let wp = WorktreePlans {
            key: "15".to_string(),
            worktree: PathBuf::from("/nonexistent/lanes-15"),
            phase_dir: PathBuf::from("/nonexistent/lanes-15/.planning/phases/15-x"),
            worktree_name: Untrusted::from_untrusted_source("lanes-15".into()),
            branch: Some(Untrusted::from_untrusted_source("gsd/phase-15".into())),
            short_ref: Untrusted::from_untrusted_source("phase-15".into()),
            others: 0,
            inference: Default::default(),
        };
        app.update(Action::AgentsScanned {
            per_project: vec![(
                OBS_ALIAS.to_string(),
                ProjectAgents {
                    worktree_plans: vec![wp.clone()],
                    scanned_at: Some(std::time::SystemTime::UNIX_EPOCH),
                    ..Default::default()
                },
            )],
        });
        app.ctx.recompute_filtered_aliases();

        let view = app
            .ctx
            .agent_views
            .get(OBS_ALIAS)
            .expect("worktree plans alone must give the alias a view");
        assert_eq!(view.worktree_plans, vec![wp]);
        assert_eq!(
            render_top_screen(&app),
            before,
            "the dashboard row must be byte-identical"
        );
    }

    /// Wait up to five seconds for an `AgentsScanned`, draining every other
    /// action the 20-tick block produces (sessions, reconciliation) meanwhile.
    async fn next_agents_scan(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        within: std::time::Duration,
    ) -> Option<Action> {
        tokio::time::timeout(within, async {
            while let Some(action) = rx.recv().await {
                if matches!(action, Action::AgentsScanned { .. }) {
                    return Some(action);
                }
            }
            None
        })
        .await
        .ok()
        .flatten()
    }

    #[tokio::test]
    async fn the_agents_scan_rides_the_session_poll_counter() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());

        for _ in 0..19 {
            app.update(Action::Tick);
        }
        assert!(
            !app.ctx.agents_scan_in_flight,
            "no scan before the 20th tick: the scan has no timer of its own"
        );
        app.update(Action::Tick);
        assert!(
            app.ctx.agents_scan_in_flight,
            "the 20th tick sets the flag before the spawn"
        );

        let scan = next_agents_scan(&mut rx, std::time::Duration::from_secs(5))
            .await
            .expect("an AgentsScanned arrives within 5 s");
        let Action::AgentsScanned { ref per_project } = scan else {
            unreachable!("next_agents_scan returns AgentsScanned only");
        };
        assert_eq!(
            per_project
                .iter()
                .map(|(a, _)| a.as_str())
                .collect::<Vec<_>>(),
            vec![OBS_ALIAS],
            "the scan covers the registered projects"
        );

        app.update(scan);
        assert!(
            !app.ctx.agents_scan_in_flight,
            "the handler clears the flag"
        );
        assert!(
            app.ctx.agent_views.is_empty(),
            "a non-git directory has no agent rows, so no view"
        );
    }

    #[tokio::test]
    async fn no_second_agents_scan_spawns_while_one_is_in_flight() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.agents_scan_in_flight = true;

        for _ in 0..20 {
            app.update(Action::Tick);
        }

        assert!(
            next_agents_scan(&mut rx, std::time::Duration::from_millis(500))
                .await
                .is_none(),
            "a scan already in flight must not be joined by a second one"
        );
        assert!(app.ctx.agents_scan_in_flight, "only the handler clears it");
    }

    #[test]
    fn the_agents_scan_handler_clears_the_in_flight_flag_for_an_empty_result() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.agents_scan_in_flight = true;

        // What the blocking closure sends after a panic.
        app.update(Action::AgentsScanned {
            per_project: Vec::new(),
        });

        assert!(!app.ctx.agents_scan_in_flight);
        assert!(app.ctx.agent_views.is_empty());
    }

    #[test]
    fn an_unchanged_agents_scan_does_not_request_a_redraw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .project_states
            .insert(OBS_ALIAS.to_string(), wave_state());

        app.needs_redraw = false;
        app.update(live_executor_scan(OBS_ALIAS));
        assert!(
            app.needs_redraw,
            "control arm: the first scan with an agent changes the map"
        );

        app.needs_redraw = false;
        app.update(live_executor_scan(OBS_ALIAS));
        assert!(
            !app.needs_redraw,
            "the same scan applied again must not repaint the frame"
        );
    }

    #[test]
    fn an_agents_scan_for_an_unregistered_alias_is_dropped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(live_executor_scan("gone"));

        assert!(
            app.ctx.agent_views.is_empty(),
            "an alias the registry does not hold must not come back through a scan"
        );

        // Control arm: the same scan for the registered alias does land.
        app.update(live_executor_scan(OBS_ALIAS));
        assert_eq!(
            app.ctx.agent_views.keys().collect::<Vec<_>>(),
            vec![OBS_ALIAS]
        );
    }

    #[test]
    fn a_project_with_no_agent_rows_gets_no_view() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(live_executor_scan(OBS_ALIAS));
        assert!(app.ctx.agent_views.contains_key(OBS_ALIAS));

        // The agents ended: the scan reports the project with nothing in it,
        // and the wholesale replacement drops its view.
        app.needs_redraw = false;
        app.update(Action::AgentsScanned {
            per_project: vec![(
                OBS_ALIAS.to_string(),
                crate::agents::ProjectAgents::default(),
            )],
        });

        assert!(app.ctx.agent_views.is_empty());
        assert!(app.needs_redraw, "the row reverts, so the frame repaints");
    }

    // ── Mouse (quick 260926-dyf) ────────────────────────────────────────

    fn mouse_app(aliases: &[&str]) -> App {
        use crate::config::RegisteredProject;
        let mut app = App::new_for_test();
        for alias in aliases {
            app.ctx.config.projects.insert(
                (*alias).to_string(),
                RegisteredProject {
                    path: PathBuf::from("/nonexistent").join(alias),
                    added: "2026-09-26".to_string(),
                    driver_opt_in: None,
                    extra: Default::default(),
                },
            );
        }
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));
        app
    }

    /// Draw the app exactly as the binary does and return the terminal row that
    /// shows `needle`.
    fn mouse_draw_and_find(app: &mut App, needle: &str) -> u16 {
        use ratatui::backend::TestBackend;
        let mut terminal =
            ratatui::Terminal::new(TestBackend::new(100, 30)).expect("TestBackend terminal");
        terminal
            .draw(|frame| crate::ui::render(frame, app))
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        (0..30u16)
            .find(|&y| {
                (0..100u16)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    .contains(needle)
            })
            .expect("the row is drawn")
    }

    fn left_press(column: u16, row: u16) -> Action {
        Action::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn mouse_toggle_key_flips_capture_and_says_so() {
        let mut app = App::new_for_test();
        assert!(app.mouse_capture, "the default config turns capture on");
        press(&mut app, KeyCode::Char('M'));
        assert!(!app.mouse_capture);
        let msg = app
            .ctx
            .status_message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default();
        assert!(msg.contains("Mouse off"), "{msg}");
        press(&mut app, KeyCode::Char('M'));
        assert!(app.mouse_capture);
        let msg = app
            .ctx
            .status_message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default();
        assert!(msg.contains("Shift+drag"), "{msg}");

        // In the `/` search, `M` is text.
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('M'));
        assert_eq!(app.ctx.filter_text, "M");
        assert!(app.mouse_capture, "a typed M must not toggle capture");
    }

    #[test]
    fn mouse_preference_false_starts_with_capture_off() {
        let mut config = Config::new();
        config.preferences.mouse = false;
        let app = App::from_config(config, PathBuf::from("/nonexistent/config.json"));
        assert!(!app.mouse_capture);
    }

    #[test]
    fn mouse_events_are_ignored_while_capture_is_off() {
        let mut app = mouse_app(&["proja", "projb", "projc"]);
        let row = mouse_draw_and_find(&mut app, "projc");
        app.mouse_capture = false;
        app.update(left_press(5, row));
        assert_eq!(app.ctx.table_state.selected(), Some(0));
        // And with capture on, the same press selects it.
        app.mouse_capture = true;
        app.update(left_press(5, row));
        assert_eq!(app.ctx.table_state.selected(), Some(2));
    }

    // ── Launch target (quick 260930-vvk) ────────────────────────────────

    #[test]
    fn launch_target_open_project_view_puts_the_detail_view_on_top() {
        let mut app = mouse_app(&["proja", "projb", "projc"]);
        assert!(app.open_project_view("projb"), "a registered alias opens");
        assert_eq!(app.screen_stack.len(), 2, "overview + project view");
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some(crate::ui::screens::detail::DetailScreen::NAME)
        );
        assert_eq!(app.ctx.selected_alias().as_deref(), Some("projb"));
    }

    #[test]
    fn launch_target_esc_matches_a_drilled_in_detail_view() {
        fn esc_presses_to_overview(app: &mut App) -> usize {
            let mut presses = 0;
            while app.screen_stack.len() > 1 && presses < 5 {
                press(app, KeyCode::Esc);
                presses += 1;
            }
            presses
        }

        let mut launched = mouse_app(&["proja", "projb", "projc"]);
        assert!(launched.open_project_view("projb"));

        let mut drilled = mouse_app(&["proja", "projb", "projc"]);
        press(&mut drilled, KeyCode::Char('j'));
        press(&mut drilled, KeyCode::Enter);
        assert_eq!(drilled.screen_stack.len(), 2, "j + Enter drilled in");

        let launched_presses = esc_presses_to_overview(&mut launched);
        let drilled_presses = esc_presses_to_overview(&mut drilled);
        assert_eq!(launched.screen_stack.len(), 1, "Esc reached the overview");
        assert_eq!(drilled.screen_stack.len(), 1, "Esc reached the overview");
        assert_eq!(
            launched_presses, drilled_presses,
            "a launched project view must take exactly as many Esc presses as a drilled-in one"
        );
        for app in [&launched, &drilled] {
            assert_eq!(app.ctx.table_state.selected(), Some(1));
            assert_eq!(app.ctx.selected_alias().as_deref(), Some("projb"));
        }
    }

    #[test]
    fn launch_target_open_project_view_refuses_an_absent_alias() {
        let mut app = mouse_app(&["proja", "projb", "projc"]);
        assert!(!app.open_project_view("absent"));
        assert_eq!(app.screen_stack.len(), 1, "nothing was pushed");
    }

    #[test]
    fn mouse_app_turns_a_quick_second_press_into_enter() {
        let mut app = mouse_app(&["proja", "projb", "projc"]);
        let row = mouse_draw_and_find(&mut app, "projc");
        app.update(left_press(5, row));
        app.update(left_press(5, row));
        assert_eq!(
            app.screen_stack.len(),
            2,
            "the double-click opened the detail view"
        );
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("detail")
        );

        // A key between the two presses prevents the double.
        let mut app = mouse_app(&["proja", "projb", "projc"]);
        let row = mouse_draw_and_find(&mut app, "projc");
        app.update(left_press(5, row));
        press(&mut app, KeyCode::Char('j'));
        app.update(left_press(5, row));
        assert_eq!(app.screen_stack.len(), 1);
        assert_eq!(app.ctx.table_state.selected(), Some(2));
    }

    // ── 30-03: the dashboard's launcher-run markers ─────────────────────────────

    /// A launcher run dir under `root` whose frontmatter says `running` with one
    /// launch and whose run lock is NOT held: the launcher died without a final
    /// status. Heartbeats are fresh, so the only thing wrong with it is the lock.
    fn write_dead_launcher_run(root: &std::path::Path, run_id: &str) {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, RunStatus, DRIVER_RUNS_REL,
        };
        let run_dir = root.join(DRIVER_RUNS_REL).join(run_id);
        std::fs::create_dir_all(&run_dir).expect("run dir");
        let mut frontmatter = parse_driver_frontmatter(include_str!(
            "../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        frontmatter.run_id = run_id.to_string();
        frontmatter.status = RunStatus::Running;
        frontmatter.status_reason = None;
        frontmatter.launches = 1;
        crate::ui::screens::driver_run::refresh_heartbeats(&mut frontmatter);
        std::fs::write(
            run_dir.join("DIRECTION.md"),
            format!(
                "{}{}",
                frontmatter.emit(),
                crate::driver_templates::DIRECTION_TEMPLATE
            ),
        )
        .expect("DIRECTION.md");
    }

    /// An `App` with two registered projects, `alpha` and `beta`, rooted at the given
    /// directories.
    fn two_project_app(alpha: &std::path::Path, beta: &std::path::Path) -> App {
        use crate::config::RegisteredProject;
        let mut app = App::new_for_test();
        for (alias, root) in [("alpha", alpha), ("beta", beta)] {
            app.ctx.config.projects.insert(
                alias.to_string(),
                RegisteredProject {
                    path: root.to_path_buf(),
                    added: "2026-07-29".to_string(),
                    driver_opt_in: None,
                    extra: Default::default(),
                },
            );
        }
        app.ctx.recompute_filtered_aliases();
        app
    }

    /// The tracer: a launcher run that died without a final status, in one of two
    /// registered projects, shows `✗ ` on that project's dashboard row and is the only
    /// row `//h` lists. With the experimental flag off neither changes.
    #[cfg(unix)]
    #[tokio::test]
    async fn launcher_marker_a_crashed_run_lights_the_badge_and_the_h_filter() {
        let alpha = tempfile::tempdir().expect("alpha dir");
        let beta = tempfile::tempdir().expect("beta dir");
        write_dead_launcher_run(alpha.path(), LAUNCHER_RUN);
        let mut app = two_project_app(alpha.path(), beta.path());

        // Before the first scan lands nothing is guessed: no badge, nothing listed.
        app.ctx.filter_text = "/h".to_string();
        app.ctx.recompute_filtered_aliases();
        assert!(app.ctx.filtered_aliases.is_empty());
        assert_eq!(
            crate::ui::screens::normal::row_badge(&app.ctx, "alpha"),
            None
        );

        let markers = launcher_markers_for(&app.ctx.config.projects, std::time::SystemTime::now());
        app.update(Action::RunsReconciled {
            runs: Vec::new(),
            last_outcomes: HashMap::new(),
            launcher_markers: markers,
        });

        let badge = crate::ui::screens::normal::row_badge(&app.ctx, "alpha").expect("a badge");
        assert_eq!(badge.glyph, "\u{2717} ");
        assert_eq!(badge.color, ratatui::style::Color::Red);
        assert_eq!(badge.modifier, ratatui::style::Modifier::BOLD);
        assert_eq!(
            crate::ui::screens::normal::row_badge(&app.ctx, "beta"),
            None
        );
        // The handler recomputed the filtered rows when the marker moved.
        assert_eq!(app.ctx.filtered_aliases, vec!["alpha".to_string()]);
        assert!(app.ctx.needs_attention_for("alpha"));
        assert!(!app.ctx.needs_attention_for("beta"));

        // Flag off: no launcher surface, so neither the badge nor the filter changes.
        app.ctx.experimental = false;
        app.ctx.recompute_filtered_aliases();
        assert_eq!(
            crate::ui::screens::normal::row_badge(&app.ctx, "alpha"),
            None
        );
        assert!(app.ctx.filtered_aliases.is_empty());
        assert!(!app.ctx.needs_attention_for("alpha"));
    }

    /// The scan rides the 20-tick reconcile task: on the twentieth tick with the flag
    /// on the `RunsReconciled` it sends carries the marker, and with the flag off no
    /// launcher scan runs, so the map is empty.
    #[cfg(unix)]
    #[tokio::test]
    async fn launcher_marker_the_slow_tick_scans_only_with_the_flag_on() {
        let alpha = tempfile::tempdir().expect("alpha dir");
        let beta = tempfile::tempdir().expect("beta dir");
        write_dead_launcher_run(alpha.path(), LAUNCHER_RUN);

        for experimental in [true, false] {
            let mut app = two_project_app(alpha.path(), beta.path());
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            app.ctx.event_tx = Some(tx);
            app.ctx.experimental = experimental;
            app.session_poll_counter = 19;
            app.update(Action::Tick);

            let mut markers = None;
            for _ in 0..300 {
                while let Ok(action) = rx.try_recv() {
                    if let Action::RunsReconciled {
                        launcher_markers, ..
                    } = action
                    {
                        markers = Some(launcher_markers);
                    }
                }
                if markers.is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let markers = markers.expect("the slow tick sends RunsReconciled");
            if experimental {
                assert_eq!(
                    markers.get("alpha").and_then(|m| m.class),
                    Some(crate::ui::screens::driver_run::MarkerClass::Crashed)
                );
                assert_eq!(markers.get("beta").and_then(|m| m.class), None);
            } else {
                assert!(markers.is_empty(), "no launcher scan with the flag off");
            }
        }
    }

    /// A `DriverRun` event for a project whose Driver tab is not on top is answered by
    /// a one-project marker scan (D-13): the badge changes without a slow tick, with no
    /// launcher listing and no re-parse. With the flag off it schedules nothing.
    #[cfg(unix)]
    #[tokio::test]
    async fn launcher_marker_a_driver_run_event_off_the_tab_rescans_the_one_project() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_dead_launcher_run(dir.path(), LAUNCHER_RUN);
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));

        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        let mut scanned = None;
        while let Some(action) = next_action(&mut rx).await {
            match action {
                Action::LauncherMarkerScanned { .. } => {
                    scanned = Some(action);
                    break;
                }
                Action::LauncherRunsRefreshed { .. } | Action::LauncherRunRefreshed { .. } => {
                    panic!("the tab is closed: no launcher listing may be scheduled")
                }
                _ => {}
            }
        }
        app.update(scanned.expect("the marker scan reports"));
        let marker = app.ctx.view_cache[OBS_ALIAS]
            .launcher_marker
            .expect("a marker");
        assert_eq!(
            marker.class,
            Some(crate::ui::screens::driver_run::MarkerClass::Crashed)
        );
        assert!(!marker.any_live);
        assert_eq!(
            crate::ui::screens::normal::row_badge(&app.ctx, OBS_ALIAS).map(|b| b.glyph),
            Some("\u{2717} ")
        );
        assert_eq!(app.ctx.reparse_dispatches, 0);

        // Flag off: no launcher scan is scheduled at all.
        app.ctx.experimental = false;
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err(), "no launcher surface, no scan");
    }

    /// With the Driver tab on top the refresh handlers recompute the marker from the
    /// rows they just stored, so no second listing is needed for the dashboard.
    #[cfg(unix)]
    #[tokio::test]
    async fn launcher_marker_the_open_tab_recomputes_the_marker_from_its_rows() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_dead_launcher_run(dir.path(), LAUNCHER_RUN);
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());

        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .launcher_marker
                .and_then(|m| m.class),
            Some(crate::ui::screens::driver_run::MarkerClass::Crashed)
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        while let Ok(action) = rx.try_recv() {
            assert!(
                !matches!(action, Action::LauncherMarkerScanned { .. }),
                "the open tab needs no separate marker scan"
            );
        }
    }

    /// A late marker scan for a project that has since been removed leaves no cache
    /// entry behind, and one that changes nothing does not redraw.
    #[tokio::test]
    async fn launcher_marker_a_scan_for_an_unregistered_project_is_dropped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        let marker = crate::ui::screens::driver_run::LauncherMarker {
            class: Some(crate::ui::screens::driver_run::MarkerClass::Stale),
            any_live: true,
        };
        app.update(Action::LauncherMarkerScanned {
            alias: "gone".to_string(),
            marker,
        });
        assert!(!app.ctx.view_cache.contains_key("gone"));

        app.needs_redraw = false;
        app.update(Action::LauncherMarkerScanned {
            alias: OBS_ALIAS.to_string(),
            marker,
        });
        assert!(app.needs_redraw);
        app.needs_redraw = false;
        app.update(Action::LauncherMarkerScanned {
            alias: OBS_ALIAS.to_string(),
            marker,
        });
        assert!(!app.needs_redraw, "an unchanged marker repaints nothing");
    }

    // ── 30-04: steering a launcher run from the Driver tab ──────────────────

    /// Wait for the next action `pick` accepts, skipping the others (refreshes the
    /// earlier steps scheduled arrive in between). Linux-only with every caller, so
    /// it is not dead code under `-D warnings` elsewhere.
    #[cfg(target_os = "linux")]
    async fn next_matching(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        pick: impl Fn(&Action) -> bool,
    ) -> Action {
        loop {
            match next_action(rx).await {
                Some(action) if pick(&action) => return action,
                Some(_) => {}
                None => panic!("the awaited action never arrived"),
            }
        }
    }

    /// A launcher run that is `running` in its frontmatter, with the template body, so
    /// `control: run` is both written and echoed.
    #[cfg(target_os = "linux")]
    fn write_live_run(root: &std::path::Path) -> (std::path::PathBuf, String) {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, RunStatus, DRIVER_RUNS_REL,
        };
        let run_dir = root.join(DRIVER_RUNS_REL).join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).expect("run dir");
        let mut frontmatter = parse_driver_frontmatter(include_str!(
            "../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        frontmatter.run_id = LAUNCHER_RUN.to_string();
        frontmatter.status = RunStatus::Running;
        frontmatter.status_reason = None;
        frontmatter.control = Some("run".to_string());
        crate::ui::screens::driver_run::refresh_heartbeats(&mut frontmatter);
        let block = frontmatter.emit();
        std::fs::write(
            run_dir.join("DIRECTION.md"),
            format!("{block}{}", crate::driver_templates::DIRECTION_TEMPLATE),
        )
        .expect("DIRECTION.md");
        (run_dir, block)
    }

    /// The tracer, end to end (D-05, D-14, D-15, D-28): on a live run, `c` then `p`
    /// writes `control: pause` into the body through `write_body`, the tool block is
    /// untouched, the status line says it was WRITTEN, and the header reads
    /// `pause (pending)` until the driver echoes it. Nothing says `paused`.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_c_then_p_shows_pending() {
        use crate::driver::lock::{acquire_at, run_lock_path};
        let dir = tempfile::tempdir().expect("temp dir");
        let (run_dir, block) = write_live_run(dir.path());
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        let _lock = acquire_at(&run_lock_path(&run_dir), LAUNCHER_RUN, std::process::id())
            .expect("the test takes the run lock");

        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("\u{25C6} running"), "a live run:\n{text}");
        assert!(!text.contains("(pending)"), "nothing pending yet:\n{text}");

        // `c` opens the chooser, `p` dispatches and pops.
        app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("driver_control")
        );
        let text = render_top_screen(&app);
        assert!(text.contains("control (now: run)>"), "{text}");
        app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("detail")
        );
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverSteerRequested { .. })
        })
        .await;
        app.update(request);
        let steered = next_matching(&mut rx, |a| matches!(a, Action::DriverSteered { .. })).await;
        let Action::DriverSteered { outcome, .. } = &steered else {
            unreachable!()
        };
        assert_eq!(
            *outcome,
            crate::driver::steer::SteerOutcome::Written {
                lock_timed_out: false
            }
        );
        app.update(steered);
        let (status, _) = app.ctx.status_message.clone().expect("a status message");
        assert_eq!(
            status,
            "control: pause written to DIRECTION.md. The driver applies it before its next dispatch."
        );

        // The write reached the body and only the body.
        let written = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&written);
        assert_eq!(fm, Some(block.as_str()));
        assert_eq!(
            crate::driver::direction::parse_direction(body)
                .control
                .as_deref(),
            Some("pause")
        );

        // The finished write scheduled a re-read of the selected run: the header
        // shows the pending word, never `paused`.
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("pause (pending)"), "{text}");
        assert!(!text.contains("paused"), "{text}");
        assert!(!text.contains("stopped"), "{text}");
        assert_eq!(app.ctx.reparse_dispatches, 0, "no project re-parse");
    }

    /// A build without the write primitive refuses every steering request with the
    /// read-only copy and writes nothing (D-14).
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn launcher_steer_a_non_linux_build_refuses_with_the_read_only_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(Action::DriverSteerRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            op: crate::driver::steer::SteerOp::SetControl("pause".to_string()),
        });
        assert_eq!(
            app.ctx.error_message.as_deref(),
            Some("read-only: steering needs Linux")
        );
    }

    /// With no event channel the result could not be reported, so the request is
    /// refused visibly and nothing is written (S4).
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_without_an_event_channel_refuses_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (run_dir, block) = write_live_run(dir.path());
        let (mut app, rx) = obs_app(dir.path());
        drop(rx);
        app.ctx.event_tx = None;
        app.update(Action::DriverSteerRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            op: crate::driver::steer::SteerOp::SetControl("pause".to_string()),
        });
        let message = app.ctx.error_message.clone().expect("a visible refusal");
        assert!(message.contains("Nothing was written"), "{message}");
        let text = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        assert!(text.starts_with(&block));
        assert_eq!(
            crate::driver::direction::parse_direction(
                crate::driver::direction::split_frontmatter(&text).1
            )
            .control
            .as_deref(),
            Some("run")
        );
    }

    /// A run id that is not a plain directory name never names a directory.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_a_run_id_with_a_separator_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.update(Action::DriverSteerRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: "../escape".to_string(),
            op: crate::driver::steer::SteerOp::SetControl("pause".to_string()),
        });
        let message = app.ctx.error_message.clone().expect("a visible refusal");
        assert!(message.contains("not a run directory name"), "{message}");
        assert!(rx.try_recv().is_err(), "nothing was scheduled");
    }

    /// D-15 and D-16 end to end: `a` on the parked cursor item opens the answer input,
    /// Enter writes `- answer:` and `- hold: false` into that item's segment through
    /// `write_body`, every item survives, and when the LOG recorded a park rule an
    /// answer does not lift, the status says so.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_a_then_enter_answers_the_parked_item_and_warns_about_the_rule() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let run_dir = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        let before = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let (fm_before, body_before) = crate::driver::direction::split_frontmatter(&before);
        let ids_before: Vec<String> = crate::driver::steer::read_items(body_before)
            .into_iter()
            .map(|i| i.id)
            .collect();

        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        // The LOG recorded that I-01 parked by a rule an answer does not lift.
        {
            let cache = app.ctx.view_cache.get_mut(OBS_ALIAS).expect("cache");
            cache.launcher_log.run_id = LAUNCHER_RUN.to_string();
            cache
                .launcher_log
                .park_rules
                .insert("I-01".to_string(), "gap-rounds".to_string());
        }

        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("driver_answer")
        );
        let text = render_top_screen(&app);
        assert!(
            text.contains("I-01 is parked by gap-rounds: an answer alone will not resume it."),
            "{text}"
        );
        for ch in "raise the budget".chars() {
            app.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("detail")
        );
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverSteerRequested { .. })
        })
        .await;
        app.update(request);
        let steered = next_matching(&mut rx, |a| matches!(a, Action::DriverSteered { .. })).await;
        app.update(steered);

        let (status, _) = app.ctx.status_message.clone().expect("a status message");
        assert!(
            status.starts_with("I-01 answered; gap-rounds parks it: [e] budget, [R] caps."),
            "{status}"
        );
        let after = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let (fm_after, body_after) = crate::driver::direction::split_frontmatter(&after);
        assert_eq!(fm_after, fm_before, "the tool block is byte-identical");
        let items = crate::driver::steer::read_items(body_after);
        assert_eq!(
            items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
            ids_before,
            "every item survives"
        );
        assert_eq!(items[0].answer.as_deref(), Some("raise the budget"));
        assert_eq!(items[0].hold.as_deref(), Some("false"));
        assert_eq!(items[1].answer, None, "the other item is untouched");
    }

    /// `u` writes `hold: false` at once for the held cursor item, with no screen between.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_u_takes_the_held_item_off_hold() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "\n### I-03 third thing\n\n- hold: true\n");
        let run_dir = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        // Move the cursor from the parked I-01 to the held I-03.
        for _ in 0..2 {
            app.handle_key(KeyCode::Char(']'), KeyModifiers::NONE);
        }
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .launcher_item_cursor
                .as_deref(),
            Some("I-03")
        );
        app.handle_key(KeyCode::Char('u'), KeyModifiers::NONE);
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverSteerRequested { .. })
        })
        .await;
        app.update(request);
        let steered = next_matching(&mut rx, |a| matches!(a, Action::DriverSteered { .. })).await;
        app.update(steered);
        assert_eq!(
            app.ctx.status_message.clone().expect("a status message").0,
            "I-03: hold set to false. The driver re-reads it before its next dispatch."
        );
        let text = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let items =
            crate::driver::steer::read_items(crate::driver::direction::split_frontmatter(&text).1);
        assert_eq!(items[2].id, "I-03");
        assert!(!items[2].is_on_hold());
    }

    /// A failed write is shown as an error that says nothing changed (the UI error
    /// row): here the item vanished from the body between the key and the write.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_steer_a_refused_write_is_an_error_not_a_success() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_items_run(dir.path(), "");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.update(Action::DriverSteerRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            op: crate::driver::steer::SteerOp::Unhold {
                item: "I-77".to_string(),
            },
        });
        let steered = next_matching(&mut rx, |a| matches!(a, Action::DriverSteered { .. })).await;
        app.update(steered);
        let message = app.ctx.error_message.clone().expect("an error message");
        assert!(
            message.starts_with("Nothing was written: I-77"),
            "{message}"
        );
        assert!(app.ctx.status_message.is_none(), "not reported as success");
    }

    // ── 30-04: editing DIRECTION.md in the user's editor ────────────────────

    /// The path of run `LAUNCHER_RUN`'s edit temp under `root`.
    #[cfg(target_os = "linux")]
    fn edit_temp(root: &std::path::Path) -> PathBuf {
        root.join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN)
            .join(crate::driver::steer::EDIT_TEMP_NAME)
    }

    /// A launcher run holding a lint-clean body (so an edit applies without the
    /// confirmation), the tab open on it with its detail loaded.
    #[cfg(target_os = "linux")]
    async fn edit_fixture(
        root: &std::path::Path,
    ) -> (
        App,
        tokio::sync::mpsc::UnboundedReceiver<Action>,
        PathBuf,
        String,
    ) {
        let run_dir = root
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).unwrap();
        let block = crate::driver::steer::tests::write_run(
            &run_dir,
            &crate::driver::steer::tests::clean_body(),
        );
        let (mut app, mut rx) = app_on_the_driver_tab(root);
        app.update(run_file_change(root, "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        (app, rx, run_dir, block)
    }

    /// `e`, through the key handler and the App, until the editor would open: the
    /// temp exists, holds the body, and `pending_editor` names it.
    #[cfg(target_os = "linux")]
    async fn open_the_editor(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) -> PathBuf {
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        let request = next_matching(rx, |a| matches!(a, Action::DriverEditRequested { .. })).await;
        app.update(request);
        let ready = next_matching(rx, |a| matches!(a, Action::DriverEditReady { .. })).await;
        app.update(ready);
        let (temp, line) = app.pending_editor.take().expect("the editor is requested");
        assert_eq!(line, None);
        temp
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_e_hands_the_body_only_temp_to_the_editor_and_applies_a_clean_save() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();

        let temp = open_the_editor(&mut app, &mut rx).await;
        assert_eq!(temp, edit_temp(dir.path()));
        assert_eq!(std::fs::read_to_string(&temp).unwrap(), clean);
        let session = app.edit_sessions.get(&temp).expect("a session").clone();
        assert_eq!(session.base, clean);
        assert_eq!(session.run_dir, run_dir);

        // The editor saved a change (simulated: no process is spawned in src/).
        let edited = clean.replace("Prove the edit flow.", "Prove the edit flow twice.");
        std::fs::write(&temp, &edited).unwrap();
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);

        assert_eq!(
            app.ctx.status_message.clone().expect("status").0,
            "DIRECTION.md updated. The driver re-reads it before its next dispatch."
        );
        let text = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()), "the tool block is byte-identical");
        assert_eq!(body, edited);
        assert!(!temp.exists(), "an applied temp is removed");
        assert!(app.edit_sessions.is_empty(), "and the session ends");
        // The run is re-read, so the new body shows without waiting for the watcher.
        assert!(matches!(
            next_matching(&mut rx, |a| matches!(
                a,
                Action::LauncherRunRefreshed { .. }
            ))
            .await,
            Action::LauncherRunRefreshed { .. }
        ));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_a_save_that_changed_nothing_writes_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
        let before = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let temp = open_the_editor(&mut app, &mut rx).await;
        // A GUI editor that returned at once looks exactly like this (Pitfall 7).
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        assert_eq!(
            app.ctx.status_message.clone().expect("status").0,
            "No changes to DIRECTION.md. Nothing was written."
        );
        assert!(!temp.exists());
        assert_eq!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap(),
            before
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_a_failed_editor_keeps_the_temp_and_writes_nothing() {
        for exit in [
            EditorExit::Failed("exit status: 1".to_string()),
            EditorExit::SpawnError("nano: No such file or directory (os error 2)".to_string()),
        ] {
            let dir = tempfile::tempdir().expect("temp dir");
            let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
            let before = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
            let temp = open_the_editor(&mut app, &mut rx).await;
            let edited = crate::driver::steer::tests::clean_body().replace("Prove it", "Mine");
            std::fs::write(&temp, &edited).unwrap();

            let status = match &exit {
                EditorExit::Failed(s) | EditorExit::SpawnError(s) => s.clone(),
                EditorExit::Success => unreachable!(),
            };
            app.after_editor(&temp, exit);
            assert_eq!(
                app.ctx.error_message.clone().expect("a message"),
                format!(
                    "The editor exited with {status}. Nothing was written. Your text is kept at {}.",
                    temp.display()
                )
            );
            assert!(temp.exists(), "the user's text is kept");
            assert_eq!(std::fs::read_to_string(&temp).unwrap(), edited);
            assert_eq!(
                std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap(),
                before,
                "nothing was written"
            );
            assert!(app.edit_sessions.is_empty());
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            while let Ok(action) = rx.try_recv() {
                assert!(
                    !matches!(action, Action::DriverEditDone { .. }),
                    "a failed editor dispatches no apply"
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_a_path_that_is_not_a_session_is_only_refreshed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, _run_dir, _block) = edit_fixture(dir.path()).await;
        app.after_editor(&dir.path().join("notes.md"), EditorExit::Success);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(
            rx.try_recv().is_err(),
            "no apply for a file we did not prepare"
        );
        assert!(app.ctx.error_message.is_none());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_a_conflict_opens_the_chooser_and_reopen_rebases_on_the_live_body() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();
        let temp = open_the_editor(&mut app, &mut rx).await;
        let mine = clean.replace("Prove it", "My title");
        std::fs::write(&temp, &mine).unwrap();

        // Someone else saves while the editor is open.
        let racing = clean.replace("It works.", "It works!");
        crate::driver::direction::write_body(&run_dir, |_| racing.clone()).unwrap();

        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("driver_edit_chooser")
        );
        let text = render_top_screen(&app);
        assert!(
            text.contains("Not applied: DIRECTION.md changed while you were editing."),
            "{text}"
        );
        assert!(temp.exists(), "the user's text is kept");
        assert!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md"))
                .unwrap()
                .ends_with(&racing),
            "the other change stands"
        );

        // `r`: the kept text is opened again against the body as it is NOW.
        app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        let reopen = next_matching(&mut rx, |a| matches!(a, Action::DriverEditReopen { .. })).await;
        app.update(reopen);
        let ready = next_matching(&mut rx, |a| matches!(a, Action::DriverEditReady { .. })).await;
        app.update(ready);
        let (reopened, _) = app.pending_editor.take().expect("the editor opens again");
        assert_eq!(reopened, temp);
        assert_eq!(
            app.edit_sessions[&temp].base, racing,
            "the live body is the new base"
        );
        assert_eq!(
            std::fs::read_to_string(&temp).unwrap(),
            mine,
            "the text is untouched"
        );

        // Saving now replaces the other change, which is what the chooser said.
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        let text = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()));
        assert_eq!(body, mine);
        assert!(!temp.exists());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_discard_removes_only_the_temp() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();
        let temp = open_the_editor(&mut app, &mut rx).await;
        std::fs::write(&temp, clean.replace("Prove it", "Mine")).unwrap();
        crate::driver::direction::write_body(&run_dir, |_| clean.replace("It works.", "Theirs"))
            .unwrap();
        let live = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);

        app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
        let discard =
            next_matching(&mut rx, |a| matches!(a, Action::DriverEditDiscard { .. })).await;
        app.update(discard);
        let discarded =
            next_matching(&mut rx, |a| matches!(a, Action::DriverEditDiscarded { .. })).await;
        app.update(discarded);
        assert!(!temp.exists());
        assert_eq!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap(),
            live,
            "DIRECTION.md is untouched"
        );
        assert_eq!(
            app.ctx.status_message.clone().expect("status").0,
            "Discarded your edit. DIRECTION.md was not changed."
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_lint_problems_ask_first_and_y_applies() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();
        let temp = open_the_editor(&mut app, &mut rx).await;
        // The template's placeholders are `driver check` problems.
        let placeholders = clean.replace(
            "Prove the edit flow.",
            "{one or two sentences: what this run is for}",
        );
        std::fs::write(&temp, &placeholders).unwrap();

        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("driver_edit_confirm")
        );
        let text = render_top_screen(&app);
        assert!(text.contains("Apply this edit?"), "{text}");
        assert!(text.contains("driver check found"), "{text}");
        assert!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md"))
                .unwrap()
                .ends_with(&clean),
            "nothing was written yet"
        );
        assert!(
            app.edit_sessions.contains_key(&temp),
            "the session waits for the answer"
        );

        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        let apply = next_matching(&mut rx, |a| matches!(a, Action::DriverEditApply { .. })).await;
        app.update(apply);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        assert!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md"))
                .unwrap()
                .ends_with(&placeholders),
            "applied anyway"
        );
        assert!(!temp.exists());
        assert!(app.edit_sessions.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_declining_the_lint_confirmation_opens_the_chooser_and_keeps_the_temp() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();
        let temp = open_the_editor(&mut app, &mut rx).await;
        std::fs::write(
            &temp,
            clean.replace(
                "Prove the edit flow.",
                "{one or two sentences: what this run is for}",
            ),
        )
        .unwrap();
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(
            app.screen_stack
                .last()
                .map(|s| s.name().to_string())
                .as_deref(),
            Some("driver_edit_chooser")
        );
        assert!(temp.exists());
        assert!(std::fs::read_to_string(run_dir.join("DIRECTION.md"))
            .unwrap()
            .ends_with(&clean));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_an_edit_without_the_section_headers_is_not_applied() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir, _block) = edit_fixture(dir.path()).await;
        let clean = crate::driver::steer::tests::clean_body();
        let temp = open_the_editor(&mut app, &mut rx).await;
        std::fs::write(&temp, clean.replace("## Items", "## Things")).unwrap();
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        let text = render_top_screen(&app);
        assert!(
            text.contains(
                "Not applied: the edited body is missing \"## Direction\" or \"## Items\"."
            ),
            "{text}"
        );
        assert!(temp.exists());
        assert!(std::fs::read_to_string(run_dir.join("DIRECTION.md"))
            .unwrap()
            .ends_with(&clean));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_a_temp_left_by_a_crashed_tui_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, _run_dir, _block) = edit_fixture(dir.path()).await;
        let temp = edit_temp(dir.path());
        std::fs::write(&temp, "the user's unsaved text").unwrap();
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        let request =
            next_matching(&mut rx, |a| matches!(a, Action::DriverEditRequested { .. })).await;
        app.update(request);
        let stale = next_matching(&mut rx, |a| matches!(a, Action::DriverEditStale { .. })).await;
        app.update(stale);
        assert!(app.pending_editor.is_none(), "no editor is opened");
        assert_eq!(
            std::fs::read_to_string(&temp).unwrap(),
            "the user's unsaved text"
        );
        let text = render_top_screen(&app);
        // The path is cut from the left to fit the row; its tail names the run.
        assert!(text.contains("A previous edit was not applied: "), "{text}");
        assert!(
            text.contains(&format!("{LAUNCHER_RUN}/.DIRECTION.md.edit.")),
            "{text}"
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_edit_apply_for_an_unknown_session_is_refused_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx, _run_dir, _block) = edit_fixture(dir.path()).await;
        app.update(Action::DriverEditApply {
            temp: dir.path().join("nope"),
            accept_lint: true,
        });
        let message = app.ctx.error_message.clone().expect("a refusal");
        assert!(message.contains("no longer open"), "{message}");
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn launcher_edit_a_non_linux_build_refuses_with_the_read_only_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(Action::DriverEditRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });
        assert_eq!(
            app.ctx.error_message.as_deref(),
            Some("read-only: steering needs Linux")
        );
        assert!(app.pending_editor.is_none());
    }

    // ── 30-05: launching and relaunching from the Driver tab ────────────────

    /// What the fake `launch` seam was given, per call: the options and the `--config`
    /// path. A `static` because the seam is a `fn` pointer and the call happens on a
    /// `spawn_blocking` thread; tests tell their own calls apart by the project path.
    #[cfg(target_os = "linux")]
    type RecordedLaunch = (crate::launcher::run::LaunchOptions, Option<PathBuf>);

    #[cfg(target_os = "linux")]
    static RECORDED_LAUNCHES: std::sync::Mutex<Vec<RecordedLaunch>> =
        std::sync::Mutex::new(Vec::new());

    #[cfg(target_os = "linux")]
    fn fake_launch(
        options: &crate::launcher::run::LaunchOptions,
        config: Option<&Path>,
    ) -> Result<u32, crate::launcher::LauncherError> {
        RECORDED_LAUNCHES
            .lock()
            .expect("the recorder")
            .push((options.clone(), config.map(Path::to_path_buf)));
        Ok(4242)
    }

    /// A host that passes every check, with the minimum claude.
    #[cfg(target_os = "linux")]
    fn fake_detect_ready() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 1, 295)),
        }
    }

    /// The launches the fake seam recorded for the project at `root`.
    #[cfg(target_os = "linux")]
    fn launches_for(root: &Path) -> Vec<RecordedLaunch> {
        RECORDED_LAUNCHES
            .lock()
            .expect("the recorder")
            .iter()
            .filter(|(options, _)| options.project == root)
            .cloned()
            .collect()
    }

    /// An `App` on the Driver tab over a crashed-looking launcher run holding a lint-clean
    /// body, with the fake launch seam and a host that passes, the run listed.
    #[cfg(target_os = "linux")]
    async fn launch_fixture(
        root: &Path,
    ) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>, PathBuf) {
        let run_dir = root
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).unwrap();
        crate::driver::steer::tests::write_run(
            &run_dir,
            &crate::driver::steer::tests::clean_body(),
        );
        let (mut app, mut rx) = app_on_the_driver_tab(root);
        app.launch_seams = LaunchSeams {
            launch: fake_launch,
            detect: fake_detect_ready,
        };
        app.update(run_file_change(root, "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        (app, rx, run_dir)
    }

    #[cfg(target_os = "linux")]
    fn top_screen_name(app: &App) -> String {
        app.screen_stack
            .last()
            .map(|s| s.name().to_string())
            .unwrap_or_default()
    }

    /// `R`, through the key handler and the App, until the form has its host check.
    #[cfg(target_os = "linux")]
    async fn open_the_launch_form(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) {
        app.handle_key(KeyCode::Char('R'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(app), "driver_launch_form");
        let request = next_matching(rx, |a| {
            matches!(a, Action::DriverLaunchPreflightRequested { .. })
        })
        .await;
        app.update(request);
        let preflighted =
            next_matching(rx, |a| matches!(a, Action::DriverLaunchPreflighted { .. })).await;
        app.update(preflighted);
    }

    /// The tracer (D-19, D-21, D-26): `R` on a crashed run opens a form prefilled from
    /// the run, the host check arrives through the injected `detect`, and `y` relaunches
    /// the SAME run id through the injected `launch` with the recorded caps, sandboxed,
    /// the registry path travelling with it. The status names the run.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_r_then_y_relaunches_the_same_run_through_the_seam() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, _run_dir) = launch_fixture(dir.path()).await;

        // Before the host check answers, the form says so and `y` does nothing.
        app.handle_key(KeyCode::Char('R'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        let loading = render_top_screen(&app);
        assert!(loading.contains("Sandbox: Loading..."), "{loading}");
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_launch_form", "y is inert");
        assert!(launches_for(dir.path()).is_empty());
        // Close it and start again through the helper (the first request is still in
        // flight; its answer is for the same run and is simply stored).
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");

        open_the_launch_form(&mut app, &mut rx).await;
        let form = render_top_screen(&app);
        assert!(form.contains("Launch driver run"), "{form}");
        assert!(form.contains(LAUNCHER_RUN), "{form}");
        assert!(
            form.contains("dispatches      10"),
            "the recorded cap:\n{form}"
        );
        assert!(
            form.contains("minutes         90"),
            "the recorded cap:\n{form}"
        );
        assert!(
            form.contains("Sandbox: bwrap + systemd scope (host check passed)"),
            "{form}"
        );

        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail", "the form closes on y");
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverLaunchRequested { .. })
        })
        .await;
        app.update(request);
        let launched = next_matching(&mut rx, |a| matches!(a, Action::DriverLaunched { .. })).await;
        let Action::DriverLaunched { result, .. } = &launched else {
            unreachable!()
        };
        assert_eq!(*result, Ok(4242));
        app.update(launched);

        let calls = launches_for(dir.path());
        assert_eq!(calls.len(), 1, "exactly one launch: {calls:?}");
        let (options, config) = &calls[0];
        assert_eq!(options.run_id, LAUNCHER_RUN);
        assert_eq!(options.alias, OBS_ALIAS);
        assert!(!options.unsandboxed);
        assert_eq!(options.prompt_file, None);
        assert_eq!(options.model, "opus");
        assert_eq!(options.limits.dispatches, Some(10));
        assert_eq!(options.limits.wall_clock_min, Some(90.0));
        assert_eq!(options.limits.gap_rounds, Some(2));
        assert_eq!(config.as_deref(), Some(app.ctx.config_path.as_path()));
        let (status, _) = app.ctx.status_message.clone().expect("a status message");
        assert_eq!(
            status,
            format!(
                "{OBS_ALIAS}: launched {LAUNCHER_RUN}. It runs detached and keeps running if you close this TUI."
            )
        );
        // The rows are re-listed so the run's status catches up.
        deliver_launcher_refresh(&mut app, &mut rx).await;
    }

    /// A failed launch is an error that names the run and says nothing was started.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_a_failed_launch_says_nothing_was_started() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx, _run_dir) = launch_fixture(dir.path()).await;
        let copy = crate::ui::screens::driver_launch::launch_error_copy(
            LAUNCHER_RUN,
            &crate::launcher::LauncherError::Locked {
                detail: "held".to_string(),
            },
        );
        app.update(Action::DriverLaunched {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            result: Err(copy.clone()),
        });
        assert_eq!(app.ctx.error_message.as_deref(), Some(copy.as_str()));
        assert!(copy.contains(LAUNCHER_RUN) && copy.ends_with("Nothing was started."));
        assert!(app.ctx.status_message.is_none());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn launcher_launch_a_non_linux_build_refuses_and_starts_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(Action::DriverLaunchRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            args: crate::cli::launch_defaults(),
        });
        let message = app.ctx.error_message.clone().expect("a refusal");
        assert!(message.ends_with("Nothing was started."), "{message}");
    }

    /// A host that lacks `bwrap` but has a new enough claude.
    #[cfg(target_os = "linux")]
    fn fake_detect_degraded() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: vec!["bwrap".to_string()],
            claude_version: Some((2, 1, 295)),
        }
    }

    /// A host whose claude is older than the hook field the gate relies on.
    #[cfg(target_os = "linux")]
    fn fake_detect_old_claude() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 0, 1)),
        }
    }

    /// Whether any `DriverLaunchRequested` is waiting on the channel.
    #[cfg(target_os = "linux")]
    fn no_launch_requested(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) -> bool {
        let mut none = true;
        while let Ok(action) = rx.try_recv() {
            none &= !matches!(action, Action::DriverLaunchRequested { .. });
        }
        none
    }

    /// D-20 end to end through the App: on a host missing `bwrap`, `y` launches nothing
    /// and says why; `U` then `n` launches nothing; only `U` then `y` launches, with
    /// `unsandboxed`, once; and the next form is sandboxed again (nothing is remembered).
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_a_degraded_host_launches_unsandboxed_only_through_the_second_confirmation(
    ) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, _run_dir) = launch_fixture(dir.path()).await;
        app.launch_seams.detect = fake_detect_degraded;

        open_the_launch_form(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(
            text.contains("Sandbox unavailable: missing bwrap."),
            "{text}"
        );
        assert!(
            text.contains("[U] launch UNSANDBOXED instead (asks again)"),
            "{text}"
        );
        // y: nothing launches; the form stays open and says why.
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        assert!(no_launch_requested(&mut rx));
        let text = render_top_screen(&app);
        assert!(
            text.contains("Launch refused: this host cannot sandbox the driver (missing: bwrap)."),
            "{text}"
        );

        // U then n: back to the tab, nothing launched.
        app.handle_key(KeyCode::Char('U'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(&app), "driver_launch_unsandboxed");
        let text = render_top_screen(&app);
        assert!(text.contains("Launch UNSANDBOXED"), "{text}");
        assert!(text.contains("WITHOUT a sandbox?"), "{text}");
        app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
        assert!(no_launch_requested(&mut rx));
        assert!(launches_for(dir.path()).is_empty());

        // U then y: exactly one launch, unsandboxed.
        open_the_launch_form(&mut app, &mut rx).await;
        app.handle_key(KeyCode::Char('U'), KeyModifiers::SHIFT);
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverLaunchRequested { .. })
        })
        .await;
        app.update(request);
        let launched = next_matching(&mut rx, |a| matches!(a, Action::DriverLaunched { .. })).await;
        app.update(launched);
        let calls = launches_for(dir.path());
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert!(calls[0].0.unsandboxed);
        assert_eq!(calls[0].0.run_id, LAUNCHER_RUN);

        // The next form is sandboxed again, and y on the same degraded host is refused.
        open_the_launch_form(&mut app, &mut rx).await;
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        assert!(no_launch_requested(&mut rx));
        assert_eq!(launches_for(dir.path()).len(), 1, "no second launch");
    }

    /// A claude older than the minimum is refused in both postures: no `[U]`, no launch.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_an_old_claude_is_refused_without_an_unsandboxed_offer() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, _run_dir) = launch_fixture(dir.path()).await;
        app.launch_seams.detect = fake_detect_old_claude;
        open_the_launch_form(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(
            text.contains("Launch refused: Claude Code 2.0.1 is older than"),
            "{text}"
        );
        assert!(!text.contains("[U]"), "{text}");
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('U'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        assert!(no_launch_requested(&mut rx));
        assert!(launches_for(dir.path()).is_empty());
    }

    /// The template's unfilled placeholders are `driver check` problems: they show in
    /// the form and block `y`, exactly as the launcher's own preflight would refuse.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_template_placeholders_block_y() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir) = launch_fixture(dir.path()).await;
        crate::driver::steer::tests::write_run(
            &run_dir,
            crate::driver_templates::DIRECTION_TEMPLATE,
        );
        open_the_launch_form(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("driver check found"), "{text}");
        assert!(text.contains("before launching."), "{text}");
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        assert!(no_launch_requested(&mut rx));
        assert!(launches_for(dir.path()).is_empty());
    }

    /// A body that says `control: pause` blocks `y`; `c` closes the form and opens the
    /// control chooser, and nothing is written for the user.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_a_paused_body_blocks_y_and_c_opens_the_chooser() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir) = launch_fixture(dir.path()).await;
        let paused =
            crate::driver::steer::set_control(&crate::driver::steer::tests::clean_body(), "pause")
                .expect("a control write");
        crate::driver::steer::tests::write_run(&run_dir, &paused);
        let before = std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap();
        open_the_launch_form(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("control is pause:"), "{text}");
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_control");
        assert!(no_launch_requested(&mut rx));
        assert_eq!(
            std::fs::read_to_string(run_dir.join("DIRECTION.md")).unwrap(),
            before,
            "nothing writes control: run automatically"
        );
    }

    /// `R` on a run that is running, whose liveness is unknown, or that is done opens no
    /// form and says why; a run that was never launched opens the form with the defaults.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_r_refuses_a_running_run_and_opens_the_defaults_for_a_new_one() {
        use crate::driver::lock::{acquire_at, run_lock_path};
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx, run_dir) = launch_fixture(dir.path()).await;
        // The file says `running`, and a held lock agrees: the run is running.
        write_dead_launcher_run(dir.path(), LAUNCHER_RUN);
        let _lock = acquire_at(&run_lock_path(&run_dir), LAUNCHER_RUN, std::process::id())
            .expect("the test takes the run lock");
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        app.handle_key(KeyCode::Char('R'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(&app), "detail", "no form for a running run");
        let (status, _) = app.ctx.status_message.clone().expect("a status message");
        assert_eq!(
            status,
            format!("{LAUNCHER_RUN} is still running (its run lock is held). Stop it first.")
        );
        assert!(no_launch_requested(&mut rx));
        drop(_lock);

        // Lower-case r never opens the launch form.
        app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
    }

    /// `R` on a never-launched run opens the same form with the CLI defaults.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_launch_r_on_a_new_run_opens_the_form_with_the_defaults() {
        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).unwrap();
        // `driver new`'s tool block (status starting, launches 0) over a clean body.
        let run = crate::driver_templates::create_run_dir(dir.path(), "newer", chrono::Utc::now())
            .expect("a new run");
        let text = std::fs::read_to_string(&run.direction).unwrap();
        let block = crate::driver::direction::split_frontmatter(&text)
            .0
            .expect("a tool block")
            .to_string();
        std::fs::write(
            &run.direction,
            format!("{block}{}", crate::driver::steer::tests::clean_body()),
        )
        .unwrap();
        std::fs::remove_dir_all(&run_dir).unwrap();

        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.launch_seams = LaunchSeams {
            launch: fake_launch,
            detect: fake_detect_ready,
        };
        app.update(Action::FileChanged {
            project_path: dir.path().to_path_buf(),
            changed_path: run.direction.clone(),
        });
        deliver_launcher_refresh(&mut app, &mut rx).await;
        open_the_launch_form(&mut app, &mut rx).await;
        let form = render_top_screen(&app);
        assert!(form.contains(&run.run_id), "{form}");
        assert!(form.contains("dispatches      10"), "{form}");
        assert!(form.contains("minutes         60"), "{form}");
        assert!(form.contains("gap rounds      2 per phase"), "{form}");
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverLaunchRequested { .. })
        })
        .await;
        app.update(request);
        let launched = next_matching(&mut rx, |a| matches!(a, Action::DriverLaunched { .. })).await;
        app.update(launched);
        let calls = launches_for(dir.path());
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].0.run_id, run.run_id);
        assert_eq!(calls[0].0.limits.wall_clock_min, Some(60.0));
    }

    // ── 30-05: a new run from the Driver tab (D-18) ─────────────────────────

    /// Today's UTC date, as the run id the slug input previews.
    #[cfg(target_os = "linux")]
    fn today() -> String {
        chrono::Utc::now().format("%Y%m%d").to_string()
    }

    /// A GSD project (`.planning/` exists) on the Driver tab, nothing listed yet, with
    /// the fake launch seams.
    #[cfg(target_os = "linux")]
    fn new_run_app(root: &Path) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        std::fs::create_dir_all(root.join(".planning")).unwrap();
        let (mut app, rx) = app_on_the_driver_tab(root);
        app.launch_seams = LaunchSeams {
            launch: fake_launch,
            detect: fake_detect_ready,
        };
        (app, rx)
    }

    /// `s`, a slug, Enter, and the App's answer: the run exists, the slug input is gone
    /// and the editor is about to open on the new run's body. Returns the run id and the
    /// edit temp's path.
    #[cfg(target_os = "linux")]
    async fn create_a_run(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        slug: &str,
    ) -> (String, PathBuf) {
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(app), "driver_slug_input");
        for c in slug.chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        let request =
            next_matching(rx, |a| matches!(a, Action::DriverNewRunRequested { .. })).await;
        app.update(request);
        let created = next_matching(rx, |a| matches!(a, Action::DriverNewRunCreated { .. })).await;
        let Action::DriverNewRunCreated { result, .. } = &created else {
            unreachable!()
        };
        let run_id = result.clone().expect("the run is created");
        app.update(created);
        assert_eq!(top_screen_name(app), "detail", "the slug input is gone");
        let ready = next_matching(rx, |a| matches!(a, Action::DriverEditReady { .. })).await;
        app.update(ready);
        let (temp, line) = app.pending_editor.take().expect("the editor is requested");
        assert_eq!(line, None);
        (run_id, temp)
    }

    /// The editor "exits" with the temp as it is, and the apply and the check are driven
    /// to their answers.
    #[cfg(target_os = "linux")]
    async fn finish_the_editor(
        app: &mut App,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
        temp: &Path,
    ) {
        app.after_editor(temp, EditorExit::Success);
        let done = next_matching(rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        let checked = next_matching(rx, |a| matches!(a, Action::DriverNewRunChecked { .. })).await;
        app.update(checked);
    }

    /// The whole flow, through the key handler and the App: `s` mints and creates the
    /// run (selected, with its DIRECTION.md and LOG.md), the editor opens on its body;
    /// an unedited template fails `driver check` and opens `Check DIRECTION.md`; `y`
    /// re-opens the editor; a clean body opens the launch form, whose `y` launches the
    /// new run through the fake seam. Zero spend.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_s_creates_edits_checks_and_launches() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (run_id, temp) = create_a_run(&mut app, &mut rx, "tab").await;
        assert_eq!(run_id, format!("{}-tab", today()));

        let run_dir = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(&run_id);
        assert!(run_dir.join("DIRECTION.md").is_file());
        assert!(run_dir.join("LOG.md").is_file());
        assert_eq!(
            temp,
            run_dir.join(crate::driver::steer::EDIT_TEMP_NAME),
            "the body-only edit temp of the new run"
        );
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .launcher_selected_id
                .as_deref(),
            Some(run_id.as_str()),
            "the new run is selected"
        );
        assert_eq!(
            app.ctx.status_message.clone().expect("a status message").0,
            format!("Created run {run_id}. It is checked before launch.")
        );
        let session = app.edit_sessions.get(&temp).expect("a session");
        assert!(session.then_launch, "the edit belongs to the new-run flow");

        // The editor leaves the template as it was: placeholders are problems.
        finish_the_editor(&mut app, &mut rx, &temp).await;
        assert_eq!(top_screen_name(&app), "driver_lint_popup");
        let text = render_top_screen(&app);
        assert!(text.contains("Check DIRECTION.md"), "{text}");
        assert!(text.contains("Launch is blocked until they are"), "{text}");
        assert!(
            app.new_run_flow
                .contains(&(OBS_ALIAS.to_string(), run_id.clone())),
            "the flow is not over"
        );

        // y: the editor opens again, still the flow's.
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
        let request =
            next_matching(&mut rx, |a| matches!(a, Action::DriverEditRequested { .. })).await;
        app.update(request);
        let ready = next_matching(&mut rx, |a| matches!(a, Action::DriverEditReady { .. })).await;
        app.update(ready);
        let (temp, _) = app
            .pending_editor
            .take()
            .expect("the editor is requested again");
        assert!(app.edit_sessions.get(&temp).expect("a session").then_launch);

        // This time the user writes a clean body.
        std::fs::write(&temp, crate::driver::steer::tests::clean_body()).unwrap();
        finish_the_editor(&mut app, &mut rx, &temp).await;
        assert_eq!(top_screen_name(&app), "driver_launch_form");
        assert!(
            app.new_run_flow.is_empty(),
            "a clean check ends the editing half of the flow"
        );
        let preflighted = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverLaunchPreflighted { .. })
        })
        .await;
        app.update(preflighted);
        let form = render_top_screen(&app);
        assert!(form.contains(&run_id), "{form}");
        assert!(form.contains("Sandbox: bwrap + systemd scope"), "{form}");
        assert!(form.contains("dispatches      10"), "the defaults:\n{form}");

        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverLaunchRequested { .. })
        })
        .await;
        app.update(request);
        let launched = next_matching(&mut rx, |a| matches!(a, Action::DriverLaunched { .. })).await;
        app.update(launched);
        let calls = launches_for(dir.path());
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].0.run_id, run_id);
        assert!(!calls[0].0.unsandboxed);
    }

    /// `n` on the popup leaves the run created and unlaunched, says so and ends the flow.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_n_leaves_the_run_unlaunched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (run_id, temp) = create_a_run(&mut app, &mut rx, "keep").await;
        finish_the_editor(&mut app, &mut rx, &temp).await;
        assert_eq!(top_screen_name(&app), "driver_lint_popup");
        app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
        let left = next_matching(&mut rx, |a| matches!(a, Action::DriverNewRunLeft { .. })).await;
        app.update(left);
        assert_eq!(
            app.ctx.status_message.clone().expect("a status message").0,
            format!("{run_id} created, not launched: [R] launches it.")
        );
        assert!(app.new_run_flow.is_empty());
        assert!(no_launch_requested(&mut rx));
        assert!(launches_for(dir.path()).is_empty());
    }

    /// A second `s` with the same slug on the same day finds the run: the input stays
    /// open with the buffer, says so, and the existing run is not touched.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_the_same_slug_twice_says_the_run_exists_and_keeps_the_buffer() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (run_id, _temp) = create_a_run(&mut app, &mut rx, "twice").await;
        let direction = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(&run_id)
            .join("DIRECTION.md");
        let before = std::fs::read_to_string(&direction).unwrap();

        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        for c in "twice".chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverNewRunRequested { .. })
        })
        .await;
        app.update(request);
        let created =
            next_matching(&mut rx, |a| matches!(a, Action::DriverNewRunCreated { .. })).await;
        let Action::DriverNewRunCreated { result, .. } = &created else {
            unreachable!()
        };
        assert_eq!(
            *result,
            Err(crate::action::NewRunFailure::Exists(run_id.clone()))
        );
        app.update(created);
        assert_eq!(
            top_screen_name(&app),
            "driver_slug_input",
            "the input stays open"
        );
        let text = render_top_screen(&app);
        assert!(
            text.contains("New run slug> twice_"),
            "the buffer is kept:\n{text}"
        );
        assert!(
            text.contains(&format!(
                "A run named {run_id} already exists. Pick another slug."
            )),
            "{text}"
        );
        assert_eq!(
            std::fs::read_to_string(&direction).unwrap(),
            before,
            "never overwritten"
        );
        // Esc closes it.
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "detail");
    }

    /// Any other creation failure closes the input and says why.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_new_run_another_failure_closes_the_input_and_says_why() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = new_run_app(dir.path());
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_slug_input");
        app.update(Action::DriverNewRunCreated {
            alias: OBS_ALIAS.to_string(),
            slug: "x".to_string(),
            result: Err(crate::action::NewRunFailure::Other("disk full".to_string())),
        });
        assert_eq!(top_screen_name(&app), "detail");
        assert_eq!(
            app.ctx.error_message.as_deref(),
            Some("Could not create the run: disk full")
        );
        assert!(app.new_run_flow.is_empty());
    }

    /// IN-02: a new-run request the App refuses before it starts (an alias that is no
    /// longer registered, a session with no event channel) closes the slug input and
    /// says why; the input is never left pending with Enter dead.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_new_run_a_refused_request_closes_the_slug_input() {
        let dir = tempfile::tempdir().expect("temp dir");

        // The alias the request names is not registered.
        let (mut app, _rx) = new_run_app(dir.path());
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_slug_input");
        app.update(Action::DriverNewRunRequested {
            alias: "ghost".to_string(),
            slug: "x".to_string(),
        });
        assert_eq!(top_screen_name(&app), "detail", "the input is closed");
        let message = app.ctx.error_message.clone().expect("an error");
        assert!(
            message.starts_with("Could not create the run: "),
            "{message}"
        );
        assert!(message.contains("Nothing was created."), "{message}");

        // A session with no event channel cannot hear the answer.
        let (mut app, _rx) = new_run_app(dir.path());
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&app), "driver_slug_input");
        app.ctx.event_tx = None;
        app.update(Action::DriverNewRunRequested {
            alias: OBS_ALIAS.to_string(),
            slug: "x".to_string(),
        });
        assert_eq!(top_screen_name(&app), "detail", "the input is closed");
        let message = app.ctx.error_message.clone().expect("an error");
        assert!(message.contains("Nothing was created."), "{message}");
        assert!(
            !dir.path()
                .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
                .exists(),
            "nothing was created"
        );
    }

    /// WR-04: the new-run check numbers its findings from the first body line, the way
    /// the body-only edit temp shows them in the editor.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_check_numbers_lint_lines_like_the_editor() {
        use crate::driver::direction::split_frontmatter;
        use crate::driver_templates::lint::lint_direction;
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (run_id, temp) = create_a_run(&mut app, &mut rx, "nums").await;
        app.after_editor(&temp, EditorExit::Success);
        let done = next_matching(&mut rx, |a| matches!(a, Action::DriverEditDone { .. })).await;
        app.update(done);
        let checked =
            next_matching(&mut rx, |a| matches!(a, Action::DriverNewRunChecked { .. })).await;
        let Action::DriverNewRunChecked { problems, .. } = &checked else {
            unreachable!()
        };
        let text = std::fs::read_to_string(
            dir.path()
                .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
                .join(&run_id)
                .join("DIRECTION.md"),
        )
        .unwrap();
        let (frontmatter, body) = split_frontmatter(&text);
        assert!(frontmatter.is_some(), "the run has a tool block");
        let editor: Vec<String> = lint_direction(body)
            .problems
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(
            !editor.is_empty(),
            "the template's placeholders are problems"
        );
        assert_ne!(
            lint_direction(&text).problems[0].to_string(),
            editor[0],
            "the whole-file numbering differs by the tool block"
        );
        assert_eq!(*problems, editor);
    }

    /// A project that is not a GSD project (no `.planning/`) is refused by the creation
    /// itself, with nothing created.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_a_directory_that_is_not_a_gsd_project_creates_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        for c in "nope".chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        let request = next_matching(&mut rx, |a| {
            matches!(a, Action::DriverNewRunRequested { .. })
        })
        .await;
        app.update(request);
        let created =
            next_matching(&mut rx, |a| matches!(a, Action::DriverNewRunCreated { .. })).await;
        app.update(created);
        assert_eq!(top_screen_name(&app), "detail");
        let message = app.ctx.error_message.clone().expect("an error");
        assert!(
            message.starts_with("Could not create the run: "),
            "{message}"
        );
        assert!(message.contains("not a GSD project"), "{message}");
        assert!(
            !dir.path().join(".planning").exists(),
            "nothing was created"
        );
    }

    /// A failed editor ends the flow with the run left unlaunched.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_an_editor_failure_ends_the_flow() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (_run_id, temp) = create_a_run(&mut app, &mut rx, "fails").await;
        app.after_editor(&temp, EditorExit::Failed("exit status: 1".to_string()));
        assert!(app.new_run_flow.is_empty());
        assert!(app.edit_sessions.is_empty());
        let message = app.ctx.error_message.clone().expect("a message");
        assert!(
            message.starts_with("The editor exited with exit status: 1."),
            "{message}"
        );
        assert!(temp.exists(), "the text is kept");
    }

    /// The footer names `[R]elaunch` on a crashed row and `[R]launch` on a never-launched
    /// one, and `s` never opens the legacy start flow.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_new_run_the_footer_names_launch_or_relaunch_for_the_selected_row() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (app, _rx, _run_dir) = launch_fixture(dir.path()).await;
        let crashed = render_top_screen(&app);
        assert!(crashed.contains("[R]elaunch"), "{crashed}");
        assert!(crashed.contains("[s]tart"), "{crashed}");
        drop(app);

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = new_run_app(dir.path());
        let (_run_id, _temp) = create_a_run(&mut app, &mut rx, "fresh").await;
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let fresh = render_top_screen(&app);
        assert!(fresh.contains("[R]launch"), "{fresh}");
        assert!(!fresh.contains("[R]elaunch"), "{fresh}");

        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_ne!(
            top_screen_name(&app),
            "driver_start",
            "s never opens the legacy flow"
        );
        assert_eq!(top_screen_name(&app), "driver_slug_input");
    }

    /// The Driver tab's keys shadow no global key (D-02, D-03, UI-SPEC "Keymap
    /// collision check"): `q`, `?`, `M`, `D` and the digits act on the Driver tab exactly
    /// as on another tab, Ctrl+C still quits, `c a u R L [ ]` keep their shipped meaning
    /// on the tabs that already bind them (the Roadmap's wave brackets are pinned in
    /// `detail.rs`, where its fixtures live), and lower-case `r` never opens the launch
    /// form.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn launcher_view_driver_keys_shadow_no_global_key() {
        use crate::ui::screens::detail::DetailScreen;

        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir
            .path()
            .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN);
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::create_dir_all(dir.path().join(".planning")).unwrap();
        crate::driver::steer::tests::write_run(
            &run_dir,
            &crate::driver::steer::tests::clean_body(),
        );

        // A fresh App on `tab`, with the launcher run listed and selected.
        let fresh = |tab: DetailSubView| -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
            let (mut app, rx) = obs_app(dir.path());
            app.ctx.recompute_filtered_aliases();
            app.ctx.table_state.select(Some(0));
            app.screen_stack
                .push(Box::new(DetailScreen::new(OBS_ALIAS.to_string())));
            app.ctx
                .detail_sub_view_per_project
                .insert(OBS_ALIAS.to_string(), tab);
            let view = crate::state_reader::driver_frontmatter::read_driver_run(&run_dir)
                .expect("the run reads");
            let cache = app.ctx.view_cache.entry(OBS_ALIAS.to_string()).or_default();
            cache.launcher_runs = vec![(LAUNCHER_RUN.to_string(), Ok(view))];
            cache.launcher_listed = true;
            cache.launcher_selected_id = Some(LAUNCHER_RUN.to_string());
            (app, rx)
        };
        // What a key leaves behind: the top screen, whether the app is quitting, the mouse
        // state, and whether the tab bar has focus (its footer).
        let observe = |app: &App| {
            (
                top_screen_name(app),
                app.should_quit,
                app.mouse_capture,
                render_top_screen(app).contains("[\u{2193}/Enter]open"),
            )
        };
        let sub_view = |app: &App| app.ctx.detail_sub_view_per_project.get(OBS_ALIAS).cloned();

        // (a) The tab-independent detail keys act exactly as they do on another tab.
        for key in ['q', '?', 'M', 'D', '1', '2', '3', '4', '5', '6', '7', '8'] {
            let modifiers = if key.is_ascii_uppercase() {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            let (mut driver, _rx1) = fresh(DetailSubView::Driver);
            let (mut other, _rx2) = fresh(DetailSubView::GitHistory);
            driver.handle_key(KeyCode::Char(key), modifiers);
            other.handle_key(KeyCode::Char(key), modifiers);
            assert_eq!(
                observe(&driver),
                observe(&other),
                "{key:?} acts differently on the Driver tab"
            );
            if key.is_ascii_digit() || key == 'D' {
                // A tab jump lands on the same tab from either side.
                assert_eq!(
                    sub_view(&driver),
                    sub_view(&other),
                    "{key:?} lands elsewhere"
                );
            } else {
                // Everything else leaves the tab it was pressed on exactly where it was.
                assert_eq!(sub_view(&driver), Some(DetailSubView::Driver), "{key:?}");
                assert_eq!(sub_view(&other), Some(DetailSubView::GitHistory), "{key:?}");
            }
        }
        // Ctrl+C still quits from the Driver tab and opens nothing.
        let (mut driver, _rx) = fresh(DetailSubView::Driver);
        driver.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(driver.should_quit);
        assert!(
            driver
                .screen_stack
                .iter()
                .all(|s| s.name() != "driver_control"),
            "Ctrl+C must not open the control chooser"
        );

        // (b) The keys this phase added keep their shipped meaning where they already had
        // one: `[` / `]` switch the Sessions / Agents sub-tab, `a` adds to the queue.
        let (mut sessions, _rx) = fresh(DetailSubView::Sessions);
        sessions.handle_key(KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(
            sessions.ctx.detail_sub_view_per_project.get(OBS_ALIAS),
            Some(&DetailSubView::Agents)
        );
        sessions.handle_key(KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(
            sessions.ctx.detail_sub_view_per_project.get(OBS_ALIAS),
            Some(&DetailSubView::Sessions)
        );
        let (mut queue, _rx) = fresh(DetailSubView::Queue);
        queue.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            top_screen_name(&queue),
            "enqueue",
            "`a` on Queue still adds"
        );
        // On a tab that binds none of the seven, none of them reaches a Driver screen or
        // sets a Driver status copy.
        for key in ['c', 'a', 'u', 'R', 'L', '[', ']'] {
            let (mut other, _rx) = fresh(DetailSubView::GitHistory);
            other.handle_key(KeyCode::Char(key), KeyModifiers::NONE);
            let name = top_screen_name(&other);
            assert!(
                !matches!(
                    name.as_str(),
                    "driver_control" | "driver_answer" | "driver_launch_form" | "driver_slug_input"
                ),
                "`{key}` opened {name} on GitHistory"
            );
            assert!(
                other.ctx.status_message.is_none(),
                "`{key}` set a status on GitHistory"
            );
            assert!(other.ctx.error_message.is_none());
        }

        // (c) Lower-case `r` on the Driver tab never opens the launch form; `R` does.
        let (mut driver, _rx) = fresh(DetailSubView::Driver);
        driver.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert_eq!(top_screen_name(&driver), "detail");
        driver.handle_key(KeyCode::Char('R'), KeyModifiers::SHIFT);
        assert_eq!(top_screen_name(&driver), "driver_launch_form");
    }
}
