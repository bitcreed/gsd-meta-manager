generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l1"
status: stopped
status_reason: "driver:stop"
control: "run"
current_item: null
current_phase: null
current_step: null
items: {total: 2, pending: 2, running: 0, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "pending", dispatches: 0, outcome: null}
  "I-02": {state: "pending", dispatches: 0, outcome: null}
caps:
  dispatches: {used: 3, limit: 3}
  wall_clock_min: {used: 0.5, limit: 10}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 0, observed: 3, denied: 2}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2646842, scope_unit: "gsdmm-driver-lacc-l1-133aea09-1.scope", host: "mediabox", started_at: "2026-10-19T22:08:11Z", last_heartbeat: "2026-10-19T22:08:11Z"}
  - {role: "driver", state: "disconnected", session_id: "f8bd9c29-c62b-4cb7-82af-4dbb4519bff6", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l1-133aea09-1.scope", host: "mediabox", started_at: "2026-10-19T22:08:11Z", last_heartbeat: "2026-10-19T22:08:39Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:08:39Z", item: "RUN", event: "launch-end", summary: "rc=0 status=stopped"}
updated_at: "2026-10-19T22:08:39Z"
