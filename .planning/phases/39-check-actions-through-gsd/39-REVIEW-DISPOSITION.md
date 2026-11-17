---
phase: 39
review: 39-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "Live-session settle copies the command and says \"paste there\" even when the focus failed"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Inbox `j` past the last item of the last row silently drops the item selection"
  - id: IN-01
    severity: info
    disposition: deferred
    title: "`move_instruction` panics on an inconsistent `SettleChoice`"
  - id: IN-02
    severity: info
    disposition: skipped
    title: "The `[x]settle` footer hint exists only on the widest inbox rung"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "The reason sentence is built twice"
open: 0
total: 5
recorded: 2026-11-17T21:47:30.951Z
---

# Phase 39: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | b7eb9d8b: lead follows the FOCUS_OK_PREFIX rule; test a_failed_focus_never_says_paste_there |
| WR-02 | warning | fixed | 614337c4: forward Leave at the last stop is a no-op; test j_past_the_last_item_of_the_last_stop_keeps_the_item |
| IN-01 | info | deferred | invariant held at every construction site today; encoding the target in Outcome::Move touches ~33 sites, a refactor beyond this review |
| IN-02 | info | skipped | deliberate width trade-off recorded in the code (keeps the 100x30 golden); x stays discoverable via help and README |
| IN-03 | info | fixed | 235a0cfd: reason_sentence() shared by instruction and move_instruction |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
