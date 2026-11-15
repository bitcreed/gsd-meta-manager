---
status: testing
phase: 36-dashboard-and-detail
source: [36-VERIFICATION.md]
started: 2026-11-15T03:00:00Z
updated: 2026-11-15T03:00:00Z
---

## Current Test

number: 1
name: Read the `You` cell wording on a real fleet at 100, 120 and 140 columns
expected: |
  `decide N`, `test N → closes M`, `fix N ph`, dim `·` and `(+N old)` are legible and tell the owner
  which projects to test first; the Progress and Backlog columns stay aligned on a terminal that
  draws U+00B7 / U+2192 double-wide (OWNER_SEP is the one-line swap). Real-fleet ASCII renders of
  the dashboard at 100/140 columns and of the detail Owner block were produced for review outside
  the repo (phase36-screens.txt in the orchestrator scratchpad); confirm on a live terminal too.
awaiting: user response

## Tests

### 1. Read the `You` cell wording on a real fleet at 100, 120 and 140 columns
expected: `decide N`, `test N → closes M`, `fix N ph`, dim `·` and `(+N old)` are legible and tell the owner which projects to test first; Progress and Backlog columns stay aligned when U+00B7 / U+2192 render double-wide (OWNER_SEP is the one-line swap)
result: [pending]

### 2. Walk the Owner block on a project's detail Phases tab
expected: Press Up on the first phase, walk the Owner block, Enter on a TEST group, open the Old debt fold, Esc back. The header reads `Owner ── alias · milestone closes after: …`; rows, expansion, folds and footers read well, glyphs align, and nothing is launched or written
result: [pending]

### 3. Decide whether the `You:` summary totals should count lanes the two-lane cap hides
expected: OWN-17 says totals equal the sum of the visible cells. The implementation (CONTEXT D-11) sums the per-project numbers over the visible rows, including a lane a `decide + test` cell hides, so `You:` can show `fix 11` while the visible cell text sums to fewer. Accept D-11, or require the totals to sum the displayed lane text only
result: [pending]

### 4. Accept or reject the recorded deviations
expected: Each is acceptable for v2.5.0 — WR-07 (projects that show ages are re-read once per UTC hour, relaxing D-03 "no timer-driven recompute"); WR-01 (a project whose folder or `.planning/` is gone shows a blank `You` cell and is left out of the K-of-N count); D-09 divergence (unreadable owner files count as fix, not test); D-16 (no colon in the Owner block header)
result: [pending]

### 5. Review the remaining UI-audit polish items
expected: The subjective "Human Review Items" and the three skipped minor items in 36-UI-REVIEW.md (column grid sized from all rows, help rows clipped on the right, fourth `You` ladder rung at four-digit counts) are acceptable or get follow-up todos
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
