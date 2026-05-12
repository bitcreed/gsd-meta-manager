pub mod backlog;
pub mod config_json;
pub mod disk_status;
pub mod git_ops;
pub mod queue_md;
pub mod roadmap_md;
pub mod state_md;

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectState {
    pub status: String,
    pub current_phase: String,
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
    /// Disk inference for the current/active phase (first non-complete, or last if all complete)
    pub current_phase_status: Option<disk_status::DiskInference>,
    /// Whether the project has a non-empty HANDOFF.md or HANDOFF.json in .planning/
    pub paused: bool,
    /// Extracted context from HANDOFF file (next_action from JSON, or first content line from MD)
    pub pause_context: Option<String>,
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

            // Derive current_phase from stopped_at, completion status, or phase number
            if !fm.stopped_at.is_empty() {
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
    }

    // Parse ROADMAP.md
    let roadmap_path = planning_dir.join("ROADMAP.md");
    if let Ok(content) = std::fs::read_to_string(&roadmap_path) {
        state.phases = roadmap_md::parse_roadmap_phases(&content);
    }

    // Run disk inference for each phase
    let mut current_phase_inference: Option<disk_status::DiskInference> = None;
    for phase in &state.phases {
        let inference = disk_status::infer_phase_status(planning_dir, &phase.number);
        // Track first non-complete phase as the current phase status
        if current_phase_inference.is_none()
            && inference.status != disk_status::DiskStatus::Complete
        {
            current_phase_inference = Some(inference.clone());
        }
        state
            .phase_disk_statuses
            .insert(phase.number.clone(), inference);
    }
    // If all phases are complete, use the last phase's status
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

    state
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
