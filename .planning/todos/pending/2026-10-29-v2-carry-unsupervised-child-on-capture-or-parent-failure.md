---
created: 2026-10-29T00:00:00.000Z
title: Child left unsupervised when capture spawn fails or (degraded) the launcher dies
area: launcher
severity: warning
security_relevant: true
source: .planning/milestones/v2.0-MILESTONE-AUDIT.md tech_debt.security_relevant; carried forward at v2.0 close [inferred — release 2026-10-29]
findings: [28 WR-14 = 29 WR-15 = 31 WR-02, 30 WR-16]
files:
  - src/launcher/process.rs (116-129 capture-thread spawn; 93-95 die-with-parent comment)
---

## Problem

- **28 WR-14 / 29 WR-15 / 31 WR-02** (one finding) if the capture thread fails to spawn, the child keeps running with nobody supervising or capturing it.
- **30 WR-16** die-with-parent exists only in the sandboxed posture. In the `--unsandboxed` degraded posture a dead launcher leaves the child running and its stream is dropped; the comment at process.rs:93-95 overstates containment.

## Solution

Kill the child (scope or process group) when capture setup fails; in degraded mode add a parent-death signal or a watchdog, and correct the comment. See `.planning/milestones/v2.0-phases/28-*/28-REVIEW-DISPOSITION.md`, `.planning/milestones/v2.0-phases/30-*/30-REVIEW-DISPOSITION.md`.
