---
phase: 06-read-only-views
plan: 01
subsystem: ui
tags: [ratatui, tabs, backlog, git-log, detail-view]

requires:
  - phase: 05-state-reader-accuracy
    provides: DetailSubView enum, Screen trait architecture, AppContext, disk inference
provides:
  - 4-tab detail view system (Phases, Roadmap, Backlog, Git)
  - BacklogItem and GitLogEntry data models in state_reader
  - ProjectViewCache for per-project view state
  - Action variants for async backlog/git data loading
  - Tab navigation via number keys 1-4 and left/right arrows
affects: [06-02-backlog-browser, 06-03-git-history-viewer]

tech-stack:
  added: [tokio::process::Command for git subprocess]
  patterns: [tab-based detail view, per-project view cache, async git data loading]

key-files:
  created:
    - src/state_reader/backlog.rs
    - src/state_reader/git_ops.rs
  modified:
    - src/state_reader/mod.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs

key-decisions:
  - "Backlog items loaded synchronously (fast filesystem reads) vs async for git log"
  - "Tab bar uses ratatui Tabs widget with Borders::BOTTOM separator"
  - "Removed legacy 'r' key toggle in favor of number-key tab switching"

patterns-established:
  - "Tab system: tab_index/sub_view_from_index helpers for DetailSubView mapping"
  - "Per-tab render methods: render_phase_list, render_roadmap, render_backlog_placeholder, render_git_placeholder"
  - "switch_to_tab helper handles scroll reset, data loading triggers, and view state"

requirements-completed: [BLOG-01, GIT-01]

duration: 5min
completed: 2026-03-27
---

# Phase 06 Plan 01: Tab System Foundation Summary

**4-tab detail view with BacklogItem/GitLogEntry data models, ProjectViewCache, and async loading infrastructure**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-27T03:07:24Z
- **Completed:** 2026-03-27T03:12:47Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Extended DetailSubView enum with Backlog and GitHistory variants
- Created BacklogItem struct with directory parsing and content loading functions
- Created GitLogEntry/GitDiffStat structs with async git subprocess functions
- Added 4 new Action variants for async data flow (BacklogLoaded, BacklogContentLoaded, GitLogLoaded, GitDiffStatLoaded)
- Implemented ProjectViewCache struct with per-project state on AppContext
- Refactored detail.rs from monolithic if/else to tab-based architecture with ratatui Tabs widget
- Tab navigation via 1-4 keys and left/right arrows, with tab-specific footer hints

## Task Commits

Each task was committed atomically:

1. **Task 1: Add data models, Action variants, and ProjectViewCache** - `6b25c01` (feat)
2. **Task 2: Implement tab bar rendering and tab navigation in DetailScreen** - `b2c29c9` (feat)

## Files Created/Modified
- `src/state_reader/backlog.rs` - BacklogItem struct, parse_backlog_items, load_backlog_content, parse_backlog_dir_name
- `src/state_reader/git_ops.rs` - GitLogEntry, GitDiffStat, load_git_log, load_diff_stat (async)
- `src/state_reader/mod.rs` - Module declarations for backlog and git_ops
- `src/action.rs` - Four new Action variants for async data loading
- `src/app.rs` - Extended DetailSubView, added view_cache to AppContext init, handlers for new Actions
- `src/ui/screens/mod.rs` - ProjectViewCache struct, view_cache field on AppContext
- `src/ui/screens/detail.rs` - Complete refactor: tab bar, per-tab rendering, tab navigation, data loading triggers

## Decisions Made
- Backlog items loaded synchronously on tab switch (filesystem reads are fast, matches existing state loading pattern)
- Git log loaded asynchronously via tokio::spawn + event_tx (subprocess calls need async)
- Removed 'r' key toggle between PhaseList/RoadmapViz in favor of unified 1-4 tab system
- Backlog dir names containing `{` or newlines are skipped (handles malformed JSON slug directories)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed non-exhaustive match on DetailSubView**
- **Found during:** Task 1
- **Issue:** Existing `'r'` key handler had match on DetailSubView with only PhaseList/RoadmapViz arms; adding Backlog/GitHistory variants caused compile error
- **Fix:** Added Backlog and GitHistory arms to the match (both map to PhaseList); later removed entirely in Task 2
- **Files modified:** src/ui/screens/detail.rs
- **Verification:** cargo check passes
- **Committed in:** 6b25c01 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary for compilation. No scope creep.

## Issues Encountered
- Pre-existing test failure in `end_to_end_add_then_list_via_cli` (registry_test) - unrelated to this plan's changes, confirmed by testing on pre-change code

## Known Stubs

- `render_backlog_placeholder` in detail.rs shows basic item list with count; full interactive browser in Plan 02
- `render_git_placeholder` in detail.rs shows basic entry list with count; full interactive viewer in Plan 03
- Backlog `backlog_selected` and `git_selected` fields exist but j/k navigation within tabs not yet wired (Plans 02/03)

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Tab infrastructure ready for Plan 02 (backlog browser) and Plan 03 (git history viewer)
- ProjectViewCache provides all state fields needed by both plans
- Action variants and App::update handlers ready to receive async data
- Both plans can work in parallel on their respective tabs

## Self-Check: PASSED

All 7 source files verified present. Both task commits (6b25c01, b2c29c9) verified in git log.

---
*Phase: 06-read-only-views*
*Completed: 2026-03-27*
