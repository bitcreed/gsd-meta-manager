generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l8"
status: stopped
status_reason: "driver:stop"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-plan-phase 1"
items: {total: 1, pending: 0, running: 1, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "running", dispatches: 1, outcome: null}
caps:
  dispatches: {used: 1, limit: 3}
  wall_clock_min: {used: 1.1, limit: 10}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 1, observed: 1, denied: 0}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2683699, scope_unit: "gsdmm-driver-lacc-l8-ab01fa37-1.scope", host: "mediabox", started_at: "2026-10-19T22:18:36Z", last_heartbeat: "2026-10-19T22:18:36Z"}
  - {role: "driver", state: "disconnected", session_id: "8fc01d5b-a751-404a-9413-6f4a63f560e3", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l8-ab01fa37-1.scope", host: "mediabox", started_at: "2026-10-19T22:18:36Z", last_heartbeat: "2026-10-19T22:19:44Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:19:44Z", item: "RUN", event: "launch-end", summary: "rc=0 status=stopped"}
updated_at: "2026-10-19T22:19:44Z"
