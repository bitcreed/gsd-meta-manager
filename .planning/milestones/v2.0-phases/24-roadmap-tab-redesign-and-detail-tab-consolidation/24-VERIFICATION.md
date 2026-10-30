---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
verified: 2026-10-29T00:00:00Z
status: passed
score: 5/5 roadmap success criteria verified; 27/27 locked decisions (D-A01-D-A15, D-B01-D-B12) accounted for (25 satisfied as written, 2 superseded by later deliberate work with the goal behaviour intact)
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
covered_digest: "v4:sha256:9f41527cd7a32cca94cfc93f828d02a694a499891cc70723a425979c0617f605"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation Verification Report

**Phase Goal:** The Roadmap tab becomes the primary per-project view — a master/detail phase
list with a git-log-style dependency graph, a selection cursor and a detail pane that says what
each phase needs, unblocks and can run alongside — and the detail tabs consolidate from ten to
eight (Roadmap · Phases · Backlog · Git · Queue · Sess · Cfg · Docs, plus Driver).

**Verified:** 2026-10-29 (HEAD 116f297; previous pass 2026-10-28 at 51ae4c5)
**Status:** passed
**Re-verification:** Yes. The 2026-10-28 report went `stale` because quick task 261029-1dp
(ce86206, "drop dead AgentRuntime key") changed two covered files. Delta check: `git diff 51ae4c5 HEAD`
over every covered source file shows exactly `src/ui/screens/detail.rs` (+5 -4) and
`src/ui/screens/mod.rs` (+2 -2); `app.rs`, `state_reader/*`, `ui/mod.rs`, `roadmap_graph.rs`,
`roadmap_view.rs`, `help.rs`, `render_escape_guard.rs`, `README.md` and `docs/ARCHITECTURE.md` are byte-identical.
The two changed hunks are comment/assert-message rewording only (a Driver-tab opt-in test doc comment and its
assert message in `detail.rs` tests, and the `AppContext::experimental` doc comment in `mod.rs`); no code path,
tab array, key arm or render function was touched. The change is mechanical as expected, and every must-have was
nevertheless re-checked against HEAD (fresh test runs and anchors below), not carried over.

## Evidence base (this run)

- **Regression gate (orchestrator, cited):** `rtk proxy cargo test --no-fail-fast` at HEAD 116f297: 55 suites, 3216 passed, 0 failed, 17 ignored. Not re-run in full.
- **Gates refreshed this round (cited):** security e4b59a6 (`threats_open: 0`, AR-24-01 accepted); UI delta 6bb6cd7 (261029-1dp has no TUI surface, score 18/24 unchanged); code review 6e8e1c0 + disposition 116f297 (new IN-06, IN-07 are info only, recorded open; nothing defeats a success criterion).
- **Structural anchors re-confirmed at HEAD:** `TAB_COUNT = 9` (`detail.rs:456`), `TAB_LABELS_FULL`/`COMPACT` (`:479`/`:494`), `DRIVER_TAB_INDEX = 8` (`:499`); `#[default] RoadmapViz` (`app.rs:19-20`) and no `DetailSubView::PhaseList` (only `archive::ArchiveDepth::PhaseList` at `detail.rs:2153`); `layout_list` (`roadmap_graph.rs:921`), `transitive_reduction` (`:576`), `resolve_deps:59`, `break_cycles:106`, `longest_path_layers:190`; `needs_rows` (`roadmap_view.rs:1238`); `handle_mouse` (`detail.rs:6348`); `layout_graph`/`RoadmapGraphWidget` grep count 0.
- **Single named tests, fresh run at HEAD, each 1 passed / 0 failed:** `daily_vow_phase_20_has_exactly_one_row`, `sentriq_phase_12_needs_what_gsd_1_15_reads_from_its_prose`, `roadmap_enter_opens_the_selected_phase_in_phases`, `phases_selection_is_shared_back_to_the_roadmap`, `m_switches_docs_between_files_and_milestones`, `nine_and_zero_are_inert_in_the_detail_view`, `roadmap_tab_draws_the_list_and_v_is_unbound`, `sentriq_and_daily_vow_hold_at_both_widths`, `the_tab_bar_widths_are_the_label_arrays_own_arithmetic`, `ttbook_build_phase_headings_parse_as_planned_phases`, `opened_on_archive_lands_on_docs_milestones`, `every_tab_index_round_trips_through_its_sub_view`, `mockup_c_stacked_at_80_columns`.
- **Targeted filters (`cargo test --lib --no-fail-fast <name>`, re-run this round, identical to the prior pass):** `roadmap_graph` 53, `roadmap_view` 37, `roadmap_md` 77, `render_escape_guard` 18, `inert` 18, `mockup` 7, `build_phase` 7, `shipped` 24, `tab_index` 2, `tab_bar_width` 1 (all 0 failed).
- **Live drive:** not repeated this round. The change set cannot affect rendering (comments and an assert message only; `roadmap_view.rs`, `roadmap_graph.rs`, `app.rs` unchanged), so the 2026-10-28 tmux observations stand, backed by the render tests above. Those observations (debug binary, isolated `--config`, `tests/fixtures/roadmaps/sentriq-ROADMAP.md`, 120x34, 80x24 and 80x30): tab bar `1:Roadmap | 2:Phases | 3:Backlog | 4:Git | 5:Queue | 6:Sess | 7:Cfg | 8:Docs`; header line (`sentriq · v0.12 Actuation Routines · phase 9 planning · 0 of 4 phases done`); "Start now" line; collapsed shipped row `▸ v0.11  1 milestone · 4 phases shipped`; `▾ v0.12 … 0/4 ━━━` band; one row per phase; lane/marker column; detail pane with Goal / Needs / Unblocks / Parallel; `(implied via 11)` under Needs for phase 12 (phase 9 appears once); `G` moving the cursor to phase 12; `Enter` opening Phases with `> P12`; `9`/`0` inert; `8` then `m` opening Docs with `Files │ [Milestones]`. `[inferred — cleanup 2026-10-28]`
- **Security:** 24-SECURITY.md (citations refreshed e4b59a6): `threats_open: 0`, one accepted risk AR-24-01.
- **Anti-pattern scan:** `TBD` hits are the legitimate user-facing `plans TBD` literal (`roadmap_view.rs:1163`), parser test strings, and `U+XXXX` escape documentation; no `FIXME`/`XXX` debt markers and no `TODO` stubs (re-grep of `roadmap_graph.rs`/`roadmap_view.rs` after excluding those literals: 0 hits).

## Requirement-ID Cross-Reference (D-A*/D-B*, 24-CONTEXT.md)

`.planning/REQUIREMENTS.md` has no "Phase 24" rows (grep count 0), consistent with ROADMAP's
"Requirements: TBD (locked decisions in `24-CONTEXT.md`)". The 27 locked decisions are treated as the
requirement set. No ID is orphaned and no plan claims an ID that CONTEXT.md does not define.

| ID | Status | Evidence at HEAD |
|----|--------|------------------|
| D-A01 list pane + lane column | ✓ | live render rows `glyph lanes marker # name plans wave`; `layout_list` (`roadmap_graph.rs:921`); `mockup_a_side_by_side_at_120_columns` |
| D-A02 detail pane Goal/Needs/Unblocks/Parallel | ✓ | live render (120 and 80 cols); `needs_rows` (`roadmap_view.rs:1238`); `mockup_c_stacked_at_80_columns` |
| D-A03 Goal parsing, both bold forms | ✓ | `a_goal_is_read_in_both_bold_forms`; live Goal paragraph from fixture |
| D-A04 status glyphs ●◉○◌ | ✓ | live render shows ◉ and ◌; `PhaseStatus` |
| D-A05 selection markers | ✓ (glyph variant) | selected row `►`, deps `⇡`, unblocks `⇣`, implied `∙` come from the shared glyph registry (`MARK_SELECTED`/`MARK_UNBLOCKS`/`MARK_IMPLIED`, `roadmap_graph.rs:230-236`; quick 261021-1id re-routed them through `glyphs.rs`). Same semantics as `▶`/`↑`/`↓`/`·`; `h_cycles_the_origin_needs_including_implied`, `l_cycles_unblocks` |
| D-A06 "Start now" line | ✓ | live line `Start now: ◉9 …`; `facts_for_mockup_c_phase_12` asserts `start_now` |
| D-A07 bands once each, shipped fold | ✓ | live `▸ v0.11 1 milestone · 4 phases shipped` + `▾ v0.12 …`; `unfolding_the_shipped_summary_reveals_one_row_per_shipped_milestone`, `daily_vow_shipped_milestones_are_the_five_before_v1_5`, `band_labels_appear_on_exactly_one_row` (`shipped` filter 24/24) |
| D-A08 transitive reduction, lane rules, no dupes | ✓ | `reduction_daily_vow_23_implies_20_via_21`, `reduction_sentriq_11_implies_9_via_10`, `daily_vow_phase_20_has_exactly_one_row` (re-run, pass), `sentriq_implied_dep_is_a_dim_marker_not_a_row` |
| D-A09 80-col stacked / ~100+ side-by-side | ✓ | live 80x24 and 80x30 stacked, 120x34 side-by-side; `sentriq_and_daily_vow_hold_at_both_widths` (re-run, pass) |
| D-A10 keys | ✓, one clause superseded | `j/k g/G h/l [/] Space Enter` tests pass (`bracket_steps_within_the_wave_and_wraps` etc.), `G` and `Enter` driven live. **`v` box-view toggle no longer exists:** quick 261022-wtw deliberately deleted `roadmap_widget.rs` and the Detail box view (commits 084fb4f, 69ac009), and `roadmap_tab_draws_the_list_and_v_is_unbound` (re-run, pass) pins it. Not a gap against the goal; see WR-05 below for the stale title hint. `[inferred — cleanup 2026-10-28]` |
| D-A11 mouse click-to-select out of scope | ✓ superseded | D-A11 deferred mouse support as follow-up scope, not a prohibition. Quick 260926-dyf later delivered it (`DetailScreen::handle_mouse`, `detail.rs:6348`). `[inferred — cleanup 2026-10-28]` |
| D-A12 `#### Build phase N` parsing | ✓ | `ttbook_build_phase_headings_parse_as_planned_phases` (re-run, pass), `roadmap_model_for_ttbook_lists_build_phases_as_phases`, `roadmap_enter_on_a_build_phase_explains_itself`, `ttbook_fixture_gsd_phase_list_is_unchanged_by_build_phase_support`; `build_phase` filter 7/7 |
| D-A13 no horizontal scroll | ✓ | no `hscroll`/`horizontal_scroll` in `roadmap_graph.rs`, `roadmap_view.rs`, `detail.rs` (grep 0 at the prior pass; these files are unchanged apart from the two comment hunks); `roadmap_view_never_draws_outside_its_rect` |
| D-A14 four bugs gone | ✓ | see SC-1/SC-2 |
| D-A15 keep parse/cycle/wave, rewrite layout | ✓ | `resolve_deps:59`, `break_cycles:106`, `longest_path_layers:190` retained; `layout_graph`/`RoadmapGraphWidget` absent from `src` (grep 0 this run), `layout_list` is the successor |
| D-B01 new tab order + Driver | ✓ | `TAB_LABELS_FULL`/`TAB_LABELS_COMPACT` (`detail.rs:479-497`), `TAB_COUNT = 9`, `DRIVER_TAB_INDEX = 8`; `every_tab_index_round_trips_through_its_sub_view` (re-run, pass) |
| D-B02 PhaseList removed, header moved | ✓ | `DetailSubView` has no `PhaseList` (`app.rs:16-`; grep this run: only `archive::ArchiveDepth::PhaseList` at `detail.rs:2153`); live Roadmap header lines (summary + Path) |
| D-B03 Pipeline→Phases at position 2, shared selection | ✓ via Enter; see WR-07 | `roadmap_enter_opens_the_selected_phase_in_phases`, `phases_selection_is_shared_back_to_the_roadmap` (both re-run, pass), live `Enter` on phase 12 opened Phases with `> P12` |
| D-B04 Archive → Docs › Milestones | ✓ | live `Files │ [Milestones]`; `m_switches_docs_between_files_and_milestones`, `opened_on_archive_lands_on_docs_milestones` (re-run, pass), `enter_on_the_shipped_milestones_row_opens_docs_milestones`, `roadmap_enter_on_the_shipped_row_opens_the_archive_view` |
| D-B05 every tab-index consumer | ✓ | round-trip, width-arithmetic and escape-guard tests pass; `README.md:95` "8-tab detail view", `README.md:47-48,255` Files/Milestones (README unchanged since prior pass) |
| D-B06 Archive move last | ✓ | 24-07 is wave 4; archive regression tests (`archive_milestone` etc.) pass in the full-suite gate |
| D-B07 default tab Roadmap | ✓ | `#[default] RoadmapViz` (`app.rs:19-20`, re-confirmed); live open lands on Roadmap |
| D-B08 PhaseList-unique content preserved | ✓ | `[Not started]`/stage badge in the detail-pane status line (live); Paused/change-tracker in header; empty states covered by `render_escape_guard` (18/18) |
| D-B09 in-memory view state | ✓ | no persistence change; `Agents`/`Archive` map onto Sessions/Docs with sub-tab active (`app.rs` doc comments) |
| D-B10 digits 9/0 inert | ✓ | `nine_and_zero_are_inert_in_the_detail_view` (re-run, pass); live `9`/`0` stayed on Roadmap |
| D-B11 compact labels, recomputed widths | ✓ | `TAB_LABELS_COMPACT` `1:Rd … 8:Dc D:Dr`; `the_tab_bar_widths_are_the_label_arrays_own_arithmetic` (re-run, pass) |
| D-B12 glyph from `phase_marker` | ✓ | `state.phase_marker(p)` drives status; live render matches disk-inferred stage |

## Goal Achievement — ROADMAP Success Criteria

| # | Success Criterion | Status | Evidence |
|---|-------------------|--------|----------|
| 1 | Every phase once, each milestone label once, implied deps as dim marker (daily-vow `20`-twice, sentriq `9→11`) | ✓ VERIFIED | `daily_vow_phase_20_has_exactly_one_row` (re-run, pass), `band_labels_appear_on_exactly_one_row`, `reduction_sentriq_11_implies_9_via_10`, `sentriq_implied_dep_is_a_dim_marker_not_a_row`; live: phase 9 occurs once, `(implied via 11)` in the Needs row, no `─►` |
| 2 | Cursor over every phase (j/k g/G h/l [/] Space); detail pane Goal/Needs/Unblocks/Parallel; no-deps phase explained | ✓ VERIFIED | navigation tests above; live render. The no-deps wording is exercised by `mockup_c_stacked_at_80_columns` (re-run, pass; "nothing declared", "no edge either way; can run any time") and seen live on phase 9 ("nothing declared", "(no deps; can run any time)"). **Note:** the original sentriq example (phase 12) now correctly *has* deps, since gsd-core 1.15's `PHASE_DEP_REF` reads the "blocks on nothing in Phases 9-11" prose as 9 and 11. The test was rewritten to `sentriq_phase_12_needs_what_gsd_1_15_reads_from_its_prose` (`detail.rs`, re-run, pass), and the capability survives on phases that genuinely declare nothing. Deliberate parity change, not a regression. `[inferred — cleanup 2026-10-28]` |
| 3 | 80-col stacked, ~100+ side-by-side, no horizontal scrolling | ✓ VERIFIED | live 80x24, 80x30 (stacked, all four detail rows visible) and 120x34 (side-by-side); `mockup_c_stacked_at_80_columns`, `mockup_a_side_by_side_at_120_columns`, `roadmap_view_never_panics_at_tiny_sizes`, `sentriq_and_daily_vow_hold_at_both_widths` |
| 4 | `#### Build phase N` headings parse as phases | ✓ VERIFIED | `ttbook_build_phase_headings_parse_as_planned_phases` (re-run, pass), `roadmap_model_for_ttbook_lists_build_phases_as_phases`, `build_phase` filter 7/7 |
| 5 | PhaseList gone with content preserved; Pipeline→Phases at 2, shares selection (Enter opens it); Archive in Docs › Milestones; all consumers agree | ✓ VERIFIED | D-B01..D-B12 rows; live Enter→Phases `> P12`; live Docs `[Milestones]`; the `(Enter opens it)` clause defines the shared-selection mechanism and works (see WR-07 caveat) |

**Score:** 5/5 success criteria verified, 0 present-but-behavior-unverified.

## Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/state_reader/roadmap_md.rs` | ✓ VERIFIED | Goal, Build-phase and shipped-milestone parsing present; `roadmap_md` filter 77/77; unchanged |
| `src/state_reader/mod.rs` | ✓ VERIFIED | planned phases / goals / milestone name wired (adapter tests pass); unchanged |
| `src/ui/roadmap_graph.rs` | ✓ VERIFIED | `layout_list`, `transitive_reduction`, cursor navigation; successor of `layout_graph`; 53 tests; unchanged |
| `src/ui/roadmap_view.rs` | ✓ VERIFIED | `RoadmapView` master/detail widget with side-by-side and stacked panes; 37 tests; unchanged |
| `src/ui/screens/detail.rs` | ✓ VERIFIED | tab arrays, `roadmap_model_at`, `roadmap_nav`/`roadmap_activate`, `switch_to_sub_view`; only a test doc comment and assert message changed (ce86206) |
| `src/ui/screens/render_escape_guard.rs` | ✓ VERIFIED | 18 tests incl. `the_planned_phase_arrives_whole_in_the_roadmap_detail_pane`; unchanged |
| `src/app.rs`, `help.rs`, `mod.rs`, `README.md`, `docs/ARCHITECTURE.md` | ✓ VERIFIED | enum, defaults, 8-tab docs consistent; `mod.rs` changed by one doc comment only |

## Key Link Verification

| From | To | Status |
|------|----|--------|
| `roadmap_model_at` → `layout_list` | ProjectState to RoadmapModel | ✓ WIRED (adapter tests; live render) |
| Roadmap `Enter` (phase) → Phases | `roadmap_activate` sets `pipeline_selected` | ✓ WIRED (`roadmap_enter_opens_the_selected_phase_in_phases`, re-run, pass; live) |
| Phases `j`/`k` → Roadmap cursor | `share_pipeline_selection` | ✓ WIRED (`phases_selection_is_shared_back_to_the_roadmap`, re-run, pass) |
| Roadmap `Enter` (shipped row) → Docs › Milestones | `switch_to_sub_view(Archive)` | ✓ WIRED (test; Docs sub-tab seen live) |
| Docs `m` → Files ⇄ Milestones | key arm | ✓ WIRED (`m_switches_docs_between_files_and_milestones`, re-run, pass; live) |

## Data-Flow Trace (Level 4)

Phase rows, goals, plan counts, waves and status come from `ProjectState` read off the real `.planning/` files; the live drive (2026-10-28) against a real ROADMAP/STATE fixture showed fixture goals, plan counts (`0/2`) and the fixture milestone name in the rendered output. No state-reader or render code changed since. Not static: ✓ FLOWING.

## Advisories (code review, UI review): none defeats a success criterion

| Finding | Source | Defeats an SC? | Disposition |
|---------|--------|----------------|-------------|
| Footer hint clipped at <=80 cols (`[e]nque…`, `[?]help` lost; seen live at 80x24) | UI-REVIEW | No. SCs 1-5 and D-A09/D-A13 govern the list and detail pane, not footer width. Shared chrome, no panic, no scroll. | Advisory |
| Stacked detail pane collapses to one Goal row at 80x30/60x22 | UI-REVIEW | No. Live 80x24 and 80x30 on sentriq showed Goal, Needs, Unblocks and Parallel all present. The collapse is real for taller goals, but SC3 requires stacking without horizontal scroll, and that holds. | Advisory |
| Done phase without a directory shows `[Not started]` (`detail.rs:2721`) | UI-REVIEW | No. Cosmetic status-badge mismatch; the glyph (D-B12) is correct. | Advisory |
| WR-07: Roadmap `j`/`k` does not write back to the Phases selection; pressing `2` or `→` after moving the cursor opens Phases on the stale phase. **Confirmed live** (cursor on 12, `2` opened Phases on P9). | REVIEW | No, narrowly. SC5 and D-B03 define sharing as "Enter opens it", and that works. The digit/arrow path is a real inconsistency and the cache doc comment overstates the sharing. | **Warning**, worth fixing |
| WR-05: pane title advertises `v list` though `v` is unbound (seen live) | REVIEW | No. Stale hint left after the box view was deleted. | Warning |
| WR-01..04 (BandKey collision on case-folded Unicode labels, `declared_phase_count` u32 overflow, build-phase range self-inclusion, GSD entry inheriting `#### Build phase` lines) | REVIEW | No. All require hostile or unusual ROADMAP input; none occurs in the fixtures or the live drive. WR-02 could panic a debug build on `Phases 0-4294967295`. | Warning, open |
| WR-06 wide-glyph column drift | REVIEW | No. Deferred per D-21-53. | Deferred |
| IN-01..05 | REVIEW | No. | Info |
| IN-06, IN-07 (review 6e8e1c0, this round) | REVIEW | No. Info only, recorded open in 24-REVIEW-DISPOSITION.md (116f297). | Info |

Disposition of all review findings is recorded in 24-REVIEW-DISPOSITION.md (original 12: 11 open, 1 deferred; this
round adds IN-06 and IN-07, info only, recorded open). None was fixed in this verification, per instruction.

## Behavioral Spot-Checks / Probes

Targeted `cargo test --lib --no-fail-fast` runs listed under "Evidence base", plus the 2026-10-28 tmux drive. No phase
probes (`scripts/*/tests/probe-*.sh`) are declared by the phase PLANs. Probe execution: not applicable.

## Human Verification Required

None. Everything was settled with tests and rendering evidence. The advisories above are not
operator-signature items; no subjective quality call is required to accept the phase.

## Gaps Summary

No gaps. The phase goal survives in the HEAD code: the master/detail Roadmap with lane-column graph, selection
cursor and Goal/Needs/Unblocks/Parallel detail pane; eight tabs plus Driver; PhaseList and Archive folded away.
`PhaseList`, `layout_graph` and `RoadmapGraphWidget` were removed by later work and their behaviour lives on in
`layout_list` and `RoadmapView`. Quick task 261029-1dp touched only comments and an assert message in covered files and
introduced no regression. The one real behavioural wart (WR-07, one-way selection sharing outside `Enter`) is a
follow-up, not a failure of any success criterion.

---

_Verified: 2026-10-29_
_Verifier: Claude (gsd-verifier)_
