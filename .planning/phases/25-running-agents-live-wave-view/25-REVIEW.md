---
phase: 25-running-agents-live-wave-view
reviewed: 2026-09-26T04:36:07Z
depth: standard
review_kind: re-review after gap-closure plan 25-07 (diff 0336d49..06f6045)
files_reviewed: 7
files_reviewed_list:
  - src/agents/fixers.rs
  - src/agents/mod.rs
  - src/agents/waves.rs
  - src/ui/screens/normal.rs
  - tests/agents_claude.rs
  - tests/agents_fixers.rs
  - tests/agents_waves.rs
findings:
  critical: 0
  warning: 5
  info: 7
  total: 12
status: issues_found
---

# Phase 25: Code Review Report (re-review after 25-07)

**Reviewed:** 2026-09-26T04:36:07Z
**Depth:** standard
**Files Reviewed:** 7 (the incremental scope of gap-closure plan 25-07, commits `29cd4f3..06f6045`). Carried-forward findings were re-checked in their own files: `src/app.rs`, `src/state_reader/git_ops.rs`, `src/agents/adapters/claude.rs`, `src/agents/worktrees.rs` and `tests/agents_scan.rs`. None of these changed since `0336d49`.
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This is a re-review after gap-closure plan 25-07, which was written to close CR-01 (orphaned worktrees took over the Status cell) and WR-01 (orphans had equal weight in the active-phase vote).

Both are resolved where they were reported:
- `classify_facts` now consumes `MAX_AGENT_AGE_SECS`.
- One predicate, `AgentLiveness::is_running`, now decides activation, the first vote tier, executor mode and the `N agents` count.
- The fixer estimate needs a running fixer.
- Byte-identical dashboard regressions pin the result.

The targeted suites pass locally (`agents_claude` 12/12, `agents_waves` 7/7, `agents_fixers` 7/7, `--lib agents::` 45/45).

The fix was applied to the wave model but not to the fixer model beside it. `fixers::estimate` still counts `Finished` fixers into the run's size, the phase vote and the `fixed` numerator. Its phase vote is still untiered. So finished orphan fixers from an aborted run can outvote the one running fixer and point the estimate at the wrong phase. That is the WR-01 defect again, in the other vote (new WR-06).

All eight deferred prior findings (WR-02..WR-05, IN-01..IN-04) are still open at the locations below. 25-07 makes two of them more significant:
- IN-01: aged-out orphans are now `Ended` rows that still create a view.
- IN-04: a never-locked worktree that reads `Finished` no longer switches the summary on.

No new critical issue was found.

## Prior Findings Status

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 | **Resolved** | See below. |
| WR-01 | **Resolved** (in `waves.rs`); the same pattern remains in `fixers.rs` as new WR-06 | `src/agents/waves.rs:319-336`: the tiered `vote` closure, where `is_running()` goes first, `Finished \| Stalled` only as the fallback, then `state.active_phase_number()`. Ties still go to the higher phase. Pinned by `running_agents_outvote_orphans_for_the_active_phase` (`waves.rs:966-996`). |
| WR-02 | Open | Unchanged: `src/app.rs:1212-1234`, `src/state_reader/git_ops.rs:202-217`. |
| WR-03 | Open | Moved to `src/agents/fixers.rs:152-171`. |
| WR-04 | Open | Moved to `src/agents/fixers.rs:163-164,266-286`. |
| WR-05 | Open | Unchanged: `src/agents/adapters/claude.rs:288-311`. |
| IN-01 | Open, impact increased | Unchanged: `src/app.rs:1697-1705,1719-1743`. |
| IN-02 | Open | Unchanged: `tests/agents_scan.rs:565`, `src/app.rs:5385`. |
| IN-03 | Open | `src/agents/worktrees.rs:352`. |
| IN-04 | Open, impact increased [inferred] | `src/agents/adapters/claude.rs:345-347`. |

**CR-01 evidence, location by location:**
- `src/agents/mod.rs:127-132`: `age > MAX_AGENT_AGE_SECS` returns `Ended` before the released-lock `Finished` rule (`:133`). Children take the same path through `classify_child` (`:171-174`).
- `src/agents/mod.rs:89-91`: `is_running` covers `Live | Idle` only.
- `src/agents/adapters/claude.rs:362-405`: `enrich_by_id` is still unbounded, as the prior finding noted. Its `last_activity` now flows into the bounded classifier, so an aged meta reads `Ended`.
- `src/agents/waves.rs:432-434`: `is_active` counts running rows and worktree-less agents only.
- `src/agents/waves.rs:471-480`: `executor_mode` needs a running row attributed to the active phase.
- `src/agents/waves.rs:485-490`: the `N agents` count covers running rows only.
- `src/agents/fixers.rs:219-224`: the running-fixer gate sits before the first `log_subjects` call.
- `src/ui/screens/normal.rs:896-901`: unchanged, and correct now, because `agent_summary_line` yields `None` for an orphan-only view. Pinned at `normal.rs:3007` and `:3069`, and at `tests/agents_claude.rs:304-362`.

The residual behaviour is by design [inferred, per the 25-07 flagged assumptions]. A held-lock crashed orphan still shows `N stalled` for up to 24 h, and a released-lock orphan's `Finished` row stops activating immediately.

## Warnings

### WR-02: A hung git call freezes the agents scan forever, and the UI keeps showing the frozen view as current (carried forward)

**File:** `src/app.rs:1212-1234`, `src/state_reader/git_ops.rs:202-217` (used by `worktree_list_porcelain` :563, `commits_ahead` :593, `dirty_count` :608, `log_subjects` :624)
**Issue:** Unchanged by 25-07. `agents_scan_in_flight` is cleared only by the `AgentsScanned` handler (`app.rs:1709`). `git_read_raw` calls `Command::output()` with no timeout. If one `git status --porcelain` in one agent worktree hangs (a stuck mount, a hanging `core.fsmonitor`), the blocking closure never sends. After that no scan is ever spawned again, and `agent_views` is frozen with the stale `scanned_at`, so the stale view still reads `live`. `catch_unwind` covers panics only.
**Fix:** Add a watchdog. Store `agents_scan_started: Option<Instant>` next to the flag. On the tick, treat a scan older than about 60 s as lost: clear the flag, drop or mark the views stale, and `tracing::warn!`. Alternatively, run the agent-scan git calls with a per-call timeout (`try_wait` polling and kill).

### WR-03: The fixer estimate reads `NN-EVAL-REVIEW.md` instead of the code review's `NN-REVIEW.md` (carried forward)

**File:** `src/agents/fixers.rs:152-171`
**Issue:** Unchanged by 25-07. `review_files` collects every name ending in `-REVIEW.md`, sorts them and takes the first. `25-EVAL-REVIEW.md` sorts before `25-REVIEW.md` (`'E' < 'R'`), and `NN-UI-REVIEW.md` also ends in `-REVIEW.md`. In an eval-reviewed phase the denominator comes from the wrong file, or is missing.
**Fix:** Match the code-review file exactly:
```rust
} else if name.ends_with("-REVIEW.md")
    && !name.ends_with("-EVAL-REVIEW.md")
    && !name.ends_with("-UI-REVIEW.md")
{
```
A better version checks the single name `{padded}-REVIEW.md`, using the phase directory's own prefix spelling.

### WR-04: Earlier review runs inflate the fixed count, can push it past the total, and a leftover REVIEW-FIX.md suppresses it (carried forward)

**File:** `src/agents/fixers.rs:163-164,266-269` (the REVIEW-FIX test) and `:274-285` (the unbounded union, no clamp)
**Issue:** Unchanged by 25-07.
1. The numerator unions finding ids from main's last 300 subjects with no lower bound. Re-reviews renumber from `CR-01`/`WR-01`, so ids fixed in earlier runs count again, and `fixed` can exceed `total` (`~52/48 fixed`).
2. Any `*-REVIEW-FIX.md`, including one committed by an earlier run or `--auto` iteration 1, hides the count for every later iteration.
3. `fixed > total` reaches the Status cell unclamped.

The new running-fixer gate now provides the "run is over" signal, so the REVIEW-FIX existence test is redundant as well as harmful.
**Fix:** Intersect the collected ids with the `### (CR|WR|IN)-\d+` headings of the current REVIEW.md, which is already read under a cap. Clamp with `fixed.min(total)`. Drop the `fix_report` early return, and rely on the running-fixer gate (`fixers.rs:222`).

### WR-05: The long-path prefix fallback can adopt another project's Claude directory and list its live subagents as this project's (carried forward)

**File:** `src/agents/adapters/claude.rs:288-311`
**Issue:** Unchanged by 25-07. When an encoding is over 200 units and its exact directory is absent, the single entry that shares the 200-unit prefix is adopted. Take two projects under one long parent. If A has no Claude directory yet and B has one, A gets B's directory. `place_live_subagents` then reports B's live subagents as A's worktree-less agents, and those now switch A's Status cell on (`is_active`, `waves.rs:433`).
**Fix:** Remove the fallback, because the three exact spellings are already hashed. If it is kept, skip `place_live_subagents` for directories found by prefix only.

### WR-06: Finished orphan fixers still shape the fixer estimate: they outvote the running fixer for the phase and inflate `N fixers` and `fixed` (new; the WR-01/CR-01 pattern left in `fixers.rs`)

**File:** `src/agents/fixers.rs:137-148` (`is_active_fixer` keeps `Finished`), `:219-228` (the count), `:240-258` (the untiered phase vote), `:274-277` (the numerator union)
**Issue:** 25-07 added a gate: at least one fixer must be running (`:222`). Past the gate, every `Finished` unattributed fixer still counts as fully as a running one. A `Finished` row lasts up to `MAX_AGENT_AGE_SECS` (a day), so an aborted fix run's released-lock fixers stay `Finished` for a day. Suppose an aborted `code-review-fix` on phase 12 leaves three finished fixers, and one fixer for phase 13 is now running:
- `estimate.fixers == 4`, so the cell reads `4 fixers`. Only one is running.
- The vote at `:252-256` is 3 to 1 for phase 12. `total` comes from `12-REVIEW.md`, and `fixed` counts phase-12 ids, including the orphans' **unmerged** commits (`:275-277`).
- The cell shows `4 fixers · ~k/N fixed` for a phase nobody is fixing.

`waves.rs` fixed exactly this for the active phase (WR-01): running rows vote first, and leftovers vote only when nothing runs. The fixer vote was left as it was. The 25-07 plan's statement that "a finished fixer counts inside a running run" assumes every finished fixer belongs to the running run. Nothing checks that.
**Fix:** Tier the vote the way `derive` does, then keep only the finished fixers that share the chosen phase:
```rust
let vote_over = |pred: fn(AgentLiveness) -> bool| -> Option<PhaseNum> { /* same BTreeMap vote, rows where pred(row.liveness) */ };
let phase = vote_over(|l| l.is_running()).or_else(|| vote_over(|_| true));
// then retain fixers whose own fixer_phase is None or == phase before
// computing `estimate.fixers` and unioning their subjects.
```
Add a test with one `Live` fixer on phase 13 and three `Finished` fixers carrying `fix(12): …` commits. Assert `phase == 13` and `fixers == 1`.

## Info

### IN-01: The handler comment says a fleet without running agents never redraws, but leftover rows redraw on every scan (carried forward; impact increased by 25-07)

**File:** `src/app.rs:1697-1705,1719-1743`
**Issue:** Unchanged. The comment says a fleet with no agents never redraws from this arm. A view is still inserted whenever `rows` is non-empty (`:1721`). `AgentView` carries `scanned_at`, so the map compares unequal on every scan and sets `needs_redraw` every ~5 s. After 25-07, every aged-out orphan is an `Ended` row with no summary forms, and it still triggers that redraw forever. That is precisely the population CR-01 was about.
**Fix:** Skip inserting a view when no row `is_running()`, no row is `Stalled` and `worktreeless` is empty. The Agents sub-view can still list rows from a separate field. Otherwise, correct the comment.

### IN-02: Tests that go through `registered_adapters()` read the real `~/.claude`, against D-C17 (carried forward)

**File:** `tests/agents_scan.rs:565`, `src/app.rs:5385` (`the_agents_scan_rides_the_session_poll_counter`)
**Issue:** Unchanged. `scan_projects_guarded` (`src/agents/mod.rs:447-468`) always builds `ClaudeCodeAdapter::from_env()`, so these tests stat the developer's real Claude config directory.
**Fix:** Add `scan_projects_guarded_with(projects, adapters, now)` and have `scan_projects_guarded` delegate to it. Tests then pass a tempdir-rooted adapter.

### IN-03: The ledger read ignores relative `gitdir:` pointers (git >= 2.48 `worktree.useRelativePaths`) (carried forward)

**File:** `src/agents/worktrees.rs:351-352`
**Issue:** Unchanged. A relative admin path returns `None`, so tier-5 ledger attribution never fires for such worktrees.
**Fix:** Resolve `worktree.join(admin)` before the `is_dir` check.

### IN-04: `lock_released` reports "released" for worktrees the runtime may never have locked [inferred] (carried forward; impact increased by 25-07)

**File:** `src/agents/adapters/claude.rs:345-347`
**Issue:** Unchanged. `Some(worktree.locked.is_none())` turns "not locked" into "released". Before 25-07, a never-locked live agent silent for more than 120 s read `Finished` and still switched the summary on. Now `Finished` never activates (`waves.rs:433`), and it never turns on the fixer gate (`fixers.rs:222`). Such an agent inside a single long tool call (a build or test run of more than 2 min) disappears from the Status cell and from the fixer estimate until it writes again. The premise, a runtime that does not lock, is still unconfirmed (RESEARCH A1 observed 13/13 locked).
**Fix:** Report `Some(true)` only for worktrees the adapter knows Claude locks, for example under `<project_root>/.claude/worktrees/`. Report `None` otherwise, so the row falls back to `Idle`/`Stalled`.

### IN-05: A worktree's live child agents never count as running, so a quiet parent with a working child can raise `N stalled` [inferred] (new, pre-existing behaviour)

**File:** `src/agents/waves.rs:432-434` (`is_active`), `:458-468` (the stalled form)
**Issue:** `is_active` looks at `row.liveness` and `worktreeless` only, never at `row.children`, which the scan classifies and keeps (`mod.rs:388-392`). Take a locked worktree whose own transcript has been quiet for more than 600 s while a nested sub-agent's transcript is fresh. The row reads `Stalled` and the view is inactive, so the cell shows the yellow `1 stalled` alert while work is visibly happening in that worktree. This is not a 25-07 regression, because `Stalled` was never active. I could not confirm that a parent transcript stays untouched while its child runs, hence [inferred].
**Fix:** Treat a row as running if it or any of its children `is_running()`, in `is_active`, the stalled count and the `N agents` count. Alternatively, classify the row with `last_activity = max(own, children)`.

### IN-06: `plan_state` re-spells the running predicate instead of using `is_running()`

**File:** `src/agents/waves.rs:280-285`
**Issue:** 25-07 established `AgentLiveness::is_running` as the single place that says what "running" means (`mod.rs:77-91`, SUMMARY `patterns-established`). `plan_state` still writes `matches!(r.liveness, AgentLiveness::Live | AgentLiveness::Idle)`. If the running set is ever widened or narrowed, a plan's `Running` state and the view's activation will disagree.
**Fix:** `.any(|r| r.liveness.is_running())`.

### IN-07: The "gate runs before any git call or file read" assertion does not test ordering

**File:** `tests/agents_fixers.rs:498-515`
**Issue:** The comment claims to prove the gate precedes all I/O (threat T-25-31). The test only asserts that `estimate` returns `None` for a lone `Finished` row under a nonexistent root. Without the gate the same input returns `Some(FixerEstimate { fixers: 1, .. })`, so the test proves the gate exists. It does not prove that no git call or read runs first. Moving the gate below `own_subjects` would still pass.
**Fix:** Point `row.path` and `main_worktree` at a directory whose git call would observably fail differently, or reword the comment to "a lone finished fixer yields no estimate". The ordering itself is currently correct (`fixers.rs:222` precedes `:230`).

---

_Reviewed: 2026-09-26T04:36:07Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
