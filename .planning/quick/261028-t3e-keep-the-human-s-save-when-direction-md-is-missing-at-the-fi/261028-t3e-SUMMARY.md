---
phase: quick-261028-t3e
plan: 01
subsystem: driver
tags: [direction-md, splice, rename-exchange, d-12, d-17]
status: complete
requirements: [QUICK-261028-t3e]
key-files:
  modified:
    - src/driver/direction.rs
    - tests/driverctl_store.rs
    - .planning/phases/30-driver-tab-on-direction-md/30-REVIEW-DISPOSITION.md
    - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-REVIEW-DISPOSITION.md
actuals:
  tasks: 3
  commits: 3
plan_head_before: 523aa7592754c90a3a2a77e6ce89d06e011bd073
plan_head_after: b407eb781fd8744bef14dca295129979ae8855a3
completed: 2026-10-28
---

# Quick 261028-t3e: keep the human's save when DIRECTION.md is missing at the exchange back

One shared `exchange_or_restore` helper now serves both `run_loop`'s exchange back and `give_back_newest`. When DIRECTION.md is absent it restores the raced human save by a no-replace rename, exchanges again on EEXIST, and `run_loop` retries the splice on the restored save (30 WR-12 / 31 WR-03).

## Commits

- 7f4fc49 fix: restore a raced save by no-replace rename when the exchange back finds DIRECTION.md absent
- b42e57c test: pin that a crashed run with DIRECTION.md absent is still refused dispatch
- b407eb7 docs: mark 30 WR-12 and 31 WR-03 fixed

## Tests

- `driver::direction`: 26 -> 28 passed (two new race-hook tests, both failed before the fix with "could not exchange back").
- `driverctl_store`: 60 -> 61 passed (guard test passes before and after the fix).
- Full `cargo test --no-fail-fast`: 3204 passed, 0 failed, 17 ignored.
- clippy `--all-targets -D warnings`: exit 0 on host, aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu. `cargo fmt --check`: exit 0.

## Mutation checks (never committed)

- `RenameFlags::NOREPLACE` -> `RenameFlags::empty()`: `exchange_back_enoent_keeps_a_file_created_before_the_restore` failed, and so did the WR-10 test `a_file_created_before_the_no_replace_restore_is_kept_and_wins`. Reverted.
- Removing `direction.items.is_empty() ||` from `blocking_recovery`: the guard test failed. Reverted, `src/driverctl/store.rs` has no diff.

## Inferred decisions [inferred — cleanup 2026-10-28]

- I-1: after a restore, `run_loop` continues the attempt loop instead of returning `GaveUp`.
- I-2: the helper handles EEXIST itself, bounded by `GIVE_BACK_ROUNDS`; `BeforeGiveBack` now fires once per outer round.
- I-3: `blocking_recovery` and 28-REVIEW-DISPOSITION are untouched; WR-04 stays deferred.

## Deviations from Plan

None in behaviour. The disposition `open:` counts were 6 in both files at execution time (t3d had landed), so each went 6 -> 5 rather than 7 -> 6. The commits are on master as directed, not a per-agent branch.

## Self-Check: PASSED

Commits 7f4fc49, b42e57c and b407eb7 exist on HEAD; the working tree holds only the coordinator's untracked and ledger files.
