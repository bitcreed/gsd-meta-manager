---
quick_id: 260722-emn
type: index
title: "Catch up to GSD 1.8.0 — README value prop + tier 1-3 state-reader/feature updates"
plans: 10
waves: 5
---

# 260722-emn — GSD 1.8.0 catch-up (multi-plan)

gsd-meta-manager (this repo, v1.5.0) is a Rust/ratatui TUI that reads other projects'
`.planning/` directories. GSD-core moved 1.2.0 → 1.8.0 and changed several on-disk
formats. This index groups the catch-up work into 10 plans across 5 waves. Within
each wave, every plan's `files_modified` set is strictly disjoint so executors can
run in parallel git worktrees. `mod.rs` is only ever edited by the plans that own it
(plan 6 in wave 2, plan 8 in wave 3) — no other plan touches it.

**Canonical reference for GSD 1.8.0 behavior (read-only):**
`/home/blk/projects/node/gsd-core/gsd-core/bin/` — cite these when format fidelity matters:
- `gsd-tools.cjs` — roadmap `update-plan-progress` (PLAN vs SUMMARY counting), `requirements ready-ids`, `smart-entry`, `workstream` routes
- `bin/lib/state-transition.cjs` — ADR-2207 status vocabulary (`All phases complete` vs `<version> milestone complete` / `Awaiting next milestone`), `current_phase` / `current_phase_name` / `current_plan` frontmatter fields
- `bin/lib/external-job.cjs` + `bin/lib/capability-registry.cjs` — `.planning/async-jobs/<job>.json` manifest and `external_job_waiting`
- `docs/how-to/enable-claude-orchestration-workflow-backend.md` + `tests/fix-2285-claude-orchestration-wiring.test.cjs` — `waves.json` wave/plan manifest shape and path (`.planning/phases/<dir>/waves.json`)
- `tests/workstream.test.cjs` — `.planning/workstreams/<ws>/` layout and `.planning/active-workstream` pointer

## Wave / plan map

| Wave | Plan | Item | files_modified |
|------|------|------|----------------|
| 1 | `260722-emn-1-PLAN.md` | A — README value proposition (docs) | `README.md` |
| 1 | `260722-emn-2-PLAN.md` | B — disk_status correctness | `src/state_reader/disk_status.rs` |
| 1 | `260722-emn-3-PLAN.md` | C — ROADMAP parsing | `src/state_reader/roadmap_md.rs` |
| 1 | `260722-emn-4-PLAN.md` | D — config schema | `src/state_reader/config_json.rs` |
| 1 | `260722-emn-5-PLAN.md` | E — git-commit-time staleness primitive | `src/state_reader/git_ops.rs` |
| 2 | `260722-emn-6-PLAN.md` | F1 — STATE.md vocab/frontmatter/async-jobs + mod.rs wiring | `src/state_reader/state_md.rs`, `src/app.rs`, `src/state_reader/mod.rs` |
| 3 | `260722-emn-7-PLAN.md` | F2 — smart-entry shell-out | `src/state_reader/queue_md.rs` |
| 3 | `260722-emn-8-PLAN.md` | G — workstreams data layer | `src/state_reader/workstreams.rs` (new), `src/state_reader/mod.rs` |
| 4 | `260722-emn-9-PLAN.md` | H — UI wiring | `src/ui/screens/detail.rs`, `src/ui/screens/normal.rs` |
| 5 | `260722-emn-10-PLAN.md` | I — release prep (v1.6.0) | `Cargo.toml`, `Cargo.lock` |

## Disjointness proof (per wave)

- **Wave 1:** `README.md` | `disk_status.rs` | `roadmap_md.rs` | `config_json.rs` | `git_ops.rs` — all distinct files. Tests are inline `#[cfg(test)] mod tests` in each `.rs`, so no shared test file. ✓
- **Wave 2:** only plan 6 runs (state_md.rs + app.rs + mod.rs). `app.rs` is included in plan 6 because `classify_status` / `format_phase_display` (the real ADR-2207 status-vocabulary drivers) and the `*old_state != state` change-detection live there; no other plan touches `app.rs`. ✓
- **Wave 3:** plan 7 = `queue_md.rs`; plan 8 = `workstreams.rs` + `mod.rs`. Disjoint. Both branch from post-wave-2 (plan 6's `ProjectState` fields already merged). ✓
- **Wave 4:** only plan 9 (UI). ✓
- **Wave 5:** only plan 10 (Cargo). ✓

## Cross-plan handoffs (additive contracts, no shared files)

These are compile-time contracts satisfied by wave ordering (a later wave branches
from the merged result of all earlier waves), NOT by editing a shared file:

1. **B → H:** plan 2 adds `has_coverage` / `has_windows` / `has_deferred_items` /
   `has_skeleton` flags to `DiskInference`; plan 9 surfaces them in the Pipeline drill-down.
2. **C → F1:** plan 3 exposes a pure `roadmap_progress(content) -> Option<RoadmapProgress>`
   parser; plan 6 (owns mod.rs) wires "prefer the Progress table over STATE.md frontmatter."
3. **D → H:** plan 4 adds config struct fields; plan 9 adds the Cfg-tab `push(...)`
   display rows in `build_defaults_entries` + toggle/edit setter match arms in detail.rs.
4. **E → F1:** plan 5 adds `git_ops::project_last_activity(project_root) -> Option<...>`;
   plan 6 stores it on `ProjectState.last_activity` (mod.rs).
5. **F1 → F2:** plan 6 adds `ProjectState.project_root`; plan 7 reads it inside
   `suggest_next_commands` to shell out to smart-entry (no signature change, no UI edits).
6. **F1/G → H:** plan 9 surfaces `external_job_waiting`, `current_phase_name`,
   `current_plan`, `last_activity`, and workstream states.

## Commit discipline

Each executor commits atomically per task using conventional-commit prefixes seen in
`git log` (`feat`/`fix`/`docs`/`chore`). Every plan's verify includes `cargo build`
and `cargo test`; plan 10 also runs `cargo clippy -- -D warnings`.
