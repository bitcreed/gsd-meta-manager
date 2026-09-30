---
quick_id: 260929-szq
phase: quick-260929-szq
plan: 01
subsystem: agents observer + TUI (dashboard, Waves, Agents, Roadmap, help)
tags: [agents, worktrees, unmerged, roadmap, waves, help]
status: complete
requires: [agents scan (scan_project_with_probe), waves::derive, roadmap_model_for]
provides:
  - src/agents/unmerged.rs (UnmergedKey/UnmergedState/UnmergedItem, detect, short_ref, glyph/label consts)
  - ProjectAgents.unmerged / AgentView.unmerged + pending_unmerged/unmerged_plan/unmerged_in_phase/awaiting_merge_at
  - PaneState::Unmerged (was Leftover) with @ref +N ~D [▶ active] hint
  - PhaseStatus::Unmerged, PhaseFacts.unmerged, RoadmapModel::set_phase_status
  - help "Work State" legend
affects: [dashboard Status cell (d is now main-only), Agents sub-view state column (8 cells)]
tech-stack:
  added: []
  patterns: [path-level SUMMARY diff main vs worktree, names-only read_dir, scan-time list read by every UI surface]
key-files:
  created:
    - src/agents/unmerged.rs
    - tests/agents_unmerged.rs
    - .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh
  modified:
    - src/agents/mod.rs
    - src/agents/waves.rs
    - src/app.rs
    - src/ui/screens/detail.rs
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_view.rs
    - src/ui/screens/help.rs
    - tests/agents_waves.rs
decisions:
  - "I-1..I-14 as planned (see Inferred decisions); I-15..I-19 added during execution"
metrics:
  duration: ~30min
  completed: 2026-09-29
actuals:
  tokens: 26600
  tasks: 3
  commits: 5
plan_head_before: d877551f6e210e1bc0fad3818b2dabfd64e35449
plan_head_after: 0d6cca6302f7a3f19cd669fca72eb0a9fea8ed55
---

# Quick 260929-szq: show completed-but-unmerged worktree work distinctly from merged work

Path-level detection of SUMMARYs that exist in a linked worktree but in none of main's `phases/`, `milestones/*-phases/` or `quick/` dirs. Each hit is recorded with its holder worktree, `@ref`, `+N ~D` counts and InProgress/AwaitingMerge state. It is drawn as cyan `◐ unmerged` on the dashboard (`◐N`), in the Waves pane (`@a486395 +34 ~1 ▶ active`), in the Agents sub-view, on the Roadmap (`◐` row plus `◐k`) and in a new help legend.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 RED | a6fbc41 | test(quick-260929-szq): add failing unmerged-detection tests |
| 1 GREEN | 103e160 | feat(quick-260929-szq): detect unmerged worktree work and surface ◐N on the dashboard |
| 2 | ee84cea | feat(quick-260929-szq): unmerged state in the Waves pane and Agents sub-view |
| 3 | c189388 | feat(quick-260929-szq): roadmap unmerged status and Work State help legend |
| 3 fix | 0d6cca6 | fix(quick-260929-szq): the roadmap list row draws the unmerged lane glyph |

`mailbot-smoke.sh` is left uncommitted. It is a docs artifact in the quick dir, and the orchestrator commits it together with this SUMMARY.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 2853 passed, 1 failed, 15 ignored, across all 57 targets. The one failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, which is expected locally.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.
- The fixtures in `tests/agents_unmerged.rs` really ran; `plain_repo()` was not `None` in this sandbox. The proof is the RED run, where the squash fixture failed on a real git assertion (see deviation 1). There are 10 tests: (a)-(h), the guarded-scan end-to-end, and a no-worktree control.
- `tests/agents_scan.rs::no_file_under_src_agents_writes_reads_proc_or_spawns` and `tests/spawn_seam_guard.rs` are both green.
- **Real mailbot smoke: SKIP.** The script printed: "the agent-a486395e992a58454 worktree no longer holds phase 05's SUMMARYs (mailbot state changed)". Since planning, mailbot's main has merged phase 05 (05-01..05-05 SUMMARYs are in `.planning/phases/05-attachment-export/`), and that worktree is gone. The only worktrees left are a01a…, a5e3… and a834….
  - **Negative real-world check:** the release binary was run read-only against the real mailbot with a temp config and temp XDG dirs. Its row shows no `◐`, which is correct now that the work is merged.
- **Synthetic mailbot smoke: PASS, in both states.** The same script was run with `MAILBOT_DIR` pointing at a scratchpad git repo:
  - Main is a copy of the snapshot's `planning-mailbot/`. The locked worktree `.claude/worktrees/agent-a486395e992a58454` is a copy of `planning-agent-a486395e992a58454/`, committed, plus a dirty REQUIREMENTS.md. The snapshot was only read with `cp`; it was never pointed at or modified.
  - Dashboard: `mailbot … ◐5 unmerged`. Roadmap: `◐ │ 5 Attachment Export 0/5 W2`. Waves: `◐ unmerged 05-01 @a486395 +1 ~1 ▶ active`.
  - After `git worktree unlock` on the synthetic repo, the same captures appear without `▶ active`.
  - **`▶ active` appeared while locked.** The lock reason was not a parseable owner, so liveness was Unknown, and locked + non-Ended gives InProgress.

## Deviations from Plan

### Auto-fixed Issues

1. **[Rule 1 - test bug] The squash fixture (b) produced a byte-identical commit.**
   - Found during: Task 1 GREEN.
   - Issue: main's copy commit had the same parent, tree, author, message and second as the worktree's commit, so it got the same sha. As a result the "still ahead" control read `+0`.
   - Fix: main commits the squash under its own message (`squash: phase 13 (#1)`).
   - Commit: 103e160.
2. **[Rule 1 - intended value change] `tests/agents_waves.rs::a_summary_committed_in_the_worktree_reads_finished_before_the_merge` now expects the new values.**
   - The idle forms are `["◐1 unmerged", "◐1"]`.
   - The executor form is `… 1/3 done · ◐1 unmerged`; it was `2/3 done`.
   - The unit test `finished_agents_count_as_done_plus_unmerged` (renamed `finished_agents_are_counted_apart_from_done`) now expects `8/35`; it was `10/35`.
   - These changes are intended per UX-SPEC (d) and I-7.
   - Commits: 103e160, a6fbc41.
3. **[Rule 1 - test collision] `help.rs::the_help_screen_has_no_driver_heading_and_no_injection_legend_when_off` asserted that the bare injection glyphs never appear with the flag off.**
   - The driver's `GLYPH_DELIVERED` is `◐` and `GLYPH_ACTED_ON` is `●`, the same as the Work State legend's unmerged/done glyphs.
   - The check now matches the legend's `{glyph} {label}` form. The existing label check still catches any surviving injection row.
   - Commit: c189388.
4. **[Rule 1 - bug found by smoke] The Roadmap list row kept the old lane glyph.**
   - `layout_list` bakes each status glyph into the row's `lanes` string. The I-11 post-pass changed `facts.status`, so the detail pane and style changed, but the list still drew `○`.
   - Fix: added `RoadmapModel::set_phase_status`, which re-statuses a phase and patches its lane glyph at cell `2*lane`. The post-pass now goes through it, and the test asserts the lane glyph.
   - Commit: 0d6cca6.
5. **[Rule 3 - clippy]** `unnecessary_to_owned` in the Waves hint (`fit_cells(item.short_ref.shown().as_ref(), 14)`). Commit: c189388.
6. **[Rule 3 - smoke driveability] `tmux send-keys Enter` arrived as a line feed.**
   - The TUI read it as Ctrl+J and typed `j` into the filter.
   - The script sends `C-m` instead and waits 2 s for the app to start before the first key.
7. **TDD ordering.**
   - Task 1 had a separate RED commit: the integration, waves and app tests. The `unmerged.rs` unit tests shipped with the module in GREEN, because they cannot exist before the module.
   - For Tasks 2 and 3, the behaviour tests were written in the same commit as the implementation, and passed on the first run. The exceptions are the lane-glyph assertion and the help collision above, which failed first.
8. **Formatting.** `rustfmt` was applied to the two new files only. The repo as a whole is not rustfmt-clean, and CI does not enforce it.

## Inferred decisions (audit)

All of these are [inferred]: the human was unavailable. I-1..I-14 are copied from the plan. I-15 onward were decided during execution.

- **I-1 Detection is path-level.** A plan is unmerged when a worktree `*-SUMMARY.md` key is not among main's SUMMARY keys.
  - Commit ancestry is rejected.
  - Main is `core.main_worktree`, falling back to `project_root`.
  - A main SUMMARY always wins, so squash and rebase merges clear. Fixture (b) proves this.
- **I-2 Scope.** Every non-prunable, non-main worktree is scanned, agent or not. Main's keys are not read when no such worktree exists.
- **I-3 Step (d) is deferred.** PLAN-only keys never flag, and there is no new git helper. `worktree_counts` is called only for a holder that is not already an agent row.
- **I-4 One item per key, and the state rule.**
  - The representative is the first holder by path.
  - The state is InProgress iff some holder's row is running, or is locked with liveness other than Ended. Otherwise it is AwaitingMerge.
  - A human worktree is always AwaitingMerge.
  - Dirty state never suppresses an item.
- **I-5 Quick keys.** The key is `YYMMDD-xxx`, validated as 6 digits, `-`, then 3 `[a-z0-9]`, followed by the end or `-`.
  - Main's set = ids with a SUMMARY in `quick/<id>-*/`.
  - A worktree quick dir whose id main holds is skipped unlisted.
  - `quick-batches/` is ignored.
- **I-6 Known limits, accepted.**
  - A SUMMARY that main deleted outright still flags.
  - A renumbered phase that collides with an archived key reads as merged.
  - `milestones/*-phases/*/` counts as main.
- **I-7 Dashboard ladder.**
  - `d` counts main only, and N counts every item in both states.
  - An active ladder becomes `forms[0] · ◐N unmerged`, then each form `· ◐N`, then the last form bare.
  - Idle: `◐N unmerged`, then `◐N`.
  - Idle with stalled rows: `◐N unmerged · S stalled`, then `◐N · S stalled`, then `S stalled`.
- **I-8 Waves.**
  - A listed plan is `PlanState::Finished`. `PaneState::Leftover` is renamed `Unmerged`.
  - Non-active phases show Unmerged for listed plans.
- **I-9 Agents sub-view.** A holder row of AwaitingMerge work reads `unmerged` in cyan, while an InProgress holder keeps its liveness word. The state column is now 8 cells (`AGENT_STATE_CELLS`).
- **I-10 short_ref.**
  - Strip `refs/heads/`, `worktree-` and `gsd/`.
  - `agent-<id>` becomes 7 chars of the id, extended until unique.
  - A detached HEAD uses the dir name.
  - The result is Untrusted, drawn as `@` + `shown()` + `fit_cells(14)`.
- **I-11 Roadmap post-pass.** The status becomes Unmerged when all of these hold:
  - the phase is not Done and not shipped;
  - k > 0;
  - `t > 0 && d + k >= t`.

  Unmerged wins over Active/Ready/Blocked. Dependents keep their laid-out status. `◐k` appears only in the detail status line.
- **I-12 Help.** The `Work State` block goes right after `Dashboard Badges`, with every glyph interpolated from constants.
- **I-13 A locked worktree with complete SUMMARYs is a combined state.** Both states render `◐ unmerged`, and InProgress adds a yellow `▶ active`. This deviates deliberately from UX-SPEC (a)'s "running > unmerged". A plan with no SUMMARY and a live agent still reads `▶ running`.
- **I-14 The smoke is read-only.** It uses a temp `--config` and temp XDG dirs, a `test -e`/`ls` precondition and navigation keys only, runs no git against mailbot, and never uses the snapshot as a live repo.
- **I-15 (new) `short_ref`'s `agent-<id>` shortening also applies to a detached HEAD's directory name** (`agent-…` dir → 7-char id), so a detached agent worktree reads `@a486395`, not a 14-cell-truncated dir name.
- **I-16 (new) Unwaved active phases.** For an active phase with no `wave:` metadata, the plan universe of `derive` now also includes the listed unmerged plans of that phase, so their counts and states stay consistent with the Waves pane.
- **I-17 (new) Start-now list.** A phase the post-pass marks Unmerged leaves the Roadmap's `start_now` list. Its remaining work is done, so it is not "start now".
- **I-18 (new) Lane glyph.** The Unmerged status is applied through `RoadmapModel::set_phase_status`, which also patches the baked lane glyph (deviation 4).
- **I-19 (new) Mailbot changed state, so the smoke was proven on a synthetic copy.**
  - Because the real mailbot has since merged phase 05, the real smoke SKIPs.
  - Real-world evidence came instead from the same script, run with `MAILBOT_DIR` pointing at a scratchpad git repo built from copies of the snapshot's main and worktree `.planning/` trees, in both locked and unlocked states.
  - A read-only run against the real mailbot served as the negative check.
  - `MAILBOT_DIR` was added as an override for this purpose. It defaults to `~/projects/python/mailbot`.

## Deferred (per plan, not built)

- the `//u` filter
- the Roadmap detail "Worktrees" block
- the ASCII box-view `^` marker / `PhaseMarker::Unmerged`
- step (d), the PLAN-only `diff --diff-filter=A` guard
- a per-worktree (HEAD sha, dir mtime) scan cache
- the `~N stray` Agents line
- the full branch+path of the selected row in the status line
- rendering quick tasks anywhere other than the dashboard count
- splitting the dashboard `◐N` by state

## Known Stubs

None.

## Self-Check: PASSED

- The files exist: `src/agents/unmerged.rs`, `tests/agents_unmerged.rs` and `mailbot-smoke.sh`.
- The commits a6fbc41, 103e160, ee84cea, c189388 and 0d6cca6 are all present in `git log`.
