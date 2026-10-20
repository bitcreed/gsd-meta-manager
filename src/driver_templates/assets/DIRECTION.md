# Driver direction: {run-id}

<!--
HUMAN-OWNED BODY. Everything in this file is yours: the driver reads it before every dispatch and
never writes it (the run dir is read-only inside the sandbox). The tool-written status block that
`driver new` put above this title (between the two `---` lines) is produced by driverctl and
replaced on every run event; anything typed into it is overwritten. This body is preserved
byte-for-byte.
Location: <main-checkout>/.planning/meta-manager/drivers/{run-id}/DIRECTION.md (gitignored).
Line format matters: `- key: value` lines under each heading are parsed; prose elsewhere is ignored.
Fill in every {...} placeholder (or delete the line), then run `gsd-meta-manager driver check`.
-->

## Direction

- control: run
  <!-- run | pause | stop. pause = finish the current dispatch, record its report, then end. stop = end after logging, dispatching nothing new. The value is exactly one word. -->
- goal: {one or two sentences: what this run is for}
- worker-model: sonnet
  <!-- model passed to worker subagents; the driver itself is launched with Opus -->
- run-budget: dispatches=20, hours=4
- isolation: worktree
  <!-- worktree | none. Passed through to workers for nested GSD dispatches that require it. -->

### Standing rules

- Never push, rebase, force, or delete branches; gsd-mm pushes from outside the sandbox.
- Do not start a new milestone, change ROADMAP scope, or delete phases; park instead.
- human-verification: defer
  <!-- defer | skip. defer (default) = a verification that needs a human (human_needed) parks the item as needs-human. skip = the driver may continue past it; write skip only when you accept that. -->
<!-- stop-rules:begin -->
Check these before every dispatch, highest first; the first rule that holds decides and names the park.
1. Human control: DIRECTION.md says `control: stop` or `control: pause` (or a dispatch is denied with `LAUNCHER CAP: control=...`): dispatch nothing new, record the in-flight worker's report, then `driverctl log end --reason pause|stop`; this ends the run and is not a park.
2. Launcher denial: any other dispatch denied with `LAUNCHER CAP: ...`: never retry the denied dispatch; park the item with `--rule launcher-cap`, and end the run if nothing else is eligible.
3. Run budget: the run's `run-budget` is exhausted: park the item in flight with `--rule run-budget` and end the run with `--reason run-budget`.
4. Divergence (defined below): park with `--rule divergence`. Divergence outranks gap rounds when both hold.
5. Gap rounds (defined below): at most the item's `gap-rounds` closure rounds per phase (2 when its budget has no `gap-rounds` key), counted by `driverctl show` -> `caps.gap_rounds`, which is authoritative over your own count; at the next `gaps_found` park with `--rule gap-rounds` and never dispatch another closure.
6. Item budget: its dispatches or its minutes (from its first dispatch) are exhausted: park with `--rule budget`.
7. No progress: two consecutive dispatches with no new commit and no change in done-when evidence: park with `--rule no-progress`.
8. Needs a human: a success criterion needs a scope change, a new milestone, real hardware, network, credentials, a human sign-off, or contradicts another criterion: park with `--rule needs-human` and the exact question. Never dispatch rounds at a gap that only a human can close.
On a `gap-rounds` or `divergence` park, also run `driverctl log needs-human` naming the budget edit and the answer that would resume the item.
A human `answer:` lifts only a `needs-human` park (rule 8). A park under rules 2-7 stays parked when only `answer:` is filled; it is lifted only by a budget edit (a higher item budget or `gap-rounds`, a higher `run-budget`, or a relaunch with higher launcher caps), and you re-check the rule that parked it before dispatching again. An empty `answer:` lifts nothing.
<!-- stop-rules:end -->
<!-- GSD can detect a stalled planner itself: set `planner.stall_detection_enabled` in the project's own .planning/config.json if you want that. It is optional, it is yours to enable, and gsd-mm never writes GSD project config. -->
- {project-specific rules, e.g. "run cargo test --no-fail-fast, not cargo test"}

## Items

<!--
One `### I-<n> <title>` block per item, in priority order. The driver takes the first item that is
not done/parked, not on hold, and whose `after` items are done (an `after` may name a later item).
Status, outcome and needs-human live in LOG.md (driver-owned), not here. To answer a parked item,
fill in `answer:` and set `hold: false`; the driver re-reads it before the next dispatch.
budget: comma-separated integer pairs from dispatches, minutes, gap-rounds (at least one).
-->

### I-01 {short title}

- intent: {what should be true afterwards and why, in plain words}
- done-when: {a check the driver can run itself, e.g. `cargo test --no-fail-fast` passes AND `.planning/phases/21-*/21-VERIFICATION.md` has status: passed}
- budget: dispatches=4, gap-rounds=2, minutes=90
- start-with: {optional GSD command hint, e.g. /gsd-plan-phase 26; the driver may choose otherwise}
- after: none
- hold: false
- answer:
