# Phase 05: State Reader Accuracy - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-26
**Phase:** 05-state-reader-accuracy
**Areas discussed:** Disk inference depth, Status display format, CLI name auto-derive, Async migration scope

---

## Disk Inference Depth

### Q1: How should disk-based inference relate to ROADMAP.md parsing?

| Option | Description | Selected |
|--------|-------------|----------|
| Disk is truth, roadmap is fallback | Disk files are ground truth for phase status. ROADMAP.md checkboxes used only when no phase directory exists. | |
| Merge both signals | Combine disk inference + ROADMAP.md into a richer picture. | |
| You decide | Claude picks the best approach based on GSD internals | ✓ |

**User's choice:** You decide
**Notes:** Claude has discretion to pick the best approach

### Q2: How deep should the disk_status inference go?

| Option | Description | Selected |
|--------|-------------|----------|
| GSD's 7 statuses | Match GSD exactly: no_directory, empty, discussed, researched, planned, partial, complete | |
| Extended with counts | GSD's 7 statuses + plan count, summary count, and file timestamps | |
| You decide | Claude picks the right level of detail | ✓ |

**User's choice:** You decide
**Notes:** Claude has discretion on depth

### Q3: Should InputMode refactoring happen in Phase 05?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, prerequisite | Research says InputMode enum will explode past 20 variants — refactor now | ✓ |
| No, defer to Phase 06 | Phase 05 is bugfixes and inference — no new screens | |
| You decide | Claude judges whether it's needed now or can wait | |

**User's choice:** Yes, prerequisite
**Notes:** User agrees with research recommendation to refactor before adding views

---

## Status Display Format

### Q1: How should disk-inferred phase status display in the dashboard project list?

| Option | Description | Selected |
|--------|-------------|----------|
| Compact pipeline | Single-line `D-R-P-E-V` with color per stage | |
| Text label only | Show most advanced stage as text | |
| You decide | Claude picks what fits existing layout | |

**User's choice:** Compact pipeline but when moving onto the line switch to showing the full text label
**Notes:** Hybrid approach — compact in table, full text when row is focused/selected

### Q2: When a milestone is fully complete, what should the dashboard show?

| Option | Description | Selected |
|--------|-------------|----------|
| "Complete" with green | Simple 'Complete' text in green, no phase number | |
| Milestone name | Show 'v1.0 Complete' or the milestone name from STATE.md | |
| You decide | Claude picks based on available space | |

**User's choice:** You decide but show milestone name if not visible elsewhere on same screen
**Notes:** Context-aware display — include milestone version when it wouldn't be redundant

---

## CLI Name Auto-Derive

### Q1: How should the CLI handle add without an alias?

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-derive from folder | Use last path component, no prompt | |
| Make alias optional positional | `add <path> [alias]` — optional, defaults to folder name | ✓ |
| Path-only shorthand | Support `gsd-meta-manager <path>` without subcommand | |

**User's choice:** Make alias optional positional
**Notes:** Change CLI signature to `add <path> [alias]`

---

## Async Migration Scope

### Q1: How much async migration should happen in Phase 05?

| Option | Description | Selected |
|--------|-------------|----------|
| Full migration now | Move all parse_project_state() file reads to spawn_blocking | ✓ |
| Just new code | Only new disk-inference code uses spawn_blocking | |
| You decide | Claude picks based on effort vs payoff | |

**User's choice:** Full migration now
**Notes:** Pay the cost once so Phase 06+ features build on async foundation

---

## Claude's Discretion

- Disk inference depth and merge strategy with ROADMAP.md
- Completed milestone display format (context-aware)

## Deferred Ideas

None — discussion stayed within phase scope
