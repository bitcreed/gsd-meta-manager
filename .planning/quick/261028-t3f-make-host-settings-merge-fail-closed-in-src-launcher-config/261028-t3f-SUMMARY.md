---
quick_id: 261028-t3f
phase: quick-261028-t3f
plan: 01
status: complete
commits: 4
plan_head_before: b407eb7
plan_head_after: 5dfbeb6
completed: 2026-10-28
key-files:
  modified:
    - src/launcher/config_dir.rs
    - tests/launcher_config_dir.rs
    - docs/DRIVER-SANDBOX.md
    - .planning/phases/28-sandboxed-driver-launcher/28-REVIEW-DISPOSITION.md
    - .planning/phases/29-master-prompt-run-templates/29-REVIEW-DISPOSITION.md
---

# Quick 261028-t3f: host managed-settings merge fails closed

`managed_settings` now ignores a host `hooks` or `permissions` value that is not a JSON object, so the gate hooks and the `DENY_TOOLS` deny list survive any host shape. This fixes 28-REVIEW WR-08 and 29-REVIEW WR-02, which are the same defect.

## Commits

- daa13d1 test: RED bwrap seed test from host files on disk (failed with `hooks` = null before the fix)
- 6a55713 fix: `("hooks" | "permissions", _) => {}` arm before the catch-all, plus the corrected claims in the module doc, the `managed_settings` doc and docs/DRIVER-SANDBOX.md
- a2da9bb test: six-shape tables (string, empty array, non-empty array, null, number, bool) for both keys in the managed and degraded postures, plus the mixed-docs merge case
- 5dfbeb6 docs: 28 WR-08 and 29 WR-02 set to fixed (open 4 -> 3 and 6 -> 5; totals 16 and 12 unchanged)

## Verification

- `cargo test --no-fail-fast`: 3207 passed, 0 failed, 17 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean, also for x86_64-apple-darwin and aarch64-apple-darwin.
- `cargo fmt --check`: clean. Cargo.toml and Cargo.lock unchanged.

## Deviations from Plan

None. The degraded-posture test passed at once (I-3), so `settings_transform` needed no change. A `cargo fmt` run reformatted only the new test code before the third commit.

## Inferred decisions

- I-1 to I-5 from the plan stand `[inferred — cleanup 2026-10-28]`: ignore (not refuse) a non-object value, six-shape table, degraded posture pinned only, disposition Source cells follow the n0y precedent, doc claims corrected in the same change.
- `[inferred — cleanup 2026-10-28]` The disposition Source cells cite the fix commit 6a55713, as the plan directs.
- `[inferred — cleanup 2026-10-28]` `commits:` is counted from b407eb7 (the HEAD named in the task), since no plan-head ledger file was written.

## Self-Check: PASSED
