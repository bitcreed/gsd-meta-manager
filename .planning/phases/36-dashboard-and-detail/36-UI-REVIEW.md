# Phase 36 — UI Review

**Audited:** 2026-11-15
**Baseline:** `36-UI-SPEC.md` (approved design contract), locked decisions in `36-CONTEXT.md`
**Screenshots:** not applicable. This is a ratatui TUI with no browser or dev server. The audit read the source,
the in-file `TestBackend` tests, and a set of pre-fix real-fleet ASCII renders (a 15 to 17 project fleet at 100 and
140 columns: dashboard, plus the detail Phases-tab Owner block for several projects). The renders predate the
code-review fixes, so each observation taken from them was re-checked against the current code before it was
reported. Nothing from the renders (names, paths, titles) is copied here.
**Review fixes:** all 12 findings of `36-REVIEW.md` (WR-01..WR-07, IN-01..IN-05) are recorded as fixed
in `36-REVIEW-FIX.md`. None is re-reported here.
**Tags:** every finding is marked `BUG` (objective deviation from the spec or incorrect behavior, verifiable in
code) or `SUBJECTIVE` (judgment, collected again under "Human Review Items"). Severity: BLOCKER breaks a
user task; WARNING degrades quality.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Wording follows the contract and the CLI vocabulary; one FIX row repeats its own check count as its status phrase, and a few documented deviations remain |
| 2. Visuals | 3/4 | Clear hierarchy (tag, phase, name, phrase, age, command); the header ladder is clipped mid-clause by the overflow hint, and workstream rows lose their phase number |
| 3. Color | 4/4 | Exactly the four reserved accents plus an explicit `Reset` on the `You` cell; no Red, no new colors |
| 4. Typography | 3/4 | Only BOLD/DIM modifiers, glyphs from the registry; ambiguous-width `·` and `→` sit in a table cell, and a status token is cut mid-word |
| 5. Spacing | 3/4 | Tier table matches the spec exactly and is tested; the 100-column tier is cramped and the detail command column can collapse to zero cells at exactly 100 |
| 6. Experience Design | 3/4 | Loading, empty, error and fail-safe states are covered and the focus model is tested; footer advertises an unreachable Owner level, and `[i] inbox` is a dead end for now |

**Overall: 19/24**

No BLOCKER was found. Five BUG findings at WARNING severity, three minor BUG findings and the SUBJECTIVE items below are open.

---

## Top 3 Priority Fixes

1. **BUG (WARNING), Owner block header is clipped mid-clause by the overflow hint.** When body rows are
   clipped the hint (` 1-12 of 34`) takes the right edge of the header row, but the header rung was chosen
   for the full block width. A rung that fits at width W is then cut by `fit_spans` to W minus the hint, so the
   user reads a half clause such as `fix 1 (ph 11…` right next to the hint. This is precisely the case the
   "whole ladder form, never clip mid-segment" rule exists to prevent, and it appears on every project with more
   rows than the cap at 100 columns. Fix: in `block_model` (`owner_block.rs:1025`) pick the header rung for
   `width - hint_cells` whenever the body will be clipped (the hint width is bounded: `" {first}-{last} of
   {total}"` is at most about 14 cells), or reserve the hint width before running `header_line`
   (`owner_block.rs:213`).
2. **BUG (WARNING), a workstream row's phase cell drops the phase number.** The phase cell is clamped to 18 cells
   (`owner_block.rs:555`) and `fit_cells` cuts from the tail (`:643`), so `ws <name> ph N` loses `ph N`, the
   only part that tells two rows of one workstream apart, whenever the workstream name is longer than about 10
   cells. The UI-SPEC's own example `ws v0-14-voice ph 26` is 20 cells and cannot be drawn under its own clamp.
   Below 100 columns there is no command column to recover the phase from, so several rows read identically.
   Fix: truncate the workstream name inside the prefix (`ws v0-14-v… ph 26`), keeping ` ph N` intact, or
   raise the clamp for rows that carry a prefix.
3. **BUG (WARNING), a FIX row can repeat its check count as its status phrase.** `inbox::detail` falls back to
   `plural(counts.checks, ...)` for a FIX group with no failed and no unreadable checks (`inbox.rs:374-376`),
   and the Owner block also fills the Checks column from the same count, giving `7 checks  7 checks` on one row.
   The phrase carries no information and hides why the group is in the FIX lane (the contract wants
   `{N} failed check(s), {M} gap(s)` and the other named phrases). Fix: in the block's `row_cells`, for a FIX
   group whose `detail()` would be that fallback, show a blank phrase (or the verification status) instead of
   repeating the count.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

Verified against the contract:
- Lane words and tags (`decide`, `test`, `fix N ph`, `fix N`, `→ closes M`, `(+N old)`, `·`; `TEST`,
  `DECIDE`, `FIX`, `BLOCKED`), `closes phase`, `{N} checks`/`1 check`, the fold strings, the `Loading...`
  and ` · M loading` summary states, the `i` status message and the help rows are all present, exact, US
  spelling and without singular "they". Zero/one/many nouns go through `inbox::plural` and `noun`.
- The block's wording comes from one shared formatter (`inbox.rs` `closes_after_clauses`, `detail`,
  `blocked_tally`), so the CLI and the TUI cannot drift. This was a spec requirement and it is met; the CLI
  goldens are unchanged.

Findings:
- **BUG (WARNING)** FIX phrase duplicates the Checks column when the group has no failed or unreadable check
  (`inbox.rs:374-376` through `owner_block.rs:314`, the `counted` and `phrase` cells). See Top fix 3.
- **BUG (minor)** The help overlay is drawn without wrapping (`help.rs:628`, no `Wrap`) in a popup 80 percent of
  the terminal wide. The new `fix N ph` row is 118 cells (13-cell key column plus a 105-cell description), the
  `s` row 107, `(+N old)` 104. The popup's inner width is 78 at 100 columns, 94 at 120 and 110 at 140, so these
  rows are cut on the right at every width the phase targets, and the `fix N ph` row even at 140. The pattern
  predates the phase (9 of 86 old rows already exceed 94 cells), but the phase added 3 more and the spec asked
  for "keys at most 13 cells" without limiting description length. Fix: shorten the descriptions to about 90
  cells or wrap the help paragraph.
- Accepted deviations, recorded in the SUMMARYs (not defects): header clause reads `not built ph 13 14` (CLI
  parity) instead of the spec's `not built: ph 13 14`; the `i` help row reads `Owner inbox (for now: shows how
  to run gsd-meta-manager inbox)` instead of the spec text that promised an inbox screen; unreadable owner
  files count as `fix` (reader behavior) where the spec said `test`. All three are consistent between code,
  help and README.
- **SUBJECTIVE** `open investigation (root_caus…` is cut at the 30-cell phrase clamp in the middle of the status
  token; the user cannot tell which status it is. See Human Review Items.
- **SUBJECTIVE** `[i] inbox` is the most prominent right-aligned element of the summary line, but until Phase
  37 it only prints a hint in the status bar. Intended (D-12), recorded for the human pass.

### Pillar 2: Visuals (3/4)

Strengths: one unmistakable focal point per surface. On the dashboard the `You` column sits between Status and
Progress with a quiet, left-aligned cell and a fleet summary row directly under the table. In the detail view
the block is a flat header, aligned row grid and a pinned fold line above the existing 40/60 split, and it
degrades by whole columns (command, then age) rather than by mid-field clipping. BLOCKED rows are DIM so
non-actionable work recedes; bold lane tags give a clean left scan line.

Findings:
- **BUG (WARNING)** Header vs overflow hint collision (Top fix 1). Present in the pre-fix renders and unchanged
  in the code: `block_model` selects the rung for the full width (`owner_block.rs:1025`), and
  `render_owner_block` cuts it with `fit_spans` (`:1124`) after reserving the hint.
- **BUG (WARNING)** Workstream rows lose `ph N` (Top fix 2). In a multi-workstream project at 100 columns a
  whole run of TEST rows differs only in the truncated tail of a name, which defeats the one-row-per-phase
  scan.
- **BUG (minor)** The column grid is computed from every row of the block (current rows plus open folds,
  `owner_block.rs:552-560`, `block_model` collecting `all`), not from the visible rows as the spec states
  ("computed per frame from the visible rows"). The phrase column is sized by the longest phrase anywhere, so a
  short visible window can show `closes phase` followed by a large blank gap before the age. The choice keeps
  columns stable while scrolling, which is defensible, but it is an unannounced deviation; the SUMMARYs do not
  list it.
- **SUBJECTIVE** The block and the phase list have no visual separation: the pinned fold line is directly
  followed by the ` Phases │ Phases` header row, and the block header starts at column 0 while every other tab
  line carries a one-cell margin. Only the BOLD `Owner` and the dim `──` mark the section.
- **SUBJECTIVE** On the dashboard at 100 columns the Status cell (13 cells, clipped) runs straight into the
  `You` cell with one blank: a clipped status such as `Awaiting next` followed by `decide 1` reads as one
  phrase. At 140 columns Status clips at 18 cells with the same effect.

### Pillar 3: Color (4/4)

Counted against the spec's reserved-accent list, across `owner.rs`, `owner_block.rs` and the new `normal.rs` and
`detail.rs` lines:
- `Color::Reset` is set explicitly on every span of the `You` cell and the summary (`owner.rs:83-90`); a test
  asserts it on rows of every row color. No Red and no Yellow in either; no `Color::` literal other than the four
  below is introduced.
- Green: one site, the `closes phase` phrase (`owner_block.rs:690`). Yellow: one site, the word `stale` in an age
  cell (`:621`). Magenta: one site, the `ws <name> ` prefix (`:646`). Cyan: one site, the cursor row
  (`cursor_style`, `:1176`) while the block has focus. That is the whole accent set the contract lists.
- Meaning survives without color: every state has a word (`closes phase`, `stale`, `awaiting your
  confirmation`, lane tags, `old`).
- Modifiers: BOLD for tags and labels, DIM for muted copy and BLOCKED/Acknowledged rows; no `ITALIC`, no
  `REVERSED`, and `UNDERLINED` only in the existing dashboard selection.

Findings (none breaks the contract):
- **SUBJECTIVE** On a project whose TEST rows nearly all close a phase the Green `closes phase` repeats once per
  row and can outweigh the cursor accent, which is the intended use of a quiet palette in the opposite
  direction. Spec-driven; check on the real fleet.
- The cursor row patches Cyan over Green, Yellow and Magenta, so those cues disappear on the focused row.
  Matches the contract (the cursor is the only reserved accent while focused); noted, not a defect.

### Pillar 4: Typography (3/4)

Strengths: three modifiers only (BOLD, DIM, plus the existing UNDERLINED selection); `fit_cells` with `…` is the
only truncation helper in the new code; display widths use `Line::width()`; `▸`/`▾` come from
`glyphs::TRIANGLE_*` and the registry glyph scan passes; the separator is one constant (`OWNER_SEP`) as the
spec required, so a swap to the one-cell `∙` is a one-line change.

Findings:
- **SUBJECTIVE** U+00B7 and U+2192 are East-Asian-Ambiguous and are in a table cell (`owner.rs:25-33`). The
  spec locks them (D-07) and records the risk; on an ambiguous-wide terminal the Progress and Backlog cells
  after `You` would shift. Unverified without the human check (UAT item 2).
- **BUG (minor)** The `You` cell ladder has a fourth rung (first lane alone, `owner.rs:162`) that the spec does
  not list; the spec's rung 4 is `fit_cells(lanes, cells)`. For a cell where the two-lane form does not fit
  (three-digit counts) this drops the second lane silently and without `…`, so `decide 1234` reads as if
  nothing else waits (the summary still counts it). Recorded as an inferred decision in 36-01. Cannot occur
  below four-digit counts, so impact is low.
- **SUBJECTIVE** `▸ Todos N` carries the expandable-fold marker although Todos cannot be opened (the cursor
  skips it). The spec flags this as a risk for UAT; a `∙` bullet is the prepared replacement.
- **SUBJECTIVE** Status phrases are clamped at 30 cells and cut mid-token (`(root_caus…`); the dashboard Phase
  and Status columns still clip without an ellipsis (pre-existing ratatui table behavior).

### Pillar 5: Spacing (3/4)

Strengths: the `>= 100` tier is `Percentage(20)/(26)/Min(13)/Length(21)/(12)/(9)`, equals the spec table at
100/120/140/160/200 (asserted), keeps Status at 13 at 100 and 120 so the three existing ladder tests stay green,
and leaves the `80..99`, `60..79` and `<60` tiers byte-identical (a test compares renders with and without
digests). The summary row appears only from 100 columns and 12 rows and keeps 2 cells before `[i] inbox`. The
detail block's fixed widths (tag 7, checks 9, age 11, name floor 12, phase 5..18) match the grid table; the
height cap `max(4, 40%)`, the 12-row hide rule and the `usize` arithmetic (IN-05) are tested.

Findings:
- **BUG (WARNING)** At exactly 100 columns the detail command column can be starved to zero cells even though
  the contract says `W >= 100` shows every column. The grid reserves the phrase column (up to 30) and the 12-cell
  name floor first and gives the command whatever remains (`owner_block.rs:552-600`); with a long phrase on any
  row the remainder is 0 (seen in a pre-fix render of a project with workstream rows: no command at 100, a cut
  `/gsd-verif…` for another). The summary acknowledges the cut command but not the zero case. Fix: when
  `W >= 100`, reserve a minimum (about 12 cells, enough for `/gsd-…`) before sizing the phrase column, or
  shrink the phrase toward 12 first as it already does for the name.
- **SUBJECTIVE** Progress is 11 cells at 100 columns, so `10/10 phases` shows as `10/10 phase` and the
  Backlog value touches it (`10/10 phase 1`). The spec accepted this (same as the old 80 tier) and left it to
  UAT; it reads worse next to a new column.
- **SUBJECTIVE** The Phase column at 100 is 19 cells; long phase labels are clipped without an ellipsis.
  Accepted unresolved item in the spec's UI Considerations table.
- The block's name floor of 12 cells leaves most names clipped at 100 columns (`Cases, Sand…`) while the
  phrase column keeps up to 30 cells and the age 11; arguably the phrase should yield first. SUBJECTIVE.

### Pillar 6: Experience Design (3/4)

State coverage (verified in code and tests):
- Loading: blank `You` cell and `You: Loading...` until a digest arrives, ` · M loading` for a partial fleet;
  rows render first, the digest arrives by `Action::ProjectStateLoaded` or `OwnerDigestLoaded`.
- Empty: no digest, or nothing to show, takes zero rows (Phases tab byte-identical, tested); clean fleet reads
  `You: ·`; a missing or `.planning`-less project shows a blank cell, never `·` (WR-01 fix).
- Error: `▸ Todos ?`, unreadable owner files counted as work, reader panic isolated from the state refresh
  (WR-04), refresh-safe cursor clamp, trailing-edge reparse (WR-06), hourly age sweep (WR-07).
- Narrow widths: below 100 columns the dashboard is unchanged; the block drops command then age; below 12 tab
  rows or under a focused full-tab Waves pane it is hidden.
- Safety: no write, launch or spawn from the block; `e o v A y` are swallowed; every project-authored string goes
  through `shown()`; the escape guard covers both new surfaces. No destructive action exists in this phase.
- Focus model: the `↑` order (tab bar, block, phase list), `Esc` then `Esc`, `q` at every level, digits resetting
  focus, and a consumed key when the block has nothing to focus (WR-05) are implemented as specified and tested.

Findings:
- **BUG (WARNING)** The footer advertises `[↑]owner` whenever the block is drawn (`detail.rs:11575-11579` via
  `owner_block_rows > 0`), but the key only enters the block when it has a cursor stop
  (`enter_owner_block`, `detail.rs:2243-2248`). A block that shows only the non-selectable Todos fold (no
  rows, no Old debt, no Acknowledged) has no stop, so `↑` goes to the tab bar while the footer names the block.
  WR-05 fixed the key side of this mismatch; the unfocused footer was missed. Fix: base `owner_block_drawn` on
  "has at least one stop".
- **SUBJECTIVE** The block appears (and shifts the phase list down by up to 40 percent of the tab) when the
  digest arrives after first paint, and again when a watcher refresh changes the row count. The list keeps its
  selection, but the content moves under the user's eyes.
- **SUBJECTIVE** `[i] inbox` and the `i` key are a documented dead end until Phase 37: it sets one status
  message and nothing else.
- The block is keyboard-only: wheel inside it does nothing (IN-04 fix) and clicks do not focus it. The spec
  allowed deferring mouse support; mouse users cannot reach rows past the cap. SUBJECTIVE.
- `/y` with no match, or an all-filtered fleet, reads `You: ·` ("nothing waits on you"), which is accurate for
  the spec's `No owner work` rule but can read as a clean fleet when it is an empty view. SUBJECTIVE.

---

## Human Review Items

These are judgment calls the code cannot settle. The two checks the spec reserves for UAT (D-23) come first.

1. **Cell wording at 120 columns (UAT 1).** Are `decide N`, `test N`, `fix N ph`, `→ closes M`, `·` and
   `(+N old)` read naturally and quickly; can the projects with the most closeable testing be found in a
   glance; is the cell quiet?
2. **Alignment at 100 and 140 columns on the real terminal font (UAT 2).** Do Progress and Backlog stay aligned
   after `·` and `→` (ambiguous-width glyphs in a table cell); is the Phase column acceptable at exactly 100;
   does the summary fit and `[i] inbox` right-align? Also look at `10/10 phase` next to Backlog and a clipped
   Status that runs into the `You` text.
3. **Owner block at 100 columns.** Are the missing or cut command column, the 12-cell name truncation and the
   workstream rows (no phase number) tolerable until the BUG fixes above land?
4. **Fold marker for Todos.** Does `▸ Todos N` read as expandable; if so, switch that one marker to `∙`.
5. **Block separation and margin.** Is a block flush to column 0 and immediately followed by the phase-list
   header clear enough, or does it want a blank row or a rule?
6. **Green `closes phase` density** on a project with many closing TEST rows.
7. **Layout shift** when the block appears or changes height after a digest refresh.
8. **`[i] inbox` as a dead end** until the inbox screen exists, and the footer discovery of `i` below 100
   columns (help only).
9. **Status phrase truncation** (`open investigation (root_caus…`): acceptable, or show the status first?

---

## Recommendation Count

- Priority fixes (BUG, WARNING): 5 (header and hint collision, workstream phase number, FIX phrase duplicate,
  `[↑]owner` footer without a stop, command column starved at 100).
- Minor BUG: 3 (column grid sized from all rows, help rows clipped, fourth `You` ladder rung).
- SUBJECTIVE items: collected under "Human Review Items" (9 groups).

---

## Files Audited

- `src/ui/screens/owner.rs` (cell, summary, numbers)
- `src/ui/screens/owner_block.rs` (header ladder, rows, grid, folds, expansion, cursor, hint)
- `src/ui/screens/normal.rs` (tiers, `You` column, summary row, `i` key)
- `src/ui/screens/detail.rs` (block placement, focus, key handler, footers)
- `src/ui/screens/help.rs` (new rows and widths)
- `src/inbox.rs` (shared wording: `detail`, `closes_after_clauses`)
- Phase artifacts: `36-UI-SPEC.md`, `36-CONTEXT.md`, `36-0{1,2,3,4}-SUMMARY.md` and PLANs (skimmed),
  `36-REVIEW.md`, `36-REVIEW-FIX.md`, `36-REVIEW-DISPOSITION.md`
- Visual evidence: pre-fix ASCII renders (100 and 140 columns), read for evaluation only

Registry safety audit: not applicable (no shadcn, no third-party registries).

---

## Fixes applied

| Finding | Result |
|---------|--------|
| 1. Header clipped by the overflow hint | c230fc5d, fixed (header rung chosen beside the widest row-range hint) |
| 2. Workstream rows lose `ph N` | 0b6c7711, fixed (name shortened inside its prefix, phase number kept) |
| 3. FIX row repeats its check count | 95c7f11a, fixed (phrase shows the file status) |
| 4. Footer offers `[↑]owner` without a stop | 70e76515, fixed (footer keys on "block has a stop") |
| 5. Command column starved at 100 columns | 979874de, fixed (12-cell command floor reserved before the phrase is sized) |
| Minor: column grid sized from all rows | skipped: defensible design choice (stable columns while scrolling), changes agreed layout |
| Minor: help rows clipped | skipped: needs copy decisions on 3 help rows, not a small fix |
| Minor: fourth `You` ladder rung | skipped: recorded inferred decision, impact only at four-digit counts |
| SUBJECTIVE items | skipped: human review |
