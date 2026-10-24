# Evidence: flaky `editor_race_smoke` (quick 261023-v1s)

base_head: 08fc9f7286f15a79c8819582f916c28becf54056
tag_v1_15_0: 08fc9f7286f15a79c8819582f916c28becf54056

## Symptom

`tests/driverctl_race.rs::editor_race_smoke` fails intermittently in the pre-tag container
(gate 4 of `./scripts/pre-tag-check.sh --container`) with the panic message
`inplace: the tool must have spliced at least once: {"gave-up": 13, "skipped-insane-body": 4}`
(the `ok > 0` non-vacuity assertion in `assert_exchange_holds`, tests/driverctl_race.rs:355-359
at 08fc9f7). Failures are classified by this MESSAGE, never by line number.

## Prior data

Quick 261023-qs8 ran the whole `driverctl_race` binary 200 times per profile in the same
container. Its `other` column was `editor_race_smoke` every time (qs8 EVIDENCE line 186
and line 213):

| qs8 run          | profile 0 | profile A | profile B | total   |
|------------------|-----------|-----------|-----------|---------|
| before (a84c7c0) | 1/200     | 11/200    | 5/200     | 17/600  |
| after (ffb2914)  | 2/200     | 4/200     | 5/200     | 11/600  |

Corrections to the notes (trust the measurements, not the notes):
- The task description and qs8 EVIDENCE cite `tests/driverctl_race.rs:348`. That is the
  line before ffb2914 grew the header by 7 lines. At 08fc9f7, line 348 is the leftover-temps
  message; the `ok > 0` assertion starts at line 355.
- qs8 EVIDENCE says the in-place case has a "0.5 s window". `editor_race_smoke` is
  `assert_exchange_holds(1.0, false)` (line 365), so each case runs 1.0 s. (0.5 s is the
  naive positive control's per-attempt length, line 375.)

## Baseline (stress-results.txt, base 08fc9f7, src/tests untouched)

Harness: `stress-editor-race.sh`, adapted from qs8's `stress-two-writers.sh` (same container
flags: host uid, `--cpuset-cpus 0-3` so nproc=4, `CARGO_INCREMENTAL=0`, the warm cache
volumes, image `gsdmm-pre-tag-check:latest`; same load profiles and tally format). `/tmp`
inside the container is **overlay**. Binary mode = the whole `driverctl_race` binary, default
4 test threads, as gate 4 runs it. Tracer: `probe 0 exact 2` appended one line (0 fails).

```
before profile=0 mode=binary n=200 fail=1 other=0 timeout=0 tree=08fc9f7+e3b0c44298fc
before profile=A mode=binary n=200 fail=1 other=0 timeout=0 tree=08fc9f7+e3b0c44298fc
before profile=B mode=binary n=200 fail=5 other=0 timeout=0 tree=08fc9f7+e3b0c44298fc
```

Target-fail total 7 >= 6, so no escalation (C, B-exact) was run. By panic MESSAGE:

| run      | iter | message                                                             | tool tally                                |
|----------|------|---------------------------------------------------------------------|-------------------------------------------|
| before 0 | 138  | `inplace: the tool must have spliced at least once`                 | gave-up 13, skipped-insane-body 2         |
| before A | 75   | **`a rename-save must never be lost`** (rename LOST=1, transient=0) | gave-up 12, ok 10 (saves=698, regens=22)  |
| before B | 90   | `rename: the tool must have spliced at least once`                  | gave-up 13                                |
| before B | 97   | `inplace: the tool must have spliced at least once`                 | gave-up 12, skipped-insane-body 5         |
| before B | 184  | `rename: the tool must have spliced at least once`                  | gave-up 11                                |
| before B | 188  | `rename: the tool must have spliced at least once`                  | gave-up 13                                |
| before B | 193  | `inplace: the tool must have spliced at least once`                 | gave-up 13, skipped-insane-body 1         |

Two distinct failure modes, not one:
- **F1, starvation (6/7):** a case ends with `ok == 0` after only 11-17 splice calls in 1.0 s,
  every one GaveUp (or skipped). Contrary to the task description, it is NOT confined to the
  in-place case: under profile B the rename case starved 3 times, the in-place case twice.
- **F2, a lost rename-save (1/7):** the exchange strategy lost one rename-save. That is the
  assertion the whole D-12/D-23 design exists to make true, so F2 is investigated as a
  possible PRODUCT defect in its own right (H7 below), not folded into F1.

Note on HEAD: while this task ran, a concurrent session committed four docs-only commits on
master (4563fd5, ee65618, 9252aa5, 5e952d0; `.planning/phases/28-*` only).
`git diff --stat 08fc9f7 HEAD -- src tests Cargo.toml Cargo.lock scripts` is empty, so the
code under test is byte-identical to 08fc9f7; later tally lines carry the newer HEAD prefix
with the same empty-diff suffix `e3b0c44298fc`.

## Hypotheses

- **H1, fsync-in-window starvation (test timing assumption).** Each attempt's window (read
  -> exchange) contains `write_temp`'s `sync_all` (direction.rs:597); on overlay it exceeds
  the human's inter-save interval, so all 5 attempts mismatch and the splice gives up.
  Predicts: window p50 >> inter-save p50 with sync_all dominant; low oks on overlay, high on
  tmpfs under the same load; failures vanish on tmpfs.
- **H2, false mismatch (PRODUCT, verify at direction.rs:660-669 or the non-UTF-8 retry at
  621-623).** Predicts: a mismatch with no human save overlapping the attempt.
- **H3, lock anomaly (PRODUCT).** Predicts: lock_timed_out > 0 or a lock wait far above 0.
- **H4, CPU starvation, not fsync.** Predicts: failures persist on tmpfs under the same load.
- **H5, the rename/in-place asymmetry.** Why would the rename case land oks and the in-place
  case not?
- **H6, anything else.** The data produced one: **H7**, below.
- **H7, the newest-wins plain rename (PRODUCT, direction.rs:688-693).** After a mismatch the
  tool exchanges back. If the file then at the temp name is not the tool's own, module fact 3
  puts it in place with a plain `std::fs::rename(temp, DIRECTION.md)`, which is a
  read-then-rename (module fact 2's named bug): a human rename-save landing between the
  exchange back and that rename is overwritten by the older save. Predicts: a lost
  rename-save only when the newest-wins branch ran, and a deterministic loss when a save is
  landed exactly in that window.

## Measurements

All probes are temporary (marker `V1S-PROBE`; tally `note=V1S-PROBE-*`), applied by script
from the scratchpad and reverted with `git checkout -- src/driver/direction.rs` plus deleting
`tests/driverctl_race_probe.rs`.

**M1, zero-code distributions, profile B (the profile with the most BEFORE failures), binary,
n=40, `V1S_KEEP_ALL=1`**, per-case lines from cases.txt (1.0 s cases):

| fs (same load)   | style   | ok p50 (min) | P(ok==0) | gave-up p50 | gave-up share p50 | skipped p50 | saves/s | regens/s | transient mean |
|------------------|---------|--------------|----------|-------------|-------------------|-------------|---------|----------|----------------|
| overlay          | inplace | 9 (0)        | 1/40     | 11          | 0.44              | 4           | 702     | 24       | 0.9            |
| overlay          | rename  | 6 (0)        | 1/40     | 11          | 0.69              | 0           | 621     | 17       | 0.8            |
| tmpfs            | inplace | 2620 (2377)  | 0/40     | 0           | 0.00              | 11          | 839     | 2633     | 0.4            |
| tmpfs            | rename  | 2448 (2159)  | 0/40     | 0           | 0.00              | 0           | 770     | 2448     | 0.6            |

```
probe profile=B mode=binary n=40 fail=2 other=0 timeout=0 tree=ee65618+e3b0c44298fc ... tmpfs=0 note=V1S-PROBE-M1-overlay
probe profile=B mode=binary n=40 fail=0 other=0 timeout=0 tree=ee65618+e3b0c44298fc ... tmpfs=1 note=V1S-PROBE-M1-tmpfs
```
The two overlay failures: `inplace ... {"gave-up": 7, "skipped-insane-body": 22}` and
`rename ... {"gave-up": 13}`. In-place LOST stayed within its Poisson bound in all 80 cases
(max 1, allowed 1).

**M2, window and correlation probe** (`tests/driverctl_race_probe.rs`, a full copy of
driverctl_race.rs whose tool threads are named `v1s-tool-<strategy>-<style>` and whose human
records each save's start/end; a temporary recorder in `run_loop`/`locked`/`write_temp`
records, for those threads only, every attempt's read, sync_all, exchange and verdict, every
splice's lock wait and `lock_timed_out`, and every exchange-back and newest-wins rename).
Profile B, binary, n=40, overlay and tmpfs:

```
probe profile=B mode=binary n=40 fail=1 other=0 timeout=0 tree=5e952d0+fa9f6a5cbc1c ... target=driverctl_race_probe tmpfs=0 note=V1S-PROBE-M2-overlay
probe profile=B mode=binary n=40 fail=0 other=0 timeout=0 tree=5e952d0+fa9f6a5cbc1c ... target=driverctl_race_probe tmpfs=1 note=V1S-PROBE-M2-tmpfs
```

| metric (per case: p50 over 40 cases; sums are over all 40)   | overlay inplace | overlay rename | tmpfs inplace | tmpfs rename |
|--------------------------------------------------------------|-----------------|----------------|---------------|--------------|
| attempt window read->exchange, p50 (us)                      | 4 952           | 5 001          | 33            | 33           |
| window p99 (us)                                              | 8 752           | 8 952          | 1 583         | 1 890        |
| sync_all p50 (us)                                            | 4 733           | 4 758          | 0             | 0            |
| sync_all share of the window                                 | 0.940           | 0.939          | 0.000         | 0.000        |
| human inter-save p50 / p99 (us)                              | 1 253 / 5 258   | 1 395 / 5 823  | 1 124 / 4 588 | 1 159 / 4 978|
| exchanged attempts overlapped by >= 1 save                   | 2474/2810 = 88% | 2453/2805 = 87%| 5596/104847 = 5.3% | 6001/98769 = 6.1% |
| mismatches WITH an overlapping save                          | 2461            | 2433           | 3078          | 3171         |
| **mismatches with NO overlapping save (false, H2)**          | **0**           | **0**          | **0**         | **0**        |
| splices per case p50                                         | 25              | 19             | 2503          | 2378         |
| **lock_timed_out (H3)**                                      | **0**           | **0**          | **0**         | **0**        |
| lock wait max per case, p50 / max (us)                       | 1 910 / 4 723   | 1 894 / 6 035  | 7 068 / 11 028| 7 328 / 25 056 |

Newest-wins branch (H7), both styles together: overlay 82 events in 40 iterations (max 5 per
iteration), longest exchange-back -> rename end 1 974 us; tmpfs 28 events, max 24 us. No
rename-save was lost in M2 (the 7 losses counted are in-place, inside their bound), so the
probe saw the precondition of F2 but not F2 itself.

The overlay failure: `inplace ... {"gave-up": 7, "skipped-insane-body": 38}`.

**M3, causal intervention in both directions** (same probe target, profile B, binary, n=40;
the intervention applies only to the smoke's named tool threads, so the load is unchanged):

```
probe profile=B mode=binary n=40 fail=0 other=0 timeout=0 tree=b189c0d+66c47c678413 ... tmpfs=0 note=V1S-PROBE-M3a-overlay-nosync
probe profile=B mode=binary n=40 fail=4 other=0 timeout=0 tree=4784524+334f74febd3f ... tmpfs=1 note=V1S-PROBE-M3b-tmpfs-delay5000us
```

| run                                            | style   | window p50 (us) | ok p50 (min) | gave-up share p50 | P(ok==0) | regens/s |
|------------------------------------------------|---------|-----------------|--------------|-------------------|----------|----------|
| M2 overlay (control)                           | inplace | 4 952           | 8 (0)        | 0.42              | 1/40     | 25       |
| M2 overlay (control)                           | rename  | 5 001           | 9 (1)        | 0.56              | 0/40     | 19       |
| **M3a overlay, `sync_all` removed**            | inplace | 74              | **954** (676)| **0.00**          | 0/40     | 965      |
| **M3a overlay, `sync_all` removed**            | rename  | 74              | **841** (686)| **0.00**          | 0/40     | 841      |
| M2 tmpfs (control)                             | inplace | 33              | 2495 (2239)  | 0.00              | 0/40     | 2503     |
| M2 tmpfs (control)                             | rename  | 33              | 2378 (1970)  | 0.00              | 0/40     | 2378     |
| **M3b tmpfs, 5 000 us sleep in the window**    | inplace | 5 098           | **7** (0)    | **0.61**          | **4/40** | 18       |
| **M3b tmpfs, 5 000 us sleep in the window**    | rename  | 5 099           | **6** (1)    | **0.65**          | 0/40     | 17       |

Both predictions of H1 held: removing the fsync on overlay raised the oks ~100x and gave-up
vanished; lengthening the window to the overlay p50 on tmpfs dropped the oks to overlay
levels and produced 4 ok==0 failures (`inplace ... {"gave-up": 13}` each). False mismatches
stayed 0 and lock timeouts 0 in both. All four probe tally lines carry their probe tree.

**M4, H7 made deterministic (red on base behaviour).** A `#[cfg(test)]` per-thread hook,
fired at three instants of `run_loop` (before the exchange, after it, after the exchange
back) and compiled out of every non-test build, lets a unit test land rename-saves exactly
there. Scenario: save 1 between read and exchange (verify mismatches), save 2 before the
exchange back (so save 2 comes back to the temp name and older save 1 to DIRECTION.md), save
3 right after the exchange back. With the hook only (no behaviour change) on the base code,
`cargo test --lib driver::direction` fails exactly that test:

```
test driver::direction::tests::linux::a_save_landing_right_after_the_exchange_back_is_not_reverted ... FAILED
  left: "...- note: save 2..."      (what DIRECTION.md holds after the splice)
 right: "...- note: save 3..."      (the newest human save)
test result: FAILED. 19 passed; 1 failed
```

The newest-wins `std::fs::rename(temp, DIRECTION.md)` at direction.rs:691 put save 2 over
save 3: a lost rename-save, which is precisely what `before A` iter 75 reported (rename
LOST=1, transient=0: the human's read-back after save k+1 still saw k+1, so the exchange
back came after it, and save k+2 then fell into the exchange-back -> rename window). The
two companion scenarios (one save before the exchange; a newer save found at the exchange
back) pass on base. M2 shows the precondition is common in the container: 82 newest-wins
renames in 40 overlay iterations, their window up to 1.97 ms against a human inter-save p50
of 1.25-1.4 ms.

## Eliminated

- **H2 (false mismatch): falsified.** 0 mismatches without an overlapping human save across
  5 615 overlay and 203 616 tmpfs exchanged attempts (M2), and 0 again in M3a/M3b.
- **H3 (lock anomaly): falsified.** `lock_timed_out` 0 in every probed splice (overlay 1 827
  splices, tmpfs 197 840, plus M3a/M3b). The worst single lock wait was 25 ms on tmpfs under
  profile B's 8 spinners (CPU preemption), 80x below the 2 s budget. Each test owns its
  TempDir, so the tool is the only DIRECTION.lock taker.
- **H4 (CPU starvation, not fsync): falsified.** Same load on tmpfs: 0/80 starved cases in
  M1, 0/80 in M2; the window is 94 % `sync_all` on overlay (M2), and removing just the
  fsync (M3a) cured it under the same load.
- **H5 (rename/in-place asymmetry): falsified as stated.** The rename case does NOT reliably
  land more oks: BEFORE B starved rename 3x and in-place 2x; M1 overlay ok p50 is 6 (rename)
  vs 9 (in-place), P(ok==0) 1/40 each. Both styles have the same ~5 ms window against a
  similar cadence (inter-save p50 1.40 ms rename vs 1.25 ms in-place; transient pauses
  0.8-1.1 per case in both, too few to matter). The in-place case's extra regens are
  `skipped-insane-body` returns (reads that caught the truncate), which never count as ok,
  and its slightly faster cadence makes a quiet 5 ms window a little rarer (M3b: in-place
  4/40 ok==0, rename 0/40). The qs8 example happened to be an in-place one.
- **H7 (newest-wins plain rename): CONFIRMED** (M4, before A iter 75).

## Product verdict

**F2 is a product defect.** Module fact 3 (direction.rs:28-30) resolves "a newer rename-save
came back to the temp name" with a plain rename (direction.rs:688-693). That is a
read-then-rename, the exact bug module fact 2 (direction.rs:18-27) forbids, applied in the
one place the code still does it: a human rename-save landing between the exchange back and
that rename is overwritten by an older save. The window is microseconds on tmpfs and up to
~2 ms on overlay, and it needs two human saves in quick succession, so production users
will rarely hit it, but it violates the module's central guarantee ("a human's bytes are
never unlinked unseen"; the race test's "a rename-save must never be lost").

**F1 is not a product defect.** With H2 and H3 falsified, GaveUp under an editor that saves
faster than one fsync is the designed human-wins outcome: `SpliceOutcome::GaveUp` is
documented as "Every attempt lost a race with a human save; the human's bytes stand"
(direction.rs:326); the fsync must precede the exchange (module fact 6, direction.rs:44-47,
D-23); production splices once per accepted event and coalesces heartbeat regens to at most
one per `HEARTBEAT_REGEN_MIN_S` = 120 s (src/driverctl/store.rs:72, `regen` at 790), so a
GaveUp there just means the next event regenerates. The smoke's `ok > 0` witness assumed
that 1 s of a ~1.3 ms-cadence human always leaves at least one fsync-length quiet window.

## Product alternatives considered (for F1)

- **Move the fsync after the exchange:** rejected, violates D-23 / fact 6 (a power loss
  between write and exchange could surface an empty DIRECTION.md).
- **More tries or longer backoff:** rejected, changes SPLICE_TRIES / the retry budget and
  only shifts the ratio; a human saving faster than one fsync still wins every attempt.
- **Re-read just before the exchange:** rejected, the temp embeds the body read before the
  fsync, so the success window still spans the fsync; a re-read can only add a cheaper
  early-out, not a quieter window.

root_cause: TWO causes behind one flaky test. (F1, 6/7 BEFORE failures, test timing assumption) Code condition: every splice attempt's read->exchange window contains `write_temp`'s `sync_all` (direction.rs:597, D-23), the verify gives up after SPLICE_TRIES=5 mismatches, and editor_race_smoke asserts `ok > 0` after 1.0 s of a human saving every ~1.3 ms; AND environment condition: on the container's overlay `/tmp` that window is 4.95-5.0 ms p50 (94 % fsync) against a 1.25-1.4 ms human inter-save p50, so 87-88 % of attempts are overlapped by a save, a case gets only ~17-25 splices of which a median 0.42-0.69 give up, and sometimes none lands (P(ok==0) about 1/40 per style under profile B). Removing the fsync on overlay raised oks ~100x (M3a); a 5 ms in-window sleep on tmpfs reproduced the starvation (M3b, 4/40). (F2, 1/7, PRODUCT) The newest-wins branch of the exchange-verify loop (direction.rs:688-693, module fact 3) puts a newer save back with a plain rename, so a third rename-save landing between the exchange back and that rename is reverted; made deterministic in M4 (red on base), observed as `before A` iter 75 (rename LOST=1).

product_verdict: yes — the newest-wins plain rename at src/driver/direction.rs:688-693 can revert a human rename-save (H7, deterministic red test M4); H2 (0 false mismatches) and H3 (0 lock timeouts) are falsified, and the starvation (F1) is designed GaveUp behaviour exposed by a test timing assumption, so it gets the test-side fix in addition.

## Fix

fix_rationale: Product branch first, then the test-side fix, because both causes are real. (F2, product, src/driver/direction.rs) The newest-wins branch keeps its meaning (the newest human save wins, fact 3) but puts the newer file in place by `renameat2(RENAME_EXCHANGE)` instead of a plain rename, and then checks what came back: if it is, unchanged (same inode AND same bytes), the older file it expected at DIRECTION.md, that file is superseded and dropped; anything else landed in the meantime, is newer still, and is exchanged into place the same way. A human file is therefore never renamed over anything and never unlinked unless a newer one is in place; after GIVE_BACK_ROUNDS=8 exchanges that do not settle, the splice returns an I/O error naming the file left at the temp name (never unlinks). D-23 is kept: fsync before the exchange, verify by body bytes, human-wins/GaveUp, SPLICE_TRIES=5, the 5 x attempt ms backoff, DIRECTION_LOCK_BUDGET=2 s, proceed-without-lock on timeout and `lock_timed_out` reporting are all untouched. The regression test is the M4 unit test, driven by a `#[cfg(test)]` hook that does not exist in non-test builds. (F1, test-side, tests/driverctl_race.rs) The product fix cannot cure the starvation (M3: it follows the fsync window, not the give-back), so the smoke replaces its timing hope with a count-based handshake, as qs8 did for the two-writers test: the tool counts `begun`/`done`/`ok` splices; while `ok == 0`, the human yields once whenever a splice has completed since its last yield, saving nothing until a splice that began after the yield has finished, so that splice sees no save at all; if its loop ends with `ok == 0` it makes one final yield, and the tool keeps splicing past its deadline until that is over. Every wait is `wait_until`-bounded (2 x DIRECTION_LOCK_BUDGET) and ends early when the tool exits (drop guards set the exit flags even on panic). After the first ok the human's cadence is exactly the old one, so the in-place Poisson accounting of `editor_race_full` is unchanged after the first ok. A new contention witness counts saves that start while a splice is in flight and asserts it is above 0 for both cases, so the race is still proven exercised. Every original assertion, message, run length (1.0 s / 15.0 s), the Poisson bound and the positive control are kept. Rejected: extending the case until ok > 0 (still P(no ok) = e^(-rate x T), margin-widening); retrying the case (rerun-until-green masks regressions such as F2 itself, which a retry would have hidden); slowing the human globally (changes the spike experiment the in-place bound is calibrated on, and is still a ratio-of-timings assumption).

**Red-first, on the current tree.** Hook + regression tests applied without the fix:
`cargo test --lib driver::direction::tests::linux` -> `a_save_landing_right_after_the_exchange_back_is_not_reverted ... FAILED`
(12 passed, 1 failed; DIRECTION.md shows save 2, the newest was save 3). With the fix:
`cargo test --lib driver::direction` -> 20 passed, 0 failed.

**Product fix alone does not cure F1** (`[INFERRED — audit]` decision to add the test-side
fix): M1 re-run with only the product change, profile B, binary, n=40:
```
probe profile=B mode=binary n=40 fail=0 other=0 timeout=0 tree=aad9f95+a1571cfa4b42 ... note=V1S-PROBE-M1-productfix-only
```
in-place ok min 1 / p10 1 / p50 4, rename ok min 1 / p10 1 / p50 4, gave-up share p50
0.48 / 0.71: the same fsync-window starvation margin as before (0/40 ok==0 here, against
~1/40 per style measured at BEFORE and in M1), as M3 predicts, so the test-side yield was
applied too.

## Verification (Task 2)

Fix commit: **717d724** `fix(quick-261023-v1s): give newer saves back by exchange and grant
the smoke one quiet splice` (src/driver/direction.rs + tests/driverctl_race.rs). The tree
suffix `31d595e20190` was recomputed just before the commit as `git diff HEAD -- src tests
Cargo.toml Cargo.lock | sha256sum` and equals the suffix on every after line (the HEAD
prefixes differ only by the concurrent session's docs-only commits):

```
after profile=0 mode=binary n=200 fail=0 other=0 timeout=0 tree=fcf5e9e+31d595e20190
after profile=A mode=binary n=200 fail=0 other=0 timeout=0 tree=ee47494+31d595e20190
after profile=B mode=binary n=200 fail=0 other=0 timeout=0 tree=ee47494+31d595e20190
```

| profile | mode   | n   | before target fails | after target fails | before/after other | timeouts |
|---------|--------|-----|---------------------|--------------------|--------------------|----------|
| 0       | binary | 200 | 1                   | 0                  | 0 / 0              | 0 / 0    |
| A       | binary | 200 | 1                   | 0                  | 0 / 0              | 0 / 0    |
| B       | binary | 200 | 5                   | 0                  | 0 / 0              | 0 / 0    |

No `other` failure in any AFTER run, so nothing new for the scan from Task 2.

**Mechanism in the container** (`probe B binary 20`, `V1S_KEEP_ALL=1`, fixed tree):
```
probe profile=B mode=binary n=20 fail=0 other=0 timeout=0 tree=ee47494+31d595e20190 ... note=V1S-PROBE-after-mechanism
```
| style   | yields per case (min/p50/max) | contended saves per case (min/p50/max) | ok (min/p50) |
|---------|-------------------------------|----------------------------------------|--------------|
| inplace | 0 / 1 / 1                     | 582 / 694 / 742                        | 3 / 12       |
| rename  | 0 / 1 / 1                     | 542 / 598 / 664                        | 3 / 10       |
| naive   | 0 / 0 / 0                     | 25 / 47 / 72                           | 1033 / 1149  |

The yield fires at most once per case (as soon as the first splice completes without an ok),
the race stays exercised (hundreds of saves land while a splice is in flight), and the naive
control never yields (I6). Load caveat `[INFERRED — audit]`: profiles A/B's load loops run
the fixed smoke after the fix; at most one yield (one splice, ~5-50 ms of human pause) per
1 s case, so the load differs by well under 5 %.

Local (host) run of the fixed binary: `exchange rename ... yields=0 contended=901
tool={"ok": 5034}`, `exchange inplace ... yields=0 contended=929`, naive `yields=0`,
`two writers: splices=273 contended_writes=106/200`; 4 passed, 1 ignored.

Mutants (marker `V1S-MUTANT`, local, on the fix, `cargo test --test driverctl_race
editor_race_smoke`, each reverted from a saved copy; `grep -rn V1S-MUTANT src tests` empty):
- M-B, `run_loop` returns GaveUp before writing any temp: failed 3/3 with
  `rename: the tool must have spliced at least once: {"gave-up": 36835}` (36450, 37071).
- M-C, the verify treats every displaced file as matching: failed 3/3 with
  `a rename-save must never be lost: the exchange-verify gives the displaced inode back`.

mutants: M-B 3/3 M-C 3/3

## Scan of race/timing tests

Patterns: **P-a** a progress / non-vacuity assertion on work that must complete within a
fixed wall-clock window while racing a competitor; **P-b** a fixed sleep or gap that must
outlast or undercut I/O-bound work (fsync, flock hold, child spawn); **P-c** a wall-clock
upper bound on I/O-bound work. Prior evidence: qs8's 1 200 `driverctl_race` binary runs
(whose only `other` failures were `editor_race_smoke`) and this task's 600 BEFORE + 600 AFTER
binary runs of the same binary (every test in it runs each iteration). Line numbers are at
the fix commit for tests/driverctl_race.rs and at 08fc9f7 elsewhere (unchanged files).

| file | test | pattern | evidence (file:line) | verdict |
|------|------|---------|----------------------|---------|
| tests/driverctl_race.rs | editor_race_smoke | P-a (`ok > 0` in 1.0 s vs an fsync window) | the yield + `ok`/`contended` asserts in `assert_exchange_holds` | FIXED here (F1) and its product race F2 fixed in src |
| tests/driverctl_race.rs | editor_race_full (ignored) | P-a (same `ok > 0`), in-place Poisson bound (statistical, 15 s) | `assert_exchange_holds(15.0, true)` | covered by the same fix; scanned empirically (`--ignored`, exact, profile 0, n=5) |
| tests/driverctl_race.rs | editor_race_naive_positive_control | P-a (LOST > 0 within 4 x 0.5 s) | naive LOST 10 per case in every M1/M2 case (max permitted by the human's single read-back) | not flaky: 0 failures in 1 200 qs8 + 1 200 v1s binary runs |
| tests/driverctl_race.rs | two_writers_under_direction_lock_lose_nothing | was P-b (100 us gap vs fsync hold) | handshake since ffb2914 (qs8) | not flaky since qs8: 0 failures in 1 200 v1s binary runs |
| tests/driverctl_race.rs | poisson_q99_matches_known_values | none (pure) | - | not flagged |
| tests/driverctl_lock_race.rs | single_attempt_is_refused_under_a_probe_hold | **P-b**: the single `acquire_at` must land inside a 30 ms LOCK_SH hold that starts before it | :68-70 | FLAGGED, measured |
| tests/driverctl_lock_race.rs | retrying_acquire_wins_under_a_probe_hold | P-b in the safe direction (500 ms budget outlasts a 30 ms hold) | :89-91 | flagged with the file (same binary) |
| tests/driverctl_lock_race.rs | stress_no_false_refusal_with_retry_and_some_without | **P-a**: the no-retry arm needs > 0 refusals in 2 000 cycles against two polling probers | :128-150 | FLAGGED, measured (same binary) |
| tests/driverctl_lock_race.rs | real_holder_is_still_refused_within_budget | **P-b**: an 800 ms hold must outlast a 500 ms budget started after the hold (300 ms margin) | :160-170 | FLAGGED, measured (same binary) |
| tests/driverctl_lock_race.rs | concurrent_probes_never_see_each_other | none: an early hold release only makes the expected `Free` more likely | :201 | not flagged |
| tests/driver_lock.rs | a_duplicate_start_refuses_promptly_rather_than_blocking | **P-c**: a whole in-process `drive` (config, envelope, then LOCK_NB) must refuse within 1 s | :443 | FLAGGED, measured |
| tests/driver_lock.rs | acquiring_the_run_lock_does_not_block_the_async_runtime | P-c, 5 s for the same refusal | :495 | flagged with the file |
| tests/driver_lock.rs | a_child_that_dies_before_taking_the_lock_... | P-c, a child refused at registry lookup reported < 10 s | :701 | flagged with the file |
| tests/driver_lock.rs | other 5 tests | 30 s polls for the holder record (:222, :329) | - | not flagged (30 s vs ms) |
| tests/driverctl_store.rs | the_run_lock_is_held_until_the_store_drops | 5 ms retry against a lock held for the whole call | :878-884 | not flagged: the holder never releases during the retry |
| tests/driverctl_store.rs | cr01_scan_tick_survives_a_fifo_... | P-c, 30 s for one `scan_tick` | :1307 | not flagged (30 s vs ms) |
| tests/driverctl_store.rs | the rest (heartbeat/coalescing tests) | an injected clock (`open_clocked`, :927), not wall time | - | not flagged |
| tests/driverctl_serve.rs | gap_scan_tick_observes_verification_writes | **P-a**: a `gap-observed` line within 5 s from a serve whose scan TICK is 2 s (serve.rs:114), with a spawned server under load | :475-486 | FLAGGED, measured |
| tests/driverctl_serve.rs | second_serve_is_refused_and_does_not_steal_the_socket | P-c, a refused child serve < 5 s | :352-363 | flagged with the file |
| tests/driverctl_serve.rs | stalled_connection_does_not_block_the_gate / a_flood_of_idle_connections_... | P-b: a 200/500 ms sleep then one client, inside CONN_TIMEOUT = 10 s (serve.rs:111), client deadline 15 s (driverctl_cli.rs:60) | :773, :802 | flagged with the file (>= 19x margin) |
| tests/driverctl_serve.rs | the rest | `wait_exit(10 s)` / `wait_listening(10 s)` hang guards | :119-133, :186-203 | flagged with the file |
| tests/executor_lifecycle.rs | a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap | **P-b**: 20 heartbeats from a `sleep 0.05`-paced shell child must each arrive within a 400 ms idle cap | :486-521 | FLAGGED, measured |
| tests/executor_lifecycle.rs | the other 13 | P-c with >= 3x margins on 400 ms..10 s caps, lower bounds on deterministic timers, 15-60 s hang guards | :313-319, :377, :422-428, :473, :606, :648, :704, :903 | flagged with the file |
| tests/driver_kill_startup.rs | a_stop_during_agent_startup_tears_down_... | **P-c**: a stop during agent startup must finish below DRIVER_TEARDOWN_GRACE = 12 s (kill.rs:57) | :419 | FLAGGED, measured |
| tests/driver_kill.rs | 3 tests | 15 s settle sleep then absence checks; 10-30 s polls | :58-61, :449, :599 | not flagged: the sleeps wait FOR something to be gone, longer only helps |
| tests/driver_reattach.rs | 3 tests (1 ignored) | 30 s polls; a 500 ms sleep before asserting the run is still ALIVE | :524, :547 | not flagged: a longer stall only makes "still alive" more certain |
| tests/journal_crash.rs | 3 tests | 20 s for the writer child to produce PRODUCING_BYTES, then SIGKILL | :113-131 | not flagged (20 s vs ms; the produced-size check is what guards non-vacuity) |
| tests/bounded_output.rs | 2 tests | P-c: a 300 ms collection window must return < 3-5 s | :19-25, :66-75 | not flagged (>= 10x) |
| tests/driverctl_reconcile.rs, tests/driverctl_replay.rs | 1 poll each | 10 s poll for a log line | :138-154, :150-163 | not flagged (10 s vs ms) |
| tests/driver_iteration_loop.rs | the_step_cap_... / CR-01 run-cap test | P-c: a 1 s run cap must end the run < 15 s | :531-560 | not flagged (15x) |
| tests/launcher_e2e.rs, tests/launcher_degraded.rs, tests/launcher_caps.rs | refusal < 5-10 s; a >= 10 s lower bound; a 4.5 s sleep before a negative "no new round" check | :740-746, :958; :393-399; :229 | not flagged (generous P-c; a lower bound; a sleep whose overrun cannot fail the check) |
| tests/envelope_credential.rs, tests/envelope_tracer.rs | 30 s fail-closed budget; deadline poll | :353-384; :250-276 | not flagged (30 s vs ms) |
| tests/envelope_carrier_reach.rs, tests/envelope_interior_path.rs, tests/envelope_word_set.rs | guard-latency bounds (< 5 s vs measured 29-420 ms) and a linearity RATIO (documented to fall, not rise, under load) | word_set :1670-1745; interior_path :2008; carrier_reach :1879 | not flagged: CPU-bound guard work, not lock/fsync; >= 10x margins measured in-file |
| tests/driverctl_client.rs, tests/driver_inbox.rs, tests/executor_transport.rs, tests/launcher_config_dir.rs, tests/launcher_live_scope.rs | 5 s socket read timeout in a fake server; 25 ms poll sleeps; 30 s hang guard; live systemd (skips without it) | client :64-80; inbox :225; transport :308 | not flagged |
| tests/async_blocking_guard.rs, driver_goal_seam.rs, envelope_command_position.rs, envelope_pr_cap.rs, envelope_wrapper_bypass.rs, envelope_wrapper_class.rs, launcher_caps (rest), launcher_live_isolation.rs, prompt_acceptance.rs, state_reader_test.rs | grep hit on `timeout`/`deadline` as a word or a constant, no wall-clock race | - | not flagged |

### Empirical scan (label `scan`, profile A, binary unless noted, base tree 3222d97+e3b0c44298fc = the fix commit's code)

```
scan profile=0 mode=exact  n=5   fail=0 other=0 timeout=0 ... target=driverctl_race filter=editor_race_full extra='--ignored'
scan profile=A mode=binary n=200 fail=0 other=0 timeout=0 ... target=driverctl_lock_race filter=single_attempt_is_refused_under_a_probe_hold
scan profile=A mode=binary n=200 fail=0 other=3 timeout=0 ... target=driverctl_serve filter=gap_scan_tick_observes_verification_writes
scan profile=A mode=binary n=200 fail=0 other=0 timeout=0 ... target=driver_kill_startup filter=a_stop_during_agent_startup_tears_down_...
scan profile=A mode=binary n=200 fail=0 other=1 timeout=0 ... target=driver_lock filter=a_duplicate_start_refuses_promptly_rather_than_blocking
scan profile=A mode=binary n=100 fail=0 other=0 timeout=0 ... target=executor_lifecycle filter=a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap
```

`executor_lifecycle` takes ~25 s per binary run (host), so its n was reduced to the 100
floor `[INFERRED — audit]`. Every flagged test passed every run. Two tests NOT flagged by the
static scan failed in the `other` column, so the static scan under-flagged them; both are
treated as shown flaky:

- **S1, `driverctl_serve::a_flood_of_idle_connections_is_answered_busy_and_the_gate_denies`,
  3/200 (iterations 1, 132, 182).** All three: `the deny reason carries the server's answer:
  ... "LAUNCHER CAP: driverctl unreachable (send request: Broken pipe (os error 32)); ..."`
  (tests/driverctl_serve.rs:811). The static scan read this test's 500 ms sleep as a safe
  P-b; the failure is elsewhere (below).
- **S2, `driver_lock::the_lock_is_released_when_the_holding_process_dies`, 1/200
  (iteration 35):** `a SIGKILLed holder releases the lock by dying ...: HeldBy { run_id:
  "2026-07-29T12-00-00Z-aaaa", pgid: 8025, ... }` (tests/driver_lock.rs:606): the single
  `lock::acquire` straight after `child.kill(); child.wait()` found the lock still held,
  naming the just-reaped driver.

### S1: the flood test, cause and fix (deviation: src/driverctl_cli.rs)

**Measurement.** The three scan failures and the BEFORE run below all carry the same reason,
`send request: Broken pipe (os error 32)`: the CLIENT's write failed, not its read. Code:
`refuse_busy` (src/driverctl/serve.rs:772-777) "answer[s] `busy` without reading the request
and close[s]", while the client (src/driverctl_cli.rs `request`, :236-268 at base) wrote its
request and turned any write error into `Transport { "send request: ..." }` before reading
anything. On a unix stream socket a write to a closed peer fails with EPIPE even though the
peer's reply is already queued for reading, so whenever the 33rd connection's `busy` answer
and close beat the client's write (a scheduling race, only seen under profile A's load), the
queued `busy` was discarded and the gate denied with the "unreachable" reason.

**Causal test (deterministic, red on base).** `request` was split, with no behaviour change,
into `send_and_read` + `read_reply`, and a socketpair unit test writes `{"ok":false,
"error":"busy"}\n` from the server end and closes it BEFORE the client sends. On the base
logic: `a_reply_queued_before_the_peer_closed_is_read_even_though_the_send_fails ...
FAILED: Transport { detail: "send request: Broken pipe (os error 32)" }`, the exact field
error. With the fix (after a BrokenPipe/ConnectionReset send failure, read; relay a complete
reply if there is one, else the send failure stands): 4/4 pass, and the control
`a_send_to_a_peer_that_closed_without_a_reply_is_still_a_send_failure` passes both before
and after.

**Verdict: product defect** (client robustness; fail-closed was preserved, the reason was
wrong). No assertion, budget or timeout in the test changed.

```
scan profile=A mode=binary n=200 fail=1 other=0 timeout=0 tree=3222d97+e3b0c44298fc ... filter=a_flood_of_idle_connections_is_answered_busy_and_the_gate_denies note=flood-before
scan profile=0 mode=binary n=200 fail=0 other=0 timeout=0 tree=3222d97+e3b0c44298fc ... note=flood-before
scan profile=A mode=binary n=200 fail=0 other=0 timeout=0 tree=3222d97+c669389732b4 ... note=flood-after
scan profile=0 mode=binary n=200 fail=0 other=0 timeout=0 tree=3222d97+c669389732b4 ... note=flood-after
```
Before: 4/400 at profile A (3 in the scan line's `other` column + 1), 0/200 at profile 0.
After: 0/200 at A and 0/200 at 0. Tree suffix `c669389732b4` = the diff of fix commit
**801c656** (`fix(quick-261023-v1s): relay a reply queued before the server closed when the
send fails`), recomputed just before the commit.

### S2: the SIGKILL lock-release test, cause and fix (deviation: tests/driver_lock.rs)

**Measurement (probe `V1S-PROBE-S2-holders`, temporary, reverted).** On a refused acquire,
the probe listed every live process with a descriptor on the lock file (`/proc/*/fd`), then
polled until the lock freed. `exact` mode, profile B, n=1000:

```
probe profile=B mode=exact n=1000 fail=6 other=0 timeout=0 tree=801c656+ea6f74899566 ... target=driver_lock filter=the_lock_is_released_when_the_holding_process_dies note=V1S-PROBE-S2-holders
```

| failure | refusal names            | live fd holders at the scan                                                                                     | lock free after |
|---------|--------------------------|-----------------------------------------------------------------------------------------------------------------|-----------------|
| 1       | killed driver pid 2723   | none (already gone)                                                                                             | 80 us           |
| 2       | killed driver pid 4964   | none                                                                                                            | 86 us           |
| 3       | killed driver pid 8182   | pid 8194, comm `tokio-rt-worker`, ppid 1 (re-parented), pgrp 8182, exe `gsd-meta-manager`, the DRIVER's cmdline | 1.82 ms         |
| 4       | killed driver pid 10601  | none                                                                                                            | 77 us           |
| 5       | killed driver pid 11688  | pid 11700, comm `tokio-rt-worker`, ppid 1, exe `gsd-meta-manager`, the driver's cmdline                         | 81 us           |
| 6       | killed driver pid 14604  | none                                                                                                            | 3.87 ms         |

The holder is a child FORKED by the driver (from a tokio worker thread) that has not yet
exec'd: it still runs the driver's image and command line. A fork copies the descriptor
table, so it shares the lock's open file description until its exec closes the O_CLOEXEC fd,
77 us to 3.9 ms later. The test kills the driver as soon as the holder record appears,
which is exactly when the driver forks its `claude` stand-in, and its single `lock::acquire`
straight after the reap then sometimes lands inside that window.

**Verdict: not a product defect; a test timing assumption (P-b).** flock (D-19) is released
when the last reference to the description closes; the kernel did release it within 3.9 ms
every time, with no process left holding it. The window is inherent to fork-then-exec of any
child while holding a flock (POSIX record locks would avoid it, but switching lock type is an
architectural change, Rule 4, out of scope; recorded here as a note, not a defect).

**Fix (test-side).** A first candidate, retrying the recovering acquire within
ACQUIRE_RETRY_BUDGET (500 ms), was REJECTED by a mutant: with the lock fd's FD_CLOEXEC
cleared (M-S2, `fcntl_setfd(.., empty)` in `acquire_at`, marker `V1S-MUTANT`) the ORIGINAL
test fails 3/3 but the retrying one passed 3/3, i.e. the retry hid a real descriptor leak.
The shipped fix keeps the single acquire and its message untouched and adds a condition, not
a margin: before the SIGKILL the test waits (bounded, 600 x 50 ms, reporting an early child
exit with its output, like `wait_for_lock_or_report`) until the run's journal carries an
`exec_started` record, i.e. the stand-in has been spawned, exec'd and spoken. The fork window
has then closed, and the orphaned stand-in really exists at the kill, so a leaked descriptor
now fails the acquire. Mutant M-S2 against the fix: failed 3/3 (`a SIGKILLed holder releases
the lock by dying ...`); reverted, `grep -rn V1S-MUTANT src tests` empty. Local: driver_lock
8/8 pass.

```
scan  profile=A mode=binary n=200  fail=0 other=1 timeout=0 tree=3222d97+e3b0c44298fc ... filter=a_duplicate_start_refuses_promptly_rather_than_blocking   (the scan line; S2 in `other`)
scan  profile=0 mode=binary n=200  fail=0 other=0 timeout=0 tree=801c656+e3b0c44298fc ... filter=the_lock_is_released_when_the_holding_process_dies note=sigkill-before
probe profile=B mode=exact  n=1000 fail=6 other=0 timeout=0 tree=801c656+ea6f74899566 ... note=V1S-PROBE-S2-holders   (probe only changes the failure path)
scan  profile=B mode=exact  n=1000 fail=0 other=0 timeout=0 tree=801c656+ac6881601251 ... note=sigkill-after
scan  profile=A mode=binary n=200  fail=0 other=0 timeout=0 tree=801c656+ac6881601251 ... note=sigkill-after
scan  profile=0 mode=binary n=200  fail=0 other=0 timeout=0 tree=801c656+ac6881601251 ... note=sigkill-after
```
Before: A binary 1/200, 0 binary 0/200, B exact 6/1000. After: 0/200, 0/200, 0/1000. The B
exact BEFORE is the probe run `[INFERRED — audit]`: its first acquire and failure test are
the original code (the probe only adds diagnostics after a refusal). Tree suffix
`ac6881601251` = the diff of fix commit **d960678** (`fix(quick-261023-v1s): kill the lock
holder only after its agent fork has exec'd`), recomputed just before the commit.

### Scan summary

| target (flagged or shown flaky) | runs | failures | verdict |
|---------------------------------|------|----------|---------|
| editor_race_full (`--ignored`, exact, profile 0) | 5 | 0 | passes with the fix |
| driverctl_lock_race (binary A) | 200 | 0 | not flaky |
| driverctl_serve gap_scan_tick (binary A) | 200 | 0 (target) | not flaky; S1 surfaced in `other` |
| driver_kill_startup (binary A) | 200 | 0 | not flaky |
| driver_lock duplicate_start (binary A) | 200 | 0 (target) | not flaky; S2 surfaced in `other` |
| executor_lifecycle keeps_emitting (binary A, n reduced to 100) | 100 | 0 | not flaky |
| **S1** driverctl_serve flood test | 600 before / 400 after | 4 before -> 0 after | FIXED, product (src/driverctl_cli.rs, 801c656) |
| **S2** driver_lock SIGKILL test | 1 400 / 1 400 | 7 before -> 0 after | FIXED, test-side (tests/driver_lock.rs, d960678) |

No open findings. Note for the static scan: both shown-flaky tests were NOT flagged by the
static patterns (S1 looked like a safe P-b sleep; S2's acquire-after-reap is a P-b gap of
zero against an unlisted competitor, a forked child), so binary-mode `other` columns were
what caught them.

## Release and local gates (final tree d960678, src/tests diff clean)

`./scripts/pre-tag-check.sh --container v1.15.0`, run alone, after every stress run:

```
=== PRE-TAG SUMMARY ===
  gate 1  tag / Cargo.toml version   PASS
  gate 2  MSRV check                 PASS
  gate 3  cargo build --release      PASS
  gate 4  cargo test                 FAILED      (see gate4-test.log)
  gate 5  cargo clippy -D warnings   PASS
  gate 6  cargo package --locked     PASS
```

pretag_container: FAIL 5/6 head=d96067862080ba4db5fb41d6bb224f45929efa3b — BLOCKER outside this task: gate 4's only failure is the git-version witness, because the image this run rebuilt now carries git 2.56.0

Gate 4 ran `cargo test --no-fail-fast` (89 test results); exactly one test failed:
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
(`left: "git version 2.56.0"`, `right: "git version 2.55.0"`). The container banner read
**MISMATCH** (installed 2.56.0 vs constant 2.55.0). The pre-tag run rebuilt
`gsdmm-pre-tag-check:latest` (created 2026-10-24T08:11Z, `docker run --rm
gsdmm-pre-tag-check:latest git --version` = `git version 2.56.0`): `ppa:git-core/ppa` has
moved from 2.55.0 (the version scripts/pre-tag-check.Dockerfile:19-22 documents) to 2.56.0.
Per CLAUDE.md, a container MISMATCH "means the Dockerfile's git has drifted from the constant
and the image needs fixing", and the witness itself says the correct response is to re-derive
the constants against the new git. That is a release-engineering decision (and the runner's
ambient git is the authority for it), independent of this task's code; it was NOT done here
`[INFERRED — audit]`. Every other test in the container, including all of driverctl_race,
driverctl_serve and driver_lock, passed.

Local gates, sequentially after the pre-tag run (host git 2.53.0):
- `rtk proxy cargo test --no-fail-fast`: 89 test results; the only failure is
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: rc 0, 0 warnings.
- `cargo fmt --check`: clean.

local_gates: full-suite only-witness clippy 0-warnings fmt clean

Integrity: `git rev-parse 'v1.15.0^{commit}'` = 08fc9f7286f15a79c8819582f916c28becf54056 (=
`tag_v1_15_0`); `git status -sb` = `master...origin/master [ahead 16]` (nothing pushed; the
16 include the concurrent session's docs commits); `git worktree list` = the main tree only;
branches `dev`, `master` only; no gsdmm container left running (every harness and pre-tag
container is `--rm`).

