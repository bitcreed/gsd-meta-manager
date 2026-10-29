---
phase: 28
review: 28-REVIEW.md
titles: json
findings:
  - id: WR-02
    severity: warning
    disposition: deferred
    title: "The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink"
  - id: WR-04
    severity: warning
    disposition: deferred
    title: "The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items"
  - id: WR-09
    severity: warning
    disposition: open
    title: "A launcher binary under `/tmp` or `/run` is unreachable inside the sandbox (NEW)"
  - id: WR-10
    severity: warning
    disposition: open
    title: "A wrong-shape host managed policy value is dropped with no diagnostic, though a non-object file is refused (NEW)"
  - id: IN-05
    severity: info
    disposition: open
    title: "The UTS namespace does not hide the host's name (NEW)"
  - id: IN-06
    severity: info
    disposition: open
    title: "A failed `record_launch_posture` leaves the launch half recorded (NEW)"
  - id: IN-07
    severity: info
    disposition: open
    title: "Host managed `permissions` scalar settings are first-wins while every other host key is last-wins (NEW)"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "A malformed host managed `hooks` or `permissions` value replaces the launcher's gate (NEW)"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "bwrap is started without `--new-session` (controlling-terminal injection)"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The master prompt is passed as a positional that can be parsed as an option, and as a single argv string"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Tests (and the harness selftest) require `.planning/` content that the crate excludes"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "Float and budget flags are unvalidated; `inf`, `NaN` and negative values reach systemd and `claude`"
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
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "`bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "`install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process"
open: 5
total: 18
recorded: 2026-10-29T05:27:48.118Z
---

# Phase 28: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-02 | warning | deferred | still present; the drivers tmpfs is emitted only when run_dir.parent() equals project/DRIVER_RUNS_REL, and validate_run_dir returns the canonical path, so a symlinked .planning skips it silently; the review's ends_with(DRIVER_RUNS_REL) fix does not match its own .planning -> docs/planning example (planning vs .planning components) and a golden test deliberately pins the out-of-project case; a design choice is needed (carry the canonical drivers dir in SandboxPaths, or refuse the launch); todo 2026-10-28-sandbox-drivers-tmpfs-skipped-for-symlinked-planning.md [inferred — cleanup 2026-10-28] |
| WR-04 | warning | deferred | still present (blocking_recovery keeps the direction.items.is_empty() clause); dropping it as the review suggests would let a crashed run dispatch while DIRECTION.md is absent or its I-NN headers do not parse, which loosens a fail-closed gate; the alternative keeps the block and rewords both refusals, which changes driver-facing protocol text, the reconcile tests and possibly MASTER-PROMPT; todo 2026-10-28-d17-recovery-block-with-no-direction-items.md [inferred — cleanup 2026-10-28] |
| WR-09 | warning | open | - |
| WR-10 | warning | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| WR-08 | warning | fixed | quick 261028-t3f 6a55713: managed_settings ignores a host managed hooks or permissions value that is not a JSON object (string, array, null, number, bool), so the launcher's gate hooks and the DENY_TOOLS deny list survive whatever shape the host value has; RED-first bwrap seed test from host files on disk plus a shape table for both keys in both postures; doc claims corrected in config_dir.rs and docs/DRIVER-SANDBOX.md [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-01 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-06 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |
| IN-01 | info | fixed | 28-REVIEW-FIX.md (not in the current review) |
| IN-02 | info | fixed | 28-REVIEW-FIX.md (not in the current review) |
| IN-03 | info | fixed | 28-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | skipped | 28-REVIEW-FIX.md (not in the current review) |
| CR-01 | critical | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
