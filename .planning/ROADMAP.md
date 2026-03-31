# Roadmap: GSD Meta Manager

## Milestones

- **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- **v1.1 Polish & Power Features** - Phases 05-09 (shipped 2026-03-27)
- **v1.2 Housekeeping & Archive Browser** - Phases 10-13 (in progress)

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>v1.0 MVP (Phases 01-04) - SHIPPED 2026-03-26</summary>

See `.planning/milestones/v1.0-phases/` for archived phase artifacts.

Phase 01: Project Foundation (3 plans, complete)
Phase 02: Dashboard & Navigation (3 plans, complete)
Phase 03: Live State & Visualization (2 plans, complete)
Phase 04: Project Creation & Queue (2 plans, complete)

</details>

<details>
<summary>v1.1 Polish & Power Features (Phases 05-09) - SHIPPED 2026-03-27</summary>

See `.planning/milestones/v1.1-phases/` for archived phase artifacts.

Phase 05: State Reader Accuracy (5 plans, complete)
Phase 06: Read-Only Views (4 plans, complete)
Phase 07: Execution Flow & GSD Integration (3 plans, complete)
Phase 08: Queue Execution (2 plans, complete)
Phase 09: Claude Session Management (2 plans, complete)

</details>

### v1.2 Housekeeping & Archive Browser

**Milestone Goal:** Clean up tech debt, add paused-project detection, build milestone archive browsing in the detail view, and research queue execution integration with GSD.

- [x] **Phase 10: Tech Debt Cleanup** - Resolve compiler warnings, fix stale integration test, establish clean baseline (completed 2026-03-31)
- [x] **Phase 11: Paused Project Detection** - Show pause badge on dashboard for projects with HANDOFF.md/HANDOFF.json (completed 2026-03-31)
- [ ] **Phase 12: Milestone Archive Browser** - Browse completed milestones and drill into past phase artifacts from the detail view
- [ ] **Phase 13: Queue Execution Research** - Document GSD autonomous mode, hook points, and design for auto-continue from QUEUE.md

## Phase Details

### Phase 10: Tech Debt Cleanup
**Goal**: Users see a clean, warning-free build and all tests pass as a reliable baseline for new feature work
**Depends on**: Phase 09 (v1.1 complete)
**Requirements**: DEBT-01, DEBT-02
**Success Criteria** (what must be TRUE):
  1. `cargo build` completes with zero warnings (no `#[allow(dead_code)]` suppressions except those needed by upcoming archive browser code)
  2. `cargo nextest run` passes all integration tests including `end_to_end_add_then_list_via_cli` with correct CLI argument order
**Plans:** 1/1 plans complete

Plans:
- [x] 10-01-PLAN.md -- Resolve dead code annotations, clippy warnings, and formatting

### Phase 11: Paused Project Detection
**Goal**: Users can tell at a glance which projects are paused and see pause context without opening files
**Depends on**: Phase 10
**Requirements**: PAUSE-01
**Success Criteria** (what must be TRUE):
  1. User sees a pause badge (distinct icon/color) on dashboard rows for any project with a HANDOFF.md or HANDOFF.json file in `.planning/`
  2. Pause badge takes priority over session-active indicator when both conditions are true
  3. Pause detection updates automatically when HANDOFF files appear or are removed (via existing file watcher)
**Plans:** 1/1 plans complete

Plans:
- [ ] 11-01-PLAN.md -- HANDOFF detection, dashboard badge, and detail view pause context

### Phase 12: Milestone Archive Browser
**Goal**: Users can browse completed milestones and drill into past phase artifacts without leaving the TUI
**Depends on**: Phase 11
**Requirements**: ARCH-01, ARCH-02, ARCH-03, ARCH-04
**Success Criteria** (what must be TRUE):
  1. User sees a list of completed milestones in an Archive tab within the detail view
  2. User can select a milestone and see its phases, then select a phase to see its artifact files
  3. User can view a selected artifact file with styled rendering (headers, bold, lists, code blocks distinguishable from plain text)
  4. Archive data loads asynchronously without blocking the TUI render loop, and completed milestone data is cached across tab switches
**Plans:** 1/3 plans executed

Plans:
- [x] 12-01-PLAN.md -- Archive data layer: types, discovery, loading, markdown renderer
- [ ] 12-02-PLAN.md -- UI integration: Archive tab, state fields, key handling, rendering
- [ ] 12-03-PLAN.md -- Build verification and visual UAT checkpoint

### Phase 13: Queue Execution Research
**Goal**: A design document exists that enables v1.3 implementation of queue execution without further research
**Depends on**: Phase 10 (independent of Phases 11-12; ordered last because it is documentation-only)
**Requirements**: QRES-01, QRES-02
**Success Criteria** (what must be TRUE):
  1. Research document covers GSD autonomous mode lifecycle, `claude -p` / `--continue` / `--resume` CLI capabilities, and hook points for automation
  2. Research document includes a concrete design for auto-continue from QUEUE.md with at least two integration strategies, identified trade-offs, and safety requirements
  3. Confidence levels are explicitly stated for each design element (HIGH/MEDIUM/LOW) to guide v1.3 planning
**Plans**: TBD

Plans:
- [ ] TBD

## Backlog

### Phase 999.2: Container Support with Claude Command Injection (BACKLOG)

**Goal:** Start, stop, and resume Claude sessions inside containers mapped to project directories. Monitor container output and inject commands directly into running Claude instances from the TUI -- enabling remote/isolated execution without terminal switching.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

Note: Backlog 999.1 (Milestone Archive Browser) promoted to Phase 12 in v1.2.

## Progress

**Execution Order:**
Phases execute in numeric order: 10 -> 11 -> 12 -> 13

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 10. Tech Debt Cleanup | v1.2 | 1/1 | Complete    | 2026-03-31 |
| 11. Paused Project Detection | v1.2 | 0/1 | Complete    | 2026-03-31 |
| 12. Milestone Archive Browser | v1.2 | 1/3 | In Progress|  |
| 13. Queue Execution Research | v1.2 | 0/0 | Not started | - |
