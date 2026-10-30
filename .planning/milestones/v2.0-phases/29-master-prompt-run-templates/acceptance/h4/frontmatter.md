generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-h4-131936"
status: parked
status_reason: "driver:all-parked"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-plan-phase 1 --gaps"
items: {total: 1, pending: 0, running: 0, done: 0, parked: 1, failed: 0}
item_states:
  "I-01": {state: "parked", dispatches: 1, outcome: "Dispatch of /gsd-plan-phase 1 --gaps denied by launcher: phase 1 gap-round cap 0 reached; no worker ran"}
caps:
  dispatches: {used: 0, limit: 4}
  wall_clock_min: {used: 0.5, limit: 15}
  gap_rounds: {used: 1, limit: 0, phase: "1"}
dispatch_audit: {logged: 1, observed: 0, denied: 1}
launches: 1
sandbox: "bwrap-scope"
prompt_version: "custom:f2f99387"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 14677, scope_unit: "gsdmm-driver-20261020-acc-h4-131936-9c70b381-1.scope", host: "mediabox", started_at: "2026-10-20T13:19:37Z", last_heartbeat: "2026-10-20T13:19:37Z"}
  - {role: "driver", state: "disconnected", session_id: "11ed9d2c-7c16-4f9b-85db-837a168dc407", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h4-131936-9c70b381-1.scope", host: "mediabox", started_at: "2026-10-20T13:19:37Z", last_heartbeat: "2026-10-20T13:20:05Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T13:20:05Z", item: "RUN", event: "launch-end", summary: "rc=0 status=parked"}
updated_at: "2026-10-20T13:20:05Z"
