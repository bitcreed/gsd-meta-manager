# Phase 37 — UI Review

**Audited:** 2026-11-15
**Baseline:** 37-UI-SPEC.md (approved 2026-11-14), plus 37-CONTEXT.md D-01..D-23
**Screenshots:** not captured. This is a ratatui terminal UI with no browser or Playwright target. The audit used the pre-rendered ASCII TestBackend snapshots (inbox default lanes, after `a`, the `A` popup, the help overlay, each at 100x40 and 140x40; Phase 36 dashboard and detail renders) and a code read of `inbox.rs`, `owner_actions.rs`, `ack_confirm.rs`, `owner_file_view.rs`, `help.rs`, `owner_block.rs` (shared formatters) and their tests. ASCII snapshots carry no color or modifier data, so cursor, `stale`, `ws` prefix, DIM and REVERSED were judged from code and tests only. The snapshots show a real fleet; nothing from them is copied here (rows are described generically).
**Widths covered:** 100 and 140 from snapshots. 80 and 120 from the tests only (`layout_tiers_140_120_100_80`, `footer_ladder_matches_the_ui_spec`, `degrade_never_panics`); not re-rendered in this audit.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Status, empty and popup copy match the contract string for string; new help rows overflow the overlay and the `v` row is cut even at 140 columns |
| 2. Visuals | 2/4 | On the real fleet at 100 columns 25 of 34 visible rows lose their phase identity (`ws v…`, `ph 1…`) and 27 of 34 carry an ellipsis, so the contract's SC2 legibility claim does not hold outside the synthetic fixture |
| 3. Color | 3/4 | Quiet-color contract is exact and test-pinned; the one consent-critical line (popup argv) is DIM-only, and the active lane is signaled by REVERSED alone |
| 4. Typography | 3/4 | Role set (BOLD / DIM / REVERSED, registry glyphs) is disciplined; truncation rules, not role choice, hurt legibility |
| 5. Spacing | 2/4 | Column budget is spent on columns that are empty in the visible lanes while the identifying cells are cut; 36 idle cells at 140 columns beside a name cut at 32 |
| 6. Experience Design | 3/4 | State coverage is strong (loading vs unreadable, identity cursor, bounded child, unconfirmable clipped popup); help context, viewer affordances and status lifetime are weak |

**Overall: 16/24**

No pillar is a 1 and no defect breaks task completion (`v`, `A`, `y`, `o`, `g` all work and refuse safely), so there are no BLOCKERs. Everything below is a WARNING unless stated.

---

## Top Priority Fixes

1. **WARNING (highest): the phase cell collapses to its 5-cell floor on a realistic fleet, so rows lose the one cell that says which phase they are** — At 100 columns the Test section shows 20 workstream rows that all read `ws v…` and 5 decimal-phase rows that read `ph 1…`; the phase number is gone entirely, which contradicts the spec's "phase number last" rule (`grid_for_inbox` shrinks `phase_w` to 5 before `alias_w` or `name_w`; `phase_cell` then has no room for the number). Meanwhile one long-alias outlier holds the alias column at 18 for every row, and the phrase column keeps 12 empty cells in a lane where no row has a phrase. The owner has to expand each row (`Enter`, read the `run` line) to learn the phase. Fix: make the phase cell's floor the width of its widest phase text (`ph NN.N`, about 7 plus the `ws ` marker) and shrink the ws name, alias and name before it; or reorder the narrowing to phrase, alias, name, phase cell. Also size Phrase to 0 when no row of the active filter has one (the filter-reflow concern in the spec is about columns appearing and disappearing, so keep the width stable per lane or accept a reflow only when switching lanes).
2. **WARNING: help opened from the inbox documents the dashboard, and the new rows overflow** — `?` from the inbox opens the shared help at the top: `Enter  Open project detail`, `a  Add project`, `q  Quit`, none of which is true on this screen. The `Owner Inbox` section is many screens below (after Filter Syntax and Owner Work). Of the 11 rows the phase added, 8 are wider than the 78 usable cells at 100 columns (97 to 146 cells) and are cut mid-sentence with no `…` (the `v` row is 146 cells, cut even at 140). The overlay has a hand-wrapped continuation convention (see the empty-key `row("", …)` lines near `help.rs:413`) that the new rows ignore. Fix: open help scrolled to (or pre-filtered to) the section for the calling screen, and hand-wrap rows longer than about 76 cells.
3. **WARNING: the `A` popup wraps its two most important lines badly** — The `File` label sits alone on a line and the project-relative path starts at column 0 of the next, hard-broken mid-token (`…18-V` / `ERIFICATION.md`). The argv line puts `node` alone, then the tools path, broken inside `gsd-` / `tools.cjs`. This is the line the owner is asked to verify before pressing Yes, it is DIM, and it is the one that wraps worst. Fix: break at `/` and spaces with a hanging indent under the value column, keep `File` and the value on one line when the value fits after the label, and consider normal weight for the argv (DIM is for decoration, this is the evidence for consent).
4. **WARNING: the read-only file viewer (`o`) has no on-screen help, no scroll position and no wrap** — `OwnerFileViewScreen::render` draws a bordered `Paragraph` with the title only: no `[Esc]back  [j/k]scroll` hint, no `n-m of N`, no "more below" cue, and no `Wrap` or horizontal scroll, so any line wider than the frame (UAT and VERIFICATION check lines routinely are) is silently cut at the border. For a screen whose purpose is to read the check text that is the content loss that matters. Fix: add a footer row with the keys and `{first}-{last} of {total}`, and either wrap at the inner width or mark cut lines with `…`.
5. **WARNING: Decide and Fix are invisible on the first screen** — With the default lanes the first 35 body rows are all `TEST — closes a phase` (74 rows in total), so the header's `decide 16 · fix 11` has nothing on screen to point to. Section titles carry no counts and there is no jump key. Fix: put row counts in the section titles (`DECIDE (14)`) or add a one-line lane strip; at minimum show the lane sizes in the scroll hint. Needs human judgment whether D-05's ordering should still put 74 rows ahead of the first Decide row.
6. **WARNING: the new Owner block keys are undiscoverable** — D-21 made `v o A y` act in the detail Owner block but froze its footer (`owner_block_footer_spans`: move, expand, folds, phases, quit, help), so the only place the keys are advertised is the help overlay. Fix after Phase 36 UAT lands: add `[v]run  [A]ck` to that footer or a short rung.
7. **WARNING: feedback outlives neither the work nor the failure** — Status messages expire after 3 s (`app.rs`), but acknowledge may run 10 s and its failure line (`Acknowledge failed: …`, capped at 120 chars, cut again at the terminal width) is the owner's only on-screen explanation. The spec records this as an accepted limit; it is still the weakest point of the `A` flow. Fix: keep an acknowledge result live until the next key, or until the busy file clears, for error outcomes.
8. **WARNING (test gap): the legibility and color tests cannot see the defects above** — `sc2_legible_at_100x30` and `quiet_color_at_100x30` run on a synthetic fleet with short workstream names (`ws w1`), integer phases and aliases of 9 cells or fewer. Add a fixture shaped like a real fleet (long alias outlier, 20+ workstream rows with 12+ cell names, decimal phases, long titles) and assert that the phase number survives at 100 and 80 columns.

Minor: Triage is a cursor stop drawn with the `▸` expand marker but never expands (`Enter` refuses). The lane-filter state is conveyed only by REVERSED (the spec admits this). Footer rung 3 (80 to 93 columns) drops `[Esc]back`, so the way out is not shown at the width where the screen is most cramped.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

Matches the contract:
- Header, section titles, filter empty copy (`No {lane} work · press {key} again for the default lanes`, `No owner work in any lane`), `No projects registered. Press Esc, then a to add one.`, `Loading...` all verified in `inbox.rs:115-118, 1026-1037` and pinned by `empty_and_loading_states`.
- Status copy in `owner_actions.rs` matches the UI-SPEC table line for line: `No command for BLOCKED work: unblock it first` (66), `Launched {command} in {alias}` (129), `Cannot open {file}: not readable` (214), `Already acknowledged: {name}` (328), `Cannot acknowledge a handoff: gsd-core has no category for it` (336), `Acknowledged {name} via gsd-core (uncommitted)` (787), `Too large to copy ({N} KiB): narrow the lane with 1-4` (537), and the OSC 52 line says the sequence was sent and must be allowed (576), never "Copied". The popup body copy (`Hides all {N} items of this file in every lane, blocked rows included. The marker lapses…`, `gsd-core edits the file's frontmatter. Nothing is committed…`) is verbatim.
- Singular and plural forms covered (`1 check`, `1 project`, `1 item`, `1 line`). No singular "they".

Findings:
- **Help rows overflow (see fix 2).** Lengths of the new `Owner Inbox` rows including the 2-space indent and 13-cell key column: `1 2 3 4` 101, `j/k Up/Down` 88, `Left / Right` 89, `g` 97, `v` 146, `A` 95, `y` 106. The dashboard `i` row (`…across every project (decide, test, fix)`) is cut at 100 columns and loses `(decide, test, fix)`. The `v` row loses "you type the command there" even at 140, which is the only place that explains the focus-instead-of-launch behavior.
- **Divergence, documented and defensible:** failure copy uses the last non-empty stderr line instead of the first (37-06 summary, RESEARCH Pitfall 7). Recorded, not scored down.
- **Footer rung 3 `[A]ck`** was flagged non-blocking by the checker; it appears at 75 to 93 columns and again at rung 4. Acceptable, but `[A]cknowledge` fits at 94 so the abbreviation is only ever seen on narrow terminals where the key matters most for a consent action.
- `No owner work` has no next step (D-22, accepted).
- Needs human review: whether `Triage 234` at rung 3 reads as a count of todos or something else without the word "todos".

### Pillar 2: Visuals (2/4)

Focal point and hierarchy are sound: BOLD header and section titles, a full-width DIM rule that separates the pinned region, a single Cyan+BOLD cursor row with a `>` mark, DIM column heads, `▸`/`▾` markers from the glyph registry. Header numbers equal the CLI summary. Columns align: in the 100-column snapshot the `checks` head ends at the same cell as every `N checks` and the `age` head sits over the Age column.

Defects, from the snapshots:
- **Phase identity lost at 100 columns.** 34 body rows are visible; 27 contain `…`; 25 of those are in the phase cell (`ws v…` on 20 rows, `ph 1…` on 5 decimal-phase rows). Rows differ only by name. At 140 columns the ws name is still cut (`ws v0-13-pu…`) but the phase is back (`ph 22`). The synthetic fixture the spec was drawn from never produced this.
- **Names cut at 32 with room to spare.** At 140 columns long titles end in `…` at the 32-cell name cap while about 36 cells to the right (the reserved Phrase column) are empty on every row of the visible section, then Age sits 36 cells after `checks`. The eye has to cross a wide gutter to pair a check count with its age.
- **Alias outlier.** One long alias clamps the alias column at 18 and truncates itself (`…-r…`) at both widths, while every other alias is 7 to 11 cells; the remaining cells are dead space on all rows. At 140 the cap of 18 truncates it although the screen is 140 wide.
- **Lane overview.** Only one section is visible on the first screen (fix 5).
- **Popup (100 and 140):** frame, title and Yes / No row are clear, and the dimmed background keeps the rows readable behind it. The `File` label orphaned on its own line and the mid-token breaks are visible defects (fix 3). The popup is 72 cells wide at both screen widths, so it covers the phase and name columns of the rows behind it, which is acceptable for a modal.
- **File viewer:** no footer, no scroll cue, no wrap (fix 4). It reuses the Docs renderer, so its heading and code colors fall outside the inbox's "quiet" palette; acceptable as reuse, but it is not covered by `quiet_color_at_100x30`.
- **Help overlay:** the `▾ more` indicator works; rows are clipped without an ellipsis (fix 2).

Needs human review (not scored): how the cursor row reads in the user's theme (Cyan+BOLD with DIM removed over the whole row), and how DIM-only BLOCKED and Acknowledged rows read in a low-contrast theme.

### Pillar 3: Color (3/4)

- Contract is met exactly. `quiet_color_at_100x30` asserts that Cyan appears only on the cursor row (every non-blank cell), Yellow only on `stale` words of non-cursor rows, Magenta only on `ws ` prefixes of non-cursor rows, and any other color panics the test. No Red or Green on the screen. The popup accent is Yellow via `render_confirm_popup`. Hardcoded colors are literals at the draw site, as in 36; no new color. Color is never the only carrier: `stale` is a word, the cursor is also `>`, state is also in titles and markers.
- Deductions:
  - The popup's exact argv, the line the consent decision rests on, is DIM (`ack_confirm.rs`, the final `Span::styled(argv, …)`); DIM on some terminals is close to invisible. Needs human review in a real theme.
  - The active lane is signaled by `REVERSED` only. Tests count exactly one REVERSED token, but in a terminal that ignores the attribute (or in a copied transcript) `[a]ll` after pressing `a` looks identical to the default; the only other cue is the section list and the `n-m of N` hint. The spec acknowledges this.
  - The file viewer inherits Docs renderer colors with no test pinning them.

### Pillar 4: Typography (3/4)

- Roles are consistent and match the contract: BOLD for `Owner inbox`, section titles and `[key]` tokens; DIM (`muted_style`/`dim()`) for age, heads, rule, expansion detail, BLOCKED and Acknowledged, hints; REVERSED only on the active lane token (`inbox.rs:942-967`). No ITALIC or UNDERLINED. All glyphs (`▸ ▾ ── — · … ← →`) come from the registry or the typography set; the glyph scan test is in place.
- Deductions: cell-level text rules produce `ws v…` / `ph 1…` / `(…` fragments that no typographic role can rescue (Pillar 2); the `—` in section titles and `·` in headers and phrases are East-Asian-Ambiguous, which the spec records as UAT item 1 (needs human review on the owner's font).
- The BLOCKED section title is DIM, as contracted; it is the only section title without BOLD. Needs human review whether that reads as "not yours" or as "disabled".

### Pillar 5: Spacing (2/4)

Contract geometry is implemented as written: header, spacer, scrolling body, full-width rule with scroll hint at its right end, pinned fold line, footer; ladders for header (six rungs), fold line (three rungs) and footer (four rungs); `degrade_never_panics` covers 80, 79, 60x20, 40x10, 20x5, 1x1 and 0x0. Expansion indent (5 / 8) and `run` line are tested.

Defects:
- `grid_for_inbox` (`inbox.rs:608-654`) computes widths over every current row of every lane, as the spec requires, so a single outlier in a lane that is off screen dictates the layout of the visible one: alias 18 from one project, Phrase reserved 12 to 32 cells from Decide and Fix rows that are 40 rows down, Age at a fixed 11 cells for values of 3 to 6 cells.
- The narrowing order (`phrase` to 12, then `phase`, then `alias`, then `name`) shrinks the phase cell first of the identifying columns, down to a floor of 5 that cannot hold `ws x… ph NN` or `ph NN.N`. The spec's worked-width table assumed the fixture's 7-cell aliases and no workstream rows, so its arithmetic ("no `…` anywhere at 100") was never true for a real fleet.
- At 140 the 36-cell Phrase gutter sits empty beside a name capped at 32 (cap too low when there is room).
- The Command column is correctly dropped below 126 columns, but at 140 the first screen has no FIX row to show it; not verifiable from the snapshots.
- The one-cell left margin and the borderless layout are fine.

### Pillar 6: Experience Design (3/4)

Strengths:
- Loading versus unreadable are distinct states (`DigestState`, `OwnerDigestFailed`, header clauses `· {M} loading` / `· {U} unreadable`); absent data is never counted as clean; `No project could be read` has its own copy.
- Selection is by identity and re-anchors after every digest change (next row, then previous, then fold line); SC3 tests cover a passed UAT and an acknowledged file; a vanished expanded group is dropped.
- `A` is validated before the popup, defaults to No, shows the exact argv and the file-level consequence, re-resolves the group on Yes and refuses on any change, is unconfirmable when clipped (60x20 test), runs a bounded, environment-scrubbed child, never commits. `v` re-validates the phase token and workstream, and the argv is one element. `o` is contained to `.planning/`, regular file, 1 MiB, never an editor. `y` has a bounded tool call, an overlap guard and an OSC 52 fallback worded honestly. Modifier chords are not action keys (WR-01). Every dynamic string goes through `shown` and the escape-guard census covers inbox, popup, viewer and status line.
- All refusals are status lines, never silent; keys on the fold line and on an empty inbox do nothing safely.

Weaknesses:
- Help is not contextual (fix 2).
- The viewer has no affordances or wrap (fix 4).
- Status copy lives 3 s on one line; acknowledge can take 10 s and failure detail is cut to the terminal width (fix 7). There is no persistent busy cue while the child runs, other than the status message that is already gone.
- Triage is a stop that offers `▸` but never expands.
- Mouse is deliberately absent (README says keyboard-only); acceptable per spec.
- Footer discoverability in the Owner block (fix 6).
- Needs human review: whether 74 rows before the first Decide row (default order) is the right first impression; `v` into the real multiplexer under tmux and herdr and the real acknowledge round trip (the three D-23 UAT items) are not auditable from snapshots.

---

## needs_human_review (not scored as defects)

1. Cursor row, `stale`, `ws` prefix, DIM rows and REVERSED lane key in the owner's real theme (snapshots carry no attributes).
2. Legibility of DIM for the popup argv, BLOCKED rows, Acknowledged rows and the Triage fold.
3. `·` and `—` alignment on the owner's font (ambiguous-width backstop, D-23 item 1).
4. Default ordering and the first screen showing only closes-a-phase Test rows on the real fleet (D-23 item 1).
5. 80 and 120 column renders on the real fleet (only tests were consulted for those widths here).
6. Multiplexer launch, live-session focus and the acknowledge round trip (D-23 items 2 and 3).

## Registry Safety

Not applicable: `shadcn_initialized: false`, ratatui widgets only, no third-party registries.

## Files Audited

- `.planning/phases/37-inbox-screen-and-actions/37-UI-SPEC.md`, `37-CONTEXT.md`, `37-01..07-SUMMARY.md`, `37-REVIEW-DISPOSITION.md`
- `src/ui/screens/inbox.rs` (grid, row, title, header, footer and fold ladders, empty copy, tests)
- `src/ui/screens/owner_actions.rs` (status copy)
- `src/ui/screens/ack_confirm.rs` (popup body)
- `src/ui/screens/owner_file_view.rs` (viewer render and keys)
- `src/ui/screens/help.rs` (Owner Inbox rows, overlay render)
- `src/ui/screens/owner_block.rs` (`phase_cell`, `style_for`, `cursor_style`, shared formatters)
- `src/ui/screens/detail.rs` (`owner_block_footer_spans`)
- `src/archive.rs` (`render_markdown_lines`, viewer rendering)
- Snapshots: `phase37-screens.txt` (100x40 and 140x40: default, after `a`, `A` popup, help) and `phase36-screens.txt` (dashboard and Owner block, frozen)

---

## Fixes applied

Applied after the audit; each is one `fix(37)` commit with its tests (the legibility test uses a synthetic real-shaped fleet: long alias, 30 workstream and decimal-phase rows, long titles; no real project text).

1. **Phase identity (Top fix 1, test gap 8).** `grid_for_inbox` now narrows the alias (to 10, then 8) and the name (to 16, then 12, then 8) before the phase cell, and floors the phase cell at its phase text plus a trimmed workstream name (12, 8, 4, 2 cells kept in turn; 5 only as a last resort). `RowCells::phase_cell_floor` is additive; `phase_cell` and the Owner block are untouched. **Deviation from the UI-SPEC narrowing contract (D-04: phrase, then phase, then alias, then name):** the contract shrank the phase cell to 5 first, which is what removed the phase number on a real fleet; the order is now phrase, alias, name, phase. `grid_for_inbox_matches_the_ui_spec_table` changed one value (60 columns: phase cell 7, not 5). The phrase column is still sized over every lane (no reflow on a lane switch), so the empty phrase column and the Age width are unchanged; phrase still gives way first, down to 12.
2. **Help (Top fix 2).** Owner rows wider than 76 cells (the Owner Work and Owner Inbox sections, plus rows that document the Owner block or the inbox) are hand-wrapped with an empty-key hanging indent (`wrapped_row`); other sections keep their authored rows. `?` from the inbox opens `HelpScreen::at_section("Owner Inbox")`.
3. **`A` popup (Top fix 3).** Label and value share a row; paths and the argv break at `/` and spaces under a hanging indent (11 cells for fields, 2 for the argv); the argv is drawn in normal weight, not DIM. The argv content is unchanged.
4. **`o` viewer (Top fix 4).** Lines wrap at the inner width (word break, style kept; the 1 MiB and line caps stay), and a footer row shows `[Esc]back  [j/k]scroll  [PgUp/PgDn]page  [Home/End]` (laddered by width) with `first-last of total`. Page size is now the body height minus one.
5. **IN-02 (37-REVIEW-FIX).** The `v` launch runs `env -u VAR ... claude <cmd>` for every `ACK_ENV_REMOVE` variable, for the multiplexer and GUI-terminal backends (argv pin tests updated; the README does not document the argv).
6. **Status lifetime (Top fix 7), the acknowledge outcome only.** The line is stamped 9 s ahead, so it lives 12 s; other messages keep 3 s. A persistent-until-key policy was not added.

Not changed (human UAT items): default lane ordering (D-02), the frozen Owner block footer (fix 6) and lane counts (fix 5).
