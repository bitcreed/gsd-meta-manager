---
created: 2026-11-03T00:00:00.000Z
title: Live view of a one-off run's stream.jsonl
area: ui
severity: minor
source: .planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md
files:
  - src/oneoff/launch.rs
  - src/ui/screens/oneoff.rs
---

## Problem

A one-off run persists the agent's raw stream-json to
`<run-dir>/stream.jsonl` (quick 261103-f5y, C-11), but nothing reads it while the
run is live. The `T` popup shows only "running Nm"; a user cannot see what the
agent is doing without opening the file by hand. CONTEXT.md put the live/attach
view explicitly out of scope for the core slice.

## Solution

Reuse the Driver tab's bounded live-output ring (`DRIVER_OUTPUT_RING_LINES`,
`render_for_terminal` per line) for a one-off: tail `stream.jsonl` from a
cursor on `spawn_blocking`, summarize assistant/tool events into one line each,
and show it under the `T` popup's summary while the run is live. Never inline
raw agent text without the sanitiser.
