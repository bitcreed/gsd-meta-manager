---
phase: 37
review: 37-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "Action keys `v`, `o`, `A`, `y` and `g` ignore key modifiers; Ctrl-V, key repeat or an unbracketed paste launches sessions with no confirm"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "A failed spawn, an unwritable temp file or an unreadable stderr sink is reported as \"Acknowledge timed out after 10 s\""
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "Cursor re-anchor lands on the fold line instead of the previous row when the last row vanishes (and the test cannot see it)"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "`--ws` value skips the flag-shape guard that every other acknowledge argument gets"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "The acknowledge launcher is resolved project-local first, so a registered project can supply the code that runs after Yes"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "The file viewer silently drops everything past line 65,535 and reads past its size cap on a swapped file"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "`owner_ack_busy` is keyed by file basename, so same-named files in different workstreams or milestones block each other"
  - id: IN-02
    severity: info
    disposition: skipped
    title: "`v` does not scrub `GSD_WORKSTREAM` and the session-pointer variables from the launched session"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "`focus_status` keys off the literal prefix `\"Switched to\"` of another module's message"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "The \"10 s\" in the acknowledge timeout text and a second `ACK_BUDGET` definition are not tied together"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "The OSC 52 and copy-size messages are slightly inaccurate or unbounded for terminals"
  - id: IN-06
    severity: info
    disposition: fixed
    title: "`start_copy_with` has no guard against overlapping copies"
open: 0
total: 12
recorded: 2026-11-15T07:00:34.611Z
---

# Phase 37: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 37-REVIEW-FIX.md |
| WR-02 | warning | fixed | 37-REVIEW-FIX.md |
| WR-03 | warning | fixed | 37-REVIEW-FIX.md |
| WR-04 | warning | fixed | 37-REVIEW-FIX.md |
| WR-05 | warning | fixed | 37-REVIEW-FIX.md |
| WR-06 | warning | fixed | 37-REVIEW-FIX.md |
| IN-01 | info | fixed | 37-REVIEW-FIX.md |
| IN-02 | info | skipped | 37-REVIEW-FIX.md |
| IN-03 | info | fixed | 37-REVIEW-FIX.md |
| IN-04 | info | fixed | 37-REVIEW-FIX.md |
| IN-05 | info | fixed | 37-REVIEW-FIX.md |
| IN-06 | info | fixed | 37-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
