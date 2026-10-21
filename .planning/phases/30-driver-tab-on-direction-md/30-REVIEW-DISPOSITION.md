---
phase: 30
review: 30-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "The launch form accepts a model name that the detached launch then rejects"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Run-directory files are opened without a file-type guard, although the phase hardens one reader against exactly that"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`steer_run` and `apply_edit` rewrite DIRECTION.md even when they changed nothing, then say \"Nothing was written\""
  - id: WR-04
    severity: warning
    disposition: open
    title: "Two line-number bases are both labelled `DIRECTION.md:<line>`"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The launch form puts the spend and \"keeps running\" disclosure last, where short terminals clip it first"
  - id: WR-06
    severity: warning
    disposition: open
    title: "`$VISUAL` / `$EDITOR` with arguments cannot be launched, and Phase 30 now depends on this path for every edit"
  - id: IN-01
    severity: info
    disposition: open
    title: "`set_item_field` mishandles a heading on an unterminated last line"
  - id: IN-02
    severity: info
    disposition: open
    title: "The slug input stays \"pending\" when the request is refused before it starts"
  - id: IN-03
    severity: info
    disposition: open
    title: "The launch form and the launcher read the `control` word with different readers"
open: 9
total: 9
recorded: 2026-10-21T03:24:22.405Z
---

# Phase 30: Code Review Disposition

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

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
