pub mod add_project;
pub mod create_project;
pub mod delete_confirm;
pub mod detail;
pub mod driver_confirm;
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
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    pub detail_scroll_offset: u16,
    pub suggestion_index: usize,
    pub input_buffer: String,
    pub needs_redraw: bool,
    pub active_sessions: Vec<crate::session_detector::ClaudeSession>,
    pub archive_cache: HashMap<String, crate::archive::MilestoneArchive>,
}

impl AppContext {
    /// Get sorted project aliases for consistent ordering in the table.
    pub fn sorted_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self.config.projects.keys().cloned().collect();
        aliases.sort_by_key(|a| a.to_lowercase());
        aliases
    }

    /// Get the alias of the currently selected project, if any.
    pub fn selected_alias(&self) -> Option<String> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered_aliases.get(i).cloned())
    }

    pub fn recompute_filtered_aliases(&mut self) {
        use crate::app::{format_phase_display, parse_filter, FilterColumn};

        let all = self.sorted_aliases();
        if self.filter_text.is_empty() {
            self.filtered_aliases = all;
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
                }
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
