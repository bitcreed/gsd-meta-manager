---
created: 2026-10-28T00:00:00.000Z
title: probe_at and read_holder_at open the lock path by name, following symlinks and blocking on a FIFO
area: driver
severity: minor
source: quick 261028-n0z planning [inferred — cleanup 2026-10-28]
completed: 2026-10-28
resolved_by: quick 261028-t3d 4977318 (probe_at, read_holder_at; run-start opens in 65233b8)
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

## Resolution

Both functions now share `acquire_at`'s guarded open (`open_lock_file`: `O_NOFOLLOW | O_NONBLOCK`, a
single-link regular file checked on the open handle), read-only. A refused type reads as
`LockProbe::Unknown(EINVAL)` or `None` without blocking, and the holder read is bounded at 4096 bytes.
The same item closed 27 WR-03/WR-04, 30 WR-11 and 31 WR-01.

Tests: `probe_at_reads_a_symlink_to_a_held_lock_as_unknown_not_held`,
`probe_at_reads_a_hard_linked_lock_file_as_unknown`, `probe_at_reads_a_fifo_as_unknown_without_blocking`,
`read_holder_at_refuses_a_symlink_or_hard_link_to_a_valid_record`,
`read_holder_at_reads_a_fifo_as_none_without_blocking`,
`read_holder_at_refuses_a_record_over_the_cap_and_reads_one_at_it` (src/driver/lock.rs),
`read_driver_run_text_with_a_fifo_launcher_lock_derives_liveness_unknown_without_blocking`
(src/state_reader/driver_frontmatter.rs) and `an_orphan_stop_does_not_block_on_a_fifo_state_json`
(src/launcher/stop.rs).
