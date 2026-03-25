# GSD Manager

## What This Is

A TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — without opening separate terminals or running `/gsd:progress` in each directory.

## Core Value

See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Unified dashboard showing all registered projects and their current phase/status
- [ ] Workflow visualization — ASCII rendering of each project's roadmap and progress
- [ ] Enqueue work — queue next phases, tasks, or verification while current work runs
- [ ] Create new project — spin up a new GSD project (directory, git, settings, kick off initialization)
- [ ] Project registration — add/remove existing GSD projects to track
- [ ] Status detection — read project state from `.planning/` files without running GSD commands

### Out of Scope

- Claude session hooking (screen, ssh, embedded terminal) — v2, research needed on best approach
- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — v1 is local only

## Context

- Built for any GSD user, not just the author
- GSD projects live in separate directories, each with a `.planning/` folder containing STATE.md, ROADMAP.md, config.json, and phase artifacts
- The key technical challenge is reading project state efficiently — either by parsing `.planning/` files directly or by having GSD output state to a centralized location (to be researched)
- Language is open — Python and Rust are candidates, research should inform the choice
- TUI is the primary interface

## Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| V1 scope: dashboard + visualization + enqueue + create | Focus on the core management loop before advanced features | — Pending |
| Claude session hooking deferred to v2 | Needs research, high complexity, not needed for core value | — Pending |
| Language TBD | Let research inform Python vs Rust decision | — Pending |

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
*Last updated: 2026-03-24 after initialization*
