---
phase: 10-tech-debt-cleanup
plan: 01
subsystem: infra
tags: [rust, clippy, dead-code, formatting, tech-debt]

# Dependency graph
requires: []
provides:
  - Zero-warning Rust build baseline for v1.2 feature work
  - Binary crate refactored to use lib crate (no duplicate module declarations)
affects: [12-archive-browser, 11-paused-detection]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Binary crate imports from lib crate via use gsd_meta_manager::* (no mod re-declarations)"
    - "Box<Action> for large enum variants to reduce ScreenAction enum size"
    - "Default impls delegate to new() for types with constructors"

key-files:
  created: []
  modified:
    - src/main.rs
    - src/event.rs
    - src/change_tracker.rs
    - src/state_reader/mod.rs
    - src/state_reader/state_md.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/backlog.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/create_project.rs
    - src/config.rs
    - src/session_detector.rs
    - src/app.rs

key-decisions:
  - "Refactored main.rs to use lib crate instead of re-declaring modules, eliminating false dead_code warnings"
  - "Removed ProjectSnapshot and initial_snapshots from ChangeTracker (genuinely dead code)"
  - "Removed BacklogItem.slug field (never consumed downstream)"
  - "Kept targeted #[allow(dead_code)] only on DispatchAction variant (reserved for Phase 12)"
  - "Boxed DispatchAction(Box<Action>) to fix large_enum_variant clippy warning"

patterns-established:
  - "Binary/lib separation: main.rs uses `use gsd_meta_manager::*` for shared modules, only declares binary-only modules (event, tui)"
  - "&Path over &PathBuf in function parameters for idiomatic Rust"

requirements-completed: [DEBT-01, DEBT-02]

# Metrics
duration: 17min
completed: 2026-03-31
---

# Phase 10 Plan 01: Tech Debt Cleanup Summary

**Zero-warning build with all dead_code suppressions resolved, clippy clean, and main.rs refactored to use lib crate**

## Performance

- **Duration:** 17 min
- **Started:** 2026-03-31T20:42:03Z
- **Completed:** 2026-03-31T20:59:22Z
- **Tasks:** 2
- **Files modified:** 28

## Accomplishments
- Eliminated all 8 `#[allow(dead_code)]` / `#[allow(unused)]` annotations (except 1 targeted on DispatchAction)
- Fixed 16+ clippy warnings across the codebase (more than the 4 originally identified)
- Applied cargo fmt across all 28 source files
- Refactored main.rs to import from lib crate, eliminating duplicate module compilation
- All 20 integration tests + 64 unit tests pass cleanly

## Task Commits

Each task was committed atomically:

1. **Task 1: Audit and resolve all dead code annotations** - `6922471` (fix)
2. **Task 2: Fix clippy warnings and formatting, verify clean baseline** - `4093a0a` (fix)

## Files Created/Modified
- `src/main.rs` - Refactored to use lib crate instead of re-declaring shared modules
- `src/event.rs` - Updated import from `crate::action` to `gsd_meta_manager::action`
- `src/change_tracker.rs` - Removed dead ProjectSnapshot struct and initial_snapshots; added Default impl
- `src/state_reader/mod.rs` - Removed `#[allow(dead_code)]` from config_json module
- `src/state_reader/state_md.rs` - Removed allow annotations; fixed manual char comparison
- `src/state_reader/roadmap_md.rs` - Removed allow annotation from RoadmapPhase
- `src/state_reader/backlog.rs` - Removed slug field from BacklogItem
- `src/ui/screens/mod.rs` - Removed blanket allow annotations; boxed DispatchAction; derived Default for ProjectViewCache
- `src/ui/screens/add_project.rs` - Changed &PathBuf to &Path
- `src/ui/screens/create_project.rs` - Changed &PathBuf to &Path
- `src/config.rs` - Added Default impl for Config
- `src/session_detector.rs` - Replaced redundant closure with function reference
- `src/app.rs` - Updated DispatchAction destructuring for Box<Action>

## Decisions Made
- Refactored main.rs to use lib crate instead of re-declaring modules -- this was the root cause of false dead_code warnings on pub items that are consumed by integration tests but appeared unused in the binary crate's separate compilation
- Removed ProjectSnapshot and initial_snapshots entirely from ChangeTracker -- genuinely unused, change detection uses old/new comparison in detect_changes() instead
- Removed BacklogItem.slug field -- confirmed via grep that no downstream code consumes it
- Kept DispatchAction with targeted #[allow(dead_code)] annotation documenting Phase 12 usage

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Refactored main.rs to use lib crate**
- **Found during:** Task 1
- **Issue:** main.rs re-declared all shared modules via `mod` instead of importing from the lib crate, causing duplicate compilation and false dead_code warnings on pub items used by integration tests
- **Fix:** Changed main.rs to `use gsd_meta_manager::*` for shared modules, keeping only binary-specific `mod event` and `mod tui`; updated event.rs import accordingly
- **Files modified:** src/main.rs, src/event.rs
- **Verification:** cargo build produces zero warnings
- **Committed in:** 6922471 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed 12 additional clippy warnings beyond the 4 planned**
- **Found during:** Task 2
- **Issue:** Plan identified 4 clippy warnings, but 16 total existed (new_without_default for NormalScreen, &PathBuf instead of &Path in 2 locations, large_enum_variant, map_or simplification in 3 places, sort_by_key, enumerate discard, clone on Copy type, derivable impl)
- **Fix:** Applied cargo clippy --fix for auto-fixable issues, manually fixed &PathBuf and large_enum_variant
- **Files modified:** Multiple screen files, config.rs, app.rs, mod.rs
- **Verification:** cargo clippy produces zero warnings
- **Committed in:** 4093a0a (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for achieving zero-warning baseline. The main.rs refactor was essential to resolve false dead_code warnings. No scope creep.

## Issues Encountered
None - all issues were resolved through the deviation auto-fixes documented above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - no stubs or placeholder data in this plan.

## Next Phase Readiness
- Clean zero-warning baseline established for all v1.2 feature work
- Binary/lib crate separation is now correct, preventing future false warnings
- DispatchAction variant preserved and documented for Phase 12 archive browser

---
*Phase: 10-tech-debt-cleanup*
*Completed: 2026-03-31*
