---
phase: 18-driver-tab-live-watch-durable-injection
plan: 03
subsystem: infra
tags: [rust, serde, stream-json, rendering, projection, run-listing, path-traversal]

requires:
  - phase: 15-executor-duplex-control-channel
    provides: "`TurnMessage`, `ResultMessage`, the envelope-tolerance posture, and D-32's measured `result`-is-absent-on-error semantics"
  - phase: 16-run-journal-state-substrate
    provides: "`JournalEvent::ExecEvent`, `from_exec_event`, `RunRecord`, `runs_root`, `new_run_id`'s lexicographic-equals-chronological id format"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "the fallible `journal::run_paths -> Option<RunPaths>` and `is_plain_run_id`, which are the whole traversal guard `list_runs` stands on"
provides:
  - "`TurnMessage::text_content` — the readable projection of one turn's content blocks"
  - "`stream_json::MessageBody` / `stream_json::ContentBlock` — the minimal, tolerance-first content-block model"
  - "`ResultMessage.duration_ms` — the per-turn wall clock the turn projection reports"
  - "`journal::exec_message_text` / `journal::turn_result_text` — `ExecEvent.text` is now prose, not a `Debug` struct dump"
  - "`journal::RunSummary`, `journal::list_runs`, `journal::sort_run_summaries_newest_first` — the Driver tab's run list, one cheap call, newest first"
affects: [18-04, 18-05, 18-07, 18-10, phase-20]

tech-stack:
  added: []
  patterns:
    - "Tolerant serde readers where `#[serde(default)]` is not enough: a field of the WRONG SHAPE is a hard parse error, and only a custom `deserialize_with` degrades it to a missing field"
    - "Display projections that compose the facts the source reported rather than inventing a third rendering of them (the `LineTruncated` arm's idiom, applied to turns)"
    - "Cheap list rows: a per-item summary read from the one small committed artifact, never from the growing transcript beside it"

key-files:
  created: []
  modified:
    - src/executor/stream_json.rs
    - src/journal/mod.rs

key-decisions:
  - "`exec_message_text` takes `&StreamMessage`, not `&TurnMessage`: the role lives on the envelope's `type`, which IS the enum variant, and the `Message` arm also carries system, control-response and rate-limit envelopes that would otherwise have kept their `Debug` rendering"
  - "`message` and `content` get custom tolerant deserialisers — `#[serde(default)]` rescues an ABSENT field and nothing else, so a body of an unmodelled shape would still have failed the whole envelope into `ExecutionEvent::Unparseable`"
  - "A non-text content block renders as a bracketed label naming its type and tool name, and is never expanded into its payload: unbounded `tool_result` content would dominate the per-run byte cap and flush a bounded output buffer"
  - "The projection deliberately does not sanitise. Terminal-control stripping is the renderer's, at buffer-append time (18-04); the journal is evidence and stripping at write time would destroy the record that the agent emitted those bytes"
  - "`list_runs` reads `run.json` as a `serde_json::Value`, not through `RunRecord` — one added required field would otherwise make every older run invisible in the UI"
  - "The traversal guard is `run_paths`'s `Option`, applied before any join; the refusal is tested against a record planted OUTSIDE the runs root, which a `read_dir` walk can never produce and therefore can never prove"

requirements-completed: []

coverage:
  - id: D1
    description: "`ExecEvent.text` carries the agent's words, not a `Debug` rendering of `TurnMessage`"
    requirement: OBS-04
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_text_only_turn_journals_the_agents_words_not_a_debug_struct"
        status: pass
      - kind: unit
        ref: "src/executor/stream_json.rs#every_golden_transcript_turn_projects_without_a_debug_rendering"
        status: pass
    human_judgment: false
  - id: D2
    description: "A non-text block keeps a visible label rather than vanishing from the pane"
    requirement: OBS-04
    verification:
      - kind: unit
        ref: "src/executor/stream_json.rs#a_tool_use_block_keeps_a_visible_label_naming_the_tool"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#a_tool_use_turn_keeps_a_visible_label_in_the_journal"
        status: pass
    human_judgment: false
  - id: D3
    description: "The envelope parser is no less tolerant than before: an unknown block shape, a wrong-typed body and a wrong-typed content field all degrade to a missing field rather than to `Unparseable`"
    verification:
      - kind: unit
        ref: "src/executor/stream_json.rs#a_body_or_content_of_the_wrong_shape_still_parses_as_a_carried_envelope"
        status: pass
      - kind: unit
        ref: "src/executor/stream_json.rs#a_content_block_shape_no_build_has_seen_degrades_to_a_label_not_a_parse_failure"
        status: pass
      - kind: unit
        ref: "src/executor/stream_json.rs#every_line_of_every_golden_transcript_parses_to_a_carried_envelope"
        status: pass
    human_judgment: false
  - id: D4
    description: "An error `result` envelope, whose `result` field is absent entirely, projects without panicking"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#an_error_result_envelope_with_no_result_field_projects_without_panicking"
        status: pass
    human_judgment: false
  - id: D5
    description: "A multi-byte turn reaches the journal byte-identically with no boundary panic"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_multibyte_turn_reaches_the_journal_byte_identically"
        status: pass
      - kind: unit
        ref: "src/executor/stream_json.rs#a_multibyte_turn_projects_its_scalars_intact"
        status: pass
    human_judgment: false
  - id: D6
    description: "A finished run's list row is built from `run.json` alone, with no journal parse (`read_all`/`tail_lines` unreachable from `list_runs`)"
    requirement: OBS-05
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_records_goal_and_command_surface_verbatim"
        status: pass
      - kind: other
        ref: "grep -n 'read_all|tail_lines' src/journal/mod.rs — both remaining hits are pre-existing tests, neither is inside list_runs"
        status: pass
    human_judgment: false
  - id: D7
    description: "Runs list newest first by lexicographic-descending run id, and the ordering rationale is in the sort function's doc"
    requirement: OBS-05
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#three_runs_come_back_newest_first"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#run_id_sorts_lexicographically_in_chronological_order"
        status: pass
    human_judgment: false
  - id: D8
    description: "A traversing run id is refused before any path is joined or read, while a plain id is still read (T-18-16)"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_traversing_run_id_is_refused_before_any_record_is_read"
        status: pass
    human_judgment: false
  - id: D9
    description: "One damaged or recordless run directory is skipped with a warning and does not hide its healthy neighbours"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#a_directory_with_no_record_is_skipped_without_hiding_its_neighbours"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 03: Readable Journal Text & Run Enumeration Summary

**`ExecEvent.text` now reads as the agent's own words instead of
`TurnMessage { session_id: Some(..), is_replay: false, .. }`, and a project's
runs are enumerable newest-first in one call that never opens a journal.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-07-29T23:09:00Z
- **Completed:** 2026-07-29T23:39:00Z
- **Tasks:** 2
- **Files modified:** 2 (0 created, 2 modified)

## Accomplishments

- **The live-output pane has something readable to render (OBS-04).**
  `from_exec_event`'s two `format!("{:?}")` arms are gone. An `assistant` turn
  journals as `assistant: reading src/driver/run.rs`; a tool call journals as
  `[tool_use: Read]` rather than disappearing; a turn boundary journals as
  `turn ended: success; 1.8s this turn; $0.12 cumulative, notional` followed by
  the agent's own prose on its own line. The named "looks done but isn't"
  failure — a pane full of `Debug` structs satisfying "watch its output" in the
  letter — is closed, and a guard test walks every golden transcript asserting
  no projection contains `TurnMessage {` or `Some(`.
- **The message body is modelled, and the parser is *more* tolerant than
  before, not less.** `MessageBody`/`ContentBlock` carry only what rendering
  needs, every field `Option` + `#[serde(default)]`. Two custom deserialisers
  close the gap `#[serde(default)]` does not: a `message` that arrives as a
  bare string, a `content` that arrives as a number, and a block whose `text`
  is an integer all now degrade to a missing field instead of failing the whole
  envelope into `ExecutionEvent::Unparseable`. All eight golden transcripts
  still parse line-for-line.
- **The Replit rule is written where the next reader will trip over it.**
  `exec_message_text`'s doc states that the agent's words may be displayed as
  content and may never drive a badge, a colour, a status word or a sort key
  (D-13), names the incident behind the rule, and points at the evidence fields
  status must come from instead. A test pins the deliberate *non*-sanitisation
  so a later reader cannot "helpfully" strip `ESC` at write time and destroy
  the evidence that the agent emitted it.
- **The Driver tab's left pane has a data source (OBS-05).** `list_runs` is one
  `read_dir` plus one small `run.json` read per run — never a journal parse, so
  a project holding the full `RETAIN_RUNS` history costs the same to list as a
  project with one run. Sorting is lexicographic-descending with the *why*
  (the id format) and the declined alternative (parsing `started_at`) recorded
  in the doc comment.
- **No new dependency, no clippy regression.** `Cargo.toml`/`Cargo.lock`
  untouched; `cargo clippy --all-targets` still reports exactly the 5
  pre-existing lints. Test count 556 → 581.

## Task Commits

1. **Task 1: A readable text projection for agent turns** — `99aefaf` (feat)
2. **Task 2: Enumerate the runs on disk for the Driver tab's left pane** — `64cfa0d` (feat)

## Files Created/Modified

**Modified**
- `src/executor/stream_json.rs` — `MessageBody`, `ContentBlock` (+ its private
  `render`), `TurnMessage.message`, `TurnMessage::text_content`, the tolerant
  `tolerant_message_body` / `tolerant_content` readers, `ResultMessage.duration_ms`,
  and the rewritten `TurnMessage` doc that no longer defers content blocks to a
  future phase. 11 new inline tests.
- `src/journal/mod.rs` — `exec_message_text`, `compose_turn`, `turn_result_text`,
  the two rewritten `from_exec_event` arms, the corrected `from_exec_event`
  rationale, plus `RunSummary`, `list_runs`, `read_run_summary`,
  `run_summary_from_value` and `sort_run_summaries_newest_first`. 14 new inline
  tests.

## Decisions Made

- **`exec_message_text` takes `&StreamMessage`, not `&TurnMessage`** (see
  deviation 1). A turn alone cannot say whether it is the agent or the user
  speaking — the role is the envelope's `type`, which is the enum variant.
- **Every arm composes a real string; none falls back to `Debug`.** The
  `Message` arm carries `System`, `ControlResponse` and `RateLimitEvent`
  envelopes too (`claude.rs:1478-1489, 1529`), so leaving those on `Debug`
  would have left a third of the pane as struct dumps.
- **A `user` turn with `is_replay` renders as `user (replay): …`.** The marker
  is protocol evidence read off the parsed envelope, not prose, so naming it is
  honest. The `acted-on` correlation is still the driver's, from `is_replay`
  directly — never from this rendered string (D-08).
- **Non-text blocks are labelled, not expanded.** A `tool_result`'s `content`
  is unbounded; expanding it would let one tool result dominate
  `MAX_RUN_JOURNAL_BYTES` and flush a bounded output ring.
- **The cost figure is labelled `cumulative, notional`** in the turn
  projection, carrying `JournalEvent::Cost`'s existing caveat to the one place
  a human actually reads it (D-12).
- **`list_runs` reads a `Value`, not a `RunRecord`** — the same tolerance
  `writer::has_end_timestamp` already applies on the write path.
- **A missing runs root logs nothing.** It is a project that has never been
  driven, not an anomaly; a missing or unparseable `run.json` inside the root
  *is* an anomaly and warns by error kind (never by content, per this module's
  logging rule).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `exec_message_text` takes `&StreamMessage`, not `&TurnMessage`**
- **Found during:** Task 1
- **Issue:** The plan specifies `fn exec_message_text(message: &TurnMessage)`
  *and* that it "composes the role prefix and `text_content()`". Those two are
  not simultaneously satisfiable: `TurnMessage` is the shared shape of both the
  `assistant` and `user` envelopes and carries no role, which lives on the
  envelope's `type` — i.e. on the `StreamMessage` variant. Separately,
  `ExecutionEvent::Message` carries `System`, `ControlResponse` and
  `RateLimitEvent` envelopes as well, so a `TurnMessage`-only helper would have
  left those three arms on their `Debug` rendering, which is the exact thing
  this plan exists to remove.
- **Fix:** `exec_message_text(&StreamMessage)` matches the variant, delegates
  turns to a small `compose_turn(role, turn)` helper, and composes a real
  string for every other variant (`system: init (claude 2.1.220, session …)`,
  `control_response: success (request …)`, the JSON `Display` of a rate-limit
  event). The doc says why the wider argument is needed. Both acceptance greps
  (`fn exec_message_text`, `fn turn_result_text`, `text_content` in both files)
  still match.
- **Files modified:** `src/journal/mod.rs`
- **Verification:** `src/journal/mod.rs#a_text_only_turn_journals_the_agents_words_not_a_debug_struct`
  and `#a_replayed_user_turn_is_named_as_a_replay_in_its_projection`.
- **Committed in:** `99aefaf`

**2. [Rule 2 - Missing Critical] Tolerant deserialisers for `message` and `content`**
- **Found during:** Task 1
- **Issue:** The plan's own `must_haves` truth requires that "an unknown
  content-block shape degrades to a missing field rather than a parse failure
  landing in `ExecutionEvent::Unparseable`", and prescribes `Option` +
  `#[serde(default)]` to get there. Those attributes rescue an **absent** field
  and nothing else. A `message` that is a bare string, or a `content` that is a
  number, is a *type* mismatch — still a hard `serde` error, still fatal to the
  whole envelope. Shipping the plan's literal shape would have satisfied its
  wording while violating the property it was written to guarantee, on a file
  whose entire stated posture is envelope tolerance and whose module doc records
  a run measured at 41% undocumented subtypes.
- **Fix:** `#[serde(default, deserialize_with = "tolerant_message_body")]` on
  `message` and `#[serde(default, deserialize_with = "tolerant_content")]` on
  `content`. A non-object body yields `None`; an array maps element-wise with a
  per-block fallback; a string body becomes one text block (the wire has carried
  both forms); anything else yields no blocks. Every field still also carries
  `Option`/`#[serde(default)]` as the plan directs.
- **Files modified:** `src/executor/stream_json.rs`
- **Verification:**
  `#a_body_or_content_of_the_wrong_shape_still_parses_as_a_carried_envelope`
  drives all four shapes; `#a_string_content_field_becomes_one_text_block`
  covers the string form.
- **Committed in:** `99aefaf`

**3. [Rule 2 - Missing Critical] `ResultMessage.duration_ms` was not modelled**
- **Found during:** Task 1
- **Issue:** The plan requires `turn_result_text` to compose "subtype, terminal
  reason where present, **turn duration**, and the cumulative cost". Phase 15
  modelled `num_turns` and `total_cost_usd` but not `duration_ms`, so the turn
  duration was not reachable.
- **Fix:** Added `#[serde(default)] pub duration_ms: Option<u64>` with a doc
  recording that it is per-turn and resets (D-29/D-12) and that presenting it as
  the run's duration would understate a multi-turn run by every turn but the
  last. Verified present on both success and error envelopes in the golden
  transcripts before adding it.
- **Files modified:** `src/executor/stream_json.rs`
- **Verification:**
  `#the_per_turn_duration_is_read_off_both_success_and_error_envelopes`
  asserts 1804 ms off fixture 01 and 3568 ms off the error fixture 02.
- **Committed in:** `99aefaf`

---

**Total deviations:** 3 auto-fixed (2 missing critical, 1 blocking)
**Impact on plan:** No scope creep and no new file. Deviation 1 changes one
signature; deviations 2 and 3 add ~30 lines inside the plan's own
`files_modified`. All of the plan's acceptance greps still match.

## Issues Encountered

- **`tests/driver_reattach.rs` is flaky under whole-suite load.** Two of five
  full `cargo test` runs failed one or two of its three tests; the same binary
  passes alone and passes when run alongside `driver_kill`, `driver_inbox` and
  `driver_tracer`. Its assertions are `/proc` liveness probes over freshly
  spawned processes, and three sibling executor agents were building and testing
  in parallel worktrees on this machine throughout. **Nothing in this plan is on
  that path** — the two files touched here contain no process spawn, no
  liveness probe and no reconciliation. Recorded rather than fixed; it is a
  timing sensitivity in a Phase 17 test, not a regression from this plan.
- **The repo carries pre-existing `rustfmt` drift** across `app.rs`,
  `journal/{writer,redact,inbox}.rs` and the older regions of both files touched
  here. `cargo fmt` is not part of the project gate. Every line **this plan
  added** was brought to `rustfmt` agreement; the pre-existing drift was left
  alone rather than swept into this diff, where it would have buried the real
  change.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **581 passed, 0 failed** (baseline 556; 486 lib + 95 across 18 integration binaries) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `rtk proxy cargo test --test executor_lifecycle --test executor_transport` | pass — the envelope-tolerance posture is unchanged |
| `grep -c 'Phase 18' src/executor/stream_json.rs` | **0** — content blocks are no longer described as a future concern |

Every count above was taken through `rtk proxy`, because plain `cargo` output is
filtered by the `rtk` summarising wrapper and a criterion that greps for
`warning:` or `test result:` without it passes **vacuously** (this bit Phase 15
and Phase 17).

## Known Stubs

- **`journal::list_runs` and `journal::RunSummary` have no production caller
  yet.** They are complete, tested and `pub`; the consumer is plan 18-04's
  Driver-tab run list, which also owns the `spawn_blocking` scheduling half this
  module deliberately does not provide (D-28). Recorded so a reader does not
  mistake an unwired-but-finished library function for an unfinished one. No
  UI asserts a state the mechanism cannot back.
- **No stub exists on the projection path** — it is live in
  `from_exec_event` and every journalled `ExecEvent` goes through it as of this
  commit.

## Threat Flags

None. The two files touched here add no network endpoint, no auth path and no
schema change at a trust boundary. Within the plan's own register:

- **T-18-16 (hostile directory name under `runs/`) is closed here** — the
  fallible `run_paths` refuses a non-plain id before any join, proven against a
  record planted outside the runs root with a positive control beside it.
- **T-18-14 (terminal-control sequences inside projected text) remains open by
  design and is 18-04's to close.** The projection deliberately preserves `ESC`
  because the journal is evidence; the render-time sanitiser strips it
  unconditionally before any byte reaches a display buffer. The dependency is
  stated in the code's own doc comment, not merely assumed here.
- **T-18-15 (an adversarially large content block)** is bounded downstream as
  planned, and this plan tightens it slightly: a non-text block is labelled
  rather than expanded, so a `tool_result` payload never enters the journal
  through this path at all.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready for the rest of wave 2 and for wave 3.**

- **18-04** has both halves of its data: `ExecEvent.text` is prose it can split
  on `\n`, sanitise and buffer, and `journal::list_runs` is the run list's
  source. Two obligations carry to it: run `list_runs` on `spawn_blocking`
  (D-28 — the doc says so, and nothing enforces it but the reader), and strip
  `ESC` at buffer-append time, because this plan deliberately did not (T-18-14).
- **18-02** is unaffected: nothing here touches `src/driver/`, and the
  `is_replay` correlation it owns reads the parsed envelope, not the projection
  this plan produces.
- **The `RunSummary` shape is deliberately flat and evidence-only.** Its
  `outcome` is `RunRecord.outcome`; anything deriving a badge, colour or sort
  key from a run must read that field and never the agent's prose (D-13).
- **Nothing was added to `AppContext`**, so the phase's one negative
  carry-forward — every new per-alias or per-run map must be pruned in
  `App::prune_driver_maps` — is untouched by this plan and still stands for
  18-04.

## Self-Check: PASSED

Both modified files present and non-empty on disk
(`src/executor/stream_json.rs`, `src/journal/mod.rs`); both task commits present
in `git log` (`99aefaf`, `64cfa0d`); `git diff --diff-filter=D HEAD~2 HEAD`
reports no deletions; working tree clean apart from this summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
