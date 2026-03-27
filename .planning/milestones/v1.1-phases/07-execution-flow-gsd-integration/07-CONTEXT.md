# Phase 07: Execution Flow & GSD Integration - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Add per-phase pipeline visualization showing the 5 GSD workflow stages (Discuss, Research, Plan, Execute, Verify) with color-coded status. Add [verified] vs [inferred] badges to status fields so users can distinguish facts from heuristics. Extends the existing DiskStatus/DiskInference infrastructure from Phase 05.

</domain>

<decisions>
## Implementation Decisions

### Pipeline Visualization Layout
- **D-01:** Pipeline visualization lives as a new tab ("Pipeline") in the detail view, reusing the tab system from Phase 06
- **D-02:** Horizontal box format: `[D]─[R]─[P]─[E]─[V]` with color per stage
- **D-03:** Plan progress shown inside Execute box: `[E 2/3]` (completed/total plans)
- **D-04:** Reuse and extend `phase_disk_statuses` / `DiskInference` from Phase 05 rather than creating a new data model

### Fact vs Inference Badges
- **D-05:** Badges appear next to status fields in the detail view: `Executing [inferred]` or `Complete [verified]`
- **D-06:** Verified = has SUMMARY.md and/or VERIFICATION.md artifacts; Inferred = disk heuristic only (no authoritative GSD artifacts)
- **D-07:** Badge styling: dim text — `[verified]` in green, `[inferred]` in dark gray
- **D-08:** GSD integration opt-in via config toggle in `~/.config/gsd-manager/config.toml`: `gsd_integration = true` (default false)

### Stage Detection Logic
- **D-09:** Extend `DiskInference` with per-stage booleans: `has_context`, `has_research`, `has_plans`, `has_summaries`, `has_verification`
- **D-10:** Skipped stages shown as dimmed `[─]` — a stage is "skipped" if a later stage is complete but this one has no artifacts
- **D-11:** Stage colors: Green = complete, Yellow = current/active, DarkGray = not started, Magenta = skipped
- **D-12:** Per-phase pipeline in the Pipeline tab — user selects a phase to see its detailed stage breakdown

### Claude's Discretion
- Pipeline tab layout proportions (phase list vs pipeline detail split)
- Exact box-drawing characters and spacing
- Whether to show pipeline summary in the Phases tab as well
- Config file creation on first toggle

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/state_reader/disk_status.rs` — DiskStatus enum (7 variants), DiskInference struct, infer_disk_status()
- `src/state_reader/mod.rs` — ProjectState with phase_disk_statuses HashMap
- `src/ui/screens/detail.rs` — DetailSubView enum, tab system (1-4 keys), render infrastructure
- `src/ui/screens/normal.rs` — compact_pipeline() already renders D-R-P-E-V with colors

### Established Patterns
- Tab system: DetailSubView enum + number-key switching + Tabs widget header
- Async data loading: Action variants + spawn_blocking + cache in ProjectViewCache
- DiskInference already has `status`, `plan_count`, `summary_count` — extend with stage booleans

### Integration Points
- DetailSubView needs Pipeline variant (tab 5)
- DiskInference needs has_context, has_research, has_plans, has_summaries, has_verification fields
- Config struct needs gsd_integration toggle
- Normal screen compact_pipeline could optionally show badges too

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Cached gsd-tools.cjs JSON output for richer state (GSD-01 advanced) — beyond file existence checks
- Interactive pipeline stage drilling (click a stage to see artifacts) — future enhancement
- Pipeline animation for active stages — cosmetic, defer

</deferred>

---

*Phase: 07-execution-flow-gsd-integration*
*Context gathered: 2026-03-27 via Smart Discuss (autonomous mode)*
