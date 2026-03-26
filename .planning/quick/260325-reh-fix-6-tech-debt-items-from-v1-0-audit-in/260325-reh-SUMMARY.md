---
phase: quick
plan: 260325-reh
subsystem: core
tags: [tracing, file-watching, dead-code, tech-debt]

requires:
  - phase: 04-visualization-creation-enqueue
    provides: watcher lifecycle, Action enum, ProjectState struct

provides:
  - tracing subscriber initialized with file appender
  - clean Action enum without dead variants
  - watcher lifecycle management on project add/remove
  - stale state cleanup on project removal

affects: []

tech-stack:
  added: []
  patterns:
    - "tracing_appender::rolling::daily for log file output"
    - "Deferred polling for .planning/ directory on new project creation"
    - "Auto-start watcher on FileChanged for late-appearing .planning/"

key-files:
  created: []
  modified:
    - src/main.rs
    - src/action.rs
    - src/app.rs
    - src/state_reader/mod.rs
    - tests/state_reader_test.rs

key-decisions:
  - "Used blocking daily rolling appender (no guard needed) for tracing output"
  - "Kept config_json module with #[allow(dead_code)] for future use after removing its only consumer"
  - "Used 2-second polling interval (30 attempts = 60s) for deferred .planning/ watch"

patterns-established: []

requirements-completed: []

duration: 4min
completed: 2026-03-26
---

# Quick Task 260325-reh: Fix 6 Tech Debt Items Summary

**Tracing subscriber with daily file appender, dead Action variant removal, watcher lifecycle fixes for inotify leak prevention and deferred new-project watching**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-26T02:47:10Z
- **Completed:** 2026-03-26T02:51:22Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Initialized tracing_subscriber with daily rolling file appender to ~/.local/share/gsd-manager/gsd-manager.log
- Removed 4 dead Action variants (Quit, AddProjectConfirm, RemoveProjectConfirm, ProjectLoaded) and their match arms
- Removed unused gsd_mode field from ProjectState and its config.json parsing block
- Fixed watcher lifecycle: unwatch on project removal, deferred watch on new project creation
- Added cleanup of detail_sub_view_per_project and last_refresh on project removal

## Task Commits

Each task was committed atomically:

1. **Task 1: Init tracing subscriber, remove dead Action variants, remove unused gsd_mode field** - `a1b6dbd` (fix)
2. **Task 2: Fix watcher lifecycle -- unwatch on removal, deferred watch for new projects, clean detail_sub_view** - `7bd6bb9` (fix)

## Files Created/Modified

- `src/main.rs` - Added tracing_subscriber init with daily rolling file appender
- `src/action.rs` - Removed 4 dead Action variants (Quit, AddProjectConfirm, RemoveProjectConfirm, ProjectLoaded)
- `src/app.rs` - Removed dead match arms; added unwatch + state cleanup on removal; added deferred watch polling + auto-start on FileChanged
- `src/state_reader/mod.rs` - Removed gsd_mode field and config.json parsing block; added #[allow(dead_code)] on config_json module
- `tests/state_reader_test.rs` - Removed gsd_mode assertions from integration tests

## Decisions Made

- Used blocking daily rolling appender (no guard needed) instead of non-blocking, matching CLAUDE.md stack guidance
- Kept config_json module intact with `#[allow(dead_code)]` since plan explicitly preserves it for future use
- Used 2-second polling interval with 30 attempts (60s total) for deferred .planning/ directory watching on new projects

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed test compilation after gsd_mode removal**
- **Found during:** Task 1
- **Issue:** Integration tests in tests/state_reader_test.rs referenced state.gsd_mode which no longer exists
- **Fix:** Removed gsd_mode assertions from test_parse_project_state_full and test_parse_project_state_missing_files
- **Files modified:** tests/state_reader_test.rs
- **Verification:** cargo test passes (all 108 tests)
- **Committed in:** a1b6dbd (Task 1 commit)

**2. [Rule 1 - Bug] Suppressed dead_code warning for config_json module**
- **Found during:** Task 1
- **Issue:** Removing the only consumer of config_json::parse_gsd_config caused dead_code warnings, but plan requires keeping the module
- **Fix:** Added #[allow(dead_code)] on the config_json module declaration
- **Files modified:** src/state_reader/mod.rs
- **Verification:** cargo build produces no warnings related to config_json
- **Committed in:** a1b6dbd (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes necessary for clean compilation after planned changes. No scope creep.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All 6 v1.0 audit tech debt items resolved
- Codebase clean for further development

---
*Quick task: 260325-reh*
*Completed: 2026-03-26*
