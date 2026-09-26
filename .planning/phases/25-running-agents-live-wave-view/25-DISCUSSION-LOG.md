# Phase 25: Running Agents & Live Wave View - Discussion Log

> **Audit trail only.** Do not use this log as input to planning, research or execution agents.
> Decisions are captured in CONTEXT.md. This log preserves the alternatives that were considered.

**Date:** 2026-09-25
**Phase:** 25-running-agents-live-wave-view
**Mode:** `--auto`, unattended. The human was unavailable, so the agent picked the recommended option in every area.
**Areas discussed:** architecture seam, Codex scope, non-intrusive git, cadence, liveness, worktree-less agents, UI placement, fix-run progress

---

## Architecture seam
| Option | Selected |
|---|---|
| A runtime-agnostic core plus pluggable enrichment adapters | ✓ (user-supplied) |
| Claude-specific reader only | |

## Codex adapter scope
| Option | Selected |
|---|---|
| Deferred, with the design notes recorded and the seam guaranteeing a self-contained add | ✓ (default; a user answer is pending) |
| In scope now | |

## Non-intrusive git
| Option | Selected |
|---|---|
| `--no-optional-locks` plus `GIT_OPTIONAL_LOCKS=0`, with `git status --porcelain` | ✓ (user-supplied) |
| Plain `git status` (takes `index.lock`) | |

## Cadence
| Option | Selected |
|---|---|
| Ride the existing 5s session-poll tick with `spawn_blocking` | ✓ [inferred] |
| `notify` watchers on `~/.claude` and the worktrees | |
| A dedicated timer | (forbidden by `app.rs` comments) |

## Liveness
| Option | Selected |
|---|---|
| Transcript mtime; live ≤120s, idle ≤10min, then stalled | ✓ [inferred] |
| Lock-line pid | (rejected: the pid belongs to the top-level session) |

## Worktree-less subagents
| Option | Selected |
|---|---|
| Show live ones in their own group | ✓ [inferred] |
| Worktree agents only | |

## UI placement
| Option | Selected |
|---|---|
| Summary in the dashboard Status cell, plus a `Sessions \| Agents` sub-view | ✓ [inferred] |
| New dashboard column, plus a ninth detail tab | (breaks the 80-column budget and the Phase 24 consolidation) |

## Fix-run progress
| Option | Selected |
|---|---|
| A `~fixed/total` estimate from REVIEW.md plus the finding ids in commits; lowest priority | ✓ [inferred] |
| Omit | |

## Claude's Discretion
Module layout, trait shape, glyphs and colours, and how the plans split into waves.

## Deferred Ideas
The Codex adapter (with its design notes), actions on agents, `notify`-based watching, and adapters for other runtimes.
