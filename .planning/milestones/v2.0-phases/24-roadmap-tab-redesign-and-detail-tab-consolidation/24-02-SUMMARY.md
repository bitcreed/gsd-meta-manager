---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 02
subsystem: ui
tags: [roadmap, layout, dag, transitive-reduction, git-log-lanes, navigation, escaping]
status: complete
requires:
  - phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
    provides: "nothing at compile time (wave 1, self-contained inputs); 24-01's reader fields are adapted into ListNode by plan 24-05"
provides:
  - "roadmap_graph::{ListNode, BandInput, ListInput} inputs"
  - "roadmap_graph::{RoadmapModel, ListRow, PhaseFacts, BandFacts, PhaseStatus, BandKey, CursorTarget} model"
  - "roadmap_graph::layout_list, is_folded, lane_text"
  - "RoadmapModel navigation: visible_targets, resolve_cursor, row_of, step, first_target, last_target, phase_index, edge_jump, wave_step, unfold_for"
  - "roadmap_graph::{EdgeDir, EdgeWalk}"
  - "Glyph consts GLYPH_* / MARK_* / BAND_* / PARALLEL_SEP / LANE_* (box-drawing set)"
  - "PhaseStatus::glyph() helper"
affects: [24-04, 24-05, 24-06]
actuals:
  tokens: 19238
  tasks: 3
  commits: 5
plan_head_before: 495b3360fcd12e4168d9e4b27fdc8114ed63f70d
tech-stack:
  added: []
  patterns:
    - "Pure list model with no ratatui types, pinned by exact `lane_text` Vec<String> assertions"
    - "Lanes computed on the unfolded row sequence; folding only drops rows afterwards"
    - "Cursor targets are keys (phase_key / BandKey), never row indices"
key-files:
  created: []
  modified:
    - src/ui/roadmap_graph.rs
key-decisions:
  - "Parallel = the other phases of the SAME wave (D-A02 as locked by the plan), not RESEARCH Pitfall 8's incomparable set; Mockup C's `◉ 9 ◌ 10 ◌ 11` for phase 12 therefore becomes `[9]` in the model"
  - "List order (unblocks, parallel, start_now, [ / ]) is the row-sequence order: shipped bands, then other bands, each in band order, then band-less phases; RoadmapModel recomputes it privately instead of carrying an extra field"
  - "Ready/Blocked is judged over the acyclic, unreduced in-graph parents, so a broken cycle never leaves both of its phases blocked forever"
  - "BandKey::Named and PhaseFacts.key hold raw-derived text for matching only; widgets must never draw them"
requirements-completed: [D-A01, D-A02, D-A04, D-A05, D-A06, D-A07, D-A08, D-A10, D-A14, D-A15, D-B12]
coverage:
  - id: D1
    description: "Transitive reduction with (dep, via) witnesses; waves = longest path + 1"
    requirement: D-A08
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#reduction_daily_vow_23_implies_20_via_21, reduction_sentriq_11_implies_9_via_10, reduction_ttbook_shape, waves_are_longest_path_plus_one"
        status: pass
    human_judgment: false
  - id: D2
    description: "Git-log lanes reproduce Mockups A, B, C and the no-deps zig-zag exactly; no phase in two rows"
    requirement: D-A14
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#lanes_mockup_a_with_bands, lanes_daily_vow_without_bands, lanes_sentriq_without_bands, lanes_roots_zig_zag, daily_vow_phase_20_has_exactly_one_row"
        status: pass
    human_judgment: false
  - id: D3
    description: "One band row per milestone, one folded shipped-summary row, fold toggles"
    requirement: D-A07
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#lanes_mockup_b_with_shipped_summary, unfolding_the_shipped_summary_reveals_one_row_per_shipped_milestone, lanes_sentriq_with_synthetic_band, folding_a_band_hides_its_rows_but_not_the_lanes_elsewhere, empty_bands_never_draw, band_labels_appear_on_exactly_one_row"
        status: pass
    human_judgment: false
  - id: D4
    description: "Status from PhaseMarker and per-phase facts (needs, implied, external, unblocks, parallel, start_now)"
    requirement: D-B12
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#facts_for_mockup_b_phase_23, facts_for_mockup_c_phase_12, list_external_dep_is_listed_not_drawn"
        status: pass
    human_judgment: false
  - id: D5
    description: "Cursor navigation for j/k, g/G, h/l, [/], fold resolution and unfold_for"
    requirement: D-A10
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#default_cursor_is_the_active_phase, step_skips_connector_rows, h_cycles_the_origin_needs_including_implied, l_cycles_unblocks, bracket_steps_within_the_wave_and_wraps, a_folded_phase_resolves_to_its_band_row, unfold_for_reveals_a_hidden_phase"
        status: pass
    human_judgment: false
  - id: D6
    description: "Only escaped text is stored for display (T-24-06); malformed graphs terminate (T-24-07)"
    verification:
      - kind: unit
        ref: "src/ui/roadmap_graph.rs#list_model_stores_only_escaped_text, list_cycle_terminates_with_one_note, list_self_dependency_is_noted, list_large_chain_stays_one_lane, list_many_roots_zig_zag_within_two_lanes"
        status: pass
    human_judgment: false
duration: 14 min
completed: 2026-09-24
---

# Phase 24 Plan 02: Roadmap List Model Summary

**`roadmap_graph.rs` now has a pure vertical list model: transitive reduction with "implied via" witnesses, git-log lanes that reproduce Mockups A, B and C exactly, milestone bands with one collapsed shipped-summary row, per-phase facts, and key-based cursor navigation for every Roadmap key. The legacy left-to-right layout still compiles for its callers until plan 24-06.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-09-24T03:12:19Z
- **Completed:** 2026-09-24T03:26Z
- **Tasks:** 3/3
- **Files modified:** 1 (`src/ui/roadmap_graph.rs`)

## Accomplishments

- `layout_list` runs the kept D-A15 pipeline (`resolve_deps` → `break_cycles` → `longest_path_layers`), then a new `transitive_reduction`. Results: daily-vow 23 gets `(20 via 21)`, sentriq 11 gets `(9 via 10)`, ttbook 10 and 11 get `(8 via 9)` and 12 gets `(9 via 10)`.
- A git-log lane assigner over the unfolded row sequence:
  - A node sits on its lowest incoming lane, and several incoming lanes produce one merge connector (`├─┘`, `├─┴─┘`).
  - A node with several children produces one fork connector (`├─┐`, `├─┼─┐`).
  - A root never takes the previous phase row's lane.
  - Band rows pass active lanes through (`  │ │     [M4]`).
- Bands:
  - Shipped milestones collapse into one `ShippedSummary` row, folded by default (`v1.0 … v1.4`, 5 milestones, 17 phases).
  - A non-shipped band with no phases draws nothing.
  - Folding drops rows only after layout, so lanes elsewhere never move.
- `PhaseFacts` and model totals: escaped id/name/goal/badge/external, wave, status, needs, implied, unblocks, parallel, `no_deps`, `no_edges`, `last_in_band`, `start_now`, `max_wave`, `done`/`total`, and one bounded cycle note.
- Status comes from the caller's `PhaseMarker` (D-B12). An external dependency counts as satisfied.
- Navigation on the model works on keys, not row indices:
  - `resolve_cursor` defaults to the active phase. A fold-hidden phase resolves to its band row, or to the shipped summary.
  - `step` skips connector rows and clamps at both ends.
  - `edge_jump` cycles the ORIGIN's needs plus implied deps, or its unblocks.
  - `wave_step` moves through the same wave and wraps.
  - `unfold_for` opens whatever fold hides a phase.
- `resolve_deps` now takes `(id, deps)` pairs, so the legacy `layout_graph` and the new model share it. All 22 legacy `roadmap_graph_*` tests pass unchanged.

## Task Commits

1. **Task 1 (tracer): band-less list end to end.** `bc3077a` feat. The tracer gate re-ran `cargo test --lib ui::roadmap_graph` (32 passed) before moving on.
2. **Task 2 (TDD): bands, shipped summary, folding.** RED `1949dcf` (7 target tests failed on assertions, `check tdd-red-evidence` returned `RED_EVIDENCE_OK`), then GREEN `5500d56`.
3. **Task 3 (TDD): navigation, robustness, escaping.** RED `bd2eb70` (8 navigation tests failed on assertions, `RED_EVIDENCE_OK`), then GREEN `28f14d5`.

## TDD Gate Compliance

- Task 2 went RED (`test(24-02)` `1949dcf`), then GREEN (`feat(24-02)` `5500d56`). No refactor commit was needed.
- Task 3 went RED (`test(24-02)` `bd2eb70`), then GREEN (`feat(24-02)` `28f14d5`).
  - The RED commit carried the navigation method signatures as stubs (returning `None` or `from.clone()`), because Rust cannot compile tests against methods that don't exist. The failures were assertion failures, not build errors.
  - The robustness and escaping tests passed at RED. That is intended: they pin behaviour Tasks 1 and 2 already delivered. The RED targets were the eight navigation tests.
- RED evidence: cargo output converted to TAP. `lanes_mockup_a_with_bands` and `default_cursor_is_the_active_phase` were the named targets, and both returned `RED_EVIDENCE_OK`.

## Verification

- `rtk proxy cargo test --lib ui::roadmap_graph`: 56 passed, 0 failed. That is the 22 legacy tests plus 34 new ones.
- `rtk proxy cargo test --no-fail-fast`: 49 suites, 2303 passed, 1 failed, 15 ignored. The baseline was 2269 passed, and the difference is exactly the 34 new tests. The single failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy -- -D warnings`: clean.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: 10 errors, all in the pre-existing files (`src/browser.rs` ×3, `src/project_creator.rs`, `tests/envelope_*.rs`). The baseline was 11, and none of the errors is in `roadmap_graph.rs`.
- `git diff --stat 495b336 -- . ':!.planning'` lists only `src/ui/roadmap_graph.rs`. `detail.rs` and `ui/mod.rs` were not touched, and `cargo build` succeeds.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] clippy `cloned_ref_to_slice_refs` in two test helper calls**
- **Found during:** Task 3 verification (`clippy --all-targets`).
- **Issue:** `toggles(&[x.clone()])` in `is_folded_defaults_and_toggles` (Task 2's test) and in `a_folded_phase_resolves_to_its_band_row`.
- **Fix:** switched to `std::slice::from_ref(&x)`.
- **Files modified:** `src/ui/roadmap_graph.rs`
- **Commit:** `28f14d5`

### Additions beyond the named tests

- `daily_vow_phase_20_has_exactly_one_row` (Task 1). Its acceptance criterion asked for this assertion.
- `is_folded_defaults_and_toggles` (Task 2), which pins the `is_folded` behaviour.

### Transient state

- The Task 1 commit `bc3077a` built with two dead-code warnings: `fold_toggles` was unused, and the `Slot::Summary`/`Slot::Band` variants were never constructed. Both are bands scaffolding, and Task 2's GREEN commit `5500d56` resolves them. They were left in place so Task 2 could have a genuine RED.

## Inferred decisions (for audit)

1. **Parallel means the same wave (D-A02 as locked in the plan).** This follows the plan's explicit override of RESEARCH Pitfall 8. As a result, sentriq 12's Parallel is `[9]`, not Mockup C's `◉ 9 ◌ 10 ◌ 11`. The widget (24-04) can still write "(no edge either way; can run any time)" using `no_edges`.
2. **"List order" is the row-sequence order:** shipped bands first, then the other bands, each in band order, then band-less phases, with input order inside each group. `unblocks`, `parallel`, `start_now`, `wave_step` and the default cursor all use it. `RoadmapModel` recomputes this order in a private `list_order()` instead of adding a field to the binding struct.
3. **Ready vs Blocked uses the acyclic, unreduced in-graph parents** (after cycle breaking). Implied deps count, and a broken cycle cannot leave both of its phases blocked forever.
4. **`no_edges` asks whether any phase declares this one,** checked over the acyclic parents. An edge dropped by cycle breaking does not count. `no_deps` is exactly "no needs, no implied, no external", as the plan words it.
5. **`last_in_band` is false for band-less phases.** Without a band there is no "(last in <milestone>)" to show.
6. **The default cursor looks for the Active phase over the full list, hidden phases included,** then settles on the row that folds it. Without an Active phase it takes the first phase that is not Done, else the first visible target. An empty model resolves to `None`.
7. **An `EdgeWalk` continues only if all three hold:** same `dir`, the cursor sits on `walk.target`, and the origin still exists. Otherwise it restarts at the cursor. `h`/`l`/`[`/`]` from a band row returns `None`.
8. **Keys are logic-only.** `PhaseFacts.key` (a `phase_key` of the raw id) and `BandKey::Named` (the lower-cased trimmed raw label, per the interface) may carry raw bytes. `list_model_stores_only_escaped_text` therefore checks every display string and deliberately excludes keys. Plans 24-04, 24-05 and 24-06 must never draw a key.
9. **Summary `phases` = Σ max(declared, listed), using saturating addition.** When the summary is unfolded, each shipped band draws its row even if it lists no phases, which is what the unfold test expects.
10. **Stored `lanes` strings are trimmed of trailing blanks.** Padding to a lane budget is the widget's job (24-04).
11. **Extra public items beyond the interface:** `PhaseStatus::glyph()`, named `LANE_*` consts (plus `LANE_UP_RIGHT` `└`, kept so the junction mapping covers every case), and `Default` on `RoadmapModel` and `ListInput`.
12. **The file was run through rustfmt.** It was rustfmt-clean at the base commit, and the rest of the repo is not fmt-enforced.

## Known Stubs

None. The Task 3 RED stubs were replaced in `28f14d5`.

## Threat Flags

None. There is no new surface: the model is pure, and T-24-06, T-24-07 and T-24-08 are covered by `list_model_stores_only_escaped_text`, the cycle, large-chain and many-roots tests, and char-based width measurement.

## Next Phase Readiness

- 24-04 (widget) can draw `RoadmapModel.rows` and `phases` directly, using the glyph consts and `PhaseStatus::glyph()`.
- 24-05 (keys) maps `j`/`k` to `step(±1)`, `g`/`G` to `first_target`/`last_target`, `h`/`l` to `edge_jump` (storing the returned `EdgeWalk`), `[`/`]` to `wave_step`, and fold-hidden jump targets to `unfold_for`.
- 24-06 deletes the legacy half (`layout_graph`, `RoadmapGraphWidget`, `layout_for_state`, `split_areas`, `milestone_tags`, …) once `detail.rs` stops calling it.

## Self-Check: PASSED

- FOUND: src/ui/roadmap_graph.rs
- FOUND commits: bc3077a, 1949dcf, 5500d56, bd2eb70, 28f14d5
