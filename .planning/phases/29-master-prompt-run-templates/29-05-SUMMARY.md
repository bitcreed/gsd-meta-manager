---
phase: 29-master-prompt-run-templates
plan: 05
subsystem: launcher-tests
tags: [launcher, reconcile, control-pause, sigkill, e2e, gap-count]

requires:
  - phase: 29-master-prompt-run-templates
    provides: "29-02 store/gate enforcement: reconcile first, already recovered, control=pause deny"
  - phase: 28-sandboxed-driver-launcher
    provides: "tests/launcher_support fake claude/bwrap/systemd shims, Fixture, wait_until"
provides:
  - "tests/launcher_reconcile.rs: deterministic SC4/PRMT-04 proof through the real `driver launch` (D-17, D-18, D-19)"
  - "A1 pinned: a gaps_found VERIFICATION.md present at launch is counted on the first scan tick"
  - "fix: a driver `end --reason pause` is kept by the launch-end verdict, no longer relabelled crashed"
affects: [29-07 harness H3/H4 (A1), 29-08 paid runs (mechanism already proven offline)]

actuals:
  tokens: 36000
  tasks: 3
  commits: 5

plan_head_before: 737335a681da20e27c6b041fd5b7f48711b482ab
plan_head_after: 331314d97f5118e99a6fa9ccb97f29ff0bea0135
commits: 5

tech-stack:
  added: []
  patterns:
    - "SIGKILL only pids a fixture's own pid files name, re-checked against /proc/<pid>/environ before signalling (T-29-19)"
    - "scenario shell scripts record each driverctl exit code in $FAKE_CLAUDE_DIR/rc_* and refusal stderr in *.err for the test to assert"

key-files:
  created:
    - tests/launcher_reconcile.rs
  modified:
    - tests/launcher_caps.rs
    - src/launcher/outcome.rs

key-decisions:
  - "D-18 pause semantics: control honoured before the next dispatch [operator-confirmed 2026-10-19]; the test doc comment carries the SC4-versus-PRMT-04 wording difference for the verifier."
  - "The relaunch scenarios are one function `relaunch_scenario(action)` rather than four `RELAUNCH_SCENARIO_<ACTION>` consts, and the paused relaunch reuses the resume scenario instead of a separate `PAUSED_RELAUNCH_SCENARIO` const [inferred - audit]: the four differ only in a two-line follow-up, so a shared generator avoids four drifting copies."
  - "Launch-end verdict: ended_by_driver with status paused is Keep [inferred - audit], since a driver pause is a decision the driver recorded, not a crash."

requirements-completed: [PRMT-04]

duration: 40min
completed: 2026-10-20
status: complete
---

# Phase 29 Plan 05: Reconcile and Pause Proofs Summary

**Through the real `driver launch` with the fake shims, a driver SIGKILLed mid-dispatch is reconciled by exactly one `recovered` per action (resume, done, reset, park, and a launcher-plus-driver crash) before any new dispatch, and a `control: pause` saved while a worker runs is honoured before the next dispatch.**

A1: a `status: gaps_found` VERIFICATION.md that exists BEFORE launch is counted on the first scan tick (`gap-observed | phase=1 | count=1` is in LOG.md before a driver waiting for it sends its first hook), so it is NOT baseline: plan 29-07's harness must pre-write the file only when it wants that round counted, and must not leave a stale `gaps_found` file in a project it wants counted clean.

## Accomplishments

- **Tracer (D-17, D-19):** `RECONCILE_BODY` (lint-clean, two items) and `CRASH_SCENARIO` start a launch, dispatch I-01 (gate allowed) and hang; the test SIGKILLs only the driver process group, the launcher records the crash, I-01 stays `running`. The relaunch scenario then proves `dispatched` exits 2 with `reconcile first: I-01`, one `recovered` exits 0, a second exits 2 with `already recovered in this launch: I-01`, and LOG.md holds exactly one `| I-01 | recovered |` line after `RUN | launch | n=2` and before launch 2's first `dispatched`.
- **Matrix (D-19):** the same property for `done` (verified + status done), `reset` (status pending), `park` (needs-human + status parked) and `resume` (re-dispatch accepted, item `running`), asserting the follow-up item state in `state.json`; plus `a_launcher_and_driver_crash_is_reconciled_on_relaunch` (launcher SIGKILLed with the driver, `RUN | crash-detected | previous_launch=1` precedes `launch | n=2`).
- **Pause (D-18, PRMT-04, SC4):** `control: pause` saved with `driver::direction::write_body` while I-01's scripted worker runs; the report is still recorded, the next driver-level hook answers `"permissionDecision":"deny"` with `LAUNCHER CAP: control=pause`, `dispatched` exits 2 naming `control=pause`, LOG has one `dispatch-denied`, one `RUN | end | reason=pause` and one `dispatched`, state is `paused`. `a_paused_run_reconciles_its_running_item_on_relaunch` (Pitfall 10) sets the body back to `run` and requires one `recovered` for the still-`running` I-01.
- **A1 probe:** `a_gaps_found_file_present_at_launch_is_counted_on_the_first_tick` in `tests/launcher_caps.rs`.

## Task Commits

1. Task 1 (tracer): `4728c8f` test - SIGKILL a fake driver mid-dispatch, require one recovered on relaunch
2. Task 2: `b8a4192` test - done/reset/park matrix and launcher-and-driver crash
3. Fix found by Task 3: `3815450` fix - keep a driver's pause end instead of relabelling the launch crashed
4. Task 3: `115b3a0` test - pause honoured before the next dispatch and paused relaunch reconcile; `331314d` test - A1 first-tick gap count

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A driver's `end --reason pause` was recorded as a crash**
- **Found during:** Task 3 (the pause e2e)
- **Issue:** `launcher::outcome::decide` kept a driver-ended run only when `status.is_terminal()`; `paused` is not terminal, so the verdict fell to `classify_exit` and the launch ended `status=crashed | reason=driver exited rc=0 without RUN end` right after the driver logged `RUN | end | reason=pause`. The state was then `crashed`, contradicting the driver's own record.
- **Fix:** `ended_by_driver && (is_terminal() || status == Paused)` returns `Verdict::Keep`; unit test `a_driver_pause_end_is_kept_not_relabelled_crashed`; module header item 3 updated.
- **Files modified:** `src/launcher/outcome.rs`
- **Commit:** `3815450`

### Process notes

- Plan artifacts named consts `RELAUNCH_SCENARIO_<ACTION>` and `PAUSED_RELAUNCH_SCENARIO`; implemented as one `relaunch_scenario(action)` generator (see key-decisions). The seven planned test names exist, with the `_done/_reset/_park` suffixes as in the plan.
- The stage commits for Tasks 1 and 2 contain only the code each task needs; the pause scenario and `write_body` import arrive with Task 3.

**Total deviations:** 1 auto-fixed (1 bug). **Impact:** the launcher now ends a paused run `paused`, which the TUI and the next relaunch read correctly; no plan scope change.

## Verification

- `cargo test --no-fail-fast --test launcher_reconcile --test launcher_caps --test launcher_e2e`: 7 + 17 + 19 passed, repeated 3x with identical results (no flakes).
- Full suite: 3644 passed, 1 failed, 22 ignored; the single failure is the expected local `envelope::policy` git-version witness.
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- Acceptance greps: `reconcile first` 1, kill 5, action words 13, `crash-detected` 3, `LAUNCHER CAP: control=pause` 1, `reason=pause` 3, A1 test name 1.
- No paid or live model run was launched (29-08 owns those).

## Known Stubs

None.

## Threat Flags

None. Threat T-29-19 mitigated: the tests signal only pids from the fixture's own pid files, re-checked against `/proc/<pid>/environ`, and every test waits for the launch to end; no real systemd unit is touched.

## Self-Check: PASSED

`tests/launcher_reconcile.rs` exists; commits `4728c8f`, `b8a4192`, `3815450`, `115b3a0`, `331314d` are ancestors of HEAD.
