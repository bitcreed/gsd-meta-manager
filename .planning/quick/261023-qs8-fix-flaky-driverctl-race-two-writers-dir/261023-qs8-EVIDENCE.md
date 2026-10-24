# Evidence: flaky `two_writers_under_direction_lock_lose_nothing` (quick 261023-qs8)

## Symptom

`./scripts/pre-tag-check.sh --container` gate 4 (quick 261023-p51, SUMMARY line 99):
`tests/driverctl_race.rs:426` panicked with "write 77: the shared lock must be obtainable,
or this test no longer exercises it". That is the `!report.lock_timed_out` assertion. It
passed on rerun. Locally (host, tmpfs `/tmp`) the test has never been seen to fail.

## Baseline (stress-results.txt, base a84c7c0, src/tests untouched)

Harness: `stress-two-writers.sh`, one `docker run` per tally with pre-tag-check section 5's
flags (host uid 1000, `--cpuset-cpus 0-3` so nproc=4, `CARGO_INCREMENTAL=0`, the warm cache
volumes), image `gsdmm-pre-tag-check:latest`. `/tmp` inside the container is **overlay**
(on the host's ext4/zvol). Binary mode = the whole `driverctl_race` binary, default 4 test
threads, exactly as gate 4 runs it.

```
before profile=0 mode=binary n=200 fail=22 other=1 timeout=0 tree=a84c7c0+e3b0c44298fc
before profile=A mode=binary n=200 fail=19 other=11 timeout=0 tree=a84c7c0+e3b0c44298fc
before profile=B mode=binary n=200 fail=8 other=5 timeout=0 tree=a84c7c0+e3b0c44298fc
```

Reproduced on the first run, so no escalation (C, B-exact) was needed. Every target failure
is the line-426 lock assertion. 41 of the 49 failures are at **write 1** (19/22, 14/19, 8/8),
the rest at writes 42-196. More CPU load (profile B) lowers the rate rather than raising it.

## Hypotheses

- **H1, starvation by a fixed gap.** The splicer re-takes DIRECTION.lock ~100 µs after
  releasing it; the writer polls with a 1 ms..50 ms backoff (lock.rs:323, 335) and wins
  only if a poll lands in a gap. With hold ≫ gap, ~46 refused polls (2 s) become likely.
  Predicts: many splices complete during a failing write, each far below 2 s, and the
  effect collapses on tmpfs, where fsync is nearly free.
- **H2, a single long hold.** One splice (an fsync stall) holds ≥ 2 s. Predicts: 0-1
  splices during the failing write and a max single hold ≈ 2 s.
- **H3, a real product race.** Two simultaneous holders, a lock leaked past `drop(guard)`,
  or a second release path. Predicts: holder count > 1, or a hold outliving its call.
- **H4, anything else.**

## Measurements

All probes were temporary (marker `QS8-PROBE`), reverted before the fix; the probe tally
lines carry tree `9e25c27+b2c34d6362ba` (the probe diff).

**M1, timing probe.** `tests/driverctl_race_probe.rs` = a full copy of `driverctl_race.rs`
(so the same concurrent tests load the binary) with the two-writers test instrumented: per
write the wall time and how many splices completed during it; per splice the call duration
and the gap to the next one. A temporary block in `locked()` (src/driver/direction.rs)
recorded, for the two probe threads only (by thread name), the lock **wait** (time in
`direction_lock`) and **hold** (time from acquisition to `drop(guard)`) of every call.
Profile 0, binary mode, `QS8_KEEP_ALL=1` (+ `--nocapture`), n=40, overlay `/tmp`:

```
probe profile=0 mode=binary n=40 fail=5 other=1 timeout=0 tree=9e25c27+b2c34d6362ba ... tmpfs=0
```

| metric (overlay, 40 iterations)          | value                                   |
|------------------------------------------|-----------------------------------------|
| splicer lock hold, per-iteration p50     | 3.0 ms .. 19.1 ms                       |
| splicer lock hold, max single hold       | 26.8 ms (all 40 iterations)             |
| writer lock hold, per-iteration p50      | 3.0 ms .. 5.5 ms                        |
| splicer gap (release -> next start), p50 | ~160 µs (`sleep(100µs)` + scheduling)   |
| writer lock wait, max                    | 2 000 137 µs (= the 2 s budget, timeout)|

The 5 timed-out writes (k=1, 3, 1, 1, 1):

| k | splices completed during the write | splicer held the lock for | longest single hold |
|---|------------------------------------|---------------------------|---------------------|
| 1 | 471 | 94.9 % of the 2 s | 23.4 ms |
| 3 | 259 | 97.3 % | 20.5 ms |
| 1 | 513 | 94.5 % | 13.6 ms |
| 1 | 509 | 94.5 % | 13.8 ms |
| 1 | 503 | 94.6 % | 13.3 ms |

An earlier n=40 overlay probe run (before `--nocapture` was wired, so only failing
iterations printed) showed the same for its 6 timed-out writes: 294-537 splices during the
write, splicer holding 93.9-94.5 % of it, longest single hold 13.2-16.8 ms.

Capture dynamics: splices per iteration are bimodal: 10/40 iterations had the splicer
complete fewer than 10 splices in total (the writer captured the lock), the rest 13-835.
Whoever holds the lock tends to keep it: the holder re-polls immediately, the loser is in
exponential backoff. Write 1 is the only write that always starts while the splicer is in
its tight loop and not in backoff, which is why 41/49 baseline failures are at write 1.
The splicer's own worst wait was 1.37 s (the mirror image, bounded by the writer's
200-write run; it never reached 2 s in 86 probed iterations).

**M2, falsification run: tmpfs.** Same profile, same n, same binary, `QS8_TMPFS=1`
(TMPDIR=/dev/shm, `df -T` = tmpfs):

```
probe profile=0 mode=binary n=40 fail=0 other=0 timeout=0 tree=9e25c27+b2c34d6362ba ... tmpfs=1
```

| metric (tmpfs, 40 iterations)        | value              |
|--------------------------------------|--------------------|
| splicer lock hold, per-iteration p50 | 58 µs .. 82 µs     |
| splicer lock hold, max               | 344 µs             |
| write wall, max                      | 7.4 ms             |
| timed-out writes                     | 0/40 iterations    |

The hold collapses ~50x (3 ms -> 60 µs) with the same CPU load, and the failure goes with it.
The hold is dominated by `write_temp`'s `sync_all` (direction.rs:597) inside the lock:
on tmpfs fsync is a no-op, on overlay-on-ext4 every fsync is a journal commit.
Host local run, tmpfs `/tmp`, for reference: hold p50 66 µs, write wall max 3.3 ms.

**M3, mutual-exclusion probe.** Same temporary block: a process-wide `AtomicU32` holder
count, incremented right after `direction_lock` succeeded and decremented right before
`drop(guard)`, restricted to the two probe threads (they share one DIRECTION.lock), with the
maximum kept in a static and printed by the probe test.
Result: **max_holders=1 in 86/86 probed iterations** (40 overlay + 40 tmpfs with full output,
plus the 6 failing iterations of the first run), including every iteration with a timeout.

**M4, code reading for H3.**
- `locked()` (src/driver/direction.rs:559-582): the guard is bound at :570 and dropped at
  :577, after `body()` at :576 returns, so the hold covers the whole body. On `body()?`
  error the guard drops on return, which still releases after the body.
- `acquire_at` (src/driver/lock.rs:196-258) opens a fresh descriptor per attempt
  (:201-209), so every attempt has its own open file description, and `flock` exclusion
  between two threads of one process is real (:211, `NonBlockingLockExclusive`).
- `RunLock` (lock.rs:139-154) has no `impl Drop` and no explicit unlock: the only release
  path is closing the held `File`.
- `write_holder` (lock.rs:261-282) writes through the held descriptor (`set_len`,
  `rewind`, `write_all`), never a rename, so it cannot hand the lock to another inode.
- `acquire_at_with_retry` (lock.rs:317-341): polling with sleep 1 ms doubling to
  `MAX_SLEEP` 50 ms (:323, :335), bounded by the budget. There is no queue and no fairness
  by design (D-13/D-20): a waiter wins only if a poll lands in a gap.

**M5, production cadence.** `DriverStore::regen` (src/driverctl/store.rs:790-806) runs one
`splice_frontmatter` per accepted event; heartbeat-only events regenerate at most every
`HEARTBEAT_REGEN_MIN_S` = 120 s (store.rs:72). A real run emits events every few seconds,
so the lock is held a few ms out of every few seconds: no production path re-takes
DIRECTION.lock in a ~100 µs loop, and nothing holds it near-continuously for 2 s. Even if
that happened, the timeout is safe and visible by design: `locked()` proceeds without the
lock (T-27-02), the exchange-verify still protects the human's bytes, and the store logs
`direction-lock-timeout`.

## Eliminated

- **H2 (single long hold): falsified.** Failing writes saw 259-537 completed splices, and
  the longest single hold in any iteration was 26.8 ms, about 75x below the 2 s budget.
- **H3 (product race): falsified.** max_holders=1 in 86/86 iterations, including all the
  timeouts; the code reading (M4) shows one release path and real per-description flock.
- **H4:** no measurement pointed elsewhere. The `other` failures are a different test,
  `editor_race_smoke` (separate finding, below).

## Product-race verdict

**No product race in the direction-lock code.** Mutual exclusion held in every probed
iteration (max_holders=1, M3), and `locked()` / `acquire_at` / `acquire_at_with_retry` /
`RunLock` have a single release path and real per-open-file-description exclusion
(file:line in M4). The lock's lack of fairness is a documented design property (D-13/D-20
poll-with-backoff). It is reachable only by a writer that re-takes the lock in a tight
loop, which no production path does (M5). Follow-up, not in scope: if a future writer ever
does loop on DIRECTION.lock, the poll schedule offers it no fairness; today the logged,
lock-free fallback covers that.

root_cause: TEST-CONSTRUCTION TIMING ASSUMPTION. Code condition: the test's splicer re-takes DIRECTION.lock ~160 µs after releasing it, and the writer's non-fair polling retry (1 ms..50 ms, ~46 polls in the 2 s budget) wins only if a poll lands in that gap. AND environment condition: the hold is dominated by `sync_all` inside the lock, which costs ~3 ms (up to 27 ms) on overlay-on-ext4 in the container versus ~60 µs on tmpfs. So the splicer holds the lock ~95 % of the time, and on the writes that start while the splicer is in its tight loop (above all write 1) all ~46 polls are refused. Measured: 259-537 splices completed during each timed-out write, 0/40 failures on tmpfs with the same load, max_holders=1 throughout.

## Fix design: measured, one candidate falsified

**M6, the plan's one-directional handshake (fix v1) only moves the starvation.** v1 made the
splicer yield while `requested > acquired` and left the writer free to re-take the lock
straight away. A temporary splicer-side probe (`QS8-PROBE`, reverted) on v1, profile 0,
binary, n=40 (`probe ... tree=9e25c27+92be13895c13`): 0 target failures, but the splicer
completed only **1-7 splices per run**, and its longest single `splice_frontmatter` call was
**367 ms .. 1.32 s**: it waited behind the writer's entire 200-write run (0.84-0.91 s).
Host: 9 splices, 31 ms max. It is the same non-fair-poll starvation with the roles swapped.
It stays under 2 s only because 200 writes x ~3-5 ms happens to fit in the budget, which is
an environment margin. And the splicer then barely exercises the lock at all.

**M7, symmetric handshake (fix v2, shipped).** v1 plus `begun`/`done` splice counters:
before each write the human waits until every splice begun so far has finished (no splice
waits behind more than one write), and until a new one has begun (so the write meets a busy
lock). Same probe, same profile and n (`probe ... tree=9e25c27+c9b60b1add8a`): 0 target
failures; splices per run 399-401 (two per write); splicer lock timeouts **0**; longest
single splice call **39 ms**; longest single write **42 ms**; contended writes **200/200**
in every run. Host: ~260 splices, ~110/200 contended, both maxima ~1.2-1.5 ms. Each side
now waits for at most one hold of the other, about 50x below the 2 s budget whatever the
fsync cost, unless a single hold reaches 2 s (H2, never observed).

fix_rationale: Test-side, src/ unchanged. The product contract the test pins (two gsd-mm writers serialised by DIRECTION.lock lose nothing, and the lock is obtainable) is correct and stays exercised unchanged. What was wrong is the test's hope that a 100 µs gap opens a window for a non-fair poller, a window whose size relative to the hold depends on fsync cost. The fix replaces that hope with a handshake in both directions. (1) `requested`/`acquired`: while the writer has asked for the lock and its edit closure (which runs only under the lock) has not yet run, the splicer starts no new splice, so the writer waits for at most the splice in flight. Right after the acquisition the splicer splices at once and so contends while the writer holds the lock. (2) `begun`/`done`: before its next write the writer waits until every begun splice has finished and a new one has begun, so the splicer is never starved by the writer either (M6 showed that one-directional yielding starves it), and the write meets a busy lock. Every wait is bounded by 2 x DIRECTION_LOCK_BUDGET, so a broken lock fails an assertion instead of hanging. A `contended > 0` witness guards non-vacuity. Every original assertion is kept; the lock budget and poll schedule (D-13/D-20 product design) are untouched; nothing is widened.

## Separate finding (not fixed here)

`editor_race_smoke` failed 17 times across the three baseline runs (the `other` column, 1
in profile 0, 11 in A, 5 in B), plus once in a probe run. Example (profile 0, iteration 124):
`inplace: the tool must have spliced at least once: {"gave-up": 13, "skipped-insane-body": 4}`
(tests/driverctl_race.rs:348). Under container fsync cost the in-place smoke case gets only
~17 tool regens in its 0.5 s window and sometimes none of them lands. That is a separate
non-vacuity timing assumption in a different test. Recorded for a follow-up quick task.
## Verification

Fix commit: **ffb2914** `fix(quick-261023-qs8): hand DIRECTION.lock over by handshake in the
two-writers test` (tests/driverctl_race.rs only; src/ byte-identical to a84c7c0).

AFTER, the same harness, profiles, mode and n as BEFORE. The tree suffix `e748692becb1` was
recomputed just before the commit as `git diff HEAD -- src tests Cargo.toml Cargo.lock |
sha256sum`, and it equals the suffix on every after line:

```
after profile=0 mode=binary n=200 fail=0 other=2 timeout=0 tree=9e25c27+e748692becb1
after profile=A mode=binary n=200 fail=0 other=4 timeout=0 tree=9e25c27+e748692becb1
after profile=B mode=binary n=200 fail=0 other=5 timeout=0 tree=9e25c27+e748692becb1
```

| profile | mode   | n   | before target fails | after target fails | before/after other | timeouts |
|---------|--------|-----|---------------------|--------------------|--------------------|----------|
| 0       | binary | 200 | 22                  | 0                  | 1 / 2              | 0 / 0    |
| A       | binary | 200 | 19                  | 0                  | 11 / 4             | 0 / 0    |
| B       | binary | 200 | 8                   | 0                  | 5 / 5              | 0 / 0    |

All 11 after-side `other` failures are `editor_race_smoke` (the separate finding above).
A follow-up `probe 0 exact 3` on the same tree printed `two writers: splices=401
contended_writes=200/200` three times, which confirms the container ran the fixed binary.

Mutant M-A (`locked()` with guard None and `lock_timed_out` true, marker `QS8-MUTANT`):
`cargo test --test driverctl_race two_writers_under_direction_lock_lose_nothing` failed
**3/3** at tests/driverctl_race.rs:488 ("write 1: the shared lock must be obtainable").
Reverted; `git diff --quiet HEAD -- src` passes.

Local gates (host, after the revert):
- `cargo test --test driverctl_race --test driverctl_lock_race`: ok (4 passed + 1 ignored; 5 passed).
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `cargo test --no-fail-fast`: 89 suites; the only failure is
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
  (the known local git-version witness; green on the runner).
