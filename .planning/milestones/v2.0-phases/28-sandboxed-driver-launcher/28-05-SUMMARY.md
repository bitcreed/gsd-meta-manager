---
phase: 28-sandboxed-driver-launcher
plan: 05
subsystem: launcher
tags: [launcher, caps, gate, rate-limit, auth, exit-classification, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-01 cap flags + auth read path (TokenSource, host_access_token); 28-02 run loop, Containment, user-stop path; 28-04 Containment::Group, posture"
  - phase: 27-driverctl-driver-frontmatter
    provides: "RunStore, set_gate, caps.gap_rounds.per_phase, classify_exit, outstanding_dispatch"
provides:
  - "launcher::gate::{mentions_phase, decide_dispatch, CapGate, Tripped, new_tripped, trip, tripped_cap}"
  - "launcher::outcome::{LaunchEnd, Verdict, StreamSummary, StreamAccumulator, decide, summarize_stream, is_auth_failure, MAX_AUTH_RELAUNCHES}"
  - "RunStore::{trip_cap, record_active_seconds, set_launcher_key, log_launcher_event}, TerminalStatus::Parked, store::EXTRA_ACTIVE_SECONDS"
  - "run.rs CapMonitor (active seconds, wall_clock_min and gap_rounds kill), RuntimeMaxSec backstop, pre-lock CapExhausted refusal, auth relaunch"
  - "LauncherError::CapExhausted (rc 1)"
  - "tests/launcher_caps.rs (16 end-to-end tests through the fake shims)"
affects: [28-06, 28-07, 28-08, 28-09, phase-29, phase-30]

plan_head_before: 461dd37e90e826fd7cf2cd01bc45cea2e7afb19b
plan_head_after: 9ddc42c0792b8ea0c332312af5a0e3ae8f04e9bc
commits: 3
actuals:
  tokens: 24200
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "One pure decision per launch end (outcome::decide), first match wins, applied by one function"
    - "A cap trip is ordered: shared marker first (the gate denies from that instant), cap-tripped committed second, containment stop last"
    - "Stream-json is read line by line into an accumulator that keeps only the last result and the quota events; lines are pre-filtered by substring before any JSON parse"
    - "Launcher-owned state.json keys go through set_launcher_key, which refuses every typed RunState field (a unit test pins the list to the serialized fields)"

key-files:
  created:
    - src/launcher/gate.rs
    - src/launcher/outcome.rs
    - tests/launcher_caps.rs
  modified:
    - src/launcher/run.rs
    - src/launcher/mod.rs
    - src/driverctl/store.rs

key-decisions:
  - "wall_clock_min is checked at the 2 s tick, after tick_once, not at the 100 ms child poll [inferred - audit]"
  - "cap-minutes 0, negative or NaN leaves no wall clock and is refused (CapExhausted) [inferred - audit]"
  - "An exhausted wall clock is refused before the lock by reading state.json directly; the same exhaustion discovered between two launches of one run is a cap trip, not an error [inferred - audit]"
  - "The host token is re-read BEFORE auth-relaunch is logged, so an unusable host token parks without a line for a relaunch that never happened [inferred - audit]"
  - "is_auth_failure requires an error result (is_error true or an error_* subtype) and looks at result and errors[] [inferred - audit]"
  - "A trip overrides a status the driver had already written, as the spike's trip does [inferred - audit]"
  - "mentions_phase is a line-for-line port, so 'phase 1.5' names phase 1 (the \\b after 1 matches before '.'); kept to preserve the measured behaviour [inferred - audit]"

requirements-completed: [LNCH-03, LNCH-05]

coverage:
  - id: D1
    description: "The dispatch cap allows dispatches 1..L and denies L+1 with 'LAUNCHER CAP: dispatch cap reached (L/L)'; limit 0 denies the first; the denial is logged as one dispatch-denied line and counted in denied_dispatches"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#dispatch_cap_denies_the_fourth_dispatch"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#limit_zero_denies_the_first_dispatch"
        status: pass
      - kind: unit
        ref: "src/launcher/gate.rs#the_dispatch_cap_allows_dispatch_l_and_denies_l_plus_1"
        status: pass
    human_judgment: false
  - id: D2
    description: "The gap cap denies a dispatch naming phase P once per_phase[P] >= L+1 and kills the whole scope at L+2 (cap-tripped cap=gap_rounds, status stopped, launcher-cap:gap_rounds); an unchanged VERIFICATION.md adds no round"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#gap_cap_denies_at_limit_plus_one_and_kills_at_limit_plus_two"
        status: pass
      - kind: unit
        ref: "src/launcher/run.rs#the_gap_cap_kills_at_limit_plus_two_and_not_before"
        status: pass
      - kind: unit
        ref: "src/launcher/gate.rs#the_gap_cap_allows_at_count_l_and_denies_at_l_plus_1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Active wall-clock seconds are summed across launches, persisted as active_seconds / caps.wall_clock_min.used, the monitor kills the scope past limit*60, each sandboxed launch carries RuntimeMaxSec=ceil(remaining)+300, and a run with no wall clock left is refused before anything is written"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#wall_clock_cap_kills_the_scope"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#runtime_max_sec_counts_the_active_seconds_the_run_already_used"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#exhausted_wall_clock_refuses_the_launch"
        status: pass
      - kind: unit
        ref: "src/launcher/run.rs#the_wall_clock_trips_one_second_past_the_limit_and_not_at_it"
        status: pass
    human_judgment: false
  - id: D4
    description: "Once cap-tripped is recorded every later gate request is denied with 'run stopped by launcher cap <cap>' even before the scope has exited; a cap trips at most once"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#a_tripped_run_denies_every_later_dispatch_before_the_scope_has_exited"
        status: pass
      - kind: unit
        ref: "src/launcher/run.rs#two_caps_crossing_on_one_tick_trip_exactly_one_and_the_wall_clock_wins"
        status: pass
    human_judgment: false
  - id: D5
    description: "A rejected rate_limit_event (or a terminal reason naming a rate limit) parks the run with its window and reset time and never relaunches; an allowed or allowed_warning event does not park"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#rate_limited_launch_parks_without_retry"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#a_terminal_reason_naming_a_rate_limit_parks_with_an_unknown_window"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#an_allowed_quota_event_does_not_park_a_crash"
        status: pass
      - kind: unit
        ref: "src/launcher/outcome.rs#a_rejection_followed_by_an_allowed_event_is_still_reported"
        status: pass
    human_judgment: false
  - id: D6
    description: "A revoked OAuth token is an auth failure, never a crash: the host path re-reads the host token and relaunches at most twice (auth-relaunch lines) then parks; the stored path parks naming 'driver auth setup'; a host token with under 5 minutes left parks"
    requirement: LNCH-05
    verification:
      - kind: e2e
        ref: "tests/launcher_caps.rs#revoked_host_token_relaunches_twice_then_parks"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#revoked_stored_token_parks_without_relaunch"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#a_host_token_about_to_expire_is_not_relaunched_with"
        status: pass
      - kind: unit
        ref: "src/launcher/outcome.rs#a_rejected_host_token_relaunches_at_most_twice"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every launch end flows through one decision table: user stop > tripped cap > driver terminal end > fresh-exit > auth > quota > stalled/crashed"
    requirement: LNCH-03
    verification:
      - kind: unit
        ref: "src/launcher/outcome.rs (decision table tests)"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#clean_exit_with_unreported_dispatch_is_stalled"
        status: pass
      - kind: e2e
        ref: "tests/launcher_caps.rs#a_fresh_exit_relaunches_and_a_user_stop_still_beats_it"
        status: pass
    human_judgment: false
  - id: D8
    description: "The caps enforce against a real bwrap/systemd scope and a real Claude driver (SC2's live L1-L5 and the carried real-Opus gap-cap case)"
    requirement: LNCH-03
    verification: []
    human_judgment: true
    rationale: "Everything here runs against fake systemd-run/bwrap/systemctl/claude shims; the live L1-L5 runs against a real scope are the 28-09 harness and the real-driver case is its carried risk (D-33)"

duration: 20min
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 05: Hard Caps and Launch-End Verdicts Summary

**The dispatch, gap-round and wall-clock caps are enforced from outside the sandbox (gate deny at the limit / L+1, scope kill at L+2 or past `limit*60` active seconds, `RuntimeMaxSec = ceil(remaining)+300` as backstop), and every launch end is decided once by a tested table: quota exhaustion parks with its window and reset time, a revoked host token relaunches twice before parking, a rejected stored token parks naming the fix, and a clean early exit reads `stalled`, not `crashed`.**

## Performance

- **Duration:** 20 min (20:03Z to 20:24Z)
- **Tasks:** 2 (a tracer and a `tdd="true"` task), 3 commits (tracer `feat`, RED `test`, GREEN `feat`)
- **Files modified:** 6 (3 created, 3 modified); +2523 / -101 lines

## Accomplishments

- `launcher::gate`: `mentions_phase` (spike regex, escaped phase key, first 4000 chars), the pure `decide_dispatch` (tripped, then gap rule, then dispatch cap) and `CapGate`, installed with `set_gate` before the first spawn. The `Tripped` marker is shared with the monitor.
- `run.rs` `CapMonitor`: scope-alive seconds summed with the run's earlier launches, persisted at most every 15 s and at launch end (`extra.active_seconds`, `caps.wall_clock_min.used`); one pure `decide_trip` (wall clock first, then gap rounds); a trip marks the cap, commits `cap-tripped`, and only then does the caller stop the containment, so the gate denies from the first instant and the record precedes the kill (T-28-15, T-28-17). Works for the scope and the degraded group alike (`Containment::stop`).
- `outcome::decide` plus `summarize_stream` / `StreamAccumulator` / `is_auth_failure`, applied by `finish_launch`. The per-launch end handling of 28-02/28-04 (`record_end`, the inline fresh-exit check, `last_result_ok`) is replaced by it.
- Store writers: `trip_cap`, `record_active_seconds`, `set_launcher_key` (refuses typed `RunState` fields), `log_launcher_event`, `TerminalStatus::Parked`.
- Prohibition honoured: nothing reads a count from the driver. Dispatches are `hook_dispatches` (store-incremented for allowed gate payloads), gap rounds are `caps.gap_rounds.per_phase` (the on-disk scan). `gate.rs` takes only `HookPayload`, `RunState`, `CapLimits`.

## The exact LOG lines the tests produced (28-09 pass rules reuse these)

```
RUN | dispatch-denied | reason=dispatch cap reached (3/3). Park the item with --rule launcher-cap and end the run.
RUN | dispatch-denied | reason=phase 1 has failed verification 3 times (gap-round cap 2); no further dispatch for phase 1. Park the item with --rule gap-rounds.
RUN | cap-tripped | cap=gap_rounds | used=4 | limit=2 | action=kill-scope
RUN | cap-tripped | cap=wall_clock_min | used=0.1 | limit=0.05 | action=kill-scope
RUN | auth-relaunch | attempt=1 | source=host
RUN | auth-relaunch | attempt=2 | source=host
RUN | launch-end | status=parked | reason=rate-limit: window=five_hour resets_at=<RFC3339, seconds, Z>
RUN | launch-end | status=parked | reason=auth: host token rejected after 2 relaunches
RUN | launch-end | status=parked | reason=auth: stored token rejected — rerun driver auth setup
```

What the driver's hook prints (stdout, one JSON line, `permissionDecisionReason`): `LAUNCHER CAP: dispatch cap reached (3/3). ...`, `LAUNCHER CAP: phase 1 has failed verification 3 times (gap-round cap 2); ...`, and after a trip `LAUNCHER CAP: run stopped by launcher cap wall_clock_min`. State after a trip: `status: stopped`, `status_reason: launcher-cap:<cap>`; the launcher prints `ended status=stopped reason=launcher-cap:<cap>`. `cap-tripped` is committed before the `launch-end` line. `state.json` also carries `active_seconds` (f64) and, after an auth relaunch, `auth_relaunches`.

## Task Commits

1. **Task 1 (tracer): cap gate, monitor kill, RuntimeMaxSec** - `61d4442` (feat)
2. **Task 2 (tdd): launch-end verdicts**
   - RED `8ef73c8` (test)
   - GREEN `9ddc42c` (feat)

**Plan metadata:** the docs commit carrying this file.

## TDD Gate Compliance

RED (`test(28-05)` `8ef73c8`) precedes GREEN (`feat(28-05)` `9ddc42c`) for Task 2.

- RED: the types of `launcher::outcome` existed but `summarize_stream`, `is_auth_failure` and `decide` were no-op stubs (`Allowed` / `false` / `Keep`). 13 of 18 unit tests failed on the planned assertions (for example `assertion failed: is_auth_failure(Some(&revoked_result()))`, `expected a park, got Keep`, `a result` missing from the stream summary), and 5 of the 16 e2e tests failed at their first planned assertion (`left: "crashed" right: "parked"` for the rate-limit run; no `auth-relaunch | attempt=1` line for the revoked-host run; the same status assertion for the stored-token, expiring-token and unknown-window runs). The tests that passed at RED characterise behaviour the plan keeps (tripped cap and driver terminal end are `Keep`; an `allowed_warning` does not park; a dying driver is `crashed`; a clean exit with a worker outstanding is `stalled`, which 28-02 already did). semanticAssessment: each failing test executed and failed on the planned assertion for the intended reason; none failed on a compile, load or fixture fault.
- `gsd_run check tdd-red-evidence` accepts TAP / JUnit / swift-testing / unittest / go output only and `cargo test` output is none of those, so the machine classifier was not run (as in 28-04); the assessment above is by hand. Task 1 is a tracer (not `tdd="true"`): one `feat` commit with its tests. `workflow.tdd_mode` is not set; no REFACTOR commit.

## Verification

- Full `cargo test --no-fail-fast`: **3406 passed, 1 failed, 22 ignored.** The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Plan verify set, all green: `launcher_caps` 16 (about 25 s; `a_tripped_run_denies...` waits out the fake `systemctl stop`), `launcher_e2e` 17, `launcher_stop` 5, `launcher_degraded` 16, `driverctl_store` 39, `spawn_seam_guard` 41, `async_blocking_guard` 9, `--lib launcher::gate` 13, `--lib launcher::outcome` 18, `--lib launcher::` 85.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `pub fn mentions_phase` in gate.rs 1; `set_gate` in run.rs 1; `pub fn trip_cap` in store.rs 1 (baseline 0); `CapExhausted` in run.rs 1; `pub fn decide` in outcome.rs 1; `Parked` in store.rs 5 (baseline 1); `auth-relaunch` in run.rs 2.
- **Mutation checks (not committed).** Killing the gap scope at `L+1` instead of `L+2` made the gap e2e fail; not installing the gate failed the three dispatch/denial tests; making the gate ignore the trip marker failed the post-trip test; not adopting the re-read host token made `revoked_host_token_relaunches_twice_then_parks` fail (`at-host-1` three times instead of 1, 2, 3).

## Files Created/Modified

- `src/launcher/gate.rs` - `mentions_phase`, `decide_dispatch`, `CapGate`, the `Tripped` marker, 13 unit tests
- `src/launcher/outcome.rs` - `decide`, `summarize_stream`, `StreamAccumulator`, `is_auth_failure`, 18 unit tests
- `src/launcher/run.rs` - `CapMonitor`, `decide_trip`, budget helpers, `finish_launch` / `auth_relaunch` / `record_terminal`, `summarize_stream_file` (replaces `last_result_ok`)
- `src/launcher/mod.rs` - `gate` and `outcome` modules, `LauncherError::CapExhausted`
- `src/driverctl/store.rs` - the four launcher writers, `TerminalStatus::Parked`, `EXTRA_ACTIVE_SECONDS`
- `tests/launcher_caps.rs` - 16 end-to-end tests

## Decisions Made

All `[inferred - audit]` unless the plan stated them; see `key-decisions`. In addition:

- `Containment::stop` is reused unchanged for a cap kill, so a degraded run is killed as a group (TERM, 12 s, KILL) and a sandboxed one by `systemctl --user stop`; RuntimeMaxSec is sandboxed only (D-29).
- `Exited` now carries the `StreamSummary` instead of `result_ok`; `classify_exit` receives `stream.result_ok()` (subtype `success`, as the spike).
- `log_launcher_event` regenerates the frontmatter (`Regen::Always`); `set_launcher_key` and `record_active_seconds` use the heartbeat schedule.

## Deviations from Plan

**1. [Rule 3 - Blocking] `StreamAccumulator::finish` tripped the spawn-seam guard**
- **Found during:** Task 2 (full-suite run)
- **Issue:** `tests/spawn_seam_guard.rs#every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper` forbids any `.finish(` under `src/`, and flagged `acc.finish(now)` (three lines).
- **Fix:** renamed the method `into_summary`.
- **Verification:** `spawn_seam_guard` 41 passed.
- **Committed in:** `9ddc42c`

**2. [Plan order] The host token is re-read before `auth-relaunch` is logged**
- The plan listed `end_launch`, the log line, `set_launcher_key`, then the re-read. With that order an unusable host token (under 5 minutes left) would leave an `auth-relaunch` line for a relaunch that never happened and two `launch-end` lines. The re-read now comes first; on success the order is `end_launch`, `auth-relaunch`, `set_launcher_key`. Behaviour on every tested path is as planned.

**3. [Rule 2 - Missing critical] Tests and guards the plan did not list**
- `a_tripped_run_denies_every_later_dispatch_before_the_scope_has_exited` (the concurrency truth, driven with a SIGTERM-proof driver), `runtime_max_sec_counts_the_active_seconds_the_run_already_used`, `a_host_token_about_to_expire_is_not_relaunched_with`, `a_terminal_reason_naming_a_rate_limit_parks_with_an_unknown_window`, `an_allowed_quota_event_does_not_park_a_crash`, `a_dying_driver_without_run_end_is_crashed`, `a_fresh_exit_relaunches_and_a_user_stop_still_beats_it`, and store tests that pin `RUN_STATE_FIELDS` to the serialized fields of `RunState`.
- `trip_between_launches`: a relaunch that finds no wall clock left records a proper `cap-tripped` instead of returning an error that would leave a `running` status with a free lock.

**Total deviations:** 1 auto-fixed (Rule 3), 1 ordering change, 1 test/guard addition. **Impact:** none on the planned behaviour.

## Issues Encountered

None beyond Deviation 1.

## Pending live

None required for this plan. The caps are proven against fake shims; the live L1-L5 runs against a real scope and the carried real-Opus gap-cap case (D-33) are 28-09. Nothing here needs bwrap, systemd or network.

## Limits and notes for the verifier

- The wall clock trips on the 2 s tick, so a run can overrun its limit by up to about one tick plus the stop time; `RuntimeMaxSec` is the hard backstop (limit + 300 s).
- `mentions_phase` keeps the spike's `\b`, so a prompt naming `phase 1.5` also names phase `1`. A gap-capped phase 1 would therefore also deny a dispatch for 1.5. Decimal phases are keyed separately in `per_phase` (`1.5`), so a capped 1.5 does not deny a dispatch for phase 1.
- An unreadable or missing launch stream file is an empty summary: no result, quota allowed, so such a launch is classified by exit code alone.
- A host `.credentials.json` whose token is rewritten by the host between a rejection and the re-read is the intended case; one that is not yet rewritten is re-used and is rejected again (counts as a relaunch, at most two).

## Handoffs

- 28-06: the stored-token park text names `driver auth setup`; keep `auth::host_access_token` / `TokenSource` read-path signatures as they are (the `coupling_justified` entry in the plan).
- 28-07: `TerminalStatus::Parked` exists; a push-step park (secret scan) can reuse `set_terminal(TerminalStatus::Parked, ..)`.
- 28-09: pass rules above; `cap-tripped` and `dispatch-denied` grammar is pinned by `tests/launcher_caps.rs`.

## Known Stubs

None.

## Threat Flags

None. The surfaces added are the plan's T-28-14 (counts only from hook payloads and the on-disk scan), T-28-15 (marker set before the store write and the kill; `onFailure: block` stays 28-01's) and T-28-17 (`cap-tripped` committed before the stop), each with a test.

## Self-Check: PASSED

Created files exist (`src/launcher/gate.rs`, `src/launcher/outcome.rs`, `tests/launcher_caps.rs`); commits `61d4442`, `8ef73c8`, `9ddc42c` are ancestors of HEAD; the plan verify commands exit 0.
