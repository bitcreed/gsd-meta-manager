//! `gsd-meta-manager inbox --json`: binary-level proofs (Phase 34, plan 01).
//!
//! The tracer test runs the shipped binary over the committed fixture
//! `tests/fixtures/owner/uat-pending`; the rest build small trees or registries in
//! temp dirs. Ages depend on the wall clock, so no test pins `age_days`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner")
        .join(name)
}

/// A version-2 registry file naming the given (alias, path) pairs.
fn registry(scratch: &Path, projects: &[(&str, &Path)]) -> PathBuf {
    let path = scratch.join("registry.json");
    let map: serde_json::Map<String, Value> = projects
        .iter()
        .map(|(alias, root)| {
            (
                (*alias).to_string(),
                serde_json::json!({"path": root, "added": "2026-01-01T00:00:00Z"}),
            )
        })
        .collect();
    std::fs::write(
        &path,
        serde_json::json!({"version": 2, "projects": map}).to_string(),
    )
    .unwrap();
    path
}

fn inbox_with(registry: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("--config")
        .arg(registry)
        .arg("inbox")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("the binary runs")
}

/// Run `inbox` against an EMPTY registry, so the developer's real one is never read.
fn inbox(args: &[&str]) -> Output {
    let scratch = tempfile::tempdir().unwrap();
    let reg = registry(scratch.path(), &[]);
    inbox_with(&reg, scratch.path(), args)
}

fn json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "exactly one line on stdout: {stdout:?}"
    );
    serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("stdout is JSON: {e}: {stdout}"))
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("an object: {value}"))
        .keys()
        .map(String::as_str)
        .collect()
}

fn set(names: &[&'static str]) -> BTreeSet<&'static str> {
    names.iter().copied().collect()
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A readable project with no phases.
fn bare_project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join(".planning/ROADMAP.md"),
        "# Roadmap\n\n## Phases\n\n- [ ] **Phase 1: Alpha** - a\n\n### Phase 1: Alpha\n\n**Goal**: a\n**Depends on**: Nothing\n",
    );
    write(
        &dir.path().join(".planning/STATE.md"),
        "---\nmilestone: v1.0\nstatus: in_progress\n---\n\n# State\n",
    );
    dir
}

fn tracer() -> (TempDir, PathBuf) {
    let scratch = tempfile::tempdir().unwrap();
    let root = fixture("uat-pending");
    let reg = registry(scratch.path(), &[("tracer", &root)]);
    (scratch, reg)
}

#[test]
fn the_tracer_fixture_lists_one_test_group_and_one_blocked_group() {
    let (scratch, reg) = tracer();
    let out = inbox_with(&reg, scratch.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stderr.is_empty(), "nothing on stderr: {out:?}");
    let doc = json(&out);
    assert_eq!(doc["schema"], "gsd-meta-manager/inbox");
    assert_eq!(doc["schema_version"], 1);

    let projects = doc["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    let groups = projects[0]["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2, "{groups:?}");

    let test = &groups[0];
    assert_eq!(test["lane"], "test");
    assert_eq!(test["phase"], "01");
    assert_eq!(test["command"], "/gsd-verify-work 01");
    let tests: Vec<(&str, &str)> = test["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["test"].as_str().unwrap(), i["category"].as_str().unwrap()))
        .collect();
    assert_eq!(tests, [("2", "pending"), ("3", "pending")]);

    let blocked = &groups[1];
    assert_eq!(blocked["lane"], "blocked");
    assert!(blocked["command"].is_null());
    let items = blocked["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["test"], "4");
    assert_eq!(items[0]["category"], "device_needed");

    let summary = &doc["summary"];
    assert_eq!(summary["test"], 2);
    assert_eq!(summary["blocked"], 1);
    assert_eq!(summary["decide"], 0);
    assert_eq!(summary["projects"], 1);
}

#[test]
fn a_debug_session_awaiting_confirmation_is_a_decide_row() {
    let scratch = tempfile::tempdir().unwrap();
    let root = fixture("debug-awaiting");
    let reg = registry(scratch.path(), &[("dbg", &root)]);

    let out = inbox_with(&reg, scratch.path(), &["--json", "--lane", "decide"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    let groups = doc["projects"][0]["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1, "{groups:?}");
    let group = &groups[0];
    assert_eq!(group["lane"], "decide");
    assert_eq!(group["kind"], "debug");
    assert_eq!(group["phase"], "");
    assert_eq!(group["phase_name"], "alpha-awaiting");
    assert_eq!(group["file"], ".planning/debug/alpha-awaiting.md");
    assert_eq!(group["file_status"], "awaiting_human_verify");
    assert_eq!(group["command"], "/gsd-debug");
    let items = group["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Alpha screen stays blank");
    assert_eq!(doc["summary"]["decide"], 1);
    assert_eq!(doc["summary"]["fix_debug"], 1);

    // Without a lane filter the open investigation is a Fix group with the same command.
    let all = json(&inbox_with(&reg, scratch.path(), &["--json"]));
    let fix: Vec<&Value> = all["projects"][0]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["lane"] == "fix")
        .collect();
    assert_eq!(fix.len(), 1, "{fix:?}");
    assert_eq!(fix[0]["phase_name"], "bravo-root-cause");
    assert_eq!(fix[0]["command"], "/gsd-debug");
    assert_eq!(all["summary"]["fix_phases"], 0);

    let text = inbox_with(&reg, scratch.path(), &["--lane", "decide"]);
    let text = String::from_utf8_lossy(&text.stdout).into_owned();
    let row = text
        .lines()
        .find(|l| l.contains("alpha-awaiting"))
        .unwrap_or_else(|| panic!("a DECIDE row: {text}"));
    assert!(text.contains("DECIDE"), "{text}");
    assert!(
        row.contains("debug") && row.contains("awaiting your confirmation"),
        "{row}"
    );
    assert!(row.contains("/gsd-debug"), "{row}");
    assert!(text.lines().next().unwrap().contains("decide 1"), "{text}");
    assert!(text.lines().next().unwrap().contains("+ 1 debug"), "{text}");
}

#[test]
fn every_entry_group_and_item_carries_the_schema_v1_keys() {
    let (scratch, reg) = tracer();
    let doc = json(&inbox_with(
        &reg,
        scratch.path(),
        &["--json", "tracer", "no-such-project"],
    ));
    assert_eq!(
        keys(&doc),
        set(&["schema", "schema_version", "summary", "projects"])
    );
    assert_eq!(
        keys(&doc["summary"]),
        set(&[
            "projects",
            "test",
            "decide",
            "fix_phases",
            "fix_debug",
            "blocked",
            "closeable_phases",
            "archived",
            "deferred",
            "acknowledged_files",
            "superseded",
            "noise",
        ])
    );
    assert_eq!(
        keys(&doc["summary"]["archived"]),
        set(&["test", "blocked", "fix", "decide"])
    );
    let project_keys = set(&[
        "target",
        "alias",
        "path",
        "error",
        "milestone",
        "groups",
        "hidden",
        "todos",
    ]);
    let hidden_keys = set(&["deferred", "acknowledged_files", "todos", "noise"]);
    let projects = doc["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    for project in projects {
        assert_eq!(keys(project), project_keys);
        assert_eq!(keys(&project["hidden"]), hidden_keys);
    }
    assert_eq!(keys(&projects[0]["milestone"]), set(&["version"]));

    let group_keys = set(&[
        "lane",
        "kind",
        "scope",
        "archived_milestone",
        "workstream",
        "phase",
        "phase_name",
        "file",
        "file_status",
        "closes_phase",
        "bookkeeping_only",
        "acknowledged",
        "age_days",
        "stale",
        "command",
        "supersedes",
        "counts",
        "items",
    ]);
    let item_keys = set(&[
        "id",
        "test",
        "name",
        "expected",
        "result",
        "category",
        "reason",
        "blocked_by",
        "gaps",
    ]);
    for group in projects[0]["groups"].as_array().unwrap() {
        assert_eq!(keys(group), group_keys);
        assert_eq!(
            keys(&group["counts"]),
            set(&["checks", "failed", "gaps", "unreadable"])
        );
        for item in group["items"].as_array().unwrap() {
            assert_eq!(keys(item), item_keys);
        }
    }

    let error = &projects[1];
    assert_eq!(error["error"]["kind"], "unresolved");
    assert!(error["milestone"].is_null());
    assert_eq!(error["groups"], serde_json::json!([]));
    assert_eq!(error["todos"], serde_json::json!([]));
    assert_eq!(
        error["hidden"],
        serde_json::json!({"deferred": 0, "acknowledged_files": 0, "todos": 0, "noise": 0})
    );
}

#[test]
fn an_empty_registry_prints_an_empty_document() {
    let out = inbox(&["--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    assert_eq!(doc["projects"], serde_json::json!([]));
    for key in [
        "projects",
        "test",
        "decide",
        "fix_phases",
        "blocked",
        "noise",
    ] {
        assert_eq!(doc["summary"][key], 0, "{key}");
    }
}

#[test]
fn an_unresolved_name_is_an_error_entry_and_exits_zero() {
    let out = inbox(&["--json", "no-such-project"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    let entry = &doc["projects"][0];
    assert_eq!(entry["target"], "no-such-project");
    assert_eq!(entry["error"]["kind"], "unresolved");
    assert!(entry["error"]["message"].is_string());
    assert_eq!(doc["summary"]["projects"], 1);
}

#[test]
fn a_registered_folder_that_is_gone_or_has_no_planning_dir_is_an_error_entry() {
    let scratch = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let gone = scratch.path().join("gone");
    let reg = registry(scratch.path(), &[("bare", bare.path()), ("gone", &gone)]);
    let doc = json(&inbox_with(&reg, scratch.path(), &["--json"]));
    let kinds: Vec<(&str, &str)> = doc["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["alias"].as_str().unwrap(),
                p["error"]["kind"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(kinds, [("bare", "no_planning"), ("gone", "folder_missing")]);
}

#[test]
fn a_name_given_twice_appears_twice_in_argv_order() {
    let a = bare_project();
    let b = bare_project();
    let scratch = tempfile::tempdir().unwrap();
    let reg = registry(scratch.path(), &[("a", a.path()), ("b", b.path())]);
    let doc = json(&inbox_with(
        &reg,
        scratch.path(),
        &["--json", "b", "a", "b"],
    ));
    let aliases: Vec<&str> = doc["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["alias"].as_str().unwrap())
        .collect();
    assert_eq!(aliases, ["b", "a", "b"]);
    let doc = json(&inbox_with(&reg, scratch.path(), &["--json"]));
    let aliases: Vec<&str> = doc["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["alias"].as_str().unwrap())
        .collect();
    assert_eq!(aliases, ["a", "b"], "registry order without names");
}

#[test]
fn two_runs_print_identical_bytes() {
    let (scratch, reg) = tracer();
    let first = inbox_with(&reg, scratch.path(), &["--json"]).stdout;
    let second = inbox_with(&reg, scratch.path(), &["--json"]).stdout;
    assert!(!first.is_empty());
    assert_eq!(first, second);
}

#[test]
fn a_lane_filter_lists_only_that_lane_and_keeps_the_summary() {
    let (scratch, reg) = tracer();
    let doc = json(&inbox_with(
        &reg,
        scratch.path(),
        &["--json", "--lane", "blocked"],
    ));
    let groups = doc["projects"][0]["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["lane"], "blocked");
    assert_eq!(doc["summary"]["test"], 2);
}

#[test]
fn an_unreadable_registry_exits_one_with_one_inbox_line() {
    let scratch = tempfile::tempdir().unwrap();
    let reg = scratch.path().join("registry.json");
    std::fs::write(&reg, "{ this is not json").unwrap();
    let out = inbox_with(&reg, scratch.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty(), "nothing on stdout: {out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with("inbox: "), "{err:?}");
    assert_eq!(
        err.trim_end().lines().count(),
        1,
        "one stderr line: {err:?}"
    );
}

#[test]
fn an_unknown_lane_is_a_usage_error() {
    let out = inbox(&["--json", "--lane", "sideways"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

/// (path, size, mtime) of every file under `dir`, sorted.
fn snapshot(dir: &Path) -> Vec<(String, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, out: &mut Vec<(String, u64, std::time::SystemTime)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                walk(&entry.path(), out);
            } else {
                out.push((
                    entry.path().display().to_string(),
                    meta.len(),
                    meta.modified().unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out
}

#[test]
fn inbox_writes_nothing_anywhere() {
    let scratch = tempfile::tempdir().unwrap();
    let root = fixture("uat-pending");
    let reg = registry(scratch.path(), &[("tracer", &root)]);
    let registry_before = std::fs::read(&reg).unwrap();
    let tree_before = snapshot(&root);

    let homes = tempfile::tempdir().unwrap();
    let mut env_dirs = Vec::new();
    for name in ["home", "data", "state", "config"] {
        let path = homes.path().join(name);
        std::fs::create_dir(&path).unwrap();
        env_dirs.push((name, path));
    }
    let dir_of = |name: &str| {
        env_dirs
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| p.clone())
            .unwrap()
    };
    let out = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("--config")
        .arg(&reg)
        .args(["inbox", "--json"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", dir_of("home"))
        .env("XDG_DATA_HOME", dir_of("data"))
        .env("XDG_STATE_HOME", dir_of("state"))
        .env("XDG_CONFIG_HOME", dir_of("config"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(json(&out)["projects"].as_array().unwrap().len(), 1);

    for (name, path) in &env_dirs {
        let entries: Vec<_> = std::fs::read_dir(path).unwrap().collect();
        assert!(entries.is_empty(), "{name} dir was written to: {entries:?}");
    }
    assert_eq!(registry_before, std::fs::read(&reg).unwrap());
    assert_eq!(tree_before, snapshot(&root));
}

#[test]
fn an_acknowledged_uat_is_counted_and_not_listed() {
    let scratch = tempfile::tempdir().unwrap();
    let root = fixture("uat-ack-match");
    let reg = registry(scratch.path(), &[("ack", &root)]);
    let out = inbox_with(&reg, scratch.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    let project = &doc["projects"][0];
    assert_eq!(project["hidden"]["acknowledged_files"], 1);
    assert_eq!(doc["summary"]["acknowledged_files"], 1);
    assert_eq!(doc["summary"]["test"], 0);
    assert!(
        project["groups"].as_array().unwrap().is_empty(),
        "{project:?}"
    );
}

#[test]
fn a_changed_acknowledgment_marker_resurfaces_the_file() {
    for name in [
        "uat-ack-count-changed",
        "uat-ack-status-changed",
        "uat-ack-missing-at",
    ] {
        let scratch = tempfile::tempdir().unwrap();
        let root = fixture(name);
        let reg = registry(scratch.path(), &[("ack", &root)]);
        let doc = json(&inbox_with(&reg, scratch.path(), &["--json"]));
        assert_eq!(doc["summary"]["acknowledged_files"], 0, "{name}");
        assert_eq!(
            doc["projects"][0]["groups"].as_array().unwrap().len(),
            1,
            "{name}"
        );
    }
}

#[test]
fn plain_inbox_prints_the_checklist_and_exits_zero() {
    let (scratch, reg) = tracer();
    let out = inbox_with(&reg, scratch.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("Owner inbox: 1 projects"), "{text}");
    assert!(text.contains("\nTEST — phase stays open\n"), "{text}");
    assert!(text.contains("    - [ ] "), "{text}");
    assert!(!text.contains('\u{1b}'), "no color: {text:?}");
    assert!(serde_json::from_str::<Value>(text.trim()).is_err());
}

#[test]
fn a_lane_filter_lists_only_that_lane_in_both_modes() {
    let (scratch, reg) = tracer();
    let doc = json(&inbox_with(
        &reg,
        scratch.path(),
        &["--json", "--lane", "test"],
    ));
    let groups = doc["projects"][0]["groups"].as_array().unwrap();
    assert!(!groups.is_empty());
    assert!(groups.iter().all(|g| g["lane"] == "test"), "{groups:?}");
    let text =
        String::from_utf8(inbox_with(&reg, scratch.path(), &["--lane", "test"]).stdout).unwrap();
    assert!(text.contains("TEST — "), "{text}");
    assert!(!text.contains("BLOCKED\n"), "{text}");
}

#[test]
fn include_todos_fills_the_todo_list_and_only_then() {
    let project = bare_project();
    write(
        &project
            .path()
            .join(".planning/todos/pending/2026-01-01-first.md"),
        "# First todo\n\nbody\n",
    );
    let scratch = tempfile::tempdir().unwrap();
    let reg = registry(scratch.path(), &[("p", project.path())]);
    let without = json(&inbox_with(&reg, scratch.path(), &["--json"]));
    assert_eq!(without["projects"][0]["todos"], serde_json::json!([]));
    assert_eq!(without["projects"][0]["hidden"]["todos"], 1);
    let with = json(&inbox_with(
        &reg,
        scratch.path(),
        &["--json", "--include", "todos"],
    ));
    let todos = with["projects"][0]["todos"].as_array().unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0]["title"], "First todo");
    let text = String::from_utf8(inbox_with(&reg, scratch.path(), &[]).stdout).unwrap();
    assert!(
        text.contains("Triage: 1 pending todos in 1 projects"),
        "{text}"
    );
}

#[test]
fn a_bogus_lane_is_a_usage_error_in_text_mode_too() {
    let out = inbox(&["--lane", "bogus"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

// ── The README contract (OWN-13) ──

const README: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));

/// Every key path of the schema v1 document: `.` between object keys, `[]` for
/// an array element (the same notation as the README table).
const INBOX_PATHS: &[&str] = &[
    "schema",
    "schema_version",
    "summary",
    "summary.projects",
    "summary.test",
    "summary.decide",
    "summary.fix_phases",
    "summary.blocked",
    "summary.closeable_phases",
    "summary.archived",
    "summary.archived.test",
    "summary.archived.blocked",
    "summary.archived.fix",
    "summary.archived.decide",
    "summary.deferred",
    "summary.acknowledged_files",
    "summary.superseded",
    "summary.noise",
    "projects",
    "projects[].target",
    "projects[].alias",
    "projects[].path",
    "projects[].error",
    "projects[].error.kind",
    "projects[].error.message",
    "projects[].milestone",
    "projects[].milestone.version",
    "projects[].groups",
    "projects[].groups[].lane",
    "projects[].groups[].kind",
    "projects[].groups[].scope",
    "projects[].groups[].archived_milestone",
    "projects[].groups[].workstream",
    "projects[].groups[].phase",
    "projects[].groups[].phase_name",
    "projects[].groups[].file",
    "projects[].groups[].file_status",
    "projects[].groups[].closes_phase",
    "projects[].groups[].bookkeeping_only",
    "projects[].groups[].acknowledged",
    "projects[].groups[].age_days",
    "projects[].groups[].stale",
    "projects[].groups[].command",
    "projects[].groups[].supersedes",
    "projects[].groups[].counts",
    "projects[].groups[].counts.checks",
    "projects[].groups[].counts.failed",
    "projects[].groups[].counts.gaps",
    "projects[].groups[].counts.unreadable",
    "projects[].groups[].items",
    "projects[].groups[].items[].id",
    "projects[].groups[].items[].test",
    "projects[].groups[].items[].name",
    "projects[].groups[].items[].expected",
    "projects[].groups[].items[].result",
    "projects[].groups[].items[].category",
    "projects[].groups[].items[].reason",
    "projects[].groups[].items[].blocked_by",
    "projects[].groups[].items[].gaps",
    "projects[].groups[].items[].gaps[].name",
    "projects[].groups[].items[].gaps[].result",
    "projects[].groups[].items[].gaps[].reason",
    "projects[].hidden",
    "projects[].hidden.deferred",
    "projects[].hidden.acknowledged_files",
    "projects[].hidden.todos",
    "projects[].hidden.noise",
    "projects[].todos",
    "projects[].todos[].file",
    "projects[].todos[].title",
    "summary.fix_debug",
];

fn inbox_set() -> BTreeSet<String> {
    INBOX_PATHS.iter().map(|p| (*p).to_string()).collect()
}

fn key_paths(doc: &Value) -> BTreeSet<String> {
    fn walk(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    out.insert(path.clone());
                    walk(child, &path, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, &format!("{prefix}[]"), out);
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    walk(value_ref(doc), "", &mut out);
    out
}

fn value_ref(value: &Value) -> &Value {
    value
}

/// The body of the first fenced `json` block in `doc` that contains `needle`.
fn json_fence(doc: &str, needle: &str) -> Value {
    let mut rest = doc;
    while let Some(start) = rest.find("```json\n") {
        let body_start = start + "```json\n".len();
        let body_len = rest[body_start..]
            .find("\n```")
            .expect("a json fence is closed");
        let body = &rest[body_start..body_start + body_len];
        if body.contains(needle) {
            return serde_json::from_str(body)
                .unwrap_or_else(|e| panic!("the json fence parses: {e}\n{body}"));
        }
        rest = &rest[body_start + body_len..];
    }
    panic!("no json fence containing {needle:?}");
}

/// The README's inbox subsection only: the status table shares paths such as
/// `projects[].alias`, so the rows must be looked up in this slice.
fn inbox_section() -> &'static str {
    let start = README
        .find("#### Owner work: `inbox`")
        .expect("the README has the inbox subsection");
    let rest = &README[start..];
    let end = rest
        .match_indices("\n### ")
        .next()
        .map_or(rest.len(), |(at, _)| at);
    &rest[..end]
}

#[test]
fn the_inbox_key_paths_are_exactly_schema_v1() {
    let scratch = tempfile::tempdir().unwrap();
    let gaps = tempfile::tempdir().unwrap();
    copy_tree(&fixture("uat-gaps-merge"), gaps.path());
    write(
        &gaps
            .path()
            .join(".planning/todos/pending/2026-01-01-first.md"),
        "# First todo\n",
    );
    let six = tempfile::tempdir().unwrap();
    copy_tree(&fixture("supersede-six-phases"), six.path());
    let reg = registry(
        scratch.path(),
        &[("gaps", gaps.path()), ("six", six.path())],
    );
    let doc = json(&inbox_with(
        &reg,
        scratch.path(),
        &[
            "gaps",
            "six",
            "no-such-project",
            "--json",
            "--include",
            "archived,deferred,acknowledged,todos",
        ],
    ));
    // Every array the two fixtures fill is non-empty, so no path is excused.
    assert_eq!(key_paths(&doc), inbox_set());
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths() {
    let doc = json_fence(README, "\"schema\": \"gsd-meta-manager/inbox\"");
    assert_eq!(key_paths(&doc), inbox_set());
}

#[test]
fn the_readme_inbox_table_documents_every_path() {
    let section = inbox_section();
    for path in INBOX_PATHS {
        assert!(
            section.contains(&format!("| `{path}` |")),
            "the README inbox section has no table row for {path}"
        );
    }
    let rows = section.lines().filter(|l| l.starts_with("| `")).count();
    assert_eq!(rows, INBOX_PATHS.len(), "one row per path, no extras");
    assert!(section.contains("project-authored") || section.contains("project data"));
}

// ── Hostile text and cost ──

const ESC: char = '\u{1b}';
const RLO: char = '\u{202e}';
const BEL: char = '\u{7}';

fn strings_of(value: &Value, out: &mut Vec<(String, String)>, key: &str) {
    match value {
        Value::String(text) => out.push((key.to_string(), text.clone())),
        Value::Array(items) => items.iter().for_each(|v| strings_of(v, out, key)),
        Value::Object(map) => map.iter().for_each(|(k, v)| strings_of(v, out, k)),
        _ => {}
    }
}

#[test]
fn hostile_strings_are_escaped_in_json_and_text() {
    let project = tempfile::tempdir().unwrap();
    let long = "A".repeat(500);
    write(
        &project.path().join(".planning/ROADMAP.md"),
        &format!(
            "# Roadmap\n\n## Phases\n\n- [ ] **Phase 01: Evil{ESC}[31mred{RLO}name** - x\n\n### Phase 01: Evil{ESC}[31mred{RLO}name\n\n**Goal**: g\n**Depends on**: Nothing\n"
        ),
    );
    write(
        &project.path().join(".planning/STATE.md"),
        "---\nmilestone: v1.0\nstatus: in_progress\n---\n\n# State\n",
    );
    write(
        &project.path().join(".planning/phases/01-evil/01-UAT.md"),
        &format!(
            "---\nstatus: testing\nupdated: 2026-01-10T12:00:00Z\n---\n\n# UAT\n\n## Tests\n\n### 1. Ring{BEL}bell {ESC}]0;pwned{BEL}\nexpected: see {RLO}txt.exe{ESC}[2J\nresult: pending\n\n### 2. {long}\nexpected: {long}\nresult: pending\n\n### 3. Skipped{ESC}[1m\nexpected: x\nresult: skipped\nreason: needs {ESC}[31mdevice{RLO}\n"
        ),
    );
    let scratch = tempfile::tempdir().unwrap();
    let reg = registry(scratch.path(), &[("evil", project.path())]);

    let out = inbox_with(
        &reg,
        scratch.path(),
        &["--json", "--include", "archived,deferred"],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    let mut strings = Vec::new();
    strings_of(&doc, &mut strings, "");
    assert!(strings.len() > 10);
    let mut capped = 0;
    for (key, text) in &strings {
        for bad in [ESC, RLO, BEL] {
            assert!(!text.contains(bad), "{key}: {text:?} carries {bad:?}");
        }
        if key != "path" && key != "file" {
            assert!(
                text.chars().count() <= 201,
                "{key}: {} chars",
                text.chars().count()
            );
        }
        if text.chars().count() == 201 {
            assert!(text.ends_with('…'), "{key}: {text:?}");
            capped += 1;
        }
    }
    assert!(
        capped >= 2,
        "the 500-character name and expectation were cut"
    );

    let text = String::from_utf8(inbox_with(&reg, scratch.path(), &[]).stdout).unwrap();
    for bad in [ESC, RLO, BEL] {
        assert!(
            !text.contains(bad),
            "plain output carries {bad:?}: {text:?}"
        );
    }
    for line in text.lines() {
        assert!(line.chars().count() <= 1000, "bounded line: {}", line.len());
    }
    assert!(text.contains("- [ ] "), "{text}");
}

fn generate_project(root: &Path, index: usize) {
    write(
        &root.join(".planning/STATE.md"),
        "---\nmilestone: v2.0\nstatus: in_progress\n---\n\n# State\n",
    );
    let mut roadmap = String::from("# Roadmap\n\n## Phases\n\n");
    for n in 1..=15 {
        roadmap.push_str(&format!("- [ ] **Phase {n:02}: Part {n}** - x\n"));
    }
    roadmap.push('\n');
    for n in 1..=15 {
        roadmap.push_str(&format!(
            "### Phase {n:02}: Part {n}\n\n**Goal**: g\n**Depends on**: Nothing\n\n"
        ));
    }
    write(&root.join(".planning/ROADMAP.md"), &roadmap);
    let results = ["pending", "pass", "issue", "skipped", "blocked"];
    for n in 1..=15 {
        let dir = root.join(format!(".planning/phases/{n:02}-part-{n}"));
        let mut uat = String::from(
            "---\nstatus: testing\nupdated: 2026-10-01T12:00:00Z\n---\n\n# UAT\n\n## Tests\n\n",
        );
        for t in 1..=10 {
            let result = results[(t + n + index) % results.len()];
            uat.push_str(&format!(
                "### {t}. Check {t} of phase {n}\nexpected: it works for case {t}\nresult: {result}\n"
            ));
            if result == "issue" || result == "skipped" || result == "blocked" {
                uat.push_str("reason: waits for a device\n");
            }
            if result == "blocked" {
                uat.push_str("blocked_by: device\n");
            }
            uat.push('\n');
        }
        write(&dir.join(format!("{n:02}-UAT.md")), &uat);
        write(
            &dir.join(format!("{n:02}-VERIFICATION.md")),
            "---\nstatus: human_needed\nupdated: 2026-10-01T12:00:00Z\nhuman_verification:\n  - \"look at the first screen\"\n  - \"look at the second screen\"\n  - \"look at the third screen\"\n---\n\n# Verification\n",
        );
    }
    for n in 1..=5 {
        let dir = root.join(format!(".planning/milestones/v1.0-phases/{n:02}-old-{n}"));
        write(
            &dir.join(format!("{n:02}-UAT.md")),
            "---\nstatus: complete\nupdated: 2026-01-01T12:00:00Z\n---\n\n# UAT\n\n## Tests\n\n### 1. Old check\nexpected: e\nresult: pending\n\n### 2. Other old check\nexpected: e\nresult: skipped\nreason: waits for a device\n",
        );
    }
}

/// The wall-clock bound of the cold-run test. The design target is one second, but a
/// loaded CI runner, parallel test binaries and an unoptimized dev profile make that
/// flaky as a pass/fail line, so the default is a generous hang detector. Set
/// `GSD_MM_STRICT_TIMING=1` to enforce the one-second target (on a quiet machine, in
/// a release profile). The adversarial-input proof lives in the deferred-items unit
/// tests, not here.
fn cold_bound() -> std::time::Duration {
    if std::env::var_os("GSD_MM_STRICT_TIMING").is_some_and(|value| value == "1") {
        std::time::Duration::from_secs(1)
    } else {
        std::time::Duration::from_secs(10)
    }
}

#[test]
fn a_cold_inbox_over_twenty_projects_is_under_a_second() {
    let bound = cold_bound();
    let scratch = tempfile::tempdir().unwrap();
    let roots: Vec<TempDir> = (0..20)
        .map(|index| {
            let dir = tempfile::tempdir().unwrap();
            generate_project(dir.path(), index);
            dir
        })
        .collect();
    let named: Vec<(String, PathBuf)> = roots
        .iter()
        .enumerate()
        .map(|(i, dir)| (format!("p{i:02}"), dir.path().to_path_buf()))
        .collect();
    let pairs: Vec<(&str, &Path)> = named
        .iter()
        .map(|(a, p)| (a.as_str(), p.as_path()))
        .collect();
    let reg = registry(scratch.path(), &pairs);

    let started = std::time::Instant::now();
    let out = inbox_with(&reg, scratch.path(), &["--json"]);
    let cold = started.elapsed();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    assert_eq!(doc["summary"]["projects"], 20);
    assert!(doc["summary"]["test"].as_u64().unwrap() > 0);
    assert!(
        cold < bound,
        "a cold `inbox --json` took {cold:?} (bound {bound:?})"
    );

    let started = std::time::Instant::now();
    let mut seen = 0;
    for (_, root) in &named {
        let planning = root.join(".planning");
        let state = gsd_meta_manager::state_reader::parse_project_state(&planning);
        let digest = gsd_meta_manager::state_reader::owner_items::read_owner_digest(
            &planning,
            &state,
            chrono::Utc::now(),
            30,
        );
        seen += digest.test;
    }
    let lib = started.elapsed();
    eprintln!("cold inbox --json: {cold:?}; lib loop: {lib:?}");
    assert!(seen > 0);
    assert!(lib < bound, "the lib loop took {lib:?} (bound {bound:?})");
}
