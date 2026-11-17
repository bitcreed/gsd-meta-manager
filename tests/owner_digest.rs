//! Fixture-level proofs of the owner's numbers (Phase 34, plan 05, D-11, D-14).
//!
//! Every test copies a committed fixture's `.planning/` into a TempDir, reads it with
//! the lib at a fixed `now`, and asserts exact numbers. Ages come from each file's
//! `updated:` value, so nothing depends on checkout mtimes.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use gsd_meta_manager::inbox::{
    build_document, render_digest, render_lanes, render_project, render_text, Filters, ProjectEntry,
};
use gsd_meta_manager::state_reader::owner_items::{
    labels_of, read_owner_digest, triage_entries, Label, Lane, OwnerDigest,
};
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
fn closes_after_counts_the_current_milestone() {
    let (_g, d) = digest_of("closes-after-milestone");
    let gate = &d.closes_after;
    assert_eq!(
        (gate.test, gate.decide, gate.fix, gate.blocked),
        (12, 0, 1, 5)
    );
    assert_eq!(gate.phases_open, ["20", "21", "22", "23", "24", "25"]);
    assert_eq!(gate.not_built, ["26"]);
    // Phase 19 (complete) and phase 27 (next milestone) appear nowhere.
    assert_eq!(gate.lane_phases.fix, ["25"]);
    assert!(!gate.lane_phases.test.iter().any(|p| p == "19" || p == "27"));
    let (_g, entry) = entry_of("closes-after-milestone", "x");
    let doc = build_document(&[entry], &Filters::default(), FOLD_DAYS);
    assert_eq!(
        doc["projects"][0]["milestone"]["closes_after"],
        serde_json::json!({
            "test": 12, "decide": 0, "fix": 1, "blocked": 5,
            "phases_open": ["20", "21", "22", "23", "24", "25"],
            "not_built": ["26"],
        })
    );
}

#[test]
fn the_closes_after_line_follows_the_header() {
    let (_g, entry) = entry_of("closes-after-milestone", "x");
    let text = render_text(&[entry], &Filters::default());
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("Owner inbox:"), "{text}");
    assert_eq!(lines[1], "");
    assert!(
        lines[2].starts_with("  x · v2.2 closes after: test 12 (ph 20 21 22 23 24 25)"),
        "{}",
        lines[2]
    );
    assert!(lines[2].contains(" · unblock 5 (ph "), "{}", lines[2]);
    assert!(lines[2].contains(" · fix 1 (ph 25)"), "{}", lines[2]);
    assert!(lines[2].ends_with(" · not built ph 26"), "{}", lines[2]);
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
    let doc = build_document(&[e], &Filters::default(), FOLD_DAYS);
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
    let by_default = build_document(std::slice::from_ref(&e), &Filters::default(), FOLD_DAYS);
    assert!(groups(&by_default).is_empty());
    assert_eq!(by_default["projects"][0]["hidden"]["acknowledged_files"], 1);
    let asked = Filters {
        acknowledged: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked, FOLD_DAYS);
    assert_eq!(groups(&doc).len(), 1);
    assert_eq!(groups(&doc)[0]["acknowledged"], true);
    for name in [
        "uat-ack-count-changed",
        "uat-ack-status-changed",
        "uat-ack-missing-at",
    ] {
        let (_g, e) = entry_of(name, "x");
        let doc = build_document(&[e], &Filters::default(), FOLD_DAYS);
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
    let plain = build_document(std::slice::from_ref(&e), &Filters::default(), FOLD_DAYS);
    assert!(groups(&plain).iter().all(|g| g["scope"] == "current"));
    assert_eq!(plain["summary"]["archived"]["test"], 10);
    assert_eq!(plain["summary"]["archived"]["blocked"], 4);
    let asked = Filters {
        archived: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked, FOLD_DAYS);
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
    let plain = build_document(std::slice::from_ref(&e), &Filters::default(), FOLD_DAYS);
    assert_eq!(plain["summary"]["deferred"], 200);
    assert_eq!(plain["projects"][0]["hidden"]["deferred"], 200);
    assert!(groups(&plain).is_empty());
    let asked = Filters {
        deferred: true,
        ..Default::default()
    };
    let doc = build_document(&[e], &asked, FOLD_DAYS);
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
    let doc = build_document(&[e], &Filters::default(), FOLD_DAYS);
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
    let doc = build_document(&[e], &Filters::default(), FOLD_DAYS);
    assert!(doc["projects"][0]["error"].is_null());
    assert!(groups(&doc).is_empty());
}

#[test]
fn testing_uat_counts_as_owner_work() {
    let (_g, e) = entry_of("uat-testing-quoted", "q");
    let doc = build_document(&[e], &Filters::default(), FOLD_DAYS);
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
    let doc = build_document(&[e], &asked, FOLD_DAYS);
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
    let all_doc = build_document(&entries, &Filters::default(), FOLD_DAYS);
    let test_only = Filters {
        lanes: vec![Lane::Test],
        ..Default::default()
    };
    let doc = build_document(&entries, &test_only, FOLD_DAYS);
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

#[test]
fn debug_sessions_fill_decide_and_fix() {
    let (_g, e) = entry_of("debug-sessions", "dbg");
    let d = &e.digest;
    assert_eq!(d.decide, 5);
    assert_eq!(d.fix_debug, 4);
    assert_eq!(d.fix_phases, 0);
    assert_eq!(d.hidden.acknowledged_files, 1);

    let slugs = |lane: Lane| -> Vec<&str> {
        let mut names: Vec<&str> = d
            .groups
            .iter()
            .filter(|g| g.lane == lane && !g.acknowledged)
            .map(|g| g.phase_name.as_str())
            .collect();
        names.sort_unstable();
        names
    };
    assert_eq!(
        slugs(Lane::Decide),
        [
            "delta-await",
            "echo-hyphen",
            "foxtrot-upper",
            "golf-recovered",
            "hotel-second"
        ]
    );
    assert_eq!(
        slugs(Lane::Fix),
        ["india-root", "juliet-diagnosed", "kilo-bare", "lima-seven"]
    );
    for ignored in [
        "knowledge-base",
        "old",
        "notes",
        "oscar-complete",
        "mike-trailing",
        "november-resolved",
    ] {
        assert!(
            d.groups.iter().all(|g| g.phase_name != ignored),
            "{ignored} must not be listed"
        );
    }

    // The default listing leaves the acknowledged session out; `include_all` lists it.
    let doc = build_document(std::slice::from_ref(&e), &Filters::default(), FOLD_DAYS);
    assert_eq!(groups(&doc).len(), 9);
    let doc = build_document(std::slice::from_ref(&e), &include_all(), FOLD_DAYS);
    let acked: Vec<&Value> = groups(&doc)
        .iter()
        .filter(|g| g["acknowledged"] == true)
        .collect();
    assert_eq!(acked.len(), 1);
    assert_eq!(acked[0]["phase_name"], "papa-acked");
    assert_eq!(doc["summary"]["decide"], 5);
    assert_eq!(doc["summary"]["fix_debug"], 4);
}

#[test]
fn context_questions_fill_decide() {
    let (_g, e) = entry_of("context-questions", "ctx");
    let d = &e.digest;
    // D-18: 02-alpha counts 5 top-level entries (the bold line folds into the one
    // before it), 2 of them open.
    assert_eq!(d.decide, 5);
    assert_eq!(d.archived.decide, 1);
    assert_eq!(d.hidden.acknowledged_files, 1);

    let doc = build_document(std::slice::from_ref(&e), &include_all(), FOLD_DAYS);
    let listed: Vec<(String, usize, bool)> = groups(&doc)
        .iter()
        .filter(|g| g["kind"] == "context_questions")
        .map(|g| {
            (
                g["file"].as_str().unwrap().to_owned(),
                g["items"].as_array().unwrap().len(),
                g["acknowledged"] == true,
            )
        })
        .collect();
    let find = |suffix: &str| listed.iter().find(|(file, _, _)| file.ends_with(suffix));
    assert_eq!(find("phases/02-alpha/02-CONTEXT.md").map(|g| g.1), Some(2));
    assert_eq!(find("phases/03-bravo/03-CONTEXT.md"), None);
    assert_eq!(
        find("phases/04-charlie/04-CONTEXT.md").map(|g| g.1),
        Some(2)
    );
    assert_eq!(
        find("phases/05-delta/05-CONTEXT.md").map(|g| (g.1, g.2)),
        Some((1, true))
    );
    assert_eq!(find("phases/01-old/01-CONTEXT.md"), None);
    assert_eq!(
        find("ws1/phases/01-stream/01-CONTEXT.md").map(|g| g.1),
        Some(1)
    );
    assert_eq!(
        find("v0.9-phases/00-legacy/00-CONTEXT.md").map(|g| g.1),
        Some(1)
    );

    let archived = groups(&doc)
        .iter()
        .find(|g| g["kind"] == "context_questions" && g["scope"] == "archived")
        .unwrap();
    assert_eq!(archived["archived_milestone"], "v0.9");
    assert_eq!(archived["lane"], "decide");
    assert!(archived["command"].is_null());
    let ws = groups(&doc)
        .iter()
        .find(|g| g["workstream"] == "ws1" && g["kind"] == "context_questions")
        .unwrap();
    assert_eq!(ws["lane"], "decide");
    assert_eq!(doc["summary"]["decide"], 5);
    assert_eq!(doc["summary"]["archived"]["decide"], 1);

    let alpha = groups(&doc)
        .iter()
        .find(|g| {
            g["file"]
                .as_str()
                .is_some_and(|f| f.ends_with("02-CONTEXT.md"))
        })
        .unwrap();
    let ids: Vec<&str> = alpha["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["02-CONTEXT.md#q4", "02-CONTEXT.md#q5"]);

    let text = render_text(std::slice::from_ref(&e), &Filters::default());
    assert!(text.contains("2 open questions in 02-CONTEXT.md"), "{text}");
}

#[test]
fn a_planned_phase_hides_its_open_questions() {
    let (_g, e) = entry_of("context-planned", "plan");
    let d = &e.digest;
    // Phase 02 (3 questions) and the archived phase 00 (1) have plans: counted, not listed.
    assert_eq!(d.hidden.planned_questions, 4);
    assert_eq!((d.decide, d.archived.decide), (2, 0));
    // Phase 02 is built, and its questions no longer gate the milestone.
    assert_eq!(d.closes_after.decide, 0);

    let doc = build_document(std::slice::from_ref(&e), &include_all(), FOLD_DAYS);
    let files: Vec<&str> = groups(&doc)
        .iter()
        .filter(|g| g["kind"] == "context_questions")
        .map(|g| g["file"].as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(files[0].ends_with("phases/03-bravo/03-CONTEXT.md"));
    assert_eq!(doc["summary"]["planned_questions"], 4);
    assert_eq!(doc["summary"]["decide"], 2);
    assert_eq!(doc["summary"]["archived"]["decide"], 0);
    assert_eq!(doc["projects"][0]["hidden"]["planned_questions"], 4);
    assert_eq!(doc["schema_version"], 1);
}

#[test]
fn a_summary_only_phase_counts_as_planned() {
    let (_g, e) = entry_of("context-summary-only", "sum");
    let d = &e.digest;
    assert_eq!(d.hidden.planned_questions, 2);
    assert_eq!(d.decide, 0);
    let doc = build_document(std::slice::from_ref(&e), &include_all(), FOLD_DAYS);
    assert!(groups(&doc)
        .iter()
        .all(|g| g["kind"] != "context_questions"));
    assert_eq!(doc["summary"]["planned_questions"], 2);
}

#[test]
fn open_questions_count_top_level_entries() {
    let (_g, e) = entry_of("context-entries", "ent");
    assert_eq!(e.digest.decide, 3);
    let doc = build_document(std::slice::from_ref(&e), &include_all(), FOLD_DAYS);
    let group = groups(&doc)
        .iter()
        .find(|g| g["kind"] == "context_questions")
        .unwrap();
    let items = group["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        ["02-CONTEXT.md#q1", "02-CONTEXT.md#q2", "02-CONTEXT.md#q3"]
    );
    let names: Vec<&str> = items.iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "- **OQ-01** — Should the export be paged for large lists?",
            "- **OQ-02** — Is the cache keyed per user or per team?",
            "- **OQ-03** — Does the report need a footer?",
        ]
    );
}

fn handoff_groups_of(doc: &Value) -> Vec<&Value> {
    groups(doc)
        .iter()
        .filter(|g| g["kind"] == "handoff")
        .collect()
}

#[test]
fn handoff_paused_is_one_stale_decide_group() {
    let (guard, entry) = entry_of("handoff-paused", "usb");
    // ProjectState judges this handoff stale (StateNewer) and so not paused; the owner
    // reader lists it anyway, flagged (D-05, D-09).
    let state = parse_project_state(&guard.path().join(".planning"));
    assert!(state.stale_handoff.is_some());
    assert!(!state.paused);
    assert_eq!(entry.digest.decide, 1);
    let doc = build_document(std::slice::from_ref(&entry), &include_all(), FOLD_DAYS);
    let found = handoff_groups_of(&doc);
    assert_eq!(found.len(), 1, "{found:?}");
    let group = found[0];
    assert_eq!(group["lane"], "decide");
    assert_eq!(group["stale"], true);
    assert_eq!(group["file"], ".planning/HANDOFF.json");
    assert_eq!(group["file_status"], "paused");
    assert_eq!(group["phase"], "04");
    assert_eq!(group["phase_name"], "Run the device check");
    assert_eq!(group["command"], "/gsd-resume-work");
    let ids: Vec<(&str, &str, &str)> = group["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["id"].as_str().unwrap(),
                i["name"].as_str().unwrap(),
                i["result"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ids,
        [
            ("HANDOFF.json#a1", "Pair the test device", "human_action"),
            ("HANDOFF.json#b1", "Vendor key missing", "blocker"),
        ]
    );
    assert_eq!(group["items"][0]["reason"], "needs the lab bench");
    assert_eq!(group["items"][1]["reason"], "use the sandbox key");
    assert_eq!(doc["summary"]["decide"], 1);
    let text = render_text(std::slice::from_ref(&entry), &Filters::default());
    let row = text.lines().find(|l| l.contains("handoff")).unwrap();
    assert!(
        row.contains("paused")
            && row.contains("1 human action pending")
            && row.contains("1 blocker"),
        "{row}"
    );
    assert!(
        row.contains("stale") && row.contains("/gsd-resume-work"),
        "{row}"
    );
}

#[test]
fn handoff_workstreams_carry_ws_commands() {
    let (_g, entry) = entry_of("handoff-workstreams", "ws");
    assert_eq!(entry.digest.decide, 3);
    let doc = build_document(std::slice::from_ref(&entry), &include_all(), FOLD_DAYS);
    let found = handoff_groups_of(&doc);
    assert_eq!(found.len(), 3, "{found:?}");
    let by_file = |file: &str| {
        *found
            .iter()
            .find(|g| g["file"] == file)
            .unwrap_or_else(|| panic!("{file}"))
    };
    let root = by_file(".planning/HANDOFF.md");
    assert_eq!(root["command"], "/gsd-resume-work");
    assert_eq!(root["phase_name"], "Resume the import work");
    assert_eq!(root["file_status"], "unknown");
    assert!(root["workstream"].is_null());
    let a = by_file(".planning/workstreams/a/HANDOFF.json");
    assert_eq!(a["command"], "/gsd-resume-work --ws a");
    assert_eq!(a["file_status"], "ready");
    assert_eq!(a["workstream"], "a");
    assert_eq!(a["items"].as_array().unwrap().len(), 1);
    let b = by_file(".planning/workstreams/b/HANDOFF.json");
    assert_eq!(b["command"], "/gsd-resume-work --ws b");
    assert_eq!(b["file_status"], "unknown");
    assert_eq!(b["items"].as_array().unwrap().len(), 0);
    assert_eq!(b["counts"]["unreadable"], 0);
    assert_eq!(doc["summary"]["decide"], 3);
}

#[test]
fn checkpoint_phase_lists_only_the_open_checkpoint() {
    let (_g, entry) = entry_of("checkpoint-phase", "cp");
    assert_eq!(entry.digest.decide, 1);
    let doc = build_document(std::slice::from_ref(&entry), &include_all(), FOLD_DAYS);
    assert!(handoff_groups_of(&doc).is_empty());
    let found: Vec<&Value> = groups(&doc)
        .iter()
        .filter(|g| g["kind"] == "checkpoint")
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    let group = found[0];
    assert_eq!(group["phase"], "02");
    assert_eq!(group["phase_name"], "Open");
    assert_eq!(group["file"], ".planning/phases/02-open/.continue-here.md");
    assert_eq!(group["file_status"], "planning_incomplete");
    assert_eq!(group["command"], "/gsd-resume-work");
    assert_eq!(group["items"][0]["name"], "02-open");
    assert_eq!(group["items"][0]["result"], "planning_incomplete");
    assert_eq!(doc["summary"]["decide"], 1);
    let text = render_text(std::slice::from_ref(&entry), &Filters::default());
    assert!(text.contains("checkpoint: planning_incomplete"), "{text}");
}

#[test]
fn verification_gaps_found_without_items_is_fix_work() {
    let (_g, entry) = entry_of("verification-gaps-empty", "gaps");
    let d = &entry.digest;
    assert_eq!((d.fix_phases, d.closeable_phases), (1, 0));
    assert_eq!(d.groups.len(), 1, "{:?}", d.groups);
    let group = &d.groups[0];
    assert_eq!(group.lane, Lane::Fix);
    assert_eq!(group.counts.unreadable, 1);
    assert!(group.items.is_empty());
    assert_eq!(group.command.as_deref(), Some("/gsd-plan-phase 16 --gaps"));
    let text = render_text(std::slice::from_ref(&entry), &Filters::default());
    assert!(
        text.contains("verification gaps_found, no gaps listed"),
        "{text}"
    );
    assert!(text.contains("/gsd-plan-phase 16 --gaps"), "{text}");
}

#[test]
fn todos_age_reports_count_and_oldest() {
    let (guard, _root) = copy_fixture("todos-age");
    // A committed fixture cannot pin an mtime: the undated todo is set here.
    let delta = guard
        .path()
        .join(".planning/todos/pending/2026-09-01-delta.md");
    let moment: DateTime<Utc> = "2026-06-01T00:00:00Z".parse().unwrap();
    std::fs::File::options()
        .write(true)
        .open(&delta)
        .unwrap()
        .set_modified(moment.into())
        .unwrap();
    let planning = guard.path().join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, now(), 30);
    assert_eq!(digest.hidden.todos, 4);
    assert_eq!(digest.hidden.todos_oldest_days, Some(166));
    assert!(!digest.hidden.todos_unreadable);
}

#[test]
fn an_unreadable_pending_dir_is_flagged() {
    let (guard, _root) = copy_fixture("todos-age");
    let pending = guard.path().join(".planning/todos/pending");
    std::fs::remove_dir_all(&pending).unwrap();
    std::fs::write(&pending, "a regular file where the folder should be").unwrap();
    let planning = guard.path().join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, now(), 30);
    assert_eq!(digest.hidden.todos, 0);
    assert_eq!(digest.hidden.todos_oldest_days, None);
    assert!(digest.hidden.todos_unreadable);
    let entry = ProjectEntry {
        alias: Some("t".to_owned()),
        digest,
        ..Default::default()
    };
    let doc = build_document(&[entry], &Filters::default(), FOLD_DAYS);
    assert_eq!(doc["projects"][0]["hidden"]["todos"], 0);
    assert_eq!(doc["projects"][0]["hidden"]["todos_unreadable"], true);
}

/// D-11: every group's `command` is its typed `run` spelled out, over every fixture.
#[test]
fn run_command_matches_the_command_string_in_every_fixture() {
    use gsd_meta_manager::state_reader::owner_items::RunVerb;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/owner");
    let mut names: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.path().join(".planning").is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty());
    let mut seen: Vec<RunVerb> = Vec::new();
    for name in &names {
        let (_guard, digest) = digest_of(name);
        for group in &digest.groups {
            assert_eq!(
                group.command,
                group.run.as_ref().map(ToString::to_string),
                "{name}: {} {}",
                group.lane.as_str(),
                group.file
            );
            if let Some(run) = &group.run {
                if !seen.contains(&run.verb) {
                    seen.push(run.verb);
                }
            }
        }
    }
    for verb in [
        RunVerb::VerifyWork,
        RunVerb::PlanPhaseGaps,
        RunVerb::Debug,
        RunVerb::ResumeWork,
    ] {
        assert!(seen.contains(&verb), "no fixture yields {verb:?}");
    }
}

// --- The single-project view (Phase 38, plan 02) -------------------------------

#[test]
fn the_single_project_view_matches_the_golden() {
    let views: Vec<String> = [
        ("same-phase-merge", "merge"),
        ("stale-fold", "old-ledger"),
        ("heuristic-labels", "labels"),
    ]
    .into_iter()
    .map(|(name, alias)| {
        let (_guard, entry) = entry_of(name, alias);
        render_project(&entry, &Filters::default(), 90)
    })
    .collect();
    assert_golden("inbox-alias.txt", &views.join("\n"));
}

#[test]
fn the_merge_fixture_is_one_entry() {
    let (_guard, entry) = entry_of("same-phase-merge", "merge");
    let phase_43: Vec<_> = entry
        .digest
        .groups
        .iter()
        .filter(|g| g.phase == "43")
        .collect();
    assert_eq!(
        phase_43.len(),
        2,
        "the UAT and the VERIFICATION are two groups"
    );
    let entries = triage_entries(&entry.digest, 90);
    let merged: Vec<_> = entries.iter().filter(|e| e.group.phase == "43").collect();
    assert_eq!(merged.len(), 1);
    assert!(merged[0].is_merged());
    assert_eq!(merged[0].group.counts.checks, 3);
    assert_eq!(merged[0].files().len(), 2);
}

#[test]
fn the_stale_fixture_folds_phase_01_and_keeps_totals() {
    let (_guard, entry) = entry_of("stale-fold", "old-ledger");
    let entries = triage_entries(&entry.digest, 90);
    let folded = |phase: &str| {
        entries
            .iter()
            .find(|e| e.group.phase == phase)
            .unwrap_or_else(|| panic!("phase {phase}"))
            .folded
    };
    assert!(folded("01"));
    assert!(!folded("02"));
    let text = render_project(&entry, &Filters::default(), 90);
    let lines: Vec<&str> = text.lines().collect();
    let at = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
    assert!(
        at("TEST \u{2014} ") < at("ph 02  Search") && at("ph 02  Search") < at("STALE (> 90 d)")
    );
    assert!(at("STALE (> 90 d)") < at("ph 01  Ledger"));
    assert!(!lines.iter().any(|l| l.starts_with("Stale:")), "{text}");
    let legacy = render_text(std::slice::from_ref(&entry), &Filters::default());
    assert_eq!(lines[0], legacy.lines().next().unwrap());
    assert!(lines[0].contains("test 3"), "{}", lines[0]);
}

#[test]
fn the_labels_fixture_labels_exactly_the_d11_hits() {
    let (_guard, entry) = entry_of("heuristic-labels", "labels");
    let mut seen: Vec<(String, Vec<&str>)> = Vec::new();
    for group in &entry.digest.groups {
        for item in &group.items {
            let labels: Vec<&str> = labels_of(item, group.lane)
                .into_iter()
                .map(Label::as_str)
                .collect();
            seen.push((item.name.clone(), labels));
        }
    }
    let labels_of_name = |name: &str| {
        seen.iter()
            .find(|(n, _)| n.contains(name))
            .unwrap_or_else(|| panic!("no item {name}: {seen:?}"))
            .1
            .clone()
    };
    for hit in [
        "Owner decision: trial length",
        "Approve the new icon",
        "Review the export screen",
        "Looks OK on a phone",
    ] {
        assert_eq!(labels_of_name(hit), vec!["decision?"], "{hit}");
    }
    assert_eq!(labels_of_name("Retry timing"), vec!["looks resolved"]);
    for miss in [
        "Preview the export",
        "Okay button copy",
        "Acceptance list renders",
        "Unresolved sync banner",
        "Should we decide the pricing tiers?",
    ] {
        assert!(labels_of_name(miss).is_empty(), "{miss}");
    }
    let text = render_project(&entry, &Filters::default(), 90);
    let suffixed = |suffix: &str| text.lines().filter(|l| l.ends_with(suffix)).count();
    assert_eq!(suffixed("  (decision?)"), 4, "{text}");
    assert_eq!(suffixed("  (looks resolved)"), 1, "{text}");
}

// --- The multi-project views (Phase 38, plan 03) -------------------------------

const FOLD_DAYS: u32 = 90;

/// The six golden fixtures plus the four of Phase 38 (D-13).
fn fleet_entries() -> Vec<(TempDir, ProjectEntry)> {
    let mut held = golden_entries();
    for (name, alias) in [
        ("same-phase-merge", "merge"),
        ("stale-fold", "old-ledger"),
        ("heuristic-labels", "labels"),
        ("scripted-checklist", "script"),
    ] {
        held.push(entry_of(name, alias));
    }
    held
}

fn fleet() -> (Vec<(TempDir, ProjectEntry)>, Vec<ProjectEntry>) {
    let held = fleet_entries();
    let entries = held.iter().map(|(_, e)| e.clone()).collect();
    (held, entries)
}

fn test_lane() -> Filters {
    Filters {
        lanes: vec![Lane::Test],
        ..Filters::default()
    }
}

#[test]
fn the_digest_matches_the_golden() {
    let (_held, entries) = fleet();
    let text = render_digest(&entries, FOLD_DAYS);
    assert_golden("inbox-digest.txt", &text);
    let lines = text.lines().count();
    assert!((15..=22).contains(&lines), "{lines} lines:\n{text}");
    assert!(!text.lines().any(|l| l.starts_with("    - [ ] ")));
    assert!(text.lines().last().unwrap().starts_with("inbox <alias>"));
}

/// The width of `line` in terminal cells.
fn cells(line: &str) -> usize {
    ratatui::text::Span::raw(line).width()
}

/// UI review 38, fix 1: the digest fits one screen. Its project, fold and hint lines
/// fit 80 columns; collapsed lines (NEXT rows, the lane view) fit 100, because a row
/// with a workstream command needs 94 cells before its phrase (see
/// `COLLAPSED_MAX_CELLS`). The lane view and the digest share that bound.
#[test]
fn digest_and_lane_view_lines_fit_the_width_budget() {
    let (_held, entries) = fleet();
    let digest = render_digest(&entries, FOLD_DAYS);
    let mut in_next = false;
    for line in digest.lines() {
        if line == "NEXT" {
            in_next = true;
        } else if line.is_empty() {
            in_next = false;
        }
        let max = if in_next && line != "NEXT" { 100 } else { 80 };
        assert!(
            cells(line) <= max,
            "{} cells > {max}: {line:?}",
            cells(line)
        );
    }
    for lanes in [
        test_lane(),
        Filters {
            lanes: vec![Lane::Test, Lane::Decide, Lane::Fix, Lane::Blocked],
            ..Filters::default()
        },
    ] {
        let text = render_lanes(&entries, &lanes, FOLD_DAYS);
        for line in text.lines() {
            assert!(cells(line) <= 100, "{} cells: {line:?}", cells(line));
        }
    }
    for name in ["inbox-digest.txt", "inbox-lane-test.txt"] {
        let golden = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/owner_golden")
                .join(name),
        )
        .unwrap();
        for line in golden.lines() {
            assert!(
                cells(line) <= 100,
                "{name}: {} cells: {line:?}",
                cells(line)
            );
        }
    }
}

/// UI review 38: the lane view computes its column widths once over every section,
/// so the checks column sits at one offset in all of them.
#[test]
fn every_lane_view_section_shares_one_column_grid() {
    let (_held, entries) = fleet();
    let filters = Filters {
        lanes: vec![Lane::Test, Lane::Fix],
        ..Filters::default()
    };
    let text = render_lanes(&entries, &filters, FOLD_DAYS);
    let offsets: Vec<usize> = text
        .lines()
        .filter(|l| l.starts_with("  ") && (l.contains(" check ") || l.contains(" checks ")))
        .map(|l| l.find(" check").unwrap())
        .collect();
    assert!(offsets.len() >= 5, "{text}");
    assert!(
        offsets.windows(2).all(|w| w[0] == w[1]),
        "{offsets:?}\n{text}"
    );
}

#[test]
fn the_test_lane_view_matches_the_golden() {
    let (_held, entries) = fleet();
    assert_golden(
        "inbox-lane-test.txt",
        &render_lanes(&entries, &test_lane(), FOLD_DAYS),
    );
}

#[test]
fn a_scripted_checklist_is_one_entry_in_every_multi_project_view() {
    let (_held, entries) = fleet();
    let lanes = render_lanes(&entries, &test_lane(), FOLD_DAYS);
    let scripted: Vec<&str> = lanes.lines().filter(|l| l.contains("51 checks")).collect();
    assert_eq!(scripted.len(), 1, "{lanes}");
    assert!(scripted[0].contains("ph 04"));
    assert!(!lanes.lines().any(|l| l.starts_with("    - [ ] ")));
    assert!(lanes.lines().any(|l| l.contains("2 files")));
    let digest = render_digest(&entries, FOLD_DAYS);
    assert!(!digest.lines().any(|l| l.starts_with("    - [ ] ")));
    let script = entries
        .iter()
        .find(|e| e.alias.as_deref() == Some("script"))
        .unwrap();
    let project = render_project(script, &Filters::default(), FOLD_DAYS);
    assert_eq!(
        project
            .lines()
            .filter(|l| l.starts_with("    - [ ] "))
            .count(),
        51
    );
}

#[test]
fn lane_sections_keep_the_legacy_order() {
    let (_held, entries) = fleet();
    let filters = Filters {
        lanes: vec![Lane::Blocked, Lane::Test],
        ..Filters::default()
    };
    let text = render_lanes(&entries, &filters, FOLD_DAYS);
    let at = |needle: &str| text.lines().position(|l| l.starts_with(needle));
    let last_test = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with("TEST \u{2014} "))
        .map(|(i, _)| i)
        .last()
        .expect("a TEST section");
    if let Some(blocked) = at("BLOCKED") {
        assert!(last_test < blocked, "{text}");
    }
    assert!(text.contains("TEST \u{2014} "));
}

#[test]
fn the_lane_stale_line_counts_only_the_selected_lanes() {
    let (_held, entries) = fleet();
    let decide = Filters {
        lanes: vec![Lane::Decide],
        ..Filters::default()
    };
    let text = render_lanes(&entries, &decide, FOLD_DAYS);
    assert!(!text.lines().any(|l| l.starts_with("TEST")), "{text}");
}

// --- The triage in the JSON (Phase 38, plan 03) --------------------------------

fn groups_of<'a>(doc: &'a Value, phase: &str) -> Vec<&'a Value> {
    doc["projects"][0]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["phase"] == phase)
        .collect()
}

#[test]
fn the_merge_link_names_both_files() {
    let (_guard, entry) = entry_of("same-phase-merge", "merge");
    let doc = build_document(std::slice::from_ref(&entry), &Filters::default(), FOLD_DAYS);
    let pair = groups_of(&doc, "43");
    assert_eq!(pair.len(), 2, "both groups stay listed");
    for group in &pair {
        assert_eq!(
            group["entry_files"],
            serde_json::json!(["43-UAT.md", "43-VERIFICATION.md"])
        );
        assert_eq!(group["folded_stale"], false);
    }
    let alone: Vec<&Value> = doc["projects"][0]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["phase"] != "43")
        .collect();
    for group in alone {
        let files = group["entry_files"].as_array().unwrap();
        assert_eq!(files.len(), 1, "{group}");
        assert!(group["file"]
            .as_str()
            .unwrap()
            .ends_with(files[0].as_str().unwrap()));
    }
}

#[test]
fn folded_stale_is_the_entry_flag_and_totals_stay() {
    let (_guard, entry) = entry_of("stale-fold", "old-ledger");
    let doc = build_document(std::slice::from_ref(&entry), &Filters::default(), FOLD_DAYS);
    assert_eq!(groups_of(&doc, "01")[0]["folded_stale"], true);
    assert_eq!(groups_of(&doc, "02")[0]["folded_stale"], false);
    assert_eq!(
        doc["summary"]["stale_folded"],
        serde_json::json!({"entries": 1, "items": 2})
    );
    assert_eq!(doc["summary"]["test"], entry.digest.test);
    let wide = build_document(std::slice::from_ref(&entry), &Filters::default(), 100_000);
    assert_eq!(wide["summary"]["test"], doc["summary"]["test"]);
    assert_eq!(
        wide["summary"]["stale_folded"],
        serde_json::json!({"entries": 0, "items": 0})
    );
}

#[test]
fn labels_appear_on_every_item() {
    let (_guard, entry) = entry_of("heuristic-labels", "labels");
    let doc = build_document(std::slice::from_ref(&entry), &Filters::default(), FOLD_DAYS);
    let mut seen = 0;
    let mut non_empty = 0;
    for (group, owner) in doc["projects"][0]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&entry.digest.groups)
    {
        for (item, source) in group["items"].as_array().unwrap().iter().zip(&owner.items) {
            let expected: Vec<&str> = labels_of(source, owner.lane)
                .iter()
                .map(|l| l.as_str())
                .collect();
            assert_eq!(item["labels"], serde_json::json!(expected));
            seen += 1;
            non_empty += usize::from(!expected.is_empty());
        }
    }
    assert!(seen > 0 && non_empty > 0);
}

#[test]
fn next_actions_in_json_equal_the_digest_order() {
    let (_held, entries) = fleet();
    let doc = build_document(&entries, &Filters::default(), FOLD_DAYS);
    let next = doc["summary"]["next_actions"].as_array().unwrap();
    assert!(!next.is_empty() && next.len() <= 5);
    for action in next {
        assert_eq!(
            keys_of(action),
            [
                "age_days",
                "alias",
                "checks",
                "closes_phase",
                "command",
                "lane",
                "phase",
                "workstream"
            ]
        );
    }
    let digest = render_digest(&entries, FOLD_DAYS);
    let next_lines: Vec<&str> = digest
        .lines()
        .skip_while(|l| *l != "NEXT")
        .skip(1)
        .take_while(|l| !l.is_empty())
        .collect();
    let json_commands: Vec<&str> = next
        .iter()
        .map(|a| a["command"].as_str().unwrap())
        .collect();
    assert_eq!(next_lines.len(), json_commands.len(), "{digest}");
    for (line, command) in next_lines.iter().zip(&json_commands) {
        assert!(
            line.contains(&format!("  {command}")),
            "{line:?} lacks {command:?}"
        );
    }
    let aliases: Vec<&str> = next.iter().map(|a| a["alias"].as_str().unwrap()).collect();
    let digest_aliases: Vec<&str> = digest
        .lines()
        .skip_while(|l| *l != "NEXT")
        .skip(1)
        .take_while(|l| !l.is_empty())
        .map(|l| l.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(digest_aliases, aliases);
    // The lane and include filters change neither list, the old totals stay.
    let narrowed = build_document(&entries, &test_lane(), FOLD_DAYS);
    assert_eq!(
        narrowed["summary"]["next_actions"],
        doc["summary"]["next_actions"]
    );
    assert_eq!(
        narrowed["summary"]["stale_folded"],
        doc["summary"]["stale_folded"]
    );
}

fn keys_of(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}
