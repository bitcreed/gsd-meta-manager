# Phase 13: Queue Execution Research - Context

**Gathered:** 2026-03-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce a design document that enables v1.3 implementation of queue execution without further research. The design must be GSD-centric and LLM-agnostic — it integrates with GSD's workflow hooks, planning artifacts, and session lifecycle, not directly with any specific LLM CLI.

</domain>

<decisions>
## Implementation Decisions

### Research Scope (GSD-Centric)
- Research GSD's workflow hooks, planning artifact lifecycle, and session management by scanning the GSD source at `~/projects/node/get-shit-done`
- Focus on how QUEUE.md items get executed through GSD commands — the TUI dispatches GSD workflows, not raw LLM calls
- Scan GSD prompts, hooks, workflows, and bin/ scripts to understand integration points
- The design must be LLM-agnostic: GSD currently uses Claude but the queue execution design should not assume any specific LLM backend

### Document Structure
- Single design document: `.planning/phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md`
- Sections must map to success criteria: GSD autonomous mode lifecycle, hook points, integration strategies, trade-offs, safety requirements
- Confidence levels (HIGH/MEDIUM/LOW) on each design element per QRES-02

### Integration Strategies
- At least 2 strategies for auto-continue from QUEUE.md, with a recommended pick
- Each strategy must identify: trigger mechanism (how TUI starts execution), session lifecycle (how GSD manages the run), artifact flow (how results get back to TUI), error handling

### Safety Requirements
- Practical safety rules: timeout limits, error handling, human escalation triggers, max retry counts
- Consider: what happens if a queued item fails mid-execution, how to prevent runaway loops, how to surface errors back to the TUI dashboard

### Claude's Discretion
- Internal document organization beyond the required sections
- How deep to go on each GSD subsystem
- Whether to include sequence diagrams or architecture sketches

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### GSD Source (primary research target)
- `~/projects/node/get-shit-done/` — GSD project root, locally checked out
- `~/projects/node/get-shit-done/bin/` — CLI tools and entry points
- `~/projects/node/get-shit-done/workflows/` — Workflow definitions (autonomous.md, execute-phase.md, etc.)
- `~/projects/node/get-shit-done/references/` — Reference docs (hooks, checkpoints, etc.)
- `~/.claude/get-shit-done/` — Installed GSD (may differ from source)

### Existing queue implementation
- `src/state_reader/queue_md.rs` — Current QUEUE.md parser
- `src/ui/screens/detail.rs` — Queue tab rendering and CRUD operations

</canonical_refs>

<code_context>
## Existing Code Insights

### Queue System (Current State)
- `QueuedAction` struct in `queue_md.rs` — has command, description, priority fields
- Queue tab supports: add, edit, delete, reorder, mark done
- Queue items stored in `.planning/QUEUE.md` per project
- No execution capability exists yet — this phase researches how to add it

### GSD Integration Points (Known)
- GSD uses `/gsd:*` skill commands dispatched via Claude Code
- Workflows chain: discuss → plan → execute → verify
- `gsd-tools.cjs` provides CLI utilities for state management
- Autonomous mode (`/gsd:autonomous`) runs full milestone cycles

</code_context>

<specifics>
## Specific Ideas

- Scan GSD source at `~/projects/node/get-shit-done` for hooks, workflow entry points, and session management patterns
- Look at how `/gsd:autonomous` chains phases — this is the closest existing pattern to queue execution
- Consider how the TUI's existing `ScreenAction::DispatchAction` could trigger GSD workflows

</specifics>

<deferred>
## Deferred Ideas

- Actual implementation of queue execution (v1.3+)
- Container-based execution (backlog 999.2)
- Remote project execution

</deferred>

---

*Phase: 13-queue-execution-research*
*Context gathered: 2026-03-31*
