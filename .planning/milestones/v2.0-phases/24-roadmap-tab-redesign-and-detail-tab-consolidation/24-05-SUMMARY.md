---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 05
subsystem: ui
tags: [roadmap, cursor, keybindings, shared-selection, folds, help, footer]
status: complete
requires:
  - phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
    provides: "24-01 reader facts (planned_phases, phase_goals, milestone_name, shipped_milestones, declared_phase_count, fixtures); 24-02 list model and navigation (layout_list, CursorTarget, BandKey, EdgeWalk, step/edge_jump/wave_step/unfold_for); 24-03 tab order (tab_index, Pipeline = Phases)"
provides:
  - "roadmap_model_for(state, cache, show_badges) -> RoadmapModel: the one ProjectState adapter"
  - "ProjectViewCache.{roadmap_cursor, roadmap_fold_toggles, roadmap_edge_walk}"
  - "DetailScreen.{roadmap_list_offset, roadmap_list_viewport} Cells for 24-06's render"
  - "Roadmap keys j/k/Up/Down/PageUp/PageDown/g/G/h/l/[/]/Space/Enter on the list view"
  - "Shared selected phase between Roadmap and Phases in both directions (D-B03)"
  - "Roadmap footer [h/l] edge, [Space] fold; five help rows replacing the stale r row"
  - "Test helpers fixture_state, roadmap_fixture, fixture_model, resolved_cursor"
affects: [24-06, 24-07]
tech-stack:
  added: []
  patterns:
    - "One resolve -> move -> unfold -> store helper (roadmap_nav) behind every Roadmap navigation arm"
    - "Match-guarded RoadmapViz branches (`if roadmap_list`) so the box view falls through to the generic scroll unchanged"
    - "Edge walk cleared once at the top of handle_key for every Roadmap key except h/l"
key-files:
  created: []
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
key-decisions:
  - "Space on a phase folds that phase's band and stores the BAND as the cursor (not the hidden phase), so the cursor visibly rests on the band row (D-A05)"
  - "Orphan phases join the active roadmap milestone when there is one; otherwise a synthetic STATE.md band; with no STATE.md milestone they stay band-less"
  - "Build-phase status message escapes the id with render_for_terminal on the raw RoadmapPhase number (T-24-17)"
requirements-completed: [D-A05, D-A07, D-A10, D-A11, D-A12, D-B03, D-B08, D-B12]
duration: 12 min
completed: 2026-09-24
plan_head_before: 119ab7050ca8331bc5630e334dd3af53665141bc
actuals:
  tokens: 10749
  tasks: 3
  commits: 4
coverage:
  - id: D1
    description: "roadmap_model_for adapter on the three real fixtures (one row per phase, sentriq synthetic band, ttbook build phases as planned phases)"
    requirement: D-A12
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_model_for_daily_vow_has_one_row_for_phase_20, roadmap_model_for_sentriq_uses_a_synthetic_band, roadmap_model_for_ttbook_lists_build_phases_as_phases"
        status: pass
    human_judgment: false
  - id: D2
    description: "Enter on a Roadmap GSD phase opens Phases on it; Phases selection writes the Roadmap cursor back"
    requirement: D-B03
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_enter_opens_the_selected_phase_in_phases, phases_selection_is_shared_back_to_the_roadmap"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every D-A10 key on real fixture data: j/k/g/G/PageUp/PageDown, h cycle, l and [ ], Space fold, jump unfolds, Enter on shipped/band/build phase, box view scroll, edge walk ends, inert without state"
    requirement: D-A10
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_j_k_g_capital_g_move_the_cursor, roadmap_h_cycles_needs_then_implied, roadmap_l_and_brackets_follow_edges_and_waves, roadmap_space_folds_and_the_cursor_moves_to_the_band, roadmap_jump_into_a_folded_band_unfolds_it, roadmap_enter_on_the_shipped_row_opens_the_archive_view, roadmap_enter_on_a_band_toggles_its_fold, roadmap_enter_on_a_build_phase_explains_itself, roadmap_box_view_keeps_generic_scroll, other_roadmap_keys_end_the_edge_walk, roadmap_keys_are_inert_without_project_state"
        status: pass
    human_judgment: false
  - id: D4
    description: "Footer and help document the Roadmap keys; stale r row gone; v row byte-identical"
    verification:
      - kind: unit
        ref: "src/ui/screens/help.rs#the_roadmap_cursor_keys_are_documented_as_whole_rows, the_stale_roadmap_toggle_row_is_gone, the_roadmap_graph_toggle_key_is_documented; src/ui/screens/detail.rs#roadmap_footer_advertises_edges_and_fold"
        status: pass
    human_judgment: false
  - id: D5
    description: "No mouse handling added (D-A11)"
    requirement: D-A11
    verification:
      - kind: command
        ref: "git diff 119ab70 -- src/ui/screens/ | grep '^+' | grep -c Mouse  -> 0"
        status: pass
    human_judgment: false
---

# Phase 24 Plan 05: Roadmap cursor, keys and shared selection Summary

**The Roadmap tab now has a cursor. One adapter, `roadmap_model_for`, turns a `ProjectState` into the 24-02 list model. That covers GSD phases plus ttbook's build phases, milestone bands, sentriq's synthetic `v0.12 Actuation Routines` band, markers, plan counts, goals and the `[stage]` badge. Every D-A10 key moves, jumps, cycles, wave-steps or folds a key-based cursor stored per project. `Enter` hands a phase to the Phases tab, and the Phases tab writes its selection back.**

## Performance

- **Duration:** about 12 min
- **Started:** 2026-09-24T03:57:23Z
- **Completed:** 2026-09-24T04:09:54Z
- **Tasks:** 3 (1 tracer, 1 TDD, 1 auto)
- **Files modified:** 3

## Accomplishments

- **`roadmap_model_for`** (`detail.rs`):
  - The nodes are `state.phases`, then `state.planned_phases`, deduplicated by `phase_key` so a GSD phase wins.
  - A phase's band comes from `milestone_index_of`. A phase with no milestone goes to the active milestone, or else to a synthetic STATE.md band.
  - Shipped flags come from `shipped_milestones`, and declared counts from `declared_phase_count`.
  - The marker is `state.phase_marker` (D-B12). The badge is `disk_suffix_spans`, concatenated and trimmed (D-B08). It no longer needs `#[allow(dead_code)]`.
  - The function does no I/O.
- **Cursor state** on `ProjectViewCache`: `roadmap_cursor: Option<CursorTarget>`, `roadmap_fold_toggles: HashSet<BandKey>` and `roadmap_edge_walk: Option<EdgeWalk>`. `DetailScreen` gains `roadmap_list_offset` and `roadmap_list_viewport` Cells for 24-06's render to write.
- **Keys**, active on the list view only:
  - `j`/`k`/`Up`/`Down` move one row. `PageUp`/`PageDown` move by `roadmap_list_viewport - 1`, at least 1. `g`/`G` go to the first/last row.
  - `h`/`l` follow edges, and repeating the key cycles the origin's needs then implied deps. `[`/`]` step within the wave and wrap.
  - `Space` folds the band under the cursor, or the cursor phase's own band.
  - `Enter`:
    - on a band, toggles its fold;
    - on the shipped summary, opens `tab_index(&DetailSubView::Archive)`;
    - on a build phase, returns a status message saying it is a "planned placeholder";
    - on a GSD phase, sets `pipeline_selected` (found by `phase_key`) and switches with `tab_index(&DetailSubView::Pipeline)`.
  - A jump into a folded band unfolds it (`unfold_for`).
  - Every Roadmap key except `h`/`l` ends the edge walk.
- **Box view** (`v`) keeps the old generic scroll. `g`/`h`/`l`/`[`/`]`/`Space`/`Enter` do nothing there. `v`, `Left`/`Right`, `e` and the digits are unchanged.
- **Shared selection back:** the Phases tab's `j`/`k`/`PageUp`/`PageDown` write `roadmap_cursor = Some(CursorTarget::Phase(phase_key(..)))` through `share_pipeline_selection`, which clamps a stale index the same way the render does.
- **Footer:** the Roadmap footer now reads `[h/l] edge  [Space] fold  [v]iew  [e]nqueue`. **Help:** five whole Roadmap rows replace the stale `r` row, and the `v` row is byte-identical.

## Task Commits

1. **Task 1 (tracer): Enter on a Roadmap phase opens it in Phases.** Adapter, cursor state and shared selection: `a8fc2c6` (feat). The tracer gate re-ran both `<verify>` commands (7 passed and 1 passed) before expanding.
2. **Task 2 (TDD): every Roadmap key.** RED `d3d35b2` (test), then GREEN `9f03b30` (feat).
3. **Task 3: footer hints and help rows.** `91da642` (feat).

## TDD Gate Compliance

- RED `d3d35b2`: 9 of the 11 new tests failed, all on assertions. The two that passed at RED, `roadmap_box_view_keeps_generic_scroll` and `roadmap_keys_are_inert_without_project_state`, pin behaviour that has to hold both before and after the change, so passing at RED is intended. The cargo output was converted to TAP and run through `check tdd-red-evidence`, with target `roadmap_j_k_g_capital_g_move_the_cursor`. The verdict was `RED_EVIDENCE_OK` (`target_test_failed`).
- GREEN `9f03b30`: 151/151 `ui::screens::detail` tests pass. No refactor commit was needed.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 49 `test result:` lines; **2339 passed, 1 failed, 15 ignored**. That is the 2320 baseline plus 19 new tests. The only failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The five `app::tests` archive regression tests pass.
- `rtk proxy cargo clippy -- -D warnings` (lib): exit 0.
- `rtk proxy cargo clippy --all-targets --keep-going -- -D warnings`: 11 errors, the same as the baseline. They are all in `src/browser.rs` (3), `src/project_creator.rs` (1) and `tests/envelope_*.rs` (7). None is new.
- `git diff 119ab70 -- src/ui/screens/` adds no line containing `Mouse` (D-A11).
- `git grep 'tab_index(&DetailSubView::Pipeline)' src/ui/screens/detail.rs` finds the handoff inside `roadmap_activate`, which the RoadmapViz `Enter` branch calls.

## Deviations from Plan

### Structural choices within the plan

- **Task 1's tracer Enter helper (`roadmap_enter`) became `roadmap_activate` in Task 2.** It now handles `Enter` and `Space` for all four target kinds. The Enter-arm branch is `DetailSubView::RoadmapViz if roadmap_list => self.roadmap_activate(code, ctx)`.
- **Edge-walk clearing is one statement at the top of `handle_key`,** not a line in every arm. It applies whenever the current view is RoadmapViz and the key is not `h`/`l`, which is exactly the rule in the must-have ("any key other than h/l ends an edge walk"), and it also covers `v`, `Left`/`Right` and the digits.
- **Test update (planned):** `generic_fixture` now sets `roadmap_box_view = true`. The graph view's `j`/`k`/`PageUp`/`PageDown` move the cursor now, so the generic-scroll tests (`test_generic_page_up_clamps_stale_offset`, `test_generic_page_down_clamps_at_content_end`) pin the box view, which still reaches the `_ =>` fallback. Their assertions did not change.

**Total deviations:** 0 auto-fixed bugs. **Impact:** none on scope.

## Inferred decisions (for audit)

1. **Orphan phases with no STATE.md milestone stay band-less** (plan-level [INFERRED]). The synthetic band is created only when three things hold: some phase has no milestone, no roadmap milestone is active, and `state.milestone` is non-empty. Its label is `"{milestone} {milestone_name}"`, or just the milestone when there is no name. Its `declared_phases` is 0 and it is never shipped.
2. **`Space` on a phase stores the band as the cursor,** not the now-hidden phase. Both resolve to the band row. Storing the band makes the cursor's resting place explicit and keeps `j`/`k` relative to the band row after a fold. A second `Space` on that band row unfolds it and the cursor stays on the band.
3. **`Space` on the shipped-summary row toggles the summary fold. `Enter` on it opens Archive.** This follows the plan's split: "Enter on a band toggles its fold, on the shipped summary returns switch_to_tab(Archive)".
4. **`PageUp`/`PageDown` before the first frame move one row,** because `roadmap_list_viewport` is 0 and the page is `max(1, viewport - 1)`.
5. **`h`/`l`/`[`/`]` with no edge or no wave peer leave the cursor where it is and clear the walk.** They do not beep or set a status message.
6. **The Roadmap footer is now 91 cells in full form**, and at 80 columns `[?]help` is clipped. The Git and Config footers already overflow 80, and plan 24-06 owns the Roadmap render and its width budget. The footer is not measured or tiered here because the plan does not ask for it.
7. **The build-phase message reads** "Build phase {id} is a planned placeholder, not a GSD phase — it has no Phases entry". The id is `render_for_terminal(&planned.number)`, which is the same escape `shown()` applies.
8. **`show_badges` comes from `ctx.config.preferences.gsd_integration`,** the flag the removed PhaseList tab used.

## Known Stubs

- `src/ui/screens/detail.rs`, `DetailScreen::roadmap_list_offset`: written in `new`, never read, under `#[allow(dead_code)]`. It is intentional, because plan 24-06's render owns the list scroll offset (the plan's interfaces block: "24-06's render writes both"). It does not block this plan's goal. `roadmap_list_viewport` is read by the PageUp/PageDown arms and stays 0 until 24-06 writes it.

## Threat Flags

None. There is no new surface beyond the plan's threat model:
- T-24-15: the handoffs use `tab_index` only, and `pipeline_selected` is found by `phase_key`.
- T-24-16: the cursor is stored as a key and re-resolved on every key press.
- T-24-17: the id is escaped before it reaches the status message.
- T-24-18: no arm touches the driver.

## Next Phase Readiness

- **24-06:** render `roadmap_model_for(state, cache, gsd_integration)`. Highlight `model.resolve_cursor(cache.roadmap_cursor)`. Write `roadmap_list_viewport` (the list height) and `roadmap_list_offset` from the render.
- **24-07:** re-route `Enter` on the shipped summary from `tab_index(&DetailSubView::Archive)` to Docs › Milestones (`switch_to_sub_view`). The one site is in `roadmap_activate`.

## Self-Check: PASSED

- FOUND: src/ui/screens/mod.rs, src/ui/screens/detail.rs, src/ui/screens/help.rs
- FOUND commits: a8fc2c6, d3d35b2, 9f03b30, 91da642
- `commits: 4` measured with `git rev-list --count 119ab70..HEAD` before the docs commit.
