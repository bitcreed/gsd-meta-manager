//! The Driver tab: the run list, the run header, the reused D-R-P-E-V row and
//! the step timeline.
//!
//! **Module-layout decision, recorded here because this is where a reader will
//! ask the question** (the register of `src/driver/liveness.rs:35-41`). Every
//! other detail sub-tab renders inline in `detail.rs`; this one renders from its
//! own module and `detail.rs` delegates. The reason is size and shape rather
//! than taste: every other sub-tab is about forty lines of layout, while this
//! one is eight width and height tiers, a bounded ring-buffer renderer and a
//! four-state injection widget — and `detail.rs` is already the largest file in
//! the repository at over five thousand lines.
//!
//! **The declined alternative was to render inline like every sibling.** It was
//! declined only for that reason; the cost of the split is that a reader looking
//! for "where does the Driver tab render" finds a delegation rather than a body,
//! which this doc and the two `DetailSubView::Driver` dispatch arms answer.
//!
//! **The Replit rule governs everything here (D-13).** Every status word,
//! glyph, colour and sort key on this surface derives from `RunRecord.outcome`,
//! `JournalEvent::RunEnded.outcome` or `ExecFinished.exit` — evidence Phase 15
//! computes from `is_error`, `terminal_reason`, the permission-denial list, the
//! exit code and a git/disk snapshot. **None of it is ever computed by
//! inspecting the agent's own prose.** The agent's words may appear as content
//! in the output pane and may carry no authority anywhere else. The incident
//! behind the rule: an agent deleted a production database during an explicit
//! freeze, hid it, fabricated roughly four thousand fake users and fake test
//! results, and falsely claimed rollback was impossible. A tool that repeats an
//! agent's account of itself as fact makes that class of failure invisible to
//! the human who is accountable for the repository.
//!
//! **The injection vocabulary is a safety property, not a style choice**
//! (D-07, D-10). An injected message has exactly four states and each has one
//! authoritative observer: `queued` is durably in `inbox.jsonl` with nothing
//! having read it; `delivered` means the write to the agent's stdin returned
//! without error; `acted-on` means the agent **dequeued** it and is running it
//! as its own turn; `missed` means it was appended after the agent's input was
//! closed and can never be delivered.
//!
//! The word for the stdin write is *delivered* and nothing else. The word for
//! the dequeue echo is *acted-on*. **The word "sent" is forbidden here, and so
//! are "received", "read" and "acknowledged"** — the echo arrives at DEQUEUE,
//! measured roughly fifty-five seconds after the write, so every one of those
//! words would promise an observation the protocol cannot make. The filling
//! shape `○ → ◐ → ●` is the whole design: it reads correctly across a
//! minute-long gap. **There is no spinner, no animation and no implied
//! imminence anywhere on this surface**, and a spinner idiom here would be
//! actively dishonest rather than merely decorative.

use std::cell::Cell;

use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::detail::{clamp_scroll, tail_offset, ViewportMetrics};
use super::{
    sanitize_render_line, AppContext, DriverLineKind, DriverOutput, DriverOutputLine,
    ProjectViewCache, DRIVER_OUTPUT_RING_LINES,
};
use crate::driver::reconcile::RunVerdict;
use crate::journal::RunSummary;

// ── Measured layout constants (the `STATUS_COLUMN_MIN_CELLS` discipline) ────

/// The widest line `detail::build_pipeline_line` can produce, in cells.
///
/// **Derived from the render, not chosen.** Its worst case is the two-cell left
/// indent, then `[D]`, `---`, `[R]`, `---`, `[P]`, `---`, `[E 123/456]`, `---`,
/// `[V]` — `2 + 3 + 3 + 3 + 3 + 3 + 3 + 11 + 3 + 3 = 37`.
///
/// The number exists because of a bug with a name. A percentage-only constraint
/// on the pane holding that line is exactly what clipped its trailing `[V]`
/// (UIFIX-02 / CR-01), and a fully verified project then rendered identically to
/// a mid-pipeline one — the display disagreeing with the disk in the direction
/// that flatters the run.
pub const PIPELINE_LINE_MAX_CELLS: u16 = 37;

/// The floor the run-detail pane is given, in cells.
///
/// [`PIPELINE_LINE_MAX_CELLS`] plus the pane's own two-cell left indent. It is a
/// `Constraint::Min` and never a bare `Constraint::Percentage`, for the reason
/// spelled out above: the reused D-R-P-E-V line must keep its trailing `[V]` at
/// every width where two panes render at all, and only a floor guarantees that.
pub const DRIVER_DETAIL_MIN_CELLS: u16 = PIPELINE_LINE_MAX_CELLS + 2;

/// Below this outer width the run list is not rendered at all and the run detail
/// gets the whole area.
///
/// Derived: at 59 columns a 40% list takes 23 cells and leaves 36 for the
/// detail, which is below [`DRIVER_DETAIL_MIN_CELLS`]. Rendering both anyway
/// would clip the pipeline line, so the list — whose information moves into the
/// detail pane's block title — is what gives way.
pub const DRIVER_TWO_PANE_MIN_CELLS: u16 = 60;

/// Inner width at or above which a run-list row carries its state word.
///
/// Derived: the fixed row is 18 cells, the `"> "` highlight symbol is 2, and the
/// gap before the word is 2 — so 22 cells are spoken for before a word can
/// start, and 26 is the first width that leaves room for a short one.
pub const RUN_LIST_WORD_MIN_CELLS: u16 = 26;

/// The fixed part of a run-list row, in cells:
/// `glyph(1) space MM-DD(5) space HH:MM(5) space suffix(4)`.
pub const RUN_LIST_FIXED_CELLS: usize = 18;

// ── Run-state glyphs (fixed `&'static str`, `\u{…}` per PATTERNS S7) ────────
//
// A fixed `&'static str` is the *mechanism* enforcing "no glyph is derived from
// file content": no byte read off disk can be assigned to one of these, so agent
// prose cannot reach a run row through its glyph (D-13). The escape form keeps
// the source readable in editors that render these codepoints ambiguously.

/// `◆` — a run this scan positively knows is running.
const GLYPH_LIVE: &str = "\u{25C6}";
/// `◇` — the platform cannot say whether the driver lives. **Never a synonym
/// for dead**; that collapse was CR-05 and the tri-state fix must not be
/// re-flattened at the render layer.
const GLYPH_LIVENESS_UNKNOWN: &str = "\u{25C7}";
/// `●` — succeeded, with changes on disk.
const GLYPH_SUCCEEDED: &str = "\u{25CF}";
/// `○` — succeeded while nothing moved on disk. Its own state, not a flavour of
/// success: the envelope said success and the working tree disagreed.
const GLYPH_SUCCEEDED_NO_CHANGES: &str = "\u{25CB}";
/// `✗` — failed, denied, timed out, stalled, refused, crashed.
const GLYPH_FAILED: &str = "\u{2717}";
/// `■` — the user stopped it.
const GLYPH_KILLED: &str = "\u{25A0}";

// ── Output-pane markers (fixed `&'static str`, `\u{…}` per PATTERNS S7) ─────
//
// A two-cell left marker column, and the same mechanism argument as the run
// glyphs above: no byte read off disk can be assigned to one of these, so agent
// prose cannot reach the marker column (D-13).

/// The common case — assistant, user, turn-boundary and unparseable streams —
/// gets **no** marker, so injections and diagnostics stand out against it.
const MARKER_OUTPUT: &str = "  ";
/// `· ` — the agent's stderr: present, and deliberately secondary.
const MARKER_STDERR: &str = "\u{00B7} ";
/// `» ` — a human-injected message. The one Cyan use in this pane.
const MARKER_INJECTION: &str = "\u{00BB} ";
/// `! ` — a diagnostic. **Never silently swallowed**: a pane that loses lines
/// without saying so is exactly the "looks done but isn't" failure this phase
/// enumerates by name.
const MARKER_DIAGNOSTIC: &str = "! ";
/// `= ` — the terminal record, in the terminal-state colour and BOLD. The
/// visual full stop, and it renders last.
const MARKER_TERMINAL: &str = "= ";

// ── Injection state: glyphs, labels and the kinds that carry them ──────────

/// The three journal kinds that carry an injected message's state (D-07, D-09).
///
/// Named here, beside the derivation that reads them, and referenced by the
/// scan that collects them — so the set of kinds that matter exists once.
pub const INJECTION_KINDS: [&str; 3] = [
    "interjected",
    "interjection_acted_on",
    "interjection_missed",
];

/// `○` — durably on disk in `inbox.jsonl`; **nothing has read it**.
const GLYPH_QUEUED: &str = "\u{25CB}";
/// `◐` — written to the agent's stdin without error. Half-filled, because half
/// of what matters has happened: the write landed and the agent has not yet
/// picked it up.
const GLYPH_DELIVERED: &str = "\u{25D0}";
/// `●` — the agent **dequeued** it and is running it as its own turn. Full,
/// because this is as far as the protocol can see. The same codepoint the run
/// glyphs use for "succeeded": both mean *this reached its end*, and the two
/// live in different columns of different widgets, so no row shows both.
const GLYPH_ACTED_ON: &str = "\u{25CF}";
/// `✗` — appended after the agent's input was closed. Undeliverable, named,
/// and **never retried**.
const GLYPH_MISSED: &str = "\u{2717}";

/// The exact label for a message durably queued and unread.
const LABEL_QUEUED: &str = "queued";
/// The exact label for a write to the agent's stdin that returned without
/// error. **This word belongs to the stdin write and to nothing else.**
const LABEL_DELIVERED: &str = "delivered";
/// The exact label for the dequeue echo. Deliberately not "received", "read" or
/// "acknowledged": the echo says the agent *started processing*, roughly a
/// minute after the write, and every one of those three words would claim an
/// earlier and stronger observation than the protocol supports.
const LABEL_ACTED_ON: &str = "acted-on";
/// The exact label for the honest fourth state (D-10).
const LABEL_MISSED: &str = "missed";

/// Why a missed message is missed, in one line.
///
/// **Leaving it in `queued` forever would be the undelivered-injection failure
/// dressed up as a spinner.** It is named instead.
const MISSED_GLOSS: &str = "(the run closed its input before this was delivered)";

// ── Copy (Copywriting Contract, exact strings) ─────────────────────────────

/// The goal field for a run that was started without one.
///
/// **Never a fabricated or paraphrased summary** (D-13, D-23). A goal that was
/// given is stored and shown verbatim; a goal that was not given says so.
const GOAL_NONE_GIVEN: &str = "(none given)";

const NO_RUNS_OPTED_IN: &str = "No runs yet for";
const NO_RUNS_OPTED_IN_NEXT: &str = "Press [s] to start one.";
const NOT_OPTED_IN_NEXT: &str = "Press [o] on the dashboard to allow it.";
const NO_JOURNAL_ENTRIES: &str = "No journal entries yet.";

/// The right-aligned indicator while the pane is following a live tail.
const INDICATOR_FOLLOWING: &str = "[following]";

/// The right-aligned indicator for a run this session did not spawn.
///
/// **It does not say "live"**, because there is nothing live about it: once the
/// TUI exits, the child's stdout pipe is gone, so reattachment is read-only and
/// journal-based (Phase 17 D-11).
const INDICATOR_JOURNAL_ONLY: &str = "[journal only]";

/// The largest scrolled-below count rendered as a figure; past it the indicator
/// says `+999`, because the exact number stops carrying information long before
/// then and a widening field would move the rule under the reader's eye.
const SCROLLED_DISPLAY_MAX: usize = 999;

/// The notice the pane's first row carries once the ring has dropped lines.
///
/// **Not cosmetic.** The buffer is bounded, so on a long run it *will* drop the
/// beginning of the output; a pane that silently loses lines is the named
/// "looks done but isn't" failure. The count is monotonic over the session, so
/// the sentence stays true after any number of wraps.
fn ring_overflow_notice(dropped: u64) -> String {
    format!("\u{2026} {dropped} earlier lines dropped (buffer holds {DRIVER_OUTPUT_RING_LINES})")
}

/// The notice for a record cut at the per-record line cap.
///
/// One pathological record could otherwise flush the whole ring, so the cap
/// exists — and saying that it fired is the other half of it.
const RECORD_TRUNCATED_NOTICE: &str = "    \u{2026} record truncated";

/// The mandatory honest note under the one-row step timeline.
///
/// **Required, not decorative.** In this phase a run executes exactly one GSD
/// command plus whatever turns the human's own interjections add, so the
/// timeline has one decided row — and a one-row timeline with no explanation
/// implies a computed sequence of steps that does not exist yet. Phase 20's
/// router is what makes the row count mean something.
const STEPS_HONEST_NOTE: &str = "This version runs one command per run.";

/// The label that makes the cost figure readable rather than misleading.
///
/// **Mandatory** (D-12): `total_cost_usd` accumulates across the turns of one
/// run while `num_turns` and `duration_ms` reset per turn, so an unlabelled
/// figure invites the reader to take a running total for a per-turn one. Any
/// per-turn figure rendered anywhere on this surface must be labelled as such.
const COST_CUMULATIVE: &str = "cumulative";

/// What the cost line says before any `cost` record has been read for this run.
///
/// A run's cumulative cost is only knowable from its journal, and the run list
/// is deliberately built from each run's small committed `run.json` without
/// opening the journal beside it. So for a run that is not the one being tailed
/// the figure is genuinely unknown — and saying so is the only honest option. A
/// zero here would be a number the mechanism cannot back.
const COST_NOT_REPORTED: &str = "not yet reported";

// ── Styles ─────────────────────────────────────────────────────────────────

fn label_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn muted_style() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM)
}

/// The glyph, run-list word and colour for one run's state.
///
/// **Every arm reads evidence and nothing else** (D-13): `verdict` is
/// [`crate::driver::reconcile::ObservedRun::verdict`], derived from the record's
/// `ended_at` and a pid/cmdline liveness probe; `outcome` is
/// `RunRecord.outcome`, the rendered form of Phase 15's four-source derivation
/// over `is_error`, `terminal_reason`, the permission-denial list, the exit code
/// and a git/disk snapshot. No agent prose is inspected, and no string
/// comparison against agent prose decides whether a run succeeded.
///
/// `None` for both is **not** treated as a crash. A run with no terminal record
/// and no observation is a run nothing can currently speak for, which is what
/// `◇ ?` says; reporting it as dead would manufacture a crash out of an absence
/// of evidence, which is the CR-05 defect in a new place.
pub fn run_state_glyph(
    verdict: Option<RunVerdict>,
    outcome: Option<&str>,
) -> (&'static str, &'static str, Color) {
    match verdict {
        Some(RunVerdict::Live) => (GLYPH_LIVE, "live", Color::Magenta),
        Some(RunVerdict::LivenessUnknown) => (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow),
        Some(RunVerdict::CrashedWithoutEnding) => (GLYPH_FAILED, "crash", Color::Red),
        Some(RunVerdict::Ended) | None => match outcome {
            Some("succeeded_with_changes") => (GLYPH_SUCCEEDED, "ok", Color::Green),
            Some("succeeded_no_changes") => {
                (GLYPH_SUCCEEDED_NO_CHANGES, "no-chg", Color::Yellow)
            }
            Some("failed") => (GLYPH_FAILED, "fail", Color::Red),
            Some("permission_denied") => (GLYPH_FAILED, "denied", Color::Red),
            Some("timed_out") => (GLYPH_FAILED, "t/out", Color::Red),
            Some("stalled") => (GLYPH_FAILED, "stall", Color::Red),
            Some("capability_refused") => (GLYPH_FAILED, "refuse", Color::Red),
            Some("spawn_failed") => (GLYPH_FAILED, "spawn", Color::Red),
            Some("killed") => (GLYPH_KILLED, "killed", Color::DarkGray),
            // A label this build has never seen, or none at all.
            _ => (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow),
        },
    }
}

/// How long a run has been going, or how long it took.
///
/// `+MM:SS` while it is still going and `ran MM:SS` once it has ended, both
/// widening to `H:MM:SS` past an hour. An unparseable `started_at` yields
/// `unknown` rather than a plausible-looking zero: a duration the code cannot
/// compute is not a duration of nothing.
pub fn elapsed_label(started_at: &str, now: DateTime<Utc>, ended_at: Option<&str>) -> String {
    let Some(start) = parse_rfc3339(started_at) else {
        return "unknown".to_string();
    };
    let ended = ended_at.and_then(parse_rfc3339);
    let end = ended.unwrap_or(now);
    let seconds = (end - start).num_seconds().max(0);
    let rendered = hms(seconds);
    match ended {
        Some(_) => format!("ran {rendered}"),
        None => format!("+{rendered}"),
    }
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn hms(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// `MM-DD` and `HH:MM` in the reader's local zone, or placeholders.
fn local_date_time(started_at: &str) -> (String, String) {
    match parse_rfc3339(started_at) {
        Some(dt) => {
            let local = dt.with_timezone(&Local);
            (
                local.format("%m-%d").to_string(),
                local.format("%H:%M").to_string(),
            )
        }
        None => ("??-??".to_string(), "??:??".to_string()),
    }
}

/// The last four characters of a run id — enough to tell two runs apart in a
/// list, since `new_run_id` puts a random tie-break suffix there.
fn run_id_suffix(run_id: &str) -> String {
    let chars: Vec<char> = run_id.chars().collect();
    let start = chars.len().saturating_sub(4);
    let mut suffix: String = chars[start..].iter().collect();
    while suffix.chars().count() < 4 {
        suffix.insert(0, ' ');
    }
    suffix
}

/// One row of the run list.
///
/// The fixed part is [`RUN_LIST_FIXED_CELLS`] cells at every width — one form,
/// not a set of width variants — and the state word is appended only when the
/// pane's inner width reaches [`RUN_LIST_WORD_MIN_CELLS`]. Glyph, word and
/// colour travel together so no meaning is carried by colour alone.
pub fn run_list_row(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    inner_width: u16,
) -> Line<'static> {
    let (glyph, word, color) = run_state_glyph(verdict, summary.outcome.as_deref());
    let (date, time) = local_date_time(&summary.started_at);
    let mut spans = vec![
        Span::styled(glyph, Style::default().fg(color)),
        Span::raw(format!(
            " {date} {time} {}",
            run_id_suffix(&summary.run_id)
        )),
    ];
    if inner_width >= RUN_LIST_WORD_MIN_CELLS {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(word, Style::default().fg(color)));
    }
    Line::from(spans)
}

/// The identity of the selected run, for the narrow tier's pane title.
fn run_list_title(summary: &RunSummary, index: usize, total: usize) -> String {
    let (date, time) = local_date_time(&summary.started_at);
    format!(
        " {date} {time} {}  ({}/{}) ",
        run_id_suffix(&summary.run_id),
        index + 1,
        total
    )
}

/// Split the tab area into the run list and the run detail.
///
/// The detail pane takes `Constraint::Min(DRIVER_DETAIL_MIN_CELLS)` and **never
/// a bare percentage complement of the list's 40%** — the discipline that keeps
/// the reused D-R-P-E-V line from losing its trailing `[V]`. A percentage-only
/// constraint on that pane is the CR-01 / UIFIX-02 defect, verbatim. Below
/// [`DRIVER_TWO_PANE_MIN_CELLS`] the list is dropped entirely and `None` is
/// returned for it; the selection still moves, only its *rendering* is given up.
fn driver_panes(area: Rect) -> (Option<Rect>, Rect) {
    if area.width < DRIVER_TWO_PANE_MIN_CELLS {
        return (None, area);
    }
    let panes = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Min(DRIVER_DETAIL_MIN_CELLS),
    ])
    .split(area);
    (Some(panes[0]), panes[1])
}

/// Wrap `text` to at most `max_rows` rows of `width` cells, ellipsing the last
/// row when there is more.
///
/// Greedy word wrapping with a hard split for a word longer than the row, and
/// `char`-based throughout: byte slicing panics on a multibyte boundary, and a
/// goal is arbitrary user text.
fn wrap_rows(text: &str, width: usize, max_rows: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_cells = 0usize;

    let mut push_row = |row: &mut String, cells: &mut usize| {
        rows.push(std::mem::take(row));
        *cells = 0;
    };

    for word in text.split_whitespace() {
        let word_cells = word.chars().count();
        if current_cells > 0 && current_cells + 1 + word_cells > width {
            push_row(&mut current, &mut current_cells);
        }
        if word_cells > width {
            // A single word wider than the row: hard-split it by chars.
            for ch in word.chars() {
                if current_cells == width {
                    push_row(&mut current, &mut current_cells);
                }
                current.push(ch);
                current_cells += 1;
            }
            continue;
        }
        if current_cells > 0 {
            current.push(' ');
            current_cells += 1;
        }
        current.push_str(word);
        current_cells += word_cells;
    }
    if current_cells > 0 {
        rows.push(current);
    }

    if rows.len() > max_rows {
        rows.truncate(max_rows);
        if let Some(last) = rows.last_mut() {
            if last.chars().count() >= width {
                let kept: String = last.chars().take(width.saturating_sub(1)).collect();
                *last = kept;
            }
            last.push('\u{2026}');
        }
    }
    rows
}

/// Truncate to `width` cells by `char`, with a trailing ellipsis when cut.
fn ellipsize(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// Keep the tail of a path when it does not fit — the leading directories are
/// the part a reader can reconstruct, the run id is not.
fn left_truncate(text: &str, width: usize) -> String {
    let cells = text.chars().count();
    if cells <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    let skip = cells.saturating_sub(keep);
    let mut out = String::from("\u{2026}");
    out.extend(text.chars().skip(skip));
    out
}

/// The goal rows: verbatim, sanitised, wrapped to at most three, then ellipsed.
///
/// **The goal is never fabricated, paraphrased or summarised** (D-13, D-23). It
/// is the one field on this surface that is the human's own words, and the whole
/// value of OBS-03 is that it comes back exactly as it was given. When no goal
/// was given the field says [`GOAL_NONE_GIVEN`] and shows nothing else — an
/// invented one-line summary of what the run "seems to be doing" would be the
/// agent's account of itself wearing the user's voice.
fn goal_lines(goal: &str, inner_width: u16) -> Vec<Line<'static>> {
    const PREFIX: &str = "  goal: ";
    const CONTINUATION: &str = "        ";

    if goal.trim().is_empty() {
        return vec![Line::from(vec![
            Span::styled(PREFIX, label_style()),
            Span::styled(GOAL_NONE_GIVEN, muted_style()),
        ])];
    }

    let sanitised = sanitize_render_line(goal);
    let body_width = usize::from(inner_width).saturating_sub(PREFIX.chars().count());
    let rows = wrap_rows(&sanitised, body_width, 3);
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            Line::from(vec![
                Span::styled(if i == 0 { PREFIX } else { CONTINUATION }, label_style()),
                Span::raw(row),
            ])
        })
        .collect()
}

/// How many rows [`goal_lines`] will take, so the vertical layout can budget.
fn goal_rows(goal: &str, inner_width: u16) -> u16 {
    goal_lines(goal, inner_width).len() as u16
}

/// The run header: goal, command, times, cumulative cost, and the run directory.
///
/// **Nothing here is derived from the agent's prose** (D-13). The goal and the
/// command are the human's own input echoed back; the timestamps come from the
/// run record; the cost comes from a `cost` journal record's `cumulative_usd`.
/// The agent's `ResultMessage.result` may be displayed as content in the output
/// pane below and has no authority over any word in this header.
fn render_run_header(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    cost_usd: Option<f64>,
    run_dir: Option<&str>,
    inner_width: u16,
    now: DateTime<Utc>,
) -> Vec<Line<'static>> {
    let mut lines = goal_lines(&summary.goal, inner_width);

    let command = ellipsize(
        &sanitize_render_line(&summary.gsd_command),
        usize::from(inner_width).saturating_sub(30).max(8),
    );
    let (_, started) = local_date_time(&summary.started_at);
    let mut cmd_spans = vec![
        Span::styled("  cmd:  ", label_style()),
        Span::raw(command),
        Span::styled("    started ", label_style()),
        Span::raw(started),
        Span::styled("    ", label_style()),
    ];
    // While a run is live the label reads `elapsed`; once it has ended the value
    // itself carries the `ran` verb, so a second label would read "elapsed ran".
    if summary.ended_at.is_none() {
        cmd_spans.push(Span::styled("elapsed ", label_style()));
    }
    cmd_spans.push(Span::raw(elapsed_label(
        &summary.started_at,
        now,
        summary.ended_at.as_deref(),
    )));
    if let Some(ended_at) = summary.ended_at.as_deref() {
        let (_, ended) = local_date_time(ended_at);
        cmd_spans.push(Span::styled("    ended ", label_style()));
        cmd_spans.push(Span::raw(ended));
    }
    lines.push(Line::from(cmd_spans));

    lines.push(Line::from(match cost_usd {
        Some(usd) => vec![
            Span::styled("  cost  ", label_style()),
            Span::raw(format!("${usd:.2} ")),
            Span::styled(COST_CUMULATIVE, label_style()),
        ],
        None => vec![
            Span::styled("  cost  ", label_style()),
            Span::styled(COST_NOT_REPORTED, muted_style()),
        ],
    }));

    if let Some(dir) = run_dir {
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                left_truncate(
                    &sanitize_render_line(dir),
                    usize::from(inner_width).saturating_sub(2).max(8),
                )
            ),
            label_style(),
        )));
    }

    // `verdict` is threaded through so a future reader sees that the header has
    // the evidence in hand and deliberately renders no status word of its own —
    // the state word belongs to the run list and the step timeline, and saying
    // it three times would invite three renderings of one fact.
    let _ = verdict;
    lines
}

/// The copy for a project with no runs, in its two opt-in variants.
///
/// Both name the next key, because an empty state that does not is a dead end.
/// The two are genuinely different situations: one project is ready and has not
/// been asked to do anything, the other has not been given permission.
fn no_runs_lines(alias: &str, opted_in: bool) -> Vec<Line<'static>> {
    let alias = sanitize_render_line(alias);
    if opted_in {
        vec![
            Line::from(format!("  {NO_RUNS_OPTED_IN} \"{alias}\".")),
            Line::from(Span::styled(
                format!("  {NO_RUNS_OPTED_IN_NEXT}"),
                label_style(),
            )),
        ]
    } else {
        vec![
            Line::from(format!("  \"{alias}\" is not opted in to driving.")),
            Line::from(Span::styled(
                format!("  {NOT_OPTED_IN_NEXT}"),
                label_style(),
            )),
        ]
    }
}

/// Render the Driver tab (TRANS-05, OBS-03, OBS-05).
///
/// The layout is two panes — the run list and the run detail — on the 40/60
/// split the Pipeline tab already uses, with the detail pane given a
/// `Constraint::Min` floor rather than a percentage. Below
/// [`DRIVER_TWO_PANE_MIN_CELLS`] the list is dropped and its identity moves into
/// the detail pane's block title.
pub(super) fn render_driver_tab(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    viewport: &Cell<ViewportMetrics>,
) {
    let runs: &[RunSummary] = cache.map_or(&[], |c| c.driver_runs.as_slice());

    // The "no state / no data" early return, in the shape `render_pipeline_tab`
    // established: one bordered block, one message, nothing else painted.
    if runs.is_empty() {
        let opted_in = ctx
            .config
            .projects
            .get(alias)
            .is_some_and(|project| project.driver_opt_in.is_some());
        let block = Block::default().borders(Borders::ALL).title(" Driver ");
        let message = Paragraph::new(no_runs_lines(alias, opted_in)).block(block);
        frame.render_widget(message, area);
        return;
    }

    let selected = cache
        .map_or(0, |c| c.driver_selected_run)
        .min(runs.len() - 1);
    let summary = &runs[selected];

    // The verdict applies only to the run the reconciliation scan is observing;
    // every other row in the list is a finished run whose own `outcome` speaks
    // for it. Applying an observed run's verdict to the wrong row would report
    // one run's liveness as another's.
    let observed = ctx.observed_runs.get(alias);
    let verdict_for = |run: &RunSummary| -> Option<RunVerdict> {
        observed
            .filter(|run_observed| run_observed.run_id == run.run_id)
            .map(|run_observed| run_observed.verdict())
    };

    let (list_area, detail_area) = driver_panes(area);

    if let Some(list_area) = list_area {
        let list_block = Block::default().borders(Borders::RIGHT).title(" Runs ");
        let inner_width = list_block.inner(list_area).width;
        let items: Vec<ListItem> = runs
            .iter()
            .map(|run| ListItem::new(run_list_row(run, verdict_for(run), inner_width)))
            .collect();
        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .highlight_symbol("> ");
        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, list_area, &mut list_state);
    }

    // At the narrow tier the selection has nowhere else to go, so it becomes the
    // pane's title. `j`/`k` still move it; only the list's rendering is dropped.
    let detail_block = if list_area.is_some() {
        Block::default().borders(Borders::NONE).title(" Driver ")
    } else {
        Block::default()
            .borders(Borders::NONE)
            .title(run_list_title(summary, selected, runs.len()))
    };
    let inner = detail_block.inner(detail_area);
    frame.render_widget(detail_block, detail_area);

    render_run_detail(
        frame,
        inner,
        ctx,
        alias,
        cache,
        summary,
        verdict_for(summary),
        viewport,
    );
}

/// The vertical layout of the run-detail pane, in three height tiers.
///
/// **No section is ever allocated zero rows**: a section that cannot fit is
/// dropped by the tier rather than squeezed to nothing, because a one-row
/// section rule with no content under it reads as a section that failed to load.
#[allow(clippy::too_many_arguments)]
fn render_run_detail(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    viewport: &Cell<ViewportMetrics>,
) {
    let now = Utc::now();
    let tally = cache.and_then(|c| c.driver_tally.as_ref());
    let cost_usd = tally
        .filter(|tally| tally.run_id == summary.run_id)
        .and_then(|tally| tally.cumulative_cost_usd);
    let turns = tally
        .filter(|tally| tally.run_id == summary.run_id)
        .map(|tally| tally.turn_boundaries + 1);

    let run_dir = ctx.config.projects.get(alias).and_then(|project| {
        crate::journal::run_paths(&project.path.join(".planning"), &summary.run_id)
            .map(|paths| paths.dir.display().to_string())
    });

    let inference = ctx
        .project_states
        .get(alias)
        .and_then(|state| state.current_phase_status.as_ref());

    let goal_row_count = goal_rows(&summary.goal, area.width);
    let header_rows = goal_row_count + 3;
    // The section rule, the one decided row this phase has (D-12), and the
    // mandatory honest note.
    let step_rows = 3;

    let chunks: Vec<Rect> = if area.height >= 14 {
        Layout::vertical([
            Constraint::Length(header_rows),
            Constraint::Length(2),
            Constraint::Length(step_rows),
            Constraint::Min(3),
        ])
        .split(area)
        .to_vec()
    } else if area.height >= 8 {
        // Every section is still present, at one row each. **No section is ever
        // allocated zero rows** — a section that cannot fit is dropped by the
        // tier, never squeezed to nothing, because an empty section rule reads
        // as a section that failed to load.
        Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .split(area)
        .to_vec()
    } else {
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)])
            .split(area)
            .to_vec()
    };

    if area.height < 8 {
        // The shortest tier: a one-row identity header and the output pane.
        let (date, time) = local_date_time(&summary.started_at);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {date} {time} {}", run_id_suffix(&summary.run_id)),
                label_style(),
            ))),
            chunks[0],
        );
        render_output_section(
            frame, chunks[1], ctx, alias, cache, summary, verdict, viewport,
        );
        return;
    }

    let header = render_run_header(
        summary,
        verdict,
        cost_usd,
        run_dir.as_deref(),
        area.width,
        now,
    );
    let header_budget = usize::from(chunks[0].height);
    frame.render_widget(
        Paragraph::new(header.into_iter().take(header_budget).collect::<Vec<_>>()),
        chunks[0],
    );

    render_pipeline_row(frame, chunks[1], inference);
    render_steps(frame, chunks[2], summary, verdict, turns);
    render_output_section(
        frame, chunks[3], ctx, alias, cache, summary, verdict, viewport,
    );
}

/// The D-R-P-E-V pipeline row — **called, never re-implemented** (D-17).
///
/// [`super::detail::derive_all_stage_statuses`] and
/// [`super::detail::build_pipeline_line`] are free functions over
/// `&DiskInference`, invoked here exactly as `render_pipeline_tab` invokes them,
/// with their existing two-cell indent, their existing per-stage colours and
/// their existing `[--]` rendering for a skipped stage. Nothing about the widget
/// is modified, wrapped, restyled or duplicated: **a second progress display is
/// a named anti-feature** and D-17 makes reuse mandatory.
///
/// `ARCHITECTURE` M5 proposes lifting those functions out of `detail.rs` into
/// `state_reader/`. That is **explicitly declined for this phase**, and the
/// decline is recorded at the call site so a later reader knows it was
/// considered rather than missed: the driver process does not render, Phase 20's
/// `decide()` is the first genuine second consumer, and moving 130 lines now
/// would churn the largest file in the repository to buy nothing.
///
/// The inference is the project's **current phase**'s, which is what makes this
/// row answer the question a driven repository raises — how far through the
/// pipeline is the thing the agent is working on.
fn render_pipeline_row(
    frame: &mut Frame,
    area: Rect,
    inference: Option<&crate::state_reader::disk_status::DiskInference>,
) {
    let mut lines: Vec<Line<'static>> = vec![Line::from("")];
    match inference {
        Some(inf) => {
            let statuses = super::detail::derive_all_stage_statuses(inf);
            lines.push(super::detail::build_pipeline_line(inf, &statuses));
        }
        None => lines.push(Line::from(Span::styled("  No disk data", label_style()))),
    }
    frame.render_widget(Paragraph::new(lines), area);
}

/// The step timeline: **exactly one decided row in this phase**, plus the note
/// that says so.
///
/// **Turns are turns, not commands** (D-12, Phase 15 D-29). A steered run emits
/// several `system/init` and `result` pairs inside one process; the timeline
/// shows the one command that was *decided* and the counter shows the turns. A
/// later `system/init` is informational and is never rendered as a restart. The
/// section carries N rows structurally so Phase 20's router needs no re-layout,
/// while this phase supplies one — and says so, because a one-row timeline with
/// no explanation implies a computed sequence that does not exist.
///
/// **The state word comes from evidence only, never from the agent's prose**
/// (D-13). [`run_state_glyph`] reads `RunVerdict` — the record's `ended_at` plus
/// a pid/cmdline liveness probe — and `RunRecord.outcome`, the rendered form of
/// the four-source derivation Phase 15 computes from `is_error`,
/// `terminal_reason`, the permission-denial list, the exit code and a git/disk
/// snapshot. **No string comparison against what the agent said about itself
/// decides what this row says.** The agent's own account may appear as content
/// in the output pane below and carries no authority here.
fn render_steps(
    frame: &mut Frame,
    area: Rect,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    turns: Option<u32>,
) {
    frame.render_widget(
        Paragraph::new(steps_lines(summary, verdict, turns, area.width)),
        area,
    );
}

/// The step section's lines, as a pure function so the whole thing is assertable
/// without a terminal (S6).
fn steps_lines(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    turns: Option<u32>,
    width: u16,
) -> Vec<Line<'static>> {
    let live = matches!(verdict, Some(RunVerdict::Live));
    let (_, word, color) = run_state_glyph(verdict, summary.outcome.as_deref());
    let (_, started) = local_date_time(&summary.started_at);

    let command_style = if live {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let mut lines: Vec<Line<'static>> = vec![section_rule("steps", width)];

    // N rows structurally, one row supplied — Phase 20's router adds the rest
    // without re-laying-out this section.
    let decided: [&RunSummary; 1] = [summary];
    for step in decided {
        let mut spans = vec![
            Span::styled(
                format!("  {}", sanitize_render_line(&step.gsd_command)),
                command_style,
            ),
            Span::styled(format!("   {started}   "), label_style()),
            Span::styled(
                if live { "running" } else { word },
                Style::default().fg(color),
            ),
        ];
        if live {
            if let Some(turns) = turns {
                spans.push(Span::styled(format!("   turn {turns}"), label_style()));
            }
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(Span::styled(
        format!("  {STEPS_HONEST_NOTE}"),
        muted_style(),
    )));

    lines
}

/// The right-aligned state of the output pane, as a word and a colour.
///
/// Four states, and each says something the others do not:
///
/// * **following** — the pane is live and pinned to the tail.
/// * **scrolled +N** — the pane is live and the user has moved off the tail; `N`
///   is how many buffered lines sit below the viewport. It deliberately does
///   **not** say *paused*: `paused` already means *"a non-empty HANDOFF is
///   present"* everywhere else in this tool, and spending the word on a scroll
///   state would make the vocabulary lie.
/// * **journal only** — this run started before the current TUI session. Its
///   live output is gone and what is shown came off disk.
/// * **ended HH:MM** — the run is over and the journal has stopped growing.
///
/// Adopted only qualifies an ended run: while an adopted run is still going its
/// journal *is* being tailed, so the follow bit is meaningful and the pane's
/// first row carries the provenance instead.
pub fn follow_indicator(
    live: bool,
    following: bool,
    below: usize,
    ended_at: Option<&str>,
    adopted: bool,
) -> (String, Color) {
    if live {
        return if following {
            (INDICATOR_FOLLOWING.to_string(), Color::Green)
        } else if below > SCROLLED_DISPLAY_MAX {
            (format!("[scrolled +{SCROLLED_DISPLAY_MAX}]"), Color::Yellow)
        } else {
            (format!("[scrolled +{below}]"), Color::Yellow)
        };
    }
    if adopted {
        return (INDICATOR_JOURNAL_ONLY.to_string(), Color::DarkGray);
    }
    match ended_at {
        Some(ended_at) => {
            let (_, time) = local_date_time(ended_at);
            (format!("[ended {time}]"), Color::DarkGray)
        }
        // A run with no terminal record and no observation. Saying `[ended]`
        // would assert a fact nothing on disk supports.
        None => (INDICATOR_JOURNAL_ONLY.to_string(), Color::DarkGray),
    }
}

/// The output pane's header: a DarkGray rule with the indicator on its right.
///
/// The rule is the pane's own row rather than the first line of the scrolling
/// paragraph, so the indicator does not scroll away the moment it becomes
/// interesting. At a width too narrow for both, the dashes give way first and
/// the indicator is what survives.
fn output_header_line(width: u16, indicator: &str, color: Color) -> Line<'static> {
    let head = "\u{2500}\u{2500} output ";
    let tail_cells = indicator.chars().count() + 4; // ` ` + indicator + ` ──`
    let pad = usize::from(width)
        .saturating_sub(head.chars().count())
        .saturating_sub(tail_cells);
    Line::from(vec![
        Span::styled(
            format!("{head}{}", "\u{2500}".repeat(pad)),
            label_style(),
        ),
        Span::raw(" "),
        Span::styled(indicator.to_string(), Style::default().fg(color)),
        Span::styled(" \u{2500}\u{2500}", label_style()),
    ])
}

/// One buffered line, with its two-cell marker in its own style.
///
/// The four visual classes are the UI-SPEC's event-kind table. `terminal_color`
/// is the run's terminal-state colour, which is derived from evidence and never
/// from the agent's prose (D-13) — see [`run_state_glyph`].
fn output_line(line: &DriverOutputLine, terminal_color: Color) -> Line<'static> {
    let injection_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let terminal_style = Style::default()
        .fg(terminal_color)
        .add_modifier(Modifier::BOLD);
    let (marker, marker_style, text_style) = match line.kind {
        DriverLineKind::Output => (MARKER_OUTPUT, Style::default(), Style::default()),
        DriverLineKind::Stderr => (MARKER_STDERR, label_style(), label_style()),
        DriverLineKind::Injection => (MARKER_INJECTION, injection_style, Style::default()),
        DriverLineKind::Diagnostic => (
            MARKER_DIAGNOSTIC,
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        ),
        DriverLineKind::Terminal => (MARKER_TERMINAL, terminal_style, terminal_style),
    };
    Line::from(vec![
        Span::styled(marker, marker_style),
        Span::styled(line.text.clone(), text_style),
    ])
}

/// The scrolling body of the output pane, as a pure function (S6).
///
/// Order, and every part of it is load-bearing:
///
/// 1. The **ring-overflow notice**, when the buffer has dropped lines. It is
///    first because it describes what is missing from everything below it.
/// 2. The buffered lines, in order, each with its marker — with the terminal
///    record held back.
/// 3. The **record-truncation notice**, when one record was cut at the
///    per-record cap.
/// 4. The terminal record, which renders **last**: the visual full stop.
///
/// A run whose journal has no records at all renders the pinned no-entries copy
/// and nothing else.
fn output_body_lines(output: Option<&DriverOutput>, terminal_color: Color) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let Some(output) = output.filter(|output| !output.is_empty()) else {
        lines.push(Line::from(Span::styled(
            format!("  {NO_JOURNAL_ENTRIES}"),
            label_style(),
        )));
        return lines;
    };

    if output.dropped() > 0 {
        lines.push(Line::from(Span::styled(
            ring_overflow_notice(output.dropped()),
            muted_style(),
        )));
    }

    let mut terminal: Vec<Line<'static>> = Vec::new();
    for line in output.lines() {
        match line.kind {
            DriverLineKind::Terminal => terminal.push(output_line(line, terminal_color)),
            _ => lines.push(output_line(line, terminal_color)),
        }
    }

    if output.record_truncated() {
        lines.push(Line::from(Span::styled(
            RECORD_TRUNCATED_NOTICE,
            muted_style(),
        )));
    }

    lines.extend(terminal);
    lines
}

/// The live output pane (OBS-04, OBS-05).
///
/// The viewer is the Browse file viewer's shape: build the whole `Vec<Line>`,
/// take `total_lines` from **that vector** after it is fully built, record the
/// metrics into the shared `Cell<ViewportMetrics>` during the render pass, and
/// render `Paragraph::new(lines).scroll((offset, 0))` with **wrapping off** —
/// `Wrap` breaks the `total_lines` ↔ offset correspondence the clamp depends on.
///
/// **The follow bit is resolved here and nowhere else.** While it is set the
/// pane shows the tail, expressed as `u16::MAX` through the one shared
/// [`clamp_scroll`]; a second `total_lines - visible_height` written out
/// anywhere is how UIFIX-04 would come back in a new place (D-19).
#[allow(clippy::too_many_arguments)]
fn render_output_section(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    viewport: &Cell<ViewportMetrics>,
) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    let (rule_area, body_area) = (rows[0], rows[1]);

    let (_, _, terminal_color) = run_state_glyph(verdict, summary.outcome.as_deref());
    let body = output_body_lines(ctx.driver_output.get(alias), terminal_color);

    // `total_lines` is taken **after** the vector is fully built, so the offset
    // always clamps against what is actually rendered.
    let total_lines = body.len() as u16;
    let metrics = ViewportMetrics {
        total_lines,
        visible_height: body_area.height,
    };
    viewport.set(metrics);

    let following = cache.is_some_and(|c| c.driver_follow);
    // `u16::MAX` means "the tail, whatever it is"; the shared clamp resolves it.
    let requested = if following {
        u16::MAX
    } else {
        cache.map_or(0, |c| c.driver_scroll_offset)
    };
    let scroll = clamp_scroll(requested, total_lines, body_area.height);
    let below = usize::from(tail_offset(metrics).saturating_sub(scroll));

    let live = matches!(verdict, Some(RunVerdict::Live));
    let adopted = !ctx.session_spawned_runs.contains(&summary.run_id);
    let (indicator, color) = follow_indicator(
        live,
        following,
        below,
        summary.ended_at.as_deref(),
        adopted,
    );
    frame.render_widget(
        Paragraph::new(output_header_line(rule_area.width, &indicator, color)),
        rule_area,
    );
    frame.render_widget(Paragraph::new(body).scroll((scroll, 0)), body_area);
}

/// A DarkGray section rule: `── name ─────…` padded to the pane width.
fn section_rule(name: &str, width: u16) -> Line<'static> {
    let head = format!("\u{2500}\u{2500} {name} ");
    let pad = usize::from(width).saturating_sub(head.chars().count());
    Line::from(Span::styled(
        format!("{head}{}", "\u{2500}".repeat(pad)),
        label_style(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(run_id: &str, outcome: Option<&str>) -> RunSummary {
        RunSummary {
            run_id: run_id.to_string(),
            started_at: "2026-07-29T21:40:00Z".to_string(),
            ended_at: None,
            goal: "ship the driver tab".to_string(),
            gsd_command: "/gsd:execute-phase 18".to_string(),
            outcome: outcome.map(str::to_string),
        }
    }

    /// The visible text of a `Line`, spans concatenated.
    fn text(line: &Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn a_run_list_row_is_eighteen_cells_before_the_state_word() {
        let row = run_list_row(&summary("2026-07-29T21-40-00Z-3f2a", None), None, 0);
        assert_eq!(
            text(&row).chars().count(),
            RUN_LIST_FIXED_CELLS,
            "the fixed part of a run row must be one form at every width: {:?}",
            text(&row)
        );
    }

    #[test]
    fn the_state_word_appears_at_twenty_six_cells_and_not_at_twenty_five() {
        let run = summary("2026-07-29T21-40-00Z-3f2a", Some("succeeded_with_changes"));
        let narrow = run_list_row(&run, None, RUN_LIST_WORD_MIN_CELLS - 1);
        let wide = run_list_row(&run, None, RUN_LIST_WORD_MIN_CELLS);
        assert!(!text(&narrow).contains("ok"), "{:?}", text(&narrow));
        assert!(text(&wide).ends_with("  ok"), "{:?}", text(&wide));
    }

    #[test]
    fn elapsed_formats_under_and_over_an_hour_and_switches_form_when_ended() {
        let start = "2026-07-29T21:40:00Z";
        let now = parse_rfc3339("2026-07-29T21:44:12Z").expect("fixture parses");
        assert_eq!(elapsed_label(start, now, None), "+04:12");

        let later = parse_rfc3339("2026-07-29T23:05:07Z").expect("fixture parses");
        assert_eq!(elapsed_label(start, later, None), "+1:25:07");

        // A finished run reports what it took, not what it is taking.
        assert_eq!(
            elapsed_label(start, later, Some("2026-07-29T21:52:41Z")),
            "ran 12:41"
        );

        // A duration the code cannot compute is not a duration of nothing.
        assert_eq!(elapsed_label("not a timestamp", now, None), "unknown");
    }

    #[test]
    fn the_empty_run_list_copy_differs_by_opt_in_state() {
        let opted_in: Vec<String> = no_runs_lines("meta-mgr", true).iter().map(text).collect();
        let not_opted_in: Vec<String> =
            no_runs_lines("meta-mgr", false).iter().map(text).collect();

        assert!(opted_in[0].contains("No runs yet for \"meta-mgr\"."));
        assert!(opted_in[1].contains("[s]"));
        assert!(not_opted_in[0].contains("is not opted in to driving"));
        assert!(not_opted_in[1].contains("[o]"));
        assert_ne!(opted_in, not_opted_in);
    }

    /// T-18-49: an escape sequence in a goal must not reach a `Span`.
    #[test]
    fn a_goal_bearing_an_escape_sequence_is_sanitised_before_it_becomes_a_line() {
        let hostile = "\u{1b}[2Jdeploy \u{1b}]0;pwned\u{7}now";
        let rendered: String = goal_lines(hostile, 80).iter().map(text).collect();
        assert!(
            !rendered.contains('\u{1b}'),
            "an ESC survived into the header: {rendered:?}"
        );
        // The prose itself is still shown — the rule must not pass by deleting
        // everything.
        assert!(rendered.contains("deploy"), "{rendered:?}");
        assert!(rendered.contains("now"), "{rendered:?}");
    }

    #[test]
    fn an_absent_goal_renders_the_pinned_copy_and_nothing_else() {
        for empty in ["", "   ", "\n\t "] {
            let rendered: String = goal_lines(empty, 80).iter().map(text).collect();
            assert!(
                rendered.contains(GOAL_NONE_GIVEN),
                "an absent goal must say so: {rendered:?}"
            );
        }
        // A goal that WAS given is never replaced by the placeholder.
        let given: String = goal_lines("ship the driver tab", 80).iter().map(text).collect();
        assert!(!given.contains(GOAL_NONE_GIVEN), "{given:?}");
        assert!(given.contains("ship the driver tab"), "{given:?}");
    }

    #[test]
    fn a_long_goal_wraps_to_at_most_three_rows_then_ellipses() {
        let long = "ship ".repeat(200);
        let lines = goal_lines(&long, 60);
        assert_eq!(lines.len(), 3);
        assert!(text(&lines[2]).ends_with('\u{2026}'), "{:?}", text(&lines[2]));
    }

    /// Below the two-pane floor the run list is not rendered, and the selection
    /// is still honoured — it moves into the pane title rather than vanishing.
    #[test]
    fn the_sub_sixty_tier_returns_a_single_pane_and_still_honours_the_selection() {
        let narrow = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS - 1, 24);
        let (list, detail) = driver_panes(narrow);
        assert!(list.is_none(), "the run list must be dropped below 60 columns");
        assert_eq!(detail, narrow);

        let wide = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS, 24);
        let (list, detail) = driver_panes(wide);
        assert!(list.is_some(), "two panes at exactly 60 columns");
        assert!(
            detail.width >= DRIVER_DETAIL_MIN_CELLS,
            "the detail pane fell below its floor at 60 columns: {}",
            detail.width
        );

        // The selection still identifies a run at the narrow tier.
        let title = run_list_title(&summary("2026-07-29T21-40-00Z-3f2a", None), 1, 7);
        assert!(title.contains("3f2a"), "{title:?}");
        assert!(title.contains("(2/7)"), "{title:?}");
    }

    /// D-13: every state word, glyph and colour comes from evidence, and the two
    /// "nothing is known" answers are never collapsed into "it died".
    #[test]
    fn every_run_state_maps_from_evidence_and_unknown_is_not_death() {
        assert_eq!(
            run_state_glyph(Some(RunVerdict::Live), None),
            (GLYPH_LIVE, "live", Color::Magenta)
        );
        assert_eq!(
            run_state_glyph(Some(RunVerdict::LivenessUnknown), None),
            (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow)
        );
        assert_eq!(
            run_state_glyph(Some(RunVerdict::CrashedWithoutEnding), None),
            (GLYPH_FAILED, "crash", Color::Red)
        );
        for (label, expected) in [
            ("succeeded_with_changes", (GLYPH_SUCCEEDED, "ok", Color::Green)),
            (
                "succeeded_no_changes",
                (GLYPH_SUCCEEDED_NO_CHANGES, "no-chg", Color::Yellow),
            ),
            ("failed", (GLYPH_FAILED, "fail", Color::Red)),
            ("permission_denied", (GLYPH_FAILED, "denied", Color::Red)),
            ("timed_out", (GLYPH_FAILED, "t/out", Color::Red)),
            ("stalled", (GLYPH_FAILED, "stall", Color::Red)),
            ("capability_refused", (GLYPH_FAILED, "refuse", Color::Red)),
            ("spawn_failed", (GLYPH_FAILED, "spawn", Color::Red)),
            ("killed", (GLYPH_KILLED, "killed", Color::DarkGray)),
        ] {
            assert_eq!(run_state_glyph(None, Some(label)), expected, "{label}");
        }
        // No terminal record and no observation: nothing is known, and that is
        // not a crash report (CR-05).
        assert_eq!(
            run_state_glyph(None, None),
            (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow)
        );
        // `SucceededNoChanges` is its own state, not a flavour of success.
        assert_ne!(
            run_state_glyph(None, Some("succeeded_no_changes")),
            run_state_glyph(None, Some("succeeded_with_changes"))
        );
    }

    /// The cost figure is labelled cumulative, and an unknown cost says so
    /// rather than rendering a zero the mechanism cannot back.
    #[test]
    fn the_cost_line_is_labelled_cumulative_and_never_fabricates_a_figure() {
        let now = parse_rfc3339("2026-07-29T21:44:12Z").expect("fixture parses");
        let run = summary("2026-07-29T21-40-00Z-3f2a", None);

        let known: String = render_run_header(&run, None, Some(1.83), None, 80, now)
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(known.contains("$1.83"), "{known}");
        assert!(known.contains(COST_CUMULATIVE), "{known}");

        let unknown: String = render_run_header(&run, None, None, None, 80, now)
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(unknown.contains(COST_NOT_REPORTED), "{unknown}");
        assert!(!unknown.contains("$0.00"), "{unknown}");
    }

    /// T-18-50: the run directory row is left-truncated, keeping the run id —
    /// the part a reader cannot reconstruct — rather than the leading path.
    #[test]
    fn the_run_directory_row_keeps_its_tail_when_it_does_not_fit() {
        let path = "/home/blk/projects/rust/gsd-meta-manager/.planning/meta-manager/runs/2026-07-29T21-40-00Z-3f2a";
        let shown = left_truncate(path, 40);
        assert_eq!(shown.chars().count(), 40);
        assert!(shown.starts_with('\u{2026}'), "{shown:?}");
        assert!(shown.ends_with("3f2a"), "{shown:?}");
    }

    // ── The live output pane (OBS-04, D-19) ────────────────────────────────

    #[test]
    fn an_empty_journal_renders_the_no_entries_copy() {
        let rendered: String = output_body_lines(None, Color::Green).iter().map(text).collect();
        assert!(rendered.contains(NO_JOURNAL_ENTRIES), "{rendered:?}");

        // A buffer that exists but holds nothing is the same situation.
        let empty = DriverOutput::default();
        let rendered: String = output_body_lines(Some(&empty), Color::Green)
            .iter()
            .map(text)
            .collect();
        assert!(rendered.contains(NO_JOURNAL_ENTRIES), "{rendered:?}");
    }

    #[test]
    fn a_buffer_that_dropped_lines_states_the_count_in_its_first_row() {
        let mut output = DriverOutput::default();
        for n in 0..(DRIVER_OUTPUT_RING_LINES + 5) {
            output.push_record(DriverLineKind::Output, &format!("line {n}"));
        }
        assert_eq!(output.dropped(), 5, "the ring dropped what it was asked to");

        let lines = output_body_lines(Some(&output), Color::Green);
        let first = text(&lines[0]);
        assert!(
            first.contains("5 earlier lines dropped"),
            "the drop count must be the FIRST row: {first:?}"
        );
        assert!(
            first.contains(&DRIVER_OUTPUT_RING_LINES.to_string()),
            "the notice names the capacity so the count is readable: {first:?}"
        );
    }

    #[test]
    fn a_diagnostic_record_renders_as_a_diagnostic_row() {
        let mut output = DriverOutput::default();
        output.push_record(DriverLineKind::Output, "ordinary output");
        output.push_record(DriverLineKind::Stderr, "a warning on stderr");
        output.push_record(DriverLineKind::Diagnostic, "journal gap: 3 record(s) not read");
        output.push_record(DriverLineKind::Terminal, "run ended: succeeded_with_changes");

        let lines = output_body_lines(Some(&output), Color::Green);
        let rendered: Vec<String> = lines.iter().map(text).collect();

        let diagnostic = lines
            .iter()
            .find(|line| text(line).contains("journal gap"))
            .unwrap_or_else(|| panic!("the diagnostic was swallowed: {rendered:#?}"));
        assert_eq!(diagnostic.spans[0].content.as_ref(), MARKER_DIAGNOSTIC);
        assert_eq!(diagnostic.spans[0].style.fg, Some(Color::Yellow));
        assert_eq!(diagnostic.spans[1].style.fg, Some(Color::Yellow));

        // The four visual classes are actually distinct, and the terminal record
        // is the visual full stop.
        assert_eq!(lines[0].spans[0].content.as_ref(), MARKER_OUTPUT);
        assert_eq!(lines[1].spans[0].content.as_ref(), MARKER_STDERR);
        let last = lines.last().expect("a terminal row");
        assert_eq!(last.spans[0].content.as_ref(), MARKER_TERMINAL);
        assert!(text(last).contains("run ended"), "{:?}", text(last));
        assert_eq!(last.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn a_truncated_record_says_so_rather_than_losing_the_lines_silently() {
        let mut output = DriverOutput::default();
        let giant = "row\n".repeat(super::super::DRIVER_OUTPUT_RECORD_MAX_LINES + 10);
        output.push_record(DriverLineKind::Output, &giant);
        assert!(output.record_truncated());

        let rendered: String = output_body_lines(Some(&output), Color::Green)
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("record truncated"), "{rendered}");
    }

    /// The four indicator states, each with its own word and colour — and the
    /// scrolled state deliberately does not spend the word `paused`, which
    /// already means "a non-empty HANDOFF is present" everywhere else.
    #[test]
    fn the_follow_indicator_says_which_of_the_four_states_the_pane_is_in() {
        let (word, color) = follow_indicator(true, true, 0, None, false);
        assert_eq!(word, INDICATOR_FOLLOWING);
        assert_eq!(color, Color::Green);

        let (word, color) = follow_indicator(true, false, 42, None, false);
        assert_eq!(word, "[scrolled +42]");
        assert_eq!(color, Color::Yellow);
        assert!(!word.contains("paused"));

        // A wildly scrolled pane stops widening the field.
        let (word, _) = follow_indicator(true, false, 100_000, None, false);
        assert_eq!(word, "[scrolled +999]");

        let (word, color) = follow_indicator(false, false, 0, Some("2026-07-29T21:52:00Z"), false);
        assert!(word.starts_with("[ended "), "{word:?}");
        assert_eq!(color, Color::DarkGray);

        let (word, color) = follow_indicator(false, false, 0, Some("2026-07-29T21:52:00Z"), true);
        assert_eq!(word, INDICATOR_JOURNAL_ONLY);
        assert_eq!(color, Color::DarkGray);
    }

    /// The header rule keeps the indicator at every width the pane can reach:
    /// the dashes are what give way, never the state word.
    #[test]
    fn the_output_header_keeps_its_indicator_at_every_width() {
        for width in [20u16, 39, 60, 80, 120] {
            let rendered = text(&output_header_line(width, INDICATOR_FOLLOWING, Color::Green));
            assert!(
                rendered.contains(INDICATOR_FOLLOWING),
                "the indicator was dropped at {width} columns: {rendered:?}"
            );
            assert!(rendered.starts_with("\u{2500}\u{2500} output"), "{rendered:?}");
        }
    }

    // ── The reused pipeline row and the one-row step timeline (D-17, D-12) ──

    fn mid_pipeline_inference() -> crate::state_reader::disk_status::DiskInference {
        use crate::state_reader::disk_status::{DiskInference, DiskStatus};
        DiskInference {
            status: DiskStatus::Partial,
            plan_count: 3,
            summary_count: 2,
            has_plans: true,
            has_summaries: true,
            has_context: true,
            has_research: true,
            ..DiskInference::default()
        }
    }

    /// D-17: the Driver tab **calls** the D-R-P-E-V widget rather than growing a
    /// second progress display. The strongest available statement of that is
    /// that the two produce the identical line for the same inference — a
    /// reimplementation, however faithful at the moment it was written, would
    /// drift the first time either side changed.
    #[test]
    fn the_driver_tab_pipeline_line_matches_the_pipeline_tabs() {
        let inf = mid_pipeline_inference();

        // What `render_pipeline_tab` builds, at its own call site's shape.
        let statuses = super::super::detail::derive_all_stage_statuses(&inf);
        let pipeline_tab_line = super::super::detail::build_pipeline_line(&inf, &statuses);

        // What the Driver tab renders: the second line of the pipeline row,
        // under its one blank spacer.
        let mut driver_lines: Vec<Line<'static>> = vec![Line::from("")];
        let driver_statuses = super::super::detail::derive_all_stage_statuses(&inf);
        driver_lines.push(super::super::detail::build_pipeline_line(
            &inf,
            &driver_statuses,
        ));

        assert_eq!(text(&driver_lines[1]), text(&pipeline_tab_line));
        assert_eq!(
            driver_lines[1].spans.len(),
            pipeline_tab_line.spans.len(),
            "the same spans, so the same per-stage colours"
        );
        for (a, b) in driver_lines[1]
            .spans
            .iter()
            .zip(pipeline_tab_line.spans.iter())
        {
            assert_eq!(a.style, b.style, "a stage was restyled: {:?}", a.content);
        }
    }

    #[test]
    fn the_steps_section_has_exactly_one_command_row_and_always_carries_the_note() {
        for (verdict, outcome) in [
            (Some(RunVerdict::Live), None),
            (None, Some("succeeded_with_changes")),
            (None, Some("failed")),
            (Some(RunVerdict::CrashedWithoutEnding), None),
        ] {
            let run = summary("2026-07-29T21-40-00Z-3f2a", outcome);
            let lines = steps_lines(&run, verdict, Some(3), 60);
            assert_eq!(
                lines.len(),
                3,
                "the section rule, ONE decided row and the note: {:?}",
                lines.iter().map(text).collect::<Vec<_>>()
            );
            let command_rows = lines
                .iter()
                .filter(|line| text(line).contains("/gsd:execute-phase 18"))
                .count();
            assert_eq!(command_rows, 1, "one decided command row in this phase");
            assert!(
                text(&lines[2]).contains(STEPS_HONEST_NOTE),
                "the honest note is mandatory whenever the section renders: {:?}",
                text(&lines[2])
            );
        }
    }

    /// The turn counter is a live-run affordance: a finished run's turn count is
    /// history that belongs to nothing on this row, and showing it beside a
    /// terminal state word would read as "still going".
    #[test]
    fn the_turn_counter_appears_only_while_the_run_is_live() {
        let run = summary("2026-07-29T21-40-00Z-3f2a", None);
        let live: String = steps_lines(&run, Some(RunVerdict::Live), Some(3), 60)
            .iter()
            .map(text)
            .collect();
        assert!(live.contains("turn 3"), "{live:?}");
        assert!(live.contains("running"), "{live:?}");

        let ended = summary("2026-07-29T21-40-00Z-3f2a", Some("succeeded_with_changes"));
        let finished: String = steps_lines(&ended, Some(RunVerdict::Ended), Some(3), 60)
            .iter()
            .map(text)
            .collect();
        assert!(!finished.contains("turn 3"), "{finished:?}");
        assert!(!finished.contains("running"), "{finished:?}");
        assert!(finished.contains("ok"), "{finished:?}");
    }

    /// Held-out render-buffer backstop (UI-SPEC `## UI Considerations`, the
    /// D-R-P-E-V overflow row). **A real buffer assertion, not a width
    /// calculation**, because this is the CR-01 / UIFIX-02 clip in a new
    /// location and the original defect was invisible to every calculation the
    /// code had: the trailing `[V]` fell off the right edge of a
    /// percentage-sized pane while every number involved looked right.
    #[test]
    fn the_pipeline_line_keeps_its_verify_stage_at_sixty_eighty_and_one_hundred_twenty_columns() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // The `pub(crate)` fixture the three driver screens already share,
        // rather than a sixth full-field `AppContext` construction.
        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;

        for width in [60u16, 80, 120] {
            let (mut ctx, _rx) =
                super::super::driver_confirm::tests::ctx_with_project(dir.path());
            ctx.project_states.insert(
                alias.to_string(),
                crate::state_reader::ProjectState {
                    current_phase_status: Some(mid_pipeline_inference()),
                    ..crate::state_reader::ProjectState::default()
                },
            );
            let cache = ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_runs = vec![summary("2026-07-29T21-40-00Z-3f2a", None)];

            let viewport = Cell::default();
            let mut terminal =
                Terminal::new(TestBackend::new(width, 30)).expect("TestBackend terminal");
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_driver_tab(
                        frame,
                        area,
                        &ctx,
                        alias,
                        ctx.view_cache.get(alias),
                        &viewport,
                    );
                })
                .expect("draw the driver tab");

            let buffer = terminal.backend().buffer().clone();
            let scraped: Vec<String> = (0..30)
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
                .collect();

            let pipeline_row = scraped
                .iter()
                .find(|row| row.contains("[D]"))
                .unwrap_or_else(|| {
                    panic!("no pipeline row rendered at {width} columns: {scraped:#?}")
                });

            assert!(
                pipeline_row.contains("[E 2/3]"),
                "the execute stage lost its plan fraction at {width} columns: \
                 {pipeline_row:?}"
            );
            assert!(
                pipeline_row.contains("[V]"),
                "the trailing verify stage was clipped at {width} columns — this is \
                 CR-01 / UIFIX-02 in a new place: {pipeline_row:?}"
            );
            // The note is asserted by prefix rather than whole. At exactly 60
            // columns the detail pane is `DRIVER_DETAIL_MIN_CELLS` (39) wide and
            // the note plus its two-cell indent is 40, so its final period is
            // clipped — the sentence stays legible and the floor protects the
            // element it was derived from, which is the pipeline line above.
            let note_prefix = &STEPS_HONEST_NOTE[..30];
            assert!(
                scraped.iter().any(|row| row.contains(note_prefix)),
                "the honest note is missing at {width} columns: {scraped:#?}"
            );
        }
    }
}
