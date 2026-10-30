generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-h3-043834"
status: stopped
status_reason: "stop:user"
control: "run"
current_item: "I-01"
current_phase: "1"
current_step: "/gsd-plan-phase 1 --gaps"
items: {total: 1, pending: 0, running: 1, done: 0, parked: 0, failed: 0}
item_states:
  "I-01": {state: "running", dispatches: 2, outcome: "gap-rounds budget raised to 1 by human; closure round 1 approved"}
caps:
  dispatches: {used: 0, limit: 4}
  wall_clock_min: {used: 0.7, limit: 15}
  gap_rounds: {used: 1, limit: 2, phase: "1"}
dispatch_audit: {logged: 2, observed: 0, denied: 0}
launches: 3
sandbox: "bwrap-scope"
prompt_version: "driver-prompt/1.0.0"
connected:
  - {role: "driver", state: "lost", session_id: null, model: null, mode: "headless", pid: null, scope_unit: null, host: "mediabox", started_at: "2026-10-20T04:38:34Z", last_heartbeat: null}
  - {role: "driver", state: "disconnected", session_id: "de2a7343-b5bb-47cc-ac6b-dbfd2fd6b1be", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h3-043834-08325b56-2.scope", host: "mediabox", started_at: "2026-10-20T04:38:35Z", last_heartbeat: "2026-10-20T04:39:02Z"}
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 697952, scope_unit: "gsdmm-driver-20261020-acc-h3-043834-08325b56-3.scope", host: "mediabox", started_at: "2026-10-20T04:39:03Z", last_heartbeat: "2026-10-20T04:39:03Z"}
  - {role: "driver", state: "disconnected", session_id: "37f87b22-0230-456d-a281-a60d3cc72fe7", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h3-043834-08325b56-3.scope", host: "mediabox", started_at: "2026-10-20T04:39:03Z", last_heartbeat: "2026-10-20T04:39:18Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T04:39:19Z", item: "RUN", event: "launch-end", summary: "status=stopped reason=stop:user"}
updated_at: "2026-10-20T04:39:19Z"
