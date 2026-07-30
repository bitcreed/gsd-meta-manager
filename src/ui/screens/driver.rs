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

use std::cell::Cell;

use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::detail::{clamp_scroll, ViewportMetrics};
use super::{sanitize_render_line, AppContext, ProjectViewCache};
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
/// `Constraint::Min(DRIVER_DETAIL_MIN_CELLS)` rather than
/// `Constraint::Percentage(60)` — the discipline that keeps the reused D-R-P-E-V
/// line from losing its trailing `[V]`. Below
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

    let goal_row_count = goal_rows(&summary.goal, area.width);
    let header_rows = goal_row_count + 3;

    // Task 3 of plan 18-09 inserts the pipeline row and the step timeline
    // between the header and the output pane; the tiers below are the same
    // shape with those two sections added.
    let chunks: Vec<Rect> = if area.height >= 14 {
        Layout::vertical([Constraint::Length(header_rows), Constraint::Min(3)])
            .split(area)
            .to_vec()
    } else if area.height >= 8 {
        Layout::vertical([Constraint::Length(2), Constraint::Min(3)])
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
        render_output_section(frame, chunks[1], ctx, alias, cache, viewport);
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

    let _ = turns;
    render_output_section(frame, chunks[1], ctx, alias, cache, viewport);
}

/// The output pane's section rule and, for now, its content.
///
/// **Plan 18-10 owns this pane's real rendering** — the follow bit and its
/// indicator, the four-state injection widget, the per-kind marker column, the
/// adopted-run notice and the ring-overflow affordances. What lands here is the
/// section rule, the empty state this plan owns, and the buffered lines as plain
/// text, plus the viewport capture the clamp ordering depends on.
fn render_output_section(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    viewport: &Cell<ViewportMetrics>,
) {
    let mut lines: Vec<Line<'static>> = vec![section_rule("output", area.width)];

    let buffered = ctx.driver_output.get(alias);
    match buffered.filter(|output| !output.is_empty()) {
        None => lines.push(Line::from(Span::styled(
            format!("  {NO_JOURNAL_ENTRIES}"),
            label_style(),
        ))),
        Some(output) => {
            for line in output.lines() {
                lines.push(Line::from(format!("  {}", line.text)));
            }
        }
    }

    // The same `Cell<ViewportMetrics>` capture the Browse and Archive viewers
    // use, feeding the same `clamp_scroll` formula — one formula, so the key
    // handler and the renderer cannot drift apart (UIFIX-04).
    let total_lines = lines.len() as u16;
    viewport.set(ViewportMetrics {
        total_lines,
        visible_height: area.height,
    });
    let scroll = clamp_scroll(
        cache.map_or(0, |c| c.driver_scroll_offset),
        total_lines,
        area.height,
    );
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
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
}
