---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 03
subsystem: ui
tags: [ratatui, detail-view, tabs, renumber, escape-guard]

requires:
  - phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
    provides: nothing (wave 1; files disjoint from 24-01 and 24-02)
provides:
  - "Detail tab order 1:Roadmap 2:Phases 3:Backlog 4:Git 5:Queue 6:Sess 7:Cfg 8:Docs, interim 9:Arch, Driver last (index 9)"
  - "DetailSubView without PhaseList; #[default] RoadmapViz"
  - "TAB_COUNT 10, DRIVER_TAB_INDEX 9, tab-bar widths 98/87/70/62"
  - "Digits 1-9 map to indices 0-8; the zero key is inert"
  - "Pipeline tab labelled and titled Phases"
  - "Tab tests that do not name a count, so plan 24-07's second renumber touches few lines"
affects: [24-05, 24-06, 24-07]

actuals:
  tokens: 15700
  tasks: 2
  commits: 2
plan_head_before: 6fea279de3db4f31431f81a5e5dce40e52545cc7

tech-stack:
  added: []
  patterns:
    - "Tab tests iterate (0..visible_tab_count(..)).map(sub_view_from_index) instead of hand-written variant lists"
    - "The last tab before Driver is DRIVER_TAB_INDEX - 1, never a literal"

key-files:
  created: []
  modified:
    - src/app.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "disk_suffix_spans kept under #[allow(dead_code)]: its only caller was PhaseList; plan 24-06 re-wires it as the [stage] badge (D-B08)"
  - "All three Pipeline block titles (both empty-state blocks and the right detail pane) read ' Phases ', per the plan's literal instruction"
  - "Escape-guard state-count prose made count-agnostic rather than renumbered ('fifteen' was already stale at 18 states; now 17)"

patterns-established:
  - "Count-agnostic tab tests: counts via visible_tab_count/TAB_COUNT, labels via the arrays, the last pre-Driver tab via DRIVER_TAB_INDEX - 1"

requirements-completed: [D-B01, D-B02, D-B03, D-B05, D-B07, D-B09, D-B10, D-B11]

coverage:
  - id: D1
    description: "New tab order and labels, Driver last; index mappings round-trip over every index"
    requirement: D-B01
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#every_tab_index_round_trips_through_its_sub_view"
        status: pass
      - kind: unit
        ref: "src/app.rs#the_driver_sub_view_is_the_last_tab_index_in_both_directions"
        status: pass
    human_judgment: false
  - id: D2
    description: "Fresh detail screen opens on Roadmap, 2 reaches Phases (titled ' Phases '), the zero key is inert"
    requirement: D-B07
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_fresh_detail_screen_opens_on_the_roadmap_and_two_reaches_phases"
        status: pass
    human_judgment: false
  - id: D3
    description: "Tab-bar widths equal the label arrays' own arithmetic; every tier renders whole and keeps the active tab"
    requirement: D-B11
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_tab_bar_widths_are_the_label_arrays_own_arithmetic"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_active_tab_label_is_always_present_in_the_rendered_bar"
        status: pass
    human_judgment: false
  - id: D4
    description: "PhaseList tab removed; escape-guard probe covers exactly the remaining sub-views with arrival set equality"
    requirement: D-B02
    verification:
      - kind: unit
        ref: "cargo test --lib ui::screens (331 passed, includes render_escape_guard probe)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Archive debug-fix regression tests untouched and green"
    requirement: D-B05
    verification:
      - kind: unit
        ref: "src/app.rs#archive_milestone_view_keeps_its_content_across_the_periodic_prune (+4 siblings)"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-09-24
status: complete
---

# Phase 24 Plan 03: Detail tab consolidation (renumber) Summary

**The detail view now opens on Roadmap. Tabs run `1:Roadmap 2:Phases 3:Backlog 4:Git 5:Queue 6:Sess 7:Cfg 8:Docs`, with Archive at an interim `9:Arch` and Driver last at index 9. The PhaseList tab and `render_phase_list` are deleted, and the zero key does nothing.**

## Performance

- **Duration:** about 9 min
- **Started:** 2026-09-24T03:26:52Z
- **Completed:** 2026-09-24T03:35:31Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `DetailSubView` no longer has a `PhaseList` variant. `#[default]` is now `RoadmapViz`, which is also the fallback in `sub_view_from_index` (out-of-range index, and the Driver index with the flag off) and in `effective_sub_view`.
- `TAB_COUNT` is 10 and `DRIVER_TAB_INDEX` is 9. The label arrays use the locked D-B01/D-B11 labels. The widths come from `Σ(len+2)+(n−1)` plus the marker cell: full 98, compact 70, full with no Driver 87, compact with no Driver 62. The anti-drift test was left as it was and passes.
- `tab_index` covers every variant with no wildcard arm. Digits `1`–`9` map to indices 0–8, and the arm for the zero key is deleted. The footer prefixes are `[1-9/D]` / `[1-9]`. They are as wide as before, so the three measured Driver footer forms still fit.
- `render_phase_list` and its arms in both `render` and `render_main_only` are removed. `phase_list_label` is kept because the Phases tab uses it.
- The Pipeline tab's block titles now read ` Phases `. The variant name is still `Pipeline`.
- In the escape guard, `ALL_SUB_VIEWS` is `[_; 10]` in tab order. The PhaseList arrival row and its label arm are removed. The RoadmapViz arrival reason now names the header's status and milestone lines.
- New end-to-end test `a_fresh_detail_screen_opens_on_the_roadmap_and_two_reaches_phases`.

## Task Commits

1. **Task 1: New tab order end to end** - `f977153` (feat)
2. **Task 2: Every tab test and probe agrees with the new order** - `1e28844` (test)

## Files Created/Modified
- `src/app.rs` - enum reordered, PhaseList removed, `#[default] RoadmapViz`, Driver doc updated to index 9 / `TAB_COUNT` 10, interim note on `Archive`, Driver index test renamed.
- `src/ui/screens/detail.rs` - constants, labels, widths and docs; mappings; digit arms; footer prefix; PhaseList render removed; Phases titles; `adjudicate_screen!` prose; tests.
- `src/ui/screens/render_escape_guard.rs` - `ALL_SUB_VIEWS`, `DETAIL_TAB_ARRIVAL`, `sub_view_label`, tab and state-count prose.

## Test renames (old → new)
- `app::tests::the_driver_sub_view_is_index_ten_in_both_directions` → `the_driver_sub_view_is_the_last_tab_index_in_both_directions` (asserts literal `9`, `10 → RoadmapViz`)
- `the_visible_tab_count_drops_the_eleventh_tab_when_experimental_is_off` → `the_visible_tab_count_drops_the_driver_tab_when_experimental_is_off`
- `the_ten_tab_bar_renders_whole_at_its_own_re_derived_widths` → `the_flag_off_bar_renders_whole_at_its_own_re_derived_widths`
- `index_ten_is_not_a_tab_when_experimental_is_off` → `an_index_past_the_visible_tabs_is_not_a_tab_when_experimental_is_off`
- `right_from_tab_nine_reaches_the_driver_tab_when_experimental_is_on` → `right_from_the_last_pre_driver_tab_reaches_the_driver_tab_when_experimental_is_on`
- `the_full_tier_renders_eleven_labels_at_its_measured_width` → `the_full_tier_renders_every_label_at_its_measured_width`
- `the_compact_tier_renders_eleven_labels_at_its_measured_width` → `the_compact_tier_renders_every_label_at_its_measured_width`

## PhaseList-only content: what remains for plan 24-06
`render_roadmap`'s header already draws the Path line, Status + Milestone, the unreadable and recovered lines, the Paused line and the change-tracker banner. I read it to confirm this. The following were drawn **only** by the removed PhaseList tab:
- **The per-phase `[stage]` badge** (`disk_suffix_spans`, D-B08). The function is kept under `#[allow(dead_code)]` so 24-06 can wire it into the detail pane.
- **The empty-state wording**: `No roadmap data available` (no phases) and `No state data available for this project.` (no state). Roadmap has its own `No state data available for roadmap.`, and 24-06 decides the final wording (D-B08).
- The per-phase `done/total plans` text, the marker legend line, and the `Backlog: N items` / `Queued:` summaries. The Backlog and Queue tabs still draw these, and the Roadmap redesign (24-04/24-06) covers the plan counts.

**Escape-guard coverage:** no third-party field lost its only render site. PhaseList drew phase number and name (the Roadmap graph and box view, and the Phases list, still draw these), status, milestone and pause context (the Roadmap header and Phases tab draw these), and queued commands (the Queue tab draws these). The set-equality probe passes with no faked arrival.

## Decisions Made
See the key-decisions list in the frontmatter.

## Inferred decisions (for audit)
1. **The `app.rs` Driver-index test was renamed in Task 1, not Task 2.** The compile fix required touching it anyway, so the rename went into the same edit. The final content matches the plan.
2. **The `"1:Phases"` literal in `the_active_tab_label_is_always_present_in_the_rendered_bar` was changed to `("1:Roadmap", "1:Rd")` in Task 1**, because Task 1's acceptance grep forbids `"1:Phases"` anywhere in `src`.
3. **`disk_suffix_spans` is kept with `#[allow(dead_code)]` and a doc note** instead of being deleted. The plan says to keep "any helper still referenced", and this one is not referenced after the removal. Deleting it would force 24-06 to write it again (RESEARCH "Don't Hand-Roll": `[stage]` badge → `disk_suffix_spans`).
4. **The right-hand detail pane's block title is also ` Phases `.** The plan says the `" Pipeline "` block titles (plural) become `" Phases "`. RESEARCH left the right pane "as appropriate". I followed the plan's wording.
5. **Escape-guard prose that counted states ("fifteen") now avoids a number.** It was already stale (18 states before this plan, 17 after). Tab counts use "ten". The history line reads "eleven at the time".
6. **Windowed-tier test widths:** the literal `76` became `TAB_BAR_COMPACT_CELLS - 1`. After the renumber, 76 is at or above the new compact width of 70, so the test would have stopped exercising the windowed tier.
7. **`generic_fixture` now parks on `RoadmapViz`**, which is now the only sub-view that reaches the `_ =>` scroll fallback. `roadmap_graph_tab_v_is_inert_on_other_tabs` and the driver-key-scope test use `Backlog` and `RoadmapViz` respectively.
8. **README.md and docs/ARCHITECTURE.md were not touched.** They are in plan 24-07's `files_modified`, not in this plan's.

## Deviations from Plan

None. The plan was executed as written. The items under "Inferred decisions" are ordering or discretion choices within the plan's scope.

## Issues Encountered
None.

## Verification
- `cargo test --no-fail-fast`: 49 suites, **2304 passed, 1 failed, 15 ignored**. That is the 2303 baseline plus the one new test. The only failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `cargo test --lib ui::screens`: 331 passed, 0 failed.
- The five archive regression tests pass, and `git diff 6fea279 -- src/app.rs` has no hunk after line 4252. The archive block starts at about line 4731.
- `cargo clippy -- -D warnings` (lib): exit 0. `cargo clippy --all-targets -- -D warnings`: 11 error lines, the same as the baseline. All are in `src/browser.rs`, `src/project_creator.rs` and `tests/envelope_*.rs`, and none are new.
- The Pitfall-5 sweep (`[1-0`, `1:Phases`, `5:Pipe`, `8:Arch`, `0:Docs`, `DetailSubView::PhaseList` over `src tests`) returns nothing.
- `KeyCode::Char('0') =>` is absent from `detail.rs`.

## User Setup Required
None.

## Next Phase Readiness
- 24-07 only needs to change `TAB_COUNT` 10→9 and `DRIVER_TAB_INDEX` 9→8, drop `9:Arch`/`9:Ar` and the `'9'` arm, re-derive the widths, and port the archive tests. The tab tests are count-agnostic, apart from the literal `9`/`10` in the `app.rs` Driver-index test, which is literal by design.
- 24-06 has to re-cover the `[stage]` badge (`disk_suffix_spans`) and the empty-state wording (D-B08).

---
*Phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation*
*Completed: 2026-09-24*

## Self-Check: PASSED
