# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## archive-milestone-view-loading — Archive drill-in flips back to "Loading..." every ~5 s
- **Date:** 2026-09-23
- **Error patterns:** Loading..., flips back after a few seconds, Archive tab, 8:Arch, milestone drill-in, content disappears on refresh, ESC and re-enter fixes it, periodic, 20-tick, prune, cache emptied
- **Root cause(s):** `App::prune_driver_maps` (every 20 ticks) retained `AppContext::archive_cache` by registered alias while the map was keyed by milestone version, so every loaded archive was dropped on every pass (introduced 392902d, 18-11, v1.7.0); the same milestone-only key also let two projects with the same archived version share one entry
- **Fix:** `archive::ArchiveCache` newtype keyed alias -> milestone whose only prune is `retain_aliases`; all readers pass the alias (3c0e38f). Open archives reload in place on Planning file changes without setting `archive_loading`, cursors clamped (581aa7d)
- **Files changed:** src/archive.rs, src/app.rs, src/ui/screens/mod.rs, src/ui/screens/detail.rs, src/ui/screens/render_escape_guard.rs, src/ui/screens/delete_confirm.rs, src/ui/screens/normal.rs, src/ui/screens/driver_confirm.rs
- **Why not caught:** test gate: the prune test `every_per_alias_driver_map_is_pruned` populated `archive_cache` with alias keys, sharing the bug's assumption; no test crossed the drill-in -> tick -> render seam
- **Recurrence guard:** type refinement (`ArchiveCache`, alias-only prune) + regression tests src/app.rs:`archive_milestone_view_keeps_its_content_across_the_periodic_prune`, `the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry`, `archive_milestone_view_reloads_in_place_when_planning_files_change`. KB pattern: when a cached view "flips back to Loading after N seconds", check every periodic prune/retain against the map's ACTUAL key before suspecting the poller that shares its timer
---

