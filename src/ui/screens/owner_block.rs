//! The Owner block of the detail Phases tab (36-03, D-15..D-19).
//!
//! A borderless, full-width block above the phase list: a header rule line with
//! the `closes after` gate, one row per current group the owner must act on, and
//! one pinned fold line (Old debt, Acknowledged, Todos). Pure and in-memory:
//! [`block_model`] turns an [`OwnerDigest`] into lines for one width, so nothing
//! on the render path reads a file. 36-04 adds focus, expansion, open folds and
//! the cursor: their state lives per project in [`OwnerBlockState`] as
//! identities ([`GroupKey`], [`Fold`], [`Stop`]), never indices, so it survives a
//! digest refresh (D-18).
//!
//! Every digest string reaches a cell through [`shown`] (the reader does not
//! escape), and every width is measured in cells with `Line::width()` and
//! `fit_cells`, never `len()`.

use crate::inbox::{self, ClauseForm};
use crate::state_reader::owner_items::{
    self, label_counts, labels_of, triage_entries, ClosesAfter, Kind, Lane, OwnerDigest,
    OwnerGroup, OwnerItem, Scope, TriageEntry,
};
use crate::ui::glyphs;
use crate::ui::screens::owner::OWNER_SEP;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::collections::HashSet;

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

/// Most item lines an expanded group shows before `... N more` (D-18).
const MAX_ITEMS: usize = 5;
/// Most gap lines an item shows before `... N more gaps` (D-18).
const MAX_GAPS: usize = 2;

/// What identifies a group across digest refreshes (D-18): an owned identity,
/// because `OwnerGroup` derives no `Hash` and an index would drift.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupKey {
    pub lane: &'static str,
    pub kind: &'static str,
    pub scope_label: Option<String>,
    pub workstream: Option<String>,
    pub phase: String,
    pub file: String,
}

impl GroupKey {
    /// The key of `group`; public so integration tests resolve a group's key
    /// through the product (39-03).
    pub fn of(group: &OwnerGroup) -> Self {
        Self {
            lane: group.lane.as_str(),
            kind: group.kind.as_str(),
            scope_label: scope_label(&group.scope),
            workstream: group.workstream.clone(),
            phase: group.phase.clone(),
            file: group.file.clone(),
        }
    }

    /// A same-phase merged row (D-08): one per (workstream, phase, lane) over its
    /// files, so its `file` is empty. [`group_for`] never resolves it (kind
    /// `entry` matches no group); a lookup goes through [`entry_for`].
    pub(crate) fn entry(first: &OwnerGroup) -> Self {
        Self {
            kind: "entry",
            file: String::new(),
            ..Self::of(first)
        }
    }

    /// A BLOCKED aggregate row: one per (workstream, phase), over all its files.
    pub(crate) fn aggregate(group: &OwnerGroup) -> Self {
        Self {
            lane: "blocked",
            kind: "blocked",
            file: String::new(),
            ..Self::of(group)
        }
    }
}

/// The group with identity `key` (current, archived or acknowledged), cloned.
/// `None` for a BLOCKED row's key (the row aggregates several groups and has no
/// single one to run) and for an identity that has vanished from the digest.
pub(crate) fn group_for(digest: &OwnerDigest, key: &GroupKey) -> Option<OwnerGroup> {
    if key.lane == "blocked" {
        return None;
    }
    digest
        .groups
        .iter()
        .find(|group| GroupKey::of(group) == *key)
        .cloned()
}

/// The groups a copy of the row with identity `key` covers (37-07, D-09): the one
/// group of a plain row, every BLOCKED group of the (workstream, phase) for an
/// aggregate row, empty for a vanished identity.
pub(crate) fn groups_for_copy(digest: &OwnerDigest, key: &GroupKey) -> Vec<OwnerGroup> {
    if key.lane == "blocked" {
        return digest
            .groups
            .iter()
            .filter(|g| {
                is_owner_row(g)
                    && g.lane == Lane::Blocked
                    && g.workstream == key.workstream
                    && g.phase == key.phase
            })
            .cloned()
            .collect();
    }
    group_for(digest, key).into_iter().collect()
}

pub(crate) fn scope_label(scope: &Scope) -> Option<String> {
    match scope {
        Scope::Current => None,
        Scope::Archived { label, .. } => Some(label.clone()),
    }
}

/// The folds on the pinned fold line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fold {
    OldDebt,
    /// Phases older than `preferences.owner.fold_days` (D-09). Inbox only: the
    /// Owner block never offers it, so `selectable_folds` does not list it.
    Stale,
    Acknowledged,
    Todos,
}

/// One cursor stop: a group row, or the fold line as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stop {
    Row(GroupKey),
    FoldLine,
}

/// The per-project Owner block view state (D-18): identities, never indices, kept
/// in memory only in `ProjectViewCache.owner_block`.
#[derive(Debug, Clone, Default)]
pub struct OwnerBlockState {
    /// Groups whose item lines are shown.
    pub expanded: HashSet<GroupKey>,
    /// Open folds (Todos never opens).
    pub open_folds: HashSet<Fold>,
    /// The stop the cursor rests on.
    pub cursor: Option<Stop>,
    /// The fold the fold line's sub-cursor rests on.
    pub fold_cursor: Option<Fold>,
    /// The item sub-cursor inside the cursor row's expansion: an
    /// `OwnerItem::id` (identity, never an index). `None` while the cursor rests
    /// on the row itself (D-04).
    pub item_cursor: Option<String>,
    /// The cursor's last index, so a vanished stop clamps to its neighbor.
    pub cursor_index: usize,
}

/// The cursor as the renderer needs it: the stop index and, on the fold line, the
/// fold the sub-cursor rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor {
    pub stop: usize,
    pub fold: Option<Fold>,
    /// The `body` index of the selected item's first line (D-04).
    pub item_line: Option<usize>,
}

/// The block as lines for one width.
pub(crate) struct BlockModel {
    pub header: Line<'static>,
    /// The header picked for the width left beside the widest row-range hint
    /// (` 1-12 of 34`), drawn when body rows are clipped so the hint never cuts a
    /// rung mid-clause.
    pub header_hinted: Line<'static>,
    /// Group rows, their expansion lines, and the rows of open folds.
    pub body: Vec<Line<'static>>,
    pub fold_line: Option<Line<'static>>,
    /// The D-15 predicate: a group row or a fold has something to show.
    pub has_content: bool,
    /// Every cursor stop in order: the group rows, then the fold line.
    pub stops: Vec<Stop>,
    /// The `body` index of each stop's row (`None` for the pinned fold line).
    pub stop_lines: Vec<Option<usize>>,
    /// The selectable folds, in fold-line order.
    pub folds: Vec<Fold>,
    /// The index in `fold_line.spans` of each fold's text.
    pub fold_marks: Vec<(Fold, usize)>,
    /// The fold line and its marks picked for two cells less, drawn when the
    /// cursor is on it: the `> ` gutter then fits without cutting the tail (IN-02).
    pub cursor_fold_line: Option<(Line<'static>, Vec<(Fold, usize)>)>,
    /// Row key, item id and the `body` index of the first line of every drawn
    /// item, in drawn order (the item sub-cursor's stops, D-04).
    pub item_lines: Vec<(GroupKey, String, usize)>,
}

fn dim() -> Style {
    muted_style()
}

pub(crate) fn blanks(n: usize) -> String {
    " ".repeat(n)
}

pub(crate) fn cells(text: &str) -> usize {
    Span::raw(text).width()
}

/// `text` cut to `w` cells and padded with blanks to exactly `w` (or `w` blanks
/// in front when `right`).
pub(crate) fn pad(text: &str, w: usize, right: bool) -> String {
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
pub(crate) struct RowCells {
    pub(crate) lane: Lane,
    /// `ws {name} ` (Magenta) for a workstream group.
    pub(crate) ws_prefix: String,
    pub(crate) phase: String,
    pub(crate) name: String,
    pub(crate) checks: String,
    pub(crate) phrase: String,
    pub(crate) green_phrase: bool,
    pub(crate) bookkeeping: bool,
    pub(crate) age: Option<(String, bool)>,
    pub(crate) command: String,
    /// `2 files` and the label counts (D-08, D-11), DIM beside the phrase in the
    /// inbox; empty from `row_cells` and so always in the Owner block.
    pub(crate) extras: String,
}

impl RowCells {
    pub(crate) fn phase_cell_width(&self) -> usize {
        cells(&self.ws_prefix) + cells(&self.phase)
    }

    /// The narrowest phase cell that still shows the whole phase text, keeping at
    /// most `keep` cells of the workstream name (`phase_cell` cuts the name
    /// first). Never wider than `phase_cell_width`.
    pub(crate) fn phase_cell_floor(&self, keep: usize) -> usize {
        let name = self
            .ws_prefix
            .strip_prefix("ws ")
            .and_then(|p| p.strip_suffix(' '));
        match name {
            Some(name) => {
                let kept = cells(name).min(keep.max(2));
                (cells("ws ") + kept + 1 + cells(&self.phase)).min(self.phase_cell_width())
            }
            None => self.phase_cell_width(),
        }
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
    owner_items::is_owner_row(group)
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

pub(crate) fn row_cells(
    digest: &OwnerDigest,
    group: &OwnerGroup,
    merged: Option<&OwnerGroup>,
) -> RowCells {
    let blocked = group.lane == Lane::Blocked;
    let ws_prefix = group
        .workstream
        .as_deref()
        .map(|ws| format!("ws {} ", shown(ws)))
        .unwrap_or_default();
    let name = match merged.map(inbox::blocked_tally).filter(|t| !t.is_empty()) {
        Some(tally) => shown(&tally),
        None => shown(&group.phase_name),
    };
    let counted = matches!(group.kind, Kind::Uat | Kind::Verification)
        && !blocked
        && !group.bookkeeping_only
        && group.counts.checks > 0;
    let checks = if counted {
        inbox::checks_cell(group.counts.checks, 2)
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
    } else if group.lane == Lane::Fix
        && counted
        && group.counts.failed == 0
        && group.counts.unreadable == 0
    {
        // `inbox::detail` would only repeat the Checks column here: say what the
        // file reports instead.
        (shown(&group.file_status.replace('_', " ")), false)
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
        extras: String::new(),
    }
}

/// One body row's source groups (D-18): the group the row speaks for, the BLOCKED
/// aggregate over its siblings, and the distinct files behind it.
#[derive(Clone)]
pub(crate) struct RowSrc {
    pub(crate) group: OwnerGroup,
    pub(crate) merged: Option<OwnerGroup>,
    pub(crate) files: Vec<String>,
    pub(crate) key: GroupKey,
    /// The per-file groups of a same-phase merged entry (D-08), empty for every
    /// other row. `merged` keeps meaning the BLOCKED tally only.
    pub(crate) parts: Vec<OwnerGroup>,
}

impl RowSrc {
    pub(crate) fn single(group: &OwnerGroup) -> Self {
        let blocked = group.lane == Lane::Blocked;
        Self {
            group: group.clone(),
            merged: blocked.then(|| group.clone()),
            files: vec![group.file.clone()],
            key: GroupKey::of(group),
            parts: Vec::new(),
        }
    }

    fn items(&self) -> &[OwnerItem] {
        &self.merged.as_ref().unwrap_or(&self.group).items
    }

    pub(crate) fn cells(&self, digest: &OwnerDigest) -> RowCells {
        row_cells(digest, &self.group, self.merged.as_ref())
    }

    /// [`cells`](Self::cells) for the inbox: the same cells plus `extras`, the
    /// file count of a merged row and the label counts (D-08, D-11; labels never
    /// move a row).
    pub(crate) fn inbox_cells(&self, digest: &OwnerDigest) -> RowCells {
        let mut cells = self.cells(digest);
        let mut extras: Vec<String> = Vec::new();
        if self.parts.len() > 1 {
            extras.push(inbox::plural(self.parts.len(), "file", "files"));
        }
        let labels = label_counts(self.items(), self.group.lane).text();
        if !labels.is_empty() {
            extras.push(labels);
        }
        cells.extras = shown(&extras.join(OWNER_SEP));
        cells
    }
}

/// The current rows in D-17 order: TEST (closes-a-phase first), DECIDE, FIX, then
/// one BLOCKED row per (workstream, phase), each lane in digest order, root groups
/// before workstream groups.
pub(crate) fn current_sources(digest: &OwnerDigest) -> Vec<RowSrc> {
    let mut picked: Vec<RowSrc> = Vec::new();
    let mut blocked_seen: Vec<(Option<String>, String)> = Vec::new();
    for group in digest.groups.iter().filter(|g| is_owner_row(g)) {
        if group.lane == Lane::Blocked {
            let key = (group.workstream.clone(), group.phase.clone());
            if blocked_seen.contains(&key) {
                continue;
            }
            let parts: Vec<&OwnerGroup> = digest
                .groups
                .iter()
                .filter(|g| {
                    is_owner_row(g)
                        && g.lane == Lane::Blocked
                        && g.workstream == key.0
                        && g.phase == key.1
                })
                .collect();
            let mut merged = group.clone();
            merged.items = parts.iter().flat_map(|g| g.items.iter().cloned()).collect();
            let mut files: Vec<String> = Vec::new();
            for part in &parts {
                if !files.contains(&part.file) {
                    files.push(part.file.clone());
                }
            }
            blocked_seen.push(key);
            picked.push(RowSrc {
                group: group.clone(),
                merged: Some(merged),
                files,
                key: GroupKey::aggregate(group),
                parts: Vec::new(),
            });
        } else {
            picked.push(RowSrc::single(group));
        }
    }
    picked.sort_by_key(|src| {
        let g = &src.group;
        (
            g.lane.order(),
            u8::from(g.lane == Lane::Test && !g.closes_phase),
            u8::from(g.workstream.is_some()),
        )
    });
    picked
}

/// The key of the row `entry` is drawn as: the BLOCKED aggregate, a merged row's
/// `GroupKey::entry`, or the one group's own identity.
pub(crate) fn entry_key(entry: &TriageEntry) -> GroupKey {
    let first = &entry.parts[0];
    if first.lane == Lane::Blocked {
        GroupKey::aggregate(first)
    } else if entry.is_merged() {
        GroupKey::entry(first)
    } else {
        GroupKey::of(first)
    }
}

/// The rows of the TUI inbox (38, D-08/D-09), next to the frozen
/// [`current_sources`]: owner entries from [`triage_entries`], the model the CLI
/// uses. Unfolded entries come first, the Stale fold's second; both in D-17 order.
/// Pure and in-memory: built on the render path from the cached digest.
pub(crate) fn inbox_sources(digest: &OwnerDigest, fold_days: u32) -> (Vec<RowSrc>, Vec<RowSrc>) {
    let entries: Vec<TriageEntry> = triage_entries(digest, fold_days)
        .into_iter()
        .filter(|entry| is_owner_row(&entry.group))
        .collect();
    let mut halves: [Vec<RowSrc>; 2] = [Vec::new(), Vec::new()];
    for (half, folded) in halves.iter_mut().zip([false, true]) {
        let mut blocked_seen: Vec<(Option<String>, String)> = Vec::new();
        for entry in entries
            .iter()
            .filter(|e| e.folded == folded || e.group.lane == Lane::Blocked)
        {
            let group = &entry.group;
            if group.lane == Lane::Blocked {
                let key = (group.workstream.clone(), group.phase.clone());
                if blocked_seen.contains(&key) {
                    continue;
                }
                // A BLOCKED row stays the 36 aggregate over every file of its
                // (workstream, phase), whatever the age of each file: a merged
                // blocked entry contributes its parts. The row is one row with one
                // identity, so it sits in the Stale fold only when every sibling is
                // folded; one fresh sibling keeps the whole aggregate in its lane.
                let blocked_entries: Vec<&TriageEntry> = entries
                    .iter()
                    .filter(|e| {
                        e.group.lane == Lane::Blocked
                            && e.group.workstream == key.0
                            && e.group.phase == key.1
                    })
                    .collect();
                if blocked_entries.iter().all(|e| e.folded) != folded {
                    continue;
                }
                let siblings: Vec<&OwnerGroup> = blocked_entries
                    .iter()
                    .flat_map(|e| e.parts.iter())
                    .collect();
                let mut merged = group.clone();
                merged.items = siblings
                    .iter()
                    .flat_map(|g| g.items.iter().cloned())
                    .collect();
                let mut files: Vec<String> = Vec::new();
                for part in &siblings {
                    if !files.contains(&part.file) {
                        files.push(part.file.clone());
                    }
                }
                blocked_seen.push(key);
                half.push(RowSrc {
                    group: group.clone(),
                    merged: Some(merged),
                    files,
                    key: entry_key(entry),
                    parts: Vec::new(),
                });
            } else if entry.is_merged() {
                half.push(RowSrc {
                    group: group.clone(),
                    merged: None,
                    files: entry.files().iter().map(|f| (*f).to_owned()).collect(),
                    key: entry_key(entry),
                    parts: entry.parts.clone(),
                });
            } else {
                half.push(RowSrc::single(&entry.parts[0]));
            }
        }
        half.sort_by_key(|src| {
            let g = &src.group;
            (
                g.lane.order(),
                u8::from(g.lane == Lane::Test && !g.closes_phase),
                u8::from(g.workstream.is_some()),
            )
        });
    }
    let [rows, folded] = halves;
    (rows, folded)
}

/// The owner entry drawn as the row with identity `key`, looked up afresh. `None`
/// for a vanished identity.
pub(crate) fn entry_for(
    digest: &OwnerDigest,
    key: &GroupKey,
    fold_days: u32,
) -> Option<TriageEntry> {
    triage_entries(digest, fold_days)
        .into_iter()
        .filter(|entry| is_owner_row(&entry.group))
        .find(|entry| entry_key(entry) == *key)
}

pub(crate) fn archived_rank(group: &OwnerGroup) -> usize {
    match &group.scope {
        Scope::Archived { rank, .. } => *rank,
        Scope::Current => 0,
    }
}

/// The rows an open Old debt shows: the archived groups the inbox lists with
/// `--include archived`, by milestone rank, then lane.
pub(crate) fn old_debt_sources(digest: &OwnerDigest) -> Vec<RowSrc> {
    let filters = inbox::Filters {
        archived: true,
        ..Default::default()
    };
    let mut groups: Vec<&OwnerGroup> = digest
        .groups
        .iter()
        .filter(|g| matches!(g.scope, Scope::Archived { .. }) && filters.lists_group(g))
        .collect();
    groups.sort_by_key(|g| (archived_rank(g), g.lane.order()));
    groups.into_iter().map(RowSrc::single).collect()
}

/// The rows an open Acknowledged shows: the groups flagged `acknowledged`.
pub(crate) fn acknowledged_sources(digest: &OwnerDigest) -> Vec<RowSrc> {
    let filters = inbox::Filters {
        archived: true,
        acknowledged: true,
        ..Default::default()
    };
    let mut groups: Vec<&OwnerGroup> = digest
        .groups
        .iter()
        .filter(|g| g.acknowledged && filters.lists_group(g))
        .collect();
    groups.sort_by_key(|g| (archived_rank(g), g.lane.order()));
    groups.into_iter().map(RowSrc::single).collect()
}

/// The distinct source files behind the row with identity `key` (current, old
/// debt or acknowledged): one for a group, several for a BLOCKED aggregate. Empty
/// for an identity that has vanished.
pub(crate) fn files_for(digest: &OwnerDigest, key: &GroupKey) -> Vec<String> {
    current_sources(digest)
        .into_iter()
        .chain(old_debt_sources(digest))
        .chain(acknowledged_sources(digest))
        .find(|src| src.key == *key)
        .map(|src| src.files)
        .unwrap_or_default()
}

/// Archived plus acknowledged totals behind the Old debt fold (matches the CLI).
fn old_debt_total(digest: &OwnerDigest) -> usize {
    digest.archived.test + digest.archived.blocked + digest.archived.fix + digest.archived.decide
}

/// The selectable folds (D-19): Old debt when it has content, Acknowledged when
/// the digest carries acknowledged groups; Todos never.
pub(crate) fn selectable_folds(digest: &OwnerDigest) -> Vec<Fold> {
    let mut folds = Vec::new();
    if old_debt_total(digest) > 0 {
        folds.push(Fold::OldDebt);
    }
    if digest.groups.iter().any(|g| g.acknowledged) {
        folds.push(Fold::Acknowledged);
    }
    folds
}

/// The cursor stops and what the view state may keep, from the digest alone (no
/// width), so a key handler can use them without drawing.
pub(crate) struct Skeleton {
    pub stops: Vec<Stop>,
    pub folds: Vec<Fold>,
    /// Every group identity the digest carries, open fold or not.
    pub keys: HashSet<GroupKey>,
}

pub(crate) fn skeleton(digest: &OwnerDigest, view: &OwnerBlockState) -> Skeleton {
    let folds = selectable_folds(digest);
    let current = current_sources(digest);
    let old = old_debt_sources(digest);
    let ack = acknowledged_sources(digest);
    let mut keys: HashSet<GroupKey> = HashSet::new();
    let mut stops: Vec<Stop> = Vec::new();
    let mut shown_rows = |rows: &[RowSrc], open: bool| {
        for src in rows {
            keys.insert(src.key.clone());
            if open {
                stops.push(Stop::Row(src.key.clone()));
            }
        }
    };
    shown_rows(&current, true);
    shown_rows(&old, view.open_folds.contains(&Fold::OldDebt));
    shown_rows(&ack, view.open_folds.contains(&Fold::Acknowledged));
    if !folds.is_empty() {
        stops.push(Stop::FoldLine);
    }
    Skeleton { stops, folds, keys }
}

/// Column widths for one block width.
struct Grid {
    phase_w: usize,
    name_w: usize,
    phrase_w: usize,
    show_age: bool,
    command_w: usize,
    /// The checks column: `CHECKS_W`, or wider for a `100+ checks` row.
    checks_w: usize,
}

/// The least the command column may shrink to where the width shows it.
const COMMAND_FLOOR_W: usize = 12;

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
    let checks_w = longest(|r| cells(&r.checks)).max(CHECKS_W);
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
        2 + TAG_W + 1 + phase_w + 2 + 2 + checks_w + 2 + phrase_w + age_part
    };
    // The command column keeps a floor wherever it is shown, so a long phrase
    // cannot starve it to nothing.
    let command_floor = if command_nat > 0 {
        2 + command_nat.min(COMMAND_FLOOR_W)
    } else {
        0
    };
    let needed = NAME_MIN_W + command_floor;
    // The phrase gives way first (down to 12), then the phase cell (down to 5).
    while fixed(phase_w, phrase_w) + needed > width && phrase_w > 12 {
        phrase_w -= 1;
    }
    while fixed(phase_w, phrase_w) + needed > width && phase_w > 5 {
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
        checks_w,
    }
}

pub(crate) fn style_for(row: &RowCells, style: Style) -> Style {
    if row.lane == Lane::Blocked {
        style.add_modifier(Modifier::DIM)
    } else {
        style
    }
}

pub(crate) fn age_spans(age: &(String, bool), row: &RowCells) -> Vec<Span<'static>> {
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

/// The phase cell as (Magenta prefix, text) in `width` cells. A workstream name
/// gives way inside its prefix (`ws v0-14-v\u{2026} ph 26`) so the phase number,
/// which tells the rows of one workstream apart, is never the part that is cut.
pub(crate) fn phase_cell(row: &RowCells, width: usize) -> (String, String) {
    if cells(&row.ws_prefix) + cells(&row.phase) <= width {
        return (row.ws_prefix.clone(), row.phase.clone());
    }
    let name = row
        .ws_prefix
        .strip_prefix("ws ")
        .and_then(|p| p.strip_suffix(' '));
    if let Some(name) = name {
        // `ws `, the name, a space, then the phase text with its own lead space.
        let room = width.saturating_sub(cells("ws ") + 1 + cells(&row.phase));
        if room >= 2 {
            return (format!("ws {} ", fit_cells(name, room)), row.phase.clone());
        }
    }
    let cut = fit_cells(&format!("{}{}", row.ws_prefix, row.phase), width);
    match cut.strip_prefix(row.ws_prefix.as_str()) {
        Some(rest) => (row.ws_prefix.clone(), rest.to_owned()),
        None if row.ws_prefix.is_empty() => (String::new(), cut),
        None => (cut, String::new()),
    }
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
    let (prefix, rest) = phase_cell(row, grid.phase_w);
    let gap = blanks(grid.phase_w.saturating_sub(cells(&prefix) + cells(&rest)));
    if prefix.is_empty() {
        spans.push(Span::styled(format!("{rest}{gap}"), plain));
    } else {
        let magenta = style_for(row, Style::default().fg(Color::Magenta));
        spans.push(Span::styled(prefix, magenta));
        spans.push(Span::styled(format!("{rest}{gap}"), plain));
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
        spans.push(Span::styled(pad(&row.checks, grid.checks_w, true), plain));
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

pub(crate) fn acknowledged_files(digest: &OwnerDigest) -> usize {
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

/// The fold line's spans and, for each fold, the index of its text span.
type FoldSpans = (Vec<Span<'static>>, Vec<(Fold, usize)>);

fn fold_spans(parts: &[(Fold, String, bool)], view: &OwnerBlockState) -> FoldSpans {
    let mut spans = Vec::new();
    let mut marks = Vec::new();
    for (index, (fold, text, muted)) in parts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        let style = if *muted { dim() } else { Style::default() };
        let glyph = if view.open_folds.contains(fold) {
            glyphs::TRIANGLE_DOWN_SMALL
        } else {
            glyphs::TRIANGLE_RIGHT_SMALL
        };
        marks.push((*fold, spans.len()));
        spans.push(Span::styled(format!("{glyph} {text}"), style));
    }
    (spans, marks)
}

fn fold_line(
    digest: &OwnerDigest,
    view: &OwnerBlockState,
    width: usize,
) -> Option<(Line<'static>, Vec<(Fold, usize)>)> {
    let old = old_debt_total(digest);
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
        let mut parts: Vec<(Fold, String, bool)> = Vec::new();
        if old > 0 {
            parts.push((Fold::OldDebt, old_text, false));
        }
        if acknowledged > 0 {
            parts.push((Fold::Acknowledged, ack_text, false));
        }
        if todos_shown {
            parts.push((Fold::Todos, todo_text.to_owned(), true));
        }
        fold_spans(&parts, view)
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
    for (spans, marks) in &rungs {
        let line = Line::from(spans.clone());
        if line.width() <= width {
            return Some((line, marks.clone()));
        }
    }
    let (last, marks) = rungs.into_iter().last().unwrap_or_default();
    Some((fit_spans(last, width), marks))
}

// ---------------------------------------------------------------------------
// Expanded group lines (D-18)
// ---------------------------------------------------------------------------

/// One dimmed line indented `indent` cells, every string already escaped.
pub(crate) fn dim_line(indent: usize, text: &str, width: usize) -> Line<'static> {
    let text = format!("{}{text}", blanks(indent));
    Line::from(Span::styled(fit_cells(&text, width), dim()))
}

/// `{test} {name}`, or the name alone.
fn item_text(item: &OwnerItem) -> String {
    match item.test.as_deref().filter(|t| !t.trim().is_empty()) {
        Some(test) => format!("{} {}", shown(test), shown(&item.name)),
        None => shown(&item.name),
    }
}

/// `expected: .. . why: .. . reason: .. . blocked by: ..`, absent parts omitted.
fn detail_text(item: &OwnerItem) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut part = |label: &str, value: &Option<String>| {
        if let Some(value) = value.as_deref().filter(|v| !v.trim().is_empty()) {
            parts.push(format!("{label}: {}", shown(value)));
        }
    };
    part("expected", &item.expected);
    part("why", &item.why_human);
    part("reason", &item.reason);
    part("blocked by", &item.blocked_by);
    (!parts.is_empty()).then(|| parts.join(OWNER_SEP))
}

fn gap_text(gap: &crate::state_reader::owner_items::OwnerGap) -> String {
    let reason = gap
        .reason
        .as_deref()
        .filter(|r| !r.trim().is_empty())
        .map(|r| format!(": {}", shown(r)))
        .unwrap_or_default();
    format!("gap: {} ({}{reason})", shown(&gap.name), shown(&gap.result))
}

/// Up to five of `items` as lines, each with its detail and gap lines, then
/// `... N more`.
/// With `labeled` (the inbox, D-11) an item the matcher labels ends in a DIM
/// `  (decision?)`, the CLI's wording.
fn item_lines(
    items: &[OwnerItem],
    width: usize,
    shift: usize,
    labeled: Option<Lane>,
    lines: &mut Vec<Line<'static>>,
    ids: &mut Vec<(usize, SubStop)>,
) {
    for item in items.iter().take(MAX_ITEMS) {
        ids.push((lines.len(), SubStop::Item(item.id.clone())));
        let text = format!("{}{}", blanks(4 + shift), item_text(item));
        let labels: Vec<&str> = labeled
            .map(|lane| labels_of(item, lane))
            .unwrap_or_default()
            .into_iter()
            .map(owner_items::Label::as_str)
            .collect();
        if labels.is_empty() {
            lines.push(Line::from(Span::raw(fit_cells(&text, width))));
        } else {
            let suffix = format!("  ({})", labels.join(OWNER_SEP));
            let room = width.saturating_sub(cells(&suffix));
            let head = fit_cells(&text, room);
            let tail = fit_cells(&suffix, width.saturating_sub(cells(&head)));
            lines.push(Line::from(vec![Span::raw(head), Span::styled(tail, dim())]));
        }
        if let Some(detail) = detail_text(item) {
            lines.push(dim_line(7 + shift, &detail, width));
        }
        for gap in item.gaps.iter().take(MAX_GAPS) {
            lines.push(dim_line(7 + shift, &gap_text(gap), width));
        }
        if item.gaps.len() > MAX_GAPS {
            let more = item.gaps.len() - MAX_GAPS;
            lines.push(dim_line(
                7 + shift,
                &format!("\u{2026} {more} more gaps"),
                width,
            ));
        }
    }
    if items.len() > MAX_ITEMS {
        let more = items.len() - MAX_ITEMS;
        lines.push(dim_line(4 + shift, &format!("\u{2026} {more} more"), width));
    }
}

/// The lines under an expanded group row: up to five items, each with its detail
/// and gap lines, then `... N more`, then one `file` line per distinct file.
#[cfg(test)]
pub(crate) fn expansion_lines(src: &RowSrc, width: usize, shift: usize) -> Vec<Line<'static>> {
    expansion_lines_mapped(src, width, shift).0
}

/// [`expansion_lines`] plus the line index of every sub-stop it draws: each item
/// (the `... N more` line is never one) and each `file PATH` line, in drawn order
/// (quick 261118-u4e, I-1). The stops equal [`row_sub_stops`].
pub(crate) fn expansion_lines_mapped(
    src: &RowSrc,
    width: usize,
    shift: usize,
) -> (Vec<Line<'static>>, Vec<(usize, SubStop)>) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut stops: Vec<(usize, SubStop)> = Vec::new();
    item_lines(src.items(), width, shift, None, &mut lines, &mut stops);
    file_lines(src, width, shift, &mut lines, &mut stops);
    (lines, stops)
}

/// One `file PATH` line per non-empty file of `src`, each a [`SubStop::File`].
fn file_lines(
    src: &RowSrc,
    width: usize,
    shift: usize,
    lines: &mut Vec<Line<'static>>,
    stops: &mut Vec<(usize, SubStop)>,
) {
    for file in src.files.iter().filter(|f| !f.is_empty()) {
        stops.push((lines.len(), SubStop::File(file.clone())));
        lines.push(dim_line(4 + shift, &format!("file {}", shown(file)), width));
    }
}

/// The lines under an expanded inbox row plus the line index of every sub-stop
/// (item or `file PATH` line, in drawn order): a merged row (D-08) lists each
/// file's items under its own `file PATH` sub-heading (each part's items in part
/// order); every other row is [`expansion_lines_mapped`] with labels. The stops
/// equal [`inbox_sub_stops`].
pub(crate) fn inbox_expansion_lines_mapped(
    src: &RowSrc,
    width: usize,
    shift: usize,
) -> (Vec<Line<'static>>, Vec<(usize, SubStop)>) {
    let lane = Some(src.group.lane);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut stops: Vec<(usize, SubStop)> = Vec::new();
    if src.parts.len() < 2 {
        item_lines(src.items(), width, shift, lane, &mut lines, &mut stops);
        file_lines(src, width, shift, &mut lines, &mut stops);
        return (lines, stops);
    }
    for part in &src.parts {
        stops.push((lines.len(), SubStop::File(part.file.clone())));
        lines.push(dim_line(
            4 + shift,
            &format!("file {}", shown(&part.file)),
            width,
        ));
        item_lines(&part.items, width, shift + 2, lane, &mut lines, &mut stops);
    }
    (lines, stops)
}

/// The ids of the first [`MAX_ITEMS`] items of `src`: exactly the ids
/// [`expansion_lines_mapped`] draws.
pub(crate) fn row_item_ids(src: &RowSrc) -> Vec<String> {
    src.items()
        .iter()
        .take(MAX_ITEMS)
        .map(|item| item.id.clone())
        .collect()
}

/// The ids [`inbox_expansion_lines_mapped`] draws: a merged row lists each part's
/// first [`MAX_ITEMS`] ids in part order, every other row is [`row_item_ids`].
pub(crate) fn inbox_item_ids(src: &RowSrc) -> Vec<String> {
    if src.parts.len() < 2 {
        return row_item_ids(src);
    }
    src.parts
        .iter()
        .flat_map(|part| part.items.iter().take(MAX_ITEMS))
        .map(|item| item.id.clone())
        .collect()
}

/// The group and item behind the drawn check `id` of the row `src` (quick
/// 261118-u4e): the candidate groups in order are the merged row's `parts`, the
/// row's own group, then the digest's groups of the same lane, scope, workstream
/// and phase (a BLOCKED aggregate's siblings); the first holding an item with
/// that id wins. `None` when a refresh made the id vanish.
pub(crate) fn item_source(
    digest: Option<&OwnerDigest>,
    src: &RowSrc,
    id: &str,
) -> Option<(OwnerGroup, OwnerItem)> {
    candidate_groups(digest, src).find_map(|group| {
        let item = group.items.iter().find(|item| item.id == id)?;
        Some((group.clone(), item.clone()))
    })
}

/// The groups that can hold what row `src` draws, in lookup order: a merged
/// row's `parts`, the row's own group, then the digest's groups of the same
/// lane, scope, workstream and phase (a BLOCKED aggregate's siblings).
fn candidate_groups<'a>(
    digest: Option<&'a OwnerDigest>,
    src: &'a RowSrc,
) -> impl Iterator<Item = &'a OwnerGroup> {
    let head = &src.group;
    let siblings = digest
        .into_iter()
        .flat_map(|d| d.groups.iter())
        .filter(move |g| {
            g.lane == head.lane
                && g.scope == head.scope
                && g.workstream == head.workstream
                && g.phase == head.phase
        });
    src.parts
        .iter()
        .chain(std::iter::once(head))
        .chain(siblings)
}

/// The line index of `stop` in an `*_expansion_lines_mapped` map.
pub(crate) fn sub_line(map: &[(usize, SubStop)], stop: &SubStop) -> Option<usize> {
    map.iter().find(|(_, s)| s == stop).map(|(line, _)| *line)
}

/// A sub-cursor stop inside an expanded row (quick 261118-u4e, I-1): a drawn
/// check, or a `file PATH` line. Hosts keep one as two fields (`item_cursor`,
/// `file_cursor`, never both set); [`SubStop::of`] and [`SubStop::store`] are the
/// only code that combines the two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubStop {
    /// An `OwnerItem::id`.
    Item(String),
    /// A project-relative file path as the group carries it.
    File(String),
}

impl SubStop {
    /// The stop the two stored fields name; the item wins if a bug set both.
    pub(crate) fn of(item: &Option<String>, file: &Option<String>) -> Option<SubStop> {
        item.clone()
            .map(SubStop::Item)
            .or_else(|| file.clone().map(SubStop::File))
    }

    /// Store `to` in the two fields: it sets one and clears the other.
    pub(crate) fn store(to: Option<SubStop>, item: &mut Option<String>, file: &mut Option<String>) {
        *item = None;
        *file = None;
        match to {
            Some(SubStop::Item(id)) => *item = Some(id),
            Some(SubStop::File(path)) => *file = Some(path),
            None => {}
        }
    }
}

/// The sub-stops of an expanded inbox row, in drawn order: a row with fewer than
/// two parts lists its checks, then one file line per non-empty file; a merged
/// row lists, per part, its file line and then that part's first [`MAX_ITEMS`]
/// checks. Exactly the stops [`inbox_expansion_lines_mapped`] draws.
pub(crate) fn inbox_sub_stops(src: &RowSrc) -> Vec<SubStop> {
    if src.parts.len() < 2 {
        return inbox_item_ids(src)
            .into_iter()
            .map(SubStop::Item)
            .chain(
                src.files
                    .iter()
                    .filter(|f| !f.is_empty())
                    .cloned()
                    .map(SubStop::File),
            )
            .collect();
    }
    src.parts
        .iter()
        .flat_map(|part| {
            std::iter::once(SubStop::File(part.file.clone())).chain(
                part.items
                    .iter()
                    .take(MAX_ITEMS)
                    .map(|item| SubStop::Item(item.id.clone())),
            )
        })
        .collect()
}

/// One step of the sub-stop walk inside an expanded row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubStep {
    /// The sub-cursor lands on this stop.
    Stop(SubStop),
    /// Backward from the first stop: back onto the row itself.
    Parent,
    /// Past either end: the walk leaves the row for the next or previous one.
    Leave,
}

/// Where the sub-cursor goes from `current` (`None` on the row) within `stops`;
/// a stop that is not listed behaves as `None`.
pub(crate) fn sub_step(stops: &[SubStop], current: Option<&SubStop>, forward: bool) -> SubStep {
    let at = current.and_then(|stop| stops.iter().position(|s| s == stop));
    match (at, forward) {
        (None, true) => stops
            .first()
            .map_or(SubStep::Leave, |stop| SubStep::Stop(stop.clone())),
        (None, false) => SubStep::Leave,
        (Some(i), true) => stops
            .get(i + 1)
            .map_or(SubStep::Leave, |stop| SubStep::Stop(stop.clone())),
        (Some(0), false) => SubStep::Parent,
        (Some(i), false) => SubStep::Stop(stops[i - 1].clone()),
    }
}

/// The group behind the `file PATH` line `path` of the row `src` (quick
/// 261118-u4e): [`item_source`]'s candidate order, matching the group's file.
pub(crate) fn file_source(
    digest: Option<&OwnerDigest>,
    src: &RowSrc,
    path: &str,
) -> Option<OwnerGroup> {
    candidate_groups(digest, src)
        .find(|group| group.file == path)
        .cloned()
}

/// One step of the item walk inside an expanded row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemStep {
    /// The sub-cursor lands on this item.
    Item(String),
    /// Backward from the first item: back onto the row itself.
    Parent,
    /// Past either end: the walk leaves the row for the next or previous one.
    Leave,
}

/// Where the item sub-cursor goes from `current` (an id, or `None` on the row)
/// within `ids`; an id not listed behaves as `None`.
pub(crate) fn item_step(ids: &[String], current: Option<&str>, forward: bool) -> ItemStep {
    let at = current.and_then(|id| ids.iter().position(|i| i == id));
    match (at, forward) {
        (None, true) => ids
            .first()
            .map_or(ItemStep::Leave, |id| ItemStep::Item(id.clone())),
        (None, false) => ItemStep::Leave,
        (Some(i), true) => ids
            .get(i + 1)
            .map_or(ItemStep::Leave, |id| ItemStep::Item(id.clone())),
        (Some(0), false) => ItemStep::Parent,
        (Some(i), false) => ItemStep::Item(ids[i - 1].clone()),
    }
}

/// The source of the row with identity `key` (current, old debt or acknowledged).
fn row_src_for(digest: &OwnerDigest, key: &GroupKey) -> Option<RowSrc> {
    current_sources(digest)
        .into_iter()
        .chain(old_debt_sources(digest))
        .chain(acknowledged_sources(digest))
        .find(|src| src.key == *key)
}

// ---------------------------------------------------------------------------
// Model, height, render
// ---------------------------------------------------------------------------

/// Patch `style` onto every span of `line`.
fn patch_line(line: &mut Line<'static>, style: Style) {
    for span in &mut line.spans {
        span.style = span.style.patch(style);
    }
}

/// The block for one project at `width` cells, with the view state's expanded
/// groups and open folds applied.
pub(crate) fn block_model(
    alias: &str,
    milestone: &str,
    digest: &OwnerDigest,
    view: &OwnerBlockState,
    width: u16,
) -> BlockModel {
    let width = usize::from(width);
    let sk = skeleton(digest, view);
    let current = current_sources(digest);
    let old = if view.open_folds.contains(&Fold::OldDebt) {
        old_debt_sources(digest)
    } else {
        Vec::new()
    };
    let ack = if view.open_folds.contains(&Fold::Acknowledged) {
        acknowledged_sources(digest)
    } else {
        Vec::new()
    };
    // One grid over every row drawn, so open-fold rows align with the rest.
    let all: Vec<RowCells> = current
        .iter()
        .chain(&old)
        .chain(&ack)
        .map(|src| src.cells(digest))
        .collect();
    let grid = grid_for(&all, width);

    let mut body: Vec<Line<'static>> = Vec::new();
    let mut stop_lines: Vec<Option<usize>> = Vec::new();
    let mut item_lines: Vec<(GroupKey, String, usize)> = Vec::new();
    let mut push_rows = |srcs: &[RowSrc], cells: &[RowCells], sub_headers: bool, dimmed: bool| {
        let mut last_label: Option<Option<String>> = None;
        for (src, row) in srcs.iter().zip(cells) {
            if sub_headers {
                let label = scope_label(&src.group.scope);
                if last_label.as_ref() != Some(&label) {
                    let text = label.as_deref().map(shown).unwrap_or_default();
                    body.push(dim_line(2, &text, width));
                    last_label = Some(label);
                }
            }
            stop_lines.push(Some(body.len()));
            let mut line = row_line(row, &grid, width);
            if dimmed {
                patch_line(&mut line, Style::default().add_modifier(Modifier::DIM));
            }
            body.push(line);
            if view.expanded.contains(&src.key) {
                let base = body.len();
                let (extras, stops) = expansion_lines_mapped(src, width, 0);
                for (line, stop) in stops {
                    if let SubStop::Item(id) = stop {
                        item_lines.push((src.key.clone(), id, base + line));
                    }
                }
                for mut extra in extras {
                    if dimmed {
                        patch_line(&mut extra, Style::default().add_modifier(Modifier::DIM));
                    }
                    body.push(extra);
                }
            }
        }
    };
    let (cur_cells, rest) = all.split_at(current.len());
    let (old_cells, ack_cells) = rest.split_at(old.len());
    push_rows(&current, cur_cells, false, false);
    push_rows(&old, old_cells, true, false);
    push_rows(&ack, ack_cells, false, true);

    let cursor_fold_line = fold_line(digest, view, width.saturating_sub(CURSOR_GUTTER_CELLS));
    let (fold_line, fold_marks) = match fold_line(digest, view, width) {
        Some((line, marks)) => (Some(line), marks),
        None => (None, Vec::new()),
    };
    // The skeleton listed the rows in the order `push_rows` drew them, and the
    // fold line stop (always drawn when a fold is selectable) comes last.
    let stops = sk.stops;
    stop_lines.resize(stops.len(), None);
    let widest_hint = format!(" {0}-{0} of {0}", body.len());
    BlockModel {
        header: header_line(alias, milestone, &digest.closes_after, width),
        header_hinted: header_line(
            alias,
            milestone,
            &digest.closes_after,
            width.saturating_sub(cells(&widest_hint)),
        ),
        has_content: !body.is_empty() || fold_line.is_some(),
        body,
        fold_line,
        stops,
        stop_lines,
        folds: sk.folds,
        fold_marks,
        cursor_fold_line,
        item_lines,
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
    // In `usize`: `tab_height * 40` overflows `u16` past 1638 rows (IN-05).
    let cap = (usize::from(tab_height) * 40 / 100).max(4);
    u16::try_from(wanted.min(cap)).unwrap_or(u16::MAX)
}

/// Body rows visible in a block `height` rows tall: all but the header and the
/// pinned fold line.
pub(crate) fn window_rows(model: &BlockModel, height: u16) -> usize {
    let fold = usize::from(model.fold_line.is_some() && height > 1);
    usize::from(height).saturating_sub(1 + fold)
}

/// The scroll offset that keeps the cursor's row in view by whole rows, starting
/// from `offset`; the cursor row's expansion stays in view when it fits.
pub(crate) fn follow_offset(
    model: &BlockModel,
    cursor_stop: Option<usize>,
    offset: usize,
    window: usize,
) -> usize {
    let total = model.body.len();
    let max_offset = total.saturating_sub(window);
    let mut offset = offset.min(max_offset);
    let Some(line) = cursor_stop
        .and_then(|stop| model.stop_lines.get(stop).copied().flatten())
        .filter(|_| window > 0)
    else {
        return offset;
    };
    let next = model
        .stop_lines
        .iter()
        .flatten()
        .copied()
        .find(|&l| l > line);
    let end = next
        .map_or(total.saturating_sub(1), |n| n - 1)
        .min(line + window - 1);
    if line < offset {
        offset = line;
    } else if end >= offset + window {
        offset = end + 1 - window;
    }
    offset.min(max_offset)
}

/// Draw the block into `area` (its height is [`block_height`]): the header, the
/// body rows from `offset`, and the fold line pinned as the last row. When body
/// rows are clipped the header's right edge shows the DIM row range. The `cursor`
/// row (focus is Owner) shows `> ` in its gutter, BOLD and Cyan (D-17).
pub(crate) fn render_owner_block(
    frame: &mut Frame,
    area: Rect,
    model: &BlockModel,
    cursor: Option<Cursor>,
    offset: usize,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let fold = usize::from(model.fold_line.is_some() && area.height > 1);
    let window = window_rows(model, area.height);
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
            let header = fit_spans(
                model.header_hinted.spans.clone(),
                usize::from(parts[0].width),
            );
            frame.render_widget(Paragraph::new(header), parts[0]);
            frame.render_widget(Paragraph::new(Span::styled(hint, dim())), parts[1]);
        }
        _ => frame.render_widget(Paragraph::new(model.header.clone()), header_rect),
    }
    let cursor_line = cursor.and_then(|c| model.stop_lines.get(c.stop).copied().flatten());
    // D-04: a selected item is highlighted in place (no gutter), its row is not.
    let item_line = cursor.and_then(|c| c.item_line);
    if window > 0 {
        let rows: Vec<Line<'static>> = model
            .body
            .iter()
            .enumerate()
            .skip(offset)
            .take(window)
            .map(|(index, line)| {
                let mut line = line.clone();
                if item_line == Some(index) {
                    patch_line(&mut line, cursor_style());
                } else if cursor_line == Some(index) && item_line.is_none() {
                    mark_cursor_row(&mut line);
                }
                line
            })
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
        let on_fold_line = cursor.is_some_and(|c| model.stops.get(c.stop) == Some(&Stop::FoldLine));
        let line = if on_fold_line {
            fold_line_with_cursor(
                model,
                line,
                cursor.and_then(|c| c.fold),
                usize::from(area.width),
            )
        } else {
            line.clone()
        };
        frame.render_widget(Paragraph::new(line), rect);
    }
}

pub(crate) fn cursor_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
        .remove_modifier(Modifier::DIM)
}

/// The cursor row: `> ` in the gutter and the whole row BOLD plus Cyan.
fn mark_cursor_row(line: &mut Line<'static>) {
    patch_line(line, cursor_style());
    if let Some(first) = line.spans.first_mut() {
        if first.content.as_ref() == "  " {
            first.content = "> ".into();
        }
    }
}

/// The cells the fold line's `> ` cursor gutter takes.
const CURSOR_GUTTER_CELLS: usize = 2;

/// The fold line behind a cursor: a `> ` gutter in front and the fold under the
/// sub-cursor BOLD plus Cyan. The line keeps to `width` cells; its rung was picked
/// for `width - 2`, so the gutter does not push a fitting rung into `…` (IN-02).
fn fold_line_with_cursor(
    model: &BlockModel,
    line: &Line<'static>,
    fold: Option<Fold>,
    width: usize,
) -> Line<'static> {
    let (line, marks) = match &model.cursor_fold_line {
        Some((narrow, marks)) => (narrow, marks),
        None => (line, &model.fold_marks),
    };
    let mut spans = line.spans.clone();
    let target = fold.and_then(|f| {
        marks
            .iter()
            .find(|(candidate, _)| *candidate == f)
            .map(|(_, index)| *index)
    });
    if let Some(span) = target.and_then(|index| spans.get_mut(index)) {
        span.style = span.style.patch(cursor_style());
    }
    spans.insert(0, Span::styled("> ", cursor_style()));
    fit_spans(spans, width)
}

// ---------------------------------------------------------------------------
// Cursor and view-state operations (D-18, D-20)
// ---------------------------------------------------------------------------

/// A cursor movement over the stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Move {
    Next,
    Prev,
    First,
    Last,
    /// By this many stops, clamped to the ends.
    By(isize),
}

/// Where a [`Move`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Moved {
    /// The cursor is on a stop (it may not have changed).
    Within,
    /// `Next` from the last stop: the cursor stays and focus leaves downward.
    PastLast,
    /// `Prev` from the first stop: the cursor stays and focus leaves upward.
    PastFirst,
}

/// The index of the cursor in `stops`; a vanished stop resolves to its last
/// index clamped to the list. `None` for an empty list.
pub(crate) fn resolve_cursor(view: &OwnerBlockState, stops: &[Stop]) -> Option<usize> {
    if stops.is_empty() {
        return None;
    }
    let found = view
        .cursor
        .as_ref()
        .and_then(|stop| stops.iter().position(|s| s == stop));
    Some(found.unwrap_or_else(|| view.cursor_index.min(stops.len() - 1)))
}

fn set_cursor(view: &mut OwnerBlockState, stops: &[Stop], index: usize) {
    view.item_cursor = None;
    view.cursor_index = index;
    view.cursor = stops.get(index).cloned();
}

/// Move the cursor over `stops` (identities; the index is only remembered).
pub(crate) fn move_cursor(view: &mut OwnerBlockState, stops: &[Stop], mv: Move) -> Moved {
    let Some(at) = resolve_cursor(view, stops) else {
        return Moved::Within;
    };
    let last = stops.len() - 1;
    let to = match mv {
        Move::Next if at == last => return Moved::PastLast,
        Move::Prev if at == 0 => return Moved::PastFirst,
        Move::Next => at + 1,
        Move::Prev => at - 1,
        Move::First => 0,
        Move::Last => last,
        Move::By(n) => at.saturating_add_signed(n).min(last),
    };
    set_cursor(view, stops, to);
    Moved::Within
}

/// Enter the block on its first stop: from the tab bar above it, or as the
/// Phases tab's only focusable level when the phase list is empty (quick
/// 261118-tm6).
pub(crate) fn enter_first(view: &mut OwnerBlockState, stops: &[Stop]) {
    if !stops.is_empty() {
        set_cursor(view, stops, 0);
    }
}

/// Enter the block on its last stop (D-20).
pub(crate) fn enter_last(view: &mut OwnerBlockState, stops: &[Stop]) {
    if !stops.is_empty() {
        set_cursor(view, stops, stops.len() - 1);
    }
}

/// The fold the fold line's sub-cursor rests on: the remembered one while it is
/// selectable, else the first selectable fold.
pub(crate) fn resolved_fold(view: &OwnerBlockState, folds: &[Fold]) -> Option<Fold> {
    view.fold_cursor
        .filter(|f| folds.contains(f))
        .or_else(|| folds.first().copied())
}

/// Toggle the group under the cursor, or, on the fold line, the fold under the
/// sub-cursor. Only view state changes. Returns whether anything toggled.
pub(crate) fn toggle(view: &mut OwnerBlockState, folds: &[Fold]) -> bool {
    match view.cursor.clone() {
        Some(Stop::Row(key)) => {
            view.item_cursor = None;
            if !view.expanded.remove(&key) {
                view.expanded.insert(key);
            }
            true
        }
        Some(Stop::FoldLine) => match resolved_fold(view, folds) {
            Some(fold) => {
                if !view.open_folds.remove(&fold) {
                    view.open_folds.insert(fold);
                }
                view.fold_cursor = Some(fold);
                true
            }
            None => false,
        },
        None => false,
    }
}

/// Move the fold sub-cursor to the next or previous selectable fold (Todos is
/// not one); it stays at the ends.
pub(crate) fn fold_step(view: &mut OwnerBlockState, folds: &[Fold], forward: bool) {
    let Some(current) = resolved_fold(view, folds) else {
        view.fold_cursor = None;
        return;
    };
    let at = folds.iter().position(|f| *f == current).unwrap_or(0);
    let to = if forward {
        (at + 1).min(folds.len() - 1)
    } else {
        at.saturating_sub(1)
    };
    view.fold_cursor = Some(folds[to]);
}

/// Bring the view state in line with a (possibly refreshed) digest: groups that
/// vanished stop being expanded, folds that lost their content close, a cursor
/// whose stop vanished falls to the nearest remaining index (D-18).
pub(crate) fn clamp(view: &mut OwnerBlockState, digest: &OwnerDigest) {
    let sk = skeleton(digest, view);
    view.expanded.retain(|key| sk.keys.contains(key));
    view.open_folds.retain(|fold| sk.folds.contains(fold));
    // Closing a fold changes the stops: take them again.
    let sk = skeleton(digest, view);
    view.fold_cursor = resolved_fold(view, &sk.folds).filter(|_| view.fold_cursor.is_some());
    let item = view.item_cursor.take();
    match resolve_cursor(view, &sk.stops) {
        Some(index) => set_cursor(view, &sk.stops, index),
        None => {
            view.cursor = None;
            view.cursor_index = 0;
        }
    }
    // The item survives while its row is still the cursor row, still expanded
    // and still draws it (D-04, T-39-11).
    view.item_cursor = item.filter(|id| match &view.cursor {
        Some(Stop::Row(key)) if view.expanded.contains(key) => {
            row_src_for(digest, key).is_some_and(|src| row_item_ids(&src).contains(id))
        }
        _ => false,
    });
}

/// A cursor step that walks the item lines of an expanded row first (D-04):
/// `forward` is `j`/Down. Leaving the row moves by [`move_cursor`]; a backward
/// leave onto an expanded row lands on that row's last drawn item.
pub(crate) fn step_with_items(
    view: &mut OwnerBlockState,
    digest: &OwnerDigest,
    stops: &[Stop],
    forward: bool,
) -> Moved {
    let mv = if forward { Move::Next } else { Move::Prev };
    let at = resolve_cursor(view, stops);
    if let Some(Stop::Row(key)) = at.and_then(|i| stops.get(i)) {
        if view.expanded.contains(key) {
            if let Some(src) = row_src_for(digest, key) {
                let ids = row_item_ids(&src);
                match item_step(&ids, view.item_cursor.as_deref(), forward) {
                    ItemStep::Item(id) => {
                        view.item_cursor = Some(id);
                        return Moved::Within;
                    }
                    ItemStep::Parent => {
                        view.item_cursor = None;
                        return Moved::Within;
                    }
                    ItemStep::Leave => {}
                }
            }
        }
    }
    view.item_cursor = None;
    let moved = move_cursor(view, stops, mv);
    if !forward && moved == Moved::Within {
        if let Some(Stop::Row(key)) = view.cursor.clone() {
            if view.expanded.contains(&key) {
                view.item_cursor =
                    row_src_for(digest, &key).and_then(|src| row_item_ids(&src).pop());
            }
        }
    }
    moved
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A synthetic digest modeled on the UI-SPEC fixture, reused by the detail tests.
#[cfg(test)]
pub(crate) fn sample_owner_digest() -> OwnerDigest {
    use crate::state_reader::owner_items::{
        ArchivedCounts, Category, GroupCounts, Hidden, LanePhases, OwnerItem, RunCommand, RunVerb,
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
        run: Some(RunCommand {
            verb: RunVerb::VerifyWork,
            phase: Some(phase.to_owned()),
            workstream: None,
        }),
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
            planned_questions: 0,
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

    #[test]
    fn a_three_digit_checks_cell_widens_the_planned_grid() {
        let row = |checks: &str| RowCells {
            lane: Lane::Test,
            ws_prefix: String::new(),
            phase: "1".to_owned(),
            name: "n".to_owned(),
            checks: checks.to_owned(),
            phrase: "p".to_owned(),
            green_phrase: false,
            bookkeeping: false,
            age: None,
            command: String::new(),
            extras: String::new(),
        };
        let narrow = grid_for(&[row("3 checks")], 120);
        assert_eq!(narrow.checks_w, CHECKS_W);
        let wide = grid_for(&[row("3 checks"), row("120 checks")], 120);
        assert_eq!(wide.checks_w, CHECKS_W + 1);
    }

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn the_header_walks_the_five_rungs() {
        let digest = sample_owner_digest();
        let at = |w: u16| {
            text(&block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), w).header)
        };
        assert_eq!(
            at(140),
            "Owner \u{2500}\u{2500} sample \u{b7} v2.2 closes after: test 12 (ph 21 22 24) \u{b7} decide 1 (ph 26) \u{b7} unblock 5 (ph 20 23) \u{b7} fix 1 (ph 25) \u{b7} not built ph 27"
        );
        let counts = at(110);
        assert!(counts.ends_with("not built 1"), "{counts}");
        assert!(counts.contains("test 12 \u{b7} decide 1"), "{counts}");
        let dropped = at(75);
        assert!(!dropped.contains("not built"), "{dropped}");
        assert!(dropped.contains("fix 1"), "{dropped}");
        for w in [21u16, 40, 75, 100, 139, 140] {
            let header =
                block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), w).header;
            assert!(header.width() <= usize::from(w), "{w}: {}", text(&header));
        }
        assert!(text(
            &block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 21).header
        )
        .ends_with('\u{2026}'));
    }

    #[test]
    fn a_fix_row_without_failed_checks_does_not_repeat_its_check_count() {
        let mut digest = sample_owner_digest();
        let fix = digest
            .groups
            .iter_mut()
            .find(|g| g.lane == Lane::Fix && g.phase == "25")
            .expect("fix group");
        fix.counts.failed = 0;
        fix.counts.gaps = 0;
        fix.counts.checks = 7;
        fix.file_status = "gaps_found".to_owned();
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 140);
        let rows: Vec<String> = model.body.iter().map(text).collect();
        let row = rows.iter().find(|r| r.contains("FIX")).expect("fix row");
        assert_eq!(row.matches("7 checks").count(), 1, "{row}");
        assert!(row.contains("gaps found"), "{row}");
    }

    #[test]
    fn the_command_column_is_not_starved_at_exactly_one_hundred_columns() {
        let mut digest = sample_owner_digest();
        // A long phrase on one row used to take every cell the command needed.
        let fix = digest
            .groups
            .iter_mut()
            .find(|g| g.lane == Lane::Fix && g.phase == "25")
            .expect("fix group");
        fix.counts.failed = 12;
        fix.counts.gaps = 12;
        fix.counts.unreadable = 3;
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 100);
        let rows: Vec<String> = model.body.iter().map(text).collect();
        let test_row = rows.iter().find(|r| r.contains("ph 21")).expect("test row");
        assert!(test_row.contains("/gsd-"), "{test_row}");
        let fix_row = rows.iter().find(|r| r.contains("FIX")).expect("fix row");
        assert!(fix_row.contains("/gsd-pl"), "{fix_row}");
    }

    #[test]
    fn a_long_workstream_name_gives_way_before_the_phase_number() {
        let mut digest = sample_owner_digest();
        let handoff = digest
            .groups
            .iter_mut()
            .find(|g| g.workstream.is_some())
            .expect("workstream group");
        handoff.workstream = Some("v0-14-voice".to_owned());
        handoff.kind = Kind::Uat;
        handoff.lane = Lane::Test;
        handoff.phase = "26".to_owned();
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 100);
        let rows: Vec<String> = model.body.iter().map(text).collect();
        let row = rows
            .iter()
            .find(|r| r.contains("ws v0-14"))
            .expect("workstream row");
        assert!(row.contains("\u{2026} ph 26"), "{row}");
    }

    #[test]
    fn a_clipped_block_header_is_a_whole_rung_beside_the_hint() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut digest = sample_owner_digest();
        // Many rows, so the body is clipped in a short block.
        let template = digest
            .groups
            .iter()
            .find(|g| g.lane == Lane::Test && g.phase == "25")
            .expect("test group")
            .clone();
        for n in 0..30 {
            let mut g = template.clone();
            g.phase = format!("{}", 40 + n);
            g.file = format!("phases/{}/UAT.md", g.phase);
            digest.groups.push(g);
        }
        for width in 60u16..=140 {
            let model = block_model(
                "sample",
                "v2.2",
                &digest,
                &OwnerBlockState::default(),
                width,
            );
            let mut terminal = Terminal::new(TestBackend::new(width, 6)).expect("backend");
            terminal
                .draw(|f| render_owner_block(f, f.area(), &model, None, 0))
                .expect("draw");
            let buffer = terminal.backend().buffer().clone();
            let first: String = (0..width)
                .map(|x| buffer.cell((x, 0)).map_or(" ", |c| c.symbol()).to_string())
                .collect();
            let first = first.trim_end().to_owned();
            let (head, hint) = first.rsplit_once(" 1-").expect("hint");
            assert!(hint.contains("of"), "{first}");
            // A whole rung ends in a number, `not built N`, or the counts form: never
            // in the middle of a clause such as `(ph 11`.
            let cut_mid_clause = head.contains('(') && !head.trim_end().ends_with(')');
            assert!(
                !cut_mid_clause && !head.trim_end().ends_with('\u{2026}') || width < 100,
                "{width}: {first}"
            );
        }
    }

    #[test]
    fn the_counts_rung_counts_the_not_built_phases_too() {
        let mut digest = sample_owner_digest();
        digest.closes_after.not_built = (27..41).map(|n| format!("{n}")).collect();
        let gate = &digest.closes_after;
        let width = |form| header_with("sample", "v2.2", gate, form, true).width();
        let counts = width(ClauseForm::CountsOnly);
        assert!(counts < width(ClauseForm::PhasesCapped(6)));
        let at_counts = text(
            &block_model(
                "sample",
                "v2.2",
                &digest,
                &OwnerBlockState::default(),
                u16::try_from(counts).unwrap(),
            )
            .header,
        );
        assert!(at_counts.ends_with("not built 14"), "{at_counts}");
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
        let header =
            text(&block_model("sample", "  ", &digest, &OwnerBlockState::default(), 200).header);
        assert!(
            header.starts_with("Owner \u{2500}\u{2500} sample closes after:"),
            "{header}"
        );
        digest.closes_after = ClosesAfter::default();
        let header =
            text(&block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 200).header);
        assert_eq!(header, "Owner \u{2500}\u{2500} sample \u{b7} v2.2");
    }

    #[test]
    fn long_header_strings_are_cut_at_the_width() {
        let digest = sample_owner_digest();
        let long = "x".repeat(300);
        for w in [21u16, 80, 120] {
            let model = block_model(&long, &long, &digest, &OwnerBlockState::default(), w);
            assert!(model.header.width() <= usize::from(w));
        }
    }

    #[test]
    fn the_height_follows_content_and_tab() {
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 120);
        assert_eq!(block_height(&model, 11), 0);
        assert_eq!(block_height(&model, 12), 4);
        assert!(block_height(&model, 30) >= 1);
        assert!(block_height(&model, 30) <= 12);
        let empty = block_model(
            "sample",
            "v2.2",
            &OwnerDigest::default(),
            &OwnerBlockState::default(),
            120,
        );
        assert!(!empty.has_content);
        assert_eq!(block_height(&empty, 30), 0);
        // IN-05: no `u16` overflow on a very tall tab; the content is the bound.
        let wanted = u16::try_from(1 + model.body.len() + 1).unwrap();
        assert_eq!(block_height(&model, u16::MAX), wanted);
        assert_eq!(block_height(&model, 1639), wanted);
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
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 120);
        assert!(model.body.is_empty());
        assert!(model.fold_line.is_some());
        digest.groups.retain(|g| g.lane == Lane::Deferred);
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 120);
        assert!(!model.has_content);
        assert_eq!(block_height(&model, 30), 0);
    }

    #[test]
    fn the_rows_follow_the_inbox_order_and_phrases() {
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 140);
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
        let model = block_model("sample", "v2.2", &digest, &OwnerBlockState::default(), 120);
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
            block_model("s", "m", &digest, &OwnerBlockState::default(), 120)
                .fold_line
                .as_ref()
                .unwrap(),
        );
        assert!(fold.contains("v1.1 ph 04, v1.2 ph 08   "), "{fold}");
        assert!(fold.ends_with("Todos ?"), "{fold}");
        digest.hidden.todos_unreadable = false;
        digest.hidden.todos_oldest_days = None;
        let fold = text(
            block_model("s", "m", &digest, &OwnerBlockState::default(), 120)
                .fold_line
                .as_ref()
                .unwrap(),
        );
        assert!(!fold.contains("Todos"), "{fold}");
        digest.hidden.acknowledged_files = 1;
        let fold = text(
            block_model("s", "m", &digest, &OwnerBlockState::default(), 120)
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
            let line = block_model("s", "m", &digest, &OwnerBlockState::default(), w)
                .fold_line
                .unwrap();
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

    /// IN-02: a rung that fits the width exactly would lose its tail to `…` once
    /// the cursor's `> ` gutter is added, so the cursor line picks its rung for two
    /// cells less and the fold under the sub-cursor stays whole.
    #[test]
    fn the_cursor_fold_line_picks_its_rung_for_the_gutter() {
        let digest = sample_owner_digest();
        let view = OwnerBlockState::default();
        let rung_two = block_model("s", "m", &digest, &view, 80).fold_line.unwrap();
        let exact = rung_two.width();
        let model = block_model("s", "m", &digest, &view, u16::try_from(exact).unwrap());
        let plain = model.fold_line.clone().unwrap();
        assert_eq!(text(&plain), text(&rung_two), "the rung fits exactly");
        let cursor = fold_line_with_cursor(&model, &plain, Some(Fold::Todos), exact);
        let shown = text(&cursor);
        assert!(cursor.width() <= exact, "{shown}");
        assert!(shown.starts_with("> "), "{shown}");
        assert!(!shown.contains('\u{2026}'), "no fold is cut: {shown}");
        assert!(shown.ends_with("Todos 11"), "{shown}");
        let todos = cursor
            .spans
            .iter()
            .find(|s| s.content.contains("Todos"))
            .expect("the Todos fold");
        assert!(
            todos.style.add_modifier.contains(Modifier::BOLD),
            "{todos:?}"
        );
    }

    #[test]
    fn has_content_matches_rows_or_fold() {
        for digest in [sample_owner_digest(), OwnerDigest::default()] {
            let m = block_model("s", "m", &digest, &OwnerBlockState::default(), 120);
            assert_eq!(m.has_content, !m.body.is_empty() || m.fold_line.is_some());
        }
        let mut fold_only = OwnerDigest::default();
        fold_only.hidden.todos = 3;
        let m = block_model("s", "m", &fold_only, &OwnerBlockState::default(), 120);
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
            let model = block_model(&long, &long, &digest, &OwnerBlockState::default(), w);
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

    fn view_with(keys: &[GroupKey]) -> OwnerBlockState {
        let mut view = OwnerBlockState::default();
        view.expanded.extend(keys.iter().cloned());
        view
    }

    fn lines_of(model: &BlockModel) -> Vec<String> {
        model.body.iter().map(text).collect()
    }

    fn key_for(digest: &OwnerDigest, phase: &str, lane: Lane) -> GroupKey {
        let group = digest
            .groups
            .iter()
            .find(|g| g.phase == phase && g.lane == lane && g.scope == Scope::Current)
            .expect("group");
        GroupKey::of(group)
    }

    #[test]
    fn expanded_lines_omit_absent_parts_and_cap_items_and_gaps() {
        use crate::state_reader::owner_items::OwnerGap;
        let mut digest = sample_owner_digest();
        let group = digest
            .groups
            .iter_mut()
            .find(|g| g.phase == "24" && g.lane == Lane::Test)
            .expect("24");
        group.items[0].test = Some("7".to_owned());
        group.items[0].why_human = Some("a device".to_owned());
        group.items[1].blocked_by = Some("a build".to_owned());
        group.items[2].gaps = (0..3)
            .map(|n| OwnerGap {
                name: format!("gap {n}"),
                result: "failed".to_owned(),
                reason: (n == 0).then(|| "slow".to_owned()),
            })
            .collect();
        let key = key_for(&digest, "24", Lane::Test);
        let model = block_model("s", "m", &digest, &view_with(&[key]), 120);
        let lines = lines_of(&model);
        let at = lines
            .iter()
            .position(|l| l.contains("clock-faces"))
            .unwrap();
        let rest: Vec<&str> = lines[at + 1..].iter().map(|l| l.trim_end()).collect();
        assert_eq!(rest[0], "    7 check 0");
        assert_eq!(rest[1], "       why: a device");
        assert_eq!(rest[2], "    check 1");
        assert_eq!(rest[3], "       blocked by: a build");
        assert_eq!(rest[4], "    check 2");
        assert_eq!(rest[5], "       gap: gap 0 (failed: slow)");
        assert_eq!(rest[6], "       gap: gap 1 (failed)");
        assert_eq!(rest[7], "       \u{2026} 1 more gaps");
        assert_eq!(rest[8], "    check 3");
        assert_eq!(rest[9], "    check 4");
        assert_eq!(rest[10], "    \u{2026} 1 more");
        assert_eq!(rest[11], "    file phases/24/24-UAT.md");
        assert!(rest[12].contains("TEST"), "{rest:?}");
        // Every expansion line but the items is DIM.
        let detail = &model.body[at + 2];
        assert!(detail.spans[0].style.add_modifier.contains(Modifier::DIM));
        // A group with no items shows just its file.
        let model = block_model(
            "s",
            "m",
            &digest,
            &view_with(&[key_for(&digest, "22", Lane::Test)]),
            120,
        );
        let lines = lines_of(&model);
        let at = lines.iter().position(|l| l.contains("notif-copy")).unwrap();
        assert_eq!(lines[at + 1].trim_end(), "    file phases/22/22-UAT.md");
    }

    #[test]
    fn a_blocked_row_expands_to_all_its_groups_and_files() {
        let digest = sample_owner_digest();
        let blocked = digest
            .groups
            .iter()
            .find(|g| g.lane == Lane::Blocked)
            .unwrap();
        let key = GroupKey::aggregate(blocked);
        let model = block_model(
            "s",
            "m",
            &digest,
            &view_with(std::slice::from_ref(&key)),
            120,
        );
        let lines = lines_of(&model);
        let at = lines.iter().position(|l| l.contains("BLOCKED")).unwrap();
        let rest: Vec<&str> = lines[at + 1..].iter().map(|l| l.trim_end()).collect();
        assert_eq!(&rest[..4], ["    a", "    b", "    c", "    d"]);
        assert_eq!(rest[4], "    file phases/23/23-UAT.md");
        // The sample's two groups share one path (a synthetic fixture), so one
        // distinct file; a second path adds a second line.
        let mut digest = digest;
        for g in digest
            .groups
            .iter_mut()
            .filter(|g| g.lane == Lane::Blocked && g.kind == Kind::Verification)
        {
            g.file = "phases/23/23-VERIFICATION.md".to_owned();
        }
        let model = block_model("s", "m", &digest, &view_with(&[key]), 120);
        let lines = lines_of(&model);
        let at = lines.iter().position(|l| l.contains("BLOCKED")).unwrap();
        assert_eq!(lines[at + 5].trim_end(), "    file phases/23/23-UAT.md");
        assert_eq!(
            lines[at + 6].trim_end(),
            "    file phases/23/23-VERIFICATION.md"
        );
    }

    #[test]
    fn stops_are_the_rows_then_the_fold_line_and_open_folds_add_theirs() {
        let digest = sample_owner_digest();
        let model = block_model("s", "m", &digest, &OwnerBlockState::default(), 120);
        assert_eq!(model.stops.len(), 9);
        assert_eq!(model.stops.last(), Some(&Stop::FoldLine));
        assert_eq!(model.folds, [Fold::OldDebt, Fold::Acknowledged]);
        assert_eq!(model.stop_lines.last(), Some(&None));
        let mut view = OwnerBlockState::default();
        view.open_folds.insert(Fold::OldDebt);
        let model = block_model("s", "m", &digest, &view, 120);
        let lines = lines_of(&model);
        // Old debt: two milestone sub-headers and three rows, the rows are stops.
        assert_eq!(model.stops.len(), 9 + 3);
        assert!(lines.iter().any(|l| l.trim() == "v1.1"));
        assert!(model
            .fold_line
            .as_ref()
            .map(text)
            .unwrap()
            .starts_with("\u{25be} Old debt"));
        // Open-fold rows follow the current rows, ahead of the pinned fold line.
        assert!(lines[8].trim() == "v1.1", "{lines:#?}");
    }

    #[test]
    fn fold_step_skips_todos_and_stays_at_the_ends() {
        let folds = [Fold::OldDebt, Fold::Acknowledged];
        let mut view = OwnerBlockState::default();
        assert_eq!(resolved_fold(&view, &folds), Some(Fold::OldDebt));
        fold_step(&mut view, &folds, true);
        assert_eq!(view.fold_cursor, Some(Fold::Acknowledged));
        fold_step(&mut view, &folds, true);
        assert_eq!(view.fold_cursor, Some(Fold::Acknowledged));
        fold_step(&mut view, &folds, false);
        fold_step(&mut view, &folds, false);
        assert_eq!(view.fold_cursor, Some(Fold::OldDebt));
        // A fold that stopped being selectable falls back to the first.
        view.fold_cursor = Some(Fold::Todos);
        assert_eq!(resolved_fold(&view, &folds), Some(Fold::OldDebt));
        fold_step(&mut view, &[], true);
        assert_eq!(view.fold_cursor, None);
    }

    #[test]
    fn toggle_flips_a_group_or_the_fold_under_the_sub_cursor() {
        let digest = sample_owner_digest();
        let sk = skeleton(&digest, &OwnerBlockState::default());
        let mut view = OwnerBlockState::default();
        enter_last(&mut view, &sk.stops);
        assert_eq!(view.cursor, Some(Stop::FoldLine));
        assert!(toggle(&mut view, &sk.folds));
        assert!(view.open_folds.contains(&Fold::OldDebt));
        assert!(toggle(&mut view, &sk.folds));
        assert!(view.open_folds.is_empty());
        move_cursor(&mut view, &sk.stops, Move::First);
        assert!(toggle(&mut view, &sk.folds));
        assert_eq!(view.expanded.len(), 1);
        assert!(toggle(&mut view, &sk.folds));
        assert!(view.expanded.is_empty());
        assert!(!toggle(&mut OwnerBlockState::default(), &sk.folds));
    }

    #[test]
    fn move_cursor_reports_the_ends_and_pages() {
        let digest = sample_owner_digest();
        let sk = skeleton(&digest, &OwnerBlockState::default());
        let mut view = OwnerBlockState::default();
        assert_eq!(
            move_cursor(&mut view, &sk.stops, Move::Prev),
            Moved::PastFirst
        );
        assert_eq!(move_cursor(&mut view, &sk.stops, Move::Last), Moved::Within);
        assert_eq!(
            move_cursor(&mut view, &sk.stops, Move::Next),
            Moved::PastLast
        );
        assert_eq!(view.cursor, Some(Stop::FoldLine));
        move_cursor(&mut view, &sk.stops, Move::By(-3));
        assert_eq!(view.cursor_index, 5);
        move_cursor(&mut view, &sk.stops, Move::By(100));
        assert_eq!(view.cursor_index, sk.stops.len() - 1);
        move_cursor(&mut view, &sk.stops, Move::By(-100));
        assert_eq!(view.cursor_index, 0);
        assert_eq!(move_cursor(&mut view, &[], Move::Next), Moved::Within);
    }

    #[test]
    fn clamp_drops_vanished_groups_and_folds_and_pulls_the_cursor_in() {
        let mut digest = sample_owner_digest();
        let sk = skeleton(&digest, &OwnerBlockState::default());
        let mut view = OwnerBlockState::default();
        view.expanded.insert(key_for(&digest, "24", Lane::Test));
        view.expanded.insert(key_for(&digest, "25", Lane::Fix));
        view.open_folds.insert(Fold::Acknowledged);
        view.fold_cursor = Some(Fold::Acknowledged);
        move_cursor(&mut view, &sk.stops, Move::By(6));
        assert!(matches!(&view.cursor, Some(Stop::Row(k)) if k.lane == "fix"));
        digest
            .groups
            .retain(|g| g.lane != Lane::Fix && !g.acknowledged);
        clamp(&mut view, &digest);
        assert_eq!(view.expanded.len(), 1);
        assert!(view.open_folds.is_empty());
        assert_eq!(view.fold_cursor, Some(Fold::OldDebt));
        // The cursor's group vanished: it sits on the same index in the new list.
        let sk = skeleton(&digest, &view);
        assert_eq!(view.cursor_index, 6);
        assert_eq!(view.cursor.as_ref(), sk.stops.get(6));
        // Everything gone: no cursor at all.
        clamp(&mut view, &OwnerDigest::default());
        assert_eq!(view.cursor, None);
        assert_eq!(resolve_cursor(&view, &[]), None);
    }

    #[test]
    fn the_scroll_window_follows_the_cursor_by_whole_rows() {
        let digest = sample_owner_digest();
        let model = block_model("s", "m", &digest, &OwnerBlockState::default(), 120);
        assert_eq!(window_rows(&model, 5), 3);
        assert_eq!(follow_offset(&model, Some(0), 5, 3), 0);
        assert_eq!(follow_offset(&model, Some(7), 0, 3), 5);
        assert_eq!(follow_offset(&model, Some(4), 5, 3), 4);
        // The fold line is pinned: it never moves the window.
        assert_eq!(follow_offset(&model, Some(8), 2, 3), 2);
        // An offset past the end clamps.
        assert_eq!(follow_offset(&model, None, 99, 3), 5);
    }

    /// The block drawn at a terminal size, one `String` per row (trailing blanks
    /// kept, so a changed fill shows up too).
    fn drawn(
        width: u16,
        term_height: u16,
        view: &OwnerBlockState,
        cursor: Option<Cursor>,
    ) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let digest = sample_owner_digest();
        let model = block_model("sample", "v2.2", &digest, view, width);
        let height = block_height(&model, term_height);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
        terminal
            .draw(|f| render_owner_block(f, f.area(), &model, cursor, 0))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..height {
            for x in 0..width {
                out.push_str(buffer.cell((x, y)).map_or(" ", |c| c.symbol()));
            }
            out.push('\n');
        }
        out
    }

    fn golden(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/owner_golden")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn pin_views() -> [(&'static str, u16, u16, OwnerBlockState, Option<Cursor>); 3] {
        let digest = sample_owner_digest();
        let first_test = digest
            .groups
            .iter()
            .find(|g| g.lane == Lane::Test && g.scope == Scope::Current)
            .expect("test group");
        let mut open = OwnerBlockState::default();
        open.expanded.insert(GroupKey::of(first_test));
        open.open_folds.insert(Fold::OldDebt);
        [
            (
                "owner_block_120x30.txt",
                120,
                30,
                OwnerBlockState::default(),
                None,
            ),
            (
                "owner_block_120x30_open.txt",
                120,
                30,
                open,
                Some(Cursor {
                    stop: 0,
                    fold: None,
                    item_line: None,
                }),
            ),
            (
                "owner_block_80x24.txt",
                80,
                24,
                OwnerBlockState::default(),
                None,
            ),
        ]
    }

    #[test]
    fn group_for_finds_a_group_by_identity() {
        let digest = sample_owner_digest();
        let first = digest
            .groups
            .iter()
            .find(|g| g.lane == Lane::Test && g.phase == "24")
            .expect("the fixture's TEST ph 24 group");
        let key = GroupKey::of(first);
        assert_eq!(group_for(&digest, &key).as_ref(), Some(first));
        // An acknowledged group is still found by its identity.
        let acked = digest
            .groups
            .iter()
            .find(|g| g.acknowledged)
            .expect("an acknowledged group");
        assert!(group_for(&digest, &GroupKey::of(acked)).is_some());
        // A BLOCKED aggregate key has no single group.
        let blocked = digest
            .groups
            .iter()
            .find(|g| g.lane == Lane::Blocked)
            .expect("a BLOCKED group");
        assert!(group_for(&digest, &GroupKey::aggregate(blocked)).is_none());
        // A vanished identity is None.
        let mut gone = key.clone();
        gone.phase = "99".to_owned();
        assert!(group_for(&digest, &gone).is_none());
    }

    /// The 36 layout is frozen (coordinator directive 4): the block renders the
    /// same bytes as when the goldens were written, before D-11's `run` field.
    #[test]
    fn owner_block_output_is_pinned() {
        for (name, w, h, view, cursor) in pin_views() {
            let got = drawn(w, h, &view, cursor);
            let want = golden(name);
            for (n, (g, e)) in got.lines().zip(want.lines()).enumerate() {
                assert_eq!(g, e, "{name} line {}", n + 1);
            }
            assert_eq!(
                got.lines().count(),
                want.lines().count(),
                "{name} line count"
            );
        }
    }

    // ── The item sub-cursor (39-02, D-04) ───────────────────────────────────

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn item_step_walks_the_drawn_ids() {
        let ab = ids(&["a", "b"]);
        let item = |id: &str| ItemStep::Item(id.to_owned());
        assert_eq!(item_step(&ab, None, true), item("a"));
        assert_eq!(item_step(&ab, Some("a"), true), item("b"));
        assert_eq!(item_step(&ab, Some("b"), true), ItemStep::Leave);
        assert_eq!(item_step(&ab, Some("b"), false), item("a"));
        assert_eq!(item_step(&ab, Some("a"), false), ItemStep::Parent);
        assert_eq!(item_step(&ab, None, false), ItemStep::Leave);
        assert_eq!(item_step(&[], None, true), ItemStep::Leave);
        // An id that is not listed behaves as the row.
        assert_eq!(item_step(&ab, Some("zzz"), true), item("a"));
        assert_eq!(item_step(&ab, Some("zzz"), false), ItemStep::Leave);
    }

    /// A merged inbox row over two copies of the sample's phase 24 group.
    fn merged_src(digest: &OwnerDigest) -> RowSrc {
        let first = digest
            .groups
            .iter()
            .find(|g| g.phase == "24" && g.lane == Lane::Test)
            .expect("phase 24");
        let mut second = first.clone();
        second.file = "phases/24/24-VERIFICATION.md".to_owned();
        second.items.truncate(2);
        for (i, item) in second.items.iter_mut().enumerate() {
            item.id = format!("v{i}");
        }
        RowSrc {
            group: first.clone(),
            merged: None,
            files: vec![first.file.clone(), second.file.clone()],
            key: GroupKey::entry(first),
            parts: vec![first.clone(), second],
        }
    }

    #[test]
    fn sub_step_walks_items_then_files() {
        let item = |id: &str| SubStop::Item(id.to_owned());
        let file = |id: &str| SubStop::File(id.to_owned());
        let stops = vec![item("a"), item("b"), file("f")];
        let stop = |s: SubStop| SubStep::Stop(s);
        assert_eq!(sub_step(&stops, None, true), stop(item("a")));
        assert_eq!(sub_step(&stops, Some(&item("a")), true), stop(item("b")));
        assert_eq!(sub_step(&stops, Some(&item("b")), true), stop(file("f")));
        assert_eq!(sub_step(&stops, Some(&file("f")), true), SubStep::Leave);
        assert_eq!(sub_step(&stops, Some(&file("f")), false), stop(item("b")));
        assert_eq!(sub_step(&stops, Some(&item("a")), false), SubStep::Parent);
        assert_eq!(sub_step(&stops, None, false), SubStep::Leave);
        assert_eq!(sub_step(&[], None, true), SubStep::Leave);
        // A stop that is not listed behaves as the row.
        assert_eq!(sub_step(&stops, Some(&item("zzz")), true), stop(item("a")));
        assert_eq!(sub_step(&stops, Some(&file("zzz")), false), SubStep::Leave);
        // An item and a file with the same text are different stops.
        assert_eq!(
            sub_step(&[file("a")], Some(&item("a")), true),
            stop(file("a"))
        );
    }

    #[test]
    fn the_two_cursor_fields_are_one_sub_stop() {
        let (mut item, mut file) = (None, None);
        assert_eq!(SubStop::of(&item, &file), None);
        SubStop::store(Some(SubStop::Item("a".to_owned())), &mut item, &mut file);
        assert_eq!((item.as_deref(), file.as_deref()), (Some("a"), None));
        SubStop::store(Some(SubStop::File("f".to_owned())), &mut item, &mut file);
        assert_eq!((item.as_deref(), file.as_deref()), (None, Some("f")));
        assert_eq!(
            SubStop::of(&item, &file),
            Some(SubStop::File("f".to_owned()))
        );
        SubStop::store(None, &mut item, &mut file);
        assert_eq!(SubStop::of(&item, &file), None);
    }

    #[test]
    fn mapped_lines_match_the_drawn_ids() {
        let digest = sample_owner_digest();
        let mut sources: Vec<RowSrc> = current_sources(&digest);
        sources.push(merged_src(&digest));
        let mut checked = 0;
        for src in &sources {
            let (lines, map) = expansion_lines_mapped(src, 100, 0);
            assert_eq!(
                lines.iter().map(text).collect::<Vec<_>>(),
                expansion_lines(src, 100, 0)
                    .iter()
                    .map(text)
                    .collect::<Vec<_>>()
            );
            let items_of = |map: &[(usize, SubStop)]| -> Vec<String> {
                map.iter()
                    .filter_map(|(_, stop)| match stop {
                        SubStop::Item(id) => Some(id.clone()),
                        SubStop::File(_) => None,
                    })
                    .collect()
            };
            assert_eq!(items_of(&map), row_item_ids(src), "{:?}", src.key);
            let (lines, map) = inbox_expansion_lines_mapped(src, 100, 0);
            assert_eq!(items_of(&map), inbox_item_ids(src), "inbox {:?}", src.key);
            // The drawn stops, in order, are exactly the walk's stops.
            let drawn: Vec<SubStop> = map.iter().map(|(_, stop)| stop.clone()).collect();
            assert_eq!(drawn, inbox_sub_stops(src), "inbox {:?}", src.key);
            assert!(
                map.windows(2).all(|w| w[0].0 < w[1].0),
                "stops are in drawn order"
            );
            // Each mapped line is where that item's (or file's) text starts.
            for (line, stop) in &map {
                match stop {
                    SubStop::Item(id) => {
                        let item = src
                            .items()
                            .iter()
                            .chain(src.parts.iter().flat_map(|p| p.items.iter()))
                            .find(|i| i.id == *id)
                            .expect("the item");
                        assert!(
                            text(&lines[*line]).contains(item.name.as_str()),
                            "{id} at {line}: {}",
                            text(&lines[*line])
                        );
                    }
                    SubStop::File(path) => {
                        let want = format!("file {path}");
                        assert!(
                            text(&lines[*line]).contains(&want),
                            "{path} at {line}: {}",
                            text(&lines[*line])
                        );
                    }
                }
                checked += 1;
            }
        }
        assert!(checked > 10, "the check ran over real items: {checked}");
        // The merged row walks the first part's drawn items, then the second's.
        let merged = merged_src(&digest);
        assert_eq!(
            inbox_item_ids(&merged),
            ids(&["check 0", "check 1", "check 2", "check 3", "check 4", "v0", "v1"])
        );
        let map = inbox_expansion_lines_mapped(&merged, 100, 0).1;
        assert_eq!(sub_line(&map, &SubStop::Item("v1".to_owned())), Some(9));
        // Merged rows draw each file line before that part's checks.
        assert_eq!(
            inbox_sub_stops(&merged)
                .iter()
                .filter(|s| matches!(s, SubStop::File(_)))
                .count(),
            2
        );
        assert_eq!(
            sub_line(&map, &SubStop::File("phases/24/24-UAT.md".to_owned())),
            Some(0)
        );
        assert_eq!(
            sub_line(
                &map,
                &SubStop::File("phases/24/24-VERIFICATION.md".to_owned())
            ),
            Some(7)
        );
    }

    /// The walk state over the sample: phase 24 (six checks, two drawn after the
    /// fifth is cut) and phase 21 are the first two TEST rows.
    fn walk_view(digest: &OwnerDigest) -> (OwnerBlockState, Vec<Stop>, GroupKey, GroupKey) {
        let mut view = OwnerBlockState::default();
        let rows = current_sources(digest);
        let (a, b) = (rows[0].key.clone(), rows[1].key.clone());
        view.expanded.insert(a.clone());
        view.expanded.insert(b.clone());
        let stops = skeleton(digest, &view).stops;
        view.cursor = Some(Stop::Row(a.clone()));
        (view, stops, a, b)
    }

    #[test]
    fn step_with_items_enters_and_leaves_rows() {
        let digest = sample_owner_digest();
        // Row a is phase 21 (two checks), row b phase 24 (six checks).
        let (mut view, stops, a, b) = walk_view(&digest);
        let fwd = |view: &mut OwnerBlockState| step_with_items(view, &digest, &stops, true);
        let back = |view: &mut OwnerBlockState| step_with_items(view, &digest, &stops, false);
        let at = |view: &OwnerBlockState| (view.cursor.clone(), view.item_cursor.clone());
        let item =
            |key: &GroupKey, n: usize| (Some(Stop::Row(key.clone())), Some(format!("check {n}")));
        assert_eq!(fwd(&mut view), Moved::Within);
        assert_eq!(at(&view), item(&a, 0));
        fwd(&mut view);
        assert_eq!(at(&view), item(&a, 1));
        // Past the last item: the next row, item cleared.
        fwd(&mut view);
        assert_eq!(at(&view), (Some(Stop::Row(b.clone())), None));
        // Backward from the row lands on the previous expanded row's LAST item.
        back(&mut view);
        assert_eq!(at(&view), item(&a, 1));
        back(&mut view);
        assert_eq!(at(&view), item(&a, 0));
        // From the first item: its own row, item cleared.
        back(&mut view);
        assert_eq!(at(&view), (Some(Stop::Row(a.clone())), None));
        assert_eq!(back(&mut view), Moved::PastFirst);
        // Row b: five stops (the sixth check is only a `... 1 more` line).
        view.cursor = Some(Stop::Row(b.clone()));
        view.item_cursor = None;
        for n in 0..5 {
            fwd(&mut view);
            assert_eq!(at(&view), item(&b, n));
        }
        fwd(&mut view);
        let next = stops[stops
            .iter()
            .position(|s| *s == Stop::Row(b.clone()))
            .unwrap()
            + 1]
        .clone();
        assert_eq!(
            at(&view),
            (Some(next), None),
            "the sixth check is not a stop"
        );
        back(&mut view);
        assert_eq!(at(&view), item(&b, 4));
    }

    #[test]
    fn rows_without_drawn_items_and_collapsed_rows_are_crossed_row_to_row() {
        let digest = sample_owner_digest();
        let rows = current_sources(&digest);
        // Phase 22 has no items at all: expanded or not, `j` crosses it.
        let empty = rows
            .iter()
            .position(|r| r.group.phase == "22")
            .expect("phase 22");
        let mut view = OwnerBlockState::default();
        view.expanded.insert(rows[empty].key.clone());
        let stops = skeleton(&digest, &view).stops;
        view.cursor = Some(Stop::Row(rows[empty].key.clone()));
        step_with_items(&mut view, &digest, &stops, true);
        assert_eq!(view.cursor, Some(Stop::Row(rows[empty + 1].key.clone())));
        assert_eq!(view.item_cursor, None);
        // `k` back onto the empty expanded row: no item to land on.
        step_with_items(&mut view, &digest, &stops, false);
        assert_eq!(view.cursor, Some(Stop::Row(rows[empty].key.clone())));
        assert_eq!(view.item_cursor, None);
        // A collapsed row is crossed too.
        let mut view = OwnerBlockState {
            cursor: Some(Stop::Row(rows[0].key.clone())),
            ..OwnerBlockState::default()
        };
        step_with_items(&mut view, &digest, &stops, true);
        assert_eq!(view.cursor, Some(Stop::Row(rows[1].key.clone())));
        assert_eq!(view.item_cursor, None);
    }

    #[test]
    fn jumps_toggles_and_refreshes_clear_the_item() {
        let digest = sample_owner_digest();
        for mv in [Move::First, Move::Last, Move::By(2), Move::By(-1)] {
            let (mut view, stops, a, _) = walk_view(&digest);
            step_with_items(&mut view, &digest, &stops, true);
            assert!(view.item_cursor.is_some());
            move_cursor(&mut view, &stops, mv);
            assert_eq!(view.item_cursor, None, "{mv:?}");
            let _ = a;
        }
        let (mut view, stops, _, _) = walk_view(&digest);
        step_with_items(&mut view, &digest, &stops, true);
        let folds = selectable_folds(&digest);
        assert!(toggle(&mut view, &folds));
        assert_eq!(view.item_cursor, None, "Enter collapses and clears");
        // A refresh that keeps the item keeps it.
        let (mut view, stops, _, _) = walk_view(&digest);
        step_with_items(&mut view, &digest, &stops, true);
        step_with_items(&mut view, &digest, &stops, true);
        clamp(&mut view, &digest);
        assert_eq!(view.item_cursor.as_deref(), Some("check 1"));
        // A refresh that drops it falls back to its row.
        let mut changed = digest.clone();
        for group in &mut changed.groups {
            if group.phase == "21" && group.lane == Lane::Test {
                group.items.retain(|i| i.id != "check 1");
            }
        }
        clamp(&mut view, &changed);
        assert_eq!(view.item_cursor, None);
        assert!(matches!(view.cursor, Some(Stop::Row(_))));
        // So does a refresh that removes the row, or one that collapses it.
        let (mut view, stops, a, _) = walk_view(&digest);
        step_with_items(&mut view, &digest, &stops, true);
        view.expanded.remove(&a);
        clamp(&mut view, &digest);
        assert_eq!(view.item_cursor, None);
    }

    #[test]
    fn the_item_highlight_changes_no_text() {
        let digest = sample_owner_digest();
        let (mut view, stops, a, _) = walk_view(&digest);
        step_with_items(&mut view, &digest, &stops, true);
        step_with_items(&mut view, &digest, &stops, true);
        let model = block_model("sample", "v2.2", &digest, &view, 100);
        let (row_line, item_at) = (
            model.stop_lines[0].expect("row line"),
            model
                .item_lines
                .iter()
                .find(|(k, id, _)| *k == a && id == "check 1")
                .map(|(_, _, line)| *line)
                .expect("item line"),
        );
        let draw = |cursor: Cursor| {
            use ratatui::backend::TestBackend;
            use ratatui::Terminal;
            let height = block_height(&model, 40);
            let mut terminal = Terminal::new(TestBackend::new(100, height)).unwrap();
            terminal
                .draw(|f| render_owner_block(f, f.area(), &model, Some(cursor), 0))
                .unwrap();
            terminal.backend().buffer().clone()
        };
        let on_row = draw(Cursor {
            stop: 0,
            fold: None,
            item_line: None,
        });
        let on_item = draw(Cursor {
            stop: 0,
            fold: None,
            item_line: Some(item_at),
        });
        let line_of = |y: usize| usize::from(u16::try_from(y).unwrap());
        // Every cell's text is the same, except the row's `> ` gutter.
        for y in 1..on_row.area.height {
            for x in 0..on_row.area.width {
                let (a, b) = (on_row[(x, y)].clone(), on_item[(x, y)].clone());
                if line_of(usize::from(y) - 1) == row_line && x < 2 {
                    continue;
                }
                assert_eq!(a.symbol(), b.symbol(), "({x},{y})");
            }
        }
        let item_y = u16::try_from(item_at + 1).unwrap();
        let cell = &on_item[(6, item_y)];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
        // The item line has no gutter change and the row line is not the cursor.
        assert_eq!(on_item[(0, item_y)].symbol(), " ");
        let row_y = u16::try_from(row_line + 1).unwrap();
        assert_eq!(on_item[(0, row_y)].symbol(), " ", "no `> ` on the row");
        assert_ne!(on_item[(6, row_y)].fg, Color::Cyan);
        assert_eq!(on_row[(0, row_y)].symbol(), ">");
    }
}
