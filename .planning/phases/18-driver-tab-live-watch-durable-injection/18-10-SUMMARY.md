---
phase: 18-driver-tab-live-watch-durable-injection
plan: 10
subsystem: ui
tags: [ratatui, tui, driver, journal, injection, scroll-clamp, evidence-derived-state]

# Dependency graph
requires:
  - phase: 18-07
    provides: "DriverInjectScreen, DriverStartScreen and the run-id contract on the injection input"
  - phase: 18-09
    provides: "The Driver tab frame — run list, run header, reused D-R-P-E-V row, step timeline, `driver_viewport`"
  - phase: 18-04
    provides: "`DriverOutput` ring, `DriverLineKind`, `sanitize_render_line`, the per-record and per-line caps"
  - phase: 18-05
    provides: "`ProjectViewCache.driver_inbox` and the run-list scan that populates it"
  - phase: 18-02
    provides: "The driver-side `interjected` / `interjection_acted_on` / `interjection_missed` records"
provides:
  - "The live output pane with its follow bit and the UIFIX-04 clamp ordering in its newest place"
  - "`derive_injection_states` — the four injection states as a pure set intersection over `inbox.jsonl` and the run's journal"
  - "`InjectionState`, `injection_rows` — always two rows, three when missed, no width branch"
  - "`TerminalState` + `terminal_state_cell` — the exhaustive, wildcard-free, evidence-derived terminal-state table"
  - "The adopted-run notice: the pane's first row for any run this session did not spawn"
  - "The Driver tab's key routing: j/k/Up/Down, PageUp/PageDown, f, G, i, s, x"
affects: [18-11, phase-20-decision-router, phase-19-git-blast-radius]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One shared `clamp_scroll` against render-recorded `ViewportMetrics` — never a second `total_lines - visible_height`"
    - "Render-layer state as a pure function of two files, recomputed every frame, with nothing cached that a restart would lose"
    - "Typed state enum + wildcard-free match, so a new upstream variant is a compile error rather than a silently wrong word"
    - "A string protocol across a process boundary is proved by round-tripping through the writer's own labeller, never by restating the labels"

key-files:
  created: []
  modified:
    - src/ui/screens/driver.rs
    - src/ui/screens/detail.rs
    - src/driver/run.rs
    - src/ui/screens/mod.rs
    - src/action.rs
    - src/app.rs

key-decisions:
  - "`interjected { delivered: false }` stays `queued` rather than being promoted — the journal is saying the stdin write FAILED, and rendering it as delivered is exactly the overstatement T-18-55 forbids"
  - "The four-state widget is SPLICED INTO the pane in place of the raw injection records rather than appended beside them, so the message text and the bare correlation ids are not printed twice"
  - "`terminal_state_cell` matches over a render-layer `TerminalState`, and `TerminalState::from_outcome` matches over `RunOutcome` — both wildcard-free, so a new upstream variant is a compile error while the table still covers the three verdict-only states a `RunOutcome` cannot express"
  - "`driver::run::outcome_label` was made `pub(crate)` so the render vocabulary is proved against the one the driver actually writes, rather than restated in a test that would agree with itself"
  - "The steps row carries the full run-detail word (`succeeded`, `succeeded, no changes on disk`); the 18-cell run-list row keeps its abbreviation"
  - "`i` requires `ObservedRun::is_live` — positively observed running, never `LivenessUnknown` — because an input aimed at a run nothing can speak for queues a message that may already be undeliverable"
  - "The adopted notice renders above the ring-overflow notice: it describes where every row below came from, while the drop notice describes only the buffer's own shortfall"

patterns-established:
  - "Forbidden-vocabulary guard: the word list lives in the TEST module and is matched on tokenised words, so the render module's own prose cannot make the assertion vacuous and `already` does not count as `read`"
  - "Width-independence proved structurally: `injection_rows` takes no width at all, and the test varies the message length instead of a width parameter that does not exist"
  - "Buffer selection by run id: the per-alias live ring speaks only for the run being tailed; every other selection reads the whole-journal snapshot, so no run's output is ever attributed to another"

requirements-completed: [OBS-04, OBS-05, STEER-02]

coverage:
  - id: D1
    description: "The live output pane scrolls with the UIFIX-04 clamp ordering, auto-follows the tail, and accounts for every dropped or truncated line"
    requirement: OBS-04
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_first_page_up_after_viewport_grows_moves_viewport, #g_jumps_to_the_driver_tail_and_re_enables_follow, #changing_the_selected_run_resets_the_offset_and_the_follow_bit"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#a_buffer_that_dropped_lines_states_the_count_in_its_first_row, #a_truncated_record_says_so_rather_than_losing_the_lines_silently, #a_diagnostic_record_renders_as_a_diagnostic_row, #an_empty_journal_renders_the_no_entries_copy"
        status: pass
    human_judgment: false
  - id: D2
    description: "An injected message shows queued, delivered, acted-on or missed — derived from disk alone, honest about the fifty-five-second gap, never claiming receipt"
    requirement: STEER-02
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#each_of_the_four_states_derives_from_the_record_set_that_supports_it, #a_message_with_both_records_derives_acted_on_rather_than_delivered, #a_message_present_only_in_the_inbox_derives_queued, #a_message_present_only_in_interjection_missed_derives_missed, #two_messages_with_identical_text_but_different_ids_derive_independently"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_rendered_rows_are_exactly_two_and_three_when_missed_at_every_width, #only_the_delivered_row_carries_an_elapsed_counter, #no_rendered_injection_string_uses_a_word_that_implies_receipt, #the_injection_widget_supersedes_the_raw_records_in_the_pane"
        status: pass
    human_judgment: false
  - id: D3
    description: "A finished or failed run is reviewable after the fact from evidence alone, over the same two-pane view on a journal that has stopped growing"
    requirement: OBS-05
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#every_terminal_state_maps_to_its_own_cell_from_evidence_alone, #the_render_vocabulary_is_the_one_the_driver_actually_writes, #liveness_unknown_is_not_rendered_as_dead, #succeeded_no_changes_is_its_own_state_and_not_a_flavour_of_success"
        status: pass
    human_judgment: false
  - id: D4
    description: "An adopted run says plainly that its live output is gone, and nothing anywhere promises otherwise"
    requirement: OBS-05
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_adopted_notice_renders_only_for_a_run_this_session_did_not_spawn, #the_follow_indicator_says_which_of_the_four_states_the_pane_is_in"
        status: pass
    human_judgment: false
  - id: D5
    description: "The Driver tab's key routing: j/k/Up/Down select runs, PageUp/PageDown scroll, f toggles follow, G jumps to the tail, i injects, s starts, x stops"
    requirement: STEER-02
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#pressing_i_with_no_live_run_sets_a_message_and_dispatches_nothing, #pressing_i_with_a_live_run_pushes_the_injection_screen, #pressing_s_on_the_driver_tab_pushes_the_start_flow, #pressing_x_on_the_driver_tab_pushes_the_stop_confirmation, #the_three_driver_keys_are_scoped_to_the_driver_tab"
        status: pass
    human_judgment: false
  - id: D6
    description: "The filling-shape progression reads correctly across a minute-long gap in a real terminal, with no spinner and no implied imminence"
    verification: []
    human_judgment: true
    rationale: "Whether ○ → ◐ → ● reads as honest progress rather than as a stalled UI is a perceptual judgement about a live 55-second gap; no unit test can observe how the shape lands on a reader over a real minute."

# Metrics
duration: 5h 00m
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 10: The Live Output Pane, the Four-State Injection Display and After-the-Fact Review Summary

**A scrolling output pane that follows its tail and never loses a line silently, injected messages in four disk-derived states that never claim receipt, and an exhaustive evidence-derived terminal-state table with a wildcard-free match.**

## Performance

- **Duration:** ~5h across two sessions (Tasks 1 + 2-plumbing in a prior worktree session, Task 2-remainder + Task 3 here)
- **Started:** 2026-07-30T01:37:34Z (first task commit)
- **Completed:** 2026-07-30T03:36:52Z (final task commit)
- **Tasks:** 3
- **Files modified:** 6

## Execution provenance

**Task 1 and the Task 2 plumbing were executed in a prior session's git worktree, which was cut off mid-plan.** That work was recovered and merged onto `master` as `258b340`:

- `e75cee3` — Task 1: the live output pane, its follow bit and the UIFIX-04 clamp (`detail.rs` +311, `driver.rs` +418)
- `a0926f2` — Task 2 plumbing: `Action::DriverRunsListed.journal`, `ProjectViewCache.driver_journal`, `DriverRunJournal`, the run-list scan's whole-journal read, the live tail's injection append, and the `INJECTION_KINDS` / glyph / label / gloss constants

Those constants landed as nine `never used` warnings, which is what made the cut-off visible: this session's work is what consumes them. This session executed the **remainder of Task 2** and **all of Task 3**, working directly in the main tree on `master` as instructed — no worktree was created.

## Accomplishments

- **The four injection states are a pure function of two files.** `derive_injection_states` intersects the ids in `inbox.jsonl` with the ids in the three journal kinds, later states winning. Nothing is cached; the answer is recomputed every frame, which is what makes STEER-03 hold across a TUI restart with no extra persistence.
- **The vocabulary is enforced mechanically.** `no_rendered_injection_string_uses_a_word_that_implies_receipt` tokenises every rendered injection string and every label against a forbidden list held in the test module — so the render module's own prose cannot soften the assertion, and `already` does not accidentally satisfy `read`.
- **The widget supersedes the raw records rather than sitting beside them.** An `interjected` line carries the text and the two transitions carry a bare correlation id; splicing the widget in at the first of them means the message appears exactly once and the ids not at all — while a journal with no derivable entries keeps its raw lines, because a transition that cannot be paired is still evidence.
- **`terminal_state_cell` is exhaustive in two directions.** The render-layer match over `TerminalState` has no wildcard, and `TerminalState::from_outcome`'s match over `RunOutcome` has none either — so a new upstream outcome variant is a compile error, while the table still covers the three verdict-only states (`live`, `liveness unknown`, `crashed without a terminal record`) that no `RunOutcome` can express.
- **The two vocabularies are proved to agree.** `driver::run::outcome_label` became `pub(crate)` so the round-trip `RunOutcome → label → TerminalState` is asserted against the string the driver actually writes to disk, rather than against a second copy of the labels in a test.
- **`i` refuses visibly and silently.** With no live run it sets the pinned refusal and dispatches nothing — the receiver is asserted directly, which is the half a message-only test would miss.
- **The adopted run says its live output is gone**, as the pane's first row, above the ring-overflow notice.

## Task Commits

1. **Task 1: The live output pane, its follow bit, and the clamp-ordering invariant (D-19)** — `e75cee3` (feat) *[prior session's worktree, recovered as `258b340`]*
2. **Task 2: The four-state injection display, derived from disk (STEER-02, D-07, D-10)** — `a0926f2` (feat, plumbing, prior session) + `47eb443` (feat, this session)
3. **Task 3: After-the-fact review, the adopted-run notice, and the Driver tab's keys (OBS-05)** — `8785de5` (feat)

## Files Created/Modified

- `src/ui/screens/driver.rs` — `InjectionState`, `InjectionEntry`, `derive_injection_states`, `injection_rows`, `injection_block`, `TerminalState`, `terminal_state_cell`, `output_for_run`, the adopted-run notice, the injection splice in `output_body_lines`, and the terminal word on the steps row
- `src/ui/screens/detail.rs` — the Driver tab's `i` / `s` / `x` key arms and the five key-routing tests
- `src/driver/run.rs` — `outcome_label` widened to `pub(crate)` with the reason recorded on it
- `src/ui/screens/mod.rs`, `src/action.rs`, `src/app.rs` — the Task 2 plumbing from the prior session (`DriverRunJournal`, the whole-journal scan read, the run-id-guarded injection append)

## Decisions Made

1. **`interjected { delivered: false }` does not promote to `delivered`.** `driver/run.rs:698-731` writes the record whether or not `Executor::send` returned `Ok`, so `delivered: false` is the journal saying the write **failed**. The message stays `queued`, which is what it still is.
2. **The widget is spliced into the pane, not appended.** Placement is at the first `Injection`-kind ring line, or at the tail when every message is still queued (a queued message is the newest thing that has happened).
3. **A render-layer `TerminalState` rather than a bare `&RunOutcome` parameter.** Three of the twelve states are `RunVerdict`s that no `RunOutcome` can express, so a function taking only `&RunOutcome` could not have rendered `live`, `liveness unknown` or `crashed without a terminal record` at all.
4. **The steps row carries the run-detail word, the run-list row keeps its abbreviation** (UI-SPEC Surface 8's two word columns). The existing assertion on `"ok"` was updated to `"succeeded"`.
5. **Buffer selection is by run id.** `output_for_run` reads the per-alias live ring only when the selected run **is** the run being tailed, and the whole-journal snapshot otherwise — the same error `DriverRunTally`'s embedded run id exists to prevent.
6. **`i` requires `is_live`, not merely `observed`.** `LivenessUnknown` is not enough to open an input aimed at a run.
7. **No detail parameter on `terminal_state_cell`.** What is on disk for a finished run is the outcome *label*; a reason, a denial count or a cap duration is simply not there. The return type stays an owned `String` so a later source can be added without touching a caller.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] A failed stdin write must not render as `delivered`**

- **Found during:** Task 2 (the injection derivation)
- **Issue:** The plan's rule is a bare set intersection — *"present in an `interjected` record is delivered"*. But `JournalEvent::Interjected` carries a `delivered: bool` that is exactly *"did `Executor::send` return `Ok`"*, and the driver writes the record on **both** branches. Following the bare rule would render a message whose write failed as `delivered`, which is the overstatement T-18-55 is a `high` `mitigate` row against and which the plan's own prohibition forbids in as many words.
- **Fix:** An `interjected` record with `delivered: false` does not promote the state; the message stays `queued`. Recorded in the function's doc as a named qualification rather than as a silent branch.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `each_of_the_four_states_derives_from_the_record_set_that_supports_it` asserts it directly.
- **Committed in:** `47eb443`

**2. [Rule 3 - Blocking] `terminal_state_cell` needed a typed input the render layer could actually hold**

- **Found during:** Task 3
- **Issue:** The plan's acceptance criterion requires a wildcard-free match "over the outcome enum", but the render layer only ever has `RunSummary.outcome: Option<String>` plus a `RunVerdict` — and three of the twelve states named in the plan's own list (`live`, `liveness unknown`, `crashed without a terminal record`) are not `RunOutcome` variants at all. A single-argument function over `&RunOutcome` could not have been called from the render path and could not have covered a third of the table.
- **Fix:** A render-layer `TerminalState` enum with **two** wildcard-free matches — `terminal_state_cell` over `TerminalState`, and `TerminalState::from_outcome` over `RunOutcome` — so a new upstream variant is still a compile error while the table covers every state.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `every_terminal_state_maps_to_its_own_cell_from_evidence_alone` and `the_render_vocabulary_is_the_one_the_driver_actually_writes`.
- **Committed in:** `8785de5`

**3. [Rule 3 - Blocking] `outcome_label` widened to `pub(crate)`**

- **Found during:** Task 3
- **Issue:** The label vocabulary is a string protocol across a process boundary. A test that spelled the nine labels out a second time would agree with itself while disagreeing with disk the moment either side changed.
- **Fix:** `driver::run::outcome_label` is `pub(crate)`, with the reason recorded on the function. The round-trip test is `#[cfg(unix)]` because `driver::run` is a unix-only module.
- **Files modified:** `src/driver/run.rs`
- **Verification:** `the_render_vocabulary_is_the_one_the_driver_actually_writes`.
- **Committed in:** `8785de5`

**4. [Rule 2 - Missing Critical] The adopted notice is ordered above the ring-overflow notice**

- **Found during:** Task 3
- **Issue:** The UI-SPEC calls both the adopted notice and the drop count *"the first rendered row"*. They can co-occur.
- **Fix:** The adopted notice goes first — it describes where every row below came from, while the drop notice describes only the buffer's own shortfall. Recorded as a numbered ordering in `output_body_lines`'s doc.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `the_adopted_notice_renders_only_for_a_run_this_session_did_not_spawn` asserts row 0; `a_buffer_that_dropped_lines_states_the_count_in_its_first_row` still asserts row 0 for the non-adopted case.
- **Committed in:** `47eb443` (notice) / `8785de5` (test)

**5. [Rule 1 - Bug] The steps row's state word was the run-list abbreviation**

- **Found during:** Task 3
- **Issue:** UI-SPEC Surface 8 assigns two different word columns — a run-list abbreviation and a run-detail word — and the steps row is in the run detail. It was rendering `ok` where the spec says `succeeded`.
- **Fix:** `steps_lines` now reads `terminal_state_cell`. The landed assertion on `"ok"` was updated to `"succeeded"`.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `the_turn_counter_appears_only_while_the_run_is_live`.
- **Committed in:** `8785de5`

---

**Total deviations:** 5 auto-fixed (2 missing critical, 2 blocking, 1 bug)
**Impact on plan:** Deviations 1 and 4 tighten the plan toward its own prohibitions. Deviations 2 and 3 are the mechanical shape the acceptance criterion required once the render layer's actual inputs were in hand. Deviation 5 implements a spec row the plan cited. No scope creep; no new dependency; no anti-feature.

## Issues Encountered

- **The plan's acceptance criterion `rg -n 'Spinner|spinner|tick_chars|throbber' src/ui/screens/driver.rs` returns matches, and this is correct rather than a violation.** Every one is a doc comment *forbidding* a spinner (the module doc, `MISSED_GLOSS`'s doc, `InjectionState`'s doc, `injection_rows`'s doc) or a test's own prose. There is no spinner widget, no animation, no tick source and no rendered string containing the word. The criterion's grep was written against the code and the phase's prose reuses the word to prohibit it — the prohibition itself is what trips the grep.
- **`ScreenAction` has no `Debug` impl**, so a `match ... other => panic!("{other:?}")` in the key tests does not compile. Resolved with a `pushed_screen_name` helper reading `Screen::name()`, which is the identity `DetailScreen::NAME` already exists to make reliable.

## Verification

| Gate | Result |
|---|---|
| `cargo build && cargo test && cargo clippy -- -D warnings` | **pass** |
| `rtk proxy cargo test` | **626 lib tests, 0 failures**; whole suite green (was 607 before this session) |
| `rtk proxy sh -c "cargo clippy --all-targets 2>&1 \| grep '^warning: ' \| grep -v generated \| wc -l"` | **5**, unchanged — the nine `never used` warnings the prior session left are consumed |
| New Cargo dependency | **none** — `Cargo.toml` and `Cargo.lock` untouched |
| `Modifier::REVERSED` in `driver.rs` / `detail.rs` | **0 occurrences** |
| `Wrap` widget on the output pane | **none** — both matches are prose explaining why it is absent |
| `saturating_sub` re-deriving `total_lines - visible_height` in `driver.rs` | **none** — one shared `clamp_scroll` |

All raw-output checks were run through `rtk proxy`; the `rtk` wrapper filters `warning:` and `test result:` lines out of plain `cargo` output, so the same greps against unwrapped `cargo` would have passed vacuously.

## Known Stubs

None. Every constant the prior session landed is now consumed by a code path with a test behind it.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema change at a trust boundary. The plan's four `high` `mitigate` rows are each discharged:

| Threat | Discharge |
|---|---|
| T-18-54 (a terminal-state word from agent prose) | `terminal_state_cell` matches exhaustively on the typed state; its doc states the Replit rule and names the incident; no prose field feeds a word, glyph or colour |
| T-18-55 (an injection state that overstates) | Pure set intersection, later states winning, `delivered: false` explicitly refused, and the tokenised forbidden-word test |
| T-18-56 (escape sequences reaching the terminal) | No second append path; the pane renders `DriverOutput::lines()`, sanitised at append time by 18-04, and `injection_rows` sanitises the one string it interpolates |
| T-18-58 (promising live output for an adopted run) | The notice is the pane's first row for any run absent from `session_spawned_runs`; every `live output` occurrence in the module either says it is gone or forbids the promise |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **18-11 is unblocked.** Its Task 1 (the scrollable help screen) needs the injection-state legend and the Driver key section, both of which now exist as named constants and bound keys.
- **The footer's three width forms and the help screen's own update are 18-11's**, not this plan's — the Driver tab currently renders no per-tab footer hints for `i` / `s` / `x`, which is the gap 18-11 Task 1 closes.
- Phase 20's router adds rows to the step timeline without re-laying-out the section; the honest note (`This version runs one command per run.`) is what has to be removed when it does.

## Self-Check: PASSED

Every file this summary claims exists on disk; every commit hash it cites resolves in `git log`.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
