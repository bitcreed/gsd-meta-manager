---
phase: quick-261028-t3d
plan: 01
subsystem: driver
tags: [lock, flock, fifo, symlink, o_nofollow, run-directory-hardening]
requires: [quick-261028-n0z]
provides:
  - "lock::open_lock_file: the one guarded open for every lock file (O_NOFOLLOW | O_NONBLOCK, single-link regular file on the handle)"
  - "lock::ensure_lock_file: the guarded DIRECTION.lock create"
  - "state.json read through read_run_file_capped in RunStore::open, check_wall_clock_budget and stop_orphaned_scope"
affects: [src/driver/lock.rs, src/driverctl/serve.rs, src/driverctl/store.rs, src/launcher/run.rs, src/launcher/stop.rs]
key-files:
  modified:
    - src/driver/lock.rs
    - src/driverctl/serve.rs
    - src/driverctl/store.rs
    - src/launcher/run.rs
    - src/launcher/stop.rs
    - src/state_reader/driver_frontmatter.rs
    - tests/driverctl_serve.rs
status: complete
commits: 3
plan_head_before: 1afd4fdb81db221a35011a2f5187cd933e068e45
plan_head_after: 523aa7592754c90a3a2a77e6ce89d06e011bd073
actuals:
  tasks: 3
  commits: 3
---

# Quick 261028-t3d Summary

Every open of `launcher.lock`, `DIRECTION.lock` and `state.json` on the TUI, `driverctl serve`, `driver launch` and `driver stop` paths now refuses a planted FIFO without blocking and a symlink without following it. Closes 27 WR-03 and WR-04, 30 WR-11 and 31 WR-01.

## Commits

- `65233b8` fix(quick-261028-t3d): guard the run-start DIRECTION.lock and state.json opens (27 WR-03)
- `4977318` fix(quick-261028-t3d): open the run-lock probe and holder read without following links or blocking (27 WR-04, 30 WR-11, 31 WR-01)
- `523aa75` docs(quick-261028-t3d): mark 27 WR-03/WR-04, 30 WR-11, 31 WR-01 fixed and close the lock-probe todo

## RED to GREEN

Task 1 (run at HEAD 1afd4fd, before any production change):
- the three `tests/driverctl_serve.rs` tests (FIFO DIRECTION.lock, dangling-symlink DIRECTION.lock, FIFO state.json) hit `wait_exit`'s 10 s "serve did not exit" panic.
- `a_symlinked_state_json_is_refused...` and `a_state_json_over_the_cap...` opened the store instead of CorruptState; `a_fifo_state_json_is_refused_without_blocking` and `the_wall_clock_check_does_not_block_on_a_fifo_state_json` hit `within_5s`'s panic.
- The `ensure_lock_file` tests ran against a temporary stub with the old bare create: the symlink/hard-link test and the FIFO test failed; the create-and-never-truncate test passes at both states by design.

Task 2 (against the Task 1 commit with the old `probe_at` and `read_holder_at`): all 8 new tests failed: the symlink probe read Held, the hard-link probe read Free, the holder reads returned Some, and the FIFO tests (probe, holder, `read_driver_run_text`, orphan stop) hit `within_5s`'s panic.

## Final counts

Full suite from `target/t3d-full.txt`, summed over 55 `test result:` lines: 3201 passed, 0 failed, 17 ignored (baseline 3183/0/17, plus 18 new tests: lock 9, store 3, run 1, stop 1, driver_frontmatter 1, driverctl_serve 3). clippy `--locked --all-targets -- -D warnings` exits 0 on host, aarch64-apple-darwin, x86_64-apple-darwin and x86_64-pc-windows-gnu. `cargo fmt --check` exits 0.

## Deviations from Plan

None in behaviour. The three `acquire_at` refusal details are byte-identical and the four existing WR-08 tests pass unmodified. Process note: the `probe_at` / `read_holder_at` edits were written together with Task 1 and then split back out, so each commit holds only its own task's production change.

## Inferred decisions [inferred — cleanup 2026-10-28]

- The link-count check applies on the read side (`probe_at`, `read_holder_at`) and in `ensure_lock_file`, so a hard link to a foreign inode is refused there too.
- No link-count check for `state.json`: the read is read-only and `write_state` replaces the file by rename, which never writes through a link.
- `ensure_lock_file` opens read+write, because a write-only nonblocking open of a reader-less FIFO fails with ENXIO and would not name the problem.
- Two extra `state.json` reads hardened as the same by-path class: `check_wall_clock_budget` (a refused file counts as no recorded time; the store then refuses it) and `stop_orphaned_scope`.
- 30 WR-11's separate marker-scan in-flight flag is filed as `.planning/todos/pending/2026-10-28-launcher-marker-scan-in-flight-flag.md`.
- The closed lock-probe todo went to `.planning/todos/completed/` (the n0z precedent), not `done/`.

## Known Stubs

None.

## Self-Check: PASSED

Commits 65233b8, 4977318 and 523aa75 are ancestors of HEAD; the three REVIEW-DISPOSITION files show `open:` 14, 6, 6 matching their `disposition: open` counts; the closed todo is in completed/ with `completed` and `resolved_by`; the new pending todo exists.
