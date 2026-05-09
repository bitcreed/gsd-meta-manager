# Roadmap: GSD Meta Manager

## Milestones

- ✅ **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- ✅ **v1.1 Polish & Power Features** - Phases 05-09 (shipped 2026-03-27)
- ✅ **v1.2 Housekeeping & Archive Browser** - Phases 10-13 (shipped 2026-04-01)
- ✅ **v1.3 Configuration & Pipeline Visibility** - 10 quick tasks (shipped 2026-05-09)

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>v1.0 MVP (Phases 01-04) - SHIPPED 2026-03-26</summary>

See `.planning/milestones/v1.0-phases/` for archived phase artifacts.

Phase 01: Project Foundation (3 plans, complete)
Phase 02: Dashboard & Navigation (3 plans, complete)
Phase 03: Live State & Visualization (2 plans, complete)
Phase 04: Project Creation & Queue (2 plans, complete)

</details>

<details>
<summary>v1.1 Polish & Power Features (Phases 05-09) - SHIPPED 2026-03-27</summary>

See `.planning/milestones/v1.1-phases/` for archived phase artifacts.

Phase 05: State Reader Accuracy (5 plans, complete)
Phase 06: Read-Only Views (4 plans, complete)
Phase 07: Execution Flow & GSD Integration (3 plans, complete)
Phase 08: Queue Execution (2 plans, complete)
Phase 09: Claude Session Management (2 plans, complete)

</details>

<details>
<summary>v1.2 Housekeeping & Archive Browser (Phases 10-13) - SHIPPED 2026-04-01</summary>

See `.planning/milestones/v1.2-phases/` for archived phase artifacts.

Phase 10: Tech Debt Cleanup (1 plan, complete)
Phase 11: Paused Project Detection (1 plan, complete)
Phase 12: Milestone Archive Browser (3 plans, complete)
Phase 13: Queue Execution Research (1 plan, complete)

</details>

<details>
<summary>v1.3 Configuration & Pipeline Visibility (10 quick tasks) - SHIPPED 2026-05-09</summary>

No formal phases — the milestone shipped entirely via `/gsd-quick` tasks
listed in `STATE.md` "Quick Tasks Completed":

- 260401-t7y: tui-textarea + $EDITOR shell-out for archive/backlog markdown
- 260403-p84: PageUp/PageDown scrolling on the detail screen
- 260405-27p: fix folder appears empty after returning from markdown view
- 260405-oum: initial Defaults tab — display + edit .planning/config.json
- 260405-urb: GitHub-ready README.md
- 260509 (defaults dropdown): replace toggle-on-Enter with dropdown picker; include `adaptive` profile
- 260509-k9m: surface intel/graphify keys; text-input for String rows; `x`-to-clear shortcut
- 260509-zh2: layer ~/.gsd/defaults.json under project config; six-section layout matching `/gsd-settings`; `[d]` toggle to edit defaults; pipeline sub-stage drill-down
- 260509-t8m: Tab-to-switch into a tmux Claude session

</details>

## Backlog

### Phase 999.2: Container Support with Claude Command Injection (BACKLOG)

**Goal:** Start, stop, and resume Claude sessions inside containers mapped to project directories. Monitor container output and inject commands directly into running Claude instances from the TUI -- enabling remote/isolated execution without terminal switching.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

Note: Backlog 999.1 (Milestone Archive Browser) promoted to Phase 12 in v1.2.
