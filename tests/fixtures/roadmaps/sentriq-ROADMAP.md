# Sentriq — Roadmap

> (sanitised)

## Current Milestone

**v0.12 — Actuation Routines** (phases 9-12)

**Milestone goal:** (sanitised)

(sanitised)

## Pre-GSD Phases (1-3) — index only

(sanitised)

| Phase | Name | Status | Record |
|-------|------|--------|--------|
| 1 | (sanitised) | Complete | (sanitised) |
| 2 | (sanitised) | Shipped; statuses were stale | (sanitised) |
| 3 | (sanitised) | Shipped; open remnants are now todos | (sanitised) |

## v0.11 Phases (4-7) — archived, index only

(sanitised)

| Phase | Name | Status | Archived detail |
|-------|------|--------|------------------|
| 4 | (sanitised) | Complete | (sanitised) |
| 5 | (sanitised) | Complete | (sanitised) |
| 6 | (sanitised) | Complete | (sanitised) |
| 7 | (sanitised) | Complete | (sanitised) |

(sanitised)

## TASK-111 Phases 1-4 — landed as quick tasks before this milestone opened

(sanitised)

| TASK-111 Phase | What it built | Requirements | Landed as |
|-----------------|---------------|---------------|-----------|
| 1 | (sanitised) | (sanitised) | (sanitised) |
| 2 | (sanitised) | (sanitised) | (sanitised) |
| 3 | (sanitised) | (sanitised) | (sanitised) |
| 4 | (sanitised) | (sanitised) | (sanitised) |

(sanitised)

## Phases

**Phase Numbering:**

- Integer phases (9, 10, 11, 12): planned milestone work
- Decimal phases (9.1, 9.2): urgent insertions (marked with INSERTED)

Milestone v0.12 continues from the last v0.11 phase and starts at Phase 9.

- [ ] **Phase 9: Routine Event Logging (Schema v18)** - (sanitised)
- [ ] **Phase 10: The Air Box Test on a Fake Transport** - (sanitised)
- [ ] **Phase 11: First Supervised On-Vehicle Run** - (sanitised)
- [ ] **Phase 12: Two-Truck Hardware Validation** - (sanitised)

## Phase Details

### Phase 9: Routine Event Logging (Schema v18)

**Goal**: (sanitised) A dispatch attempt, including every refusal, is durably on the record before anything leaves the device, capturing the state at dispatch, and that record survives every deletion path the app has.
**Depends on**: Nothing within v0.12's GSD-tracked phases (neutral note about earlier quick tasks)
**Plans**: 2 plans

Plans:
- [ ] 09-01-PLAN.md — (sanitised)
- [ ] 09-02-PLAN.md — (sanitised)

---

### Phase 10: The Air Box Test on a Fake Transport

**Goal**: (sanitised) The first routine runs end to end against a fake transport and an injected clock, with exact frames in order, the held dwell and every documented status resolving to its own meaning, and nothing reaching real hardware.
**Depends on**: Phase 9 (neutral reason)
**Plans**: TBD

---

### Phase 11: First Supervised On-Vehicle Run

**Goal**: (sanitised) One supervised dispatch on real hardware proves the screen and the wire agree, with any divergence from the reference capture written up.
**Depends on**: Phase 10 (neutral reason one), Phase 9 (neutral reason two)
**Plans**: TBD

---

### Phase 12: Two-Truck Hardware Validation

**Goal**: (sanitised) On real hardware, both units work from one device with separately scoped data, and a first real session is logged with a completed scan.
**Depends on**: Nothing in this milestone — opportunistic, and blocks on nothing in Phases 9-11
**Plans**: TBD

## Progress

**Execution Order:** (sanitised)

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 9. Routine Event Logging (Schema v18) | 0/2 | Planned | - |
| 10. The Air Box Test on a Fake Transport | 0/TBD | Not started | - |
| 11. First Supervised On-Vehicle Run | 0/TBD | Not started | - |
| 12. Two-Truck Hardware Validation | 0/TBD | Not started (carried from v0.11) | - |

## Scope Explicitly Excluded from v0.12

(sanitised)

- (sanitised)
- (sanitised)

## Ad-hoc Work

(sanitised)
