//! Layer B: the owner's rules over Layer A's files (Phase 34, D-11, D-13, D-14).
//!
//! This plan implements R1 (lanes), R9 (age), R10 (commands), the D-20 sort and the
//! counts. R4 (acknowledgment) arrives with plan 34-06; R2, R3, R5, R6 and R7 with
//! plan 34-04. These rules are the owner's own and are never compared with the
//! `audit-uat` oracle.
//!
//! - **R1 lanes.** `pending`, `human_uat` and `skipped_unresolved` are checks to
//!   run (`test`); the blocked categories wait on something (`blocked`); `issue`,
//!   a `failed` or `partial` result, and anything unreadable need planning (`fix`).
//!   A `deferred` category is only counted. An `unknown` result is noise in a file
//!   that declares itself complete, resolved or passed, and a fix otherwise
//!   [inferred: the fail-safe direction].
//! - **R9 age.** Whole days between `updated` (else the file's modification time)
//!   and the caller's `now`, floored at zero; stale past `stale_days`.
//! - **R10 command.** Built only from a phase token of the shape `^\d+[A-Za-z]?(\.\d+)*$`
//!   and a workstream name of `^[A-Za-z0-9._-]+$`; anything else gives no command,
//!   so a hostile directory name can never reach a command line.

use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};

use super::{
    AuditFile, AuditItem, AuditKind, Category, GroupCounts, Kind, Lane, OwnerDigest, OwnerGroup,
    OwnerItem, Scope,
};
use crate::state_reader::phase_num::PhaseNum;
use crate::state_reader::ProjectState;

/// Where an item is listed, or that it is only counted.
enum Placement {
    Lane(Lane),
    Deferred,
    Noise,
}

fn is_failed_result(result: &str) -> bool {
    matches!(result, "issue" | "failed" | "partial")
}

/// R1: the placement of one item of a file whose status is `file_status`.
fn place(item: &AuditItem, file_status: &str) -> Placement {
    match item.category {
        Category::Pending | Category::HumanUat | Category::SkippedUnresolved => {
            Placement::Lane(Lane::Test)
        }
        Category::DeviceNeeded
        | Category::BuildNeeded
        | Category::ThirdParty
        | Category::ServerBlocked
        | Category::Blocked => Placement::Lane(Lane::Blocked),
        Category::Issue => Placement::Lane(Lane::Fix),
        Category::Deferred => Placement::Deferred,
        Category::Unknown => {
            if is_failed_result(&item.result) {
                Placement::Lane(Lane::Fix)
            } else if matches!(file_status, "complete" | "resolved" | "passed") {
                Placement::Noise
            } else {
                Placement::Lane(Lane::Fix)
            }
        }
    }
}

/// `^\d+[A-Za-z]?(\.\d+)*$`.
fn is_command_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    let mut at = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if at == 0 {
        return false;
    }
    if bytes.get(at).is_some_and(u8::is_ascii_alphabetic) {
        at += 1;
    }
    while at < bytes.len() {
        if bytes[at] != b'.' {
            return false;
        }
        let digits = bytes[at + 1..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digits == 0 {
            return false;
        }
        at += 1 + digits;
    }
    true
}

/// `^[A-Za-z0-9._-]+$`: the identity alphabet, spelled once in `text`.
fn is_workstream_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(crate::text::is_identity_char)
}

/// The project state a file's phase is looked up in: its workstream's, else the
/// project's own.
fn scope_state<'a>(state: &'a ProjectState, workstream: Option<&str>) -> &'a ProjectState {
    workstream
        .and_then(|name| state.workstreams.iter().find(|ws| ws.name == name))
        .map_or(state, |ws| &ws.state)
}

/// R10: the command that works on a group, if any.
fn command_for(
    lane: Lane,
    file: &AuditFile,
    phase_number: &str,
    unreadable: usize,
) -> Option<String> {
    if !matches!(file.scope, Scope::Current) || unreadable > 0 {
        return None;
    }
    if !is_command_token(phase_number) {
        return None;
    }
    let ws = match file.workstream.as_deref() {
        Some(name) if is_workstream_name(name) => format!(" --ws {name}"),
        Some(_) => return None,
        None => String::new(),
    };
    match lane {
        Lane::Test => Some(format!("/gsd-verify-work {phase_number}{ws}")),
        Lane::Fix => Some(format!("/gsd-plan-phase {phase_number} --gaps{ws}")),
        Lane::Decide | Lane::Blocked | Lane::Deferred => None,
    }
}

/// Item ids: `<file basename>#<test>` for numbered rows, `#g<k>` for gaps, `#v<k>`
/// for unnumbered verification entries and `#d<k>` for deferred entries (k is
/// 1-based within the file and origin); a repeat within one file gets `~2`, `~3`.
struct Ids {
    basename: String,
    per_origin: HashMap<&'static str, usize>,
    seen: HashMap<String, usize>,
}

impl Ids {
    fn new(rel_path: &str) -> Self {
        Ids {
            basename: rel_path.rsplit('/').next().unwrap_or(rel_path).to_owned(),
            per_origin: HashMap::new(),
            seen: HashMap::new(),
        }
    }

    fn next(&mut self, item: &AuditItem) -> String {
        use super::Origin;
        let (tag, numbered) = match item.origin {
            Origin::Row | Origin::HumanVerification | Origin::VerificationGap
                if item.test.is_some() =>
            {
                ("", true)
            }
            Origin::Row | Origin::HumanVerification | Origin::VerificationGap => ("v", false),
            Origin::Gap | Origin::GapTable => ("g", false),
            Origin::Deferred => ("d", false),
        };
        let base = if numbered {
            format!("{}#{}", self.basename, item.test.as_deref().unwrap_or(""))
        } else {
            let count = self.per_origin.entry(tag).or_insert(0);
            *count += 1;
            format!("{}#{tag}{count}", self.basename)
        };
        let times = self.seen.entry(base.clone()).or_insert(0);
        *times += 1;
        if *times == 1 {
            base
        } else {
            format!("{base}~{times}")
        }
    }
}

fn owner_item(item: &AuditItem, id: String) -> OwnerItem {
    OwnerItem {
        id,
        test: item.test.clone(),
        name: item.name.clone(),
        expected: item.expected.clone(),
        result: item.result.clone(),
        category: item.category,
        reason: item.reason.clone(),
        blocked_by: item.blocked_by.clone(),
        gaps: Vec::new(),
    }
}

/// The directory name after its token: `alpha` for `01-alpha`.
fn slug_after_token(phase_dir: &str, phase: &str) -> String {
    phase_dir
        .strip_prefix(phase)
        .unwrap_or(phase_dir)
        .trim_start_matches(['-', '_'])
        .to_owned()
}

fn age_of(file: &AuditFile, now: DateTime<Utc>) -> Option<i64> {
    file.updated
        .or(file.modified)
        .map(|moment| (now - moment).num_days().max(0))
}

/// The numeric test of an item, for the item sort.
fn numeric_test(item: &OwnerItem) -> (bool, u64) {
    match item.test.as_deref().and_then(|t| t.parse::<u64>().ok()) {
        Some(n) => (false, n),
        None => (true, 0),
    }
}

/// The D-20 sort key of a group.
fn sort_key(group: &OwnerGroup) -> (usize, usize, u8, bool, Option<PhaseNum>, String, String) {
    let (scope, rank) = match &group.scope {
        Scope::Current => (0, 0),
        Scope::Archived { rank, .. } => (1, *rank),
    };
    let parsed = PhaseNum::parse(&group.phase);
    (
        scope,
        rank,
        group.lane.order(),
        parsed.is_none(),
        parsed,
        group.phase.clone(),
        group.file.clone(),
    )
}

/// Build the digest of one project from Layer A's files.
pub(super) fn build_digest(
    files: Vec<AuditFile>,
    state: &ProjectState,
    now: DateTime<Utc>,
    stale_days: u32,
) -> OwnerDigest {
    let mut digest = OwnerDigest::default();
    for file in &files {
        if file.acknowledged {
            digest.hidden.acknowledged_files += 1;
            continue;
        }
        if file.kind != AuditKind::Uat {
            continue;
        }
        let mut ids = Ids::new(&file.rel_path);
        let mut by_lane: Vec<(Lane, Vec<(&AuditItem, String)>)> =
            [Lane::Test, Lane::Fix, Lane::Blocked]
                .into_iter()
                .map(|lane| (lane, Vec::new()))
                .collect();
        for item in &file.items {
            let id = ids.next(item);
            match place(item, &file.status) {
                Placement::Lane(lane) => {
                    if let Some((_, bucket)) = by_lane.iter_mut().find(|(l, _)| *l == lane) {
                        bucket.push((item, id));
                    }
                }
                Placement::Deferred => digest.hidden.deferred += 1,
                Placement::Noise => digest.hidden.noise += 1,
            }
        }
        let unreadable = if file.unreadable.is_some() {
            1
        } else {
            file.unparsed_blocks
        };
        let scope_state = scope_state(state, file.workstream.as_deref());
        let roadmap_phase = scope_state.roadmap_phase(&file.phase);
        let phase_name = roadmap_phase.map_or_else(
            || slug_after_token(&file.phase_dir, &file.phase),
            |phase| phase.name.clone(),
        );
        let phase_number = roadmap_phase.map_or(file.phase.as_str(), |phase| phase.number.as_str());
        for (lane, bucket) in by_lane {
            let group_unreadable = if lane == Lane::Fix { unreadable } else { 0 };
            if bucket.is_empty() && group_unreadable == 0 {
                continue;
            }
            let mut items: Vec<OwnerItem> = bucket
                .iter()
                .map(|(item, id)| owner_item(item, id.clone()))
                .collect();
            items.sort_by_key(numeric_test);
            let counts = GroupCounts {
                checks: items.len(),
                failed: bucket
                    .iter()
                    .filter(|(i, _)| is_failed_result(&i.result))
                    .count(),
                gaps: bucket
                    .iter()
                    .filter(|(i, _)| {
                        matches!(i.origin, super::Origin::Gap | super::Origin::GapTable)
                    })
                    .count(),
                unreadable: group_unreadable,
            };
            let age_days = age_of(file, now);
            digest.groups.push(OwnerGroup {
                lane,
                kind: Kind::Uat,
                scope: file.scope.clone(),
                workstream: file.workstream.clone(),
                phase: file.phase.clone(),
                phase_name: phase_name.clone(),
                file: file.rel_path.clone(),
                file_status: file.status.clone(),
                closes_phase: false,
                bookkeeping_only: false,
                acknowledged: file.acknowledged,
                age_days,
                stale: age_days.is_some_and(|days| days > i64::from(stale_days)),
                command: command_for(lane, file, phase_number, group_unreadable),
                supersedes: Vec::new(),
                counts,
                items,
            });
        }
    }
    digest.groups.sort_by_key(sort_key);

    let mut fix_phases: BTreeSet<(Option<&str>, &str)> = BTreeSet::new();
    for group in &digest.groups {
        match (&group.scope, group.lane) {
            (Scope::Current, Lane::Test) => digest.test += group.items.len(),
            (Scope::Current, Lane::Blocked) => digest.blocked += group.items.len(),
            (Scope::Current, Lane::Fix) => {
                fix_phases.insert((group.workstream.as_deref(), group.phase.as_str()));
            }
            (Scope::Archived { .. }, Lane::Test) => digest.archived.test += group.items.len(),
            (Scope::Archived { .. }, Lane::Blocked) => digest.archived.blocked += group.items.len(),
            (Scope::Archived { .. }, Lane::Fix) => digest.archived.fix += group.items.len(),
            _ => {}
        }
    }
    digest.fix_phases = fix_phases.len();
    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{Origin, Unreadable};
    use crate::state_reader::roadmap_md::RoadmapPhase;

    fn state() -> ProjectState {
        ProjectState {
            phases: vec![RoadmapPhase {
                number: "01".to_owned(),
                name: "Alpha".to_owned(),
                description: String::new(),
                completed: false,
                total_plans: 0,
                completed_plans: 0,
                depends_on: Vec::new(),
            }],
            ..Default::default()
        }
    }

    fn row(test: &str, result: &str, category: Category) -> AuditItem {
        AuditItem {
            origin: Origin::Row,
            test: Some(test.to_owned()),
            name: format!("check {test}"),
            expected: None,
            result: result.to_owned(),
            category,
            reason: None,
            blocked_by: None,
        }
    }

    fn file(items: Vec<AuditItem>) -> AuditFile {
        AuditFile {
            rel_path: ".planning/phases/01-alpha/01-UAT.md".to_owned(),
            kind: AuditKind::Uat,
            scope: Scope::Current,
            workstream: None,
            phase: "01".to_owned(),
            phase_dir: "01-alpha".to_owned(),
            status: "testing".to_owned(),
            source: Vec::new(),
            updated: None,
            modified: None,
            acknowledged: false,
            unparsed_blocks: 0,
            unreadable: None,
            items,
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn digest(files: Vec<AuditFile>) -> OwnerDigest {
        build_digest(files, &state(), now(), 30)
    }

    fn lanes(digest: &OwnerDigest) -> Vec<(&'static str, Vec<&str>)> {
        digest
            .groups
            .iter()
            .map(|g| {
                (
                    g.lane.as_str(),
                    g.items.iter().map(|i| i.test.as_deref().unwrap()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn r1_puts_each_category_in_its_lane() {
        let d = digest(vec![file(vec![
            row("1", "pending", Category::Pending),
            row("2", "human_needed", Category::HumanUat),
            row("3", "skipped", Category::SkippedUnresolved),
            row("4", "skipped", Category::DeviceNeeded),
            row("5", "blocked", Category::Blocked),
            row("6", "blocked", Category::ServerBlocked),
            row("7", "blocked", Category::BuildNeeded),
            row("8", "blocked", Category::ThirdParty),
            row("9", "issue", Category::Issue),
            row("10", "failed", Category::Unknown),
            row("11", "partial", Category::Unknown),
            row("12", "skipped", Category::Deferred),
        ])]);
        assert_eq!(
            lanes(&d),
            [
                ("test", vec!["1", "2", "3"]),
                ("fix", vec!["9", "10", "11"]),
                ("blocked", vec!["4", "5", "6", "7", "8"]),
            ]
        );
        assert_eq!((d.test, d.blocked, d.fix_phases), (3, 5, 1));
        assert_eq!(d.hidden.deferred, 1);
    }

    #[test]
    fn r1_unknown_is_noise_in_a_finished_file_and_a_fix_otherwise() {
        for status in ["complete", "resolved", "passed"] {
            let mut f = file(vec![row("1", "weird", Category::Unknown)]);
            f.status = status.to_owned();
            let d = digest(vec![f]);
            assert!(d.groups.is_empty(), "{status}");
            assert_eq!(d.hidden.noise, 1, "{status}");
        }
        let d = digest(vec![file(vec![row("1", "weird", Category::Unknown)])]);
        assert_eq!(lanes(&d), [("fix", vec!["1"])]);
    }

    #[test]
    fn unparsed_blocks_and_unreadable_files_make_a_fix_group_without_a_command() {
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.unparsed_blocks = 2;
        let d = digest(vec![f]);
        let fix = d.groups.iter().find(|g| g.lane == Lane::Fix).unwrap();
        assert!(fix.items.is_empty());
        assert_eq!(fix.counts.unreadable, 2);
        assert_eq!(fix.command, None);
        assert!(d
            .groups
            .iter()
            .find(|g| g.lane == Lane::Test)
            .unwrap()
            .command
            .is_some());

        let mut f = file(Vec::new());
        f.unreadable = Some(Unreadable::TooLarge);
        let d = digest(vec![f]);
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].counts.unreadable, 1);
    }

    #[test]
    fn r9_age_prefers_updated_then_modified_and_never_goes_negative() {
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.updated = Some(now() - chrono::Duration::days(45) - chrono::Duration::hours(5));
        f.modified = Some(now() - chrono::Duration::days(1));
        let g = &digest(vec![f.clone()]).groups[0].clone();
        assert_eq!((g.age_days, g.stale), (Some(45), true));
        f.updated = None;
        let g = &digest(vec![f.clone()]).groups[0].clone();
        assert_eq!((g.age_days, g.stale), (Some(1), false));
        f.modified = Some(now() + chrono::Duration::days(3));
        assert_eq!(digest(vec![f.clone()]).groups[0].age_days, Some(0));
        f.modified = None;
        let g = &digest(vec![f]).groups[0].clone();
        assert_eq!((g.age_days, g.stale), (None, false));
    }

    #[test]
    fn r9_stale_needs_strictly_more_days_than_the_threshold() {
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.updated = Some(now() - chrono::Duration::days(30));
        assert!(!build_digest(vec![f.clone()], &state(), now(), 30).groups[0].stale);
        f.updated = Some(now() - chrono::Duration::days(31));
        assert!(build_digest(vec![f], &state(), now(), 30).groups[0].stale);
    }

    #[test]
    fn r10_commands_use_the_roadmap_number_and_a_checked_token() {
        let d = digest(vec![file(vec![
            row("1", "pending", Category::Pending),
            row("2", "issue", Category::Issue),
            row("3", "blocked", Category::Blocked),
        ])]);
        let command = |lane| {
            d.groups
                .iter()
                .find(|g| g.lane == lane)
                .unwrap()
                .command
                .clone()
        };
        assert_eq!(command(Lane::Test).as_deref(), Some("/gsd-verify-work 01"));
        assert_eq!(
            command(Lane::Fix).as_deref(),
            Some("/gsd-plan-phase 01 --gaps")
        );
        assert_eq!(command(Lane::Blocked), None);

        let mut hostile = file(vec![row("1", "pending", Category::Pending)]);
        hostile.phase = "x; rm -rf".to_owned();
        assert_eq!(digest(vec![hostile]).groups[0].command, None);

        let mut ws = file(vec![row("1", "pending", Category::Pending)]);
        ws.workstream = Some("alpha-1".to_owned());
        assert_eq!(
            digest(vec![ws]).groups[0].command.as_deref(),
            Some("/gsd-verify-work 01 --ws alpha-1"),
            "an unlisted workstream falls back to the project's roadmap"
        );
        let mut bad_ws = file(vec![row("1", "pending", Category::Pending)]);
        bad_ws.workstream = Some("a b".to_owned());
        assert_eq!(digest(vec![bad_ws]).groups[0].command, None);
    }

    #[test]
    fn archived_groups_have_no_command_and_count_separately() {
        let mut f = file(vec![
            row("1", "pending", Category::Pending),
            row("2", "blocked", Category::Blocked),
            row("3", "issue", Category::Issue),
        ]);
        f.scope = Scope::Archived {
            label: "v1.0".to_owned(),
            rank: 0,
        };
        let d = digest(vec![f]);
        assert!(d.groups.iter().all(|g| g.command.is_none()));
        assert_eq!((d.test, d.blocked, d.fix_phases), (0, 0, 0));
        assert_eq!(
            (d.archived.test, d.archived.blocked, d.archived.fix),
            (1, 1, 1)
        );
    }

    #[test]
    fn ids_follow_the_schema_and_repeats_get_a_suffix() {
        let mut gap = row("", "failed", Category::Unknown);
        gap.origin = Origin::Gap;
        gap.test = None;
        let d = digest(vec![file(vec![
            row("2", "pending", Category::Pending),
            row("2", "pending", Category::Pending),
            row("2", "pending", Category::Pending),
            gap.clone(),
            gap,
        ])]);
        let ids: Vec<&str> = d
            .groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| i.id.as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                "01-UAT.md#2",
                "01-UAT.md#2~2",
                "01-UAT.md#2~3",
                "01-UAT.md#g1",
                "01-UAT.md#g2"
            ]
        );
    }

    #[test]
    fn groups_sort_by_scope_lane_phase_then_file_and_items_by_test() {
        let mut a = file(vec![
            row("10", "pending", Category::Pending),
            row("2", "pending", Category::Pending),
        ]);
        a.phase = "10".to_owned();
        a.rel_path = ".planning/phases/10-x/10-UAT.md".to_owned();
        let mut b = file(vec![row("1", "pending", Category::Pending)]);
        b.phase = "9".to_owned();
        b.rel_path = ".planning/phases/9-x/9-UAT.md".to_owned();
        let mut c = file(vec![row("1", "pending", Category::Pending)]);
        c.phase = "odd".to_owned();
        c.rel_path = ".planning/phases/odd/odd-UAT.md".to_owned();
        let mut old = file(vec![row("1", "pending", Category::Pending)]);
        old.scope = Scope::Archived {
            label: "v1".to_owned(),
            rank: 0,
        };
        let d = digest(vec![c, a, old, b]);
        let order: Vec<&str> = d.groups.iter().map(|g| g.phase.as_str()).collect();
        assert_eq!(order, ["9", "10", "odd", "01"]);
        assert_eq!(
            d.groups[1]
                .items
                .iter()
                .map(|i| i.test.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["2", "10"]
        );
    }

    #[test]
    fn acknowledged_files_are_counted_and_not_listed() {
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.acknowledged = true;
        let d = digest(vec![f]);
        assert!(d.groups.is_empty());
        assert_eq!(d.hidden.acknowledged_files, 1);
    }

    #[test]
    fn the_phase_name_is_the_roadmap_name_else_the_directory_slug() {
        let d = digest(vec![file(vec![row("1", "pending", Category::Pending)])]);
        assert_eq!(d.groups[0].phase_name, "Alpha");
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.phase = "07".to_owned();
        f.phase_dir = "07-gamma-ray".to_owned();
        assert_eq!(digest(vec![f]).groups[0].phase_name, "gamma-ray");
    }

    #[test]
    fn command_tokens_and_workstream_names_are_checked() {
        for ok in ["1", "01", "12A", "7.1", "07.1.2", "3a"] {
            assert!(is_command_token(ok), "{ok}");
        }
        for bad in ["", "a1", "1.", "1..2", "1 2", "1;", "1.a", "1ab"] {
            assert!(!is_command_token(bad), "{bad}");
        }
        assert!(is_workstream_name("ws-1.a_b"));
        assert!(!is_workstream_name(""));
        assert!(!is_workstream_name("a/b"));
    }
}
