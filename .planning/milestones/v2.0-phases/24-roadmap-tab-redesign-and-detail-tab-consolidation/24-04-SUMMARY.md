---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 04
subsystem: ui
tags: [roadmap, ratatui, stateful-widget, master-detail, git-log-lanes, escaping]
status: complete
requires:
  - phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
    provides: "24-02's RoadmapModel, ListRow, PhaseFacts, BandFacts, CursorTarget, BandKey, layout_list, lane_text, resolve_cursor/row_of/phase_index and the glyph consts"
provides:
  - "ui::roadmap_view::RoadmapView (StatefulWidget) and RoadmapViewState { offset, list_rows }"
  - "ui::roadmap_view::{panes, keep_visible}"
  - "ui::roadmap_view::{ROADMAP_SIDE_BY_SIDE_MIN_COLS, LANE_CAP_NARROW, LANE_CAP_WIDE, LANE_OVERFLOW}"
  - "src/ui/mod.rs escape test ported to RoadmapView; mod.rs imports no legacy graph type"
affects: [24-05, 24-06]
actuals:
  tokens: 20722
  tasks: 3
  commits: 4
plan_head_before: a341e8ce43ea94a42fa065af324775f9db4e0559
tech-stack:
  added: []
  patterns:
    - "Detail pane as a list of Items (Line / Wrapped / Hint) drawn top to bottom; the Hint is dropped first when the pane is short"
    - "Wrapped-text height measured by rendering Paragraph::wrap into a scratch Buffer (no hand-rolled word wrap; ratatui's line_count is behind an unstable feature)"
    - "Char-based truncate/pad/fit helpers over escaped text; every cell goes through Block/Paragraph into a sub-rect of area ∩ buf.area"
key-files:
  created:
    - src/ui/roadmap_view.rs
  modified:
    - src/ui/mod.rs
key-decisions:
  - "RoadmapView resolves its cursor through RoadmapModel::resolve_cursor, so cursor None selects the active phase and a fold-hidden phase shows its band row"
  - "A planned phase's status line shows `planned (not a GSD phase)` in place of `plans TBD` (it fits 44 inner columns that way)"
  - "The side-by-side no-deps note wraps under the Parallel label; the stacked form appends it to the Parallel line (Mockup C)"
  - "A node whose own lane is past the cap draws its status glyph in the overflow cell instead of ┆"
  - "Ids are capped at 8 cells in the list and in detail entries; the pane title shows the full escaped id"
requirements-completed: [D-A01, D-A02, D-A04, D-A05, D-A06, D-A07, D-A09, D-A13, D-A14]
coverage:
  - id: D1
    description: "Side-by-side list + detail pane at 120 columns (Mockup A): Start-now line, header, phase rows with lanes, ▶/↑/↓ markers, detail name, band · wave W of M, Needs, Unblocks with the other band's short id, Parallel"
    requirement: D-A01
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#mockup_a_side_by_side_at_120_columns"
        status: pass
    human_judgment: false
  - id: D2
    description: "Stacked 80-column form (Mockup C) with `nothing declared`, `(no edge either way; can run any time)` and the `j/k move` hint; Mockup B's implied dep, `nothing (last in v1.5)`, the · marker, the truncated name and the shipped summary row"
    requirement: D-A14
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#mockup_c_stacked_at_80_columns, mockup_b_implied_dep_and_shipped_summary_at_120"
        status: pass
    human_judgment: false
  - id: D3
    description: "Band rows and the shipped summary are drawn once each with ▾/▸ fold glyphs; band and summary cursors get their own detail panes"
    requirement: D-A07
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#band_labels_are_drawn_once, a_band_cursor_has_its_own_detail_pane, the_shipped_summary_cursor_lists_its_milestones"
        status: pass
    human_judgment: false
  - id: D4
    description: "Width handling without horizontal scroll: breakpoint at 100 columns, lanes past the 4/6 cap collapse into one ┆, keep_visible scrolling, list_rows reported"
    requirement: D-A13
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#lanes_past_the_cap_collapse_into_one_overflow_column, keep_visible_clamps_both_ways, the_cursor_row_is_scrolled_into_view"
        status: pass
    human_judgment: false
  - id: D5
    description: "Empty model, planned phase and stage badge cases"
    requirement: D-A02
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#an_empty_model_explains_itself, a_planned_phase_says_so, the_stage_badge_is_in_the_status_line"
        status: pass
    human_judgment: false
  - id: D6
    description: "Escape-safe and size-safe (T-24-12, T-24-13): no raw ESC/U+202E reaches a cell, no panic at tiny sizes, no bleed outside the Rect, multibyte names truncate on char boundaries; the ported two-direction test in src/ui/mod.rs"
    verification:
      - kind: unit
        ref: "src/ui/roadmap_view.rs#roadmap_view_stores_and_draws_only_escaped_text, roadmap_view_never_panics_at_tiny_sizes, roadmap_view_never_draws_outside_its_rect, a_multibyte_name_truncates_on_char_boundaries; src/ui/mod.rs#the_roadmap_view_renders_a_clean_phase_name_unchanged_and_a_control_one_differently"
        status: pass
    human_judgment: false
  - id: D7
    description: "How the widget looks and reads at 80 and 120 columns next to the real header, tab bar and footer (colours, density, spacing)"
    verification: []
    human_judgment: true
    rationale: "Visual quality is subjective, and the widget has no caller until 24-05 and 24-06 wire it into the detail screen. Judge it at phase-level UAT on the real screen."
duration: 18 min
completed: 2026-09-24
---

# Phase 24 Plan 04: Roadmap View Widget Summary

**`ui::roadmap_view::RoadmapView` is a `StatefulWidget` over 24-02's escaped `RoadmapModel`. It draws a phase list with a capped git-log lane column, and beside it (at 100+ columns) or under it (below 100) a detail pane for the phase, band or shipped summary under the cursor. There is no horizontal scroll; the cursor row always scrolls into view. Mockups A, B and C reproduce closely at 120 and 80 columns.**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-09-24T03:37:13Z
- **Completed:** 2026-09-24T03:55:26Z
- **Tasks:** 3/3
- **Files modified:** 2 (`src/ui/roadmap_view.rs` created, `src/ui/mod.rs`)

## Accomplishments

- **Split.** `panes(area)` puts the detail pane beside the list at `ROADMAP_SIDE_BY_SIDE_MIN_COLS` (100) and above, with width `clamp(w·2/5, 40, 56)`. Below 100 it stacks the pane under the list, with height `min(9, h/2)`. A zero-size area returns zero rects.
- **List pane.**
  - The `Start now:` line joins the active and ready phases with ` ║ `.
  - A `lanes  #  Phase … plans  wave` header follows. Neither line scrolls.
  - Each phase row is `{lanes}{marker}{id}  {name…}{plans}  W<wave>`.
  - Band rows read `▾|▸ label  done/total ━━━`. The shipped summary reads `▸ v1.0 … v1.4   5 milestones · 17 phases shipped`.
  - A `Notes` line takes the last row when the model has a cycle note.
- **Styles.**
  - Glyph colours follow D-A04: ● dim, ◉ yellow bold, ○ green, ◌ default. Done rows are dimmed.
  - The cursor row is reversed and marked `▶`. The selection's needs are marked `↑` (cyan), what it unblocks `↓` (magenta), and its implied deps `·` (dim).
- **Detail pane, side-by-side form (Mockups A and B).**
  - Header lines: the name, then `{band label} · wave W of M`, then the status line: glyph, word, plans or `plans TBD`, the badge verbatim, and `planned (not a GSD phase)`.
  - `Goal` is wrapped with `Paragraph::wrap`.
  - `Needs` lists the needs, then each implied dep with an `(implied via N)` row, then external deps. A phase with no deps reads `nothing declared`.
  - `Unblocks` adds the other band's short id where it differs. With nothing to unblock it reads `nothing (last in v1.5)` or `nothing`.
  - `Parallel` lists the other phases in the same wave (a done peer is marked `done`), or `none in wave W`. When the phase has no deps, the D-A14 explanation follows.
- **Detail pane, stacked form (Mockup C).** One head line, then the Goal with a hanging indent, then `Needs … Unblocks …` on one line when both fit, then `Parallel … (no edge either way; can run any time)`, then the `j/k move` hint. When the pane is short, the hint is dropped first.
- **Band and summary panes.** A band under the cursor shows its label, `done/total phases done` and `Space fold/unfold`. The shipped summary shows each shipped milestone with its declared phase count and `⏎ open milestones`.
- **Lane cap.** At most 4 lanes are drawn below 100 columns and 6 at 100 or more. The rest collapse into one `┆` column (T-24-14).
- **Scrolling.** `keep_visible` and the `clamp_scroll` idiom keep the cursor row in view. `state.offset` is updated in place, and `state.list_rows` reports the number of visible model rows.
- **Empty model.** It draws `No roadmap data available`.
- **Ported escape test.** `src/ui/mod.rs`'s two-direction escape test now exercises `RoadmapView` on the detail pane's name line. `mod.rs` imports no legacy graph type.

## Task Commits

1. **Task 1 (tracer): side-by-side list + detail pane.** `bad0bbd` (feat). The tracer gate re-ran `cargo test --lib ui::roadmap_view` (1 passed) before expanding.
2. **Task 2 (TDD): stacked layout, bands, lane cap, scrolling, empty states.** RED `9ae44b3` (test), GREEN `ef6625d` (feat).
3. **Task 3 (TDD): escaping, tiny sizes, no bleed, multibyte, and the `mod.rs` port.** `a39feeb` (test). See TDD Gate Compliance.

## TDD Gate Compliance

- **Task 2.**
  - RED `test(24-04)` `9ae44b3`: 10 of the 11 named tests failed on assertions. The RED commit carried a `keep_visible` stub that returned `offset` unchanged, so the tests compiled.
  - `the_stage_badge_is_in_the_status_line` already passed at RED, because Task 1's action specified the status line with the badge.
  - `a_planned_phase_says_so` failed at RED for a real reason: at 44 inner columns the planned marker was truncated. GREEN fixed that.
  - `check tdd-red-evidence` on `mockup_c_stacked_at_80_columns` returned `RED_EVIDENCE_OK`.
  - GREEN `feat(24-04)` `ef6625d`: all 12 tests pass. No refactor commit was needed.
- **Task 3: unexpected GREEN at RED.** It has only a `test(24-04)` commit (`a39feeb`) and no `feat` commit.
  - All four widget tests and the ported `mod.rs` test passed on their first run. Tasks 1 and 2 were built to this contract: escaped model strings only, char-based truncation, and every write clipped to sub-rects of `area ∩ buf.area`. So these tests pin behaviour that already exists, the same pattern as 24-02's robustness tests.
  - Per the fail-fast rule, I checked that the tests can fail. Three throwaway mutations, reverted from a backup, each turned exactly the matching test red:
    - drawing `PhaseFacts::key` in the pane title failed `roadmap_view_stores_and_draws_only_escaped_text`;
    - byte-slice truncation failed `a_multibyte_name_truncates_on_char_boundaries`;
    - a detail pane one row taller than its rect failed `roadmap_view_never_draws_outside_its_rect`.

## Verification

- `rtk proxy cargo test --lib ui::roadmap_view`: 16 passed, 0 failed.
- `rtk proxy cargo test --lib ui::`: 413 passed, 0 failed.
- `rtk proxy cargo test --no-fail-fast`: 49 suites, 2320 passed, 1 failed, 15 ignored. The baseline was 2304 passed; the difference is exactly the 16 new `roadmap_view` tests, because the `mod.rs` test was replaced one for one. The single failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy -- -D warnings`: clean.
- `rtk proxy cargo clippy --all-targets --keep-going -- -D warnings`: 11 errors, all in the baseline files (`src/browser.rs` ×3, `src/project_creator.rs`, `tests/envelope_*.rs` ×7). None is in `roadmap_view.rs` or `mod.rs`.
- `git grep -n -F -e RoadmapGraphWidget -e layout_graph -e GraphNode -- src/ui/mod.rs` prints nothing.
- `git diff --stat a341e8c..HEAD -- src` lists only `src/ui/roadmap_view.rs` and `src/ui/mod.rs`. `src/ui/screens/` was not touched.
- Prohibitions:
  - There is no horizontal scroll state or key. `RoadmapViewState` has only `offset` and `list_rows`.
  - No `str::len` or byte slicing is applied to displayed text. The only `.len()` calls are on `GAP`, a static ASCII literal, and on char vectors.
  - There is no mouse handling.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A planned phase's status line did not fit the side-by-side pane.**
- **Found during:** Task 2 RED (`a_planned_phase_says_so` at 120 columns).
- **Issue:** `○ ready · plans TBD · planned (not a GSD phase)` is 46 cells, and the pane has 44 inner cells, so the marker was truncated.
- **Fix:** A planned phase shows `planned (not a GSD phase)` in place of `plans TBD`. A known plan count is still shown.
- **Files modified:** `src/ui/roadmap_view.rs`
- **Commit:** `ef6625d`

**2. [Rule 1 - Bug] The no-deps note was cut at 44 columns.**
- **Found during:** Task 2 RED.
- **Issue:** `(no edge either way; can run any time)` is 38 cells, but only 34 remain after the 10-cell label indent.
- **Fix:** In the side-by-side form the note is a wrapped item with a hanging indent under the Parallel label.
- **Files modified:** `src/ui/roadmap_view.rs`
- **Commit:** `ef6625d`

**3. [Rule 1 - Test bug] The lane-cap test's `body()` helper kept the right border `│`.**
- **Found during:** Task 2 GREEN.
- **Issue:** `│` is a lane glyph, so the test flagged its own border.
- **Fix:** `body()` now drops the padding and border on both sides.
- **Commit:** `ef6625d`

**4. [Rule 1 - Lint] clippy `type_complexity` on a test spec vector.**
- **Fix:** Factored out a `Row<'a>` type alias.
- **Commit:** `ef6625d`

### Additions beyond the plan text

- Both panes have one cell of inner horizontal padding, matching the mockups' ` Start now:` and ` Checkout & payments`.
- `roadmap_view_never_draws_outside_its_rect` checks two more rects, full-size side-by-side and stacked, besides the plan's `Rect::new(3, 2, 5, 3)`.
- `roadmap_view_never_panics_at_tiny_sizes` also covers the empty and hostile models, the shipped cursor, no cursor, and a stale `offset: 99`.

---

**Total deviations:** 4 auto-fixed (2 layout bugs, 1 test bug, 1 lint).
**Impact on plan:** All four were needed for correctness at the specified widths. There is no scope creep, and the files stayed inside `files_modified`.

## Inferred decisions (for audit)

1. **The cursor is resolved inside the widget** through `RoadmapModel::resolve_cursor`. `cursor: None` therefore selects the active phase, and a stored fold-hidden phase shows its band row. The screen (24-05) may pass the stored cursor unresolved.
2. **Wording for a phase with no deps.** Mockup C wording, `(no edge either way; can run any time)`, when nothing depends on it either (`no_edges`). Otherwise `(no deps; can run any time)`. This second wording is the plan's own `[INFERRED — audit]` item.
3. **A planned phase drops `plans TBD`** in favour of `planned (not a GSD phase)`, so the line fits 44 columns (deviation 1).
4. **A node past the lane cap** draws its status glyph in the overflow cell instead of `┆`, so its status stays visible. Pass-through lanes past the cap still collapse to `┆`.
5. **Id width is capped at 8 cells** in the list and in detail entries. Longer ids, such as a hostile id's escaped form, are truncated with `…`. The pane title shows the full escaped id.
6. **The lane column width is measured over all model rows, not only the visible slice.** Otherwise columns would shift while scrolling.
7. **The shipped-summary row is not padded to the lane column.** Mockup B draws `▸ v1.0 …` flush left. Band rows are padded.
8. **The shipped pane's count per milestone is `max(declared, listed)`**, matching 24-02's summary total.
9. **Detail entries omit the plans tail when the count is unknown**, instead of showing `—`. Implied deps show no plans, as in Mockup B's `(implied via 21)` row.
10. **Wrapped-text height is measured by rendering into a scratch `Buffer`.** ratatui 0.30's `Paragraph::line_count` sits behind the unstable `unstable-rendered-line-info` feature, which is not enabled in `Cargo.toml`.
11. **The Notes line reads `Notes  <first note>`** and takes the list pane's last row, only when the pane has more than two inner rows.
12. **The band pane's hint is `Space fold/unfold`,** plus `   j/k move` in the stacked form. The shipped pane's hint is `⏎ open milestones   Space fold/unfold`.

## Known Stubs

None. The Task 2 RED `keep_visible` stub was replaced in `ef6625d`.

## Threat Flags

None. There is no new surface. T-24-12 is covered by `roadmap_view_stores_and_draws_only_escaped_text` and the ported `mod.rs` test. T-24-13 is covered by the tiny-size, no-bleed and multibyte tests. T-24-14 is covered by the lane cap and its test.

## Next Phase Readiness

- **24-05 (keys / view state).**
  - Render `RoadmapView { model, cursor: cache.roadmap_cursor.as_ref() }` with a `RoadmapViewState` whose `offset` persists in `DetailScreen.roadmap_list_offset`.
  - Read `state.list_rows` back into `roadmap_list_viewport` for PageUp/PageDown.
  - The widget already keeps the cursor row visible.
- **24-06 (detail screen).** Lay the widget into the Roadmap content area. `panes()` is public for tests such as `roadmap_list_pane`. The legacy `RoadmapGraphWidget` no longer has a caller in `src/ui/mod.rs`.

---
*Phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation*
*Completed: 2026-09-24*

## Self-Check: PASSED

- FOUND: src/ui/roadmap_view.rs, src/ui/mod.rs, 24-04-SUMMARY.md
- FOUND commits: bad0bbd, 9ae44b3, ef6625d, a39feeb
