---
quick_id: 261118-tki
mode: quick
phase: quick-261118-tki
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/help.rs
  - README.md
  - docs/GETTING-STARTED.md
autonomous: true
requirements: [QUICK-261118-tki]

estimate:
  tokens: 120000
  raw_tokens: 120000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "In the help overlay, pressing a key that some row's key column lists (e.g. `s`, `x`, `G`, `Enter`, `Space`, `Left`, `Tab`, `3`) marks every row that lists it. The current row gets the ASCII marker `> ` plus REVERSED|BOLD, and every other matching row gets `* ` plus BOLD. A row that mentions the key only in its description is never marked (D-01, D-05)."
    - "The popup title reads ` Help  <key>: <i>/<n> `. Pressing the same key again moves to the next match and wraps from n back to 1. The view scrolls through clamp_scroll so the current match is visible and not under the more-indicator row (D-03, D-06)."
    - "A key that no key column lists (e.g. `n`, `9`, F5, Shift+Tab) sets the title to ` Help  no help entry for <key> ` and marks nothing. Key names in the title are ASCII only: any other char is shown as `U+XXXX` (D-03, I-tki-7)."
    - "`?` and Esc close the overlay as before. Up, Down, PgUp and PgDn scroll it with the UIFIX-04 ordering. `j` and `k` no longer scroll; they highlight like any other key (D-04)."
    - "Each key-section row of the help body, with the experimental flag on or off, has a key column that tokenizes to at least one key or whose first word is in NON_KEY_LABELS. A test fails on any new row that meets neither condition (D-02)."
    - "The README `?` row and the docs/GETTING-STARTED.md `?` bullet describe the new behavior. The overlay's own closing hint names the scroll keys and the find-a-key behavior (D-08)."
  artifacts:
    - path: src/ui/screens/help.rs
      provides: "HelpKey, key_tokens, help_entries, apply_highlight, reveal_offset, key_label, the Highlight state on HelpScreen, and the find arm in handle_key"
      contains: "fn key_tokens"
    - path: README.md
      provides: "Key bindings table `?` row describing highlight and cycling"
      contains: "highlights the rows"
  key_links:
    - from: src/ui/screens/help.rs
      to: src/ui/screens/detail.rs
      via: "reveal_offset calls clamp_scroll (and more_below) for every scroll a match causes; no new max-scroll arithmetic"
      pattern: "clamp_scroll"
    - from: src/ui/screens/help.rs
      to: src/ui/screens/help.rs
      via: "handle_key and render both derive matches from help_entries(&help_lines(ctx.experimental)), so the counted rows are exactly the drawn rows"
      pattern: "help_entries"
---

<!-- Positive gates (EXPECT>=1) on new identifiers: these literals must appear in actions. -->
<!-- planner-discipline-allow: fn key_tokens -->
<!-- planner-discipline-allow: fn help_entries -->
<!-- planner-discipline-allow: CURRENT_MATCH_MARKER -->
<!-- planner-discipline-allow: NON_KEY_SECTIONS -->
<!-- planner-discipline-allow: fn reveal_offset -->
<!-- planner-discipline-allow: NON_KEY_LABELS -->
<!-- planner-discipline-allow: no help entry for -->
<!-- planner-discipline-allow: Up/Down, PgUp/PgDn scroll -->
<!-- planner-discipline-allow: Any other key: highlight its rows -->
<!-- planner-discipline-allow: Finding a key -->
<!-- planner-discipline-allow: highlights the rows -->
<!-- planner-discipline-allow: highlight -->

<objective>
Owner request (verbatim): "When I'm in the help menu, I want to be able to press a key and it highlights the action (apart from ESC)."

Today `HelpScreen::handle_key` (src/ui/screens/help.rs:666) pops on `?`/Esc, scrolls on `j`/Down, `k`/Up, PageDown and PageUp, and consumes every other key without effect. This plan turns those keys into a find. A key press marks every row whose key column lists that key, the view scrolls to the current match, and pressing the key again moves to the next one.

Decisions. The orchestrator inferred all of them; record each as inferred in the SUMMARY.
- D-01 [inferred, orchestrator]: A pressed printable key, or a named key the help lists (Enter, Tab, Space, the arrows, ...), highlights every row whose key column names it. Matching parses key-column tokens (`j/k` gives j and k, `Esc / q` gives q, `1 2 3 4` gives 3, `1-8` gives 1 to 8, named tokens map from KeyCode). Descriptions are never substring-matched. Uppercase and lowercase are different keys.
- D-02 [inferred, orchestrator]: The tokenizer covers the spellings help_lines actually uses. A test proves every row's key column tokenizes to at least one key or is explicitly exempt, so no future row can silently become unmatchable.
- D-03 [inferred, orchestrator]: The view scrolls so the match is visible. The same key again cycles to the next match and wraps. The overlay shows a hint like `x: 2/3`. A key with no match shows `no help entry for <key>`.
- D-04 [inferred, orchestrator]: Esc closes as today. `?` keeps closing, so `?` itself cannot be highlighted (documented). Arrows and PgUp/PgDn keep scrolling. `j`/`k` stop scrolling and highlight instead. Tests and README that rely on j/k scrolling the help are updated.
- D-05 [inferred, orchestrator]: The highlight never relies on color alone. It uses reverse/bold plus a textual ASCII marker on matched rows, with a stronger marker on the current match. The src/ui glyph scan (src/ui/glyphs.rs tests, `drawn_codepoints`) stays green.
- D-06 [inferred, orchestrator]: Scroll math reuses clamp_scroll. No new copy of the max-scroll subtraction (module doc lines 18-23).
- D-07 [inferred, orchestrator]: Tests follow the file's whole-row style (expected rows built through `row()` / `wrapped_row()`, never `contains(key)`). Verification: `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings`, run through `rtk bypass`.
- D-08 [inferred, orchestrator]: README's help-overlay key docs are updated.

Planner decisions (inferred, recorded as I-tki-N in the SUMMARY):
- I-tki-1: Only Up, Down, PgUp and PgDn scroll. Left and Right never scrolled the overlay (it has no horizontal scroll), so they highlight the `Left / Right` rows. Home, End and other unlisted keys give the no-match notice.
- I-tki-2: Rows under the legend and pseudo-key headings (`NON_KEY_SECTIONS`: Filter Syntax, Owner Work, Dashboard Badges, Work State, Driver Run States, Driver Item States) are never matched, because their first column holds filter syntax, a counter label or a glyph, not a key. Inside key sections, mouse gestures and the inbox's pseudo-keys are exempt by the label's first word (`NON_KEY_LABELS`: Click, Double-click, Wheel, Shift+drag, Stale, Labels). Any other heading, including one added later, is a key section by default, so the coverage test forces a decision for its rows.
- I-tki-3: A fresh key starts at the first match whose first line is at or below the current top row, and wraps to the first match in the body when there is none. Help opened from the inbox at the Owner Inbox section therefore finds the inbox's own row first. The counter is the 1-based position in body order.
- I-tki-4: Markers replace the row's two-space indent, so the key column stays aligned: `> ` for the current match and `* ` for the others. The current row is REVERSED|BOLD and the others are BOLD. Continuation rows of a matched entry (empty key column) get the same marker and style.
- I-tki-5: The hint lives in the popup's block title (` Help  x: 2/4 `), so it takes no content row and the body tests stay unchanged.
- I-tki-6: Shift is ignored, because chars already carry case. An ASCII lowercase char reported with SHIFT is uppercased, since some keyboard protocols send Shift+d that way. Ctrl is part of the key: the `Ctrl+C` row tokenizes to Ctrl+c, which can never be pressed here because app.rs quits on it before any screen sees it. That is documented. Alt is ignored.
- I-tki-7: The title shows only fixed key names and ASCII-graphic chars. Any other char is shown as `U+XXXX`, and an unnamed KeyCode as `that key`. Modifier-only and Null key events change nothing.
- I-tki-8: Esc closes even while a highlight is active. There is no clear-first step.
- I-tki-9: The closing DIM hint becomes two lines that both fit the 62-cell inner width of the 80x24 popup.
- I-tki-10: When the current match is already visible, the view does not move. Otherwise clamp_scroll puts the match on the top row, or at the view's maximum near the end.
- I-tki-11: `Shift+Tab` (BackTab) is in the grammar even though no row uses it today.

Purpose: let the owner find what a key does from inside the help overlay, without reading the whole body.
Output: src/ui/screens/help.rs (find path plus tests), README.md, docs/GETTING-STARTED.md.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@src/ui/screens/help.rs

Read only these ranges of other files (no whole-file reads):
- src/ui/screens/detail.rs:286-309 (`ViewportMetrics`, `clamp_scroll`).
- src/ui/glyphs.rs:309-400 (the production source scan: what it counts, and that ASCII is outside it).
- src/ui/screens/render_escape_guard.rs:3789-3815 (`every_adjudication_reason_is_non_empty_and_names_values_not_verdicts`: the reason must not contain the whole tokens `escaped` or `safe`).
- src/app.rs:3917-3930 (`App::handle_key`: only Ctrl+C is intercepted before the top screen).
- src/ui/screens/driver_confirm.rs:390 (`ctx_with_project`; the fixture's `experimental` flag is ON).
- README.md:253-272 (Key bindings table; the `?` row is line 268).
- docs/GETTING-STARTED.md:160-171 (the `?` bullet is line 169).

Facts measured by the planner at HEAD 3266fb5d (re-measure if the body changed):
- The help test filter `rtk bypass cargo test --locked --lib -- ui::screens::help` lists 35 tests.
- Key-column spellings in help_lines: `j / Down`, `k / Up`, `Enter`, `b`, `/`, `s`, `i`, `a`, `c`, `d`, `G`, `?`, `e`, `j/k g/G`, `h / l`, `[ / ]`, `Space`, `j/k PgUp/PgDn`, `t`, `m`, `g`, `q / Esc`, `Ctrl+C`, `1-8`, `Left / Right`, `Down / Enter`, `Up`, `Esc`, `q`, `Tab`, `Right / Enter`, `PgUp / PgDn`, `Enter / Space`, `Left / Esc`, `Up / k`, `v o A y x`, `Click`, `Click value`, `Click` + the fold triangles, `Double-click`, `Wheel`, `M`, `Shift+drag`, `Shift+D`, `j / k`, `f`, `u`, `R`, `x`, `o`, `T`, `v`, `1 2 3 4`, `j/k Up/Down`, `A`, `y`, `Stale`, `Labels`, `Esc / q`, plus empty labels on continuation rows.
- Expected match counts with the flag ON (the fixture default): `s` 2 (sort row first), `x` 4 (Owner block `v o A y x` first, then the two Driver `x` rows, then the inbox `x`; flag OFF: 2), `g` 5 (the Roadmap `j/k g/G` row first, the inbox `g` row last), `j` 7, `Left` 5, `Space` 3, `Tab` 1, `3` 2, `8` 1, `9` 0, `n` 0, `N` 0, `m` 1, `M` 1, Ctrl+c 1, `D` 1 (flag OFF: 0), plain `c` 2.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end, pressing `s` in the help overlay marks its rows and counts them in the title (tracer)</name>
  <files>src/ui/screens/help.rs</files>
  <read_first>
    - src/ui/screens/help.rs:208-260 (`heading`, `row`, the start of `help_lines`), :587-650 (`wrapped_row`, `wrap_owner_sections`), :652-762 (`adjudicate_screen!`, `handle_key`, `render`, `more_below`), :764-820 (test helpers `body_with`, `press`, `screen_with_viewport`), :1120-1144 (the 100x30 TestBackend render test to copy the buffer-reading shape from)
    - src/ui/screens/detail.rs:286-309
  </read_first>
  <behavior>
    - key_tokens("v o A y x") is Ok with the five plain char keys v, o, A, y, x in that order. key_tokens("j/k g/G") is Ok with j, k, g, G. key_tokens("/") is Ok with the slash key. key_tokens("[ / ]") is Ok with `[` and `]`. key_tokens("?") is Ok with `?`. key_tokens("Click value") is Err (it stays Err after Task 2).
    - help_entries(&help_lines(true)) yields exactly two entries whose keys contain plain `s`: the SORT_ROW row and the Driver row "Start a new run: name it, edit it, check it, launch it", each compared as the whole row built through `row()`. The Filter Syntax `/term/s` row yields no keyed entry. The Driver `e` entry ("Edit this run's DIRECTION.md in $EDITOR") spans its three continuation rows (last minus first is 3).
    - apply_highlight for `s` with current 0 changes exactly two lines and keeps the line count. One line starts with CURRENT_MATCH_MARKER followed by the `s` key column and SORT_ROW, one line starts with MATCH_MARKER followed by the Driver `s` row, and every other line is byte-identical to help_lines(true).
    - End to end: build the `ctx_with_project` fixture (flag on) and HelpScreen::new(), call Screen::handle_key(KeyCode::Char('s'), KeyModifiers::NONE), then Screen::render into a 100x30 TestBackend. One buffer row contains "> s" plus padding plus "Toggle sort", the cell under that `>` has Modifier::REVERSED, and the popup's top border row contains "s: 1/2".
  </behavior>
  <action>
Per D-01, D-05 and D-06, add the find-a-key path to src/ui/screens/help.rs for one key kind, plain characters, wired through every layer: tokenizer, entry parser, key-handler state, render. Write the behavior tests first in the file's test module, building expected rows through `row()` as the house style requires (D-07).

1. Key type: a private `HelpKey` struct deriving Clone, Copy, PartialEq, Eq and Debug, holding a crossterm `KeyCode` and a `ctrl: bool`. Plain chars have ctrl false.

2. Tokenizer `fn key_tokens(label: &str) -> Result<Vec<HelpKey>, String>` over a key-column label:
   - A label that is exactly "/" is the slash key.
   - Otherwise split on whitespace and drop words that are exactly "/" (they separate alternatives, as in `[ / ]` and `q / Esc`).
   - Split each remaining word on '/', dropping empty pieces, so `j/k` and `g/G` give two keys each.
   - A piece of exactly one char is that char, with case kept (D-01: `G` and `g` are different keys).
   - Any other piece returns Err(piece).
   Task 2 adds named keys to this same function, so write it as one match on the piece and the extension stays additive.

3. Section filter: `const NON_KEY_SECTIONS: [&str; 6]` holding "Filter Syntax", "Owner Work", "Dashboard Badges", "Work State", "Driver Run States" and "Driver Item States" (I-tki-2). Every other heading is a key section.

4. Entry parser `fn help_entries(lines: &[Line<'static>]) -> Vec<HelpEntry>`. `HelpEntry` records the first and last line index, the trimmed label and the keys. Walk the lines by their concatenated span text, the same join `at_section` uses:
   - A non-empty line that does not start with a space is a heading and decides whether the rows after it are in a key section.
   - An empty line ends the current entry.
   - A line starting with two spaces inside a key section is a key row. Its label is chars 2..15, trimmed. Index by CHARS: `row()` pads by chars, and the Mouse section's `Click` + fold-triangle label is multibyte, so the byte slicing wrap_owner_sections uses would mis-split it.
   - An empty label directly after an entry's last line extends that entry. This covers both `row("", ..)` continuations and wrapped_row continuations.
   - A non-empty label starts a new entry whose keys are `key_tokens(label).unwrap_or_default()`.
   Never read the description text (D-01: no substring matching).

5. State: add `highlight: Option<Highlight>` to HelpScreen and initialize it to None in `new()` (`at_section` goes through `new()`). Define `enum Highlight { Found { key: HelpKey, current: usize } }`; Task 2 adds the no-match variant. `current` indexes the list of matching entries in body order.

6. Key handler: leave the existing `?`/Esc, Down, PageDown, Up and PageUp arms untouched in this task. Replace the consume-everything fallback with an arm for `KeyCode::Char(c)`. It computes help_entries(&help_lines(ctx.experimental)) and collects the entries whose keys contain the plain char. With at least one match it sets Found with current 0; with none it sets highlight to None. Either way it sets ctx.needs_redraw and returns ScreenAction::None. Other key codes stay consumed with no effect; Task 2 widens this.

7. Render: factor out a pure `fn apply_highlight(lines, entries, matched, current) -> Vec<Line<'static>>` and call it from render.
   - For every line of every matched entry, replace the leading two-space indent with a two-char ASCII marker: `CURRENT_MATCH_MARKER` ("> ") on the current entry and `MATCH_MARKER` ("* ") on the others (I-tki-4). ASCII keeps the src/ui glyph scan out of it.
   - Rebuild each such line as one span styled Modifier::REVERSED | Modifier::BOLD for the current entry and Modifier::BOLD for the others. Add no color: the marker and the modifier carry the meaning on a monochrome terminal (D-05).
   - Unmatched lines pass through unchanged and the line count does not change, so the recorded ViewportMetrics and the clamp_scroll call in render are unaffected (D-06).
   - Title: while a Found highlight is active the block title is ` Help  <key>: <i>/<n> ` with a 1-based i (I-tki-5); otherwise it stays ` Help `.

8. Key names: `fn key_label(key: HelpKey) -> String`, used by the title. Write it as a match on the KeyCode with the char arm complete now: "Space" for ' ', the char itself when `is_ascii_graphic()`, otherwise `U+` followed by at least four uppercase hex digits. A ctrl key gets the prefix "Ctrl+". Named keys arrive in Task 2. This is I-tki-7 / T-tki-01: no non-ASCII or control char from a key press ever reaches a cell.

Commit when green: `feat(quick-261118-tki): highlight the help rows that list a pressed key`, with a path-scoped `git commit -- src/ui/screens/help.rs`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib -- ui::screens::help</automated>
    <automated>rtk bypass bash -c 'set -e; F=src/ui/screens/help.rs; for t in "fn key_tokens" "fn help_entries" "CURRENT_MATCH_MARKER" "NON_KEY_SECTIONS"; do test "$(grep -v "^\s*//" $F | grep -c "$t")" -ge 1 || { echo "MISSING $t"; exit 1; }; done; echo T1-OK'</automated>
    Gate baselines: fn key_tokens [HEAD=0 EXPECT>=1]; fn help_entries [HEAD=0 EXPECT>=1]; CURRENT_MATCH_MARKER [HEAD=0 EXPECT>=1]; NON_KEY_SECTIONS [HEAD=0 EXPECT>=1].
  </verify>
  <done>The 35 existing help tests and the new tokenizer, entry, highlight and end-to-end tests pass. With the flag on, pressing `s` marks the sort row as current (`> ` plus REVERSED) and the Driver `s` row as `* `, and the title reads `s: 1/2`. The commit is made.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Every listed key is findable, with named keys, ranges, cycling, scroll-to-match and the no-match notice</name>
  <files>src/ui/screens/help.rs</files>
  <read_first>
    - src/ui/screens/help.rs (the Task 1 additions; `more_below` at the end of the production code; `at_section`)
    - src/ui/screens/detail.rs:301-309 (`clamp_scroll`)
  </read_first>
  <behavior>
    - Tokenizer table, one case for every spelling listed in the plan context, Ok with exactly these keys: "j / Down" gives j and Down. "j/k PgUp/PgDn" gives j, k, PageUp and PageDown. "q / Esc" gives q and Esc. "Ctrl+C" gives Ctrl+c. "1-8" gives 1 to 8. "Enter / Space" gives Enter and ' '. "Shift+D" gives 'D'. "Shift+Tab" gives BackTab. "1 2 3 4" gives 1, 2, 3 and 4. "j/k Up/Down" gives j, k, Up and Down. "Up / k", "Left / Esc", "Right / Enter", "Down / Enter", "Tab" and "Esc / q" work the same way. "8-1" and "Click value" are Err.
    - Coverage, for experimental true and false: every entry in a key section either has a NON_KEY_LABELS first word or has key_tokens Ok with at least one key. Every key-section row with a non-empty label has a space at char 15 (the label fits the 13-cell column). Every NON_KEY_SECTIONS heading occurs as a line of the flag-on body. Every NON_KEY_LABELS word is the first word of at least one label in the flag-on body, so no exemption goes stale.
    - Cycling `x` with the flag on: the hints are x: 1/4, x: 2/4, x: 3/4, x: 4/4 and then x: 1/4 again (wrap). The first current row is the Owner block `v o A y x` row. With the flag off the first hint is x: 1/2.
    - Case: `m` matches only the Docs / Sessions `m` row and `M` only the mouse-capture `M` row.
    - Range: `3` gives 2 matches (`1-8` and `1 2 3 4`) and `8` gives 1. `9` gives the title hint `no help entry for 9` and marks nothing.
    - Named keys: Tab gives 1 match, Space 3 and Left 5. F(5) gives `no help entry for F5` and BackTab `no help entry for Shift+Tab`. Char('é') gives `no help entry for U+00E9` and Char('\u{1b}') gives `no help entry for U+001B`.
    - No substring matching: `n` and `N` both give the no-match hint, although many descriptions contain them.
    - Modifiers: Char('D') with SHIFT and Char('d') with SHIFT both make the Driver `Shift+D` row current. Char('c') with CONTROL gives exactly the `Ctrl+C` row (1/1). Plain `c` never marks the `Ctrl+C` row.
    - Start from view: HelpScreen::at_section("Owner Inbox", true) with a recorded viewport, then `g`, makes the inbox row "Go to the project: ..." current with hint `g: 5/5`. HelpScreen::new() followed by `g` makes the Roadmap `j/k g/G` row current with hint `g: 1/5`.
    - reveal_offset, pure cases: a line inside the view leaves the offset unchanged. A line on the last visible row while more_below is true moves the view. A line below the view gives clamp_scroll(line, total, visible). A line past the last page gives the clamp maximum. visible_height 0 gives clamp_scroll(line, total, 0).
    - reveal_offset through handle_key: in a screen whose viewport is (help_lines(true).len(), 20), after each `x` press the current match's first line lies in [offset, offset + 20) and offset equals clamp_scroll(offset, total, 20).
  </behavior>
  <action>
Per D-02, D-03 and D-06 (and I-tki-1, 3, 6, 7, 10 and 11), extend the Task 1 path to every key the key column lists. Write the behavior tests first, with whole rows built through `row()` / `wrapped_row()` (D-07).

1. key_tokens named pieces, exact and case-sensitive:
   - Enter, Esc, Tab, Up, Down, Left, Right, Home and End map to the KeyCode of the same name.
   - Space maps to the char ' '. PgUp maps to PageUp and PgDn to PageDown.
   - `Shift+Tab` maps to BackTab. `Shift+` followed by one ASCII letter maps to that letter uppercased.
   - `Ctrl+` followed by one char maps to a ctrl key on the lowercased char.
   - A three-char piece of the form digit, dash, digit with first <= last maps to every digit in between.
   - Everything else stays Err(piece).
   None of these pieces contains '/', so the Task 1 split is unchanged.

2. Exemptions: `const NON_KEY_LABELS: [&str; 6]` holding "Click", "Double-click", "Wheel", "Shift+drag", "Stale" and "Labels", compared with the label's first whitespace-separated word. These are the mouse gestures and the inbox's pseudo-keys (I-tki-2). Exempt entries get no keys and an `exempt` flag on HelpEntry, so the coverage test can tell an exempt entry from one that failed to tokenize. An entry that is neither exempt nor tokenizable is a test failure, never a silent skip.

3. Pressed-key normalization `HelpKey::pressed(code, modifiers) -> Option<HelpKey>`. It returns None for KeyCode::Modifier(_) and KeyCode::Null, and the state stays unchanged. ctrl is true when modifiers contain CONTROL. Shift is otherwise ignored, except that an ASCII lowercase char reported with SHIFT is uppercased (I-tki-6). Alt is ignored.

4. Highlight gains a `Missing { key }` variant. The fallback arm now handles every key code that reaches it, through pressed():
   - No matches: set Missing. The title becomes ` Help  no help entry for <key> ` and nothing is marked.
   - The same key as the active Found: current becomes (current + 1) % n (D-03 wrap).
   - Any other key: a fresh search. current is the first match whose first line is at or below the clamped top row, or 0 when there is none (I-tki-3).

5. Scrolling: add `fn reveal_offset(offset: u16, line: usize, vp: ViewportMetrics) -> u16`.
   - Let top be clamp_scroll(offset, vp.total_lines, vp.visible_height).
   - The line is visible when top <= line < top + visible_height, except the last visible row while more_below(top, ..) is true, because MORE_INDICATOR paints over that row.
   - Visible: return top. Otherwise return clamp_scroll(line as u16, total_lines, visible_height), which puts the match on the top row, or at the view's maximum near the end (I-tki-10).
   This is the only scroll arithmetic this plan adds. Write no new max-scroll subtraction; clamp_scroll is the single copy (D-06, module doc lines 18-23). After every Found update, handle_key sets scroll_offset to reveal_offset(scroll_offset, first line of the current entry, self.viewport.get()).

6. key_label for named keys: Enter, Esc, Tab, "Shift+Tab" for BackTab, Backspace, Del, Ins, Home, End, Up, Down, Left, Right, PgUp, PgDn, and F<n> for function keys. Any other KeyCode becomes "that key" (I-tki-7).

Run clippy here as well: the new match arms are where an unused variant or a needless clone would show.

Commit when green: `feat(quick-261118-tki): find every listed help key, cycle matches and scroll to them`, with a path-scoped `git commit -- src/ui/screens/help.rs`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib -- ui::screens::help</automated>
    <automated>rtk bypass cargo clippy --locked --all-targets -- -D warnings</automated>
    <automated>rtk bypass bash -c 'set -e; F=src/ui/screens/help.rs; for t in "fn reveal_offset" "NON_KEY_LABELS" "no help entry for"; do test "$(grep -v "^\s*//" $F | grep -c "$t")" -ge 1 || { echo "MISSING $t"; exit 1; }; done; test "$(grep -v "^\s*//" $F | grep -c "total_lines\.saturating_sub")" -eq 0; test "$(grep -v "^\s*//" $F | grep -c "total_lines - ")" -eq 0; echo T2-OK'</automated>
    Gate baselines: fn reveal_offset [HEAD=0 EXPECT>=1]; NON_KEY_LABELS [HEAD=0 EXPECT>=1]; no help entry for [HEAD=0 EXPECT>=1]; total_lines\.saturating_sub [HEAD=0 EXPECT==0 @3266fb5d GUARD MUTATION: reveal_offset computing the last-page offset by hand instead of calling clamp_scroll]; "total_lines - " [HEAD=0 EXPECT==0 @3266fb5d GUARD MUTATION: a plain-minus max-scroll copy in reveal_offset or the find arm].
  </verify>
  <done>Every key-section row with either flag tokenizes or is exempt, and the coverage test proves it. Cycling wraps, the view follows the current match through clamp_scroll, unlisted keys give `no help entry for <key>` with ASCII-only names, and the help tests and clippy are green. The commit is made.</done>
</task>

<task type="auto">
  <name>Task 3: `j`/`k` highlight instead of scrolling, the overlay explains the find, docs follow</name>
  <files>src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md</files>
  <read_first>
    - src/ui/screens/help.rs:1-37 (module doc), :579-582 (the closing DIM hint), :652-663 (adjudication reason), the Down/Up arms of handle_key, and the test `page_up_at_the_top_stays_at_the_top`
    - src/ui/screens/render_escape_guard.rs:3789-3815
    - README.md:253-272, docs/GETTING-STARTED.md:160-171
  </read_first>
  <action>
1. Per D-04, in HelpScreen::handle_key drop the `j` alternative from the Down arm and the `k` alternative from the Up arm, so `j` and `k` fall through to the find arm. Leave the arrow and PageUp/PageDown arms and their UIFIX-04 ordering comments as they are. `?` and Esc keep popping, including while a highlight is active (I-tki-8).

2. Tests:
   - In `page_up_at_the_top_stays_at_the_top`, press KeyCode::Up where it pressed `k`. The test covers the up arm's floor, and `k` no longer reaches that arm.
   - Add `j_and_k_highlight_rows_instead_of_scrolling`. With screen_with_viewport(200, 20), pressing `j` leaves the offset at 0 (the first `j` row is visible) and sets the hint `j: 1/7`. Then pressing Down moves the offset to 1, because the arrows still scroll. Pressing `k` then gives a hint that starts with `k: `.
   - Add an assertion that Esc pops (ScreenAction::Pop) while a highlight is active.

3. Closing hint: replace the one DIM closing line, the one that names j/k as scroll keys, with two DIM lines: "Press ? or Esc to close   Up/Down, PgUp/PgDn scroll" and "Any other key: highlight its rows (again: next match)" (I-tki-9). Both fit the 62-cell inner width of the 80x24 popup. Neither may contain the words the D-28 test `the_help_screen_never_says_sent_received_or_acknowledged` forbids. The body still ends on a non-blank row. Do not quote the retired hint wording anywhere in the file, tests included, because the gate below counts it.

4. Module doc: add a `## Finding a key (quick 261118-tki)` section covering:
   - What a key press does: the marks, the title hint, the cycle with wrap, and the no-match notice.
   - Matching reads key-column tokens only, never descriptions, and is case-sensitive.
   - The token grammar.
   - NON_KEY_SECTIONS, NON_KEY_LABELS and the coverage test that keeps both honest.
   - Which keys can never highlight and why: `?` and Esc close the overlay, Ctrl+C is taken by app.rs before any screen sees it, and Up/Down/PgUp/PgDn scroll. Rows such as `Up` or `PgUp / PgDn` are found through their other tokens or not at all.
   - Markers are ASCII, and style plus marker carry the meaning, never color alone.
   - Every scroll a match causes goes through clamp_scroll via reveal_offset.

5. Adjudication reason: extend the `adjudicate_screen!` reason string. The screen now also draws, in its title, the name of the key the user pressed. key_label builds that name from fixed names and ASCII-graphic chars only, and shows any other char as its U+XXXX code. The reason must not contain the whole tokens `escaped` or `safe` (render_escape_guard's `every_adjudication_reason_is_non_empty_and_names_values_not_verdicts`).

6. Docs (D-08):
   - README.md, the Key bindings table's `?` row (line 268): ↑ / ↓ and `PgUp` / `PgDn` scroll the overlay. Any other key highlights the rows that list it, and pressing it again moves to the next one; the title shows e.g. `x: 2/4`, or `no help entry for <key>`. `Esc` also closes it. The row must contain the phrase "highlights the rows".
   - docs/GETTING-STARTED.md line 169: extend the `?` bullet with one clause saying that a key pressed in the overlay highlights the rows that list it.

7. Commits:
   - `feat(quick-261118-tki): j/k find rows in the help overlay instead of scrolling` via `git commit -- src/ui/screens/help.rs`.
   - `docs(quick-261118-tki): document finding a key in the help overlay` via `git commit -- README.md docs/GETTING-STARTED.md`.
   Do not commit this quick task's PLAN.md or SUMMARY.md; the quick orchestrator does that. Nothing is pushed.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --no-fail-fast -- --quiet</automated>
    <automated>rtk bypass cargo clippy --locked --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo clippy --locked --target aarch64-apple-darwin --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo fmt --check</automated>
    <automated>rtk bypass bash -c 'set -e; F=src/ui/screens/help.rs; test "$(grep -v "^\s*//" $F | grep -cE "KeyCode::Char\(.j.\) \| KeyCode::Down")" -eq 0; test "$(grep -v "^\s*//" $F | grep -cE "KeyCode::Char\(.k.\) \| KeyCode::Up")" -eq 0; test "$(grep -v "^\s*//" $F | grep -c "j/k, PgUp/PgDn scroll")" -eq 0; test "$(grep -v "^\s*//" $F | grep -c "Up/Down, PgUp/PgDn scroll")" -ge 1; test "$(grep -v "^\s*//" $F | grep -c "Any other key: highlight its rows")" -ge 1; test "$(grep -c "Finding a key" $F)" -ge 1; test "$(grep -c "help overlay (.j. / .k. and .PgUp. / .PgDn. scroll it" README.md)" -eq 0; test "$(grep -c "highlights the rows" README.md)" -ge 1; test "$(grep -ci "highlight" docs/GETTING-STARTED.md)" -ge 1; echo T3-OK'</automated>
    Gate baselines: j Down-arm alternative [HEAD=1 EXPECT==0]; k Up-arm alternative [HEAD=1 EXPECT==0]; old closing hint "j/k, PgUp/PgDn scroll" [HEAD=1 EXPECT==0]; "Up/Down, PgUp/PgDn scroll" [HEAD=0 EXPECT>=1]; "Any other key: highlight its rows" [HEAD=0 EXPECT>=1]; "Finding a key" [HEAD=0 EXPECT>=1]; README old `?` wording [HEAD=1 EXPECT==0]; README "highlights the rows" [HEAD=0 EXPECT>=1]; GETTING-STARTED "highlight" (case-insensitive) [HEAD=0 EXPECT>=1].
  </verify>
  <done>`j` and `k` highlight and the arrows still scroll. The closing hint, module doc, adjudication reason, README and GETTING-STARTED describe the find. The full suite (--no-fail-fast), clippy on all three targets and fmt are clean, and both commits are made.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Terminal key events -> help title | A key press arrives as a crossterm KeyCode. Its name is now drawn into the popup title, so input reaches the frame for the first time on this screen. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-tki-01 | Tampering | `key_label` -> block title (terminal output) | low | mitigate | Only fixed key names and `is_ascii_graphic()` chars are drawn. Any other char (ESC, other control chars, wide or combining chars) is shown as `U+XXXX`, so no control sequence or width-ambiguous cell enters the frame. Unit-tested with Char('\u{1b}') and Char('é') in Task 2. |
| T-tki-02 | Denial of service | help_lines/help_entries recomputed per key press and per frame | low | accept | The body is about 200 authored lines; tokenizing it is microseconds, and the same body is already rebuilt on every frame today. |
| T-tki-03 | Information disclosure | help body | low | accept | The overlay still draws only authored text plus the pressed key's name. No project state or identity is read (render_escape_guard's HelpScreen probe stays as it is). |
| T-tki-SC | Tampering | npm/pip/cargo installs | low | accept | No package is installed and no Cargo dependency is added (crossterm and ratatui are already dependencies). |
</threat_model>

<verification>
Full regression after the last commit. Use `rtk bypass` so rtk cannot filter the results.
- `rtk bypass cargo test --locked --no-fail-fast -- --quiet` exits 0. Report the summed raw `test result:` counts. The help module had 35 tests at HEAD and must have more now.
- `rtk bypass cargo clippy --locked --all-targets -- -D warnings` exits 0, and so do the `x86_64-pc-windows-gnu` and `aarch64-apple-darwin` targets (release.yml cross-target-check).
- `rtk bypass cargo fmt --check` is clean.
- The glyph scan (src/ui/glyphs.rs tests) and render_escape_guard tests pass as part of the full suite.
- `git log --oneline -4` shows the four commits (feat, feat, feat, docs), each made with a path-scoped `git commit -- <paths>`. `git status --short` lists nothing except this quick task's PLAN.md/SUMMARY.md.
</verification>

<success_criteria>
- In the help overlay, any key a key column lists marks its rows (ASCII marker plus style), the title counts them, repeat presses cycle and wrap, and the view follows through clamp_scroll.
- Unlisted keys give `no help entry for <key>` with an ASCII-only name. Descriptions are never matched.
- `?`/Esc close, Up/Down/PgUp/PgDn scroll, and `j`/`k` find.
- The coverage test fails on any future key row that cannot be tokenized and is not exempt.
- README and GETTING-STARTED describe the behavior. The full suite, the three clippy targets and fmt are clean.
</success_criteria>

<output>
Create `.planning/quick/261118-tki-help-overlay-key-highlight/261118-tki-SUMMARY.md` when done. Record D-01..D-08 as inferred (orchestrator) and I-tki-1..11 as inferred (planner), the keys that can never highlight (`?`, Esc, Ctrl+C, Up, Down, PgUp, PgDn) and why, the final match counts the tests pin, and the raw test counts.
</output>
