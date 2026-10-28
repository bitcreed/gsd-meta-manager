---
created: 2026-10-28T00:00:00.000Z
title: Retire or cfg(test) the production-dead executor remnants (31-REVIEW IN-04)
area: driver
severity: minor
source: .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-REVIEW.md (IN-04), deferred by quick 261028-n12 [inferred — cleanup 2026-10-28]
files:
  - src/executor/mod.rs
  - src/executor/stream_json.rs
  - src/executor/runtime.rs (AgentRuntime::resolve)
  - src/state_reader/git_ops.rs (head_sha)
  - src/registry.rs (check_prompt_input_drift)
  - src/driver/rate_limit.rs (the only stream_json user, in tests)
---

## Problem

After Phase 31 several items are reachable only from tests while the module doc in
`src/executor/mod.rs` still says every submodule "has a live user":

- `stream_json` has only a `#[cfg(test)]` user (the `rate_limit.rs` tests), and is a
  module of about 1000 lines.
- `AgentRuntime::resolve` has no caller.
- `head_sha` in `git_ops.rs` is used by tests only.
- `registry::check_prompt_input_drift` has no production caller (its one caller, the
  legacy executor gate, went in Phase 31) [inferred — cleanup 2026-10-28].

## Solution

Move `stream_json` behind `#[cfg(test)]` or next to `rate_limit`'s tests; delete
`resolve` and `head_sha`, or justify keeping them; retire `check_prompt_input_drift`
or justify it; and reword the "has a live user" claim in `executor/mod.rs`. Then run
the spawn-seam, async-blocking and other guard tests.
