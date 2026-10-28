---
created: 2026-10-28T00:00:00.000Z
title: Open DIRECTION.lock with O_NOFOLLOW + regular-file check, and cap read_run_file at 1 MiB (30-REVIEW WR-08, WR-09)
area: driver
severity: minor
source: .planning/phases/30-driver-tab-on-direction-md/30-REVIEW.md
completed: 2026-10-28
resolved_by: quick 261028-n0z 670e099 (WR-08 lock open) and 0f85617 (WR-09 1 MiB cap)
files:
  - src/driver/lock.rs
  - src/driver/direction.rs:588-593
---

## Problem

Two hardening gaps in run-directory access, both still present at 2026-10-28 [inferred — cleanup 2026-10-28]:

- **WR-08:** `acquire_at` opens `<run>/DIRECTION.lock` following symlinks, then truncates, writes and chmods
  it. A symlink planted there by an `--unsandboxed` run makes a TUI steer clobber the target file.
- **WR-09:** `read_run_file` reads DIRECTION.md with no size cap, unlike every other run-dir reader (1 MiB).

Both need a writer in the run directory, so they are advisory, but WR-08 is a destructive write and sits in
the T-30-04/T-30-24 threat model, so it is deferred rather than skipped.

## Solution

Fix text is in 30-REVIEW.md (WR-08: NOFOLLOW|NONBLOCK + `is_file()` before flock/set_len, also covers
`launcher.lock`; WR-09: `take(CAP+1)` and refuse over-cap). Add the symlinked-lock sentinel test and the
just-over-cap test.
