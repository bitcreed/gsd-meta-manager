---
phase: quick-261118-u4e
plan: 01
subsystem: ui
tags: [ratatui, owner-inbox, owner-block, file-viewer, sub-cursor]
requires:
  - phase: 39
    provides: item sub-cursor (D-04) inside an expanded owner row
  - phase: 37
    provides: read-only owner file viewer and read_owner_file containment
provides:
  - check lines and `file PATH` lines of an expanded group are j/k stops in the inbox and the Owner block
  - Enter on either opens the read-only viewer at the right line, marked and in view
affects: [owner inbox, owner block, help, README]
status: complete
commits: 4
plan_head_before: 37e089197a40e6f9d5ed3f802d81b9f06f7c26c7
plan_head_after: be39d8687e8eedb09d2f9299ace7700190ad3263
actuals:
  tokens: 19255
  tasks: 3
  commits: 4
key-files:
  created: []
  modified:
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/owner_file_view.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/help.rs
    - README.md
key-decisions:
  - "Sub-cursor is one SubStop enum (Item | File) stored as two exclusive fields through a single helper pair"
  - "Positions are found by text search at key time (target_line); no parser change"
  - "o on a multi-file row now points at the selectable file lines"
completed: 2026-11-18
---

# Quick 261118-u4e: Enter opens the owner source file at the right place

Enter on a check line or a `file PATH` line of an expanded group opens the existing read-only
viewer on that file, scrolled so the target sits two rows below the top, with a `POINTER_RIGHT`
gutter marker and REVERSED+BOLD on the line. Works in the inbox and in the Phases-tab Owner block.

## Commits

| Task | Commit | What |
| ---- | ------ | ---- |
| 1 (tracer) | 0c9d5926 | `target_line` locator, `open_at`, `OwnerFileViewScreen::at_line`, inbox Enter on a check line (Space always toggles) |
| 2 | 2360e615 | `SubStop`/`sub_step`, inbox `file_cursor`, file lines selectable, Enter on a file line, `o` refusal text, two minimal settle_popup test hunks |
| 3 | 23f0669b | Owner block `file_cursor`/`row_sub_stops`, `owner_open_sub`, highlight via `sub_stop_line`, help rows, README, `item_step` removed |
| fix | be39d868 | existing detail test `sc4_...` walk counts the file line |

## Inferred decisions

All are INFERRED (from the plan's decisions; the owner gave only the quote in the plan objective).

- I-1 INFERRED: in an expanded group every drawn check line and every `file PATH` line is a j/k sub-stop, in drawn order, in both hosts. `... N more` and the inbox `run ...` line are not stops. Merged inbox rows draw `file PATH` before each part's checks, so their file stop precedes those checks.
- I-2 INFERRED: Enter on a check opens at that check; Enter on a file line opens that file at the group's first listed check (FIX: the first failure/gap entry), else the phase heading, else the top.
- I-3 INFERRED: Enter on a group row still expands/collapses; Space always toggles (also from a sub-row); the fold line is unchanged.
- I-4 INFERRED: `o v A y g x` on a sub-row keep acting on the parent row (phase 39 D-04); `o` opens at the top, `x` on a file line is the phase-level settle.
- I-5 INFERRED: target line has a 2-cell gutter (`POINTER_RIGHT` + space, two blanks on other rows), REVERSED+BOLD on its rows, target 2 rows below the top; no target found renders exactly as before.
- I-6 INFERRED: the `o` refusal on a multi-file row now reads "N files behind this row: expand it and press Enter on a file line".
- I-7 INFERRED: positions come from text search at key time on the text the viewer shows; the oracle-port parsers carry no line numbers and are untouched.
- I-8 INFERRED: footer hints unchanged (golden-tested rungs); help and README document Enter on sub-rows.

Implementation-level choices (also INFERRED):

- A target line past the end or on the "more lines not shown" notice is dropped; the viewer then opens at the top.
- `target_line` matches the phase heading on a word boundary (`phase 2` does not match `Phase 24`).
- `item_source` and `file_source` share one private `candidate_groups` iterator (merged parts, the row's group, same lane/scope/workstream/phase siblings).
- In the Owner block, a sub-stop that a refresh made vanish is dropped by `clamp` before the key runs, so Enter then acts on the row (a toggle); the `owner_open` fallback in `owner_open_sub` is defensive only.
- The Task 1 commit kept the old `o` refusal text; the new text landed with Task 2, when file lines became selectable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Existing walk tests assumed items were the last stop of a row**
- **Found during:** Tasks 1-3 test runs, and the full run
- **Issue:** the longer walk (file line as a stop) and Enter-on-check-opens broke `enter_expands_a_group_with_its_run_line`, `expansion_survives_a_refresh_and_a_vanished_group_is_dropped` (inbox), `sc4_an_expanded_group_shows_its_checks_beside_the_collapsed_rows` (detail) and the two `item_walk_*` / merged walk tests. The plan expected `enter_expands_a_group_with_its_run_line` unchanged, but its `k` then Enter lands on a check line, where Enter now opens the file.
- **Fix:** adjusted the key counts (one more `j`/`k` for the file line); the `k`-then-Enter step now walks back to the row first.
- **Files modified:** src/ui/screens/inbox.rs, src/ui/screens/detail.rs
- **Commits:** 0c9d5926, 2360e615, be39d868

No Rule 4 items. No auth gates.

## Verification

- `cargo test --no-fail-fast` (full): 69 suites, 4605 passed, 1 failed, 19 ignored. Lib suite: `3739 passed; 0 failed; 3 ignored`. The single failure is the known `owner_audit_conformance::ours_equals_the_oracle_on_every_owner_fixture` (`test result: FAILED. 2 passed; 1 failed`), pre-existing and handled elsewhere.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `git diff 37e08919 -- settle_popup.rs`: two hunks, both inside the two named tests. `help.rs`: four hunks (two rows, two test tuples).
- Acceptance greps all at their EXPECT values (new fns/fields 1, `item_step(` 0, `file line` in help 4 and README 4, no `Command::new`/`SuspendAndEdit` in either owner file).

## Known Stubs

None.

## Threat Flags

None. The only new read path is `open_at`, which goes through `read_owner_file` (T-u4e-01); nothing is spawned (T-u4e-03); the gutter marker is a registry glyph (T-u4e-04); `target_line` uses plain substring/char compares (T-u4e-02).

## Self-Check: PASSED

- Commits 0c9d5926, 2360e615, 23f0669b, be39d868 are ancestors of HEAD.
- All modified files exist; SUMMARY and PLAN are the only uncommitted paths.
