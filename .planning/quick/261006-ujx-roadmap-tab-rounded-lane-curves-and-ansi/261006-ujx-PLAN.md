---
quick_id: 261006-ujx
mode: quick
phase: quick-261006-ujx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/mod.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/detail.rs
autonomous: true
requirements: [QUICK-261006-ujx]

estimate:
  tokens: 150000
  raw_tokens: 150000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Roadmap fork and merge connector rows draw rounded corners (a fork reads `├─╮`, a merge `├─╯`, a leftward fork `╭─┤`). No laid lane string contains a square corner. `├ ┤ ┬ ┴ ┼ │ ─` are unchanged, and connector rows stay separate rows [I-1]"
    - "Every lane/edge char on phase, connector, band and shipped-summary rows takes `LANE_PALETTE[(char_index / 2) % 5]`, so a column keeps its hue on every row and the 6th lane wraps to Magenta"
    - "A lane whose owning (upstream) phase is Done draws DarkGray [I-6]"
    - "A node glyph is styled by its status only, never by its lane: Done DarkGray, Active Yellow+BOLD, Ready Green, Blocked Red (not bold) [I-3], Unmerged Cyan. `↑` stays Cyan, `↓` Magenta, the selected row REVERSED, and the overflow `┆` is DarkGray"
    - "With a phase under the cursor, the lane cells of its needs/unblocks edges draw BOLD and keep their palette hue even when the owner is Done. Every other lane cell gets DIM. With a band or the shipped summary under the cursor, nothing is bolded or dimmed [I-7, I-8]"
    - "When `NO_COLOR` is set to a non-empty value, Roadmap lanes draw with no palette hue, and done lanes use DIM instead of DarkGray. Status glyph colours, BOLD, DIM and REVERSED are unchanged, and the Git tab is unaffected [I-5]"
    - "The Git tab and the Roadmap share one `pub(crate) const LANE_PALETTE: [Color; 5] = [Magenta, Yellow, Cyan, Green, Blue]`. The Git tab's private six-colour palette is gone [I-4, I-9]"
  artifacts:
    - path: src/ui/mod.rs
      provides: "pub(crate) const LANE_PALETTE: [ratatui::style::Color; 5]"
    - path: src/ui/roadmap_graph.rs
      provides: "rounded LANE_DOWN_RIGHT/LANE_DOWN_LEFT/LANE_UP_LEFT/LANE_UP_RIGHT; pub struct LaneLink { owner, target }; links: Vec<Option<LaneLink>> on LaneRow and on every ListRow variant, one entry per char of `lanes`"
    - path: src/ui/roadmap_view.rs
      provides: "lane_style(lane, owner_done, on_chain, dimmed, palette); per-char lane_spans; LaneCtx { selected, palette }; lane_palette_enabled() (NO_COLOR); glyph_style Blocked = Red"
    - path: src/ui/screens/detail.rs
      provides: "graph_spans coloured from crate::ui::LANE_PALETTE"
  key_links:
    - from: "roadmap_graph::assign_lanes (active: Vec<Option<LaneLink>>)"
      to: "ListRow::*.links"
      via: "pass_through / connector_row return (String, Vec<Option<LaneLink>>), threaded through layout_list's row mapping"
    - from: "ListRow::*.links + RoadmapModel::phases[owner].status"
      to: "roadmap_view::lane_style"
      via: "lane_spans per-char loop in row_line (all four row kinds) and phase_spans"
    - from: "src/ui/mod.rs LANE_PALETTE"
      to: "roadmap_view::lane_style AND detail.rs graph_spans"
      via: "crate::ui::LANE_PALETTE (single definition)"
---

<objective>
Restyle the Roadmap tab's lane column to match the git-graph look already on the Git tab (reference commit 0ca9663). Lane corners become rounded. Each lane gets a palette colour by column. A lane out of a done phase draws DarkGray. The selected phase's needs/unblocks edges draw BOLD while the rest draw DIM. Status stays on the node glyph, carried by both colour and shape. One shared palette serves the Git and Roadmap tabs.

Purpose: you can follow a dependency edge down the Roadmap by colour, and history (done) recedes while what is still open stands out.
Output: rounded corner constants, per-char lane edge data (`LaneLink`) from the assigner to `ListRow`, a per-char lane renderer with `lane_style`, a narrow `NO_COLOR` switch, the shared `LANE_PALETTE`, and tests.

Requester spec items, numbered here so tasks can cite them (the requester's spec is the authority; these are restatements):
- S-01 Glyphs: `LANE_DOWN_RIGHT` becomes `╭`, `LANE_DOWN_LEFT` `╮`, `LANE_UP_LEFT` `╯`, `LANE_UP_RIGHT` `╰`. Keep `├ ┤ ┬ ┴ ┼ │ ─`. Update the doc comments and the tests that assert the old corners. Node status glyphs are unchanged.
- S-02 Status colours: Done DarkGray, Active Yellow+BOLD, Ready Green, Blocked Red (not bold), Unmerged Cyan. `MARK_DEP` Cyan, `MARK_UNBLOCKS` Magenta and the selected row REVERSED are unchanged. `LANE_OVERFLOW` is DarkGray.
- S-03 Lane colour: lane/edge chars use the palette colour of lane index `char_index / 2`, on connector, band and shipped-summary rows too. A lane is DarkGray (or DIM) when its owning/upstream phase is Done.
- S-04 A node glyph is coloured by status, never by lane.
- S-05 Selection: the lane cells leading to the selected phase's needs/unblocks get BOLD and keep their hue even when done-dim. Every other lane gets DIM. This sits on top of the existing `↑`/`↓` markers.
- S-06 `NO_COLOR` skips the lane palette and keeps status colours and weights. Add it narrowly, for Roadmap lanes only.
- S-07 Shared palette: one `pub(crate) const LANE_PALETTE` of `[Magenta, Yellow, Cyan, Green, Blue]`, cycling, used by both the Git tab and the Roadmap. No Red.
- S-08 Implementation shape: rewrite `lane_spans()` as a per-char loop. The char at `glyph_at` takes `glyph_style(status)`. Every other non-space char takes a new `lane_style(...)` helper placed beside `glyph_style`. Adjacent chars of the same style merge into one span. Update callers: `phase_spans` stops dimming the whole lane cell, and the Connector, Band and ShippedSummary arms of `row_line` change too.
- S-09 Gate: `cargo build`; clippy with no NEW warnings; `cargo test --no-fail-fast`, where the lone `src/envelope/policy.rs` git-version failure is expected. Never run `cargo fmt`. Use `rustfmt --check` on our own files only, and `rtk proxy cargo ...` for raw output.

**Inferred decisions [INFERRED — the human was unavailable; the executor MUST copy this list into SUMMARY under "Inferred decisions / deviations" for operator audit]:**
- I-1 (glyph scope): read the spec's "Fork reads `○─╮`" as illustrating the rounded fork end, not as a layout change. The assigner keeps emitting fork and merge connectors as their own rows beneath and above the node, so under a node `○` the fork row reads `├─╮`. This is a pure glyph swap. Basis: the spec keeps `├ ┤ ┬ ┴ ┼`, which appear only on connector rows, and its IMPLEMENTATION section names no assigner layout change.
- I-2 (lane data shape): the spec suggests `owner: Vec<Option<usize>>` per lane on `LaneRow`. This plan instead uses `links: Vec<Option<LaneLink>>`, where `pub struct LaneLink { pub owner: usize, pub target: usize }`, with ONE entry PER CHAR of `lanes`. `owner` is the spec's "phase that opened the lane" (upstream). `target` is the phase the lane runs to (downstream). There are three reasons:
  - The assigner's `active` value is the downstream target, not the opener, so the opener has to be recorded alongside it anyway.
  - The selection chain (S-05) needs both ends: otherwise a need's lane into an unrelated child would bold.
  - Per-char means the `─` separator of a horizontal run belongs to the run's edge rather than to an unrelated lane it crosses, so a bold merge into the selection never shows a dim gap.

  Hue still follows `char_index / 2` exactly as S-03 says. The links decide only done-dimming and selection weight.
- I-3 (Blocked): Blocked changes from `Style::default()` (phase-24 D-A04, `24-CONTEXT.md:34`) to Red, not bold. The spec's palette note, "Red conflicts with Blocked", only makes sense if Blocked is red. Its "(all unchanged)" is read as covering `↑`, `↓` and REVERSED. The same `glyph_style` also colours the detail pane's status glyph, which turns red for blocked phases as a result.
- I-4 (Git tab hue shift): moving the Git tab from its six-colour palette (with Red) to the shared five-colour one changes the hue of Git lanes 4 and up. The spec says this is intended.
- I-5 (NO_COLOR): handled narrowly. `lane_palette_enabled()` returns false when `NO_COLOR` is present and non-empty (no-color.org semantics), read once per Roadmap list render. It drops only the lane palette hue. A done-owner lane then gets `Modifier::DIM` instead of DarkGray. Status glyph colours, the `↑`/`↓` colours, the `┆` DarkGray and every modifier stay. Tests never set the env var, because it is process-global and tests run in parallel. They drive the `palette: false` path through `LaneCtx` instead.
- I-6 (done-dim rule): a lane is done-dim when its `owner` (upstream) phase's status is `Done`, even when its target is not done. This follows the spec's literal "owning/upstream phase is Done". Status is read from `RoadmapModel::phases` at render time, so the unmerged post-pass (`set_phase_status`) is honoured.
- I-7 (selection scope): S-05 applies only when the cursor is on a phase. A band or shipped-summary cursor has no selected phase, so no lane is bolded or dimmed.
- I-8 (on-chain definition): a lane char is on the chain of selected phase `s` when `link.owner == s && phases[s].unblocks.contains(&link.target)`, or `link.target == s && phases[s].needs.contains(&link.owner)`. Lanes draw only reduced edges, so implied dependencies have no lane to bold. Their `·` marker is unchanged.
- I-9 (palette home): `LANE_PALETTE` lives in `src/ui/mod.rs`, the common parent of `roadmap_view` and `screens::detail`.
- I-10 (horizontal-run ownership): on a connector row, a span char with no lane of its own (an inactive column's `─` cell, or any `─` separator inside the run) carries the link of the nearest `others` lane beyond it, on the far side from the node's `lane`. An active lane crossed by the run (`┼` cell) keeps its own link.
- I-11 (incidental fix): the doc block at `src/ui/screens/detail.rs:12051-12062`, which documents `first_string_entry`, currently sits glued above the Git palette's doc line. When that palette is deleted, the block moves onto `first_string_entry` (detail.rs:12109-12110), so it does not silently become `graph_spans`'s doc.

Out of scope: `src/ui/roadmap_widget.rs:91-99` (the legacy phase-box borders, not lanes). Its light box corners stay.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/ui/roadmap_graph.rs
@src/ui/roadmap_view.rs

Code facts the planner verified at HEAD 0ca9663 (cite these; do not re-derive):
- `src/ui/roadmap_graph.rs`:
  - Lane constants are at 242-264; the four corners at 254-262 carry doc comments that name the old shapes.
  - `junction(left, right, up, down)` (545-560) maps `(false,true,false,true)` to `LANE_DOWN_RIGHT`, `(true,false,false,true)` to `LANE_DOWN_LEFT`, `(false,true,true,false)` to `LANE_UP_RIGHT` and `(true,false,true,false)` to `LANE_UP_LEFT`.
  - `LaneRow { kind: LaneKind, lanes: String }` is private (539-543).
  - `free_lane` (564-568) and `claim` (570-575) work on `active: &[Option<usize>]`.
  - `join_cells(cells, span)` (579-587) emits, per lane column `l`, the cell at char `2l`, then a separator at `2l+1` (`─` when `lo <= l < hi`, else a space), and finally `trim_end`s.
  - `pass_through(active, node)` is at 591-601. `connector_row(active, lane, others, fork)` is at 605-630, with `lo`/`hi` the min/max over `lane` and `others`.
  - `assign_lanes` (643-729):
    - `active: Vec<Option<usize>>` (663) holds each lane's DOWNSTREAM target.
    - The incoming filter is `active[l] == Some(u)` (682-684).
    - The merge connector (686-695) is built BEFORE the closing lanes are cleared, so `active` still holds them.
    - The node row is built after `claim(lane, None)` (700-704), so the node's own column is `None`.
    - In the fork (705-722), `claim(lane, Some(first))` and each forked `claim(l, Some(c))` happen BEFORE `connector_row(..., true)`.
    - Forked lanes come from `free_lane`, so they can sit LEFT of `lane`, which gives a leftward fork. Merges always close into the lowest incoming lane, so merge `others > lane`.
  - `ListRow` (360-392) is a `pub` enum deriving `Debug, Clone, PartialEq, Eq`. Its variants are `ShippedSummary { text, milestones, phases, folded, lanes }`, `Band { band, key, label, short, done, total, folded, lanes }`, `Connector { lanes }` and `Phase { node, lane, lanes }`.
  - `layout_list`'s row mapping is at 900-927.
  - `RoadmapModel::set_phase_status` (1031-1056) rewrites one char of a Phase row's `lanes` in place; the unmerged glyph is the single char `\u{25D0}`.
  - `lane_text` is at 1271-1297.
  - Struct patterns that must gain `..` once `links` exists: 1039-1043, 1277, 1290, 1944, 2464.
  - Test fixtures asserting square corners: 1471, 1584-1591 (`MOCKUP_A_LANES`), 1767-1772, 1898, 2130.
  - Tests: helpers `build(spec, bands, toggles)` at 1316 and `plain(spec)` at 1350; the module opens at 1299.
- `src/ui/roadmap_view.rs`:
  - Imports are at 23-32, and `LANE_OVERFLOW` (`┆`) is at 43.
  - `dim()` returns `fg(DarkGray)` and `bold()` adds BOLD (257-263).
  - `glyph_style` (265-278) has `Blocked => Style::default()` at 273. It is also used by the detail pane at 676, 966, 1028 and 1192.
  - `lanes_of` (290-297) uses the exhaustive pattern `Connector { lanes }`.
  - `lane_spans(lanes, glyph_at, glyph, base, width)` is at 299-324. `cap_lanes(lanes, glyph_at, cap)` (330-351) keeps chars `[..2*cap]` 1:1, then either the node glyph or one `┆` at index `2*cap`.
  - `render_list` (579-): `selected_phase` is at 600-603, and the row loop at 644-646 calls `self.row_line(row, selected, selected_phase, &cols, w)`, the ONLY caller.
  - `row_line` is at 703-791:
    - Phase arm at 714-716.
    - Band arm at 718-739, with `lane_spans(..., plain, plain, cols.lane)` at 727.
    - ShippedSummary arm at 741-763: an exhaustive destructure, with lanes pushed unpadded as one span at 755. Its `lanes` is always empty in practice, because the summary is always the first row.
    - Connector arm at 764-767.
    - The selected-row REVERSED map is at 769-779.
  - `mark_for` is at 793-810. `phase_spans(node, lane, lanes, selected, selected_phase, cols)` is at 812-848: `base` is dim for a Done row (825-829) and is applied to the WHOLE lane cell at 831.
  - Tests (module at 1361):
    - Helpers: `build(spec, bands, &Extras)` at 1392, `render(model, cursor, w, h, state) -> Buffer` at 1445, `rect_text` at 1459, `body` at 1688.
    - `LANE_GLYPHS` (1685) lists the square corners.
    - `eight_lanes()` (2096) is the overflow fixture, used by `lanes_past_the_cap_collapse_into_one_overflow_column` (2119).
    - A buffer-cell style assertion pattern is at 2289: `c.modifier.contains(Modifier::REVERSED)`.
- `src/ui/screens/detail.rs`:
  - `graph_spans(&entry.graph)` is called at 6648.
  - The Git palette is a private six-colour array (12063-12071, including `Color::Red`). The doc block for `first_string_entry` is stranded above it at 12051-12062.
  - `graph_spans` (12073-12089) styles char `i` with palette index `(i / 2) % len`.
  - Its test `lane_colour_is_stable_per_column_and_cycles` (12095-12106) asserts that lane 6 (`spans[12]`) wraps to Magenta; that holds only for a six-colour palette.
  - `first_string_entry` is at 12109-12110.
  - The roadmap tests here use `ListRow::Phase { node, lanes, .. }` (24145), which already has `..`.
- `src/ui/mod.rs` (1-8) declares the ui submodules. It has no palette and no `Color` import.
- `NO_COLOR` is read nowhere in `src/`. No source-census test forbids env reads in `src/ui`. `Option::is_none_or` is already used (roadmap_graph.rs:566; MSRV 1.88).
- Gate baselines measured at planning time:
  - `rustfmt --check --edition 2021 <file>` already reports pre-existing diff hunks: roadmap_graph.rs 4, roadmap_view.rs 7, screens/detail.rs 517.
  - `src/ui/mod.rs` must NOT be rustfmt-checked, because it recurses into every ui submodule (673 hunks).
  - The rtk hook filters piped output, so always redirect `rtk proxy ...` output to a file and `rtk proxy grep` that file.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Tracer — lane edge data from the assigner to the screen: palette hue per column and done-dim, on all four row kinds</name>
  <files>src/ui/mod.rs, src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs</files>
  <behavior>
    Graph tests (module helpers `build`, `plain` and `idx(model, id)` at 1316-1360; all `LaneLink` indices are phase indices, resolved through `idx`):
    - `lane_links_match_lane_chars_one_to_one`: for every row of `plain(DAILY_VOW)`, `plain(SENTRIQ)`, `build(BOOKLY, BOOKLY_BANDS, ..)` (Mockup A) and `build(CROSS_FOLD, CROSS_FOLD_BANDS, ..)`, folded and unfolded, `links.len() == lanes.chars().count()`
    - `lane_links_name_the_edge_each_cell_draws`, covering DAILY_VOW and the `1,2,3 -> 4` merge fixture (graph test near 2120):
      - DAILY_VOW (`21` forks to `22` and `23`; the fork row reads `├─┐` today). Char 0 carries `LaneLink { owner: idx("21"), target: idx("22") }`. Chars 1 and 2 carry owner `idx("21")` and target `idx("23")` [I-10]. On the `22` row, char 2 (`│`) carries that same 21→23 link, and the node's own char 0 is `None`.
      - The merge fixture (`├─┴─┘` today). Every non-space char carries `target == idx("4")`. Char 1, the separator, carries the same link as char 2 (owner `idx("2")`). Char 3 carries the same link as char 4 (owner `idx("3")`) [I-10].
    - `a_leftward_fork_links_its_run_to_the_forked_lane`: use the fixture `[("1","",&[],F), ("2","",&[],F), ("3","",&["2"],F), ("4","",&["2"],F)]`. The planner traced the expected layout by hand; confirm it with `lane_text` before asserting. `1` takes lane 0 and frees it. `2` is a root that avoids the previous row's lane, so it takes lane 1. `3` continues lane 1, and `4` takes free lane 0, so the fork row reads `┌─┤` today. Its char 0 and char 1 both carry `LaneLink { owner: idx("2"), target: idx("4") }`, and char 2 carries the 2→3 link.

    View tests. Use a CHAIN fixture shared with Task 2, built with the module's `build(spec, bands, &Extras)`: all six phases in band 0, and bands `[("M1 Chain", false, 6)]`. The phases are `("1","Alpha",&[],D)`, `("2","Bravo",&[],D)`, `("3","Charlie",&["1"],C)`, `("4","Delta",&["3"],F)`, `("5","Echo",&["3"],F)` and `("6","Foxtrot",&["2"],F)`. Expected lane_text, hand-traced (confirm before asserting): `[M1]`, `o 1`, `│ o 2`, `o │ 3`, then the fork row `├─┼─┐`, then `o │ │ 4`, `  │ o 5`, `  o 6`. So the 1→3 lane passes row 2 on column 0, and the unrelated 2→6 lane runs down column 1 through rows 3-5, crossed by the fork. All three view tests render at 120x30 with the cursor on `CursorTarget::Band(model.bands[0].key.clone())`, so Task 2's selection weighting cannot change what they expect. Assert on buffer cells: lane char `i` of the model row drawn at body line `k` sits at `(inner.x + i, inner.y + 2 + k)`, with `inner` = `pane_block().inner(list_rect)`. Confirm with `rect_text`.
    - `lane_cells_take_the_palette_hue_of_their_column`: every non-space lane char whose link owner is NOT Done has `fg == LANE_PALETTE[(i / 2) % 5]`. These are the fork row's chars 0, 1, 3 and 4 and row 4's char 4. Column 2's two cells (fork-row char 4 and row-4 char 4) both read `LANE_PALETTE[2]`. A unit check also asserts `lane_style(5, false, false, false, true).fg == Some(LANE_PALETTE[0])`, since the 6th lane wraps.
    - `a_lane_out_of_a_done_phase_draws_dark_gray`: row 2's char 0 (owner `1`, Done) and column-1 chars on rows 3-5 plus the fork row's `┼` (owner `2`, Done) have `fg == Color::DarkGray`. The `3` row's node glyph is Yellow+BOLD, which is not a lane hue (S-04).
    - `a_done_row_no_longer_dims_its_whole_lane_cell`: uses the fixture `[("1","A",&[],C), ("2","B",&[],D), ("3","C",&["1"],F)]` in one band; `2` takes lane 1, so its row reads `│ o`. On the Done `2` row, char 0 (`│`, owned by the Active `1`) has `fg == LANE_PALETTE[0]`, not DarkGray (S-08). The row's name text cells are still DarkGray.
  </behavior>
  <action>
    Wire one path end to end, from data to buffer cell: the edge each lane char belongs to, and the hue/done-dim decision made from it. Task 2 adds selection weight, NO_COLOR, Blocked red and overflow styling on top of the same `lane_style`. Task 3 does the glyph swap and the Git-tab adoption.

    Before editing anything, capture the gate baselines in a fresh `mktemp -d` directory (call it $B) and keep it for Task 3:
    - `rtk proxy cargo clippy --all-targets > $B/clippy.before 2>&1`
    - For each of roadmap_graph.rs, roadmap_view.rs and screens/detail.rs, run `rtk proxy rustfmt --check --edition 2021 <file> > $B/<name>.fmt.before 2>&1`. Expect 4, 7 and 517 `Diff in` hunks respectively.

    (a) `src/ui/mod.rs` (per S-07, I-9): add `use ratatui::style::Color;` and a documented `pub(crate) const LANE_PALETTE: [Color; 5] = [Color::Magenta, Color::Yellow, Color::Cyan, Color::Green, Color::Blue];`. Its doc says: the lane hue, by column, for every git-log style lane column (Git tab and Roadmap); no Red, because Red marks Blocked and Stalled elsewhere; the hue for lane index `l` is `LANE_PALETTE[l % LANE_PALETTE.len()]`. Hand-format it, and do not run rustfmt on this file.

    (b) `src/ui/roadmap_graph.rs` (per I-2, I-10):
    - Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct LaneLink { pub owner: usize, pub target: usize }`. Its doc: the reduced dependency edge a lane char draws; `owner` is the phase whose row opened the lane (upstream, the dependency), `target` the phase it runs down to (the dependent); both are indices into `RoadmapModel::phases`.
    - Change `active` in `assign_lanes` to `Vec<Option<LaneLink>>`. `free_lane` and `claim` take that type. The incoming filter becomes `active[l].is_some_and(|k| k.target == u)`. After node `u`, `claim` stores `Some(LaneLink { owner: u, target: first })` for the continued lane and `Some(LaneLink { owner: u, target: c })` for each forked lane. Clearing stores `None` as today.
    - `pass_through` and `connector_row` return `(String, Vec<Option<LaneLink>>)`. The links have ONE ENTRY PER CHAR of the returned string, truncated to the same length after `trim_end`. Build them beside the cells in `join_cells`, which takes per-cell links plus the separator-link rule.
    - On a pass-through row, the cell char `2l` carries `active[l]`, except the node's own column, which carries `None`. Separators (spaces) carry `None`.
    - On a connector row, a cell char carries `active[l]` when that is `Some`. This covers the node's lane, every `others` lane and any crossed `┼` lane.
    - An inactive cell strictly inside `(lo, hi)`, and every `─` separator inside `[lo, hi)`, carry the link of the nearest `others` lane BEYOND it, on the far side from `lane`. For positions right of `lane`, that is the smallest `o` in `others` that is greater than the position. For positions left of `lane`, it is the largest `o` in `others` that is less than or equal to the separator's column, or less than the cell's column.
    - Everything else carries `None`.
    - Add `links: Vec<Option<LaneLink>>` to `LaneRow` and to EVERY `ListRow` variant. Document it on `ListRow`: one entry per char of `lanes`; `None` on blanks and on a phase row's own node char; hue is never derived from it (hue is `char_index / 2`).
    - Thread the links through `layout_list`'s row mapping (900-927).
    - Add `..` to the struct patterns at 1039-1043 (`set_phase_status` leaves `links` untouched, since a status change moves no edge), 1277, 1290, 1944 and 2464.
    - Write the three graph tests from <behavior> first and see them fail. The existing lane_text fixtures must stay byte-identical in this task.

    (c) `src/ui/roadmap_view.rs` (per S-03, S-04, S-08, I-6):
    - Add `lane_style(lane: usize, owner_done: bool, on_chain: bool, dimmed: bool, palette: bool) -> Style` directly below `glyph_style`, with a doc comment. Its full contract, which Task 2 relies on:
      - hue = `LANE_PALETTE[lane % len]` when `palette`, else no fg.
      - If `on_chain`: fg = hue (kept even when `owner_done`), plus BOLD.
      - Else if `owner_done`: fg DarkGray when `palette`, else `Modifier::DIM`.
      - Else: fg = hue.
      - Then, if `dimmed && !on_chain`, add `Modifier::DIM`.
    - Rewrite `lane_spans` as a per-char loop (S-08). Its signature takes the capped `lanes`, the row's `links`, `glyph_at`, the glyph `Style`, a per-char lane-style decision (a closure `&dyn Fn(usize, Option<LaneLink>) -> Style` receiving the char index and `links.get(i).copied().flatten()`), and `width`.
      - The char at `glyph_at` takes the glyph style.
      - A space takes `Style::default()`.
      - Any other char takes the closure's style.
      - Adjacent chars with equal style merge into one span.
      - Push the trailing fill span only when the fill is non-zero.
      - Never index `links` directly: a shorter or empty `links` must degrade to `None`, never panic (T-ujx-02).
    - Add a `RoadmapView` method that is the closure body. It returns `lane_style(i / 2, owner_done, on_chain, dimmed, palette)`, where:
      - `owner_done` is `link` is `Some` and `self.phase(link.owner)` has status `Done`.
      - `on_chain` and `dimmed` are false in this task; Task 2 fills them from `LaneCtx`.
      - `palette` is true in this task.
    - Update every caller:
      - `phase_spans`: drop `base` from the lane cell, keep `base` on the id/name/plans/wave text, and pass the row's links.
      - The Band arm.
      - The ShippedSummary arm: route its lanes through `lane_spans` with `width` equal to its char count, so no fill is added and the fold-glyph column (`glyph_col` / `spans_cells`) is unchanged.
      - The Connector arm.
      - `row_line`'s Phase arm and `lanes_of`, which gain `links` / `..`.
    - Import `LaneLink` and `crate::ui::LANE_PALETTE`.
    - Write the three view tests from <behavior> first. Read buffer cells via `buf.cell((x, y))` `.fg` / `.modifier`, as at 2289. Locate the lane column as the first `cols.lane` cells inside the list border and padding (the `body()` helper's offset of 2).
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast ui::roadmap</automated>
  </verify>
  <done>All `ui::roadmap*` tests pass, including the six new ones. The pre-existing lane_text fixtures are byte-identical. A rendered Roadmap colours every lane char by its column from `LANE_PALETTE`, draws a done-owner lane DarkGray, and keeps node glyphs in their status style. Committed as the tracer.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Selection chain weight, NO_COLOR, Blocked red and overflow DarkGray</name>
  <files>src/ui/roadmap_view.rs</files>
  <behavior>
    - `lane_style_bolds_the_chain_and_keeps_its_hue_when_done`: `lane_style(3, true, true, false, true)` has `fg == LANE_PALETTE[3]` and contains BOLD. `lane_style(3, true, false, false, true)` has `fg == DarkGray` and no BOLD
    - `lane_style_dims_off_chain_lanes_only`: with `dimmed = true, on_chain = false`, the style contains DIM. With `on_chain = true` (even if `dimmed` is passed true), it has no DIM
    - `lane_style_without_palette_keeps_weight_only`: with `palette = false`, fg is `None` in every combination, `owner_done` yields DIM, and `on_chain` still yields BOLD
    - `selecting_a_phase_bolds_its_edges_and_dims_the_rest` (render, Task 1's CHAIN fixture, cursor on `phase("3")`):
      - Row 2's char 0 (the 1→3 lane) is BOLD with `fg == LANE_PALETTE[0]`, so the hue is kept although owner `1` is Done.
      - The fork row's chars 0, 1, 3 and 4 (the 3→4 and 3→5 edges) are BOLD. Its char 2 (`┼`, the crossed 2→6 lane) has DIM and no BOLD.
      - Column-1 chars on rows 4 and 5 (2→6) have DIM.
      - Row 4's char 4 (3→5) is BOLD.
      - The `↑` marker on row 1 is Cyan, the `↓` markers on rows 4 and 5 are Magenta, and row 3's cells are REVERSED.
    - `a_band_cursor_bolds_and_dims_nothing` (render, CHAIN, band cursor): no lane cell has BOLD or DIM. Done-owner lanes are DarkGray and the rest take the palette hue
    - `no_color_drops_the_lane_hue_but_keeps_status_and_weight`: call `row_line` directly on CHAIN's fork row and its `3` row with `LaneCtx { selected: Some(idx of "3"), palette: false }`, using `RoadmapView { model, cursor }.columns(w, LANE_CAP_WIDE)`. No lane char's fg is in `LANE_PALETTE`. The node glyph of `3` keeps Yellow+BOLD, and chain chars are still BOLD
    - `the_blocked_glyph_is_red_and_not_bold`: `glyph_style(PhaseStatus::Blocked).fg == Some(Color::Red)` and it has no BOLD. Done is DarkGray, Active Yellow+BOLD, Ready Green, and Unmerged Cyan (the existing test at 2478 still passes)
    - `the_overflow_column_draws_dark_gray`: render `eight_lanes()` at 80 columns. Every `┆` cell has `fg == DarkGray`
  </behavior>
  <action>
    Implement S-02, S-05 and S-06 on top of Task 1's `lane_style`; its contract is already final.

    - Add `#[derive(Clone, Copy)] struct LaneCtx { selected: Option<usize>, palette: bool }`, documented. It replaces the `selected_phase: Option<usize>` parameter of `row_line` and `phase_spans`. This keeps both under clippy's argument limit. `mark_for` keeps taking `ctx.selected`.
    - In `render_list`, build it once from the existing `selected_phase` (600-603) and `lane_palette_enabled()`.
    - Add `fn lane_palette_enabled() -> bool`, true unless `std::env::var_os("NO_COLOR")` is present and non-empty, per I-5 and no-color.org. Its doc comment says it is deliberately scoped to the Roadmap lane palette only, and that status colours and weights are unaffected. Do not touch the Git tab or any other screen.
    - In the per-char style method from Task 1:
      - Compute `on_chain` per I-8 against `self.phase(selected)`'s `needs` and `unblocks`. It is false when `link` or `selected` is `None`.
      - `dimmed` is `ctx.selected.is_some() && !on_chain` (I-7).
      - Pass `ctx.palette`.
    - Style the `LANE_OVERFLOW` char (`┆`, matched by char against the constant) with `dim()` (DarkGray) in every mode, unless it sits at `glyph_at`, in which case the node glyph wins.
    - Change `glyph_style`'s `Blocked` arm to `Style::default().fg(Color::Red)`, with no BOLD (S-02, I-3). Update its doc comment to cite this quick task and note the supersession of D-A04's "default".
    - Write the eight tests from <behavior> first. For render fixtures, use the module's `build(spec, bands, &Extras)` and `render(...)` helpers. Pick ids such that phase indices equal list order, so the assertions can name `LaneLink { owner, target }` by index.
    - If any pre-existing test pins a lane cell's old style (plain or row-dim), update it to the spec'd style and list it in SUMMARY.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast ui::roadmap_view</automated>
  </verify>
  <done>Selecting a phase bolds exactly its reduced need and unblock lanes, keeping their hue even out of a done phase, and dims every other lane. A band cursor changes no weight. The `palette: false` path draws lanes without hue while status colours and BOLD stay. Blocked is red and not bold. `┆` is DarkGray. All `ui::roadmap_view` tests pass.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Rounded corners, Git tab on the shared palette, and the full gate</name>
  <files>src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs</files>
  <behavior>
    - roadmap_graph `lane_corners_are_rounded`: `LANE_DOWN_RIGHT == "\u{256D}"`, `LANE_DOWN_LEFT == "\u{256E}"`, `LANE_UP_LEFT == "\u{256F}"`, `LANE_UP_RIGHT == "\u{2570}"`. `junction(false,true,false,true) == LANE_DOWN_RIGHT`, and likewise for the other three corners. `LANE_TEE_RIGHT`, `LANE_TEE_LEFT`, `LANE_TEE_DOWN`, `LANE_TEE_UP`, `LANE_CROSS`, `LANE_VERTICAL` and `LANE_HORIZONTAL` keep their code points
    - roadmap_graph `no_laid_lane_uses_a_square_corner`: across every fixture model in the module (DAILY_VOW, SENTRIQ, MOCKUP A with its bands, CROSS_FOLD folded and unfolded, the merge fixture), no row's `lanes` contains any of `\u{250C}`, `\u{2510}`, `\u{2514}` or `\u{2518}` (written as escapes in the test)
    - updated fixtures: `lanes_daily_vow_without_bands` reads `├─╮`. `MOCKUP_A_LANES` reads `├─╮`, `├─╯`, `├─╮` and `├─┼─╮`. 1767-1772 match it. 1898 reads `├─┴─┴─┴─┴─╯` and 2130 reads `├─┴─╯`
    - roadmap_view `LANE_GLYPHS` (1685) lists `╭╮╯╰` in place of the square corners, and the overflow test still passes
    - detail.rs `lane_colour_is_stable_per_column_and_cycles`: `spans[0]`, `spans[2]` and `spans[4]` are Magenta, Yellow and Cyan, and the 6th lane (`spans[10]` of `"│ ".repeat(6)`) wraps to Magenta. A new assertion checks that for every char index `i`, `graph_spans` fg equals `LANE_PALETTE[(i / 2) % LANE_PALETTE.len()]`, and that `LANE_PALETTE` does not contain `Color::Red`
  </behavior>
  <action>
    Implement S-01 and S-07 (I-1, I-4, I-11), then run the S-09 gate.

    (a) `src/ui/roadmap_graph.rs`:
    - Change the four corner constants (254-262) to `\u{256D}` (`LANE_DOWN_RIGHT`), `\u{256E}` (`LANE_DOWN_LEFT`), `\u{256F}` (`LANE_UP_LEFT`) and `\u{2570}` (`LANE_UP_RIGHT`). Rewrite their doc comments to name the rounded shapes in backticks: `╮` is the far right end of a fork, `╭` the far left end of a fork, `╯` the far right end of a merge, and `╰` a merge ending to the right (not produced by the assigner; kept for totality). Mention the git-graph style shared with the Git tab (quick 261006-ujx). `junction` needs no change.
    - Update every square-corner fixture listed in <behavior> to the rounded char. Do this only where the old corner occurs; do not touch `├ ┤ ┬ ┴ ┼`.
    - Add the two tests.

    (b) `src/ui/roadmap_view.rs`: update the `LANE_GLYPHS` test constant (1685).

    (c) `src/ui/screens/detail.rs`:
    - Delete the Git tab's private `GRAPH_PALETTE` array and its one-line doc (12063-12071).
    - Move the stranded `first_string_entry` doc block (12051-12062) to sit on `first_string_entry` (12109-12110), per I-11.
    - Make `graph_spans` index `crate::ui::LANE_PALETTE` (import it or path-qualify it). Update its doc comment to say the hue comes from the palette shared with the Roadmap lanes.
    - Update the graph_spans test per <behavior>. The cycle is now five, so fix the old "7th lane" comment, and add the shared-palette and no-Red assertions.

    (d) Gate (S-09), with $B being Task 1's baseline directory. If $B was lost, re-capture the baselines with the same commands inside a temporary `git worktree add <tmp> <commit before Task 1>`, then `git worktree remove` it. Never stash or reset the working tree for this.
    - `cargo build` must succeed.
    - Run `rtk proxy cargo clippy --all-targets > $B/clippy.after 2>&1`. The set of `^warning:` lines in clippy.after, excluding the `generated N warnings` summary line, must be a subset of those in clippy.before. That means no NEW warnings; there are 5 pre-existing.
    - Run `rtk proxy cargo test --no-fail-fast > $B/test.log 2>&1`. `rtk proxy grep -c ' FAILED$' $B/test.log` must be at most 1, and that line must be the `envelope::policy` git-version witness. Any other failure is a regression to fix.
    - For each of roadmap_graph.rs, roadmap_view.rs and screens/detail.rs, the `Diff in` hunk count from `rtk proxy rustfmt --check --edition 2021 <file>` must be no greater than its baseline in $B. Never run rustfmt without `--check`. Never run `cargo fmt`. Never rustfmt `src/ui/mod.rs`.
    - Record all counts in SUMMARY, together with the full I-1..I-11 list.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast graph_spans</automated>
    <automated>rtk proxy cargo test --lib --no-fail-fast ui::roadmap</automated>
    <automated>bash -c '! rtk proxy grep -q "const GRAPH_PALETTE" src/ui/screens/detail.rs && rtk proxy grep -q "LANE_PALETTE" src/ui/screens/detail.rs && rtk proxy grep -q "LANE_PALETTE" src/ui/roadmap_view.rs && rtk proxy grep -q "pub(crate) const LANE_PALETTE" src/ui/mod.rs'</automated>
  </verify>
  <done>The Roadmap draws rounded fork and merge corners and no square corner survives in any laid fixture. The Git tab colours its lanes from the shared five-colour `LANE_PALETTE` and has no palette of its own. Build is green. Clippy reports no new warning. `cargo test --no-fail-fast` fails only the known `envelope::policy` git-version witness. rustfmt hunk counts for the three touched files did not grow. SUMMARY lists I-1..I-11 and the gate numbers.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| model text → terminal | `ListRow::lanes` holds only static lane/status glyphs built in `roadmap_graph`. Third-party text (ids, names, labels) is escaped upstream and never passes through `lane_spans` |
| process environment → render | `NO_COLOR` is read on each Roadmap list render. Only its presence and emptiness are inspected; its value is never drawn or parsed |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-ujx-01 | Tampering | `lane_spans` per-char rewrite (src/ui/roadmap_view.rs) | low | mitigate | The loop re-emits exactly the chars of the capped `lanes` string, styling them and nothing else. Every row's drawn text is unchanged; the existing lane_text fixtures and the Roadmap render-escape probes stay green under the Task 3 full gate |
| T-ujx-02 | Denial of Service | `links` lookup by char index (src/ui/roadmap_view.rs, src/ui/roadmap_graph.rs) | medium | mitigate | `links` is read only with `.get(i)`, and a short or empty vector degrades to an unlinked char. A test pins `links.len() == lanes.chars().count()` for laid rows. `set_phase_status` swaps one single-char glyph and keeps the length |
| T-ujx-03 | Information Disclosure | `lane_palette_enabled` env read | low | accept | Reads one well-known public opt-out variable's presence, with no secret and no echo. Tests never mutate the process env (I-5) |
| T-ujx-04 | Repudiation | Unrecorded behaviour change (Blocked red, Git hue shift) | low | mitigate | I-3 and I-4 are recorded as inferred decisions and must be copied into SUMMARY for operator audit |
| T-ujx-SC | Tampering | npm/pip/cargo installs | low | accept | No new dependencies: ratatui and std only. No install task, so no package-legitimacy gate applies |
</threat_model>

<verification>
Run from the repo root, after Task 3:
- `cargo build`
- `rtk proxy cargo clippy --all-targets > $B/clippy.after 2>&1`, then compare it to `$B/clippy.before`: no new `warning:` lines.
- `rtk proxy cargo test --no-fail-fast > $B/test.log 2>&1`: exactly one failure, the `envelope::policy` git-version witness, which is environmental on this machine. Always count from the file with `rtk proxy grep`, never through a pipe.
- `rtk proxy rustfmt --check --edition 2021` on roadmap_graph.rs, roadmap_view.rs and screens/detail.rs: the `Diff in` counts must be at most 4, 7 and 517, the baselines measured at planning time and re-captured by Task 1.

Manual look (optional and non-blocking; the tests above are the evidence): `cargo run`, open a project's Roadmap tab, and move the cursor onto a phase with dependencies.
</verification>

<success_criteria>
- Rounded corners on every Roadmap fork and merge, with the glyph tests and updated fixtures green.
- Per-column lane hue from the shared `LANE_PALETTE` on phase, connector, band and summary rows. Done-owner lanes are DarkGray.
- Status-only node glyph styling, with Blocked now Red. `┆` is DarkGray.
- The selection chain is BOLD with hue kept; other lanes are DIM; a band cursor changes no weight.
- `NO_COLOR` drops the lane hue only.
- The Git tab is on the shared five-colour palette.
- The S-09 gate holds: build green, no new clippy warnings, only the known policy.rs failure, and rustfmt hunks not grown.
- SUMMARY lists I-1..I-11 for operator audit.
</success_criteria>

<output>
Create `.planning/quick/261006-ujx-roadmap-tab-rounded-lane-curves-and-ansi/261006-ujx-SUMMARY.md` when done.
</output>
