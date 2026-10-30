generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l4"
status: stopped
status_reason: "launcher-cap:gap_rounds"
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
  wall_clock_min: {used: 3.3, limit: 10}
  gap_rounds: {used: 4, limit: 2, phase: "1"}
dispatch_audit: {logged: 0, observed: 0, denied: 0}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2662650, scope_unit: "gsdmm-driver-lacc-l4-9218ce1c-1.scope", host: "mediabox", started_at: "2026-10-19T22:12:13Z", last_heartbeat: "2026-10-19T22:12:13Z"}
  - {role: "driver", state: "disconnected", session_id: "57ee82a6-a6f9-4bda-b795-e3d644269ad0", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l4-9218ce1c-1.scope", host: "mediabox", started_at: "2026-10-19T22:12:13Z", last_heartbeat: "2026-10-19T22:14:51Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:15:33Z", item: "RUN", event: "launch-end", summary: "rc=143 status=stopped"}
updated_at: "2026-10-19T22:15:33Z"
