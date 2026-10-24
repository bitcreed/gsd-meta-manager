---
phase: quick-261023-v1s
plan: 01
subsystem: driver/direction (splice), driverctl client, race tests
tags: [flaky-test, race, exchange-verify, fsync, container, stress]
status: complete
requires:
  - quick-261023-qs8 stress harness and evidence
provides:
  - deterministic editor_race_smoke (count handshake + contention witness)
  - newest-wins give-back by RENAME_EXCHANGE (no plain rename) in the splice
  - driverctl client relays a reply queued before an EPIPE
  - SIGKILL lock-release test that kills only after the agent fork has exec'd
affects:
  - gate 4 of scripts/pre-tag-check.sh
tech-stack:
  added: []
  patterns:
    - cfg(test) per-thread race hook for deterministic splice-race unit tests
    - count-based yield handshake instead of timing hopes in race tests
key-files:
  created:
    - .planning/quick/261023-v1s-fix-flaky-editor-race-smoke-test-in-driv/stress-editor-race.sh
    - .planning/quick/261023-v1s-fix-flaky-editor-race-smoke-test-in-driv/stress-results.txt
    - .planning/quick/261023-v1s-fix-flaky-editor-race-smoke-test-in-driv/261023-v1s-EVIDENCE.md
  modified:
    - src/driver/direction.rs
    - tests/driverctl_race.rs
    - src/driverctl_cli.rs
    - tests/driver_lock.rs
decisions:
  - "Product verdict YES: the newest-wins plain rename (direction.rs:688-693 at base) could revert a rename-save; fixed by exchange-and-verify loop, red-first unit test"
  - "The fsync-window starvation is designed GaveUp behaviour; fixed test-side with a count handshake, all assertions/constants kept"
  - "Scan found two unflagged flaky tests; fixed S1 in the product client, S2 test-side (a retry variant was rejected by a CLOEXEC mutant)"
  - "Release gate blocked 5/6 by git-version drift in the rebuilt image (2.56.0 vs constant 2.55.0); constants NOT re-derived here"
metrics:
  duration: "~5h (2026-10-24 03:30Z-08:35Z)"
  completed: 2026-10-24
actuals:
  tokens: 24200
  tasks: 3
  commits: 6
plan_head_before: 4af7366a8362d7e4a97f676eb7f5429ee7fc9bbc
plan_head_after: 58acaa2bb12ea1d99599369bc5249bfae7c27b31
---

# Phase quick-261023-v1s Plan 01: Fix flaky editor_race_smoke Summary

The smoke flaked for two reasons. Most failures (6/7) were splice starvation: on overlay the fsync makes each attempt's window about 5 ms, while the test's human saves every 1.3 ms, so in a 1 s case sometimes no splice lands. That is a timing assumption in the test, and it is fixed with a count-based yield handshake. One failure in seven was a real product race: the newest-wins branch put a newer save back with a plain rename, which could revert a still newer rename-save. That is fixed with an exchange-and-verify loop and a unit test that fails on the old code. The scan of sibling tests found two more flaky tests, and both are fixed.

## Before / after (container `gsdmm-pre-tag-check`, binary mode, `editor_race_smoke`)

| profile | mode   | n   | target fails before -> after | other before/after | timeouts |
|---------|--------|-----|------------------------------|--------------------|----------|
| 0       | binary | 200 | 1 -> 0                       | 0 / 0              | 0 / 0    |
| A       | binary | 200 | 1 -> 0                       | 0 / 0              | 0 / 0    |
| B       | binary | 200 | 5 -> 0                       | 0 / 0              | 0 / 0    |

Every after line carries tree suffix `31d595e20190`, the diff of fix commit 717d724. The BEFORE total was 7, which is at least 6, so no escalation was needed.

## Root cause

- **F1, starvation (6/7, a test timing assumption).** Each attempt's read->exchange window contains `sync_all`, which D-23 requires. The window measured 4.95-5.0 ms p50 on overlay, 94 % of it fsync. The human's inter-save interval is 1.25-1.4 ms, so 87-88 % of attempts overlapped a save and a case got only about 20 splices. On tmpfs the window is 33 us. Interventions:
  - Removing the fsync on overlay raised oks about 100x (p50 6-9 -> 841-954) and gave-up dropped to 0.
  - A 5 ms sleep inside the window on tmpfs brought oks down to overlay levels and gave 4/40 cases with ok==0.
  - The rename/in-place asymmetry from the task note was not reproduced: both styles starve. The B baseline starved rename 3 times and in-place twice.
- **F2, lost rename-save (1/7, product).** Module fact 3 handled a newer save with `std::fs::rename(temp, DIRECTION.md)`. That is a read-then-rename, the bug that fact 2 forbids. A third save landing between the exchange back and that rename was reverted. The `#[cfg(test)]` hook test reproduced it deterministically on the base code.
- **Product verdict: yes** (F2). False mismatches were 0 across 5 615 overlay and 203 616 tmpfs attempts, and lock timeouts were 0. That rules out H2 and H3.
- **Alternatives considered for F1, all rejected:**
  - Moving the fsync after the exchange violates D-23.
  - More tries or backoff only shift the ratio.
  - A re-read before the exchange leaves the fsync inside the window.

## Fix

- **src/driver/direction.rs:**
  - `give_back_newest` replaces the plain rename. It puts the newer file in place by `RENAME_EXCHANGE` and repeats while what comes back is not the expected older file unchanged (same inode and same bytes).
  - It errors after 8 rounds and never unlinks a human file.
  - D-23, SPLICE_TRIES=5, the backoff, the 2 s lock budget and `lock_timed_out` are all unchanged.
  - Three unit tests use a `cfg(test)` race hook. The newest-save one is red on the old code and green on the fix.
- **tests/driverctl_race.rs:**
  - Count-based yield: until the tool's first ok, the human yields one splice that began after the yield. A final yield runs at the deadline, bounded by `wait_until`, with drop-guard exit flags.
  - New witness: `contended > 0`, with the message "no save landed while a splice was in flight".
  - All original assertions and messages are kept, as are the 1.0 s / 15.0 s run lengths, the Poisson bound and the positive control.
  - Measured in the container: at most 1 yield per case, 542-742 contended saves per case, and the naive control never yields.
- **Mutants:** M-B (no splice lands) failed the smoke 3/3, and M-C (no give-back) failed it 3/3.

## Scan

- **Flagged and measured, all clean:**
  - `driverctl_lock_race`: 0/200
  - `driverctl_serve` gap_scan_tick: 0/200
  - `driver_kill_startup`: 0/200
  - `driver_lock` duplicate_start: 0/200
  - `executor_lifecycle` keeps_emitting: 0/100
  - `editor_race_full --ignored`: 0/5
- **S1, `driverctl_serve::a_flood_of_idle_connections_...`: 4 failures in 600 runs before, 0/400 after (product fix, 801c656).** `refuse_busy` closes without reading the request. The client's send failed with EPIPE and threw away the `busy` reply that was already queued. The client now reads after a BrokenPipe/ConnectionReset and relays a complete reply. A socketpair unit test fails on the old code with the exact field error.
- **S2, `driver_lock::the_lock_is_released_when_the_holding_process_dies`: 7 failures in 1400 runs before, 0/1400 after (test-side fix, d960678).**
  - A /proc probe found the holder: a forked child of a tokio worker thread that had not yet exec'd. The lock freed 77 us to 3.9 ms after the reap, so flock (D-19) is not defective.
  - The test now waits for the journal's `exec_started` record before the SIGKILL. The single acquire is unchanged.
  - A 500 ms retry variant was rejected: with mutant M-S2 (lock fd CLOEXEC cleared), the retry version passed 3/3 while the original test failed 3/3. The shipped fix fails M-S2 3/3.
- No open findings. The full table is in EVIDENCE under "Scan of race/timing tests".

## Gates

- **`./scripts/pre-tag-check.sh --container v1.15.0`: 5/6. BLOCKER outside this task.** Gate 4's only failure (out of 89 test results, `--no-fail-fast`) is the git-version witness. The run rebuilt the image, and `ppa:git-core/ppa` now ships git 2.56.0 while the constant is 2.55.0, so the container banner read MISMATCH. Fixing it means re-deriving `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION`, with the runner's git as the authority. That is an operator/release decision and was not done here.
- **Local:** the full suite fails only the witness, clippy reports 0 warnings, and fmt is clean.
- **Integrity:** tag v1.15.0 is still at 08fc9f7, nothing was pushed (master is ahead of origin by 16), and there are no stray worktrees, branches or containers.

## Deviations from Plan

1. **[Rule 1 - Bug] Product fix in src/driver/direction.rs (F2).** Found in Task 1 from before-A iteration 75. Under the plan's product branch the test-side fix was also applied, because the product fix alone left the F1 margin: ok min/p10 was 1 with the product fix only. Commit 717d724.
2. **[Rule 1 - Bug, scan] src/driverctl_cli.rs (S1).** Commit 801c656.
3. **[Rule 1 - test timing, scan] tests/driver_lock.rs (S2).** Commit d960678.
4. **Commit counting.** A concurrent session committed docs-only `.planning/phases/28-*` and `30-*` changes to master throughout. There were no src/tests/Cargo changes, verified with `git diff --stat 08fc9f7 HEAD -- src tests Cargo.*`. As a result `git rev-list --count plan_head_before..HEAD` is 10, but only 6 of those commits are this task's (`--grep=quick-261023-v1s`). The `commits: 6` above is that measured count.
5. **Branch guard.** Commits went to `master`, the protected default name, following the orchestrator's sequential-on-master instruction, as qs8 did.

## Inferred decisions `[INFERRED — audit]`

- Profile set, n and the escalation threshold were taken from the plan (7 >= 6, so no escalation).
- The test-side yield was added alongside the product fix. The evidence: M3 shows the starvation follows the fsync window, and the product-fix-only probe still had ok p10 = 1.
- Wording and placement of the contention witness.
- The load caveat for A/B after the fix: at most 1 yield per case, under 5 % load difference.
- `executor_lifecycle` scan n was reduced to the 100 floor (about 25 s per binary run).
- The S2 exact-B BEFORE line is the probe run. Its acquire and failure check are the original code.
- The S2 fork window is treated as inherent to flock plus fork/exec. Switching to POSIX record locks would be architectural (Rule 4), so it is noted, not done.
- The git constants were not re-derived and the Dockerfile was not changed, so the gate blocker is reported instead.

## Notes-vs-reality corrections

- The task and qs8 cite `tests/driverctl_race.rs:348`. At 08fc9f7 the `ok > 0` assertion is at :355; line 348 is the leftover-temps message.
- qs8 says the in-place case has a "0.5 s window". The smoke actually runs 1.0 s per case; 0.5 s is the positive control's per-attempt length.

## Known Stubs

None.

## Self-Check: PASSED

- Files: the harness, stress-results.txt, EVIDENCE, src/driver/direction.rs, tests/driverctl_race.rs, src/driverctl_cli.rs and tests/driver_lock.rs all exist.
- Commits aad9f95, 717d724, 3222d97, 801c656, d960678 and 58acaa2 are all ancestors of HEAD.
- Task 3's automated verify cannot pass as written, because it requires `pretag_container: PASS 6/6` and the gate blocker above prevents that. EVIDENCE records the honest `FAIL 5/6` line instead.
