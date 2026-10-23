---
phase: quick-261022-wtw
plan: 01
subsystem: ui
tags: [detail-screen, roadmap, dead-code-removal, ratatui]
requires: []
provides:
  - "Roadmap tab with a single rendering path (the lanes list); `v` unbound"
affects: [src/ui/screens/detail.rs, src/ui/mod.rs, src/state_reader/mod.rs]
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/mod.rs
    - src/state_reader/mod.rs
    - src/state_reader/state_md.rs
    - tests/state_reader_test.rs
    - README.md
    - docs/ARCHITECTURE.md
    - docs/DEVELOPMENT.md
    - .planning/codebase/ARCHITECTURE.md
    - .planning/codebase/STRUCTURE.md
  deleted:
    - src/ui/roadmap_widget.rs
decisions:
  - "I-1..I-8 from the plan applied as written (see Inferred decisions)"
metrics:
  duration: ~35m
  completed: 2026-10-23
status: complete
actuals:
  tokens: 40000
  tasks: 3
  commits: 3
plan_head_before: a117e4da9998b0da8c57249d61cf12dd415f19e1
plan_head_after: 460a21976fb076592cc41f4431c83fe2b4029054
---

# Phase quick-261022-wtw Plan 01: Remove the Detail screen Roadmap box view Summary

The Roadmap tab now draws only the lanes list. The `v` toggle, the `roadmap_box_view` flag, the widget module, the generic-scroll machinery only the box view reached, and the box-view text in the help popup, footer and README are all gone.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 084fb4f | refactor(quick-261022-wtw): remove the Roadmap box view from the Detail screen |
| 2 | 69ac009 | refactor(quick-261022-wtw): delete the Roadmap boxes widget and its only dependents |
| 3 | 460a219 | docs(quick-261022-wtw): drop the Roadmap box view from README and docs trees |

`git diff --stat a117e4d..HEAD`: 14 files changed, 135 insertions(+), 1020 deletions(-).

## What changed

- `detail.rs`: removed `scroll_offset` and `generic_viewport` from `DetailScreen`, removed `roadmap_list_active`, and removed the `scroll_offset` parameter from `switch_to_tab` and `switch_to_sub_view` (about 15 callers updated). The four scroll matches (j/Down, k/Up, PageDown, PageUp) lost their `_ =>` generic arm. `render_roadmap` renders `RoadmapView` unconditionally. The `v` arm and the footer `[v]` span are gone.
- `mod.rs` (screens): `ProjectViewCache.roadmap_box_view` removed.
- `help.rs`: the Roadmap `v` row and `the_roadmap_graph_toggle_key_is_documented` removed.
- `render_escape_guard.rs`: both box-view entries removed together (`DETAIL_TAB_ARRIVAL` and `DETAIL_SUB_STATES`), so the two-way set equality still holds.
- `src/ui/roadmap_widget.rs` deleted. `PhaseMarker::glyph` deleted. The two widget-rendering integration tests deleted from `tests/state_reader_test.rs`, along with the `rendered_glyph_column` and `phase3_glyph` helpers.
- The ratatui C1-drop measurement survives as `ui::tests::ratatui_drops_a_c1_grapheme_before_a_cell_exists`.
- README key table and mouse paragraph updated. Four doc trees now list `roadmap_view.rs` in place of the deleted file.

## Verification

| Check | Result |
|-------|--------|
| Baseline (before edits) | passed=4136 failed=1 ignored=24 |
| Final | passed=4127 failed=1 ignored=24 (delta -9, as expected) |
| Only failure | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (expected on this machine) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings |
| `cargo fmt --check` | clean |
| Task 1 gate (`ui::screens`) | 891 passed |
| Task 2 gates | `ui::` 992 passed, `state_reader::` 480 passed, `state_reader_test` 68 passed (1 ignored) |
| Grep gates | box-view/generic-scroll identifiers in `src/ui/screens`: 0; `roadmap_widget` outside the verbatim census line: 0; README `box view`/`On the list view`: 0; docs trees: 0 |

Test delta, by test: -2 widget unit tests (deleted with the module), -2 `state_reader_test`, -4 in `detail.rs` (`test_generic_page_up_clamps_stale_offset`, `test_generic_page_down_clamps_at_content_end`, `roadmap_box_view_keeps_generic_scroll`, `roadmap_graph_tab_v_is_inert_on_other_tabs`), -1 `help.rs`. Net 0 for the replaced and renamed `detail.rs` tests and for the `ui/mod.rs` swap.

## Deviations from Plan

None - plan executed as written. The proof gate in Task 1 step 2g did not fire: the four scroll matches compiled with no wildcard (no E0004), confirming that no other view reached the generic arms.

Small additions beyond the literal steps, all consequences of the removals:
- The `driver_viewport` and `git_commit_viewport` field docs counted sibling `Cell`s by ordinal ("fourth ... three above", "fifth ... four above"). Removing `generic_viewport` made them wrong, so they now read "third ... two above" and "fourth ... three above".
- The new test's assertion message was reworded ("no per-phase boxes are drawn") so the Task 1 grep gate for "box list" reads 0.
- `opened_on` no longer needs `let mut screen`; changed to `let screen` (clippy/unused_mut).

## Inferred decisions `[INFERRED — audit]`

All carried from the plan, applied as written:
- I-1: the generic `_ =>` scroll arms, `generic_viewport` and `DetailScreen.scroll_offset` were reached only by the box view. Compiler exhaustiveness confirmed this. The Enter/Space `_ => ScreenAction::None` arm stays.
- I-2: `src/ui/roadmap_widget.rs` deleted entirely.
- I-3: `PhaseMarker::glyph` deleted. `PhaseMarker` and `decide` stay.
- I-4: the two widget-rendering integration tests deleted. Four other `state_reader_test` tests still pin the state-level facts.
- I-5: the ratatui C1-drop measurement moved to its own test.
- I-6: `v` left unbound. `other_roadmap_keys_end_the_edge_walk` keeps `v` in its key list, and a replacement test pins `v` as a no-op.
- I-7: provenance comments reworded so they do not name the deleted module path. The census `Sites: [` line is untouched.
- I-8: four doc trees got a one-line swap to `roadmap_view.rs`. Historical artifacts are untouched.

## Known Stubs

None.

## Threat Flags

None. The escape-probe coverage for the remaining Roadmap renderers (default arrival, planned-phase cursor, synthetic band) is unchanged and passing (T-wtw-01). The C1-drop pin is preserved (T-wtw-02).

## Self-Check: PASSED

- `src/ui/roadmap_widget.rs` is absent; `src/ui/roadmap_view.rs` exists.
- Commits 084fb4f, 69ac009 and 460a219 are ancestors of HEAD.
- Not staged or committed: `.planning/milestone.lock`, `.planning/musk/ledger.jsonl`, SUMMARY.md, STATE.md, PLAN.md.
