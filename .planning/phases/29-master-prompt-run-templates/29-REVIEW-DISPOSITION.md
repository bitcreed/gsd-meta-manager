---
phase: 29
review: 29-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "lint and store still disagree on which line is `- control:` (WR-01 only half closed)"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`hook_is_kept` treats any script under `~/.claude/hooks/` or `~/.claude/scripts/` as a GSD hook"
  - id: WR-08
    severity: warning
    disposition: open
    title: "the read-side link-count refusal turns a held run lock into `Unknown` after any hard link is made (new, 2026-10-29)"
  - id: WR-09
    severity: warning
    disposition: open
    title: "managed `permissions` scalars resolve first-wins while every other managed key resolves last-wins (new, 2026-10-29)"
  - id: WR-10
    severity: warning
    disposition: open
    title: "wrong-shape and unrecognised host policy is dropped with no diagnostic, unlike the top-level refusal (new, 2026-10-29)"
  - id: IN-01
    severity: info
    disposition: open
    title: "store comment says \"every other event still records\" when DIRECTION.md is unreadable, but item events are rejected"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "`filter_user_hooks` keeps any event entry that has no `hooks` array"
  - id: IN-03
    severity: info
    disposition: open
    title: "`next` refusal escapes the target twice"
  - id: IN-06
    severity: info
    disposition: open
    title: "a refused or link-flipped run lock surfaces as \"Invalid argument\" with the cause discarded (new, 2026-10-29)"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "`managed_settings` lets a non-object host `hooks`/`permissions` replace the launcher's gate and deny list"
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
open: 8
total: 16
recorded: 2026-10-29T05:12:27.186Z
---

# Phase 29: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-03 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| WR-10 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | fixed | quick 261028-v2z bfbd231 (30 WR-13): managed settings transform runs well_formed_hooks before filter_user_hooks (src/launcher/config_dir.rs:617-619) and is_well_formed_hook_entry (:365-378) drops any entry that is not an object with an array `hooks`, so a flat entry never reaches the filter; the `return true` at :556-558 is now unreachable; verified at HEAD in the 2026-10-29 re-review [inferred — cleanup 2026-10-28] |
| IN-03 | info | open | - |
| IN-06 | info | open | - |
| WR-02 | warning | fixed | quick 261028-t3f 6a55713: same defect as 28-REVIEW WR-08; managed_settings ignores a host managed hooks or permissions value that is not a JSON object (string, array, null, number, bool), so the launcher's gate hooks and the DENY_TOOLS deny list survive whatever shape the host value has; RED-first bwrap seed test from host files on disk plus a shape table for both keys in both postures; doc claims corrected in config_dir.rs and docs/DRIVER-SANDBOX.md [inferred — cleanup 2026-10-28] |
| WR-04 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-06 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 29-REVIEW-FIX.md (not in the current review) |
| IN-04 | info | fixed | 29-REVIEW-FIX.md (not in the current review) |
| IN-05 | info | skipped | 29-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
