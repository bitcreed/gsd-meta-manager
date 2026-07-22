---
quick_id: 260722-emn
plan: 1
item: A
subsystem: docs
status: complete
tags: [readme, docs, value-proposition, gsd-1.8.0]
requires: []
provides: [readme-value-proposition, gsd-1.8.0-compat-note]
affects: [README.md]
key-files:
  created: []
  modified: [README.md]
decisions:
  - Framed claude-orchestration as complementary (within-project parallelism) vs gsd-meta-manager (cross-project observation), not competing
  - Grounded all factual claims about claude-orchestration in gsd-core CHANGELOG 1.8.0 + explanation doc (BETA, default-off, claude-only, no new on-disk state)
metrics:
  duration: ~4min
  completed: 2026-07-22
---

# Quick Task 260722-emn Plan 1: README Value Proposition Summary

Clarified README.md's value proposition against GSD 1.8.0's experimental
`claude-orchestration` backend and stated GSD 1.8.0 `.planning/` state-format
compatibility, without inventing any features the binary lacks.

## What Changed

Three edits to `README.md`, all additive prose (33 insertions, 0 deletions):

1. **New "Why GSD Meta Manager?" subsection** (after "What is it?") positioning
   the tool as the cross-project command center, with four real capabilities:
   zero-token on-disk `.planning/` state reading, Claude session
   detection/launch/resume + auto-registration, tmux `Tab`-to-switch focus, and
   milestone archive browsing with inline markdown rendering.
2. **New "Not the same as GSD's claude-orchestration backend" subsection**
   distinguishing the tool from GSD 1.8.0's opt-in execution backend: that backend
   parallelizes plan execution inside a single Claude session on a single project
   via Claude Code's Workflow tool and writes no new on-disk state format, whereas
   the Meta Manager observes and acts across many projects. Framed as complementary.
3. **New "Compatibility" note** under Requirements stating the tool reads GSD 1.8.0
   `.planning/` state formats and is non-intrusive / backend-agnostic.

The `<!-- generated-by: gsd-doc-writer -->` leading comment was preserved and all
existing accurate sections (Features, Installation, Quick start, Usage, How it
works) were left intact. Prose kept to ~80 columns, matching the file.

## Factual Grounding

Claims about `claude-orchestration` were verified against the canonical GSD 1.8.0
reference at `/home/blk/projects/node/gsd-core`:
- `CHANGELOG.md` [1.8.0] entry (#1143 / #2044) — default-off, BETA, claude-only;
  adopts Claude Code's Workflow tool; waves → `parallel()` barriers; produces
  identical commits/artifacts to the inline path.
- `docs/explanation/claude-orchestration-capability.md` — confirms the emitted
  Workflow script composes the same `gsd-executor` agent and produces the same
  `SUMMARY.md` artifacts and commits (no new on-disk state format).

No fabricated features were introduced.

## Verification

- `cargo build` — succeeds (172 crates, docs-only change; sanity gate only).
- `grep -qi "1.8.0" README.md` — PASS
- `grep -qi "orchestration" README.md` — PASS
- `grep -qi "cross-project|across.*projects" README.md` — PASS
- `generated-by` comment preserved (line 1) — PASS
- No file deletions in the commit.

## Deviations from Plan

None - plan executed exactly as written. (Reference doc path in the plan's
`<context>` pointed to a non-existent nested `gsd-core/gsd-core/docs/...` path;
located the equivalent canonical docs under `/home/blk/projects/node/gsd-core`
and verified all claims there. No change to plan intent or scope.)

## Commits

- `8259a32`: docs(260722-emn): clarify value proposition vs GSD 1.8.0 claude-orchestration

## Self-Check: PASSED

- FOUND: README.md (modified)
- FOUND commit: 8259a32
