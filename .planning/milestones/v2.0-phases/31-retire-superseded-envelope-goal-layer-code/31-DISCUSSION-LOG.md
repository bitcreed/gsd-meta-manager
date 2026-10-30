# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-24
**Phase:** 31-retire-superseded-envelope-goal-layer-code
**Mode:** `--auto` (human unavailable; every selection is Claude's, marked `[inferred — audit]`)
**Areas discussed:** Legacy `drive` scope, Codex drive, Envelope module layout, Threat-closure set, 28.1 dependency, Experimental gate, Phase 22 archive, Git-version witness

---

## Legacy `drive` path scope

| Option | Description | Selected |
|--------|-------------|----------|
| Delete `drive` + its exclusive code | Matches 28 D-01 / 30 D-01,D-02 hand-overs; removes main consumer of guard, goal layer, inbox | ✓ |
| Keep `drive`, strip only safety seams | Rewires a dead path just to delete from it | |

**Choice:** [auto] delete. **Notes:** costly-reversible; journal run dirs on disk untouched.

## Codex drive (quick 260922-hdj)

| Option | Description | Selected |
|--------|-------------|----------|
| Remove with `drive`, keep `executor::runtime` + live stream_json parts | Only `drive` ever ran CodexExecutor | ✓ |
| Escalate to user | Removes a shipped experimental capability | |

**Choice:** [auto] remove; flagged as the top audit item rather than escalated (prior phases already scheduled the legacy driver for deletion).

## Envelope module layout

| Option | Description | Selected |
|--------|-------------|----------|
| Keep `src/envelope/{mod,scan,ledger,policy(reduced)}` in place | Already imported by `launcher/push.rs`; zero behaviour churn | ✓ |
| Move/rename into `src/launcher/` | Cosmetic; touches every import | |

## Threat-closure set

| Option | Description | Selected |
|--------|-------------|----------|
| All open Phase 19 ids, classified G / N / K | Includes T-19-112 and T-19-121 (blocking, missing from ROADMAP list) | ✓ |
| Only the ROADMAP's four | Leaves blocking ids open on deleted code | |

**Notes:** kept-code threats closed only if the push-step context actually removes the attack; Phase 21 CR-01 (surviving session_detector) not closed as superseded.

## Phase 28.1 dependency

| Option | Description | Selected |
|--------|-------------|----------|
| Treat 28.1 as landed; close N-class ids now with "reopens if D-11 fails" | Per coordinator carry-in | ✓ |
| Wait for D-11 live probe | Blocks phase on a human item | |

## `GSDMM_EXPERIMENTAL_FEATURES`

| Option | Description | Selected |
|--------|-------------|----------|
| Keep flag; drop only deleted-surface gate sites | Un-gating is a product decision | ✓ |
| Remove flag entirely | Would un-gate the Phase 28/30 launcher | |

## Phase 22 archive

| Option | Description | Selected |
|--------|-------------|----------|
| In-place `status: superseded` + `22-SUPERSEDED.md` | Keeps ROADMAP links and gsd-tools lookup | ✓ |
| Move directory to milestone archive | Belongs to complete-milestone | |

## Git-version witness

| Option | Description | Selected |
|--------|-------------|----------|
| Delete with classifier; update pre-tag-check, Dockerfile, CI, CLAUDE.md | Constant has no users outside the classifier | ✓ |
| Keep witness standalone | Witnesses nothing once the classifier is gone | |

## Claude's Discretion

Plan/wave split, `ParkReason` pruning, closure-row wording, optional `bollard` guard test, optional module rename.

## Deferred Ideas

Un-gating experimental; Codex on the new launcher; physical Phase 22 move. Todos (4 scoring ≥ 0.4) reviewed and not folded (scope guardrail overrides auto-fold).
