# Phase 38: Inbox digest and triage hygiene - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-16
**Phase:** 38-inbox-digest-and-triage-hygiene
**Areas discussed:** CLI default and views, multi-project collapse, data cleanups vs heuristics, context tags, view routing, ranking, merge semantics, stale fold, labels, JSON
**Mode:** `--auto` (unattended). The owner decided A-D before the run. The rest was decided by the orchestrator from the planning artifacts and marked [inferred].

---

## A. CLI default output (owner)

| Option | Description | Selected |
|--------|-------------|----------|
| Short digest by default | Per-project lane counts and the top few items; `<alias>`, `--lane`, `--all` for more; `--json` unchanged | ✓ |
| Keep the full checklist as default | Add a `--digest` flag instead | |

**User's choice:** Short digest by default.

## B. Multi-project granularity (owner)

| Option | Description | Selected |
|--------|-------------|----------|
| Collapse to phases | One entry per phase, e.g. "ph 43 · 5 checks" | ✓ |
| Individual checks | One row per check | |

**User's choice:** Collapse to phases.

## C. Context tags desk/phone/truck (owner)

**User's choice:** Deferred, out of scope.

## D. What may change data (owner)

| Option | Description | Selected |
|--------|-------------|----------|
| Only pure-parsing cleanups change data; heuristics are labels | Age fold (~90 d), same-phase file merge and count alignment change what is shown. `decision?` / `looks resolved` only label | ✓ |
| Heuristics may move or hide rows | | |

**User's choice:** As selected. The app never writes GSD files.

## View routing [inferred]

| Option | Description | Selected |
|--------|-------------|----------|
| json > --all > one project > --lane > digest; bare `--include` implies `--all` | Keeps old `--include` invocations printing the full list | ✓ |
| `--include` adds sections to the digest | Breaks existing `--include archived` use | |

## Ranking of next actions [inferred]

| Option | Description | Selected |
|--------|-------------|----------|
| closes-phase, fewest checks, newer first; Test/Decide/Fix only | Follows OWN-24 wording ("older items rank lower") | ✓ |
| Reuse the TUI order (progress, oldest first) | Contradicts OWN-24 | |

## Same-phase merge [inferred]

| Option | Description | Selected |
|--------|-------------|----------|
| Display merge, summed count, no item dedup; JSON keeps per-file groups plus a link | Oracle and key paths untouched; no heuristic | ✓ |
| Dedup near-identical items | Text matching is a heuristic, which decision D excludes | |

## Stale fold [inferred]

| Option | Description | Selected |
|--------|-------------|----------|
| Separate `owner.fold_days` = 90; totals and gates keep counting | The fold is triage-only; the milestone still needs the work | ✓ |
| Reuse `stale_days` (30) | Would fold a third of the fleet | |

## Labels [inferred]

| Option | Description | Selected |
|--------|-------------|----------|
| Word-boundary keyword lists, `OK` case-sensitive, not on Decide-lane items; dim suffix and counts | | ✓ |

## Claude's Discretion

Digest wording and spacing, JSON key names, fold config key name, where the merge lives (Layer B
or a shared view model), `A` on a merged entry, plan split.

## Deferred Ideas

Context tags; moving/hiding rows by heuristic (OWN-F02); item-level dedup; a TUI digest view; 37
UAT item 6 (owner's call); check actions (Phase 39).
