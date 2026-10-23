---
phase: quick-261022-wtw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/help.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/roadmap_widget.rs        # deleted
  - src/ui/mod.rs
  - src/state_reader/mod.rs
  - src/state_reader/state_md.rs
  - tests/state_reader_test.rs
  - README.md
  - docs/ARCHITECTURE.md
  - docs/DEVELOPMENT.md
  - .planning/codebase/ARCHITECTURE.md
  - .planning/codebase/STRUCTURE.md
files_deleted:
  - src/ui/roadmap_widget.rs
autonomous: true
requirements: [QUICK-261022-wtw]

estimate:
  tokens: 150000
  raw_tokens: 150000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "On the Detail screen's Roadmap tab, `v` does nothing. The key returns ScreenAction::None, the tab stays RoadmapViz, and a second render is byte-identical to the first (test `roadmap_tab_draws_the_list_and_v_is_unbound`). No other tab binds `v` either."
    - "The Roadmap tab always draws the lanes list: `render_roadmap` renders `roadmap_view::RoadmapView` with no branch. `src/ui/roadmap_widget.rs` is gone, `src/ui/mod.rs` no longer declares it, and `ProjectViewCache` has no view-mode flag."
    - "The Roadmap list keys (j/k, Down/Up, PgUp/PgDn, g/G, h/l, [ ], Space, Enter) behave exactly as before; the 24-05 Roadmap tests pass unchanged. The four scroll-key matches in `DetailScreen::handle_key` list every DetailSubView variant and have no wildcard arm. `DetailScreen` no longer has `scroll_offset` or `generic_viewport`."
    - "The help popup, the Roadmap footer, the README key table and its mouse paragraph no longer mention `v` or the box view. The four docs trees that listed the deleted file list `roadmap_view.rs` in its place."
    - "The escape probe (`the_screen_renders_identity_escaped` and the two-way arrival-set check) passes without the box-view sub-state and arrival row. The ratatui C1-drop measurement survives as its own test, `ratatui_drops_a_c1_grapheme_before_a_cell_exists`."
    - "Gate: `cargo test --no-fail-fast` gives passed = the Task 1 baseline minus 9 and ignored = baseline. The only failure is envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against. `cargo clippy --all-targets -- -D warnings` exits 0, and so does `cargo fmt --check`."
  artifacts:
    - path: "src/ui/screens/detail.rs"
      provides: "Roadmap tab with the lanes list only; `v` unbound; exhaustive scroll-key matches"
      contains: "fn roadmap_tab_draws_the_list_and_v_is_unbound"
    - path: "src/ui/mod.rs"
      provides: "ratatui C1-drop dependency pin, moved out of the deleted widget spot-check"
      contains: "fn ratatui_drops_a_c1_grapheme_before_a_cell_exists"
    - path: "src/ui/screens/render_escape_guard.rs"
      provides: "escape probe without the box-view sub-state or arrival row"
    - path: "README.md"
      provides: "Roadmap key table and mouse paragraph without the box view"
  key_links:
    - from: "src/ui/screens/detail.rs render_roadmap"
      to: "src/ui/roadmap_view.rs RoadmapView"
      via: "unconditional render_stateful_widget + roadmap_list_offset / roadmap_list_viewport / record_list / record_fold_marks"
      pattern: "roadmap_view::RoadmapView"
    - from: "src/ui/screens/detail.rs handle_key RoadmapViz arms"
      to: "DetailScreen::roadmap_nav / roadmap_activate"
      via: "unguarded `DetailSubView::RoadmapViz =>` arms in j/k/PgDn/PgUp and Enter/Space; `on_roadmap` guard on g/G/h/l/[/]"
      pattern: "DetailSubView::RoadmapViz =>"
    - from: "render_escape_guard.rs DETAIL_SUB_STATES"
      to: "render_escape_guard.rs DETAIL_TAB_ARRIVAL"
      via: "two-way set equality: both box-view entries are removed together"
      pattern: "RoadmapViz tab, cursor on the planned phase"
---

<objective>
Remove the Detail screen's Roadmap box view, which is the `v` toggle on the Roadmap tab. The Roadmap
tab's lanes list made it obsolete. Remove every piece of code that only it needed: the render
branch, the widget module, the view-mode flag, the toggle key, the generic-scroll machinery that
only it reached, the footer/help/README text, the escape-probe state and its tests. Anything another
view still uses stays.

Purpose: one Roadmap rendering path, no dead state, and no docs advertising a key that no longer does anything.
Output: an edited `src/ui/screens/*`, a deleted `src/ui/roadmap_widget.rs`, trimmed state_reader and
integration tests, and updated README/docs. Three atomic commits.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

Decisions made while planning. The human was unavailable, so each one is marked for later audit:

- I-1 `[INFERRED — audit]`: in `DetailScreen::handle_key`, the generic `_ =>` scroll arms of the
  j/Down, k/Up, PageDown and PageUp matches are reached ONLY by the box view. The same goes for the
  `generic_viewport` Cell they read and for `DetailScreen.scroll_offset`. All four go. Evidence
  gathered at planning time:
  - Each of the four matches already has an explicit arm for all 11 `DetailSubView` variants
    (RoadmapViz, Pipeline, Backlog, GitHistory, Queue, Sessions, Agents, Archive, Defaults, Browse,
    Driver).
  - Browse and Archive scroll their file views in their own arms, with `browser_viewport` /
    `archive_viewport` and their own cache offsets. They never reach `_ =>`.
  - `generic_viewport` is written only inside the box branch of `render_roadmap`.
  - The test-block header above `generic_fixture` says the box view is the only state that
    reaches the fallback.
  - The orchestrator's caution that Browse/Archive use the fallback does not hold for these
    matches. The compiler's exhaustiveness check is the proof gate (Task 1 step 2g).
  - The Enter/Space match's `_ => ScreenAction::None` arm DOES serve other views and stays.
- I-2 `[INFERRED — audit]`: `src/ui/roadmap_widget.rs` is deleted entirely. Its only production
  caller is the box branch.
- I-3 `[INFERRED — audit]`: `PhaseMarker::glyph` is deleted. Its only production caller was the
  widget, and no Detail-screen legend draws `+ * o` any more. `PhaseMarker` itself and
  `PhaseMarker::decide` stay, because `roadmap_model_for` uses them.
- I-4 `[INFERRED — audit]`: the two integration tests that render the widget are deleted. The
  state-level facts they cover are already pinned by four tests in `tests/state_reader_test.rs`:
  `each_phase_marker_follows_that_phases_own_disk_status`,
  `a_decimal_phase_written_padded_and_unpadded_is_one_row_with_its_disk_data`,
  `a_decimal_current_phase_is_the_active_phase_not_the_count_fallback` and
  `plan_counts_fall_back_to_disk_when_the_roadmap_lists_no_plans`.
- I-5 `[INFERRED — audit]`: the widget spot-check in `src/ui/mod.rs` contains this tree's only
  executable pin of a ratatui behaviour: control graphemes are dropped before a cell exists. The
  census doc relies on that behaviour, which has nothing to do with the box view, so the measurement
  moves to its own test.
- I-6 `[INFERRED — audit]`: `v` is left unbound. `other_roadmap_keys_end_the_edge_walk` keeps `v`
  in its key list, because the edge-walk reset at the top of `handle_key` fires for any key other
  than h/l, bound or not. A replacement test pins `v` as a no-op. The footer test is flipped to
  assert that `[v]` is absent.
- I-7 `[INFERRED — audit]`: some historical provenance comments cite the deleted module. They are
  reworded so they no longer name its path:
  - detail.rs ~264, ~8659 and ~21308
  - screens/mod.rs ~2738
  - render_escape_guard.rs ~4106
  - state_md.rs ~647

  The census doc in `src/ui/mod.rs` (~482, the line starting `assertion` that carries
  `Sites: [`) is byte-verbatim recorded evidence. Its own doc says so, and it stays untouched.
- I-8 `[INFERRED — audit]`: four doc trees list the deleted file: docs/ARCHITECTURE.md,
  docs/DEVELOPMENT.md, .planning/codebase/ARCHITECTURE.md and .planning/codebase/STRUCTURE.md. Each
  gets a one-line swap to `roadmap_view.rs`, with no new rows. No `*UI-SPEC.md` mentions the box
  view (grep at planning time), so no UI-SPEC is edited. Historical artifacts stay untouched:
  `.planning/phases/**`, `.planning/quick/**`, `.planning/milestones/**`, `.planning/debug/**` and
  the 260923-md1 row of STATE.md's quick-task table.
- Config: no config option exists for the box view (grep of src/config.rs, src/app.rs at planning time).

Expected test delta, which is −9 `#[test]` fns:
- `src/ui/roadmap_widget.rs`: −2 (its whole tests module)
- `tests/state_reader_test.rs`: −2
- detail.rs: −4
  - deleted: `test_generic_page_up_clamps_stale_offset`, `test_generic_page_down_clamps_at_content_end`,
    `roadmap_box_view_keeps_generic_scroll`, `roadmap_graph_tab_v_is_inert_on_other_tabs`
  - one replaced (net 0) and one renamed (net 0)
- help.rs: −1
- `src/ui/mod.rs`: net 0 (one test removed, one added)

Commit convention: one commit per task, with messages `refactor(quick-261022-wtw): ...` /
`docs(quick-261022-wtw): ...`. Every commit message MUST end with the trailer line
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

rtk note: wrap whole pipelines in `rtk proxy sh -c '...'`. rtk filters any grep/awk that sits
downstream of a bare `rtk proxy`.
</context>

<!-- planner-discipline-allow: roadmap_widget -->
<!-- planner-discipline-allow: fn glyph(self) -->
<!-- planner-discipline-allow: KeyCode::Char('v') if -->
<!-- planner-discipline-allow: On the list view -->
<!-- planner-discipline-allow: box view|box list -->
<!-- planner-discipline-allow: roadmap_widget|RoadmapWidget -->
<!-- planner-discipline-allow: box_view|box view|box list|generic_viewport|roadmap_list_active -->
<!-- planner-discipline-allow: pub scroll_offset: u16|scroll_offset: &mut u16 -->

<tasks>

<task type="tracer">
  <name>Task 1: Detail screen end-to-end: the Roadmap tab renders only the lanes list and `v` is unbound</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/mod.rs, src/ui/screens/help.rs, src/ui/screens/render_escape_guard.rs</files>
  <read_first>
    - src/ui/screens/detail.rs: read only the regions named below. The file is about 33k lines, so locate each one by grep for the named identifier, not by its line number.
    - src/ui/screens/render_escape_guard.rs ~1700-1760 and ~1930-2000
  </read_first>
  <action>
**Step 0: baseline.** Before any edit, measure the full suite once. Run
`rtk proxy sh -c 'cargo test --no-fail-fast > target/wtw-base.log 2>&1; grep -E "^test result:" target/wtw-base.log | awk "{p+=\$4; f+=\$6; i+=\$8} END {print \"passed=\" p \" failed=\" f \" ignored=\" i}"; grep -E "^test .* FAILED$" target/wtw-base.log'`.
Record BASE_PASSED, BASE_FAILED and BASE_IGNORED, plus the failing names, in the SUMMARY. The
expected sole failure is the envelope::policy git-version constants test.

**Step 1: `src/ui/screens/mod.rs`.**
- In `ProjectViewCache`, delete the Roadmap view-mode bool field (`roadmap_box_view`) together
  with its 3-line doc. It sits just above `roadmap_cursor`.
- In the IN-02 test doc near ~2738, cut the trailing clause that cites the deleted widget's width
  arithmetic, so the sentence ends "matching `section_rule` here."

**Step 2: `src/ui/screens/detail.rs`, production code (I-1, I-6).**

a. Remove the import `use crate::ui::roadmap_widget::RoadmapWidget;` (~line 21).

b. `pub struct DetailScreen` (~799):
- Delete the `pub scroll_offset: u16` field.
- Delete the `generic_viewport: Cell<ViewportMetrics>` field and its doc. That doc wrongly claims
  the Phases tab reaches the fallback.
- In `DetailScreen::new` (~1206), delete both initializers.

c. Delete `fn roadmap_list_active` and its doc (~1594-1601).

d. `fn content_at_first_row` (~2213):
- The `DetailSubView::RoadmapViz` branch keeps only the list logic, which is the
  `match self.roadmap_model_and_cursor(ctx)` returning `true` for None and
  `first_target().is_none_or(...)` otherwise. Drop the if/else and the else's
  `generic_viewport`/`clamp_scroll` read.
- In its doc bullet "a scroll view (...)", drop the Roadmap entry so the bullet names only a
  Browse or Archive file view.

e. `fn switch_to_tab` (~3305) and `fn switch_to_sub_view` (~3324):
- Remove the `scroll_offset: &mut u16` parameter and the `*scroll_offset = 0;` statement.
- Change the doc phrase "(scroll reset plus each tab's data loading)" to "(each tab's data loading)".
- Update every caller. Each one passes `&mut self.scroll_offset`, or `&mut screen.scroll_offset`
  in `opened_on`; grep finds about 15 of them.
- Delete the now-meaningless `self.scroll_offset = 0;` statements in the three `q` / `Esc` Pop
  arms. They are around line ~2104 (the Waves-pane focus handler), ~3651 and ~3701.

f. In `handle_key` (the `let current_view = effective_sub_view(...)` block, ~3572), replace the
`let roadmap_list = ... && self.roadmap_list_active(ctx);` binding and its comment with
`let on_roadmap = current_view == DetailSubView::RoadmapViz;` and the comment
"The Roadmap tab's cursor keys (D-A10)."

g. The four scroll matches inside `handle_key` sit in these arms:
- `KeyCode::Char('j') | KeyCode::Down` (~3833; this is the main handler, not the Waves-pane one at ~2109)
- `KeyCode::Char('k') | KeyCode::Up` (~4013)
- `KeyCode::PageDown` (~4153)
- `KeyCode::PageUp` (~4346)

In each one:
- Change `DetailSubView::RoadmapViz if roadmap_list =>` to `DetailSubView::RoadmapViz =>`.
- Delete the trailing `_ => { ... }` generic-scroll arm. It carries one of two comments: "add the
  delta, then clamp", or "clamp FIRST, subtract second".
- In the j arm, delete the two-line comment above the RoadmapViz arm, which says the other view
  "falls through to the generic scroll below".

PROOF GATE: these matches name all 11 variants, so they must compile with no wildcard. If rustc
reports E0004 (non-exhaustive patterns) on any of them, some view still reaches the fallback. In
that case:
- restore that one arm, plus the field(s) it reads;
- give the arm a comment naming the view(s) that reach it;
- record the deviation in the SUMMARY.

Do not invent a new arm.

h. The Enter/Space match (~5042): change `DetailSubView::RoadmapViz if roadmap_list =>` to
`DetailSubView::RoadmapViz =>`. KEEP `_ => ScreenAction::None`, because the other views use it.

i. Delete the toggle arm `KeyCode::Char('v') if current_view == DetailSubView::RoadmapViz => { ... }`
(~6033-6041) and the 2-line comment above it. Do not bind `v` anywhere.

In the comment above the g/G/h/l/[/] arms (~6042-6047), drop the phrase "live on the list view
only — the box view leaves them unbound, as they were before". Keep the rest. Change those six
guards from `if roadmap_list` to `if on_roadmap`.

j. The mouse arm `ClickTarget::FoldMarker(row)` (~6393): its condition becomes
`*current_view != DetailSubView::RoadmapViz`.

k. `fn render_roadmap` (~6741):
- Delete the `box_view` local.
- Delete the whole `if box_view { ... }` branch: `active_phase_number`, `phase_block_h`, the
  `generic_viewport.set`, `max_scroll`/`clamped_offset` and the widget render.
- Un-nest the former `else` body so it runs unconditionally. That body is the
  `roadmap_view::RoadmapView` `render_stateful_widget`, the `roadmap_list_offset` /
  `roadmap_list_viewport` writes, `record_list` and `record_fold_marks`. Keep its comments.

l. In `fn footer_spans`, RoadmapViz arm (~11209): delete the `[v]` span and the `iew  ` span. Keep
`[h/l]`, `[Space]` and `[e]`.

m. Reword the provenance comments that name the deleted module (I-7):
- The doc above the session-id truncation helper (~264): "the `&s[..n]` panic round 8 fixed in
  ..." becomes "fixed in the Roadmap boxes widget (deleted by quick 261022-wtw)".
- The IN-02 popup-width comment (~8659): end it at "matches `section_rule` here."
- The T-21-25-05 test-block comment (~21308): use the same "Roadmap boxes widget (deleted by quick
  261022-wtw)" wording.

New comments must not contain any of these literals: the view-mode field name, the phrases
"box view" or "box list", the removed Cell or fn names, or the deleted module's path. The Task
gates grep for them.

**Step 3: `src/ui/screens/detail.rs`, tests (`mod tests`).**
- Delete the whole "CD-03 / IN-07 closure: the generic `_ =>` fallback" block. That is its comment
  header (~21223-21229), `fn generic_fixture`, `test_generic_page_up_clamps_stale_offset` and
  `test_generic_page_down_clamps_at_content_end`.
- Delete `fn roadmap_box_flag`, `roadmap_graph_tab_v_is_inert_on_other_tabs` and
  `roadmap_box_view_keeps_generic_scroll`.
- Replace `roadmap_graph_tab_draws_the_graph_by_default_and_v_toggles_the_box_list` with
  `roadmap_tab_draws_the_list_and_v_is_unbound`. It uses the same `roadmap_graph_fixture()`:
  1. Render once with `render_detail_to_text`.
  2. Keep the four list assertions: contains "┌ Roadmap ", contains "Start now:", no "─►", and
     `roadmap_list_viewport.get() > 0`.
  3. Capture `ctx.view_cache.get(TEST_ALIAS).and_then(|c| c.roadmap_cursor.clone())`.
  4. Call `screen.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &mut ctx)`.
  5. Assert all of the following:
     - it returned `ScreenAction::None`;
     - `stored_view(&ctx) == DetailSubView::RoadmapViz`;
     - the stored cursor is unchanged;
     - a second `render_detail_to_text` equals the first render byte-for-byte;
     - neither render contains "P1: Name 1".
- Rename `roadmap_graph_tab_footer_advertises_v` to `roadmap_footer_does_not_advertise_v`. Its
  assertions become `!footer.contains("[v]")` and `footer.contains("[e]")`.
- `roadmap_footer_advertises_edges_and_fold`: remove `"[v]"` from its hint array.
- `other_roadmap_keys_end_the_edge_walk`: KEEP `KeyCode::Char('v')` in its key list (I-6) and
  delete the `cache.roadmap_box_view = false;` line.
- `roadmap_keys_are_inert_without_project_state`: delete the `screen.scroll_offset` assertion.
- `switching_to_the_backlog_tab_returns_no_screen_action`: drop the `scroll_offset` local, its
  `&mut` argument and the "the tab switch resets the scroll offset" assertion.
- Any other test the compiler flags for the removed field, parameter or helper: delete only the
  part about the box view or the scroll offset.

**Step 4: `src/ui/screens/help.rs`.**
- In `help_lines`, delete the Roadmap `v` row (~254).
- Delete the test `the_roadmap_graph_toggle_key_is_documented` (~1208-1220).

**Step 5: `src/ui/screens/render_escape_guard.rs`.** The arrival check is a two-way set equality,
so remove BOTH entries named "RoadmapViz tab, box view" together:
- the `DETAIL_TAB_ARRIVAL` row (~1736-1741);
- the `DETAIL_SUB_STATES` entry and its 3-line comment (~1981-1989).

Then fix two docs:
- In the `DETAIL_SUB_STATES` doc bullet (~1944-1948), drop "box view /" and the clause about the
  flag selecting the legacy list. The bullet keeps the planned-phase cursor and the synthetic band.
- In the EnqueueScreen spot-check doc (~4106), say "the since-deleted Roadmap boxes widget's phase
  name" in place of the file name.

**Step 6:** run `cargo fmt`, then commit `refactor(quick-261022-wtw): remove the Roadmap box view from the Detail screen`,
ending the message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

HEAD baselines, from the planning-time grep:
- 50 hits for the view/Cell/fn regex in src/ui/screens
- 11 hits for the module regex in src/ui/screens
- 1 guarded `v` arm
- 3 lines matching the scroll_offset field/param regex

All four must reach 0.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast ui::screens && test "$(grep -rniE 'box_view|box view|box list|generic_viewport|roadmap_list_active' src/ui/screens | wc -l)" -eq 0 && test "$(grep -rnE 'roadmap_widget|RoadmapWidget' src/ui/screens | wc -l)" -eq 0 && test "$(grep -c "KeyCode::Char('v') if" src/ui/screens/detail.rs)" -eq 0 && test "$(grep -cE 'pub scroll_offset: u16|scroll_offset: &mut u16' src/ui/screens/detail.rs)" -eq 0 && test "$(grep -c 'fn roadmap_tab_draws_the_list_and_v_is_unbound' src/ui/screens/detail.rs)" -eq 1</automated>
  </verify>
  <done>
- Every `ui::screens` lib test passes, including the escape probe, the arrival-set check, the 24-05
  Roadmap key tests and the new `roadmap_tab_draws_the_list_and_v_is_unbound`.
- The four scroll matches have no wildcard arm. If the proof gate ever fired, the arm is restored
  and recorded instead.
- The five grep gates read 0 / 0 / 0 / 0 / 1.
- Commit 1 has landed with the required trailer.
  </done>
</task>

<task type="auto">
  <name>Task 2: Delete the widget module and the code and tests only it used</name>
  <files>src/ui/roadmap_widget.rs, src/ui/mod.rs, src/state_reader/mod.rs, src/state_reader/state_md.rs, tests/state_reader_test.rs</files>
  <read_first>
    - src/ui/mod.rs ~800-935 (the widget spot-check test and the doc of the queue-delete test after it)
    - src/state_reader/mod.rs ~455-500 (the `PhaseMarker` doc, variants and `glyph`)
    - tests/state_reader_test.rs ~1425-1466, ~1684-1740, ~2091-2126
  </read_first>
  <action>
1. Run `git rm src/ui/roadmap_widget.rs` (I-2). Its two `#[cfg(test)]` tests go with it. Remove
   `pub mod roadmap_widget;` from the top of `src/ui/mod.rs`.

2. In the `src/ui/mod.rs` tests module (I-5), delete
   `the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently` (doc and
   body). First move its dependency measurement into a new `#[test] fn
   ratatui_drops_a_c1_grapheme_before_a_cell_exists()` placed where the deleted test was. Move it
   verbatim:
   - the "The dependency behaviour, MEASURED here" comment;
   - the `with_c1` / `without_c1` Buffer setup;
   - the `assert_eq!` with its "ratatui is expected to drop the C1 grapheme outright" message.

   Give the new test a two-to-three-line doc. It should say the test pins the ratatui behaviour
   that the census doc's "What this census does NOT claim" section rests on, and that it was moved
   out of the deleted widget spot-check.

   In the doc of the next test, `the_queue_delete_prompt_is_unchanged_for_a_clean_command_and_changed_for_a_control_one`,
   change "the second of the eight files spot-checked" to "one of the eight files converted".

   Leave the recorded census output (~482, the `assertion ... Sites: [` line) byte-identical (I-7).

3. `src/state_reader/mod.rs` (I-3):
   - Delete `PhaseMarker::glyph` (its doc and `pub fn glyph(self)`). Keep `impl PhaseMarker` with
     `decide`.
   - In the variant docs, drop the "Rendered `+`." / "Rendered `*`." / "Rendered `o`." sentences.
   - In the enum doc, rewrite the opening "**One decision, two call sites** (…the deleted widget
     link… and the Detail screen's Phases list)" so it names today's callers:
     `ProjectState::phase_marker` for a project's own phases and a direct `PhaseMarker::decide` for
     workstream phases, both inside `roadmap_model_for` in `src/ui/screens/detail.rs`.
   - Keep the history paragraph, including picsync, that explains why both inputs moved to disk.
   - Leave no intra-doc link to the deleted module.

4. In `src/state_reader/state_md.rs` (~647, the `yaml_double_quoted` doc), change the parenthetical
   to "(`untrusted::bounded`, the since-deleted Roadmap boxes widget)".

5. `tests/state_reader_test.rs` (I-4):
   - Delete `the_rendered_roadmap_draws_the_current_glyph_on_the_frontier_not_behind_it` (doc and
     body).
   - Delete the `rendered_glyph_column` helper and its doc.
   - Delete `the_rendered_roadmap_marks_the_decimal_phase_current_once` (doc and body).
   - In `a_phase_complete_on_disk_but_unchecked_in_the_roadmap_reads_done`, delete the final
     `assert_eq!(phase3_glyph(&state), "+");`, then delete the `phase3_glyph` helper. The
     `PhaseMarker::Done` assertion directly above already pins that fact.
   - Remove any `use` that becomes unused.

6. Sanity: `production_ui_code_draws_only_registry_or_exempt_glyphs` in `src/ui/glyphs.rs` asserts it
   reads at least 25 files. After the deletion it reads 26: 24 `.rs` under src/ui, plus 3 extras,
   minus render_escape_guard. No edit is needed there.

7. Run `cargo fmt`, then commit `refactor(quick-261022-wtw): delete the Roadmap boxes widget and its only dependents`,
   ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

HEAD baselines from the planning-time grep:
- 28 hits for the module regex over src and tests, excluding the `Sites: [` line. Target 0.
- 1 `fn glyph(self)` in state_reader. Target 0.
- 0 C1 test. Target 1.
  </action>
  <verify>
    <automated>test ! -e src/ui/roadmap_widget.rs && test "$(grep -rnE 'roadmap_widget|RoadmapWidget' src tests | grep -v 'Sites: \[' | wc -l)" -eq 0 && test "$(grep -c 'fn glyph(self)' src/state_reader/mod.rs)" -eq 0 && test "$(grep -c 'fn ratatui_drops_a_c1_grapheme_before_a_cell_exists' src/ui/mod.rs)" -eq 1 && rtk proxy cargo test --lib --no-fail-fast ui:: && rtk proxy cargo test --lib --no-fail-fast state_reader:: && rtk proxy cargo test --no-fail-fast --test state_reader_test</automated>
  </verify>
  <done>
- The widget file is gone and `src/ui/mod.rs` does not declare it.
- `PhaseMarker::glyph` is gone.
- The `ui::` lib tests pass, including the census tests, the new C1 pin and the glyph source scan.
  So do the `state_reader::` lib tests and the `state_reader_test` integration suite.
- The four gates read as listed.
- Commit 2 has landed with the required trailer.
  </done>
</task>

<task type="auto">
  <name>Task 3: README and docs without the box view, then the full release-style gate</name>
  <files>README.md, docs/ARCHITECTURE.md, docs/DEVELOPMENT.md, .planning/codebase/ARCHITECTURE.md, .planning/codebase/STRUCTURE.md</files>
  <read_first>
    - README.md ~268-282 and ~328-340
  </read_first>
  <action>
1. README.md, "Tab-specific keys" table:
   - Delete the row whose key cell is `v` and which describes switching between the graph list and
     the box view (~278).
   - In the Roadmap `j k g G PgUp PgDn` row (~273), drop the leading "On the list view: " and
     capitalise the next word ("Move the phase cursor, ...").

2. README.md, the "Not mouse-driven, on purpose" paragraph (~330-338). Replace the clause from
   "the Roadmap box view (`v`) has no cursor, and the phase names in its detail pane's" through
   "(`h` / `l` follow them)" with "the phase names in the Roadmap detail pane's Needs / Unblocks /
   Parallel lines are not links (`h` / `l` follow them)". Keep the rest of the paragraph, and keep
   its line wrapping near the original width.

3. One-line swaps of the deleted file's tree entry (I-8). Keep each file's column and comment style
   and add no new rows:
   - docs/ARCHITECTURE.md (~167): "roadmap_view.rs    Roadmap tab widget: lanes list and detail pane".
   - docs/DEVELOPMENT.md (~180): the bare filename `roadmap_view.rs`.
   - .planning/codebase/STRUCTURE.md (~61): `roadmap_view.rs` with the comment "#  Roadmap tab lanes widget".
   - .planning/codebase/ARCHITECTURE.md (~112): in the Presentation Layer "Location:" list, change
     the deleted path to `src/ui/roadmap_view.rs`.

4. Do NOT touch:
   - historical artifacts under `.planning/phases/`, `.planning/quick/`, `.planning/milestones/` or
     `.planning/debug/`;
   - STATE.md's 260923-md1 quick-task row;
   - any `*UI-SPEC.md` (none mentions the box view).

5. Final gate, all from the repo root:
   - `cargo fmt --check`, which must exit 0.
   - `rtk proxy cargo clippy --all-targets -- -D warnings`, which must exit 0. This is the
     no-dead-code / no-unused-import proof.
   - The full suite, with
     `rtk proxy sh -c 'cargo test --no-fail-fast > target/wtw-final.log 2>&1; grep -E "^test result:" target/wtw-final.log | awk "{p+=\$4; f+=\$6; i+=\$8} END {print \"passed=\" p \" failed=\" f \" ignored=\" i}"; grep -E "^test .* FAILED$" target/wtw-final.log'`.

   Required results:
   - passed == BASE_PASSED − 9;
   - ignored == BASE_IGNORED;
   - the only FAILED line is
     envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against.

   A different delta must be explained test-by-test in the SUMMARY.

6. Commit `docs(quick-261022-wtw): drop the Roadmap box view from README and docs trees`, ending
   with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

HEAD baselines from the planning-time grep:
- 2 case-insensitive box-view/box-list hits in README. Target 0.
- 1 "On the list view". Target 0.
- 4 hits for the deleted module name in docs and .planning/codebase. Target 0.
  </action>
  <verify>
    <automated>test "$(grep -ciE 'box view|box list' README.md)" -eq 0 && test "$(grep -c 'On the list view' README.md)" -eq 0 && test "$(grep -rn 'roadmap_widget' docs .planning/codebase | wc -l)" -eq 0 && cargo fmt --check && rtk proxy cargo clippy --all-targets -- -D warnings && rtk proxy sh -c 'cargo test --no-fail-fast > target/wtw-final.log 2>&1; grep -E "^test result:" target/wtw-final.log | awk "{p+=\$4; f+=\$6; i+=\$8} END {print \"passed=\" p \" failed=\" f \" ignored=\" i}"; test "$(grep -cE "^test .* FAILED$" target/wtw-final.log)" -le 1'</automated>
  </verify>
  <done>
- README and the four doc trees carry no mention of the box view or the deleted file.
- fmt and clippy both exit 0.
- The full suite shows passed = BASE_PASSED − 9, ignored = BASE_IGNORED, and only the expected
  envelope::policy git-version failure. The SUMMARY records both the baseline and the final numbers.
- Commit 3 has landed with the required trailer.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| `.planning/ROADMAP.md` / STATE.md → terminal cells | Third-party project text (SAFE-07) that the Roadmap tab draws. Removing a renderer shrinks this surface. What must not shrink is the escape coverage of the renderers that stay. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-wtw-01 | Tampering | render_escape_guard.rs DETAIL_SUB_STATES / DETAIL_TAB_ARRIVAL | medium | mitigate | Only the box-view entries are removed, both together. The two-way arrival-set equality and `the_screen_renders_identity_escaped` must pass in Task 1's `ui::screens` run. The lanes list stays probed by the default RoadmapViz arrival and by the "cursor on the planned phase" and synthetic-band states. |
| T-wtw-02 | Information disclosure | src/ui/mod.rs census + the ratatui C1 behaviour | low | mitigate | The only executable pin of ratatui's control-grapheme drop moves to `ratatui_drops_a_c1_grapheme_before_a_cell_exists` and is not deleted (I-5). The census tests run in Task 2's `ui::` gate. |
| T-wtw-03 | Denial of service | detail.rs handle_key scroll matches | low | mitigate | The wildcard arms are removed only after the compiler proves exhaustiveness (Task 1 step 2g). No key can reach a deleted path. The 24-05 key tests and `roadmap_keys_are_inert_without_project_state` must stay green. |
| T-wtw-SC | Tampering | npm/pip/cargo installs | high | accept | No package is added, removed or bumped, and Cargo.toml/Cargo.lock are untouched. The legitimacy gate does not apply. |
</threat_model>

<verification>
- Task 1 gate: `ui::screens` lib tests are green, and the screens-scoped grep gates for the
  view-mode, generic-scroll and module identifiers all read 0.
- Task 2 gate: the widget file is deleted, there are no module references outside the verbatim
  census record, `PhaseMarker::glyph` is gone, the C1 pin exists, and the `ui::`, `state_reader::`
  and `state_reader_test` suites are green.
- Task 3 gate (final):
  - README and the docs trees are clean;
  - `cargo fmt --check` exits 0;
  - `cargo clippy --all-targets -- -D warnings` exits 0;
  - `cargo test --no-fail-fast` gives BASE_PASSED − 9 passed, 1 failed (envelope::policy
    git-version, expected on this machine) and BASE_IGNORED ignored.
</verification>

<success_criteria>
- The Detail screen's Roadmap tab has one rendering path, the lanes list. `v` is unbound and
  advertised nowhere: not in the help popup, the footer or the README.
- No code, state field, parameter, test helper or doc remains that only the box view needed.
  Anything another view uses still exists (the Enter/Space `_` arm, the Browse/Archive scroll arms,
  `ViewportMetrics`, `clamp_scroll`, `PhaseMarker::decide`, `phase_plan_counts`, glyph constants).
- Compiler, clippy, fmt and the full suite all agree, with the test delta explained (−9).
- The SUMMARY lists every `[INFERRED — audit]` item (I-1..I-8) and any deviation.
</success_criteria>

<output>
Create `.planning/quick/261022-wtw-remove-the-detail-screen-roadmap-box-vie/261022-wtw-SUMMARY.md` when done.
</output>
