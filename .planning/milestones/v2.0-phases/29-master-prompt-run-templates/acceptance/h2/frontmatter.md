generated_by: driverctl 1.14.0   # TOOL-WRITTEN. Do not edit: regenerated on every run event.
driver_frontmatter_version: 1
run_id: "20261020-acc-h2-132852"
status: parked
status_reason: "driver:all-parked"
control: "run"
current_item: "I-02"
current_phase: "0"
current_step: "Create notes/h2b.txt holding one line, then commit the file"
items: {total: 2, pending: 0, running: 0, done: 1, parked: 1, failed: 0}
item_states:
  "I-01": {state: "done", dispatches: 1, outcome: "notes/h2.txt committed (5c393c3)"}
  "I-02": {state: "parked", dispatches: 1, outcome: "notes/h2b.txt committed only on worktree branch 22de049; awaiting merge decision"}
caps:
  dispatches: {used: 2, limit: 6}
  wall_clock_min: {used: 2.8, limit: 15}
  gap_rounds: {used: 0, limit: 2, phase: null}
dispatch_audit: {logged: 2, observed: 2, denied: 0}
launches: 2
sandbox: "bwrap-scope"
prompt_version: "driver-prompt/1.0.0"
connected:
  - {role: "driver", state: "disconnected", session_id: "b91700e5-fad5-4fc9-8422-82a01a304f32", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h2-132852-9bf41f1a-1.scope", host: "mediabox", started_at: "2026-10-20T13:28:52Z", last_heartbeat: "2026-10-20T13:31:02Z"}
  - {role: "launcher", state: "disconnected", session_id: null, model: null, mode: "bwrap-scope", pid: 68980, scope_unit: "gsdmm-driver-20261020-acc-h2-132852-9bf41f1a-2.scope", host: "mediabox", started_at: "2026-10-20T13:31:03Z", last_heartbeat: "2026-10-20T13:31:03Z"}
  - {role: "driver", state: "disconnected", session_id: "42c52203-bfca-4df5-9764-11002312c786", model: "haiku", mode: "headless", pid: null, scope_unit: "gsdmm-driver-20261020-acc-h2-132852-9bf41f1a-2.scope", host: "mediabox", started_at: "2026-10-20T13:31:03Z", last_heartbeat: "2026-10-20T13:31:38Z"}
heartbeat_stale_after_s: 300
last_event: {at: "2026-10-20T13:31:39Z", item: "RUN", event: "launch-end", summary: "rc=0 status=parked"}
updated_at: "2026-10-20T13:31:39Z"
