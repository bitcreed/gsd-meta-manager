---
phase: 37-inbox-screen-and-actions
plan: 02
subsystem: owner-items reader, Owner block
tags: [typed-command, run-command, golden-pin]
requires: ["37-01"]
provides:
  - RunVerb, RunCommand (+ Display), OwnerGroup.run
  - owner_block_output_is_pinned with three goldens
affects: [37-03]
key-files:
  created:
    - tests/fixtures/owner_golden/owner_block_120x30.txt
    - tests/fixtures/owner_golden/owner_block_120x30_open.txt
    - tests/fixtures/owner_golden/owner_block_80x24.txt
  modified:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/gate.rs
    - src/inbox.rs
    - src/ui/screens/owner.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/render_escape_guard.rs
    - tests/owner_digest.rs
key-decisions:
  - "D-11: command is always run's Display; the slash commands are spelled only in RunCommand's Display"
  - "Pin goldens written once from the pre-change render in their own commit"
requirements-completed: [OWN-21]
duration: 25 min
completed: 2026-11-14
status: complete
commits: 3
plan_head_before: 47fa42af429fc256649664d3a820b2970f6b909f
plan_head_after: aa6d58668313b5bf571a9f4426303fc4835caaa4
actuals:
  tokens: 14000
  tasks: 2
  commits: 3
---

# Phase 37 Plan 02: Typed owner command and Owner block pin Summary

Every owner group now carries `run: Option<RunCommand>` and its `command` string is that value's `Display`; the Owner block render is pinned byte for byte by three goldens written before the change.

## Accomplishments

- Pin (38e66683): `owner_block_output_is_pinned` renders `sample_owner_digest()` at 120x30 collapsed, 120x30 with the first TEST group expanded, Old debt open and the cursor on stop 0, and 80x24 collapsed. Goldens are read at run time and were written once from the pre-change render.
- Typed command (ff276c38): `RunVerb`, `RunCommand` and `Display` in mod.rs; `run_for`, `resume_run`, `debug_run` replace the string builders in rules.rs; the late Fix reassignment sets `run` and `command` from one builder call; every other `OwnerGroup` literal got `run`. No `format!("/gsd-...")` remains in rules.rs.
- Tests (aa6d5866): `run_for_keeps_the_command_rules` (including `1;rm`, fullwidth digits, `a b`, `../x`), `resume_run_names_only_safe_workstreams`, `the_fix_reassignment_sets_run_and_command_together`, a `run`/`command` agreement helper on six existing command tests, `run_command_display_spells_every_shape`, and the fixture-wide `run_command_matches_the_command_string_in_every_fixture` (also asserts all four verbs occur).

## Deviations from Plan

None in behavior. Process notes:
- [Inferred] The 120x30 pin renders into `block_height(model, 30)` rows (cap 12), so in the "open" golden the body is clipped and the Old debt rows are below the window; the open fold line and the cursor/expansion are pinned. The 80x24 state is clipped the same way. Matches how the block is drawn in a tab of that size.
- Task 2's Display rows were already covered by Task 1's `run_command_display_spells_every_shape`; no second edit to mod.rs.
- The pin's golden writer was a temporary ignored test, removed before the commit; no golden has been rewritten (`git rev-list --count` over the three files prints 1).

**Total deviations:** 0 auto-fixed.

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4290 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings`: clean; also clean for `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- Owner text goldens (`inbox.txt`, `inbox-include-all.txt`), inbox_cli and status_cli unchanged and passing.

## Known Stubs

None.

## Self-Check: PASSED

Created goldens exist; commits 38e66683, ff276c38, aa6d5866 are ancestors of HEAD.
