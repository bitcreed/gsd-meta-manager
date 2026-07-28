---
phase: quick-260728-kfx
plan: 01
subsystem: state_reader
tags: [roadmap-parsing, deduplication, tui-phases-pane, bugfix]
status: complete
requires:
  - src/state_reader/roadmap_md.rs::parse_roadmap_phases
provides:
  - src/state_reader/roadmap_md.rs::merge_duplicate_phases
affects:
  - src/state_reader/mod.rs (consumer only — file untouched)
  - TUI Phases pane rendering
tech_stack:
  added: []
  patterns:
    - "Order-preserving dedupe via HashMap<key, index> into an output Vec (single O(n) pass, no nested scan)"
key_files:
  created: []
  modified:
    - src/state_reader/roadmap_md.rs
decisions:
  - "Phases are keyed by `number` alone — the same identity key the caller already uses to build `phase_disk_statuses`"
  - "Field merge rule: `completed` ORs, `name`/`description` take first non-empty, plan counts take max"
metrics:
  duration: 6min
  completed: 2026-07-28
  tasks: 2
  files: 1
  commits: 3
---

# Quick Task 260728-kfx: Dedupe Phases in parse_roadmap_phases Summary

`parse_roadmap_phases` now merges the two roadmap descriptions of each phase (summary checklist + `## Phase Details` heading) into one entry, fixing the TUI Phases pane listing every phase twice.

## What Was Built

A standard GSD 1.8.0 ROADMAP.md describes each phase twice — once as a summary checklist item near the top (`- [x] **Phase 4: Title** - desc`), once under `## Phase Details` as `### Phase 4: Title` with the plan items beneath. Both forms are recognized by the existing regexes, so every phase was emitted twice, and the two copies disagreed on plan counts: the checklist copy stops scanning immediately (the next line is another header) and reports `total_plans: 0`, while the detail copy captures the real counts.

Added a private, pure `merge_duplicate_phases(Vec<RoadmapPhase>) -> Vec<RoadmapPhase>` helper, called as the final expression of `parse_roadmap_phases`. It walks the input once, keeping a `HashMap<String, usize>` from phase number to that number's index in the output `Vec`. First sighting pushes; repeat sightings merge in place:

| Field | Merge rule | Why |
|-------|-----------|-----|
| `completed` | logical OR | the heading form always reports `false`, so the checklist checkbox must survive |
| `name`, `description` | first non-empty wins | the heading form carries no description |
| `total_plans`, `completed_plans` | max | whichever copy actually scanned the plan list wins |

First-seen phase order is preserved, so Phases pane ordering is unchanged.

## Key Implementation Details

- `parse_roadmap_phases`'s public signature, both regex recognizers, the plan-item scan loop, the sentinel filter, and the strikethrough filter are all untouched — the only change to the function body is `phases` → `merge_duplicate_phases(phases)`.
- `RoadmapPhase`, `RoadmapProgress`, `roadmap_progress`, and `src/state_reader/mod.rs` are unmodified.
- Single new import: `use std::collections::HashMap;`.
- The literal `## Phase Details` heading never becomes a phase — `PHASE_ID` requires a leading digit, so `Details` cannot match.

## Verification

Real-world check against the roadmap that motivated the task (`/home/blk/projects/flutter/sentriq/.planning/ROADMAP.md`, which has 5 phases in both a summary checklist and a `## Phase Details` section) via a throwaway test, since a path outside the repo cannot be committed as a fixture:

- Before: 10 entries (phases 4–8 each twice)
- After: `phases=["4", "5", "6", "7", "8"]`, `counts=[(5,5), (0,0), (0,0), (0,0), (0,0)]`

The throwaway test was removed after the check; the working tree was confirmed clean against the last commit.

Committed regression tests:

- `test_parse_roadmap_dedupes_summary_and_details` — checklist-then-details layout; asserts 3 phases (was 6), first-seen order `["4","5","6"]`, description and checkbox survive from the checklist form, plan counts survive from the detail section, and no phase is named `Details`.
- `test_parse_roadmap_dedupes_details_before_checklist` — mirror layout; asserts 1 phase with `completed: true` (OR rule), description filled by the later copy, and counts `2`/`1` retained from the earlier copy (max rule).

Gate results:

- `cargo build` — succeeds
- `cargo test` — 212 passed across 5 suites
- `cargo clippy -- -D warnings` — exits 0, no `#[allow]` attributes needed

All pre-existing `roadmap_md` tests pass with no edits to their assertions, notably `test_parse_roadmap_heading_levels_and_plans` (distinct numbers 2 and 3 → still 2 phases) and `test_parse_roadmap_details_wrapped` (distinct numbers 1 and 2 → still 2 phases, counts 2/2 on phase 1).

## Deviations from Plan

None — plan executed exactly as written.

## Threat Model Compliance

- **T-kfx-01 (DoS)** — mitigated: single O(n) pass with a `HashMap` index; no nested scan over `phases`, so a roadmap with thousands of repeated headings stays linear.
- **T-kfx-02 (Tampering)** — mitigated: the merge only combines values already produced by the existing recognizers; no new parsing, no new field sources, and the sentinel/strikethrough filters still run before any phase enters the merge.
- **T-kfx-03 (Info disclosure)** — accepted as planned: the merge changes cardinality, not what is rendered.

No new security-relevant surface introduced. No package installs.

## Known Stubs

None.

## Commits

| Commit | Type | Description |
|--------|------|-------------|
| 19e278a | test | failing test for duplicate phase merge (RED) |
| a3877ea | fix | `merge_duplicate_phases` dedupe implementation (GREEN) |
| d9953a0 | test | details-before-checklist merge order coverage |

## Self-Check: PASSED

- `src/state_reader/roadmap_md.rs` — FOUND, contains `merge_duplicate_phases` and both `test_parse_roadmap_dedupes_*` tests
- Commit 19e278a — FOUND
- Commit a3877ea — FOUND
- Commit d9953a0 — FOUND
- `src/state_reader/mod.rs` — untouched (confirmed via `git diff --stat`: 1 file changed across the whole task)
