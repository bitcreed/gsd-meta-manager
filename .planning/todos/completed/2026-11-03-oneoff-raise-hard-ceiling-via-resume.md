---
created: 2026-11-03T00:00:00.000Z
title: Raise a one-off's hard spend ceiling by stopping and resuming the session
area: launcher
resolved_by: promoted to backlog Phase 999.10 (.planning/ROADMAP.md)
severity: minor
source: .planning/quick/261103-gt1-live-viewer-with-message-injection-and-s/261103-gt1-CONTEXT.md
files:
  - src/oneoff/launch.rs
  - src/oneoff/runtime.rs
  - src/oneoff/session.rs
---

## Problem

A one-off's `--max-budget-usd` is the agent's own hard ceiling, fixed at launch
(quick 261103-gt1, C-14, C-19). The soft cap and the spend meter only make the ceiling
visible and let the operator ask the agent to wrap up earlier; nothing can raise the
ceiling itself. A run that is on track but nears the ceiling can only be left to end
as `over_budget`, or be started again from scratch in a new clone.

## Solution

Add a "raise the ceiling" action: stop the live session, then relaunch the agent with
`claude --resume <session-id>` and a higher `--max-budget-usd` in the SAME clone and
run dir, so the work and the conversation carry on. The session id comes from the
stream's `system` init event (classified as `StreamEvent::System`; the classifier would
need to keep the id, runtime-side). It needs a decision on the run record (one run,
several sessions: `run.json` would keep the ceiling history), on the report contract
(the agent has not reported yet) and on the wall clock (it should keep counting from
the first spawn). Out of scope for 261103-gt1 by decision.
