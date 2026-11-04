---
phase: quick-261103-qqm
plan: 01
subsystem: oneoff
tags: [oneoff, resume-chain, recycle, land, run-lock]
status: complete
requires:
  - quick-261103-o2r
  - quick-261103-pd7
provides:
  - "resume_successors / resume_predecessors: bounded, link-checked chain walks (store.rs)"
  - "successor_landing_branch / contained_candidate / close_out_predecessors (recycle.rs)"
  - "chain_agent_commits (land.rs), unlink_if_held shared by LandLock::drop and run_foreground"
affects: [task land merge, task list scan, TUI start scan, run_foreground]
key-files:
  modified:
    - src/oneoff/store.rs
    - src/oneoff/recycle.rs
    - src/oneoff/land.rs
    - src/oneoff/launch.rs
    - tests/oneoff_e2e.rs
    - docs/CONFIGURATION.md
decisions:
  - "I-01..I-07 applied as written in the plan (see below)"
commits: 3
plan_head_before: 69c1c4f2a9a3783d8f45ee64e356d599e9d6b160
plan_head_after: c89c74a4786715f3cf236b05ef3d98d4b600acc8
actuals:
  tokens: 60000
  tasks: 3
  commits: 3
---

# Phase quick-261103-qqm Plan 01: One-off resume-chain cleanup Summary

A landed successor now closes out every predecessor in its resume chain (recorded
`already-landed`, branch deleted, clone removed), the scan closes out predecessors an
older landing stranded, the ff message counts the agent's commits across the chain, and no
`run.lock` outlives its run.

## Commits

| Task | Commit | Subject |
| ---- | ------ | ------- |
| 1 | 225041b | close out a resume chain when its successor lands and count the chain's agent commits |
| 2 | 54275fd | scan closes out stranded resume predecessors and run.lock goes with its run |
| 3 | c89c74a | document resume-chain close-out and run.lock removal |

## Tracer: failing line at HEAD 69c1c4f

`resume_chain_landing_closes_out_the_predecessor` failed before the change on the message
assertion:

```
task land merge: landed 2 commit(s) of oneoff-20261104-012922-2 on main (1 by the agent; now at 17931cfb29ea)
task land merge: removed the run's clone; kept its record (4 KiB)
```

No predecessor line, and the count said 1. It passes after Task 1.

## Test results

`cargo test --no-fail-fast` (raw log `target/qqm-host.log`): 59 `test result: ok` lines,
3587 passed, 0 failed, 17 ignored (baseline 3577 + 10 new tests). `cargo clippy
--all-targets -- -D warnings` clean; `cargo fmt --check` clean.

New tests: store `resume_chain_walk_*` (4); recycle `resume_chain_branch_*` (1),
`stale_run_lock_*` (1); e2e `resume_chain_landing_closes_out_the_predecessor`,
`resume_chain_predecessor_left_by_an_older_landing_is_closed_by_the_scan`,
`resume_chain_discarded_successor_keeps_the_predecessor`, `run_lock_is_removed_when_a_run_ends`.
The `--from master` e2e now expects `(2 by the agent`. The recycle leftover test lost its
I-17 clause. The `run_lock` e2e was confirmed red with the launcher unlink disabled.

## Inferred decisions as applied `[inferred — audit]`

- **I-01** Predecessors are recorded with `LandingKind::AlreadyLanded` through
  `land::record_already_landed`; no new kind.
- **I-02** `successor_landing_branch` requires Finished + head + `resumed_by` + no own
  work-on-target landing; walks successors with both-direction link checks; first successor
  with a Merged/AlreadyLanded landing gives its latest landing's detail if
  `plausible_branch`. Pushed never qualifies; Paused / 0 commits do not disqualify.
  `tend_contained` still requires the head to be an ancestor of the branch tip.
- **I-03** The o2r I-17 rule and the `tracked_at_head` parameter are removed from
  `leftover_slim_note` / `tend_run`.
- **I-04** `land::merge` calls `recycle::close_out_predecessors` after the successor's slim,
  using `tend_contained` (non-waiting `try_land_lock`); a held lock leaves the predecessor
  to the next scan (reproduced by the older-landing e2e). Output adds `<id>: ... recorded as
  landed` plus `<id>: removed its clone`.
- **I-05** `chain_agent_commits` sums `rev-list --count head ^base ^old_tip` over the run and
  its predecessors, falling back to recorded `commits`.
- **I-06** `run_foreground` unlinks `run.lock` via `unlink_if_held` (`RunLock::is_at`)
  before `drop(lock)`, unconditionally after the final record write; `clear_stale_run_lock`
  runs in `tend_run` right after the Finished/landing gate and is removed from
  `tidy_finished`. `run_detached`'s poll is unchanged.
- **I-07** `MAX_RESUME_CHAIN = 32`, visited set, ids only through `read_view`.

## Deviations from Plan

None in behaviour. Notes:

- The first draft of Task 1 left an unused `run_lock_files` test helper; it was removed
  before the Task 1 commit (Task 2's `run_lock` e2e checks `run.lock` directly).
- While proving the `run_lock` e2e red I reverted `launch.rs` with `git checkout`, which also
  dropped the uncommitted fix; I re-applied it identically before committing Task 2.
- The "Cleanup and recovery" docs also state the 32-link bound and the both-sides link rule.

## Known Stubs

None.

## Threat Flags

None. No new network, auth or trust-boundary surface; T-qqm-01..05 mitigations are as
planned (both-direction links, `plausible_branch`, ancestry re-check under the land lock,
`clone_preserved`, bounded walks, unlink only while held).

## Real-data cleanup (left to the orchestrator)

One `task list` on the gsd-meta-manager project should close out
`oneoff-20261104-005114` through the scan path that
`resume_chain_predecessor_left_by_an_older_landing_is_closed_by_the_scan` reproduces.

## Self-Check: PASSED

- Commits 225041b, 54275fd, c89c74a are ancestors of HEAD.
- All six modified files exist and are committed; working tree has only the untracked
  quick-task directory.

## Real-data cleanup (orchestrator, post-execution)

`target/release/gsd-meta-manager task list .` on 2026-11-03: recorded `oneoff-20261104-005114` as landed (already-landed), removed its clone (119 MB -> under 1 MB; run.json, stream.jsonl, report kept), deleted branch `gsd-oneoff/oneoff-20261104-005114`, removed its stale `run.lock`. No `*.lock` remains under the project's oneoff dir. A second `task list` printed no warnings.
