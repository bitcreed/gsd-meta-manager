# Phase 35: Decisions, code-needed and gates - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-14
**Phase:** 35-decisions-code-needed-and-gates
**Areas discussed:** signal model and kinds, debug lanes, open-question filter, handoff and
checkpoints, counts and `closes_after`, VERIFICATION display names (added scope), conformance

Mode: `--auto` (unattended). Every choice below is the recommended option, decided by the
orchestrator from the report, REQUIREMENTS.md, ROADMAP.md, Phase 34 artifacts and a code scout.

---

## Debug sessions

| Option | Description | Selected |
|--------|-------------|----------|
| Port `scanDebugSessions`, normalize in Layer B | Oracle-comparable open set; `awaiting*` → Decide, rest → Fix | ✓ |
| Native heuristic reader | Simpler, no oracle parity | |

## Open questions (D11)

| Option | Description | Selected |
|--------|-------------|----------|
| Port `deriveOpenQuestions`, filter RESOLVED/ANSWERED/CLOSED in Layer B | Oracle parity on raw set, owner rule on top | ✓ |
| Filter inside Layer A | Breaks oracle comparison | |

## Decide count unit

| Option | Description | Selected |
|--------|-------------|----------|
| Items (session, handoff, checkpoint, question) | Matches `test` counting checks | ✓ |
| Rows/groups | Coarser; one CONTEXT file = 1 | |

## Debug investigations count

| Option | Description | Selected |
|--------|-------------|----------|
| New additive `fix_debug` | `fix_phases` keeps its meaning | ✓ |
| Fold into `fix_phases` | Changes the meaning of a published key | |

## `closes_after`

| Option | Description | Selected |
|--------|-------------|----------|
| Item totals of phase-scoped groups, `phases_open` + `not_built` | Root-scoped signals excluded | ✓ |
| Include root-scoped debug/handoff in totals | Not attributable to a phase | |

## VERIFICATION display names (added scope)

| Option | Description | Selected |
|--------|-------------|----------|
| Layer A keeps oracle string; Layer B uses structured `test`/`expected`/`why_human` | Oracle parity kept, owner sees clean text | ✓ |
| Change Layer A name | Breaks oracle comparison | |
| Post-hoc string splitting of the flattened text | Fragile on commas and colons in prose | |

**Notes:** observed live: `test: Weekly summary live send (PRICE-14) …, expected: …, why_human: …`.
Regression test required (D-16).

## Conformance

| Option | Description | Selected |
|--------|-------------|----------|
| Extend oracle harness with `audit-open` for debug + CONTEXT | Same opt-out env, source scan extended | ✓ |
| Fixture tests only | No upstream parity for the new Layer A | |

## Todos

`todo.match-phase` returned 17 keyword-only matches, all outside the owner reader; none folded.

## Claude's Discretion

Module split, structured-field type layout, cache reuse, checklist wording, workstream
`closes_after` exposure, plan grouping.

## Deferred Ideas

Report §6 deferred list; upstream issues §8 items 8-9; Phase 34 info items IN-02, IN-04 and the
`App::update(FileChanged)` eviction test.
