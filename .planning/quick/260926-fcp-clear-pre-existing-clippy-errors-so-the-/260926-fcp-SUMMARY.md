---
phase: quick-260926-fcp
plan: 01
subsystem: tests / lint hygiene
tags: [clippy, lint, tests, no-behavior-change]
status: complete
requires: []
provides:
  - "`cargo clippy --all-targets -- -D warnings` exits 0"
affects: []
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - src/browser.rs
    - src/project_creator.rs
    - tests/envelope_config_resolution.rs
    - tests/envelope_control_carrier.rs
    - tests/envelope_carrier_reach.rs
    - tests/envelope_wrapper_class.rs
decisions:
  - "[inferred] single_element_loop rewritten to a block binding, not #[allow]ed (per plan)"
  - "[inferred] commit prefix fix(quick-260926-fcp) per orchestrator constraint, overriding the plan's chore(...)"
metrics:
  completed: 2026-09-26
  tasks: 2
  files: 6
plan_head_before: 7a7c050
actuals:
  tasks: 2
  commits: 1
requirements: [QUICK-260926-fcp]
---

# Quick 260926-fcp: Clear pre-existing clippy --all-targets errors Summary

I rewrote 12 lint sites in 6 files so that `cargo clippy --all-targets -- -D warnings` now exits 0. The release gate `cargo clippy -- -D warnings` still exits 0. No `#[allow]` was added and behavior is unchanged.

## Commit

- `56f5583` fix(quick-260926-fcp): clear clippy --all-targets lints (no behavior change). The commit is on master and was not pushed. Diff: 6 files, +13/-15.

## Lints fixed (12 sites, 9 lint classes)

| File | Lint | Rewrite |
|------|------|---------|
| src/browser.rs:156-158 | bool_assert_comparison x3 | `assert_eq!(x, true/false)` becomes `assert!(x)` / `assert!(!x)` |
| src/project_creator.rs:146 | cmp_owned | `result != PathBuf::from("~")` becomes `result != Path::new("~")`. `Path` was already in scope via `use super::*` |
| tests/envelope_config_resolution.rs:2206 | bool_assert_comparison | `assert_eq!(helper(..).0, false, MSG)` becomes `assert!(!helper(..).0, MSG)`. The message is byte-identical |
| tests/envelope_control_carrier.rs:978 | single_element_loop | `for name in ["shred"] {..}` becomes `{ let name = "shred"; .. }`. The assertion and message are unchanged |
| tests/envelope_carrier_reach.rs:1711 | useless_format | `format!("{ALIAS}")` becomes `ALIAS.to_string()`. The element type stays `String` |
| tests/envelope_wrapper_class.rs:6127 | unused_mut | dropped `mut` on the `push` closure binding |
| tests/envelope_wrapper_class.rs:6213 | manual_ignore_case_cmp | `.to_ascii_lowercase() == "alias"` becomes `.eq_ignore_ascii_case("alias")`. ASCII semantics are the same |
| tests/envelope_wrapper_class.rs:10795 | chars_last_cmp | `chars().next_back() != Some('=')` becomes `!ends_with('=')` |
| tests/envelope_wrapper_class.rs:11232 | manual_contains | `.iter().any(\|e\| *e == "tar ...")` becomes `.contains(&"tar ...")` |

All sites were cleared in round 1. The round-2 clippy run surfaced no new lints, so Task 2 had no further sites to fix.

## Lints allowed

None. No `#[allow(...)]` was added anywhere.

## Verification

- `rtk proxy cargo clippy -- -D warnings` (release gate): exit 0 (CLIPPY_GATE_OK)
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0 (CLIPPY_ALL_OK). It was 101 before the change.
- `rtk proxy cargo test --no-fail-fast`: 56 suites, **2665 passed, 1 failed, 15 ignored**. The single failure is the known environmental git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, because local git differs from the pinned 2.55.0. Every envelope suite ran, including envelope_config_resolution (30), envelope_control_carrier (37), envelope_carrier_reach (39) and envelope_wrapper_class (60). No test was added or removed, so the pass count cannot have dropped.
- The diff touches no production logic under `src/envelope/`.

## Inferred decisions (for audit)

1. **[inferred] single_element_loop is rewritten, not allowed.** The loop pins the one program name (`shred`) that the carrier-rule test uses. It is not a list meant to grow. This follows the plan's recorded decision.
2. **[inferred] Commit type is `fix(...)`, not the plan's `chore(...)`.** The orchestrator's constraint asked for the `fix(quick-260926-fcp): ...` prefix, and that constraint is more recent than the plan text.
3. **[inferred] Pre-change test baseline was not re-run.** The edits neither add nor remove tests, so the test count is unchanged by construction. The post-change run shows only the known witness failure.

## Deviations from Plan

The only change from the plan is the commit prefix described in decision 2. Everything else was executed as written.

## Self-Check: PASSED

- All 6 modified files exist and are in commit 56f5583 (verified with `git log`).
