//! The Owner block of the detail Phases tab (36-03, D-15..D-19).
//!
//! A borderless, full-width block above the phase list: a header rule line with
//! the `closes after` gate, one row per current group the owner must act on, and
//! one pinned fold line (Old debt, Acknowledged, Todos). Pure and in-memory:
//! [`block_model`] turns an [`OwnerDigest`] into lines for one width, so nothing
//! on the render path reads a file. 36-04 adds focus, expansion and the cursor.
//!
//! Every digest string reaches a cell through [`shown`] (the reader does not
//! escape), and every width is measured in cells with `Line::width()` and
//! `fit_cells`, never `len()`.

use crate::inbox::{self, ClauseForm};
use crate::state_reader::owner_items::{ClosesAfter, Kind, Lane, OwnerDigest, OwnerGroup, Scope};
use crate::ui::glyphs;
use crate::ui::screens::owner::OWNER_SEP;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::detail::{fit_cells, fit_spans, shown};
use super::driver::muted_style;

/// The block hides below this many tab rows (D-15).
const MIN_TAB_ROWS: u16 = 12;
/// Width of the lane tag column (`BLOCKED`).
const TAG_W: usize = 7;
/// Width of the right-aligned checks column (`12 checks`).
const CHECKS_W: usize = 9;
/// Width of the age column (`123 d stale`).
const AGE_W: usize = 11;
/// Below this width the command column is dropped (D-17).
const COMMAND_MIN_W: usize = 100;
/// Below this width the age column is dropped too (D-17).
const AGE_MIN_W: usize = 80;
/// Smallest the name column gets before the phrase and phase cell give way.
const NAME_MIN_W: usize = 12;

/// The block as lines for one width.
pub(crate) struct BlockModel {
    pub header: Line<'static>,
    pub body: Vec<Line<'static>>,
    pub fold_line: Option<Line<'static>>,
    /// The D-15 predicate: a group row or a fold has something to show.
    pub has_content: bool,
}

fn dim() -> Style {
    muted_style()
}

fn blanks(n: usize) -> String {
    " ".repeat(n)
}

fn cells(text: &str) -> usize {
    Span::raw(text).width()
}

/// `text` cut to `w` cells and padded with blanks to exactly `w` (or `w` blanks
/// in front when `right`).
fn pad(text: &str, w: usize, right: bool) -> String {
    let cut = fit_cells(text, w);
    let gap = w.saturating_sub(cells(&cut));
    if right {
        format!("{}{cut}", blanks(gap))
    } else {
        format!("{cut}{}", blanks(gap))
    }
}

// ---------------------------------------------------------------------------
// Header (D-16)
// ---------------------------------------------------------------------------

fn header_with(
    alias: &str,
    milestone: &str,
    gate: &ClosesAfter,
    form: ClauseForm,
    with_not_built: bool,
) -> Line<'static> {
    let clauses = inbox::closes_after_clauses(gate, form);
    let mut parts = clauses.lanes;
    if with_not_built {
        parts.extend(clauses.not_built);
    }
    let milestone = shown(milestone);
    let mut text = shown(alias);
    if !milestone.trim().is_empty() {
        text.push_str(OWNER_SEP);
        text.push_str(&milestone);
    }
    if !parts.is_empty() {
        text.push_str(" closes after: ");
        text.push_str(&shown(&parts.join(OWNER_SEP)));
    }
    Line::from(vec![
        Span::styled("Owner", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("\u{2500}\u{2500}", dim()),
        Span::raw(format!(" {text}")),
    ])
}

/// The header at `width`: the first whole form of the five-rung ladder.
fn header_line(alias: &str, milestone: &str, gate: &ClosesAfter, width: usize) -> Line<'static> {
    let rungs = [
        (ClauseForm::Full, true),
        (ClauseForm::PhasesCapped(6), true),
        (ClauseForm::CountsOnly, true),
        (ClauseForm::CountsOnly, false),
    ];
    for (form, with_not_built) in rungs {
        let line = header_with(alias, milestone, gate, form, with_not_built);
        if line.width() <= width {
            return line;
        }
    }
    let last = header_with(alias, milestone, gate, ClauseForm::CountsOnly, false);
    fit_spans(last.spans, width)
}

// ---------------------------------------------------------------------------
// Rows (D-17)
// ---------------------------------------------------------------------------

/// One body row before layout: every string already escaped.
struct RowCells {
    lane: Lane,
    /// `ws {name} ` (Magenta) for a workstream group.
    ws_prefix: String,
    phase: String,
    name: String,
    checks: String,
    phrase: String,
    green_phrase: bool,
    bookkeeping: bool,
    age: Option<(String, bool)>,
    command: String,
}

impl RowCells {
    fn phase_cell_width(&self) -> usize {
        cells(&self.ws_prefix) + cells(&self.phase)
    }
}

fn lane_tag(lane: Lane) -> &'static str {
    match lane {
        Lane::Test => "TEST",
        Lane::Decide => "DECIDE",
        Lane::Fix => "FIX",
        Lane::Blocked => "BLOCKED",
        Lane::Deferred => "DEFERRED",
    }
}

fn is_owner_row(group: &OwnerGroup) -> bool {
    group.scope == Scope::Current
        && !group.acknowledged
        && matches!(
            group.lane,
            Lane::Test | Lane::Decide | Lane::Fix | Lane::Blocked
        )
}

fn phase_text(group: &OwnerGroup) -> String {
    match group.kind {
        Kind::Debug => "debug".to_owned(),
        Kind::Handoff => "handoff".to_owned(),
        _ => format!("ph {}", shown(&group.phase)),
    }
}

/// Items waiting in `lane` for the same scope, workstream and phase as `group`.
fn sibling_items(digest: &OwnerDigest, group: &OwnerGroup, lane: Lane) -> usize {
    digest
        .groups
        .iter()
        .filter(|g| {
            g.lane == lane
                && g.scope == group.scope
                && g.workstream == group.workstream
                && g.phase == group.phase
                && !g.acknowledged
        })
        .map(|g| g.items.len())
        .sum()
}

fn test_phrase(digest: &OwnerDigest, group: &OwnerGroup) -> (String, bool) {
    if group.closes_phase {
        return ("closes phase".to_owned(), true);
    }
    let mut parts = Vec::new();
    let fix = sibling_items(digest, group, Lane::Fix);
    if fix > 0 {
        parts.push(format!("+ fix {fix}"));
    }
    let unblock = sibling_items(digest, group, Lane::Blocked);
    if unblock > 0 {
        parts.push(format!("+ unblock {unblock}"));
    }
    (parts.join(" "), false)
}

fn row_cells(digest: &OwnerDigest, group: &OwnerGroup, merged: Option<&OwnerGroup>) -> RowCells {
    let blocked = group.lane == Lane::Blocked;
    let ws_prefix = group
        .workstream
        .as_deref()
        .map(|ws| format!("ws {} ", shown(ws)))
        .unwrap_or_default();
    let name = match merged {
        Some(all) => shown(&inbox::blocked_tally(all)),
        None => shown(&group.phase_name),
    };
    let counted = matches!(group.kind, Kind::Uat | Kind::Verification)
        && !blocked
        && !group.bookkeeping_only
        && group.counts.checks > 0;
    let checks = if counted {
        inbox::plural(group.counts.checks, "check", "checks")
    } else {
        String::new()
    };
    let (phrase, green_phrase) = if blocked {
        (String::new(), false)
    } else if group.lane == Lane::Test {
        if group.bookkeeping_only {
            let text = match group.command {
                Some(_) => inbox::detail(group),
                None => "all checks passed".to_owned(),
            };
            (shown(&text), false)
        } else {
            let (text, green) = test_phrase(digest, group);
            (shown(&text), green)
        }
    } else {
        (shown(&inbox::detail(group)), false)
    };
    let age = group
        .age_days
        .map(|days| (format!("{days} d"), group.stale));
    let command = if blocked || group.bookkeeping_only {
        String::new()
    } else {
        group.command.as_deref().map(shown).unwrap_or_default()
    };
    RowCells {
        lane: group.lane,
        ws_prefix,
        phase: phase_text(group),
        name,
        checks,
        phrase,
        green_phrase,
        bookkeeping: group.bookkeeping_only && group.command.is_some(),
        age,
        command,
    }
}

/// The body rows in D-17 order: TEST (closes-a-phase first), DECIDE, FIX, then one
/// BLOCKED row per (workstream, phase), each lane in digest order, root groups
/// before workstream groups.
fn body_rows(digest: &OwnerDigest) -> Vec<RowCells> {
    let mut picked: Vec<(OwnerGroup, Option<OwnerGroup>)> = Vec::new();
    let mut blocked_seen: Vec<(Option<String>, String)> = Vec::new();
    for group in digest.groups.iter().filter(|g| is_owner_row(g)) {
        if group.lane == Lane::Blocked {
            let key = (group.workstream.clone(), group.phase.clone());
            if blocked_seen.contains(&key) {
                continue;
            }
            let mut merged = group.clone();
            merged.items = digest
                .groups
                .iter()
                .filter(|g| {
                    is_owner_row(g)
                        && g.lane == Lane::Blocked
                        && g.workstream == key.0
                        && g.phase == key.1
                })
                .flat_map(|g| g.items.iter().cloned())
                .collect();
            blocked_seen.push(key);
            picked.push((group.clone(), Some(merged)));
        } else {
            picked.push((group.clone(), None));
        }
    }
    picked.sort_by_key(|(g, _)| {
        (
            g.lane.order(),
            u8::from(g.lane == Lane::Test && !g.closes_phase),
            u8::from(g.workstream.is_some()),
        )
    });
    picked
        .iter()
        .map(|(group, merged)| row_cells(digest, group, merged.as_ref()))
        .collect()
}

/// Column widths for one block width.
struct Grid {
    phase_w: usize,
    name_w: usize,
    phrase_w: usize,
    show_age: bool,
    command_w: usize,
}

fn grid_for(rows: &[RowCells], width: usize) -> Grid {
    let show_age = width >= AGE_MIN_W;
    let longest = |f: fn(&RowCells) -> usize| rows.iter().map(f).max().unwrap_or(0);
    let mut phase_w = longest(RowCells::phase_cell_width).clamp(5, 18);
    // A BLOCKED row's tally spills over the columns it leaves blank, so it does
    // not size the name column.
    let name_nat = rows
        .iter()
        .filter(|r| r.lane != Lane::Blocked)
        .map(|r| cells(&r.name))
        .max()
        .unwrap_or(0)
        .max(NAME_MIN_W);
    let mut phrase_w = rows
        .iter()
        .filter(|r| !r.bookkeeping)
        .map(|r| cells(&r.phrase))
        .max()
        .unwrap_or(0)
        .clamp(12, 30);
    let command_nat = if width >= COMMAND_MIN_W {
        longest(|r| cells(&r.command))
    } else {
        0
    };
    let age_part = if show_age { 2 + AGE_W } else { 0 };
    // gutter + tag + 1, the phase cell and its gap, the name's gap, the checks
    // column, the phrase column with its gap, and the age.
    let fixed = |phase_w: usize, phrase_w: usize| {
        2 + TAG_W + 1 + phase_w + 2 + 2 + CHECKS_W + 2 + phrase_w + age_part
    };
    // The phrase gives way first (down to 12), then the phase cell (down to 5).
    while fixed(phase_w, phrase_w) + NAME_MIN_W > width && phrase_w > 12 {
        phrase_w -= 1;
    }
    while fixed(phase_w, phrase_w) + NAME_MIN_W > width && phase_w > 5 {
        phase_w -= 1;
    }
    let avail = width.saturating_sub(fixed(phase_w, phrase_w));
    let (name_w, command_w) = if command_nat > 0 {
        let name_w = name_nat.min(avail.saturating_sub(2 + command_nat).max(NAME_MIN_W));
        let command_w = avail.saturating_sub(name_w + 2).min(command_nat);
        (name_w.min(avail), command_w)
    } else {
        (name_nat.min(avail), 0)
    };
    Grid {
        phase_w,
        name_w,
        phrase_w,
        show_age,
        command_w,
    }
}

fn style_for(row: &RowCells, style: Style) -> Style {
    if row.lane == Lane::Blocked {
        style.add_modifier(Modifier::DIM)
    } else {
        style
    }
}

fn age_spans(age: &(String, bool), row: &RowCells) -> Vec<Span<'static>> {
    let mut out = vec![Span::styled(age.0.clone(), dim())];
    if age.1 {
        out.push(Span::styled(" ", dim()));
        out.push(Span::styled(
            "stale",
            style_for(row, Style::default().fg(Color::Yellow)),
        ));
    }
    out
}

/// The row as spans, cut to `width` and without trailing blanks.
fn row_line(row: &RowCells, grid: &Grid, width: usize) -> Line<'static> {
    let plain = style_for(row, Style::default());
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw("  "));
    let tag_style = if row.lane == Lane::Blocked {
        dim()
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    spans.push(Span::styled(
        format!("{} ", pad(lane_tag(row.lane), TAG_W, false)),
        tag_style,
    ));
    // Phase cell: the Magenta `ws` prefix, then the phase text.
    let full = format!("{}{}", row.ws_prefix, row.phase);
    let cut = fit_cells(&full, grid.phase_w);
    let gap = blanks(grid.phase_w.saturating_sub(cells(&cut)));
    if !row.ws_prefix.is_empty() {
        let magenta = style_for(row, Style::default().fg(Color::Magenta));
        match cut.strip_prefix(row.ws_prefix.as_str()) {
            Some(rest) => {
                spans.push(Span::styled(row.ws_prefix.clone(), magenta));
                spans.push(Span::styled(format!("{rest}{gap}"), plain));
            }
            None => spans.push(Span::styled(format!("{cut}{gap}"), magenta)),
        }
    } else {
        spans.push(Span::styled(format!("{cut}{gap}"), plain));
    }
    spans.push(Span::raw("  "));
    if row.lane == Lane::Blocked {
        let used = 2 + TAG_W + 1 + grid.phase_w + 2;
        spans.push(Span::styled(
            fit_cells(&row.name, width.saturating_sub(used)),
            plain,
        ));
    } else {
        spans.push(Span::styled(pad(&row.name, grid.name_w, false), plain));
    }
    if row.lane != Lane::Blocked {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(pad(&row.checks, CHECKS_W, true), plain));
        spans.push(Span::raw("  "));
        // The phase column, which a bookkeeping phrase may spill out of across
        // the age and command columns (their cells are blank on that row).
        let mut phrase = row.phrase.clone();
        let mut spilled = false;
        if row.bookkeeping && cells(&phrase) > grid.phrase_w {
            let region = grid.phrase_w
                + if grid.show_age { 2 + AGE_W } else { 0 }
                + if grid.command_w > 0 {
                    2 + grid.command_w
                } else {
                    0
                };
            if cells(&phrase) <= region {
                spilled = true;
            } else {
                phrase = "all checks passed".to_owned();
            }
        }
        let phrase_style = if row.green_phrase {
            style_for(row, Style::default().fg(Color::Green))
        } else {
            plain
        };
        if spilled {
            spans.push(Span::styled(phrase, phrase_style));
        } else {
            spans.push(Span::styled(
                pad(&phrase, grid.phrase_w, false),
                phrase_style,
            ));
            if grid.show_age {
                spans.push(Span::raw("  "));
                match &row.age {
                    Some(age) => {
                        let used: usize = age_spans(age, row).iter().map(Span::width).sum();
                        spans.extend(age_spans(age, row));
                        spans.push(Span::raw(blanks(AGE_W.saturating_sub(used))));
                    }
                    None => spans.push(Span::raw(blanks(AGE_W))),
                }
            }
            if grid.command_w > 0 && !row.command.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(fit_cells(&row.command, grid.command_w), plain));
            }
        }
    }
    // Trailing blanks carry no meaning and are not drawn.
    while spans
        .last()
        .is_some_and(|span| span.content.trim().is_empty())
    {
        spans.pop();
    }
    if let Some(last) = spans.last_mut() {
        let trimmed = last.content.trim_end().to_owned();
        last.content = trimmed.into();
    }
    fit_spans(spans, width)
}

// ---------------------------------------------------------------------------
// Fold line (D-19, collapsed)
// ---------------------------------------------------------------------------

/// The Old debt entries: the first two distinct (label, phase) archived groups
/// the inbox would list with `--include archived`, and whether more follow.
fn old_debt_summary(digest: &OwnerDigest) -> String {
    let filters = inbox::Filters {
        archived: true,
        ..Default::default()
    };
    let mut seen: Vec<String> = Vec::new();
    for group in &digest.groups {
        let Scope::Archived { label, .. } = &group.scope else {
            continue;
        };
        if !filters.lists_group(group) {
            continue;
        }
        let entry = format!("{} ph {}", shown(label), shown(&group.phase));
        if !seen.contains(&entry) {
            seen.push(entry);
        }
    }
    let mut text = seen.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
    if seen.len() > 2 {
        text.push_str(" \u{2026}");
    }
    text
}

fn acknowledged_files(digest: &OwnerDigest) -> usize {
    if digest.hidden.acknowledged_files > 0 {
        return digest.hidden.acknowledged_files;
    }
    let mut files: Vec<&str> = digest
        .groups
        .iter()
        .filter(|g| g.acknowledged)
        .map(|g| g.file.as_str())
        .collect();
    files.sort_unstable();
    files.dedup();
    files.len()
}

fn fold_spans(parts: &[(String, bool)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, (text, muted)) in parts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        let style = if *muted { dim() } else { Style::default() };
        spans.push(Span::styled(
            format!("{} {text}", glyphs::TRIANGLE_RIGHT_SMALL),
            style,
        ));
    }
    spans
}

fn fold_line(digest: &OwnerDigest, width: usize) -> Option<Line<'static>> {
    let old = digest.archived.test
        + digest.archived.blocked
        + digest.archived.fix
        + digest.archived.decide;
    let acknowledged = acknowledged_files(digest);
    let hidden = &digest.hidden;
    let todos_shown = hidden.todos > 0 || hidden.todos_unreadable;
    if old == 0 && acknowledged == 0 && !todos_shown {
        return None;
    }
    let summary = old_debt_summary(digest);
    // The Todos text for the full and the short forms.
    let (todos_full, todos_short) = if hidden.todos_unreadable {
        ("Todos ?".to_owned(), "Todos ?".to_owned())
    } else {
        let full = match hidden.todos_oldest_days {
            Some(days) => format!("Todos {} (oldest {days} d)", hidden.todos),
            None => format!("Todos {}", hidden.todos),
        };
        (full, format!("Todos {}", hidden.todos))
    };
    let build = |old_text: String, ack_text: String, todo_text: &str| {
        let mut parts: Vec<(String, bool)> = Vec::new();
        if old > 0 {
            parts.push((old_text, false));
        }
        if acknowledged > 0 {
            parts.push((ack_text, false));
        }
        if todos_shown {
            parts.push((todo_text.to_owned(), true));
        }
        fold_spans(&parts)
    };
    let with_summary = if summary.is_empty() {
        format!("Old debt ({old})")
    } else {
        format!("Old debt ({old}): {summary}")
    };
    let ack_full = format!(
        "Acknowledged ({})",
        inbox::plural(acknowledged, "file", "files")
    );
    let rungs = [
        build(with_summary, ack_full.clone(), &todos_full),
        build(format!("Old debt ({old})"), ack_full, &todos_full),
        build(
            format!("Old debt ({old})"),
            format!("Acknowledged ({acknowledged})"),
            &todos_short,
        ),
    ];
    for spans in &rungs {
        let line = Line::from(spans.clone());
        if line.width() <= width {
            return Some(line);
        }
    }
    let last = rungs.into_iter().last().unwrap_or_default();
    Some(fit_spans(last, width))
}

// ---------------------------------------------------------------------------
// Model, height, render
// ---------------------------------------------------------------------------

/// The block for one project at `width` cells.
pub(crate) fn block_model(
    alias: &str,
    milestone: &str,
    digest: &OwnerDigest,
    width: u16,
) -> BlockModel {
    let width = usize::from(width);
    let rows = body_rows(digest);
    let grid = grid_for(&rows, width);
    let body: Vec<Line<'static>> = rows.iter().map(|row| row_line(row, &grid, width)).collect();
    let fold_line = fold_line(digest, width);
    BlockModel {
        header: header_line(alias, milestone, &digest.closes_after, width),
        has_content: !body.is_empty() || fold_line.is_some(),
        body,
        fold_line,
    }
}

/// Rows the block takes in a tab `tab_height` rows tall (D-15): zero when there is
/// nothing to show or the tab is short, else the header, the body rows and the fold
/// line, capped at 40% of the tab (at least 4 rows).
pub(crate) fn block_height(model: &BlockModel, tab_height: u16) -> u16 {
    if !model.has_content || tab_height < MIN_TAB_ROWS {
        return 0;
    }
    let wanted = 1 + model.body.len() + usize::from(model.fold_line.is_some());
    let cap = usize::from((tab_height * 40 / 100).max(4));
    u16::try_from(wanted.min(cap)).unwrap_or(u16::MAX)
}

/// Draw the block into `area` (its height is [`block_height`]): the header, the
/// body rows from `offset`, and the fold line pinned as the last row. When body
/// rows are clipped the header's right edge shows the DIM row range.
pub(crate) fn render_owner_block(frame: &mut Frame, area: Rect, model: &BlockModel, offset: usize) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let fold = usize::from(model.fold_line.is_some() && area.height > 1);
    let window = usize::from(area.height).saturating_sub(1 + fold);
    let total = model.body.len();
    let offset = offset.min(total.saturating_sub(window));
    let hint = (total > window && window > 0).then(|| {
        format!(
            " {}-{} of {total}",
            offset + 1,
            (offset + window).min(total)
        )
    });
    let header_rect = Rect { height: 1, ..area };
    match hint {
        Some(hint) if usize::from(area.width) > cells(&hint) => {
            let parts = Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(u16::try_from(cells(&hint)).unwrap_or(u16::MAX)),
            ])
            .split(header_rect);
            let header = fit_spans(model.header.spans.clone(), usize::from(parts[0].width));
            frame.render_widget(Paragraph::new(header), parts[0]);
            frame.render_widget(Paragraph::new(Span::styled(hint, dim())), parts[1]);
        }
        _ => frame.render_widget(Paragraph::new(model.header.clone()), header_rect),
    }
    if window > 0 {
        let rows: Vec<Line<'static>> = model
            .body
            .iter()
            .skip(offset)
            .take(window)
            .cloned()
            .collect();
        let body_rect = Rect {
            y: area.y + 1,
            height: u16::try_from(window).unwrap_or(u16::MAX),
            ..area
        };
        frame.render_widget(Paragraph::new(rows), body_rect);
    }
    if let (1, Some(line)) = (fold, &model.fold_line) {
        let rect = Rect {
            y: area.y + area.height - 1,
            height: 1,
            ..area
        };
        frame.render_widget(Paragraph::new(line.clone()), rect);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A synthetic digest modeled on the UI-SPEC fixture, reused by the detail tests.
#[cfg(test)]
pub(crate) fn sample_owner_digest() -> OwnerDigest {
    use crate::state_reader::owner_items::{
        ArchivedCounts, Category, GroupCounts, Hidden, LanePhases, OwnerItem,
    };
    let item = |name: &str, category: Category| OwnerItem {
        id: name.to_owned(),
        test: None,
        name: name.to_owned(),
        expected: None,
        result: "pending".to_owned(),
        category,
        reason: None,
        blocked_by: None,
        gaps: Vec::new(),
        why_human: None,
    };
    let group = |lane: Lane, kind: Kind, phase: &str, name: &str| OwnerGroup {
        lane,
        kind,
        scope: Scope::Current,
        workstream: None,
        phase: phase.to_owned(),
        phase_name: name.to_owned(),
        file: format!("phases/{phase}/{phase}-UAT.md"),
        file_status: "partial".to_owned(),
        closes_phase: false,
        bookkeeping_only: false,
        acknowledged: false,
        age_days: Some(2),
        stale: false,
        command: Some(format!("/gsd-verify-work {phase}")),
        supersedes: Vec::new(),
        counts: GroupCounts::default(),
        items: Vec::new(),
    };
    let with_items = |mut g: OwnerGroup, n: usize| {
        g.items = (0..n)
            .map(|i| item(&format!("check {i}"), Category::HumanUat))
            .collect();
        g.counts = GroupCounts {
            checks: n,
            ..GroupCounts::default()
        };
        g
    };
    let mut test_24 = with_items(group(Lane::Test, Kind::Uat, "24", "clock-faces"), 6);
    test_24.closes_phase = true;
    let mut test_21 = with_items(group(Lane::Test, Kind::Uat, "21", "android-faces"), 2);
    test_21.closes_phase = true;
    test_21.age_days = Some(29);
    test_21.stale = true;
    let test_25 = with_items(group(Lane::Test, Kind::Uat, "25", "first-unlock"), 3);
    let mut test_22 = group(Lane::Test, Kind::Uat, "22", "notif-copy");
    test_22.bookkeeping_only = true;
    let mut debug = group(Lane::Decide, Kind::Debug, "", "widget-tap-bug");
    debug.command = Some("/gsd-debug".to_owned());
    debug.file_status = "awaiting_human_verify".to_owned();
    let mut handoff = group(Lane::Decide, Kind::Handoff, "26", "voice-pass");
    handoff.workstream = Some("v14".to_owned());
    handoff.file_status = "paused".to_owned();
    handoff.command = Some("/gsd-resume-work --ws v14".to_owned());
    handoff.age_days = None;
    let mut fix_25 = with_items(group(Lane::Fix, Kind::Verification, "25", "quota-clip"), 1);
    fix_25.counts.failed = 1;
    fix_25.counts.gaps = 1;
    fix_25.command = Some("/gsd-plan-phase 25 --gaps".to_owned());
    fix_25.age_days = None;
    let mut blocked_a = group(Lane::Blocked, Kind::Uat, "23", "store-listing");
    blocked_a.items = vec![
        item("a", Category::DeviceNeeded),
        item("b", Category::DeviceNeeded),
        item("c", Category::ThirdParty),
    ];
    let mut blocked_b = group(Lane::Blocked, Kind::Verification, "23", "store-listing");
    blocked_b.items = vec![item("d", Category::Pending)];
    let archived = |label: &str, rank: usize, lane: Lane, phase: &str| {
        let mut g = group(lane, Kind::Uat, phase, "old-work");
        g.scope = Scope::Archived {
            label: label.to_owned(),
            rank,
        };
        g
    };
    let mut ack_1 = group(Lane::Test, Kind::Uat, "19", "ack-one");
    ack_1.acknowledged = true;
    let mut ack_2 = group(Lane::Fix, Kind::Verification, "18", "ack-two");
    ack_2.acknowledged = true;
    let deferred = group(Lane::Deferred, Kind::Deferred, "20", "later");
    let ph = |list: &[&str]| list.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();
    OwnerDigest {
        groups: vec![
            test_21,
            test_22,
            test_24,
            test_25,
            debug,
            handoff,
            fix_25,
            blocked_a,
            blocked_b,
            archived("v1.1", 0, Lane::Test, "04"),
            archived("v1.2", 1, Lane::Fix, "08"),
            archived("v1.2", 1, Lane::Blocked, "09"),
            ack_1,
            ack_2,
            deferred,
        ],
        test: 12,
        decide: 1,
        fix_phases: 1,
        fix_debug: 0,
        blocked: 5,
        closeable_phases: 2,
        archived: ArchivedCounts {
            test: 3,
            blocked: 1,
            fix: 2,
            decide: 0,
        },
        hidden: Hidden {
            deferred: 1,
            acknowledged_files: 8,
            noise: 0,
            todos: 11,
            todos_oldest_days: Some(96),
            todos_unreadable: false,
        },
        closes_after: ClosesAfter {
            test: 12,
            decide: 1,
            fix: 1,
            blocked: 5,
            phases_open: ph(&["20", "21", "22", "23", "24", "25", "26"]),
            not_built: ph(&["27"]),
            lane_phases: LanePhases {
                test: ph(&["21", "22", "24"]),
                decide: ph(&["26"]),
                fix: ph(&["25"]),
                blocked: ph(&["20", "23"]),
            },
        },
        ..OwnerDigest::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn the_header_walks_the_five_rungs() {
        let digest = sample_owner_digest();
        let at = |w: u16| text(&block_model("sample", "v2.2", &digest, w).header);
        assert_eq!(
            at(140),
            "Owner \u{2500}\u{2500} sample \u{b7} v2.2 closes after: test 12 (ph 21 22 24) \u{b7} decide 1 (ph 26) \u{b7} unblock 5 (ph 20 23) \u{b7} fix 1 (ph 25) \u{b7} not built ph 27"
        );
        let counts = at(110);
        assert!(counts.ends_with("not built ph 27"), "{counts}");
        assert!(counts.contains("test 12 \u{b7} decide 1"), "{counts}");
        let dropped = at(75);
        assert!(!dropped.contains("not built"), "{dropped}");
        assert!(dropped.contains("fix 1"), "{dropped}");
        for w in [21u16, 40, 75, 100, 139, 140] {
            let header = block_model("sample", "v2.2", &digest, w).header;
            assert!(header.width() <= usize::from(w), "{w}: {}", text(&header));
        }
        assert!(text(&block_model("sample", "v2.2", &digest, 21).header).ends_with('\u{2026}'));
    }

    #[test]
    fn a_capped_phase_list_is_the_second_rung() {
        let mut digest = sample_owner_digest();
        digest.closes_after.lane_phases.test = (1..=9).map(|n| format!("{n}")).collect();
        let full = text(&header_with(
            "sample",
            "v2.2",
            &digest.closes_after,
            ClauseForm::Full,
            true,
        ));
        let width = full.chars().count() - 1;
        let header = text(&header_line("sample", "v2.2", &digest.closes_after, width));
        assert!(header.contains("(ph 1 2 3 4 5 6 \u{2026})"), "{header}");
    }

    #[test]
    fn a_blank_milestone_drops_its_part_and_no_clause_drops_closes_after() {
        let mut digest = sample_owner_digest();
        let header = text(&block_model("sample", "  ", &digest, 200).header);
        assert!(
            header.starts_with("Owner \u{2500}\u{2500} sample closes after:"),
            "{header}"
        );
        digest.closes_after = ClosesAfter::default();
        let header = text(&block_model("sample", "v2.2", &digest, 200).header);
        assert_eq!(header, "Owner \u{2500}\u{2500} sample \u{b7} v2.2");
    }

    #[test]
    fn long_header_strings_are_cut_at_the_width() {
        let digest = sample_owner_digest();
        let long = "x".repeat(300);
        for w in [21u16, 80, 120] {
            let model = block_model(&long, &long, &digest, w);
            assert!(model.header.width() <= usize::from(w));
        }
    }

    #[test]
    fn the_height_follows_content_and_tab() {
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, 120);
        assert_eq!(block_height(&model, 11), 0);
        assert_eq!(block_height(&model, 12), 4);
        assert!(block_height(&model, 30) >= 1);
        assert!(block_height(&model, 30) <= 12);
        let empty = block_model("sample", "v2.2", &OwnerDigest::default(), 120);
        assert!(!empty.has_content);
        assert_eq!(block_height(&empty, 30), 0);
    }

    #[test]
    fn only_acknowledged_archived_zero_and_deferred_show_nothing() {
        let mut digest = sample_owner_digest();
        digest
            .groups
            .retain(|g| g.acknowledged || g.lane == Lane::Deferred);
        digest.archived = Default::default();
        digest.hidden.acknowledged_files = 0;
        digest.hidden.todos = 0;
        digest.hidden.todos_oldest_days = None;
        // Acknowledged groups still earn the Acknowledged fold (D-15).
        let model = block_model("sample", "v2.2", &digest, 120);
        assert!(model.body.is_empty());
        assert!(model.fold_line.is_some());
        digest.groups.retain(|g| g.lane == Lane::Deferred);
        let model = block_model("sample", "v2.2", &digest, 120);
        assert!(!model.has_content);
        assert_eq!(block_height(&model, 30), 0);
    }

    #[test]
    fn the_rows_follow_the_inbox_order_and_phrases() {
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, 140);
        let rows: Vec<String> = model.body.iter().map(text).collect();
        assert_eq!(rows.len(), 8, "{rows:#?}");
        let tags: Vec<&str> = rows
            .iter()
            .map(|r| r.split_whitespace().next().unwrap())
            .collect();
        assert_eq!(
            tags,
            ["TEST", "TEST", "TEST", "TEST", "DECIDE", "DECIDE", "FIX", "BLOCKED"]
        );
        // Closes-a-phase first, in digest order, then the rest.
        assert!(
            rows[0].contains("ph 21") && rows[0].contains("closes phase"),
            "{}",
            rows[0]
        );
        assert!(rows[1].contains("ph 24") && rows[1].contains("closes phase"));
        assert!(
            rows[2].contains("ph 22")
                && rows[2].contains("all checks passed, run /gsd-verify-work 22 to finalize")
        );
        assert!(
            rows[3].contains("ph 25") && rows[3].contains("+ fix 1"),
            "{}",
            rows[3]
        );
        assert!(rows[4].contains("debug") && rows[4].contains("awaiting your confirmation"));
        assert!(rows[5].contains("ws v14 handoff"), "{}", rows[5]);
        assert!(rows[6].contains("1 failed check, 1 gap") && rows[6].contains("1 check"));
        assert!(rows[6].contains("/gsd-plan-phase 25 --gaps"));
        assert!(
            rows[7].contains("ph 23")
                && rows[7].contains("2 device \u{b7} 1 third party \u{b7} 1 other")
        );
        assert!(rows[0].contains("29 d stale"), "{}", rows[0]);
        // No acknowledged, archived or Deferred row.
        assert!(!rows
            .iter()
            .any(|r| r.contains("ack-") || r.contains("old-work") || r.contains("later")));
        // A bookkeeping-only row carries no command column of its own.
        assert_eq!(rows[2].matches("/gsd-verify-work 22").count(), 1);
    }

    #[test]
    fn the_fold_line_names_the_three_folds() {
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, 120);
        let fold = text(model.fold_line.as_ref().unwrap());
        assert_eq!(
            fold,
            "\u{25b8} Old debt (6): v1.1 ph 04, v1.2 ph 08 \u{2026}   \u{25b8} Acknowledged (8 files)   \u{25b8} Todos 11 (oldest 96 d)"
        );
    }

    #[test]
    fn two_archived_groups_end_without_an_ellipsis_and_unreadable_todos_read_a_question_mark() {
        let mut digest = sample_owner_digest();
        digest
            .groups
            .retain(|g| !(matches!(&g.scope, Scope::Archived { .. }) && g.phase == "09"));
        digest.hidden.todos_unreadable = true;
        digest.hidden.todos = 0;
        let fold = text(
            block_model("s", "m", &digest, 120)
                .fold_line
                .as_ref()
                .unwrap(),
        );
        assert!(fold.contains("v1.1 ph 04, v1.2 ph 08   "), "{fold}");
        assert!(fold.ends_with("Todos ?"), "{fold}");
        digest.hidden.todos_unreadable = false;
        digest.hidden.todos_oldest_days = None;
        let fold = text(
            block_model("s", "m", &digest, 120)
                .fold_line
                .as_ref()
                .unwrap(),
        );
        assert!(!fold.contains("Todos"), "{fold}");
        digest.hidden.acknowledged_files = 1;
        let fold = text(
            block_model("s", "m", &digest, 120)
                .fold_line
                .as_ref()
                .unwrap(),
        );
        assert!(fold.contains("Acknowledged (1 file)"), "{fold}");
    }

    #[test]
    fn the_fold_line_ladder_has_four_rungs() {
        let digest = sample_owner_digest();
        let at = |w: u16| {
            let line = block_model("s", "m", &digest, w).fold_line.unwrap();
            assert!(line.width() <= usize::from(w));
            text(&line)
        };
        assert!(at(120).contains(": v1.1 ph 04"));
        let two = at(80);
        assert!(
            !two.contains(": v1.1") && two.contains("Acknowledged (8 files)"),
            "{two}"
        );
        let three = at(60);
        assert!(
            three.contains("Acknowledged (8)")
                && three.contains("Todos 11")
                && !three.contains("oldest"),
            "{three}"
        );
        assert!(at(30).ends_with('\u{2026}'));
    }

    #[test]
    fn has_content_matches_rows_or_fold() {
        for digest in [sample_owner_digest(), OwnerDigest::default()] {
            let m = block_model("s", "m", &digest, 120);
            assert_eq!(m.has_content, !m.body.is_empty() || m.fold_line.is_some());
        }
        let mut fold_only = OwnerDigest::default();
        fold_only.hidden.todos = 3;
        let m = block_model("s", "m", &fold_only, 120);
        assert!(m.has_content && m.body.is_empty());
        assert_eq!(block_height(&m, 30), 2);
    }

    #[test]
    fn long_texts_stay_inside_the_width() {
        let long = "y".repeat(300);
        let mut digest = sample_owner_digest();
        for g in &mut digest.groups {
            g.phase_name = long.clone();
            g.command = Some(format!("/gsd-verify-work {long}"));
            g.phase = long.clone();
            if let Scope::Archived { label, .. } = &mut g.scope {
                *label = long.clone();
            }
        }
        digest.closes_after.lane_phases.test = vec![long.clone(); 3];
        for w in [21u16, 40, 79, 80, 99, 100, 120] {
            let model = block_model(&long, &long, &digest, w);
            for line in model
                .body
                .iter()
                .chain(model.fold_line.iter())
                .chain([&model.header])
            {
                assert!(line.width() <= usize::from(w), "{w}: {}", text(line));
            }
        }
    }
}
