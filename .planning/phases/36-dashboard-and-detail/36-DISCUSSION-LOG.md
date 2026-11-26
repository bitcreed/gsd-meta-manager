# Phase 36: Dashboard and detail - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-14
**Phase:** 36-dashboard-and-detail
**Areas discussed:** digest loading and cost, cell wording and lane cap, summary-line invariant,
sort rank and `/y` grammar, Owner block placement and focus, folds

Mode: `--auto` (unattended). Every choice below is the recommended option, decided by the
orchestrator from the report, REQUIREMENTS.md, ROADMAP.md, Phase 34/35 artifacts and a code scout.

---

## Where the digest is computed and stored

| Option | Description | Selected |
|--------|-------------|----------|
| In the existing reparse task, stored in an `AppContext` map | One dispatch site, `ProjectState` untouched for driver/router callers | ✓ |
| `ProjectState.owner` field (34 D-15 wording) | Every `parse_project_state` caller carries and compares it | |
| Separate background task | Second dispatch site, breaks the OBS-06 counter claim | |

## Lane cap with three lanes

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed order decide, test, fix; drop fix | OWN-16 wording | ✓ |
| Mockup order (`fix 4 ph · test 6`) | Contradicts OWN-16 | |

## Summary "equals visible cells"

| Option | Description | Selected |
|--------|-------------|----------|
| Σ of per-project numbers over visible (filtered) rows, one shared function | Holds under the two-lane cap and filters | ✓ |
| Fleet-wide totals | Breaks the invariant under a filter | |

## Summary line placement

| Option | Description | Selected |
|--------|-------------|----------|
| Own row, `>= 100` columns only | No change below 100 columns | ✓ |
| Inside the existing footer | Too tight next to the key hints | |

## `/y` grammar

| Option | Description | Selected |
|--------|-------------|----------|
| Extend the `term/x` suffix grammar: `/y`, `/y test|decide|fix` | Consistent with `/h` | ✓ |
| Separate prefix grammar | Second grammar | |

## Owner block placement

| Option | Description | Selected |
|--------|-------------|----------|
| Full-width top of the Phases tab, zero rows when empty | Project-wide header; existing snapshots unchanged | ✓ |
| Inside the right pane per selected phase | Loses the project-wide view | |

## Folds

| Option | Description | Selected |
|--------|-------------|----------|
| Old debt / Acknowledged expandable, Todos count only | Todo triage is deferred | ✓ |
| Todos expandable with titles | Needs a todo read on load; triage deferred | |

## Claude's Discretion

Column constraint, row breakpoints, `/y` short forms, module split, plan grouping.

## Deferred Ideas

Phase 37 actions (`i` screen, `v`, `A`, `y`, `o`); report §6 deferred list; 34 IN-02, IN-04.
Reviewed todos: 17 keyword-only matches, none folded.
