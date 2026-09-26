---
quick_id: 260926-kes
phase: quick-260926-kes
plan: 01
subsystem: ui/mouse
status: complete
tags: [mouse, tui, detail-view, roadmap, config, docs, git, backlog, queue, driver]
requires: [quick-260926-dyf, quick-260926-jnf]
provides:
  - "RoadmapModel::target_at; RoadmapViewState.list_body / fold_marks"
  - "MouseArm {None, Replay(KeyCode), Swallow} replay for double-clicks"
  - "DetailRegions.fold_marks / config_rows / dropdown / scroll_panes"
  - "ClickTarget::{FoldMarker, ConfigRow, DropdownOption, DropdownOutside}; WheelTarget::Pane"
  - "persisted list offsets for Roadmap-adjacent lists: Docs Files, Milestones x3, Backlog, Git, Queue, Config, Driver runs"
  - "scroll_git_commit / scroll_driver_output shared by PageUp/PageDown and the wheel"
affects: [src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs, src/ui/screens/driver.rs, src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md]
tech-stack:
  added: []
  patterns:
    - "rects recorded at render into DetailRegions (RefCell), reset every frame on both render paths"
    - "pure hit tests (click_target / wheel_target) routed into the existing handle_key arms"
    - "render_offset_list: the persisted-offset ListState idiom as one call"
key-files:
  created: []
  modified:
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_view.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "I-1..I-17 as planned (MouseArm, band double = Space, measured fold-glyph cells, Cfg value click on the already-selected row, modal chooser, Queue double inert, persisted offsets, dashboard unchanged)"
  - "E-1 render_offset_list helper; E-2 Queue block drawn apart from its List; E-3 one render_driver method for both render paths"
metrics:
  duration: "~31 min (14:55 -> 15:26 -05:00)"
  completed: 2026-09-26
  tasks: 3
  files: 7
estimate:
  tokens: 300000
  tasks: 3
actuals:
  tokens: 36400
  tasks: 3
  commits: 5
plan_head_before: b01586089b41f1094782226d0493e97d71478586
---

# Quick 260926-kes: mouse row interaction on the remaining detail tabs — Summary

**One-liner:** every keyboard-selectable detail-view list (Roadmap, Cfg, Docs Files/Milestones, Backlog, Git, Queue, Driver runs) is now click-selectable through its scroll offset. A double-click replays that list's Enter through a `MouseArm` (Space on a Roadmap band; Queue is inert). Roadmap fold glyphs, the Cfg value column and the Cfg chooser are clickable, and the wheel scrolls the Backlog, Git-commit, Driver-output and Docs-file panes under the pointer. Every rect is measured from the spans or `ListState` actually drawn.

## Commits

| # | Hash | Message |
|---|------|---------|
| 1 | d85b72f | test(quick-260926-kes): add failing Roadmap and Docs mouse row tests (RED, compile-fail) |
| 2 | 1efb554 | feat(quick-260926-kes): click-select Roadmap and Docs rows, fold markers, MouseArm replay |
| 3 | 79053f4 | test(quick-260926-kes): add failing Cfg, Backlog, Git, Queue and Driver mouse tests (RED, compile-fail) |
| 4 | c9ac9af | feat(quick-260926-kes): mouse rows on Cfg, Backlog, Git, Queue and Driver; pane wheel |
| 5 | 3068cd4 | docs(quick-260926-kes): document the per-tab mouse map and what is left out |

`git rev-list --count b015860..HEAD` = 5 (measured). SUMMARY/STATE are left for the orchestrator's docs commit.

## Measured gates

| Gate | Result |
|------|--------|
| Baseline (re-measured before Task 1, HEAD b015860) | 56 suites, **2773 passed / 1 failed / 15 ignored** |
| `rtk proxy cargo test --no-fail-fast` (final, HEAD 3068cd4) | 56 suites, **2797 passed / 1 failed / 15 ignored**. The single failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (the expected local git-version witness). +24 = the 24 new `mouse_*` tests |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exit 0 |
| `rtk proxy cargo clippy -- -D warnings` | exit 0 |
| No-render-I/O scan of added lines in `git diff b015860..HEAD -- src/ui` | Clean in non-test code. The only matches are test fixtures: `mouse_docs_files_ctx` (tempdir `create_dir_all`/`write`), `mouse_queue_ctx` (`create_dir_all`) and `mouse_tree_snapshot` (`read_dir`/`is_dir`/`read`, the Queue no-write proof) |
| Task verifies | `-- mouse_ roadmap`: 228 passed. `-- mouse_ config git backlog queue driver`: 586 passed + the witness. `-- mouse_help mouse_ help`: 86 passed |

## New tests (24)

- roadmap_graph: `mouse_target_at_maps_rows_and_skips_connectors`
- roadmap_view: `mouse_roadmap_view_records_the_list_body_and_fold_glyph_cells` (120x30, 80x24 and 120x12; offsets 0 and 4; three models; empty model and zero area)
- detail, Task 1:
  - `mouse_roadmap_click_selects_the_row_under_the_pointer`
  - `mouse_roadmap_click_maps_through_the_scrolled_offset`
  - `mouse_roadmap_fold_marker_click_toggles_the_band`
  - `mouse_roadmap_double_click_folds_bands_and_opens_phases`
  - `mouse_roadmap_wheel_steps_the_cursor_over_list_and_detail`
  - `mouse_docs_files_click_selects_and_double_click_enters_or_opens`
  - `mouse_docs_milestones_click_selects_at_each_depth`
  - `mouse_docs_lists_map_clicks_through_their_persisted_offset`
  - `mouse_docs_file_view_wheel_scrolls_the_content`
- detail, Task 2:
  - Cfg: `mouse_config_click_on_key_or_value_selects_the_row`, `mouse_config_rows_value_column_matches_the_rendered_value`, `mouse_config_value_click_on_the_selected_row_is_enter`, `mouse_config_double_click_is_enter_and_a_secret_prompt_opens_empty`, `mouse_config_chooser_click_highlights_then_applies_and_outside_closes`, `mouse_config_wheel_moves_the_selection_and_steps_an_open_chooser`, `mouse_config_text_prompt_still_ignores_the_mouse`
  - Backlog, Git, Queue, Driver: `mouse_backlog_click_selects_and_double_click_opens_the_pane`, `mouse_git_click_selects_and_double_click_loads_the_commit`, `mouse_git_wheel_over_the_commit_pane_scrolls_the_message`, `mouse_queue_click_selects_and_double_click_never_marks_done`, `mouse_driver_click_selects_a_run_and_wheel_scrolls_its_output`
  - `mouse_scrolled_lists_map_clicks_through_their_persisted_offset` (Backlog, Git, Queue, Driver runs)
- Extended dyf tests (not new): `mouse_click_target_is_a_pure_function_of_the_regions` (FoldMarker beats its row; ConfigRow maps visible to underlying and sets `on_value`; an open chooser makes DropdownOption/Nothing/DropdownOutside beat tabs and rows), `mouse_wheel_target_is_a_pure_function_of_the_regions` (each `Pane(..)` inside its rect; Waves still first), `mouse_regions_reset_every_frame_on_both_render_paths` (`fold_marks`, `config_rows`, `dropdown` and `scroll_panes` are reset on `render` and on `render_main_only`), `mouse_help_block_is_documented_as_whole_rows`.

## Final per-tab mouse map

| Tab | Click | Double-click | Wheel |
|-----|-------|--------------|-------|
| 1 Roadmap (list view) | Phase, band or shipped row selects it (`roadmap_cursor`, edge walk cleared). A connector does nothing. The `▸`/`▾` cell plus its trailing space folds or unfolds that band or the shipped group, puts the cursor on it, and swallows the double press | Band or shipped row: fold (Space). Phase row: Enter, which opens it in Phases | Over the list or the detail pane: steps the cursor one target, never climbing to the tab bar |
| 1 Roadmap (box view `v`) | Focus only | none | The existing generic scroll |
| 2 Phases / Waves pane | unchanged (dyf) | unchanged (dyf) | unchanged (dyf) |
| 3 Backlog | Selects a row. If the content pane is open, closes it first | Enter: opens the pane on that row (loads content) | Over the open pane: scrolls it 1 line (its j/k). Over the list while the pane is open: nothing. Otherwise moves the selection |
| 4 Git | Selects a commit. A different row closes an open commit pane; the same row keeps it | Enter: loads the commit (`git show`) | Over the commit pane (message and Files): scrolls the message 1 line, clamped. Over the log: the j/k path (moves and closes the pane) |
| 5 Queue | Selects the row | **nothing** (Enter would mark it done and save) | Moves the selection |
| 6 Sessions / Agents | unchanged (dyf) | unchanged (dyf) | unchanged (dyf) |
| 7 Cfg | A click on the category, key, value or `*` cell selects the underlying entry, through the `/` filter and the scroll. A value click on the row that was already selected (visible, no chooser open) is Enter: chooser, prompt (a secret opens empty) or integer step, and the double press is swallowed | Enter (same rules) | Moves the selection through the visible rows, never climbing. With the chooser open it steps the chooser and applies nothing |
| 7 Cfg chooser (open) | An option highlights it. A click on the highlighted option applies it (Enter). Anywhere outside the popup, tab bar included, is Esc: closes it, applies nothing | Applies the option | Steps the chooser |
| 7 Cfg text prompt | ignored (dyf I-13) | ignored | ignored |
| 8 Docs Files | Selects a row at List depth. The Files/Milestones labels switch sub-tab (dyf) | Enter: enters a folder, opens a `.md` | Over an open file: scrolls it 1 line. Over the list: moves the selection |
| 8 Docs Milestones | Selects at MilestoneList / PhaseList / FileList (per-depth offsets) | Enter: descends a level or opens a file | Over an open file: scrolls it. Over a list: moves the selection |
| Driver (flag on) | A run row selects it through `move_driver_selection` (resets the output pane, re-arms follow, reschedules the scan) | nothing (the tab has no Enter) | Over the run detail: scrolls the output 1 line. Up clears follow; reaching the tail re-arms it (D-19). Over the run list: moves the selection |

## Dashboard audit (I-13)

`NormalScreen::handle_mouse` (`src/ui/screens/normal.rs` ~749) and `render_main` (~1091, `table_region`) already make the project table click-selectable (with the persisted table offset), double-click = Enter and wheel over the table = step. The table is the dashboard's only keyboard-selectable widget, so there was **no code change**.

## Inferred decisions (for audit)

The planned ones, all implemented as written:

- **I-1:** `MouseArm {None, Replay(KeyCode), Swallow}` replaces `mouse_row_armed`. A double never re-hit-tests. A double with arm `None` falls through to the single-click path, as dyf's did. `NormalScreen`'s bool is untouched.
- **I-2:** a Roadmap band or shipped-row double-click is Space. On a phase row it is Enter. On a connector it arms nothing.
- **I-3:** the fold-marker hit area is the two cells of the drawn `"{glyph} "` span. Its column is the summed width of the spans pushed before it in `row_line`, then confirmed against the fitted line (a glyph `fit` cut away records no marker). A click on it runs `roadmap_activate(Space)` and arms Swallow.
- **I-4:** the Roadmap detail pane has no scroll of its own; the wheel over it steps the cursor. The box view records no list.
- **I-5:** Cfg `value_x` is measured per row from the spans vector the row draws; a long pass-through key pushes it right (pinned). A value click is Enter only on the already-selected, visible row with no chooser open.
- **I-6:** the chooser is modal: option / highlighted-option-applies / outside = Esc. The wheel keeps stepping it.
- **I-7:** the Cfg `d` scope title gets no click target.
- **I-8:** Backlog: a row click closes the open pane, and the double re-opens it. The pane wheel is its j/k. The list wheel is inert while the pane is open.
- **I-9:** Git: `scroll_git_commit` is extracted, and PageDown/PageUp call it with `±PAGE_SCROLL_LINES`.
- **I-10:** a Queue row click arms nothing (T-kes-02).
- **I-11:** `scroll_driver_output` is extracted (D-19 preserved). A run click uses `move_driver_selection`. No output rect is recorded under the dry-run preview.
- **I-12:** persisted offsets `browser_list_offset`, `archive_list_offsets[3]`, `backlog_list_offset`, `git_list_offset`, `queue_list_offset`, `config_list_offset` (visible positions) and `driver_list_offset`.
- **I-13:** dashboard unchanged.
- **I-14:** one wheel event is one row or one line.
- **I-15:** Docs Files is a drill-down list: a double-click is the Enter arm.
- **I-16:** a row click sets `roadmap_cursor` and clears `roadmap_edge_walk`, with no unfold.
- **I-17:** every test name starts with `mouse_`.

Added during execution:

- **E-1:** `DetailScreen::render_offset_list` makes dyf's persisted-offset idiom one call (seed `ListState::with_offset`, draw, store the offset back, `record_list`). Docs Files, the three Milestones depths, Backlog (both layouts), Git and Queue use it. Sessions, Agents and Phases keep their inline copies (untouched).
- **E-2:** the Queue list's bordered block is now drawn on its own and the `List` renders into the block's inner area, so the recorded rect is exactly the item rows. The cells drawn are identical.
- **E-3:** both `render` and `render_main_only` now call one `DetailScreen::render_driver`, which calls `driver::render_driver_tab` and records its `DriverTabRegions { list, output }`. Neither path can record less than the other.
- **E-4:** defensive no-ops:
  - a `FoldMarker` click off the Roadmap list view only focuses;
  - a `DropdownOption` click with no chooser open does nothing;
  - a Browse row click outside List depth and an Archive row click in FileView arm nothing.
- **E-5:** Task 2's implementation was drafted before its tests. To keep the TDD commit order honest, the draft was saved as a patch, the two files were restored (`git checkout -- <file>`), the tests were written and committed RED (compile-fail, as jnf's RED commits were), and the patch was re-applied for the feat commit.
- **E-6 (test fix, in the feat commit):** `mouse_config_click_on_key_or_value_selects_the_row` first used absolute visible positions 3 and 5. The `mode` row opens with the list scrolled (offset 59), so the test now picks two drawn rows off the recorded offset. It was a test bug, not a behaviour change.
- **E-7 (smoke setup):** reading (d) needed a multi-line commit message to show visible scrolling. One long-body commit was added to the **scratch** repo, and the log was reloaded with `p` `p`.

## Intentionally left out

1. **Queue double-click (I-10).** Enter there is a destructive, persisted "mark done" (removed from the queue and saved, no undo).
2. **The Cfg scope title (`d` toggle, I-7).** It is a block title, not a tab strip.
3. **Labels:** breadcrumbs (the Docs Files header, the Milestones breadcrumb), the Git mode line (`p` hint), footers and titles.
4. **Roadmap box-view rows** (there is no cursor), and the phase names in the Roadmap detail pane's Needs / Unblocks / Parallel lines (click-to-jump would duplicate `h`/`l`).
5. **The wheel over the Backlog list while its content pane is open (I-8).**
6. **The help overlay and every modal, overlay and text-entry screen**, the Cfg String/Secret prompts included (dyf I-13).
7. **dyf's standing fences:** the `‹`/`›` overflow markers, drag, right/middle click, hover and horizontal scroll.

The README "Not mouse-driven" paragraph carries these in condensed form.

## Old → new test / expectation mappings

- **Persisted offsets:** no existing test expectation changed. The full suite went 2773 → 2797 passed with the same single failure.
- **`mouse_help_block_is_documented_as_whole_rows`:**
  - `Click` "Mouse: switch tab / sub-tab; select a row and focus its pane" → "Mouse: switch tab / sub-tab; select a row on any tab and focus its pane"
  - `Double-click` "Mouse: open the row, the same as Enter" → "Mouse: open the row, the same as Enter (Space on a Roadmap band)"
  - `Wheel` "Mouse: move the selection in the pane under the pointer" → "Mouse: move the selection, or scroll the pane, under the pointer"
  - New rows: `Click value` "Mouse: Config tab: on the selected row, the same as Enter" and `Click \u{25B8}/\u{25BE}` "Mouse: Roadmap: fold / unfold that milestone band"
  - `M` and `Shift+drag` are unchanged.
- **`mouse_click_target_is_a_pure_function_of_the_regions`:** the struct literal gains `fold_marks`, `config_rows`, `dropdown` and `scroll_panes`. The old assertions are unchanged; new ones are added.
- **Driver PageUp/PageDown/follow tests and Git pane PageUp/PageDown tests:** pass unchanged through the extracted helpers.

## tmux SGR smoke test (D-08)

Setup:

- Private server `tmux -L kes`, 120x36, `bash --norc`, `target/debug/gsd-meta-manager --config <scratch>/config.json` with `HOME=<scratch>/home`.
- The scratch project is a copy of `.planning/`, `git init` with 41 commits, registered as `proj`.
- The startup scan auto-registered four other projects into the scratch config. They were never clicked; `proj` was opened with the keyboard.
- The server was killed afterwards (`tmux -L kes ls` → "no server running").
- The real repo's `git status` shows only the pre-existing untracked `.gsd/` and `.planning/state.json`.

| # | Action (1-based SGR) | Before | After |
|---|----------------------|--------|-------|
| a1 | Roadmap: click phase 16 row (30,14) | detail pane title `Phase 22` | title `Phase 16`, `▶` marker on row 16 |
| a2 | Click the v2.0 band's `▾` (11,11) | `▾ v2.0 …` with phases 14-25 listed | `▸ v2.0 …`, phases hidden, detail pane `v2.0` |
| a3 | Double-click the v2.0 band label (25,11) | `▸` folded | `▾` unfolded, phases 14… listed again |
| a4 | Wheel-down twice over the detail pane (95,20) | cursor on band `v2.0` | detail title `Phase 14`, then `Phase 15` |
| b1 | Cfg: `sha256(config.json)` | `86347526…709f` | — |
| b2 | Click the `plan_check` key cell (30,6) | `research` highlighted (bg 8), help pane for research | `plan_check` highlighted, help pane for plan_check |
| b3 | Click `plan_check`'s value (53,6) | no popup | `┌ plan_check ┐` chooser with `● true` / `false` |
| b4 | Click outside the popup (5,28) | popup drawn | popup gone. `sha256(config.json)` still `86347526…709f` (unchanged) |
| c1 | Docs Files at the `.planning` root: double-click `[DIR] codebase` (12,6) | breadcrumb `Docs > .planning` | breadcrumb `Docs > .planning/codebase` |
| c2 | Double-click `ARCHITECTURE.md` (8,6) | file list | view open, gutter `1 2 3 4` |
| c3 | Wheel-down x3 over the text (40,15) | gutter starts at `1` | gutter starts at `4` |
| d1 | Git, after PageDown x2 (list scrolled, first row `commit 28`): click (30,7) | `>` on `init` | `>` on `smoke commit 27` (the row under the pointer) |
| d2 | Double-click (30,7) | no pane | `Commit: 3b23779` pane, message `smoke commit 27` |
| d3 | Double-click the long-body commit (30,6), then wheel-down x3 over the pane (40,21) | message shows `long body commit` / blank / `body line 1` | message shows `body line 2..5`. The selection stays on `8686acb` and the pane stays open |
| e1 | Queue: `a` → `/gsd:progress` → Enter; then double-click it (20,6) | `Queue (1 items)`, `QUEUE.md` sha `08329403…1a82` | still `Queue (1 items)` with `/gsd:progress`. `QUEUE.md` sha unchanged, no status message |

## D → proof

| D | Proof |
|---|-------|
| D-01 Roadmap | `mouse_roadmap_*` tests (5, detail), `mouse_target_at_*`, `mouse_roadmap_view_records_*`; smoke a1-a4 |
| D-02 Cfg | `mouse_config_*` tests (7), the pure click-target extension; smoke b1-b4 |
| D-03 Docs | `mouse_docs_*` tests (4), dyf `mouse_click_on_a_sub_tab_switches_it` (Files → Milestones); smoke c1-c3 |
| D-04 other tabs + dashboard | `mouse_backlog_*`, `mouse_git_*` (2), `mouse_queue_*`, `mouse_driver_*`, `mouse_scrolled_lists_*`; the dashboard audit above; smoke d1-d3, e1 |
| D-05 architecture | regions recorded at render and reset on both paths (reset test extended); pure `click_target`/`wheel_target`; keyboard arms reused (`handle_key`, `roadmap_activate`, `move_driver_selection`, the extracted scroll helpers); no-render-I/O scan clean |
| D-06 tests | 24 new `mouse_*` tests plus 4 extended |
| D-07 docs | help rows, README table and "Not mouse-driven" paragraph, GETTING-STARTED bullet |
| D-08 gates | table above plus the smoke readings |

## Deviations from Plan

None that change scope. The execution-time additions are E-1..E-7 above: helper extraction, the Queue block split, the one Driver render method, defensive no-ops, the RED-commit ordering via a patch, one test fix and the smoke setup.

## Known Stubs

None.

## Threat Flags

None. No new endpoints, file access, auth paths or schema. T-kes-01..05 are mitigated as planned and each is covered by a named test.

## Self-Check: PASSED

- FOUND: src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs, src/ui/screens/driver.rs, src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md (all modified in b015860..HEAD)
- FOUND commits: d85b72f, 1efb554, 79053f4, c9ac9af, 3068cd4
