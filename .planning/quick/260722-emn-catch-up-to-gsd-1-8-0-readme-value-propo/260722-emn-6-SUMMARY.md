---
quick_id: 260722-emn
plan: 6
item: F1
subsystem: state_reader
status: complete
tags: [adr-2207, frontmatter, roadmap-progress, async-jobs, staleness]
requires:
  - roadmap_md::roadmap_progress (plan 3)
  - git_ops::project_last_activity (plan 5)
provides:
  - state_md::StateFrontmatter.current_phase / current_phase_name / current_plan
  - state_md::is_milestone_terminal / is_all_phases_complete
  - ProjectState.current_phase_name / current_plan / external_job_waiting / last_activity / project_root
  - app::classify_status ADR-2207 vocabulary
affects:
  - src/state_reader/state_md.rs
  - src/state_reader/mod.rs
  - src/app.rs
tech-stack:
  patterns:
    - serde deserialize_with visitor to coerce YAML scalar (string|int|float|null) to Option<String>
key-files:
  modified:
    - src/state_reader/state_md.rs
    - src/state_reader/mod.rs
    - src/app.rs
decisions:
  - "All phases complete is an ADR-2207 intermediate state -> classify Idle, never a premature milestone Complete"
  - "Milestone-terminal statuses (<version> milestone complete / Awaiting next milestone) classify Complete"
  - "ROADMAP ## Progress table is authoritative for counts; frontmatter is the fallback"
  - "Numeric current_phase/current_plan coerced to String via a scalar visitor so a bare number never aborts frontmatter parsing"
  - "Executed Task 3 (mod.rs) before Task 2 (app.rs) to keep every commit compiling (app.rs consumes ProjectState.current_phase_name)"
metrics:
  tasks: 3
  files: 3
  completed: 2026-07-22
status_line: ADR-2207 status vocabulary + GSD 1.8.0 frontmatter keys, Progress-table authority, async-job detection, and staleness wired into ProjectState and the status/phase classifiers
---

# Quick 260722-emn Plan 6: ADR-2207 Vocabulary, Frontmatter Keys, and Wave-1 Wiring Summary

Adopted GSD 1.8.0's ADR-2207 status vocabulary and `current_phase*/current_plan`
frontmatter, made the ROADMAP `## Progress` table authoritative for counts, detected
`.planning/async-jobs/*.json` external jobs, and wired wave-1 primitives
(`roadmap_progress`, `project_last_activity`) into `ProjectState` — so the dashboard
distinguishes a done-but-not-terminated milestone (`All phases complete`) from a truly
finished one (`<version> milestone complete`).

## What Was Built

### Task 1 — STATE.md frontmatter keys + ADR-2207 helpers (`state_md.rs`, commit `ae2295f`)
- `StateFrontmatter` gains `current_phase`, `current_phase_name`, `current_plan`
  (all `#[serde(default)]`).
- `current_phase` / `current_plan` use a `deserialize_with` visitor
  (`de_opt_scalar_string`) accepting a YAML string, integer, float, or null → `Option<String>`,
  so a bare numeric value (`current_phase: 14`) never aborts the whole frontmatter parse.
- `is_milestone_terminal` (contains "milestone complete" OR "awaiting next milestone") and
  `is_all_phases_complete` (contains "all phases complete") — both case-insensitive.
- Inline tests: string form, numeric form, absent→None, and both helpers across the three
  status families.

### Task 3 — `parse_project_state` rework + new `ProjectState` fields (`mod.rs`, commit `d114026`)
- `ProjectState` gains `current_phase_name: String`, `current_plan: String`,
  `external_job_waiting: bool`, `last_activity: Option<DateTime<Utc>>`, `project_root: PathBuf`.
- `project_root` = parent of `.planning/`; `last_activity` = `git_ops::project_last_activity(project_root)`.
- `current_phase` derivation preference: `current_phase_name` → `current_phase` (as `Phase {n}`)
  → ADR-2207 status (milestone-terminal renders `<milestone> Complete`; `All phases complete`
  renders literally, NOT a premature milestone complete) → legacy stopped_at/count heuristic.
- After `parse_roadmap_phases`, `roadmap_progress(&content)` overrides the four progress
  counts when a `## Progress` table is present.
- `detect_async_jobs` sets `external_job_waiting` when `.planning/async-jobs/` holds any `*.json`.
- Eight tempfile-backed inline tests covering every new behavior plus legacy preservation.

### Task 2 — ADR-2207-aware classification (`app.rs`, commit `2d33f07`)
- `classify_status`: milestone-terminal → `Complete`; `All phases complete` → `Idle`
  (both branches precede the generic `contains("complete")` check so the intermediate
  state no longer falls through to Complete). Ordinary statuses unchanged.
- `format_phase_display` returns a non-empty `current_phase_name` when present; otherwise
  the existing count-based logic is unchanged.
- Four new tests; all prior `test_format_phase_display_*` cases still green.

## Deviations from Plan

### Execution-order adjustment (Rule 3 — blocking build order)
- **Issue:** Task 2 (`app.rs::format_phase_display`) reads `ProjectState.current_phase_name`,
  a field added in Task 3 (`mod.rs`). Committing Task 2 before Task 3 would produce a
  non-compiling commit, and each task's `<verify>` runs `cargo test`.
- **Fix:** Executed and committed Task 3 before Task 2. Every commit compiles and passes
  tests; task→commit-message mapping preserved.
- **Files modified:** none beyond plan scope.

## Deferred Issues

- **`clippy::large_enum_variant` on `Action` (`src/action.rs`)** — pre-existing (the embedded
  `ProjectState` was already ~272 bytes, above clippy's 200-byte threshold, before this plan).
  Plan 6's mandated fields enlarged it but did not introduce the lint. The fix (boxing the
  variant) lives in `src/action.rs`, which is not in this plan's `files_modified`, so it is
  out of scope. `cargo build` and `cargo test` (the required gates) both pass. Logged in
  `deferred-items.md`.

## Verification

- `cargo build` — clean.
- `cargo test` — 192 passed (5 suites), up from 168 (+24 new tests across the three files).
- ADR-2207 status distinction, frontmatter-key preference, Progress-table authority,
  async-job detection, and staleness/project_root population all covered by tests.

## Self-Check: PASSED
