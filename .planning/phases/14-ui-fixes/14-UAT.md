---
status: complete
phase: 14-ui-fixes
source: [14-VERIFICATION.md]
started: 2026-07-29T00:00:00Z
updated: 2026-10-29T00:58:27.335Z
---

## Current Test

[testing complete]

## Tests

### 1. D-R-P-E-V column alignment and badge glyph width, live terminal

test: Open the dashboard at 80 and 60 columns in a real terminal (not just the TestBackend check re-run during verification) and visually compare the Status column against `executing` / `v1.0 Complete` rows; also compare the alias-column start position across a paused row, an async-job row, and a session row.
expected: `D` starts at the same column as sibling Status text, and the alias text starts at the same column regardless of which badge (if any) is shown.
why_human: Actual glyph advance width for `⏸` / `⏳` / `▶` varies by terminal emulator and font beyond what `unicode-width` / ratatui's `Span::width()` reports (code review WR-01, confirmed unfixed by direct source read — `alias_badge` at normal.rs:61-77 is byte-identical to the initial-verification read). The Status-column clipping defect itself is now CLOSED and independently re-confirmed; this item is narrower and pre-existing, carried forward unchanged from the 2026-07-28 initial verification.
result: pass
source: agent-evidence [inferred — cleanup 2026-10-28]
evidence: |
  Settled by agent evidence at c7a9ed9 (human unavailable) [inferred — cleanup 2026-10-28].
  - Root cause (glyph advance width beyond unicode-width) closed by quick task 261021-1id
    (8421547..07ae883): every status/badge glyph now comes from src/ui/glyphs.rs, whose rule
    rejects East-Asian-Ambiguous/Wide/emoji glyphs. `rtk proxy cargo test --lib
    every_registry_glyph_is_width_safe` -> 1 passed. The badges named here (⏸ / ⏳ / ▶) were
    replaced by registry glyphs (e.g. ▶ U+25B6 -> ► U+25BA, EAW Neutral).
  - TestBackend: a_badged_row_and_an_unbadged_row_keep_every_later_column_aligned and
    test_status_column_aligns_with_plain_status_text -> 2 passed.
  - Live render: built binary driven in tmux (scratch --config, 2 registered + 5 auto-registered
    projects) at 80 and 60 columns; pane captured and measured per row. Phase cell starts at
    col 22/20 and Status cell at col 41/37 on all 8 rows at 80/60, across rows with the ►
    session badge, no badge, and workstream sub-rows; `D  R  P  E  V` cells start in the same
    column as sibling plain Status text ("2 agents", "1run · ◒8", "◒30 unmerged").
  - Interpretation [inferred — cleanup 2026-10-28]: per 14-UI-SPEC Badge Priority rank 4, an
    unbadged alias renders flush (badged alias text is +2 cells), so "same column regardless of
    badge" is read as: same column across badge kinds, and every later column identical with
    or without a badge. Paused / external-job badges not reproducible live; covered by the
    registry width-safety test since all badges share the one-cell registry rule.

### 2. `$EDITOR` suspend/resume round-trip and post-edit refresh

test: On the Docs (Browse) tab, press `e` on a markdown file, edit and save it in `$EDITOR`, then quit the editor and observe the TUI.
expected: The TUI resumes, and the viewer shows the newly-saved content.
why_human: No test in this repo drives a real terminal or the suspend/resume process boundary (code review WR-03, confirmed unfixed by direct source read — the resume path still does not invalidate `cache.browser_file_content`). Explicitly outside plan 14-04's scope fence. The current shipped behavior is expected to show *stale* pre-edit content; the human check should confirm this and file it as a follow-up.
result: pass
source: agent-evidence [inferred — cleanup 2026-10-28]
evidence: |
  Settled by agent evidence at c7a9ed9 (human unavailable) [inferred — cleanup 2026-10-28].
  The why_human note's expected stale-content defect (WR-03) was fixed in e90be67
  "fix(browser): reload viewed file cache after $EDITOR exits".
  - `rtk proxy cargo test --lib editor_exit_reloads_browser_file_cache` -> 1 passed.
  - Live: built binary in tmux (100x30) with EDITOR/VISUAL = a script that appends
    "APPENDED-BY-SCRIPTED-EDITOR <ts>" to its argument. Docs tab -> .planning/NOTES.md showed
    lines 1-3 ("Notes", "", "original line"); pressed `e`; editor ran with the file path, TUI
    resumed with status "Editor closed: NOTES.md" and the viewer now shows line 4
    "APPENDED-BY-SCRIPTED-EDITOR 1793235479", matching the file on disk. No restart, no
    manual refresh.

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none]
