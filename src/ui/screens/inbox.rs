//! The Owner inbox screen (37-03, OWN-20, D-01..D-05).
//!
//! `i` on the dashboard opens one list of what waits on the owner across every
//! registered project: TEST that closes a phase, TEST that leaves the phase open,
//! DECIDE and FIX. `v` runs the selected group's GSD command in its project
//! ([`super::owner_actions`]). 37-04 completed the layout: the narrowing grid with
//! the FIX Command column, the header, fold-line and footer ladders, the lane
//! filters and the empty states; the expansion, opening the folds and the scroll
//! hint are 37-05.
//!
//! **Pure and in-memory (36 D-02, OWN-12).** Every frame is built from
//! `ctx.owner_digests` and `ctx.project_states`; nothing on the render path reads,
//! stats or parses a file. The rows are the Owner block's own rows
//! ([`owner_block::current_sources`], [`owner_block::row_cells`]), so the two
//! surfaces cannot disagree about what a row says.
//!
//! **The cursor is an identity (D-10).** It is the (alias, [`owner_block::GroupKey`])
//! of the selected row, resolved against the current rows on every frame and key,
//! so a digest refresh moves the selection to a neighbor instead of leaving it on
//! whatever row now sits at the old index.

use super::ack_confirm::AckHost;
use super::detail::{fit_cells, fit_spans, shown};
use super::driver::DRY_RUN_LOADING;
use super::owner::{owner_numbers, OWNER_SEP};
use super::owner_actions;
use super::owner_block::{self, blanks, cells, pad, Fold, GroupKey, RowCells, RowSrc};
use super::{AppContext, Screen, ScreenAction};
use crate::inbox::phase_key;
use crate::state_reader::owner_items::{
    is_owner_row, stale_totals, triage_entries, Lane, OwnerGroup, StaleTotals,
};
use crate::ui::glyphs;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::HashSet;

/// A row's identity across digest refreshes: the project and the group inside it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InboxKey {
    pub alias: String,
    pub key: GroupKey,
}

/// One cursor stop: a group row (current or in an open fold), or the pinned fold
/// line as a whole (its sub-cursor is `InboxViewState::fold_cursor`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InboxStop {
    Row(InboxKey),
    FoldLine,
}

/// Which lanes the inbox lists (D-03): the default is the four actionable
/// sections; `1`-`4` narrow to one lane and `a` adds BLOCKED. Pressing the active
/// key again returns to the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LaneFilter {
    #[default]
    Default,
    Test,
    Decide,
    Fix,
    Blocked,
    All,
}

impl LaneFilter {
    /// Whether `section` is listed under this filter.
    fn shows(self, section: Section) -> bool {
        match self {
            LaneFilter::Default => section != Section::Blocked,
            LaneFilter::Test => matches!(section, Section::TestCloses | Section::TestOpen),
            LaneFilter::Decide => section == Section::Decide,
            LaneFilter::Fix => section == Section::Fix,
            LaneFilter::Blocked => section == Section::Blocked,
            LaneFilter::All => true,
        }
    }

    /// The index of the header token this filter highlights; `None` for the
    /// default, which is silent.
    fn token(self) -> Option<usize> {
        match self {
            LaneFilter::Default => None,
            LaneFilter::Test => Some(0),
            LaneFilter::Decide => Some(1),
            LaneFilter::Fix => Some(2),
            LaneFilter::Blocked => Some(3),
            LaneFilter::All => Some(4),
        }
    }

    fn of_key(key: char) -> Option<Self> {
        match key {
            '1' => Some(LaneFilter::Test),
            '2' => Some(LaneFilter::Decide),
            '3' => Some(LaneFilter::Fix),
            '4' => Some(LaneFilter::Blocked),
            'a' => Some(LaneFilter::All),
            _ => None,
        }
    }

    /// `No {lane} work` and the key that brings the default back.
    fn empty_copy(self) -> String {
        let (lane, key) = match self {
            LaneFilter::Test => ("test", '1'),
            LaneFilter::Decide => ("decide", '2'),
            LaneFilter::Fix => ("fix", '3'),
            LaneFilter::Blocked => ("blocked", '4'),
            LaneFilter::All => return " No owner work in any lane".to_owned(),
            LaneFilter::Default => return " No owner work".to_owned(),
        };
        format!(" No {lane} work \u{b7} press {key} again for the default lanes")
    }
}

/// The inbox's view state: identities and offsets, never row indices alone.
#[derive(Debug, Clone, Default)]
pub struct InboxViewState {
    /// The lanes listed (D-03); in memory only.
    pub filter: LaneFilter,
    /// The stop the cursor rests on; `None` until the first key (the first row).
    pub cursor: Option<InboxStop>,
    /// The cursor's last index, so a vanished row clamps to its neighbor.
    pub cursor_index: usize,
    /// The first body line drawn.
    pub offset: usize,
    /// Groups whose lines are shown under their row (D-06); identities, never
    /// indices, in memory only.
    pub expanded: HashSet<InboxKey>,
    /// Open folds (D-02); Triage never opens.
    pub open_folds: HashSet<Fold>,
    /// The fold the fold line's sub-cursor rests on.
    pub fold_cursor: Option<Fold>,
}

/// The sections in D-02 order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Section {
    TestCloses,
    TestOpen,
    Decide,
    Fix,
    Blocked,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::TestCloses => "TEST \u{2014} closes a phase",
            Section::TestOpen => "TEST \u{2014} phase stays open (also needs fix or unblock)",
            Section::Decide => "DECIDE",
            Section::Fix => "FIX \u{2014} code needed",
            Section::Blocked => "BLOCKED",
        }
    }

    fn of(src: &RowSrc) -> Self {
        match src.group.lane {
            Lane::Test if src.group.closes_phase => Section::TestCloses,
            Lane::Test => Section::TestOpen,
            Lane::Decide => Section::Decide,
            Lane::Fix => Section::Fix,
            // Deferred never reaches `current_sources`.
            Lane::Blocked | Lane::Deferred => Section::Blocked,
        }
    }
}

/// One group row before layout.
#[derive(Clone)]
pub(crate) struct InboxRow {
    pub alias: String,
    pub src: RowSrc,
    pub section: Section,
    /// The project's phase progress, `(done, total)`.
    pub progress: (u32, u32),
    /// Open current-milestone phases with plans (`closes_after.phases_open`).
    pub phases_open: usize,
    /// `Some` for a row listed by an open fold (Old debt, Acknowledged), `None`
    /// for a current row.
    pub fold: Option<Fold>,
}

impl InboxRow {
    pub(crate) fn key(&self) -> InboxKey {
        InboxKey {
            alias: self.alias.clone(),
            key: self.src.key.clone(),
        }
    }
}

/// Every current row of every registered project that has a digest, unsorted.
///
/// Built from [`owner_block::inbox_sources`] (the triage model the CLI uses): a
/// same-phase UAT and VERIFICATION pair is one row (D-08) and a phase older than
/// `preferences.owner.fold_days` is left to the Stale fold (D-09). A project with no digest yet (or whose folder is gone) has no
/// rows: absent data is not clean data, and the loading copy is 37-04's.
pub(crate) fn inbox_rows(ctx: &AppContext) -> Vec<InboxRow> {
    let fold_days = ctx.config.preferences.owner_fold_days();
    let mut aliases: Vec<&String> = ctx.config.projects.keys().collect();
    aliases.sort();
    let mut rows = Vec::new();
    for alias in aliases {
        let Some(digest) = ctx.owner_digest(alias) else {
            continue;
        };
        let progress = ctx
            .project_states
            .get(alias.as_str())
            .map(|state| {
                let p = state.phase_progress();
                (p.done, p.total)
            })
            .unwrap_or((0, 0));
        let phases_open = digest.closes_after.phases_open.len();
        for src in owner_block::inbox_sources(digest, fold_days).0 {
            rows.push(InboxRow {
                alias: alias.clone(),
                section: Section::of(&src),
                src,
                progress,
                phases_open,
                fold: None,
            });
        }
    }
    rows
}

/// Oldest first, a row with no age after every row with one.
fn by_age(a: Option<i64>, b: Option<i64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(&x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Highest progress ratio first, by cross-multiplication (no float); a project
/// with no phases counts as ratio 0.
fn by_progress(a: (u32, u32), b: (u32, u32)) -> Ordering {
    let norm = |(done, total): (u32, u32)| {
        if total == 0 {
            (0u64, 1u64)
        } else {
            (u64::from(done), u64::from(total))
        }
    };
    let (ad, at) = norm(a);
    let (bd, bt) = norm(b);
    (bd * at).cmp(&(ad * bt))
}

/// D-05, one pure function: the sections in D-02 order; inside TEST the project
/// closest to its milestone close first (ties: fewer open phases), then oldest;
/// inside DECIDE, FIX and BLOCKED oldest first; every tie ends in alias, phase
/// key, kind, file and workstream, so the order is **total**: every permutation
/// of the same rows sorts to the same sequence.
pub(crate) fn sort_groups(rows: &mut [InboxRow]) {
    rows.sort_by(|a, b| {
        let test = matches!(a.section, Section::TestCloses | Section::TestOpen);
        a.section
            .cmp(&b.section)
            .then_with(|| {
                if test {
                    by_progress(a.progress, b.progress)
                        .then_with(|| a.phases_open.cmp(&b.phases_open))
                } else {
                    Ordering::Equal
                }
            })
            .then_with(|| by_age(a.src.group.age_days, b.src.group.age_days))
            .then_with(|| a.alias.cmp(&b.alias))
            .then_with(|| phase_key(&a.src.group.phase).cmp(&phase_key(&b.src.group.phase)))
            .then_with(|| a.src.key.kind.cmp(b.src.key.kind))
            .then_with(|| a.src.key.file.cmp(&b.src.key.file))
            .then_with(|| a.src.key.workstream.cmp(&b.src.key.workstream))
            .then_with(|| a.src.key.scope_label.cmp(&b.src.key.scope_label))
    });
}

/// The rows the screen lists, sorted and filtered: BLOCKED is not shown by
/// default (D-03).
pub(crate) fn visible_rows(ctx: &AppContext) -> Vec<InboxRow> {
    let mut rows = inbox_rows(ctx);
    sort_groups(&mut rows);
    let filter = ctx.inbox_view.filter;
    rows.retain(|row| filter.shows(row.section));
    rows
}

/// The folds the fold line offers, in line order (D-02, D-19): Old debt and
/// Acknowledged when they have content, Triage when todos were counted or could
/// not be read (it is a stop so a refusal can reach it, but it never opens).
pub(crate) fn fleet_folds(ctx: &AppContext) -> Vec<Fold> {
    let t = fold_totals(ctx);
    let mut folds = Vec::new();
    if t.old_debt > 0 {
        folds.push(Fold::OldDebt);
    }
    if t.stale.entries > 0 {
        folds.push(Fold::Stale);
    }
    if t.acknowledged > 0 {
        folds.push(Fold::Acknowledged);
    }
    if t.todos > 0 || t.todos_unreadable > 0 {
        folds.push(Fold::Todos);
    }
    folds
}

/// The rows an open `fold` lists over every project that has a digest: Old debt
/// by milestone (rank, then label) with each project's lane order inside, the
/// Acknowledged rows by project. Triage lists nothing.
fn fold_rows(ctx: &AppContext, fold: Fold) -> Vec<InboxRow> {
    let fold_days = ctx.config.preferences.owner_fold_days();
    let mut aliases: Vec<&String> = ctx.config.projects.keys().collect();
    aliases.sort();
    let mut rows = Vec::new();
    for alias in aliases {
        let Some(digest) = ctx.owner_digest(alias) else {
            continue;
        };
        let sources = match fold {
            Fold::OldDebt => owner_block::old_debt_sources(digest),
            Fold::Stale => owner_block::inbox_sources(digest, fold_days).1,
            Fold::Acknowledged => owner_block::acknowledged_sources(digest),
            Fold::Todos => Vec::new(),
        };
        let progress = (0, 0);
        for src in sources {
            rows.push(InboxRow {
                alias: alias.clone(),
                section: Section::of(&src),
                src,
                progress,
                phases_open: 0,
                fold: Some(fold),
            });
        }
    }
    if fold == Fold::Stale {
        sort_groups(&mut rows);
    }
    if fold == Fold::OldDebt {
        // Stable: the alias and lane order inside one milestone label survives.
        rows.sort_by_key(|row| {
            (
                owner_block::archived_rank(&row.src.group),
                owner_block::scope_label(&row.src.group.scope),
            )
        });
    }
    rows
}

/// Every cursor stop that is a group row, in body order: the listed current rows,
/// then the rows of the open folds (D-02).
pub(crate) fn stop_rows(ctx: &AppContext) -> Vec<InboxRow> {
    let mut rows = visible_rows(ctx);
    let folds = fleet_folds(ctx);
    for fold in [Fold::OldDebt, Fold::Stale, Fold::Acknowledged] {
        if ctx.inbox_view.open_folds.contains(&fold) && folds.contains(&fold) {
            rows.extend(fold_rows(ctx, fold));
        }
    }
    rows
}

/// The cursor as an index over the stops: `0..rows.len()` for a group row,
/// `rows.len()` for the fold line (present when a fold is selectable). A vanished
/// identity falls to the row that took its index, else the one before (D-10); no
/// cursor yet is the first stop. `None` when there is no stop at all.
pub(crate) fn cursor_pos(ctx: &AppContext, rows: &[InboxRow]) -> Option<usize> {
    let has_fold = !fleet_folds(ctx).is_empty();
    let count = rows.len() + usize::from(has_fold);
    if count == 0 {
        return None;
    }
    let view = &ctx.inbox_view;
    Some(match &view.cursor {
        Some(InboxStop::FoldLine) => count - 1,
        // A vanished row falls to the row that took its index, else the one
        // before it; the fold line only when no row is left (D-10).
        Some(InboxStop::Row(key)) => rows
            .iter()
            .position(|row| row.key() == *key)
            .unwrap_or_else(|| {
                if rows.is_empty() {
                    count - 1
                } else {
                    view.cursor_index.min(rows.len() - 1)
                }
            }),
        None => 0,
    })
}

/// Put the cursor on stop `to` (clamped): the row's identity, or the fold line.
fn set_cursor(ctx: &mut AppContext, rows: &[InboxRow], to: usize) {
    let folds = fleet_folds(ctx);
    let count = rows.len() + usize::from(!folds.is_empty());
    if count == 0 {
        ctx.inbox_view.cursor = None;
        ctx.inbox_view.cursor_index = 0;
        return;
    }
    let to = to.min(count - 1);
    if to < rows.len() {
        ctx.inbox_view.cursor = Some(InboxStop::Row(rows[to].key()));
    } else {
        ctx.inbox_view.cursor = Some(InboxStop::FoldLine);
        if !ctx
            .inbox_view
            .fold_cursor
            .is_some_and(|fold| folds.contains(&fold))
        {
            ctx.inbox_view.fold_cursor = folds.first().copied();
        }
    }
    ctx.inbox_view.cursor_index = to;
}

/// The fold the sub-cursor rests on: the stored one when it is still offered,
/// else the first.
fn sub_cursor(ctx: &AppContext) -> Option<Fold> {
    let folds = fleet_folds(ctx);
    ctx.inbox_view
        .fold_cursor
        .filter(|fold| folds.contains(fold))
        .or_else(|| folds.first().copied())
}

/// D-10: forget what a refresh made vanish. An expanded identity that no digest
/// lists any more leaves the set (even one a filter or a closed fold hides now,
/// which stays expanded while it exists); a fold with no content is closed.
fn prune_view(ctx: &mut AppContext) {
    let mut live: HashSet<InboxKey> = inbox_rows(ctx).iter().map(InboxRow::key).collect();
    for fold in [Fold::OldDebt, Fold::Stale, Fold::Acknowledged] {
        live.extend(fold_rows(ctx, fold).iter().map(InboxRow::key));
    }
    ctx.inbox_view.expanded.retain(|key| live.contains(key));
    let folds = fleet_folds(ctx);
    ctx.inbox_view
        .open_folds
        .retain(|fold| *fold != Fold::Todos && folds.contains(fold));
    if !ctx
        .inbox_view
        .fold_cursor
        .is_some_and(|fold| folds.contains(&fold))
    {
        ctx.inbox_view.fold_cursor = None;
    }
}

/// After a digest update (a watcher refresh, a first fill, a failure) move the
/// cursor's identity onto what resolves now (D-10, SC3): the group itself when it
/// survives, else the row that took its place (the next one), else the previous
/// one, else the fold line, else nothing. Expansion and open folds of surviving
/// groups stay. Without a cursor there is nothing to anchor: the first stop is
/// implicit.
pub(crate) fn reanchor(ctx: &mut AppContext) {
    prune_view(ctx);
    if ctx.inbox_view.cursor.is_none() {
        return;
    }
    let rows = stop_rows(ctx);
    match cursor_pos(ctx, &rows) {
        Some(at) => set_cursor(ctx, &rows, at),
        None => {
            ctx.inbox_view.cursor = None;
            ctx.inbox_view.cursor_index = 0;
        }
    }
}

/// What the screen knows about one registered project's owner data (37-04,
/// coordinator directive 2): absent data is either still expected or never
/// coming, and is never counted as clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigestState {
    /// A digest is held.
    Loaded,
    /// No digest yet; the first fill or the reparse is still expected.
    Loading,
    /// No digest and none will come: the reader panicked, or the folder or
    /// `.planning/` is gone.
    Unreadable,
}

pub(crate) fn digest_state(ctx: &AppContext, alias: &str) -> DigestState {
    if ctx.owner_digest(alias).is_some() {
        DigestState::Loaded
    } else if ctx.owner_state_unavailable(alias) || ctx.owner_digest_failed.contains(alias) {
        DigestState::Unreadable
    } else {
        DigestState::Loading
    }
}

/// How many registered projects are in each [`DigestState`]: `(loaded, loading,
/// unreadable)`.
fn state_counts(ctx: &AppContext) -> (usize, usize, usize) {
    let (mut loaded, mut loading, mut unreadable) = (0, 0, 0);
    for alias in ctx.config.projects.keys() {
        match digest_state(ctx, alias) {
            DigestState::Loaded => loaded += 1,
            DigestState::Loading => loading += 1,
            DigestState::Unreadable => unreadable += 1,
        }
    }
    (loaded, loading, unreadable)
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Narrowest the Checks column gets (`12 checks`); `120 checks` widens it.
const CHECKS_W: usize = 9;
/// Cells of the unit in a checks cell (` checks`): the digits take the rest.
const CHECKS_UNIT_W: usize = 7;
/// Width of the Age column (`203 d stale`).
const AGE_W: usize = 11;
/// The Age column shows from this width up (UI-SPEC narrowing order, step 2).
const AGE_MIN_W: usize = 80;
/// The FIX Command column's floor and cap (UI-SPEC "Row grid").
const COMMAND_MIN_W: usize = 12;
const COMMAND_MAX_W: usize = 44;
/// Cursor, marker and the space after them.
const GUTTER_W: usize = 3;
/// The phrase column's floor and cap (UI-SPEC "Row grid").
const PHRASE_MIN_W: usize = 12;
const PHRASE_MAX_W: usize = 32;
/// The cells a phrase keeps beside its extras before an extra gives way.
const PHRASE_KEEP_W: usize = 8;

/// One row's cells, escaped, for layout.
struct Cells {
    alias: String,
    section: Section,
    cells: RowCells,
    /// The checks count when the row's checks cell is drawn (aligned over the
    /// column by [`inbox::checks_cell`](crate::inbox::checks_cell)).
    checks_n: Option<usize>,
}

fn cells_of(ctx: &AppContext, row: &InboxRow) -> Option<Cells> {
    let digest = ctx.owner_digest(&row.alias)?;
    let mut cells = row.src.inbox_cells(digest);
    // Section 1 says "closes a phase" in its title: no per-row phrase there.
    if row.fold.is_none() && row.section == Section::TestCloses && !row.src.group.bookkeeping_only {
        cells.phrase.clear();
    }
    let checks_n = (!cells.checks.is_empty()).then_some(row.src.group.counts.checks);
    Some(Cells {
        alias: shown(&row.alias),
        section: row.section,
        cells,
        checks_n,
    })
}

/// The column widths of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grid {
    pub alias_w: usize,
    pub phase_w: usize,
    pub name_w: usize,
    /// The checks column: [`CHECKS_W`], or wider for a count of three digits.
    pub checks_w: usize,
    pub phrase_w: usize,
    pub age_w: usize,
    /// The FIX rows' command, after the age; 0 when it does not fit.
    pub command_w: usize,
}

impl Grid {
    fn total(&self) -> usize {
        GUTTER_W
            + self.alias_w
            + 2
            + self.phase_w
            + 2
            + self.name_w
            + 2
            + self.checks_w
            + if self.phrase_w > 0 {
                2 + self.phrase_w
            } else {
                0
            }
            + if self.age_w > 0 { 2 + self.age_w } else { 0 }
            + if self.command_w > 0 {
                2 + self.command_w
            } else {
                0
            }
    }

    fn checks_x(&self) -> usize {
        GUTTER_W + self.alias_w + 2 + self.phase_w + 2 + self.name_w + 2
    }

    fn age_x(&self) -> usize {
        self.checks_x()
            + self.checks_w
            + if self.phrase_w > 0 {
                2 + self.phrase_w
            } else {
                0
            }
            + 2
    }
}

/// The widths over **every** current row, filtered or not, so a later filter or an
/// expansion never reflows the columns. Natural widths, then the UI-SPEC narrowing
/// order (D-04): the FIX Command column only when it fits without squeezing any
/// other column, Age by width, then the phrase toward the room its extras need
/// (38-UI-REVIEW fix 3: `2 files` and label counts stay whole), the alias toward
/// 8, the name toward 8 and the phase cell toward its floor, the phrase toward 12,
/// the phase cell toward 5; whatever is still too wide is cut by `fit_spans` per
/// row.
fn grid_for_inbox(all: &[Cells], width: usize) -> Grid {
    let widest = |f: &dyn Fn(&Cells) -> usize| all.iter().map(f).max().unwrap_or(0);
    let phrase_nat = widest(&|c| phrase_cell_w(c));
    let digits =
        crate::inbox::count_digits(all.iter().filter_map(|c| c.checks_n).max().unwrap_or(0));
    let mut grid = Grid {
        alias_w: widest(&|c| cells(&c.alias)).clamp(8, 18),
        phase_w: widest(&|c| c.cells.phase_cell_width()).clamp(5, 18),
        name_w: all
            .iter()
            .filter(|c| c.section != Section::Blocked)
            .map(|c| cells(&c.cells.name))
            .max()
            .unwrap_or(0)
            .clamp(8, 32),
        checks_w: CHECKS_W.max(digits + CHECKS_UNIT_W),
        phrase_w: if phrase_nat == 0 {
            0
        } else {
            phrase_nat.max(PHRASE_MIN_W)
        },
        age_w: if width >= AGE_MIN_W { AGE_W } else { 0 },
        command_w: 0,
    };
    let command_natural = all
        .iter()
        .filter(|c| c.section == Section::Fix)
        .map(|c| cells(&c.cells.command))
        .max()
        .unwrap_or(0);
    if command_natural > 0 && grid.age_w > 0 {
        let natural = command_natural.clamp(COMMAND_MIN_W, COMMAND_MAX_W);
        // Measured with the phrase column at its pre-extras cap, so a row's labels
        // never cost the FIX rows their command.
        let capped = Grid {
            phrase_w: grid.phrase_w.min(PHRASE_MAX_W),
            ..grid
        };
        if capped.total() + 2 + natural <= width {
            grid.command_w = natural;
        }
    }
    // Phase identity outranks the other cells: the phase text must stay whole, so
    // the alias and the name give way before the phase cell does, and the phase
    // cell gives up workstream-name cells (never the phase number) before it
    // drops below the floor of 5.
    let floor = |keep: usize| {
        all.iter()
            .map(|c| c.cells.phase_cell_floor(keep))
            .max()
            .unwrap_or(0)
            .clamp(5, 18)
    };
    // (alias floor, name floor, workstream-name cells the phase cell keeps)
    let steps: [(usize, usize, usize); 4] = [(10, 16, 12), (8, 12, 8), (8, 8, 4), (8, 8, 2)];
    // The extras (`2 files`, label counts) keep their room until the other
    // columns have given way; only then does the phrase drop to its floor.
    let extras_floor = widest(&|c| extras_floor_w(c)).clamp(PHRASE_MIN_W, PHRASE_MAX_W);
    while grid.total() > width && grid.phrase_w > extras_floor {
        grid.phrase_w -= 1;
    }
    for (alias_min, name_min, keep) in steps {
        while grid.total() > width && grid.alias_w > alias_min {
            grid.alias_w -= 1;
        }
        while grid.total() > width && grid.name_w > name_min {
            grid.name_w -= 1;
        }
        let min_phase = floor(keep).min(grid.phase_w);
        while grid.total() > width && grid.phase_w > min_phase {
            grid.phase_w -= 1;
        }
    }
    while grid.total() > width && grid.phrase_w > PHRASE_MIN_W {
        grid.phrase_w -= 1;
    }
    while grid.total() > width && grid.phase_w > 5 {
        grid.phase_w -= 1;
    }
    grid
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The phrase column's natural width: the phrase (capped at [`PHRASE_MAX_W`]),
/// then two cells and the extras (`2 files`, label counts) when the row has any.
fn phrase_cell_w(c: &Cells) -> usize {
    let extras = cells(&c.cells.extras);
    let phrase = cells(&c.cells.phrase).min(PHRASE_MAX_W);
    if extras == 0 {
        phrase
    } else if phrase == 0 {
        extras
    } else {
        phrase + 2 + extras
    }
}

/// The phrase column a row needs to keep all its extras whole beside the first
/// [`PHRASE_KEEP_W`] cells of its phrase; 0 for a row without extras.
fn extras_floor_w(c: &Cells) -> usize {
    let extras = cells(&c.cells.extras);
    let phrase = cells(&c.cells.phrase);
    match (extras, phrase) {
        (0, _) => 0,
        (_, 0) => extras,
        _ => phrase.min(PHRASE_KEEP_W) + 2 + extras,
    }
}

/// A row's phrase cell in `w` cells: the phrase, then the extras that fit
/// **whole** (`2 files` first, then each label count), the phrase cut to the
/// rest. An extra that does not fit is dropped, never cut mid-label, and leaves
/// ` · …` after the last whole one (Enter lists each item's label); the phrase
/// keeps at least [`PHRASE_KEEP_W`] cells (or all of it) before an extra gives
/// way. Returns `(phrase, extras)`.
fn phrase_and_extras(c: &Cells, w: usize) -> (String, String) {
    let phrase = &c.cells.phrase;
    if c.cells.extras.is_empty() {
        return (fit_cells(phrase, w), String::new());
    }
    let phrase_w = cells(phrase);
    let (keep, gap) = if phrase_w == 0 {
        (0, 0)
    } else {
        (phrase_w.min(PHRASE_KEEP_W), 2)
    };
    let parts: Vec<&str> = c.cells.extras.split(OWNER_SEP).collect();
    // A dropped extra leaves a trace: ` · …` after the last whole one.
    let extras = (1..=parts.len())
        .rev()
        .map(|n| {
            let mut text = parts[..n].join(OWNER_SEP);
            if n < parts.len() {
                text.push_str(OWNER_SEP);
                text.push('\u{2026}');
            }
            text
        })
        .find(|text| keep + gap + cells(text) <= w)
        .unwrap_or_default();
    if extras.is_empty() {
        return (fit_cells(phrase, w), String::new());
    }
    let room = w.saturating_sub(gap + cells(&extras));
    (fit_cells(phrase, room), extras)
}

/// One group row as spans, cut to `width`, the cursor row Cyan + BOLD throughout.
fn row_line(c: &Cells, grid: &Grid, width: usize, cursor: bool, expanded: bool) -> Line<'static> {
    let plain = owner_block::style_for(&c.cells, Style::default());
    let mut spans: Vec<Span<'static>> = vec![
        Span::raw(if cursor { ">" } else { " " }),
        Span::raw(if expanded {
            glyphs::TRIANGLE_DOWN_SMALL
        } else {
            glyphs::TRIANGLE_RIGHT_SMALL
        }),
        Span::raw(" "),
        Span::styled(pad(&c.alias, grid.alias_w, false), plain),
        Span::raw("  "),
    ];
    let (prefix, rest) = owner_block::phase_cell(&c.cells, grid.phase_w);
    let gap = blanks(grid.phase_w.saturating_sub(cells(&prefix) + cells(&rest)));
    if !prefix.is_empty() {
        spans.push(Span::styled(
            prefix,
            owner_block::style_for(
                &c.cells,
                Style::default().fg(ratatui::style::Color::Magenta),
            ),
        ));
    }
    spans.push(Span::styled(format!("{rest}{gap}"), plain));
    spans.push(Span::raw("  "));
    if c.section == Section::Blocked {
        // The tally spills over the columns to its right.
        let used = GUTTER_W + grid.alias_w + 2 + grid.phase_w + 2;
        spans.push(Span::styled(
            fit_cells(&c.cells.name, width.saturating_sub(used)),
            plain,
        ));
    } else {
        spans.push(Span::styled(pad(&c.cells.name, grid.name_w, false), plain));
        spans.push(Span::raw("  "));
        let checks = c.checks_n.map_or_else(String::new, |n| {
            crate::inbox::checks_cell(n, grid.checks_w - CHECKS_UNIT_W)
        });
        spans.push(Span::styled(pad(&checks, grid.checks_w, true), plain));
        if grid.phrase_w > 0 {
            spans.push(Span::raw("  "));
            // The phrase, then its extras (`2 files`, label counts) DIM; the
            // extras are kept whole and the phrase gives way first.
            let (phrase, extras) = phrase_and_extras(c, grid.phrase_w);
            let mut used = cells(&phrase);
            spans.push(Span::styled(phrase, plain));
            if !extras.is_empty() {
                let gap = if used == 0 { 0 } else { 2 };
                spans.push(Span::raw(blanks(gap)));
                used += gap + cells(&extras);
                spans.push(Span::styled(extras, plain.add_modifier(Modifier::DIM)));
            }
            spans.push(Span::raw(blanks(grid.phrase_w.saturating_sub(used))));
        }
        if grid.age_w > 0 {
            spans.push(Span::raw("  "));
            match &c.cells.age {
                Some(age) => {
                    let age_spans = owner_block::age_spans(age, &c.cells);
                    let used: usize = age_spans.iter().map(Span::width).sum();
                    spans.extend(age_spans);
                    spans.push(Span::raw(blanks(grid.age_w.saturating_sub(used))));
                }
                None => spans.push(Span::raw(blanks(grid.age_w))),
            }
        }
        if grid.command_w > 0 && c.section == Section::Fix && !c.cells.command.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                fit_cells(&c.cells.command, grid.command_w),
                plain,
            ));
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
    if cursor {
        for span in &mut spans {
            span.style = span.style.patch(owner_block::cursor_style());
        }
    }
    fit_spans(spans, width)
}

/// A section title: BOLD (DIM for BLOCKED). The first title also carries the DIM
/// column heads `checks` and `age`, when they fit after it.
fn title_line(section: Section, heads: Option<&Grid>, width: usize) -> Line<'static> {
    let style = if section == Section::Blocked {
        dim()
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let title = format!(" {}", section.title());
    let mut spans = vec![Span::styled(title.clone(), style)];
    if let Some(grid) = heads {
        let at = cells(&title);
        if at < grid.checks_x().saturating_sub(1) {
            spans.push(Span::raw(blanks(grid.checks_x() - at)));
            spans.push(Span::styled(pad("checks", grid.checks_w, true), dim()));
            if grid.age_w > 0 {
                let used = grid.checks_x() + grid.checks_w;
                spans.push(Span::raw(blanks(grid.age_x().saturating_sub(used))));
                spans.push(Span::styled("age", dim()));
            }
        }
    }
    fit_spans(spans, width)
}

/// One line of the body: a section title, a milestone sub-header, the row with
/// that index (over the stop rows), or a line under an expanded row.
#[derive(Debug, Clone, PartialEq)]
enum BodyItem {
    Title(Section),
    /// An archived milestone's label above its Old debt rows (DIM, escaped).
    Sub(String),
    Row(usize),
    Extra(Line<'static>),
}

/// The body's lines over the stop rows: titles in front of each section, a
/// sub-header in front of each Old debt milestone, `extra(row)` under a row.
fn build_plan(
    rows: &[InboxRow],
    extra: &mut dyn FnMut(&InboxRow) -> Vec<Line<'static>>,
) -> Vec<BodyItem> {
    let mut plan = Vec::new();
    let mut last: Option<Section> = None;
    let mut last_label: Option<Option<String>> = None;
    for (index, row) in rows.iter().enumerate() {
        if row.fold.is_none() {
            if last != Some(row.section) {
                plan.push(BodyItem::Title(row.section));
                last = Some(row.section);
            }
        } else if row.fold == Some(Fold::OldDebt) {
            let label = owner_block::scope_label(&row.src.group.scope);
            if last_label.as_ref() != Some(&label) {
                plan.push(BodyItem::Sub(
                    label.as_deref().map(shown).unwrap_or_default(),
                ));
                last_label = Some(label);
            }
        }
        plan.push(BodyItem::Row(index));
        plan.extend(extra(row).into_iter().map(BodyItem::Extra));
    }
    plan
}

/// The body's lines with nothing expanded.
#[cfg(test)]
fn body_plan(rows: &[InboxRow]) -> Vec<BodyItem> {
    build_plan(rows, &mut |_| Vec::new())
}

/// The group a key acts on for `row`, looked up by identity as the key is pressed:
/// the digest's own group, else the entry a merged row stands for, else the row's
/// own group (a BLOCKED aggregate has no single group; the run path refuses it by
/// lane).
fn row_group(ctx: &AppContext, row: &InboxRow) -> OwnerGroup {
    let fold_days = ctx.config.preferences.owner_fold_days();
    ctx.owner_digest(&row.alias)
        .and_then(|digest| {
            owner_block::group_for(digest, &row.src.key).or_else(|| {
                owner_block::entry_for(digest, &row.src.key, fold_days).map(|entry| entry.group)
            })
        })
        .unwrap_or_else(|| row.src.group.clone())
}

/// The lines under an expanded row (D-06): the Owner block's own formatter one
/// cell further right (the inbox gutter is three cells), then, inbox only, the
/// exact command `v` would launch.
fn expansion_for(ctx: &AppContext, row: &InboxRow, width: usize) -> Vec<Line<'static>> {
    let mut lines = owner_block::inbox_expansion_lines(&row.src, width, 1);
    let group = row_group(ctx, row);
    if let Ok(plan) = owner_actions::run_plan(ctx, &row.alias, &group) {
        lines.push(owner_block::dim_line(
            5,
            &format!("run {}", shown(&plan.command.to_string())),
            width,
        ));
    }
    if row.fold == Some(Fold::Acknowledged) {
        for line in &mut lines {
            for span in &mut line.spans {
                span.style = span.style.add_modifier(Modifier::DIM);
            }
        }
    }
    lines
}

/// [`build_plan`] with the expanded rows' lines at `width`.
fn full_plan(ctx: &AppContext, rows: &[InboxRow], width: usize) -> Vec<BodyItem> {
    build_plan(rows, &mut |row| {
        if ctx.inbox_view.expanded.contains(&row.key()) {
            expansion_for(ctx, row, width)
        } else {
            Vec::new()
        }
    })
}

/// The first body line drawn so the cursor row is visible, keeping `offset` when
/// it already is. A row that opens its section (or milestone) brings its title
/// with it, and its expansion stays in view when it fits.
fn scroll_for(offset: usize, plan: &[BodyItem], cursor_row: usize, height: usize) -> usize {
    if height == 0 {
        return 0;
    }
    let line = plan
        .iter()
        .position(|item| matches!(item, BodyItem::Row(index) if *index == cursor_row))
        .unwrap_or(0);
    let top = if line > 0 && matches!(plan[line - 1], BodyItem::Title(_) | BodyItem::Sub(_)) {
        line - 1
    } else {
        line
    };
    let mut end = line;
    while end + 1 < plan.len() && matches!(plan[end + 1], BodyItem::Extra(_)) {
        end += 1;
    }
    let end = end.min(line + height - 1);
    let mut offset = offset;
    if end >= offset + height {
        offset = end + 1 - height;
    }
    if top < offset {
        offset = top;
    }
    if line >= offset + height {
        offset = line + 1 - height;
    }
    offset.min(plan.len().saturating_sub(height))
}

/// The header's left text (D-02) over every registered project that has a digest.
/// The degenerate states have their own words: no project, or no digest at all.
fn header_left(ctx: &AppContext, with_projects: bool, with_blocked: bool) -> String {
    let count = ctx.config.projects.len();
    let projects = format!(
        "{count} {}",
        if count == 1 { "project" } else { "projects" }
    );
    let (loaded, loading, _) = state_counts(ctx);
    if count == 0 {
        return format!(" Owner inbox \u{2500}\u{2500} {projects}");
    }
    if loaded == 0 {
        return if loading > 0 {
            " Owner inbox \u{2500}\u{2500} loading".to_owned()
        } else {
            format!(" Owner inbox \u{2500}\u{2500} {projects}")
        };
    }
    let (mut test, mut decide, mut fix, mut blocked) = (0usize, 0usize, 0usize, 0usize);
    let mut phases_only = true;
    for alias in ctx.config.projects.keys() {
        let Some(digest) = ctx.owner_digest(alias) else {
            continue;
        };
        let numbers = owner_numbers(digest);
        test += numbers.test;
        decide += numbers.decide;
        fix += numbers.fix;
        phases_only &= numbers.fix_is_phases;
        blocked += digest.blocked;
    }
    let fix_text = if phases_only {
        format!("fix {fix} {}", if fix == 1 { "phase" } else { "phases" })
    } else {
        format!("fix {fix}")
    };
    let projects = if with_projects {
        format!("{projects}{OWNER_SEP}")
    } else {
        String::new()
    };
    let blocked = if with_blocked {
        format!("{OWNER_SEP}blocked {blocked}")
    } else {
        String::new()
    };
    format!(
        " Owner inbox \u{2500}\u{2500} {projects}test {test}{OWNER_SEP}decide {decide}\
         {OWNER_SEP}{fix_text}{blocked}"
    )
}

/// The lane keys on the right: `[n]` BOLD, the name plain; the active token
/// REVERSED (D-03).
fn lane_keys(short: bool, active: Option<usize>) -> Vec<Span<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let names: [(&str, &str, &str); 5] = [
        ("[1]", "Test", "T"),
        ("[2]", "Decide", "D"),
        ("[3]", "Fix", "F"),
        ("[4]", "Blocked", "B"),
        ("[a]", "ll", "ll"),
    ];
    let mut spans = Vec::new();
    for (index, (key, long, abbr)) in names.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let mark = if active == Some(index) {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        };
        spans.push(Span::styled((*key).to_owned(), bold.add_modifier(mark)));
        spans.push(Span::styled(
            (if short { *abbr } else { *long }).to_owned(),
            Style::default().add_modifier(mark),
        ));
    }
    spans
}

/// The header (UI-SPEC "Header ladder", six rungs): the first rung that fits
/// wins. A rung first drops the DIM ` · {U} unreadable` clause, then
/// ` · {M} loading`, before the next rung is tried.
fn header_line(ctx: &AppContext, width: usize, active: Option<usize>) -> Line<'static> {
    let (_, loading, unreadable) = state_counts(ctx);
    let clause = |n: usize, word: &str| {
        (n > 0).then(|| Span::styled(format!("{OWNER_SEP}{n} {word}"), dim()))
    };
    let loading_clause = clause(loading, "loading");
    let unreadable_clause = clause(unreadable, "unreadable");
    let levels: [Vec<Span<'static>>; 3] = [
        loading_clause
            .iter()
            .chain(unreadable_clause.iter())
            .cloned()
            .collect(),
        loading_clause.iter().cloned().collect(),
        Vec::new(),
    ];
    // (project count, blocked clause, lane keys: Some(short) or none)
    let rungs: [(bool, bool, Option<bool>); 5] = [
        (true, true, Some(false)),
        (false, true, Some(false)),
        (false, true, Some(true)),
        (false, false, Some(true)),
        (false, false, None),
    ];
    for (with_projects, with_blocked, keys) in rungs {
        let left = header_left(ctx, with_projects, with_blocked);
        let right = keys.map(|short| lane_keys(short, active));
        let right_w = right
            .as_ref()
            .map_or(0, |r| r.iter().map(Span::width).sum::<usize>());
        for clauses in &levels {
            let left_w = cells(&left) + clauses.iter().map(Span::width).sum::<usize>();
            let fits = match &right {
                Some(_) => left_w + 2 + right_w <= width,
                None => left_w <= width,
            };
            if !fits {
                continue;
            }
            let mut spans = vec![Span::raw(left)];
            spans.extend(clauses.iter().cloned());
            if let Some(right) = right {
                spans.push(Span::raw(blanks(width - left_w - right_w)));
                spans.extend(right);
            }
            return Line::from(spans);
        }
    }
    fit_spans(vec![Span::raw(header_left(ctx, false, false))], width)
}

/// The body copy while no row is listed (UI-SPEC "Empty, loading and error
/// states"), all DIM. Absent data is `loading` or `unreadable`, never clean.
fn empty_copy(ctx: &AppContext) -> String {
    let (loaded, loading, unreadable) = state_counts(ctx);
    if ctx.config.projects.is_empty() {
        " No projects registered. Press Esc, then a to add one.".to_owned()
    } else if loaded == 0 && loading > 0 {
        format!(" {DRY_RUN_LOADING}")
    } else if loaded == 0 {
        format!(" No project could be read ({unreadable} unreadable)")
    } else {
        ctx.inbox_view.filter.empty_copy()
    }
}

/// One footer hint: the bracketed key and its label at the three label rungs.
struct FooterKey {
    key: &'static str,
    /// Rung 1 (long), rung 2 (short), rung 3 and 4 (shortest) labels.
    labels: [&'static str; 3],
    /// Kept from rung 3 on.
    rung3: bool,
    /// Kept at the minimal rung 4.
    rung4: bool,
}

/// The hints in the UI-SPEC footer order, **only for keys that work**: later
/// plans insert their key at its UI-SPEC position (`A`, `y`) when it works.
const FOOTER_KEYS: [FooterKey; 9] = [
    FooterKey {
        key: "[j/k]",
        labels: ["move", "move", "move"],
        rung3: true,
        rung4: false,
    },
    FooterKey {
        key: "[Enter]",
        labels: ["expand", "expand", "expand"],
        rung3: true,
        rung4: false,
    },
    FooterKey {
        key: "[g]",
        labels: ["o to project", "oto", "oto"],
        rung3: true,
        rung4: false,
    },
    FooterKey {
        key: "[v]",
        labels: ["run command", "run", "run"],
        rung3: true,
        rung4: true,
    },
    FooterKey {
        key: "[o]",
        labels: ["pen file", "pen", "pen"],
        rung3: true,
        rung4: false,
    },
    FooterKey {
        key: "[A]",
        labels: ["cknowledge", "cknowledge", "ck"],
        rung3: true,
        rung4: true,
    },
    FooterKey {
        key: "[y]",
        labels: ["copy checklist", "copy", "copy"],
        rung3: true,
        rung4: true,
    },
    FooterKey {
        key: "[Esc]",
        labels: ["back", "back", "back"],
        rung3: false,
        rung4: false,
    },
    FooterKey {
        key: "[?]",
        labels: ["help", "help", "help"],
        rung3: true,
        rung4: true,
    },
];

/// The hints while the cursor rests on the fold line.
fn fold_footer(rung: usize) -> Vec<Span<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hints: &[(&str, &str)] = if rung == 0 {
        &[
            ("[\u{2190}/\u{2192}]", "fold"),
            ("[Enter]", "expand"),
            ("[y]", "copy"),
            ("[j/k]", "move"),
            ("[Esc]", "back"),
            ("[?]", "help"),
        ]
    } else {
        &[
            ("[\u{2190}/\u{2192}]", "fold"),
            ("[Enter]", "expand"),
            ("[y]", "copy"),
            ("[j/k]", "move"),
            ("[?]", "help"),
        ]
    };
    let mut spans = vec![Span::raw("  ")];
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled((*key).to_owned(), bold));
        spans.push(Span::raw((*label).to_owned()));
    }
    spans
}

/// The hints of one rung, two leading spaces, `[key]` BOLD.
fn footer_rung(rung: usize, has_stops: bool) -> Vec<Span<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::raw("  ")];
    let mut first = true;
    for hint in &FOOTER_KEYS {
        // With no stop (an empty inbox) only the keys that need none remain.
        let keep = match hint.key {
            "[Esc]" | "[?]" => true,
            _ => has_stops,
        };
        let kept = match rung {
            0 | 1 => true,
            2 => hint.rung3,
            _ => hint.rung4,
        };
        if !keep || !kept {
            continue;
        }
        if !first {
            spans.push(Span::raw("  "));
        }
        first = false;
        spans.push(Span::styled(hint.key.to_owned(), bold));
        spans.push(Span::raw(hint.labels[rung.min(2)].to_owned()));
    }
    spans
}

/// The footer: the status message while one is live, else the first rung of the
/// hint ladder that fits `width` (the fold-line hints while the cursor rests on
/// the fold line).
fn footer_line(ctx: &AppContext, width: usize, has_stops: bool, on_fold: bool) -> Line<'static> {
    if let Some((message, _)) = &ctx.status_message {
        // The status line is composed from authored copy and project text in one
        // string, so the escape happens here, at the consumer (as on the dashboard).
        return Line::from(Span::raw(fit_cells(&shown(message), width)));
    }
    let mut last = Vec::new();
    for rung in 0..4 {
        let spans = if on_fold {
            if rung > 1 {
                break;
            }
            fold_footer(rung)
        } else {
            footer_rung(rung, has_stops)
        };
        if spans.iter().map(Span::width).sum::<usize>() <= width {
            return Line::from(spans);
        }
        last = spans;
    }
    fit_spans(last, width)
}

// ---------------------------------------------------------------------------
// Fold line (D-02, D-19), collapsed
// ---------------------------------------------------------------------------

/// The fleet totals behind the pinned fold line, over every registered project
/// that has a digest: what the CLI's folds count (`inbox::render_text`).
#[derive(Debug, Default, PartialEq, Eq)]
struct FoldTotals {
    old_debt: usize,
    old_projects: usize,
    /// Archived open questions are not checks: `items` once any are counted.
    old_unit_items: bool,
    /// The entries older than `fold_days`, summed over every project (D-09).
    stale: StaleTotals,
    acknowledged: usize,
    todos: usize,
    todo_projects: usize,
    todos_unreadable: usize,
}

fn fold_totals(ctx: &AppContext) -> FoldTotals {
    let fold_days = ctx.config.preferences.owner_fold_days();
    let mut totals = FoldTotals::default();
    for alias in ctx.config.projects.keys() {
        let Some(digest) = ctx.owner_digest(alias) else {
            continue;
        };
        let old = digest.archived.test
            + digest.archived.blocked
            + digest.archived.fix
            + digest.archived.decide;
        totals.old_debt += old;
        totals.old_projects += usize::from(old > 0);
        totals.old_unit_items |= digest.archived.decide > 0;
        let stale = stale_totals(
            triage_entries(digest, fold_days)
                .iter()
                .filter(|entry| is_owner_row(&entry.group)),
        );
        totals.stale.entries += stale.entries;
        totals.stale.items += stale.items;
        totals.stale.non_check |= stale.non_check;
        totals.acknowledged += owner_block::acknowledged_files(digest);
        totals.todos += digest.hidden.todos;
        totals.todo_projects += usize::from(digest.hidden.todos > 0);
        totals.todos_unreadable += usize::from(digest.hidden.todos_unreadable);
    }
    totals
}

impl FoldTotals {
    fn is_empty(&self) -> bool {
        self.old_debt == 0
            && self.stale.entries == 0
            && self.acknowledged == 0
            && self.todos == 0
            && self.todos_unreadable == 0
    }
}

/// The pinned fold line (UI-SPEC "Fold-line ladder"): the first of three rungs
/// that fits, folds without content omitted, then `fit_cells` on the shortest.
/// `None` when no fold has content. An open fold's marker is the down triangle.
#[cfg(test)]
fn fold_line(ctx: &AppContext, width: usize) -> Option<Line<'static>> {
    fold_line_at(ctx, width, None)
}

/// [`fold_line`], and when the cursor rests on it (`cursor` is the sub-cursor's
/// fold) the `>` gutter and the sub-cursor's fold in Cyan + BOLD.
fn fold_line_at(
    ctx: &AppContext,
    width: usize,
    cursor: Option<Option<Fold>>,
) -> Option<Line<'static>> {
    let t = fold_totals(ctx);
    if t.is_empty() {
        return None;
    }
    let plural = crate::inbox::plural;
    let unit = |n: usize| {
        if t.old_unit_items {
            plural(n, "item", "items")
        } else {
            plural(n, "check", "checks")
        }
    };
    let projects = |n: usize| plural(n, "project", "projects");
    let all_unreadable = t.todos == 0;
    let fold_days = ctx.config.preferences.owner_fold_days();
    // (old debt, acknowledged, triage) texts of one rung; the third is DIM.
    let rung = |n: usize| -> Vec<(String, bool, Fold)> {
        let mut parts = Vec::new();
        if t.old_debt > 0 {
            parts.push((
                match n {
                    0 => format!(
                        "Old debt: {} in {} (archived milestones)",
                        unit(t.old_debt),
                        projects(t.old_projects)
                    ),
                    1 => format!(
                        "Old debt: {} in {}",
                        unit(t.old_debt),
                        projects(t.old_projects)
                    ),
                    _ => format!("Old debt ({})", t.old_debt),
                },
                false,
                Fold::OldDebt,
            ));
        }
        if t.stale.entries > 0 {
            parts.push((
                match n {
                    0 => crate::inbox::stale_fold_text(&t.stale, fold_days),
                    1 => format!(
                        "Stale: {}",
                        plural(
                            t.stale.items,
                            if t.stale.non_check { "item" } else { "check" },
                            if t.stale.non_check { "items" } else { "checks" }
                        )
                    ),
                    // Items, as Old debt counts checks (38-UI-REVIEW fix 6).
                    _ => format!("Stale ({})", t.stale.items),
                },
                false,
                Fold::Stale,
            ));
        }
        if t.acknowledged > 0 {
            parts.push((
                match n {
                    0 | 1 => format!("Acknowledged ({})", plural(t.acknowledged, "file", "files")),
                    _ => format!("Acknowledged ({})", t.acknowledged),
                },
                false,
                Fold::Acknowledged,
            ));
        }
        if t.todos > 0 || t.todos_unreadable > 0 {
            let text = match (n, all_unreadable) {
                (0 | 1, true) => "Triage: ?".to_owned(),
                (_, true) => "Triage ?".to_owned(),
                (0, false) => {
                    let mut text = format!(
                        "Triage: {} in {}",
                        plural(t.todos, "pending todo", "pending todos"),
                        projects(t.todo_projects)
                    );
                    if t.todos_unreadable > 0 {
                        text.push_str(&format!(" (+{} unreadable)", t.todos_unreadable));
                    }
                    text
                }
                (1, false) => format!(
                    "Triage: {} in {}",
                    plural(t.todos, "todo", "todos"),
                    projects(t.todo_projects)
                ),
                (_, false) => format!("Triage {}", t.todos),
            };
            parts.push((text, true, Fold::Todos));
        }
        parts
    };
    let build = |parts: Vec<(String, bool, Fold)>| -> Vec<Span<'static>> {
        let mut spans = vec![if cursor.is_some() {
            Span::styled(">", owner_block::cursor_style())
        } else {
            Span::raw(" ")
        }];
        for (index, (text, muted, fold)) in parts.into_iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw("   "));
            }
            let mut style = if muted { dim() } else { Style::default() };
            if cursor == Some(Some(fold)) {
                style = style.patch(owner_block::cursor_style());
            }
            let marker = if ctx.inbox_view.open_folds.contains(&fold) {
                glyphs::TRIANGLE_DOWN_SMALL
            } else {
                glyphs::TRIANGLE_RIGHT_SMALL
            };
            spans.push(Span::styled(format!("{marker} {text}"), style));
        }
        spans
    };
    for n in 0..2 {
        let spans = build(rung(n));
        if spans.iter().map(Span::width).sum::<usize>() <= width {
            return Some(Line::from(spans));
        }
    }
    Some(fit_spans(build(rung(2)), width))
}

// ---------------------------------------------------------------------------
// The screen
// ---------------------------------------------------------------------------

/// The Owner inbox.
pub struct InboxScreen {
    /// The body height of the last frame, for `PageUp`/`PageDown` and the scroll.
    viewport: Cell<usize>,
    /// The width of the last frame: expansion lines are cut to it, and the
    /// scroll counts them.
    width: Cell<usize>,
}

/// The rows under the body.
#[derive(Clone)]
struct Pinned {
    rule: Rect,
    fold: Rect,
    footer: Rect,
    has_stops: bool,
    /// The cursor rests on the fold line.
    on_fold: bool,
    /// ` {first}-{last} of {total}` while body rows are clipped.
    hint: Option<String>,
}

impl InboxScreen {
    /// The rule row, the pinned fold line and the footer under the body.
    fn render_pinned(&self, frame: &mut Frame, ctx: &AppContext, pinned: Pinned, width: usize) {
        if pinned.rule.height > 0 {
            let mut spans = Vec::new();
            let mut rule_w = width;
            if let Some(hint) = pinned.hint.as_deref().filter(|h| cells(h) < width) {
                rule_w = width - cells(hint);
                spans.push(Span::styled("\u{2500}".repeat(rule_w), dim()));
                spans.push(Span::styled(hint.to_owned(), dim()));
            } else {
                spans.push(Span::styled("\u{2500}".repeat(rule_w), dim()));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), pinned.rule);
        }
        let cursor = pinned.on_fold.then(|| sub_cursor(ctx));
        if let (Some(fold), true) = (fold_line_at(ctx, width, cursor), pinned.fold.height > 0) {
            frame.render_widget(Paragraph::new(fold), pinned.fold);
        }
        frame.render_widget(
            Paragraph::new(footer_line(ctx, width, pinned.has_stops, pinned.on_fold)),
            pinned.footer,
        );
    }

    pub fn new() -> Self {
        Self {
            viewport: Cell::new(0),
            width: Cell::new(80),
        }
    }

    /// Scroll so the row at stop `at` (with its expansion) is in view; the fold
    /// line is pinned and never scrolls.
    fn follow(&self, ctx: &mut AppContext, rows: &[InboxRow], at: usize) {
        if at >= rows.len() {
            return;
        }
        let plan = full_plan(ctx, rows, self.width.get());
        ctx.inbox_view.offset =
            scroll_for(ctx.inbox_view.offset, &plan, at, self.viewport.get().max(1));
    }

    /// Move the cursor to stop `to` (clamped) and keep it in view.
    fn move_to(&self, ctx: &mut AppContext, rows: &[InboxRow], to: usize) {
        set_cursor(ctx, rows, to);
        let at = ctx.inbox_view.cursor_index;
        self.follow(ctx, rows, at);
    }

    /// `1`-`4` and `a` (D-03): the same key again returns to the default lanes.
    /// The cursor keeps its identity when its row is still listed, else falls to
    /// the neighbor that took its index.
    fn filter_key(&self, ctx: &mut AppContext, key: char) {
        let Some(pressed) = LaneFilter::of_key(key) else {
            return;
        };
        ctx.inbox_view.filter = if ctx.inbox_view.filter == pressed {
            LaneFilter::Default
        } else {
            pressed
        };
        reanchor(ctx);
        ctx.inbox_view.offset = 0;
        ctx.needs_redraw = true;
    }

    fn step(&self, ctx: &mut AppContext, by: isize) {
        let rows = stop_rows(ctx);
        let Some(at) = cursor_pos(ctx, &rows) else {
            return;
        };
        let to = at.saturating_add_signed(by);
        self.move_to(ctx, &rows, to);
        ctx.needs_redraw = true;
    }

    /// The group row under the cursor, `Ok(None)` when the cursor is on the fold
    /// line, `Err(())` when there is no stop (the key is then ignored).
    fn selected(ctx: &AppContext) -> Result<Option<InboxRow>, ()> {
        let rows = stop_rows(ctx);
        match cursor_pos(ctx, &rows) {
            None => Err(()),
            Some(at) => Ok(rows.get(at).cloned()),
        }
    }

    fn say(ctx: &mut AppContext, message: String) {
        ctx.status_message = Some((message, std::time::Instant::now()));
        ctx.needs_redraw = true;
    }

    /// `Enter` / `Space` (D-06): toggle the group row's expansion; on the fold
    /// line open or close the fold under the sub-cursor (Triage is refused). It
    /// never launches anything and never moves the cursor.
    fn toggle_key(&self, ctx: &mut AppContext) {
        let rows = stop_rows(ctx);
        let Some(at) = cursor_pos(ctx, &rows) else {
            return;
        };
        if let Some(row) = rows.get(at) {
            let key = row.key();
            if !ctx.inbox_view.expanded.remove(&key) {
                ctx.inbox_view.expanded.insert(key);
            }
            self.follow(ctx, &rows, at);
        } else {
            match sub_cursor(ctx) {
                Some(Fold::Todos) => Self::say(
                    ctx,
                    "Todos are only counted here: see gsd-meta-manager inbox --include todos"
                        .to_owned(),
                ),
                Some(fold) => {
                    let was_open = ctx.inbox_view.open_folds.remove(&fold);
                    if !was_open {
                        ctx.inbox_view.open_folds.insert(fold);
                    }
                }
                None => {}
            }
        }
        ctx.needs_redraw = true;
    }

    /// `Left` / `Right` on the fold line: move the sub-cursor between the
    /// folds; on a group row they are swallowed.
    fn fold_step(&self, ctx: &mut AppContext, forward: bool) {
        let rows = stop_rows(ctx);
        let Some(at) = cursor_pos(ctx, &rows) else {
            return;
        };
        if at < rows.len() {
            return;
        }
        let folds = fleet_folds(ctx);
        let here = sub_cursor(ctx).and_then(|fold| folds.iter().position(|f| *f == fold));
        let to = match (here, forward) {
            (Some(i), true) => (i + 1).min(folds.len().saturating_sub(1)),
            (Some(i), false) => i.saturating_sub(1),
            (None, _) => 0,
        };
        ctx.inbox_view.fold_cursor = folds.get(to).copied();
        ctx.needs_redraw = true;
    }

    /// `o` (D-08): the group's source file in the read-only viewer. No stop, no
    /// message; the fold line says what to select.
    fn open_key(&self, ctx: &mut AppContext) -> ScreenAction {
        let row = match Self::selected(ctx) {
            Err(()) => return ScreenAction::None,
            Ok(None) => {
                Self::say(ctx, "Select a group row to open its file".to_owned());
                return ScreenAction::None;
            }
            Ok(Some(row)) => row,
        };
        match owner_actions::open_target(ctx, &row.alias, &row.src.files) {
            Ok(action) => {
                ctx.needs_redraw = true;
                action
            }
            Err(message) => {
                Self::say(ctx, message);
                ScreenAction::None
            }
        }
    }

    /// `A` (D-14, D-16): the confirm popup for the group's file, or the status
    /// line that says why not. Nothing is written until Yes.
    fn ack_key(&self, ctx: &mut AppContext) -> ScreenAction {
        let row = match Self::selected(ctx) {
            Err(()) => return ScreenAction::None,
            Ok(None) => {
                Self::say(ctx, "Select a group row to acknowledge it".to_owned());
                return ScreenAction::None;
            }
            Ok(Some(row)) => row,
        };
        // A same-phase merged row stands for several files (D-08): acknowledging
        // is file-level, so it never picks one for the owner (T-38-09).
        if row.src.parts.len() > 1 {
            let n = row.src.parts.len();
            Self::say(
                ctx,
                format!(
                    "{n} files behind this row: press g to pick a file in the Owner block, then A"
                ),
            );
            return ScreenAction::None;
        }
        // A BLOCKED aggregate has no single group: its row's own group stands in
        // and the request path refuses it by lane.
        let group = row_group(ctx, &row);
        match owner_actions::ack_open(ctx, &row.alias, &group, AckHost::Inbox) {
            Ok(action) => {
                ctx.needs_redraw = true;
                action
            }
            Err(message) => {
                Self::say(ctx, message);
                ScreenAction::None
            }
        }
    }

    /// The 37 copy path (D-09): the checklist of the groups behind `row`, looked up
    /// by identity as the key is pressed; a BLOCKED aggregate covers every group of
    /// its (workstream, phase).
    fn copy_groups(ctx: &mut AppContext, row: &InboxRow) -> Result<ScreenAction, String> {
        let mut groups = ctx
            .owner_digest(&row.alias)
            .map(|digest| owner_block::groups_for_copy(digest, &row.src.key))
            .unwrap_or_default();
        if groups.is_empty() {
            groups.push(row.src.group.clone());
        }
        owner_actions::copy_text(&row.alias, &groups)
            .map(|(text, lines)| owner_actions::copy(ctx, text, lines))
    }

    /// `y` (D-09): copy the selected group's checklist, exactly the lines
    /// `gsd-meta-manager inbox` prints for it. On the fold line Old debt copies its
    /// checklist and the other folds say why not; an empty inbox ignores the key.
    fn copy_key(&self, ctx: &mut AppContext) -> ScreenAction {
        let outcome = match Self::selected(ctx) {
            Err(()) => return ScreenAction::None,
            Ok(None) => owner_actions::fold_copy(ctx, sub_cursor(ctx)),
            Ok(Some(row))
                if matches!(row.fold, None | Some(Fold::Stale))
                    && row.src.group.lane != Lane::Blocked =>
            {
                // The entry is looked up by identity as the key is pressed and
                // copies the lines `inbox ALIAS` prints for it (38, D-08).
                let fold_days = ctx.config.preferences.owner_fold_days();
                let entry = ctx
                    .owner_digest(&row.alias)
                    .and_then(|digest| owner_block::entry_for(digest, &row.src.key, fold_days));
                match entry {
                    Some(entry) => owner_actions::copy_entry_text(&row.alias, &entry)
                        .map(|(text, lines)| owner_actions::copy(ctx, text, lines)),
                    None => Self::copy_groups(ctx, &row),
                }
            }
            Ok(Some(row)) => Self::copy_groups(ctx, &row),
        };
        ctx.needs_redraw = true;
        match outcome {
            Ok(action) => action,
            Err(message) => {
                Self::say(ctx, message);
                ScreenAction::None
            }
        }
    }

    /// `g` (D-07): the project's detail view on the Phases tab with the Owner
    /// block on the same group. The inbox pushes the screen itself: the App's
    /// project-open helper refuses once more than the dashboard is on the stack.
    /// The dashboard table is re-selected to the project when it is listed there,
    /// so the dashboard below shows the same project when the owner comes back
    /// all the way (RESEARCH Pitfall 6).
    fn goto_key(&self, ctx: &mut AppContext) -> ScreenAction {
        let row = match Self::selected(ctx) {
            Err(()) => return ScreenAction::None,
            Ok(None) => {
                Self::say(ctx, "Select a group row to open its project".to_owned());
                return ScreenAction::None;
            }
            Ok(Some(row)) => row,
        };
        if let Some(index) = ctx.filtered_aliases.iter().position(|a| *a == row.alias) {
            ctx.table_state.select(Some(index));
        }
        ctx.needs_redraw = true;
        // A merged row opens the Owner block on its first file's group (D-08).
        let key = row
            .src
            .parts
            .first()
            .map_or_else(|| row.src.key.clone(), GroupKey::of);
        ScreenAction::Push(Box::new(super::detail::DetailScreen::opened_on_owner(
            row.alias.clone(),
            &key,
            ctx,
        )))
    }

    /// `v` (D-11, D-12): run the selected group's command through `launch`. No
    /// stop, no message, no action; on the fold line the status says what to do.
    pub(crate) fn run_key(
        &self,
        ctx: &mut AppContext,
        launch: &mut dyn FnMut(&owner_actions::RunPlan) -> String,
    ) -> ScreenAction {
        let row = match Self::selected(ctx) {
            Err(()) => return ScreenAction::None,
            Ok(None) => {
                Self::say(ctx, "Select a group row to run its command".to_owned());
                return ScreenAction::None;
            }
            Ok(Some(row)) => row,
        };
        // The group is looked up by its identity as the key is pressed; a BLOCKED
        // aggregate has no single group, so its row's own group stands in and the
        // run path refuses it by lane.
        let group = row_group(ctx, &row);
        let message = owner_actions::run_group(ctx, &row.alias, &group, launch);
        Self::say(ctx, message);
        ScreenAction::None
    }
}

impl Default for InboxScreen {
    fn default() -> Self {
        Self::new()
    }
}

crate::ui::screens::adjudicate_screen!(
    InboxScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws every registered alias (`shown`), each group's phase name, session \
     name, file-status phrase, workstream name and tally from the owner digests \
     read out of `.planning/` (`row_cells`, which escapes them through `shown`), \
     the header numbers, and the status line (`shown`, because it composes \
     authored copy with a project's phase token or workstream in one string). \
     Fixture states: the inbox over a hostile digest, and with a status message.",
);

impl Screen for InboxScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        // The side-effecting letters act only unchorded (WR-01): Ctrl-V is the
        // paste reflex and arrives as `Char('v')` with CONTROL.
        let plain = owner_actions::is_plain_action_key(modifiers);
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(super::help::HelpScreen::at_section(
                    "Owner Inbox",
                    ctx.experimental,
                )))
            }
            KeyCode::Char('v') if plain => self.run_key(ctx, &mut owner_actions::launch_real),
            KeyCode::Char('o') if plain => self.open_key(ctx),
            KeyCode::Char('A') if plain => self.ack_key(ctx),
            KeyCode::Char('y') if plain => self.copy_key(ctx),
            KeyCode::Char('g') if plain => self.goto_key(ctx),
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.toggle_key(ctx);
                ScreenAction::None
            }
            KeyCode::Left => {
                self.fold_step(ctx, false);
                ScreenAction::None
            }
            KeyCode::Right => {
                self.fold_step(ctx, true);
                ScreenAction::None
            }
            KeyCode::Char(key) if LaneFilter::of_key(key).is_some() => {
                self.filter_key(ctx, key);
                ScreenAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.step(ctx, 1);
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.step(ctx, -1);
                ScreenAction::None
            }
            KeyCode::Home => {
                self.step(ctx, isize::MIN / 2);
                ScreenAction::None
            }
            KeyCode::End => {
                self.step(ctx, isize::MAX / 2);
                ScreenAction::None
            }
            KeyCode::PageDown => {
                let page =
                    isize::try_from(self.viewport.get().saturating_sub(1).max(1)).unwrap_or(1);
                self.step(ctx, page);
                ScreenAction::None
            }
            KeyCode::PageUp => {
                let page =
                    isize::try_from(self.viewport.get().saturating_sub(1).max(1)).unwrap_or(1);
                self.step(ctx, -page);
                ScreenAction::None
            }
            // Everything else is swallowed: no key reaches the dashboard below.
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        self.width.set(width);
        let spacer = u16::from(area.height >= 7);
        // The spacer goes first on a short screen, then the fold line (UI-SPEC).
        let fold_height = u16::from(area.height >= 6 && !fold_totals(ctx).is_empty());
        let [header, _, body, rule, fold, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(spacer),
            Constraint::Min(0),
            Constraint::Length(u16::from(area.height >= 3)),
            Constraint::Length(fold_height),
            Constraint::Length(1),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(header_line(ctx, width, ctx.inbox_view.filter.token())),
            header,
        );

        let rows = stop_rows(ctx);
        let cursor = cursor_pos(ctx, &rows);
        let has_stops = cursor.is_some();
        let on_fold = cursor.is_some_and(|at| at >= rows.len());
        if rows.is_empty() {
            self.viewport.set(usize::from(body.height));
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    fit_cells(&empty_copy(ctx), width),
                    dim(),
                ))),
                body,
            );
            let pinned = Pinned {
                rule,
                fold,
                footer,
                has_stops,
                on_fold,
                hint: None,
            };
            self.render_pinned(frame, ctx, pinned, width);
            return;
        }
        let mut every: Vec<Cells> = inbox_rows(ctx)
            .iter()
            .chain(rows.iter().filter(|row| row.fold.is_some()))
            .filter_map(|row| cells_of(ctx, row))
            .collect();
        every.shrink_to_fit();
        let grid = grid_for_inbox(&every, width);
        let plan = full_plan(ctx, &rows, width);
        let height = usize::from(body.height);
        self.viewport.set(height);
        let offset = match cursor.filter(|at| *at < rows.len()) {
            Some(at) => scroll_for(ctx.inbox_view.offset, &plan, at, height),
            None => ctx
                .inbox_view
                .offset
                .min(plan.len().saturating_sub(height.max(1))),
        };
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(height);
        for (line_index, item) in plan.iter().enumerate().skip(offset).take(height) {
            lines.push(match item {
                BodyItem::Title(section) => {
                    title_line(*section, (line_index == 0).then_some(&grid), width)
                }
                BodyItem::Sub(label) => owner_block::dim_line(2, label, width),
                BodyItem::Extra(line) => line.clone(),
                BodyItem::Row(index) => {
                    let row = &rows[*index];
                    match cells_of(ctx, row) {
                        Some(c) => {
                            let expanded = ctx.inbox_view.expanded.contains(&row.key());
                            let mut line =
                                row_line(&c, &grid, width, cursor == Some(*index), expanded);
                            if row.fold == Some(Fold::Acknowledged) {
                                for span in &mut line.spans {
                                    span.style = span.style.add_modifier(Modifier::DIM);
                                }
                            }
                            line
                        }
                        None => Line::from(""),
                    }
                }
            });
        }
        frame.render_widget(Paragraph::new(lines), body);
        let hint = (height > 0 && plan.len() > height).then(|| {
            format!(
                " {}-{} of {}",
                offset + 1,
                (offset + height).min(plan.len()),
                plan.len()
            )
        });
        let pinned = Pinned {
            rule,
            fold,
            footer,
            has_stops,
            on_fold,
            hint,
        };
        self.render_pinned(frame, ctx, pinned, width);
    }

    fn name(&self) -> &str {
        "inbox"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplexer::{
        Detected, Detection, EnvSnapshot, ExecArgv, LaunchPlan, MultiplexerBackend, MuxError,
    };
    use crate::state_reader::owner_items::{
        Category, GroupCounts, Kind, OwnerDigest, OwnerGroup, OwnerItem, RunCommand, RunVerb, Scope,
    };
    use crate::ui::screens::normal::NormalScreen;
    use crate::ui::screens::tests::{ctx_with_aliases, owe};
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    fn item(name: &str) -> OwnerItem {
        OwnerItem {
            id: name.to_owned(),
            test: None,
            name: name.to_owned(),
            expected: None,
            result: "pending".to_owned(),
            category: Category::HumanUat,
            reason: None,
            blocked_by: None,
            gaps: Vec::new(),
            why_human: None,
        }
    }

    struct G {
        lane: Lane,
        kind: Kind,
        phase: &'static str,
        name: &'static str,
        checks: usize,
        closes: bool,
        age: Option<i64>,
        stale: bool,
    }

    fn group(g: G) -> OwnerGroup {
        let run = match (g.lane, g.kind) {
            (Lane::Test, _) => Some(RunCommand {
                verb: RunVerb::VerifyWork,
                phase: Some(g.phase.to_owned()),
                workstream: None,
            }),
            (Lane::Fix, Kind::Verification) => Some(RunCommand {
                verb: RunVerb::PlanPhaseGaps,
                phase: Some(g.phase.to_owned()),
                workstream: None,
            }),
            (_, Kind::Debug) => Some(RunCommand {
                verb: RunVerb::Debug,
                phase: None,
                workstream: None,
            }),
            (_, Kind::Handoff) => Some(RunCommand {
                verb: RunVerb::ResumeWork,
                phase: None,
                workstream: None,
            }),
            _ => None,
        };
        OwnerGroup {
            lane: g.lane,
            kind: g.kind,
            scope: Scope::Current,
            workstream: None,
            phase: g.phase.to_owned(),
            phase_name: g.name.to_owned(),
            file: format!("phases/{0}/{0}-UAT.md", g.phase),
            file_status: "partial".to_owned(),
            closes_phase: g.closes,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: g.age,
            stale: g.stale,
            command: run.as_ref().map(ToString::to_string),
            run,
            supersedes: Vec::new(),
            counts: GroupCounts {
                checks: g.checks,
                ..GroupCounts::default()
            },
            items: (0..g.checks).map(|i| item(&format!("check {i}"))).collect(),
        }
    }

    fn digest(groups: Vec<OwnerGroup>) -> OwnerDigest {
        let count = |lane: Lane| {
            groups
                .iter()
                .filter(|g| g.lane == lane)
                .map(|g| g.items.len().max(1))
                .sum::<usize>()
        };
        OwnerDigest {
            test: count(Lane::Test),
            decide: groups.iter().filter(|g| g.lane == Lane::Decide).count(),
            fix_phases: groups.iter().filter(|g| g.lane == Lane::Fix).count(),
            blocked: count(Lane::Blocked),
            groups,
            ..OwnerDigest::default()
        }
    }

    fn test_group(
        phase: &'static str,
        name: &'static str,
        checks: usize,
        closes: bool,
        age: i64,
        stale: bool,
    ) -> OwnerGroup {
        group(G {
            lane: Lane::Test,
            kind: Kind::Uat,
            phase,
            name,
            checks,
            closes,
            age: Some(age),
            stale,
        })
    }

    /// The UI-SPEC "Fixture fleet": nine synthetic projects.
    pub(crate) fn inbox_fleet_ctx() -> AppContext {
        let aliases = [
            "alder-api",
            "birch-cli",
            "cedar-web",
            "dune-app",
            "elm-shop",
            "fir-sync",
            "holly-pics",
            "iris-gate",
            "lime-bug",
        ];
        let mut ctx = ctx_with_aliases(&aliases);
        // The fleet's oldest rows are 203 and 182 days old: a fold at 365 days keeps
        // them in their lanes, so the 37 snapshots keep their meaning. The fold has
        // its own context, `inbox_triage_ctx`, at the default 90.
        ctx.config
            .preferences
            .extra
            .insert("owner".to_owned(), serde_json::json!({"fold_days": 365}));
        let progress = |ctx: &mut AppContext, alias: &str, done: u32, total: u32| {
            let state = ctx.project_states.get_mut(alias).expect("registered");
            state.completed_phases = done;
            state.total_phases = total;
        };
        progress(&mut ctx, "dune-app", 24, 25);
        progress(&mut ctx, "alder-api", 9, 10);
        progress(&mut ctx, "birch-cli", 13, 18);
        owe(
            &mut ctx,
            "dune-app",
            digest(vec![test_group(
                "24",
                "widget-clock-faces",
                6,
                true,
                2,
                false,
            )]),
        );
        owe(
            &mut ctx,
            "alder-api",
            digest(vec![test_group("10", "cases-replay", 3, true, 10, false)]),
        );
        owe(
            &mut ctx,
            "birch-cli",
            digest(vec![test_group(
                "18",
                "visual-parity-gate",
                3,
                true,
                203,
                true,
            )]),
        );
        let mut cedar_fix = group(G {
            lane: Lane::Fix,
            kind: Kind::Verification,
            phase: "05",
            name: "login-flow",
            checks: 2,
            closes: false,
            age: Some(9),
            stale: false,
        });
        cedar_fix.counts.failed = 2;
        cedar_fix.counts.gaps = 1;
        let mut cedar_decide = group(G {
            lane: Lane::Decide,
            kind: Kind::ContextQuestions,
            phase: "07",
            name: "onboarding-copy",
            checks: 0,
            closes: false,
            age: Some(11),
            stale: false,
        });
        cedar_decide.file = "phases/07/07-CONTEXT.md".to_owned();
        cedar_decide.counts.checks = 1;
        let mut cedar = digest(vec![
            test_group("05", "login-flow", 1, false, 11, false),
            cedar_fix,
            cedar_decide,
        ]);
        cedar.hidden.todos = 7;
        cedar.hidden.todos_oldest_days = Some(96);
        owe(&mut ctx, "cedar-web", cedar);
        let mut elm_fix = group(G {
            lane: Lane::Fix,
            kind: Kind::Verification,
            phase: "16",
            name: "checkout-totals",
            checks: 3,
            closes: false,
            age: Some(18),
            stale: false,
        });
        elm_fix.counts.failed = 3;
        elm_fix.counts.gaps = 2;
        let mut elm = digest(vec![elm_fix]);
        elm.hidden.acknowledged_files = 8;
        owe(&mut ctx, "elm-shop", elm);
        let mut handoff = group(G {
            lane: Lane::Decide,
            kind: Kind::Handoff,
            phase: "",
            name: "release-prep",
            checks: 0,
            closes: false,
            age: Some(182),
            stale: true,
        });
        handoff.file_status = "paused".to_owned();
        handoff.items = vec![OwnerItem {
            result: "human_action".to_owned(),
            ..item("sign the form")
        }];
        owe(&mut ctx, "fir-sync", digest(vec![handoff]));
        let mut holly = OwnerDigest::default();
        holly.archived.test = 3;
        owe(&mut ctx, "holly-pics", holly);
        let mut blocked = group(G {
            lane: Lane::Blocked,
            kind: Kind::Uat,
            phase: "23",
            name: "store-listing",
            checks: 0,
            closes: false,
            age: None,
            stale: false,
        });
        blocked.items = vec![item("a"), item("b"), item("c")];
        owe(&mut ctx, "iris-gate", digest(vec![blocked]));
        let mut debug = group(G {
            lane: Lane::Decide,
            kind: Kind::Debug,
            phase: "",
            name: "tap-no-open-app",
            checks: 0,
            closes: false,
            age: Some(21),
            stale: false,
        });
        debug.file_status = "awaiting_human_verify".to_owned();
        let mut lime = digest(vec![debug]);
        lime.hidden.todos = 4;
        owe(&mut ctx, "lime-bug", lime);
        ctx
    }

    /// A real-shaped synthetic fleet: one long alias, many workstream rows with
    /// long names, decimal phases and long titles. No real project text.
    pub(crate) fn realistic_fleet_ctx() -> AppContext {
        let aliases = [
            "alpha-lab",
            "beta-platform-research-lab",
            "gamma-app",
            "delta-svc",
            "epsilon-ui",
        ];
        let mut ctx = ctx_with_aliases(&aliases);
        for (i, alias) in aliases.iter().enumerate() {
            let mut groups = Vec::new();
            for n in 0..6 {
                let mut g = test_group(
                    "42",
                    "long-descriptive-phase-title-number-one",
                    3,
                    true,
                    4,
                    false,
                );
                g.workstream = Some(format!("v0.{} P4{}", 10 + i, n));
                if n % 3 == 2 {
                    g.phase = "42.1".to_owned();
                    g.file = "phases/42.1/42.1-UAT.md".to_owned();
                }
                g.file = format!("{}-{n}", g.file);
                groups.push(g);
            }
            if i == 1 {
                let mut fix = group(G {
                    lane: Lane::Fix,
                    kind: Kind::Verification,
                    phase: "7",
                    name: "another-rather-long-fix-title",
                    checks: 2,
                    closes: false,
                    age: Some(9),
                    stale: false,
                });
                fix.counts.failed = 2;
                groups.push(fix);
            }
            owe(&mut ctx, alias, digest(groups));
        }
        ctx
    }

    /// Phase numbers of every workstream row stay readable at 100 and 80 columns.
    #[test]
    fn phase_identity_survives_on_a_realistic_fleet() {
        let ctx = realistic_fleet_ctx();
        for (w, h) in [(100u16, 30u16), (80, 24)] {
            let text = render_inbox_text(&ctx, w, h);
            let rule = text
                .iter()
                .position(|l| l.starts_with('\u{2500}'))
                .expect("rule");
            let rows = group_rows(&text);
            assert!(rows.len() >= 6, "{w}: {}", text.join("\n"));
            for y in rows {
                let l = &text[y];
                assert!(y < rule);
                assert!(
                    l.contains(" ph 42") || l.contains(" ph 7"),
                    "{w}x{h} row lost its phase number: {l:?}"
                );
            }
        }
    }

    /// The identities of the rows as `alias phase` in order, for the ordering tests.
    fn order(rows: &[InboxRow]) -> Vec<String> {
        rows.iter()
            .map(|r| format!("{} {}", r.alias, r.src.group.phase))
            .collect()
    }

    #[test]
    fn sort_groups_follows_d05() {
        let ctx = inbox_fleet_ctx();
        let mut rows = inbox_rows(&ctx);
        sort_groups(&mut rows);
        assert_eq!(
            order(&rows),
            [
                "dune-app 24",
                "alder-api 10",
                "birch-cli 18",
                "cedar-web 05",
                "fir-sync ",
                "lime-bug ",
                "cedar-web 07",
                "elm-shop 16",
                "cedar-web 05",
                "iris-gate 23",
            ]
        );
        let sections: Vec<Section> = rows.iter().map(|r| r.section).collect();
        assert_eq!(
            sections,
            [
                Section::TestCloses,
                Section::TestCloses,
                Section::TestCloses,
                Section::TestOpen,
                Section::Decide,
                Section::Decide,
                Section::Decide,
                Section::Fix,
                Section::Fix,
                Section::Blocked,
            ]
        );
        // Every permutation sorts to the same sequence (a total order).
        let want = order(&rows);
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        for _ in 0..200 {
            let mut shuffled = rows.clone();
            for i in (1..shuffled.len()).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                shuffled.swap(i, (seed % (i as u64 + 1)) as usize);
            }
            sort_groups(&mut shuffled);
            assert_eq!(order(&shuffled), want);
        }
    }

    #[test]
    fn sort_groups_breaks_every_tie_the_same_way() {
        let ctx = inbox_fleet_ctx();
        let base = inbox_rows(&ctx)
            .into_iter()
            .find(|r| r.alias == "dune-app")
            .expect("dune-app row");
        // Two rows equal in progress, open phases and age: alias decides, then
        // phase key, then kind and file.
        let mut a = base.clone();
        a.alias = "zeta".to_owned();
        let mut b = base.clone();
        b.alias = "alpha".to_owned();
        let mut c = base.clone();
        c.alias = "alpha".to_owned();
        c.src.group.phase = "9".to_owned();
        let mut d = base.clone();
        d.alias = "alpha".to_owned();
        d.src.group.phase = "10".to_owned();
        let mut e = base.clone();
        e.alias = "alpha".to_owned();
        e.src.group.phase = "10".to_owned();
        e.src.key.file = "a.md".to_owned();
        let mut rows = vec![a, b, c, d, e];
        sort_groups(&mut rows);
        let files: Vec<(String, String, String)> = rows
            .iter()
            .map(|r| {
                (
                    r.alias.clone(),
                    r.src.group.phase.clone(),
                    r.src.key.file.clone(),
                )
            })
            .collect();
        // Alias first; then the numeric phase key (9 before 10 before 24); then
        // the file.
        let want = [
            ("alpha", "9", "phases/24/24-UAT.md"),
            ("alpha", "10", "a.md"),
            ("alpha", "10", "phases/24/24-UAT.md"),
            ("alpha", "24", "phases/24/24-UAT.md"),
            ("zeta", "24", "phases/24/24-UAT.md"),
        ];
        let got: Vec<(&str, &str, &str)> = files
            .iter()
            .map(|(a, p, f)| (a.as_str(), p.as_str(), f.as_str()))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn rows_without_an_age_sort_after_rows_with_one() {
        let ctx = inbox_fleet_ctx();
        let base = inbox_rows(&ctx)
            .into_iter()
            .find(|r| r.alias == "lime-bug")
            .expect("a DECIDE row");
        let mut none = base.clone();
        none.alias = "aaa".to_owned();
        none.src.group.age_days = None;
        let mut rows = vec![none, base];
        sort_groups(&mut rows);
        assert_eq!(rows[0].alias, "lime-bug");
        assert_eq!(rows[1].alias, "aaa");
    }

    // --- rendering --------------------------------------------------------

    fn draw(ctx: &AppContext, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
        let screen = InboxScreen::new();
        terminal
            .draw(|frame| {
                let area = frame.area();
                Screen::render(&screen, frame, area, ctx);
            })
            .expect("draw the inbox");
        terminal.backend().buffer().clone()
    }

    /// Every terminal row as plain text, one char per cell.
    pub(crate) fn render_inbox_text(ctx: &AppContext, width: u16, height: u16) -> Vec<String> {
        let buffer = draw(ctx, width, height);
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer.cell((x, y)).expect("cell").symbol().to_owned())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn the_inbox_renders_sections_in_order_at_100x30() {
        let ctx = inbox_fleet_ctx();
        let text = render_inbox_text(&ctx, 100, 30);
        let line = |needle: &str| {
            text.iter()
                .position(|l| l.contains(needle))
                .unwrap_or_else(|| panic!("no {needle:?} in:\n{}", text.join("\n")))
        };
        assert!(
            text[0].starts_with(" Owner inbox \u{2500}\u{2500} test 13"),
            "{:?}",
            text[0]
        );
        assert!(text[0].contains("test "), "{:?}", text[0]);
        // At 100 columns the lane keys take their short rung; at 140 the long one.
        assert!(
            text[0].contains("[1]T [2]D [3]F [4]B [a]ll"),
            "{:?}",
            text[0]
        );
        let wide = render_inbox_text(&ctx, 140, 30);
        assert!(wide[0].contains("[1]Test [2]Decide [3]Fix [4]Blocked [a]ll"));
        assert!(wide[0].contains("9 projects"), "{:?}", wide[0]);
        assert!(text[1].trim().is_empty(), "the spacer row");
        let closes = line("TEST \u{2014} closes a phase");
        let open = line("TEST \u{2014} phase stays open");
        let decide = line("DECIDE");
        let fix = line("FIX \u{2014} code needed");
        assert!(closes < open && open < decide && decide < fix);
        assert!(!text.join("\n").contains("BLOCKED"), "BLOCKED is hidden");
        // The first row starts with `>` and carries the marker.
        assert!(
            text[closes + 1].starts_with(">\u{25b8} dune-app"),
            "{:?}",
            text[closes + 1]
        );
        assert!(text[closes + 2].starts_with(" \u{25b8} alder-api"));
        // The footer lists only the keys that work.
        assert!(text[29].contains("[j/k]move"), "{:?}", text[29]);
        // At 100 columns the long rung (107 cells) does not fit, so the short one
        // shows; `[y]copy` joins in 37-07.
        assert!(text[29].contains("[v]run"));
        assert!(text[29].contains("[Enter]expand"));
        assert!(text[29].contains("[g]oto"));
        assert!(text[29].contains("[o]pen"));
        assert!(text[29].contains("[A]cknowledge"));
        // The first row is Cyan + BOLD, every cell of it.
        let buffer = draw(&ctx, 100, 30);
        let y = (closes + 1) as u16;
        for x in 0..20u16 {
            let cell = buffer.cell((x, y)).unwrap();
            assert_eq!(cell.fg, Color::Cyan, "x={x}");
            assert!(cell.modifier.contains(Modifier::BOLD), "x={x}");
        }
        let other = buffer.cell((4, y + 1)).unwrap();
        assert_ne!(other.fg, Color::Cyan);
    }

    #[test]
    fn the_header_sums_every_registered_project() {
        let ctx = inbox_fleet_ctx();
        let left = header_left(&ctx, true, true);
        assert!(left.contains("9 projects"), "{left}");
        assert!(left.contains("test 13"), "{left}");
        assert!(left.contains("decide 3"), "{left}");
        assert!(left.contains("fix 2 phases"), "{left}");
        assert!(left.contains("blocked 3"), "{left}");
    }

    #[test]
    fn the_inbox_never_panics_at_tiny_sizes() {
        let ctx = inbox_fleet_ctx();
        for (w, h) in [
            (80, 24),
            (79, 24),
            (40, 10),
            (20, 5),
            (1, 1),
            (0, 0),
            (3, 2),
        ] {
            let text = render_inbox_text(&ctx, w, h);
            for row in &text {
                assert!(
                    ratatui::text::Line::from(row.as_str()).width() <= usize::from(w),
                    "{row:?}"
                );
            }
        }
    }

    // --- loading, unreadable and the live cursor (37-04) --------------------

    /// The x of `needle` in terminal row `y` of a rendered buffer.
    fn x_of(text: &[String], y: usize, needle: &str) -> usize {
        let at = text[y]
            .find(needle)
            .unwrap_or_else(|| panic!("no {needle:?} in {:?}", text[y]));
        text[y][..at].chars().count()
    }

    #[test]
    fn a_failed_digest_reads_unreadable_not_loading() {
        let mut ctx = inbox_fleet_ctx();
        // One project is still loading, one will never load.
        ctx.owner_digests.remove("holly-pics");
        ctx.owner_digests.remove("iris-gate");
        ctx.owner_digest_failed.insert("iris-gate".to_owned());
        assert_eq!(digest_state(&ctx, "holly-pics"), DigestState::Loading);
        assert_eq!(digest_state(&ctx, "iris-gate"), DigestState::Unreadable);
        assert_eq!(digest_state(&ctx, "dune-app"), DigestState::Loaded);
        let text = render_inbox_text(&ctx, 160, 30);
        assert!(
            text[0].contains("blocked 0 \u{b7} 1 loading \u{b7} 1 unreadable"),
            "{:?}",
            text[0]
        );
        let buffer = draw(&ctx, 160, 30);
        let x = x_of(&text, 0, "1 loading");
        let y = x_of(&text, 0, "1 unreadable");
        for at in [x, x + 4, y, y + 8] {
            let cell = buffer.cell((at as u16, 0)).expect("cell");
            assert!(cell.modifier.contains(Modifier::DIM), "x={at}");
        }
        // The failed project is not counted as loading, and the numbers stay
        // plain: a project with no digest is never counted as clean.
        assert!(!text[0].contains("2 loading"));
        // The unreadable clause is the first thing a narrow header drops.
        let narrow = render_inbox_text(&ctx, 100, 30);
        assert!(narrow[0].contains("1 loading"), "{:?}", narrow[0]);
        assert!(!narrow[0].contains("unreadable"), "{:?}", narrow[0]);
        let tight = render_inbox_text(&ctx, 90, 30);
        assert!(!tight[0].contains("loading"), "{:?}", tight[0]);
        // A project whose `.planning/` is gone is unreadable without a mark.
        let mut gone = inbox_fleet_ctx();
        gone.project_states.get_mut("dune-app").unwrap().presence =
            crate::state_reader::ProjectPresence::NoPlanning;
        assert_eq!(digest_state(&gone, "dune-app"), DigestState::Unreadable);
        let text = render_inbox_text(&gone, 160, 30);
        assert!(text[0].contains("1 unreadable"), "{:?}", text[0]);
    }

    #[test]
    fn no_project_could_be_read() {
        let mut ctx = ctx_with_aliases(&["alder-api", "birch-cli"]);
        ctx.owner_digest_failed.insert("alder-api".to_owned());
        ctx.owner_digest_failed.insert("birch-cli".to_owned());
        let text = render_inbox_text(&ctx, 100, 30);
        assert!(text[0].contains("2 projects"), "{:?}", text[0]);
        assert!(text[0].contains("2 unreadable"), "{:?}", text[0]);
        assert!(!text[0].contains("loading"), "{:?}", text[0]);
        assert_eq!(text[2].trim(), "No project could be read (2 unreadable)");
        let buffer = draw(&ctx, 100, 30);
        assert!(buffer
            .cell((3, 2))
            .expect("cell")
            .modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn every_digest_missing_reads_loading() {
        let ctx = ctx_with_aliases(&["alder-api", "birch-cli"]);
        let text = render_inbox_text(&ctx, 100, 30);
        assert!(text[0].starts_with(" Owner inbox \u{2500}\u{2500} loading"));
        assert!(
            !text[0].contains("test "),
            "no numbers while nothing loaded"
        );
        assert_eq!(text[2].trim(), "Loading...");
        // One failed, one still loading: still `Loading...`, the failure is named.
        let mut ctx = ctx;
        ctx.owner_digest_failed.insert("birch-cli".to_owned());
        let text = render_inbox_text(&ctx, 100, 30);
        assert!(text[0].contains("1 unreadable"), "{:?}", text[0]);
        assert_eq!(text[2].trim(), "Loading...");
    }

    /// Select the row of `alias` in `ctx`, as a key press would.
    fn select(ctx: &mut AppContext, alias: &str) {
        let rows = visible_rows(ctx);
        let at = rows
            .iter()
            .position(|r| r.alias == alias)
            .expect("a row for the alias");
        ctx.inbox_view.cursor = Some(InboxStop::Row(rows[at].key()));
        ctx.inbox_view.cursor_index = at;
    }

    /// The alias under the cursor, read through the production `cursor_pos`
    /// (the fold line is `"<fold line>"`, no stop at all is `None`).
    fn cursor_alias(ctx: &AppContext) -> Option<String> {
        let rows = stop_rows(ctx);
        cursor_pos(ctx, &rows).map(|at| {
            rows.get(at)
                .map_or_else(|| "<fold line>".to_owned(), |row| row.alias.clone())
        })
    }

    #[test]
    fn selection_falls_to_the_next_row_then_previous_then_none() {
        let empty = |ctx: &mut AppContext, alias: &str| {
            ctx.owner_digests
                .insert(alias.to_owned(), OwnerDigest::default());
            reanchor(ctx);
        };
        // The middle row vanishes: the next one takes the cursor.
        let mut ctx = inbox_fleet_ctx();
        select(&mut ctx, "alder-api");
        empty(&mut ctx, "alder-api");
        assert_eq!(cursor_alias(&ctx).as_deref(), Some("birch-cli"));
        // It is a real identity now: a later refresh that reorders keeps it.
        assert_eq!(
            ctx.inbox_view.cursor,
            Some(InboxStop::Row(visible_rows(&ctx)[1].key()))
        );
        // The last row vanishes: the previous one.
        let mut ctx = inbox_fleet_ctx();
        let rows = visible_rows(&ctx);
        let last = rows.last().expect("rows");
        ctx.inbox_view.cursor = Some(InboxStop::Row(last.key()));
        ctx.inbox_view.cursor_index = rows.len() - 1;
        let alias = last.alias.clone();
        empty(&mut ctx, &alias);
        let rows = visible_rows(&ctx);
        assert_eq!(
            cursor_alias(&ctx).as_deref(),
            Some(rows.last().expect("rows").alias.as_str())
        );
        // Every row vanishes: no cursor, no stop, no panic.
        let mut ctx = inbox_fleet_ctx();
        select(&mut ctx, "dune-app");
        for alias in ctx.config.projects.keys().cloned().collect::<Vec<_>>() {
            empty(&mut ctx, &alias);
        }
        assert_eq!(ctx.inbox_view.cursor, None);
        assert_eq!(cursor_alias(&ctx), None);
        let text = render_inbox_text(&ctx, 100, 30);
        assert_eq!(text[2].trim(), "No owner work");
    }

    #[test]
    fn a_refresh_for_one_project_keeps_the_cursor_on_its_group() {
        let mut ctx = inbox_fleet_ctx();
        select(&mut ctx, "birch-cli");
        // Project B (alder-api) gets one more check and a new age: its rows
        // change, A's group keeps the cursor, wherever it now sits.
        let mut changed = ctx.owner_digests["alder-api"].clone();
        changed.groups[0].counts.checks = 9;
        changed.groups[0].age_days = Some(400);
        ctx.owner_digests.insert("alder-api".to_owned(), changed);
        reanchor(&mut ctx);
        assert_eq!(cursor_alias(&ctx).as_deref(), Some("birch-cli"));
        // Updates in any order, an empty stop list and a 0x0 area never panic.
        for alias in ["dune-app", "alder-api", "birch-cli", "cedar-web"] {
            ctx.owner_digests
                .insert(alias.to_owned(), OwnerDigest::default());
            reanchor(&mut ctx);
            let _ = render_inbox_text(&ctx, 100, 30);
            let _ = render_inbox_text(&ctx, 0, 0);
        }
        // fir-sync, lime-bug and elm-shop still hold rows; the cursor sits on one.
        assert!(cursor_alias(&ctx).is_some());
    }

    // --- the layout contract (37-04, D-02, D-03, D-04, D-22) --------------------

    fn every_cells(ctx: &AppContext) -> Vec<Cells> {
        inbox_rows(ctx)
            .iter()
            .filter_map(|row| cells_of(ctx, row))
            .collect()
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// `ctx` with `keys` pressed on a fresh inbox screen.
    fn press(ctx: &mut AppContext, keys: &str) {
        let mut screen = InboxScreen::new();
        for key in keys.chars() {
            screen.handle_key(KeyCode::Char(key), KeyModifiers::NONE, ctx);
        }
    }

    /// The body rows of a render: the group rows, which carry the `>`/space
    /// cursor column and the marker.
    fn group_rows(text: &[String]) -> Vec<usize> {
        let rule = text
            .iter()
            .position(|l| l.starts_with('\u{2500}'))
            .unwrap_or(text.len());
        (0..rule)
            .filter(|&y| text[y].chars().nth(1) == Some('\u{25b8}'))
            .collect()
    }

    #[test]
    fn grid_for_inbox_matches_the_ui_spec_table() {
        let ctx = inbox_fleet_ctx();
        let every = every_cells(&ctx);
        let grid = |w| grid_for_inbox(&every, w);
        // Natural widths: 3 + 9 + 2 + 7 + 2 + 18 + 2 + 9 + 2 + 32 + 2 + 11 = 99.
        let natural = grid(99);
        assert_eq!(
            (natural.alias_w, natural.phase_w, natural.name_w),
            (9, 7, 18)
        );
        assert_eq!((natural.phrase_w, natural.age_w), (32, 11));
        assert_eq!(natural.total(), 99);
        // The Command column (natural 25) shows from 126 up, not before.
        let wide = grid(140);
        assert_eq!(wide.command_w, 25);
        assert_eq!((wide.phrase_w, wide.name_w, wide.age_w), (32, 18, 11));
        assert_eq!(grid(126).command_w, 25);
        assert_eq!(grid(125).command_w, 0, "99 + 2 + 25 = 126 > 125");
        for w in [120, 100] {
            let g = grid(w);
            assert_eq!(g.command_w, 0, "W {w}");
            assert_eq!((g.phrase_w, g.name_w, g.age_w), (32, 18, 11), "W {w}");
        }
        // 80: the phrase gives way first, the name stays whole.
        let g80 = grid(80);
        assert_eq!((g80.phrase_w, g80.name_w, g80.age_w), (13, 18, 11));
        assert_eq!(g80.total(), 80);
        // 79: Age is gone, so the phrase has room again (UI-SPEC: "shrinks as
        // needed"); the plan's "12" at 79 is not what the arithmetic gives.
        let g79 = grid(79);
        assert_eq!((g79.age_w, g79.name_w), (0, 18));
        assert_eq!(g79.phrase_w, 25);
        assert_eq!(g79.total(), 79);
        // Narrower: phrase to 12, alias and name, the phase cell only down to its
        // phase text (fix 37-UI-REVIEW #1), then 5 as the last resort; always
        // within W, the name floored at 8.
        let g60 = grid(60);
        assert_eq!(g60.phrase_w, 12);
        assert!(g60.total() <= 60 && g60.name_w >= 8 && g60.name_w < 18);
        assert_eq!((g60.alias_w, g60.phase_w), (8, 7));
        let g20 = grid(20);
        assert_eq!(
            (g20.phrase_w, g20.phase_w, g20.alias_w, g20.name_w),
            (12, 5, 8, 8)
        );
        // No FIX row, no Command column, whatever the width.
        let mut no_fix = inbox_fleet_ctx();
        no_fix.owner_digests.remove("elm-shop");
        no_fix.owner_digests.remove("cedar-web");
        assert_eq!(grid_for_inbox(&every_cells(&no_fix), 200).command_w, 0);
    }

    #[test]
    fn layout_tiers_140_120_100_80() {
        let ctx = inbox_fleet_ctx();
        let every = every_cells(&ctx);
        for (w, h) in [(140u16, 40u16), (120, 40), (100, 30), (80, 24)] {
            let text = render_inbox_text(&ctx, w, h);
            let width = usize::from(w);
            let (header, fold, footer) = (
                &text[0],
                &text[usize::from(h) - 2],
                &text[usize::from(h) - 1],
            );
            // Header rung per width: 116 / 103 / 87 / 75 plus the leading space.
            match w {
                140 | 120 => {
                    assert!(header.contains("9 projects \u{b7} test 13"), "{header:?}");
                    assert!(header.contains("[1]Test [2]Decide [3]Fix [4]Blocked [a]ll"));
                }
                100 => {
                    assert!(!header.contains("projects"), "{header:?}");
                    assert!(header.contains("blocked 3"));
                    assert!(header.contains("[1]T [2]D [3]F [4]B [a]ll"));
                }
                _ => {
                    assert!(!header.contains("blocked"), "{header:?}");
                    assert!(header.contains("[1]T [2]D [3]F [4]B [a]ll"));
                    assert!(header.contains("fix 2 phases"));
                }
            }
            // Fold rung.
            match w {
                140 => assert!(
                    fold.contains(
                        "\u{25b8} Old debt: 3 checks in 1 project (archived milestones)   \
                         \u{25b8} Acknowledged (8 files)   \
                         \u{25b8} Triage: 11 pending todos in 2 projects"
                    ),
                    "{fold:?}"
                ),
                120 | 100 => {
                    assert!(
                        fold.contains(
                            "\u{25b8} Old debt: 3 checks in 1 project   \
                             \u{25b8} Acknowledged (8 files)   \
                             \u{25b8} Triage: 11 todos in 2 projects"
                        ),
                        "{fold:?}"
                    );
                    assert!(!fold.contains("archived milestones"));
                }
                _ => assert!(
                    fold.contains(
                        "\u{25b8} Old debt (3)   \u{25b8} Acknowledged (8)   \u{25b8} Triage 11"
                    ),
                    "{fold:?}"
                ),
            }
            assert!(footer.contains("[j/k]move"), "{footer:?}");
            // Command column: FIX rows at 140 only.
            let commands = text
                .iter()
                .filter(|l| l.contains("/gsd-plan-phase"))
                .count();
            assert_eq!(commands, if w == 140 { 2 } else { 0 }, "W {w}");
            if w == 140 {
                assert!(text.iter().any(|l| l.contains("/gsd-plan-phase 16 --gaps")));
                assert!(text.iter().any(|l| l.contains("/gsd-plan-phase 05 --gaps")));
            }
            // Age at all four widths, and every column starts where the grid says.
            let grid = grid_for_inbox(&every, width);
            assert!(grid.age_w > 0);
            assert!(text.iter().any(|l| l.contains("203 d stale")), "W {w}");
            let rows = group_rows(&text);
            assert_eq!(rows.len(), 9, "W {w}: nine group rows");
            for y in rows {
                let chars: Vec<char> = text[y].chars().collect();
                let at = |x: usize, n: usize| -> String {
                    chars.iter().skip(x).take(n).collect::<String>()
                };
                // Checks right-aligned: the cell ends with `check(s)` or is blank.
                let checks = at(grid.checks_x(), CHECKS_W);
                assert!(
                    checks.trim().is_empty()
                        || checks.trim_end().ends_with("checks")
                        || checks.trim_end().ends_with("check"),
                    "W {w} row {y}: {checks:?}"
                );
                assert_eq!(checks.chars().count(), CHECKS_W.min(checks.chars().count()));
                // Age starts with its digits at the grid's x.
                let age = at(grid.age_x(), 3);
                let digits = age.chars().next().is_some_and(|c| c.is_ascii_digit());
                assert!(digits || age.trim().is_empty(), "W {w} row {y}: {age:?}");
            }
        }
        // The footer ladder: the first rung that fits (the whole strings are
        // asserted by `footer_ladder_matches_the_ui_spec`).
        let ctx = inbox_fleet_ctx();
        let footer = |w: usize| line_text(&footer_line(&ctx, w, true, false));
        assert!(footer(126).ends_with("[y]copy checklist  [Esc]back  [?]help"));
        assert!(footer(125).ends_with("[y]copy  [Esc]back  [?]help"));
        assert!(footer(93).ends_with("[A]ck  [y]copy  [?]help"));
        assert_eq!(footer(65), "  [v]run  [A]ck  [y]copy  [?]help");
        assert_eq!(
            footer(10),
            "  [v]run \u{2026}",
            "the minimal rung is cut to W"
        );
        // The cursor on the fold line has its own hints (UI-SPEC "Footer ladder").
        let fold = |w: usize| line_text(&footer_line(&ctx, w, true, true));
        assert_eq!(
            fold(80),
            "  [\u{2190}/\u{2192}]fold  [Enter]expand  [y]copy  [j/k]move  [Esc]back  [?]help"
        );
        assert_eq!(
            fold(60),
            "  [\u{2190}/\u{2192}]fold  [Enter]expand  [y]copy  [j/k]move  [?]help"
        );
        assert_eq!(
            line_text(&footer_line(&ctx, 80, false, false)),
            "  [Esc]back  [?]help",
            "no stops, no movement or run key"
        );
    }

    /// UI-SPEC "Footer ladder": the whole rung strings, at 140, 100 and 80 columns.
    #[test]
    fn footer_ladder_matches_the_ui_spec() {
        let ctx = inbox_fleet_ctx();
        let footer =
            |w: usize, stops: bool, fold: bool| line_text(&footer_line(&ctx, w, stops, fold));
        let rung1 = "  [j/k]move  [Enter]expand  [g]o to project  [v]run command  [o]pen file  [A]cknowledge  [y]copy checklist  [Esc]back  [?]help";
        let rung2 = "  [j/k]move  [Enter]expand  [g]oto  [v]run  [o]pen  [A]cknowledge  [y]copy  [Esc]back  [?]help";
        let rung3 = "  [j/k]move  [Enter]expand  [g]oto  [v]run  [o]pen  [A]ck  [y]copy  [?]help";
        assert_eq!(rung1.chars().count(), 126);
        assert_eq!(rung2.chars().count(), 94);
        assert_eq!(rung3.chars().count(), 75);
        assert_eq!(footer(140, true, false), rung1);
        assert_eq!(footer(100, true, false), rung2);
        assert_eq!(footer(80, true, false), rung3);
        assert_eq!(footer(126, true, false), rung1);
        assert_eq!(footer(125, true, false), rung2);
        assert_eq!(footer(94, true, false), rung2);
        assert_eq!(footer(93, true, false), rung3);
        assert_eq!(footer(75, true, false), rung3);
        assert_eq!(footer(74, true, false), "  [v]run  [A]ck  [y]copy  [?]help");
        assert_eq!(
            footer(140, true, true),
            "  [\u{2190}/\u{2192}]fold  [Enter]expand  [y]copy  [j/k]move  [Esc]back  [?]help"
        );
        assert_eq!(footer(140, false, false), "  [Esc]back  [?]help");
    }

    fn wayland_probes() -> crate::clipboard::ClipboardProbes {
        crate::clipboard::ClipboardProbes {
            os: "linux",
            wayland_display: true,
            display: true,
            on_path: vec!["wl-copy"],
        }
    }

    fn press_y(ctx: &mut AppContext) -> ScreenAction {
        InboxScreen::new().handle_key(KeyCode::Char('y'), KeyModifiers::NONE, ctx)
    }

    fn status(ctx: &AppContext) -> String {
        ctx.status_message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default()
    }

    /// The first row is dune-app's TEST group of six checks: seven lines.
    fn dune_checklist() -> (String, usize) {
        let ctx = inbox_fleet_ctx();
        let digest = ctx.owner_digest("dune-app").expect("digest");
        owner_actions::copy_text("dune-app", &digest.groups).expect("text")
    }

    #[test]
    fn y_copies_or_falls_back_to_osc52() {
        // With a native tool: nothing is claimed yet, the runner gets (tool, text) once.
        let mut ctx = inbox_fleet_ctx();
        let (text, lines) = dune_checklist();
        assert_eq!(lines, 7);
        owner_actions::force_clipboard(Some(wayland_probes()));
        let action = press_y(&mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            owner_actions::seam_ran(),
            [(crate::clipboard::Tool::WlCopy, text.clone(), 7)]
        );
        assert_eq!(status(&ctx), "", "no copy claim before the tool answered");

        // Without one: OSC 52, and the status says only what is true.
        owner_actions::force_clipboard(None);
        let mut ctx = inbox_fleet_ctx();
        let ScreenAction::WriteOsc52(sequence) = press_y(&mut ctx) else {
            panic!("no tool means OSC 52");
        };
        assert_eq!(sequence, crate::clipboard::osc52(&text));
        assert_eq!(
            status(&ctx),
            "Sent 7 lines to the terminal clipboard (OSC 52; the terminal must allow it)"
        );
        assert!(owner_actions::seam_ran().is_empty());
        owner_actions::force_clipboard(None);
    }

    #[test]
    fn y_is_ignored_on_an_empty_inbox() {
        let mut ctx = ctx_with_aliases(&["a"]);
        owner_actions::force_clipboard(Some(wayland_probes()));
        assert!(matches!(press_y(&mut ctx), ScreenAction::None));
        assert!(owner_actions::seam_ran().is_empty());
        assert_eq!(status(&ctx), "");
        owner_actions::force_clipboard(None);
    }

    #[test]
    fn y_on_the_folds() {
        let mut ctx = inbox_fleet_ctx();
        let mut old = test_group("03", "old-checks", 2, false, 400, false);
        old.scope = Scope::Archived {
            label: "v0.9".to_owned(),
            rank: 0,
        };
        let mut holly = digest(vec![old.clone()]);
        holly.archived.test = 2;
        owe(&mut ctx, "holly-pics", holly);
        owner_actions::force_clipboard(Some(wayland_probes()));
        let mut screen = InboxScreen::new();
        let key = |screen: &mut InboxScreen, ctx: &mut AppContext, code| {
            screen.handle_key(code, KeyModifiers::NONE, ctx)
        };
        key(&mut screen, &mut ctx, KeyCode::End);
        // Old debt first: the OLD DEBT lines the CLI prints.
        key(&mut screen, &mut ctx, KeyCode::Char('y'));
        let mut want = String::from("OLD DEBT\n");
        for line in crate::inbox::group_checklist("holly-pics", &old) {
            want.push_str(&line);
            want.push('\n');
        }
        let ran = owner_actions::seam_ran();
        assert_eq!(ran.len(), 1);
        assert_eq!(ran[0].1, want);
        assert_eq!(ran[0].2, 4, "heading, group line, two checks");
        // Acknowledged, then Triage: refusals, nothing more copied.
        key(&mut screen, &mut ctx, KeyCode::Right);
        key(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert_eq!(
            status(&ctx),
            "Nothing to copy: acknowledged items are already handled"
        );
        key(&mut screen, &mut ctx, KeyCode::Right);
        key(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert_eq!(status(&ctx), "Nothing to copy: todos are only counted here");
        assert_eq!(owner_actions::seam_ran().len(), 1);
        owner_actions::force_clipboard(None);
    }

    #[test]
    fn sc2_legible_at_100x30() {
        let ctx = inbox_fleet_ctx();
        let text = render_inbox_text(&ctx, 100, 30);
        // The body: from the title row to the rule row.
        let rule = text
            .iter()
            .position(|l| l.starts_with('\u{2500}'))
            .expect("the rule row");
        for (y, line) in text.iter().enumerate().take(rule).skip(2) {
            assert!(!line.contains('\u{2026}'), "row {y} is cut: {line:?}");
        }
        let every = every_cells(&ctx);
        let grid = grid_for_inbox(&every, 100);
        let rows = group_rows(&text);
        assert_eq!(rows.len(), 9);
        // Every group row has its columns at the same x.
        for y in &rows {
            let chars: Vec<char> = text[*y].chars().collect();
            assert_eq!(chars[3], text[*y].chars().nth(3).unwrap());
            assert!(chars.len() >= grid.checks_x());
            assert_eq!(chars[grid.checks_x() - 1], ' ', "row {y}");
            assert_eq!(chars[grid.checks_x() - 2], ' ', "row {y}");
        }
        // Titles in D-02 order, with the DIM heads on the first title.
        let first_title = text
            .iter()
            .position(|l| l.contains("TEST \u{2014} closes a phase"))
            .expect("the first title");
        assert!(text[first_title].contains("checks"));
        assert!(text[first_title].contains("age"));
        let checks_at = x_of(&text, first_title, "checks");
        assert_eq!(checks_at, grid.checks_x() + 3, "right-aligned over Checks");
        let age_at = text[first_title]
            .rfind("age")
            .map(|i| text[first_title][..i].chars().count())
            .expect("the age head");
        assert_eq!(age_at, grid.age_x());
        let buffer = draw(&ctx, 100, 30);
        for x in [checks_at, checks_at + 5, age_at, age_at + 2] {
            let cell = buffer.cell((x as u16, first_title as u16)).expect("cell");
            assert!(cell.modifier.contains(Modifier::DIM), "x={x}");
        }
        // The second title carries no heads.
        let second = text
            .iter()
            .position(|l| l.contains("phase stays open"))
            .expect("the second title");
        assert!(!text[second].contains("checks") && !text[second].contains("age"));
    }

    #[test]
    fn degrade_never_panics() {
        let ctx = inbox_fleet_ctx();
        for (w, h) in [
            (80u16, 24u16),
            (79, 24),
            (60, 20),
            (40, 10),
            (20, 5),
            (1, 1),
            (0, 0),
            (3, 2),
            (100, 6),
            (100, 7),
            (10, 40),
        ] {
            let text = render_inbox_text(&ctx, w, h);
            assert_eq!(text.len(), usize::from(h));
        }
        // The lines themselves, before the buffer clips them: never wider than W,
        // and no Age below 80 columns.
        let every = every_cells(&ctx);
        let rows = visible_rows(&ctx);
        for w in 0..=150usize {
            let grid = grid_for_inbox(&every, w);
            if w < AGE_MIN_W {
                assert_eq!(grid.age_w, 0, "W {w}");
            }
            assert!(
                grid.total() <= w
                    || (grid.phrase_w, grid.phase_w, grid.alias_w, grid.name_w) == (12, 5, 8, 8),
                "W {w}: {grid:?}"
            );
            assert!(header_line(&ctx, w, None).width() <= w, "header W {w}");
            assert!(
                footer_line(&ctx, w, true, false).width() <= w,
                "footer W {w}: {:?}",
                footer_line(&ctx, w, true, false)
            );
            if let Some(fold) = fold_line(&ctx, w) {
                assert!(fold.width() <= w, "fold W {w}");
            }
            for row in &rows {
                let c = cells_of(&ctx, row).expect("cells");
                for cursor in [false, true] {
                    assert!(
                        row_line(&c, &grid, w, cursor, false).width() <= w,
                        "row W {w}"
                    );
                }
            }
            for section in [Section::TestCloses, Section::Blocked] {
                assert!(title_line(section, Some(&grid), w).width() <= w);
            }
        }
        // The name is the last column to be cut: at 60 the phrase is at its floor
        // before the name loses a cell.
        let g = grid_for_inbox(&every, 60);
        assert_eq!(g.phrase_w, 12);
        // 80x24: phrases cut to 13 cells, names whole.
        let text = render_inbox_text(&ctx, 80, 24);
        assert!(text.iter().any(|l| l.contains("paused \u{b7} 1 h\u{2026}")));
        assert!(text.iter().any(|l| l.contains("visual-parity-gate")));
        assert!(text.iter().any(|l| l.contains("widget-clock-faces")));
    }

    #[test]
    fn filters_show_their_sections_and_highlight_one_key() {
        let titles = [
            "TEST \u{2014} closes a phase",
            "TEST \u{2014} phase stays open",
            "DECIDE",
            "FIX \u{2014} code needed",
            "BLOCKED",
        ];
        // (keys pressed, sections shown, the REVERSED token)
        let states: [(&str, [bool; 5], &str); 6] = [
            ("", [true, true, true, true, false], ""),
            ("1", [true, true, false, false, false], "[1]Test"),
            ("2", [false, false, true, false, false], "[2]Decide"),
            ("3", [false, false, false, true, false], "[3]Fix"),
            ("4", [false, false, false, false, true], "[4]Blocked"),
            ("a", [true, true, true, true, true], "[a]ll"),
        ];
        let pos = |text: &[String], needle: &str| {
            text.iter()
                .skip(2)
                .find_map(|l| l.find(needle).map(|i| l[..i].chars().count()))
        };
        let probes = [
            "widget-clock-faces",
            "6 checks",
            "tap-no-open-app",
            "awaiting your",
            "checkout-totals",
            "3 failed checks",
            "182 d stale",
            "20",
        ];
        let reference = {
            let mut ctx = inbox_fleet_ctx();
            press(&mut ctx, "a");
            render_inbox_text(&ctx, 140, 40)
        };
        let left_of = |text: &[String]| {
            let end = text[0].find("[1]").expect("the lane keys");
            text[0][..end].trim_end().to_owned()
        };
        let numbers = left_of(&reference);
        for (keys, shown, token) in states {
            let mut ctx = inbox_fleet_ctx();
            press(&mut ctx, keys);
            let text = render_inbox_text(&ctx, 140, 40);
            for (title, want) in titles.iter().zip(shown) {
                let got = text.iter().skip(2).any(|l| l.contains(title));
                assert_eq!(got, want, "keys {keys:?} title {title}");
            }
            // Header numbers and column starts never move with the filter.
            assert_eq!(left_of(&text), numbers, "keys {keys:?}");
            for probe in probes {
                if let (Some(a), Some(b)) = (pos(&text, probe), pos(&reference, probe)) {
                    assert_eq!(a, b, "keys {keys:?} probe {probe:?}");
                }
            }
            // Exactly one REVERSED token, and none for the default.
            let buffer = draw(&ctx, 140, 40);
            let reversed: String = (0..140u16)
                .filter_map(|x| buffer.cell((x, 0)))
                .filter(|c| c.modifier.contains(Modifier::REVERSED))
                .map(|c| c.symbol().to_owned())
                .collect();
            assert_eq!(reversed, token, "keys {keys:?}");
            for y in 2..40u16 {
                for x in 0..140u16 {
                    assert!(
                        !buffer
                            .cell((x, y))
                            .expect("cell")
                            .modifier
                            .contains(Modifier::REVERSED),
                        "REVERSED only in the header"
                    );
                }
            }
        }
        // BLOCKED: one DIM row per project and phase, the tally from the name column.
        let mut ctx = inbox_fleet_ctx();
        // A second blocked project, sorted first, takes the cursor: the iris-gate
        // row is then a plain DIM row.
        let mut second = ctx.owner_digests["iris-gate"].clone();
        second.groups[0].phase = "09".to_owned();
        ctx.owner_digests.insert("dune-app".to_owned(), second);
        press(&mut ctx, "4");
        let text = render_inbox_text(&ctx, 140, 40);
        let blocked = text
            .iter()
            .position(|l| l.contains("iris-gate"))
            .expect("the blocked row");
        assert!(text[blocked].contains("ph 23"), "{:?}", text[blocked]);
        let buffer = draw(&ctx, 140, 40);
        let y = blocked as u16;
        assert!(buffer
            .cell((6, y))
            .unwrap()
            .modifier
            .contains(Modifier::DIM));
        let title = text
            .iter()
            .position(|l| l.contains("BLOCKED"))
            .expect("the BLOCKED title") as u16;
        assert!(buffer
            .cell((3, title))
            .unwrap()
            .modifier
            .contains(Modifier::DIM));
        // The same key twice returns to the default.
        let mut ctx = inbox_fleet_ctx();
        press(&mut ctx, "22");
        assert_eq!(ctx.inbox_view.filter, LaneFilter::Default);
        press(&mut ctx, "3a");
        assert_eq!(ctx.inbox_view.filter, LaneFilter::All);
        // A lane key is a filter, not a run: nothing is launched or messaged.
        assert!(ctx.status_message.is_none());
        // The cursor row survives a filter that still lists it, and falls to a
        // neighbor otherwise.
        let mut ctx = inbox_fleet_ctx();
        select(&mut ctx, "elm-shop");
        press(&mut ctx, "3");
        assert_eq!(cursor_alias(&ctx).as_deref(), Some("elm-shop"));
        press(&mut ctx, "2");
        assert!(cursor_alias(&ctx).is_some());
    }

    #[test]
    fn empty_and_loading_states() {
        let dimmed = |ctx: &AppContext, y: u16, x: u16| {
            draw(ctx, 100, 30)
                .cell((x, y))
                .expect("cell")
                .modifier
                .contains(Modifier::DIM)
        };
        // No registered project.
        let ctx = ctx_with_aliases(&[]);
        let text = render_inbox_text(&ctx, 100, 30);
        assert!(text[0].starts_with(" Owner inbox \u{2500}\u{2500} 0 projects"));
        assert_eq!(
            text[2].trim(),
            "No projects registered. Press Esc, then a to add one."
        );
        assert!(dimmed(&ctx, 2, 3));
        assert_eq!(text[29].trim(), "[Esc]back  [?]help");

        // Partial: some digests missing; the rest renders and is counted.
        let mut partial = inbox_fleet_ctx();
        partial.owner_digests.remove("lime-bug");
        let text = render_inbox_text(&partial, 140, 30);
        assert!(text[0].contains("\u{b7} 1 loading"), "{:?}", text[0]);
        assert!(text.iter().any(|l| l.contains("dune-app")));
        assert!(text[0].contains("decide 2"), "absent data is not counted");

        // Loaded and clean: `No owner work`, and the folds are still drawn.
        let mut clean = ctx_with_aliases(&["alder-api"]);
        let mut digest = OwnerDigest::default();
        digest.hidden.todos = 1;
        owe(&mut clean, "alder-api", digest);
        let text = render_inbox_text(&clean, 140, 30);
        assert!(text[0].contains("1 project \u{b7} test 0"), "{:?}", text[0]);
        assert_eq!(text[2].trim(), "No owner work");
        assert!(dimmed(&clean, 2, 3));
        assert!(
            text[28].contains("\u{25b8} Triage: 1 pending todo in 1 project"),
            "{:?}",
            text[28]
        );
        // An empty lane says so, and which key brings the default back.
        for (keys, copy) in [
            (
                "1",
                "No test work \u{b7} press 1 again for the default lanes",
            ),
            (
                "2",
                "No decide work \u{b7} press 2 again for the default lanes",
            ),
            (
                "3",
                "No fix work \u{b7} press 3 again for the default lanes",
            ),
            (
                "4",
                "No blocked work \u{b7} press 4 again for the default lanes",
            ),
            ("a", "No owner work in any lane"),
        ] {
            let mut ctx = ctx_with_aliases(&["alder-api"]);
            owe(&mut ctx, "alder-api", OwnerDigest::default());
            press(&mut ctx, keys);
            let text = render_inbox_text(&ctx, 100, 30);
            assert_eq!(text[2].trim(), copy, "keys {keys:?}");
            assert!(dimmed(&ctx, 2, 3));
        }
        // A lane that is empty while another is not.
        let mut ctx = inbox_fleet_ctx();
        ctx.owner_digests.remove("iris-gate");
        press(&mut ctx, "4");
        let text = render_inbox_text(&ctx, 100, 30);
        assert_eq!(
            text[2].trim(),
            "No blocked work \u{b7} press 4 again for the default lanes"
        );
        // Absent data is never counted as clean: a project with no digest at all.
        let loading = ctx_with_aliases(&["alder-api", "birch-cli"]);
        let text = render_inbox_text(&loading, 100, 30);
        assert!(!text.iter().any(|l| l.contains("No owner work")));
    }

    #[test]
    fn quiet_color_at_100x30() {
        // The fixture fleet plus one workstream group, so the Magenta prefix exists.
        let fleet = || {
            let mut ctx = inbox_fleet_ctx();
            let mut ws = test_group("30", "ws-gate", 2, true, 5, false);
            ws.workstream = Some("w1".to_owned());
            let mut dune = ctx.owner_digests["dune-app"].clone();
            dune.groups.push(ws);
            ctx.owner_digests.insert("dune-app".to_owned(), dune);
            ctx
        };
        for cursor_on in ["dune-app", "birch-cli", "ws"] {
            let mut ctx = fleet();
            let rows = visible_rows(&ctx);
            let at = if cursor_on == "ws" {
                rows.iter()
                    .position(|r| r.src.group.workstream.is_some())
                    .expect("the workstream row")
            } else {
                rows.iter()
                    .position(|r| r.alias == cursor_on && r.src.group.workstream.is_none())
                    .expect("the row")
            };
            ctx.inbox_view.cursor = Some(InboxStop::Row(rows[at].key()));
            ctx.inbox_view.cursor_index = at;
            let text = render_inbox_text(&ctx, 100, 30);
            let buffer = draw(&ctx, 100, 30);
            let cursor_y = group_rows(&text)
                .into_iter()
                .find(|&y| text[y].starts_with('>'))
                .expect("the cursor row");
            let (mut cyan, mut yellow, mut magenta) = (Vec::new(), Vec::new(), Vec::new());
            for y in 0..30u16 {
                for x in 0..100u16 {
                    let cell = buffer.cell((x, y)).expect("cell");
                    match cell.fg {
                        Color::Cyan => cyan.push((x, y)),
                        Color::Yellow => yellow.push((x, y)),
                        Color::Magenta => magenta.push((x, y)),
                        Color::Reset => {}
                        other => panic!("({x},{y}) has the color {other:?}"),
                    }
                }
            }
            // Cyan: only on the cursor row, on every visible character of it.
            assert!(cyan.iter().all(|&(_, y)| usize::from(y) == cursor_y));
            let row: Vec<char> = text[cursor_y].chars().collect();
            for (x, ch) in row.iter().enumerate() {
                if *ch != ' ' {
                    assert!(cyan.contains(&(x as u16, cursor_y as u16)), "x={x} {ch}");
                }
            }
            // Yellow: exactly the `stale` words of the other rows.
            let mut want_yellow = Vec::new();
            let mut want_magenta = Vec::new();
            for (y, line) in text.iter().enumerate() {
                if y == cursor_y || y < 2 {
                    continue;
                }
                if let Some(i) = line.find("stale") {
                    let x = line[..i].chars().count();
                    want_yellow.extend((x..x + 5).map(|x| (x as u16, y as u16)));
                }
                if group_rows(&text).contains(&y) {
                    if let Some(i) = line.find("ws w1 ") {
                        let x = line[..i].chars().count();
                        want_magenta.extend((x..x + 6).map(|x| (x as u16, y as u16)));
                    }
                }
            }
            assert_eq!(yellow, want_yellow, "cursor on {cursor_on}");
            assert_eq!(magenta, want_magenta, "cursor on {cursor_on}");
            assert!(!yellow.is_empty(), "a stale word is on screen");
            assert!(
                cursor_on == "ws" || !magenta.is_empty(),
                "the ws prefix is on screen"
            );
        }
    }

    #[test]
    fn the_fold_line_ladder() {
        let ctx = inbox_fleet_ctx();
        let at = |ctx: &AppContext, w: usize| fold_line(ctx, w).map(|l| line_text(&l));
        let rung1 = " \u{25b8} Old debt: 3 checks in 1 project (archived milestones)   \
                     \u{25b8} Acknowledged (8 files)   \
                     \u{25b8} Triage: 11 pending todos in 2 projects";
        let rung2 = " \u{25b8} Old debt: 3 checks in 1 project   \
                     \u{25b8} Acknowledged (8 files)   \
                     \u{25b8} Triage: 11 todos in 2 projects";
        let rung3 = " \u{25b8} Old debt (3)   \u{25b8} Acknowledged (8)   \u{25b8} Triage 11";
        assert_eq!(cells(rung1), 126);
        assert_eq!(cells(rung2), 96);
        assert_eq!(cells(rung3), 50);
        assert_eq!(at(&ctx, 126).as_deref(), Some(rung1));
        assert_eq!(at(&ctx, 125).as_deref(), Some(rung2));
        assert_eq!(at(&ctx, 96).as_deref(), Some(rung2));
        assert_eq!(at(&ctx, 95).as_deref(), Some(rung3));
        assert_eq!(at(&ctx, 50).as_deref(), Some(rung3));
        assert_eq!(
            at(&ctx, 20).as_deref().map(cells),
            Some(20),
            "the last rung is cut to W"
        );
        // The Triage fold is DIM, the others are not.
        let line = fold_line(&ctx, 140).expect("a fold line");
        let styles: Vec<bool> = line
            .spans
            .iter()
            .filter(|s| s.content.contains('\u{25b8}'))
            .map(|s| s.style.add_modifier.contains(Modifier::DIM))
            .collect();
        assert_eq!(styles, [false, false, true]);

        // A fold with no content is omitted; none at all, no line.
        let mut some = inbox_fleet_ctx();
        some.owner_digests
            .get_mut("elm-shop")
            .unwrap()
            .hidden
            .acknowledged_files = 0;
        assert_eq!(
            at(&some, 140).as_deref(),
            Some(
                " \u{25b8} Old debt: 3 checks in 1 project (archived milestones)   \
                 \u{25b8} Triage: 11 pending todos in 2 projects"
            )
        );
        let mut none = ctx_with_aliases(&["alder-api"]);
        owe(&mut none, "alder-api", OwnerDigest::default());
        assert_eq!(at(&none, 140), None);
        let text = render_inbox_text(&none, 100, 30);
        assert!(text[28].starts_with('\u{2500}'), "the rule, no fold line");
        assert!(text[29].contains("[Esc]back"));

        // Archived open questions are not checks: `items`, as the CLI says.
        let mut items = inbox_fleet_ctx();
        items
            .owner_digests
            .get_mut("holly-pics")
            .unwrap()
            .archived
            .decide = 2;
        assert!(at(&items, 140)
            .unwrap()
            .contains("Old debt: 5 items in 1 project (archived milestones)"));
        let mut one = inbox_fleet_ctx();
        one.owner_digests
            .get_mut("holly-pics")
            .unwrap()
            .archived
            .test = 1;
        assert!(at(&one, 140)
            .unwrap()
            .contains("Old debt: 1 check in 1 project"));

        // Unreadable todos: counted at rung 1 only; all unreadable is `?`.
        let mut unreadable = inbox_fleet_ctx();
        unreadable
            .owner_digests
            .get_mut("fir-sync")
            .unwrap()
            .hidden
            .todos_unreadable = true;
        assert!(at(&unreadable, 160)
            .unwrap()
            .contains("11 pending todos in 2 projects (+1 unreadable)"));
        assert!(!at(&unreadable, 126).unwrap().contains("unreadable"));
        let mut all = ctx_with_aliases(&["alder-api"]);
        let mut digest = OwnerDigest::default();
        digest.hidden.todos_unreadable = true;
        owe(&mut all, "alder-api", digest);
        assert_eq!(at(&all, 140).as_deref(), Some(" \u{25b8} Triage: ?"));
        assert_eq!(at(&all, 5).as_deref().map(cells), Some(5));
        // Only one pending todo in one project.
        let mut single = ctx_with_aliases(&["alder-api"]);
        let mut digest = OwnerDigest::default();
        digest.hidden.todos = 1;
        owe(&mut single, "alder-api", digest);
        assert_eq!(
            at(&single, 140).as_deref(),
            Some(" \u{25b8} Triage: 1 pending todo in 1 project")
        );
        assert_eq!(
            at(&single, 36).as_deref(),
            Some(" \u{25b8} Triage: 1 todo in 1 project")
        );
    }

    // --- keys -------------------------------------------------------------

    /// A backend that records the opens it receives.
    struct Recorder {
        opened: Mutex<Vec<(PathBuf, Vec<String>)>>,
    }

    impl MultiplexerBackend for Recorder {
        fn name(&self) -> &'static str {
            "recorder"
        }
        fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
            Some(Detection {
                inside: true,
                reachable: Ok(()),
            })
        }
        fn open(&self, _env: &EnvSnapshot, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
            self.opened
                .lock()
                .unwrap()
                .push((cwd.to_path_buf(), argv.as_slice().to_vec()));
            Ok(())
        }
    }

    fn recorder() -> (&'static Recorder, LaunchPlan) {
        let backend: &'static Recorder = Box::leak(Box::new(Recorder {
            opened: Mutex::new(Vec::new()),
        }));
        let plan = LaunchPlan {
            env: EnvSnapshot::default(),
            backends: vec![Detected {
                backend,
                detection: Detection {
                    inside: true,
                    reachable: Ok(()),
                },
            }],
            gui: None,
        };
        (backend, plan)
    }

    /// `v` through the real multiplexer seam over the recording backend.
    fn launch_via(plan: &LaunchPlan, run: &owner_actions::RunPlan) -> String {
        crate::multiplexer::launch_with(
            plan,
            &run.cwd,
            crate::ui::screens::detail::owner_mux_argv(&run.command),
            &|term| crate::ui::screens::detail::owner_terminal_argv(term, &run.command),
            &mut |_, _, _| Err(std::io::Error::other("no GUI in tests")),
            &format!("Launched {} in {}", run.command, run.alias),
        )
    }

    /// Dashboard `i`, then the keys, then `v`: the screen the `i` arm pushed.
    fn open_inbox_and(ctx: &mut AppContext, keys: &[KeyCode]) -> Box<dyn Screen> {
        let mut normal = NormalScreen::new();
        let ScreenAction::Push(mut screen) =
            normal.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, ctx)
        else {
            panic!("`i` must push the inbox");
        };
        for code in keys {
            screen.handle_key(*code, KeyModifiers::NONE, ctx);
        }
        screen
    }

    /// `v` on the pushed screen's own `run_key`, which is what the key arm calls
    /// with `launch_real`, over the recording backend.
    fn press_v(ctx: &mut AppContext, plan: &LaunchPlan) {
        InboxScreen::new().run_key(ctx, &mut |run| launch_via(plan, run));
    }

    #[test]
    fn i_v_launches_the_top_row() {
        let mut ctx = inbox_fleet_ctx();
        let _screen = open_inbox_and(&mut ctx, &[]);
        assert!(ctx.status_message.is_none(), "`i` shows no message");
        let (backend, plan) = recorder();
        press_v(&mut ctx, &plan);
        let opened = backend.opened.lock().unwrap().clone();
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].0, PathBuf::from("/nonexistent/dune-app"));
        assert_eq!(
            opened[0].1,
            [
                crate::ui::screens::detail::scrubbed_program(),
                vec!["/gsd-verify-work 24".to_owned()]
            ]
            .concat()
        );
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Launched /gsd-verify-work 24 in dune-app")
        );
    }

    #[test]
    fn sc1_i_j_v_launches_verify_work_in_the_project() {
        let mut ctx = inbox_fleet_ctx();
        let _screen = open_inbox_and(&mut ctx, &[KeyCode::Char('j')]);
        let (backend, plan) = recorder();
        press_v(&mut ctx, &plan);
        let opened = backend.opened.lock().unwrap().clone();
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].0, PathBuf::from("/nonexistent/alder-api"));
        assert_eq!(
            opened[0].1,
            [
                crate::ui::screens::detail::scrubbed_program(),
                vec!["/gsd-verify-work 10".to_owned()]
            ]
            .concat()
        );
    }

    #[test]
    fn a_live_session_is_focused_not_launched() {
        let mut ctx = inbox_fleet_ctx();
        let project = ctx.config.projects["dune-app"].path.clone();
        ctx.active_sessions = vec![crate::session_detector::ClaudeSession {
            pid: 4242,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: project.clone(),
            start_time: Some(1),
            tty: None,
        }];
        let _screen = open_inbox_and(&mut ctx, &[]);
        let (backend, _plan) = recorder();
        // The focus branch records what it was asked to do and injects nothing.
        let mut seen: Vec<(String, String)> = Vec::new();
        let screen = InboxScreen::new();
        screen.run_key(&mut ctx, &mut |run| {
            assert!(run.live_session.is_some(), "the live session is carried");
            seen.push((run.alias.clone(), run.command.to_string()));
            owner_actions::focus_status("Switched to tmux pane %3".to_owned(), &run.command)
        });
        assert_eq!(
            seen,
            [("dune-app".to_owned(), "/gsd-verify-work 24".to_owned())]
        );
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Switched to tmux pane %3 \u{b7} type /gsd-verify-work 24")
        );
        assert!(
            backend.opened.lock().unwrap().is_empty(),
            "nothing launched"
        );
    }

    #[test]
    fn the_cursor_follows_its_row_through_a_digest_refresh() {
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[KeyCode::Char('j')]);
        // The cursor is on alder-api. A refresh removes the row above it.
        ctx.owner_digests
            .insert("dune-app".to_owned(), OwnerDigest::default());
        let rows = visible_rows(&ctx);
        let at = cursor_pos(&ctx, &rows).expect("a cursor");
        assert_eq!(rows[at].alias, "alder-api");
        // The row under the cursor vanishes: the selection clamps to a neighbor.
        ctx.owner_digests
            .insert("alder-api".to_owned(), OwnerDigest::default());
        let rows = visible_rows(&ctx);
        let at = cursor_pos(&ctx, &rows).expect("a cursor");
        assert!(at < rows.len(), "a row, not the fold line");
        screen.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
    }

    #[test]
    fn j_k_home_end_and_paging_move_the_cursor() {
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let rows = stop_rows(&ctx);
        let at = |ctx: &AppContext| cursor_pos(ctx, &rows).unwrap();
        assert_eq!(at(&ctx), 0);
        // The fold line is the last stop: `End` and `j` past the last row reach it.
        screen.handle_key(KeyCode::End, KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len(), "End lands on the fold line");
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        screen.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len(), "stays on the fold line");
        screen.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len() - 1);
        screen.handle_key(KeyCode::Home, KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), 0);
        screen.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), 0, "stays on the first row");
    }

    #[test]
    fn v_on_an_empty_inbox_does_nothing() {
        let mut ctx = ctx_with_aliases(&["alder-api"]);
        let screen = InboxScreen::new();
        let mut launched = 0;
        let action = screen.run_key(&mut ctx, &mut |_| {
            launched += 1;
            String::new()
        });
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(launched, 0);
        assert!(ctx.status_message.is_none());
    }

    #[test]
    fn the_scroll_keeps_the_cursor_and_its_section_title_in_view() {
        let rows = visible_rows(&inbox_fleet_ctx());
        let plan = body_plan(&rows);
        // Titles and rows interleave: 4 titles + 9 rows... never panics, always in view.
        for height in 1..plan.len() + 2 {
            for cursor in 0..rows.len() {
                let offset = scroll_for(0, &plan, cursor, height);
                let line = plan
                    .iter()
                    .position(|i| *i == BodyItem::Row(cursor))
                    .unwrap();
                assert!(
                    line >= offset && line < offset + height,
                    "h={height} c={cursor}"
                );
            }
        }
    }

    // --- drill-in: expand, folds, scroll, `o`, `g` (37-05) -------------------

    /// Draw `screen` itself (so it learns the width and the body height, as in the
    /// app, where a frame always precedes a key) and return the rows as text.
    fn render_with(screen: &dyn Screen, ctx: &AppContext, w: u16, h: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("backend");
        terminal
            .draw(|frame| {
                let area = frame.area();
                screen.render(frame, area, ctx);
            })
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buffer.cell((x, y)).expect("cell").symbol().to_owned())
                    .collect::<String>()
            })
            .collect()
    }

    /// Each key after a frame, as the app delivers them.
    fn keys_on(
        screen: &mut dyn Screen,
        ctx: &mut AppContext,
        codes: &[KeyCode],
        (w, h): (u16, u16),
    ) -> Vec<ScreenAction> {
        codes
            .iter()
            .map(|code| {
                let _ = render_with(&*screen, ctx, w, h);
                screen.handle_key(*code, KeyModifiers::NONE, ctx)
            })
            .collect()
    }

    fn line_of<'a>(text: &'a [String], needle: &str) -> (usize, &'a String) {
        text.iter()
            .enumerate()
            .find(|(_, l)| l.contains(needle))
            .unwrap_or_else(|| panic!("no row with {needle:?} in {text:#?}"))
    }

    /// The fleet plus archived and acknowledged groups, so the folds have rows.
    fn fleet_with_folds() -> AppContext {
        let mut ctx = inbox_fleet_ctx();
        let archived = |phase: &'static str, name: &'static str, label: &str, rank: usize| {
            let mut g = test_group(phase, name, 2, false, 40, false);
            g.scope = Scope::Archived {
                label: label.to_owned(),
                rank,
            };
            g.file = format!("milestones/{label}/{phase}-UAT.md");
            g
        };
        let holly = ctx.owner_digests.get_mut("holly-pics").unwrap();
        holly.groups.push(archived("03", "old-gate", "v1.0", 0));
        holly.groups.push(archived("02", "older-gate", "v0.9", 1));
        let mut ack = test_group("08", "signed-off", 1, false, 5, false);
        ack.acknowledged = true;
        ctx.owner_digests
            .get_mut("elm-shop")
            .unwrap()
            .groups
            .push(ack);
        ctx
    }

    fn key_of(ctx: &AppContext, alias: &str, phase: &str) -> InboxKey {
        stop_rows(ctx)
            .iter()
            .find(|row| row.alias == alias && row.src.group.phase == phase)
            .unwrap_or_else(|| panic!("no row {alias} {phase}"))
            .key()
    }

    fn put_cursor_on(ctx: &mut AppContext, alias: &str, phase: &str) {
        let rows = stop_rows(ctx);
        let at = rows
            .iter()
            .position(|row| row.alias == alias && row.src.group.phase == phase)
            .expect("a row");
        ctx.inbox_view.cursor = Some(InboxStop::Row(rows[at].key()));
        ctx.inbox_view.cursor_index = at;
    }

    #[test]
    fn enter_expands_a_group_with_its_run_line() {
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let (at, row) = line_of(&text, "dune-app");
        assert!(row.starts_with(">\u{25be} dune-app"), "{row:?}");
        // Items at column 5, five of six, then `... 1 more`; the file and the
        // command at column 5.
        for i in 0..5 {
            assert_eq!(text[at + 1 + i].trim_end(), format!("     check {i}"));
        }
        assert_eq!(text[at + 6].trim_end(), "     \u{2026} 1 more");
        assert_eq!(text[at + 7].trim_end(), "     file phases/24/24-UAT.md");
        assert_eq!(text[at + 8].trim_end(), "     run /gsd-verify-work 24");
        assert!(
            text[at + 9].starts_with(" \u{25b8} alder-api"),
            "{:?}",
            text[at + 9]
        );
        // Everything under the row but the items is DIM.
        let buffer = draw_screen(&*screen, &ctx, size);
        let dim_at = |y: usize| {
            buffer
                .cell((6, y as u16))
                .unwrap()
                .modifier
                .contains(Modifier::DIM)
        };
        assert!(!dim_at(at + 1), "the item line is not DIM");
        assert!(dim_at(at + 6) && dim_at(at + 7) && dim_at(at + 8));
        // The cursor stays on the row; `j` moves into the next group row.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('j')], size);
        let text = render_with(&*screen, &ctx, size.0, size.1);
        assert!(line_of(&text, "alder-api")
            .1
            .starts_with(">\u{25b8} alder-api"));
        assert!(line_of(&text, "dune-app")
            .1
            .starts_with(" \u{25be} dune-app"));
        // `Enter` and `Space` toggle; collapsing shows nothing under the row.
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Char('k'), KeyCode::Enter],
            size,
        );
        let text = render_with(&*screen, &ctx, size.0, size.1);
        assert!(!text.join("\n").contains("check 0"));
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char(' ')], size);
        assert!(render_with(&*screen, &ctx, size.0, size.1)
            .join("\n")
            .contains("check 0"));
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char(' ')], size);
        assert!(ctx.inbox_view.expanded.is_empty());
    }

    fn draw_screen(
        screen: &dyn Screen,
        ctx: &AppContext,
        (w, h): (u16, u16),
    ) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("backend");
        terminal
            .draw(|frame| {
                let area = frame.area();
                screen.render(frame, area, ctx);
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    #[test]
    fn a_group_without_a_command_has_no_run_line_and_a_blocked_row_lists_its_files() {
        let mut ctx = inbox_fleet_ctx();
        // iris-gate's BLOCKED row over two files; cedar's DECIDE row has no command.
        let mut second = ctx.owner_digests["iris-gate"].groups[0].clone();
        second.file = "phases/23/23-VERIFICATION.md".to_owned();
        second.items = vec![item("d")];
        ctx.owner_digests
            .get_mut("iris-gate")
            .unwrap()
            .groups
            .push(second);
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 40);
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('4')], size);
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        let text = render_with(&*screen, &ctx, size.0, size.1).join("\n");
        assert!(text.contains("file phases/23/23-UAT.md"), "{text}");
        assert!(text.contains("file phases/23/23-VERIFICATION.md"));
        assert!(!text.contains("run /"), "BLOCKED has no command");
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Char('4'), KeyCode::Char('2')],
            size,
        );
        put_cursor_on(&mut ctx, "cedar-web", "07");
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        let text = render_with(&*screen, &ctx, size.0, size.1).join("\n");
        assert!(text.contains("file phases/07/07-CONTEXT.md"), "{text}");
        assert!(!text.contains("run /"), "a DECIDE question has no command");
    }

    #[test]
    fn expansion_survives_a_refresh_and_a_vanished_group_is_dropped() {
        let mut ctx = fleet_with_folds();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 40);
        // Expand dune-app and alder-api, open Old debt from the fold line.
        keys_on(
            &mut *screen,
            &mut ctx,
            &[
                KeyCode::Enter,
                KeyCode::Char('j'),
                KeyCode::Enter,
                KeyCode::End,
                KeyCode::Enter,
            ],
            size,
        );
        let dune = key_of(&ctx, "dune-app", "24");
        let alder = key_of(&ctx, "alder-api", "10");
        assert_eq!(ctx.inbox_view.expanded.len(), 2);
        assert!(ctx.inbox_view.open_folds.contains(&Fold::OldDebt));
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        // dune-app's group vanishes on a refresh: it leaves the set, the rest stays.
        ctx.owner_digests
            .insert("dune-app".to_owned(), OwnerDigest::default());
        reanchor(&mut ctx);
        assert!(!ctx.inbox_view.expanded.contains(&dune));
        assert!(ctx.inbox_view.expanded.contains(&alder));
        assert!(ctx.inbox_view.open_folds.contains(&Fold::OldDebt));
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        let _ = render_with(&*screen, &ctx, size.0, size.1);
        // The same group coming back is not expanded again.
        ctx.owner_digests.insert(
            "dune-app".to_owned(),
            digest(vec![test_group(
                "24",
                "widget-clock-faces",
                6,
                true,
                2,
                false,
            )]),
        );
        reanchor(&mut ctx);
        assert!(!ctx.inbox_view.expanded.contains(&dune));
        // A fold whose content vanishes is closed, and the cursor leaves the fold
        // line only when no fold is left at all.
        ctx.owner_digests
            .insert("holly-pics".to_owned(), OwnerDigest::default());
        reanchor(&mut ctx);
        assert!(!ctx.inbox_view.open_folds.contains(&Fold::OldDebt));
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        for alias in ["cedar-web", "elm-shop", "lime-bug"] {
            ctx.owner_digests
                .insert(alias.to_owned(), OwnerDigest::default());
        }
        reanchor(&mut ctx);
        assert_ne!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        let _ = render_with(&*screen, &ctx, size.0, size.1);
    }

    #[test]
    fn the_fold_line_walks_and_opens_old_debt_and_acknowledged() {
        let mut ctx = fleet_with_folds();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 44);
        // `j` past the last group reaches the fold line, sub-cursor on Old debt.
        let rows = stop_rows(&ctx).len();
        let downs = vec![KeyCode::Char('j'); rows];
        keys_on(&mut *screen, &mut ctx, &downs, size);
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        assert_eq!(sub_cursor(&ctx), Some(Fold::OldDebt));
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let (_, fold) = line_of(&text, "Old debt");
        assert!(fold.starts_with(">\u{25b8} Old debt"), "{fold:?}");
        // `\u{2192}` and `\u{2190}` move the sub-cursor and clamp at the ends.
        let seen: Vec<Option<Fold>> = [
            KeyCode::Right,
            KeyCode::Right,
            KeyCode::Right,
            KeyCode::Left,
            KeyCode::Left,
            KeyCode::Left,
        ]
        .iter()
        .map(|code| {
            keys_on(&mut *screen, &mut ctx, &[*code], size);
            sub_cursor(&ctx)
        })
        .collect();
        assert_eq!(
            seen,
            [
                Some(Fold::Acknowledged),
                Some(Fold::Todos),
                Some(Fold::Todos),
                Some(Fold::Acknowledged),
                Some(Fold::OldDebt),
                Some(Fold::OldDebt),
            ]
        );
        // Enter opens Old debt: archived rows under DIM milestone sub-headers.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let v10 = line_of(&text, "  v1.0").0;
        let v09 = line_of(&text, "  v0.9").0;
        assert!(v10 < v09, "rank 0 before rank 1");
        assert!(text[v10 + 1].contains("holly-pics") && text[v10 + 1].contains("03"));
        assert!(text[v09 + 1].contains("holly-pics") && text[v09 + 1].contains("02"));
        assert!(line_of(&text, "Old debt").1.contains("\u{25be} Old debt"));
        let buffer = draw_screen(&*screen, &ctx, size);
        assert!(buffer
            .cell((2, v10 as u16))
            .unwrap()
            .modifier
            .contains(Modifier::DIM));
        // The fold rows are stops: `k` from the fold line lands on the last.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('k')], size);
        let last = stop_rows(&ctx).last().unwrap().key();
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::Row(last)));
        assert_eq!(last_alias(&ctx), "holly-pics");
        // Expanding a fold row shows its lines; `j` returns to the fold line.
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Enter, KeyCode::Char('j')],
            size,
        );
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        // Enter again closes Old debt.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        assert!(ctx.inbox_view.open_folds.is_empty());
        assert!(!render_with(&*screen, &ctx, size.0, size.1)
            .join("\n")
            .contains("v1.0"));
        // Acknowledged: all rows DIM.
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Right, KeyCode::Enter],
            size,
        );
        assert!(ctx.inbox_view.open_folds.contains(&Fold::Acknowledged));
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let (ack_line, _) = text
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.contains("elm-shop"))
            .unwrap();
        let buffer = draw_screen(&*screen, &ctx, size);
        for x in 3..11u16 {
            assert!(
                buffer
                    .cell((x, ack_line as u16))
                    .unwrap()
                    .modifier
                    .contains(Modifier::DIM),
                "x={x}"
            );
        }
        assert!(line_of(&text, "Acknowledged")
            .1
            .contains("\u{25be} Acknowledged"));
    }

    fn last_alias(ctx: &AppContext) -> String {
        stop_rows(ctx).last().unwrap().alias.clone()
    }

    #[test]
    fn triage_never_expands() {
        let mut ctx = fleet_with_folds();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::End, KeyCode::Right, KeyCode::Right],
            size,
        );
        assert_eq!(sub_cursor(&ctx), Some(Fold::Todos));
        ctx.status_message = None;
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Enter], size);
        assert!(ctx.inbox_view.open_folds.is_empty(), "Triage never opens");
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Todos are only counted here: see gsd-meta-manager inbox --include todos")
        );
        // `Space` is refused the same way; `v`, `o` and `g` name what to select.
        for (key, want) in [
            ('v', "Select a group row to run its command"),
            ('o', "Select a group row to open its file"),
            ('g', "Select a group row to open its project"),
        ] {
            ctx.status_message = None;
            let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char(key)], size).remove(0);
            assert!(matches!(action, ScreenAction::None), "{key}");
            assert_eq!(
                ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
                Some(want)
            );
        }
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
    }

    #[test]
    fn the_inbox_scrolls_with_the_cursor() {
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        // 14 rows: the body is 9 lines for 13 plan lines (4 titles, 9 rows).
        let size = (100, 14);
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let rule = text.iter().position(|l| l.starts_with('\u{2500}')).unwrap();
        assert!(text[rule].ends_with(" 1-9 of 13"), "{:?}", text[rule]);
        assert!(text[rule].starts_with("\u{2500}\u{2500}"));
        // The fold line is pinned under the rule, whatever scrolls.
        assert!(text[rule + 1].contains("Old debt"));
        // Walking down scrolls whole rows; the cursor row is always drawn.
        for step in 0..8 {
            keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('j')], size);
            let text = render_with(&*screen, &ctx, size.0, size.1);
            let rows = stop_rows(&ctx);
            let at = cursor_pos(&ctx, &rows).unwrap();
            let alias = &rows[at].alias;
            assert!(
                text.iter()
                    .any(|l| l.starts_with(&format!(">\u{25b8} {alias}"))),
                "step {step}: {alias} not drawn\n{text:#?}"
            );
            assert!(text.iter().any(|l| l.contains("Old debt")), "fold pinned");
        }
        let text = render_with(&*screen, &ctx, size.0, size.1);
        let rule = text.iter().position(|l| l.starts_with('\u{2500}')).unwrap();
        assert!(text[rule].ends_with(" 5-13 of 13"), "{:?}", text[rule]);
        // Home brings the first section's title back with the first row.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Home], size);
        let text = render_with(&*screen, &ctx, size.0, size.1);
        assert!(text
            .iter()
            .any(|l| l.contains("TEST \u{2014} closes a phase")));
        assert_eq!(ctx.inbox_view.offset, 0);
        // A section's first row brings its own title along.
        put_cursor_on(&mut ctx, "cedar-web", "07");
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Char('j'), KeyCode::Char('k')],
            size,
        );
        let text = render_with(&*screen, &ctx, size.0, size.1);
        assert!(text.iter().any(|l| l.trim() == "DECIDE"), "{text:#?}");
        // `PgDn` moves by the body height minus 1 (8 stops), clamped to the end.
        keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Home, KeyCode::PageDown],
            size,
        );
        assert_eq!(ctx.inbox_view.cursor_index, 8);
        keys_on(&mut *screen, &mut ctx, &[KeyCode::PageDown], size);
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::FoldLine));
        keys_on(&mut *screen, &mut ctx, &[KeyCode::PageUp], size);
        assert_eq!(ctx.inbox_view.cursor_index, 1);
        // No hint when nothing is clipped; zero-height and zero-stop never panic.
        let text = render_with(&*screen, &ctx, 100, 40);
        assert!(!text.iter().any(|l| l.contains(" of ")));
        let empty = ctx_with_aliases(&["alder-api"]);
        let _ = render_with(&*screen, &empty, 100, 3);
    }

    // --- `o` and `g` ----------------------------------------------------------

    /// `dune-app`'s project folder is a real temp dir holding its UAT file, and
    /// its group points at it.
    fn with_real_uat(ctx: &mut AppContext) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let phase = dir.path().join(".planning/phases/24-clock");
        std::fs::create_dir_all(&phase).unwrap();
        std::fs::write(phase.join("24-UAT.md"), "# Phase 24 UAT\n\n- check one\n").unwrap();
        ctx.config.projects.get_mut("dune-app").unwrap().path = dir.path().to_path_buf();
        ctx.owner_digests.get_mut("dune-app").unwrap().groups[0].file =
            ".planning/phases/24-clock/24-UAT.md".to_owned();
        dir
    }

    /// WR-01: Ctrl or Alt with an action letter (Ctrl-V is the paste reflex, and
    /// a terminal in raw mode delivers it as `Char('v')` with CONTROL) is not the
    /// action: nothing opens, starts or says anything. Shift stays allowed because
    /// `A` arrives shifted.
    #[test]
    fn a_control_or_alt_chord_is_not_an_action_key() {
        let mut ctx = inbox_fleet_ctx();
        let _dir = with_real_uat(&mut ctx);
        let mut screen = open_inbox_and(&mut ctx, &[]);
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ] {
            for key in ['o', 'g', 'v', 'A', 'y'] {
                let action = screen.handle_key(KeyCode::Char(key), modifiers, &mut ctx);
                assert!(
                    matches!(action, ScreenAction::None),
                    "{modifiers:?}+{key} must do nothing"
                );
                assert!(ctx.status_message.is_none(), "{modifiers:?}+{key}");
            }
        }
        // The plain key still works, and so does a shifted one.
        let action = screen.handle_key(KeyCode::Char('o'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Push(_)));
        let action = screen.handle_key(KeyCode::Char('g'), KeyModifiers::SHIFT, &mut ctx);
        assert!(matches!(action, ScreenAction::Push(_)));
    }

    #[test]
    fn o_opens_the_viewer_and_never_the_editor() {
        let mut ctx = inbox_fleet_ctx();
        let _dir = with_real_uat(&mut ctx);
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        let before = ctx.inbox_view.cursor.clone();
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('o')], size).remove(0);
        let ScreenAction::Push(mut viewer) = action else {
            panic!("`o` pushes the viewer, never SuspendAndEdit");
        };
        assert_eq!(viewer.name(), "owner_file_view");
        assert!(ctx.status_message.is_none(), "opening shows no message");
        let text = render_with(&*viewer, &ctx, 100, 12).join("\n");
        assert!(
            text.contains("dune-app \u{b7} .planning/phases/24-clock/24-UAT.md"),
            "{text}"
        );
        assert!(text.contains("Phase 24 UAT"));
        // `Esc` returns; the inbox cursor is where it was.
        let back = viewer.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(back, ScreenAction::Pop));
        assert_eq!(ctx.inbox_view.cursor, before);
        // A missing file is a status line, no push.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('j')], size);
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('o')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Cannot open phases/10/10-UAT.md: not readable")
        );
        // An empty inbox ignores the key and says nothing.
        let mut empty = ctx_with_aliases(&["alder-api"]);
        let mut screen = open_inbox_and(&mut empty, &[]);
        let action = keys_on(&mut *screen, &mut empty, &[KeyCode::Char('o')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert!(empty.status_message.is_none());
    }

    /// `A` (37-06): a refusal is a status line, a good row opens the confirm popup
    /// (nothing runs), and the fold line says what to select.
    #[test]
    fn a_opens_the_acknowledge_popup_and_refuses_in_the_status_line() {
        let mut ctx = inbox_fleet_ctx();
        owner_actions::force_launcher(Some(owner_actions::test_launcher()));
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        // No milestone version: refused, no popup.
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('A')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Cannot acknowledge: dune-app has no known milestone version")
        );
        // With one: the popup, and no status message.
        ctx.project_states.get_mut("dune-app").unwrap().milestone = "v2.0".to_owned();
        ctx.status_message = None;
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('A')], size).remove(0);
        let ScreenAction::Push(popup) = action else {
            panic!("`A` opens the confirm popup");
        };
        assert_eq!(popup.name(), "ack_confirm");
        assert!(ctx.status_message.is_none(), "opening the popup is silent");
        assert!(ctx.owner_ack_busy.is_empty(), "nothing runs before Yes");
        // A busy file is never offered again.
        ctx.owner_ack_busy
            .insert(("dune-app".to_owned(), "phases/24/24-UAT.md".to_owned()));
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('A')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Already acknowledging 24-UAT.md")
        );
        // The fold line is not a group.
        keys_on(&mut *screen, &mut ctx, &[KeyCode::End], size);
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('A')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Select a group row to acknowledge it")
        );
        // An empty inbox ignores the key and says nothing.
        let mut empty = ctx_with_aliases(&["alder-api"]);
        let mut screen = open_inbox_and(&mut empty, &[]);
        let action = keys_on(&mut *screen, &mut empty, &[KeyCode::Char('A')], size).remove(0);
        assert!(matches!(action, ScreenAction::None));
        assert!(empty.status_message.is_none());
    }

    #[test]
    fn o_on_a_blocked_row_over_two_files_points_at_g() {
        let mut ctx = inbox_fleet_ctx();
        let mut second = ctx.owner_digests["iris-gate"].groups[0].clone();
        second.file = "phases/23/23-VERIFICATION.md".to_owned();
        ctx.owner_digests
            .get_mut("iris-gate")
            .unwrap()
            .groups
            .push(second);
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        let action = keys_on(
            &mut *screen,
            &mut ctx,
            &[KeyCode::Char('4'), KeyCode::Char('o')],
            size,
        )
        .remove(1);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("2 files behind this row: press g to pick a file in the Owner block")
        );
    }

    #[test]
    fn g_opens_the_project_on_the_same_group() {
        use super::super::owner_block::Stop;
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let size = (100, 30);
        put_cursor_on(&mut ctx, "cedar-web", "05");
        let fix_key = stop_rows(&ctx)
            .iter()
            .find(|r| r.alias == "cedar-web" && r.src.group.lane == Lane::Fix)
            .unwrap()
            .key();
        ctx.inbox_view.cursor = Some(InboxStop::Row(fix_key.clone()));
        let at = ctx
            .filtered_aliases
            .iter()
            .position(|a| a == "cedar-web")
            .unwrap();
        ctx.table_state.select(Some(0));
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('g')], size).remove(0);
        let ScreenAction::Push(detail) = action else {
            panic!("`g` pushes the detail view itself");
        };
        assert_eq!(detail.name(), "detail");
        assert_eq!(
            ctx.table_state.selected(),
            Some(at),
            "the dashboard follows"
        );
        assert_eq!(
            ctx.view_cache["cedar-web"].owner_block.cursor,
            Some(Stop::Row(fix_key.key.clone()))
        );
        assert!(ctx.status_message.is_none());
        // The detail view holds the keyboard on the Owner block.
        let opened = super::super::detail::DetailScreen::opened_on_owner(
            "cedar-web".to_owned(),
            &fix_key.key,
            &mut ctx,
        );
        assert!(opened.owner_focused());
        // A project the dashboard filter hides still opens, and the table stays.
        ctx.filtered_aliases = vec!["alder-api".to_owned()];
        ctx.table_state.select(Some(0));
        put_cursor_on(&mut ctx, "dune-app", "24");
        let action = keys_on(&mut *screen, &mut ctx, &[KeyCode::Char('g')], size).remove(0);
        assert!(matches!(action, ScreenAction::Push(_)));
        assert_eq!(ctx.table_state.selected(), Some(0));
    }

    #[test]
    fn g_on_a_group_the_block_does_not_list_lands_on_the_tab_top() {
        let mut ctx = fleet_with_folds();
        // holly-pics' archived group is not a stop while its Old debt fold is closed.
        let key = key_of_all(&ctx, "holly-pics", "03");
        let opened = super::super::detail::DetailScreen::opened_on_owner(
            "holly-pics".to_owned(),
            &key,
            &mut ctx,
        );
        assert!(!opened.owner_focused(), "the tab top, focus on the content");
        assert_eq!(ctx.view_cache["holly-pics"].owner_block.cursor, None);
        // An alias with no digest at all opens the same way.
        let none = super::super::detail::DetailScreen::opened_on_owner(
            "alder-api".to_owned(),
            &key,
            &mut ctx,
        );
        assert!(!none.owner_focused());
    }

    fn key_of_all(ctx: &AppContext, alias: &str, phase: &str) -> GroupKey {
        ctx.owner_digests[alias]
            .groups
            .iter()
            .find(|g| g.phase == phase)
            .map(GroupKey::of)
            .expect("a group")
    }

    #[test]
    fn q_from_the_detail_view_returns_to_the_inbox() {
        let mut ctx = inbox_fleet_ctx();
        let mut stack: Vec<Box<dyn Screen>> = vec![Box::new(NormalScreen::new())];
        let ScreenAction::Push(inbox) =
            stack[0].handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx)
        else {
            panic!("`i` pushes the inbox");
        };
        stack.push(inbox);
        let size = (120, 40);
        // Move to the third row, then `g`.
        for code in [KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('g')] {
            let _ = render_with(&**stack.last().unwrap(), &ctx, size.0, size.1);
            match stack
                .last_mut()
                .unwrap()
                .handle_key(code, KeyModifiers::NONE, &mut ctx)
            {
                ScreenAction::Push(screen) => stack.push(screen),
                ScreenAction::None => {}
                _ => panic!("unexpected action"),
            }
        }
        assert_eq!(stack.len(), 3);
        assert_eq!(stack[2].name(), "detail");
        let cursor = ctx.inbox_view.cursor.clone();
        // A frame first (the app always draws before the next key), then `q`.
        let _ = render_with(&**stack.last().unwrap(), &ctx, size.0, size.1);
        let action =
            stack
                .last_mut()
                .unwrap()
                .handle_key(KeyCode::Char('q'), KeyModifiers::NONE, &mut ctx);
        assert!(
            matches!(action, ScreenAction::Pop),
            "`q` leaves the detail view"
        );
        stack.pop();
        assert_eq!(stack.last().unwrap().name(), "inbox");
        assert_eq!(
            ctx.inbox_view.cursor, cursor,
            "the inbox cursor did not move"
        );
    }

    #[test]
    fn a_hostile_drilled_in_inbox_has_no_raw_control_cell_and_nothing_past_the_width() {
        let hostile = "a\u{1b}[31m\u{202e}b\u{200b}c\u{7}";
        let long = format!("{hostile}{}", "x".repeat(300));
        let mut ctx = ctx_with_aliases(&[long.as_str()]);
        let mut g = test_group("24", "widget", 6, true, 2, false);
        g.phase_name = long.clone();
        g.file = long.clone();
        for item in &mut g.items {
            item.name = long.clone();
            item.expected = Some(long.clone());
        }
        let mut old = g.clone();
        old.scope = Scope::Archived {
            label: long.clone(),
            rank: 0,
        };
        let mut d = digest(vec![g, old]);
        d.archived.test = 1;
        owe(&mut ctx, &long, d);
        ctx.status_message = Some((
            format!("Cannot open {long}: not readable"),
            std::time::Instant::now(),
        ));
        let key = key_of(&ctx, &long, "24");
        ctx.inbox_view.expanded.insert(key);
        ctx.inbox_view.open_folds.insert(Fold::OldDebt);
        for width in [80u16, 120] {
            let rows = stop_rows(&ctx);
            for line in full_plan(&ctx, &rows, usize::from(width)) {
                if let BodyItem::Extra(line) = line {
                    assert!(
                        line.width() <= usize::from(width),
                        "{width}: {}",
                        line.width()
                    );
                }
            }
            let screen = InboxScreen::new();
            let buffer = draw_screen(&screen, &ctx, (width, 40));
            for cell in buffer.content() {
                assert!(
                    !cell.symbol().chars().any(char::is_control),
                    "{:?}",
                    cell.symbol()
                );
            }
            let text = render_with(&screen, &ctx, width, 40).join("\n");
            assert!(!text.contains('\u{1b}') && !text.contains('\u{202e}'));
        }
    }

    // -----------------------------------------------------------------------
    // Phase 38: the triage model in the TUI (D-06, D-08, D-09)
    // -----------------------------------------------------------------------

    /// A Test-lane group of `phase` in workstream `market`, `kind` UAT or
    /// VERIFICATION, with its own file and item names.
    fn market_group(kind: Kind, checks: usize, age: i64) -> OwnerGroup {
        let mut g = group(G {
            lane: Lane::Test,
            kind,
            phase: "43",
            name: "ledger-export",
            checks,
            closes: false,
            age: Some(age),
            stale: false,
        });
        g.workstream = Some("market".to_owned());
        let run = RunCommand {
            verb: RunVerb::VerifyWork,
            phase: Some("43".to_owned()),
            workstream: Some("market".to_owned()),
        };
        g.command = Some(run.to_string());
        g.run = Some(run);
        let tag = if kind == Kind::Uat {
            "UAT"
        } else {
            "VERIFICATION"
        };
        g.file = format!("phases/43/43-{tag}.md");
        g.items = (0..checks)
            .map(|i| item(&format!("{tag} item {i}")))
            .collect();
        g
    }

    /// `inbox_triage_ctx` (default fold of 90 days): `market-app` holds a UAT and a
    /// VERIFICATION of one phase and workstream; `ledger` one group 120 days old and
    /// one 5 days old; `script` one UAT of 51 checks.
    fn inbox_triage_ctx() -> AppContext {
        let mut ctx = ctx_with_aliases(&["ledger", "market-app", "script"]);
        owe(
            &mut ctx,
            "market-app",
            digest(vec![
                market_group(Kind::Uat, 2, 9),
                market_group(Kind::Verification, 1, 4),
            ]),
        );
        owe(
            &mut ctx,
            "ledger",
            digest(vec![
                test_group("11", "old-checks", 2, false, 120, false),
                test_group("12", "new-checks", 1, false, 5, false),
            ]),
        );
        owe(
            &mut ctx,
            "script",
            digest(vec![test_group("7", "scripted", 51, false, 3, false)]),
        );
        ctx
    }

    fn rows_of(ctx: &AppContext, alias: &str) -> Vec<InboxRow> {
        stop_rows(ctx)
            .into_iter()
            .filter(|row| row.alias == alias)
            .collect()
    }

    #[test]
    fn a_same_phase_pair_is_one_row() {
        let mut ctx = inbox_triage_ctx();
        let rows = rows_of(&ctx, "market-app");
        assert_eq!(rows.len(), 1, "UAT and VERIFICATION are one row");
        assert_eq!(rows[0].src.group.counts.checks, 3);
        assert_eq!(rows[0].src.parts.len(), 2);
        let text = render_inbox_text(&ctx, 120, 40);
        let (_, row) = line_of(&text, "market-app");
        assert!(row.contains("3 checks"), "{row:?}");
        // Enter lists each file's items under its own sub-heading.
        ctx.inbox_view.expanded.insert(rows[0].key());
        let text = render_inbox_text(&ctx, 120, 40);
        let at = |needle: &str| line_of(&text, needle).0;
        assert!(at("file phases/43/43-UAT.md") < at("UAT item 0"));
        assert!(at("UAT item 1") < at("file phases/43/43-VERIFICATION.md"));
        assert!(at("file phases/43/43-VERIFICATION.md") < at("VERIFICATION item 0"));
    }

    #[test]
    fn stale_entries_leave_the_lanes_for_the_stale_fold() {
        let mut ctx = inbox_triage_ctx();
        let rows = rows_of(&ctx, "ledger");
        assert_eq!(rows.len(), 1, "only the 5-day row is listed");
        assert_eq!(rows[0].src.group.phase, "12");
        assert!(fleet_folds(&ctx).contains(&Fold::Stale));
        let line = line_text(&fold_line(&ctx, 140).expect("a fold line"));
        assert!(
            line.contains("Stale: 2 checks in 1 phase older than 90 d"),
            "{line:?}"
        );
        // The narrow rung counts checks like Old debt, not phases.
        let narrow = line_text(&fold_line(&ctx, 12).expect("a fold line"));
        assert_eq!(narrow, " \u{25b8} Stale (2)");
        ctx.inbox_view.open_folds.insert(Fold::Stale);
        let rows = rows_of(&ctx, "ledger");
        assert_eq!(rows.len(), 2, "the open fold lists the 120-day row");
        assert_eq!(rows[1].src.group.phase, "11");
        assert_eq!(rows[1].fold, Some(Fold::Stale));
        // The header numbers stay those of the digest.
        let closed = inbox_triage_ctx();
        let mut wide = inbox_triage_ctx();
        wide.config
            .preferences
            .extra
            .insert("owner".to_owned(), serde_json::json!({"fold_days": 3650}));
        assert_eq!(
            render_inbox_text(&closed, 120, 40)[0],
            render_inbox_text(&wide, 120, 40)[0]
        );
        assert!(
            !fleet_folds(&wide).contains(&Fold::Stale),
            "no stale entry, no Stale fold"
        );
    }

    #[test]
    fn a_scripted_checklist_is_one_row() {
        let ctx = inbox_triage_ctx();
        assert_eq!(rows_of(&ctx, "script").len(), 1);
        let text = render_inbox_text(&ctx, 120, 40);
        assert!(line_of(&text, "script").1.contains("51 checks"));
    }

    #[test]
    fn a_merged_row_refuses_acknowledge() {
        let mut ctx = inbox_triage_ctx();
        ctx.project_states.get_mut("market-app").unwrap().milestone = "v2.0".to_owned();
        put_cursor_on(&mut ctx, "market-app", "43");
        let action =
            InboxScreen::new().handle_key(KeyCode::Char('A'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None), "no popup");
        assert_eq!(
            status(&ctx),
            "2 files behind this row: press g to pick a file in the Owner block, then A"
        );
        assert!(ctx.owner_ack_busy.is_empty());
    }

    #[test]
    fn g_on_a_merged_row_opens_the_first_file_group() {
        let mut ctx = inbox_triage_ctx();
        put_cursor_on(&mut ctx, "market-app", "43");
        let action =
            InboxScreen::new().handle_key(KeyCode::Char('g'), KeyModifiers::NONE, &mut ctx);
        let ScreenAction::Push(detail) = action else {
            panic!("`g` pushes the detail view");
        };
        assert_eq!(detail.name(), "detail");
        let uat = GroupKey::of(&ctx.owner_digests["market-app"].groups[0]);
        assert_eq!(
            ctx.view_cache["market-app"].owner_block.cursor,
            Some(owner_block::Stop::Row(uat))
        );
    }

    #[test]
    fn v_on_a_merged_row_runs_the_shared_command() {
        let mut ctx = inbox_triage_ctx();
        put_cursor_on(&mut ctx, "market-app", "43");
        let (backend, plan) = recorder();
        press_v(&mut ctx, &plan);
        let opened = backend.opened.lock().unwrap().clone();
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].0, PathBuf::from("/nonexistent/market-app"));
        assert_eq!(
            opened[0].1,
            [
                crate::ui::screens::detail::scrubbed_program(),
                vec!["/gsd-verify-work 43 --ws market".to_owned()]
            ]
            .concat()
        );
    }

    #[test]
    fn y_on_a_merged_row_copies_the_entry_checklist() {
        let mut ctx = inbox_triage_ctx();
        put_cursor_on(&mut ctx, "market-app", "43");
        owner_actions::force_clipboard(Some(wayland_probes()));
        let _ = press_y(&mut ctx);
        let entry = owner_block::entry_for(
            &ctx.owner_digests["market-app"],
            &rows_of(&ctx, "market-app")[0].src.key,
            90,
        )
        .expect("the entry");
        let mut want = crate::inbox::entry_checklist("market-app", &entry).join("\n");
        want.push('\n');
        let ran = owner_actions::seam_ran();
        assert_eq!(ran.last().expect("a copy").1, want);
        assert!(want.contains("file 43-UAT.md") && want.contains("file 43-VERIFICATION.md"));
        owner_actions::force_clipboard(None);
        // `o` keeps its refusal.
        let action =
            InboxScreen::new().handle_key(KeyCode::Char('o'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            status(&ctx),
            "2 files behind this row: press g to pick a file in the Owner block"
        );
    }

    #[test]
    fn y_on_the_stale_fold_says_why_not() {
        let mut ctx = inbox_triage_ctx();
        let mut screen = InboxScreen::new();
        screen.handle_key(KeyCode::End, KeyModifiers::NONE, &mut ctx);
        assert_eq!(sub_cursor(&ctx), Some(Fold::Stale));
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(
            status(&ctx),
            "Nothing to copy: open Stale and copy one phase"
        );
    }

    #[test]
    fn a_merge_appearing_keeps_the_cursor_on_its_row() {
        let mut ctx = ctx_with_aliases(&["market-app"]);
        owe(
            &mut ctx,
            "market-app",
            digest(vec![market_group(Kind::Uat, 2, 9)]),
        );
        put_cursor_on(&mut ctx, "market-app", "43");
        owe(
            &mut ctx,
            "market-app",
            digest(vec![
                market_group(Kind::Uat, 2, 9),
                market_group(Kind::Verification, 1, 4),
            ]),
        );
        reanchor(&mut ctx);
        let rows = stop_rows(&ctx);
        assert_eq!(rows.len(), 1);
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::Row(rows[0].key())));
        // And back: the merge vanishes, the cursor rests on the row that took its index.
        owe(
            &mut ctx,
            "market-app",
            digest(vec![market_group(Kind::Uat, 2, 9)]),
        );
        reanchor(&mut ctx);
        let rows = stop_rows(&ctx);
        assert_eq!(ctx.inbox_view.cursor, Some(InboxStop::Row(rows[0].key())));
    }

    /// Every permutation of `items` (Heap's algorithm).
    fn permutations<T: Clone>(items: &mut Vec<T>, k: usize, out: &mut Vec<Vec<T>>) {
        if k <= 1 {
            out.push(items.clone());
            return;
        }
        for i in 0..k {
            permutations(items, k - 1, out);
            let swap = if k.is_multiple_of(2) { i } else { 0 };
            items.swap(swap, k - 1);
        }
    }

    #[test]
    fn a_partly_folded_blocked_phase_is_one_row() {
        let blocked = |kind: Kind, file: &str, age: i64| {
            let mut g = group(G {
                lane: Lane::Blocked,
                kind,
                phase: "23",
                name: "store-listing",
                checks: 0,
                closes: false,
                age: Some(age),
                stale: false,
            });
            g.file = file.to_owned();
            g.items = vec![item("a")];
            g
        };
        let d = digest(vec![
            blocked(Kind::Uat, "phases/23/23-UAT.md", 3),
            blocked(Kind::Checkpoint, "phases/23/.continue-here.md", 200),
        ]);
        let (lane, stale) = owner_block::inbox_sources(&d, 90);
        let keys: Vec<_> = lane.iter().chain(stale.iter()).map(|s| &s.key).collect();
        assert_eq!(keys.len(), 1, "one aggregate row for the phase: {keys:?}");
        // The newest sibling decides the placement: the fresh file keeps it in the lane.
        assert_eq!(lane.len(), 1);
        assert_eq!(lane[0].files.len(), 2);
    }

    #[test]
    fn inbox_sources_sort_is_total() {
        let mut stale_a = test_group("11", "old-a", 2, false, 120, false);
        stale_a.file = "phases/11/11-UAT.md".to_owned();
        let stale_b = test_group("13", "old-b", 1, false, 130, false);
        let mut groups = vec![
            market_group(Kind::Uat, 2, 9),
            market_group(Kind::Verification, 1, 4),
            test_group("44", "fresh", 1, false, 3, false),
            stale_a,
            stale_b,
        ];
        let mut all = Vec::new();
        let n = groups.len();
        permutations(&mut groups, n, &mut all);
        assert_eq!(all.len(), 120);
        let listing = |groups: Vec<OwnerGroup>| {
            let mut ctx = ctx_with_aliases(&["market-app"]);
            owe(&mut ctx, "market-app", digest(groups));
            ctx.inbox_view.open_folds.insert(Fold::Stale);
            stop_rows(&ctx)
                .iter()
                .map(|row| format!("{:?}", row.src.key))
                .collect::<Vec<_>>()
        };
        let first = listing(all[0].clone());
        assert_eq!(first.len(), 4, "merged row, fresh row, two stale rows");
        for perm in all {
            assert_eq!(listing(perm), first);
        }
    }

    #[test]
    fn the_checks_column_lines_up() {
        let mut ctx = ctx_with_aliases(&["p-one", "p-two", "p-twelve", "p-many"]);
        for (alias, checks, phase) in [
            ("p-one", 1, "1"),
            ("p-two", 2, "2"),
            ("p-twelve", 12, "3"),
            ("p-many", 120, "4"),
        ] {
            owe(
                &mut ctx,
                alias,
                digest(vec![test_group(phase, "counted", checks, false, 3, false)]),
            );
        }
        let text = render_inbox_text(&ctx, 140, 40);
        assert!(text.iter().any(|l| l.contains("120 checks")), "uncut");
        // The digit before the unit sits in one column for every row.
        let digit_x = |alias: &str, unit: &str| {
            let (_, line) = line_of(&text, alias);
            line[..line.find(unit).unwrap()].chars().count() - 1
        };
        let xs: Vec<usize> = [
            ("p-one", " check "),
            ("p-two", " checks"),
            ("p-twelve", " checks"),
        ]
        .iter()
        .map(|(a, u)| digit_x(a, u))
        .collect();
        assert!(xs.iter().all(|x| *x == xs[0]), "{xs:?}");
        // `120 checks` pushes its digits left of the others, unit-aligned: the
        // unit's first letter is in the same column on every row.
        let unit_x = |alias: &str, unit: &str| {
            let (_, line) = line_of(&text, alias);
            line[..line.find(unit).unwrap()].chars().count()
        };
        assert_eq!(unit_x("p-one", " check "), unit_x("p-many", " checks"));
        // The `checks` head ends where the cells end.
        let (_, head) = line_of(&text, "checks  ");
        let head_end = head[..head.find("checks").unwrap()].chars().count() + "checks".len();
        let many_end = unit_x("p-many", " checks") + " checks".len();
        assert_eq!(head_end, many_end, "{head:?}");
    }

    /// Whether the buffer cell at `(x, y)` is drawn DIM.
    fn dim_at(buffer: &ratatui::buffer::Buffer, x: usize, y: usize) -> bool {
        buffer
            .cell((x as u16, y as u16))
            .expect("cell")
            .style()
            .add_modifier
            .contains(Modifier::DIM)
    }

    /// `inbox_triage_ctx` plus `fix-labels`, a FIX row whose phrase and two labels
    /// compete for the phrase column, with the Stale fold open: the 38-UI-REVIEW
    /// fix 3 fleet.
    fn narrow_extras_ctx() -> AppContext {
        let mut ctx = inbox_triage_ctx();
        let mut fix = group(G {
            lane: Lane::Fix,
            kind: Kind::Verification,
            phase: "21",
            name: "settlement-ledger-export",
            checks: 2,
            closes: false,
            age: Some(6),
            stale: false,
        });
        fix.counts.failed = 1;
        fix.counts.gaps = 1;
        fix.items[0].name = "approve the layout".to_owned();
        fix.items[1].name = "bug resolved in 41".to_owned();
        ctx.config.projects.insert(
            "fix-labels".to_owned(),
            ctx.config.projects["ledger"].clone(),
        );
        owe(&mut ctx, "fix-labels", digest(vec![fix]));
        ctx.inbox_view.open_folds.insert(Fold::Stale);
        ctx
    }

    #[test]
    fn extras_stay_whole_at_80_and_100_columns() {
        // A cut label (`4 deci`, `1 looks res`) says nothing: every fragment of a
        // label on a row is the whole label.
        let no_cut_label = |row: &str| {
            for label in ["decision?", "looks resolved", "files"] {
                for n in 2..label.len() {
                    let stub = format!("{}\u{2026}", &label[..n]);
                    assert!(!row.contains(&stub), "{label:?} cut in {row:?}");
                }
            }
        };
        for (w, h) in [(80u16, 24u16), (100, 30)] {
            let ctx = narrow_extras_ctx();
            let text = render_inbox_text(&ctx, w, h);
            let all = text.join("\n");
            for line in &text {
                assert!(line.chars().count() <= w as usize, "{w}: {line:?}");
                no_cut_label(line);
            }
            // The merged row keeps `2 files` whole, its age on the age column.
            let (_, merged) = line_of(&text, "market-");
            assert!(merged.contains("3 checks  2 files"), "{w}: {merged:?}");
            let age_x = |row: &str, age: &str| row[..row.find(age).unwrap()].chars().count();
            let (_, fix) = line_of(&text, "fix-lab");
            assert_eq!(age_x(merged, "4 d"), age_x(fix, "6 d"), "{w}:\n{all}");
            assert!(fix.contains("1 failed check"), "{w}: {fix:?}");
            // The open Stale fold lists its row and the fold line stays pinned.
            assert!(
                line_of(&text, "ph 11").1.contains("2 checks"),
                "{w}:\n{all}"
            );
            assert!(all.contains("\u{25be} Stale"), "{w}:\n{all}");
            if w >= 100 {
                // One label whole, the dropped one traced by ` · …`.
                assert!(fix.contains("1 decision? \u{b7} \u{2026}"), "{fix:?}");
                let buffer = draw(&ctx, w, h);
                let (y, _) = line_of(&text, "fix-lab");
                let x = fix[..fix.find("1 decision?").unwrap()].chars().count();
                assert!(dim_at(&buffer, x, y), "the label count is DIM");
            }
        }
        // With room for both, both labels show and no trace is drawn.
        let text = render_inbox_text(&narrow_extras_ctx(), 140, 30);
        let (_, fix) = line_of(&text, "fix-lab");
        assert!(
            fix.contains("1 failed check, 1 gap  1 decision? \u{b7} 1 looks resolved"),
            "{fix:?}"
        );
    }

    #[test]
    fn labels_are_dim_and_never_move_a_row() {
        let build = |names: [&str; 2]| {
            let mut ctx = ctx_with_aliases(&["lab-a", "lab-b"]);
            let mut a = test_group("5", "labels", 2, false, 4, false);
            a.items[0].name = names[0].to_owned();
            a.items[1].name = names[1].to_owned();
            owe(&mut ctx, "lab-a", digest(vec![a]));
            owe(
                &mut ctx,
                "lab-b",
                digest(vec![test_group("6", "plain", 1, false, 9, false)]),
            );
            ctx
        };
        let plain = build(["first check", "second check"]);
        let mut labeled = build(["approve the layout", "bug resolved in 41"]);
        let key_rows = |ctx: &AppContext| {
            stop_rows(ctx)
                .iter()
                .map(|r| {
                    (
                        r.alias.clone(),
                        r.section,
                        r.src.group.counts.checks,
                        r.fold,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            key_rows(&plain),
            key_rows(&labeled),
            "labels never move a row"
        );
        let text = render_inbox_text(&labeled, 120, 40);
        let (y, row) = line_of(&text, "lab-a");
        assert!(
            row.contains("1 decision? \u{b7} 1 looks resolved"),
            "{row:?}"
        );
        let buffer = draw(&labeled, 120, 40);
        let x = row[..row.find("1 decision?").unwrap()].chars().count();
        assert!(dim_at(&buffer, x, y), "the label count is DIM");
        assert!(!render_inbox_text(&plain, 120, 40)[y].contains("decision?"));
        // An expanded item line ends in its DIM `(label)`.
        let key = rows_of(&labeled, "lab-a")[0].key();
        labeled.inbox_view.expanded.insert(key);
        let text = render_inbox_text(&labeled, 120, 40);
        let (y, line) = line_of(&text, "approve the layout");
        assert!(line.trim_end().ends_with("(decision?)"), "{line:?}");
        let buffer = draw(&labeled, 120, 40);
        let x = line.find("(decision?)").unwrap();
        assert!(dim_at(&buffer, line[..x].chars().count(), y));
        assert!(line_of(&text, "bug resolved in 41")
            .1
            .trim_end()
            .ends_with("(looks resolved)"));
    }
}
