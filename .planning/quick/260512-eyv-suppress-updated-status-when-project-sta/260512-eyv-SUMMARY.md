---
quick_id: 260512-eyv
description: Suppress "Updated" status when project state is unchanged
status: complete
date: 2026-05-12
commits:
  - d97da1d chore(quick-260512-eyv): derive PartialEq on ProjectState and sub-types
  - 4dcd271 fix(quick-260512-eyv): suppress "Updated" status on no-op state changes
---

# Quick Task 260512-eyv — Summary

## What Changed

The TUI no longer flashes `Updated: <project>` when a watcher-triggered re-parse produces a state identical to the cached one. Notifications now fire only on actual visible changes (status transitions, phase/plan progress, queue, backlog, pause state, disk inferences, etc.).

## Files Modified

- **`src/state_reader/mod.rs`** — `ProjectState`: added `PartialEq`.
- **`src/state_reader/roadmap_md.rs`** — `RoadmapPhase`: added `PartialEq`.
- **`src/state_reader/queue_md.rs`** — `QueuedAction`: added `PartialEq`.
- **`src/state_reader/disk_status.rs`** — `DiskInference`: added `PartialEq`.
- **`src/app.rs`** — `Action::ProjectStateLoaded` handler now compares `old_state != state` before setting `status_message`, calling `recompute_filtered_aliases`, and requesting a redraw. First-time loads still flash once (no prior state to compare). `ChangeTracker::detect_changes` is still called unconditionally so its event log keeps its current scope.

## Verification

- `cargo build` — clean
- `cargo test` — **111 passed**
- `cargo clippy -- -D warnings` — no issues

## Out of Scope

- The file watcher itself (still useful for waking caches and the change-tracker).
- Reworking `ChangeTracker` semantics — orthogonal concern.
