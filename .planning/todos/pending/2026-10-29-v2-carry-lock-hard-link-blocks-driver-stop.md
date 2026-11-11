---
created: 2026-10-29T00:00:00.000Z
title: A hard link to the run lock makes driver stop fail
area: driver
severity: warning
security_relevant: true
source: .planning/milestones/v2.0-MILESTONE-AUDIT.md tech_debt.security_relevant; carried forward at v2.0 close [inferred — release 2026-10-29]
findings: [29 WR-08]
files:
  - src/driver/lock.rs
audit_acknowledged:
  milestone: v2.3.0
  at: 2026-11-10
---

## Problem

**29 WR-08**: since acquire/probe refuse a multiply-linked lock file (quick 261028-n0z / t3d), anyone with write access to the run dir can plant a hard link to the lock and make `driver stop` fail. That degrades CTRL-01 (stoppable) to a denial of service. Needs write access to the run dir.

## Solution

Let the stop path identify the holder by a route a planted link cannot block (for example the launcher's scope / recorded pgid, verified against /proc), or make the lock live in a directory the driver cannot write. See `.planning/milestones/v2.0-phases/29-*/29-REVIEW-DISPOSITION.md`.
