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

/// Whether the selected launcher run's heartbeat age needs a frame this tick (D-08).
///
/// The age is `now − heartbeat`, recomputed at render time, so it changes every
/// second a live run is on screen and a frame only on a file event would freeze it
/// (and with it the `stale` flag). The Driver tab is on top **and** the highlighted launcher run is live, so an idle
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
/// Shared by the heartbeat repaint gate and the Driver tab's rescan rather than
/// written twice: both questions ("should the heartbeat age repaint" and "should
/// the launcher run list be rescanned") have the same visibility half and differ
/// only in whether they also require a live run.
///
/// **The rescan must NOT require a live run** (CR-04), and that difference is
/// the whole reason this is its own function. A run that has just ended is
/// precisely when the run list's glyph and the outcome become stale and most
/// need re-reading; a rescan gated on liveness would stop firing exactly when
/// it started mattering.
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

/// Precedence of a [`StatusCategory`] when several scopes of one project
/// disagree: higher wins. Private; only [`project_status_category`] ranks.
fn status_precedence(category: &StatusCategory) -> u8 {
    match category {
        StatusCategory::Blocked => 4,
        StatusCategory::Active => 3,
        StatusCategory::Idle => 2,
        StatusCategory::Unknown => 1,
        StatusCategory::Complete => 0,
    }
}

/// The one status category a project's dashboard row stands for: the
/// highest-precedence [`classify_status`] across the root `STATE.md` and every
/// parallel workstream, with precedence Blocked > Active > Idle > Unknown >
/// Complete.
///
/// Most-attention / most-active wins, so a project counts as `done` only when
/// every scope is Complete (a root at `phase_complete` with a workstream still
/// `executing` is Active, not done). Unknown outranks Complete because a scope
/// whose status cannot be read is not demonstrably finished. With no
/// workstreams the result is exactly `classify_status` of the root status.
///
/// Pure and in-memory: it reads only the already-parsed state, so the render
/// loop may call it per frame without touching the disk.
pub fn project_status_category(state: &ProjectState) -> StatusCategory {
    let mut best = classify_status(&state.status);
    for ws in &state.workstreams {
        let candidate = classify_status(&ws.state.status);
        if status_precedence(&candidate) > status_precedence(&best) {
            best = candidate;
        }
    }
    best
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
/// slightly false about the repository — the same argument a string's
/// truncation marker makes, here applied to a document.
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

/// The marker that says "this is a parallel workstream" wherever one is drawn:
/// `ws*` for the workstream `.planning/active-workstream` names, plain `ws`
/// otherwise. The dashboard line ([`format_workstream_display`]) and the Detail
/// screen's Roadmap and Phases tabs ([`workstream_tag`]) share it, so the one
/// word means the one thing on every screen.
pub fn workstream_marker(ws: &crate::state_reader::workstreams::WorkstreamState) -> &'static str {
    if ws.active {
        "ws*"
    } else {
        "ws"
    }
}

/// The milestone a workstream is identified by on screen: STATE.md's
/// `milestone`; a workstream whose STATE.md names none shows its directory name
/// instead. Third-party text: the caller escapes it.
pub fn workstream_milestone_label(ws: &crate::state_reader::workstreams::WorkstreamState) -> &str {
    let milestone = ws.state.milestone.trim();
    if milestone.is_empty() {
        ws.name.as_str()
    } else {
        milestone
    }
}

/// What marks a workstream's phases on the Detail screen: `ws v0.14` (its
/// [`workstream_marker`], then its [`workstream_milestone_label`]). A phase row
/// reads `ws v0.14 P26: Push-to-Talk Voice` — the dashboard's own line, minus
/// the live-claim logic that picks one phase for a one-line summary.
///
/// Third-party text: the caller escapes it for the terminal.
pub fn workstream_tag(ws: &crate::state_reader::workstreams::WorkstreamState) -> String {
    format!(
        "{} {}",
        workstream_marker(ws),
        workstream_milestone_label(ws)
    )
}

/// The dashboard text of one parallel workstream: `ws v0.14 P26: Push-to-Talk
/// Voice` — its milestone, then the phase it is working.
///
/// **Why this exists, and why it is a line of its own.** Workstreams used to
/// reach the dashboard only as a `[N ws]` count appended to the project's phase
/// label. sentriq's root label is 54 cells wide and the Phase column is a
/// percentage of the terminal, so the count was clipped away at every ordinary
/// width — and even drawn it named neither a milestone nor a phase. A parallel
/// milestone is a line of work in its own right, so each gets its own line,
/// with the identifying words first so a narrow column clips the phase *name*
/// and never the milestone or the number.
///
/// - `ws*` marks the workstream `.planning/active-workstream` names; plain `ws`
///   otherwise. (GSD 1.18 mostly keeps that pointer per session in a temp dir,
///   so on disk there often is none.)
/// - The milestone is STATE.md's `milestone`; a workstream whose STATE.md names
///   none shows its directory name instead.
/// - The phase is [`WorkstreamState::focus_phase_number`]: a live
///   `milestone.lock` claim, else the workstream's most advanced real
///   unfinished phase, else the established active-phase rule.
/// - An unreadable STATE.md says so rather than guess a phase, exactly as
///   [`format_phase_display`] does, and a repaired one carries
///   [`RECOVERED_STATE_MARKER`].
///
/// The result is third-party text: the caller escapes it for the terminal.
///
/// [`WorkstreamState::focus_phase_number`]: crate::state_reader::workstreams::WorkstreamState::focus_phase_number
pub fn format_workstream_display(
    ws: &crate::state_reader::workstreams::WorkstreamState,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let marker = workstream_marker(ws);
    let label = workstream_milestone_label(ws);
    if ws.state.state_md_unreadable {
        return format!(
            "{marker} {label} {}",
            unreadable_state_label(ws.state.state_md_fault_position)
        );
    }
    let recovered = if ws.state.state_md_recovered {
        RECOVERED_STATE_MARKER
    } else {
        ""
    };
    match ws.focus_phase_number(now) {
        Some(number) => {
            let name = ws
                .focus_phase(now)
                .map(|phase| phase.name.as_str())
                .unwrap_or("Unknown");
            format!("{marker} {recovered}{label} P{number}: {name}")
        }
        None => format!("{marker} {recovered}{label}"),
    }
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Clone, Copy)]
pub struct LaunchSeams {
    pub launch: fn(
        &crate::launcher::run::LaunchOptions,
        Option<&Path>,
    ) -> Result<u32, crate::launcher::LauncherError>,
    pub detect: fn() -> crate::launcher::host::HostCaps,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Default for LaunchSeams {
    fn default() -> Self {
        Self {
            launch: crate::launcher::run::launch_detached,
            detect: crate::launcher::host::detect,
        }
    }
}

/// The one-off launch the Driver tab's confirm popup reaches (quick 261103-f5y).
/// A seam for the reason [`LaunchSeams`] is one: the production value re-executes
/// this binary, which a unit test must not do. A sibling, not a field of
/// `LaunchSeams`, so the tests that build that struct literally stay unchanged.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Clone, Copy)]
pub struct OneoffSeams {
    pub run: fn(
        &crate::oneoff::launch::OneoffOptions,
        Option<&Path>,
    ) -> Result<crate::oneoff::launch::Started, crate::launcher::LauncherError>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Default for OneoffSeams {
    fn default() -> Self {
        Self {
            run: crate::oneoff::launch::run_detached,
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub launch_seams: LaunchSeams,
    /// The one-off launch the Driver tab reaches (quick 261103-f5y).
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub oneoff_seams: OneoffSeams,
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

/// How often the viewer's tail looks at the run's files when nothing is new.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const ONEOFF_TAIL_POLL: std::time::Duration = std::time::Duration::from_millis(250);

/// What the viewer's background tail follows (quick 261103-hx7, C-30).
#[cfg(any(target_os = "linux", target_os = "macos"))]
struct TailSpec {
    alias: String,
    run_id: String,
    epoch: u64,
    /// The directory holding the followed `stream.jsonl`: the run dir, or its
    /// `land/<n>` attempt dir.
    stream_dir: PathBuf,
    /// The project's one-off dir (the run dir's parent), for the liveness re-read.
    project_dir: PathBuf,
    /// The conflict-resolver attempt followed, `None` for the run's own stream.
    attempt: Option<u32>,
}

/// The viewer's background tail (blocking file I/O, so always on `spawn_blocking`):
/// replay the stream from the start, then follow it. It stops when `cancel` is
/// set (the viewer dropped), when the channel is gone, or once the run is finished and
/// the stream is drained. A resolver attempt is live while its run's land lock is held
/// and it is the newest attempt.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn oneoff_tail_loop(
    spec: &TailSpec,
    mut info: crate::ui::screens::oneoff_viewer::RunInfo,
    cancel: &std::sync::atomic::AtomicBool,
    tx: &UnboundedSender<Action>,
) {
    use crate::oneoff::store::{read_view, LiveState};
    use crate::oneoff::stream::{FollowBatch, Follower, Meter};
    use std::sync::atomic::Ordering;

    let mut follower = Follower::new(&spec.stream_dir);
    let mut last: Option<(Meter, crate::ui::screens::oneoff_viewer::RunInfo)> = None;
    let mut meter = Meter::default();
    let mut tick: u64 = 0;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let finished_before = matches!(info.live, LiveState::Finished | LiveState::Crashed);
        let batch = follower.poll().unwrap_or_else(|_| FollowBatch {
            lines: Vec::new(),
            meter: meter.clone(),
        });
        meter = batch.meter.clone();
        if tick & 3 == 0 {
            if let Some(view) = read_view(&spec.project_dir, &spec.run_id) {
                match spec.attempt {
                    None => {
                        info.live = view.live;
                        info.ended_at = view.record.ended_at.clone();
                        info.soft_cap_usd = view.record.soft_cap_usd;
                        info.soft_cap_hit_at = view.record.soft_cap_hit_at.clone();
                    }
                    Some(n) => {
                        info.live = resolver_live(&view, n);
                    }
                }
            }
        }
        tick += 1;
        let drained = batch.lines.is_empty();
        let done = finished_before && drained;
        let current = (meter.clone(), info.clone());
        if !drained || done || last.as_ref() != Some(&current) {
            let sent = tx.send(Action::OneoffStreamBatch {
                alias: spec.alias.clone(),
                run_id: spec.run_id.clone(),
                epoch: spec.epoch,
                lines: batch.lines,
                meter: meter.clone(),
                info: info.clone(),
                done,
            });
            if sent.is_err() {
                return;
            }
            last = Some(current);
        }
        if done {
            return;
        }
        if drained {
            std::thread::sleep(ONEOFF_TAIL_POLL);
        }
    }
}

/// Whether the conflict-resolver attempt `n` of `view` is running right now: a landing
/// is in progress and `n` is the newest attempt (a resolver's liveness is its run's
/// land lock, C-30).
fn resolver_live(
    view: &crate::oneoff::store::OneoffView,
    n: u32,
) -> crate::oneoff::store::LiveState {
    use crate::oneoff::store::LiveState;
    if view.landing && view.land_attempts.last() == Some(&n) {
        LiveState::Running
    } else {
        LiveState::Finished
    }
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
                crate::ui::screens::driver_steer::STEERING_UNSUPPORTED_PLATFORM.to_string(),
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
            reparse_dispatches: 0,
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
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            launch_seams: LaunchSeams::default(),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            oneoff_seams: OneoffSeams::default(),
            mouse_capture,
            click_tracker: crate::ui::mouse::ClickTracker::default(),
        }
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

    /// Re-list the selected project's launcher runs **if the Driver tab is the
    /// surface on screen** (CR-04).
    ///
    /// The launcher rows are refreshed by watcher events under a run's directory, but a
    /// lock released by `SIGKILL` emits no file event, so without this the tab is
    /// frozen at the moment it was entered: a run that crashed while you watch would
    /// keep its `running` glyph until the next write.
    ///
    /// **Gated on visibility, and deliberately so.** The listing is a `read_dir` plus
    /// a bounded read and a lock probe per run; running it for every registered project
    /// on every tick would pay a fleet-sized cost for one pane's freshness.
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
        // The launcher rows ride the same slow tick (D-13): a lock released by
        // SIGKILL emits no file event, so this is what turns that run into
        // `crashed` within one tick while the tab is open.
        self.ctx.schedule_launcher_refresh(&alias, &path);
    }

    /// Resolve one alias's project root, or refuse visibly (S4): a message in
    /// `ctx.error_message` and a redraw, never a bare `return`, so the refusal has
    /// one wording.
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
    /// experimental surfaces: the key that reaches this exists only on the Driver tab.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        // D-07: resolved here, on this thread, so a malformed `driver.egress` refuses the
        // launch with the reason in the status line and starts nothing.
        let egress_allow = match self
            .ctx
            .config
            .projects
            .get(alias)
            .map(crate::config::RegisteredProject::driver_egress_allowlist)
            .transpose()
        {
            Ok(hosts) => hosts.unwrap_or_default(),
            Err(error) => {
                self.ctx.status_message = Some((
                    format!(
                        "Not launching {}: projects.{}.{}",
                        crate::text::render_for_terminal(run_id),
                        crate::text::render_for_terminal(alias),
                        crate::text::render_for_terminal(&error.to_string())
                    ),
                    std::time::Instant::now(),
                ));
                self.needs_redraw = true;
                return;
            }
        };
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
                egress_allow,
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

    /// Show a one-off outcome and re-read the project's Driver tab (and so its
    /// one-off status line).
    fn finish_oneoff(&mut self, alias: &str, message: &str) {
        self.ctx.status_message = Some((
            crate::text::render_for_terminal(message).to_string(),
            std::time::Instant::now(),
        ));
        if let Some(path) = self.ctx.config.projects.get(alias).map(|p| p.path.clone()) {
            self.ctx.schedule_launcher_refresh(alias, &path);
        }
        self.needs_redraw = true;
    }

    /// `v` / `r` in the `T` run list: open the live viewer on the cached run `run_id`
    /// (or on its conflict-resolver attempt `land/<n>`) and start ONE background tail
    /// (never on the render thread). The tail stops when the viewer drops (its cancel
    /// flag) or, by itself, once the stream is finished and drained.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn open_oneoff_viewer(&mut self, alias: &str, run_id: &str, attempt: Option<u32>) {
        use crate::ui::screens::oneoff_viewer::{OneoffStream, OneoffViewerScreen, RunInfo};
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::sync::Arc;

        static EPOCH: AtomicU64 = AtomicU64::new(0);
        let Some((_, tx)) = self.oneoff_target(alias) else {
            return;
        };
        let Some(view) = self
            .ctx
            .view_cache
            .get(alias)
            .and_then(|cache| cache.oneoff_runs.iter().find(|v| v.record.run_id == run_id))
            .cloned()
        else {
            self.ctx.status_message = Some((
                "No such one-off run to view; press T to list the runs".to_owned(),
                std::time::Instant::now(),
            ));
            self.needs_redraw = true;
            return;
        };
        if let Some(n) = attempt {
            if !view.land_attempts.contains(&n) {
                self.ctx.status_message = Some((
                    "That run has no such resolver session".to_owned(),
                    std::time::Instant::now(),
                ));
                self.needs_redraw = true;
                return;
            }
        }
        let info = match attempt {
            None => RunInfo {
                live: view.live,
                hard_usd: view.record.max_budget_usd,
                started_at: view.record.started_at.clone(),
                ended_at: view.record.ended_at.clone(),
                soft_cap_usd: view.record.soft_cap_usd,
                soft_cap_hit_at: view.record.soft_cap_hit_at.clone(),
            },
            // A resolver session has no soft cap and its own clock the viewer does not
            // show; its ceiling is the run's budget as the meter's denominator.
            Some(n) => RunInfo {
                live: resolver_live(&view, n),
                hard_usd: view.record.max_budget_usd,
                started_at: view.record.started_at.clone(),
                ended_at: None,
                soft_cap_usd: None,
                soft_cap_hit_at: None,
            },
        };
        let epoch = EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
        let run_id = view.record.run_id.clone();
        let stream_dir = match attempt {
            None => view.run_dir.clone(),
            Some(n) => view
                .run_dir
                .join(crate::oneoff::LAND_DIR)
                .join(n.to_string()),
        };
        self.ctx
            .view_cache
            .entry(alias.to_owned())
            .or_default()
            .oneoff_stream = Some(
            OneoffStream::new(run_id.clone(), epoch, stream_dir.clone(), info.clone())
                .with_attempt(attempt),
        );
        let cancel = Arc::new(AtomicBool::new(false));
        self.screen_stack.push(Box::new(OneoffViewerScreen::new(
            alias.to_owned(),
            epoch,
            Arc::clone(&cancel),
        )));
        self.needs_redraw = true;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let spec = TailSpec {
                alias: alias.to_owned(),
                run_id,
                epoch,
                stream_dir,
                project_dir: view
                    .run_dir
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default(),
                attempt,
            };
            tokio::task::spawn_blocking(move || {
                oneoff_tail_loop(&spec, info, &cancel, &tx);
            });
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let _ = (run_id, stream_dir, info, cancel, tx);
    }

    /// `d` in the `T` run list: read the landed merge's diff off the render thread. The
    /// run, its recorded merge commit and its conflicted files come from the cache; the
    /// git read runs on `spawn_blocking` (quick 261103-hx7, C-30, I-22).
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn request_oneoff_diff(&mut self, alias: &str, run_id: &str) {
        let Some((project, tx)) = self.oneoff_target(alias) else {
            return;
        };
        let landing = self
            .ctx
            .view_cache
            .get(alias)
            .and_then(|cache| cache.oneoff_runs.iter().find(|v| v.record.run_id == run_id))
            .and_then(|view| view.record.last_merge_landing().cloned());
        let Some((sha, conflicted)) = landing.and_then(|l| l.merge_sha.map(|s| (s, l.conflicted)))
        else {
            let _ = tx.send(Action::OneoffDiffLoaded {
                alias: alias.to_owned(),
                run_id: run_id.to_owned(),
                result: Err("this run has no merge commit to show".to_owned()),
            });
            return;
        };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let alias = alias.to_owned();
            let run_id = run_id.to_owned();
            tokio::task::spawn_blocking(move || {
                let result = crate::oneoff::land::merge_diff(&project, &sha, &conflicted);
                let _ = tx.send(Action::OneoffDiffLoaded {
                    alias,
                    run_id,
                    result,
                });
            });
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let _ = (project, sha, conflicted);
    }

    /// `o` in the merge diff (quick 261103-klf, C-21, I-04): export the named run's full
    /// merge diff on `spawn_blocking`, answered by [`Action::OneoffDiffExported`].
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn request_oneoff_diff_export(&mut self, alias: &str, run_id: &str) {
        let Some((project, tx)) = self.oneoff_target(alias) else {
            return;
        };
        let found = self
            .ctx
            .view_cache
            .get(alias)
            .and_then(|cache| cache.oneoff_runs.iter().find(|v| v.record.run_id == run_id))
            .and_then(|view| {
                view.record
                    .last_merge_landing()
                    .and_then(|l| l.merge_sha.clone())
                    .map(|sha| (sha, view.run_dir.clone()))
            });
        let Some((sha, run_dir)) = found else {
            let _ = tx.send(Action::OneoffDiffExported {
                alias: alias.to_owned(),
                run_id: run_id.to_owned(),
                result: Err("this run has no merge commit to export".to_owned()),
            });
            return;
        };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let alias = alias.to_owned();
            let run_id = run_id.to_owned();
            tokio::task::spawn_blocking(move || {
                let result = crate::oneoff::land::export_merge_diff(&project, &run_dir, &sha);
                let _ = tx.send(Action::OneoffDiffExported {
                    alias,
                    run_id,
                    result,
                });
            });
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let _ = (project, sha, run_dir);
    }

    /// New lines from the viewer's tail: kept only for the viewer that asked.
    #[allow(clippy::too_many_arguments)]
    fn oneoff_stream_batch(
        &mut self,
        alias: &str,
        run_id: &str,
        epoch: u64,
        lines: Vec<String>,
        meter: crate::oneoff::stream::Meter,
        info: crate::ui::screens::oneoff_viewer::RunInfo,
        done: bool,
    ) {
        if !self.ctx.experimental {
            return;
        }
        if let Some(stream) = self
            .ctx
            .view_cache
            .get_mut(alias)
            .and_then(|cache| cache.oneoff_stream.as_mut())
            .filter(|s| s.epoch == epoch && s.run_id == run_id)
        {
            stream.push(lines);
            stream.meter = meter;
            stream.info = info;
            stream.done = done;
            self.needs_redraw = true;
        }
    }

    /// The viewer's composer: put a request in the run's inbox, off the render thread.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn request_oneoff_inject(
        &mut self,
        alias: &str,
        run_dir: PathBuf,
        request: crate::oneoff::inbox::InboxRequest,
    ) {
        let Some((_, tx)) = self.oneoff_target(alias) else {
            return;
        };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            use crate::oneoff::inbox::{send_message, set_soft_cap, InboxRequest};
            let alias = alias.to_owned();
            tokio::task::spawn_blocking(move || {
                let run_id = run_dir
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let result = match request {
                    InboxRequest::Message { text } => send_message(&run_dir, &text)
                        .map(|_| format!("Message queued for {run_id}")),
                    InboxRequest::SoftCap { usd } => set_soft_cap(&run_dir, usd)
                        .map(|_| format!("Soft cap request queued for {run_id}")),
                }
                .map_err(|e| e.to_string());
                let _ = tx.send(Action::OneoffInjectDone { alias, result });
            });
        }
    }

    /// The registered project and the event channel a one-off action needs, refusing
    /// visibly (S4) when the surfaces are off, the platform cannot run one-offs, the
    /// project is unknown or there is no channel.
    fn oneoff_target(
        &mut self,
        alias: &str,
    ) -> Option<(PathBuf, tokio::sync::mpsc::UnboundedSender<Action>)> {
        if !self.ctx.experimental {
            return None;
        }
        if !crate::ui::screens::driver_steer::can_steer() {
            self.ctx.status_message = Some((
                crate::ui::screens::driver_steer::STEERING_UNSUPPORTED_PLATFORM.to_string(),
                std::time::Instant::now(),
            ));
            self.needs_redraw = true;
            return None;
        }
        let path = self.project_path_for(alias)?;
        let Some(tx) = self.ctx.event_tx.clone() else {
            self.ctx.error_message = Some(format!(
                "Could not reach the one-off seam for '{alias}': this session has no event channel."
            ));
            self.needs_redraw = true;
            return None;
        };
        Some((path, tx))
    }

    /// `t`: list the project's pending todos off the render thread.
    fn request_oneoff_todos(&mut self, alias: &str) {
        let Some((path, tx)) = self.oneoff_target(alias) else {
            return;
        };
        let alias = alias.to_string();
        tokio::task::spawn_blocking(move || {
            let todos = crate::oneoff::todos::list_pending(&path).map_err(|e| e.to_string());
            let _ = tx.send(Action::OneoffTodosListed { alias, todos });
        });
    }

    /// The confirm popup's Enter: start a detached one-off through [`OneoffSeams`].
    /// The registry reads happen here, on this thread; the TUI never offers
    /// `--unsandboxed` (C-09).
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn request_oneoff_run(
        &mut self,
        alias: &str,
        mode: crate::oneoff::Mode,
        task: crate::oneoff::TaskSpec,
        model: String,
    ) {
        let Some((project, tx)) = self.oneoff_target(alias) else {
            return;
        };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let egress_allow = match self
                .ctx
                .config
                .projects
                .get(alias)
                .map(crate::config::RegisteredProject::driver_egress_allowlist)
                .transpose()
            {
                Ok(hosts) => hosts.unwrap_or_default(),
                Err(error) => {
                    self.ctx.status_message = Some((
                        format!(
                            "Not starting a one-off: projects.{}.{}",
                            crate::text::render_for_terminal(alias),
                            crate::text::render_for_terminal(&error.to_string())
                        ),
                        std::time::Instant::now(),
                    ));
                    self.needs_redraw = true;
                    return;
                }
            };
            let defaults = crate::cli::task_run_defaults();
            let options = crate::oneoff::launch::OneoffOptions {
                project,
                alias: alias.to_string(),
                mode,
                task,
                model,
                max_budget_usd: defaults.max_budget_usd,
                cap_minutes: defaults.cap_minutes,
                unsandboxed: false,
                run_id: None,
                egress_allow,
            };
            let config_path = self.ctx.config_path.clone();
            let run = self.oneoff_seams.run;
            let alias_for_task = alias.to_string();
            tokio::task::spawn_blocking(move || {
                let result = run(&options, Some(&config_path))
                    .map(|started| started.run_id)
                    .map_err(|error| error.to_string());
                let _ = tx.send(Action::OneoffRunStarted {
                    alias: alias_for_task,
                    result,
                });
            });
            self.ctx.status_message = Some((
                format!(
                    "Starting a one-off on {}",
                    crate::text::render_for_terminal(alias)
                ),
                std::time::Instant::now(),
            ));
            self.needs_redraw = true;
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let _ = (project, tx);
    }

    /// A confirmed landing or kill from the `T` popup, on `spawn_blocking`.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(unused_variables)
    )]
    fn request_oneoff_command(
        &mut self,
        alias: &str,
        run_id: Option<String>,
        command: crate::oneoff::OneoffCommand,
    ) {
        let Some((project, tx)) = self.oneoff_target(alias) else {
            return;
        };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            use crate::oneoff::OneoffCommand;
            // A merge landing may need a conflict-resolver session, which runs with the
            // project's egress allowlist as a run does (quick 261103-hx7, C-20, I-12).
            let egress_allow = if command == OneoffCommand::Merge {
                match self
                    .ctx
                    .config
                    .projects
                    .get(alias)
                    .map(crate::config::RegisteredProject::driver_egress_allowlist)
                    .transpose()
                {
                    Ok(hosts) => hosts.unwrap_or_default(),
                    Err(error) => {
                        self.ctx.status_message = Some((
                            format!(
                                "Not merging: projects.{}.{}",
                                crate::text::render_for_terminal(alias),
                                crate::text::render_for_terminal(&error.to_string())
                            ),
                            std::time::Instant::now(),
                        ));
                        self.needs_redraw = true;
                        return;
                    }
                }
            } else {
                Vec::new()
            };
            let alias_for_task = alias.to_string();
            tokio::task::spawn_blocking(move || {
                let id = run_id.unwrap_or_default();
                let result = match command {
                    // The land runs off the render thread with a never-set stop flag: if
                    // the TUI exits, bwrap's `--die-with-parent` ends the resolver.
                    OneoffCommand::Merge => crate::oneoff::land::merge(
                        &project,
                        &id,
                        &crate::oneoff::land::LandOptions::new(egress_allow),
                    ),
                    OneoffCommand::Push => match crate::launcher::config_dir::driver_root() {
                        Some(root) => {
                            crate::oneoff::land::push(&project, &alias_for_task, &id, &root)
                        }
                        None => Err("no per-user data directory is available".to_owned()),
                    },
                    OneoffCommand::Discard => crate::oneoff::land::discard(&project, &id),
                    OneoffCommand::Stop => {
                        match crate::oneoff::launch::stop(
                            &project,
                            (!id.is_empty()).then_some(id.as_str()),
                            std::time::Duration::from_secs(60),
                        ) {
                            Ok(outcome) => Ok(outcome.message()),
                            Err(error) => Err(error.to_string()),
                        }
                    }
                };
                let message = match result {
                    Ok(done) => done.replace('\n', "; "),
                    Err(why) => format!("{} refused: {}", command.verb(), why.replace('\n', "; ")),
                };
                let _ = tx.send(Action::OneoffCommandDone {
                    alias: alias_for_task,
                    message,
                });
            });
            self.ctx.status_message = Some((
                format!(
                    "One-off {} on {}...",
                    command.verb(),
                    crate::text::render_for_terminal(alias)
                ),
                std::time::Instant::now(),
            ));
            self.needs_redraw = true;
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let _ = (project, tx);
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
                crate::ui::screens::driver_steer::STEERING_UNSUPPORTED_PLATFORM.to_string(),
            );
            return;
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn request_new_run_check(&mut self, _alias: &str, _run_id: &str) {
        self.refuse_steering(
            crate::ui::screens::driver_steer::STEERING_UNSUPPORTED_PLATFORM.to_string(),
        );
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
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
                crate::ui::screens::driver_steer::STEERING_UNSUPPORTED_PLATFORM.to_string(),
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

    // The one-line `App::schedule_*` wrappers that stood here are gone (WR-09). They
    // delegated to the identically named `AppContext` methods and neither had a
    // caller anywhere in `src/` or `tests/`: every real call goes to `ctx.schedule_*`
    // directly, because a
    // `Screen` is handed an `&mut AppContext` and never an `&mut App`. They were
    // `pub`, so no `dead_code` warning fired and the duplication was invisible to
    // the compiler. `App`'s own callers, such as the tick rescan, go through
    // `self.ctx` for the same reason.

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

                // The selected launcher run's heartbeat age (30-02, D-08): gated on the
                // Driver tab being on top with a live run, on this tick, with no timer of
                // its own. The age is `now - heartbeat`, so it differs every tick forever;
                // an unconditional redraw would repaint an idle dashboard four times a
                // second for the life of the process.
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

                    // The launcher-marker scan rides THIS counter and must never
                    // get a timer of its own (D-13, ARCHITECTURE §4.4(c) are both
                    // explicit). Two timers reading run state at slightly
                    // different phases would double the syscall load for no extra
                    // freshness and would make "how stale can the dashboard be?" a
                    // question with two answers. Do not tidy this into its own
                    // interval.
                    //
                    // `spawn_blocking` for the same reason as the line above:
                    // lock probes and `DIRECTION.md` reads are synchronous fs work
                    // and this file's idiom for that is unambiguous.
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        let projects = self.ctx.config.projects.clone();
                        // The launcher-run markers (30-03, D-13): one `read_dir` plus
                        // bounded reads per project, on the same counter, never a
                        // timer of their own. With the experimental flag off no
                        // launcher surface is shown, so the reads are skipped
                        // altogether.
                        let experimental = self.ctx.experimental;
                        tokio::task::spawn_blocking(move || {
                            let launcher_markers = if experimental {
                                launcher_markers_for(&projects, std::time::SystemTime::now())
                            } else {
                                HashMap::new()
                            };
                            let _ = tx.send(Action::LauncherMarkersRescanned { launcher_markers });
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
            // The launcher-run markers (30-03, D-13), replaced wholesale: the scan is
            // authoritative, so a project absent from it has no marker, and one never
            // scanned has none either (no guessed class). A merge would keep a badge
            // after its project was unregistered or its run ended; the absence of an
            // entry is how the scan says both. The filter and the attention sort read
            // the marker, so they are recomputed when any marker moved. The redraw is
            // equality-guarded (`apply_launcher_markers` returns whether anything
            // changed): a scan lands every ~5s for the life of the process.
            Action::LauncherMarkersRescanned { launcher_markers } => {
                if self.ctx.apply_launcher_markers(&launcher_markers) {
                    self.ctx.recompute_filtered_aliases();
                    self.needs_redraw = true;
                }
            }
            // The agents scan is authoritative, so `agent_views` is **replaced**
            // and never merged — the `LauncherMarkersRescanned` rule above (D-C01).
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
                    // `◒N` is a pending action (quick 260929-szq, I-7).
                    // Worktree-held plans are something to show too, in the
                    // Phases pane and on the Roadmap (quick 261014-0jv); the
                    // dashboard cell is unchanged because `summary_forms`
                    // yields no form for them (ID-10). The same goes for a
                    // workstream's own lists (quick 261023-05i): a scan entry
                    // exists only when one of them is non-empty, and the
                    // dashboard cell reads the project's view alone (I-5).
                    if agents.rows.is_empty()
                        && agents.worktreeless.is_empty()
                        && agents.unmerged.is_empty()
                        && agents.worktree_plans.is_empty()
                        && agents.workstreams.is_empty()
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
            // 28-07, D-11: stop a launcher run through the library function the
            // `driver stop` CLI calls. Linux-only like the launcher itself.
            Action::LauncherStopRequested { alias, run_id } => {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                self.stop_launcher_run(&alias, &run_id);
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
            // rule. Every arm that reaches a write has its unsupported-platform refusal in
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
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                self.request_launch_preflight(&alias, &run_id);
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                self.request_launch(&alias, &run_id, args);
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
            // Quick 261103-f5y: one-off tasks. Every arm is inert with the
            // experimental surfaces off; the reads, the launch and the landings run on
            // `spawn_blocking`.
            Action::OneoffTodosRequested { alias } => self.request_oneoff_todos(&alias),
            Action::OneoffTodosListed { alias, todos } => {
                if self.ctx.experimental {
                    self.ctx.view_cache.entry(alias).or_default().oneoff_todos = Some(todos);
                    self.needs_redraw = true;
                }
            }
            Action::OneoffStatusRefreshed { alias, runs } => {
                if self.ctx.experimental {
                    self.ctx.view_cache.entry(alias).or_default().oneoff_runs = runs;
                    self.needs_redraw = true;
                }
            }
            Action::OneoffViewerOpen {
                alias,
                run_id,
                attempt,
            } => self.open_oneoff_viewer(&alias, &run_id, attempt),
            Action::OneoffDiffRequested { alias, run_id } => {
                self.request_oneoff_diff(&alias, &run_id)
            }
            Action::OneoffDiffLoaded {
                alias,
                run_id,
                result,
            } => {
                if self.ctx.experimental {
                    self.ctx.view_cache.entry(alias).or_default().oneoff_diff =
                        Some((run_id, result));
                    self.needs_redraw = true;
                }
            }
            Action::OneoffDiffExportRequested { alias, run_id } => {
                self.request_oneoff_diff_export(&alias, &run_id)
            }
            Action::OneoffDiffExported { result, .. } => {
                match result {
                    Ok(path) => self.pending_editor = Some((path, None)),
                    Err(why) => {
                        self.ctx.status_message = Some((
                            format!("merge diff export failed: {why}"),
                            std::time::Instant::now(),
                        ));
                    }
                }
                self.needs_redraw = true;
            }
            Action::OneoffStreamBatch {
                alias,
                run_id,
                epoch,
                lines,
                meter,
                info,
                done,
            } => self.oneoff_stream_batch(&alias, &run_id, epoch, lines, meter, info, done),
            Action::OneoffInjectRequested {
                alias,
                run_dir,
                request,
            } => self.request_oneoff_inject(&alias, run_dir, request),
            Action::OneoffInjectDone { alias, result } => {
                if self.ctx.experimental {
                    let message = match result {
                        Ok(done) => done,
                        Err(why) => format!("Not sent to {alias}: {why}"),
                    };
                    self.ctx.status_message = Some((
                        crate::text::render_for_terminal(&message).to_string(),
                        std::time::Instant::now(),
                    ));
                    self.needs_redraw = true;
                }
            }
            Action::OneoffRunRequested {
                alias,
                mode,
                task,
                model,
            } => self.request_oneoff_run(&alias, mode, task, model),
            Action::OneoffCommandRequested {
                alias,
                run_id,
                command,
            } => self.request_oneoff_command(&alias, run_id, command),
            Action::OneoffRunStarted { alias, result } => {
                if self.ctx.experimental {
                    let message = match result {
                        Ok(run_id) => format!("One-off {run_id} started on {alias}"),
                        Err(why) => format!("One-off not started on {alias}: {why}"),
                    };
                    self.finish_oneoff(&alias, &message);
                }
            }
            Action::OneoffCommandDone { alias, message } => {
                if self.ctx.experimental {
                    self.finish_oneoff(&alias, &format!("{alias}: {message}"));
                }
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
        }
    }

    /// Drop driver state the registry no longer justifies (D-27).
    ///
    /// **The leak this closes.** The alias-keyed maps are keyed by alias and
    /// `registry::remove_project` does not touch them (it has no access to
    /// `AppContext`; see its doc, which names the cleanup sites), so an unregistered
    /// alias would otherwise keep its entries for the whole life of the process. A
    /// project that is no longer a key in `config.projects` cannot produce another
    /// refresh or another view, so every entry it owns is dead weight.
    ///
    /// (Phase 31 removed the journal-cursor and live-output maps, and with them the
    /// second pass that kept the newest `RETAIN_RUNS` run ids per alias: nothing in
    /// the TUI tails a journal any more.)
    ///
    /// Called from the **existing** 20-tick block, never from a timer of its own
    /// (D-13, D-27).
    fn prune_driver_maps(&mut self) {
        let registered = &self.ctx.config.projects;
        // `view_cache` joins the pass too, and plan 18-09 is what makes that
        // necessary rather than tidy. It is keyed by alias and was never pruned,
        // which cost little while it held browse listings and git logs for
        // projects the user had merely visited — but the driver payloads it holds
        // (the launcher rows, their detail and LOG tail) are exactly the per-alias
        // driver state the phase's carry-forward is about, so leaving them in an
        // unpruned map would satisfy the obligation for the maps it names while
        // breaking it for the one it does not.
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

    // --- debug session `workstreams-not-surfaced`: format_workstream_display ----

    use crate::state_reader::workstreams::WorkstreamState;

    /// A workstream with the given roadmap phases `(number, name)`.
    fn workstream(
        name: &str,
        milestone: &str,
        phases: &[(&str, &str)],
        declared: Option<u32>,
    ) -> WorkstreamState {
        WorkstreamState {
            name: name.to_string(),
            active: false,
            claim: None,
            state: ProjectState {
                milestone: milestone.to_string(),
                state_md_phase_number: declared.map(crate::state_reader::phase_num::PhaseNum::from),
                phases: phases
                    .iter()
                    .map(|(number, name)| RoadmapPhase {
                        number: (*number).to_string(),
                        name: (*name).to_string(),
                        description: String::new(),
                        completed: false,
                        total_plans: 0,
                        completed_plans: 0,
                        depends_on: Vec::new(),
                    })
                    .collect(),
                ..Default::default()
            },
        }
    }

    /// The milestone and the phase NUMBER come before the phase name, so a
    /// narrow Phase column clips the name and never the two facts that identify
    /// the line of work.
    #[test]
    fn a_workstream_line_leads_with_its_milestone_and_phase_number() {
        let ws = workstream(
            "v0-14-voice-assistant",
            "v0.14",
            &[("25", "Conversation Memory"), ("26", "Push-to-Talk Voice")],
            Some(26),
        );
        let line = format_workstream_display(&ws, chrono::Utc::now());
        assert_eq!(line, "ws v0.14 P26: Push-to-Talk Voice");
        assert!(line.find("v0.14").unwrap() < line.find("Push-to-Talk").unwrap());
    }

    /// Quick 261023-05m: with the claim lapsed the line names the phase that is
    /// really being built (28), not the directory-less frontier (27). A live
    /// claim still leads, and a workstream with nothing unfinished is unchanged.
    #[test]
    fn a_lapsed_workstream_line_names_its_real_unfinished_phase() {
        let td = tempfile::TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        let now = chrono::Utc::now();
        crate::state_reader::workstreams::write_sentriq_workstreams_fixture(&planning, now);
        let state = crate::state_reader::parse_project_state(&planning);
        let find = |name: &str| {
            state
                .workstreams
                .iter()
                .find(|w| w.name == name)
                .unwrap_or_else(|| panic!("{name} was not loaded"))
        };
        let (v14, v13) = (find("v0-14-voice-assistant"), find("v0-13-public-beta"));
        let lapsed = now + crate::state_reader::workstreams::MILESTONE_CLAIM_TTL;

        assert_eq!(
            format_workstream_display(v14, now),
            "ws v0.14 P26: Push-to-Talk Voice"
        );
        assert_eq!(
            format_workstream_display(v14, lapsed),
            "ws v0.14 P28: Mechanic Toolbox, Full Data Access and Voice-Started Actions"
        );
        assert_eq!(
            format_workstream_display(v13, lapsed),
            "ws v0.13 P22: Play Release Readiness"
        );
    }

    #[test]
    fn the_active_workstream_is_marked_and_a_milestone_less_one_shows_its_directory() {
        let mut ws = workstream("alpha", "", &[("1", "Foundation")], Some(1));
        assert_eq!(
            format_workstream_display(&ws, chrono::Utc::now()),
            "ws alpha P1: Foundation"
        );
        ws.active = true;
        assert_eq!(
            format_workstream_display(&ws, chrono::Utc::now()),
            "ws* alpha P1: Foundation"
        );
    }

    /// A workstream whose roadmap lists no phase has none to name; the line is
    /// still drawn, because the workstream exists.
    #[test]
    fn a_workstream_without_roadmap_phases_still_gets_a_line() {
        let ws = workstream("beta", "v0.9", &[], None);
        assert_eq!(
            format_workstream_display(&ws, chrono::Utc::now()),
            "ws v0.9"
        );
    }

    /// A phase the roadmap cannot name draws `Unknown`, as the project's own
    /// cell does, rather than a blank.
    #[test]
    fn an_unrecognised_active_phase_draws_unknown() {
        let ws = workstream("gamma", "v0.8", &[("1", "One")], Some(5));
        // Declared 5 is above the roadmap; the active-phase rule clamps up to
        // max(declared, frontier) = 5, which the roadmap does not list.
        assert_eq!(
            format_workstream_display(&ws, chrono::Utc::now()),
            "ws v0.8 P5: Unknown"
        );
    }

    /// The same honesty as the project cell: a STATE.md that could not be read
    /// says so instead of naming a phase derived from counts, and a repaired
    /// one carries the repaired marker.
    #[test]
    fn an_unreadable_or_repaired_workstream_state_md_is_admitted() {
        let mut ws = workstream("delta", "v0.7", &[("3", "Slice")], Some(3));
        ws.state.state_md_unreadable = true;
        let line = format_workstream_display(&ws, chrono::Utc::now());
        assert_eq!(line, format!("ws v0.7 {UNREADABLE_STATE_LABEL}"));
        assert!(!line.contains("P3"), "{line}");

        let mut ws = workstream("delta", "v0.7", &[("3", "Slice")], Some(3));
        ws.state.state_md_recovered = true;
        assert_eq!(
            format_workstream_display(&ws, chrono::Utc::now()),
            format!("ws {RECOVERED_STATE_MARKER}v0.7 P3: Slice")
        );
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

    // --- quick 261023-05j: aggregate status over root + workstreams ----------

    fn ws_with_status(status: &str) -> crate::state_reader::workstreams::WorkstreamState {
        crate::state_reader::workstreams::WorkstreamState {
            state: ProjectState {
                status: status.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn project_with(root: &str, workstreams: &[&str]) -> ProjectState {
        ProjectState {
            status: root.to_string(),
            workstreams: workstreams.iter().map(|s| ws_with_status(s)).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn ws_aggregate_sentriq_root_is_complete_but_project_is_active() {
        let td = tempfile::TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        crate::state_reader::workstreams::write_sentriq_workstreams_fixture(
            &planning,
            chrono::Utc::now(),
        );
        let state = crate::state_reader::parse_project_state(&planning);
        assert_eq!(classify_status(&state.status), StatusCategory::Complete);
        assert_eq!(project_status_category(&state), StatusCategory::Active);
    }

    #[test]
    fn ws_aggregate_flat_project_equals_classify_status() {
        for status in [
            "executing",
            "blocked",
            "ready to plan",
            "phase_complete",
            "verifying",
            "v1.5.0 milestone complete",
            "shipped",
            "",
        ] {
            let state = project_with(status, &[]);
            assert_eq!(
                project_status_category(&state),
                classify_status(status),
                "flat parity for {status:?}"
            );
        }
    }

    #[test]
    fn ws_aggregate_precedence_table() {
        let cases: Vec<(&str, Vec<&str>, StatusCategory)> = vec![
            ("phase_complete", vec!["shipped"], StatusCategory::Unknown),
            ("executing", vec!["blocked"], StatusCategory::Blocked),
            ("ready to plan", vec!["executing"], StatusCategory::Active),
            ("shipped", vec!["ready to plan"], StatusCategory::Idle),
            (
                "phase_complete",
                vec!["phase_complete", "phase_complete"],
                StatusCategory::Complete,
            ),
            (
                "phase_complete",
                vec!["ready to plan", "executing", "shipped"],
                StatusCategory::Active,
            ),
        ];
        for (root, wss, want) in cases {
            let forward = project_with(root, &wss);
            assert_eq!(project_status_category(&forward), want, "{root} + {wss:?}");
            let mut rev = wss.clone();
            rev.reverse();
            let backward = project_with(root, &rev);
            assert_eq!(
                project_status_category(&backward),
                want,
                "order independent: {root} + {rev:?}"
            );
        }
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

    /// A launcher-marker scan result carrying `markers`.
    fn rescanned(markers: Vec<(&str, crate::ui::screens::driver_run::LauncherMarker)>) -> Action {
        Action::LauncherMarkersRescanned {
            launcher_markers: markers
                .into_iter()
                .map(|(alias, marker)| (alias.to_string(), marker))
                .collect(),
        }
    }

    /// A marker of `class`, holding the run lock only when it is `Driven`.
    fn marker_of(
        class: crate::ui::screens::driver_run::MarkerClass,
    ) -> crate::ui::screens::driver_run::LauncherMarker {
        crate::ui::screens::driver_run::LauncherMarker {
            class: Some(class),
            any_live: class == crate::ui::screens::driver_run::MarkerClass::Driven,
        }
    }

    /// A write under the legacy `meta-manager/runs/<id>/` directory: inert since
    /// Phase 31 (`classify_change` maps it to `Ignored`), so it must neither tail nor
    /// re-parse nor touch the dedup map.
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

    #[tokio::test]
    async fn a_marker_rescan_replaces_the_marker_rather_than_merging_it() {
        use crate::ui::screens::driver_run::MarkerClass;
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(rescanned(vec![(
            OBS_ALIAS,
            marker_of(MarkerClass::Crashed),
        )]));
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS].launcher_marker,
            Some(marker_of(MarkerClass::Crashed))
        );

        // The new scan expresses "that run is gone" by NOT returning it, and a
        // merge would keep the crashed badge forever.
        app.update(rescanned(Vec::new()));
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS].launcher_marker, None,
            "the scan is authoritative: a marker it no longer returns must be \
             cleared, not merged"
        );
    }

    #[tokio::test]
    async fn an_unchanged_marker_rescan_does_not_request_a_redraw() {
        use crate::ui::screens::driver_run::MarkerClass;
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(rescanned(vec![(OBS_ALIAS, marker_of(MarkerClass::Driven))]));

        // The scan lands every ~5s for the whole life of the process. Without
        // the equality guard this would repaint twelve times a minute forever
        // with nothing on screen changed.
        app.needs_redraw = false;
        app.update(rescanned(vec![(OBS_ALIAS, marker_of(MarkerClass::Driven))]));
        assert!(
            !app.needs_redraw,
            "an identical scan result must not request a redraw"
        );

        // The control arm: a real change still does.
        app.update(rescanned(vec![(
            OBS_ALIAS,
            marker_of(MarkerClass::Crashed),
        )]));
        assert!(
            app.needs_redraw,
            "a run going from driven to crashed must request a redraw, or the guard \
             above is suppressing genuine updates too"
        );
    }

    /// The heartbeat gate wired through a real `Action::Tick`, not just the predicate.
    #[tokio::test]
    async fn a_tick_repaints_only_when_the_driver_tab_is_watching_a_live_launcher_run() {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, DerivedStatus, DriverRunView, RunStatus,
        };

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));
        let frontmatter = parse_driver_frontmatter(include_str!(
            "../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        {
            let cache = app.ctx.view_cache.entry(OBS_ALIAS.to_string()).or_default();
            cache.launcher_runs = vec![(
                "20261020-a".to_string(),
                Ok(DriverRunView {
                    frontmatter,
                    derived: DerivedStatus::AsWritten(RunStatus::Running),
                }),
            )];
            cache.launcher_listed = true;
        }

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
            "the heartbeat age must advance where it can actually be seen"
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

    /// The rescan gate is **visibility, not liveness**, and the difference is
    /// the whole finding.
    ///
    /// A run that has just ended is exactly when the run list's glyph and its
    /// outcome go stale and most need re-reading. Gating the rescan on
    /// `launcher_heartbeat_redraw_wanted` would stop it firing at the moment it
    /// started mattering.
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

        // And a tab with NO live run still satisfies it, where the heartbeat
        // gate does not.
        let empty = crate::ui::screens::ProjectViewCache::default();
        assert!(!launcher_heartbeat_redraw_wanted(
            DetailScreen::NAME,
            Some(&DetailSubView::Driver),
            Some(&empty)
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
        // No cache or an empty one: nothing to age.
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
    /// The launcher listing had only two call sites — tab entry and a file event
    /// under a run directory — so a lock released by `SIGKILL`, which emits no
    /// file event, never moved a run off `running` while the user watched.
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
                !matches!(action, Action::LauncherRunsRefreshed { .. }),
                "the rescan must ride the EXISTING counter — a scan before the \
                 twentieth tick means it got a timer of its own"
            );
        }

        app.update(Action::Tick);
        assert!(
            launcher_listing_reported(&mut rx).await.is_some(),
            "the twentieth tick must re-list the launcher runs while the Driver \
             tab is open"
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
        assert!(!text.contains("\u{29EB} running"), "{text}");

        // 2. The lock is held by this test: the same bytes now read running.
        let _lock = acquire_at(&run_lock_path(&run_dir), LAUNCHER_RUN, std::process::id())
            .expect("the test takes the run lock");
        app.update(run_file_change(dir.path(), "DIRECTION.md"));
        deliver_launcher_refresh(&mut app, &mut rx).await;
        let text = render_top_screen(&app);
        assert!(text.contains("\u{29EB} running"), "{text}");
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
            row("I-01 \u{2762} parked").contains("> I-01"),
            "the cursor starts on the parked item:\n{text}"
        );
        assert!(
            row("I-02 \u{2219} unseen").contains("second thing"),
            "{text}"
        );
        let nine = row("I-09 \u{25CD} done");
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
    /// for it. The listing is a `read_dir` and a lock probe per run, so an ungated
    /// rescan would cost the whole fleet for one pane's freshness.
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
                !matches!(action, Action::LauncherRunsRefreshed { .. }),
                "the dashboard is on top; nothing is looking at a run list"
            );
        }
    }

    /// The driver payloads live on `ProjectViewCache` (the launcher rows, their detail
    /// and the LOG tail), so the map that holds it joins the prune pass. Without
    /// this, the cache of a project the user unregistered would sit in memory for
    /// the life of the process.
    #[tokio::test]
    async fn pruning_drops_the_view_cache_for_an_unregistered_alias() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        for alias in ["gone", OBS_ALIAS] {
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.launcher_selected = 2;
            cache.launcher_selected_id = Some("20261019-a".to_string());
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
        assert_eq!(kept.launcher_selected, 2);
        assert_eq!(
            kept.launcher_selected_id.as_deref(),
            Some("20261019-a"),
            "the prune must not clear a live cache's contents"
        );
    }

    // ── The launcher rows and the sub-view ────────────────────────────────

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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    async fn launcher_stop_stopped_sets_the_status_line_and_relists_the_launcher_runs() {
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

        // The launcher listing follows: its rows' derived status is what the stop
        // just changed.
        let mut launcher = None;
        while launcher.is_none() {
            match next_action(&mut rx).await {
                Some(Action::LauncherRunsRefreshed {
                    alias,
                    launcher_runs,
                    ..
                }) => launcher = Some((alias, launcher_runs)),
                Some(_) => {}
                None => panic!("a re-list of the launcher runs must follow"),
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

    /// The interactive removal path cleans the driver maps immediately, rather
    /// than leaving them to the periodic backstop.
    #[tokio::test]
    async fn removing_a_project_interactively_clears_its_per_alias_maps() {
        use crate::ui::screens::delete_confirm::DeleteConfirmScreen;
        use crate::ui::screens::Screen;
        use crossterm::event::{KeyCode, KeyModifiers};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        // `save_config` must land somewhere real, or the removal reports a save
        // failure and returns before it reaches the cleanup block.
        app.ctx.config_path = dir.path().join("config.json");

        app.ctx
            .last_refresh
            .insert(OBS_ALIAS.to_string(), std::time::Instant::now());

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
            !app.ctx.last_refresh.contains_key(OBS_ALIAS),
            "last_refresh leaked past an interactive removal"
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
    async fn every_per_alias_map_is_pruned() {
        const GONE: &str = "gone";

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Populate every alias-keyed map for BOTH a registered alias and an
        // alias the registry has never heard of, so each assertion below has a
        // control arm and none of them can pass by emptying the map.
        for alias in [GONE, OBS_ALIAS] {
            // The launcher payloads live inside `view_cache`, so they inherit that
            // map's prune rather than needing their own — and this test is where
            // that inheritance is checked rather than assumed.
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.launcher_selected = 1;
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
            // Alias-keyed: every one of these must be pruned, and each is asserted
            // below.
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

            // Not alias-keyed at all: scalars, handles and single values.
            // `experimental` is the startup-resolved
            // experimental-features flag (260917-fko D1): one scalar bool, not
            // alias-keyed, never mutated after `from_config`, and therefore
            // nothing for the prune to reach.
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
            reparse_dispatches: _,
            sort_mode: _,
            watcher: _,
            detail_scroll_offset: _,
            suggestion_index: _,
            input_buffer: _,
            needs_redraw: _,
            active_sessions: _,
        } = &app.ctx;

        assert!(!view_cache.contains_key(GONE), "view_cache leaked");
        assert!(!last_refresh.contains_key(GONE), "last_refresh leaked");
        assert!(!archive_cache.has_alias(GONE), "archive_cache leaked");

        // The control arm for all of them at once: the registered alias kept
        // everything, so none of the assertions above passed because the prune
        // cleared the map.
        assert!(last_refresh.contains_key(OBS_ALIAS));
        assert!(archive_cache.get(OBS_ALIAS, "v1.0").is_some());
        let kept = view_cache
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its view cache");
        assert_eq!(kept.launcher_selected, 1);
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
    /// no agent row still gets a view, and its Status cell shows `◒N`.
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
            screen.contains("\u{25D2}1"),
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

    /// Quick 261023-05i: a scan whose only data sits in one workstream's own
    /// lists still gives the alias a view (the Detail tabs read its nested
    /// workstream view), and the dashboard Status cell does not move (I-5).
    #[tokio::test]
    async fn agents_scan_keeps_a_project_whose_only_worktree_work_is_in_a_workstream() {
        use crate::agents::unmerged::{UnmergedItem, UnmergedKey, UnmergedState};
        use crate::agents::{waves::PlanRef, ProjectAgents, WorkstreamAgents};
        use crate::state_reader::workstreams::WorkstreamState;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        let mut state = wave_state();
        state.workstreams.push(WorkstreamState {
            name: "v0-14-voice".to_string(),
            ..Default::default()
        });
        app.ctx.project_states.insert(OBS_ALIAS.to_string(), state);
        app.ctx.recompute_filtered_aliases();
        let before = render_top_screen(&app);

        let item = UnmergedItem {
            key: UnmergedKey::Plan(PlanRef::from_id("28-02").expect("valid id")),
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
                    workstreams: vec![WorkstreamAgents {
                        name: "v0-14-voice".to_string(),
                        unmerged: vec![item],
                        ..Default::default()
                    }],
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
            .expect("workstream-only work must give the alias a view");
        assert!(view.workstreams.contains_key("v0-14-voice"));
        assert_eq!(view.unmerged_in_phase("28"), 0, "never the project's");
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
    #[cfg(unix)]
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
    #[cfg(unix)]
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
        app.update(Action::LauncherMarkersRescanned {
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

    /// The scan rides the 20-tick block: on the twentieth tick with the flag on the
    /// `LauncherMarkersRescanned` it sends carries the marker, and with the flag off no
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
                    if let Action::LauncherMarkersRescanned { launcher_markers } = action {
                        markers = Some(launcher_markers);
                    }
                }
                if markers.is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let markers = markers.expect("the slow tick sends LauncherMarkersRescanned");
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        assert!(text.contains("\u{29EB} running"), "a live run:\n{text}");
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
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn launcher_steer_an_unsupported_platform_build_refuses_with_the_read_only_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(Action::DriverSteerRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
            op: crate::driver::steer::SteerOp::SetControl("pause".to_string()),
        });
        assert_eq!(
            app.ctx.error_message.as_deref(),
            Some("read-only: steering needs Linux or macOS")
        );
    }

    /// With no event channel the result could not be reported, so the request is
    /// refused visibly and nothing is written (S4).
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn edit_temp(root: &std::path::Path) -> PathBuf {
        root.join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
            .join(LAUNCHER_RUN)
            .join(crate::driver::steer::EDIT_TEMP_NAME)
    }

    /// A launcher run holding a lint-clean body (so an edit applies without the
    /// confirmation), the tab open on it with its detail loaded.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn launcher_edit_an_unsupported_platform_build_refuses_with_the_read_only_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.update(Action::DriverEditRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: LAUNCHER_RUN.to_string(),
        });
        assert_eq!(
            app.ctx.error_message.as_deref(),
            Some("read-only: steering needs Linux or macOS")
        );
        assert!(app.pending_editor.is_none());
    }

    // ── 30-05: launching and relaunching from the Driver tab ────────────────

    /// What the fake `launch` seam was given, per call: the options and the `--config`
    /// path. A `static` because the seam is a `fn` pointer and the call happens on a
    /// `spawn_blocking` thread; tests tell their own calls apart by the project path.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    type RecordedLaunch = (crate::launcher::run::LaunchOptions, Option<PathBuf>);

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    static RECORDED_LAUNCHES: std::sync::Mutex<Vec<RecordedLaunch>> =
        std::sync::Mutex::new(Vec::new());

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn fake_detect_ready() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 1, 295)),
            overlay: true,
        }
    }

    /// The launches the fake seam recorded for the project at `root`.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn top_screen_name(app: &App) -> String {
        app.screen_stack
            .last()
            .map(|s| s.name().to_string())
            .unwrap_or_default()
    }

    /// `R`, through the key handler and the App, until the form has its host check.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn launcher_launch_an_unsupported_platform_build_refuses_and_starts_nothing() {
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn fake_detect_degraded() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: vec!["bwrap".to_string()],
            claude_version: Some((2, 1, 295)),
            overlay: false,
        }
    }

    /// A host whose claude is older than the hook field the gate relies on.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn fake_detect_old_claude() -> crate::launcher::host::HostCaps {
        crate::launcher::host::HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 0, 1)),
            overlay: true,
        }
    }

    /// Whether any `DriverLaunchRequested` is waiting on the channel.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        // The watcher reports paths under the registered project path. `run.direction`
        // is canonical, which differs from the temp dir on macOS (`/var` is a link to
        // `/private/var`), so the event path is rebuilt from the project path.
        app.update(Action::FileChanged {
            project_path: dir.path().to_path_buf(),
            changed_path: dir
                .path()
                .join(crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL)
                .join(&run.run_id)
                .join("DIRECTION.md"),
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn today() -> String {
        chrono::Utc::now().format("%Y%m%d").to_string()
    }

    /// A GSD project (`.planning/` exists) on the Driver tab, nothing listed yet, with
    /// the fake launch seams.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        // Ask for a listing now rather than waiting for the one `create_a_run` set
        // going. Creating the run starts that listing and the editor's prepare as two
        // independent tasks, and `create_a_run` skips every action but the editor's:
        // when the listing lands first it is skipped with them, and waiting for it here
        // timed out after 10 s ("the launcher listing never reported") whenever the
        // machine was busy. The run exists on disk by now, so any listing shows it.
        let project = app.ctx.config.projects[OBS_ALIAS].path.clone();
        app.ctx.schedule_launcher_refresh(OBS_ALIAS, &project);
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    // ── quick 261103-f5y: the one-off seam ───────────────────────────────

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    static RECORDED_ONEOFFS: std::sync::Mutex<
        Vec<(crate::oneoff::launch::OneoffOptions, Option<PathBuf>)>,
    > = std::sync::Mutex::new(Vec::new());

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn fake_oneoff(
        options: &crate::oneoff::launch::OneoffOptions,
        config: Option<&Path>,
    ) -> Result<crate::oneoff::launch::Started, crate::launcher::LauncherError> {
        RECORDED_ONEOFFS
            .lock()
            .expect("the recorder")
            .push((options.clone(), config.map(Path::to_path_buf)));
        Ok(crate::oneoff::launch::Started {
            run_id: "oneoff-20261103-000000".to_owned(),
            pid: 4242,
            log: PathBuf::from("/x/launch.log"),
        })
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_confirmed_one_off_goes_through_the_seam_with_the_cli_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.oneoff_seams = OneoffSeams { run: fake_oneoff };
        app.update(Action::OneoffRunRequested {
            alias: OBS_ALIAS.to_string(),
            mode: crate::oneoff::Mode::Fast,
            task: crate::oneoff::TaskSpec::Text("fix x".to_owned()),
            model: "sonnet".to_owned(),
        });
        let started =
            next_matching(&mut rx, |a| matches!(a, Action::OneoffRunStarted { .. })).await;
        let Action::OneoffRunStarted { result, .. } = &started else {
            unreachable!()
        };
        assert_eq!(result.as_deref(), Ok("oneoff-20261103-000000"));
        let recorded = RECORDED_ONEOFFS
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(o, _)| o.task == crate::oneoff::TaskSpec::Text("fix x".to_owned()))
            .cloned()
            .expect("the seam was called");
        let defaults = crate::cli::task_run_defaults();
        let (options, config) = recorded;
        assert_eq!(options.alias, OBS_ALIAS);
        assert_eq!(options.mode, crate::oneoff::Mode::Fast);
        assert_eq!(options.model, "sonnet");
        assert_eq!(options.max_budget_usd, defaults.max_budget_usd);
        assert_eq!(options.cap_minutes, defaults.cap_minutes);
        assert!(!options.unsandboxed, "the TUI never offers --unsandboxed");
        assert_eq!(config.as_deref(), Some(app.ctx.config_path.as_path()));
        app.update(started);
        let message = app
            .ctx
            .status_message
            .clone()
            .map(|(t, _)| t)
            .unwrap_or_default();
        assert!(message.contains("started"), "{message}");
    }

    #[test]
    fn one_off_results_fill_the_view_cache_and_are_inert_with_the_flag_off() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _rx) = app_on_the_driver_tab(dir.path());
        let todos = vec![crate::oneoff::todos::PendingTodo {
            file_name: "a.md".to_owned(),
            title: "A".to_owned(),
        }];
        app.ctx.experimental = false;
        app.update(Action::OneoffTodosListed {
            alias: OBS_ALIAS.to_string(),
            todos: Ok(todos.clone()),
        });
        assert!(app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .is_none_or(|c| c.oneoff_todos.is_none()));
        app.update(Action::OneoffRunRequested {
            alias: OBS_ALIAS.to_string(),
            mode: crate::oneoff::Mode::Quick,
            task: crate::oneoff::TaskSpec::Text("x".to_owned()),
            model: "opus".to_owned(),
        });
        assert!(app.ctx.status_message.is_none(), "inert with the flag off");

        app.ctx.experimental = true;
        app.update(Action::OneoffTodosListed {
            alias: OBS_ALIAS.to_string(),
            todos: Ok(todos.clone()),
        });
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].oneoff_todos, Some(Ok(todos)));
        let view = crate::ui::screens::oneoff::tests::finished_view("x");
        app.update(Action::OneoffStatusRefreshed {
            alias: OBS_ALIAS.to_string(),
            runs: vec![view.clone()],
        });
        assert_eq!(app.ctx.view_cache[OBS_ALIAS].oneoff_runs, vec![view]);
    }

    /// A run with a resolved merge landing, in the project's cache.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn cached_resolved_run(app: &mut App) -> String {
        use crate::oneoff::store::{Landing, LandingKind, MergeHow};
        let mut view = crate::ui::screens::oneoff::tests::finished_view("x");
        view.land_attempts = vec![1, 2];
        view.record.landed.push(Landing {
            kind: LandingKind::Merged,
            at: "2026-11-03T11:00:00Z".to_owned(),
            detail: "main".to_owned(),
            how: Some(MergeHow::Resolved),
            merge_sha: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
            conflicted: vec!["shared.txt".to_owned()],
            attempts: vec!["land/1".to_owned()],
            range: None,
        });
        let id = view.record.run_id.clone();
        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .oneoff_runs = vec![view];
        id
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_merge_diff_is_read_off_the_render_thread_and_kept_under_its_run() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        let id = cached_resolved_run(&mut app);

        // The flag off: nothing is asked and nothing is stored.
        app.ctx.experimental = false;
        app.update(Action::OneoffDiffRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: id.clone(),
        });
        app.update(Action::OneoffDiffLoaded {
            alias: OBS_ALIAS.to_string(),
            run_id: id.clone(),
            result: Ok(vec!["x".to_owned()]),
        });
        assert!(app.ctx.view_cache[OBS_ALIAS].oneoff_diff.is_none());
        assert!(rx.try_recv().is_err(), "inert with the flag off");

        app.ctx.experimental = true;
        // The project is not a git repository, so the blocking read answers an error:
        // the answer comes through the channel, not from the update call.
        app.update(Action::OneoffDiffRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: id.clone(),
        });
        assert!(app.ctx.view_cache[OBS_ALIAS].oneoff_diff.is_none());
        let loaded = next_matching(&mut rx, |a| matches!(a, Action::OneoffDiffLoaded { .. })).await;
        let Action::OneoffDiffLoaded { run_id, result, .. } = &loaded else {
            unreachable!()
        };
        assert_eq!(run_id, &id);
        assert!(result.is_err(), "{result:?}");
        app.update(loaded);
        let (stored, result) = app.ctx.view_cache[OBS_ALIAS].oneoff_diff.clone().unwrap();
        assert_eq!(stored, id);
        assert!(result.is_err());
    }

    #[test]
    fn an_exported_diff_opens_in_the_editor_and_a_failed_export_only_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _rx) = app_on_the_driver_tab(dir.path());
        app.update(Action::OneoffDiffExported {
            alias: OBS_ALIAS.to_string(),
            run_id: "oneoff-20261103-142233".to_owned(),
            result: Err("disk full".to_owned()),
        });
        assert!(app.pending_editor.is_none(), "no editor opens on a failure");
        let message = app
            .ctx
            .status_message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default();
        assert!(message.contains("disk full"), "{message}");
        let path = dir.path().join("merge-0123456789ab.diff");
        app.update(Action::OneoffDiffExported {
            alias: OBS_ALIAS.to_string(),
            run_id: "oneoff-20261103-142233".to_owned(),
            result: Ok(path.clone()),
        });
        assert_eq!(app.pending_editor, Some((path, None)));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_run_without_a_merge_gets_an_error_answer_instead_of_a_git_read() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, mut rx) = app_on_the_driver_tab(dir.path());
        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .oneoff_runs = vec![crate::ui::screens::oneoff::tests::finished_view("x")];
        app.update(Action::OneoffDiffRequested {
            alias: OBS_ALIAS.to_string(),
            run_id: "oneoff-20261103-142233".to_owned(),
        });
        let loaded = rx.try_recv().expect("answered at once, no blocking task");
        assert!(matches!(
            loaded,
            Action::OneoffDiffLoaded { result: Err(why), .. } if why.contains("no merge commit")
        ));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_viewer_opens_only_on_a_cached_run_and_a_real_resolver_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _rx) = app_on_the_driver_tab(dir.path());
        let id = cached_resolved_run(&mut app);
        let depth = app.screen_stack.len();
        let refused = |app: &App| {
            app.ctx
                .status_message
                .as_ref()
                .map(|(m, _)| m.clone())
                .unwrap_or_default()
        };

        app.update(Action::OneoffViewerOpen {
            alias: OBS_ALIAS.to_string(),
            run_id: "oneoff-20261103-000001".to_owned(),
            attempt: None,
        });
        assert!(
            refused(&app).contains("No such one-off run"),
            "{}",
            refused(&app)
        );
        app.ctx.status_message = None;
        app.update(Action::OneoffViewerOpen {
            alias: OBS_ALIAS.to_string(),
            run_id: id,
            attempt: Some(7),
        });
        assert!(
            refused(&app).contains("no such resolver"),
            "{}",
            refused(&app)
        );
        assert_eq!(app.screen_stack.len(), depth, "no viewer was opened");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn the_viewer_follows_the_chosen_resolver_attempt_dir() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _rx) = app_on_the_driver_tab(dir.path());
        let id = cached_resolved_run(&mut app);
        let depth = app.screen_stack.len();
        app.update(Action::OneoffViewerOpen {
            alias: OBS_ALIAS.to_string(),
            run_id: id,
            attempt: Some(2),
        });
        assert_eq!(app.screen_stack.len(), depth + 1);
        let stream = app.ctx.view_cache[OBS_ALIAS]
            .oneoff_stream
            .as_ref()
            .unwrap();
        assert_eq!(stream.attempt, Some(2));
        assert!(stream.run_dir.ends_with("land/2"), "{:?}", stream.run_dir);
        // No landing is in progress, so the finished resolver session is not live.
        assert_eq!(stream.info.live, crate::oneoff::store::LiveState::Finished);
        assert_eq!(stream.info.soft_cap_usd, None);
    }
}
