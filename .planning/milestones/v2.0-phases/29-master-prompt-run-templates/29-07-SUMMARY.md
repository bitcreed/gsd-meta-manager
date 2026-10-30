---
phase: 29-master-prompt-run-templates
plan: 07
subsystem: acceptance-harness
tags: [acceptance, harness, cost-gates, consent, prereg, rubric, rehearsal, d19, d23, d24, d25]

requires:
  - phase: 29-master-prompt-run-templates
    provides: "29-05 reconcile/pause proofs and the A1 first-tick gap count, 29-06 embedded-prompt launch, DIRECTION preflight and hidden --prompt-file, 29-04 driver new / driver check, the h4 stripped-prompt fixture"
  - phase: 28-sandboxed-driver-launcher
    provides: "scripts/launcher-acceptance.sh helper library, the d33 PASS reused for the carried risk, the spike-003 seed v2 kit"
provides:
  - "scripts/prompt-acceptance.sh selftest | run | check for cases h1 h2 h3 h4 o1, with its own $25 ledger and consent (exit 10), ceiling (8) and pre-registration (9) gates"
  - "tests/prompt_acceptance.rs: CI-safe refusal and evidence proofs, nine zero-spend rehearsals through the real driver new / check / launch, two #[ignore] live entry points"
  - "29-P29-RUBRIC.md pre-registered at acceb35 and acceptance/prereg.txt naming it"
affects: [29-08 paid runs (the only remaining step is the operator's cost-consent checkpoint)]

actuals:
  tokens: 32000
  tasks: 3
  commits: 4

plan_head_before: c8057b3234e58788753105dfd90cf18bab904078
plan_head_after: ab3d60afcee19a56acfc22e3f1db6b38626cd914
commits: 4

tech-stack:
  added: []
  patterns:
    - "a gate that cannot be reached by a test or a rehearsal: consent, ceiling and prereg are all evaluated before the first launch, and a rehearsal reports instead of blocking only when ACC_PATH_PREPEND fakes are in use and the evidence root is away from the committed tree"
    - "scripted rehearsal: a fake claude that runs a /bin/sh scenario calling driverctl the way the prompt does, with a bwrap fake that honours --chdir and a scenario that ends when the launcher is SIGKILLed (the fake scope has no die-with-parent)"
    - "verdict re-derived from evidence files alone, each rule shown to bite by a PASS fixture and mutations in the selftest"

key-files:
  created:
    - scripts/prompt-acceptance.sh
    - tests/prompt_acceptance.rs
    - .planning/phases/29-master-prompt-run-templates/29-P29-RUBRIC.md
    - .planning/phases/29-master-prompt-run-templates/acceptance/prereg.txt
  modified:
    - scripts/launcher-acceptance.sh

key-decisions:
  - "Gate order is consent (10), ceiling (8), prereg (9), not the plan's consent, prereg, ceiling [inferred - audit]: with prereg last, the CI-safe exit-8 proof needs no real pre-registration and does not depend on the rubric commit existing. Every gate still runs before the first launch."
  - "prereg.txt holds the bare 40-hex sha (as the plan's verify command requires); pa_prereg_check feeds it to the Phase 28 prereg_gate as PREREG_SHA=<sha> through a temp file, so the 28 gate logic is reused unchanged."
  - "h2 gets a second item I-02 (after I-01) [inferred - audit]: with only I-01, a finished chore ends the run all-done and the pause (honoured before the NEXT dispatch, D-18 [operator-confirmed 2026-10-19]) is never exercised. H2-4 therefore forbids a dispatch of any item after the pause save."
  - "Haiku rerun policy is mechanical: one RERUN, only after an infrastructure fault (observed.txt infra_fault=yes: no launch produced a result line) or a missing verdict; a PASS or a logic FAIL refuses the rerun. o1 retries only after ADJUST or a missing verdict, once, never a third attempt."
  - "The Haiku chores use worker-model haiku and the generous caps dispatches 6 / minutes 15 [inferred - audit]; the cases measure prompt behaviour, not capacity."

requirements-completed: [PRMT-01, PRMT-03, PRMT-04, DRIVE-03]

duration: 23min
completed: 2026-10-20
status: complete
---

# Phase 29 Plan 07: Acceptance Harness and Pre-registered Rubric Summary

**`scripts/prompt-acceptance.sh` drives the real `driver new`, `driver check` and `driver launch` for five cases (h1 crash, h2 pause, h3 OQ-R1, h4 stripped deny, o1 Opus) behind consent, ceiling and pre-registration gates; every case has been rehearsed end to end for free, h1 to h4 derive RESULT PASS against a scripted fake driver, and the rubric is committed alone at `acceb35`.**

## Performance

- **Duration:** about 23 min
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files:** 4 created, 1 modified

## Accomplishments

- **Tracer (D-25):** `scripts/launcher-acceptance.sh` now runs its dispatch only when executed (`if [[ "${BASH_SOURCE[0]}" == "$0" ]]`), so `prompt-acceptance.sh` sources its helpers; its selftest still prints `launcher-acceptance selftest OK`, and its header records that the l1-l8 / d33 DIRECTION shapes predate the Phase 29 launch preflight and would be refused on a re-run (Pitfall 12). The new harness has its own `PH`, `ACC` (honouring `ACCEPTANCE_DIR`), rubric, `MAX_CEILING=25`, ledger and usage.
- **Gates (T-29-23, T-29-25):** `run` exits 2 on existing evidence (unless `RERUN=1`), 10 unless `acceptance/consent.txt` holds exactly one `approved_ceiling_usd=<n>` (0 < n <= 25) and one `approved_cases=<list>` naming the case, 8 when the ledger plus this launch's `--max-budget-usd` would pass min(approved, `CAMPAIGN_CEILING_USD`, 25), 9 unless the rubric commit in `prereg.txt` is an ancestor of HEAD, was committed alone and is unchanged since, 7 when the h4 fixture differs from HEAD. A launch with no `result` line is charged its full budget as `killed-full-budget` (shown live in the scripted h1 rehearsal: `CAMPAIGN spent=0.4000` before launch 2). `CAMPAIGN_CEILING_USD` above 25 exits 2.
- **Cases (D-19, D-23, D-24):** all five create the run with `driver new` (id parsed from the `created run <id>` line), fill the template in place, require `driver check` rc 0 (`check.txt`, X-check evidence), and launch by run id alone (h4: the stripped fixture through the hidden `--prompt-file`, expected `custom:<sha8>`). h3 seeds its park host-side with a real `driverctl serve` plus `driverctl log` sequence at zero model cost. o1 runs the spike-003 seed v2 setup, the `allow_default_branch_commits` patch and the verifier-triggered drift watcher (stopped on every exit path by the shared cleanup trap) before a foreground Opus launch.
- **Verdicts:** `check <case>` re-derives `verdict.txt` from evidence files alone and exits 0 only on PASS (o1: or ADJUST). The selftest holds a PASS fixture and 4 to 15 mutations per case, covering each pre-registered rule in both directions, the ADJUST paths of o1 and the combined retry verdict.
- **Rubric (D-23, D-25):** `29-P29-RUBRIC.md` pre-registers setup, exact LOG patterns, PASS/ADJUST/FAIL, the $25 ledger and consent rule, retry rules and the carried-risk coverage (o1 = rules-on half; the 28-09 d33 PASS reused for the prompt-rule-disabled half, bound to the embedded text by h4 and the derived-variant test). Committed alone as `acceb35`; `prereg.txt` (a separate commit `ab3d60a`) holds only that sha.

## Zero-spend rehearsals (nothing under the committed acceptance tree)

Each ran `ACC_PATH_PREPEND=<fakes> ACCEPTANCE_DIR=<scratch> bash scripts/prompt-acceptance.sh run <case>` through the real binary (`driver new`, `driver check`, `driver launch`) against pass-through `systemd-run`, `bwrap` and `systemctl` shims and a fake `claude`, with a dummy 0600 token under a scratch `XDG_CONFIG_HOME`. Every console began `REHEARSAL: consent would refuse (exit 10)` and `REHEARSAL: prereg would refuse (exit 9)`, ended `REHEARSAL: the path ran through driver new, driver check and driver launch; this verdict (...) is not a measurement`, and exited 0. The test asserts the committed `acceptance/` listing is identical before and after (it did not exist until the prereg commit).

| Case | Fake | Fake claude hits | Derived verdict |
|------|------|------------------|-----------------|
| h1 | plain (prints a zero-cost result line) | 2 (launch 1, launch 2) | FAIL, not meaningful (no driver behaviour); `H1-new` PASS `20261020-acc-h1-...`, `H1-check` PASS |
| h1 | scripted (dispatch + hook, hang, SIGKILL, recover, finish) | 2 | **PASS**: H1-1..H1-5, crash-detected line 52 before launch 2 line 53, recovered line 55 before dispatched line 57 |
| h2 | scripted (wait for `control: pause`, report, denied hook, `end pause`; relaunch recovers) | 2 | **PASS**: H2-1..H2-5, `RUN | end | reason=pause` line 56, recovered line 60 before dispatched line 61 |
| h3 | scripted (A ends all-parked, B dispatches); park seeded by real `driverctl serve` | 2 | **PASS**: H3-seed, H3-A, H3-B |
| h4 | scripted (denied hook, park `launcher-cap`, end); fixture via `--prompt-file` | 1 | **PASS**: H4-1..H4-4, `custom:f2f99387` in the launch line and the frontmatter |
| o1 | plain; seed v2, watcher alive, no `--prompt-file` in run-env.txt | 1 | FAIL, not meaningful; `O1-version` PASS (`prompt=driver-prompt/1.0.0`), `O1-nodeny` PASS, `O1-new`/`O1-check` PASS |

Plain-fake h2, h3 and h4 runs also reached the fake claude (2, 2 and 1 hits) before they were replaced in the suite by their scripted forms. The scripted forms are an addition to the plan: they prove the harness's positive path (wait for a real dispatch, kill, relaunch, pause edit, budget edit) offline, which the plain fake cannot.

## Task Commits

1. **Task 1 (tracer):** `97e657e` feat - the harness skeleton, consent/prereg/ceiling gates, h1 and its free rehearsal
2. **Task 2:** `79a2adc` feat - h2, h3, h4 and o1 with verdict re-derivation, scripted rehearsals, `live_h2_pause`
3. **Task 3:** `acceb35` docs - pre-register the Phase 29 acceptance rubric (ALONE); `ab3d60a` docs - record the rubric commit sha in `acceptance/prereg.txt`

## Deviations from Plan

### Auto-fixed Issues

None. No bug in earlier-plan code surfaced.

### Design deviations (documented, no scope change)

- **Gate order** consent, ceiling, prereg instead of consent, prereg, ceiling (key-decisions). All gates still precede the first launch, which is what T-29-25 requires.
- **Scripted rehearsals** for h1 to h4 in addition to the plain fake (plan Task 1/2 asked only for a fake that records argv and prints a result line). The plain fake cannot take the harness past "wait for dispatched", so the scripted path is the only offline evidence that the kill, pause-edit, budget-edit, relaunch and verdict code works.
- **h2 second item I-02**, and a rubric that forbids a dispatch of any item after the pause save, rather than only I-01 (key-decisions).
- The rehearsal tests place their scratch tree under `~/.local/state/gsdmm-rh/` because the harness refuses a scratch root under `/tmp`; they skip with a message when that directory cannot be created or lies inside a git work tree, and when `flock` or `timeout` is absent.

**Total deviations:** 0 auto-fixed, 4 design notes. **Impact:** none on the acceptance questions; the pre-registered pass rules are the case table's.

## Phase gate (offline, before the cost checkpoint)

- `bash scripts/prompt-acceptance.sh selftest` and `bash scripts/launcher-acceptance.sh selftest`: OK.
- `cargo test --no-fail-fast -- --skip the_config_section_constants_record_the_git_version_they_were_derived_against`: exit 0; 3680 passed, 0 failed, 24 ignored (the two new `#[ignore]` live entries included; nothing else changed in the ignored set).
- `tests/prompt_acceptance.rs`: 18 passed, 2 ignored; `tests/launcher_acceptance.rs`: 7 passed.
- `cargo clippy --all-targets -- -D warnings`: clean; `cargo fmt --check`: clean.
- `test "$(git log --format=%H -1 -- 29-P29-RUBRIC.md)" = "$(cat acceptance/prereg.txt)"` succeeds (`acceb353a82dea32a56b17d943cad5073335ea94`); `git show --stat` of that commit lists exactly the rubric.
- The real pre-registration gate was exercised once with trap tools and a valid temporary consent: it passed and the run stopped at host preconditions (exit 5, only the exit-99 bwrap trap was hit). No `consent.txt` exists in `acceptance/`; no paid or live model run was launched.

## Invocation for plan 29-08

After the operator's consent is written to `.planning/phases/29-master-prompt-run-templates/acceptance/consent.txt` (`approved_ceiling_usd=<n <= 25>` and `approved_cases=h1 h2 h3 h4 o1`, written by 29-08 only), per case from the repo root:

```
bash scripts/prompt-acceptance.sh run  <h1|h2|h3|h4|o1>     # builds target/release, gates, spends, writes acceptance/<case>/
bash scripts/prompt-acceptance.sh check <case>              # read-only re-derivation, exit 0 only on PASS (o1: or ADJUST)
RERUN=1 bash scripts/prompt-acceptance.sh run <case>        # the single rerun / the single o1 retry (acceptance/o1/retry/)
```

`SCRATCH_ROOT` defaults to `~/.local/state/gsd-meta-manager/prompt-acceptance/<case>-<utc>` (outside /tmp). Host needs bwrap, user systemd `running|degraded`, claude >= 2.1.295 and a stored or fresh host token with at least CAP_M + 15 minutes. The h1 relaunch and h2/h3 detached launches are bounded by the per-launch `--max-budget-usd` and by `driver stop`.

## Known Stubs

None.

## Threat Flags

None new. T-29-23 (rubric committed alone, gate 9 on every run), T-29-24 (derived files only, `envelope scan` and secret grep per evidence dir, CI test that the committed tree holds no `*.jsonl`, `claude-config` or token) and T-29-25 (consent 10, ceiling 8, prereg 9 on every real `run`; rehearsals need fake tools and a non-committed evidence root; `#[ignore]` entries only call the gated harness) are mitigated and tested. T-29-SC: no package added.

## Notes for the verifier

- An unrelated orphan from an earlier session was seen and left alone: pid 1545256, `/bin/sh /tmp/claude-1000/t/.tmpsfRhvB/scenario.sh` (a `tests/launcher_support` fixture scenario, about 9 h old). No process of this plan survives its tests.
- The local `envelope::policy` git-version witness is skipped in the gate command above, as the plan specifies; it is the expected local-only failure.

## Self-Check: PASSED

Created files exist: `scripts/prompt-acceptance.sh`, `tests/prompt_acceptance.rs`, `29-P29-RUBRIC.md`, `acceptance/prereg.txt`. Commits `97e657e`, `79a2adc`, `acceb35`, `ab3d60a` are ancestors of HEAD.
