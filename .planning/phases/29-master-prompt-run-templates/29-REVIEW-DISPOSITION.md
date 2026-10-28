---
phase: 29
review: 29-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "lint and store still disagree on which line is `- control:` (WR-01 only half closed)"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`managed_settings` lets a non-object host `hooks`/`permissions` replace the launcher's gate and deny list"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`hook_is_kept` treats any script under `~/.claude/hooks/` or `~/.claude/scripts/` as a GSD hook"
  - id: IN-01
    severity: info
    disposition: open
    title: "store comment says \"every other event still records\" when DIRECTION.md is unreadable, but item events are rejected"
  - id: IN-02
    severity: info
    disposition: open
    title: "`filter_user_hooks` keeps any event entry that has no `hooks` array"
  - id: IN-03
    severity: info
    disposition: open
    title: "`next` refusal escapes the target twice"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "`mentions_phase` attributes a decimal phase to its integer parent, so the gap-round gate denies the wrong dispatch"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "DIRECTION template promises an `isolation:` knob the master prompt never reads"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "The \"reconcile first\" refusal is enforced only on the `dispatched` log record, not on the dispatch itself"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "The H2 acceptance case cannot stage its premise, and the committed evidence is an unresolved FAIL"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "`DEFAULT_GAP_CAP` duplicates the CLI default"
  - id: IN-05
    severity: info
    disposition: skipped
    title: "`shell_quote` is lossy for non-UTF-8 paths"
open: 6
total: 12
recorded: 2026-10-28T23:48:38.570Z
---

# Phase 29: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| WR-04 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-06 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | fixed | 29-REVIEW-FIX.md (not in the current review) |
| IN-05 | info | skipped | 29-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
