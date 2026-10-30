generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l2"
status: stopped
status_reason: "launcher-cap:wall_clock_min"
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
  wall_clock_min: {used: 1, limit: 1}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 0, observed: 0, denied: 0}
launches: 1
sandbox: "bwrap-scope"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2649240, scope_unit: "gsdmm-driver-lacc-l2-04abdf43-1.scope", host: "mediabox", started_at: "2026-10-19T22:08:40Z", last_heartbeat: "2026-10-19T22:08:40Z"}
  - {role: "driver", state: "disconnected", session_id: "72b76d64-55f9-4a9e-aeea-c9f52e3906d0", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l2-04abdf43-1.scope", host: "mediabox", started_at: "2026-10-19T22:08:40Z", last_heartbeat: "2026-10-19T22:08:47Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:09:40Z", item: "RUN", event: "launch-end", summary: "rc=143 status=stopped"}
updated_at: "2026-10-19T22:09:40Z"
