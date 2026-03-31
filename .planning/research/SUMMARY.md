# Project Research Summary

**Project:** GSD Meta Manager
**Domain:** Rust TUI — milestone archive browser, paused-project detection, tech debt cleanup, queue execution research (v1.2)
**Researched:** 2026-03-31
**Confidence:** HIGH

## Executive Summary

This is a v1.2 housekeeping-and-features milestone for an existing, production-quality Rust TUI application. The core stack (ratatui 0.30, crossterm 0.29, tokio 1.50, notify-debouncer-full 0.5) is validated and stable. No new Cargo dependencies are needed for any v1.2 feature — all deliverables are achievable with the existing codebase and established patterns. This is an incremental milestone, not a greenfield build, and the research consistently reinforces reuse of existing architecture over introducing new abstractions.

The recommended approach is to work in dependency order: clean up tech debt first (establishes a safe baseline for testing), then add paused-project detection (smallest scope, immediate user value, independent of the archive tab), then build the milestone archive browser (the headline feature, MEDIUM complexity), and finally produce the queue execution research document (no code, any order). All four work areas are fully independent and could be parallelized if bandwidth allows. The archive browser is the only feature with meaningful complexity — it requires three-level drill-down navigation and async file loading — but both patterns are already established in the codebase (backlog browser and git log loader provide exact templates).

The primary risks are architectural rather than technological: blocking the render loop with synchronous archive directory scanning, accepting a shallow HANDOFF.json existence check that produces stale paused badges, and removing dead-code annotations without auditing which ones are needed by the incoming archive browser. All three risks have clear mitigations documented in PITFALLS.md. A secondary UX risk is tab bar overflow at 80 columns when adding an 8th tab — this must be resolved at design time (shorten labels or use a separate screen) before implementing the archive browser UI.

---

## Key Findings

### Recommended Stack

No new dependencies are required. The v1.2 features map entirely onto existing crates: `std::fs::read_dir` (via `tokio::spawn_blocking`) for archive scanning, `serde_json` for HANDOFF.json parsing, and the existing `List` + `ListState` widget pattern for three-level navigation. The `tui-tree-widget` (0.24.0) and `tui-markdown` (0.3.7) crates were evaluated and explicitly rejected — the archive hierarchy is only 3 levels deep (making a tree widget unnecessary), and raw markdown renders acceptably for read-only planning artifacts.

**Core technologies (unchanged from v1.1):**
- **ratatui 0.30 + crossterm 0.29**: TUI rendering and terminal backend — the de facto standard, modular in 0.30
- **tokio 1.50**: Async runtime — required for concurrent file watching + event handling without blocking render
- **notify-debouncer-full 0.5**: Filesystem watching — 200ms debounce for `.planning/` directory change events
- **serde + serde_json**: JSON parsing — HANDOFF.json deserialization with graceful failure handling
- **anyhow + color-eyre**: Error handling — ergonomic propagation through async event loops

**Explicitly not adding:**
- `tui-tree-widget`: 3-level archive hierarchy does not justify a new widget paradigm alongside the established `ListState` pattern
- `tui-markdown` / `pulldown-cmark`: Raw markdown is acceptable for read-only planning artifacts; upgrade path exists (one-line change) if needed later
- `walkdir`: Nested `std::fs::read_dir` is sufficient for 3-level milestone structure

See `.planning/research/STACK.md` for full version table, evaluated alternatives, and compatibility matrix.

### Expected Features

**Must have (table stakes):**
- **Paused project detection** — HANDOFF.json badge on dashboard; project was paused with `/gsd:pause-work` and dashboard should reflect it
- **Fix stale integration test** — `end_to_end_add_then_list_via_cli` uses wrong CLI arg order; broken test is CI rot
- **Resolve 11 compiler warnings** — accumulated from v1.1 rapid development; clean foundation for v1.2 code
- **Deferred visual UAT** — 4 human verification checks from Phase 06 never ran; verification debt must close

**Should have (differentiators):**
- **Milestone Archive Browser tab** — browse completed milestones and drill into past phase artifacts (PLAN, SUMMARY, VERIFICATION, CONTEXT files); turns the app from status viewer to project archaeology tool
- **Queue execution research document** — design for how queue items become executable Claude sessions in v1.3; covers headless mode, auto-approve, session chaining, completion detection

**Defer to v1.3:**
- Headless queue execution (`claude -p`) — implement the researched design
- Auto-continue mode — batch queue processing with safety controls
- Execution status tracking in queue view with running/done/failed badges

**Defer to v2+:**
- Container support with direct command injection (backlog 999.2)
- Plugin system / extensibility
- Remote project management (SSH)

See `.planning/research/FEATURES.md` for full feature dependency graph, anti-features list, and analogous TUI patterns.

### Architecture Approach

The codebase follows TEA (The Elm Architecture) with a screen stack. All state mutation flows through `App::update()`; rendering is pure. The v1.2 features integrate cleanly into this model: HANDOFF.json detection extends the existing `StateReader` flow (file change event → parse → update cache → redraw), and the archive browser adds an 8th tab to `DetailScreen` using the same async-load-on-tab-switch pattern already used by the Git and Backlog tabs. No new screens or architectural patterns are needed.

**New files (2 total):**
1. `src/state_reader/handoff.rs` (~50 lines) — parse HANDOFF.json into `PauseInfo` struct
2. `src/state_reader/milestones.rs` (~120 lines) — scan `milestones/` directory for `ArchivedMilestone` structs

**Modified files (8 total):**
- `state_reader/mod.rs` — add `is_paused: bool`, `pause_info: Option<PauseInfo>` to `ProjectState`; call `handoff::parse_handoff()` in `parse_project_state()`
- `app.rs` — add `DetailSubView::Archives` variant; handle new action variants in `App::update()`
- `action.rs` — two new `Action` variants: `ArchivesLoaded` and `ArchiveContentLoaded`
- `ui/screens/mod.rs` — add archive browse state fields (`archive_depth`, selection indices, content cache) to `ProjectViewCache`
- `ui/screens/normal.rs` — add pause badge (cyan pause icon, priority over session badge) to alias cell
- `ui/screens/detail.rs` — add Archives tab rendering; add pause info panel in PhaseList tab

**Key data flow additions:**
- HANDOFF.json detection: automatic via existing watcher; `FileChanged` → `parse_project_state()` → `handoff::parse_handoff()` → badge update
- Archive loading: lazy on first tab activation; `spawn_blocking(scan_milestones)` → `Action::ArchivesLoaded` → cache; artifact content loaded on-demand via `Action::ArchiveContentLoaded`

See `.planning/research/ARCHITECTURE.md` for complete component boundaries, data flow diagrams, navigation patterns, and anti-patterns to avoid.

### Critical Pitfalls

1. **Archive browser blocks render loop** — synchronous directory recursion on a 3-4 level tree blocks TUI render. Use `tokio::spawn_blocking` for all archive I/O; never read files in `render()`. Establish this pattern in the first plan task before building UI widgets.

2. **HANDOFF.json race conditions** — file watcher can fire while GSD is mid-write, producing truncated JSON. Wrap serde parse in `Result`; on failure retain previous state and retry after 500ms. Use permissive deserialization (no `deny_unknown_fields`) for forward compatibility.

3. **HANDOFF.json existence-only check produces stale badges** — file may persist after resume if GSD cleanup failed. Check `"status": "paused"` in content AND cross-reference against recent git commit timestamp for staleness detection.

4. **Tech debt removal deletes needed API surface** — 7 `#[allow(dead_code)]` locations include `ScreenAction` variants the archive browser will need. Audit each annotation individually before removing; run `cargo nextest run` after every single removal, not in batch.

5. **Tab bar overflow at 80 columns** — current 7 tabs are ~85 characters; an 8th adds 10+, exceeding standard 80-column terminals. Abbreviate labels (e.g., `8:Ar`) with full names in the help overlay, OR implement Archive as a separate screen pushed onto `screen_stack`. Resolve at design time before any widget work.

See `.planning/research/PITFALLS.md` for the full pitfall list including 7 critical pitfalls, performance traps, UX pitfalls, integration gotchas, and a "looks done but isn't" checklist.

---

## Implications for Roadmap

Based on research, suggested phase structure (4 phases, all fully independent):

### Phase 1: Tech Debt Cleanup
**Rationale:** Clean foundation before adding new code. Removes ambiguity about which dead-code annotations to keep, ensures tests pass as a baseline, and closes verification debt from v1.1. Pitfall 3 (deleting a `ScreenAction` variant the archive browser needs) is prevented by doing this cleanup first with per-annotation auditing.
**Delivers:** Zero compiler warnings, passing integration tests with correct CLI arg order, completed visual UAT documentation
**Addresses:** Fix stale integration test, resolve 11 compiler warnings, deferred visual UAT (all table stakes)
**Avoids:** Pitfall 3 (tech debt cleanup breaking screen dispatch API needed by archive browser)
**Research flag:** None needed — all items are already enumerated in the v1.1 milestone audit with file and line references

### Phase 2: Paused Project Detection
**Rationale:** Smallest scope with immediate user value. Modifies only `state_reader/mod.rs`, `normal.rs`, and `detail.rs` (PhaseList section) — independent of the archive browser. Building this before the archive browser keeps the code surface minimal and makes the HANDOFF.json reader testable in isolation before the more complex async patterns of Phase 3.
**Delivers:** Pause badge on dashboard (cyan pause icon, higher priority than active-session badge), pause info panel in PhaseList tab, `state_reader/handoff.rs` with defensive error handling and staleness detection
**Addresses:** Paused project detection (table stakes)
**Avoids:** Pitfall 2 (HANDOFF.json race condition), Pitfall 6 (existence-only check producing stale badges)
**Research flag:** None needed — implementation path fully specified in ARCHITECTURE.md with exact Rust code snippets

### Phase 3: Milestone Archive Browser
**Rationale:** The headline feature with the highest complexity in v1.2. Benefits from a clean, warning-free codebase (Phase 1) and can be built independently of Phase 2. Three-level drill-down navigation (milestone → phase → artifact) and async content loading are the only non-trivial design elements, and both have direct precedents (backlog split-pane and git log async loader).
**Delivers:** New "8:Archives" tab in DetailScreen, `src/state_reader/milestones.rs`, drill-down navigation with breadcrumbs, on-demand artifact content viewing with scroll support
**Addresses:** Milestone Archive Browser (differentiator)
**Avoids:** Pitfall 1 (render loop blocking — lazy loading only), Pitfall 5 (raw markdown text walls — implement minimal heading/code-block styling), Pitfall 7 (tab bar overflow — resolve label abbreviation strategy at design time)
**Research flag:** Low — archive directory structure and navigation pattern are fully specified. One open design decision (tab label abbreviation vs. separate screen) should be resolved at plan time with a quick 80x24 terminal test. If markdown styling with `pulldown-cmark` is in scope, that sub-task warrants a focused research spike (~50-100 lines of mapping code).

### Phase 4: Queue Execution Research Document
**Rationale:** Documentation-only deliverable, no code changes. Fully independent — can run in parallel with any other phase or at the end. The output is a design document for v1.3 implementation, not code.
**Delivers:** `.planning/research/QUEUE-EXECUTION-DESIGN.md` covering GSD CLI invocation patterns, session lifecycle, three integration strategies (launch-time injection, autonomous pre-phase hook, phase insertion), safety requirements, and completion detection strategies
**Addresses:** Queue execution research (differentiator)
**Avoids:** Pitfall 4 (designing for stdin injection instead of CLI invocation — research must reference `autonomous.md`, `pause-work.md`, `resume-project.md` by name)
**Research flag:** Core research already completed in FEATURES.md. The document itself is the deliverable. No additional `/gsd:research-phase` needed.

### Phase Ordering Rationale

- Tech debt first prevents the specific pitfall where the archive browser needs a `ScreenAction` variant that was deleted during batch cleanup
- Paused detection second is the lowest-risk code change; validates the HANDOFF.json reader pattern (defensive serde, retry on failure) before archive browser adds more complex async patterns on top
- Archive browser third has the largest new code surface (2 new files, 6 modified files) and benefits from a clean, warning-free codebase with a validated async pattern
- Queue execution research has no code dependencies and can float to any position; placing it last prevents it from being blocked if other phases consume more time than estimated

### Research Flags

Phases likely needing `/gsd:research-phase` during planning:
- **Phase 3 (Archive Browser):** One design decision requires resolution before coding — tab bar overflow at 80 columns (abbreviate labels vs. separate screen). A 15-minute prototype test at 80x24 during plan creation is sufficient; a full research phase is not warranted.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Tech Debt):** All items already enumerated in the milestone audit with file/line references; execution is mechanical
- **Phase 2 (Paused Detection):** Architecture fully specified in ARCHITECTURE.md with concrete Rust code; HANDOFF.json schema verified from a live example
- **Phase 4 (Queue Research):** The research is done; the deliverable is writing it up as a design document

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified via crates.io API; no new dependencies simplifies the risk surface entirely |
| Features | HIGH | All features grounded in existing codebase state, GSD workflow files read directly, and live HANDOFF.json schema verified |
| Architecture | HIGH | Based on direct analysis of all 32 source files; component boundaries and data flow are concrete with exact file/line references |
| Pitfalls | HIGH | Identified from actual code (not speculative); file locations and line numbers cited; HANDOFF.json race verified from live pause workflow |

**Overall confidence:** HIGH

### Gaps to Address

- **Tab bar layout at 80 columns:** PITFALLS.md flags overflow but the choice between abbreviated labels and a separate screen is a UX design decision, not a research gap. Resolve at Phase 3 plan time with a quick 80x24 terminal test. Recommendation: try abbreviated labels first (`8:Ar`); fall back to separate screen if any existing tab becomes unreadable.
- **HANDOFF.json staleness threshold:** The staleness heuristic (compare HANDOFF.json timestamp against most recent git commit) needs a concrete threshold. Recommendation: flag as stale if the most recent git commit postdates the handoff file by more than 1 hour. Validate against real-world pause/resume cycles during Phase 2 verification.
- **Markdown styling scope for archive viewer:** PITFALLS.md recommends minimal markdown-to-styled-spans conversion via pulldown-cmark (~50-100 lines). STACK.md recommends starting with raw text and upgrading to `tui-markdown` if needed. These are compatible starting points; the Phase 3 plan must pick one. Recommendation: start with raw text (consistent with existing backlog tab); treat markdown styling as an optional stretch goal within Phase 3.
- **Queue execution design completeness:** Claude Code CLI capabilities are at HIGH confidence; the auto-continue pattern (inspired by Ralph TUI) is at MEDIUM; completion detection reliability with real GSD workflows is at LOW (untested). The Phase 4 research document should be explicit about these confidence levels to guide v1.3 planning.

---

## Sources

### Primary (HIGH confidence)
- crates.io API — tui-tree-widget 0.24.0, tui-markdown 0.3.7 evaluated and rejected; existing stack versions confirmed unchanged
- Direct codebase analysis — all 32 source files in `src/`, integration tests in `tests/`, v1.1 milestone audit
- GSD workflow files — `autonomous.md`, `pause-work.md`, `resume-project.md`, `complete-milestone.md` read directly
- Live `HANDOFF.json` example — `/home/blk/projects/web/shopify-error-tracker/.planning/HANDOFF.json` schema verified with all fields
- Claude Code CLI docs — headless flags: `-p`, `--output-format json`, `--allowedTools`, `--continue`, `--resume`, `--bare`
- `.planning/milestones/` directory — v1.0 and v1.1 archive structures verified locally with phase and artifact listings
- `.planning/todos/pending/` — existing todo for pause detection confirmed with full implementation notes

### Secondary (MEDIUM confidence)
- [Ralph TUI](https://peerlist.io/leonardo_zanobi/articles/ralph-tui-ai-agent-orchestration-that-actually-works) — auto-continue queue execution pattern for AI agent orchestration (single project, not yet tested with GSD)
- [LibHunt: ratatui vs textual](https://www.libhunt.com/compare-ratatui-vs-textual) — performance comparison data
- [ratatui-explorer](https://github.com/tatounee/ratatui-explorer) — file explorer widget evaluated and rejected as too heavy

### Tertiary (LOW confidence)
- Completion detection reliability for GSD workflows — untested; derived from general Claude Code CLI behavior, not GSD-specific validation; must be empirically verified in v1.3

---
*Research completed: 2026-03-31*
*Ready for roadmap: yes*
