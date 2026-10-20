generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-o1-133927"
status: parked
status_reason: "driver:all-parked"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-execute-phase 1 --gaps-only"
items: {total: 1, pending: 0, running: 0, done: 0, parked: 1, failed: 0}
item_states:
  "I-01": {state: "parked", dispatches: 4, outcome: "2nd gaps_found after round 1; driverctl caps.gap_rounds used 2/2; feed keeps growing new rule classes"}
caps:
  dispatches: {used: 4, limit: 10}
  wall_clock_min: {used: 16.5, limit: 90}
  gap_rounds: {used: 2, limit: 2, phase: "1"}
dispatch_audit: {logged: 4, observed: 4, denied: 0}
launches: 1
sandbox: "bwrap-scope"
prompt_version: "driver-prompt/1.0.0"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 101320, scope_unit: "gsdmm-driver-20261020-acc-o1-133927-19d6bc8c-1.scope", host: "mediabox", started_at: "2026-10-20T13:39:29Z", last_heartbeat: "2026-10-20T13:39:29Z"}
  - {role: "driver", state: "disconnected", session_id: "6c8f1032-1f79-4fe5-a171-ab6750035032", model: "opus", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-o1-133927-19d6bc8c-1.scope", host: "mediabox", started_at: "2026-10-20T13:39:29Z", last_heartbeat: "2026-10-20T13:55:56Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T13:55:57Z", item: "RUN", event: "launch-end", summary: "rc=0 status=parked"}
updated_at: "2026-10-20T13:55:57Z"
