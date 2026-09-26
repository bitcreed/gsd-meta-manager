---
phase: 25-running-agents-live-wave-view
fixed_at: 2026-09-26T00:00:00Z
review_path: .planning/phases/25-running-agents-live-wave-view/25-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 25: Code Review Fix Report

**Fixed at:** 2026-09-26
**Source review:** .planning/phases/25-running-agents-live-wave-view/25-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-02..WR-06; CR-01/WR-01 were already resolved by 25-07; Info findings are out of scope)
- Fixed: 5
- Skipped: 0

Line numbers in REVIEW.md were stale because of later quick tasks (260926-x0v, -06g, -0u3, -16t, -1t1, -2l4, -dyf). Each issue was found again in the current code before it was fixed. None had already been fixed by later work.

## Fixed Issues

### WR-02: A hung git call freezes the agents scan forever

**Files modified:** `src/bounded_output.rs` (new), `src/lib.rs`, `src/state_reader/git_ops.rs`, `src/session_detector.rs`, `tests/bounded_output.rs` (new)
**Commit:** 4728936
**Applied fix:**
- **Shared helper.** The new `bounded_output::stdout_within(&mut Command, budget)` follows the `advisory::run_client` pattern: spawn, poll with `try_wait` (1 ms backoff up to 25 ms), and kill and reap at the deadline.
  - **Pipe drain.** Stdout is drained on a reader thread while the child runs, so the pipe buffer cannot deadlock.
  - **Leftover process.** If a leftover grandchild holds the pipe, the reader is abandoned rather than joined.
  - **Stdin and stderr.** Stdin is null and stderr is discarded; both callers read stdout only.
  - **Spawn guard.** The helper never calls `Command::new`, so the spawn-seam allowlist is unchanged.
- **git.** `git_ops` gains `git_read_raw_within` and the constant `AGENT_GIT_BUDGET = 10s`. A single `git_read_command` builder is shared with `git_read_raw`, so the no-lock flags live in one place. The four agent-scan reads (`worktree_list_porcelain`, `commits_ahead`, `dirty_count`, `log_subjects`) now use the bounded variant.
- **pgrep.** `session_detector::pgrep_exact` is bounded by `PGREP_BUDGET = 2s`. A timeout reads as "could not run" (`None`).
- **Tests.** `tests/bounded_output.rs` covers five cases: a timeout, 1 MiB of stdout drained, status and stdout passed through, a spawn failure, and a leftover process holding stdout.

**Judgement calls:**
- [inferred] The budgets are 10 s for agent-scan git reads and 2 s for pgrep, as in the orchestrator guidance.
- [inferred] Only the agent-scan git helpers are bounded. `git_read_raw` itself is unchanged, because its other callers (envelope hooks, credentials, the status preview) are outside this finding's scope.
- [inferred] No watchdog was added. Every process spawned on the scan path is now bounded: the git reads and the codex `pgrep` in the probe. What remains is plain filesystem reads, which every state reader in the app shares.

### WR-03: The fixer estimate reads `NN-EVAL-REVIEW.md` instead of `NN-REVIEW.md`

**Files modified:** `src/agents/fixers.rs`, `tests/agents_fixers.rs`
**Commit:** 7efe219
**Applied fix:** The new `is_code_review_name(name, phase)` accepts only `<NN>-REVIEW.md`, where `<NN>` is the estimate's phase in any padding (the review's "better version"). `NN-EVAL-REVIEW.md` and `NN-UI-REVIEW.md` are no longer matched.
**Tests:** a pure unit test, plus the integration test `eval_and_ui_reviews_never_supply_the_total`.

### WR-04: Earlier review runs inflate `fixed`, which can exceed `total`, and a leftover REVIEW-FIX.md hides the count

**Files modified:** `src/agents/fixers.rs`, `tests/agents_fixers.rs`
**Commit:** 3a2b838
**Applied fix:**
- **Intersect with the current review.** The REVIEW.md is read once under the existing cap. It yields `findings.total` and the `### (CR|WR|IN)-NN` heading ids. The collected fix ids are intersected with those headings, ignoring padding (`WR-1` matches `WR-01`).
- **Clamp.** `fixed` is clamped with `min(total)`.
- **No REVIEW-FIX early return.** A `*-REVIEW-FIX.md` no longer ends the estimate. The running-fixer gate alone decides whether the run is over.

**Judgement call:** [inferred] If a REVIEW.md has no finding headings, `fixed` falls back to every collected id, still clamped. This covers headings cut off by the cap and frontmatter-only fixtures.

**Test updated:** `a_review_fix_report_hides_the_estimate` expected the old behaviour ("no counts" whenever a REVIEW-FIX.md exists). It is replaced by `a_leftover_review_fix_report_does_not_hide_a_running_estimate`, which expects `~3/48`. Its ids changed from WR-00..02 to WR-01..03.

**New tests:** the integration test `only_the_current_reviews_findings_count_and_fixed_never_exceeds_total`, plus a unit test.

**Remaining limitation:** a re-review that reuses a number (for example WR-01) still matches an earlier run's `fix(NN): WR-01` subject on main. The review's fix bounds this by intersecting and clamping; it does not remove it.

### WR-05: The long-path prefix fallback can adopt another project's Claude directory

**Files modified:** `src/agents/adapters/claude.rs`, `tests/agents_claude.rs`
**Commit:** 6d0b5a3
**Applied fix:** The prefix fallback was removed from `project_dirs`, as the review recommends. Only the exact encoded directories of the three candidate spellings are used.

**Judgement call:** [inferred] The fallback was not load-bearing. No test exercised it, and `encode_project_dir` is pinned byte for byte against Claude Code's algorithm. The 25-02 plan's must-have wording ("with a unique-prefix fallback") is superseded by this fix.

**Test:** `a_long_path_project_never_adopts_a_siblings_claude_directory` fails before the fix, which was confirmed by reverting it, and passes after.

### WR-06: Finished orphan fixers outvote the running fixer and inflate `N fixers` and `fixed`

**Files modified:** `src/agents/fixers.rs`, `tests/agents_fixers.rs`
**Commit:** 6b5e034
**Applied fix:**
- **Tiered vote.** The phase vote now works like `waves::derive`. It runs first over running fixers (`is_running`), and over all fixers only when no running fixer names a phase. Ties still go to the higher phase.
- **Filter.** After the vote, only fixers whose own phase is `None` or the chosen phase are kept. Only they make up `estimate.fixers`, and only their subjects are unioned into `fixed`.

**Judgement call:** [inferred] A running fixer of a different phase is also excluded from the count, following the review's "retain fixers whose own fixer_phase is None or == phase". Its commits could not have counted anyway, because `finding_ids` filters by phase.

**Regression test:** `a_running_fixer_outvotes_finished_orphans_of_another_phase` sets up one Live fixer on phase 13 and three Finished fixers with `fix(12)` commits. It asserts `phase == 13`, `fixers == 1`, `fixed == 1` and `total == 5`. Existing tests did not need changes.

## Verification

All gates ran in the **main checkout** at `/home/blk/projects/rust/gsd-meta-manager`, on branch `master`. No worktree was used, because isolation was forced to none. The results can be reproduced from this tree.

- **Tests.** `rtk proxy cargo test --no-fail-fast` gave 2665 passed and 1 failed across 56 suites. The single failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known git-version witness that is expected to fail on this machine.
- **Clippy.** `cargo clippy -- -D warnings` is clean. `--all-targets` still reports only the 11 pre-existing errors, in `src/browser.rs`, `src/project_creator.rs` and `tests/envelope_*`. None of them are in touched files.
- **Formatting.** The project is not fmt-clean as a whole, so formatting was scoped:
  - `rustfmt` was run on the new files and on the files that were fmt-clean at HEAD: `src/agents/fixers.rs`, `src/agents/adapters/claude.rs` and `tests/agents_claude.rs`.
  - For the files that were already dirty (`git_ops.rs`, `session_detector.rs`, `tests/agents_fixers.rs`), only the new hunks were formatted by hand. The pre-existing drift was left alone.

---

_Fixed: 2026-09-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
