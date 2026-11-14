---
phase: 34
review: 34-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "Quadratic fence rescan per heading in `deferred-items.md` hangs the reader on a hostile file"
  - id: WR-01
    severity: warning
    disposition: open
    title: "Containment check is vacuous above the final path component, so the reader follows a symlinked `phases/` or `workstreams/` out of the project"
  - id: WR-02
    severity: warning
    disposition: open
    title: "A VERIFICATION file whose frontmatter does not parse reads as clean"
  - id: WR-03
    severity: warning
    disposition: open
    title: "The 1 MiB read cap is bypassed by `parse_project_state`, which reads the same files with an unbounded `read_to_string`"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Impossible-state guard: `file_unreadable` in `apply_r7` is always false at its use"
  - id: WR-05
    severity: warning
    disposition: open
    title: "Wall-clock assertions make `a_cold_inbox_over_twenty_projects_is_under_a_second` flaky"
  - id: IN-01
    severity: info
    disposition: open
    title: "Stale `allow(dead_code)` attributes hide an unused production function"
  - id: IN-02
    severity: info
    disposition: open
    title: "Item ids can collide after `terminal_capped` truncation"
  - id: IN-03
    severity: info
    disposition: open
    title: "`fix_phases` compares raw phase tokens, unlike every other phase comparison"
  - id: IN-04
    severity: info
    disposition: open
    title: "The parse cache is bounded by entry count, not bytes"
  - id: IN-05
    severity: info
    disposition: open
    title: "`inbox` reports zero pending todos when listing them failed"
open: 11
total: 11
recorded: 2026-11-14T19:40:38.418Z
---

# Phase 34: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
