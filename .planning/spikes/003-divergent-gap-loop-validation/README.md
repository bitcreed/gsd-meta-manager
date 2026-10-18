---
spike: 003
idea: queue-driven-master-agent
name: divergent-gap-loop-validation
type: standard
validates: "Given the unmodified spike-002 launcher and driverctl and a v0.3 Opus driver on a real GSD scratch project whose verifier criterion tests a feed that advances after every plan SUMMARY commit, when it runs gap closure, then it dispatches rounds 1 and 2, parks needs-human with rule divergence at the third gaps_found without any launcher deny, a prompt-rule-disabled run is stopped by the launcher gap cap at L+1/L+2, no planner or executor runs twice, and the Opus driver cost per stage and per phase is measured"
verdict: PARTIAL
related: [001, 002]
tags: [driver, gap-loop, divergence, async-agent, launcher-caps, cost, bwrap]
---

# Spike 003: The divergent gap-closure loop, validated end to end

## What This Validates

Spike 002 proved the launcher's caps and the tool-written log on a project whose planted gaps were
recognised as unclosable on sight: in every encounter the Opus planner or driver read the contradiction and
parked `needs-human` before any gap-closure round ran, so the loop itself (verify, `gaps_found`, plan
`--gaps`, execute `--gaps-only`, verify again) was never exercised. This spike builds a target that moves
where the model cannot read it, and drives the unmodified spike-002 mechanism through the loop with a real
Opus driver.

**VALID-01 (the loop runs and the driver stops it).** On a scratch project whose verifier criterion tests a
host-fed feed that gains one new house-rule class after every plan SUMMARY commit, the v0.3 Opus driver
dispatches gap-closure rounds 1 and 2 and, at the third `gaps_found`, parks `needs-human`. The launcher
never denies a dispatch in that run.

**VALID-02 (divergence is named, not masked).** The park is recorded with `rule=divergence` by the
tool-written log, and the findings of each `gaps_found` differ from the previous one (the target moved), so
the park is not a hidden `gap-rounds` stop.

**VALID-03 (no duplicate work).** No planner and no verifier runs twice under one worker, no executor with the
same description runs twice, and the driver spawns no worker it did not log. Measured from the worker tree
(`agent-*.meta.json`), not from prose.

**VALID-04 (cost is measured).** The Opus driver cost, and the worker cost, are measured per stage and per
driven GSD phase from transcripts and the tool-written log windows, including a launch that was killed
before it produced a `result` event. Costs are list-price equivalents under a subscription.

The launcher's hard caps are also exercised: a run whose prompt rule is deliberately removed must be stopped
by the launcher (deny at L+1, kill at L+2), with the kill shown by a labelled stand-in and by the scripted
L7 run because a compliant driver never reaches L+2 on its own.

## Artifacts

All paths relative to `.planning/spikes/003-divergent-gap-loop-validation/`. Spike 002 is executed in place
and never edited or copied.

| File | Purpose |
|------|---------|
| `README.md` | this file: frontmatter, how to run, the pre-registered checks and rubric, results after the runs |
| `MASTER-PROMPT.md` | driver prompt v0.3: spike 002's v0.2 plus the gap-round, divergence and precedence clauses |
| `experiment/MASTER-PROMPT.run2.md` | generated Run-2 prompt v0.3-run2 (gap-round rules removed by asserted removal) |
| `templates/DIRECTION.run1.md` | Run 1 direction: item budget with gap-rounds=2 and the standing gap-round rule |
| `templates/DIRECTION.run2.md` | Run 2 direction: gap-round rule and budget key removed |
| `tools/drift.py` | host-side feed watcher: `emit N`, `watch`, `selftest`; advances the feed per SUMMARY commit and snapshots each verification |
| `tools/make_variant.py` | generates the Run-2 prompt and direction; `--check` fails on drift |
| `tools/dup_check.py` | VALID-03 duplicate detector over the worker tree; exit 0 (no duplicate), 1 (duplicate), 2 (no data) |
| `tools/stage_cost.py` | VALID-04 per-stage, per-phase and total cost from transcripts and LOG windows; `--rates-from` prices a killed launch |
| `experiment/setup-scratch.sh` | builds the scratch GSD project outside this repo and initialises the run |
| `experiment/run.sh` | one entry point per mode: `l7`, `smoke1`, `smoke2`, `run1`, `run2` |
| `experiment/cap-tests/L7-drift-gap-cap.md` | scripted Haiku driver for the deny-then-kill run (L7) |
| `experiment/acceptance.py` | mechanical checks over a run's results: modes `l7`, `run1`, `run2`, plus `rubric-check` and `selftest` |
| `experiment/results/<label>/` | derived outputs only: LOG, state, direction, drift log, verification snapshots, reports (never transcripts) |
| `experiment/results/prereg.txt` | written by plan 26-03: `PREREG_SHA=` of the commit that froze this README's rubric |

## How to Run

Every command below lives in a bash script or is run through `bash` (the interactive shell is fish).

```bash
# one launch; first stdout line is SCRATCH_ROOT=<abs path>; touches <SCRATCH_ROOT>/DONE on exit
bash .planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh <mode>
#   mode: l7 | smoke1 | smoke2 | run1 | run2
#   env:  SCRATCH_ROOT (default under ${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/spike-003/)
#         RESULTS_LABEL (default: the mode; the run id becomes s003-<label>)
#         MODEL, RELAUNCH=1 (reuse the label's recorded scratch root)
# exit: 0 ran to completion; 2 bad mode or scratch root inside a git work tree; 5 host token too short;
#       6 secret gate hit; 7 make_variant.py --check drift
```

Run 1 and Run 2 launch with `--max-budget-usd 15`. Spike 002's `cost_split.py` also runs once per launch
(`cost-launch-<n>.txt`). Then, per label, from the repository root:

```bash
S3=.planning/spikes/003-divergent-gap-loop-validation
R=$S3/experiment/results/<label>
python3 -I $S3/tools/dup_check.py <STATE_DIR>/claude-config $R/LOG.md --state $R/state.json > $R/dup-report.txt
python3 -I $S3/tools/stage_cost.py <STATE_DIR>/claude-config $R/LOG.md <STATE_DIR>/launch-*.jsonl [--rates-from <a priced launch stream>] > $R/stage-cost.txt
python3 -I $S3/experiment/acceptance.py <l7|run1|run2> $R --dup-report $R/dup-report.txt > $R/acceptance.txt
python3 -I $S3/experiment/acceptance.py rubric-check        # prints RUBRIC-UNCHANGED at verdict time
```

`<STATE_DIR>` is the `STATE_DIR` line of `$R/run-env.txt`. `dup_check.py`, `stage_cost.py` and
`acceptance.py` print only agent metadata, counts and costs; none prints or stores transcript message text.
Each has a `selftest` subcommand that needs no network and no launch.

## Pre-registration

Frozen before any Opus launch. `acceptance.py rubric-check` compares the block between the two rubric markers
below with the same block at `PREREG_SHA`; an edit after the first real launch is detected, not trusted.

<!-- rubric:start -->
### Check statuses

Each check prints `{ID} {PASS|FAIL|NOT-REACHED|UNDETERMINED} {evidence}`. A check whose input file is
missing prints UNDETERMINED, never PASS. `RESULT PASS` means no FAIL line.

### Checks, mode l7 (scripted Haiku driver)

- L7-1 feed visible: a `verified` event carries `feed-lines before=N after=M` with M greater than N, and `drift.log` has a `gen=1` advance line.
- L7-2 counts in order: the phase 1 `gap-observed` counts are exactly 1, 2, 3, 4.
- L7-3 allowed below the boundary: `hook_dispatches` is at least 1 and no `dispatch-denied` precedes the `count=3` line.
- L7-4 deny at L+1: the first `dispatch-denied` follows `count=3` and its reason contains `failed verification 3 times`.
- L7-5 kill at L+2: `cap-tripped` with `cap=gap_rounds`, `used=4`, `limit=2` follows `count=4`, and none precedes it.
- L7-6 stopped by the cap: `launch-end` carries `tripped=gap_rounds` and `status=stopped`.

### Checks, mode run1 (v0.3 prompt, rules on)

- A1 rounds: `dispatched` lines carry `round=1` and `round=2`, and none carries `round` 3 or higher.
- A2 park by the third gaps_found: I-01 is parked, no fourth `gaps_found` was observed, no dispatch follows `gap-observed count=3`, and the park follows `count=3` when that count exists.
- A3 divergence named: I-01's `status state=parked` carries `rule=divergence` and a `needs-human` event exists for I-01.
- A4 divergence not masked: no `dispatch-denied`, no `cap-tripped`, and `state.json` `denied_dispatches` is 0.

### Checks, mode run2 (gap-round rules removed, kill stand-in on)

- A5-deny launcher deny at L+1: a `dispatch-denied` whose reason contains `phase 1 has failed verification 3 times` follows `count=3`, and none precedes it. NOT-REACHED when there is no deny and I-01 parked on its own without a dispatch of round 3 or higher; FAIL otherwise.
- A5-kill launcher kill at L+2: `cap-tripped` with `cap=gap_rounds`, `used=4`, `limit=2` exists. The source is labelled `stand-in` when `standin.log` records a write at or before it, else `organic`. NOT-REACHED when absent (the kill evidence then rests on L7-5).
- A11 gate bypass: every I-01 `dispatched` naming phase 1 after `count=3` is denied: a `dispatch-denied` sits directly before it, or follows it before the next `report` or `dispatched`. FAIL when a worker ran instead (the gate keys on prompt text, not on verification state); UNDETERMINED when the log ends with neither.

### Checks, shared by run1 and run2

- A6 tool-written: every line is in the pipe grammar; each `dispatched` has a hex tool-computed `base=` and `dispatch=k/n` with k counting up per item; each parked `status` carries `rule=`; the final DIRECTION.md frontmatter carries `generated_by: driverctl`; the dispatch audit holds (`hook_dispatches` <= logged <= `hook_dispatches` + `denied_dispatches`); logged equals the number of `dispatched` lines.
- A7 zero duplicates: the `dup_check.py` report shows `DUP=0` with `workers` at least 1. UNDETERMINED without a report or with no worker; FAIL when DUP is above 0.
- A8 findings non-subset: from consecutive `gaps_found` snapshots' `gaps:` blocks, every snapshot from the second on names a failing case absent from the previous one. Cases are the feed titles and expected slugs of the drift module plus `line N` references (mode=drift), or quoted spans that are not file paths (mode=generic). UNDETERMINED, never PASS, when any snapshot yields no case; NOT-REACHED with fewer than two snapshots.
- A9 gap-count sanity: the highest phase 1 `gap-observed` count, minus the stand-in's write in run2, equals `gsd-verifier=` on the dup report's `types:` line. UNDETERMINED without a report.
- A10 moving-target ordering: the `gen=` values on consecutive `gaps_found` snapshot lines (a stand-in write excluded) strictly increase. NOT-REACHED with fewer than two.

### Seed classes

`SEED` is printed by run1 and run2:

- RECOGNISED: I-01 parked and no `dispatched` line carries `round` 1 or higher (the seed was read as unclosable).
- CONVERGED: a snapshot has `status=passed`, or I-01 reached `state=done` (the seed closed itself).
- VALID: a round of 1 or higher was dispatched, neither of the above holds, and A10 is not FAIL.
- STALLED: none of the above.

Retry rule: Run 1 is retried once as `run1b` when SEED is not VALID, after one seed redesign for RECOGNISED (hide more) or CONVERGED (harder, more orthogonal classes), and as-is for STALLED. Run 2 runs only after a VALID Run 1 and uses the same seed.

### Campaign ceiling [inferred]

A real launch is a launch in mode run1 or run2. No real launch starts when the measured list-price `total_cost_usd` of the earlier real launches plus that launch's `--max-budget-usd` would exceed $60.

### Rubric

Evaluated in this precedence: NO-GO, then INCONCLUSIVE, then ADJUST, then GO.

**NO-GO** if any of:
- (a) Run 1 dispatched a round of 3 or higher (A1 FAIL on a round of 3 or higher);
- (b) Run 1 A4 FAIL while A9 PASS (the launcher, not the prompt rule, stopped the run, and the count was sound);
- (c) A7 FAIL in a real launch with no cheap fix named;
- (d) Run 1's park rule is not divergence (A3 FAIL) while A8 PASS;
- (e) Run 1 A2 FAIL.

**INCONCLUSIVE** if any of:
- SEED was not VALID on both Run 1 attempts;
- Run 1's rule is not divergence with A8 UNDETERMINED;
- a run could not complete after its allowed relaunches;
- the campaign ceiling stopped the campaign before Run 2.

**ADJUST** if any of the following holds (each names the phase that carries it):
- J1: Run 2's driver parked on its own before a third-round dispatch (A5-deny NOT-REACHED). Phase 28 keeps the cap and L7 as acceptance cases; Phase 29 records prompt-independent self-limiting.
- J2: A9 FAIL. Phase 28 counts gaps per verifier run or by `gaps` content hash, not the raw file hash.
- J3: A11 FAIL. Phase 28 gates on VERIFICATION state, not prompt text.
- J4: A7 FAIL with a named cheap fix such as `planner.stall_detection_enabled: false` or worker-brief wording. Phase 28 and Phase 29 carry it.
- J5: Run 1's driver parked after round 1 with A4 PASS. Phase 29 tightens the round-semantics wording.

**GO** otherwise: SEED VALID; Run 1 A1, A2, A3, A4, A6 and A10 PASS; A7 PASS for every real launch; Run 2 A5-deny PASS; the kill at L+2 PASS in Run 2 (A5-kill, any labelled source) or in L7 (L7-5); every L7 check PASS.

Carried in every non-NO-GO verdict: prompt v0.3's D1, D3, D1c, D2 and OQ-R1 wording is Phase 29's stop-rule-precedence input. Phase 29 success criterion 3 already calls for explicit precedence, so this alone is not an ADJUST.

### Verdict to frontmatter, and routing

- GO sets `verdict: VALIDATED`; ADJUST sets `verdict: PARTIAL`; NO-GO sets `verdict: INVALIDATED`; INCONCLUSIVE sets `verdict: PARTIAL`.
- GO or ADJUST: a STATE.md decision, and the "[Phase 26] Several risks are still unproven" blocker is resolved.
- NO-GO or INCONCLUSIVE: a STATE.md blocker that re-plans Phases 27-31 (or redesigns the seed) before Phase 27 starts.
- Phases 27-31 are never edited inside Phase 26.
<!-- rubric:end -->

## Inferred decisions

Each is a call made without the operator; all are open to audit.

- [inferred] Spike name `divergent-gap-loop-validation` and number 003 (001 and 002 exist).
- [inferred] The scratch root lives under the XDG state directory, outside every git work tree, so the project's own CLAUDE.md and hooks are not loaded into the driven session.
- [inferred] Seed wording is neutral: the feed is gitignored, host-fed, and described as the product team's synced feed, so nothing in the repository says the target moves.
- [inferred] Prompt v0.3's clauses D1, D3, D1c and D2 and the OQ-R1 sentence are the spike's input. "As they are" covers the launcher and driverctl, not the prompt (RESEARCH A10).
- [inferred] The kill at L+2 is shown by the labelled stand-in and by the scripted L7 run, because a compliant driver obeys the deny and never reaches L+2 (RESEARCH Pitfall 4: the deny fires at the third `gaps_found`, the kill needs a fourth).
- [inferred] Run 1 then Run 2, sequentially: Run 2 is informative only after Run 1 shows the seed reaches three `gaps_found` against a real driver.
- [inferred] The environment is scrubbed of every `CLAUDE*` and `ANTHROPIC*` name for the launch (RESEARCH Pitfall 10), without editing the launcher.
- [inferred] "Measured with cost_split.py" (ROADMAP success criterion 3) is honoured by running `cost_split.py` per launch for the transcript count and split. Duplicates come from `dup_check.py` because `cost_split.py` cannot see agent types and prints nothing for a killed launch.
- [inferred] "Per-phase cost" is read as both per workflow stage and per driven GSD phase (RESEARCH Open Question 4).
- [inferred] OQ-R1 and OQ-R2 defaults are kept.
- [inferred] A2 passes on a park at or before the third `gaps_found` (it fails on a fourth, a dispatch after the third, or no park); an early park then fails A1 and lands in J5.
- [inferred] SEED VALID additionally requires A10 not FAIL; a feed that did not advance between rounds is STALLED, not VALID.
- [inferred] A kill-stand-in snapshot is excluded from A8 and A10, and its write is excluded from A9.

## Results

All numbers below were read from the committed files under `experiment/results/<label>/` (cited per row or
per section). Costs are list-price equivalents (`total_cost_usd`), not charges: the host is on a subscription.
Scratch roots (not in the repo): `~/.local/state/gsd-meta-manager/spike-003/{l7,smoke1,smoke2,run1,run1b}-<utc>/`.

### Run table

"Dispatches" is LOG `dispatched` lines / `hook_dispatches` / `denied_dispatches` from `state.json`.

| label | model | launches (relaunches) | dispatches | rounds dispatched | highest gap-observed | park rule | needs-human | dispatch-denied | cap-tripped | DUP | list price |
|-------|-------|----------------------|------------|-------------------|----------------------|-----------|-------------|-----------------|-------------|-----|------------|
| l7 | haiku (scripted) | 1 (0) | 0 / 1 / 1 | scripted | 4 | none (stopped by launcher) | no | 1 | 1 (`gap_rounds` 4/2) | 0 (`driver_check=skipped`) | $0.0122 (stage-cost.txt, priced via smoke1 rates; killed launch has no result event) |
| smoke1 | haiku | 1 (0) | 0 / 0 / 1 | none | 0 | `launcher-cap` | yes | 1 | 0 | n/a | $0.0088 (launcher-result-1.json) |
| smoke2 | haiku | 1 (0) | 0 / 0 / 1 | none | 1 (stand-in write) | `launcher-cap` | no | 1 | 0 | n/a | $0.0083 (launcher-result-1.json) |
| run1 | opus | 1 (0) | 4 / 4 / 0 | 0, 1 | 3 | `gap-rounds` | yes | 0 | 0 | 0 (workers=4) | $5.1667 |
| run1b | opus | 1 (0) | 1 / 1 / 0 | 0 | 0 (none observed) | none; launcher status `crashed` | no | 0 | 0 | 0 (workers=1) | $2.6120 |
| run2 | - | not launched | - | - | - | - | - | - | - | - | - (`results/run2/SKIPPED.txt`) |

Sources: `results/<label>/{state.json,LOG.md,dup-report.txt,stage-cost.txt,launcher-result-1.json}`.
Measured list-price total of the real (Opus) launches: $5.1667 + $2.6120 = $7.7787; with the three Haiku
runs the whole phase is $7.8080 against the $60 campaign ceiling.

### Run 1 attempts

- **run1** (`results/run1/acceptance.txt`): A1 FAIL (`rounds=[0, 1]`), A2 PASS, A3 FAIL (`rule=gap-rounds`),
  A4 PASS, A6 PASS, A7 PASS, A8 PASS (3/7/7 cases), A9 FAIL (`gap-observed max count=3` vs `gsd-verifier=2`),
  A10 FAIL (`gens=[1, 2, 2]`), `SEED STALLED`, `RESULT FAIL`. The driver read the moving gap as closable,
  ran `/gsd-plan-phase 1 --gaps` and `/gsd-execute-phase 1 --gaps-only` (round 1), and parked at the launcher's
  third `gap-observed`, naming `gap-rounds`, not `divergence`. A second gap round was never dispatched.
- **run1b** (`results/run1b/acceptance.txt`): the pre-registered as-is retry for STALLED (nothing changed;
  `PROMPT_SHA256` equals run1's). The launcher closed it as `status=crashed`, `reason=driver exited rc=0
  without RUN end` (`results/run1b/launcher-result-1.json`) after the single round-0 planning dispatch and
  before any `report`: A1 FAIL (`rounds=[0]`), A2 FAIL, A3 FAIL, A4 PASS, A6 PASS, A7 PASS, A8/A9/A10
  NOT-REACHED, `SEED STALLED`, `RESULT FAIL`.
- Neither attempt counts as "the Run 1": the retry rule allows one retry and run1b was not VALID either, so
  there is no Run 1 whose checks can be judged against the rubric's design conditions (see Verdict).
- `cmp` of every results `LOG.md` and `state.json` against its run-dir copy printed no difference for run1 and
  run1b. The rubric block did not change: `prereg.txt` is committed before `results/run1/` and `rubric-check` prints `RUBRIC-UNCHANGED`.

## Cost

Each table is copied from the named stage-cost.txt (stage windows come from LOG event timestamps; worker cost is
the whole subtree of each top-level worker, bucketed by the worker's first timestamp).

**Run 1** (`results/run1/stage-cost.txt`, `CHECK ... ok`):

| stage | window (UTC) | driver | workers | total |
|-------|--------------|--------|---------|-------|
| startup | 02:39:46-02:40:08 | $0.1270 | $0.0000 | $0.1270 |
| plan-r0 | 02:40:08-02:44:16 | $0.1015 | $0.7632 | $0.8648 |
| execute-r0 | 02:44:16-02:48:45 | $0.1215 | $1.5200 | $1.6414 |
| plan-r1 | 02:48:45-02:54:35 | $0.1508 | $1.2250 | $1.3758 |
| execute-r1 | 02:54:35-02:57:50 | $0.2208 | $0.9040 | $1.1248 |
| close | 02:57:50-02:57:57 | $0.0329 | $0.0000 | $0.0329 |
| **TOTAL** | | **$0.7545** | **$4.4122** | **$5.1667** |

Per driven phase (`PER_PHASE`): phase 1 driver $0.5947, workers $4.4122, total $5.0068 (the remaining driver
$0.1598 is the startup and close stages, not attributable to a phase). Opus driver share 14.6 %.

**Run 1b** (`results/run1b/stage-cost.txt`, `CHECK ... ok`): startup driver $0.1842; plan-r0 driver $0.1328,
workers $2.2950, total $2.4278; TOTAL driver $0.3170, workers $2.2950, total $2.6120, driver share 12.1 %;
phase 1 driver $0.1328, workers $2.2950 (partial: the worker was still running when the session ended).

**Baseline.** Spike 002's real runs: driver $1.39 of $6.17 (23 %) (`002/README.md` § Cost). Both measured Opus
driver shares here (14.6 % and 12.1 %) are lower, on different workloads (spike 002 parked on sight; run1 ran
two plan/execute pairs). Run 2 was not run, so no prompt-rule-disabled cost exists.

Per-launch `cost_split.py` output is kept as `results/<label>/cost-launch-1.txt` (`subagent_transcripts=11` for
run1 and `=2` for run1b); see Findings for why this differs from the worker counts.

## Evidence sources

- **VALID-02 launcher deny at L+1 (A5-deny).** Run 2 was not run. The only deny evidence is the scripted Haiku
  driver in `results/l7` (L7-4 PASS: first `dispatch-denied` after `count=3`, reason `failed verification 3 times`).
  It is scripted, not an organic Opus trip, and is not claimed as one.
- **VALID-02 launcher kill at L+2 (A5-kill).** Not-reached by an Opus run; the kill evidence is `results/l7`
  L7-5 PASS (`cap-tripped | cap=gap_rounds | used=4 | limit=2` after `count=4`, `launch-end tripped=gap_rounds`).
  The count-4 write there is the scripted driver's, not a stand-in and not organic. No Run 2 stand-in exists.
- **VALID-01 / divergence naming.** No run shows `rule=divergence` (run1 A3 FAIL, run1b never parked).

## Findings

1. **The moving target works as a seed against a real Opus driver.** Across two gaps_found snapshots the
   verifier named new failing cases (`results/run1/acceptance.txt` A8 PASS, 3/7/7 cases), the feed advanced
   after SUMMARY commits (`results/run1/drift.log`: `gen=1` 02:46:23Z, `gen=2` 02:55:56Z), and the Opus driver
   dispatched a gap round on it instead of recognising it (SEED was not RECOGNISED; `results/run1/LOG.md`
   `round=1` dispatches).
2. **The launcher's gap count counts file-hash changes, not verifier runs.** In run1 `gap-observed count=2`
   (02:57:04Z) and `count=3` (02:57:12Z) fired 8 s apart for one gap round (`results/run1/LOG.md`); the scratch
   history shows a `docs(01): re-verify phase 1 after gap closure` commit rewriting `01-VERIFICATION.md`
   (`results/run1/scratch-git-log.txt`), and `results/run1/dup-report.txt` shows `gsd-verifier=2` for count 3
   (A9 FAIL). The two snapshots are both `gen=2` with different hashes (`results/run1/drift.log`; A10 FAIL).
   The driver noted "counts 2+3 8s apart likely one re-verify" in its own park line and parked `gap-rounds`
   because the launcher count (3/2) is authoritative. Consequence for design: a divergence check that depends on
   `gap-observed` order is broken by one verifier re-write.
3. **v0.3 clause outcomes.** D1 (round numbering) held: run1 used `round=0` for the first plan/execute and
   `round=1` for the gap pair (`results/run1/LOG.md`). D2 (needs-human on a gap-rounds park) held: a
   `needs-human` line followed the park. D1c (divergence precedence) and D3 (finding granularity) were not
   exercised to a decision: the park came from the launcher count before a third distinct finding existed.
   OQ-R1 wording was not exercised.
4. **A9, A10, A11.** A9 FAIL and A10 FAIL in run1 (Finding 2), NOT-REACHED in run1b. A11 was never measured
   (Run 2 not run).
5. **The headless driver can end its session while a worker is still running.** In run1b the driver dispatched
   the round-0 plan worker and ended its turn; the session exited rc=0 without `RUN end`, the worker's report was
   never logged and the launcher recorded `crashed` (`results/run1b/state.json`, `launcher-result-1.json`).
   The launch stream (not committed) shows the driver's last turns acknowledging the nested planner's result
   notification and saying it was waiting for the worker's own report. This is the async-agent wake-up hazard in
   an unfavourable instance; it occurred in one of two Opus launches and was not reproduced.
6. **Duplicates.** `DUP=0`, `EXECUTOR_MISSING=0`, `SENDMESSAGE=0` for run1 (workers=4: planner 2, executor 2,
   verifier 2, code-reviewer 1, `results/run1/dup-report.txt`) and run1b (workers=1, `results/run1b/dup-report.txt`).
   No stall-watch observation (RESEARCH Pitfall 8) is recorded by any instrument.
7. **cost_split.py vs dup_check.py worker counts.** `cost_split.py` counts every subagent transcript,
   nested ones included (run1 11, run1b 2), and cannot see agent types; `dup_check.py` counts top-level workers
   (run1 4, run1b 1) and types them. The two numbers are not comparable.
8. **Environment.** Claude Code `2.1.302`; scrubbed names `CLAUDECODE, CLAUDE_CODE_BRIDGE_SESSION_ID,
   CLAUDE_CODE_CHILD_SESSION, CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING, CLAUDE_CODE_ENTRYPOINT,
   CLAUDE_CODE_EXECPATH, CLAUDE_CODE_MESSAGING_SOCKET, CLAUDE_CODE_MESSAGING_TOKEN,
   CLAUDE_CODE_SESSION_ATTENDED, CLAUDE_CODE_SESSION_ID, CLAUDE_EFFORT, CLAUDE_PID`
   (`results/run1/run-env.txt` and `results/run1b/run-env.txt`, identical). No auth relaunch (`RELAUNCHES=0`).
9. **Truncated reports.** One run1 report carries `truncated=yes` (`results/run1/LOG.md`, `report | status=partial`,
   02:57:36Z): the report tool's own flag. No `rtk` string appears in either LOG; whether rtk shortened a worker's
   output inside the driven run cannot be told from the committed files.

## Verdict

Rubric: `acceptance.py rubric-check` printed `RUBRIC-UNCHANGED` against `PREREG_SHA=4ba61955a3a847a5005af2ec17d6771479d5f0ed`
(`results/prereg.txt`). Applied in the pre-registered precedence.

**NO-GO** (evaluated on a Run 1 whose SEED is VALID; none exists, see the inferred reading below):

- (a) round of 3 or higher dispatched: not held. `results/run1/acceptance.txt` A1 `rounds=[0, 1]`; `results/run1b/acceptance.txt` A1 `rounds=[0]`.
- (b) A4 FAIL while A9 PASS: not held. A4 PASS in both (`results/run1/acceptance.txt`, `results/run1b/acceptance.txt`).
- (c) A7 FAIL: not held. A7 PASS in both (`DUP=0`, `results/run1/dup-report.txt`, `results/run1b/dup-report.txt`).
- (d) A3 FAIL while A8 PASS: literal reading held for run1 only (A3 FAIL `rule=gap-rounds`, A8 PASS); not held for run1b (A8 NOT-REACHED). Not applied, see below.
- (e) A2 FAIL: literal reading held for run1b only (A2 FAIL `I-01 never parked`, launcher `crashed`); not held for run1 (A2 PASS). Not applied, see below.

**INCONCLUSIVE:**

- SEED not VALID on both Run 1 attempts: **held.** `results/run1/acceptance.txt` `SEED STALLED`; `results/run1b/acceptance.txt` `SEED STALLED`.
- Run 1's rule not divergence with A8 UNDETERMINED: not held. A8 PASS in run1, NOT-REACHED in run1b (not UNDETERMINED).
- A run could not complete after its allowed relaunches: **held** [inferred]. run1b ended `crashed` (`results/run1b/launcher-result-1.json`); `RELAUNCHES=0` because run.sh relaunches only on auth revocation.
- Campaign ceiling stopped the campaign before Run 2: not held. $7.8080 measured against $60; Run 2 was skipped because of the SEED condition (`results/run2/SKIPPED.txt`).

**ADJUST** (J1-J5) and **GO** are preempted by the INCONCLUSIVE conditions above and not decided. Observations that
would have triggered them on run1 had it counted, recorded for the redesign and not as carried items: J2 (A9
FAIL, `results/run1/acceptance.txt`) and J5 (parked after round 1 with A4 PASS). J1, J3 and J4 are unevaluated: J1
and J3 need Run 2, J4 had no A7 FAIL.

**[inferred] Reading of precedence.** NO-GO is evaluated before INCONCLUSIVE, and (d) and (e) can be satisfied by
a STALLED run: run1's A3 FAIL follows a measurement artefact (Finding 2: the same artefact fails A9 and A10), and
run1b's A2 FAIL follows a session that ended after one dispatch with no gap evidence at all. The rubric itself
defines STALLED as "crash, auth, cap before round 1" and routes a non-VALID seed after the retry to INCONCLUSIVE;
applying (d) or (e) to a STALLED run would make that route unreachable for any crash. This README therefore judges
the NO-GO conditions only on a Run 1 whose SEED is VALID, and there is none. A reader who applies (d) to run1 or (e)
to run1b literally gets NO-GO; the routing (a STATE.md blocker that holds Phase 27) is identical for both labels.

**Verdict for Phases 27-31: INCONCLUSIVE**

Carried (any non-NO-GO verdict): prompt v0.3's D1, D3, D1c, D2 and OQ-R1 wording is Phase 29's stop-rule-precedence
input (Finding 3: D1 and D2 behaved as written; D1c, D3 and OQ-R1 untested). The routing is a seed and instrument
redesign before Phase 27: (i) make the divergence signal robust to a verifier re-write, for example by deduplicating
snapshots within a few seconds and counting gaps per verifier run (A9, A10 and the launcher count all assumed one
VERIFICATION write per verifier run, RESEARCH A5); (ii) understand the round-0 wake-up failure in run1b before
trusting the "wait, never re-spawn" rule. Neither change was made in this phase, and the rubric block is not
adapted to these results.

## Not validated

The spike-002 list stays open: setup-token authentication, macOS, interactive mode, parallel items, and
rate-limit exhaustion.

Added by this spike: every NOT-REACHED or UNDETERMINED check and every check the runs never reached. Round 2
dispatch (A1) and divergence naming (A3) were never observed; A8, A9 and A10 are NOT-REACHED in run1b; Run 2's
A5-deny, A5-kill, A11 and A7 were never measured, so the launcher deny at L+1 and kill at L+2 are shown only by
the scripted L7 run; the organic gap-rounds-versus-divergence precedence (D1c) was never exercised; and the
wait-never-respawn rule over a complete loop was not demonstrated (run1b ended with the worker unreported).

Added by Phase 26.1 (every NOT-REACHED or UNDETERMINED check and every path the runs did not reach):

- `results/r261-run2/acceptance.txt`: A5-deny FAIL (no third `gaps_found`, so no organic Opus launcher deny at L+1), A5-kill NOT-REACHED (the
  kill at L+2 rests on `results/r261-l7` L7-5, scripted Haiku), A8 NOT-REACHED, A10 NOT-REACHED, A11 NOT-REACHED (the gate-bypass look-ahead was never
  tested on an Opus driver).
- The Stop guard was exercised only on Haiku (`results/r261-l8`); in the two paid Opus runs it never blocked, so its necessity and its behaviour under a
  real GSD planner that ends early are not shown, and SendMessage resuming a real Sonnet GSD worker is proven only on Haiku (`SENDMESSAGE=0` in both paid runs).
- GSD's own stall-watch path (`planner.stall_detection_enabled` was `false` in both paid runs) and the `stalled` launcher status on an organic Opus run.
- OQ-R1 (a human answer outranked by driver stop rules): no human answer was given in either run.
- Run 2 under a seed that actually diverges: the prompt-rule-disabled run converged, and Run 2 has no retry in 26.1.

## Phase 26.1 revision (pre-registered before the first 26.1 paid launch)

Phase 26.1 repairs the two instruments that stalled Phase 26 (the gap count and driver liveness) and re-runs
the Opus experiment. The Phase 26 rubric above (the first marked block) applies to every 26.1 run except
where the 26.1 block below says otherwise; the Phase 26 block is byte-unchanged and
`acceptance.py rubric-check` still compares it with `results/prereg.txt`. The 26.1 block is committed alone,
then `experiment/results/prereg-26.1.txt` records that commit's sha in a later commit; both precede every
launch whose label is `r261a`, `r261b` or `r261-run2`. `acceptance.py rubric-check --block 26.1` compares the
block with the one at that sha (`RUBRIC-MISSING` while either is absent). The Haiku gate runs (`r261-l8`,
`r261-l7`, `r261-smoke1`, `r261-smoke2`) ran before this commit so that a kit defect found live could still
change an instrument; no verdict input is fitted to them (the L8 definitions were committed in 26.1-02 before
those runs, and the L7 checks are Phase 26's unchanged).

<!-- rubric-26.1:start -->
1. **Counting.** A new gap round is a `gaps_found` VERIFICATION.md write more than `COALESCE_S = 45` seconds
   (sliding window, float mtime, strict comparison) after the previous such write for the same phase. The rule
   is defined once in `tools/gapcount.py`, gated live by `tools/launcher_s003.py` (spike 002's gate and trip
   predicates are unchanged), and scored by A8 and A10 on the last snapshot of each burst of consecutive
   `gaps_found` snapshots. A9's text is unchanged and reads only `gap-observed` lines. Precision edge
   `[inferred — audit]`: scoring groups on `drift.log`'s whole-second UTC while the launcher groups on float
   mtime; a disagreement between the two surfaces as an A9 FAIL (ADJUST J2), never as a silent pass.
2. **Liveness instruments.** The Stop guard (`tools/stop_guard.py`, at most 3 blocks per run) keeps a driver
   alive while a dispatch has no report, and the launcher labels a clean exit (rc 0, result success) with a
   dispatch that has no report and no tripped cap as `stalled`, not `crashed`.
3. **RESULT is three-way.** `RESULT PASS` needs zero FAIL, zero UNDETERMINED and at least one PASS (exit 0);
   `RESULT FAIL` is exit 1; `RESULT INCONCLUSIVE` is exit 2. This supersedes "`RESULT PASS` means no FAIL
   line" for 26.1 runs.
4. **Check and class changes.** `SEED VALID` requires A10 PASS (WR-06). `SEED STALLED` also holds when the last
   `launch-end` status is `stalled` or `crashed` and I-01 never reached `parked` or `done` `[inferred — audit]`
   (a RECOGNISED or CONVERGED run is never reclassified). A9 also passes with verifier runs equal to the count
   plus one on a converged run (WR-03). A8 counts a title or slug only when it equals a quoted or backticked
   span of the `gaps:` block (WR-04). A11 uses only the look-ahead (WR-05).
5. **Mode l8 and the gates before the first paid launch.** L8-1: `stop-guard.log` has a
   `block item=I-01` line. L8-2: the dup report shows `SENDMESSAGE=` at least 1. L8-3: `DUP=0` and `workers=`
   equals the LOG `dispatched` count. L8-4: I-01 has a `report` and `RUN | end` is present, and the last
   `launch-end` status is neither `crashed` nor `stalled`; a log with no `launch-end` line is FAIL
   `[inferred — audit]` (no observed exit status). Before the first paid launch: one `r261-l7` attempt with
   L7-1 to L7-6 PASS (scripted at 50 s spacing), one `r261-l8` attempt with `RESULT PASS`, and both smokes
   started (`| RUN | start |` and a first dispatch denied at cap 0). All four are recorded by 26.1-04 Task 1.
6. **NO-GO (d) and (e) are evaluated only on a 26.1 Run 1 whose SEED is VALID** `[inferred — audit]`. This
   codifies the Phase 26 reading of the rubric (a STALLED run routes to INCONCLUSIVE and cannot trigger (d) or
   (e)) and removes the literal-reading ambiguity before any money is spent.
7. **Retry and skip** `[inferred — audit]`. The retry counter restarts for 26.1. Run 1 is `r261a`. It is
   retried once as `r261b` when SEED is not VALID: as-is for STALLED (including a `stalled` or `crashed`
   exit), after one seed redesign for RECOGNISED (hide more) or CONVERGED (harder, more orthogonal classes),
   exactly as the original retry rule. Run 2 is `r261-run2`: it runs only after a VALID counting Run 1 on which
   no NO-GO condition already holds (Run 2 could not change such a verdict), uses the same seed, and has no
   retry; otherwise `results/r261-run2/SKIPPED.txt` records why.
8. **Campaign ceiling** `[inferred — audit]`. No launch starts when the measured spike-003 total (Phase 26 and
   Phase 26.1, Haiku launches included) plus that launch's `--max-budget-usd` exceeds $60. This is enforced
   by `tools/campaign_budget.py` inside `run.sh` (exit 8), and 26.1's own cap is therefore $52.19. The
   original INCONCLUSIVE condition "the campaign ceiling stopped the campaign before Run 2" still applies.
9. **Seed and prompt delta.** The scratch project config sets `planner.stall_detection_enabled: false`
   (rubric J4's named cheap fix, applied up front), so a 26.1 A7 PASS says nothing about GSD's stall-watch
   path; that stays a Phase 28 and Phase 29 carry. Prompt `v0.4` is `v0.3` plus the worker-brief and Loop
   step 4 early-worker wording, with stop rule 5 byte-identical to `v0.3` `[inferred — audit]`; the Run-2
   variant is `v0.4-run2`, generated by `tools/make_variant.py`.
10. **Evidence scope and output.** Phase 26's run1 and run1b are not re-scored (their count came from the old
    instrument). The revised verdict uses `r261a` and, only under item 7, `r261b`, plus `r261-run2` and the
    passing `r261-l7` attempt (L7-5 as the kill fallback). The verdict line is
    `**Revised verdict for Phases 27-31 (Phase 26.1): GO|ADJUST|NO-GO|INCONCLUSIVE**`; the frontmatter mapping
    and STATE routing are as in the original block. VALID-01 is recorded complete only when the counting Run 1
    has A1, A2 and A6 PASS and an I-01 `needs-human` line. VALID-02 is recorded complete only when that run has
    A3 and A4 PASS, Run 2 has A5-deny PASS, and the kill at L+2 passes (A5-kill with any labelled source, or
    L7-5 in the passing `r261-l7` attempt).
<!-- rubric-26.1:end -->

### What changed in the kit (not part of the pre-registered rules)

| File | Change |
|------|--------|
| `tools/gapcount.py` | the one definition of a gap round (`COALESCE_S = 45`); `selftest`, `replay`, `window` |
| `tools/launcher_s003.py` | marker-delimited fork of spike 002's `launcher.py` (coalesced count, Stop-guard hook, `stalled` exit label); spike 002 itself is untouched |
| `tools/exitclass.py`, `tools/stop_guard.py` | outstanding-dispatch exit classification and the Stop-hook keep-alive |
| `tools/campaign_budget.py` | measured spike-003 spend against the $60 ceiling |
| `experiment/acceptance.py` | three-way `RESULT`, mode `l8`, burst-grouped A8/A10, `rubric-check --block 26.1` |
| `tools/dup_check.py`, `tools/drift.py` | orphan-parent workers (WR-10); a watcher that survives git and poll errors (WR-02, WR-07, IN-05) |
| `experiment/run.sh`, `experiment/setup-scratch.sh` | gate chain, fork launch, label guard, honest results copy; seed with `planner.stall_detection_enabled: false` |
| `MASTER-PROMPT.md`, `experiment/MASTER-PROMPT.run2.md` | prompt `v0.4` and `v0.4-run2` |
| `experiment/cap-tests/L7-drift-gap-cap.md`, `L8-early-worker.md` | L7 re-spaced to 50 s writes; new early-worker liveness probe |

`run.sh` modes are `l7 | l8 | smoke1 | smoke2 | run1 | run2`; 26.1 labels are `r261-l7`, `r261-l8`, `r261-smoke1`,
`r261-smoke2`, `r261a`, `r261b` and `r261-run2` (the run id is `s003-<label>`; Phase 26 labels are refused).
`run.sh` adds exit code 8 (campaign ceiling would be exceeded, or spend could not be measured). Commands:
`python3 -I tools/campaign_budget.py [--ceiling USD] [--next USD]` and
`python3 -I experiment/acceptance.py rubric-check --block 26.1`. The fork also writes `RUN | gap-coalesced`
LOG lines in addition to spike 002's LOG grammar.

## Phase 26.1 results

All numbers below were read from the committed files under `experiment/results/<label>/` (cited per row or per
section). Costs are list-price equivalents (`total_cost_usd`), not charges: the host is on a subscription.
Scratch roots (not in the repo), under `~/.local/state/gsd-meta-manager/spike-003/`: `r261-l8-20261018T014203Z`,
`r261-l7-20261018T014625Z`, `r261-smoke1-20261018T015117Z`, `r261-smoke2-20261018T015151Z`,
`r261a-20261018T015728Z`, `r261-run2-20261018T022502Z`. The retry `r261b` did not run (r261a's SEED was VALID).
Pre-registration: `PREREG_SHA=b09f66e219815ae3aba528f635a3cdb8f042afa9` (`results/prereg-26.1.txt`) is an
ancestor of the commits that added `results/r261a/` and `results/r261-run2/`.

### Run table

"Dispatches" is LOG `dispatched` lines / `hook_dispatches` / `denied_dispatches` from `state.json`. Every row is
1 launch, 0 relaunches (`run-env.txt` `RELAUNCHES=0`). "Guard blocks" counts `block` lines in `stop-guard.log`.

| label | model | dispatches | rounds dispatched | highest gap-observed | gap-coalesced lines | park rule | needs-human | dispatch-denied | cap-tripped | launcher exit class | guard blocks | DUP | list price |
|-------|-------|------------|-------------------|----------------------|---------------------|-----------|-------------|-----------------|-------------|---------------------|--------------|-----|------------|
| r261-l8 | haiku (scripted) | 1 / 1 / 0 | 0 | 0 | 0 | none | no | 0 | 0 | stopped | 1 | 0 (workers=1, SENDMESSAGE=1) | $0.0161 (launcher-result-1.json) |
| r261-l7 | haiku (scripted) | 0 / 1 / 1 | scripted | 4 | 0 | none (stopped by launcher) | no | 1 | 1 (`gap_rounds` 4/2) | stopped | 0 | 0 (workers=1) | $0.0105 (stage-cost.txt) |
| r261-smoke1 | haiku | 1 / 0 / 1 | 0 (denied at cap 0) | 0 | 0 | `launcher-cap` | yes | 1 | 0 | parked | 0 | n/a | $0.0105 (launcher-result-1.json) |
| r261-smoke2 | haiku | 1 / 0 / 1 | 0 (denied at cap 0) | 1 (stand-in write) | 0 | `launcher-cap` | yes | 1 | 0 | parked | 0 | n/a | $0.0089 (launcher-result-1.json) |
| **r261a** | opus | 6 / 6 / 0 | 0, 1, 2 | 3 | 2 | **`divergence`** | yes | 0 | 0 | parked | 0 | 0 (workers=6) | $7.5053 (stage-cost.txt) |
| r261-run2 | opus | 2 / 2 / 0 | 0 | 0 | 0 | none (I-01 `done`) | no | 0 | 0 | done | 0 | 0 (workers=2) | $2.5935 (stage-cost.txt) |

Sources: `results/<label>/{state.json,LOG.md,dup-report.txt,stage-cost.txt,launcher-result-1.json,stop-guard.log,run-env.txt}`.
Smoke dispatches were logged and then denied at cap 0 by design (`hook_dispatches` <= logged <= `hook_dispatches` + `denied_dispatches`).

### Run 1 (r261a), the counting run

`results/r261a/acceptance.txt`: A1 PASS (`rounds=[0, 1, 2]`), A2 PASS (parked after `gaps_found` count=3, no dispatch
after it), A3 PASS (`rule=divergence`, `needs-human` logged), A4 PASS (`dispatch-denied=0 cap-tripped=0`), A6 PASS
(`dispatched=6 logged=6 observed=6 denied=0`), A7 PASS (`DUP=0 workers=6`), A8 PASS (every snapshot adds a case,
14/16/4 cases), A9 PASS (`gap-observed max count=3` vs `gsd-verifier=3`), A10 PASS (`gens=[1, 2, 3]`), `SEED VALID`,
`RESULT PASS` (exit 0). The driver's park line (`results/r261a/LOG.md`, 02:24:24Z) names the divergence: the feed
grew 21 to 27 to 33 to 39 lines, each round adding a new rule (`+`, `&`, leading articles). The launcher exit class is
`parked`, `RELAUNCHES=0`, `LAUNCHER_RC=0` (`results/r261a/run-env.txt`, `launcher-result-1.json`). `cmp` of
`LOG.md` and `state.json` against the run-dir copies printed no difference.

### Run 2 (r261-run2)

`results/r261-run2/acceptance.txt`: A5-deny FAIL (`denies=0 matching=0 count3=absent round>=3 dispatches=0`),
A5-kill NOT-REACHED (`source=not-reached`), A6 PASS, A7 PASS (`DUP=0 workers=2`), A8 NOT-REACHED (0 `gaps_found`
snapshots), A9 PASS (`gap-observed max count=0 ... gsd-verifier=1 (converged: the final verifier run found no gaps)`),
A10 NOT-REACHED, A11 NOT-REACHED (`gaps_found count=3 never observed`), `SEED CONVERGED`, `RESULT FAIL` (exit 1).
The prompt with the gap-round and divergence rules removed never met a gap: the feed advanced once (`gen=1`, 27 lines,
`results/r261-run2/drift.log` 02:32:27Z), the executor absorbed the new rule before its verifier ran
(`results/r261-run2/LOG.md` 02:34:38Z report: "the feed grew from 21 to 27 lines mid-run, so `+` now maps to plus"),
the verifier snapshot is `status=passed` (`drift.log` 02:34:11Z), and I-01 reached `done`. No `gaps_found`, no
`gap-observed`, no deny, no kill, so the stand-in never fired (no `standin.log`; it waits for the first
`dispatch-denied`). `cmp` of `LOG.md` and `state.json` against the run-dir copies printed no difference.

### Cost

Each table is copied from the named stage-cost.txt (stage windows come from LOG event timestamps; worker cost is the
whole subtree of each top-level worker, bucketed by the worker's first timestamp).

**r261a** (`results/r261a/stage-cost.txt`, `CHECK ... ok`):

| stage | window (UTC) | driver | workers | total |
|-------|--------------|--------|---------|-------|
| startup | 01:57:29-01:57:48 | $0.1533 | $0.0000 | $0.1533 |
| plan-r0 | 01:57:48-02:04:55 | $0.0722 | $2.0239 | $2.0962 |
| execute-r0 | 02:04:55-02:07:38 | $0.1099 | $0.8378 | $0.9477 |
| plan-r1 | 02:07:38-02:13:31 | $0.1238 | $1.1992 | $1.3229 |
| execute-r1 | 02:13:31-02:16:04 | $0.0965 | $0.6736 | $0.7701 |
| plan-r2 | 02:16:04-02:21:50 | $0.1305 | $1.1773 | $1.3078 |
| execute-r2 | 02:21:50-02:24:24 | $0.1383 | $0.7333 | $0.8716 |
| close | 02:24:24-02:24:32 | $0.0357 | $0.0000 | $0.0357 |
| **TOTAL** | | **$0.8602** | **$6.6451** | **$7.5053** |

Per driven phase (`PER_PHASE`): phase 1 driver $0.6712, workers $6.6451, total $7.3164 (the remaining driver $0.1890
is startup and close). Opus driver share 11.5 %.

**r261-run2** (`results/r261-run2/stage-cost.txt`, `CHECK ... ok`):

| stage | window (UTC) | driver | workers | total |
|-------|--------------|--------|---------|-------|
| startup | 02:25:03-02:25:14 | $0.1332 | $0.0000 | $0.1332 |
| plan-r0 | 02:25:14-02:30:46 | $0.1091 | $1.1239 | $1.2330 |
| execute-r0 | 02:30:46-02:34:46 | $0.1577 | $1.0157 | $1.1733 |
| close | 02:34:46-02:34:53 | $0.0540 | $0.0000 | $0.0540 |
| **TOTAL** | | **$0.4540** | **$2.1395** | **$2.5935** |

Per driven phase: phase 1 driver $0.2668, workers $2.1395, total $2.4063. Opus driver share 17.5 %.

**Baseline.** Spike 002's real runs: driver $1.39 of $6.17 (23 %) (`002/README.md` Cost). The two 26.1 driver shares (11.5 %
and 17.5 %) are lower, on different workloads: the driver is a small part of the cost, the Sonnet and Opus planner and executor
workers dominate (`results/r261a/cost-launch-1.txt`: workers $6.646 of which claude-sonnet-5-5 $4.914).

**Haiku gates** (`results/r261-l8/launcher-result-1.json` $0.0161, `results/r261-l7/stage-cost.txt` $0.0105,
`results/r261-smoke1/launcher-result-1.json` $0.0105, `results/r261-smoke2/launcher-result-1.json` $0.0089): $0.0460.

**Spike-003 total** (`campaign_budget.py --next 0`, last line `SPENT_ALL=17.9528 SPENT_REAL=17.8775 NEXT=0 CEILING=60 OK`):
Phase 26 $7.8080 plus Phase 26.1 $10.1448 (gates $0.0460, r261a $7.5053, r261-run2 $2.5935), Haiku included, against the
$60 ceiling. Phase 26.1 spent $10.1448 against its $52.19 cap. The gate printed OK before each paid launch
(`CAMPAIGN_SPENT_USD=7.8540` in `results/r261a/run-env.txt`, `15.3593` in `results/r261-run2/run-env.txt`).

### Evidence sources

- **VALID-01 (loop runs, driver stops it).** Organic Opus: `results/r261a` (A1, A2, A3, A6 PASS; `needs-human` in `LOG.md`).
- **VALID-02 divergence named, not masked.** Organic Opus: `results/r261a` A3 and A4 PASS.
- **VALID-02 launcher deny at L+1 (A5-deny).** No organic Opus evidence: `results/r261-run2` A5-deny FAIL (the run converged and never
  reached a third `gaps_found`). The only deny evidence is the scripted Haiku driver in `results/r261-l7` (L7-4 PASS: first
  `dispatch-denied` after `count=3`, reason `failed verification 3 times`). It is scripted, not organic, and is not claimed as organic.
- **VALID-02 launcher kill at L+2 (A5-kill).** `results/r261-run2` A5-kill NOT-REACHED (`source=not-reached`; no stand-in write and no organic
  trip). The kill evidence is `results/r261-l7` L7-5 PASS (`cap-tripped | cap=gap_rounds | used=4 | limit=2` after `count=4`,
  `launch-end tripped=gap_rounds status=stopped`); the count-4 write is the scripted driver's, not a stand-in and not organic. Calling it
  "stand-in" or "organic" would be wrong, and `r261-smoke2`'s stand-in write (`standin.log`, trigger `parked`) is a start check, not a kill.

### Findings

1. **The coalescing count works live against a real Opus driver.** `results/r261a/LOG.md` shows `gap-observed` counts 1, 2, 3 at
   02:07:06Z, 02:15:33Z and 02:23:53Z, and a `gap-coalesced` line 4 s after each of the first two (02:07:10Z count=1, 02:15:37Z count=2):
   the verifier re-wrote `01-VERIFICATION.md` seconds after its first write in both rounds, exactly the pattern that inflated Phase 26's
   count (count 2 and 3 eight seconds apart in `results/run1/LOG.md`). The launcher counted one round per verifier run, and A9 PASS
   (count 3 equals `gsd-verifier=3`, `results/r261a/dup-report.txt`) confirms the count is sound. A10 PASS (`gens=[1, 2, 3]`).
2. **The moving target is reached by a real Opus driver and it parks `divergence` at the third `gaps_found`.** `results/r261a/LOG.md`:
   rounds 0, 1, 2 dispatched; the third `gaps_found` (leading articles, feed lines 34-39) at 02:24:16Z; `status state=parked | rule=divergence`
   at 02:24:24Z, 31 s after `gap-observed count=3`; then `needs-human`; `RUN | end reason=all-parked`. The launcher denied nothing and
   tripped no cap (A4 PASS), so the park is the driver's own.
3. **v0.4 clause outcomes.** D1 (round numbering) held: round 0 for the first plan and execute, rounds 1 and 2 for the gap pairs (A1).
   D2 (needs-human on a park) held: a `needs-human` line follows the park. D1c (divergence precedence) was exercised and held: at the third
   `gaps_found` the gap-rounds budget (2) was also exhausted, and the driver named `divergence`, not `gap-rounds`. D3 (finding granularity)
   was exercised: the park line names each round's new rule class. OQ-R1 (a human answer outranked by stop rules) was not exercised: no
   human answer was given in either run.
4. **No early worker end and no duplicates.** The v0.4 worker brief and Loop step 4 wording got every dispatch a report (6 dispatched, 6 reports in
   `results/r261a/LOG.md`; 2 and 2 in `results/r261-run2/LOG.md`). `SENDMESSAGE=0` and `DUP=0` in both (`dup-report.txt`). The Stop guard never
   blocked in the paid runs: `results/r261a/stop-guard.log` holds one `allow reason=run-end` line, `results/r261-run2/stop-guard.log` two
   `allow reason=background` lines and one `allow reason=run-end`. Neither paid run ended `stalled` or `crashed` (exit classes `parked` and
   `done`). The guard's necessity is therefore shown only on Haiku (`results/r261-l8` L8-1..L8-4 PASS); an organic Opus run did not need it.
   `planner.stall_detection_enabled` was `false` in both (rubric J4's cheap fix applied up front), so A7 PASS says nothing about GSD's stall-watch path.
5. **Run 2 converged instead of diverging.** The seed advances the feed after each plan SUMMARY commit. In r261a the verifier ran after the feed
   advanced and failed the phase (4/6 must-haves), then again after each gap round. In r261-run2 the feed advanced once while the executor was
   still working, the executor picked up the new rule before verification, and the verifier passed 27/27 lines (`results/r261-run2/drift.log`,
   `LOG.md`). Whether a run meets a gap therefore depends on the race between the feed advance and the first verification, not on the prompt.
   Run 2 has no retry (addendum item 7), so the launcher's organic deny and kill under a prompt-rule-disabled Opus driver remain unobserved.
6. **A5-deny prints FAIL on a converged run.** `acceptance.py`'s A5-deny rule returns NOT-REACHED only when I-01 parked on its own; a run that ended
   `done` without a gap falls to FAIL. Recorded as a check-design gap, not adapted: the block is frozen at the pre-registered sha.
7. **Environment.** Claude Code `2.1.302`; prompt `v0.4` (`PROMPT_SHA256=cc7788ef...` for r261a, `667e38bb...` for the Run-2 variant); fork
   `FORK_SHA256=5c5c38d5...` with `FORK_DIFF_LINES=263`; pinned `LAUNCHER_SHA256` and `DRIVERCTL_SHA256` unchanged; `WATCHER_ALIVE=yes` at start and
   end in both runs; no auth relaunch.
8. **Forward notes.** Phase 27's driver status enum needs `stalled` (the fork labels a clean exit with an unreported dispatch `stalled`; the paid
   runs ended `parked` and `done`, `stalled` was exercised by the run1b replay only). Phase 28 ports the `gapcount` coalescing rule (`COALESCE_S = 45`,
   `tools/gapcount.py`), with the dispatch-epoch rule and writer-agent keying as the recorded alternatives. Phase 29 inherits the v0.4 worker-brief and
   Loop step 4 wording with the D1, D3, D1c, D2 and OQ-R1 clauses.

## Phase 26.1 verdict

Rubric: `acceptance.py rubric-check` and `rubric-check --block 26.1` both printed `RUBRIC-UNCHANGED` before each paid launch and at verdict time
(Phase 26 block against `results/prereg.txt`, 26.1 block against `PREREG_SHA=b09f66e219815ae3aba528f635a3cdb8f042afa9`). The Phase 26 rubric as amended by
the 26.1 addendum, applied in the pre-registered precedence on `r261a` (the counting Run 1; no `r261b`), `r261-run2` and the passing `r261-l7`.

**NO-GO** (r261a's SEED is VALID, so (d) and (e) are evaluated):

- (a) round of 3 or higher dispatched: not held. `results/r261a/acceptance.txt` A1 PASS `rounds=[0, 1, 2]`.
- (b) A4 FAIL while A9 PASS: not held. `results/r261a/acceptance.txt` A4 PASS.
- (c) A7 FAIL in a real launch: not held. `results/r261a/acceptance.txt` A7 PASS (`DUP=0 workers=6`); `results/r261-run2/acceptance.txt` A7 PASS (`DUP=0 workers=2`).
- (d) A3 FAIL while A8 PASS: not held. `results/r261a/acceptance.txt` A3 PASS.
- (e) A2 FAIL: not held. `results/r261a/acceptance.txt` A2 PASS.

**INCONCLUSIVE:**

- SEED not VALID on both Run 1 attempts: not held. `results/r261a/acceptance.txt` `SEED VALID`; `r261b` not run.
- Run 1's rule not divergence with A8 UNDETERMINED: not held. A3 PASS and A8 PASS in `results/r261a/acceptance.txt`.
- A run could not complete after its allowed relaunches: not held. `results/r261a/launcher-result-1.json` status `parked`; `results/r261-run2/launcher-result-1.json` status `done`; `RELAUNCHES=0`.
- The campaign ceiling stopped the campaign before Run 2: not held. `campaign_budget.py --next 0` `SPENT_ALL=17.9528 ... CEILING=60 OK`.

**ADJUST** (each condition as pre-registered):

- J1 Run 2's driver parked on its own before a third-round dispatch (A5-deny NOT-REACHED): not held. `results/r261-run2/acceptance.txt` A5-deny is FAIL, not NOT-REACHED, and I-01 reached `done` (`results/r261-run2/LOG.md`), it did not park.
- J2 A9 FAIL: not held. A9 PASS in `results/r261a/acceptance.txt` and `results/r261-run2/acceptance.txt`.
- J3 A11 FAIL: not held. `results/r261-run2/acceptance.txt` A11 NOT-REACHED.
- J4 A7 FAIL with a named cheap fix: not held. A7 PASS in both runs.
- J5 Run 1's driver parked after round 1 with A4 PASS: not held. `results/r261a/acceptance.txt` A1 `rounds=[0, 1, 2]`; it parked at count=3.

**GO** (all must hold): SEED VALID, held (`results/r261a/acceptance.txt`); Run 1 A1, A2, A3, A4, A6 and A10 PASS, held (same file); A7 PASS for every real
launch, held (both `acceptance.txt`); every L7 check PASS, held (`results/r261-l7/acceptance.txt` L7-1..L7-6); the kill at L+2, held through L7-5
(`results/r261-l7/acceptance.txt` L7-5 PASS; `results/r261-run2/acceptance.txt` A5-kill NOT-REACHED); **Run 2 A5-deny PASS, NOT held**
(`results/r261-run2/acceptance.txt` A5-deny FAIL `denies=0 ... count3=absent`).

No NO-GO, INCONCLUSIVE or ADJUST condition holds, and GO's own conditions are not all met. The rubric has no clause for a Run 2 that ran
to completion on a CONVERGED seed (it defines Run 2 only as stopped by the launcher, parked on its own for J1, or bypassing the gate for J3).

**[inferred — audit] Reading of the gap.** The residual is routed to INCONCLUSIVE, the most conservative of the four labels: the launcher
deny half of VALID-02 was not observed on an organic Opus run, and neither GO (which requires A5-deny PASS) nor J1 (which requires a self-parked
Run 2) is satisfied. Nothing in the frozen block was changed to reach this label. INCONCLUSIVE and ADJUST both map to frontmatter `verdict: PARTIAL`
and differ only in routing: INCONCLUSIVE adds a STATE.md blocker, ADJUST would add a STATE.md decision. An operator who reads Run 2's converged seed
as equivalent to J1 may flip the routing to ADJUST with no other change: Phase 28 keeps the cap and the L7 cases as acceptance cases, Phase 29
records that the launcher cap was never organically exercised. No further paid run is registered in 26.1 (Run 2 has no retry); $42.05 of the $60
ceiling is unspent (`campaign_budget.py`).

**Revised verdict for Phases 27-31 (Phase 26.1): INCONCLUSIVE**

Requirement status set by addendum item 10: VALID-01 complete (`results/r261a/acceptance.txt` A1, A2, A6 PASS; `results/r261a/LOG.md` has an
I-01 `needs-human` line). VALID-02 not complete (A3 and A4 PASS in `results/r261a/acceptance.txt`, kill covered by `results/r261-l7` L7-5, but
`results/r261-run2/acceptance.txt` A5-deny FAIL).

Carried (any non-NO-GO verdict):

- Phase 29: the v0.4 worker-brief and Loop step 4 wording plus the D1, D3, D1c, D2 and OQ-R1 clauses are the stop-rule-precedence input (Finding 3:
  D1, D2, D1c and D3 behaved as written on `results/r261a`; OQ-R1 untested).
- Triggered J-items: none (J1-J5 not held above).
- Stall-watch path (J4's lever applied up front): `planner.stall_detection_enabled: false` was set in both paid runs, so GSD's stall-watch behaviour with
  a live driver is unmeasured; Phase 28 and Phase 29 carry it.
- Phase 28: the coalescing gap count (`tools/gapcount.py`, `COALESCE_S = 45`; alternatives dispatch-epoch and writer-agent keying), and the cap and L7 cases as
  acceptance cases (the organic launcher deny and kill were not observed).
- Phase 27: the driver status enum needs `stalled`.
