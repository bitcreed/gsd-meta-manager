---
phase: quick-261028-v2z
plan: 01
subsystem: launcher
tags: [security, managed-settings, review-fix, WR-13]
status: complete
requires: [quick-261028-t3f]
provides:
  - well_formed_hooks / well_formed_permissions shape filters in both settings postures
key-files:
  modified:
    - src/launcher/config_dir.rs
    - tests/launcher_config_dir.rs
    - docs/DRIVER-SANDBOX.md
    - .planning/phases/30-driver-tab-on-direction-md/30-REVIEW-DISPOSITION.md
decisions:
  - "I-1..I-7 from the plan applied as written: drop and normalize, never refuse; string/bool permissions settings kept; same filter in the degraded posture [inferred — cleanup 2026-10-28]"
metrics:
  tasks: 3
  commits: 5
  plan_head_before: 79ab42d85c6b2d2e2329561781670671f033af4c
  plan_head_after: 40870ca01c73cdcce8bf5a353bf6cc93ab118381
actuals:
  tasks: 3
  commits: 5
commits: 5
plan_head_before: 79ab42d85c6b2d2e2329561781670671f033af4c
plan_head_after: 40870ca01c73cdcce8bf5a353bf6cc93ab118381
completed: 2026-10-28
---

# Quick 261028-v2z: Fix WR-13 (managed_settings nested wrong-shape host values) Summary

`managed_settings` and `settings_transform` now merge only the well-formed part of a host `hooks` / `permissions` object (event arrays, well-formed matcher entries, string rule items, string/bool permission settings), so a wrong-shape nested value can never sit beside the gate hooks or the DENY_TOOLS deny list.

## Commits

| Sha | Subject |
|-----|---------|
| 601d20f | test: pin that a wrong-shape nested host hooks or permissions value never reaches the managed seed (RED; seed test failed at HEAD) |
| 1158139 | fix: drop wrong-shape nested host hooks and permissions values from the managed settings |
| 92dac7a | test: pin every wrong-shape nested hooks and permissions value in both postures (degraded table RED) |
| bfbd231 | fix: drop wrong-shape nested host hooks and permissions values from the degraded settings.json |
| 40870ca | docs: record 30 WR-13 as fixed |

## Tests added

- `tests/launcher_config_dir.rs::a_managed_seed_drops_wrong_shape_nested_host_values_and_keeps_the_gate`
- `launcher::config_dir::tests::a_wrong_shape_nested_host_value_never_reaches_the_managed_file`
- `launcher::config_dir::tests::a_wrong_shape_nested_host_value_never_reaches_the_degraded_settings`
- `launcher::config_dir::tests::well_formed_host_values_still_merge_beside_wrong_shape_ones`
- Inverted: `a_non_array_host_deny_list_does_not_replace_the_launchers` now asserts `ask` is dropped (I-5).

## Gate

`rtk proxy cargo test --no-fail-fast`: 3211 passed, 0 failed, 17 ignored (raw totals summed over all `test result:` lines). `cargo clippy --all-targets -- -D warnings` clean. `cargo fmt --check` clean. Cargo.toml / Cargo.lock unchanged.

## Deviations from Plan

None - plan executed as written. Notes: the base HEAD was 79ab42d (plan baseline 44f1291, only docs commits between). Execution ran on master per the sequential-execution instruction. The 28 and 29 disposition files do not list WR-13 and are unchanged. The Task 1 and Task 2 `<verify>` shell snippets were not run verbatim; their equivalent checks (named tests pass, copy arms gone count 0, `passed through, as before` count 0, WR-13 cited, `wrong shape` in the doc) were run directly.

## Known Stubs

None.

## Self-Check: PASSED

All five commits are ancestors of HEAD; modified files exist.
