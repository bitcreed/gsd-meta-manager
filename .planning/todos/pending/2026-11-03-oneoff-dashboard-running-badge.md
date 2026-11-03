---
created: 2026-11-03T00:00:00.000Z
title: Dashboard badge for a running one-off task
area: ui
severity: minor
source: .planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md
files:
  - src/ui/screens/normal.rs
  - src/oneoff/store.rs
  - src/ui/screens/mod.rs
---

## Problem

A one-off task (quick 261103-f5y) shows its status only on the project's Driver
tab (`t` / `T` and the one-line status). The dashboard row gives no sign that a
sandboxed agent is working on the project, so a user scanning the dashboard
cannot see it. CONTEXT.md asked for a small running badge "if cheap"; it was
deferred (I-13) because the dashboard has no per-project read of the one-off
root today: it would need a lock probe of
`<data>/gsd-meta-manager/oneoff/<hash8>/oneoff.lock` for every registered
project on the dashboard tick.

## Solution

Fold a one-off probe into the existing 20-tick launcher-marker scan
(`Action::LauncherMarkersRescanned`): one `oneoff::store::probe` per project,
off the render thread, stored beside `launcher_marker`, and a badge in the
Status cell only while it reads `HeldBy`. Gate it on
`GSDMM_EXPERIMENTAL_FEATURES` like every driver surface, and add the
render-escape and flag-off tests.
