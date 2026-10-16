---
phase: quick-261016-dx5
plan: 01
subsystem: dashboard-status-classification
tags: [classify_status, gsd-core, verifying]
key-files:
  modified: [src/app.rs]
status: complete
bug_confirmed: true
commits: 2
plan_head_before: 21085da
plan_head_after: e45c571b8eb2318b7887b61accb37fd707e9a266
commit_shas: [e9325ce, e45c571]
actuals:
  tasks: 2
  commits: 2
---

# Quick 261016-dx5: classify_status awaiting-verification fix

One new `else if` branch in `classify_status` (src/app.rs) makes both the raw gsd-core wording
`Phase complete — ready for verification` and the normalized frontmatter token `verifying` classify as
`StatusCategory::Idle` (previously Complete and Unknown respectively).

## Bug confirmed: YES

- RED 1 (Task 1): `classify_status("Phase complete — ready for verification")` failed with `left: Complete`
  (src/app.rs `contains("complete")` branch).
- RED 2 (Task 2): `classify_status("verifying")` failed with `left: Unknown`.
- Branch order: Active, Blocked, awaiting-verification, Complete, Idle-words.

## Commits

- e9325ce: Task 1 (tracer): `ready for verification` branch + test (RED to GREEN)
- e45c571: Task 2: `verifying` match, dash variant, blocked precedence, `Ready to execute` pin

## Tests

- `cargo test --lib classify_status`: 5 passed, 0 failed.
- Full `cargo test --no-fail-fast`: lib 2106 passed, 1 failed (expected: `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`), 1 ignored; every other test target ok.
- `cargo fmt --check`: clean. `cargo clippy --all-targets -- -D warnings`: clean.

## Planning evidence (from PLAN)

- `ProjectState.status` is the STATE.md frontmatter `status:` (state_md.rs:683); the body `Status:` is not read.
- gsd-core fde7d4e80 `nextPhaseWording` writes the em-dash wording; gsd-core 1.17.0 `normalizeStateStatus` maps it to
  `verifying` (`STATUS_EXACT_TOKENS`); unrecognized prose/hyphen variants pass through verbatim.
- Real data: `~/projects/flutter/nomosquitoz` has `status: verifying` (showed Unknown before the fix); `~/projects/web/predix` likewise.

## Inferred decisions (audit)

- I-1: bug is real (confirmed by both RED observations); fix both forms.
- I-2: category is `Idle` (no new variant), consistent with `ready to plan` / `All phases complete`.
- I-3: precedence Active, Blocked, awaiting-verification, Complete, Idle-words.
- I-4: substrings `ready for verification` and `verifying` (lowercased); `verified`, `discussing`, `paused` out of scope (musk M-0001).
- I-5: `Ready to execute` stays Idle, pinned by `test_classify_status_ready_to_execute_is_idle`; normalized `executing` still reads Active (accepted divergence).
- I-6: inline branch only, no new predicate in state_md.rs; comment at the `/gsd:complete-milestone` line left untouched (dx4 owns it).
- Executor note: the Task 1 commit held only the `ready for verification` branch (per task split); Task 2 extended it.

## Deviations from Plan

None. Plan executed as written.

## Self-Check: PASSED

src/app.rs modified; commits e9325ce and e45c571 present on master.
