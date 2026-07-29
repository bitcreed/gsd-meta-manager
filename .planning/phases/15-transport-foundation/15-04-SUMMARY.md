---
phase: 15-transport-foundation
plan: 04
subsystem: executor-lifecycle
tags: [deadlines, idle-cap, wall-clock-cap, process-group, teardown, sigterm, reaping, orphan-prevention]
status: complete

requires:
  - phase: 15-01
    provides: "OQ1 PASS verdict; the measured 774.6s multi-step run that sets the scale these caps must not undercut"
  - phase: 15-02
    provides: "the spawn path, the three pipe tasks, the Coordinator, ExecutionOptions' cap fields, the pgid recorded at spawn"
  - phase: 15-03
    provides: "the fail-closed gate and the duplex control half that share claude.rs; the Coordinator shape this plan extends"
provides:
  - "A supervisor racing five things: the stream, the child's exit future, the wall-clock cap, the idle cap and the cancel signal (D-13)"
  - "last_line_at — a watch channel the READER stamps on every observed line; the idle arm re-arms from it and from nothing else"
  - "Breach{WallClock,Idle} → RunOutcome::{TimedOut,Stalled}: two caps, two classifications, never collapsed"
  - "terminate_group / tear_down_group / finish_teardown / await_clean_exit — the four-step teardown, split so a cancel never pays two grace periods"
  - "EXIT_DRAIN_CAP (30s) — the bound that makes 'no path awaits exit unbounded' absolute rather than nearly true"
  - "tests/fixtures/fake-claude-{slow,spawner,deaf}.sh (mode 100755)"
  - "tests/executor_lifecycle.rs — 6 unix-gated tests, 1 ignored"
affects:
  - "Phase 17 (the user-facing kill switch and orphan reaping build directly on tear_down_group and the recorded pgid)"
  - "Phase 20 (run-level step/wall-clock/no-progress caps sit ABOVE these invocation-level ones)"
  - "15-06 (consumes ExecutionEvent and RunOutcome; TimedOut and Stalled are now reachable from the supervisor, not only from the derivation)"

tech-stack:
  added: []
  patterns:
    - "A watch channel as a liveness clock: the producer stamps, the consumer re-arms a sleep_until from the stamp — the timer is driven by observed work rather than by elapsed time"
    - "tokio::select! as borrow arbitration, not just concurrency: the exit future's &mut borrow ends when select drops the losers, which is the only reason signal() is callable in the same block"
    - "A deadline that starts inside the loop and is honoured outside it (grace_deadline), so a teardown step can elapse concurrently with useful work instead of after it"
    - "A select arm whose deadline is Option: the precondition gates the arm, so the unwrap_or fallback is provably never observed"
    - "One parameterised stand-in for a behaviour with two settings (silent vs chattering), rather than two scripts that drift apart"

key-files:
  created:
    - "tests/executor_lifecycle.rs"
    - "tests/fixtures/fake-claude-slow.sh"
    - "tests/fixtures/fake-claude-spawner.sh"
    - "tests/fixtures/fake-claude-deaf.sh"
  modified:
    - "src/executor/claude.rs"
    - "docs/TESTING.md"

key-decisions:
  - "The exit arm does NOT stop the loop — the loop ends on the reader's EOF, so a run's tail (including its last `result`) is never lost to a race with process exit"
  - "A cancel takes SIGTERM immediately and keeps draining under a grace clock, so the CLI's clean shutdown overlaps with its own stream drain instead of following it"
  - "The teardown is split into terminate_group + finish_teardown(grace) so the cancel path spends the grace it already spent, never a second one"
  - "The idle cap is stamped by BOTH readers, not stdout alone: a child writing diagnostics is a child that is alive"
  - "The ordering constraint (idle cap > background-wait ceiling) is asserted on the DEFAULTS by a unit test rather than enforced at runtime, because clamping it would silently rewrite a caller's deliberate configuration"
  - "EXIT_DRAIN_CAP (30s) added so the clean-exit path is bounded too — without it the D-13 promise held for hangs but not for a child that closed its stream and then hung"

requirements-completed: [TRANS-01, TRANS-02]

coverage:
  - id: D1
    description: "A grandchild spawned by the child is gone after teardown, proven by signalling its recorded pid"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_grandchild_spawned_by_the_child_is_gone_after_teardown"
        status: pass
    human_judgment: false
  - id: D2
    description: "A child that ignores the terminate signal is still killed and still reaped, and the escalation comes AFTER the grace rather than instead of it"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped"
        status: pass
        note: "#[ignore]d — runs only under `cargo test --test executor_lifecycle -- --ignored`. Verified passing in 10.01s, i.e. the grace genuinely elapsed."
    human_judgment: false
  - id: D3
    description: "A torn-down run is reaped and reports an exit status — the observable no-zombie proof"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_torn_down_run_is_reaped_and_reports_an_exit_status"
        status: pass
    human_judgment: false
  - id: D4
    description: "A run that goes silent trips the idle cap and is reported as stalled, while its wall-clock cap is nowhere near breached"
    requirement: TRANS-02
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_child_that_goes_silent_trips_the_idle_cap_and_is_reported_as_stalled"
        status: pass
    human_judgment: false
  - id: D5
    description: "A run emitting continuously past its own idle cap is never killed by it and loses no events"
    requirement: TRANS-02
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap"
        status: pass
    human_judgment: false
  - id: D6
    description: "A wall-clock breach and an idle breach produce different classified outcomes"
    requirement: TRANS-02
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_run_that_outlives_the_wall_clock_cap_is_reported_as_timed_out"
        status: pass
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_child_that_goes_silent_trips_the_idle_cap_and_is_reported_as_stalled"
        status: pass
    human_judgment: false
  - id: D7
    description: "The idle cap is strictly greater than the background-subagent wait ceiling, so the ceiling always fires first"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#the_default_idle_cap_is_strictly_greater_than_the_background_wait_ceiling"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#the_default_wall_clock_cap_is_the_looser_of_the_two_caps"
        status: pass
    human_judgment: false
  - id: D8
    description: "MUST NOT leave an orphaned process tree behind when a run ends, times out, or is stopped"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_grandchild_spawned_by_the_child_is_gone_after_teardown"
        status: pass
      - kind: other
        ref: "! grep -q '\\.kill()\\.await' src/executor/claude.rs — the combined convenience method is never the teardown story"
        status: pass
    human_judgment: true
    rationale: "The grandchild test proves the group teardown reaches one level of nesting on the cancel path. It does not prove it for an arbitrarily deep tree, nor for the crash path — a hard TUI crash mid-run leaves the group with no on-disk record to reap from, which this phase's threat register accepts and names as Phase 17's (T-15-35). The KillOnDrop backstop covers panics and early returns within the process."

metrics:
  duration: "~50 min"
  completed: "2026-07-29"
  tasks: 2
  commits: 2
  files_created: 4
  files_modified: 2
---

# Phase 15 Plan 04: Process Lifecycle and Deadlines Summary

**A run is now bounded and stoppable at the executor level: process exit is never awaited
unbounded on any path, two independent caps race it — one measuring elapsed time and one
measuring silence — and teardown takes down and reaps the whole process group, proven against a
grandchild that outlives its parent and against a child that ignores the terminate signal
outright.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 2 (both auto)
- **Files created:** 4
- **Files modified:** 2
- **Tests added:** 8 (2 unit, 6 integration of which 1 ignored) — suite went 348 → 355 + 1 ignored

## Task Commits

1. **Task 1: supervisor with two independent deadlines (D-13)** — `843c8e6` (feat)
2. **Task 2: process-group teardown and the lifecycle tests (D-14, D-24)** — `f27dda0` (feat)

## The bug the ignored test found

This is the substantive finding of the plan, and it was invisible to every fast test.

**Task 1 shipped a cancel path that could never escalate.** The cancel arm sent SIGTERM and then
deliberately kept looping, so that the CLI's clean shutdown — turn abort, Bash-tree teardown,
`SessionEnd` hooks — would overlap with the drain of its remaining stream rather than being cut
off by it. That is the right instinct and it is what SIGTERM-first is *for*. But nothing bounded
the drain. A child that honours SIGTERM closes stdout, the reader hits EOF, the loop ends, and
the teardown proceeds — so all five fast tests passed. A child that **ignores** SIGTERM closes
nothing, and the loop drained it forever. The escalation to the uncatchable signal never ran.

The deaf-child test caught it exactly: the run did not fail, it sat for **310 seconds** and came
back `Stalled { idle_for: 300s }`. The idle cap — a backstop for a different failure entirely —
was the only thing that ended it. That is a teardown that does not tear anything down, wearing a
plausible-looking outcome.

The fix makes the grace a first-class part of the loop rather than something that happens after
it. A cancel now starts a `grace_deadline`, and a select arm races it against the drain: the
stream closing first is the clean path, the grace expiring first stops the drain and hands over
to the escalation. Because the grace has already been spent by then, the teardown was split —
`tear_down_group` (steps 1-4, for paths that have signalled nothing yet) versus
`finish_teardown(grace)` (steps 2-4, taking whatever grace is left) — so a cancel never pays two
grace periods. Post-fix the deaf child is killed and reaped in **10.01 seconds**, which is the
grace to three significant figures and therefore also evidence that the escalation fires
immediately after it rather than at some later timeout.

**The generalisable lesson:** a teardown step that runs concurrently with useful work needs its
own deadline *in the same select* as that work. Putting it after the loop means it only runs if
the loop ends, and the whole reason for escalation is the case where the loop does not.

## The two caps, and why only one of them is a guess

Both defaults were set in 15-02 and are unchanged here; what this plan added is the machinery
that enforces them and the doc comment that says which number is load-bearing.

| Cap | Default | Where it comes from |
|-----|---------|---------------------|
| Wall-clock | 4 hours | A frank guess. No tuning data exists. Retune freely. |
| Idle | 15 minutes | **Derived.** Must stay strictly greater than the background-subagent ceiling. |
| `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` | 10 minutes | Set explicitly, never inherited (D-14). |

The derivation is the part to preserve. The CLI waits for background subagents up to that
ceiling, so a **healthy** run can legitimately emit nothing for ten minutes. An idle cap at or
below the ceiling would therefore kill healthy runs before the ceiling ever fired. When retuning,
preserve the ordering, not the numbers — and the ordering is asserted by
`the_default_idle_cap_is_strictly_greater_than_the_background_wait_ceiling` rather than left to
review.

Deliberately **not** enforced at runtime. Clamping a caller's `idle_cap` up to exceed their
`bg_wait_ceiling_ms` would silently rewrite a deliberate configuration — and the lifecycle tests
are exactly such a caller, scaling both down together (`bg_wait_ceiling_ms: idle_ms / 2`) so they
exercise the shape the defaults describe rather than an inverted one.

For scale: the phase spike's real multi-step run took **774.6 seconds** — comfortably past the
ten-minute ceiling, comfortably inside the fifteen-minute idle cap. A cap shorter than a real run
makes the executor useless in production, and that run is the measurement that says where the
floor is.

## Three design points worth stating

### The exit arm does not stop the loop

`status = child.wait()` is one of the five arms, but when it wins it records the status and
disables itself — it does not break. The loop still ends on the **reader's** EOF. Breaking on
process exit would race the tail of the stream: stdout closes at exit, so the last lines are
already framed and in flight, and the last of them is usually the `result` envelope the entire
outcome derivation reads. An executor that breaks on exit truncates runs while reporting
whatever it happened to have collected.

### The idle clock is stamped by the reader, and by both readers

`last_line_at` is a `watch<Instant>` the reader stamps *before* forwarding the item, so a line
that is merely slow to be consumed still counts as liveness — the question the idle cap answers
is "is the child still producing?", not "is the driver keeping up?". Stderr stamps it too, which
is a small widening of the plan's letter: a child loudly retrying on stderr is alive, and
counting only stdout would tear it down as stalled.

What the clock is emphatically **not** derived from is any duration field on the terminal
envelope. `duration_api_ms` aggregates across parallel API calls and was measured *exceeding* the
wall-clock `duration_ms` in a clean run, and both fields arrive only *with* the envelope — which
is to say, after the run this was supposed to detect (Pitfall E).

### `select!` as borrow arbitration

`ChildWrapper::wait()` holds a `&mut` borrow of the child for its future's whole life, and
`signal()` needs the same object. The resolution is the race itself: when another arm wins,
`select!` drops every loser including the exit future, the borrow ends, and by the time control
reaches the statement *after* the macro the signal call is legal. Every handler body here sets a
flag and nothing else; the acting happens after the macro, where the borrow is provably gone.

One consequence worth recording for whoever touches this next: the exit future is recreated each
pass. `ProcessGroupChild::wait()` caches its exit status the instant the leader is reaped and
*then* runs the multi-pass group reap, so dropping it between those two points would leave
grandchildren unreaped (gotcha 4). That drop cannot happen here: the reap loop's ten non-blocking
passes are synchronous — not await points, so not preemptible — and its blocking fallback is only
reached when a process **of ours** in the group is still unreaped, which with a single direct
child does not occur. The final `wait()` in `finish_teardown` is additionally never raced against
anything.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A cancel started no grace clock, so the escalation could never fire**

- **Found during:** Task 2, running the ignored deaf-child test
- **Issue:** documented in full under "The bug the ignored test found" above. Task 1's cancel
  path signalled SIGTERM and drained the stream with no bound; a child that ignored the signal
  was drained until the idle cap fired 300 seconds later.
- **Fix:** a `grace_deadline` set when the cancel arrives and raced as a sixth select arm; the
  teardown split into `tear_down_group` (steps 1-4) and `finish_teardown(grace)` (steps 2-4) so
  the elapsed grace is credited rather than repeated.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** the deaf test goes from a 310-second `Stalled` to a 10.01-second `Killed`.
- **Committed in:** `f27dda0`

**2. [Rule 2 - Missing critical functionality] The clean-exit path was still unbounded**

- **Found during:** Task 1
- **Issue:** D-13 says never await exit unbounded. The two caps cover a child that hangs *while
  streaming*, but the path taken when the stream ends normally called `child.wait()` with no
  bound at all — so a child that closed stdout and then hung would have hung the supervisor with
  it. The caps do not help there: they are disabled once the stream is done.
- **Fix:** `EXIT_DRAIN_CAP` (30 seconds, matching the CLI's own documented exit-drain cap) wraps
  that wait, and expiring escalates into the same four-step teardown.
- **Files modified:** `src/executor/claude.rs`
- **Committed in:** `843c8e6`

### Interpretations recorded

**3. One paced stand-in instead of a separate "chatty" one**

The plan names a slow variant that "emits an initialisation event and then goes silent forever".
`fake-claude-slow.sh` does exactly that at `0 0 silent`, and also takes `<heartbeats> <interval>
<ending>` so the same script covers the chattering case the plan's fourth test requires. Two
scripts for one behaviour with two settings would have drifted apart; the silent case is
unchanged and is still the default reading of the name.

**4. The grandchild pid travels as a forward-compat unknown**

The plan says the spawner "prints that grandchild's pid on a line the test can read". The test
reads events, not raw lines, and `ExecutionEvent::Message` carries a parsed `StreamMessage` with
nowhere to put a pid — the raw text is preserved only for `Unknown` and `Unparseable`. So the
announcement uses a message `type` no CLI emits, which the tolerant parser carries as
`ExecutionEvent::Unknown { raw }` with its text intact. This incidentally exercises D-09's
forward-compat path with a real process rather than a string literal.

**5. Stderr stamps the idle clock too**

The plan's `key_links` name the stdout reader as the publisher. Both readers publish here; the
reasoning is in "The idle clock is stamped by the reader" above. It is strictly the more
conservative direction — it can only prevent a teardown, never cause one.

**6. `docs/TESTING.md` had a stale claim that had to go**

The file asserted "There is no dedicated `tests/fixtures/` directory" and "Do not commit fixtures
under `tests/fixtures/` — that directory does not exist". Both became false in 15-02. Corrected
rather than worked around, since the file is in this plan's allocation and leaving a document
that instructs the reader to do the opposite of what the repository does is worse than a gap.

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical functionality), 4 interpretations
recorded. **Impact on plan:** none on scope. Every file touched is in this plan's
`files_modified` allocation; `src/executor/mod.rs` was deliberately not opened, so the cap fields
and the `TimedOut`/`Stalled` variants are consumed exactly as 15-02 shaped them.

## Files Created/Modified

- `src/executor/claude.rs` — `EXIT_DRAIN_CAP`, `Breach`, the five-arm supervisor with its
  `grace_deadline`, `terminate_group`, `tear_down_group`, `finish_teardown`, `await_clean_exit`,
  the `last_line_at` watch plumbing through both readers, and two unit tests
- `tests/executor_lifecycle.rs` — 6 unix-gated tests over three stand-ins, 1 `#[ignore]`d
- `tests/fixtures/fake-claude-slow.sh` — paced stand-in, silent or chattering, mode 100755
- `tests/fixtures/fake-claude-spawner.sh` — announces a long-lived grandchild's pid, mode 100755
- `tests/fixtures/fake-claude-deaf.sh` — `trap '' TERM`, mode 100755
- `docs/TESTING.md` — the ignored-test invocation and why it matters, a table of the five
  stand-ins, why the lifecycle tests are Unix-only, and the correction described above

## Issues Encountered

Only the cancel-escalation bug, which is documented above at length because the way it was found
matters as much as the fix: it was reachable only through a test the default suite skips.

## Known Stubs

None introduced by this plan.

One **coverage gap**, named rather than papered over: the escalation half of the teardown — grace
expiry, the uncatchable signal, and the reap of a child that ignored SIGTERM — is proven **only**
by the `#[ignore]`d test. `cargo test` does not run it. A regression that breaks escalation alone
therefore passes the default gate, which is exactly what happened once already during this plan.
The grace is a private 10-second constant, so a fast variant would require making it configurable
on `ExecutionOptions` — that is in `src/executor/mod.rs`, outside this plan's allocation. Two
mitigations are in place: `docs/TESTING.md` states in bold that `-- --ignored` must be run before
tagging a release and says *why*, and this summary records it. **Recommended for Phase 17**,
which owns the user-facing kill switch built on this teardown: promote the grace to a
configurable option and de-`ignore` the test.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema at a trust boundary. Every
`mitigate` disposition in this plan's register is implemented:

| Threat | Status |
|--------|--------|
| T-15-30 (orphaned process tree after a kill) | Group SIGTERM → grace → group SIGKILL → awaited multi-pass reap; a spawned grandchild is provably gone afterwards |
| T-15-31 (unbounded wait on a hung child) | Two caps race the exit future, and the clean-exit path is bounded by `EXIT_DRAIN_CAP` — no unbounded exit await remains |
| T-15-32 (idle cap killing a healthy subagent wave) | Ceiling set explicitly at 10 min; idle cap derived to exceed it; ordering asserted by a unit test and stated in a doc comment |
| T-15-33 (inherited CLAUDE\* variables leaking) | Scrubbed at spawn before the one variable this executor sets; recorded in the module doc (carried from 15-02, doc added here) |
| T-15-34 (zombie left after teardown) | The final `wait()` is never raced; two tests assert a reported exit status after teardown |
| T-15-35 (crash orphaning a group with no on-disk record) | **Accepted**, unchanged. Durable recording and crash reconciliation are Phase 17's; `KillOnDrop` covers panics and early returns within the process |

## Verification

| Gate | Result |
|------|--------|
| Task 1 `<verify>` | PASS |
| Task 2 `<verify>` | PASS |
| `cargo build` | PASS |
| `cargo test` | PASS — 355 passed, 1 ignored, 7 suites (was 348) |
| `cargo test --test executor_lifecycle` | PASS — 5 passed, 1 ignored |
| `cargo test --test executor_lifecycle -- --ignored` | PASS — 1 passed in 10.01s |
| `cargo clippy -- -D warnings` | PASS |
| `cargo clippy --all-targets` | **exactly 5** pre-existing lints — the frozen count did not grow |

The all-targets budget was re-measured after `cargo clean -p gsd-meta-manager`, and the five are
the expected ones: `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`.
No integration-test file contributed a lint.

Every grep-shaped acceptance criterion was executed individually against
`src/executor/claude.rs`: `tokio::select!` (2), `wall_clock_cap` (8), `idle_cap` (10),
`last_line_at` (11), `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` (3), `pgid` (4), `signal(15)` (2),
`start_kill()` (5); and the negative `\.kill()\.await` (0). `test -x` passes for all three new
stand-ins and `git ls-files -s` reports mode `100755` for each.

## Next Phase Readiness

No blockers. Three notes for what builds on this:

- **Phase 17** inherits `tear_down_group` and the pgid recorded on the handle. Its user-facing
  kill switch is this sequence plus a durable record; the detached-spawn and crash-reconciliation
  half is what closes the accepted T-15-35 window. It is also the natural home for making
  `TEARDOWN_GRACE` configurable and de-`ignore`ing the escalation test.
- **Phase 20**'s run-level caps sit *above* these. These bound one `claude` invocation; step,
  wall-clock and no-progress caps across a multi-step run are a different layer, and a doc comment
  on `Coordinator::run` says so at the point of confusion.
- **`RunOutcome::TimedOut`** now has two producers with different meanings for its `after` field:
  the supervisor sets the cap it actually breached, while 15-05's derivation sets
  `Duration::ZERO` as a documented sentinel for "externally bounded, duration unknown". A consumer
  rendering that field must handle zero as "unknown", not as "instantly".

## Self-Check: PASSED

All six files present (`src/executor/claude.rs`, `tests/executor_lifecycle.rs`, the three
stand-ins at mode `100755`, `docs/TESTING.md`); both commit hashes (`843c8e6`, `f27dda0`) resolve
in `git log` on `worktree-agent-a846aab095fc027ac`.

---
*Phase: 15-transport-foundation*
*Completed: 2026-07-29*
