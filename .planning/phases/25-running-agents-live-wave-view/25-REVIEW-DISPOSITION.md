---
phase: 25
review: 25-REVIEW.md
titles: json
findings:
  - id: WR-07
    severity: warning
    disposition: open
    title: "A project whose only agents-scan output is worktree-only phases gets no view, so its ghost phases and `N wt phases` tag never show (new)"
  - id: WR-08
    severity: warning
    disposition: open
    title: "The per-call git budget does not bound the scan: a wedged filesystem still freezes every view for minutes (residual WR-02)"
  - id: WR-09
    severity: warning
    disposition: open
    title: "The fixer estimate reads the project's `.planning`, even though every neighbouring detector was made workstream-aware (new) [inferred]"
  - id: IN-01
    severity: info
    disposition: open
    title: "The handler comment says a fleet without agents never redraws, but any inserted view redraws on every scan (carried forward)"
  - id: IN-02
    severity: info
    disposition: open
    title: "Tests that go through `registered_adapters()` read the real `~/.claude` (carried forward; now three sites)"
  - id: IN-03
    severity: info
    disposition: open
    title: "The ledger read ignores relative `gitdir:` pointers (carried forward)"
  - id: IN-04
    severity: info
    disposition: open
    title: "`lock_released` reports \"released\" for worktrees the runtime may never have locked [inferred] (carried forward)"
  - id: IN-05
    severity: info
    disposition: open
    title: "A worktree's live children never count as running (carried forward) [inferred]"
  - id: IN-06
    severity: info
    disposition: open
    title: "`plan_state` re-spells the running predicate (carried forward)"
  - id: IN-07
    severity: info
    disposition: open
    title: "The \"gate runs before any git call or file read\" test does not test ordering (carried forward)"
  - id: IN-08
    severity: info
    disposition: open
    title: "Doc comments name the wrong git helper and the wrong glyph (new)"
  - id: IN-09
    severity: info
    disposition: open
    title: "A workstream named at the end of a sentence is not recognised as named (new)"
  - id: IN-10
    severity: info
    disposition: open
    title: "`summary_in_worktree` is always false for a plan whose phase directory exists only in the worktree (new)"
  - id: IN-11
    severity: info
    disposition: open
    title: "`stage_of` counts summaries and plans that the plan-pairing rules exclude (new)"
  - id: IN-12
    severity: info
    disposition: open
    title: "Integration tests silently pass when `git init` fails (new)"
  - id: IN-13
    severity: info
    disposition: open
    title: "The zero-write guard test cannot see several write paths (new)"
  - id: IN-14
    severity: info
    disposition: open
    title: "The `N wt phases` tag defeats the \"ends with `stalled` is yellow\" rule (new)"
  - id: IN-15
    severity: info
    disposition: open
    title: "Letter-suffixed plan ids are accepted by `worktrees.rs` and then abstained on everywhere else (new)"
  - id: IN-16
    severity: info
    disposition: open
    title: "An owner with no comparable start is judged dead when its cwd is outside the project roots [inferred]"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "A hung git call freezes the agents scan forever"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "The fixer estimate reads `NN-EVAL-REVIEW.md` instead of `NN-REVIEW.md`"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "Earlier review runs inflate `fixed`, which can exceed `total`, and a leftover REVIEW-FIX.md hides the count"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "The long-path prefix fallback can adopt another project's Claude directory"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "Finished orphan fixers outvote the running fixer and inflate `N fixers` and `fixed`"
open: 19
total: 24
recorded: 2026-10-29T00:37:32.503Z
---

# Phase 25: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |
| IN-09 | info | open | - |
| IN-10 | info | open | - |
| IN-11 | info | open | - |
| IN-12 | info | open | - |
| IN-13 | info | open | - |
| IN-14 | info | open | - |
| IN-15 | info | open | - |
| IN-16 | info | open | - |
| WR-02 | warning | fixed | 25-REVIEW-FIX.md; per-call git budget still at HEAD: src/state_reader/git_ops.rs:235,243-256 git_read_raw_within (aggregate scan bound is open as WR-08) [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-03 | warning | fixed | 25-REVIEW-FIX.md; NN-REVIEW.md-only filter still at HEAD: src/agents/fixers.rs:172-175 [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-04 | warning | fixed | 25-REVIEW-FIX.md; intersect-and-clamp still at HEAD: src/agents/fixers.rs:237-245 [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-05 | warning | fixed | 25-REVIEW-FIX.md; exact-encoded dirs only at HEAD: src/agents/adapters/claude.rs:282-307 [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-06 | warning | fixed | 25-REVIEW-FIX.md; running-first vote at HEAD: src/agents/fixers.rs:318-342 [inferred — cleanup 2026-10-28] (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.

## Earlier-round findings outside this ledger (reviews before 5579bbc)

CR-01 and WR-01 of the 25-07 gap-closure round were closed by plan 25-07 itself, not by a fix report, so no
ledger row carries them. The HEAD re-review (9faf0be) confirms both resolved. They are kept here as prose so the
history survives. [inferred — cleanup 2026-10-28]

- r-prior CR-01, orphaned worktrees took over the Status cell: **fixed** (plan 25-07). Evidence: the single `AgentLiveness::is_running` predicate at src/agents/mod.rs:104-106, with the age-out at src/agents/mod.rs:157-159.
- r-prior WR-01, orphans had equal weight in the active-phase vote: **fixed** (plan 25-07). Evidence: the tiered vote at src/agents/waves.rs:411-427.

IN-01..IN-07 keep their ids. The HEAD review re-confirmed each one as present (carried forward), and none was ever
triaged, so they stay `open`. WR-07..09 and IN-08..16 are NEW at HEAD and stay `open`. No code was changed in this
round. WR-09, IN-15 and IN-16 are reviewer inferences and are not yet confirmed. [inferred — cleanup 2026-10-28]
