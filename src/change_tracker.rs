use crate::state_reader::ProjectState;
use std::collections::HashMap;
use std::time::Instant;

/// A single change event detected for a project.
#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub description: String,
    pub timestamp: Instant,
}

/// In-memory change tracker that detects status transitions and phase completions
/// since app launch. Per D-07: in-memory only, no persistence. Per D-08: only tracks
/// phase completions and status transitions.
pub struct ChangeTracker {
    changes: HashMap<String, Vec<ChangeEvent>>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }

    /// Record the initial state of a project at app startup.
    /// Currently a no-op; retained for API compatibility (change detection
    /// uses old/new comparison in `detect_changes` instead).
    pub fn record_initial(&mut self, _alias: &str, _state: &ProjectState) {
        // Intentionally empty -- initial snapshots were removed as dead code.
        // Change detection compares old vs new state directly.
    }

    /// Compare old and new states, generating change events for status transitions
    /// and phase completions (per D-08).
    pub fn detect_changes(&mut self, alias: &str, old: &ProjectState, new: &ProjectState) {
        let now = Instant::now();
        let events = self.changes.entry(alias.to_string()).or_default();

        // Status transition (per D-08)
        if old.status != new.status {
            events.push(ChangeEvent {
                description: format!("Status: {} -> {}", old.status, new.status),
                timestamp: now,
            });
        }

        // Phase completions (per D-08): check each phase in new that is completed but was not in old
        for new_phase in &new.phases {
            if new_phase.completed {
                let was_completed = old
                    .phases
                    .iter()
                    .find(|p| p.number == new_phase.number)
                    .map(|p| p.completed)
                    .unwrap_or(false);

                if !was_completed {
                    events.push(ChangeEvent {
                        description: format!("Phase {} completed", new_phase.name),
                        timestamp: now,
                    });
                }
            }
        }

        // Plan completions: if total completed_plans increased
        if new.completed_plans > old.completed_plans {
            let diff = new.completed_plans - old.completed_plans;
            events.push(ChangeEvent {
                description: format!("{} more plan{} completed", diff, if diff == 1 { "" } else { "s" }),
                timestamp: now,
            });
        }
    }

    /// Get the most recent change event for a project.
    pub fn latest_change(&self, alias: &str) -> Option<&ChangeEvent> {
        self.changes.get(alias).and_then(|events| events.last())
    }

    /// Format an Instant as a human-readable relative time string.
    pub fn format_elapsed(instant: Instant) -> String {
        let elapsed = instant.elapsed().as_secs();
        if elapsed < 60 {
            "just now".to_string()
        } else if elapsed < 3600 {
            format!("{}m ago", elapsed / 60)
        } else if elapsed < 86400 {
            format!("{}h ago", elapsed / 3600)
        } else {
            format!("{}d ago", elapsed / (86400))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    fn make_state(status: &str, completed_plans: u32, phases: Vec<(&str, &str, bool)>) -> ProjectState {
        ProjectState {
            status: status.to_string(),
            completed_plans,
            phases: phases
                .into_iter()
                .map(|(num, name, completed)| RoadmapPhase {
                    number: num.to_string(),
                    name: name.to_string(),
                    description: String::new(),
                    completed,
                    total_plans: 0,
                    completed_plans: 0,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_detect_status_transition() {
        let mut tracker = ChangeTracker::new();
        let old = make_state("Ready to plan", 0, vec![]);
        let new = make_state("Executing", 0, vec![]);
        tracker.detect_changes("proj", &old, &new);

        let latest = tracker.latest_change("proj").unwrap();
        assert!(latest.description.contains("Status: Ready to plan -> Executing"));
    }

    #[test]
    fn test_detect_phase_completion() {
        let mut tracker = ChangeTracker::new();
        let old = make_state("active", 0, vec![("1", "Core", false)]);
        let new = make_state("active", 0, vec![("1", "Core", true)]);
        tracker.detect_changes("proj", &old, &new);

        let latest = tracker.latest_change("proj").unwrap();
        assert!(latest.description.contains("Phase Core completed"));
    }

    #[test]
    fn test_detect_plan_completions() {
        let mut tracker = ChangeTracker::new();
        let old = make_state("active", 2, vec![]);
        let new = make_state("active", 5, vec![]);
        tracker.detect_changes("proj", &old, &new);

        let latest = tracker.latest_change("proj").unwrap();
        assert!(latest.description.contains("3 more plans completed"));
    }

    #[test]
    fn test_no_changes() {
        let mut tracker = ChangeTracker::new();
        let state = make_state("active", 2, vec![("1", "Core", true)]);
        tracker.detect_changes("proj", &state, &state.clone());

        assert!(tracker.latest_change("proj").is_none());
    }

    #[test]
    fn test_format_elapsed_just_now() {
        let instant = Instant::now();
        assert_eq!(ChangeTracker::format_elapsed(instant), "just now");
    }
}
