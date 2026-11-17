---
phase: 38-inbox-digest-and-triage-hygiene
plan: 04
subsystem: tui-inbox
tags: [rust, ratatui, inbox, triage, stale-fold, labels]
requires:
  - phase: 38-01
    provides: triage_entries, entry_checklist, checks_cell, stale_fold_text, label_counts
provides:
  - TUI inbox rows from inbox_sources (same-phase merge, Stale fold)
  - merged-row keys (A refused, g, v, y)
  - aligned checks column (inbox and Owner block), DIM extras and label suffixes
  - help rows and escape-guard coverage
affects: [38-03]
status: complete
commits: 2
actuals:
  tokens: 100000
  tasks: 2
  commits: 2
key-files:
  modified:
    - src/ui/screens/owner_block.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - src/inbox.rs
    - src/app.rs
    - tests/fixtures/owner_golden/owner_block_120x30.txt
    - tests/fixtures/owner_golden/owner_block_120x30_open.txt
    - tests/fixtures/owner_golden/owner_block_80x24.txt
requirements-completed: [OWN-26, OWN-27, OWN-28]
---

# Phase 38 Plan 04: TUI inbox triage Summary

The TUI inbox (`i`) now reads the same triage model as the CLI. A same-phase UAT and VERIFICATION pair is one
row (`3 checks`, `2 files`); phases older than `preferences.owner.fold_days` leave the lanes for a Stale fold
after Old debt (widest rung is `inbox::stale_fold_text`); the checks column aligns its digits; labels show DIM.

## Tasks

1. `inbox_sources`, `Fold::Stale`, merged-row keys (commit 4c03b83b)
2. Checks alignment, DIM extras and labels, help, escape guard, goldens (commit b1fd4f30)

## Verification

- `cargo test` all suites green (lib 3626 tests); `cargo clippy --all-targets -- -D warnings` clean.
- `git diff --numstat` of the three Owner block goldens against 5f855cb8: `1 1` each (the `quota-clip` row,
  `1 check` one cell left), edited by hand.

## Deviations from Plan

- `src/app.rs` (outside the plan's file list): `obs_app` pins `fold_days` to 3650. Three `sc3_*` tests use
  fixtures with fixed dates that age past 90 days as the clock advances, so they would have moved into the
  Stale fold (latent clock dependence, fixed).
- `src/inbox.rs`: dropped the `cfg_attr(not(test), allow(dead_code))` on `entry_line` as planned.
- Inbox BLOCKED aggregation takes the parts of merged blocked entries (UAT+VERIFICATION in the Blocked lane
  merge in `triage_entries`), so a BLOCKED row still lists every file as at 37.
- `help` test strips `old debt, stale, acknowledged and triage` before the forbidden-word check.

## Inferred decisions

- `A` refused on a merged row; `GroupKey::entry` (kind `entry`) for merged rows; `y` on BLOCKED, Old debt and
  Acknowledged rows keeps the 37 path; `y` on the Stale fold refuses; Stale fold rows carry no sub-header;
  `inbox_fleet_ctx` runs at fold_days 365.
- Extras draw after the phrase with a 2-cell gap (none when the phrase is empty).

## Known Stubs

None.

## Self-Check: PASSED
