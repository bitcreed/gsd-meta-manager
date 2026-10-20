generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-h1-132352"
status: parked
status_reason: "driver:all-parked"
control: "run"
current_item: "I-01"
current_phase: "0"
current_step: "Create notes/h1.txt with one line, run sleep 90 once, commit the file"
items: {total: 1, pending: 0, running: 0, done: 0, parked: 1, failed: 0}
item_states:
  "I-01": {state: "parked", dispatches: 2, outcome: "worker committed on worktree branch; current branch main lacks the commit; human must bring it onto main"}
caps:
  dispatches: {used: 2, limit: 6}
  wall_clock_min: {used: 2.3, limit: 15}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 2, observed: 2, denied: 0}
launches: 2
sandbox: "bwrap-scope"
prompt_version: "driver-prompt/1.0.0"
connected:
  - {role: "driver", state: "lost", session_id: "8f19363c-2808-4dd4-850c-99161088702b", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h1-132352-49285860-1.scope", host: "mediabox", started_at: "2026-10-20T13:23:52Z", last_heartbeat: "2026-10-20T13:24:05Z"}
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 27923, scope_unit: "gsdmm-driver-20261020-acc-h1-132352-49285860-2.scope", host: "mediabox", started_at: "2026-10-20T13:24:06Z", last_heartbeat: "2026-10-20T13:24:06Z"}
  - {role: "driver", state: "disconnected", session_id: "d13cf374-db93-4ba1-92ba-6763de88e0cf", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h1-132352-49285860-2.scope", host: "mediabox", started_at: "2026-10-20T13:24:06Z", last_heartbeat: "2026-10-20T13:26:24Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T13:26:25Z", item: "RUN", event: "launch-end", summary: "rc=0 status=parked"}
updated_at: "2026-10-20T13:26:25Z"
