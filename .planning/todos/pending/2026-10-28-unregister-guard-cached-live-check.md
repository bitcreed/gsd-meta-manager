---
created: 2026-10-28T00:00:00.000Z
title: Unregister guard: read run locks at confirm time and share one live predicate (31-REVIEW IN-08, IN-10)
area: ui
severity: minor
source: git show f0eb8b8:.planning/phases/31-retire-superseded-envelope-goal-layer-code/31-REVIEW.md (IN-08, IN-10; no longer in the current 31-REVIEW.md), deferred by quick 261028-n12 [inferred — cleanup 2026-10-28]
files:
  - src/ui/screens/delete_confirm.rs (do_remove_project guard)
  - src/ui/screens/detail.rs (driver_live_for)
  - docs/CONFIGURATION.md
  - tests/async_blocking_guard.rs
---

## Problem

The unregister guard in `delete_confirm.rs` reads only the cached launcher marker and
is gated on `ctx.experimental`. It therefore misses three cases: before the first
scan has run, a run started within the last slow tick, and the experimental flag
being off. It also re-implements the predicate that `detail::driver_live_for`
already holds (IN-08), and the limits of the guard are not stated to the user (IN-10).

## Solution

At confirm time, re-read this alias's run locks with one `launcher_markers_for` call,
and check that this synchronous read is acceptable under `tests/async_blocking_guard.rs`
(it runs in a UI key handler). Drop the `ctx.experimental` clause from the guard,
since refusing to unregister a project with a live lock is safe with the flag off.
Either share a predicate with `driver_live_for` where the two agree, or document why
the guard's is stronger. Disclose any remaining window in `docs/CONFIGURATION.md`.
Doing IN-08 alone first would be undone by the IN-10 fix, so they are one change
[inferred — cleanup 2026-10-28].
