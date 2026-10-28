---
created: 2026-10-28T00:00:00.000Z
title: probe_at and read_holder_at open the lock path by name, following symlinks and blocking on a FIFO
area: driver
severity: minor
source: quick 261028-n0z planning [inferred — cleanup 2026-10-28]
files:
  - src/driver/lock.rs (probe_at, read_holder_at)
---

## Problem

Both functions `File::open` the lock path by name, which follows a symlink and
blocks on a FIFO with no writer. The callers that are reached:

- src/state_reader/driver_frontmatter.rs:923 (`read_driver_run_text`, inside the TUI's spawn_blocking worker on every refresh)
- src/launcher/stop.rs:102/134/149/161
- src/launcher/run.rs:1531-1532
- src/launcher/push.rs:939

A FIFO planted at launcher.lock by a writer in the run directory therefore
parks a blocking worker. This is the WR-02 class, read-side, with no destructive
write. (quick 261028-n0z closed the write-side in `acquire_at`.)

## Solution

Open read-only with `O_NOFOLLOW | O_NONBLOCK`, as `open_run_file` does. A
non-regular file makes `probe_at` return `LockProbe::Unknown` and
`read_holder_at` return `None`, which keeps the "Unknown is never Free" posture.
Add a linux-gated FIFO-no-block test for each.
