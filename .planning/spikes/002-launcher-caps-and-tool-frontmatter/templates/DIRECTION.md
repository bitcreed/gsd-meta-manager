---
generated_by: driverctl 0.2-spike   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "{run-id}"
status: running
status_reason: null
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-execute-phase 1"
items: {total: 2, pending: 1, running: 1, done: 0, parked: 0, failed: 0}
caps:
  dispatches: {used: 3, limit: 10}
  wall_clock_min: {used: 14.2, limit: 60}
  gap_rounds: {used: 1, limit: 2, phase: "1"}
dispatch_audit: {logged: 3, observed: 3, denied: 0}
launches: 1
connected:
  - {role: "launcher", state: "connected", session_id: null, model: null, mode: "bwrap", pid: 41234, scope_unit: "gsdmm-{run-id}-1791930000.scope", host: "mediabox", started_at: "2026-10-13T22:00:00Z", last_heartbeat: "2026-10-13T22:14:10Z"}
  - {role: "driver", state: "connected", session_id: "7c0e...", model: "opus", mode: "headless", pid: 41240, scope_unit: "gsdmm-{run-id}-1791930000.scope", host: "mediabox", started_at: "2026-10-13T22:00:00Z", last_heartbeat: "2026-10-13T22:14:12Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-13T22:13:58Z", item: "I-01", event: "dispatched", summary: "step=/gsd-execute-phase 1 base=3b5167b round=0 dispatch=3/6 phase=1"}
updated_at: "2026-10-13T22:14:12Z"
---
# Driver direction: {run-id}

<!--
HUMAN-OWNED BODY. The block between the two `---` lines above is TOOL-WRITTEN by driverctl (the
launcher-side single writer): it is replaced on every run event, and anything typed into it is
overwritten. Everything below the closing `---` is preserved byte-for-byte; the tool never edits it.
The driver reads this file before every dispatch and never writes it (it is read-only in the sandbox).
gsd-mm's Queue/Driver tab may edit the body on the human's behalf (it is the same owner).
Location: <main-checkout>/.planning/meta-manager/drivers/{run-id}/DIRECTION.md (gitignored).
The driver always uses this absolute main-checkout path, even when it works in a git worktree.
Line format matters: `- key: value` lines under each heading are parsed; prose elsewhere is ignored.
-->

## Direction

- control: run
  <!-- run | pause | stop. pause = finish the current dispatch, then idle-exit. stop = exit now after logging. -->
- goal: {one or two sentences: what this run is for}
- worker-model: sonnet
  <!-- model passed to worker subagents; the driver itself is launched with Opus -->
- run-budget: dispatches=20, hours=4
- isolation: worktree
  <!-- worktree | none. Passed through to workers for nested GSD dispatches that require it. -->

### Standing rules

- Never push, rebase, force, or delete branches; gsd-mm pushes from outside the sandbox.
- Do not start a new milestone, change ROADMAP scope, or delete phases; park instead.
- Max 2 gap-closure rounds per phase; a third is a park, never a dispatch.
- {project-specific rules, e.g. "run cargo test --no-fail-fast, not cargo test"}

## Items

<!--
One `### {id} {title}` block per item, in priority order. The driver takes the first item that is
not done/parked, not on hold, and whose `after` items are done.
Status, outcome and needs-human live in LOG.md (driver-owned) - not here. To answer a parked item,
fill in `answer:` and set `hold: false`; the driver re-reads it before the next dispatch.
-->

### I-01 {short title}

- intent: {what should be true afterwards and why, in plain words}
- done-when: {a check the driver can run itself, e.g. `cargo test --no-fail-fast` passes AND `.planning/phases/21-*/21-VERIFICATION.md` has status: passed}
- budget: dispatches=4, gap-rounds=2, minutes=90
- start-with: {optional GSD command hint, e.g. /gsd-plan-phase 26; the driver may choose otherwise}
- after: none
- hold: false
- answer:

### I-02 {short title}

- intent: {...}
- done-when: {...}
- budget: dispatches=1, minutes=20
- after: I-01
- hold: false
- answer:
