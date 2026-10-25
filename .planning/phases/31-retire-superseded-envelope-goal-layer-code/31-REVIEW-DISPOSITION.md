---
phase: 31
review: 31-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Opt-in disclosure still describes retired model seams and a re-confirmation that nothing performs"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Opt-in prompts and docs claim the push step reads a branch namespace from the opt-in; it does not"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Unregistering a project is no longer guarded against a live launcher run"
  - id: WR-04
    severity: warning
    disposition: open
    title: "PR-ledger refusal message cites a \"5-second deadline\" the push step no longer has"
  - id: WR-05
    severity: warning
    disposition: open
    title: "SAFE-04 redaction can be bypassed when the captured stderr line exceeds the 4 KiB tail window"
  - id: IN-01
    severity: info
    disposition: open
    title: "Orphaned comment block above the `Driverctl` arm describes arms that no longer exist"
  - id: IN-02
    severity: info
    disposition: open
    title: "Slow-tick comments still reference the deleted reconciliation probe, inbox and journal"
  - id: IN-03
    severity: info
    disposition: open
    title: "Opt-in toggle popup dims the dashboard even though it is only reachable from the Driver tab"
  - id: IN-04
    severity: info
    disposition: open
    title: "User-visible Preferences text names an internal phase number"
  - id: IN-05
    severity: info
    disposition: open
    title: "Leftover naming and attributes from retired code"
  - id: IN-06
    severity: info
    disposition: open
    title: "Release docs and packaging comments retain stale claims"
open: 11
total: 11
recorded: 2026-10-25T07:02:44.875Z
---

# Phase 31: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
