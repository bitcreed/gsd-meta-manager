---
phase: 28
review: 28-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`bind_private` now replaces whatever sits at the target path (stage-then-`rename`), so the \"refuse a live socket\" rule has a race"
  - id: WR-02
    severity: warning
    disposition: deferred
    title: "The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "Run-store and CLI reads of `DIRECTION.md` are unbounded and blocking, unlike the TUI reader hardened for the same threat"
  - id: WR-04
    severity: warning
    disposition: deferred
    title: "The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "macOS enablement is half applied (TUI still gates on Linux and shows a contradictory message)"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "Credential hiding is still a denylist; `$HOME` is otherwise readable while the network is open (prior WR-02, reduced)"
  - id: IN-01
    severity: info
    disposition: skipped
    title: "Campaign ledger check-and-append race is still open (prior IN-04)"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "The `tty_open` probe cannot fail without a controlling terminal"
  - id: IN-03
    severity: info
    disposition: skipped
    title: "Signal escalation counts \"stop already set\", not \"second signal\""
  - id: IN-04
    severity: info
    disposition: fixed
    title: "Doc drift and small leftovers"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "`bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "`install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process"
open: 0
total: 12
recorded: 2026-10-24T04:14:55.006Z
---

# Phase 28: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | fixed by quick 261028-n12 e98520c (bind_private publishes the staged socket with hard_link, so a file or live socket that appears at the target after vetting gives EEXIST and is left untouched; the staging-path length check stays because the bind itself needs the staged path to fit sun_path, and dropping it would only turn a clear refusal into EINVAL) [inferred — cleanup 2026-10-28] |
| WR-02 | warning | deferred | still present; the drivers tmpfs is emitted only when run_dir.parent() equals project/DRIVER_RUNS_REL, and validate_run_dir returns the canonical path, so a symlinked .planning skips it silently; the review's ends_with(DRIVER_RUNS_REL) fix does not match its own .planning -> docs/planning example (planning vs .planning components) and a golden test deliberately pins the out-of-project case; a design choice is needed (carry the canonical drivers dir in SandboxPaths, or refuse the launch); todo 2026-10-28-sandbox-drivers-tmpfs-skipped-for-symlinked-planning.md [inferred — cleanup 2026-10-28] |
| WR-03 | warning | fixed | fixed by quick 261028-n12 dc812c0 (RunStore::direction_read, preflight_direction and driver check read DIRECTION.md through read_run_file_capped: open_run_file plus the 1 MiB cap; a FIFO or over-cap file now denies the gate as unreadable instead of blocking) |
| WR-04 | warning | deferred | still present (blocking_recovery keeps the direction.items.is_empty() clause); dropping it as the review suggests would let a crashed run dispatch while DIRECTION.md is absent or its I-NN headers do not parse, which loosens a fail-closed gate; the alternative keeps the block and rewords both refusals, which changes driver-facing protocol text, the reconcile tests and possibly MASTER-PROMPT; todo 2026-10-28-d17-recovery-block-with-no-direction-items.md [inferred — cleanup 2026-10-28] |
| WR-05 | warning | fixed | 261028-n11 76a9ecb, de20ec3, 9b68124: TUI launch, steering and stop enabled on macOS (user decision 2026-10-28); portable host_name; umask test Linux-gated |
| WR-06 | warning | fixed | fixed by Phase 28.1 (D-16 $HOME tmpfs with only the derived allowlist re-bound; D-06 egress_env_sets overrides both proxy spellings and ALL_PROXY is stripped; /etc is a tmpfs with re-bound entries). --unsandboxed has no filesystem isolation by design (D-28..D-30) [inferred — cleanup 2026-10-28] |
| IN-01 | info | skipped | launcher-acceptance.sh is kept as a historical record and helper library. Paid campaigns are operator-run one at a time against one ledger, so the check-and-append never has a concurrent writer. prompt-acceptance.sh shares the pattern under the same single-operator rule, and flock is not on stock macOS [inferred — cleanup 2026-10-28] |
| IN-02 | info | fixed | fixed by quick 261028-n12 88a61ec (the tty_open == NONE assertion runs only when the test process has a controlling terminal; otherwise it prints a SKIP note) |
| IN-03 | info | skipped | latent: nothing but the first signal sets user_stop, and a launch is one process, so neither the early escalation nor the never-reset global is reachable. Revisit if the launcher is embedded [inferred — cleanup 2026-10-28] |
| IN-04 | info | fixed | fixed by quick 261028-n12 d344401 (launcher/mod.rs rule 6 names home and prompt as ungated too; duplicate awaiting-id paragraph removed from recovery_rejection; mode_config header lists WRITE_FIRST) |
| CR-01 | critical | fixed | 28-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 28-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
