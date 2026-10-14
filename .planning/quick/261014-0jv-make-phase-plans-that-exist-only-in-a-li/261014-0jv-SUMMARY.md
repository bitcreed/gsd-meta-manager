---
phase: quick-261014-0jv
plan: 01
subsystem: agents / ui
tags: [worktree, phases-tab, roadmap, read-only]
status: complete
requires: [agents::worktrees::scan_worktrees, state_reader::disk_status::infer_disk_status]
provides: [agents::worktree_plans, AgentView::worktree_plans_for, Waves-pane worktree fallback, Roadmap worktree counts/badge]
affects: [src/ui/screens/detail.rs, src/app.rs]
key-files:
  created: [src/agents/worktree_plans.rs, tests/agents_worktree_plans.rs]
  modified: [src/agents/mod.rs, src/agents/waves.rs, src/agents/worktree_phases.rs, src/state_reader/disk_status.rs, src/app.rs, src/ui/screens/detail.rs, README.md]
requirements: [QUICK-261014-0jv]
commits: 3
plan_head_before: 6588b740b2018aa28e9a22f19d2c21532115e44d
plan_head_after: 14aa20c4e7ad564c28b9305ff856abdd67aeaaee
actuals:
  tasks: 3
  commits: 3
---

# Quick 261014-0jv: Worktree-only phase plans are browsable

Plans that live only in a linked git worktree (mailbot's `lanes-13` / `lanes-15`) now appear in the Phases tab's Waves pane under ` Waves 2 · 1/2 done · plans in worktree lanes-15 (gsd/phase-15) `, open with `e` from the holder's file, and are counted on the Roadmap as `d/t` with a `[<stage> @<ref>]` badge. Main's plans and archives always win.

## Commits

| Task | Sha | Title |
|------|-----|-------|
| 1 | 770d9ac | feat(261014-0jv): tracer - worktree-held phase plans fill an empty main phase in the Waves pane |
| 2 | a0d252d | feat(261014-0jv): rank several worktree holders, open the holder's PLAN, mark the phase list |
| 3 | 14aa20c | feat(261014-0jv): Roadmap counts and @ref badge for worktree-held plans |

## What now displays

- Phases tab, Waves pane title: widest ` Waves {n} · {d}/{t} done · plans in worktree {name} ({branch}) [+N more] `; then ` Waves · {marker} `, ` Waves · plans in worktree {name} `, ` Waves · worktree `. Flat (no wave metadata) drops the wave count. Main-sourced titles are byte-identical.
- Left phase list: the `Nw` marker follows the browsed (worktree) inference.
- `e` on a worktree-sourced plan row: `SuspendAndEdit(<holder>/.planning/phases/<dir>/<id>-PLAN.md)` under the T-2l4-02 parent-equality guard; missing file gives `No PLAN.md found for this plan`.
- Roadmap row/detail: `1/2` and `1/2 plans` instead of `—` / `plans TBD`; badge `[Executing 1/2 @phase-15]` instead of `[Empty]`. Main ROADMAP checklist counts still win; Unmerged status no longer double counts worktree SUMMARYs.

## Inferred decisions as implemented (ID-1..ID-12)

All twelve were implemented as written in the plan. Notes on the ones with implementation detail:

- ID-1: resolution is in `agents::scan_project_with_probe` (spawn_blocking, ~5 s poll), reusing the scan's single `CoreScan`. `ProjectState` / `phase_disk_statuses` / the driver router never see worktree plans.
- ID-2: no notify watcher; first paint waits for the first agents scan; the winners' PLAN/SUMMARY files are re-read each poll (no mtime memo).
- ID-3: main "has plans" = names-only in the scan (`PLAN.md`/`*-PLAN.md` in main's `phases/<id>-*`, or key archived under `milestones/*-phases/`), and `plan_count == 0` in the UI (`browsed_inference`).
- ID-4: rank by newest `*-PLAN.md` mtime (unreadable ranks oldest), tie to smallest worktree path, superseded-only winner falls through; `others` = candidates - 1 (a superseded candidate that lost still counts), shown as ` +N more`.
- ID-5: plan states in a worktree pane use the worktree's own inference (summarized = Done).
- ID-8: label = holder path's last component; branch = git short name; both `Untrusted`, stored pre-escaped in `WavesWorktree`.
- ID-11: `Eq` added to `PlanTokens`, `PlanMeta`, `DiskInference` (no float fields).
- ID-12: `worktree_plans` is the first agents detector that reads content, only via `infer_disk_status` on winners.

## Deviations from Plan

None - plan executed as written. Minor executor-level choices (not plan changes):

- `browsed_inference` also falls back to the worktree when main has NO `phase_disk_statuses` entry for the phase (not only `plan_count == 0`), so an absent main entry still resolves.
- Roadmap worktree badge omits the `[verified]`/`[inferred]` suffix `disk_suffix_spans` can add under `show_badges`; the plan specifies the badge as exactly `[<stage> @<ref>]`.
- `cargo fmt` was run before each commit (it reformatted the new test file and the new module).
- During execution `.planning/ROADMAP.md`, `.planning/STATE.md` and `.planning/phases/26..31-*` showed as modified/untracked in the working tree. They were not touched by this task (another process), and I committed none of them.

## Verification

- `cargo build`: exit 0. `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0. `cargo fmt --check`: clean.
- `rtk proxy cargo test --no-fail-fast`: 60 suites, 2982 passed, 1 failed, 16 ignored. The sole failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. `tests/agents_scan.rs::no_file_under_src_agents_writes_reads_proc_or_spawns` passed.
- New tests: `tests/agents_worktree_plans.rs` 9 pass + 1 ignored; `--lib worktree_plans` 15 pass (incl. the `app.rs` gate test).
- Mailbot `#[ignore]` run (once, read-only): `PASS: phases 13 and 15 resolve to their worktree plans` (13 -> `lanes-13` / `gsd/phase-13`, 5 plans; 15 -> `lanes-15` / `gsd/phase-15`, 7 plans).
- `git worktree list` shows only the main tree; fixtures live in tempdirs.

## Follow-ups (deferred, not done)

- ID-6: `AgentView.waves` is still derived from main's inference, so a Running agent in a worktree-sourced phase reads Queued/Planned in the pane.
- ID-10: `worktree_phases` alone still does not open an `AgentView`, so ghost nodes need another trigger; only worktree plans (and rows/unmerged) do.
- ID-2: no live watcher; updates ride the ~5 s agents poll.
- Edge accepted (ID-3): a main dir whose only plans are superseded shows nothing from a worktree.

## Known Stubs

None.

## Threat Flags

None - no new endpoints, auth paths or trust boundaries beyond the plan's threat model (T-0jv-01..07 mitigated as planned; hostile name/branch escape pinned by `worktree_plans_hostile_worktree_name_and_branch_reach_no_cell_unescaped`).

## Self-Check: PASSED

Created files exist (`src/agents/worktree_plans.rs`, `tests/agents_worktree_plans.rs`); commits 770d9ac, a0d252d, 14aa20c are ancestors of HEAD.
