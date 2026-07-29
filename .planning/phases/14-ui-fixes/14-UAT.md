---
status: testing
phase: 14-ui-fixes
source: [14-VERIFICATION.md]
started: 2026-07-29T00:00:00Z
updated: 2026-07-29T00:00:00Z
---

## Current Test

number: 1
name: D-R-P-E-V column alignment and badge glyph width, live terminal
expected: |
  `D` starts at the same column as sibling Status text (already independently
  re-confirmed at the byte level via TestBackend at widths 44-200), and the alias
  text starts at the same column regardless of which badge (⏸ / ⏳ / ▶ / none) is
  shown.
awaiting: user response

## Tests

### 1. D-R-P-E-V column alignment and badge glyph width, live terminal

test: Open the dashboard at 80 and 60 columns in a real terminal (not just the TestBackend check re-run during verification) and visually compare the Status column against `executing` / `v1.0 Complete` rows; also compare the alias-column start position across a paused row, an async-job row, and a session row.
expected: `D` starts at the same column as sibling Status text, and the alias text starts at the same column regardless of which badge (if any) is shown.
why_human: Actual glyph advance width for `⏸` / `⏳` / `▶` varies by terminal emulator and font beyond what `unicode-width` / ratatui's `Span::width()` reports (code review WR-01, confirmed unfixed by direct source read — `alias_badge` at normal.rs:61-77 is byte-identical to the initial-verification read). The Status-column clipping defect itself is now CLOSED and independently re-confirmed; this item is narrower and pre-existing, carried forward unchanged from the 2026-07-28 initial verification.
result: [pending]

### 2. `$EDITOR` suspend/resume round-trip and post-edit refresh

test: On the Docs (Browse) tab, press `e` on a markdown file, edit and save it in `$EDITOR`, then quit the editor and observe the TUI.
expected: The TUI resumes, and the viewer shows the newly-saved content.
why_human: No test in this repo drives a real terminal or the suspend/resume process boundary (code review WR-03, confirmed unfixed by direct source read — the resume path still does not invalidate `cache.browser_file_content`). Explicitly outside plan 14-04's scope fence. The current shipped behavior is expected to show *stale* pre-edit content; the human check should confirm this and file it as a follow-up.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
