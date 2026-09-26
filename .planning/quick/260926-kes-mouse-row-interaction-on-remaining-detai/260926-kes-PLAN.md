---
quick_id: 260926-kes
mode: quick
phase: quick-260926-kes
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/help.rs
  - README.md
  - docs/GETTING-STARTED.md
autonomous: true
requirements: [QUICK-260926-kes]

estimate:
  tokens: 300000
  raw_tokens: 300000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Roadmap list: a click on a phase, band or shipped-group row moves the Roadmap cursor to that row, and still hits the right row after the list has scrolled. A click on a row's fold marker (the drawn ▸/▾ cell and its trailing space) folds or unfolds that band or the shipped group. A double-click on a band or the shipped row folds/unfolds it; a double-click on a phase row opens it in Phases, the same as Enter. The wheel over the list or the detail pane steps the cursor (D-01)"
    - "Cfg: a click on the category, key or value cell of an option row selects that option, mapped through the `/` filter and the scroll offset to the underlying entry. A double-click, or a single click on the value of the row that was already selected, runs the tab's Enter arm: chooser, text/secret prompt, or integer step. A secret prompt opened by the mouse is empty (260926-jnf T-jnf-02). In an open chooser a click highlights an option, a click on the highlighted option (or a double-click) applies it, and a click outside the popup closes it without applying anything (D-02)"
    - "Docs: Files and Milestones rows are click-selectable at every depth, including after scrolling. A double-click enters a folder, opens a file, or descends a Milestones level (the Enter arm). The wheel scrolls an open file. The Files/Milestones labels switch sub-tabs (D-03)"
    - "Backlog, Git, Queue and Driver rows are click-selectable through persisted scroll offsets. A double-click is Enter on Backlog (opens the content pane) and Git (loads the commit). A Queue double-click never marks an action done. The wheel over the Backlog content pane, the Git commit pane or the Driver output pane scrolls that pane. Over a list it moves the selection. The dashboard audit is recorded (D-04)"
    - "Every new hit rect is recorded at render time on both detail render paths and reset every frame. Clicks and wheel steps are mapped by pure functions over DetailRegions and then go through the existing keyboard handlers. No render path gains file I/O (D-05)"
    - "`mouse_*` tests pin click→item mapping per tab (scrolled lists and fold markers included), wheel routing, the double-click replay rules and the Queue/secret safety properties (D-06)"
    - "The help overlay, README and GETTING-STARTED describe the new mouse map and list what is intentionally not mouse-driven (D-07)"
    - "`rtk proxy cargo test --no-fail-fast` fails only the git-version witness. `rtk proxy cargo clippy --all-targets -- -D warnings` and `rtk proxy cargo clippy -- -D warnings` exit 0. The tmux SGR smoke test is recorded in the SUMMARY, or its infeasibility is recorded with the reason (D-08)"
  artifacts:
    - path: src/ui/roadmap_graph.rs
      provides: "pub RoadmapModel::target_at(row) -> Option<CursorTarget> (delegates to the private target_of; connectors -> None)"
    - path: src/ui/roadmap_view.rs
      provides: "RoadmapViewState out-fields list_body: Rect and fold_marks: Vec<(usize, Rect)> written by render_list from the spans it draws"
    - path: src/ui/screens/detail.rs
      provides: "MouseArm {None, Replay(KeyCode), Swallow}; DetailRegions.fold_marks / config_rows / dropdown / scroll_panes; ClickTarget::{FoldMarker, ConfigRow, DropdownOption, DropdownOutside}; WheelTarget::Pane(ScrollPane); persisted list offsets for Backlog, Git, Queue, Docs Files, Docs Milestones (per depth), Cfg and Driver runs; scroll_git_commit / scroll_driver_output helpers shared with PageUp/PageDown"
    - path: src/ui/screens/driver.rs
      provides: "render_driver_tab takes the persisted run-list offset and returns the run list's ListRegion and the run-detail rect"
    - path: src/ui/screens/help.rs
      provides: "Mouse block rows for row clicks on every tab, the fold marker, the Config value click, double-click and wheel"
  key_links:
    - from: "render_roadmap / render_backlog_tab / render_git_tab / render_queue_tab / render_browser_tab / render_archive_tab / render_defaults_tab / driver::render_driver_tab"
      to: "DetailRegions (RefCell), reset by reset_regions on render and render_main_only"
      via: "record_list plus the new record_* helpers, fed from the same ListState/spans the widget drew"
    - from: "DetailRegions::click_target / wheel_target (pure)"
      to: "DetailScreen::click / wheel"
      via: "ClickTarget / WheelTarget per current sub-view"
    - from: "DetailScreen::click / the MouseArm replay"
      to: "handle_key(Enter | Space), roadmap_activate, move_driver_selection, scroll_git_commit, scroll_driver_output"
      via: "the keyboard code paths, never a second copy of their logic"
---

# Quick 260926-kes: mouse row interaction on the remaining detail tabs

<objective>
Close the mouse gap that quick 260926-dyf left on purpose (its I-9 fence: "Git, Backlog, Queue, Config, Roadmap and Docs rows get the wheel and click-to-focus only"). Every detail-view list that has keyboard selection becomes click-selectable. A double-click performs that list's Enter. The wheel scrolls the pane under the pointer. Roadmap fold markers, the Config value column, the Config chooser, the Git commit pane and the Driver output pane get the specific behaviour the user asked for. All of it follows dyf's architecture: rects are recorded at render into `DetailRegions`, mapped by pure hit tests, and routed through the existing keyboard code paths.

Purpose: the user reported that Roadmap, Cfg and Docs rows cannot be clicked, and asked for an audit of every other tab and the dashboard, with anything intentionally left out listed.

Output: code in roadmap_graph.rs, roadmap_view.rs, detail.rs and driver.rs; `mouse_*` tests; help, README and GETTING-STARTED updates; a tmux smoke reading; a SUMMARY that carries the inferred decisions and the left-out list below.

**Decision IDs.** D-01..D-08 are the user's request, in order. They are locked scope:

- D-01 (1:Roadmap): a click selects an item. Milestone bands and the shipped group fold/unfold on a double-click and on a click on the fold marker. The wheel scrolls the list and any detail pane under the pointer.
- D-02 (7:Cfg): a click on EITHER the key column or the value column selects a config option. A double-click, or a click on the value of the already-selected row, behaves like Enter: it opens the chooser/editor under 260926-jnf's secret rules, so a secret is never prefilled. Sub-view/scope switches are clickable if they are tab-like. The wheel scrolls.
- D-03 (8:Docs): a click selects folders/files. A double-click on a folder enters it; on a file it opens it like Enter. The content pane scrolls with the wheel. The Docs sub-tabs (Files/Milestones) are clickable.
- D-04: audit every remaining detail tab (Backlog, Queue, Git, and the Driver tab when the experimental flag is on) and the dashboard, and close the same gap. Every keyboard-selectable list becomes click-selectable, a double-click is Enter, and the wheel scrolls the pane under the pointer. List anything intentionally left out.
- D-05: follow the existing architecture. Rects are recorded at render into `DetailRegions` (RefCell), hit-test/mapping lives in pure functions, render does no file I/O, keyboard and mouse share action code paths, and clicks respect list scroll offsets.
- D-06: tests for each tab's click→item mapping (scrolled lists and fold markers included) and wheel routing.
- D-07: update the help overlay and the README / GETTING-STARTED mouse sections.
- D-08: gates are `rtk proxy cargo test --no-fail-fast` (only the `src/envelope/policy.rs` git-version witness may fail) and `rtk proxy cargo clippy --all-targets -- -D warnings`, plus a tmux smoke test with synthetic SGR mouse input if feasible.

**Inferred decisions (for audit).** The human is unavailable. Copy these into the SUMMARY under "Inferred decisions". Number execution-time additions E-1, E-2, …

- **[I-1] `MouseArm` replaces `mouse_row_armed: bool`** in `DetailScreen`. It is an enum:
  - `None`;
  - `Replay(KeyCode)`: a row click selected something, and the completing double-click replays this key through `handle_key`;
  - `Swallow`: this click already acted, so the completing double press is ignored.
  - dyf's I-7 is kept: a double never re-hit-tests. Any wheel event and any other click reset it to `None`.
  - Why: the Roadmap bands need Space rather than Enter, and without `Swallow` a double-click on a fold marker would toggle twice. A double-click on the Config value would open the chooser and then close it again through the outside-click rule. A value click on an integer row would step it twice.
  - `NormalScreen`'s bool is untouched.
- **[I-2] Roadmap double-click on a band or the shipped-group row is Space (fold/unfold), not Enter.** On the shipped row, Enter would open Docs › Milestones. The user asked for fold by double-click, so the mouse folds. Keyboard Enter still opens Milestones. A double-click on a phase row is Enter (it opens the phase in Phases). A connector row selects nothing and arms nothing.
- **[I-3] The fold-marker hit area is the two cells of the drawn `"{glyph} "` span.** Its column is measured from the spans `row_line` actually builds (for a band, after the lane padding; for the shipped row, after the raw lane text). It is never re-derived as a constant, the same principle as dyf's E-2 for sub-tab rects.
  - A single click on it puts the cursor on that row and calls `roadmap_activate(Space)`, and arms `Swallow`.
  - It is the only single click on the Roadmap with an effect. Fold state is in-memory view state (`roadmap_fold_toggles` in the view cache) and is never persisted, so no dyf T-dyf-03 guarantee about persisted actions is weakened.
- **[I-4] The Roadmap detail pane has no scroll state.** `draw_items` fits its content by truncating. So "wheel scrolls any detail pane under the pointer" becomes: the wheel over the detail pane steps the list cursor, which is the existing Content route, and the pane redraws for the new row.
  - The box view (`v`) has no cursor. A click there only focuses Content; the wheel keeps the existing generic scroll.
- **[I-5] Cfg row geometry.**
  - A click anywhere on an option row selects it (category, key, value, or the ` *` defaults marker).
  - The "value" part starts at the value span's first cell and runs to the row's end. It is measured PER ROW from the spans drawn, because the key column is `{:<30}` of the escaped key and pass-through keys longer than 30 cells push the value right.
  - A single click on the value of the row that was ALREADY selected before the click is Enter. That requires the selection to be visible under the filter and no chooser to be open; the click then arms `Swallow`.
  - A double-click is Enter through `Replay(Enter)`.
  - The integer arm's Enter persists immediately, so this is two deliberate acts, never one (see T-kes-01).
- **[I-6] Cfg chooser (dropdown), mouse rules.**
  - A click on an option sets `defaults_dropdown_selected`.
  - A click on the already-highlighted option applies it through the Enter arm and arms `Swallow`. A double-click on an option also applies it.
  - A click anywhere outside the popup is `handle_key(Esc)`: the chooser closes and nothing is applied. This applies on the tab bar too; the chooser is modal, so a tab click while it is open only closes it.
  - The wheel keeps its existing behaviour: while the chooser is open, Down/Up step it.
  - String/Secret text prompts still ignore the mouse entirely (dyf I-13, `config_text_entry_active`).
- **[I-7] The Cfg scope switch is not tab-like.** The scope switch is `d` (project `config.json` ↔ `~/.gsd/defaults.json`), drawn only as the list block's title (` Config Settings ` / ` Global Defaults (…) `). That is a title, not a tab strip, so it gets no click target. See the left-out list.
- **[I-8] Backlog.** The open content pane IS `backlog_expanded` (1t1 I-10: not a separate focus level).
  - A click on a list row while the pane is open is a click on the list level. It closes the pane (`backlog_expanded = false`, `backlog_scroll = 0`), selects the row, and arms `Replay(Enter)`. A double-click therefore re-opens the pane on the new row through the Enter arm, which loads the content.
  - A click inside the open pane only focuses Content.
  - The wheel over the open pane scrolls it through `handle_key(Down/Up)`; the pane owns j/k.
  - The wheel over the list while the pane is open is ignored, because the pane owns j/k.
- **[I-9] Git.**
  - A click on a commit selects it. If the selection changed it also runs `close_git_commit_detail()`, exactly as every j/k/PageUp/PageDown selection move does. It arms `Replay(Enter)`, so a double-click loads the commit.
  - The wheel over the commit pane area (message and Files together) scrolls the message one line per step through `scroll_git_commit`. That helper is extracted from the PageDown/PageUp arms: PageDown adds then clamps, PageUp clamps first. Those two arms now call it with ±`PAGE_SCROLL_LINES`.
  - The wheel over the log moves the selection (existing).
- **[I-10] Queue: a click selects, a double-click arms NOTHING.** Queue's Enter marks the selected action done. That removes it from the queue and saves through `queue_mutate_and_save`, and there is no undo. A stray double-click must not delete queued work. Keyboard Enter is unchanged. Rating: reversible. Wiring it later is a one-line arm change.
- **[I-11] Driver (experimental flag on).**
  - A click on a run row selects it through `move_driver_selection(ctx, delta)`, the j/k path, which also resets the output pane and reschedules the scan. It arms nothing; the tab has no Enter action.
  - The wheel over the run detail scrolls the output pane one line per step through `scroll_driver_output`. That helper is extracted from the PageDown/PageUp arms and keeps D-19 exactly: going down re-arms follow at the tail, going up clears follow.
  - The wheel over the run list moves the selection (existing).
  - While the dry-run preview replaces the run detail, no output rect is recorded.
- **[I-12] Persisted list offsets** (dyf I-12 pattern: seed `ListState::with_offset` from a `Cell<usize>`, store `offset()` back, record the same offset in the region):
  - `backlog_list_offset`, `git_list_offset`, `queue_list_offset`, `browser_list_offset`;
  - `archive_list_offsets: [Cell<usize>; 3]`, one per `archive_selected` depth;
  - `config_list_offset`, in VISIBLE positions;
  - `driver_list_offset`.
  - Roadmap already persists `roadmap_list_offset`.
- **[I-13] Dashboard audit: no change.** The only keyboard-selectable widget on the dashboard is the project table. dyf already made it click / double-click / wheel, with a persisted offset (`NormalScreen::handle_mouse`, `render_main`'s `table_region`).
- **[I-14] One wheel event is one step.** That means one row, or one text line for the Git message and Driver output panes: dyf I-10 unchanged.
- **[I-15] Docs Files is a drill-down list, not a tree.** It is `browser::list_dir` with `browser_current_dir`. "Expand/collapse" therefore means "enter": a double-click on a folder is the Enter arm (enter), and on a file it is the Enter arm (open, `BrowserDepth::View`). Going up stays on the existing keys.
- **[I-16] A Roadmap row click sets `roadmap_cursor = Some(target)` and clears `roadmap_edge_walk`,** as `roadmap_nav` does for every non-edge move. No unfold is needed, because a clicked row is by definition visible.
- **[I-17] Test names start with `mouse_`,** so `-- mouse_` selects the dyf tests and these together.

**Intentionally left out.** Copy this into the SUMMARY and the README Mouse section (short form):

1. Queue double-click (I-10). Enter there is a destructive, persisted "mark done".
2. The Cfg scope title (`d` toggle) (I-7). It is not tab-like.
3. Breadcrumbs (the Docs Files header, the Milestones breadcrumb), the Git mode line (`p` hint), footers and titles. They are labels, not controls.
4. Roadmap box-view rows (there is no cursor), and the phase names inside the Roadmap detail pane's Needs / Unblocks / Parallel lines (click-to-jump would duplicate `h`/`l` edge-follow).
5. The wheel over the Backlog list while its content pane is open (I-8).
6. The help overlay and every modal, overlay and text-entry screen, Cfg String/Secret prompts included. These are dyf I-13, unchanged.
7. dyf's standing fences: `‹`/`›` overflow markers, drag, right/middle click, hover, horizontal scroll.

**Working rules.**

- **Line numbers drift.** Every line number below is a locator to grep near, not a contract. detail.rs is about 23.2k lines; grep, then Read with offset/limit, and never read it whole.
- **Baseline.** The latest measured baseline is the 260926-jnf SUMMARY: 56 suites, **2773 passed / 1 failed / 15 ignored**. HEAD 2ab99f0 is docs-only on top of it. Re-measure before Task 1. The single failure, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, is expected locally; any other failure is yours.
- **Test runs.** Always use `rtk proxy cargo test --no-fail-fast …`. Redirect the full run to a scratch file and sum its `test result:` lines from the file with `rtk proxy awk`. Never pipe cargo into grep/awk: rtk filters downstream of `rtk proxy`, which truncates the counts (project memory).
- **Clippy.** `rtk proxy cargo clippy --all-targets -- -D warnings` must exit 0 (260926-fcp cleared the pre-existing findings), and so must `rtk proxy cargo clippy -- -D warnings`.
- **Changed expectations.** If persisting an offset changes any existing test expectation, do not loosen it. Record it in the SUMMARY as an old → new mapping with the reason, the way dyf reported "none".
- **Where you run.** Master, main tree, no worktree isolation, no push.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/260926-dyf-enable-mouse-support-in-the-tui/260926-dyf-SUMMARY.md
@src/ui/mouse.rs

Interfaces as they are today (grep to confirm):

- `src/ui/mouse.rs`: `MouseInput::{Click{column,row,double}, Wheel{column,row,down}}`; `ListRegion { rect, offset, len }` with `row_at(column, row) -> Option<usize>`, which maps through the offset and is `None` past `len`; `ClickTracker` (500 ms, same row, ±1 column); `tab_entry_rects`.
- `src/ui/roadmap_graph.rs`:
  - `BAND_OPEN` (▾, U+25BE) ~232 and `BAND_FOLDED` (▸, U+25B8) ~234.
  - `BandKey { Shipped, Named }` ~313; `CursorTarget { Phase(key), Band(BandKey) }` ~323; `ListRow { ShippedSummary, Band, Connector, Phase }` ~354; `RoadmapModel { rows, phases, bands, … }` ~432.
  - Private `target_of(&ListRow)` ~993. `row_of` ~1068, `resolve_cursor` ~1048, `step` ~1077.
- `src/ui/roadmap_view.rs`:
  - `RoadmapViewState { offset, list_rows }` ~96 derives `Debug, Default, Clone, Copy`.
  - `render_list` ~559-620: the block inner; then two non-scrolling lines (Start now, header); then `body` model rows from `offset`; then an optional Notes line on the last row.
  - `row_line` ~661-736: the Band glyph span `format!("{glyph} ")` comes after `lane_spans(…, cols.lane)` (padded to `cols.lane`). The ShippedSummary glyph span comes after the raw capped lanes span.
  - `StatefulWidget::render` ~1261: `panes_for` splits list/detail; the empty-model early return is ~1267.
  - Test `render` helper ~1376.
- `src/ui/screens/detail.rs`:
  - `DetailScreen` fields ~782-854 (`roadmap_list_offset`, `phase_list_offset`/`sessions_offset`/`agents_offset`, `regions: RefCell<DetailRegions>`, `mouse_row_armed`, 8 uses). `new` ~1026.
  - `DetailRegions` ~873-897 (`tab_bar`, `sub_tab_strip`, `content`, `pane`, `waves_pane`, `waves_rows`, `tabs`, `sub_tabs`, `list`). `ClickTarget` ~917; `WheelTarget` ~929; `click_target` ~939; `wheel_target` ~966.
  - `reset_regions` ~1058 (constructs every field); `record_list` ~1169; `record_pane` ~1202; `sub_tab_row` ~1225.
  - `move_driver_selection` ~1301; `roadmap_model_and_cursor` ~1346; `roadmap_nav` ~1362; `roadmap_activate` ~1413; `content_at_first_row` ~1831.
  - `handle_key` arms:
    - Esc closing an open Cfg chooser ~3157;
    - j/Down ~3182;
    - PageDown Git ~3503 and Driver ~3649;
    - PageUp Git ~3680 and Driver ~3801;
    - Enter|Space ~3892: Queue 3897 (mark done + save), Backlog 3937, Git 3981, Archive 4156, Defaults 4265 (secret opens empty ~4317), Browse 4336, Roadmap 4366;
    - `d` scope toggle ~4571.
  - `render` ~5146 and `render_main_only` ~6983 both call `reset_regions` and the same per-tab renders.
  - `handle_mouse` ~5257, `wheel` ~5292, `click` ~5322.
  - Per-tab renders:
    - `render_roadmap` ~5518 (the stateful `RoadmapView` render and the offset store ~5634-5647);
    - `render_backlog_tab` ~5658 (fresh `ListState`; expanded split ~5733; `record_pane(chunks[1])` ~5754);
    - `render_git_tab` ~5804 (`log_area` list ~5941; `detail_area = content_chunks[2]` ~5947);
    - `render_queue_tab` ~6310 (a `List` with its own block rendered into `inner`);
    - `render_sessions_tab` ~6368, whose persisted-offset + `record_list` idiom at ~6457-6466 is the pattern to copy;
    - `render_archive_tab` ~6567 (three list depths plus FileView);
    - `render_browser_tab` ~6753 (List depth / View depth);
    - `render_defaults_tab` ~7045: row spans ~7169 (`"  "`, category `{:<18}`, `" "`, key `{:<30}` of the escaped key, value, optional `" *"`); the list block `Borders::TOP` ~7220; the `ListState` selects the VISIBLE position ~7244-7252; the chooser popup ~7336-7402; the text prompt popup ~7274.
  - Helpers: `visible_defaults_indices`, `defaults_selection_visible`, `entries_for_cache`, `dropdown_options`, `share_pipeline_selection`, `switch_to_sub_view`, `clamp_scroll`, `driver_offset_now`, `tail_offset`, `PAGE_SCROLL_LINES`.
  - Test helpers:
    - `test_ctx` ~15049, `press` ~15156, `render_detail_buffer(&screen, &ctx, w, h)`, `buffer_row`;
    - `git_rows_fixture` ~15370 and `git_pane_fixture(body_lines, scroll)` ~15398;
    - `backlog_content_fixture` ~18373; `ctx_on_config_row(config, key)` ~18685; `populated_gsd_config` / `sparse_gsd_config`;
    - `editing_a_secret_row_never_prefills_and_masks_typed_input` ~18912 (the secret-row pattern);
    - `roadmap_fixture("daily-vow" | "ttbook" | …)` ~19730 with `fixture_model`, `band_key`, `fold_toggles`, `set_roadmap_cursor`, and the fold tests ~20141;
    - `test_run(…)` for Driver runs ~15675; `md_entry(name)` for Browse ~13440;
    - mouse helpers ~22499-22562 (`mouse_click`, `mouse_double`, `mouse_click_twice`, `mouse_mid`, `mouse_list`, `mouse_wheel` ~22991);
    - the dyf mouse tests ~22565-23200.
- `src/ui/screens/driver.rs`: `render_driver_tab(frame, area, ctx, alias, cache, viewport)` ~997 (runs `List` with a `Borders::RIGHT` block and a fresh `ListState` ~1051-1069; `detail_area` ~1073). Its callers are detail.rs ~5204 and ~7034, and driver.rs tests ~3996, ~4087 and ~4288.
- `src/ui/screens/normal.rs`: `handle_mouse` ~749; the table region ~1078-1096 (reference only, no change).
- `src/ui/screens/help.rs`: Mouse block ~285-297 (every description unique in the body); `mouse_help_block_is_documented_as_whole_rows` ~1109.
- `README.md` Mouse section ~286-301; `docs/GETTING-STARTED.md` mouse paragraph ~156-158.
- House rule: no raw non-ASCII glyph in Rust source; write `\u{25B8}` / `\u{25BE}` escapes (see `normal.rs` ~55-70, `DRIVER_LIVE_MARKER`).
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): the MouseArm replay + new region plumbing, proven end to end on the Roadmap (rows, fold markers, wheel), then expanded to Docs Files and Milestones</name>
  <files>src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs</files>
  <behavior>
    - mouse_target_at_maps_rows_and_skips_connectors (roadmap_graph.rs): for a model with a band, the shipped summary, phases and at least one connector, `target_at(i)` equals the target of `rows[i]` (Band / Band(Shipped) / Phase). It is `None` for connector rows and for `i >= rows.len()`.
    - mouse_roadmap_view_records_the_list_body_and_fold_glyph_cells (roadmap_view.rs), at 120x30 (side by side) and 80x24 (stacked):
      - `list_body` starts two rows below the list block's inner top and ends above the Notes line;
      - every `fold_marks` entry's first cell holds `BAND_OPEN` or `BAND_FOLDED` in the rendered buffer, and its row index is that row's model index;
      - with a state offset > 0, only visible rows have marks, and their y matches `list_body.y + (row - offset)`;
      - an empty model and a zero-size area leave `list_body` empty and `fold_marks` empty.
    - mouse_roadmap_click_selects_the_row_under_the_pointer (detail.rs, `roadmap_fixture("daily-vow")`):
      - a click on a phase row's name sets `roadmap_cursor` to that Phase and focuses Content;
      - a click on a band label sets it to that Band;
      - a click on a connector leaves the cursor unchanged;
      - a click on the Start-now or header line changes nothing but focus.
    - mouse_roadmap_click_maps_through_the_scrolled_offset: at a height where the list scrolls, move the cursor near the end with `G`, render, and click the first body row. The cursor becomes `target_at(offset)` with offset > 0, where offset is the recorded one.
    - mouse_roadmap_fold_marker_click_toggles_the_band:
      - a click on a named band's marker folds it (checked with `fold_toggles` / `is_folded`) and parks the cursor on the band;
      - a second, later click on the re-rendered marker unfolds it;
      - `mouse_click_twice` on the marker toggles exactly once (Swallow);
      - a click on the shipped row's marker toggles the shipped group.
    - mouse_roadmap_double_click_folds_bands_and_opens_phases:
      - `mouse_click_twice` on a band label toggles its fold (Replay(Space));
      - on the shipped row it toggles the shipped fold and the sub-view stays RoadmapViz, not Archive;
      - on a phase row that has a Phases entry it lands on the Pipeline tab with `pipeline_selected` on that phase (Replay(Enter)).
    - mouse_roadmap_wheel_steps_the_cursor_over_list_and_detail: a wheel-down over the list and a wheel-down over the detail pane each move the cursor one target, as `j` does. A wheel-up with the cursor on the first target leaves it there and keeps focus at Content (it never climbs).
    - mouse_docs_files_click_selects_and_double_click_enters_or_opens (a tempdir with a subfolder and an .md file, loaded through the Browse arrival):
      - a click selects the row;
      - `mouse_click_twice` on the folder changes `browser_current_dir` to it;
      - on the file it sets `browser_depth == View`.
    - mouse_docs_lists_map_clicks_through_their_persisted_offset: a Files list and a Milestones list longer than the viewport, scrolled by keys. A click on the first visible row selects index == recorded offset > 0. The offset is unchanged by the click's re-render (the row stays under the pointer).
    - mouse_docs_milestones_click_selects_at_each_depth: with `archive_milestones` populated, a click sets `archive_selected[0]` and a double-click descends to PhaseList. With archive data in `ctx.archive_cache`, clicks set `archive_selected[1]` and `archive_selected[2]` at their depths.
    - mouse_docs_file_view_wheel_scrolls_the_content: in Browse View and in Archive FileView, a wheel-down over the text raises the scroll offset by 1, clamped at the bottom. A wheel over the sub-tab strip does nothing.
    - mouse_click_target_is_a_pure_function_of_the_regions (extend the dyf test): FoldMarker beats ListRow on the same cell; tabs still beat everything except an open chooser (Task 2 adds that case).
    - mouse_regions_reset_every_frame_on_both_render_paths (extend the dyf test): the new fields are empty after rendering a tab that does not draw them, on `render` and on `render_main_only`.
  </behavior>
  <action>
Implements D-01, D-03, D-05 and D-06 (Roadmap and Docs), with I-1, I-2, I-3, I-4, I-12, I-15, I-16 and I-17. TDD: commit the failing `mouse_*` tests first, then the implementation.

1. **roadmap_graph.rs.** Add `pub fn target_at(&self, row: usize) -> Option<CursorTarget>` on `RoadmapModel`. It returns `self.rows.get(row)` passed through the existing private `target_of`, so there is one row→target rule. Doc it as the mouse's row mapping.

2. **roadmap_view.rs.**
   - Add two OUT fields to `RoadmapViewState`:
     - `list_body: Rect`, the rect of the scrolling model rows only (below Start-now/header, above Notes);
     - `fold_marks: Vec<(usize, Rect)>`, one per VISIBLE Band / ShippedSummary row: the model row index and the rect of its two-cell `"{glyph} "` span.
   - Drop `Copy` from the derive, because a Vec cannot be Copy. Fix any site that relied on it; the compiler will name them.
   - In `render_list`, clear both fields at entry and on the `inner.is_empty()` return. Also clear them in `render`'s empty-model and zero-area returns.
   - Record the glyph column from the spans `row_line` builds: return the glyph span's cell offset alongside the Line, or factor a helper that `row_line` itself uses. Never re-derive it as `cols.lane` or any constant in a second place (I-3).
   - The rect's y is `list_body.y + (i - offset)`. Its x is `inner.x` plus the summed widths of the spans before the glyph, clipped to `inner`.

3. **detail.rs, the shared mechanism** (implement it before the Roadmap arms; this is the tracer's spine).
   - Replace `mouse_row_armed: bool` with `mouse_arm: MouseArm`: a private enum `None | Replay(KeyCode) | Swallow`, with `Default` = `None`.
   - In `handle_mouse`, a `double: true` click does the following:
     - `Replay(k)`: take the arm and return `handle_key(k, NONE)`;
     - `Swallow`: take the arm and return `ScreenAction::None`;
     - `None`: run the single-click path as today.
   - Every other click resets the arm before hit-testing, and every wheel event resets it to `None`.
   - Extend `DetailRegions` with `fold_marks: Vec<FoldMarkRegion { rect, row }>`, and extend `reset_regions` to match. Task 2 adds the remaining fields; add them there, not here, so each commit stays dead-code-free under `-D warnings` (dyf's E-3).
   - Add `ClickTarget::FoldMarker(usize)`. `click_target` checks it after the Waves rows and before the list (I-3).
   - Keep `click_target` and `wheel_target` pure.

4. **detail.rs, Roadmap.**
   - In `render_roadmap`'s list branch, seed `RoadmapViewState` with `offset: self.roadmap_list_offset.get()` and `..Default::default()`.
   - After the render, keep the two existing stores. Then call `record_list(ListRegion { rect: view_state.list_body, offset: view_state.offset, len: model.rows.len() })` and a new `record_fold_marks(view_state.fold_marks)`.
   - The box-view branch records nothing (I-4).
   - In `click`, the `ListRow(i)` arm on `RoadmapViz` does the following:
     - builds `(model, _)` through `roadmap_model_and_cursor` (the keys' one resolution site);
     - takes `model.target_at(i)`;
     - on `Some(t)`: sets `cache.roadmap_cursor = Some(t)` and `cache.roadmap_edge_walk = None` (I-16), and arms `Replay(Char(' '))` for a Band target or `Replay(Enter)` for a Phase target (I-2);
     - on `None` (connector): focus only, arm `None`.
   - The `FoldMarker(i)` arm resolves `target_at(i)` the same way, sets the cursor to it, calls `self.roadmap_activate(KeyCode::Char(' '), ctx)` (the Space path, not a copy), and arms `Swallow`.
   - Wheel: no routing change is needed; Content already goes to `roadmap_nav` Step. Pin it with the wheel test (I-4).

5. **detail.rs, Docs.**
   - `render_browser_tab` List depth: seed a persisted `browser_list_offset: Cell<usize>`, store it back, then `record_list(ListRegion { rect: content_area, offset, len: browser_entries.len() })`. The block is `Borders::NONE`, so inner equals the area.
   - `render_archive_tab`: for each of MilestoneList / PhaseList / FileList, do the same with `archive_list_offsets[depth]` (depth 0/1/2, matching `archive_selected`). `len` is the number of items that list draws (PhaseList: top-level files plus phases).
   - FileView and View depth record no list.
   - The `click` `ListRow(i)` arm:
     - on `Browse`: sets `browser_selected = i` (only while `browser_depth == List`) and arms `Replay(Enter)`;
     - on `Archive`: sets `archive_selected[depth] = i` for the current `archive_depth` (no-op in FileView) and arms `Replay(Enter)`.
   - The Enter arms do the entering and opening (I-15).
   - Confirm dyf's `mouse_click_on_a_sub_tab_switches_it` covers the Files ↔ Milestones pair. If it covers only Sessions/Agents, add the Docs pair to it.

6. Add every new `DetailScreen` Cell to `new`. Keep the whole diff free of file reads in render functions: every recorded value is layout arithmetic over state that is already in memory (D-05).
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_ roadmap</automated>
  </verify>
  <done>All Task 1 `mouse_*` tests pass, along with every existing `roadmap*` and dyf `mouse_*` test. A Roadmap phase-row click, a fold-marker click and a band double-click each work end to end through `handle_mouse`. Docs Files and Milestones rows click-select through persisted offsets, and a double-click enters, opens or descends. `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0. Committed as a test commit and a feat commit.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Cfg (rows, value column, chooser), Backlog, Git, Queue and Driver rows, scroll-pane wheel routing, plus the dashboard audit</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/driver.rs</files>
  <behavior>
    - mouse_config_click_on_key_or_value_selects_the_row:
      - unfiltered, a click on a row's key cell and a click on another row's value cell each set `defaults_selected` to that row's underlying index, and neither opens anything;
      - with a `/` filter confirmed, a click on visible row k selects `visible[k]` (the underlying index, not k);
      - scrolled (select near the end with keys, render at a short height), a click on the first visible row selects the entry at the recorded offset.
    - mouse_config_rows_value_column_matches_the_rendered_value: for every recorded row, the buffer cell at `value_x` starts the drawn value text. Include a config with a pass-through key longer than 30 cells, whose `value_x` sits further right than a short key's.
    - mouse_config_value_click_on_the_selected_row_is_enter:
      - on an enum row already selected, a click on its value opens the chooser (`defaults_editing == Some(idx)`);
      - on an UNselected row the same click only selects;
      - `mouse_click_twice` on the value of an already-selected integer row changes the value exactly once (Swallow);
      - no click opens anything while a filter hides the selected row.
    - mouse_config_double_click_is_enter_and_a_secret_prompt_opens_empty: with the `editing_a_secret_row_never_prefills_and_masks_typed_input` fixture (brave_search holding a key), a double-click on the row opens the secret prompt with `defaults_text_buffer.char_count() == 0`. The rendered tab contains no fragment of the stored key.
    - mouse_config_chooser_click_highlights_then_applies_and_outside_closes:
      - with the chooser open, a click on option j sets `defaults_dropdown_selected = j` without applying;
      - a second click on it applies it (`defaults_editing == None` and the value changed, the `enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick` pattern);
      - in a fresh open, a click outside the popup closes it with the value unchanged;
      - a click on a tab while the chooser is open only closes it (the sub-view is unchanged).
    - mouse_config_wheel_moves_the_selection_and_steps_an_open_chooser:
      - with no chooser open, a wheel-down over the list moves `defaults_selected` to the next visible row (filter respected);
      - with the chooser open, a wheel-down steps `defaults_dropdown_selected` and applies nothing;
      - a wheel-up at the first visible row never climbs to the tab bar.
    - mouse_config_text_prompt_still_ignores_the_mouse: after opening a String prompt by double-click, a click and a wheel change neither the buffer nor the selection. This keeps dyf's `mouse_clicks_are_ignored_while_typing_in_config` intact.
    - mouse_backlog_click_selects_and_double_click_opens_the_pane (`backlog_content_fixture` or inline items):
      - a click selects;
      - `mouse_click_twice` opens the pane (`backlog_expanded`);
      - with the pane open, a click on another list row closes it and selects that row;
      - a wheel over the open pane raises `backlog_scroll`;
      - a wheel over the list while the pane is open changes neither `backlog_selected` nor `backlog_scroll`.
    - mouse_git_click_selects_and_double_click_loads_the_commit (`git_rows_fixture`):
      - a click selects a row;
      - with a commit pane open (`git_pane_fixture`), a click on a DIFFERENT row closes the pane, and a click on the same row keeps it;
      - `mouse_click_twice` sets `loading_commit_detail` for the clicked row.
    - mouse_git_wheel_over_the_commit_pane_scrolls_the_message (`git_pane_fixture`):
      - wheel-down over the message area and over the Files area each raise `git_commit_scroll` by 1, clamped at the bottom;
      - wheel-up at 0 stays at 0;
      - `git_selected` is unchanged;
      - wheel over the log moves `git_selected` and closes the pane (the j path).
      - The existing PageUp/PageDown git pane tests (~15287-15318) pass unchanged.
    - mouse_queue_click_selects_and_double_click_never_marks_done: with two queued actions, a click selects row 1. `mouse_click_twice` on row 0 leaves `queued_actions` unchanged, with the same length and order, sets no status message, and writes no queue file.
    - mouse_driver_click_selects_a_run_and_wheel_scrolls_its_output (`ctx.experimental = true`, two `test_run`s):
      - a click on run row 1 sets `driver_selected_run = 1` and re-arms `driver_follow` (the `move_driver_selection` reset);
      - a wheel-up over the run detail clears `driver_follow` and lowers the offset;
      - a wheel-down back to the tail re-arms it;
      - a wheel over the run list moves the selection.
      - The existing Driver PageUp/PageDown/follow tests pass unchanged.
    - mouse_scrolled_lists_map_clicks_through_their_persisted_offset: for Backlog, Git, Queue and the Driver runs, with each list longer than its viewport and scrolled by keys, a click on the first visible row selects index == recorded offset > 0.
    - mouse_wheel_target_routes_scroll_panes_first (extend dyf's pure wheel test): `Pane(Backlog | GitCommit | DriverOutput)` wins over Content inside its rect, and the Waves pane still wins first.
    - mouse_click_target_is_a_pure_function_of_the_regions (extend): with `dropdown` Some, DropdownOption / DropdownOutside beat Tab; ConfigRow beats ListRow.
  </behavior>
  <action>
Implements D-02, D-04, D-05 and D-06, with I-5 to I-14. TDD: failing tests first, then the implementation.

1. **Regions.** Extend `DetailRegions` (and `reset_regions`) with:
   - `config_rows: Option<ConfigRowsRegion { list: ListRegion, rows: Vec<ConfigRowHit { underlying: usize, value_x: u16 }> }>`, with rows indexed by VISIBLE position over the whole visible list;
   - `dropdown: Option<DropdownRegion { popup: Rect, options: ListRegion }>`;
   - `scroll_panes: Vec<ScrollPaneRegion { rect, pane: ScrollPane }>`, where `ScrollPane` is `Backlog | GitCommit | DriverOutput` and is `Copy`.

   New targets:
   - `ClickTarget::ConfigRow { index, on_value }`, `DropdownOption(usize)` and `DropdownOutside`;
   - `WheelTarget::Pane(ScrollPane)`.

   `click_target` order becomes:
   1. dropdown, when Some: an option → `DropdownOption`; elsewhere inside the popup → `Nothing`; anywhere outside it → `DropdownOutside` (I-6, modal);
   2. tabs; sub-tabs; Waves rows; `FoldMarker`;
   3. `ConfigRow`, with `on_value = column >= value_x` of that row;
   4. `ListRow`; Waves pane; Content; `Nothing`.

   `wheel_target` order: Waves pane, then scroll panes, then Content (outside the strip), then `Nothing`. Both stay pure.

2. **Cfg (`render_defaults_tab`).**
   - Seed the list's `ListState` with a persisted `config_list_offset` (in visible positions) and store it back.
   - Record `config_rows`:
     - `list.rect` = the list block's inner area (below the `Borders::TOP` title row);
     - `offset` = the state's offset;
     - `len` = `visible.len()`, or 0 when only the "No config keys match" line is drawn.
   - For each visible entry, compute `value_x` as `rect.x` plus the summed widths of the spans that precede the value span in the very spans vector the row draws. Build that vector once and measure it, so the column cannot drift from the render (I-5).
   - When the chooser popup is drawn, record `dropdown`: `popup` = its rect; `options` = `ListRegion { rect: popup block inner, offset: 0, len: options.len() }`.
   - The text-prompt popup records nothing; the mouse is ignored there anyway.
   - In `click`:
     - `ConfigRow { index, on_value }` first reads whether `index` was the selected row before the click. That requires `defaults_selected == index`, `defaults_selection_visible(cache)` and `defaults_editing.is_none()`.
     - If it was selected and `on_value`: call `handle_key(Enter)` and arm `Swallow`.
     - Otherwise: set `defaults_selected = index` and arm `Replay(Enter)`.
     - `DropdownOption(j)`: if `defaults_dropdown_selected == j`, call `handle_key(Enter)` and arm `Swallow`. Otherwise set `defaults_dropdown_selected = j` and arm `Replay(Enter)`.
     - `DropdownOutside`: call `handle_key(Esc)` (the chooser-close arm) and arm `None`.
   - All persistence stays inside the unchanged Enter arm, and so does the secret no-prefill rule (T-jnf-02). No new code path touches `defaults_text_buffer`. The `config_text_entry_active` early return in `handle_mouse` stays first.

3. **Backlog.**
   - Persist `backlog_list_offset` and `record_list` for the list rect: `chunks[0]` when expanded, else `inner`. The List has no block.
   - When the pane is open, also push `ScrollPane::Backlog` with `chunks[1]`.
   - The `ListRow(i)` arm on Backlog: if `backlog_expanded`, set it false and set `backlog_scroll = 0`. Then set `backlog_selected = i` and arm `Replay(Enter)` (I-8).
   - In `wheel`:
     - `Pane(Backlog)` focuses Content and calls `handle_key(Down/Up)`;
     - Content on Backlog while `backlog_expanded` returns `ScreenAction::None`.

4. **Git.**
   - Persist `git_list_offset` and `record_list(ListRegion { rect: log_area, … })`.
   - When `has_detail`, push `ScrollPane::GitCommit` with the whole `detail_area` (message and Files).
   - Extract `fn scroll_git_commit(&self, ctx, delta: i32)` from the PageDown/PageUp `git_commit_detail.is_some()` branches. Positive delta adds, then clamps through `git_commit_viewport`. Negative delta clamps first, then subtracts. It is a no-op when there is no detail. Both arms call it with ±`PAGE_SCROLL_LINES`; the wheel calls it with ±1 (I-9, I-14).
   - The `ListRow(i)` arm: if `i != git_selected`, set it and call `close_git_commit_detail()`. Then arm `Replay(Enter)`.

5. **Queue.** Persist `queue_list_offset`. `record_list` with `rect` = the list block's inner area (the list's own block inside `inner`). The `ListRow(i)` arm sets `queue_selected = i` and arms `None` (I-10). Add a doc comment there naming the reason: Enter marks the action done and saves it.

6. **Driver (driver.rs + detail.rs).**
   - Change `render_driver_tab` to take `list_offset: &Cell<usize>` and return a small `pub(super)` struct with two fields:
     - `list: Option<ListRegion>`: the runs list block's inner area (`Borders::RIGHT`), the persisted offset and `runs.len()`;
     - `output: Option<Rect>`: `detail_area`, only when the run detail (not the dry-run preview) is drawn.
   - Update both detail.rs call sites to pass `&self.driver_list_offset`, `record_list` the list, and push `ScrollPane::DriverOutput`. Update the three driver.rs test call sites (they may drop the return value).
   - Extract `fn scroll_driver_output(&self, ctx, delta: i32)` from the Driver PageDown/PageUp arms, keeping D-19 exactly:
     - down: `driver_offset_now`, add, clamp, then `driver_follow = offset >= tail_offset(vp)`;
     - up: `driver_offset_now`, subtract, then `driver_follow = false`.
     - Both arms call it with ±`PAGE_SCROLL_LINES`; the wheel calls it with ±1.
   - The `ListRow(i)` arm on Driver calls `self.move_driver_selection(ctx, i as isize - current as isize)` when the index differs, and arms `None` (I-11).

7. **Dashboard audit (I-13).** Read `NormalScreen::handle_mouse` and `render_main` to confirm the project table is the dashboard's only keyboard-selectable widget and that dyf covers it (click, double, wheel, persisted offset). Make no code change. Record the finding in the SUMMARY.

8. Add every new Cell to `DetailScreen::new`. Render functions only record rects computed from values already in hand (D-05).
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_ config git backlog queue driver</automated>
  </verify>
  <done>
- All Task 2 `mouse_*` tests pass, along with every existing Config, secret (jnf), Git pane, Backlog, Queue and Driver follow test, unchanged or reported as an old → new mapping.
- Cfg rows are click-selectable across the filter and scroll. A value click on the selected row and a double-click open the chooser or editor, and secrets open empty. The chooser is mouse-operable, and an outside click closes it without applying.
- Backlog, Git, Queue and Driver rows are click-selectable. A Queue double-click is inert.
- The wheel scrolls the Backlog, Git commit and Driver output panes.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- Committed as a test commit and a feat commit.
  </done>
</task>

<task type="auto">
  <name>Task 3: help overlay, README and GETTING-STARTED mouse sections; full gates; tmux SGR smoke test</name>
  <files>src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md</files>
  <action>
Implements D-07 and D-08.

1. **help.rs Mouse block** (~285-297). Keep it as whole rows, with every description unique in the body. The rows become:
   - `Click`: "Mouse: switch tab / sub-tab; select a row on any tab and focus its pane";
   - a new `Click value` row: "Mouse: Config tab: on the selected row, the same as Enter";
   - a new fold-marker row, keyed `Click \u{25B8}/\u{25BE}` (escapes, per the house rule): "Mouse: Roadmap: fold / unfold that milestone band";
   - `Double-click`: "Mouse: open the row, the same as Enter (Space on a Roadmap band)";
   - `Wheel`: "Mouse: move the selection, or scroll the pane, under the pointer";
   - `M` and `Shift+drag` unchanged.

   Update `mouse_help_block_is_documented_as_whole_rows` to pin the new rows in both flag states. Record old → new in the SUMMARY.

2. **README.md Mouse section** (~286-301).
   - Rewrite the Click / Double-click / Wheel rows for the per-tab behaviour:
     - rows on every tab list;
     - the Roadmap fold marker, and band double-click = fold;
     - the Config value click and the chooser rules;
     - Docs folder/file double-click;
     - the Git / Backlog / Driver pane wheel.
   - Add a short "Not mouse-driven" paragraph that condenses the plan's "Intentionally left out" list. It must at least cover the Queue double-click (reason: Enter marks the action done), the Config scope title, breadcrumbs, the Roadmap box view, and dialogs/help/text entry.
   - Keep the `M` and Shift+drag rows and the `preferences.mouse` sentence.

3. **docs/GETTING-STARTED.md** (~156-158). Update the mouse paragraph so it matches: click any row, double-click to open (fold on a Roadmap band), wheel over a pane to scroll it, and Queue double-click does nothing.

4. **Full gates.** Run each and record the result in the SUMMARY:
   - `rtk proxy cargo test --no-fail-fast`, redirected to a scratch file and summed from its `test result:` lines. Expected: 56 suites; the baseline passed count plus the new `mouse_*` tests; 1 failed (the git-version witness only); 15 ignored.
   - `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.
   - `rtk proxy cargo clippy -- -D warnings`: exit 0.
   - The no-render-I/O check in the verification section below.

5. **tmux smoke** (D-08, if tmux is available; otherwise record why not). Use a private socket, `tmux -L kes`, at 120x36 running `bash --norc`, so the user's tmux server is never touched. Build with `cargo build`.
   - Make a scratch GSD project:
     - copy this repo's `.planning/` into `<scratch>/proj/.planning`;
     - run `git init` there, one commit, and about 40 more commits with `git commit --allow-empty` so the Git list scrolls;
     - register only it: `target/debug/gsd-meta-manager --config <scratch>/config.json add <scratch>/proj proj`;
     - launch with `--config <scratch>/config.json`.
   - Never click inside any project the startup session scan auto-registers (dyf E-8).
   - Inject SGR sequences with `tmux -L kes send-keys -t kes -l`, using 1-based coordinates read off `capture-pane -p`:
     - left press: ESC `[<0;COL;ROWM`;
     - wheel down: ESC `[<65;COL;ROWM`;
     - wheel up: ESC `[<64;COL;ROWM`;
     - double-click: two presses in one `send-keys` call.
   - Readings to capture (capture-pane before and after each):
     - (a) Roadmap: a phase-row click moves the selection and the detail pane's name line; a click on a band's ▸/▾ flips the glyph; a band double-click flips it back; a wheel over the detail pane moves the selection.
     - (b) Cfg: record `sha256sum` of `<scratch>/proj/.planning/config.json` first. Click a key cell to move the highlight. Click that row's value: the chooser popup appears. Click outside it: the popup is gone. Re-check `sha256sum`: unchanged.
     - (c) Docs Files: a folder double-click changes the breadcrumb; a `.md` double-click opens the view; a wheel-down advances the gutter line numbers.
     - (d) Git: after scrolling, a click selects the row under the pointer; a double-click opens the Commit pane; a wheel over it scrolls the message.
     - (e) Queue: add one action with `a`, then double-click it; it is still listed.
   - Quit with `q` and kill the tmux server with `tmux -L kes kill-server`. Put the readings in a table in the SUMMARY, like dyf's.

6. **SUMMARY.** Write it with:
   - commits;
   - measured gates;
   - the new test list;
   - the final per-tab mouse map;
   - the dashboard audit;
   - the Inferred decisions I-1..I-17 plus any E-n;
   - the "Intentionally left out" list;
   - old → new test mappings;
   - the tmux readings;
   - Self-Check.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_help mouse_ help</automated>
  </verify>
  <done>
- The help overlay, README and GETTING-STARTED describe the new mouse map and the left-out list.
- The full suite fails only the git-version witness.
- Both clippy runs exit 0.
- The no-render-I/O check is clean.
- The tmux readings (or the documented infeasibility) are in the SUMMARY.
- Committed as a docs commit and the SUMMARY commit.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| terminal → App (mouse) | Synthetic or accidental mouse events from the terminal (any process that can write to the tty, or a user's stray double-click) now reach selection and Enter paths on more tabs |
| TUI → `.planning/config.json` / `~/.gsd/defaults.json` / the queue file | The Cfg Enter arm and the Queue Enter arm persist to disk |
| TUI → subprocess | Git Enter spawns `git show <hash>` (argv, no shell); Browse/Archive Enter read files in the handler |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-kes-01 | Tampering | Cfg value-click / chooser click (detail.rs `click` ConfigRow / DropdownOption) | medium | mitigate | A value click runs Enter only on the row that was already selected and visible before the click. A chooser option applies only once it is already highlighted. Every other first click only selects. `Swallow` stops the completing double press from running Enter a second time (tested on an integer row). Text prompts still ignore the mouse (`config_text_entry_active` first) |
| T-kes-02 | Tampering | Queue double-click (ListRow arm on Queue) | high | mitigate | Queue rows arm nothing, so a double-click can never reach the Enter arm that removes the action and saves the queue. `mouse_queue_click_selects_and_double_click_never_marks_done` asserts that `queued_actions` is unchanged and that no status message is set (I-10) |
| T-kes-03 | Information disclosure | Secret config rows reached by the mouse | high | mitigate | The mouse only calls the unchanged Enter arm, which opens a Secret prompt with an empty buffer (T-jnf-02). No new render of values. A test asserts an empty buffer and no fragment of the stored key in the rendered tab |
| T-kes-04 | Tampering | Stale or mis-offset regions mapping a click to the wrong item, followed by a double-click acting on it (Sessions resume, Cfg integer step, Git load) | medium | mitigate | Both render paths reset regions every frame (`reset_regions`). The offsets are the same persisted `ListState` offsets the widgets drew with. Cfg maps visible → underlying through the recorded `rows`. The double-click replays on the first click's selection and never re-hit-tests (dyf I-7). Scrolled-list tests exist per tab |
| T-kes-05 | Spoofing | Roadmap fold-marker and Cfg value-column rects | low | mitigate | Both are measured from the spans actually drawn, never from constants, and pinned against the rendered buffer (`mouse_roadmap_view_records_the_list_body_and_fold_glyph_cells`, `mouse_config_rows_value_column_matches_the_rendered_value`) |
| T-kes-06 | Elevation of privilege | Git double-click → `git show <hash>` | low | accept | The existing Enter path is reused verbatim. The hash is a raw argv element (no shell) and is taken from the loaded log entry. The mouse adds no new input to it |
| T-kes-07 | Denial of service | Wheel floods into scroll helpers | low | accept | dyf's reader filter (T-dyf-01) already limits the FIFO to presses and vertical wheel steps. Each step is O(1) clamp arithmetic on in-memory state |
| T-kes-SC | Tampering | npm/pip/cargo installs | low | accept | No new crates, no package-manager install in this plan |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: run into a scratch file and sum its `test result:` lines. 56 suites; passed = baseline + the new `mouse_*` count; failed = 1 (only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`); 15 ignored.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0, and `rtk proxy cargo clippy -- -D warnings` exits 0.
- `rtk proxy cargo test --no-fail-fast -- mouse_` shows every dyf and kes mouse test passing.
- No render I/O added. Over the non-test hunks of `git diff <task-1 base>..HEAD -- src/ui`, the added lines contain none of `std::fs`, `read_to_string`, `read_dir`, `File::open`, `.exists()`, `is_file`, `is_dir`. Test-module fixtures (tempdirs) are exempt and must be named in the SUMMARY if they match.
- The tmux SGR smoke readings (a)-(e) are recorded, including the unchanged `config.json` sha256 and the Queue item still present, or the reason tmux was infeasible.
</verification>

<success_criteria>
- D-01..D-08 are each proven by named tests and/or tmux readings in the SUMMARY's D → proof table.
- Every keyboard-selectable list in the detail view (Roadmap, Phases, Backlog, Git, Queue, Sessions, Agents, Cfg, Docs Files, Docs Milestones, and the Driver runs with the flag on) is click-selectable through its scroll offset. The dashboard audit is recorded.
- A double-click equals Enter everywhere except the documented Roadmap-band Space rule (I-2) and the Queue exclusion (I-10).
- The wheel scrolls the pane under the pointer: lists move their selection; the Backlog, Git commit, Driver output and Docs file panes scroll.
- The "Intentionally left out" list appears in the SUMMARY and in condensed form in the README.
- No new crates. No file I/O in render. Keyboard behaviour is unchanged apart from standard scrolling from the persisted offsets, and any resulting expectation change is reported as old → new.
</success_criteria>

<output>
Create `.planning/quick/260926-kes-mouse-row-interaction-on-remaining-detai/260926-kes-SUMMARY.md` when done. The quick workflow adds the STATE.md quick-task row.
</output>
