---
phase: quick-261103-pd7
plan: 01
subsystem: oneoff
tags: [oneoff, land, recycle, land.lock, sandbox, testing]
status: complete
requires: []
provides:
  - LandingKind::AlreadyLanded (serde `already-landed`) and the landing it records
  - the scan records contained runs as already-landed
  - land.lock removed on every release (LandLock guard, RunLock::is_at)
  - suite passes inside a bwrap approximation of the one-off sandbox; TESTING.md says how to check
affects: [oneoff land, oneoff recycle scan, docs/TESTING.md, docs/CONFIGURATION.md]
requirements-completed: [QUICK-261103-pd7]
actuals:
  tokens: 10000
  tasks: 3
  commits: 3
plan_head_before: 4f38f18bd67ba0f674540360f85a416e57550b88
plan_head_after: 1766a37884b64cd438698177f8f3c9c0038db3b4
key-files:
  created: []
  modified:
    - src/oneoff/store.rs
    - src/oneoff/land.rs
    - src/oneoff/resolve.rs
    - src/oneoff/resume.rs
    - src/oneoff/recycle.rs
    - src/driver/lock.rs
    - src/task_cli.rs
    - src/ui/screens/oneoff.rs
    - src/ui/screens/render_escape_guard.rs
    - src/app.rs
    - tests/oneoff_e2e.rs
    - tests/driverctl_store.rs
    - tests/launcher_degraded.rs
    - docs/TESTING.md
    - docs/CONFIGURATION.md
key-decisions:
  - "already-landed is its own LandingKind; every landed-ness reader goes through LandingKind::puts_work_on_target"
  - "the scan records contained runs under a non-blocking land lock"
  - "land.lock is unlinked by the holder, on release, while held, only if the path still names the held inode"
---

# Phase quick-261103-pd7 Plan 01: sandbox-sensitive test, already-landed runs, land.lock churn Summary

A run whose head is already in its target branch now closes out as an `already-landed` landing (by `task land merge`, by the resolver, or by the scan), `land.lock` is removed on every release, and the whole suite passes inside a bwrap approximation of the one-off sandbox.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | 5ba8bef | record a run whose head is already in its target as already-landed |
| 2 | 8694ef9 | unlink land.lock on every release and record contained runs in the scan |
| 3 | 1766a37 | make the suite pass inside the one-off sandbox and document it |

## What was built

**Task 1.** `LandingKind::AlreadyLanded` (`already-landed`), `puts_work_on_target`, `as_str`, and `Landing.target_tip`. `merge_locked` (src/oneoff/land.rs) checks, after the target is resolved and before the source, planted gate and dirty-checkout facts, whether the head is an ancestor of the target tip and calls `record_already_landed`; `merge()` then slims exactly as after any landing. The resolver (`Step::AlreadyLanded`, src/oneoff/resolve.rs) does the same when the target moved onto the head, and in place of the old no-op-merge `fail`. `containing_commit` finds the first-parent commit that brought the head in (merge commit, or the head itself when it sits on that line, or the tip when git cannot say). Readers (`branch_state`, `run_counts`, `run_source`, `resume_refusal`, `slim_locked`, `leftover_slim_note`) use the helper; `last_merge_landing` stays Merged-only. `task list` shows `already-landed`, `task status` prints `landed:    already-landed ...` plus a `contained by <sha12>` line (no merge commit/diff lines), the TUI list shows `already`. The post-fetch error is reworded to "the run already landed in <branch>; nothing to merge", so a second merge of a landed run still exits 1.

**Task 2.** `LandLock` guard (land.rs) wraps `RunLock`; its `Drop` unlinks the file only while held and only if `RunLock::is_at(path)`. `take_land_lock` retries when it locked an inode that was unlinked meanwhile; `try_land_lock` takes one non-blocking attempt. The explicit unlinks in `slim_run` and `recover_dead` are gone. The scan (`tend_run` -> `tend_contained`, recycle.rs) records contained runs via `already_landed_candidate` (pure) then `record_already_landed`, then `slim_locked`; it skips a run whose land lock is held. `RecycleReport.already_landed` is in `summary` and `absorb`. TUI `landing_label`/`landing_lines` and a hostile-detail case in the render-escape guard cover the new kind. CONFIGURATION.md documents landing and the scan bullet.

**Task 3.** `tests/driverctl_store.rs` asserts that no frontmatter line starts with `sandbox:` or `sandbox_missing:` instead of `!text.contains("sandbox")`. TESTING.md has a "Running inside the one-off sandbox" subsection (what differs, the assert-keys rule, the bwrap command, no CI job) and the corrected `oneoff/` fixtures row.

## Item 4: land.lock root cause (D-05)

`driver::lock::acquire_at` opens the lock path with `create(true)` (src/driver/lock.rs:259-260 before this change) and a `RunLock` has no `Drop` (src/driver/lock.rs:126-136), so releasing closes the fd and leaves the file. Probes never create it (`probe_at` opens read-only, returns `Absent` on NotFound: src/driver/lock.rs:437-440, used by `landing_in_progress` land.rs:100-105 and `lock_is_free` recycle.rs:675-677). Every `take_land_lock` (land.rs:108-121) therefore created `land.lock`, and only two paths unlinked it, both only on a `Slimmed` outcome: `slim_run` (recycle.rs:339-342) and `recover_dead` (recycle.rs:868). The churn source is `tidy_finished` (recycle.rs:750), which runs on every scan for every finished run that is not slimmed, including an already-slimmed landed run (`leftover_slim_note` returns None for it, recycle.rs:586-588); it took the lock and released it without unlinking. The same leak existed in `merge_locked` (land.rs:748, every exit including refusals), `slim_run` on Kept/Skipped (recycle.rs:331-343), `recover_dead` early returns (recycle.rs:805-813), `age_out` (recycle.rs:988-1002) and the resume start (resume.rs:261). Measured in the user's data dir at planning time: both run dirs of project hash `f4c69b74` held a `land.lock` with a holder record `pgid 3124208, started_at 2026-11-04T00:12:42Z`, from one scan; run `...000607` had been slimmed at 00:08:00 and its lock unlinked then, so the 00:12:42 lock can only have come from `tidy_finished`.

Fix: every taker now returns a `LandLock` (land.rs, the guard and `take_land_lock`/`try_land_lock`); its `Drop` unlinks while the lock is still held, guarded by `RunLock::is_at` (src/driver/lock.rs). `run.lock` is not taken through this helper and is untouched. Test `no_land_lock_is_left_after_a_landing_a_refusal_or_a_scan` and `the_land_lock_file_is_unlinked_when_released` pin it.

## Item 3: stale preservation message (D-04)

No code change. `clone_preserved` is evaluated live on every scan (`tidy_finished`, `slim_locked`); nothing stores the verdict. The text was accurate when printed: `land_failure` was recorded 18:12:08, master fast-forwarded to `4f38f18` at 18:12:38 (reflog), the scan ran 18:12:42 and `land/1/work` was removed (mtime of `land/1`) at 18:12:44, i.e. by the first scan after the fast-forward. The regression test `a_resolver_clone_goes_once_its_commit_is_on_a_project_branch` (recycle.rs) was written first and passed at HEAD: the clone is kept with a warning while its commit is unreachable, and removed with no warning once the commit is on a project branch.

## Item 1: sandbox-sensitive tests found and fixed

Found by an evidence-based sweep (host run vs bwrap-approximation run, plus a grep sweep of tests and `#[cfg(test)]` modules for hostname/`sandbox` substring, ambient git, `/tmp` and network patterns):

1. `tests/driverctl_store.rs` `a_launch_without_a_posture_writes_neither_key_and_clears_a_stale_one`: the substring assertion on DIRECTION.md (whose host line is `gsdmm-sandbox` inside the sandbox). Fixed to check frontmatter key lines.
2. Hardening, same class (not failing today): `tests/driverctl_store.rs:1692` and `tests/launcher_degraded.rs:513` asserted `!contains("sandbox_missing")`; they now match the key line `"\nsandbox_missing:"`.

No other test differed. The bwrap run, whose approximation is stricter than the real sandbox about git identity, showed no further failures. The conformance test did not trip its missing-oracle path because `~/.claude` is bound read-only (so no by-design oracle line to report). The D-01 optional pin (a host name containing `sandbox`) was not added: `host_name()` (src/driverctl/store.rs:473) has no injection seam and adding one only for this was not worth it.

## Verification

| Run | Result lines | Passed | Failed | Ignored |
|-----|--------------|--------|--------|---------|
| host `cargo test --no-fail-fast` (target/pd7-host.log) | 59 | 3577 | 0 | 17 |
| bwrap sandbox approximation, `--offline` (target/pd7-sandbox.log) | 59 | 3577 | 0 | 17 |

(One host result line is interleaved with a "Running" line in the raw log, `driver_router_table`, 6 passed; counted from the `N passed; N failed; N ignored` fragments.) Baseline at o2r was 3561 passed, 17 ignored, 59 result lines; +16 passed are the new tests. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean.

## Deviations from Plan

**1. [Rule 3 - blocking, ordering] Lock guard and TUI label landed in the Task 1 commit.** `take_land_lock` changing its return type and `Landing.target_tip` forced the `LandLock` guard, `RunLock::is_at`, `try_land_lock` and the TUI `already` label to compile together with Task 1. Task 2's commit carries the scan, the explicit-unlink removals, the tests, `landing_lines` and the docs. `try_land_lock` therefore had a dead-code warning between the commits (clean at HEAD).

**2. Tracer test-first not strictly followed.** The two e2e tests were written, but implementation was done before the first red run; the pre-change behavior (exit 1, "nothing to merge (discard the run)") is the recorded defect.

**3. [inferred - audit] Scan excludes runs with `resumed_by`.** Not in the plan's exclusion list; a run another run continues is landed through its successor (klf C-09), so the scan leaves it alone.

**4. [inferred - audit] Two extra `sandbox_missing` assertions hardened** (Item 1 above), beyond the single test the plan names.

**5. [inferred - audit] `tend_contained` adds a warning line** `<id>: its head is already in <branch>; recorded as landed` so `task list` stderr names the run (the summary line has only counts).

Other inferred decisions carried from the plan: `containing_commit` method (first-parent list intersected with ancestry-path, oldest, `[inferred - audit]`); "already in" early check applies to the target the user chose, including `--into`.

## Backward compatibility

Binaries older than this one cannot parse `already-landed` in run.json (the store is local to one user; accepted per D-02).

## Known Stubs

None.

## Threat Flags

None. The scan now writes (records landings, deletes `gsd-oneoff/<id>`), as T-pd7-01 covers; it reads only the project's own objects and re-checks everything under the land lock.

## Self-Check: PASSED

- Commits 5ba8bef, 8694ef9, 1766a37 are ancestors of HEAD.
- Files in key-files exist; target/pd7-host.log and target/pd7-sandbox.log exist.
