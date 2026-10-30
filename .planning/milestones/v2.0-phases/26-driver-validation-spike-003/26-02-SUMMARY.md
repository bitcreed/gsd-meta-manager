---
phase: 26-driver-validation-spike-003
plan: 02
subsystem: spike-tooling
tags: [spike, measurement, pre-registration, dup-check, stage-cost, acceptance, stdlib-python]
requires:
  - phase: 26-driver-validation-spike-003 plan 01 (run.sh / results / drift.log contract only; no output of 26-01 is read by the tools)
provides:
  - tools/dup_check.py (VALID-03 duplicate detector, exit 0/1/2)
  - tools/stage_cost.py (VALID-04 per-stage, per-phase, total cost; --rates-from)
  - experiment/acceptance.py (modes l7, run1, run2, rubric-check, selftest)
  - README.md with the rubric frozen between the rubric markers (anti-HARKing)
affects: [26-03]
tech-stack:
  added: []
  patterns: [stdlib-only python run as python3 -I, selftest subcommand with tempdir fixtures, planted-marker privacy assertion]
key-files:
  created:
    - .planning/spikes/003-divergent-gap-loop-validation/tools/dup_check.py
    - .planning/spikes/003-divergent-gap-loop-validation/tools/stage_cost.py
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py
    - .planning/spikes/003-divergent-gap-loop-validation/README.md
  modified: []
key-decisions:
  - "README commit 4ba61955a3a847a5005af2ec17d6771479d5f0ed is the commit 26-03 records as PREREG_SHA"
  - "A2 passes on a park at or before the third gaps_found [inferred]"
  - "SEED VALID requires A10 not FAIL; otherwise STALLED [inferred]"
requirements-completed: [VALID-01, VALID-02, VALID-03, VALID-04]
status: complete
commits: 4
plan_head_before: ac5d40c086d1373cd2fa4061aa5312973e473052
plan_head_after: 4ba61955a3a847a5005af2ec17d6771479d5f0ed
actuals:
  tokens: 26300   # chars/4 over the four files as committed (105152 chars); not a harness token count
  tasks: 3
  commits: 4
---

# Phase 26 Plan 02: Measurement layer and pre-registered rubric Summary

Three stdlib tools turn a run's files into duplicate, cost and PASS/FAIL evidence without reading any transcript text into the repo, and the spike README freezes the GO/ADJUST/NO-GO/INCONCLUSIVE rubric in a committed block before any Opus launch.

Requirements marked complete (VALID-01..04) are the measurement and pre-registration half; the runs that produce the evidence are plan 26-03.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | 534c349 | feat(26-02): measurement tracer - dup_check, stage_cost and acceptance run1 on a synthetic launch |
| 2 RED | 5daadf1 | test(26-02): selftest cases for l7, run2, A8-A10, rubric-check and spike-002 real logs |
| 2 GREEN | bfba0b6 | feat(26-02): l7/run2 checks, findings non-subset, gap-count sanity, ordering and rubric-check |
| 3 | 4ba6195 | docs(26-02): spike 003 README with pre-registered checks, rubric and routing |

**PREREG_SHA for 26-03:** `4ba61955a3a847a5005af2ec17d6771479d5f0ed` (`git log -1 --format=%H -- .planning/spikes/003-divergent-gap-loop-validation/README.md`). `acceptance.py rubric-check --sha HEAD` printed `RUBRIC-UNCHANGED` against it.

## What was built

- **dup_check.py** rebuilds each session's agent tree from `agent-*.meta.json`. DUP is a second `gsd-planner` or `gsd-verifier`, a second same-description `gsd-executor`, or a top-level worker beyond logged dispatches minus denies. A verifier without an executor is `EXECUTOR_MISSING` (reported, not a duplicate). A LOG with no I-item `dispatched` line (L7 shape) prints `driver_check=skipped`. A dispatched session with no metadata exits 2.
- **stage_cost.py** buckets driver messages by their own timestamp and each worker's whole subtree by the worker's first timestamp into LOG windows (`startup`, `{plan|execute|step}-r{N}`, `close`, `unbucketed`). It keeps the last usage per `message.id`, solves per-model rates from the first launch with a `result` event (or `--rates-from`), and its `CHECK` line compares the per-stage driver sum with an independently summed driver total.
- **acceptance.py** parses the pipe grammar (comment block ignored, report bodies attached to their events), imports spike 002's `driverctl` in place for `split_frontmatter`, and implements L7-1..L7-6, A1-A4, A5-deny, A5-kill, A6-A11, SEED, `rubric-check` and `selftest`.
- **README.md** carries the frontmatter (eight keys, `verdict: PENDING`), How to Run, and the pre-registered block: every check with its pass condition, SEED classes, retry rule, the $60 campaign ceiling, the rubric with J1-J5, the verdict-to-frontmatter mapping and the routing rule.

## Verification

- All three `selftest` subcommands print `selftest OK`.
- Tracer: a synthetic Run-1 launch (six dispatches, six workers with planner/executor/verifier children, a launch stream with a priced `result`) flows through `dup_check.py` (DUP=0), `stage_cost.py` (`CHECK ... ok`; TOTAL equals the fixture's priced total within $0.01), and `acceptance.py run1`: A1, A2, A3, A4, A6, A7 PASS, `SEED VALID`. Appending a `round=3` dispatched line makes A1 FAIL and the exit non-zero.
- The planted marker `PLANTED-SECRET-c0ffee` (in driver and worker transcript messages) appears in no tool output.
- Spike 002's committed `L3-LOG.md`, `L4-LOG.md` and `R2-hidden-fixture/LOG.md` parse from their real paths (deny after `count=3`; `cap-tripped used=4 limit=2`; multi-line report bodies attached).
- Beyond the plan: `acceptance.py l7` on 26-01's real `results/l7` prints L7-1..L7-6 all PASS. `acceptance.py run1` on `results/smoke1` (a Haiku dispatch-cap-0 smoke, not a real run) prints A1 FAIL, A3 FAIL, A4 FAIL, `SEED RECOGNISED`, i.e. the checks fail in the right direction on real data. Neither output was written to a results file (26-03 owns `acceptance.txt`).
- `git diff --exit-code a16e338 -- .planning/spikes/002-launcher-caps-and-tool-frontmatter` exits 0; no `src/`, `tests/`, `Cargo.toml` or `Cargo.lock` change since the plan base; `git grep "import driverctl"` prints one line and no `driverctl.py` exists under spike 003.
- README gates: markers present once each, `verdict: PENDING`, all check, ADJUST and class ids present, and the only `$digit` match is the $60 ceiling line.

### A8 break-and-revert (acceptance criterion)

Replacing the A8 staleness test (`stale = [k + 1 for k in range(1, len(sets)) if not (sets[k] - sets[k - 1])]`) with `stale = []` (every snapshot treated as new) made the selftest fail: the `a8fail` case, where snapshot 3 repeats snapshot 2's cases, no longer produced A8 FAIL. The file was restored from a saved copy and the selftest returned to `selftest OK`; the committed `bfba0b6` contains the unbroken comparison.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] dup_check selftest fixture contradicted its own contract**
- **Found during:** Task 1
- **Issue:** the "denied dispatch" case had 1 dispatched + 1 denied + 1 worker, which by the contract (workers beyond dispatched minus denied) is a DUP. A denied attempt is itself a logged dispatch, so the fixture was wrong, not the tool.
- **Fix:** fixture now logs 2 dispatched, 1 denied, 1 worker (0 duplicates).
- **Files modified:** `tools/dup_check.py` (selftest only), same commit as Task 1.

**2. [Rule 1 - Bug] stage_cost per-phase split and dead branches**
- **Found during:** Task 1
- **Issue:** first draft bucketed by stage name, so two phases sharing a stage name (`plan-r0`) would have split PER_PHASE cost by window count, and it carried unused branches.
- **Fix:** bucket per window, aggregate by name only for printing; PER_PHASE sums the windows' own buckets.
- **Files modified:** `tools/stage_cost.py`, same commit as Task 1.

**3. [Rule 3 - Blocking] avoid `__pycache__` inside spike 002's tree**
- **Found during:** Task 1
- **Issue:** importing `driverctl` from spike 002 would write bytecode into a tree this plan must leave unchanged.
- **Fix:** `sys.dont_write_bytecode = True` before the import. One `__pycache__` that an ad-hoc import of `acceptance` created under spike 003 `experiment/` was removed before committing.

### Notes on TDD

Task 2's RED commit (5daadf1) failed as intended for l7, run2, A8-A10, A11 and rubric-check. The dup_check L7-shape and re-spawn cases and the stage_cost `--rates-from` / `priced=no` cases already passed at RED: Task 1's action text had put those behaviours in the tools, so Task 2 only added their selftest coverage there (they live in each tool's own selftest).

## [inferred] decisions

- **A2** passes when I-01 parked, no fourth `gaps_found` was seen, no dispatch followed `count=3` and the park followed `count=3` if it exists; a park before the third `gaps_found` also passes A2 (it then fails A1 and lands in J5).
- **A5-deny** is NOT-REACHED only with no deny, I-01 parked, and no dispatch of round 3 or higher; any other no-deny outcome is FAIL.
- **A7** with `DUP=0` and zero workers is UNDETERMINED, not FAIL, so a launch that spawned nothing cannot trigger NO-GO (c).
- **A9** with no `gap-observed` and no verifier is NOT-REACHED; the stand-in subtraction uses the number of `standin.log` writes (one in the current `run.sh`).
- **A11** accepts a `dispatch-denied` directly before the `dispatched` line as well as after it, since the log order of the driver's attempt versus the hook's deny is not fixed; UNDETERMINED when the log ends with neither a deny nor a report.
- **Stand-in snapshot labelling** (A8, A10): the first `gaps_found` snapshot line at or after each `standin.log` write is treated as the stand-in's and excluded.
- **SEED final:** VALID requires A10 not FAIL; otherwise STALLED. A snapshot with `status=passed` or I-01 `state=done` is CONVERGED, evaluated after RECOGNISED, in that order.
- **A8 case extraction** uses `tools/drift.py` (lazy import via `importlib`, `emit(MAX_GEN)` titles and slugs matched on word boundaries, plus `line N` references) and falls back to quoted non-path spans, printing `mode=drift` or `mode=generic`.
- **rubric-check** exits 2 (not 0, not 1) when no sha is available or `git show` fails, so a missing `prereg.txt` cannot read as unchanged.
- **Window convention (stage_cost):** a `launch-end` closes its window one second late because LOG timestamps have one-second resolution; `end` and `dispatched` boundaries are exclusive on the earlier window.

## Known Stubs

None. The README Results section is intentionally empty until 26-03 (stated in the README).

## Threat Flags

None. No network endpoint, auth path or schema was added; the tools read agent metadata, usage and tool_use names only (T-26-08 mitigated and asserted by every selftest), and an empty or missing input yields exit 2 or UNDETERMINED (T-26-11).

## Issues Encountered

- Spend: none (synthetic fixtures and committed logs only).
- The `tool_use` count in `dup_check.py` uses block ids when present; a transcript lacking ids falls back to file:line keys.
- Wall-clock: the four commits span a few minutes by git timestamps; the executor did not record a precise start time.

## Self-Check: PASSED

- Files present: `tools/dup_check.py`, `tools/stage_cost.py`, `experiment/acceptance.py`, `README.md` under `.planning/spikes/003-divergent-gap-loop-validation/`.
- Commits reachable from HEAD: 534c349, 5daadf1, bfba0b6, 4ba6195.
- Measured `commits: 4` from `rev-list --count ac5d40c..HEAD` at SUMMARY write time.
