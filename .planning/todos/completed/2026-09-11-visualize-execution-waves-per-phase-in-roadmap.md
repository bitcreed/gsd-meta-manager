---
created: 2026-09-12T05:13:07.293Z
title: Visualize execution waves per phase in roadmap
area: ui
severity: major
files:
  - src/state_reader/roadmap_md.rs
  - .planning/ROADMAP.md
---

## Problem

The roadmap view (backed by `.planning/ROADMAP.md` and parsed via
`src/state_reader/roadmap_md.rs`) currently shows phases and their plan
counts, but doesn't surface how a phase's plans are grouped into parallel
execution waves. GSD's `/gsd-execute-phase` plans phases with dependency-aware
wave parallelization (waves of plans that ran concurrently vs. sequentially),
and that wave/dependency metadata can live in plan frontmatter or phase
manifests. Today `roadmap_md.rs` has no wave parsing at all — grep confirms no
"wave" handling exists in `src/state_reader/`.

Users managing multiple GSD projects from this TUI have no at-a-glance way to
see which plans in a phase ran together (wave N) vs. which were serialized,
or to see the plan-to-phase grouping visually, without opening the project's
own `.planning/` files in an editor.

## Solution

TBD. Likely needs:
- Determine where wave/dependency metadata is actually recorded across GSD
  versions (phase manifest, PLAN.md frontmatter, or ROADMAP.md phase
  sections) — confirm this is derivable before committing to a rendering
  approach; the todo's request is conditional on "if that information can be
  derived".
- Extend `src/state_reader/roadmap_md.rs` (or a new parser) to extract
  wave/dependency grouping per phase.
- Render waves in the roadmap view — e.g., group plan chips/rows by wave
  number under each phase, or a small parallel-lanes visualization — reusing
  the existing `Canvas`/`Block` rendering patterns already used elsewhere in
  the TUI for phase display (see `src/ui/screens/detail.rs`).
- Gracefully degrade (show flat plan list, no wave grouping) for projects/
  GSD versions where wave metadata isn't present.
