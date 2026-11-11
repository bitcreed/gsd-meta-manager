---
phase: 33
review: 33-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "herdr cwd-only tier can focus a pane that does not run the session, and it pre-empts tmux's exact tty match"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "herdr pid tier is gated behind a byte-exact cwd comparison, so it never runs for symlinked or non-canonical project paths"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "`MuxError::is_try_next` and the documented error classes are not applied by either seam function"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "`Detection.inside` can never be false, so `order_innermost`'s `inside` check guards nothing"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "`ExecArgv` refusals name \"a tmux window\" when raised on the herdr path"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "`parse_layout_apply_reply` treats `\"error\": null` as a failure and never checks the echoed id"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "Blocking bounds are uneven, and an abandoned connect thread is leaked per timeout"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "Test listeners in `mod.rs` and `herdr.rs` can hang the suite if the client never connects"
open: 0
total: 8
recorded: 2026-11-11T01:36:36.984Z
---

# Phase 33: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 33-REVIEW-FIX.md |
| WR-02 | warning | fixed | 33-REVIEW-FIX.md |
| WR-03 | warning | fixed | 33-REVIEW-FIX.md |
| IN-01 | info | fixed | 33-REVIEW-FIX.md |
| IN-02 | info | fixed | 33-REVIEW-FIX.md |
| IN-03 | info | fixed | 33-REVIEW-FIX.md |
| IN-04 | info | fixed | 33-REVIEW-FIX.md |
| IN-05 | info | fixed | 33-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
