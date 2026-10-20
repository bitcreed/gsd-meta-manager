---
phase: 28
review: 28-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "`bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177"
  - id: WR-01
    severity: warning
    disposition: open
    title: "bwrap is started without `--new-session` (controlling-terminal injection)"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Credential hiding is a denylist and is incomplete (env and home files), while the network is open"
  - id: WR-03
    severity: warning
    disposition: open
    title: "The master prompt is passed as a positional that can be parsed as an option, and as a single argv string"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`RERUN=1` deletes the committed evidence BEFORE any gate that can refuse the rerun"
  - id: WR-05
    severity: warning
    disposition: open
    title: "Tests (and the harness selftest) require `.planning/` content that the crate excludes"
  - id: WR-06
    severity: warning
    disposition: open
    title: "Float and budget flags are unvalidated; `inf`, `NaN` and negative values reach systemd and `claude`"
  - id: WR-07
    severity: warning
    disposition: open
    title: "`install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process"
  - id: IN-01
    severity: info
    disposition: open
    title: "`DRIVER_LOCK_HELD=1` is exported to the driver but nothing in `src/` reads it"
  - id: IN-02
    severity: info
    disposition: open
    title: "Module documentation drift in `process.rs` and `auth.rs`"
  - id: IN-03
    severity: info
    disposition: open
    title: "Harness-tests pass vacuously when their tools are missing"
  - id: IN-04
    severity: info
    disposition: open
    title: "The campaign ledger is read-modify-append without a lock"
open: 12
total: 12
recorded: 2026-10-20T00:40:33.015Z
---

# Phase 28: Code Review Disposition

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
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
