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
}
