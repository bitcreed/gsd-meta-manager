---
quick_id: 261006-ujx
phase: quick-261006-ujx
plan: 01
subsystem: ui/roadmap
status: complete
tags: [roadmap, lanes, git-graph, palette, no-color]
requires: []
provides:
  - "pub(crate) const LANE_PALETTE: [Color; 5] (src/ui/mod.rs)"
  - "pub struct LaneLink { owner, target } + links on every ListRow variant"
  - "lane_style / per-char lane_spans / LaneCtx / lane_palette_enabled (roadmap_view)"
affects: [src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs, src/ui/mod.rs]
tech-stack:
  added: []
  patterns: ["per-char lane edge data (LaneLink) threaded from the assigner to the renderer"]
key-files:
  created: []
  modified:
    - src/ui/mod.rs
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_view.rs
    - src/ui/screens/detail.rs
decisions:
  - "I-1..I-11 adopted as planned (see below)"
  - "phase_spans takes (lanes, links) as one tuple argument to stay under clippy's 7-argument limit"
metrics:
  duration: "~45 min"
  completed: 2026-10-07
estimate:
  tokens: 150000
  tasks: 3
actuals:
  tokens: 15100
  tasks: 3
  commits: 3
plan_head_before: 0ca9663
plan_head_after: 8e29ddf
---

# Quick 261006-ujx: Roadmap tab rounded lane curves and ANSI lane colours

The Roadmap's lane column now looks like the git-graph style on the Git tab. Fork and merge corners are rounded (`├─╮`, `├─╯`, `╭─┤`). Each lane char takes its column's hue from the five-colour `LANE_PALETTE`, which the Git tab now shares. A lane out of a Done phase draws DarkGray. With a phase selected, its reduced need/unblock lanes draw BOLD (they keep their hue even out of a Done phase) and every other lane draws DIM. A non-empty `NO_COLOR` drops only the lane hue. Blocked glyphs are Red, and the `┆` overflow column is DarkGray.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 (tracer) | 0c2a968 | `LaneLink` per-char edge data from `assign_lanes` to `ListRow.links`; `lane_style` plus a per-char `lane_spans`; hue by column; done-owner DarkGray; Done row no longer dims its lane cell |
| 2 | 4093790 | `LaneCtx`, selection-chain BOLD/DIM, `lane_palette_enabled()` (NO_COLOR), Blocked Red, `┆` DarkGray |
| 3 | 8e29ddf | Rounded corner constants and fixtures; Git tab `graph_spans` on `crate::ui::LANE_PALETTE`; stranded doc moved back onto `first_string_entry` |

## Gate results (S-09)

- `cargo build`: green.
- `cargo clippy --all-targets`: no new warnings. The set of `^warning:` lines in clippy.after is identical to clippy.before (the same 5 pre-existing: lib 4, lib-test 5 with 4 duplicates). 0 errors. One `too_many_arguments` (8/7) on `phase_spans`, introduced mid-Task-1, was fixed before that task was committed.
- `cargo test --no-fail-fast`: 58 suites, **2946 passed, 1 failed, 15 ignored**. The one failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (git-version witness, environmental on this machine).
- Per-task verify: `cargo test --lib ui::roadmap`: 78 passed. `graph_spans`: 2 passed. The palette grep check passes.
- `rustfmt --check --edition 2021` `Diff in` hunks (baseline → after): roadmap_graph.rs 4 → 4, roadmap_view.rs 7 → 7, screens/detail.rs 517 → 517. `src/ui/mod.rs` was hand-formatted and not checked, per plan. `cargo fmt` was never run.

## New tests

- roadmap_graph: `lane_links_match_lane_chars_one_to_one`, `lane_links_name_the_edge_each_cell_draws`, `a_leftward_fork_links_its_run_to_the_forked_lane`, `lane_corners_are_rounded`, `no_laid_lane_uses_a_square_corner`
- roadmap_view: `lane_cells_take_the_palette_hue_of_their_column`, `a_lane_out_of_a_done_phase_draws_dark_gray`, `a_done_row_no_longer_dims_its_whole_lane_cell`, `lane_style_bolds_the_chain_and_keeps_its_hue_when_done`, `lane_style_dims_off_chain_lanes_only`, `lane_style_without_palette_keeps_weight_only`, `selecting_a_phase_bolds_its_edges_and_dims_the_rest`, `a_band_cursor_bolds_and_dims_nothing`, `no_color_drops_the_lane_hue_but_keeps_status_and_weight`, `the_blocked_glyph_is_red_and_not_bold`, `the_overflow_column_draws_dark_gray`
- detail.rs: `graph_spans_use_the_shared_lane_palette`, plus the updated `lane_colour_is_stable_per_column_and_cycles` (the 6th lane wraps)

Updated fixtures (square corners changed to rounded only): `lanes_daily_vow_without_bands`, `MOCKUP_A_LANES`, `folding_a_band_drops_the_lanes_only_its_phases_owned`, `unfolded_cross_fold_layout_is_pinned`, `list_fan_in_merges_once`, the roadmap_view `LANE_GLYPHS`, and the `lanes_mockup_b_with_shipped_summary` literal (it gained `links: Vec::new()`). No pre-existing test pinned a lane cell's old plain or row-dim style, so none needed a style update.

## Inferred decisions / deviations (for operator audit)

Inferred decisions copied from the plan [INFERRED: the human was unavailable]. All were implemented as written:

- **I-1 (glyph scope):** the spec's "Fork reads `○─╮`" is read as illustrating the rounded fork end, not as a layout change. Fork and merge connectors stay on their own rows, so under a node the fork row reads `├─╮`. This is a pure glyph swap.
- **I-2 (lane data shape):** `links: Vec<Option<LaneLink>>` has one entry PER CHAR of `lanes`, with `LaneLink { owner (upstream), target (downstream) }`. It replaces the spec's `owner: Vec<Option<usize>>` per lane. Hue still follows `char_index / 2`. The links decide only done-dimming and selection weight.
- **I-3 (Blocked):** Blocked changes from `Style::default()` (D-A04) to Red, not bold. The detail pane's status glyph turns red for blocked phases too.
- **I-4 (Git tab hue shift):** the Git tab moves from its six-colour palette (with Red) to the shared five-colour one, so lanes 4 and up change hue.
- **I-5 (NO_COLOR):** `lane_palette_enabled()` is false when `NO_COLOR` is present and non-empty. It is read once per Roadmap list render and drops only the lane hue. A done-owner lane then uses DIM instead of DarkGray. Tests drive `palette: false` through `LaneCtx` and never set the env var.
- **I-6 (done-dim rule):** a lane is done-dim when its `owner` (upstream) phase is `Done`, read from `RoadmapModel::phases` at render time, so `set_phase_status` is honoured.
- **I-7 (selection scope):** bold/dim applies only when the cursor is on a phase. A band or shipped-summary cursor weights nothing.
- **I-8 (on-chain):** `link.owner == s && unblocks(s).contains(target)` or `link.target == s && needs(s).contains(owner)`. Implied dependencies have no lane to bold.
- **I-9 (palette home):** `LANE_PALETTE` lives in `src/ui/mod.rs`.
- **I-10 (horizontal-run ownership):** on a connector row, a run char with no lane of its own carries the link of the nearest `others` lane beyond it, on the far side from the node's lane. A crossed `┼` lane keeps its own link.
- **I-11 (incidental fix):** the `first_string_entry` doc block that was stranded above the Git palette now sits on `first_string_entry`, placed before its `#[cfg(test)]` attribute.

Executor deviations:

1. **[Rule 3 - Blocking] `phase_spans` argument count.** Adding `links` took `phase_spans` to 8/7 arguments, and Task 2's `LaneCtx` alone would not have brought it back under the limit. The row's `(lanes, links)` is now passed as one destructured tuple argument. No behaviour change. Commit 0c2a968.
2. **Test fixture choice (minor).** `lane_links_match_lane_chars_one_to_one` also covers TTBOOK, Mockup A with M4 folded, and CROSS_FOLD with v3 folded, beyond the plan's list. It also asserts that a blank char never carries a link.
3. **Ledger.** The commit ledger was written to `.git/gsd-plan-head-before-quick-261006-ujx-01` (base 0ca9663). `commits: 3` was measured with `git rev-list --count 0ca9663..HEAD`.

Optional manual look (`cargo run`, open the Roadmap tab) was not performed. It is non-blocking per the plan, and the buffer-cell tests are the evidence.

## Threat surface

Nothing new beyond the plan's threat model. T-ujx-02 is mitigated: `links` is read only via `.get(i)` in `lane_spans`, and the length parity is pinned by test. T-ujx-03: the `NO_COLOR` presence check only, with no echo.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/ui/mod.rs, src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs
- FOUND commits: 0c2a968, 4093790, 8e29ddf (all ancestors of HEAD)
