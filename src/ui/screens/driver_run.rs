//! The launcher-first Driver tab (Phase 30, plan 30-01): the project's launcher
//! runs as the run list, and the selected run's status header.
//!
//! **Every word, glyph and colour here comes from [`DerivedStatus`]** and nothing
//! else (D-04, D-09, T-30-01). `DerivedStatus` is the frontmatter's written status
//! corrected by a probe of the run lock, so a run whose frontmatter says `running`
//! while its lock is free reads `crashed`, and a forged `done` is shown only as the
//! word the file wrote. No glyph, colour or sort key is chosen from `status_reason`,
//! from a connection's `state`, from a heartbeat's freshness or from any text the
//! driver wrote (the Replit rule, 18-CONTEXT D-13): the frontmatter is display data,
//! because a sibling run directory is writable from inside a sandbox (28-SECURITY).
//!
//! **No file I/O happens in this module** (D-25). The rows arrive in
//! `ProjectViewCache::launcher_runs`, filled off the render thread by
//! `AppContext::schedule_launcher_refresh`; every function below is a pure
//! function of that cache.
//!
//! Untrusted text is composed `shown_capped` FIRST (control characters and the
//! invisible class) and cut with [`fit_cells`] SECOND, to the room left after the
//! spans already placed. The cut is by display width rather than by `char` count
//! (orchestrator decision 2, `[inferred — audit]`), so a CJK or emoji run id can
//! never overflow its row.

use std::cell::Cell;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::detail::{fit_cells, ViewportMetrics};
use super::driver::{
    driver_panes, label_style, muted_style, render_dry_run_preview, render_run_detail,
    run_list_row, run_list_title, shown_capped, DriverTabRegions, DRIVER_PANE_TITLE,
    DRY_RUN_LOADING,
};
use super::{AppContext, ProjectViewCache};
use crate::driver::reconcile::RunVerdict;
use crate::journal::RunSummary;
use crate::state_reader::driver_frontmatter::{DerivedStatus, DriverRunEntry, RunStatus};

// ── Run-status glyphs (fixed `&'static str`, `\u{…}` per PATTERNS S7) ───────
//
// A fixed `&'static str` is the *mechanism* enforcing "no glyph is derived from
// file content": no byte read off disk can be assigned to one of these, so a
// forged frontmatter cannot reach a run row through its glyph.

/// `◆` U+25C6 — a run whose lock is held (`running`, `starting`).
pub const GLYPH_RUN_LIVE: &str = "\u{25C6}";
/// `✗` U+2717 — crashed, stalled. The word disambiguates the failure family.
pub const GLYPH_RUN_CRASHED: &str = "\u{2717}";
/// `◇` U+25C7 — liveness unknown, an unknown status word, an unreadable run.
pub const GLYPH_RUN_UNKNOWN: &str = "\u{25C7}";
/// `○` U+25CB — a minted run that has never been launched.
pub const GLYPH_RUN_UNLAUNCHED: &str = "\u{25CB}";
/// `⚑` U+2691 — parked: the run needs a human.
pub const GLYPH_RUN_PARKED: &str = "\u{2691}";
/// `‖` U+2016 — paused.
pub const GLYPH_RUN_PAUSED: &str = "\u{2016}";
/// `■` U+25A0 — stopped.
pub const GLYPH_RUN_STOPPED: &str = "\u{25A0}";
/// `●` U+25CF — done.
pub const GLYPH_RUN_DONE: &str = "\u{25CF}";
/// `▸` U+25B8 — the collapsed legacy-runs row.
pub const LEGACY_COLLAPSED: &str = "\u{25B8}";
/// `▾` U+25BE — the expanded legacy-runs row.
pub const LEGACY_EXPANDED: &str = "\u{25BE}";

// ── Measured layout constants (the `STATUS_COLUMN_MIN_CELLS` discipline) ────

/// Width of the list-word column. Derived: the longest list word is `unlaunched`
/// (10 cells).
pub const RUN_WORD_CELLS: usize = 10;

/// Inner list width at which the word column appears.
///
/// Derived: highlight 2 + glyph 1 + gap 1 + word 10 + gap 1 + posture 1 = 16 cells
/// of fixed columns, plus a 20-cell run-id floor = 36.
pub const RUN_ROW_WORD_MIN_CELLS: u16 = 36;

/// The width of the label column in the header rows: a 2-cell indent, the 7-cell
/// label (`reason`) and a 1-cell gap, so every value starts at column 10.
const HEADER_LABEL_CELLS: usize = 10;

/// Posture cell: the run's frontmatter says `sandbox: degraded`.
const POSTURE_DEGRADED: &str = "!";

// ── Status vocabulary (D-04) ────────────────────────────────────────────────

/// What one run's status looks like: glyph, list word, detail word and style.
///
/// `glyph` is always one of the fixed `GLYPH_RUN_*` constants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCell {
    pub glyph: &'static str,
    /// The word in the run-list row, at most [`RUN_WORD_CELLS`] cells.
    pub list_word: String,
    /// The word in the detail header.
    pub detail_word: String,
    pub style: Style,
}

fn plain_cell(glyph: &'static str, word: &str, color: Color) -> StatusCell {
    StatusCell {
        glyph,
        list_word: word.to_owned(),
        detail_word: word.to_owned(),
        style: Style::default().fg(color),
    }
}

/// `✗ crashed`, Red and BOLD: the one status that must not be skimmed past.
fn crashed_cell() -> StatusCell {
    StatusCell {
        style: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ..plain_cell(GLYPH_RUN_CRASHED, "crashed", Color::Red)
    }
}

/// The UI-SPEC run status table, derived **only** from [`DerivedStatus`].
///
/// The raw `status:` line is never shown as the status (D-04): `Crashed` and
/// `LivenessUnknown` carry the written word for the header's reason row, not for
/// this cell. An entry that could not be read is `unreadable`, Yellow.
pub fn status_cell(entry: &DriverRunEntry) -> StatusCell {
    let Ok(view) = &entry.1 else {
        return plain_cell(GLYPH_RUN_UNKNOWN, "unreadable", Color::Yellow);
    };
    match &view.derived {
        DerivedStatus::Crashed { .. } => crashed_cell(),
        DerivedStatus::LivenessUnknown { .. } => StatusCell {
            list_word: "liveness?".to_owned(),
            detail_word: "liveness unknown".to_owned(),
            ..plain_cell(GLYPH_RUN_UNKNOWN, "", Color::Yellow)
        },
        DerivedStatus::NotLaunched => StatusCell {
            list_word: "unlaunched".to_owned(),
            detail_word: "not launched".to_owned(),
            ..plain_cell(GLYPH_RUN_UNLAUNCHED, "", Color::DarkGray)
        },
        DerivedStatus::AsWritten(status) => match status {
            RunStatus::Running => plain_cell(GLYPH_RUN_LIVE, "running", Color::Magenta),
            RunStatus::Starting => plain_cell(GLYPH_RUN_LIVE, "starting", Color::Magenta),
            RunStatus::Parked => plain_cell(GLYPH_RUN_PARKED, "parked", Color::Red),
            RunStatus::Paused => plain_cell(GLYPH_RUN_PAUSED, "paused", Color::Yellow),
            RunStatus::Stopped => plain_cell(GLYPH_RUN_STOPPED, "stopped", Color::DarkGray),
            RunStatus::Done => plain_cell(GLYPH_RUN_DONE, "done", Color::Green),
            RunStatus::Stalled => plain_cell(GLYPH_RUN_CRASHED, "stalled", Color::Red),
            // Written by the launcher itself, not derived from a free lock.
            RunStatus::Crashed => crashed_cell(),
            RunStatus::Unknown(word) => {
                let shown = shown_capped(word);
                StatusCell {
                    list_word: fit_cells(&shown, RUN_WORD_CELLS),
                    detail_word: shown,
                    ..plain_cell(GLYPH_RUN_UNKNOWN, "", Color::Yellow)
                }
            }
        },
    }
}

// ── Run-list rows ───────────────────────────────────────────────────────────

fn selection_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn posture_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Whether the entry's frontmatter records the degraded (unsandboxed) posture.
fn is_degraded(entry: &DriverRunEntry) -> bool {
    entry
        .1
        .as_ref()
        .is_ok_and(|view| view.frontmatter.sandbox.as_deref() == Some("degraded"))
}

/// One launcher-run row:
/// `highlight(2) · glyph(1) · space · [word padded to 10 · space] · run id · posture(1)`.
///
/// The selected row's highlight and run id take the Selection style (Cyan, BOLD)
/// while the glyph and word keep the status colour, so the selection never
/// replaces the status triple. The word column appears only when `inner_width` is
/// at least [`RUN_ROW_WORD_MIN_CELLS`]. The run id is sanitised with
/// `shown_capped` and then cut with [`fit_cells`] to the room measured from the
/// spans already placed, so no run id can push the posture cell off the row. The
/// posture cell is `!` (Yellow, BOLD) when the frontmatter says `sandbox:
/// degraded`, else a space.
pub fn run_row(entry: &DriverRunEntry, selected: bool, inner_width: u16) -> Line<'static> {
    let cell = status_cell(entry);
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(6);
    let mut used = 0usize;

    let (prefix, id_style) = if selected {
        ("> ", selection_style())
    } else {
        ("  ", Style::default())
    };
    spans.push(Span::styled(prefix, selection_style()));
    used += 2;
    spans.push(Span::styled(cell.glyph, cell.style));
    spans.push(Span::raw(" "));
    used += 2;
    if inner_width >= RUN_ROW_WORD_MIN_CELLS {
        let word = fit_cells(&cell.list_word, RUN_WORD_CELLS);
        let pad = RUN_WORD_CELLS.saturating_sub(Span::raw(word.as_str()).width());
        spans.push(Span::styled(
            format!("{word}{}", " ".repeat(pad)),
            cell.style,
        ));
        spans.push(Span::raw(" "));
        used += RUN_WORD_CELLS + 1;
    }
    // One cell is always reserved for the posture column.
    let room = usize::from(inner_width).saturating_sub(used + 1);
    spans.push(Span::styled(
        fit_cells(&shown_capped(&entry.0), room),
        id_style,
    ));
    spans.push(if is_degraded(entry) {
        Span::styled(POSTURE_DEGRADED, posture_style())
    } else {
        Span::raw(" ")
    });
    Line::from(spans)
}

// ── The selected run's header (rows 1-2) ────────────────────────────────────

/// Header row 1: `glyph · detail word · run id`.
fn header_row_1(entry: &DriverRunEntry, width: usize) -> Line<'static> {
    let cell = status_cell(entry);
    let lead = format!("{} {}  ", cell.glyph, cell.detail_word);
    let room = width.saturating_sub(Span::raw(lead.as_str()).width());
    Line::from(vec![
        Span::styled(lead, cell.style),
        Span::raw(fit_cells(&shown_capped(&entry.0), room)),
    ])
}

/// Header row 2: the `reason` label and the UI-SPEC reason text.
///
/// A crashed run reads `run lock released without a final status (file says
/// {written})` in Red, a liveness-unknown run `lock probe failed (file says
/// {written})` in Yellow, and any other run its `status_reason` or `(none
/// recorded)` muted. The `{written}` word is the frontmatter's own status line,
/// shown here and nowhere else as the thing the lock contradicts.
fn header_row_2(
    view: &crate::state_reader::driver_frontmatter::DriverRunView,
    width: usize,
) -> Line<'static> {
    let room = width.saturating_sub(HEADER_LABEL_CELLS);
    let (text, style) = match &view.derived {
        DerivedStatus::Crashed { written } => (
            format!(
                "run lock released without a final status (file says {})",
                shown_capped(written.as_str())
            ),
            Style::default().fg(Color::Red),
        ),
        DerivedStatus::LivenessUnknown { written } => (
            format!(
                "lock probe failed (file says {})",
                shown_capped(written.as_str())
            ),
            Style::default().fg(Color::Yellow),
        ),
        _ => match view
            .frontmatter
            .status_reason
            .as_deref()
            .filter(|reason| crate::text::carries_visible_content(reason))
        {
            Some(reason) => (shown_capped(reason), Style::default()),
            None => ("(none recorded)".to_owned(), muted_style()),
        },
    };
    Line::from(vec![
        Span::styled(format!("  {:<7} ", "reason"), label_style()),
        Span::styled(fit_cells(&text, room), style),
    ])
}

/// The detail pane's lines for the selected run, top to bottom.
///
/// An unreadable entry gets the UI-SPEC unreadable copy: the error in Red and the
/// promise that the run is read again on the next change to its directory.
fn detail_lines(entry: &DriverRunEntry, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![header_row_1(entry, width)];
    match &entry.1 {
        Ok(view) => lines.push(header_row_2(view, width)),
        Err(error) => {
            let message = format!(
                "DIRECTION.md could not be read: {}",
                shown_capped(&error.to_string())
            );
            lines.push(Line::from(Span::styled(
                fit_cells(&message, width),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "It is read again on the next change to the run directory.",
                muted_style(),
            )));
        }
    }
    lines
}

/// What a steering, launch or kill key says when the highlight is on a legacy run.
pub const LEGACY_READ_ONLY: &str = "Legacy runs are read-only here.";

// ── Kill (D-22) ─────────────────────────────────────────────────────────────

/// Whether `x` may kill a run with this derived status: it is live, it crashed
/// (its scope may outlive the launcher), or the lock probe could not tell. A run
/// that is positively not running has nothing to kill.
pub fn kill_available(derived: &DerivedStatus) -> bool {
    matches!(
        derived,
        DerivedStatus::AsWritten(RunStatus::Running | RunStatus::Starting)
            | DerivedStatus::Crashed { .. }
            | DerivedStatus::LivenessUnknown { .. }
    )
}

/// Whether `x` may kill this row. An unreadable row has no derived status, so
/// nothing establishes that it is not running: it is treated like
/// `LivenessUnknown` rather than denied a kill key, because a runaway scope whose
/// `DIRECTION.md` was corrupted or grown past the read cap must still be
/// stoppable from here (`[inferred — audit]`; `stop_run` itself reports
/// "the run is not running" when the lock is free).
pub fn entry_killable(entry: &DriverRunEntry) -> bool {
    entry
        .1
        .as_ref()
        .map_or(true, |view| kill_available(&view.derived))
}

// ── Selection across launcher rows then legacy rows (D-01) ──────────────────

/// What the highlight is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// A launcher run: an index into `launcher_runs`.
    Launcher(usize),
    /// A legacy journal run: an index into `driver_runs`.
    Legacy(usize),
    /// Nothing: no launcher run, and no legacy row shown and selected.
    Nothing,
}

/// Whether the project has Phase 17/18 journal runs, so the collapsed section
/// exists at all.
fn has_legacy(cache: &ProjectViewCache) -> bool {
    !cache.driver_runs.is_empty()
}

/// Whether the legacy rows themselves are in the list.
fn legacy_rows_shown(cache: &ProjectViewCache) -> bool {
    has_legacy(cache) && cache.legacy_expanded
}

/// Where the highlight is. A legacy selection counts only while the legacy rows
/// are shown, so collapsing the section can never leave the detail pane on a row
/// the list no longer draws.
pub fn selection(cache: Option<&ProjectViewCache>) -> Selection {
    let Some(cache) = cache else {
        return Selection::Nothing;
    };
    if cache.legacy_row_selected && legacy_rows_shown(cache) {
        return Selection::Legacy(cache.driver_selected_run.min(cache.driver_runs.len() - 1));
    }
    match cache.launcher_runs.len() {
        0 => Selection::Nothing,
        n => Selection::Launcher(cache.launcher_selected.min(n - 1)),
    }
}

/// The highlight's place in the one linear order `j`/`k` walk: launcher rows
/// first, then (when shown) the legacy rows. The collapsed-section header row is
/// not a position.
pub fn selection_position(cache: Option<&ProjectViewCache>) -> usize {
    let launcher = cache.map_or(0, |c| c.launcher_runs.len());
    match selection(cache) {
        Selection::Launcher(index) => index,
        Selection::Legacy(index) => launcher + index,
        Selection::Nothing => 0,
    }
}

/// The linear position of list row `row` (as drawn, header included), or `None`
/// for the header row and rows past the end. The mouse's way into the same order.
pub fn position_for_row(cache: &ProjectViewCache, row: usize) -> Option<usize> {
    let launcher = cache.launcher_runs.len();
    if row < launcher {
        return Some(row);
    }
    if !legacy_rows_shown(cache) || row == launcher {
        return None;
    }
    let position = row - 1;
    (position - launcher < cache.driver_runs.len()).then_some(position)
}

/// Move the highlight `delta` places along the linear order, clamped at both ends,
/// and keep the pins coherent. Pure: it touches the cache and nothing else.
///
/// Returns whether the highlight now sits on a legacy row, in which case the
/// caller reschedules the run-list scan (the legacy detail's inbox and journal are
/// the *selected* run's) exactly as the shipped `j`/`k` did. Moving onto a
/// different legacy run resets the output pane to its tail with follow on (D-19).
pub fn move_selection(cache: &mut ProjectViewCache, delta: isize) -> bool {
    let launcher = cache.launcher_runs.len();
    let legacy = if legacy_rows_shown(cache) {
        cache.driver_runs.len()
    } else {
        0
    };
    let total = launcher + legacy;
    if total == 0 {
        return false;
    }
    let current = selection_position(Some(cache));
    let next = if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize)
    }
    .min(total - 1);

    if next < launcher {
        cache.legacy_row_selected = false;
        cache.launcher_selected = next;
        cache.launcher_selected_id = cache.launcher_runs.get(next).map(|(id, _)| id.clone());
        return false;
    }
    let index = next - launcher;
    let moved = !cache.legacy_row_selected || cache.driver_selected_run != index;
    cache.legacy_row_selected = true;
    cache.driver_selected_run = index;
    if moved {
        cache.driver_scroll_offset = 0;
        cache.driver_follow = true;
    }
    true
}

/// `L`: show or hide the legacy rows. Inert while there are no legacy runs.
/// Returns whether anything changed.
///
/// Expanding a section that is the only thing in the list puts the highlight on
/// its first row (there is no launcher row to keep it on); collapsing it returns
/// the highlight to the launcher rows.
pub fn toggle_legacy(cache: &mut ProjectViewCache) -> bool {
    if !has_legacy(cache) {
        return false;
    }
    cache.legacy_expanded = !cache.legacy_expanded;
    if cache.legacy_expanded {
        if cache.launcher_runs.is_empty() {
            cache.legacy_row_selected = true;
            cache.driver_scroll_offset = 0;
            cache.driver_follow = true;
        }
    } else {
        cache.legacy_row_selected = false;
    }
    true
}

// ── Which keys act on the current selection (the footer's input) ───────────

/// Which Driver-tab keys are available for the current selection, so the footer
/// names only keys that do something and no hint promises a no-op.
///
/// Later plans add their own fields (control, answer, edit, relaunch).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DriverKeys {
    /// `x`: a launcher row is selected and its run may be killed.
    pub kill: bool,
    /// `L`: legacy runs exist.
    pub legacy: bool,
    /// A legacy row is selected: the tab is read-only for it.
    pub legacy_row: bool,
    /// `s`: bound (it opens the shipped start flow until plan 30-05 rebinds it).
    pub start: bool,
    /// `PgUp`/`PgDn`/`f`: there is an output pane to scroll, which is the legacy
    /// run's (plan 30-02 extends it to the LOG pane).
    pub scroll: bool,
}

/// The keys available for `alias`'s current selection. Pure over the context.
pub fn driver_keys(ctx: &AppContext, alias: &str) -> DriverKeys {
    let cache = ctx.view_cache.get(alias);
    let sel = selection(cache);
    DriverKeys {
        kill: match (sel, cache) {
            (Selection::Launcher(index), Some(cache)) => {
                entry_killable(&cache.launcher_runs[index])
            }
            _ => false,
        },
        legacy: cache.is_some_and(has_legacy),
        legacy_row: matches!(sel, Selection::Legacy(_)),
        start: true,
        scroll: matches!(sel, Selection::Legacy(_)),
    }
}

// ── The help legend ─────────────────────────────────────────────────────────

/// One row of the `Driver Run States` help legend per UI-SPEC run status: the glyph
/// (a fixed constant), the detail word, and a one-line gloss that promises no more
/// than the lock probe and the written status can establish.
pub const RUN_STATES_LEGEND: [(&str, &str, &str); 12] = [
    (
        GLYPH_RUN_LIVE,
        "running",
        "run lock held; the driver is working",
    ),
    (
        GLYPH_RUN_LIVE,
        "starting",
        "run lock held; the driver has not begun",
    ),
    (
        GLYPH_RUN_CRASHED,
        "crashed",
        "run lock free with no final status written",
    ),
    (
        GLYPH_RUN_UNKNOWN,
        "liveness unknown",
        "lock probe failed: neither crashed nor running",
    ),
    (
        GLYPH_RUN_UNLAUNCHED,
        "not launched",
        "minted and never launched",
    ),
    (
        GLYPH_RUN_PARKED,
        "parked",
        "stopped itself: it needs a human",
    ),
    (GLYPH_RUN_PAUSED, "paused", "held by an operator"),
    (
        GLYPH_RUN_STOPPED,
        "stopped",
        "ended by a stop or a tripped cap",
    ),
    (GLYPH_RUN_DONE, "done", "finished all its items"),
    (
        GLYPH_RUN_CRASHED,
        "stalled",
        "ended its turn with a worker unreported",
    ),
    (
        GLYPH_RUN_UNKNOWN,
        "other word",
        "a status word this build does not know",
    ),
    (
        GLYPH_RUN_UNKNOWN,
        "unreadable",
        "its DIRECTION.md could not be read",
    ),
];

// ── The composition ─────────────────────────────────────────────────────────

/// The narrow tier's pane title: ` {run id} ({i}/{n}) `, sanitised and cut to the
/// width the pane has.
fn narrow_title(entry: &DriverRunEntry, index: usize, total: usize, width: u16) -> String {
    let suffix = format!(" ({}/{}) ", index + 1, total);
    let room = usize::from(width).saturating_sub(Span::raw(suffix.as_str()).width() + 1);
    format!(" {}{suffix}", fit_cells(&shown_capped(&entry.0), room))
}

/// The empty state: a bordered block with the UI-SPEC heading and body.
fn render_empty_state(frame: &mut Frame, area: Rect, alias: &str) {
    // The registry key is drawn here: both escape classes, composed.
    let alias = shown_capped(alias);
    let body = if cfg!(target_os = "linux") {
        "Press [s] to start one: name it, edit its DIRECTION, check it, then launch."
    } else {
        "Starting and steering runs needs Linux. Runs started elsewhere show here read-only."
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(DRIVER_PANE_TITLE);
    let message = Paragraph::new(vec![
        Line::from(format!("No driver runs yet for \"{alias}\".")),
        Line::from(Span::styled(body, label_style())),
    ])
    .block(block)
    .wrap(Wrap { trim: false });
    frame.render_widget(message, area);
}

/// The collapsed (`▸`) or expanded (`▾`) legacy-section row, DarkGray.
fn legacy_header_row(count: usize, expanded: bool) -> Line<'static> {
    let glyph = if expanded {
        LEGACY_EXPANDED
    } else {
        LEGACY_COLLAPSED
    };
    Line::from(Span::styled(
        format!("  {glyph} legacy runs ({count}), read-only"),
        label_style(),
    ))
}

/// One legacy run row: the shipped `run_list_row` behind the same two-cell highlight
/// column the launcher rows use.
fn legacy_row(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    selected: bool,
    inner_width: u16,
) -> Line<'static> {
    let prefix = if selected {
        Span::styled("> ", selection_style())
    } else {
        Span::raw("  ")
    };
    let mut line = run_list_row(summary, verdict, inner_width.saturating_sub(2));
    line.spans.insert(0, prefix);
    line
}

/// Render the launcher-first Driver tab: the run list (two panes from
/// `DRIVER_TWO_PANE_MIN_CELLS` columns) and the selected run's detail.
///
/// The list is the launcher runs, newest first, then (only when Phase 17/18
/// journal runs exist) one DarkGray `legacy runs` row that `L` expands into the
/// shipped legacy rows. The detail pane shows the selected launcher run's header,
/// or the shipped legacy run detail for a selected legacy row, or the dry-run
/// preview while the legacy start flow is at Step B, or the empty state.
///
/// Reads only `cache`; opens no file (D-25). Before the first
/// `Action::LauncherRunsRefreshed` for the alias lands, the detail pane shows
/// `Loading...` and no status glyph: an empty state there would claim there are no
/// runs when nobody has looked yet. The list's rows are recorded for the mouse.
pub(super) fn render_launcher_tab(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    viewport: &Cell<ViewportMetrics>,
    list_offset: &Cell<usize>,
) -> DriverTabRegions {
    let mut regions = DriverTabRegions::default();
    if area.width == 0 || area.height == 0 {
        return regions;
    }
    let runs: &[DriverRunEntry] = cache.map_or(&[], |c| c.launcher_runs.as_slice());
    let legacy: &[RunSummary] = cache.map_or(&[], |c| c.driver_runs.as_slice());
    let listed = cache.is_some_and(|c| c.launcher_listed);
    let preview = cache.and_then(|c| c.driver_dry_run.as_ref());

    // Nothing to list, nothing to preview, and the listing has landed: the whole
    // tab is the empty state.
    if listed && runs.is_empty() && legacy.is_empty() && preview.is_none() {
        render_empty_state(frame, area, alias);
        return regions;
    }

    let sel = selection(cache);
    let expanded = cache.is_some_and(legacy_rows_shown);
    let (list_area, detail_area) = driver_panes(area);

    // The verdict applies only to the run the reconciliation scan is observing;
    // every other legacy row is a finished run whose own `outcome` speaks for it.
    let observed = ctx.observed_runs.get(alias);
    let verdict_for = |run: &RunSummary| -> Option<RunVerdict> {
        observed
            .filter(|observed| observed.run_id == run.run_id)
            .map(|observed| observed.verdict())
    };

    if let Some(list_area) = list_area {
        let list_block = Block::default().borders(Borders::RIGHT).title(" Runs ");
        let rows = list_block.inner(list_area);
        let mut items: Vec<ListItem> = runs
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                ListItem::new(run_row(
                    entry,
                    sel == Selection::Launcher(index),
                    rows.width,
                ))
            })
            .collect();
        if !legacy.is_empty() {
            items.push(ListItem::new(legacy_header_row(legacy.len(), expanded)));
            if expanded {
                items.extend(legacy.iter().enumerate().map(|(index, run)| {
                    ListItem::new(legacy_row(
                        run,
                        verdict_for(run),
                        sel == Selection::Legacy(index),
                        rows.width,
                    ))
                }));
            }
        }
        let len = items.len();
        // The row carries its own `"> "` highlight, so the widget needs none; the
        // state exists for the persisted offset, so a clicked row stays put.
        let mut list_state = ListState::default().with_offset(list_offset.get());
        list_state.select(match sel {
            Selection::Launcher(index) => Some(index),
            Selection::Legacy(index) => Some(runs.len() + 1 + index),
            Selection::Nothing => None,
        });
        frame.render_stateful_widget(
            List::new(items).block(list_block),
            list_area,
            &mut list_state,
        );
        list_offset.set(list_state.offset());
        regions.list = Some(crate::ui::mouse::ListRegion {
            rect: rows,
            offset: list_state.offset(),
            len,
        });
    }

    // The empty state beside a collapsed legacy row: a bordered block in the
    // detail pane, `[inferred — audit]`.
    if sel == Selection::Nothing && preview.is_none() && listed {
        render_empty_state(frame, detail_area, alias);
        return regions;
    }

    // At the narrow tier the selection has nowhere else to go, so its identity
    // becomes the pane's title. `j`/`k` still move it.
    let title = match (list_area, sel) {
        (None, Selection::Launcher(index)) => {
            narrow_title(&runs[index], index, runs.len(), detail_area.width)
        }
        (None, Selection::Legacy(index)) => run_list_title(&legacy[index], index, legacy.len()),
        _ => DRIVER_PANE_TITLE.to_string(),
    };
    let detail_block = Block::default().borders(Borders::NONE).title(title);
    let inner = detail_block.inner(detail_area);
    frame.render_widget(detail_block, detail_area);

    // The dry-run preview replaces the detail while the legacy start flow is at
    // Step B, and replaces nothing else (D-26).
    if let Some(preview) = preview {
        render_dry_run_preview(frame, inner, preview);
        return regions;
    }
    match sel {
        Selection::Legacy(index) => {
            let run = &legacy[index];
            render_run_detail(
                frame,
                inner,
                ctx,
                alias,
                cache,
                run,
                verdict_for(run),
                viewport,
            );
            regions.output = Some(detail_area);
        }
        Selection::Launcher(index) => {
            let lines = detail_lines(&runs[index], usize::from(inner.width));
            frame.render_widget(Paragraph::new(lines), inner);
        }
        Selection::Nothing => {
            // Not listed yet.
            let loading = Line::from(Span::styled(DRY_RUN_LOADING, muted_style()));
            frame.render_widget(Paragraph::new(loading), inner);
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::driver_frontmatter::{
        parse_driver_frontmatter, DriverRunView, FrontmatterError,
    };
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/driverctl/direction/s003-run1b.md");

    fn entry_with(
        id: &str,
        sandbox: Option<&str>,
        reason: Option<&str>,
        derived: DerivedStatus,
    ) -> DriverRunEntry {
        let mut frontmatter = parse_driver_frontmatter(FIXTURE).expect("the spike fixture parses");
        frontmatter.run_id = id.to_owned();
        frontmatter.sandbox = sandbox.map(str::to_owned);
        frontmatter.status_reason = reason.map(str::to_owned);
        (
            id.to_owned(),
            Ok(DriverRunView {
                frontmatter,
                derived,
            }),
        )
    }

    fn entry(id: &str, derived: DerivedStatus) -> DriverRunEntry {
        entry_with(id, None, None, derived)
    }

    fn crashed() -> DerivedStatus {
        DerivedStatus::Crashed {
            written: RunStatus::Running,
        }
    }

    fn running() -> DerivedStatus {
        DerivedStatus::AsWritten(RunStatus::Running)
    }

    fn cache_of(rows: Vec<DriverRunEntry>) -> ProjectViewCache {
        ProjectViewCache {
            launcher_runs: rows,
            launcher_listed: true,
            ..ProjectViewCache::default()
        }
    }

    fn buffer_rows(buffer: &Buffer, width: u16, height: u16) -> Vec<String> {
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                    .collect::<String>()
            })
            .collect()
    }

    fn render(cache: ProjectViewCache, width: u16, height: u16) -> (Vec<String>, Buffer) {
        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;
        let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
        ctx.view_cache.insert(alias.to_string(), cache);
        let viewport = Cell::default();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_launcher_tab(
                    frame,
                    area,
                    &ctx,
                    alias,
                    ctx.view_cache.get(alias),
                    &viewport,
                    &Cell::new(0),
                );
            })
            .expect("draw the launcher tab");
        let buffer = terminal.backend().buffer().clone();
        (buffer_rows(&buffer, width, height), buffer)
    }

    fn row_with<'a>(rows: &'a [String], needle: &str) -> &'a str {
        rows.iter()
            .find(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no row contains {needle:?}:\n{}", rows.join("\n")))
    }

    // ── status_cell: the UI-SPEC table ─────────────────────────────────────

    #[test]
    fn status_cell_follows_the_ui_spec_table_for_every_derived_status() {
        let cases: Vec<(DerivedStatus, &str, &str, &str, Color)> = vec![
            (
                running(),
                GLYPH_RUN_LIVE,
                "running",
                "running",
                Color::Magenta,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Starting),
                GLYPH_RUN_LIVE,
                "starting",
                "starting",
                Color::Magenta,
            ),
            (
                crashed(),
                GLYPH_RUN_CRASHED,
                "crashed",
                "crashed",
                Color::Red,
            ),
            (
                DerivedStatus::LivenessUnknown {
                    written: RunStatus::Running,
                },
                GLYPH_RUN_UNKNOWN,
                "liveness?",
                "liveness unknown",
                Color::Yellow,
            ),
            (
                DerivedStatus::NotLaunched,
                GLYPH_RUN_UNLAUNCHED,
                "unlaunched",
                "not launched",
                Color::DarkGray,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Parked),
                GLYPH_RUN_PARKED,
                "parked",
                "parked",
                Color::Red,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Paused),
                GLYPH_RUN_PAUSED,
                "paused",
                "paused",
                Color::Yellow,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Stopped),
                GLYPH_RUN_STOPPED,
                "stopped",
                "stopped",
                Color::DarkGray,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Done),
                GLYPH_RUN_DONE,
                "done",
                "done",
                Color::Green,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Stalled),
                GLYPH_RUN_CRASHED,
                "stalled",
                "stalled",
                Color::Red,
            ),
            (
                DerivedStatus::AsWritten(RunStatus::Crashed),
                GLYPH_RUN_CRASHED,
                "crashed",
                "crashed",
                Color::Red,
            ),
        ];
        for (derived, glyph, list_word, detail_word, color) in cases {
            let cell = status_cell(&entry("20261020-a", derived.clone()));
            assert_eq!(cell.glyph, glyph, "{derived:?}");
            assert_eq!(cell.list_word, list_word, "{derived:?}");
            assert_eq!(cell.detail_word, detail_word, "{derived:?}");
            assert_eq!(cell.style.fg, Some(color), "{derived:?}");
        }
    }

    #[test]
    fn a_crashed_status_is_red_and_bold_and_a_live_one_is_not() {
        let crashed = status_cell(&entry("a", crashed()));
        assert!(crashed.style.add_modifier.contains(Modifier::BOLD));
        let live = status_cell(&entry("a", running()));
        assert!(!live.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn an_unknown_status_word_is_sanitised_and_cut_to_the_word_column() {
        let cell = status_cell(&entry(
            "a",
            DerivedStatus::AsWritten(RunStatus::Unknown(
                "up\u{1b}[2Jgraded-beyond-ten".to_owned(),
            )),
        ));
        assert_eq!(cell.glyph, GLYPH_RUN_UNKNOWN);
        assert!(!cell.list_word.chars().any(char::is_control), "{cell:?}");
        assert!(!cell.detail_word.chars().any(char::is_control), "{cell:?}");
        assert!(Span::raw(cell.list_word.as_str()).width() <= RUN_WORD_CELLS);
        assert!(cell.list_word.ends_with('\u{2026}'), "{cell:?}");
    }

    #[test]
    fn an_unreadable_entry_is_unreadable_yellow() {
        let cell = status_cell(&(
            "20261020-bad".to_owned(),
            Err(FrontmatterError::NoFrontmatter),
        ));
        assert_eq!(
            (cell.glyph, cell.list_word.as_str(), cell.style.fg),
            (GLYPH_RUN_UNKNOWN, "unreadable", Some(Color::Yellow))
        );
    }

    #[test]
    fn the_status_never_comes_from_the_written_word_or_the_reason() {
        // A forged `done` reason and a `running` written word on a free lock:
        // the cell reads crashed and nothing the file said changes that.
        let forged = entry_with("a", None, Some("all done, healthy, running"), crashed());
        let cell = status_cell(&forged);
        assert_eq!(
            (cell.glyph, cell.list_word.as_str()),
            (GLYPH_RUN_CRASHED, "crashed")
        );
    }

    // ── run_row ────────────────────────────────────────────────────────────

    fn text_of(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn a_narrow_row_has_no_word_column_and_a_wide_row_has_one() {
        let row = entry("20261020-driver-tab", running());
        assert_eq!(
            text_of(&run_row(&row, true, 30)).trim_end(),
            "> \u{25C6} 20261020-driver-tab"
        );
        assert_eq!(
            text_of(&run_row(&row, false, 40)).trim_end(),
            "  \u{25C6} running    20261020-driver-tab"
        );
    }

    #[test]
    fn the_posture_cell_marks_a_degraded_run_and_only_that() {
        let degraded = entry_with("20261020-a", Some("degraded"), None, running());
        let sandboxed = entry_with("20261020-b", Some("bwrap-scope"), None, running());
        let bare = entry("20261020-c", running());
        assert!(text_of(&run_row(&degraded, false, 40)).ends_with('!'));
        assert!(!text_of(&run_row(&sandboxed, false, 40)).contains('!'));
        assert!(!text_of(&run_row(&bare, false, 40)).contains('!'));
        let posture = run_row(&degraded, false, 40).spans.pop().unwrap();
        assert_eq!(posture.style.fg, Some(Color::Yellow));
        assert!(posture.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_long_run_id_is_cut_before_the_posture_cell_and_never_overflows() {
        let long = "20261020-an-extremely-long-run-identifier-that-cannot-fit-on-one-row";
        let row = entry_with(long, Some("degraded"), None, running());
        for width in [16u16, 20, 30, 36, 40] {
            let line = run_row(&row, false, width);
            let text = text_of(&line);
            assert!(line.width() <= usize::from(width), "{width}: {text:?}");
            assert!(
                text.ends_with('!'),
                "{width}: the posture survives: {text:?}"
            );
            assert!(
                text.contains('\u{2026}'),
                "{width}: the id is cut: {text:?}"
            );
        }
    }

    #[test]
    fn a_cjk_or_emoji_run_id_is_cut_by_display_width() {
        let wide = "日本語のとても長い実行識別子日本語のとても長い実行識別子";
        let emoji = "🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀";
        for id in [wide, emoji] {
            let row = entry_with(id, Some("degraded"), None, crashed());
            for width in [12u16, 17, 24, 36, 40] {
                let line = run_row(&row, true, width);
                assert!(
                    line.width() <= usize::from(width),
                    "{id:?} at {width} cells measured {}: {:?}",
                    line.width(),
                    text_of(&line)
                );
            }
        }
    }

    #[test]
    fn a_hostile_run_id_reaches_the_row_sanitised() {
        let row = entry("evil\u{1b}]0;pwned\u{7}\u{1b}[31mid\u{202e}x", running());
        let text = text_of(&run_row(&row, false, 60));
        assert!(!text.chars().any(char::is_control), "{text:?}");
        assert!(!text.contains('\u{202e}'), "{text:?}");
    }

    // ── the composition ────────────────────────────────────────────────────

    #[test]
    fn three_runs_render_their_derived_glyph_and_word() {
        let rows_in = || {
            cache_of(vec![
                entry("20261020-crashed", crashed()),
                entry("20261019-running", running()),
                entry("20261018-fresh", DerivedStatus::NotLaunched),
            ])
        };
        // 80 columns: the list's inner width (31) is below the word column's
        // floor (36), so each row carries its glyph and run id only.
        let (rows, _) = render(rows_in(), 80, 24);
        assert!(row_with(&rows, "20261020-crashed").contains("\u{2717} 20261020-crashed"));
        assert!(row_with(&rows, "20261019-running").contains("\u{25C6} 20261019-running"));
        assert!(row_with(&rows, "20261018-fresh").contains("\u{25CB} 20261018-fresh"));
        assert!(rows
            .iter()
            .any(|row| row.contains("Driver \u{2014} EXPERIMENTAL")));

        // 100 columns: the word column appears.
        let (rows, _) = render(rows_in(), 100, 24);
        let crashed_row = row_with(&rows, "20261020-crashed");
        assert!(crashed_row.contains("\u{2717} crashed"), "{crashed_row:?}");
        let running_row = row_with(&rows, "20261019-running");
        assert!(running_row.contains("\u{25C6} running"), "{running_row:?}");
        let fresh_row = row_with(&rows, "20261018-fresh");
        assert!(fresh_row.contains("\u{25CB} unlaunched"), "{fresh_row:?}");
        // The first row is selected.
        assert!(crashed_row.contains("> "), "{crashed_row:?}");
    }

    #[test]
    fn the_crashed_run_is_red_and_bold_in_its_list_row_and_its_header() {
        let cache = cache_of(vec![entry("20261020-crashed", crashed())]);
        let (_, buffer) = render(cache, 80, 24);
        let mut hits = 0;
        for y in 0..24u16 {
            for x in 0..80u16 {
                let Some(cell) = buffer.cell((x, y)) else {
                    continue;
                };
                if cell.symbol() == "\u{2717}" {
                    assert_eq!(cell.fg, Color::Red, "({x},{y})");
                    assert!(cell.modifier.contains(Modifier::BOLD), "({x},{y})");
                    hits += 1;
                }
            }
        }
        assert_eq!(hits, 2, "one glyph in the list row, one in the header");
    }

    #[test]
    fn the_header_reads_the_lock_not_the_file_for_a_crashed_run() {
        let cache = cache_of(vec![entry_with(
            "20261020-crashed",
            None,
            Some("driver dispatched I-03"),
            crashed(),
        )]);
        let (rows, buffer) = render(cache, 120, 24);
        let reason = row_with(&rows, "reason");
        assert!(
            reason.contains("run lock released without a final status (file says running)"),
            "{reason:?}"
        );
        assert!(
            !reason.contains("driver dispatched"),
            "the file's own reason must not replace the lock's verdict: {reason:?}"
        );
        // The reason is Red.
        let y = rows.iter().position(|row| row.contains("reason")).unwrap();
        let x = rows[y].find("run lock").unwrap();
        let col = u16::try_from(rows[y][..x].chars().count()).unwrap();
        assert_eq!(
            buffer.cell((col, u16::try_from(y).unwrap())).unwrap().fg,
            Color::Red
        );
    }

    #[test]
    fn the_header_says_the_probe_failed_for_liveness_unknown_and_never_crashed() {
        let cache = cache_of(vec![entry(
            "20261020-unknown",
            DerivedStatus::LivenessUnknown {
                written: RunStatus::Running,
            },
        )]);
        let (rows, _) = render(cache, 120, 24);
        let header = row_with(&rows, "liveness unknown");
        assert!(header.contains("20261020-unknown"), "{header:?}");
        let reason = row_with(&rows, "reason");
        assert!(
            reason.contains("lock probe failed (file says running)"),
            "{reason:?}"
        );
        assert!(!rows.iter().any(|row| row.contains("crashed")), "{rows:#?}");
    }

    #[test]
    fn another_status_shows_its_reason_or_none_recorded() {
        let with = cache_of(vec![entry_with(
            "20261020-a",
            None,
            Some("driver dispatched I-03"),
            running(),
        )]);
        let (rows, _) = render(with, 100, 24);
        assert!(row_with(&rows, "reason").contains("driver dispatched I-03"));

        let without = cache_of(vec![entry("20261020-a", running())]);
        let (rows, _) = render(without, 100, 24);
        assert!(row_with(&rows, "reason").contains("(none recorded)"));
    }

    #[test]
    fn before_the_first_listing_the_detail_says_loading_and_shows_no_glyph() {
        let cache = ProjectViewCache::default();
        let (rows, _) = render(cache, 80, 24);
        assert!(
            rows.iter().any(|row| row.contains("Loading...")),
            "{rows:#?}"
        );
        assert!(!rows.iter().any(|row| row.contains("No driver runs yet")));
        for glyph in [
            GLYPH_RUN_LIVE,
            GLYPH_RUN_CRASHED,
            GLYPH_RUN_UNKNOWN,
            GLYPH_RUN_UNLAUNCHED,
        ] {
            assert!(!rows.iter().any(|row| row.contains(glyph)), "{glyph}");
        }
    }

    #[test]
    fn a_project_with_no_runs_gets_the_empty_state_in_a_bordered_block() {
        let (rows, _) = render(cache_of(Vec::new()), 100, 12);
        assert!(
            rows.iter()
                .any(|row| row.contains("No driver runs yet for \"proj\".")),
            "{rows:#?}"
        );
        if cfg!(target_os = "linux") {
            assert!(rows.iter().any(|row| row.contains(
                "Press [s] to start one: name it, edit its DIRECTION, check it, then launch."
            )));
        } else {
            assert!(rows.iter().any(|row| row.contains("needs Linux")));
        }
        assert!(
            rows[0].starts_with('\u{250C}'),
            "a bordered block: {:?}",
            rows[0]
        );
    }

    #[test]
    fn an_unreadable_entry_is_listed_with_the_error_and_hides_no_other_row() {
        let cache = cache_of(vec![
            entry("20261020-ok", running()),
            (
                "20261019-bad".to_owned(),
                Err(FrontmatterError::Io {
                    detail: "denied".to_owned(),
                }),
            ),
        ]);
        let (rows, _) = render(cache, 100, 24);
        assert!(row_with(&rows, "20261019-bad").contains("unreadable"));
        assert!(row_with(&rows, "20261020-ok").contains("running"));

        let mut cache = cache_of(vec![
            entry("20261020-ok", running()),
            (
                "20261019-bad".to_owned(),
                Err(FrontmatterError::Io {
                    detail: "denied".to_owned(),
                }),
            ),
        ]);
        cache.launcher_selected = 1;
        let (rows, _) = render(cache, 140, 24);
        assert!(row_with(&rows, "DIRECTION.md could not be read").contains("denied"));
        assert!(rows
            .iter()
            .any(|row| row.contains("It is read again on the next change to the run directory.")));
    }

    #[test]
    fn the_narrow_tier_titles_the_detail_with_the_selected_run() {
        let mut cache = cache_of(vec![
            entry("20261020-first", running()),
            entry("20261019-second", crashed()),
        ]);
        cache.launcher_selected = 1;
        let (rows, _) = render(cache, 50, 12);
        assert!(rows[0].contains("20261019-second (2/2)"), "{:?}", rows[0]);
        // No list pane at this width: only the one run's header is drawn.
        assert!(!rows.iter().any(|row| row.contains("20261020-first")));
    }

    #[test]
    fn a_selection_taller_than_the_list_stays_visible_through_the_offset() {
        let rows_in: Vec<DriverRunEntry> = (0..20)
            .map(|n| entry(&format!("202610{:02}-run", 30 - n), running()))
            .collect();
        let mut cache = cache_of(rows_in);
        cache.launcher_selected = 17;
        let offset = Cell::new(0usize);
        let dir = tempfile::tempdir().unwrap();
        let alias = super::super::driver_confirm::tests::ALIAS;
        let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
        ctx.view_cache.insert(alias.to_string(), cache);
        let viewport = Cell::default();
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        let mut regions = DriverTabRegions::default();
        terminal
            .draw(|frame| {
                regions = render_launcher_tab(
                    frame,
                    frame.area(),
                    &ctx,
                    alias,
                    ctx.view_cache.get(alias),
                    &viewport,
                    &offset,
                );
            })
            .unwrap();
        let rows = buffer_rows(terminal.backend().buffer(), 80, 10);
        assert!(row_with(&rows, "> ").contains("run"), "{rows:#?}");
        assert!(offset.get() > 0, "the list scrolled to keep row 17 visible");
        let region = regions.list.expect("the list region is recorded");
        assert_eq!((region.len, region.offset), (20, offset.get()));
    }

    #[test]
    fn the_rendered_buffer_holds_no_control_character_for_hostile_text() {
        let cache = cache_of(vec![entry_with(
            "evil\u{1b}]0;pwned\u{7}id",
            Some("degraded"),
            Some("reason\u{1b}[2J\u{202e}"),
            DerivedStatus::AsWritten(RunStatus::Unknown("up\u{1b}[2Jword".to_owned())),
        )]);
        let (rows, _) = render(cache, 100, 24);
        for row in &rows {
            assert!(!row.chars().any(char::is_control), "{row:?}");
            assert!(!row.contains('\u{202e}'), "{row:?}");
        }
    }

    // ── kill availability, selection and the legacy section ────────────────

    #[test]
    fn kill_is_available_for_live_crashed_and_unknown_runs_only() {
        use RunStatus::*;
        for (derived, expected) in [
            (DerivedStatus::AsWritten(Running), true),
            (DerivedStatus::AsWritten(Starting), true),
            (DerivedStatus::Crashed { written: Running }, true),
            (DerivedStatus::LivenessUnknown { written: Running }, true),
            (DerivedStatus::NotLaunched, false),
            (DerivedStatus::AsWritten(Stopped), false),
            (DerivedStatus::AsWritten(Done), false),
            (DerivedStatus::AsWritten(Paused), false),
            (DerivedStatus::AsWritten(Parked), false),
            (DerivedStatus::AsWritten(Stalled), false),
            (DerivedStatus::AsWritten(Crashed), false),
        ] {
            assert_eq!(kill_available(&derived), expected, "{derived:?}");
        }
    }

    fn summary(run_id: &str) -> RunSummary {
        RunSummary {
            run_id: run_id.to_owned(),
            started_at: "2026-07-29T21:40:00Z".to_owned(),
            ended_at: None,
            goal: "ship the tab".to_owned(),
            gsd_command: "/gsd:execute-phase 18".to_owned(),
            outcome: None,
        }
    }

    fn mixed_cache(launcher: usize, legacy: usize) -> ProjectViewCache {
        let mut cache = cache_of(
            (0..launcher)
                .map(|n| entry(&format!("2026102{n}-run"), running()))
                .collect(),
        );
        cache.driver_runs = (0..legacy)
            .map(|n| summary(&format!("2026-07-29T21-40-0{n}Z-3f2a")))
            .collect();
        cache
    }

    #[test]
    fn the_selection_walks_launcher_rows_then_legacy_rows_only_when_expanded() {
        let mut cache = mixed_cache(2, 2);
        assert_eq!(selection(Some(&cache)), Selection::Launcher(0));
        assert!(!move_selection(&mut cache, 1));
        assert!(
            !move_selection(&mut cache, 5),
            "collapsed: stops at launcher 1"
        );
        assert_eq!(selection(Some(&cache)), Selection::Launcher(1));

        cache.legacy_expanded = true;
        assert!(move_selection(&mut cache, 1), "onto the first legacy row");
        assert_eq!(selection(Some(&cache)), Selection::Legacy(0));
        assert_eq!(selection_position(Some(&cache)), 2);
        assert!(
            move_selection(&mut cache, 9),
            "clamped at the last legacy row"
        );
        assert_eq!(selection(Some(&cache)), Selection::Legacy(1));
        assert!(!move_selection(&mut cache, -2), "back onto a launcher row");
        assert_eq!(selection(Some(&cache)), Selection::Launcher(1));
        assert_eq!(cache.launcher_selected_id.as_deref(), Some("20261021-run"));
    }

    #[test]
    fn moving_onto_a_different_legacy_run_resets_the_output_pane() {
        let mut cache = mixed_cache(0, 2);
        cache.legacy_expanded = true;
        cache.legacy_row_selected = true;
        cache.driver_scroll_offset = 40;
        cache.driver_follow = false;
        assert!(move_selection(&mut cache, 1));
        assert_eq!((cache.driver_scroll_offset, cache.driver_follow), (0, true));
        // Staying on the same run leaves the pane where the user put it.
        cache.driver_scroll_offset = 7;
        cache.driver_follow = false;
        assert!(move_selection(&mut cache, 1), "still a legacy row");
        assert_eq!(
            (cache.driver_scroll_offset, cache.driver_follow),
            (7, false)
        );
    }

    #[test]
    fn a_legacy_selection_is_honoured_only_while_its_rows_are_shown() {
        let mut cache = mixed_cache(1, 1);
        cache.legacy_row_selected = true;
        assert_eq!(
            selection(Some(&cache)),
            Selection::Launcher(0),
            "collapsed: the stored flag cannot leave the detail on a hidden row"
        );
        cache.legacy_expanded = true;
        assert_eq!(selection(Some(&cache)), Selection::Legacy(0));
        assert_eq!(selection(None), Selection::Nothing);
    }

    #[test]
    fn toggling_the_section_lands_the_highlight_sensibly() {
        // Inert without legacy runs.
        let mut cache = mixed_cache(1, 0);
        assert!(!toggle_legacy(&mut cache));
        assert!(!cache.legacy_expanded);

        // With launcher rows the highlight stays on its launcher run.
        let mut cache = mixed_cache(1, 1);
        assert!(toggle_legacy(&mut cache));
        assert_eq!(selection(Some(&cache)), Selection::Launcher(0));

        // With nothing else in the list it goes onto the first legacy run, and
        // collapsing gives it back.
        let mut cache = mixed_cache(0, 1);
        assert!(toggle_legacy(&mut cache));
        assert_eq!(selection(Some(&cache)), Selection::Legacy(0));
        assert!(toggle_legacy(&mut cache));
        assert_eq!(selection(Some(&cache)), Selection::Nothing);
    }

    #[test]
    fn a_clicked_row_maps_through_the_section_header() {
        let mut cache = mixed_cache(2, 2);
        // Collapsed: rows are launcher 0, launcher 1, header.
        assert_eq!(position_for_row(&cache, 1), Some(1));
        assert_eq!(position_for_row(&cache, 2), None, "the header");
        assert_eq!(position_for_row(&cache, 3), None, "past the end");
        // Expanded: rows are launcher 0, launcher 1, header, legacy 0, legacy 1.
        cache.legacy_expanded = true;
        assert_eq!(position_for_row(&cache, 2), None, "the header");
        assert_eq!(position_for_row(&cache, 3), Some(2));
        assert_eq!(position_for_row(&cache, 4), Some(3));
        assert_eq!(position_for_row(&cache, 5), None);
    }

    #[test]
    fn a_launcher_run_selected_beside_a_collapsed_legacy_row_shows_the_launcher_header() {
        let (rows, _) = render(mixed_cache(2, 1), 140, 24);
        let first = rows
            .iter()
            .position(|row| row.contains("20261020-run"))
            .expect("the first launcher row");
        let legacy = rows
            .iter()
            .position(|row| row.contains("legacy runs (1), read-only"))
            .expect("the collapsed row");
        assert!(legacy > first, "{rows:#?}");
        assert!(rows[legacy].contains('\u{25B8}'));
        assert!(
            rows.iter().any(|row| row.contains("reason")),
            "launcher header"
        );
        assert!(
            !rows.iter().any(|row| row.contains("steps")),
            "no legacy detail"
        );
    }

    #[test]
    fn a_selected_legacy_row_renders_the_shipped_legacy_detail_and_no_launcher_header() {
        let mut cache = mixed_cache(1, 1);
        cache.legacy_expanded = true;
        cache.legacy_row_selected = true;
        let (rows, _) = render(cache, 140, 40);
        assert!(
            rows.iter()
                .any(|row| row.contains("\u{25BE} legacy runs (1), read-only")),
            "{rows:#?}"
        );
        // The shipped run detail's `steps` section rule.
        assert!(
            rows.iter()
                .any(|row| row.contains("\u{2500}\u{2500} steps ")),
            "{rows:#?}"
        );
        assert!(
            !rows.iter().any(|row| row.contains("reason  ")),
            "the launcher header must not draw over a legacy run: {rows:#?}"
        );
    }

    #[test]
    fn the_empty_state_sits_beside_a_collapsed_legacy_row() {
        let (rows, _) = render(mixed_cache(0, 1), 140, 24);
        assert!(
            rows.iter()
                .any(|row| row.contains("legacy runs (1), read-only")),
            "{rows:#?}"
        );
        assert!(
            rows.iter()
                .any(|row| row.contains("No driver runs yet for \"proj\".")),
            "{rows:#?}"
        );
    }

    #[test]
    fn the_dry_run_preview_replaces_the_detail_whatever_is_selected() {
        let mut cache = mixed_cache(1, 0);
        cache.driver_dry_run = Some(super::super::DryRunPreview {
            command: "/gsd:execute-phase 18".to_owned(),
            report: Some("Would run: /gsd:execute-phase 18\n".to_owned()),
        });
        let (rows, _) = render(cache, 140, 24);
        assert!(
            rows.iter().any(|row| row.contains("Would run:")),
            "{rows:#?}"
        );
        assert!(
            !rows.iter().any(|row| row.contains("reason  ")),
            "{rows:#?}"
        );
    }

    #[test]
    fn the_narrow_tier_titles_a_legacy_selection_with_its_run() {
        let mut cache = mixed_cache(0, 1);
        cache.legacy_expanded = true;
        cache.legacy_row_selected = true;
        let (rows, _) = render(cache, 50, 20);
        assert!(rows[0].contains("(1/1)"), "{:?}", rows[0]);
    }
}
