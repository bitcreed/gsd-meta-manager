---
phase: quick-261125-jx7
plan: 01
subsystem: registry
tags: [auto-registration, session-discovery, temp-dir-guard, gsd-marker]
requires: []
provides:
  - "registry::is_gsd_project_root, GSD_PLANNING_MARKERS, system_temp_root, auto_register_from_sessions_with_temp_root"
affects: [src/registry.rs, src/launch_target.rs, README.md, docs/GETTING-STARTED.md]
key-files:
  modified:
    - src/registry.rs
    - src/launch_target.rs
    - tests/registry_worktree_guard.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "[inferred] Marker helper is new, in src/registry.rs; no existing marker check existed to reuse"
  - "[inferred] Temp-dir guard applies to session auto-registration only; launch_target::auto_register gets the marker rule but not the temp guard"
  - "[inferred] A cwd with no .planning/ stays a silent skip; only the two new refusals log, at debug"
  - "[inferred] A degenerate temp dir (empty, relative, filesystem root) disables the temp guard"
  - "[inferred] The new test module sits above mod tests, because tests/spawn_seam_guard.rs forbids items after the test-module marker"
status: complete
commits: 4
plan_head_before: 379161180f8f7a820e08a9e6d2bbe589f46a106f
plan_head_after: a363aeab7a26070f1fa4306e4248de32e35557bf
actuals:
  tasks: 3
  commits: 4
---

# Phase quick-261125-jx7 Plan 01: Don't auto-register non-GSD or temp-dir folders Summary

Session auto-registration now registers only roots whose `.planning/` holds a GSD marker file (`STATE.md`, `ROADMAP.md`, `PROJECT.md` or `config.json`) and that lie outside the canonicalized system temp dir; `gsd-meta-manager <dir>` requires the same marker.

## What changed

- `src/registry.rs`: `GSD_PLANNING_MARKERS`, `is_gsd_project_root` (marker must be a regular file), `system_temp_root` / `temp_guard_root` / `is_under_temp_root` (canonical, component-wise, degenerate value disables the guard), and `auto_register_from_sessions_with_temp_root`. The public `auto_register_from_sessions` keeps its signature and passes the real temp root. The guard judges the RESOLVED root, so a worktree session in the temp dir whose main lies elsewhere still registers its main. The two new refusals log at debug; a cwd without `.planning/` stays silent.
- `src/launch_target.rs`: `auto_register` uses `is_gsd_project_root`; no temp guard (the user named the folder).
- Tests: 9 new in `gsd_root_guard_tests` plus 1 in `launch_target`; the 18 in-source and 7 integration auto-register call sites moved to the injectable entry, fixtures given markers, no coverage deleted.
- Docs: README (Quick start step 4, Examples paragraph) and GETTING-STARTED state both rules.

## Deviations from Plan

**1. [Rule 1 - Bug] New test module placed after `mod tests`**
- **Found during:** Task 3 full-suite run (`no_production_item_follows_a_test_module_marker` in `tests/spawn_seam_guard.rs` failed).
- **Fix:** moved `gsd_root_guard_tests` above `mod tests`. Commit a363aeab.

Otherwise the plan ran as written.

## Verification

Raw log `target/jx7-final.log` (`--no-fail-fast`): 69 suites ok, 0 FAILED, 4701 passed, 0 failed. `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

## Commits

- 95439348 fix(quick-261125-jx7): auto-register only GSD project roots outside the system temp dir
- 3b38a387 fix(quick-261125-jx7): launch-target registration requires a GSD marker in .planning/
- 6f4d046e docs(quick-261125-jx7): auto-registration needs a GSD marker and skips the system temp dir
- a363aeab fix(quick-261125-jx7): keep the new registry test module above mod tests for the spawn-seam guard

## Notes for the user

Install the new build so a running TUI stops picking up test shims; until then, running the suite can re-register the fake `claude` shim's temp dir in a live older TUI. The stray `.tmp8d6c3g` entry is already removed.

## Known Stubs

None.

## Self-Check: PASSED

All four commits are ancestors of HEAD; `git rev-list --count` from the recorded base gives 4.
