---
status: resolved
trigger: "Audit rest of codebase for raw string/integer phase-id handling that mis-handles decimal (7.1) or zero-padded (07.1/05) phases; route through src/state_reader/phase_num.rs; regression tests; commit."
created: 2026-09-23
updated: 2026-09-23
---

# Debug: phase-id normalization audit (follow-up to ttbook-detail-pane-phases)

## Symptoms

- **Expected:** every place that identifies, compares, sorts, parses or looks up a phase treats
  `7.1` == `07.1`, `5` == `05`, and orders `7 < 7.1 < 8` (via `state_reader::phase_num`).
- **Actual:** `ttbook-detail-pane-phases` left `phase_disk_statuses` lookups, deferred-verification
  rows and driver gates unaudited.
- **Reproduction:** ttbook (ROADMAP `7.1`, dir `07.1-…`, now 07.1-NN PLAN/SUMMARY pairs), read-only.

## Evidence

Sweep: `parse::<…>` on ids, `{:02}` formatting, `==`/`!=` on `.number`/`current_phase`,
`phase_disk_statuses.get`, `depends_on`, `deferred_verification`, sorts, `NN-` prefix
matching, `-PLAN.md`/`-SUMMARY.md` handling, archive and backlog dir parsing.
Around 30 sites audited.

## Resolution

root_cause / fix (real bugs):
  1. `disk_status::plan_index` parsed the phase half as `u32`, so `07.1-01-slug-PLAN.md` never
     paired with `07.1-01-SUMMARY.md` (an executed inserted phase read `Planned`). Now
     `(PhaseNum, u32)`; `PlanTokens::label` renders `07.1-01` via new `PhaseNum::padded`.
  2. `archive::parse_phase_dir` parsed the prefix as `u32`, so archived `07.1-…` phases were
     dropped from the Archive tab. `PhaseArchive.number` is a `PhaseNum`, sorted numerically.
  3. `driver::router`: argv target, `Depends on` refs and roadmap rows compared with raw `==`,
     so `07.1` on argv parked `StateUnverified` against roadmap `7.1`, and `Depends on: Phase 07`
     stayed unsatisfied against a complete `7`. The G15 `phase_identity` kept padding, so a
     `07.1` Deferred Verification row never gated `7.1` (fail-open). New
     `ProjectState::{roadmap_phase, disk_status_for}` (same_phase, walks `phases` in order, so
     no map iteration); router + `run::drpev_stages` use them; `phase_identity` → `phase_key`.
  4. `change_tracker`: raw `==` re-announced "Phase completed" when a merged row's first-seen
     spelling moved (`07.1` → `7.1`). Now `same_phase`.
  5. `roadmap_md::is_sentinel_phase`: missed padded `00` / `0999.x`. Now numeric.

benign (dismissed):
  - `phase_disk_statuses.get(&phase.number)` in detail.rs, roadmap_widget, `phase_plan_counts`,
    `PhaseMarker::decide`, `run::typed_state_lines`: the key comes from the same merged row.
  - `goal.rs` `roadmap_phases.contains(named_phase)`: deliberate byte-equality to the roadmap
    token (approval security control); the seam hands the model the roadmap spelling; fail-closed.
  - backlog `999.N` f64 sort key; `plan_waves` lexical plan-id sort (same phase prefix);
    `state_md::scalar_u32` (counts); `session_detector`/`liveness`/`gate.rs`/`policy.rs`
    parses (PIDs, versions, brace ranges) are not phase ids.
  - `state_md::deferred_verification_phases` returns raw cells; normalization happens at the
    single consumer (router), now pad-insensitive.

verification: `cargo test --no-fail-fast` → 2190 passed / 1 failed (known local
  `envelope::policy` git-version constants test). `cargo clippy -- -D warnings` clean.

## Decisions [AUDIT]

- [AUDIT] Router keeps the argv spelling in `Park.detail` and the emitted command
  (`/gsd-execute-phase 07.1`); only lookups are normalized — keeps SAFE-04's "detail is the argv
  token" contract, and GSD accepts either spelling.
- [AUDIT] `plan_index` pairing change is phase-id matching, not status derivation; no status
  rules were touched.
- [AUDIT] goal.rs exact-match refusal left as-is (security control, fail-closed).
- [AUDIT] Debugged directly in the dispatched subagent (no nested session-manager/debugger
  chain; the human is unavailable and those agents checkpoint via AskUserQuestion). No Agent()
  for GSD types was spawned, so no isolation dispatch applied.

## Regression coverage

- `phase_num::tests::padded_is_the_directory_prefix_spelling`
- `disk_status::tests::slugged_decimal_phase_plans_pair_with_their_summaries`
- `archive::tests::archived_decimal_phases_are_listed_in_numeric_order`
- `router::tests::{decimal_and_padded_phase_ids_resolve_pad_insensitively,
  the_deferred_verification_gate_is_pad_insensitive_for_decimal_phases}`
- `change_tracker::tests::a_respelled_completed_phase_is_not_reported_as_newly_completed`
- `roadmap_md::tests::padded_backlog_sentinels_are_excluded_too`

## Left open

- None known.
