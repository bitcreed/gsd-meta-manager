---
quick_id: 260722-emn
plan: 5
item: E
subsystem: state_reader
status: complete
tags: [git, staleness, last-activity, mtime]
requires: []
provides: [project_last_activity]
affects: [src/state_reader/git_ops.rs]
tech-stack:
  added: []
  patterns: [synchronous std::process::Command for git, chrono UTC timestamps, shallow .planning walk]
key-files:
  created: []
  modified: [src/state_reader/git_ops.rs]
decisions:
  - "Synchronous (std::process::Command, not tokio) because consumer parse_project_state is sync"
  - "git committer date (%cI, strict ISO 8601) preferred over file mtimes which lie after clone/copy/touch"
  - "Shallow .planning walk (top level + one subdir level) is sufficient for staleness"
metrics:
  duration: 6min
  tasks: 2
  files: 1
  completed: 2026-07-22
---

# Quick 260722-emn Plan 5: git_ops staleness / last-activity Summary

Added a synchronous `project_last_activity` primitive to `src/state_reader/git_ops.rs` that
derives a project's last-activity timestamp from its most recent git commit time (committer
date via `git -C <root> log -1 --format=%cI`), with a filesystem-mtime fallback (newest file
under `.planning/`, then the project directory's own mtime) for non-git or empty repos.

## What Was Built

- `git_last_commit_time(project_root) -> Option<DateTime<Utc>>` — runs `git -C <root> log -1
  --format=%cI` via `std::process::Command`; returns `None` on spawn failure, non-zero exit,
  empty output, or RFC 3339 parse error. Never panics.
- `mtime_last_activity(project_root) -> Option<DateTime<Utc>>` — shallow walk of `.planning/`
  (top level plus one level of subdirectories) collecting file `modified()` times and taking
  the max; falls back to the `project_root` directory mtime when `.planning/` is absent;
  `None` when nothing is readable.
- `pub fn project_last_activity(project_root) -> Option<DateTime<Utc>>` — returns
  `git_last_commit_time(...).or_else(|| mtime_last_activity(...))`.
- Five inline tests (git repo → Some with graceful skip if the sandbox forbids commits;
  non-git dir → None; fresh `.planning/STATE.md` → Some within a minute of now; missing path
  → None; non-git dir → mtime fallback Some).

## Scope Adherence

Touched ONLY `src/state_reader/git_ops.rs` (including its inline tests). No edits to mod.rs,
app.rs, or any other file. Wiring `project_last_activity` into `ProjectState` is deferred to
plan 6; UI display to plan 9. The new function is `pub` but not yet called by any consumer —
this is intentional per the plan (the primitive-only deliverable) and produces no dead_code
warning because the function is public.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | git_last_commit_time via `git log -1 --format=%cI` | b8f7c4a | src/state_reader/git_ops.rs |
| 2 | mtime fallback + public project_last_activity | f18217f | src/state_reader/git_ops.rs |

## Verification

- `cargo build`: clean, zero warnings (verified with a forced recompile of the crate).
- `cargo test`: 130 passed (5 suites). `cargo test --lib git_ops`: 5 passed.
- Behavior (git-preferred, mtime-fallback, None-on-nothing) covered by inline tests.

## Deviations from Plan

None — plan executed exactly as written. TDD flow followed pragmatically: each task committed
atomically as a single `feat` including its inline tests, matching the plan's commit guidance
(`feat(260722-emn): ...`).

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/state_reader/git_ops.rs (modified)
- FOUND: commit b8f7c4a (Task 1)
- FOUND: commit f18217f (Task 2)
