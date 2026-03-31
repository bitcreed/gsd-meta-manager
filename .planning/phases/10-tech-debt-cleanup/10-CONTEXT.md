# Phase 10: Tech Debt Cleanup - Context

**Gathered:** 2026-03-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Resolve all `#[allow(dead_code)]` suppressions and ensure a clean, warning-free, clippy-clean baseline for v1.2 feature work. No new features — cleanup only.

</domain>

<decisions>
## Implementation Decisions

### Dead Code Strategy
- **D-01:** Thorough cleanup — remove genuinely unused code, properly wire up structs that Phase 12 (Archive Browser) will need
- **D-02:** Structs like `StateFrontmatter`, `ProgressInfo`, `RoadmapPhase` may be needed for archive browsing — make them used (expose via pub API or tests) rather than deleting
- **D-03:** `BacklogItem.slug` field — evaluate if used anywhere; remove if genuinely dead
- **D-04:** `ScreenAction` enum and `change_tracker::ProjectSnapshot` — evaluate usage; remove dead variants/structs

### Cleanup Thoroughness
- **D-05:** Run `cargo clippy` and fix all warnings, not just dead_code
- **D-06:** Ensure `cargo fmt --check` passes
- **D-07:** No new test coverage requirements — just ensure existing 20 tests pass

### Claude's Discretion
- How to resolve each dead_code item (remove vs wire up) — use judgment based on Phase 12 needs
- Whether to add `#[cfg(test)]` usage for structs that are only useful in tests
- Clippy lint fixes — apply standard Rust idioms

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone audit (source of tech debt items)
- `.planning/milestones/v1.1-MILESTONE-AUDIT.md` — Lists all tech debt items from v1.1

### Research
- `.planning/research/PITFALLS.md` — Warns about dead_code items needed by archive browser
- `.planning/research/ARCHITECTURE.md` — Phase 12 integration points (which structs will be needed)

</canonical_refs>

<code_context>
## Existing Code Insights

### Dead Code Locations (7 annotations)
- `src/change_tracker.rs:14` — `ProjectSnapshot` struct
- `src/state_reader/state_md.rs:4` — `StateFrontmatter` struct
- `src/state_reader/state_md.rs:25` — `ProgressInfo` struct
- `src/state_reader/roadmap_md.rs:4` — `RoadmapPhase` struct
- `src/state_reader/backlog.rs:7` — `BacklogItem.slug` field
- `src/ui/screens/mod.rs:33` — `ScreenAction` enum
- `src/state_reader/mod.rs:3` — module-level suppression

### Current Test State
- 20 tests pass (unit + integration)
- `end_to_end_add_then_list_via_cli` already uses correct CLI arg order
- No compiler warnings (all suppressed via `#[allow(dead_code)]`)

### Build State
- `cargo build` — 0 warnings (with suppressions)
- Goal: 0 warnings WITHOUT suppressions

</code_context>

<specifics>
## Specific Ideas

No specific requirements — thorough cleanup with Claude's judgment on each item.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-tech-debt-cleanup*
*Context gathered: 2026-03-31*
