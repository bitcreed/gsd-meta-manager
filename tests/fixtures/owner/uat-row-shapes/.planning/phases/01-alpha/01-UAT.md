---
status: partial
updated: 2026-01-10T12:00:00Z
---

# Phase 01 UAT

## Tests

### 1. Greeting shown
expected: the greeting is shown
result: pass
### 2. Sync status
expected: sync finishes
result: skipped
reason: Deferred follow-up for the next release
### 3. Background refresh
expected: the list refreshes
result: skipped
reason: the refresh service is not running
### 4. Camera capture
expected: a photo is stored
result: skipped
reason: needs a physical camera
### 5. Store listing
expected: the listing opens
result: skipped
reason: only visible in the release build
### 6. Unexplained skip
expected: it works
result: skipped
reason: no particular cause
### 7. Remote service
expected: the call succeeds
result: blocked
blocked_by: staging server is down
### 8. Handset only
expected: vibration happens
result: blocked
blocked_by: a physical device lab
### 9. Preview channel
expected: the update arrives
result: blocked
blocked_by: waiting for a preview build
### 10. Card payment
expected: the card is charged
result: blocked
blocked_by: sandbox account of a third-party payment provider
### 11. Waiting on legal
expected: the text is approved
result: blocked
blocked_by: sign-off from the owner
### 12. Assumed fine
expected: it looks fine
result: assumed_pass
### 13. Closed item
expected: it was handled
result: closed
### 14. Failing export
expected: the export contains all rows
result: failed
### 15. Half done
expected: both parts work
result: partial
### 16. Broken search
expected: search finds notes
result: issue
reported: the search is empty
### 17. Fenced example
expected: the example is documented
```
result: pending
reason: this belongs to the sample
```
result: pass
### 18. Block scalar reasons
expected: |
  first line
  second line
result: skipped
reason: |
  needs the simulator
  and a long wait
blocked_by: >
  the staging
  server
### 19. Folded reason
expected: ok
result: skipped
reason: >
  a folded
  physical reason
### 20. Überprüfung 確認 🔒
expected: multi-byte text is intact
result: pending
