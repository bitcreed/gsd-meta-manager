---
status: partial
source:
  - 23-VERIFICATION.md
updated: 2026-01-10T12:00:00Z
---

# UAT

## Tests

### 1. Invoice mail
expected: the mail arrives
result: skipped
reason: no particular cause
### 2. Identity login
expected: login works
result: blocked
blocked_by: third-party identity provider is not reachable
### 3. Store listing
expected: the listing shows
result: blocked
blocked_by: waiting for a release build
### 4. Legal text
expected: the text is approved
result: blocked
blocked_by: sign-off from the owner
