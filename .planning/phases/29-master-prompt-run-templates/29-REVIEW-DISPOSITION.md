---
phase: 29
review: 29-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "DIRECTION lint and the store's parser read different plans, so `driver check` can certify a file the store reads differently"
  - id: WR-02
    severity: warning
    disposition: open
    title: "The human `pause`/`stop` control fails open on a read error and on any non-lowercase spelling"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "DIRECTION.md content reaches the terminal unescaped through lint findings"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`mentions_phase` attributes a decimal phase to its integer parent, so the gap-round gate denies the wrong dispatch"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "DIRECTION template promises an `isolation:` knob the master prompt never reads"
  - id: WR-06
    severity: warning
    disposition: open
    title: "The \"reconcile first\" refusal is enforced only on the `dispatched` log record, not on the dispatch itself"
  - id: WR-07
    severity: warning
    disposition: open
    title: "The H2 acceptance case cannot stage its premise, and the committed evidence is an unresolved FAIL"
  - id: IN-01
    severity: info
    disposition: skipped
    title: "Prompt uses `$RUN_DIR` and `$PROJECT_ROOT` as if they were environment variables"
  - id: IN-02
    severity: info
    disposition: open
    title: "`pa_selftest` skip logic for a missing spike directory has the wrong precedence"
  - id: IN-03
    severity: info
    disposition: open
    title: "Preflight reorders refusals: a paused or invalid DIRECTION now masks \"already launched\""
  - id: IN-04
    severity: info
    disposition: fixed
    title: "`DEFAULT_GAP_CAP` duplicates the CLI default"
  - id: IN-05
    severity: info
    disposition: skipped
    title: "`shell_quote` is lossy for non-UTF-8 paths"
open: 7
total: 12
recorded: 2026-10-20T15:18:58.385Z
---

# Phase 29: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | fixed | 29-REVIEW-FIX.md |
| WR-04 | warning | open | - |
| WR-05 | warning | fixed | 29-REVIEW-FIX.md |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| IN-01 | info | skipped | 29-REVIEW-FIX.md |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | fixed | 29-REVIEW-FIX.md |
| IN-05 | info | skipped | 29-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
