# Driver direction: {run-id}

<!--
HUMAN-OWNED. The driver reads this file before every dispatch and never writes it.
gsd-mm's Queue/Driver tab may edit it on the human's behalf (it is the same owner).
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
