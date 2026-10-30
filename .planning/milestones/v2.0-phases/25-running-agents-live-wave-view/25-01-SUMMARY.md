---
phase: 25-running-agents-live-wave-view
plan: 01
subsystem: agents
tags: [git-worktree, porcelain, adapter-seam, liveness, catch_unwind, non-intrusion]

requires:
  - phase: 19-gitsafe
    provides: "git_ops::git_read_raw (--no-optional-locks wrapper)"
  - phase: 21-llm-goal-layer
    provides: "text::Untrusted carrier"
provides:
  - "src/agents core: AgentLiveness, LIVE_SECS/IDLE_SECS/MAX_AGENT_AGE_SECS, classify_liveness, AgentRow, ProjectAgents, scan_project_with, scan_projects_guarded"
  - "src/agents/worktrees.rs: RawWorktree, CoreWorktree, CoreScan, BranchPlan, parse_porcelain_z, parse_porcelain_lines, is_agent_worktree, parse_agent_branch, valid_agent_id, scan_worktrees"
  - "src/agents/adapters/mod.rs: AgentAdapter, CoreSnapshot, Enrichment, ChildAgent, AdapterReport, registered_adapters (empty)"
  - "git_ops: GIT_OPTIONAL_LOCKS=0 in git_read_raw; worktree_list_porcelain, PorcelainListing, commits_ahead, dirty_count, is_full_hex_sha"
  - "AGENT-01..AGENT-07 in REQUIREMENTS.md; ROADMAP Phase 25 requirements line"
affects: [25-02, 25-03, 25-04, 25-05, 25-06]

actuals:
  tokens: 20900
  tasks: 3
  commits: 5
plan_head_before: 8b92938e633e0f7337b30a6126fd7eede5b26aff

tech-stack:
  added: []
  patterns:
    - "Adapters report facts only; the core classifies liveness once (classify_liveness)"
    - "Every agent git read goes through git_read_raw (flag + env var)"
    - "catch_unwind per adapter and per project; panic -> empty report + tracing::warn"
    - "Runtime zero-write/spawn/proc guard walks src/agents at test time, tokens split in halves"

key-files:
  created:
    - src/agents/mod.rs
    - src/agents/worktrees.rs
    - src/agents/adapters/mod.rs
    - tests/agents_scan.rs
  modified:
    - src/state_reader/git_ops.rs
    - src/lib.rs
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "Git count calls live in worktrees::worktree_counts (pub(crate)), called from scan_project_with, so every git call in src/agents stays in worktrees.rs [inferred]"
  - "The no-linked-worktree prefilter reports main_worktree = project_root and base_sha = None; base_sha None is the observable proof the listing was skipped [inferred]"
  - "parse_porcelain_lines also undoes git's three-digit octal byte escapes (non-ASCII paths under core.quotePath), beyond the plan's four escapes [inferred]"
  - "is_agent_worktree and the agent-id path rule are tried against both the registered root and git's canonical main-worktree path [inferred]"
  - "registered_adapters() itself runs inside catch_unwind in scan_projects_guarded; adapter name() calls are guarded too [inferred]"

patterns-established:
  - "Test-only adapter through scan_project_with is the D-A07 proof shape later adapters reuse"
  - "Fixture steps after a successful git init assert, so a degraded fixture cannot make a test pass vacuously"

requirements-completed: [AGENT-01, AGENT-03]

coverage:
  - id: D1
    description: "Agent worktrees enumerate (path under .claude/worktrees/, agent branch families, or adapter claim) with commits ahead of main HEAD and a dirty count; main and plain user worktrees excluded"
    requirement: AGENT-01
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#one_agent_worktree_is_found_with_its_commit_and_dirty_counts"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#the_main_worktree_and_a_plain_user_worktree_are_not_agent_rows"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#an_agent_worktree_just_spawned_reads_zero_commits_and_zero_dirty"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#commits_ahead_refuses_a_base_that_is_not_a_full_hex_sha"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#dirty_count_counts_modified_and_untracked_files"
        status: pass
    human_judgment: false
  - id: D2
    description: "The scan never refreshes or locks a live agent worktree's index (observed red with both no-lock lines removed)"
    requirement: AGENT-01
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#scanning_a_live_agent_worktree_never_touches_its_index"
        status: pass
    human_judgment: false
  - id: D3
    description: "Porcelain -z and newline parsing, D-A06 branch grammar, agent-id validation, agent predicate, ledger plan, deleted/prunable and non-git degradation"
    requirement: AGENT-01
    verification:
      - kind: unit
        ref: "src/agents/worktrees.rs#tests (7 tests)"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_codex_style_branch_yields_its_plan_and_spawn_time"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_worktree_deleted_from_disk_keeps_its_row_with_unknown_counts"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_ledger_file_confirms_the_plan"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_non_git_project_scans_to_no_worktree_rows"
        status: pass
    human_judgment: false
  - id: D4
    description: "Adapter seam: facts-only trait, test-only adapter claims outside the predicate with zero core edits, first claim wins, out-of-range ignored, panicking adapter leaves rows intact, guarded projects scan sorted and isolated"
    requirement: AGENT-03
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#a_test_only_adapter_claims_a_worktree_outside_the_predicate"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#the_first_adapter_to_claim_a_worktree_wins"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_panicking_adapter_leaves_the_core_rows_intact"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#a_projects_scan_is_sorted_by_alias_and_isolates_failures"
        status: pass
    human_judgment: false
  - id: D5
    description: "Liveness thresholds (120/600s) classified in one place with pinned boundaries"
    requirement: AGENT-03
    verification:
      - kind: unit
        ref: "src/agents/mod.rs#tests (5 classify tests)"
        status: pass
    human_judgment: false
  - id: D6
    description: "src/agents is mechanically write-free, /proc-free and spawn-free (observed red on a planted fs::write); spawn allowlist unchanged"
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#no_file_under_src_agents_writes_reads_proc_or_spawns"
        status: pass
      - kind: integration
        ref: "cargo test --test spawn_seam_guard (41 passed)"
        status: pass
    human_judgment: false
  - id: D7
    description: "AGENT-01..07 recorded in REQUIREMENTS.md (bullets + traceability) and ROADMAP Phase 25"
    verification:
      - kind: other
        ref: "git grep -c 'AGENT-0' -- .planning/REQUIREMENTS.md == 14; ROADMAP requirements line present, no TBD"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 01: Agent Observer Core and Adapter Seam Summary

**Runtime-agnostic agent-worktree scan over lock-free git (`--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0`), a facts-only `Box<dyn AgentAdapter>` seam with `catch_unwind` per adapter and per project, and liveness classified once at 120s/600s. A real-worktree test proves the scan leaves a live agent's index untouched.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-26T02:30:19Z
- **Completed:** 2026-09-26T02:45:26Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 8

## Accomplishments

- `git_read_raw` now sets `GIT_OPTIONAL_LOCKS=0` alongside `--no-optional-locks`. `worktree_list_porcelain` (tries `-z`, falls back to the newline form), `commits_ahead` (full lowercase hex base only) and `dirty_count` all go through it.
- `src/agents/worktrees.rs` covers porcelain parsing in both forms (including C-unquoting), the D-C03 predicate widened to GSD's branch regex, the D-A06 `agent-p{plan}-{ts}` grammar, `valid_agent_id`, the ledger plan read, a no-linked-worktree prefilter, and degraded rows (prunable, non-git).
- `src/agents/adapters/mod.rs` defines the trait and types exactly as in `<interfaces>`. `registered_adapters()` is empty for now and names where Claude (25-02) and Codex (deferred) register.
- `src/agents/mod.rs` has the zero-write doctrine, the `AgentLiveness` enum, the const block, `classify_liveness`, `scan_project_with` (first claim wins, out-of-range indexes ignored, only live worktree-less agents kept) and `scan_projects_guarded`.
- Two red-when-broken controls were observed: removing both no-lock lines from `git_read_raw` made `scanning_a_live_agent_worktree_never_touches_its_index` fail with "the scan rewrote the index", and a planted `std::fs::write` in `worktrees.rs` made the zero-write guard fail and name the file and line. Both changes were reverted before any commit.
- AGENT-01..07 were added to REQUIREMENTS.md and ROADMAP Phase 25.

## Task Commits

1. **Task 1: tracer, one agent worktree end to end:** `13bda07` (feat). Tracer gate: automated verify re-run and passed, then expanded.
2. **Task 2: porcelain edge cases, grammar, ledger, degraded rows, non-intrusion:** `c93db16` (test, RED) → `ed562b3` (feat, GREEN)
3. **Task 3: liveness, seam hardening, zero-write guard, requirement ids:** `9048876` (test, RED) → `21e3ee1` (feat, GREEN)

No REFACTOR commits were needed.

## Files Created/Modified

- `src/agents/mod.rs`: doctrine, liveness, rows, `scan_project_with`, `scan_projects_guarded`, 5 classify tests
- `src/agents/worktrees.rs`: git-only core, parsers, predicate, grammar, ledger, prefilter, 7 pure tests
- `src/agents/adapters/mod.rs`: the adapter seam and the "Adding a runtime" doc
- `src/state_reader/git_ops.rs`: env var in `git_read_raw`, 3 helpers plus `is_full_hex_sha`, 2 tests
- `src/lib.rs`: `pub mod agents;`
- `tests/agents_scan.rs`: 13 integration tests (tracer, non-intrusion proof, seam, zero-write guard)
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`: AGENT ids

## Test Results

- `cargo test --test agents_scan --no-fail-fast`: 13 passed.
- `cargo test --lib agents::`: 12 passed.
- git_ops: 21 passed, including the 2 new tests.
- `spawn_seam_guard`: 41 passed. `tests/spawn_seam_guard.rs` is unchanged since the plan base.
- `cargo test --no-fail-fast`: 50 suites, 2393 passed, 1 failed, 15 ignored. The one failure is the known local git-version witness (`envelope::policy::...the_git_version_they_were_derived_against`).
- `cargo clippy -- -D warnings`: clean. `cargo clippy --all-targets` has no warnings in any file this plan touched (see deferred-items.md for the others).

## TDD Gate Compliance

- The RED and GREEN commits exist for both TDD tasks (`test(25-01)` then `feat(25-01)`).
- **Task 2 RED targets:** `porcelain_lines_fallback_unquotes_c_style_paths`, `a_ledger_file_confirms_the_plan` and `a_non_git_project_scans_to_no_worktree_rows` (prefilter half). Each failed on its behavior assertion.
- **Task 3 RED target:** `a_projects_scan_is_sorted_by_alias_and_isolates_failures`, against a stub `scan_projects_guarded` that returned an empty Vec. It failed on the sorted-aliases assertion.
- **Tests that were GREEN at RED time:** the other tests in both RED commits already passed, because the Task 1 tracer had delivered those behaviors (parsers, grammar, claims, `catch_unwind`, `classify_liveness`). This is expected when TDD tasks follow a tracer, not a defect.
- **`gsd-tools check tdd-red-evidence`:** it returned `INVALID_RED / zero_tests_discovered` for both RED records. The checker parses only node TAP output and cannot read libtest's format; cargo's own output shows the named target failing on an assertion. `workflow.tdd_mode` is not enabled, so this gate is advisory. [inferred: proceeded on cargo's output]

## Decisions Made

See `key-decisions` in the frontmatter. Every entry is marked [inferred] for audit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Octal byte escapes in the non-`-z` porcelain fallback**
- **Found during:** Task 2
- **Issue:** Under the default `core.quotePath`, git spells non-ASCII path bytes as `\NNN` octal. With only the plan's four escapes, every non-English path would have been kept as a raw quoted string, which breaks the portability constraint.
- **Fix:** `try_unquote_c_style` also decodes three-digit octal bytes and requires valid UTF-8. Any other escape still keeps the raw string.
- **Files modified:** src/agents/worktrees.rs
- **Verification:** the `/tmp/caf\303\251` case in `porcelain_lines_fallback_unquotes_c_style_paths`
- **Committed in:** c93db16 / ed562b3

**2. [Rule 2 - Missing critical] Fixture steps after `git init` assert instead of returning `None`**
- **Found during:** Task 1
- **Issue:** When the plan's `agents_fixture` returns `None`, every test silently skips. A failure in `worktree add` would have made the whole suite pass without testing anything.
- **Fix:** only a failed `git init` skips; every later step asserts.
- **Files modified:** tests/agents_scan.rs
- **Committed in:** 13bda07

**3. [Rule 2] `worktree_counts` helper and `is_full_hex_sha` added as `pub(crate)`**
- These keep every git call in `src/agents/` inside `worktrees.rs`, and let one hex rule serve both the base and `commits_ahead`. No public contract changed.

---

**Total deviations:** 3 auto-fixed (all Rule 2). **Impact:** each one closes a portability or vacuous-pass gap. The public names and fields in `<interfaces>` are exactly as written.

## Issues Encountered

- `gsd-tools windows append` refused to record the intentional stub, because `.planning/WINDOWS.md` was already inconsistent (row 19: the table disagrees with the JSON). This is logged in deferred-items.md.

## Known Stubs

| File | Line | Stub | Reason / resolution |
|------|------|------|---------------------|
| src/agents/adapters/mod.rs | 107 | `registered_adapters()` returns `Vec::new()` | Intentional per plan. 25-02 registers `ClaudeCodeAdapter` (one `pub mod claude;` line plus one registry line). Not wired to the UI until 25-04. |

## Threat Flags

None. No new surface beyond the plan's threat register: T-25-01..07 are mitigated as planned, and T-25-07 rendering lands in 25-05.

## User Setup Required

None.

## Next Phase Readiness

- The 25-02 Claude adapter and the 25-03 wave model can build in parallel against the contracts that now exist verbatim. 25-02 touches only `src/agents/adapters/claude.rs` plus two lines in `adapters/mod.rs`, and 25-03 touches `src/agents/waves.rs`.
- Audit item from the plan's flagged assumptions: an adapter that claims every worktree is currently accepted.

## Self-Check: PASSED

- All 4 created files are present on disk.
- Commits 13bda07, c93db16, ed562b3, 9048876 and 21e3ee1 are all present.
- `git rev-list --count 8b92938..HEAD` = 5.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*
