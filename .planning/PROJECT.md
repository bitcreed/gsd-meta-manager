# GSD Meta Manager

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

- ✓ State reader accuracy — disk-based phase inference, plan counting fixes — v1.1
- ✓ GSD integration — verified/inferred badges, gsd_integration config toggle — v1.1
- ✓ Claude session management — detect active sessions, dashboard indicator, launch/resume — v1.1
- ✓ Queue management — full CRUD (add, edit, delete, reorder, mark done) in Queue tab — v1.1
- ✓ Git history viewer — scrollable log, planning-only toggle, diff stats — v1.1
- ✓ Backlog browser — list with content preview, queue promotion — v1.1
- ✓ Execution flow pipeline — per-phase D-R-P-E-V visualization with color-coded stages — v1.1

### Active

(Requirements defined below — see REQUIREMENTS.md)

### Out of Scope

- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — local first, SSH research in backlog

## Current Milestone: v1.2 Housekeeping & Archive Browser

**Goal:** Clean up tech debt, add paused-project detection, research queue execution integration with GSD, and add milestone archive browsing to the detail view.

**Target features:**
- Detect paused projects (HANDOFF.md) and show pause badge on dashboard
- Tech debt cleanup: fix stale integration test, resolve compiler warnings, address deferred visual UAT
- Queue execution research: document GSD hooks, autonomous mode, session lifecycle, and design for auto-continue from QUEUE.md (research only)
- Milestone Archive Browser tab (from backlog 999.1): browse completed milestones and drill into past phase artifacts from the TUI

## Current State

All v1.2 phases complete (10-13). Tech debt cleaned, paused detection added, archive browser built, queue execution researched. Ready for milestone lifecycle.

## Context

- Shipped v1.1 with 6,319 LOC Rust, 79 commits in v1.1 cycle
- Tech stack: Rust, ratatui 0.30, crossterm 0.29, tokio, notify-debouncer-full
- Screen trait architecture with 8 screen modules and 7-tab detail view
- Disk-based phase inference via /proc-like directory scanning
- Queue is now fully managed (CRUD) but not executable (execution research deferred)
- Claude session detection via pgrep + /proc (Linux-only)
- Tech debt resolved: zero warnings, all tests pass (Phase 10 complete)

## Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| V1 scope: dashboard + visualization + enqueue + create | Focus on the core management loop before advanced features | ✓ Good — shipped all 22 requirements |
| Rust + ratatui over Python + Textual | Single binary, zero-cost abstractions, no runtime dependency | ✓ Good — 3.8K LOC, fast, portable |
| Claude session detection via pgrep + /proc | Reliable on Linux, no Claude API dependency | ✓ Good — works for local sessions, Linux-only |
| Queue as managed CRUD, not executable | Execution needs GSD hooks research, CRUD sufficient for v1.1 | ✓ Good — users can manage queue, execution deferred |
| Screen trait refactor in Phase 05 | 11 InputMode variants would explode with new views | ✓ Good — enabled 7-tab detail view cleanly |
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
*Last updated: 2026-04-01 — v1.2 all phases complete*
