# Feature Research

**Domain:** GSD TUI meta-manager v1.2 — housekeeping, archive browser, queue execution research
**Researched:** 2026-03-31
**Confidence:** MEDIUM-HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features that complete the v1.2 milestone promise. Missing these = milestone feels unfinished.

| Feature | Why Expected | Complexity | Depends On | Notes |
|---------|--------------|------------|------------|-------|
| Paused project detection (HANDOFF.md badge) | Users pause work with `/gsd:pause-work`; dashboard should reflect paused state with a `\|\|` badge | LOW | `state_reader/mod.rs`, `ProjectState` struct | Check for `.planning/HANDOFF.md` or `.planning/HANDOFF.json` existence; add `has_handoff: bool` to `ProjectState`; todo already documented |
| Fix stale integration test | `end_to_end_add_then_list_via_cli` uses old CLI arg order (`add <alias> <path>` vs `add <path> [alias]`); broken test = CI rot | LOW | `tests/` directory | Identified in v1.1 milestone audit; mechanical fix |
| Resolve compiler warnings | 11 warnings (unused fields, dead code from future-facing APIs) accumulated over v1.1 | LOW | Various source files | Prefix unused fields with `_` or add `#[allow(dead_code)]` where intentional |
| Deferred visual UAT | 4 visual checks from Phase 06 never run by human (tab nav, backlog split-pane, git scrolling, diff stats) | LOW | Running binary, human tester | Not code work — verification work; document results |

### Differentiators (Competitive Advantage)

Features that make v1.2 a meaningful upgrade. Not universally expected, but high value.

| Feature | Value Proposition | Complexity | Depends On | Notes |
|---------|-------------------|------------|------------|-------|
| Milestone Archive Browser tab | Browse completed milestones and drill into past phase artifacts (SUMMARYs, VERIFICATIONs, PLANs, CONTEXTs) from TUI — turns the app into a project archaeology tool, not just a status viewer | MEDIUM | Existing detail view tab system, filesystem reading | Uses the 8th tab slot (currently empty); milestone data already on disk in `.planning/milestones/` |
| Queue execution research document | Design document for how queue items become executable Claude sessions — covers headless mode, auto-approve, session chaining, and completion detection | LOW | Understanding of Claude Code CLI, GSD workflows | Research-only deliverable; no code changes; informs v1.3+ implementation |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-execute queue items on project open | "If I queued it, I want it done" | Stale items, destructive commands, missing context; auto-execution without confirmation violates project constraint of non-intrusiveness | Manual trigger with confirmation dialog; research document covers safe patterns |
| Full milestone diff viewer | "Show me everything that changed in v1.0" | Combines potentially thousands of git commits; rendering time and memory explosion; scope creep into gitui territory | Show milestone-level summary (phase count, date range, requirement coverage) and link to existing Git tab for commit details |
| Editable archive files | "Let me fix that old SUMMARY" | Archive files are historical record; editing breaks audit trail and may conflict with git history | Read-only view; direct user to editor for intentional modifications |
| Tree view with full expand/collapse for archive | "I want a filesystem explorer" | Over-engineered for the actual data shape; milestones have only 2-3 nesting levels; tree widget adds a dependency for marginal benefit | Flat list of milestones, then flat list of phases within milestone, then flat list of files within phase — three levels of drill-down using existing List widget |
| Real-time queue execution status in dashboard | "Show me running/completed/failed badges live" | Requires process monitoring, exit code capture, and session lifecycle tracking — this is the full queue execution feature, not a v1.2 scope item | Research document captures the design; implement in v1.3+ |

---

## Feature Details

### 1. Paused Project Detection (Table Stakes, LOW)

Detect `.planning/HANDOFF.md` (or `.planning/HANDOFF.json`) and show a pause indicator on the dashboard.

**Implementation:**
1. Add `has_handoff: bool` to `ProjectState` struct
2. In `parse_project_state()`, check `planning_dir.join("HANDOFF.md").exists() || planning_dir.join("HANDOFF.json").exists()`
3. In `NormalScreen` dashboard row, prepend `||` badge (dim yellow) to alias when `state.has_handoff` is true
4. Badge coexists with session indicator (`>`) — show both if applicable

**Why HANDOFF.md and not just HANDOFF.json:** GSD's `/gsd:pause-work` creates both `HANDOFF.json` (machine-readable) and `.continue-here.md` (human-readable). The JSON file is the reliable indicator. However, some older GSD versions may only produce `HANDOFF.md`. Check for both.

**Confidence:** HIGH — todo already documented in `.planning/todos/pending/`, implementation path is clear, minimal risk.

### 2. Tech Debt Cleanup (Table Stakes, LOW)

Three items from v1.1 milestone audit, all mechanical:

**Stale integration test:** Update `end_to_end_add_then_list_via_cli` to use current CLI arg order (`add <path> [alias]`). Verify with `cargo nextest run`.

**Compiler warnings:** Address 11 warnings. Strategy:
- Unused fields that are future-facing: add `#[allow(dead_code)]` with a comment explaining the intent
- Truly dead code: remove it
- Unused imports: remove them

**Visual UAT:** Run the binary and manually verify 4 deferred checks from Phase 06. Document results in a UAT file.

**Confidence:** HIGH — all items already identified and scoped in the audit.

### 3. Milestone Archive Browser Tab (Differentiator, MEDIUM)

A new tab in the detail view for browsing completed milestone artifacts.

**Data source:** `.planning/milestones/` directory, structured as:
```
.planning/milestones/
  v1.0-MILESTONE-AUDIT.md
  v1.0-REQUIREMENTS.md
  v1.0-ROADMAP.md
  v1.0-phases/
    01-core-infrastructure/
      01-CONTEXT.md
      01-RESEARCH.md
      01-01-PLAN.md
      01-01-SUMMARY.md
      01-VERIFICATION.md
      ...
    02-dashboard-and-navigation/
      ...
  v1.1-MILESTONE-AUDIT.md
  v1.1-REQUIREMENTS.md
  v1.1-ROADMAP.md
  v1.1-phases/
    05-state-reader-accuracy/
      ...
```

**UX pattern: Three-level drill-down (no tree widget needed)**

Level 1 — Milestone list:
```
  Archive
  -------
  > v1.1 — Polish & Power Features  (5 phases, 21 reqs)
    v1.0 — MVP                       (4 phases, 22 reqs)
```
- Scrollable list of milestone versions
- Show phase count, requirement count, completion date from MILESTONE-AUDIT.md
- `Enter` to drill into selected milestone

Level 2 — Phase list within milestone:
```
  v1.1 > Phases
  -------------
  > 05 State Reader Accuracy    (5 plans)
    06 Read-Only Views           (4 plans)
    07 Execution Flow & GSD      (3 plans)
    08 Queue Execution           (2 plans)
    09 Claude Session Mgmt       (2 plans)

  [AUDIT] [REQUIREMENTS] [ROADMAP]
```
- List phases from `vX.Y-phases/` subdirectories
- Parse phase directory names for number and slug (same pattern as existing phase parsing)
- Bottom section: milestone-level documents (audit, requirements, roadmap)
- `Enter` to drill into phase files; `Esc`/`Backspace` to go back to milestone list

Level 3 — File list within phase:
```
  v1.1 > 05 State Reader > Files
  --------------------------------
  > 05-CONTEXT.md
    05-DISCUSSION-LOG.md
    05-RESEARCH.md
    05-01-PLAN.md
    05-01-SUMMARY.md
    ...
    05-VERIFICATION.md
    05-HUMAN-UAT.md
```
- List `.md` files in the phase directory
- `Enter` to view file content in a scrollable markdown viewer (reuse existing backlog content preview pattern)
- `Esc`/`Backspace` to go back to phase list

**Why not use `tui-tree-widget`:** The milestone hierarchy has exactly 3 fixed levels (milestone > phase > file). A tree widget adds a dependency and visual complexity (expand/collapse icons, indentation management) for a structure that is better served by sequential drill-down. The existing `List` + `ListState` pattern used by Backlog and Git tabs is proven and consistent. Users navigate with `Enter` to go deeper and `Esc` to go back — the same pattern already used in the backlog browser.

**Implementation approach:**
1. Add `DetailSubView::Archive` variant and extend `TAB_TITLES` to 8 tabs
2. Create `archive` module in `src/state_reader/` to scan `.planning/milestones/` directory
3. Data model:
   ```rust
   pub struct MilestoneArchive {
       pub version: String,        // "v1.0", "v1.1"
       pub name: String,           // from ROADMAP.md or directory name
       pub phase_count: u32,
       pub requirement_count: u32,
       pub completed_date: Option<String>,
       pub phases: Vec<ArchivedPhase>,
       pub docs: Vec<PathBuf>,     // milestone-level .md files
   }

   pub struct ArchivedPhase {
       pub number: String,         // "01", "05"
       pub name: String,           // "core-infrastructure"
       pub plan_count: u32,
       pub files: Vec<PathBuf>,
   }
   ```
4. Archive data is static (completed milestones don't change) — parse once, cache forever, no file watching needed
5. Render using existing `List` widget pattern with breadcrumb navigation in the block title

**Complexity assessment:** MEDIUM because:
- Directory parsing is straightforward (glob + sort)
- The three-level navigation requires state management for "which level am I on" and "what's selected at each level"
- Markdown content viewing reuses existing backlog preview code
- No new dependencies needed

**Confidence:** HIGH — directory structure is stable and well-understood; UI pattern matches existing tabs.

### 4. Queue Execution Research (Differentiator, LOW complexity — research only)

Design document covering how queue items can become executable Claude sessions in a future milestone. No code changes in v1.2.

**Key findings from research:**

**Claude Code CLI capabilities (verified from official docs):**
- `claude -p "{command}"` — headless execution, prints result, exits
- `claude -p "{command}" --output-format json` — structured output with `result`, `session_id`, `usage`
- `claude -p "{command}" --allowedTools "Bash,Read,Edit"` — auto-approve specific tools
- `claude --continue` — continue most recent session in the project directory
- `claude --resume {session-id}` — resume a specific session
- `--bare` mode — skip auto-discovery of hooks/MCP/CLAUDE.md; deterministic execution
- `--output-format stream-json` — real-time token streaming for progress monitoring

**Auto-continue pattern (inspired by Ralph TUI):**
The Ralph TUI project demonstrates the canonical "auto-continue" queue pattern for AI agent orchestration:
1. Task queue holds structured work items
2. Orchestrator selects next task, constructs prompt with context
3. Agent executes autonomously
4. Completion detection triggers (exit code, output parsing, file change)
5. Orchestrator marks done, selects next task
6. Space to start, `p` to pause — user retains control

**Proposed execution modes for gsd-meta-manager:**

| Mode | Command | When to Use | Safety |
|------|---------|-------------|--------|
| Interactive | Open terminal, user types command | Complex/ambiguous tasks | Safest — user in the loop |
| Headless single | `claude -p "{cmd}" --output-format json` | Simple, well-defined commands | Medium — auto-approve needed |
| Headless chain | Sequential `claude -p` with `--continue` | Multiple related queue items | Medium — session context preserved |
| Auto-continue | Loop: execute item, detect completion, next item | Batch processing | Risky — needs safeguards |

**GSD-specific integration points:**
- `/gsd:quick "{task}"` — self-contained tasks; ideal for headless execution
- `/gsd:execute-phase` — structured phase execution; needs full GSD context
- `/gsd:autonomous` — drives all remaining phases; long-running, needs monitoring
- `/gsd:do "{task}"` — routes to appropriate GSD command; good for arbitrary queue items

**Safety requirements for auto-continue:**
1. Confirmation before starting batch execution
2. Pause capability (user presses `p` to stop after current item)
3. Failure stops the chain (don't execute item N+1 if item N failed)
4. Session isolation — each queue item gets its own session or explicitly chains
5. Timeout per item (configurable, default 10 minutes)
6. Exit code / output validation to determine success/failure
7. Queue items marked with execution timestamp and result

**Completion detection strategies:**
- Process exit code (0 = success, non-zero = failure)
- JSON output parsing (`--output-format json` includes `result` field)
- File change detection via existing `notify` watcher (`.planning/` changes after execution)
- Session JSONL tail check (last message indicates completion)

**What to defer past v1.3:**
- Streaming output display in TUI (complex, marginal value for status monitoring)
- Parallel queue execution (multiple items simultaneously)
- Queue item dependencies (execute B only after A succeeds)
- Remote execution (SSH to other machines)

**Confidence:** HIGH for CLI capabilities (verified from official docs). MEDIUM for auto-continue pattern (based on Ralph TUI architecture, not yet implemented in this codebase). LOW for completion detection reliability (untested with real GSD workflows).

---

## Feature Dependencies

```
Paused project detection
    (independent, no blockers)

Tech debt cleanup
    (independent, no blockers)

Deferred visual UAT
    (independent, no blockers)

Milestone Archive Browser
    └──requires──> Existing detail view tab system (built in v1.1)
    └──requires──> Existing List/ListState pattern (built in v1.1)
    └──reuses───> Backlog content preview renderer (built in Phase 06)

Queue Execution Research
    (independent — research document only, no code dependencies)
```

### Dependency Notes

- **Archive Browser requires tab system:** The 7-tab detail view was built in Phase 06. Adding an 8th tab is mechanical (extend `TAB_TITLES`, add `DetailSubView::Archive`, add match arm). The pattern is well-established.
- **Archive Browser reuses backlog preview:** The backlog browser already renders markdown content in a scrollable pane. The archive file viewer can reuse the same rendering logic.
- **No cross-dependencies between v1.2 features:** All four feature areas (pause detection, tech debt, archive browser, queue research) can be worked on in any order or in parallel.

## MVP Definition

### v1.2 Scope (This Milestone)

- [x] Paused project detection via HANDOFF.md badge -- completes dashboard status picture
- [x] Fix stale integration test -- CI hygiene
- [x] Resolve 11 compiler warnings -- code quality
- [x] Complete deferred visual UAT from v1.1 -- verification debt
- [x] Milestone Archive Browser tab -- the headline feature; browse past work
- [x] Queue execution research document -- design for v1.3 implementation

### Defer to v1.3 (Queue Execution Implementation)

- [ ] Headless queue execution (`claude -p`) -- implement the researched design
- [ ] Auto-continue mode -- batch queue processing with safety controls
- [ ] Execution status tracking in queue view -- pending/running/done/failed badges

### Future Consideration (v2+)

- [ ] Container support with Claude command injection (backlog 999.2) -- major feature, needs architecture
- [ ] Plugin system / extensibility -- per PROJECT.md, deferred until core stabilizes
- [ ] Remote project management (SSH) -- local-first per constraints

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Paused project detection | MEDIUM | LOW | P1 |
| Fix stale integration test | LOW | LOW | P1 |
| Resolve compiler warnings | LOW | LOW | P1 |
| Visual UAT completion | MEDIUM | LOW | P1 |
| Milestone Archive Browser | HIGH | MEDIUM | P1 |
| Queue execution research | MEDIUM | LOW | P1 |
| Headless queue execution | HIGH | HIGH | P2 (v1.3) |
| Auto-continue mode | MEDIUM | HIGH | P3 (v1.3+) |

**Priority key:**
- P1: v1.2 scope, ship this milestone
- P2: v1.3 scope, designed in v1.2 research
- P3: Future scope, informed by research

## Analogous TUI Archive/History Browsers

| Pattern | Example App | How They Do It | Our Approach |
|---------|-------------|----------------|--------------|
| Git log browsing | gitui, lazygit | Scrollable list with detail pane on Enter | Already built in Git tab; archive browser follows same pattern |
| File tree navigation | ranger, lf, yazi | Three-column: parent/current/preview | Too complex for 3-level hierarchy; drill-down with breadcrumbs is simpler and matches existing UX |
| Nested list drill-down | k9s (Kubernetes TUI) | Select namespace > pods > containers; Esc to go back | Exactly our pattern: milestone > phase > file with Esc to go back |
| Read-only document viewer | glow (markdown TUI) | Scrollable rendered markdown | We render raw markdown lines (no formatting); sufficient for viewing PLANs/SUMMARYs |

## Sources

- [Claude Code headless/programmatic docs](https://code.claude.com/docs/en/headless) -- `-p` flag, `--output-format json`, `--allowedTools`, `--continue`, `--resume`, `--bare` mode (HIGH confidence, official docs)
- [Ralph TUI](https://peerlist.io/leonardo_zanobi/articles/ralph-tui-ai-agent-orchestration-that-actually-works) -- auto-continue queue execution pattern for AI agent orchestration (MEDIUM confidence, single project)
- [tui-tree-widget](https://crates.io/crates/tui-tree-widget) v0.24.0 -- tree widget for ratatui; evaluated and rejected for archive browser (HIGH confidence, crates.io)
- [ratatui-explorer](https://github.com/tatounee/ratatui-explorer) -- file explorer widget for ratatui; evaluated, too heavy for our use case (MEDIUM confidence)
- `.planning/milestones/` directory structure -- verified locally; 2 milestones (v1.0, v1.1), 9 phases total, consistent naming conventions (HIGH confidence)
- `.planning/todos/pending/2026-03-27-detect-paused-projects-via-handoff-md-badge.md` -- existing todo for pause detection (HIGH confidence)
- v1.1 milestone audit -- tech debt items enumerated with phase attribution (HIGH confidence)
- GSD `/gsd:pause-work` workflow source -- creates HANDOFF.json and .continue-here.md (HIGH confidence, read from source)
- [k9s](https://github.com/derailed/k9s) -- Kubernetes TUI drill-down pattern reference (HIGH confidence, well-known project)

---
*Feature research for: GSD Meta Manager v1.2 — Housekeeping & Archive Browser*
*Researched: 2026-03-31*
