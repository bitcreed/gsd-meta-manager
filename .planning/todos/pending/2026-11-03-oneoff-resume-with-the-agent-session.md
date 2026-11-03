---
created: 2026-11-03T00:00:00.000Z
title: Resume a one-off through the agent's own session instead of a primed fresh one
area: launcher
severity: minor
source: .planning/quick/261103-klf-cancel-pause-one-off-runs-and-resolvers-/261103-klf-CONTEXT.md
files:
  - src/oneoff/resume.rs
  - src/oneoff/launch.rs
  - src/oneoff/runtime.rs
---

## Problem

`task resume` (quick 261103-klf, C-09, C-10) continues a paused, stopped or killed
one-off as a NEW run with a FRESH agent session primed with the old task, the old
report's headline and follow-ups and the commit log. The agent loses its conversation:
what it had read, tried and decided before it was paused.

klf chose the fresh session on purpose. The session transcript lives in the per-run
config dir inside the old sandbox, whether it can be picked up again could not be
verified without a paid run, and the fresh path is fully testable on the fake harness.

## Solution

Behind the `AgentRuntime` seam, resume through the agent's own session id: keep the id
the stream's `system` init event carries (classified runtime-side, never in a neutral
file), and start the new session with the agent's resume flag in the SAME per-run config
dir, copied or re-bound into the new run, so the conversation carries on. It needs:

- a decision on where the old config dir goes (the new run has its own run dir, so the
  transcript has to be copied in or the config dir re-bound);
- a fallback to the primed fresh session when the transcript cannot be resumed;
- a paid-run check that the resumed agent really continues from where it was paused.

See also `2026-11-03-oneoff-raise-hard-ceiling-via-resume.md`, which needs the same
session-id plumbing to raise a hard ceiling by stopping and resuming in the SAME run dir;
the two should share it.
