---
phase: 27
review: 27-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "`driverctl log` rejects any value that starts with `-`, where the argparse client accepted it"
  - id: WR-02
    severity: warning
    disposition: open
    title: "A failed `FileWatcher::new` leaves the terminal in raw mode on the alternate screen"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Run-start opens of untrusted run-dir names are unhardened (`DIRECTION.lock`, `state.json`)"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`probe_at` and `read_holder_at` can block, follow links and read without a bound"
  - id: WR-05
    severity: warning
    disposition: open
    title: "`serve` shutdown overwrites a driver's `paused` end with `stopped`"
  - id: WR-06
    severity: warning
    disposition: open
    title: "Log-directory fallback to `/tmp` is a predictable path in a shared directory"
  - id: IN-01
    severity: info
    disposition: open
    title: "Wrong binary name in the duplicate-alias hint"
  - id: IN-02
    severity: info
    disposition: open
    title: "A 447-byte DIRECTION.md frontmatter costs about 0.8 s of CPU per parse"
  - id: IN-03
    severity: info
    disposition: open
    title: "Module docs cite a RUSTSEC advisory in a parser the crate no longer uses"
  - id: IN-04
    severity: info
    disposition: open
    title: "`fm_status` regex lets `\\s*` cross a newline"
  - id: IN-05
    severity: info
    disposition: open
    title: "The lock module's \"worst case is a misleading string\" claim is no longer true"
  - id: IN-06
    severity: info
    disposition: open
    title: "Stale references to the deleted `drive` command and goal layer"
  - id: IN-07
    severity: info
    disposition: open
    title: "Report-body handling: Unicode line separators allowed, stdin unbounded, over-1-MiB reports rejected whole"
  - id: IN-08
    severity: info
    disposition: open
    title: "`read_driver_run_text` does not re-check size after reading and fails on non-UTF-8"
  - id: IN-09
    severity: info
    disposition: open
    title: "Test reliability notes"
  - id: IN-10
    severity: info
    disposition: open
    title: "Committed fixtures carry a real username, hostname and session id"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Gap scan reads driver-plantable files with no type or size guard, under the store mutex"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "Gap-round `seen` is keyed by digest alone, so identical files in different phases collapse"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "Client `connect` has no timeout, contradicting the documented total deadline"
  - id: WR-09
    severity: warning
    disposition: fixed
    title: "Wall-clock assertions in default-run tests are flake-prone"
open: 16
total: 20
recorded: 2026-10-29T00:24:12.282Z
---

# Phase 27: Code Review Disposition

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
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |
| IN-09 | info | open | - |
| IN-10 | info | open | - |
| CR-01 | critical | fixed | 27-REVIEW-FIX.md; guard still at HEAD: src/driverctl/scan.rs:24-29 MAX_VERIFICATION_BYTES + O_NONBLOCK/O_NOFOLLOW [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-07 | warning | fixed | 27-REVIEW-FIX.md; per-phase seen key still at HEAD: src/driverctl/counters.rs:81-84 [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-08 | warning | fixed | 27-REVIEW-FIX.md; total client deadline still at HEAD: src/driverctl_cli.rs:56,245 [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-09 | warning | fixed | 27-REVIEW-FIX.md; not re-raised by the HEAD review [inferred — cleanup 2026-10-28] (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.

## Prior-round findings whose ids were reused (round 1, review 822e4c7)

The 2026-10-28 HEAD re-review (eb7528b) renumbered from WR-01/IN-01, so the gate dropped these recorded
decisions. They are kept here as prose so the history survives. Each fix is still present at HEAD.
None of them targets code Phase 31 deleted: Phase 31 removed `drive`, the goal layer and the executors
under `src/driver/*` and `src/app.rs`. The driverctl serve, store, log and direction code they cite
still exists, so none is retired-by-31. [inferred — cleanup 2026-10-28]

- r1 WR-01, per-read timeout lets trickling clients hold slots: **fixed** (27-REVIEW-FIX.md). Evidence: TOTAL `CONN_TIMEOUT` deadline at src/driverctl/serve.rs:50,701.
- r1 WR-02, `fresh-exit` then shutdown leaves `running`: **fixed** (27-REVIEW-FIX.md). Evidence: the terminal-only keep at src/driverctl/store.rs:1371. HEAD WR-05 is a separate, newer gap in the same guard: `paused` is not terminal.
- r1 WR-03, LOG.md appended before state.json is durable: **fixed** (27-REVIEW-FIX.md).
- r1 WR-04, early returns after `begin_launch`: **fixed** (27-REVIEW-FIX.md).
- r1 WR-05, stale-socket replacement without a staleness check: **fixed** (27-REVIEW-FIX.md). Evidence: the D-21 rule at src/driverctl/serve.rs:24.
- r1 WR-06, control characters in header values: **fixed** (27-REVIEW-FIX.md). Evidence: src/driverctl/log.rs:149-155.
- r1 IN-01, orphaned state.json temps never swept: **deferred** (info, outside the --fix scope). It now looks addressed by `sweep_stale_temps`, which src/driverctl/serve.rs:83 imports.
- r1 IN-02, a ticker-thread panic is silent: **deferred** (info, outside the --fix scope).
- r1 IN-03, the replay test accepts a changed dispatch budget: **deferred** (info, outside the --fix scope).

All 16 HEAD findings (WR-01..06, IN-01..10) are NEW and stay `open`. No code was changed in this round.
