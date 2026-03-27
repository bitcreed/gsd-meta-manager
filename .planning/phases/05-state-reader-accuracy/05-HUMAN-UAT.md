---
status: diagnosed
phase: 05-state-reader-accuracy
source: [05-VERIFICATION.md]
started: 2026-03-26T00:00:00Z
updated: 2026-03-26T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Pipeline color rendering on unfocused rows
expected: Dashboard shows compact D-R-P-E-V pipeline with green (complete), yellow (active), dark-gray (dormant) per stage
result: pass

### 2. Expanded text on selected row
expected: Selected/focused row shows expanded text like "Executing 2/3", "Planned (3 plans)" instead of compact pipeline
result: issue
reported: "Pipeline is always one step ahead: e.g. [D] [R] [P] E V shows Planned, [D] [R] [P] [E] V shows Executing (but it's done executing, it's validating). When R is orange it shows Discussed. User decision: drop expanded status text for now."
severity: minor

### 3. Disk status brackets in detail view
expected: Detail view phase list shows "[Executing 2/3]", "[Planned (3 plans)]" brackets after phase names
result: issue
reported: "Works but needs a header that explains the two numbers with and without []"
severity: cosmetic

### 4. Completed milestone dimmed display
expected: Projects with completed milestones show "v1.0 Complete" in dimmed gray text
result: pass

## Summary

total: 4
passed: 2
issues: 2
pending: 0
skipped: 0
blocked: 0

## Gaps

- truth: "Selected/focused dashboard row shows expanded text describing current phase status"
  status: failed
  reason: "User reported: expanded status label is one step ahead of actual stage. User decision: drop expanded status text entirely for now."
  severity: minor
  test: 2
  artifacts: [src/ui/screens/normal.rs]
  missing: []

- truth: "Detail view phase list has header explaining bracket notation for disk status"
  status: failed
  reason: "User reported: needs a header/legend explaining what the two numbers with and without [] mean"
  severity: cosmetic
  test: 3
  artifacts: [src/ui/screens/detail.rs]
  missing: []
