generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "lacc-l5"
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
  dispatches: {used: 0, limit: 10}
  wall_clock_min: {used: 0.1, limit: 10}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 0, observed: 0, denied: 0}
launches: 2
sandbox: "bwrap-scope"
connected:
  - {role: "driver", state: "lost", session_id: "8d4bc25e-9d8f-4d69-9f1d-3c48946aeefd", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l5-30e60c68-1.scope", host: "mediabox", started_at: "2026-10-19T22:07:47Z", last_heartbeat: "2026-10-19T22:07:52Z"}
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 2645109, scope_unit: "gsdmm-driver-lacc-l5-30e60c68-2.scope", host: "mediabox", started_at: "2026-10-19T22:07:53Z", last_heartbeat: "2026-10-19T22:07:53Z"}
  - {role: "driver", state: "disconnected", session_id: "ca5dc671-5305-47c8-acc2-7b5345952b89", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-lacc-l5-30e60c68-2.scope", host: "mediabox", started_at: "2026-10-19T22:07:53Z", last_heartbeat: "2026-10-19T22:08:00Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-19T22:08:01Z", item: "RUN", event: "launch-end", summary: "rc=0 status=stopped"}
updated_at: "2026-10-19T22:08:01Z"
