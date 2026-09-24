---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
verified: 2026-09-24T05:10:00Z
status: passed
score: 5/5 roadmap success criteria verified; 27/27 locked decisions (D-A01-D-A15, D-B01-D-B12) accounted for and satisfied
covered_files:
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-01-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-01-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-02-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-02-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-03-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-03-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-04-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-04-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-05-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-05-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-06-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-06-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-07-PLAN.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-07-SUMMARY.md
  - .planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-CONTEXT.md
  - README.md
  - docs/ARCHITECTURE.md
  - src/app.rs
  - src/state_reader/mod.rs
  - src/state_reader/roadmap_md.rs
  - src/ui/mod.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
covered_digest: "v1:sha256:a04acb182152851556ac15dc342655dc06b4c0efd9a9c743dc4637e8a9ec448e"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation Verification Report

**Phase Goal:** The Roadmap tab becomes the primary per-project view — a master/detail phase
list with a git-log-style dependency graph, a selection cursor and a detail pane that says what
each phase needs, unblocks and can run alongside — and the detail tabs consolidate from ten to
eight (Roadmap · Phases · Backlog · Git · Queue · Sess · Cfg · Docs, plus Driver).

**Verified:** 2026-09-24
**Status:** passed
**Re-verification:** No — initial verification

## Requirement-ID Cross-Reference (D-A*/D-B*, 24-CONTEXT.md)

REQUIREMENTS.md has no Phase 24 entries (confirmed by grep — 0 matches), consistent with the
ROADMAP's own "Requirements: TBD (locked decisions in `24-CONTEXT.md`)". All 27 locked decisions
were treated as the requirement set and cross-referenced against every plan's `requirements:`
frontmatter field. No ID is orphaned (declared in CONTEXT but claimed by no plan) and no plan
claims an ID CONTEXT.md does not define.

| ID | Claimed by | Satisfied | Evidence |
|----|-----------|-----------|----------|
| D-A01 (list pane, git-log lane column) | 24-02, 24-04 | ✓ | `layout_list`/`RoadmapView`; `mockup_a_side_by_side_at_120_columns` passes |
| D-A02 (detail pane: Goal/Needs/Unblocks/Parallel) | 24-02, 24-04, 24-06 | ✓ | `roadmap_view.rs` detail-pane items; sentriq/ttbook fixture tests pass |
| D-A03 (Goal parsing, both bold forms) | 24-01, 24-06 | ✓ | `a_goal_is_read_in_both_bold_forms` passes |
| D-A04 (status glyph set/colours) | 24-02, 24-04 | ✓ | `PhaseStatus::glyph()`; glyph consts in `roadmap_graph.rs` |
| D-A05 (selection markers, cursor persistence) | 24-02, 24-04, 24-05 | ✓ | `CursorTarget`-keyed cursor; `a_folded_phase_resolves_to_its_band_row` passes |
| D-A06 ("Start now" line) | 24-02, 24-04 | ✓ | `RoadmapModel.start_now`; drawn in `roadmap_view.rs` |
| D-A07 (milestone bands, shipped-summary fold) | 24-01, 24-02, 24-04, 24-05 | ✓ | `unfolding_the_shipped_summary_reveals_one_row_per_shipped_milestone`, `daily_vow_shipped_milestones_are_the_five_before_v1_5` pass |
| D-A08 (transitive reduction, git-log lane rules, no dupes) | 24-02, 24-06 | ✓ | `reduction_daily_vow_23_implies_20_via_21`, `reduction_sentriq_11_implies_9_via_10`, `daily_vow_phase_20_has_exactly_one_row`, `band_labels_appear_on_exactly_one_row` all pass |
| D-A09 (80-col stack / ~100-col side-by-side) | 24-04, 24-06 | ✓ | `mockup_c_stacked_at_80_columns`, `sentriq_roadmap_at_80_columns_is_stacked_and_shows_9_once`, `mockup_a_side_by_side_at_120_columns` pass |
| D-A10 (keys: j/k, g/G, h/l, [/], Space, Enter, v) | 24-02, 24-05 | ✓ | `roadmap_enter_*`, `bracket_steps_within_the_wave_and_wraps`, `h_cycles_the_origin_needs_including_implied`, `l_cycles_unblocks` pass |
| D-A11 (no mouse handling) | 24-05 | ✓ | grep for `MouseEvent`/`handle_mouse` in touched files: 0 matches |
| D-A12 (Build phase N parsing, selectable) | 24-01, 24-05, 24-06 | ✓ | `ttbook_build_phase_headings_parse_as_planned_phases`, `roadmap_model_for_ttbook_lists_build_phases_as_phases`, `roadmap_enter_on_a_build_phase_explains_itself` pass |
| D-A13 (no horizontal scroll mechanism) | 24-04, 24-06 | ✓ | grep for `horizontal_scroll`/`hscroll` in touched files: 0 matches |
| D-A14 (eliminate the 4 named bugs) | 24-01, 24-02, 24-04, 24-06 | ✓ | see SC-1 row below |
| D-A15 (keep parse/cycle/wave, rewrite layout; legacy deleted) | 24-02, 24-06 | ✓ | `roadmap_graph.rs` legacy `layout_graph`/`RoadmapGraphWidget` removed per 24-06-SUMMARY; `layout_list` present |
| D-B01 (new 8-tab order + Driver) | 24-03, 24-07 | ✓ | `TAB_LABELS_FULL` order + `every_tab_index_round_trips_through_its_sub_view` |
| D-B02 (PhaseList removed, content moved to Roadmap header) | 24-03, 24-06 | ✓ | `DetailSubView` has no `PhaseList` variant; `roadmap_summary_line`, Paused/change-tracker banner in `render_roadmap` header |
| D-B03 (Pipeline renamed Phases, shared selection) | 24-03, 24-05 | ✓ | `roadmap_enter_opens_the_selected_phase_in_phases`, `phases_selection_is_shared_back_to_the_roadmap` pass |
| D-B04 (Archive folds into Docs › Milestones) | 24-07 | ✓ | `opened_on_archive_lands_on_docs_milestones`, `m_switches_docs_between_files_and_milestones`, `enter_on_the_shipped_milestones_row_opens_docs_milestones` pass |
| D-B05 (every tab-index consumer updated) | 24-03, 24-07 | ✓ | round-trip test, width-arithmetic test, escape-guard probe, help text, README, ARCHITECTURE.md all updated and passing |
| D-B06 (Archive move sequenced last, after debug fix) | 24-07 | ✓ | 24-07 is wave 4, depends_on [24-03,24-05,24-06]; the 5 named archive regression tests pass unmodified in assertions |
| D-B07 (default tab is Roadmap) | 24-03 | ✓ | `#[default] RoadmapViz` in `src/app.rs:19-20` |
| D-B08 (PhaseList-unique content preserved) | 24-05, 24-06 | ✓ | Paused/change-tracker/`[stage]` badge moved into Roadmap header/detail pane (`detail.rs:4087` region) |
| D-B09 (in-memory-only view state, no migration) | 24-03, 24-07 | ✓ | `detail_sub_view_per_project` unchanged storage shape; `Archive` kept as a sub-view per 24-07 inferred decision |
| D-B10 (digits 9/0 inert) | 24-03, 24-07 | ✓ | `nine_and_zero_are_inert_in_the_detail_view` passes |
| D-B11 (compact labels, recomputed widths) | 24-03, 24-07 | ✓ | `the_tab_bar_widths_are_the_label_arrays_own_arithmetic` passes |
| D-B12 (status glyph from `phase_marker`, not checkbox) | 24-02, 24-05, 24-06 | ✓ | `state.phase_marker(p)` wired at `detail.rs:1572`; folded todo closed citing this fix |

## Goal Achievement — ROADMAP Success Criteria

| # | Success Criterion (ROADMAP.md) | Status | Evidence |
|---|---------------------------------|--------|----------|
| 1 | Every phase renders exactly once, each milestone label exactly once, implied deps show as a dim marker not a duplicate row (daily-vow `20`-twice, sentriq `9→11` bugs gone) | ✓ VERIFIED | `daily_vow_phase_20_has_exactly_one_row`, `band_labels_appear_on_exactly_one_row`, `reduction_sentriq_11_implies_9_via_10`, `sentriq_implied_dep_is_a_dim_marker_not_a_row` — all pass |
| 2 | Selection cursor over every phase (j/k, g/G, h/l, [/], Space); detail pane shows Goal/Needs/Unblocks/Parallel; no-deps phase gets an explanation instead of "Nothing in this milestone" | ✓ VERIFIED | `sentriq_phase_12_explains_it_has_no_deps`, `bracket_steps_within_the_wave_and_wraps`, `h_cycles_the_origin_needs_including_implied`, `l_cycles_unblocks`, `a_folded_phase_resolves_to_its_band_row` — all pass |
| 3 | 80-column stacked layout, ~100+-column side-by-side, no horizontal scrolling | ✓ VERIFIED | `mockup_c_stacked_at_80_columns`, `sentriq_and_daily_vow_hold_at_both_widths`, `mockup_a_side_by_side_at_120_columns`, `roadmap_view_never_draws_outside_its_rect`, `roadmap_view_never_panics_at_tiny_sizes` — all pass; no horizontal-scroll state exists in the touched files |
| 4 | `#### Build phase N` headings (ttbook) parse as phases | ✓ VERIFIED | `ttbook_build_phase_headings_parse_as_planned_phases`, `roadmap_model_for_ttbook_lists_build_phases_as_phases`, `ttbook_fixture_gsd_phase_list_is_unchanged_by_build_phase_support` (GSD-facing parser untouched) — all pass |
| 5 | PhaseList gone (content preserved in Roadmap header); Pipeline renamed Phases at position 2, shares selection; Archive lives in Docs › Milestones; every tab-index consumer agrees | ✓ VERIFIED | see D-B01–D-B11 rows above; the five archive debug-fix regression tests pass unmodified, opened via Docs › Milestones |

**Score:** 5/5 success criteria verified, 0 present-but-behavior-unverified.

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/roadmap_md.rs` | Goal parsing, Build-phase parsing, shipped-milestone facts | ✓ VERIFIED | 56 lib tests pass, incl. `parse_planned_build_phases`, `parse_phase_goals`, `shipped_milestones` |
| `src/state_reader/mod.rs` | `ProjectState.planned_phases`/`phase_goals`/`milestone_name` | ✓ VERIFIED | fields present, populated by `parse_project_state`, exercised by adapter tests |
| `src/ui/roadmap_graph.rs` | `layout_list`, `RoadmapModel`, navigation, legacy layout removed | ✓ VERIFIED | 39 lib tests pass; grep confirms no `layout_graph`/`RoadmapGraphWidget` remain (24-06 deletion) |
| `src/ui/roadmap_view.rs` | `RoadmapView` StatefulWidget, side-by-side/stacked panes | ✓ VERIFIED | 19 lib tests pass, incl. escape/panic/rect-bound tests |
| `src/ui/screens/detail.rs` | tab constants, `roadmap_model_for`, `render_roadmap`, `switch_to_sub_view` | ✓ VERIFIED | tab, cursor, adapter and render tests all pass (see test log above) |
| `src/ui/screens/render_escape_guard.rs` | `ALL_SUB_VIEWS` without `PhaseList`, new text paths covered | ✓ VERIFIED | 15 lib tests pass, incl. `the_planned_phase_arrives_whole_in_the_roadmap_detail_pane` |
| `src/app.rs` | `DetailSubView` w/o `PhaseList`, `#[default] RoadmapViz`, ported archive tests | ✓ VERIFIED | enum inspected directly; 5 named archive regression tests pass |
| `src/ui/screens/help.rs` | Roadmap key rows, `m` row, stale `r` row gone | ✓ VERIFIED | rows present; remaining `"r"` row is the unrelated dashboard "Start a driver run" binding |
| `README.md`, `docs/ARCHITECTURE.md` | 8-tab description, Docs › Milestones | ✓ VERIFIED | both files updated (grep confirms) |

## Key Link Verification

| From | To | Via | Status |
|------|----|----|--------|
| `roadmap_model_for` | `layout_list` (ProjectState → RoadmapModel) | adapter builds `ListInput` from state | ✓ WIRED — `roadmap_model_for_sentriq_uses_a_synthetic_band`, `roadmap_model_for_ttbook_lists_build_phases_as_phases` pass |
| Roadmap `Enter` (phase row) | Phases tab (`Pipeline`) | `tab_index(&DetailSubView::Pipeline)` | ✓ WIRED — `roadmap_enter_opens_the_selected_phase_in_phases` passes |
| Phases `j`/`k` selection | `roadmap_cursor` (write-back) | `share_pipeline_selection` | ✓ WIRED — `phases_selection_is_shared_back_to_the_roadmap` passes |
| Roadmap `Enter` (shipped row) | Docs › Milestones | `switch_to_sub_view(.., Archive, ..)` | ✓ WIRED — `enter_on_the_shipped_milestones_row_opens_docs_milestones`, `roadmap_enter_on_the_shipped_row_opens_the_archive_view` pass |
| `DetailScreen::opened_on(.., Archive, ..)` | Docs › Milestones | `switch_to_sub_view` direct call | ✓ WIRED — `opened_on_archive_lands_on_docs_milestones` passes |
| Docs tab `m` key | Files ⇄ Milestones toggle | `docs_sub_tab_strip` + key arm | ✓ WIRED — `m_switches_docs_between_files_and_milestones` passes |

## Behavioral Spot-Checks / Test Evidence

All checks below were run directly by the verifier (`rtk proxy cargo test --no-fail-fast --lib <filter>`), not taken from SUMMARY.md claims:

| Area | Filter | Result |
|------|--------|--------|
| Layout engine | `roadmap_graph` | 39 passed, 0 failed |
| Master/detail widget | `roadmap_view` | 19 passed, 0 failed |
| ROADMAP parser | `roadmap_md` | 56 passed, 0 failed |
| Tab index round-trip | `tab_index` | 2 passed, 0 failed |
| Tab-bar width arithmetic | `tab_bar_width` | 1 passed, 0 failed |
| Escape-guard probe | `render_escape_guard` | 15 passed, 0 failed |
| Digit-inert behavior | `inert` | 8 passed, 0 failed (incl. `nine_and_zero_are_inert_in_the_detail_view`) |
| Docs `m` switch | `m_switch` | 1 passed, 0 failed |
| `opened_on` Archive routing | `opened_on` | 1 passed, 0 failed |
| Mockups A/B/C fixtures | `mockup` | 7 passed, 0 failed |
| Sentriq fixture (SC-1, SC-2, SC-3) | `sentriq` | 11 passed, 0 failed |
| ttbook build-phase fixture (SC-4) | `build_phase` | 7 passed, 0 failed |
| Shipped-summary fixture (D-A07/D-B04) | `shipped` | 10 passed, 0 failed |
| Roadmap↔Phases shared selection | `pipeline_selected`/`share_pipeline`/manual read | `phases_selection_is_shared_back_to_the_roadmap` passed |
| 5 archive debug-fix regression tests | `archive_milestone`, `the_same_milestone_version_in_two_projects`, `an_in_place_reload_that_shrinks` | 5/5 passed (assertions untouched, opened via Docs › Milestones) |

Full workspace run already performed by the orchestrator on this HEAD (re-verified as current):
49 suites, 2342 passed, 1 failed (`envelope::policy::…the_config_section_constants_record_the_git_version_they_were_derived_against` — pinned local-git-vs-CI-git environment mismatch, unrelated to phase 24, documented in project memory), 15 ignored. Clippy `--all-targets --keep-going`: 11 pre-existing errors in `src/browser.rs`, `src/project_creator.rs`, `tests/envelope_*.rs` — none in phase 24 files, none new.

## Anti-Patterns Found

None blocking. `TBD`/`FIXME`/`XXX`/"placeholder" greps across all phase-24-touched files return only legitimate domain text (`"plans TBD"` empty-state literal, `U+XXXX` escape-format documentation, "planned placeholder" — the D-A12 build-phase term of art itself). No mouse-event code and no horizontal-scroll state were added, matching the D-A11/D-A13 prohibitions.

**Informational — not a gap:** 24-07's own "Inferred decisions" section (item 6) flags that the Roadmap tab's single-line footer is 91 cells with the experimental flag on / 87 with it off, so `[?]help` clips past 80 columns. This is the shared `footer_spans`/`build_footer` chrome used by *every* detail tab (Queue, Browse, Defaults already carry similarly long single-form hint strings — `footer_spans`'s doc comment states "only the Driver tab tiers on width today; the other tabs keep their single shipped form"), not new state introduced by this phase, and it clips via the existing `Paragraph` render (no panic, no scroll). Neither D-A09/D-A13 (which govern the Roadmap list+detail pane's own layout, confirmed rendering cleanly at 80×24 with no horizontal scroll) nor any other locked decision specifies a footer-width bound. The SUMMARY itself correctly scopes this as a follow-up, not phase-24 scope.

## Requirements Coverage

REQUIREMENTS.md carries no Phase 24 rows (phase uses locked decisions as its requirement set per ROADMAP.md). See the Requirement-ID Cross-Reference table above — all 27 D-A*/D-B* IDs are claimed by at least one plan and satisfied with test evidence. No orphaned requirements.

## Human Verification Required

None. All checkable items were settled with direct test execution and code inspection (file:line citations above). The one open item from the executors (footer width) was resolved by inspection rather than escalated, per instruction, because it is observable/checkable code, not a subjective call.

## Gaps Summary

No gaps. All 5 ROADMAP success criteria and all 27 locked decisions (D-A01-D-A15, D-B01-D-B12) are implemented and covered by passing tests that the verifier ran directly (not SUMMARY.md claims). The legacy left-to-right graph layout was deleted (D-A15), the PhaseList tab is gone with its content preserved in the Roadmap header (D-B02/D-B08), Archive now lives at Docs › Milestones with all five archive debug-fix regression tests passing unmodified (D-B04/D-B06), and every tab-index consumer (round-trip tests, width arithmetic, escape-guard probe, help text, README, ARCHITECTURE.md) agrees with the final 8-tab-plus-Driver order (D-B05).

---

_Verified: 2026-09-24T05:10:00Z_
_Verifier: Claude (gsd-verifier)_
