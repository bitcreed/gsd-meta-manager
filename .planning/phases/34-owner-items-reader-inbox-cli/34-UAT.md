---
status: testing
phase: 34-owner-items-reader-inbox-cli
source: [34-VERIFICATION.md]
started: 2026-11-14T20:05:27Z
updated: 2026-11-14T20:05:27Z
---

## Current Test

number: 1
name: Re-run the live check on a cold OS page cache with the release binary: gsd-meta-manager inbox --json over the owner's registered projects
expected: |
  Wall time under 1 s (warm release measured 0.29-0.33 s; one cold debug run took 10.7 s wall with 1.3 s CPU).
awaiting: user response

## Tests

### 1. Re-run the live check on a cold OS page cache with the release binary: gsd-meta-manager inbox --json over the owner's registered projects
expected: Wall time under 1 s. Warm release measured 0.29-0.33 s; the first debug run after a fresh build took 10.7 s wall with only 1.3 s CPU, which points at cold-disk I/O.
result: [pending]

### 2. Read the plain `gsd-meta-manager inbox` checklist for your own projects and judge whether it answers 'what checks do I have to run?' (wording, ordering, noise)
expected: The lane sections, ordering (closes-a-phase first) and one-line group format read well enough to paste into a note
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
