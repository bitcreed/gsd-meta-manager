---
phase: 01-core-infrastructure
plan: 03
subsystem: ui
tags: [ratatui, crossterm, tokio, tui, tea-pattern, event-bus]

# Dependency graph
requires:
  - phase: 01-core-infrastructure (plan 01)
    provides: Registry CRUD (add/remove/list projects), Config persistence, CLI subcommands
  - phase: 01-core-infrastructure (plan 02)
    provides: StateReader parsing STATE.md and ROADMAP.md into ProjectState struct
provides:
  - Interactive TUI dashboard launched via `gsd-manager` (no subcommand)
  - Terminal lifecycle management (init/restore with panic safety)
  - Async event bus with crossterm key reader and tick timer
  - App state machine with TEA update pattern and input modes
  - Project table rendering with Alias, Phase, Status, Path columns
  - Add/remove project flows via keyboard prompts
  - Empty state messaging and adaptive column layout
affects: [02-file-watching, 03-state-display, 04-roadmap-viz]

# Tech tracking
tech-stack:
  added: [ratatui 0.30, crossterm 0.29 event-stream, tokio mpsc, futures-util]
  patterns: [TEA (The Elm Architecture) with Action enum and App::update, async EventBus with mpsc channels, stateless UI render functions]

key-files:
  created:
    - src/tui.rs
    - src/event.rs
    - src/app.rs
    - src/ui/mod.rs
    - src/ui/project_list.rs
  modified:
    - src/main.rs
    - src/action.rs
    - Cargo.toml

key-decisions:
  - "Used RawKey(KeyEvent) action variant so event reader is stateless; App::update handles mode-specific key interpretation"
  - "Render takes &mut App to allow TableState mutation during draw without interior mutability"
  - "250ms tick interval for status message expiry checks"
  - "Synchronous project state loading at startup (acceptable for Phase 1 scope)"

patterns-established:
  - "TEA loop: EventBus -> mpsc -> App::update(Action) -> needs_redraw -> terminal.draw(ui::render)"
  - "InputMode enum drives both key handling in update() and footer rendering in UI"
  - "Status messages with timed auto-clear via Tick action"

requirements-completed: [REG-01, REG-02, STATE-01, STATE-02, STATE-03]

# Metrics
duration: 12min
completed: 2026-03-25
---

# Phase 01 Plan 03: TUI Event Loop and Interactive Dashboard Summary

**Async TUI dashboard with TEA state machine, crossterm event bus, project table with state reader integration, and keyboard-driven add/remove flows**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-25T08:04:00Z
- **Completed:** 2026-03-25T08:16:35Z
- **Tasks:** 3 (2 auto + 1 human-verify checkpoint)
- **Files modified:** 9

## Accomplishments
- Wired async event loop with tokio mpsc channels connecting crossterm key events to App state machine
- Implemented full TEA pattern: Action enum with RawKey variant, App::update dispatching by InputMode, stateless render functions
- Built project table with 4 columns (Alias, Phase, Status, Path), empty state messaging, adaptive width handling, and highlighted selection
- Delivered complete add/remove project flows via keyboard prompts with error handling and status messages
- Terminal lifecycle uses ratatui::init/restore with automatic panic hook for clean terminal restoration

## Task Commits

Each task was committed atomically:

1. **Task 1: Terminal lifecycle, event bus, and App state machine** - `1e8dbdc` (feat)
2. **Task 2: Stub TUI rendering -- project table, footer, input prompts, empty state** - `e670bcb` (feat)
3. **Task 3: Verify full TUI flow** - checkpoint:human-verify, approved by user

## Files Created/Modified
- `src/tui.rs` - Terminal init/restore lifecycle wrapper using ratatui::init
- `src/event.rs` - Async EventBus with crossterm EventStream reader and tick timer via tokio mpsc
- `src/app.rs` - App struct with InputMode enum and TEA update method handling all keyboard interactions
- `src/action.rs` - Action enum with RawKey(KeyEvent) variant for stateless event forwarding
- `src/ui/mod.rs` - Root render dispatch module
- `src/ui/project_list.rs` - Project table widget with all 4 columns, empty state, footer, input prompts, delete confirmation
- `src/main.rs` - Wired TUI mode (no subcommand) with event loop, terminal lifecycle, and graceful shutdown
- `Cargo.toml` - Added futures dependency for crossterm EventStream
- `Cargo.lock` - Updated lockfile

## Decisions Made
- Used `RawKey(KeyEvent)` action variant instead of individual key actions -- keeps event reader stateless while App::update handles mode-specific interpretation
- Render function takes `&mut App` to allow ratatui TableState mutation during draw without needing interior mutability (RefCell)
- 250ms tick interval balances responsiveness for status message expiry with low CPU overhead
- Synchronous project state loading at startup is acceptable for Phase 1 (async loading deferred to Phase 2+ when file watching is added)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 1 complete: CLI subcommands, state reader, and interactive TUI all functional
- Ready for Phase 2 (file watching): EventBus architecture supports adding notify filesystem events as another channel source
- Ready for Phase 3 (state display enhancements): ProjectState struct populated and rendered in table
- Ready for Phase 4 (roadmap visualization): App state machine supports adding new views/modes

## Self-Check: PASSED

- All 8 key files verified present on disk
- Both task commits (1e8dbdc, e670bcb) verified in git log

---
*Phase: 01-core-infrastructure*
*Completed: 2026-03-25*
