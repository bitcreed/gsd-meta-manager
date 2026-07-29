---
phase: 18-driver-tab-live-watch-durable-injection
plan: 02
subsystem: infra
tags: [rust, tokio, spawn-blocking, flock, stream-json, replay-echo, correlation, ndjson]

requires:
  - phase: 15-executor-duplex-control-channel
    provides: "`TurnMessage::is_replay`, `Executor::send`, the `--replay-user-messages` argv flag, and the measured dequeue-echo timing (D-29..D-32) this plan's correlator rests on"
  - phase: 16-run-journal-state-substrate
    provides: "`JournalRun::start`/`record`, `JournalEvent`, `reader::read_all`"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`lock::acquire`, `RunLock`, `dry_run::build_report`, `execute_run`'s biased drain loop, and WR-10 itself"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "`journal::inbox`, the D-11 stdin lifetime, `Interjected { id }`, `InterjectionActedOn`/`InterjectionMissed` schema, `read_inbox` on `spawn_blocking`"
provides:
  - "`ClaudeExecutor::observing_replay_echoes` — the raw wire line of every `user` replay echo, on an unbounded channel"
  - "`PendingAcks` + `match_replay_echo` — the FIFO exact-text correlator that turns the dequeue echo into `interjection_acted_on`"
  - "`replay_echo_text` — the tolerant body reader the driver uses to interpret an echo"
  - "`journal_as_missed` — the single emission site for `interjection_missed`, reached from the post-close poll and the post-stream sweep"
  - "Four `spawn_blocking` boundaries in `src/driver/`: `build_report`, `lock::acquire`, `JournalRun::start`, and 18-01's inbox tail"
  - "`tests/fixtures/fake-claude-paced.sh` — a stand-in that holds the first turn open and lingers after stdin EOF, so the close boundary can be stood on rather than raced"
affects: [18-04, 18-09, 18-10, phase-20]

tech-stack:
  added: []
  patterns:
    - "Out-of-band observation channel on the concrete executor (`observing_spawn`'s register), carrying the RAW wire line so interpretation stays with the party D-08 makes responsible for it"
    - "A guard whose descriptor IS the resource is moved back OUT of a `spawn_blocking` closure through the join handle, never dropped inside it"
    - "One classifier, two call sites: `journal_as_missed` is reached from the live poll and from the post-stream sweep, so the two answers cannot drift"

key-files:
  created:
    - tests/fixtures/fake-claude-paced.sh
  modified:
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/executor/claude.rs
    - tests/driver_inbox.rs
    - tests/driver_lock.rs

key-decisions:
  - "The echo channel carries the **raw wire line**, not extracted text: the driver parses it, which is what D-08 actually asks for, and it keeps `src/executor/stream_json.rs` — plan 18-03's file — untouched"
  - "`observing_replay_echoes` is unbounded, so the executor's coordinator (which owns the caps, the cancel and the teardown) can never be parked by a slow consumer"
  - "The executor filters on `is_replay` before publishing; the driver re-reads the same wire field before correlating. Two independent checks of one field, because a `user` line without it is a tool result"
  - "The inbox poll arm keeps running after `close_input()` rather than being guarded off, so an undeliverable message reports `missed` within a poll interval instead of at run end"
  - "`RunLock` and `JournalRun` are moved back out of their blocking tasks; the declined alternative (release inside, re-acquire later) is the same race with a smaller name"
  - "`a_run_idle_at_the_empty_inbox_step_is_reported_stalled_not_succeeded` asserts the driver's outcome derivation rather than driving a real idle breach, because `DriveArgs` has no idle-cap knob and adding one is the new code path the task text forbids"

patterns-established:
  - "When a needed seam lives in a sibling plan's file, reach for an unowned seam that yields the same guarantee rather than editing across the fence"
  - "A test that must stand on a lifecycle boundary gets a fixture that announces the boundary (a marker file) and then lingers, instead of a sleep in the test"
  - "Mid-run journal reads use a tolerant reader; the post-mortem read stays strict, because a torn tail means different things at the two moments"

requirements-completed: []

coverage:
  - id: D1
    description: "A delivered message's `acted-on` state comes from the agent's own `isReplay` echo, correlated by exact text, and is journaled after the delivery record rather than at the moment of the stdin write"
    requirement: STEER-02
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_delivered_message_is_journaled_acted_on_when_the_replay_echo_arrives"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#an_echo_matches_the_pending_message_with_the_same_text"
        status: pass
    human_judgment: false
  - id: D2
    description: "Two messages with identical text are acked in delivery order, so the second echo can never re-ack the first message"
    requirement: STEER-02
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#two_identical_messages_are_acked_in_delivery_order"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#two_identical_texts_are_matched_in_delivery_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "Correlation is exact `String` equality — no trimming, no case folding, no normalisation — and an unmatched echo (the run's own command prompt, for instance) consumes nothing"
    requirement: STEER-02
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_non_matching_echo_leaves_the_deque_untouched"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#an_echo_against_an_empty_deque_matches_nothing"
        status: pass
    human_judgment: false
  - id: D4
    description: "A `user` envelope without the replay marker is a tool result and can never ack anything; a body shape no known version emits degrades to `None` rather than a parse failure"
    requirement: STEER-02
    verification:
      - kind: unit
        ref: "src/driver/run.rs#the_echo_text_comes_out_of_the_wire_body_only_when_the_replay_marker_is_true"
        status: pass
    human_judgment: false
  - id: D5
    description: "A message appended after the driver has closed stdin is journaled `interjection_missed` exactly once, is never retried onto the closed stdin, and gets no `interjected` record"
    requirement: STEER-02
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_appended_after_stdin_closed_is_journaled_missed_and_never_retried"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#a_message_left_in_the_inbox_when_the_stream_ends_is_journaled_as_missed"
        status: pass
    human_judgment: false
  - id: D6
    description: "A message appended while the run is still live is delivered and never recorded as missed — the final pre-close drain resolves the race in the user's favour"
    requirement: STEER-03
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_appended_before_the_final_drain_is_still_delivered"
        status: pass
    human_judgment: false
  - id: D7
    description: "Held-out durability backstop: a torn final line is neither delivered nor discarded while torn, and is delivered exactly once when completed"
    requirement: STEER-03
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_torn_final_line_is_neither_delivered_nor_lost_and_arrives_once_completed"
        status: pass
    human_judgment: false
  - id: D8
    description: "A delivered message still awaiting its echo when the run ends keeps its `interjected` record and gains no fabricated `interjection_acted_on`"
    requirement: STEER-02
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_delivered_message_awaiting_its_echo_is_never_reported_as_missed"
        status: pass
    human_judgment: false
  - id: D9
    description: "No blocking `flock`, filesystem or `git` call remains directly inside an `async fn` in `src/driver/`, and the run lock survives the boundary"
    requirement: null
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#acquiring_the_run_lock_does_not_block_the_async_runtime"
        status: pass
      - kind: other
        ref: "grep -c spawn_blocking src/driver/mod.rs src/driver/run.rs -> 1 and 5; the RunLock and JournalRun are bound from the join handle, not inside the closure"
        status: pass
    human_judgment: false
  - id: D10
    description: "A run parked at the empty-inbox step for the idle cap reaches the journal labelled `stalled`, never a success label"
    requirement: null
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_run_idle_at_the_empty_inbox_step_is_reported_stalled_not_succeeded"
        status: pass
      - kind: integration
        ref: "tests/executor_lifecycle.rs:435 (pre-existing) — the idle breach itself, against a real silent child"
        status: pass
    human_judgment: false
  - id: D11
    description: "No driver-side turn-boundary flush buffer was introduced (D-02)"
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#a_message_sent_mid_turn_is_not_buffered_by_the_driver"
        status: pass
    human_judgment: false

duration: 31min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 02: Driver Honesty & the Blocking Boundary Summary

**The driver now says `acted-on` only when the agent's own dequeue echo says so — correlated FIFO by exact text, from parsed envelopes rather than a rendered projection — every injected message reaches a named terminal state on disk, and the four blocking sections that could park the runtime run on the blocking pool with the run lock surviving the boundary.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-07-29T23:19:00Z
- **Completed:** 2026-07-29T23:50:00Z
- **Tasks:** 3
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments

- **The third state is real, not a third label.** `PendingAcks` holds
  `(id, text as sent)` for every message whose `Executor::send` returned `Ok`;
  the agent's `isReplay: true` echo is matched against it front-to-back on exact
  `String` equality, and the match journals `interjection_acted_on { id }`. The
  integration test asserts the record's **sequence number is greater than** the
  `interjected` record it acks — so a driver that recorded the transition at the
  moment of the stdin write, collapsing two states measured 55 seconds apart,
  fails on the one assertion that can catch it.
- **Two identical messages are acked in delivery order.** The echoes are
  byte-identical, so nothing in an echo says which message it acks; delivery
  order is the only honest answer. An implementation that matched the newest
  pending entry first would produce `[second, second]` on disk, which reads as
  "everything is fine" on any per-id display.
- **A tool result can never ack anything.** A `user` envelope without the replay
  marker is a tool result, and both the executor's filter and the driver's own
  re-read of the wire field refuse it. `replay_echo_text` is tolerant end to end:
  invalid JSON, a missing body, or a content shape no known CLI emits all yield
  `None` and a logged non-event, never a parse failure that could end a run.
- **The honest fourth state reports promptly.** The inbox poll arm now keeps
  running after `close_input()`; anything it reads is journaled `missed` within a
  poll interval rather than at run end. `InterjectionMissed` is constructed in
  exactly one place, reached from both the live poll and the post-stream sweep,
  so the two answers cannot drift.
- **Four `spawn_blocking` boundaries, with the lock surviving all of them.**
  `dry_run::build_report`'s two `git` shell-outs, `lock::acquire`,
  `JournalRun::start`, and 18-01's inbox tail. `RunLock` and `JournalRun` are
  moved back **out** through the join handle — a `RunLock` dropped inside the
  closure would close the descriptor that *is* the advisory lock, silently
  admitting a second concurrent driver while this run believed it held it.
- **No new dependency, no clippy regression.** `Cargo.toml`/`Cargo.lock`
  untouched; `cargo clippy --all-targets` still reports exactly the 5 pre-existing
  lints. Test count 556 → 569.

## Task Commits

1. **Tasks 1 + 2: the replay-echo correlator and the missed classifier** —
   `4c811c4` (feat)
2. **Task 3: every blocking call in the driver behind `spawn_blocking`** —
   `66c34ae` (fix)

## Files Created/Modified

**Created**
- `tests/fixtures/fake-claude-paced.sh` — `fake-claude-turns.sh` with two
  windows added and nothing else changed: the first turn's `result` is held back
  (so a test can act before the first close), and stdin EOF creates
  `<stdin-log>.eof` before a linger (so a test can act after it).

**Modified**
- `src/driver/run.rs` — `REPLAY_MARKER`, `PendingAcks`, `match_replay_echo`,
  `replay_echo_text`, `correlate_replay_echo`, `journal_as_missed`, the echo
  `select!` arm, the post-close sweep branch on the inbox poll, the post-loop
  echo drain, and the `spawn_blocking` wraps around `lock::acquire` and
  `JournalRun::start`.
- `src/driver/mod.rs` — `dry_run::build_report` on the blocking pool.
- `src/executor/claude.rs` — `ClaudeExecutor::replay_observer`,
  `observing_replay_echoes`, the `Coordinator` field, and the publish in
  `handle_item`'s turn-message arm.
- `tests/driver_inbox.rs` — five new tests plus `config_for`, `drive_args_with`,
  `eof_marker_of`, `wait_until`, `torn_halves`, `append_raw`,
  `delivered_ids_now`, `ids_of_kind`.
- `tests/driver_lock.rs` — `acquiring_the_run_lock_does_not_block_the_async_runtime`.

## Decisions Made

- **The echo channel carries the raw wire line.** See deviation 1 for why a
  channel exists at all. Given one, carrying the *raw* line rather than extracted
  text is the choice D-08 argues for directly: the driver is the party that must
  interpret the protocol, and handing it bytes rather than a projection is what
  keeps that true. It also means 18-03's `TurnMessage::text_content` — a
  *rendering* concern — and this plan's correlation are independent, which they
  should be.
- **Unbounded, deliberately.** A bounded echo channel would let a slow consumer
  park the executor's coordinator, which owns the caps, the cancel and the
  teardown. That is the failure `forward`'s deadline exists to prevent on the
  event channel, and it would be worse here because the coordinator has no
  equivalent escape. Depth is bounded in practice by how many user messages one
  run sends.
- **The correlation is driven by the echo channel alone, not by the
  `ExecutionEvent::Message` stream.** The event channel is bounded and `forward`
  *drops* events when a consumer stalls; correlating off both would let a dropped
  event desynchronise the FIFO and mis-ack a message. One source, no drift.
- **The inbox poll arm stays live after the close** rather than remaining guarded
  on `stdin_open`. The post-stream sweep alone would have satisfied every
  assertion — but only after the agent finished, which for a long turn is exactly
  the interval in which a `queued` message is indistinguishable from a slow
  agent.
- **`drain_undelivered` journals nothing.** A message delivered but never echoed
  keeps its `interjected` record and gains no transition. Fabricating one would
  assert an observation the driver never made, which is the lie the whole
  three-state display exists to prevent. It returns ids only so the *count* can
  be logged.
- **The dry-run wrap's `JoinError` re-runs inline** rather than inventing a
  `DriveError` variant. That is `ClaudeExecutor::capture_snapshot`'s established
  answer in this tree, and the branch is reachable only if `build_report`
  panicked (it does not — `git_ops` reports a failed shell-out as data) or the
  runtime is already shutting down. The token is **cloned** into the closure so
  the fallback needs no second `DrivableProject::from_registry` call site, which
  `tests/spawn_seam_guard.rs` checks mechanically.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The echoed text is not reachable from `ExecutionEvent`, and the file that would fix it belongs to a parallel sibling**

- **Found during:** Task 1
- **Issue:** The task directs the driver to *"watch the driver's own
  `handle.events` stream for `ExecutionEvent::Message(m)` … and extract the
  message text"*. `TurnMessage` does not model the message body — the type's own
  doc calls content blocks *"Phase 18's rendering concern"* — and
  `ExecutionEvent::Message(Box<StreamMessage>)` drops the raw line that
  `Envelope::Parsed` carried. So **no text is reachable from the driver today**,
  and the correlator the plan specifies cannot be written as written. The fix the
  plan implies is a body model on `TurnMessage`, which is plan **18-03**'s named
  artifact (`MessageBody`, `ContentBlock`, `TurnMessage::text_content`) in a file
  this plan may not touch — 18-03 is running in a sibling worktree right now, and
  the same edit from both would conflict at merge or be silently clobbered.
  Adding an `ExecutionEvent` variant is no better: `journal::from_exec_event`
  matches it exhaustively and lives in 18-03's other file.
- **Fix:** Added an out-of-band observation channel on `ClaudeExecutor`, in the
  exact register of the existing `observing_spawn`:
  `observing_replay_echoes(tx)` publishes the **raw wire line** of every `user`
  envelope whose `is_replay` is true. Both files it touches
  (`src/executor/claude.rs`, and nothing in `src/executor/mod.rs`) are owned by
  no wave-2 sibling, so there is no fence to cross and no hunk to clobber. The
  driver parses the line itself with `replay_echo_text`, which is strictly *more*
  faithful to D-08 than reading a pre-extracted string would have been.
- **Files modified:** `src/executor/claude.rs`, `src/driver/run.rs`
- **Verification:** `rtk proxy cargo test --test driver_inbox` (7 passed),
  `--test executor_transport` (8 passed, including the D-02 no-buffering guard),
  full suite green.
- **Committed in:** `4c811c4`

**2. [Deliberate] Tasks 1 and 2 landed in one commit**

- **Found during:** Task 2
- **Issue:** The plan asks for a commit per task. Task 2's changes are not
  separable from Task 1's at hunk granularity: the `select!` loop tail contains
  Task 1's echo arm, Task 2's post-close sweep branch and Task 1's post-loop
  drain in one contiguous edit, and the two tasks' unit tests share a block. A
  mechanical split would have produced an intermediate commit that either did not
  compile or shipped a poll-arm branch with no classifier behind it.
- **Fix:** One `feat(18-02)` commit whose message names both tasks and separates
  their changes explicitly. Task 3 is its own `fix(18-02)` commit as planned.
- **Impact:** Reviewability, not correctness. Both tasks' acceptance criteria are
  verified independently below.

**3. [Rule 2 - Missing Critical] `a_run_idle_at_the_empty_inbox_step_is_reported_stalled_not_succeeded` is a unit test, not an integration test**

- **Found during:** Task 2
- **Issue:** The task asks for this in `tests/driver_inbox.rs` *"with a shortened
  `idle_cap` so the test is fast"* — and in the same breath asks that it *"assert
  it with the existing outcome derivation rather than adding a new code path"*.
  Those two cannot both hold: `execute_run` builds `ExecutionOptions::default()`
  internally and `DriveArgs` exposes no idle-cap knob, so a driver-level test
  would require adding exactly the new code path the second instruction forbids —
  or waiting out the real fifteen-minute cap.
- **Fix:** The test lives in `src/driver/run.rs` and asserts the derivation the
  driver actually owns: `outcome_label(Stalled)` is `"stalled"` and is not any
  success label, which is the only thing the driver contributes to this outcome.
  The breach itself — an idle cap firing against a real silent child and producing
  `RunOutcome::Stalled` — is already proved at `tests/executor_lifecycle.rs:435`.
  The test's doc states both halves, including what it does not prove.
- **Files modified:** `src/driver/run.rs`
- **Committed in:** `4c811c4`

**4. [Rule 3 - Blocking] A new paced fixture was required to stand on the close boundary**

- **Found during:** Task 2
- **Issue:** Three of Task 2's tests are about what happens immediately before
  and immediately after `close_input()`. Against `fake-claude-turns.sh` that
  window is a few milliseconds wide — the stand-in answers each turn instantly,
  so the driver closes stdin at the first boundary and the child exits — and
  every one of those tests would have been a sleep racing a lifecycle event. The
  first attempt at the torn-line test failed for exactly this reason: it waited
  on the agent's stdin log, which the stand-in writes when it *consumes* a line,
  one turn later than the driver writes it.
- **Fix:** Added `tests/fixtures/fake-claude-paced.sh`, which holds the **first**
  turn's `result` back and, at stdin EOF, creates `<stdin-log>.eof` before
  lingering. The marker makes "the driver has closed stdin" an observed fact
  rather than an elapsed duration. `fake-claude-turns.sh` was left untouched so
  tests that need no windows do not pay for them — the same reasoning 18-01
  applied to `fake-claude-echo.sh`. The torn-line test now waits on the driver's
  own `interjected` record, read through a tolerant mid-run reader.
- **Files modified:** `tests/fixtures/fake-claude-paced.sh` (new),
  `tests/driver_inbox.rs`
- **Committed in:** `4c811c4`

**5. [Rule 1 - Bug] `MISSED_AFTER_CLOSE_REASON` kept its landed name**

- **Found during:** Task 2
- **Issue:** The plan's artifact list names the constant
  `MISSED_AFTER_CLOSE_REASON`; 18-01 landed it as `MISSED_AFTER_CLOSE` and its
  own unit test asserts against that identifier.
- **Fix:** Kept `MISSED_AFTER_CLOSE`. A rename would have churned a passing test
  in a sibling's committed work to no behavioural end; the constant's value,
  position and single-emission-site property are what the acceptance criteria
  actually check.
- **Impact:** None beyond the identifier.

---

**Total deviations:** 5 (1 blocking-with-design-consequence, 1 missing-critical,
2 blocking-mechanical, 1 deliberate process). Deviation 1 is the only one that
changes the shape of the delivered artifact, and it changes it in the direction
D-08 argues for.

## Issues Encountered

- **The stdin log is a consumption record, not a delivery record.** The
  stand-ins append to it when they *read* a line, which under a paced fixture is
  a full turn after the driver wrote it. The first torn-line test asserted on it
  and failed 6/7 — the fragment's completion arrived after the driver had already
  closed stdin. Waiting on the driver's own journal record fixed it, and the
  distinction is worth remembering for every later test in this phase.
- **`journal_records` is strict about torn tails, which is right for a
  post-mortem and wrong mid-run.** A journal read while its writer is appending
  may legitimately end in a partial line. `delivered_ids_now` is the tolerant
  sibling, and both docs name the moment they are for.
- **The `spawn_seam_guard` almost caught a second `from_registry` call site.**
  The first draft of the dry-run fallback rebuilt the `DrivableProject` rather
  than cloning it, which would have broken the single-call-site property that
  test exists to hold. Cloning the token was both cheaper and correct.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **569 passed, 0 failed** (baseline after wave 1: 556; +13) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `grep -c spawn_blocking src/driver/{mod,run}.rs` | 1 and 5 (criteria: ≥1, ≥3) |
| `grep -n InterjectionMissed src/driver/run.rs` | exactly one match, at the emission site |
| `grep -n is_replay src/driver/run.rs` | 3 matches (see note below) |

Every count above was taken through `rtk proxy`, because plain `cargo` output is
filtered by the `rtk` summarising wrapper and a criterion that greps for
`warning:` or `test result:` without it passes **vacuously**.

Note on the `is_replay` criterion: the three matches in `src/driver/run.rs` are
in doc comments, because the driver reads the field by its **wire** spelling
(`isReplay`, via `REPLAY_MARKER`) rather than through the parsed struct — a
consequence of deviation 1. The criterion's intent, *"the driver now reads the
field that Phase 15 modelled but never interpreted"*, is met: the driver both
reads it and acts on it, and `the_echo_text_comes_out_of_the_wire_body_only_when_the_replay_marker_is_true`
pins the behaviour rather than the spelling.

## Known Stubs

None. Every artifact this plan declares has a producer and a test.

Carried forward from 18-01 and now **resolved**: `JournalEvent::InterjectionActedOn`
was schema-only after wave 1; this plan emits it from
`correlate_replay_echo`, and `tests/driver_inbox.rs` asserts the record on disk.

## Threat Flags

None. The plan's `<threat_model>` rows are all addressed:

| Threat | Disposition | Where |
|---|---|---|
| T-18-08 (forged echo) | mitigated | ids come from `PendingAcks`, never from the envelope; an echo can at worst ack a message the user really sent |
| T-18-09 (no terminal record) | mitigated | `journal_as_missed` is the single site, reached from both the live poll and the post-stream sweep |
| T-18-10 (parked runtime) | mitigated | four `spawn_blocking` boundaries plus `acquiring_the_run_lock_does_not_block_the_async_runtime` |
| T-18-11 (lock released early) | mitigated | `RunLock` moved back out through the join handle; asserted structurally at the call site and by the four pre-existing lock tests |
| T-18-12 (echoed text in logs) | mitigated | every new log line carries a kind or a count; no message body reaches `tracing` |
| T-18-13 (package installs) | accepted | zero Cargo dependencies added |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready.** The journal now carries all four states for an injected message, and
every one of them is reconstructible from disk alone:

- **18-04 / 18-09 / 18-10** can render `queued` (an id in `inbox.jsonl` with no
  journal record), `delivered` (`interjected`), `acted-on`
  (`interjection_acted_on`) and `missed` (`interjection_missed`). The reason
  string on the last is `MISSED_AFTER_CLOSE` — machine-readable, and the
  human-readable gloss is the render layer's to write.
- **18-03** is unaffected by this plan: `TurnMessage::text_content` remains
  entirely its own, and nothing here depends on it. If 18-03 lands a body model,
  a later plan may collapse `replay_echo_text` onto it; that is a simplification,
  not a fix, and neither blocks the other.
- **Phase 20** inherits a driver in which no blocking call sits inside an
  `async fn`, which is what makes the run bounds and the quota park it adds
  enforceable rather than aspirational.

**Carried obligations**

- The negative carry-forward still holds and this plan still adds nothing to it:
  **every new per-alias or per-run map must be pruned in
  `App::prune_driver_maps`**. `PendingAcks` lives in the driver process's own run
  state, not on `AppContext`.
- `requirements-completed` is deliberately empty. STEER-02's mechanism is
  complete and proved here, but the requirement closes when a user can see the
  states — which is 18-09's and 18-07's.

## Self-Check: PASSED

`tests/fixtures/fake-claude-paced.sh` present on disk and executable; both
commits (`4c811c4`, `66c34ae`) present in `git log`; no deletions in either task
commit (`git diff --diff-filter=D HEAD~1 HEAD` empty for both); working tree
clean apart from this file.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
