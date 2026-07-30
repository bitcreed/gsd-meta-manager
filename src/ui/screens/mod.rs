pub mod add_project;
pub mod create_project;
pub mod delete_confirm;
pub mod detail;
pub mod driver_confirm;
pub mod driver_inject;
pub mod driver_start;
pub mod enqueue;
pub mod help;
pub mod normal;
pub mod queue_delete_confirm;

use crate::action::Action;
use crate::app::DetailSubView;
use crate::change_tracker::ChangeTracker;
use crate::config::Config;
use crate::executor::RunState;
use crate::main_loop::ExecEvent;
use crate::state_reader::backlog::BacklogItem;
use crate::state_reader::git_ops::{GitDiffStat, GitLogEntry};
use crate::state_reader::ProjectState;
use crate::watcher::FileWatcher;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::TableState;
use ratatui::Frame;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use tokio::sync::mpsc::{Sender, UnboundedSender};

pub trait Screen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext);
    fn name(&self) -> &str;
}

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    Quit,
    SetStatusMessage(String),
    /// Suspend the TUI and open a file in $EDITOR.
    SuspendAndEdit(std::path::PathBuf),
    /// Used by archive browser (Phase 12) to dispatch async load actions.
    #[allow(dead_code)]
    DispatchAction(Box<Action>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultsEditTarget {
    #[default]
    Project,
    Global,
}

// ── Live driver output: the first bounded in-memory collection in this repo ──
//
// `grep -rn VecDeque src/` returned nothing before this block, so there was no
// precedent to imitate — only the conventions the three constants below and the
// sanitiser follow deliberately (D-18).

/// How many rendered lines one alias's live-output ring holds before the oldest
/// are dropped.
///
/// The bound is what stops a multi-hour run growing this buffer without limit:
/// a driven run emits stream events continuously for hours, and one 68-second
/// spike turn alone emitted 22 `thinking_tokens` events. At
/// [`DRIVER_OUTPUT_LINE_CELLS`] characters per line this bounds one alias to
/// roughly 400 KB.
///
/// 2000 is a defensible starting value with **no tuning data behind it**
/// (the [`crate::main_loop::EXEC_BATCH`] idiom). It is a named constant so
/// tuning is a one-line change.
pub const DRIVER_OUTPUT_RING_LINES: usize = 2_000;

/// The hard per-line truncation applied at append time, by **character** count.
///
/// A line longer than this is cut and suffixed with a single ellipsis. The count
/// is in `char`s and never bytes: byte slicing panics on a multibyte boundary,
/// and every string in this buffer originated on disk in an agent's output,
/// where multibyte content is routine rather than exotic.
///
/// 512 is a defensible starting value with **no tuning data behind it** — it is
/// comfortably wider than any terminal this tool is used in, so the truncation
/// is a memory bound rather than a display decision. Named so tuning is a
/// one-line change.
pub const DRIVER_OUTPUT_LINE_CELLS: usize = 512;

/// How many lines one journal record may expand into before the rest is cut.
///
/// One record's text is split on newlines before it is buffered, so a single
/// pathological record could otherwise flush the whole ring in one append and
/// erase every line that preceded it. This cap is what makes that impossible.
///
/// 64 is a defensible starting value with **no tuning data behind it**; it is
/// generous for an assistant message and small against
/// [`DRIVER_OUTPUT_RING_LINES`]. Named so tuning is a one-line change.
pub const DRIVER_OUTPUT_RECORD_MAX_LINES: usize = 64;

/// The escape character, stripped unconditionally by [`sanitize_render_line`].
const ESC: char = '\u{1b}';

/// What every other C0 control character and `DEL` is replaced with: `·`.
///
/// A `\u{…}` escape rather than the literal glyph, following the house rule that
/// no raw glyph appears in source (`normal.rs:69-74`).
const CONTROL_REPLACEMENT: char = '\u{00b7}';

/// The truncation marker appended to a line cut at [`DRIVER_OUTPUT_LINE_CELLS`]: `…`.
const ELLIPSIS: char = '\u{2026}';

/// Spaces one tab expands to.
const TAB_WIDTH: usize = 4;

/// Which journal source a buffered line came from, and therefore how it renders.
///
/// The kind is carried on the line rather than re-derived at render time because
/// the record it came from is gone by then — the buffer holds owned `String`s
/// and plain data, never a borrow of the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverLineKind {
    /// `ExecEvent` on the assistant / user / turn streams — the common case.
    Output,
    /// `ExecEvent` on the `stderr` stream: present but secondary.
    Stderr,
    /// An interjection record: queued, delivered, acted-on or missed.
    Injection,
    /// `Diagnostic`, `EventsDropped`, `JournalTruncated` — never silently swallowed.
    Diagnostic,
    /// The terminal record (`RunEnded` / `ExecFinished`): the visual full stop.
    Terminal,
}

/// One sanitised, already-truncated line of live driver output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverOutputLine {
    /// Which journal source produced it.
    pub kind: DriverLineKind,
    /// The rendered text, already through [`sanitize_render_line`].
    pub text: String,
}

/// One alias's bounded live-output ring (D-18).
///
/// * A **sibling map value** on [`AppContext`], shaped like `run_states` and
///   `journal_cursors`. It deliberately does NOT live on `ProjectState`: that
///   type derives `PartialEq`, and `app.rs` uses the derived equality to
///   suppress the "Updated: {alias}" status message. Live output changes every
///   few seconds, so a field there would flood the status bar for an entire
///   multi-hour run — defeating a deliberate v1.4 feature for the whole duration
///   of the thing it is meant to report (ARCHITECTURE AP1).
/// * It holds owned `String`s, an enum and two counters — **never a file handle
///   and never a join handle**. `Action` derives `Clone` and a handle is not
///   `Clone` (D-20).
/// * **The map is pruned**, on the same 20-tick block as the reconciliation
///   probe, in `App::prune_driver_maps`. A new per-alias map that is not pruned
///   reintroduces the Phase 16 leak under a new name; this is the phase's only
///   remaining carry-forward obligation and it is a negative one.
///
/// The inner collection is **private on purpose**: the cap is enforced in
/// [`push_record`](DriverOutput::push_record) and a caller able to push directly
/// could bypass it, which is exactly the "a `Vec` that grows" failure the phase
/// context names.
#[derive(Debug, Default)]
pub struct DriverOutput {
    lines: VecDeque<DriverOutputLine>,
    dropped: u64,
    record_truncated: bool,
}

impl DriverOutput {
    /// Sanitise one journal record's text and append it, dropping the oldest
    /// lines while over [`DRIVER_OUTPUT_RING_LINES`].
    ///
    /// **The cap asymmetry follows `journal/writer.rs:97-118`:** the counter is
    /// the mechanism, not a per-drop log. Every dropped line increments
    /// [`dropped`](DriverOutput::dropped) and nothing else happens; the render
    /// layer shows one first line naming the count. A per-drop notice would
    /// itself be a line, and the buffer would spend its whole capacity
    /// announcing its own overflow.
    pub fn push_record(&mut self, kind: DriverLineKind, raw: &str) {
        let (lines, truncated) = sanitize_record_lines(raw);
        if truncated {
            self.record_truncated = true;
        }
        for text in lines {
            self.lines.push_back(DriverOutputLine { kind, text });
            while self.lines.len() > DRIVER_OUTPUT_RING_LINES {
                self.lines.pop_front();
                self.dropped = self.dropped.saturating_add(1);
            }
        }
    }

    /// The buffered lines, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &DriverOutputLine> {
        self.lines.iter()
    }

    /// How many lines the ring has dropped over this alias's whole session.
    ///
    /// Monotonic: it counts drops, not the current shortfall, so the rendered
    /// notice reads "{N} earlier lines dropped" truthfully after any number of
    /// wraps.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Whether **any** record appended to this buffer was cut at
    /// [`DRIVER_OUTPUT_RECORD_MAX_LINES`].
    ///
    /// Deliberately sticky rather than tracking only the most recent record: a
    /// pane that silently loses lines is precisely the "looks done but isn't"
    /// failure the phase context enumerates, and a flag that a later ordinary
    /// record clears would lose the signal.
    pub fn record_truncated(&self) -> bool {
        self.record_truncated
    }

    /// How many lines are currently buffered. Never exceeds
    /// [`DRIVER_OUTPUT_RING_LINES`].
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether nothing has been buffered yet.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Drop every buffered line and reset both overflow signals.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
        self.record_truncated = false;
    }
}

/// Make one string from disk safe to hand to the terminal.
///
/// **Every string this phase renders that originated on disk is untrusted**, and
/// this is the single append-time gate all of them pass through (18-UI-SPEC,
/// `## Untrusted Input Boundary`). Agent prose lands in `ExecEvent.text`,
/// `Diagnostic.detail`, `RunRecord.goal` on re-read and `gsd_command`.
///
/// The rules, applied in this order:
///
/// 1. **`ESC` (`0x1B`) is stripped unconditionally.** This is the single
///    highest-value rule in the boundary: without it, agent prose can emit
///    ANSI/OSC sequences that repaint the screen, forge a status line, move the
///    cursor, or set the window title. Stripping the introducer is what makes
///    the rest of a sequence inert text.
/// 2. `\t` expands to [`TAB_WIDTH`] spaces.
/// 3. Every other C0 control character (`0x00`–`0x1F`) and `DEL` (`0x7F`) is
///    replaced with [`CONTROL_REPLACEMENT`] — present, visible, and harmless.
/// 4. The result is truncated by **`char`** count to
///    [`DRIVER_OUTPUT_LINE_CELLS`] and suffixed with [`ELLIPSIS`]. Byte slicing
///    panics on a multibyte boundary, so the truncation is a `char` operation
///    and there is a test for exactly that.
///
/// This is built **beside** [`crate::journal::redact::RedactedLine`] rather than
/// on top of it: that type caps payload bytes and redacts secrets for what is
/// written to disk, which is a different job from making bytes already on disk
/// safe to paint.
///
/// Pure, which is what makes it testable at all.
pub fn sanitize_render_line(raw: &str) -> String {
    let cap = DRIVER_OUTPUT_LINE_CELLS;
    let mut out = String::new();
    let mut n = 0usize;
    let mut overflowed = false;

    // Pushing through one helper is what makes the tab expansion and the cap
    // interact correctly: a tab that would straddle the cap is cut at the cap
    // like any other run of characters.
    let push = |out: &mut String, n: &mut usize, ch: char| -> bool {
        if *n >= cap {
            return false;
        }
        out.push(ch);
        *n += 1;
        true
    };

    for ch in raw.chars() {
        if ch == ESC {
            continue;
        }
        let fitted = if ch == '\t' {
            (0..TAB_WIDTH).all(|_| push(&mut out, &mut n, ' '))
        } else if (ch as u32) < 0x20 || ch == '\u{7f}' {
            push(&mut out, &mut n, CONTROL_REPLACEMENT)
        } else {
            push(&mut out, &mut n, ch)
        };
        if !fitted {
            overflowed = true;
            break;
        }
    }

    if overflowed {
        // Give the marker a cell of its own so the result stays within the cap.
        out.pop();
        out.push(ELLIPSIS);
    }
    out
}

/// Split one journal record's text into buffered lines, sanitising each.
///
/// The split on `\n` happens **first**, before anything else: assistant output
/// is genuinely multi-line and is more readable split than collapsed onto one
/// truncated row. Returns the lines and whether the record was cut at
/// [`DRIVER_OUTPUT_RECORD_MAX_LINES`] — the caller surfaces the cut rather than
/// swallowing it.
pub fn sanitize_record_lines(raw: &str) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut truncated = false;
    for segment in raw.split('\n') {
        if lines.len() >= DRIVER_OUTPUT_RECORD_MAX_LINES {
            truncated = true;
            break;
        }
        lines.push(sanitize_render_line(segment));
    }
    (lines, truncated)
}

#[derive(Default)]
pub struct ProjectViewCache {
    pub backlog_items: Vec<BacklogItem>,
    pub backlog_selected: usize,
    pub backlog_expanded: bool,
    pub git_entries: Vec<GitLogEntry>,
    pub git_selected: usize,
    pub git_planning_only: bool,
    pub git_diff_stat: Option<GitDiffStat>,
    pub loading_backlog: bool,
    pub loading_git: bool,
    pub loading_diff: bool,
    pub pipeline_selected: usize,
    pub queue_selected: usize,
    pub sessions_selected: usize,
    pub archive_depth: crate::archive::ArchiveDepth,
    pub archive_milestones: Vec<String>,
    pub archive_selected: [usize; 4],
    pub archive_scroll_offset: u16,
    pub archive_loading: bool,
    pub archive_file_content: Option<String>,
    pub archive_file_name: Option<String>,
    pub defaults_config: Option<crate::state_reader::config_json::GsdConfig>,
    /// Parsed contents of ~/.gsd/defaults.json — layered under
    /// `defaults_config` for display and editable via the [d] toggle.
    pub defaults_user_config: Option<crate::state_reader::config_json::GsdConfig>,
    /// Which file the Defaults tab is currently editing.
    pub defaults_edit_target: DefaultsEditTarget,
    pub defaults_selected: usize,
    /// When `Some(entry_idx)`, a dropdown OR text input is open for that entry.
    /// The render code decides which UI to show based on the entry's kind.
    pub defaults_editing: Option<usize>,
    /// Cursor position inside the open dropdown (only used for Bool/Enum).
    pub defaults_dropdown_selected: usize,
    /// In-progress text the user is typing for a String-kind entry.
    pub defaults_text_buffer: String,
    // ── Docs browser tab state ────────────────────────────────────────
    pub browser_depth: crate::browser::BrowserDepth,
    /// `None` until the user first activates the Docs tab; then set to the
    /// initial active-phase dir (or `.planning/` root if milestone complete).
    pub browser_current_dir: Option<PathBuf>,
    /// Root directory of the browse session (the project's `.planning/`).
    /// Used as the boundary for "navigate up" — Backspace at this dir is a no-op.
    pub browser_root: Option<PathBuf>,
    /// Entry directory the browser opened on (active phase or root). `g` jumps
    /// to `browser_root`; `p` jumps back here.
    pub browser_entry_dir: Option<PathBuf>,
    pub browser_entries: Vec<crate::browser::BrowserEntry>,
    pub browser_selected: usize,
    pub browser_scroll_offset: u16,
    pub browser_file_content: Option<String>,
    pub browser_file_name: Option<String>,
    // ── Driver tab view state (D-18) ──────────────────────────────────
    //
    // View state lives here rather than beside the ring buffer on
    // `AppContext` because `ProjectViewCache` is `#[derive(Default)]`: these
    // five fields are additive with zero constructor churn, while a field on
    // `AppContext` costs an edit at all four construction sites.
    /// Index into the run list of the run whose detail is shown.
    pub driver_selected_run: usize,
    /// Scroll offset of the live-output pane, in lines.
    ///
    /// Clamped in the key handler against the viewport metrics the render pass
    /// records — PageDown adds *then* clamps, PageUp/Up clamps *first* then
    /// subtracts (UIFIX-04).
    pub driver_scroll_offset: u16,
    /// Whether the output pane is following the tail.
    ///
    /// The one behaviour a `Paragraph::scroll` viewer does not give for free
    /// (D-19). `false` by default, which is correct: an unselected Driver tab
    /// has nothing to follow, and the tab sets it when a run is selected.
    pub driver_follow: bool,
    /// The inbox messages queued for the selected run, newest last.
    ///
    /// The TUI's state for a message is a pure function of the ids in
    /// `inbox.jsonl` and the ids present in each journal kind, which is what
    /// makes STEER-03 hold across a restart with no extra persisted state.
    pub driver_inbox: Vec<crate::journal::inbox::InboxMessage>,
    /// This project's runs on disk, **newest first** (OBS-05).
    ///
    /// Populated by `App::schedule_run_list_scan` from
    /// [`crate::journal::list_runs`], which reads each run's small committed
    /// `run.json` and never the journal beside it — so a project holding the
    /// full retention history costs the same to list as one with a single run.
    /// The order is [`crate::journal::sort_run_summaries_newest_first`]'s:
    /// lexicographic-descending on the run id, which is chronological because
    /// `new_run_id`'s format makes byte order time order.
    ///
    /// Plain data, and it lives here rather than in a sibling map on
    /// `AppContext` for the reason the four fields above do: this struct is
    /// `#[derive(Default)]`, so the field is additive with zero constructor
    /// churn, while an `AppContext` field costs an edit at five construction
    /// sites.
    pub driver_runs: Vec<crate::journal::RunSummary>,
}

pub struct AppContext {
    pub config: Config,
    pub config_path: PathBuf,
    pub project_states: HashMap<String, ProjectState>,
    pub table_state: TableState,
    pub filtered_aliases: Vec<String>,
    pub filter_text: String,
    pub change_tracker: ChangeTracker,
    pub detail_sub_view_per_project: HashMap<String, DetailSubView>,
    pub view_cache: HashMap<String, ProjectViewCache>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub error_message: Option<String>,
    pub event_tx: Option<UnboundedSender<Action>>,
    /// Long-lived sender for the **separate, bounded** executor channel (D-17).
    ///
    /// Two things about this field are load-bearing, and neither is stylistic:
    ///
    /// * It is **separate** from `event_tx`. High-volume stream traffic must
    ///   never travel through the `Action` FIFO, because `pump`'s biased
    ///   `select!` polls that FIFO first and would starve everything else.
    /// * It is a clone held for the **process lifetime**. The executor channel
    ///   legitimately has no producer between runs, and a channel with no live
    ///   sender closes — which permanently disables its `select!` arm
    ///   (Pitfall C). Keeping this clone alive is what stops that.
    pub exec_tx: Option<Sender<ExecEvent>>,
    /// Per-alias live driver state (D-19).
    ///
    /// A **sibling map**, shaped exactly like `last_refresh` and
    /// `archive_cache` above. Driver state deliberately does NOT live on
    /// `ProjectState`: that type derives `PartialEq`, and `app.rs` uses the
    /// derived equality to suppress the "Updated: {alias}" status message.
    /// Driver state changes every few seconds — one 68-second spike turn
    /// emitted 22 `thinking_tokens` events — so putting it there would flood
    /// the status bar for an entire multi-hour run.
    pub run_states: HashMap<String, RunState>,
    /// Count of full `parse_project_state` dispatches this process has issued.
    ///
    /// * **Load-bearing for OBS-06.** A driver journal append must never
    ///   increment it. `App::schedule_reparse` is the only writer, which is
    ///   what makes the assertion "zero re-parses" mean anything.
    /// * It is deliberately **not** `#[cfg(test)]`-gated, for two reasons: a
    ///   gated counter means the test exercises a different binary than
    ///   production, and the count is a legitimate diagnostic in its own right
    ///   that Phase 18's driver surface may want to render.
    pub reparse_dispatches: u64,
    /// Byte offset **and last observed `seq`** into each run journal, keyed by
    /// `(alias, run_id)` (D-13, D-28).
    ///
    /// * A **sibling map**, shaped exactly like `run_states` above and
    ///   `last_refresh` / `archive_cache` below. Phase 16 extends that
    ///   neighbourhood rather than recreating it (D-19).
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. A journal offset moves every few seconds, so a field
    ///   there would flood the status bar for an entire multi-hour run (D-18).
    /// * It holds offsets, sequence numbers, run ids and counts — **never a file
    ///   handle or a join handle**. `Action` derives `Clone` and a handle is not
    ///   `Clone` (D-20).
    /// * The value carries `last_seq` beside the byte offset because a byte
    ///   offset alone cannot detect a lost event: the gap check compared records
    ///   only *within one batch*, so a discontinuity falling between two tail
    ///   reads was invisible — the shape a multi-hour run produces. The stored
    ///   seq seeds the next batch's check through
    ///   [`seq_gaps_from`](crate::journal::reader::seq_gaps_from) (D-28).
    /// * **The map is pruned**, on the same 20-tick block as the reconciliation
    ///   probe: entries for unregistered aliases are dropped and at most
    ///   [`RETAIN_RUNS`](crate::journal::RETAIN_RUNS) run ids per alias are kept.
    ///   Before that it was inserted in exactly one place, removed in none, and
    ///   grew for the process lifetime. See `App::prune_driver_maps` (D-27).
    pub journal_cursors: HashMap<(String, String), crate::journal::reader::JournalCursor>,
    /// Per-alias observed driver run, from the reconciliation scan (D-13, D-25).
    ///
    /// * A **sibling map**, shaped exactly like `run_states` and
    ///   `journal_cursors` above. Phase 17 extends that neighbourhood rather
    ///   than recreating it.
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. A driver's observed state moves every few seconds, so a
    ///   field there would flood the status bar for an entire multi-hour run —
    ///   defeating a deliberate v1.4 feature for the whole duration of the thing
    ///   it is meant to report (D-25, ARCHITECTURE AP1).
    /// * It holds ids, counts and strings — **never a file handle or a join
    ///   handle**. `Action` derives `Clone` and a handle is not `Clone` (D-20).
    /// * An entry is **"observed", not "reattached" and not "streaming"** (D-11).
    ///   Once the TUI has exited, the driver's stdout pipe is gone and live
    ///   re-streaming is physically impossible; what this map carries is a
    ///   journal-tail handle and a pgid to signal.
    /// * **Phase 17 closed having added no field to `ProjectState`** — the whole
    ///   phase's driver state is this map, `run_states`, `journal_cursors` and
    ///   `session_spawned_runs`, all siblings here. `git diff` over
    ///   `src/state_reader/` across the phase is the checkable form of that
    ///   claim, and plan 17-07 records it (D-25).
    pub observed_runs: HashMap<String, crate::driver::reconcile::ObservedRun>,
    /// The run ids **this TUI session spawned**, which decides D-07's reaping
    /// arm at stop time.
    ///
    /// * It is a **separate set rather than a flag on `ObservedRun`**, and that
    ///   is the whole reason it exists. `observed_runs` is *replaced wholesale*
    ///   by every reconciliation scan, because the scan is authoritative and a
    ///   merge would keep entries the disk no longer justifies — so any flag
    ///   stored there survives at most five seconds and then silently reverts to
    ///   whatever the disk implies. The disk cannot imply this: `run.json`
    ///   records the driver's own pid, never who its parent was.
    /// * A run id in this set means the driver is **our child**, so the reaping
    ///   task `spawn_detached` created owns its `wait()`. A run id absent from it
    ///   was adopted after a restart, was reparented to init, and can only be
    ///   confirmed dead by re-probing `/proc` (D-07).
    /// * It holds run ids and nothing else — no handles — so `Action` stays
    ///   `Clone` for the same reason the sibling maps above do.
    /// * It is never pruned, and it does not need to be: it grows by one entry
    ///   per run this session starts, and `driver_max_concurrent` defaults to
    ///   one. A session that starts a thousand runs has a thousand short strings.
    pub session_spawned_runs: std::collections::HashSet<String>,
    /// Per-alias bounded live driver output (D-18).
    ///
    /// * A **sibling map**, shaped exactly like `run_states`, `journal_cursors`
    ///   and `observed_runs` above. Phase 18 extends that neighbourhood rather
    ///   than recreating it.
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. Live output arrives every few seconds for the whole
    ///   length of a run, so a field there would flood the status bar for an
    ///   entire multi-hour run — defeating a deliberate v1.4 feature for the
    ///   whole duration of the thing it is meant to report (ARCHITECTURE AP1).
    /// * It holds owned strings, an enum and two counters — **never a file
    ///   handle and never a join handle**. `Action` derives `Clone` and a handle
    ///   is not `Clone` (D-20).
    /// * **The map is pruned**, on the same 20-tick block as the reconciliation
    ///   probe, in `App::prune_driver_maps`. A new per-alias map that is not
    ///   pruned reintroduces the Phase 16 leak under a new name; that is this
    ///   phase's only remaining carry-forward obligation and it is a negative
    ///   one.
    pub driver_output: HashMap<String, DriverOutput>,
    /// How the dashboard orders its rows (D-25, OBS-07).
    ///
    /// [`SortMode::Alphabetical`] is the default and that is load-bearing — see
    /// the type's own doc.
    pub sort_mode: SortMode,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    pub detail_scroll_offset: u16,
    pub suggestion_index: usize,
    pub input_buffer: String,
    pub needs_redraw: bool,
    pub active_sessions: Vec<crate::session_detector::ClaudeSession>,
    pub archive_cache: HashMap<String, crate::archive::MilestoneArchive>,
}

/// How the dashboard orders its rows (D-25, OBS-07).
///
/// **Alphabetical is the default and keeping it so is load-bearing.** A
/// dashboard whose row order changes under the cursor while a run progresses is
/// a usability regression a demo will not catch: the user looks away, a run
/// finishes, the ranking shifts, and the next keystroke acts on a different
/// project than the one that was under the cursor. The attention-first mode is
/// opt-in, is announced in the summary row while it is active, and pins the
/// selection to the **alias** rather than the index for exactly that reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Case-insensitive alphabetical. Unchanged from v1.0.
    #[default]
    Alphabetical,
    /// Stable sort by [`attention_rank`], alphabetical within each rank.
    AttentionFirst,
}

/// Whether a project is waiting on a human (D-14, OBS-02, OBS-07).
///
/// **A predicate over evidence that exists today, not a fabricated state.** The
/// four sources, each of which something already writes:
///
/// * `state.paused` — a non-empty HANDOFF, the existing v1.4 signal that is
///   already badged elsewhere.
/// * `state.external_job_waiting` — an external job the project is blocked on.
/// * The last finished run's outcome being [`RunOutcome::PermissionDenied`],
///   [`RunOutcome::Failed`], [`RunOutcome::Stalled`] or [`RunOutcome::TimedOut`]
///   — the four the four-source derivation produces that a human must answer.
///   [`RunOutcome::Killed`] is deliberately absent: a run the user stopped is
///   not a run waiting on them.
/// * `parked`, fed forward-compatibly from a `JournalEvent::Parked` record if
///   one is ever present.
///
/// **Do not wire this to `JournalEvent::Parked` alone. Nothing emits that until
/// Phase 20.** The badge would never light, no test would catch it because the
/// test would emit the event by hand, and a badge that can never light is worse
/// than no badge because it teaches the user to ignore it (D-14). The `parked`
/// parameter is the one arm whose producer does not exist yet, which is exactly
/// why it is one arm of four rather than the whole predicate.
///
/// A **live** run suppresses only the stale-outcome arm: something is actively
/// driving, so an outcome from an earlier run is not a summons. The paused,
/// external-job and parked arms still fire, because those are facts about the
/// project rather than about a finished run.
///
/// Pure, and that is what makes it testable at all — every arm above has a unit
/// test, including the ones a running system reaches rarely.
pub fn needs_human(
    state: &ProjectState,
    run: Option<&crate::driver::reconcile::ObservedRun>,
    last_outcome: Option<&crate::executor::RunOutcome>,
    parked: bool,
) -> bool {
    use crate::executor::RunOutcome;

    if parked || state.paused || state.external_job_waiting {
        return true;
    }
    if run.is_some_and(|r| r.is_live()) {
        return false;
    }
    matches!(
        last_outcome,
        Some(
            RunOutcome::PermissionDenied { .. }
                | RunOutcome::Failed { .. }
                | RunOutcome::Stalled { .. }
                | RunOutcome::TimedOut { .. }
        )
    )
}

/// Where a project sorts under [`SortMode::AttentionFirst`]: lower is sooner.
///
/// Three ranks and no more, because the sort is stable and alphabetical order is
/// preserved within each: rank 0 is a project waiting on a human, rank 1 is one
/// being driven right now, and rank 2 is everything else. Driven-and-live ranks
/// **below** needs-a-human deliberately — a live run needs nothing from the
/// user, while a parked one does.
///
/// Pure, so the ordering is testable without a dashboard.
pub fn attention_rank(needs_human: bool, driven_and_live: bool) -> u8 {
    if needs_human {
        0
    } else if driven_and_live {
        1
    } else {
        2
    }
}

impl AppContext {
    /// Get sorted project aliases for consistent ordering in the table.
    ///
    /// Alphabetical order is computed first in **both** modes, which is what
    /// makes [`SortMode::AttentionFirst`] stable: `sort_by_key` is a stable
    /// sort, so projects of equal rank keep the case-insensitive alphabetical
    /// order established here.
    ///
    /// `sort_by_key` rather than `sort_by` is not stylistic — commit 1984a6c
    /// changed it for clippy 1.97's `unnecessary_sort_by`, and the lint returns
    /// if that is undone.
    pub fn sorted_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self.config.projects.keys().cloned().collect();
        aliases.sort_by_key(|a| a.to_lowercase());
        if self.sort_mode == SortMode::AttentionFirst {
            aliases.sort_by_key(|a| self.attention_rank_for(a));
        }
        aliases
    }

    /// Whether `alias` is waiting on a human, from the state this context holds.
    ///
    /// The `parked` argument is `false` because nothing emits
    /// `JournalEvent::Parked` before Phase 20 (D-14), and `last_outcome` is
    /// `None` because the typed [`crate::executor::RunOutcome`] of a *finished*
    /// run is not in this map — `observed_runs` holds runs that have not ended.
    /// **That arm's producer exists on disk today** (`RunRecord.outcome`); only
    /// the reader is a later plan's, and the caller that has a run summary in
    /// hand passes it to [`needs_human`] directly.
    pub fn needs_human_for(&self, alias: &str) -> bool {
        self.project_states
            .get(alias)
            .is_some_and(|state| needs_human(state, self.observed_runs.get(alias), None, false))
    }

    /// This alias's [`attention_rank`], from the state this context holds.
    fn attention_rank_for(&self, alias: &str) -> u8 {
        let driven_and_live = self
            .observed_runs
            .get(alias)
            .is_some_and(|run| run.is_live());
        attention_rank(self.needs_human_for(alias), driven_and_live)
    }

    /// Get the alias of the currently selected project, if any.
    pub fn selected_alias(&self) -> Option<String> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered_aliases.get(i).cloned())
    }

    /// Re-derive [`table_state`](AppContext::table_state) from the alias that
    /// was selected before the row set changed.
    ///
    /// **Pinned to the alias, never to the index.** Under
    /// [`SortMode::AttentionFirst`] a run finishing re-ranks its project, and an
    /// index-preserving selection would move the row out from under the cursor
    /// — so the next keystroke would act on a project the user never chose.
    /// When the previously selected alias is gone the index is clamped into
    /// range rather than left dangling, and an empty list selects nothing.
    fn pin_selection_to_alias(&mut self, previous: Option<String>) {
        if self.filtered_aliases.is_empty() {
            self.table_state.select(None);
            return;
        }
        let last = self.filtered_aliases.len() - 1;
        let index = previous
            .and_then(|alias| self.filtered_aliases.iter().position(|a| *a == alias))
            .unwrap_or_else(|| self.table_state.selected().unwrap_or(0).min(last));
        self.table_state.select(Some(index));
    }

    pub fn recompute_filtered_aliases(&mut self) {
        use crate::app::{format_phase_display, parse_filter, FilterColumn};

        let previously_selected = self.selected_alias();
        let all = self.sorted_aliases();
        if self.filter_text.is_empty() {
            self.filtered_aliases = all;
            self.pin_selection_to_alias(previously_selected);
            return;
        }
        let (term, column) = parse_filter(&self.filter_text);
        let term_lower = term.to_lowercase();
        self.filtered_aliases = all
            .into_iter()
            .filter(|alias| {
                let state = self.project_states.get(alias);
                match column {
                    FilterColumn::Name => alias.to_lowercase().contains(&term_lower),
                    FilterColumn::Phase => state.is_some_and(|s| {
                        format_phase_display(s).to_lowercase().contains(&term_lower)
                    }),
                    FilterColumn::Status => {
                        state.is_some_and(|s| s.status.to_lowercase().contains(&term_lower))
                    }
                    FilterColumn::All => {
                        alias.to_lowercase().contains(&term_lower)
                            || state.is_some_and(|s| {
                                s.status.to_lowercase().contains(&term_lower)
                                    || format_phase_display(s).to_lowercase().contains(&term_lower)
                            })
                    }
                    // `term` still applies across every column, so `/term/h` is
                    // "matches term AND needs a human". The all-rows form needs
                    // no special case: an empty term makes `contains` true for
                    // every alias, so `needs_human` alone decides (OBS-07).
                    FilterColumn::NeedsHuman => {
                        let matches_term = alias.to_lowercase().contains(&term_lower)
                            || state.is_some_and(|s| {
                                s.status.to_lowercase().contains(&term_lower)
                                    || format_phase_display(s).to_lowercase().contains(&term_lower)
                            });
                        matches_term && self.needs_human_for(alias)
                    }
                }
            })
            .collect();
        self.pin_selection_to_alias(previously_selected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::liveness::Liveness;
    use crate::driver::reconcile::ObservedRun;
    use crate::executor::RunOutcome;
    use crate::state_reader::ProjectState;

    /// An `AppContext` with `aliases` registered and nothing else set.
    ///
    /// The fifth full-field `AppContext` construction in the tree, and it lives
    /// here on purpose: the sort and filter behaviour under test is this
    /// module's, so its fixture belongs beside it rather than being borrowed
    /// from a screen's test module.
    fn ctx_with_aliases(aliases: &[&str]) -> AppContext {
        use crate::config::{Config, RegisteredProject};
        use ratatui::widgets::TableState;

        let mut config = Config::new();
        for alias in aliases {
            config.projects.insert(
                (*alias).to_string(),
                RegisteredProject {
                    path: PathBuf::from("/nonexistent").join(alias),
                    added: "2026-07-29".to_string(),
                    driver_opt_in: None,
                    extra: Default::default(),
                },
            );
        }

        let mut ctx = AppContext {
            config,
            config_path: PathBuf::from("/nonexistent/config.json"),
            project_states: aliases
                .iter()
                .map(|a| ((*a).to_string(), ProjectState::default()))
                .collect(),
            table_state: TableState::default(),
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
            session_spawned_runs: std::collections::HashSet::new(),
            driver_output: HashMap::new(),
            sort_mode: SortMode::default(),
            watcher: None,
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            suggestion_index: 0,
            input_buffer: String::new(),
            needs_redraw: false,
            active_sessions: Vec::new(),
            archive_cache: HashMap::new(),
        };
        ctx.recompute_filtered_aliases();
        ctx
    }

    /// An `ObservedRun` with only the field these assertions read varied.
    fn observed(alias: &str, liveness: Liveness) -> ObservedRun {
        ObservedRun {
            alias: alias.to_string(),
            run_id: format!("2026-07-29T12-00-00Z-{alias}"),
            pid: 4242,
            pgid: 4242,
            started_at: "2026-07-29T12:00:00Z".to_string(),
            goal: "ship it".to_string(),
            gsd_command: "/gsd-execute-phase".to_string(),
            liveness,
        }
    }

    /// Mark `alias` as paused, which is one of the four needs-human sources.
    fn pause(ctx: &mut AppContext, alias: &str) {
        ctx.project_states
            .get_mut(alias)
            .expect("the fixture registered this alias")
            .paused = true;
    }

    #[test]
    fn pushing_past_the_ring_cap_keeps_exactly_the_cap_and_counts_every_drop() {
        let mut buf = DriverOutput::default();
        let overshoot = 50;
        for i in 0..(DRIVER_OUTPUT_RING_LINES + overshoot) {
            buf.push_record(DriverLineKind::Output, &format!("line {i}"));
        }

        assert_eq!(
            buf.len(),
            DRIVER_OUTPUT_RING_LINES,
            "the ring must hold exactly its cap — a buffer that keeps growing is \
             the 'a Vec that grows' failure this cap exists to prevent"
        );
        assert_eq!(
            buf.dropped(),
            overshoot as u64,
            "every dropped line must be counted, because the count is the whole \
             mechanism the render layer has for admitting the loss"
        );
        let first = buf
            .lines()
            .next()
            .expect("the ring is at its cap, so it has a first line");
        assert_eq!(
            first.text, "line 50",
            "the oldest lines are the ones dropped, so the survivor at the front \
             is the first line past the overshoot"
        );
    }

    #[test]
    fn a_record_with_more_newlines_than_the_record_cap_is_cut_and_reports_the_cut() {
        let raw = (0..200)
            .map(|i| format!("row {i}"))
            .collect::<Vec<_>>()
            .join("\n");

        let (lines, truncated) = sanitize_record_lines(&raw);
        assert_eq!(
            lines.len(),
            DRIVER_OUTPUT_RECORD_MAX_LINES,
            "one record must not expand past the record cap, or a single giant \
             record flushes the whole ring in one append"
        );
        assert!(
            truncated,
            "the cut must be reported, not swallowed — the render layer shows a \
             trailing notice and cannot infer the cut from the line count alone"
        );

        let mut buf = DriverOutput::default();
        buf.push_record(DriverLineKind::Output, &raw);
        assert!(
            buf.record_truncated(),
            "the buffer must carry the truncation flag forward from the record"
        );
        assert_eq!(buf.len(), DRIVER_OUTPUT_RECORD_MAX_LINES);
    }

    #[test]
    fn a_line_of_multibyte_characters_longer_than_the_cap_truncates_without_panicking() {
        // U+1F680 ROCKET is four bytes per char, so a byte-based truncation at
        // the cap would land inside a scalar and panic.
        let raw: String = std::iter::repeat_n('\u{1F680}', DRIVER_OUTPUT_LINE_CELLS + 100).collect();

        let out = sanitize_render_line(&raw);
        assert_eq!(
            out.chars().count(),
            DRIVER_OUTPUT_LINE_CELLS,
            "truncation counts chars, never bytes"
        );
        assert!(
            out.ends_with(ELLIPSIS),
            "a truncated line must say so, or the reader cannot tell it was cut"
        );
        assert!(
            out.chars().all(|c| c == '\u{1F680}' || c == ELLIPSIS),
            "no scalar may be split: every surviving char is a whole rocket"
        );
    }

    #[test]
    fn an_escape_bearing_string_never_reaches_the_buffer_with_its_escape() {
        // A colour SGR, a cursor move, and an OSC window-title set — the three
        // shapes that repaint, reposition, and retitle.
        let hostile = "\u{1b}[31mred\u{1b}[0m \u{1b}[2J \u{1b}]0;pwned\u{7}";

        let out = sanitize_render_line(hostile);
        assert!(
            !out.contains(ESC),
            "ESC must be stripped unconditionally — with it, agent prose can \
             repaint the screen, forge a status line, move the cursor, or set \
             the window title. Got: {out:?}"
        );

        let mut buf = DriverOutput::default();
        buf.push_record(DriverLineKind::Output, hostile);
        assert!(
            buf.lines().all(|l| !l.text.contains(ESC)),
            "no ESC may enter the buffer through the append path either"
        );
        assert!(
            out.contains("red"),
            "the prose itself is still shown — only the introducer is removed"
        );
    }

    #[test]
    fn every_c0_control_and_del_becomes_the_replacement_glyph() {
        // Every C0 except TAB (expanded) and ESC (stripped), plus DEL.
        let raw: String = (0u32..0x20)
            .filter(|c| *c != 0x09 && *c != 0x1b)
            .chain(std::iter::once(0x7f))
            .map(|c| char::from_u32(c).expect("C0 and DEL are valid scalars"))
            .collect();

        let out = sanitize_render_line(&raw);
        assert!(
            out.chars().all(|c| c == CONTROL_REPLACEMENT),
            "every C0 control and DEL renders as the replacement glyph — present \
             and visible, never executable. Got: {out:?}"
        );
        assert_eq!(
            out.chars().count(),
            raw.chars().count(),
            "replacement is one-for-one, so nothing is silently dropped"
        );
    }

    #[test]
    fn a_tab_expands_to_four_spaces() {
        assert_eq!(
            sanitize_render_line("a\tb"),
            "a    b",
            "a tab is expanded rather than replaced, because indentation carries \
             meaning in the output this pane shows"
        );
    }

    #[test]
    fn splitting_on_newlines_happens_before_any_other_rule() {
        let (lines, truncated) = sanitize_record_lines("first\nsecond\nthird");
        assert_eq!(lines, vec!["first", "second", "third"]);
        assert!(!truncated);

        // And a newline reaching the single-line sanitiser directly — a header
        // interpolation, say — is a control character like any other.
        assert_eq!(
            sanitize_render_line("first\nsecond"),
            format!("first{CONTROL_REPLACEMENT}second"),
            "a newline that did not go through the record split must not be able \
             to forge a second row"
        );
    }

    // ── needs_human, the sort mode, and the filter ────────────────────

    #[test]
    fn needs_human_is_true_for_each_of_the_four_evidence_sources() {
        let quiet = ProjectState::default();
        assert!(
            !needs_human(&quiet, None, None, false),
            "a registered project with no evidence at all is not waiting on \
             anyone — a predicate that is always true is not a predicate"
        );

        let paused = ProjectState {
            paused: true,
            ..Default::default()
        };
        assert!(
            needs_human(&paused, None, None, false),
            "a non-empty HANDOFF is the existing v1.4 signal and must count"
        );

        let waiting = ProjectState {
            external_job_waiting: true,
            ..Default::default()
        };
        assert!(
            needs_human(&waiting, None, None, false),
            "an external job the phase is blocked on needs a human"
        );

        assert!(
            needs_human(&quiet, None, None, true),
            "the forward-compatible Parked arm must fire when something \
             eventually sets it (Phase 20 owns the producer)"
        );

        for outcome in [
            RunOutcome::PermissionDenied {
                denials: Vec::new(),
            },
            RunOutcome::Failed {
                reason: "boom".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: Some(1),
            },
            RunOutcome::Stalled {
                idle_for: std::time::Duration::from_secs(900),
            },
            RunOutcome::TimedOut {
                after: std::time::Duration::from_secs(14_400),
            },
        ] {
            assert!(
                needs_human(&quiet, None, Some(&outcome), false),
                "a finished run that ended {outcome:?} is a question only a human \
                 can answer"
            );
        }

        assert!(
            !needs_human(
                &quiet,
                None,
                Some(&RunOutcome::Killed { turns: Vec::new() }),
                false
            ),
            "a run the user themselves stopped is not a run waiting on them"
        );
    }

    #[test]
    fn a_live_run_suppresses_the_stale_outcome_arm_but_not_the_others() {
        let quiet = ProjectState::default();
        let live = observed("proj", Liveness::Alive);
        let stale = RunOutcome::Failed {
            reason: "an earlier run".to_string(),
            subtype: None,
            terminal_reason: None,
            exit_code: Some(1),
        };

        assert!(
            !needs_human(&quiet, Some(&live), Some(&stale), false),
            "something is actively driving, so an earlier run's failure is not a \
             summons"
        );

        let paused = ProjectState {
            paused: true,
            ..Default::default()
        };
        assert!(
            needs_human(&paused, Some(&live), Some(&stale), false),
            "a HANDOFF is a fact about the project, not about a finished run, so \
             a live run must not hide it"
        );
    }

    #[test]
    fn alphabetical_is_the_default_sort_mode() {
        let ctx = ctx_with_aliases(&["Zebra", "apple", "Mango"]);
        assert_eq!(
            ctx.sort_mode,
            SortMode::Alphabetical,
            "alphabetical must stay the default: a dashboard whose row order \
             changes under the cursor while a run progresses is a usability \
             regression a demo will not catch (D-25)"
        );
        assert_eq!(ctx.sorted_aliases(), vec!["apple", "Mango", "Zebra"]);
    }

    #[test]
    fn attention_first_is_stable_across_projects_of_equal_rank() {
        let mut ctx = ctx_with_aliases(&["alpha", "bravo", "charlie", "delta"]);
        pause(&mut ctx, "bravo");
        pause(&mut ctx, "delta");
        ctx.observed_runs
            .insert("charlie".to_string(), observed("charlie", Liveness::Alive));
        ctx.sort_mode = SortMode::AttentionFirst;

        assert_eq!(
            ctx.sorted_aliases(),
            vec!["bravo", "delta", "charlie", "alpha"],
            "rank 0 (needs a human) then rank 1 (driven and live) then the rest, \
             with the alphabetical order preserved *within* each rank — an \
             unstable sort would let equal-rank rows shuffle on every scan"
        );

        assert_eq!(
            attention_rank(true, true),
            0,
            "needs-a-human outranks driven-and-live: a live run needs nothing \
             from the user, a parked one does"
        );
        assert_eq!(attention_rank(false, true), 1);
        assert_eq!(attention_rank(false, false), 2);
    }

    #[test]
    fn the_needs_human_filter_parses_with_and_without_a_term() {
        use crate::app::{parse_filter, FilterColumn};

        assert_eq!(
            parse_filter("api/h"),
            ("api".to_string(), FilterColumn::NeedsHuman)
        );
        // The all-rows form. The search prompt supplies its own leading `/`, so
        // this renders as `//h` on screen. No special case in the grammar: an
        // empty term makes the column match true for every row.
        assert_eq!(
            parse_filter("/h"),
            (String::new(), FilterColumn::NeedsHuman)
        );
        assert_eq!(
            parse_filter("api"),
            ("api".to_string(), FilterColumn::All),
            "the existing grammar is untouched"
        );

        let mut ctx = ctx_with_aliases(&["api", "web", "apidocs"]);
        pause(&mut ctx, "api");
        pause(&mut ctx, "web");

        ctx.filter_text = "/h".to_string();
        ctx.recompute_filtered_aliases();
        assert_eq!(
            ctx.filtered_aliases,
            vec!["api", "web"],
            "an empty term must match every alias and leave needs_human to decide"
        );

        ctx.filter_text = "api/h".to_string();
        ctx.recompute_filtered_aliases();
        assert_eq!(
            ctx.filtered_aliases,
            vec!["api"],
            "the term still narrows across columns; `apidocs` matches the term \
             but does not need a human"
        );
    }

    #[test]
    fn a_needs_human_filter_matching_nothing_yields_an_empty_list_and_no_selection() {
        let mut ctx = ctx_with_aliases(&["api", "web"]);
        ctx.table_state.select(Some(1));

        ctx.filter_text = "/h".to_string();
        ctx.recompute_filtered_aliases();

        assert!(
            ctx.filtered_aliases.is_empty(),
            "no project needs a human, so the filter yields nothing rather than \
             falling back to everything"
        );
        assert_eq!(
            ctx.table_state.selected(),
            None,
            "a selection pointing past an empty list is how a render panics; the \
             clamp must reach None"
        );
        assert_eq!(ctx.selected_alias(), None);
    }

    #[test]
    fn a_project_that_is_both_driven_and_needs_a_human_appears_exactly_once() {
        let mut ctx = ctx_with_aliases(&["alpha", "busy"]);
        pause(&mut ctx, "busy");
        ctx.observed_runs
            .insert("busy".to_string(), observed("busy", Liveness::Alive));

        ctx.sort_mode = SortMode::AttentionFirst;
        let sorted = ctx.sorted_aliases();
        assert_eq!(
            sorted.iter().filter(|a| *a == "busy").count(),
            1,
            "satisfying two predicates must not duplicate a row"
        );
        assert_eq!(sorted, vec!["busy", "alpha"]);

        ctx.filter_text = "/h".to_string();
        ctx.recompute_filtered_aliases();
        assert_eq!(
            ctx.filtered_aliases,
            vec!["busy"],
            "and the filter yields it once, not twice"
        );
    }

    #[test]
    fn the_selection_is_pinned_to_the_alias_when_a_rerank_moves_its_row() {
        let mut ctx = ctx_with_aliases(&["alpha", "zulu"]);
        ctx.sort_mode = SortMode::AttentionFirst;
        ctx.recompute_filtered_aliases();
        ctx.table_state.select(Some(1));
        assert_eq!(ctx.selected_alias().as_deref(), Some("zulu"));

        // A run parks; `zulu` jumps to rank 0 and its row index changes.
        pause(&mut ctx, "zulu");
        ctx.recompute_filtered_aliases();

        assert_eq!(ctx.filtered_aliases, vec!["zulu", "alpha"]);
        assert_eq!(
            ctx.table_state.selected(),
            Some(0),
            "the selection follows the alias, not the index — otherwise a run \
             finishing moves the row out from under the cursor and the next \
             keystroke acts on a project the user never chose"
        );
        assert_eq!(ctx.selected_alias().as_deref(), Some("zulu"));
    }

    #[test]
    fn a_selection_whose_alias_is_gone_is_clamped_rather_than_left_dangling() {
        let mut ctx = ctx_with_aliases(&["alpha", "bravo", "charlie"]);
        ctx.table_state.select(Some(2));
        assert_eq!(ctx.selected_alias().as_deref(), Some("charlie"));

        ctx.filter_text = "a".to_string();
        ctx.recompute_filtered_aliases();

        assert_eq!(ctx.filtered_aliases, vec!["alpha", "bravo", "charlie"]);
        // Now narrow to a set that no longer holds the selected alias.
        ctx.filter_text = "bravo".to_string();
        ctx.recompute_filtered_aliases();
        assert_eq!(ctx.filtered_aliases, vec!["bravo"]);
        assert_eq!(
            ctx.table_state.selected(),
            Some(0),
            "an index past the end of the new list must be clamped into range"
        );
    }
}
