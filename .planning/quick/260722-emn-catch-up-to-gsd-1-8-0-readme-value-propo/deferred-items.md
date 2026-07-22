# Deferred Items — 260722-emn

## Out-of-scope discoveries (not fixed)

### clippy::large_enum_variant on `Action` (src/action.rs:6)

- **Found during:** plan 6, Task 3 (adding fields to `ProjectState`).
- **Warning:** `large size difference between variants` — the `Action::ProjectStateLoaded`
  variant (which embeds `ProjectState`) is now ~360 bytes.
- **Pre-existing:** `ProjectState` was already ~272 bytes before this plan (360 − ~88 bytes
  of new fields), i.e. already above clippy's 200-byte default threshold. Plan 6's
  mandated field additions (`current_phase_name`, `current_plan`, `external_job_waiting`,
  `last_activity`, `project_root`) nudged it larger but did not introduce the lint.
- **Why not fixed here:** the fix (boxing the variant, e.g. `Box<ProjectState>`) lives in
  `src/action.rs`, which is NOT in plan 6's `files_modified` list. Out of scope per the
  executor scope boundary.
- **Suggested fix:** box the large `Action` variants (`ProjectStateLoaded`, `ArchiveLoaded`)
  in a follow-up quick task that owns `src/action.rs` and its call sites.
- **Gates unaffected:** `cargo build` and `cargo test` both pass.
