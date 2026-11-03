---
created: 2026-11-03T00:00:00.000Z
title: One-off run history beyond the latest run
area: ui
severity: minor
resolved_by: quick 261103-hx7
source: .planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md
files:
  - src/oneoff/store.rs
  - src/ui/screens/oneoff.rs
  - src/task_cli.rs
---

## Problem

`task status` and the `T` popup show only the project's latest one-off run
(quick 261103-f5y). Older runs stay on disk until discarded, but there is no way
to list, inspect, land or discard them except `task land ... <run-id>` with an
id read from the directory listing.

## Solution

Add `task list <project>` and a run list in the `T` popup (newest first,
bounded like the launcher's 20-run list), with the landing keys acting on the
selected run. `store::latest_run_id` already orders run ids; generalise it to a
bounded listing.
