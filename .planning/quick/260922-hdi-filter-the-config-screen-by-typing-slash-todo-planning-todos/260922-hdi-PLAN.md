---
phase: quick-260922-hdi
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
autonomous: true
requirements: [QUICK-260922-hdi]

estimate:
  tokens: 140000
  raw_tokens: 140000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "On the Config Settings (Defaults) tab, pressing / opens a filter input that is echoed, escaped, in the list block title together with a (visible/total) count; every typed character narrows the rows to those whose key OR category contains the filter text, case-insensitively."
    - "While the filter input has focus, every character key (including x, d, r, q, j, k, ?, digits) only edits the filter text: no config value is cleared, the edit target does not flip, the tab does not switch and the screen does not pop. Left/Right/Tab and other non-character keys are swallowed."
    - "Up/Down/PageUp/PageDown (and j/k once the filter is confirmed) move only among the visible rows, and defaults_selected / defaults_editing always hold UNDERLYING (unfiltered) indices into entries_for_cache."
    - "Enter while typing confirms the filter; Enter on the confirmed filtered selection acts on the correct underlying row: on an Unset enum row it opens that exact key's chooser (defaults_editing == Some(that row's underlying index)) and applying a pick writes that key and no other."
    - "Esc while typing, or Esc/q with a confirmed filter, clears the filter and keeps the screen, with the cursor on the same underlying row; with no filter, Esc/q pops exactly as before; an open chooser/text popup still closes first."
    - "A filter that matches nothing renders 'No config keys match', draws no help pane, and Enter / x / arrows are inert no-ops that do not panic."
    - "With an empty filter and no input focus, the tab's navigation arithmetic, rendering and title are identical to the pre-change behaviour, and the Unset Enter-opens-chooser regression block from 72b5e03..9ef0f7f is byte-identical and green."
    - "Re-arriving on the Config tab clears any filter and input focus; the d target toggle and r reload keep the cursor on a visible row."
  artifacts:
    - "src/ui/screens/mod.rs — ProjectViewCache gains defaults_filter: String and defaults_filter_typing: bool (additive; struct stays #[derive(Default)])"
    - "src/ui/screens/detail.rs — config_row_matches / visible_defaults_indices / move_defaults_selection / snap_defaults_selection helpers beside entries_for_cache; handle_config_filter_key typing intercept; `/` arm; filter-aware nav, Esc/q, Enter/x guards, d/r snap, arrival reset; filtered render_defaults_tab; [/]filter footer hint; config_filter_* tests"
  key_links:
    - "`/` arm in DetailScreen::handle_key -> cache.defaults_filter_typing = true -> handle_config_filter_key intercept placed AFTER the String-edit intercept and BEFORE `match code`, so typed keys never reach the x / d / r / digit / q arms"
    - "visible_defaults_indices -> move_defaults_selection (Down/Up/PgDn/PgUp arms + typing intercept) and render_defaults_tab (items built from visible, ListState::select(visible position), highlight by underlying index)"
    - "defaults_selected (underlying index) -> unchanged Enter arm entries.get(selected) -> dropdown_options -> defaults_editing = Some(selected) -> set_config_value(active, entry.key, ..): the filter only guards this path, it never rewrites it"
    - "switch_to_tab Defaults arrival reset -> clears defaults_filter and defaults_filter_typing alongside the existing defaults_selected = 0"
---

<objective>
Add a vim/less-style `/` filter to the Config Settings screen (`DetailSubView::Defaults`,
rendered by `render_defaults_tab` in `src/ui/screens/detail.rs`). `/` opens a filter input,
typing narrows the ~130 rows by key (and category), Esc clears, and Enter/arrows act on the
filtered selection, which always maps back to the correct underlying row.

Purpose: finding one config key currently means scrolling the whole list
(todo `.planning/todos/pending/2026-09-22-filter-the-config-screen-by-typing-slash.md`).
The filter must not regress the `ConfigValueKind::Unset` Enter-opens-chooser fix landed in
72b5e03..9ef0f7f.

Architecture (from 260922-hdi-RESEARCH.md, "Recommended Design"): `defaults_selected` and
`defaults_editing` STAY underlying indices into `entries_for_cache`. The filter is a derived
list of visible underlying indices; only navigation, the Esc/q arm, two no-op guards and the
render become filter-aware. Enter/x/dropdown-apply/text-input/popup code keeps speaking
underlying indices and is not rewritten. The typing intercept mirrors the dashboard's
`NormalScreen::handle_search_key` (`src/ui/screens/normal.rs:651-695`).

Output: filter state on `ProjectViewCache`, filter logic + render in `detail.rs`, a footer
hint, and a `config_filter_*` test set that drives the real `handle_key`.
</objective>

<inferred_decisions>
No CONTEXT.md exists for a quick task and the human is unavailable. These choices were made
from RESEARCH.md's Assumptions Log and are marked for later audit:

- [INFERRED A1] Match on key + category only, case-insensitive substring (mirrors the
  dashboard's `to_lowercase().contains`). Help text is excluded because its prose makes short
  terms hit dozens of unrelated rows. Value is excluded so that editing a value can never make
  the row vanish from under the cursor.
- [INFERRED A2] Enter while typing only confirms the filter (drops input focus, keeps the
  filter). The next Enter edits the selected row. This mirrors the dashboard (`normal.rs:687-692`).
- [INFERRED A3] With a confirmed filter, `q` clears it like Esc does, because the two keys
  share one arm (this is already true for closing an open popup).
- [INFERRED A4] `/` re-opens the input seeded with the current filter text so it can be
  refined. The dashboard clears it instead. Esc still clears.
- [INFERRED A5] The filter echo and count live in the list block title
  (e.g. ` Config Settings  /drift_ (5/130) `), not the footer. `footer_spans` has no cache
  access; the footer only gains a static `[/]filter` hint.
- [INFERRED A6] On every change to the filter text while typing, the cursor jumps to the first
  visible row if the filter is non-empty (mirrors the dashboard's `select(Some(0))`). It is left
  where it is if the filter became empty. `/` re-open, `d` and `r` only snap when the current
  row is hidden.
- [INFERRED A7] Arrow and Page keys move the cursor while typing. j/k are text while typing
  and navigate once the filter is confirmed.
- [INFERRED A8] `d` (target toggle) and `r` (reload) keep the filter, because the key set is
  the same across targets. Re-arriving on the tab clears it.
- Help screen (`help.rs`) is left unchanged. It has no Config section today.
</inferred_decisions>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/STATE.md
@.planning/quick/260922-hdi-filter-the-config-screen-by-typing-slash-todo-planning-todos/260922-hdi-RESEARCH.md

Anchors (verified against HEAD 9ef0f7f; line numbers drift after the first edit, so re-locate each anchor by its identifier):
- `src/ui/screens/mod.rs:906-984`: `#[derive(Default)] pub struct ProjectViewCache`. It holds `defaults_selected: usize` (971), `defaults_editing: Option<usize>` (974), `defaults_dropdown_selected`, and `defaults_text_buffer: EditBuffer` (984). Every struct literal in the tree uses `..Default::default()`, so new fields are additive.
- `detail.rs` `switch_to_tab` (1258) has a Defaults arrival reset at 1315-1332 that sets `defaults_selected = 0` and `defaults_editing = None` and clears the buffer.
- `detail.rs` `DetailScreen::handle_key` (1461): the String-edit intercept is at 1479-1497, then `match code {` at 1499. The Esc/q arm's Defaults popup-close block is at 1589-1599, followed by `ScreenAction::Pop` at 1600-1602.
- `detail.rs` nav arms, Defaults non-editing branches: Down/j at 1713-1719, Up/k at 1835, PageDown at 1986-1990, PageUp at 2122. `PAGE_SCROLL_LINES` is at line 25.
- `detail.rs` Enter/Space Defaults arm is at 2583-2643. Its non-editing branch starts at 2607 (`let selected = cache.defaults_selected;`).
- `detail.rs` other Defaults arms: `x` clear at 2708-2753 (its non-editing branch is at 2716-2719), `r` reload at 2755-2771, and the `d` toggle at 2814-2841 (`defaults_selected = 0` at 2833).
- `detail.rs` `render_defaults_tab` spans 4957-5231. It contains the empty-config early return (4970-4983), the items `enumerate()` loop with `entry.show_category` (4985-5064), the title (5066-5069), `ListState::select(Some(selected))` (5090-5092) and the help pane clamp (5094-5101).
- `detail.rs` `footer_spans` Defaults arm is at 6016-6025. The pinned string is in `test_other_footers_unchanged_by_browse_edit_hint` at 9468-9479.
- `detail.rs` `entries_for_cache` (7358) and `entries_count_for_cache` (7371): new helpers go beside these.
- `detail.rs` `shown()` (77-79) is the escape helper. Every operator/untrusted string drawn into a cell goes through it.
- `detail.rs` test harness pieces: `test_ctx()` (10118; no projects registered, so a Project-target persist is a no-op), `press()` (10223, discards the ScreenAction; call `screen.handle_key(code, KeyModifiers::NONE, &mut ctx)` directly when the action matters), `ctx_on_config_row(config, key)` and `sparse_gsd_config()` (13418-13439), the TestBackend buffer-scrape pattern (13462-13481), and `build_defaults_entries(&config, None)` and `dropdown_options(&kind)`.
- The Unset regression block is at `detail.rs:13412-13556`, starting at the comment `// --- debug enter-on-unset-config-row: Enter opens the chooser`. Its tests are `enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick` and `every_unset_choice_row_opens_a_chooser_whose_options_all_apply`. The test module's closing `}` is at 13557.
- Existing pattern to mirror is `src/ui/screens/normal.rs`: the intercept at 415-417, `/` entry at 588-595, `handle_search_key` at 651-695, and the echo in `render_search_footer` at 1074-1101.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Tracer, "/ filter then Enter opens the right Unset row's chooser" end to end (state, key intercept, navigation, render)</name>
  <files>src/ui/screens/mod.rs, src/ui/screens/detail.rs</files>
  <read_first>
    - src/ui/screens/detail.rs lines 1479-1500 (String-edit intercept + start of `match code`), 1700-1722, 1829-1840, 1984-1992, 2118-2126 (nav arms), 2583-2643 (Enter arm), 4957-5101 (render_defaults_tab list/title/ListState/help), 7355-7375 (entries_for_cache), 13412-13481 (Unset regression helpers + TestBackend scrape)
    - src/ui/screens/normal.rs lines 651-695 (handle_search_key, the pattern to mirror)
    - src/ui/screens/mod.rs lines 965-985 (ProjectViewCache defaults fields)
  </read_first>
  <behavior>
    New tests go in the `detail.rs` test module, APPENDED AFTER the last existing test (after `every_unset_choice_row_opens_a_chooser_whose_options_all_apply`, just before the module's closing brace). Every name starts with `config_filter_`, and the tests reuse `ctx_on_config_row`, `sparse_gsd_config`, `press`, `build_defaults_entries`, `dropdown_options` and the TestBackend scrape pattern. Do not edit any line of the existing Unset block.
    - config_filter_enter_on_a_filtered_unset_row_opens_that_rows_chooser:
      1. Setup: `ctx_on_config_row(sparse_gsd_config(), "mode")` parks the cursor on `mode`. `target_idx` is the position of `workflow.context_drift_action` in `build_defaults_entries(&sparse_gsd_config(), None)`.
      2. Preconditions: `target_idx != mode_idx`, and the target row's value is "(unset)".
      3. Type the filter: press `/`, then each char of "context_drift_action". Assert `defaults_filter == "context_drift_action"`, `defaults_filter_typing == true` and `defaults_selected == target_idx`. The query contains x, d, r, a, i, o, n and none of them fired its shortcut.
      4. Confirm: press Enter. Assert typing is false, the filter is kept and `defaults_editing` is None.
      5. Open: press Enter again. Assert `defaults_editing == Some(target_idx)`.
      6. Apply: press Down, then Enter. The target row now reads `dropdown_options(&entries[target_idx].kind)[1]`, and every other row's value equals its value before the test.
    - config_filter_arrows_move_only_among_matching_rows:
      1. Setup: sparse config, cursor on `mode`. `expected` is the list of indices whose key, lowercased, contains "drift", computed in the test and NOT via the production helper. Precondition: `expected.len() == 5`.
      2. Type "drift" after `/`. Assert `defaults_selected == expected[0]`.
      3. Down x4 visits `expected[1..=4]` in order, and a 5th Down stays on `expected[4]`.
      4. Up returns to `expected[3]`, PageUp goes to `expected[0]` and PageDown goes to `expected[4]`.
      5. Enter confirms. j/k now move within `expected` the same way.
      6. Case-insensitivity: a fresh ctx typing "DRIFT" selects the same set.
    - config_filter_render_shows_only_matches_and_count:
      1. Draw at TestBackend 160x45 after `/` + "drift" (still typing).
      2. The scraped text contains all 5 drift keys, and does NOT contain `granularity`. Precondition: `granularity` is a row key and its key does not contain "drift".
      3. The text contains "/drift" and "(5/{total})", where total = `build_defaults_entries(..).len()`.
      4. Confirm with Enter. The title no longer contains "/drift_", while "/drift" and the count remain.
    Run them first and watch them fail (RED, compile failure on missing fields counts), then implement (GREEN).
  </behavior>
  <action>
    State (src/ui/screens/mod.rs): add to ProjectViewCache after `defaults_text_buffer`:
    - `pub defaults_filter: String`, documented as the operator-typed `/` filter for the Config tab. It is matched raw and only ever drawn through `shown()`.
    - `pub defaults_filter_typing: bool`, documented as true while the filter input line has focus.
    Both are additive. Default stays derived.

    Helpers (detail.rs, beside `entries_for_cache`):
    - `config_row_matches(entry: &ConfigEntry, q_lower: &str) -> bool`: true when `q_lower` is empty, or when `entry.key` lowercased or `entry.category` lowercased contains it. This is key + category only [INFERRED A1].
    - `visible_defaults_indices(cache, entries) -> Vec<usize>`: the underlying indices of rows matching `cache.defaults_filter` lowercased. It returns all indices when the filter is empty.
    - `move_defaults_selection(cache: &mut ProjectViewCache, delta: isize)`:
      - Filter EMPTY: reproduce today's arithmetic exactly. Positive delta gives `min(selected + delta, count - 1)` when count > 0; negative delta gives `saturating_sub`. This keeps stale-cursor behaviour identical.
      - Filter non-empty: find the selected row's position in visible. If it is hidden, snap to `visible[0]`. Otherwise clamp position + delta to `[0, len-1]` and store `visible[new_pos]`.
      - An empty visible list is a no-op.
    - `snap_defaults_selection(cache)`: only when the filter is non-empty, visible is non-empty and selected is not in visible, set selected to `visible[0]`.
    - `select_first_visible(cache)`: when the filter is non-empty and visible is non-empty, set selected to `visible[0]` [INFERRED A6].
    Each helper builds entries itself via `entries_for_cache(cache)` so callers only hold `&mut ProjectViewCache`.

    Key handling (DetailScreen::handle_key):
    1. Typing intercept. Place it right AFTER the existing String-edit intercept block and BEFORE `match code`. When `current_view == DetailSubView::Defaults` and the cache for `self.alias` has `defaults_filter_typing == true` and `defaults_editing.is_none()`, return `self.handle_config_filter_key(code, ctx)`.
    2. New method `handle_config_filter_key`, modelled on `normal.rs::handle_search_key`. Its arms:
       - `Char(c)`: push c, then `select_first_visible`.
       - `Backspace`: pop, then `select_first_visible`.
       - `Esc`: clear the filter and set typing false. The selection is untouched; it is an underlying index.
       - `Enter`: set typing false, keep the filter, then `snap_defaults_selection` [INFERRED A2].
       - `Down` / `Up` / `PageDown` / `PageUp`: call `move_defaults_selection` with +1 / -1 / +PAGE_SCROLL_LINES / -PAGE_SCROLL_LINES [INFERRED A7].
       - `_`: `ScreenAction::None`.
       Every arm sets `ctx.needs_redraw = true` and returns `ScreenAction::None`. This is what keeps x/d/r/q/digits/?/Left/Right/Tab from reaching their global arms while typing (RESEARCH Pitfall 2).
    3. New arm `KeyCode::Char('/') if current_view == DetailSubView::Defaults`, placed in `match code` near the `x` arm:
       - When `defaults_editing.is_some()`: redraw-free `ScreenAction::None`.
       - Otherwise: set `defaults_filter_typing = true`, keep the existing filter text [INFERRED A4], call `snap_defaults_selection`, set redraw, return None.
    4. In the Down/j, Up/k, PageDown and PageUp arms, replace ONLY the Defaults non-editing branch's arithmetic with `move_defaults_selection(cache, ±1 / ±PAGE_SCROLL_LINES as isize)`. The dropdown-cursor (editing) branches stay unchanged.

    Render (render_defaults_tab):
    - Keep the `entries.is_empty()` early return.
    - Read `filter` / `typing` from the cache (defaults when there is no cache). Compute `visible` via `visible_defaults_indices`.
    - Build `items` by iterating `visible` (underlying idx + entry) instead of `enumerate()` over all entries. Keep the existing span construction for each row exactly as is, including its `shown()` escaping of value/pass-through key.
    - Category column:
      - Filter empty: use `entry.show_category` unchanged, so the unfiltered render is byte-identical.
      - Filter non-empty: show the category when the row is the first visible row or its category differs from the previous visible row's category.
    - Highlight when the underlying `idx == selected`.
    - ListState:
      - Filter empty: keep `select(Some(selected))` exactly.
      - Filter non-empty: `select(visible.iter().position(|&i| i == selected))`. This is the VISIBLE position, never the underlying index (RESEARCH Pitfall 3).
    - Title: when the filter is non-empty or typing is true, append to the existing base title a segment of the form space, `/`, `shown(filter)`, then `_` only while typing, then ` (visible/total) `. Example: ` Config Settings  /drift_ (5/130) `. Both the Project and Global titles get the segment. With the filter empty and not typing, the title is unchanged.
    - Help pane:
      - Filter empty: keep the existing clamp.
      - Filter non-empty: render help only when `visible` contains `selected`.
    - The popup block is unchanged.

    Do not add value-matching or help-text matching. Do not change the Enter/x/dropdown/text-input code in this task.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --lib config_filter_</automated>
    <automated>rtk proxy cargo test --no-fail-fast --lib -- enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick every_unset_choice_row_opens_a_chooser_whose_options_all_apply</automated>
    <automated>rtk proxy bash -c 'l=$(cargo test --lib config_filter_ -- --list 2>&1) || { echo "$l" | tail -20; exit 1; }; n=$(grep -c ": test$" <<<"$l"); echo "config_filter_ tests: $n"; [ "$n" -ge 3 ]'</automated>
  </verify>
  <done>
    - The 3 config_filter_ tests exist (the --list count is 3 or more) and pass.
    - The two Unset regression tests pass unchanged.
    - `/`, then typing a query that contains x/d/r, then Enter and Enter opens the chooser for the matching key's underlying row, and the pick writes that key only.
    - Commit the RED tests, then the GREEN implementation.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Esc/q clearing, typing swallows shortcuts, empty-result inertness, d/r/arrival consistency, escaped echo</name>
  <files>src/ui/screens/detail.rs</files>
  <read_first>
    - src/ui/screens/detail.rs: 1258-1335 (switch_to_tab + Defaults arrival reset), 1586-1604 (Esc/q arm Defaults block + Pop), 2600-2645 (Enter non-editing branch), 2708-2753 (x arm), 2755-2771 (r arm), 2814-2841 (d arm), render_defaults_tab as modified by Task 1
  </read_first>
  <behavior>
    Append these after Task 1's tests, all prefixed `config_filter_`. Capture ScreenAction by calling `screen.handle_key(code, KeyModifiers::NONE, &mut ctx)` directly and asserting with `matches!`. Key-order safety: never press x or an applying Enter AFTER a `d` in the same test. `d` switches the target to Global, whose persist path is the operator's real `~/.gsd/defaults.json`. The Project target has no project registered in `test_ctx()`, so it never writes.
    - config_filter_typing_swallows_shortcut_keys:
      1. Sparse config, cursor on a SET row (`mode`). Snapshot the entries' values.
      2. Press `/`, then in order x, q, 3, r, ?, j, k, then d LAST.
      3. Every action is `ScreenAction::None`. The filter equals "xq3r?jkd".
      4. `defaults_edit_target` is still Project and the sub-view is still Defaults.
      5. Every row value is unchanged. `mode` still reads "yolo".
      6. Left, Right, Tab and Delete also return None and leave the sub-view unchanged.
      This pins behaviour Task 1 already built, so it may be GREEN on first run; that is expected.
    - config_filter_esc_clears_typing_then_confirmed_filter_then_pops:
      1. `/` + "drift", then Esc while typing. The filter is empty, typing is false, the action is None, and the cursor stays on the same underlying row it held before Esc.
      2. `/` + "drift", Enter, then Esc. The filter is cleared and the action is None.
      3. Repeat that with `q` in place of Esc: the same, no Pop [INFERRED A3].
      4. Esc with no filter returns `ScreenAction::Pop`.
      5. With a confirmed filter and a chooser open (Enter on the drift_action row), the first Esc closes the popup and keeps the filter, the second clears the filter, and the third pops.
    - config_filter_empty_result_is_inert:
      1. `/` + "zzzz", Enter. Snapshot `defaults_selected` and all values.
      2. Enter, x, Down, Up, PageDown and PageUp change neither selection, values nor `defaults_editing`, and none of them panics.
      3. The render at 120x30 contains "No config keys match" and "(0/".
    - config_filter_x_clears_the_filtered_rows_key_only:
      1. Config `{"mode":"yolo","workflow":{"drift_threshold":5}}` via `parse_gsd_config`, cursor on `mode`.
      2. `/` + "drift_threshold", Enter, x.
      3. The `workflow.drift_threshold` row reads "(unset)" and `mode` still reads "yolo".
    - config_filter_arrival_and_target_toggle_keep_cursor_consistent:
      1. Arrival: set `defaults_filter = "drift"` and typing true in the cache, then call `switch_to_tab(TEST_ALIAS, tab_index(&DetailSubView::Defaults), &mut offset, &mut ctx)`. The filter is empty and typing is false.
      2. Target toggle: in a fresh ctx, confirm filter "drift", then press d. `defaults_selected` is one of the drift rows' underlying indices, not 0 unless row 0 matches. Assert nothing mutating after this.
    - config_filter_echo_is_escaped:
      1. `/`, then `Char('\u{1b}')` and `Char('a')`. The cache filter holds the raw ESC char, since matching is raw.
      2. The TestBackend render has no cell whose symbol contains '\u{1b}', and the title still shows the `(n/total)` segment.
  </behavior>
  <action>
    Esc/q arm (the Defaults block inside `KeyCode::Esc | KeyCode::Char('q')`): keep the existing popup-close check first and unchanged. After it, and before the fall-through to the scroll reset and `ScreenAction::Pop`, add a check. If `cache.defaults_filter` is non-empty, clear it, set `defaults_filter_typing = false`, set redraw and return `ScreenAction::None` [INFERRED A3]. With an empty filter the arm pops exactly as before.

    Enter/Space arm, Defaults non-editing branch only: before `entries.get(selected)`, add a guard. When `defaults_filter` is non-empty and `visible_defaults_indices` does not contain `selected`, do nothing: skip to the existing redraw/None tail. The dropdown-apply (editing) branch and everything after the guard stay byte-for-byte unchanged.

    `x` arm: add the same guard inside `if cache.defaults_editing.is_none()` before `entries.get(selected)`.

    `d` arm: call `snap_defaults_selection(cache)` right after the existing `defaults_selected = 0` [INFERRED A8, the filter is kept]. `r` arm: call `snap_defaults_selection(cache)` after the reload, because the pass-through row set may change.

    Arrival reset in `switch_to_tab` (the `new_view == DetailSubView::Defaults` block): add `defaults_filter.clear()` and `defaults_filter_typing = false` beside the existing `defaults_selected = 0` (RESEARCH Pitfall 6).

    Render empty result: in render_defaults_tab, when the filter is non-empty and `visible` is empty:
    - Render the list block with the normal title (including the ` (0/total) ` echo) around a single dim (DarkGray) ListItem holding the literal "  No config keys match".
    - Pass a ListState with `select(None)`.
    - Skip the help pane.
    - `move_defaults_selection` already no-ops on an empty visible list; re-check that no path indexes `visible[0]` unguarded.

    Escaping: the title segment must interpolate `shown(&filter)`, never the raw filter. Never put the filter into a `ConfigHelp`, which must stay `&'static`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --lib config_filter_</automated>
    <automated>rtk proxy bash -c 'l=$(cargo test --lib config_filter_ -- --list 2>&1) || { echo "$l" | tail -20; exit 1; }; n=$(grep -c ": test$" <<<"$l"); echo "config_filter_ tests: $n"; [ "$n" -ge 9 ]'</automated>
    <automated>rtk proxy cargo test --no-fail-fast --lib -- enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick every_unset_choice_row_opens_a_chooser_whose_options_all_apply</automated>
  </verify>
  <done>
    - At least 9 config_filter_ tests exist and all pass.
    - Esc/q semantics match the truths.
    - An empty result is inert and renders "No config keys match".
    - Typing never clears a value, flips the target, switches the tab or pops.
    - The filter echo is escaped.
    - The Unset regression tests still pass.
    - Commit the RED tests where they fail first, then the GREEN implementation.
  </done>
</task>

<task type="auto">
  <name>Task 3: [/]filter footer hint, pinned footer update, full regression gate</name>
  <files>src/ui/screens/detail.rs</files>
  <read_first>
    - src/ui/screens/detail.rs: footer_spans Defaults arm (search `DetailSubView::Defaults => {` inside `fn footer_spans`), test_other_footers_unchanged_by_browse_edit_hint
  </read_first>
  <action>
    Footer: in `footer_spans`, Defaults arm, push a bold `[/]` span followed by raw `filter  ` after the `[r]` / `eload  ` pair. The resulting Defaults footer at width 120 is exactly: `  [Esc]back  [1-0/D]tabs  [j/k]scroll  [Enter]edit  [x] clear  [d] defaults  [r]eload  [/]filter  [?]help`. Update the pinned Defaults expectation in `test_other_footers_unchanged_by_browse_edit_hint` to that exact string. The Backlog pin in the same test does not change. `the_tabs_hint_drops_shift_d_on_every_tab_when_experimental_is_off` checks only the prefix and needs no change.

    Then run the full gate. The only acceptable failing test anywhere is `the_config_section_constants_record_the_git_version_they_were_derived_against` in src/envelope/policy.rs, which fails locally by design (local git 2.53.0 vs the constant's 2.55.0). Any other failure is a regression to fix.

    Clippy:
    - `cargo clippy -- -D warnings` (the release gate's form) must exit 0.
    - `cargo clippy --all-targets` has pre-existing warnings in tests/envelope_*.rs, src/browser.rs and src/project_creator.rs. Those are out of scope; leave them. None may point at src/ui/screens/detail.rs or src/ui/screens/mod.rs.

    Unset-fix regression proof: the 145 lines starting at the comment `// --- debug enter-on-unset-config-row: Enter opens the chooser` must be identical to commit 9ef0f7f. The new tests were appended after this block, so no diff is expected.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --lib test_other_footers_unchanged_by_browse_edit_hint</automated>
    <automated>rtk proxy bash -c 'out=$(cargo test --no-fail-fast 2>&1); grep -q "^test result:" <<<"$out" || { echo "no test results"; exit 1; }; if grep -q "^error: could not compile" <<<"$out"; then echo "compile error"; exit 1; fi; failed=$(grep -E "^test .+ \.\.\. FAILED$" <<<"$out"); echo "FAILED list:"; echo "$failed"; bad=$(grep -v the_config_section_constants_record_the_git_version_they_were_derived_against <<<"$failed" | grep .); [ -z "$bad" ]'</automated>
    <automated>rtk proxy cargo clippy -- -D warnings</automated>
    <automated>rtk proxy bash -c 'out=$(cargo clippy --all-targets 2>&1) || { echo "$out" | tail -30; exit 1; }; ! grep -E "\-\-> src/ui/screens/(detail|mod)\.rs" <<<"$out"'</automated>
    <automated>rtk proxy bash -c 'base=$(git show 9ef0f7f:src/ui/screens/detail.rs) || exit 1; m="debug enter-on-unset-config-row: Enter opens the chooser"; diff <(awk -v m="$m" "index(\$0,m){f=1} f" <<<"$base" | head -n 145) <(awk -v m="$m" "index(\$0,m){f=1} f" src/ui/screens/detail.rs | head -n 145)'</automated>
  </verify>
  <done>
    - The footer pin test passes with `[/]filter`.
    - The full `cargo test --no-fail-fast` FAILED list contains exactly one line, and that line names `the_config_section_constants_record_the_git_version_they_were_derived_against`.
    - `cargo clippy -- -D warnings` exits 0.
    - All-targets clippy reports nothing in detail.rs or mod.rs.
    - The Unset block diff against 9ef0f7f is empty.
    - Committed.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| operator keyboard -> DetailScreen filter state | Typed text becomes `defaults_filter` and is echoed into a terminal cell. Every other key is a potential config mutation (x clears on disk, d flips the write target). |
| project `.planning/config.json` -> Config rows | Pass-through keys (category "Not modelled") are project-supplied and untrusted (T-VQW-01). The filter matches against them. |
| filtered view -> underlying config mutation | A visible-position vs underlying-index confusion would edit or clear the wrong key and persist it. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-HDI-01 | Tampering | handle_config_filter_key intercept | high | mitigate | The intercept sits before `match code` and returns None for every key, so x/d/r/q/digits cannot fire while typing. Pinned by config_filter_typing_swallows_shortcut_keys (Task 2) and by the tracer query containing x/d/r (Task 1). |
| T-HDI-02 | Tampering | Enter / x on a filtered selection | high | mitigate | `defaults_selected`/`defaults_editing` stay underlying indices. The Enter/x bodies are unchanged except for a visible-guard. Pinned by config_filter_enter_on_a_filtered_unset_row_opens_that_rows_chooser (other rows unchanged) and config_filter_x_clears_the_filtered_rows_key_only. |
| T-HDI-03 | Tampering (terminal injection) | render_defaults_tab title echo | medium | mitigate | The filter is interpolated only via `shown()` (`render_for_terminal`). Pinned by config_filter_echo_is_escaped (no raw ESC cell). |
| T-HDI-04 | Denial of Service | empty result / stale cursor | medium | mitigate | No unguarded `visible[0]`. Enter/x get a visible-guard, the help pane is skipped and ListState is None. Pinned by config_filter_empty_result_is_inert. |
| T-HDI-05 | Information Disclosure | help pane | low | mitigate | Help renders only for a visible selected row while a filter is active. |
| T-HDI-06 | Tampering | pass-through keys matched by the filter | low | accept | Keys are matched raw in memory and drawn through the existing escaped row render, which is unchanged. The filter adds no new render of untrusted text. |
| T-HDI-07 | Tampering | test suite writing the operator's ~/.gsd/defaults.json | medium | mitigate | Tests never press x or an applying Enter after `d`. The Project target has no registered project in `test_ctx()`, so persisting is a no-op. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast --lib config_filter_`: all of 9 or more tests pass.
- `rtk proxy cargo test --no-fail-fast`: the only FAILED test is the git-version witness in src/envelope/policy.rs.
- `rtk proxy cargo clippy -- -D warnings` exits 0, and all-targets clippy is silent for detail.rs and mod.rs.
- The Unset regression block is byte-identical to 9ef0f7f and both of its tests pass.
</verification>

<success_criteria>
- The Config Settings screen has a working `/` filter: typing narrows by key/category, the echo and count show in the title, Esc clears, and Enter/arrows act on the filtered selection mapped to the right underlying row.
- No shortcut fires while typing. An empty result is inert. The unfiltered tab is unchanged.
- The ConfigValueKind::Unset Enter-opens-chooser fix (72b5e03..9ef0f7f) is not regressed, and is re-proved through the filter.
- The inferred decisions A1-A8 are recorded in SUMMARY.md for audit.
</success_criteria>

<output>
Create `.planning/quick/260922-hdi-filter-the-config-screen-by-typing-slash-todo-planning-todos/260922-hdi-SUMMARY.md` when done. Record the inferred decisions A1-A8, the final config_filter_ test count and the full-suite FAILED list.
</output>
