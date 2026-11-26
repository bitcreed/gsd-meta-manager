---
status: in_progress
updated: 2026-01-10T12:00:00Z
---

# Phase 01 UAT

## Tests

### 1. Real pending row
expected: one
result: pending

### 2. Row without a result
expected: two

Some prose line that owns nothing.
  ### 3. Indented under prose
  result: pending

notes:
  ### 4. Indented under a key line
  result: pending

### 5. Row before the open fence
expected: five
result: pending

```
an example that never ends
