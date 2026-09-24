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

## backlog-content-empty — Backlog tab content pane shows "Empty — no .md files" for every item
- **Date:** 2026-09-24
- **Error patterns:** Empty — no .md files in this backlog directory, 3:Backlog, Enter expand, Content pane, 999.x, .gitkeep, every item every project, ROADMAP.md BACKLOG section, multi-line body one row, middle dot ·
- **Root cause(s):** `backlog::load_backlog_content` read only `.md` files inside `.planning/phases/999.N-<slug>/`, but GSD's add-backlog workflow stores the item only as a `### Phase 999.N: … (BACKLOG)` section in ROADMAP.md and leaves the dir with `.gitkeep` (11/11 registered dirs); latent co-defect: the pane escaped the whole body with `Untrusted::shown()`, which turns `\n` into `·` and collapses multi-line content into one row
- **Fix:** `roadmap_md::phase_section` + loader returns ROADMAP entry then every dir `.md` (labelled), None only when neither (3a8d77e); pane renders via `archive::render_markdown_lines(as_raw_for_logic_only())` with wrap, new empty text (d44c160)
- **Files changed:** src/state_reader/roadmap_md.rs, src/state_reader/backlog.rs, src/ui/screens/detail.rs, src/archive.rs
- **Why not caught:** test gate — backlog fixtures planted a `999.1-BACKLOG.md` (a layout GSD never writes), sharing the loader's assumption; no test crossed Enter -> load -> render with a `.gitkeep`-only dir or multi-line content
- **Recurrence guard:** regression tests src/state_reader/backlog.rs:`a_gitkeep_only_backlog_item_loads_its_roadmap_section`, src/ui/screens/detail.rs:`enter_on_a_gitkeep_only_backlog_item_draws_its_roadmap_section`, `a_multi_line_backlog_body_renders_one_row_per_line_not_one_joined_row`. KB pattern: (1) when a reader's source is measured always-empty, find where the upstream GSD workflow (`gsd-core/workflows/*.md`) actually writes the data and mirror THAT layout in fixtures; (2) never render a multi-line `Untrusted` through `shown()` — split per line via `render_markdown_lines`
---

