//! The Roadmap tab's left-to-right phase dependency graph (quick 260923-md1).
//!
//! Two halves, deliberately separated:
//!
//! * A **pure layout** ([`layout_graph`]) that turns an ordered list of phases
//!   and their DECLARED dependencies into a [`GraphLayout`]: rows of typed
//!   [`Segment`]s, note lines and a current-phase detail line. No ratatui type
//!   appears in it, so [`render_text`] can pin exact output in unit tests.
//! * A **widget** ([`RoadmapGraphWidget`]) that styles those same segments and
//!   draws them through `Paragraph::scroll`, never through raw buffer index
//!   math, so it clips to its own `Rect` at every size.
//!
//! Edges come only from `RoadmapPhase::depends_on`. Ids are matched through
//! [`phase_key`], never by raw string equality, so `07` and `7` are one phase.
//!
//! **Every phase id, dependency id, phase name and milestone label is
//! third-party text** read out of a project's `.planning/ROADMAP.md`. Each one
//! is escaped through `crate::text::render_for_terminal` (or
//! `Untrusted::shown()`) BEFORE it is measured, and only the escaped form is
//! ever stored in a [`GraphLayout`]. Widths are `chars().count()` of the
//! escaped text, the same deliberate IN-02/IN-03 deferral the box widget
//! records.

use crate::state_reader::phase_num::phase_key;
use crate::state_reader::roadmap_md::RoadmapPhase;
use crate::state_reader::{PhaseMarker, ProjectState};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use std::collections::{HashMap, HashSet, VecDeque};

/// One phase as the layout sees it. The strings are RAW third-party text, used
/// for logic (matching) and escaped before anything is stored for display.
#[derive(Debug, Clone, Copy)]
pub struct GraphNode<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub deps: &'a [String],
    /// Index into the `milestones` slice passed to [`layout_graph`].
    pub milestone: Option<usize>,
}

/// A milestone as the graph draws it. All three texts are ALREADY escaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneTag {
    pub label: String,
    pub short: String,
    pub name: String,
    pub active: bool,
}

/// One entry of the `Milestones:` header band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderItem {
    pub text: String,
    pub active: bool,
}

/// A run of cells in one graph row. Every text is already escaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A phase node, `idx` indexing the input node slice.
    Node { idx: usize, text: String },
    /// A repeated label on a reference row (drawn dim).
    Reference { idx: usize, text: String },
    /// Edge glyphs.
    Edge(String),
    /// Blank cells.
    Space(String),
    /// A row-end milestone label.
    Milestone(String),
}

impl Segment {
    /// The cells this segment occupies, as text.
    pub fn text(&self) -> &str {
        match self {
            Segment::Node { text, .. } | Segment::Reference { text, .. } => text,
            Segment::Edge(t) | Segment::Space(t) | Segment::Milestone(t) => t,
        }
    }
}

/// One drawn line of the graph body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphRow {
    pub segments: Vec<Segment>,
}

/// Where a node sits in the graph body: row index into [`GraphLayout::rows`],
/// starting column and label width in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    pub row: usize,
    pub col: usize,
    pub width: usize,
}

/// The whole laid-out graph: header band, body rows, notes and detail line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphLayout {
    pub header: Vec<HeaderItem>,
    pub rows: Vec<GraphRow>,
    pub notes: Vec<String>,
    pub detail: Option<String>,
    pub current: Option<CellSpan>,
}

/// Escape third-party text for a cell. The ONE composition, via
/// `render_for_terminal`; nothing in this file calls a narrower helper.
fn esc(raw: &str) -> String {
    String::from(crate::text::render_for_terminal(raw))
}

/// Width of already-escaped text, by `char` (IN-02/IN-03 deferral).
fn width_of(text: &str) -> usize {
    text.chars().count()
}

// ---------------------------------------------------------------------------
// Dependency resolution and cycle breaking
// ---------------------------------------------------------------------------

/// Per node, the in-graph parents (dependencies) in declared order.
type ParentLists = Vec<Vec<usize>>;
/// Per node, the escaped external dependency ids in declared order.
type ExternalLists = Vec<Vec<String>>;

/// Resolve every node's declared deps against the node index (D1).
///
/// Returns the in-graph parents, the external deps, and one self-cycle note per
/// node that names itself.
fn resolve_deps(
    nodes: &[GraphNode<'_>],
    labels: &[String],
) -> (ParentLists, ExternalLists, Vec<String>) {
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        index.entry(phase_key(node.id)).or_insert(i);
    }

    let mut parents: ParentLists = vec![Vec::new(); nodes.len()];
    let mut externals: ExternalLists = vec![Vec::new(); nodes.len()];
    let mut self_notes = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        let own = phase_key(node.id);
        let mut seen: HashSet<String> = HashSet::new();
        let mut self_cycle = false;
        for dep in node.deps {
            let key = phase_key(dep);
            if !seen.insert(key.clone()) {
                continue;
            }
            if key == own {
                self_cycle = true;
                continue;
            }
            match index.get(&key) {
                Some(&j) if j != i => parents[i].push(j),
                // A duplicate node whose key resolves to itself is a self-cycle
                // too; `j == i` cannot differ from `key == own`, but stay total.
                Some(_) => self_cycle = true,
                None => externals[i].push(esc(dep)),
            }
        }
        if self_cycle {
            self_notes.push(format!("dependency cycle: {} (edges ignored)", labels[i]));
        }
    }
    (parents, externals, self_notes)
}

/// Drop every edge that closes a cycle (D2, T-md1-02).
///
/// Iterative DFS with white/gray/black colouring and an explicit
/// `(node, next-dep cursor)` stack: never recursion on third-party depth. An
/// edge to a GRAY node is a back edge; it is dropped and a note names the
/// stack path from the gray target up to the current node.
fn break_cycles(parents: &mut ParentLists, labels: &[String], notes: &mut Vec<String>) {
    const WHITE: u8 = 0;
    const GRAY: u8 = 1;
    const BLACK: u8 = 2;
    let n = parents.len();
    let mut color = vec![WHITE; n];
    let mut dropped: HashSet<(usize, usize)> = HashSet::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for start in 0..n {
        if color[start] != WHITE {
            continue;
        }
        color[start] = GRAY;
        stack.push((start, 0));
        while let Some(top) = stack.last_mut() {
            let (u, k) = (top.0, top.1);
            top.1 += 1;
            match parents[u].get(k).copied() {
                Some(v) => match color[v] {
                    GRAY => {
                        dropped.insert((u, v));
                        let from = stack.iter().position(|&(w, _)| w == v).unwrap_or(0);
                        let path: Vec<&str> = stack[from..]
                            .iter()
                            .map(|&(w, _)| labels[w].as_str())
                            .collect();
                        notes.push(format!(
                            "dependency cycle: {} (edges ignored)",
                            path.join(", ")
                        ));
                    }
                    WHITE => {
                        color[v] = GRAY;
                        stack.push((v, 0));
                    }
                    _ => {}
                },
                None => {
                    color[u] = BLACK;
                    stack.pop();
                }
            }
        }
    }

    if !dropped.is_empty() {
        for (u, list) in parents.iter_mut().enumerate() {
            list.retain(|&v| !dropped.contains(&(u, v)));
        }
    }
}

/// Longest-path layers over the (now acyclic) parent lists, via Kahn's order.
fn longest_path_layers(parents: &ParentLists) -> Vec<usize> {
    let n = parents.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for (u, list) in parents.iter().enumerate() {
        indeg[u] = list.len();
        for &p in list {
            children[p].push(u);
        }
    }
    let mut layer = vec![0usize; n];
    let mut queue: VecDeque<usize> = (0..n).filter(|&u| indeg[u] == 0).collect();
    while let Some(u) = queue.pop_front() {
        for &c in &children[u] {
            layer[c] = layer[c].max(layer[u] + 1);
            indeg[c] -= 1;
            if indeg[c] == 0 {
                queue.push_back(c);
            }
        }
    }
    layer
}

// ---------------------------------------------------------------------------
// Row placement
// ---------------------------------------------------------------------------

/// One gap cell of one row: what occupies the space between layer `k` and
/// `k + 1` on that row.
#[derive(Debug, Clone, Default)]
struct GapCell {
    /// A horizontal edge `(src, tgt)` crossing this gap on this row.
    straight: Option<(usize, usize)>,
    /// Reserved by a chain's last node, so a later edge from it may use it.
    reserved: Option<usize>,
}

/// The logical occupancy of one node row: a node slot per layer and a gap
/// cell per adjacent layer pair.
#[derive(Debug, Clone)]
struct RowSlots {
    nodes: Vec<Option<usize>>,
    gaps: Vec<GapCell>,
}

impl RowSlots {
    fn new(layers: usize) -> Self {
        RowSlots {
            nodes: vec![None; layers],
            gaps: vec![GapCell::default(); layers.saturating_sub(1)],
        }
    }
}

/// Everything row placement needs to read.
struct Graph<'g> {
    layer: &'g [usize],
    parents: &'g ParentLists,
    primary_children: &'g [Vec<usize>],
    layers: usize,
}

/// Mutable placement state.
struct Placement {
    rows: Vec<RowSlots>,
    row_of: Vec<usize>,
    /// Edges drawn as reference rows, `(src, tgt)`.
    refs: Vec<(usize, usize)>,
}

impl Placement {
    fn fresh_row(&mut self, layers: usize) -> usize {
        self.rows.push(RowSlots::new(layers));
        self.rows.len() - 1
    }

    /// Place `start` and its first-primary-child chain on row `r`; queue every
    /// other primary child as a pending branch.
    fn place_chain(
        &mut self,
        g: &Graph<'_>,
        start: usize,
        r: usize,
        pending: &mut Vec<(usize, usize)>,
    ) {
        let mut cur = start;
        loop {
            let k = g.layer[cur];
            if let Some(slot) = self.rows[r].nodes.get_mut(k) {
                *slot = Some(cur);
            }
            self.row_of[cur] = r;
            let kids = &g.primary_children[cur];
            for &kid in kids.iter().skip(1) {
                pending.push((cur, kid));
            }
            match kids.first() {
                Some(&first) => {
                    if let Some(gap) = self.rows[r].gaps.get_mut(k) {
                        gap.straight = Some((cur, first));
                    }
                    cur = first;
                }
                None => {
                    if let Some(gap) = self.rows[r].gaps.get_mut(k) {
                        gap.reserved = Some(cur);
                    }
                    break;
                }
            }
        }
    }
}

/// Pick the next pending branch: deepest child layer first, ties to the lower
/// roadmap index.
fn take_next(pending: &mut Vec<(usize, usize)>, layer: &[usize]) -> Option<(usize, usize)> {
    let best = pending
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| layer[a.1].cmp(&layer[b.1]).then(b.1.cmp(&a.1)))
        .map(|(i, _)| i)?;
    Some(pending.remove(best))
}

/// Place every node on a row (D2 rule 5). Branches whose child cannot join
/// its parent's row go on a fresh bottom row plus a reference row.
fn place_rows(g: &Graph<'_>, n: usize) -> Placement {
    let mut pl = Placement {
        rows: Vec::new(),
        row_of: vec![0; n],
        refs: Vec::new(),
    };
    for root in (0..n).filter(|&u| g.layer[u] == 0) {
        let r = pl.fresh_row(g.layers);
        let mut pending = Vec::new();
        pl.place_chain(g, root, r, &mut pending);
        while let Some((p, c)) = take_next(&mut pending, g.layer) {
            let r = pl.fresh_row(g.layers);
            pl.place_chain(g, c, r, &mut pending);
            pl.refs.push((p, c));
        }
    }
    // Every secondary edge is a reference row.
    for (t, list) in g.parents.iter().enumerate() {
        for &s in list {
            let primary = g.primary_children[s].contains(&t);
            if !primary {
                pl.refs.push((s, t));
            }
        }
    }
    pl
}

// ---------------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------------

/// What one painted cell belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Node(usize),
    Reference(usize),
    Edge,
    Space,
}

type Cells = Vec<(char, Kind)>;

fn put(cells: &mut Cells, col: usize, ch: char, kind: Kind) {
    if cells.len() <= col {
        cells.resize(col + 1, (' ', Kind::Space));
    }
    cells[col] = (ch, kind);
}

fn put_text(cells: &mut Cells, col: usize, text: &str, kind: Kind) {
    for (i, ch) in text.chars().enumerate() {
        put(cells, col + i, ch, kind);
    }
}

/// Trim trailing blanks and merge runs of one kind into segments.
fn to_segments(mut cells: Cells) -> Vec<Segment> {
    while cells.last().is_some_and(|c| c.1 == Kind::Space) {
        cells.pop();
    }
    let mut out: Vec<Segment> = Vec::new();
    let mut i = 0;
    while i < cells.len() {
        let kind = cells[i].1;
        let mut j = i;
        let mut text = String::new();
        while j < cells.len() && cells[j].1 == kind {
            text.push(cells[j].0);
            j += 1;
        }
        out.push(match kind {
            Kind::Node(idx) => Segment::Node { idx, text },
            Kind::Reference(idx) => Segment::Reference { idx, text },
            Kind::Edge => Segment::Edge(text),
            Kind::Space => Segment::Space(text),
        });
        i = j;
    }
    out
}

/// Column geometry: per-layer label width and layer start column.
struct Columns {
    width: Vec<usize>,
    start: Vec<usize>,
}

impl Columns {
    /// `gap[k]` is the width of the gap after layer `k`.
    fn new(width: Vec<usize>, gap: &[usize]) -> Self {
        let mut start = Vec::with_capacity(width.len());
        let mut col = 0usize;
        for (k, w) in width.iter().enumerate() {
            start.push(col);
            col += w + gap.get(k).copied().unwrap_or(0);
        }
        Columns { width, start }
    }
}

/// Paint one node row into cells.
fn paint_node_row(row: &RowSlots, cols: &Columns, labels: &[String]) -> Cells {
    let mut cells: Cells = Vec::new();
    for (k, slot) in row.nodes.iter().enumerate() {
        let base = cols.start[k];
        let mut padded = false;
        if let Some(u) = *slot {
            let label = &labels[u];
            let lw = width_of(label);
            put_text(&mut cells, base, label, Kind::Node(u));
            let gap = row.gaps.get(k);
            let left = gap.is_some_and(|g| g.straight.is_some_and(|(s, _)| s == u));
            if left && lw < cols.width[k] {
                padded = true;
                put(&mut cells, base + lw, ' ', Kind::Space);
                for c in base + lw + 1..base + cols.width[k] {
                    put(&mut cells, c, '─', Kind::Edge);
                }
            }
        }
        let Some(gap) = row.gaps.get(k) else { continue };
        let g0 = base + cols.width[k];
        if gap.straight.is_some() {
            let lead = if padded { '─' } else { ' ' };
            put(
                &mut cells,
                g0,
                lead,
                if padded { Kind::Edge } else { Kind::Space },
            );
            put(&mut cells, g0 + 1, '─', Kind::Edge);
            put(&mut cells, g0 + 2, '►', Kind::Edge);
            put(&mut cells, g0 + 3, ' ', Kind::Space);
        }
    }
    cells
}

/// Paint one reference row: `<src> ───► <tgt>`, both labels at their own
/// layer columns.
fn paint_reference_row(
    s: usize,
    t: usize,
    layer: &[usize],
    cols: &Columns,
    labels: &[String],
) -> Cells {
    let mut cells: Cells = Vec::new();
    let src_col = cols.start[layer[s]];
    let tgt_col = cols.start[layer[t]];
    let src_end = src_col + width_of(&labels[s]);
    put_text(&mut cells, src_col, &labels[s], Kind::Reference(s));
    put(&mut cells, src_end, ' ', Kind::Space);
    let arrow = tgt_col.saturating_sub(2).max(src_end + 1);
    for c in src_end + 1..arrow {
        put(&mut cells, c, '─', Kind::Edge);
    }
    put(&mut cells, arrow, '►', Kind::Edge);
    put(&mut cells, arrow + 1, ' ', Kind::Space);
    put_text(&mut cells, arrow + 2, &labels[t], Kind::Reference(t));
    cells
}

// ---------------------------------------------------------------------------
// The layout entry point
// ---------------------------------------------------------------------------

/// Lay out `nodes` as a left-to-right dependency graph (D1, D2).
///
/// Pure: no ratatui types. `current` is the index of the current phase, which
/// gets a detail line and a [`CellSpan`].
pub fn layout_graph(
    nodes: &[GraphNode<'_>],
    milestones: &[MilestoneTag],
    current: Option<usize>,
) -> GraphLayout {
    let _ = milestones;
    let n = nodes.len();
    let labels: Vec<String> = nodes.iter().map(|node| esc(node.id)).collect();

    let (mut parents, externals, self_notes) = resolve_deps(nodes, &labels);
    let mut cycle_notes = self_notes;
    break_cycles(&mut parents, &labels, &mut cycle_notes);
    let layer = longest_path_layers(&parents);
    let layers = layer.iter().copied().max().map_or(0, |m| m + 1);

    // Primary parent: the first-declared remaining dep one layer back.
    let mut primary_children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for u in 0..n {
        if layer[u] == 0 {
            continue;
        }
        if let Some(&p) = parents[u].iter().find(|&&p| layer[p] + 1 == layer[u]) {
            primary_children[p].push(u);
        }
    }

    let graph = Graph {
        layer: &layer,
        parents: &parents,
        primary_children: &primary_children,
        layers,
    };
    let mut placement = place_rows(&graph, n);

    // Columns.
    let mut width = vec![1usize; layers];
    for u in 0..n {
        width[layer[u]] = width[layer[u]].max(width_of(&labels[u]));
    }
    let gap = vec![4usize; layers.saturating_sub(1)];
    let cols = Columns::new(width, &gap);

    let mut rows: Vec<GraphRow> = placement
        .rows
        .iter()
        .map(|row| GraphRow {
            segments: to_segments(paint_node_row(row, &cols, &labels)),
        })
        .collect();

    // Reference rows: by target roadmap index, then source declaration order.
    placement.refs.sort_by_key(|&(s, t)| {
        let pos = parents[t]
            .iter()
            .position(|&p| p == s)
            .unwrap_or(usize::MAX);
        (t, pos)
    });
    for &(s, t) in &placement.refs {
        rows.push(GraphRow {
            segments: to_segments(paint_reference_row(s, t, &layer, &cols, &labels)),
        });
    }

    // Notes: one external line, then the cycle notes.
    let mut notes = Vec::new();
    let groups: Vec<String> = externals
        .iter()
        .enumerate()
        .filter(|(_, deps)| !deps.is_empty())
        .map(|(i, deps)| format!("{} ◄ {}", labels[i], deps.join(", ")))
        .collect();
    if !groups.is_empty() {
        notes.push(format!("external deps: {}", groups.join("; ")));
    }
    notes.extend(cycle_notes);

    let (detail, current_span) = match current.filter(|&i| i < n) {
        Some(i) => (
            Some(format!("▶ P{}: {}", labels[i], esc(nodes[i].name))),
            Some(CellSpan {
                row: placement.row_of[i],
                col: cols.start[layer[i]],
                width: width_of(&labels[i]),
            }),
        ),
        None => (None, None),
    };

    GraphLayout {
        header: Vec::new(),
        rows,
        notes,
        detail,
        current: current_span,
    }
}

/// The layout as plain text lines: header, rows, notes, detail (in order).
pub fn render_text(layout: &GraphLayout) -> Vec<String> {
    let mut out = Vec::new();
    if !layout.header.is_empty() {
        let items: Vec<String> = layout
            .header
            .iter()
            .map(|h| format!("{} {}", if h.active { '◆' } else { '◇' }, h.text))
            .collect();
        out.push(format!("Milestones: {}", items.join("  ")));
    }
    for row in &layout.rows {
        out.push(row.segments.iter().map(Segment::text).collect());
    }
    out.extend(layout.notes.iter().cloned());
    if let Some(detail) = &layout.detail {
        out.push(detail.clone());
    }
    out
}

/// Split `area` into (header band, graph body, footer) for `layout`.
pub fn split_areas(layout: &GraphLayout, area: Rect) -> [Rect; 3] {
    let header_h: u16 = if layout.header.is_empty() { 0 } else { 1 };
    let footer_lines = layout.notes.len() + usize::from(layout.detail.is_some());
    let footer_h = u16::try_from(footer_lines).unwrap_or(u16::MAX);
    Layout::vertical([
        Constraint::Length(header_h),
        Constraint::Min(0),
        Constraint::Length(footer_h),
    ])
    .areas(area)
}

/// Graph nodes for a roadmap's phases, in roadmap order, without milestones.
pub fn nodes_from_phases(phases: &[RoadmapPhase]) -> Vec<GraphNode<'_>> {
    phases
        .iter()
        .map(|p| GraphNode {
            id: &p.number,
            name: &p.name,
            deps: &p.depends_on,
            milestone: None,
        })
        .collect()
}

/// One [`PhaseMarker`] per phase, the same decision the box widget makes.
pub fn phase_markers(state: &ProjectState) -> Vec<PhaseMarker> {
    let active = state.active_phase_number();
    state
        .phases
        .iter()
        .map(|p| PhaseMarker::decide(&p.number, p.completed, &state.phase_disk_statuses, &active))
        .collect()
}

/// The graph for a project state. `current` is the first `Current` marker.
pub fn layout_for_state(state: &ProjectState, markers: &[PhaseMarker]) -> GraphLayout {
    let nodes = nodes_from_phases(&state.phases);
    let current = markers.iter().position(|m| *m == PhaseMarker::Current);
    layout_graph(&nodes, &[], current)
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

/// Draws a [`GraphLayout`]: header band, the graph body (vertically scrolled,
/// horizontally offset to keep the current node in view) and the footer.
pub struct RoadmapGraphWidget<'a> {
    pub layout: &'a GraphLayout,
    pub markers: &'a [PhaseMarker],
    pub scroll_offset: u16,
}

impl RoadmapGraphWidget<'_> {
    fn segment_style(&self, segment: &Segment) -> Style {
        match segment {
            Segment::Node { idx, .. } => match self.markers.get(*idx).copied() {
                Some(PhaseMarker::Done) => Style::default().fg(Color::DarkGray),
                Some(PhaseMarker::Current) => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                _ => Style::default(),
            },
            Segment::Reference { .. } => Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            Segment::Milestone(_) => Style::default().fg(Color::Cyan),
            Segment::Edge(_) | Segment::Space(_) => Style::default(),
        }
    }
}

impl Widget for RoadmapGraphWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let [header, graph, footer] = split_areas(self.layout, area);

        if !self.layout.header.is_empty() && header.height > 0 && header.width > 0 {
            let mut spans = vec![Span::styled(
                "Milestones: ",
                Style::default().add_modifier(Modifier::BOLD),
            )];
            for (i, item) in self.layout.header.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                let (glyph, style) = if item.active {
                    (
                        '◆',
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ('◇', Style::default())
                };
                spans.push(Span::styled(format!("{} {}", glyph, item.text), style));
            }
            Paragraph::new(Line::from(spans)).render(header, buf);
        }

        if graph.height > 0 && graph.width > 0 {
            let lines: Vec<Line> = self
                .layout
                .rows
                .iter()
                .map(|row| {
                    Line::from(
                        row.segments
                            .iter()
                            .map(|s| Span::styled(s.text().to_string(), self.segment_style(s)))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let h = self.layout.current.map_or(0, |span| {
                (span.col + span.width + 2).saturating_sub(usize::from(graph.width))
            });
            let h = u16::try_from(h).unwrap_or(u16::MAX);
            Paragraph::new(lines)
                .scroll((self.scroll_offset, h))
                .render(graph, buf);
        }

        if footer.height > 0 && footer.width > 0 {
            let mut lines: Vec<Line> = self
                .layout
                .notes
                .iter()
                .map(|n| {
                    Line::from(Span::styled(
                        n.clone(),
                        Style::default().fg(Color::DarkGray),
                    ))
                })
                .collect();
            if let Some(detail) = &self.layout.detail {
                lines.push(Line::from(Span::styled(
                    detail.clone(),
                    Style::default().fg(Color::Yellow),
                )));
            }
            Paragraph::new(lines).render(footer, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test phase list: `(id, name, deps)`.
    type Spec<'a> = &'a [(&'a str, &'a str, &'a [&'a str])];

    fn owned_deps(spec: Spec<'_>) -> Vec<Vec<String>> {
        spec.iter()
            .map(|(_, _, deps)| deps.iter().map(|d| d.to_string()).collect())
            .collect()
    }

    fn lay(spec: Spec<'_>, current: Option<usize>) -> GraphLayout {
        let deps = owned_deps(spec);
        let nodes: Vec<GraphNode<'_>> = spec
            .iter()
            .zip(&deps)
            .map(|((id, name, _), deps)| GraphNode {
                id,
                name,
                deps,
                milestone: None,
            })
            .collect();
        layout_graph(&nodes, &[], current)
    }

    fn text(spec: Spec<'_>, current: Option<usize>) -> Vec<String> {
        render_text(&lay(spec, current))
    }

    #[test]
    fn roadmap_graph_o1_chain() {
        assert_eq!(
            text(
                &[("1", "", &[]), ("2", "", &["1"]), ("3", "", &["2"])],
                None
            ),
            vec!["1 ─► 2 ─► 3"]
        );
    }

    #[test]
    fn roadmap_graph_o2_roots_only() {
        assert_eq!(
            text(&[("1", "", &[]), ("2", "", &[]), ("3", "", &[])], None),
            vec!["1", "2", "3"]
        );
    }

    #[test]
    fn roadmap_graph_o3_cycle_terminates_with_note() {
        assert_eq!(
            text(
                &[("1", "", &["3"]), ("2", "", &["1"]), ("3", "", &["2"])],
                None
            ),
            vec!["2 ─► 3 ─► 1", "dependency cycle: 1, 3, 2 (edges ignored)"]
        );
    }

    #[test]
    fn roadmap_graph_o3b_self_dependency() {
        assert_eq!(
            text(&[("1", "", &["1"])], None),
            vec!["1", "dependency cycle: 1 (edges ignored)"]
        );
    }

    #[test]
    fn roadmap_graph_o4_external_dep() {
        assert_eq!(
            text(&[("14", "", &["7"]), ("15", "", &["14"])], None),
            vec!["14 ─► 15", "external deps: 14 ◄ 7"]
        );
    }

    #[test]
    fn roadmap_graph_o4b_padded_and_decimal_ids() {
        assert_eq!(
            text(
                &[("7", "", &[]), ("7.1", "", &["07"]), ("8", "", &["7.1"])],
                None
            ),
            vec!["7 ─► 7.1 ─► 8"]
        );
    }

    #[test]
    fn roadmap_graph_o5_skip_layer_edge_is_a_reference_row() {
        assert_eq!(
            text(
                &[("1", "", &[]), ("2", "", &["1"]), ("3", "", &["2", "1"])],
                None
            ),
            vec!["1 ─► 2 ─► 3", "1 ──────► 3"]
        );
    }

    #[test]
    fn roadmap_graph_o6_current_phase_detail_line() {
        assert_eq!(
            text(
                &[
                    ("1", "Alpha", &[]),
                    ("2", "Build", &["1"]),
                    ("3", "Ship", &["2"])
                ],
                Some(1)
            ),
            vec!["1 ─► 2 ─► 3", "▶ P2: Build"]
        );
    }

    #[test]
    fn roadmap_graph_decimal_node_is_ordinary() {
        assert_eq!(
            text(&[("21", "", &[]), ("21.1", "", &["21"])], None),
            vec!["21 ─► 21.1"]
        );
    }

    #[test]
    fn roadmap_graph_escapes_the_detail_line_and_external_deps() {
        let name = "Evil\u{1b}[31m name \u{202E}rtl";
        let bad_dep = "9\u{1b}[2J";
        let lines = text(&[("1", name, &[bad_dep])], Some(0));
        let detail = lines.last().unwrap();
        assert_eq!(
            *detail,
            format!("▶ P1: {}", crate::text::render_for_terminal(name))
        );
        for line in &lines {
            assert!(!line.contains('\u{1b}'), "raw ESC reached {line:?}");
            assert!(!line.contains('\u{202E}'), "raw U+202E reached {line:?}");
        }
        let note = format!(
            "external deps: 1 ◄ {}",
            crate::text::render_for_terminal(bad_dep)
        );
        assert!(lines.contains(&note), "{lines:?}");
    }

    fn o6() -> GraphLayout {
        lay(
            &[
                ("1", "Alpha", &[]),
                ("2", "Build", &["1"]),
                ("3", "Ship", &["2"]),
            ],
            Some(1),
        )
    }

    fn buffer_rows(buf: &Buffer) -> Vec<String> {
        let area = buf.area;
        (area.y..area.y + area.height)
            .map(|y| {
                (area.x..area.x + area.width)
                    .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()).to_string())
                    .collect::<String>()
            })
            .collect()
    }

    fn assert_no_panic_at_tiny_sizes(layout: &GraphLayout) {
        let markers = vec![PhaseMarker::Future; 64];
        for (w, h) in [(1u16, 1u16), (5, 3), (10, 3), (20, 5)] {
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(area);
            RoadmapGraphWidget {
                layout,
                markers: &markers,
                scroll_offset: 3,
            }
            .render(area, &mut buf);
        }
        // An offset Rect inside a larger buffer: nothing outside it changes.
        let full = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(full);
        for y in 0..10 {
            buf.set_string(0, y, "x".repeat(20), Style::default());
        }
        let before = buf.clone();
        let inner = Rect::new(3, 2, 5, 3);
        RoadmapGraphWidget {
            layout,
            markers: &markers,
            scroll_offset: 0,
        }
        .render(inner, &mut buf);
        for y in 0..10u16 {
            for x in 0..20u16 {
                let inside = (3..8).contains(&x) && (2..5).contains(&y);
                if !inside {
                    assert_eq!(buf.cell((x, y)), before.cell((x, y)), "bled at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn roadmap_graph_never_panics_or_bleeds_at_tiny_sizes() {
        assert_no_panic_at_tiny_sizes(&o6());
        assert_no_panic_at_tiny_sizes(&GraphLayout::default());
    }

    #[test]
    fn roadmap_graph_widget_text_matches_render_text() {
        let layout = o6();
        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);
        let markers = [PhaseMarker::Done, PhaseMarker::Current, PhaseMarker::Future];
        RoadmapGraphWidget {
            layout: &layout,
            markers: &markers,
            scroll_offset: 0,
        }
        .render(area, &mut buf);
        let drawn: Vec<String> = buffer_rows(&buf)
            .into_iter()
            .map(|r| r.trim_end().to_string())
            .filter(|r| !r.is_empty())
            .collect();
        assert_eq!(drawn, render_text(&layout));
    }

    #[test]
    fn roadmap_graph_auto_offset_keeps_the_current_node_visible() {
        let ids: Vec<String> = (1..=30).map(|i| i.to_string()).collect();
        let deps: Vec<Vec<String>> = (1..=30)
            .map(|i| {
                if i == 1 {
                    Vec::new()
                } else {
                    vec![(i - 1).to_string()]
                }
            })
            .collect();
        let nodes: Vec<GraphNode<'_>> = ids
            .iter()
            .zip(&deps)
            .map(|(id, deps)| GraphNode {
                id,
                name: "x",
                deps,
                milestone: None,
            })
            .collect();
        let layout = layout_graph(&nodes, &[], Some(29));
        let area = Rect::new(0, 0, 40, 4);
        let mut buf = Buffer::empty(area);
        let markers = vec![PhaseMarker::Future; 30];
        RoadmapGraphWidget {
            layout: &layout,
            markers: &markers,
            scroll_offset: 0,
        }
        .render(area, &mut buf);
        let row0 = &buffer_rows(&buf)[0];
        assert!(row0.contains("30"), "{row0:?}");
        assert!(!row0.starts_with("1 ─►"), "{row0:?}");
    }
}
