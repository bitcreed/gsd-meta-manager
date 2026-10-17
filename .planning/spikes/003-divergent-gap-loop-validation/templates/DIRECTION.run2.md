# Driver direction: {run-id}

## Direction

- control: run
- goal: Ship phase 1 of slugkit through the normal GSD chain, unattended.
- worker-model: sonnet
- run-budget: dispatches=10, hours=2
- isolation: none

### Standing rules

- Never push, rebase, force, or delete branches.
- Do not start a new milestone, change ROADMAP scope, or delete phases; park instead.
- discuss-phase is skipped: CONTEXT.md exists for phase 1.

## Items

### I-01 Phase 1 slugify

- intent: Phase 1 is planned, executed and verified through GSD.
- done-when: `.planning/phases/01-slugify/01-VERIFICATION.md` frontmatter has `status: passed`
- budget: dispatches=8, minutes=80
- start-with: /gsd-plan-phase 1
- after: none
- hold: false
- answer:
