---
created: 2026-10-29T00:00:00.000Z
title: Run-file open and size hardening in driverctl (LOG.md, log fallback, state.json)
area: driverctl
severity: warning
security_relevant: true
source: .planning/milestones/v2.0-MILESTONE-AUDIT.md tech_debt.security_relevant; carried forward at v2.0 close [inferred — release 2026-10-29]
findings: [27 WR-10, 27 WR-06, 27 WR-11]
related_todos: [2026-10-29-driverctl-log-values-unredacted.md]
files:
  - src/driverctl/
---

## Problem

- **27 WR-10** the LOG.md append opens without O_NOFOLLOW (reachable only under `--unsandboxed`).
- **27 WR-06** the log fallback uses a predictable `/tmp` path.
- **27 WR-11** `state.json` has a read cap but no matching write bound.
- Related, already tracked: `2026-10-29-driverctl-log-values-unredacted.md` (LOG.md values written unredacted).

## Solution

Open run files with O_NOFOLLOW|O_NONBLOCK and a regular-file check (as `src/driver/lock.rs` now does), move the fallback under the per-user state dir with 0700, and bound writes to the read cap. See `.planning/milestones/v2.0-phases/27-*/27-REVIEW-DISPOSITION.md`.
