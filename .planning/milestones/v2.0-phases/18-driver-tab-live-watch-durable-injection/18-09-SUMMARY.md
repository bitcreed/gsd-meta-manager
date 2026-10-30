---
phase: 18-driver-tab-live-watch-durable-injection
plan: 09
subsystem: ui
tags: [rust, ratatui, tabs, width-tiers, run-list, pipeline-reuse, testbackend, sanitiser]

requires:
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 03
    provides: "`journal::RunSummary`, `list_runs`, `sort_run_summaries_newest_first` — the run list's data source, read from each run's committed `run.json` and never from the journal beside it"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 04
    provides: "`DriverOutput`, `sanitize_render_line`, `DriverLineKind`, and the four `ProjectViewCache` view-state fields"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 05
    provides: "`DetailSubView::Driver` at index 10, `App::schedule_run_list_scan`, `ProjectViewCache.driver_runs`, `DetailScreen::NAME`, and the `DriverJournalAppended` seam this plan reads the cost and the turn count from"
provides:
  - "`detail::tab_titles(width, active, driver_live)` — one function for both tab-bar render sites, with three width tiers and a window that always contains the active tab"
  - "`TAB_BAR_FULL_CELLS` / `TAB_BAR_COMPACT_CELLS` / `TAB_COUNT` / `DRIVER_TAB_INDEX`"
  - "`Shift+D`, the Driver arm in `switch_to_tab`, and the three measured Driver footer forms"
  - "`src/ui/screens/driver.rs` — `render_driver_tab`, `run_list_row`, `run_state_glyph`, `elapsed_label`, `PIPELINE_LINE_MAX_CELLS`, `DRIVER_DETAIL_MIN_CELLS`"
  - "`AppContext::schedule_run_list_scan` — the scheduler reachable from a `Screen`, with `App::schedule_run_list_scan` delegating"
  - "`ProjectViewCache.driver_tally` / `DriverRunTally` and `app::update_driver_tally` — the cumulative cost and the turn count, read off the journal tail"
  - "`ViewportMetrics` / `clamp_scroll` / `derive_all_stage_statuses` / `build_pipeline_line` / `StageStatus` widened to `pub(super)` — visibility widens, not moves"
  - "`DetailScreen.driver_viewport` — the output pane's recorded metrics, for 18-10's clamp"
  - "`view_cache` pruned in `App::prune_driver_maps`"
affects: [18-10, 18-11, phase-20]

tech-stack:
  added: []
  patterns:
    - "Resolve every width tier in one pure function that returns both the rendered vector and the index into it, so the tiering is assertable without a terminal — the reason `footer_spans` was split out of `build_footer`, applied to the tab bar"
    - "A reserved marker cell that is always present, so a state change never alters a widget's width and nothing shifts sideways under the user's eye"
    - "A held-out `TestBackend` buffer scrape for any defect a width calculation could not see — the original clip was invisible to every number the code had"
    - "A tally keyed by run id *inside* the value rather than by being a map, so 'this fact is not about the run you are looking at' is expressible"

key-files:
  created:
    - src/ui/screens/driver.rs
  modified:
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/app.rs

key-decisions:
  - "`tab_titles` returns the label vector AND the adjusted select index, because a windowed bar whose select index still points into the unwindowed vector highlights the wrong tab — the two must be computed together or not at all"
  - "The windowed tier grows the window outward from the active tab, right first then left, so the active label is the one thing never given up; if even it alone overflows it is still rendered, because a clipped label the user can see beats a correct one they cannot"
  - "The Driver footer's full form is selected at 101 columns, not the UI-SPEC's 100 — the spec's own table gives the form's width as 101, and a threshold one cell below the render is the `STATUS_COLUMN_MIN_CELLS` bug in miniature"
  - "`schedule_run_list_scan` moved onto `AppContext` and `App`'s method delegates: a `Screen` is handed an `&mut AppContext` and never an `&mut App`, so the choice was one function reachable from both or two copies of a `spawn_blocking` closure that would drift"
  - "The Driver tab's `switch_to_tab` arm rescans on every visit rather than only when the list is empty, because unlike the backlog or the git log this list changes while the user is looking away"
  - "The state word for a run with no terminal record and no observation is `◇ ?`, never `crash`: absence of evidence is not evidence of death, which is the CR-05 defect in a new place"
  - "The cost line says `not yet reported` when no `cost` record has been read for the selected run, rather than rendering `$0.00` — a zero would be a figure the mechanism cannot back"
  - "`view_cache` joined the prune pass, because this phase moved three driver payloads into it and the carry-forward would otherwise be met for the maps it names and broken for the one it does not"

patterns-established:
  - "Width tiering: one pure function per tiered widget, returning everything the render site needs, with each threshold equal to the form's measured width"
  - "Reused widgets are called through a visibility widen and asserted by a parity test that compares text AND per-span style, so a later reimplementation is a test failure rather than a drift"

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "`DetailSubView::Driver` is the 11th tab at all six sites, reachable by Left/Right and by Shift+D"
    requirement: TRANS-05
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#every_tab_index_round_trips_through_its_sub_view"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#shift_d_and_right_both_reach_the_driver_tab"
        status: pass
      - kind: other
        ref: "grep -c 'Tabs::new' src/ui/screens/detail.rs == 2, both fed by tab_titles; grep -c 'DetailSubView::Driver' == 15"
        status: pass
    human_judgment: false
  - id: D2
    description: "The active tab's label is present in the rendered bar at every width, with overflow markers where truncated — the >=80-column defect carried since Phase 12 is retired"
    verification:
      - kind: automated_ui
        ref: "src/ui/screens/detail.rs#the_active_tab_label_is_always_present_in_the_rendered_bar (TestBackend scrape at 40/60/80/120)"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_windowed_tier_always_contains_the_active_tab"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_windowed_tier_fits_and_marks_the_side_it_truncated"
        status: pass
    human_judgment: false
  - id: D3
    description: "The Driver label carries a live marker in a reserved final cell that is always present, so the bar's width never changes when a run starts or ends"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_driver_label_is_the_same_width_live_and_idle"
        status: pass
    human_judgment: false
  - id: D4
    description: "The originating goal prompt is readable for any driven project — verbatim, sanitised, wrapped to at most three rows, with `(none given)` for an absent one"
    requirement: OBS-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#an_absent_goal_renders_the_pinned_copy_and_nothing_else"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#a_goal_bearing_an_escape_sequence_is_sanitised_before_it_becomes_a_line"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#a_long_goal_wraps_to_at_most_three_rows_then_ellipses"
        status: pass
    human_judgment: false
  - id: D5
    description: "The run list sorts newest first, renders one 18-cell form at every width, and shows its state word only above 26 cells of inner width"
    requirement: OBS-05
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#a_run_list_row_is_eighteen_cells_before_the_state_word"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_state_word_appears_at_twenty_six_cells_and_not_at_twenty_five"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#three_runs_come_back_newest_first (18-03, the sort this plan calls)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every run state maps from evidence — RunVerdict and RunRecord.outcome — and LivenessUnknown is never collapsed into dead"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#every_run_state_maps_from_evidence_and_unknown_is_not_death"
        status: pass
      - kind: other
        ref: "grep 'ResultMessage|result_text' src/ui/screens/driver.rs — one hit, in a doc comment forbidding the practice; no prose field feeds a colour, glyph or word"
        status: pass
    human_judgment: false
  - id: D7
    description: "The D-R-P-E-V widget is reused verbatim and its trailing [V] survives at 60, 80 and 120 columns"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_driver_tab_pipeline_line_matches_the_pipeline_tabs"
        status: pass
      - kind: automated_ui
        ref: "src/ui/screens/driver.rs#the_pipeline_line_keeps_its_verify_stage_at_sixty_eighty_and_one_hundred_twenty_columns"
        status: pass
      - kind: other
        ref: "grep -c 'fn build_pipeline_line|fn derive_all_stage_statuses|fn stage_color' — 3 in detail.rs, 0 in driver.rs"
        status: pass
    human_judgment: false
  - id: D8
    description: "The step timeline has exactly one decided row and always carries the honest note; the turn counter appears only while live"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_steps_section_has_exactly_one_command_row_and_always_carries_the_note"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_turn_counter_appears_only_while_the_run_is_live"
        status: pass
    human_judgment: false
  - id: D9
    description: "Elapsed time and cumulative cost render, correctly labelled, and an unknown cost says so rather than rendering a zero"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#elapsed_formats_under_and_over_an_hour_and_switches_form_when_ended"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_cost_line_is_labelled_cumulative_and_never_fabricates_a_figure"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_journal_batch_tallies_the_cumulative_cost_and_the_turn_boundaries"
        status: pass
    human_judgment: false
  - id: D10
    description: "Below 60 columns the run list collapses and the selection moves into the detail pane's title; the detail pane never falls below its floor"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_sub_sixty_tier_returns_a_single_pane_and_still_honours_the_selection"
        status: pass
    human_judgment: false
  - id: D11
    description: "The Driver tab's visual composition at a real terminal — spacing, colour balance, and whether the header reads as a run at a glance"
    verification: []
    human_judgment: true
    rationale: "Every mechanical property above is asserted, but whether the assembled surface is legible and well-proportioned to a person driving a real project is an aesthetic judgment no test makes. The run list, header, pipeline row and step timeline have never been seen together on a terminal."

duration: 56min
completed: 2026-07-30
status: complete
---

# Phase 18 Plan 09: The Driver Tab's Frame Summary

**The 11th tab exists, is reachable by `Shift+D`, and is visible at 40, 60, 80 and 120
columns — as is every other active tab, which was not true before this plan — and behind
it a two-pane surface shows a project's runs newest first with, for the selected one, the
goal it was given verbatim, how long it has been going, what it has cost, and the same
D-R-P-E-V line the Pipeline tab renders, called rather than reimplemented.**

## Performance

- **Duration:** 56 min
- **Started:** 2026-07-30T00:21:00Z
- **Completed:** 2026-07-30T01:17:00Z
- **Tasks:** 3, plus one carry-forward discharge
- **Files created:** 1 · **modified:** 3
- **Tests:** 668 → 696 passing, 0 failing

## Accomplishments

- **The tab bar's shipped defect is retired, not inherited.** Ten tabs already
  occupied 96 cells, so at an 80-column terminal `9:Cfg` and `0:Docs` were
  silently dropped off the right edge — the "tab bar overflow at 80 columns"
  blocker carried in `STATE.md` since Phase 12. Appending an 11th would have
  taken the bar to 107 cells with the Driver tab *last*, i.e. the tab that never
  renders at any common width. `tab_titles` resolves three tiers instead: full
  labels at 107, compact at 77, and below that a contiguous window of compact
  labels that **always contains the active tab**, with DarkGray `‹`/`›` markers
  on whichever side was cut. The function's doc carries the sentence that
  specifies it: *a tab bar that silently drops the active tab is a defect, not a
  tier*.
- **The bar is built once now, not twice.** Both render sites — the one in
  `render` and its duplicate inside `render_main_only`, which `EnqueueScreen` and
  `DriverInjectScreen` paint behind their footers — take their titles from the
  same call. That duplication is exactly how a tier lands on one path and not the
  other, and D-15 names missing one of the six sites as the classic half-landing.
  All six are done: the title vector, `tab_index`, `sub_view_from_index`,
  `switch_to_tab`, both dispatches, and `footer_spans`.
- **The live marker never moves the bar.** The Driver label's final cell holds
  `◆` in Magenta while a run is live and a space otherwise; the cell is always
  present, so starting or stopping a run does not shift ten tabs sideways under
  the user's eye. Inside a project's detail view the dashboard is not visible,
  and *the user must never be unsure whether something is driving their repo* has
  to hold on this screen too.
- **The held-out backstop is a real buffer scrape.** The detail screen is
  rendered into a `TestBackend` at 40, 60, 80 and 120 columns with the Driver tab
  active and the cells are read back, because the original defect was invisible
  to every calculation the code had: the numbers all looked right and the tabs
  were gone. The same discipline is applied a second time to the D-R-P-E-V line
  in the narrower Driver pane — `[E 2/3]` and the trailing `[V]` are asserted
  present at 60, 80 and 120 columns, which is CR-01 / UIFIX-02 in a new location
  getting a real assertion rather than an arithmetic one.
- **The pipeline widget is called, and a parity test says so structurally.**
  `the_driver_tab_pipeline_line_matches_the_pipeline_tabs` asserts the two
  produce the identical text **and** the identical per-span style for the same
  `DiskInference`, so a future reimplementation — however faithful the day it was
  written — fails a test rather than drifting quietly. `grep -c` over the three
  function definitions reports 3 in `detail.rs` and **0** in `driver.rs`. The
  visibility change is a widen to `pub(super)`, not a move; `ARCHITECTURE` M5's
  lift into `state_reader/` is declined for this phase and the decline is
  recorded at the call site so a later reader knows it was considered.
- **The Replit rule is written where it will be tripped over.** Both the module
  header and `render_steps`'s doc state that every status word, glyph, colour and
  sort key derives from `RunVerdict` and `RunRecord.outcome` — evidence Phase 15
  computes from `is_error`, `terminal_reason`, the permission-denial list, the
  exit code and a git/disk snapshot — and never from the agent's own account of
  itself, with the incident named. `run_state_glyph` has no arm that inspects
  prose, and the one occurrence of `ResultMessage` in the file is the doc comment
  forbidding it.
- **Two honest answers replace two plausible-looking numbers.** A run with no
  terminal record and no observation renders `◇ ?`, never `crash`: an absence of
  evidence is not evidence of death, which is the CR-05 collapse in a new place.
  And the cost line reads `not yet reported` until a `cost` record has been read
  for the selected run, rather than `$0.00` — a run's cumulative cost is only
  knowable from its journal, and the run list is deliberately built without
  opening one.
- **The goal comes back exactly as it was given (OBS-03).** Verbatim, sanitised
  through 18-04's `sanitize_render_line`, wrapped to at most three rows then
  ellipsed, with `(none given)` in DarkGray + DIM when none was given. The
  escape-bearing-goal test asserts no `ESC` survives *and* that the prose is
  still shown, so the rule cannot pass by deleting everything.
- **No new dependency, no clippy regression.** `git diff Cargo.toml Cargo.lock`
  is empty; `cargo clippy --all-targets` still reports exactly the 5 pre-existing
  lints, counted through `rtk proxy`.

## Task Commits

1. **Task 1: The 11th tab, all six sites, and a bar that never hides the active tab** — `17454c0` (feat)
2. **Task 2: The two-pane Driver tab, its run list and its run header** — `d6e05e3` (feat)
3. **Task 3: Reuse the D-R-P-E-V row and add the one-row step timeline** — `f588b0a` (feat)
4. **Carry-forward discharge: prune `view_cache` for unregistered aliases** — `e2d1f33` (fix)

## Files Created/Modified

**Created**

- `src/ui/screens/driver.rs` — the Driver tab's render module: the two constants,
  `render_driver_tab`, `driver_panes`, `run_list_row`, `run_state_glyph`,
  `elapsed_label`, `goal_lines`, `render_run_header`, `render_pipeline_row`,
  `render_steps` / `steps_lines`, `render_output_section`, the six run-state
  glyph constants and the pinned copy, plus 15 inline tests including two
  `TestBackend` backstops.

**Modified**

- `src/ui/screens/detail.rs` — `tab_titles` and `windowed_tab_titles` replacing
  `TAB_TITLES` and both duplicated bar constructions; `TAB_COUNT`,
  `DRIVER_TAB_INDEX`, `TAB_BAR_FULL_CELLS`, `TAB_BAR_COMPACT_CELLS`, the marker
  and overflow constants; `driver_live_for`; the `Driver` arm in `switch_to_tab`;
  `Shift+D`; both render dispatches delegating to `driver::render_driver_tab`;
  `driver_footer_spans` and the width-aware `footer_spans` / `build_footer`;
  `ViewportMetrics`, `clamp_scroll`, `StageStatus`, `derive_all_stage_statuses`
  and `build_pipeline_line` widened to `pub(super)`;
  `DetailScreen.driver_viewport`; and 9 new inline tests.
- `src/ui/screens/mod.rs` — `pub mod driver;`; `DriverRunTally` and
  `ProjectViewCache.driver_tally`; `AppContext::schedule_run_list_scan`.
- `src/app.rs` — `update_driver_tally` and its call from the
  `DriverJournalAppended` handler; `App::schedule_run_list_scan` reduced to a
  delegation; `view_cache` added to `prune_driver_maps`; 2 new inline tests.

## Decisions Made

- **`tab_titles` returns the vector and the select index together.** A windowed
  bar whose select index still points into the unwindowed vector highlights the
  wrong tab, which is a worse failure than no highlight at all. The two are
  computed by one function because they cannot be correct independently.
- **The window grows right first, then left.** The active tab is the seed and is
  never given up; if even it alone overflows the bar it is still rendered,
  because a clipped label the user can see beats a correct one they cannot.
- **The Driver footer's full form is selected at 101 columns, not 100.** The
  UI-SPEC's table gives the form's measured width as 101 and its condition as
  `>= 100`, which are inconsistent by one cell — at exactly 100 the trailing `p`
  of `[?]help` falls off the edge. A threshold that does not match the render is
  the `STATUS_COLUMN_MIN_CELLS` bug in miniature, so the threshold *is* the
  measured width and a test keeps them equal.
- **The run-list rescan is unconditional on tab entry.** Unlike the backlog or
  the git log, this list changes while the user is looking away — a run started
  from the dashboard finishes, a new one begins — so a cached-once list would
  show a stale set of runs indefinitely. The scan is bounded by the retention cap
  and runs on `spawn_blocking`.
- **The observed run's verdict is applied only to the row whose id matches.**
  `observed_runs` is keyed by alias and holds the one run the scan is watching;
  applying its verdict to every row would report one run's liveness as another's.
- **`driver_tally` carries its run id inside the value.** That is what makes
  *"this cost is not about the run you are looking at"* expressible, which is the
  honest answer for every row except the one being tailed. A bare
  `Option<f64>` would have shown the live run's cost against a historical one.
- **The output pane's real rendering is deliberately left to 18-10.** What landed
  is the section rule, the `No journal entries yet.` empty state this plan owns,
  the buffered lines as plain text, and the `Cell<ViewportMetrics>` capture the
  clamp depends on. The marker column, the follow bit and its indicator, the
  four-state injection widget and the adopted-run notice are that plan's named
  deliverables, and guessing at them here would have produced two renderings to
  reconcile.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `schedule_run_list_scan` was unreachable from `switch_to_tab`**
- **Found during:** Task 1
- **Issue:** The plan requires the Driver arm in `switch_to_tab` to schedule the
  run-list scan "through 18-05's `schedule_run_list_scan`". That function is an
  `App` method, and `switch_to_tab` is reached from `Screen::handle_key`, which
  is handed an `&mut AppContext` and never an `&mut App`. The call was not
  expressible.
- **Fix:** The body moved to `AppContext::schedule_run_list_scan` in
  `src/ui/screens/mod.rs` and `App::schedule_run_list_scan` became a one-line
  delegation with a doc recording why. One implementation, two call paths.
  `ctx.needs_redraw` is synced into `App::needs_redraw` by the main loop, so the
  refusal path is unchanged in effect.
- **Alternative rejected:** inlining a second `spawn_blocking` closure in
  `switch_to_tab`, matching the git and archive arms. Two copies of a scan that
  resolves a run id through the fallible `run_paths`, reads N `run.json` files
  and tails an inbox is two things to keep in agreement, and the traversal guard
  is one of them.
- **Files modified:** `src/app.rs`, `src/ui/screens/mod.rs`
- **Committed in:** `17454c0`

**2. [Rule 2 - Missing Critical] The cumulative cost and the turn count had no source**
- **Found during:** Task 2
- **Issue:** The plan makes the cumulative cost line mandatory and the turn
  counter part of the steps row, but neither fact is reachable.
  `journal::RunSummary` is built from the committed `run.json`, which carries no
  cost and no turn count; `RunRecord` has no such field either. 18-05's
  `driver_line_for_record` returns `None` for `cost` records on the stated
  grounds that they are *"header data"* — but nothing was reading them as header
  data, so the cost line would have been a UI element that could never light,
  which is the named "a badge that can never light is worse than no badge"
  failure.
- **Fix:** `DriverRunTally { run_id, cumulative_cost_usd, turn_boundaries }` on
  `ProjectViewCache`, folded by `app::update_driver_tally` from the same batch of
  records the ring buffer already receives. Both facts are evidence — a `cost`
  record's `cumulative_usd` and an `exec_event` on the `turn_completed` stream —
  and neither is inferred. The tally resets when the run id changes.
- **Alternative rejected:** rendering `$0.00` until a cost arrives. A zero is a
  figure the mechanism cannot back, and the whole surface's contract is that it
  never asserts a state the evidence does not support. The unknown case says
  `not yet reported` instead.
- **Files modified:** `src/app.rs`, `src/ui/screens/mod.rs`
- **Verification:** `a_journal_batch_tallies_the_cumulative_cost_and_the_turn_boundaries`
  drives a real `Action::DriverJournalAppended` and asserts both the fold and the
  cross-run reset; `the_cost_line_is_labelled_cumulative_and_never_fabricates_a_figure`
  asserts both header forms.
- **Committed in:** `d6e05e3`

**3. [Rule 2 - Missing Critical] `view_cache` was never pruned, and this phase filled it with driver state**
- **Found during:** Task 3 (reviewing the phase's negative carry-forward)
- **Issue:** The carry-forward is that every new per-alias driver map is retained
  by registered alias in `App::prune_driver_maps`. `journal_cursors`,
  `run_states`, `observed_runs` and `driver_output` are all in that pass.
  `ProjectViewCache` is not — and it now holds `driver_runs` (a `RunSummary` per
  run on disk, 18-05), `driver_inbox` (every queued message for the selected run,
  18-04) and `driver_tally` (this plan). The obligation was being met for the
  maps it names and broken for the one it does not.
- **Fix:** `view_cache` joined the retain pass. Dropping the whole entry for an
  unregistered alias is correct rather than merely convenient: the project is
  gone, so every view state it held describes something the user can no longer
  open.
- **Files modified:** `src/app.rs`
- **Verification:** `pruning_drops_the_view_cache_for_an_unregistered_alias`, with
  a control arm asserting a registered alias keeps its cache **and** its
  contents — a prune that emptied every entry would otherwise satisfy a
  `contains_key` assertion.
- **Committed in:** `e2d1f33`

**4. [Rule 1 - Bug] The UI-SPEC's full-footer threshold is one cell below the form it selects**
- **Found during:** Task 1
- **Issue:** The spec's Driver footer table gives the full form's measured width
  as 101 cells and its condition as `width >= 100`. At exactly 100 columns the
  form is one cell too wide and the trailing `p` of `[?]help` is clipped.
- **Fix:** `DRIVER_FOOTER_FULL_CELLS = 101`, with the discrepancy recorded on the
  constant and `each_driver_footer_form_fits_the_width_that_selects_it` keeping
  the threshold equal to the render.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `17454c0`

**5. [Rule 3 - Blocking] `driver_viewport` landed in Task 2 rather than Task 1**
- **Found during:** Task 1
- **Issue:** Task 1's action adds `DetailScreen.driver_viewport` alongside the
  three existing viewport cells, but nothing in Task 1 reads it — `driver.rs`
  does not exist yet — so `rustc` reports `field is never read` and the task's
  own `cargo clippy -- -D warnings` acceptance criterion fails.
- **Fix:** The field lands in Task 2's commit, where `render_driver_tab` records
  the output pane's metrics into it. The `pub(super)` widening of
  `ViewportMetrics` and `clamp_scroll` — the criterion Task 1 actually greps for
  — is unchanged and landed in Task 1.
- **Alternative rejected:** an `#[allow(dead_code)]` for one commit. A lint
  suppression that has to be remembered and removed is worse than a field that
  arrives with its first reader.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `d6e05e3`

**6. [Rule 3 - Blocking] Task 3's content was held back from Task 2's commit**
- **Found during:** Task 2
- **Issue:** `render_driver_tab` is one function and its vertical layout names
  every section, so writing Task 2 and Task 3 together was the natural order —
  but the plan makes them separate commits and a commit whose sections are
  present but unreachable is not atomic.
- **Fix:** Task 2's commit lays the header and the output section with a comment
  naming Task 3 as what inserts the pipeline row and the step timeline between
  them; Task 3's commit adds both sections, the two width-tier chunks, the
  visibility widen and the parity and backstop tests. Both commits build, test
  and pass clippy on their own. 18-07 recorded the same ordering discipline.
- **Files modified:** `src/ui/screens/driver.rs`
- **Committed in:** `d6e05e3`, `f588b0a`

**7. [Rule 2 - Missing Critical] `run_list_row` needs the pane's inner width**
- **Found during:** Task 2
- **Issue:** The plan's artifact list gives the signature as
  `run_list_row(summary, verdict) -> Line`, while the same sentence requires "the
  fixed 18-cell row plus the state word above 26 cells" and the task's own test
  requires the word to appear at 26 cells of inner width and not at 25. A
  function with no width parameter cannot satisfy that.
- **Fix:** `run_list_row(summary, verdict, inner_width)`. The width is read from
  `Block::inner` at the call site, so the threshold is measured against the same
  rectangle the list actually renders into rather than against the outer area.
- **Files modified:** `src/ui/screens/driver.rs`
- **Committed in:** `d6e05e3`

---

**Total deviations:** 7 auto-fixed (3 blocking, 3 missing critical, 1 bug).
**Impact on plan:** No scope creep and no new dependency. Deviations 2 and 3 add
production wiring the plan's own mandatory UI elements required; deviations 5 and
6 are commit-ordering only and change nothing about what shipped. Every
acceptance criterion in all three tasks is met.

## Issues Encountered

- **The honest note is clipped by one cell at exactly 60 columns.** The detail
  pane's floor is `DRIVER_DETAIL_MIN_CELLS` (39), derived from the pipeline line;
  `  This version runs one command per run.` is 40 cells, so its final period
  falls off. The buffer backstop asserts the note by prefix and records the
  reason. The floor protects the element it was derived from — the pipeline line,
  whose clipping was a real bug — and the sentence remains fully legible. Raising
  the floor to 40 to save a period would widen the detail pane at the run list's
  expense at every width.
- **`rg` is not installed on this machine**, so every acceptance-criterion grep
  was run through `grep` with equivalent patterns. The counts are recorded in the
  Verification table below.
- **`tests/driver_reattach.rs` did not flake during this plan**, and was run in
  isolation afterwards to confirm rather than assumed — 3 passed.
- **Every count in this summary was taken through `rtk proxy`.** Plain `cargo`
  output is filtered by the summarising wrapper, so a criterion grepping for
  `warning:` or `test result:` without it passes **vacuously**. This bit Phase 15
  and Phase 17 and would have made the clippy-count claim below meaningless.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **696 passed, 0 failed** (baseline 668; 593 lib + 103 across 18 integration binaries) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `rtk proxy cargo test --test driver_reattach` (isolated) | pass (3) |
| `grep -c 'Tabs::new' src/ui/screens/detail.rs` | **2**, and `grep -B4` shows `tab_titles` above each |
| `grep -c 'DetailSubView::Driver' src/ui/screens/detail.rs` | **15** — well past the required 5 |
| `grep -n "Char('D')" src/ui/screens/detail.rs` | matches — Shift+D is bound |
| `grep -n 'pub(super) fn clamp_scroll\|pub(super) struct ViewportMetrics'` | both match — one clamp formula, shared |
| `grep -n 'pub mod driver;' src/ui/screens/mod.rs` | matches |
| `grep -c 'Constraint::Percentage(60)' src/ui/screens/driver.rs` | **0** — the floor is a `Min`, and the doc explaining why avoids the literal so the grep is non-vacuous |
| `grep -c 'none given' src/ui/screens/driver.rs` | **1** — one constant, referenced everywhere including the tests |
| `grep -c 'sanitize_render_line' src/ui/screens/driver.rs` | **6** — every disk-sourced string in the header |
| `grep -c 'fn build_pipeline_line\|fn derive_all_stage_statuses\|fn stage_color'` | **3** in `detail.rs`, **0** in `driver.rs` |
| `grep -B12 'fn render_steps' \| grep -c 'D-13'` | **1** — the evidence-only rule is at the function |
| `git diff --diff-filter=D` across all four commits | empty — no file was deleted |

### Threat register

| Threat ID | Disposition | Evidence |
|---|---|---|
| T-18-48 (a status word or glyph derived from agent prose) | **mitigated** | `run_state_glyph` maps only from `RunVerdict` and `RunRecord.outcome`; `every_run_state_maps_from_evidence_and_unknown_is_not_death` pins the whole table including the two "nothing is known" answers; the module header and `render_steps`'s doc both state the rule with the incident named; the single `ResultMessage` occurrence in the file is the doc forbidding it |
| T-18-49 (escape sequences in a goal or command reaching the header) | **mitigated** | The goal, the command and the run directory row all pass through `sanitize_render_line`; `a_goal_bearing_an_escape_sequence_is_sanitised_before_it_becomes_a_line` feeds a clear-screen and an OSC window-title set and asserts no `ESC` survives **and** that the prose is still shown |
| T-18-50 (the run directory row leaking a path outside the project) | **mitigated** | The row is built from `journal::run_paths`, whose `Option` refuses a non-plain run id before any join (D-27); `the_run_directory_row_keeps_its_tail_when_it_does_not_fit` asserts the truncation keeps the run id rather than the leading path |
| T-18-51 (an unbounded goal or command wrapped into the header) | **mitigated** | The goal wraps to at most three rows then ellipses and the command is ellipsis-truncated, both by `char` count so a multibyte boundary cannot panic; `a_long_goal_wraps_to_at_most_three_rows_then_ellipses` drives a 1000-character goal |
| T-18-52 (an invisible active tab misleading the user about which view they are in) | **mitigated** | `the_active_tab_label_is_always_present_in_the_rendered_bar` scrapes a `TestBackend` buffer at 40, 60, 80 and 120 columns for two different active tabs and also asserts an overflow marker wherever the bar was truncated |
| T-18-53 (package-manager installs) | **accepted** | Zero dependencies added; `Cargo.toml`/`Cargo.lock` byte-identical to the base |

## Known Stubs

Each is a declared seam with a named owner. None asserts a state the mechanism
cannot back.

1. **The output pane renders buffered lines as plain text.** No marker column, no
   follow indicator, no injection rows, no adopted-run notice, no ring-overflow
   affordance. **Owner: 18-10**, which names every one of those as its own
   deliverable. What landed is the section rule, the `No journal entries yet.`
   empty state this plan owns, and the `Cell<ViewportMetrics>` capture 18-10's
   clamp reads — so nothing promises output it does not show, and the pane says
   plainly when there is none.
2. **The Driver tab's key routing is absent.** `j`/`k`, `PageUp`/`PageDown`, `f`,
   `G`, `i`, `s` and `x` are not bound; the run selection is fixed at whatever
   `driver_selected_run` holds, which is 0 until something writes it. **Owner:
   18-10**, whose artifact list names the arms explicitly. `Shift+D`, `Left`,
   `Right` and the digits all work, so the tab is reachable and the newest run's
   detail renders.
3. **The step timeline's turn counter needs a live tail.** `driver_tally` is
   populated only while `Action::DriverJournalAppended` is arriving for the
   selected run, so a historical run shows no turn count — correctly, since its
   turn count is not on disk anywhere the run list reads. No UI claims otherwise.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **18-10** inherits a rendered frame: `render_driver_tab` with its three width
  tiers and three height tiers, `DetailScreen.driver_viewport` already recording
  the output pane's metrics through the shared `clamp_scroll`, the two-pane
  split, and `run_state_glyph`'s evidence table to extend into the exhaustive
  `terminal_state_cell`. The two sections it owns — the output pane and the key
  routing — have their layout slots and their `Cell` waiting.
- **The clamp-ordering invariant is untouched.** No new scroll handler was added;
  `clamp_scroll` gained visibility and nothing else, and its four existing tests
  plus the `detail.rs:5046-5204` block still pass unmodified.
- **Requirements.** `OBS-03` is marked complete: the originating goal prompt is
  now readable for any driven project, verbatim, on a tab the user can reach.
  `TRANS-05` and `OBS-04` are deliberately **not** marked — "watches its output"
  is not true until 18-10 renders the pane and binds the keys, and marking them
  here would assert a capability the user cannot exercise.
- **The negative carry-forward is fully discharged and slightly widened.** Every
  per-alias driver map this phase added is now retained by registered alias in
  `App::prune_driver_maps`, including `view_cache`, which was outside the pass
  until this plan put driver payloads in it. A plan that adds another map
  inherits the obligation afresh.

## Self-Check: PASSED

All four files present on disk (`src/ui/screens/driver.rs` created;
`src/ui/screens/detail.rs`, `src/ui/screens/mod.rs`, `src/app.rs` modified); all
four commits present in `git log` (`17454c0`, `d6e05e3`, `f588b0a`, `e2d1f33`);
`git diff --diff-filter=D HEAD~4 HEAD` empty — no file was deleted; working tree
clean apart from this summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-30*
