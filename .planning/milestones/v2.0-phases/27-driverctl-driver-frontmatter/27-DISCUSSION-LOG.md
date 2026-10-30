# Phase 27: driverctl & Driver Frontmatter - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-18
**Phase:** 27-driverctl-driver-frontmatter
**Mode:** `--auto` (operator unavailable; recommended option selected, inferred picks tagged `[inferred — audit]`)
**Areas discussed:** Status enum / stalled, Gap-round counting ownership, Liveness probe target, DIRECTION write lock, Race-test gating, Todo folding

---

## Status enum / stalled
| Option | Description | Selected |
|--------|-------------|----------|
| Add `stalled` as a typed terminal variant; port `outstanding_dispatch` into driverctl | Single definition shared by launcher + Stop guard | ✓ |
| Add `stalled` to the enum only; leave classification wholly to Phase 28 | Smaller phase, duplicated event logic later | |
**Choice:** [auto] first option (recommended). Classification call site stays in Phase 28.

## Gap-round counting ownership
| Option | Description | Selected |
|--------|-------------|----------|
| Phase 27 owns the 26.1 coalescing counter (COALESCE_S=45); Phase 28 consumes `per_phase` | Matches DCTL-04 "counters come only from tool-observed state" | ✓ |
| Phase 27 keeps spike-002 raw hash-distinct count; Phase 28 ports coalescing | Literal ROADMAP/MANIFEST wording; reintroduces Phase 26 Run 1 inflation in 27 | |
**Choice:** [auto] first option. `[inferred — audit]` it shifts the rule's home from 28 to 27.

## Liveness probe target
| Option | Description | Selected |
|--------|-------------|----------|
| `probe_held(path)` in driver/lock.rs, path parameterized (CTRL-05 lock by default) | Reuses the repo's only flock idiom; Phase 28 picks final path | ✓ |
| Hard-code a per-run `<run-dir>/run.lock` | Pre-empts Phase 28's LNCH-02 decision | |

## DIRECTION write lock
| Option | Description | Selected |
|--------|-------------|----------|
| Separate `<run-dir>/DIRECTION.lock` + body-only write helper shipped now | Steering never contends with liveness lock | ✓ |
| Reuse the run flock | Conflates liveness with edit serialization | |

## Race-test gating
| Option | Description | Selected |
|--------|-------------|----------|
| Full 15 s run `#[ignore]` (opt-in) + short smoke in default suite | Keeps cargo test fast; full measurement on demand | ✓ |
| Full run in default suite | Adds ~30 s + flakiness to every test run | |

## Todo folding
| Option | Description | Selected |
|--------|-------------|----------|
| Fold none (both matches outside driverctl boundary) | Scope guardrail over the >=0.4 auto-fold rule | ✓ |
| Auto-fold both (score 0.9) | Would pull Phase 30 UI and router policy into 27 | |

## Claude's Discretion
- Module layout, sync vs tokio socket server, plan split / waves.

## Deferred Ideas
- macOS `renamex_np` splice; launcher classify/caps/Stop guard (Phase 28); 26.1 WR-10/WR-11 (spike harness only).
