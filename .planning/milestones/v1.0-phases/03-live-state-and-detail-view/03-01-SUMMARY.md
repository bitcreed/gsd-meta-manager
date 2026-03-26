---
phase: 03-live-state-and-detail-view
plan: 01
subsystem: infra
tags: [notify, file-watching, debouncer, inotify, live-refresh]

requires:
  - phase: 01-core-infrastructure
    provides: EventBus (tokio mpsc), Action enum, App struct with TEA update loop
provides:
  - FileWatcher module wrapping notify-debouncer-full with 200ms debounce
  - Action::FileChanged variant flowing through EventBus
  - Targeted per-project re-parse on filesystem change
  - 500ms dedup window preventing event storms
affects: [03-02, detail-view, change-tracking]

tech-stack:
  added: [notify 8.x, notify-debouncer-full 0.5]
  patterns: [callback-to-tokio-mpsc bridge, per-project dedup refresh]

key-files:
  created: [src/watcher.rs]
  modified: [Cargo.toml, src/action.rs, src/lib.rs, src/app.rs, src/main.rs]

key-decisions:
  - "Used notify-debouncer-full 0.5.0 (not mini) for file ID cache and full event metadata"
  - "Bridge notify std callback to tokio mpsc via UnboundedSender::send() (non-async, thread-safe)"
  - "500ms dedup window in App prevents re-parse storms from GSD batch writes"
  - "STATE-05 documented as research conclusion: file watching covers hook use case"

patterns-established:
  - "Callback-to-channel bridge: notify callback calls tx.send() directly (UnboundedSender is non-async)"
  - "Per-project dedup: last_refresh HashMap with Instant comparison"
  - "extract_project_root: walk up path to find .planning/ parent"

requirements-completed: [STATE-04, STATE-05]

duration: 3min
completed: 2026-03-25
---

# Phase 3 Plan 1: Filesystem Watching Summary

**Live auto-refresh via notify-debouncer-full 0.5 watching .planning/ directories with 200ms debounce and 500ms dedup**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-25T22:26:35Z
- **Completed:** 2026-03-25T22:30:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- FileWatcher module with notify 8.x and 200ms debounce watching .planning/ directories recursively
- Action::FileChanged events flow through EventBus to App::update() for targeted per-project re-parse
- 500ms dedup window prevents re-parse storms from rapid GSD batch writes
- STATUS-05 documented: file watching covers hook-based push use case without added complexity

## Task Commits

Each task was committed atomically:

1. **Task 1: Add notify dependencies and create FileWatcher module** - `25e59a7` (feat)
2. **Task 2: Wire FileWatcher into App and main, with dedup and STATE-05 doc** - `c251c5c` (feat)

## Files Created/Modified
- `Cargo.toml` - Added notify 8.x and notify-debouncer-full 0.5 dependencies
- `src/watcher.rs` - FileWatcher struct with new/watch/unwatch, extract_project_root helper, STATE-05 doc
- `src/action.rs` - Added FileChanged { project_path } variant
- `src/lib.rs` - Added pub mod watcher
- `src/app.rs` - Added last_refresh dedup HashMap, FileChanged handler with targeted re-parse
- `src/main.rs` - Initialize FileWatcher for all registered projects on TUI startup

## Decisions Made
- Used notify-debouncer-full 0.5.0 (not mini) for file ID cache and full event metadata
- Bridge notify std callback to tokio mpsc via UnboundedSender::send() which is non-async and safe from the notify callback thread
- 500ms dedup window in App::update() prevents re-parse storms from rapid GSD batch writes
- STATE-05 documented as research conclusion: file watching covers the hook push use case

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed deprecated .watcher() API on Debouncer**
- **Found during:** Task 1
- **Issue:** Plan specified calling `self.debouncer.watcher().watch()` but notify-debouncer-full 0.5 deprecated `.watcher()` -- Debouncer now has watch/unwatch methods directly
- **Fix:** Called `self.debouncer.watch()` and `self.debouncer.unwatch()` directly
- **Files modified:** src/watcher.rs
- **Verification:** cargo check passes
- **Committed in:** 25e59a7

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** API correction needed due to notify-debouncer-full 0.5 deprecating the `.watcher()` accessor. No scope creep.

## Issues Encountered
None beyond the deprecated API noted above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- FileWatcher foundation ready for change tracking (DASH-05) in future plans
- EventBus pattern proven for additional event sources
- Detail view (03-02) can build on the live refresh infrastructure

---
*Phase: 03-live-state-and-detail-view*
*Completed: 2026-03-25*
