---
created: 2026-11-03T00:00:00.000Z
title: A second agent runtime behind the one-off seam
area: driver
severity: minor
source: .planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md
files:
  - src/oneoff/runtime.rs
  - tests/oneoff_runtime_seam_guard.rs
---

## Problem

`oneoff::runtime::AgentRuntime` has one variant, Claude (quick 261103-f5y,
C-10). Every agent-specific fact (argv, env, config seeding, credential, egress
upstream, stream parsing, models) lives there and the seam guard keeps the rest
neutral, but no second runtime (Codex, ...) exists, so the seam's shape is
untested against a real second implementation.

## Solution

Add a second variant behind the same methods: its argv (with or without a
budget cap, `caps().budget_cap`), its config dir and seeding, its credential and
egress upstream, and a stream parser into `AgentOutcome`. Choose it per run (CLI
flag first, then the confirm popup), never through a config surface unless one
is asked for, and extend the seam guard's positive control.
