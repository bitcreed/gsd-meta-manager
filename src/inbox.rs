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
    Category, ClosesAfter, Kind, Lane, OwnerDigest, OwnerGroup, OwnerItem, Scope,
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

fn item_json(item: &OwnerItem) -> Value {
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
    })
}

fn group_json(group: &OwnerGroup) -> Value {
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
        "supersedes": group.supersedes.iter().map(|id| terminal_capped(id)).collect::<Vec<_>>(),
        "counts": {
            "checks": group.counts.checks,
            "failed": group.counts.failed,
            "gaps": group.counts.gaps,
            "unreadable": group.counts.unreadable,
        },
        "items": group.items.iter().map(item_json).collect::<Vec<_>>(),
    })
}

fn project_json(entry: &ProjectEntry, filters: &Filters) -> Value {
    let (error, milestone, groups, hidden, todos) = match &entry.error {
        Some(error) => (
            json!({"kind": error.kind, "message": error.message}),
            Value::Null,
            Vec::new(),
            json!({
                "deferred": 0,
                "acknowledged_files": 0,
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
                .filter(|group| filters.lists_group(group))
                .map(group_json)
                .collect(),
            json!({
                "deferred": entry.digest.hidden.deferred,
                "acknowledged_files": entry.digest.hidden.acknowledged_files,
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
pub fn build_document(entries: &[ProjectEntry], filters: &Filters) -> Value {
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
            "superseded": sum(|d| d.superseded),
            "noise": sum(|d| d.hidden.noise),
        },
        "projects": entries.iter().map(|entry| project_json(entry, filters)).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// Plain checklist (D-21)
// ---------------------------------------------------------------------------

/// A phase number as a sort key: numeric first, then the raw text.
fn phase_key(phase: &str) -> (u64, String) {
    let digits: String = phase.chars().take_while(char::is_ascii_digit).collect();
    (digits.parse().unwrap_or(u64::MAX), phase.to_string())
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// `2 device · 1 third party`: the blocked items by what they wait for.
fn blocked_tally(group: &OwnerGroup) -> String {
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

fn detail(group: &OwnerGroup) -> String {
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

fn group_line(alias: &str, group: &OwnerGroup) -> String {
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
            detail(group),
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
        detail(group),
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
        out.push(format!("  {}", group_line(&row.alias, row.group)));
        for item in &row.group.items {
            out.push(format!("    - [ ] {}", terminal_capped(&item.name)));
            for gap in &item.gaps {
                out.push(format!("        - {}", terminal_capped(&gap.name)));
            }
        }
    }
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
            let lanes = [
                ("test", gate.test, &gate.lane_phases.test),
                ("decide", gate.decide, &gate.lane_phases.decide),
                ("unblock", gate.blocked, &gate.lane_phases.blocked),
                ("fix", gate.fix, &gate.lane_phases.fix),
            ];
            let mut parts: Vec<String> = lanes
                .iter()
                .filter(|(_, count, _)| *count > 0)
                .map(|(label, count, phases)| {
                    format!("{label} {count} (ph {})", phase_list(phases))
                })
                .collect();
            if parts.is_empty() && !gate.phases_open.is_empty() {
                parts.push(format!(
                    "no owner work (ph {})",
                    phase_list(&gate.phases_open)
                ));
            }
            if !gate.not_built.is_empty() {
                parts.push(format!("not built ph {}", phase_list(&gate.not_built)));
            }
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
    let sum =
        |count: fn(&OwnerDigest) -> usize| read.iter().map(|e| count(&e.digest)).sum::<usize>();
    let fix_debug = sum(|d| d.fix_debug);
    let debug_tail = if fix_debug > 0 {
        format!(" + {fix_debug} debug")
    } else {
        String::new()
    };
    let mut out = vec![format!(
        "Owner inbox: {} projects · test {} · decide {} · fix {} phases{debug_tail} · blocked {}",
        entries.len(),
        sum(|d| d.test),
        sum(|d| d.decide),
        sum(|d| d.fix_phases),
        sum(|d| d.blocked),
    )];

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
            "Old debt: {old_total} {unit} in {old_projects} projects (archived milestones)"
        ));
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
            "Triage: {todo_total} pending todos in {todo_projects} projects"
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
            "Triage: pending todos unreadable in {todos_unreadable} projects"
        ));
    }
    if !folds.is_empty() {
        out.push(String::new());
        out.extend(folds);
    }

    let failed: Vec<String> = entries
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
        .collect();
    if !failed.is_empty() {
        out.push(String::new());
        out.extend(failed);
    }
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
    fn project_authored_strings_are_escaped() {
        let doc = build_document(&[entry(vec![group(Lane::Test)])], &Filters::default());
        let name = doc["projects"][0]["groups"][0]["phase_name"]
            .as_str()
            .unwrap();
        assert!(!name.contains('\u{1b}'), "{name:?}");
    }

    #[test]
    fn lane_filters_list_only_the_chosen_lanes_and_leave_the_summary_alone() {
        let entries = [entry(vec![group(Lane::Test), group(Lane::Blocked)])];
        let all = build_document(&entries, &Filters::default());
        assert_eq!(all["projects"][0]["groups"].as_array().unwrap().len(), 2);
        let only = Filters {
            lanes: vec![Lane::Blocked],
            ..Default::default()
        };
        let doc = build_document(&entries, &only);
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
        let doc = build_document(&[failed], &Filters::default());
        let project = &doc["projects"][0];
        assert_eq!(project["error"]["kind"], "unresolved");
        assert!(project["milestone"].is_null());
        assert_eq!(project["groups"], json!([]));
        assert_eq!(project["todos"], json!([]));
        assert_eq!(project["hidden"]["noise"], 0);
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
            "Old debt: 2 checks in 1 projects (archived milestones)"
        );
        assert_eq!(
            fold(3),
            "Old debt: 5 items in 1 projects (archived milestones)"
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
                    .any(|line| line == "Triage: pending todos unreadable in 1 projects"),
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
}
