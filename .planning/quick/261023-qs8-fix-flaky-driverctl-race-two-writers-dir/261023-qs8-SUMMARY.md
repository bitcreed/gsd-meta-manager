---
phase: quick-261023-qs8
plan: 01
subsystem: tests/driver-direction-lock
status: complete
tags: [flaky-test, release-gate, direction-lock, container, stress]
requires: [scripts/pre-tag-check.sh container mode, gsdmm-pre-tag-check image + cache volumes]
provides: [deterministic two_writers_under_direction_lock_lose_nothing, in-container stress harness]
affects: [pre-tag-check gate 4]
tech-stack:
  added: []
  patterns: [bidirectional acquisition handshake instead of fixed timing gaps, tree-hashed stress tallies]
key-files:
  created:
    - .planning/quick/261023-qs8-fix-flaky-driverctl-race-two-writers-dir/stress-two-writers.sh
    - .planning/quick/261023-qs8-fix-flaky-driverctl-race-two-writers-dir/stress-results.txt
    - .planning/quick/261023-qs8-fix-flaky-driverctl-race-two-writers-dir/261023-qs8-EVIDENCE.md
  modified:
    - tests/driverctl_race.rs
decisions:
  - "Root cause is test-side (fixed splicer gap vs a non-fair polling lock, with an fsync-dominated hold); src/ left byte-identical"
  - "The handshake is bidirectional, not the plan's one-directional one, because measurement showed one-directional yielding starves the splicer instead [INFERRED — audit]"
  - "editor_race_smoke flakes (the 'other' column) recorded as a separate finding, not fixed here"
metrics:
  duration: 91min
  completed: 2026-10-24
estimate:
  tokens: 95000
  tasks: 3
actuals:
  tokens: 8300
  tasks: 3
  commits: 3
plan_head_before: a84c7c0b3a2f4e432024b78f3dfb69ae4cde3865
plan_head_after: 5ee12e10746b0aad56c48809c2bb76b259120193
---

# Phase quick-261023-qs8 Plan 01: Fix flaky two-writers DIRECTION.lock test Summary

The flaky test's splicer took DIRECTION.lock back about 160 µs after each release. The lock's retry is a non-fair poll, and in the container each hold costs about 3 ms of fsync on overlay `/tmp`, so the splicer held the lock about 95 % of the time and the writer's 2 s of polls all missed. The fix replaces the fixed gap with a handshake that runs in both directions. The rate went from 22/200, 19/200 and 8/200 to 0/200 on every profile. There is no product race, and src/ is unchanged.

## Before / after (in-container, `gsdmm-pre-tag-check:latest`, cpuset 0-3, overlay /tmp, binary mode)

| profile | mode   | n   | before target fails | after target fails | other (before / after) | timeouts (before / after) |
|---------|--------|-----|---------------------|--------------------|------------------------|---------------------------|
| 0 (gate-faithful) | binary | 200 | 22 | 0 | 1 / 2  | 0 / 0 |
| A (4 spin + 1 fsync loop) | binary | 200 | 19 | 0 | 11 / 4 | 0 / 0 |
| B (8 spin + 2 fsync loops) | binary | 200 | 8  | 0 | 5 / 5  | 0 / 0 |

Before tree `a84c7c0+e3b0c44298fc` (empty diff). After tree `9e25c27+e748692becb1`, which is exactly fix commit ffb2914; the hash was recomputed before the commit. Reproduced on the first profile, so no escalation was needed.

## Root cause (evidence: 261023-qs8-EVIDENCE.md, M1-M2)

The failure needs a code condition and an environment condition together:
- **Code:** the test's splicer re-took the lock after a fixed `sleep(100µs)`. `acquire_at_with_retry` (src/driver/lock.rs:317-341) polls with a sleep that starts at 1 ms and doubles to 50 ms, about 46 polls in 2 s, with no queue or fairness, so the writer won only if a poll landed in that gap.
- **Environment:** the hold is dominated by `sync_all` inside the lock (direction.rs:597). It is about 3 ms (max 27 ms) on overlay in the container and about 60 µs on tmpfs.
- **Measured:** during each timed-out write the splicer completed 259-537 splices and held the lock 94-97 % of the time. The longest single hold was 26.8 ms, so H2 (one long stall) is falsified. On tmpfs, with the same binary, load and n, there were 0/40 failures.
- **Why write 1:** 41 of the 49 baseline failures are at write 1. That is the only write that always starts while the splicer is in its tight loop rather than in backoff. Once one side holds the lock it tends to keep it.

## Product-race verdict

No product race (EVIDENCE M3-M5):
- A temporary holder-count probe in `locked()` saw max_holders=1 in 86/86 iterations, including every timeout.
- The guard spans the whole body (direction.rs:570-577), every acquisition attempt opens a fresh descriptor (lock.rs:201-211), and `RunLock` has no `impl Drop`, so closing the descriptor is the only release path (lock.rs:139-154).
- Production splices once per event, with heartbeats coalesced to at least 120 s (store.rs:72, 790-806). No production path loops on the lock.
- Follow-up (out of scope): the poll schedule gives a future tight-loop writer no fairness. Today's fallback, which proceeds without the lock and logs `direction-lock-timeout`, covers it.

## Fix

Test-side, in tests/driverctl_race.rs only:
- **`requested`/`acquired` handshake:** the splicer starts no new splice while the writer is waiting for the lock. Once the writer's edit closure has run, which happens only under the lock, the splicer splices at once, so it contends while the writer holds the lock.
- **`begun`/`done` handshake:** before each write the writer waits until every splice already begun has finished and a new one has begun. The splicer is never starved, and the next write still meets a busy lock.
- **Bounded waits:** every wait gives up after 2 × `DIRECTION_LOCK_BUDGET`, so a broken lock fails an assertion instead of hanging the test.
- **Contention witness:** a new `contended > 0` assertion checks that writes still meet a busy lock.

Every original assertion is kept. The lock budget and poll schedule are unchanged.

Measured with the fix in the container: the longest single write was 42 ms, the longest single splice 39 ms, there were 0 splicer lock timeouts, and 200/200 writes were contended.

## Mutant check

Mutant M-A makes `locked()` never take the lock and report `lock_timed_out`. The fixed test failed 3/3 at tests/driverctl_race.rs:488 ("write 1: the shared lock must be obtainable"). The mutant is reverted and src/ is clean.

## Local gates

- `cargo test --test driverctl_race --test driverctl_lock_race`: pass.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `cargo test --no-fail-fast`: 89 suites; the only failure is the known `envelope::policy` git-version witness.

## Deviations from Plan

**1. [Rule 1 - Bug in the prescribed fix] The handshake is bidirectional, not one-directional [INFERRED — audit]**
- **Found during:** Task 2, before the AFTER runs.
- **Issue:** a temporary splicer-side probe on the plan's one-directional design (v1) showed it only swaps the roles in the starvation. The splicer completed only 1-7 splices per run, and a single splice call waited up to 1.32 s, which was the writer's whole 200-write run. It stays under 2 s only by environment margin, and the splicer barely exercises the lock.
- **Fix:** added `begun`/`done` counters so the writer also yields (EVIDENCE M6/M7). The AtomicBool `in_splice` in the plan became `begun > done`, which carries the same meaning.
- **Commit:** ffb2914.

**2. [Rule 3 - Harness] `QS8_KEEP_ALL=1` also passes `--nocapture` in binary mode**
- Without it, the probe output of passing iterations was swallowed by libtest, which made the first tmpfs probe run uninformative. Probe runs only; the before and after runs do not set KEEP_ALL, so their behaviour is identical. Commit 5ee12e1.

**3. The probe target is a full copy of driverctl_race.rs, not just the one test**
- In binary mode the same concurrent tests then load the probe binary as they load the real one in gate 4.

## Separate finding (not fixed)

`editor_race_smoke` failed 17 times in the before runs and 11 in the after runs (the `other` column), and every one of them was this test. Message: `inplace: the tool must have spliced at least once` (tests/driverctl_race.rs:348). Under container fsync cost, the in-place case gets about 17 tool regens in its 0.5 s window, and sometimes none of them lands. This is a separate non-vacuity timing assumption and a candidate for a follow-up quick task. It will also flake gate 4.

## Inferred decisions [INFERRED — audit]

- Profile set, n=200 and escalation order (planner choice). No escalation was needed.
- The cache-volume chown fallback in the harness. It was never triggered, because the volumes were already writable.
- A 100 µs post-splice sleep kept only as CPU relief.
- The `contended > 0` non-vacuity witness. It measured 200/200 in the container and about 110/200 on the host.
- The bidirectional handshake (deviation 1).
- No production change. The lack of fairness is recorded as a follow-up only.

## Threat Flags

None. No new network, auth or file-access surface. The harness reuses the capped volumes, runs as the host uid with `--rm`, and never mounts the docker socket.

## Commits

- 9e25c27 docs(quick-261023-qs8): add two-writers stress harness and before baseline
- ffb2914 fix(quick-261023-qs8): hand DIRECTION.lock over by handshake in the two-writers test
- 5ee12e1 docs(quick-261023-qs8): record after-fix stress rates and evidence

## Self-Check: PASSED

- FOUND: stress-two-writers.sh, stress-results.txt, 261023-qs8-EVIDENCE.md, tests/driverctl_race.rs
- FOUND: 9e25c27, ffb2914, 5ee12e1 (ancestors of HEAD)
- `grep -rn QS8- src tests` is empty, tests/driverctl_race_probe.rs is absent and `git diff --quiet a84c7c0 HEAD -- src` holds
- No stray worktrees, branches or containers; nothing pushed; no tags touched
