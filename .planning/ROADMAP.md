# Roadmap: GSD Manager

## Overview

Build a Rust TUI dashboard that lets users see every GSD project's status at a glance and act on any of them without leaving the terminal. The journey starts by laying a correct async foundation (event loop, terminal lifecycle, state reader, registry) before adding the visible dashboard and navigation layer, then making it live with file watching and drill-down detail, and finally shipping the GSD-specific differentiators: roadmap visualization, project creation, and work enqueue.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Core Infrastructure** - Async TUI foundation, state reader, and project registry
- [ ] **Phase 2: Dashboard and Navigation** - Main project list, status indicators, keyboard nav, search, and help overlay
- [ ] **Phase 3: Live State and Detail View** - File-watcher auto-refresh and project drill-down
- [ ] **Phase 4: Visualization, Creation, and Enqueue** - ASCII roadmap, new project creation, and work enqueue

## Phase Details

### Phase 1: Core Infrastructure
**Goal**: The application starts, renders cleanly, and can read and persist GSD project state — the tested foundation everything else builds on
**Depends on**: Nothing (first phase)
**Requirements**: REG-01, REG-02, REG-03, STATE-01, STATE-02, STATE-03
**Success Criteria** (what must be TRUE):
  1. User can add a GSD project by path and it is validated and saved across restarts
  2. User can remove a tracked project and it is gone on next launch
  3. Typed ProjectState structs are parsed from `.planning/` files (STATE.md, ROADMAP.md, config.json) without invoking GSD
  4. Phase progress and backlog item counts are readable from the parsed state
  5. Terminal exits cleanly under all conditions including panic — no corrupted terminal state
**Plans:** 3 plans

Plans:
- [x] 01-01-PLAN.md — Project scaffold, config/registry, and CLI subcommands (add/remove/list)
- [x] 01-02-PLAN.md — State reader module parsing STATE.md, ROADMAP.md, config.json, and backlog count
- [x] 01-03-PLAN.md — Async event loop, TUI terminal lifecycle, and stub project list UI

**UI hint**: yes

### Phase 2: Dashboard and Navigation
**Goal**: Users can see all registered projects at a glance and navigate the TUI with keyboard-driven workflows
**Depends on**: Phase 1
**Requirements**: DASH-01, DASH-02, DASH-03, NAV-01, NAV-02, NAV-03, NAV-04, NAV-05
**Success Criteria** (what must be TRUE):
  1. User sees a scrollable project list with name, current phase, and status per row, color-coded by workflow state
  2. A persistent status bar shows aggregate counts across all projects (e.g., "5 projects: 2 active, 1 blocked, 2 idle")
  3. User navigates with vim-style keys (j/k, Enter, Esc, q) and sees a help overlay on `?`
  4. User presses `/` to filter the project list by name or status
  5. TUI adapts to terminal size changes and exits cleanly with full terminal state restored
**Plans:** 2 plans

Plans:
- [x] 02-01-PLAN.md — Color-coded dashboard table with rich columns, aggregate status bar, and resize handling
- [x] 02-02-PLAN.md — Help overlay popup and search/filter visual verification

**UI hint**: yes

### Phase 3: Live State and Detail View
**Goal**: The dashboard stays current without manual refresh, and users can drill into any project for a phase-level breakdown
**Depends on**: Phase 2
**Requirements**: STATE-04, STATE-05, DET-01, DET-02, DASH-05
**Success Criteria** (what must be TRUE):
  1. Dashboard auto-refreshes when any registered project's `.planning/` files change — no user action needed
  2. User can drill into a project and see: path, all roadmap phases, current phase, and task completion counts
  3. Detail view shows per-phase status (pending, in-progress, complete)
  4. A change summary shows what changed since last visit (e.g., "Phase 3 completed 2h ago")
**Plans:** 2 plans

Plans:
- [x] 03-01-PLAN.md — File watcher with notify 8.x debouncer, auto-refresh on .planning/ changes
- [ ] 03-02-PLAN.md — Detail view with phase breakdown, plan counts, status icons, and change summary

**UI hint**: yes

### Phase 4: Visualization, Creation, and Enqueue
**Goal**: Users can see a project's full roadmap visually, create new GSD projects from the TUI, and queue next actions
**Depends on**: Phase 3
**Requirements**: DASH-04, CREATE-01, CREATE-02, CREATE-03, ENQ-01, ENQ-02, ENQ-03
**Success Criteria** (what must be TRUE):
  1. User sees an ASCII roadmap visualization of a project's phases with progress markers for a selected project
  2. User can create a new GSD project (name + path) from the TUI — directory and git repo initialized (user runs /gsd:new-project for full GSD init)
  3. Newly created project appears in the dashboard immediately after creation
  4. User can enqueue a next action for a project, see it in the detail view, stored in .planning/QUEUE.md
**Plans:** 2/3 plans executed

Plans:
- [x] 04-01-PLAN.md — ASCII roadmap widget with vertical pipeline and detail view toggle
- [x] 04-02-PLAN.md — Project creation flow with git init, hooks, and auto-registration
- [ ] 04-03-PLAN.md — Work enqueue with QUEUE.md, command suggestions, and detail view display

**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core Infrastructure | 3/3 | Complete | - |
| 2. Dashboard and Navigation | 2/2 | Complete | - |
| 3. Live State and Detail View | 0/2 | Not started | - |
| 4. Visualization, Creation, and Enqueue | 2/3 | In Progress|  |

## Backlog

### Phase 999.1: Execution Flow Graph View (BACKLOG)

**Goal:** Per-project ASCII execution flow graph showing the full discuss→plan→execute→verify pipeline for each phase, with color-coded status. A third detail view tab (alongside PhaseList and RoadmapViz) toggled via a key. Scans each phase directory for `*-CONTEXT.md`, `*-PLAN.md`, `*-SUMMARY.md`, `*-VERIFICATION.md` to determine pipeline stage. Renders vertical flow with phases connected by arrows, each showing sub-stages (discuss, plan with count, execute with waves, verify with score). Colors: green=done, yellow=in-progress, dim=pending. Similar to the ASCII roadmap widget but focused on the GSD workflow pipeline rather than just phase status.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

### Phase 999.3: Queue Editor and Reorder (BACKLOG)

**Goal:** Add ability to edit and reorder enqueued actions in the detail view. Currently queued items are append-only via `e` key. This adds: select a queued item with j/k, delete with `d`, move up/down with `K`/`J` (shift), edit in-place with `Enter`. Changes persist back to `.planning/QUEUE.md`. Extends the existing `queue_md` module's read/write capabilities.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

### Phase 999.2: Milestone Plan Editor with Claude Launch (BACKLOG)

**Goal:** Navigate milestones and phases via Tab key, directly edit PLAN.md files inline in the TUI, and launch a Claude Code session into a specific phase's directory for hands-on work. Research needed on how to spawn/attach to a Claude terminal session — options include `std::process::Command` to launch `claude` CLI in a new terminal, tmux/screen session management, or embedded terminal widget. The editing piece is a text editor widget for PLAN.md files. The navigation piece extends the existing detail view with Tab cycling through milestones/phases.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

### Phase 999.4: Fix Roadmap Parser Plan Counting (BACKLOG)

**Goal:** Fix the `parse_roadmap_phases` function in `roadmap_md.rs` which incorrectly counts plans per phase. The parser scans lines between checklist headers for plan items, but plans live in `### Phase N:` detail subsections further down. P1-P3 get 0 plans (adjacent checklist lines), P4 captures all 10. Fix: parse plan items from the `## Phase Details` subsections keyed by phase number, or better yet, count `*-PLAN.md` and `*-SUMMARY.md` files on disk per phase directory instead of parsing the markdown.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)
