---
status: testing
phase: 39-check-actions-through-gsd
source: [39-VERIFICATION.md]
started: 2026-11-17T21:52:09Z
updated: 2026-11-17T21:52:09Z
---

## Current Test

number: 1
name: Won't do on a phase row (inbox) and on one check (Owner block) on a throwaway project (fixture copy plus git repo) in the real multiplexer (herdr, else tmux) with the watcher running
expected: |
  The launched Claude session resumes the UAT session without a resume/restart question, shows no xargs warning, records result: skipped with a reason beginning `Deferred follow-up: will not do.`, offers no 999.x promotion; after the watcher refresh the row/check leaves the TEST lane
awaiting: user response

## Tests

### 1. Won't do on a phase row (inbox) and on one check (Owner block) on a throwaway project (fixture copy plus git repo) in the real multiplexer (herdr, else tmux) with the watcher running
expected: The launched Claude session resumes the UAT session without a resume/restart question, shows no xargs warning, records result: skipped with a reason beginning `Deferred follow-up: will not do.`, offers no 999.x promotion; after the watcher refresh the row/check leaves the TEST lane
result: [pending]

### 2. Deferred permanently on the same phase row and check
expected: Same as above with `deferred permanently` in the recorded reason; no `. .` artifact when the reason is empty
result: [pending]

### 3. Move to phase M (phase row and one check)
expected: Checks recorded as `Deferred follow-up: moved to phase M.`; exactly one todo appears under .planning/todos/pending/ titled with phase M's number and name; `gsd-tools query todo.match-phase M` lists it; source phase can close without the checks
result: [pending]

### 4. Project with a live Claude session: x ... Yes
expected: The live session is focused, the status says `paste the command there`, pasting runs the same instruction, no second session appears; OSC 52 reaches the real terminal clipboard (and a native tool copy when present)
result: [pending]

### 5. VERIFICATION-only phase entry (RESEARCH Pitfall 11)
expected: Its human checks leave the list after settle; if not, record it (A stays the file-level answer)
result: [pending]

### 6. Instruction wording of all three outcomes, and popup fit at 80x24 and 100x30 by eye (the live-session popup is one row taller)
expected: The wording reads right to the owner; no popup row is clipped
result: [pending]

### 7. Inbox footer at >=137 columns vs 100/80 columns
expected: `[x]settle` appears only at >=137 columns (deliberate trade-off, review IN-02); owner accepts that x is otherwise discoverable via help and README, or asks to widen the ladder
result: [pending]

## Summary

total: 7
passed: 0
issues: 0
pending: 7
skipped: 0
blocked: 0

## Gaps
