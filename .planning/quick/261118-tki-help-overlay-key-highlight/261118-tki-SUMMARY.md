---
phase: quick-261118-tki
plan: 01
subsystem: ui/help-overlay
tags: [help, tui, key-find, highlight]
status: complete
key-files:
  modified:
    - src/ui/screens/help.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions: [D-01..D-08 (inferred, orchestrator), I-tki-1..11 (inferred, planner)]
metrics:
  tasks: 3
  commits: 4
  commits_note: "`git rev-list --count 3266fb5d..HEAD` is 8: the 4 task commits below plus 4 commits of quick 261118-tm6 that were merged into the branch while this task ran (merge 36b448d5)."
plan_head_before: 3266fb5db6ee3047d440bf7bf66b95a63b60a830
plan_head_after: c102198c5c3ecdf170d1f580c998721103e679e6
actuals:
  tokens: 12000   # chars/4 over the 4 task commits' diff (about 48 kB)
  tasks: 3
  commits: 4
---

# Phase quick-261118-tki Plan 01: Help overlay key highlight Summary

Pressing a key in the help overlay now marks every row whose key column lists it (ASCII markers plus REVERSED/BOLD), counts the matches in the popup title, cycles on repeat, and scrolls the current match into view through `clamp_scroll`.

## Behavior before vs after

| Input in the help overlay | Before | After |
|---------------------------|--------|-------|
| `?`, Esc | close | close (also while a highlight is active) |
| Down, PgDn, Up, PgUp | scroll | scroll (UIFIX-04 ordering unchanged) |
| `j`, `k` | scroll | highlight rows, like any other key |
| any other listed key (`s`, `x`, `G`, Enter, Space, Left, Tab, `3`, ...) | consumed, no effect | marks every row listing it: current row `> ` + REVERSED\|BOLD, others `* ` + BOLD; title ` Help  x: 2/4 `; same key again moves to the next match and wraps |
| unlisted key (`n`, `9`, F5, Shift+Tab, Home, non-ASCII) | consumed | title ` Help  no help entry for <key> `, nothing marked; name is ASCII only (`U+XXXX` for other chars) |
| closing hint | one DIM line naming `j/k, PgUp/PgDn` | two DIM lines: scroll keys (Up/Down, PgUp/PgDn) and "Any other key: highlight its rows (again: next match)" |

Matching reads key-column tokens only (never descriptions) and is case-sensitive. Pinned match counts (flag on): `s` 2, `x` 4 (flag off 2), `g` 5, `j` 7, `Left` 5, `Space` 3, `Tab` 1, `3` 2, `8` 1, `m` 1, `M` 1, `c` 2, `D` 1, Ctrl+c 1; `9`, `n`, `N`, F5, Shift+Tab, U+00E9, U+001B give no match.

Keys that can never highlight: `?` and Esc (close the overlay), Ctrl+C (taken by `app.rs` before any screen), Up, Down, PgUp, PgDn (scroll). Rows such as `Up` or `PgUp / PgDn` are found through their other tokens or not at all.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | adb58bdb | feat(quick-261118-tki): highlight the help rows that list a pressed key |
| 2 | 37e08919 | feat(quick-261118-tki): find every listed help key, cycle matches and scroll to them |
| 3a | 1a1620b1 | feat(quick-261118-tki): j/k find rows in the help overlay instead of scrolling |
| 3b | c102198c | docs(quick-261118-tki): document finding a key in the help overlay |

## Decisions (all inferred, none confirmed by the owner)

Orchestrator-inferred:
- D-01 [inferred]: key-column tokens decide a match; no description matching; case-sensitive.
- D-02 [inferred]: the tokenizer covers every spelling in use; a coverage test fails on any key row that neither tokenizes nor is exempt.
- D-03 [inferred]: scroll to the match, cycle with wrap, show `x: i/n`, and `no help entry for <key>` otherwise.
- D-04 [inferred]: Esc and `?` close; arrows and PgUp/PgDn scroll; `j`/`k` highlight. The `page_up_at_the_top_stays_at_the_top` test now presses Up.
- D-05 [inferred]: highlight never relies on color; ASCII markers plus style, glyph scan stays green.
- D-06 [inferred]: all scroll arithmetic goes through `clamp_scroll` (`reveal_offset`); no new max-scroll subtraction (grep gates at 0).
- D-07 [inferred]: whole-row tests built through `row()`; verification through `rtk bypass`.
- D-08 [inferred]: README `?` row and GETTING-STARTED `?` bullet updated.

Planner-inferred (I-tki-N):
- I-tki-1 [inferred]: only Up/Down/PgUp/PgDn scroll; Left/Right highlight; Home/End give the no-match notice.
- I-tki-2 [inferred]: `NON_KEY_SECTIONS` (6 headings) never match; `NON_KEY_LABELS` (Click, Double-click, Wheel, Shift+drag, Stale, Labels) exempt mouse gestures and inbox pseudo-keys; any other heading is a key section by default.
- I-tki-3 [inferred]: a fresh key starts at the first match at or below the top row, wrapping to the first in the body; counter is the 1-based body position.
- I-tki-4 [inferred]: markers replace the two-space indent (`> ` current, `* ` others); continuation rows of a matched entry get the same marker and style.
- I-tki-5 [inferred]: the hint lives in the block title, so it takes no content row.
- I-tki-6 [inferred]: Shift ignored except that an ASCII lowercase char with SHIFT is uppercased; Ctrl is part of the key (Ctrl+C row is unreachable, documented); Alt ignored.
- I-tki-7 [inferred]: titles show fixed names and ASCII-graphic chars only, other chars as `U+XXXX`, unnamed codes as `that key`; modifier-only and Null events change nothing.
- I-tki-8 [inferred]: Esc closes even with a highlight active.
- I-tki-9 [inferred]: the closing hint is two DIM lines, each at most 62 cells.
- I-tki-10 [inferred]: a match already visible does not move the view; otherwise `clamp_scroll` puts it on the top row (or at the maximum near the end), and the last visible row counts as hidden while the more-indicator is drawn.
- I-tki-11 [inferred]: `Shift+Tab` (BackTab) is in the grammar although no row uses it.

## Deviations from Plan

None - plan executed as written. Implementation notes: `HelpEntry` does not store the label (the coverage test re-reads it from the line), and its `exempt` flag carries `#[cfg_attr(not(test), allow(dead_code))]` because only the coverage test reads it. The tracer's test file additions were folded into the same `tests` module; the no-match "nothing is marked" assertion checks the popup's border-adjacent marker cells.

## Verification (raw)

- `rtk bypass cargo test --locked --no-fail-fast`: summed across 69 test binaries, **4591 passed, 0 failed, 19 ignored** (lib: 3724 passed, 3 ignored). Help module: 52 tests (35 at plan time).
- `cargo clippy --locked --all-targets -- -D warnings`: clean; also clean for `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- `cargo fmt --check`: clean.
- Plan grep gates: Task 1, 2 and 3 gates all pass (`total_lines.saturating_sub` and `total_lines - ` count 0 in help.rs).

## Known Stubs

None.

## Threat Flags

None. T-tki-01 (key name reaching the title) is mitigated by `key_label` and unit-tested with `\u{1b}` and `\u{e9}`.

## Self-Check: PASSED

- src/ui/screens/help.rs, README.md and docs/GETTING-STARTED.md are modified in commits adb58bdb, 37e08919, 1a1620b1 and c102198c, all present in `git log`.
