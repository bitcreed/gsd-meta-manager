---
phase: 35-decisions-code-needed-and-gates
plan: 03
subsystem: owner-inbox
tags: [owner-items, handoff, checkpoint, todos, inbox, status]
requires:
  - phase: 35-01
    provides: signal_item, the decide counting rule, Kind::{Handoff,Checkpoint}
  - phase: 35-02
    provides: OpenScan, context groups, ArchivedCounts.decide
provides:
  - "owner_items::handoff (read_handoffs, read_checkpoints): native, capped, contained readers"
  - "owner_items::todos (todo_stats) over oneoff::todos::pending_paths"
  - "Decide groups of kind handoff and checkpoint; Fix group for an empty gaps_found VERIFICATION"
  - "hidden.todos_oldest_days and hidden.todos_unreadable (schema v1, append-only)"
  - "one clock reading per run in inbox and status"
affects: [35-04]
tech-stack:
  added: []
  patterns: ["PauseScan input beside OpenScan", "shared enumeration for count and listing"]
key-files:
  created:
    - src/state_reader/owner_items/handoff.rs
    - src/state_reader/owner_items/todos.rs
    - tests/fixtures/owner/handoff-paused/
    - tests/fixtures/owner/handoff-workstreams/
    - tests/fixtures/owner/checkpoint-phase/
    - tests/fixtures/owner/verification-gaps-empty/
    - tests/fixtures/owner/todos-age/
  modified:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/scan.rs
    - src/oneoff/todos.rs
    - src/inbox.rs
    - src/inbox_cli.rs
    - src/status_cli.rs
    - README.md
    - tests/inbox_cli.rs
    - tests/owner_digest.rs
key-decisions:
  - "A stale handoff is listed with stale: true, never filtered ([INFERRED] F1); paused, needs_human and the attention items are untouched (D-09)"
  - "A checkpoint counts 1 in decide even when unreadable (no item); the plan's item-count rule would have counted 0"
  - "build_digest_with takes a PauseScan; build_digest stays as a cfg(test) wrapper so the existing rule tests are unchanged"
  - "The empty gaps_found group sets unreadable 1 in the loop and restores the planning command afterwards, so the Phase 34 suppression for unreadable files is untouched"
requirements-completed: [OWN-10]
duration: one session
completed: 2026-11-14
status: complete
actuals:
  tokens: 21000
  tasks: 2
  commits: 2
plan_head_before: 3e4c1ebbb8080a475be956fd13c1b9477fceb396
plan_head_after: 8875770c2d30f2e61adf8b72966e575a83ae93e1
---

# Phase 35 Plan 03: Handoffs, checkpoints, empty gaps and todo age Summary

Paused handoffs (with their pending human actions and blockers), open phase checkpoints and `gaps_found` verifications that list nothing now reach the owner inbox as Decide and Fix work, and the inbox reports the pending-todo count with its oldest age and an unreadable flag, all read natively from their stated fields.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 handoffs and checkpoints as Decide groups, SC1 fixture CLI test | cc6fdcbf | `handoff.rs`, `PauseScan`, group builders, DECIDE rows, fixtures `handoff-paused`, `handoff-workstreams`, `checkpoint-phase` |
| 2 empty gaps_found Fix work, todo stats, one clock | 8875770c | D-07 group, `todos.rs`, `pending_paths`, two new `hidden` keys, `now` passed into `read_entry` / `project_entry`, fixtures `verification-gaps-empty`, `todos-age` |

## Verification (raw)

- `cargo test --no-fail-fast` (whole crate): 4150 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `owner_audit_conformance` 3 passed (all five new fixtures accepted with `oracle.expect` `same`); `owner_open_conformance` 3 passed; `status_cli` 28 passed; `inbox_cli` 24 passed; `owner_digest` 24 passed.
- SC1 fixture form (`sc1_decide_lane_lists_the_awaiting_sessions_and_the_paused_handoff`): 5 debug groups, one stale handoff with items `HANDOFF.json#a1` / `#b1`, `summary.decide` 6.
- todos-age at now 2026-11-14: `hidden.todos` 4, `todos_oldest_days` 166, `todos_unreadable` false.
- SC4: no key-path string removed from `INBOX_PATHS` (diff gate prints 0); `todos_count` count 0; goldens untouched.
- Read-only check on usbee (not committed): `inbox --json --lane decide` shows one `handoff` group, `stale: true`, `file_status: "paused"`, 6 items (RESEARCH saw 6); `hidden.todos` 1, `todos_oldest_days` 184, `todos_unreadable` false.

## Deviations from Plan

**1. [Rule 3 - Blocking] Workstream fixtures need a `phases/` directory.** The audit-uat oracle aborts with "No phases directory found" for a workstream tree without one; `handoff-workstreams` gained `workstreams/{a,b}/phases/03-stream/03-01-PLAN.md`.

**2. [Rule 1 - Correctness] An unreadable checkpoint counts 1 in `decide`.** The plan's counting rule (items) gives 0 for a group without items, which hides a failure. The counting loop now treats `Kind::Checkpoint` like `Kind::Handoff` (always 1).

**3. [Rule 3 - Blocking] `build_digest` kept as a `cfg(test)` wrapper.** Production calls `build_digest_with(files, open, pause, ...)`; the ~12 existing rule tests keep the old signature.

**4.** Handoff list entries are capped at 200 per list (unplanned hardening, T-35-12 spirit); `phase` text that is not a command token (`Phase 12`) gives `""` instead of being parsed.

## Inferred decisions

F1 stale-but-listed handoffs; A8 terminal checkpoint statuses; A5 `todos_unreadable` instead of a null count; D-07 command kept with `counts.unreadable` 1; object entries without the name field (`action` / `description`) are skipped; a handoff whose JSON is invalid or whose file cannot be read is an unreadable Decide group, not absent; a symlinked HANDOFF (or one escaping the tree) is treated as absent.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register: T-35-11 (commands only from validated tokens, `bad name` and `../x` unit-tested), T-35-12 (1 MiB boundary test, invalid JSON, unreadable checkpoint), T-35-13 (symlink test), T-35-14 (200 cap test), T-35-15 (`an_unreadable_pending_dir_is_flagged`).

## Self-Check: PASSED

Created files exist (handoff.rs, todos.rs, five fixture directories); commits cc6fdcbf and 8875770c are ancestors of HEAD.
