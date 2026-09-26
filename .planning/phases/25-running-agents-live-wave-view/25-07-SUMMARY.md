---
phase: 25-running-agents-live-wave-view
plan: 07
subsystem: agents
tags: [rust, liveness, orphan-worktrees, dashboard, gap-closure, cr-01, wr-01]

requires:
  - phase: 25-running-agents-live-wave-view
    provides: "classify_facts / MAX_AGENT_AGE_SECS (25-01), Claude adapter (25-02), waves::derive + AgentView (25-03), dashboard Status cell (25-04), fixers::estimate (25-06)"
provides:
  - "classify_facts consumes MAX_AGENT_AGE_SECS: activity older than a day reads Ended whatever the lock says"
  - "AgentLiveness::is_running (Live | Idle only)"
  - "running-only activation of the Status-cell summary; Finished counts only inside a running ladder"
  - "tiered active-phase vote: running rows first, Finished/Stalled only as fallback (WR-01)"
  - "running-fixer gate in fixers::estimate"
affects: [phase-25-verification, agent-view, dashboard-status-cell, fixer-estimate, deferred-prune-action]

plan_head_before: 2d5be488dc3eb8c2bd6bacde7261ad94031a2d23
actuals:
  tokens: 6900
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "One predicate (AgentLiveness::is_running) decides everything that switches a summary on; Finished is done work that counts only inside a running run"
    - "Tiered vote: a lower-evidence tier votes only when the higher tier is empty"

key-files:
  created: []
  modified:
    - src/agents/mod.rs
    - src/agents/waves.rs
    - src/agents/fixers.rs
    - src/ui/screens/normal.rs
    - tests/agents_claude.rs
    - tests/agents_waves.rs
    - tests/agents_fixers.rs

key-decisions:
  - "[inferred] An aged-out agent maps to the existing Ended variant, not a new Orphaned one; every consumer already treats Ended as over, and the row is still listed (D-C16)"
  - "[inferred] The bound is the existing MAX_AGENT_AGE_SECS (86 400 s of inactivity), strictly greater-than; no second threshold (D-C08)"
  - "[inferred] Finished never switches the summary on (amends RESEARCH Pattern 6 per 25-REVIEW CR-01 and D-C14's 'while agents are running'); between a wave's last finisher and the merge the cell shows normal status"
  - "[inferred] WR-01 folded in: the active-phase vote is tiered (running first, Finished/Stalled fallback, then STATE.md)"
  - "[inferred] The executor ladder needs a running row attributed to the active phase, and N agents counts running rows plus worktree-less agents only"
  - "[inferred] The running-fixer gate keeps is_active_fixer's Live|Idle|Finished set as the run's size; only the switch-on needs a running fixer"

patterns-established:
  - "Liveness activation rule lives in one method beside the thresholds (AgentLiveness::is_running)"

requirements-completed: [AGENT-01, AGENT-02, AGENT-04, AGENT-05, AGENT-06, AGENT-07]

coverage:
  - id: D1
    description: "An agent silent for more than MAX_AGENT_AGE_SECS reads Ended whatever its lock says; at exactly the bound it still reads Finished/Stalled; aged children read Ended"
    requirement: AGENT-04
    verification:
      - kind: unit
        ref: "src/agents/mod.rs#agents::tests::an_agent_silent_past_the_age_bound_reads_ended"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_released_lock_orphan_days_stale_reads_ended_and_is_not_active"
        status: pass
      - kind: integration
        ref: "tests/agents_claude.rs#a_locked_orphan_days_stale_reads_ended_not_stalled"
        status: pass
    human_judgment: false
  - id: D2
    description: "An orphan-only milestone-complete project renders a dashboard frame byte-identical to the no-view frame (v1.0 Complete), both aged-out and within-bound Finished; a within-bound stalled orphan still shows 1 stalled"
    requirement: AGENT-05
    verification:
      - kind: automated_ui
        ref: "src/ui/screens/normal.rs#ui::screens::normal::tests::an_aged_out_orphan_leaves_the_status_cell_byte_identical"
        status: pass
      - kind: automated_ui
        ref: "src/ui/screens/normal.rs#ui::screens::normal::tests::a_finished_orphan_within_the_age_bound_leaves_the_status_cell_byte_identical"
        status: pass
      - kind: automated_ui
        ref: "src/app.rs#app::tests::an_agents_scan_reaches_the_dashboard_status_cell"
        status: pass
    human_judgment: false
  - id: D3
    description: "Only Live/Idle rows (or a live worktree-less agent) switch the summary on; running rows win the active-phase vote and select the executor ladder; Finished still counts toward done inside a running ladder"
    requirement: AGENT-02
    verification:
      - kind: unit
        ref: "src/agents/mod.rs#agents::tests::only_live_and_idle_are_running"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#agents::waves::tests::a_finished_row_alone_never_switches_the_summary_on"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#agents::waves::tests::running_agents_outvote_orphans_for_the_active_phase"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#agents::waves::tests::finished_agents_count_as_done_plus_unmerged"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#a_summary_committed_in_the_worktree_reads_finished_before_the_merge"
        status: pass
    human_judgment: false
  - id: D4
    description: "The fixer estimate exists only while an unattributed gsd-code-fixer is Live or Idle; finished fixers still count inside a running run; the gate precedes any git call or file read"
    requirement: AGENT-07
    verification:
      - kind: integration
        ref: "tests/agents_fixers.rs#a_finished_fixer_run_yields_no_estimate"
        status: pass
    human_judgment: false
  - id: D5
    description: "Orphan rows are never hidden or pruned: they stay in ProjectAgents.rows with adapter and git facts; no write, /proc read or spawn under src/agents/"
    requirement: AGENT-01
    verification:
      - kind: integration
        ref: "tests/agents_claude.rs#a_released_lock_orphan_days_stale_reads_ended_and_is_not_active"
        status: pass
      - kind: integration
        ref: "tests/agents_scan.rs#no_file_under_src_agents_writes_reads_proc_or_spawns"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 07: Orphan Age Bound (CR-01 gap closure) Summary

**Agents silent for more than a day now read `Ended`, and only `Live`/`Idle` agents can switch on the dashboard summary, choose the active phase or turn on the fixer estimate. Leftover worktrees from aborted GSD runs no longer hide a project's real status, such as `v1.0 Complete`.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-09-26T04:20:44Z
- **Completed:** 2026-09-26T04:30:08Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- `classify_facts` consumes `MAX_AGENT_AGE_SECS`. The check `age > MAX_AGENT_AGE_SECS` returns `Ended` before the released-lock `Finished` rule. Children go through the same path via `classify_child`. The docs for `Ended`, `MAX_AGENT_AGE_SECS` and `classify_liveness` ("In order") are rewritten.
- New `AgentLiveness::is_running()` (Live | Idle). `AgentView::is_active`, the `N agents` count and `executor_mode` all use it. The private `is_active_liveness` is removed.
- The active-phase vote in `derive` is now tiered (WR-01): running rows vote first, `Finished`/`Stalled` rows only when nothing attributed runs, then STATE.md. Ties still go to the higher phase.
- `fixers::estimate` returns `None` unless a collected fixer is running. The check sits before the first `git_ops::log_subjects` call.
- Byte-identical dashboard regressions at 80 and 120 columns, for a 3-day-old orphan and for a 2-hour `Finished` orphan. Plus real-worktree tests through the Claude adapter.

## Task Commits

1. **Task 1: aged-out orphan tracer (age bound)**: `29cd4f3` (fix). Tests and implementation are in one commit because it is a tracer task. RED was run and observed before the edit.
2. **Task 2: running-only activation + tiered vote**: `98ff94e` (test, RED), `7ff3a71` (feat, GREEN)
3. **Task 3: running-fixer gate**: `4e12f08` (test, RED), `0ae0d61` (feat, GREEN)

**Plan metadata:** see the final `docs(25-07)` commit

## RED evidence (observed before each implementation)

- **Task 1** (`cargo test --lib` and `--test agents_claude`):
  - `an_agent_silent_past_the_age_bound_reads_ended`: `left: Finished, right: Ended` ("released").
  - `an_aged_out_orphan_leaves_the_status_cell_byte_identical`: `left: Finished, right: Ended` ("released lock, days stale").
  - `a_released_lock_orphan_days_stale_reads_ended_and_is_not_active`: `left: Finished, right: Ended`.
  - `a_locked_orphan_days_stale_reads_ended_not_stalled`: `left: Stalled, right: Ended`.
  - The rendered forms (`1 agent` / `2 agents` for released orphans, `1 stalled` for the held one) were not reached. The liveness assertions fail first. The forms follow from the pre-fix code path: `Finished` counted in `is_active_liveness`, and `Stalled` fed the inactive `N stalled` form.
- **Task 2:**
  - `only_live_and_idle_are_running`: `left: true, right: false` for `Finished`. `is_running` was stubbed with the old `Live|Idle|Finished` set so the test compiled and failed on its assertion.
  - `running_agents_outvote_orphans_for_the_active_phase`: `left: Some(PhaseNum([12])), right: Some(PhaseNum([13]))`.
  - `a_finished_row_alone_never_switches_the_summary_on`: fails at `!view.is_active()`.
  - `a_finished_orphan_within_the_age_bound_...`: fails at `!view.is_active()`.
  - `tests/agents_waves.rs::a_summary_committed_...`: fails at `!view.is_active()`.
- **Task 3:** `a_finished_fixer_run_yields_no_estimate`: `left: Some(FixerEstimate { fixers: 1, phase: Some(12), fixed: Some(1), total: Some(48) }), right: None`.

## Files Created/Modified

- `src/agents/mod.rs`: the age bound in `classify_facts`, `AgentLiveness::is_running`, and rewritten docs for `Ended`, `MAX_AGENT_AGE_SECS`, `classify_liveness` and `ProjectAgents.fixer_estimate`. Two new unit tests.
- `src/agents/waves.rs`: the tiered vote, running-only `is_active`, `executor_mode` and `N agents` count. The `derive`/`is_active`/`summary_forms` docs cite CR-01/WR-01. Two new pure tests.
- `src/agents/fixers.rs`: the running-fixer gate, and the module, `FixerEstimate.fixers` and `estimate` docs.
- `src/ui/screens/normal.rs`: helpers `milestone_complete_ctx`, `classified`, `orbit_view`, and two byte-identical dashboard tests.
- `tests/agents_claude.rs`: two real-worktree orphan tests.
- `tests/agents_waves.rs`: `a_summary_committed_in_the_worktree_reads_finished_before_the_merge` rewritten. A lone finished worktree is inactive; adding a live 13-03 worktree gives `P13 · w2/2 · 1 run · 2/3 done` and counts `(1, 1, 1)`.
- `tests/agents_fixers.rs`: `Scripted.lock_released` (`fixer()` keeps `Some(false)`) and `a_finished_fixer_run_yields_no_estimate`.

## Decisions Made

All decisions are marked [inferred] in the frontmatter `key-decisions` (unattended run). They match the plan's `<flagged_assumptions>` exactly. No new decisions came up during execution.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test fixture: the two-phase state marked 13-01/13-02 done in main**
- **Found during:** Task 2 GREEN
- **Issue:** My first `phases_12_and_13_state` reused `phase_13_state()`, where 13-01..13-08 are summarized in main. So the live agents on 13-01/13-02 counted as `Done`, not `Running`, and `running` read 0.
- **Fix:** Phase 13 in that helper is now its own 3-plan wave with nothing done. This keeps the plan's `13-01`/`13-02` ids and the `running == 2` assertion.
- **Files modified:** `src/agents/waves.rs` (test code only)
- **Verification:** `running_agents_outvote_orphans_for_the_active_phase` is ok. RED still held, because the pre-fix vote picks phase 12 on the first assertion.
- **Committed in:** `7ff3a71`

**2. [Process - inferred] RED evidence checker not usable for cargo output**
- `gsd-tools check tdd-red-evidence` parses node TAP output only. For a cargo run it reports `INVALID_RED (zero_tests_discovered)`, even though cargo shows the target test FAILED on its assertion.
- The RED gate was instead met by the cargo output above: the named target test failed on an assertion for the planned behaviour.
- The checker is required only for `type: tdd` plans, and this plan is `type: execute`.

**3. [Process] `is_running` stub in the Task 2 RED commit**
- A test calling a missing method would fail to compile, which is not a valid RED.
- The RED commit therefore added `is_running` with the old activation set (`Live|Idle|Finished`), so the test failed on its assertion. GREEN replaced the body.

---

**Total deviations:** 1 auto-fixed (Rule 1, test fixture) plus 2 process notes. **Impact:** none on scope. No production behaviour beyond the plan.

## TDD Gate Compliance

- Task 2: `test(25-07)` `98ff94e`, then `feat(25-07)` `7ff3a71`.
- Task 3: `test(25-07)` `4e12f08`, then `feat(25-07)` `0ae0d61`.
- Task 1 (tracer): one `fix(25-07)` commit `29cd4f3`. The tests were written and run RED before the implementation (evidence above) but committed together. The tracer task's commit protocol is "commit like auto". No REFACTOR commits were needed.

## Verification

- `cargo test --lib agents::`: 45 passed. `--lib ui::screens`: 407 passed. `--lib app::tests`: 76 passed (including `an_agents_scan_reaches_the_dashboard_status_cell`). `--lib ui::screens::detail`: 196 passed.
- `--test agents_claude`: 12/12. `--test agents_waves`: 7/7. `--test agents_fixers`: 7/7. `--test agents_scan`: 13/13 (including `no_file_under_src_agents_writes_reads_proc_or_spawns`).
- `cargo clippy -- -D warnings`: clean. The `--all-targets` errors predate this plan and are in untouched files; they are already in deferred-items.md.
- `cargo test --no-fail-fast`: 53 `test result:` lines, **2488 passed, 1 failed, 15 ignored**. The only failure is the known local witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. 2488 is the 25-06 floor of 2479 plus the 9 new tests.
- Gap truth check (25-VERIFICATION gaps[0].missing):
  - Item 1: `age > MAX_AGENT_AGE_SECS` is at `src/agents/mod.rs:130`, before the `lock_released == Some(true)` test at :133.
  - Item 2: the regression tests are the Claude-adapter orphan test and the two byte-identical dashboard tests.
- Prohibitions: no worktree is pruned, unlocked or written. The orphan rows stay in `ProjectAgents.rows`, as asserted in `tests/agents_claude.rs`.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CR-01 is closed and WR-01 is folded in. Phase 25 is ready for re-verification (`/gsd-verify-work 25`, or the verifier re-run).
- Deferred review findings (WR-02..WR-05, IN-01..IN-04) stay out of scope, per the plan's table.
- IN-04 interaction: a never-locked worktree silent for more than 120 s now reads `Finished` and no longer switches the cell on. This is unconfirmed; it would need a runtime that does not lock worktrees.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*

## Self-Check: PASSED
