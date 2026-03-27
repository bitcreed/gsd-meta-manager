# Roadmap: GSD Meta Manager

## Milestones

- **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- **v1.1 Polish & Power Features** - Phases 05-09 (in progress)

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

### v1.1 Polish & Power Features

**Milestone Goal:** Fix state reader accuracy, add read-only project views (backlog, git, flow graph), GSD integration, and make the queue actionable with Claude session management.

- [x] **Phase 05: State Reader Accuracy** - Fix plan counting, completed-milestone display, and disk-based phase inference (completed 2026-03-26)
- [x] **Phase 06: Read-Only Views** - Backlog browser and git history viewer as new detail-view screens (completed 2026-03-27)
- [ ] **Phase 07: Execution Flow & GSD Integration** - Per-phase pipeline visualization and verified-vs-inferred status badges
- [ ] **Phase 08: Queue Execution** - Make queued items executable with confirmation, status tracking, and terminal handoff
- [ ] **Phase 09: Claude Session Management** - Detect, browse, and launch Claude sessions from the TUI

## Phase Details

### Phase 05: State Reader Accuracy
**Goal**: Users see correct, trustworthy project state on the dashboard without manual verification
**Depends on**: Phase 04 (v1.0 foundation)
**Requirements**: STATE-01, STATE-02, STATE-03, CLI-01
**Success Criteria** (what must be TRUE):
  1. User sees correct plan counts per phase (standalone PLAN.md files counted accurately)
  2. User sees "Complete" status when all phases in a milestone are done (not "P5: Unknown")
  3. User sees phase status derived from disk artifacts (discuss/research/plan/execute/verify stages) with confidence indicators
  4. User can register a project by path only — name auto-derived from last folder component
**Plans**: 5 plans

Plans:
- [x] 05-01-PLAN.md — Fix plan counting regex, milestone completion bug, and CLI alias auto-derive
- [x] 05-02-PLAN.md — Screen architecture refactor (InputMode to Screen trait) and async I/O migration
- [x] 05-03-PLAN.md — Disk inference module and compact pipeline display on dashboard
- [x] 05-04-PLAN.md — Gap closure: wire pipeline display and disk status brackets into active screen files
- [x] 05-05-PLAN.md — Gap closure: drop expanded status text, add detail view legend

### Phase 06: Read-Only Views
**Goal**: Users can browse backlog items and git history without leaving the TUI
**Depends on**: Phase 05
**Requirements**: BLOG-01, BLOG-02, BLOG-03, GIT-01, GIT-02, GIT-03
**Success Criteria** (what must be TRUE):
  1. User can scroll through backlog items (999.*) in a list within the detail view
  2. User can view markdown content of a selected backlog item
  3. User can queue a promotion command for a backlog item via the existing queue system
  4. User can view a scrollable git log for any registered project
  5. User can toggle between full-repo and .planning/-scoped git history
**Plans**: 4 plans

Plans:
- [x] 06-01-PLAN.md — Tab system foundation, data models, Action variants, and tab bar navigation
- [x] 06-02-PLAN.md — Backlog browser: scrollable list, content preview, queue promotion
- [x] 06-03-PLAN.md — Git history viewer: scrollable log, planning-only toggle, diff stats
- [x] 06-04-PLAN.md — Gap closure: backlog split-pane content preview and queue promotion wiring

### Phase 07: Execution Flow & GSD Integration
**Goal**: Users see per-phase workflow pipeline status and can distinguish verified facts from disk-inferred state
**Depends on**: Phase 06
**Requirements**: FLOW-01, FLOW-02, FLOW-03, GSD-01, GSD-02
**Success Criteria** (what must be TRUE):
  1. User sees a per-phase pipeline visualization showing discuss/research/plan/execute/verify stages
  2. User sees color-coded status per pipeline stage (not started, current, complete, skipped)
  3. User sees plan execution progress as a fraction in the execute stage
  4. User sees [verified] vs [inferred] badges on status fields when GSD integration is enabled
**Plans**: 3 plans

Plans:
- [x] 07-01-PLAN.md — Extend data model (DiskInference fields, config toggle, Pipeline variant)
- [ ] 07-02-PLAN.md — Pipeline tab with phase selection and stage visualization
- [x] 07-03-PLAN.md — Verified/inferred badges on phase status lines

### Phase 08: Queue Execution
**Goal**: Users can execute queued items from the TUI with process lifecycle management
**Depends on**: Phase 07
**Requirements**: QUEUE-01, QUEUE-02, QUEUE-03
**Success Criteria** (what must be TRUE):
  1. User can execute a queued item and sees a confirmation dialog before it runs
  2. User sees queue item status (pending/in-progress/done/failed) update in real time
  3. User can add, remove, and reorder queue items from the queue view
**Plans**: TBD
**UI hint**: yes

### Phase 09: Claude Session Management
**Goal**: Users can see which projects have active Claude sessions and launch or resume sessions from the TUI
**Depends on**: Phase 08
**Requirements**: SESS-01, SESS-02, SESS-03
**Success Criteria** (what must be TRUE):
  1. User sees an active-session indicator on the dashboard for projects with running Claude sessions
  2. User can browse a session list showing last activity and status in the detail view
  3. User can resume an existing or launch a new Claude session from the TUI
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 05 -> 06 -> 07 -> 08 -> 09

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 5. State Reader Accuracy | v1.1 | 5/5 | Complete | 2026-03-26 |
| 6. Read-Only Views | v1.1 | 0/4 | Not started | - |
| 7. Execution Flow & GSD Integration | v1.1 | 0/3 | Not started | - |
| 8. Queue Execution | v1.1 | 0/0 | Not started | - |
| 9. Claude Session Management | v1.1 | 0/0 | Not started | - |
