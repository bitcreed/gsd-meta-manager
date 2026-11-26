---
phase: 36
review: 36-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "A missing or `.planning`-less project shows `·` (\"nothing waits on you\") instead of an absent value"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Deleting and re-adding an alias keeps the old project's digest indefinitely"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "`ClauseForm::CountsOnly` still lists every not-built phase"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "A panic in the owner reader now also drops the project-state refresh"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Owner focus hands keys to the phase list while the footer still advertises the Owner keys"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "The 500 ms reparse dedup can leave the digest (and so the You cell, rank and filter) stale indefinitely"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "Ages and the `stale` flag are frozen at read time"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "The `LauncherMarkerScanned` comment now sits above the `OwnerDigestLoaded` arm"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "The fold-line ladder ignores the 2-cell `> ` cursor prefix"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "The dashboard and the Owner block disagree on what \"old\" counts"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "The mouse wheel over the Owner block scrolls the phase list and drops Owner focus"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "`block_height` can overflow `u16`"
open: 0
total: 12
recorded: 2026-11-15T02:02:42.488Z
---

# Phase 36: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 36-REVIEW-FIX.md |
| WR-02 | warning | fixed | 36-REVIEW-FIX.md |
| WR-03 | warning | fixed | 36-REVIEW-FIX.md |
| WR-04 | warning | fixed | 36-REVIEW-FIX.md |
| WR-05 | warning | fixed | 36-REVIEW-FIX.md |
| WR-06 | warning | fixed | 36-REVIEW-FIX.md |
| WR-07 | warning | fixed | 36-REVIEW-FIX.md |
| IN-01 | info | fixed | 36-REVIEW-FIX.md |
| IN-02 | info | fixed | 36-REVIEW-FIX.md |
| IN-03 | info | fixed | 36-REVIEW-FIX.md |
| IN-04 | info | fixed | 36-REVIEW-FIX.md |
| IN-05 | info | fixed | 36-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
