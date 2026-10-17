---
phase: 26
review: 26-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "`cp -r` nests `verification/` into itself on `RELAUNCH=1`"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Transient git failure regresses the feed to generation 0"
  - id: WR-03
    severity: warning
    disposition: open
    title: "A9 FAILs a legitimately converged run"
  - id: WR-04
    severity: warning
    disposition: open
    title: "A8 slug matching false-positives on common English words"
  - id: WR-05
    severity: warning
    disposition: open
    title: "A11 treats any dispatch right after a deny as denied"
  - id: WR-06
    severity: warning
    disposition: open
    title: "`SEED VALID` is emitted while A10 is UNDETERMINED or NOT-REACHED"
  - id: WR-07
    severity: warning
    disposition: open
    title: "Feed watcher has no error handling and nothing checks its liveness"
  - id: WR-08
    severity: warning
    disposition: open
    title: "Watcher and stand-in are still running while results are copied"
  - id: WR-09
    severity: warning
    disposition: open
    title: "Unvalidated label and path feed destructive `rm -rf`, `sed` and results overwrite"
  - id: WR-10
    severity: warning
    disposition: open
    title: "`dup_check.py` ignores workers whose parent metadata is missing"
  - id: IN-01
    severity: info
    disposition: open
    title: "Three divergent copies of the LOG parser"
  - id: IN-02
    severity: info
    disposition: open
    title: "`acceptance.py` module docstring and usage text are stale"
  - id: IN-03
    severity: info
    disposition: open
    title: "Temp DIRECTION file leaks if `launcher.py init` fails"
  - id: IN-04
    severity: info
    disposition: open
    title: "`stage_cost.py` silently prices unmatched models at $0"
  - id: IN-05
    severity: info
    disposition: open
    title: "Snapshot status regex fails on quoted values"
  - id: IN-06
    severity: info
    disposition: open
    title: "Auth-failure relaunch detection is brittle"
  - id: IN-07
    severity: info
    disposition: open
    title: "Result copy errors are swallowed"
open: 17
total: 17
recorded: 2026-10-17T03:18:36.358Z
---

# Phase 26: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| WR-10 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
