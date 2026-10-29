---
created: 2026-10-29T00:00:00.000Z
title: driverctl LOG.md writes driver-supplied values without redaction
area: driver
severity: minor
source: .planning/v2.0-MILESTONE-AUDIT.md (SAFE-04 evidence), split out by quick 261029-1dp [inferred — 2026-10-29]
files:
  - src/driverctl/log.rs (format_line)
---

## Problem

`driverctl log` appends driver-authored records to LOG.md. `format_line` in
`src/driverctl/log.rs` writes the `kv` values and `detail` lines verbatim, so a
credential-shaped string the driver passes as a value would reach LOG.md unredacted.
SAFE-04 asks that credentials are redacted when captured, not when rendered.

Quick 261029-1dp closed the master stdout/stderr half of SAFE-04 (the launcher capture
pump) and marked SAFE-04 Complete. This is the remaining, lower-risk half: LOG.md holds
records the driver writes about its own decisions, not captured process output, and no
credential enters the bwrap sandbox (LNCH-04/12), so the exposure is low (threat
T-261029-06, accepted).

## Solution

Run each `kv` value and `detail` line through `crate::journal::redact::redact` in
`format_line` (before the bytes are appended), with a test that a `ghp_`/`sk-ant-` shaped
value never reaches the file. Keep the single-line `kv` shape (redaction replacements
contain no newline).
