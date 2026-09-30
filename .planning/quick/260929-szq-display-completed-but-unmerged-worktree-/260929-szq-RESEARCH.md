# 260929-szq detection research (read-only subagent, 2026-09-29)

## Snapshot finding (ttbook)
- 0 of 58 worktrees hold a SUMMARY absent from main; yet 13 worktrees are `ahead>0` by rev-list (work arrived via merge-lane/squash). Reproduced: after squash merge `rev-list master..wt` still reports ahead.
- Naive PLAN path-diff flags all 58 worktrees (main renamed `14-16-PLAN.md` → `999.2-*/14-16-PLAN.parked.md`, deleted 15-10..19 / 19-07..17 after replanning).
- => Commit ancestry is REJECTED as the done-signal. Path-level, keyed by plan/quick id, is chosen.

## Seams (file:line)
- Worktree scan: Tick arm app.rs:1269-1296 every 20 ticks (~5s), `agents_scan_in_flight` guard, spawn_blocking + catch_unwind → `Action::AgentsScanned{per_project}` (action.rs:196) → handler app.rs:1771 replaces `ctx.agent_views` (ui/screens/mod.rs:1504). Empty-skip at app.rs:1786 must let unmerged-only projects through.
- `agents::worktrees::scan_worktrees` (src/agents/worktrees.rs:428) → `CoreScan{main path, base_sha, Vec<CoreWorktree{path, branch: Untrusted, locked, prunable, agent_pattern, agent_id}>}`.
- Git only via state_reader/git_ops.rs: `git_read_raw_within` :247 (--no-optional-locks, GIT_OPTIONAL_LOCKS=0, 10s budget :239), `commits_ahead` :676, `dirty_count` :691, `is_full_hex_sha` :661. tests/spawn_seam_guard.rs:45 forbids spawns under src/agents/ — new git reads must be new git_ops helpers with hex-sha/literal-pathspec argv.
- Existing: `AgentRow.summary_in_worktree` (agents/mod.rs:279, :447 via `worktree_holds_summary` :326) → `Finished` liveness :156 — does NOT check main.
- Plug-in: extend `ProjectAgents` / `scan_project_with_probe` (agents/mod.rs:284, :377) with `unmerged: Vec<UnmergedItem{kind: Phase|Quick, id, worktree_path, branch: Untrusted, state}>`.
- Done model: disk_status.rs:19 `DiskStatus`; `infer_disk_status` :959 (FIX/GAPCLOSURE exclusion :1181-1185); `plan_index` stem→(phase,n); `phase_is_done` state_reader/mod.rs:530; `phase_plan_counts` :566. Quick tasks not modelled in src/ (key = `YYMMDD-xxx` dir prefix).

## Algorithm (per non-prunable worktree)
a. read_dir names only: `.planning/phases/*/` and `.planning/quick/*/` → `*-SUMMARY.md` (same FIX/GAPCLOSURE exclusions) and `*-PLAN.md`. No git; catches uncommitted files.
b. Main key set once per project from main worktree FS (`phases/*`, `quick/*`, `milestones/*-phases/*`); keys `(phase,n)` via plan_index, quick id.
c. Unmerged-done = worktree SUMMARY key ∉ main set.
d. (optional) PLAN-only keys ∉ main count only if ADDED on branch: new git_ops helper `git --no-optional-locks -C <wt> diff --name-only --diff-filter=A <base_sha>...HEAD -- .planning/phases .planning/quick`.
- [inferred] "Main" = branch checked out in first porcelain block (the registered path), not origin/HEAD.
- [inferred] InProgress = locked || dirty>0 || agent Live/Idle; else unmerged-done.
- Cost: filenames only; cache per worktree by (HEAD sha, dir mtimes); ride existing 5s scan, no new watcher (D-B04); skip prunable.

## Fixtures
- Reuse tests/agents_scan.rs:43-100 (`plain_repo()`, `add_worktree`, `commit_file`; returns None if sandboxed). Integration tests in tests/, not in src/agents/.
- Cases: worktree SUMMARY committed → flagged; squash-merged to main → cleared; PLAN deleted on main → not flagged in stale worktree; untracked SUMMARY → flagged.
