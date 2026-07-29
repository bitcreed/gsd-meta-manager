---
phase: 16-run-journal-state-substrate
plan: 02
subsystem: infra
tags: [tokio, mpsc, backpressure, observability, executor]

# Dependency graph
requires:
  - phase: 15-transport-foundation
    provides: "the bounded executor event channel, EVENT_FORWARD_TIMEOUT, the `forward` funnel that counts dropped events, and the run loop's terminal path"
provides:
  - "ExecutionEvent::EventsDropped { count } — the dropped-event total, observable on the executor stream instead of only in the log"
  - "report_dropped: a bounded, best-effort, once-per-run emitter for that count, called in the terminal path ahead of Exited"
  - "An explicit no-op reducer arm in App::apply_exec_event, so a drop report cannot manufacture run liveness"
affects: [16-06 journal record mapping, 16-05 app.rs reducer extension, phase-18 driver surface]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Supervisor-side sends in the terminal path are wrapped in tokio::time::timeout at EVENT_FORWARD_TIMEOUT — the same bounded shape as the Exited send, for the same CR-01 reason"
    - "Diagnostic events get an explicit reducer arm rather than falling into the liveness-promoting catch-all"

key-files:
  created: []
  modified:
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/app.rs

key-decisions:
  - "A new stream variant rather than a widened ExecutionHandle/RunOutcome — D-37 protects those types, and the drop count is different in kind because Phase 15 never recorded it anywhere a consumer could see"
  - "No boxing for EventsDropped: RESEARCH §8 measured ExecutionEvent at 120 bytes against a 200-byte difference threshold, so an 8-byte variant is nowhere near it — recorded in the doc so nobody boxes reflexively"
  - "The report is emitted BEFORE the Exited forward, so a journal reading the stream in order sees the loss before the ending"
  - "The report is best-effort: if the consumer is still stalled at run end it is lost and only the tracing::warn! remains, because a lost diagnostic is strictly better than a parked run"
  - "Test channel locals were renamed off `events_tx` so the single-call-site grep in the acceptance criteria stays able to distinguish one call site from four"

patterns-established:
  - "Once-per-run terminal diagnostics: report at the end, emit nothing when there is nothing to report, so the absence of the event is itself the signal"
  - "A grep-based acceptance criterion constrains test-local naming; tests must not shadow the production identifier the criterion keys on"

requirements-completed: [OBS-01]

coverage:
  - id: D1
    description: "The dropped-event count is observable on the ExecutionEvent stream exactly once per lossy run, carrying the running total and nothing else"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#a_run_that_dropped_events_reports_the_count_once"
        status: pass
    human_judgment: false
  - id: D2
    description: "A lossless run emits no drop report at all, so the absence of the event is the signal"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#a_run_that_dropped_nothing_reports_nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "Reporting the count cannot park the run — the send goes through the same bounded hand-off every other supervisor-side send uses"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#reporting_a_drop_into_a_full_channel_returns_within_the_forward_bound"
        status: pass
    human_judgment: false
  - id: D4
    description: "An events-dropped report does not move a run's rendered state, because it is a diagnostic and not evidence of process liveness"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/app.rs#a_drop_report_does_not_move_a_runs_state"
        status: pass
    human_judgment: false
  - id: D5
    description: "No type D-37 protects (ExecutionHandle, GateOutcome, RunOutcome) gained or lost a field, and Phase 15's verified transport is unregressed"
    verification:
      - kind: other
        ref: "git diff -U0 src/executor/mod.rs | grep -c 'pub struct ExecutionHandle\\|pub enum RunOutcome' == 0; git diff --stat src/executor/gate.rs empty"
        status: pass
      - kind: integration
        ref: "rtk proxy cargo test --test executor_lifecycle (11 pass) && --test executor_transport (7 pass)"
        status: pass
    human_judgment: false

# Metrics
duration: 12 min
completed: 2026-07-29
status: complete
---

# Phase 16 Plan 02: The Dropped-Event Report Summary

**Phase 15's dropped-event count now reaches the `ExecutionEvent` stream as `EventsDropped { count }` — once per lossy run, bounded so it cannot park the run, and explicitly barred from moving a run's rendered state.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-29T14:34Z (approx.)
- **Completed:** 2026-07-29T14:46Z
- **Tasks:** 2 (both TDD — 4 commits)
- **Files modified:** 3

## Accomplishments

- `ExecutionEvent::EventsDropped { count: u64 }` exists, documented with its provenance (D-33, plan 15-08's handover) and with the measured `large_enum_variant` budget that settles the boxing question against re-litigation.
- `report_dropped` emits exactly one report per run and only when something was actually lost, wrapped in `tokio::time::timeout` at `EVENT_FORWARD_TIMEOUT` so a stalled consumer cannot hold the run loop open past its outcome send (CR-01).
- The single call site sits in the terminal path immediately ahead of the `Exited` forward, so a journal reading the stream in order sees the loss before the ending.
- `App::apply_exec_event` gained an explicit `EventsDropped` arm above the catch-all, so a diagnostic emitted after the process stopped producing can no longer promote an `Idle` alias to `Starting`.
- `ExecutionHandle`, `GateOutcome` and `RunOutcome` are byte-identical to before (D-37); `src/executor/gate.rs` is untouched.

## Task Commits

Each task was committed atomically, following the RED/GREEN cycle:

1. **Task 1 (RED): failing test for the dropped-event report** — `12fda61` (test)
2. **Task 1 (GREEN): report the count on the executor stream** — `f084415` (feat)
3. **Task 2 (RED): failing test for the drop report's reducer arm** — `3fa00be` (test)
4. **Task 2 (GREEN): explicit no-op reducer arm** — `f5469a8` (feat)

No REFACTOR commit was needed — both implementations are already at their minimal shape.

## Files Created/Modified

- `src/executor/mod.rs` — added the `EventsDropped { count }` variant to `ExecutionEvent`, with a doc recording why it exists, why it carries a count and nothing else, and why it is not boxed.
- `src/executor/claude.rs` — added `report_dropped` beside `forward` sharing its doc idiom; added its single call site in the terminal path ahead of the `Exited` send; updated the `dropped_events` comment, which no longer claims the log is the count's only destination; added three tests.
- `src/app.rs` — added the explicit `ExecutionEvent::EventsDropped { .. } => {}` arm above the catch-all, extended `apply_exec_event`'s doc in the file's own "what this deliberately does not do" style, and added `a_drop_report_does_not_move_a_runs_state`.

## Decisions Made

- **A new stream variant, not a widened type.** D-37 records that Phase 15 stored `session_id`, `capabilities`, `claude_code_version` and the derived `RunOutcome` specifically so Phase 16 could journal them without a signature change. The drop count is different in kind — Phase 15 never recorded it anywhere a consumer could read — so a new event is the honest mechanism. An `Arc<AtomicU64>` on the handle was rejected for the same reason D-20 bans handles in cloneable message types.
- **Emit before `Exited`, not after.** Stream order is the only ordering a journal consumer gets, and a loss report that arrives after the ending is a report about a run the reader has already closed out.
- **Best-effort, explicitly.** The helper's doc states that a still-stalled consumer loses the report and only the `tracing::warn!` in `forward` remains. That degradation is named rather than left to be rediscovered, because the alternative — an unbounded send — is a run that never ends.
- **The 5-second test is deliberate.** `reporting_a_drop_into_a_full_channel_returns_within_the_forward_bound` waits out a real `EVENT_FORWARD_TIMEOUT` against a pre-filled capacity-1 channel. `tokio::time::pause()` would make it instant but would assert against virtual time; the plan asked for a wall-clock bound because the property under test *is* a deadline, and a generous multiple (3×) keeps it from flaking on a loaded machine.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test-local channel names defeated a grep-based acceptance criterion**

- **Found during:** Task 1 (verification of the acceptance criteria after the GREEN commit)
- **Issue:** The criterion `grep -n 'report_dropped(&events_tx' src/executor/claude.rs` must report exactly one line — the run loop's single call site. My three new tests followed the module's existing `feed()` helper convention and named their channel locals `events_tx` / `events_rx`, so the grep matched four lines and the criterion could no longer distinguish one production call site from several test invocations. A future regression check running that grep would have been meaningless.
- **Fix:** Renamed the test locals to `report_tx` / `report_rx` and added a comment at the top of the new test block explaining that the naming is load-bearing for the call-site check. A first attempt at that comment quoted the grep pattern literally and re-broke the check by matching itself; the comment was reworded to describe the pattern instead of containing it.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** `grep -n 'report_dropped(&events_tx' src/executor/claude.rs` now returns exactly line 1232, which is above line 1243 (`ExecutionEvent::Exited(status)`), as the criterion requires. `grep -c 'report_dropped'` returns 7 (≥ 2).
- **Committed in:** `f084415` (folded into the Task 1 GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Confined to test-local naming; no production behaviour changed. It surfaced a small general lesson worth carrying forward — a grep-based acceptance criterion constrains identifier choice in tests as well as in production code, and a comment that quotes its own grep pattern is self-defeating.

## Issues Encountered

None. Both RED gates failed for the intended reason (Task 1 on an empty channel where a report was expected; Task 2 on `Some(Starting)` where `Some(Idle)` was required, which is exactly the catch-all promotion the arm exists to prevent), and both GREEN gates passed on the first implementation attempt.

## Verification Results

Run under `rtk proxy` where raw output matters (D-39) — the wrapper strips `warning:` and `test result:` lines, so any check greping wrapped output would pass vacuously.

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `rtk proxy cargo test` | **367 passed, 0 failed** (312 lib + 11 + 7 + 12 + 25) vs 363 baseline — +4, no regressions |
| `tests/executor_lifecycle.rs` | 11 passed (unchanged) |
| `tests/executor_transport.rs` | 7 passed (unchanged) |
| `cargo clippy -- -D warnings` | exit 0 |
| `rtk proxy cargo clippy --all-targets` | **exactly 5 warnings** — the pre-existing baseline, unchanged |
| `git diff --stat` vs base | only `src/app.rs`, `src/executor/claude.rs`, `src/executor/mod.rs` |
| D-37 stability | `git diff -U0 src/executor/mod.rs \| grep -c 'pub struct ExecutionHandle\|pub enum RunOutcome'` → `0`; `git diff --stat src/executor/gate.rs` → empty |

**Zero new crate dependencies**, as required.

## Self-Check: PASSED

- All three modified files exist on disk.
- All four commits verified present in `git log`: `12fda61`, `f084415`, `3fa00be`, `f5469a8`.
- Every task-level acceptance criterion re-run after the final commit; all pass.
- No stubs, no skipped tests, no unrun `<verify>` blocks — nothing to record in the broken-windows ledger.

## Known Stubs

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 16-06** can map `ExecutionEvent::EventsDropped { count }` into a journal record directly; the variant is the stable surface it was asked to consume, and it arrives ahead of `Exited` in stream order.
- **Plan 16-05** (Wave 2, also edits `src/app.rs`) is unobstructed: per the coordination note, the change here is confined to one added match arm plus a doc-comment paragraph. The `FileChanged` arm and `apply_exec_event`'s surrounding shape were not restructured.
- No blockers.

---
*Phase: 16-run-journal-state-substrate*
*Completed: 2026-07-29*
