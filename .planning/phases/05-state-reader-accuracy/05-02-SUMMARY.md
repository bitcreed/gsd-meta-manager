---
phase: 05-state-reader-accuracy
plan: 02
subsystem: ui
tags: [ratatui, screen-architecture, async, spawn-blocking, refactor]

requires:
  - phase: 05-state-reader-accuracy
    provides: "disk-based phase inference and plan counting fixes (05-01)"
provides:
  - "Screen trait with handle_key/render/name for modular screen system"
  - "AppContext struct holding all shared TUI state"
  - "Screen stack (Vec<Box<dyn Screen>>) replacing InputMode enum"
  - "Async file I/O via spawn_blocking + ProjectStateLoaded action"
  - "7 Screen implementations: Normal, Detail, AddProject, DeleteConfirm, Help, Enqueue, CreateProject"
affects: [phase-06, phase-07, all-future-screen-additions]

tech-stack:
  added: []
  patterns:
    - "Screen trait dispatch via screen_stack.last_mut().handle_key()"
    - "Search as boolean flag on NormalScreen (overlay, not separate screen)"
    - "Help as overlay screen that renders Clear + popup over parent"
    - "Modal flows via screen pushes (e.g., AddProject: alias phase -> path phase)"
    - "spawn_blocking for FileChanged -> ProjectStateLoaded async pipeline"

key-files:
  created:
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/help.rs
    - src/ui/screens/enqueue.rs
    - src/ui/screens/create_project.rs
  modified:
    - src/app.rs
    - src/action.rs
    - src/ui/mod.rs
    - src/main.rs

key-decisions:
  - "Search mode implemented as boolean flag on NormalScreen rather than separate screen"
  - "Help screen implemented as overlay (Clear + popup) that pops on any key"
  - "AppContext holds needs_redraw separately from App; synced in main loop"
  - "Old UI files (detail_view.rs, project_list.rs, help_overlay.rs) deleted entirely"

patterns-established:
  - "Screen trait: all new screens implement Screen with handle_key/render/name"
  - "ScreenAction enum: Push/Pop/Quit/SetStatusMessage/DispatchAction for screen transitions"
  - "AppContext: shared mutable state passed to all screen handle_key and render methods"
  - "Modal flows: multi-phase screens use internal enum (e.g., AddPhase::Alias, AddPhase::Path)"

requirements-completed: []

duration: 10min
completed: 2026-03-26
---

# Phase 05 Plan 02: Screen Architecture Refactor Summary

**Replaced 11-variant InputMode enum with Screen trait + screen stack, migrated file I/O to async spawn_blocking**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-26T21:49:17Z
- **Completed:** 2026-03-26T21:59:58Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments
- Defined Screen trait, ScreenAction enum, and AppContext struct for modular screen architecture
- Migrated all 11 InputMode variants to 7 Screen implementations (some merged: search into NormalScreen)
- Added ProjectStateLoaded action variant and async FileChanged handler via spawn_blocking
- Deleted 658 lines of old UI code (detail_view.rs, project_list.rs, help_overlay.rs)
- All 38 existing tests pass unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Define Screen trait, AppContext, and migrate all screens** - `c297631` (feat)
2. **Task 2: Delete old InputMode-based UI files and clean up** - `d1a7c64` (refactor)

## Files Created/Modified
- `src/ui/screens/mod.rs` - Screen trait, ScreenAction enum, AppContext struct
- `src/ui/screens/normal.rs` - NormalScreen with search-as-flag overlay
- `src/ui/screens/detail.rs` - DetailScreen with scroll_offset and roadmap toggle
- `src/ui/screens/add_project.rs` - Two-phase alias/path modal
- `src/ui/screens/delete_confirm.rs` - Y/N confirmation screen
- `src/ui/screens/help.rs` - Overlay help popup
- `src/ui/screens/enqueue.rs` - Queue input with Tab suggestions
- `src/ui/screens/create_project.rs` - Three-phase name/path/confirm modal
- `src/app.rs` - Replaced InputMode with screen_stack + AppContext
- `src/action.rs` - Added ProjectStateLoaded variant
- `src/ui/mod.rs` - Simplified to delegate to screen_stack.last()
- `src/main.rs` - Updated to use app.ctx for event_tx and watcher

## Decisions Made
- Search mode kept as boolean flag on NormalScreen (not a separate screen) since it's an overlay filter on the same view
- Help implemented as overlay screen that renders Clear widget + centered popup, pops on ? or Esc
- AppContext.needs_redraw synced to App.needs_redraw in main loop to bridge screen-level and app-level redraw signals
- Old UI files deleted entirely rather than deprecated, since all functionality moved to screens/

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Screen architecture is ready for Phase 06+ screen additions without enum explosion
- Async file I/O via spawn_blocking eliminates render loop blocking
- New screens can be added by implementing the Screen trait and pushing onto screen_stack

---
*Phase: 05-state-reader-accuracy*
*Completed: 2026-03-26*
