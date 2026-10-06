---
created: 2026-10-02T03:09:08.172Z
title: Show worktree-only roadmap phases in project view
area: ui
severity: minor
files:
  - src/agents/unmerged.rs:1-21,57,144,165
  - src/agents/mod.rs:285,382,496
  - src/agents/worktrees.rs:307,428,490
  - src/agents/waves.rs:195,543
  - src/ui/screens/detail.rs:1592,2772,2905-2925
  - src/ui/roadmap_graph.rs:335,410
  - src/ui/roadmap_view.rs:976,1129
  - src/state_reader/mod.rs:45,108,268-275,501,530,752
  - src/watcher.rs:74
  - src/launch_target.rs:9,101
---

## Problem

When a project's roadmap is extended **only inside a linked worktree**, the
dashboard shows nothing about the new phases. Quick task 260929-szq made
*unmerged plan SUMMARYs* visible (`◐ unmerged`), but this is a different case.
The worktree adds whole phases: ROADMAP entries, REQUIREMENTS, phase dirs with
CONTEXT/RESEARCH/PLAN, and no SUMMARYs yet. Main's `.planning/` has none of
them, so:

- `parse_project_state` only ever reads `<registered root>/.planning`
  (`app.rs:711-715`, `:1061`, `:1142`, `:1498`). It never reads a worktree's
  ROADMAP/STATE/REQUIREMENTS.
- `unmerged::detect` keys only on `*-SUMMARY.md` names. A worktree phase that is
  planned but not executed produces nothing.
- Even when it does produce a key, roadmap layout nodes come only from main's
  `state.phases`. A key for a worktree-only phase raises the dashboard `◐N` count
  but has **no node** on the Roadmap. The `d+k>=t` rule (`detail.rs:2912`) also
  needs main's plan total `t`, which doesn't exist for such a phase.

Goal: show worktree-only phases (and their pre-execution progress) in the
project view, ideally in the same visual language as unmerged work (`◐`, with
branch/worktree attribution). Example: a ghost or overlay node "Phase 13 ·
researched · in agent-a8f6…". The dashboard row should also hint that
roadmap-level work is pending in a worktree.

### Reference snapshot: sentriq, 2026-10-01T22:07:54-05:00 (not reproducible later)

Repo `~/projects/flutter/sentriq` (Flutter; `.planning/config.json` has
`workflow.use_worktrees=false`). The worktree was created by **Claude Code's
Agent-tool isolation**, not by GSD, so GSD-side config can't be relied on to
predict worktrees.

`git worktree list --porcelain`:
```
worktree /home/blk/projects/flutter/sentriq
HEAD 7a4b461b6482225eac2a0ab10107ee85b29501bb
branch refs/heads/master

worktree /home/blk/projects/flutter/sentriq/.claude/worktrees/agent-a8f61b155146442ce
HEAD 613f7450aed19349a1b352aa6bd08509b0023089
branch refs/heads/worktree-agent-a8f61b155146442ce
locked claude agent agent-a8f61b155146442ce (pid 31956 start 84234)
```
- The lock file `.git/worktrees/<id>/locked` names a live `claude --resume` PID.
  The same gitdir has `CLAUDE_BASE` = merge-base `8c3ab573`.
- master...branch left/right = `37 8`: the branch is 8 ahead and 37 behind.
  Main executed 11.1 to 12/12 after the fork; the branch still shows 11.1 at
  5/12. **The two sides diverged both ways**, and both edited ROADMAP and STATE.
- Worktree dirty: only untracked `.planning/state.json`. All phase 12-15 files
  are committed on the branch.
- 16 stale `worktree-agent-a*` branches have no worktree attached. They must not
  show up.

Main tree `.planning/`: milestone v0.12 "Actuation Routines". ROADMAP phases
are 9 [x], 10 [x], 11 [ ], 11.1 [ ] (INSERTED), and backlog 999.1. STATE has
`current_phase: "11.1"`, `status: verifying`, `total_phases: 4`, 38%. The phase
dirs are 09, 10, 11, 11.1 and 999.1. **There is no mention of phases 12-15 or of
the worktree anywhere in main's `.planning/`.** The only signals are
`git worktree list` with its lock file and the directory
`.claude/worktrees/agent-…/`. (`.gsd/dispatch-isolation-sentinel.json` and
`.planning/milestone.lock` both refer to 11.1. The milestone.lock PID is dead and
stale.)

Worktree-only content, from branch log `master..branch`:
```
613f7450 docs(13): research, pattern map and validation strategy
8b36d8fa docs(12): create phase plan — four plans in four waves, field run last
b10ecbd3 docs(12): research, pattern map and validation strategy
b9164480 docs(15): capture phase context (assumptions mode)
ef06e20a docs(14): capture phase context (assumptions mode)
5a43d4bf docs(13): capture phase context (assumptions mode)
6f66d6d5 docs(12): capture phase context (assumptions mode)
13fd5344 docs: add phases 12-15 for the remaining documented routines
```
`git diff --stat master...branch -- .planning/`: 26 files, +4406/-5.
ROADMAP.md +151 (four new phase lines, a new "Remaining documented procedures
(Phases 12-15)" section, and detail sections). REQUIREMENTS.md +30
(ROUT-21..26). STATE.md +2: only "Roadmap Evolution" notes; its frontmatter was
never updated, so it still says 11.1 and `total_phases: 4`. musk/ledger.jsonl
+3.

| Phase | Dir | Artifacts (committed) | Implied stage |
|---|---|---|---|
| 12 Intake Throttle Output Test | `12-intake-throttle-output-test` | CONTEXT, DISCUSSION-LOG, RESEARCH, PATTERNS, VALIDATION, 12-01..04-PLAN | planned 0/4 |
| 13 Adaptation Resets and Calibrations That Write | `13-adaptation-resets-and-calibrations-that-write` | CONTEXT, DISCUSSION-LOG, RESEARCH, PATTERNS, VALIDATION | researched |
| 14 Stationary Desoot Forced Regen | `14-stationary-desoot-forced-regen` | CONTEXT, DISCUSSION-LOG | context |
| 15 Capture-Gated Ram Catalog Procedures | `15-capture-gated-ram-catalog-procedures` | CONTEXT, DISCUSSION-LOG | context |

REQUIREMENTS.md on the branch adds the section "Remaining Documented Procedures
(added 2026-10-01, Phases 12-15)" with ROUT-21..26, all `[ ]` Pending. None of
these appear on master. Traceability mapping: 12→ROUT-21; 13→ROUT-22, -23, -24;
14→ROUT-25; 15→ROUT-26. The coverage line goes from master's `22 total … Mapped:
19` to the branch's `28 total … Mapped: 25`. Since the fork, master edited only
the ROUT-F4 block (absorbed by 11.1), so REQUIREMENTS should merge cleanly.
ROADMAP's progress table and STATE's Current Position will likely conflict.
Overlay could also surface per-phase requirement IDs from the worktree's
traceability table.

The branch's ROADMAP progress table for these phases: `12 | 0/4 | Planned`,
`13 | 0/? | Not started`, `14/15 | 0/? | Not started — capture-gated`. Note
that the worktree's row for 11.1 (5/12) is **stale relative to main**
(12/12). Main must stay authoritative for every phase it already knows; only
phases absent from main should come from the worktree.

The worktree's STATE "Roadmap Evolution" lines (useful as the source of a
"why/what" tooltip):
```
- Phases 12-15 added 2026-10-01 (owner: "plan the remaining documented routines"; planning only, on a side branch): ... All wait on 11.1 and the 11-03 field run. ROUT-21..26.
- Phases 12-15 context gathered 2026-10-01 (assumptions mode, unattended, side branch): 12, 13, 14, 15 done; next is plan-phase 12.
```

## Solution

TBD. Starting hints from mapping the current code:

- **Data:** add a field next to `ProjectAgents.unmerged`. It would be filled in
  `scan_project_with_probe` beside `unmerged::detect` (`agents/mod.rs:496`), for
  example `worktree_phases: Vec<WorktreePhase { id, name, stage, plans_done,
  plans_total, worktree, branch, short_ref, ahead, behind, dirty }>`. It covers
  phases present in a worktree's ROADMAP.md and/or `phases/` but absent from main
  (including `milestones/*-phases/`). Derive the stage from artifacts
  (CONTEXT, RESEARCH, PLAN, SUMMARY), the same ladder `disk_status.rs` uses.
  Reuse `DiskInference` if it can run on a worktree dir.
- **Render:** in `roadmap_model_for` (`detail.rs:~2772`), add *ghost nodes* for
  worktree-only phases instead of only re-statusing existing ones. Give them a
  distinct `PhaseStatus` (or `Unmerged` plus a stage) and attribute them to the
  worktree. They must not satisfy dependents (I-11; dependents are computed
  before the post-pass). The dashboard could add a count or glyph for
  "N phases in worktree".
- **Contract changes to decide explicitly:**
  - `unmerged.rs` is currently name-only (I-1, I-3). Parsing a worktree's
    ROADMAP.md is a new file-content read. Bound its size, keep branch names and
    ROADMAP text `Untrusted`, and stay read-only (no git writes, no fetch).
  - Scan cadence: the agents scan runs once per poll tick with the
    `agents_scan_in_flight` guard, so the overlay lags one poll behind. The
    watcher only watches `<root>/.planning` (`watcher.rs:74`), and
    `extract_project_root` would resolve a worktree path to the worktree, not
    the alias. Either accept poll latency or map worktree paths to their owner
    alias.
  - Phase-id normalisation must match `PhaseNum` and `phase_disk_statuses`
    keys (`mod.rs:268-275`). Inserted ids like `11.1` must work.
  - Collisions: if main later adds a phase with the same number but a different
    name than the worktree's (both sides keep diverging), show the conflict
    instead of silently merging. The I-6 known limit (renumber colliding with an
    archived phase) already applies to the SUMMARY overlay.
  - Which worktrees count: agent worktrees (`is_agent_worktree`, under
    `.claude/worktrees/` or on agent branches) for sure. Decide whether to include
    human-made linked worktrees; x0v refuses to register them as projects but
    they could still be overlay sources. Ignore branches with no worktree.
  - Should a worktree's *divergent* STATE frontmatter (stale 11.1) ever surface?
    Probably not; main wins for known phases.
- The roadmap-detail "Worktrees" block deferred in
  `260929-szq-PLAN.md:125-127` is a natural home for a per-worktree breakdown
  (branch, ahead/behind, lock PID liveness, phases carried).
- Fixture: rebuild the sentriq shape synthetically in a test repo (main at
  phase 11.1, linked worktree under `.claude/worktrees/agent-x` adding phases
  12-15 at mixed stages, diverged both ways, plus stale agent branches without
  worktrees).
