generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-d33"
status: stopped
status_reason: "launcher-cap:gap_rounds"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-plan-phase 1 --gaps"
items: {total: 1, pending: 0, running: 1, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "running", dispatches: 3, outcome: "executed; gaps_found round 1 (+ to plus)"}
caps:
  dispatches: {used: 2, limit: 10}
  wall_clock_min: {used: 12.1, limit: 90}
  gap_rounds: {used: 2, limit: 0, phase: "1"}
dispatch_audit: {logged: 3, observed: 2, denied: 1}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2996765, scope_unit: "gsdmm-driver-lacc-d33-16259ac2-1.scope", host: "mediabox", started_at: "2026-10-20T00:05:39Z", last_heartbeat: "2026-10-20T00:05:39Z"}
  - {role: "driver", state: "disconnected", session_id: "45f6bb38-d40e-487e-84cd-5848aa29249c", model: "opus", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-d33-16259ac2-1.scope", host: "mediabox", started_at: "2026-10-20T00:05:39Z", last_heartbeat: "2026-10-20T00:17:42Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T00:17:44Z", item: "RUN", event: "launch-end", summary: "rc=143 status=stopped"}
updated_at: "2026-10-20T00:17:44Z"
