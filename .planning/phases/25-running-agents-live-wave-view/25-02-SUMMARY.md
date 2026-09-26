---
phase: 25-running-agents-live-wave-view
plan: 02
subsystem: agents
tags: [claude-code, adapter, session-metadata, liveness, tolerant-parsing, utf16-encoding]

requires:
  - phase: 25-01
    provides: "AgentAdapter seam (CoreSnapshot, Enrichment, ChildAgent, AdapterReport, registered_adapters), CoreWorktree, valid_agent_id, LIVE_SECS/MAX_AGENT_AGE_SECS, scan_project_with, classify_liveness"
provides:
  - "src/agents/adapters/claude.rs: ClaudeCodeAdapter::{new, from_env}, resolve_config_root, encode_project_dir, META_READ_CAP, read_meta_capped, tolerant Meta"
  - "registered_adapters() now returns the Claude Code adapter (first real runtime behind the seam)"
  - "Per-id worktree enrichment (agentType, description, transcript mtime, ended, lock_released), id-less path joins, nested children, live worktree-less subagents"
affects: [25-04, 25-05]

actuals:
  tokens: 15100
  tasks: 3
  commits: 5
plan_head_before: 9cc7048b18399f6cd849fe96ad7ae0ec968778b8

tech-stack:
  added: []
  patterns:
    - "One module plus registration in adapters/mod.rs is the whole cost of a runtime (D-A07 demonstrated)"
    - "Metas extracted field by field from serde_json::Value; unparseable meta = no data this scan"
    - "Transcripts statted, never opened; the only content read goes through read_meta_capped"
    - "Encoding vectors pinned from the runtime's own JS under node, not from the implementation"

key-files:
  created:
    - src/agents/adapters/claude.rs
    - tests/agents_claude.rs
  modified:
    - src/agents/adapters/mod.rs

key-decisions:
  - "An unparseable meta (garbage, truncated, over the cap) is no data for that row, and the transcript mtime is not reported either. A parsed meta with one mistyped field loses only that field [inferred]"
  - "read_meta_capped also refuses a symlink or a non-regular file: a FIFO would block the scan, and a symlink could point a .meta.json at a transcript [inferred]"
  - "A live meta whose worktreePath names a worktree that already has its own agent attaches there as a child. It is not reported as worktree-less [inferred]"
  - "The MAX_AGENT_AGE_SECS session bound covers the whole second pass (children, id-less path joins, worktree-less). Only the per-id worktree lookup is unbounded [inferred]"
  - "When several sessions hold the same agent id, the id pass takes the one with the newest transcript. The second pass handles each id once, in sorted session-then-id order [inferred]"
  - "Registration replaces Vec::new() with a vec![] literal holding the Claude entry, and the Codex comment stays inside it. So mod.rs is +6/-3 lines rather than exactly two [inferred]"

patterns-established:
  - "Adapter unit tests build CoreSnapshot by hand over tempdir paths (no git). Integration tests use a real worktree plus a fake config root"
  - "Red-when-broken control for a bound: disable it, observe the test fail, restore before committing"

requirements-completed: [AGENT-04, AGENT-03]

coverage:
  - id: D1
    description: "A live Claude executor's worktree row carries agentType, description and Live state, found through a tempdir config root, with the adapter registered in adapters/mod.rs"
    requirement: AGENT-04
    verification:
      - kind: integration
        ref: "tests/agents_claude.rs#a_live_claude_executor_is_enriched_end_to_end"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#registered_adapters_includes_claude_code"
        status: pass
    human_judgment: false
  - id: D2
    description: "encode_project_dir matches Claude Code 2.1.283 byte for byte (ASCII, dots/spaces, non-BMP, >200 with negative and positive hash, i32::MIN), and the config root rule ignores a relative or empty CLAUDE_CONFIG_DIR"
    requirement: AGENT-04
    verification:
      - kind: unit
        ref: "src/agents/adapters/claude.rs#encoding_matches_claude_code_for_ascii_dots_and_spaces"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#encoding_emits_two_dashes_for_a_non_bmp_char"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#encoding_over_200_units_appends_the_base36_hash"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_relative_or_empty_config_dir_falls_back_to_home"
        status: pass
    human_judgment: false
  - id: D3
    description: "Malformed, oversized or missing metadata degrades only the affected row or field. Transcripts are never opened, and unsafe agent ids never reach a path"
    requirement: AGENT-03
    verification:
      - kind: unit
        ref: "src/agents/adapters/claude.rs#garbage_truncated_and_mistyped_metas_degrade_field_by_field"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_meta_larger_than_the_cap_is_no_data"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#the_meta_reader_refuses_a_transcript_path"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#an_unsafe_agent_id_never_reaches_the_filesystem"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_worktree_without_metadata_degrades_to_the_core_row"
        status: pass
    human_judgment: false
  - id: D4
    description: "Joins by worktreePath (raw, then canonical) or agent-id stem; nested agents attach as children by inheritedWorktreePath or parentAgentId"
    requirement: AGENT-04
    verification:
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_symlinked_worktree_path_still_joins"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_meta_naming_another_worktree_is_rejected"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_nested_agent_attaches_to_its_worktree_as_a_child"
        status: pass
      - kind: unit
        ref: "src/agents/adapters/claude.rs#a_parent_agent_id_attaches_a_child_without_an_inherited_path"
        status: pass
    human_judgment: false
  - id: D5
    description: "Lifecycle facts classify correctly: a released lock plus a stale transcript reads Finished; a held lock naming a live pid plus a 601 s transcript reads Stalled; stoppedByUser reads Ended"
    requirement: AGENT-04
    verification:
      - kind: integration
        ref: "tests/agents_claude.rs#a_released_lock_with_a_stale_transcript_reads_finished"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_locked_worktree_with_a_stale_transcript_reads_stalled_even_with_a_live_pid"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_stopped_by_user_meta_reads_ended"
        status: pass
    human_judgment: false
  - id: D6
    description: "Live worktree-less subagents are listed verbatim while live. A long-running agent is not hidden by the spawn-clock mtime. A day-old session is skipped for the worktree-less pass only (bound control observed red)"
    requirement: AGENT-04
    verification:
      - kind: integration
        ref: "tests/agents_claude.rs#a_live_worktree_less_subagent_is_listed_and_a_stale_one_is_not"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#agent_type_is_shown_verbatim_without_a_gsd_filter"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_long_running_agent_in_an_old_subagents_dir_is_still_live"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_day_old_session_is_skipped_for_worktree_less_agents_only"
        status: pass
    human_judgment: false
  - id: D7
    description: "src/agents stays write-, /proc- and spawn-free with claude.rs present; the spawn allowlist is unchanged; no strict unknown-field rejection"
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#no_file_under_src_agents_writes_reads_proc_or_spawns"
        status: pass
      - kind: integration
        ref: "cargo test --test spawn_seam_guard (41 passed)"
        status: pass
      - kind: other
        ref: "git grep -n 'deny_unknown_fields' -- src/agents (no output)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Against the real ~/.claude of a live Claude Code install, the adapter finds the right project directory and the encoding still matches the installed version"
    requirement: AGENT-04
    verification: []
    human_judgment: true
    rationale: "Every test uses a tempdir config root by design (D-C17). The real-install check waits for the dashboard wiring in 25-04/25-05, and the Claude Code version may drift (Assumption A2)"

duration: 11min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 02: Claude Code Adapter Summary

**The Claude Code adapter reads `<config>/projects/<encoded path>/<session>/subagents/agent-<id>.meta.json` (at most 64 KiB, field by field) and stats the sibling transcript without opening it. It attaches agent type, description, transcript-mtime liveness, lock-release and ended facts to worktree rows, nests child agents, and lists live worktree-less subagents. It is registered through one module plus the registry entry.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-09-26T02:48:21Z
- **Completed:** 2026-09-26T02:59:28Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 3

## Accomplishments

- **Config root:** `resolve_config_root` uses an absolute, non-empty `CLAUDE_CONFIG_DIR`, and otherwise `<home>/.claude`. `from_env()` only resolves the path; no test ever calls it.
- **Directory encoding:** `encode_project_dir` reproduces Claude Code 2.1.283's UTF-16 encoding. A non-BMP char becomes two dashes, and names over 200 units are cut and get a base-36 `abs(i32)` hash suffix. The pinned vectors were printed by the runtime's own JS under node, and that `node -e` command is recorded in the test comment.
- **Project lookup:** candidates are the registered path, its canonical form and the main worktree, deduplicated. If an over-200 name misses, a unique-prefix fallback applies. Nothing is ever decoded.
- **Meta reading:** `read_meta_capped` is the only content read in the module. It refuses anything not named `.meta.json`, and any symlink or non-regular file. It caps the read at `META_READ_CAP` with `Read::take`, parses a `serde_json::Value`, and extracts each field on its own.
- **Id pass:** among the sessions that hold `agent-<id>`, the one with the newest transcript wins. Its meta is accepted by `worktreePath` (raw, then canonical) or, when it has no path, by the id stem. It reports `lock_released = Some(!locked)`; the lock-reason pid is never parsed.
- **Second pass:** it stats each unmatched `agent-*.jsonl` and reads a meta only when the transcript is within `LIVE_SECS`. It joins id-less worktrees by path, and attaches children by `inheritedWorktreePath` or `parentAgentId`. It emits the remaining live agents as `worktreeless`, with `agentType` verbatim. Sessions whose `subagents/` mtime is older than a day are skipped.
- **Registration:** `pub mod claude;` plus the `Box::new(claude::ClaudeCodeAdapter::from_env())` registry entry in `src/agents/adapters/mod.rs`. No edit to `src/agents/mod.rs`, `worktrees.rs` or any UI file (D-A07).

## Task Commits

1. **Task 1: tracer, a live Claude executor enriched end to end:** `0aba5b1` (feat). Tracer gate (interactive, end-of-phase, automated-only verify): `agents_claude` (2), `agents_scan` (13) and `spawn_seam_guard` (41) re-run green and the file boundary checked, then expanded.
2. **Task 2: encoding vectors, tolerant metas, joins, children, lifecycle:** `06ded9a` (test, RED) → `17f63f8` (feat, GREEN)
3. **Task 3: live worktree-less subagents and the scan-cost bound:** `7a3f734` (test, RED) → `e2d4b4c` (feat, GREEN)

No REFACTOR commits were needed.

## Files Created/Modified

- `src/agents/adapters/claude.rs`: the adapter, plus 12 unit tests in one trailing `#[cfg(test)]` module
- `src/agents/adapters/mod.rs`: `pub mod claude;` and the registry entry
- `tests/agents_claude.rs`: 10 integration tests (tracer, registry, 4 lifecycle, 4 worktree-less)

## Test Results

- `cargo test --lib agents::adapters::claude`: 12 passed.
- `cargo test --test agents_claude --no-fail-fast`: 10 passed.
- `cargo test --test agents_scan --no-fail-fast`: 13 passed; the zero-write/spawn/proc guard now covers `claude.rs`.
- `cargo test --test spawn_seam_guard`: 41 passed.
- `cargo test --no-fail-fast`: 51 `test result:` lines, 2415 passed, 1 failed, 15 ignored. The one failure is the known local git-version witness (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`). The count is 2393 + 22 new.
- `cargo clippy -- -D warnings`: clean. `cargo clippy --all-targets` shows nothing in either new file.
- `git grep -n 'deny_unknown_fields' -- src/agents`: no output.
- `git diff --stat 9cc7048 -- src tests`: exactly `claude.rs`, `adapters/mod.rs` and `tests/agents_claude.rs` (the D-A07 boundary).
- **Bound control:** with the session-age skip disabled, `a_day_old_session_is_skipped_for_worktree_less_agents_only` failed, listing the old session's agent. The change was reverted before commit.

## TDD Gate Compliance

- Both TDD tasks have their RED and GREEN commits in order: `test(25-02)` followed by `feat(25-02)`.
- **Task 2 RED targets:** `a_nested_agent_attaches_to_its_worktree_as_a_child` and `a_parent_agent_id_attaches_a_child_without_an_inherited_path`. Both failed on the children-count assertion (left 0, right 1).
- **Task 3 RED targets:** `a_live_worktree_less_subagent_is_listed_and_a_stale_one_is_not`, `agent_type_is_shown_verbatim_without_a_gsd_filter` and `a_long_running_agent_in_an_old_subagents_dir_is_still_live`. Each failed on its worktree-less assertion.
- **Green at RED time:** the encoding, config-root, tolerance, join and lifecycle tests, because the Task 1 tracer had already delivered those behaviors. The same holds for `a_day_old_session_is_skipped_for_worktree_less_agents_only`, which passed vacuously while nothing was emitted. Its bound was proven by the red-when-broken control above instead.
- **`gsd-tools check tdd-red-evidence`:** not run. Per 25-01, it parses only node TAP output and cannot read libtest's format. `workflow.tdd_mode` is not enabled, so the gate is advisory. [inferred: proceeded on cargo's output]

## Decisions Made

See `key-decisions` in the frontmatter. Every entry is marked [inferred] for audit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] `read_meta_capped` refuses symlinks and non-regular files**
- **Found during:** Task 1
- **Issue:** checking only the file name leaves two holes. A `.meta.json` symlink could redirect the read to a transcript (T-25-08). A FIFO under that name would block the scan thread indefinitely.
- **Fix:** `symlink_metadata(..).file_type().is_file()` is checked before the open.
- **Files modified:** src/agents/adapters/claude.rs
- **Verification:** the symlink case in `the_meta_reader_refuses_a_transcript_path`
- **Committed in:** 0aba5b1

**2. [Scope interpretation] The session-age bound covers the whole second pass**
- **Found during:** Task 3
- **Issue:** the plan words the bound as applying to "the worktree-less group". The second pass also attaches children and joins id-less worktrees. Bounding only the group would still stat every transcript in every session, which bounds nothing.
- **Fix:** the whole second pass skips sessions older than a day. The function doc states the wider consequence: such agents are also not attached as children or path-joined. The per-id pass is unbounded, as the plan requires.
- **Committed in:** e2d4b4c

---

**Total deviations:** 1 auto-fixed (Rule 2) and 1 scope interpretation. **Impact:** the Rule 2 fix closes a privacy and hang gap on the one content read. The interpretation keeps the D-C09 cost bound meaningful and is documented in code.

## Issues Encountered

None.

## Known Stubs

None. The 25-01 stub (`registered_adapters()` returning an empty Vec) is resolved by this plan.

## Threat Flags

None. T-25-08..14 are mitigated as planned (T-25-14's rendering lands in 25-05), and no new surface was added.

## User Setup Required

None.

## Next Phase Readiness

- 25-04 can call `scan_projects_guarded`, which now carries Claude Code facts. 25-05 renders `agent_type`/`description` through `Untrusted::shown()`.
- For audit: D8 (behavior against a real `~/.claude`) is left to phase verification once the UI is wired. The `a_projects_scan_is_sorted_by_alias_and_isolates_failures` test in `tests/agents_scan.rs` goes through `registered_adapters()`, so it now resolves a real config root, although only a tempdir project path is ever looked up under it.

## Self-Check: PASSED

- `src/agents/adapters/claude.rs` and `tests/agents_claude.rs` exist on disk.
- Commits 0aba5b1, 06ded9a, 17f63f8, 7a3f734 and e2d4b4c are all present.
- `git rev-list --count 9cc7048..HEAD` = 5.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*
