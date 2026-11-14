//! Fixture-level proofs of the owner's numbers (Phase 34, plan 05, D-11, D-14).
//!
//! Every test copies a committed fixture's `.planning/` into a TempDir, reads it with
//! the lib at a fixed `now`, and asserts exact numbers. Ages come from each file's
//! `updated:` value, so nothing depends on checkout mtimes.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use gsd_meta_manager::inbox::{build_document, render_text, Filters, ProjectEntry};
use gsd_meta_manager::state_reader::owner_items::{read_owner_digest, Lane, OwnerDigest};
use gsd_meta_manager::state_reader::parse_project_state;
use serde_json::Value;
use tempfile::TempDir;

fn now() -> DateTime<Utc> {
    "2026-11-14T00:00:00Z".parse().unwrap()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fixture copied into a TempDir (the guard keeps it alive).
fn copy_fixture(name: &str) -> (TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner")
        .join(name)
        .join(".planning");
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&src, &dir.path().join(".planning"));
    let root = dir.path().to_path_buf();
    (dir, root)
}

fn entry_of(name: &str, alias: &str) -> (TempDir, ProjectEntry) {
    let (guard, root) = copy_fixture(name);
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, now(), 30);
    let progress = state.phase_progress();
    let entry = ProjectEntry {
        target: None,
        alias: Some(alias.to_string()),
        path: Some(PathBuf::from(format!("/fixture/{alias}"))),
        error: None,
        milestone_version: Some(state.milestone.clone()),
        phases_done: progress.done,
        phases_total: progress.total,
        digest,
        todos: Vec::new(),
        todos_count: 0,
    };
    (guard, entry)
}

fn digest_of(name: &str) -> (TempDir, OwnerDigest) {
    let (guard, entry) = entry_of(name, "x");
    (guard, entry.digest)
}

fn include_all() -> Filters {
    Filters {
        lanes: Vec::new(),
        archived: true,
        deferred: true,
        acknowledged: true,
        todos: true,
    }
}

fn groups(doc: &Value) -> &Vec<Value> {
    doc["projects"][0]["groups"].as_array().unwrap()
}

fn count_items(digest: &OwnerDigest, lane: Lane) -> usize {
    digest
        .groups
        .iter()
        .filter(|g| {
            g.lane == lane && g.scope == gsd_meta_manager::state_reader::owner_items::Scope::Current
        })
        .map(|g| g.items.len())
        .sum()
}

#[test]
fn supersede_six_phases_reports_eighteen_checks() {
    let (_g, d) = digest_of("supersede-six-phases");
    assert_eq!(d.test, 12);
    assert_eq!(d.blocked, 5);
    assert_eq!(d.fix_phases, 1);
    assert_eq!(d.hidden.noise, 2);
    assert_eq!(d.superseded, 21);
    assert_eq!(d.closeable_phases, 3);
    let fix: Vec<_> = d.groups.iter().filter(|g| g.lane == Lane::Fix).collect();
    assert_eq!(fix.len(), 1);
    assert_eq!(fix[0].items.len(), 1);
    assert_eq!(fix[0].phase, "25");
    assert!(fix[0].file.ends_with("25-UAT.md"), "{}", fix[0].file);
    assert_eq!(
        count_items(&d, Lane::Test) + count_items(&d, Lane::Blocked) + count_items(&d, Lane::Fix),
        18
    );
    let mut closing: Vec<&str> = d
        .groups
        .iter()
        .filter(|g| g.closes_phase)
        .map(|g| {
            assert_eq!(g.lane, Lane::Test);
            g.phase.as_str()
        })
        .collect();
    closing.sort_unstable();
    assert_eq!(closing, ["21", "22", "24"]);
}

#[test]
fn gaps_merge_reports_ten_failed_checks_and_fourteen_gaps() {
    let (_g, e) = entry_of("uat-gaps-merge", "gaps");
    let doc = build_document(&[e], &Filters::default());
    let groups = groups(&doc);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["lane"], "fix");
    assert_eq!(
        groups[0]["counts"],
        serde_json::json!({"checks": 10, "failed": 10, "gaps": 14, "unreadable": 0})
    );
    let merged: usize = groups[0]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["gaps"].as_array().unwrap().len())
        .sum();
    assert_eq!(merged, 14);
}

#[test]
fn acknowledged_files_are_counted_and_listed_only_on_request() {
    let (_g, e) = entry_of("uat-ack-match", "ack");
    let by_default = build_document(std::slice::from_ref(&e), &Filters::default());
    assert!(groups(&by_default).is_empty());
    assert_eq!(by_default["projects"][0]["hidden"]["acknowledged_files"], 1);
    let asked = Filters {
        acknowledged: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked);
    assert_eq!(groups(&doc).len(), 1);
    assert_eq!(groups(&doc)[0]["acknowledged"], true);
    for name in [
        "uat-ack-count-changed",
        "uat-ack-status-changed",
        "uat-ack-missing-at",
    ] {
        let (_g, e) = entry_of(name, "x");
        let doc = build_document(&[e], &Filters::default());
        assert!(!groups(&doc).is_empty(), "{name} is listed by default");
        assert_eq!(
            doc["projects"][0]["hidden"]["acknowledged_files"], 0,
            "{name}"
        );
        assert!(
            groups(&doc).iter().all(|g| g["acknowledged"] == false),
            "{name}"
        );
    }
}

#[test]
fn archived_milestones_are_old_debt_only() {
    let (_g, e) = entry_of("archived-complete-skipped", "old");
    assert_eq!(e.digest.test, 0);
    assert_eq!(e.digest.archived.test, 10);
    assert_eq!(e.digest.archived.blocked, 4);
    let plain = build_document(std::slice::from_ref(&e), &Filters::default());
    assert!(groups(&plain).iter().all(|g| g["scope"] == "current"));
    assert_eq!(plain["summary"]["archived"]["test"], 10);
    assert_eq!(plain["summary"]["archived"]["blocked"], 4);
    let asked = Filters {
        archived: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked);
    let old: Vec<&Value> = groups(&doc)
        .iter()
        .filter(|g| g["scope"] == "archived")
        .collect();
    assert_eq!(old.len(), 8);
    for group in old {
        assert_eq!(group["archived_milestone"], "v1.5");
        assert!(group["command"].is_null());
    }
}

#[test]
fn deferred_items_are_a_count_only() {
    let (_g, e) = entry_of("deferred-200", "deferred");
    assert_eq!(e.digest.hidden.deferred, 200);
    let plain = build_document(std::slice::from_ref(&e), &Filters::default());
    assert_eq!(plain["summary"]["deferred"], 200);
    assert_eq!(plain["projects"][0]["hidden"]["deferred"], 200);
    assert!(groups(&plain).is_empty());
    let asked = Filters {
        deferred: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked);
    assert_eq!(groups(&doc).len(), 1);
    assert_eq!(groups(&doc)[0]["lane"], "deferred");
    assert_eq!(groups(&doc)[0]["items"].as_array().unwrap().len(), 200);
}

#[test]
fn source_mismatch_shows_both() {
    let (_g, d) = digest_of("source-mismatch");
    let test: Vec<_> = d.groups.iter().filter(|g| g.lane == Lane::Test).collect();
    assert_eq!(test.len(), 2);
    let counts: Vec<(&str, usize)> = test
        .iter()
        .map(|g| (g.kind.as_str(), g.items.len()))
        .collect();
    assert_eq!(counts, [("uat", 3), ("verification", 2)]);
    assert_eq!(d.superseded, 0);
}

#[test]
fn gaps_found_is_never_superseded() {
    let (_g, d) = digest_of("verification-gaps-found");
    let fix: Vec<_> = d.groups.iter().filter(|g| g.lane == Lane::Fix).collect();
    assert_eq!(fix.len(), 1);
    assert_eq!(fix[0].kind.as_str(), "verification");
    assert_eq!(fix[0].items.len(), 8);
    assert_eq!(d.superseded, 0);
}

#[test]
fn out_of_set_verification_is_unreadable_checks() {
    let (_g, e) = entry_of("verification-out-of-set", "oos");
    let doc = build_document(&[e], &Filters::default());
    let fix: Vec<&Value> = groups(&doc).iter().filter(|g| g["lane"] == "fix").collect();
    assert_eq!(fix.len(), 1);
    assert_eq!(fix[0]["items"], serde_json::json!([]));
    assert_eq!(fix[0]["counts"]["unreadable"], 1);
    assert!(fix[0]["command"].is_null());
    assert!(groups(&doc)
        .iter()
        .any(|g| g["lane"] == "test" && g["kind"] == "uat"));
}

#[test]
fn no_phases_is_empty_not_an_error() {
    let (_g, e) = entry_of("no-phases", "none");
    let doc = build_document(&[e], &Filters::default());
    assert!(doc["projects"][0]["error"].is_null());
    assert!(groups(&doc).is_empty());
}

#[test]
fn testing_uat_counts_as_owner_work() {
    let (_g, e) = entry_of("uat-testing-quoted", "q");
    let doc = build_document(&[e], &Filters::default());
    let test: Vec<&Value> = groups(&doc)
        .iter()
        .filter(|g| g["lane"] == "test")
        .collect();
    assert_eq!(test.len(), 1);
    assert_eq!(test[0]["items"].as_array().unwrap().len(), 3);
    assert_eq!(test[0]["file_status"], "testing");
}

#[test]
fn workstreams_carry_their_workstream() {
    let (_g, e) = entry_of("workstreams", "ws");
    let asked = Filters {
        archived: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked);
    let alpha = groups(&doc)
        .iter()
        .find(|g| g["workstream"] == "alpha" && g["scope"] == "current")
        .expect("the alpha group");
    assert!(alpha["command"].as_str().unwrap().ends_with(" --ws alpha"));
    let old = groups(&doc)
        .iter()
        .find(|g| g["archived_milestone"] == "alpha/v0.1")
        .expect("the alpha archived group");
    assert!(old["command"].is_null());
}

// --- Filters -----------------------------------------------------------------

fn golden_entries() -> Vec<(TempDir, ProjectEntry)> {
    [
        ("supersede-six-phases", "six"),
        ("uat-gaps-merge", "gaps"),
        ("archived-complete-skipped", "old"),
        ("deferred-200", "deferred"),
        ("uat-ack-match", "ack"),
        ("workstreams", "ws"),
    ]
    .into_iter()
    .map(|(name, alias)| entry_of(name, alias))
    .collect()
}

fn assert_golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner_golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        expected == actual,
        "golden mismatch: {}\n--- expected\n{expected}\n--- actual\n{actual}",
        path.display()
    );
}

#[test]
fn the_default_checklist_matches_the_golden() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    assert_golden("inbox.txt", &render_text(&entries, &Filters::default()));
}

#[test]
fn the_include_all_checklist_matches_the_golden() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    assert_golden(
        "inbox-include-all.txt",
        &render_text(&entries, &include_all()),
    );
}

#[test]
fn lane_filters_narrow_both_modes_and_never_the_summary() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    let all_doc = build_document(&entries, &Filters::default());
    let test_only = Filters {
        lanes: vec![Lane::Test],
        ..Default::default()
    };
    let doc = build_document(&entries, &test_only);
    assert_eq!(doc["summary"], all_doc["summary"]);
    for (project, all) in doc["projects"]
        .as_array()
        .unwrap()
        .iter()
        .zip(all_doc["projects"].as_array().unwrap())
    {
        assert_eq!(project["hidden"], all["hidden"]);
        for group in project["groups"].as_array().unwrap() {
            assert_eq!(group["lane"], "test");
        }
    }
    let text = render_text(&entries, &test_only);
    assert!(text.contains("TEST — closes a phase"));
    assert!(!text.contains("FIX — code needed"));
    assert!(!text.contains("BLOCKED\n"));
    let rest = Filters {
        lanes: vec![Lane::Fix, Lane::Blocked],
        ..Default::default()
    };
    let text = render_text(&entries, &rest);
    assert!(!text.contains("TEST — "));
    assert!(text.contains("FIX — code needed") && text.contains("BLOCKED"));
    let first = |t: &str| t.lines().next().unwrap().to_string();
    assert_eq!(
        first(&text),
        first(&render_text(&entries, &Filters::default()))
    );
}

#[test]
fn archived_include_adds_an_old_debt_section() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    let plain = render_text(&entries, &Filters::default());
    assert!(!plain.contains("OLD DEBT\n"));
    assert!(plain.contains("Old debt: "));
    let asked = Filters {
        archived: true,
        ..Default::default()
    };
    let text = render_text(&entries, &asked);
    assert!(text.contains("\nOLD DEBT\n"));
    assert!(!text.contains("Old debt: "));
}

#[test]
fn closing_groups_sort_before_open_ones_inside_test() {
    let (_g, e) = entry_of("supersede-six-phases", "six");
    let text = render_text(&[e], &Filters::default());
    let closes = text.find("TEST — closes a phase").unwrap();
    let stays = text.find("TEST — phase stays open").unwrap();
    assert!(closes < stays);
    let block = &text[closes..stays];
    let (p21, p22, p24) = (
        block.find("ph 21").unwrap(),
        block.find("ph 22").unwrap(),
        block.find("ph 24").unwrap(),
    );
    assert!(p21 < p22 && p22 < p24, "{block}");
}
