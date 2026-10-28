---
phase: quick-261028-n0z
plan: 01
status: complete
commits: 4
plan_head_before: 3eb4920
plan_head_after: be597ed
---

# Quick 261028-n0z: Phase 30 WR-08 / WR-09 / WR-10 Summary

Closed three deferred Phase 30 review warnings in the DIRECTION.md write path: a give-back ENOENT now restores the newest save by a no-replace rename, the lock files open without following links, and the splice reads and writes are capped at 1 MiB.

## Commits

| Finding | Sha | What |
|---------|-----|------|
| WR-10 | 44eef32 | give_back_newest ENOENT arm: `renameat_with(NOREPLACE)`; EEXIST runs another exchange round; race_hook Points BeforeGiveBack/BeforeRestore |
| WR-08 | 670e099 | acquire_at: O_NOFOLLOW + O_NONBLOCK, refuse non-regular or nlink != 1 before flock (DIRECTION.lock and launcher.lock) |
| WR-09 | 0f85617 | read_run_file capped via handle size check plus take(CAP+1); run_loop write-side cap |
| docs | be597ed | WR-08/09/10 marked fixed, two todos moved to completed/, new pending todo for probe_at/read_holder_at |

## Tests

- `cargo test --no-fail-fast --lib driver::` 164 passed, 0 failed (154 + 10 new: 2 WR-10, 4 lock + 1 steer WR-08, 2 direction + 1 steer WR-09).
- Integration: driverctl_race 4 passed + 1 ignored, driverctl_lock_race 5, driverctl_liveness 11 (unchanged).
- Full suite: 3170 passed, 0 failed, 17 ignored (plan baseline 3168; siblings n0v/n0x added 2 tests).
- clippy `--all-targets -D warnings` exit 0 on Linux, aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu. rustfmt clean on the three touched files.

## RED to GREEN evidence

- WR-08: the 4 lock tests and the steer test failed before the fix (lock followed the symlink / failed from set_len with EINVAL on the FIFO), pass after.
- WR-09: the steer over-cap test failed before the fix; the two direction tests were written together with the fix (see deviations).
- WR-10: both tests were written together with the fix. Verified by mutation instead (below).

## WR-10 mutation check (reverted, never committed)

- `RenameFlags::NOREPLACE` -> `RenameFlags::empty()`: `a_file_created_before_the_no_replace_restore_is_kept_and_wins` FAILED (21 passed, 1 failed), as required.
- Disabling the ENOENT arm (`&& false`): both WR-10 tests FAILED (20 passed, 2 failed).
- Restored; 22 passed in `driver::direction`.

## Deviations from Plan

- WR-10 and WR-09 direction.rs tests were written in the same edit as their fixes, not strictly RED-first; the mutations above (and the pre-fix failing steer test) stand in as the failing-without-fix evidence.
- Task 1 commit was amended once (rustfmt wrapping) before any later work; sha 44eef32 is the final one.

## Inferred decisions [inferred — cleanup 2026-10-28]

- Link-count check (`nlink() != 1`) in acquire_at: a hard link is a regular file, so only the count stops `set_len(0)` truncating through it.
- Write-side cap in run_loop: required so the tool's own over-cap file cannot come back from the exchange-back as `bytes: None` and be given back as if it were a human save.
- Todos moved to `.planning/todos/completed/` rather than `done/` (the GSD-canonical directory).

## Known Stubs

None.

## Self-Check: PASSED

Commits 44eef32, 670e099, 0f85617, be597ed exist on master; IN-05..IN-09 disposition rows unchanged.
