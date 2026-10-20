---
status: testing
phase: 28-sandboxed-driver-launcher
source: [28-VERIFICATION.md]
started: 2026-10-20T00:47:15Z
updated: 2026-10-20T00:47:15Z
---

## Current Test

number: 1
name: Run `gsd-meta-manager driver auth setup` and complete the browser login (`claude setup-token`)
expected: |
  A long-lived token is stored at ~/.config/gsd-meta-manager/driver-oauth-token (mode 0600); `driver auth status` then reports 'stored token' and the next launch uses it
awaiting: user response

## Tests

### 1. Run `gsd-meta-manager driver auth setup` and complete the browser login (`claude setup-token`)
expected: A long-lived token is stored at ~/.config/gsd-meta-manager/driver-oauth-token (mode 0600); `driver auth status` then reports 'stored token' and the next launch uses it
result: [pending]

### 2. On a real macOS host (no bwrap, no systemd-run): `driver launch` without a flag, then with `--unsandboxed`
expected: Refuses by default naming the missing pieces; with the flag runs unsandboxed, records `sandbox: degraded` on the run, and the Driver tab shows the badge
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
