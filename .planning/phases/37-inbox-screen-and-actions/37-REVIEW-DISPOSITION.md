---
phase: 37
review: 37-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Action keys `v`, `o`, `A`, `y` and `g` ignore key modifiers; Ctrl-V, key repeat or an unbracketed paste launches sessions with no confirm"
  - id: WR-02
    severity: warning
    disposition: open
    title: "A failed spawn, an unwritable temp file or an unreadable stderr sink is reported as \"Acknowledge timed out after 10 s\""
  - id: WR-03
    severity: warning
    disposition: open
    title: "Cursor re-anchor lands on the fold line instead of the previous row when the last row vanishes (and the test cannot see it)"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`--ws` value skips the flag-shape guard that every other acknowledge argument gets"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The acknowledge launcher is resolved project-local first, so a registered project can supply the code that runs after Yes"
  - id: WR-06
    severity: warning
    disposition: open
    title: "The file viewer silently drops everything past line 65,535 and reads past its size cap on a swapped file"
  - id: IN-01
    severity: info
    disposition: open
    title: "`owner_ack_busy` is keyed by file basename, so same-named files in different workstreams or milestones block each other"
  - id: IN-02
    severity: info
    disposition: open
    title: "`v` does not scrub `GSD_WORKSTREAM` and the session-pointer variables from the launched session"
  - id: IN-03
    severity: info
    disposition: open
    title: "`focus_status` keys off the literal prefix `\"Switched to\"` of another module's message"
  - id: IN-04
    severity: info
    disposition: open
    title: "The \"10 s\" in the acknowledge timeout text and a second `ACK_BUDGET` definition are not tied together"
  - id: IN-05
    severity: info
    disposition: open
    title: "The OSC 52 and copy-size messages are slightly inaccurate or unbounded for terminals"
  - id: IN-06
    severity: info
    disposition: open
    title: "`start_copy_with` has no guard against overlapping copies"
open: 12
total: 12
recorded: 2026-11-15T07:00:19.953Z
---

# Phase 37: Code Review Disposition

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

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
