generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l7"
status: stopped
status_reason: "launcher-cap:gap_rounds"
control: "run"
current_item: null
current_phase: null
current_step: null
items: {total: 1, pending: 1, running: 0, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "pending", dispatches: 0, outcome: null}
caps:
  dispatches: {used: 1, limit: 5}
  wall_clock_min: {used: 3, limit: 10}
  gap_rounds: {used: 4, limit: 2, phase: "1"}
dispatch_audit: {logged: 0, observed: 1, denied: 1}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2673356, scope_unit: "gsdmm-driver-lacc-l7-23e06448-1.scope", host: "mediabox", started_at: "2026-10-19T22:15:35Z", last_heartbeat: "2026-10-19T22:15:35Z"}
  - {role: "driver", state: "disconnected", session_id: "d3423e39-52da-40a5-99a9-3744662877d9", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l7-23e06448-1.scope", host: "mediabox", started_at: "2026-10-19T22:15:35Z", last_heartbeat: "2026-10-19T22:18:31Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:18:34Z", item: "RUN", event: "launch-end", summary: "rc=143 status=stopped"}
updated_at: "2026-10-19T22:18:34Z"
