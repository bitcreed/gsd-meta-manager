---
phase: quick-261023-05i
verified: 2026-10-23T12:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".planning/quick/261023-05i-agent-and-worktree-details-for-workstream-phases-wherever-ro/261023-05i-PLAN.md"
  - ".planning/quick/261023-05i-agent-and-worktree-details-for-workstream-phases-wherever-ro/261023-05i-SUMMARY.md"
  - "src/agents/mod.rs"
  - "src/agents/unmerged.rs"
  - "src/agents/waves.rs"
  - "src/agents/workstream_scope.rs"
  - "src/agents/worktree_phases.rs"
  - "src/agents/worktree_plans.rs"
  - "src/app.rs"
  - "src/ui/screens/detail.rs"
  - "tests/agents_workstreams.rs"
covered_digest: "v3:sha256:1c1b79ef3f48a6c45eba2dce597efe62b23ceed9466fe02cc751cc4335917762"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 7/7
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Quick 261023-05i: Agent and worktree details for workstream phases - Verification

**Goal:** Wherever root phases show agents-scan data (worktree badge, worktree-held plan counts, unmerged post-pass, Enter-to-agent and Sessions > Agents cross-jump in the Detail Roadmap and Phases tabs), show it for workstream phases too, scoped by (workstream, number) so the project's Phase N data never reaches a workstream's Phase N. Regression tests include the same-number collision (root P16 vs v0.13 P16).

**Re-verification:** Yes, on final master HEAD 981da0e. The earlier pass went stale because batch items 05j, 05l and 05m edited `src/app.rs` and `src/ui/screens/detail.rs` afterwards. All seven truths were re-checked against the current code. Result: no regression. The only change to `src/agents/` since 05i's last feature commit is cd0e92e (a tokenizer delegation to `crate::text::is_identity_char` in `workstream_scope.rs`, behavior-neutral).

Post-05i drift reviewed in `detail.rs`: 05l adds placeholder bands for empty workstreams, and 05m changes the workstream "active" phase in `selected_waves` to `focus_phase_number`. Neither touches the scoped view lookup. `agent_view_of` (detail.rs:9796) still returns `view.workstreams.get(&ws.name)` for workstream phases. `phase_badge_and_counts` is still fed the workstream's own view (3067). The ghost rows (2988), the unmerged post-pass (3145), `focus_agent_of_plan` (1887) and `focus_plan_in_phases` (1956) are intact. `app.rs` still gates on `agents.workstreams.is_empty()` (2996).

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A workstream phase shows the worktree badge `[<stage> @<ref>]`, the worktree's plan counts, and its plans in the Waves pane (`plans in worktree <name>`), and `e` opens the holder's file | VERIFIED | `phase_badge_and_counts` (detail.rs:3213) is shared by project and workstream nodes, and the workstream loop (3056-3067) feeds it the workstream's own view. `selected_waves` (1760) resolves the scope view through `agent_view_of` (9796) and `waves_edit` (2021) opens `wt.phase_dir`, or the workstream's own planning dir when main holds the plans. Test `sentriq_a_workstream_phase_shows_its_own_worktree_plans_badge_and_counts` passes (re-run). |
| 2 | Unmerged post-pass for workstream phases, plus ghost rows in the workstream's band | VERIFIED | The post-pass (detail.rs:3138-3200) splits the scoped key and reads `view.workstreams.get(dir)` per phase. Workstream ghosts are built against the workstream's own roadmap and matched by scoped key. Tests pass (re-run): `sentriq_workstream_unmerged_plans_mark_only_their_own_phase`, `sentriq_a_workstream_ghost_phase_sits_in_its_workstream_band`, and the real-git `a_workstream_worktree_finishing_plans_marks_its_own_phase_unmerged_end_to_end`. |
| 3 | Same-number collision, both directions (project P16 vs v0.13 P16, project 27 vs v0.14 27) | VERIFIED | Scan: `detect_in` is called once per scope with its own `.planning[/workstreams/<ws>]` rel (agents/mod.rs:565-567). `sentriq_the_projects_worktree_plans_never_reach_a_same_numbered_workstream_phase` and the swapped case in `sentriq_workstream_unmerged_plans_mark_only_their_own_phase` pass (re-run). The integration tests assert `scan.unmerged` is empty when only v0.14 finished work and that `unmerged_in_phase("16")` is 1 for the project and 0 for every workstream. |
| 4 | Cross-jump: Enter on a workstream plan row selects THAT workstream's agent, and Enter on the agent opens the workstream's phase (scoped key) | VERIFIED | `waves_activate` (1846) passes `model.workstream` into `focus_agent_of_plan`, which matches on `row.workstream == workstream` and plan. `agents_enter` (1927) passes the row's workstream to `focus_plan_in_phases`, which finds the phase by (workstream, number). Tests `sentriq_enter_on_a_workstream_plan_selects_that_workstreams_agent` and `sentriq_enter_on_an_agent_opens_its_own_scopes_phase` pass (re-run). |
| 5 | Agent rows resolve to a scope at scan time (naming, path evidence, single candidate, project), names only; `summary_in_worktree` checked in the resolved scope | VERIFIED | `workstream_scope::resolve` is called from the row construction (agents/mod.rs:491). It uses `read_dir` names only; `tests/agents_scan.rs` (13) and `tests/spawn_seam_guard.rs` (41) pass (re-run). Tests pass (re-run): unit `resolve_picks_the_scope_by_naming_then_evidence_then_uniqueness`, `resolve_leaves_an_ambiguous_plan_with_the_project`; integration `agent_rows_resolve_to_the_workstream_their_worktree_works_in`, `an_agent_description_naming_a_workstream_picks_it_among_candidates`. |
| 6 | Projects without workstreams are unchanged | VERIFIED | `resolve` returns `None` with no names; `derive` leaves `workstreams` empty without data; `the_projects_view_counts_only_its_own_rows_and_lists_all` and `a_workstream_row_never_switches_the_projects_executor_ladder` pass (re-run). |
| 7 | Gates: full suite, clippy, fmt | VERIFIED | Not re-run by this verifier (instructed not to). Cited from the coordinator's final run on 981da0e: 89 targets, 4200 passed, 1 failed (the expected `envelope::policy` git-version test), 24 ignored; clippy 0 warnings; fmt clean. |

**Score:** 7/7 truths verified, 0 behavior-unverified.

## Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/agents/workstream_scope.rs` | VERIFIED | 290 lines; `names`, `scope_planning` and `resolve` are used by mod.rs. |
| `src/agents/mod.rs` | VERIFIED | `AgentRow.workstream`, `ProjectAgents.workstreams` and `WorkstreamAgents`; per-workstream `detect_in` calls (565-567). |
| `src/agents/waves.rs` | VERIFIED | `AgentView.workstream` and `AgentView.workstreams`; `derive` and `derive_scope`; the executor-mode filter is scoped. |
| `src/agents/{unmerged,worktree_phases,worktree_plans}.rs` | VERIFIED | `detect_in` added; `detect` delegates with `.planning`. |
| `src/app.rs` | VERIFIED | The AgentsScanned emptiness check includes `agents.workstreams.is_empty()` (2996), pinned by `agents_scan_keeps_a_project_whose_only_worktree_work_is_in_a_workstream`. |
| `src/ui/screens/detail.rs` | VERIFIED | `agent_view_of` picks the scope's view; the Roadmap, Waves, `e`, `focus_agent_of_plan` and `focus_plan_in_phases` are scoped. |
| `tests/agents_workstreams.rs` | VERIFIED | 4 real-git tests, all pass (re-run). |

## Key Links

| From | To | Status |
|------|----|--------|
| `agent_view_of` | `AgentView.workstreams` (detail.rs:9804) | WIRED |
| `scan_project_with_probe` | per-scope `detect_in` (agents/mod.rs:565-567) | WIRED |
| row construction | `workstream_scope::resolve` (agents/mod.rs:491) | WIRED |
| `Action::AgentsScanned` | `agents.workstreams` (app.rs:2996) | WIRED |
| `focus_agent_of_plan` / `focus_plan_in_phases` | `AgentRow.workstream` | WIRED |

## Behavioral Spot-Checks (all re-run on 981da0e)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Integration suite | `cargo test --test agents_workstreams` | 4 passed | PASS |
| Workstream lib tests and neighbours | `cargo test --lib -- sentriq_workstream sentriq_a_workstream sentriq_enter_on sentriq_the_projects workstream_scope the_projects_view a_workstream_row derive_gives agents_scan_keeps resolve_` | 24 passed, 0 failed | PASS |
| Scan guards | `cargo test --test agents_scan --test spawn_seam_guard` | 13 + 41 passed | PASS |

## Anti-Patterns

No TBD/FIXME/XXX markers in `workstream_scope.rs` or `tests/agents_workstreams.rs`. No stubs found. The workstream views hold real scan data, not static values.

## Notes (informational, not gaps)

- The plan's premise "only agent-a3e0c1b holds v0.14 27-01..27-04" no longer holds on the live sentriq tree, because main now has those plans. Main wins by design, and the `@a3e0c1b` 0/4 case is covered by fixture tests.
- I-10: a workstream that exists only inside a worktree (not in main's `.planning/workstreams`) is not detected. This is a documented inferred decision and outside the item goal.

## Human Verification Required

None. The checks above are all programmatic.
