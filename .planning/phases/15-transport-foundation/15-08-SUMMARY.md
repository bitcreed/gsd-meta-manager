---
phase: 15-transport-foundation
plan: 08
subsystem: executor
tags: [transport, supervisor, deadlines, backpressure, process-group, teardown, gap-closure]
status: complete

requires:
  - 15-04 (the supervisor select! loop, the two deadlines and the four-step teardown this plan repairs)
  - 15-07 (the envelope collection and run-end control drain in the same function; this plan is wave 6 behind it)
provides:
  - "A supervisor whose every bound is evaluated unconditionally on every loop pass, independent of select! arm ordering"
  - "A bounded hand-off to the caller (EVENT_FORWARD_TIMEOUT) with a counted drop, so a stalled consumer cannot disable the caps or cancel"
  - "An absolute post-exit drain bound (POST_EXIT_DRAIN_CAP) that still reaps a group which outlived its leader"
  - "A run that ALWAYS ends with outcome_tx sent"
affects:
  - Phase 16 (the journal will want the dropped-event count; it is new operational behaviour)
  - Phase 18 (queued -> delivered -> acted-on display reads events that may now be dropped under backpressure)
  - Phase 20 (the router acts on RunOutcome as fact, which now always arrives)

tech-stack:
  added: []
  patterns:
    - "Two layers, kept apart: the select! is the WAKE-UP mechanism and the loop head is the ENFORCEMENT mechanism, so arm ordering is a stream-fidelity preference and never a correctness dependency"
    - "Every cross-task hand-off is bounded by the EARLIEST armed deadline, so a bound expiring mid-send unparks the waiter at exactly the right instant"
    - "A bound that switches off when the thing it guards happens is not a bound"
    - "Every new async integration test carries its own tokio::time::timeout so a regression FAILS rather than HANGS CI"

key-files:
  created:
    - tests/fixtures/fake-claude-orphan.sh
  modified:
    - src/executor/claude.rs
    - tests/executor_lifecycle.rs
    - tests/fixtures/fake-claude-slow.sh

key-decisions:
  - "EVENT_FORWARD_TIMEOUT = 5s — generous against the TUI's known blocking $EDITOR shell-out, but finite, because a parked supervisor has every cap AND cancel disabled at once"
  - "POST_EXIT_DRAIN_CAP = 5s — the CLI terminates its background Bash tasks about five seconds after the final result, so a stream still open past that is being held by a survivor"
  - "The post-loop Exited send was bounded too (deviation): it shares the channel a stalled consumer has already filled and was a third way to end a run without sending outcome_tx"
  - "The orphan stand-in's descendant chatters briefly before falling silent — a descendant silent from the first instant never lets the exited flag flip at all, because ProcessGroupChild::wait blocks inside the group reap"
  - "The SIGKILL-escalation test is un-ignored: this plan gave the escalation path a second entry point, and a two-entry path with zero CI coverage is the shape that produced CR-02"

patterns-established:
  - "Loop-head enforcement: read every deadline and the cancel signal at the top of each supervisor pass, before the select!, and act on a terminate observed there immediately rather than deferring it behind the very send it exists to interrupt"
  - "Bounded diagnostic loss: an event that cannot be delivered inside its bound is counted and logged as a NUMBER, never as content"

requirements-completed: [TRANS-01]

coverage:
  - id: D1
    description: "Both deadlines, the grace, the post-exit drain slot and the cancel signal are evaluated on every supervisor loop pass, not only when select! reaches their arm"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked"
        status: pass
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_cancel_is_still_honoured_while_the_event_consumer_is_blocked"
        status: pass
    human_judgment: false
  - id: D2
    description: "The forward of a parsed event to events_tx is bounded by the earliest armed deadline and by EVENT_FORWARD_TIMEOUT, so a stalled consumer cannot park the supervisor with every cap disabled"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked"
        status: pass
      - kind: other
        ref: "grep -c 'timeout_at(forward_deadline' src/executor/claude.rs == 1 and grep -cE 'events_tx$' src/executor/claude.rs == 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "An event dropped because its bounded forward expired is counted and reported through a count-only tracing::warn!, never with the event's content"
    requirement: TRANS-01
    verification:
      - kind: other
        ref: "observed live: 1 drop in the wall-clock flood test, 3 in the cancel flood test, via temporary instrumentation on the committed tree"
        status: pass
    human_judgment: true
    rationale: "That the warning carries no raw line content is a privacy property read from the source, not something a test asserts; SAFE-04 redact-at-capture does not land until Phase 16, so anything logged now stays unredacted forever (T-15-53)."
  - id: D4
    description: "An absolute post-exit drain bound, NOT guarded by the exited flag, ends the loop when the leader has exited but a surviving descendant still holds the stdout pipe"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run"
        status: pass
    human_judgment: false
  - id: D5
    description: "A group not proven reaped still gets the documented four-step teardown, and the run always ends with outcome_tx sent and no surviving descendant"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run"
        status: pass
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped"
        status: pass
    human_judgment: false
  - id: D6
    description: "The three D-13 doc-claim sites describe the enforced mechanism rather than the intended one"
    requirement: TRANS-01
    verification: []
    human_judgment: true
    rationale: "Whether a doc comment now matches the code is a reading judgment; the defect being remedied is precisely that a plausible-sounding comment passed review while the code contradicted it, so a grep for the words would reproduce the failure mode rather than catch it."

metrics:
  duration: ~65 min
  tasks: 3
  files: 4
  completed: 2026-07-29
  tests_before: 361 passed, 1 ignored
  tests_after: 363 passed, 0 ignored
---

# Phase 15 Plan 08: Gap Closure — The Supervisor's Two Reachable Hangs Summary

Made every bound the supervisor owns enforceable on every loop pass, bounded the hand-off to the caller so a stalled consumer cannot disable them, and gave the post-exit drain an absolute cap that still reaps a process group which outlived its leader.

## What shipped

Two defects in one `select!` block, deliberately not split across plans because fixing them separately would mean rewriting the same twenty lines twice.

### CR-01 — a hot stream or a stalled consumer disabled both caps AND cancel

The loop was single-layer: the `biased;` `select!` was simultaneously the wake-up and the enforcement mechanism. Two independent failures fell out of that:

- **Starvation.** With the reader arm first, a child emitting faster than the loop retired kept arm 1 permanently ready, so the wall-clock arm, the idle arm, the grace arm and `cancel_rx` were *never polled*.
- **Suspension.** Once the reader arm won, the loop sat awaiting `events_tx.send(...)` **outside** the macro, with every timer future already dropped.

Both are now structurally impossible. An unconditional block at the top of every pass reads `wall_deadline`, `idle_deadline`, `grace_deadline`, `drain_deadline` and drains `cancel_rx` with a non-blocking `try_recv`, and the per-pass `forward_deadline` is the **earliest** of `EVENT_FORWARD_TIMEOUT` and every armed bound — so a cap expiring while the supervisor is parked unparks it at exactly the right instant and the loop head classifies the breach on the next pass.

The `select!` and all its arms are kept, `biased;` included. With enforcement hoisted, arm ordering is a stream-fidelity preference and reader-first is still the right one.

### CR-02 — an observed exit switched off every remaining bound

Every deadline arm was guarded `if !exited`, and the post-loop dispatch's `if exited { exit_status }` skipped the teardown entirely. The only remaining escape was stdout EOF — which requires *every* process holding the write end to be gone, and `claude` routinely backgrounds Bash grandchildren that inherit stdout.

`POST_EXIT_DRAIN_CAP` is armed by the exit arm and read by both the loop head and a `select!` arm gated **only** on the deadline being armed. The post-loop dispatch now splits on whether the group was *proven* reaped:

| Condition | Action | Reported status |
|---|---|---|
| exited, drain bound did NOT expire | nothing — stdout EOF proves every writer is gone | the observed `exit_status` (byte-identical to today) |
| exited, drain bound DID expire | full four-step teardown; returned status discarded | the leader's own `exit_status` |

The teardown in the second row is for the survivors, not for the verdict: the leader's status is the authoritative liveness signal.

## Re-verification facts (requested by the plan's `<output>`)

### Every `missing:` bullet in `15-VERIFICATION.md`, closed and attributed

**GAP 1 (SC-2 / TRANS-02 / CR-04)** — both bullets closed by **15-07**:

1. *"`Coordinator::run` must collect `Vec<ResultMessage>` … and call `derive_run_outcome_from_envelopes`"* — **closed by 15-07.** The coordinator collects full envelopes; the envelope-discarding entry point was deleted, not deprecated.
2. *"An end-to-end regression test driving a stand-in whose terminal envelope carries a non-empty `permission_denials[]` through the real `Coordinator`"* — **closed by 15-07** (`a_permission_blocked_run_is_reported_as_permission_denied_not_success`).

**GAP 2 (SC-1 / TRANS-01 / CR-01·02·03)** — four bullets, split across the two plans:

1. *"Deadlines and cancel must be evaluated unconditionally each loop pass … and the forward to `events_tx` must be bounded"* — **closed by 15-08, Task 1.** Both halves: the loop-head block and `EVENT_FORWARD_TIMEOUT`.
2. *"An absolute post-exit drain bound (not guarded by `!exited`) that still calls `tear_down_group` / `terminate_group` when the group has not been proven reaped"* — **closed by 15-08, Task 2.**
3. *"`pending_control` cleared at run end … and/or a bounded timeout on `interrupt()`'s `rx.await`"* — **closed by 15-07**, which implemented *both* release paths rather than either.
4. *"A regression test exercising each of the three conditions"* — closed across both plans, one condition each:
   - fast stream / blocked consumer → **15-08**, two tests (the wall cap and the cancel);
   - exited-but-descendant-alive → **15-08**, `a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run`;
   - interrupt with no `control_response` → **15-07**, two tests (drain and cap).

No `missing:` bullet in either gap is left open.

### The two chosen constants

**`EVENT_FORWARD_TIMEOUT = Duration::from_secs(5)`.** The consumer is a TUI that can legitimately stop draining for a while — the blocking `$EDITOR` shell-out is the documented case — so the ceiling is generous rather than tight. But it is finite, because a supervisor parked on a send has its wall-clock cap, its idle cap, its grace and its cancel **all** disabled simultaneously, which is CR-01 exactly. Five seconds is far longer than any healthy TUI stall and far shorter than a run. The cost of exceeding it is one counted, logged dropped event; that trade is deliberate, and a dropped diagnostic is strictly preferable to an unbounded park — the former loses a line of history and says so, the latter loses the run.

**`POST_EXIT_DRAIN_CAP = Duration::from_secs(5)`.** Once the leader has exited the remaining lines are already framed and in flight, and the CLI terminates its background Bash tasks about five seconds after the final result (D-14, "Other headless behaviours"). A stream still open after that is being held by something that outlived the leader — precisely the condition the bound exists to end. It is the *mirror* of `EXIT_DRAIN_CAP`, not a duplicate: that one bounds *"the stream closed — is the process gone?"*, this one bounds *"the process is gone — is the stream closed?"*. The doc comments now cross-reference each other by direction, because the names invite the confusion.

### Measured wall-clock duration of each test

Run individually, `cargo test --test executor_lifecycle <name>`:

| Test | Duration | Inner hard timeout | RED observed |
|---|---|---|---|
| `a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked` | **8.02s** | 60s | yes — 61.01s inner-timeout failure |
| `a_cancel_is_still_honoured_while_the_event_consumer_is_blocked` | **20.12s** | 60s | yes — 61.01s inner-timeout failure |
| `a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run` | **5.01s** | 60s | yes — 60.01s inner-timeout failure |
| `a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped` (un-ignored) | **10.01s** | none (waits out the real grace) | n/a — pre-existing test |

The whole `executor_lifecycle` suite runs in **20.13s** wall-clock: the harness runs these concurrently on threads, so the escalation test's ten seconds are absorbed by the cancel test beside it rather than added to it. Every RED was directly observed as a **failure**, never as a hung suite — that is the single property these hard timeouts exist for.

The cancel test's 20 seconds are the design, not slack: up to `EVENT_FORWARD_TIMEOUT` to observe the cancel while parked, then the real ten-second `TEARDOWN_GRACE` elapsing under the drain, then the escalation.

### `cargo clippy --all-targets` warning count

**Exactly 5**, unchanged from the pre-replan baseline, and none in any file either plan touched:

| Count | Lint | File |
|---|---|---|
| 3 | used `assert_eq!` with a literal bool | `src/browser.rs:131`, `:132`, `:133` |
| 1 | this creates an owned instance just for comparison | `src/project_creator.rs:146:27` |
| 1 | items after a test module | `src/state_reader/mod.rs:258:1` |

`cargo clippy --all-targets --message-format short | grep -cE 'src/executor/(claude|mod|outcome)\.rs|tests/executor_(lifecycle|transport)\.rs'` returns **0**.

Note for whoever re-runs this: clippy caches, and a second invocation on an unchanged tree prints `No issues found` rather than the five. `touch` a source file first, or the check is silently vacuous.

### The scope fence held

`git status --porcelain src/main_loop.rs src/executor/gate.rs` returns **empty output** on a clean tree, and neither file appears in any of this plan's three commit diffs (nor 15-07's two). SC-3 and SC-4 were independently verified clean and were fenced out of both plans.

### Dropped-event counts observed live

New operational behaviour Phase 16's journal will want. Measured by temporarily instrumenting the counter on the committed tree, then restoring with `git checkout --`:

| Test | Events dropped |
|---|---|
| `a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked` | **1** |
| `a_cancel_is_still_honoured_while_the_event_consumer_is_blocked` | **3** |

Both are exactly what the mechanism predicts: the wall-clock run parks once on a forward whose deadline *is* the wall deadline and drops that one event as it breaches; the cancel run parks repeatedly across its ~15 seconds of grace and drops one per park. **No other test in the suite drops anything** — a run whose consumer keeps up never loses an event, which is what makes `a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap` still receive all twenty of its heartbeats.

## Task commits

| Task | Name | Commit | Files |
|---|---|---|---|
| 1 (tracer, TDD) | Per-pass bound evaluation and the bounded event forward | `c01c1a8` | claude.rs, executor_lifecycle.rs, fake-claude-slow.sh |
| 2 (TDD) | The post-exit drain bound and always reaping the group | `9b84c29` | claude.rs, executor_lifecycle.rs, **fake-claude-orphan.sh** (new) |
| 3 | The D-13 doc claims and the retired ignore | `86503bf` | claude.rs, executor_lifecycle.rs |

## Files Created/Modified

- `src/executor/claude.rs` — the loop-head enforcement block, `EVENT_FORWARD_TIMEOUT`, `POST_EXIT_DRAIN_CAP`, `forward()`, the drain `select!` arm, the split post-loop dispatch, and the three rewritten doc-claim sites.
- `tests/fixtures/fake-claude-orphan.sh` (**new**, executable) — a leader that exits while a descendant keeps the stdout pipe open.
- `tests/fixtures/fake-claude-slow.sh` — a zero-interval fast path so the stand-in can actually flood.
- `tests/executor_lifecycle.rs` — the `flooding` helper, the `FAKE_ORPHAN` const, the three regression tests, and the retired ignore.

## Decisions Made

Beyond the two constants documented above:

- **The breach checks at the loop head are guarded on `!cancelled`, and that guard is required rather than incidental.** Hoisting them makes them run every pass, which would make the pre-existing breach-outranks-cancel race (WR-02) fire far *more* often than it does today. The guard keeps that race exactly as frequent as it already is. The outcome match's arm ordering — which is what WR-02 is actually about — was left untouched as out of scope.
- **A terminate observed at the loop head is acted on at the loop head.** Deferring it to the post-`select!` block, where the original code handles it, would park the SIGTERM behind the very send the cancel exists to interrupt — adding a full `EVENT_FORWARD_TIMEOUT` to every cancel latency under load.
- **`read_stderr`'s own direct send was left alone**, per the plan. It runs in its own task holding no supervisor state, which is exactly why D-04's rule that stdout and stderr are never merged means blocking there cannot park the supervisor.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] The post-loop `Exited` send was unbounded on the same channel a stalled consumer had already filled**

- **Found during:** Task 1, reasoning through why the new wall-clock test would still hang after the loop-head fix.
- **Issue:** The plan scoped the bounded forward to `handle_item` and explicitly left the post-loop `events_tx.send(ExecutionEvent::Exited(status)).await` alone. But that send sits between *"the outcome is decided"* and *"the outcome is sent"*, on the very channel the flood test keeps full — and `ExecutionHandle::wait_outcome` awaits only `outcome_rx`, draining nothing. Left unbounded it is a **third** reachable path to a run that never sends `outcome_tx`, which directly contradicts this plan's own must-have truth that a run ALWAYS ends with `outcome_tx` sent. Neither new flood test could have passed with it in place.
- **Fix:** Wrapped it in `tokio::time::timeout(EVENT_FORWARD_TIMEOUT, …)`. The `Exited` event may be dropped for a consumer that has stopped draining; the outcome is not.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** Both flood tests pass; every test that *does* drain still observes its `Exited` event (`reported_an_exit` assertions in four tests). The plan's acceptance grep `grep -c "events_tx.send(" == 1` still holds, since this is the same single call site.
- **Committed in:** `c01c1a8`

**2. [Rule 3 — Blocking] `drain_deadline` / `drain_expired` declared in Task 2 rather than Task 1**

- **Found during:** Task 1, running `cargo clippy -- -D warnings`.
- **Issue:** The plan asks Task 1 to declare both locals so the loop head reads them from the start, with Task 2 only setting them. But in Task 1 nothing assigns `drain_deadline` (so `mut` is unused) and nothing reads `drain_expired` (so its assignment is dead) — both are `-D warnings` failures, and Task 1's own acceptance criteria require `cargo clippy -- -D warnings` to exit 0. Each commit must independently pass the gate.
- **Fix:** Deferred both declarations, the loop-head drain check and the `forward_deadline` drain term to Task 2, where the exit arm assigns the deadline and the post-loop dispatch reads the flag. The end state is byte-for-byte what the plan specifies; only the commit boundary moved.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** `cargo clippy -- -D warnings` exits 0 at both `c01c1a8` and `9b84c29`; all Task 2 acceptance greps pass (`POST_EXIT_DRAIN_CAP` ≥ 3, `drain_deadline.is_some()` ≥ 1, `drain_expired` ≥ 3).
- **Committed in:** `c01c1a8` / `9b84c29`

**3. [Rule 1 — Bug] The orphan stand-in's descendant must chatter briefly; a fully silent one proves nothing**

- **Found during:** Task 2, first GREEN attempt.
- **Issue:** The plan specifies *"a plain long sleep is enough, because holding the write-end file descriptor is the whole mechanism and it never needs to write."* Empirically it is not enough, for a reason worth recording: `ProcessGroupChild::wait()` awaits the leader, **caches** its status, then blocks in `spawn_blocking` reaping the rest of the group. Against a descendant silent from the first instant, the supervisor's exit-arm future is created, reaches that blocking reap, and stays pending — nothing else is ready to displace it — so `exited` **never flips**, `drain_deadline` is never armed, and the run ends on the idle cap reporting `Stalled`. The test would have proven nothing about the post-exit drain.
- **Fix:** The descendant emits ten lines at 200 ms before falling silent. Those lines let another `select!` arm win, dropping the partially-polled wait future; the next poll then returns the **cached** leader status immediately — which is exactly the "exited but NOT reaped" state CR-02 describes, and which the review itself names as the reachable mechanism. It then holds the pipe in silence for 90 s, so the drain bound is demonstrably the only thing that can end the run.
- **Files modified:** `tests/fixtures/fake-claude-orphan.sh` (documented in the script at the point of the decision)
- **Verification:** RED failed on its own 60 s inner timeout (60.01s); GREEN passes in 5.01s, which is `POST_EXIT_DRAIN_CAP` plus the teardown and nothing else.
- **Committed in:** `9b84c29`

**4. [Rule 3 — Blocking] `forward`'s sender parameter is named `sender`, not `events_tx`**

- **Found during:** Task 1, checking acceptance criteria.
- **Issue:** The plan's artifact table gives the signature as `forward(events_tx: &mpsc::Sender<ExecutionEvent>, …)`. With that name, `forward`'s own body reads `events_tx.send(event)` — which makes the plan's own acceptance criterion `grep -c "events_tx.send(" src/executor/claude.rs` return **2**, not the required `1`. The criterion's stated purpose is to guard against a *new direct* send being introduced; the bounded funnel is the opposite of that.
- **Fix:** Named the parameter `sender`. This is also better as code: `events_tx` is the coordinator's name for its channel, and `forward` receiving it as a generic `sender` makes the "everything goes through here" distinction visible in the source. The executable criterion was preferred over the documentation table.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** `events_tx.send(` = 1, `tx.send(ExecutionEvent::Stderr` = 1, `events_tx$` (end of line) = 0 — all three plan criteria satisfied simultaneously.
- **Committed in:** `c01c1a8`

---

**Total deviations:** 4 auto-fixed (2 bugs, 2 blocking).
**Impact on plan:** No scope creep — every change stayed inside the plan's three `files_modified`, and `Cargo.toml`/`Cargo.lock` are untouched, as the threat model requires. Deviation 1 was necessary for the plan's own success criteria to be reachable at all; deviations 2 and 4 are mechanical consequences of the gate being stricter than the plan's prose; deviation 3 corrects a factual assumption about `process-wrap` semantics.

## Issues Encountered

**The Task 1 revert check produced a different failure mode than the plan predicted, and the difference is informative.** The criterion asks that reverting *only* the loop-head block makes both tests fail on their inner timeouts. Observed: the cancel test did (61.01s, `Elapsed`), but the wall-clock test failed on its **outcome assertion** instead, reporting `SucceededNoChanges`. Reason: with the bounded forward still present, expired forwards keep dropping events and the loop keeps consuming, so the flood eventually completes and the run ends *normally* — with the wall-clock cap breached and never reported. That is a wrong answer rather than a hang, which is arguably the worse failure, and it confirms the loop-head block is load-bearing. The original RED (before either change) failed on the inner timeout exactly as specified.

**A failing orphan test leaves one stray `sleep` for up to 90 seconds.** libtest ends the process with `exit`, so no destructor runs and `KillOnDrop` never fires — the descendant's self-terminating bound is the real backstop, not a third safety net. This is recorded in the fixture's header so the next reader does not assume otherwise. On a passing run the executor's own teardown reaps it within ~5 seconds, which is what the test asserts.

## Verification

Run from the project root, all green:

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | **363 passed, 0 failed, 0 ignored** (baseline 361 passed + 1 ignored; +2 new, +1 un-ignored) |
| `cargo test -- --include-ignored` | identical — there is nothing left to include |
| `cargo clippy -- -D warnings` | exit 0, no issues |
| `cargo clippy --all-targets` | **exactly 5 warnings**, all pre-existing, none in a file this replan touched |
| `cargo test --test executor_lifecycle` | 11 tests, 0 failed, 0 ignored |
| `cargo test --test executor_transport` | 7 passed — the clean end-to-end path is unregressed |
| `grep -rlE '^[[:space:]]*#\[ignore' tests/ src/ \| wc -l` | `0` |
| `cargo test 2>&1 \| grep -cE '[1-9][0-9]* ignored'` | `0` |
| `git status --porcelain src/main_loop.rs src/executor/gate.rs` | empty |
| `git log --oneline -3` | `fix(15-08)`, `fix(15-08)`, `test(15-08)` |

Acceptance greps, all satisfied: `EVENT_FORWARD_TIMEOUT` = 3; `timeout_at(forward_deadline` = 1; `events_tx$` = 0 (was 8); `events_tx.send(` = 1; `tx.send(ExecutionEvent::Stderr` = 1; `cancel_rx.try_recv()` = 1; `sleep "$INTERVAL"` in the slow fixture = 1 (now guarded); `POST_EXIT_DRAIN_CAP` = 4; `drain_deadline.is_some()` = 1 (and that arm does **not** also test `exited`); `drain_expired` = 4; `^\s*wait\s` in the orphan fixture = 0 (discriminating: `1` against the spawner, `0` against the slow stand-in); `/home/|home-blk` in the orphan fixture = 0.

Both revert checks were performed by hand against the committed tree and restored: disabling only the loop-head block fails both flood tests within 61s, and disabling only the post-exit drain arm fails the orphan test on its 60s inner timeout.

## Unregressed behaviour explicitly re-checked

- `a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap` — still passes, still receives **all twenty** heartbeats. A run emitting continuously into a healthy consumer is still never killed by the idle cap, and never drops an event.
- `a_child_that_goes_silent_trips_the_idle_cap_and_is_reported_as_stalled` and `a_run_that_outlives_the_wall_clock_cap_is_reported_as_timed_out` — unchanged.
- `a_grandchild_spawned_by_the_child_is_gone_after_teardown` and `a_torn_down_run_is_reaped_and_reports_an_exit_status` — the existing teardown proofs are unregressed.
- The multi-turn tests are untouched: `result` still closes a **turn**, never the run (D-29), and 15-07's envelope collection and run-end control drain were not modified.

## Known Stubs

None. Both defects are closed in production code with regression tests against real spawned children; nothing was left as a placeholder, and no `TODO`/`FIXME` marker was introduced.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or trust-boundary schema change. The six `mitigate` dispositions in the plan's threat register were all applied:

- **T-15-50** (DoS, supervisor loop): bounds hoisted to a per-pass evaluation; the forward capped at the earliest armed deadline. Two regression tests.
- **T-15-51** (DoS, post-exit drain): `POST_EXIT_DRAIN_CAP`, explicitly not guarded by the exited flag; `outcome_tx` is always sent — including from the post-loop `Exited` send, which deviation 1 also bounded.
- **T-15-52** (EoP, surviving group members): the post-exit path runs the full SIGTERM → grace → SIGKILL → reap teardown whenever the group was not proven reaped; the regression test asserts the descendant is gone within 5 seconds.
- **T-15-53** (info disclosure, drop diagnostic): the warning carries the running count and the channel capacity and nothing else — no event, no raw line, no message body.
- **T-15-54** (info disclosure, fixture): `fake-claude-orphan.sh` is fully synthetic; both the literal and dash-encoded host-path forms grep to zero.
- **T-15-55** (repudiation, doc comments): all three D-13 claim sites rewritten to describe the enforced mechanism. The claim was not weakened — it is now enforced, and the docs say how.
- **T-15-SC** (tampering, package installs): accepted, and vacuous — zero package-manager tasks; `Cargo.toml` and `Cargo.lock` are untouched.

## Next Phase Readiness

Phase 15's SC-1 gap is closed on both halves it owned. Every `missing:` bullet in both `15-VERIFICATION.md` gaps is now addressed across 15-07 and 15-08, so the phase is ready for a re-verification pass.

Two things the re-verifier should carry forward rather than rediscover:

1. **The dropped-event count is new operational behaviour.** It is currently a `tracing::warn!` and nothing else. Phase 16's journal should record it per run — a run that silently lost 40 events is materially different from one that lost none, and the count is the only signal that distinguishes them.
2. **`WR-02` (breach outranks cancel in the outcome match) is still open and was deliberately not touched.** The loop-head breach checks are guarded on `!cancelled` specifically so hoisting them did not make that race more frequent. Whoever fixes WR-02 should fix the outcome match's arm ordering, not remove that guard.

## Self-Check: PASSED

- `src/executor/claude.rs` — FOUND
- `tests/fixtures/fake-claude-orphan.sh` — FOUND (executable)
- `tests/fixtures/fake-claude-slow.sh` — FOUND
- `tests/executor_lifecycle.rs` — FOUND
- commit `c01c1a8` — FOUND
- commit `9b84c29` — FOUND
- commit `86503bf` — FOUND

---
*Phase: 15-transport-foundation*
*Completed: 2026-07-29*
