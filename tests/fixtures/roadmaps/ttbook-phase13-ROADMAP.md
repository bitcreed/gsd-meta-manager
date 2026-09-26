# Roadmap

## Overview

(sanitised)

## Milestones

- ✅ **v1 Milestone 1: Baseline records and review** - Phases 1-7.1 (shipped 2026-09-23)
- 🚧 **v2 Core engine and shared service** - Phases 8-13 (in progress)
- 📋 **Milestone 3: Guided first run and scheduled runs** - build phases 14-15 (planned)
- 📋 **Milestone 4: Messaging module** - build phases 16-17 (planned)
- 📋 **Milestone 5: Browser frontend** - build phase 18 (planned)

## Phases

<details>
<summary>✅ v1 Baseline records and review (Phases 1-7.1) - SHIPPED 2026-09-23</summary>

- [x] Phase 1: Access and session handling (R1) (6/6 plans) - completed 2026-09-23; (sanitised)
- [x] Phase 2: Stack and dependency review (R2) (13/13 plans) - completed 2026-09-23
- [x] Phase 3: Architecture and data model (R3) (8/8 plans) - completed 2026-09-22
- [x] Phase 4: Deployment and operations (R4) (9/9 plans) - completed 2026-09-23
- [x] Phase 5: Module redesign (R5) (9/9 plans) - completed 2026-09-22
- [x] Phase 6: Domain rules and open facts (R6) (8/8 plans) - completed 2026-09-22
- [x] Phase 7: Consolidation (12/12 plans) - completed 2026-09-23
- [x] Phase 7.1: Apply rulings and review findings (INSERTED) (16/16 plans) - completed 2026-09-23

(sanitised)

</details>

### 🚧 v2 Core engine and shared service (In Progress)

**Milestone Goal:** (sanitised)

(sanitised)

- [x] **Phase 8: Rule layer and rule data** - (sanitised)
- [x] **Phase 9: Persistence, migrations and snapshots** - (sanitised)
- [x] **Phase 10: Work queue, idempotency and pacing** - (sanitised)
- [x] **Phase 11: Remote client and token port** - (sanitised)
- [ ] **Phase 12: Service layer and command line** - (sanitised)
- [ ] **Phase 13: Host deployment and operations** - (sanitised)

## Phase Details

(sanitised)

### Phase 8: Rule layer and rule data

**Goal**: (sanitised) The pure rule layer enforces every hard limit from versioned rule data that records where each value came from, and the review findings routed to this phase are fixed.
**Depends on**: Milestone 1 (Phase 7: neutral earlier work and its review)
**Plans**: 9 plans in 5 waves

Plans:
**Wave 1**

- [ ] 08-01-PLAN.md — (sanitised)
- [ ] 08-02-PLAN.md — (sanitised)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 08-03-PLAN.md — (sanitised)
- [ ] 08-04-PLAN.md — (sanitised)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 08-05-PLAN.md — (sanitised)

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 08-06-PLAN.md — (sanitised)
- [ ] 08-07-PLAN.md — (sanitised)
- [ ] 08-09-PLAN.md — (sanitised)

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 08-08-PLAN.md — (sanitised)

### Phase 9: Persistence, migrations and snapshots

**Goal**: (sanitised) One local database holds the migrations, an append-only log and the cached tokens, and snapshots, restores and pruning are verified.
**Depends on**: Phase 8
**Plans**: TBD

### Phase 10: Work queue, idempotency and pacing

**Goal**: (sanitised) The engine drives queued work through a single transition table with write-ahead logging, so a retry or a crash never repeats a change.
**Depends on**: Phase 8, Phase 9
**Plans**: TBD

### Phase 11: Remote client and token port

**Goal**: (sanitised) A client behind two ports, tested only against fakes and fixtures, stops on a challenge and never writes while writes are off.
**Depends on**: Phase 8, Phase 9
**Plans**: TBD

### Phase 12: Service layer and command line

**Goal**: (sanitised) One service interface serves every frontend, and a thin command line carries the exit-code table and the output contract.
**Depends on**: Phase 9, Phase 10, Phase 11
**Plans**: TBD

### Phase 13: Host deployment and operations

**Goal**: (sanitised) The system runs on its host as managed units with a credential store, a clock check, health checks, notices, snapshots and a safe update path.
**Depends on**: Phase 12

(sanitised)
**Plans**: TBD

## Planned milestones (placeholders, not GSD phases)

(sanitised)

### 📋 Milestone 3 "Guided first run and scheduled runs" (planned)

#### Build phase 14 (Milestone 3): Guided first run

**Goal**: (sanitised) The first real change is made under supervision and an explicit go-ahead, verified after the write and logged.
**Depends on**: Build phases 8-13
**Plans**: TBD

#### Build phase 15 (Milestone 3): Scheduled window runs

**Goal**: (sanitised) The scheduled run applies due work unattended and fails closed on any uncertainty about timing or access.
**Depends on**: Build phase 14
**Plans**: TBD

### 📋 Milestone 4 "Messaging module" (planned)

#### Build phase 16 (Milestone 4): Messaging client and transcript store

**Goal**: (sanitised) A messaging client and a redacted append-only transcript store run in their own process.
**Depends on**: Build phases 12, 13
**Plans**: TBD

#### Build phase 17 (Milestone 4): Autonomous responder, staged hold and remote control

**Goal**: (sanitised) The responder drafts replies under a staged hold and a data guardrail, with a scripted fallback.
**Depends on**: Build phases 12, 13, 16
**Plans**: TBD

### 📋 Milestone 5 "Browser frontend" (planned)

#### Build phase 18 (Milestone 5): Browser frontend

**Goal**: (sanitised) A small browser frontend offers an overview, a list and queue visibility on top of the one service interface.
**Depends on**: Build phase 12
**Plans**: TBD

(sanitised)

## Progress

**Execution Order:** 8 → 9 → (10 ∥ 11) → 12 → 13

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 8. Rule layer and rule data | 9/9 | Complete | 2026-09-24 |
| 9. Persistence, migrations and snapshots | 9/9 | Complete | 2026-09-24 |
| 10. Work queue, idempotency and pacing | 9/9 | Complete | 2026-09-24 |
| 11. Remote client and token port | 9/9 | Complete | 2026-09-24 |
| 12. Service layer and command line | 0/TBD | In Progress|  |
| 13. Host deployment and operations | 0/TBD | In Progress|  |
