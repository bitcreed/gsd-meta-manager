---
phase: 28
review: 28-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "`bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "bwrap is started without `--new-session` (controlling-terminal injection)"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Credential hiding is a denylist and is incomplete (env and home files), while the network is open"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The master prompt is passed as a positional that can be parsed as an option, and as a single argv string"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "`RERUN=1` deletes the committed evidence BEFORE any gate that can refuse the rerun"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Tests (and the harness selftest) require `.planning/` content that the crate excludes"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "Float and budget flags are unvalidated; `inf`, `NaN` and negative values reach systemd and `claude`"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "`install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "`DRIVER_LOCK_HELD=1` is exported to the driver but nothing in `src/` reads it"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "Module documentation drift in `process.rs` and `auth.rs`"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "Harness-tests pass vacuously when their tools are missing"
  - id: IN-04
    severity: info
    disposition: skipped
    title: "The campaign ledger is read-modify-append without a lock"
open: 0
total: 12
recorded: 2026-10-20T01:06:13.950Z
---

# Phase 28: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 28-REVIEW-FIX.md |
| WR-01 | warning | fixed | 28-REVIEW-FIX.md |
| WR-02 | warning | fixed | 28-REVIEW-FIX.md |
| WR-03 | warning | fixed | 28-REVIEW-FIX.md |
| WR-04 | warning | fixed | 28-REVIEW-FIX.md |
| WR-05 | warning | fixed | 28-REVIEW-FIX.md |
| WR-06 | warning | fixed | 28-REVIEW-FIX.md |
| WR-07 | warning | fixed | 28-REVIEW-FIX.md |
| IN-01 | info | fixed | 28-REVIEW-FIX.md |
| IN-02 | info | fixed | 28-REVIEW-FIX.md |
| IN-03 | info | fixed | 28-REVIEW-FIX.md |
| IN-04 | info | skipped | 28-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
