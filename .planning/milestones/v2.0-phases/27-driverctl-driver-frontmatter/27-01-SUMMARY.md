---
phase: 27-driverctl-driver-frontmatter
plan: 01
subsystem: driver
tags: [flock, renameat2, rename-exchange, rustix, race-test, direction-md]

requires:
  - phase: 17-driver
    provides: "src/driver/lock.rs per-project flock primitive (acquire/read_holder)"
provides:
  - "lock.rs: acquire_at, acquire_at_with_retry, probe_at/LockProbe, run_lock_path, read_holder_at, RUN_LOCK_FILE, ACQUIRE_RETRY_BUDGET"
  - "driver::direction: split_frontmatter, body_sane, parse_direction, direction_lock, splice_frontmatter, write_body, sweep_stale_temps"
  - "tests/driverctl_lock_race.rs (D-20 race tests 1-5) and tests/driverctl_race.rs (editor-race port)"
affects: [27-02, 27-04, 27-06, phase-28, phase-30]

actuals:
  tokens: 18000
  tasks: 3
  commits: 3
plan_head_before: 1866e0761570f50563adf5822ffa566a4275b868
plan_head_after: df175f0cf12dc70e5be172425288b4e0525ba4bd

tech-stack:
  added: []
  patterns:
    - "exchange-verify: renameat2(RENAME_EXCHANGE) of an fsynced temp, verify displaced BODY BYTES, swap back and retry on mismatch"
    - "bounded-retry non-blocking flock acquire; LOCK_SH read-only probe"

key-files:
  created:
    - src/driver/direction.rs
    - tests/driverctl_lock_race.rs
    - tests/driverctl_race.rs
  modified:
    - src/driver/lock.rs
    - src/driver/mod.rs

key-decisions:
  - "DIRECTION.md primitives live at src/driver/direction.rs (beside lock.rs), not src/driverctl/ [inferred - audit, from the plan]"
  - "ACQUIRE_RETRY_BUDGET = 500 ms [inferred - audit, from the plan]"

requirements-completed: [DCTL-02, DCTL-03]

duration: 40min
completed: 2026-10-19
status: complete
---

# Phase 27 Plan 01: DIRECTION.md splice and run-lock primitives Summary

**Exchange-verify DIRECTION.md frontmatter splice (renameat2 RENAME_EXCHANGE, body-byte verify, swap-back) with a shared DIRECTION.lock and `write_body`, plus the LOCK_SH probe / bounded-retry acquire primitives, proven by a ported editor race (0 lost rename-saves in 15 s) and the D-20 probe/acquire race tests.**

## Accomplishments

- `src/driver/lock.rs`: `acquire` is now a wrapper over `acquire_at`; added `acquire_at_with_retry` (1 ms doubling backoff, 50 ms cap, 500 ms budget), `probe_at` / `LockProbe {Held, Free, Absent, Unknown(errno)}` (O_RDONLY, LOCK_SH|LOCK_NB, never creates), `read_holder_at`, `run_lock_path`. Header gained facts 6 and 7. The per-project `run.lock` behaviour is unchanged (`tests/driver_lock.rs`: 8 pass).
- `src/driver/direction.rs` (new): frontmatter split (LF/CRLF, marker-gated), `body_sane`, `parse_direction`, `direction_lock` (2 s), `splice_frontmatter`, `write_body`, `sweep_stale_temps`. Linux exchange functions are cfg-gated with `UnsupportedPlatform` twins elsewhere.
- Tests: 15 unit tests in `direction.rs`, 3 new in `lock.rs`, 5 in `tests/driverctl_lock_race.rs`, 4 + 1 ignored in `tests/driverctl_race.rs`.

## Editor race, full run (`editor_race_full`, 15 s per style)

```
exchange rename  saves= 11886 regens=  55195 LOST=    0 transient=   4 final_body_sane=true tool={"ok": 55195}
exchange inplace saves= 13555 regens=  74130 LOST=    0 transient=   3 final_body_sane=true tool={"ok": 73508, "skipped-insane-body": 622}
in-place: lost=0 allowed(poisson_q99 of 7.40e-5 x 13555 saves)=4
```

Positive control (naive read-once-rename, 0.5 s): `naive rename saves= 15 regens= 22224 LOST= 10`.

## Test results

- `cargo test --no-fail-fast`: 2124 lib passed, 1 failed (the expected git-version constants test in `envelope/policy.rs`); every integration target passes, including `driverctl_lock_race` (5), `driverctl_race` (4, 1 ignored), `driver_lock` (8), `async_blocking_guard` (9).
- `cargo clippy --all-targets -- -D warnings`: clean.

## Deviations from Plan

None - plan executed as written. TDD note: the two `tdd="true"` tasks exercise library code landed in Task 1 (plan-structured: Task 1 is the tracer carrying the implementation), so there is no separate RED commit; the negative/positive controls in the tests (single-attempt refusal, no-retry refusals > 0, naive LOST > 0) supply the non-vacuity evidence a RED step would.

## Inferred decisions (audit)

- Non-UTF-8 reads of DIRECTION.md inside the exchange loop are treated as a retryable half-written save (file untouched); a persistently non-UTF-8 file ends as `GaveUp` rather than an error.
- A failed exchange-back leaves the displaced file at the temp name and returns `Io` naming its path (never unlinks it, so human bytes cannot be destroyed; `sweep_stale_temps` is age-gated).
- `ENOENT` from the exchange (DIRECTION.md vanished mid-splice) maps to `NoDirection` after removing our temp.
- Temp is created with mode 0600 then chmod'd to the original mode before fsync, so content is never briefly world-readable.
- `two_writers...` test inserts a 100 us sleep between splices so the other writer's lock retry can land, and asserts `lock_timed_out == false`.
- `real_holder_is_still_refused_within_budget` additionally asserts elapsed >= budget (a real holder is refused only after the budget is spent).

## Notes for later phases

- Phase 30 (D-13): steering edits MUST call `driver::direction::write_body`; its DIRECTION watcher should ignore `.DIRECTION.md.fm.*` temps (Pitfall 9).
- 27-04 consumes `lock::probe_at`; the Phase 28 bwrap mask hand-off for DIRECTION.lock (T-27-02) is recorded in 27-06.

## Known Stubs

None.

## Threat Flags

None - no new network, auth or schema surface beyond the plan's threat model.

## Self-Check: PASSED

- FOUND: src/driver/direction.rs, tests/driverctl_lock_race.rs, tests/driverctl_race.rs
- FOUND commits: 60db20e, 095399c, df175f0 (ancestors of HEAD)
- `commits: 3` counts the three task commits measured from the ledger base; the SUMMARY docs commit follows it.
