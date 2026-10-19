---
generated_by: driverctl 0.0.0-golden   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "golden"
status: stalled
status_reason: "driver ended its turn with a worker outstanding (dispatch I-01 has no report in LOG.md); the process exited cleanly, so this is not a crash"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-execute-phase 1"
items: {total: 2, pending: 1, running: 1, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "running", dispatches: 2, outcome: null}
  "I-02": {state: "pending", dispatches: 0, outcome: null}
caps:
  dispatches: {used: 2, limit: 10}
  wall_clock_min: {used: 14.2, limit: 60}
  gap_rounds: {used: 1, limit: 2, phase: "1"}
dispatch_audit: {logged: 2, observed: 2, denied: 0}
launches: 1
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap", pid: 41234, scope_unit: "gsdmm-golden-1.scope", host: "goldenhost", started_at: "2026-01-01T00:00:00Z", last_heartbeat: "2026-01-01T00:10:00Z"}
  - {role: "driver", state: "disconnected", session_id: "7c0e1d2a-0000-4000-8000-000000000001", model: "opus", mode: "headless", pid: 41240, scope_unit: "gsdmm-golden-1.scope", host: "goldenhost", started_at: "2026-01-01T00:00:00Z", last_heartbeat: "2026-01-01T00:10:00Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-01-01T00:20:00Z", item: "RUN", event: "launch-end", summary: "rc=0 status=stalled"}
updated_at: "2026-01-01T00:20:00Z"
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
