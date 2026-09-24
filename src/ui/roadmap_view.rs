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
    BandKey, CursorTarget, ListRow, PhaseFacts, PhaseStatus, RoadmapModel, MARK_DEP, MARK_IMPLIED,
    MARK_SELECTED, MARK_UNBLOCKS, PARALLEL_SEP,
};
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
#[derive(Debug, Default, Clone, Copy)]
pub struct RoadmapViewState {
    /// In/out: the first visible model row.
    pub offset: usize,
    /// Out: how many model rows the list pane shows (for PageUp/PageDown).
    pub list_rows: u16,
}

/// The (list, detail) rects for `area`: side by side at
/// [`ROADMAP_SIDE_BY_SIDE_MIN_COLS`] and above, stacked below that. A
/// zero-size area yields two zero-size rects.
pub fn panes(area: Rect) -> (Rect, Rect) {
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
        let detail = DETAIL_STACKED_ROWS.min(area.height / 2);
        let list = area.height - detail;
        (
            Rect::new(area.x, area.y, area.width, list),
            Rect::new(area.x, area.y.saturating_add(list), area.width, detail),
        )
    }
}

/// The first visible row that keeps `row` inside a window of `visible` rows
/// starting near `offset`. (RED stub: returns `offset` unchanged.)
pub fn keep_visible(offset: usize, _row: usize, _visible: usize) -> usize {
    offset
}

// ---------------------------------------------------------------------------
// Char-based text helpers
// ---------------------------------------------------------------------------

/// Width of already-escaped text, by `char`.
fn cells(text: &str) -> usize {
    text.chars().count()
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

/// The status glyph's style (D-A04).
fn glyph_style(status: PhaseStatus) -> Style {
    match status {
        PhaseStatus::Done => dim(),
        PhaseStatus::Active => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PhaseStatus::Ready => Style::default().fg(Color::Green),
        PhaseStatus::Blocked => Style::default(),
    }
}

fn status_word(status: PhaseStatus) -> &'static str {
    match status {
        PhaseStatus::Done => "done",
        PhaseStatus::Active => "active",
        PhaseStatus::Ready => "ready",
        PhaseStatus::Blocked => "blocked",
    }
}

fn lanes_of(row: &ListRow) -> &str {
    match row {
        ListRow::ShippedSummary { lanes, .. }
        | ListRow::Band { lanes, .. }
        | ListRow::Connector { lanes }
        | ListRow::Phase { lanes, .. } => lanes,
    }
}

/// The lane cell as spans in `width` cells, the char at `glyph_at` (the
/// phase's own node) drawn in `glyph`.
fn lane_spans(
    lanes: &str,
    glyph_at: Option<usize>,
    glyph: Style,
    base: Style,
    width: usize,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = lanes.chars().collect();
    let mut spans = Vec::with_capacity(4);
    match glyph_at.filter(|&g| g < chars.len()) {
        Some(g) => {
            spans.push(Span::styled(chars[..g].iter().collect::<String>(), base));
            spans.push(Span::styled(chars[g].to_string(), glyph));
            spans.push(Span::styled(
                chars[g + 1..].iter().collect::<String>(),
                base,
            ));
        }
        None => spans.push(Span::styled(lanes.to_string(), base)),
    }
    let fill = width.saturating_sub(chars.len());
    spans.push(Span::styled(" ".repeat(fill), base));
    spans
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

    fn render(self, area: Rect, buf: &mut Buffer) {
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

/// Draw `items` top to bottom into `area`. When they do not fit, the hint is
/// dropped first (with the blank rows before it), then rows are cut from the
/// bottom.
fn draw_items(mut items: Vec<Item>, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }
    let total: u32 = items
        .iter()
        .map(|item| u32::from(item.height(area.width, area.height)))
        .sum();
    if total > u32::from(area.height) {
        items.retain(|item| !matches!(item, Item::Hint(_)));
        while items.last().is_some_and(Item::is_blank) {
            items.pop();
        }
    }
    let bottom = area.bottom();
    let mut y = area.y;
    for item in items {
        if y >= bottom {
            break;
        }
        let left = bottom - y;
        let h = item.height(area.width, left).min(left);
        if h == 0 {
            continue;
        }
        item.render(Rect::new(area.x, y, area.width, h), buf);
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
}

impl RoadmapView<'_> {
    fn phase(&self, node: usize) -> Option<&PhaseFacts> {
        self.model.phases.get(node)
    }

    fn columns(&self, w: usize) -> Cols {
        let lane = self
            .model
            .rows
            .iter()
            .map(|row| cells(lanes_of(row)))
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
        }
    }

    // --- list pane --------------------------------------------------------

    fn render_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut RoadmapViewState,
        cursor: Option<&CursorTarget>,
    ) {
        let block = pane_block()
            .title_top(Line::from(" Roadmap "))
            .title_top(Line::from(" v list ").right_aligned());
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.is_empty() {
            state.list_rows = 0;
            return;
        }
        let w = usize::from(inner.width);
        let cols = self.columns(w);
        let selected_phase = match cursor {
            Some(CursorTarget::Phase(key)) => self.model.phase_index(key),
            _ => None,
        };
        let selected_row = cursor.and_then(|c| self.model.row_of(c));

        let body = usize::from(inner.height).saturating_sub(2);
        state.list_rows = u16::try_from(body).unwrap_or(u16::MAX);
        state.offset = state.offset.min(self.model.rows.len().saturating_sub(1));

        let mut lines = vec![self.start_now_line(w), self.header_line(&cols, w)];
        for (i, row) in self
            .model
            .rows
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(body)
        {
            let selected = selected_row == Some(i);
            lines.push(self.row_line(row, selected, selected_phase, &cols, w));
        }
        Paragraph::new(lines).render(inner, buf);
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

    fn row_line(
        &self,
        row: &ListRow,
        selected: bool,
        selected_phase: Option<usize>,
        cols: &Cols,
        w: usize,
    ) -> Line<'static> {
        let spans = match row {
            ListRow::Phase { node, lane, lanes } => {
                self.phase_spans(*node, *lane, lanes, selected, selected_phase, cols)
            }
            ListRow::Band { label, lanes, .. } => {
                let mut spans =
                    lane_spans(lanes, None, Style::default(), Style::default(), cols.lane);
                spans.push(Span::styled(label.clone(), bold()));
                spans
            }
            ListRow::ShippedSummary { text, lanes, .. } => {
                let mut spans =
                    lane_spans(lanes, None, Style::default(), Style::default(), cols.lane);
                spans.push(Span::raw(text.clone()));
                spans
            }
            ListRow::Connector { lanes } => {
                lane_spans(lanes, None, Style::default(), Style::default(), cols.lane)
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
        fit(spans, w)
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

    /// `{lanes}{marker}{id}  {name}{plans}  {W<wave>}` (D-A01).
    fn phase_spans(
        &self,
        node: usize,
        lane: usize,
        lanes: &str,
        selected: bool,
        selected_phase: Option<usize>,
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
        let mut spans = lane_spans(
            lanes,
            lane.checked_mul(2),
            glyph_style(p.status),
            base,
            cols.lane,
        );
        let (mark, mark_style) = self.mark_for(node, selected, selected_phase);
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

    fn render_detail(&self, area: Rect, buf: &mut Buffer, cursor: Option<&CursorTarget>) {
        if area.is_empty() {
            return;
        }
        let phase = match cursor {
            Some(CursorTarget::Phase(key)) => {
                self.model.phase_index(key).and_then(|u| self.phase(u))
            }
            _ => None,
        };
        if let Some(p) = phase {
            let block = pane_block().title_top(Line::from(format!(" Phase {} ", p.id)));
            let inner = block.inner(area);
            block.render(area, buf);
            let items = self.phase_items(p, usize::from(inner.width));
            draw_items(items, inner, buf);
            return;
        }
        let band = match cursor {
            Some(CursorTarget::Band(BandKey::Named(_))) => cursor.and_then(|c| match c {
                CursorTarget::Band(key) => self.model.bands.iter().find(|b| b.key == *key),
                CursorTarget::Phase(_) => None,
            }),
            _ => None,
        };
        let block = pane_block();
        let inner = block.inner(area);
        block.render(area, buf);
        if let Some(b) = band {
            draw_items(
                vec![Item::Line(fit(
                    vec![Span::styled(b.label.clone(), bold())],
                    usize::from(inner.width),
                ))],
                inner,
                buf,
            );
        }
    }

    /// `{glyph} {word} · {d}/{t} plans  {badge} · planned (not a GSD phase)`.
    fn status_spans(p: &PhaseFacts) -> Vec<Span<'static>> {
        let mut spans = vec![
            Span::styled(p.status.glyph(), glyph_style(p.status)),
            Span::raw(format!(" {}", status_word(p.status))),
        ];
        spans.push(Span::raw(match p.plans {
            Some((d, t)) => format!("{DOT}{d}/{t} plans"),
            None => format!("{DOT}plans TBD"),
        }));
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
        let mut rows = if p.parallel.is_empty() {
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
                        _ => String::new(),
                    },
                    note: None,
                    dimmed: false,
                })
                .collect();
            self.entry_rows(&entries, avail)
        };
        if let Some(note) = Self::no_deps_note(p) {
            rows.push(vec![Span::styled(note, dim())]);
        }
        rows
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
        items.push(Item::blank());
        items.push(Item::Hint(fit(vec![Span::styled(HINT_WIDE, dim())], w)));
        items
    }
}

impl StatefulWidget for RoadmapView<'_> {
    type State = RoadmapViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            state.list_rows = 0;
            return;
        }
        let cursor = self.model.resolve_cursor(self.cursor);
        let (list, detail) = panes(area);
        self.render_list(list, buf, state, cursor.as_ref());
        self.render_detail(detail, buf, cursor.as_ref());
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
    type Spec<'a> = &'a [(&'a str, &'a str, &'a [&'a str], PhaseMarker, Option<usize>)];
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

    /// A row's cells after the list border and padding.
    fn body(row: &str) -> Vec<char> {
        row.chars().skip(2).collect()
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
        let spec: Vec<(&str, &str, &[&str], PhaseMarker, Option<usize>)> = (0..30)
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
}
