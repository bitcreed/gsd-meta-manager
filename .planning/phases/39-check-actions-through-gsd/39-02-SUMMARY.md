---
phase: 39-check-actions-through-gsd
plan: 02
subsystem: owner-inbox-tui
tags: [rust, ratatui, inbox, owner-block, settle, move, item-cursor]
requires:
  - phase: 39-01
    provides: owner_settle core, settle_popup screens, inbox x arm
provides:
  - "item sub-cursor (j/k walk of drawn checks) in the inbox and the detail Owner block; x on an item settles ONE check (OWN-29)"
  - "Owner block x arm (D-05)"
  - "Move outcome: x m, phase step, optional reason, popup rows and the D-06 warning; instruction asks GSD for the deferral record and one todo for phase M (OWN-30, OWN-31 filing half, by instruction)"
affects: [39-03]
status: complete
commits: 3
plan_head_before: edb1e7f2a48927b6bab2cae5d23d1edb50914692
plan_head_after: 790038791b79abaf282cd57fcae55fa4818c1bc1
actuals:
  tokens: 60000
  tasks: 3
  commits: 3
key-files:
  modified:
    - src/ui/screens/owner_block.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/render_escape_guard.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/mod.rs
key-decisions:
  - "Item cursor is a sub-cursor beside fold_cursor (identity = OwnerItem::id); Stop/InboxStop enums unchanged"
  - "Phase step is its own screen between chooser and reason field"
requirements-completed: []
---

# Phase 39 Plan 02: Check rows, Owner block, Move Summary

`j`/`k` now walk the drawn checks (first five) of an expanded row in both the inbox and the Owner block, `x` on a
check settles that one check, and `x m` moves checks to another phase through the same `/gsd-verify-work N`
launch. The app runs no todo command; the instruction asks GSD to file one todo titled with phase M's number and
name. REQUIREMENTS.md is intentionally not updated (39-03 owes parts).

## Tasks

1. Tracer: item sub-cursor in owner_block, highlight, Owner block `x` arm (7f0ac574)
2. Inbox item walk, highlight, check targets, parent-group parity (708735d9)
3. Move: candidates, phase step, template, popup rows, escape guard (79003879)

## Verification

- `rtk bypass cargo test --no-fail-fast` (raw `test result:` lines summed): 4553 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt` applied. No fixture file changed.
- Phase-level Move popup (heuristic-labels 01 to 02, 7 checks) at 80x24: 18 of 19 body rows (unclipped); the check
  popup fits at 80x24, the phase popup at 100x30.

## Deviations from Plan

**1. [Rule 1 - behavior change] Existing tests walked `j` over expanded rows**
- `owner_block_state_survives_a_digest_refresh`, `sc4_an_expanded_group_shows_its_checks_beside_the_collapsed_rows`
  (detail), `enter_expands_a_group_with_its_run_line`, `expansion_survives_a_refresh_and_a_vanished_group_is_dropped`,
  `the_fold_line_walks_and_opens_old_debt_and_acknowledged` (inbox) assumed one `j` leaves an expanded row; D-04 makes
  `j` visit its item lines first. Their key sequences were extended (no assertion about layout changed). The
  `owner_block_swallows_action_keys` test no longer lists `x` (now a settle key; covered by new tests).
- No golden changed.

**2. [Rule 1 - bug] No-cursor-yet inbox**: with `cursor == None` (first stop implicit) an item could not attach;
`step_items` now materializes the cursor first. Commit 708735d9.

**3. Dead-code placement**: `expansion_lines` is `cfg(test)`; the unmapped `inbox_expansion_lines` was removed (the
mapped builder replaced it).

## Inferred decisions

- [inferred] The chooser row reads `[m] move` (not `[m] move to phase`): the longer text clips `[Esc] cancel` at 80
  columns for a phase target.
- [inferred, plan] Phase step is its own screen; `j`/`k` move the pick only while the typed entry is empty; a letter
  first is ignored; typed entry takes ASCII digits, letters, `.` up to 16 chars.
- [inferred] A candidate with an empty cleaned name is listed unnamed; `1x` style tokens are valid phase tokens.
- [inferred] `x` on the inbox fold line keeps 39-01's wording; the Owner block fold line says `Select a check or a
  group row to settle it`.
- Whether gsd-core's keyword match ranks the filed todo for phase M remains human UAT (39-03).
- Edge probe OWN-29 (unclassified) stays flagged for human review.

## Known Stubs

None.

## Self-Check: PASSED

Commits 7f0ac574, 708735d9, 79003879 are ancestors of HEAD; modified files exist.
