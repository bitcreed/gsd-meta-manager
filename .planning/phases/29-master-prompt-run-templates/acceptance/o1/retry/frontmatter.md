generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-o1-135833"
status: parked
status_reason: "driver:all-parked"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-plan-phase 1 --gaps"
items: {total: 1, pending: 0, running: 0, done: 0, parked: 1, failed: 0}
item_states:
  "I-01": {state: "parked", dispatches: 5, outcome: "feed criterion contradicts SLUG-01/CONTEXT; 01-03 plan ready, awaiting human answer"}
caps:
  dispatches: {used: 5, limit: 10}
  wall_clock_min: {used: 26.8, limit: 90}
  gap_rounds: {used: 2, limit: 2, phase: "1"}
dispatch_audit: {logged: 5, observed: 5, denied: 0}
launches: 1
sandbox: "bwrap-scope"
prompt_version: "driver-prompt/1.0.0"
connected:
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 204203, scope_unit: "gsdmm-driver-20261020-acc-o1-135833-63cb3857-1.scope", host: "mediabox", started_at: "2026-10-20T13:58:34Z", last_heartbeat: "2026-10-20T13:58:34Z"}
  - {role: "driver", state: "disconnected", session_id: "42a6716d-5b62-423a-9f39-dbecc53151b4", model: "opus", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-o1-135833-63cb3857-1.scope", host: "mediabox", started_at: "2026-10-20T13:58:34Z", last_heartbeat: "2026-10-20T14:25:23Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T14:25:23Z", item: "RUN", event: "launch-end", summary: "rc=0 status=parked"}
updated_at: "2026-10-20T14:25:23Z"
