---
spike: 003
idea: queue-driven-master-agent
name: divergent-gap-loop-validation
type: standard
validates: "Given the unmodified spike-002 launcher and driverctl and a v0.3 Opus driver on a real GSD scratch project whose verifier criterion tests a feed that advances after every plan SUMMARY commit, when it runs gap closure, then it dispatches rounds 1 and 2, parks needs-human with rule divergence at the third gaps_found without any launcher deny, a prompt-rule-disabled run is stopped by the launcher gap cap at L+1/L+2, no planner or executor runs twice, and the Opus driver cost per stage and per phase is measured"
verdict: PENDING
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

Recorded by plan 26-03 after the runs. Until then no number in this README is a measurement.

## Not validated

The spike-002 list stays open: setup-token authentication, macOS, interactive mode, parallel items, and
rate-limit exhaustion.
