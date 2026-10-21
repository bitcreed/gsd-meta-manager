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
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::detail::{clamp_scroll, fit_cells, tail_offset, ViewportMetrics};
use super::driver::{
    driver_panes, follow_indicator, label_style, muted_style, pane_header_line,
    render_dry_run_preview, render_run_detail, ring_overflow_notice, run_list_row, run_list_title,
    shown_capped, DriverTabRegions, DRIVER_PANE_TITLE, DRY_RUN_LOADING,
};
use super::{AppContext, DriverLineKind, DriverOutput, LauncherLogTail, ProjectViewCache};
use crate::driver::reconcile::RunVerdict;
use crate::driver::steer::BodyItem;
use crate::journal::reader::{tail_file, TailCursor};
use crate::journal::RunSummary;
use crate::state_reader::driver_frontmatter::{
    Connection, DerivedStatus, DriverFrontmatter, DriverRunEntry, DriverRunView, ItemState,
    RunStatus,
};

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

/// `·` U+00B7 — an item that is in the DIRECTION body but that the driver has not
/// recorded a state for yet.
pub const GLYPH_ITEM_UNSEEN: &str = "\u{00B7}";

/// Item rows shown before the `… {k} more items` row (UI-SPEC `ITEMS_MAX_ROWS`).
pub const ITEMS_MAX_ROWS: usize = 8;

/// The item id column: `I-NNN` is five cells.
const ITEM_ID_CELLS: usize = 5;

/// The item state-word column: `unseen`/`pending`/`running` are at most seven
/// cells, padded to eight so the dispatch count has a gap before it.
const ITEM_WORD_CELLS: usize = 8;

/// `◈` U+25C8 — a run whose lock is held but whose heartbeat is older than the
/// file's own threshold (D-08). Never `crashed`: the lock is the liveness authority.
pub const GLYPH_RUN_STALE: &str = "\u{25C8}";

/// `█` U+2588 — a filled cap-bar cell.
const GLYPH_BAR_FILLED: &str = "\u{2588}";
/// `░` U+2591 — an empty cap-bar cell.
const GLYPH_BAR_EMPTY: &str = "\u{2591}";

/// Cells in a cap bar: one cell is 10 % of the limit.
pub const CAP_BAR_CELLS: usize = 10;

/// The fixed cells of a cap row: indent 2 + label 10 + gap 1 + value 9 (`9999/9999`,
/// right-aligned) + gap 1 + bar 10 = 33. Must stay `<= DRIVER_DETAIL_MIN_CELLS` (39)
/// so the bar is complete at the narrowest detail pane; a test asserts it.
pub const CAP_ROW_FIXED_CELLS: u16 = 33;

/// The value column of a cap row, wide enough for `9999/9999`.
const CAP_VALUE_CELLS: usize = 9;

/// Connection rows shown before the `… {k} more` row.
pub const CONNECTED_MAX_ROWS: usize = 3;

/// LOG tail content rows, below its rule (the layout tiers of plan 30-03 reserve it).
pub const LOG_MIN_ROWS: usize = 3;

/// The role column of a connection row (`launcher`, `driver`).
const CONNECTED_ROLE_CELLS: usize = 9;
/// The state column of a connection row.
const CONNECTED_STATE_CELLS: usize = 12;

/// The separator between the two halves of a header row.
const HEADER_SEP: &str = "  \u{B7}  ";

/// The marker after a frontmatter-only item's (empty) title.
const NOT_IN_DIRECTION: &str = "[not in DIRECTION]";

/// The cells every item row spends before its title, from the UI-SPEC row
/// `cursor(2) · id(5) · glyph(1) · gap(1) · word(8) · "{n}d" in 3 · gap(1) · flag(4)
/// · gap(2)`: `2 + 5 + 1 + 1 + 8 + 3 + 1 + 4 + 2 = 27`. The title gets whatever the
/// pane has beyond this, measured from the rendered spans rather than from the
/// constant, so a wider dispatch count cannot push it off the row.
pub const ITEM_ROW_FIXED_CELLS: usize = 27;

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
    status_cell_at(entry, SystemTime::now())
}

/// [`status_cell`] at a given instant: the same table, plus the stale case (D-08). A
/// run whose lock is held (`running`/`starting`) and whose heartbeat is older than the
/// file's own `heartbeat_stale_after_s` reads `◈ stale` in Yellow, with the written
/// word kept for the detail header. It is never called crashed.
pub fn status_cell_at(entry: &DriverRunEntry, now: SystemTime) -> StatusCell {
    let cell = base_status_cell(entry);
    match &entry.1 {
        Ok(view) if heartbeat_stale(&view.frontmatter, &view.derived, now) => StatusCell {
            glyph: GLYPH_RUN_STALE,
            list_word: "stale".to_owned(),
            style: Style::default().fg(Color::Yellow),
            ..cell
        },
        _ => cell,
    }
}

fn base_status_cell(entry: &DriverRunEntry) -> StatusCell {
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
    run_row_at(entry, selected, inner_width, SystemTime::now())
}

/// [`run_row`] at a given instant (the stale word depends on it).
pub fn run_row_at(
    entry: &DriverRunEntry,
    selected: bool,
    inner_width: u16,
    now: SystemTime,
) -> Line<'static> {
    let cell = status_cell_at(entry, now);
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

// ── Control, heartbeat and the stale rule (D-05, D-08) ──────────────────────

/// Whether a derived status is a live one: the run lock is held and the written
/// status is `running` or `starting`. The only liveness the stale rule accepts.
pub fn is_live(derived: &DerivedStatus) -> bool {
    matches!(
        derived,
        DerivedStatus::AsWritten(RunStatus::Running | RunStatus::Starting)
    )
}

/// Where a heartbeat stamp came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatSource {
    /// The `last_heartbeat` of the `connected[]` entry with `role: driver`.
    Driver,
    /// The newest `last_heartbeat` of any connection: the launcher's own, which is
    /// all there is until the driver's first heartbeat lands (the store records the
    /// driver entry at launch with `last_heartbeat: None`). `[inferred — audit]`
    LauncherStart,
}

/// The heartbeat stamp to age: the `role: driver` entry's `last_heartbeat`, else the
/// newest stamp among all connections, tagged [`HeartbeatSource::LauncherStart`].
/// `None` when no connection carries a stamp. "Newest" compares parsed instants; a
/// stamp that does not parse ranks below any that does.
pub fn heartbeat_source(fm: &DriverFrontmatter) -> Option<(String, HeartbeatSource)> {
    if let Some(stamp) = fm
        .connected
        .iter()
        .find(|c| c.role == "driver")
        .and_then(|c| c.last_heartbeat.as_deref())
    {
        return Some((stamp.to_owned(), HeartbeatSource::Driver));
    }
    let rank = |stamp: &str| crate::driverctl::log::epoch_seconds(stamp).unwrap_or(f64::MIN);
    let mut newest: Option<&str> = None;
    for stamp in fm
        .connected
        .iter()
        .filter_map(|c| c.last_heartbeat.as_deref())
    {
        if newest.is_none_or(|best| rank(stamp) > rank(best)) {
            newest = Some(stamp);
        }
    }
    newest.map(|stamp| (stamp.to_owned(), HeartbeatSource::LauncherStart))
}

/// `now − stamp` in whole seconds, rounded down. A stamp in the future (clock skew)
/// is `0`; one that does not parse is `None`, never a panic.
pub fn heartbeat_age_secs(stamp: &str, now: SystemTime) -> Option<u64> {
    let at = crate::driverctl::log::epoch_seconds(stamp)?;
    let now = now.duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
    let age = now - at;
    Some(if age.is_finite() && age > 0.0 {
        age.floor() as u64
    } else {
        0
    })
}

/// An age in seconds as `m:ss` under an hour, `h:mm:ss` under a day, `{d}d {h}h`
/// beyond.
pub fn format_age(secs: u64) -> String {
    match secs {
        0..=3599 => format!("{}:{:02}", secs / 60, secs % 60),
        3600..=86_399 => format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60),
        _ => format!("{}d {}h", secs / 86_400, secs % 86_400 / 3600),
    }
}

/// An RFC 3339 UTC stamp `ago_secs` before now. Test fixtures use it because the
/// shared fixture file's heartbeats are fixed dates, which a live run would read as
/// stale against the real clock.
#[cfg(test)]
pub(crate) fn stamp_ago(ago_secs: u64) -> String {
    let at = SystemTime::now() - std::time::Duration::from_secs(ago_secs);
    crate::driverctl::log::utc_string(chrono::DateTime::<chrono::Utc>::from(at))
}

/// Give every connection of a fixture frontmatter a heartbeat of right now.
#[cfg(test)]
pub(crate) fn refresh_heartbeats(fm: &mut DriverFrontmatter) {
    for conn in &mut fm.connected {
        conn.last_heartbeat = Some(stamp_ago(1));
    }
}

/// The one stale rule (D-08), reused by the run list, the header and the status
/// tokens: the run is live (`AsWritten(Running | Starting)`, which the reader returns
/// only while the run lock is held) **and** its heartbeat is older than the file's own
/// `heartbeat_stale_after_s`. The threshold is never a TUI constant, and a run that is
/// not live is never judged: its age is information, not a verdict. No heartbeat
/// stamp, or one that does not parse, is not stale either: there is nothing to age.
pub fn heartbeat_stale(fm: &DriverFrontmatter, derived: &DerivedStatus, now: SystemTime) -> bool {
    is_live(derived)
        && heartbeat_source(fm)
            .and_then(|(stamp, _)| heartbeat_age_secs(&stamp, now))
            .is_some_and(|age| age > fm.heartbeat_stale_after_s)
}

// ── Dashboard markers (30-03, D-12) ─────────────────────────────────────────

/// The class one launcher run contributes to its project's dashboard marker (D-12).
///
/// Declared in precedence order, **highest first**, so the derived `Ord` ranks
/// `Crashed` lowest and the project marker is the [`Ord::min`] of its runs' classes:
/// crashed > parked > driven > stale > unknown. The classes are exclusive per run
/// (first match in [`run_marker_class`]): a live run with a stale heartbeat is
/// `Stale`, never `Driven`, or the stale marker could never light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkerClass {
    /// The launcher is gone without a final status (free lock), or the launcher
    /// itself wrote `crashed`.
    Crashed,
    /// The run stopped for a human, or is live with an item parked.
    Parked,
    /// The run's lock is held and its heartbeat is fresh.
    Driven,
    /// The run's lock is held but its heartbeat is older than the file's threshold.
    Stale,
    /// The lock probe could not tell whether the run is alive.
    Unknown,
}

/// One project's launcher-run marker (D-12): the highest class among the runs
/// considered, and whether any of them holds its run lock (the tab-bar `D:Drive◆`
/// light). Plain data, so it travels in an `Action` and sits in the view cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LauncherMarker {
    /// `None` when no considered run has a class: nothing to flag.
    pub class: Option<MarkerClass>,
    /// Whether any considered run derives `AsWritten(Running | Starting)`.
    pub any_live: bool,
}

impl LauncherMarker {
    /// Whether the marker raises the attention flag (OBS-07, D-12): crashed, parked
    /// or stale. `Unknown` never counts, and neither does `Driven` (a healthy run).
    pub fn needs_attention(&self) -> bool {
        matches!(
            self.class,
            Some(MarkerClass::Crashed | MarkerClass::Parked | MarkerClass::Stale)
        )
    }
}

/// The class one run contributes, by the first match of the UI-SPEC table: crashed,
/// parked, stale, driven, unknown. `None` for a run that is none of those (not
/// launched, paused, stopped, done, stalled, an unknown status word, or an entry that
/// could not be read `[inferred — audit]`).
///
/// Chosen only from [`DerivedStatus`] (the lock probe), the closed [`ItemState`] enum
/// and the file's own heartbeat threshold; no text the driver wrote picks a class
/// (the Replit rule). A merely stale, liveness-unknown, not-launched or paused run is
/// never `Crashed`: a false crash invites a relaunch over a live run.
pub fn run_marker_class(entry: &DriverRunEntry, now: SystemTime) -> Option<MarkerClass> {
    let view = entry.1.as_ref().ok()?;
    let fm = &view.frontmatter;
    match &view.derived {
        // `AsWritten(Crashed)` is the launcher's own record; `launcher_marker` only
        // lets it through for the most recent run (a terminal status).
        DerivedStatus::Crashed { .. } | DerivedStatus::AsWritten(RunStatus::Crashed) => {
            Some(MarkerClass::Crashed)
        }
        DerivedStatus::AsWritten(RunStatus::Parked) => Some(MarkerClass::Parked),
        derived if is_live(derived) => {
            if fm
                .item_states
                .0
                .iter()
                .any(|(_, item)| item.state == ItemState::Parked)
            {
                Some(MarkerClass::Parked)
            } else if heartbeat_stale(fm, derived, now) {
                Some(MarkerClass::Stale)
            } else {
                Some(MarkerClass::Driven)
            }
        }
        DerivedStatus::LivenessUnknown { .. } => Some(MarkerClass::Unknown),
        _ => None,
    }
}

/// Reduce a project's newest-first run listing to its [`LauncherMarker`] (D-12).
///
/// The runs considered are every run whose **written** status is not terminal, plus
/// the most recent run (`entries[0]`), so an older run that is still going is not
/// hidden behind a newer finished one. The class is the maximum by precedence;
/// `any_live` is true when any considered run derives `AsWritten(Running | Starting)`.
/// An empty listing is no class. Pure: the lock probes already happened in the listing.
pub fn launcher_marker(entries: &[DriverRunEntry], now: SystemTime) -> LauncherMarker {
    let mut marker = LauncherMarker::default();
    for (index, entry) in entries.iter().enumerate() {
        let Ok(view) = &entry.1 else { continue };
        if index != 0 && view.frontmatter.status.is_terminal() {
            continue;
        }
        if is_live(&view.derived) {
            marker.any_live = true;
        }
        if let Some(class) = run_marker_class(entry, now) {
            marker.class = Some(marker.class.map_or(class, |best| best.min(class)));
        }
    }
    marker
}

/// The control word to show, and its pending suffix (D-05, D-28). `body_control` is
/// the LIVE word from the DIRECTION body; `fm_control` is what the frontmatter last
/// echoed.
///
/// Equal words (case-insensitive) show the word alone. Different words show the BODY
/// word with `(pending)` on a live run (the driver reads it before its next dispatch)
/// or `(at next launch)` on one that is not live; never `paused` or `stopped` for a
/// write nothing has echoed. No `control:` line in the body is `(no control line)`.
pub fn control_shown(
    body_control: Option<&str>,
    fm_control: Option<&str>,
    live: bool,
) -> (String, Option<&'static str>) {
    let Some(body) = body_control else {
        return ("(no control line)".to_owned(), None);
    };
    let echoed = fm_control.is_some_and(|fm| fm.trim().eq_ignore_ascii_case(body.trim()));
    let suffix = match (echoed, live) {
        (true, _) => None,
        (false, true) => Some("(pending)"),
        (false, false) => Some("(at next launch)"),
    };
    (shown_capped(body), suffix)
}

// ── Caps (D-07) ─────────────────────────────────────────────────────────────

/// A 10-cell bar: `filled = min(10, used * 10 / limit)` with floor division. A limit
/// of `0` is a full bar (and at-limit), never a division by zero. `[inferred — audit]`
pub fn cap_bar(used: u64, limit: u64) -> String {
    // A limit of 0 has no ratio: the bar is full rather than a division by zero.
    let filled = used
        .saturating_mul(10)
        .checked_div(limit)
        .map_or(CAP_BAR_CELLS, |cells| {
            usize::try_from(cells)
                .unwrap_or(CAP_BAR_CELLS)
                .min(CAP_BAR_CELLS)
        });
    format!(
        "{}{}",
        GLYPH_BAR_FILLED.repeat(filled),
        GLYPH_BAR_EMPTY.repeat(CAP_BAR_CELLS - filled)
    )
}

/// Whether a cap is at or over its limit: `used >= limit`, and a limit of `0` always.
pub fn cap_at_limit(used: u64, limit: u64) -> bool {
    limit == 0 || used >= limit
}

/// A fractional minutes figure as whole minutes, rounded down.
fn whole_minutes(value: f64) -> u64 {
    if value.is_finite() && value > 0.0 {
        value.floor() as u64
    } else {
        0
    }
}

/// One cap row: `indent 2 · label 10 · gap · value right-aligned 9 · gap · bar 10`,
/// then `extra` after the bar. A cap at or over its limit is Yellow throughout with
/// the value BOLD; a cap with no limit shows `{used}/–` and `no limit` muted instead
/// of a bar.
fn cap_row(
    label: &str,
    used: u64,
    limit: Option<u64>,
    extra: Option<String>,
    width: usize,
) -> Line<'static> {
    let at_limit = limit.is_some_and(|limit| cap_at_limit(used, limit));
    let base = if at_limit {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let value = match limit {
        Some(limit) => format!("{used}/{limit}"),
        None => format!("{used}/\u{2013}"),
    };
    let value_style = if at_limit {
        base.add_modifier(Modifier::BOLD)
    } else {
        base
    };
    let mut spans = vec![
        Span::styled(format!("  {} ", pad_cells(label, 10)), base),
        Span::styled(format!("{value:>CAP_VALUE_CELLS$}"), value_style),
        Span::styled(" ", base),
    ];
    spans.push(match limit {
        Some(limit) => Span::styled(cap_bar(used, limit), base),
        None => Span::styled("no limit", muted_style()),
    });
    if let Some(extra) = extra {
        spans.push(Span::styled(format!("  {extra}"), label_style()));
    }
    clip_line(Line::from(spans), width)
}

/// The audit row (D-07): `logged · observed · denied`. `logged != observed` makes the
/// whole row Red followed by ` MISMATCH` (Red, BOLD); `denied > 0` makes the denied
/// value Yellow. The widest form that fits `width` with its `MISMATCH` is used, so
/// the flag is never clipped off a narrow pane.
fn audit_row(
    audit: &crate::state_reader::driver_frontmatter::DispatchAudit,
    width: usize,
) -> Line<'static> {
    let mismatch = audit.logged != audit.observed;
    let base = if mismatch {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };
    let flag_cells = if mismatch { " MISMATCH".len() } else { 0 };
    let (l, o, d) = (audit.logged, audit.observed, audit.denied);
    let forms: [[String; 6]; 3] = [
        [
            "logged ".into(),
            l.to_string(),
            "  \u{B7}  observed ".into(),
            o.to_string(),
            "  \u{B7}  denied ".into(),
            d.to_string(),
        ],
        [
            "logged ".into(),
            l.to_string(),
            " \u{B7} observed ".into(),
            o.to_string(),
            " \u{B7} denied ".into(),
            d.to_string(),
        ],
        [
            String::new(),
            l.to_string(),
            "/".into(),
            o.to_string(),
            "/".into(),
            d.to_string(),
        ],
    ];
    let label = format!("  {:<7} ", "audit");
    let room = width.saturating_sub(Span::raw(label.as_str()).width() + flag_cells);
    let form = forms
        .iter()
        .find(|form| form.iter().map(|part| part.chars().count()).sum::<usize>() <= room)
        .unwrap_or(&forms[2]);
    let denied_style = if d > 0 { base.fg(Color::Yellow) } else { base };
    let mut spans = vec![Span::styled(
        label,
        if mismatch { base } else { label_style() },
    )];
    for (index, part) in form.iter().enumerate() {
        let style = if index == 5 { denied_style } else { base };
        spans.push(Span::styled(part.clone(), style));
    }
    if mismatch {
        spans.push(Span::styled(" MISMATCH", base.add_modifier(Modifier::BOLD)));
    }
    clip_line(Line::from(spans), width)
}

/// The three cap rows, each self-labelled (`dispatches`, `minutes`, `gap rounds`).
/// The MEDIUM tier draws exactly these (no rule, no audit row: the audit lives in the
/// status tokens); the FULL tier wraps them in [`caps_section`].
pub fn cap_rows(fm: &DriverFrontmatter, width: usize) -> Vec<Line<'static>> {
    let caps = &fm.caps;
    vec![
        cap_row(
            "dispatches",
            u64::from(caps.dispatches.used),
            caps.dispatches.limit.map(u64::from),
            None,
            width,
        ),
        cap_row(
            "minutes",
            whole_minutes(caps.wall_clock_min.used),
            caps.wall_clock_min.limit.map(whole_minutes),
            None,
            width,
        ),
        cap_row(
            "gap rounds",
            u64::from(caps.gap_rounds.used),
            caps.gap_rounds.limit.map(u64::from),
            caps.gap_rounds
                .phase
                .as_deref()
                .map(|phase| format!("phase {}", shown_capped(phase))),
            width,
        ),
    ]
}

/// Whether any cap is at or over its limit, judged on the displayed whole numbers
/// exactly as [`cap_row`] judges it (the `cap!` status token).
fn any_cap_at_limit(fm: &DriverFrontmatter) -> bool {
    let caps = &fm.caps;
    caps.dispatches
        .limit
        .is_some_and(|limit| cap_at_limit(u64::from(caps.dispatches.used), u64::from(limit)))
        || caps.wall_clock_min.limit.is_some_and(|limit| {
            cap_at_limit(
                whole_minutes(caps.wall_clock_min.used),
                whole_minutes(limit),
            )
        })
        || caps
            .gap_rounds
            .limit
            .is_some_and(|limit| cap_at_limit(u64::from(caps.gap_rounds.used), u64::from(limit)))
}

/// The caps section (FULL): its rule, the three caps, and the audit row.
pub fn caps_section(fm: &DriverFrontmatter, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![section_rule("caps", width)];
    lines.extend(cap_rows(fm, width));
    lines.push(audit_row(&fm.dispatch_audit, width));
    lines
}

// ── Connected (D-09) ────────────────────────────────────────────────────────

/// One connection row: `role (9) · state (12) · model · pid {pid} · hb {age} ·
/// {scope_unit} · {session_id first 8}`, absent fields omitted, every field through
/// `shown_capped`. No connection field decides liveness (D-09); `state` is the
/// driver's own word, shown as content.
fn connection_row(conn: &Connection, now: SystemTime, width: usize) -> Line<'static> {
    let cut = |text: &str, cells: usize| pad_cells(&fit_cells(&shown_capped(text), cells), cells);
    let mut text = format!(
        "  {} {}",
        cut(&conn.role, CONNECTED_ROLE_CELLS),
        cut(&conn.state, CONNECTED_STATE_CELLS)
    );
    let mut push = |field: String| {
        text.push_str("  ");
        text.push_str(&field);
    };
    if let Some(model) = conn.model.as_deref() {
        push(shown_capped(model));
    }
    if let Some(pid) = conn.pid {
        push(format!("pid {pid}"));
    }
    if let Some(age) = conn
        .last_heartbeat
        .as_deref()
        .and_then(|stamp| heartbeat_age_secs(stamp, now))
    {
        push(format!("hb {}", format_age(age)));
    }
    if let Some(unit) = conn.scope_unit.as_deref() {
        push(shown_capped(unit));
    }
    if let Some(session) = conn.session_id.as_deref() {
        push(shown_capped(&session.chars().take(8).collect::<String>()));
    }
    clip_line(Line::from(text), width)
}

/// The connected section: its rule, at most [`CONNECTED_MAX_ROWS`] rows then `… {k}
/// more`, or `(none recorded)` for an empty list.
pub fn connected_section(
    fm: &DriverFrontmatter,
    now: SystemTime,
    width: usize,
) -> Vec<Line<'static>> {
    connected_section_fit(fm, now, width, usize::MAX)
}

/// [`connected_section`] in at most `rows` lines (the rule counts). With room for
/// everything it is the full section; with less, `… {k} more` takes the last line and
/// the connections before it shrink to fit; with room for the rule and one line, it
/// shows the first connection alone. `rows` below 2 would be a bare rule, which the
/// tiers never allot (D-29).
pub fn connected_section_fit(
    fm: &DriverFrontmatter,
    now: SystemTime,
    width: usize,
    rows: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![section_rule("connected", width)];
    if fm.connected.is_empty() {
        lines.push(Line::from(Span::styled("  (none recorded)", muted_style())));
        return lines;
    }
    let total = fm.connected.len();
    let want = total.min(CONNECTED_MAX_ROWS);
    let demand = want + usize::from(total > want);
    let avail = rows.saturating_sub(1).max(1);
    let (shown, more) = if avail >= demand {
        (want, total > want)
    } else if avail >= 2 {
        (avail - 1, true)
    } else {
        (1, false)
    };
    for conn in fm.connected.iter().take(shown) {
        lines.push(connection_row(conn, now, width));
    }
    if more {
        lines.push(Line::from(Span::styled(
            format!("  \u{2026} {} more", total - shown),
            muted_style(),
        )));
    }
    lines
}

// ── The selected run's header (rows 1-6) ────────────────────────────────────

/// Header row 1: `glyph · detail word · run id`.
fn header_row_1(entry: &DriverRunEntry, now: SystemTime, width: usize) -> Line<'static> {
    let cell = status_cell_at(entry, now);
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
fn header_row_2(view: &DriverRunView, width: usize) -> Line<'static> {
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

/// `  {label:<7} ` — the label column of a header row; values start at column 10.
fn label_span(label: &str) -> Span<'static> {
    Span::styled(format!("  {label:<7} "), label_style())
}

/// How much of the heartbeat half to draw, widest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeartbeatForm {
    /// `heartbeat 3:10 ago (launcher start) stale (after 5:00)`.
    Full,
    /// `heartbeat 3:10 ago stale`: the tags that are context, not flags, dropped.
    Short,
    /// `hb 3:10 stale`: for the narrowest detail pane.
    Tiny,
}

/// The heartbeat half of header row 3 (D-08): `heartbeat {age} ago` with the
/// `(launcher start)` tag after a fallback stamp, and on a stale live run ` stale`
/// (Yellow, BOLD) and ` (after {threshold})` (DarkGray). A run that is not live shows
/// the age DarkGray with no judgement. No usable stamp is `heartbeat (none recorded)`.
/// The narrower forms drop the context tags and never the `stale` flag.
fn heartbeat_spans(
    view: &DriverRunView,
    now: SystemTime,
    form: HeartbeatForm,
) -> Vec<Span<'static>> {
    let fm = &view.frontmatter;
    let Some(age) = heartbeat_source(fm)
        .and_then(|(stamp, source)| heartbeat_age_secs(&stamp, now).map(|age| (age, source)))
    else {
        let text = if form == HeartbeatForm::Tiny {
            "hb (none)"
        } else {
            "heartbeat (none recorded)"
        };
        return vec![Span::styled(text, muted_style())];
    };
    let (age, source) = age;
    let live = is_live(&view.derived);
    let body = if live {
        Style::default()
    } else {
        label_style()
    };
    let lead = match form {
        HeartbeatForm::Tiny => format!("hb {}", format_age(age)),
        _ => format!("heartbeat {} ago", format_age(age)),
    };
    let mut spans = vec![Span::styled(lead, body)];
    if source == HeartbeatSource::LauncherStart && form == HeartbeatForm::Full {
        spans.push(Span::styled(" (launcher start)", label_style()));
    }
    if heartbeat_stale(fm, &view.derived, now) {
        spans.push(Span::styled(
            " stale",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        if form == HeartbeatForm::Full {
            spans.push(Span::styled(
                format!(" (after {})", format_age(fm.heartbeat_stale_after_s)),
                label_style(),
            ));
        }
    }
    spans
}

/// Header row 3: the live control word (from the body) and the heartbeat.
///
/// When the pair does not fit, the control word gives way before the heartbeat does:
/// first the heartbeat sheds its context tags (never `stale`), and the control word
/// is cut to the room left, so a flag outlives a benign word.
fn header_row_3(
    view: &DriverRunView,
    body_control: Option<&str>,
    now: SystemTime,
    width: usize,
) -> Line<'static> {
    let live = is_live(&view.derived);
    let (word, suffix) = control_shown(body_control, view.frontmatter.control.as_deref(), live);
    let suffix_text = suffix.map(|s| format!(" {s}")).unwrap_or_default();
    let control_cells = Span::raw(word.as_str()).width() + Span::raw(suffix_text.as_str()).width();
    let sep_cells = Span::raw(HEADER_SEP).width();
    let word_style = if body_control.is_none() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let forms = [
        HeartbeatForm::Full,
        HeartbeatForm::Short,
        HeartbeatForm::Tiny,
    ];
    let room_for = |heartbeat: &[Span<'static>]| {
        let cells: usize = heartbeat.iter().map(Span::width).sum();
        width.saturating_sub(HEADER_LABEL_CELLS + sep_cells + cells)
    };
    // The widest form that leaves the whole control word room; failing that, the
    // widest that leaves it six cells; failing that, the narrowest.
    let candidates: Vec<Vec<Span<'static>>> = forms
        .iter()
        .map(|form| heartbeat_spans(view, now, *form))
        .collect();
    let heartbeat = candidates
        .iter()
        .find(|hb| room_for(hb) >= control_cells)
        .or_else(|| candidates.iter().find(|hb| room_for(hb) >= 6))
        .unwrap_or(&candidates[2])
        .clone();
    let room = room_for(&heartbeat);
    let mut spans = vec![label_span("control")];
    if room >= control_cells {
        spans.push(Span::styled(word, word_style));
        if suffix.is_some() {
            spans.push(Span::styled(suffix_text, label_style()));
        }
    } else {
        spans.push(Span::styled(
            fit_cells(&format!("{word}{suffix_text}"), room),
            word_style,
        ));
    }
    spans.push(Span::styled(HEADER_SEP, label_style()));
    spans.extend(heartbeat);
    clip_line(Line::from(spans), width)
}

/// Header row 4: the sandbox posture and the prompt version. `UNSANDBOXED` (Yellow,
/// BOLD) with the `missing:` list (DarkGray) for a degraded run; a `custom:` prompt is
/// Yellow. Absent values are `(not recorded)` muted.
fn header_row_4(fm: &DriverFrontmatter, width: usize) -> Line<'static> {
    let mut spans = vec![label_span("sandbox")];
    match fm.sandbox.as_deref() {
        Some("degraded") => {
            spans.push(Span::styled(
                "UNSANDBOXED",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            if let Some(missing) = fm.sandbox_missing.as_ref().filter(|m| !m.is_empty()) {
                let list = missing
                    .iter()
                    .map(|item| shown_capped(item))
                    .collect::<Vec<_>>()
                    .join(", ");
                spans.push(Span::styled(format!(" missing: {list}"), label_style()));
            }
        }
        Some(other) => spans.push(Span::raw(shown_capped(other))),
        None => spans.push(Span::styled("(not recorded)", muted_style())),
    }
    spans.push(Span::styled(HEADER_SEP, label_style()));
    match fm.prompt_version.as_deref() {
        Some(version) => {
            let custom = version.starts_with("custom:");
            spans.push(Span::raw("prompt "));
            spans.push(Span::styled(
                shown_capped(version),
                if custom {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ));
        }
        None => spans.push(Span::styled("prompt (not recorded)", muted_style())),
    }
    clip_line(Line::from(spans), width)
}

/// Header row 5: `current_item · phase {current_phase} · {current_step}`, absent
/// parts omitted; none at all is `(no item in progress)` muted.
fn header_row_5(fm: &DriverFrontmatter, width: usize) -> Line<'static> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(item) = fm.current_item.as_deref() {
        parts.push(shown_capped(item));
    }
    if let Some(phase) = fm.current_phase.as_deref() {
        parts.push(format!("phase {}", shown_capped(phase)));
    }
    if let Some(step) = fm.current_step.as_deref() {
        parts.push(shown_capped(step));
    }
    let value = if parts.is_empty() {
        Span::styled("(no item in progress)", muted_style())
    } else {
        Span::raw(parts.join(HEADER_SEP))
    };
    clip_line(Line::from(vec![label_span("at"), value]), width)
}

/// Header row 6: `{HH:MM local} {item} {event}: {summary}`; the time DarkGray, the
/// rest default; none at all is `(no events yet)` muted.
fn header_row_6(fm: &DriverFrontmatter, width: usize) -> Line<'static> {
    let event = &fm.last_event;
    let time = event.at.as_deref().and_then(|at| {
        chrono::DateTime::parse_from_rfc3339(at)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
    });
    let head: Vec<String> = [event.item.as_deref(), event.event.as_deref()]
        .into_iter()
        .flatten()
        .map(shown_capped)
        .collect();
    let summary = event.summary.as_deref().map(shown_capped);
    if time.is_none() && head.is_empty() && summary.is_none() {
        return Line::from(vec![
            label_span("last"),
            Span::styled("(no events yet)", muted_style()),
        ]);
    }
    let mut text = head.join(" ");
    if let Some(summary) = summary {
        if text.is_empty() {
            text = summary;
        } else {
            text = format!("{text}: {summary}");
        }
    }
    let mut spans = vec![label_span("last")];
    if let Some(time) = time {
        spans.push(Span::styled(format!("{time} "), label_style()));
    }
    spans.push(Span::raw(text));
    clip_line(Line::from(spans), width)
}

/// The six header rows of the FULL tier (UI-SPEC "Header — FULL").
pub fn header_rows(
    entry: &DriverRunEntry,
    view: &DriverRunView,
    body_control: Option<&str>,
    now: SystemTime,
    width: usize,
) -> Vec<Line<'static>> {
    vec![
        header_row_1(entry, now, width),
        header_row_2(view, width),
        header_row_3(view, body_control, now, width),
        header_row_4(&view.frontmatter, width),
        header_row_5(&view.frontmatter, width),
        header_row_6(&view.frontmatter, width),
    ]
}

// ── The selected run's detail: one read of its own files (D-06, D-25) ──────

/// Everything the selected-run view needs from one read of the run's `DIRECTION.md`:
/// the frontmatter with its derived status, the live `control` word and the items
/// from the body.
///
/// Plain data (`Clone`, no handle), because it travels from a `spawn_blocking` task
/// to the app as an `Action` payload. It is display data only: the frontmatter in it
/// is as forgeable as any other (28-SECURITY), so nothing here decides liveness.
#[derive(Debug, Clone, PartialEq)]
pub struct RunDetail {
    /// The run directory name the read was made for.
    pub run_id: String,
    /// The frontmatter and the status derived from the run lock.
    pub view: DriverRunView,
    /// The live `control` word of the DIRECTION **body** (`parse_direction`,
    /// lower-cased), not the frontmatter's echo of it (D-05).
    pub body_control: Option<String>,
    /// The body's items in document order: ids, titles, `hold`, `answer`.
    pub items: Vec<BodyItem>,
}

/// Read `run_dir` once and split it into [`RunDetail`].
///
/// **Blocking file I/O (the file read and the lock probe): call it only inside
/// `spawn_blocking`** (D-25), never from a render function or a key handler. The
/// error is the reader's own display text, shown as the unreadable copy.
pub fn load_run_detail(run_dir: &Path, run_id: &str) -> Result<RunDetail, String> {
    let (view, text) = crate::state_reader::driver_frontmatter::read_driver_run_text(run_dir)
        .map_err(|error| error.to_string())?;
    let (_, body) = crate::driver::direction::split_frontmatter(&text);
    Ok(RunDetail {
        run_id: run_id.to_owned(),
        view,
        body_control: crate::driver::direction::parse_direction(body).control,
        items: crate::driver::steer::read_items(body),
    })
}

/// How an item row relates the two sources (D-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMark {
    /// In the body, and the driver recorded this state for it.
    Seen(ItemState),
    /// In the body only: the driver has not recorded a state for it.
    Unseen,
    /// In the frontmatter only: the driver knows it, the body no longer lists it.
    NotInDirection(ItemState),
}

/// One row of the merged item list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRow {
    pub id: String,
    /// The `### I-NN` heading text; `None` for a frontmatter-only item.
    pub title: Option<String>,
    pub mark: ItemMark,
    /// Dispatches the frontmatter counted for this item (0 when unseen).
    pub dispatches: u32,
    /// The body says `- hold: true`.
    pub hold: bool,
    /// The frontmatter's `outcome` for this item.
    pub outcome: Option<String>,
    /// The body's non-empty `- answer:`.
    pub answer: Option<String>,
}

/// Merge the body's items with the frontmatter's `item_states` (D-06): body items in
/// body order, then frontmatter-only items in `item_states` order. An id in both is
/// one row. A repeated body id keeps its first heading (the lint flags the repeat).
pub fn merged_items(detail: &RunDetail) -> Vec<ItemRow> {
    let states = &detail.view.frontmatter.item_states.0;
    let state_of = |id: &str| states.iter().find(|(key, _)| key == id).map(|(_, e)| e);
    let mut rows: Vec<ItemRow> = Vec::with_capacity(detail.items.len());
    for item in &detail.items {
        if rows.iter().any(|row| row.id == item.id) {
            continue;
        }
        let entry = state_of(&item.id);
        rows.push(ItemRow {
            id: item.id.clone(),
            title: Some(item.title.clone()),
            mark: entry.map_or(ItemMark::Unseen, |e| ItemMark::Seen(e.state)),
            dispatches: entry.map_or(0, |e| e.dispatches),
            hold: item.is_on_hold(),
            outcome: entry.and_then(|e| e.outcome.clone()),
            answer: item.answer.clone(),
        });
    }
    for (id, entry) in states {
        if rows.iter().any(|row| &row.id == id) {
            continue;
        }
        rows.push(ItemRow {
            id: id.clone(),
            title: None,
            mark: ItemMark::NotInDirection(entry.state),
            dispatches: entry.dispatches,
            hold: false,
            outcome: entry.outcome.clone(),
            answer: None,
        });
    }
    rows
}

impl ItemRow {
    /// The driver recorded this item as parked.
    pub fn is_parked(&self) -> bool {
        matches!(
            self.mark,
            ItemMark::Seen(ItemState::Parked) | ItemMark::NotInDirection(ItemState::Parked)
        )
    }

    /// The item has a `### I-NN` heading in the body, so a steering edit has a segment
    /// to write into. A frontmatter-only item has none.
    pub fn in_body(&self) -> bool {
        !matches!(self.mark, ItemMark::NotInDirection(_))
    }

    /// `a` has something to answer: the item is parked or on hold.
    pub fn answerable(&self) -> bool {
        self.is_parked() || self.hold
    }
}

/// Where the item cursor starts on selecting a run: the first `parked` item, else
/// `current_item` when it is listed, else the first row (UI-SPEC, D-06).
pub fn item_cursor_default(rows: &[ItemRow], current_item: Option<&str>) -> Option<String> {
    let parked = |mark: ItemMark| {
        matches!(
            mark,
            ItemMark::Seen(ItemState::Parked) | ItemMark::NotInDirection(ItemState::Parked)
        )
    };
    rows.iter()
        .find(|row| parked(row.mark))
        .or_else(|| current_item.and_then(|id| rows.iter().find(|row| row.id == id)))
        .or_else(|| rows.first())
        .map(|row| row.id.clone())
}

/// The item cursor id to use: `cursor` when it is still listed, else the default.
pub fn resolved_item_cursor(
    rows: &[ItemRow],
    cursor: Option<&str>,
    current_item: Option<&str>,
) -> Option<String> {
    cursor
        .filter(|id| rows.iter().any(|row| row.id == *id))
        .map(str::to_owned)
        .or_else(|| item_cursor_default(rows, current_item))
}

/// The detail of the run `run_id`, when the cache holds one **for that run**. A
/// detail left over from the previously selected run is never shown under another.
pub fn detail_for<'a>(cache: Option<&'a ProjectViewCache>, run_id: &str) -> Option<&'a RunDetail> {
    cache
        .and_then(|c| c.launcher_detail.as_deref())
        .filter(|detail| detail.run_id == run_id)
}

/// The index the selection lands on after `runs` replaced the listing: the row of
/// the pinned run id when it is still listed, else the old index clamped into range
/// (0 for an empty list). The one rule both the off-thread task (to choose which
/// run's detail to read) and the app handler (to move the selection) apply, so the
/// detail read is always for the run the handler then selects.
pub fn pinned_index(runs: &[DriverRunEntry], pinned_id: Option<&str>, old_index: usize) -> usize {
    pinned_id
        .and_then(|id| runs.iter().position(|(run_id, _)| run_id == id))
        .unwrap_or_else(|| old_index.min(runs.len().saturating_sub(1)))
}

/// Keep the item cursor on a listed item of the cached detail: when the stored id is
/// not one of its rows, pin the default (first parked, else `current_item`, else the
/// first row). Called after the detail or the selection changed; pure.
pub fn settle_item_cursor(cache: &mut ProjectViewCache) {
    let Some(detail) = cache.launcher_detail.as_deref() else {
        return;
    };
    let rows = merged_items(detail);
    let listed = cache
        .launcher_item_cursor
        .as_deref()
        .is_some_and(|id| rows.iter().any(|row| row.id == id));
    if !listed {
        let current = detail.view.frontmatter.current_item.as_deref();
        cache.launcher_item_cursor = item_cursor_default(&rows, current);
    }
}

/// Store `detail` as the selected run's when it is for the selected run; a result
/// for any other run is dropped. A cached detail that is not the selected run's is
/// cleared either way, so a run's items are never drawn under another run.
pub fn store_detail(cache: &mut ProjectViewCache, detail: Option<Box<RunDetail>>) {
    let selected = cache.launcher_selected_id.clone();
    if let Some(detail) = detail.filter(|d| Some(&d.run_id) == selected.as_ref()) {
        cache.launcher_detail = Some(detail);
    } else if cache
        .launcher_detail
        .as_deref()
        .is_some_and(|d| Some(&d.run_id) != selected.as_ref())
    {
        cache.launcher_detail = None;
    }
    settle_item_cursor(cache);
}

/// Move the item cursor of the selected launcher run by `delta` rows, clamped at
/// both ends. Inert (returns `false`) with no items or no detail for the selected
/// run. Pure: it touches the cache and nothing else.
pub fn move_item_cursor(cache: &mut ProjectViewCache, delta: isize) -> bool {
    let Selection::Launcher(index) = selection(Some(cache)) else {
        return false;
    };
    let run_id = cache.launcher_runs[index].0.clone();
    let Some(detail) = detail_for(Some(cache), &run_id) else {
        return false;
    };
    let rows = merged_items(detail);
    if rows.is_empty() {
        return false;
    }
    let current = detail.view.frontmatter.current_item.clone();
    let at = resolved_item_cursor(
        &rows,
        cache.launcher_item_cursor.as_deref(),
        current.as_deref(),
    )
    .and_then(|id| rows.iter().position(|row| row.id == id))
    .unwrap_or(0);
    let next = if delta < 0 {
        at.saturating_sub(delta.unsigned_abs())
    } else {
        at.saturating_add(delta.unsigned_abs())
    }
    .min(rows.len() - 1);
    cache.launcher_item_cursor = Some(rows[next].id.clone());
    true
}

// ── The items section (D-06) ────────────────────────────────────────────────

/// A section rule: `── {name} ───…`, DarkGray, padded to `width` cells.
fn section_rule(name: &str, width: usize) -> Line<'static> {
    let lead = format!("\u{2500}\u{2500} {name} ");
    let fill = width.saturating_sub(Span::raw(lead.as_str()).width());
    Line::from(Span::styled(
        format!("{lead}{}", "\u{2500}".repeat(fill)),
        label_style(),
    ))
}

/// `text` padded with spaces to `cells` display cells (never cut).
fn pad_cells(text: &str, cells: usize) -> String {
    let pad = cells.saturating_sub(Span::raw(text).width());
    format!("{text}{}", " ".repeat(pad))
}

/// The glyph, word and colour of an item mark. Fixed constants and the closed
/// `ItemState` enum only: nothing a file wrote reaches a glyph or a colour.
fn item_look(mark: ItemMark) -> (&'static str, &'static str, Color) {
    let state = match mark {
        ItemMark::Unseen => return (GLYPH_ITEM_UNSEEN, "unseen", Color::DarkGray),
        ItemMark::Seen(state) | ItemMark::NotInDirection(state) => state,
    };
    match state {
        ItemState::Pending => (GLYPH_RUN_UNLAUNCHED, "pending", Color::DarkGray),
        ItemState::Running => (GLYPH_RUN_LIVE, "running", Color::Magenta),
        ItemState::Done => (GLYPH_RUN_DONE, "done", Color::Green),
        ItemState::Parked => (GLYPH_RUN_PARKED, "parked", Color::Red),
        ItemState::Failed => (GLYPH_RUN_CRASHED, "failed", Color::Red),
    }
}

/// One item row (UI-SPEC D-06):
/// `cursor(2) · id(5) · glyph · word(8) · "{n}d"(3) · gap · hold(4) · gap(2) · title`.
///
/// The title is sanitised first (`shown_capped`) and cut by display width second
/// (`fit_cells`) to the room measured from the spans already placed, never to a
/// fixed number: when `room > 1` it is cut to `room - 1`, and when `room <= 1` the
/// title span is dropped entirely, so an orphan `…` cannot appear. A
/// frontmatter-only item carries ` [not in DIRECTION]` in DarkGray after its title.
pub fn item_row(row: &ItemRow, selected: bool, width: usize) -> Line<'static> {
    let (glyph, word, color) = item_look(row.mark);
    let state_style = Style::default().fg(color);
    let id_style = if selected {
        selection_style()
    } else {
        Style::default()
    };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(if selected { "> " } else { "  " }, selection_style()),
        Span::styled(
            pad_cells(
                &fit_cells(&shown_capped(&row.id), ITEM_ID_CELLS),
                ITEM_ID_CELLS,
            ),
            id_style,
        ),
        Span::styled(glyph, state_style),
        Span::raw(" "),
        Span::styled(pad_cells(word, ITEM_WORD_CELLS), state_style),
        Span::raw(format!("{:>3}", format!("{}d", row.dispatches))),
        Span::raw(" "),
        if row.hold {
            Span::styled("hold", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("    ")
        },
        Span::raw("  "),
    ];
    let used: usize = spans.iter().map(Span::width).sum();
    let foreign = matches!(row.mark, ItemMark::NotInDirection(_));
    let marker_cells = if foreign { NOT_IN_DIRECTION.len() } else { 0 };
    let title = row
        .title
        .as_deref()
        .map(shown_capped)
        .filter(|title| !title.is_empty());
    // A separating space only when a title precedes the marker.
    let room = width.saturating_sub(used + marker_cells + usize::from(foreign));
    let mut placed_title = false;
    if let Some(title) = title {
        if room > 1 {
            spans.push(Span::raw(fit_cells(&title, room - 1)));
            placed_title = true;
        }
    }
    if foreign {
        if placed_title {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(NOT_IN_DIRECTION, label_style()));
    }
    Line::from(spans)
}

/// The cursor-item row under the item rows: `{id} {state}: {outcome}` plus
/// `  answer: {answer}` when the body has one, else `{id}: no outcome recorded`
/// muted (UI-SPEC). Task 3 of plan 30-02 adds the park rule.
fn cursor_item_line(row: &ItemRow, park_rule: Option<&str>, width: usize) -> Line<'static> {
    let (_, word, color) = item_look(row.mark);
    let id = shown_capped(&row.id);
    let mut spans: Vec<Span<'static>> = Vec::new();
    match row
        .outcome
        .as_deref()
        .filter(|o| crate::text::carries_visible_content(o))
    {
        Some(outcome) => {
            let by = park_rule
                .filter(|_| {
                    matches!(
                        row.mark,
                        ItemMark::Seen(ItemState::Parked)
                            | ItemMark::NotInDirection(ItemState::Parked)
                    )
                })
                .map(|rule| format!(" by {}", shown_capped(rule)))
                .unwrap_or_default();
            spans.push(Span::raw(format!("  {id} ")));
            spans.push(Span::styled(
                format!("{word}{by}"),
                Style::default().fg(color),
            ));
            spans.push(Span::raw(format!(": {}", shown_capped(outcome))));
        }
        None => {
            spans.push(Span::styled(
                format!("  {id}: no outcome recorded"),
                muted_style(),
            ));
        }
    }
    if let Some(answer) = row.answer.as_deref() {
        spans.push(Span::raw(format!("  answer: {}", shown_capped(answer))));
    }
    let line = Line::from(spans);
    clip_line(line, width)
}

/// `line` cut to `width` display cells, the cut marked with `…`. Spans keep their
/// styles; text is only ever dropped from the right.
fn clip_line(line: Line<'static>, width: usize) -> Line<'static> {
    if line.width() <= width {
        return line;
    }
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len());
    let mut left = width;
    for span in line.spans {
        let w = span.width();
        if w <= left {
            left -= w;
            spans.push(span);
            continue;
        }
        spans.push(Span::styled(fit_cells(&span.content, left), span.style));
        break;
    }
    Line::from(spans)
}

/// The lines of the items section for the selected run: its rule, the item rows
/// (at most [`ITEMS_MAX_ROWS`], the window keeping the cursor visible, then `… {k}
/// more items`), and the cursor-item row.
///
/// With no detail yet the section reads `Loading...`; with no items in either
/// source, `No items in DIRECTION.md yet. Add one with [e].`, so the section is
/// never header-only (D-29).
pub fn items_section(
    detail: Option<&RunDetail>,
    cursor: Option<&str>,
    park_rules: Option<&std::collections::BTreeMap<String, String>>,
    width: usize,
) -> Vec<Line<'static>> {
    items_section_fit(detail, cursor, park_rules, width, usize::MAX, true)
}

/// [`items_section`] in at most `rows` lines (the rule counts), with or without the
/// trailing cursor-item row (FULL draws it, MEDIUM does not).
///
/// With room for every item row (at most [`ITEMS_MAX_ROWS`]) and the `… {k} more
/// items` row, that is the whole section. With less, the last item line becomes `…
/// {k} more items` and the item rows before it shrink to fit, the window keeping the
/// cursor on screen; with room for one line only, it is the cursor's own row. A
/// section is never header-only: the rule always has at least one line under it, and
/// the callers never allot fewer than two rows (D-29).
pub fn items_section_fit(
    detail: Option<&RunDetail>,
    cursor: Option<&str>,
    park_rules: Option<&std::collections::BTreeMap<String, String>>,
    width: usize,
    rows_budget: usize,
    cursor_row: bool,
) -> Vec<Line<'static>> {
    let mut lines = vec![section_rule("items", width)];
    let Some(detail) = detail else {
        lines.push(Line::from(Span::styled(
            format!("  {DRY_RUN_LOADING}"),
            muted_style(),
        )));
        return lines;
    };
    let rows = merged_items(detail);
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No items in DIRECTION.md yet. Add one with [e].",
            muted_style(),
        )));
        return lines;
    }
    let current = detail.view.frontmatter.current_item.as_deref();
    let at = resolved_item_cursor(&rows, cursor, current)
        .and_then(|id| rows.iter().position(|row| row.id == id))
        .unwrap_or(0);
    let want = rows.len().min(ITEMS_MAX_ROWS);
    let demand = want + usize::from(rows.len() > want);
    // The lines under the rule that the item rows (and the `more` row) may use.
    let avail = rows_budget
        .saturating_sub(1 + usize::from(cursor_row))
        .max(1);
    let (shown, more) = if avail >= demand {
        (want, rows.len() > want)
    } else if avail >= 2 {
        (avail - 1, true)
    } else {
        (1, false)
    };
    // The smallest window that keeps the cursor row on screen.
    let start = (at + 1).saturating_sub(shown);
    for (offset, row) in rows[start..start + shown].iter().enumerate() {
        lines.push(item_row(row, start + offset == at, width));
    }
    if more {
        lines.push(Line::from(Span::styled(
            format!("  \u{2026} {} more items", rows.len() - shown),
            muted_style(),
        )));
    }
    if cursor_row {
        let rule = park_rules
            .and_then(|rules| rules.get(&rows[at].id))
            .map(String::as_str);
        lines.push(cursor_item_line(&rows[at], rule, width));
    }
    lines
}

// ── The LOG.md tail (D-23, D-16, D-25) ─────────────────────────────────────

/// The `rule=` values the LOG template documents for a `status state=parked` event.
/// A park rule is recorded only from this closed set (T-30-06): anything else a
/// log line claims is ignored, never shown as a reason.
pub const PARK_RULES: [&str; 8] = [
    "budget",
    "gap-rounds",
    "divergence",
    "no-progress",
    "needs-human",
    "scope",
    "run-budget",
    "launcher-cap",
];

/// How many park rules one tail remembers. A log is tool-written, but a hostile run
/// directory could name any number of items; the map stays bounded (T-30-07).
const PARK_RULES_MAX: usize = 256;

/// How many `tail_file` calls one refresh makes: each reads at most
/// `MAX_TAIL_BYTES` (4 MiB), so a refresh reads at most 16 MiB (T-30-07).
const LOG_READS_PER_REFRESH: usize = 4;

/// The name of the log file inside a run directory. The pane reads this file and
/// nothing else: not the stream-json transcript, not the launcher's own log (D-23).
const LOG_FILE_NAME: &str = "LOG.md";

/// What one blocking read of `LOG.md` found: plain data carried by the refresh
/// actions to the app (D-25).
#[derive(Debug, Clone, PartialEq)]
pub struct LogRead {
    /// The run the file belongs to.
    pub run_id: String,
    /// The cursor offset this read **started from**, as the cache held it when the
    /// read was scheduled. The app applies a result only if the cache's cursor is
    /// still exactly here, so a duplicated or stale read shows no line twice.
    pub from_offset: u64,
    /// Where the next read starts.
    pub cursor: TailCursor,
    /// `(dev, ino)` of the file read (unix; `None` elsewhere).
    pub ident: Option<(u64, u64)>,
    /// The complete new lines, the template preamble already dropped.
    pub lines: Vec<String>,
    /// The file was truncated, or replaced by another file: the ring starts over.
    pub reset: bool,
    /// The run has no `LOG.md`.
    pub missing: bool,
}

/// The cursor and file identity to read from for `run_id`: the tail's own when it
/// is the tail of that run, else the start of the file. The one rule the blocking
/// tasks apply, so a read for a newly selected run can never inherit another run's
/// offset.
pub fn log_cursor_for(cache: &ProjectViewCache, run_id: &str) -> (TailCursor, Option<(u64, u64)>) {
    let tail = &cache.launcher_log;
    if tail.run_id == run_id {
        (tail.cursor, tail.ident)
    } else {
        (TailCursor::default(), None)
    }
}

/// Read what was appended to `<run_dir>/LOG.md` since `cursor` (D-23).
///
/// **Blocking file I/O: call it only inside `spawn_blocking`** (D-25). It reads the
/// log by byte offset, so an append costs only the new bytes. A file that is gone
/// is `missing`; a file shorter than the cursor (`restarted`) or whose `(dev, ino)`
/// differs from the known `ident` (replaced) sets `reset` and is read from the
/// start. On a read from offset 0 the template preamble is dropped by discarding
/// lines until the first one `driverctl::log::parse_log` recognises as an event
/// header. `LOG.md` is opened once with `open_run_file` (no link followed, no FIFO
/// waited on); the type check, the `(dev, ino)` and every tail read use that one
/// handle, so a path that is not a regular file is unreadable and nothing is
/// re-opened by name after a check.
///
/// The `(dev, ino)` read is `#[cfg(unix)]`; elsewhere `ident` is `None` and only
/// truncation is detected.
pub fn read_log_delta(
    run_dir: &Path,
    run_id: &str,
    cursor: TailCursor,
    ident: Option<(u64, u64)>,
) -> LogRead {
    let path = run_dir.join(LOG_FILE_NAME);
    let mut read = LogRead {
        run_id: run_id.to_owned(),
        from_offset: cursor.offset,
        cursor,
        ident,
        lines: Vec::new(),
        reset: false,
        missing: false,
    };
    // One open, vetted on the handle (T-30-23, T-30-24): `open_run_file` refuses a
    // link (`O_NOFOLLOW`), never blocks on a FIFO (`O_NONBLOCK`) and checks the type
    // of what it opened. A missing log is `missing`; any other refusal leaves the
    // read untouched.
    let mut file = match crate::state_reader::driver_frontmatter::open_run_file(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            read.missing = true;
            return read;
        }
        Err(_) => return read,
    };
    #[cfg(unix)]
    let now_ident = {
        use std::os::unix::fs::MetadataExt;
        let Ok(meta) = file.metadata() else {
            return read;
        };
        Some((meta.dev(), meta.ino()))
    };
    #[cfg(not(unix))]
    let now_ident: Option<(u64, u64)> = None;

    let mut cursor = cursor;
    if ident.is_some_and(|known| Some(known) != now_ident) {
        cursor = TailCursor::default();
        read.reset = true;
    }
    read.ident = now_ident;

    let mut lines: Vec<String> = Vec::new();
    let mut from_start = cursor.offset == 0;
    for _ in 0..LOG_READS_PER_REFRESH {
        let Ok(batch) = tail_file(&mut file, cursor) else {
            break;
        };
        if batch.restarted {
            // Whatever was gathered came from the file as it was before it shrank.
            lines.clear();
            read.reset = true;
            from_start = true;
        }
        cursor = batch.cursor;
        let progressed = !batch.lines.is_empty() || batch.skipped_oversize;
        lines.extend(batch.lines);
        if !progressed {
            break;
        }
    }
    if from_start {
        // The template preamble: everything before the first event header.
        let first = lines
            .iter()
            .position(|line| !crate::driverctl::log::parse_log(line).is_empty())
            .unwrap_or(lines.len());
        lines.drain(..first);
    }
    read.cursor = cursor;
    read.lines = lines;
    read
}

/// Whether `item` is plausibly an item id from the log (`I-NN`), bounded so a
/// hostile log cannot grow the park-rule map with arbitrary keys.
fn is_item_key(item: &str) -> bool {
    item.len() <= 32
        && item.starts_with("I-")
        && item.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Append one log line to the tail's ring through `push_record`, the only
/// sanitising append, and record a park rule when the line is a `status` event
/// with `state=parked` and a `rule` from [`PARK_RULES`].
///
/// An event header is rewritten as `{HH:MM:SS local} {item} {event} {k=v ...}` and
/// pushed as [`DriverLineKind::Output`]; any other line (a worker report's detail
/// line) as [`DriverLineKind::Stderr`] with its own four-space indent removed. No
/// colour or glyph is ever chosen from the text: the kinds only say header or not.
fn push_log_line(tail: &mut LauncherLogTail, line: &str) {
    let Some(event) = crate::driverctl::log::parse_log(line).into_iter().next() else {
        tail.ring.push_record(
            DriverLineKind::Stderr,
            line.strip_prefix("    ").unwrap_or(line),
        );
        return;
    };
    if event.event == "status" && event.get("state") == Some("parked") && is_item_key(&event.item) {
        if let Some(rule) = event.get("rule").filter(|rule| PARK_RULES.contains(rule)) {
            if tail.park_rules.len() < PARK_RULES_MAX || tail.park_rules.contains_key(&event.item) {
                tail.park_rules.insert(event.item.clone(), rule.to_owned());
            }
        }
    }
    let time = chrono::DateTime::parse_from_rfc3339(&event.at)
        .map(|at| {
            at.with_timezone(&chrono::Local)
                .format("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| "--:--:--".to_owned());
    let mut text = format!("{time} {} {}", event.item, event.event);
    for (key, value) in &event.kv {
        text.push_str(&format!(" {key}={value}"));
    }
    tail.ring.push_record(DriverLineKind::Output, &text);
}

/// Apply one off-thread read to the cache. Returns whether it was applied.
///
/// A read is dropped unless its run is the selected launcher run **and** it started
/// where the cache's cursor now stands (a read for a run the tail has not seen
/// starts a fresh tail, and must itself have started at 0). A `reset` replaces the
/// ring and sets the notice. Selecting another run starts the new run's tail with
/// follow on and the scroll at the tail (D-23).
pub fn apply_log_read(cache: &mut ProjectViewCache, read: LogRead) -> bool {
    if cache.launcher_selected_id.as_deref() != Some(read.run_id.as_str()) {
        return false;
    }
    if cache.launcher_log.run_id == read.run_id {
        if read.from_offset != cache.launcher_log.cursor.offset {
            return false;
        }
    } else {
        if read.from_offset != 0 {
            return false;
        }
        cache.launcher_log = LauncherLogTail {
            run_id: read.run_id.clone(),
            ring: DriverOutput::for_run(&read.run_id),
            ..LauncherLogTail::default()
        };
        cache.driver_follow = true;
        cache.driver_scroll_offset = 0;
    }
    let tail = &mut cache.launcher_log;
    if read.reset {
        tail.ring = DriverOutput::for_run(&read.run_id);
        tail.park_rules.clear();
        tail.reset_notice = true;
    }
    tail.cursor = read.cursor;
    tail.ident = read.ident;
    tail.missing = read.missing;
    for line in &read.lines {
        push_log_line(tail, line);
    }
    true
}

/// The log body: the reset notice, the ring-overflow notice, then the ring's lines
/// (or the empty-state copy). A header line is a DarkGray `HH:MM:SS` then the rest
/// in the default style; any other line is indented 4 cells and DarkGray. The
/// ring's text reaches a `Span` only through [`crate::text::Untrusted::shown`].
fn log_body_lines(tail: &LauncherLogTail) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    if tail.reset_notice {
        lines.push(Line::from(Span::styled(
            "! LOG.md was truncated or replaced, reading it from the start",
            Style::default().fg(Color::Yellow),
        )));
    }
    if tail.ring.dropped() > 0 {
        lines.push(Line::from(Span::styled(
            ring_overflow_notice(tail.ring.dropped()),
            muted_style(),
        )));
    }
    if tail.ring.is_empty() {
        let copy = if tail.missing {
            "This run has no LOG.md."
        } else {
            "No events logged yet."
        };
        lines.push(Line::from(Span::styled(format!("  {copy}"), muted_style())));
        return lines;
    }
    for line in tail.ring.lines() {
        let text: String = line.text.shown().into();
        match line.kind {
            DriverLineKind::Output => {
                // `HH:MM:SS` is 8 ASCII cells written by `push_log_line`.
                let split = text.char_indices().nth(8).map_or(text.len(), |(at, _)| at);
                let (time, rest) = text.split_at(split);
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(time.to_owned(), label_style()),
                    Span::raw(rest.to_owned()),
                ]));
            }
            _ => lines.push(Line::from(Span::styled(
                format!("      {text}"),
                label_style(),
            ))),
        }
    }
    lines
}

/// Draw the LOG pane into `area` (its rule row, then the scrolling body).
///
/// The viewer is the legacy output pane's shape: the whole `Vec<Line>` is built,
/// `total_lines` taken from it, the metrics recorded into `viewport` during the
/// render pass, and `Paragraph::scroll` applied with **no `Wrap`**. The follow bit
/// resolves here through the one shared [`clamp_scroll`]/[`tail_offset`]; the keys
/// (`PgUp`/`PgDn`/`f`/`G`) read the same `driver_scroll_offset`, `driver_follow` and
/// viewport the legacy pane uses, so the UIFIX-04 clamp order is not re-implemented.
///
/// The indicator is `[following]` (Green) or `[scrolled +N]` (Yellow, capped) for a
/// live run and `[not live]` (DarkGray) otherwise. It never says `paused`.
fn render_log_section(
    frame: &mut Frame,
    area: Rect,
    cache: Option<&ProjectViewCache>,
    run_id: &str,
    live: bool,
    viewport: &Cell<ViewportMetrics>,
) {
    if area.height == 0 {
        return;
    }
    let rule_area = Rect { height: 1, ..area };
    let body_area = Rect {
        y: area.y + 1,
        height: area.height - 1,
        ..area
    };
    // A tail left over from another run is never drawn under this one.
    let tail = cache
        .map(|c| &c.launcher_log)
        .filter(|tail| tail.run_id == run_id);
    let body = match tail {
        Some(tail) => log_body_lines(tail),
        None => vec![Line::from(Span::styled(
            format!("  {DRY_RUN_LOADING}"),
            muted_style(),
        ))],
    };
    let total_lines = u16::try_from(body.len()).unwrap_or(u16::MAX);
    let metrics = ViewportMetrics {
        total_lines,
        visible_height: body_area.height,
    };
    viewport.set(metrics);
    let following = cache.is_some_and(|c| c.driver_follow);
    let requested = if following {
        u16::MAX
    } else {
        cache.map_or(0, |c| c.driver_scroll_offset)
    };
    let scroll = clamp_scroll(requested, total_lines, body_area.height);
    let below = usize::from(tail_offset(metrics).saturating_sub(scroll));
    let (indicator, color) = if live {
        follow_indicator(true, following, below, None, false)
    } else {
        ("[not live]".to_owned(), Color::DarkGray)
    };
    frame.render_widget(
        Paragraph::new(pane_header_line("log", rule_area.width, &indicator, color)),
        rule_area,
    );
    if body_area.height > 0 {
        frame.render_widget(Paragraph::new(body).scroll((scroll, 0)), body_area);
    }
}

// ── Status tokens (D-10, shared by MEDIUM header row 2 and the SUMMARY rows) ──

/// One status token: its spans, and whether it is a flag (something the operator
/// should not miss) rather than a benign fact.
struct Token {
    spans: Vec<Span<'static>>,
    flag: bool,
}

impl Token {
    fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }
}

/// The separator between two status tokens.
const TOKEN_SEP: &str = "  ";

fn flag_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Every status token a readable run has, in the UI-SPEC order:
///
/// 1. `ctl {word}` (+ ` (pending)` / ` (at next launch)`)
/// 2. `hb {age}` (+ ` stale`, Yellow BOLD)
/// 3. `audit {l}/{o}/{d} MISMATCH` (Red, `MISMATCH` BOLD)
/// 4. `UNSANDBOXED` (Yellow BOLD)
/// 5. `cap!` (Yellow BOLD)
/// 6. `audit {l}/{o}/{d}` (DarkGray)
/// 7. `prompt custom` (Yellow)
///
/// `detail` is the cached read of THIS run (the caller passes it only when its run id
/// matches), whose body gives the live `control` word. Without it the frontmatter's
/// echo is all there is, so a run with neither shows no `ctl` token rather than a
/// claim about a body nobody read `[inferred — audit]`. An unreadable run has none.
fn status_token_list(
    entry: &DriverRunEntry,
    detail: Option<&RunDetail>,
    now: SystemTime,
) -> Vec<Token> {
    let Ok(view) = &entry.1 else {
        return Vec::new();
    };
    let fm = &view.frontmatter;
    let live = is_live(&view.derived);
    let mut tokens: Vec<Token> = Vec::new();

    // 1. control.
    let control = match detail {
        Some(detail) => Some(control_shown(
            detail.body_control.as_deref(),
            fm.control.as_deref(),
            live,
        )),
        None => fm
            .control
            .as_deref()
            .map(|echo| control_shown(Some(echo), Some(echo), live)),
    };
    if let Some((word, suffix)) = control {
        let unlined = detail.is_some_and(|d| d.body_control.is_none());
        let mut spans = vec![Span::styled(
            format!("ctl {word}"),
            if unlined {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            },
        )];
        if let Some(suffix) = suffix {
            spans.push(Span::styled(format!(" {suffix}"), label_style()));
        }
        tokens.push(Token { spans, flag: false });
    }

    // 2. heartbeat age, with the stale flag.
    if live {
        if let Some(age) =
            heartbeat_source(fm).and_then(|(stamp, _)| heartbeat_age_secs(&stamp, now))
        {
            let stale = heartbeat_stale(fm, &view.derived, now);
            let mut spans = vec![Span::raw(format!("hb {}", format_age(age)))];
            if stale {
                spans.push(Span::styled(" stale", flag_style()));
            }
            tokens.push(Token { spans, flag: stale });
        }
    }

    // 3 / 6. the audit: a mismatch is a flag, a match is a benign fact.
    let audit = &fm.dispatch_audit;
    let counts = format!("{}/{}/{}", audit.logged, audit.observed, audit.denied);
    let mismatch = audit.logged != audit.observed;
    if mismatch {
        tokens.push(Token {
            spans: vec![
                Span::styled(format!("audit {counts} "), Style::default().fg(Color::Red)),
                Span::styled(
                    "MISMATCH",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ],
            flag: true,
        });
    }

    // 4. the sandbox posture.
    if fm.sandbox.as_deref() == Some("degraded") {
        tokens.push(Token {
            spans: vec![Span::styled("UNSANDBOXED", flag_style())],
            flag: true,
        });
    }

    // 5. a cap at or over its limit.
    if any_cap_at_limit(fm) {
        tokens.push(Token {
            spans: vec![Span::styled("cap!", flag_style())],
            flag: true,
        });
    }

    if !mismatch {
        tokens.push(Token {
            spans: vec![Span::styled(format!("audit {counts}"), label_style())],
            flag: false,
        });
    }

    // 7. an operator-supplied prompt.
    if fm
        .prompt_version
        .as_deref()
        .is_some_and(|version| version.starts_with("custom:"))
    {
        tokens.push(Token {
            spans: vec![Span::styled(
                "prompt custom",
                Style::default().fg(Color::Yellow),
            )],
            flag: false,
        });
    }
    tokens
}

/// The status tokens that fit `cells` whole, joined by two spaces, never split.
///
/// When they do not all fit, benign tokens are dropped first and flag tokens last,
/// each group from the end of the UI-SPEC order, so `MISMATCH`, `UNSANDBOXED`, `cap!`
/// and a stale heartbeat outlive `audit {l}/{o}/{d}`, `prompt custom`, a fresh
/// heartbeat and even the `ctl` token that heads the order `[inferred — audit]` (the
/// UI-SPEC says both "dropped from the end" and "a flag always outlives a benign one",
/// and only this reading satisfies both). With room for none of them, nothing is drawn.
pub fn status_tokens(
    entry: &DriverRunEntry,
    detail: Option<&RunDetail>,
    now: SystemTime,
    cells: usize,
) -> Vec<Span<'static>> {
    let tokens = status_token_list(entry, detail, now);
    let mut keep = vec![true; tokens.len()];
    let used = |keep: &[bool]| -> usize {
        let kept: Vec<usize> = (0..tokens.len()).filter(|i| keep[*i]).collect();
        kept.iter().map(|i| tokens[*i].width()).sum::<usize>()
            + Span::raw(TOKEN_SEP).width() * kept.len().saturating_sub(1)
    };
    while used(&keep) > cells {
        let victim = (0..tokens.len())
            .rev()
            .find(|i| keep[*i] && !tokens[*i].flag)
            .or_else(|| (0..tokens.len()).rev().find(|i| keep[*i]));
        match victim {
            Some(index) => keep[index] = false,
            None => break,
        }
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, token) in tokens.into_iter().enumerate() {
        if !keep[index] {
            continue;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(TOKEN_SEP));
        }
        spans.extend(token.spans);
    }
    spans
}

/// MEDIUM header row 2: the status tokens behind a two-cell indent.
fn token_row(
    entry: &DriverRunEntry,
    detail: Option<&RunDetail>,
    now: SystemTime,
    width: usize,
) -> Line<'static> {
    let mut spans = vec![Span::raw("  ")];
    spans.extend(status_tokens(entry, detail, now, width.saturating_sub(2)));
    Line::from(spans)
}

// ── SUMMARY rows (D-10) ─────────────────────────────────────────────────────

/// Cells the run id keeps before the status tokens may take the rest of a SUMMARY row.
const SUMMARY_ID_MIN_CELLS: usize = 12;

/// One SUMMARY row, `> ◆ running    20261020-driver-tab  ctl pause (pending)  hb 3:10
/// stale`: highlight, glyph, word padded to [`RUN_WORD_CELLS`], the run id cut by
/// display width, then the status tokens that fit whole.
///
/// The run id yields to the tokens down to [`SUMMARY_ID_MIN_CELLS`] cells, so a flag
/// token is not pushed off a narrow row by a long id. An unreadable run reads
/// `unreadable` with no tokens. Everything untrusted goes through `shown_capped`
/// first and `fit_cells` second.
pub fn summary_row(
    entry: &DriverRunEntry,
    detail: Option<&RunDetail>,
    selected: bool,
    now: SystemTime,
    width: usize,
) -> Line<'static> {
    let cell = status_cell_at(entry, now);
    let (prefix, id_style) = if selected {
        ("> ", selection_style())
    } else {
        ("  ", Style::default())
    };
    let word = pad_cells(&fit_cells(&cell.list_word, RUN_WORD_CELLS), RUN_WORD_CELLS);
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(prefix, selection_style()),
        Span::styled(cell.glyph, cell.style),
        Span::raw(" "),
        Span::styled(word, cell.style),
        Span::raw(" "),
    ];
    let used: usize = spans.iter().map(Span::width).sum();
    let room = width.saturating_sub(used);
    let id = shown_capped(&entry.0);
    let id_cells = Span::raw(id.as_str()).width();
    let sep = Span::raw(TOKEN_SEP).width();

    let all = status_tokens(entry, detail, now, usize::MAX);
    let all_cells: usize = all.iter().map(Span::width).sum();
    let token_room = if all.is_empty() || id_cells + sep + all_cells <= room {
        all_cells
    } else {
        room.saturating_sub(id_cells.min(SUMMARY_ID_MIN_CELLS) + sep)
    };
    let tokens = if all_cells <= token_room {
        all
    } else {
        status_tokens(entry, detail, now, token_room)
    };
    let tokens_cells: usize = tokens.iter().map(Span::width).sum();
    let id_room = if tokens.is_empty() {
        room
    } else {
        room.saturating_sub(tokens_cells + sep)
    };
    spans.push(Span::styled(fit_cells(&id, id_room), id_style));
    if !tokens.is_empty() {
        spans.push(Span::raw(TOKEN_SEP));
        spans.extend(tokens);
    }
    // Below the fixed part's own width (a pane no tab ever gives) the row is cut, not
    // allowed to spill.
    clip_line(Line::from(spans), width)
}

// ── Height tiers (D-10, D-29) ───────────────────────────────────────────────

/// Detail **inner** height (the pane's rows below its one-row title) at which the FULL
/// tier starts: header 6, items 3 (rule + one item + the cursor-item row), caps 5
/// (rule + three caps + audit), connected 2 (rule + one row), LOG 4 (rule +
/// [`LOG_MIN_ROWS`]): `6 + 3 + 5 + 2 + 4 = 20`.
pub const DRIVER_FULL_MIN_ROWS: u16 = 20;

/// Detail inner height at which the MEDIUM tier starts: header 2, caps 3 (self-labelled,
/// no rule), items 2 (rule + one item), LOG 4: `2 + 3 + 2 + 4 = 11`. Below it the tab
/// is one summary row per run.
pub const DRIVER_MEDIUM_MIN_ROWS: u16 = 11;

const FULL_HEADER_ROWS: usize = 6;
const FULL_ITEMS_FLOOR: usize = 3;
const FULL_CAPS_ROWS: usize = 5;
const FULL_CONNECTED_FLOOR: usize = 2;
const MEDIUM_HEADER_ROWS: usize = 2;
const MEDIUM_CAPS_ROWS: usize = 3;
const MEDIUM_ITEMS_FLOOR: usize = 2;
/// The LOG section's floor in both tiers: its rule and [`LOG_MIN_ROWS`] body rows.
const LOG_FLOOR_ROWS: usize = 1 + LOG_MIN_ROWS;

const _: () = assert!(
    FULL_HEADER_ROWS + FULL_ITEMS_FLOOR + FULL_CAPS_ROWS + FULL_CONNECTED_FLOOR + LOG_FLOOR_ROWS
        == DRIVER_FULL_MIN_ROWS as usize
);
const _: () = assert!(
    MEDIUM_HEADER_ROWS + MEDIUM_CAPS_ROWS + MEDIUM_ITEMS_FLOOR + LOG_FLOOR_ROWS
        == DRIVER_MEDIUM_MIN_ROWS as usize
);

/// Which layout the tab area's height selects (D-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightTier {
    /// `H == 0`: nothing is drawn.
    None,
    /// `1 <= H <= 11`: no sections, one summary row per run across the whole width.
    Summary,
    /// `12 <= H <= 20`.
    Medium,
    /// `H >= 21`.
    Full,
}

/// The tier for a tab area `h` rows tall. The detail pane's title takes one row, so its
/// inner height is `h - 1`, and the tiers are named by that: FULL at
/// [`DRIVER_FULL_MIN_ROWS`] inner rows (`h >= 21`), MEDIUM at [`DRIVER_MEDIUM_MIN_ROWS`]
/// (`h >= 12`).
pub fn height_tier(h: u16) -> HeightTier {
    let inner = h.saturating_sub(1);
    if h == 0 {
        HeightTier::None
    } else if inner >= DRIVER_FULL_MIN_ROWS {
        HeightTier::Full
    } else if inner >= DRIVER_MEDIUM_MIN_ROWS {
        HeightTier::Medium
    } else {
        HeightTier::Summary
    }
}

/// How the rows of a tier are shared out (the rest are fixed: header, caps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierRows {
    /// Rows of the items section, its rule included.
    pub items: usize,
    /// Rows of the connected section, its rule included (always 0 in MEDIUM).
    pub connected: usize,
    /// Rows of the LOG section, its rule included.
    pub log: usize,
}

/// Share `inner` rows between the sections of `tier` (D-10, D-29).
///
/// Every section first gets its floor (or its whole demand when that is smaller, so a
/// section with nothing to show does not hold rows back). Spare rows then go to the items
/// (up to what they can show), then, in FULL only, to the connections, and every row
/// that remains goes to the LOG. A section that cannot get its floor is dropped by the
/// tier, never squeezed: the callers use FULL only from [`DRIVER_FULL_MIN_ROWS`] and
/// MEDIUM only from [`DRIVER_MEDIUM_MIN_ROWS`], where every floor fits.
pub fn allocate_rows(
    tier: HeightTier,
    inner: usize,
    items_demand: usize,
    connected_demand: usize,
) -> TierRows {
    match tier {
        HeightTier::Full => {
            let fixed = FULL_HEADER_ROWS + FULL_CAPS_ROWS;
            let items_floor = items_demand.min(FULL_ITEMS_FLOOR);
            let connected_floor = connected_demand.min(FULL_CONNECTED_FLOOR);
            let spare =
                inner.saturating_sub(fixed + items_floor + connected_floor + LOG_FLOOR_ROWS);
            let items_extra = spare.min(items_demand - items_floor);
            let connected_extra = (spare - items_extra).min(connected_demand - connected_floor);
            let items = items_floor + items_extra;
            let connected = connected_floor + connected_extra;
            TierRows {
                items,
                connected,
                log: inner.saturating_sub(fixed + items + connected),
            }
        }
        HeightTier::Medium => {
            let fixed = MEDIUM_HEADER_ROWS + MEDIUM_CAPS_ROWS;
            let items_floor = items_demand.min(MEDIUM_ITEMS_FLOOR);
            let spare = inner.saturating_sub(fixed + items_floor + LOG_FLOOR_ROWS);
            let items = items_floor + spare.min(items_demand - items_floor);
            TierRows {
                items,
                connected: 0,
                log: inner.saturating_sub(fixed + items),
            }
        }
        HeightTier::None | HeightTier::Summary => TierRows {
            items: 0,
            connected: 0,
            log: 0,
        },
    }
}

/// What the tiered composition draws for the selected launcher run: the lines above
/// the LOG, and whether the LOG takes the rows below them.
struct Composition {
    top: Vec<Line<'static>>,
    log: bool,
}

/// An unreadable run's detail: the error in Red and the promise that the run is read
/// again on the next change to its directory (UI-SPEC). No sections, so no LOG.
fn unreadable_lines(
    entry: &DriverRunEntry,
    error: &crate::state_reader::driver_frontmatter::FrontmatterError,
    now: SystemTime,
    width: usize,
) -> Vec<Line<'static>> {
    let message = format!(
        "DIRECTION.md could not be read: {}",
        shown_capped(&error.to_string())
    );
    vec![
        header_row_1(entry, now, width),
        Line::from(Span::styled(
            fit_cells(&message, width),
            Style::default().fg(Color::Red),
        )),
        Line::from(Span::styled(
            "It is read again on the next change to the run directory.",
            muted_style(),
        )),
    ]
}

/// The tiered detail composition for the selected launcher run (D-10, D-29).
///
/// FULL: header 6, items (rule + item rows + the cursor-item row), caps 5, connected,
/// then the LOG. MEDIUM: header 2 (row 1 and the status tokens), the three self-labelled
/// caps, items (rule + item rows), then the LOG. `inner_height` is the pane's rows
/// below its title; `tier` is FULL or MEDIUM (SUMMARY has its own renderer).
fn compose_selected(
    entry: &DriverRunEntry,
    cache: Option<&ProjectViewCache>,
    now: SystemTime,
    width: usize,
    inner_height: usize,
    tier: HeightTier,
) -> Composition {
    let view = match &entry.1 {
        Ok(view) => view,
        Err(error) => {
            return Composition {
                top: unreadable_lines(entry, error, now, width),
                log: false,
            }
        }
    };
    let detail = detail_for(cache, &entry.0);
    let cursor = cache.and_then(|c| c.launcher_item_cursor.as_deref());
    // The log's park rules, only from the tail of THIS run.
    let rules = cache
        .map(|c| &c.launcher_log)
        .filter(|tail| tail.run_id == entry.0)
        .map(|tail| &tail.park_rules);
    let full = tier == HeightTier::Full;
    let items_demand = items_section_fit(detail, cursor, rules, width, usize::MAX, full).len();
    let connected_demand = if full {
        connected_section_fit(&view.frontmatter, now, width, usize::MAX).len()
    } else {
        0
    };
    let rows = allocate_rows(tier, inner_height, items_demand, connected_demand);
    let items = items_section_fit(detail, cursor, rules, width, rows.items, full);

    let top = if full {
        let mut lines = header_rows(
            entry,
            view,
            detail.and_then(|d| d.body_control.as_deref()),
            now,
            width,
        );
        lines.extend(items);
        lines.extend(caps_section(&view.frontmatter, width));
        lines.extend(connected_section_fit(
            &view.frontmatter,
            now,
            width,
            rows.connected,
        ));
        lines
    } else {
        let mut lines = vec![
            header_row_1(entry, now, width),
            token_row(entry, detail, now, width),
        ];
        lines.extend(cap_rows(&view.frontmatter, width));
        lines.extend(items);
        lines
    };
    Composition { top, log: true }
}

/// What `i` says on the Driver tab since Phase 30 (D-02): the injection input is
/// retired, and these are the keys that steer a run now.
pub const INJECTION_RETIRED: &str = "Message injection is retired. Steer this run through DIRECTION.md: [c] control, [a] answer, [e] edit.";

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

/// Whether the highlighted row is a launcher run whose lock is held
/// (`AsWritten(Running | Starting)`). The input of the heartbeat-age redraw gate.
pub fn selected_run_is_live(cache: Option<&ProjectViewCache>) -> bool {
    match (selection(cache), cache) {
        (Selection::Launcher(index), Some(cache)) => cache.launcher_runs[index]
            .1
            .as_ref()
            .is_ok_and(|view| is_live(&view.derived)),
        _ => false,
    }
}

// ── What a steering key acts on (30-04) ─────────────────────────────────────

/// What `c`, `a`, `u` and `e` act on for the current selection.
///
/// Owned data, so a key handler can read it and then set a status message on the
/// same context without a borrow outliving the read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteerTarget {
    /// A readable launcher run: its id and the control word its cached body holds
    /// (`None` when no detail is cached for it yet, or the body has no control line).
    Run {
        run_id: String,
        body_control: Option<String>,
    },
    /// A launcher run whose `DIRECTION.md` could not be read: nothing to steer.
    Unreadable { run_id: String },
    /// A legacy journal run: read-only here (D-01).
    Legacy,
    /// No run is highlighted.
    Nothing,
}

/// The steering target for `cache`'s current selection. Pure over the cache.
pub fn steer_target(cache: Option<&ProjectViewCache>) -> SteerTarget {
    match (selection(cache), cache) {
        (Selection::Launcher(index), Some(cache)) => {
            let (run_id, entry) = &cache.launcher_runs[index];
            if entry.is_ok() {
                SteerTarget::Run {
                    run_id: run_id.clone(),
                    body_control: detail_for(Some(cache), run_id)
                        .and_then(|detail| detail.body_control.clone()),
                }
            } else {
                SteerTarget::Unreadable {
                    run_id: run_id.clone(),
                }
            }
        }
        (Selection::Legacy(_), _) => SteerTarget::Legacy,
        _ => SteerTarget::Nothing,
    }
}

/// What a steering key says when the highlighted run cannot be steered: `None` for a
/// readable launcher run (the key proceeds), the refusal copy otherwise. A key on a
/// build that cannot write DIRECTION.md says so first.
pub fn steer_refusal(target: &SteerTarget) -> Option<String> {
    if !super::driver_steer::can_steer() && !matches!(target, SteerTarget::Nothing) {
        return Some(super::driver_steer::STEERING_NEEDS_LINUX.to_string());
    }
    match target {
        SteerTarget::Run { .. } | SteerTarget::Nothing => None,
        SteerTarget::Legacy => Some(LEGACY_READ_ONLY.to_string()),
        SteerTarget::Unreadable { run_id } => Some(format!(
            "{} could not be read, so it cannot be steered.",
            shown_capped(run_id)
        )),
    }
}

/// The item under the item cursor of the selected launcher run: the merged row the
/// cursor resolves to (the first parked item, else `current_item`, else the first,
/// until `[`/`]` moved it). `None` with no launcher selection, no cached detail for
/// the selected run, or no items.
pub fn cursor_item_row(cache: Option<&ProjectViewCache>) -> Option<ItemRow> {
    let cache = cache?;
    let Selection::Launcher(index) = selection(Some(cache)) else {
        return None;
    };
    let detail = detail_for(Some(cache), &cache.launcher_runs[index].0)?;
    let rows = merged_items(detail);
    let current = detail.view.frontmatter.current_item.as_deref();
    let id = resolved_item_cursor(&rows, cache.launcher_item_cursor.as_deref(), current)?;
    rows.into_iter().find(|row| row.id == id)
}

/// The rule the selected run's LOG recorded for `item`'s last park, valid only while
/// the cached tail speaks for `run_id` (30-02, D-16).
pub fn park_rule_for(cache: Option<&ProjectViewCache>, run_id: &str, item: &str) -> Option<String> {
    cache
        .filter(|cache| cache.launcher_log.run_id == run_id)
        .and_then(|cache| cache.launcher_log.park_rules.get(item))
        .cloned()
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
        let from_legacy = cache.legacy_row_selected;
        cache.legacy_row_selected = false;
        cache.launcher_selected = next;
        let id = cache.launcher_runs.get(next).map(|(id, _)| id.clone());
        if id != cache.launcher_selected_id {
            // Another run: its items, its cursor and its LOG are not this run's.
            // The detail and the tail are read again for the new selection.
            cache.launcher_item_cursor = None;
            if cache.launcher_detail.as_deref().map(|d| &d.run_id) != id.as_ref() {
                cache.launcher_detail = None;
            }
        }
        if id != cache.launcher_selected_id || from_legacy {
            // The pane starts at its tail with follow on (D-23): the scroll state
            // is shared with the legacy output pane and belongs to neither run.
            cache.driver_scroll_offset = 0;
            cache.driver_follow = true;
        }
        cache.launcher_selected_id = id;
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
    /// `s`: start a new run (30-05, D-18); Linux only, because creating the run and
    /// editing its body are Linux-only writes.
    pub start: bool,
    /// `PgUp`/`PgDn`/`f`: there is a pane to scroll: the LOG tail of a launcher run
    /// (30-02) or the output pane of a legacy run.
    pub scroll: bool,
    /// `[` / `]`: the selected launcher run has items to move the cursor over.
    pub item: bool,
    /// `c`: a readable launcher run is selected and this build can steer (Linux).
    pub control: bool,
    /// `a`: the item under the cursor is parked or on hold, is in the body, and this
    /// build can steer.
    pub answer: bool,
    /// `u`: the item under the cursor is on hold and this build can steer.
    pub unhold: bool,
    /// `e`: a readable launcher run is selected and this build can steer (Linux).
    pub edit: bool,
    /// This build cannot steer (not Linux): `c`, `a`, `u`, `e` are absent and the
    /// footer says why (D-14).
    pub read_only_platform: bool,
    /// `R`: the selected launcher run may be launched (`Launch`, never launched) or
    /// relaunched (`Relaunch`), and this build can launch (30-05, D-21). `None` for a
    /// running run, an unknown liveness, a done run, a legacy row or a non-Linux build.
    pub launch: Option<super::driver_launch::LaunchKind>,
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
        start: super::driver_steer::can_steer(),
        scroll: !matches!(sel, Selection::Nothing),
        item: match (sel, cache) {
            (Selection::Launcher(index), Some(cache)) => {
                detail_for(Some(cache), &cache.launcher_runs[index].0)
                    .is_some_and(|detail| !merged_items(detail).is_empty())
            }
            _ => false,
        },
        control: super::driver_steer::can_steer()
            && matches!(steer_target(cache), SteerTarget::Run { .. }),
        answer: super::driver_steer::can_steer()
            && cursor_item_row(cache).is_some_and(|row| row.in_body() && row.answerable()),
        unhold: super::driver_steer::can_steer()
            && cursor_item_row(cache).is_some_and(|row| row.in_body() && row.hold),
        edit: super::driver_steer::can_steer()
            && matches!(steer_target(cache), SteerTarget::Run { .. }),
        read_only_platform: !super::driver_steer::can_steer(),
        launch: match (sel, cache) {
            (Selection::Launcher(index), Some(cache)) if super::driver_steer::can_steer() => cache
                .launcher_runs[index]
                .1
                .as_ref()
                .ok()
                .and_then(|view| super::driver_launch::launch_kind_for(&view.derived).ok()),
            _ => None,
        },
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

/// One row of the `Driver Item States` help legend per UI-SPEC item row: the glyph
/// (a fixed constant), the word, and a one-line gloss that claims no more than the
/// body and the driver's recorded state can establish.
pub const ITEM_STATES_LEGEND: [(&str, &str, &str); 7] = [
    (
        GLYPH_RUN_UNLAUNCHED,
        "pending",
        "recorded by the driver, not started",
    ),
    (GLYPH_RUN_LIVE, "running", "recorded as being worked on"),
    (GLYPH_RUN_DONE, "done", "recorded as finished"),
    (
        GLYPH_RUN_PARKED,
        "parked",
        "the driver stopped on it: it needs a human",
    ),
    (GLYPH_RUN_CRASHED, "failed", "recorded as failed"),
    (
        GLYPH_ITEM_UNSEEN,
        "unseen",
        "in DIRECTION.md; the driver has recorded no state for it",
    ),
    (
        "\u{2013}",
        "not in DIRECTION",
        "recorded by the driver; DIRECTION.md no longer lists it",
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
    // One clock read per frame: the heartbeat ages and the stale flag of every row
    // and of the header are judged against the same instant. No file is touched.
    let now = SystemTime::now();
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
    let tier = height_tier(area.height);
    let (list_area, detail_area) = driver_panes(area);

    // The verdict applies only to the run the reconciliation scan is observing;
    // every other legacy row is a finished run whose own `outcome` speaks for it.
    let observed = ctx.observed_runs.get(alias);
    let verdict_for = |run: &RunSummary| -> Option<RunVerdict> {
        observed
            .filter(|observed| observed.run_id == run.run_id)
            .map(|observed| observed.verdict())
    };

    // SUMMARY (D-10): no sections, so no header can be orphaned. Both panes give way
    // to one row per run across the whole width, windowed to keep the selection on
    // screen. The dry-run preview (the retired start flow's Step B) keeps the pane.
    if tier == HeightTier::Summary && preview.is_none() {
        // No LOG is drawn, so the scroll keys must not act on a stale viewport.
        viewport.set(ViewportMetrics::default());
        if runs.is_empty() && legacy.is_empty() {
            // Nothing listed yet (the listed-and-empty case returned above).
            let loading = Line::from(Span::styled(DRY_RUN_LOADING, muted_style()));
            frame.render_widget(Paragraph::new(loading), area);
            return regions;
        }
        let width = usize::from(area.width);
        let mut items: Vec<ListItem> = runs
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                ListItem::new(summary_row(
                    entry,
                    detail_for(cache, &entry.0),
                    sel == Selection::Launcher(index),
                    now,
                    width,
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
                        area.width,
                    ))
                }));
            }
        }
        let len = items.len();
        let mut list_state = ListState::default().with_offset(list_offset.get());
        list_state.select(match sel {
            Selection::Launcher(index) => Some(index),
            Selection::Legacy(index) => Some(runs.len() + 1 + index),
            Selection::Nothing => None,
        });
        frame.render_stateful_widget(List::new(items), area, &mut list_state);
        list_offset.set(list_state.offset());
        regions.list = Some(crate::ui::mouse::ListRegion {
            rect: area,
            offset: list_state.offset(),
            len,
        });
        return regions;
    }

    if let Some(list_area) = list_area {
        let list_block = Block::default().borders(Borders::RIGHT).title(" Runs ");
        let rows = list_block.inner(list_area);
        let mut items: Vec<ListItem> = runs
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                ListItem::new(run_row_at(
                    entry,
                    sel == Selection::Launcher(index),
                    rows.width,
                    now,
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
            let entry = &runs[index];
            let comp = compose_selected(
                entry,
                cache,
                now,
                usize::from(inner.width),
                usize::from(inner.height),
                tier,
            );
            // The sections take the rows the tier gave them; the LOG tail takes every
            // row below, its rule and at least [`LOG_MIN_ROWS`] under it (D-29).
            let top_rows = u16::try_from(comp.top.len())
                .unwrap_or(u16::MAX)
                .min(inner.height);
            let top = Rect {
                height: top_rows,
                ..inner
            };
            frame.render_widget(Paragraph::new(comp.top), top);
            if comp.log && inner.height > top_rows {
                let log_area = Rect {
                    y: inner.y + top_rows,
                    height: inner.height - top_rows,
                    ..inner
                };
                let live = entry.1.as_ref().is_ok_and(|view| is_live(&view.derived));
                render_log_section(frame, log_area, cache, &entry.0, live, viewport);
                regions.output = Some(log_area);
            }
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
        // The fixture's own stamps are fixed dates: a live run would read stale.
        refresh_heartbeats(&mut frontmatter);
        // And its last event's summary says `status=crashed`, which a test that asserts
        // a run is never called crashed must not trip over.
        frontmatter.last_event.summary = Some("rc=0".to_owned());
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

    // ── the merged item list (D-06) ────────────────────────────────────────

    use crate::state_reader::driver_frontmatter::{ItemStateEntry, ItemStates};

    fn body_item(id: &str, title: &str, hold: Option<&str>, answer: Option<&str>) -> BodyItem {
        BodyItem {
            id: id.to_owned(),
            title: title.to_owned(),
            hold: hold.map(str::to_owned),
            answer: answer.map(str::to_owned),
        }
    }

    fn state_entry(state: ItemState, dispatches: u32, outcome: Option<&str>) -> ItemStateEntry {
        ItemStateEntry {
            state,
            dispatches,
            outcome: outcome.map(str::to_owned),
        }
    }

    fn detail_of(
        id: &str,
        items: Vec<BodyItem>,
        states: Vec<(&str, ItemStateEntry)>,
        current_item: Option<&str>,
    ) -> RunDetail {
        let (_, entry) = entry(id, running());
        let mut view = entry.expect("the fixture parses");
        view.frontmatter.item_states = ItemStates(
            states
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        );
        view.frontmatter.current_item = current_item.map(str::to_owned);
        RunDetail {
            run_id: id.to_owned(),
            view,
            body_control: Some("run".to_owned()),
            items,
        }
    }

    #[test]
    fn merged_items_keep_body_order_then_frontmatter_only_in_item_states_order() {
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-02", "second", None, None),
                body_item("I-01", "first", Some("true"), Some("use /run")),
            ],
            vec![
                ("I-09", state_entry(ItemState::Done, 3, Some("shipped"))),
                (
                    "I-01",
                    state_entry(ItemState::Parked, 2, Some("needs a socket path")),
                ),
                ("I-07", state_entry(ItemState::Failed, 1, None)),
            ],
            None,
        );
        let rows = merged_items(&detail);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["I-02", "I-01", "I-09", "I-07"],
            "an id in both is one row"
        );
        assert_eq!(rows[0].mark, ItemMark::Unseen, "body-only");
        assert_eq!(rows[0].dispatches, 0);
        assert_eq!(rows[1].mark, ItemMark::Seen(ItemState::Parked), "in both");
        assert_eq!(rows[1].dispatches, 2);
        assert!(rows[1].hold);
        assert_eq!(rows[1].answer.as_deref(), Some("use /run"));
        assert_eq!(rows[1].outcome.as_deref(), Some("needs a socket path"));
        assert_eq!(rows[2].mark, ItemMark::NotInDirection(ItemState::Done));
        assert_eq!(rows[2].title, None);
        assert_eq!(rows[3].mark, ItemMark::NotInDirection(ItemState::Failed));
    }

    #[test]
    fn a_repeated_body_id_is_one_row() {
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-01", "a", None, None),
                body_item("I-01", "again", None, None),
            ],
            vec![],
            None,
        );
        assert_eq!(merged_items(&detail).len(), 1);
    }

    #[test]
    fn the_item_cursor_starts_on_the_first_parked_item_else_current_else_first() {
        let rows = |states: Vec<(&str, ItemStateEntry)>| {
            merged_items(&detail_of(
                "20261020-a",
                vec![
                    body_item("I-01", "a", None, None),
                    body_item("I-02", "b", None, None),
                    body_item("I-03", "c", None, None),
                ],
                states,
                None,
            ))
        };
        let parked = rows(vec![
            ("I-02", state_entry(ItemState::Running, 1, None)),
            ("I-03", state_entry(ItemState::Parked, 1, None)),
        ]);
        assert_eq!(
            item_cursor_default(&parked, Some("I-02")).as_deref(),
            Some("I-03")
        );
        let none_parked = rows(vec![("I-02", state_entry(ItemState::Running, 1, None))]);
        assert_eq!(
            item_cursor_default(&none_parked, Some("I-02")).as_deref(),
            Some("I-02")
        );
        assert_eq!(
            item_cursor_default(&none_parked, Some("I-77")).as_deref(),
            Some("I-01"),
            "an unlisted current item falls to the first row"
        );
        assert_eq!(item_cursor_default(&[], Some("I-01")), None);
        // A stored cursor wins while it is listed, and re-defaults when it is not.
        assert_eq!(
            resolved_item_cursor(&parked, Some("I-01"), None).as_deref(),
            Some("I-01")
        );
        assert_eq!(
            resolved_item_cursor(&parked, Some("I-99"), None).as_deref(),
            Some("I-03")
        );
    }

    fn text_lines(lines: &[Line<'_>]) -> Vec<String> {
        lines.iter().map(text_of).collect()
    }

    #[test]
    fn item_rows_carry_the_glyph_word_dispatches_hold_and_title() {
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-03", "Wire the run list", Some("true"), None),
                body_item("I-04", "Steering transforms", None, None),
                body_item("I-05", "Dashboard marker", None, None),
            ],
            vec![
                (
                    "I-03",
                    state_entry(ItemState::Parked, 2, Some("which socket path?")),
                ),
                ("I-04", state_entry(ItemState::Running, 1, None)),
                ("I-09", state_entry(ItemState::Done, 4, Some("shipped"))),
            ],
            None,
        );
        let lines = items_section(Some(&detail), None, None, 70);
        let text = text_lines(&lines);
        assert!(text[0].starts_with("\u{2500}\u{2500} items "), "{text:#?}");
        assert_eq!(
            text[1].trim_end(),
            "> I-03 \u{2691} parked   2d hold  Wire the run list"
        );
        assert_eq!(
            text[2].trim_end(),
            "  I-04 \u{25C6} running  1d       Steering transforms"
        );
        assert_eq!(
            text[3].trim_end(),
            "  I-05 \u{B7} unseen   0d       Dashboard marker"
        );
        assert!(
            text[4].starts_with("  I-09 \u{25CF} done     4d")
                && text[4].ends_with("[not in DIRECTION]"),
            "{:?}",
            text[4]
        );
        // The cursor-item row: the parked item, its outcome.
        assert_eq!(text[5], "  I-03 parked: which socket path?");
        // Colours come from the closed enum only.
        let parked = &lines[1];
        assert_eq!(parked.spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(
            parked.spans[2].style.fg,
            Some(Color::Red),
            "the parked glyph"
        );
        assert_eq!(
            parked.spans[7].style.fg,
            Some(Color::Yellow),
            "the hold flag"
        );
        assert_eq!(lines[3].spans[2].style.fg, Some(Color::DarkGray), "unseen");
        assert_eq!(
            lines[4].spans.last().unwrap().style.fg,
            Some(Color::DarkGray)
        );
    }

    #[test]
    fn the_cursor_item_row_names_the_answer_or_that_no_outcome_is_recorded() {
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-01", "a", None, Some("use /run/user")),
                body_item("I-02", "b", None, None),
            ],
            vec![(
                "I-01",
                state_entry(ItemState::Parked, 1, Some("needs a path")),
            )],
            None,
        );
        let with_answer = text_lines(&items_section(Some(&detail), Some("I-01"), None, 80));
        assert_eq!(
            with_answer.last().unwrap(),
            "  I-01 parked: needs a path  answer: use /run/user"
        );
        let none = text_lines(&items_section(Some(&detail), Some("I-02"), None, 80));
        assert_eq!(
            none.last().unwrap().trim_end(),
            "  I-02: no outcome recorded"
        );
        // The rule from the LOG, when one is known, is named.
        let mut rules = std::collections::BTreeMap::new();
        rules.insert("I-01".to_owned(), "needs-human".to_owned());
        let by = text_lines(&items_section(
            Some(&detail),
            Some("I-01"),
            Some(&rules),
            80,
        ));
        assert!(
            by.last()
                .unwrap()
                .contains("parked by needs-human: needs a path"),
            "{by:#?}"
        );
    }

    #[test]
    fn no_items_in_either_source_gives_the_placeholder_and_no_detail_gives_loading() {
        let empty = detail_of("20261020-a", vec![], vec![], None);
        let text = text_lines(&items_section(Some(&empty), None, None, 60));
        assert_eq!(text.len(), 2, "{text:?}");
        assert_eq!(
            text[1].trim(),
            "No items in DIRECTION.md yet. Add one with [e]."
        );
        let loading = text_lines(&items_section(None, None, None, 60));
        assert_eq!(loading[1].trim(), "Loading...");
    }

    #[test]
    fn more_items_than_rows_show_a_more_row_and_keep_the_cursor_visible() {
        let items: Vec<BodyItem> = (1..=12)
            .map(|n| body_item(&format!("I-{n:02}"), &format!("item {n}"), None, None))
            .collect();
        let detail = detail_of("20261020-a", items, vec![], None);
        let at = |cursor: &str| text_lines(&items_section(Some(&detail), Some(cursor), None, 60));
        let top = at("I-01");
        assert!(top[1].starts_with("> I-01"), "{top:#?}");
        assert_eq!(top.iter().filter(|l| l.contains("more items")).count(), 1);
        assert!(
            top.iter().any(|l| l.trim() == "\u{2026} 4 more items"),
            "{top:#?}"
        );
        let low = at("I-12");
        assert!(low.iter().any(|l| l.starts_with("> I-12")), "{low:#?}");
        assert!(
            !low.iter().any(|l| l.contains("I-01 ")),
            "the window scrolled: {low:#?}"
        );
        let rows_shown = low.iter().filter(|l| l.contains(" item ")).count();
        assert_eq!(rows_shown, ITEMS_MAX_ROWS, "{low:#?}");
    }

    #[test]
    fn a_long_title_is_cut_by_display_width_and_a_cramped_row_drops_it() {
        let wide =
            "配線を確認するための非常に長い見出しです配線を確認するための非常に長い見出しです";
        let emoji = "🚀".repeat(40);
        for title in [
            wide.to_owned(),
            emoji,
            "an ordinary but rather long item title".repeat(3),
        ] {
            let row = ItemRow {
                id: "I-01".to_owned(),
                title: Some(title.clone()),
                mark: ItemMark::Seen(ItemState::Running),
                dispatches: 1,
                hold: true,
                outcome: None,
                answer: None,
            };
            for width in [28usize, 29, 30, 39, 60, 80] {
                let line = item_row(&row, false, width);
                assert!(
                    line.width() <= width,
                    "{title:?} at {width} measured {}: {:?}",
                    line.width(),
                    text_of(&line)
                );
            }
            // Room 1 or 0: no title span and no orphan ellipsis.
            let cramped = text_of(&item_row(&row, false, ITEM_ROW_FIXED_CELLS + 1));
            assert!(!cramped.contains('\u{2026}'), "{cramped:?}");
            // Room 2: one cell of title plus the ellipsis is the smallest cut.
            let tight = text_of(&item_row(&row, false, ITEM_ROW_FIXED_CELLS + 3));
            assert!(tight.contains('\u{2026}'), "{tight:?}");
        }
    }

    #[test]
    fn a_foreign_item_keeps_its_marker_when_the_pane_is_cramped() {
        let row = ItemRow {
            id: "I-09".to_owned(),
            title: None,
            mark: ItemMark::NotInDirection(ItemState::Done),
            dispatches: 0,
            hold: false,
            outcome: None,
            answer: None,
        };
        assert!(text_of(&item_row(&row, false, 60)).ends_with("[not in DIRECTION]"));
    }

    #[test]
    fn a_hostile_item_title_and_id_reach_the_rows_sanitised() {
        let detail = detail_of(
            "20261020-a",
            vec![body_item(
                "I-01",
                "evil\u{1b}[2J\u{202e}title",
                None,
                Some("a\u{7}b"),
            )],
            vec![(
                "I-0\u{1b}]0;x\u{7}9",
                state_entry(ItemState::Done, 1, Some("o\u{1b}[31mk")),
            )],
            None,
        );
        for cursor in ["I-01", "I-0\u{1b}]0;x\u{7}9"] {
            for line in items_section(Some(&detail), Some(cursor), None, 80) {
                let text = text_of(&line);
                assert!(!text.chars().any(char::is_control), "{text:?}");
                assert!(!text.contains('\u{202e}'), "{text:?}");
            }
        }
    }

    // ── the selected run's items through the composition ───────────────────

    fn cache_with_detail(detail: RunDetail) -> ProjectViewCache {
        let run = detail.run_id.clone();
        let mut cache = cache_of(vec![entry(&run, running())]);
        cache.launcher_selected_id = Some(run);
        cache.launcher_detail = Some(Box::new(detail));
        cache
    }

    #[test]
    fn the_items_render_under_the_selected_run_and_only_for_its_own_detail() {
        let detail = detail_of(
            "20261020-a",
            vec![body_item("I-01", "first thing", None, None)],
            vec![("I-01", state_entry(ItemState::Parked, 2, Some("stuck")))],
            None,
        );
        let (rows, _) = render(cache_with_detail(detail.clone()), 100, 24);
        assert!(
            row_with(&rows, "I-01 \u{2691}").contains("parked"),
            "{rows:#?}"
        );
        assert!(
            rows.iter().any(|r| r.contains("\u{2500}\u{2500} items")),
            "{rows:#?}"
        );

        // A detail left over from another run is not drawn under this one.
        let mut stale = cache_with_detail(detail);
        stale.launcher_detail.as_mut().unwrap().run_id = "20261019-other".to_owned();
        let (rows, _) = render(stale, 100, 24);
        assert!(!rows.iter().any(|r| r.contains("first thing")), "{rows:#?}");
        assert!(rows.iter().any(|r| r.contains("Loading...")), "{rows:#?}");
    }

    #[test]
    fn a_cjk_and_emoji_title_never_overflows_the_row_at_60_columns() {
        let wide =
            "配線を確認するための非常に長い見出しです配線を確認するための非常に長い見出しです";
        let emoji = "🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀";
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-01", wide, None, None),
                body_item("I-02", emoji, None, None),
            ],
            vec![],
            None,
        );
        let (rows, _) = render(cache_with_detail(detail.clone()), 60, 24);
        let first = row_with(&rows, "I-01");
        assert!(
            first.contains('\u{2026}'),
            "the long title is cut with an ellipsis: {first:?}"
        );
        let second = row_with(&rows, "I-02");
        assert!(second.contains('\u{2026}'), "{second:?}");
        // The pane floor is 39 cells; no item row is wider than the pane, whatever the
        // title's script (a wide glyph counts two cells).
        for width in [39usize, 45, 60] {
            for line in items_section(Some(&detail), None, None, width)
                .iter()
                .skip(1)
            {
                assert!(line.width() <= width, "{width}: {:?}", text_of(line));
            }
        }
    }

    #[test]
    fn the_item_cursor_moves_by_id_and_clamps() {
        let detail = detail_of(
            "20261020-a",
            (1..=3)
                .map(|n| body_item(&format!("I-0{n}"), "t", None, None))
                .collect(),
            vec![],
            None,
        );
        let mut cache = cache_with_detail(detail);
        assert!(move_item_cursor(&mut cache, 1));
        assert_eq!(cache.launcher_item_cursor.as_deref(), Some("I-02"));
        assert!(move_item_cursor(&mut cache, 9));
        assert_eq!(cache.launcher_item_cursor.as_deref(), Some("I-03"));
        assert!(move_item_cursor(&mut cache, -9));
        assert_eq!(cache.launcher_item_cursor.as_deref(), Some("I-01"));
        // Inert with no detail, and with no items.
        cache.launcher_detail = None;
        assert!(!move_item_cursor(&mut cache, 1));
        let mut empty = cache_with_detail(detail_of("20261020-a", vec![], vec![], None));
        assert!(!move_item_cursor(&mut empty, 1));
        assert_eq!(empty.launcher_item_cursor, None);
    }

    #[test]
    fn pinned_index_follows_the_id_and_clamps_the_index() {
        let runs = vec![
            entry("20261020-c", running()),
            entry("20261019-b", running()),
            entry("20261018-a", running()),
        ];
        assert_eq!(pinned_index(&runs, Some("20261018-a"), 0), 2);
        assert_eq!(pinned_index(&runs, Some("20261001-gone"), 7), 2);
        assert_eq!(pinned_index(&runs, None, 1), 1);
        assert_eq!(pinned_index(&[], Some("x"), 4), 0);
    }

    #[test]
    fn store_detail_keeps_only_the_selected_runs_detail_and_pins_the_cursor() {
        let detail = detail_of(
            "20261020-a",
            vec![
                body_item("I-01", "a", None, None),
                body_item("I-02", "b", None, None),
            ],
            vec![("I-02", state_entry(ItemState::Parked, 1, None))],
            None,
        );
        let mut cache = cache_of(vec![entry("20261020-a", running())]);
        cache.launcher_selected_id = Some("20261020-a".to_owned());
        store_detail(&mut cache, Some(Box::new(detail.clone())));
        assert_eq!(
            cache.launcher_item_cursor.as_deref(),
            Some("I-02"),
            "first parked"
        );
        // A stored cursor that is still listed stays put.
        cache.launcher_item_cursor = Some("I-01".to_owned());
        store_detail(&mut cache, Some(Box::new(detail.clone())));
        assert_eq!(cache.launcher_item_cursor.as_deref(), Some("I-01"));
        // A detail for another run is dropped.
        let mut other = detail.clone();
        other.run_id = "20261019-other".to_owned();
        store_detail(&mut cache, Some(Box::new(other)));
        assert_eq!(
            cache.launcher_detail.as_deref().map(|d| d.run_id.as_str()),
            Some("20261020-a")
        );
        // The selection moved: the old run's detail is cleared, not shown.
        cache.launcher_selected_id = Some("20261018-new".to_owned());
        store_detail(&mut cache, None);
        assert!(cache.launcher_detail.is_none());
    }

    #[test]
    fn moving_the_launcher_selection_resets_the_cursor_and_drops_the_old_detail() {
        let detail = detail_of(
            "20261020-a",
            vec![body_item("I-01", "a", None, None)],
            vec![],
            None,
        );
        let mut cache = cache_with_detail(detail);
        cache.launcher_runs.push(entry("20261019-b", running()));
        cache.launcher_item_cursor = Some("I-01".to_owned());
        cache.driver_follow = false;
        cache.driver_scroll_offset = 9;
        assert!(!move_selection(&mut cache, 1));
        assert_eq!(cache.launcher_selected_id.as_deref(), Some("20261019-b"));
        assert_eq!(cache.launcher_item_cursor, None);
        assert!(cache.launcher_detail.is_none());
        assert_eq!(
            (cache.driver_follow, cache.driver_scroll_offset),
            (true, 0),
            "another run's LOG starts at its tail with follow on"
        );
        // Staying on the same run leaves the pane where the user put it.
        cache.driver_follow = false;
        cache.driver_scroll_offset = 4;
        assert!(!move_selection(&mut cache, 0));
        assert_eq!(
            (cache.driver_follow, cache.driver_scroll_offset),
            (false, 4)
        );
    }

    // ── Task 2: control, heartbeat, caps, audit, connected ─────────────────

    /// An entry built from the fixture with `edit` applied to its frontmatter (after
    /// the heartbeats were refreshed), for the header, caps and connected tests.
    fn entry_edit(
        id: &str,
        derived: DerivedStatus,
        edit: impl FnOnce(&mut DriverFrontmatter),
    ) -> DriverRunEntry {
        let (id, entry) = entry(id, derived);
        let mut view = entry.expect("the fixture parses");
        edit(&mut view.frontmatter);
        (id, Ok(view))
    }

    fn at(secs_after_epoch: u64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(secs_after_epoch)
    }

    #[test]
    fn the_control_table_follows_the_ui_spec() {
        // Equal words, case-insensitively: the word alone, live or not.
        assert_eq!(
            control_shown(Some("pause"), Some("pause"), true),
            ("pause".into(), None)
        );
        assert_eq!(
            control_shown(Some("pause"), Some("Pause"), false),
            ("pause".into(), None)
        );
        // Different, live: the body word, pending.
        assert_eq!(
            control_shown(Some("pause"), Some("run"), true),
            ("pause".into(), Some("(pending)"))
        );
        // Different, not live: the body word, at next launch. [inferred — audit]
        assert_eq!(
            control_shown(Some("stop"), Some("run"), false),
            ("stop".into(), Some("(at next launch)"))
        );
        // The frontmatter never echoed one: still different.
        assert_eq!(
            control_shown(Some("run"), None, true),
            ("run".into(), Some("(pending)"))
        );
        // No control line in the body.
        assert_eq!(
            control_shown(None, Some("run"), true),
            ("(no control line)".into(), None)
        );
        // Never the words the driver has not echoed.
        for (body, fm, live) in [
            ("pause", "run", true),
            ("stop", "run", false),
            ("pause", "stop", true),
        ] {
            let (word, suffix) = control_shown(Some(body), Some(fm), live);
            assert_eq!(word, body, "the body word, never `paused`/`stopped`");
            assert!(suffix.is_some());
        }
    }

    #[test]
    fn age_formats_cross_each_unit_boundary() {
        for (secs, text) in [
            (0, "0:00"),
            (59, "0:59"),
            (60, "1:00"),
            (190, "3:10"),
            (3599, "59:59"),
            (3600, "1:00:00"),
            (86_399, "23:59:59"),
            (86_400, "1d 0h"),
            (90_000, "1d 1h"),
        ] {
            assert_eq!(format_age(secs), text, "{secs}");
        }
    }

    #[test]
    fn the_age_is_whole_seconds_rounded_down_with_future_and_garbage_handled() {
        let now = at(1_000_000_000);
        let stamp = |secs: u64| {
            crate::driverctl::log::utc_string(chrono::DateTime::<chrono::Utc>::from(at(secs)))
        };
        assert_eq!(
            heartbeat_age_secs(&stamp(1_000_000_000 - 190), now),
            Some(190)
        );
        // A fraction of a second is rounded down.
        let fractional = "2001-09-09T01:46:39.900Z"; // 999_999_999.9 s after the epoch
        assert_eq!(heartbeat_age_secs(fractional, now), Some(0));
        // A heartbeat stamped in the future (clock skew) is 0:00, never negative.
        assert_eq!(heartbeat_age_secs(&stamp(1_000_000_500), now), Some(0));
        // An unparseable stamp is None, never a panic.
        for garbage in ["", "yesterday", "2026-13-45T99:00:00Z", "\u{1b}[2J"] {
            assert_eq!(heartbeat_age_secs(garbage, now), None, "{garbage:?}");
        }
    }

    #[test]
    fn the_stale_boundary_is_the_files_threshold_and_the_age_must_exceed_it() {
        let now = at(2_000_000_000);
        let stamp = |ago: u64| {
            crate::driverctl::log::utc_string(chrono::DateTime::<chrono::Utc>::from(at(
                2_000_000_000 - ago,
            )))
        };
        let fm_with = |ago: u64, threshold: u64| {
            let (_, entry) = entry_edit("20261020-a", running(), |fm| {
                fm.heartbeat_stale_after_s = threshold;
                for conn in &mut fm.connected {
                    conn.last_heartbeat = Some(stamp(ago));
                }
            });
            entry.expect("readable").frontmatter
        };
        assert!(
            !heartbeat_stale(&fm_with(300, 300), &running(), now),
            "exactly 300 s is not stale"
        );
        assert!(
            heartbeat_stale(&fm_with(301, 300), &running(), now),
            "301 s is stale"
        );
        // The threshold is the file's, not a constant.
        assert!(heartbeat_stale(&fm_with(121, 120), &running(), now));
        assert!(!heartbeat_stale(&fm_with(121, 600), &running(), now));
        // Starting counts as live; nothing else does.
        assert!(heartbeat_stale(
            &fm_with(999, 300),
            &DerivedStatus::AsWritten(RunStatus::Starting),
            now
        ));
        for derived in [
            crashed(),
            DerivedStatus::NotLaunched,
            DerivedStatus::LivenessUnknown {
                written: RunStatus::Running,
            },
            DerivedStatus::AsWritten(RunStatus::Parked),
            DerivedStatus::AsWritten(RunStatus::Done),
            DerivedStatus::AsWritten(RunStatus::Paused),
        ] {
            assert!(
                !heartbeat_stale(&fm_with(99_999, 300), &derived, now),
                "{derived:?} is not live, so it is never judged stale"
            );
        }
        // No stamp at all: nothing to age.
        let (_, none) = entry_edit("20261020-a", running(), |fm| {
            for conn in &mut fm.connected {
                conn.last_heartbeat = None;
            }
        });
        assert!(!heartbeat_stale(
            &none.unwrap().frontmatter,
            &running(),
            now
        ));
    }

    #[test]
    fn the_heartbeat_comes_from_the_driver_entry_else_the_newest_stamp_tagged_launcher_start() {
        let (_, e) = entry_edit("20261020-a", running(), |fm| {
            fm.connected[0].last_heartbeat = Some("2026-10-20T10:00:05Z".into()); // launcher
            fm.connected[1].last_heartbeat = Some("2026-10-20T10:00:01Z".into());
            // driver, OLDER
        });
        let fm = e.unwrap().frontmatter;
        assert_eq!(
            heartbeat_source(&fm),
            Some(("2026-10-20T10:00:01Z".into(), HeartbeatSource::Driver)),
            "the driver's own stamp wins even when the launcher's is newer"
        );
        let (_, e) = entry_edit("20261020-a", running(), |fm| {
            fm.connected[0].last_heartbeat = Some("2026-10-20T10:00:05Z".into());
            fm.connected[1].last_heartbeat = None; // the driver has not beaten yet
        });
        assert_eq!(
            heartbeat_source(&e.unwrap().frontmatter),
            Some((
                "2026-10-20T10:00:05Z".into(),
                HeartbeatSource::LauncherStart
            ))
        );
        let (_, e) = entry_edit("20261020-a", running(), |fm| {
            fm.connected.clear();
        });
        assert_eq!(heartbeat_source(&e.unwrap().frontmatter), None);
    }

    fn rendered_rows(entry: DriverRunEntry, width: u16, height: u16) -> Vec<String> {
        render(cache_of(vec![entry]), width, height).0
    }

    #[test]
    fn the_full_header_has_six_rows_with_the_ui_spec_content() {
        let entry = entry_edit("20261020-driver-tab", running(), |fm| {
            fm.status_reason = Some("driver dispatched I-03".into());
            fm.control = Some("run".into());
            fm.sandbox = Some("degraded".into());
            fm.sandbox_missing = Some(vec!["bwrap".into(), "systemd-run".into()]);
            fm.prompt_version = Some("custom:3fa9c0d1".into());
            fm.current_item = Some("I-03".into());
            fm.current_phase = Some("30".into());
            fm.current_step = Some("execute".into());
            fm.last_event = crate::state_reader::driver_frontmatter::LastEvent {
                at: Some("2026-10-20T14:02:11Z".into()),
                item: Some("I-03".into()),
                event: Some("dispatched".into()),
                summary: Some("plan 30-02".into()),
            };
            fm.connected[1].last_heartbeat = Some(stamp_ago(42));
        });
        let rows = rendered_rows(entry, 140, 30);
        let find = |needle: &str| {
            row_with(&rows, needle)
                .split('\u{2502}')
                .next_back()
                .unwrap()
                .trim_end()
                .to_owned()
        };
        assert!(find("reason").contains("driver dispatched I-03"));
        // No body detail yet: the body has no control line.
        let control = find("control ");
        assert!(control.contains("(no control line)"), "{control}");
        assert!(control.contains("heartbeat 0:4"), "{control}");
        assert!(control.contains(" ago"), "{control}");
        assert_eq!(
            find("sandbox"),
            "  sandbox UNSANDBOXED missing: bwrap, systemd-run  \u{B7}  prompt custom:3fa9c0d1"
        );
        assert_eq!(
            find("  at "),
            "  at      I-03  \u{B7}  phase 30  \u{B7}  execute"
        );
        let local = chrono::DateTime::parse_from_rfc3339("2026-10-20T14:02:11Z")
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string();
        assert_eq!(
            find("  last "),
            format!("  last    {local} I-03 dispatched: plan 30-02")
        );
    }

    #[test]
    fn empty_header_fields_render_their_named_placeholders() {
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.sandbox = None;
            fm.prompt_version = None;
            fm.current_item = None;
            fm.current_phase = None;
            fm.current_step = None;
            fm.last_event = Default::default();
            fm.connected.clear();
        });
        let rows = rendered_rows(entry, 140, 30);
        let find = |needle: &str| {
            row_with(&rows, needle)
                .split('\u{2502}')
                .next_back()
                .unwrap()
                .trim_end()
                .to_owned()
        };
        assert_eq!(
            find("sandbox"),
            "  sandbox (not recorded)  \u{B7}  prompt (not recorded)"
        );
        assert_eq!(find("  at "), "  at      (no item in progress)");
        assert_eq!(find("  last "), "  last    (no events yet)");
        assert!(find("control ").contains("heartbeat (none recorded)"));
        assert!(rows.iter().any(|r| r.contains("(none recorded)")));
    }

    #[test]
    fn a_live_run_with_an_old_heartbeat_reads_stale_with_the_threshold_and_never_crashed() {
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.heartbeat_stale_after_s = 120;
            fm.connected[1].last_heartbeat = Some(stamp_ago(190));
        });
        let rows = rendered_rows(entry, 140, 30);
        let control = row_with(&rows, "control ");
        assert!(control.contains("heartbeat 3:1"), "{control}");
        assert!(control.contains("ago stale (after 2:00)"), "{control}");
        let list = row_with(&rows, "20261020-a");
        assert!(
            list.contains("\u{25C8}"),
            "the list glyph is the stale one: {list}"
        );
        assert!(list.contains("stale"), "{list}");
        assert!(!rows.iter().any(|r| r.contains("crashed")), "{rows:#?}");
        // `stale` is Yellow and BOLD in the header.
        let (_, buffer) = render(
            cache_of(vec![entry_edit("20261020-a", running(), |fm| {
                fm.heartbeat_stale_after_s = 120;
                fm.connected[1].last_heartbeat = Some(stamp_ago(190));
            })]),
            140,
            30,
        );
        let mut found = false;
        for y in 0..30u16 {
            let line: String = (0..140u16)
                .map(|x| buffer.cell((x, y)).map_or(" ", |c| c.symbol()))
                .collect();
            if let Some(col) = line.find(" stale (after") {
                let cell = buffer.cell((u16::try_from(col + 1).unwrap(), y)).unwrap();
                assert_eq!(cell.fg, Color::Yellow);
                assert!(cell.modifier.contains(Modifier::BOLD));
                found = true;
            }
        }
        assert!(found, "no stale flag drawn");
    }

    #[test]
    fn a_run_that_is_not_live_shows_the_age_without_a_stale_judgement() {
        for derived in [
            DerivedStatus::AsWritten(RunStatus::Done),
            DerivedStatus::AsWritten(RunStatus::Paused),
        ] {
            let entry = entry_edit("20261020-a", derived, |fm| {
                fm.connected[1].last_heartbeat = Some(stamp_ago(9_000));
            });
            let rows = rendered_rows(entry, 140, 30);
            let control = row_with(&rows, "control ");
            assert!(control.contains("heartbeat 2:30:0"), "{control}");
            assert!(!control.contains("stale"), "{control}");
        }
    }

    #[test]
    fn the_launcher_start_fallback_is_labelled() {
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.connected[1].last_heartbeat = None;
            fm.connected[0].last_heartbeat = Some(stamp_ago(20));
        });
        let rows = rendered_rows(entry, 140, 30);
        let control = row_with(&rows, "control ");
        assert!(control.contains("ago (launcher start)"), "{control}");
    }

    #[test]
    fn a_body_control_the_frontmatter_has_not_echoed_is_pending_never_paused() {
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.control = Some("run".into())
        });
        let mut detail = detail_of("20261020-a", vec![], vec![], None);
        detail.body_control = Some("pause".to_owned());
        let mut cache = cache_with_detail(detail);
        cache.launcher_runs = vec![entry];
        let (rows, buffer) = render(cache, 140, 30);
        let control = row_with(&rows, "control ");
        assert!(control.contains("pause (pending)"), "{control}");
        assert!(!control.contains("paused"), "{control}");
        // `(pending)` is DarkGray.
        let y = rows
            .iter()
            .position(|r| r.contains("pause (pending)"))
            .unwrap();
        let col = rows[y].find("(pending)").unwrap();
        let cell = buffer
            .cell((
                u16::try_from(rows[y][..col].chars().count()).unwrap(),
                u16::try_from(y).unwrap(),
            ))
            .unwrap();
        assert_eq!(cell.fg, Color::DarkGray);
    }

    #[test]
    fn a_narrow_pane_cuts_the_control_word_before_the_heartbeat_flag() {
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.heartbeat_stale_after_s = 120;
            fm.connected[1].last_heartbeat = Some(stamp_ago(400));
        });
        let mut detail = detail_of("20261020-a", vec![], vec![], None);
        detail.body_control = Some("pause".to_owned());
        let mut cache = cache_with_detail(detail);
        cache.launcher_runs = vec![entry];
        let entry = cache.launcher_runs[0].clone();
        let lines = header_rows(
            &entry,
            entry.1.as_ref().expect("readable"),
            Some("pause"),
            SystemTime::now(),
            52,
        );
        let row = text_of(&lines[2]);
        assert!(Span::raw(row.as_str()).width() <= 52, "{row:?}");
        assert!(
            row.contains("stale"),
            "the flag survives a narrow pane: {row:?}"
        );
    }

    #[test]
    fn cap_bars_use_floor_division_and_limit_zero_is_full() {
        for (used, limit, filled) in [
            (0, 10, 0),
            (1, 10, 1),
            (7, 10, 7),
            (9, 10, 9),
            (10, 10, 10),
            (47, 60, 7),
            (3, 7, 4),
            (2, 7, 2),
            (99, 10, 10),
            (5, 0, 10),
        ] {
            let bar = cap_bar(used, limit);
            assert_eq!(bar.chars().count(), CAP_BAR_CELLS);
            assert_eq!(
                bar.chars().filter(|c| *c == '\u{2588}').count(),
                filled,
                "{used}/{limit}"
            );
        }
        assert!(cap_at_limit(10, 10));
        assert!(!cap_at_limit(9, 10));
        assert!(
            cap_at_limit(0, 0),
            "limit 0 is at-limit, never a division by zero"
        );
        assert!(cap_at_limit(11, 10));
    }

    #[test]
    fn the_cap_row_arithmetic_fits_the_detail_floor() {
        // indent 2 + label 10 + gap 1 + value 9 + gap 1 + bar 10.
        assert_eq!(
            usize::from(CAP_ROW_FIXED_CELLS),
            2 + 10 + 1 + CAP_VALUE_CELLS + 1 + CAP_BAR_CELLS
        );
        const { assert!(CAP_ROW_FIXED_CELLS <= super::super::driver::DRIVER_DETAIL_MIN_CELLS) };
    }

    fn caps_text(fm: &DriverFrontmatter, width: usize) -> Vec<String> {
        caps_section(fm, width).iter().map(text_of).collect()
    }

    fn fm_with_caps(edit: impl FnOnce(&mut DriverFrontmatter)) -> DriverFrontmatter {
        let (_, entry) = entry_edit("20261020-a", running(), edit);
        entry.unwrap().frontmatter
    }

    #[test]
    fn caps_show_used_over_limit_bars_whole_minutes_and_the_gap_phase() {
        let fm = fm_with_caps(|fm| {
            fm.caps.dispatches = crate::state_reader::driver_frontmatter::Cap {
                used: 7,
                limit: Some(10),
            };
            fm.caps.wall_clock_min = crate::state_reader::driver_frontmatter::WallClockCap {
                used: 47.9,
                limit: Some(60.0),
            };
            fm.caps.gap_rounds = crate::state_reader::driver_frontmatter::GapCap {
                used: 1,
                limit: Some(2),
                phase: Some("30".into()),
            };
        });
        let text = caps_text(&fm, 60);
        assert!(text[0].starts_with("\u{2500}\u{2500} caps "));
        assert_eq!(text[1].trim_end(), "  dispatches      7/10 \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}");
        assert_eq!(text[2].trim_end(), "  minutes        47/60 \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}");
        assert_eq!(text[3].trim_end(), "  gap rounds       1/2 \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}  phase 30");
    }

    #[test]
    fn a_cap_at_its_limit_is_yellow_with_a_bold_value_and_one_below_is_plain() {
        let fm = fm_with_caps(|fm| {
            fm.caps.dispatches = crate::state_reader::driver_frontmatter::Cap {
                used: 10,
                limit: Some(10),
            };
            fm.caps.wall_clock_min = crate::state_reader::driver_frontmatter::WallClockCap {
                used: 59.0,
                limit: Some(60.0),
            };
        });
        let lines = caps_section(&fm, 60);
        let at_limit = &lines[1];
        assert!(
            at_limit
                .spans
                .iter()
                .all(|s| s.style.fg == Some(Color::Yellow)),
            "{at_limit:?}"
        );
        assert!(
            at_limit.spans[1]
                .style
                .add_modifier
                .contains(Modifier::BOLD),
            "the value is BOLD"
        );
        let below = &lines[2];
        assert!(
            below.spans.iter().all(|s| s.style.fg.is_none()),
            "{below:?}"
        );
    }

    #[test]
    fn a_cap_with_no_limit_shows_a_dash_and_no_limit_instead_of_a_bar() {
        let fm = fm_with_caps(|fm| {
            fm.caps.dispatches = crate::state_reader::driver_frontmatter::Cap {
                used: 4,
                limit: None,
            };
        });
        let text = caps_text(&fm, 60);
        assert_eq!(text[1].trim_end(), "  dispatches       4/\u{2013} no limit");
        assert!(!text[1].contains('\u{2588}') && !text[1].contains('\u{2591}'));
        let lines = caps_section(&fm, 60);
        let no_limit = lines[1].spans.last().unwrap();
        assert_eq!(no_limit.style.fg, Some(Color::DarkGray));
        assert!(no_limit.style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn a_limit_of_zero_is_a_full_bar_in_the_at_limit_style() {
        let fm = fm_with_caps(|fm| {
            fm.caps.dispatches = crate::state_reader::driver_frontmatter::Cap {
                used: 0,
                limit: Some(0),
            };
        });
        let lines = caps_section(&fm, 60);
        assert_eq!(
            text_of(&lines[1]).matches('\u{2588}').count(),
            CAP_BAR_CELLS
        );
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn the_audit_row_flags_logged_not_equal_to_observed_and_only_that() {
        let ok = fm_with_caps(|fm| {
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 7,
                observed: 7,
                denied: 0,
            }
        });
        let lines = caps_section(&ok, 70);
        let audit = text_of(&lines[4]);
        assert!(
            audit.contains("logged 7")
                && audit.contains("observed 7")
                && audit.contains("denied 0"),
            "{audit}"
        );
        assert!(!audit.contains("MISMATCH"), "7/7 is no mismatch: {audit}");

        let bad = fm_with_caps(|fm| {
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 7,
                observed: 6,
                denied: 2,
            }
        });
        let lines = caps_section(&bad, 70);
        let audit = text_of(&lines[4]);
        assert!(audit.ends_with(" MISMATCH"), "{audit}");
        // The whole row is Red; MISMATCH is also BOLD; the denied value is Yellow.
        let last = lines[4].spans.last().unwrap();
        assert_eq!(last.style.fg, Some(Color::Red));
        assert!(last.style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(lines[4].spans[1].style.fg, Some(Color::Red));
        let denied = lines[4]
            .spans
            .iter()
            .find(|s| s.content == "2")
            .expect("the denied value");
        assert_eq!(denied.style.fg, Some(Color::Yellow));
        // Denied alone is Yellow on a clean row.
        let denied_only = fm_with_caps(|fm| {
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 3,
                observed: 3,
                denied: 1,
            }
        });
        let lines = caps_section(&denied_only, 70);
        let denied = lines[4].spans.iter().find(|s| s.content == "1").unwrap();
        assert_eq!(denied.style.fg, Some(Color::Yellow));
        assert!(!text_of(&lines[4]).contains("MISMATCH"));
    }

    #[test]
    fn mismatch_survives_the_narrowest_detail_pane() {
        let bad = fm_with_caps(|fm| {
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 12,
                observed: 11,
                denied: 3,
            }
        });
        for width in [39usize, 45, 60, 100] {
            let lines = caps_section(&bad, width);
            for line in &lines {
                assert!(line.width() <= width, "{width}: {:?}", text_of(line));
            }
            let audit = text_of(&lines[4]);
            assert!(audit.ends_with("MISMATCH"), "{width}: {audit:?}");
            // Every cap row's bar is complete at the floor.
            for row in &lines[1..4] {
                let t = text_of(row);
                assert_eq!(
                    t.matches('\u{2588}').count() + t.matches('\u{2591}').count(),
                    CAP_BAR_CELLS,
                    "{width}: {t:?}"
                );
            }
        }
    }

    #[test]
    fn connections_list_three_rows_then_a_more_row_and_omit_absent_fields() {
        let conn =
            |role: &str, pid: Option<u32>| crate::state_reader::driver_frontmatter::Connection {
                role: role.into(),
                state: "connected".into(),
                pid,
                ..Default::default()
            };
        let fm = fm_with_caps(|fm| {
            fm.connected = vec![
                conn("launcher", Some(11)),
                conn("driver", None),
                conn("driver", Some(33)),
                conn("driver", Some(44)),
            ];
            fm.connected[1].model = Some("opus".into());
            fm.connected[1].session_id = Some("1f83d5f6-8bee-4f7c".into());
            fm.connected[1].last_heartbeat = Some(stamp_ago(65));
            fm.connected[1].scope_unit = Some("unit.scope".into());
        });
        let text: Vec<String> = connected_section(&fm, SystemTime::now(), 100)
            .iter()
            .map(text_of)
            .collect();
        assert!(text[0].starts_with("\u{2500}\u{2500} connected "));
        assert_eq!(text[1].trim_end(), "  launcher  connected     pid 11");
        assert_eq!(
            text[2].trim_end(),
            "  driver    connected     opus  hb 1:05  unit.scope  1f83d5f6"
        );
        assert_eq!(text[3].trim_end(), "  driver    connected     pid 33");
        assert_eq!(text[4].trim(), "\u{2026} 1 more");
        assert_eq!(text.len(), 5, "three rows and the more row: {text:#?}");

        let none = fm_with_caps(|fm| fm.connected.clear());
        let text: Vec<String> = connected_section(&none, SystemTime::now(), 60)
            .iter()
            .map(text_of)
            .collect();
        assert_eq!(text[1].trim(), "(none recorded)");
        assert_eq!(text.len(), 2);
    }

    #[test]
    fn hostile_header_caps_and_connection_text_reaches_the_rows_sanitised() {
        let evil = "evil\u{1b}]0;pwned\u{7}\u{1b}[31m\u{202e}x";
        let entry = entry_edit("20261020-a", running(), |fm| {
            fm.status_reason = Some(evil.into());
            fm.current_item = Some(evil.into());
            fm.current_phase = Some(evil.into());
            fm.current_step = Some(evil.into());
            fm.sandbox = Some("degraded".into());
            fm.sandbox_missing = Some(vec![evil.into()]);
            fm.prompt_version = Some(evil.into());
            fm.last_event = crate::state_reader::driver_frontmatter::LastEvent {
                at: Some(evil.into()),
                item: Some(evil.into()),
                event: Some(evil.into()),
                summary: Some(evil.into()),
            };
            fm.caps.gap_rounds.phase = Some(evil.into());
            fm.connected[0].role = evil.into();
            fm.connected[0].state = evil.into();
            fm.connected[0].model = Some(evil.into());
            fm.connected[0].scope_unit = Some(evil.into());
            fm.connected[0].session_id = Some(evil.into());
        });
        let rows = rendered_rows(entry, 160, 30);
        for row in &rows {
            assert!(!row.chars().any(char::is_control), "{row:?}");
            assert!(!row.contains('\u{202e}'), "{row:?}");
        }
        assert!(
            rows.iter().filter(|r| r.contains("evil")).count() >= 5,
            "{rows:#?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_held_lock_with_an_old_heartbeat_reads_stale_and_a_free_lock_reads_crashed() {
        use crate::driver::lock::{acquire_at, run_lock_path};
        use crate::state_reader::driver_frontmatter::read_driver_run;

        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir.path().join("20261020-lock");
        std::fs::create_dir_all(&run_dir).unwrap();
        let mut fm = parse_driver_frontmatter(FIXTURE).expect("fixture");
        fm.run_id = "20261020-lock".to_owned();
        fm.status = RunStatus::Running;
        fm.launches = 1;
        fm.heartbeat_stale_after_s = 300;
        fm.last_event.summary = Some("rc=0".to_owned());
        // The fixture's own stamps are weeks old.
        std::fs::write(
            run_dir.join("DIRECTION.md"),
            format!(
                "{}\n## Direction\n\n- control: run\n\n## Items\n",
                fm.emit()
            ),
        )
        .unwrap();

        let lock = acquire_at(
            &run_lock_path(&run_dir),
            "20261020-lock",
            std::process::id(),
        )
        .expect("the test takes the run lock");
        let held = ("20261020-lock".to_owned(), read_driver_run(&run_dir));
        let rows = render(cache_of(vec![held]), 140, 30).0;
        let list = row_with(&rows, "20261020-lock");
        assert!(
            list.contains("\u{25C8} stale"),
            "held lock, old heartbeat: {list}"
        );
        assert!(row_with(&rows, "control ").contains("stale"));
        assert!(!rows.iter().any(|r| r.contains("crashed")), "{rows:#?}");

        drop(lock);
        let free = ("20261020-lock".to_owned(), read_driver_run(&run_dir));
        let rows = render(cache_of(vec![free]), 140, 30).0;
        assert!(
            row_with(&rows, "20261020-lock").contains("\u{2717} crashed"),
            "{rows:#?}"
        );
        assert!(!rows.iter().any(|r| r.contains("stale")), "{rows:#?}");
    }

    // ── Task 3: the LOG.md tail (launcher_log_) ────────────────────────────

    const LOG_RUN: &str = "20261020-log";

    /// The parts of a cache the render reads, copied (the cache is not `Clone`).
    fn copy_of(cache: &ProjectViewCache) -> ProjectViewCache {
        ProjectViewCache {
            launcher_runs: cache.launcher_runs.clone(),
            launcher_listed: cache.launcher_listed,
            launcher_selected_id: cache.launcher_selected_id.clone(),
            launcher_selected: cache.launcher_selected,
            launcher_detail: cache.launcher_detail.clone(),
            launcher_item_cursor: cache.launcher_item_cursor.clone(),
            launcher_log: cache.launcher_log.clone(),
            driver_follow: cache.driver_follow,
            driver_scroll_offset: cache.driver_scroll_offset,
            ..ProjectViewCache::default()
        }
    }

    fn log_header(at: &str, item: &str, event: &str, kv: &str) -> String {
        format!("- {at} | {item} | {event}{kv}\n")
    }

    fn template_log() -> String {
        crate::driver_templates::LOG_TEMPLATE
            .replace(crate::driver_templates::RUN_ID_TOKEN, LOG_RUN)
    }

    fn log_cache() -> ProjectViewCache {
        let mut cache = cache_of(vec![entry(LOG_RUN, running())]);
        cache.launcher_selected_id = Some(LOG_RUN.to_owned());
        cache
    }

    /// One scheduled-and-delivered refresh: read from the cache's cursor, apply.
    fn refresh_log(cache: &mut ProjectViewCache, dir: &Path) -> bool {
        let (cursor, ident) = log_cursor_for(cache, LOG_RUN);
        apply_log_read(cache, read_log_delta(dir, LOG_RUN, cursor, ident))
    }

    fn ring_texts(cache: &ProjectViewCache) -> Vec<String> {
        cache
            .launcher_log
            .ring
            .lines()
            .map(|line| String::from(line.text.shown()))
            .collect()
    }

    fn local_hms(at: &str) -> String {
        chrono::DateTime::parse_from_rfc3339(at)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string()
    }

    fn append(dir: &Path, text: &str) {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("LOG.md"))
            .unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }

    #[test]
    fn launcher_log_an_append_reads_only_the_new_bytes_and_never_a_line_twice() {
        let dir = tempfile::tempdir().unwrap();
        let first = log_header(
            "2026-10-20T14:02:11Z",
            "I-01",
            "dispatched",
            " | step=execute",
        );
        let second = log_header("2026-10-20T14:02:12Z", "I-01", "report", " | status=done");
        std::fs::write(
            dir.path().join("LOG.md"),
            format!("{}{first}{second}", template_log()),
        )
        .unwrap();
        let mut cache = log_cache();
        assert!(refresh_log(&mut cache, dir.path()));
        // The template preamble is skipped; the two events are the whole ring.
        assert_eq!(
            ring_texts(&cache),
            vec![
                format!(
                    "{} I-01 dispatched step=execute",
                    local_hms("2026-10-20T14:02:11Z")
                ),
                format!(
                    "{} I-01 report status=done",
                    local_hms("2026-10-20T14:02:12Z")
                ),
            ]
        );
        let before = cache.launcher_log.cursor.offset;
        assert_eq!(
            before,
            (template_log().len() + first.len() + second.len()) as u64
        );

        // Nothing appended: nothing new, nothing repeated.
        assert!(refresh_log(&mut cache, dir.path()));
        assert_eq!(ring_texts(&cache).len(), 2);

        append(
            dir.path(),
            &log_header("2026-10-20T14:09:40Z", "RUN", "end", " | reason=pause"),
        );
        assert!(refresh_log(&mut cache, dir.path()));
        let texts = ring_texts(&cache);
        assert_eq!(texts.len(), 3, "only the third was pushed: {texts:?}");
        assert_eq!(
            texts[2],
            format!("{} RUN end reason=pause", local_hms("2026-10-20T14:09:40Z"))
        );
    }

    #[test]
    fn launcher_log_a_detail_line_keeps_under_its_header_and_loses_its_own_indent() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "{}{}    status: done\n    next: ship\n",
            template_log(),
            log_header("2026-10-20T14:02:11Z", "I-03", "report", " | status=done")
        );
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        let texts = ring_texts(&cache);
        assert_eq!(texts[1], "status: done");
        assert_eq!(texts[2], "next: ship");
        let kinds: Vec<DriverLineKind> = cache.launcher_log.ring.lines().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                DriverLineKind::Output,
                DriverLineKind::Stderr,
                DriverLineKind::Stderr
            ]
        );
    }

    #[test]
    fn launcher_log_a_torn_last_line_waits_for_its_newline() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LOG.md"), template_log()).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        append(dir.path(), "- 2026-10-20T14:02:11Z | I-01 | disp");
        refresh_log(&mut cache, dir.path());
        assert!(ring_texts(&cache).is_empty(), "a torn line is not shown");
        let torn_at = cache.launcher_log.cursor.offset;
        append(dir.path(), "atched | step=plan\n");
        refresh_log(&mut cache, dir.path());
        let texts = ring_texts(&cache);
        assert_eq!(texts.len(), 1, "{texts:?}");
        assert!(texts[0].ends_with("I-01 dispatched step=plan"), "{texts:?}");
        assert!(cache.launcher_log.cursor.offset > torn_at);
    }

    #[test]
    fn launcher_log_a_truncation_resets_the_ring_and_reads_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let big = format!(
            "{}{}{}",
            template_log(),
            log_header("2026-10-20T14:02:11Z", "I-01", "dispatched", ""),
            log_header("2026-10-20T14:02:12Z", "I-01", "report", "")
        );
        std::fs::write(dir.path().join("LOG.md"), &big).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        assert_eq!(ring_texts(&cache).len(), 2);
        assert!(!cache.launcher_log.reset_notice);

        // Rewritten shorter, in place (the same inode).
        let small = log_header("2026-10-20T15:00:00Z", "RUN", "start", "");
        assert!(small.len() < big.len());
        std::fs::write(dir.path().join("LOG.md"), &small).unwrap();
        refresh_log(&mut cache, dir.path());
        assert!(cache.launcher_log.reset_notice);
        let texts = ring_texts(&cache);
        assert_eq!(texts.len(), 1, "the old lines are gone: {texts:?}");
        assert!(texts[0].ends_with("RUN start"));
        let (rows, _) = render(
            {
                let mut c = copy_of(&cache);
                c.driver_follow = true;
                c
            },
            140,
            40,
        );
        assert!(
            rows.iter().any(
                |r| r.contains("! LOG.md was truncated or replaced, reading it from the start")
            ),
            "{rows:#?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn launcher_log_a_replacement_is_detected_by_device_and_inode() {
        let dir = tempfile::tempdir().unwrap();
        let first = format!(
            "{}{}",
            template_log(),
            log_header("2026-10-20T14:02:11Z", "I-01", "dispatched", "")
        );
        std::fs::write(dir.path().join("LOG.md"), &first).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        let known = cache.launcher_log.ident.expect("the identity is recorded");

        // A NEW file, as long as the old one plus one event, renamed over LOG.md:
        // the length alone would read as an ordinary append.
        let replacement = format!(
            "{}{}{}",
            template_log(),
            log_header("2026-10-20T16:00:00Z", "I-09", "other", ""),
            log_header("2026-10-20T16:00:01Z", "I-09", "more", "")
        );
        assert!(replacement.len() > first.len());
        let staged = dir.path().join("LOG.md.new");
        std::fs::write(&staged, &replacement).unwrap();
        std::fs::rename(&staged, dir.path().join("LOG.md")).unwrap();
        refresh_log(&mut cache, dir.path());
        assert_ne!(cache.launcher_log.ident, Some(known));
        assert!(cache.launcher_log.reset_notice);
        let texts = ring_texts(&cache);
        assert_eq!(
            texts.len(),
            2,
            "re-read from the start, preamble dropped: {texts:?}"
        );
        assert!(texts[0].ends_with("I-09 other"));
    }

    #[test]
    fn launcher_log_a_stale_read_is_discarded() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "{}{}",
            template_log(),
            log_header("2026-10-20T14:02:11Z", "I-01", "dispatched", "")
        );
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        // Two reads scheduled from the same cursor (0): the first applies...
        let a = read_log_delta(dir.path(), LOG_RUN, TailCursor::default(), None);
        let b = read_log_delta(dir.path(), LOG_RUN, TailCursor::default(), None);
        assert!(apply_log_read(&mut cache, a));
        let shown = ring_texts(&cache).len();
        // ...the second started at 0 but the cursor has moved: discarded, no repeat.
        assert!(!apply_log_read(&mut cache, b));
        assert_eq!(ring_texts(&cache).len(), shown);
        // A read for a run that is not the selected one is dropped too.
        let mut other = read_log_delta(dir.path(), LOG_RUN, cache.launcher_log.cursor, None);
        other.run_id = "20261019-other".to_owned();
        assert!(!apply_log_read(&mut cache, other));
        // And a first read for a newly selected run must have started at 0.
        cache.launcher_selected_id = Some("20261019-new".to_owned());
        let mut fresh = read_log_delta(dir.path(), LOG_RUN, TailCursor { offset: 5 }, None);
        fresh.run_id = "20261019-new".to_owned();
        assert!(!apply_log_read(&mut cache, fresh));
        assert_eq!(
            cache.launcher_log.run_id, LOG_RUN,
            "the tail was not hijacked"
        );
    }

    #[test]
    fn launcher_log_the_shipped_template_has_no_events_and_a_missing_file_says_so() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LOG.md"), template_log()).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        assert!(ring_texts(&cache).is_empty(), "the preamble is not events");
        let (rows, _) = render(copy_of(&cache), 140, 40);
        assert!(
            rows.iter().any(|r| r.contains("No events logged yet.")),
            "{rows:#?}"
        );
        assert!(!rows
            .iter()
            .any(|r| r.contains("HUMAN-OWNED") || r.contains("TOOL-WRITTEN")));

        let empty = tempfile::tempdir().unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, empty.path());
        assert!(cache.launcher_log.missing);
        let (rows, _) = render(copy_of(&cache), 140, 40);
        assert!(
            rows.iter().any(|r| r.contains("This run has no LOG.md.")),
            "{rows:#?}"
        );
    }

    #[test]
    fn launcher_log_a_hostile_header_and_detail_line_render_escaped() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "{}{}    detail\u{1b}]0;pwned\u{7}\u{202e}x\n",
            template_log(),
            log_header(
                "2026-10-20T14:02:11Z",
                "I-01",
                "dispatched",
                " | step=a\u{1b}[2Jb\u{202e}c"
            )
        );
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        let (rows, _) = render(copy_of(&cache), 140, 40);
        for row in &rows {
            assert!(!row.chars().any(char::is_control), "{row:?}");
            assert!(!row.contains('\u{202e}'), "{row:?}");
        }
        assert!(
            rows.iter().any(|r| r.contains("I-01 dispatched step=")),
            "{rows:#?}"
        );
    }

    #[test]
    fn launcher_log_park_rules_come_only_from_the_closed_set() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "{}{}{}{}{}",
            template_log(),
            log_header(
                "2026-10-20T14:02:11Z",
                "I-01",
                "status",
                " | state=parked | rule=gap-rounds | outcome=x"
            ),
            log_header(
                "2026-10-20T14:02:12Z",
                "I-02",
                "status",
                " | state=parked | rule=bogus"
            ),
            log_header(
                "2026-10-20T14:02:13Z",
                "I-03",
                "status",
                " | state=running | rule=budget"
            ),
            log_header(
                "2026-10-20T14:02:14Z",
                "RUN",
                "status",
                " | state=parked | rule=budget"
            ),
        );
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        let rules = &cache.launcher_log.park_rules;
        assert_eq!(rules.get("I-01").map(String::as_str), Some("gap-rounds"));
        assert_eq!(rules.get("I-02"), None, "an unknown rule is ignored");
        assert_eq!(
            rules.get("I-03"),
            None,
            "only a parked status records a rule"
        );
        assert_eq!(rules.get("RUN"), None, "only item ids");
        assert_eq!(rules.len(), 1);
        // And the cursor-item row names it.
        let mut detail = detail_of(
            LOG_RUN,
            vec![body_item("I-01", "a", None, None)],
            vec![("I-01", state_entry(ItemState::Parked, 1, Some("stuck")))],
            None,
        );
        detail.run_id = LOG_RUN.to_owned();
        cache.launcher_detail = Some(Box::new(detail));
        cache.launcher_item_cursor = Some("I-01".to_owned());
        let (rows, _) = render(copy_of(&cache), 140, 40);
        assert!(
            rows.iter()
                .any(|r| r.contains("I-01 parked by gap-rounds: stuck")),
            "{rows:#?}"
        );
    }

    #[test]
    fn launcher_log_a_reset_forgets_the_old_park_rules() {
        let dir = tempfile::tempdir().unwrap();
        let first = format!(
            "{}{}",
            template_log(),
            log_header(
                "2026-10-20T14:02:11Z",
                "I-01",
                "status",
                " | state=parked | rule=budget"
            )
        );
        std::fs::write(dir.path().join("LOG.md"), &first).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        assert!(cache.launcher_log.park_rules.contains_key("I-01"));
        std::fs::write(
            dir.path().join("LOG.md"),
            log_header("2026-10-20T15:00:00Z", "RUN", "start", ""),
        )
        .unwrap();
        refresh_log(&mut cache, dir.path());
        assert!(cache.launcher_log.park_rules.is_empty());
    }

    #[test]
    fn launcher_log_more_than_the_ring_holds_shows_the_overflow_notice() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = template_log();
        for n in 0..2001 {
            text.push_str(&log_header(
                "2026-10-20T14:02:11Z",
                "I-01",
                "tick",
                &format!(" | n={n}"),
            ));
        }
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        assert_eq!(
            cache.launcher_log.ring.len(),
            crate::ui::screens::DRIVER_OUTPUT_RING_LINES
        );
        let mut shown = copy_of(&cache);
        shown.driver_follow = false;
        shown.driver_scroll_offset = 0;
        let (rows, _) = render(shown, 140, 40);
        assert!(
            rows.iter()
                .any(|r| r.contains("\u{2026} 1 earlier lines dropped (buffer holds 2000)")),
            "{rows:#?}"
        );
    }

    #[test]
    fn launcher_log_a_log_that_is_not_a_regular_file_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        // A directory named LOG.md: not a file, opened by nobody.
        std::fs::create_dir(dir.path().join("LOG.md")).unwrap();
        let read = read_log_delta(dir.path(), LOG_RUN, TailCursor::default(), None);
        assert!(read.lines.is_empty() && !read.missing && !read.reset);
        #[cfg(unix)]
        {
            let other = tempfile::tempdir().unwrap();
            let target = other.path().join("secret.txt");
            std::fs::write(
                &target,
                log_header("2026-10-20T14:02:11Z", "I-01", "leak", ""),
            )
            .unwrap();
            std::os::unix::fs::symlink(&target, other.path().join("LOG.md")).unwrap();
            let read = read_log_delta(other.path(), LOG_RUN, TailCursor::default(), None);
            assert!(
                read.lines.is_empty(),
                "a link planted at LOG.md is not followed"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn read_log_delta_skips_a_fifo_log_without_blocking() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let dir = tempfile::tempdir().unwrap();
        mkfifo(&dir.path().join("LOG.md"));
        let run = dir.path().to_path_buf();
        let cursor = TailCursor { offset: 7 };
        let read = within_5s(move || read_log_delta(&run, LOG_RUN, cursor, None));
        assert!(read.lines.is_empty());
        assert!(!read.missing, "a FIFO is not an absent log");
        assert!(!read.reset);
        assert_eq!(read.cursor, cursor, "the cursor is untouched");
        assert_eq!(read.ident, None);

        // A regular log still tails incrementally on the one handle, and a replaced
        // one (a different inode) still resets.
        let plain = tempfile::tempdir().unwrap();
        let log = plain.path().join("LOG.md");
        std::fs::write(&log, log_header("2026-10-20T14:02:11Z", "I-01", "one", "")).unwrap();
        let first = read_log_delta(plain.path(), LOG_RUN, TailCursor::default(), None);
        assert!(!first.lines.is_empty(), "{first:?}");
        assert!(first.ident.is_some());
        let again = read_log_delta(plain.path(), LOG_RUN, first.cursor, first.ident);
        assert!(again.lines.is_empty() && !again.reset);
        std::fs::remove_file(&log).unwrap();
        std::fs::write(&log, log_header("2026-10-20T14:09:00Z", "I-02", "two", "")).unwrap();
        let replaced = read_log_delta(plain.path(), LOG_RUN, first.cursor, first.ident);
        assert!(replaced.reset, "{replaced:?}");
        assert!(!replaced.lines.is_empty());
    }

    #[test]
    fn launcher_log_the_pane_reads_log_md_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("worker-output.txt"),
            log_header("2026-10-20T14:02:11Z", "I-01", "from-another-file", ""),
        )
        .unwrap();
        std::fs::write(dir.path().join("transcript.jsonl"), "{\"type\":\"x\"}\n").unwrap();
        let read = read_log_delta(dir.path(), LOG_RUN, TailCursor::default(), None);
        assert!(
            read.missing,
            "no LOG.md means missing, whatever else is in the directory"
        );
        assert!(read.lines.is_empty());
    }

    #[test]
    fn launcher_log_the_rule_shows_following_scrolled_or_not_live() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = template_log();
        for n in 0..60 {
            text.push_str(&log_header(
                "2026-10-20T14:02:11Z",
                "I-01",
                "tick",
                &format!(" | n={n}"),
            ));
        }
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());

        // Following: Green `[following]`, and the newest line is on screen.
        let mut following = copy_of(&cache);
        following.driver_follow = true;
        let (rows, buffer) = render(following, 140, 30);
        let rule = row_with(&rows, "\u{2500}\u{2500} log ");
        assert!(rule.contains("[following]"), "{rule}");
        assert!(rows.iter().any(|r| r.contains("n=59")), "{rows:#?}");
        let y = rows.iter().position(|r| r.contains("[following]")).unwrap();
        let col = rows[y].find("[following]").unwrap();
        let cell = buffer
            .cell((
                u16::try_from(rows[y][..col].chars().count()).unwrap(),
                u16::try_from(y).unwrap(),
            ))
            .unwrap();
        assert_eq!(cell.fg, Color::Green);

        // Scrolled to the top: Yellow `[scrolled +N]` and the oldest line shows.
        let mut scrolled = copy_of(&cache);
        scrolled.driver_follow = false;
        scrolled.driver_scroll_offset = 0;
        let (rows, _) = render(scrolled, 140, 30);
        let rule = row_with(&rows, "\u{2500}\u{2500} log ");
        assert!(rule.contains("[scrolled +"), "{rule}");
        assert!(!rule.contains("paused"), "{rule}");
        assert!(
            rows.iter()
                .any(|r| r.contains("n=0 ") || r.ends_with("n=0")),
            "{rows:#?}"
        );

        // A run that is not live: DarkGray `[not live]`.
        let mut dead = copy_of(&cache);
        dead.launcher_runs = vec![entry(LOG_RUN, DerivedStatus::AsWritten(RunStatus::Done))];
        let (rows, buffer) = render(dead, 140, 30);
        let rule = row_with(&rows, "\u{2500}\u{2500} log ");
        assert!(rule.contains("[not live]"), "{rule}");
        let y = rows.iter().position(|r| r.contains("[not live]")).unwrap();
        let col = rows[y].find("[not live]").unwrap();
        let cell = buffer
            .cell((
                u16::try_from(rows[y][..col].chars().count()).unwrap(),
                u16::try_from(y).unwrap(),
            ))
            .unwrap();
        assert_eq!(cell.fg, Color::DarkGray);
    }

    #[test]
    fn launcher_log_a_header_is_a_dark_gray_time_then_default_text_and_detail_is_indented() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "{}{}    status: done\n",
            template_log(),
            log_header("2026-10-20T14:02:11Z", "I-01", "report", " | status=done")
        );
        std::fs::write(dir.path().join("LOG.md"), text).unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        let lines = log_body_lines(&cache.launcher_log);
        // Header: margin, the time in DarkGray, then the rest in the default style.
        let time = local_hms("2026-10-20T14:02:11Z");
        assert_eq!(
            text_of(&lines[0]),
            format!("  {time} I-01 report status=done")
        );
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::DarkGray));
        assert_eq!(
            lines[0].spans[2].style.fg, None,
            "no colour chosen from log content"
        );
        // Detail: indented 4 cells beyond the margin, all DarkGray.
        assert_eq!(text_of(&lines[1]), "      status: done");
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn launcher_log_selecting_another_run_starts_its_tail_at_the_bottom_with_follow_on() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("LOG.md"),
            format!(
                "{}{}",
                template_log(),
                log_header("2026-10-20T14:02:11Z", "I-01", "start", "")
            ),
        )
        .unwrap();
        let mut cache = log_cache();
        refresh_log(&mut cache, dir.path());
        cache.driver_follow = false;
        cache.driver_scroll_offset = 7;

        // The user moves onto another run: its cursor starts at 0, and when the
        // read lands the pane is reset to the tail with follow on.
        cache.launcher_runs.push(entry("20261019-other", running()));
        cache.launcher_selected_id = Some("20261019-other".to_owned());
        let (cursor, ident) = log_cursor_for(&cache, "20261019-other");
        assert_eq!((cursor, ident), (TailCursor::default(), None));
        let read = read_log_delta(dir.path(), "20261019-other", cursor, ident);
        assert!(apply_log_read(&mut cache, read));
        assert_eq!(cache.launcher_log.run_id, "20261019-other");
        assert!(cache.driver_follow);
        assert_eq!(cache.driver_scroll_offset, 0);
        // Until then the pane never draws the previous run's lines under it.
        let mut waiting = log_cache();
        waiting.launcher_log = LauncherLogTail {
            run_id: "20261019-previous".into(),
            ..LauncherLogTail::default()
        };
        let (rows, _) = render(waiting, 140, 40);
        assert!(
            !rows.iter().any(|r| r.contains("No events logged yet.")),
            "{rows:#?}"
        );
    }

    // ── Dashboard markers (30-03, D-12) ────────────────────────────────────

    /// A run whose written status is `written` and whose derived status is `derived`.
    fn marker_run(id: &str, written: RunStatus, derived: DerivedStatus) -> DriverRunEntry {
        let mut run = entry(id, derived);
        run.1.as_mut().expect("readable").frontmatter.status = written;
        run
    }

    /// The same run with a heartbeat older than the file's threshold.
    fn aged(mut run: DriverRunEntry) -> DriverRunEntry {
        let fm = &mut run.1.as_mut().expect("readable").frontmatter;
        let ago = fm.heartbeat_stale_after_s + 600;
        for conn in &mut fm.connected {
            conn.last_heartbeat = Some(stamp_ago(ago));
        }
        run
    }

    /// One non-terminal run (written `running`) of each marker class.
    fn run_of(class: MarkerClass, id: &str) -> DriverRunEntry {
        let live = || marker_run(id, RunStatus::Running, running());
        match class {
            MarkerClass::Crashed => marker_run(id, RunStatus::Running, crashed()),
            MarkerClass::Parked => {
                let mut run = live();
                run.1.as_mut().expect("readable").frontmatter.item_states = ItemStates(vec![(
                    "I-01".to_owned(),
                    ItemStateEntry {
                        state: ItemState::Parked,
                        dispatches: 1,
                        outcome: None,
                    },
                )]);
                run
            }
            MarkerClass::Driven => live(),
            MarkerClass::Stale => aged(live()),
            MarkerClass::Unknown => marker_run(
                id,
                RunStatus::Running,
                DerivedStatus::LivenessUnknown {
                    written: RunStatus::Running,
                },
            ),
        }
    }

    const ALL_CLASSES: [MarkerClass; 5] = [
        MarkerClass::Crashed,
        MarkerClass::Parked,
        MarkerClass::Driven,
        MarkerClass::Stale,
        MarkerClass::Unknown,
    ];

    #[test]
    fn launcher_marker_a_free_lock_running_run_is_crashed() {
        let run = marker_run("20261020-a", RunStatus::Running, crashed());
        assert_eq!(
            run_marker_class(&run, SystemTime::now()),
            Some(MarkerClass::Crashed)
        );
        let marker = launcher_marker(&[run], SystemTime::now());
        assert_eq!(marker.class, Some(MarkerClass::Crashed));
        assert!(!marker.any_live, "a crashed run does not hold its lock");
        assert!(marker.needs_attention());
    }

    #[test]
    fn launcher_marker_over_no_runs_is_no_class() {
        let marker = launcher_marker(&[], SystemTime::now());
        assert_eq!(marker, LauncherMarker::default());
        assert!(!marker.needs_attention());
    }

    #[test]
    fn launcher_marker_each_class_comes_from_its_own_run_shape() {
        let now = SystemTime::now();
        for class in ALL_CLASSES {
            assert_eq!(
                run_marker_class(&run_of(class, "20261020-a"), now),
                Some(class),
                "{class:?}"
            );
        }
        // The launcher's own `crashed` / `parked` words are classes too.
        let written_crashed = marker_run(
            "20261020-a",
            RunStatus::Crashed,
            DerivedStatus::AsWritten(RunStatus::Crashed),
        );
        assert_eq!(
            run_marker_class(&written_crashed, now),
            Some(MarkerClass::Crashed)
        );
        let written_parked = marker_run(
            "20261020-a",
            RunStatus::Parked,
            DerivedStatus::AsWritten(RunStatus::Parked),
        );
        assert_eq!(
            run_marker_class(&written_parked, now),
            Some(MarkerClass::Parked)
        );
    }

    /// Every pair of classes across two runs yields the higher one, and the order of
    /// the runs does not matter.
    #[test]
    fn launcher_marker_precedence_truth_table() {
        let now = SystemTime::now();
        for a in ALL_CLASSES {
            for b in ALL_CLASSES {
                let runs = [run_of(a, "20261020-b"), run_of(b, "20261020-a")];
                // Crashed > parked > driven > stale > unknown: the earlier in
                // `ALL_CLASSES` wins.
                let expected = ALL_CLASSES
                    .iter()
                    .find(|class| **class == a || **class == b)
                    .copied();
                assert_eq!(launcher_marker(&runs, now).class, expected, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn launcher_marker_a_stale_live_run_is_stale_not_driven() {
        let now = SystemTime::now();
        let run = run_of(MarkerClass::Stale, "20261020-a");
        assert_eq!(run_marker_class(&run, now), Some(MarkerClass::Stale));
        let marker = launcher_marker(&[run], now);
        assert_eq!(marker.class, Some(MarkerClass::Stale));
        assert!(marker.any_live, "stale is still a live lock");
        assert!(marker.needs_attention());
        // Never crashed: staleness is not a crash.
        assert_ne!(marker.class, Some(MarkerClass::Crashed));
    }

    #[test]
    fn launcher_marker_liveness_unknown_alone_is_unknown_and_not_attention() {
        let marker = launcher_marker(
            &[run_of(MarkerClass::Unknown, "20261020-a")],
            SystemTime::now(),
        );
        assert_eq!(marker.class, Some(MarkerClass::Unknown));
        assert!(!marker.any_live);
        assert!(!marker.needs_attention(), "unknown never counts");
    }

    #[test]
    fn launcher_marker_a_driven_run_is_not_an_attention_flag() {
        let marker = launcher_marker(
            &[run_of(MarkerClass::Driven, "20261020-a")],
            SystemTime::now(),
        );
        assert_eq!(marker.class, Some(MarkerClass::Driven));
        assert!(marker.any_live);
        assert!(!marker.needs_attention());
    }

    #[test]
    fn launcher_marker_a_not_launched_run_and_a_paused_run_have_no_class() {
        let now = SystemTime::now();
        let fresh = marker_run(
            "20261020-a",
            RunStatus::Starting,
            DerivedStatus::NotLaunched,
        );
        let paused = marker_run(
            "20261020-b",
            RunStatus::Paused,
            DerivedStatus::AsWritten(RunStatus::Paused),
        );
        assert_eq!(run_marker_class(&fresh, now), None);
        assert_eq!(run_marker_class(&paused, now), None);
        // Both are considered (not terminal) and neither flags anything.
        assert_eq!(
            launcher_marker(&[fresh, paused], now),
            LauncherMarker::default()
        );
    }

    #[test]
    fn launcher_marker_a_terminal_done_most_recent_run_has_no_class() {
        let done = marker_run(
            "20261020-a",
            RunStatus::Done,
            DerivedStatus::AsWritten(RunStatus::Done),
        );
        assert_eq!(
            launcher_marker(&[done], SystemTime::now()),
            LauncherMarker::default()
        );
    }

    #[test]
    fn launcher_marker_a_non_terminal_older_run_behind_a_newer_done_run_is_still_considered() {
        let now = SystemTime::now();
        let newest = marker_run(
            "20261020-b",
            RunStatus::Done,
            DerivedStatus::AsWritten(RunStatus::Done),
        );
        let older = run_of(MarkerClass::Crashed, "20261020-a");
        assert_eq!(
            launcher_marker(&[newest, older], now).class,
            Some(MarkerClass::Crashed)
        );
    }

    #[test]
    fn launcher_marker_only_the_most_recent_terminal_run_is_considered() {
        let now = SystemTime::now();
        let written_crashed = || {
            marker_run(
                "20261020-a",
                RunStatus::Crashed,
                DerivedStatus::AsWritten(RunStatus::Crashed),
            )
        };
        let done = || {
            marker_run(
                "20261020-b",
                RunStatus::Done,
                DerivedStatus::AsWritten(RunStatus::Done),
            )
        };
        // Behind a newer finished run, a long-finished crash is history.
        assert_eq!(
            launcher_marker(&[done(), written_crashed()], now),
            LauncherMarker::default()
        );
        // As the most recent run, the launcher's own `crashed` is a class.
        assert_eq!(
            launcher_marker(&[written_crashed()], now).class,
            Some(MarkerClass::Crashed)
        );
    }

    #[test]
    fn launcher_marker_any_live_follows_running_and_starting_only() {
        let now = SystemTime::now();
        let starting = marker_run(
            "20261020-a",
            RunStatus::Starting,
            DerivedStatus::AsWritten(RunStatus::Starting),
        );
        assert!(launcher_marker(&[starting], now).any_live);
        for class in [MarkerClass::Crashed, MarkerClass::Unknown] {
            assert!(
                !launcher_marker(&[run_of(class, "20261020-a")], now).any_live,
                "{class:?}"
            );
        }
    }

    #[test]
    fn launcher_marker_an_unreadable_run_has_no_class() {
        let bad: DriverRunEntry = (
            "20261020-a".to_owned(),
            Err(FrontmatterError::Io {
                detail: "boom".to_owned(),
            }),
        );
        assert_eq!(run_marker_class(&bad, SystemTime::now()), None);
        assert_eq!(
            launcher_marker(&[bad], SystemTime::now()),
            LauncherMarker::default()
        );
    }

    // ── Task 3: height tiers, status tokens, SUMMARY rows (30-03, D-10, D-29) ──

    #[test]
    fn the_height_tier_follows_the_ui_spec_boundaries() {
        for (h, tier) in [
            (0, HeightTier::None),
            (1, HeightTier::Summary),
            (11, HeightTier::Summary),
            (12, HeightTier::Medium),
            (20, HeightTier::Medium),
            (21, HeightTier::Full),
            (40, HeightTier::Full),
        ] {
            assert_eq!(height_tier(h), tier, "H = {h}");
        }
        // The constants are the detail INNER heights: the tab area is one row taller
        // (the pane title).
        assert_eq!(DRIVER_FULL_MIN_ROWS, 20);
        assert_eq!(DRIVER_MEDIUM_MIN_ROWS, 11);
        assert_eq!(height_tier(DRIVER_FULL_MIN_ROWS + 1), HeightTier::Full);
        assert_eq!(height_tier(DRIVER_FULL_MIN_ROWS), HeightTier::Medium);
        assert_eq!(height_tier(DRIVER_MEDIUM_MIN_ROWS + 1), HeightTier::Medium);
        assert_eq!(height_tier(DRIVER_MEDIUM_MIN_ROWS), HeightTier::Summary);
    }

    #[test]
    fn allocation_gives_every_section_its_floor_at_the_tier_minimum() {
        // FULL at 20 inner rows: header 6 + items 3 + caps 5 + connected 2 + log 4.
        let full = allocate_rows(HeightTier::Full, 20, 11, 5);
        assert_eq!(
            full,
            TierRows {
                items: 3,
                connected: 2,
                log: 4
            }
        );
        assert_eq!(6 + full.items + 5 + full.connected + full.log, 20);
        // MEDIUM at 11: header 2 + caps 3 + items 2 + log 4.
        let medium = allocate_rows(HeightTier::Medium, 11, 11, 0);
        assert_eq!(
            medium,
            TierRows {
                items: 2,
                connected: 0,
                log: 4
            }
        );
        assert_eq!(2 + 3 + medium.items + medium.log, 11);
    }

    #[test]
    fn allocation_spare_rows_go_to_items_then_connections_then_the_log() {
        // Five spare rows: all to the items (which can take eight more).
        let a = allocate_rows(HeightTier::Full, 25, 11, 5);
        assert_eq!((a.items, a.connected, a.log), (8, 2, 4));
        // Items satisfied at 4 rows, then the connections up to their demand, then
        // every remaining row to the LOG.
        let b = allocate_rows(HeightTier::Full, 30, 4, 5);
        assert_eq!((b.items, b.connected, b.log), (4, 5, 10));
        // A section with nothing to show holds no rows back.
        let c = allocate_rows(HeightTier::Full, 20, 2, 2);
        assert_eq!((c.items, c.connected), (2, 2));
        assert_eq!(6 + c.items + 5 + c.connected + c.log, 20);
        // MEDIUM: items first, the rest to the LOG.
        let d = allocate_rows(HeightTier::Medium, 19, 9, 0);
        assert_eq!((d.items, d.log), (9, 5));
    }

    #[test]
    fn allocation_always_adds_up_and_never_starves_the_log() {
        for inner in 20..=60usize {
            for items in [2usize, 3, 4, 9, 12] {
                for conn in [2usize, 3, 5] {
                    let r = allocate_rows(HeightTier::Full, inner, items, conn);
                    assert_eq!(6 + r.items + 5 + r.connected + r.log, inner);
                    assert!(r.log >= LOG_FLOOR_ROWS, "{inner} {items} {conn}: {r:?}");
                    assert!(r.items <= items && r.connected <= conn);
                }
            }
        }
        for inner in 11..=60usize {
            for items in [2usize, 3, 9, 12] {
                let r = allocate_rows(HeightTier::Medium, inner, items, 0);
                assert_eq!(2 + 3 + r.items + r.log, inner);
                assert!(r.log >= LOG_FLOOR_ROWS, "{inner} {items}: {r:?}");
            }
        }
    }

    /// A readable running run with every status token present: a stale heartbeat, a
    /// pending `pause`, an audit mismatch, a degraded sandbox, a cap at its limit and
    /// a custom prompt.
    fn token_entry(id: &str, mismatch: bool) -> DriverRunEntry {
        entry_edit(id, running(), |fm| {
            fm.control = Some("run".to_owned());
            fm.heartbeat_stale_after_s = 120;
            for conn in &mut fm.connected {
                conn.last_heartbeat = Some(stamp_ago(190));
            }
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 7,
                observed: if mismatch { 6 } else { 7 },
                denied: 0,
            };
            fm.sandbox = Some("degraded".to_owned());
            fm.caps.dispatches.used = 10;
            fm.caps.dispatches.limit = Some(10);
            fm.prompt_version = Some("custom:3fa9c0d1".to_owned());
        })
    }

    fn token_detail(id: &str) -> RunDetail {
        let mut detail = detail_of(id, vec![], vec![], None);
        detail.body_control = Some("pause".to_owned());
        detail
    }

    fn spans_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|span| span.content.as_ref()).collect()
    }

    #[test]
    fn the_status_tokens_follow_the_ui_spec_order_and_styles() {
        let entry = token_entry("20261020-a", true);
        let detail = token_detail("20261020-a");
        let spans = status_tokens(&entry, Some(&detail), SystemTime::now(), usize::MAX);
        assert_eq!(
            spans_text(&spans),
            "ctl pause (pending)  hb 3:10 stale  audit 7/6/0 MISMATCH  UNSANDBOXED  cap!  \
             prompt custom"
                .replace("3:10", &format_age(190))
        );
        let style_of = |needle: &str| {
            spans
                .iter()
                .find(|span| span.content.contains(needle))
                .map(|span| span.style)
                .unwrap_or_else(|| panic!("no span with {needle:?}"))
        };
        assert_eq!(style_of(" stale").fg, Some(Color::Yellow));
        assert!(style_of(" stale").add_modifier.contains(Modifier::BOLD));
        assert_eq!(style_of("MISMATCH").fg, Some(Color::Red));
        assert!(style_of("MISMATCH").add_modifier.contains(Modifier::BOLD));
        assert_eq!(style_of("UNSANDBOXED").fg, Some(Color::Yellow));
        assert_eq!(style_of("cap!").fg, Some(Color::Yellow));
        assert_eq!(style_of("(pending)").fg, Some(Color::DarkGray));
        assert_eq!(style_of("prompt custom").fg, Some(Color::Yellow));

        // A matching audit is the benign DarkGray form, after `cap!`.
        let ok = token_entry("20261020-b", false);
        let spans = status_tokens(&ok, None, SystemTime::now(), usize::MAX);
        let text = spans_text(&spans);
        assert!(
            text.contains("  cap!  audit 7/7/0  prompt custom"),
            "{text}"
        );
        assert!(!text.contains("MISMATCH"));
        assert_eq!(
            spans
                .iter()
                .find(|s| s.content == "audit 7/7/0")
                .unwrap()
                .style
                .fg,
            Some(Color::DarkGray)
        );
    }

    #[test]
    fn the_status_tokens_are_dropped_whole_benign_first_and_never_split() {
        let entry = token_entry("20261020-a", true);
        let detail = token_detail("20261020-a");
        let full = spans_text(&status_tokens(
            &entry,
            Some(&detail),
            SystemTime::now(),
            usize::MAX,
        ));
        let whole: Vec<String> = full.split("  ").map(str::to_owned).collect();
        assert_eq!(whole.len(), 6, "{whole:?}");

        let mut previous: Option<Vec<String>> = None;
        for cells in (0..=full.chars().count() + 2).rev() {
            let spans = status_tokens(&entry, Some(&detail), SystemTime::now(), cells);
            let text = spans_text(&spans);
            assert!(
                Span::raw(text.as_str()).width() <= cells,
                "{cells}: {text:?}"
            );
            let kept: Vec<String> = if text.is_empty() {
                Vec::new()
            } else {
                text.split("  ").map(str::to_owned).collect()
            };
            // Never split: every kept piece is a whole token, and the order holds.
            let positions: Vec<usize> = kept
                .iter()
                .map(|piece| {
                    whole
                        .iter()
                        .position(|w| w == piece)
                        .unwrap_or_else(|| panic!("{cells}: {piece:?} is a split token"))
                })
                .collect();
            assert!(
                positions.windows(2).all(|w| w[0] < w[1]),
                "{cells}: {kept:?}"
            );
            // Shrinking only ever removes tokens.
            if let Some(previous) = &previous {
                assert!(kept.iter().all(|k| previous.contains(k)), "{cells}");
            }
            // A flag outlives a benign token: if a flag is gone, every benign one is.
            let flags = ["hb ", "audit 7/6/0 MISMATCH", "UNSANDBOXED", "cap!"];
            let is_flag = |token: &str| flags.iter().any(|f| token.starts_with(f));
            let dropped_flag = whole.iter().any(|t| is_flag(t) && !kept.contains(t));
            if dropped_flag {
                assert!(
                    kept.iter().all(|t| is_flag(t)),
                    "{cells}: a benign token outlived a dropped flag: {kept:?}"
                );
            }
            previous = Some(kept);
        }
        // Specifically: at 40 cells MISMATCH and UNSANDBOXED outlive `prompt custom`
        // and the benign audit.
        let at_40 = spans_text(&status_tokens(&entry, Some(&detail), SystemTime::now(), 40));
        assert!(at_40.contains("MISMATCH"), "{at_40:?}");
        assert!(!at_40.contains("prompt custom"), "{at_40:?}");
        let ok = token_entry("20261020-b", false);
        let at_30 = spans_text(&status_tokens(&ok, None, SystemTime::now(), 30));
        assert!(at_30.contains("UNSANDBOXED"), "{at_30:?}");
        assert!(!at_30.contains("audit 7/7/0") && !at_30.contains("prompt custom"));
    }

    #[test]
    fn the_status_tokens_for_a_quiet_run_are_the_control_and_the_audit_only() {
        let quiet = entry_edit("20261020-a", running(), |fm| {
            fm.control = Some("run".to_owned());
        });
        let text = spans_text(&status_tokens(&quiet, None, SystemTime::now(), usize::MAX));
        assert!(text.starts_with("ctl run  hb 0:0"), "{text}");
        assert!(
            text.ends_with("  audit 7/7/0") || text.contains("  audit "),
            "{text}"
        );
        assert!(!text.contains("UNSANDBOXED") && !text.contains("cap!"));
        // No control anywhere and no detail: no `ctl` claim at all.
        let none = entry_edit("20261020-a", running(), |fm| fm.control = None);
        let text = spans_text(&status_tokens(&none, None, SystemTime::now(), usize::MAX));
        assert!(!text.contains("ctl"), "{text}");
        // An unreadable run has no tokens.
        let bad: DriverRunEntry = (
            "20261020-a".to_owned(),
            Err(FrontmatterError::Io { detail: "x".into() }),
        );
        assert!(status_tokens(&bad, None, SystemTime::now(), usize::MAX).is_empty());
    }

    #[test]
    fn summary_row_is_highlight_glyph_word_id_and_the_tokens_that_fit() {
        let entry = entry_edit("20261020-driver-tab", running(), |fm| {
            fm.control = Some("run".to_owned());
        });
        let mut detail = detail_of("20261020-driver-tab", vec![], vec![], None);
        detail.body_control = Some("pause".to_owned());
        let now = SystemTime::now();
        let line = summary_row(&entry, Some(&detail), true, now, 100);
        let text = text_of(&line);
        assert!(
            text.starts_with("> \u{25C6} running    20261020-driver-tab  ctl pause (pending)"),
            "{text:?}"
        );
        assert_eq!(
            line.spans[0].style.fg,
            Some(Color::Cyan),
            "the highlight is Selection"
        );
        assert_eq!(
            line.spans[1].style.fg,
            Some(Color::Magenta),
            "the glyph keeps the status colour"
        );
        let plain = text_of(&summary_row(&entry, None, false, now, 100));
        assert!(
            plain.starts_with("  \u{25C6} running    20261020-driver-tab  ctl run"),
            "{plain:?}"
        );

        // Never wider than the row, at any width; the id yields before a flag does.
        let flagged = token_entry("20261020-driver-tab", true);
        for width in 12..=120 {
            let text = text_of(&summary_row(&flagged, None, false, now, width));
            assert!(
                Span::raw(text.as_str()).width() <= width,
                "{width}: {text:?}"
            );
        }
        let wide = text_of(&summary_row(&flagged, None, false, now, 90));
        assert!(
            wide.contains("MISMATCH") && wide.contains("UNSANDBOXED"),
            "{wide:?}"
        );
        let narrow = text_of(&summary_row(&flagged, None, false, now, 40));
        assert!(
            narrow.contains("20261020-dr"),
            "the id keeps its floor: {narrow:?}"
        );
    }

    #[test]
    fn summary_row_cuts_the_id_by_display_width_and_reads_unreadable_plainly() {
        let wide = "\u{4E2D}\u{6587}".repeat(20);
        let entry = entry(&wide, running());
        for width in [20usize, 33, 47] {
            let text = text_of(&summary_row(&entry, None, false, SystemTime::now(), width));
            assert!(
                Span::raw(text.as_str()).width() <= width,
                "{width}: {text:?}"
            );
        }
        let bad: DriverRunEntry = (
            "20261020-a".to_owned(),
            Err(FrontmatterError::Io { detail: "x".into() }),
        );
        let text = text_of(&summary_row(&bad, None, false, SystemTime::now(), 60));
        assert!(
            text.contains("unreadable") && text.contains("20261020-a"),
            "{text:?}"
        );
    }

    // ── The tiered composition, rendered ───────────────────────────────────

    /// The shared fixture of the sweeps: a launcher run with ten items, four
    /// connections, a stale heartbeat, an audit mismatch, a degraded sandbox, a cap at
    /// its limit and fifty LOG lines, beside two other runs.
    const SWEEP_RUN: &str = "20261020-driver-tab";

    fn sweep_cache(dir: &Path) -> ProjectViewCache {
        let selected = entry_edit(SWEEP_RUN, running(), |fm| {
            fm.control = Some("run".to_owned());
            fm.heartbeat_stale_after_s = 120;
            for conn in &mut fm.connected {
                conn.last_heartbeat = Some(stamp_ago(900));
            }
            while fm.connected.len() < 4 {
                let mut extra = fm.connected[0].clone();
                extra.role = format!("worker-{}", fm.connected.len());
                fm.connected.push(extra);
            }
            fm.dispatch_audit = crate::state_reader::driver_frontmatter::DispatchAudit {
                logged: 7,
                observed: 6,
                denied: 1,
            };
            fm.sandbox = Some("degraded".to_owned());
            fm.sandbox_missing = Some(vec!["bwrap".to_owned()]);
            fm.caps.dispatches.used = 10;
            fm.caps.dispatches.limit = Some(10);
            fm.caps.gap_rounds.phase = Some("30".to_owned());
            fm.prompt_version = Some("custom:3fa9c0d1".to_owned());
            fm.current_item = Some("I-03".to_owned());
        });
        let items: Vec<BodyItem> = (1..=10)
            .map(|n| {
                body_item(
                    &format!("I-{n:02}"),
                    &format!("Item number {n} of the run"),
                    None,
                    None,
                )
            })
            .collect();
        let states: Vec<(String, ItemStateEntry)> = vec![
            (
                "I-01".to_owned(),
                state_entry(ItemState::Done, 2, Some("shipped")),
            ),
            ("I-02".to_owned(), state_entry(ItemState::Done, 1, None)),
            (
                "I-03".to_owned(),
                state_entry(ItemState::Parked, 2, Some("which socket path?")),
            ),
        ];
        let mut detail = detail_of(SWEEP_RUN, items, vec![], Some("I-03"));
        detail.view = selected.1.clone().expect("readable");
        detail.view.frontmatter.item_states = ItemStates(states);
        detail.body_control = Some("pause".to_owned());
        let runs = vec![
            selected,
            entry("20261019-older", crashed()),
            entry("20261018-oldest", DerivedStatus::AsWritten(RunStatus::Done)),
        ];
        let mut cache = cache_of(runs);
        cache.launcher_selected_id = Some(SWEEP_RUN.to_owned());
        cache.launcher_detail = Some(Box::new(detail));
        let mut log = template_log_for(SWEEP_RUN);
        for n in 0..50 {
            log.push_str(&log_header(
                "2026-10-20T14:02:11Z",
                "I-03",
                "dispatched",
                &format!(" | step=execute n={n}"),
            ));
        }
        std::fs::write(dir.join("LOG.md"), log).expect("LOG.md");
        let (cursor, ident) = log_cursor_for(&cache, SWEEP_RUN);
        apply_log_read(&mut cache, read_log_delta(dir, SWEEP_RUN, cursor, ident));
        cache
    }

    fn template_log_for(run_id: &str) -> String {
        crate::driver_templates::LOG_TEMPLATE.replace(crate::driver_templates::RUN_ID_TOKEN, run_id)
    }

    /// The same fixture with a legacy run expanded and selected instead.
    fn legacy_sweep_cache(dir: &Path) -> ProjectViewCache {
        let mut cache = sweep_cache(dir);
        cache.driver_runs = vec![crate::journal::RunSummary {
            run_id: "2026-07-20T09-00-00Z-beef".to_owned(),
            started_at: "2026-07-20T09:00:00Z".to_owned(),
            ended_at: None,
            goal: "a goal long enough to need a row of its own in the header".to_owned(),
            gsd_command: "/gsd-execute-phase 3".to_owned(),
            outcome: None,
        }];
        cache.legacy_expanded = true;
        cache.legacy_row_selected = true;
        cache.driver_selected_run = 0;
        cache
    }

    /// The detail pane's text per tab row: every cell right of the list pane.
    fn detail_rows(buffer: &Buffer, width: u16, height: u16) -> Vec<String> {
        let (_, detail) = driver_panes(Rect::new(0, 0, width, height));
        (0..height)
            .map(|y| {
                (detail.x..width)
                    .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                    .collect::<String>()
            })
            .collect()
    }

    /// D-29: no row that is a section rule or the pane's title header is the last row
    /// of the area, or is followed by a rule or an empty row; a LOG rule has its
    /// minimum body under it.
    fn assert_no_orphans(rows: &[String], height: u16, label: &str) {
        let tier = height_tier(height);
        let titled = matches!(tier, HeightTier::Medium | HeightTier::Full);
        for (y, row) in rows.iter().enumerate() {
            let row = row.trim_end();
            let is_rule = row.starts_with("\u{2500}\u{2500}");
            let is_title = y == 0 && titled;
            if !(is_rule || is_title) {
                continue;
            }
            let next = rows.get(y + 1).map(|r| r.trim_end());
            assert!(
                next.is_some_and(|n| !n.trim().is_empty() && !n.starts_with("\u{2500}\u{2500}")),
                "{label}: the {} at row {y} has no content under it:\n{}",
                if is_rule { "rule" } else { "title" },
                rows.join("\n")
            );
            if row.starts_with("\u{2500}\u{2500} log ") {
                assert!(
                    y + LOG_MIN_ROWS < rows.len(),
                    "{label}: the LOG rule at row {y} has fewer than {LOG_MIN_ROWS} rows:\n{}",
                    rows.join("\n")
                );
            }
        }
        assert!(
            !rows
                .last()
                .is_some_and(|r| r.trim_end().starts_with("\u{2500}\u{2500}")),
            "{label}: the bottom row is a rule:\n{}",
            rows.join("\n")
        );
    }

    fn sweep_render(cache: ProjectViewCache, width: u16, height: u16) -> Vec<String> {
        let (_, buffer) = render(cache, width, height);
        detail_rows(&buffer, width, height)
    }

    /// The backstop for the launcher layout: at no height from 1 to 40 and no width
    /// of 40, 60, 80 or 120 does a section rule or header render without content
    /// directly under it, and the bottom row is never a rule.
    #[test]
    fn launcher_orphan_sweep_no_rule_or_header_without_content_at_any_height() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut rules_seen = 0usize;
        for width in [40u16, 60, 80, 120] {
            for height in 1..=40u16 {
                let rows = sweep_render(sweep_cache(dir.path()), width, height);
                assert_no_orphans(&rows, height, &format!("launcher W={width} H={height}"));
                rules_seen += rows
                    .iter()
                    .filter(|r| r.starts_with("\u{2500}\u{2500}"))
                    .count();
            }
        }
        // The sweep is not vacuous: the taller heights did draw rules.
        assert!(rules_seen > 100, "{rules_seen}");
    }

    /// The same for a selected legacy row, whose detail drops the pipeline row and
    /// the steps section together between 8 and 13 rows (Phase 18 WR-05).
    #[test]
    fn legacy_orphan_sweep_no_rule_or_header_without_content_at_any_height() {
        let dir = tempfile::tempdir().expect("temp dir");
        for width in [40u16, 60, 80, 120] {
            for height in 1..=40u16 {
                let rows = sweep_render(legacy_sweep_cache(dir.path()), width, height);
                assert_no_orphans(&rows, height, &format!("legacy W={width} H={height}"));
                if (12..=14).contains(&height) {
                    assert!(
                        !rows.iter().any(|r| r.contains("steps")),
                        "legacy W={width} H={height}: the steps section is dropped with the pipeline row:\n{}",
                        rows.join("\n")
                    );
                }
            }
        }
    }

    fn rule_names(rows: &[String]) -> Vec<String> {
        rows.iter()
            .filter_map(|r| r.strip_prefix("\u{2500}\u{2500} "))
            .filter_map(|r| r.split_whitespace().next())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_tiers_draw_the_sections_the_ui_spec_names() {
        let dir = tempfile::tempdir().expect("temp dir");
        // FULL (H >= 21): items, caps, connected, log.
        let rows = sweep_render(sweep_cache(dir.path()), 100, 21);
        assert_eq!(
            rule_names(&rows),
            ["items", "caps", "connected", "log"],
            "{rows:#?}"
        );
        // MEDIUM (12..=20): no caps or connected rule; the caps are self-labelled rows
        // between the tokens and the items, and the cursor-item row is gone.
        let rows = sweep_render(sweep_cache(dir.path()), 200, 20);
        assert_eq!(rule_names(&rows), ["items", "log"], "{rows:#?}");
        let at = |needle: &str| {
            rows.iter()
                .position(|r| r.contains(needle))
                .unwrap_or_else(|| panic!("{needle}: {rows:#?}"))
        };
        assert!(at("ctl ") < at("dispatches"));
        assert!(at("dispatches") < at("minutes") && at("minutes") < at("gap rounds"));
        assert!(at("gap rounds") < at("\u{2500}\u{2500} items"));
        assert!(!rows
            .iter()
            .any(|r| r.contains("answer:") || r.contains("I-03 parked")));
        assert!(
            !rows.iter().any(|r| r.contains("audit  ")),
            "no audit row in MEDIUM"
        );
        // SUMMARY (H <= 11): one row per run across the whole width, no sections.
        let (rows, _) = render(sweep_cache(dir.path()), 100, 11);
        assert!(
            rows[0].starts_with("> \u{25C6}") || rows[0].starts_with("> \u{25C8}"),
            "{rows:#?}"
        );
        assert!(rows[0].contains(SWEEP_RUN), "{rows:#?}");
        assert!(rows[1].contains("20261019-older") && rows[1].contains("crashed"));
        assert!(rows[2].contains("20261018-oldest") && rows[2].contains("done"));
        assert!(!rows
            .iter()
            .any(|r| r.contains("\u{2500}\u{2500}") || r.contains("Runs")));
        // H == 0 draws nothing at all.
        let regions = {
            let dir2 = tempfile::tempdir().expect("temp dir");
            let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir2.path());
            ctx.view_cache.insert(
                super::super::driver_confirm::tests::ALIAS.to_string(),
                sweep_cache(dir.path()),
            );
            let mut terminal = Terminal::new(TestBackend::new(60, 1)).expect("terminal");
            let mut drew = None;
            terminal
                .draw(|frame| {
                    drew = Some(render_launcher_tab(
                        frame,
                        Rect::new(0, 0, 60, 0),
                        &ctx,
                        super::super::driver_confirm::tests::ALIAS,
                        ctx.view_cache
                            .get(super::super::driver_confirm::tests::ALIAS),
                        &Cell::default(),
                        &Cell::new(0),
                    ));
                })
                .expect("draw");
            drew.expect("rendered")
        };
        assert!(regions.list.is_none() && regions.output.is_none());
    }

    #[test]
    fn the_medium_header_row_two_is_the_status_tokens() {
        let dir = tempfile::tempdir().expect("temp dir");
        let rows = sweep_render(sweep_cache(dir.path()), 200, 20);
        // Row 0 is the pane title, row 1 the run header, row 2 the tokens.
        assert!(rows[1].contains(SWEEP_RUN), "{rows:#?}");
        assert!(rows[2].starts_with("  ctl pause (pending)"), "{rows:#?}");
        assert!(
            rows[2].contains("MISMATCH") && rows[2].contains("UNSANDBOXED"),
            "{rows:#?}"
        );
    }

    /// W = 60 is the 39-cell detail floor: every cap row keeps its whole 10-cell bar
    /// and the `MISMATCH` and `UNSANDBOXED` words are not clipped.
    #[test]
    fn cap_bars_and_flag_tokens_are_whole_at_the_detail_floor() {
        let dir = tempfile::tempdir().expect("temp dir");
        for height in [21u16, 30, 40] {
            let rows = sweep_render(sweep_cache(dir.path()), 60, height);
            for label in ["dispatches", "minutes", "gap rounds"] {
                let row = rows
                    .iter()
                    .find(|r| r.trim_start().starts_with(label))
                    .unwrap_or_else(|| panic!("{label}: {rows:#?}"));
                let cells = row
                    .chars()
                    .filter(|c| matches!(c, '\u{2588}' | '\u{2591}'))
                    .count();
                assert_eq!(cells, CAP_BAR_CELLS, "{label} at H={height}: {row:?}");
            }
            assert!(
                rows.iter().any(|r| r.contains("MISMATCH")),
                "H={height}: {rows:#?}"
            );
            assert!(
                rows.iter().any(|r| r.contains("UNSANDBOXED")),
                "H={height}: {rows:#?}"
            );
        }
        // And at MEDIUM the token row keeps its flags whole at the same width.
        let rows = sweep_render(sweep_cache(dir.path()), 60, 20);
        let tokens = &rows[2];
        for flag in ["MISMATCH", "UNSANDBOXED"] {
            assert!(
                tokens.contains(flag) || !tokens.contains(&flag[..3]),
                "{flag}: {tokens:?}"
            );
        }
        assert!(tokens.contains("MISMATCH"), "{tokens:?}");
    }

    #[test]
    fn a_short_items_budget_ends_in_a_more_row_and_keeps_the_cursor_visible() {
        let items: Vec<BodyItem> = (1..=10)
            .map(|n| body_item(&format!("I-{n:02}"), "title", None, None))
            .collect();
        let detail = detail_of("20261020-a", items, vec![], None);
        // rule + 3 rows (2 items and `more`), no cursor-item row.
        let lines = text_lines(&items_section_fit(
            Some(&detail),
            Some("I-07"),
            None,
            60,
            4,
            false,
        ));
        assert_eq!(lines.len(), 4, "{lines:#?}");
        assert!(
            lines[1].contains("I-06") && lines[2].contains("I-07"),
            "{lines:#?}"
        );
        assert!(lines[3].contains("8 more items"), "{lines:#?}");
        // One line under the rule: the cursor's own row, nothing else.
        let lines = text_lines(&items_section_fit(
            Some(&detail),
            Some("I-07"),
            None,
            60,
            2,
            false,
        ));
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("I-07"), "{lines:#?}");
        // With the cursor-item row, one item row remains at the FULL floor of 3.
        let lines = text_lines(&items_section_fit(
            Some(&detail),
            Some("I-01"),
            None,
            60,
            3,
            true,
        ));
        assert_eq!(lines.len(), 3, "{lines:#?}");
        assert!(
            lines[1].contains("I-01") && lines[2].contains("I-01"),
            "{lines:#?}"
        );
    }

    #[test]
    fn a_short_connected_budget_ends_in_a_more_row() {
        let (_, entry) = entry("20261020-a", running());
        let mut fm = entry.expect("readable").frontmatter;
        while fm.connected.len() < 5 {
            let extra = fm.connected[0].clone();
            fm.connected.push(extra);
        }
        let now = SystemTime::now();
        let lines = text_lines(&connected_section_fit(&fm, now, 80, 3));
        assert_eq!(lines.len(), 3, "{lines:#?}");
        assert!(lines[2].contains("4 more"), "{lines:#?}");
        let lines = text_lines(&connected_section_fit(&fm, now, 80, 2));
        assert_eq!(lines.len(), 2, "{lines:#?}");
        let full = text_lines(&connected_section_fit(&fm, now, 80, usize::MAX));
        assert_eq!(full.len(), 1 + CONNECTED_MAX_ROWS + 1);
    }

    #[test]
    fn the_summary_tier_keeps_the_selection_in_view_and_records_the_mouse_region() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut cache = sweep_cache(dir.path());
        for n in 0..12 {
            cache
                .launcher_runs
                .push(entry(&format!("20261001-run-{n:02}"), running()));
        }
        cache.launcher_selected_id = Some("20261001-run-11".to_owned());
        cache.launcher_selected = cache.launcher_runs.len() - 1;
        let (rows, _) = render(cache, 80, 5);
        assert_eq!(rows.len(), 5);
        assert!(
            rows.iter()
                .any(|r| r.starts_with("> ") && r.contains("20261001-run-11")),
            "the selected row is windowed into view: {rows:#?}"
        );
    }
}
