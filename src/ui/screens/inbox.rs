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

use super::detail::{fit_cells, fit_spans, shown};
use super::driver::DRY_RUN_LOADING;
use super::owner::{owner_numbers, OWNER_SEP};
use super::owner_actions;
use super::owner_block::{self, blanks, cells, pad, GroupKey, RowCells, RowSrc};
use super::{AppContext, Screen, ScreenAction};
use crate::inbox::phase_key;
use crate::state_reader::owner_items::Lane;
use crate::ui::glyphs;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cell::Cell;
use std::cmp::Ordering;

/// A row's identity across digest refreshes: the project and the group inside it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InboxKey {
    pub alias: String,
    pub key: GroupKey,
}

/// One cursor stop. The fold line joins in 37-05.
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
/// Built from [`owner_block::current_sources`], so there is no second row
/// selection rule. A project with no digest yet (or whose folder is gone) has no
/// rows: absent data is not clean data, and the loading copy is 37-04's.
pub(crate) fn inbox_rows(ctx: &AppContext) -> Vec<InboxRow> {
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
        for src in owner_block::current_sources(digest) {
            rows.push(InboxRow {
                alias: alias.clone(),
                section: Section::of(&src),
                src,
                progress,
                phases_open,
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

/// The index of the cursor in `rows`; a vanished identity resolves to its last
/// index clamped to the list. `None` for an empty list.
pub(crate) fn resolve_cursor(view: &InboxViewState, rows: &[InboxRow]) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let found = match &view.cursor {
        Some(InboxStop::Row(key)) => rows.iter().position(|row| row.key() == *key),
        _ => None,
    };
    Some(found.unwrap_or_else(|| view.cursor_index.min(rows.len() - 1)))
}

/// After a digest update (a watcher refresh, a first fill, a failure) move the
/// cursor's identity onto what resolves now (D-10, SC3): the group itself when it
/// survives, else the row that took its place (the next one), else the previous
/// one, else nothing. Without a cursor there is nothing to anchor: the first row
/// is implicit.
pub(crate) fn reanchor(ctx: &mut AppContext) {
    if ctx.inbox_view.cursor.is_none() {
        return;
    }
    let rows = visible_rows(ctx);
    match resolve_cursor(&ctx.inbox_view, &rows) {
        Some(at) => {
            ctx.inbox_view.cursor = Some(InboxStop::Row(rows[at].key()));
            ctx.inbox_view.cursor_index = at;
        }
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

/// Width of the Checks column (`12 checks`).
const CHECKS_W: usize = 9;
/// Width of the Age column (`203 d stale`).
const AGE_W: usize = 11;
/// The Age column shows from this width up (UI-SPEC narrowing order, step 2).
const AGE_MIN_W: usize = 80;
/// The FIX Command column's floor and cap (UI-SPEC "Row grid").
const COMMAND_MIN_W: usize = 12;
const COMMAND_MAX_W: usize = 44;
/// Cursor, marker and the space after them.
const GUTTER_W: usize = 3;

/// One row's cells, escaped, for layout.
struct Cells {
    alias: String,
    section: Section,
    cells: RowCells,
}

fn cells_of(ctx: &AppContext, row: &InboxRow) -> Option<Cells> {
    let digest = ctx.owner_digest(&row.alias)?;
    let mut cells = row.src.cells(digest);
    // Section 1 says "closes a phase" in its title: no per-row phrase there.
    if row.section == Section::TestCloses && !row.src.group.bookkeeping_only {
        cells.phrase.clear();
    }
    Some(Cells {
        alias: shown(&row.alias),
        section: row.section,
        cells,
    })
}

/// The column widths of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grid {
    pub alias_w: usize,
    pub phase_w: usize,
    pub name_w: usize,
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
            + CHECKS_W
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
            + CHECKS_W
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
/// other column, Age by width, then the phrase toward 12, the phase cell toward 5,
/// the alias toward 8 and the name toward 8; whatever is still too wide is cut by
/// `fit_spans` per row.
fn grid_for_inbox(all: &[Cells], width: usize) -> Grid {
    let widest = |f: &dyn Fn(&Cells) -> usize| all.iter().map(f).max().unwrap_or(0);
    let phrase_nat = widest(&|c| cells(&c.cells.phrase));
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
        phrase_w: if phrase_nat == 0 {
            0
        } else {
            phrase_nat.clamp(12, 32)
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
        if grid.total() + 2 + natural <= width {
            grid.command_w = natural;
        }
    }
    while grid.total() > width && grid.phrase_w > 12 {
        grid.phrase_w -= 1;
    }
    while grid.total() > width && grid.phase_w > 5 {
        grid.phase_w -= 1;
    }
    while grid.total() > width && grid.alias_w > 8 {
        grid.alias_w -= 1;
    }
    while grid.total() > width && grid.name_w > 8 {
        grid.name_w -= 1;
    }
    grid
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// One group row as spans, cut to `width`, the cursor row Cyan + BOLD throughout.
fn row_line(c: &Cells, grid: &Grid, width: usize, cursor: bool) -> Line<'static> {
    let plain = owner_block::style_for(&c.cells, Style::default());
    let mut spans: Vec<Span<'static>> = vec![
        Span::raw(if cursor { ">" } else { " " }),
        Span::raw(glyphs::TRIANGLE_RIGHT_SMALL),
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
        spans.push(Span::styled(pad(&c.cells.checks, CHECKS_W, true), plain));
        if grid.phrase_w > 0 {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                pad(&c.cells.phrase, grid.phrase_w, false),
                plain,
            ));
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
            spans.push(Span::styled(pad("checks", CHECKS_W, true), dim()));
            if grid.age_w > 0 {
                let used = grid.checks_x() + CHECKS_W;
                spans.push(Span::raw(blanks(grid.age_x().saturating_sub(used))));
                spans.push(Span::styled("age", dim()));
            }
        }
    }
    fit_spans(spans, width)
}

/// One line of the body: a section title or the row with that index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyItem {
    Title(Section),
    Row(usize),
}

/// The body's lines over the visible rows, titles in front of each section.
fn body_plan(rows: &[InboxRow]) -> Vec<BodyItem> {
    let mut plan = Vec::new();
    let mut last: Option<Section> = None;
    for (index, row) in rows.iter().enumerate() {
        if last != Some(row.section) {
            plan.push(BodyItem::Title(row.section));
            last = Some(row.section);
        }
        plan.push(BodyItem::Row(index));
    }
    plan
}

/// The first body line drawn so the cursor row is visible, keeping `offset` when
/// it already is. A row that opens its section brings its title with it.
fn scroll_for(offset: usize, plan: &[BodyItem], cursor_row: usize, height: usize) -> usize {
    if height == 0 {
        return 0;
    }
    let line = plan
        .iter()
        .position(|item| *item == BodyItem::Row(cursor_row))
        .unwrap_or(0);
    let top = if line > 0 && matches!(plan[line - 1], BodyItem::Title(_)) {
        line - 1
    } else {
        line
    };
    let mut offset = offset;
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
/// plans insert their key at its UI-SPEC position (`Enter`, `g`, `o`, `A`, `y`)
/// when it works.
const FOOTER_KEYS: [FooterKey; 4] = [
    FooterKey {
        key: "[j/k]",
        labels: ["move", "move", "move"],
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

/// The hints of one rung, two leading spaces, `[key]` BOLD.
fn footer_rung(rung: usize, has_stops: bool) -> Vec<Span<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::raw("  ")];
    let mut first = true;
    for hint in &FOOTER_KEYS {
        // With no stop (an empty inbox) only the keys that need none remain.
        let keep = match hint.key {
            "[j/k]" | "[v]" => has_stops,
            _ => true,
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
/// hint ladder that fits `width`.
fn footer_line(ctx: &AppContext, width: usize, has_stops: bool) -> Line<'static> {
    if let Some((message, _)) = &ctx.status_message {
        // The status line is composed from authored copy and project text in one
        // string, so the escape happens here, at the consumer (as on the dashboard).
        return Line::from(Span::raw(fit_cells(&shown(message), width)));
    }
    let mut last = Vec::new();
    for rung in 0..4 {
        let spans = footer_rung(rung, has_stops);
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
    acknowledged: usize,
    todos: usize,
    todo_projects: usize,
    todos_unreadable: usize,
}

fn fold_totals(ctx: &AppContext) -> FoldTotals {
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
            && self.acknowledged == 0
            && self.todos == 0
            && self.todos_unreadable == 0
    }
}

/// The pinned fold line (UI-SPEC "Fold-line ladder"): the first of three rungs
/// that fits, folds without content omitted, then `fit_cells` on the shortest.
/// `None` when no fold has content. Collapsed only; opening is 37-05.
fn fold_line(ctx: &AppContext, width: usize) -> Option<Line<'static>> {
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
    // (old debt, acknowledged, triage) texts of one rung; the third is DIM.
    let rung = |n: usize| -> Vec<(String, bool)> {
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
            ));
        }
        if t.acknowledged > 0 {
            parts.push((
                match n {
                    0 | 1 => format!("Acknowledged ({})", plural(t.acknowledged, "file", "files")),
                    _ => format!("Acknowledged ({})", t.acknowledged),
                },
                false,
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
            parts.push((text, true));
        }
        parts
    };
    let build = |parts: Vec<(String, bool)>| -> Vec<Span<'static>> {
        let mut spans = vec![Span::raw(" ")];
        for (index, (text, muted)) in parts.into_iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw("   "));
            }
            let style = if muted { dim() } else { Style::default() };
            spans.push(Span::styled(
                format!("{} {text}", glyphs::TRIANGLE_RIGHT_SMALL),
                style,
            ));
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
}

/// The rows under the body.
#[derive(Clone, Copy)]
struct Pinned {
    rule: Rect,
    fold: Rect,
    footer: Rect,
    has_stops: bool,
}

impl InboxScreen {
    /// The rule row, the pinned fold line and the footer under the body.
    fn render_pinned(&self, frame: &mut Frame, ctx: &AppContext, pinned: Pinned, width: usize) {
        if pinned.rule.height > 0 {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled("\u{2500}".repeat(width), dim()))),
                pinned.rule,
            );
        }
        if let (Some(fold), true) = (fold_line(ctx, width), pinned.fold.height > 0) {
            frame.render_widget(Paragraph::new(fold), pinned.fold);
        }
        frame.render_widget(
            Paragraph::new(footer_line(ctx, width, pinned.has_stops)),
            pinned.footer,
        );
    }

    pub fn new() -> Self {
        Self {
            viewport: Cell::new(0),
        }
    }

    /// Move the cursor to row `to` (clamped) and keep it in view.
    fn move_to(&self, ctx: &mut AppContext, rows: &[InboxRow], to: usize) {
        let Some(row) = rows.get(to.min(rows.len().saturating_sub(1))) else {
            return;
        };
        let to = to.min(rows.len() - 1);
        ctx.inbox_view.cursor = Some(InboxStop::Row(row.key()));
        ctx.inbox_view.cursor_index = to;
        let plan = body_plan(rows);
        ctx.inbox_view.offset =
            scroll_for(ctx.inbox_view.offset, &plan, to, self.viewport.get().max(1));
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
        let rows = visible_rows(ctx);
        let Some(at) = resolve_cursor(&ctx.inbox_view, &rows) else {
            return;
        };
        let to = at.saturating_add_signed(by);
        self.move_to(ctx, &rows, to);
        ctx.needs_redraw = true;
    }

    /// `v` (D-11, D-12): run the selected group's command through `launch`. No
    /// stop, no message, no action.
    pub(crate) fn run_key(
        &self,
        ctx: &mut AppContext,
        launch: &mut dyn FnMut(&owner_actions::RunPlan) -> String,
    ) -> ScreenAction {
        let rows = visible_rows(ctx);
        let Some(at) = resolve_cursor(&ctx.inbox_view, &rows) else {
            return ScreenAction::None;
        };
        let row = &rows[at];
        // The group is looked up by its identity as the key is pressed; a BLOCKED
        // aggregate has no single group, so its row's own group stands in and the
        // run path refuses it by lane.
        let group = ctx
            .owner_digest(&row.alias)
            .and_then(|digest| owner_block::group_for(digest, &row.src.key))
            .unwrap_or_else(|| row.src.group.clone());
        let message = owner_actions::run_group(ctx, &row.alias, &group, launch);
        ctx.status_message = Some((message, std::time::Instant::now()));
        ctx.needs_redraw = true;
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
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(super::help::HelpScreen::new()))
            }
            KeyCode::Char('v') => self.run_key(ctx, &mut owner_actions::launch_real),
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

        let rows = visible_rows(ctx);
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
                has_stops: false,
            };
            self.render_pinned(frame, ctx, pinned, width);
            return;
        }
        let every: Vec<Cells> = inbox_rows(ctx)
            .iter()
            .filter_map(|row| cells_of(ctx, row))
            .collect();
        let grid = grid_for_inbox(&every, width);
        let plan = body_plan(&rows);
        let height = usize::from(body.height);
        self.viewport.set(height);
        let cursor = resolve_cursor(&ctx.inbox_view, &rows);
        let offset = match cursor {
            Some(at) => scroll_for(ctx.inbox_view.offset, &plan, at, height),
            None => 0,
        };
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(height);
        for (line_index, item) in plan.iter().enumerate().skip(offset).take(height) {
            lines.push(match item {
                BodyItem::Title(section) => {
                    title_line(*section, (line_index == 0).then_some(&grid), width)
                }
                BodyItem::Row(index) => match cells_of(ctx, &rows[*index]) {
                    Some(c) => row_line(&c, &grid, width, cursor == Some(*index)),
                    None => Line::from(""),
                },
            });
        }
        frame.render_widget(Paragraph::new(lines), body);
        let pinned = Pinned {
            rule,
            fold,
            footer,
            has_stops: true,
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
        assert!(text[29].contains("[v]run command"));
        assert!(!text[29].contains("[g]"));
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

    fn cursor_alias(ctx: &AppContext) -> Option<String> {
        let rows = visible_rows(ctx);
        resolve_cursor(&ctx.inbox_view, &rows).map(|at| rows[at].alias.clone())
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
        // Narrower: phrase to 12, phase to 5, alias to 8, then the name; always
        // within W, the name floored at 8.
        let g60 = grid(60);
        assert_eq!(g60.phrase_w, 12);
        assert!(g60.total() <= 60 && g60.name_w >= 8 && g60.name_w < 18);
        assert_eq!((g60.alias_w, g60.phase_w), (8, 5));
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
        // The footer ladder: the first rung that fits.
        let ctx = inbox_fleet_ctx();
        let footer = |w: usize| line_text(&footer_line(&ctx, w, true));
        assert_eq!(
            footer(47),
            "  [j/k]move  [v]run command  [Esc]back  [?]help"
        );
        assert_eq!(footer(46), "  [j/k]move  [v]run  [Esc]back  [?]help");
        assert_eq!(footer(37), "  [j/k]move  [v]run  [?]help");
        assert_eq!(footer(26), "  [v]run  [?]help");
        assert_eq!(
            footer(10),
            "  [v]run \u{2026}",
            "the minimal rung is cut to W"
        );
        assert_eq!(
            line_text(&footer_line(&ctx, 80, false)),
            "  [Esc]back  [?]help",
            "no stops, no movement or run key"
        );
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
                footer_line(&ctx, w, true).width() <= w,
                "footer W {w}: {:?}",
                footer_line(&ctx, w, true)
            );
            if let Some(fold) = fold_line(&ctx, w) {
                assert!(fold.width() <= w, "fold W {w}");
            }
            for row in &rows {
                let c = cells_of(&ctx, row).expect("cells");
                for cursor in [false, true] {
                    assert!(row_line(&c, &grid, w, cursor).width() <= w, "row W {w}");
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
            ["env", "claude", "/gsd-verify-work 24"].map(String::from)
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
            ["env", "claude", "/gsd-verify-work 10"].map(String::from)
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
        let at = resolve_cursor(&ctx.inbox_view, &rows).expect("a cursor");
        assert_eq!(rows[at].alias, "alder-api");
        // The row under the cursor vanishes: the selection clamps to a neighbor.
        ctx.owner_digests
            .insert("alder-api".to_owned(), OwnerDigest::default());
        let rows = visible_rows(&ctx);
        let at = resolve_cursor(&ctx.inbox_view, &rows).expect("a cursor");
        assert!(at < rows.len());
        screen.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
    }

    #[test]
    fn j_k_home_end_and_paging_move_the_cursor() {
        let mut ctx = inbox_fleet_ctx();
        let mut screen = open_inbox_and(&mut ctx, &[]);
        let rows = visible_rows(&ctx);
        let at = |ctx: &AppContext| resolve_cursor(&ctx.inbox_view, &rows).unwrap();
        assert_eq!(at(&ctx), 0);
        screen.handle_key(KeyCode::End, KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len() - 1);
        screen.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len() - 1, "stays on the last row");
        screen.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(at(&ctx), rows.len() - 2);
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
}
