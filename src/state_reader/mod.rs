pub mod state_md;
pub mod roadmap_md;
#[allow(dead_code)]
pub mod config_json;
pub mod queue_md;
pub mod disk_status;

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
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
            } else if fm.progress.completed_phases >= fm.progress.total_phases && fm.progress.total_phases > 0 {
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
