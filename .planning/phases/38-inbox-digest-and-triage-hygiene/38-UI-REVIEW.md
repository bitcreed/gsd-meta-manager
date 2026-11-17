# Phase 38 — UI Review

**Audited:** 2026-11-17
**Baseline:** abstract 6-pillar standards (no 38-UI-SPEC). The inherited contract for the TUI inbox is `37-UI-SPEC.md` (approved 2026-11-14) with its applied 37-UI-REVIEW fixes; the phase decisions are `38-CONTEXT.md` D-01..D-14.
**Screenshots:** not captured. Ratatui TUI plus plain CLI text, no browser or dev server; the screenshot gitignore gate was not run because nothing was captured. The CLI was audited from the new goldens (`inbox-digest.txt`, `inbox-lane-test.txt`, `inbox-alias.txt`), measured at 80, 100 and 120 columns, and the legacy `--all` goldens (`inbox.txt`, `inbox-include-all.txt`, unchanged). The TUI was audited from the Owner block goldens (`owner_block_80x24.txt`, `owner_block_120x30*.txt`) and a code read of the phase-38 diff (`7be4a791..HEAD`) of `ui/screens/inbox.rs`, `owner_block.rs`, `owner_actions.rs`, `help.rs` and the TUI tests. Goldens carry no modifier data, so DIM was judged from code and the `labels_are_dim_and_never_move_a_row` test.
**Interaction captures:** off (`workflow.ui_interaction_capture` is false)
**Widths covered:** CLI 80/100/120 by measurement. TUI 120 and 140 from the new tests only; no phase-38 test renders the inbox at 80 or 100 columns.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Hint line, fold copy and refusals are precise and actionable; `Owner inbox: 1 projects` now heads every `inbox <alias>` render, and several digest lines read ambiguously (`merge test 3`, `no owner work` in the owner-work block, lane-less NEXT rows) |
| 2. Visuals | 3/4 | The digest has a clear top-down hierarchy (totals, projects, NEXT, folds, hint) and the merge reads as one entry; NEXT can show the same phase twice with only the command to tell Test from Fix |
| 3. Color | 3/4 | No new color. Labels and `N files` are DIM-only and text-carried; inside DIM rows (Acknowledged, BLOCKED) they cannot be told apart from the row |
| 4. Typography | 3/4 | Role set unchanged and disciplined; the label tally appears in three spellings across views, and TUI extras are cut mid-word at narrow phrase widths |
| 5. Spacing | 2/4 | D-10 digit alignment is exact, but the collapsed-line grid pads to the widest phrase: the 24-line digest needs 31 rows at 80 columns and 29 at 100, so the "one screen" goal fails on a standard terminal |
| 6. Experience Design | 3/4 | View routing, line budget, `--all` back-compat, merged-row refusals and the WR-01 fix are solid; the 25-line budget is already taller than an 80x24 terminal, and labels on phrased TUI rows disappear at 100 columns, untested |

**Overall: 17/24**

No pillar is a 1 and no defect blocks a task (every view renders, every action works or refuses with a next step), so there are no BLOCKERs. Everything below is a WARNING unless marked minor.

---

## Top Priority Fixes

1. **WARNING (highest): the digest is not one screen at 80 or 100 columns.** `collapsed_block` (`src/inbox.rs:1366`) pads the phrase column to its widest cell with no cap. In `inbox-digest.txt` one FIX phrase (`1 failed check, 0 gaps · 1 looks resolved`, 41 cells) pushes every NEXT line to about 119 cells and leaves a 46-cell gutter on rows that have no phrase. The project lines run to about 107 cells. Measured on the golden: 24 lines need **31 rows at 80 columns** (7 lines wrap), 29 at 100 and 26 at 120. The budget itself (`DIGEST_MAX_LINES = 25`) is already taller than an 80x24 terminal before any wrap, so the hint line is the first thing to scroll off. Fix: in `collapsed_block`, put the phrase last after the command (an unpadded trailing column), or cap it at about 24 cells with `…`; move age before the phrase. Cap the digest at 22 lines (24 rows minus a prompt). Add a test that asserts every digest and lane-view golden line is at most 100 cells, and ideally 80.
2. **WARNING: `Owner inbox: 1 projects` on every single-project render.** `totals_line` (`src/inbox.rs:1127`) writes `{} projects` without `plural`, so all three renders in `inbox-alias.txt` start with `1 projects`. `fold_lines` makes the same mistake (`in {old_projects} projects`, `in {todo_projects} projects`, `unreadable in {n} projects`; see the unit-test strings `Old debt: 2 checks in 1 projects`, `Triage: 1 pending todo in 1 projects`). The bug predates the phase, but Phase 38 made `inbox <alias>` a primary view, and there it now shows every time. The TUI fold line uses `plural` and says `1 project`, so the CLI and the TUI disagree. Fix: use `plural(n, "project", "projects")` in both functions. That changes the output of `--all` for one-project runs, so treat it as a deliberate exception to the byte-for-byte rule. The fixture goldens (6 and 10 projects) do not move. Update the asserts at `tests/inbox_cli.rs:592, 656, 762, 1475` and `src/inbox.rs:1810-1832, 1931`.
3. **WARNING: TUI labels and `N files` vanish at 100 columns on rows that have a phrase.** `row_line` (`src/ui/screens/inbox.rs:757-774`) draws the extras inside the phrase column after the phrase and a 2-cell gap, cut to whatever room is left. `grid_for_inbox` shrinks the phrase column first, down to 12, and on any real fleet it does reach 12 at 100 columns: the natural row is about 133 cells. A FIX row (`1 failed check, 1 gap` is 21 cells) then has no room for its extras, and `1 looks resolved` is dropped without a trace. A `+ fix 1` row (7 cells plus the 2-cell gap) keeps 3 cells of `4 decision?`, a cut label that says nothing. Merged rows in "closes a phase" survive only because that section has no phrase. D-11's "the CLI and the TUI share the wording" therefore holds only at about 120 columns and up. Fix: reserve the widest extras width (at least 7 cells, enough for `2 files`) before the phrase narrows, or move the extras into the name column's spare cells. Add 80x24 and 100x30 buffer tests with a merged row, a labeled FIX row and an open Stale fold. This repeats the test gap 37-UI-REVIEW fix 8 named: every new phase-38 TUI test renders at 120 or 140.

Further warnings:

4. **NEXT rows hide their lane.** `labels ph 02 Sync 1 check` appears twice in the NEXT block of `inbox-digest.txt`, once as Test and once as Fix. Only the trailing command (`/gsd-verify-work` against `/gsd-plan-phase --gaps`) and the FIX phrase tell them apart, and at 80 columns the command has wrapped to the next row. Fix: add a short lane cell (`test`, `decide`, `fix`) after the alias in the NEXT rows only.
5. **Project lines read ambiguously.** `  merge test 3` (`project_line`, no gate clause) reads as a project named "merge test". `  old closes after: no owner work (ph 24)` sits in a block that lists projects *with* owner work. `elsewhere:` appears only when a gate clause comes before it. Fix: always write `elsewhere: test 3`, or `outside the milestone gate: test 3`, and give the alias a padded column as NEXT has. For the `no owner work` clause, say what is left: `closes after: ph 24 (no owner work)`.
6. **The narrow fold rung mixes units without saying so.** At rung 2 (`inbox.rs:1386-1414`) the line reads `▸ Old debt (6)   ▸ Stale (14)   ▸ Acknowledged (8)   ▸ Triage 11`, which is checks, then phases, then files, then todos. Stale is the only fold that counts entries, not items, and it sits next to Old debt, which counts checks. Fix: make `Stale (N)` count `t.stale.items`, as Old debt does, or label the units.
7. **The help row `decision?` packs two labels into one row.** It reads `Label only: the text reads like a ruling; looks resolved: it says fixed; a label never moves or hides a row`, so the second label has no key cell and scanning for `looks resolved` finds nothing. Fix: give each label its own row (`decision?` and `looks resolved`) and say "label only, never moves or hides a row" once.
8. **Advice on a merged row leads nowhere.** `o` on a merged row says `2 files behind this row: expand it with Enter, or press g` (`owner_actions.rs:210`), but expansion lines are not cursor stops, so expanding never lets `o` open a file. Only `g` works. `A` sends the owner to the Owner block, where the same phase is two separate rows (D-06 keeps that block frozen), so the merged entry the owner just saw is gone. Fix: say `press g to pick a file in the Owner block` in both refusals, or make `o` open the first file and name it in the status line.

Minor: the alias view keeps raw Markdown from CONTEXT questions (`- [ ] - **Q-01 [OPEN]** Should…`), and FIX phrases keep a zero part (`1 failed check, 0 gaps`). Both strings predate the phase and render as they did before, but the new fixtures put them in a golden. The TUI expansion names files by their relative path (`file phases/43/43-UAT.md`) while the CLI uses the basename (`file 43-UAT.md`). Rows of an open Stale fold carry no sub-header, so their lane (Test or Fix) is lost once they leave their section.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

Strengths:
- The digest hint line `inbox <alias> · inbox --lane test|decide|fix|blocked · inbox --all` names every way out of the digest. `+N more projects (inbox --all)` and `Unreadable: N projects (a, b, c, …) · inbox --all says why` announce whatever is left out. Nothing is hidden silently.
- `Stale: 33 checks in 14 phases older than 90 d` (`stale_fold_text`, `inbox.rs:1139`) sits next to the existing folds in the same `Label: N unit in P container` form. After IN-03 it switches to `items` / `entries` once a debug session, handoff or checkpoint is folded.
- `STALE (> 90 d)` in the alias view uses the configured `fold_days`, not a literal. The help row names `preferences.owner.fold_days (default 90)`, which fixes the hardcoded 90 that 38-REVIEW IN-03 found.
- Refusals give a next step: `Nothing to copy: open Stale and copy one phase`, `2 files behind this row: press g and acknowledge one file in the Owner block` (but see fix 8).
- The vocabulary matches 36 D-17: `ph N`, `N checks`, `→ closes phase`, `stale`, `N files`. US spelling. No singular "they".

Defects:
- `1 projects` in `totals_line` and `fold_lines` (fix 2). Tests pin the wrong form in four places.
- Lane-less NEXT rows (fix 4), ambiguous project lines (fix 5), the merged help row (fix 7) and the merged-row advice (fix 8).
- `4 decision?` uses a count with a label that is not a noun, so it reads as "4 decision". It is consistent with D-11's `2 decision?`, which is locked, so this is minor. `4 decision? · 1 looks resolved` works as a tally.

### Pillar 2: Visuals (3/4)

- Hierarchy: the digest reads top-down in the order D-03 asks for: totals line, project block, a blank line, `NEXT`, folds, the hint. The `NEXT` heading is all caps like the `TEST — …`, `DECIDE`, `FIX — code needed` and `STALE (> 90 d)` headings, so headings look the same in every view.
- The merge works visually. `inbox-lane-test.txt` shows `merge ph market/43 Trader mode 3 checks → closes phase · 2 files`. The alias view puts a `file 43-UAT.md` / `file 43-VERIFICATION.md` subheading over each file's items, and the 51-step scripted UAT is one line (`51 checks`) in the lane view.
- The checks column reads as one column: `51 checks`, ` 1 check `, ` 7 checks` share a digit edge in the lane golden, and the Owner block goldens moved by exactly one cell (`quota-clip  1 check `).
- Defects: duplicate-looking NEXT rows (fix 4). Wrapped NEXT lines at 80 and 100 columns break the column grid that makes NEXT scannable (fix 1). The project block is ragged-left (`six closes after:`, `merge test 3`, `old-ledger closes after:`), while NEXT directly below it is a padded grid, so the two blocks look as if they come from different tools.
- In the TUI, a Stale fold row with its `▸`/`▾` marker matches Old debt. Open Stale rows have no sub-header (minor, above).

### Pillar 3: Color (3/4)

- No new color anywhere. The CLI views are plain text. The TUI adds only `Modifier::DIM` on extras (`plain.add_modifier(Modifier::DIM)`, `inbox.rs:771`) and on the `(label)` suffix of expanded item lines. The 37 quiet-color contract (Cyan only on the cursor row, Yellow only on `stale`, Magenta only on the `ws` prefix) stands, and `NO_COLOR` is not affected by this phase. The CLI emits no SGR codes, and the TUI color set is unchanged from 37, which already did not consult `NO_COLOR`; only `roadmap_view.rs` does.
- Meaning is carried by text (`decision?`, `looks resolved`, `2 files`, `Stale`). No meaning rides on color alone.
- Deductions: on rows that are already DIM (Acknowledged rows, BLOCKED rows) the DIM extras do not stand out from the row at all. On the cursor row, `cursor_style()` removes DIM, so the extras take full Cyan+BOLD weight and become as loud as the name. Neither is wrong, but in the one place a label matters most (a label on a row the owner is about to act on) the emphasis is inverted. Needs human review in the owner's theme: whether DIM `decision?` is legible at all on a low-contrast palette (the same open question as 37 for DIM rows).

### Pillar 4: Typography (3/4)

- The roles are unchanged from 37: BOLD headings and keys, DIM muted fragments, glyphs from the registry (`▸ ▾ · … →`). No ITALIC, UNDERLINED or new glyph. `→` (U+2192) in the CLI `→ closes phase` is in the TYPOGRAPHY set.
- The label tally comes in three forms: collapsed lines `7 checks  4 decision?` (own column, no separator), the alias view entry line `7 checks · 4 decision?` (` · ` separator), and item lines `…  (decision?)` (parenthesized). The CLI and the TUI share the words, as D-11 requires, but not the punctuation. Minor. Pick the ` · ` form for both group-line variants.
- Truncation: the CLI caps the name at 32 cells with `…` and never cuts the phrase, which causes fix 1. The TUI cuts extras mid-label with `fit_cells` (a 3-cell remnant of `4 decision?` after a `+ fix 1` phrase at a 12-cell column, fix 3); a label cut that way says nothing. Drop the whole extra rather than cut a label.

### Pillar 5: Spacing (2/4)

What works:
- D-10 is exact. `checks_cell(n, digits)` right-aligns the number in `count_digits(max)` cells (at least 2) and pads the unit to 6, so `1 check ` and `2 checks` share a digit edge. The TUI's `Grid::checks_w = CHECKS_W.max(digits + CHECKS_UNIT_W)` widens for three digits, and IN-01 extended the same planned width to the Owner block (`a_three_digit_checks_cell_widens_the_planned_grid`). The Owner block goldens moved by one cell, as D-10 allowed.
- `collapsed_block` drops a column no row fills (no empty phrase column in the lane golden's "closes a phase" section), and trailing blanks are trimmed.

Defects:
- **The phrase column has no cap** (fix 1). In `inbox-digest.txt` the widest phrase (41 cells) sets the column for all five NEXT rows. Rows 2 and 4 carry a 46-cell run of blanks between `1 check` and `4 d`. Line widths: NEXT about 119 cells, project lines up to about 107. Rows needed: 31 at 80 columns, 29 at 100, 26 at 120, for a 24-line text whose whole purpose is to fit one screen.
- **The lane view sizes columns per section.** `push_rows` calls `collapsed_block` once per section, so `inbox-lane-test.txt` has the alias column 5 cells wide under `TEST — closes a phase` and 10 under `TEST — phase stays open`, and the phase, name and checks columns shift between sections. Fix: compute the widths once over every printed row of the view, then render each section with them (the TUI's `grid_for_inbox` already works this way).
- **The TUI phrase column hosts the extras and is the first column to shrink** (fix 3). The extras inherit the column's 12-cell floor, which was sized for phrases alone.
- The project block has no columns (Pillar 2).

### Pillar 6: Experience Design (3/4)

Strengths:
- View routing follows D-02 exactly: `--json` first, `--all` keeps the legacy text, a single project prints its full checklist, `--lane` gives the lane view, and anything else gives the digest. A bare `--include` implies `--all`, so existing scripts keep working, and the `--all` goldens are byte-identical.
- Nothing is lost by the fold. The Stale fold and the `STALE (> N d)` section keep folded work reachable, and the totals, `closes_after` and header numbers keep counting it. A test pins the TUI header as identical with `fold_days` at 90 and at 3650.
- The merged-row actions are safe: `A` refuses with a reason (file-level acknowledge cannot target two files), `v` runs the shared command, `y` copies `entry_checklist` (the CLI's own text, 37 D-09), and `y` on Stale refuses with a next step. Selection survives a merge appearing under the cursor (`a_merge_appearing_keeps_the_cursor_on_its_row`).
- WR-01 is fixed: a partly folded BLOCKED phase is one row with a unique key (`a_partly_folded_blocked_phase_is_one_row`). Folding is placed by the newest sibling. That rule is recorded in 38-REVIEW-FIX as needing human confirmation.
- Labels never move, rank or count a row (`labels_are_dim_and_never_move_a_row`, plus the CLI label-independence test).

Weaknesses:
- The line budget is above the screen it targets (fix 1). D-14's evidence is 25 lines on the real fleet, which overflows an 80x24 terminal even before any line wraps.
- Labels and file counts depend on the width (fix 3). No test covers 80 or 100 columns.
- The merged-row advice and the move from a merged row in the inbox to two rows in the Owner block (fix 8).
- Open Stale rows lose their lane context.
- Needs human review (D-14 UAT): whether `inbox` on the real fleet reads as a useful one-screen answer. The audit predicts no at 80 or 100 columns because of fix 1, and probably yes at 120 or more.

---

## needs_human_review (not scored as defects)

1. The D-14 readability check on the real fleet, at the owner's usual terminal width.
2. Legibility of DIM `decision?` / `looks resolved` / `N files` in the owner's theme, including inside DIM rows and on the cursor row.
3. Whether the newest sibling should decide where a partly folded BLOCKED row goes (38-REVIEW-FIX WR-01 note).
4. Whether the label false positives (IN-04, skipped: `Tap Accept`, `Review screen renders`) make `N decision?` noise on real UATs. D-14 evidence: 28 `decision?` labels across the fleet, against 1 `looks resolved`.

## Registry Safety

Not applicable: ratatui widgets only, no `components.json`, no third-party registries.

## Files Audited

- `.planning/phases/38-inbox-digest-and-triage-hygiene/38-CONTEXT.md`, `38-01..04-SUMMARY.md`, `38-01..04-PLAN.md` (intent), `38-REVIEW.md`, `38-REVIEW-FIX.md`
- `.planning/phases/37-inbox-screen-and-actions/37-UI-SPEC.md`, `37-UI-REVIEW.md` (baseline and format)
- `src/inbox.rs` (`render_project`, `render_lanes`, `totals_line`, `stale_fold_text`, `fold_lines`, `checks_cell`, `collapsed_of`, `collapsed_block`, `entry_line`, `entry_checklist`, `project_line`, `render_digest`, `detail`)
- `src/state_reader/owner_items/labels.rs` (`LabelCounts::text`), `triage.rs` (`next_actions`)
- `src/ui/screens/inbox.rs` (`Grid`, `grid_for_inbox`, `phrase_cell_w`, `row_line`, `fold_line_at`, merged-row refusals, phase-38 tests)
- `src/ui/screens/owner_block.rs` (`cursor_style`, extras, `checks_w`)
- `src/ui/screens/owner_actions.rs` (`copy_entry_text`, `fold_copy`, merged `o` refusal)
- `src/ui/screens/help.rs` (new `Stale` and `decision?` rows, `wrapped_row`, `wrap_owner_sections`)
- Goldens: `tests/fixtures/owner_golden/inbox-digest.txt`, `inbox-lane-test.txt`, `inbox-alias.txt`, `inbox.txt`, `owner_block_80x24.txt`, `owner_block_120x30.txt`
- Tests: `tests/inbox_cli.rs` (plural asserts at 592, 656, 762, 1475)

---

## Fixes applied

CLI side, 2026-11-17 (src/inbox.rs and tests; src/ui/ untouched apart from a formatting-only commit).

| Finding | Status | Commit |
|---------|--------|--------|
| 1. Digest not one screen | Fixed, partly (see the note below) | `ed77b450` |
| 2. `1 projects` | Fixed. `fix 1 phases` is fixed in the same commit. The `--all` goldens `inbox.txt` and `inbox-include-all.txt` are byte-identical | `4eb2e60a` |
| 3. TUI extras at 100 columns | Deferred to the TUI pass (src/ui) | n/a |
| 4. NEXT rows hide their lane | Fixed: a `test`/`decide`/`fix` cell follows the alias | `9489af74` |
| 5. Project-line wording | Deferred, not trivial: `no owner work (ph N)` comes from the shared `closes_after_clauses` | n/a |
| 6. Narrow fold rung units | Deferred to the TUI pass. The rung lives in `src/ui/screens/inbox.rs:1404`: `Stale ({})` should count `t.stale.items` | n/a |
| 7, 8 | TUI, out of scope for this pass | n/a |
| Pillar 5: lane-view widths per section | Fixed: one `CollapsedGrid` covers every section | `878c633b` |
| (gate) `cargo fmt --check` failed on HEAD | Formatting only | `f84a8a3c` |

Note on fix 1:
- Collapsed lines now print the phrase last, after the command, with no padding. Phrases are cut on whole ` · ` parts with `…`.
- The NEXT name column is capped at 20 cells.
- Project lines fit 80 cells. When a line is too wide, its phase lists step down from 4 per lane to 2, then to counts only.
- `DIGEST_MAX_LINES` went from 25 to 22.
- A new test bounds the width: project, fold and hint lines must be at most 80 cells, and NEXT and lane-view rows at most 100.
- 80 cells cannot fit a NEXT row: the fixture's workstream row takes 94 cells before its phrase (alias, lane, phase, name, checks, age and the whole command).
- In the golden digest that row's phrase shrinks to `…`.
- The golden digest is 22 lines and 100 cells at its widest. It fits a 100x24 terminal. At 80 columns, 5 NEXT rows wrap, so it needs 27 rows.
- To fit 80 columns, NEXT would have to drop a D-03 cell (age or name) or move the command to a second line. That needs a decision from the owner.
