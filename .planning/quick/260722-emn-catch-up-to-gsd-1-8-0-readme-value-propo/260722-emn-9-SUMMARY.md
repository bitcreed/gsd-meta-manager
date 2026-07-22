---
quick_id: 260722-emn
plan: 9
item: H
wave: 4
status: complete
subsystem: ui
tags: [tui, ratatui, pipeline-tab, cfg-tab, dashboard, gsd-1.8.0]
requires:
  - 260722-emn-2  # disk_status artifact flags
  - 260722-emn-4  # config_json new structs
  - 260722-emn-6  # RoadmapProgress
  - 260722-emn-8  # ProjectState fields / workstreams
provides:
  - waves.json manifest rendering in Pipe tab
  - GSD 1.8.0 config keys in Cfg tab (view + edit + persist)
  - workstreams / external-job-waiting / new-artifact surfacing in dashboard + pipeline
affects:
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
tech-stack:
  added: []
  patterns:
    - lenient serde deserialization for evolving JSON manifests
    - ConfigValueKind::ReadOnly for shape-varying (JSON-value) config keys
key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
decisions:
  - Waves manifest resolved for the SELECTED phase (matches the Pipe tab right pane) rather than a separate "current phase" lookup.
  - Shape-varying keys (security_asvs_level, security_block_on, review.reviewer_instances, sub_repos) rendered read-only via a new ConfigValueKind::ReadOnly variant; not made editable per plan.
  - New config keys use dotted key strings (e.g. "workflow.mvp_mode", "statusline.show_git") to disambiguate nested paths in the setter match arms.
  - Workstream cue placed in the Phase column so it survives all 80/60/40 layout branches; external-job badge placed on the alias cell below pause, above session priority.
metrics:
  duration: ~18min
  completed: 2026-07-22
  tasks: 3
  files: 2
human_verification:
  status: passed
  verified_on: 2026-07-22
  checkpoint: "Task 4 (checkpoint:human-verify, blocking) — visually verified by user post-release"
  what: "Pipe tab waves.json rendering; Cfg tab new keys grouping + bool toggle/persist; dashboard workstream cue + external-job-waiting badge; layout intact at 80/60/40 columns."
  how: "cargo run against a registered project; open a phase with waves.json, open Cfg tab and toggle a new bool, confirm a project with .planning/workstreams/* and one with .planning/async-jobs/*.json render their cues."
---

# Quick 260722-emn Plan 9: UI wiring (waves.json, config keys, workstreams/external-job/artifacts) Summary

Wired the wave-1–3 data-layer additions into the TUI so the reader work becomes visible: the Pipe tab now renders the `waves.json` parallelism manifest, the Cfg tab lists and edits the new GSD 1.4–1.8 config keys, and the dashboard/pipeline surface workstreams, external-job-waiting, and the four new artifact sub-stages.

## What was built

### Task 1 — waves.json manifest in Pipe tab (commit 23dd9ae)
- Added lenient `parse_waves_manifest` plus `WavesManifest`/`WaveEntry`/`WavePlan` structs (`#[serde(default)]` on every field; unknown fields ignored). Wave id tolerated as string or number; plan id read from `id` or `plan`.
- `build_waves_lines` renders a compact "Waves (parallelism)" section: per-wave label, a parallelism hint (`N parallel`, plus `Nf` files-touched when present), and the plan ids.
- `render_pipeline_tab` resolves the selected phase's dir via `find_phase_dir(<path>/.planning, phase.number)` and reads `<phase_dir>/waves.json`; absent/unparsable → nothing rendered (no error noise).
- Unit tests: two-wave manifest, garbage → `None`, lenient defaults (numeric wave id, id-less plan, empty object).

### Task 2 — GSD 1.8.0 config keys in Cfg tab (commit 03a631b)
- New categories: Orchestration, Statusline, Routing, External Job, Capabilities, Review.
- Extended Planning with `specless_probe_fallback`, `assumption_delta`, `plan_drift_precheck`, `plan_chunked`, `context_guard_mode`; extended Execution with `api_coverage_gate`, `windows_enforce`, `mvp_mode`, `test_gate_timeout`, `code_review_command`, plus read-only `security_asvs_level` / `security_block_on`.
- Added `graphify.graph_path`, `phase_id_convention`, `claude_md_path`, and read-only `sub_repos`.
- Editable scalar/bool/int keys wired through `set_config_value` (dropdown), `mutate_config_entry` (Enter toggle/increment), `set_string_value` (text edit), and `clear_config_value` (x-to-clear), all persisting via `get_or_insert_with`.
- Shape-varying keys render read-only via a new `ConfigValueKind::ReadOnly` variant (italic dark-gray), with the exhaustive `mutate_config_entry` match updated to `ReadOnly => false`.

### Task 3 — workstreams, external-job-waiting, new artifacts (commit 1518a91, plus the external-job pipeline indicator landed in 23dd9ae)
- `build_substage_lines`: Skeleton/Windows/Deferred added under Plan, Coverage under Execute; `plan_touched`/`exec_touched` gates extended so these sections still show on otherwise-untouched phases.
- Pipeline right pane: distinct "external job waiting" line when `state.external_job_waiting` (so a blocked phase reads as blocked, not stuck).
- `normal.rs::render_main`: hourglass badge on the alias cell for `external_job_waiting` (priority pause > external-job > session); workstream cue in the Phase column (`[ws:<active>]` or `[N ws]`) across all 80/60/40 layout branches.

## Verification
- `cargo build`: clean.
- `cargo test`: 210 passed (5 suites).
- `cargo clippy`: clean for both modified files (the single remaining warning is a pre-existing large-enum-variant lint in `src/action.rs`, out of scope for this plan).
- `parse_waves_manifest` unit-tested (3 tests).

## Deviations from Plan
- The external-job-waiting pipeline indicator (a Task 3 item) was implemented together with the `render_pipeline_tab` edit in Task 1's commit rather than Task 3's, since both touched the same render block. Net behavior matches the plan; only the commit boundary differs.
- No other deviations. Auto-fix count: the only inline fix was surfacing `WavePlan.files_modified` in the wave line to make the captured-but-unread field load-bearing (removed a dead-code warning) — consistent with the plan's "capture files_modified if present".

## Known Stubs
None. All new UI surfaces are wired to real data-layer fields; no placeholder/mock data introduced.

## Human Verification (deferred)
Task 4 is a blocking `checkpoint:human-verify` for visual/layout confirmation. Per execution constraints it was not blocked on; it was deferred to the user and **verified by the user on 2026-07-22** (post-v1.6.0-release): Pipe waves rendering, Cfg new keys, workstream/external-job badges all confirmed good at the target widths.

## Self-Check: PASSED
- Commits present: 23dd9ae (Task 1), 03a631b (Task 2), 1518a91 (Task 3) — verified in git log.
- Files modified: src/ui/screens/detail.rs, src/ui/screens/normal.rs — both exist and compile.
