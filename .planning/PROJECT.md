# GSD Meta Manager

## What This Is

A Rust TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — with live filesystem watching, ASCII roadmap visualization, project creation, work enqueueing, paused project detection, and milestone archive browsing.

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

- ✓ Tech debt cleanup — zero warnings, zero clippy lints, clean formatting — v1.2
- ✓ Paused project detection — HANDOFF file detection with cyan pause badge on dashboard — v1.2
- ✓ Milestone archive browser — 8-tab detail view with drill-down, styled markdown, async loading — v1.2

- ✓ Defaults tab — 9-tab detail view exposing `.planning/config.json` with categorized rendering, dropdown picker for booleans/enums, inline text-input for string rows (`base_branch`, etc.), `x`-to-clear shortcut, `r`-to-reload, six sections mirroring `/gsd-settings` — v1.3
- ✓ Defaults layering — `~/.gsd/defaults.json` parsed and surfaced as fallback for unset rows with `*` marker; `[d]` toggle to edit the global defaults file directly — v1.3
- ✓ Pipeline sub-stage drill-down — Plan and Execute parents expand into per-toggle artifact rows (PATTERNS, PLAN-CHECK, VALIDATION, UI-SPEC, UI-CHECK, AI-SPEC, REVIEW, UI-REVIEW) with ✓/○ markers — v1.3
- ✓ Tmux session switching — Tab on the project list or Sessions tab jumps focus to the matching tmux pane (mirrors claudectl) — v1.3
- ✓ Markdown editing polish — `tui-textarea` + `$EDITOR` shell-out for archive/backlog markdown, PageUp/PageDown scrolling on detail screen — v1.3
- ✓ GitHub-ready README — landing-page README with quickstart, screenshots, and kid-friendly GSD explainer — v1.3
- ✓ Queue execution research — design document with 2 strategies, safety requirements, LLM-agnostic — v1.2

### Active

(No active requirements — start next milestone with `/gsd:new-milestone`)

### Out of Scope

- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — local first, SSH research in backlog
- Offline mode — real-time file watching is core value

## Current State

Shipped v1.3. 13 phases + 10 quick tasks across 4 milestones. v1.3 (Configuration & Pipeline Visibility) shipped entirely via `/gsd-quick` — no formal phases — and centred on the Defaults tab (six-section layout mirroring `/gsd-settings`, `~/.gsd/defaults.json` layering with `*` marker for inherited values, `[d]` toggle, dropdown/text-input editors, `x`-to-clear), pipeline sub-stage drill-down, and tmux Tab-to-switch.

## Context

- Shipped v1.2 with 7,630 LOC Rust, 120+ commits across 3 milestones
- Tech stack: Rust, ratatui 0.30, crossterm 0.29, tokio, notify-debouncer-full
- Screen trait architecture with 8 screen modules and 8-tab detail view (Archive added in v1.2)
- Disk-based phase inference via /proc-like directory scanning
- Queue is fully managed (CRUD) with execution design ready for v1.3
- Claude session detection via pgrep + /proc (Linux-only)
- Paused project detection via HANDOFF.md/HANDOFF.json with content validation
- Milestone archive browser with 4-level drill-down and styled markdown rendering
- Zero build warnings, zero clippy warnings, 25 integration tests pass

## Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| V1 scope: dashboard + visualization + enqueue + create | Focus on the core management loop before advanced features | ✓ Good — shipped all 22 requirements |
| Rust + ratatui over Python + Textual | Single binary, zero-cost abstractions, no runtime dependency | ✓ Good — 7.6K LOC, fast, portable |
| Claude session detection via pgrep + /proc | Reliable on Linux, no Claude API dependency | ✓ Good — works for local sessions, Linux-only |
| Queue as managed CRUD, not executable | Execution needs GSD hooks research, CRUD sufficient for v1.1 | ✓ Good — execution design ready for v1.3 |
| Screen trait refactor in Phase 05 | 11 InputMode variants would explode with new views | ✓ Good — enabled 8-tab detail view cleanly |
| File-watching over polling for live updates | notify-debouncer-full with 200ms debounce | ✓ Good — responsive without CPU overhead |
| Custom markdown renderer over pulldown-cmark | No new dependency, line-by-line regex sufficient for archive display | ✓ Good — tiered header styling, code blocks, bold |
| Per-item isolation for queue execution (v1.3) | Simpler than session chaining, each item runs independently from disk state | — Pending v1.3 implementation |
| LLM-agnostic queue execution design | GSD could use any LLM backend, not just Claude | ✓ Good — Executor trait interface designed |

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
*Last updated: 2026-04-01 — v1.2 milestone complete*
