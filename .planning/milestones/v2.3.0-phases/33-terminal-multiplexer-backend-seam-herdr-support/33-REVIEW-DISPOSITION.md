---
phase: 33
review: 33-REVIEW.md
titles: json
findings:
  - id: IN-06
    severity: info
    disposition: open
    title: "`same_dir` adds unbounded filesystem I/O per herdr pane on the UI thread"
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
open: 1
total: 9
recorded: 2026-11-11T01:50:39.935Z
---

# Phase 33: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| IN-06 | info | open | - |
| WR-01 | warning | fixed | 33-REVIEW-FIX.md (not in the current review) |
| WR-02 | warning | fixed | 33-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 33-REVIEW-FIX.md (not in the current review) |
| IN-01 | info | fixed | 33-REVIEW-FIX.md (not in the current review) |
| IN-02 | info | fixed | 33-REVIEW-FIX.md (not in the current review) |
| IN-03 | info | fixed | 33-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | fixed | 33-REVIEW-FIX.md (not in the current review) |
| IN-05 | info | fixed | 33-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
