---
phase: 29-master-prompt-run-templates
plan: 08
subsystem: acceptance-live-runs
tags: [acceptance, paid-runs, haiku, opus, consent, ledger, d13, d17, d18, d19, d23, d24, d25]

requires:
  - phase: 29-master-prompt-run-templates
    provides: "29-07 harness scripts/prompt-acceptance.sh, rubric pre-registered at acceb35, rehearsals"
provides:
  - "operator cost consent record (approve-all, $25 ceiling) committed alone"
  - "live evidence and re-derivable verdicts: h1 PASS, h2 FAIL (H2-5), h3 PASS, h4 PASS, o1 ADJUST x2"
  - "campaign ledger totalling $12.7557"
affects: [29-verification, carried-risk-26.1, PRMT-03, PRMT-04, DRIVE-01]

actuals:
  tokens: 40000
  tasks: 3
  commits: 7

plan_head_before: 55385faf86d4eb698e42fbb54316cf744331d8d1
plan_head_after: 92e8b04842eb358d1727dd31e6bc3865880bef00
commits: 7

tech-stack:
  added: []
  patterns:
    - "one evidence commit per case immediately after check, so an interruption cannot strand evidence"

key-files:
  created:
    - .planning/phases/29-master-prompt-run-templates/acceptance/consent.txt
    - .planning/phases/29-master-prompt-run-templates/acceptance/campaign.txt
    - .planning/phases/29-master-prompt-run-templates/acceptance/h1/
    - .planning/phases/29-master-prompt-run-templates/acceptance/h2/
    - .planning/phases/29-master-prompt-run-templates/acceptance/h3/
    - .planning/phases/29-master-prompt-run-templates/acceptance/h4/
    - .planning/phases/29-master-prompt-run-templates/acceptance/o1/
  modified: []

key-decisions:
  - "h2 is reported as FAIL and NOT rerun: H2-5 failed on logic, not infrastructure (observed.txt infra_fault=no), and the rubric forbids logic reruns. The verdict is untouched."
  - "o1 got exactly one retry after the first ADJUST (ledger 5.41 + 10 within $25); two ADJUSTs are recorded as ADJUST with evidence, no third attempt (D-25)."
  - "Requirements PRMT-03, PRMT-04 and DRIVE-01 are left unmarked complete because their live evidence is ADJUST or FAIL [inferred - audit]; PRMT-01 and DRIVE-03 are marked."

requirements-completed: [PRMT-01, DRIVE-03]

duration: 3h (resumed after a host reboot; h3 ran before it)
completed: 2026-10-20
status: complete

coverage:
  - id: D1
    description: "Operator cost consent recorded before any spend"
    verification:
      - kind: other
        ref: "git show e7006ca -- acceptance/consent.txt (touches only that file)"
        status: pass
    human_judgment: false
  - id: D2
    description: "h1 live: SIGKILL mid-dispatch, relaunch logs exactly one I-01 recovered before its first dispatched"
    requirement: PRMT-04
    verification:
      - kind: other
        ref: "bash scripts/prompt-acceptance.sh check h1 -> RESULT PASS"
        status: pass
    human_judgment: false
  - id: D3
    description: "h3 live: an answer alone lifts nothing, a gap-rounds budget edit does (OQ-R1)"
    requirement: PRMT-03
    verification:
      - kind: other
        ref: "bash scripts/prompt-acceptance.sh check h3 -> RESULT PASS"
        status: pass
    human_judgment: false
  - id: D4
    description: "h4 live: stripped prompt yields one dispatch-denied, driver parks launcher-cap, never retries"
    requirement: PRMT-03
    verification:
      - kind: other
        ref: "bash scripts/prompt-acceptance.sh check h4 -> RESULT PASS"
        status: pass
    human_judgment: false
  - id: D5
    description: "h2 live: pause mid-dispatch honoured (H2-1..4 pass) but the relaunch logged no recovered for I-01 (H2-5 FAIL)"
    requirement: PRMT-04
    verification:
      - kind: other
        ref: "bash scripts/prompt-acceptance.sh check h2 -> RESULT FAIL"
        status: fail
    human_judgment: true
    rationale: "Whether H2-5 is a rubric over-specification (I-01 had already finished, so D-17 requires no recovered) or a prompt defect is a judgment for the operator."
  - id: D6
    description: "o1 live (real Opus, embedded prompt, cap 2): no deny, no cap trip, but the driver self-parked before a third gaps_found in both attempts"
    requirement: DRIVE-01
    verification:
      - kind: other
        ref: "bash scripts/prompt-acceptance.sh check o1 -> RESULT ADJUST"
        status: unknown
    human_judgment: true
    rationale: "Two ADJUSTs; closing the 26.1 rules-on carried risk needs the operator to accept or redesign the seed (D-25: no loop)."
---

# Phase 29 Plan 08: Paid Acceptance Summary

**The paid campaign ran inside the $25 ceiling at $12.7557: h1, h3 and h4 PASS under a real Haiku driver, h2 FAILs on one check (H2-5), and the real-Opus case o1 ended ADJUST twice (the driver self-parks before a third gaps_found), so the 26.1 rules-on carried risk stays open pending the operator.**

## Operator decision (Task 1)

`approve-all`, cases `h1 h2 h3 h4 o1`, ceiling $25, decided 2026-10-20T04:37:56Z, recorded alone at `e7006ca`. The decision was relayed by the orchestrator from the operator's reply; the checkpoint itself was satisfied before this continuation started.

## Verdicts

| Case | RESULT | Evidence commit | Notes |
|------|--------|-----------------|-------|
| h3 | PASS | f0a45a8 | ran before a host reboot interrupted the executor; evidence committed on resume; no rerun |
| h4 | PASS | 8d22af7 | one organic `dispatch-denied` (`has failed verification`), I-01 parked `launcher-cap`, never retried, `prompt=custom:f2f99387` |
| h1 | PASS | 8666dff | one `recovered` (line 59) before launch 2's first `dispatched` (line 61) |
| h2 | FAIL | be0f8f8 | H2-1..H2-4 PASS, H2-5 FAIL; not rerun (logic fault) |
| o1 attempt 1 | ADJUST | 7f229e9 | parked I-01 `gap-rounds` at the second gaps_found |
| o1 retry | ADJUST | 92e8b04 | rounds 0,1,2 dispatched, parked `needs-human` before count 3 |

Every verdict re-derives with `bash scripts/prompt-acceptance.sh check <case>` (exit 0).

### h2 analysis (FAIL, honest)

The pause worked as designed: the `control: pause` save at 13:29:09 landed while the worker ran; the worker's `report` was recorded, I-01 was verified done, `RUN | end | reason=pause` followed and no dispatch came after the save (H2-1..H2-4 PASS). Launch 2 logged `reconcile | found=0` and dispatched I-02 with no `recovered`, so H2-5 failed. The likely reason is that I-01 was already `done` when launch 1 ended, and D-17 requires a `recovered` only for an item left `running`. That points at the rubric premise (a worker that outlives the pause save, so nothing is left running) rather than at the prompt, but it is an inference and the verdict stands as FAIL. The 29-07 rehearsal could not catch it because the fake worker's timing differs. Crash reconciliation under a real model is shown by h1.

### o1 analysis (two ADJUSTs)

Both attempts used `prompt=driver-prompt/1.0.0` (O1-version PASS), produced zero `dispatch-denied` and zero `cap-tripped` (O1-nodeny PASS) and short, well-formed reports (O1-report PASS). The Opus driver used at least two GSD commands (`/gsd-plan-phase`, `/gsd-execute-phase`, with `--gaps` and `--gaps-only`).
- Attempt 1 ($4.7953): parked `gap-rounds` after the second gaps_found, citing `driverctl caps.gap_rounds used 2/2`; the seed feed grew 21->27->33 lines mid-run.
- Retry ($7.3440): dispatched rounds 0, 1 and 2, then parked `needs-human` after the round-2 gap plan because the feed contradicted SLUG-01 (`&`->`n`), before a third gaps_found existed (O1-rounds and O1-park NOT-REACHED).
Both are the self-limiting pattern (r261-run2b) the rubric classes as ADJUST: the rules held and the launcher backstop never needed to act, but the exact count-3 park was not observed. Two ADJUSTs are recorded; no third attempt (D-25).

## Cost ledger (`acceptance/campaign.txt`)

| Launch | USD | Source |
|--------|-----|--------|
| h3 launch 2 | 0.0090 | result |
| h3 launch 3 | 0.15 | killed-full-budget (deliberate `driver stop`) |
| h4 launch 1 | 0.0103 | result |
| h1 launch 1 | 0.40 | killed-full-budget (SIGKILL by design) |
| h1 launch 2 | 0.0138 | result |
| h2 launch 1 | 0.0176 | result |
| h2 launch 2 | 0.0157 | result |
| o1 attempt 1 | 4.7953 | result |
| o1 retry | 7.3440 | result |
| **Total** | **12.7557** | ceiling 25 |

## Carried-risk disposition (26.1)

- Rules-disabled backstop half: closed by 28-09 d33 (reused, D-23) and bound to the embedded text by h4 PASS.
- Rules-on half under a real Opus driver with the launcher cap at L=2: NOT closed. Two ADJUSTs with evidence; pending operator sign-off. No evidence of a defect (no deny, no cap trip, driver self-limits), but SC3's third-gaps_found park was not observed live.

## Final gate

- `cargo test --no-fail-fast -- --skip the_config_section_constants_record_the_git_version_they_were_derived_against`: 86 suites, 3680 passed, 0 failed, 24 ignored (includes `tests/prompt_acceptance.rs`).
- `cargo clippy --all-targets -- -D warnings`: exit 0.

## Deviations from Plan

**1. [Interruption] Host reboot between h3's run and its evidence commit**
- **Found during:** Task 2 (prior executor)
- **Issue:** h3 ran to completion but its evidence and ledger were untracked when the host rebooted.
- **Fix:** `check h3` re-derived RESULT PASS on resume and the evidence was committed (f0a45a8) without a rerun; no processes survived.

**2. [Reported result] Task 2 acceptance criterion not met for h2**
- The criterion "four PASS verdicts" is not met: h2 is FAIL (see analysis). The plan's verify-command `check h2` exits 0 but prints `RESULT FAIL`. Per the rubric no logic rerun was made and nothing was edited.

**3. [Reported result] Task 3 reached ADJUST, not PASS** (permitted outcome; recorded per D-25).

**4. [Bookkeeping] Requirements** PRMT-03, PRMT-04 and DRIVE-01 left unmarked complete [inferred - audit]; see key-decisions.

**Total deviations:** 1 interruption, 2 reported non-PASS outcomes, 1 bookkeeping choice. **Impact:** all spend authorised and within the ceiling; live acceptance is incomplete for h2 and o1 and needs operator disposition.

## Known Stubs

None.

## Threat Flags

None. The committed evidence is derived files only; `envelope scan` reported no findings on the h2 and o1 scratch runs and `tests/prompt_acceptance.rs` (committed-tree scan) passed.

## Self-Check: PASSED

- consent.txt, campaign.txt and the five case directories exist.
- Commits e7006ca, f0a45a8, 8d22af7, 8666dff, be0f8f8, 7f229e9, 92e8b04 are ancestors of HEAD.
- Ledger sum 12.7557 <= 25; prereg commit is an ancestor of HEAD.
