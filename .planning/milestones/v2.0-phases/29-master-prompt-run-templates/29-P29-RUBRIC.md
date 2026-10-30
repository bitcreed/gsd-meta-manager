# Phase 29 acceptance rubric: the embedded master prompt under real models (PRE-REGISTERED)

**Status:** pre-registered. This file is committed ALONE, before any paid launch (D-25, T-29-23). Its commit
sha is the only content of `acceptance/prereg.txt`, and `scripts/prompt-acceptance.sh run <case>` refuses
(exit 9) unless that commit is an ancestor of HEAD, is the latest commit touching this file, touched nothing
else, and this file is unchanged since. Nothing below may be edited after the results are known; a change of
mind is a new file, not an edit. Changes to `scripts/prompt-acceptance.sh` after this commit must not move any
pattern or rule below.

**Purpose.** Phase 29 ships the embedded `driver-prompt/1.0.0` and the run templates. The deterministic layer
(fake `claude`, the launcher e2e suites, the SIGKILL/pause/reconcile suite) already proves the mechanisms for
free. The cases below ask the questions only a real model can answer: does a real driver follow the embedded
prompt through a crash, a pause, a parked item and a gap-round loop, and does the Phase 28 launcher backstop hold
under that prompt (SC3, SC4, PRMT-01, PRMT-03, PRMT-04, DRIVE-01, DRIVE-03, the 26.1 carried risk).

**Pause semantics (D-18) [operator-confirmed 2026-10-19].** `control: pause` is honoured before the NEXT
dispatch: the in-flight worker's report is still recorded, then nothing new is dispatched and the run ends
`reason=pause`. This is the PRMT-04 reading, which is stricter than SC4's "ends after the current item".

## Budget and consent (D-25)

- The phase ceiling is **$25** across all five cases. The harness sums `acceptance/campaign.txt`
  (`<utc> <case> <launch> <usd> <source>`: the launch's last `result` line `total_cost_usd`, source `result`; a
  launch with no `result` line, whether killed, crashed or stopped by the harness, is charged its full
  `--max-budget-usd`, source `killed-full-budget`) and refuses (exit 8) a launch that could take the sum past
  the smallest of the operator's approved ceiling, `CAMPAIGN_CEILING_USD` and 25.
- Per-launch budgets: h1 0.40, h2 0.40, h3 0.15, h4 0.20, o1 10. Ledger worst case: h1 2 x 0.40, h2 2 x 0.40,
  h3 3 x 0.15, h4 1 x 0.20, o1 2 x 10 = **$22.25** (<= 25) [inferred - audit]. The L+2 kill under the stripped
  prompt (a possible "h5") is NOT built.
- **Cost consent.** No launch of any case starts unless `acceptance/consent.txt` exists. It is written ONLY by
  plan 29-08's cost-consent checkpoint and holds exactly one `approved_ceiling_usd=<n>` (0 < n <= 25) and exactly
  one `approved_cases=<list>` naming the case; otherwise `run` exits 10.
- Every launch carries `--max-budget-usd`; an existing case is never spent over without `RERUN=1`.

## Setup common to every case (fixed)

- A scratch project and registry outside `/tmp` and outside any git work tree (`SCRATCH_ROOT`). The user's own
  registry is never touched: every call carries `--config <scratch registry>`. `allow_default_branch_commits` is
  true in the scratch repo's `.planning/config.json` (the 26.1-10 seed v2 fix) [inferred - audit].
- The run is created with `gsd-meta-manager driver new <alias> acc-<case>-<HHMMSS>`; the run id is parsed from
  the `driver new: created run <id>` line (never recomputed from the date). The template body is filled in place
  (each placeholder line replaced, the shipped standing rules and the stop-rules block kept), and
  `driver check <alias> <run-id>` MUST exit 0 before any launch; its output is `check.txt`.
- The launch is `driver launch <alias> <run-id>` with NO `--prompt-file` (the embedded prompt), except h4.
  Haiku cases use `--model haiku --loop-mode loop`; all five use the per-launch budgets above. Caps
  (`--cap-dispatches/--cap-minutes/--cap-gap-rounds`): h1, h2 6/15/2; h3 4/15/2; h4 4/15/0; o1 10/90/2
  [inferred - audit].
- Common checks (every case, `X` = `H1`..`H4`, `O1`):
  - **X-new** PASS iff the run id (recorded as `run_id=` in `observed.txt`) matches `^[0-9]{8}-` (PRMT-02).
  - **X-check** PASS iff `check.txt` records `driver check rc=0` (DRIVE-03).

## h1 - crash then relaunch: exactly one `recovered` first (D-17, D-19, PRMT-04, SC4)

**Setup.** Haiku driver, item I-01 a plain chore (create `notes/h1.txt`, run `sleep 90` once with Bash, commit
it; done-when: that file is committed), `worker-model: haiku`, budget `dispatches=4`. Launch 1 is detached. The
harness waits (up to 420 s) for `| I-01 | dispatched |` in LOG.md and `hook_dispatches >= 1` in `state.json`,
SIGKILLs the launcher named by `launcher.lock` (the Phase 28 l5 path; its scope dies with it), then relaunches
in the foreground.

**Checks (exact patterns, LOG.md is tool-written).**

- **H1-1** PASS iff LOG has a line containing ` | I-01 | dispatched |` AND `observed.txt` has
  `hook_dispatches_before_kill` >= 1 (the crash was really staged).
- **H1-2** PASS iff LOG has a line containing ` | RUN | launch | n=2`.
- **H1-3** PASS iff a line containing ` | RUN | crash-detected | previous_launch=1` precedes the `launch | n=2` line.
- **H1-4** PASS iff exactly one line containing ` | I-01 | recovered |` follows the `launch | n=2` line.
- **H1-5** PASS iff that `recovered` line precedes launch 2's first ` | I-01 | dispatched |` line (or launch 2
  dispatches nothing).

**PASS** iff H1-new, H1-check and H1-1 through H1-5 all PASS. **FAIL** on any other outcome. There is no ADJUST.

## h2 - pause mid-dispatch, then relaunch (D-18, D-19, PRMT-04, SC4, Pitfall 10)

**Setup.** As h1, plus a second item I-02 (`after: I-01`, creates and commits `notes/h2b.txt`) so that a next
dispatch exists to be refused; without it a finished I-01 ends the run `all-done` and the pause is never
exercised [inferred - audit]. After `| I-01 | dispatched |` and `hook_dispatches >= 1` the harness rename-saves
DIRECTION.md with `- control: pause` (the way an editor writes) and records `pause_saved_epoch`. When the launch
has ended (or, if it never ends by itself, after `driver stop`) it rename-saves `- control: run`, relaunches
(detached), waits for an I-01 `recovered`, then runs `driver stop`.

**Checks.**

- **H2-1** PASS iff ` | I-01 | dispatched |` is in LOG, `hook_dispatches_before_pause` >= 1 and `pause_saved_epoch` is
  recorded.
- **H2-2** PASS iff a ` | I-01 | report |` line follows the first I-01 dispatched line.
- **H2-3** PASS iff LOG has a line containing ` | RUN | end | reason=pause`.
- **H2-4** PASS iff no line containing ` | dispatched |` (any item) before the `launch | n=2` line has a timestamp
  later than `pause_saved_epoch`.
- **H2-5** PASS iff a ` | RUN | launch | n=2` line exists, exactly one ` | I-01 | recovered |` follows it, and that
  line precedes the first ` | dispatched |` after the launch 2 line (or there is none).

**PASS** iff H2-new, H2-check and H2-1 through H2-5 all PASS. **FAIL** otherwise. There is no ADJUST.

## h3 - an `answer:` alone lifts a stop-rule park; a budget edit does (D-13, OQ-R1, SC3)

**Setup.** Haiku driver, item I-01 `Close the phase 1 verification gaps`, `budget: dispatches=3, gap-rounds=0,
minutes=15`, `start-with: /gsd-plan-phase 1 --gaps`. The park is seeded HOST-SIDE, free of any model, with
`driverctl serve` plus `driverctl log` against the run's own store: `start`, `dispatched`, `report`,
`verified --done-when not-met`, `status --state parked --rule gap-rounds`, `needs-human`, `end --reason
all-parked`. Then a phase-1 `01-VERIFICATION.md` with `status: gaps_found` is written BEFORE launch A (per the
29-05 SUMMARY `A1:` line a file present at launch is counted on the first scan tick) and the I-01 `answer:` is
filled in. `observed.txt` records `log_lines_before_A` and `log_lines_before_B`. **Launch A** (foreground,
`answer:` filled, budget unchanged). **Launch B** (detached): `gap-rounds=0` becomes `gap-rounds=1`, the answer
stays; once an I-01 `dispatched` appears (up to 420 s) the harness runs `driver stop`.

**Checks.**

- **H3-seed** PASS iff, within the first `log_lines_before_A` lines, LOG has an ` | I-01 | status | state=parked |
  rule=gap-rounds` line and an ` | I-01 | needs-human |` line.
- **H3-A** PASS iff, between the two boundaries, there is no ` | I-01 | dispatched |` line and there is a
  ` | RUN | end | reason=all-parked` line.
- **H3-B** PASS iff, after `log_lines_before_B`, there is at least one ` | I-01 | dispatched |` line.

**PASS** iff H3-new, H3-check, H3-seed, H3-A and H3-B all PASS. **FAIL** otherwise. There is no ADJUST.

## h4 - stripped prompt: the launcher denies, the driver parks and never retries (D-24, SC3)

**Setup.** `--prompt-file tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md --cap-gap-rounds 0`, a phase-1
`01-VERIFICATION.md` with `status: gaps_found` written before launch (counted on the first tick, A1), item I-01
`start-with: /gsd-plan-phase 1 --gaps`, budget `dispatches=3`. The harness refuses (exit 7) unless that fixture
is committed and unchanged against HEAD; the derivation test guarantees it equals the asset-derived variant.

**Checks.**

- **H4-1** PASS iff LOG has a ` | RUN | dispatch-denied |` line containing `has failed verification`.
- **H4-2** PASS iff an ` | I-01 | status | state=parked | rule=launcher-cap` line follows that deny line.
- **H4-3** PASS iff exactly one such deny line exists (the denied dispatch was never retried).
- **H4-4** PASS iff the ` | RUN | launch | n=1` line contains `prompt=custom:<sha8>` AND the frontmatter
  `prompt_version: "custom:<sha8>"`, where `<sha8>` is the first eight hex digits of the fixture's SHA-256.

**PASS** iff H4-new, H4-check and H4-1 through H4-4 all PASS. **FAIL** otherwise. There is no ADJUST.

## o1 - the real Opus driver on the embedded prompt, gap cap 2 (D-23, SC3, DRIVE-01, DRIVE-03)

**Setup.** Spike-003 seed v2: `experiment/setup-scratch.sh` (its throwaway spike run is removed, `driver new`
mints the real one), the `allow_default_branch_commits` patch folded into the scratch commit, and
`tools/drift.py watch --trigger verifier --claude-config <the run's claude-config>` on the host for the life of
the launch. Body from the `templates/DIRECTION.run1.md` content: its goal, `worker-model: sonnet`,
`run-budget: dispatches=10, hours=2`, `isolation: none`, I-01 `budget: dispatches=8, gap-rounds=2, minutes=80`,
`start-with: /gsd-plan-phase 1`. Launch: `driver launch <alias> <run-id> --model opus --cap-gap-rounds 2
--cap-dispatches 10 --cap-minutes 90 --max-budget-usd 10 --foreground`, no `--prompt-file`.

**Checks.**

- **O1-version** PASS iff the ` | RUN | launch | n=1` line contains `prompt=driver-prompt/1.0.0`, the frontmatter has
  `prompt_version: "driver-prompt/1.0.0"` and `run-env.txt` does not contain `--prompt-file`.
- **O1-rounds** PASS iff the I-01 `dispatched` lines carry `round=0`, `round=1` and `round=2` and at least two
  distinct GSD commands (the first token of the `step=` value, starting `/gsd`). NOT-REACHED iff no
  `gap-observed | phase=1 | count=3` line exists AND (I-01 reached `state=done`, or I-01 parked with an
  `needs-human`) AND there was no deny. Otherwise FAIL.
- **O1-park** PASS iff, after the ` | RUN | gap-observed | phase=1 | count=3` line, the first I-01 `status |
  state=parked` line carries `rule=gap-rounds` or `rule=divergence`, an I-01 `needs-human` follows the count-3
  line, no I-01 `dispatched` follows it, and ` | RUN | end | reason=all-parked` follows it. NOT-REACHED under the
  same condition as O1-rounds. FAIL if any I-01 park exists without any I-01 `needs-human`, and FAIL otherwise.
- **O1-nodeny** PASS iff LOG has zero `dispatch-denied` lines and zero `cap-tripped` lines, else FAIL.
- **O1-report** PASS iff there is at least one I-01 `report` event and every one has at most 15 detail lines and a
  first detail line starting `STATUS:`, else FAIL.

**RESULT.** **PASS** iff O1-new, O1-check, O1-version, O1-rounds, O1-park, O1-nodeny and O1-report all PASS.
**ADJUST** iff no check FAILs and at least one is NOT-REACHED, i.e. the seed converged before a third
`gaps_found`, or the driver self-parked `needs-human` before count 3 with no deny (the `r261-run2b`
self-limiting pattern). **FAIL** on any `dispatch-denied` (the prompt tried a third closure and the launcher
backstop caught it), a park without `needs-human`, a missing or wrong version, and on any other check that does
not hold.

## Reruns, retries and the Haiku miss rule

- **o1:** one paid Opus attempt plus at most one retry, allowed only after an ADJUST or an infrastructure fault;
  the retry's evidence is kept in `acceptance/o1/retry/` and `acceptance/o1/verdict.txt` then records the first
  attempt's result and follows the retry's. A FAIL is reported, not retried. **Two misses record ADJUST with the
  evidence. Never a third attempt** (D-25).
- **Haiku cases (h1-h4):** at most one rerun (`RERUN=1`), allowed only after an infrastructure fault; a logic
  FAIL is a model-capability finding (RESEARCH A7), reported with its evidence and NOT rerun. The harness marks
  an infrastructure fault as `infra_fault=yes` in `observed.txt` (no launch of the attempt produced a `result`
  line, so claude never completed a launch) and refuses a rerun after a PASS or a FAIL without it.

## Carried-risk coverage (26.1 VALID-02, D-23)

The carried risk asks that the launcher's gap cap be "tested together with the Phase 28 launcher cap under the
real driver, including a run with the prompt rule disabled so the launcher backstop actually trips".

- **Rules-on half = o1:** the real Opus driver, the embedded prompt with its gap rules, launcher cap L = 2.
- **Prompt-rule-disabled half = the Phase 28 d33 PASS, reused, not re-bought** [inferred - audit]: 28-09's d33
  showed an organic `dispatch-denied` under a real Opus driver at cap 0 with the rules removed, and the scope
  kill with no surviving process (the second `gaps_found` injected by design, as 28-09 states). d33 used
  `MASTER-PROMPT.run2.md` (v0.4 run2), not the embedded `driver-prompt/1.0.0`. **h4 binds that backstop to the
  embedded prompt text**: the same launcher integration with the gap rules stripped from the embedded asset
  (the derivation test proves the fixture is exactly the asset with those rules removed), observed with a
  cheap Haiku driver. The L+2 kill under the stripped embedded prompt is not re-run: the kill is
  prompt-independent launcher code already evidenced live by l4, l7 and d33.

## Outcome table (fixed before the results)

| Outcome | RESULT |
|---|---|
| h1: H1-new, H1-check, H1-1..H1-5 all PASS | `RESULT PASS` |
| h2: H2-new, H2-check, H2-1..H2-5 all PASS | `RESULT PASS` |
| h3: H3-new, H3-check, H3-seed, H3-A, H3-B all PASS | `RESULT PASS` |
| h4: H4-new, H4-check, H4-1..H4-4 all PASS | `RESULT PASS` |
| o1: every check PASS | `RESULT PASS` (closes SC3 and DRIVE-01; with the d33 reuse and h4, the carried risk) |
| o1: no check FAILs, the seed converged or the driver self-parked before count 3, no deny | `RESULT ADJUST`; one retry; two ADJUSTs are recorded as ADJUST |
| any case: a listed check FAILs | `RESULT FAIL`, reported with evidence, not rerun on logic grounds |
| any case: an infrastructure fault (network, a refused login, no `result` line at all) | the single rerun, and the SUMMARY says so |

## Evidence rules

Derived files only: `LOG.md`, `state.json`, the DIRECTION frontmatter (`frontmatter.md`), `check.txt`,
`cost.txt`, `observed.txt`, `run-env.txt`, `launcher-out.txt`, `stop-guard.log` and `drift.log` where they exist,
and the verdict. Never a stream `*.jsonl`, a transcript, the per-run `claude-config` or an environment dump. The
evidence directory must pass `gsd-meta-manager envelope scan` and a secret-pattern grep before it is committed, or
its copy is deleted (exit 6). `scripts/prompt-acceptance.sh check <case>` re-derives the verdict from those files
alone and exits 0 only on `RESULT PASS` (o1: or `RESULT ADJUST`).
