---
quick_id: 260722-emn
plan: 8
item: G
subsystem: state_reader
status: complete
tags: [workstreams, gsd-1.8.0, state-reader, data-layer]
requires: [260722-emn-6]
provides: [ProjectState.workstreams, load_workstreams]
affects: [src/state_reader/mod.rs, src/state_reader/workstreams.rs]
tech-stack:
  added: []
  patterns: [reuse-parse_project_state-for-workstream-dirs, parent-dir-recursion-guard]
key-files:
  created: [src/state_reader/workstreams.rs]
  modified: [src/state_reader/mod.rs]
decisions:
  - "Recursion guard keys on planning_dir's immediate parent being named `workstreams` — bounds workstream loading to one level without a depth counter."
  - "Reuse super::parse_project_state for each workstream dir (a workstream dir is exactly a planning-shaped dir) rather than a bespoke parser."
metrics:
  duration: ~6min
  completed: 2026-07-22
  tasks: 2
  files: 2
---

# Quick 260722-emn Plan 8: Workstreams Data Layer Summary

Added a GSD 1.8.0 workstreams data layer: `src/state_reader/workstreams.rs` detects
`.planning/workstreams/<ws>/` subdirectories, parses each as its own `ProjectState` via the
existing `parse_project_state` reader, resolves the active workstream from
`.planning/active-workstream`, and exposes the sorted list on `ProjectState.workstreams` with a
one-level recursion guard.

## What Was Built

### Task 1 — `workstreams.rs` module (commit 883798c)
- New `pub struct WorkstreamState { name, active, state }` (derives Debug, Clone, Default, PartialEq).
- `pub fn load_workstreams(planning_dir: &Path) -> Vec<WorkstreamState>`:
  - Reads `<planning_dir>/workstreams/`, parses each subdirectory with `super::parse_project_state`.
  - Reads `<planning_dir>/active-workstream` (trimmed) to set the `active` flag on the matching name.
  - Sorts results by name for stability; returns an empty `Vec` when `workstreams/` is missing or unreadable (never panics).
- `pub mod workstreams;` declared in `state_reader/mod.rs` (the only mod.rs change in Task 1).
- 4 inline tempfile tests: two workstreams both returned + sorted, active-flag resolution, no-pointer means none active, missing dir returns empty.

### Task 2 — expose on `ProjectState` with recursion guard (commit 30e1760)
- Added `pub workstreams: Vec<workstreams::WorkstreamState>` to `ProjectState`.
- Populated in `parse_project_state` via `load_workstreams`, guarded so a workstream sub-parse
  does not re-enter workstream loading: skipped when the planning dir's immediate parent directory
  is named `workstreams`. Recursion depth is bounded to one level.
- 2 inline tests: a two-workstream project yields `workstreams.len() == 2` with each sub-state's
  `workstreams` empty and the active flag flowing through; a flat project yields an empty vec.

## Verification

- `cargo test --lib workstreams`: 4 passed.
- `cargo build`: clean (10 crates, no warnings surfaced in tail).
- `cargo test` (full suite): 198 passed across 5 suites.

## Deviations from Plan

None — plan executed exactly as written.

## Scope Adherence

Only `src/state_reader/workstreams.rs` (new) and `src/state_reader/mod.rs` were modified, matching
`files_modified`. `queue_md.rs` (concurrently edited by the wave-3 plan-7 executor) was not touched.
No UI file edited (workstream surfacing is plan 9). ROADMAP.md and STATE.md were not modified per
constraints.

## Known Stubs

None.

## Self-Check: PASSED
- FOUND: src/state_reader/workstreams.rs
- FOUND: commit 883798c (Task 1)
- FOUND: commit 30e1760 (Task 2)
