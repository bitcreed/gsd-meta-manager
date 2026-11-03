---
created: 2026-11-03T00:00:00.000Z
title: Allow more than one concurrent one-off per project
area: driver
severity: minor
source: .planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md
files:
  - src/oneoff/launch.rs
  - src/oneoff/store.rs
---

## Problem

Quick 261103-f5y allows one live one-off per project: the daemon holds a
per-project `flock` (`oneoff.lock`) and a second dispatch exits 3. Two
`/gsd-quick` runs in two clones would both append to `.planning/STATE.md`'s
quick-task table and both mint quick ids, so merging the second would conflict
and the ids could collide.

## Solution

Decide how two runs can coexist: a per-run lock instead of a per-project one,
plus either a post-merge STATE.md reconciliation or a host-assigned quick id
passed in the prompt. Until then the single lock stays. The status line and the
`T` popup would also need a run list (see the run-history todo).
