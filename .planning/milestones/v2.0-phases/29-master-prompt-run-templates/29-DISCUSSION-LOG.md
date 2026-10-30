# Phase 29: Master Prompt & Run Templates - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-19
**Phase:** 29-master-prompt-run-templates
**Areas discussed:** Prompt embedding & versioning, driver new & templates, Stop-rule precedence, Reconciliation & control, Router hint, Paid acceptance & carried risk
**Mode:** unattended (operator unavailable). No questions were asked; every choice below was made by the
discuss agent from planning artifacts and is tagged `[inferred — audit]` in CONTEXT.md unless sourced.

---

## Prompt embedding & versioning
| Option | Description | Selected |
|---|---|---|
| `include_str!` asset + Rust const, version in frontmatter and LOG launch line | Single binary, testable drift | ✓ |
| Prompt installed as a file under the data dir | Editable, but version drift and a second artifact | |
**Choice:** embedded asset, `driver-prompt/1.0.0`, `--prompt-file` kept hidden and recorded as `custom:<sha8>`.

## `driver new` & templates
| Option | Description | Selected |
|---|---|---|
| Refuse on run-id collision | Never touch an existing run | ✓ |
| Auto-suffix `-2` | Convenient, but hides a likely mistake | |
**Notes:** UTC date; frontmatter from the Phase 27 emitter (`starting`); DRIVE-03 met by a preflight lint plus a read-only `driver check`.

## Stop-rule precedence
**Choice:** control > launcher-cap > run-budget > divergence > gap-rounds > item budget > no-progress > needs-human; `answer:` lifts only needs-human parks (OQ-R1 default). Divergence-over-gap-rounds is spike evidence; the rest of the order is inferred.

## Reconciliation & control
| Option | Description | Selected |
|---|---|---|
| Model-side only (prompt) | Spike behaviour, unenforced | |
| Model-side + store rejects dispatch until each running item has exactly one `recovered` | Mechanical "exactly one" | ✓ |
**Notes:** launcher gate also denies dispatch on `control: pause|stop`. SC4 "after the current item" read as "after the current dispatch" (PRMT-04) — flagged for audit.

## Router hint
**Choice:** top-level `next <project|path> [--target-phase N] --json`, read-only, `"advisory": true`, cited once in the prompt.

## Paid acceptance & carried risk
| Option | Description | Selected |
|---|---|---|
| Re-run both halves (rules on + rules off) on Opus | ~$20+, re-buys d33 | |
| Reuse 28-09 d33 for the backstop half; one Opus run with the embedded prompt, rules on, cap L=2 | Covers what 28 could not | ✓ |
**Notes:** Haiku cases for rules-stripped integration, OQ-R1, crash/relaunch and pause. Proposed ceiling $25 — needs operator sign-off.

## Claude's Discretion
- Module layout, `next --json` shape beyond the required fields, plan/wave split.

## Deferred Ideas
- l3 harness pacing fix (quick task); DRIVE-07 auto verification; editor/body UI (Phase 30); stall-detection config; Codex runtime.
