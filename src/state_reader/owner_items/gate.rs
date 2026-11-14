//! R8, the milestone gate: which open phases of the current milestone close after which
//! owner work (Phase 35, D-12, OWN-08).
//!
//! The totals are item counts over the root scope's current-milestone ROADMAP phases
//! that are not complete, after Layer B (superseded, acknowledged, archived and
//! deferred work never reach them). Debug sessions and handoffs are root signals, not
//! phase work, so they never count. A phase without a plan on disk is not built: it is
//! listed in `not_built` and adds nothing to the totals.

use std::collections::BTreeSet;

use super::{ClosesAfter, Kind, Lane, LanePhases, OwnerGroup, Scope};
use crate::state_reader::phase_num::{phase_key, same_phase, PhaseNum};
use crate::state_reader::ProjectState;

/// Phase order: numeric phases by number, anything unparseable last by text.
fn order_key(phase: &str) -> (bool, Option<PhaseNum>, String) {
    let parsed = PhaseNum::parse(phase);
    (parsed.is_none(), parsed, phase.to_owned())
}

fn sorted(mut phases: Vec<String>) -> Vec<String> {
    phases.sort_by_key(|phase| order_key(phase));
    phases
}

/// The gate of the root scope (workstream scopes are not exposed, [INFERRED] D-12).
pub(super) fn closes_after(groups: &[OwnerGroup], state: &ProjectState) -> ClosesAfter {
    let mut gate = ClosesAfter::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut lanes = LanePhases::default();
    for phase in state
        .phases
        .iter()
        .filter(|phase| !phase.completed && state.in_current_milestone(&phase.number))
    {
        if !seen.insert(phase_key(&phase.number)) {
            continue;
        }
        let built = state
            .disk_status_for(&phase.number)
            .is_some_and(|disk| disk.plan_count > 0);
        if !built {
            gate.not_built.push(phase.number.clone());
            continue;
        }
        gate.phases_open.push(phase.number.clone());
        let mut counts = [0usize; 4];
        for group in groups.iter().filter(|group| {
            matches!(group.scope, Scope::Current)
                && !group.acknowledged
                && group.workstream.is_none()
                && !matches!(group.kind, Kind::Debug | Kind::Handoff)
                && same_phase(&group.phase, &phase.number)
        }) {
            // A checkpoint is one decision whatever it lists, as in the digest total.
            let items = if group.kind == Kind::Checkpoint {
                1
            } else {
                group.items.len()
            };
            match group.lane {
                Lane::Test => counts[0] += items,
                Lane::Decide => counts[1] += items,
                Lane::Fix => counts[2] += items + group.counts.unreadable,
                Lane::Blocked => counts[3] += items,
                Lane::Deferred => {}
            }
        }
        let [test, decide, fix, blocked] = counts;
        gate.test += test;
        gate.decide += decide;
        gate.fix += fix;
        gate.blocked += blocked;
        for (count, list) in [
            (test, &mut lanes.test),
            (decide, &mut lanes.decide),
            (fix, &mut lanes.fix),
            (blocked, &mut lanes.blocked),
        ] {
            if count > 0 {
                list.push(phase.number.clone());
            }
        }
    }
    gate.phases_open = sorted(gate.phases_open);
    gate.not_built = sorted(gate.not_built);
    gate.lane_phases = LanePhases {
        test: sorted(lanes.test),
        decide: sorted(lanes.decide),
        fix: sorted(lanes.fix),
        blocked: sorted(lanes.blocked),
    };
    gate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::disk_status::DiskInference;
    use crate::state_reader::owner_items::{Category, GroupCounts, OwnerItem};
    use crate::state_reader::roadmap_md::RoadmapPhase;

    fn phase(number: &str, completed: bool) -> RoadmapPhase {
        RoadmapPhase {
            number: number.to_owned(),
            name: "P".to_owned(),
            description: String::new(),
            completed,
            total_plans: 0,
            completed_plans: 0,
            depends_on: Vec::new(),
        }
    }

    fn built(plans: u32) -> DiskInference {
        DiskInference {
            plan_count: plans,
            ..Default::default()
        }
    }

    fn state(phases: &[(&str, bool, u32)]) -> ProjectState {
        let mut state = ProjectState::default();
        for (number, completed, plans) in phases {
            state.phases.push(phase(number, *completed));
            state
                .phase_disk_statuses
                .insert((*number).to_owned(), built(*plans));
        }
        state
    }

    fn item() -> OwnerItem {
        OwnerItem {
            id: "x#1".to_owned(),
            test: None,
            name: "n".to_owned(),
            expected: None,
            result: "pending".to_owned(),
            category: Category::Pending,
            reason: None,
            blocked_by: None,
            gaps: Vec::new(),
            why_human: None,
        }
    }

    fn group(lane: Lane, kind: Kind, phase: &str, items: usize, unreadable: usize) -> OwnerGroup {
        OwnerGroup {
            lane,
            kind,
            scope: Scope::Current,
            workstream: None,
            phase: phase.to_owned(),
            phase_name: String::new(),
            file: "f".to_owned(),
            file_status: "x".to_owned(),
            closes_phase: false,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: None,
            stale: false,
            command: None,
            supersedes: Vec::new(),
            counts: GroupCounts {
                unreadable,
                ..Default::default()
            },
            items: (0..items).map(|_| item()).collect(),
        }
    }

    #[test]
    fn lane_totals_follow_the_counting_rule() {
        let state = state(&[("1", false, 1)]);
        let groups = vec![
            group(Lane::Test, Kind::Uat, "1", 3, 0),
            group(Lane::Blocked, Kind::Uat, "1", 2, 0),
            group(Lane::Fix, Kind::Verification, "1", 1, 1),
            group(Lane::Decide, Kind::ContextQuestions, "1", 2, 0),
            group(Lane::Decide, Kind::Checkpoint, "1", 0, 1),
        ];
        let gate = closes_after(&groups, &state);
        assert_eq!(
            (gate.test, gate.decide, gate.fix, gate.blocked),
            (3, 3, 2, 2)
        );
        assert_eq!(gate.phases_open, ["1"]);
        assert_eq!(gate.lane_phases.test, ["1"]);
        assert_eq!(gate.lane_phases.decide, ["1"]);
    }

    #[test]
    fn signals_that_are_not_phase_work_add_nothing() {
        let state = state(&[("1", false, 1)]);
        let mut acknowledged = group(Lane::Test, Kind::Uat, "1", 4, 0);
        acknowledged.acknowledged = true;
        let mut archived = group(Lane::Test, Kind::Uat, "1", 4, 0);
        archived.scope = Scope::Archived {
            label: "v1".to_owned(),
            rank: 0,
        };
        let mut stream = group(Lane::Test, Kind::Uat, "1", 4, 0);
        stream.workstream = Some("a".to_owned());
        let groups = vec![
            group(Lane::Fix, Kind::Debug, "", 1, 0),
            group(Lane::Decide, Kind::Handoff, "1", 2, 0),
            group(Lane::Deferred, Kind::Deferred, "1", 5, 0),
            acknowledged,
            archived,
            stream,
        ];
        let gate = closes_after(&groups, &state);
        assert_eq!(
            (gate.test, gate.decide, gate.fix, gate.blocked),
            (0, 0, 0, 0)
        );
        assert_eq!(gate.phases_open, ["1"]);
        assert!(gate.not_built.is_empty());
    }

    #[test]
    fn complete_and_unbuilt_phases_are_not_counted() {
        let state = state(&[("1", true, 1), ("2", false, 0), ("3", false, 1)]);
        let groups = vec![
            group(Lane::Test, Kind::Uat, "1", 9, 0),
            group(Lane::Test, Kind::Uat, "2", 9, 0),
            group(Lane::Test, Kind::Uat, "3", 1, 0),
        ];
        let gate = closes_after(&groups, &state);
        assert_eq!(gate.test, 1);
        assert_eq!(gate.phases_open, ["3"]);
        assert_eq!(gate.not_built, ["2"]);
    }

    #[test]
    fn a_phase_without_a_disk_entry_is_not_built() {
        let mut state = state(&[("1", false, 1)]);
        state.phases.push(phase("2", false));
        let gate = closes_after(&[], &state);
        assert_eq!(gate.not_built, ["2"]);
        assert_eq!(gate.phases_open, ["1"]);
    }

    #[test]
    fn a_padded_group_phase_counts_toward_the_roadmap_number() {
        let state = state(&[("7", false, 1)]);
        let groups = vec![group(Lane::Test, Kind::Uat, "07", 2, 0)];
        let gate = closes_after(&groups, &state);
        assert_eq!(gate.test, 2);
        assert_eq!(gate.phases_open, ["7"]);
    }

    #[test]
    fn phases_come_out_in_numeric_order_once_each() {
        let state = state(&[
            ("10", false, 1),
            ("2", false, 1),
            ("7.1", false, 1),
            ("7", false, 1),
            ("7", false, 1),
        ]);
        let groups = vec![
            group(Lane::Test, Kind::Uat, "10", 1, 0),
            group(Lane::Test, Kind::Uat, "2", 1, 0),
            group(Lane::Test, Kind::Uat, "7.1", 1, 0),
        ];
        let gate = closes_after(&groups, &state);
        assert_eq!(gate.phases_open, ["2", "7", "7.1", "10"]);
        assert_eq!(gate.lane_phases.test, ["2", "7.1", "10"]);
    }

    #[test]
    fn a_state_without_phases_is_all_zero() {
        let gate = closes_after(&[], &ProjectState::default());
        assert_eq!(gate, ClosesAfter::default());
    }
}
