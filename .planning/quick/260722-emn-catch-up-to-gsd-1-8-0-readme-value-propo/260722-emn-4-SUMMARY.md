---
quick_id: 260722-emn
plan: 4
item: D
subsystem: state_reader
status: complete
tags: [config, serde, data-model, gsd-1.8]
requires: []
provides: [config_json.gsd-1.4-1.8-keys]
affects: [src/state_reader/config_json.rs]
tech-stack:
  added: []
  patterns: [serde-optional-skip-serializing, serde_json::Value-for-shape-varying-keys]
key-files:
  created: []
  modified: [src/state_reader/config_json.rs]
decisions:
  - Shape-varying keys (review.reviewer_instances, security_asvs_level, security_block_on, sub_repos) modeled as Option<serde_json::Value>, mirroring the existing quick_branch_template precedent.
  - Every new GsdConfig/WorkflowConfig/GraphifyConfig field uses skip_serializing_if = "Option::is_none" so unset keys are never injected on save (non-intrusive constraint).
metrics:
  duration: 6min
  completed: 2026-07-22
  tasks: 2
  files: 1
---

# Quick 260722-emn Plan 4: config_json.rs GSD 1.4–1.8 Schema Keys Summary

Extended `GsdConfig` with the GSD 1.4–1.8 `config.json` blocks and scalar keys the struct did not yet model — six new nested blocks, twelve new `workflow.*` gates, `graphify.graph_path`, and three top-level scalars — all as omit-when-unset options so the Cfg tab reads and round-trips them without dropping data or rewriting a project's config with null keys.

## What Was Built

### Task 1 — New top-level config blocks and scalar keys (commit 9527bde)
- New structs: `ClaudeOrchestrationConfig` (enabled, execution_backend, min_agent_sdk_version), `StatuslineConfig` (show_context_tokens, state_format, show_git), `DynamicRoutingConfig` (provider_escalation, max_escalations), `ReviewConfig` (reviewer_instances as `serde_json::Value`), `ExternalJobConfig` (submit_timeout_ms, poll_timeout_ms, artifact_dir), `CapabilitiesConfig` (strict_known_registries, auto_update).
- New `GsdConfig` fields for all six blocks plus scalars `phase_id_convention`, `claude_md_path`, and `sub_repos` (`serde_json::Value`).
- Every new `GsdConfig` field annotated `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- Tests: full-blob deserialize with value assertions; `{}` → all None; `GsdConfig::default()` serialize omits all new keys.

### Task 2 — New workflow.* gates and graphify.graph_path (commit 4994c05)
- `WorkflowConfig` extended with 1.8 gates `api_coverage_gate`, `windows_enforce`, `specless_probe_fallback`, `assumption_delta` (bools), `test_gate_timeout` (u32); and 1.4–1.6 keys `context_guard_mode` (String), `plan_drift_precheck` (bool), `security_asvs_level` / `security_block_on` (shape-varying `serde_json::Value`), `mvp_mode` (bool), `code_review_command` (String), `plan_chunked` (bool).
- `GraphifyConfig` extended with `graph_path` (String).
- All new fields use `skip_serializing_if = "Option::is_none"`.
- Tests: round-trip (parse → serialize → re-parse) preserving values; `{}` and a partial `workflow`/`graphify` block leave all new keys None.

## Verification

- `cargo build` — clean.
- `cargo test` — 130 passed (5 suites).
- `cargo test --lib config_json` — 9 passed (4 pre-existing + 5 new).
- No UI file touched; `src/state_reader/mod.rs` untouched. Cfg-tab surfacing remains deferred to plan 9 (Item H).

## Deviations from Plan

None — plan executed exactly as written. Key names were cross-checked against the canonical GSD 1.8.0 source in `/home/blk/projects/node/gsd-core/gsd-core/bin/lib/*.cjs` before implementation.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/state_reader/config_json.rs
- FOUND commit: 9527bde (Task 1)
- FOUND commit: 4994c05 (Task 2)
