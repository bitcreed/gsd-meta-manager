use gsd_meta_manager::state_reader::config_json::parse_gsd_config;
use gsd_meta_manager::state_reader::roadmap_md::parse_roadmap_phases;
use gsd_meta_manager::state_reader::state_md::{
    extract_frontmatter, parse_state_md, read_frontmatter, FrontmatterFault, FrontmatterOutcome,
    StateVersion,
};
use gsd_meta_manager::state_reader::backlog::parse_backlog_items;
use gsd_meta_manager::state_reader::{count_backlog_items, parse_project_state, PhaseMarker};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// STATE.md parsing tests
// ============================================================================

#[test]
fn test_parse_real_state_md() {
    let content = "\
---
gsd_state_version: \"1.0\"
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
    // GSD writes this QUOTED. It is compared as a version, never as text — a
    // `f64` field here is what discarded the whole frontmatter for every real
    // project while this very test passed against an unquoted fixture.
    assert_eq!(fm.gsd_state_version, Some(StateVersion::parse("1.0")));
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
// Backlog count/parse agreement (260916-vr0)
//
// The overview advertises a backlog count; the Backlog tab draws a row per
// parsed item. Those were two different rules, and on every real disk they
// disagreed totally: measured across all six registered projects, 12 of 12
// `999.*` directories contain exactly one entry — `.gitkeep` — and zero contain
// a `.md` file. The parser required a `.md` file, so it returned zero items for
// every project that had a backlog at all.
//
// The fixtures below mirror that measured shape rather than an idealised one: a
// `999.N-slug` directory whose entire payload is its NAME.
// ============================================================================

/// A `phases/` tree in the shape every measured project actually has on disk:
/// two well-formed `999.N-slug` directories holding nothing but a zero-byte
/// `.gitkeep`, beside one ordinary phase directory.
fn make_md_less_backlog_fixture() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let phases_dir = tmp.path().join("phases");
    fs::create_dir_all(phases_dir.join("01-core-infra")).unwrap();
    for dir_name in [
        "999.1-milestone-archive-browser-tab",
        "999.2-container-claude-injection",
    ] {
        let item_dir = phases_dir.join(dir_name);
        fs::create_dir_all(&item_dir).unwrap();
        // Exactly what is in every real one: an empty `.gitkeep`, no `.md`.
        fs::write(item_dir.join(".gitkeep"), "").unwrap();
    }
    tmp
}

/// **The reported symptom, as a test.** The overview says 2, the tab draws 0.
#[test]
fn md_less_backlog_directories_reach_the_tab_instead_of_being_discarded() {
    let tmp = make_md_less_backlog_fixture();
    let counted = count_backlog_items(tmp.path());
    let drawn = parse_backlog_items(tmp.path()).len();

    assert_eq!(
        counted, 2,
        "the fixture is only meaningful if the overview counts both directories; \
         overview counted {counted}, expected 2"
    );
    assert_eq!(
        drawn, 2,
        "overview counted {counted}, the tab could draw {drawn}. A backlog \
         directory's payload is its NAME; requiring a `.md` file inside it \
         discards 100% of the directories that exist on real disks."
    );
}

/// **The invariant.** The number the overview advertises IS the number of rows
/// the tab can draw. Not "close to" — the same number, from the same rule.
#[test]
fn the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw() {
    let tmp = make_md_less_backlog_fixture();
    let counted = count_backlog_items(tmp.path()) as usize;
    let drawn = parse_backlog_items(tmp.path()).len();

    assert_eq!(
        counted, drawn,
        "overview counted {counted}, the tab could draw {drawn}. These must be \
         one rule in one place; two rules kept in agreement by care is what \
         produced a non-zero count above an empty list."
    );
}

/// The description fallback and the absent-file fields, on the shape that is
/// the norm rather than the exception.
#[test]
fn a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug() {
    let tmp = make_md_less_backlog_fixture();
    let items = parse_backlog_items(tmp.path());

    assert_eq!(
        items.len(),
        2,
        "overview counted {}, the tab could draw {}",
        count_backlog_items(tmp.path()),
        items.len()
    );

    let numbers: Vec<&str> = items
        .iter()
        .map(|i| i.number.as_raw_for_logic_only())
        .collect();
    assert_eq!(
        numbers,
        vec!["999.1", "999.2"],
        "items keep their existing ascending order by number"
    );

    let descriptions: Vec<&str> = items
        .iter()
        .map(|i| i.description.as_raw_for_logic_only())
        .collect();
    assert_eq!(
        descriptions,
        vec![
            "Milestone archive browser tab",
            "Container claude injection"
        ],
        "with no `.md` heading to read, the humanized slug is the description"
    );

    for item in &items {
        assert!(
            item.path.is_none(),
            "{}: there is no `.md` file, so there is no path to one",
            item.dir_name.as_raw_for_logic_only()
        );
        assert!(
            item.content.is_none(),
            "{}: `parse_backlog_items` never loads content",
            item.dir_name.as_raw_for_logic_only()
        );
    }
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
gsd_state_version: \"1.0\"
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
    //
    // An assertion, not an early return (IN-07): this directory is committed and
    // always present, so a `return` here would turn "the reader could not find
    // its subject" into a pass — the vacuous shape every other guard in this
    // phase is written to avoid.
    let planning = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".planning");
    assert!(
        planning.is_dir(),
        "this repository ships its own .planning/ directory; if it is missing, \
         every assertion below would be checking nothing"
    );
    let state = parse_project_state(&planning);
    assert!(
        !state.phases.is_empty(),
        "the roadmap declares phases; if this is empty the reader found nothing \
         to assert against and the checks below are vacuous"
    );
    // Task 3: the roadmap's declared dependencies survive the trip onto
    // ProjectState, where the router's dependency condition will read them.
    let phase_20 = state.phases.iter().find(|p| p.number == "20");
    if let Some(phase) = phase_20 {
        assert_eq!(
            phase.depends_on,
            vec!["16", "17", "19"],
            "the dependency condition must read what the roadmap DECLARES; a \
             numbering heuristic would have said `19` alone"
        );
    }
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

// ============================================================================
// Which phase `current_phase_status` names (WR-04)
// ============================================================================

/// A planning dir declaring two phases, where phase 1 is EXECUTED but not
/// verified and phase 2 is only planned.
fn two_phase_project(verification: Option<&str>) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();
    fs::write(planning.join("STATE.md"), "---\nstatus: executing\n---\n# State\n").unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n- [ ] **Phase 1: First** - implemented, awaiting verification\n\
         - [ ] **Phase 2: Second** - planned only\n",
    )
    .unwrap();

    let one = planning.join("phases").join("01-first");
    fs::create_dir_all(&one).unwrap();
    fs::write(one.join("01-01-PLAN.md"), "# Plan\n").unwrap();
    fs::write(one.join("01-01-SUMMARY.md"), "# Summary\n").unwrap();
    if let Some(status) = verification {
        fs::write(
            one.join("01-VERIFICATION.md"),
            format!("---\nstatus: {status}\n---\n# Verification\n"),
        )
        .unwrap();
    }

    let two = planning.join("phases").join("02-second");
    fs::create_dir_all(&two).unwrap();
    fs::write(two.join("02-01-PLAN.md"), "# Plan\n").unwrap();

    tmp
}

#[test]
fn the_current_phase_status_is_the_first_unexecuted_phase_not_the_first_unverified_one() {
    use gsd_meta_manager::state_reader::disk_status::DiskStatus;

    // **The decision this pins (WR-04).** Phase 20 made `Complete` a conjunction
    // — implementation AND verification passed — and inserted `Executed` beneath
    // it. A "first non-Complete" scan therefore stops at a phase awaiting
    // verification, which silently moved this value one phase backwards for
    // every project with an unverified completed phase.
    //
    // The threshold is implementation, deliberately: this value feeds the
    // dashboard row's compact pipeline cell, which sits beside a phase label
    // taken from STATE.md's `current_phase` — a label GSD advances on execution.
    // A cell describing a different phase from the one its own row names is the
    // worse failure. The verification gate surfaces through the needs-human
    // badge and the driver's DRIVE-05 gate set instead, both of which read the
    // per-phase inference rather than this summary.
    let unverified = two_phase_project(Some("human_needed"));
    let state = parse_project_state(unverified.path());

    assert_eq!(
        state.phase_disk_statuses.get("1").map(|i| i.status),
        Some(DiskStatus::Executed),
        "the premise: phase 1 is implemented and its verification is \
         human_needed, so it reads Executed and NOT Complete. Without this the \
         assertion below would be vacuous"
    );
    assert_eq!(
        state
            .current_phase_status
            .as_ref()
            .map(|inference| inference.status),
        Some(DiskStatus::Planned),
        "the current phase must be phase 2 — the first phase whose \
         implementation is unfinished. Stopping at phase 1 would describe a \
         phase the dashboard row does not name"
    );

    // The same tree with phase 1 fully verified answers identically, which is
    // what makes the choice a threshold rather than a coincidence of this
    // fixture's verification status.
    let verified = two_phase_project(Some("passed"));
    assert_eq!(
        parse_project_state(verified.path())
            .current_phase_status
            .as_ref()
            .map(|inference| inference.status),
        Some(DiskStatus::Planned),
        "a verified phase 1 and an unverified one must both hand the current \
         phase to phase 2; if they differ, the dashboard's current phase moves \
         when a VERIFICATION.md lands rather than when work does"
    );

    // And with no verification artifact at all — the shape most projects are in,
    // since GSD writes one only when the phase is verified.
    let bare = two_phase_project(None);
    assert_eq!(
        parse_project_state(bare.path())
            .current_phase_status
            .as_ref()
            .map(|inference| inference.status),
        Some(DiskStatus::Planned),
        "a project that never runs /gsd:verify-work must not have its dashboard \
         pinned to its first executed phase forever"
    );
}

// ============================================================================
// An unreadable STATE.md must not look like an absent one
// ============================================================================

/// The failure mode that let the quoted-`gsd_state_version` bug ship unnoticed:
/// a STATE.md that exists but cannot be read produced byte-identical
/// `ProjectState` to a project that has none, so nothing downstream could tell
/// a guess from a fact.
#[test]
fn an_unparseable_state_md_is_distinguishable_from_an_absent_one() {
    let absent = TempDir::new().unwrap();
    fs::write(
        absent.path().join("ROADMAP.md"),
        "# Roadmap\n\n### Phase 1: Foundation\n",
    )
    .unwrap();
    let absent_state = parse_project_state(absent.path());
    assert!(!absent_state.state_md_unreadable);
    assert!(absent_state.state_md_fault.is_none());

    let broken = TempDir::new().unwrap();
    // A frontmatter block that is genuinely not a mapping.
    fs::write(
        broken.path().join("STATE.md"),
        "---\n- one\n- two\n---\n# State\n",
    )
    .unwrap();
    fs::write(
        broken.path().join("ROADMAP.md"),
        "# Roadmap\n\n### Phase 1: Foundation\n",
    )
    .unwrap();
    let broken_state = parse_project_state(broken.path());
    assert!(broken_state.state_md_unreadable);
    assert_eq!(
        broken_state.state_md_fault,
        Some(FrontmatterFault::NotAMapping)
    );
}

/// A frontmatter block whose only fault is a top-level plain scalar carrying a
/// colon-space. The shape measured on a real foreign `.planning/STATE.md`: the
/// faulty `stopped_at` on file line 7, an already-quoted sibling whose value
/// also carries a colon-space, and a bare `progress:` opening nested keys.
const SENTRIQ_SHAPED_STATE_MD: &str = "---\ngsd_state_version: 1.0\nmilestone: v0.12\ncurrent_phase: 9\ncurrent_phase_name: Routine Event Logging\nstatus: planning\nstopped_at: Completed 260910-p8v (DTC history is vehicle-scoped — the trace recorder covers the connect path). Earlier: 260910-x8d — the discovery keystone.\nlast_activity_desc: \"260910-x8d, the discovery keystone: Phase 5 builds its worklist PER MODULE\"\nprogress:\n  total_phases: 4\n  completed_phases: 0\n---\n# State\n";

/// End-to-end through the reader every screen actually calls: the repairable
/// file yields its real phase AND says it was repaired.
#[test]
fn a_repairable_state_md_reads_and_reports_that_it_was_repaired() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("STATE.md"), SENTRIQ_SHAPED_STATE_MD).unwrap();
    fs::write(
        tmp.path().join("ROADMAP.md"),
        "# Roadmap\n\n### Phase 9: Routine Event Logging\n",
    )
    .unwrap();
    let state = parse_project_state(tmp.path());

    assert!(
        state.state_md_recovered,
        "a document this tool rewrote before believing must say so"
    );
    assert!(!state.state_md_unreadable);
    assert!(state.state_md_fault.is_none());
    assert!(state.state_md_fault_position.is_none());
    // And the values the unreadable path would have lost:
    assert_eq!(state.status, "planning");
    assert_eq!(state.current_phase, "Routine Event Logging");
    assert_eq!(state.state_md_phase_number, Some(9));
}

/// The three outcomes are mutually exclusive by construction — a file is clean,
/// or repaired, or unreadable, never two of them. Collapsing any pair is the
/// failure mode this whole register exists to prevent.
#[test]
fn recovered_and_unreadable_are_never_both_set() {
    let clean = "---\nstatus: executing\ncurrent_phase: 4\n---\n# State\n";
    let unreadable = "---\n- one\n- two\n---\n# State\n";
    for (name, content, expect_recovered, expect_unreadable) in [
        ("clean", clean, false, false),
        ("recovered", SENTRIQ_SHAPED_STATE_MD, true, false),
        ("unreadable", unreadable, false, true),
    ] {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("STATE.md"), content).unwrap();
        let state = parse_project_state(tmp.path());
        assert!(
            !(state.state_md_recovered && state.state_md_unreadable),
            "{name}: a file cannot be both repaired and unreadable"
        );
        assert_eq!(state.state_md_recovered, expect_recovered, "{name}");
        assert_eq!(state.state_md_unreadable, expect_unreadable, "{name}");
    }
}

/// An unreadable STATE.md carries WHERE it broke, as FILE-relative numbers, all
/// the way to the struct the render path reads.
#[test]
fn an_unreadable_state_md_carries_its_fault_position_to_the_render_path() {
    let tmp = TempDir::new().unwrap();
    // The repair cannot help here: the fault is an unclosed flow collection on
    // an indented line, which the candidate rule declines to touch.
    fs::write(
        tmp.path().join("STATE.md"),
        "---\nstatus: planning\nprogress:\n  total_phases: [4\n---\n# State\n",
    )
    .unwrap();
    let state = parse_project_state(tmp.path());
    assert!(state.state_md_unreadable);
    assert!(!state.state_md_recovered);
    let position = state
        .state_md_fault_position
        .expect("the parser located this fault");
    assert!(
        position.line >= 4,
        "the fault is at or after the file's line 4; got {}",
        position.line
    );
    assert!(position.column > 0);
}

/// The real foreign file, READ-ONLY, addressed by an environment variable.
///
/// `#[ignore]`d and self-skipping, on the precedent this module already sets
/// for real-file reads: the repository commits the SHAPE, and reading an actual
/// file on a particular machine is opt-in. No absolute path is committed, and
/// nothing here writes, creates or truncates anything.
///
/// Run it with:
/// `GSD_META_MANAGER_REAL_STATE_MD=/path/to/.planning/STATE.md \
///  cargo test --test state_reader_test -- --ignored --nocapture`
#[test]
#[ignore = "reads a real file outside the repository; opt in with GSD_META_MANAGER_REAL_STATE_MD"]
fn a_real_foreign_state_md_reads_read_only() {
    let Ok(path) = std::env::var("GSD_META_MANAGER_REAL_STATE_MD") else {
        println!("GSD_META_MANAGER_REAL_STATE_MD unset — nothing to read");
        return;
    };
    let Ok(content) = fs::read_to_string(&path) else {
        println!("{path}: unreadable");
        return;
    };
    match read_frontmatter(&content) {
        FrontmatterOutcome::Parsed(fm) => {
            println!("Parsed (clean). stopped_at = {:?}", fm.stopped_at);
        }
        FrontmatterOutcome::Recovered {
            frontmatter,
            repaired_lines,
        } => {
            println!("Recovered ({repaired_lines} line(s) repaired in memory).");
            println!("  status        = {:?}", frontmatter.status);
            println!("  current_phase = {:?}", frontmatter.current_phase);
            println!("  stopped_at    = {} bytes", frontmatter.stopped_at.len());
            println!("  stopped_at    = {:?}", frontmatter.stopped_at);
        }
        FrontmatterOutcome::Absent => println!("Absent: no frontmatter block"),
        FrontmatterOutcome::Unreadable { fault, position } => {
            println!("Unreadable: {} at {position:?}", fault.describe());
        }
    }
}

/// A STATE.md carrying the quoted version GSD writes must yield its phase, not
/// a flag. This is the whole bug, asserted end-to-end through the real reader.
#[test]
fn the_quoted_state_version_no_longer_blanks_the_phase() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\ncurrent_phase: 4\ncurrent_phase_name: Pixel over ADB\n\
         status: executing\nprogress:\n  total_phases: 8\n  completed_phases: 1\n---\n# State\n",
    )
    .unwrap();
    let state = parse_project_state(tmp.path());
    assert!(!state.state_md_unreadable);
    assert_eq!(state.status, "executing");
    assert_eq!(state.current_phase, "Pixel over ADB");
    assert_eq!(state.current_phase_name, "Pixel over ADB");
}

// ============================================================================
// The active phase is the disk frontier, not the roadmap's completion count
// ============================================================================

/// picsync's shape, reduced: the roadmap's `## Progress` table says 2 of 8
/// complete while phase 4 is already planned on disk. `completed_phases + 1`
/// answers 3; the frontier answers 4, and 4 is what STATE.md says too.
#[test]
fn the_active_phase_is_the_disk_frontier_not_the_completion_count() {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();

    fs::write(
        planning.join("ROADMAP.md"),
        // The `## Progress` table shape GSD writes (copied from picsync's):
        // phase 3 is "In Progress", so the table counts 2 complete — while
        // phase 4 is already planned on disk.
        "# Roadmap\n\n\
         - [x] **Phase 1: Bootstrap** - one\n\
         - [x] **Phase 2: Ingest** - two\n\
         - [ ] **Phase 3: Vertical Slice** - three\n\
         - [ ] **Phase 4: Pixel over ADB** - four\n\n\
         ## Progress\n\n\
         | Phase | Plans Complete | Status | Completed |\n\
         |-------|----------------|--------|-----------|\n\
         | 1. Bootstrap | 6/6 | Complete | 2026-09-09 |\n\
         | 2. Ingest | 11/11 | Complete | 2026-09-09 |\n\
         | 3. Vertical Slice | 7/7 | In Progress — awaiting live checks |  |\n\
         | 4. Pixel over ADB | 0/TBD | Not started | - |\n",
    )
    .unwrap();

    let phases = planning.join("phases");
    // Phases 1-3 executed (plan + summary each), phase 4 planned only.
    for n in ["01", "02", "03"] {
        let dir = phases.join(format!("{}-done", n));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{}-01-PLAN.md", n)), "# Plan").unwrap();
        fs::write(dir.join(format!("{}-01-SUMMARY.md", n)), "# Summary").unwrap();
    }
    let p4 = phases.join("04-pixel-over-adb");
    fs::create_dir_all(&p4).unwrap();
    fs::write(p4.join("04-01-PLAN.md"), "# Plan").unwrap();

    let state = parse_project_state(planning);

    // The stale count that used to drive every label.
    assert_eq!(state.completed_phases, 2);
    // What the disk actually says.
    assert_eq!(
        state.current_phase_number,
        Some(4),
        "phase 4 is planned but not executed — it is the frontier"
    );
    assert_eq!(state.active_phase_number(), 4);
}

/// Without a parsable roadmap there is no frontier to prefer, so the old
/// arithmetic is still the answer.
#[test]
fn without_roadmap_phases_the_active_phase_falls_back_to_the_count() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nstatus: executing\nprogress:\n  total_phases: 5\n  completed_phases: 2\n---\n",
    )
    .unwrap();
    let state = parse_project_state(tmp.path());
    assert_eq!(state.current_phase_number, None);
    assert_eq!(state.active_phase_number(), 3);
}

/// The direction the clamp PRESERVES: STATE.md ahead of the disk frontier still
/// wins. GSD advances `current_phase` on discussion, before any artifact of the
/// new phase exists, so it is the only witness that work has moved on — and a
/// frontier held back by an earlier phase left unfinished must not drag the
/// dashboard back to it.
///
/// Phase 2 was deferred (planned, never executed) so the frontier sits at 2,
/// while phase 3 is executed and phase 4 is where STATE.md says the work is.
#[test]
fn state_md_phase_number_wins_when_it_leads_the_disk_frontier() {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();
    fs::write(
        planning.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nstatus: planning\ncurrent_phase: 4\n---\n",
    )
    .unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [x] **Phase 1: One** - a\n\
         - [ ] **Phase 2: Two** - b\n\
         - [ ] **Phase 3: Three** - c\n\
         - [ ] **Phase 4: Four** - d\n",
    )
    .unwrap();
    let phases = planning.join("phases");
    // Phases 1 and 3 executed; phase 2 deferred with plans only, so it — not
    // phase 4 — is the first phase below `Executed` and therefore the frontier.
    for n in ["01", "03"] {
        let dir = phases.join(format!("{}-x", n));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{}-01-PLAN.md", n)), "# Plan").unwrap();
        fs::write(dir.join(format!("{}-01-SUMMARY.md", n)), "# Summary").unwrap();
    }
    let p2 = phases.join("02-deferred");
    fs::create_dir_all(&p2).unwrap();
    fs::write(p2.join("02-01-PLAN.md"), "# Plan").unwrap();
    // Phase 4: discussed only — no directory on disk yet.

    let state = parse_project_state(planning);
    assert_eq!(
        state.current_phase_number,
        Some(2),
        "the frontier is the first phase below Executed"
    );
    assert_eq!(state.state_md_phase_number, Some(4));
    assert_eq!(
        state.active_phase_number(),
        4,
        "STATE.md leads the frontier, and leading is allowed"
    );
}

// ============================================================================
// The per-phase marker is that phase's own disk evidence
// ============================================================================

/// Build picsync's exact shape: a phase that is COMPLETE on disk (plans,
/// matching summaries, a passing `*-VERIFICATION.md`) while ROADMAP's `## Phases`
/// checkbox is still `- [ ]` and its `## Progress` Status cell says something
/// other than the literal `Complete`/`Done` the counter matches.
///
/// **STATE.md's `current_phase` is picsync's real value — 3, one BEHIND the
/// disk frontier**, not the 4 this fixture used to claim. That single digit is
/// the whole of the `active_phase_number` clamp bug: a GSD agent with a partial
/// view wrote 3 while phase 3 was already `Complete`/`Passed` on disk and phase
/// 4 was already planned, and the `Option::or` ladder let it outrank both.
fn picsync_shaped_planning_dir() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();

    fs::write(
        planning.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nstatus: executing\ncurrent_phase: 3\n---\n",
    )
    .unwrap();

    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [x] **Phase 1: Bootstrap** - one\n\
         - [x] **Phase 2: Ingest** - two\n\
         - [ ] **Phase 3: Vertical Slice** - three\n\
         - [ ] **Phase 4: Pixel over ADB** - four\n\
         - [ ] **Phase 5: iPhone over AFC** - five\n\n\
         ## Progress\n\n\
         | Phase | Plans Complete | Status | Completed |\n\
         |-------|----------------|--------|-----------|\n\
         | 1. Bootstrap | 6/6 | Complete | 2026-09-09 |\n\
         | 2. Ingest | 11/11 | Complete | 2026-09-09 |\n\
         | 3. Vertical Slice | 7/7 | In Progress — verified, awaiting 2 live checks |  |\n\
         | 4. Pixel over ADB | 0/TBD | Not started | - |\n\
         | 5. iPhone over AFC | 0/TBD | Not started | - |\n",
    )
    .unwrap();

    let phases = planning.join("phases");
    // Phases 1-3: implementation done AND verification passed => Complete.
    for n in ["01", "02", "03"] {
        let dir = phases.join(format!("{}-done", n));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{}-01-PLAN.md", n)), "# Plan").unwrap();
        fs::write(dir.join(format!("{}-01-SUMMARY.md", n)), "# Summary").unwrap();
        fs::write(
            dir.join(format!("{}-VERIFICATION.md", n)),
            "---\nstatus: passed\n---\n# Verification\n",
        )
        .unwrap();
    }
    // Phase 4: plans written, nothing executed.
    let p4 = phases.join("04-pixel-over-adb");
    fs::create_dir_all(&p4).unwrap();
    fs::write(p4.join("04-01-PLAN.md"), "# Plan").unwrap();
    // Phase 5: no directory at all.

    tmp
}

/// The regression this test exists for: with `active_phase_number()` correctly
/// reporting 4, phase 3 matched neither "done" (its checkbox is unchecked and
/// the count that used to stand in for done says 2) nor "current", and rendered
/// as `o` — *future* — for a phase with a passing verification on disk.
#[test]
fn a_phase_complete_on_disk_but_unchecked_in_the_roadmap_reads_done() {
    let tmp = picsync_shaped_planning_dir();
    let state = parse_project_state(tmp.path());

    // The two unreliable inputs, unchanged: the count still says 2, and phase
    // 3's checkbox is still unchecked. The marker must not depend on either.
    assert_eq!(state.completed_phases, 2, "the ## Progress table still says 2");
    let phase3 = state
        .phases
        .iter()
        .find(|p| p.number == "3")
        .expect("phase 3 parsed from the roadmap");
    assert!(!phase3.completed, "phase 3's `- [ ]` checkbox is still stale");

    assert_eq!(state.active_phase_number(), 4);
    assert_eq!(
        state.phase_marker(phase3),
        PhaseMarker::Done,
        "phase 3 is Complete on disk with verification passed"
    );
    assert_eq!(phase3_glyph(&state), "+");
}

fn phase3_glyph(state: &gsd_meta_manager::state_reader::ProjectState) -> &'static str {
    state
        .phases
        .iter()
        .find(|p| p.number == "3")
        .map(|p| state.phase_marker(p).glyph())
        .unwrap()
}

/// Every phase in picsync's shape, so a fix that only moves phase 3 and breaks
/// its neighbours cannot pass.
#[test]
fn each_phase_marker_follows_that_phases_own_disk_status() {
    let tmp = picsync_shaped_planning_dir();
    let state = parse_project_state(tmp.path());

    let marker_of = |n: &str| {
        let p = state.phases.iter().find(|p| p.number == n).unwrap();
        state.phase_marker(p)
    };

    assert_eq!(marker_of("1"), PhaseMarker::Done, "complete on disk");
    assert_eq!(marker_of("2"), PhaseMarker::Done, "complete on disk");
    assert_eq!(marker_of("3"), PhaseMarker::Done, "complete on disk");
    assert_eq!(marker_of("4"), PhaseMarker::Current, "planned, and active");
    assert_eq!(marker_of("5"), PhaseMarker::Future, "no directory yet");
}

/// A phase whose implementation is finished but whose verification has not
/// passed is behind the frontier, so it reads `+` — the same threshold
/// `parse_project_state` uses to place the frontier, not a second one. The
/// verification nuance is carried by the Detail screen's `[Executed]` badge.
#[test]
fn an_executed_but_unverified_phase_is_behind_the_frontier_and_reads_done() {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();
    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [ ] **Phase 1: One** - a\n\
         - [ ] **Phase 2: Two** - b\n",
    )
    .unwrap();
    let phases = planning.join("phases");
    let p1 = phases.join("01-one");
    fs::create_dir_all(&p1).unwrap();
    fs::write(p1.join("01-01-PLAN.md"), "# Plan").unwrap();
    fs::write(p1.join("01-01-SUMMARY.md"), "# Summary").unwrap();
    let p2 = phases.join("02-two");
    fs::create_dir_all(&p2).unwrap();
    fs::write(p2.join("02-01-PLAN.md"), "# Plan").unwrap();

    let state = parse_project_state(planning);
    let marker_of = |n: &str| {
        let p = state.phases.iter().find(|p| p.number == n).unwrap();
        state.phase_marker(p)
    };
    assert_eq!(state.active_phase_number(), 2);
    assert_eq!(marker_of("1"), PhaseMarker::Done);
    assert_eq!(marker_of("2"), PhaseMarker::Current);
}

/// The degrade path: no disk scan reached this phase at all, so the roadmap's
/// own bookkeeping is what is left. It must still produce a marker — the
/// pre-`PhaseMarker` answer — rather than declaring every phase unfinished.
#[test]
fn without_disk_inference_the_marker_falls_back_to_the_roadmap_checkbox() {
    use std::collections::HashMap;
    let empty = HashMap::new();

    assert_eq!(
        PhaseMarker::decide("1", true, &empty, 3),
        PhaseMarker::Done,
        "checked box, nothing on disk to contradict it"
    );
    assert_eq!(
        PhaseMarker::decide("3", false, &empty, 3),
        PhaseMarker::Current
    );
    assert_eq!(
        PhaseMarker::decide("7", false, &empty, 3),
        PhaseMarker::Future
    );
    // Zero-padded roadmap entries still match a bare active number.
    assert_eq!(
        PhaseMarker::decide("04", false, &empty, 4),
        PhaseMarker::Current
    );
}

/// Current outranks done, pinned on `decide` directly: a phase that is
/// `Executed` with verification `human_needed` — implementation finished, a
/// human's judgement outstanding — still draws `*` when it is the active one.
/// Under the opposite order the roadmap would draw `+` there and, for a project
/// whose active phase is a finished one, carry no `*` at all — the glyph a human
/// scans for, missing from the screen that exists to show it.
///
/// The active phase is supplied here rather than derived; which phase
/// `active_phase_number` picks is the clamp's business and is pinned
/// end-to-end above.
#[test]
fn the_active_phase_keeps_the_current_glyph_even_when_it_is_finished() {
    use gsd_meta_manager::state_reader::disk_status::{DiskInference, DiskStatus};
    use std::collections::HashMap;

    let mut disk = HashMap::new();
    disk.insert(
        "19".to_string(),
        DiskInference {
            status: DiskStatus::Executed,
            ..Default::default()
        },
    );
    assert_eq!(
        PhaseMarker::decide("19", false, &disk, 19),
        PhaseMarker::Current,
        "implementation done, verification awaiting a human, and named as current"
    );

    // Same order with the checkbox as the only witness.
    let empty = HashMap::new();
    assert_eq!(
        PhaseMarker::decide("5", true, &empty, 5),
        PhaseMarker::Current
    );
}

/// This repository's own disk shape: STATE.md names phase 19 while phases 20 and
/// 21 are further along on disk. 20 and 21 used to draw `o` — *future* — because
/// their roadmap checkboxes are unchecked and they are not the current phase.
///
/// The active phase is passed as 19 to isolate `decide`'s ordering. Note that
/// `active_phase_number` no longer returns 19 for this shape: 19/20/21 are all
/// at or above `Executed`, so the frontier — and therefore the clamp — is 22.
#[test]
fn phases_past_the_named_current_one_read_done_not_future() {
    use gsd_meta_manager::state_reader::disk_status::{DiskInference, DiskStatus};
    use std::collections::HashMap;

    let mut disk = HashMap::new();
    for (n, s) in [
        ("19", DiskStatus::Executed),
        ("20", DiskStatus::Complete),
        ("21", DiskStatus::Executed),
        ("22", DiskStatus::NoDirectory),
    ] {
        disk.insert(
            n.to_string(),
            DiskInference {
                status: s,
                ..Default::default()
            },
        );
    }

    assert_eq!(PhaseMarker::decide("19", false, &disk, 19), PhaseMarker::Current);
    assert_eq!(PhaseMarker::decide("20", false, &disk, 19), PhaseMarker::Done);
    assert_eq!(PhaseMarker::decide("21", false, &disk, 19), PhaseMarker::Done);
    assert_eq!(PhaseMarker::decide("22", false, &disk, 19), PhaseMarker::Future);
}

// ============================================================================
// The active phase is a CLAMP: max(STATE.md, disk frontier)
//
// STATE.md may lead the disk (a phase discussed or planned before any artifact
// of it exists) but must never drag the active phase below work that is
// demonstrably on disk. The `Option::or` ladder this replaces let picsync's
// `current_phase: 3` outrank a phase 3 that was `Complete`/`Passed` and a phase
// 4 that was already planned, so phase 3 drew `*` (current) on the same screen
// whose badge called it `[Complete]`.
// ============================================================================

/// picsync's exact reported shape, end to end: STATE.md says 3, the disk
/// frontier is 4, phase 3 is `Complete` with a passing verification. The active
/// phase must be 4 and phase 3 must read done.
#[test]
fn state_md_behind_the_disk_frontier_is_clamped_up_to_it() {
    let tmp = picsync_shaped_planning_dir();
    let state = parse_project_state(tmp.path());

    assert_eq!(
        state.state_md_phase_number,
        Some(3),
        "STATE.md's own number, one behind the frontier"
    );
    assert_eq!(
        state.current_phase_number,
        Some(4),
        "phase 4 is planned but not executed — it is the frontier"
    );
    assert_eq!(
        state.active_phase_number(),
        4,
        "the frontier is a floor: a STATE.md number cannot un-write work on disk"
    );

    // The self-contradiction the bug produced: `*` on a phase the same scan
    // calls Complete.
    let phase3 = state
        .phases
        .iter()
        .find(|p| p.number == "3")
        .expect("phase 3 parsed from the roadmap");
    assert_eq!(
        state.phase_marker(phase3),
        PhaseMarker::Done,
        "Complete on disk with verification passed — never `*`"
    );
    assert_eq!(
        state.phase_disk_statuses.get("3").map(|d| d.status),
        Some(gsd_meta_manager::state_reader::disk_status::DiskStatus::Complete),
        "the badge beside the glyph still says Complete — they must agree"
    );
}

/// The glyph column read off rendered cells, not off the decision that produced
/// them — the only form of this assertion that catches a marker which is decided
/// correctly and drawn wrong.
///
/// picsync's reported row, verbatim: `1:+ 2:+ 3:+ 4:* 5:o`. (Its real roadmap has
/// eight phases; this fixture carries the five the report turns on.)
#[test]
fn the_rendered_roadmap_draws_the_current_glyph_on_the_frontier_not_behind_it() {
    use gsd_meta_manager::ui::roadmap_widget::RoadmapWidget;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;

    let tmp = picsync_shaped_planning_dir();
    let state = parse_project_state(tmp.path());

    let area = Rect::new(0, 0, 60, 40);
    let mut buf = Buffer::empty(area);
    RoadmapWidget {
        phases: &state.phases,
        // The one notion of "current phase". Every call site goes through it.
        current_phase_num: state.active_phase_number(),
        disk_statuses: &state.phase_disk_statuses,
        scroll_offset: 0,
    }
    .render(area, &mut buf);

    assert_eq!(
        rendered_glyph_column(&buf, area),
        vec!['+', '+', '+', '*', 'o'],
        "phase 3 is done on disk; the frontier, phase 4, is where the `*` belongs"
    );
}

/// The glyph column, read off the rendered cells. Mirrors the helper in
/// `ui::roadmap_widget`'s own tests: a content line is the one carrying
/// `P<number>:`, and the glyph sits immediately before its leading space.
fn rendered_glyph_column(buf: &ratatui::buffer::Buffer, area: ratatui::layout::Rect) -> Vec<char> {
    let mut out = Vec::new();
    for y in area.y..area.y + area.height {
        let row: String = (area.x..area.x + area.width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<Vec<_>>()
            .concat();
        if let Some(i) = row.find(" P") {
            if row[i + 2..].starts_with(|c: char| c.is_ascii_digit()) {
                if let Some(g) = row[..i].chars().next_back() {
                    out.push(g);
                }
            }
        }
    }
    out
}

/// Zero-padded roadmap ids and a zero-padded STATE.md number resolve to the same
/// integers as bare ones — end to end, through the real parse. This codebase has
/// shipped the `04` vs `4` bug class before.
#[test]
fn zero_padded_phase_ids_clamp_and_mark_like_bare_ones() {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();
    fs::write(
        planning.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nstatus: executing\ncurrent_phase: \"03\"\n---\n",
    )
    .unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [ ] **Phase 03: Three** - c\n\
         - [ ] **Phase 04: Four** - d\n\
         - [ ] **Phase 05: Five** - e\n",
    )
    .unwrap();
    let phases = planning.join("phases");
    let p3 = phases.join("03-three");
    fs::create_dir_all(&p3).unwrap();
    fs::write(p3.join("03-01-PLAN.md"), "# Plan").unwrap();
    fs::write(p3.join("03-01-SUMMARY.md"), "# Summary").unwrap();
    let p4 = phases.join("04-four");
    fs::create_dir_all(&p4).unwrap();
    fs::write(p4.join("04-01-PLAN.md"), "# Plan").unwrap();

    let state = parse_project_state(planning);
    assert_eq!(
        state.state_md_phase_number,
        Some(3),
        "`03` is the integer 3, not a string"
    );
    assert_eq!(
        state.current_phase_number,
        Some(4),
        "the frontier, from `04`"
    );
    assert_eq!(state.active_phase_number(), 4);

    let marker_of = |n: &str| {
        let p = state.phases.iter().find(|p| p.number == n).unwrap();
        state.phase_marker(p)
    };
    assert_eq!(marker_of("03"), PhaseMarker::Done);
    assert_eq!(
        marker_of("04"),
        PhaseMarker::Current,
        "the zero-padded roadmap id matches the bare active number"
    );
    assert_eq!(marker_of("05"), PhaseMarker::Future);
}

/// The comparison is numeric, not lexical. `"9" > "10"` as strings; 9 < 10 as
/// numbers, and the frontier at 10 must win.
#[test]
fn the_clamp_compares_numerically_not_lexically() {
    use gsd_meta_manager::state_reader::ProjectState;

    let state = ProjectState {
        state_md_phase_number: Some(9),
        current_phase_number: Some(10),
        ..Default::default()
    };
    assert_eq!(
        state.active_phase_number(),
        10,
        "lexical max would answer 9 — `\"9\"` sorts after `\"10\"`"
    );

    // And the same the other way: a leading STATE.md still leads.
    let state = ProjectState {
        state_md_phase_number: Some(10),
        current_phase_number: Some(9),
        ..Default::default()
    };
    assert_eq!(state.active_phase_number(), 10);

    // Equal sources are a no-op, not a bump.
    let state = ProjectState {
        state_md_phase_number: Some(7),
        current_phase_number: Some(7),
        ..Default::default()
    };
    assert_eq!(state.active_phase_number(), 7);
}

/// Every degrade path in one place: either source may be absent, and the
/// arithmetic fallback fires only when BOTH are.
#[test]
fn the_clamp_degrades_when_a_source_is_missing() {
    use gsd_meta_manager::state_reader::ProjectState;

    // Only STATE.md (no parsable roadmap phases, so no frontier).
    let state = ProjectState {
        state_md_phase_number: Some(6),
        current_phase_number: None,
        completed_phases: 1,
        ..Default::default()
    };
    assert_eq!(
        state.active_phase_number(),
        6,
        "one source present — it answers; the count does not get a vote"
    );

    // Only the frontier (no STATE.md, or an unreadable one).
    let state = ProjectState {
        state_md_phase_number: None,
        current_phase_number: Some(4),
        completed_phases: 1,
        ..Default::default()
    };
    assert_eq!(state.active_phase_number(), 4);

    // Neither: the ROADMAP `## Progress` count, the weakest source, and the only
    // case in which it is consulted at all.
    let state = ProjectState {
        state_md_phase_number: None,
        current_phase_number: None,
        completed_phases: 2,
        ..Default::default()
    };
    assert_eq!(state.active_phase_number(), 3);

    // Nothing at all: phase 1 is where a project with no evidence starts.
    assert_eq!(ProjectState::default().active_phase_number(), 1);
}

/// A roadmap of non-numeric / prefixed ids (`M-2`, `0.3`, `4a`) must not panic
/// and must not collapse to phase 0. Both clamp sources parse with
/// `parse::<u32>().ok()`, so such an id arrives as `None` and simply does not
/// participate — the tolerance that already existed upstream, preserved.
#[test]
fn non_numeric_phase_ids_do_not_panic_or_become_zero() {
    let tmp = TempDir::new().unwrap();
    let planning = tmp.path();
    fs::write(
        planning.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nstatus: executing\ncurrent_phase: M-2\nprogress:\n  total_phases: 3\n  completed_phases: 1\n---\n",
    )
    .unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [x] **Phase M-1: Migration One** - a\n\
         - [ ] **Phase M-2: Migration Two** - b\n\
         - [ ] **Phase 0.3: Fractional** - c\n",
    )
    .unwrap();

    let state = parse_project_state(planning);
    assert_eq!(
        state.state_md_phase_number, None,
        "`M-2` is not a u32 — it abstains rather than parsing as 0"
    );
    assert_eq!(
        state.current_phase_number, None,
        "no roadmap id parses, so there is no numeric frontier either"
    );
    // Both absent: the arithmetic fallback, and never 0.
    assert_eq!(state.active_phase_number(), 2, "completed_phases + 1");

    // Markers still resolve for every phase — no panic, none spuriously current.
    for p in &state.phases {
        assert_ne!(
            state.phase_marker(p),
            PhaseMarker::Current,
            "no non-numeric id can match a numeric active phase"
        );
    }
}
