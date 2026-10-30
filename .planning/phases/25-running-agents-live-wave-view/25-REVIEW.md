---
phase: 25-running-agents-live-wave-view
reviewed: 2026-10-29T00:00:00Z
depth: standard
review_kind: incremental re-review at HEAD 0a2ef70, diff base 9faf0be (the prior full re-review, taken at 54a73bf); the 22 files are everything changed in `git diff 9faf0be..HEAD`, and only the quick 261029-1dp slice (ce86206) is Phase 25-owned
files_reviewed: 22
files_reviewed_list:
  - docs/CONFIGURATION.md
  - docs/DRIVER-SANDBOX.md
  - docs/GSD-CORE-SYNC.md
  - src/agents/adapters/mod.rs
  - src/config.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/executor/mod.rs
  - src/launcher/config_dir.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/launcher/stop.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 3
  info: 16
  total: 19
status: issues_found
---

# Phase 25: Code Review Report (incremental re-review at HEAD 0a2ef70)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 22 (this pass). The 24 files of the prior full pass (9faf0be / 54a73bf) are listed under "Prior review (9faf0be)" below and are verified here by `git diff --stat`, not re-read.
**Status:** issues_found (no new findings; 19 carried findings remain open)

## Narrative Findings (AI reviewer)

## Summary

This file is the union of two passes. The first is the full HEAD re-verification at 54a73bf (review commit 9faf0be). It is preserved verbatim below under "Prior review (9faf0be)", with every finding kept: CR-01, WR-01..WR-09 and IN-01..IN-16. The second is this incremental pass over `git diff 9faf0be..HEAD`. The scope resolver returned `status: degraded, source: phase-range`, and `git log` shows no Phase 25 task commit since the prior review. So the 22-file window is not Phase 25 work. It is quick 261028-t3c / 261028-v2z (driver lock and direction hardening, `driverctl` store and serve, the managed-settings merge) and quick 261029-1dp (`f4cfa01`, the SAFE-04 redacting capture pump; `ce86206`, removal of the dead `AgentRuntime` config key and doc-comment corrections).

**What touches Phase 25's own surface.** Three hunks, none of them behavioural.
- `src/agents/adapters/mod.rs:12-17`. The doc paragraph "Why `Box<dyn>`" used to cite `src/executor/runtime.rs`. That file is gone. The new text says "the enum dispatch the retired executor used for its runtime value (removed in quick 261029-1dp)". I checked it against `git show 9faf0be:src/executor/runtime.rs`. `AgentRuntime` was an `enum` with `match` arms in `as_str`, `program` and `from_config_value`, so the rationale ("every new variant there was an edit to every `match`") was true and is stated correctly. The comment still reads well, and the D-A07 argument it makes (one module plus one registration line) rests on `registered_adapters`, which is unchanged. No stale pointer remains in `src/agents/**`: `grep executor src/agents` finds only the unrelated `executor_mode` and "thirteen live executors" wave-model names.
- `src/ui/screens/detail.rs:21257-21277`. This is a test doc comment and an assertion message in the Driver-tab opt-in test. It is far from every range this phase cited in `detail.rs` (`:3233-3253`, `:9723-9830`, `:10797-11110`), and the hunk is net +1 line after all of them. No Phase 25 line citation shifted.
- `src/ui/screens/mod.rs:1389-1393`. This is the doc comment on `AppContext::experimental`. It is not in the agents or wave-view path.

**Did the agents scan, adapters, wave view or Agents sub-view regress?** No. `git diff --stat 9faf0be HEAD` over `src/agents/**`, `src/app.rs`, `src/state_reader/git_ops.rs`, `src/ui/screens/normal.rs`, `src/ui/screens/help.rs` and `tests/agents_*.rs` shows exactly one changed file, `src/agents/adapters/mod.rs`, and the change is 3 insertions and 2 deletions, all `//!` lines. I also ran the targeted suites at HEAD with `--no-fail-fast`: `agents_claude` 13, `agents_fixers` 10, `agents_process` 7, `agents_scan` 13, `agents_unmerged` 10, `agents_waves` 9, `agents_workstreams` 4, `agents_worktree_plans` 9 passed with 1 ignored, and the `agents::` library filter 82. All passed, none failed. I checked the dangling-reference risk the `AgentRuntime` removal creates. `grep` for `AgentRuntime`, `executor::runtime`, `executor/runtime`, `UnrecognizedRuntime`, `::default_runtime` and `.runtime()` across `src`, `tests`, `docs` and `README.md` finds only the three deliberate "removed in 261029-1dp" prose mentions (`src/executor/mod.rs:13`, `src/config.rs:66`, `src/registry.rs:2363`) and the stale assertion message already recorded elsewhere (below). The legacy `runtime` and `default_runtime` keys are still round-tripped through the flattened `extra` maps: `src/ui/screens/preferences.rs:228-240` asserts that `default_runtime` survives a save.

**Did any prior finding resolve or move?** None. Every file that carries a carried-forward finding is unchanged since 9faf0be, and I re-grepped the sites that are cheap to check. WR-07 is still live: the handler's skip list at `src/app.rs:2221-2227` still omits `worktree_phases`. IN-08 is still live: `src/agents/mod.rs:12` still says `git_ops::git_read_raw`, `src/agents/worktrees.rs:9` still says `git_read_raw`, and `src/agents/unmerged.rs:31` still says U+25D0 above a constant of `"\u{25D2}"`. All line numbers cited in the prior review remain exact. [inferred — cleanup 2026-10-28]

**New findings:** none. The rest of the delta was read for anything that could reach the agents scan or the wave view. It could not, since none of it is on that path (`driver/lock.rs` and `direction.rs`, `driverctl`, `launcher/**`, the registry, the config reader). Every defect I found in it is already recorded by an earlier re-review of the same 22-file window, so by the task's rule I cross-reference them and do not re-raise them as Phase 25 findings:
- Stale `FIVE subjects` / `UnrecognizedRuntime` assertion message at `src/registry.rs:2368-2371`, still stale after 261029-1dp even though the count beside it is `* 4`: Phase 29 IN-09, Phase 28 (`28-REVIEW.md:383`), Phase 31 (`31-REVIEW.md:107`).
- Multi-line PEM unredacted by the line pump: Phase 29 WR-14, Phase 31 WR-01. Pump-thread spawn failure leaves the child unsupervised: Phase 29 WR-15, Phase 31 WR-02. Pump launcher-death rationale: Phase 30 WR-16.
- Read-side link-count refusal turns a held lock into `Unknown`: Phase 29 WR-08. First-doc-wins for managed `permissions`: Phase 29 WR-09, Phase 30 WR-14. Hook-item validation: Phase 30 WR-15.
- Everything else in the Phase 24 re-review list (`24-REVIEW.md`, IN-06 and IN-07 and its "Already recorded elsewhere" list).

The one user-visible claim in the window that I verified independently is the new `is_opted_in` doc (`src/registry.rs:955-960`) and the `AppContext::experimental` doc: "no launch is gated on the record; it feeds only `launcher::push::policy_for`". `grep is_opted_in\|driver_opt_in` over non-test `src` finds no launch gate, and `REQUIREMENTS.md:24,50,332` record CTRL-03 as superseded by user decision, so the docs agree with the code.

No security issue was found in the Phase 25 scope. Findings counts in the frontmatter count only open findings, as before: 3 warnings and 16 infos, all carried, none new.

## Prior Findings Status

Verdicts are at HEAD 0a2ef70. "Unchanged" means `git diff --stat 9faf0be HEAD -- <file>` is empty, so the finding and its line numbers stand as written below.

| ID | Verdict | Evidence at HEAD 0a2ef70 |
|----|---------|--------------------------|
| CR-01 | Resolved (unchanged) | `src/agents/mod.rs` unchanged since 9faf0be. `AgentLiveness::is_running` (`mod.rs:104-106`), age-out at `:157-159`, consumers `waves.rs:655`, `:425-427`, `fixers.rs:289`. |
| WR-01 | Resolved (unchanged) | `src/agents/waves.rs` unchanged. Tiered vote at `waves.rs:411-427`. |
| WR-02 | Resolved for one hang; aggregate open as WR-08 | `src/state_reader/git_ops.rs` unchanged. `git_read_raw_within` `:243-256`, `AGENT_GIT_BUDGET` `:235`. |
| WR-03 | Resolved (unchanged) | `src/agents/fixers.rs` unchanged. `is_code_review_name` `:172-175`. |
| WR-04 | Resolved (unchanged) | `fixers.rs:237-245` intersect and clamp. |
| WR-05 | Resolved (unchanged) | `src/agents/adapters/claude.rs` unchanged. `project_dirs` `:282-307`. |
| WR-06 | Resolved (unchanged) | `fixers.rs:318-342` running-first vote. |
| WR-07 | Open, still present (re-grepped) | `src/app.rs` unchanged. `app.rs:2221-2227` skip list still lacks `worktree_phases`; `grep worktree_phases src/app.rs` is still empty. |
| WR-08 | Open (unchanged) | `src/agents/mod.rs`, `unmerged.rs`, `git_ops.rs`, `app.rs` unchanged. No scan-level deadline added. |
| WR-09 | Open (unchanged) [inferred] | `src/agents/fixers.rs` unchanged. `fixers.rs:352-353` still resolves against the project `.planning` only. |
| IN-01 | Open (unchanged) | `app.rs:2195-2196`, `:2221-2248` unchanged. |
| IN-02 | Open (unchanged) | `tests/agents_scan.rs:565`, `tests/agents_unmerged.rs:315`, `app.rs:5506`, `mod.rs:640` unchanged. |
| IN-03 | Open (unchanged) | `src/agents/worktrees.rs` unchanged. `:395-398`. |
| IN-04 | Open (unchanged) [inferred] | `claude.rs:342` unchanged. |
| IN-05 | Open (unchanged) [inferred] | `waves.rs:654-656`, `:703-707`, `:740-745` unchanged. |
| IN-06 | Open (unchanged) | `waves.rs:323-326` unchanged. |
| IN-07 | Open (unchanged) | `tests/agents_fixers.rs` unchanged. `:557-573`. |
| IN-08 | Open, still present (re-grepped) | `src/agents/mod.rs:12`, `worktrees.rs:9`, `unmerged.rs:31-32` read at HEAD: still name `git_read_raw` and U+25D0. The one agents file touched since 9faf0be (`adapters/mod.rs`) is not one of the three sites. |
| IN-09 | Open (unchanged) | `src/agents/workstream_scope.rs` unchanged. `:66-71`. |
| IN-10 | Open (unchanged) | `mod.rs:503-520` unchanged. |
| IN-11 | Open (unchanged) | `src/agents/worktree_phases.rs` unchanged. `:95-122`. |
| IN-12 | Open (unchanged) | `tests/agents_process.rs`, `tests/agents_unmerged.rs` unchanged. |
| IN-13 | Open (unchanged) | `tests/agents_scan.rs:595-614` unchanged. |
| IN-14 | Open (unchanged) | `waves.rs:688-697`, `src/ui/screens/normal.rs:524-528` unchanged. |
| IN-15 | Open (unchanged) [inferred] | `worktrees.rs:240-251`, `waves.rs:53-56,86-92`, `phase_num.rs` unchanged. |
| IN-16 | Open (unchanged) [inferred] | `src/agents/processes.rs:104-114` unchanged. |

### Prior review (9faf0be)

The text below is the 9faf0be report, kept verbatim. Its "Prior Findings Status" table is superseded by the table above, which extends it to every id.

#### Summary (as of 9faf0be)

Prior review (5579bbc): CR-01 and WR-01 were resolved by plan 25-07. WR-02 through WR-06 were fixed by commits 4728936, 7efe219, 3a2b838, 6b5e034 and 6d0b5a3. This re-review verified every fix against source and found no regression. No critical issue was found.

The code added since then is mostly sound. That covers unmerged detection, workstream scoping, worktree plans and phases, and the Linux-only process evidence. It adds three warnings:
- **WR-07.** A project whose only agents-scan output is worktree-only phases never gets a view, so the feature added in 23a947f is invisible in that case.
- **WR-08.** The WR-02 fix bounds each git call but not the scan, so a wedged filesystem still freezes the views for minutes.
- **WR-09.** The fixer estimate was not made workstream-aware when everything beside it was.

All seven prior info findings (IN-01..IN-07) are still present, with updated locations below. Nine new info items follow.

Files reviewed in that pass (24): `src/agents/**` (`mod.rs`, `adapters/mod.rs`, `adapters/claude.rs`, `fixers.rs`, `processes.rs`, `unmerged.rs`, `waves.rs`, `workstream_scope.rs`, `worktree_phases.rs`, `worktree_plans.rs`, `worktrees.rs`), `src/state_reader/git_ops.rs` (agent-worktree helpers `:199-256`, `:600-726`), `src/app.rs`, `src/ui/screens/normal.rs`, `src/ui/screens/help.rs`, `src/ui/screens/detail.rs`, and the eight `tests/agents_*.rs` files. The `src/agents/**` files, `git_ops.rs` and the tests were read in full or in their production sections. `app.rs`, `normal.rs`, `help.rs` and `detail.rs` were reviewed only at the agents and wave-view call sites. Those are `app.rs:1822-1870` and `:2188-2249`, `normal.rs:506-530` and `:1129-1163`, `help.rs:151-190`, and `detail.rs:3233-3253`, `:9723-9830` and `:10797-11110`.

## Critical Issues

None.

## Warnings

### WR-07: A project whose only agents-scan output is worktree-only phases gets no view, so its ghost phases and `N wt phases` tag never show (new)

**File:** `src/app.rs:2221-2227`; consumers `src/agents/waves.rs:688-697`, `src/ui/screens/detail.rs:3237-3253`
**Issue:** The `AgentsScanned` handler skips a project (`continue`) unless one of `rows`, `worktreeless`, `unmerged`, `worktree_plans` or `workstreams` is non-empty. `worktree_phases` is not in that list. `grep worktree_phases src/app.rs` returns nothing, and commit 23a947f did not touch `app.rs`.

A worktree-only phase at the started, context or researched stage has a directory but no `*-PLAN.md`, so `worktree_plans` is empty. It is also not an agent row when its worktree lives outside `.claude/worktrees/` on a non-agent branch. For example:
```
git worktree add ../proj-p15 -b feature/p15
# then discuss-phase writes 15-CONTEXT.md there
```
Then `rows`, `unmerged`, `worktree_plans` and `workstreams` are all empty, and only `worktree_phases` is set. No `AgentView` is inserted. As a result:
- `AgentView::summary_forms` (`waves.rs:688-697`), which adds the `N wt phase(s)` tag, never runs for that project.
- `ghosts_of` (`detail.rs:3237-3253`) reads `view.worktree_phases` from `ctx.agent_views`, so the Roadmap shows no ghost node.

The workstream scope is unaffected, because its `WorkstreamAgents` entry makes `workstreams` non-empty. The project scope is the one that regresses. No test drives this through the handler. `tests/agents_workstreams.rs:229-268` uses a `.claude/worktrees/` path, which produces an agent row and masks the gap.
**Fix:**
```rust
if agents.rows.is_empty()
    && agents.worktreeless.is_empty()
    && agents.unmerged.is_empty()
    && agents.worktree_phases.is_empty()
    && agents.worktree_plans.is_empty()
    && agents.workstreams.is_empty()
{
    continue;
}
```
Add a handler test with a `ProjectAgents` holding only `worktree_phases`. Assert that `agent_views` gets an entry and that `summary_forms()` is `["1 wt phase"]`.

### WR-08: The per-call git budget does not bound the scan: a wedged filesystem still freezes every view for minutes (residual WR-02)

**File:** `src/agents/mod.rs:456-552` (rows) and `:556-558`, `:590-595`; `src/agents/unmerged.rs:280-283`; `src/state_reader/git_ops.rs:235,243`; `src/app.rs:1844-1870`
**Issue:** WR-02's fix kills one git call after 10 s, and that part is correct. The scan itself is sequential and has no overall deadline. Each agent row costs up to two counts calls (`worktree_counts`) and one `commit_subjects` call. Each non-agent holder costs two counts calls. Each fixer costs one `log_subjects` call. Fixers and `worktree_list_porcelain` add more, and the listing retries without `-z` after a timeout, so a timed-out listing costs 20 s. If the stuck resource is shared (one NFS or FUSE mount holding the worktrees, which is the realistic case), every call times out. With K worktrees the scan then takes roughly 10 s × 3K, so 15 worktrees means about 7 minutes. Meanwhile `agents_scan_in_flight` stays set (`app.rs:1854`, cleared only at `:2200`). The dashboard keeps the previous `AgentView`, whose `scanned_at` is fixed and whose ages are computed from it (`detail.rs:11091-11107`), so it still reads `live`. This is the WR-02 symptom, bounded in time but not in practice. [inferred: the multiplication assumes all calls hit the same wedged mount.]
**Fix:** Give the scan a deadline. Create `let deadline = Instant::now() + Duration::from_secs(30)` in `scan_projects_guarded` and thread a `&ScanBudget` into the git helpers, or into `worktrees::worktree_counts`, `commit_subjects` and the fixer reads. Once the budget is spent, or after N consecutive timeouts, return `None` and `Vec::new()` for the rest of the scan. Failing that, record `agents_scan_started: Instant` next to the flag. On the tick, treat a scan older than 60 s as lost: drop `agent_views` or mark them stale, and `tracing::warn!` once.

### WR-09: The fixer estimate reads the project's `.planning`, even though every neighbouring detector was made workstream-aware (new) [inferred]

**File:** `src/agents/fixers.rs:352-353` (and `:357-374`); contrast `src/agents/mod.rs:488-498` and `src/agents/waves.rs:372-394`
**Issue:** Quick 261023-05i scoped unmerged, worktree phases and plans, agent rows and the wave model per workstream. `fixers::estimate` was not touched. It resolves the phase directory with `find_phase_dir(&project_root.join(".planning"), …)` only, and `AgentRow::workstream` is `None` for fixers anyway, because `plan` is `None` (`mod.rs:488`). For a `code-review-fix` run on workstream phase 16 (`--ws v0-13-public-beta`), the estimate reads the project's `16-REVIEW.md` if the project has a phase 16. The total and the heading intersection come from the wrong review, and `fixed` is counted from `fix(16)` subjects of both scopes. If the project has no phase 16, the estimate silently degrades to `N fixers` with no counts. The dashboard's project cell shows the wrong `~f/t`. I have not confirmed how a `--ws` fix run labels its fixers, hence [inferred].
**Fix:** Resolve the scope the way `workstream_scope::resolve` does. Use the fixer description's tokens to pick a workstream that holds the phase, then call `find_phase_dir(&workstream_scope::scope_planning(&planning, ws), …)`. When the phase exists in several scopes and nothing names one, return the count-less estimate rather than guessing the project's. Add a fixture with phase 16 in the project and in a workstream.

## Info

### IN-01: The handler comment says a fleet without agents never redraws, but any inserted view redraws on every scan (carried forward)

**File:** `src/app.rs:2195-2196`, `:2221-2248`
**Issue:** Present. A view is inserted whenever any list is non-empty. `AgentView` carries `scanned_at`, so `agent_views != views` is true on every scan and `needs_redraw` is set about every 5 s. This now includes views that draw no age, such as an unmerged-only or worktree-plans-only view, and aged-out `Ended` rows. The comment is also stale, since it only mentions "agents".
**Fix:** Compare views with `scanned_at` ignored, redrawing only when something is on screen that shows an age (a `Live`, `Idle` or `Stalled` row). Otherwise, correct the comment.

### IN-02: Tests that go through `registered_adapters()` read the real `~/.claude` (carried forward; now three sites)

**File:** `tests/agents_scan.rs:565`, `tests/agents_unmerged.rs:315`, `src/app.rs:5506` (`the_agents_scan_rides_the_session_poll_counter`); cause `src/agents/mod.rs:640`
**Issue:** Present. `scan_projects_guarded` always builds `ClaudeCodeAdapter::from_env()`, against D-C17. The `app.rs` test also builds the real `process_probe()`, which spawns `pgrep`.
**Fix:** Add `scan_projects_guarded_with(projects, adapters, probe, now)`, delegating from the public function. Tests pass a tempdir-rooted adapter and `NoProcessProbe`.

### IN-03: The ledger read ignores relative `gitdir:` pointers (carried forward)

**File:** `src/agents/worktrees.rs:395-398`
**Issue:** Present. `!admin.is_absolute()` returns `None`. With git >= 2.48 `worktree.useRelativePaths`, tier-5 ledger attribution never fires.
**Fix:** `let admin = if admin.is_absolute() { admin } else { worktree.join(admin) };` before the `is_dir` check.

### IN-04: `lock_released` reports "released" for worktrees the runtime may never have locked [inferred] (carried forward)

**File:** `src/agents/adapters/claude.rs:342`
**Issue:** Present. `Some(worktree.locked.is_none())` makes a never-locked, quiet-for-more-than-120-s worktree `Finished`. `Finished` never activates the view (`waves.rs:655`) or the fixer gate (`fixers.rs:289`). It also reads as `AwaitingMerge` in `unmerged::item_state` (`unmerged.rs:226-229`). A long build or test run inside a never-locked worktree vanishes from the Status cell until it writes again.
**Fix:** Report `Some(true)` only for worktrees the adapter knows Claude locks (under `<project_root>/.claude/worktrees/`), else `None`.

### IN-05: A worktree's live children never count as running (carried forward) [inferred]

**File:** `src/agents/waves.rs:654-656` (`is_active`), `:703-707` (stalled form), `:740-745` (agent count)
**Issue:** Present. A quiet parent with a fresh child reads `Stalled`, so the cell shows the yellow `1 stalled` while work is visibly happening.
**Fix:** Treat a row as running if it or any child `is_running()`, in all three places.

### IN-06: `plan_state` re-spells the running predicate (carried forward)

**File:** `src/agents/waves.rs:323-326`
**Issue:** Present. `matches!(r.liveness, Live | Idle)` duplicates `AgentLiveness::is_running`. The vote at `:425` and `unmerged.rs:226` already use the predicate.
**Fix:** `.any(|r| r.liveness.is_running())`.

### IN-07: The "gate runs before any git call or file read" test does not test ordering (carried forward)

**File:** `tests/agents_fixers.rs:557-573`
**Issue:** Present. The test proves only that a lone `Finished` fixer yields `None`. Moving the gate below `own_subjects` (`fixers.rs:292`) would still pass, because the nonexistent root makes git fail to `Vec::new()` either way.
**Fix:** Reword the comment to "a lone finished fixer yields no estimate". Or inject a git-failure side effect the test can observe.

### IN-08: Doc comments name the wrong git helper and the wrong glyph (new)

**File:** `src/agents/mod.rs:11-13`, `src/agents/worktrees.rs:5-10`, `src/agents/unmerged.rs:31-32`
**Issue:**
- `mod.rs` and `worktrees.rs` say every git call goes through `git_ops::git_read_raw`. Since WR-02 the four agent-scan calls go through `git_read_raw_within`. The no-lock property is shared through `git_read_command` (`git_ops.rs:213`), but the 10 s kill is part of the contract now and is undocumented there.
- `unmerged.rs:31` documents `GLYPH_UNMERGED` as "U+25D0", but the constant is `"\u{25D2}"` (`:32`), and the tests pin `\u{25D2}`.

**Fix:** Update the three comments. Name `git_read_raw_within` and the budget, and say U+25D2.

### IN-09: A workstream named at the end of a sentence is not recognised as named (new)

**File:** `src/agents/workstream_scope.rs:66-71`
**Issue:** `names_token` splits on non-identity characters, and `.` is an identity character (`text.rs:251`). In `"Execute plan 1 of phase 16 for v0-13-public-beta."` the token is `v0-13-public-beta.`, which is not equal to the name. The naming tier silently falls through to evidence or uniqueness, and then probably to the project. The tier-1 attribution (`waves.rs:214-216`) already strips this punctuation for plan ids.
**Fix:** Compare `token.trim_end_matches(['.', '-', '_']) == name` as well as the exact token. Add a case with a trailing period.

### IN-10: `summary_in_worktree` is always false for a plan whose phase directory exists only in the worktree (new)

**File:** `src/agents/mod.rs:503-520`
**Issue:** The phase directory is resolved from main's listing only (`:508`, comment at `:499-502`). A plan in a worktree-only phase (exactly the lanes-15 case that `worktree_plans.rs` was written for) has `relative == None`, so its SUMMARY is never found. `Finished` then arrives only through the 120 s staleness rule, and only if the lock is released. The same plan still reads `Finished` through `listed`/unmerged in `plan_state`, so the user-visible damage is small.
**Fix:** If main has no directory, fall back to `find_phase_dir(&wt_scope, …)` in the worktree's own tree.

### IN-11: `stage_of` counts summaries and plans that the plan-pairing rules exclude (new)

**File:** `src/agents/worktree_phases.rs:95-122`
**Issue:** It excludes only names containing `-FIX-`. A `NN-GAPCLOSURE-SUMMARY.md`, a phase-level `NN-SUMMARY.md` or a `*-GAPCLOSURE-PLAN.md` still counts. `unmerged::summary_plan` (`unmerged.rs:98-106`), `worktree_holds_summary` and `workstream_scope::plan_entries` all exclude `-GAPCLOSURE-SUMMARY.md` and require a plan index. So `plans_done` can exceed `plans_total` on the ghost node (`3/2`), and a phase with a single gap-closure summary reads `executing`.
**Fix:** Share one `is_plan_summary(name)` predicate (plan index required, FIX and GAPCLOSURE excluded) between these modules. Clamp `done` to `plans`.

### IN-12: Integration tests silently pass when `git init` fails (new)

**File:** `tests/agents_process.rs:149-151,181-183,255-257`, `tests/agents_unmerged.rs:100-102` (the same `let Some(..) else { return; }` shape in every agents suite)
**Issue:** `plain_repo()` returns `None` when `git init` fails, and each test then returns green without a message. In a runner with no git, the unmerged, process-evidence and workstream suites would all report a pass while exercising nothing. This is the same shape as the hazard the project memory records for fail-fast hiding suites.
**Fix:** Print `eprintln!("SKIP: git unavailable")`. Or, better, assert in CI with `if std::env::var_os("CI").is_some() { panic!(..) }`.

### IN-13: The zero-write guard test cannot see several write paths (new)

**File:** `tests/agents_scan.rs:595-614`
**Issue:** The forbidden list covers `fs::write`, `File::create*`, `create_dir*`, `remove_*`, `rename(`, `OpenOptions`, `set_modified`, `set_permissions`, `write_all(` and spawns. It does not cover `fs::copy`, `fs::hard_link`, `symlink(`, `File::options()`, `set_len` or `fs::truncate`. `File::options().write(true)` bypasses the `Open`+`Options` token entirely. The `take_while(line != "#[cfg(test)]")` also stops at the first such line, so a production item placed after a `#[cfg(test)]`-gated `use` is not scanned. [inferred]
**Fix:** Add the missing tokens. Scan to the `mod tests` line rather than the first `#[cfg(test)]`.

### IN-14: The `N wt phases` tag defeats the "ends with `stalled` is yellow" rule (new)

**File:** `src/agents/waves.rs:688-697`, `src/ui/screens/normal.rs:524-528`
**Issue:** `summary_forms` inserts a wider form `"{first} · {tag}"` in front. For an inactive view whose first form is `S stalled`, the widest form becomes `S stalled · 1 wt phase`. That does not end with `stalled`, so it renders cyan (normal), while the narrower fallback `S stalled` renders yellow (alert). The colour alert depends on how wide the cell happens to be. The doc at `normal.rs:515` says yellow means all-stalled.
**Fix:** Key the colour off a structured flag (`view.is_stalled_only()`) rather than the suffix of the text. Or put the tag in front: `"{tag} · {first}"`.

### IN-15: Letter-suffixed plan ids are accepted by `worktrees.rs` and then abstained on everywhere else (new)

**File:** `src/agents/worktrees.rs:240-251` (`plan_id_re`, `valid_plan_id`, accepts `12A-01`); `src/agents/waves.rs:53-56,86-92` (`PlanRef::from_id`, digits only); `src/state_reader/phase_num.rs:34-47` (numeric only); `unmerged.rs`/`worktree_phases.rs` via `split_dir`
**Issue:** Commit debaf77 widened the branch and ledger plan id to `12A-01` (gsd-core 1.15.0). `BranchPlan.plan` and `ledger_plan` therefore carry such ids. `PlanRef::from_id` rejects them, `PhaseNum::parse` rejects `12A`, and `split_dir("12A-foo")` is `None`. A letter-suffixed phase thus gets no attribution, no unmerged detection and no ghost node. It fails closed rather than wrong, but the half-support is untested here and the widened regex is dead for the agents scan. [inferred: depends on whether letter-suffixed phase dirs exist in registered projects.]
**Fix:** Either document that the agents scan is numeric-only and revert the regex widening, or give `PhaseNum`/`PlanRef` a suffix segment and have `phase_key` canonicalise it.

### IN-16: An owner with no comparable start is judged dead when its cwd is outside the project roots [inferred]

**File:** `src/agents/processes.rs:104-114` (the `_ => match cwd` arm); roots built at `src/agents/mod.rs:422-427`
**Issue:** For a lock that carries no procfs start, a live owner is `OwnerGone`, so every agent of it reads `Ended`, whenever its cwd is not under the project root or a worktree. A Claude session launched from a parent directory (a monorepo root above the registered project, or `~`) that spawns agents into this project's worktrees would have its live agents hidden. Claude 2.1.283 writes `start T`, so the exact-start arm normally decides, and the cwd arm is the fallback for `(pid N)` locks only. I could not confirm which Claude releases write the start-less form, hence [inferred].
**Fix:** When the start is not comparable and the cwd is outside, return `Unknown`, not `OwnerGone`. Keep `Absent` and a mismatched start as the only death signals.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
