---
created: 2026-10-28T00:00:00.000Z
title: Launcher-marker scan: add an in-flight flag like agents_scan_in_flight (30-REVIEW WR-11, separate part)
area: ui
severity: minor
source: .planning/phases/30-driver-tab-on-direction-md/30-REVIEW.md WR-11 fix text, deferred by quick 261028-t3d [inferred — cleanup 2026-10-28]
files:
  - src/app.rs (the 20-tick `launcher_markers_for` spawn, the `LauncherMarkersRescanned` handler, and `every_per_alias_map_is_pruned`'s exhaustive AppContext destructure)
  - src/ui/screens/mod.rs (AppContext)
  - src/ui/screens/delete_confirm.rs (AppContext literal)
  - src/ui/screens/normal.rs (AppContext literal)
  - src/ui/screens/driver_confirm.rs (AppContext literal)
  - src/ui/screens/detail.rs (AppContext literal)
---

## Problem

The launcher-marker scan is spawned on every 20th tick with no in-flight guard, unlike
`agents_scan_in_flight`. After quick 261028-t3d a planted FIFO or symlink at `launcher.lock` no longer
parks it (the probe is a guarded, non-blocking open). A slow or hung filesystem (NFS, FUSE) under any
registered project can still stack one blocking task per scan, and each one holds a tokio blocking-pool
thread.

This is the "Separately" part of 30-REVIEW WR-11's fix text. It is slow-filesystem hardening rather than a
FIFO or symlink open, and it adds an `AppContext` field to six struct literals, so it was split out of the
t3d item.

## Solution

Mirror `agents_scan_in_flight`: set the flag before the spawn, wrap the scan in `catch_unwind` so the
closure always sends (an empty map after a panic), and clear the flag first thing in the
`LauncherMarkersRescanned` handler. Classify the new field in the exhaustive destructure in
`every_per_alias_map_is_pruned`. Add tests mirroring `no_second_agents_scan_spawns_while_one_is_in_flight`
and `the_agents_scan_handler_clears_the_in_flight_flag_for_an_empty_result`.
