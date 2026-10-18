---
phase: 26
review: 26-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`cp -r` nests `verification/` into itself on `RELAUNCH=1`"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Transient git failure regresses the feed to generation 0"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "A9 FAILs a legitimately converged run"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "A8 slug matching false-positives on common English words"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "A11 treats any dispatch right after a deny as denied"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "`SEED VALID` is emitted while A10 is UNDETERMINED or NOT-REACHED"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "Feed watcher has no error handling and nothing checks its liveness"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "Watcher and stand-in are still running while results are copied"
  - id: WR-09
    severity: warning
    disposition: fixed
    title: "Unvalidated label and path feed destructive `rm -rf`, `sed` and results overwrite"
  - id: WR-10
    severity: warning
    disposition: fixed
    title: "`dup_check.py` ignores workers whose parent metadata is missing"
  - id: IN-01
    severity: info
    disposition: deferred
    title: "Three divergent copies of the LOG parser"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "`acceptance.py` module docstring and usage text are stale"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "Temp DIRECTION file leaks if `launcher.py init` fails"
  - id: IN-04
    severity: info
    disposition: deferred
    title: "`stage_cost.py` silently prices unmatched models at $0"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "Snapshot status regex fails on quoted values"
  - id: IN-06
    severity: info
    disposition: deferred
    title: "Auth-failure relaunch detection is brittle"
  - id: IN-07
    severity: info
    disposition: fixed
    title: "Result copy errors are swallowed"
open: 0
total: 17
recorded: 2026-10-17T03:18:36.358Z
---

# Phase 26: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 2cc903c (26.1-03: run.sh `cp -a` snapshots, no nesting on RELAUNCH) |
| WR-02 | warning | fixed | fb022c2 (26.1-02: drift.py generation never regresses on a git failure) |
| WR-03 | warning | fixed | 3b53276 (26.1-02: A9 accepts verifier runs = count + 1 on a converged run) |
| WR-04 | warning | fixed | 3b53276 (26.1-02: A8 counts only quoted or backticked titles and slugs) |
| WR-05 | warning | fixed | 3b53276 (26.1-02: A11 look-behind removed) |
| WR-06 | warning | fixed | f909f93 (26.1-02: SEED VALID needs A10 PASS) |
| WR-07 | warning | fixed | 2cc903c (26.1-03: run.sh aborts on a dead watcher, WATCHER_ALIVE in run-env), fb022c2 (26.1-02: drift.py watch loop survives poll errors) |
| WR-08 | warning | fixed | 2cc903c (26.1-03: watcher and stand-in reaped before the results copy) |
| WR-09 | warning | fixed | 2cc903c (26.1-03: label guard and no-wipe rule in run.sh), 901abf6 (26.1-03: setup-scratch.sh basename guard) |
| WR-10 | warning | fixed | d724efe (26.1-02: dup_check treats an orphan-parent node as top-level) |
| IN-01 | info | deferred | shared LOG parser is a refactor; new 26.1 tools import dup_check.parse_log instead of adding a copy |
| IN-02 | info | fixed | f909f93 (26.1-02: acceptance.py docstring and usage rewritten) |
| IN-03 | info | fixed | 901abf6 (26.1-03: setup-scratch.sh cleans the temp DIRECTION file) |
| IN-04 | info | deferred | stage_cost.py CHECK line already reports MISMATCH when an unpriced model leaves the total short |
| IN-05 | info | fixed | fb022c2 (26.1-02: drift.py reads a quoted status) |
| IN-06 | info | deferred | auth relaunch path unchanged in 26.1; RELAUNCHES=0 in every recorded run |
| IN-07 | info | fixed | 2cc903c (26.1-03: run.sh reports a failed results copy and exits 1) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
