# 260929-szq UX spec — unmerged worktree work

Source: senior TUI/UX designer subagent (read-only review), 2026-09-29. [inferred] = decided without the human; audit later.

## Current state (anchors)
- Phase "done" is decided ONLY from main `.planning/` (`phase_is_done` src/state_reader/mod.rs:530; `PhaseMarker::decide` :501). No worktree awareness in disk_status.rs / mod.rs / roadmap_md.rs.
- Conflation sites: src/agents/waves.rs:565 `let d = self.done + self.finished;` (dashboard "4/9 done" counts worktree-only work); src/ui/screens/detail.rs:9609 `AgentLiveness::Finished => "done"`.
- Existing pending-merge precursor: `PaneState::Leftover` `◐` cyan "leftover" (detail.rs:8556-8617, from `PlanState::Finished` waves.rs:103). Extend it, don't invent.
- Palette: roadmap_graph.rs:216-222 `● ◉ ○ ◌`; roadmap_widget ASCII `+ * o`; waves pane `✓ ▶ ◐ ! · ○`; agent words live/idle/finished/stall; dashboard badges normal.rs:107-124; agent summary normal.rs:437 cyan/yellow. No NO_COLOR handling → glyph+word must carry meaning.

## (a) States (precedence)
1. merged-done: SUMMARY exists in main tree (main always wins; covers squash/rebase merges).
2. running on worktree: agent Live/Idle or LOCKED worktree with recent write → `▶` yellow "running"; locked/dirty but inactive > IDLE_SECS → `!` red "stalled".
3. unmerged: not in main, worktree holds SUMMARY (or commits ahead), no live agent → `◐` cyan "unmerged" (rename Waves "leftover").
- 0-ahead dirty unlocked ended worktrees = stray; not counted; dim in Agents sub-view `~N stray` [inferred].

## (b) Which worktree/branch
- `short_ref()`: strip `refs/heads/`, `worktree-`, `gsd/`; `agent-<hex>` → `@` + 7 chars of id (extend until unique); other names truncated to 14 cells with `…`; always via `shown()`.
- Inline only at plan/task granularity: Waves rows dim tail `@a0973c1 +33` / `~3`.
- Roadmap phase rows: counts only. Roadmap detail pane: "Worktrees" block `◐ 15-05 @a0973c1 +33`.
- Full branch + path only for the selected row (status line / Agents sub-view). Help explains `@ +N ~N`.

## (c) Glyphs / colors
- `●`/`✓` merged; `◐` cyan unmerged; `▶` yellow running; `!` red stalled — distinct single-cell shapes, colorblind + monochrome safe; every glyph paired with a word (help.rs:38 rule).
- ASCII widget: `PhaseMarker::Unmerged` glyph `^` [inferred]; legend `+ done  ^ unmerged  * current  o future`.
- `PhaseStatus::Unmerged` (roadmap_graph.rs:332) = not done in main but all plans done-or-unmerged; cyan, not dim. Partially-unmerged phase keeps its glyph + `◐k` tail: `◉ active · 2/5 plans ◐3`.
- Unmerged does NOT satisfy DAG dependents [inferred].

## (d) Project aggregate
- Dashboard Status column agent-summary ladder (waves.rs:518 `summary_forms`, normal.rs:437): `d` = main only; append `◐N unmerged` → `◐N`. Also emitted when no agent active. Counts plans + quick tasks. Cyan; `stalled` yellow takes precedence.
- Filter `//u` = projects with pending merges [inferred, optional].

## Help legend ("Work State", built from glyph constants, rule T-18-63)
```
  ✓ ● done      merged into the main branch
  ◐ unmerged    finished on a worktree branch, not yet in main
  ▶ running     a live agent is working on a worktree
  ! stalled     locked/dirty worktree, no recent activity
  @a0973c1      worktree branch (agent id, 7 chars); +N commits ahead, ~N dirty paths
  Dashboard ◐N  N plans or quick tasks waiting to be merged
```

## Caveats
- commits-ahead vs main HEAD misreports rebase-merged branches → main-SUMMARY-wins rule; quick tasks check main `quick/`.
- Quick tasks aren't rendered in the UI today; only the dashboard count includes them.
