---
phase: 38-inbox-digest-and-triage-hygiene
fixed_at: 2026-11-17T00:00:00Z
review_path: .planning/phases/38-inbox-digest-and-triage-hygiene/38-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 5
skipped: 1
status: partial
---

# Phase 38: Code Review Fix Report

**Fixed at:** 2026-11-17
**Source review:** .planning/phases/38-inbox-digest-and-triage-hygiene/38-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6
- Fixed: 5
- Skipped: 1

Verification ran in the worktree `/home/blk/projects/rust/gsd-meta-manager-owner-inbox` (branch `feat/v2.5.0-owner-inbox`), not the main checkout.

## Fixed Issues

### WR-01: A partly folded BLOCKED phase yields two rows with one GroupKey

**Files modified:** `src/ui/screens/owner_block.rs`, `src/ui/screens/inbox.rs`
**Commit:** acce9eb1
**Applied fix:** `inbox_sources` now aggregates BLOCKED siblings across both halves and places the single aggregate row in the Stale fold only when every sibling is folded (one fresh sibling keeps it in the lane). Regression test `a_partly_folded_blocked_phase_is_one_row` (a fresh UAT plus a 200-day-old CHECKPOINT of one phase; same-phase UAT/VERIFICATION merge first, so a checkpoint is what reproduces it) failed before the fix with two identical keys and passes after. Logic change: fixed, requires human verification of the placement rule (newest sibling decides).

### IN-01: Owner block checks cell widens a row past the planned grid

**Files modified:** `src/ui/screens/owner_block.rs`
**Commit:** 06d58362
**Applied fix:** `Grid` carries `checks_w` (the widest checks cell, at least `CHECKS_W`), used by `fixed()` and `row_line`, as the inbox grid does. Test `a_three_digit_checks_cell_widens_the_planned_grid`.

### IN-02: README misdescribes the `looks resolved` label

**Files modified:** `README.md`
**Commit:** 0093007e
**Applied fix:** Reworded to "a check whose name, expected text or reason says it was resolved or fixed".

### IN-03: Help row hardcodes 90 days; Stale line calls non-phases phases

**Files modified:** `src/inbox.rs`, `src/ui/screens/help.rs`
**Commit:** 4fa7a3e6
**Applied fix:** Help row now reads "Fold of entries idle for longer than preferences.owner.fold_days (default 90)". `stale_fold_text` uses `entries` instead of `phases` when `non_check` is set; the unit test is updated. No golden changed (the golden uses the check wording).

### IN-05: Duplicated fixture-aging code in tests

**Files modified:** `tests/inbox_cli.rs`
**Commit:** c9c9bf49
**Applied fix:** Extracted `stamp_file` and `age_file`; both aging tests and `fresh_copy_of` use them.

## Skipped Issues

### IN-04: Label heuristics have known false positives

**File:** `src/state_reader/owner_items/labels.rs:40,57`
**Reason:** Skipped as an inferred decision. 38-CONTEXT.md D-11 [user: D] allows heuristics only as labels that never move, hide or re-rank a row, and the `heuristic-labels` fixture and a test deliberately pin the near-miss behavior (including "not resolved yet"). Narrowing the keyword matcher would change a user-locked, fixture-pinned heuristic for a pure noise reduction; the README already calls labels hints only.
**Original issue:** `accept`, `review` and `resolved` match ordinary UAT steps, diluting the `N decision?` count.

---

_Fixed: 2026-11-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
