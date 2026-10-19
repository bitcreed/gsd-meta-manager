---
generated_by: driverctl 0.0.0-golden   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "golden"
status: starting
status_reason: null
control: "run"
current_item: null
current_phase: null
current_step: null
items: {total: 2, pending: 2, running: 0, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "pending", dispatches: 0, outcome: null}
  "I-02": {state: "pending", dispatches: 0, outcome: null}
caps:
  dispatches: {used: 0, limit: 10}
  wall_clock_min: {used: 0, limit: 60}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 0, observed: 0, denied: 0}
launches: 0
connected: []
heartbeat_stale_after_s: 300
last_event: {at: null, item: null, event: null, summary: null}
updated_at: "2026-01-01T00:00:00Z"
---
# Driver direction: golden

## Direction

- control: run
- goal: Ship phase 1 of the golden project, unattended.

## Items

### I-01 Phase 1 first slice

- intent: Phase 1 is planned, executed and verified.
- hold: false

### I-02 Phase 2 second slice

- intent: Phase 2 is planned, executed and verified.
- after: I-01
- hold: false
