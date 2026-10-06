---
status: complete
---
# 261006-wtp: Show worktree-only roadmap phases

Names-only detector `src/agents/worktree_phases.rs` (phase dirs a linked worktree holds and main lacks, stage ladder CONTEXT/RESEARCH/PLAN/SUMMARY), carried via `ProjectAgents`/`AgentView.worktree_phases`. `roadmap_model_for` adds ghost nodes (status Unmerged, badge `[stage @ref]`, never start_now, no dependents); main wins any shared key. Dashboard summary gains `N wt phases`.

Inferred decisions [audit]: dirs only (no ROADMAP.md content read, keeps I-1 names-only); ghosts skipped when main's ROADMAP/shipped lists the key (no conflict display); poll latency accepted (no watcher change); all non-main non-prunable linked worktrees count, agent or not.
Open: ROADMAP-only phases with no dir; ahead/behind/dirty and lock liveness per ghost; "Worktrees" detail block; requirement IDs; collision display.
