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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 83 | 4 | Initial build — established TEA pattern, file watching, state reader |

### Cumulative Quality

| Milestone | LOC | Tests | Key Metric |
|-----------|-----|-------|------------|
| v1.0 | 3,887 | 12 integration | Full requirement coverage (22/22) |

### Top Lessons (Verified Across Milestones)

1. Infer state from disk truth, not parsed markdown counters
