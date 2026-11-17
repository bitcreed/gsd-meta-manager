---
phase: 38-inbox-digest-and-triage-hygiene
reviewed: 2026-11-17T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - README.md
  - src/agent_skill.md
  - src/app.rs
  - src/cli.rs
  - src/config.rs
  - src/inbox.rs
  - src/inbox_cli.rs
  - src/main.rs
  - src/state_reader/owner_items/gate.rs
  - src/state_reader/owner_items/labels.rs
  - src/state_reader/owner_items/mod.rs
  - src/state_reader/owner_items/triage.rs
  - src/ui/screens/help.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/inbox_cli.rs
  - tests/owner_digest.rs
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
  necessity: 0
status: issues_found
---

# Phase 38: Code Review Report

**Reviewed:** 2026-11-17
**Depth:** standard
**Files Reviewed:** 19 (fixtures under `tests/fixtures/owner/{heuristic-labels,same-phase-merge,scripted-checklist,stale-fold}` and the new `owner_golden` files were checked for private text and correctness; a grep for real project names, home paths and addresses found nothing)

## Summary

The phase is sound. The triage model is pure and shared by the CLI and the TUI. Every project-derived string in the new CLI views goes through `terminal_capped`, and the digest budget arithmetic holds (`room` cannot reach zero with the fold blocks the code can emit). `view_of` follows D-02, and `slots` returns exactly one slot per target, so `entries[0]` in the project view is safe. Labels emit constants only, and the label and ranking independence test pins D-11. Layer A is untouched: `gate::lane_items` is a pure extraction with the same arithmetic.

One real defect sits in the TUI row model: a BLOCKED phase that is partly folded produces two rows with the same identity. The rest is documentation drift and small heuristics.

## Warnings

### WR-01: A partly folded BLOCKED phase yields two rows with one GroupKey

**File:** `src/ui/screens/owner_block.rs:575-612` (`inbox_sources`), `entry_key` at `:557`
**Issue:** The BLOCKED aggregate is built once per half (unfolded, folded). It collects its siblings only with the same `folded` value. A (workstream, phase) with one fresh and one older (more than `fold_days`) BLOCKED entry, for example a UAT and a CHECKPOINT of different ages, therefore appears once in the lane and once in the Stale fold. Both rows carry the identical `GroupKey::aggregate` key. `InboxRow::key()` is then not unique. Toggling `expanded` expands both rows at once. `entry_for` and `prune_view` resolve only the first match. `groups_for_copy` copies both entries' groups for whichever row is selected. The CLI has no such collision because it never aggregates BLOCKED rows.
**Fix:** Make the aggregate identity differ per half, or aggregate over all siblings regardless of `folded` and place the row by the newest sibling. For the second option, compute `folded` for the aggregate as `siblings.iter().all(|e| e.folded)` and loop once over the unfiltered entries. Add a test with a fresh and an older BLOCKED entry of one phase that asserts all `InboxRow::key()` values are distinct.

## Info

### IN-01: Owner block checks cell widens a row past the planned grid at 100 or more checks

**File:** `src/ui/screens/owner_block.rs:919` (grid at `:802`)
**Issue:** `pad(&row.checks, CHECKS_W.max(cells(&row.checks)), true)` makes a 3-digit row 10 cells wide. `fixed()` reserves `CHECKS_W = 9` for every row. Such a row shifts its phrase, age and command columns one cell right of its neighbors, and `fit_spans` may clip its tail. The old code truncated instead. The block is documented as frozen for the 36 UAT, and the TUI inbox already handles this through `Grid::checks_w`.
**Fix:** Either keep `pad(&row.checks, CHECKS_W, true)` here (the old truncation), or derive a `checks_w` from the rows in `Grid` as the inbox does and use it in both `fixed()` and `row_line`.

### IN-02: README misdescribes the `looks resolved` label

**File:** `README.md` (schema table row `projects[].groups[].items[].labels`), against `src/state_reader/owner_items/labels.rs:82`
**Issue:** The README says `looks resolved` is for "a failed check whose text says it was fixed". The matcher applies to any item in any lane, including pending checks, and reads the name, `expected` and `reason`.
**Fix:** Reword to "a check whose name, expected text or reason says it was resolved or fixed".

### IN-03: Help overlay and Stale line hardcode or mislabel what is configurable or not a phase

**File:** `src/ui/screens/help.rs` (the `Stale` row), `src/inbox.rs:1138-1148` (`stale_fold_text`)
**Issue:** The help row says "more than 90 days" while the threshold is `preferences.owner.fold_days`, so it is wrong for any user who changes it. Separately, `Stale: N items in P phases` counts folded debug sessions, handoffs and checkpoints as "phases". The `items` unit switch handles the first noun only.
**Fix:** Reword the help row to "older than preferences.owner.fold_days (default 90)". Switch the second noun to `entries` when `totals.non_check` is set, or accept the wording and document it.

### IN-04: Label heuristics have known false positives

**File:** `src/state_reader/owner_items/labels.rs:40,57`
**Issue:** `accept` and `review` match ordinary UAT steps ("Tap Accept on the permission prompt", "Review screen renders"), and `resolved` matches "not resolved yet". The test asserts the last one as intended. This fits the label-only rule (D-11), so it is only a noise risk. It dilutes the `N decision?` count on collapsed lines for UATs that mention UI buttons.
**Fix:** Optional. Drop a leading negation (`not|un`) from the `resolved` hit, and consider matching `accept` only with `owner|approve|ratify` context. Otherwise note it in the README as a heuristic.

### IN-05: Duplicated fixture-aging code in tests

**File:** `tests/inbox_cli.rs:1435-1448` and `:1520-1533`
**Issue:** The same 14-line "rewrite `updated:` to 200 days ago" block appears twice, and `fresh_copy_of` repeats the walk-and-rewrite pattern. A change to the fixture layout has to be made in three places.
**Fix:** Extract `age_file(path: &Path, days: i64)` and have `fresh_copy_of` take a timestamp.

---

_Reviewed: 2026-11-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
