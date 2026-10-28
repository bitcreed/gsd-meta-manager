---
quick_id: 261028-n0x
status: complete
commits: 3
plan_head_before: 30dcf32
plan_head_after: e573c7d
actuals:
  tasks: 3
  commits: 3
---

# Quick 261028-n0x: Phase 19 surviving pin gaps (T-19-72, T-19-73) Summary

Two test-only pins close the last two open Phase 19 threat ids; the register now reads 139 total / 139 closed / 0 open.

- Lib baseline before Task 1: 2528 passed, 1 ignored. After: 2530 passed (+2).
- S72 = `dd1669e` (scan.rs test), S73 = `25a4721` (config.rs test), docs = `e573c7d`.
- Only `#[cfg(test)]` lines added in both files (51 and 53 insertions, 0 deletions). `git diff --quiet` on both files exits 0 after the mutations.

## Mutation evidence

- M1 (`is_clean` OR `external == Absent`): `envelope::scan::tests::an_absent_external_scanner_never_changes_the_verdict ... FAILED` — "worktree with a planted key: an absent external scanner moved the verdict".
- M2 (`is_clean` AND `external != Absent`): same test FAILED — "clean worktree: an absent external scanner moved the verdict".
- M3 (bare `Literal { token }` variant): `error[E0004]: non-exhaustive patterns: &config::CredentialSource::Literal { .. } not covered` at src/config.rs:956, "could not compile (lib test)".
- M4 (variant plus witness arm): `config::tests::a_credential_source_has_no_literal_token_variant ... FAILED` — "a literal-token shape parsed: {"source":"literal","token":"x"}: Literal { token: "x" }".

## Deviations from Plan

None. Note: the plan's "2529 tests at HEAD" is the total count; the passed count measured was 2528 (1 ignored), and the +2 check is against that.

[inferred — cleanup 2026-10-28] The todo went to `.planning/todos/completed/` per the plan.

## Self-Check: PASSED
