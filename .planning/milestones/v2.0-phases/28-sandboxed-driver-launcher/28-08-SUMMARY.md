---
phase: 28-sandboxed-driver-launcher
plan: 08
subsystem: launcher
tags: [launcher, push, git-mirror, secret-scan, pr-cap, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-05 store writers (set_launcher_key, log_launcher_event, TerminalStatus::Parked) and the launch-end decision table; 28-06 driver_cli resolve/report shape; 28-02 process.rs spawn seam and run lock"
  - phase: 19-git-safety-envelope
    provides: "classify_push_ref / default_namespace / DEFAULT_NAMESPACE_ROOT, scan_with_external, record_and_check (SAFE-01/03/06)"
provides:
  - "launcher::push::{prepare_mirror, push_run, push_finished_run, PushContext, PushOutcome, dest_ref, github_repo, check_destination, policy_for, redact_urls, MIRROR_DIR}"
  - "RunStore::{record_push, record_pr, park_after_push}, PushRecord, EXTRA_PUSH"
  - "process::run_bounded_with_stderr and bounded_output::stdout_within_with_stderr"
  - "Preferences::driver_open_pr, LaunchOptions.{open_pr, pr_policy}, DriverAction::Push (driver push <project> <run-id>)"
  - "state.json keys launch_branch, launch_start_sha, push {at, dest, sha, outcome, pr}"
  - "tests/launcher_push.rs (10 end-to-end tests against local bare remotes and a fake gh)"
affects: [28-09, phase-29, phase-30, phase-31]

plan_head_before: feba389e0f9c980a2f37f551c0981e24778be673
plan_head_after: d4ff3cba2cec43728582bbbe356fd98382e89dfc
commits: 3
actuals:
  tokens: 21660
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Push from a launcher-owned bare mirror: the URL is pinned (gsdmm.pushUrl) when the mirror is first created at a launch, the tip is fetched from the checkout into a private ref (refs/gsdmm/<run-id>), and only the mirror pushes"
    - "Every refusal precedes the next step: status, recorded branch, pinned remote, ref names, namespace, fetch, nothing-new/already-sent, tree scan, push, record, PR"
    - "A fresh push record replaces the previous one, so a PR is attempted per pushed tip and the SAFE-06 ledger (not the record) is what says no to the second"
    - "git/gh error lines are URL-redacted before they are printed or logged"

key-files:
  created:
    - src/launcher/push.rs
    - tests/launcher_push.rs
  modified:
    - src/launcher/run.rs
    - src/launcher/process.rs
    - src/launcher/mod.rs
    - src/bounded_output.rs
    - src/driverctl/store.rs
    - src/driver_cli.rs
    - src/cli.rs
    - src/config.rs

key-decisions:
  - "The first launch is told by launch_start_sha being present, not launch_branch: a detached HEAD records no branch, and a later launch must not record whatever the driver left checked out [inferred - audit]"
  - "LaunchOptions carries pr_policy (an EnvelopePolicy) beside open_pr, so the launcher's end-of-run push uses the project's configured PR caps; the plan listed only open_pr [inferred - audit]"
  - "An existing mirror keeps its pinned URL even when it has none (a project without origin at its first launch stays without a push remote); to change remotes delete the mirror [inferred - audit, as the plan]"
  - "Mirror/pin failures at launch start are reported and do not refuse the launch; the push then refuses (no_push_remote / no_recorded_branch) instead of guessing [inferred - audit]"
  - "A pinned URL must not be option-shaped, the ext:: transport, or multi-line; every launcher git runs with GIT_ALLOW_PROTOCOL=file:git:http:https:ssh, GIT_TERMINAL_PROMPT=0 and the repository-selecting GIT_* variables removed [inferred - audit]"
  - "Error lines from git and gh pass through redact_urls (user:token@ is dropped) before any print or LOG line [inferred - audit]"
  - "A PR is attempted after every Pushed outcome when open_pr is on (the new push record carries no pr), so the second pushed tip of a run reaches the ledger and the per-run cap refuses it with pr-refused; an unchanged tip (AlreadyPushed) never retries a PR [inferred - audit]"
  - "A ledger that cannot be used refuses the PR (pr-refused reason=ledger_unavailable), fail closed [inferred - audit]"
  - "Outcomes other than NothingToPush and AlreadyPushed are printed to the launcher's stderr at run end (launcher.log when detached); a refused or failed push never changes how the launch ended [inferred - audit]"

patterns-established:
  - "Launcher-only store writers for a new record (record_push, record_pr, park_after_push) keep their LOG grammar next to the writer; the caller adds refusal lines with log_launcher_event"

requirements-completed: [LNCH-06]

coverage:
  - id: D1
    description: "A finished run's recorded branch tip is sent from a launcher-owned mirror to refs/heads/gsd-auto/<alias>/<run-id> (the project's tip, nothing else on the remote), recorded once under push in state.json, with RUN | push logged; a second push of an unchanged tip writes nothing"
    requirement: LNCH-06
    verification:
      - kind: e2e
        ref: "tests/launcher_push.rs#push_sends_the_recorded_branch_tip_into_the_namespace"
        status: pass
      - kind: e2e
        ref: "tests/launcher_push.rs#launcher_pushes_once_at_run_end"
        status: pass
      - kind: e2e
        ref: "tests/launcher_push.rs#a_run_that_added_no_commits_pushes_nothing"
        status: pass
    human_judgment: false
  - id: D2
    description: "The push goes to the URL pinned when the mirror was created, never to a checkout config the driver rewrote; the mirror has hooks disabled and keeps its pin when the checkout changes"
    requirement: LNCH-06
    verification:
      - kind: e2e
        ref: "tests/launcher_push.rs#push_uses_the_pinned_remote_not_the_rewritten_checkout_config"
        status: pass
      - kind: unit
        ref: "src/launcher/push.rs#an_existing_mirror_keeps_its_pinned_url_when_the_checkout_is_rewritten"
        status: pass
      - kind: unit
        ref: "src/launcher/push.rs#a_new_mirror_pins_the_checkouts_origin_and_has_no_hooks"
        status: pass
    human_judgment: false
  - id: D3
    description: "A tip whose tree holds a credential pushes nothing, parks the run with push-refused:secret_detected, logs RUN | push-refused | reason=secret_detected, and the key text appears in no output, LOG or state"
    requirement: LNCH-06
    verification:
      - kind: e2e
        ref: "tests/launcher_push.rs#secret_in_the_tip_parks_and_pushes_nothing"
        status: pass
    human_judgment: false
  - id: D4
    description: "Only the run's own branch is a destination: refs/heads/main, a bare gsd-auto/<alias> prefix and another alias's prefix are refused by the reused classify_push_ref; the push has no --force and no + refspec"
    requirement: LNCH-06
    verification:
      - kind: unit
        ref: "src/launcher/push.rs#only_the_runs_own_branch_in_its_own_alias_namespace_is_allowed"
        status: pass
      - kind: unit
        ref: "src/launcher/push.rs#the_destination_is_the_run_branch_in_the_reserved_namespace"
        status: pass
    human_judgment: false
  - id: D5
    description: "driver push refuses a run whose lock is held (live) and a crashed run (status running, no final status); a launch ending crashed pushes nothing"
    requirement: LNCH-06
    verification:
      - kind: e2e
        ref: "tests/launcher_push.rs#push_refuses_a_live_run"
        status: pass
      - kind: e2e
        ref: "tests/launcher_push.rs#push_refuses_a_crashed_run"
        status: pass
    human_judgment: false
  - id: D6
    description: "No gh process starts unless preferences.driver.open_pr is a real true; with it on, exactly one gh pr create runs after a push (repo, head, base, title), the URL is recorded as push.pr, the second pushed tip is refused by the SAFE-06 per-run cap with RUN | pr-refused | reason=pr_cap_exceeded, and a non-GitHub remote logs pr-skipped"
    requirement: LNCH-06
    verification:
      - kind: e2e
        ref: "tests/launcher_push.rs#no_pr_without_open_pr"
        status: pass
      - kind: e2e
        ref: "tests/launcher_push.rs#open_pr_opens_one_pr_then_the_cap_refuses"
        status: pass
      - kind: e2e
        ref: "tests/launcher_push.rs#non_github_remote_skips_the_pr"
        status: pass
      - kind: unit
        ref: "src/config.rs#open_pr_is_off_unless_the_user_turned_it_on"
        status: pass
      - kind: unit
        ref: "src/config.rs#open_pr_reads_only_a_real_boolean"
        status: pass
    human_judgment: false
  - id: D7
    description: "The push works against a real network remote (GitHub over ssh/https with the user's credentials) and the real gh creates a pull request"
    requirement: LNCH-06
    verification: []
    human_judgment: true
    rationale: "Every test pushes to a local bare repository over file:// and uses a gh shim; a real remote needs the user's credentials and a real repository, and nothing here may push to the project's own public origin. The git argv and the gh argv are the ones shown below"

duration: 21min
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 08: Host-side Push from a Launcher-owned Mirror Summary

**A driven run's commits now leave the machine only through the launcher: the recorded branch tip is fetched from the checkout into a bare mirror gsd-mm owns (hooks off, push URL pinned before any sandbox existed), the tip's whole tree passes the SAFE-03 scan, and one fixed refspec sends it to `refs/heads/gsd-auto/<alias>/<run-id>` once at run end (or by `driver push`), with an opt-in `gh pr create` that the SAFE-06 ledger caps at one per run.**

## Performance

- **Duration:** 21 min (about 21:08Z to 21:29Z)
- **Tasks:** 2 (a tracer and a `tdd="true"` task), 3 commits (tracer `feat`, RED `test`, GREEN `feat`)
- **Files modified:** 10 (2 created, 8 modified); +2035 / -9 lines

## Accomplishments

- `launcher::push`: `prepare_mirror` (bare mirror at `<driver-root>/<hash8(project)>/mirror.git`, `core.hooksPath=/dev/null`, `gsdmm.pushUrl` pinned from the checkout's `remote.origin.url` only when the mirror is created), `push_run` (the ordered chain of refusals below), `push_finished_run` (what `driver push` calls: refuses a live run, takes the run lock, opens the store).
- Run side: `record_push_basis` pins the mirror and records `launch_branch` / `launch_start_sha` at the first launch before `begin_launch` and the sandbox; `push_at_run_end` pushes once after `decide` wrote a final `done`, `parked` or `stopped`, while the lock is still held. Nothing pushes after `crashed`, `stalled` or a relaunch verdict, never between launches.
- `driver push <alias|path> <run-id>` (rc 0 for pushed / already pushed / nothing to push, 1 for a refusal or failure).
- Store: `record_push` (`state.json` `push`, `RUN | push | dest | sha | outcome`), `record_pr` (`push.pr`, `RUN | pr | url`), `park_after_push` (parks even after a driver `RUN end`, which `set_terminal` would keep).
- PR step (D-26): off unless `preferences.driver.open_pr` is a JSON `true`; GitHub remotes only; `record_and_check` runs before `gh` starts and a refusal or an unusable ledger logs `pr-refused`.

## The exact commands

The push, as `push_run` builds it (the tip is a full 40-hex commit id; `<mirror>` is `<driver-root>/<hash8>/mirror.git`; `<url>` is the pinned `gsdmm.pushUrl`; env `GIT_TERMINAL_PROMPT=0`, `GIT_ALLOW_PROTOCOL=file:git:http:https:ssh`, repository-selecting `GIT_*` variables removed):

```
git --git-dir <mirror> fetch --quiet --no-tags --no-recurse-submodules <project> +refs/heads/<branch>:refs/gsdmm/<run-id>
git --git-dir <mirror> rev-parse --verify --quiet refs/gsdmm/<run-id>^{commit}
git --git-dir <mirror> worktree add --detach --force --quiet <driver-root>/<hash8>/scan-<run-id> <tip>
git --git-dir <mirror> push --quiet --no-recurse-submodules <url> <tip>:refs/heads/gsd-auto/<alias>/<run-id>
```

The `+` is on the fetch into the private ref only; the push refspec has none and there is no `--force`.

The `gh` call the shim captured (`open_pr_opens_one_pr_then_the_cap_refuses`, one line per call, run from `<driver-root>`, env `GH_PROMPT_DISABLED=1`, `NO_COLOR=1`):

```
gh pr create --repo o/r --head gsd-auto/p28/r28 --base work --title "gsd-mm driver run r28" --body "Pushed by gsd-meta-manager driver run r28 (p28)."
```

LOG lines the tests produced:

```
RUN | push | dest=refs/heads/gsd-auto/p28/r28 | sha=<12 hex> | outcome=pushed
RUN | pr | url=https://github.com/o/r/pull/1
RUN | pr-refused | reason=pr_cap_exceeded
RUN | pr-skipped | reason=not-github
RUN | push-refused | reason=secret_detected | findings=<n>
```

State after a secret hit: `status: parked`, `status_reason: push-refused:secret_detected`. Refusal reasons returned by `push_run` (stable identifiers): `run_not_finished`, `no_recorded_branch`, `no_push_remote`, `run_live` (CLI only), `push_outside_namespace`, `secret_detected`.

## Task Commits

1. **Task 1 (tracer): push a finished run's recorded branch tip from the mirror** - `efb8712` (feat)
2. **Task 2 (tdd): run-end push, secret parking, opt-in PR under the cap**
   - RED `d7649dd` (test)
   - GREEN `d4ff3cb` (feat)

**Plan metadata:** the docs commits carrying this file and the state update.

## TDD Gate Compliance

RED (`test(28-08)` `d7649dd`) precedes GREEN (`feat(28-08)` `d4ff3cb`) for Task 2.

- RED: `Preferences::driver_open_pr` was a stub returning `false`; the integration tests drive the real binary and needed no stub. 6 of the 10 `launcher_push` tests failed and 2 of the 3 new `config::` tests failed, each on its planned assertion: `launcher_pushes_once_at_run_end` and `push_uses_the_pinned_remote_...` (`left: None right: Some(<tip>)`, nothing pushed at run end), `no_pr_without_open_pr` (same: "the work was pushed"), `secret_in_the_tip_parks_and_pushes_nothing` (`left: "done" right: "parked"`), `open_pr_opens_one_pr_then_the_cap_refuses` (`one pull request: []`, 0 gh calls), `non_github_remote_skips_the_pr` (no `pr-skipped` line), and the two config tests on `driver_open_pr()` being `false` for a real `true`. The tests that passed at RED characterise behaviour Task 1 already shipped (`push_refuses_a_live_run`, `push_refuses_a_crashed_run`, the tracer, a run with no commits, and `open_pr_reads_only_a_real_boolean`). semanticAssessment: each failing test executed and failed on the planned assertion for the intended reason; none failed on a compile, load or fixture fault.
- `gsd_run check tdd-red-evidence` accepts TAP / JUnit / swift-testing / unittest / go output only and `cargo test` output is none of those, so the machine classifier was not run (as in 28-04 and 28-05); the assessment is by hand. Task 1 is a tracer (one `feat` commit with its tests). `workflow.tdd_mode` is not set; no REFACTOR commit.

## Verification

- Full `cargo test --no-fail-fast`: **3514 passed, 1 failed, 22 ignored.** The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The known flaky `driverctl::serve::tests::wr05_a_live_socket_is_never_replaced` passed (not touched by this plan).
- Plan verify set, all green: `launcher_push` 10, `--lib launcher::push` 12, `--lib config::` 13, `launcher_e2e` 17, `launcher_caps` 16, `launcher_stop` 5, `envelope_pr_cap` 11, `spawn_seam_guard` 41, `async_blocking_guard` 9.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `pub fn push_run` in push.rs 1; `classify_push_ref` 3; `scan_with_external` 2; `pub fn record_push` in store.rs 1; `pub fn driver_open_pr` in config.rs 1 (baseline 0); `record_and_check` in push.rs 2; `push_run` in run.rs 2.
- **Mutation checks (not committed).** Making the scan always clean failed `secret_in_the_tip_parks_and_pushes_nothing` (1 failure); making the PR step ignore `open_pr` failed `no_pr_without_open_pr`; pushing to a remote name instead of the pinned URL failed six tests.

## Files Created/Modified

- `src/launcher/push.rs` - the push step, mirror, PR step, 12 unit tests
- `tests/launcher_push.rs` - 10 end-to-end tests (local bare remotes, `gh` shim)
- `src/launcher/run.rs` - `record_push_basis`, `push_at_run_end`, `LaunchOptions.{open_pr, pr_policy}`, `LaunchContext.driver_root`
- `src/launcher/process.rs` - `run_bounded_with_stderr` (stderr to a temp file, last line kept; git env scrub)
- `src/bounded_output.rs` - `stdout_within_with_stderr`
- `src/driverctl/store.rs` - `PushRecord`, `record_push`, `record_pr`, `park_after_push`, `EXTRA_PUSH`
- `src/driver_cli.rs`, `src/cli.rs` - `driver push`; `resolve_project` also returns the loaded registry
- `src/config.rs` - `Preferences::driver_open_pr` and 3 tests
- `src/launcher/mod.rs` - the `push` module and header fact 12

## Decisions Made

See `key-decisions` (all `[inferred - audit]` unless the plan stated them). The residual the plan does not close: `git fetch` from a hostile checkout runs `git upload-pack` in that checkout on the host. Per git's documentation `uploadpack.packObjectsHook` is ignored from repository config, and the fetch uses `--no-recurse-submodules`; no repo-local setting this build is aware of runs a command during a local fetch, but that is a statement about git, not something this plan tests.

## Deviations from Plan

**1. [Rule 3 - Blocking] `src/bounded_output.rs` extended**
- **Found during:** Task 1
- **Issue:** the plan's `run_bounded_with_stderr` needs the child's stderr, but `stdout_within` hard-wires `stderr(Stdio::null())` after the caller sets it, and bounded_output.rs was not in `files_modified`.
- **Fix:** `stdout_within_with_stderr(command, budget, stderr)`; `stdout_within` now calls it with `Stdio::null()` (behaviour unchanged). The stderr sink is a file, never a pipe.
- **Committed in:** `efb8712`

**2. [Rule 3 - Blocking] The spawn-seam guard flagged `Command::new` in push.rs unit tests**
- **Found during:** Task 1 (guard run)
- **Fix:** the unit tests run git through the module's own `git_in`, which goes through `process.rs`. push.rs stays off the allowlist, as the plan says.
- **Committed in:** `efb8712`

**3. [Rule 2 - Missing critical] Hardening the plan did not list**
- URL redaction of git/gh error lines, `plausible_url` / `plausible_branch` validation, `GIT_ALLOW_PROTOCOL`, removal of `GIT_DIR` and friends from launcher git calls, `git check-ref-format` on the destination and the private ref (run ids may contain `..`), and a fail-closed refusal when the ledger is unusable. Each has a unit test or is covered by the end-to-end tests.
- **Committed in:** `efb8712`, `d4ff3cb`

**4. [Plan addition] `RunStore::record_pr` and `LaunchOptions.pr_policy`**
- The plan listed `record_push` and `park_after_push`, and `open_pr` only. `record_pr` keeps the `push.pr` update and its LOG line in the store; `pr_policy` lets the launcher's own end-of-run push use the project's configured PR caps.

**5. [Plan addition] Tests not listed:** `a_run_that_added_no_commits_pushes_nothing`, the `cli` parse test for `driver push`, the mirror unit tests, `pr_url`, `redact_urls`, `plausible_*`.

**Total deviations:** 2 auto-fixed (Rule 3), 1 hardening (Rule 2), 2 additions. **Impact:** none on the planned behaviour.

## Issues Encountered

None beyond Deviation 2.

## Pending live

None required for this plan. Everything runs on this host against local bare repositories and fake `claude`/`gh`/`systemd-run`/`bwrap` shims; nothing needs bwrap, systemd or a network. A push to a real GitHub remote with a real `gh` is deliberately not run (D7, `human_judgment`).

## Limits and notes for the verifier

- A mirror that already exists is never re-pinned, so a project that had no `origin` at its first launch has no push remote until the user deletes `<driver-root>/<hash8>/mirror.git`. The refusal says so.
- The push chain re-fetches and re-scans on every `driver push` of a run whose tip moved; an unchanged tip returns before the scan.
- A run state directory named `mirror.git` (a legal run id) would share its path with the mirror. Not guarded; `prepare_mirror` only treats a directory with a `HEAD` file as a mirror.
- The PR step runs once per pushed tip. A re-push of a new tip after a PR exists therefore logs `pr-refused` (the per-run cap), and an `AlreadyPushed` outcome never retries a PR that failed the first time.
- Precision (LNCH-06, backstop): the PR cap reuses `envelope::ledger`'s second-precision RFC3339 stamp (`Utc::now().to_rfc3339_opts(Secs, true)`); push.rs does no time arithmetic of its own.

## Handoffs

- 28-09: the push LOG grammar above; `driver push` is a plain verb the harness can call; `state.json` `push` is the place to read the result.
- Phase 30: `push` and `launch_branch` / `launch_start_sha` are `extra` keys (not typed `RunState` fields).

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-28-20 (pinned URL, hooks off, `push_uses_the_pinned_remote_...`), T-28-21 (`secret_in_the_tip_parks_and_pushes_nothing`), T-28-22 (fixed refspec, `check_destination` unit tests), T-28-23 (`open_pr_opens_one_pr_then_the_cap_refuses`) and T-28-24 (`core.hooksPath=/dev/null`, asserted again on every launch) each have a test or an assertion.

## Self-Check: PASSED

Created files exist (`src/launcher/push.rs`, `tests/launcher_push.rs`); commits `efb8712`, `d7649dd`, `d4ff3cb` are ancestors of HEAD; the plan verify commands exit 0.
