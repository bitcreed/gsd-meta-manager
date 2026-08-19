use gsd_meta_manager::state_reader::config_json::parse_gsd_config;
use gsd_meta_manager::state_reader::roadmap_md::parse_roadmap_phases;
use gsd_meta_manager::state_reader::state_md::{extract_frontmatter, parse_state_md};
use gsd_meta_manager::state_reader::{count_backlog_items, parse_project_state};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// STATE.md parsing tests
// ============================================================================

#[test]
fn test_parse_real_state_md() {
    let content = "\
---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
stopped_at: Phase 1 context gathered
last_updated: \"2026-03-25T04:22:49.224Z\"
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---
# Project State

---

Some body content with a horizontal rule above.
";
    let fm = parse_state_md(content).unwrap();
    assert_eq!(fm.status, "planning");
    assert_eq!(fm.progress.total_phases, 4);
    assert_eq!(fm.progress.completed_phases, 0);
    assert_eq!(fm.milestone, "v1.0");
    assert_eq!(fm.milestone_name, "milestone");
    assert_eq!(fm.gsd_state_version, 1.0);
    assert_eq!(fm.stopped_at, "Phase 1 context gathered");
    assert_eq!(fm.progress.percent, 0);
}

#[test]
fn test_parse_state_md_empty() {
    assert!(parse_state_md("").is_none());
}

#[test]
fn test_parse_state_md_no_frontmatter() {
    assert!(parse_state_md("# Just a heading\nNo frontmatter here.").is_none());
}

#[test]
fn test_parse_state_md_missing_optional_fields() {
    let content = "---\nstatus: active\n---\n";
    let fm = parse_state_md(content).unwrap();
    assert_eq!(fm.status, "active");
    assert_eq!(fm.progress.total_phases, 0);
    assert_eq!(fm.milestone, "");
    assert!(fm.last_activity.is_none());
}

#[test]
fn test_extract_frontmatter_preserves_yaml() {
    let content = "---\nkey: value\nnested:\n  a: 1\n---\n# Body";
    let yaml = extract_frontmatter(content).unwrap();
    assert!(yaml.contains("key: value"));
    assert!(yaml.contains("nested:"));
    assert!(yaml.contains("  a: 1"));
}

// ============================================================================
// ROADMAP.md parsing tests
// ============================================================================

#[test]
fn test_parse_real_roadmap() {
    let content = "\
# Roadmap

## Phases

- [ ] **Phase 1: Core Infrastructure** - Async TUI foundation, state reader, and project registry
- [ ] **Phase 2: Dashboard and Navigation** - Main project list, status indicators, keyboard nav
- [x] **Phase 3: Live State and Detail View** - File-watcher auto-refresh and project drill-down
- [ ] **Phase 4: Visualization, Creation, and Enqueue** - ASCII roadmap, new project creation
";
    let phases = parse_roadmap_phases(content);
    assert_eq!(phases.len(), 4);
    assert!(!phases[0].completed);
    assert_eq!(phases[0].number, "1");
    assert_eq!(phases[0].name, "Core Infrastructure");
    assert!(phases[0].description.contains("Async TUI foundation"));
    assert!(phases[2].completed);
    assert_eq!(phases[1].name, "Dashboard and Navigation");
    assert_eq!(phases[3].number, "4");
}

#[test]
fn test_parse_roadmap_empty() {
    let phases = parse_roadmap_phases("");
    assert!(phases.is_empty());
}

#[test]
fn test_parse_roadmap_case_insensitive_x() {
    let content = "- [X] **Phase 1: Done Phase** - Completed\n";
    let phases = parse_roadmap_phases(content);
    assert_eq!(phases.len(), 1);
    assert!(phases[0].completed);
    assert_eq!(phases[0].name, "Done Phase");
}

// ============================================================================
// config.json parsing tests
// ============================================================================

#[test]
fn test_parse_gsd_config() {
    let content = r#"{"mode": "yolo", "granularity": "coarse"}"#;
    let config = parse_gsd_config(content).unwrap();
    assert_eq!(config.mode, "yolo");
    assert_eq!(config.granularity, "coarse");
}

#[test]
fn test_parse_gsd_config_invalid_json() {
    assert!(parse_gsd_config("not json").is_none());
}

#[test]
fn test_parse_gsd_config_missing_fields() {
    let config = parse_gsd_config("{}").unwrap();
    assert_eq!(config.mode, "");
    assert_eq!(config.granularity, "");
}

#[test]
fn test_parse_gsd_config_extra_fields_ignored() {
    let content =
        r#"{"mode": "yolo", "granularity": "coarse", "extra_field": true, "workflow": {}}"#;
    let config = parse_gsd_config(content).unwrap();
    assert_eq!(config.mode, "yolo");
}

// ============================================================================
// Backlog count tests
// ============================================================================

#[test]
fn test_count_backlog_zero() {
    let tmp = TempDir::new().unwrap();
    let phases_dir = tmp.path().join("phases");
    fs::create_dir_all(phases_dir.join("01-core-infra")).unwrap();
    assert_eq!(count_backlog_items(tmp.path()), 0);
}

#[test]
fn test_count_backlog_two() {
    let tmp = TempDir::new().unwrap();
    let phases_dir = tmp.path().join("phases");
    fs::create_dir_all(phases_dir.join("01-core-infra")).unwrap();
    fs::create_dir_all(phases_dir.join("999.1-backlog")).unwrap();
    fs::create_dir_all(phases_dir.join("999.2-future")).unwrap();
    assert_eq!(count_backlog_items(tmp.path()), 2);
}

#[test]
fn test_count_backlog_missing_phases_dir() {
    let tmp = TempDir::new().unwrap();
    // No phases/ subdirectory
    assert_eq!(count_backlog_items(tmp.path()), 0);
}

#[test]
fn test_count_backlog_ignores_files_starting_with_999() {
    let tmp = TempDir::new().unwrap();
    let phases_dir = tmp.path().join("phases");
    fs::create_dir_all(&phases_dir).unwrap();
    // Create a file (not directory) starting with 999
    fs::write(phases_dir.join("999-notes.txt"), "not a dir").unwrap();
    assert_eq!(count_backlog_items(tmp.path()), 0);
}

// ============================================================================
// Integration: parse_project_state
// ============================================================================

#[test]
fn test_parse_project_state_full() {
    let tmp = TempDir::new().unwrap();
    let planning_dir = tmp.path();

    // Write STATE.md
    let state_content = "\
---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
stopped_at: Phase 1 context gathered
last_updated: \"2026-03-25T04:22:49.224Z\"
progress:
  total_phases: 4
  completed_phases: 1
  total_plans: 3
  completed_plans: 1
  percent: 25
---
# State
";
    fs::write(planning_dir.join("STATE.md"), state_content).unwrap();

    // Write ROADMAP.md
    let roadmap_content = "\
# Roadmap

- [x] **Phase 1: Core Infrastructure** - Foundation
- [ ] **Phase 2: Dashboard** - UI
- [ ] **Phase 3: Live State** - Watcher
- [ ] **Phase 4: Viz** - Graphs
";
    fs::write(planning_dir.join("ROADMAP.md"), roadmap_content).unwrap();

    // Write config.json
    fs::write(
        planning_dir.join("config.json"),
        r#"{"mode": "yolo", "granularity": "fine"}"#,
    )
    .unwrap();

    // Create phases dir with one backlog item
    let phases_dir = planning_dir.join("phases");
    fs::create_dir_all(phases_dir.join("01-core")).unwrap();
    fs::create_dir_all(phases_dir.join("999.1-backlog")).unwrap();

    let state = parse_project_state(planning_dir);

    assert_eq!(state.status, "planning");
    assert_eq!(state.milestone, "v1.0");
    assert_eq!(state.total_phases, 4);
    assert_eq!(state.completed_phases, 1);
    assert_eq!(state.total_plans, 3);
    assert_eq!(state.completed_plans, 1);
    assert_eq!(state.phases.len(), 4);
    assert!(state.phases[0].completed);
    assert!(!state.phases[1].completed);
    assert_eq!(state.backlog_count, 1);
    assert_eq!(state.current_phase, "Phase 1 context gathered");
}

#[test]
fn test_parse_project_state_missing_files() {
    let tmp = TempDir::new().unwrap();
    let state = parse_project_state(tmp.path());

    assert_eq!(state.status, "unknown");
    assert_eq!(state.total_phases, 0);
    assert_eq!(state.completed_phases, 0);
    assert!(state.phases.is_empty());
    assert_eq!(state.backlog_count, 0);
    // No panic -- graceful degradation
}

#[test]
fn test_parse_project_state_malformed_state_md() {
    let tmp = TempDir::new().unwrap();
    // Write invalid YAML frontmatter
    fs::write(
        tmp.path().join("STATE.md"),
        "---\ninvalid: [yaml: {{{\n---\n",
    )
    .unwrap();

    let state = parse_project_state(tmp.path());
    assert_eq!(state.status, "unknown"); // Falls back to default
}

#[test]
fn test_parse_project_state_not_paused() {
    let tmp = TempDir::new().unwrap();
    // No HANDOFF files at all
    fs::write(tmp.path().join("STATE.md"), "---\nstatus: active\n---\n").unwrap();
    let state = parse_project_state(tmp.path());
    assert!(!state.paused);
    assert!(state.pause_context.is_none());
}

#[test]
fn test_parse_project_state_paused_with_handoff_json() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("STATE.md"), "---\nstatus: active\n---\n").unwrap();
    fs::write(
        tmp.path().join("HANDOFF.json"),
        r#"{"next_action": "Fix timeout"}"#,
    )
    .unwrap();
    let state = parse_project_state(tmp.path());
    assert!(state.paused);
    assert_eq!(state.pause_context, Some("Fix timeout".to_string()));
}

#[test]
fn test_parse_project_state_paused_with_handoff_md() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("STATE.md"), "---\nstatus: active\n---\n").unwrap();
    fs::write(
        tmp.path().join("HANDOFF.md"),
        "# Handoff\nReview the API design\n",
    )
    .unwrap();
    let state = parse_project_state(tmp.path());
    assert!(state.paused);
    assert_eq!(
        state.pause_context,
        Some("Review the API design".to_string())
    );
}

#[test]
fn test_parse_project_state_empty_handoff_ignored() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("STATE.md"), "---\nstatus: active\n---\n").unwrap();
    // Empty HANDOFF.json should NOT trigger paused state
    fs::write(tmp.path().join("HANDOFF.json"), "").unwrap();
    let state = parse_project_state(tmp.path());
    assert!(!state.paused);
    assert!(state.pause_context.is_none());
}

#[test]
fn test_parse_project_state_handoff_md_no_heading() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("STATE.md"), "---\nstatus: active\n---\n").unwrap();
    fs::write(tmp.path().join("HANDOFF.md"), "Do the thing\n").unwrap();
    let state = parse_project_state(tmp.path());
    assert!(state.paused);
    assert_eq!(state.pause_context, Some("Do the thing".to_string()));
}

// ============================================================================
// HANDOFF / pause detection tests
// ============================================================================
// (Tests above cover pause detection via parse_project_state integration)

#[test]
fn test_parse_project_state_derives_current_phase_from_completed() {
    let tmp = TempDir::new().unwrap();
    // STATE.md with no stopped_at
    let state_content = "\
---
status: active
progress:
  total_phases: 4
  completed_phases: 2
---
";
    fs::write(tmp.path().join("STATE.md"), state_content).unwrap();

    let state = parse_project_state(tmp.path());
    assert_eq!(state.status, "active");
    assert_eq!(state.current_phase, "Phase 3"); // completed_phases + 1
}

// ============================================================================
// Plan 20-03 Task 1 — GSD's `executed` vocabulary reaches ProjectState
// ============================================================================
//
// The tests in `src/state_reader/disk_status.rs` prove the inference in
// isolation. These prove the *seam*: that the corrected status and the typed
// verification status arrive on `ProjectState.phase_disk_statuses`, which is
// what the dashboard and the driver's router both read. A reader that inferred
// correctly and threaded nothing through would pass every unit test and change
// nothing a router can see.

/// Build a `.planning/` holding a ROADMAP declaring one phase plus the given
/// files under `phases/<phase_dir>/`. Returns the TempDir (keep it alive).
fn planning_with_phase(phase_number: &str, phase_dir: &str, files: &[(&str, &str)]) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path().join(".planning");
    fs::create_dir_all(&planning).unwrap();
    fs::write(planning.join("STATE.md"), "---\nstatus: executing\n---\n").unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        format!("- [ ] **Phase {phase_number}: Fixture** - a phase\n"),
    )
    .unwrap();
    let dir = planning.join("phases").join(phase_dir);
    fs::create_dir_all(&dir).unwrap();
    for (name, content) in files {
        fs::write(dir.join(name), content).unwrap();
    }
    tmp
}

#[test]
fn test_project_state_reports_executed_for_an_unverified_phase() {
    use gsd_meta_manager::state_reader::disk_status::{DiskStatus, VerificationStatus};

    let tmp = planning_with_phase(
        "19",
        "19-thing",
        &[
            ("19-01-PLAN.md", "plan"),
            ("19-01-SUMMARY.md", "summary"),
            ("19-VERIFICATION.md", "---\nstatus: human_needed\n---\n"),
        ],
    );
    let state = parse_project_state(&tmp.path().join(".planning"));
    let inference = state
        .phase_disk_statuses
        .get("19")
        .expect("the declared phase has a disk inference");

    assert_eq!(
        inference.status,
        DiskStatus::Executed,
        "a phase whose verification needs a human must not read as finished on \
         the value the dashboard and the router share (DRIVE-05)"
    );
    assert_ne!(inference.status, DiskStatus::Complete);
    assert_eq!(
        inference.verification_status,
        VerificationStatus::HumanNeeded
    );
    // The corrected predicate also moves what "the current phase" means: an
    // executed-but-unverified phase is still the phase in flight.
    assert_eq!(
        state.current_phase_status.as_ref().map(|i| i.status),
        Some(DiskStatus::Executed)
    );
}

#[test]
fn test_project_state_reports_complete_only_when_verification_passed() {
    use gsd_meta_manager::state_reader::disk_status::{DiskStatus, VerificationStatus};

    let tmp = planning_with_phase(
        "19",
        "19-thing",
        &[
            ("19-01-PLAN.md", "plan"),
            ("19-01-SUMMARY.md", "summary"),
            ("19-VERIFICATION.md", "---\nstatus: passed\n---\n"),
        ],
    );
    let state = parse_project_state(&tmp.path().join(".planning"));
    let inference = state.phase_disk_statuses.get("19").unwrap();
    assert_eq!(inference.status, DiskStatus::Complete);
    assert_eq!(inference.verification_status, VerificationStatus::Passed);
}

#[test]
fn test_project_state_never_reports_complete_without_a_passing_verification() {
    use gsd_meta_manager::state_reader::disk_status::DiskStatus;

    // The property, swept over every gating status plus the absent artifact.
    for frontmatter in [
        None,
        Some("---\nstatus: human_needed\n---\n"),
        Some("---\nstatus: gaps_found\n---\n"),
        Some("---\nstatus: stale\n---\n"),
        Some("---\nstatus: missing\n---\n"),
        Some("---\nstatus: something_new\n---\n"),
        Some("no frontmatter at all\n\n```\nstatus: passed\n```\n"),
    ] {
        let mut files: Vec<(&str, &str)> =
            vec![("19-01-PLAN.md", "plan"), ("19-01-SUMMARY.md", "summary")];
        if let Some(content) = frontmatter {
            files.push(("19-VERIFICATION.md", content));
        }
        let tmp = planning_with_phase("19", "19-thing", &files);
        let state = parse_project_state(&tmp.path().join(".planning"));
        let inference = state.phase_disk_statuses.get("19").unwrap();
        assert_eq!(
            inference.status,
            DiskStatus::Executed,
            "only a `passed` verification admits Complete; {frontmatter:?} is not one"
        );
    }
}

// ============================================================================
// Plan 20-03 Task 2 — the rest of the disk-observable human-judgement gates
// ============================================================================

#[test]
fn test_non_empty_root_continue_here_sets_the_project_gate() {
    let tmp = planning_with_phase("19", "19-thing", &[]);
    fs::write(
        tmp.path().join(".planning").join(".continue-here.md"),
        "# Stopped\n\nAsk the user which option to take.\n",
    )
    .unwrap();
    let state = parse_project_state(&tmp.path().join(".planning"));
    assert!(
        state.continue_here_present,
        "a project-root continue-here marker is a hard stop whose only bypass is \
         --force (next.md:46-58)"
    );
}

#[test]
fn test_empty_root_continue_here_does_not_set_the_project_gate() {
    let tmp = planning_with_phase("19", "19-thing", &[]);
    fs::write(
        tmp.path().join(".planning").join(".continue-here.md"),
        "   \n\t\n  \n",
    )
    .unwrap();
    let state = parse_project_state(&tmp.path().join(".planning"));
    assert!(
        !state.continue_here_present,
        "content check, not existence check — the precedent the handoff detector \
         already sets. A file trimmed to nothing is a leftover, not a signal"
    );
}

#[test]
fn test_no_root_continue_here_does_not_set_the_project_gate() {
    let tmp = planning_with_phase("19", "19-thing", &[]);
    let state = parse_project_state(&tmp.path().join(".planning"));
    assert!(!state.continue_here_present);
}

#[test]
fn test_phase_continue_here_gate_flows_onto_the_phase_inference() {
    let tmp = planning_with_phase("19", "19-thing", &[]);
    let phase_dir = tmp
        .path()
        .join(".planning")
        .join("phases")
        .join("19-thing");
    fs::write(
        phase_dir.join(".continue-here.md"),
        "| Task | Severity |\n|---|---|\n| Decide the schema | blocking |\n",
    )
    .unwrap();
    let state = parse_project_state(&tmp.path().join(".planning"));
    assert!(state
        .phase_disk_statuses
        .get("19")
        .expect("the declared phase has an inference")
        .continue_here_blocking);
}

#[test]
fn test_outstanding_uat_gate_flows_onto_the_phase_inference() {
    let tmp = planning_with_phase(
        "19",
        "19-thing",
        &[("19-UAT.md", "---\nstatus: pending\n---\n")],
    );
    let state = parse_project_state(&tmp.path().join(".planning"));
    let inference = state.phase_disk_statuses.get("19").unwrap();
    assert!(inference.uat_status.is_outstanding());
    assert_eq!(inference.uat_status.as_str(), "pending");
}

#[test]
fn test_deferred_verification_row_names_the_deferred_phase() {
    use gsd_meta_manager::state_reader::state_md::deferred_verification_phases;

    let content = "\
---
status: executing
---

## Deferred Verification

| Phase | State | Resume |
|-------|-------|--------|
| 19 | verification_deferred_human | /gsd-verify-work 19 |
| 0.3 | verification_deferred_human | /gsd-verify-work 0.3 |

Prose after the table.

## Something Else

| Phase | State |
|-------|-------|
| 21 | not deferred at all |
";
    assert_eq!(
        deferred_verification_phases(content),
        vec!["19".to_string(), "0.3".to_string()],
        "the section scope closes at the next heading, so a table further down \
         the document is never read as a deferred-verification row"
    );

    let tmp = planning_with_phase("19", "19-thing", &[]);
    fs::write(tmp.path().join(".planning").join("STATE.md"), content).unwrap();
    let state = parse_project_state(&tmp.path().join(".planning"));
    assert_eq!(state.deferred_verification_phases, vec!["19", "0.3"]);
}

#[test]
fn test_no_deferred_verification_section_yields_no_phases() {
    use gsd_meta_manager::state_reader::state_md::deferred_verification_phases;

    assert!(deferred_verification_phases("---\nstatus: executing\n---\n").is_empty());
    // A malformed table under the right heading is still not an error.
    assert!(deferred_verification_phases("## Deferred Verification\n\n| |\n|---|\n").is_empty());
    assert!(deferred_verification_phases("").is_empty());
}

#[test]
fn test_error_and_failed_project_statuses_read_as_a_gate() {
    use gsd_meta_manager::state_reader::state_md::is_error_status;

    for status in ["error", "failed", "ERROR", " Failed "] {
        assert!(is_error_status(status), "{status:?} is next.md:60-69's hard stop");
    }
    for status in [
        "executing",
        "planning",
        "recovered from error",
        "failed to reach the registry",
        "",
    ] {
        assert!(
            !is_error_status(status),
            "{status:?} is prose about a failure, not a project in one; a \
             substring test would park on both"
        );
    }
}

#[test]
fn test_this_repositorys_own_planning_dir_reads_without_panicking() {
    use gsd_meta_manager::state_reader::disk_status::DiskStatus;

    // Every new read pointed at real data, including the stale phase-19 marker.
    let planning = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".planning");
    if !planning.is_dir() {
        return;
    }
    let state = parse_project_state(&planning);
    assert!(
        !state.phases.is_empty(),
        "the roadmap declares phases; if this is empty the reader found nothing \
         to assert against and the checks below are vacuous"
    );
    for (number, inference) in &state.phase_disk_statuses {
        // The DRIVE-05 invariant itself, over real artifacts: Complete is a
        // conjunction, so it can never coexist with a non-passing verification.
        if inference.status == DiskStatus::Complete {
            assert!(
                inference.verification_status.is_passed()
                    || !inference.has_verification,
                "phase {number} reads Complete while its verification status is \
                 {:?}",
                inference.verification_status
            );
        }
    }
}
