//! The owner presentation core (36 D-11).
//!
//! Pure functions, no I/O: one set of numbers derived from an [`OwnerDigest`]
//! feeds the dashboard `You` cell, and (36-02) the summary line, the attention
//! rank and the `/y` filter, so none of them can disagree about what "owner
//! work" means.
//!
//! The `You` cell is a quiet text cell (report section 3, D-07): every span sets
//! `Color::Reset` explicitly, because the dashboard row carries
//! `fg(row_color)` and a bare span would inherit Red, Yellow or Green from it.

use crate::state_reader::owner_items::{Lane, OwnerDigest, Scope};
use crate::state_reader::phase_num::phase_key;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashSet;

/// The separator between two lanes of the `You` cell.
///
/// U+00B7 is East-Asian-Ambiguous, so this is the one constant a swap to
/// `glyphs::BULLET_OPERATOR` would edit (UI-SPEC Typography).
pub(crate) const OWNER_SEP: &str = " \u{b7} ";

/// The clean-project glyph (U+00B7), dim.
const CLEAN: &str = "\u{b7}";

/// The arrow of ` -> closes M` (U+2192).
const CLOSES: &str = " \u{2192} closes ";

/// What waits on the owner for one project, as plain numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OwnerNumbers {
    pub decide: usize,
    pub test: usize,
    /// `fix_phases + fix_debug` (D-07).
    pub fix: usize,
    /// Whether the fix number counts phases only (no debug investigation), which
    /// is what earns the ` ph` suffix.
    pub fix_is_phases: bool,
    /// Phases that close by testing alone.
    pub closeable: usize,
    /// Archived actionable debt: test + fix + decide (blocked excluded, D-08).
    pub old: usize,
    /// Distinct (workstream, phase) pairs holding current Test work (36-02).
    pub test_phases: usize,
}

/// Map a digest to the numbers every owner surface shares.
pub(crate) fn owner_numbers(digest: &OwnerDigest) -> OwnerNumbers {
    let mut seen: HashSet<(Option<&str>, String)> = HashSet::new();
    for group in &digest.groups {
        if group.lane == Lane::Test
            && group.scope == Scope::Current
            && !group.acknowledged
            && !group.bookkeeping_only
            && !group.items.is_empty()
        {
            seen.insert((group.workstream.as_deref(), phase_key(&group.phase)));
        }
    }
    OwnerNumbers {
        decide: digest.decide,
        test: digest.test,
        fix: digest.fix_phases + digest.fix_debug,
        fix_is_phases: digest.fix_debug == 0,
        closeable: digest.closeable_phases,
        old: digest.archived.test + digest.archived.fix + digest.archived.decide,
        test_phases: seen.len(),
    }
}

/// Whether the project has current-milestone work the owner must act on: the
/// predicate that makes the cell something other than `·` (D-13).
pub(crate) fn has_owner_work(numbers: &OwnerNumbers) -> bool {
    numbers.decide + numbers.test + numbers.fix > 0
}

fn quiet() -> Style {
    Style::default().fg(Color::Reset)
}

fn quiet_dim() -> Style {
    Style::default()
        .fg(Color::Reset)
        .add_modifier(Modifier::DIM)
}

/// The lane parts in the fixed order decide, test, fix, at most two (D-06).
fn lane_parts(numbers: &OwnerNumbers) -> Vec<String> {
    let mut parts = Vec::with_capacity(3);
    if numbers.decide > 0 {
        parts.push(format!("decide {}", numbers.decide));
    }
    if numbers.test > 0 {
        parts.push(format!("test {}", numbers.test));
    }
    if numbers.fix > 0 {
        if numbers.fix_is_phases {
            parts.push(format!("fix {} ph", numbers.fix));
        } else {
            parts.push(format!("fix {}", numbers.fix));
        }
    }
    parts.truncate(2);
    parts
}

fn lanes_line(parts: &[String], closes: Option<usize>, old: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(OWNER_SEP, quiet()));
        }
        spans.push(Span::styled(part.clone(), quiet()));
    }
    if let Some(m) = closes {
        spans.push(Span::styled(format!("{CLOSES}{m}"), quiet()));
    }
    if old > 0 {
        spans.push(Span::styled(format!(" (+{old} old)"), quiet_dim()));
    }
    Line::from(spans)
}

/// The `You` cell: `None` (no digest yet) is blank, never `·` (D-09).
///
/// The first whole form that fits `cells` wins (the `agent_summary_line`
/// ladder): lanes + closes + old, lanes + closes, lanes, the first lane alone,
/// then `fit_cells` with `…`. No form ever ends inside a number.
pub(crate) fn you_cell_line(numbers: Option<&OwnerNumbers>, cells: u16) -> Line<'static> {
    let Some(numbers) = numbers else {
        return Line::default();
    };
    let cells = usize::from(cells);
    if !has_owner_work(numbers) {
        let mut spans = vec![Span::styled(CLEAN, quiet_dim())];
        if numbers.old > 0 {
            spans.push(Span::styled(
                format!(" (+{} old)", numbers.old),
                quiet_dim(),
            ));
        }
        let full = Line::from(spans);
        if full.width() <= cells {
            return full;
        }
        return Line::from(Span::styled(CLEAN, quiet_dim()));
    }

    let parts = lane_parts(numbers);
    let closes = (parts.len() == 1 && numbers.test > 0 && numbers.closeable > 0)
        .then_some(numbers.closeable);
    let forms = [
        lanes_line(&parts, closes, numbers.old),
        lanes_line(&parts, closes, 0),
        lanes_line(&parts, None, 0),
        lanes_line(&parts[..1], None, 0),
    ];
    for form in forms {
        if form.width() <= cells {
            return form;
        }
    }
    Line::from(Span::styled(
        super::detail::fit_cells(&parts[0], cells),
        quiet(),
    ))
}

/// One lane of the `/y [lane]` filter (D-14).
///
/// A closed three-variant enum rather than `owner_items::Lane`, so a
/// `/y blocked` filter cannot be represented: blocked work is not the owner's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerLane {
    Test,
    Decide,
    Fix,
}

/// The count one lane holds for a project (Fix is `fix_phases + fix_debug`).
pub(crate) fn lane_count(numbers: &OwnerNumbers, lane: OwnerLane) -> usize {
    match lane {
        OwnerLane::Test => numbers.test,
        OwnerLane::Decide => numbers.decide,
        OwnerLane::Fix => numbers.fix,
    }
}

/// The fleet totals over the visible rows (D-11): the same numbers the `You`
/// cell shows, summed, so the summary line can never disagree with the table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OwnerTotals {
    pub test: usize,
    /// Per-project distinct (workstream, phase) Test pairs, summed.
    pub test_phases: usize,
    pub closeable: usize,
    pub decide: usize,
    pub fix: usize,
    /// Some visible project has a debug investigation counted in `fix`.
    pub any_fix_debug: bool,
    /// Visible rows with owner work (`K`).
    pub with_work: usize,
    /// Visible rows (`N`).
    pub visible: usize,
    /// Visible rows whose digest has not arrived yet.
    pub pending: usize,
}

/// Sum the owner numbers of the visible rows; `None` is a row still loading.
pub(crate) fn owner_totals<'a>(
    visible: impl IntoIterator<Item = Option<&'a OwnerDigest>>,
) -> OwnerTotals {
    let mut totals = OwnerTotals::default();
    for row in visible {
        totals.visible += 1;
        let Some(digest) = row else {
            totals.pending += 1;
            continue;
        };
        let n = owner_numbers(digest);
        totals.test += n.test;
        totals.test_phases += n.test_phases;
        totals.closeable += n.closeable;
        totals.decide += n.decide;
        totals.fix += n.fix;
        totals.any_fix_debug |= n.fix > 0 && !n.fix_is_phases;
        totals.with_work += usize::from(has_owner_work(&n));
    }
    totals
}

/// The cells `[i] inbox` takes; the left part keeps two cells clear of it.
pub(crate) const INBOX_HINT_CELLS: u16 = 9;

/// The right part of the summary row: `[i]` bold, then ` inbox`.
pub(crate) fn summary_right_spans() -> Vec<Span<'static>> {
    vec![
        Span::styled("[i]", quiet().add_modifier(Modifier::BOLD)),
        Span::styled(" inbox", quiet()),
    ]
}

fn noun(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// The D-10 clauses at one rung of the fit ladder.
fn summary_clauses(
    totals: &OwnerTotals,
    with_projects: bool,
    with_close: bool,
    with_phases: bool,
) -> Vec<String> {
    let mut clauses = Vec::with_capacity(4);
    if totals.test > 0 {
        let mut test = format!("test {}", totals.test);
        if with_phases && totals.test_phases > 0 {
            test.push_str(&format!(
                " in {}",
                noun(totals.test_phases, "phase", "phases")
            ));
        }
        if with_close && totals.closeable > 0 {
            test.push_str(&format!(" ({} close)", totals.closeable));
        }
        clauses.push(test);
    }
    if totals.decide > 0 {
        clauses.push(format!("decide {}", totals.decide));
    }
    if totals.fix > 0 {
        if totals.any_fix_debug {
            clauses.push(format!("fix {}", totals.fix));
        } else {
            clauses.push(format!("fix {}", noun(totals.fix, "phase", "phases")));
        }
    }
    if with_projects && totals.with_work > 0 {
        clauses.push(format!(
            "{} of {}",
            totals.with_work,
            noun(totals.visible, "project", "projects")
        ));
    }
    clauses
}

fn left_line(clauses: &[String], loading: Option<usize>) -> Line<'static> {
    let mut spans = vec![
        Span::styled("You:", quiet().add_modifier(Modifier::BOLD)),
        Span::styled(" ", quiet()),
    ];
    if clauses.is_empty() {
        spans.push(Span::styled(CLEAN, quiet_dim()));
    }
    for (index, clause) in clauses.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(OWNER_SEP, quiet()));
        }
        spans.push(Span::styled(clause.clone(), quiet()));
    }
    if let Some(m) = loading {
        spans.push(Span::styled(format!("{OWNER_SEP}{m} loading"), quiet_dim()));
    }
    Line::from(spans)
}

/// The left part of the summary row (D-10): `You:` and the fleet totals.
///
/// The first whole form that fits `cells` wins: full, without `K of N
/// projects`, without `(C close)`, without ` in P phases`, then `fit_cells`
/// with `…`. The loading clause stays through the four whole forms, because a
/// fail-safe signal outranks detail. No color.
pub(crate) fn summary_left_line(totals: &OwnerTotals, cells: u16) -> Line<'static> {
    let cells = usize::from(cells);
    if totals.visible > 0 && totals.pending == totals.visible {
        return Line::from(vec![
            Span::styled("You:", quiet().add_modifier(Modifier::BOLD)),
            Span::styled(" ", quiet()),
            Span::styled(super::driver::DRY_RUN_LOADING, quiet_dim()),
        ]);
    }
    let loading = (totals.pending > 0).then_some(totals.pending);
    let rungs = [
        (true, true, true),
        (false, true, true),
        (false, false, true),
        (false, false, false),
    ];
    let mut last = Vec::new();
    for (projects, close, phases) in rungs {
        let clauses = summary_clauses(totals, projects, close, phases);
        let line = left_line(&clauses, loading);
        if line.width() <= cells {
            return line;
        }
        last = clauses;
    }
    let body: String = left_line(&last, loading)
        .spans
        .iter()
        .skip(2)
        .map(|s| s.content.as_ref())
        .collect();
    let mut spans = vec![
        Span::styled("You:", quiet().add_modifier(Modifier::BOLD)),
        Span::styled(" ", quiet()),
    ];
    spans.push(Span::styled(
        super::detail::fit_cells(&body, cells.saturating_sub(5)),
        quiet(),
    ));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::Category;
    use crate::state_reader::owner_items::{
        ArchivedCounts, GroupCounts, Kind, OwnerGroup, OwnerItem,
    };

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn numbers(
        decide: usize,
        test: usize,
        fix: usize,
        phases: bool,
        closeable: usize,
    ) -> OwnerNumbers {
        OwnerNumbers {
            decide,
            test,
            fix,
            fix_is_phases: phases,
            closeable,
            ..OwnerNumbers::default()
        }
    }

    fn cell(n: &OwnerNumbers) -> String {
        text(&you_cell_line(Some(n), 21))
    }

    #[test]
    fn you_cell_forms() {
        assert_eq!(
            cell(&numbers(0, 10, 0, true, 4)),
            "test 10 \u{2192} closes 4"
        );
        assert_eq!(cell(&numbers(0, 4, 0, true, 0)), "test 4");
        assert_eq!(cell(&numbers(1, 0, 2, true, 0)), "decide 1 \u{b7} fix 2 ph");
        assert_eq!(cell(&numbers(0, 6, 4, true, 0)), "test 6 \u{b7} fix 4 ph");
        assert_eq!(cell(&numbers(2, 5, 1, true, 0)), "decide 2 \u{b7} test 5");
        assert_eq!(cell(&numbers(0, 0, 2, false, 0)), "fix 2");
        assert_eq!(cell(&numbers(1, 3, 0, true, 2)), "decide 1 \u{b7} test 3");
        assert_eq!(cell(&OwnerNumbers::default()), "\u{b7}");
        assert_eq!(text(&you_cell_line(None, 21)), "");
        assert_eq!(you_cell_line(None, 21).width(), 0);
    }

    #[test]
    fn the_cell_is_quiet_on_every_span() {
        let forms = [
            numbers(0, 10, 0, true, 4),
            numbers(1, 0, 2, true, 0),
            OwnerNumbers::default(),
            OwnerNumbers {
                old: 3,
                ..OwnerNumbers::default()
            },
        ];
        for n in forms {
            let line = you_cell_line(Some(&n), 21);
            assert!(!line.spans.is_empty());
            for span in &line.spans {
                assert_eq!(span.style.fg, Some(Color::Reset), "{:?}", span.content);
            }
        }
        let clean = you_cell_line(Some(&OwnerNumbers::default()), 21);
        assert!(clean.spans[0].style.add_modifier.contains(Modifier::DIM));
    }

    fn group(lane: Lane, phase: &str, ws: Option<&str>) -> OwnerGroup {
        OwnerGroup {
            lane,
            kind: Kind::Uat,
            scope: Scope::Current,
            workstream: ws.map(str::to_string),
            phase: phase.to_string(),
            phase_name: String::new(),
            file: String::new(),
            file_status: String::new(),
            closes_phase: false,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: None,
            stale: false,
            command: None,
            supersedes: Vec::new(),
            counts: GroupCounts::default(),
            items: vec![OwnerItem {
                id: "1".into(),
                test: None,
                name: "t".into(),
                expected: None,
                result: "pending".into(),
                category: Category::Pending,
                reason: None,
                blocked_by: None,
                gaps: Vec::new(),
                why_human: None,
            }],
        }
    }

    #[test]
    fn owner_numbers_maps_the_digest() {
        let mut bookkeeping = group(Lane::Test, "09", None);
        bookkeeping.bookkeeping_only = true;
        let mut acked = group(Lane::Test, "10", None);
        acked.acknowledged = true;
        let mut archived = group(Lane::Test, "11", None);
        archived.scope = Scope::Archived {
            label: "v1".into(),
            rank: 0,
        };
        let digest = OwnerDigest {
            groups: vec![
                group(Lane::Test, "07", None),
                group(Lane::Test, "7", None),
                group(Lane::Test, "07", Some("ws")),
                group(Lane::Fix, "08", None),
                bookkeeping,
                acked,
                archived,
            ],
            test: 5,
            decide: 2,
            fix_phases: 3,
            fix_debug: 0,
            blocked: 9,
            closeable_phases: 4,
            archived: ArchivedCounts {
                test: 1,
                blocked: 9,
                fix: 2,
                decide: 3,
            },
            ..OwnerDigest::default()
        };
        let n = owner_numbers(&digest);
        assert_eq!(n.fix, 3);
        assert!(n.fix_is_phases);
        assert_eq!(n.closeable, 4);
        assert_eq!(n.old, 6, "archived.blocked adds nothing");
        assert_eq!(
            n.test_phases, 2,
            "07 and 7 count once; the ws group is its own"
        );
        assert!(has_owner_work(&n));

        let debug = OwnerDigest {
            fix_phases: 1,
            fix_debug: 1,
            ..OwnerDigest::default()
        };
        let d = owner_numbers(&debug);
        assert_eq!(d.fix, 2);
        assert!(!d.fix_is_phases);

        let blocked_only = OwnerDigest {
            blocked: 4,
            ..OwnerDigest::default()
        };
        assert!(!has_owner_work(&owner_numbers(&blocked_only)));
        let archived_only = OwnerDigest {
            archived: ArchivedCounts {
                test: 3,
                ..ArchivedCounts::default()
            },
            ..OwnerDigest::default()
        };
        assert!(!has_owner_work(&owner_numbers(&archived_only)));
    }

    #[test]
    fn old_debt_suffix_and_the_fit_ladder() {
        let mut n = numbers(1, 0, 0, true, 0);
        n.old = 2;
        assert_eq!(cell(&n), "decide 1 (+2 old)");
        let dim = you_cell_line(Some(&n), 21);
        assert!(dim
            .spans
            .last()
            .unwrap()
            .style
            .add_modifier
            .contains(Modifier::DIM));

        let archived_only = OwnerNumbers {
            old: 3,
            ..OwnerNumbers::default()
        };
        assert_eq!(cell(&archived_only), "\u{b7} (+3 old)");
        for span in &you_cell_line(Some(&archived_only), 21).spans {
            assert!(span.style.add_modifier.contains(Modifier::DIM));
        }

        let mut closes = numbers(0, 10, 0, true, 4);
        closes.old = 9;
        assert_eq!(cell(&closes), "test 10 \u{2192} closes 4");
        assert_eq!(text(&you_cell_line(Some(&closes), 12)), "test 10");

        assert_eq!(
            cell(&numbers(99, 0, 99, true, 0)),
            "decide 99 \u{b7} fix 99 ph"
        );
        assert_eq!(cell(&numbers(100, 0, 99, true, 0)), "decide 100");
        assert_eq!(
            text(&you_cell_line(Some(&numbers(100, 0, 0, true, 0)), 5)),
            "deci\u{2026}"
        );
    }

    #[test]
    fn an_unreadable_file_reads_as_fix() {
        let mut g = group(Lane::Fix, "03", None);
        g.counts.unreadable = 1;
        let digest = OwnerDigest {
            groups: vec![g],
            fix_phases: 1,
            ..OwnerDigest::default()
        };
        assert_eq!(cell(&owner_numbers(&digest)), "fix 1 ph");
    }
    fn counts(test: usize, closeable: usize, decide: usize, fix_phases: usize) -> OwnerDigest {
        OwnerDigest {
            test,
            closeable_phases: closeable,
            decide,
            fix_phases,
            ..OwnerDigest::default()
        }
    }

    /// The counts of the 15-project fleet (synthetic), `None` for the pending one.
    fn fleet() -> Vec<Option<OwnerDigest>> {
        let debug = OwnerDigest {
            fix_debug: 2,
            ..OwnerDigest::default()
        };
        vec![
            Some(counts(3, 1, 0, 0)),
            Some(counts(10, 4, 0, 0)),
            Some(counts(6, 0, 0, 4)),
            Some(counts(12, 3, 0, 0)),
            Some(counts(0, 0, 1, 2)),
            Some(counts(0, 0, 1, 0)),
            Some(OwnerDigest::default()),
            Some(OwnerDigest::default()),
            Some(OwnerDigest {
                blocked: 4,
                ..OwnerDigest::default()
            }),
            None,
            Some(counts(5, 0, 2, 1)),
            Some(debug),
            Some(OwnerDigest {
                fix_phases: 1,
                fix_debug: 1,
                ..OwnerDigest::default()
            }),
            Some(counts(0, 0, 1, 0)),
            Some(counts(4, 0, 0, 0)),
        ]
    }

    #[test]
    fn owner_totals_sum_the_visible_rows() {
        let digests = fleet();
        let t = owner_totals(digests.iter().map(Option::as_ref));
        assert_eq!(t.test, 40);
        assert_eq!(t.closeable, 8);
        assert_eq!(t.decide, 5);
        assert_eq!(t.fix, 11);
        assert!(t.any_fix_debug);
        assert_eq!(t.with_work, 11);
        assert_eq!(t.visible, 15);
        assert_eq!(t.pending, 1);
        assert_eq!(owner_totals(std::iter::empty()), OwnerTotals::default());
    }

    fn left(t: &OwnerTotals) -> String {
        text(&summary_left_line(t, 200))
    }

    #[test]
    fn summary_forms() {
        let digests = fleet();
        let mut t = owner_totals(digests.iter().map(Option::as_ref));
        assert_eq!(
            left(&t),
            "You: test 40 (8 close) \u{b7} decide 5 \u{b7} fix 11 \u{b7} 11 of 15 projects \u{b7} 1 loading"
        );
        t.test_phases = 20;
        t.closeable = 17;
        assert!(left(&t).starts_with("You: test 40 in 20 phases (17 close)"));
        t.test_phases = 1;
        assert!(left(&t).starts_with("You: test 40 in 1 phase (17 close)"));
        t.visible = 1;
        t.with_work = 1;
        t.pending = 0;
        assert!(left(&t).ends_with("1 of 1 project"));
        t.any_fix_debug = false;
        t.fix = 1;
        assert!(left(&t).contains("fix 1 phase \u{b7}"));
        t.fix = 3;
        assert!(left(&t).contains("fix 3 phases \u{b7}"));

        let idle = OwnerTotals {
            visible: 2,
            ..OwnerTotals::default()
        };
        let line = summary_left_line(&idle, 80);
        assert_eq!(text(&line), "You: \u{b7}");
        assert!(line.spans[2].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(left(&OwnerTotals::default()), "You: \u{b7}");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));

        let loading = OwnerTotals {
            visible: 3,
            pending: 3,
            ..OwnerTotals::default()
        };
        let line = summary_left_line(&loading, 80);
        assert_eq!(text(&line), "You: Loading...");
        assert!(line.spans[2].style.add_modifier.contains(Modifier::DIM));

        let some_loading_no_work = OwnerTotals {
            visible: 3,
            pending: 1,
            ..OwnerTotals::default()
        };
        assert_eq!(left(&some_loading_no_work), "You: \u{b7} \u{b7} 1 loading");
    }

    #[test]
    fn summary_ladder() {
        let t = OwnerTotals {
            test: 40,
            test_phases: 20,
            closeable: 17,
            decide: 5,
            fix: 11,
            any_fix_debug: true,
            with_work: 11,
            visible: 15,
            pending: 1,
        };
        let at = |cells: u16| text(&summary_left_line(&t, cells));
        let full = at(200);
        assert_eq!(
            full,
            "You: test 40 in 20 phases (17 close) \u{b7} decide 5 \u{b7} fix 11 \u{b7} 11 of 15 projects \u{b7} 1 loading"
        );
        let w = |s: &str| Span::raw(s.to_string()).width() as u16;
        let r2 =
            "You: test 40 in 20 phases (17 close) \u{b7} decide 5 \u{b7} fix 11 \u{b7} 1 loading";
        assert_eq!(at(w(r2)), r2);
        let r3 = "You: test 40 in 20 phases \u{b7} decide 5 \u{b7} fix 11 \u{b7} 1 loading";
        assert_eq!(at(w(r3)), r3);
        let r4 = "You: test 40 \u{b7} decide 5 \u{b7} fix 11 \u{b7} 1 loading";
        assert_eq!(at(w(r4)), r4);
        let r5 = at(w(r4) - 1);
        assert!(r5.ends_with('\u{2026}'), "{r5}");
        assert!(Span::raw(r5.clone()).width() <= usize::from(w(r4) - 1));
        assert!(r5.starts_with("You: test 40"));
        // No rung from 2 to 4 ends inside a number or loses the loading clause.
        for rung in [r2, r3, r4] {
            assert!(rung.ends_with("1 loading"));
        }
    }

    #[test]
    fn lanes_and_hint() {
        let n = numbers(1, 2, 3, true, 0);
        assert_eq!(lane_count(&n, OwnerLane::Decide), 1);
        assert_eq!(lane_count(&n, OwnerLane::Test), 2);
        assert_eq!(lane_count(&n, OwnerLane::Fix), 3);
        let hint: String = summary_right_spans()
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(hint, "[i] inbox");
        assert_eq!(hint.len() as u16, INBOX_HINT_CELLS);
    }
}
