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

<!-- v1s:continue -->
