---
phase: 15-transport-foundation
plan: 06
subsystem: tui-event-loop
tags: [tokio-select, biased, bounded-channel, backpressure, event-loop, ratatui, tdd, trans-03]
status: complete

requires:
  - phase: 15-02
    provides: "ExecutionEvent and RunState — both final, and RunState deliberately absent from ProjectState"
  - phase: 15-01
    provides: "the OQ1 PASS verdict this plan's precondition gates on"
provides:
  - "src/main_loop.rs — the extracted, testable select! body: pump(), PumpOutcome, EXEC_BATCH, REDRAW_INTERVAL, EXEC_CHANNEL_CAPACITY, ExecEvent"
  - "A biased tokio::select! whose arm order IS the priority order: input, then a bounded executor drain, then the loop's own redraw timer"
  - "A separate, bounded, process-lifetime executor channel that can never self-disable its select! arm"
  - "AppContext sibling fields exec_tx and run_states — driver state off ProjectState (D-19)"
  - "App::apply_exec_event and App::new_for_test (plus a private from_config both constructors share)"
  - "Three deterministic TRANS-03 property tests, each confirmed RED against a deliberate mutation"
affects:
  - "Phase 17 (spawns runs; owns the Idle→Starting→Running→Stopping→Finished close-out that this plan deliberately stops short of)"
  - "Phase 18 (every rendered driver surface; the bounded batch drain is what makes token-level rendering safe)"
  - "Any future phase tempted to route high-volume traffic through the Action FIFO — the biased-first Action arm makes that a starvation bug"

tech-stack:
  added: []
  patterns:
    - "Extract the select! body into a free `pump()` in the LIBRARY so the loop's priority ordering is testable at all — main.rs is the binary and nothing in it is reachable from `cargo test --lib`"
    - "Biased select! where arm order is documented AS the priority contract, with the starvation flip-side recorded at the arm itself"
    - "Bounded batch drain via one blocking recv() plus `batch - 1` try_recv(), so a burst returns control to the loop head"
    - "A per-iteration `sleep` instead of a long-lived `Interval`, because a free function cannot own an Interval across calls and a per-call Interval fires immediately"
    - "Channel-item envelope (`ExecEvent`) tagging a domain event with its routing key, so one process-lifetime channel can serve every alias"
    - "Mutation-confirmed tests: each property proven RED against a deliberate regression before being accepted GREEN"

key-files:
  created:
    - "src/main_loop.rs"
  modified:
    - "src/lib.rs"
    - "src/main.rs"
    - "src/app.rs"
    - "src/ui/screens/mod.rs"
    - "src/ui/screens/detail.rs"

key-decisions:
  - "pump() lives in src/main_loop.rs in the library, not src/main.rs — the placement decision the plan told us to make now rather than discover at test time"
  - "The channel carries ExecEvent { alias, event }, not a bare ExecutionEvent: one process-lifetime channel serves every alias, so a bare event would be unroutable to the per-alias map"
  - "The redraw arm is a per-iteration `sleep`, not a `tokio::time::Interval` with MissedTickBehavior::Delay — behaviourally identical, and the only form that survives extraction into a free function"
  - "apply_exec_event maps Exited to Stopping, never to Finished(outcome): a RunOutcome cannot be derived from an exit status alone, and fabricating one would be a lie the UI renders"
  - "EXEC_BATCH 64, REDRAW_INTERVAL 16ms, EXEC_CHANNEL_CAPACITY 8192 — all named constants, all untuned starting values (RESEARCH assumption A6)"
  - "Property 3 uses a multi-threaded runtime and a pre-filled channel, because on the default single-threaded runtime the consumer takes turns with the producer and the queue never backs up past one batch"

requirements-completed: [TRANS-03]

coverage:
  - id: D1
    description: "A keypress queued behind ten thousand executor events is handled on the FIRST pump iteration, not after them"
    requirement: TRANS-03
    verification:
      - kind: unit
        ref: "src/main_loop.rs#keypress_is_handled_before_a_flood_of_executor_events"
        status: pass
    human_judgment: false
  - id: D2
    description: "One pump iteration drains at most EXEC_BATCH executor events, so a burst returns control to the loop head rather than starving a redraw"
    requirement: TRANS-03
    verification:
      - kind: unit
        ref: "src/main_loop.rs#executor_burst_is_drained_in_bounded_batches"
        status: pass
    human_judgment: false
  - id: D3
    description: "Frames continue to render while the executor channel is saturated"
    requirement: TRANS-03
    verification:
      - kind: unit
        ref: "src/main_loop.rs#tui_renders_repeatedly_while_the_executor_channel_is_saturated"
        status: pass
    human_judgment: false
  - id: D4
    description: "The executor channel is bounded, is separate from the Action channel, and is created once for the process lifetime, so its select arm can never permanently disable itself when a run ends"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "grep -q 'mpsc::channel' src/main.rs (bounded, created once at startup); the sender clone is stashed on AppContext.exec_tx and the local binding is held until after run_tui_loop returns; pump() additionally carries an `else` branch"
        status: pass
    human_judgment: true
    rationale: "The three mechanisms are verified by construction and by grep, but no test exercises the between-runs window in which the arm would self-disable, because nothing in Phase 15 spawns a run from the TUI — Phase 17 owns that seam. The `else` branch is unreachable today (the timer arm's pattern is irrefutable), so it is a backstop rather than a tested path."
  - id: D5
    description: "The blocking editor shell-out keeps its exact observable behaviour — suspend, resolve the editor from the two environment variables then the fallback, run, report the three outcomes, re-initialise, request a redraw"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "git diff of src/main.rs shows zero changed lines in the editor block"
        status: pass
    human_judgment: true
    rationale: "Byte-preservation is verified by diff, which is stronger than a test would be here — but the block has no test coverage at all, before or after. That is the pre-existing wart D-18 explicitly accepts, not a gap this plan introduced."
  - id: D6
    description: "Driver state lives in sibling maps on AppContext and never on ProjectState, whose derived equality app.rs uses to suppress status-bar spam"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "grep -q 'run_states' src/ui/screens/mod.rs and grep -q 'Debug, Clone, Default, PartialEq' src/state_reader/mod.rs — ProjectState's derives are untouched"
        status: pass
    human_judgment: false
  - id: D7
    description: "No process handle, child stdin, or task join handle can ride inside an Action, which still derives Clone"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "grep -q '#[derive(Debug, Clone)]' src/action.rs and `cargo build` exits 0 — the compiler is the gate"
        status: pass
    human_judgment: false
  - id: D8
    description: "The 250ms tick survives, because it drives the twenty-tick session poll and the three-second status-message expiry"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "grep -q 'spawn_tick(250)' src/main.rs"
        status: pass
    human_judgment: false
  - id: D9
    description: "This plan adds no widget, no screen, and no layout"
    requirement: TRANS-03
    verification:
      - kind: other
        ref: "test ! -e src/ui/screens/driver.rs; `ls src/ui/screens/` is unchanged from before the plan"
        status: pass
    human_judgment: false

metrics:
  duration: "~35 min"
  completed: "2026-07-29"
  tasks: 2
  commits: 2
  files_created: 1
  files_modified: 5
---

# Phase 15 Plan 06: Responsive Event Loop Under Stream Load Summary

**A keypress queued behind ten thousand executor events is now provably handled on the first pump iteration — proven, not asserted, by three deterministic tests that were each confirmed to fail against a deliberate regression before being accepted.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2 (1 implementation, 1 TDD)
- **Files created:** 1
- **Files modified:** 5
- **Tests added:** 3 (259 → 262 lib tests)

## Accomplishments

- **The loop body is in the library, where it can actually be tested.** This was the plan's
  single most important structural instruction and it is worth restating why: `src/main.rs`
  declares `mod event; mod tui;` locally and is compiled as the *binary*. Nothing defined
  there is reachable from a `cargo test --lib` target, so a `pump()` left in `main.rs` would
  have made all three TRANS-03 tests unreachable — discovered at test-writing time, after the
  restructure was already committed.
- **Arm order is the priority contract, and it is documented as such at the arm.** The Action
  arm is first, so a control key can never queue behind bulk stream traffic. The comment at
  that arm records the flip side rather than just the benefit: *because* the arm is biased
  first, routing anything high-volume through the `Action` FIFO in a later phase would starve
  the other two. D-17's rule is load-bearing, and the place a future contributor will look is
  the arm itself, not a planning document.
- **Pitfall C is guarded twice.** The executor channel is created once for the process lifetime
  with a long-lived sender clone on `AppContext` *and* the local binding held until after
  `run_tui_loop` returns; `pump()` additionally carries an `else` branch. A third, incidental
  guard fell out of the extraction: `select!` disables an arm only for the lifetime of one
  `select!` expression, and `pump` builds a fresh one per call, so a disabled arm also recovers
  on the next iteration.
- **The editor block is byte-preserved.** `git diff src/main.rs` shows zero changed lines in
  `:155-197`. D-18 asked for observable behaviour to be preserved; the diff is the evidence.
- **Every test was confirmed RED before being accepted GREEN**, against three separate
  mutations. Details below — this is the part that distinguishes a test that guards the
  property from a test that merely passes alongside it.

## Task Commits

1. **Task 1: Biased, bounded event loop with `pump()` in the library** — `30b5816` (feat)
2. **Task 2: Deterministic proof that a keypress is never starved** — `0c1c34c` (test)

## The chosen constants and their rationale

All three are named constants in `src/main_loop.rs`, so tuning any of them is a one-line
change. None has tuning data behind it — this is RESEARCH assumption A6 carried forward
verbatim, and D-13 makes real tuning an explicitly later concern.

| Constant | Value | Rationale |
|----------|-------|-----------|
| `EXEC_BATCH` | 64 | The bound that returns control to the loop head. Large enough that the realistic event rate (the spike's 68-second turn emitted ~27 events *total*) never hits it, small enough that a synthetic burst cannot hold the loop for a perceptible interval. |
| `REDRAW_INTERVAL` | 16ms | ~60Hz. The arm that lets the loop make progress with no message at all — before this it was rescued only incidentally by the 250ms tick. |
| `EXEC_CHANNEL_CAPACITY` | 8192 | Deliberately generous. Bounded so that a render loop parked in the blocking editor shell-out applies backpressure rather than growing without limit (T-15-26) — but backpressure on the reader risks Claude's thirty-second exit drain, so the buffer wants a lot of headroom before that backpressure is ever felt. |

## Decisions the plan left open

### The channel carries `ExecEvent { alias, event }`, not a bare `ExecutionEvent`

15-PATTERNS sketched `pub exec_tx: Option<mpsc::Sender<ExecutionEvent>>`. That does not close:
the channel is created **once for the process lifetime** and is shared by every alias that is
ever driven, while `apply_exec_event` writes into a map **keyed by alias**. A bare
`ExecutionEvent` carries no alias and has no other way to be routed, so the handler would have
had nowhere to put it.

`ExecEvent` is a two-field envelope defined in `src/main_loop.rs` — the loop's own channel
contract, and the natural home for it, since `src/executor/mod.rs` is closed for this phase
(15-02 shipped its whole type surface at once specifically so no wave-3 plan reopens it).
`AppContext` imports it from there. The alternative — one channel per run — is exactly the
shape Pitfall C warns against.

### The redraw arm is a per-iteration `sleep`, not a `tokio::time::Interval`

The plan's action text specifies "a 16ms tick with a delaying missed-tick behaviour", and
RESEARCH's Pattern 3 sketch creates the `Interval` *outside* the loop. That works when the
loop body is inline. It does not survive the extraction into a free `pump()`: a free function
cannot own an `Interval` across calls, and an `Interval` reconstructed per call **fires its
first tick immediately**, which would spin the loop at full CPU.

Keeping the plan's four-parameter `pump` signature (which the test sketches also assume) and a
correct timer therefore required choosing one form. `MissedTickBehavior::Delay` means the next
delay starts when the previous tick was *consumed* — which is precisely what a per-iteration
`sleep(REDRAW_INTERVAL)` does. The two are behaviourally identical here, so this is the same
decision expressed in the only form that survives extraction. The reasoning is recorded in a
comment at the arm.

### `apply_exec_event` never fabricates a `Finished` outcome

`ExecutionEvent::Exited` moves the alias to `RunState::Stopping`, not to
`RunState::Finished(outcome)`. This is deliberate and is the more interesting half of the
handler: a `RunOutcome` cannot be derived from an exit status alone — D-26's matrix needs the
collected turns and the disk delta as well — so constructing one here would produce a value
the UI then renders as fact. The driver that owns the run (Phase 17) is what closes the state
out. The reasoning is in the method's doc comment so the next person does not "fix" it.

Any other observed line moves an `Idle` alias to `Starting`, so a run whose gated
`system/init` was somehow missed (a torn first line) does not read as `Idle` while output
streams past.

## Files Created/Modified

- `src/main_loop.rs` *(new)* — `pump`, `PumpOutcome`, `ExecEvent`, `EXEC_BATCH`,
  `REDRAW_INTERVAL`, `EXEC_CHANNEL_CAPACITY`, and the three property tests
- `src/lib.rs` — `pub mod main_loop;` between `executor` and `project_creator`
- `src/main.rs` — bounded executor channel created once at startup, sender clone stashed on
  `app.ctx`, `run_tui_loop` gains a fourth parameter and calls `pump`; the editor block and
  `spawn_tick(250)` untouched
- `src/app.rs` — `App::new_for_test`, `App::apply_exec_event`, and a private `from_config`
  that `App::new` and the test constructor share; two new lines in the `AppContext` literal
- `src/ui/screens/mod.rs` — `AppContext::exec_tx` and `AppContext::run_states`, each with a
  doc comment recording why the field is shaped the way it is
- `src/ui/screens/detail.rs` — two lines in the `test_ctx()` fixture (see deviations)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] A second exhaustive `AppContext` literal exists outside the plan's file list**

- **Found during:** Task 1 verification
- **Issue:** The plan states that `App::new` is the construction site needing a line per new
  field. There is a second one: `test_ctx()` at `src/ui/screens/detail.rs:4943`, a test fixture
  added in 14-04 that mirrors `app.rs` with an equally exhaustive literal and no
  `..Default::default()`. `cargo build` passed — the fixture is `#[cfg(test)]` — and `cargo test`
  then failed with `E0063: missing fields exec_tx and run_states`. The plan's `files_modified`
  list did not include this file, so the gap would have surfaced as a broken test target.
- **Fix:** `exec_tx: None, run_states: HashMap::new()` — the same two lines added to `app.rs`.
- **Files modified:** `src/ui/screens/detail.rs`
- **Verification:** `cargo test` green, 262 passing.
- **Committed in:** `30b5816`

**Scope check:** `detail.rs` is not assigned to either concurrent wave-3 plan (15-03 owns
`gate.rs`, `claude.rs`, `error.rs`, `tests/executor_transport.rs`; 15-05 owns `outcome.rs` and
`state_reader/git_ops.rs`), so there is no collision risk. The change is two lines in a test
fixture and adds no behaviour.

### Interpretations recorded

**2. Property 3's first draft applied no real load — caught by its own guard**

The first version of `tui_renders_repeatedly_while_the_executor_channel_is_saturated` failed
with `the channel was never actually saturated (peak queue depth 64)`. On the default
single-threaded `#[tokio::test]` runtime the flooding task and the render loop simply take
turns, so the queue never backs up past one batch and the test would have been reporting a
comfortable frame count under no load at all.

Fixed by running the test on a two-worker multi-threaded runtime — which is also the honest
model of production, where the reader task genuinely races the render loop — and by pre-filling
the channel to capacity before the first frame, so saturation is a *precondition* of the
measurement rather than something the test hopes will emerge during it. The `peak_queue`
assertion that caught this is retained deliberately: without it, a harness that quietly
stopped applying load would still pass.

**3. Task 2's RED came from mutation, not from absence**

Task 1 necessarily lands `pump()` before Task 2 writes its tests, so the tests cannot start red
by having no implementation. The plan anticipates this and asks instead that each test be
confirmed to fail "against a deliberately unbiased variant". That was done, one mutation at a
time, with the working tree restored between each:

| Mutation | Test that failed | Observed failure |
|----------|------------------|------------------|
| Executor arm moved *before* the Action arm (naive FIFO ordering) | `keypress_is_handled_before_a_flood_of_executor_events` | `keypress starved behind 9936 queued executor events (pump returned ExecEvents(64))` |
| Batch bound removed (`while applied < usize::MAX`) | `executor_burst_is_drained_in_bounded_batches` | drained the whole flood in one iteration |
| `apply_exec_event` no longer sets `needs_redraw` | `tui_renders_repeatedly_while_the_executor_channel_is_saturated` | `only 1 frames rendered in 2s under a saturated executor channel (peak queue depth 8192)` |

Each mutation was reverted immediately and the suite re-run green before committing. The first
row is the important one: it is the exact regression D-17 exists to prevent, and the assertion
message names the damage rather than just reporting `false`.

---

**Total deviations:** 1 auto-fixed (blocking), 2 interpretations recorded.
**Impact on plan:** None on scope. Every acceptance criterion in both tasks passes as written.

## Issues Encountered

Only the two above, and both were caught by the gates rather than by inspection — the `E0063`
by `cargo test`, and the no-load harness by the test's own `peak_queue` guard. Neither required
a design change.

## Known Stubs

None. Every surface this plan ships is wired and exercised.

The one thing that looks like a stub and is not: `AppContext.run_states` has no *producer* in
this phase. Nothing in Phase 15 spawns a run from the TUI — `DrivableProject`'s production
constructor is Phase 17's — so the map is written only by `apply_exec_event`, which the
property tests drive directly. That is the plan's intended shape, not an unfinished edge: this
plan ships the loop, and Phase 17 ships the thing that feeds it.

## Outstanding wart for a later quick task (D-18)

**The `pending_editor` shell-out still blocks the render thread.** `src/main.rs` calls
`ratatui::restore()` and then a synchronous `std::process::Command::status()` inside the loop
body, so the TUI is unreadable for as long as the user's editor is open, and — once Phase 17
starts driving runs — executor events accumulate in the channel meanwhile.

This was preserved deliberately and byte-for-byte. It is off this phase's critical path, and
widening the blast radius of an event-loop change is exactly how a foundation phase turns into
a regression hunt. The bounded channel is what makes the blocking window *safe* rather than
unbounded: a long editor session now applies backpressure to the reader instead of growing
memory without limit (T-15-29, accepted).

Suggested shape for the fix when it is taken up: move the shell-out to `spawn_blocking` and
deliver its result back as an `Action`, so the loop keeps pumping while the editor is open. It
is a self-contained quick task; nothing in this plan blocks it.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern, or schema at a trust boundary.
Every `mitigate` disposition in the plan's register is implemented:

- **T-15-25** (control keys starved) — the biased Action-first arm, proven by
  `keypress_is_handled_before_a_flood_of_executor_events`, which fails the instant the bias is
  removed or the channels are merged. Confirmed by mutation, not assumed.
- **T-15-26** (unbounded channel growth while the editor holds the loop) — `mpsc::channel`, not
  `unbounded_channel`, with a generous 8192 capacity.
- **T-15-27** (`select!` panicking with every arm disabled) — created once for the process
  lifetime, long-lived sender clone on `AppContext`, plus an `else` branch, plus the incidental
  per-call recovery noted above.
- **T-15-28** (events silently dropped when full) — **no `try_send` on the production path.**
  The only sender in `src/main.rs` is the stashed clone; producers use `send().await`, which
  applies backpressure rather than dropping. `try_send` appears only inside the test module,
  where its failure is an explicit `expect` that fails the test. The prohibition against a
  silent drop is therefore satisfied by construction: there is no code path that discards an
  event without either blocking or panicking.
- **T-15-29** (blocking editor shell-out) — **accepted** per D-18, documented above.

## Verification

| Gate | Result |
|------|--------|
| Task 1 `<verify>` | PASS |
| Task 2 `<verify>` | PASS |
| `cargo build` | PASS |
| `cargo test` | PASS — 262 lib tests (was 259), 6 suites, 0 failures |
| `cargo clippy -- -D warnings` | PASS |
| `cargo clippy --all-targets --message-format=short` | **exactly 5** warning lines — the frozen pre-existing set (browser.rs ×3, project_creator.rs ×1, state_reader/mod.rs ×1). Count did not grow. |
| `cargo test --lib main_loop` | PASS — exactly the three named property tests |

Every grep-shaped acceptance criterion from both tasks was executed individually:
`pub mod main_loop;`, `biased;`, `EXEC_BATCH`, `mpsc::channel`, `exec_tx`, `run_states`,
`Debug, Clone, Default, PartialEq` in `state_reader/mod.rs`, `#[derive(Debug, Clone)]` in
`action.rs`, `spawn_tick(250)`, `pub fn new_for_test`, `pub fn apply_exec_event`; and the
negatives `test ! -e src/ui/screens/driver.rs` plus an unchanged `ls src/ui/screens/`.

## Flagged Planner Assumption — carried forward, still open

The plan surfaced **TRANS-03 / unclassified** as an explicit unresolved planner assumption: the
spec-less edge probe returned `unclassified` for TRANS-03, and the planner's reading was that
its edge is the PRIORITY-under-load axis rather than a data-shape axis. This plan implemented
that reading — all three properties are priority and progress properties, none is an
input-shape property.

Recording it here rather than closing it: nothing in execution either confirmed or refuted the
reading, because no data-shape edge presented itself. The loop routes envelopes and does not
interpret them (the dummy event's variant is irrelevant to everything proven), so if TRANS-03
does have a data-shape edge, this plan is not where it would surface — `stream_json.rs`'s
tolerant model in 15-02 is.

## Next Phase Readiness

No blockers. The seams the later phases need are in place and fixed:

- **Phase 17** takes `app.ctx.exec_tx.clone()` to feed events in, and owns the
  `Starting → Running → Stopping → Finished(outcome)` close-out that `apply_exec_event`
  deliberately stops short of. It needs no change to `pump`.
- **Phase 18** renders `app.ctx.run_states`. If it wants token-level output it must add
  `--include-partial-messages` to the argv, which raises the event rate by orders of magnitude
  — the bounded batch drain is what makes that safe, and the three tests in `main_loop.rs` are
  what will prove it still holds.

## Self-Check: PASSED

- `src/main_loop.rs` exists on disk (17,265 bytes).
- All five modified files carry their changes.
- Both commit hashes (`30b5816`, `0c1c34c`) resolve in `git log`.

---
*Phase: 15-transport-foundation*
*Completed: 2026-07-29*
