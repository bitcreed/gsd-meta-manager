# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-23
**Phase:** 24-roadmap-tab-redesign-and-detail-tab-consolidation
**Mode:** `--auto`, unattended (human unavailable). No AskUserQuestion calls were made.
**Areas discussed:** Roadmap layout, Detail-tab consolidation, PhaseList audit, Parser scope, Todo folding

---

## Roadmap layout (A)

Pre-decided by the user before this session: three mockups (A/B/C, now in `MOCKUPS.md`) were produced and verified, and the master/detail design with the glyph, key, milestone and graph rules in CONTEXT.md §A was approved. Recorded as locked; no alternatives re-opened.

**User's choice:** master/detail list + lane column + detail pane (all of §A).
**Notes:** mouse click-to-select explicitly deferred.

---

## Detail-tab consolidation (B)

Pre-decided by the user: new order `1:Roadmap · 2:Phases · 3:Backlog · 4:Git · 5:Queue · 6:Sess · 7:Cfg · 8:Docs` + Driver; PhaseList removed; Pipeline renamed Phases; Archive folded into Docs as "Milestones". Sequencing: Archive→Docs after the concurrent `/gsd-debug` Archive-Loading fix.

**User's choice:** as above (locked).

---

## Agent-resolved details

| Question | Selected | Why |
|---|---|---|
| Default tab after PhaseList removal | Roadmap | It is tab 1 and supersedes PhaseList |
| Unused digit keys 9/0 | no-op | Stale jumps would land on the wrong tab |
| Persisted view-state migration | none needed | `detail_sub_view_per_project` is in-memory, enum-keyed |
| PhaseList unique content | keep Paused line, change banner, disk `[stage]` badge, empty states; drop Backlog/Queued summaries | Audit of `render_phase_list` |
| `#### Build phase N` location | ttbook is at `/home/blk/projects/python/ttbook`, phases 14-18, with `(Milestone M)` parenthetical | Brief's path/range did not match the filesystem |
| Goal syntax | accept `**Goal**:` and `**Goal:**` | Both forms exist in real roadmaps |
| Todo folding | fold only the phase-list marker todo | Matcher over-scored eight unrelated todos |

## Claude's Discretion

Module split, lane-column width budget, Docs sub-tab key, plan/wave split (subject to Archive→Docs last).

## Deferred Ideas

Mouse click-to-select in the Roadmap list.
