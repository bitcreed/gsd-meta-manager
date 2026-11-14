---
phase: 35
review: 35-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`is_answered` hides open questions that merely contain or start with RESOLVED, ANSWERED or CLOSED"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "An unreadable CONTEXT file is listed but counted nowhere (summary `decide`, `closes_after.decide`)"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The `closes after:` line is silently truncated by the 200-character cap, dropping the lane parts that matter most"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "Stale and misleading doc comment on `OPEN_CACHE`"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "Generality without instantiation, `OpenKind` as a cache key discriminator"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "Docs drift for item ids, `acknowledged_files`, and the Old debt fold"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "`hidden.todos_unreadable` is JSON-only, so the plain checklist still shows no sign of an unreadable todo folder"
  - id: IN-05
    severity: info
    disposition: skipped
    title: "A phase with owner work but no plan is reported as `not built` and contributes nothing to the gate"
open: 0
total: 8
recorded: 2026-11-14T22:52:50.453Z
---

# Phase 35: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 35-REVIEW-FIX.md, b0b8fa3c |
| WR-02 | warning | fixed | 35-REVIEW-FIX.md, 9bcebd09 |
| WR-03 | warning | fixed | 35-REVIEW-FIX.md, 6ad3aa23 |
| IN-01 | info | fixed | 35-REVIEW-FIX.md, 1b3e19b3 |
| IN-02 | info | fixed | 35-REVIEW-FIX.md, 742373f4 |
| IN-03 | info | fixed | 35-REVIEW-FIX.md, cf81b028 |
| IN-04 | info | fixed | 35-REVIEW-FIX.md, 8215266c |
| IN-05 | info | skipped | 35-REVIEW-FIX.md; open by design: decision D-12 defines plan-less phases as not built (changing it is a scope decision and churns golden files) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
