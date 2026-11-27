---
phase: 34
review: 34-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "A VERIFICATION with an unterminated or non-mapping frontmatter block still reads as clean (WR-02 fixed only half the class)"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "The new G0 `HANDOFF.json` read is an unbounded, blocking `read_to_string` in a reader this phase hardens against hostile project trees"
  - id: IN-01
    severity: info
    disposition: open
    title: "`inbox::phase_key` orders decimal phases lexically, unlike the digest's own sort"
  - id: IN-02
    severity: info
    disposition: open
    title: "The 1 MiB cap makes an oversized artifact indistinguishable from an absent one in `disk_status`"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Quadratic fence rescan per heading in `deferred-items.md` hangs the reader on a hostile file"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The 1 MiB read cap is bypassed by `parse_project_state`, which reads the same files with an unbounded `read_to_string`"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "Impossible-state guard: `file_unreadable` in `apply_r7` is always false at its use"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Wall-clock assertions make `a_cold_inbox_over_twenty_projects_is_under_a_second` flaky"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "`fix_phases` compares raw phase tokens, unlike every other phase comparison"
  - id: IN-04
    severity: info
    disposition: open
    title: "The parse cache is bounded by entry count, not bytes"
  - id: IN-05
    severity: info
    disposition: open
    title: "`inbox` reports zero pending todos when listing them failed"
open: 4
total: 11
recorded: 2026-11-27T07:16:11.918Z
---

# Phase 34: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 34-REVIEW-FIX.md |
| WR-02 | warning | fixed | 34-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| CR-01 | critical | fixed | 34-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 34-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 34-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 34-REVIEW-FIX.md (not in the current review) |
| IN-03 | info | fixed | 34-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | open | - (not in the current review) |
| IN-05 | info | open | - (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
