---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-07-30T03:39:40.539Z
---

# Broken Windows Ledger

> Cross-phase defect register. `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 18 | deviation | src/ui/screens/driver.rs |  | 18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record | open |  | 2026-07-30T03:39:40.539Z |  |

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
  }
]
````
