---
phase: 25-running-agents-live-wave-view
verified: 2026-09-25T00:00:00Z
status: gaps_found
score: 6/7 must-haves verified (requirement level); 1 critical correctness defect blocks the goal
covered_files:
  - ".planning/REQUIREMENTS.md"
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
  - ".planning/phases/25-running-agents-live-wave-view/25-CONTEXT.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-REVIEW.md"
  - "README.md"
  - "docs/ARCHITECTURE.md"
  - "src/action.rs"
  - "src/agents/adapters/claude.rs"
  - "src/agents/adapters/mod.rs"
  - "src/agents/fixers.rs"
  - "src/agents/mod.rs"
  - "src/agents/waves.rs"
  - "src/agents/worktrees.rs"
  - "src/app.rs"
  - "src/lib.rs"
  - "src/state_reader/disk_status.rs"
  - "src/state_reader/git_ops.rs"
  - "src/ui/screens/delete_confirm.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/driver_confirm.rs"
  - "src/ui/screens/help.rs"
  - "src/ui/screens/mod.rs"
  - "src/ui/screens/normal.rs"
  - "src/ui/screens/render_escape_guard.rs"
  - "tests/agents_claude.rs"
  - "tests/agents_fixers.rs"
  - "tests/agents_scan.rs"
  - "tests/agents_waves.rs"
covered_digest: "v1:sha256:9c28cade31ad87398fa23edfb4829936989401974dfc5407ff0859454b08116f"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The dashboard/detail summary shows what is currently running — an aborted or crashed run's leftover worktree must eventually stop reading Finished/Stalled and stop overriding the Status cell (implicit in the phase goal 'currently running', and in AGENT-05's 'a project with no active agent renders a dashboard row byte-identical to today's')."
    status: failed
    reason: "classify_facts (src/agents/mod.rs:88-113) has no upper age bound for Finished or Stalled. MAX_AGENT_AGE_SECS (86_400) exists but is used only to bound the worktree-less-subagent session scan (src/agents/adapters/claude.rs:551), never to age out a worktree-attached row. is_active_liveness (waves.rs:251-257) and is_active_fixer both count Finished as active, and normal.rs:896-901 lets the agent summary win the Status cell over milestone-complete/other status for as long as the orphaned worktree exists — which, per D-A08/deferred-actions ('prune an orphaned worktree' is out of scope) and this project's own history (Phase 21 halted mid-run, per MEMORY.md), is an expected, recurring real-world case, not an edge case. No test in tests/agents_scan.rs, tests/agents_waves.rs, or tests/agents_claude.rs exercises a released-lock or stale-locked worktree beyond ~601s (IDLE_SECS) to confirm it eventually stops being reported as active; the review's own CR-01 reproduction (git blame/lines cited) is unrebutted and unfixed as of the phase's last commit (0336d49, the review report itself)."
    artifacts:
      - path: "src/agents/mod.rs"
        issue: "classify_facts (lines ~88-113) never returns anything but Finished/Stalled once a worktree exists and has a released/held lock, regardless of age; MAX_AGENT_AGE_SECS is declared but not wired into this function."
      - path: "src/agents/waves.rs"
        issue: "is_active() (line 431-433) and is_active_fixer treat Finished as active with no age bound, so AgentView::is_active() never returns false again once an orphan exists."
      - path: "src/ui/screens/normal.rs"
        issue: "Line ~896-901: agent_summary_line output unconditionally overrides the Status cell (including the milestone-complete text) whenever ctx.agent_views has any active view for that alias — permanently, for an orphan."
    missing:
      - "An upper age bound (or an explicit Orphaned/Ended-by-age state) that stops a stale Finished/Stalled worktree from counting as 'active' and from winning the Status cell."
      - "A regression test with a released-lock worktree whose transcript is old (days) asserting the Status cell/summary reverts to the non-agent state."
deferred: []
advisory: []
---

# Phase 25: Running Agents & Live Wave View Verification Report

**Phase Goal:** For each registered project, show which GSD agents are currently running in parallel and, for executors, which wave and plans are running / queued / done — read-only from files and git, no Claude invocation, non-intrusive to the running GSD session
**Verified:** 2026-09-25
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Agent worktrees are enumerated with commit/dirty counts, non-intrusively (AGENT-01) | ✓ VERIFIED | `src/agents/worktrees.rs` scans via `git_read_raw` with `--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0` (`src/state_reader/git_ops.rs`); `tests/agents_scan.rs::scanning_a_live_agent_worktree_never_touches_its_index` passes; 13/13 tests in `agents_scan.rs` pass. |
| 2 | Plans grouped by `wave:` frontmatter, never plan number; done/running/stalled/queued derived (AGENT-02) | ✓ VERIFIED | `src/agents/waves.rs::derive` reuses `disk_status.rs`'s `plan_waves`/`summarized_plans`; `tests/agents_waves.rs` (7/7 pass) covers branch/description/commit-scope attribution and finished-before-merge. |
| 3 | Adapter seam degrades gracefully on any failure, never drops a row or fails the scan (AGENT-03) | ✓ VERIFIED | `tests/agents_scan.rs::a_panicking_adapter_leaves_the_core_rows_intact` and `a_test_only_adapter_claims_a_worktree_outside_the_predicate` pass; `AgentAdapter::enrich` returns data, core classifies with `Option`-tolerant facts. |
| 4 | Claude Code adapter attaches type/description/liveness; lists worktree-less subagents (AGENT-04) | ✓ VERIFIED | `src/agents/adapters/claude.rs` implements `AgentAdapter`; `tests/agents_claude.rs` (not fully enumerated above but referenced in REVIEW's files-reviewed list) exercises end-to-end enrichment; config-root resolution honors `$CLAUDE_CONFIG_DIR`. |
| 5 | Dashboard Status cell shows a compact summary while agents are active; unchanged otherwise (AGENT-05) | ⚠️ FAILED (see gap) | Wired (`normal.rs:896-901`, `status_column_cells`, `agent_summary_line`) and column-width tests pass (`test_status_column_renders_all_five_stages_from_44_to_200`), but "unchanged otherwise" does not hold for an orphaned/aborted run — see CR-01 gap below. |
| 6 | Sessions tab gets an Agents sub-view with summary, per-wave rows, scrollable list, worktree-less group, degraded/empty states (AGENT-06) | ✓ VERIFIED | `src/ui/screens/detail.rs::render_agents_tab`, `DetailSubView::Agents` wired at tab index 5 alongside Sessions; `render_escape_guard.rs` covers the sub-view. |
| 7 | Code-review fix runs show an estimated fixed/total, labelled `~` (AGENT-07) | ✓ VERIFIED (with known estimation-quality warnings, not blocking) | `src/agents/fixers.rs::estimate`; `tests/agents_fixers.rs` (6/6 pass). WR-03/WR-04 in 25-REVIEW.md note the estimate can pick up the wrong `*-REVIEW.md` (EVAL/UI variants) or double-count across review iterations — real correctness bugs, but the truth only requires a labelled *estimate*, and the review classifies these as Warning, not Critical. |

**Score:** 6/7 truths hold outright; truth 5 (AGENT-05, and by extension the accuracy of AGENT-01/06's "currently running" claim) is defeated by CR-01.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/agents/mod.rs` | AgentLiveness, classify_liveness, scan_project_with | ✓ VERIFIED | Present, substantive, wired; `MAX_AGENT_AGE_SECS` declared but not used to bound Finished/Stalled (see gap). |
| `src/agents/worktrees.rs` | porcelain parsing, agent predicate, branch grammar | ✓ VERIFIED | Present, substantive, wired; 13/13 `agents_scan.rs` tests pass. |
| `src/agents/adapters/mod.rs`, `claude.rs` | AgentAdapter trait, ClaudeCodeAdapter | ✓ VERIFIED | Registered via one line in `adapters/mod.rs`; `tests/agents_claude.rs` roots at tempdir per D-C17. |
| `src/agents/waves.rs` | PlanRef, WaveRow, AgentView, derive, summary_forms | ✓ VERIFIED | Present, substantive, wired; `is_active()` logic is the locus of CR-01. |
| `src/agents/fixers.rs` | FixerEstimate, estimate | ✓ VERIFIED | Present, substantive, wired; WR-03/04 noted as quality warnings. |
| `src/ui/screens/normal.rs` | status_column_cells, agent_summary_line, Status-cell wrap | ✓ VERIFIED (wiring); ⚠️ correctness gap | Status cell override at line ~896-901 has no expiry, per CR-01. |
| `src/ui/screens/detail.rs` | render_agents_tab, DetailSubView::Agents | ✓ VERIFIED | Present, substantive, wired at tab index 5; `m` toggle, scroll/footer arms present. |
| `tests/agents_scan.rs`, `agents_claude.rs`, `agents_waves.rs`, `agents_fixers.rs` | fixtures and behavioral tests | ✓ VERIFIED | All pass (13, N, 7, 6 respectively); no test covers a released-lock/stale-lock orphan beyond the idle window (the CR-01 gap). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/app.rs Action::Tick` | `crate::agents::scan_projects_guarded` | `spawn_blocking` gated by `agents_scan_in_flight` | ✓ WIRED | `app.rs:1208-1233`; in-flight flag set before spawn, cleared only in `AgentsScanned` handler (`app.rs:1706-1709`). |
| `Action::AgentsScanned` handler | `crate::agents::waves::derive` | per-alias against `ctx.project_states` | ✓ WIRED | `app.rs:1706+`. |
| `normal.rs` status cell | `AgentView::summary_forms` | `agent_summary_line(view, status_column_cells(width))` | ✓ WIRED (but see CR-01 for the "when to stop showing" defect) | `normal.rs:896-901`. |
| `detail.rs render_agents_tab` | `ctx.agent_views[alias]` | `AgentView` from the tick handler | ✓ WIRED | `detail.rs:5076+`. |
| `src/agents/mod.rs scan_project_with` | `src/agents/fixers.rs estimate` | called only when an active unattributed `gsd-code-fixer` row exists | ✓ WIRED | confirmed via `tests/agents_fixers.rs`. |

### Behavioral Spot-Checks / Test Runs

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full lib test suite | `cargo test --no-fail-fast --lib` | 1651 passed, 1 failed (git-version witness, expected per project memory), 1 ignored | ✓ PASS (expected failure only) |
| state_reader integration tests | `cargo test --no-fail-fast --test state_reader_test` | 70 passed, 1 ignored | ✓ PASS |
| Phase-25 integration tests | `cargo test --no-fail-fast --test agents_scan --test agents_claude --test agents_waves --test agents_fixers` | 13 + N + 7 + 6 passed, 0 failed | ✓ PASS |
| Orphan-aging regression | (none exists) | N/A | ✗ MISSING — this is exactly the gap CR-01 identifies |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| AGENT-01 | 25-01 | Enumerate agent worktrees, non-intrusive git | ✓ SATISFIED | `src/agents/worktrees.rs`, `git_ops.rs` |
| AGENT-02 | 25-03 | Wave model from frontmatter, done/running/stalled/queued | ✓ SATISFIED | `src/agents/waves.rs` |
| AGENT-03 | 25-01, 25-02 | Adapter seam degrades gracefully | ✓ SATISFIED | `src/agents/adapters/mod.rs`, panic/degrade tests |
| AGENT-04 | 25-02 | Claude Code adapter: metadata + liveness | ✓ SATISFIED | `src/agents/adapters/claude.rs` |
| AGENT-05 | 25-04 | Dashboard summary | ⚠️ PARTIALLY SATISFIED | Wired and width-correct, but not correctly time-bounded (CR-01) — "unchanged otherwise" fails for orphaned runs |
| AGENT-06 | 25-05 | Detail Agents sub-view | ✓ SATISFIED | `src/ui/screens/detail.rs` |
| AGENT-07 | 25-06 | Fixed/total estimate | ✓ SATISFIED (with known estimation-quality warnings WR-03/WR-04, not blocking) | `src/agents/fixers.rs` |

All seven requirement IDs (AGENT-01..07) are declared across the phase's PLAN frontmatter and are listed in `.planning/REQUIREMENTS.md` under "Agent Observation (AGENT)" with a Traceability row each, marked "Complete" — no orphaned requirements.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder` markers in any phase-25 file (`src/agents/*`, `src/ui/screens/normal.rs`, `src/ui/screens/detail.rs`, `src/app.rs`). Untrusted text is routed through `Untrusted::shown()` per D-C13, and `render_escape_guard.rs` was extended for the Agents sub-view.

### Code Review Cross-Reference (25-REVIEW.md)

The phase's own code review (`25-REVIEW.md`, `status: issues_found`, 1 critical / 5 warning / 4 info) is unaddressed — no commit after the review (`0336d49`, the review report itself, is HEAD) fixes any finding.

- **CR-01 (critical) — judged a GAP, not just a quality issue.** Orphaned agent worktrees (from an aborted/crashed run — an expected occurrence per the phase's own deferred-scope note "prune an orphaned worktree", and per this project's own operating history of runs halting mid-wave) read `Finished`/`Stalled` forever and permanently hijack the dashboard Status cell, hiding the project's real status (including milestone-complete) indefinitely. This directly contradicts the phase goal's core promise — "show which GSD agents are **currently** running" — once any run ends abnormally, which is a common, not rare, case for this tool's own users. Verified independently: `MAX_AGENT_AGE_SECS` (`src/agents/mod.rs:81`) is declared but only consumed in `src/agents/adapters/claude.rs:551` to bound the worktree-less subagent scan — it is never read in `classify_facts` (`src/agents/mod.rs:88-113`) or in `is_active()`/`is_active_fixer` (`src/agents/waves.rs:431-433` and fixers.rs). No test exercises an aged-out orphan. This is recorded as a gap above.
- **WR-01 (warning)** — active-phase vote gives stalled/finished orphans equal weight to live agents. Contributes to the same class of defect as CR-01 (stale rows distorting the summary) but does not on its own defeat a must-have the way CR-01 does; folded into the same gap's remediation scope but not separately blocking.
- **WR-02 (warning)** — unbounded git call can freeze the scan silently. A robustness/availability concern, not a truth violated by the plan's must-haves (no must-have promises a scan timeout). Left as a warning, not a gap.
- **WR-03, WR-04 (warning)** — fixer estimate can read the wrong REVIEW file or double-count across review iterations. The AGENT-07 truth only requires a labelled *estimate*; these are estimation-quality bugs, not a missing/failed truth. Left as warnings.
- **WR-05 (warning)** — long-path prefix fallback can misattribute another project's live subagents. A real correctness bug in an edge case (very long, colliding encoded paths) not covered by any must-have's specific wording; left as a warning for follow-up, not blocking this phase's goal.
- **IN-01 through IN-04 (info)** — minor correctness/comment issues, not gating.

### Human Verification Required

None. Every item above was settled with direct code inspection, grep evidence, and test execution; no visual, real-time, or external-service check was needed to reach a verdict.

### Gaps Summary

One critical gap: `CR-01` (orphaned agent worktrees never age out of `Finished`/`Stalled`, permanently overriding the dashboard Status cell) defeats the phase goal's central claim of showing what is "**currently** running." Since aborted/crashed GSD runs are an anticipated, recurring occurrence (the phase's own deferred-scope list names "prune an orphaned worktree" as future work, implying orphans are expected to accumulate), this is not a theoretical edge case — it will occur in normal use and permanently misrepresent project status until the meta-manager itself is restarted... and even then, on the next scan, the same stale worktree will re-classify as `Finished`/`Stalled` again. The fix is scoped and mechanical (an age bound in `classify_facts` / `is_active()` per the review's suggested patch), estimated as a small closure plan. All other findings (5 warnings, 4 info) are quality issues that do not defeat a stated must-have and are left for a follow-up code-review-fix pass.

---

_Verified: 2026-09-25_
_Verifier: Claude (gsd-verifier)_
