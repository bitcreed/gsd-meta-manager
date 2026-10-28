---
created: 2026-10-28T00:00:00.000Z
title: A DIRECTION.md save can be lost when give-back hits ENOENT; restore with a NOREPLACE rename (30-REVIEW WR-10)
area: driver
severity: major
source: .planning/phases/30-driver-tab-on-direction-md/30-REVIEW.md
completed: 2026-10-28
resolved_by: quick 261028-n0z 44eef32 (WR-10: give-back ENOENT restores by NOREPLACE rename)
files:
  - src/driver/direction.rs:684-694
  - src/launcher/run.rs:613
  - src/driverctl/serve.rs:930
---

## Problem

`give_back_newest` restores the human's newest save with `renameat2(RENAME_EXCHANGE)`, which fails with
`ENOENT` if DIRECTION.md is absent at that instant (an unlink-then-create editor save, or a `git checkout`).
The newest bytes are then left only at `.DIRECTION.md.fm.<pid>.<n>`, and the next launch or `driverctl serve`
sweeps that temp after 60 s (`STALE_TEMP_AGE`). The save is gone. Still present in code at 2026-10-28
(no NOREPLACE in direction.rs).

## Solution

In the give-back exchange error arm, on `Errno::NOENT` rename with `RenameFlags::NOREPLACE`: `Ok` returns,
`EXIST` continues to the next round, anything else is an error. Add a race-hook `Point` between the exchange
back and the give-back so a unit test can unlink DIRECTION.md there and assert the newest save lands at
DIRECTION.md. Full fix text: 30-REVIEW.md WR-10.
