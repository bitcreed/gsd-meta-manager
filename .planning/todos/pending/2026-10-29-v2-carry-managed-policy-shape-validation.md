---
created: 2026-10-29T00:00:00.000Z
title: Managed-policy values of the wrong shape are dropped silently or merged loosely
area: launcher
severity: warning
security_relevant: true
source: .planning/milestones/v2.0-MILESTONE-AUDIT.md tech_debt.security_relevant; carried forward at v2.0 close [inferred — release 2026-10-29]
findings: [28 WR-10 = 29 WR-10 = 31 WR-03, 30 WR-14, 30 WR-15]
files:
  - src/launcher/config_dir.rs (managed_settings)
audit_acknowledged:
  milestone: v2.3.0
  at: 2026-11-10
---

## Problem

- **28 WR-10 / 29 WR-10 / 31 WR-03** (one finding) wrong-shape host managed-policy values are dropped silently, with no report to the operator.
- **30 WR-14** permissions scalars merge first-wins.
- **30 WR-15** hook items without a string `command` still reach the managed file.

Quicks 261028-t3f and 261028-v2z made the object-level merge fail closed; these item-level cases remain.

## Solution

Validate each item's shape, refuse (or loudly report) rather than drop, define a deterministic precedence for permissions scalars, and filter command-less hook items, each with a test. See the 28/29/30/31 `*-REVIEW-DISPOSITION.md` under `.planning/milestones/v2.0-phases/`.
