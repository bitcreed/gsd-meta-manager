---
phase: 26-driver-validation-spike-003
plan: 03
subsystem: spike-experiment
tags: [spike-003, opus-driver, gap-loop, divergence, inconclusive, cost-measurement]
requires:
  - phase: 26-driver-validation-spike-003 plan 01 (seed, launch kit, L7)
  - phase: 26-driver-validation-spike-003 plan 02 (measurement tools, frozen rubric)
provides:
  - results/prereg.txt (PREREG_SHA recorded before the first Opus launch)
  - results/l7 dup-report, stage-cost, acceptance (all L7 checks PASS)
  - results/run1 and results/run1b (two measured Opus launches), results/run2/SKIPPED.txt
  - spike 003 README results, cost tables, evidence sources, findings and verdict; MANIFEST row 003; one STATE blocker
affects: [phase-27, phase-28, phase-29]
tech-stack:
  added: []
  patterns: [pre-registered rubric applied from committed files, scratch roots outside the repo, derived-files-only results]
key-files:
  created:
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/prereg.txt
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/l7/{acceptance,dup-report,stage-cost}.txt
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/run1/
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/run1b/
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/run2/SKIPPED.txt
  modified:
    - .planning/spikes/003-divergent-gap-loop-validation/README.md
    - .planning/spikes/MANIFEST.md
    - .planning/STATE.md
key-decisions:
  - "[inferred] Verdict INCONCLUSIVE: NO-GO (d)/(e) are literally satisfiable by the two STALLED runs but are judged only on a Run 1 with a VALID seed; none exists"
  - "[inferred] Neither Run 1 attempt counts as 'the Run 1'; run1b was the pre-registered as-is retry for STALLED and was not VALID"
  - "[inferred] A launcher status of crashed with RELAUNCHES=0 is 'a run could not complete after its allowed relaunches' (run.sh relaunches only on auth revocation)"
requirements-completed: [VALID-01, VALID-02, VALID-03, VALID-04]
duration: ~80 min wall (two Opus launches of 18 min and 9 min)
completed: 2026-10-17
status: complete
commits: 5
plan_head_before: 46090a5624da80aeec91f20b2e70d7a7daa3eae0
plan_head_after: 0c5bf9043d67c6e2b76560a03c33a3be506e8fd4
actuals:
  tokens: 22100   # chars/4 over added lines of the five task commits (88465 chars); not a harness token count
  tasks: 3
  commits: 5
---

# Phase 26 Plan 03: Runs and verdict Summary

Two real Opus launches of the unmodified spike-002 launcher both ended with SEED STALLED, so by the pre-registered rubric the spike verdict is INCONCLUSIVE, Run 2 was skipped, and STATE.md carries a blocker holding Phase 27. VALID-03 (DUP=0) and VALID-04 (per-stage driver cost) are measured; VALID-01 and the divergence half of VALID-02 are not demonstrated.

Requirements VALID-01..04 are listed in the frontmatter as the plan's targets; the evidence for VALID-01 and the Opus half of VALID-02 was NOT obtained (see Verdict). Do not read `requirements-completed` as satisfied evidence for those two.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (pre-flight, prereg) | 3c62a3d | chore(26-03): preregistration sha and L7 measurement reports before first Opus launch |
| 1 (run1) | 604dfd7 | feat(26-03): Run 1 attempt 1 - Opus driver on moving-target seed, measured (SEED STALLED) |
| 1 (run1b) | 03837c1 | feat(26-03): Run 1 attempt 2 (run1b) - as-is retry ... (SEED STALLED) |
| 2 | 47708b9 | chore(26-03): record Run 2 skipped - seed not VALID on both Run 1 attempts |
| 3 | 0c5bf90 | docs(26-03): spike 003 verdict INCONCLUSIVE - README results, cost, MANIFEST row 003, STATE blocker |

`prereg.txt` (3c62a3d) is an ancestor of the run1 commit (604dfd7), checked with `git merge-base --is-ancestor`. `PREREG_SHA=4ba61955a3a847a5005af2ec17d6771479d5f0ed`; `rubric-check` printed `RUBRIC-UNCHANGED` before Run 1, after run1b and at verdict time.

## What happened

- **Pre-flight:** `make_variant.py --check`, `drift.py` and the three tool selftests all passed; spike 002 unchanged (`git diff a16e338` clean). `acceptance.py l7` over the existing `results/l7` printed L7-1..L7-6 PASS, `RESULT PASS`; `dup_check.py` exit 0.
- **run1** (scratch root `~/.local/state/gsd-meta-manager/spike-003/run1-20261017T023945Z`, 1 launch, 0 relaunches, 18 min): the Opus driver ran plan and execute for phase 1 (round 0), observed `gaps_found`, and dispatched a gap round (`/gsd-plan-phase 1 --gaps`, `/gsd-execute-phase 1 --gaps-only`, round 1). A second verifier re-write of `01-VERIFICATION.md` 8 s after the first made the launcher log `gap-observed count=2` and `count=3` for one gap round (two verifier runs); the driver parked `rule=gap-rounds` citing the launcher count. A1 FAIL, A2 PASS, A3 FAIL, A4 PASS, A6 PASS, A7 PASS, A8 PASS, A9 FAIL (3 vs 2), A10 FAIL (gens 1,2,2), SEED STALLED. $5.1667.
- **run1b** (scratch root `.../run1b-20261017T025830Z`, 1 launch, 0 relaunches, 9 min): pre-registered as-is retry. The driver dispatched the round-0 plan worker and ended its turn; the session exited rc=0 without `RUN end`; launcher status `crashed`. A1/A2/A3 FAIL, A4/A6/A7 PASS, A8/A9/A10 NOT-REACHED, SEED STALLED. $2.6120.
- **run2:** not launched (`results/run2/SKIPPED.txt`): the rubric runs Run 2 only after a VALID Run 1.
- **Integrity:** `cmp` of results `LOG.md` and `state.json` against the run-dir copies printed no difference for run1 and run1b; the secret-pattern gate was clean over `results/`, README and MANIFEST (the only repo-wide match in the spike dir is `run.sh`'s own gate pattern at line 187, a quirk of the plan's broader grep). No transcript or stream file was committed.

## Verdict

**Verdict for Phases 27-31: INCONCLUSIVE** (README frontmatter `verdict: PARTIAL`).

Deciding lines: INCONCLUSIVE "SEED not VALID on both Run 1 attempts" held (`results/run1/acceptance.txt` SEED STALLED; `results/run1b/acceptance.txt` SEED STALLED), and "a run could not complete" held (`results/run1b/launcher-result-1.json`, status crashed). The ceiling did not stop the campaign ($7.81 of $60).

**Judgement call for audit (the only one that could change the label):** NO-GO is evaluated first. Read literally, (d) A3 FAIL while A8 PASS holds for run1 and (e) A2 FAIL holds for run1b. I did not apply them, because run1's A3 FAIL comes from the same measurement artefact that fails A9 and A10, and run1b's A2 FAIL comes from a session that ended after one dispatch with no gap evidence; applying NO-GO to a STALLED run would make the rubric's own INCONCLUSIVE route unreachable for any crash. Both labels route identically (a STATE blocker holding Phase 27), but NO-GO would set `verdict: INVALIDATED`. The full argument is in the README Verdict section.

## Measurements

| Run | Opus driver | Workers | Total | Driver share |
|-----|-------------|---------|-------|--------------|
| run1 (`results/run1/stage-cost.txt`) | $0.7545 | $4.4122 | $5.1667 | 14.6 % |
| run1b (`results/run1b/stage-cost.txt`) | $0.3170 | $2.2950 | $2.6120 | 12.1 % |

run1 per stage (driver / workers): startup 0.1270/0, plan-r0 0.1015/0.7632, execute-r0 0.1215/1.5200, plan-r1 0.1508/1.2250, execute-r1 0.2208/0.9040, close 0.0329/0. Phase 1 driver $0.5947. Spike 002 baseline driver share 23 %. Opus real-launch total $7.7787; whole phase including Haiku runs $7.8080 (L7 0.0122 priced via `--rates-from`, smoke1 0.0088, smoke2 0.0083). `DUP=0`, `EXECUTOR_MISSING=0`, `SENDMESSAGE=0` for both launches. A5-kill source: not-reached by an Opus run (no Run 2); the kill evidence is `results/l7` L7-5 (scripted Haiku driver, labelled as such). Launcher deny evidence is also L7 only (L7-4).

## Deviations from Plan

None to code. Plan executed as written: the retry rule (STALLED, change nothing) and the skip rule were applied exactly.

One plan-vs-reality note: the Task 3 secret-pattern grep over the whole spike directory matches `experiment/run.sh` (it contains the gate's own pattern). Results, README and MANIFEST are clean; no change was made.

## [inferred] decisions

- INCONCLUSIVE rather than NO-GO (see Verdict).
- No run is "the Run 1 that counts"; the README documents both attempts and judges NO-GO conditions only on a VALID-seed run.
- A `crashed` launch with no auth revocation counts as "could not complete after its allowed relaunches".
- Carried items under INCONCLUSIVE: v0.3 D1/D3/D1c/D2/OQ-R1 wording to Phase 29 (D1, D2 observed as written; D1c, D3, OQ-R1 untested); J2 and J5 recorded as observations, not as triggered ADJUST items (precedence).
- MANIFEST gained two `(spike 003)` bullets: the wording carry, and the redesign observation (verifier re-write over-counts gaps; wake-up failure).
- The post-hoc suggestion to dedupe snapshots and count gaps per verifier run is recorded as a redesign input only; the frozen rubric and instruments were not adapted to the results.

## STATE entry

`- [Phase 26] Spike 003 verdict INCONCLUSIVE: redesign the seed and instruments (divergence signal robust to a verifier re-write of VERIFICATION.md; headless-driver wake-up before a worker reports) or re-plan Phases 27-31 (NO-GO if the rubric's (d)/(e) are read literally) before Phase 27 starts — see .planning/spikes/003-divergent-gap-loop-validation/README.md` (added through `state.add-blocker`; the `[Phase 26] Several risks are still unproven` blocker is intentionally left open).

## Known Stubs

None.

## Threat Flags

None. Mitigations T-26-12..16 held: derived files only in results, secret gate re-run before each commit, per-launch `--max-budget-usd 15` with measured total $7.81 of $60, `cmp` against run dirs, unmodified spike 002, scratch outside the repo.

## Issues Encountered

- Neither Opus launch reached a second gap round. The first was ended by a launcher gap count inflated by one verifier re-write, the second by the headless session exiting with a worker unreported. Both are recorded as Findings 2 and 5 in the spike README.
- `ROADMAP.md`, `REQUIREMENTS.md`, `src/`, `tests/` and Cargo files are untouched; tracking updates are left to the orchestrator as instructed.

## Self-Check: PASSED

- Files present: `prereg.txt`, `results/{l7,run1,run1b}/{acceptance,dup-report,stage-cost}.txt`, `results/run2/SKIPPED.txt`, README, MANIFEST row 003, STATE blocker (`grep -c "Spike 003 verdict" .planning/STATE.md` = 1).
- Commits 3c62a3d, 604dfd7, 03837c1, 47708b9, 0c5bf90 are ancestors of HEAD; `commits: 5` measured with `rev-list --count 46090a5..HEAD` before this SUMMARY commit.
