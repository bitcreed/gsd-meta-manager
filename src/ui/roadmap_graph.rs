//! The Roadmap tab's phase dependency model (phase 24).
//!
//! An ordered list of phases ([`ListNode`]) and milestone bands
//! ([`BandInput`]) becomes a [`RoadmapModel`] through [`layout_list`]: one
//! [`ListRow::Phase`] per phase (no phase ever appears in two rows), one band
//! row per milestone, one collapsed shipped-summary row, and connector rows
//! drawn with git-log-style lanes. Dependencies are transitively reduced
//! first; each dropped edge is kept as an `implied` pair `(dep, via)`.
//! Per-phase facts (needs, unblocks, parallel, status, wave) and cursor
//! navigation are pure functions on the model, so exact text ([`lane_text`])
//! and every Roadmap key are pinned without a terminal. No ratatui type
//! appears here; the widget is `ui::roadmap_view`, and the one
//! `ProjectState` adapter is `ui::screens::detail::roadmap_model_for`.
//!
//! The dependency pipeline (D-A15) is `resolve_deps` → `break_cycles` →
//! `longest_path_layers` (a wave is `layer + 1`); a broken cycle becomes one
//! bounded note ([`cycle_note`]). Edges come only from declared
//! dependencies. Ids are matched through [`phase_key`], never by raw string
//! equality, so `07` and `7` are one phase. (The left-to-right graph this
//! replaced, quick 260923-md1, was removed in plan 24-06: its reference rows
//! drew a phase twice.)
//!
//! **Every phase id, dependency id, phase name, goal and milestone label is
//! third-party text** read out of a project's `.planning/ROADMAP.md`. Each one
//! is escaped through `crate::text::render_for_terminal` (or
//! `Untrusted::shown()`) BEFORE it is measured, and only the escaped form is
//! ever stored for display in a [`RoadmapModel`]. The raw text is used only
//! for matching ([`phase_key`], [`BandKey`]), never drawn.

use crate::state_reader::phase_num::phase_key;
use crate::state_reader::roadmap_md;
use crate::state_reader::PhaseMarker;
use std::collections::{HashMap, HashSet, VecDeque};

/// Escape third-party text for a cell. The ONE composition, via
/// `render_for_terminal`; nothing in this file calls a narrower helper.
fn esc(raw: &str) -> String {
    String::from(crate::text::render_for_terminal(raw))
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
/// `nodes` is `(raw id, raw declared deps)` per node, in list order. Returns
/// the in-graph parents,
/// the external deps, and one self-cycle path (the node's escaped label) per
/// node that names itself.
fn resolve_deps(
    nodes: &[(&str, &[String])],
    labels: &[String],
) -> (ParentLists, ExternalLists, Vec<String>) {
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, &(id, _)) in nodes.iter().enumerate() {
        index.entry(phase_key(id)).or_insert(i);
    }

    let mut parents: ParentLists = vec![Vec::new(); nodes.len()];
    let mut externals: ExternalLists = vec![Vec::new(); nodes.len()];
    let mut self_cycles = Vec::new();
    for (i, &(id, deps)) in nodes.iter().enumerate() {
        let own = phase_key(id);
        let mut seen: HashSet<String> = HashSet::new();
        let mut self_cycle = false;
        for dep in deps {
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
            self_cycles.push(labels[i].clone());
        }
    }
    (parents, externals, self_cycles)
}

/// Drop every edge that closes a cycle (D2, T-md1-02).
///
/// Iterative DFS with white/gray/black colouring and an explicit
/// `(node, next-dep cursor)` stack: never recursion on third-party depth. An
/// edge to a GRAY node is a back edge; it is dropped and `cycles` gains the
/// stack path from the gray target up to the current node (escaped labels,
/// comma-joined). [`cycle_note`] folds the paths into one footer line.
fn break_cycles(parents: &mut ParentLists, labels: &[String], cycles: &mut Vec<String>) {
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
                        cycles.push(path.join(", "));
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

/// Most cycle paths [`cycle_note`] spells out before a `+K more` suffix.
const MAX_CYCLES_SHOWN: usize = 3;

/// ONE footer line for every broken cycle (WR-01), so a malformed roadmap with
/// many back edges cannot grow the footer over the graph body. A single cycle
/// keeps the singular form; several collapse into a count, the first
/// [`MAX_CYCLES_SHOWN`] paths and a `+K more` suffix.
fn cycle_note(cycles: &[String]) -> Option<String> {
    match cycles {
        [] => None,
        [only] => Some(format!("dependency cycle: {only} (edges ignored)")),
        _ => {
            let shown: Vec<&str> = cycles
                .iter()
                .take(MAX_CYCLES_SHOWN)
                .map(String::as_str)
                .collect();
            let more = cycles.len() - shown.len();
            let suffix = if more > 0 {
                format!("; +{more} more")
            } else {
                String::new()
            };
            Some(format!(
                "dependency cycles: {} (edges ignored): {}{}",
                cycles.len(),
                shown.join("; "),
                suffix
            ))
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
// The vertical list model (phase 24)
// ---------------------------------------------------------------------------

/// Status glyph: done (drawn dim).
pub const GLYPH_DONE: &str = "\u{25CF}";
/// Status glyph: the active phase (yellow, bold).
pub const GLYPH_ACTIVE: &str = "\u{25C9}";
/// Status glyph: ready — not done, every in-graph dependency done (green).
pub const GLYPH_READY: &str = "\u{25CB}";
/// Status glyph: blocked on an unfinished dependency.
pub const GLYPH_BLOCKED: &str = "\u{25CC}";
/// Status glyph: every remaining plan finished on a worktree, not yet merged
/// (cyan, not dim; quick 260929-szq). The one glyph the whole UI shares.
pub const GLYPH_UNMERGED: &str = crate::agents::unmerged::GLYPH_UNMERGED;
/// Marker of the selected row (the row is also drawn reversed).
pub const MARK_SELECTED: &str = "\u{25B6}";
/// Marker of a dependency of the selection (cyan).
pub const MARK_DEP: &str = "\u{2191}";
/// Marker of a phase the selection unblocks (magenta).
pub const MARK_UNBLOCKS: &str = "\u{2193}";
/// Marker of an implied (transitively reduced) dependency (dim).
pub const MARK_IMPLIED: &str = "\u{00B7}";
/// An unfolded band.
pub const BAND_OPEN: &str = "\u{25BE}";
/// A folded band.
pub const BAND_FOLDED: &str = "\u{25B8}";
/// A band row's trailing fill.
pub const BAND_FILL: &str = "\u{2501}";
/// Separator between phases that can run in parallel (the Start-now line).
pub const PARALLEL_SEP: &str = "\u{2551}";
/// Lane: a pass-through vertical.
pub const LANE_VERTICAL: &str = "\u{2502}";
/// Lane: a horizontal run.
pub const LANE_HORIZONTAL: &str = "\u{2500}";
/// Lane junction `├`: a node's lane forking or merging to the right.
pub const LANE_TEE_RIGHT: &str = "\u{251C}";
/// Lane junction `┤`: a node's lane forking to the left.
pub const LANE_TEE_LEFT: &str = "\u{2524}";
/// Lane junction `┬`: an intermediate forked lane.
pub const LANE_TEE_DOWN: &str = "\u{252C}";
/// Lane junction `┴`: an intermediate merged lane.
pub const LANE_TEE_UP: &str = "\u{2534}";
/// Lane junction `╮`: the far right end of a fork. The four lane corners are
/// rounded, the git-graph style shared with the Git tab's commit graph
/// (quick 261006-ujx).
pub const LANE_DOWN_LEFT: &str = "\u{256E}";
/// Lane junction `╭`: the far left end of a fork.
pub const LANE_DOWN_RIGHT: &str = "\u{256D}";
/// Lane junction `╯`: the far right end of a merge.
pub const LANE_UP_LEFT: &str = "\u{256F}";
/// Lane junction `╰`: a merge ending to the right (not produced by the
/// assigner, which always merges into the lowest lane; kept for totality).
pub const LANE_UP_RIGHT: &str = "\u{2570}";
/// Lane junction `┼`: a horizontal crossing an unrelated active lane.
pub const LANE_CROSS: &str = "\u{253C}";

/// Width of the lane column in [`lane_text`].
const LANE_TEXT_COLUMN: usize = 10;

/// One phase as the list model sees it. `id`, `name` and `deps` are RAW
/// third-party text, used for matching and escaped before anything is stored.
#[derive(Debug, Clone)]
pub struct ListNode<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub deps: &'a [String],
    /// Index into [`ListInput::bands`]; `None` (or out of range) is band-less.
    pub band: Option<usize>,
    /// The caller's done/current decision (D-B12).
    pub marker: PhaseMarker,
    /// Implementation finished, independent of the current marker; from
    /// `state_reader::phase_is_done`. The band and model `done` counts read
    /// this rather than the marker, because a phase that is both finished and
    /// current draws `*` yet still counts as done — which is what keeps the
    /// active band's count equal to the Roadmap header's
    /// (`ProjectState::phase_progress`).
    pub done: bool,
    /// Plans done / total, when known.
    pub plans: Option<(u32, u32)>,
    pub goal: Option<&'a crate::text::Untrusted>,
    /// A planned (placeholder, non-GSD) phase.
    pub planned: bool,
    /// Reader-generated stage text carrying its own brackets
    /// (e.g. `[Executing 16/18] [verified]`).
    pub badge: Option<String>,
}

/// One milestone band as the list model sees it.
#[derive(Debug, Clone)]
pub struct BandInput {
    pub label: crate::text::Untrusted,
    pub shipped: bool,
    /// The phase count the roadmap declares for this milestone (it may list
    /// fewer phases than it shipped).
    pub declared_phases: u32,
}

/// Everything [`layout_list`] reads: the phases in list order and the bands.
#[derive(Debug, Clone, Default)]
pub struct ListInput<'a> {
    pub nodes: Vec<ListNode<'a>>,
    pub bands: Vec<BandInput>,
}

/// A band's identity for folding and the cursor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BandKey {
    /// The one collapsed row standing for every shipped milestone.
    Shipped,
    /// A milestone band: the lower-cased trimmed RAW label (logic only,
    /// never drawn).
    Named(String),
}

/// What the Roadmap cursor rests on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CursorTarget {
    /// A phase, by `phase_key`.
    Phase(String),
    /// A band row or the shipped-summary row.
    Band(BandKey),
}

/// A phase's status (D-A04), decided from the caller's [`PhaseMarker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStatus {
    Done,
    Active,
    Ready,
    Blocked,
    /// Not done in main; every remaining plan finished on a worktree,
    /// unmerged — never satisfies dependents (quick 260929-szq, I-11). Set
    /// only by the Roadmap's post-pass over an agent view, never by
    /// [`layout_list`].
    Unmerged,
}

impl PhaseStatus {
    /// The status glyph (`●` `◉` `○` `◌` `◐`).
    pub fn glyph(self) -> &'static str {
        match self {
            PhaseStatus::Done => GLYPH_DONE,
            PhaseStatus::Active => GLYPH_ACTIVE,
            PhaseStatus::Ready => GLYPH_READY,
            PhaseStatus::Blocked => GLYPH_BLOCKED,
            PhaseStatus::Unmerged => GLYPH_UNMERGED,
        }
    }
}

/// One visible row of the list. Every string is already escaped; `lanes` is
/// the lane column, two cells per lane, trailing blanks trimmed.
///
/// `links` carries one entry per char of `lanes`: the [`LaneLink`] edge that
/// char draws, `None` on blanks and on a phase row's own node char. It decides
/// only done-dimming and selection weight; a lane's hue is never derived from
/// it (hue is `char_index / 2`, quick 261006-ujx).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListRow {
    /// Every shipped milestone, collapsed into one row (D-A07).
    ShippedSummary {
        /// First and last shipped short ids (`v1.0 … v1.4`).
        text: String,
        milestones: usize,
        phases: u32,
        folded: bool,
        lanes: String,
        links: Vec<Option<LaneLink>>,
    },
    /// A milestone band header.
    Band {
        band: usize,
        key: BandKey,
        label: String,
        short: String,
        done: usize,
        total: usize,
        folded: bool,
        lanes: String,
        links: Vec<Option<LaneLink>>,
    },
    /// A fork or merge between lanes.
    Connector {
        lanes: String,
        links: Vec<Option<LaneLink>>,
    },
    /// A phase; `lanes` carries its status glyph at cell `2 * lane`.
    Phase {
        node: usize,
        lane: usize,
        lanes: String,
        links: Vec<Option<LaneLink>>,
    },
}

/// Everything the Roadmap knows about one phase. Display strings are escaped;
/// indices point into [`RoadmapModel::phases`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseFacts {
    /// `phase_key` of the raw id (logic only).
    pub key: String,
    pub id: String,
    pub name: String,
    pub goal: Option<String>,
    pub band: Option<usize>,
    pub wave: usize,
    pub status: PhaseStatus,
    pub planned: bool,
    pub plans: Option<(u32, u32)>,
    /// Plans of this phase finished on a worktree, unmerged (either state);
    /// 0 from [`layout_list`], filled by the Roadmap's agent-view post-pass.
    pub unmerged: u32,
    pub badge: Option<String>,
    /// Transitively reduced dependencies, declared order.
    pub needs: Vec<usize>,
    /// Dropped dependencies: `(dep, via)`.
    pub implied: Vec<(usize, usize)>,
    /// Dependencies that name no listed phase (escaped).
    pub external: Vec<String>,
    /// Phases whose reduced dependencies include this one, list order.
    pub unblocks: Vec<usize>,
    /// The other phases of the same wave, list order (D-A02).
    pub parallel: Vec<usize>,
    /// No needs, no implied and no external dependencies.
    pub no_deps: bool,
    /// `no_deps`, and no phase depends on this one either.
    pub no_edges: bool,
    /// The last phase row of its band.
    pub last_in_band: bool,
}

/// Everything the Roadmap knows about one band (escaped display strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandFacts {
    pub key: BandKey,
    pub label: String,
    pub short: String,
    pub shipped: bool,
    pub done: usize,
    pub total: usize,
    pub declared_phases: u32,
}

/// The laid-out Roadmap list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoadmapModel {
    /// Visible rows only (folded rows dropped).
    pub rows: Vec<ListRow>,
    /// One per input node, input order.
    pub phases: Vec<PhaseFacts>,
    /// One per input band, input order.
    pub bands: Vec<BandFacts>,
    /// Active and ready phases, list order (D-A06).
    pub start_now: Vec<usize>,
    pub max_wave: usize,
    pub done: usize,
    pub total: usize,
    /// At most one bounded cycle line (WR-01).
    pub notes: Vec<String>,
}

/// Whether `key` is folded: only the shipped summary is folded by default,
/// and a toggle flips the default.
pub fn is_folded(key: &BandKey, fold_toggles: &HashSet<BandKey>) -> bool {
    (*key == BandKey::Shipped) != fold_toggles.contains(key)
}

/// Drop every declared parent that another declared parent already implies
/// (D-A08, RESEARCH Pattern 1), over ACYCLIC parents.
///
/// Returns the reduced parents (declared order kept) and, per node, the
/// dropped edges as `(implied parent, via)`. The witness `via` is the first
/// KEPT parent whose ancestry contains the dropped one, else the first other
/// declared parent that does. Ancestor sets are built in layer order (a
/// parent always sits on a lower layer), iteratively: O(n·e), no recursion.
fn transitive_reduction(
    parents: &ParentLists,
    layer: &[usize],
) -> (ParentLists, Vec<Vec<(usize, usize)>>) {
    let n = parents.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&u| layer[u]);
    let mut anc: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for &u in &order {
        let mut set: HashSet<usize> = HashSet::new();
        for &p in &parents[u] {
            set.insert(p);
            set.extend(anc[p].iter().copied());
        }
        anc[u] = set;
    }

    let mut reduced: ParentLists = vec![Vec::new(); n];
    let mut implied: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for u in 0..n {
        let declared = &parents[u];
        let implies = |q: usize, p: usize| q != p && anc[q].contains(&p);
        let kept: Vec<usize> = declared
            .iter()
            .copied()
            .filter(|&p| !declared.iter().any(|&q| implies(q, p)))
            .collect();
        for &p in declared {
            if kept.contains(&p) {
                reduced[u].push(p);
                continue;
            }
            let via = kept
                .iter()
                .copied()
                .find(|&q| implies(q, p))
                .or_else(|| declared.iter().copied().find(|&q| implies(q, p)));
            if let Some(via) = via {
                implied[u].push((p, via));
            }
        }
    }
    (reduced, implied)
}

/// One entry of a row sequence: the full list order ([`row_sequence`]) or
/// its visible part, which the lane assigner walks (quick 260927-gi9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Summary,
    Band(usize),
    Phase(usize),
}

/// What a laid-out (visible) row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaneKind {
    Summary,
    Band(usize),
    /// A fork or merge belonging to this node's row.
    Connector(usize),
    /// A node on a lane.
    Phase(usize, usize),
}

#[derive(Debug, Clone)]
struct LaneRow {
    kind: LaneKind,
    lanes: String,
    /// One entry per char of `lanes` (see [`ListRow`]).
    links: Vec<Option<LaneLink>>,
}

/// The reduced dependency edge a lane char draws (quick 261006-ujx). `owner`
/// is the phase whose row opened the lane (upstream, the dependency), `target`
/// the phase it runs down to (the dependent); both are indices into
/// [`RoadmapModel::phases`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneLink {
    pub owner: usize,
    pub target: usize,
}

/// A lane string and its per-char links, as the lane builders return them.
type LaneCells = (String, Vec<Option<LaneLink>>);

/// A lane junction from its four connections.
fn junction(left: bool, right: bool, up: bool, down: bool) -> &'static str {
    match (left, right, up, down) {
        (true, true, true, true) => LANE_CROSS,
        (true, true, false, true) => LANE_TEE_DOWN,
        (true, true, true, false) => LANE_TEE_UP,
        (false, true, true, true) => LANE_TEE_RIGHT,
        (true, false, true, true) => LANE_TEE_LEFT,
        (false, true, false, true) => LANE_DOWN_RIGHT,
        (true, false, false, true) => LANE_DOWN_LEFT,
        (false, true, true, false) => LANE_UP_RIGHT,
        (true, false, true, false) => LANE_UP_LEFT,
        (_, _, true, _) | (_, _, _, true) => LANE_VERTICAL,
        _ => LANE_HORIZONTAL,
    }
}

/// The lowest free lane, never `avoid` (the previous phase row's lane, so an
/// unrelated root never stacks under it as if chained — D-A08).
fn free_lane(active: &[Option<LaneLink>], avoid: Option<usize>) -> usize {
    (0..)
        .find(|&l| active.get(l).is_none_or(Option::is_none) && Some(l) != avoid)
        .unwrap_or(0)
}

fn claim(active: &mut Vec<Option<LaneLink>>, lane: usize, target: Option<LaneLink>) {
    if active.len() <= lane {
        active.resize(lane + 1, None);
    }
    active[lane] = target;
}

/// Join per-lane cells into a lane string: each lane is its glyph plus the
/// cell to its right (`─` inside a horizontal span, else blank), trimmed.
/// Each cell carries its link; the separator right of lane `l` carries
/// `sep_link(l)` when drawn `─`, else `None`. The links come back one per
/// char of the trimmed string.
fn join_cells(
    cells: &[(&str, Option<LaneLink>)],
    span: Option<(usize, usize)>,
    sep_link: impl Fn(usize) -> Option<LaneLink>,
) -> LaneCells {
    let mut out = String::new();
    let mut links = Vec::with_capacity(cells.len() * 2);
    for (l, &(cell, link)) in cells.iter().enumerate() {
        out.push_str(cell);
        links.extend(std::iter::repeat_n(link, cell.chars().count()));
        let inside = span.is_some_and(|(lo, hi)| l >= lo && l < hi);
        if inside {
            out.push_str(LANE_HORIZONTAL);
            links.push(sep_link(l));
        } else {
            out.push(' ');
            links.push(None);
        }
    }
    let trimmed = out.trim_end().to_string();
    links.truncate(trimmed.chars().count());
    (trimmed, links)
}

/// The link at `active[l]`, `None` past the end or on a free lane.
fn link_at(active: &[Option<LaneLink>], l: usize) -> Option<LaneLink> {
    active.get(l).copied().flatten()
}

/// A row that only passes active lanes through (band rows, phase rows' other
/// lanes). The node's own cell carries no link.
fn pass_through(active: &[Option<LaneLink>], node: Option<(usize, &str)>) -> LaneCells {
    let width = active.len().max(node.map_or(0, |(l, _)| l + 1));
    let cells: Vec<(&str, Option<LaneLink>)> = (0..width)
        .map(|l| match node {
            Some((lane, glyph)) if lane == l => (glyph, None),
            _ => match link_at(active, l) {
                Some(link) => (LANE_VERTICAL, Some(link)),
                None => (" ", None),
            },
        })
        .collect();
    join_cells(&cells, None, |_| None)
}

/// A fork (`fork`, `others` are the new lanes opening below) or merge
/// (`others` are the lanes closing into `lane`) connector row around `lane`.
///
/// Links: a cell on an active lane (the node's, an `others` lane, a crossed
/// `┼` lane) carries that lane's link. A horizontal-run char with no lane of
/// its own — an inactive `─` cell or a `─` separator — carries the link of the
/// nearest `others` lane beyond it, on the far side from `lane` (I-10).
fn connector_row(
    active: &[Option<LaneLink>],
    lane: usize,
    others: &[usize],
    fork: bool,
) -> LaneCells {
    let lo = others.iter().copied().fold(lane, usize::min);
    let hi = others.iter().copied().fold(lane, usize::max);
    let width = active.len().max(hi + 1);
    // The `others` lane owning run position `l`: right of `lane`, the
    // smallest `o > l`; left of it, the largest `o <= l` (for a run cell,
    // whose own column is never an `others` lane, that is the largest `o < l`).
    let beyond = |l: usize, right: bool| -> Option<LaneLink> {
        let o = if right {
            others.iter().copied().filter(|&o| o > l).min()
        } else {
            others.iter().copied().filter(|&o| o <= l).max()
        };
        o.and_then(|o| link_at(active, o))
    };
    let cells: Vec<(&str, Option<LaneLink>)> = (0..width)
        .map(|l| {
            let inside = l > lo && l < hi;
            let own = link_at(active, l);
            if l == lane {
                (junction(lo < lane, hi > lane, true, true), own)
            } else if others.contains(&l) {
                (junction(l > lo, l < hi, !fork, fork), own)
            } else if own.is_some() {
                (if inside { LANE_CROSS } else { LANE_VERTICAL }, own)
            } else if inside {
                (LANE_HORIZONTAL, beyond(l, l > lane))
            } else {
                (" ", None)
            }
        })
        .collect();
    join_cells(&cells, Some((lo, hi)), |l| beyond(l, l >= lane))
}

/// Assign git-log lanes over the row sequence it is given — the visible rows,
/// folds already applied (RESEARCH Pattern 2; quick 260927-gi9).
///
/// Edges run only from an earlier row to a later one along REDUCED parents,
/// and only when both endpoints are in `seq`: an edge touching a folded
/// phase draws no lane.
/// A node sits on its lowest incoming lane (several → one merge connector
/// before it); a root takes the lowest free lane that is not the previous
/// phase row's (a band row resets that). After a node, its first later child
/// continues its lane and every further child takes the lowest free lane
/// (one fork connector after it).
fn assign_lanes(seq: &[Slot], reduced: &ParentLists, glyphs: &[&str]) -> Vec<LaneRow> {
    let n = reduced.len();
    let mut pos = vec![usize::MAX; n];
    for (r, slot) in seq.iter().enumerate() {
        if let Slot::Phase(u) = *slot {
            pos[u] = r;
        }
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (c, list) in reduced.iter().enumerate() {
        for &p in list {
            if pos[p] != usize::MAX && pos[c] != usize::MAX && pos[p] < pos[c] {
                children[p].push(c);
            }
        }
    }
    for list in &mut children {
        list.sort_by_key(|&c| pos[c]);
    }

    let mut active: Vec<Option<LaneLink>> = Vec::new();
    let mut prev: Option<usize> = None;
    let mut out: Vec<LaneRow> = Vec::with_capacity(seq.len());
    for slot in seq {
        let u = match *slot {
            Slot::Summary | Slot::Band(_) => {
                let kind = match *slot {
                    Slot::Band(b) => LaneKind::Band(b),
                    _ => LaneKind::Summary,
                };
                let (lanes, links) = pass_through(&active, None);
                out.push(LaneRow { kind, lanes, links });
                prev = None;
                continue;
            }
            Slot::Phase(u) => u,
        };
        let incoming: Vec<usize> = (0..active.len())
            .filter(|&l| active[l].is_some_and(|k| k.target == u))
            .collect();
        let lane = match incoming.split_first() {
            Some((&first, rest)) => {
                if !rest.is_empty() {
                    let (lanes, links) = connector_row(&active, first, rest, false);
                    out.push(LaneRow {
                        kind: LaneKind::Connector(u),
                        lanes,
                        links,
                    });
                    for &l in rest {
                        active[l] = None;
                    }
                }
                first
            }
            None => free_lane(&active, prev),
        };
        claim(&mut active, lane, None);
        let (lanes, links) =
            pass_through(&active, Some((lane, glyphs.get(u).copied().unwrap_or(" "))));
        out.push(LaneRow {
            kind: LaneKind::Phase(u, lane),
            lanes,
            links,
        });
        match children[u].split_first() {
            Some((&first, rest)) => {
                claim(
                    &mut active,
                    lane,
                    Some(LaneLink {
                        owner: u,
                        target: first,
                    }),
                );
                let mut forked = Vec::with_capacity(rest.len());
                for &c in rest {
                    let l = free_lane(&active, None);
                    claim(
                        &mut active,
                        l,
                        Some(LaneLink {
                            owner: u,
                            target: c,
                        }),
                    );
                    forked.push(l);
                }
                if !forked.is_empty() {
                    let (lanes, links) = connector_row(&active, lane, &forked, true);
                    out.push(LaneRow {
                        kind: LaneKind::Connector(u),
                        lanes,
                        links,
                    });
                }
            }
            None => claim(&mut active, lane, None),
        }
        while active.last().is_some_and(Option::is_none) {
            active.pop();
        }
        prev = Some(lane);
    }
    out
}

/// Lay out `input` as the vertical Roadmap list (D-A01, D-A07, D-A08).
///
/// Pure: no ratatui types, no I/O. Rows hidden by `fold_toggles` (see
/// [`is_folded`]) are dropped first, then lanes are laid out over the visible
/// rows only, so a folded milestone reserves no gutter width and an edge
/// touching a folded phase draws no lane (quick 260927-gi9). Facts, bands,
/// `start_now` and waves stay computed over the full graph.
pub fn layout_list(input: &ListInput<'_>, fold_toggles: &HashSet<BandKey>) -> RoadmapModel {
    let nodes = &input.nodes;
    let n = nodes.len();
    let labels: Vec<String> = nodes.iter().map(|node| esc(node.id)).collect();

    // The kept D-A15 pipeline.
    let pairs: Vec<(&str, &[String])> = nodes.iter().map(|node| (node.id, node.deps)).collect();
    let (mut parents, externals, mut cycles) = resolve_deps(&pairs, &labels);
    break_cycles(&mut parents, &labels, &mut cycles);
    let layer = longest_path_layers(&parents);
    let (reduced, implied) = transitive_reduction(&parents, &layer);

    // Status (D-B12): an external dependency counts as satisfied (A12).
    let status: Vec<PhaseStatus> = (0..n)
        .map(|u| match nodes[u].marker {
            PhaseMarker::Done => PhaseStatus::Done,
            PhaseMarker::Current => PhaseStatus::Active,
            PhaseMarker::Future => {
                if parents[u]
                    .iter()
                    .all(|&p| nodes[p].marker == PhaseMarker::Done)
                {
                    PhaseStatus::Ready
                } else {
                    PhaseStatus::Blocked
                }
            }
        })
        .collect();

    // Bands (D-A07): members in input order; an out-of-range band is none.
    let band_of: Vec<Option<usize>> = nodes
        .iter()
        .map(|node| node.band.filter(|&b| b < input.bands.len()))
        .collect();
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); input.bands.len()];
    for (u, band) in band_of.iter().enumerate() {
        if let Some(b) = *band {
            members[b].push(u);
        }
    }
    let bands: Vec<BandFacts> = input
        .bands
        .iter()
        .enumerate()
        .map(|(b, band)| {
            let label = String::from(band.label.shown());
            let (short, _) = roadmap_md::split_milestone_label(&label);
            BandFacts {
                key: band_key(band),
                label,
                short,
                shipped: band.shipped,
                done: members[b].iter().filter(|&&u| nodes[u].done).count(),
                total: members[b].len(),
                declared_phases: band.declared_phases,
            }
        })
        .collect();
    let shipped: Vec<usize> = (0..bands.len()).filter(|&b| bands[b].shipped).collect();

    let seq = row_sequence(&bands, &members, &band_of);

    // Folding happens BEFORE lane layout (quick 260927-gi9, D-01): lanes
    // exist only for the rows that are drawn, so a folded milestone reserves
    // no gutter and visible rows pack into the leftmost columns. An edge with
    // a folded endpoint has no position in the visible sequence, so
    // `assign_lanes` draws no lane for it (I-1). The price: toggling a fold
    // may move the lanes of rows outside it (I-4). With nothing folded the
    // visible sequence is `seq` itself, so the unfolded layout is unchanged.
    let summary_folded = is_folded(&BandKey::Shipped, fold_toggles);
    let band_hidden = |b: usize| bands[b].shipped && summary_folded;
    let phases_hidden = |b: usize| band_hidden(b) || is_folded(&bands[b].key, fold_toggles);
    let node_hidden = |u: usize| band_of[u].is_some_and(phases_hidden);
    let visible: Vec<Slot> = seq
        .iter()
        .copied()
        .filter(|slot| match *slot {
            Slot::Summary => true,
            Slot::Band(b) => !band_hidden(b),
            Slot::Phase(u) => !node_hidden(u),
        })
        .collect();
    let glyphs: Vec<&str> = status.iter().map(|s| s.glyph()).collect();
    let laid = assign_lanes(&visible, &reduced, &glyphs);

    // List order of the nodes.
    let order: Vec<usize> = seq
        .iter()
        .filter_map(|slot| match *slot {
            Slot::Phase(u) => Some(u),
            _ => None,
        })
        .collect();
    let mut pos = vec![usize::MAX; n];
    for (i, &u) in order.iter().enumerate() {
        pos[u] = i;
    }

    let mut unblocks: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut declared_by_any = vec![false; n];
    for c in 0..n {
        for &p in &reduced[c] {
            unblocks[p].push(c);
        }
        for &p in &parents[c] {
            declared_by_any[p] = true;
        }
    }
    for list in &mut unblocks {
        list.sort_by_key(|&c| pos[c]);
    }

    let phases: Vec<PhaseFacts> = (0..n)
        .map(|u| {
            let node = &nodes[u];
            let no_deps = reduced[u].is_empty() && implied[u].is_empty() && externals[u].is_empty();
            PhaseFacts {
                key: phase_key(node.id),
                id: labels[u].clone(),
                name: esc(node.name),
                goal: node.goal.map(|g| String::from(g.shown())),
                band: band_of[u],
                wave: layer[u] + 1,
                status: status[u],
                planned: node.planned,
                plans: node.plans,
                unmerged: 0,
                badge: node.badge.as_deref().map(esc),
                needs: reduced[u].clone(),
                implied: implied[u].clone(),
                external: externals[u].clone(),
                unblocks: unblocks[u].clone(),
                parallel: order
                    .iter()
                    .copied()
                    .filter(|&v| v != u && layer[v] == layer[u])
                    .collect(),
                no_deps,
                no_edges: no_deps && !declared_by_any[u],
                last_in_band: band_of[u].is_some_and(|b| members[b].last() == Some(&u)),
            }
        })
        .collect();

    let summary_text = match (shipped.first(), shipped.last()) {
        (Some(&first), Some(&last)) if first != last => {
            format!("{} \u{2026} {}", bands[first].short, bands[last].short)
        }
        (Some(&only), _) => bands[only].short.clone(),
        _ => String::new(),
    };
    let summary_phases: u32 = shipped
        .iter()
        .map(|&b| {
            let listed = u32::try_from(members[b].len()).unwrap_or(u32::MAX);
            bands[b].declared_phases.max(listed)
        })
        .fold(0u32, u32::saturating_add);

    // Every laid row is visible: the `visible` filter above is the single
    // place folding is decided.
    let rows: Vec<ListRow> = laid
        .into_iter()
        .map(|row| match row.kind {
            LaneKind::Summary => ListRow::ShippedSummary {
                text: summary_text.clone(),
                milestones: shipped.len(),
                phases: summary_phases,
                folded: summary_folded,
                lanes: row.lanes,
                links: row.links,
            },
            LaneKind::Band(b) => ListRow::Band {
                band: b,
                key: bands[b].key.clone(),
                label: bands[b].label.clone(),
                short: bands[b].short.clone(),
                done: bands[b].done,
                total: bands[b].total,
                folded: is_folded(&bands[b].key, fold_toggles),
                lanes: row.lanes,
                links: row.links,
            },
            LaneKind::Connector(_) => ListRow::Connector {
                lanes: row.lanes,
                links: row.links,
            },
            LaneKind::Phase(node, lane) => ListRow::Phase {
                node,
                lane,
                lanes: row.lanes,
                links: row.links,
            },
        })
        .collect();

    RoadmapModel {
        rows,
        start_now: order
            .iter()
            .copied()
            .filter(|&u| matches!(status[u], PhaseStatus::Active | PhaseStatus::Ready))
            .collect(),
        max_wave: layer.iter().map(|l| l + 1).max().unwrap_or(0),
        done: nodes.iter().filter(|node| node.done).count(),
        total: n,
        notes: cycle_note(&cycles).into_iter().collect(),
        phases,
        bands,
    }
}

/// A band's key: its lower-cased trimmed RAW label (logic only).
fn band_key(band: &BandInput) -> BandKey {
    BandKey::Named(band.label.as_raw_for_logic_only().trim().to_lowercase())
}

/// The unfolded row sequence (D-A07): the shipped summary (when any band
/// shipped) followed by every shipped band and its phases; then every
/// non-shipped band that has phases, each followed by its phases; then the
/// band-less phases. A non-shipped band without phases draws nothing.
fn row_sequence(
    bands: &[BandFacts],
    members: &[Vec<usize>],
    band_of: &[Option<usize>],
) -> Vec<Slot> {
    let mut seq = Vec::with_capacity(bands.len() + band_of.len() + 1);
    let push_band = |seq: &mut Vec<Slot>, b: usize| {
        seq.push(Slot::Band(b));
        seq.extend(members[b].iter().map(|&u| Slot::Phase(u)));
    };
    if bands.iter().any(|band| band.shipped) {
        seq.push(Slot::Summary);
        for b in (0..bands.len()).filter(|&b| bands[b].shipped) {
            push_band(&mut seq, b);
        }
    }
    for b in (0..bands.len()).filter(|&b| !bands[b].shipped && !members[b].is_empty()) {
        push_band(&mut seq, b);
    }
    seq.extend(
        band_of
            .iter()
            .enumerate()
            .filter(|(_, band)| band.is_none())
            .map(|(u, _)| Slot::Phase(u)),
    );
    seq
}

/// Which edges `h` / `l` follow (D-A10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDir {
    /// Dependencies: reduced needs, then implied ones (`h`).
    Needs,
    /// Phases this one unblocks (`l`).
    Unblocks,
}

/// An `h` / `l` walk in progress: repeating the key while the cursor still
/// sits on `target` continues cycling the ORIGIN's edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeWalk {
    /// `phase_key` of the phase the walk started from.
    pub origin: String,
    pub dir: EdgeDir,
    /// Index into the origin's candidate list.
    pub index: usize,
    /// `phase_key` of the phase the walk last jumped to.
    pub target: String,
}

/// `t` with a phase key normalised through [`phase_key`] (`07` → `7`).
fn normalized(t: &CursorTarget) -> CursorTarget {
    match t {
        CursorTarget::Phase(key) => CursorTarget::Phase(phase_key(key)),
        CursorTarget::Band(_) => t.clone(),
    }
}

/// Open `key` if `fold_toggles` folds it.
fn open_band(key: &BandKey, fold_toggles: &mut HashSet<BandKey>) {
    if is_folded(key, fold_toggles) && !fold_toggles.remove(key) {
        fold_toggles.insert(key.clone());
    }
}

/// Cursor navigation (D-A05, D-A10). Pure: targets are keys, never row
/// indices, so a ROADMAP reload or a fold never moves the selection to a
/// different phase (RESEARCH Pitfall 7). A returned target may be a
/// fold-hidden phase (`h`/`l`, `[`/`]`); the caller then calls
/// [`RoadmapModel::unfold_for`].
impl RoadmapModel {
    /// Re-status phase `node` after layout, keeping its lane glyph in step:
    /// the glyph `layout_list` baked into the phase row's `lanes` at cell
    /// `2 * lane` is replaced by the new status's glyph (quick 260929-szq's
    /// unmerged post-pass). Lanes, edges and every other phase are untouched,
    /// so dependents keep the status they were laid out with.
    pub fn set_phase_status(&mut self, node: usize, status: PhaseStatus) {
        let Some(facts) = self.phases.get_mut(node) else {
            return;
        };
        let old = facts.status.glyph();
        facts.status = status;
        let new = status.glyph();
        for row in &mut self.rows {
            if let ListRow::Phase {
                node: n,
                lane,
                lanes,
                ..
            } = row
            {
                if *n != node {
                    continue;
                }
                let at = 2 * *lane;
                let mut chars: Vec<String> = lanes.chars().map(String::from).collect();
                if chars.get(at).is_some_and(|c| c == old) {
                    chars[at] = new.to_string();
                    *lanes = chars.concat();
                }
            }
        }
    }

    /// The cursor target a row stands for; connectors stand for none.
    fn target_of(&self, row: &ListRow) -> Option<CursorTarget> {
        match row {
            ListRow::ShippedSummary { .. } => Some(CursorTarget::Band(BandKey::Shipped)),
            ListRow::Band { key, .. } => Some(CursorTarget::Band(key.clone())),
            ListRow::Phase { node, .. } => self
                .phases
                .get(*node)
                .map(|p| CursorTarget::Phase(p.key.clone())),
            ListRow::Connector { .. } => None,
        }
    }

    /// The cursor target model row `row` stands for — the mouse's row mapping
    /// (quick 260926-kes, D-01). The one `target_of` rule, so a click and the
    /// keys agree on what a row is; `None` for a connector row and past the
    /// last row.
    pub fn target_at(&self, row: usize) -> Option<CursorTarget> {
        self.rows.get(row).and_then(|r| self.target_of(r))
    }

    /// Every phase in list order, hidden ones included: shipped bands, then
    /// the other bands, each in band order, then band-less phases — the
    /// order `layout_list` lays them out in.
    fn list_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.phases.len()).collect();
        order.sort_by_key(|&u| match self.phases[u].band {
            Some(b) => (usize::from(!self.bands[b].shipped), b, u),
            None => (2, 0, u),
        });
        order
    }

    /// Every row the cursor can rest on, in row order (connectors excluded).
    pub fn visible_targets(&self) -> Vec<CursorTarget> {
        self.rows
            .iter()
            .filter_map(|row| self.target_of(row))
            .collect()
    }

    /// Where `t` can be shown: itself when visible; a fold-hidden phase or a
    /// hidden shipped band resolves to the row that folds it.
    fn settle(&self, t: &CursorTarget) -> Option<CursorTarget> {
        let t = normalized(t);
        if self.row_of(&t).is_some() {
            return Some(t);
        }
        let band = match &t {
            CursorTarget::Phase(key) => self.phases[self.phase_index(key)?].band?,
            CursorTarget::Band(key) => self.bands.iter().position(|b| b.key == *key)?,
        };
        let own = CursorTarget::Band(self.bands.get(band)?.key.clone());
        if own != t && self.row_of(&own).is_some() {
            return Some(own);
        }
        let summary = CursorTarget::Band(BandKey::Shipped);
        (self.bands[band].shipped && self.row_of(&summary).is_some()).then_some(summary)
    }

    /// The target a stored cursor resolves to: the stored target when the
    /// model still has it (a fold-hidden phase → its band row, or the shipped
    /// summary); otherwise the active phase, else the first phase not done,
    /// else the first visible target. `None` only for an empty model.
    pub fn resolve_cursor(&self, stored: Option<&CursorTarget>) -> Option<CursorTarget> {
        if let Some(t) = stored.and_then(|t| self.settle(t)) {
            return Some(t);
        }
        let order = self.list_order();
        let pick = order
            .iter()
            .copied()
            .find(|&u| self.phases[u].status == PhaseStatus::Active)
            .or_else(|| {
                order
                    .iter()
                    .copied()
                    .find(|&u| self.phases[u].status != PhaseStatus::Done)
            });
        pick.and_then(|u| self.settle(&CursorTarget::Phase(self.phases[u].key.clone())))
            .or_else(|| self.first_target())
    }

    /// The visible row index of `t`, `None` when it is hidden or unknown.
    pub fn row_of(&self, t: &CursorTarget) -> Option<usize> {
        let t = normalized(t);
        self.rows
            .iter()
            .position(|row| self.target_of(row).as_ref() == Some(&t))
    }

    /// Move `delta` targets from `from` (resolved first), clamped at both
    /// ends; connector rows are never targets.
    pub fn step(&self, from: &CursorTarget, delta: isize) -> CursorTarget {
        let targets = self.visible_targets();
        let Some(current) = self.resolve_cursor(Some(from)) else {
            return from.clone();
        };
        let at = targets.iter().position(|t| *t == current).unwrap_or(0);
        let to = at
            .saturating_add_signed(delta)
            .min(targets.len().saturating_sub(1));
        targets.get(to).cloned().unwrap_or(current)
    }

    /// The first visible target (`g`).
    pub fn first_target(&self) -> Option<CursorTarget> {
        self.visible_targets().into_iter().next()
    }

    /// The last visible target (`G`).
    pub fn last_target(&self) -> Option<CursorTarget> {
        self.visible_targets().pop()
    }

    /// The index in [`RoadmapModel::phases`] of the phase with `key`
    /// (matched through [`phase_key`], so `07` finds `7`).
    pub fn phase_index(&self, key: &str) -> Option<usize> {
        let key = phase_key(key);
        self.phases.iter().position(|p| p.key == key)
    }

    /// Follow an edge from `from` (`h`: needs then implied deps; `l`: the
    /// phases it unblocks). Repeating with the returned walk while the cursor
    /// still sits on `walk.target` cycles the ORIGIN's edges, wrapping; any
    /// other walk restarts at the cursor. `None` when there is no edge.
    pub fn edge_jump(
        &self,
        from: &CursorTarget,
        walk: Option<&EdgeWalk>,
        dir: EdgeDir,
    ) -> Option<(CursorTarget, EdgeWalk)> {
        let CursorTarget::Phase(key) = from else {
            return None;
        };
        let at = self.phase_index(key)?;
        let continued = walk.and_then(|w| {
            let on_target = self.phase_index(&w.target) == Some(at);
            let origin = self.phase_index(&w.origin)?;
            (w.dir == dir && on_target).then_some((origin, w.index.wrapping_add(1)))
        });
        let (origin, index) = continued.unwrap_or((at, 0));
        let p = &self.phases[origin];
        let candidates: Vec<usize> = match dir {
            EdgeDir::Needs => p
                .needs
                .iter()
                .copied()
                .chain(p.implied.iter().map(|&(dep, _)| dep))
                .collect(),
            EdgeDir::Unblocks => p.unblocks.clone(),
        };
        if candidates.is_empty() {
            return None;
        }
        let index = index % candidates.len();
        let target = self.phases[candidates[index]].key.clone();
        Some((
            CursorTarget::Phase(target.clone()),
            EdgeWalk {
                origin: p.key.clone(),
                dir,
                index,
                target,
            },
        ))
    }

    /// The next (`forward`) or previous phase of the same wave in list
    /// order, wrapping; `None` for a band row or a phase alone in its wave.
    pub fn wave_step(&self, from: &CursorTarget, forward: bool) -> Option<CursorTarget> {
        let CursorTarget::Phase(key) = from else {
            return None;
        };
        let at = self.phase_index(key)?;
        let wave = self.phases[at].wave;
        let same: Vec<usize> = self
            .list_order()
            .into_iter()
            .filter(|&u| self.phases[u].wave == wave)
            .collect();
        if same.len() < 2 {
            return None;
        }
        let pos = same.iter().position(|&u| u == at)?;
        let next = if forward {
            (pos + 1) % same.len()
        } else {
            (pos + same.len() - 1) % same.len()
        };
        Some(CursorTarget::Phase(self.phases[same[next]].key.clone()))
    }

    /// Edit `fold_toggles` so the phase `phase_key` is visible: its band is
    /// unfolded and, for a shipped band, so is the shipped summary.
    pub fn unfold_for(&self, phase_key: &str, fold_toggles: &mut HashSet<BandKey>) {
        let Some(band) = self
            .phase_index(phase_key)
            .and_then(|u| self.phases[u].band)
            .and_then(|b| self.bands.get(b))
        else {
            return;
        };
        open_band(&band.key, fold_toggles);
        if band.shipped {
            open_band(&BandKey::Shipped, fold_toggles);
        }
    }
}

/// The model as plain text, one line per visible row: the lane column padded
/// to ten cells (the node cell drawn `o`), then the phase id, `[short]` for a
/// band, `[shipped]` for the shipped summary, nothing for a connector.
pub fn lane_text(model: &RoadmapModel) -> Vec<String> {
    model
        .rows
        .iter()
        .map(|row| {
            let (lanes, label) = match row {
                ListRow::Phase {
                    node, lane, lanes, ..
                } => (
                    lanes
                        .chars()
                        .enumerate()
                        .map(|(i, c)| if i == 2 * lane { 'o' } else { c })
                        .collect::<String>(),
                    model
                        .phases
                        .get(*node)
                        .map_or(String::new(), |p| p.id.clone()),
                ),
                ListRow::Band { lanes, short, .. } => (lanes.clone(), format!("[{short}]")),
                ListRow::ShippedSummary { lanes, .. } => (lanes.clone(), "[shipped]".to_string()),
                ListRow::Connector { lanes, .. } => (lanes.clone(), String::new()),
            };
            format!("{lanes:<LANE_TEXT_COLUMN$}{label}")
                .trim_end()
                .to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // The vertical list model (phase 24)
    // -----------------------------------------------------------------------

    const D: PhaseMarker = PhaseMarker::Done;
    const C: PhaseMarker = PhaseMarker::Current;
    const F: PhaseMarker = PhaseMarker::Future;

    /// A list spec: `(id, name, deps, marker)`.
    type LSpec<'a> = &'a [(&'a str, &'a str, &'a [&'a str], PhaseMarker)];
    /// Bands: `(label, shipped, declared phases, member ids)`.
    type BSpec<'a> = &'a [(&'a str, bool, u32, &'a [&'a str])];

    fn build(spec: LSpec<'_>, bands: BSpec<'_>, toggles: &HashSet<BandKey>) -> RoadmapModel {
        let deps: Vec<Vec<String>> = spec
            .iter()
            .map(|(_, _, deps, _)| deps.iter().map(|d| d.to_string()).collect())
            .collect();
        let input = ListInput {
            nodes: spec
                .iter()
                .zip(&deps)
                .map(|(&(id, name, _, marker), deps)| ListNode {
                    id,
                    name,
                    deps,
                    band: bands.iter().position(|(_, _, _, ids)| ids.contains(&id)),
                    marker,
                    done: marker == PhaseMarker::Done,
                    plans: None,
                    goal: None,
                    planned: false,
                    badge: None,
                })
                .collect(),
            bands: bands
                .iter()
                .map(|&(label, shipped, declared, _)| BandInput {
                    label: crate::text::Untrusted::from_untrusted_source(label.to_string()),
                    shipped,
                    declared_phases: declared,
                })
                .collect(),
        };
        layout_list(&input, toggles)
    }

    fn plain(spec: LSpec<'_>) -> RoadmapModel {
        build(spec, &[], &HashSet::new())
    }

    fn idx(model: &RoadmapModel, id: &str) -> usize {
        model
            .phases
            .iter()
            .position(|p| p.id == id)
            .unwrap_or_else(|| panic!("no phase {id}"))
    }

    fn facts<'m>(model: &'m RoadmapModel, id: &str) -> &'m PhaseFacts {
        &model.phases[idx(model, id)]
    }

    fn ids(model: &RoadmapModel, list: &[usize]) -> Vec<String> {
        list.iter().map(|&i| model.phases[i].id.clone()).collect()
    }

    fn implied_ids(model: &RoadmapModel, id: &str) -> Vec<(String, String)> {
        facts(model, id)
            .implied
            .iter()
            .map(|&(dep, via)| (model.phases[dep].id.clone(), model.phases[via].id.clone()))
            .collect()
    }

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    /// daily-vow v1.5 (Mockup B): 18 depends on the external 17.
    const DAILY_VOW: LSpec<'static> = &[
        ("18", "Calibration Foundation", &["17"], D),
        ("19", "Effective Profile Resolution", &["18"], D),
        ("20", "Notification Scheduling Extraction", &["19"], D),
        ("21", "Probes End-to-End", &["20"], D),
        ("22", "Response-Weighted Nudge Selection", &["21"], D),
        ("23", "Transparency, Reset & History", &["21", "20"], C),
    ];

    /// sentriq v0.12 (Mockup C).
    const SENTRIQ: LSpec<'static> = &[
        ("9", "Routine Event Logging", &[], C),
        ("10", "The Air Box Test", &["9"], F),
        ("11", "First Supervised On-Vehicle Run", &["10", "9"], F),
        ("12", "Two-Truck Hardware Validation", &[], F),
    ];

    /// ttbook's v2 phases plus build phases 14-18.
    const TTBOOK: LSpec<'static> = &[
        ("8", "", &[], F),
        ("9", "", &["8"], F),
        ("10", "", &["8", "9"], F),
        ("11", "", &["8", "9"], F),
        ("12", "", &["9", "10", "11"], F),
        ("13", "", &["12"], F),
        ("14", "", &["8", "9", "10", "11", "12", "13"], F),
        ("15", "", &["14"], F),
        ("16", "", &["12", "13"], F),
        ("17", "", &["12", "13", "16"], F),
        ("18", "", &["12"], F),
    ];

    const ROOTS: LSpec<'static> = &[
        ("1", "", &[], F),
        ("2", "", &[], F),
        ("3", "", &[], F),
        ("4", "", &[], F),
    ];

    #[test]
    fn reduction_daily_vow_23_implies_20_via_21() {
        let model = plain(DAILY_VOW);
        assert_eq!(implied_ids(&model, "23"), pairs(&[("20", "21")]));
        assert_eq!(ids(&model, &facts(&model, "23").needs), vec!["21"]);
    }

    #[test]
    fn reduction_sentriq_11_implies_9_via_10() {
        let model = plain(SENTRIQ);
        assert_eq!(implied_ids(&model, "11"), pairs(&[("9", "10")]));
        assert_eq!(ids(&model, &facts(&model, "11").needs), vec!["10"]);
    }

    #[test]
    fn reduction_ttbook_shape() {
        let model = plain(TTBOOK);
        assert_eq!(implied_ids(&model, "10"), pairs(&[("8", "9")]));
        assert_eq!(implied_ids(&model, "11"), pairs(&[("8", "9")]));
        assert_eq!(implied_ids(&model, "12"), pairs(&[("9", "10")]));
        assert_eq!(ids(&model, &facts(&model, "12").needs), vec!["10", "11"]);
        assert_eq!(ids(&model, &facts(&model, "14").needs), vec!["13"]);
        assert_eq!(facts(&model, "14").implied.len(), 5);
        assert_eq!(ids(&model, &facts(&model, "17").needs), vec!["16"]);
    }

    #[test]
    fn waves_are_longest_path_plus_one() {
        let model = plain(DAILY_VOW);
        let waves: Vec<usize> = model.phases.iter().map(|p| p.wave).collect();
        assert_eq!(waves, vec![1, 2, 3, 4, 5, 5]);
        assert_eq!(model.max_wave, 5);

        let model = plain(SENTRIQ);
        let waves: Vec<usize> = model.phases.iter().map(|p| p.wave).collect();
        assert_eq!(waves, vec![1, 2, 3, 1]);
    }

    #[test]
    fn lanes_daily_vow_without_bands() {
        assert_eq!(
            lane_text(&plain(DAILY_VOW)),
            vec![
                "o         18",
                "o         19",
                "o         20",
                "o         21",
                "├─╮",
                "o │       22",
                "  o       23",
            ]
        );
    }

    #[test]
    fn lanes_sentriq_without_bands() {
        assert_eq!(
            lane_text(&plain(SENTRIQ)),
            vec![
                "o         9",
                "o         10",
                "o         11",
                "  o       12"
            ]
        );
    }

    #[test]
    fn lanes_roots_zig_zag() {
        assert_eq!(
            lane_text(&plain(ROOTS)),
            vec!["o         1", "  o       2", "o         3", "  o       4"]
        );
    }

    #[test]
    fn daily_vow_phase_20_has_exactly_one_row() {
        let model = plain(DAILY_VOW);
        let rows_of_20 = model
            .rows
            .iter()
            .filter(
                |row| matches!(row, ListRow::Phase { node, .. } if model.phases[*node].id == "20"),
            )
            .count();
        assert_eq!(rows_of_20, 1);
        // Every phase has exactly one row.
        let mut seen: Vec<usize> = model
            .rows
            .iter()
            .filter_map(|row| match row {
                ListRow::Phase { node, .. } => Some(*node),
                _ => None,
            })
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..model.phases.len()).collect::<Vec<_>>());
    }

    #[test]
    fn facts_for_mockup_b_phase_23() {
        let model = plain(DAILY_VOW);
        let p = facts(&model, "23");
        assert_eq!(ids(&model, &p.needs), vec!["21"]);
        assert_eq!(implied_ids(&model, "23"), pairs(&[("20", "21")]));
        assert_eq!(ids(&model, &p.parallel), vec!["22"]);
        assert!(p.unblocks.is_empty());
        assert_eq!(p.status, PhaseStatus::Active);
        assert_eq!(p.wave, 5);
        assert!(!p.no_deps && !p.no_edges);
        // The external 17 counts as satisfied; 18 is done anyway.
        assert_eq!(facts(&model, "18").external, vec!["17".to_string()]);
        assert_eq!(ids(&model, &model.start_now), vec!["23"]);
        assert_eq!((model.done, model.total), (5, 6));
    }

    #[test]
    fn facts_for_mockup_c_phase_12() {
        let model = plain(SENTRIQ);
        let p = facts(&model, "12");
        assert!(p.no_deps);
        assert!(p.no_edges);
        assert_eq!(ids(&model, &p.parallel), vec!["9"]);
        assert_eq!(p.status, PhaseStatus::Ready);
        assert_eq!(facts(&model, "10").status, PhaseStatus::Blocked);
        // 9 has no deps but 10 depends on it: it has edges.
        assert!(facts(&model, "9").no_deps && !facts(&model, "9").no_edges);
        assert_eq!(ids(&model, &model.start_now), vec!["9", "12"]);
    }

    /// Mockup A's invented `bookly` roadmap.
    const BOOKLY: LSpec<'static> = &[
        ("8", "Booking data model", &[], D),
        ("9", "Availability API", &["8"], D),
        ("10", "Slot picker UI", &["9"], C),
        ("11", "Calendar sync", &["9"], F),
        ("12", "Checkout & payments", &["10", "11"], F),
        ("13", "Confirmation flow", &["12"], F),
        ("14", "Reminders & notifications", &["13"], F),
        ("15", "Live booking launch", &["14"], F),
        ("16", "Chat backend", &["13"], F),
        ("17", "Chat UI", &["16"], F),
        ("18", "Web booking portal", &["12"], F),
    ];

    const BOOKLY_BANDS: BSpec<'static> = &[
        (
            "M3 Live booking",
            false,
            8,
            &["8", "9", "10", "11", "12", "13", "14", "15"],
        ),
        ("M4 Support chat", false, 2, &["16", "17"]),
        ("M5 Web", false, 1, &["18"]),
    ];

    const MOCKUP_A_LANES: &[&str] = &[
        "          [M3]",
        "o         8",
        "o         9",
        "├─╮",
        "o │       10",
        "│ o       11",
        "├─╯",
        "o         12",
        "├─╮",
        "o │       13",
        "├─┼─╮",
        "o │ │     14",
        "o │ │     15",
        "  │ │     [M4]",
        "  │ o     16",
        "  │ o     17",
        "  │       [M5]",
        "  o       18",
    ];

    /// daily-vow's five shipped milestones (no listed phases) and v1.5.
    const DAILY_VOW_BANDS: BSpec<'static> = &[
        ("v1.0 MVP", true, 3, &[]),
        ("v1.1 Daily Rhythm", true, 2, &[]),
        ("v1.2 Learning", true, 4, &[]),
        ("v1.3 Adaptive Timing", true, 4, &[]),
        ("v1.4 Probes", true, 4, &[]),
        (
            "v1.5 Closing the Loop",
            false,
            6,
            &["18", "19", "20", "21", "22", "23"],
        ),
        ("Requirement Coverage", false, 0, &[]),
    ];

    const SENTRIQ_BANDS: BSpec<'static> = &[
        ("v0.11 Phases", true, 4, &[]),
        (
            "v0.12 Actuation Routines",
            false,
            4,
            &["9", "10", "11", "12"],
        ),
        ("Scope Explicitly Excluded from v0.12", false, 0, &[]),
    ];

    fn toggles(keys: &[BandKey]) -> HashSet<BandKey> {
        keys.iter().cloned().collect()
    }

    fn band_rows(model: &RoadmapModel) -> Vec<&ListRow> {
        model
            .rows
            .iter()
            .filter(|row| matches!(row, ListRow::Band { .. }))
            .collect()
    }

    #[test]
    fn is_folded_defaults_and_toggles() {
        let none = HashSet::new();
        let named = BandKey::Named("m4 support chat".to_string());
        assert!(is_folded(&BandKey::Shipped, &none));
        assert!(!is_folded(&named, &none));
        assert!(!is_folded(&BandKey::Shipped, &toggles(&[BandKey::Shipped])));
        assert!(is_folded(&named, &toggles(std::slice::from_ref(&named))));
    }

    #[test]
    fn lanes_mockup_a_with_bands() {
        let model = build(BOOKLY, BOOKLY_BANDS, &HashSet::new());
        assert_eq!(lane_text(&model), MOCKUP_A_LANES.to_vec());

        let p = facts(&model, "12");
        assert_eq!(ids(&model, &p.needs), vec!["10", "11"]);
        assert_eq!(ids(&model, &p.unblocks), vec!["13", "18"]);
        assert!(p.parallel.is_empty());
        assert_eq!(p.wave, 4);
        assert_eq!(ids(&model, &model.start_now), vec!["10", "11"]);
        assert_eq!((model.bands[0].done, model.bands[0].total), (2, 8));
        assert_eq!(model.bands[0].short, "M3");
        assert_eq!(
            model.bands[0].key,
            BandKey::Named("m3 live booking".to_string())
        );
        for id in ["15", "17", "18"] {
            assert!(facts(&model, id).last_in_band, "{id}");
        }
        assert!(!facts(&model, "14").last_in_band);
        match band_rows(&model)[0] {
            ListRow::Band {
                label,
                short,
                done,
                total,
                folded,
                ..
            } => {
                assert_eq!(
                    (label.as_str(), short.as_str(), *done, *total, *folded),
                    ("M3 Live booking", "M3", 2, 8, false)
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn lanes_mockup_b_with_shipped_summary() {
        let model = build(DAILY_VOW, DAILY_VOW_BANDS, &HashSet::new());
        let text = lane_text(&model);
        assert_eq!(
            text[..3].to_vec(),
            vec!["          [shipped]", "          [v1.5]", "o         18"]
        );
        assert_eq!(text.len(), 2 + 7, "{text:?}");
        assert_eq!(
            model.rows[0],
            ListRow::ShippedSummary {
                text: "v1.0 \u{2026} v1.4".to_string(),
                milestones: 5,
                phases: 17,
                folded: true,
                lanes: String::new(),
                links: Vec::new(),
            }
        );
        let v15: Vec<&ListRow> = band_rows(&model)
            .into_iter()
            .filter(|row| matches!(row, ListRow::Band { label, .. } if label == "v1.5 Closing the Loop"))
            .collect();
        assert_eq!(v15.len(), 1);
        assert_eq!(band_rows(&model).len(), 1);
        assert!(facts(&model, "23").last_in_band);
        assert!(!facts(&model, "22").last_in_band);
        assert_eq!(model.bands.len(), DAILY_VOW_BANDS.len());
        assert_eq!(model.bands[0].key, BandKey::Named("v1.0 mvp".to_string()));
    }

    #[test]
    fn unfolding_the_shipped_summary_reveals_one_row_per_shipped_milestone() {
        let folded = lane_text(&build(DAILY_VOW, DAILY_VOW_BANDS, &HashSet::new()));
        let model = build(DAILY_VOW, DAILY_VOW_BANDS, &toggles(&[BandKey::Shipped]));
        let open = lane_text(&model);
        let mut expected = vec!["          [shipped]".to_string()];
        for short in ["v1.0", "v1.1", "v1.2", "v1.3", "v1.4"] {
            expected.push(format!("          [{short}]"));
        }
        expected.extend(folded[1..].iter().cloned());
        assert_eq!(open, expected);
        assert!(matches!(
            model.rows[0],
            ListRow::ShippedSummary { folded: false, .. }
        ));
    }

    #[test]
    fn lanes_sentriq_with_synthetic_band() {
        let model = build(SENTRIQ, SENTRIQ_BANDS, &HashSet::new());
        assert_eq!(
            lane_text(&model),
            vec![
                "          [shipped]",
                "          [v0.12]",
                "o         9",
                "o         10",
                "o         11",
                "  o       12",
            ]
        );
        assert!(matches!(
            &model.rows[0],
            ListRow::ShippedSummary { text, milestones: 1, phases: 4, folded: true, .. } if text == "v0.11"
        ));
    }

    /// Quick 260927-gi9 (I-5, G3): folding M4 drops 13's fork lane to the
    /// hidden 16, so lane 2 is no longer reserved on the rows below 13.
    #[test]
    fn folding_a_band_drops_the_lanes_only_its_phases_owned() {
        let m4 = BandKey::Named("m4 support chat".to_string());
        let model = build(BOOKLY, BOOKLY_BANDS, &toggles(&[m4]));
        let expected: &[&str] = &[
            "          [M3]",
            "o         8",
            "o         9",
            "├─╮",
            "o │       10",
            "│ o       11",
            "├─╯",
            "o         12",
            "├─╮",
            "o │       13",
            "o │       14",
            "o │       15",
            "  │       [M4]",
            "  │       [M5]",
            "  o       18",
        ];
        assert_eq!(lane_text(&model), expected.to_vec());
        assert!(model.rows.iter().any(|row| matches!(
            row,
            ListRow::Band { short, folded: true, lanes, .. } if short == "M4" && lanes == "  │"
        )));
        // The facts still cover every phase.
        assert_eq!(model.phases.len(), BOOKLY.len());
    }

    /// Quick 260927-gi9 (I-7): the reported shape. Shipped v1 lists no
    /// phases; shipped v2's six independent phases all feed v3's 14.
    const CROSS_FOLD: LSpec<'static> = &[
        ("8", "Booking model", &[], D),
        ("9", "Availability", &[], D),
        ("10", "Slot picker", &[], D),
        ("11", "Calendar sync", &[], D),
        ("12", "Checkout", &[], D),
        ("13", "Confirmation", &[], D),
        (
            "14",
            "Supervised first live booking",
            &["8", "9", "10", "11", "12", "13"],
            C,
        ),
        ("15", "Unattended window runs", &["14"], F),
        ("16", "Five9 client and transcript store", &[], D),
        ("17", "Autonomous responder", &["16"], F),
        ("18", "Telegram commands, users and group", &[], D),
        ("19", "Telegram Mini App and LAN HTTPS", &["18"], F),
        ("20", "Web frontend", &["19"], F),
    ];

    const CROSS_FOLD_BANDS: BSpec<'static> = &[
        ("v1 Foundations", true, 8, &[]),
        ("v2 Core booking", true, 6, &["8", "9", "10", "11", "12", "13"]),
        (
            "v3 Live booking, support chat and Telegram",
            false,
            6,
            &["14", "15", "16", "17", "18", "19"],
        ),
        ("Milestone 6: Web frontend", false, 1, &["20"]),
    ];

    fn v3_key() -> BandKey {
        BandKey::Named("v3 live booking, support chat and telegram".to_string())
    }

    /// Quick 260927-gi9 (D-01, D-02, D-03, I-1, I-3; G1, G1b): folded
    /// milestones reserve no lanes, and an edge with a hidden endpoint draws
    /// nothing.
    #[test]
    fn folding_the_shipped_summary_drops_lanes_of_hidden_phases() {
        let model = build(CROSS_FOLD, CROSS_FOLD_BANDS, &HashSet::new());
        let g1: &[&str] = &[
            "          [shipped]",
            "          [v3]",
            "o         14",
            "o         15",
            "  o       16",
            "  o       17",
            "o         18",
            "o         19",
            "│         [M6]",
            "o         20",
        ];
        assert_eq!(lane_text(&model), g1.to_vec());
        assert!(
            !model
                .rows
                .iter()
                .any(|row| matches!(row, ListRow::Connector { .. })),
            "{:#?}",
            lane_text(&model)
        );
        // 14 is the first visible phase row, on lane 0.
        assert_eq!(phase_lanes(&model).first(), Some(&0));
        let fourteen = idx(&model, "14");
        assert!(model.rows.iter().any(
            |row| matches!(row, ListRow::Phase { node, lane: 0, .. } if *node == fourteen)
        ));

        let both = build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[v3_key()]));
        let g1b: &[&str] = &[
            "          [shipped]",
            "          [v3]",
            "          [M6]",
            "o         20",
        ];
        assert_eq!(lane_text(&both), g1b.to_vec());

        // The facts are fold-independent (I-3).
        let open = build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[BandKey::Shipped]));
        for m in [&model, &both, &open] {
            assert_eq!(
                ids(m, &facts(m, "14").needs),
                vec!["8", "9", "10", "11", "12", "13"]
            );
            assert_eq!(ids(m, &m.start_now), ids(&open, &open.start_now));
        }
    }

    /// Quick 260927-gi9 (D-04, G2): with every fold open the layout is what
    /// the pre-fix code drew.
    #[test]
    fn unfolded_cross_fold_layout_is_pinned() {
        let model = build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[BandKey::Shipped]));
        let g2: &[&str] = &[
            "          [shipped]",
            "          [v1]",
            "          [v2]",
            "o         8",
            "│ o       9",
            "│ │ o     10",
            "│ │ │ o   11",
            "│ │ │ │ o 12",
            "│ │ │ │ │ o13",
            "│ │ │ │ │ │[v3]",
            "├─┴─┴─┴─┴─╯",
            "o         14",
            "o         15",
            "  o       16",
            "  o       17",
            "o         18",
            "o         19",
            "│         [M6]",
            "o         20",
        ];
        assert_eq!(lane_text(&model), g2.to_vec());
    }

    #[test]
    fn empty_bands_never_draw() {
        for model in [
            build(SENTRIQ, SENTRIQ_BANDS, &HashSet::new()),
            build(DAILY_VOW, DAILY_VOW_BANDS, &HashSet::new()),
        ] {
            let text = lane_text(&model);
            for line in &text {
                assert!(!line.contains("[Scope"), "{text:?}");
                assert!(!line.contains("[Requirement"), "{text:?}");
            }
            assert_eq!(band_rows(&model).len(), 1, "{text:?}");
        }
    }

    #[test]
    fn band_labels_appear_on_exactly_one_row() {
        for model in [
            build(BOOKLY, BOOKLY_BANDS, &HashSet::new()),
            build(DAILY_VOW, DAILY_VOW_BANDS, &HashSet::new()),
        ] {
            let text = lane_text(&model);
            assert!(!band_rows(&model).is_empty(), "{text:?}");
            for band in band_rows(&model) {
                let ListRow::Band { label, short, .. } = band else {
                    unreachable!()
                };
                let with_label = model
                    .rows
                    .iter()
                    .filter(|row| match row {
                        ListRow::Band { label: l, .. } => l == label,
                        ListRow::ShippedSummary { text, .. } => text.contains(label.as_str()),
                        ListRow::Connector { lanes, .. } | ListRow::Phase { lanes, .. } => {
                            lanes.contains(label.as_str())
                        }
                    })
                    .count();
                assert_eq!(with_label, 1, "{label}");
                let tag = format!("[{short}]");
                assert_eq!(text.iter().filter(|l| l.contains(&tag)).count(), 1);
            }
        }
    }

    // ----- robustness -------------------------------------------------------

    /// Each phase index appears in exactly one visible phase row.
    fn assert_each_phase_once(model: &RoadmapModel) {
        let mut seen: Vec<usize> = model
            .rows
            .iter()
            .filter_map(|row| match row {
                ListRow::Phase { node, .. } => Some(*node),
                _ => None,
            })
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..model.phases.len()).collect::<Vec<_>>());
    }

    /// `count` phases `1..=count`; `chained` makes each depend on the last.
    fn generated(count: usize, chained: bool) -> RoadmapModel {
        let ids: Vec<String> = (1..=count).map(|i| i.to_string()).collect();
        let deps: Vec<Vec<String>> = (1..=count)
            .map(|i| {
                if chained && i > 1 {
                    vec![(i - 1).to_string()]
                } else {
                    Vec::new()
                }
            })
            .collect();
        let input = ListInput {
            nodes: ids
                .iter()
                .zip(&deps)
                .map(|(id, deps)| ListNode {
                    id,
                    name: "x",
                    deps,
                    band: None,
                    marker: F,
                    done: false,
                    plans: None,
                    goal: None,
                    planned: false,
                    badge: None,
                })
                .collect(),
            bands: Vec::new(),
        };
        layout_list(&input, &HashSet::new())
    }

    fn phase_lanes(model: &RoadmapModel) -> Vec<usize> {
        model
            .rows
            .iter()
            .filter_map(|row| match row {
                ListRow::Phase { lane, .. } => Some(*lane),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn list_cycle_terminates_with_one_note() {
        let model = plain(&[
            ("1", "", &["2"], F),
            ("2", "", &["1"], F),
            ("3", "", &["5"], F),
            ("4", "", &["3"], F),
            ("5", "", &["4"], F),
        ]);
        assert_each_phase_once(&model);
        assert_eq!(model.notes.len(), 1, "{:?}", model.notes);
        assert!(
            model.notes[0].starts_with("dependency cycles: 2"),
            "{:?}",
            model.notes
        );
    }

    /// WR-01 on the list model (ported from the removed legacy layout's
    /// `roadmap_graph_wr01_cycle_notes_collapse_into_one_line`): many broken
    /// cycles collapse into ONE bounded note.
    #[test]
    fn list_many_cycles_collapse_into_one_bounded_note() {
        let model = plain(&[
            ("1", "", &["2"], F),
            ("2", "", &["1"], F),
            ("3", "", &["4"], F),
            ("4", "", &["3"], F),
            ("5", "", &["6"], F),
            ("6", "", &["5"], F),
            ("7", "", &["8"], F),
            ("8", "", &["7"], F),
            ("9", "", &["9", "99"], F),
        ]);
        assert_each_phase_once(&model);
        assert_eq!(model.notes.len(), 1, "{:?}", model.notes);
        let note = &model.notes[0];
        assert!(note.starts_with("dependency cycles: 5 (edges ignored): "), "{note}");
        assert!(note.ends_with("; +2 more"), "{note}");
        assert_eq!(facts(&model, "9").external, vec!["99".to_string()]);

        assert_eq!(cycle_note(&[]), None);
        assert_eq!(
            cycle_note(&["1, 2".to_string(), "3".to_string()]).as_deref(),
            Some("dependency cycles: 2 (edges ignored): 1, 2; 3")
        );
    }

    #[test]
    fn list_self_dependency_is_noted() {
        let model = plain(&[("1", "", &["1"], F)]);
        assert_eq!(lane_text(&model), vec!["o         1"]);
        assert_eq!(model.notes, vec!["dependency cycle: 1 (edges ignored)"]);
        assert!(facts(&model, "1").needs.is_empty());
    }

    #[test]
    fn list_external_dep_is_listed_not_drawn() {
        let model = plain(&[("14", "", &["7"], F), ("15", "", &["14"], F)]);
        assert_eq!(lane_text(&model), vec!["o         14", "o         15"]);
        assert_eq!(facts(&model, "14").external, vec!["7".to_string()]);
        assert!(!facts(&model, "14").no_deps);
        // An external dependency counts as satisfied (RESEARCH A12).
        assert_eq!(facts(&model, "14").status, PhaseStatus::Ready);
        assert_eq!(facts(&model, "15").status, PhaseStatus::Blocked);
    }

    #[test]
    fn list_padded_and_decimal_ids_match() {
        let model = plain(&[
            ("6", "", &[], F),
            ("07", "", &["6"], F),
            ("07.1", "", &["07"], F),
        ]);
        assert_eq!(ids(&model, &facts(&model, "07").needs), vec!["6"]);
        assert_eq!(ids(&model, &facts(&model, "07.1").needs), vec!["07"]);
        assert_eq!(
            lane_text(&model),
            vec!["o         6", "o         07", "o         07.1"]
        );
        assert_eq!(model.phase_index("7"), Some(1));
        assert_eq!(model.phase_index("07.1"), Some(2));
    }

    #[test]
    fn list_skip_layer_edge_is_implied_not_repeated() {
        let model = plain(&[
            ("1", "", &[], F),
            ("2", "", &["1"], F),
            ("3", "", &["2", "1"], F),
        ]);
        assert_eq!(
            lane_text(&model),
            vec!["o         1", "o         2", "o         3"]
        );
        assert_eq!(implied_ids(&model, "3"), pairs(&[("1", "2")]));
        assert_each_phase_once(&model);
    }

    #[test]
    fn list_fan_in_merges_once() {
        let model = plain(&[
            ("1", "", &[], F),
            ("2", "", &[], F),
            ("3", "", &[], F),
            ("4", "", &["1", "2", "3"], F),
        ]);
        assert_eq!(
            lane_text(&model),
            vec![
                "o         1",
                "│ o       2",
                "│ │ o     3",
                "├─┴─╯",
                "o         4",
            ]
        );
        let connectors = model
            .rows
            .iter()
            .filter(|row| matches!(row, ListRow::Connector { .. }))
            .count();
        assert_eq!(connectors, 1);
    }

    // -----------------------------------------------------------------------
    // Lane links (quick 261006-ujx, I-2, I-10)
    // -----------------------------------------------------------------------

    /// A row's lane string and its per-char links.
    fn lanes_and_links(row: &ListRow) -> (&str, &[Option<LaneLink>]) {
        match row {
            ListRow::ShippedSummary { lanes, links, .. }
            | ListRow::Band { lanes, links, .. }
            | ListRow::Connector { lanes, links }
            | ListRow::Phase { lanes, links, .. } => (lanes, links),
        }
    }

    /// The single connector row of `model`.
    fn only_connector(model: &RoadmapModel) -> &ListRow {
        let mut it = model
            .rows
            .iter()
            .filter(|row| matches!(row, ListRow::Connector { .. }));
        let row = it.next().expect("a connector row");
        assert!(it.next().is_none(), "{:#?}", lane_text(model));
        row
    }

    /// The phase row of `id`.
    fn phase_row<'m>(model: &'m RoadmapModel, id: &str) -> &'m ListRow {
        let node = idx(model, id);
        model
            .rows
            .iter()
            .find(|row| matches!(row, ListRow::Phase { node: n, .. } if *n == node))
            .unwrap_or_else(|| panic!("no row for {id}"))
    }

    fn link(model: &RoadmapModel, owner: &str, target: &str) -> Option<LaneLink> {
        Some(LaneLink {
            owner: idx(model, owner),
            target: idx(model, target),
        })
    }

    #[test]
    fn lane_links_match_lane_chars_one_to_one() {
        let models = [
            plain(DAILY_VOW),
            plain(SENTRIQ),
            plain(TTBOOK),
            build(BOOKLY, BOOKLY_BANDS, &HashSet::new()),
            build(
                BOOKLY,
                BOOKLY_BANDS,
                &toggles(&[BandKey::Named("m4 support chat".to_string())]),
            ),
            build(CROSS_FOLD, CROSS_FOLD_BANDS, &HashSet::new()),
            build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[BandKey::Shipped])),
            build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[v3_key()])),
        ];
        for model in &models {
            for row in &model.rows {
                let (lanes, links) = lanes_and_links(row);
                assert_eq!(links.len(), lanes.chars().count(), "{lanes:?}");
                // A blank never carries an edge.
                for (c, l) in lanes.chars().zip(links) {
                    if c == ' ' {
                        assert_eq!(*l, None, "{lanes:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn lane_links_name_the_edge_each_cell_draws() {
        // DAILY_VOW: 21 forks to 22 (lane 0) and 23 (lane 1).
        let model = plain(DAILY_VOW);
        let (lanes, links) = lanes_and_links(only_connector(&model));
        assert_eq!(lanes.chars().count(), 3, "{lanes:?}");
        assert_eq!(links[0], link(&model, "21", "22"));
        assert_eq!(links[1], link(&model, "21", "23"));
        assert_eq!(links[2], link(&model, "21", "23"));
        let (lanes, links) = lanes_and_links(phase_row(&model, "22"));
        assert_eq!(lanes.chars().nth(2), Some('\u{2502}'));
        assert_eq!(links[0], None, "the node's own char");
        assert_eq!(links[2], link(&model, "21", "23"));

        // The `1,2,3 -> 4` merge: every drawn char runs to 4, and each `─`
        // separator belongs to the lane beyond it.
        let model = plain(&[
            ("1", "", &[], F),
            ("2", "", &[], F),
            ("3", "", &[], F),
            ("4", "", &["1", "2", "3"], F),
        ]);
        let (lanes, links) = lanes_and_links(only_connector(&model));
        assert_eq!(lanes.chars().count(), 5, "{lanes:?}");
        for (c, l) in lanes.chars().zip(links) {
            if c != ' ' {
                assert_eq!(l.map(|k| k.target), Some(idx(&model, "4")), "{lanes:?}");
            }
        }
        assert_eq!(links[0], link(&model, "1", "4"));
        assert_eq!(links[1], link(&model, "2", "4"));
        assert_eq!(links[1], links[2]);
        assert_eq!(links[3], link(&model, "3", "4"));
        assert_eq!(links[3], links[4]);
    }

    #[test]
    fn a_leftward_fork_links_its_run_to_the_forked_lane() {
        let model = plain(&[
            ("1", "", &[], F),
            ("2", "", &[], F),
            ("3", "", &["2"], F),
            ("4", "", &["2"], F),
        ]);
        let text = lane_text(&model);
        assert_eq!(
            text,
            vec![
                "o         1",
                "  o       2",
                "\u{256D}\u{2500}\u{2524}",
                "│ o       3",
                "o         4",
            ]
        );
        let (_, links) = lanes_and_links(only_connector(&model));
        assert_eq!(links[0], link(&model, "2", "4"));
        assert_eq!(links[1], link(&model, "2", "4"));
        assert_eq!(links[2], link(&model, "2", "3"));
    }

    #[test]
    fn lane_corners_are_rounded() {
        assert_eq!(LANE_DOWN_RIGHT, "\u{256D}");
        assert_eq!(LANE_DOWN_LEFT, "\u{256E}");
        assert_eq!(LANE_UP_LEFT, "\u{256F}");
        assert_eq!(LANE_UP_RIGHT, "\u{2570}");
        assert_eq!(junction(false, true, false, true), LANE_DOWN_RIGHT);
        assert_eq!(junction(true, false, false, true), LANE_DOWN_LEFT);
        assert_eq!(junction(true, false, true, false), LANE_UP_LEFT);
        assert_eq!(junction(false, true, true, false), LANE_UP_RIGHT);
        // The tees, the cross and the straights keep their code points.
        assert_eq!(LANE_TEE_RIGHT, "\u{251C}");
        assert_eq!(LANE_TEE_LEFT, "\u{2524}");
        assert_eq!(LANE_TEE_DOWN, "\u{252C}");
        assert_eq!(LANE_TEE_UP, "\u{2534}");
        assert_eq!(LANE_CROSS, "\u{253C}");
        assert_eq!(LANE_VERTICAL, "\u{2502}");
        assert_eq!(LANE_HORIZONTAL, "\u{2500}");
    }

    #[test]
    fn no_laid_lane_uses_a_square_corner() {
        const SQUARE: [char; 4] = ['\u{250C}', '\u{2510}', '\u{2514}', '\u{2518}'];
        let models = [
            plain(DAILY_VOW),
            plain(SENTRIQ),
            plain(TTBOOK),
            build(BOOKLY, BOOKLY_BANDS, &HashSet::new()),
            build(CROSS_FOLD, CROSS_FOLD_BANDS, &HashSet::new()),
            build(CROSS_FOLD, CROSS_FOLD_BANDS, &toggles(&[BandKey::Shipped])),
            plain(&[
                ("1", "", &[], F),
                ("2", "", &[], F),
                ("3", "", &[], F),
                ("4", "", &["1", "2", "3"], F),
            ]),
            plain(&[
                ("1", "", &[], F),
                ("2", "", &[], F),
                ("3", "", &["2"], F),
                ("4", "", &["2"], F),
            ]),
        ];
        let mut corners = 0;
        for model in &models {
            for row in &model.rows {
                let (lanes, _) = lanes_and_links(row);
                assert!(!lanes.contains(SQUARE), "{lanes:?}");
                corners += lanes
                    .chars()
                    .filter(|c| "\u{256D}\u{256E}\u{256F}\u{2570}".contains(*c))
                    .count();
            }
        }
        assert!(corners > 0, "the fixtures draw rounded corners");
    }

    #[test]
    fn list_large_chain_stays_one_lane() {
        let model = generated(300, true);
        assert_eq!(model.rows.len(), 300);
        assert!(phase_lanes(&model).iter().all(|&l| l == 0));
        assert_eq!(model.max_wave, 300);
        assert_each_phase_once(&model);
    }

    #[test]
    fn list_many_roots_zig_zag_within_two_lanes() {
        let model = generated(60, false);
        let lanes = phase_lanes(&model);
        assert_eq!(lanes.len(), 60);
        for (i, lane) in lanes.iter().enumerate() {
            assert_eq!(*lane, i % 2, "row {i}");
        }
        assert_eq!(model.rows.len(), 60);
    }

    // ----- navigation -------------------------------------------------------

    fn phase(key: &str) -> CursorTarget {
        CursorTarget::Phase(key.to_string())
    }

    fn named(key: &str) -> CursorTarget {
        CursorTarget::Band(BandKey::Named(key.to_string()))
    }

    fn mockup_a() -> RoadmapModel {
        build(BOOKLY, BOOKLY_BANDS, &HashSet::new())
    }

    fn mockup_b() -> RoadmapModel {
        build(DAILY_VOW, DAILY_VOW_BANDS, &HashSet::new())
    }

    #[test]
    fn default_cursor_is_the_active_phase() {
        assert_eq!(mockup_a().resolve_cursor(None), Some(phase("10")));
        assert_eq!(mockup_b().resolve_cursor(None), Some(phase("23")));
        // An unknown stored key falls back to the default.
        assert_eq!(
            mockup_a().resolve_cursor(Some(&phase("99"))),
            Some(phase("10"))
        );
        // A known visible phase stays where it is.
        assert_eq!(
            mockup_a().resolve_cursor(Some(&phase("14"))),
            Some(phase("14"))
        );
        // No active phase: the first phase that is not done.
        let model = plain(&[("1", "", &[], D), ("2", "", &["1"], F)]);
        assert_eq!(model.resolve_cursor(None), Some(phase("2")));
        // Everything done: the first visible target.
        let model = plain(&[("1", "", &[], D)]);
        assert_eq!(model.resolve_cursor(None), Some(phase("1")));
        assert_eq!(RoadmapModel::default().resolve_cursor(None), None);
    }

    #[test]
    fn step_skips_connector_rows() {
        let model = mockup_a();
        let targets = model.visible_targets();
        let connectors = model
            .rows
            .iter()
            .filter(|row| matches!(row, ListRow::Connector { .. }))
            .count();
        assert_eq!(targets.len(), model.rows.len() - connectors);
        assert_eq!(targets[0], named("m3 live booking"));
        assert_eq!(model.first_target(), Some(named("m3 live booking")));
        assert_eq!(model.last_target(), Some(phase("18")));

        // 9 → 10 across the fork connector, and back.
        assert_eq!(model.step(&phase("9"), 1), phase("10"));
        assert_eq!(model.step(&phase("10"), -1), phase("9"));
        assert_eq!(model.step(&phase("11"), 1), phase("12"));
        // Band rows are targets; the ends clamp.
        assert_eq!(model.step(&phase("15"), 1), named("m4 support chat"));
        assert_eq!(model.step(&phase("8"), -1), named("m3 live booking"));
        assert_eq!(
            model.step(&named("m3 live booking"), -1),
            named("m3 live booking")
        );
        assert_eq!(model.step(&phase("17"), 5), phase("18"));

        assert_eq!(model.row_of(&phase("8")), Some(1));
        assert_eq!(model.row_of(&phase("10")), Some(4));
        assert_eq!(model.row_of(&named("m5 web")), Some(16));
    }

    /// Quick 260926-kes (D-01): the mouse's row -> target mapping is the one
    /// `target_of` rule — bands, the shipped summary and phases map to their
    /// target, connectors and rows past the end to none.
    #[test]
    fn mouse_target_at_maps_rows_and_skips_connectors() {
        let (mut bands, mut shipped, mut phases, mut connectors) = (0, 0, 0, 0);
        for model in [mockup_a(), mockup_b()] {
            for (i, row) in model.rows.iter().enumerate() {
                let got = model.target_at(i);
                match row {
                    ListRow::ShippedSummary { .. } => {
                        shipped += 1;
                        assert_eq!(got, Some(CursorTarget::Band(BandKey::Shipped)));
                    }
                    ListRow::Band { key, .. } => {
                        bands += 1;
                        assert_eq!(got, Some(CursorTarget::Band(key.clone())));
                    }
                    ListRow::Phase { node, .. } => {
                        phases += 1;
                        assert_eq!(got, Some(phase(&model.phases[*node].key)));
                    }
                    ListRow::Connector { .. } => {
                        connectors += 1;
                        assert_eq!(got, None, "row {i} is a connector");
                    }
                }
                if let Some(t) = &got {
                    assert_eq!(model.row_of(t), Some(i), "target_at and row_of agree");
                }
            }
            assert_eq!(model.target_at(model.rows.len()), None, "past the end");
            assert_eq!(model.target_at(usize::MAX), None);
        }
        assert!(bands > 0 && shipped > 0 && phases > 0 && connectors > 0,
            "the fixtures cover every row kind: {bands} {shipped} {phases} {connectors}");
    }

    #[test]
    fn h_cycles_the_origin_needs_including_implied() {
        let model = mockup_b();
        let (t1, w1) = model
            .edge_jump(&phase("23"), None, EdgeDir::Needs)
            .expect("23 has needs");
        assert_eq!(t1, phase("21"));
        let (t2, w2) = model
            .edge_jump(&t1, Some(&w1), EdgeDir::Needs)
            .expect("cycle");
        assert_eq!(t2, phase("20"));
        assert_eq!(w2.origin, "23");
        let (t3, _) = model
            .edge_jump(&t2, Some(&w2), EdgeDir::Needs)
            .expect("wrap");
        assert_eq!(t3, phase("21"));

        // A walk whose target is not the cursor restarts at the cursor.
        let (t, w) = model
            .edge_jump(&phase("22"), Some(&w2), EdgeDir::Needs)
            .expect("22 needs 21");
        assert_eq!((t, w.origin.as_str()), (phase("21"), "22"));
        // Only an external dependency: nothing to jump to.
        assert_eq!(model.edge_jump(&phase("18"), None, EdgeDir::Needs), None);
        // A band row has no edges.
        assert_eq!(
            model.edge_jump(&CursorTarget::Band(BandKey::Shipped), None, EdgeDir::Needs),
            None
        );
    }

    #[test]
    fn l_cycles_unblocks() {
        let model = mockup_a();
        let (t1, w1) = model
            .edge_jump(&phase("12"), None, EdgeDir::Unblocks)
            .expect("12 unblocks");
        assert_eq!(t1, phase("13"));
        let (t2, w2) = model
            .edge_jump(&t1, Some(&w1), EdgeDir::Unblocks)
            .expect("cycle");
        assert_eq!(t2, phase("18"));
        let (t3, _) = model
            .edge_jump(&t2, Some(&w2), EdgeDir::Unblocks)
            .expect("wrap");
        assert_eq!(t3, phase("13"));
        // A walk in the other direction restarts at the cursor.
        let (t, _) = model
            .edge_jump(&t2, Some(&w2), EdgeDir::Needs)
            .expect("18 needs 12");
        assert_eq!(t, phase("12"));
        assert_eq!(model.edge_jump(&phase("15"), None, EdgeDir::Unblocks), None);
    }

    #[test]
    fn bracket_steps_within_the_wave_and_wraps() {
        let model = mockup_a();
        assert_eq!(model.wave_step(&phase("13"), true), Some(phase("18")));
        assert_eq!(model.wave_step(&phase("18"), true), Some(phase("13")));
        assert_eq!(model.wave_step(&phase("13"), false), Some(phase("18")));
        assert_eq!(model.wave_step(&phase("10"), true), Some(phase("11")));
        // 12 is alone in wave 4.
        assert_eq!(model.wave_step(&phase("12"), true), None);
        assert_eq!(model.wave_step(&named("m3 live booking"), true), None);
    }

    /// Phase 1 in a shipped band, phase 2 in the open one.
    const SHIPPED_SPEC: LSpec<'static> = &[("1", "", &[], D), ("2", "", &["1"], C)];
    const SHIPPED_BANDS: BSpec<'static> =
        &[("v1.0 MVP", true, 1, &["1"]), ("v2 Next", false, 1, &["2"])];

    #[test]
    fn a_folded_phase_resolves_to_its_band_row() {
        let m4 = BandKey::Named("m4 support chat".to_string());
        let model = build(BOOKLY, BOOKLY_BANDS, &toggles(std::slice::from_ref(&m4)));
        assert_eq!(model.row_of(&phase("16")), None);
        assert_eq!(
            model.resolve_cursor(Some(&phase("16"))),
            Some(CursorTarget::Band(m4))
        );
        // A fold-hidden target is still a phase the model knows.
        assert_eq!(model.phase_index("16"), Some(8));

        // Under the folded shipped summary: the summary row.
        let model = build(SHIPPED_SPEC, SHIPPED_BANDS, &HashSet::new());
        assert_eq!(
            model.resolve_cursor(Some(&phase("1"))),
            Some(CursorTarget::Band(BandKey::Shipped))
        );
        assert_eq!(
            model.resolve_cursor(Some(&named("v1.0 mvp"))),
            Some(CursorTarget::Band(BandKey::Shipped))
        );
        assert_eq!(
            model.visible_targets()[0],
            CursorTarget::Band(BandKey::Shipped)
        );
    }

    #[test]
    fn unfold_for_reveals_a_hidden_phase() {
        let m4 = BandKey::Named("m4 support chat".to_string());
        let mut folds = toggles(&[m4]);
        let model = build(BOOKLY, BOOKLY_BANDS, &folds);
        model.unfold_for("16", &mut folds);
        assert!(folds.is_empty());
        let model = build(BOOKLY, BOOKLY_BANDS, &folds);
        assert!(model.row_of(&phase("16")).is_some());
        // Already visible: nothing changes.
        model.unfold_for("16", &mut folds);
        assert!(folds.is_empty());

        // A phase under the folded shipped summary opens the summary.
        let mut folds = HashSet::new();
        let model = build(SHIPPED_SPEC, SHIPPED_BANDS, &folds);
        model.unfold_for("1", &mut folds);
        assert_eq!(folds, toggles(&[BandKey::Shipped]));
        let model = build(SHIPPED_SPEC, SHIPPED_BANDS, &folds);
        assert_eq!(model.row_of(&phase("1")), Some(2));
    }

    // ----- escaping (T-24-06) -----------------------------------------------

    #[test]
    fn list_model_stores_only_escaped_text() {
        let id = "1\u{1b}[31m";
        let name = "Evil\u{1b}[31m name \u{202E}rtl";
        let goal_raw = "Goal\u{1b}]0;title\u{7} \u{202E}x";
        let label = "v9 Bad\u{1b}[2J \u{202E}band";
        let shipped_label = "v8\u{1b}[1m Old \u{202E}one";
        let dep = "9\u{1b}[2J";
        let badge = "[stage\u{1b}[5m]".to_string();
        let goal = crate::text::Untrusted::from_untrusted_source(goal_raw.to_string());
        let deps = vec![dep.to_string()];
        let deps2 = vec![id.to_string(), "2\u{202E}".to_string()];
        let input = ListInput {
            nodes: vec![
                ListNode {
                    id,
                    name,
                    deps: &deps,
                    band: Some(1),
                    marker: C,
                    done: false,
                    plans: Some((1, 2)),
                    goal: Some(&goal),
                    planned: false,
                    badge: Some(badge.clone()),
                },
                ListNode {
                    id: "2\u{202E}",
                    name,
                    deps: &deps2,
                    band: Some(0),
                    marker: F,
                    done: false,
                    plans: None,
                    goal: None,
                    planned: true,
                    badge: None,
                },
            ],
            bands: vec![
                BandInput {
                    label: crate::text::Untrusted::from_untrusted_source(shipped_label.to_string()),
                    shipped: true,
                    declared_phases: 3,
                },
                BandInput {
                    label: crate::text::Untrusted::from_untrusted_source(label.to_string()),
                    shipped: false,
                    declared_phases: 1,
                },
            ],
        };
        let model = layout_list(&input, &toggles(&[BandKey::Shipped]));

        // Every DISPLAY string; `key`/`BandKey` are logic-only and never drawn.
        let mut shown: Vec<String> = lane_text(&model);
        shown.extend(model.notes.iter().cloned());
        for row in &model.rows {
            match row {
                ListRow::ShippedSummary { text, lanes, .. } => {
                    shown.extend([text.clone(), lanes.clone()])
                }
                ListRow::Band {
                    label,
                    short,
                    lanes,
                    ..
                } => shown.extend([label.clone(), short.clone(), lanes.clone()]),
                ListRow::Connector { lanes, .. } | ListRow::Phase { lanes, .. } => {
                    shown.push(lanes.clone())
                }
            }
        }
        for p in &model.phases {
            shown.extend([p.id.clone(), p.name.clone()]);
            shown.extend(p.goal.iter().cloned());
            shown.extend(p.badge.iter().cloned());
            shown.extend(p.external.iter().cloned());
        }
        for b in &model.bands {
            shown.extend([b.label.clone(), b.short.clone()]);
        }
        for text in &shown {
            assert!(
                !text.contains('\u{1b}') && !text.contains('\u{202E}') && !text.contains('\u{7}'),
                "raw control reached {text:?}"
            );
        }

        let r = |s: &str| String::from(crate::text::render_for_terminal(s));
        let p = &model.phases[0];
        assert_eq!(p.id, r(id));
        assert_eq!(p.name, r(name));
        assert_eq!(p.goal.as_deref(), Some(r(goal_raw).as_str()));
        assert_eq!(p.badge.as_deref(), Some(r(&badge).as_str()));
        assert_eq!(p.external, vec![r(dep)]);
        assert_eq!(model.bands[1].label, r(label));
        assert_eq!(model.bands[0].label, r(shipped_label));
        // Matching still works on the raw id: 2 depends on 1.
        assert_eq!(ids(&model, &model.phases[1].needs), vec![r(id)]);
        assert!(lane_text(&model).iter().any(|l| l.ends_with(&r(id))));
    }
}
