---
phase: 28
review: 28-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "`bind_private` now replaces whatever sits at the target path (stage-then-`rename`), so the \"refuse a live socket\" rule has a race"
  - id: WR-02
    severity: warning
    disposition: open
    title: "The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Run-store and CLI reads of `DIRECTION.md` are unbounded and blocking, unlike the TUI reader hardened for the same threat"
  - id: WR-04
    severity: warning
    disposition: open
    title: "The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items"
  - id: WR-05
    severity: warning
    disposition: open
    title: "macOS enablement is half applied (TUI still gates on Linux and shows a contradictory message)"
  - id: WR-06
    severity: warning
    disposition: open
    title: "Credential hiding is still a denylist; `$HOME` is otherwise readable while the network is open (prior WR-02, reduced)"
  - id: IN-01
    severity: info
    disposition: open
    title: "Campaign ledger check-and-append race is still open (prior IN-04)"
  - id: IN-02
    severity: info
    disposition: open
    title: "The `tty_open` probe cannot fail without a controlling terminal"
  - id: IN-03
    severity: info
    disposition: open
    title: "Signal escalation counts \"stop already set\", not \"second signal\""
  - id: IN-04
    severity: info
    disposition: open
    title: "Doc drift and small leftovers"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "`bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "`install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process"
open: 10
total: 12
recorded: 2026-10-24T04:14:55.006Z
---

# Phase 28: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| CR-01 | critical | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
