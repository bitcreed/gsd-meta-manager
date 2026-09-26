---
phase: 25-running-agents-live-wave-view
reviewed: 2026-09-25T12:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - docs/ARCHITECTURE.md
  - README.md
  - src/action.rs
  - src/agents/adapters/claude.rs
  - src/agents/adapters/mod.rs
  - src/agents/fixers.rs
  - src/agents/mod.rs
  - src/agents/waves.rs
  - src/agents/worktrees.rs
  - src/app.rs
  - src/lib.rs
  - src/state_reader/disk_status.rs
  - src/state_reader/git_ops.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/agents_claude.rs
  - tests/agents_fixers.rs
  - tests/agents_scan.rs
  - tests/agents_waves.rs
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 25: Code Review Report

**Reviewed:** 2026-09-25T12:00:00Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the phase-25 diff (`ca843ff^..HEAD`): the agent observer core, the Claude Code adapter, the wave model, the fixer estimate, the tick/handler wiring in `app.rs`, the dashboard Status cell, the Sessions › Agents sub-view, and the new tests.

The read-only discipline holds up. Every git call goes through `git_read_raw` with `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0`. Base shas and ranges are validated before they reach argv. Agent ids are validated before they become file names. Meta reads are capped and refuse symlinks. Agent-authored text is rendered only through `shown()`. I found no injection or path-traversal defect.

The defects are in the liveness and aggregation semantics:
- Nothing ages out `Finished` or `Stalled` rows, so orphaned worktrees take over the dashboard for good.
- The active-phase vote gives orphans the same weight as live agents.
- The scan has no timeout, so one hung git call freezes the whole feature silently.
- The fixer estimate can read the wrong REVIEW file, and it counts finding ids from earlier review runs.

## Critical Issues

### CR-01: Orphaned agent worktrees read `Finished`/`Stalled` forever and permanently take over the dashboard Status cell

**File:** `src/agents/mod.rs:96-113`, `src/agents/adapters/claude.rs:370-405`, `src/agents/waves.rs:251-257,431-433,452-464`, `src/agents/fixers.rs:133-143`, `src/ui/screens/normal.rs:896-901`
**Issue:** `classify_facts` sets no upper age bound for `Finished` (lock released + age > `LIVE_SECS`) or `Stalled` (age > `IDLE_SECS`, lock held). The id pass `enrich_by_id` is also deliberately unbounded ("a worktree agent is found however old its session is"). RESEARCH §Pitfall 4 notes that Claude Code releases the lock when an agent finishes and leaves the worktree until the orchestrator merges it. If a GSD run is aborted (Ctrl-C, context exhaustion, crash), its worktrees stay on disk unlocked. The phase CONTEXT itself lists "prune an orphaned worktree" as a deferred action, so orphans are expected. Each such worktree then reads `Finished` indefinitely, and:

- `is_active_liveness` treats `Finished` as active, so `AgentView::is_active()` is true. `summary_forms()` then returns a ladder such as `w2/11 0run` or `1 agent`.
- `normal.rs:896` makes that summary win the Status cell over every other state, including milestone-complete. The project's real status is hidden for as long as the orphan exists, with no agent running.
- `is_active_fixer` also counts `Finished`, so an orphaned fixer worktree shows `1 fixer` forever.
- An orphan whose Claude session died mid-run (lock never released) reads `Stalled` forever. The Status cell then reads `N stalled` permanently.

The summary is meant to answer "what is running right now", and for an orphaned project the answer is wrong on every frame after the run ends.
**Fix:** Bound the non-live verdicts by age, and keep `Finished` from counting as "active" unless something is actually running. For example:
```rust
// mod.rs — classify_facts
let age = age_secs(last, now);
if age > MAX_AGENT_AGE_SECS {
    return AgentLiveness::Ended; // or a new `Orphaned` state excluded from summaries
}
```
and in `waves.rs`:
```rust
pub fn is_active(&self) -> bool {
    self.agents.iter().any(|r| matches!(r.liveness, AgentLiveness::Live | AgentLiveness::Idle))
        || !self.worktreeless.is_empty()
}
```
`Finished` still counts inside the wave ladder while a wave is in flight. It just must not switch the ladder on by itself. Apply the same rule to `is_active_fixer`. Add a test with a released-lock worktree whose transcript is days old and assert that the Status cell is untouched.

## Warnings

### WR-01: Active-phase vote gives stalled/finished orphans equal weight with live agents

**File:** `src/agents/waves.rs:324-343`
**Issue:** `derive` chooses `active_phase` by counting attributed rows that are `Live | Idle | Finished | Stalled`, one vote each. Take three leftover `Stalled`/`Finished` worktrees from phase 12 and two `Live` executors on phase 13. The active phase becomes 12. The waves, `plan_total`, and `running` (0) all describe phase 12, and the dashboard reads `P12 · 0 run · …` while the phase-13 agents that are actually running never appear in the summary. Even without CR-01, a single `Stalled` row outvotes nothing live only by accident of the counts.
**Fix:** Vote in tiers: take the phase with the most `Live`/`Idle` rows, and fall back to `Finished`/`Stalled` only when no row is `Live` or `Idle`:
```rust
let vote = |pred: fn(AgentLiveness) -> bool| { /* count rows where pred(row.liveness) */ };
let active_phase = vote(|l| matches!(l, Live | Idle))
    .or_else(|| vote(|l| matches!(l, Finished | Stalled)))
    .unwrap_or_else(|| state.active_phase_number());
```

### WR-02: A hung git call freezes the agents scan forever, and the UI keeps showing the frozen view as current

**File:** `src/app.rs:1212-1233`, `src/state_reader/git_ops.rs:202-217` (used by `worktree_list_porcelain`, `commits_ahead`, `dirty_count`, `log_subjects`)
**Issue:** The in-flight flag is cleared only when `AgentsScanned` arrives. `git_read_raw` uses `Command::output()` with no timeout, and each scan runs `git status --porcelain` in every agent worktree of every project. If one of those calls blocks (a stuck filesystem or NFS mount, a hanging `core.fsmonitor` daemon, a huge untracked tree on a cold cache), the blocking closure never returns. `catch_unwind` guards against panics only, not hangs. After that:
- `agents_scan_in_flight` stays `true` and no scan is ever spawned again.
- `agent_views` is frozen.
- Ages are computed from the stale `scanned_at`, so the frozen view still looks fresh (`0s`, `live`).
The dashboard then reports agents as live long after they have ended, with no indication. RESEARCH Pitfall 6 addressed only the panic path.
**Fix:** Bound the scan. Either run the agent-scan git calls with a per-call timeout (spawn, then `wait_timeout` and kill; the `wait-timeout` crate or a polling `try_wait` loop), or add a watchdog: store `agents_scan_started: Option<Instant>` beside the flag, and on the tick treat a scan older than a limit (e.g. 60 s) as lost. Clear the flag, mark the views stale (or drop them), and `tracing::warn!`.

### WR-03: The fixer estimate reads `NN-EVAL-REVIEW.md` instead of the code review's `NN-REVIEW.md`

**File:** `src/agents/fixers.rs:147-166`
**Issue:** `review_files` collects every entry ending in `-REVIEW.md`, sorts them, and takes the first. GSD writes `{padded_phase}-EVAL-REVIEW.md` (gsd-eval-review) and `{padded_phase}-UI-REVIEW.md` (gsd-ui-review) into the same phase directory. `"25-EVAL-REVIEW.md"` sorts before `"25-REVIEW.md"` because `'E' < 'R'`. So in any AI phase that has been eval-reviewed, the estimate reads the eval review's frontmatter. That file either has no `findings.total` (no count is shown) or a different total, so the wrong denominator is shown.
**Fix:** Match the code-review file exactly. Require that the part after the phase prefix is exactly `REVIEW.md`:
```rust
} else if name.ends_with("-REVIEW.md")
    && !name.ends_with("-EVAL-REVIEW.md")
    && !name.ends_with("-UI-REVIEW.md")
{
```
A better version derives `{padded}-REVIEW.md` from `phase.padded()` and checks that single name, after first matching the phase directory's own prefix spelling.

### WR-04: Earlier review runs inflate the fixed count, can push it past the total, and a leftover REVIEW-FIX.md suppresses it

**File:** `src/agents/fixers.rs:256-275`
**Issue:** Three related defects, all driven by GSD's `code-review-fix` lifecycle (`gsd-core/workflows/code-review-fix.md:78,278,347,378`):
1. The numerator unions finding ids from the main worktree's last 300 subjects, with no lower bound. Finding ids are numbered per review (`CR-01`, `WR-01`, …), and every re-review renumbers from 01. So `fix(12): WR-03` commits from an earlier review of the same phase, or from an earlier `--auto` iteration, are counted as fixed in the current run. The estimate starts above zero and can exceed the denominator (`~52/48 fixed`), and nothing clamps it.
2. `review_files` treats any `*-REVIEW-FIX.md` as "the run is over". In `--auto` mode the fixer writes REVIEW-FIX.md in iteration 1 and overwrites it in iterations 2-3, so those iterations never show a count. A REVIEW-FIX.md committed by an earlier run of the phase hides the count for every later run.
3. The numbers are rendered verbatim, so `fixed > total` reaches the Status cell.
**Fix:** Intersect the collected ids with the ids that actually occur in the current REVIEW.md. The capped read already exists; scan its body for `### (CR|WR|IN)-\d+` headings. Clamp with `fixed.min(total)`. For the "run is over" test, compare mtimes (REVIEW-FIX.md newer than REVIEW.md *and* no active fixer), or drop the test and rely on the fixer-count gate, which already hides the estimate when no fixer is active.

### WR-05: The long-path prefix fallback can adopt another project's Claude directory and list its live subagents as this project's

**File:** `src/agents/adapters/claude.rs:288-310`
**Issue:** When a candidate encoding is over 200 units and its exact directory is absent, `project_dirs` accepts the *single* entry under `<root>/projects/` that starts with the same 200-unit prefix. Two projects whose encoded paths share their first 200 units (deep monorepo checkouts, sibling projects under one long parent) differ only in the hash suffix. If project A has no Claude directory yet and project B does, B's directory is the unique match and is adopted as A's. `place_live_subagents` then reports B's live subagents as A's **worktree-less agents**. A's dashboard shows `N agents` and A's Agents sub-view lists B's descriptions. The id pass is harmless (ids are random), but pass two misattributes.
**Fix:** Remove the fallback. The exact hashed name is already computed for every spelling (registered, canonical, git's main path), so a "hash drift" needs a fourth spelling that no candidate covers. If the fallback is kept, restrict it to the id-keyed pass: skip `place_live_subagents` for directories found by prefix only, so another project's agents can never become worktree-less rows here.

## Info

### IN-01: The handler comment says a fleet without running agents never redraws, but leftover rows redraw on every scan

**File:** `src/app.rs:1697-1706,1726-1742`
**Issue:** The comment says "A fleet with no agents never redraws from this arm, because its map stays empty." Any project with a leftover agent-pattern worktree (an `Unknown`/`Ended` row) still gets a view, and `AgentView` includes `scanned_at` in its `PartialEq`. The map therefore compares unequal on every scan and sets `needs_redraw` every ~5 s, forever, even though nothing on screen changes. `summary_forms()` is empty for such a view.
**Fix:** Either skip inserting views whose rows are all `Unknown`/`Ended` with no worktree-less agents (the Agents sub-view can still show them from a separate field), or correct the comment.

### IN-02: Tests that go through `registered_adapters()` read the real `~/.claude`, against D-C17

**File:** `tests/agents_scan.rs:565`, `src/app.rs` test `the_agents_scan_rides_the_session_poll_counter`
**Issue:** `scan_projects_guarded` always builds `ClaudeCodeAdapter::from_env()`, so these tests stat (and, for a long temp path, list) the developer's real `$CLAUDE_CONFIG_DIR` or `~/.claude/projects`. The adapter docs state that "no test ever reads the real home directory (D-C17)". A local Claude directory that happens to match could change the results.
**Fix:** Add `scan_projects_guarded_with(projects, adapters, now)` and have `scan_projects_guarded` delegate to it. Tests then pass a tempdir-rooted adapter or an empty list.

### IN-03: The ledger read ignores relative `gitdir:` pointers (git >= 2.48 `worktree.useRelativePaths`)

**File:** `src/agents/worktrees.rs:351-354`
**Issue:** `read_ledger_plan` returns `None` for a non-absolute admin path. Newer git can write relative `gitdir:` lines, and for those worktrees the tier-5 ledger attribution silently never fires.
**Fix:** Resolve a relative path against the worktree directory (`worktree.join(admin)`) before the `is_dir` check. Keep the plan-id validation as it is.

### IN-04: `lock_released` reports "released" for worktrees the runtime may never have locked [inferred]

**File:** `src/agents/adapters/claude.rs:344-348`
**Issue:** `lock_released: Some(worktree.locked.is_none())` turns "not locked" into "released". `Enrichment::lock_released` documents `None` for "the adapter cannot say". If a worktree was never locked by Claude (a Claude version that does not lock, or a worktree the user unlocked), a live agent that is silent for more than 120 s reads `Finished`. A long build or test run inside a single tool call is enough, and its plan is then counted `Finished`/unmerged. I could not confirm that this happens with the current Claude Code, hence [inferred].
**Fix:** Report `Some(true)` only for worktrees the adapter knows Claude locks, for example paths under `<project_root>/.claude/worktrees/`. Report `None` otherwise, so the row falls back to `Idle`/`Stalled`.

---

_Reviewed: 2026-09-25T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
