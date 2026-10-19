# 28-09 checkpoint: cost consent required before any paid launch

**Type:** human-action (cost consent). **Gate:** blocking-human.
**Plan:** 28-09 (`28-09-PLAN.md`). **There is deliberately no `28-09-SUMMARY.md`:** the plan is not complete, and
its existence would mark it done. ROADMAP/REQUIREMENTS/STATE were not advanced.
`plan_head_before: b7b06c655bbe88b67d75c62a63d384265dfcbd08` (HEAD when the non-spending part began).

No paid launch has been made. Zero dollars are recorded in `acceptance/campaign.txt` (header line only). No call
to the real `claude` binary other than `--version` was made, and no login or `claude setup-token` ran.

## Done (all non-spending)

| What | Commit |
|---|---|
| `scripts/launcher-acceptance.sh`: `selftest`, `run <case>` and read-only `check <case>` for l1 l2 l3 l4 l5 l7 l8 d33; mode table; campaign ledger (`ceiling_ok`, exit 8); scratch-root refusals (exit 2); prereg gate (exit 9); evidence copy + `envelope scan` + secret grep (exit 6) | `52970fb` |
| D-33 rubric `28-D33-RUBRIC.md`, committed ALONE | `7fe051a` (`docs(28-09): pre-register the D-33 rubric`) |
| `acceptance/d33/prereg.txt` (`PREREG_SHA=7fe051a4f199de3dcc60d0a2c47406b377d65556`) and `acceptance/campaign.txt` (header only) | `9f1d0c8` |
| `tests/launcher_acceptance.rs` (7 CI-safe tests) and the injection-mtime fix (last counted write + 46 s, as the rubric says) | `70130e8` |

Verification of that work (nothing spent):

- `bash scripts/launcher-acceptance.sh selftest` prints `launcher-acceptance selftest OK`. It covers the mode table,
  the ceiling arithmetic (including a third $9 launch refused), the scratch-root refusals, the prereg gate (missing,
  edited, later-edited, foreign-sha and not-alone refusals) and every verdict rule on a PASS fixture plus mutated
  FAIL fixtures. Run under trap `claude`/`systemctl`/`systemd-run`/`bwrap`/`curl` shims it touched none of them.
- Pre-spend refusals exercised: `run d33` without a prereg exits 9; `CAMPAIGN_CEILING_USD=25` exits 2; a ledger at
  $19.9 refuses a $0.20 launch with 8; `/tmp` scratch exits 2; existing evidence exits 2 without `RERUN=1`.
- Full suite `cargo test --no-fail-fast`: 3521 passed, 1 failed, 22 ignored. The one failure is the expected local
  git-version witness (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`).
  One earlier run also failed `driverctl::stop_guard::tests::a_falsy_agent_id_does_not_make_a_stop_a_subagent_stop`
  with `PermissionDenied` creating a temp dir; it passes alone and on the rerun, is in code this plan did not touch,
  and is a flake logged here as a deferred item. `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Harness plumbing was rehearsed end to end on this host with a FAKE `claude` (real bwrap and systemd-run, real
  `driver launch`, real store; the real claude was not on PATH): `l2`, `l4`, `l5` and `d33` PASS their verdict
  rules (d33 exercised the injection at the organic deny: `source=injected`, kill observed, no survivor), and `l1`, `l3`, `l7`, `l8`
  ran their plumbing and FAILed as expected for a driver that does nothing. Rehearsal evidence lived under
  `/tmp` and `~/.local/state` and was deleted; it is not evidence for the phase (the script marks it `REHEARSAL=1`
  and refuses a rehearsal that points at the committed evidence tree).

## What remains (needs the operator's go)

Each `run` rebuilds the release binary, launches the REAL `claude`, and writes `acceptance/<case>/`. Run them on
this host, OUTSIDE any tool sandbox, with a current Claude login (`gsd-meta-manager driver auth status` must show a
stored token, or a host token with at least `cap-minutes + 15` minutes left; d33 needs 105). Commit after each case.

Estimated spend is the per-launch `--max-budget-usd` ceiling (the mode table); measured Haiku cost in the spikes was
cents (L1-L5 under $0.05 in all, L7 $0.008). A launch that is killed leaves no `result` line, and the ledger then
charges the FULL budget, so `campaign.txt` will read conservatively high.

| Order | Command (from the repo root) | Model | Per-launch budget | Notes |
|---|---|---|---|---|
| 1 (Task 1 tracer) | `bash scripts/launcher-acceptance.sh run l5` | Haiku | $0.15 (two launches: the SIGKILLed one is charged $0.15, the relaunch bills its `result`) | about 3 min |
| 2 | `bash scripts/launcher-acceptance.sh run l1` | Haiku | $0.20 | dispatch cap 3 |
| 3 | `bash scripts/launcher-acceptance.sh run l2` | Haiku | $0.20 | wall clock 1 min; killed, charged $0.20 |
| 4 | `bash scripts/launcher-acceptance.sh run l3` | Haiku | $0.20 | host gap writer, about 4 min; scope killed at count 4, charged $0.20 |
| 5 | `bash scripts/launcher-acceptance.sh run l4` | Haiku | $0.20 | host gap writer, about 4 min; killed, charged $0.20 |
| 6 | `bash scripts/launcher-acceptance.sh run l7` | Haiku | $0.40 | drift watcher, about 4 min; killed, charged $0.40 |
| 7 | `bash scripts/launcher-acceptance.sh run l8` | Haiku | $0.40 | Stop guard case |

Haiku worst case on the ledger: about $1.90 (precondition for d33: `campaign.txt` sums to less than $2).

| Order | Command | Model | Per-launch budget |
|---|---|---|---|
| 8 (Task 3) | `bash scripts/launcher-acceptance.sh run d33` | Opus | $9, up to 90 min wall clock |
| 8b, only if the first d33 RESULT is not PASS | `RERUN=1 bash scripts/launcher-acceptance.sh run d33` | Opus | $9, exactly once, evidence in `acceptance/d33/retry/`; never a third |

D-33 worst case $18 on the ledger (a killed Opus launch is charged its full $9), so the phase total stays at most
about $19.9 under the $20 ceiling; the harness refuses (exit 8) any launch that would pass it. `CAMPAIGN_CEILING_USD`
may only lower the 20.

After each case: `bash scripts/launcher-acceptance.sh check <case>` must print `RESULT PASS` (d33: `RESULT PASS` or
`RESULT ADJUST`), then
`git add .planning/phases/28-sandboxed-driver-launcher/acceptance/<case> .planning/phases/28-sandboxed-driver-launcher/acceptance/campaign.txt`
and commit it. Retry rules: a Haiku case whose first run FAILs on an infrastructure fault (network, a refused login)
may be re-run once with `RERUN=1`, and the SUMMARY says so; a logic FAIL is reported, not re-run.

Then, in plan order, a continuation agent does the rest of 28-09:

1. the Task 1/2/3 verify commands (`check` for l1 l2 l3 l4 l5 l7 l8 d33, `selftest`);
2. the phase gate: `rtk proxy cargo test --no-fail-fast -- --skip the_config_section_constants_record_the_git_version_they_were_derived_against`
   and `rtk proxy cargo clippy --all-targets -- -D warnings`;
3. `28-09-SUMMARY.md` with every verdict line, per-launch and campaign costs, the D-33 organic/injected split and the
   carried-risk disposition (ADJUST hands it to Phase 29 SC3); self-check; then the STATE/ROADMAP/REQUIREMENTS updates
   (`state.advance-plan`, `roadmap.update-plan-progress 28 28-09 complete`, requirements LNCH-03 LNCH-02 LNCH-04 DRIVE-01).

## Decisions taken without the operator `[inferred — audit]`

- Haiku per-launch budgets are $0.20 (l1-l4), $0.15 (l5) and $0.40 (l7, l8), so the Haiku worst case stays under the plan's $2.
- A killed launch has no `result` line and `cost_split.py` needs one, so the ledger charges such a launch its full `--max-budget-usd` (`source=budget-charged`).
- "Evidence exists" is `run-env.txt` or `verdict.txt` in `acceptance/<case>/` (d33's directory already holds `prereg.txt`); `run-env.txt` is written before any spend, so a half-run case also refuses a second paid launch.
- d33's retry is allowed whenever the first RESULT is not PASS (the rubric's FAIL-on-infrastructure sentence), never a third. After a retry the top-level `d33/verdict.txt` is a combined verdict (a `D33-attempt-1` line, then the retry's lines); `check d33` re-derives exactly that.
- l5: launcher A runs a copy of the L5 prompt that sleeps 120 s so the run is live; the relaunch uses the plain L5 prompt (it ends at once). Host-precondition failures exit 5; l3 and l4 use a dispatch cap of 10 and 10 minutes.
- The Rust store logs gap rounds with `source=driverctl-scan` (the spike said `launcher-scan`); the pass rules do not depend on it.
- At run end the launcher tries its host-side push; the scratch repos have no remote, so each launch's `launcher-out.txt` shows `push: refused (no_push_remote)` and the run status is unchanged.
- The spike's `state.json` is deleted from each scratch run dir before launch so the Rust store starts the run (plan decision); `launcher.lock`, the socket and the per-run `claude-config` live outside the evidence and are never copied.
