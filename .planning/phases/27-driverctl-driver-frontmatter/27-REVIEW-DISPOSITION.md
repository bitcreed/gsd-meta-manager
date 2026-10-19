---
phase: 27
review: 27-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "Gap scan reads driver-plantable files with no type or size guard, under the store mutex"
  - id: WR-01
    severity: warning
    disposition: open
    title: "Connection timeout is per-read, so trickling clients hold all 32 slots indefinitely"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`end --reason fresh-exit` followed by shutdown leaves status `running`, so the reader derives a false `crashed`"
  - id: WR-03
    severity: warning
    disposition: open
    title: "LOG.md line is appended before `state.json` is durable, so a failed save leaves them diverged"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Early returns after `begin_launch` leave status `running` and a stale socket file"
  - id: WR-05
    severity: warning
    disposition: open
    title: "Stale-socket replacement never checks that the socket is actually stale"
  - id: WR-06
    severity: warning
    disposition: open
    title: "Driver-supplied header values may carry control characters other than CR/LF"
  - id: WR-07
    severity: warning
    disposition: open
    title: "Gap-round `seen` is keyed by digest alone, so identical files in different phases collapse"
  - id: WR-08
    severity: warning
    disposition: open
    title: "Client `connect` has no timeout, contradicting the documented total deadline"
  - id: WR-09
    severity: warning
    disposition: open
    title: "Wall-clock assertions in default-run tests are flake-prone"
  - id: IN-01
    severity: info
    disposition: open
    title: "Orphaned `state.json` temp files are never swept"
  - id: IN-02
    severity: info
    disposition: open
    title: "A panic in the ticker thread is silent and permanent"
  - id: IN-03
    severity: info
    disposition: open
    title: "Replay test silently accepts a changed dispatch budget"
open: 13
total: 13
recorded: 2026-10-19T06:02:40.141Z
---

# Phase 27: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
