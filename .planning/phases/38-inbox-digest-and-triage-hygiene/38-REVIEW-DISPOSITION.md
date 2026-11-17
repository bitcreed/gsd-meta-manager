---
phase: 38
review: 38-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "A partly folded BLOCKED phase yields two rows with one GroupKey"
  - id: IN-01
    severity: info
    disposition: open
    title: "Owner block checks cell widens a row past the planned grid at 100 or more checks"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "README misdescribes the `looks resolved` label"
  - id: IN-03
    severity: info
    disposition: open
    title: "Help overlay and Stale line hardcode or mislabel what is configurable or not a phase"
  - id: IN-04
    severity: info
    disposition: skipped
    title: "Label heuristics have known false positives"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "Duplicated fixture-aging code in tests"
open: 2
total: 6
recorded: 2026-11-17T17:52:47.530Z
---

# Phase 38: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 38-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | fixed | 38-REVIEW-FIX.md |
| IN-03 | info | open | - |
| IN-04 | info | skipped | 38-REVIEW-FIX.md |
| IN-05 | info | fixed | 38-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
