---
phase: 36-dashboard-and-detail
plan: 01
subsystem: ui
tags: [ratatui, owner-inbox, dashboard, digest]
requires:
  - phase: 34-35
    provides: read_owner_digest, OwnerDigest, evict_project
provides:
  - "src/ui/screens/owner.rs presentation core (OwnerNumbers, owner_numbers, has_owner_work, OWNER_SEP, you_cell_line)"
  - "ProjectStateLoaded.digest and Action::OwnerDigestLoaded"
  - "AppContext.owner_digests and AppContext::schedule_owner_digest"
  - "the >= 100 column dashboard tier with the You column"
affects: [36-02, 36-03, 36-04]
tech-stack:
  added: []
  patterns: ["sibling alias-keyed map beside ProjectState", "digest in the reparse blocking task", "whole-form fit ladder"]
key-files:
  created: [src/ui/screens/owner.rs]
  modified:
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/delete_confirm.rs
key-decisions:
  - "Digest lives in AppContext.owner_digests, never on ProjectState"
  - "First fill is digest-only (schedule_owner_digest), never a re-parse"
requirements-completed: [OWN-16]
status: complete
commits: 3
plan_head_before: 7bfe5d1a929119a1c43a5fab5e3d6edf8999219e
plan_head_after: ba1fa7c523cc178ab12a9b9cab840ee372d701e6
actuals:
  tokens: 40000
  tasks: 3
  commits: 3
completed: 2026-11-14
---

# Phase 36 Plan 01: Dashboard You column Summary

A quiet `You` column at 100 columns and wider shows decide/test/fix numbers from an owner digest that is computed off the render path (in the reparse task, plus a digest-only first fill at startup and registration) and stored beside, not on, the project state.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 | d79de897 | feat(36-01): show owner numbers in a You column on the dashboard |
| 2 | 4873e51a | feat(36-01): fill the owner digests at startup and after registration |
| 3 | ba1fa7c5 | test(36-01): pin the You column widths and the 15-project fleet cells |

## What was built

- `owner.rs`: pure core. `you_cell_line` takes the first whole ladder form that fits (lanes + closes + old, lanes + closes, lanes, first lane alone, `fit_cells`), every span `Color::Reset`, `·` and `(+N old)` dim, `None` is blank.
- `ProjectStateLoaded.digest` computed right after `parse_project_state` in `schedule_reparse`; `reparse_dispatches` unchanged in meaning. The handler redraws/re-sorts on a digest-only change but posts `Updated:` only on a state change; digests of unregistered aliases are not stored.
- `OwnerDigestLoaded` first fill: scheduled from main.rs after `event_tx` is set, auto-register, `CreateProjectResult` success and the add-project screen; fills only a registered alias without a digest.
- `dashboard_columns` / `render_main` gain the `>= 100` tier (20/26/Min 13/Length 21/12/9). Resolved widths equal the UI-SPEC table; Status is 13 at 100 and 120.
- Prune: `owner_digests.retain` in `prune_driver_maps`, classified in the exhaustive destructure test.

## Tests added

`you_cell_forms`, `the_cell_is_quiet_on_every_span`, `owner_numbers_maps_the_digest`, `old_debt_suffix_and_the_fit_ladder`, `an_unreadable_file_reads_as_fix` (owner.rs); `a_watcher_refresh_delivers_a_fresh_owner_digest`, `a_digest_only_change_redraws_without_an_updated_message`, three `a_first_fill_*` tests (app.rs); `the_you_column_shows_owner_numbers_at_100_columns_and_up`, `the_you_tier_resolves_the_specified_column_widths`, `below_100_columns_the_dashboard_ignores_owner_digests`, `the_you_column_shows_every_fleet_cell_at_100_120_and_140`, `sc1_...`, `sc2_...`, `the_you_cell_is_quiet_on_every_row_color`, `a_project_without_a_digest_has_a_blank_you_cell` (normal.rs). The clone test in action.rs covers `OwnerDigestLoaded`.

## Verification (raw)

- `rtk bypass cargo test --locked --no-fail-fast --all-targets`: 4186 passed, 0 failed, 19 ignored, across 64 result lines.
- `rtk bypass cargo test --lib`: 3357 passed, 0 failed, 3 ignored (after task 2).
- `rtk bypass cargo test --test async_blocking_guard`: 8 passed, 0 failed.
- `rtk bypass cargo clippy --all-targets -- -D warnings`: clean (no warnings, exit 0).
- `read_owner_digest` occurrences in normal.rs and main.rs: 0.

## Divergences from CONTEXT

D-09 (CONTEXT/UI-SPEC call unreadable owner files Test work): in the live reader (rules.rs) unreadable UAT or VERIFICATION files are counted in `fix_phases`, so a project whose only owner work is an unreadable file reads `fix 1 ph` (test `an_unreadable_file_reads_as_fix`). D-09's fail-safe holds unchanged: a project without a digest shows a blank cell, never `·`.

## Deviations from Plan

- **TDD commit split.** The plan marks Task 1 and Task 3 `tdd="true"` with a RED commit first. I wrote tests and implementation together per task and committed each task once: the tracer task changes eight files that must compile together, so a standalone RED commit could not build. Tests were written to the behavior blocks and pass; no separate failing-test commits exist.
- **Task 3 scope shift.** The `(+N old)` suffix and the fit ladder (planned for Task 3 in owner.rs) were implemented in Task 1's `you_cell_line` and covered by Task 1's tests (`old_debt_suffix_and_the_fit_ladder`). Task 3's commit holds only the normal.rs fleet and width tests.
- **Existing test edit.** `test_dashboard_columns_header_and_width_counts_match` gained widths 100 and 99 (plan-mandated); no other existing test body changed, and the three ladder tests are untouched. No test needed a You-column edit.
- **Left alone.** `cargo fmt` reformats `src/state_reader/disk_status.rs` (pre-existing, unrelated); I reverted that file each time so it is not in these commits.

## Inferred decisions

- Inferred decision: both digest handlers ignore an alias that is not registered (per plan).
- Inferred decision: ladder rung 4 (first lane alone) before `fit_cells`, per plan.
- Inferred decision: `test_phases` counts only groups with at least one item and not bookkeeping-only, current scope, unacknowledged (RESEARCH Correction 6).
- Inferred decision: `owe` test helper in mod.rs carries `#[allow(dead_code)]` because 36-02 is its first consumer.

## Known Stubs

None.

## Threat Flags

None. `read_owner_digest` runs only inside `spawn_blocking` (reparse task and `schedule_owner_digest`); `async_blocking_guard` passes.

## Human UAT (run in /gsd-verify-work)

1. Cell wording on the real fleet at 120 columns. 2. Alignment of Progress and Backlog at 100 and 140 columns with the `·` and `→` glyphs on the owner's terminal font.

## Self-Check: PASSED

Files and commits verified present (owner.rs, three commits reachable from HEAD).
