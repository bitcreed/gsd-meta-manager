---
phase: 18-driver-tab-live-watch-durable-injection
plan: 01
subsystem: infra
tags: [rust, tokio, ndjson, serde_json, path-traversal, stdin, fsync, spawn_blocking]

requires:
  - phase: 15-executor-duplex-control-channel
    provides: "`Executor::send`, `ExecutionHandle::close_input`, `UserMessage::text`, and the measured `isReplay`/`result` semantics (D-29..D-32) this plan's stdin lifetime depends on"
  - phase: 16-run-journal-state-substrate
    provides: "`JournalEvent`, `JournalRun`, `reader::tail_lines`/`TailCursor`, `run_paths`, `RUNS_GITIGNORE_BODY`"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`driver::drive`, `execute_run`'s biased drain loop, `lock::acquire`, `reconcile_one`, and the WR-02 reproduction this plan closes"
provides:
  - "`journal::inbox` — the durable TUI→driver message channel (`append` + `sync_data`, `tail` delegating to `reader::tail_lines`)"
  - "`RunPaths.inbox` and a fallible `journal::run_paths -> Option<RunPaths>`"
  - "`journal::is_plain_run_id` — the single-plain-component predicate both WR-02 directions use"
  - "Journal schema: `Interjected { id, text, delivered }`, `InterjectionActedOn`, `InterjectionMissed`"
  - "The D-11 stdin lifetime: stdin closes at a turn boundary with an empty inbox, not at spawn"
  - "`DriveError::RunIdInvalid` — the loud CLI-seam refusal"
  - "`tests/fixtures/fake-claude-turns.sh` — a multi-turn stand-in emitting one `result` per turn"
affects: [18-02, 18-03, 18-04, 18-05, 18-07, 18-10, phase-20]

tech-stack:
  added: []
  patterns:
    - "Fallible path helper (`-> Option<RunPaths>`) as a compiler-enforced validation seam"
    - "Per-run append-only inbox with an fsync durability boundary, tailed from a byte cursor"
    - "Blocking filesystem work inside the driver's select loop goes through `tokio::task::spawn_blocking` (D-28)"

key-files:
  created:
    - src/journal/inbox.rs
    - tests/driver_inbox.rs
    - tests/journal_run_paths.rs
    - tests/fixtures/fake-claude-turns.sh
  modified:
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/driver/reconcile.rs
    - src/error.rs
    - src/cli.rs
    - src/app.rs

key-decisions:
  - "`run_paths` returns `Option<RunPaths>` with no infallible variant kept alongside — keeping one is how the next caller escapes validation (D-27 `promote`)"
  - "`is_plain_run_id` compares the resolved component back against the original string, so `./x` and `x/` are refused rather than normalised"
  - "The inbox append pays `sync_data()` and the journal writer deliberately does not: STEER-03's criterion is survival of the writing process, which the journal's rationale explicitly does not claim"
  - "The inbox tail delegates to `reader::tail_lines` rather than growing a second copy of five measured edge cases (D-04)"
  - "Injected text is a Rust `String` end to end; the length cap counts `char`s, never bytes"
  - "`InterjectionMissed` is emitted in this plan by a post-stream sweep, not deferred to 18-02, so no queued message is ever silently abandoned (D-10)"
  - "A new `fake-claude-turns.sh` fixture rather than editing `fake-claude-echo.sh`, whose exact event sequence the Phase 15 transport tests pin"
  - "The end-to-end test drives `drive()` in-process and asserts the agent's exit code from the `exec_finished` record, so the proof is an ordering assertion and never a sleep"

patterns-established:
  - "Compiler-enumerated validation: make the shared helper fallible instead of adding a validator callers may forget"
  - "Durability divergence is documented at the point of divergence, naming the sibling it differs from and why"
  - "Regression tests for a path-traversal fix walk the whole sandbox tree before/after rather than checking named paths, and carry a positive control so the fix cannot pass as a broken feature"

requirements-completed: []

coverage:
  - id: D1
    description: "A JSON line appended and fsynced to `inbox.jsonl` with no reader process in existence is later read by the running driver, written to the agent's stdin in the pinned `send` wire shape, and journaled as an `interjected` record carrying the same client-generated id"
    requirement: STEER-03
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled"
        status: pass
    human_judgment: false
  - id: D2
    description: "The driver no longer closes the agent's stdin after spawn; it closes at a `TurnCompleted` boundary after a final inbox drain finds nothing, and the agent still exits 0"
    requirement: STEER-01
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_run_with_an_empty_inbox_closes_stdin_at_the_first_turn_boundary_and_exits_zero"
        status: pass
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled"
        status: pass
    human_judgment: false
  - id: D3
    description: "A `--run-id` that is not exactly one `Component::Normal` creates no file anywhere and exits non-zero"
    verification:
      - kind: integration
        ref: "tests/journal_run_paths.rs#a_traversing_run_id_creates_nothing_outside_the_runs_root_and_exits_non_zero"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#only_a_single_plain_component_is_accepted_as_a_run_id"
        status: pass
    human_judgment: false
  - id: D4
    description: "An `active` file naming a traversing id causes zero reads outside the runs root, while a plain id is still followed"
    verification:
      - kind: integration
        ref: "tests/journal_run_paths.rs#an_active_pointer_naming_a_traversing_id_causes_no_read_outside_the_runs_root"
        status: pass
    human_judgment: false
  - id: D5
    description: "`run_paths` returns `Option<RunPaths>`, so every caller is enumerated by the compiler"
    verification:
      - kind: other
        ref: "cargo build && cargo clippy -- -D warnings (five callers: JournalRun::start via create_run_dir, read_active_run, reconcile_one, schedule_journal_tail, the inbox path)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Injected text is carried as a UTF-8 `String` end to end and round-trips byte-identically; any length cap is applied in `char` counts and never panics on a multibyte boundary"
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled (INJECTED carries U+1F680 and CJK)"
        status: pass
      - kind: unit
        ref: "src/journal/inbox.rs#the_length_cap_counts_chars_and_never_splits_a_scalar"
        status: pass
      - kind: unit
        ref: "src/journal/inbox.rs#a_multibyte_message_round_trips_byte_identically"
        status: pass
    human_judgment: false
  - id: D7
    description: "A message appended before the final pre-close drain is delivered; a message appended after the close is journaled as `missed`, never left in `queued`"
    verification:
      - kind: integration
        ref: "tests/driver_inbox.rs#a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled (delivered side; asserts no `interjection_missed`)"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#a_message_left_in_the_inbox_when_the_stream_ends_is_journaled_as_missed"
        status: pass
    human_judgment: false
  - id: D8
    description: "Held-out durability check (backstop): with `inbox.jsonl` ending in a torn line, the driver delivers every complete line exactly once, neither delivers nor discards the fragment, and delivers it once completed"
    verification:
      - kind: unit
        ref: "src/journal/inbox.rs#a_torn_line_is_neither_delivered_nor_discarded_and_arrives_once_completed"
        status: pass
    human_judgment: false
  - id: D9
    description: "No driver-side flush buffer exists anywhere (D-02)"
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#a_message_sent_mid_turn_is_not_buffered_by_the_driver"
        status: pass
    human_judgment: false

duration: 52min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 01: Durable Injection Spine & WR-02 Summary

**A human-authored message fsynced to `inbox.jsonl` before any reader process exists now reaches a live agent's stdin and lands in `journal.jsonl` with its correlation id — and both reproduced path-traversal directions are closed by a fallible `run_paths` that conscripts the compiler into finding every caller.**

## Performance

- **Duration:** 52 min
- **Started:** 2026-07-29T20:55:00Z
- **Completed:** 2026-07-29T21:47:00Z
- **Tasks:** 2
- **Files modified:** 17 (4 created, 13 modified)

## Accomplishments

- **The spine works end to end.** A line appended and fsynced to
  `.planning/meta-manager/runs/<run-id>/inbox.jsonl`, with the driver process not
  yet in existence, is tailed by the running driver, written to the agent's stdin
  in the exact `Executor::send` wire shape, and recorded as an `interjected`
  record carrying the same client-generated id. The proof is an **ordering**
  assertion — the journal file does not exist when the append is made — never a
  sleep, because the criterion under test is durability.
- **The stdin lifetime change that made steering possible at all (D-11).**
  `close_input()` no longer runs after spawn. While that line stood,
  STEER-01/02/03 were not merely unimplemented but physically impossible: the
  writer task breaks its loop on `Close` and every later `send` returns
  `WriterGone`. It now fires at a `TurnCompleted` boundary after a final inbox
  drain finds nothing, so N human-steered turns work for free and the run still
  terminates naturally with the agent exiting 0.
- **Both reproduced WR-02 directions closed (D-27).** `run_paths` is fallible,
  `is_plain_run_id` refuses anything that is not a single `Component::Normal`,
  `driver::drive` refuses a hostile `--run-id` loudly and non-zero via
  `DriveError::RunIdInvalid`, and `writer::read_active_run` checks the component
  **before** its `is_dir()` call — the half that matters more, because the
  `active` file lives inside the driven project and the agent writes it.
- **No message is silently abandoned (D-10).** A post-stream sweep journals
  anything still queued as `interjection_missed` with its id, so every message
  reaches a named terminal state rather than sitting in `queued` forever.
- **No new dependency, no clippy regression.** `Cargo.toml`/`Cargo.lock`
  untouched; `cargo clippy --all-targets` still reports exactly the 5
  pre-existing lints. Test count 542 → 556.

## Task Commits

1. **Task 1: End-to-end "a queued message reaches the agent"** — `412a913` (feat)
2. **Task 2: Close both directions of WR-02 and refuse a hostile run id at the CLI seam** — `63353ff` (fix)

## Files Created/Modified

**Created**
- `src/journal/inbox.rs` — the inbox module: `INBOX_FILE`, `InboxMessage`,
  `new_message_id`, `cap_chars`, `append` (one `write_all` + `sync_data`), `tail`
  (delegating to `reader::tail_lines`), `InboxRead` carrying both diagnostic
  flags plus an unparseable count.
- `tests/driver_inbox.rs` — the end-to-end durability test and the D-11
  empty-inbox termination regression.
- `tests/journal_run_paths.rs` — both WR-02 regression directions, with a
  whole-tree footprint comparison and a positive control.
- `tests/fixtures/fake-claude-turns.sh` — multi-turn stdin-reactive stand-in
  emitting one `result` per turn.

**Modified**
- `src/journal/mod.rs` — `is_plain_run_id`, `run_paths -> Option<RunPaths>`,
  `RunPaths.inbox`, `Interjected.id`, `InterjectionActedOn`,
  `InterjectionMissed`, `EMITTED_KINDS` +3, `RESERVED_KINDS` −1, `is_content`
  rationale.
- `src/journal/writer.rs` — `create_run_dir` refuses a non-plain id before
  creating anything; `read_active_run` gains the component check ahead of
  `is_dir()`.
- `src/driver/run.rs` — `INBOX_POLL_INTERVAL`, `read_inbox`,
  `deliver_pending_inbox`, `sweep_inbox_as_missed`, the relocated `close_input`,
  and the drain-loop inbox arm (last in the `biased` select).
- `src/driver/mod.rs` — the `RunIdInvalid` refusal at the `drive` seam.
- `src/driver/reconcile.rs` — the fallible `run_paths` threads through as one `?`.
- `src/error.rs` — `DriveError::RunIdInvalid { run_id }` plus `Display` and
  `source` arms.
- `src/cli.rs` — a comment recording why the shape check lives in `drive` rather
  than in a per-flag `value_parser`.
- `src/app.rs` — `schedule_journal_tail` returns early and logs by kind for a
  non-plain run id.
- `tests/driver_{kill,kill_startup,reattach,tracer}.rs` — updated for the
  `run_paths` signature (compiler-enumerated).

## Decisions Made

- **`promote`, not `add-alongside`, for `run_paths`.** No infallible variant is
  kept: keeping one is exactly how the next caller escapes validation. The
  reproduction in `17-REVIEW.md` showed a validation helper that merely *exists*
  gets called at three of five sites.
- **`is_plain_run_id` compares the component back against the original string.**
  `Path::components()` silently normalises `./x` and `x/`, so a check that only
  matched on `Component::Normal` would accept ids whose written form is not the
  name they resolve to. It also refuses to normalise or canonicalise: that reads
  the filesystem, follows symlinks the agent may have planted, and answers
  differently for a path that does not exist yet — which is every new run.
- **`sync_data()` in `inbox::append` and nowhere else.** The doc comment names
  the sibling it diverges from (`journal/writer.rs`) and why: the journal's
  measured rationale claims survival of *process* death, which a returned
  `write(2)` already provides; STEER-03 asks for survival across the death of the
  writing process observed by a process that did not yet exist, which it does not.
- **`InterjectionMissed` is emitted here rather than deferred.** See deviation 2.
- **The end-to-end test calls `drive()` in-process.** Spawning the binary would
  require interpreting an exit status; calling the run body directly lets the
  test assert on the returned `Result` and read the agent's own exit code out of
  the `exec_finished` record, which is what the must-have actually names.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] A new multi-turn fixture was required for the D-11 loop**
- **Found during:** Task 1
- **Issue:** The plan directs the end-to-end test at
  `tests/fixtures/fake-claude-echo.sh`, which emits its only terminal `result`
  **after stdin EOF**. That is correct while the driver closes stdin at spawn and
  fatal afterwards: under D-11 the driver waits for a `result` before closing
  stdin, and that stand-in waits for the close before producing one. The two
  deadlock. Editing `fake-claude-echo.sh` in place was rejected —
  `tests/executor_transport.rs` pins its exact event sequence, including the D-02
  no-buffering guard this plan must not disturb.
- **Fix:** Added `tests/fixtures/fake-claude-turns.sh`, identical to its sibling
  except that it emits one `result` per turn as the real CLI does. Its header doc
  records the single behavioural difference and why the sibling was left alone.
- **Files modified:** `tests/fixtures/fake-claude-turns.sh` (new),
  `tests/driver_inbox.rs`
- **Verification:** `rtk proxy cargo test --test driver_inbox` (2 passed);
  `rtk proxy cargo test --test executor_transport` still passes (8 passed),
  including the D-02 guard.
- **Committed in:** `412a913`

**2. [Rule 2 - Missing Critical] `InterjectionMissed` is emitted in this plan, not deferred**
- **Found during:** Task 1
- **Issue:** The task action says this plan emits only `Interjected` and leaves
  the two transitions to 18-02. But this plan's own `must_haves.truths` requires
  that *"a message appended after `close_input()` is journaled and rendered as
  `missed`, never left in `queued` indefinitely"*, and one of its three
  prohibitions is that *"an injected message must never be silently
  abandoned"*. Shipping the stdin close without the sweep would leave a real
  window in which a queued message has no terminal state at all — and the window
  is not hypothetical, because the drain loop stops polling the moment stdin is
  closed.
- **Fix:** Added `sweep_inbox_as_missed`, run once after the event stream ends,
  which journals every remaining message as `interjection_missed { id, reason }`.
  It is deliberately **not** a retry. `InterjectionActedOn` remains schema-only
  and stays 18-02's, as the task action specifies — that one needs the
  `isReplay` correlation state machine, which this plan does not build.
- **Files modified:** `src/driver/run.rs`, `src/journal/mod.rs`
- **Verification:**
  `src/driver/run.rs#a_message_left_in_the_inbox_when_the_stream_ends_is_journaled_as_missed`
  asserts the specific id comes back, not merely a count.
- **Committed in:** `412a913`

**3. [Rule 3 - Blocking] `reconcile.rs` and `app.rs` were fixed properly in Task 1**
- **Found during:** Task 1
- **Issue:** The plan assigns those two callers to Task 2 and asks Task 1 for
  "only the minimal edits needed to compile". A minimal edit here would have been
  an `.expect()` or an `unwrap_or_default()` — i.e. deliberately writing the
  panic Task 2 would then have to remove, in a commit that would have shipped a
  reachable panic on an agent-controlled value.
- **Fix:** Both were given their final shape in Task 1 (`?` in `reconcile_one`, a
  logged early return in `schedule_journal_tail`). Task 2's scope is unchanged
  otherwise: the `read_active_run` check, the `DriveError::RunIdInvalid` seam
  refusal, and both regression tests.
- **Files modified:** `src/driver/reconcile.rs`, `src/app.rs`
- **Verification:** `grep -rn 'run_paths(' src/ | grep -v 'fn run_paths'` shows
  no production call site discarding the `Option` with `unwrap`/`expect`.
- **Committed in:** `412a913`

**4. [Rule 3 - Blocking] Four existing test files updated for the new signature**
- **Found during:** Task 1
- **Issue:** `tests/driver_{kill,kill_startup,reattach,tracer}.rs` call
  `journal::run_paths(...)` and field-access the result. This is the fallibility
  doing exactly its job — enumerating callers — but they are outside the plan's
  `files_modified`.
- **Fix:** Each site takes `.expect("the fixture run id is a plain path
  component")`, which is correct in a test whose run id is a hard-coded literal.
- **Files modified:** `tests/driver_kill.rs`, `tests/driver_kill_startup.rs`,
  `tests/driver_reattach.rs`, `tests/driver_tracer.rs`
- **Verification:** Full suite green.
- **Committed in:** `412a913`

**5. [Rule 2 - Missing Critical] `src/error.rs` carries the new variant**
- **Found during:** Task 2
- **Issue:** The plan's `files_modified` omits `src/error.rs`, but `DriveError`
  lives there (the plan's own acceptance criterion greps
  `src/driver/mod.rs src/error.rs` for `RunIdInvalid`, so this is a frontmatter
  omission rather than a scope change).
- **Fix:** Added the variant with its `Display` and `source` arms; `source` is
  exhaustive in this enum by a Phase 17 decision, so the compiler required the
  new arm.
- **Files modified:** `src/error.rs`
- **Committed in:** `63353ff`

---

**Total deviations:** 5 auto-fixed (2 missing critical, 3 blocking)
**Impact on plan:** No scope creep. Deviation 2 is the only one that adds
behaviour beyond the task text, and it adds it because this plan's own
`must_haves` and prohibitions require it; it takes ~15 lines and leaves 18-02's
named deliverable (`acted-on`) untouched.

## Issues Encountered

- **The `tokio::select!` borrow question.** The new inbox arm needs `&mut handle`
  inside the *event* arm's body while the event arm's own future borrowed
  `handle.events`. This works because `select!` drops the whole futures tuple
  before running any handler — the property `run.rs`'s existing comment already
  relies on for the terminate arm. No restructuring was needed.
- **Two `#[tokio::test]` runtimes each installing a SIGTERM handler** in
  `tests/driver_inbox.rs` was a theoretical concern; both tests pass reliably in
  parallel across repeated runs.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **556 passed, 0 failed** (baseline 542; 461 lib + 95 across 16 integration binaries) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |

Note: every count above was taken through `rtk proxy`, because plain `cargo`
output is filtered by the `rtk` summarising wrapper and a criterion that greps
for `warning:` or `test result:` without it passes **vacuously** (this bit Phase
15 and Phase 17).

## Known Stubs

- `JournalEvent::InterjectionActedOn` is **schema only** in this plan: the
  variant, its kind, and its `EMITTED_KINDS` entry exist, and nothing constructs
  it. This is deliberate and is plan 18-02's named deliverable — it needs the
  `isReplay` echo correlation state machine (D-08), which requires holding
  `{id → text}` for delivered messages and watching the driver's own event stream.
  It is recorded here so a reader does not mistake the declared kind for a
  producer. Nothing renders it yet either, so no UI asserts a state the mechanism
  cannot back.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready for wave 2.**

- **18-02 (driver honesty)** has everything it needs: the two transition variants
  are declared and emitted-declared, `deliver_pending_inbox` is the natural place
  to record `{id → text}` for correlation, and `MISSED_AFTER_CLOSE` is the single
  reason string the display renders. Its remaining work is the `isReplay`
  correlation and D-28/WR-10's remaining `spawn_blocking` boundaries
  (`dry_run::build_report`, `lock::acquire`, `JournalRun::start`) — this plan
  wrapped only the paths it added.
- **18-03/18-04/18-10** can read `interjected`, `interjection_acted_on` and
  `interjection_missed` off the journal; the TUI's message state is a pure
  function of ids in `inbox.jsonl` and ids in each journal kind, reconstructible
  from disk alone, which is what makes STEER-03 hold across a TUI restart with no
  extra persisted state.
- **18-05/18-07** own the TUI-side append. It must go through
  `journal::inbox::append` on `spawn_blocking` (D-06/D-28) and must derive its
  path from `run_paths(...)?.inbox`, never by joining `inbox.jsonl` itself.

**Carried obligations for later plans in this phase**

- `requirements-completed` is deliberately empty. STEER-01 is not reachable by a
  user until 18-07 gives them a screen to type into, and STEER-03's user-visible
  half likewise. The mechanism both rest on is proven here; the requirements
  close later in the phase.
- CONTEXT's one remaining carry-forward is negative: **every new per-alias or
  per-run map a later plan adds must be pruned in `App::prune_driver_maps`**.
  This plan added none — the inbox cursor lives in the driver process's own run
  state, not on `AppContext`.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
