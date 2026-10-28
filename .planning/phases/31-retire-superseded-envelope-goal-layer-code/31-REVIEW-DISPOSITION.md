---
phase: 31
review: 31-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "`DRIVER_TEARDOWN_GRACE` lower-bound guard now compares against a mirror of a deleted constant"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Comments and assertion messages in `session_detector.rs` still say this build's own executor emits `--session-id` and `codex exec`"
  - id: IN-01
    severity: info
    disposition: open
    title: "Comments still cite deleted files and line numbers"
  - id: IN-02
    severity: info
    disposition: open
    title: "Broken intra-doc link to a deleted constant"
  - id: IN-03
    severity: info
    disposition: open
    title: "Spawn-seam guard still names the process-wrap marker and a capability type that no longer exist"
  - id: IN-04
    severity: info
    disposition: open
    title: "Production-dead code is retained, and the module doc overstates its use"
  - id: IN-05
    severity: info
    disposition: open
    title: "Retargeted app tests keep \"driver maps\" names but now assert a non-driver map"
  - id: IN-06
    severity: info
    disposition: open
    title: "Src-wide negative guards for Codex bypass flags and MCP config went with their subjects"
  - id: IN-07
    severity: info
    disposition: open
    title: "`DriverOptIn` docs in `config.rs` still say the drift check runs at the spawn gate and that drift re-confirms"
  - id: IN-08
    severity: info
    disposition: open
    title: "`delete_confirm.rs` re-implements the \"driver is live\" predicate that `detail.rs` already has"
  - id: IN-09
    severity: info
    disposition: open
    title: "`OVERLONG_STDERR_NOTE` hard-codes the window size as a literal"
  - id: IN-10
    severity: info
    disposition: open
    title: "The unregister guard is advisory: it covers the cached scan only, and its limits are not stated to the user"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "Unregistering a project is no longer guarded against a live launcher run"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "PR-ledger refusal message cites a \"5-second deadline\" the push step no longer has"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "SAFE-04 redaction can be bypassed when the captured stderr line exceeds the 4 KiB tail window"
open: 12
total: 15
recorded: 2026-10-28T05:11:07.620Z
---

# Phase 31: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - (not in the current review) |
| IN-08 | info | open | - (not in the current review) |
| IN-09 | info | open | - (not in the current review) |
| IN-10 | info | open | - (not in the current review) |
| WR-03 | warning | fixed | 36e0805 (31-08) (not in the current review) |
| WR-04 | warning | fixed | 21b94df, a51b39a (31-09) (not in the current review) |
| WR-05 | warning | fixed | 21b94df (31-09) (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
