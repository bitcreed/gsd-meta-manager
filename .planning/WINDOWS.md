---
schema_version: 1
open_count: 2
waived_count: 0
fixed_count: 0
total_count: 2
last_updated: 2026-08-18T17:21:17.313Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 18 | deviation | src/ui/screens/driver.rs |  | 18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record | open |  | 2026-07-30T03:39:40.539Z |  |
| 2 | 19 | deviation | src/envelope/mod.rs |  | GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented | open |  | 2026-08-18T17:21:17.313Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "18",
    "file": "src/ui/screens/driver.rs",
    "line": null,
    "description": "18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-30T03:39:40.539Z",
    "resolved_at": null
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "19",
    "file": "src/envelope/mod.rs",
    "line": null,
    "description": "GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T17:21:17.313Z",
    "resolved_at": null
  }
]
````
