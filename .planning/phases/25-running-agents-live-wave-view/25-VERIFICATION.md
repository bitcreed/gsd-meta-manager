---
phase: 25-running-agents-live-wave-view
verified: 2026-10-28T20:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".planning/phases/25-running-agents-live-wave-view/25-01-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-01-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-02-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-02-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-03-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-03-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-04-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-04-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-05-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-05-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-06-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-06-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-07-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-07-SUMMARY.md"
  - "src/action.rs"
  - "src/agents/adapters/claude.rs"
  - "src/agents/adapters/mod.rs"
  - "src/agents/fixers.rs"
  - "src/agents/mod.rs"
  - "src/agents/processes.rs"
  - "src/agents/unmerged.rs"
  - "src/agents/waves.rs"
  - "src/agents/workstream_scope.rs"
  - "src/agents/worktree_phases.rs"
  - "src/agents/worktree_plans.rs"
  - "src/agents/worktrees.rs"
  - "src/app.rs"
  - "src/lib.rs"
  - "src/session_detector.rs"
  - "src/state_reader/disk_status.rs"
  - "src/state_reader/git_ops.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/help.rs"
  - "src/ui/screens/normal.rs"
  - "src/ui/screens/render_escape_guard.rs"
  - "tests/agents_claude.rs"
  - "tests/agents_fixers.rs"
  - "tests/agents_process.rs"
  - "tests/agents_scan.rs"
  - "tests/agents_unmerged.rs"
  - "tests/agents_waves.rs"
  - "tests/agents_workstreams.rs"
  - "tests/agents_worktree_plans.rs"
covered_digest: "v4:sha256:e34adaa2401465b1dd1f2b5b940ecf7fa4ce6ba1fc8ec8d2af3a3a03e0175f94"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 7/7
  gaps_closed: []
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
advisory:
  - finding: "WR-07: a project whose only scan output is worktree-only phases gets no AgentView (src/app.rs:2221-2227 omits worktree_phases), so its ghost phases and `N wt phases` tag never show"
    category: other
    reason: "Belongs to the post-phase quick task 23a947f (worktree-only phases), not AGENT-01..07. Confirmed by reading the handler condition."
    evidence_status: "code read; no failing test"
  - finding: "WR-08: the per-call git budget (10 s) does not bound the whole scan; a wedged shared mount can hold agents_scan_in_flight for minutes"
    category: other
    reason: "Availability only. The scan runs off the render path and no AGENT requirement promises a scan deadline."
    evidence_status: "reviewer arithmetic [inferred]"
  - finding: "WR-09: fixers::estimate reads the project's .planning, not the workstream's, for `--ws` fix runs"
    category: other
    reason: "Accuracy of a labelled `~f/t` estimate on a workstream edge case. AGENT-07 only requires a labelled estimate."
    evidence_status: "reviewer inference, unconfirmed [inferred]"
  - finding: "UI-REVIEW fix 1: the Waves pane words a finished-not-merged plan `unmerged` while the Agents tab words the same agent's liveness `done`"
    category: other
    reason: "Two different axes (plan integration state vs agent liveness), both true. Wording alignment is a UX follow-up, not a missing behaviour."
    evidence_status: "reproduced in tmux, see Behavioral Spot-Checks"
---

# Phase 25: Running Agents & Live Wave View Verification Report

**Phase Goal:** For each registered project, show which GSD agents are currently running in parallel and, for executors, which wave and plans are running / queued / done, read-only from files and git, no Claude invocation, non-intrusive to the running GSD session
**Verified:** 2026-10-28
**Status:** passed
**Re-verification:** Yes. The earlier `passed` verdict (2026-09-26) predates the review fixes WR-03..06 and four quick tasks that touched `src/agents/**`, `tests/agents_*` and the UI screens: unmerged-work detection (260929-szq), worktree-plan and ghost-phase views (261014-0jv, 23a947f), workstream scoping (261023-05i), and the process-table doctrine amendment (260926-06g). Every truth was re-checked at HEAD `2ec531d`. Source is unchanged since `54a73bf`; later commits are `.planning` docs only.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | AGENT-01: agent worktrees are enumerated with commits-ahead and dirty counts, via read-only git that takes no lock in a live agent's worktree | ✓ VERIFIED | `tests/agents_scan.rs` 13/13, including `scanning_a_live_agent_worktree_never_touches_its_index` and `no_file_under_src_agents_writes_reads_proc_or_spawns`. In the tmux run the fixture's agent rows showed `+0 ~1`, and afterwards `git status` and `.git/*.lock` in the fixture showed no change from the TUI. |
| 2 | AGENT-02: plans group by PLAN.md `wave:`; each reads done / finished / running / stalled / queued; the current wave is identified | ✓ VERIFIED | `tests/agents_waves.rs` 9/9 and `agents::waves` unit tests pass. The tmux Waves pane showed `w1  2 parallel · 1 done · 1 running` and `w2  2 parallel · 1 unmerged · 1 queued`, with the current-wave marker `▸ w1`. A finished-not-merged plan (`PlanState::Finished`) renders with the word `unmerged`; see the advisory below. |
| 3 | AGENT-03: any adapter failure degrades a row to what git knows and never drops it or fails the scan | ✓ VERIFIED | `a_panicking_adapter_leaves_the_core_rows_intact`, `a_test_only_adapter_claims_a_worktree_outside_the_predicate` and the other `agents_scan` tests pass. |
| 4 | AGENT-04: the Claude adapter attaches type, description and liveness from `<config>/projects/<encoded>/*/subagents/`, honours `$CLAUDE_CONFIG_DIR`, and lists worktree-less subagents | ✓ VERIFIED | `tests/agents_claude.rs` 13/13. WR-05 (exact encoded dirs only, `claude.rs:282-307`) is in place. In the tmux run the type `gsd-executor` and the description-derived plan `05-02` attached from a throwaway `$HOME/.claude`. |
| 5 | AGENT-05: while agents run, the dashboard Status cell shows a compact summary that fits the column; with no active agents the row is unchanged | ✓ VERIFIED | tmux at 160x45 showed `w1/2 · 1 run · 1/4` in the Status cell for the project with a live agent, while projects without running agents kept the `D R P E V` ladder or `◒N unmerged`. Tests: `a_row_without_active_agents_is_byte_identical`, the two orphan byte-identical tests, `an_agents_scan_reaches_the_dashboard_status_cell` and `the_agents_scan_rides_the_session_poll_counter` all `ok`. `AgentLiveness::is_running()` (Live or Idle only, `mod.rs:104`) is the single activation predicate and the day-age-out is at `mod.rs:157-159`. |
| 6 | AGENT-06: the Sessions tab has an Agents sub-view with summary, per-wave rows, scrollable list, worktree-less group, and explicit empty and degraded states | ✓ VERIFIED | tmux `6` then Right showed `P05 · w1/2 · 1 run · 1/4 done`, the wave row `▸w1 1 running  w2∙`, and per-agent rows `live 05-02 gsd-executor +0 ~1 25s`. `--lib ui::screens::detail` 432/432 pass, including `no_running_agents_is_the_empty_state`. |
| 7 | AGENT-07: code-review fix runs show an estimated fixed/total from REVIEW.md `findings.total` and `fix(NN):` subjects, labelled as an estimate | ✓ VERIFIED | `tests/agents_fixers.rs` 10/10. The WR-03/04/06 fixes hold at HEAD: NN-REVIEW.md-only (`fixers.rs:172-175`), intersect-and-clamp (`fixers.rs:237-245`), running-first vote (`fixers.rs:318-342`), and the running-fixer gate. The `~` label is preserved. WR-09 (workstream-scoped review file) is an open, unconfirmed edge case. |

Prohibitions (non-intrusive, read-only, no Claude invocation): `no_file_under_src_agents_writes_reads_proc_or_spawns` passes, and `Command::new` appears only in `src/session_detector.rs:132` (`pgrep -x codex`), outside `src/agents`.

**Score:** 7/7 truths verified. 0 are behavior-unverified.

### Process-table doctrine change (260926-06g)

The goal says "read-only from files and git". Quick 260926-06g amended D-B01/D-B03 so that `src/agents` still reads no process table itself. The caller takes a snapshot through `session_detector::ProcessProbe` and passes it in as data (`src/agents/processes.rs:1-12`). The probe reads one pid's stat file and readlinks its cwd. It sends no signal, does not ptrace, and opens no other process's fds (`session_detector.rs:1002-1035`). Off Linux it is `NoProcessProbe` and liveness stays mtime-only. A live lock owner never upgrades a row; only its death is evidence. I judge that this keeps the intent of "non-intrusive, no Claude invocation" because it is a passive read of procfs and invokes nothing in the running session. Both guards still pass: `tests/agents_process.rs` 7/7 and the no-spawn guard above. `[inferred — cleanup 2026-10-28]`

### Judgment on review and UI findings

| Finding | Defeats a must-have? | Reason |
|---------|----------------------|--------|
| UI-REVIEW fix 1: `unmerged` in the Waves pane vs `done` in the Agents tab | No. Advisory. | I reproduced it. `agent_state_word(Finished)` returns `done` (`detail.rs:10752`) as agent liveness; `PaneState::Unmerged` is the word for `PlanState::Finished` (`detail.rs:9539`). The facts differ: the agent stopped, the plan is not merged. AGENT-02 and AGENT-06 are met as written. The review's claim that "done contradicts a plan with no SUMMARY" is a wording problem. In my fixture the unlocked worktree reads `Finished` through `lock_released`, which is open IN-04 `[inferred]`. A UX follow-up should align the vocabulary. |
| UI-REVIEW fixes 2 and 3 (unlabelled columns, first-paint ladder before the first scan) | No. Advisory. | Presentation polish; behaviour is correct once the scan lands (about 5 s in my run). |
| WR-07 (`app.rs:2221-2227` omits `worktree_phases`) | No. Advisory. | Affects only the Roadmap ghost node and `N wt phases` tag from quick 23a947f for worktree-only phases outside `.claude/worktrees/`. It is not in AGENT-01..07. Real defect, worth a small fix. |
| WR-08 (scan has no overall deadline) | No. Advisory. | The scan is off the render path. The failure mode is stale views on a wedged mount, and no requirement promises a scan deadline. |
| WR-09 (fixer estimate not workstream-aware) | No. Advisory. | Accuracy of a labelled estimate on a workstream edge case; unconfirmed. |
| IN-01..IN-16 | No. | Info-level and not gating. IN-13 (the zero-write guard cannot see several write paths) is why I also checked the fixture for stray files after the tmux run. |
| 25-SECURITY.md | Supports. | 33/33 threats closed, 0 open. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/agents/{mod,worktrees,waves,fixers,processes}.rs` | Core scan, classification, wave model, fixer estimate | ✓ VERIFIED | Substantive (6.7k lines under `src/agents`), wired from `src/app.rs` scan handler and UI. |
| `src/agents/adapters/{mod,claude}.rs` | Pluggable adapter seam and Claude adapter | ✓ VERIFIED | Wired through `registered_adapters()`. |
| `src/agents/{unmerged,worktree_plans,worktree_phases,workstream_scope}.rs` | Post-phase additions | ✓ VERIFIED | Covered by `agents_unmerged`, `agents_worktree_plans`, `agents_workstreams` (10, 9 and 4 tests pass). Not part of AGENT-01..07. |
| `src/ui/screens/{normal,detail}.rs` | Status-cell summary and Agents tab | ✓ VERIFIED | Rendered and driven in tmux. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| Tick counter (`app.rs`) | `scan_projects_guarded` | `the_agents_scan_rides_the_session_poll_counter` | ✓ WIRED | Test passes. |
| `AgentsScanned` handler | `ctx.agent_views` | insert `AgentView` per project | ✓ WIRED | `an_agents_scan_reaches_the_dashboard_status_cell` passes. One exception: WR-07 (advisory). |
| `ctx.agent_views` | dashboard Status cell | `agent_summary_line` | ✓ WIRED | Seen in tmux: `w1/2 · 1 run · 1/4`. |
| `ctx.agent_views` | Sessions > Agents tab and Waves pane | `render_agents_tab`, `render_waves_pane` | ✓ WIRED | Seen in tmux. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| Status cell | `agent_views[alias]` | `git worktree list --porcelain`, `rev-list`, `status`, the adapter's meta and jsonl mtime, PLAN.md `wave:` | Yes. The tmux fixture's disk state showed up as live/done rows and wave counts. | ✓ FLOWING |
| Agents tab rows | `AgentView.rows` | same | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Phase-25 integration suites | `rtk proxy cargo test --no-fail-fast --test agents_claude --test agents_waves --test agents_fixers --test agents_scan --test agents_process --test agents_unmerged --test agents_workstreams --test agents_worktree_plans` | 13+9+10+13+7+10+4+9 passed, 0 failed (1 ignored) | ✓ PASS |
| Agents unit tests | `cargo test --lib agents` | 103 passed, 0 failed | ✓ PASS |
| Byte-identical dashboard regressions | `cargo test --lib byte_identical` | 3 phase-25 tests `ok` | ✓ PASS |
| Detail screen | `cargo test --lib ui::screens::detail` | 432 passed, 0 failed | ✓ PASS |
| Full suite (collected gate, HEAD 54a73bf, source unchanged since) | `rtk proxy cargo test --no-fail-fast` | 3183 passed, 0 failed, 17 ignored; clippy `-D warnings` and `fmt --check` exit 0 | ✓ PASS (cited, not re-run) |
| Rendered TUI | detached tmux 160x45, throwaway `$HOME`/XDG under the scratchpad, fixture with two `.claude/worktrees/agent-*` worktrees and subagent meta | Status cell, Waves pane and Agents tab as in Truths 2, 5 and 6. Session killed afterwards. | ✓ PASS |

### Probe Execution

SKIPPED. The phase declares no probes (`probe-*.sh`), and `scripts/*/tests/probe-*.sh` is not part of its plans.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| AGENT-01 | 25-01, 25-07 | Enumerate agent worktrees non-intrusively | ✓ SATISFIED | Truth 1 |
| AGENT-02 | 25-03, 25-07 | Wave model from frontmatter | ✓ SATISFIED | Truth 2 |
| AGENT-03 | 25-01, 25-02 | Adapter seam degrades gracefully | ✓ SATISFIED | Truth 3 |
| AGENT-04 | 25-02, 25-07 | Claude adapter | ✓ SATISFIED | Truth 4 |
| AGENT-05 | 25-04, 25-07 | Dashboard summary | ✓ SATISFIED | Truth 5 |
| AGENT-06 | 25-05, 25-07 | Agents sub-view | ✓ SATISFIED | Truth 6 |
| AGENT-07 | 25-06, 25-07 | Fixed/total estimate | ✓ SATISFIED | Truth 7 |

All seven IDs appear in PLAN frontmatter (`requirements:` of plans 25-01..25-07). All seven are `[x]` in `.planning/REQUIREMENTS.md` (lines 109-115) with `Complete` in the traceability table (lines 370-376). No orphaned requirements are mapped to Phase 25.

### Anti-Patterns Found

None blocking. A grep for `TBD|FIXME|XXX|TODO|HACK` over `src/agents/**` and `tests/agents_*.rs` returned no unreferenced markers. The three items in `deferred-items.md` (WINDOWS.md drift, clippy warnings, fmt) are closed with evidence in that file.

### Human Verification Required

None. Everything was settled with tests, code reading and a tmux drive of the real binary.

### Gaps Summary

No gaps. All seven requirements hold at HEAD after the later review fixes and quick tasks, with 0 critical review findings. Four advisory items are recorded above (WR-07, WR-08, WR-09, and the UI-review `unmerged`/`done` wording). They are good candidates for a small follow-up pass. None defeats an AGENT-01..07 must-have.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
