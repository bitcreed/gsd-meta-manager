---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 06
subsystem: ui
tags: [roadmap, ratatui, detail-view, escape-guard, fixtures, layout]
status: complete
requires:
  - phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
    provides: "24-01 reader facts and real-roadmap fixtures; 24-02 list model; 24-03 tab order and the PhaseList removal; 24-04 RoadmapView widget; 24-05 roadmap_model_for, the cursor cells and fixture_state"
provides:
  - "render_roadmap draws RoadmapView over roadmap_model_for with the stored cursor; it writes roadmap_list_offset and roadmap_list_viewport back"
  - "roadmap_summary_line plus a borderless header (Path, then the unreadable, recovered, Paused and change-banner lines only when present)"
  - "roadmap_view::panes_for: a stacked split that gives the list its rows first"
  - "draw_items shortens the goal (marked with an ellipsis) before it cuts the Needs, Unblocks or Parallel lines"
  - "Test helpers render_detail_to_text_at, roadmap_list_pane, rows_numbered, row_numbered, summary_line, line_with, two_phase_roadmap"
  - "Escape probe covers phase_goals, planned_phases and milestone_name, with two new RoadmapViz sub-states"
  - "roadmap_graph.rs reduced to the dependency pipeline plus the list model"
affects: [24-07]
plan_head_before: 161101aa261f7e33737045712aa023857d4e2f0b
actuals:
  tokens: 25135
  tasks: 3
  commits: 4
tech-stack:
  added: []
  patterns:
    - "Content-aware stacked split: the detail pane gives rows (from 9 down to 6) to a list that needs them; panes(area) == panes_for(area, 0)"
    - "Shrink order for a short detail pane: drop the hint, then shorten the wrapped goal, then cut rows from the bottom"
    - "Screen-level fixture tests locate the list pane by its ' Roadmap ' title and corners, never by fixed coordinates"
key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/ui/roadmap_view.rs
    - src/ui/roadmap_graph.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/mod.rs
key-decisions:
  - "At 80x24 the stacked detail pane shrinks toward 6 rows when the list needs the rows, and the goal is shortened before any edge line. This keeps daily-vow's v1.5 band on screen while 23 is selected."
  - "The summary line switches to Mockup C's `k/n done` when it is wider than the terminal"
  - "The side-by-side no-deps note drops its Parallel indent when the indent would split the phrase"
  - "The escape probe reaches milestone_name through a `synthetic band` sub-state that clears the fixture's milestones, because only a project with no active roadmap milestone draws milestone_name"
requirements-completed: [D-A01, D-A02, D-A03, D-A08, D-A09, D-A12, D-A14, D-A15, D-B02, D-B08, D-B12]
coverage:
  - id: D1
    description: "The Roadmap tab draws the header, the lane list and the detail pane from live state; v still toggles the box view"
    requirement: D-A01
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_graph_tab_draws_the_graph_by_default_and_v_toggles_the_box_list"
        status: pass
    human_judgment: false
  - id: D2
    description: "SC-1 on real data: daily-vow 20 and sentriq 9 each on one row, one row per band label, no legacy arrow, implied deps as a dim marker with (implied via N)"
    requirement: D-A08
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#daily_vow_roadmap_renders_phase_20_once_at_120, sentriq_roadmap_at_80_columns_is_stacked_and_shows_9_once, sentriq_implied_dep_is_a_dim_marker_not_a_row, sentriq_and_daily_vow_hold_at_both_widths"
        status: pass
    human_judgment: false
  - id: D3
    description: "sentriq phase 12 reads nothing declared / can run any time at both widths (D-A14)"
    requirement: D-A14
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#sentriq_phase_12_explains_it_has_no_deps"
        status: pass
    human_judgment: false
  - id: D4
    description: "ttbook build phases 14-18 are list rows under bands M3/M4/M5; G selects 18, whose detail pane says planned (not a GSD phase) (SC-4)"
    requirement: D-A12
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#ttbook_build_phases_are_list_rows_not_bands"
        status: pass
    human_judgment: false
  - id: D5
    description: "80x24 stacked and 120x30 side by side, with no line wider than the terminal; a short detail pane keeps its edge lines"
    requirement: D-A09
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#sentriq_and_daily_vow_hold_at_both_widths, a_short_stacked_detail_pane_shortens_the_goal_not_the_edges, the_summary_line_compacts_its_count_when_too_wide; src/ui/roadmap_view.rs#panes_for_gives_the_list_its_rows_before_the_detail_pane_shrinks"
        status: pass
    human_judgment: false
  - id: D6
    description: "The former PhaseList content is on the Roadmap: summary line, Path, the Paused, change-banner, unreadable and recovered lines, the [stage] badge, and both empty-state wordings"
    requirement: D-B08
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_header_keeps_paused_and_change_banner, the_selected_phase_shows_its_stage_badge, roadmap_empty_states_explain_themselves"
        status: pass
    human_judgment: false
  - id: D7
    description: "A disk-Complete phase whose ROADMAP box is unticked draws the done glyph (D-B12)"
    requirement: D-B12
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_disk_complete_phase_with_an_unticked_box_draws_done"
        status: pass
    human_judgment: false
  - id: D8
    description: "The escape probe covers goal, planned build-phase name and milestone_name; the planned phase arrives whole; the legacy graph is deleted"
    requirement: D-A15
    verification:
      - kind: unit
        ref: "cargo test --lib ui::screens::render_escape_guard (15 passed, incl. the_planned_phase_arrives_whole_in_the_roadmap_detail_pane)"
        status: pass
      - kind: other
        ref: "git grep -F -e RoadmapGraphWidget -e 'fn layout_graph' -e 'fn layout_for_state' -e 'fn split_areas' -e 'fn milestone_decorations' -e 'Segment::Reference' -- src  -> no output"
        status: pass
    human_judgment: false
  - id: D9
    description: "How the new tab reads on a real terminal at 80x24 and 120x30: density, colours, the 6-row stacked detail pane for long lists, and the shortened goal"
    verification: []
    human_judgment: true
    rationale: "Visual quality and whether a shortened goal is an acceptable trade at 80x24 are subjective and need phase-level UAT on the real screen"
duration: 16 min
completed: 2026-09-24
---

# Phase 24 Plan 06: The new Roadmap on screen Summary

**The Roadmap tab now shows the 24-04 lane list and detail pane over `roadmap_model_for`, below a borderless header built from the former PhaseList content. Fixture tests on daily-vow, sentriq and ttbook, at 80×24 and 120×30, pin that each phase has one row, each band label one row, implied deps are a dim `·`, and build phases are rows. The escape probe now covers goals, planned phases and `milestone_name`. About 1,500 lines of the legacy left-to-right graph are deleted.**

## Performance

- **Duration:** about 16 min
- **Started:** 2026-09-24T04:12:58Z
- **Completed:** 2026-09-24T04:29:19Z
- **Tasks:** 3 (1 tracer, 1 TDD, 1 auto)
- **Files modified:** 5

## Accomplishments

- **`render_roadmap`** (`detail.rs`) builds `roadmap_model_for(state, cache, gsd_integration)`. It renders `RoadmapView { model, cursor: stored cursor }` with `render_stateful_widget`, seeding `RoadmapViewState` from `roadmap_list_offset`. It stores `offset` and `list_rows` back into `roadmap_list_offset` and `roadmap_list_viewport`, which gives PageUp/PageDown a real page size. It does no I/O. The box view (`v`) is unchanged.
- **Header** (D-B02, D-B08). It has no border.
  - Line 1 is `roadmap_summary_line`: `{alias} · {milestone} · phase {n} {status} · {k} of {n} phases done`. The milestone is the active roadmap label, else STATE.md's `{milestone} {milestone_name}`. The status takes its status colour.
  - Line 2 is `Path:`.
  - The unreadable, recovered, Paused and change-banner lines appear only when present.
  - With no project state the tab reads `No state data available for this project.`. With no phases it reads `No roadmap data available`, from the widget.
- **80×24 budget.**
  - `roadmap_view::panes_for(area, list_rows)`: in the stacked form, the detail pane gives rows to the list, from 9 down to 6.
  - `draw_items` drops the hint first. Then it shortens the wrapped goal, marked with `…`, before it cuts any Needs, Unblocks or Parallel line.
  - Too wide for the terminal, the summary line switches to Mockup C's `k/n done`.
- **Escape probe.** `hostile_project_state` now carries `phase_goals` for phase 1 and for planned phase `90`, one `planned_phases` entry named with the identity, and `milestone_name`. There are two new sub-states:
  - `RoadmapViz tab, cursor on the planned phase` takes the cursor key from the fixture's own planned phase.
  - `RoadmapViz tab, synthetic band` clears the milestones, so the probe draws `milestone_name`.

  Both have arrival rows. The `RoadmapViz tab` row now says what the list and the detail pane draw. A new test checks that the planned phase's name and goal arrive whole, clean and hostile.
- **Legacy removal** (D-A15). `roadmap_graph.rs` went from 3,683 lines to about 2,250. It keeps `esc`, `ParentLists`, `ExternalLists`, `resolve_deps`, `break_cycles`, `MAX_CYCLES_SHOWN`, `cycle_note`, `longest_path_layers` and the whole list model. The module doc now describes only what remains.

## Task Commits

1. **Task 1 (tracer): the header, RoadmapView and cursor; daily-vow shows 20 once.** `9753069` (feat). The tracer gate re-ran both `<verify>` commands before expanding: 1 passed, and 3 passed.
2. **Task 2 (TDD): defects stay gone on three fixtures at two widths.** RED `6d99b66` (test), GREEN `8dac3d1` (feat).
3. **Task 3: escape-guard coverage, then delete the legacy layout.** `f69f152` (feat).

## TDD Gate Compliance

- **RED `6d99b66`:** 2 of the 9 new tests failed on assertions.
  - `sentriq_and_daily_vow_hold_at_both_widths` failed because daily-vow's `v1.5` band scrolled off at 80×24.
  - `sentriq_phase_12_explains_it_has_no_deps` failed because at 120 columns the note wrapped mid-phrase as `can run any` / `time)`.
  - The cargo output was converted to TAP and checked with `check tdd-red-evidence`, targeting `sentriq_and_daily_vow_hold_at_both_widths`. The verdict was `RED_EVIDENCE_OK` (`target_test_failed`).
  - The other 7 already passed at RED. They pin behaviour that Task 1 and plans 24-04/24-05 had already delivered: the stage badge, D-B12's glyph, the empty states, the header lines, ttbook's rows and the stacked split. Their job is to keep it from regressing (T-24-21).
- **GREEN `8dac3d1`:** every `ui::` test passes. No refactor commit was needed.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 49 `test result:` lines. **2333 passed, 1 failed, 15 ignored.**
  - The count is 2339 − 22 deleted legacy tests + 16 new tests.
  - The only failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
  - All five archive regression tests pass: `archive_milestone_view_keeps_its_content_across_the_periodic_prune`, `the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry`, `archive_milestone_view_reloads_in_place_when_planning_files_change`, `the_archive_milestone_list_picks_up_a_newly_archived_milestone_in_place` and `an_in_place_reload_that_shrinks_the_listing_keeps_the_cursor_on_a_row`.
- `rtk proxy cargo clippy -- -D warnings` (lib): exit 0.
- `rtk proxy cargo clippy --all-targets --keep-going -- -D warnings`: 11 errors, the same as the baseline. They are all in `src/browser.rs` (3), `src/project_creator.rs` (1) and `tests/envelope_*.rs` (7).
- Acceptance greps:
  - The legacy-layout `git grep` prints nothing.
  - `render_escape_guard.rs` mentions `phase_goals`.
  - `detail.rs` has no `layout_for_state`, `split_areas` or `RoadmapGraphWidget`.
  - `render_roadmap` contains no `std::fs` or `read_to_string` call.

## Ported and deleted tests

**Ported PhaseList-header tests:** none existed. A search for `Paused (HANDOFF`, `STATE.md unreadable`, `repaired in memory` and `latest_change` in the tests found no detail-screen test for those lines. The `app.rs` hits are dashboard-cell tests. `roadmap_header_keeps_paused_and_change_banner` now covers all four lines on `RoadmapViz`.

**Deleted legacy tests (22) and the tests that now cover each:**

| Deleted | Covered by |
|---|---|
| `roadmap_graph_o1_chain` | `lanes_daily_vow_without_bands`, `list_large_chain_stays_one_lane` |
| `roadmap_graph_o2_roots_only` | `lanes_roots_zig_zag`, `list_many_roots_zig_zag_within_two_lanes` |
| `roadmap_graph_o3_cycle_terminates_with_note` | `list_cycle_terminates_with_one_note` |
| `roadmap_graph_o3b_self_dependency` | `list_self_dependency_is_noted` |
| `roadmap_graph_wr01_cycle_notes_collapse_into_one_line` | **ported** as `list_many_cycles_collapse_into_one_bounded_note` |
| `roadmap_graph_wr01_footer_never_takes_the_whole_body` | **ported** as `roadmap_view::the_cycle_note_takes_one_list_row_and_never_the_list` |
| `roadmap_graph_o4_external_dep` | `list_external_dep_is_listed_not_drawn` |
| `roadmap_graph_o4b_padded_and_decimal_ids` | `list_padded_and_decimal_ids_match` |
| `roadmap_graph_o5_skip_layer_edge_is_a_reference_row` | `list_skip_layer_edge_is_implied_not_repeated`, `daily_vow_phase_20_has_exactly_one_row` |
| `roadmap_graph_o6_current_phase_detail_line` | `default_cursor_is_the_active_phase`, `facts_for_mockup_b_phase_23` |
| `roadmap_graph_o7_context_example` | `lanes_mockup_a_with_bands` |
| `roadmap_graph_o8_mixed_width_layer_pads_with_dashes_and_fans_in` | `list_fan_in_merges_once` |
| `roadmap_graph_o9_blocked_fan_in_becomes_a_reference_row` | `list_fan_in_merges_once`, `list_skip_layer_edge_is_implied_not_repeated` |
| `roadmap_graph_o10_example_with_milestones_end_to_end` | `lanes_mockup_b_with_shipped_summary`, `band_labels_appear_on_exactly_one_row` |
| `roadmap_graph_o7_without_milestones_has_no_decorations` | `lanes_daily_vow_without_bands`, `empty_bands_never_draw` |
| `roadmap_graph_escapes_milestone_labels_in_header_and_row_end` | `list_model_stores_only_escaped_text` |
| `roadmap_graph_escapes_the_detail_line_and_external_deps` | `list_model_stores_only_escaped_text`, `roadmap_view_stores_and_draws_only_escaped_text` |
| `roadmap_graph_example_never_panics_at_tiny_sizes` | `roadmap_view_never_panics_at_tiny_sizes` |
| `roadmap_graph_never_panics_or_bleeds_at_tiny_sizes` | `roadmap_view_never_panics_at_tiny_sizes`, `roadmap_view_never_draws_outside_its_rect` |
| `roadmap_graph_decimal_node_is_ordinary` | `list_padded_and_decimal_ids_match` |
| `roadmap_graph_widget_text_matches_render_text` | the `lanes_*` `lane_text` tests and the `mockup_*` widget tests |
| `roadmap_graph_auto_offset_keeps_the_current_node_visible` | `the_cursor_row_is_scrolled_into_view` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] daily-vow's band scrolled off at 80×24.**
- **Found during:** Task 2 RED.
- **Issue:** At 80×24, with a 2-row header, the stacked split gave the list 9 rows, so only 5 model rows were visible. With 23 selected, the `v1.5` band was not shown.
- **Fix:** Added `roadmap_view::panes_for(area, list_rows)`. The stacked detail pane now gives rows to a list that needs them, from 9 down to 6. `panes(area)` is still `panes_for(area, 0)`, byte-for-byte the old split. `draw_items` now shortens the wrapped goal, marked with `…`, before it cuts Needs, Unblocks or Parallel, so a 6-row pane still shows the edges.
- **Files modified:** `src/ui/roadmap_view.rs`, which is outside `files_modified`. The split lives in the widget.
- **Commit:** `8dac3d1`

**2. [Rule 1 - Bug] The side-by-side no-deps note broke mid-phrase.**
- **Found during:** Task 2 RED.
- **Issue:** Under the 10-cell Parallel indent, `(no edge either way; can run any time)` wrapped as `can run any` / `time)` in the 44-cell pane.
- **Fix:** The note keeps the indent only when it fits whole. Otherwise it starts at the pane's left edge.
- **Commit:** `8dac3d1`

**3. [Rule 1 - Bug] The summary line was cut mid-word at 80 columns** (ttbook: `0 of 11 phases d`).
- **Fix:** When the line is wider than the terminal, the count uses Mockup C's `k/n done`.
- **Commit:** `8dac3d1`

**4. [Rule 3 - Blocking] `width_of` removed from `roadmap_graph.rs`.**
- **Issue:** The plan's keep-list names it, but its only callers were in the deleted legacy half. Leaving it would raise a dead-code warning, and `clippy -D warnings` must stay clean.
- **Commit:** `f69f152`

### Additions beyond the plan text

- **A third probe sub-state, `RoadmapViz tab, synthetic band`.** Without it `milestone_name` never reaches a cell in the probe, because the hostile fixture's milestone is active. The plan's truth lists "`milestone_name` synthetic band" as a path the probe must cover.
- **Two ported WR-01 tests and three extra tests:** `a_short_stacked_detail_pane_shortens_the_goal_not_the_edges`, `the_summary_line_compacts_its_count_when_too_wide` and `panes_for_gives_the_list_its_rows_before_the_detail_pane_shrinks`.
- **Stale wording updated:**
  - "graph" became "list" in the `DetailScreen` adjudication reason, the `v` key comment, `roadmap_list_active`'s doc, and the `roadmap_box_view` doc in `src/ui/screens/mod.rs`. That last file is outside `files_modified`, and the change there is a doc comment only.
  - The help row `Roadmap tab: graph / box view` was left byte-identical. 24-05 pins it.

**Total deviations:** 4 auto-fixed (3 layout bugs, 1 blocking lint). **Impact:** all four were needed to meet the plan's own 80×24 and 120×30 assertions or to keep clippy clean. No scope creep beyond the two files noted.

## Inferred decisions (for audit)

1. **Stacked split floor of 6 rows.** That is the pane head, one goal row, Needs and Parallel inside the borders. At 80×24 daily-vow gets a 6-row detail pane and sentriq gets 8. Mockup C's 9 is kept whenever the list fits. The alternative was to drop `Path:` or pin the band row. Dropping `Path:` breaks the D-B08 must-have, and pinning the band row adds a new list concept.
2. **Goal shortening order:** hint first, then the goal (at least one row, marked with `…`), then rows from the bottom. Needs, Unblocks and Parallel are the pane's point (D-A02), and the goal is prose.
3. **The summary line's active phase** is `state.active_phase_number()`, the same source `phase_marker` uses. The status is left out when it is empty, so there is no trailing space.
4. **The header has one leading space and no border.** The Paused, unreadable and recovered lines keep their existing two-space wording.
5. **No-state wording** is `No state data available for this project.` inside a bordered block, and the old `...for roadmap.` string is gone (D-B08).
6. **Planned-phase escape check:** the escaped hostile identity (`demo` + tag char + soft hyphen) is far narrower than the 54-cell name line at the probe's 200 columns. So the name line does not wrap, and a test pins that it arrives whole. No wrap change was needed.
7. **The planned probe phase uses id `90`** (`PROBE_PLANNED_PHASE_ID`), an id no GSD phase in the fixture uses. It has no dependencies, and its goal is keyed through `phase_key`.
8. **Roadmap footer at 80 columns:** `[?]help` is still clipped (91 cells, noted by 24-05). The Git and Config footers overflow the same way. No plan task asks for footer tiering, so it is left as is (see Next Phase Readiness).
9. **`model` is built on every Roadmap render, box view included,** because the summary line's `k of n` needs it. The build is in-memory and O(n·e).

## Known Stubs

None. `roadmap_list_offset` lost its `#[allow(dead_code)]`, because the render reads and writes it now.

## Threat Flags

None. There is no new surface. T-24-19 is covered by the extended probe and the whole-arrival test. For T-24-20, `render_roadmap` has no `std::fs` call, and the model is built in memory. T-24-21 is covered by the fixture render tests.

## Next Phase Readiness

- **24-07:** re-route `Enter` on the shipped summary to Docs › Milestones in `roadmap_activate`. Plan 24-07's renumber does not touch the Roadmap render.
- **Open, not blocking:** at 80 columns the Roadmap footer clips `[?]help`. A footer width tier for all tabs would be a small follow-up.

---
*Phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation*
*Completed: 2026-09-24*

## Self-Check: PASSED

- FOUND: src/ui/screens/detail.rs, src/ui/roadmap_view.rs, src/ui/roadmap_graph.rs, src/ui/screens/render_escape_guard.rs, src/ui/screens/mod.rs
- FOUND commits: 9753069, 6d99b66, 8dac3d1, f69f152
- `commits: 4` was measured with `git rev-list --count 161101a..HEAD` before the docs commit.
