---
quick_id: 260722-emn
plan: 3
item: C
status: complete
subsystem: state_reader
tags: [roadmap-parsing, gsd-1.8.0, progress-table]
provides:
  - roadmap_md::RoadmapProgress
  - roadmap_md::roadmap_progress
affects:
  - src/state_reader/roadmap_md.rs
key-files:
  modified:
    - src/state_reader/roadmap_md.rs
decisions:
  - Broadened phase-ID token to a shared PHASE_ID const covering decimals, project-code/milestone prefixes, and a trailing letter (999.x)
  - Retirement/backlog encoded by FILTERING phases out of the returned Vec (no new RoadmapPhase fields), per plan scope guard
  - Progress-table columns matched by NAME (case-insensitive), not position, so flat 4-column and milestone-grouped 5-column layouts both parse
metrics:
  tasks: 3
  files: 1
  commits: 3
---

# Quick 260722-emn Plan 3: roadmap_md.rs GSD 1.8.0 parsing Summary

Extended `src/state_reader/roadmap_md.rs` to the heading forms and progress
conventions GSD 1.8.0 roadmaps use: `###` markdown headings, parenthetical
cluster tags, project-code/milestone-prefixed IDs, `<details>`-wrapped
checklists, strikethrough retirement, backlog sentinels, and a new
name-addressed `## Progress` table parser (`RoadmapProgress` + `roadmap_progress`).

## Tasks

- **Task 1 — additional phase heading forms:** Introduced a shared `PHASE_ID`
  token `(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?`, added a `###`-heading
  recognizer (no checkbox → `completed: false`, empty description), stripped
  optional parenthetical tags from number/name, and made
  `<details>`/`<summary>`/`</details>` wrappers transparent (they match neither
  the plan nor phase recognizers, so the plan-item scan is unaffected). The
  inner plan-scan stop condition now checks both heading shapes. Commit `1b1e763`.
- **Task 2 — retirement/backlog exclusion:** Added `is_sentinel_phase` (strips
  an alphabetic project-code prefix, then excludes `0`, `999`, `999.x` while
  keeping ordinary decimals like `0.3`) and strikethrough detection (`~~...~~`
  via the optional capture groups plus a line-level `contains("~~")` guard).
  Both filters apply at push time, so retired/backlog phases never enter the
  returned Vec or any count. Commit `0cb6b90`.
- **Task 3 — `## Progress` table parser:** Added `pub struct RoadmapProgress`
  (Debug/Clone/Default/PartialEq) and `pub fn roadmap_progress(&str) ->
  Option<RoadmapProgress>`. It scopes to the `## Progress` section, locates the
  markdown table (header row followed by a delimiter row), builds a
  case-insensitive header-name → index map, and derives phase totals
  (excluding 999.x/0 backlog rows), completed phases (Status = complete/done),
  and plan totals from `Plans Complete` cells (`N/M` → completed `N`, total `M`).
  Returns `None` when the section is absent or the required `Phase` /
  `Plans Complete` columns are missing. Commit `d5c381b`.

## Design Notes

- The `RoadmapPhase` struct field set is unchanged (scope guard honored) —
  `change_tracker.rs` builds it with a full struct literal and is untouched.
- Column matching is by name, proven position-independent by a column-reordered
  test. The 5-column milestone-grouped layout parses via the same code path.
- `RoadmapProgress` / `roadmap_progress` are `pub` in a crate with a `lib.rs`
  library target, so they are reachable API — no dead-code warning even though
  the mod.rs wiring ("prefer Progress table over STATE.md frontmatter") is
  deferred to plan 6.

## Verification

- `cargo build`: succeeds.
- `cargo test`: 140 passed (whole workspace); 22 of those in `roadmap_md`
  (6 pre-existing + 16 new).
- `cargo clippy --lib`: no issues.
- New behaviors covered by inline tests: `###`/`##`/`####` headings, parenthetical
  tags, `M-2`/`AB-29` prefixes, `<details>` nesting, decimals preserved,
  strikethrough exclusion, `Phase 0`/`999`/`999.2` sentinel exclusion, three
  real phases unaffected, flat 4-column table, milestone-grouped 5-column table,
  column-reordered table, backlog-row exclusion, absent-section `None`,
  uninterpretable-header `None`.

## Deviations from Plan

None — plan executed exactly as written. Commits used the plan's suggested
conventional messages (feat/fix/feat), one atomic commit per task.

## Known Stubs

None. `roadmap_progress` is fully implemented; the mod.rs consumer wiring is
explicitly out of scope for this plan (assigned to plan 6) and is not a stub.

## Self-Check: PASSED

- `src/state_reader/roadmap_md.rs` present and modified across all three commits.
- Commits `1b1e763`, `0cb6b90`, `d5c381b` exist on `worktree-agent-a70b5d796d81b4533`.
