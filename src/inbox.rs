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
use crate::state_reader::owner_items::{Lane, OwnerDigest, OwnerGroup, OwnerItem, Scope};
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
    /// The pending todos to list (empty unless `--include todos`).
    pub todos: Vec<PendingTodo>,
    /// The pending todo count, whether listed or not.
    pub todos_count: usize,
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
    fn lists(&self, lane: Lane) -> bool {
        if self.lanes.is_empty() {
            lane != Lane::Deferred
        } else {
            self.lanes.contains(&lane)
        }
    }
}

fn opt_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| json!(terminal_capped(text)))
}

fn item_json(item: &OwnerItem) -> Value {
    json!({
        "id": terminal_capped(&item.id),
        "test": opt_text(item.test.as_deref()),
        "name": terminal_capped(&item.name),
        "expected": opt_text(item.expected.as_deref()),
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
            json!({"deferred": 0, "acknowledged_files": 0, "todos": 0, "noise": 0}),
            Vec::new(),
        ),
        None => (
            Value::Null,
            json!({"version": opt_text(entry.milestone_version.as_deref().filter(|v| !v.trim().is_empty()))}),
            entry
                .digest
                .groups
                .iter()
                .filter(|group| filters.lists(group.lane))
                .map(group_json)
                .collect(),
            json!({
                "deferred": entry.digest.hidden.deferred,
                "acknowledged_files": entry.digest.hidden.acknowledged_files,
                "todos": entry.todos_count,
                "noise": entry.digest.hidden.noise,
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
            "decide": 0,
            "fix_phases": sum(|d| d.fix_phases),
            "blocked": sum(|d| d.blocked),
            "closeable_phases": sum(|d| d.closeable_phases),
            "archived": {
                "test": sum(|d| d.archived.test),
                "blocked": sum(|d| d.archived.blocked),
                "fix": sum(|d| d.archived.fix),
            },
            "deferred": sum(|d| d.hidden.deferred),
            "acknowledged_files": sum(|d| d.hidden.acknowledged_files),
            "superseded": sum(|d| d.superseded),
            "noise": sum(|d| d.hidden.noise),
        },
        "projects": entries.iter().map(|entry| project_json(entry, filters)).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{Category, GroupCounts, Kind};

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
}
