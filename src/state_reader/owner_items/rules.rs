//! Layer B: the owner's rules over Layer A's files (Phase 34, D-11, D-13, D-14).
//!
//! Rules: R1 (lanes, UAT and VERIFICATION), R2 (supersede), R3 (gap merge), R4
//! (acknowledgment), R5 (archived counts), R6 (deferred), R7 (closeable phases and
//! bookkeeping-only groups), R9 (age), R10 (commands), the D-20 sort and the counts.
//! These rules are the owner's own and are never compared with the `audit-uat` oracle.
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
    AuditFile, AuditItem, AuditKind, Category, GroupCounts, Kind, Lane, Origin, OwnerDigest,
    OwnerGap, OwnerGroup, OwnerItem, Scope,
};
use crate::state_reader::disk_status::{DiskStatus, VerificationStatus};
use crate::state_reader::phase_num::{same_phase, PhaseNum};
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

/// One item on its way into a group, with the gaps R3 merged into it.
struct Entry<'a> {
    item: &'a AuditItem,
    id: String,
    gaps: Vec<OwnerGap>,
}

/// The basename of a POSIX relative path.
fn basename(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

fn same_phase_token(a: &str, b: &str) -> bool {
    a == b || same_phase(a, b)
}

/// R2: which files are superseded, and which VERIFICATION names each UAT supersedes.
///
/// A human_needed VERIFICATION is superseded when a UAT of the same scope,
/// workstream and phase lists its exact file name in `source` (D-11 D3 strict).
/// gaps_found, unreadable and every other status are never superseded.
fn supersession(files: &[AuditFile]) -> (Vec<bool>, Vec<Vec<String>>) {
    let mut superseded = vec![false; files.len()];
    let mut supersedes: Vec<Vec<String>> = vec![Vec::new(); files.len()];
    for (vi, verification) in files.iter().enumerate() {
        if verification.kind != AuditKind::Verification
            || verification.unreadable.is_some()
            || verification.status != "human_needed"
        {
            continue;
        }
        let name = basename(&verification.rel_path);
        for (ui, uat) in files.iter().enumerate() {
            if uat.kind == AuditKind::Uat
                && uat.scope == verification.scope
                && uat.workstream == verification.workstream
                && same_phase_token(&uat.phase, &verification.phase)
                && uat.source.iter().any(|entry| entry == name)
            {
                superseded[vi] = true;
                if !supersedes[ui].iter().any(|have| have == name) {
                    supersedes[ui].push(name.to_owned());
                }
            }
        }
    }
    (superseded, supersedes)
}

/// R3: the entries of a UAT file, with each Gap or GapTable item that names a row
/// with result `issue` moved into that row's `gaps`.
fn uat_entries<'a>(file: &'a AuditFile, ids: &mut Ids) -> Vec<Entry<'a>> {
    let mut entries: Vec<Entry<'a>> = file
        .items
        .iter()
        .map(|item| Entry {
            item,
            id: ids.next(item),
            gaps: Vec::new(),
        })
        .collect();
    let mut merged = vec![false; entries.len()];
    for gap_at in 0..entries.len() {
        let gap = entries[gap_at].item;
        if !matches!(gap.origin, Origin::Gap | Origin::GapTable) {
            continue;
        }
        let Some(test) = gap.test.as_deref() else {
            continue;
        };
        let row_at = entries.iter().position(|entry| {
            entry.item.origin == Origin::Row
                && entry.item.result == "issue"
                && entry.item.test.as_deref() == Some(test)
        });
        if let Some(row_at) = row_at {
            let owner_gap = OwnerGap {
                name: gap.name.clone(),
                result: gap.result.clone(),
                reason: gap.reason.clone(),
            };
            entries[row_at].gaps.push(owner_gap);
            merged[gap_at] = true;
        }
    }
    let mut at = 0;
    entries.retain(|_| {
        at += 1;
        !merged[at - 1]
    });
    entries
}

/// The roadmap name and number of a file's phase.
fn phase_labels(state: &ProjectState, file: &AuditFile) -> (String, String) {
    let roadmap_phase = scope_state(state, file.workstream.as_deref()).roadmap_phase(&file.phase);
    (
        roadmap_phase.map_or_else(
            || slug_after_token(&file.phase_dir, &file.phase),
            |phase| phase.name.clone(),
        ),
        roadmap_phase.map_or_else(|| file.phase.clone(), |phase| phase.number.clone()),
    )
}

fn counts_of(entries: &[Entry<'_>], unreadable: usize) -> GroupCounts {
    GroupCounts {
        checks: entries.len(),
        failed: entries
            .iter()
            .filter(|e| e.item.origin == Origin::Row && is_failed_result(&e.item.result))
            .count(),
        gaps: entries
            .iter()
            .map(|e| {
                e.gaps.len() + usize::from(matches!(e.item.origin, Origin::Gap | Origin::GapTable))
            })
            .sum(),
        unreadable,
    }
}

/// Everything `new_group` needs besides the entries.
struct GroupContext<'a> {
    file: &'a AuditFile,
    kind: Kind,
    labels: &'a (String, String),
    supersedes: &'a [String],
    now: DateTime<Utc>,
    stale_days: u32,
}

fn new_group(
    ctx: &GroupContext<'_>,
    lane: Lane,
    entries: Vec<Entry<'_>>,
    unreadable: usize,
) -> OwnerGroup {
    let file = ctx.file;
    let counts = counts_of(&entries, unreadable);
    let mut items: Vec<OwnerItem> = entries
        .iter()
        .map(|entry| {
            let mut item = owner_item(entry.item, entry.id.clone());
            item.gaps = entry.gaps.clone();
            item
        })
        .collect();
    items.sort_by_key(numeric_test);
    let age_days = age_of(file, ctx.now);
    OwnerGroup {
        lane,
        kind: ctx.kind,
        scope: file.scope.clone(),
        workstream: file.workstream.clone(),
        phase: file.phase.clone(),
        phase_name: ctx.labels.0.clone(),
        file: file.rel_path.clone(),
        file_status: file.status.clone(),
        closes_phase: false,
        bookkeeping_only: false,
        acknowledged: file.acknowledged,
        age_days,
        stale: age_days.is_some_and(|days| days > i64::from(ctx.stale_days)),
        command: command_for(lane, file, &ctx.labels.1, unreadable),
        supersedes: ctx.supersedes.to_vec(),
        counts,
        items,
    }
}

/// The entries of one file by lane (Test, Fix, Blocked, Deferred) and the number of
/// checks the file could not be read for.
fn lane_entries<'a>(file: &'a AuditFile, digest: &mut OwnerDigest) -> ([Vec<Entry<'a>>; 4], usize) {
    let mut lanes: [Vec<Entry<'a>>; 4] = Default::default();
    let slot = |lane: Lane| match lane {
        Lane::Test => 0,
        Lane::Fix => 1,
        Lane::Blocked => 2,
        Lane::Decide | Lane::Deferred => 3,
    };
    let counted = usize::from(!file.acknowledged);
    let mut ids = Ids::new(&file.rel_path);
    let unreadable = if file.unreadable.is_some() {
        1
    } else {
        file.unparsed_blocks
    };
    match file.kind {
        AuditKind::Uat => {
            for entry in uat_entries(file, &mut ids) {
                match place(entry.item, &file.status) {
                    Placement::Lane(lane) => lanes[slot(lane)].push(entry),
                    Placement::Deferred => {
                        digest.hidden.deferred += counted;
                        lanes[3].push(entry);
                    }
                    Placement::Noise => digest.hidden.noise += counted,
                }
            }
        }
        AuditKind::Verification => {
            if file.unreadable.is_none() {
                for item in &file.items {
                    let entry = Entry {
                        item,
                        id: ids.next(item),
                        gaps: Vec::new(),
                    };
                    let test =
                        file.status == "human_needed" && item.origin == Origin::HumanVerification;
                    lanes[if test { 0 } else { 1 }].push(entry);
                }
            }
        }
        AuditKind::Deferred => {
            for item in &file.items {
                digest.hidden.deferred += counted;
                lanes[3].push(Entry {
                    item,
                    id: ids.next(item),
                    gaps: Vec::new(),
                });
            }
        }
    }
    (lanes, unreadable)
}

fn kind_of(kind: AuditKind) -> Kind {
    match kind {
        AuditKind::Uat => Kind::Uat,
        AuditKind::Verification => Kind::Verification,
        AuditKind::Deferred => Kind::Deferred,
    }
}

/// R7 (D-12): closeable phases and bookkeeping-only groups, per current scope.
fn apply_r7(digest: &mut OwnerDigest, files: &[AuditFile], state: &ProjectState, ctx: &Context) {
    let mut scopes: Vec<(Option<&str>, &ProjectState)> = vec![(None, state)];
    scopes.extend(
        state
            .workstreams
            .iter()
            .map(|ws| (Some(ws.name.as_str()), &ws.state)),
    );
    let mut extra: Vec<OwnerGroup> = Vec::new();
    for (workstream, scope) in scopes {
        for phase in scope.phases.iter().filter(|phase| !phase.completed) {
            let in_phase = |group: &OwnerGroup| {
                matches!(group.scope, Scope::Current)
                    && !group.acknowledged
                    && group.workstream.as_deref() == workstream
                    && same_phase_token(&group.phase, &phase.number)
            };
            let phase_files = || {
                files.iter().filter(|file| {
                    matches!(file.scope, Scope::Current)
                        && !file.acknowledged
                        && file.workstream.as_deref() == workstream
                        && same_phase_token(&file.phase, &phase.number)
                })
            };
            let Some(disk) = scope.disk_status_for(&phase.number) else {
                continue;
            };
            let groups: Vec<&OwnerGroup> = digest.groups.iter().filter(|g| in_phase(g)).collect();
            let test_items: usize = groups
                .iter()
                .filter(|g| g.lane == Lane::Test)
                .map(|g| g.items.len())
                .sum();
            let has_fix = groups.iter().any(|g| g.lane == Lane::Fix);
            let has_blocked = groups.iter().any(|g| g.lane == Lane::Blocked);
            if has_fix || has_blocked {
                continue;
            }
            let file_unreadable =
                phase_files().any(|file| file.unparsed_blocks > 0 || file.unreadable.is_some());
            let verification_ok = matches!(
                disk.verification_status,
                VerificationStatus::Passed | VerificationStatus::HumanNeeded
            );
            if test_items > 0 {
                if disk.status >= DiskStatus::Executed && !file_unreadable && verification_ok {
                    for group in digest
                        .groups
                        .iter_mut()
                        .filter(|g| in_phase(g) && g.lane == Lane::Test)
                    {
                        group.closes_phase = true;
                    }
                    digest.closeable_phases += 1;
                }
                continue;
            }
            for file in phase_files().filter(|file| file.kind == AuditKind::Uat) {
                let terminal = matches!(file.status.as_str(), "complete" | "resolved" | "passed");
                if terminal && disk.verification_status != VerificationStatus::HumanNeeded {
                    continue;
                }
                let labels = phase_labels(state, file);
                let group_ctx = GroupContext {
                    file,
                    kind: Kind::Uat,
                    labels: &labels,
                    supersedes: &[],
                    now: ctx.now,
                    stale_days: ctx.stale_days,
                };
                let mut group = new_group(&group_ctx, Lane::Test, Vec::new(), 0);
                group.bookkeeping_only = true;
                extra.push(group);
            }
        }
    }
    digest.groups.extend(extra);
}

/// The caller-supplied inputs the group builders share.
struct Context {
    now: DateTime<Utc>,
    stale_days: u32,
}

/// Build the digest of one project from Layer A's files.
pub(super) fn build_digest(
    files: Vec<AuditFile>,
    state: &ProjectState,
    now: DateTime<Utc>,
    stale_days: u32,
) -> OwnerDigest {
    let ctx = Context { now, stale_days };
    let mut digest = OwnerDigest::default();
    let (superseded, supersedes) = supersession(&files);
    for (at, file) in files.iter().enumerate() {
        // R4: an acknowledged file counts once in `hidden.acknowledged_files` and in no
        // other tally. Its groups are still built, flagged, so a request to include
        // acknowledged work can list them; the default listing and every count skip them.
        if file.acknowledged {
            digest.hidden.acknowledged_files += 1;
        }
        if superseded[at] {
            // R2: the UAT lists these checks; counting them twice is the bug.
            digest.superseded += file.items.len();
            continue;
        }
        let labels = phase_labels(state, file);
        let group_ctx = GroupContext {
            file,
            kind: kind_of(file.kind),
            labels: &labels,
            supersedes: &supersedes[at],
            now,
            stale_days,
        };
        let (lanes, unreadable) = lane_entries(file, &mut digest);
        let order = [Lane::Test, Lane::Fix, Lane::Blocked, Lane::Deferred];
        for (lane, entries) in order.into_iter().zip(lanes) {
            let group_unreadable = if lane == Lane::Fix { unreadable } else { 0 };
            if entries.is_empty() && group_unreadable == 0 {
                continue;
            }
            digest
                .groups
                .push(new_group(&group_ctx, lane, entries, group_unreadable));
        }
    }
    apply_r7(&mut digest, &files, state, &ctx);
    digest.groups.sort_by_key(sort_key);

    let mut fix_phases: BTreeSet<(Option<&str>, &str)> = BTreeSet::new();
    for group in digest.groups.iter().filter(|group| !group.acknowledged) {
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

impl OwnerDigest {
    /// Whether the phase holds an open check in a UAT that is not finished: a current,
    /// unacknowledged UAT group in the Test or Fix lane with at least one item, whose
    /// file status is outside complete, resolved, passed and deferred (OWN-09).
    pub fn has_open_uat_items(&self, workstream: Option<&str>, phase: &str) -> bool {
        self.groups.iter().any(|group| {
            group.kind == Kind::Uat
                && matches!(group.scope, Scope::Current)
                && !group.acknowledged
                && matches!(group.lane, Lane::Test | Lane::Fix)
                && group.workstream.as_deref() == workstream
                && same_phase_token(&group.phase, phase)
                && !group.items.is_empty()
                && !matches!(
                    group.file_status.as_str(),
                    "complete" | "resolved" | "passed" | "deferred"
                )
        })
    }
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
                ("deferred", vec!["12"]),
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
    fn acknowledged_files_are_counted_once_and_flagged() {
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        f.acknowledged = true;
        let d = digest(vec![f]);
        assert_eq!(d.hidden.acknowledged_files, 1);
        assert_eq!(d.groups.len(), 1, "built, flagged, never counted");
        assert!(d.groups[0].acknowledged);
        assert_eq!((d.test, d.blocked, d.fix_phases), (0, 0, 0));
        assert_eq!(d.hidden.deferred + d.hidden.noise, 0);
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

    // ---- Plan 34-04: R2, R3, R5, R6, R7 and VERIFICATION lanes ----

    use crate::state_reader::disk_status::DiskInference;

    fn verification_file(phase: &str, status: &str, items: Vec<AuditItem>) -> AuditFile {
        let mut f = file(items);
        f.rel_path = format!(".planning/phases/{phase}-x/{phase}-VERIFICATION.md");
        f.kind = AuditKind::Verification;
        f.phase = phase.to_owned();
        f.phase_dir = format!("{phase}-x");
        f.status = status.to_owned();
        f
    }

    fn human(test: &str) -> AuditItem {
        AuditItem {
            origin: Origin::HumanVerification,
            ..row(test, "human_needed", Category::HumanUat)
        }
    }

    fn uat_for(phase: &str, source: &[&str], items: Vec<AuditItem>) -> AuditFile {
        let mut f = file(items);
        f.rel_path = format!(".planning/phases/{phase}-x/{phase}-UAT.md");
        f.phase = phase.to_owned();
        f.phase_dir = format!("{phase}-x");
        f.source = source.iter().map(|s| (*s).to_owned()).collect();
        f
    }

    fn gap(test: Option<&str>, origin: Origin) -> AuditItem {
        AuditItem {
            origin,
            test: test.map(str::to_owned),
            name: "a gap".to_owned(),
            result: "failed".to_owned(),
            category: Category::Unknown,
            ..row("", "failed", Category::Unknown)
        }
    }

    #[test]
    fn r2_a_uat_that_lists_a_human_needed_verification_supersedes_it() {
        let v = verification_file(
            "20",
            "human_needed",
            vec![human("1"), human("2"), human("3")],
        );
        let u = uat_for(
            "20",
            &["20-VERIFICATION.md"],
            vec![row("1", "pending", Category::Pending)],
        );
        let d = digest(vec![u, v]);
        assert_eq!(d.superseded, 3);
        assert!(d.groups.iter().all(|g| g.kind == Kind::Uat));
        assert!(d
            .groups
            .iter()
            .all(|g| g.supersedes == ["20-VERIFICATION.md"]));
        assert_eq!(d.test, 1);
    }

    #[test]
    fn r2_adjacency_only_the_exactly_named_same_phase_verification_is_superseded() {
        // a source of another kind, an extra token, a different phase, a lookalike name
        let v = || verification_file("20", "human_needed", vec![human("1")]);
        let d = digest(vec![uat_for("20", &["11-03-PLAN.md"], vec![]), v()]);
        assert_eq!((d.superseded, d.groups.len()), (0, 1));
        let d = digest(vec![
            uat_for("20", &["20-VERIFICATION-old.md"], vec![]),
            v(),
        ]);
        assert_eq!(d.superseded, 0);
        let other_phase = verification_file("21", "human_needed", vec![human("1")]);
        let d = digest(vec![
            uat_for("20", &["21-VERIFICATION.md"], vec![]),
            other_phase,
        ]);
        assert_eq!(d.superseded, 0);
        let mut other_ws = v();
        other_ws.workstream = Some("beta".to_owned());
        let d = digest(vec![
            uat_for("20", &["20-VERIFICATION.md"], vec![]),
            other_ws,
        ]);
        assert_eq!(d.superseded, 0);
        // empty source, and a superseded VERIFICATION with zero items adds nothing
        let d = digest(vec![uat_for("20", &[], vec![]), v()]);
        assert_eq!(d.superseded, 0);
        let empty = verification_file("20", "human_needed", vec![]);
        let d = digest(vec![uat_for("20", &["20-VERIFICATION.md"], vec![]), empty]);
        assert_eq!(d.superseded, 0);
        // gaps_found is never superseded
        let g = verification_file("20", "gaps_found", vec![human("1")]);
        let d = digest(vec![uat_for("20", &["20-VERIFICATION.md"], vec![]), g]);
        assert_eq!(d.superseded, 0);
        assert_eq!(lanes(&d), [("fix", vec!["1"])]);
    }

    #[test]
    fn r2_archived_pairs_count_and_two_test_groups_order_by_file() {
        let scope = Scope::Archived {
            label: "v1".to_owned(),
            rank: 2,
        };
        let mut v = verification_file("20", "human_needed", vec![human("1"), human("2")]);
        v.scope = scope.clone();
        let mut u = uat_for("20", &["20-VERIFICATION.md"], vec![]);
        u.scope = scope;
        assert_eq!(digest(vec![u, v]).superseded, 2);

        let v = verification_file("20", "human_needed", vec![human("1")]);
        let u = uat_for("20", &[], vec![row("1", "pending", Category::Pending)]);
        let d = digest(vec![v, u]);
        let files: Vec<&str> = d.groups.iter().map(|g| g.file.as_str()).collect();
        assert_eq!(
            files,
            [
                ".planning/phases/20-x/20-UAT.md",
                ".planning/phases/20-x/20-VERIFICATION.md"
            ]
        );
    }

    #[test]
    fn verification_lanes_follow_the_status() {
        let d = digest(vec![verification_file(
            "20",
            "human_needed",
            vec![human("1")],
        )]);
        assert_eq!(lanes(&d), [("test", vec!["1"])]);
        assert_eq!(d.groups[0].kind, Kind::Verification);
        let d = digest(vec![verification_file(
            "20",
            "gaps_found",
            vec![human("1"), human("2")],
        )]);
        assert_eq!(lanes(&d), [("fix", vec!["1", "2"])]);
        let mut bad = verification_file("20", "bogus", vec![]);
        bad.unreadable = Some(Unreadable::OutOfSetStatus("bogus".to_owned()));
        let d = digest(vec![bad]);
        let g = &d.groups[0];
        assert_eq!(
            (g.lane, g.items.len(), g.counts.unreadable, &g.command),
            (Lane::Fix, 0, 1, &None)
        );
    }

    #[test]
    fn r3_gaps_merge_into_issue_rows_and_the_rest_stay_standalone() {
        let mut rows: Vec<AuditItem> = (1..=3)
            .map(|n| row(&n.to_string(), "issue", Category::Issue))
            .collect();
        rows.push(row("4", "pending", Category::Pending));
        let mut items = rows;
        items.push(gap(Some("1"), Origin::Gap));
        items.push(gap(Some("1"), Origin::GapTable));
        items.push(gap(Some("3"), Origin::Gap));
        items.push(gap(Some("4"), Origin::Gap)); // a pending row: standalone
        items.push(gap(None, Origin::Gap)); // no test: standalone
        items.push(gap(Some("9"), Origin::Gap)); // no row: standalone
        let d = digest(vec![file(items)]);
        let fix = d.groups.iter().find(|g| g.lane == Lane::Fix).unwrap();
        let tests: Vec<&str> = fix
            .items
            .iter()
            .map(|i| i.test.as_deref().unwrap_or("-"))
            .collect();
        assert_eq!(tests, ["1", "2", "3", "4", "9", "-"]);
        assert_eq!(fix.items[0].gaps.len(), 2);
        assert_eq!(fix.items[1].gaps.len(), 0);
        assert_eq!(fix.items[2].gaps.len(), 1);
        assert_eq!(
            (fix.counts.checks, fix.counts.failed, fix.counts.gaps),
            (6, 3, 6)
        );
    }

    #[test]
    fn r3_ten_issues_and_fourteen_gaps_make_one_group_of_ten() {
        let mut items: Vec<AuditItem> = (1..=10)
            .map(|n| row(&n.to_string(), "issue", Category::Issue))
            .collect();
        for n in 1..=14 {
            items.push(gap(Some(&((n % 10) + 1).to_string()), Origin::Gap));
        }
        let d = digest(vec![file(items)]);
        assert_eq!(d.groups.len(), 1);
        let c = d.groups[0].counts;
        assert_eq!((c.checks, c.failed, c.gaps, c.unreadable), (10, 10, 14, 0));
        assert_eq!(d.fix_phases, 1);
    }

    #[test]
    fn r3_without_gaps_the_gap_count_is_zero_and_gap_only_groups_have_no_failures() {
        let d = digest(vec![file(vec![row("1", "issue", Category::Issue)])]);
        assert_eq!(d.groups[0].counts.gaps, 0);
        let d = digest(vec![file(vec![gap(None, Origin::Gap)])]);
        let c = d.groups[0].counts;
        assert_eq!((c.failed, c.gaps), (0, 1));
    }

    #[test]
    fn r5_and_r6_archived_and_deferred_work_stays_out_of_current_counts() {
        let mut a = file(vec![
            row("1", "skipped", Category::SkippedUnresolved),
            row("2", "skipped", Category::DeviceNeeded),
            row("3", "issue", Category::Issue),
            row("4", "issue", Category::Issue),
        ]);
        a.scope = Scope::Archived {
            label: "v1".to_owned(),
            rank: 1,
        };
        let current = file(vec![row("1", "pending", Category::Pending)]);
        let d = digest(vec![a, current]);
        assert_eq!(
            (d.test, d.blocked, d.fix_phases, d.closeable_phases),
            (1, 0, 0, 0)
        );
        assert_eq!(
            (d.archived.test, d.archived.blocked, d.archived.fix),
            (1, 1, 2)
        );
        assert!(d.groups[0].scope == Scope::Current);
        assert!(d.groups[1..]
            .iter()
            .all(|g| g.command.is_none() && g.scope != Scope::Current));

        let mut def = file(
            (0..200)
                .map(|n| AuditItem {
                    origin: Origin::Deferred,
                    ..row(&n.to_string(), "unresolved", Category::Deferred)
                })
                .collect(),
        );
        def.kind = AuditKind::Deferred;
        def.status = "unresolved".to_owned();
        def.rel_path = ".planning/phases/01-alpha/deferred-items.md".to_owned();
        let d = digest(vec![def]);
        assert_eq!(d.hidden.deferred, 200);
        assert!(d.groups.iter().all(|g| g.lane == Lane::Deferred));
        assert_eq!((d.test, d.blocked, d.fix_phases), (0, 0, 0));
    }

    /// One row of the OWN-07 table.
    struct R7Case {
        name: &'static str,
        items: Vec<AuditItem>,
        disk: DiskStatus,
        verification: VerificationStatus,
        unparsed: usize,
        closeable: bool,
        bookkeeping: bool,
    }

    #[test]
    fn r7_closeable_table() {
        let t = || row("1", "pending", Category::Pending);
        let cases = vec![
            R7Case {
                name: "fix present",
                items: vec![t(), row("2", "issue", Category::Issue)],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::Passed,
                unparsed: 0,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "blocked present",
                items: vec![t(), row("2", "skipped", Category::DeviceNeeded)],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::Passed,
                unparsed: 0,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "gaps_found",
                items: vec![t()],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::GapsFound,
                unparsed: 0,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "stale",
                items: vec![t()],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::Stale,
                unparsed: 0,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "missing summary",
                items: vec![t()],
                disk: DiskStatus::Partial,
                verification: VerificationStatus::Passed,
                unparsed: 0,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "parse gap",
                items: vec![t()],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::Passed,
                unparsed: 1,
                closeable: false,
                bookkeeping: false,
            },
            R7Case {
                name: "zero open",
                items: vec![],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::Passed,
                unparsed: 0,
                closeable: false,
                bookkeeping: true,
            },
            R7Case {
                name: "clean",
                items: vec![t()],
                disk: DiskStatus::Executed,
                verification: VerificationStatus::HumanNeeded,
                unparsed: 0,
                closeable: true,
                bookkeeping: false,
            },
        ];
        for case in cases {
            let mut st = state();
            st.phase_disk_statuses.insert(
                "01".to_owned(),
                DiskInference {
                    status: case.disk,
                    verification_status: case.verification,
                    ..Default::default()
                },
            );
            let mut f = file(case.items);
            f.unparsed_blocks = case.unparsed;
            let d = build_digest(vec![f], &st, now(), 30);
            assert_eq!(
                d.closeable_phases,
                usize::from(case.closeable),
                "{}",
                case.name
            );
            let test_groups: Vec<&OwnerGroup> =
                d.groups.iter().filter(|g| g.lane == Lane::Test).collect();
            assert_eq!(
                test_groups.iter().any(|g| g.closes_phase),
                case.closeable,
                "{}",
                case.name
            );
            let book: Vec<&&OwnerGroup> =
                test_groups.iter().filter(|g| g.bookkeeping_only).collect();
            assert_eq!(book.len(), usize::from(case.bookkeeping), "{}", case.name);
            if case.bookkeeping {
                assert!(book[0].items.is_empty());
                assert_eq!(book[0].command.as_deref(), Some("/gsd-verify-work 01"));
                assert!(!book[0].closes_phase);
            }
        }
    }

    #[test]
    fn r7_ignores_completed_and_unknown_phases_and_acknowledged_files() {
        let mut st = state();
        st.phase_disk_statuses.insert(
            "01".to_owned(),
            DiskInference {
                status: DiskStatus::Executed,
                verification_status: VerificationStatus::Passed,
                ..Default::default()
            },
        );
        let mut done = st.clone();
        done.phases[0].completed = true;
        let f = file(vec![row("1", "pending", Category::Pending)]);
        assert_eq!(
            build_digest(vec![f.clone()], &done, now(), 30).closeable_phases,
            0
        );
        assert_eq!(
            build_digest(vec![f.clone()], &st, now(), 30).closeable_phases,
            1
        );
        let mut acked = f;
        acked.acknowledged = true;
        assert_eq!(
            build_digest(vec![acked], &st, now(), 30).closeable_phases,
            0
        );
    }

    #[test]
    fn has_open_uat_items_follows_the_owner_predicate() {
        let open = |f: AuditFile| digest(vec![f]).has_open_uat_items(None, "01");
        assert!(open(file(vec![row("1", "pending", Category::Pending)])));
        assert!(open(file(vec![row("1", "issue", Category::Issue)])));
        let mut f = file(vec![row("1", "pending", Category::Pending)]);
        for done in ["complete", "resolved", "passed", "deferred"] {
            f.status = done.to_owned();
            assert!(!open(f.clone()), "{done}");
        }
        f.status = "draft".to_owned();
        assert!(open(f.clone()));
        f.acknowledged = true;
        assert!(!open(f.clone()));
        f.acknowledged = false;
        f.scope = Scope::Archived {
            label: "v1".to_owned(),
            rank: 0,
        };
        assert!(!open(f));
        assert!(!open(file(vec![row(
            "1",
            "skipped",
            Category::DeviceNeeded
        )])));
        let mut unreadable = file(Vec::new());
        unreadable.unreadable = Some(Unreadable::TooLarge);
        assert!(!open(unreadable));
        let d = digest(vec![file(vec![row("1", "pending", Category::Pending)])]);
        assert!(!d.has_open_uat_items(Some("other"), "01"));
        assert!(d.has_open_uat_items(None, "1"));
    }
}
