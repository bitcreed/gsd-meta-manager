pub mod backlog;
pub mod config_json;
pub mod disk_status;
pub mod git_ops;
pub mod queue_md;
pub mod roadmap_md;
pub mod state_md;
pub mod workstreams;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectState {
    pub status: String,
    pub current_phase: String,
    /// ADR-2207: human-readable current phase name from STATE.md frontmatter
    /// (`current_phase_name`); empty when absent.
    pub current_phase_name: String,
    /// ADR-2207: current plan identifier from STATE.md frontmatter
    /// (`current_plan`); empty when absent.
    pub current_plan: String,
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
    pub milestone: String,
    pub backlog_count: u32,
    pub phases: Vec<roadmap_md::RoadmapPhase>,
    pub queued_actions: Vec<queue_md::QueuedAction>,
    /// Per-phase disk inference keyed by phase number (e.g., "01", "05")
    pub phase_disk_statuses: HashMap<String, disk_status::DiskInference>,
    /// Disk inference for the current/active phase: the first phase whose
    /// **implementation** is unfinished (status below
    /// [`disk_status::DiskStatus::Executed`]), or the last phase when every one
    /// of them is implemented.
    ///
    /// The threshold is implementation and not verification, deliberately — see
    /// the comment at the assignment site in [`parse_project_state`] (WR-04).
    pub current_phase_status: Option<disk_status::DiskInference>,
    /// Whether the project has a non-empty HANDOFF.md or HANDOFF.json in .planning/
    pub paused: bool,
    /// Extracted context from HANDOFF file (next_action from JSON, or first content line from MD)
    pub pause_context: Option<String>,
    /// True when `.planning/async-jobs/` holds at least one `*.json` manifest —
    /// the phase is legitimately waiting on an external job (not stuck).
    pub external_job_waiting: bool,
    /// Most recent activity timestamp (last commit, mtime fallback) for the project.
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// Filesystem root of the project (the directory containing `.planning/`).
    pub project_root: PathBuf,
    /// G10: a **non-empty** `.planning/.continue-here.md` at the project root.
    ///
    /// A hard stop whose only bypass is `--force` (`next.md:46-58`). The content
    /// check, rather than a bare existence check, follows the precedent
    /// [`detect_handoff`] already sets in this file: a file trimmed to nothing
    /// is a leftover, not a signal.
    pub continue_here_present: bool,
    /// G15: the phases carrying a `## Deferred Verification` row in STATE.md.
    ///
    /// Named rather than counted, because what a router or a human needs is
    /// *which* phase still owes a verification, not how many do.
    pub deferred_verification_phases: Vec<String>,
    /// GSD 1.8.0 parallel workstreams under `.planning/workstreams/<ws>/`.
    /// Empty for flat projects. Bounded to one level: a workstream's own
    /// sub-state carries an empty `workstreams` vec (recursion guard below).
    pub workstreams: Vec<workstreams::WorkstreamState>,
}

/// Detect HANDOFF.md or HANDOFF.json in a planning directory.
/// Returns (is_paused, optional_context_string).
/// HANDOFF.json: extracts `next_action` field.
/// HANDOFF.md: extracts first non-empty line after any `#` heading, or first non-empty line.
/// Empty files (after trim) are ignored -- not considered paused.
fn detect_handoff(planning_dir: &Path) -> (bool, Option<String>) {
    // Try HANDOFF.json first
    let json_path = planning_dir.join("HANDOFF.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if !content.trim().is_empty() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let ctx = val
                    .get("next_action")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                return (true, ctx);
            }
            // Non-empty but invalid JSON -- still paused, no context
            return (true, None);
        }
    }

    // Try HANDOFF.md
    let md_path = planning_dir.join("HANDOFF.md");
    if let Ok(content) = std::fs::read_to_string(&md_path) {
        if !content.trim().is_empty() {
            // Extract first non-empty line after any # heading line, or first non-empty line
            let mut found_heading = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    found_heading = true;
                    continue;
                }
                if !trimmed.is_empty() {
                    return (true, Some(trimmed.to_string()));
                }
            }
            // File has content but only headings or whitespace
            if found_heading {
                return (true, None);
            }
            return (true, None);
        }
    }

    (false, None)
}

/// Parse a GSD project's .planning/ directory into a ProjectState.
/// Gracefully handles missing or malformed files -- never panics.
pub fn parse_project_state(planning_dir: &Path) -> ProjectState {
    let mut state = ProjectState {
        status: "unknown".to_string(),
        ..Default::default()
    };

    // Resolve the project root (parent of .planning/) and its last-activity
    // timestamp (last commit time, mtime fallback).
    state.project_root = planning_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| planning_dir.to_path_buf());
    state.last_activity = git_ops::project_last_activity(&state.project_root);

    // Parse STATE.md
    let state_md_path = planning_dir.join("STATE.md");
    if let Ok(content) = std::fs::read_to_string(&state_md_path) {
        if let Some(fm) = state_md::parse_state_md(&content) {
            state.status = if fm.status.is_empty() {
                "unknown".to_string()
            } else {
                fm.status
            };
            state.total_phases = fm.progress.total_phases;
            state.completed_phases = fm.progress.completed_phases;
            state.total_plans = fm.progress.total_plans;
            state.completed_plans = fm.progress.completed_plans;

            // ADR-2207 frontmatter keys (empty when absent).
            state.current_phase_name = fm.current_phase_name.clone().unwrap_or_default();
            state.current_plan = fm.current_plan.clone().unwrap_or_default();

            // Derive current_phase. Preference order:
            //   1. explicit `current_phase_name` frontmatter,
            //   2. explicit `current_phase` frontmatter (as `Phase {n}`),
            //   3. ADR-2207 status: milestone-terminal renders `<milestone> Complete`,
            //      but the intermediate `All phases complete` must NOT (the milestone
            //      is not done — it awaits `/gsd:complete-milestone`),
            //   4. the legacy stopped_at / count-based completion heuristic.
            if let Some(name) = fm
                .current_phase_name
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                state.current_phase = name.to_string();
            } else if let Some(ph) = fm.current_phase.as_deref().filter(|s| !s.is_empty()) {
                state.current_phase = format!("Phase {}", ph);
            } else if state_md::is_milestone_terminal(&state.status) {
                state.current_phase = if !fm.milestone.is_empty() {
                    format!("{} Complete", fm.milestone)
                } else {
                    "Complete".to_string()
                };
            } else if state_md::is_all_phases_complete(&state.status) {
                state.current_phase = "All phases complete".to_string();
            } else if !fm.stopped_at.is_empty() {
                state.current_phase = fm.stopped_at.clone();
            } else if fm.progress.completed_phases >= fm.progress.total_phases
                && fm.progress.total_phases > 0
            {
                state.current_phase = if !fm.milestone.is_empty() {
                    format!("{} Complete", fm.milestone)
                } else {
                    "Complete".to_string()
                };
            } else {
                state.current_phase = format!("Phase {}", fm.progress.completed_phases + 1);
            }

            state.milestone = fm.milestone;
        }
        // G15 reads the document body, not the frontmatter, so it is parsed
        // from the same content regardless of whether the frontmatter parsed.
        state.deferred_verification_phases = state_md::deferred_verification_phases(&content);
    }

    // G10: a non-empty project-root continue-here marker. Content check, not
    // existence check, on `detect_handoff`'s precedent.
    state.continue_here_present = std::fs::read_to_string(planning_dir.join(".continue-here.md"))
        .map(|content| !content.trim().is_empty())
        .unwrap_or(false);

    // Parse ROADMAP.md. The `## Progress` table (GSD 1.8.0) is authoritative
    // for progress counts when present; otherwise STATE.md frontmatter stands.
    let roadmap_path = planning_dir.join("ROADMAP.md");
    if let Ok(content) = std::fs::read_to_string(&roadmap_path) {
        state.phases = roadmap_md::parse_roadmap_phases(&content);
        if let Some(prog) = roadmap_md::roadmap_progress(&content) {
            state.total_phases = prog.total_phases;
            state.completed_phases = prog.completed_phases;
            state.total_plans = prog.total_plans;
            state.completed_plans = prog.completed_plans;
        }
    }

    // Run disk inference for each phase
    let mut current_phase_inference: Option<disk_status::DiskInference> = None;
    for phase in &state.phases {
        let inference = disk_status::infer_phase_status(planning_dir, &phase.number);
        // Track the first phase whose IMPLEMENTATION is not finished as the
        // current phase status.
        //
        // **`< Executed`, and the threshold is a decision rather than an
        // inheritance (WR-04).** This condition read `!= Complete` until Phase
        // 20 made `Complete` a conjunction (implementation AND verification
        // passed) and inserted `Executed` beneath it. That silently moved the
        // threshold: every phase awaiting verification now reads `Executed`, so
        // the loop stopped at the first *unverified* phase instead of the first
        // *unexecuted* one, and this repository's own dashboard cell moved from
        // phase 20 to phase 19.
        //
        // The frontier is implementation, and the reason is what this value
        // feeds: the dashboard row's compact D-R-P-E-V cell
        // (`ui/screens/normal.rs`) sits beside a phase LABEL taken from
        // STATE.md's `current_phase`, which GSD advances on execution. A cell
        // describing a different phase from the one its own row names is worse
        // than a cell that omits something. The verification gate is not lost by
        // this choice — it surfaces through the needs-human badge (D-24) and,
        // for the driver, through the DRIVE-05 gate set, both of which read the
        // per-phase inference directly rather than this summary.
        //
        // Written against the `Ord` Phase 20 added, so the next variant inserted
        // below `Executed` is included automatically and one inserted above it
        // is not — which is the behaviour a threshold wants and the reason this
        // is a comparison rather than a list of variants.
        if current_phase_inference.is_none() && inference.status < disk_status::DiskStatus::Executed
        {
            current_phase_inference = Some(inference.clone());
        }
        state
            .phase_disk_statuses
            .insert(phase.number.clone(), inference);
    }
    // If every phase is implemented, use the last phase's status — including
    // when some of them are still awaiting verification, which is the same
    // threshold the loop above uses and for the same reason.
    if current_phase_inference.is_none() && !state.phases.is_empty() {
        if let Some(last) = state.phases.last() {
            current_phase_inference = state.phase_disk_statuses.get(&last.number).cloned();
        }
    }
    state.current_phase_status = current_phase_inference;

    // Count backlog items
    state.backlog_count = count_backlog_items(planning_dir);

    // Load queued actions from QUEUE.md
    state.queued_actions = queue_md::load_queue(planning_dir);

    // Detect HANDOFF files for pause state
    let (paused, pause_context) = detect_handoff(planning_dir);
    state.paused = paused;
    state.pause_context = pause_context;

    // Detect async external jobs (a phase waiting on an external job is
    // legitimately blocked, not stuck).
    state.external_job_waiting = detect_async_jobs(planning_dir);

    // Load parallel workstreams (GSD 1.8.0). Recursion guard: skip when this
    // planning dir is itself a workstream (immediate parent named `workstreams`),
    // so a workstream sub-parse never re-enters workstream loading. This bounds
    // recursion depth to one level.
    let inside_workstream = planning_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some("workstreams");
    if !inside_workstream {
        state.workstreams = workstreams::load_workstreams(planning_dir);
    }

    state
}

/// Returns true when `.planning/async-jobs/` contains at least one `*.json`
/// manifest. GSD 1.8.0 writes `.planning/async-jobs/<job>.json` for a phase
/// waiting on an external (long-running) job.
fn detect_async_jobs(planning_dir: &Path) -> bool {
    let dir = planning_dir.join("async-jobs");
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && e.path().extension().and_then(|x| x.to_str()) == Some("json")
            })
        })
        .unwrap_or(false)
}

/// Count directories matching the 999* pattern in .planning/phases/.
/// These represent backlog items in GSD projects.
pub fn count_backlog_items(planning_dir: &Path) -> u32 {
    let phases_dir = planning_dir.join("phases");
    std::fs::read_dir(&phases_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("999"))
                        .unwrap_or(false)
                        && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a temp project with a `.planning/` dir and the given files
    /// (relative paths under `.planning/`). Returns the TempDir (keep it alive).
    fn make_planning(files: &[(&str, &str)]) -> TempDir {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        fs::create_dir_all(&planning).unwrap();
        for (rel, content) in files {
            let path = planning.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
        td
    }

    #[test]
    fn test_frontmatter_current_phase_name_flows_into_state() {
        let td = make_planning(&[(
            "STATE.md",
            "---\nstatus: executing\ncurrent_phase: 14\ncurrent_phase_name: Live State\ncurrent_plan: \"0.3\"\n---\n",
        )]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase_name, "Live State");
        assert_eq!(state.current_plan, "0.3");
        // current_phase prefers the explicit frontmatter phase name.
        assert_eq!(state.current_phase, "Live State");
        // project_root is the directory containing .planning/
        assert_eq!(state.project_root, td.path());
    }

    #[test]
    fn test_frontmatter_current_phase_number_fallback() {
        // Only current_phase present (no name) → `Phase {n}`.
        let td = make_planning(&[(
            "STATE.md",
            "---\nstatus: executing\ncurrent_phase: 14\n---\n",
        )]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "Phase 14");
    }

    #[test]
    fn test_progress_table_overrides_frontmatter_counts() {
        let state_md = "---\nstatus: executing\nprogress:\n  total_phases: 9\n  completed_phases: 9\n  total_plans: 20\n  completed_plans: 20\n---\n";
        let roadmap = "## Progress\n\n| Phase | Plans Complete | Status | Completed |\n| --- | --- | --- | --- |\n| 1. Alpha | 2/2 | Complete | ✅ |\n| 2. Beta | 1/3 | In Progress | |\n";
        let td = make_planning(&[("STATE.md", state_md), ("ROADMAP.md", roadmap)]);
        let state = parse_project_state(&td.path().join(".planning"));
        // The ## Progress table is authoritative and overrides the frontmatter.
        assert_eq!(state.total_phases, 2);
        assert_eq!(state.completed_phases, 1);
        assert_eq!(state.total_plans, 5);
        assert_eq!(state.completed_plans, 3);
    }

    #[test]
    fn test_no_progress_table_uses_frontmatter_counts() {
        let state_md = "---\nstatus: executing\nprogress:\n  total_phases: 4\n  completed_phases: 2\n  total_plans: 8\n  completed_plans: 5\n---\n";
        let roadmap = "# Roadmap\n\n- [ ] **Phase 1: Alpha** - no progress table\n";
        let td = make_planning(&[("STATE.md", state_md), ("ROADMAP.md", roadmap)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.total_phases, 4);
        assert_eq!(state.completed_phases, 2);
        assert_eq!(state.total_plans, 8);
        assert_eq!(state.completed_plans, 5);
    }

    #[test]
    fn test_async_jobs_sets_external_job_waiting() {
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("async-jobs/job1.json", "{\"id\":\"job1\"}"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.external_job_waiting);
    }

    #[test]
    fn test_no_async_jobs_means_not_waiting() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.external_job_waiting);
    }

    #[test]
    fn test_handoff_md_non_empty_sets_paused() {
        // UI-SPEC UIFIX-01 row 1: non-empty HANDOFF.md after trim → paused,
        // context is the first non-empty non-heading line.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "HANDOFF.md",
                "# Handoff\n\nPick up at plan 14-02\n\nmore detail\n",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert_eq!(state.pause_context.as_deref(), Some("Pick up at plan 14-02"));
    }

    #[test]
    fn test_handoff_json_non_empty_sets_paused_with_context() {
        // UI-SPEC UIFIX-01 row 2: HANDOFF.json `next_action` becomes the context.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "HANDOFF.json",
                "{\"next_action\":\"Run /gsd-execute-phase 14\"}",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert_eq!(
            state.pause_context.as_deref(),
            Some("Run /gsd-execute-phase 14")
        );
    }

    #[test]
    fn test_handoff_md_whitespace_only_is_not_paused() {
        // UI-SPEC UIFIX-01 row 3: a whitespace-only file is not a pause signal.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("HANDOFF.md", "   \n\t\n  \n"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_no_handoff_file_is_not_paused() {
        // UI-SPEC UIFIX-01 row 4: no HANDOFF file at all.
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_handoff_json_invalid_is_paused_without_context() {
        // UI-SPEC UIFIX-01 row 8: the badge never depends on JSON parsing.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("HANDOFF.json", "{not valid json at all"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_all_phases_complete_is_intermediate() {
        // `All phases complete` (ADR-2207 intermediate) must NOT render a
        // premature `<milestone> Complete`.
        let state_md = "---\nstatus: All phases complete\nmilestone: v1.5.0\nprogress:\n  total_phases: 5\n  completed_phases: 5\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "All phases complete");
    }

    #[test]
    fn test_milestone_terminal_renders_complete() {
        let state_md = "---\nstatus: v1.5.0 milestone complete\nmilestone: v1.5.0\nprogress:\n  total_phases: 5\n  completed_phases: 5\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "v1.5.0 Complete");
    }

    #[test]
    fn test_legacy_project_preserves_prior_behavior() {
        // No new artifacts: frontmatter counts, stopped_at heuristic, empty new
        // fields, and no external job — all as before.
        let state_md = "---\nstatus: planning\nmilestone: v1.0\nstopped_at: Phase 1 context gathered\nprogress:\n  total_phases: 4\n  completed_phases: 0\n  total_plans: 0\n  completed_plans: 0\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.status, "planning");
        assert_eq!(state.total_phases, 4);
        assert_eq!(state.completed_phases, 0);
        assert_eq!(state.current_phase, "Phase 1 context gathered");
        assert_eq!(state.current_phase_name, "");
        assert_eq!(state.current_plan, "");
        assert!(!state.external_job_waiting);
        // project_root and last_activity are populated for any project.
        assert_eq!(state.project_root, td.path());
        assert!(state.last_activity.is_some());
    }

    #[test]
    fn test_workstreams_populated_on_project_state() {
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/alpha/STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
            ("active-workstream", "alpha\n"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.workstreams.len(), 2);
        // Bounded recursion: each workstream sub-state has no nested workstreams.
        assert!(state.workstreams.iter().all(|w| w.state.workstreams.is_empty()));
        // Active flag flows through from the active-workstream pointer.
        assert!(state.workstreams.iter().find(|w| w.name == "alpha").unwrap().active);
    }

    #[test]
    fn test_flat_project_has_empty_workstreams() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.workstreams.is_empty());
    }
}
