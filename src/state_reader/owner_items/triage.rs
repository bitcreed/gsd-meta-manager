//! Layer B triage: the digest's groups as the entries the owner reads (Phase 38).
//!
//! Pure functions of one [`OwnerDigest`], the `fold_days` preference and, for the
//! ranking, nothing else: no clock is read, so every view built from them (the CLI
//! digest, `inbox ALIAS`, the lane view, the TUI inbox) agrees.
//!
//! * **D-08, same-phase merge.** Current, non-acknowledged UAT and VERIFICATION
//!   groups of one workstream, phase and lane become one [`TriageEntry`]. This is
//!   display grouping only: the digest's groups, counts and the D3 strict dedup are
//!   untouched, and no item is dropped.
//! * **D-09, stale fold.** An owner entry whose newest age exceeds `fold_days` is
//!   `folded`: it leaves NEXT and the lane sections, while every total, the gate and
//!   `status --json` `owner` keep counting it.
//! * **D-07, ranking.** [`next_actions`] picks the few entries the owner should work
//!   next: those that close a phase first, then the fewest checks, then the newest.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::{gate, Kind, Lane, OwnerDigest, OwnerGroup, Scope};
use crate::state_reader::phase_num::{phase_key, same_phase};

/// How many next actions the digest names.
pub const NEXT_LIMIT: usize = 5;

/// Whether `group` is a row of the owner's four lanes: current milestone, not
/// acknowledged, lane Test, Decide, Fix or Blocked.
pub fn is_owner_row(group: &OwnerGroup) -> bool {
    matches!(group.scope, Scope::Current)
        && !group.acknowledged
        && matches!(
            group.lane,
            Lane::Test | Lane::Decide | Lane::Fix | Lane::Blocked
        )
}

/// One entry the owner reads: a single group, or the merge of several (D-08).
#[derive(Debug, Clone)]
pub struct TriageEntry {
    /// The entry as one group: a clone of the only part, or the synthetic merge.
    pub group: OwnerGroup,
    /// The digest groups the entry stands for, in digest order.
    pub parts: Vec<OwnerGroup>,
    /// The positions of `parts` in `digest.groups`.
    pub indices: Vec<usize>,
    /// Older than `fold_days` (D-09).
    pub folded: bool,
}

impl TriageEntry {
    /// The file of every part, in part order.
    pub fn files(&self) -> Vec<&str> {
        self.parts.iter().map(|part| part.file.as_str()).collect()
    }

    /// Whether the entry stands for more than one group.
    pub fn is_merged(&self) -> bool {
        self.parts.len() > 1
    }
}

fn mergeable(group: &OwnerGroup) -> bool {
    is_owner_row(group) && matches!(group.kind, Kind::Uat | Kind::Verification)
}

/// The entry group of a merge (D-08).
fn merged_group(parts: &[OwnerGroup]) -> OwnerGroup {
    let mut group = parts[0].clone();
    if parts.len() == 1 {
        return group;
    }
    group.counts.checks = parts.iter().map(|p| p.counts.checks).sum();
    group.counts.failed = parts.iter().map(|p| p.counts.failed).sum();
    group.counts.gaps = parts.iter().map(|p| p.counts.gaps).sum();
    group.counts.unreadable = parts.iter().map(|p| p.counts.unreadable).sum();
    group.items = parts.iter().flat_map(|p| p.items.clone()).collect();
    group.supersedes = parts.iter().flat_map(|p| p.supersedes.clone()).collect();
    // The newest age, with the stale flag of the part that holds it.
    let newest = parts
        .iter()
        .filter(|p| p.age_days.is_some())
        .min_by_key(|p| p.age_days);
    group.age_days = newest.and_then(|p| p.age_days);
    group.stale = newest.is_some_and(|p| p.stale);
    // The first part that has a command carries it (with its typed form).
    match parts.iter().find(|p| p.command.is_some()) {
        Some(part) => {
            group.run = part.run.clone();
            group.command = part.command.clone();
        }
        None => {
            group.run = None;
            group.command = None;
        }
    }
    group.closes_phase = parts.iter().all(|p| p.closes_phase);
    group.bookkeeping_only = parts.iter().all(|p| p.bookkeeping_only);
    group.acknowledged = false;
    group
}

/// The digest's groups as entries: same-phase merge (D-08), then the stale fold
/// (D-09). Every group belongs to exactly one entry; entries keep the digest order
/// of their first group.
pub fn triage_entries(digest: &OwnerDigest, fold_days: u32) -> Vec<TriageEntry> {
    let mut slots: Vec<Vec<usize>> = Vec::new();
    let mut by_key: HashMap<(Option<String>, String, u8), usize> = HashMap::new();
    for (index, group) in digest.groups.iter().enumerate() {
        if mergeable(group) {
            let key = (
                group.workstream.clone(),
                phase_key(&group.phase),
                group.lane.order(),
            );
            if let Some(&slot) = by_key.get(&key) {
                slots[slot].push(index);
                continue;
            }
            by_key.insert(key, slots.len());
        }
        slots.push(vec![index]);
    }
    slots
        .into_iter()
        .map(|indices| {
            let parts: Vec<OwnerGroup> = indices
                .iter()
                .map(|&index| digest.groups[index].clone())
                .collect();
            let group = merged_group(&parts);
            let folded = is_owner_row(&group)
                && group
                    .age_days
                    .is_some_and(|days| days > i64::from(fold_days));
            TriageEntry {
                group,
                parts,
                indices,
                folded,
            }
        })
        .collect()
}

/// What the folded entries hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StaleTotals {
    pub entries: usize,
    pub items: usize,
    /// A folded entry is not a UAT or VERIFICATION one, so its count is not checks.
    pub non_check: bool,
}

/// The totals of the folded entries among `entries`.
pub fn stale_totals<'a>(entries: impl IntoIterator<Item = &'a TriageEntry>) -> StaleTotals {
    let mut totals = StaleTotals::default();
    for entry in entries.into_iter().filter(|entry| entry.folded) {
        totals.entries += 1;
        totals.items += entry.group.counts.checks;
        if !matches!(entry.group.kind, Kind::Uat | Kind::Verification) {
            totals.non_check = true;
        }
    }
    totals
}

/// Whether the entry ranks among the ones that close a phase.
pub fn ranks_as_closing(group: &OwnerGroup) -> bool {
    group.closes_phase || group.bookkeeping_only
}

/// The number of checks the owner has to do for the entry: none for a bookkeeping
/// one (only the finishing command is left).
pub fn ranked_checks(group: &OwnerGroup) -> usize {
    if group.bookkeeping_only {
        0
    } else {
        group.counts.checks
    }
}

/// One ranked next action.
#[derive(Debug, Clone, Copy)]
pub struct NextAction<'a> {
    pub alias: &'a str,
    pub entry: &'a TriageEntry,
}

fn age_key(age: Option<i64>) -> (bool, i64) {
    (age.is_none(), age.unwrap_or(0))
}

fn first_file(entry: &TriageEntry) -> &str {
    entry.parts.first().map_or("", |part| part.file.as_str())
}

fn rank(a: &NextAction, b: &NextAction) -> Ordering {
    let (ga, gb) = (&a.entry.group, &b.entry.group);
    ranks_as_closing(gb)
        .cmp(&ranks_as_closing(ga))
        .then(ranked_checks(ga).cmp(&ranked_checks(gb)))
        .then(age_key(ga.age_days).cmp(&age_key(gb.age_days)))
        .then(a.alias.cmp(b.alias))
        .then_with(|| phase_key(&ga.phase).cmp(&phase_key(&gb.phase)))
        .then(ga.lane.order().cmp(&gb.lane.order()))
        .then_with(|| first_file(a.entry).cmp(first_file(b.entry)))
}

/// The next actions across `projects` (alias, entries), best first, at most `limit`.
///
/// Candidates are current, non-acknowledged, non-folded Test, Decide and Fix
/// entries that have a command (an entry without one, such as a CONTEXT question,
/// names nothing to run). Order (D-07): closes a phase first, fewest checks, newest
/// first with no age last, then alias, phase, lane and file, so the list does not
/// depend on the order of the input.
pub fn next_actions<'a>(
    projects: &[(&'a str, &'a [TriageEntry])],
    limit: usize,
) -> Vec<NextAction<'a>> {
    let mut candidates: Vec<NextAction<'a>> = projects
        .iter()
        .flat_map(|(alias, entries)| {
            entries
                .iter()
                .filter(|entry| {
                    !entry.folded
                        && is_owner_row(&entry.group)
                        && matches!(entry.group.lane, Lane::Test | Lane::Decide | Lane::Fix)
                        && entry.group.command.is_some()
                })
                .map(move |entry| NextAction { alias, entry })
        })
        .collect();
    candidates.sort_by(rank);
    candidates.truncate(limit);
    candidates
}

/// Owner work per lane, in items.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneCounts {
    pub test: usize,
    pub decide: usize,
    pub fix: usize,
    pub blocked: usize,
}

impl LaneCounts {
    /// Whether no lane holds work.
    pub fn is_empty(&self) -> bool {
        self.test + self.decide + self.fix + self.blocked == 0
    }
}

/// The owner work the project's gate (`closes after`) does not count: rows of a
/// workstream, debug sessions, handoffs and phases outside `phases_open`. Counted
/// with the gate's own per-group rule.
pub fn outside_gate(digest: &OwnerDigest) -> LaneCounts {
    let mut counts = LaneCounts::default();
    for group in digest.groups.iter().filter(|g| is_owner_row(g)) {
        let counted = group.workstream.is_none()
            && !matches!(group.kind, Kind::Debug | Kind::Handoff)
            && digest
                .closes_after
                .phases_open
                .iter()
                .any(|open| same_phase(&group.phase, open));
        if counted {
            continue;
        }
        let items = gate::lane_items(group);
        match group.lane {
            Lane::Test => counts.test += items,
            Lane::Decide => counts.decide += items,
            Lane::Fix => counts.fix += items,
            Lane::Blocked => counts.blocked += items,
            Lane::Deferred => {}
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{
        Category, ClosesAfter, GroupCounts, OwnerItem, RunCommand, RunVerb,
    };

    fn item(name: &str) -> OwnerItem {
        OwnerItem {
            id: format!("x#{name}"),
            test: None,
            name: name.to_owned(),
            expected: None,
            result: "pending".to_owned(),
            category: Category::Pending,
            reason: None,
            blocked_by: None,
            gaps: Vec::new(),
            why_human: None,
        }
    }

    fn group(lane: Lane, kind: Kind, ws: Option<&str>, phase: &str, checks: usize) -> OwnerGroup {
        let file = format!(
            "{}{phase}-{}.md",
            ws.map(|w| format!("{w}/")).unwrap_or_default(),
            kind.as_str()
        );
        OwnerGroup {
            lane,
            kind,
            scope: Scope::Current,
            workstream: ws.map(str::to_owned),
            phase: phase.to_owned(),
            phase_name: "Name".to_owned(),
            file,
            file_status: "testing".to_owned(),
            closes_phase: false,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: Some(10),
            stale: false,
            run: Some(RunCommand {
                verb: RunVerb::VerifyWork,
                phase: Some(phase.to_owned()),
                workstream: ws.map(str::to_owned),
            }),
            command: Some(format!("/gsd-verify-work {phase}")),
            supersedes: Vec::new(),
            counts: GroupCounts {
                checks,
                ..Default::default()
            },
            items: (0..checks).map(|n| item(&format!("c{n}"))).collect(),
        }
    }

    fn digest(groups: Vec<OwnerGroup>) -> OwnerDigest {
        OwnerDigest {
            groups,
            ..Default::default()
        }
    }

    fn aged(mut g: OwnerGroup, age: Option<i64>) -> OwnerGroup {
        g.age_days = age;
        g
    }

    #[test]
    fn a_uat_and_a_verification_group_of_one_phase_merge() {
        let mut verification = aged(
            group(Lane::Test, Kind::Verification, Some("market"), "43", 1),
            Some(4),
        );
        verification.stale = true;
        let uat = aged(
            group(Lane::Test, Kind::Uat, Some("market"), "43", 2),
            Some(10),
        );
        let entries = triage_entries(&digest(vec![uat, verification]), 90);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.group.counts.checks, 3);
        assert_eq!(entry.parts.len(), 2);
        assert_eq!(entry.indices, [0, 1]);
        assert_eq!(entry.group.age_days, Some(4));
        assert!(entry.group.stale, "the stale flag of the newest part");
        assert!(entry.is_merged());
        assert_eq!(entry.files().len(), 2);
        assert_eq!(entry.group.items.len(), 3);
    }

    #[test]
    fn padded_and_plain_spellings_of_a_phase_merge() {
        let a = group(Lane::Test, Kind::Uat, None, "7.1", 1);
        let b = group(Lane::Test, Kind::Verification, None, "07.1", 1);
        assert_eq!(triage_entries(&digest(vec![a, b]), 90).len(), 1);
    }

    #[test]
    fn lane_workstream_and_kind_keep_entries_apart() {
        let test = group(Lane::Test, Kind::Uat, None, "43", 1);
        let fix = group(Lane::Fix, Kind::Verification, None, "43", 1);
        let other_ws = group(Lane::Test, Kind::Uat, Some("a"), "43", 1);
        let debug_a = group(Lane::Fix, Kind::Debug, None, "", 1);
        let debug_b = group(Lane::Fix, Kind::Debug, None, "", 1);
        let mut archived = group(Lane::Test, Kind::Uat, None, "43", 1);
        archived.scope = Scope::Archived {
            label: "v1".into(),
            rank: 0,
        };
        let mut ack = group(Lane::Test, Kind::Uat, None, "43", 1);
        ack.acknowledged = true;
        let d = digest(vec![test, fix, other_ws, debug_a, debug_b, archived, ack]);
        let entries = triage_entries(&d, 90);
        assert_eq!(entries.len(), 7);
        let mut seen: Vec<usize> = entries.iter().flat_map(|e| e.indices.clone()).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..7).collect::<Vec<_>>());
    }

    #[test]
    fn a_merge_takes_the_first_command_and_closes_only_when_every_part_does() {
        let mut a = group(Lane::Test, Kind::Uat, None, "43", 1);
        a.command = None;
        a.run = None;
        a.counts.unreadable = 1;
        a.closes_phase = true;
        let mut b = group(Lane::Test, Kind::Verification, None, "43", 1);
        b.counts.unreadable = 2;
        let entries = triage_entries(&digest(vec![a.clone(), b.clone()]), 90);
        let entry = &entries[0];
        assert_eq!(entry.group.command.as_deref(), Some("/gsd-verify-work 43"));
        assert!(entry.group.run.is_some());
        assert!(!entry.group.closes_phase);
        assert_eq!(entry.group.counts.unreadable, 3);
        b.closes_phase = true;
        let entries = triage_entries(&digest(vec![a, b]), 90);
        assert!(entries[0].group.closes_phase);
    }

    #[test]
    fn the_fold_uses_the_newest_age_and_spares_the_boundary() {
        let at = |age: Option<i64>| {
            triage_entries(
                &digest(vec![aged(group(Lane::Test, Kind::Uat, None, "1", 1), age)]),
                90,
            )[0]
            .folded
        };
        assert!(!at(Some(90)));
        assert!(at(Some(91)));
        assert!(!at(None));
        let merged = triage_entries(
            &digest(vec![
                aged(group(Lane::Test, Kind::Uat, None, "1", 1), Some(120)),
                aged(group(Lane::Test, Kind::Verification, None, "1", 1), Some(3)),
            ]),
            90,
        );
        assert!(!merged[0].folded, "one fresh part keeps the entry");
        let mut old = aged(group(Lane::Test, Kind::Uat, None, "1", 1), Some(300));
        old.scope = Scope::Archived {
            label: "v1".into(),
            rank: 0,
        };
        assert!(!triage_entries(&digest(vec![old]), 90)[0].folded);
    }

    #[test]
    fn stale_totals_count_folded_entries_and_flag_non_checks() {
        let d = digest(vec![
            aged(group(Lane::Test, Kind::Uat, None, "1", 2), Some(200)),
            aged(
                group(Lane::Fix, Kind::Verification, None, "2", 1),
                Some(200),
            ),
            aged(group(Lane::Test, Kind::Uat, None, "3", 5), Some(1)),
        ]);
        let entries = triage_entries(&d, 90);
        let totals = stale_totals(&entries);
        assert_eq!(
            totals,
            StaleTotals {
                entries: 2,
                items: 3,
                non_check: false
            }
        );
        let d = digest(vec![aged(
            group(Lane::Decide, Kind::ContextQuestions, None, "1", 1),
            Some(200),
        )]);
        assert!(stale_totals(&triage_entries(&d, 90)).non_check);
    }

    fn actions(projects: &[(&str, Vec<OwnerGroup>)], limit: usize) -> Vec<(String, String)> {
        let held: Vec<(&str, Vec<TriageEntry>)> = projects
            .iter()
            .map(|(alias, groups)| (*alias, triage_entries(&digest(groups.clone()), 90)))
            .collect();
        let view: Vec<(&str, &[TriageEntry])> =
            held.iter().map(|(a, e)| (*a, e.as_slice())).collect();
        next_actions(&view, limit)
            .into_iter()
            .map(|n| (n.alias.to_owned(), n.entry.group.phase.clone()))
            .collect()
    }

    fn closing(mut g: OwnerGroup) -> OwnerGroup {
        g.closes_phase = true;
        g
    }

    #[test]
    fn a_closing_entry_ranks_before_a_smaller_one_that_does_not_close() {
        let groups = vec![
            group(Lane::Test, Kind::Uat, None, "1", 1),
            closing(group(Lane::Test, Kind::Uat, None, "2", 4)),
            closing(group(Lane::Test, Kind::Uat, None, "3", 1)),
        ];
        let got = actions(&[("a", groups)], 5);
        let phases: Vec<&str> = got.iter().map(|(_, p)| p.as_str()).collect();
        assert_eq!(phases, ["3", "2", "1"]);
    }

    #[test]
    fn equal_checks_rank_newest_first_with_no_age_last() {
        let groups = vec![
            aged(group(Lane::Test, Kind::Uat, None, "1", 2), None),
            aged(group(Lane::Test, Kind::Uat, None, "2", 2), Some(40)),
            aged(group(Lane::Test, Kind::Uat, None, "3", 2), Some(3)),
        ];
        let got = actions(&[("a", groups)], 5);
        let phases: Vec<&str> = got.iter().map(|(_, p)| p.as_str()).collect();
        assert_eq!(phases, ["3", "2", "1"]);
    }

    #[test]
    fn a_bookkeeping_entry_ranks_first_and_unrunnable_ones_are_absent() {
        let mut book = group(Lane::Test, Kind::Uat, None, "9", 0);
        book.bookkeeping_only = true;
        let mut context = group(Lane::Decide, Kind::ContextQuestions, None, "5", 2);
        context.command = None;
        context.run = None;
        let blocked = group(Lane::Blocked, Kind::Uat, None, "6", 1);
        let folded = aged(group(Lane::Test, Kind::Uat, None, "7", 1), Some(500));
        let groups = vec![
            closing(group(Lane::Test, Kind::Uat, None, "1", 1)),
            book,
            context,
            blocked,
            folded,
        ];
        let got = actions(&[("a", groups)], 5);
        let phases: Vec<&str> = got.iter().map(|(_, p)| p.as_str()).collect();
        assert_eq!(phases, ["9", "1"]);
    }

    #[test]
    fn the_limit_keeps_the_first_entries() {
        let groups: Vec<OwnerGroup> = (1..=8)
            .map(|n| group(Lane::Test, Kind::Uat, None, &n.to_string(), n))
            .collect();
        assert_eq!(actions(&[("a", groups)], 5).len(), 5);
    }

    #[test]
    fn the_ranking_does_not_depend_on_the_input_order() {
        let a = vec![
            group(Lane::Test, Kind::Uat, None, "1", 2),
            group(Lane::Fix, Kind::Verification, None, "1", 2),
            closing(group(Lane::Test, Kind::Uat, None, "2", 2)),
        ];
        let b = vec![
            group(Lane::Test, Kind::Uat, None, "1", 2),
            aged(group(Lane::Decide, Kind::Uat, None, "4", 2), None),
        ];
        let base = actions(&[("a", a.clone()), ("b", b.clone())], 5);
        assert_eq!(base.len(), 5);
        let mut a_rev = a.clone();
        a_rev.reverse();
        let mut b_rev = b.clone();
        b_rev.reverse();
        assert_eq!(actions(&[("b", b_rev), ("a", a_rev)], 5), base);
        assert_eq!(actions(&[("b", b), ("a", a)], 5), base);
    }

    #[test]
    fn outside_gate_counts_what_the_root_gate_skips() {
        let mut d = digest(vec![
            group(Lane::Test, Kind::Uat, Some("market"), "43", 2),
            group(Lane::Decide, Kind::Debug, None, "", 1),
            group(Lane::Test, Kind::Uat, None, "1", 5),
        ]);
        d.closes_after = ClosesAfter {
            phases_open: vec!["1".to_owned()],
            ..Default::default()
        };
        let counts = outside_gate(&d);
        assert_eq!(
            counts,
            LaneCounts {
                test: 2,
                decide: 1,
                fix: 0,
                blocked: 0
            }
        );
        assert!(!counts.is_empty());
        d.groups.clear();
        assert!(outside_gate(&d).is_empty());
    }
}
