---
phase: quick-260512-ecm
plan: 01
subsystem: state-reader + ui-detail
tags: [secure-phase, substage, disk-inference, detail-screen]
dependency_graph:
  requires: []
  provides:
    - DiskInference.has_security
    - "Security" row in Plan sub-stages drill-down
  affects:
    - src/state_reader/disk_status.rs
    - src/ui/screens/detail.rs
tech_stack:
  added: []
  patterns:
    - Mirrors existing per-artifact suffix detection (PATTERNS / PLAN-CHECK / VALIDATION / UI-SPEC / UI-CHECK / AI-SPEC / REVIEW / UI-REVIEW)
key_files:
  created: []
  modified:
    - src/state_reader/disk_status.rs
    - src/ui/screens/detail.rs
decisions:
  - SECURITY is an informational sub-stage artifact, not a sixth pipeline stage — it does NOT alter D/R/P/E/V status.
  - Security row sits at the top of the Plan sub-stages list (frames the rest of planning conceptually).
metrics:
  duration: "~5 minutes"
  completed: 2026-05-12
  tasks_completed: 3
  files_modified: 2
  commits: 3
requirements:
  - QUICK-260512-ecm
---

# quick-260512-ecm: Add support for GSD's optional secure-phase artifact

One-liner: Mirror the existing per-artifact suffix-detection pattern for `SECURITY.md` so meta-manager surfaces `/gsd:secure-phase` output in the Plan sub-stages drill-down.

## What changed

### `src/state_reader/disk_status.rs`

- **Struct field** (line ~25): added `pub has_security: bool,` adjacent to `has_verification` (peer of optional, top-level-ish per-phase artifacts that do NOT gate a pipeline stage).
- **Mutable accumulator** (line ~77): added `let mut has_security = false;` next to the other `has_*` flags.
- **Detection branch** (lines ~123–126): added
  ```rust
  if name == "SECURITY.md" || name.ends_with("-SECURITY.md") {
      has_security = true;
      continue;
  }
  ```
  Placed immediately after the `VALIDATION.md` branch, mirroring the exact shape (early-filter `continue;`) of the other sub-stage detectors.
- **Return literal** (line ~175): added `has_security,` adjacent to `has_verification`.

### `src/ui/screens/detail.rs`

- **`build_substage_lines`** (lines ~3033–3052):
  - Extended `plan_touched` to include `|| inf.has_security` so the Plan sub-stages section renders when SECURITY.md is the only planning artifact present.
  - Added `push_substage(&mut lines, "Security", inf.has_security);` at the **top** of the Plan sub-stages list (before Patterns).

## Tests added

In `src/state_reader/disk_status.rs::tests`:

1. `test_security_md_detected` — `05-01-SECURITY.md` sets `has_security` AND does not alter `plan_count`/`summary_count`/`status` (informational-only invariant).
2. `test_standalone_security_md_detected` — bare `SECURITY.md` sets `has_security`.
3. `test_empty_dir_has_no_security` — empty directory leaves `has_security == false`.

All three pass. All 17 `disk_status` tests pass. Full suite: **105 passed** (`cargo test`).

## Deviations from Plan

None — plan executed exactly as written. Tests for Task 1 were committed as a separate `test(...)` commit before implementation, per the plan's `tdd="true"` directive.

## Verification

- `cargo check --all-targets` — clean (0 warnings, 0 errors)
- `cargo test` — 105 passed across 5 suites
- `cargo clippy --all-targets` on touched files — clean
  - Note: a pre-existing `clippy::cmp_owned` warning in `src/project_creator.rs:146` is **out of scope** (logged in `deferred-items.md`); `nextest` is not installed on this host, so `cargo test` was used as the fallback specified by the plan.
- Reference fixture confirmed:
  `~/projects/rust/usbee/.planning/phases/01-tile-popover-hotplug-daemon-missing-state-v0-1/01-01-SECURITY.md` exists and matches the `*-SECURITY.md` suffix the detector now recognizes. When meta-manager points at the usbee project, the detail screen for that phase will now show `✓ Security` at the top of the Plan sub-stages list.

## Commits

- `60aaf8d` — `test(quick-260512-ecm): add failing tests for SECURITY.md detection` (RED)
- `7de6cd7` — `feat(quick-260512-ecm): detect SECURITY.md sub-phase artifact` (GREEN)
- `e17afe7` — `feat(quick-260512-ecm): render Security row in Plan sub-stages`

## TDD Gate Compliance

- RED commit present: `60aaf8d` (`test(quick-260512-ecm): ...`)
- GREEN commit present: `7de6cd7` (`feat(quick-260512-ecm): detect SECURITY.md ...`)
- No REFACTOR needed — implementation matched the existing pattern exactly.

## Self-Check: PASSED

- src/state_reader/disk_status.rs — FOUND (struct field + detection branch + 3 tests)
- src/ui/screens/detail.rs — FOUND (Security push_substage + plan_touched extension)
- Commit 60aaf8d — FOUND
- Commit 7de6cd7 — FOUND
- Commit e17afe7 — FOUND
