generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l3"
status: crashed
status_reason: "driver exited rc=0 without RUN end"
control: "run"
current_item: null
current_phase: null
current_step: null
items: {total: 2, pending: 2, running: 0, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "pending", dispatches: 0, outcome: null}
  "I-02": {state: "pending", dispatches: 0, outcome: null}
caps:
  dispatches: {used: 6, limit: 10}
  wall_clock_min: {used: 2.6, limit: 10}
  gap_rounds: {used: 3, limit: 2, phase: "1"}
dispatch_audit: {logged: 0, observed: 6, denied: 2}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 3510634, scope_unit: "gsdmm-driver-lacc-l3-44581b63-1.scope", host: "mediabox", started_at: "2026-10-20T01:16:02Z", last_heartbeat: "2026-10-20T01:16:02Z"}
  - {role: "driver", state: "disconnected", session_id: "6ab61d51-c1c3-424d-b9fe-d808e36b64bc", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l3-44581b63-1.scope", host: "mediabox", started_at: "2026-10-20T01:16:02Z", last_heartbeat: "2026-10-20T01:18:35Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T01:18:36Z", item: "RUN", event: "launch-end", summary: "status=crashed reason=driver exited rc=0 without RUN end"}
updated_at: "2026-10-20T01:18:36Z"
