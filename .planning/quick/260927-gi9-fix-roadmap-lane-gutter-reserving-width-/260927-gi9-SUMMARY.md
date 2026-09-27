---
quick_id: 260927-gi9
phase: quick-260927-gi9
plan: 01
subsystem: ui/roadmap
status: complete
tags: [roadmap, lanes, folding, gutter, tdd]
requires: []
provides:
  - "layout_list lays out lanes over the visible slot sequence only"
affects: [src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs]
tech-stack:
  added: []
  patterns: ["fold filter before lane assignment; one fold decision site"]
key-files:
  created: []
  modified:
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_view.rs
decisions:
  - "I-1: edges with a folded endpoint draw nothing (no stub), both directions"
  - "I-4: fold stability traded for compact gutter; unfolded layout byte-identical"
metrics:
  duration: ~20m
  completed: 2026-09-27
requirements: [QUICK-260927-gi9]
actuals:
  tokens: 4518   # chars/4 over the 18074-char diff 9106409..2860a12
  tasks: 2
  commits: 3
plan_head_before: 9106409c35cddef844f4e6c72bc779500aee573f
plan_head_after: 2860a120acb6f28781305f6f398706f95b2ffb3b
---

# Quick 260927-gi9: Roadmap lane gutter computed over visible rows only Summary

When milestones are folded, the Roadmap list now lays out lanes over the visible rows only. With v1 … v2 folded, the v3 band row draws no lanes, phase 14 sits on lane 0 with no merge connector, and the lane column shrinks to the 8-cell minimum.

## Root cause

`layout_list` called `assign_lanes` over the full unfolded slot sequence and dropped hidden rows only afterwards. So v2's hidden phases 8..13 still held their lanes into 14 on the visible v3 band row and on 14's merge connector. That widened the gutter from 8 to 11 cells at 120 columns (9 at 80, capped).

## Fix

- `src/ui/roadmap_graph.rs:789-808` (`layout_list`): the fold predicates (`summary_folded`, `band_hidden`, `phases_hidden`, `node_hidden`) now run before layout. A `visible` slot sequence (Summary always, Band unless `band_hidden`, Phase unless `node_hidden`) is what goes to `assign_lanes`. The body of `assign_lanes` is unchanged.
- The `rows` mapping is now a total `map` with an exhaustive match (it used to be a `filter_map` with hide guards). The visible filter is the only place folding is decided.
- `order`/`pos`, PhaseFacts, `start_now`, bands, `max_wave` and `notes` are still computed over the full `seq`.
- Doc comments updated on `Slot`, `LaneKind`, `assign_lanes`, `layout_list` and the inline fold block; each cites quick 260927-gi9.
- `src/ui/roadmap_view.rs`: only tests changed (I-6).

## Commits

| sha | subject |
|-----|---------|
| c8cfaa6 (`<red>`) | test(quick-260927-gi9): add failing folded-lane gutter tests |
| c540d5d (`<green>`) | fix(quick-260927-gi9): lay out Roadmap lanes over visible rows only |
| 2860a12 (`<t2>`) | test(quick-260927-gi9): hit-test invariant covers the compact lane gutter |

`git log --oneline c8cfaa6^..c540d5d` lists exactly the test commit and then the fix commit.

## RED run (c8cfaa6, `rtk proxy cargo test --lib --no-fail-fast -- roadmap_graph::tests roadmap_view::tests`)

58 tests: 55 passed, 3 failed. The failures were exactly the three expected tests:
- `ui::roadmap_graph::tests::folding_the_shipped_summary_drops_lanes_of_hidden_phases`. Its diff showed `│ │ │ │ │ │[v3]` followed by `├─┴─┴─┴─┴─┘`, as predicted.
- `ui::roadmap_graph::tests::folding_a_band_drops_the_lanes_only_its_phases_owned`. The pre-fix output still had the `├─┼─┐` fork and lane 2 on 14, 15 and M4.
- `ui::roadmap_view::tests::folded_milestones_do_not_widen_the_lane_column`. At 120x30, "14" sat at cell 14 instead of 11 (the lane column was 11 cells).

`unfolded_cross_fold_layout_is_pinned` (G2) PASSED on the pre-fix code, so the hand-derived G2 literal was correct as written.

After GREEN (c540d5d): 58/58 pass with no compiler warnings.

## Inferred decisions (for audit)

- **[I-1] Edges with a hidden endpoint are dropped entirely, with no stub, in both directions** (hidden → visible such as v2 → 14, and visible → hidden such as 13 → folded 16).
  - How: `assign_lanes`' existing guard only adds an edge when both endpoints have a position in the sequence it walks, so feeding it the visible sequence drops these edges with no algorithm change.
  - Why: this matches how external dependencies render (no lane). A stub would cost a column and need a new glyph or row kind. The folded band row and the detail pane's Needs/Unblocks lists already show the edge.
  - Consequence: a visible phase whose only parents are hidden is laid out as a root. 14 and 20 (in G1b) take lane 0.
- **[I-2]** No bridging edges are drawn, and the transitive reduction is not recomputed over the visible subgraph. A chain visible → hidden → visible draws no lane between the two visible phases.
- **[I-3]** Everything except rows and lanes stays computed over the full graph: PhaseFacts, `start_now`, bands, `max_wave`, `notes`, and `order`/`pos`. The test asserts that 14's needs and `start_now` are identical under folded, double-folded and open states.
- **[I-4] The old fold-stability invariant is given up.** Toggling a fold may now move lanes outside it (for example, M4 folded drops lane 2 from 14 and 15). The unfolded layout is byte-identical because, with nothing hidden, the visible sequence equals `seq`. This is pinned by G2, `lanes_mockup_a_with_bands`, the daily-vow and sentriq lane tests, and the unfolding test, all unchanged and passing.
- **[I-5]** `folding_a_band_hides_its_rows_but_not_the_lanes_elsewhere` was rewritten and renamed to `folding_a_band_drops_the_lanes_only_its_phases_owned` (golden G3; the M4 band lanes are now `"  │"`). It was not deleted.
- **[I-6]** There is no production change in `roadmap_view.rs` or `detail.rs`. Gutter width comes from the model's visible lane cells, and fold-mark rects come from the drawn spans. The tests prove this: the compact mark is at `list_body.x + 8`, and the unfolded mark is further right.
- **[I-7]** The CROSS_FOLD fixture is synthetic and mirrors the reported shape. Shipped v1 has no listed phases. Shipped v2 has 8..13, all feeding 14. Then v3 has 14..19 and M6 has 20.
- **E-1 (execution):** `phase_lanes` in the graph tests returns a `Vec<usize>` in row order, not a map. The "14 on lane 0" check therefore asserts that the first visible phase row's lane is 0, and that a `ListRow::Phase { node: idx("14"), lane: 0 }` exists.
- **E-2 (execution):** HEAD was on `master`, the main working tree, as instructed by the orchestrator (ISOLATION=none). `gsd_run query git.base-branch --is-protected master` returned `false` (the default branch is `dev`), so the pre-commit branch guard passed.
- No G1, G1b, G2 or G3 literal needed correcting. All four matched the code as hand-derived.

## Gates

- `rtk proxy cargo test --no-fail-fast`, redirected to a scratch file and read: across 56 test result lines, **2800 passed, 1 failed, 15 ignored**. The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local git-version witness. No `detail.rs` roadmap test needed updating.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0, no warnings.
- `git diff c8cfaa6^..2860a12 -- src/ui/roadmap_view.rs`: both hunks are inside `mod tests` (which starts at line 1346).

## Deviations from Plan

None beyond E-1 (test helper shape) and E-2 (branch-guard note). The plan was otherwise executed exactly as written.

## Threat surface

No new surface. T-gi9-02: `list_model_stores_only_escaped_text` and `roadmap_view_stores_and_draws_only_escaped_text` pass in the full suite.

## Self-Check: PASSED

- FOUND: src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs
- FOUND commits: c8cfaa6, c540d5d, 2860a12
