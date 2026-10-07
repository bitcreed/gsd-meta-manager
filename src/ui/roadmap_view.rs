//! The Roadmap tab's master/detail widget (phase 24: D-A01, D-A02, D-A09, D-A13).
//!
//! [`RoadmapView`] draws a [`RoadmapModel`] from `ui::roadmap_graph` as a phase
//! list with a narrow git-log lane column, plus a detail pane for the row under
//! the cursor. The detail pane sits beside the list at
//! [`ROADMAP_SIDE_BY_SIDE_MIN_COLS`] columns and above, and stacks under it
//! below that. Nothing scrolls horizontally: the name column absorbs the width.
//!
//! **Contract.**
//!
//! * **Only the model's stored text and this file's static literals are drawn.**
//!   `roadmap_graph` escapes every third-party string (id, name, goal, badge,
//!   band label) through `crate::text::render_for_terminal` before storing it.
//!   The model's keys (`PhaseFacts::key`, `BandKey::Named`) are raw-derived
//!   and used here for matching only. They are never drawn.
//! * **Measured by chars.** Every width and every truncation is
//!   `chars().count()` of the stored text, never `str::len` or a byte slice.
//!   A multibyte name therefore cannot panic the render.
//! * **Clipped to its `Rect`.** The area is intersected with the buffer, and
//!   every cell is written through a `Block` or `Paragraph` rendered into a
//!   sub-rect of it.

use crate::ui::roadmap_graph::{
    BandFacts, BandKey, CursorTarget, LaneLink, ListRow, PhaseFacts, PhaseStatus, RoadmapModel,
    BAND_FILL, BAND_FOLDED, BAND_OPEN, GLYPH_UNMERGED, MARK_DEP, MARK_IMPLIED, MARK_SELECTED,
    MARK_UNBLOCKS, PARALLEL_SEP,
};
use crate::ui::LANE_PALETTE;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph, StatefulWidget, Widget, Wrap};

/// At this Roadmap area width and above, the detail pane sits beside the list
/// (D-A09). Below it, the pane stacks under the list.
pub const ROADMAP_SIDE_BY_SIDE_MIN_COLS: u16 = 100;
/// Lanes shown below [`ROADMAP_SIDE_BY_SIDE_MIN_COLS`] before the rest collapse
/// into one [`LANE_OVERFLOW`] column.
pub const LANE_CAP_NARROW: usize = 4;
/// Lanes shown at [`ROADMAP_SIDE_BY_SIDE_MIN_COLS`] and above.
pub const LANE_CAP_WIDE: usize = 6;
/// The one column that stands for every lane past the cap (`┆`).
pub const LANE_OVERFLOW: &str = "\u{2506}";

/// Side-by-side detail pane width bounds.
const DETAIL_MIN_COLS: u16 = 40;
const DETAIL_MAX_COLS: u16 = 56;
/// Stacked detail pane height, borders included (Mockup C).
const DETAIL_STACKED_ROWS: u16 = 9;
/// The least the stacked detail pane shrinks to for a list that needs the
/// rows: its head, one goal row, Needs and Parallel inside the borders.
const DETAIL_STACKED_MIN_ROWS: u16 = 6;
/// The narrowest lane column.
const MIN_LANE_CELLS: usize = 8;
/// The marker column (` ▶ `).
const MARK_CELLS: usize = 3;
/// The widest id column; longer ids are truncated.
const MAX_ID_CELLS: usize = 8;
/// The plans column (`  0/3`, right-aligned).
const PLANS_CELLS: usize = 5;
/// The wave column (`W4`).
const WAVE_CELLS: usize = 4;
/// The gap between list columns.
const GAP: &str = "  ";
/// The detail pane's section label column (`Needs     `).
const LABEL_CELLS: usize = 10;
/// Footer hint of the side-by-side phase detail pane.
const HINT_WIDE: &str = "\u{23CE} open in Phases   h/l follow edge";
/// Footer hint of the stacked phase detail pane (Mockup C).
const HINT_STACKED: &str = "\u{23CE} open in Phases   h/l follow edge   j/k move";
/// Footer hint of a band's detail pane.
const HINT_BAND: &str = "Space fold/unfold";
/// Footer hint of the shipped summary's detail pane.
const HINT_SHIPPED: &str = "\u{23CE} open milestones   Space fold/unfold";
/// Appended to a stacked pane's hint.
const HINT_MOVE: &str = "   j/k move";
/// The hanging indent under a section label.
const INDENT: &str = "          ";
/// What an empty model draws.
const EMPTY: &str = "No roadmap data available";
/// Marks the cut in truncated text.
const ELLIPSIS: char = '\u{2026}';
/// The plans column when the count is unknown.
const NO_PLANS: &str = "\u{2014}";
/// Separator inside a detail line.
const DOT: &str = " \u{00B7} ";

/// The Roadmap tab's list + detail widget. `cursor` is resolved through
/// [`RoadmapModel::resolve_cursor`], so `None` selects the active phase.
pub struct RoadmapView<'a> {
    pub model: &'a RoadmapModel,
    pub cursor: Option<&'a CursorTarget>,
}

/// Scroll state of the list pane.
///
/// Not `Copy` since quick 260926-kes: `fold_marks` is a `Vec`.
#[derive(Debug, Default, Clone)]
pub struct RoadmapViewState {
    /// In/out: the first visible model row.
    pub offset: usize,
    /// Out: how many model rows the list pane shows (for PageUp/PageDown).
    pub list_rows: u16,
    /// Out: the rect of the scrolling model rows only — below the Start-now
    /// and header lines, above the Notes line; its first row draws model row
    /// `offset` (quick 260926-kes, D-01). Empty when nothing is listed.
    pub list_body: Rect,
    /// Out: one entry per VISIBLE band or shipped-summary row — its model row
    /// index and the rect of the two-cell `"{glyph} "` span it drew, measured
    /// from the spans `row_line` built rather than re-derived ([inferred I-3]).
    pub fold_marks: Vec<(usize, Rect)>,
}

/// Per-render lane styling context (quick 261006-ujx): the phase under the
/// cursor, whose need/unblock lanes draw BOLD while every other lane draws
/// DIM (`None` on a band or shipped-summary cursor: no weighting, I-7), and
/// whether lanes take palette hues at all (see [`lane_palette_enabled`]).
#[derive(Clone, Copy)]
struct LaneCtx {
    selected: Option<usize>,
    palette: bool,
}

/// False when `NO_COLOR` is set to a non-empty value (no-color.org). Scoped
/// deliberately to the Roadmap lane palette only: status glyph colours, the
/// `↑`/`↓` markers and every BOLD/DIM/REVERSED weight are unaffected, and no
/// other screen reads it (quick 261006-ujx, I-5).
fn lane_palette_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

/// The (list, detail) rects for `area`: side by side at
/// [`ROADMAP_SIDE_BY_SIDE_MIN_COLS`] and above, stacked below that. A
/// zero-size area yields two zero-size rects.
pub fn panes(area: Rect) -> (Rect, Rect) {
    panes_for(area, 0)
}

/// [`panes`] for a list that wants `list_rows` rows, borders included. Only
/// the stacked form reads it: the detail pane gives up rows it would take
/// (down to [`DETAIL_STACKED_MIN_ROWS`], never below [`panes`]' own share at
/// a tiny height) so the list is not scrolled while the terminal still has
/// the rows to show it whole — at 80×24 that is what keeps a milestone's band
/// row on screen with its last phase selected. `0` is exactly [`panes`].
pub fn panes_for(area: Rect, list_rows: u16) -> (Rect, Rect) {
    if area.is_empty() {
        let zero = Rect::new(area.x, area.y, 0, 0);
        return (zero, zero);
    }
    if area.width >= ROADMAP_SIDE_BY_SIDE_MIN_COLS {
        let detail = u16::try_from(u32::from(area.width) * 2 / 5)
            .unwrap_or(u16::MAX)
            .clamp(DETAIL_MIN_COLS, DETAIL_MAX_COLS);
        let list = area.width - detail;
        (
            Rect::new(area.x, area.y, list, area.height),
            Rect::new(area.x.saturating_add(list), area.y, detail, area.height),
        )
    } else {
        let share = DETAIL_STACKED_ROWS.min(area.height / 2);
        let detail = share
            .min(area.height.saturating_sub(list_rows))
            .max(share.min(DETAIL_STACKED_MIN_ROWS));
        let list = area.height - detail;
        (
            Rect::new(area.x, area.y, area.width, list),
            Rect::new(area.x, area.y.saturating_add(list), area.width, detail),
        )
    }
}

/// `word` or `words` for `n`.
fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

/// The first visible row that keeps `row` inside a window of `visible` rows:
/// `offset` itself when `row` is already inside it, else the nearest offset
/// that shows `row` (at the top when scrolling up, at the bottom when
/// scrolling down). A zero-row window counts as one row.
pub fn keep_visible(offset: usize, row: usize, visible: usize) -> usize {
    let visible = visible.max(1);
    if row < offset {
        row
    } else if row >= offset.saturating_add(visible) {
        row + 1 - visible
    } else {
        offset
    }
}

// ---------------------------------------------------------------------------
// Char-based text helpers
// ---------------------------------------------------------------------------

/// Width of already-escaped text, by `char`.
fn cells(text: &str) -> usize {
    text.chars().count()
}

/// The summed cell width of `spans`.
fn spans_cells(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| cells(&s.content)).sum()
}

/// `text` cut to `cols` chars, the cut marked with `…`. Char-wise, never
/// byte-wise (the `truncate_subject` rule of `screens::detail`).
fn truncate(text: &str, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }
    if cells(text) <= cols {
        return text.to_string();
    }
    let mut out: String = text.chars().take(cols - 1).collect();
    out.push(ELLIPSIS);
    out
}

/// `text` truncated to `cols` and left-aligned in exactly `cols` cells.
fn pad_right(text: &str, cols: usize) -> String {
    let mut out = truncate(text, cols);
    let fill = cols.saturating_sub(cells(&out));
    out.extend(std::iter::repeat_n(' ', fill));
    out
}

/// `text` truncated to `cols` and right-aligned in exactly `cols` cells.
fn pad_left(text: &str, cols: usize) -> String {
    let cut = truncate(text, cols);
    let mut out = " ".repeat(cols.saturating_sub(cells(&cut)));
    out.push_str(&cut);
    out
}

/// `spans` fitted into `cols` cells: the overflowing span is cut and marked
/// with `…`, everything after it dropped.
fn fit(spans: Vec<Span<'static>>, cols: usize) -> Line<'static> {
    let total: usize = spans.iter().map(|s| cells(&s.content)).sum();
    if total <= cols {
        return Line::from(spans);
    }
    if cols == 0 {
        return Line::default();
    }
    let mut budget = cols - 1;
    let mut out = Vec::with_capacity(spans.len());
    let mut last = Style::default();
    for span in spans {
        last = span.style;
        let width = cells(&span.content);
        if width <= budget {
            budget -= width;
            out.push(span);
            continue;
        }
        let cut: String = span.content.chars().take(budget).collect();
        out.push(Span::styled(cut, span.style));
        break;
    }
    out.push(Span::styled(ELLIPSIS.to_string(), last));
    Line::from(out)
}

/// A bordered pane with one cell of inner padding left and right (the
/// mockups' ` Start now:` / ` Checkout & payments`).
fn pane_block() -> Block<'static> {
    Block::bordered().padding(Padding::horizontal(1))
}

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// The status glyph's style (D-A04). Blocked is Red, not bold, since quick
/// 261006-ujx (I-3) — superseding D-A04's default style — so status reads by
/// colour as well as shape now that lanes carry palette hues.
fn glyph_style(status: PhaseStatus) -> Style {
    match status {
        PhaseStatus::Done => dim(),
        PhaseStatus::Active => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PhaseStatus::Ready => Style::default().fg(Color::Green),
        PhaseStatus::Blocked => Style::default().fg(Color::Red),
        // Cyan and never dim: finished work still waiting on a merge is a
        // pending action, not history (quick 260929-szq).
        PhaseStatus::Unmerged => Style::default().fg(Color::Cyan),
    }
}

/// A lane char's style (quick 261006-ujx, git-graph look): lane index `lane`
/// is the char's column (`char_index / 2`), never derived from its edge.
///
/// * The hue is `LANE_PALETTE[lane % len]` when `palette`, else no fg
///   (`NO_COLOR`).
/// * `on_chain` (an edge of the selected phase): the hue — kept even when
///   `owner_done` — plus BOLD.
/// * Else `owner_done` (the lane's upstream phase is Done): DarkGray, or DIM
///   without a palette.
/// * Else the hue.
/// * Then `dimmed && !on_chain` adds DIM.
fn lane_style(lane: usize, owner_done: bool, on_chain: bool, dimmed: bool, palette: bool) -> Style {
    let hue = palette.then(|| LANE_PALETTE[lane % LANE_PALETTE.len()]);
    let mut style = Style::default();
    if on_chain {
        if let Some(hue) = hue {
            style = style.fg(hue);
        }
        style = style.add_modifier(Modifier::BOLD);
    } else if owner_done {
        style = if palette {
            style.fg(Color::DarkGray)
        } else {
            style.add_modifier(Modifier::DIM)
        };
    } else if let Some(hue) = hue {
        style = style.fg(hue);
    }
    if dimmed && !on_chain {
        style = style.add_modifier(Modifier::DIM);
    }
    style
}

fn status_word(status: PhaseStatus) -> &'static str {
    match status {
        PhaseStatus::Done => "done",
        PhaseStatus::Active => "active",
        PhaseStatus::Ready => "ready",
        PhaseStatus::Blocked => "blocked",
        PhaseStatus::Unmerged => crate::agents::unmerged::LABEL_UNMERGED,
    }
}

fn lanes_of(row: &ListRow) -> &str {
    match row {
        ListRow::ShippedSummary { lanes, .. }
        | ListRow::Band { lanes, .. }
        | ListRow::Connector { lanes, .. }
        | ListRow::Phase { lanes, .. } => lanes,
    }
}

/// The lane cell as spans in `width` cells, one char at a time (quick
/// 261006-ujx): the char at `glyph_at` (the phase's own node) is drawn in
/// `glyph`, a blank plain, the [`LANE_OVERFLOW`] column DarkGray, and every
/// other char in `lane(i, link)`, where
/// `link` is `links.get(i)` — a short or empty `links` degrades to `None`,
/// never a panic (T-ujx-02). Adjacent chars of one style share a span.
fn lane_spans(
    lanes: &str,
    links: &[Option<LaneLink>],
    glyph_at: Option<usize>,
    glyph: Style,
    lane: &dyn Fn(usize, Option<LaneLink>) -> Style,
    width: usize,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(4);
    let mut run = String::new();
    let mut run_style = Style::default();
    let mut count = 0;
    for (i, c) in lanes.chars().enumerate() {
        count += 1;
        let style = if Some(i) == glyph_at {
            glyph
        } else if c == ' ' {
            Style::default()
        } else if LANE_OVERFLOW.starts_with(c) {
            dim()
        } else {
            lane(i, links.get(i).copied().flatten())
        };
        if style != run_style && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), run_style));
        }
        run_style = style;
        run.push(c);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, run_style));
    }
    let fill = width.saturating_sub(count);
    if fill > 0 {
        spans.push(Span::raw(" ".repeat(fill)));
    }
    spans
}

/// `lanes` with every lane past `cap` collapsed into one [`LANE_OVERFLOW`]
/// cell (T-24-14), and where the node glyph at `glyph_at` now sits. A node
/// whose own lane is past the cap takes the overflow cell itself, so its
/// status glyph stays visible.
fn cap_lanes(lanes: &str, glyph_at: Option<usize>, cap: usize) -> (String, Option<usize>) {
    let keep = cap.saturating_mul(2);
    let chars: Vec<char> = lanes.chars().collect();
    if chars.len() <= keep {
        return (lanes.to_string(), glyph_at);
    }
    let mut out: String = chars[..keep].iter().collect();
    match glyph_at {
        Some(g) if g >= keep => {
            if let Some(glyph) = chars.get(g) {
                out.push(*glyph);
            }
            (out, Some(keep))
        }
        _ => {
            if chars[keep..].iter().any(|c| *c != ' ') {
                out.push_str(LANE_OVERFLOW);
            }
            (out.trim_end().to_string(), glyph_at)
        }
    }
}

// ---------------------------------------------------------------------------
// Detail pane items
// ---------------------------------------------------------------------------

/// One piece of a detail pane, drawn top to bottom.
enum Item {
    /// One row.
    Line(Line<'static>),
    /// Text wrapped by `Paragraph::wrap`, with an optional hanging label.
    Wrapped {
        label: &'static str,
        text: String,
        style: Style,
    },
    /// The key hint: the first thing dropped when the pane is too short.
    Hint(Line<'static>),
}

impl Item {
    fn blank() -> Self {
        Item::Line(Line::default())
    }

    fn is_blank(&self) -> bool {
        matches!(self, Item::Line(line) if line.spans.iter().all(|s| s.content.trim().is_empty()))
    }

    /// Rows this item takes in `width` columns, at most `max`.
    fn height(&self, width: u16, max: u16) -> u16 {
        match self {
            Item::Line(_) | Item::Hint(_) => 1,
            Item::Wrapped { label, text, .. } => {
                let label_w = u16::try_from(cells(label)).unwrap_or(u16::MAX);
                wrapped_height(text, width.saturating_sub(label_w), max)
            }
        }
    }

    /// Draw into `area`; `clipped` says wrapped text lost rows to the pane's
    /// height, so its last drawn row ends in `…`.
    fn render(self, area: Rect, buf: &mut Buffer, clipped: bool) {
        match self {
            Item::Line(line) | Item::Hint(line) => Paragraph::new(line).render(area, buf),
            Item::Wrapped { label, text, style } => {
                let label_w = u16::try_from(cells(label)).unwrap_or(u16::MAX);
                let [head, body] =
                    Layout::horizontal([Constraint::Length(label_w), Constraint::Min(0)])
                        .areas(area);
                if label_w > 0 {
                    Paragraph::new(Span::styled(label, bold())).render(head, buf);
                }
                Paragraph::new(Span::styled(text, style))
                    .wrap(Wrap { trim: true })
                    .render(body, buf);
                if clipped && !body.is_empty() {
                    mark_cut(body, buf, style);
                }
            }
        }
    }
}

/// Rows `text` takes when `Paragraph::wrap` lays it out in `width` columns,
/// measured by rendering into a scratch buffer of at most `max` rows.
fn wrapped_height(text: &str, width: u16, max: u16) -> u16 {
    if width == 0 || max == 0 {
        return 0;
    }
    let scratch = Rect::new(0, 0, width, max);
    let mut buf = Buffer::empty(scratch);
    Paragraph::new(text.to_string())
        .wrap(Wrap { trim: true })
        .render(scratch, &mut buf);
    (0..max)
        .rev()
        .find(|&y| (0..width).any(|x| buf.cell((x, y)).is_some_and(|c| c.symbol() != " ")))
        .map_or(1, |y| y + 1)
}

/// End the last row of wrapped text cut short in `body` with `…`: right
/// after its last drawn character, or over it when the row is full.
fn mark_cut(body: Rect, buf: &mut Buffer, style: Style) {
    let y = body.bottom() - 1;
    let last = (body.x..body.right())
        .rev()
        .find(|&x| buf.cell((x, y)).is_some_and(|c| c.symbol() != " "));
    let x = match last {
        Some(x) if x + 1 < body.right() => x + 1,
        Some(x) => x,
        None => body.x,
    };
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(&ELLIPSIS.to_string()).set_style(style);
    }
}

/// Draw `items` top to bottom into `area`. When they do not fit, the hint is
/// dropped first (with the blank rows before it), then wrapped text (the goal)
/// is shortened — to one row at the least, its cut marked `…` — so the Needs,
/// Unblocks and Parallel lines keep their rows, and only then are rows cut
/// from the bottom.
fn draw_items(mut items: Vec<Item>, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }
    let heights = |items: &[Item]| -> Vec<u16> {
        items
            .iter()
            .map(|item| item.height(area.width, area.height))
            .collect()
    };
    let sum = |h: &[u16]| -> u32 { h.iter().map(|&r| u32::from(r)).sum() };
    let mut full = heights(&items);
    if sum(&full) > u32::from(area.height) {
        items.retain(|item| !matches!(item, Item::Hint(_)));
        while items.last().is_some_and(Item::is_blank) {
            items.pop();
        }
        full = heights(&items);
    }
    let mut rows = full.clone();
    let mut over = sum(&rows).saturating_sub(u32::from(area.height));
    for (item, r) in items.iter().zip(rows.iter_mut()) {
        if over == 0 {
            break;
        }
        if matches!(item, Item::Wrapped { .. }) {
            let cut = u16::try_from(over.min(u32::from(r.saturating_sub(1)))).unwrap_or(0);
            *r -= cut;
            over -= u32::from(cut);
        }
    }
    let bottom = area.bottom();
    let mut y = area.y;
    for ((item, want), whole) in items.into_iter().zip(rows).zip(full) {
        if y >= bottom {
            break;
        }
        let h = want.min(bottom - y);
        if h == 0 {
            continue;
        }
        item.render(Rect::new(area.x, y, area.width, h), buf, h < whole);
        y += h;
    }
}

/// One phase entry of a Needs / Unblocks / Parallel section.
struct Entry {
    node: usize,
    tail: String,
    /// A continuation row under the entry (`(implied via 21)`).
    note: Option<String>,
    dimmed: bool,
}

/// Prefix each row of a section with its label (first row) or the label's
/// indent (later rows), fitted to `w`.
fn section(label: &str, rows: Vec<Vec<Span<'static>>>, w: usize) -> Vec<Item> {
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            let head = if i == 0 {
                Span::styled(pad_right(label, LABEL_CELLS), bold())
            } else {
                Span::raw(" ".repeat(LABEL_CELLS))
            };
            let mut spans = vec![head];
            spans.extend(row);
            Item::Line(fit(spans, w))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The widget
// ---------------------------------------------------------------------------

/// The list pane's column widths.
struct Cols {
    lane: usize,
    id: usize,
    name: usize,
    /// Lanes drawn before the overflow column.
    cap: usize,
}

impl RoadmapView<'_> {
    fn phase(&self, node: usize) -> Option<&PhaseFacts> {
        self.model.phases.get(node)
    }

    /// The style of lane char `i` drawing edge `link` (the [`lane_spans`]
    /// callback): its column's hue, DarkGray when the lane's owner is Done
    /// (I-6); BOLD on an edge of `ctx.selected`, DIM on every other lane
    /// while a phase is selected.
    fn lane_cell_style(&self, ctx: LaneCtx, i: usize, link: Option<LaneLink>) -> Style {
        let owner_done = link
            .and_then(|k| self.phase(k.owner))
            .is_some_and(|p| p.status == PhaseStatus::Done);
        let on_chain = self.on_chain(ctx.selected, link);
        let dimmed = ctx.selected.is_some() && !on_chain;
        lane_style(i / 2, owner_done, on_chain, dimmed, ctx.palette)
    }

    /// Whether `link` is a reduced edge of the selected phase (I-8): out of
    /// it to a phase it unblocks, or into it from a phase it needs.
    fn on_chain(&self, selected: Option<usize>, link: Option<LaneLink>) -> bool {
        let (Some(s), Some(k)) = (selected, link) else {
            return false;
        };
        let Some(p) = self.phase(s) else {
            return false;
        };
        (k.owner == s && p.unblocks.contains(&k.target))
            || (k.target == s && p.needs.contains(&k.owner))
    }

    fn columns(&self, w: usize, cap: usize) -> Cols {
        let lane = self
            .model
            .rows
            .iter()
            .map(|row| {
                let glyph_at = match row {
                    ListRow::Phase { lane, .. } => lane.checked_mul(2),
                    _ => None,
                };
                cells(&cap_lanes(lanes_of(row), glyph_at, cap).0)
            })
            .max()
            .unwrap_or(0)
            .max(MIN_LANE_CELLS);
        let id = self
            .model
            .phases
            .iter()
            .map(|p| cells(&p.id))
            .max()
            .unwrap_or(1)
            .clamp(1, MAX_ID_CELLS);
        let fixed = lane + MARK_CELLS + id + GAP.len() + PLANS_CELLS + GAP.len() + WAVE_CELLS;
        Cols {
            lane,
            id,
            name: w.saturating_sub(fixed),
            cap,
        }
    }

    // --- list pane --------------------------------------------------------

    fn render_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut RoadmapViewState,
        cursor: Option<&CursorTarget>,
        cap: usize,
    ) {
        let block = pane_block()
            .title_top(Line::from(" Roadmap "))
            .title_top(Line::from(" v list ").right_aligned());
        let inner = block.inner(area);
        block.render(area, buf);
        state.list_body = Rect::default();
        state.fold_marks.clear();
        if inner.is_empty() {
            state.list_rows = 0;
            return;
        }
        let w = usize::from(inner.width);
        let cols = self.columns(w, cap);
        let selected_phase = match cursor {
            Some(CursorTarget::Phase(key)) => self.model.phase_index(key),
            _ => None,
        };
        let selected_row = cursor.and_then(|c| self.model.row_of(c));
        let ctx = LaneCtx {
            selected: selected_phase,
            palette: lane_palette_enabled(),
        };

        // The Start-now and header lines never scroll; a Notes line, when
        // there is one, takes the pane's last row.
        let note = self.model.notes.first().filter(|_| inner.height > 2);
        let body = usize::from(inner.height).saturating_sub(2 + usize::from(note.is_some()));
        state.list_rows = u16::try_from(body).unwrap_or(u16::MAX);
        // The `clamp_scroll` idiom: never past the tail, then the cursor row.
        let mut offset = state.offset.min(self.model.rows.len().saturating_sub(body));
        if let Some(row) = selected_row {
            offset = keep_visible(offset, row, body);
        }
        state.offset = offset;

        if let Some(note) = note {
            let y = inner.bottom() - 1;
            Paragraph::new(fit(
                vec![
                    Span::styled("Notes  ", bold()),
                    Span::styled(note.clone(), dim()),
                ],
                w,
            ))
            .render(Rect::new(inner.x, y, inner.width, 1), buf);
        }
        let list_area = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner.height - u16::from(note.is_some()),
        );

        // The mouse's rects (quick 260926-kes): the body rows the loop below
        // draws, and each band row's fold glyph at the column its spans put
        // it — so a click maps onto what is drawn, never onto a constant.
        let body_rows = u16::try_from(body).unwrap_or(u16::MAX);
        let body_y = inner.y.saturating_add(2).min(inner.bottom());
        state.list_body = Rect::new(inner.x, body_y, inner.width, body_rows);

        let mut lines = vec![self.start_now_line(w), self.header_line(&cols, w)];
        for (i, row) in self.model.rows.iter().enumerate().skip(offset).take(body) {
            let selected = selected_row == Some(i);
            let (line, glyph_col) = self.row_line(row, selected, ctx, &cols, w);
            if let Some(col) = glyph_col.and_then(|c| u16::try_from(c).ok()) {
                let y = body_y.saturating_add(u16::try_from(i - offset).unwrap_or(u16::MAX));
                let x = inner.x.saturating_add(col);
                if x < inner.right() && y < inner.bottom() {
                    let width = 2.min(inner.right() - x);
                    state.fold_marks.push((i, Rect::new(x, y, width, 1)));
                }
            }
            lines.push(line);
        }
        Paragraph::new(lines).render(list_area, buf);
    }

    /// `Start now: ◉10 Slot picker UI (active) ║ ○11 Calendar sync` (D-A06).
    fn start_now_line(&self, w: usize) -> Line<'static> {
        let mut spans = vec![Span::styled("Start now: ", bold())];
        let now: Vec<&PhaseFacts> = self
            .model
            .start_now
            .iter()
            .filter_map(|&u| self.phase(u))
            .collect();
        if now.is_empty() {
            spans.push(Span::styled("nothing ready", dim()));
        }
        for (i, p) in now.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(format!(" {PARALLEL_SEP} "), dim()));
            }
            spans.push(Span::styled(p.status.glyph(), glyph_style(p.status)));
            let mut text = format!("{} {}", p.id, p.name);
            if p.status == PhaseStatus::Active {
                text.push_str(" (active)");
            }
            spans.push(Span::raw(text));
        }
        fit(spans, w)
    }

    /// `  lanes   #  Phase … plans  wave`.
    fn header_line(&self, cols: &Cols, w: usize) -> Line<'static> {
        let text = format!(
            "{}{}{}{GAP}{}{}{GAP}wave",
            pad_right(" lanes", cols.lane),
            " ".repeat(MARK_CELLS),
            pad_left("#", cols.id),
            pad_right("Phase", cols.name),
            pad_left("plans", PLANS_CELLS),
        );
        fit(vec![Span::styled(text, dim())], w)
    }

    /// One list row, and — on a band or shipped-summary row — the cell
    /// column its fold glyph was drawn at (quick 260926-kes, [inferred I-3]):
    /// the summed widths of the spans pushed before the glyph span, confirmed
    /// against the fitted line, so a glyph `fit` cut away records nothing.
    fn row_line(
        &self,
        row: &ListRow,
        selected: bool,
        ctx: LaneCtx,
        cols: &Cols,
        w: usize,
    ) -> (Line<'static>, Option<usize>) {
        let plain = Style::default();
        let lane = |i: usize, link: Option<LaneLink>| self.lane_cell_style(ctx, i, link);
        let mut glyph_col = None;
        let spans = match row {
            ListRow::Phase {
                node,
                lane,
                lanes,
                links,
            } => self.phase_spans(*node, *lane, (lanes, links), selected, ctx, cols),
            // `{lanes}{▾|▸} {label}  {done}/{total} ━━━…` (D-A07).
            ListRow::Band {
                label,
                done,
                total,
                folded,
                lanes,
                links,
                ..
            } => {
                let (lanes, _) = cap_lanes(lanes, None, cols.cap);
                let mut spans = lane_spans(&lanes, links, None, plain, &lane, cols.lane);
                let glyph = if *folded { BAND_FOLDED } else { BAND_OPEN };
                let tail = format!("{GAP}{done}/{total} ");
                let used = cols.lane + 2 + cells(&tail);
                let label = truncate(label, w.saturating_sub(used));
                let fill = w.saturating_sub(used + cells(&label));
                glyph_col = Some(spans_cells(&spans));
                spans.push(Span::styled(format!("{glyph} "), bold()));
                spans.push(Span::styled(label, bold()));
                spans.push(Span::styled(tail, dim()));
                spans.push(Span::styled(BAND_FILL.repeat(fill), dim()));
                spans
            }
            // `{▸|▾} v1.0 … v1.4   5 milestones · 17 phases shipped`.
            ListRow::ShippedSummary {
                text,
                milestones,
                phases,
                folded,
                lanes,
                links,
            } => {
                let (lanes, _) = cap_lanes(lanes, None, cols.cap);
                let glyph = if *folded { BAND_FOLDED } else { BAND_OPEN };
                let count = format!(
                    "   {milestones} {}{DOT}{phases} {} shipped",
                    plural(*milestones, "milestone"),
                    plural(usize::try_from(*phases).unwrap_or(usize::MAX), "phase"),
                );
                // Unpadded, as before: `width` is the lanes' own width.
                let mut spans = lane_spans(&lanes, links, None, plain, &lane, cells(&lanes));
                glyph_col = Some(spans_cells(&spans));
                spans.push(Span::styled(format!("{glyph} "), bold()));
                spans.push(Span::styled(text.clone(), bold()));
                spans.push(Span::styled(count, dim()));
                let used: usize = spans.iter().map(|s| cells(&s.content)).sum();
                spans.push(Span::raw(" ".repeat(w.saturating_sub(used))));
                spans
            }
            ListRow::Connector { lanes, links } => {
                let (lanes, _) = cap_lanes(lanes, None, cols.cap);
                lane_spans(&lanes, links, None, plain, &lane, cols.lane)
            }
        };
        let spans = if selected {
            spans
                .into_iter()
                .map(|s| {
                    let style = s.style.add_modifier(Modifier::REVERSED);
                    s.style(style)
                })
                .collect()
        } else {
            spans
        };
        let line = fit(spans, w);
        let glyph_col = glyph_col.filter(|&col| {
            let mut start = 0;
            line.spans.iter().any(|s| {
                let hit = start == col
                    && (s.content.starts_with(BAND_OPEN) || s.content.starts_with(BAND_FOLDED));
                start += cells(&s.content);
                hit
            })
        });
        (line, glyph_col)
    }

    /// The marker cell of `node` relative to the selected phase (D-A05).
    fn mark_for(&self, node: usize, selected: bool, sel: Option<usize>) -> (&'static str, Style) {
        if selected {
            return (MARK_SELECTED, bold());
        }
        let Some(s) = sel.and_then(|i| self.phase(i)) else {
            return (" ", Style::default());
        };
        if s.needs.contains(&node) {
            (MARK_DEP, Style::default().fg(Color::Cyan))
        } else if s.unblocks.contains(&node) {
            (MARK_UNBLOCKS, Style::default().fg(Color::Magenta))
        } else if s.implied.iter().any(|&(dep, _)| dep == node) {
            (MARK_IMPLIED, dim())
        } else {
            (" ", Style::default())
        }
    }

    /// `{lanes}{marker}{id}  {name}{plans}  {W<wave>}` (D-A01). The
    /// third argument is the row's `(lanes, links)`.
    fn phase_spans(
        &self,
        node: usize,
        lane: usize,
        (lanes, links): (&str, &[Option<LaneLink>]),
        selected: bool,
        ctx: LaneCtx,
        cols: &Cols,
    ) -> Vec<Span<'static>> {
        let Some(p) = self.phase(node) else {
            return Vec::new();
        };
        let base = if p.status == PhaseStatus::Done {
            dim()
        } else {
            Style::default()
        };
        // `base` dims a Done row's text only, never its lane cell: each lane
        // char takes its own edge's style (quick 261006-ujx, S-08).
        let (lanes, glyph_at) = cap_lanes(lanes, lane.checked_mul(2), cols.cap);
        let style = |i: usize, link: Option<LaneLink>| self.lane_cell_style(ctx, i, link);
        let mut spans = lane_spans(
            &lanes,
            links,
            glyph_at,
            glyph_style(p.status),
            &style,
            cols.lane,
        );
        let (mark, mark_style) = self.mark_for(node, selected, ctx.selected);
        spans.push(Span::styled(format!(" {mark} "), mark_style));
        let plans = p
            .plans
            .map_or_else(|| NO_PLANS.to_string(), |(d, t)| format!("{d}/{t}"));
        spans.push(Span::styled(
            format!(
                "{}{GAP}{}{}{GAP}{}",
                pad_left(&p.id, cols.id),
                pad_right(&p.name, cols.name),
                pad_left(&plans, PLANS_CELLS),
                pad_right(&format!("W{}", p.wave), WAVE_CELLS),
            ),
            base,
        ));
        spans
    }

    // --- detail pane ------------------------------------------------------

    /// The detail pane for whatever the cursor rests on: a phase (side-by-side
    /// or stacked form), a band, or the shipped summary (D-A02, D-A07).
    fn render_detail(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cursor: Option<&CursorTarget>,
        stacked: bool,
    ) {
        if area.is_empty() {
            return;
        }
        let w = usize::from(pane_block().inner(area).width);
        let (title, items) = match cursor {
            Some(CursorTarget::Phase(key)) => {
                match self.model.phase_index(key).and_then(|u| self.phase(u)) {
                    Some(p) if stacked => (format!(" Phase {} ", p.id), self.stacked_items(p, w)),
                    Some(p) => (format!(" Phase {} ", p.id), self.phase_items(p, w)),
                    None => (String::new(), Vec::new()),
                }
            }
            Some(CursorTarget::Band(BandKey::Shipped)) => {
                (" Shipped ".to_string(), self.shipped_items(w, stacked))
            }
            Some(CursorTarget::Band(key)) => {
                match self.model.bands.iter().find(|b| b.key == *key) {
                    Some(b) => (format!(" {} ", b.short), Self::band_items(b, w, stacked)),
                    None => (String::new(), Vec::new()),
                }
            }
            None => (String::new(), Vec::new()),
        };
        let block = pane_block().title_top(Line::from(title));
        let inner = block.inner(area);
        block.render(area, buf);
        draw_items(items, inner, buf);
    }

    /// A band under the cursor: its label, progress and the fold key.
    fn band_items(b: &BandFacts, w: usize, stacked: bool) -> Vec<Item> {
        let mut progress = format!("{}/{} phases done", b.done, b.total);
        if b.shipped {
            progress.push_str(DOT);
            progress.push_str("shipped");
        }
        vec![
            Item::Line(fit(vec![Span::styled(b.label.clone(), bold())], w)),
            Item::Line(fit(vec![Span::raw(progress)], w)),
            Item::blank(),
            Item::Hint(fit(
                vec![Span::styled(
                    format!("{HINT_BAND}{}", if stacked { HINT_MOVE } else { "" }),
                    dim(),
                )],
                w,
            )),
        ]
    }

    /// The shipped summary under the cursor: every shipped milestone with the
    /// phase count it declares.
    fn shipped_items(&self, w: usize, stacked: bool) -> Vec<Item> {
        let mut items = Vec::new();
        if let Some(ListRow::ShippedSummary {
            milestones, phases, ..
        }) = self
            .model
            .rows
            .iter()
            .find(|r| matches!(r, ListRow::ShippedSummary { .. }))
        {
            items.push(Item::Line(fit(
                vec![Span::styled(
                    format!(
                        "{milestones} {}{DOT}{phases} {} shipped",
                        plural(*milestones, "milestone"),
                        plural(usize::try_from(*phases).unwrap_or(usize::MAX), "phase"),
                    ),
                    bold(),
                )],
                w,
            )));
            items.push(Item::blank());
        }
        for b in self.model.bands.iter().filter(|b| b.shipped) {
            let declared = usize::try_from(b.declared_phases)
                .unwrap_or(usize::MAX)
                .max(b.total);
            items.push(Item::Line(fit(
                vec![
                    Span::raw(b.label.clone()),
                    Span::styled(
                        format!("{GAP}{declared} {}", plural(declared, "phase")),
                        dim(),
                    ),
                ],
                w,
            )));
        }
        items.push(Item::blank());
        items.push(Item::Hint(fit(
            vec![Span::styled(
                format!("{HINT_SHIPPED}{}", if stacked { HINT_MOVE } else { "" }),
                dim(),
            )],
            w,
        )));
        items
    }

    /// `{glyph} {word} · {d}/{t} plans  {badge} · planned (not a GSD phase)`.
    /// A planned phase says so instead of its plan count: it has no GSD plans.
    fn status_spans(p: &PhaseFacts) -> Vec<Span<'static>> {
        let mut spans = vec![
            Span::styled(p.status.glyph(), glyph_style(p.status)),
            Span::raw(format!(" {}", status_word(p.status))),
        ];
        match p.plans {
            Some((d, t)) => spans.push(Span::raw(format!("{DOT}{d}/{t} plans"))),
            None if !p.planned => spans.push(Span::raw(format!("{DOT}plans TBD"))),
            None => {}
        }
        // `◐k`: plans of this phase finished on a worktree, not yet merged
        // (quick 260929-szq, I-11); detail status line only.
        if p.unmerged > 0 {
            spans.push(Span::styled(
                format!(" {GLYPH_UNMERGED}{}", p.unmerged),
                Style::default().fg(Color::Cyan),
            ));
        }
        if let Some(badge) = &p.badge {
            spans.push(Span::styled(
                format!("{GAP}{badge}"),
                Style::default().fg(Color::Cyan),
            ));
        }
        if p.planned {
            spans.push(Span::styled(
                format!("{DOT}planned (not a GSD phase)"),
                dim(),
            ));
        }
        spans
    }

    /// The short id of the band `node` sits in.
    fn band_short(&self, band: Option<usize>) -> Option<&str> {
        band.and_then(|b| self.model.bands.get(b))
            .map(|b| b.short.as_str())
    }

    /// Aligned entry rows: `{glyph} {id} {name…} {tail}`.
    fn entry_rows(&self, entries: &[Entry], avail: usize) -> Vec<Vec<Span<'static>>> {
        let facts: Vec<(&Entry, &PhaseFacts)> = entries
            .iter()
            .filter_map(|e| self.phase(e.node).map(|p| (e, p)))
            .collect();
        let id_w = facts
            .iter()
            .map(|(_, p)| cells(&p.id))
            .max()
            .unwrap_or(0)
            .min(MAX_ID_CELLS);
        let tail_w = facts.iter().map(|(e, _)| cells(&e.tail)).max().unwrap_or(0);
        let name_max = facts.iter().map(|(_, p)| cells(&p.name)).max().unwrap_or(0);
        let fixed = 2 + id_w + 1 + if tail_w > 0 { 1 + tail_w } else { 0 };
        let name_w = name_max.min(avail.saturating_sub(fixed));
        let mut rows = Vec::with_capacity(facts.len());
        for (e, p) in facts {
            let text_style = if e.dimmed { dim() } else { Style::default() };
            let mut text = format!("{} {}", pad_right(&p.id, id_w), pad_right(&p.name, name_w));
            if tail_w > 0 {
                text.push(' ');
                text.push_str(&e.tail);
            }
            rows.push(vec![
                Span::styled(p.status.glyph(), glyph_style(p.status)),
                Span::styled(format!(" {}", text.trim_end()), text_style),
            ]);
            if let Some(note) = &e.note {
                rows.push(vec![Span::styled(format!("  {note}"), dim())]);
            }
        }
        rows
    }

    fn plans_tail(&self, node: usize) -> String {
        self.phase(node)
            .and_then(|p| p.plans)
            .map_or_else(String::new, |(d, t)| format!("{d}/{t}"))
    }

    /// Needs: reduced deps, implied deps (`(implied via N)`), external deps,
    /// or `nothing declared`.
    fn needs_rows(&self, p: &PhaseFacts, avail: usize) -> Vec<Vec<Span<'static>>> {
        if p.no_deps {
            return vec![vec![Span::styled("nothing declared", dim())]];
        }
        let mut entries: Vec<Entry> = p
            .needs
            .iter()
            .map(|&u| Entry {
                node: u,
                tail: self.plans_tail(u),
                note: None,
                dimmed: false,
            })
            .collect();
        entries.extend(p.implied.iter().map(|&(dep, via)| Entry {
            node: dep,
            tail: String::new(),
            note: self.phase(via).map(|v| format!("(implied via {})", v.id)),
            dimmed: true,
        }));
        let mut rows = self.entry_rows(&entries, avail);
        rows.extend(
            p.external
                .iter()
                .map(|id| vec![Span::styled(format!("{id} (outside this roadmap)"), dim())]),
        );
        rows
    }

    /// Unblocks, with the other band's short id when it differs.
    fn unblocks_rows(&self, p: &PhaseFacts, avail: usize) -> Vec<Vec<Span<'static>>> {
        if p.unblocks.is_empty() {
            let text = match self.band_short(p.band) {
                Some(short) if p.last_in_band => format!("nothing (last in {short})"),
                _ => "nothing".to_string(),
            };
            return vec![vec![Span::styled(text, dim())]];
        }
        let entries: Vec<Entry> = p
            .unblocks
            .iter()
            .map(|&u| {
                let other = self.phase(u).and_then(|q| q.band);
                let tail = match other {
                    Some(_) if other != p.band => self.band_short(other).unwrap_or("").to_string(),
                    _ => String::new(),
                };
                Entry {
                    node: u,
                    tail,
                    note: None,
                    dimmed: false,
                }
            })
            .collect();
        self.entry_rows(&entries, avail)
    }

    /// The no-deps explanation (D-A14, Mockup C wording).
    fn no_deps_note(p: &PhaseFacts) -> Option<&'static str> {
        match (p.no_deps, p.no_edges) {
            (true, true) => Some("(no edge either way; can run any time)"),
            (true, false) => Some("(no deps; can run any time)"),
            _ => None,
        }
    }

    /// Parallel: the other phases of the same wave (D-A02).
    fn parallel_rows(&self, p: &PhaseFacts, avail: usize) -> Vec<Vec<Span<'static>>> {
        if p.parallel.is_empty() {
            vec![vec![Span::styled(
                format!("none in wave {}", p.wave),
                dim(),
            )]]
        } else {
            let entries: Vec<Entry> = p
                .parallel
                .iter()
                .map(|&u| Entry {
                    node: u,
                    tail: match self.phase(u) {
                        Some(q) if q.status == PhaseStatus::Done => "done".to_string(),
                        Some(q) if q.status == PhaseStatus::Unmerged => {
                            crate::agents::unmerged::LABEL_UNMERGED.to_string()
                        }
                        _ => String::new(),
                    },
                    note: None,
                    dimmed: false,
                })
                .collect();
            self.entry_rows(&entries, avail)
        }
    }

    /// The side-by-side phase detail pane (Mockups A and B).
    fn phase_items(&self, p: &PhaseFacts, w: usize) -> Vec<Item> {
        let mut where_line = String::new();
        if let Some(band) = p.band.and_then(|b| self.model.bands.get(b)) {
            where_line.push_str(&band.label);
            where_line.push_str(DOT);
        }
        where_line.push_str(&format!("wave {} of {}", p.wave, self.model.max_wave));

        let mut items = vec![
            Item::Line(fit(vec![Span::styled(p.name.clone(), bold())], w)),
            Item::Line(fit(vec![Span::raw(where_line)], w)),
            Item::Line(fit(Self::status_spans(p), w)),
            Item::blank(),
            Item::Line(fit(vec![Span::styled("Goal", bold())], w)),
            match &p.goal {
                Some(goal) => Item::Wrapped {
                    label: "",
                    text: goal.clone(),
                    style: Style::default(),
                },
                None => Item::Line(fit(vec![Span::styled("no goal in ROADMAP.md", dim())], w)),
            },
            Item::blank(),
        ];
        let avail = w.saturating_sub(LABEL_CELLS);
        items.extend(section("Needs", self.needs_rows(p, avail), w));
        items.extend(section("Unblocks", self.unblocks_rows(p, avail), w));
        items.extend(section("Parallel", self.parallel_rows(p, avail), w));
        if let Some(note) = Self::no_deps_note(p) {
            // Under the Parallel label when it fits there whole; otherwise
            // from the pane's left edge, so the phrase is not broken
            // mid-sentence at the 40-56 column pane widths.
            let indented = cells(note) + LABEL_CELLS <= w;
            items.push(Item::Wrapped {
                label: if indented { INDENT } else { "" },
                text: note.to_string(),
                style: dim(),
            });
        }
        items.push(Item::blank());
        items.push(Item::Hint(fit(vec![Span::styled(HINT_WIDE, dim())], w)));
        items
    }

    /// `{glyph} {id}{tail}` for the compact (stacked) form.
    fn compact_entry(&self, node: usize, tail: &str) -> Vec<Span<'static>> {
        let Some(q) = self.phase(node) else {
            return Vec::new();
        };
        vec![
            Span::styled(q.status.glyph(), glyph_style(q.status)),
            Span::raw(format!(" {}{tail}", q.id)),
        ]
    }

    /// Entries two cells apart.
    fn join_compact(groups: Vec<Vec<Span<'static>>>) -> Vec<Span<'static>> {
        let mut out = Vec::new();
        for (i, group) in groups.into_iter().filter(|g| !g.is_empty()).enumerate() {
            if i > 0 {
                out.push(Span::raw(GAP));
            }
            out.extend(group);
        }
        out
    }

    fn label(text: &str) -> Span<'static> {
        Span::styled(pad_right(text, LABEL_CELLS), bold())
    }

    /// The stacked phase detail pane (Mockup C): one head line, the goal with
    /// a hanging indent, Needs and Unblocks (one line when both fit),
    /// Parallel, and the hint.
    fn stacked_items(&self, p: &PhaseFacts, w: usize) -> Vec<Item> {
        let mut place = String::from("   ");
        if let Some(short) = self.band_short(p.band) {
            place.push_str(short);
            place.push_str(DOT);
        }
        place.push_str(&format!("wave {}{DOT}", p.wave));
        let mut head = vec![Span::styled(p.name.clone(), bold()), Span::raw(place)];
        head.extend(Self::status_spans(p));

        let goal = match &p.goal {
            Some(goal) => Item::Wrapped {
                label: "Goal  ",
                text: goal.clone(),
                style: Style::default(),
            },
            None => Item::Wrapped {
                label: "Goal  ",
                text: "no goal in ROADMAP.md".to_string(),
                style: dim(),
            },
        };

        let needs = if p.no_deps {
            vec![Span::styled("nothing declared", dim())]
        } else {
            let mut groups: Vec<Vec<Span<'static>>> =
                p.needs.iter().map(|&u| self.compact_entry(u, "")).collect();
            groups.extend(p.implied.iter().map(|&(dep, via)| {
                let via = self.phase(via).map_or(String::new(), |v| v.id.clone());
                self.compact_entry(dep, &format!(" (implied via {via})"))
            }));
            groups.extend(
                p.external
                    .iter()
                    .map(|id| vec![Span::styled(format!("{id} (outside this roadmap)"), dim())]),
            );
            Self::join_compact(groups)
        };
        let unblocks = if p.unblocks.is_empty() {
            let text = match self.band_short(p.band) {
                Some(short) if p.last_in_band => format!("nothing (last in {short})"),
                _ => "nothing".to_string(),
            };
            vec![Span::styled(text, dim())]
        } else {
            Self::join_compact(
                p.unblocks
                    .iter()
                    .map(|&u| {
                        let other = self.phase(u).and_then(|q| q.band);
                        let tail = match other {
                            Some(_) if other != p.band => {
                                format!(" {}", self.band_short(other).unwrap_or(""))
                            }
                            _ => String::new(),
                        };
                        self.compact_entry(u, &tail)
                    })
                    .collect(),
            )
        };
        let width_of =
            |spans: &[Span<'static>]| spans.iter().map(|s| cells(&s.content)).sum::<usize>();
        let one_line = 2 * LABEL_CELLS + 3 + width_of(&needs) + width_of(&unblocks) <= w;
        let mut deps_lines = Vec::new();
        if one_line {
            let mut line = vec![Self::label("Needs")];
            line.extend(needs);
            line.push(Span::raw("   "));
            line.push(Self::label("Unblocks"));
            line.extend(unblocks);
            deps_lines.push(Item::Line(fit(line, w)));
        } else {
            let mut line = vec![Self::label("Needs")];
            line.extend(needs);
            deps_lines.push(Item::Line(fit(line, w)));
            let mut line = vec![Self::label("Unblocks")];
            line.extend(unblocks);
            deps_lines.push(Item::Line(fit(line, w)));
        }

        let mut parallel = vec![Self::label("Parallel")];
        if p.parallel.is_empty() {
            parallel.push(Span::styled(format!("none in wave {}", p.wave), dim()));
        } else {
            parallel.extend(Self::join_compact(
                p.parallel
                    .iter()
                    .map(|&u| {
                        let done = self.phase(u).is_some_and(|q| q.status == PhaseStatus::Done);
                        self.compact_entry(u, if done { " done" } else { "" })
                    })
                    .collect(),
            ));
        }
        if let Some(note) = Self::no_deps_note(p) {
            parallel.push(Span::styled(format!("   {note}"), dim()));
        }

        let mut items = vec![Item::Line(fit(head, w)), goal];
        items.extend(deps_lines);
        items.push(Item::Line(fit(parallel, w)));
        items.push(Item::Hint(fit(vec![Span::styled(HINT_STACKED, dim())], w)));
        items
    }
}

impl StatefulWidget for RoadmapView<'_> {
    type State = RoadmapViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.intersection(buf.area);
        // Nothing is listed on the two early returns: no stale mouse rects.
        state.list_body = Rect::default();
        state.fold_marks.clear();
        if area.is_empty() {
            state.list_rows = 0;
            return;
        }
        if self.model.rows.is_empty() {
            state.offset = 0;
            state.list_rows = 0;
            let block = pane_block().title_top(Line::from(" Roadmap "));
            let inner = block.inner(area);
            block.render(area, buf);
            Paragraph::new(Span::styled(EMPTY, dim())).render(inner, buf);
            return;
        }
        let cursor = self.model.resolve_cursor(self.cursor);
        let stacked = area.width < ROADMAP_SIDE_BY_SIDE_MIN_COLS;
        let cap = if stacked {
            LANE_CAP_NARROW
        } else {
            LANE_CAP_WIDE
        };
        // Rows the list wants whole: its model rows, the Start-now and
        // header lines, the Notes line and the two borders.
        let want = self.model.rows.len() + 4 + usize::from(!self.model.notes.is_empty());
        let (list, detail) = panes_for(area, u16::try_from(want).unwrap_or(u16::MAX));
        self.render_list(list, buf, state, cursor.as_ref(), cap);
        self.render_detail(detail, buf, cursor.as_ref(), stacked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::PhaseMarker;
    use crate::text::Untrusted;
    use crate::ui::roadmap_graph::{
        self, layout_list, BandInput, ListInput, ListNode, BAND_FILL, BAND_FOLDED, BAND_OPEN,
    };
    use std::collections::HashSet;

    const D: PhaseMarker = PhaseMarker::Done;
    const C: PhaseMarker = PhaseMarker::Current;
    const F: PhaseMarker = PhaseMarker::Future;

    /// `(id, name, deps, marker, band)`.
    type Row<'a> = (&'a str, &'a str, &'a [&'a str], PhaseMarker, Option<usize>);
    /// A list spec.
    type Spec<'a> = &'a [Row<'a>];
    /// `(label, shipped, declared phases)`.
    type Bands<'a> = &'a [(&'a str, bool, u32)];

    /// Per-phase extras, keyed by id.
    #[derive(Default)]
    struct Extras<'a> {
        plans: &'a [(&'a str, (u32, u32))],
        goals: &'a [(&'a str, &'a str)],
        planned: &'a [&'a str],
        badges: &'a [(&'a str, &'a str)],
        toggles: HashSet<BandKey>,
    }

    fn build(spec: Spec<'_>, bands: Bands<'_>, extras: &Extras<'_>) -> RoadmapModel {
        let deps: Vec<Vec<String>> = spec
            .iter()
            .map(|(_, _, deps, _, _)| deps.iter().map(|d| d.to_string()).collect())
            .collect();
        let goals: Vec<Option<Untrusted>> = spec
            .iter()
            .map(|(id, ..)| {
                extras
                    .goals
                    .iter()
                    .find(|(g, _)| g == id)
                    .map(|(_, text)| Untrusted::from_untrusted_source(text.to_string()))
            })
            .collect();
        let input = ListInput {
            nodes: spec
                .iter()
                .zip(&deps)
                .zip(&goals)
                .map(|((&(id, name, _, marker, band), deps), goal)| ListNode {
                    id,
                    name,
                    deps,
                    band,
                    marker,
                    done: marker == PhaseMarker::Done,
                    plans: extras.plans.iter().find(|(p, _)| *p == id).map(|(_, v)| *v),
                    goal: goal.as_ref(),
                    planned: extras.planned.contains(&id),
                    badge: extras
                        .badges
                        .iter()
                        .find(|(b, _)| *b == id)
                        .map(|(_, text)| text.to_string()),
                })
                .collect(),
            bands: bands
                .iter()
                .map(|&(label, shipped, declared)| BandInput {
                    label: Untrusted::from_untrusted_source(label.to_string()),
                    shipped,
                    declared_phases: declared,
                })
                .collect(),
        };
        layout_list(&input, &extras.toggles)
    }

    fn phase(id: &str) -> CursorTarget {
        CursorTarget::Phase(id.to_string())
    }

    fn render(
        model: &RoadmapModel,
        cursor: Option<&CursorTarget>,
        w: u16,
        h: u16,
        state: &mut RoadmapViewState,
    ) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(RoadmapView { model, cursor }, area, &mut buf, state);
        buf
    }

    /// The text of `rect` in `buf`, one string per row.
    fn rect_text(buf: &Buffer, rect: Rect) -> Vec<String> {
        (rect.y..rect.bottom())
            .map(|y| {
                (rect.x..rect.right())
                    .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()).to_string())
                    .collect::<String>()
            })
            .collect()
    }

    /// The rows of `lines` from the one starting with `from` (after the
    /// border) up to, not including, the one starting with `to`.
    fn between(lines: &[String], from: &str, to: &str) -> String {
        let body: Vec<String> = lines
            .iter()
            .map(|l| l.trim_start_matches('│').trim_start().to_string())
            .collect();
        let start = body
            .iter()
            .position(|l| l.starts_with(from))
            .unwrap_or_else(|| panic!("no {from:?} in {lines:#?}"));
        let end = body[start + 1..]
            .iter()
            .position(|l| l.starts_with(to))
            .map_or(body.len(), |i| start + 1 + i);
        body[start..end].join("\n")
    }

    /// The list rows naming `name` (the Start-now line excluded).
    fn rows_naming<'l>(lines: &'l [String], name: &str) -> Vec<&'l String> {
        let rows: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains(name) && !l.contains("Start now:"))
            .collect();
        assert!(!rows.is_empty(), "no row for {name:?} in {lines:#?}");
        rows
    }

    /// Mockup A's invented `bookly` roadmap (24-02's `lanes_mockup_a_with_bands`).
    const BOOKLY: Spec<'static> = &[
        ("8", "Booking data model", &[], D, Some(0)),
        ("9", "Availability API", &["8"], D, Some(0)),
        ("10", "Slot picker UI", &["9"], C, Some(0)),
        ("11", "Calendar sync", &["9"], F, Some(0)),
        ("12", "Checkout & payments", &["10", "11"], F, Some(0)),
        ("13", "Confirmation flow", &["12"], F, Some(0)),
        ("14", "Reminders & notifications", &["13"], F, Some(0)),
        ("15", "Live booking launch", &["14"], F, Some(0)),
        ("16", "Chat backend", &["13"], F, Some(1)),
        ("17", "Chat UI", &["16"], F, Some(1)),
        ("18", "Web booking portal", &["12"], F, Some(2)),
    ];

    const BOOKLY_BANDS: Bands<'static> = &[
        ("M3 Live booking", false, 8),
        ("M4 Support chat", false, 2),
        ("M5 Web", false, 1),
    ];

    const BOOKLY_PLANS: &[(&str, (u32, u32))] = &[
        ("8", (3, 3)),
        ("9", (4, 4)),
        ("10", (2, 3)),
        ("11", (0, 2)),
        ("12", (0, 3)),
    ];

    const BOOKLY_GOALS: &[(&str, &str)] = &[(
        "12",
        "A held slot can be paid for end to end, with refunds and an emailed receipt, before the hold expires.",
    )];

    fn bookly() -> RoadmapModel {
        build(
            BOOKLY,
            BOOKLY_BANDS,
            &Extras {
                plans: BOOKLY_PLANS,
                goals: BOOKLY_GOALS,
                ..Extras::default()
            },
        )
    }

    #[test]
    fn mockup_a_side_by_side_at_120_columns() {
        let model = bookly();
        let cursor = phase("12");
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&cursor), 120, 28, &mut state);
        let (list, detail) = panes(Rect::new(0, 0, 120, 28));
        assert_eq!(list.y, detail.y);
        assert!(detail.x >= list.right(), "{list:?} {detail:?}");

        let detail_text = rect_text(&buf, detail);
        let joined = detail_text.join("\n");
        for needle in [
            "Phase 12",
            "Checkout & payments",
            "wave 4 of 7",
            "none in wave 4",
        ] {
            assert!(joined.contains(needle), "{needle:?} missing:\n{joined}");
        }
        let needs = between(&detail_text, "Needs", "Unblocks");
        assert!(!needs.contains("nothing declared"), "{needs}");
        assert!(needs.contains("10") && needs.contains("11"), "{needs}");
        let unblocks = between(&detail_text, "Unblocks", "Parallel");
        for needle in ["13", "18", "M5"] {
            assert!(unblocks.contains(needle), "{needle:?} missing:\n{unblocks}");
        }

        let list_text = rect_text(&buf, list);
        for row in rows_naming(&list_text, "Checkout & payments") {
            assert!(row.contains(MARK_SELECTED), "{row}");
        }
        for name in ["Slot picker UI", "Calendar sync"] {
            for row in rows_naming(&list_text, name) {
                assert!(row.contains(MARK_DEP), "{row}");
            }
        }
        for name in ["Confirmation flow", "Web booking portal"] {
            for row in rows_naming(&list_text, name) {
                assert!(row.contains(MARK_UNBLOCKS), "{row}");
            }
        }
        let start = list_text
            .iter()
            .find(|l| l.contains("Start now:"))
            .expect("a Start now line");
        for needle in ["Slot picker UI (active)", PARALLEL_SEP, "Calendar sync"] {
            assert!(start.contains(needle), "{needle:?} missing: {start}");
        }
    }

    // -----------------------------------------------------------------------
    // Task 2: stacked layout, bands, lane cap, scrolling, empty states
    // -----------------------------------------------------------------------

    /// sentriq (Mockup C): the synthetic `v0.12` band and a shipped `v0.11`.
    const SENTRIQ: Spec<'static> = &[
        ("9", "Routine Event Logging (Schema v18)", &[], C, Some(1)),
        (
            "10",
            "The Air Box Test on a Fake Transport",
            &["9"],
            F,
            Some(1),
        ),
        (
            "11",
            "First Supervised On-Vehicle Run",
            &["10", "9"],
            F,
            Some(1),
        ),
        ("12", "Two-Truck Hardware Validation", &[], F, Some(1)),
    ];

    const SENTRIQ_BANDS: Bands<'static> = &[
        ("v0.11 Phases", true, 4),
        ("v0.12 Actuation Routines", false, 4),
    ];

    const SENTRIQ_GOALS: &[(&str, &str)] = &[(
        "12",
        "On real hardware, both trucks work from one phone with separately scoped data, and a first real F350 drive produces a logged session and a completed DTC scan.",
    )];

    /// daily-vow v1.5 (Mockup B): five shipped milestones listing no phases.
    const DAILY_VOW: Spec<'static> = &[
        ("18", "Calibration Foundation", &["17"], D, Some(5)),
        ("19", "Effective Profile Resolution", &["18"], D, Some(5)),
        (
            "20",
            "Notification Scheduling Extraction & Detection",
            &["19"],
            D,
            Some(5),
        ),
        ("21", "Probes End-to-End", &["20"], D, Some(5)),
        (
            "22",
            "Response-Weighted Nudge Selection",
            &["21"],
            D,
            Some(5),
        ),
        (
            "23",
            "Transparency, Reset & History",
            &["21", "20"],
            C,
            Some(5),
        ),
    ];

    const DAILY_VOW_BANDS: Bands<'static> = &[
        ("v1.0 MVP", true, 3),
        ("v1.1 Daily Rhythm", true, 2),
        ("v1.2 Learning", true, 4),
        ("v1.3 Adaptive Timing", true, 4),
        ("v1.4 Probes", true, 4),
        ("v1.5 Closing the Loop", false, 6),
        ("Requirement Coverage", false, 0),
    ];

    fn daily_vow(toggles: HashSet<BandKey>) -> RoadmapModel {
        build(
            DAILY_VOW,
            DAILY_VOW_BANDS,
            &Extras {
                plans: &[
                    ("18", (8, 8)),
                    ("19", (6, 6)),
                    ("20", (5, 5)),
                    ("21", (5, 5)),
                    ("22", (5, 5)),
                ],
                toggles,
                ..Extras::default()
            },
        )
    }

    /// Lane glyphs and status glyphs: what must never appear past the cap.
    const LANE_GLYPHS: &str = "│─├┤┬┴┐┌┘└┼●◉○◌";

    /// A row's cells inside the list border and padding (both sides).
    fn body(row: &str) -> Vec<char> {
        let chars: Vec<char> = row.chars().collect();
        chars[2..chars.len().saturating_sub(2).max(2)].to_vec()
    }

    #[test]
    fn panes_for_gives_the_list_its_rows_before_the_detail_pane_shrinks() {
        let area = Rect::new(0, 0, 80, 18);
        let heights = |want: u16| {
            let (list, detail) = panes_for(area, want);
            assert_eq!(list.height + detail.height, area.height);
            assert_eq!(detail.y, list.bottom());
            (list.height, detail.height)
        };
        assert_eq!(heights(0), (9, 9), "0 is panes()");
        assert_eq!(panes_for(area, 0), panes(area));
        assert_eq!(heights(9), (9, 9), "a list that fits keeps Mockup C's split");
        assert_eq!(heights(10), (10, 8));
        assert_eq!(heights(13), (12, 6));
        assert_eq!(heights(40), (12, 6), "never below six detail rows");
        let tiny = Rect::new(0, 0, 80, 8);
        assert_eq!(panes_for(tiny, 40), panes(tiny), "a tiny pane keeps its share");
        let wide = Rect::new(0, 0, 120, 18);
        assert_eq!(panes_for(wide, 40), panes(wide), "side by side ignores it");
    }

    /// WR-01 in the widget (ported from the removed legacy layout's
    /// `roadmap_graph_wr01_footer_never_takes_the_whole_body`): the cycle note
    /// takes the list pane's last row only, and not at all when the pane has
    /// no row to spare for it.
    #[test]
    fn the_cycle_note_takes_one_list_row_and_never_the_list() {
        let model = build(
            &[
                ("1", "One", &["2"], F, None),
                ("2", "Two", &["1"], F, None),
                ("3", "Three", &["4"], F, None),
                ("4", "Four", &["3"], F, None),
            ],
            &[],
            &Extras::default(),
        );
        assert_eq!(model.notes.len(), 1, "{:?}", model.notes);

        let mut state = RoadmapViewState::default();
        let buf = render(&model, None, 80, 24, &mut state);
        let want = u16::try_from(model.rows.len() + 5).unwrap();
        let (list, _) = panes_for(Rect::new(0, 0, 80, 24), want);
        let lines = rect_text(&buf, list);
        let notes: Vec<&String> = lines.iter().filter(|l| l.contains("Notes")).collect();
        assert_eq!(notes.len(), 1, "{lines:#?}");
        assert!(notes[0].contains("dependency cycles: 2"), "{}", notes[0]);
        assert!(lines[usize::from(list.height) - 2].contains("Notes"), "{lines:#?}");
        for name in ["One", "Two", "Three", "Four"] {
            assert!(lines.iter().any(|l| l.contains(name)), "{name}: {lines:#?}");
        }

        // A list pane of at most two inner rows (Start now and the header)
        // has no row for the note: at 80 columns that is every height to 9.
        for h in 1..=9u16 {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, None, 80, h, &mut state);
            let (list, _) = panes_for(Rect::new(0, 0, 80, h), want);
            if list.height <= 4 {
                let text = rect_text(&buf, list).join("\n");
                assert!(!text.contains("Notes"), "h={h}:\n{text}");
            }
        }
    }

    #[test]
    fn mockup_c_stacked_at_80_columns() {
        let model = build(
            SENTRIQ,
            SENTRIQ_BANDS,
            &Extras {
                plans: &[("9", (0, 2))],
                goals: SENTRIQ_GOALS,
                ..Extras::default()
            },
        );
        let cursor = phase("12");
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&cursor), 80, 20, &mut state);
        let (list, detail) = panes(Rect::new(0, 0, 80, 20));
        assert!(detail.y > list.y, "{list:?} {detail:?}");
        assert_eq!((list.width, detail.width), (80, 80));

        let joined = rect_text(&buf, detail).join("\n");
        for needle in [
            "Two-Truck Hardware Validation",
            "nothing declared",
            "no edge either way; can run any time",
            "j/k move",
        ] {
            assert!(joined.contains(needle), "{needle:?} missing:\n{joined}");
        }
        let list_text = rect_text(&buf, list).join("\n");
        assert!(
            list_text.contains("Two-Truck Hardware Validation"),
            "{list_text}"
        );
    }

    #[test]
    fn mockup_b_implied_dep_and_shipped_summary_at_120() {
        let model = daily_vow(HashSet::new());
        let cursor = phase("23");
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&cursor), 120, 28, &mut state);
        let (list, detail) = panes(Rect::new(0, 0, 120, 28));

        let detail_text = rect_text(&buf, detail);
        let joined = detail_text.join("\n");
        for needle in ["(implied via 21)", "nothing (last in v1.5)"] {
            assert!(joined.contains(needle), "{needle:?} missing:\n{joined}");
        }
        let parallel = between(&detail_text, "Parallel", "\u{23CE}");
        assert!(parallel.contains("22"), "{parallel}");

        let list_text = rect_text(&buf, list);
        let row_20: Vec<&String> = list_text
            .iter()
            .filter(|l| l.contains(" 20 ") && l.contains("Notification"))
            .collect();
        assert_eq!(row_20.len(), 1, "{list_text:#?}");
        assert!(row_20[0].contains(MARK_IMPLIED), "{}", row_20[0]);
        let name_end = row_20[0]
            .trim_end_matches('│')
            .trim_end()
            .split("  ")
            .find(|part| part.contains("Notification"))
            .unwrap_or_default()
            .to_string();
        assert!(name_end.ends_with(ELLIPSIS), "{name_end:?}");

        let summary: Vec<&String> = list_text
            .iter()
            .filter(|l| l.contains("5 milestones \u{00B7} 17 phases shipped"))
            .collect();
        assert_eq!(summary.len(), 1, "{list_text:#?}");
        assert!(summary[0].contains(BAND_FOLDED), "{}", summary[0]);
    }

    #[test]
    fn band_labels_are_drawn_once() {
        let m4 = BandKey::Named("m4 support chat".to_string());
        let model = build(
            BOOKLY,
            BOOKLY_BANDS,
            &Extras {
                toggles: std::iter::once(m4).collect(),
                ..Extras::default()
            },
        );
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&phase("12")), 120, 28, &mut state);
        let (list, _) = panes(Rect::new(0, 0, 120, 28));
        let list_text = rect_text(&buf, list);
        for (label, glyph) in [
            ("M3 Live booking", BAND_OPEN),
            ("M4 Support chat", BAND_FOLDED),
            ("M5 Web", BAND_OPEN),
        ] {
            let rows: Vec<&String> = list_text.iter().filter(|l| l.contains(label)).collect();
            assert_eq!(rows.len(), 1, "{label}: {list_text:#?}");
            assert!(rows[0].contains(glyph), "{label}: {}", rows[0]);
            assert!(rows[0].contains(BAND_FILL), "{label}: {}", rows[0]);
        }

        // The unfolded shipped summary: every shipped label on one row too.
        let model = daily_vow(std::iter::once(BandKey::Shipped).collect());
        let buf = render(&model, Some(&phase("23")), 120, 28, &mut state);
        let list_text = rect_text(&buf, list);
        for (label, ..) in DAILY_VOW_BANDS.iter().filter(|b| b.1) {
            let rows = list_text.iter().filter(|l| l.contains(label)).count();
            assert_eq!(rows, 1, "{label}: {list_text:#?}");
        }
        let summary = list_text
            .iter()
            .find(|l| l.contains("phases shipped"))
            .expect("the shipped summary row");
        assert!(summary.contains(BAND_OPEN), "{summary}");
    }

    /// Quick 260926-kes (D-01, I-3, T-kes-05): the list body and the fold
    /// glyph cells the mouse hit-tests are what the render drew.
    #[test]
    fn mouse_roadmap_view_records_the_list_body_and_fold_glyph_cells() {
        let unfolded = daily_vow(std::iter::once(BandKey::Shipped).collect());
        // Quick 260927-gi9 (D-05): the compact folded gutter and the wide
        // unfolded one.
        let models = [
            daily_vow(HashSet::new()),
            unfolded,
            bookly(),
            cross_fold(HashSet::new()),
            cross_fold(std::iter::once(BandKey::Shipped).collect()),
        ];
        for model in models {
            for (w, h) in [(120u16, 30u16), (80, 24), (120, 12)] {
                for start in [0usize, 4] {
                    let at = format!("{w}x{h}, offset {start}");
                    let mut state = RoadmapViewState {
                        offset: start,
                        ..RoadmapViewState::default()
                    };
                    let buf = render(&model, Some(&phase("12")), w, h, &mut state);
                    let want = model.rows.len() + 4 + usize::from(!model.notes.is_empty());
                    let (list, _) =
                        panes_for(Rect::new(0, 0, w, h), u16::try_from(want).unwrap());
                    let inner = pane_block().inner(list);
                    let body = state.list_body;
                    assert_eq!(body.x, inner.x, "{at}");
                    assert_eq!(body.width, inner.width, "{at}");
                    assert_eq!(body.y, inner.y + 2, "{at}: below Start now and the header");
                    let note = u16::from(!model.notes.is_empty() && inner.height > 2);
                    assert_eq!(body.bottom(), inner.bottom() - note, "{at}: above Notes");
                    assert_eq!(body.height, state.list_rows, "{at}");

                    let visible = state.offset..state.offset + usize::from(state.list_rows);
                    let expected: Vec<usize> = model
                        .rows
                        .iter()
                        .enumerate()
                        .filter(|(i, row)| {
                            visible.contains(i)
                                && matches!(
                                    row,
                                    ListRow::Band { .. } | ListRow::ShippedSummary { .. }
                                )
                        })
                        .map(|(i, _)| i)
                        .collect();
                    let marked: Vec<usize> = state.fold_marks.iter().map(|m| m.0).collect();
                    assert_eq!(marked, expected, "{at}: one mark per visible band row");
                    for (row, rect) in &state.fold_marks {
                        let y = body.y + u16::try_from(row - state.offset).unwrap();
                        assert_eq!(rect.y, y, "{at}: row {row}");
                        assert_eq!(rect.height, 1, "{at}");
                        assert_eq!(rect.width, 2, "{at}: the glyph and its space");
                        let glyph = buf.cell((rect.x, rect.y)).map(|c| c.symbol().to_string());
                        assert!(
                            glyph.as_deref() == Some(BAND_OPEN)
                                || glyph.as_deref() == Some(BAND_FOLDED),
                            "{at}: row {row} marks {glyph:?}"
                        );
                        let folded = match model.rows[*row] {
                            ListRow::Band { folded, .. }
                            | ListRow::ShippedSummary { folded, .. } => folded,
                            _ => unreachable!(),
                        };
                        let drawn = if folded { BAND_FOLDED } else { BAND_OPEN };
                        assert_eq!(glyph.as_deref(), Some(drawn), "{at}: row {row}");
                        assert!(rect.x >= inner.x && rect.right() <= inner.right(), "{at}");
                    }
                }
            }
        }

        // A scrolled list marks only what it shows.
        let model = daily_vow(std::iter::once(BandKey::Shipped).collect());
        let mut state = RoadmapViewState::default();
        render(&model, Some(&phase("23")), 80, 14, &mut state);
        assert!(state.offset > 0, "phase 23 at 80x14 needs a scrolled list");
        assert!(state.fold_marks.iter().all(|(row, _)| *row >= state.offset));

        // An empty model and a zero-size area record nothing.
        let mut state = RoadmapViewState {
            list_body: Rect::new(1, 1, 5, 5),
            fold_marks: vec![(0, Rect::new(1, 1, 2, 1))],
            ..RoadmapViewState::default()
        };
        render(&RoadmapModel::default(), None, 80, 24, &mut state);
        assert!(state.list_body.is_empty());
        assert!(state.fold_marks.is_empty());
        let mut state = RoadmapViewState {
            list_body: Rect::new(1, 1, 5, 5),
            fold_marks: vec![(0, Rect::new(1, 1, 2, 1))],
            ..RoadmapViewState::default()
        };
        let model = bookly();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        StatefulWidget::render(
            RoadmapView { model: &model, cursor: None },
            Rect::new(0, 0, 0, 0),
            &mut buf,
            &mut state,
        );
        assert!(state.list_body.is_empty());
        assert!(state.fold_marks.is_empty());
    }

    /// Quick 260927-gi9 (I-7): the reported shape. Shipped v1 lists no
    /// phases; shipped v2's six independent phases all feed v3's 14.
    const CROSS_FOLD: Spec<'static> = &[
        ("8", "Booking model", &[], D, Some(1)),
        ("9", "Availability", &[], D, Some(1)),
        ("10", "Slot picker", &[], D, Some(1)),
        ("11", "Calendar sync", &[], D, Some(1)),
        ("12", "Checkout", &[], D, Some(1)),
        ("13", "Confirmation", &[], D, Some(1)),
        (
            "14",
            "Supervised first live booking",
            &["8", "9", "10", "11", "12", "13"],
            C,
            Some(2),
        ),
        ("15", "Unattended window runs", &["14"], F, Some(2)),
        ("16", "Five9 client and transcript store", &[], D, Some(2)),
        ("17", "Autonomous responder", &["16"], F, Some(2)),
        ("18", "Telegram commands, users and group", &[], D, Some(2)),
        ("19", "Telegram Mini App and LAN HTTPS", &["18"], F, Some(2)),
        ("20", "Web frontend", &["19"], F, Some(3)),
    ];

    const CROSS_FOLD_BANDS: Bands<'static> = &[
        ("v1 Foundations", true, 8),
        ("v2 Core booking", true, 6),
        ("v3 Live booking, support chat and Telegram", false, 6),
        ("Milestone 6: Web frontend", false, 1),
    ];

    fn cross_fold(toggles: HashSet<BandKey>) -> RoadmapModel {
        build(
            CROSS_FOLD,
            CROSS_FOLD_BANDS,
            &Extras {
                toggles,
                ..Extras::default()
            },
        )
    }

    /// Quick 260927-gi9 (D-01, D-05): with v1 … v2 folded, the lane column
    /// is the minimum width, so the ids and the v3 fold mark sit right after
    /// it. Columns are measured from the recorded list body.
    #[test]
    fn folded_milestones_do_not_widen_the_lane_column() {
        let model = cross_fold(HashSet::new());
        for (w, h) in [(120u16, 30u16), (80, 30)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&phase("14")), w, h, &mut state);
            let body = state.list_body;
            let rows = rect_text(&buf, body);
            let dump = || rows.join("\n");

            // (a) The phase id column starts right after lanes and marker.
            let row = rows
                .iter()
                .find(|r| r.contains("Supervised first live booking"))
                .unwrap_or_else(|| panic!("{w}x{h}: no row for 14 in\n{}", dump()));
            let id: String = row
                .chars()
                .skip(MIN_LANE_CELLS + MARK_CELLS)
                .take(2)
                .collect();
            assert_eq!(id, "14", "{w}x{h}:\n{}", dump());

            // (b) The v3 fold mark sits right after the minimum lane column.
            let v3 = state
                .fold_marks
                .iter()
                .find(|(i, _)| matches!(&model.rows[*i], ListRow::Band { short, .. } if short == "v3"))
                .unwrap_or_else(|| panic!("{w}x{h}: no v3 fold mark in\n{}", dump()));
            assert_eq!(
                v3.1.x,
                body.x + u16::try_from(MIN_LANE_CELLS).unwrap(),
                "{w}x{h}:\n{}",
                dump()
            );

            // (c) No lane is drawn past the minimum lane column.
            for r in &rows {
                let past: String = r
                    .chars()
                    .skip(MIN_LANE_CELLS)
                    .take(MARK_CELLS)
                    .collect();
                assert!(
                    !past
                        .chars()
                        .any(|c| LANE_GLYPHS.contains(c) || LANE_OVERFLOW.contains(c)),
                    "{w}x{h}: lane glyph in the marker cells of {r:?}"
                );
            }
        }

        // Unfolded, v2's lanes really are drawn on the v3 row, so the gutter
        // widens and the fold mark moves right with the drawn glyph.
        let open = cross_fold(std::iter::once(BandKey::Shipped).collect());
        let mut state = RoadmapViewState::default();
        render(&open, Some(&phase("14")), 120, 40, &mut state);
        let v3 = state
            .fold_marks
            .iter()
            .find(|(i, _)| matches!(&open.rows[*i], ListRow::Band { short, .. } if short == "v3"))
            .expect("unfolded v3 fold mark");
        assert!(
            v3.1.x > state.list_body.x + u16::try_from(MIN_LANE_CELLS).unwrap(),
            "{v3:?} vs {:?}",
            state.list_body
        );
    }

    /// `1` forks into eight children that all merge into `10`: eight lanes
    /// run side by side.
    fn eight_lanes() -> RoadmapModel {
        const WIDE: Spec<'static> = &[
            ("1", "Root", &[], C, None),
            ("2", "Alpha", &["1"], F, None),
            ("3", "Bravo", &["1"], F, None),
            ("4", "Charlie", &["1"], F, None),
            ("5", "Delta", &["1"], F, None),
            ("6", "Echo", &["1"], F, None),
            ("7", "Foxtrot", &["1"], F, None),
            ("8", "Golf", &["1"], F, None),
            ("9", "Hotel", &["1"], F, None),
            (
                "10",
                "Merge",
                &["2", "3", "4", "5", "6", "7", "8", "9"],
                F,
                None,
            ),
        ];
        build(WIDE, &[], &Extras::default())
    }

    #[test]
    fn lanes_past_the_cap_collapse_into_one_overflow_column() {
        let model = eight_lanes();
        let widest = model
            .rows
            .iter()
            .map(|r| lanes_of(r).chars().count())
            .max()
            .unwrap_or(0);
        assert!(
            widest >= 15,
            "the fixture must draw 8 lanes: {:#?}",
            roadmap_graph::lane_text(&model)
        );
        for (w, cap) in [(80u16, LANE_CAP_NARROW), (120, LANE_CAP_WIDE)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&phase("1")), w, 40, &mut state);
            let (list, _) = panes(Rect::new(0, 0, w, 40));
            let list_text = rect_text(&buf, list);
            // Skip the top border, Start-now and header rows and the bottom border.
            let rows: Vec<Vec<char>> = list_text[3..list_text.len() - 1]
                .iter()
                .map(|r| body(r))
                .collect();
            assert!(
                rows.iter().any(|r| r.contains(&'\u{2506}')),
                "{w}: no overflow column in {list_text:#?}"
            );
            for row in &rows {
                let past: String = row.iter().skip(2 * cap + 1).collect();
                assert!(
                    !past.chars().any(|c| LANE_GLYPHS.contains(c)),
                    "{w}: lane glyph past the cap in {:?}",
                    row.iter().collect::<String>()
                );
            }
        }
    }

    #[test]
    fn keep_visible_clamps_both_ways() {
        assert_eq!(keep_visible(0, 30, 10), 21);
        assert_eq!(keep_visible(25, 3, 10), 3);
        assert_eq!(keep_visible(5, 8, 10), 5);
    }

    #[test]
    fn the_cursor_row_is_scrolled_into_view() {
        let ids: Vec<String> = (1..=30).map(|i| i.to_string()).collect();
        let names: Vec<String> = (1..=30).map(|i| format!("Step {i}")).collect();
        let deps: Vec<Vec<&str>> = (1..=30)
            .map(|i| {
                if i == 1 {
                    Vec::new()
                } else {
                    vec![ids[i - 2].as_str()]
                }
            })
            .collect();
        let spec: Vec<Row<'_>> = (0..30)
            .map(|i| {
                let marker = if i == 29 { C } else { D };
                (
                    ids[i].as_str(),
                    names[i].as_str(),
                    deps[i].as_slice(),
                    marker,
                    None,
                )
            })
            .collect();
        let model = build(&spec, &[], &Extras::default());
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&phase("30")), 120, 12, &mut state);
        let (list, _) = panes(Rect::new(0, 0, 120, 12));
        assert_eq!(state.list_rows, 8);
        assert_eq!(state.offset, 22);
        let list_text = rect_text(&buf, list);
        let row = rows_naming(&list_text, "Step 30");
        assert!(row[0].contains(MARK_SELECTED), "{list_text:#?}");
    }

    #[test]
    fn an_empty_model_explains_itself() {
        let model = RoadmapModel::default();
        for (w, h) in [(80u16, 20u16), (120, 28)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, None, w, h, &mut state);
            let text = rect_text(&buf, Rect::new(0, 0, w, h)).join("\n");
            assert!(text.contains("No roadmap data available"), "{text}");
            assert_eq!(state.list_rows, 0);
        }
    }

    #[test]
    fn a_planned_phase_says_so() {
        let spec: Spec<'_> = &[("14", "Supervised first live booking", &[], F, None)];
        let model = build(
            spec,
            &[],
            &Extras {
                planned: &["14"],
                ..Extras::default()
            },
        );
        for (w, h) in [(120u16, 28u16), (80, 20)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&phase("14")), w, h, &mut state);
            let (_, detail) = panes(Rect::new(0, 0, w, h));
            let joined = rect_text(&buf, detail).join("\n");
            assert!(
                joined.contains("planned (not a GSD phase)"),
                "{w}:\n{joined}"
            );
        }
    }

    #[test]
    fn the_stage_badge_is_in_the_status_line() {
        let spec: Spec<'_> = &[("7", "Queue", &[], C, None)];
        let model = build(
            spec,
            &[],
            &Extras {
                plans: &[("7", (2, 3))],
                badges: &[("7", "[Executing 2/3]")],
                ..Extras::default()
            },
        );
        for (w, h) in [(120u16, 28u16), (80, 20)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&phase("7")), w, h, &mut state);
            let (_, detail) = panes(Rect::new(0, 0, w, h));
            let text = rect_text(&buf, detail);
            let status = text
                .iter()
                .find(|l| l.contains("active"))
                .unwrap_or_else(|| panic!("{w}: no status line in {text:#?}"));
            assert!(status.contains("[Executing 2/3]"), "{w}: {status}");
            assert!(!status.contains("[["), "{w}: {status}");
        }
    }

    #[test]
    fn a_band_cursor_has_its_own_detail_pane() {
        let model = bookly();
        let key = model.bands[0].key.clone();
        let cursor = CursorTarget::Band(key);
        for (w, h) in [(120u16, 28u16), (80, 30)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&cursor), w, h, &mut state);
            let (list, detail) = panes(Rect::new(0, 0, w, h));
            let joined = rect_text(&buf, detail).join("\n");
            for needle in [
                " M3 ",
                "M3 Live booking",
                "2/8 phases done",
                "Space fold/unfold",
            ] {
                assert!(
                    joined.contains(needle),
                    "{w}: {needle:?} missing:\n{joined}"
                );
            }
            let list_text = rect_text(&buf, list);
            let band_row = list_text
                .iter()
                .find(|l| l.contains("M3 Live booking"))
                .expect("the M3 band row");
            assert!(
                buf.cell((list.x + 3, list.y + 3))
                    .is_some_and(|c| c.modifier.contains(Modifier::REVERSED)),
                "{w}: the band row under the cursor is drawn reversed: {band_row}"
            );
        }
    }

    #[test]
    fn the_shipped_summary_cursor_lists_its_milestones() {
        let model = daily_vow(HashSet::new());
        let cursor = CursorTarget::Band(BandKey::Shipped);
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&cursor), 120, 28, &mut state);
        let (_, detail) = panes(Rect::new(0, 0, 120, 28));
        let text = rect_text(&buf, detail);
        let joined = text.join("\n");
        assert!(joined.contains(" Shipped "), "{joined}");
        assert!(joined.contains("\u{23CE} open milestones"), "{joined}");
        for (label, _, declared) in DAILY_VOW_BANDS.iter().filter(|b| b.1) {
            let line = text
                .iter()
                .find(|l| l.contains(label))
                .unwrap_or_else(|| panic!("{label} missing:\n{joined}"));
            let count = if *declared == 1 {
                "1 phase".to_string()
            } else {
                format!("{declared} phases")
            };
            assert!(line.contains(&count), "{label}: {line}");
        }
    }

    // -----------------------------------------------------------------------
    // Task 3: escaping, tiny sizes, no bleed, multibyte truncation
    // -----------------------------------------------------------------------

    /// Every cell's symbol, row by row.
    fn buffer_text(buf: &Buffer) -> String {
        rect_text(buf, buf.area).join("\n")
    }

    fn escaped(raw: &str) -> String {
        String::from(crate::text::render_for_terminal(raw))
    }

    const HOSTILE_ID: &str = "7\u{202E}";
    const HOSTILE_NAME: &str = "Evil\u{1b}[31m \u{202E}name";
    const HOSTILE_GOAL: &str = "Goal\u{1b}[2J \u{202E}text";
    const HOSTILE_LABEL: &str = "M9\u{1b}[31m \u{202E}Hostile";

    fn hostile() -> RoadmapModel {
        let spec: Spec<'_> = &[
            ("6", "Clean", &[], D, Some(0)),
            (HOSTILE_ID, HOSTILE_NAME, &["6"], C, Some(0)),
        ];
        build(
            spec,
            &[(HOSTILE_LABEL, false, 2)],
            &Extras {
                goals: &[(HOSTILE_ID, HOSTILE_GOAL)],
                badges: &[(HOSTILE_ID, "[Exec\u{1b}[31m 1/2]")],
                ..Extras::default()
            },
        )
    }

    #[test]
    fn roadmap_view_stores_and_draws_only_escaped_text() {
        for raw in [HOSTILE_ID, HOSTILE_NAME, HOSTILE_GOAL, HOSTILE_LABEL] {
            assert_ne!(escaped(raw), raw, "the fixture {raw:?} must need escaping");
        }
        let model = hostile();
        let key = model.phases[1].key.clone();
        let band = CursorTarget::Band(model.bands[0].key.clone());
        let on_phase = CursorTarget::Phase(key);
        for (w, h) in [(120u16, 28u16), (80, 20)] {
            for cursor in [&on_phase, &band] {
                let mut state = RoadmapViewState::default();
                let buf = render(&model, Some(cursor), w, h, &mut state);
                for cell in buf.content() {
                    let symbol = cell.symbol();
                    assert!(
                        !symbol.contains('\u{1b}') && !symbol.contains('\u{202E}'),
                        "{w}x{h}: raw control in a cell: {symbol:?}"
                    );
                }
                let text = buffer_text(&buf);
                assert!(text.contains(&escaped(HOSTILE_NAME)), "{w}:\n{text}");
                assert!(text.contains(&escaped(HOSTILE_LABEL)), "{w}:\n{text}");
            }
        }
        // The side-by-side phase pane shows the escaped id, goal and badge.
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&on_phase), 120, 28, &mut state);
        let text = buffer_text(&buf);
        for raw in [HOSTILE_ID, HOSTILE_GOAL, "[Exec\u{1b}[31m 1/2]"] {
            assert!(text.contains(&escaped(raw)), "{raw:?}:\n{text}");
        }
    }

    #[test]
    fn roadmap_view_never_panics_at_tiny_sizes() {
        let models = [
            bookly(),
            daily_vow(HashSet::new()),
            eight_lanes(),
            hostile(),
            RoadmapModel::default(),
        ];
        for model in &models {
            let mut cursors = vec![None];
            cursors.extend(model.visible_targets().into_iter().take(3).map(Some));
            if let Some(band) = model.bands.first() {
                cursors.push(Some(CursorTarget::Band(band.key.clone())));
            }
            cursors.push(Some(CursorTarget::Band(BandKey::Shipped)));
            for cursor in &cursors {
                for (w, h) in [(1u16, 1u16), (5, 3), (10, 3), (20, 5), (40, 8)] {
                    let mut state = RoadmapViewState {
                        offset: 99,
                        ..RoadmapViewState::default()
                    };
                    render(model, cursor.as_ref(), w, h, &mut state);
                }
            }
        }
    }

    #[test]
    fn roadmap_view_never_draws_outside_its_rect() {
        let model = bookly();
        let cursor = phase("12");
        for (full, inner) in [
            (Rect::new(0, 0, 20, 10), Rect::new(3, 2, 5, 3)),
            (Rect::new(0, 0, 140, 40), Rect::new(5, 3, 120, 30)),
            (Rect::new(0, 0, 100, 30), Rect::new(7, 4, 85, 22)),
        ] {
            let mut buf = Buffer::empty(full);
            for y in full.y..full.bottom() {
                buf.set_string(0, y, "x".repeat(usize::from(full.width)), Style::default());
            }
            let before = buf.clone();
            let mut state = RoadmapViewState::default();
            StatefulWidget::render(
                RoadmapView {
                    model: &model,
                    cursor: Some(&cursor),
                },
                inner,
                &mut buf,
                &mut state,
            );
            for y in full.y..full.bottom() {
                for x in full.x..full.right() {
                    if !inner.contains(ratatui::layout::Position { x, y }) {
                        assert_eq!(
                            buf.cell((x, y)),
                            before.cell((x, y)),
                            "bled at ({x},{y}) rendering into {inner:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_multibyte_name_truncates_on_char_boundaries() {
        let name: String = "\u{00DC}berpr\u{00FC}fung \u{65E5}\u{672C}\u{8A9E} \u{2713} "
            .chars()
            .cycle()
            .take(70)
            .collect();
        assert_eq!(name.chars().count(), 70);
        assert!(name.len() > 70, "the fixture must be multibyte");
        let spec: Vec<Row<'_>> = vec![("1", name.as_str(), &[], C, None)];
        let model = build(&spec, &[], &Extras::default());
        for (w, h) in [(80u16, 20u16), (120, 28), (61, 12)] {
            let mut state = RoadmapViewState::default();
            let buf = render(&model, Some(&phase("1")), w, h, &mut state);
            let (list, _) = panes(Rect::new(0, 0, w, h));
            let list_text = rect_text(&buf, list);
            let rows = rows_naming(&list_text, "\u{00DC}berpr\u{00FC}fung");
            assert!(rows[0].contains(ELLIPSIS), "{w}: {}", rows[0]);
        }
    }

    // ── quick 260929-szq: the unmerged phase status ───────────────────────

    #[test]
    fn the_unmerged_status_is_cyan_undimmed_and_shares_the_glyph() {
        assert_eq!(PhaseStatus::Unmerged.glyph(), GLYPH_UNMERGED);
        assert_eq!(GLYPH_UNMERGED, crate::agents::unmerged::GLYPH_UNMERGED);
        assert_eq!(status_word(PhaseStatus::Unmerged), "unmerged");
        let style = glyph_style(PhaseStatus::Unmerged);
        assert_eq!(style.fg, Some(Color::Cyan));
        assert_ne!(style, dim());
    }

    #[test]
    fn the_status_line_ends_with_the_unmerged_count() {
        let mut model = build(
            &[("05", "Export", &[], C, None)],
            &[],
            &Extras {
                plans: &[("05", (2, 5))],
                ..Extras::default()
            },
        );
        let spans_of = |model: &RoadmapModel| -> String {
            RoadmapView::status_spans(&model.phases[0])
                .iter()
                .map(|s| s.content.to_string())
                .collect()
        };
        assert!(!spans_of(&model).contains(GLYPH_UNMERGED), "k = 0: no tail");
        model.phases[0].unmerged = 3;
        let spans = RoadmapView::status_spans(&model.phases[0]);
        let last = spans.last().expect("spans");
        assert_eq!(last.content, format!(" {GLYPH_UNMERGED}3"));
        assert_eq!(last.style.fg, Some(Color::Cyan));
        assert!(spans_of(&model).contains("2/5 plans"), "{}", spans_of(&model));
    }

    // ── quick 261006-ujx: git-graph lane styling ──────────────────────────

    /// Six phases in one band: `1` and `2` done, `3` active and forking to
    /// `4` and `5`, and the unrelated `2 -> 6` lane crossing the fork.
    /// Phase indices equal list order (`1` is index 0).
    const CHAIN: Spec<'static> = &[
        ("1", "Alpha", &[], D, Some(0)),
        ("2", "Bravo", &[], D, Some(0)),
        ("3", "Charlie", &["1"], C, Some(0)),
        ("4", "Delta", &["3"], F, Some(0)),
        ("5", "Echo", &["3"], F, Some(0)),
        ("6", "Foxtrot", &["2"], F, Some(0)),
    ];
    const CHAIN_BANDS: Bands<'static> = &[("M1 Chain", false, 6)];
    /// CHAIN's model rows: the band, then these.
    const CHAIN_ROW_2: usize = 2;
    const CHAIN_ROW_3: usize = 3;
    const CHAIN_FORK: usize = 4;
    const CHAIN_ROW_4: usize = 5;
    const CHAIN_ROW_5: usize = 6;

    fn chain() -> RoadmapModel {
        let model = build(CHAIN, CHAIN_BANDS, &Extras::default());
        let fork = format!(
            "{}{}{}{}{}",
            roadmap_graph::LANE_TEE_RIGHT,
            roadmap_graph::LANE_HORIZONTAL,
            roadmap_graph::LANE_CROSS,
            roadmap_graph::LANE_HORIZONTAL,
            roadmap_graph::LANE_DOWN_LEFT,
        );
        assert_eq!(
            roadmap_graph::lane_text(&model),
            vec![
                "          [M1]",
                "o         1",
                "\u{2502} o       2",
                "o \u{2502}       3",
                fork.as_str(),
                "o \u{2502} \u{2502}     4",
                "  \u{2502} o     5",
                "  o       6",
            ]
        );
        model
    }

    /// The buffer cell of lane char `i` on model row `k` (no scroll), for a
    /// render at `w` x `h`.
    fn lane_cell(buf: &Buffer, w: u16, h: u16, k: usize, i: usize) -> &ratatui::buffer::Cell {
        let (list, _) = panes(Rect::new(0, 0, w, h));
        let inner = pane_block().inner(list);
        let x = inner.x + u16::try_from(i).unwrap();
        let y = inner.y + 2 + u16::try_from(k).unwrap();
        buf.cell((x, y)).expect("in the buffer")
    }

    /// CHAIN at 120x30 with `cursor`.
    fn chain_buf(model: &RoadmapModel, cursor: &CursorTarget) -> Buffer {
        let mut state = RoadmapViewState::default();
        render(model, Some(cursor), 120, 30, &mut state)
    }

    fn band_cursor(model: &RoadmapModel) -> CursorTarget {
        CursorTarget::Band(model.bands[0].key.clone())
    }

    #[test]
    fn lane_cells_take_the_palette_hue_of_their_column() {
        let model = chain();
        let buf = chain_buf(&model, &band_cursor(&model));
        let cell = |k, i| lane_cell(&buf, 120, 30, k, i);
        for (k, i) in [
            (CHAIN_FORK, 0),
            (CHAIN_FORK, 1),
            (CHAIN_FORK, 3),
            (CHAIN_FORK, 4),
            (CHAIN_ROW_4, 4),
        ] {
            assert_eq!(cell(k, i).fg, LANE_PALETTE[(i / 2) % 5], "row {k} char {i}");
        }
        assert_eq!(cell(CHAIN_FORK, 4).fg, LANE_PALETTE[2]);
        assert_eq!(cell(CHAIN_ROW_4, 4).fg, LANE_PALETTE[2]);
        // The 6th lane wraps to the first hue.
        assert_eq!(
            lane_style(5, false, false, false, true).fg,
            Some(LANE_PALETTE[0])
        );
    }

    #[test]
    fn a_lane_out_of_a_done_phase_draws_dark_gray() {
        let model = chain();
        let buf = chain_buf(&model, &band_cursor(&model));
        let cell = |k, i| lane_cell(&buf, 120, 30, k, i);
        // 1 -> 3 passing row 2; 2 -> 6 down column 1, crossed by the fork.
        for (k, i) in [
            (CHAIN_ROW_2, 0),
            (CHAIN_ROW_3, 2),
            (CHAIN_FORK, 2),
            (CHAIN_ROW_4, 2),
            (CHAIN_ROW_5, 2),
        ] {
            assert_eq!(cell(k, i).fg, Color::DarkGray, "row {k} char {i}");
        }
        // The node glyph keeps its status style, never a lane hue (S-04).
        let node = cell(CHAIN_ROW_3, 0);
        assert_eq!(node.symbol(), PhaseStatus::Active.glyph());
        assert_eq!(node.fg, Color::Yellow);
        assert!(node.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_done_row_no_longer_dims_its_whole_lane_cell() {
        let model = build(
            &[
                ("1", "A", &[], C, Some(0)),
                ("2", "B", &[], D, Some(0)),
                ("3", "C", &["1"], F, Some(0)),
            ],
            &[("M1", false, 3)],
            &Extras::default(),
        );
        assert_eq!(roadmap_graph::lane_text(&model)[2], "\u{2502} o       2");
        let cursor = band_cursor(&model);
        let buf = chain_buf(&model, &cursor);
        let lane = lane_cell(&buf, 120, 30, 2, 0);
        assert_eq!(lane.fg, LANE_PALETTE[0], "owned by the active 1");
        // The row's text is still dimmed.
        let (list, _) = panes(Rect::new(0, 0, 120, 30));
        let inner = pane_block().inner(list);
        let y = inner.y + 2 + 2;
        let name = (inner.x + 8..inner.right())
            .filter_map(|x| buf.cell((x, y)))
            .find(|c| c.symbol() == "B")
            .expect("the name cell");
        assert_eq!(name.fg, Color::DarkGray);
    }

    /// Every non-blank, non-node lane char of CHAIN's model rows as
    /// `(row, char index, link)`.
    fn chain_lane_chars(model: &RoadmapModel) -> Vec<(usize, usize, Option<LaneLink>)> {
        let mut out = Vec::new();
        for (k, row) in model.rows.iter().enumerate() {
            let (lanes, links, glyph_at) = match row {
                ListRow::Phase {
                    lane, lanes, links, ..
                } => (lanes, links, Some(2 * lane)),
                ListRow::Band { lanes, links, .. }
                | ListRow::ShippedSummary { lanes, links, .. }
                | ListRow::Connector { lanes, links } => (lanes, links, None),
            };
            for (i, c) in lanes.chars().enumerate() {
                if c != ' ' && Some(i) != glyph_at {
                    out.push((k, i, links.get(i).copied().flatten()));
                }
            }
        }
        out
    }

    #[test]
    fn lane_style_bolds_the_chain_and_keeps_its_hue_when_done() {
        let on = lane_style(3, true, true, false, true);
        assert_eq!(on.fg, Some(LANE_PALETTE[3]));
        assert!(on.add_modifier.contains(Modifier::BOLD));
        let off = lane_style(3, true, false, false, true);
        assert_eq!(off.fg, Some(Color::DarkGray));
        assert!(!off.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn lane_style_dims_off_chain_lanes_only() {
        for owner_done in [false, true] {
            for palette in [false, true] {
                let off = lane_style(1, owner_done, false, true, palette);
                assert!(off.add_modifier.contains(Modifier::DIM));
                let on = lane_style(1, owner_done, true, true, palette);
                assert!(!on.add_modifier.contains(Modifier::DIM), "{on:?}");
            }
        }
    }

    #[test]
    fn lane_style_without_palette_keeps_weight_only() {
        for lane in 0..7 {
            for owner_done in [false, true] {
                for on_chain in [false, true] {
                    for dimmed in [false, true] {
                        let style = lane_style(lane, owner_done, on_chain, dimmed, false);
                        assert_eq!(style.fg, None, "{style:?}");
                        if on_chain {
                            assert!(style.add_modifier.contains(Modifier::BOLD));
                        }
                    }
                }
            }
        }
        let done = lane_style(0, true, false, false, false);
        assert!(done.add_modifier.contains(Modifier::DIM));
        let open = lane_style(0, false, false, false, false);
        assert_eq!(open, Style::default());
    }

    #[test]
    fn selecting_a_phase_bolds_its_edges_and_dims_the_rest() {
        let model = chain();
        let buf = chain_buf(&model, &phase("3"));
        let cell = |k, i| lane_cell(&buf, 120, 30, k, i);
        let bold = |k: usize, i: usize| {
            let c = cell(k, i);
            assert!(
                c.modifier.contains(Modifier::BOLD),
                "row {k} char {i}: {c:?}"
            );
            assert!(
                !c.modifier.contains(Modifier::DIM),
                "row {k} char {i}: {c:?}"
            );
            assert_eq!(c.fg, LANE_PALETTE[(i / 2) % 5], "row {k} char {i}");
        };
        let dimmed = |k: usize, i: usize| {
            let c = cell(k, i);
            assert!(
                c.modifier.contains(Modifier::DIM),
                "row {k} char {i}: {c:?}"
            );
            assert!(
                !c.modifier.contains(Modifier::BOLD),
                "row {k} char {i}: {c:?}"
            );
        };
        // 1 -> 3 keeps its hue although 1 is Done.
        bold(CHAIN_ROW_2, 0);
        for i in [0, 1, 3, 4] {
            bold(CHAIN_FORK, i);
        }
        bold(CHAIN_ROW_4, 4);
        // The unrelated 2 -> 6 lane.
        dimmed(CHAIN_FORK, 2);
        dimmed(CHAIN_ROW_4, 2);
        dimmed(CHAIN_ROW_5, 2);
        // The markers sit at ` {mark} ` right after the lane column.
        let cols = RoadmapView {
            model: &model,
            cursor: None,
        }
        .columns(80, LANE_CAP_WIDE);
        let mark = |k| lane_cell(&buf, 120, 30, k, cols.lane + 1);
        assert_eq!(mark(1).symbol(), MARK_DEP);
        assert_eq!(mark(1).fg, Color::Cyan);
        for k in [CHAIN_ROW_4, CHAIN_ROW_5] {
            assert_eq!(mark(k).symbol(), MARK_UNBLOCKS);
            assert_eq!(mark(k).fg, Color::Magenta);
        }
        for i in 0..cols.lane + 6 {
            assert!(
                cell(CHAIN_ROW_3, i).modifier.contains(Modifier::REVERSED),
                "char {i}"
            );
        }
    }

    #[test]
    fn a_band_cursor_bolds_and_dims_nothing() {
        let model = chain();
        let buf = chain_buf(&model, &band_cursor(&model));
        let chars = chain_lane_chars(&model);
        assert!(chars.len() >= 10, "{chars:?}");
        for (k, i, link) in chars {
            let c = lane_cell(&buf, 120, 30, k, i);
            assert!(
                !c.modifier.intersects(Modifier::BOLD | Modifier::DIM),
                "row {k} char {i}: {c:?}"
            );
            let owner_done =
                link.is_some_and(|l| model.phases[l.owner].status == PhaseStatus::Done);
            let want = if owner_done {
                Color::DarkGray
            } else {
                LANE_PALETTE[(i / 2) % 5]
            };
            assert_eq!(c.fg, want, "row {k} char {i}");
        }
    }

    /// `(char, style)` for each cell of `line`.
    fn line_cells(line: &Line<'_>) -> Vec<(char, Style)> {
        line.spans
            .iter()
            .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
            .collect()
    }

    #[test]
    fn no_color_drops_the_lane_hue_but_keeps_status_and_weight() {
        let model = chain();
        let view = RoadmapView {
            model: &model,
            cursor: None,
        };
        let w = 80;
        let cols = view.columns(w, LANE_CAP_WIDE);
        let ctx = LaneCtx {
            selected: Some(2),
            palette: false,
        };
        assert_eq!(model.phases[2].id, "3");
        let (fork, _) = view.row_line(&model.rows[CHAIN_FORK], false, ctx, &cols, w);
        let fork = line_cells(&fork);
        for (i, &(c, style)) in fork.iter().take(5).enumerate() {
            assert_ne!(c, ' ', "{fork:?}");
            assert!(
                style.fg.is_none_or(|fg| !LANE_PALETTE.contains(&fg)),
                "char {i}: {style:?}"
            );
            if i != 2 {
                assert!(style.add_modifier.contains(Modifier::BOLD), "char {i}");
            }
        }
        let (row3, _) = view.row_line(&model.rows[CHAIN_ROW_3], false, ctx, &cols, w);
        let row3 = line_cells(&row3);
        let (glyph, style) = row3[0];
        assert_eq!(glyph.to_string(), PhaseStatus::Active.glyph());
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::BOLD));
        for &(c, style) in row3.iter().skip(1).take(cols.lane - 1) {
            assert!(
                style.fg.is_none_or(|fg| !LANE_PALETTE.contains(&fg)),
                "{c}: {style:?}"
            );
        }
        // The 2 -> 6 lane on the 3 row is off the chain and owned by a Done
        // phase: DIM, no hue.
        assert_eq!(row3[2].1.fg, None);
        assert!(row3[2].1.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn the_blocked_glyph_is_red_and_not_bold() {
        let blocked = glyph_style(PhaseStatus::Blocked);
        assert_eq!(blocked.fg, Some(Color::Red));
        assert!(!blocked.add_modifier.contains(Modifier::BOLD));
        assert_eq!(glyph_style(PhaseStatus::Done).fg, Some(Color::DarkGray));
        let active = glyph_style(PhaseStatus::Active);
        assert_eq!(active.fg, Some(Color::Yellow));
        assert!(active.add_modifier.contains(Modifier::BOLD));
        assert_eq!(glyph_style(PhaseStatus::Ready).fg, Some(Color::Green));
        assert_eq!(glyph_style(PhaseStatus::Unmerged).fg, Some(Color::Cyan));
    }

    #[test]
    fn the_overflow_column_draws_dark_gray() {
        let model = eight_lanes();
        let mut state = RoadmapViewState::default();
        let buf = render(&model, Some(&phase("1")), 80, 40, &mut state);
        let (list, _) = panes(Rect::new(0, 0, 80, 40));
        let mut seen = 0;
        for y in list.y..list.bottom() {
            for x in list.x..list.right() {
                let c = buf.cell((x, y)).expect("cell");
                if c.symbol() == LANE_OVERFLOW {
                    seen += 1;
                    assert_eq!(c.fg, Color::DarkGray, "({x}, {y})");
                }
            }
        }
        assert!(seen > 0, "{:#?}", rect_text(&buf, list));
    }
}
