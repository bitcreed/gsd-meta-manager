---
status: testing
phase: 38-inbox-digest-and-triage-hygiene
source: [38-VERIFICATION.md, 38-UI-REVIEW.md]
started: 2026-11-17T13:10:00Z
updated: 2026-11-17T13:10:00Z
---

## Current Test

number: 1
name: Digest and TUI inbox look on the real fleet
expected: |
  Run plain `gsd-meta-manager inbox`, then open the TUI inbox (`i`) at 120 and 100 columns in your real terminal and theme. The digest fits one screen and its NEXT list feels like the right five things to do. In the TUI, same-phase UAT+VERIFICATION pairs are one row, the Stale fold sits after Old debt, the checks column aligns its digits, and the DIM `decision?` / `looks resolved` labels are legible against your theme.
awaiting: user response

## Tests

### 1. Digest and TUI inbox look on the real fleet
expected: Run plain `gsd-meta-manager inbox`, then open the TUI inbox (`i`) at 120 and 100 columns in your real terminal and theme. The digest fits one screen and its NEXT list feels like the right five things to do. In the TUI, same-phase UAT+VERIFICATION pairs are one row, the Stale fold sits after Old debt, the checks column aligns its digits, and the DIM `decision?` / `looks resolved` labels are legible against your theme.
result: [pending]

### 2. Label accuracy on real data
expected: Scan `inbox --all` or `--json` on the real fleet for `decision?` and `looks resolved` labels. Labels are useful enough as hints. IN-04 (false positives for accept, review, resolved) was skipped on purpose as label-only per D-11; confirm that noise level is acceptable.
result: [pending]

### 3. Width target (80 versus 100 columns)
expected: The digest is 22 lines and its widest NEXT row is 100 cells, so it fits 100x24; at 80 columns the five NEXT rows wrap (27 rows). In the TUI inbox at 80 columns a FIX row's `decision?` / `looks resolved` labels drop whole, leaving ` · …`. Decide whether 100 columns is the target. If 80 is required, NEXT must drop a D-03 cell (age or name) or put the command on a second line, and the TUI FIX row needs another rung.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
