# GSD Manager

## What This Is

A Rust TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — with live filesystem watching, ASCII roadmap visualization, project creation, and work enqueueing.

## Core Value

See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## Requirements

### Validated

- ✓ Project registration — add/remove existing GSD projects to track — v1.0
- ✓ Status detection — read project state from `.planning/` files without running GSD commands — v1.0
- ✓ Unified dashboard — color-coded project list with rich columns, aggregate status bar, vim navigation, search/filter, help overlay — v1.0
- ✓ Live state and detail view — file watcher auto-refresh, drill-down detail panel with phase breakdown and change summary — v1.0
- ✓ Workflow visualization — ASCII roadmap with phase boxes, status icons, progress markers — v1.0
- ✓ Enqueue work — free-form command queue with suggestions, stored in .planning/QUEUE.md — v1.0
- ✓ Create new project — TUI modal for name/path, git init, hook support, auto-register — v1.0

### Active

- [ ] Fix state reader accuracy — plan counting, phase completion inference from disk
- [ ] GSD integration — hooks, cached status, fact vs assumption distinction
- [ ] Claude session management — detect, attach, launch sessions from TUI
- [ ] Queue execution — make QUEUE.md actionable, not just a reminder list
- [ ] Git history viewer — scrollable git log with repo and .planning/ scopes
- [ ] Backlog browser — view/edit/promote backlog items from TUI
- [ ] Execution flow graph — per-phase discuss/plan/execute/verify pipeline view

### Out of Scope

- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — local first, SSH research in backlog

## Current Milestone: v1.1 Polish & Power Features

**Goal:** Fix state reader accuracy, add GSD integration hooks, Claude session management, queue execution, git history, backlog browser, and execution flow graph.

**Target features:**
- Fix state reader accuracy — plan counting, phase completion inference from disk
- GSD integration — hooks, cached status, fact vs assumption distinction
- Claude session management — detect, attach, launch sessions from TUI
- Queue execution — make QUEUE.md actionable, not just a reminder list
- Git history viewer — scrollable git log with repo and .planning/ scopes
- Backlog browser — view/edit/promote backlog items from TUI
- Execution flow graph — per-phase discuss/plan/execute/verify pipeline view

## Context

- Shipped v1.0 with 3,887 LOC Rust, 83 commits over 2 days
- Tech stack: Rust, ratatui 0.30, crossterm, tokio, notify-debouncer-full
- State reading works by parsing `.planning/` files directly (STATE.md, ROADMAP.md, config.json)
- Known issue: ROADMAP.md parser has plan-counting bugs; disk-based inference planned for v1.1
- Known issue: dashboard shows "P5: Unknown" when milestone is fully complete
- Queue is append-only and passive (GSD doesn't consume QUEUE.md) — execution research planned

## Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| V1 scope: dashboard + visualization + enqueue + create | Focus on the core management loop before advanced features | ✓ Good — shipped all 22 requirements |
| Rust + ratatui over Python + Textual | Single binary, zero-cost abstractions, no runtime dependency | ✓ Good — 3.8K LOC, fast, portable |
| Claude session hooking deferred to v1.1 | Needs research, high complexity, not needed for core value | — Pending, researching in v1.1 |
| QUEUE.md as passive reminder (not GSD-consumed) | Simplest viable enqueue; execution research needed | ⚠️ Revisit — users expect it to be actionable |
| File-watching over polling for live updates | notify-debouncer-full with 200ms debounce | ✓ Good — responsive without CPU overhead |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-26 after v1.1 milestone start*
