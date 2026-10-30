---
phase: 16-run-journal-state-substrate
plan: 06
subsystem: journal / run lifecycle
tags: [obs-01, obs-06, safe-04, sigkill, redaction, run-json, phase-gate, ndjson]
status: complete

requires:
  - phase: "16-01"
    provides: "journal module, JournalEvent/RunRecord schema, RedactedLine seam, JournalWriter"
  - phase: "16-02"
    provides: "ExecutionEvent::EventsDropped — the drop report this plan journals (D-33)"
  - phase: "16-03"
    provides: "create_run_dir, write_run_record, prune_runs, active pointer, crash_test_writer_loop and its two planted constants"
  - phase: "16-04"
    provides: "reader::read_all, seq_gaps, ReadDiagnostics — the replay side this plan asserts with"
  - phase: "16-05"
    provides: "reparse_dispatches counter and its control arm — success criterion 2's evidence, traced here"
  - phase: "15"
    provides: "ExecutionEvent, ExecutionHandle, GateOutcome, RunOutcome — read off, never widened (D-37)"
provides:
  - "JournalRun — one run's whole lifecycle behind one API, with run.json written exactly twice"
  - "from_exec_event — the ExecutionEvent to JournalEvent consumer mapping (D-36)"
  - "EMITTED_KINDS / RESERVED_KINDS — the mechanical statement of what this phase writes"
  - "tests/journal_crash.rs — real SIGKILL survival and on-disk redaction via a re-exec harness"
  - "The phase boundary paragraph in src/journal/mod.rs naming Phases 17-20 as owners"
affects:
  - "Phase 17: JournalRun is the API detached spawn drives; pid/pgid and the absent ended_at are the facts its reconciliation reads"
  - "Phase 18: from_exec_event's debug projection is the seam a richer render replaces; Interjected is schema-only until then"
  - "Phase 20: RESERVED_KINDS is the migration-free slot for observed/decided/parked"

tech-stack:
  added: []
  patterns:
    - "Re-exec the integration-test binary with libtest --exact plus an env sentinel to run library code in a second process"
    - "Assert redaction against the library's own constants and its own redactor, so neither side of the comparison can be retyped out of agreement"
    - "One lifecycle type owning paths + writer + record + clock, with a counted write budget pinned by a debug assertion"

key-files:
  created:
    - tests/journal_crash.rs
  modified:
    - src/journal/mod.rs

key-decisions:
  - "record_exec/record return io::Result while start/finish return anyhow::Result: the per-event hot path wants one cheap error type, the rare lifecycle calls want context."
  - "from_exec_event is a pure per-event projection; the two run-scoped fields on exec_finished (cost_usd, duration_s) are stamped by JournalRun, which owns the clock and the cost total."
  - "TurnCompleted maps to exec_event on a turn-boundary stream label, never to exec_finished — a turn is not the run, and conflating them would misreport one multi-turn run as several."
  - "run_ended is appended BEFORE the terminal run.json write, so a crash between them leaves the journal naming an ending the record does not claim — the shape Phase 17 reconciles, not the inverse."
  - "The crash test derives its expected replacement literals by calling the library redactor on the library's planted constants, so a change to either cannot make the test vacuous."

patterns-established:
  - "Phase-boundary paragraph in a module doc: name the four later owners so the next phase does not reopen this module to answer a question it does not hold."
  - "A kinds-emitted/kinds-reserved constant pair, asserted against every mapped variant, is how a phase proves it did not start emitting a later phase's vocabulary early."

requirements-completed: [OBS-01, OBS-06, SAFE-04]

coverage:
  - id: D1
    description: "A run has one API that opens its directory, prunes, writes its record, journals every executor event it observes, and closes the record out — writing run.json exactly twice"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_run_writes_its_record_exactly_twice"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#an_executor_stream_becomes_a_journal_a_reader_can_replay"
        status: pass
    human_judgment: false
  - id: D2
    description: "Everything the journal records is read off types Phase 15 already carries; no executor signature widened and no Serialize derive added to the inbound wire model"
    requirement: OBS-01
    verification:
      - kind: other
        ref: "git diff --stat src/executor/ (empty for this plan); StreamMessage/SystemMessage/TurnMessage/ResultMessage still derive Deserialize only"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase"
        status: pass
    human_judgment: false
  - id: D3
    description: "A writer killed with a real signal leaves every completed step and the last event before death readable, with zero sequence gaps and at most one torn line, and that torn line is the last"
    requirement: OBS-01
    verification:
      - kind: integration
        ref: "tests/journal_crash.rs#a_sigkilled_writer_leaves_every_flushed_line_readable"
        status: pass
    human_judgment: false
  - id: D4
    description: "A credential and a dash-encoded home path planted by the killed child are already replaced in the surviving bytes on disk"
    requirement: SAFE-04
    verification:
      - kind: integration
        ref: "tests/journal_crash.rs#a_sigkilled_writers_surviving_bytes_are_already_redacted"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#a_planted_credential_is_already_redacted_in_the_file_bytes"
        status: pass
    human_judgment: false
  - id: D5
    description: "The dropped-event count from Phase 15's bounded drain reaches the journal as a number a reader can read back (D-33)"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_dropped_event_count_reaches_the_journal"
        status: pass
    human_judgment: false
  - id: D6
    description: "The phase's three success criteria each map to a named runnable check, the project gate passes, and the pre-existing lint count is unchanged at 5"
    verification:
      - kind: other
        ref: "cargo build && rtk proxy cargo test (427 passed) && cargo clippy -- -D warnings (exit 0); rtk proxy cargo clippy --all-targets | grep '^warning: ' | grep -v generated | wc -l == 5"
        status: pass
    human_judgment: false

metrics:
  duration: ~35 min
  completed: 2026-07-29
  tasks: 3
  tests_added: 7
  tests_total: 427
---

# Phase 16 Plan 06: Close the Phase — Run Lifecycle, Crash Proof, Gate Summary

**One `JournalRun` API owns a run start to finish with `run.json` written exactly twice, every
`ExecutionEvent` variant maps onto the journal vocabulary without widening a Phase 15 type, and a
real `SIGKILL` against a process running the real writer leaves an unbroken, already-redacted
file behind.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-07-29T15:30Z
- **Tasks:** 3
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- **`JournalRun`** — the one type a driver holds for a run. `start()` prunes first (D-32), lands
  the ignore file before any journal byte exists (D-08), writes `run.json` once, points `active`,
  opens the journal and appends `run_started`. `finish()` appends `run_ended`, stamps the record,
  performs the second and last record write, then clears the pointer. A `record_writes` counter
  and a `debug_assert_eq!` pin the total at **exactly 2** (D-06).
- **`from_exec_event`** — the D-36 consumer mapping. Every one of the ten `ExecutionEvent`
  variants projects onto a journal kind, reading only fields Phase 15 already recorded for this
  purpose (D-37). `git diff --stat src/executor/` is empty for this plan.
- **`EMITTED_KINDS` / `RESERVED_KINDS`** — the split D-36 requires this phase to *state*, made
  mechanical by a test that drives every variant through the mapping and asserts no produced kind
  is one Phase 18 or Phase 20 owns.
- **`tests/journal_crash.rs`** — RESEARCH §6.2's harness: re-exec this test binary with
  `--exact journal_child_writer`, select the child role with an env sentinel, kill it with the
  standard library's child kill (SIGKILL on unix). Six assertions, and the redaction assertion
  reads the file's bytes back.
- **The phase boundary, written down** — `src/journal/mod.rs`'s module doc now names the four
  later owners, so the next phase does not reopen this module to answer a question it never held.

## Task Commits

1. **Task 1: One run, one lifecycle, one mapping** — `70c7fcf` (feat)
2. **Task 2: Kill the writer for real** — `4584948` (test)
3. **Task 3: Phase gate, constants audit, boundary paragraph** — `53add47` (docs)

## Files Created/Modified

- `tests/journal_crash.rs` — **created.** The re-exec crash harness: `journal_child_writer` (the
  child role), `a_sigkilled_writer_leaves_every_flushed_line_readable`,
  `a_sigkilled_writers_surviving_bytes_are_already_redacted`.
- `src/journal/mod.rs` — **modified.** `JournalRun`, `from_exec_event`, `stream_label`,
  `EMITTED_KINDS`, `RESERVED_KINDS`, four tests, the untuned admission on `MAX_TAIL_BYTES`, and
  the "what this phase deliberately does not do" section.

## The Three ROADMAP Success Criteria, Mapped to Evidence

| # | Criterion | Named evidence | Command |
|---|---|---|---|
| 1 | After a run is killed mid-flight or its host process dies, the on-disk journal still shows every step it completed and the last event before death | `a_sigkilled_writer_leaves_every_flushed_line_readable` — **five distinct assertions**: the child died **by signal** (`status.code() == None`); **>100 complete lines** survived (measured ~2 150 in RESEARCH §3.2, so 100 is a generous floor); **zero** `seq` gaps, plus `seq[0] == 1` and `seq[last] == count` so nothing interior is missing; **at most one torn line and only as the last** (a ceiling — 0/8 measured, not an expectation); and every complete line **reparses into a `JournalRecord`** | `rtk proxy cargo test --test journal_crash` |
| 2 | A run appending events every few seconds for an hour leaves TUI navigation as responsive as when idle — journal writes do not trigger a full project re-parse | `journal_appends_never_trigger_a_full_reparse` (500 driver-path events, `reparse_dispatches` unchanged) **AND its control arm** `a_planning_write_still_triggers_a_reparse` (counter `+ exactly 1`). **Both are required.** The zero-arm alone also passes against a handler that dropped `FileChanged` entirely; the control arm is what proves the alias lookup resolves and the route is live. Corroborated by `the_driver_route_leaves_the_refresh_dedup_map_untouched` and `a_tail_result_advances_only_its_own_cursor`. **No wall-clock assertion was added**, following `src/main_loop.rs`'s own recorded reasoning that an untuned timing number is corroboration, never evidence (D-17, RESEARCH §7.6) | `rtk proxy cargo test --lib app::` |
| 3 | A credential or token that appears in a run's output is already redacted in the journal file on disk, not merely in the rendered view | Three layers: the **29-row corpus** (`every_corpus_case_redacts_exactly_as_specified`); the **idempotence property** (`redaction_is_idempotent_over_the_whole_corpus`); and the **byte-level assertion after the kill** (`a_sigkilled_writers_surviving_bytes_are_already_redacted`), which includes the **dash-encoded** `-home-<user>-<repo>` case as its own explicit assertion — the WR-15 shape that leaked in Phase 15 and survived two scans | `rtk proxy cargo test --test journal_crash` + `rtk proxy cargo test --lib journal::redact` |

## RESEARCH Requirements-to-Test Map — Every Row Satisfied

All eighteen rows were `❌ Wave 0` at phase start. Each now has a named satisfying test. **No row
is left blank.**

| # | Req | Behaviour | Satisfying test | Command |
|---|---|---|---|---|
| 1 | OBS-01 | SIGKILLed writer leaves every flushed line readable | `a_sigkilled_writer_leaves_every_flushed_line_readable` | `rtk proxy cargo test --test journal_crash` |
| 2 | OBS-01 | Torn final line is skipped, not fatal | `a_partial_trailing_line_is_not_consumed_and_completes_on_the_next_read` | `rtk proxy cargo test --lib journal::reader` |
| 3 | OBS-01 | `seq` gap is reported, not fatal | `a_sequence_gap_is_reported_and_the_read_still_returns_every_record` | `rtk proxy cargo test --lib journal::reader` |
| 4 | OBS-01 | Unknown `kind` survives with its fields | `an_unknown_kind_keeps_every_one_of_its_fields` | `rtk proxy cargo test --lib journal::reader` |
| 5 | OBS-01 | `run.json` written atomically, exactly twice | `a_run_record_is_written_atomically_and_is_world_readable` (atomicity, mode 0644, no litter) + `a_run_writes_its_record_exactly_twice` (**this plan** — count `== 2`, not `>= 2`) | `rtk proxy cargo test --lib journal::` |
| 6 | OBS-01 | `runs/.gitignore` tracks `run.json`, ignores the rest | `staging_a_real_repo_tracks_run_json_and_ignores_the_journal` (real `git init` + `git add -A` + `git ls-files`, not `git check-ignore`) | `rtk proxy cargo test --test journal_gitignore` |
| 7 | OBS-06 | Driver-path `FileChanged` issues **zero** re-parse dispatches | `journal_appends_never_trigger_a_full_reparse` | `rtk proxy cargo test --lib app::` |
| 8 | OBS-06 | Planning-path `FileChanged` **still** re-parses (control arm) | `a_planning_write_still_triggers_a_reparse` | `rtk proxy cargo test --lib app::` |
| 9 | OBS-06 | `classify_change` is exhaustive over path shapes | `classify_change_names_a_journal_path_as_driver`, `classify_change_names_state_md_as_planning`, `classify_change_does_not_treat_a_bare_runs_directory_as_driver`, `classify_change_returns_planning_for_a_path_outside_the_project` | `rtk proxy cargo test --lib journal::tests::classify` |
| 10 | OBS-06 | One debounce batch with journal-first still emits the planning event | `a_batch_with_the_journal_first_still_emits_the_planning_event` (+ `two_journal_paths_in_one_batch_emit_one_driver_event`, `a_driver_path_and_a_planning_path_emit_two_events_for_one_root`, `paths_outside_a_planning_directory_emit_nothing`) | `rtk proxy cargo test --lib watcher::` |
| 11 | OBS-06 | Tail cost is proportional to bytes appended, not project size | `a_tail_result_advances_only_its_own_cursor` (the cursor is the cost model: one run's offset moves, its sibling's does not) + `the_driver_route_leaves_the_refresh_dedup_map_untouched` | `rtk proxy cargo test --lib app::` |
| 12 | SAFE-04 | Planted Anthropic-shaped key is `[REDACTED:…]` **in the file bytes** | `a_sigkilled_writers_surviving_bytes_are_already_redacted` + `a_planted_credential_is_already_redacted_in_the_file_bytes` | `rtk proxy cargo test --test journal_crash` + `--lib journal::writer` |
| 13 | SAFE-04 | Planted **dash-encoded** `-home-<user>-<repo>` is redacted in the file bytes (WR-15) | same two tests — the dash case is a **separate explicit assertion** in each, never folded into a general "path" check | same |
| 14 | SAFE-04 | All §4.4 corpus cases redact as specified | `every_corpus_case_redacts_exactly_as_specified` (29 rows against a `>= 27` floor; every row asserts an **exact** output; three rows expect their input back verbatim) | `rtk proxy cargo test --lib journal::redact` |
| 15 | SAFE-04 | `redact(redact(x)) == redact(x)` over the corpus | `redaction_is_idempotent_over_the_whole_corpus` | `rtk proxy cargo test --lib journal::redact` |
| 16 | SAFE-04 | Object **keys** are redacted, not only values | `object_keys_are_redacted_and_collisions_are_disambiguated` | `rtk proxy cargo test --lib journal::redact` |
| 17 | SAFE-04 | Every written line reparses as valid JSON | `every_appended_record_is_one_complete_line_with_one_newline` + `a_payload_with_embedded_newlines_still_serialises_to_one_line`; and after a kill, by `a_sigkilled_writer_leaves_every_flushed_line_readable`'s parse of every surviving line | `rtk proxy cargo test --lib journal::` |
| 18 | SAFE-04 | No `src/journal/**` symbol accepts a bare `String` line | Compile-time. `JournalWriter::append` takes `&JournalEvent` and every byte reaches the file only through `RedactedLine::new`, whose field is private, whose accessor is `pub(crate)`, and which implements **no traits at all** — verified by 16-01's greps (`grep -c 'for RedactedLine'` → `0`, `grep -c 'pub(crate) fn as_line'` → `1`, `grep -c 'for_testing'` → `0`) | `cargo build` |

## The Four Growth Constants — Audit

Each is a named constant whose doc states its value's rationale **and** explicitly admits no
tuning data exists, in the shape `src/main_loop.rs:41-64` uses. One line quoted from each:

| Constant | Value | Quoted admission |
|---|---|---|
| `MAX_EVENT_PAYLOAD_BYTES` | 8 KiB | *"It is a defensible starting value with **no tuning data behind it**; it is a named constant so tuning is a one-line change."* |
| `MAX_RUN_JOURNAL_BYTES` | 64 MiB | *"Also untuned; see [`RETAIN_RUNS`] for the disk budget it pairs with."* |
| `RETAIN_RUNS` | 10 | *"This constant and [`MAX_RUN_JOURNAL_BYTES`] are **one budget, not two**: at the 64 MiB worst case they imply a 640 MB per-project ceiling. Tuning either without the other moves that ceiling silently. Untuned starting value."* |
| `MAX_TAIL_BYTES` | 4 MiB | *"Like the three growth constants above it, this is a defensible starting value with **no tuning data behind it** — it is a named constant so tuning is a one-line change."* (**added by this plan** — the only one that fell short) |

The per-run cap and the retention count are documented **together as one storage budget**, as
RESEARCH §10 requires: `RETAIN_RUNS`'s doc names `MAX_RUN_JOURNAL_BYTES`, states the 640 MB
product, and says tuning either without the other moves the ceiling silently.

## Phase-Wide Hygiene Checks

```
git diff --stat f1e5cda Cargo.toml Cargo.lock        (empty)   zero new dependencies across the phase
git diff --stat f1e5cda src/ui/
  src/ui/screens/mod.rs    | 23 +++         the two AppContext fields + their docs
  src/ui/screens/detail.rs |  2 ++          their initialisation in a test helper
```

**No file under `src/ui/` gained a render path for journal content (D-36).** The `detail.rs`
change is two struct-literal lines inside `mod tests`; the `mod.rs` change is the two
`AppContext` fields (`reparse_dispatches`, `journal_cursors`) and their doc comments. Phase 16
ships no UI, as CONTEXT scopes it.

## Verification — The Project Gate

```
cargo build                                                     clean
rtk proxy cargo test                                427 passed, 0 failed
  366 lib + 11 executor_lifecycle + 7 executor_transport
  + 3 journal_crash + 3 journal_gitignore + 12 registry + 25 state_reader
cargo clippy -- -D warnings                                   exit 0
rtk proxy cargo clippy --all-targets
  | grep '^warning: ' | grep -v generated | wc -l                    5
    browser.rs:131, :132, :133   project_creator.rs:146   state_reader/mod.rs:258
rtk proxy cargo test --test journal_crash              3 passed in 0.08s   (bound: 60s)
cargo doc --no-deps                          4 warnings, all pre-existing, none in src/journal/
git diff --diff-filter=D --name-only 0b105c7 HEAD             (empty)  no deletions
```

**Test totals.** Phase-start baseline **363**; base commit of this plan **420**; now **427**
(**+7** here: 4 lib + 3 integration). **Phase total: +64 tests, none removed.**

**Lint delta.** Exactly **5**, in the three pre-existing files only — the same set, same
locations, as D-38's baseline. It did not grow and it did not shrink; a shrink would have meant
this phase touched a file it had no business opening.

## Decisions Made

- **`record_exec` / `record` return `io::Result`, `start` / `finish` return `anyhow::Result`.**
  The per-event path is called once per stream event and wants one cheap error type to react to;
  the lifecycle calls are rare and want context. `anyhow::Error` converts into
  `io::Error::other` cleanly, so nothing is lost.
- **`from_exec_event` stays a pure per-event projection.** The two run-scoped fields on
  `exec_finished` (`cost_usd`, `duration_s`) cannot be known from one event, so the function
  leaves them empty and `JournalRun::record_exec` — which owns the clock and the running cost —
  stamps them. The alternative, threading run state into the mapping's signature, would have made
  the mapping untestable in isolation.
- **A turn boundary is not a run boundary.** `TurnCompleted` maps to `exec_event` on a
  `turn_completed` stream label. Mapping it to `exec_finished` would misreport one multi-turn run
  as several runs.
- **The `Message` projection is `format!("{message:?}")`, and no `Serialize` derive was added to
  the inbound wire model.** The debug rendering is the projection available without widening a
  Phase 15 type (D-37); it travels through the redactor like every other string; and Phase 18 may
  want a richer projection once it has a surface to render one into.
- **`run_ended` is appended before the terminal `run.json` write.** A crash in that window leaves
  a journal that names the ending and a record that does not — the shape Phase 17's reconciliation
  reads. The inverse ordering would leave a record claiming an ending the journal never saw.
- **The crash test derives its expected literals from the library redactor.** `redact(...)` is
  called on the library's own planted constants rather than retyping `[REDACTED:…]`, so a change
  to either the planted data or the replacement table cannot silently make the test vacuous. It
  also keeps `grep -c 'sk-ant' tests/journal_crash.rs` at `0`.

## Deviations from Plan

### 1. [Rule 3 — blocking] Task 1's `Serialize`-derive criterion was unsatisfiable as written

- **Found during:** Task 1 verification.
- **Criterion:** *"`grep -rc 'derive(.*Serialize' src/executor/stream_json.rs | grep -v ':0$'`
  produces no output."*
- **Issue:** It produces `5` at the **base commit**, before this plan touched anything. Those five
  derives are on the **outbound** stdin types (`UserMessage`, `UserMessageBody`, `TextBlock` and
  two siblings at `stream_json.rs:227-280`), which have always been serialised because the
  executor writes them. The criterion would have failed identically against an empty diff, so as
  written it is a vacuous gate rather than a guard.
- **Resolution:** The criterion's *intent* — "no serialisation derive was **added** to the wire
  model to make journalling easier" — was verified two ways, both stronger than the grep:
  `git diff --stat src/executor/` is **empty** for this plan (nothing in that tree changed at
  all), and the four **inbound** types the mapping actually reads (`StreamMessage`,
  `SystemMessage`, `TurnMessage`, `ResultMessage`) still derive `Deserialize` only. The count is
  unchanged at 5 between `0b105c7` and `HEAD`.
- **Files modified:** none — a correction to a plan criterion, not to code.

### 2. [Rule 2 — missing critical] `MAX_TAIL_BYTES` had no untuned admission

- **Found during:** Task 3's constant audit.
- **Issue:** Three of the four growth constants explicitly admitted no tuning data exists.
  `MAX_TAIL_BYTES` explained *why* 4 MiB is a wide margin and why the branch is pure defence, but
  never said the number itself is untuned — so a later reader could have mistaken it for a
  measured bound.
- **Fix:** Added the admission in the same shape the other three use.
- **Files modified:** `src/journal/mod.rs`
- **Committed in:** `53add47`

### 3. [Rule 2 — missing critical] Two accessors beyond the plan's symbol list

- **Found during:** Task 1.
- **Issue:** The plan lists `paths()` as the only accessor, but the exactly-twice assertion needs
  to read the write count, and the mapping assertion needs the record's `argv_digest` to compare
  against what landed in the file. Reaching either through a test-only escape hatch would have
  reopened a seam for a test's convenience.
- **Fix:** Added `record_writes() -> usize` and `record_document() -> &RunRecord`, both read-only
  and both documented. Neither exposes the writer or the record mutably, so `run.json`'s
  write-exactly-twice invariant stays enforced by the type.
- **Files modified:** `src/journal/mod.rs`
- **Committed in:** `70c7fcf`

---

**Total deviations:** 3 (1 blocking criterion correction, 2 missing-critical additions).
**Impact on plan:** None on scope. Deviation 1 changed no code; deviations 2 and 3 are small,
documented additions required to satisfy the plan's own acceptance criteria.

## Threat Mitigations Applied

| Threat ID | Applied |
|---|---|
| T-16-29 | Every mapped string reaches the file only through `JournalWriter::append` → `RedactedLine::new`, whose sole constructor is the redactor; the crash test asserts on the surviving **bytes**, not an in-memory value |
| T-16-30 | `finish()` writes the terminal record after the run has ended, and `prune_runs` refuses to prune a run whose `ended_at` is absent — so the crash evidence Phase 17 reads survives |
| T-16-31 | The mapping reads existing fields only. `git diff --stat src/executor/` is empty for this plan and no `Serialize` derive was added to the inbound wire model |
| T-16-32 | The wait-until-producing loop is deadline-bounded (20 s) and panics with the observed byte count rather than looping; the suite returns in **0.08 s** against a 60 s bound |
| T-16-33 | The child is this same test binary, selected by an env var set only inside the parent's own spawn call, running a loudly-named test-support function. No new binary target, no release surface |
| T-16-34 | The planted values are synthetic and live in library constants; the expected replacements are computed by calling the library redactor, so the test cannot drift from what the child wrote |

## Issues Encountered

None. The re-exec harness worked first time, exactly as RESEARCH §6.2 recorded it.

## Known Stubs

None. Every symbol this plan added is wired and exercised.

Two things that **look** like stubs and are not, with their reasons written into the code:

- **`RunStarted { dry_run: false }` is hardcoded.** Dry-run is Phase 17's; until it exists every
  run is a real one. The field is written now so that phase adds no schema migration (D-36).
- **`RESERVED_KINDS` names four kinds nothing emits.** That is the point — they are schema-only so
  Phases 18 and 20 need no migration, and
  `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` asserts both halves: each
  reserved kind is backed by a real constructible variant, and none is produced by this phase's
  mapping.

## User Setup Required

None — no external service configuration.

## Next Phase Readiness

**Phase 16 is complete.** All three ROADMAP success criteria map to named, runnable checks; all
eighteen RESEARCH map rows have satisfying tests; the gate is green with the lint count unchanged
at 5; and `Cargo.toml` / `Cargo.lock` are byte-identical to the phase's starting point.

Handover notes for Phase 17:

- **`JournalRun` is the API to drive.** Hand it a `RunRecord` at spawn time and call `record_exec`
  per event off the `ExecutionEvent` stream. `run.json`'s two writes are already correctly placed
  relative to agent spawn and exit; do not add a third.
- **`RunRecord::opt_in` is `Option<String>` and unpopulated.** Phase 17 owns the `driver_opt_in`
  record, and this field is where it lands.
- **`RunStarted::dry_run` is hardcoded `false`.** Phase 17 owns dry-run and should thread it
  through `JournalRun::start`.
- **`schedule_journal_tail` in `src/app.rs` is `fn`, not `pub fn`** (carried from 16-05). A driver
  that needs to trigger a tail outside a watcher event must promote it, with the reason recorded.
- **`from_exec_event`'s `Message` projection is a debug rendering.** Phase 18 should replace it
  with a structured projection once there is a surface to render into — and must do so without
  adding a `Serialize` derive to the inbound wire model, which is what D-37 protects.

## Self-Check: PASSED

- `tests/journal_crash.rs` present on disk, contains `current_exe` and `crash_test_writer_loop`.
- `src/journal/mod.rs` present, contains `pub struct JournalRun`, `pub fn from_exec_event`,
  `EMITTED_KINDS`, `RESERVED_KINDS`, and `ExecutionEvent::` links to `src/executor/mod.rs`.
- All three task commits present in `git log`: `70c7fcf`, `4584948`, `53add47`.
- Every task's `<acceptance_criteria>` re-run and passing, with the one unsatisfiable criterion
  documented as deviation 1 rather than silently skipped.
- Plan-level `<verification>` re-run: build clean, 427 tests green, `clippy -- -D warnings` exit 0,
  `--all-targets` exactly 5, `Cargo.toml`/`Cargo.lock` unchanged across the phase, crash test
  returns in 0.08 s.
- `STATE.md` and `ROADMAP.md` untouched, as required for a parallel worktree executor.

---
*Phase: 16-run-journal-state-substrate*
*Completed: 2026-07-29*
