---
created: 2026-10-29T00:00:00.000Z
title: Capture-pump redaction edge cases (SAFE-04 follow-ups)
area: launcher
severity: warning
security_relevant: true
source: .planning/milestones/v2.0-MILESTONE-AUDIT.md tech_debt.security_relevant; carried forward at v2.0 close [inferred — release 2026-10-29]
findings: [28 WR-11, 28 WR-12, 28 WR-13 = 29 WR-14 = 31 WR-01]
files:
  - src/launcher/process.rs (pump, ~105-186)
audit_acknowledged:
  milestone: v2.3.0
  at: 2026-11-10
---

## Problem

The master's stdout/stderr capture pump (SAFE-04 launcher half) has three open edge cases:

- **28 WR-11** stdout and stderr share one pipe; a write larger than PIPE_BUF can interleave mid-line and split a token so the redactor misses it.
- **28 WR-12** a non-JSON line is redacted as raw text, where the `\b` rules miss tokens that follow a JSON escape.
- **28 WR-13 / 29 WR-14 / 31 WR-01** (one finding) a multi-line plain-text PEM private key is not redacted.

No credential enters the bwrap sandbox (LNCH-04/12), which bounds the exposure.

## Solution

Separate pipes (or line-atomic framing) per stream, JSON-unescape before matching, and a multi-line PEM block rule, each with a test that a credential-shaped input never reaches the journal. Status per finding: `.planning/milestones/v2.0-phases/28-*/28-REVIEW-DISPOSITION.md`, `29-*`, `31-*`.
