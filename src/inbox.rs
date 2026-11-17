//! The `inbox` document: the owner's open items across projects as one
//! schema-versioned JSON value (Phase 34, D-18, D-19).
//!
//! Pure: the entries are built by the caller (`inbox_cli`), so a test builds them by
//! hand. Every key of schema v1 is present on every project, group and item,
//! error entries included; Phase 35 keys that have no data yet carry zero values.
//! No timestamp appears anywhere, so two runs over one tree are byte-identical.
//!
//! Every project-authored string goes through [`terminal_capped`] (escaped for a
//! terminal, then capped). Filesystem paths are emitted raw: they are identifiers a
//! script will `cd` into, and JSON encoding still escapes C0 controls.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::oneoff::todos::PendingTodo;
use crate::state_reader::owner_items::{
    is_owner_row, label_counts, labels_of, next_actions, outside_gate, ranked_checks,
    ranks_as_closing, stale_totals, triage_entries, Category, ClosesAfter, Kind, Lane, LaneCounts,
    OwnerDigest, OwnerGroup, OwnerItem, Scope, StaleTotals, TriageEntry, NEXT_LIMIT,
};
use crate::text::terminal_capped;

/// The `schema` field.
pub const SCHEMA: &str = "gsd-meta-manager/inbox";

/// The `schema_version` field. Bumped on any rename, removal or type change; new
/// keys may be added without a bump.
pub const SCHEMA_VERSION: u64 = 1;

/// Why a project could not be read: the `status` kinds `unresolved`,
/// `folder_missing` and `no_planning`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryError {
    pub kind: &'static str,
    pub message: String,
}

/// One requested project: either read (`error` is `None`) or why not.
#[derive(Debug, Clone, Default)]
pub struct ProjectEntry {
    pub target: Option<String>,
    pub alias: Option<String>,
    pub path: Option<PathBuf>,
    pub error: Option<EntryError>,
    pub milestone_version: Option<String>,
    pub phases_done: u32,
    pub phases_total: u32,
    pub digest: OwnerDigest,
    /// The pending todos to list (empty unless `--include todos`). The count lives in
    /// `digest.hidden.todos`.
    pub todos: Vec<PendingTodo>,
}

/// Which groups are listed. `summary` and `hidden` always report the full counts.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    /// Empty means all four owner lanes.
    pub lanes: Vec<Lane>,
    pub archived: bool,
    pub deferred: bool,
    pub acknowledged: bool,
    pub todos: bool,
}

impl Filters {
    /// Whether `group` is listed: the lane filter applies to the four owner lanes,
    /// and archived, deferred and acknowledged work is listed only on request.
    /// `summary` and `hidden` never depend on this.
    pub fn lists_group(&self, group: &OwnerGroup) -> bool {
        let lane_ok = match group.lane {
            Lane::Deferred => self.deferred,
            lane => self.lanes.is_empty() || self.lanes.contains(&lane),
        };
        lane_ok
            && (self.archived || matches!(group.scope, Scope::Current))
            && (self.acknowledged || !group.acknowledged)
    }
}

fn opt_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| json!(terminal_capped(text)))
}

/// The R8 gate as JSON: four lane totals and the two phase lists, every phase string
/// escaped and capped. Shared by `inbox` and `status`.
pub fn closes_after_json(gate: &ClosesAfter) -> Value {
    let phases = |list: &[String]| {
        list.iter()
            .map(|phase| terminal_capped(phase))
            .collect::<Vec<_>>()
    };
    json!({
        "test": gate.test,
        "decide": gate.decide,
        "fix": gate.fix,
        "blocked": gate.blocked,
        "phases_open": phases(&gate.phases_open),
        "not_built": phases(&gate.not_built),
    })
}

fn item_json(item: &OwnerItem, lane: Lane) -> Value {
    json!({
        "id": terminal_capped(&item.id),
        "test": opt_text(item.test.as_deref()),
        "name": terminal_capped(&item.name),
        "expected": opt_text(item.expected.as_deref()),
        "why_human": opt_text(item.why_human.as_deref()),
        "result": terminal_capped(&item.result),
        "category": item.category.as_str(),
        "reason": opt_text(item.reason.as_deref()),
        "blocked_by": opt_text(item.blocked_by.as_deref()),
        "gaps": item.gaps.iter().map(|gap| json!({
            "name": terminal_capped(&gap.name),
            "result": terminal_capped(&gap.result),
            "reason": opt_text(gap.reason.as_deref()),
        })).collect::<Vec<_>>(),
        "labels": labels_of(item, lane).iter().map(|label| label.as_str()).collect::<Vec<_>>(),
    })
}

/// The basenames of an entry's files, sorted, each escaped and capped.
fn entry_files_json(entry: &TriageEntry) -> Vec<String> {
    let mut names: Vec<String> = entry
        .files()
        .iter()
        .map(|file| terminal_capped(file.rsplit('/').next().unwrap_or(file)))
        .collect();
    names.sort();
    names
}

fn group_json(group: &OwnerGroup, entry: Option<&TriageEntry>) -> Value {
    let (scope, archived_milestone) = match &group.scope {
        Scope::Current => ("current", Value::Null),
        Scope::Archived { label, .. } => ("archived", json!(terminal_capped(label))),
    };
    json!({
        "lane": group.lane.as_str(),
        "kind": group.kind.as_str(),
        "scope": scope,
        "archived_milestone": archived_milestone,
        "workstream": opt_text(group.workstream.as_deref()),
        "phase": terminal_capped(&group.phase),
        "phase_name": terminal_capped(&group.phase_name),
        "file": group.file,
        "file_status": terminal_capped(&group.file_status),
        "closes_phase": group.closes_phase,
        "bookkeeping_only": group.bookkeeping_only,
        "acknowledged": group.acknowledged,
        "age_days": group.age_days,
        "stale": group.stale,
        "command": group.command,
        "folded_stale": entry.is_some_and(|e| e.folded),
        "entry_files": entry.map_or_else(
            || vec![terminal_capped(group.file.rsplit('/').next().unwrap_or(&group.file))],
            entry_files_json,
        ),
        "supersedes": group.supersedes.iter().map(|id| terminal_capped(id)).collect::<Vec<_>>(),
        "counts": {
            "checks": group.counts.checks,
            "failed": group.counts.failed,
            "gaps": group.counts.gaps,
            "unreadable": group.counts.unreadable,
        },
        "items": group.items.iter().map(|item| item_json(item, group.lane)).collect::<Vec<_>>(),
    })
}

fn project_json(entry: &ProjectEntry, filters: &Filters, triage: &[TriageEntry]) -> Value {
    let (error, milestone, groups, hidden, todos) = match &entry.error {
        Some(error) => (
            json!({"kind": error.kind, "message": error.message}),
            Value::Null,
            Vec::new(),
            json!({
                "deferred": 0,
                "acknowledged_files": 0,
                "planned_questions": 0,
                "todos": 0,
                "noise": 0,
                "todos_oldest_days": Value::Null,
                "todos_unreadable": false,
            }),
            Vec::new(),
        ),
        None => (
            Value::Null,
            json!({
                "version": opt_text(entry.milestone_version.as_deref().filter(|v| !v.trim().is_empty())),
                "closes_after": closes_after_json(&entry.digest.closes_after),
            }),
            entry
                .digest
                .groups
                .iter()
                .enumerate()
                .filter(|(_, group)| filters.lists_group(group))
                .map(|(index, group)| {
                    let owner = triage.iter().find(|t| t.indices.contains(&index));
                    group_json(group, owner)
                })
                .collect(),
            json!({
                "deferred": entry.digest.hidden.deferred,
                "acknowledged_files": entry.digest.hidden.acknowledged_files,
                "planned_questions": entry.digest.hidden.planned_questions,
                "todos": entry.digest.hidden.todos,
                "noise": entry.digest.hidden.noise,
                "todos_oldest_days": entry.digest.hidden.todos_oldest_days,
                "todos_unreadable": entry.digest.hidden.todos_unreadable,
            }),
            entry
                .todos
                .iter()
                .map(|todo| {
                    json!({
                        "file": terminal_capped(&todo.file_name),
                        "title": terminal_capped(&todo.title),
                    })
                })
                .collect(),
        ),
    };
    json!({
        "target": opt_text(entry.target.as_deref()),
        "alias": opt_text(entry.alias.as_deref()),
        "path": entry.path.as_ref().map(|path| path.display().to_string()),
        "error": error,
        "milestone": milestone,
        "groups": groups,
        "hidden": hidden,
        "todos": todos,
    })
}

/// The whole document: schema, summary over every entry, then the projects.
pub fn build_document(entries: &[ProjectEntry], filters: &Filters, fold_days: u32) -> Value {
    let triaged: Vec<Vec<TriageEntry>> = entries
        .iter()
        .map(|entry| {
            if entry.error.is_some() {
                Vec::new()
            } else {
                triage_entries(&entry.digest, fold_days)
            }
        })
        .collect();
    let aliases: Vec<String> = entries.iter().map(alias_of).collect();
    let view: Vec<(&str, &[TriageEntry])> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.error.is_none())
        .map(|(i, _)| (aliases[i].as_str(), triaged[i].as_slice()))
        .collect();
    let next: Vec<Value> = next_actions(&view, NEXT_LIMIT)
        .iter()
        .map(|action| {
            let group = &action.entry.group;
            json!({
                "alias": terminal_capped(action.alias),
                "workstream": opt_text(group.workstream.as_deref()),
                "phase": terminal_capped(&group.phase),
                "lane": group.lane.as_str(),
                "checks": ranked_checks(group),
                "closes_phase": ranks_as_closing(group),
                "command": group.command,
                "age_days": group.age_days,
            })
        })
        .collect();
    let stale = stale_totals(triaged.iter().flatten());
    let read = || entries.iter().filter(|entry| entry.error.is_none());
    let sum =
        |count: fn(&OwnerDigest) -> usize| read().map(|entry| count(&entry.digest)).sum::<usize>();
    json!({
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "summary": {
            "projects": entries.len(),
            "test": sum(|d| d.test),
            "decide": sum(|d| d.decide),
            "fix_phases": sum(|d| d.fix_phases),
            "fix_debug": sum(|d| d.fix_debug),
            "blocked": sum(|d| d.blocked),
            "closeable_phases": sum(|d| d.closeable_phases),
            "archived": {
                "test": sum(|d| d.archived.test),
                "blocked": sum(|d| d.archived.blocked),
                "fix": sum(|d| d.archived.fix),
                "decide": sum(|d| d.archived.decide),
            },
            "deferred": sum(|d| d.hidden.deferred),
            "acknowledged_files": sum(|d| d.hidden.acknowledged_files),
            "planned_questions": sum(|d| d.hidden.planned_questions),
            "superseded": sum(|d| d.superseded),
            "noise": sum(|d| d.hidden.noise),
            "next_actions": next,
            "stale_folded": {"entries": stale.entries, "items": stale.items},
        },
        "projects": entries
            .iter()
            .zip(&triaged)
            .map(|(entry, triage)| project_json(entry, filters, triage))
            .collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// Plain checklist (D-21)
// ---------------------------------------------------------------------------

/// A phase number as a sort key: numeric first, then the raw text.
pub(crate) fn phase_key(phase: &str) -> (u64, String) {
    let digits: String = phase.chars().take_while(char::is_ascii_digit).collect();
    (digits.parse().unwrap_or(u64::MAX), phase.to_string())
}

pub(crate) fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// `2 device · 1 third party`: the blocked items by what they wait for.
pub(crate) fn blocked_tally(group: &OwnerGroup) -> String {
    const ORDER: [(Category, &str); 5] = [
        (Category::ServerBlocked, "server"),
        (Category::DeviceNeeded, "device"),
        (Category::BuildNeeded, "build"),
        (Category::ThirdParty, "third party"),
        (Category::Blocked, "blocked"),
    ];
    let mut parts: Vec<String> = ORDER
        .iter()
        .filter_map(|(category, label)| {
            let n = group
                .items
                .iter()
                .filter(|i| i.category == *category)
                .count();
            (n > 0).then(|| format!("{n} {label}"))
        })
        .collect();
    let other = group
        .items
        .iter()
        .filter(|i| !ORDER.iter().any(|(category, _)| *category == i.category))
        .count();
    if other > 0 {
        parts.push(format!("{other} other"));
    }
    parts.join(" · ")
}

pub(crate) fn detail(group: &OwnerGroup) -> String {
    if group.bookkeeping_only {
        let command = group
            .command
            .as_deref()
            .map_or_else(String::new, terminal_capped);
        return format!("all checks passed, run {command} to finalize");
    }
    if group.kind == Kind::Debug {
        return match group.lane {
            Lane::Decide => "awaiting your confirmation".to_owned(),
            _ if group.counts.unreadable > 0 => "unreadable session".to_owned(),
            _ => {
                let status = group.items.first().map_or("", |item| item.result.as_str());
                format!("open investigation ({})", terminal_capped(status))
            }
        };
    }
    if group.kind == Kind::Verification
        && group.lane == Lane::Fix
        && group.file_status == "gaps_found"
        && group.items.is_empty()
        && group.counts.unreadable > 0
    {
        return "verification gaps_found, no gaps listed".to_owned();
    }
    if group.kind == Kind::Handoff {
        if group.counts.unreadable > 0 && group.items.is_empty() {
            return "unreadable handoff".to_owned();
        }
        let count = |result: &str| group.items.iter().filter(|i| i.result == result).count();
        let mut parts = Vec::new();
        if group.file_status != "unknown" {
            parts.push(terminal_capped(&group.file_status));
        }
        let actions = count("human_action");
        if actions > 0 {
            parts.push(format!(
                "{} pending",
                plural(actions, "human action", "human actions")
            ));
        }
        let blockers = count("blocker");
        if blockers > 0 {
            parts.push(plural(blockers, "blocker", "blockers"));
        }
        return parts.join(" · ");
    }
    if group.kind == Kind::Checkpoint {
        return if group.counts.unreadable > 0 && group.items.is_empty() {
            "unreadable checkpoint".to_owned()
        } else {
            format!("checkpoint: {}", terminal_capped(&group.file_status))
        };
    }
    if group.kind == Kind::ContextQuestions {
        let file = terminal_capped(group.file.rsplit('/').next().unwrap_or(&group.file));
        if group.counts.unreadable > 0 && group.items.is_empty() {
            return format!("unreadable file {file}");
        }
        return format!(
            "{} in {file}",
            plural(group.counts.checks, "open question", "open questions")
        );
    }
    match group.lane {
        Lane::Blocked => blocked_tally(group),
        Lane::Deferred => plural(group.counts.checks, "item", "items"),
        Lane::Fix => {
            let mut parts = Vec::new();
            if group.counts.failed > 0 {
                parts.push(format!(
                    "{}, {}",
                    plural(group.counts.failed, "failed check", "failed checks"),
                    plural(group.counts.gaps, "gap", "gaps")
                ));
            }
            if group.counts.unreadable > 0 {
                parts.push(plural(
                    group.counts.unreadable,
                    "unreadable check",
                    "unreadable checks",
                ));
            }
            if parts.is_empty() {
                parts.push(plural(group.counts.checks, "check", "checks"));
            }
            parts.join(", ")
        }
        _ => plural(group.counts.checks, "check", "checks"),
    }
}

pub(crate) fn group_line(alias: &str, group: &OwnerGroup) -> String {
    group_line_ext(alias, group, "")
}

/// [`group_line`] with `tail` appended to the detail part (the merged-entry file
/// count and the label tally of [`entry_line`]).
fn group_line_ext(alias: &str, group: &OwnerGroup, tail: &str) -> String {
    let detail_text = {
        let base = detail(group);
        if base.is_empty() {
            tail.trim_start_matches(" \u{b7} ").to_owned()
        } else {
            format!("{base}{tail}")
        }
    };
    let label = match (group.kind, &group.workstream) {
        (Kind::Debug, _) => Some("debug".to_owned()),
        (Kind::Handoff, None) => Some("handoff".to_owned()),
        (Kind::Handoff, Some(ws)) => Some(format!("handoff {}", terminal_capped(ws))),
        _ => None,
    };
    if let Some(label) = label {
        let mut parts = vec![
            terminal_capped(alias),
            label,
            terminal_capped(&group.phase_name),
            detail_text.clone(),
        ];
        if let Some(days) = group.age_days {
            parts.push(format!(
                "{days} d{}",
                if group.stale { " stale" } else { "" }
            ));
        }
        if let Some(command) = &group.command {
            parts.push(terminal_capped(command));
        }
        parts.retain(|part| !part.is_empty());
        return parts.join("  ");
    }
    let phase = match &group.workstream {
        Some(ws) => format!(
            "ph {}/{}",
            terminal_capped(ws),
            terminal_capped(&group.phase)
        ),
        None => format!("ph {}", terminal_capped(&group.phase)),
    };
    let mut parts = vec![
        terminal_capped(alias),
        phase,
        terminal_capped(&group.phase_name),
        detail_text,
    ];
    if let Scope::Archived { label, .. } = &group.scope {
        parts.push(format!("({})", terminal_capped(label)));
    }
    if let Some(days) = group.age_days {
        parts.push(format!(
            "{days} d{}",
            if group.stale { " stale" } else { "" }
        ));
    }
    if !group.bookkeeping_only {
        if let Some(command) = &group.command {
            parts.push(terminal_capped(command));
        }
    }
    parts.retain(|part| !part.is_empty());
    parts.join("  ")
}

fn alias_of(entry: &ProjectEntry) -> String {
    entry
        .alias
        .clone()
        .or_else(|| entry.target.clone())
        .unwrap_or_else(|| "(unnamed)".to_string())
}

struct Row<'a> {
    entry: &'a ProjectEntry,
    alias: String,
    group: &'a OwnerGroup,
}

fn push_section(out: &mut Vec<String>, heading: &str, rows: &[&Row]) {
    if rows.is_empty() {
        return;
    }
    out.push(String::new());
    out.push(heading.to_string());
    for row in rows {
        out.extend(group_checklist(&row.alias, row.group));
    }
}

/// The lines the CLI prints for one group: the group line, then each item and its
/// gaps. Shared by `inbox` (`push_section`) and the TUI's `y`, so the copied text
/// and the printed text cannot disagree (37-07, D-09).
pub(crate) fn group_checklist(alias: &str, group: &OwnerGroup) -> Vec<String> {
    let mut out = vec![format!("  {}", group_line(alias, group))];
    for item in &group.items {
        out.push(format!("    - [ ] {}", terminal_capped(&item.name)));
        for gap in &item.gaps {
            out.push(format!("        - {}", terminal_capped(&gap.name)));
        }
    }
    out
}

/// The `OLD DEBT` section as `inbox --include archived` prints it, over `groups`
/// (alias, group) pairs: the heading, then each admitted archived group in the
/// CLI's alias, phase, file order. Empty when no group qualifies.
pub(crate) fn old_debt_checklist(groups: &[(String, &OwnerGroup)]) -> Vec<String> {
    let filters = Filters {
        archived: true,
        ..Filters::default()
    };
    let mut rows: Vec<&(String, &OwnerGroup)> = groups
        .iter()
        .filter(|(_, g)| matches!(g.scope, Scope::Archived { .. }) && filters.lists_group(g))
        .collect();
    rows.sort_by(|a, b| {
        (&a.0, phase_key(&a.1.phase), &a.1.file).cmp(&(&b.0, phase_key(&b.1.phase), &b.1.file))
    });
    if rows.is_empty() {
        return Vec::new();
    }
    let mut out = vec!["OLD DEBT".to_string()];
    for (alias, group) in rows {
        out.extend(group_checklist(alias, group));
    }
    out
}

/// Alias, phase, file: the order of every section but TEST.
fn by_location(a: &Row, b: &Row) -> std::cmp::Ordering {
    (&a.alias, phase_key(&a.group.phase), &a.group.file).cmp(&(
        &b.alias,
        phase_key(&b.group.phase),
        &b.group.file,
    ))
}

/// Project-derived phase numbers, each escaped and capped on its own, space-joined.
fn phase_list(phases: &[String]) -> String {
    phases
        .iter()
        .map(|phase| terminal_capped(phase))
        .collect::<Vec<_>>()
        .join(" ")
}

/// How much of a `closes after` clause list to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClauseForm {
    /// Every phase listed.
    Full,
    /// At most this many phases per list, then ` …`.
    PhasesCapped(usize),
    /// Counts only, no phase lists.
    CountsOnly,
}

/// The `closes after` clauses of one project: the lane clauses (`test 12 (ph 21 22)`)
/// and the separate not-built clause (`not built ph 26`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ClosesAfterClauses {
    pub lanes: Vec<String>,
    pub not_built: Option<String>,
}

/// The shared wording of the R8 gate (CLI line and the Owner block header): lanes in
/// the order test, decide, unblock, fix, zero lanes omitted; `no owner work` when all
/// four are zero and phases are open; `not built ph …` for unbuilt phases. Each phase
/// string is escaped and capped on its own.
pub(crate) fn closes_after_clauses(gate: &ClosesAfter, form: ClauseForm) -> ClosesAfterClauses {
    let phases = |list: &[String]| -> String {
        match form {
            ClauseForm::Full | ClauseForm::CountsOnly => phase_list(list),
            ClauseForm::PhasesCapped(cap) if list.len() > cap => {
                format!("{} \u{2026}", phase_list(&list[..cap]))
            }
            ClauseForm::PhasesCapped(_) => phase_list(list),
        }
    };
    let with_phases = |head: String, list: &[String]| -> String {
        if form == ClauseForm::CountsOnly {
            head
        } else {
            format!("{head} (ph {})", phases(list))
        }
    };
    let lanes = [
        ("test", gate.test, &gate.lane_phases.test),
        ("decide", gate.decide, &gate.lane_phases.decide),
        ("unblock", gate.blocked, &gate.lane_phases.blocked),
        ("fix", gate.fix, &gate.lane_phases.fix),
    ];
    let mut out: Vec<String> = lanes
        .iter()
        .filter(|(_, count, _)| *count > 0)
        .map(|(label, count, list)| with_phases(format!("{label} {count}"), list))
        .collect();
    if out.is_empty() && !gate.phases_open.is_empty() {
        out.push(with_phases("no owner work".to_owned(), &gate.phases_open));
    }
    // Under `CountsOnly` the not-built clause is counted like the lanes, so each rung
    // of the Owner block header ladder is never wider than the one before it.
    let not_built = (!gate.not_built.is_empty()).then(|| match form {
        ClauseForm::CountsOnly => format!("not built {}", gate.not_built.len()),
        ClauseForm::Full | ClauseForm::PhasesCapped(_) => {
            format!("not built ph {}", phases(&gate.not_built))
        }
    });
    ClosesAfterClauses {
        lanes: out,
        not_built,
    }
}

/// One `closes after:` line per project with open or not-built current-milestone
/// phases (D-13, §3.2): the lanes that hold work, each with its phases.
///
/// Only the project-derived pieces (alias, version, each phase) are escaped and
/// capped; the generated structure is not, so a long line keeps its `fix` and
/// `not built` tail (WR-03).
fn push_closes_after(out: &mut Vec<String>, read: &[&ProjectEntry]) {
    let lines: Vec<String> = read
        .iter()
        .filter_map(|entry| {
            let gate = &entry.digest.closes_after;
            if gate.phases_open.is_empty() && gate.not_built.is_empty() {
                return None;
            }
            let clauses = closes_after_clauses(gate, ClauseForm::Full);
            let mut parts = clauses.lanes;
            parts.extend(clauses.not_built);
            let version = entry
                .milestone_version
                .as_deref()
                .filter(|v| !v.trim().is_empty())
                .map(|v| format!(" · {}", terminal_capped(v)))
                .unwrap_or_default();
            Some(format!(
                "  {}{version} closes after: {}",
                terminal_capped(&alias_of(entry)),
                parts.join(" · ")
            ))
        })
        .collect();
    if !lines.is_empty() {
        out.push(String::new());
        out.extend(lines);
    }
}

/// The plain checklist for a terminal or a note: no color, no TUI chrome. The
/// summary line and the folds count everything; the sections list what `filters`
/// admits.
pub fn render_text(entries: &[ProjectEntry], filters: &Filters) -> String {
    let read: Vec<&ProjectEntry> = entries.iter().filter(|e| e.error.is_none()).collect();
    let mut out = vec![totals_line(entries)];

    push_closes_after(&mut out, &read);

    let rows: Vec<Row> = read
        .iter()
        .flat_map(|entry| {
            entry
                .digest
                .groups
                .iter()
                .filter(|group| filters.lists_group(group))
                .map(|group| Row {
                    entry,
                    alias: alias_of(entry),
                    group,
                })
        })
        .collect();
    let current = |r: &&Row| matches!(r.group.scope, Scope::Current) && !r.group.acknowledged;
    let mut test: Vec<&Row> = rows
        .iter()
        .filter(|r| current(r) && r.group.lane == Lane::Test)
        .collect();
    test.sort_by(|a, b| {
        let remaining = |r: &Row| r.entry.phases_total.saturating_sub(r.entry.phases_done);
        b.group
            .closes_phase
            .cmp(&a.group.closes_phase)
            .then(remaining(a).cmp(&remaining(b)))
            .then(b.group.age_days.cmp(&a.group.age_days))
            .then_with(|| by_location(a, b))
    });
    let (closes, stays): (Vec<&Row>, Vec<&Row>) =
        test.into_iter().partition(|r| r.group.closes_phase);
    push_section(&mut out, "TEST — closes a phase", &closes);
    push_section(&mut out, "TEST — phase stays open", &stays);
    let lane_rows = |lane: Lane| {
        let mut v: Vec<&Row> = rows
            .iter()
            .filter(|r| current(r) && r.group.lane == lane)
            .collect();
        v.sort_by(|a, b| by_location(a, b));
        v
    };
    push_section(&mut out, "DECIDE", &lane_rows(Lane::Decide));
    push_section(&mut out, "FIX — code needed", &lane_rows(Lane::Fix));
    push_section(&mut out, "BLOCKED", &lane_rows(Lane::Blocked));

    let sorted = |keep: &dyn Fn(&Row) -> bool| {
        let mut v: Vec<&Row> = rows.iter().filter(|r| keep(r)).collect();
        v.sort_by(|a, b| by_location(a, b));
        v
    };
    let archived = sorted(&|r| matches!(r.group.scope, Scope::Archived { .. }));
    push_section(&mut out, "OLD DEBT", &archived);
    let deferred =
        sorted(&|r| matches!(r.group.scope, Scope::Current) && r.group.lane == Lane::Deferred);
    push_section(&mut out, "DEFERRED", &deferred);
    let acknowledged = sorted(&|r| {
        matches!(r.group.scope, Scope::Current)
            && r.group.lane != Lane::Deferred
            && r.group.acknowledged
    });
    push_section(&mut out, "ACKNOWLEDGED", &acknowledged);

    if filters.todos {
        let with_todos: Vec<&&ProjectEntry> = read.iter().filter(|e| !e.todos.is_empty()).collect();
        if !with_todos.is_empty() {
            out.push(String::new());
            out.push("TODOS".to_string());
            for entry in with_todos {
                out.push(format!(
                    "  {}  {}",
                    terminal_capped(&alias_of(entry)),
                    plural(entry.todos.len(), "pending todo", "pending todos")
                ));
                for todo in &entry.todos {
                    out.push(format!("    - [ ] {}", terminal_capped(&todo.title)));
                }
            }
        }
    }

    let folds = fold_lines(&read, filters, None);
    if !folds.is_empty() {
        out.push(String::new());
        out.extend(folds);
    }

    let failed = failed_lines(entries);
    if !failed.is_empty() {
        out.push(String::new());
        out.extend(failed);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// The lane sections of one project's owner entries, in `render_text` order: TEST
/// that closes a phase, TEST that leaves it open, DECIDE, FIX, BLOCKED. Each list
/// carries the same sort as the matching `render_text` section.
fn entry_sections<'a>(entries: &[&'a TriageEntry]) -> Vec<(&'static str, Vec<&'a TriageEntry>)> {
    let location = |a: &TriageEntry, b: &TriageEntry| {
        (phase_key(&a.group.phase), &a.group.file).cmp(&(phase_key(&b.group.phase), &b.group.file))
    };
    let mut test: Vec<&TriageEntry> = entries
        .iter()
        .copied()
        .filter(|e| e.group.lane == Lane::Test)
        .collect();
    test.sort_by(|a, b| {
        b.group
            .closes_phase
            .cmp(&a.group.closes_phase)
            .then(b.group.age_days.cmp(&a.group.age_days))
            .then_with(|| location(a, b))
    });
    let (closes, stays): (Vec<&TriageEntry>, Vec<&TriageEntry>) =
        test.into_iter().partition(|e| e.group.closes_phase);
    let lane = |lane: Lane| {
        let mut v: Vec<&TriageEntry> = entries
            .iter()
            .copied()
            .filter(|e| e.group.lane == lane)
            .collect();
        v.sort_by(|a, b| location(a, b));
        v
    };
    vec![
        ("TEST \u{2014} closes a phase", closes),
        ("TEST \u{2014} phase stays open", stays),
        ("DECIDE", lane(Lane::Decide)),
        ("FIX \u{2014} code needed", lane(Lane::Fix)),
        ("BLOCKED", lane(Lane::Blocked)),
    ]
}

/// `inbox ALIAS` (D-04): one project's full checklist. The totals line, its
/// `closes after` line, the lane sections over its unfolded entries (a same-phase
/// pair is one entry whose items sit under a `file NAME` line each), the entries
/// folded for age in a trailing `STALE (> N d)` section in the same order, then the
/// `--include` sections and fold lines. Nothing is hidden for a single project, so
/// no `Stale:` fold line is printed: the STALE section lists the work itself.
/// `--lane` narrows the sections as it does for [`render_text`]. An entry that could
/// not be read prints its totals line and its error line.
pub fn render_project(entry: &ProjectEntry, filters: &Filters, fold_days: u32) -> String {
    let all = std::slice::from_ref(entry);
    let mut out = vec![totals_line(all)];
    if entry.error.is_some() {
        out.push(String::new());
        out.extend(failed_lines(all));
        let mut text = out.join("\n");
        text.push('\n');
        return text;
    }
    let read = vec![entry];
    push_closes_after(&mut out, &read);

    let alias = alias_of(entry);
    let triage = triage_entries(&entry.digest, fold_days);
    let listed = |folded: bool| -> Vec<&TriageEntry> {
        triage
            .iter()
            .filter(|e| {
                e.folded == folded && is_owner_row(&e.group) && filters.lists_group(&e.group)
            })
            .collect()
    };
    let push_entries = |out: &mut Vec<String>, heading: &str, rows: &[&TriageEntry]| {
        if rows.is_empty() {
            return;
        }
        out.push(String::new());
        out.push(heading.to_string());
        for row in rows {
            out.extend(entry_checklist(&alias, row));
        }
    };
    for (heading, rows) in entry_sections(&listed(false)) {
        push_entries(&mut out, heading, &rows);
    }
    let stale: Vec<&TriageEntry> = entry_sections(&listed(true))
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .collect();
    push_entries(&mut out, &format!("STALE (> {fold_days} d)"), &stale);

    let push_groups = |out: &mut Vec<String>, heading: &str, keep: &dyn Fn(&OwnerGroup) -> bool| {
        let mut groups: Vec<&OwnerGroup> = entry
            .digest
            .groups
            .iter()
            .filter(|g| filters.lists_group(g) && keep(g))
            .collect();
        groups.sort_by(|a, b| (phase_key(&a.phase), &a.file).cmp(&(phase_key(&b.phase), &b.file)));
        if groups.is_empty() {
            return;
        }
        out.push(String::new());
        out.push(heading.to_string());
        for group in groups {
            out.extend(group_checklist(&alias, group));
        }
    };
    push_groups(&mut out, "OLD DEBT", &|g| {
        matches!(g.scope, Scope::Archived { .. })
    });
    push_groups(&mut out, "DEFERRED", &|g| {
        matches!(g.scope, Scope::Current) && g.lane == Lane::Deferred
    });
    push_groups(&mut out, "ACKNOWLEDGED", &|g| {
        matches!(g.scope, Scope::Current) && g.lane != Lane::Deferred && g.acknowledged
    });

    if filters.todos && !entry.todos.is_empty() {
        out.push(String::new());
        out.push("TODOS".to_string());
        out.push(format!(
            "  {}  {}",
            terminal_capped(&alias),
            plural(entry.todos.len(), "pending todo", "pending todos")
        ));
        for todo in &entry.todos {
            out.push(format!("    - [ ] {}", terminal_capped(&todo.title)));
        }
    }

    let folds = fold_lines(&read, filters, None);
    if !folds.is_empty() {
        out.push(String::new());
        out.extend(folds);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// One entry of the lane view with the project it belongs to.
struct LaneRow<'a> {
    project: &'a ProjectEntry,
    alias: String,
    entry: TriageEntry,
}

/// A single group as an entry of its own, for the sections that are not merged.
fn lone_entry(group: &OwnerGroup) -> TriageEntry {
    TriageEntry {
        group: group.clone(),
        parts: vec![group.clone()],
        indices: Vec::new(),
        folded: false,
    }
}

/// `inbox --lane L` over several projects (Phase 38, D-05, D-06): the totals line,
/// the legacy sections of the selected lanes with one collapsed line per phase, the
/// `--include` sections as collapsed lines, then the fold lines (`Stale:` counting
/// the selected lanes only) and the failed projects. No checklist item and no
/// `closes after` block is printed.
pub fn render_lanes(entries: &[ProjectEntry], filters: &Filters, fold_days: u32) -> String {
    let read: Vec<&ProjectEntry> = entries.iter().filter(|e| e.error.is_none()).collect();
    let mut out = vec![totals_line(entries)];

    let mut rows: Vec<LaneRow> = Vec::new();
    let mut folded: Vec<TriageEntry> = Vec::new();
    for project in &read {
        for entry in triage_entries(&project.digest, fold_days) {
            if !is_owner_row(&entry.group) || !filters.lists_group(&entry.group) {
                continue;
            }
            if entry.folded {
                folded.push(entry);
            } else {
                rows.push(LaneRow {
                    project,
                    alias: alias_of(project),
                    entry,
                });
            }
        }
    }
    let location = |a: &LaneRow, b: &LaneRow| {
        (
            &a.alias,
            phase_key(&a.entry.group.phase),
            &a.entry.group.file,
        )
            .cmp(&(
                &b.alias,
                phase_key(&b.entry.group.phase),
                &b.entry.group.file,
            ))
    };
    let push_rows = |out: &mut Vec<String>, heading: &str, rows: &[&LaneRow]| {
        if rows.is_empty() {
            return;
        }
        let cells: Vec<Collapsed> = rows
            .iter()
            .map(|row| collapsed_of(&row.alias, &row.entry))
            .collect();
        out.push(String::new());
        out.push(heading.to_string());
        out.extend(collapsed_block(&cells));
    };
    let mut test: Vec<&LaneRow> = rows
        .iter()
        .filter(|r| r.entry.group.lane == Lane::Test)
        .collect();
    test.sort_by(|a, b| {
        let remaining = |r: &LaneRow| r.project.phases_total.saturating_sub(r.project.phases_done);
        b.entry
            .group
            .closes_phase
            .cmp(&a.entry.group.closes_phase)
            .then(remaining(a).cmp(&remaining(b)))
            .then(b.entry.group.age_days.cmp(&a.entry.group.age_days))
            .then_with(|| location(a, b))
    });
    let (closes, stays): (Vec<&LaneRow>, Vec<&LaneRow>) =
        test.into_iter().partition(|r| r.entry.group.closes_phase);
    push_rows(&mut out, "TEST \u{2014} closes a phase", &closes);
    push_rows(&mut out, "TEST \u{2014} phase stays open", &stays);
    for (heading, lane) in [
        ("DECIDE", Lane::Decide),
        ("FIX \u{2014} code needed", Lane::Fix),
        ("BLOCKED", Lane::Blocked),
    ] {
        let mut v: Vec<&LaneRow> = rows.iter().filter(|r| r.entry.group.lane == lane).collect();
        v.sort_by(|a, b| location(a, b));
        push_rows(&mut out, heading, &v);
    }

    // The `--include` sections list single groups, as `render_text` selects them.
    let mut extra: Vec<LaneRow> = Vec::new();
    for project in &read {
        for group in project
            .digest
            .groups
            .iter()
            .filter(|g| filters.lists_group(g) && !is_owner_row(g))
        {
            extra.push(LaneRow {
                project,
                alias: alias_of(project),
                entry: lone_entry(group),
            });
        }
    }
    let pick = |keep: &dyn Fn(&OwnerGroup) -> bool| {
        let mut v: Vec<&LaneRow> = extra.iter().filter(|r| keep(&r.entry.group)).collect();
        v.sort_by(|a, b| location(a, b));
        v
    };
    push_rows(
        &mut out,
        "OLD DEBT",
        &pick(&|g| matches!(g.scope, Scope::Archived { .. })),
    );
    push_rows(
        &mut out,
        "DEFERRED",
        &pick(&|g| matches!(g.scope, Scope::Current) && g.lane == Lane::Deferred),
    );
    push_rows(
        &mut out,
        "ACKNOWLEDGED",
        &pick(&|g| matches!(g.scope, Scope::Current) && g.lane != Lane::Deferred && g.acknowledged),
    );

    if filters.todos {
        let with_todos: Vec<&&ProjectEntry> = read.iter().filter(|e| !e.todos.is_empty()).collect();
        if !with_todos.is_empty() {
            out.push(String::new());
            out.push("TODOS".to_string());
            for entry in with_todos {
                out.push(format!(
                    "  {}  {}",
                    terminal_capped(&alias_of(entry)),
                    plural(entry.todos.len(), "pending todo", "pending todos")
                ));
            }
        }
    }

    let stale = stale_totals(folded.iter());
    let folds = fold_lines(&read, filters, Some((&stale, fold_days)));
    if !folds.is_empty() {
        out.push(String::new());
        out.extend(folds);
    }
    let failed = failed_lines(entries);
    if !failed.is_empty() {
        out.push(String::new());
        out.extend(failed);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// The totals line: the same numbers `summary` and `status --json` `owner` carry.
fn totals_line(entries: &[ProjectEntry]) -> String {
    let read: Vec<&ProjectEntry> = entries.iter().filter(|e| e.error.is_none()).collect();
    let sum =
        |count: fn(&OwnerDigest) -> usize| read.iter().map(|e| count(&e.digest)).sum::<usize>();
    let fix_debug = sum(|d| d.fix_debug);
    let debug_tail = if fix_debug > 0 {
        format!(" + {fix_debug} debug")
    } else {
        String::new()
    };
    format!(
        "Owner inbox: {} · test {} · decide {} · fix {}{debug_tail} · blocked {}",
        plural(entries.len(), "project", "projects"),
        sum(|d| d.test),
        sum(|d| d.decide),
        plural(sum(|d| d.fix_phases), "phase", "phases"),
        sum(|d| d.blocked),
    )
}

/// `Stale: 3 checks in 2 phases older than 90 d`: what the fold hides (D-09). The
/// unit reads `items` and the noun `entries` once a folded entry is not a check
/// (a debug session, handoff or checkpoint is not a phase).
pub(crate) fn stale_fold_text(totals: &StaleTotals, fold_days: u32) -> String {
    let (items, entries) = if totals.non_check {
        (
            plural(totals.items, "item", "items"),
            plural(totals.entries, "entry", "entries"),
        )
    } else {
        (
            plural(totals.items, "check", "checks"),
            plural(totals.entries, "phase", "phases"),
        )
    };
    format!("Stale: {items} in {entries} older than {fold_days} d")
}

/// The fold lines below the sections: what is counted but not listed. `stale` adds
/// the `Stale:` line right after `Old debt:` when its entries are non-zero.
fn fold_lines(
    read: &[&ProjectEntry],
    filters: &Filters,
    stale: Option<(&StaleTotals, u32)>,
) -> Vec<String> {
    let sum =
        |count: fn(&OwnerDigest) -> usize| read.iter().map(|e| count(&e.digest)).sum::<usize>();
    let mut folds = Vec::new();
    let old_debt =
        |d: &OwnerDigest| d.archived.test + d.archived.blocked + d.archived.fix + d.archived.decide;
    let old_total: usize = read.iter().map(|e| old_debt(&e.digest)).sum();
    let old_projects = read.iter().filter(|e| old_debt(&e.digest) > 0).count();
    if !filters.archived && old_total > 0 {
        // Archived open questions are not checks: the fold says "items" once any are
        // in the total, and keeps the "checks" wording of a checks-only total.
        let unit = if sum(|d| d.archived.decide) > 0 {
            "items"
        } else {
            "checks"
        };
        folds.push(format!(
            "Old debt: {old_total} {unit} in {} (archived milestones)",
            plural(old_projects, "project", "projects")
        ));
    }
    if let Some((totals, fold_days)) = stale {
        if totals.entries > 0 {
            folds.push(stale_fold_text(totals, fold_days));
        }
    }
    let ack = sum(|d| d.hidden.acknowledged_files);
    if !filters.acknowledged && ack > 0 {
        folds.push(format!("Acknowledged: {}", plural(ack, "file", "files")));
    }
    let deferred_items = sum(|d| d.hidden.deferred);
    if !filters.deferred && deferred_items > 0 {
        folds.push(format!(
            "Deferred: {}",
            plural(deferred_items, "item", "items")
        ));
    }
    let todo_total: usize = read.iter().map(|e| e.digest.hidden.todos).sum();
    let todo_projects = read.iter().filter(|e| e.digest.hidden.todos > 0).count();
    if !filters.todos && todo_total > 0 {
        folds.push(format!(
            "Triage: {} in {}",
            plural(todo_total, "pending todo", "pending todos"),
            plural(todo_projects, "project", "projects")
        ));
    }
    // A pending folder that could not be listed counts 0; say so, since nothing is
    // listed for it even with `--include todos`.
    let todos_unreadable = read
        .iter()
        .filter(|e| e.digest.hidden.todos_unreadable)
        .count();
    if todos_unreadable > 0 {
        folds.push(format!(
            "Triage: pending todos unreadable in {}",
            plural(todos_unreadable, "project", "projects")
        ));
    }
    folds
}

/// One `alias: message` line per project that could not be read.
fn failed_lines(entries: &[ProjectEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|e| {
            e.error.as_ref().map(|error| {
                format!(
                    "{}: {}",
                    terminal_capped(&alias_of(e)),
                    terminal_capped(&error.message)
                )
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Entries, collapsed lines and the digest (Phase 38)
// ---------------------------------------------------------------------------

/// The width of `text` in terminal cells, the measure the TUI uses. Library code
/// keeps its own copy instead of importing the TUI's helper.
fn cell_width(text: &str) -> usize {
    ratatui::text::Span::raw(text).width()
}

/// `text` cut to at most `w` cells (the cut marked with `…`, a wide character never
/// split), then padded with blanks to exactly `w`, in front when `right`.
fn pad_cells(text: &str, w: usize, right: bool) -> String {
    let cut = if cell_width(text) <= w {
        text.to_owned()
    } else if w == 0 {
        String::new()
    } else {
        let mut out = String::new();
        let mut used = 0;
        let mut buf = [0u8; 4];
        for ch in text.chars() {
            let width = cell_width(ch.encode_utf8(&mut buf));
            if used + width > w - 1 {
                break;
            }
            out.push(ch);
            used += width;
        }
        out.push('\u{2026}');
        out
    };
    let gap = " ".repeat(w.saturating_sub(cell_width(&cut)));
    if right {
        format!("{gap}{cut}")
    } else {
        format!("{cut}{gap}")
    }
}

/// How many digits a count column needs: at least 2, more from 100 on.
pub(crate) fn count_digits(max: usize) -> usize {
    max.to_string().len().max(2)
}

/// A checks count as an aligned cell: the number right-aligned in `digits`
/// columns, a space, the unit left-aligned in 6 (` 1 check `, ` 2 checks`).
pub(crate) fn checks_cell(n: usize, digits: usize) -> String {
    let unit = if n == 1 { "check " } else { "checks" };
    format!("{n:>digits$} {unit}")
}

/// The cells of one collapsed line (D-06).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Collapsed {
    pub alias: String,
    pub phase: String,
    pub name: String,
    pub count: Option<usize>,
    pub phrase: String,
    pub age: String,
    pub command: String,
}

/// The phase cell of a group as [`group_line`] writes it.
fn phase_cell(group: &OwnerGroup) -> String {
    let mut cell = match (group.kind, &group.workstream) {
        (Kind::Debug, _) => "debug".to_owned(),
        (Kind::Handoff, None) => "handoff".to_owned(),
        (Kind::Handoff, Some(ws)) => format!("handoff {}", terminal_capped(ws)),
        (_, Some(ws)) => format!(
            "ph {}/{}",
            terminal_capped(ws),
            terminal_capped(&group.phase)
        ),
        (_, None) => format!("ph {}", terminal_capped(&group.phase)),
    };
    if !matches!(group.kind, Kind::Debug | Kind::Handoff) {
        if let Scope::Archived { label, .. } = &group.scope {
            cell.push_str(&format!(" ({})", terminal_capped(label)));
        }
    }
    cell
}

/// The collapsed cells of one entry: one line per phase (D-06).
pub(crate) fn collapsed_of(alias: &str, entry: &TriageEntry) -> Collapsed {
    let group = &entry.group;
    let counted = matches!(group.kind, Kind::Uat | Kind::Verification)
        && matches!(group.lane, Lane::Test | Lane::Fix)
        && !group.bookkeeping_only
        && group.counts.checks > 0;
    let count = counted.then_some(group.counts.checks);
    let mut phrase = Vec::new();
    if count.is_none()
        || (group.lane == Lane::Fix && (group.counts.failed > 0 || group.counts.unreadable > 0))
    {
        phrase.push(detail(group));
    }
    if group.closes_phase {
        phrase.push("\u{2192} closes phase".to_owned());
    }
    if entry.is_merged() {
        phrase.push(plural(entry.parts.len(), "file", "files"));
    }
    let labels = label_counts(&group.items, group.lane).text();
    if !labels.is_empty() {
        phrase.push(labels);
    }
    phrase.retain(|part| !part.is_empty());
    Collapsed {
        alias: terminal_capped(alias),
        phase: phase_cell(group),
        name: terminal_capped(&group.phase_name),
        count,
        phrase: phrase.join(" \u{b7} "),
        age: group.age_days.map_or_else(String::new, |days| {
            format!("{days} d{}", if group.stale { " stale" } else { "" })
        }),
        command: if group.bookkeeping_only {
            String::new()
        } else {
            group
                .command
                .as_deref()
                .map_or_else(String::new, terminal_capped)
        },
    }
}

/// Collapsed lines padded into columns: every column but the command takes the
/// width of its widest cell (the name at most 32 cells), a column no row fills is
/// left out, and the checks column right-aligns its number.
pub(crate) fn collapsed_block(rows: &[Collapsed]) -> Vec<String> {
    const NAME_CAP: usize = 32;
    let widest = |cell: fn(&Collapsed) -> &str| {
        rows.iter()
            .map(|row| cell_width(cell(row)))
            .max()
            .unwrap_or(0)
    };
    let alias_w = widest(|r| &r.alias);
    let phase_w = widest(|r| &r.phase);
    let name_w = widest(|r| &r.name).min(NAME_CAP);
    let phrase_w = widest(|r| &r.phrase);
    let age_w = widest(|r| &r.age);
    let digits = count_digits(rows.iter().filter_map(|r| r.count).max().unwrap_or(0));
    let has_count = rows.iter().any(|r| r.count.is_some());
    rows.iter()
        .map(|row| {
            let mut cols = vec![
                pad_cells(&row.alias, alias_w, false),
                pad_cells(&row.phase, phase_w, false),
            ];
            if name_w > 0 {
                cols.push(pad_cells(&row.name, name_w, false));
            }
            if has_count {
                cols.push(
                    row.count
                        .map_or_else(|| " ".repeat(digits + 1 + 6), |n| checks_cell(n, digits)),
                );
            }
            if phrase_w > 0 {
                cols.push(pad_cells(&row.phrase, phrase_w, false));
            }
            if age_w > 0 {
                cols.push(pad_cells(&row.age, age_w, false));
            }
            if !row.command.is_empty() {
                cols.push(row.command.clone());
            }
            format!("  {}", cols.join("  ")).trim_end().to_owned()
        })
        .collect()
}

/// The group line of one entry: a merged entry adds `N files`, and the label tally
/// follows (D-08, D-11).
pub(crate) fn entry_line(alias: &str, entry: &TriageEntry) -> String {
    let mut tail = String::new();
    if entry.is_merged() {
        tail.push_str(&format!(
            " \u{b7} {}",
            plural(entry.parts.len(), "file", "files")
        ));
    }
    let labels = label_counts(&entry.group.items, entry.group.lane).text();
    if !labels.is_empty() {
        tail.push_str(&format!(" \u{b7} {labels}"));
    }
    group_line_ext(alias, &entry.group, &tail)
}

/// The lines `inbox` prints for one entry: the entry line, then each item and its
/// gaps; a merged entry puts a `file NAME` line before each part's items, and an
/// item the matcher labels ends in `  (decision?)` or `  (looks resolved)`. Shared
/// with `inbox ALIAS` and the TUI's `y`, so the printed and the copied text cannot
/// disagree (37 D-09). For one unlabeled group it equals [`group_checklist`].
pub(crate) fn entry_checklist(alias: &str, entry: &TriageEntry) -> Vec<String> {
    let mut out = vec![format!("  {}", entry_line(alias, entry))];
    for part in &entry.parts {
        if entry.is_merged() {
            let name = part.file.rsplit('/').next().unwrap_or(&part.file);
            out.push(format!("    file {}", terminal_capped(name)));
        }
        for item in &part.items {
            let labels = labels_of(item, part.lane);
            let suffix = if labels.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = labels.iter().map(|l| l.as_str()).collect();
                format!("  ({})", names.join(" \u{b7} "))
            };
            out.push(format!("    - [ ] {}{suffix}", terminal_capped(&item.name)));
            for gap in &item.gaps {
                out.push(format!("        - {}", terminal_capped(&gap.name)));
            }
        }
    }
    out
}

/// The most lines the digest prints.
const DIGEST_MAX_LINES: usize = 25;
/// The most lines the project block holds.
const DIGEST_MAX_PROJECT_LINES: usize = 10;
/// The last line of the digest.
const DIGEST_HINT: &str =
    "inbox <alias> \u{b7} inbox --lane test|decide|fix|blocked \u{b7} inbox --all";

/// ` · elsewhere: test 2, decide 1`: the owner work the gate does not count.
fn lane_count_text(counts: &LaneCounts) -> String {
    [
        ("test", counts.test),
        ("decide", counts.decide),
        ("unblock", counts.blocked),
        ("fix", counts.fix),
    ]
    .iter()
    .filter(|(_, n)| *n > 0)
    .map(|(label, n)| format!("{label} {n}"))
    .collect::<Vec<_>>()
    .join(", ")
}

/// One project line of the digest (D-03), or `None` for a project without owner
/// work.
fn project_line(entry: &ProjectEntry) -> Option<String> {
    if !entry.digest.groups.iter().any(is_owner_row) {
        return None;
    }
    let gate = &entry.digest.closes_after;
    let clauses = closes_after_clauses(gate, ClauseForm::PhasesCapped(4));
    let mut parts = clauses.lanes;
    parts.extend(clauses.not_built);
    let elsewhere = lane_count_text(&outside_gate(&entry.digest));
    let version = entry
        .milestone_version
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(|v| format!(" \u{b7} {}", terminal_capped(v)))
        .unwrap_or_default();
    let head = format!("  {}{version}", terminal_capped(&alias_of(entry)));
    let mut line = if parts.is_empty() {
        head
    } else {
        format!("{head} closes after: {}", parts.join(" \u{b7} "))
    };
    if !elsewhere.is_empty() {
        if parts.is_empty() {
            line.push_str(&format!(" {elsewhere}"));
        } else {
            line.push_str(&format!(" \u{b7} elsewhere: {elsewhere}"));
        }
    }
    Some(line)
}

/// The digest `inbox` prints by default (Phase 38, D-01, D-03): the totals line, one
/// line per project with owner work, the top next actions, the fold lines and a hint,
/// within [`DIGEST_MAX_LINES`] lines. Everything it leaves out is announced by a
/// count line; every total still counts all work.
pub fn render_digest(entries: &[ProjectEntry], fold_days: u32) -> String {
    let read: Vec<&ProjectEntry> = entries.iter().filter(|e| e.error.is_none()).collect();
    let triaged: Vec<(String, Vec<TriageEntry>)> = read
        .iter()
        .map(|entry| (alias_of(entry), triage_entries(&entry.digest, fold_days)))
        .collect();

    // The sections after the project block, each without its leading blank line.
    let mut sections: Vec<Vec<String>> = Vec::new();
    let view: Vec<(&str, &[TriageEntry])> = triaged
        .iter()
        .map(|(alias, list)| (alias.as_str(), list.as_slice()))
        .collect();
    let next = next_actions(&view, NEXT_LIMIT);
    let next_block: Vec<String> = if next.is_empty() {
        Vec::new()
    } else {
        let rows: Vec<Collapsed> = next
            .iter()
            .map(|action| collapsed_of(action.alias, action.entry))
            .collect();
        let mut block = vec!["NEXT".to_owned()];
        block.extend(collapsed_block(&rows));
        block
    };
    let stale = stale_totals(triaged.iter().flat_map(|(_, list)| list.iter()));
    let mut folds = fold_lines(&read, &Filters::default(), Some((&stale, fold_days)));
    let failed: Vec<String> = entries
        .iter()
        .filter(|e| e.error.is_some())
        .map(|e| terminal_capped(&alias_of(e)))
        .collect();
    if !failed.is_empty() {
        let shown: Vec<&str> = failed.iter().take(3).map(String::as_str).collect();
        let more = if failed.len() > 3 { ", \u{2026}" } else { "" };
        folds.push(format!(
            "Unreadable: {} ({}{more}) \u{b7} inbox --all says why",
            plural(failed.len(), "project", "projects"),
            shown.join(", ")
        ));
    }
    if !next_block.is_empty() {
        sections.push(next_block);
    }
    if !folds.is_empty() {
        sections.push(folds);
    }

    // The project block: most closeable phases first, then alias.
    let mut projects: Vec<(&ProjectEntry, String)> = read
        .iter()
        .filter_map(|entry| project_line(entry).map(|line| (*entry, line)))
        .collect();
    projects.sort_by(|a, b| {
        b.0.digest
            .closeable_phases
            .cmp(&a.0.digest.closeable_phases)
            .then_with(|| alias_of(a.0).cmp(&alias_of(b.0)))
    });
    // Totals, every section with its blank line, the blank line and the hint.
    let others: usize = 1 + sections.iter().map(|s| 1 + s.len()).sum::<usize>() + 2;
    let room = DIGEST_MAX_LINES
        .saturating_sub(others + 1)
        .min(DIGEST_MAX_PROJECT_LINES);
    let mut block: Vec<String> = Vec::new();
    if !projects.is_empty() && room > 0 {
        if projects.len() <= room {
            block.extend(projects.into_iter().map(|(_, line)| line));
        } else {
            let shown = room - 1;
            let rest = projects.len() - shown;
            block.extend(projects.into_iter().take(shown).map(|(_, line)| line));
            block.push(format!("  +{rest} more projects (inbox --all)"));
        }
    }

    let mut out = vec![totals_line(entries)];
    if !block.is_empty() {
        out.push(String::new());
        out.extend(block);
    }
    for section in sections {
        out.push(String::new());
        out.extend(section);
    }
    if out.len() > 1 {
        out.push(String::new());
    }
    out.push(DIGEST_HINT.to_owned());
    let mut text = out.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{Category, GroupCounts, Kind, LanePhases};

    fn group(lane: Lane) -> OwnerGroup {
        OwnerGroup {
            lane,
            kind: Kind::Uat,
            scope: Scope::Current,
            workstream: None,
            phase: "01".into(),
            phase_name: "Alpha\u{1b}[31m".into(),
            file: ".planning/phases/01-alpha/01-UAT.md".into(),
            file_status: "testing".into(),
            closes_phase: false,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: Some(3),
            stale: false,
            run: Some(crate::state_reader::owner_items::RunCommand {
                verb: crate::state_reader::owner_items::RunVerb::VerifyWork,
                phase: Some("01".into()),
                workstream: None,
            }),
            command: Some("/gsd-verify-work 01".into()),
            supersedes: Vec::new(),
            counts: GroupCounts {
                checks: 1,
                ..Default::default()
            },
            items: vec![OwnerItem {
                id: "01-UAT.md#2".into(),
                test: Some("2".into()),
                name: "Two".into(),
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

    fn entry(groups: Vec<OwnerGroup>) -> ProjectEntry {
        ProjectEntry {
            alias: Some("a".into()),
            path: Some("/p".into()),
            digest: OwnerDigest {
                groups,
                test: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn the_document_names_its_schema_and_sums_the_projects() {
        let doc = build_document(
            &[entry(vec![group(Lane::Test)]), entry(Vec::new())],
            &Filters::default(),
            90,
        );
        assert_eq!(doc["schema"], "gsd-meta-manager/inbox");
        assert_eq!(doc["schema_version"], 1);
        assert_eq!(doc["summary"]["projects"], 2);
        assert_eq!(doc["summary"]["test"], 2);
        assert_eq!(doc["summary"]["decide"], 0);
        assert_eq!(doc["summary"]["fix_debug"], 0);
        assert!(doc["projects"][0]["milestone"]["version"].is_null());
    }

    #[test]
    fn closes_after_clauses_forms() {
        use crate::state_reader::owner_items::LanePhases;
        let ph = |list: &[&str]| list.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();
        let gate = ClosesAfter {
            test: 12,
            decide: 0,
            fix: 1,
            blocked: 5,
            phases_open: ph(&["20", "21"]),
            not_built: ph(&["26"]),
            lane_phases: LanePhases {
                test: ph(&["21", "22", "24"]),
                decide: Vec::new(),
                fix: ph(&["25"]),
                blocked: ph(&["20", "23"]),
            },
        };
        let full = closes_after_clauses(&gate, ClauseForm::Full);
        assert_eq!(
            full.lanes,
            [
                "test 12 (ph 21 22 24)",
                "unblock 5 (ph 20 23)",
                "fix 1 (ph 25)"
            ]
        );
        assert_eq!(full.not_built.as_deref(), Some("not built ph 26"));
        let capped = closes_after_clauses(&gate, ClauseForm::PhasesCapped(2));
        assert_eq!(capped.lanes[0], "test 12 (ph 21 22 \u{2026})");
        assert_eq!(capped.lanes[2], "fix 1 (ph 25)");
        let counts = closes_after_clauses(&gate, ClauseForm::CountsOnly);
        assert_eq!(counts.lanes, ["test 12", "unblock 5", "fix 1"]);
        assert_eq!(counts.not_built.as_deref(), Some("not built 1"));

        // Counts only lists no phases, however many are not built: the header ladder
        // stays monotonic (rung 3 is never wider than rung 2).
        let many_unbuilt = ClosesAfter {
            not_built: (26..40).map(|n| n.to_string()).collect(),
            ..gate.clone()
        };
        let capped = closes_after_clauses(&many_unbuilt, ClauseForm::PhasesCapped(6));
        let counts = closes_after_clauses(&many_unbuilt, ClauseForm::CountsOnly);
        assert_eq!(counts.not_built.as_deref(), Some("not built 14"));
        assert!(
            counts.not_built.as_ref().map(String::len) < capped.not_built.as_ref().map(String::len)
        );

        let idle = ClosesAfter {
            phases_open: ph(&["20", "21"]),
            ..Default::default()
        };
        assert_eq!(
            closes_after_clauses(&idle, ClauseForm::Full).lanes,
            ["no owner work (ph 20 21)"]
        );
        assert_eq!(
            closes_after_clauses(&idle, ClauseForm::CountsOnly).lanes,
            ["no owner work"]
        );
        let none = closes_after_clauses(&ClosesAfter::default(), ClauseForm::Full);
        assert!(none.lanes.is_empty() && none.not_built.is_none());
    }

    #[test]
    fn project_authored_strings_are_escaped() {
        let doc = build_document(&[entry(vec![group(Lane::Test)])], &Filters::default(), 90);
        let name = doc["projects"][0]["groups"][0]["phase_name"]
            .as_str()
            .unwrap();
        assert!(!name.contains('\u{1b}'), "{name:?}");
    }

    #[test]
    fn lane_filters_list_only_the_chosen_lanes_and_leave_the_summary_alone() {
        let entries = [entry(vec![group(Lane::Test), group(Lane::Blocked)])];
        let all = build_document(&entries, &Filters::default(), 90);
        assert_eq!(all["projects"][0]["groups"].as_array().unwrap().len(), 2);
        let only = Filters {
            lanes: vec![Lane::Blocked],
            ..Default::default()
        };
        let doc = build_document(&entries, &only, 90);
        let groups = doc["projects"][0]["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["lane"], "blocked");
        assert_eq!(doc["summary"]["test"], 1);
    }

    #[test]
    fn an_error_entry_has_the_full_key_set_with_empty_data() {
        let failed = ProjectEntry {
            target: Some("nope".into()),
            error: Some(EntryError {
                kind: "unresolved",
                message: "m".into(),
            }),
            ..Default::default()
        };
        let doc = build_document(&[failed], &Filters::default(), 90);
        let project = &doc["projects"][0];
        assert_eq!(project["error"]["kind"], "unresolved");
        assert!(project["milestone"].is_null());
        assert_eq!(project["groups"], json!([]));
        assert_eq!(project["todos"], json!([]));
        assert_eq!(project["hidden"]["noise"], 0);
        assert_eq!(project["hidden"]["planned_questions"], 0);
        assert_eq!(doc["summary"]["planned_questions"], 0);
        assert_eq!(doc["summary"]["projects"], 1);
    }

    #[test]
    fn the_old_debt_fold_says_items_once_archived_questions_are_in_it() {
        let fold = |decide: usize| {
            let mut e = entry(Vec::new());
            e.digest.archived.test = 2;
            e.digest.archived.decide = decide;
            render_text(&[e], &Filters::default())
                .lines()
                .find(|line| line.starts_with("Old debt: "))
                .expect("an Old debt fold")
                .to_owned()
        };
        assert_eq!(
            fold(0),
            "Old debt: 2 checks in 1 project (archived milestones)"
        );
        assert_eq!(
            fold(3),
            "Old debt: 5 items in 1 project (archived milestones)"
        );
    }

    #[test]
    fn an_unreadable_todo_folder_has_its_own_fold() {
        let mut broken = entry(Vec::new());
        broken.digest.hidden.todos_unreadable = true;
        for filters in [
            Filters::default(),
            Filters {
                todos: true,
                ..Default::default()
            },
        ] {
            let text = render_text(&[broken.clone(), entry(Vec::new())], &filters);
            assert!(
                text.lines()
                    .any(|line| line == "Triage: pending todos unreadable in 1 project"),
                "{text}"
            );
        }
        let clean = render_text(&[entry(Vec::new())], &Filters::default());
        assert!(!clean.contains("unreadable"), "{clean}");
    }

    #[test]
    fn a_long_closes_after_line_keeps_every_lane_and_escapes_each_piece() {
        let phases = |range: std::ops::Range<u32>| -> Vec<String> {
            range.map(|n| format!("{n}.1")).collect()
        };
        let mut long = entry(Vec::new());
        long.alias = Some(format!("{}\u{1b}[31m", "a".repeat(60)));
        long.milestone_version = Some("v9.9\u{1b}[0m".into());
        long.digest.closes_after = ClosesAfter {
            test: 12,
            decide: 12,
            fix: 12,
            blocked: 12,
            phases_open: phases(1..13),
            not_built: vec!["13".into(), "14\u{7}".into()],
            lane_phases: LanePhases {
                test: phases(1..13),
                decide: phases(1..13),
                fix: phases(1..13),
                blocked: phases(1..13),
            },
        };
        let text = render_text(&[long], &Filters::default());
        let line = text
            .lines()
            .find(|line| line.contains("closes after:"))
            .expect("a closes after line");
        assert!(line.chars().count() > crate::text::MAX_TEXT_CHARS, "{line}");
        assert!(!line.ends_with('…'), "{line}");
        assert!(
            line.contains(&format!("fix 12 (ph {})", phases(1..13).join(" "))),
            "{line}"
        );
        assert!(line.contains("not built ph 13 "), "{line}");
        assert!(
            !line.contains('\u{1b}') && !line.contains('\u{7}'),
            "{line:?}"
        );
    }

    // --- Phase 38: cells, entries, digest -----------------------------------

    fn fresh(mut g: OwnerGroup, phase: &str, age: Option<i64>) -> OwnerGroup {
        g.phase = phase.into();
        g.age_days = age;
        g.file = format!(".planning/phases/{phase}/{phase}-UAT.md");
        g.command = Some(format!("/gsd-verify-work {phase}"));
        g
    }

    fn named(alias: &str, groups: Vec<OwnerGroup>) -> ProjectEntry {
        let mut e = entry(groups);
        e.alias = Some(alias.into());
        e
    }

    #[test]
    fn checks_cell_aligns_the_number_and_the_unit() {
        assert_eq!(checks_cell(1, 2), " 1 check ");
        assert_eq!(checks_cell(2, 2), " 2 checks");
        assert_eq!(checks_cell(12, 2), "12 checks");
        assert_eq!(checks_cell(120, 3), "120 checks");
        assert_eq!(checks_cell(1, 3), "  1 check ");
        assert_eq!(count_digits(0), 2);
        assert_eq!(count_digits(99), 2);
        assert_eq!(count_digits(100), 3);
    }

    #[test]
    fn pad_cells_cuts_with_an_ellipsis_and_counts_wide_characters() {
        assert_eq!(pad_cells("abcdef", 4, false), "abc\u{2026}");
        assert_eq!(pad_cells("ab", 4, true), "  ab");
        assert_eq!(pad_cells("ab", 4, false), "ab  ");
        assert_eq!(cell_width("\u{4e2d}"), 2);
        // A two-cell character is never split: 3 cells hold one of them and the cut.
        assert_eq!(
            pad_cells("\u{4e2d}\u{4e2d}\u{4e2d}", 3, false),
            "\u{4e2d}\u{2026}"
        );
    }

    #[test]
    fn a_thin_digest_names_next_actions_folds_and_the_hint() {
        let mut a = group(Lane::Test);
        a = fresh(a, "01", Some(2));
        a.closes_phase = true;
        let b = fresh(group(Lane::Test), "02", Some(9));
        let mut e = named("alpha", vec![a, b]);
        e.digest.hidden.deferred = 3;
        let text = render_digest(&[e], 90);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("Owner inbox: 1 project "), "{text}");
        let next = lines.iter().position(|l| *l == "NEXT").expect("NEXT");
        assert!(lines[next + 1].contains("ph 01"), "{text}");
        assert!(lines[next + 1].contains("\u{2192} closes phase"), "{text}");
        assert!(lines[next + 1].contains("/gsd-verify-work 01"), "{text}");
        assert!(lines[next + 2].contains("ph 02"), "{text}");
        assert!(lines.contains(&"Deferred: 3 items"), "{text}");
        assert!(lines.iter().all(|l| !l.starts_with("    - [ ] ")));
        assert_eq!(
            *lines.last().unwrap(),
            "inbox <alias> \u{b7} inbox --lane test|decide|fix|blocked \u{b7} inbox --all"
        );
    }

    #[test]
    fn an_empty_fleet_renders_the_totals_and_the_hint_only() {
        let text = render_digest(&[], 90);
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(text.starts_with("Owner inbox: 0 projects"));
        let quiet = render_digest(&[named("q", Vec::new())], 90);
        assert!(!quiet.contains("NEXT"), "{quiet}");
        assert_eq!(quiet.lines().count(), 2, "{quiet}");
        assert_eq!(
            render_digest(&[entry(vec![group(Lane::Test)])], 90)
                .lines()
                .next(),
            render_text(&[entry(vec![group(Lane::Test)])], &Filters::default())
                .lines()
                .next()
        );
    }

    #[test]
    fn stale_fold_text_counts_phases_and_picks_the_unit() {
        let t = |entries, items, non_check| StaleTotals {
            entries,
            items,
            non_check,
        };
        assert_eq!(
            stale_fold_text(&t(2, 3, false), 90),
            "Stale: 3 checks in 2 phases older than 90 d"
        );
        assert_eq!(
            stale_fold_text(&t(1, 1, false), 90),
            "Stale: 1 check in 1 phase older than 90 d"
        );
        assert_eq!(
            stale_fold_text(&t(1, 2, true), 30),
            "Stale: 2 items in 1 entry older than 30 d"
        );
    }

    #[test]
    fn a_folded_entry_leaves_next_but_the_totals_and_a_stale_line_stay() {
        let old = fresh(group(Lane::Test), "01", Some(200));
        let text = render_digest(&[named("alpha", vec![old])], 90);
        assert!(!text.contains("NEXT"), "{text}");
        assert!(
            text.lines()
                .any(|l| l == "Stale: 1 check in 1 phase older than 90 d"),
            "{text}"
        );
        assert!(text.contains("test 1"), "{text}");
    }

    #[test]
    fn project_lines_show_the_gate_and_the_work_elsewhere() {
        use crate::state_reader::owner_items::LanePhases;
        let mut with_gate = named("beta", vec![fresh(group(Lane::Test), "01", Some(1))]);
        with_gate.milestone_version = Some("v2.0".into());
        with_gate.digest.closeable_phases = 1;
        with_gate.digest.closes_after = ClosesAfter {
            test: 1,
            phases_open: vec!["01".into()],
            lane_phases: LanePhases {
                test: vec!["01".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut ws = fresh(group(Lane::Test), "43", Some(1));
        ws.workstream = Some("market".into());
        with_gate.digest.groups.push(ws);
        let mut debug_only = named("alpha", vec![group(Lane::Fix)]);
        debug_only.digest.groups[0].kind = Kind::Debug;
        let quiet = named("gamma", Vec::new());
        let text = render_digest(&[quiet, debug_only, with_gate], 90);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[2], "  beta \u{b7} v2.0 closes after: test 1 (ph 01) \u{b7} elsewhere: test 1",
            "{text}"
        );
        assert_eq!(lines[3], "  alpha fix 1", "{text}");
        assert!(!text.contains("gamma"), "{text}");
    }

    #[test]
    fn the_worst_case_digest_stays_within_the_line_budget() {
        let mut fleet: Vec<ProjectEntry> = (0..20)
            .map(|n| {
                let mut live = fresh(group(Lane::Test), "01", Some(5));
                live.closes_phase = n % 2 == 0;
                let old = fresh(group(Lane::Test), "02", Some(200));
                let mut e = named(&format!("proj{n:02}"), vec![live, old]);
                e.digest.archived.test = 2;
                e.digest.hidden.acknowledged_files = 1;
                e.digest.hidden.deferred = 2;
                e.digest.hidden.todos = 1;
                e.digest.hidden.todos_unreadable = n < 2;
                e.digest.closeable_phases = n % 3;
                e
            })
            .collect();
        for name in ["gone", "also-gone"] {
            fleet.push(ProjectEntry {
                alias: Some(name.into()),
                error: Some(EntryError {
                    kind: "unresolved",
                    message: "m".into(),
                }),
                ..Default::default()
            });
        }
        let text = render_digest(&fleet, 90);
        let count = text.lines().count();
        assert!(count <= 25, "{count} lines\n{text}");
        assert_eq!(
            text.lines()
                .filter(|l| l.starts_with("Unreadable:"))
                .count(),
            1
        );
        assert!(text.contains("more projects (inbox --all)"), "{text}");
        assert!(text.lines().any(|l| l.starts_with("Stale:")), "{text}");
        eprintln!("worst-case digest lines: {count}");
    }

    #[test]
    fn an_unlabeled_single_entry_checklist_equals_the_group_checklist() {
        let g = group(Lane::Test);
        let entries = triage_entries(
            &OwnerDigest {
                groups: vec![g.clone()],
                ..Default::default()
            },
            90,
        );
        assert_eq!(entry_checklist("a", &entries[0]), group_checklist("a", &g));
    }

    #[test]
    fn a_merged_entry_lists_its_files_and_labels_items() {
        let mut uat = fresh(group(Lane::Test), "43", Some(4));
        uat.workstream = Some("market".into());
        uat.items = vec![uat.items[0].clone(), uat.items[0].clone()];
        uat.items[0].name = "Approve the copy".into();
        uat.items[1].name = "Fixed by 12\u{1b}[31m".into();
        uat.counts.checks = 2;
        uat.file = ".planning/phases/43/43-UAT.md".into();
        let mut ver = uat.clone();
        ver.kind = Kind::Verification;
        ver.file = ".planning/phases/43/43-VERIFICATION\u{7}.md".into();
        ver.items = vec![uat.items[0].clone()];
        ver.items[0].name = "Plain".into();
        ver.counts.checks = 1;
        let entries = triage_entries(
            &OwnerDigest {
                groups: vec![uat, ver],
                ..Default::default()
            },
            90,
        );
        assert_eq!(entries.len(), 1);
        let lines = entry_checklist("a", &entries[0]);
        assert!(lines[0].contains("3 checks \u{b7} 2 files"), "{lines:?}");
        assert!(
            lines[0].contains("1 decision? \u{b7} 1 looks resolved"),
            "{lines:?}"
        );
        assert_eq!(lines[1], "    file 43-UAT.md");
        assert_eq!(lines[2], "    - [ ] Approve the copy  (decision?)");
        assert!(lines[3].ends_with("  (looks resolved)"), "{lines:?}");
        assert!(
            lines[4].starts_with("    file 43-VERIFICATION"),
            "{lines:?}"
        );
        assert_eq!(lines[5], "    - [ ] Plain");
        for line in &lines {
            assert!(
                !line.contains('\u{1b}') && !line.contains('\u{7}'),
                "{line:?}"
            );
        }
        let collapsed = collapsed_of("a", &entries[0]);
        assert!(collapsed.phrase.contains("2 files"), "{collapsed:?}");
        assert!(
            collapsed
                .phrase
                .contains("1 decision? \u{b7} 1 looks resolved"),
            "{collapsed:?}"
        );
        let plain = triage_entries(
            &OwnerDigest {
                groups: vec![group(Lane::Test)],
                ..Default::default()
            },
            90,
        );
        assert_eq!(collapsed_of("a", &plain[0]).phrase, "");
    }
    #[test]
    fn entry_checklist_equals_group_checklist_on_the_golden_fixtures() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/owner");
        let now: chrono::DateTime<chrono::Utc> = "2026-11-14T00:00:00Z".parse().unwrap();
        let mut compared = 0;
        for name in [
            "supersede-six-phases",
            "uat-gaps-merge",
            "archived-complete-skipped",
            "deferred-200",
            "uat-ack-match",
            "workstreams",
        ] {
            let planning = root.join(name).join(".planning");
            let state = crate::state_reader::parse_project_state(&planning);
            let digest =
                crate::state_reader::owner_items::read_owner_digest(&planning, &state, now, 30);
            for entry in triage_entries(&digest, 90) {
                let plain = !entry.is_merged()
                    && entry
                        .group
                        .items
                        .iter()
                        .all(|item| labels_of(item, entry.group.lane).is_empty());
                if plain {
                    assert_eq!(
                        entry_checklist("a", &entry),
                        group_checklist("a", &entry.group),
                        "{name}"
                    );
                    compared += 1;
                }
            }
        }
        assert!(compared >= 6, "{compared} entries compared");
    }

    #[test]
    fn a_three_digit_count_prints_uncut_and_aligns_the_units() {
        let row = |count: usize| Collapsed {
            alias: "a".into(),
            phase: "ph 01".into(),
            name: "Name".into(),
            count: Some(count),
            ..Collapsed::default()
        };
        let lines = collapsed_block(&[row(120), row(3)]);
        assert!(lines[0].contains("120 checks"), "{lines:?}");
        assert!(lines[1].contains("  3 checks"), "{lines:?}");
        assert_eq!(lines[0].find("checks"), lines[1].find("checks"));
    }

    #[test]
    fn a_merged_pair_shares_entry_files_and_the_folded_flag() {
        let mut uat = group(Lane::Test);
        uat.age_days = Some(200);
        let mut verification = group(Lane::Test);
        verification.kind = Kind::Verification;
        verification.file = ".planning/phases/01-alpha/01-VERIFICATION.md".into();
        verification.age_days = Some(5);
        let doc = build_document(&[entry(vec![uat, verification])], &Filters::default(), 90);
        let groups = doc["projects"][0]["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0]["entry_files"], groups[1]["entry_files"]);
        assert_eq!(
            groups[0]["entry_files"],
            serde_json::json!(["01-UAT.md", "01-VERIFICATION.md"])
        );
        assert_eq!(groups[0]["folded_stale"], groups[1]["folded_stale"]);
        assert_eq!(groups[0]["items"][0]["labels"], serde_json::json!([]));
    }
}
