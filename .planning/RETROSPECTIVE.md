# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-26
**Phases:** 4 | **Plans:** 10 | **Commits:** 83

### What Was Built
- Rust TUI dashboard with color-coded project list, aggregate status bar, and vim-style navigation
- Live filesystem watching with 200ms debounce for auto-refresh on .planning/ changes
- Project detail view with phase breakdown, plan counts, status icons, and change tracking
- ASCII roadmap visualization with box-drawing pipeline widget
- Project creation flow with git init, modal input, hooks, and auto-registration
- Work enqueue system with QUEUE.md, tab-cyclable command suggestions

### What Worked
- Rust + ratatui stack delivered fast, portable single binary with no runtime dependencies
- TEA (The Elm Architecture) pattern kept TUI state predictable and testable
- Phase-by-phase GSD workflow matched well to incremental TUI feature building
- File watching via notify-debouncer-full was straightforward to integrate with tokio

### What Was Inefficient
- ROADMAP.md parser relies on markdown structure rather than disk truth — led to plan counting bugs
- STATE.md `completed_phases` counter creates stale state vs actual files on disk
- QUEUE.md is passive (GSD doesn't consume it) — users expect queued commands to be executable
- Some backlog directory names got malformed JSON slugs (999.1, 999.2)

### Patterns Established
- Atomic file writes via tempfile + rename for all state mutations (config, queue)
- `spawn_blocking` for IO-bound operations (git init, file discovery) to keep render loop responsive
- Channel-based architecture: crossterm events, notify events, and tick timer multiplexed via `tokio::select!`

### Key Lessons
1. Parse state from disk files, not markdown documents — markdown structure is fragile and accumulates parsing bugs
2. Phase completion should be inferred from PLAN/SUMMARY pairs, not a counter in STATE.md
3. Enqueue features need a clear execution path — passive reminders frustrate users who expect action

### Cost Observations
- Model profile: quality (opus throughout)
- 2-day build from greenfield to 22-requirement MVP
- Notable: research phase for tech stack paid off — zero mid-build stack pivots

---

## Milestone: v1.1 — Polish & Power Features

**Shipped:** 2026-03-27
**Phases:** 5 | **Plans:** 16

### What Was Built
- State reader accuracy with disk-based phase inference
- GSD integration badges (verified/inferred)
- Queue management CRUD, git history viewer, backlog browser
- Execution flow pipeline (D-R-P-E-V visualization)
- Claude session management

### What Worked
- Screen trait refactor (Phase 05) enabled adding 4 new tabs without explosion
- Disk-status inference proved more reliable than parsing STATE.md counters

### What Was Inefficient
- Some phases could have been parallelized (Queue + Git History were independent)

### Key Lessons
1. Refactor before feature expansion — Screen trait made 7-tab view possible
2. Disk truth beats document parsing for status detection

---

## Milestone: v1.2 — Housekeeping & Archive Browser

**Shipped:** 2026-04-01
**Phases:** 4 | **Plans:** 6

### What Was Built
- Zero-warning baseline (all dead_code resolved, clippy clean)
- Paused project detection (HANDOFF file → cyan badge on dashboard)
- Milestone Archive Browser (8th tab: 4-level drill-down, styled markdown, async loading)
- Queue execution design document (2 strategies, safety requirements, LLM-agnostic)

### What Worked
- Autonomous mode (`/gsd:autonomous`) ran 4 phases with minimal user intervention (only grey area approvals and one tech debt decision)
- Smart discuss with batch table proposals was fast — user accepted all recommended answers
- Phase 10 cleanup first prevented Phase 12 from needing to work around dead code annotations
- Research phase (13) produced actionable design document that can drive v1.3 planning directly

### What Was Inefficient
- Worktree merges occasionally caused binary/lib duplication conflicts (Phase 12 Wave 1)
- UI-SPEC checker's pixel-based spacing rules don't map well to terminal cell units — required revision loop for a false positive
- Phase 12 Plan 03 (visual UAT) was deferred — still needs manual testing

### Patterns Established
- `archive_file_name` cached for breadcrumb accuracy — can't rely on index lookups for mixed-level navigation
- Tab bar abbreviation pattern (Pipeline → Pipe, Sessions → Sess) for 80-column fit
- LLM-agnostic Executor trait pattern for future queue execution

### Key Lessons
1. Clean baseline before feature work pays off immediately (Phase 10 → Phase 12 dependency)
2. Documentation-only phases (research) work well in autonomous mode — no code conflicts
3. Worktree agents need explicit instructions about binary/lib crate structure to avoid duplicate module declarations
4. UI-SPEC checker rules need terminal-aware exceptions — pixel grid rules produce false positives for TUI apps

### Cost Observations
- Model profile: quality (opus for execution, sonnet for verification)
- 4 phases completed in a single autonomous session
- Notable: Phase 13 research used the GSD source directly — much more accurate than web-only research

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 83 | 4 | Initial build — established TEA pattern, file watching, state reader |
| v1.1 | 79 | 5 | Screen trait refactor, 7-tab detail view, disk-based inference |
| v1.2 | 41 | 4 | Autonomous mode, archive browser, paused detection, queue execution design |

### Cumulative Quality

| Milestone | LOC | Tests | Key Metric |
|-----------|-----|-------|------------|
| v1.0 | 3,887 | 12 integration | Full requirement coverage (22/22) |
| v1.1 | 6,319 | 20 integration | Screen trait arch, 7 tabs |
| v1.2 | 7,630 | 25 integration | Zero warnings, 8 tabs, 9/9 requirements |

### Top Lessons (Verified Across Milestones)

1. Infer state from disk truth, not parsed markdown counters
2. Refactor before feature expansion — pays off immediately in the next phase
3. Autonomous mode works best with well-specified success criteria and clean baselines
