use gsd_manager::state_reader::config_json::parse_gsd_config;
use gsd_manager::state_reader::roadmap_md::parse_roadmap_phases;
use gsd_manager::state_reader::state_md::{extract_frontmatter, parse_state_md};
use gsd_manager::state_reader::{count_backlog_items, parse_project_state};
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
    let content = r#"{"mode": "yolo", "granularity": "coarse", "extra_field": true, "workflow": {}}"#;
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
    assert_eq!(state.gsd_mode, "yolo");
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
    assert_eq!(state.gsd_mode, "");
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
