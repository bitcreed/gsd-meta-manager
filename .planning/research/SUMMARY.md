# Project Research Summary

**Project:** GSD Meta Manager (gsd-meta-manager) v1.1
**Domain:** Rust TUI multi-project dashboard — power features for Claude session management, queue execution, and GSD workflow visibility
**Researched:** 2026-03-26
**Confidence:** HIGH (stack and architecture verified against live codebase and Claude CLI; session detection MEDIUM due to undocumented internals)

## Executive Summary

GSD Meta Manager v1.1 adds five high-value capabilities to the existing v1.0 TEA-architecture Rust TUI: accurate state reading (fixing three known parser bugs), a backlog browser, a git history viewer, an execution flow graph, and Claude session management with queue execution. The foundation is already solid — Rust 1.85+, ratatui 0.30, crossterm 0.29, tokio 1.50, and notify 8 are validated and versioned. Only two new dependencies are warranted for v1.1: `sysinfo 0.38` for cross-platform process detection and `petgraph 0.8` for the execution flow graph data model. Everything else — git integration, Claude session launching, backlog parsing — is achievable with `tokio::process::Command` and existing dependencies.

The critical risk for v1.1 is architectural: the existing `InputMode` enum has 11 variants and the `App` struct already carries 18 fields. Adding five new features without refactoring first will cause the flat enum to collapse under its own weight past ~15-20 variants. Research strongly recommends refactoring to a screen/component architecture (implementing a `Screen` trait with `handle_key()`, `render()`, and `update()` methods) before any new screens are added. This is the single highest-leverage change for v1.1 and prevents a 2-3 day recovery refactor later.

The second major risk is async I/O discipline. The current `parse_project_state()` does synchronous `std::fs::read_to_string()` on the main thread. Adding git log parsing (100-500ms on large repos), backlog file scanning, and disk-based phase inference on top of synchronous file I/O will make the render loop visibly laggy. All new I/O must go through `tokio::task::spawn_blocking()` with results dispatched as Action variants through the existing event bus — the pattern already exists in the codebase (`CreateProjectResult`) and must be applied universally before new features are layered on.

---

## Key Findings

### Recommended Stack

The existing stack requires only two additions for v1.1. `sysinfo 0.38.3` (38M+ downloads, cross-platform process enumeration) handles Claude session detection without the Unix-only fragility of `/proc` parsing. `petgraph 0.8.2` (pure Rust, zero transitive bloat) provides the graph data model for the execution flow pipeline. All other v1.1 features — git history, queue execution, backlog browsing, session launching — use `tokio::process::Command`, existing `serde_json`, and existing file I/O patterns.

Notably absent from the new dependencies: `git2`/`gix` (rejected — C FFI and 50+ transitive crates for a read-only log query), Claude Agent SDK (rejected — requires bundling a runtime, violates single-binary constraint), `tui-textarea` (deferred — unnecessary for current editing scope), and `ascii-petgraph` (rejected — physics layout wrong for a fixed 4-stage pipeline).

**Core technologies (additions only):**
- `sysinfo 0.38`: Claude process detection — only cross-platform solution; wrap in `spawn_blocking`, not async context
- `petgraph 0.8`: Execution flow graph data model — pure Rust, provides topological sort and traversal; rendering is a separate custom ratatui Widget

**Patterns for v1.1:**
- Claude session detection: periodic scan every 5-10s via `tokio::time::interval` + `spawn_blocking`; supplement with `notify` watching `~/.claude/history.jsonl`
- Queue execution: `tokio::process::Command` with `kill_on_drop(true)`, stream `--output-format stream-json` via async `BufReader`
- Git log: `Command::output()` with null-byte `--format=%x00%H%x00%an%x00%aI%x00%s` for unambiguous parsing
- Execution flow rendering: custom `Widget` impl (same pattern as existing `RoadmapWidget`), fixed 4-column horizontal layout

See `.planning/research/STACK.md` for full version table, CLI flag reference, and compatibility matrix.

### Expected Features

**Must have (table stakes):**
- Fix plan counting regex (999.4) — dashboard shows wrong progress; users notice immediately and lose trust
- Fix P5:Unknown on completed milestones (999.5) — visible bug for every finished milestone
- Disk-based phase completion inference (999.6) — replicates GSD's own `disk_status` algorithm from `roadmap.cjs`; ROADMAP.md checkboxes are often stale
- GSD integration hooks (999.7) — optional enrichment via `gsd-tools.cjs` JSON output; must degrade gracefully when GSD tools are absent
- Backlog browser (999.11) — dashboard already shows backlog count; users expect drill-in

**Should have (differentiators):**
- Claude session management (999.10) — see active sessions per project, resume/launch from TUI; the killer feature justifying the meta-manager
- Queue execution (999.9) — turns QUEUE.md from passive reminder into actionable launcher
- Execution flow graph (999.1) — per-phase pipeline visualization (discuss/research/plan/execute/verify)
- Git history viewer (999.8) — scrollable git log scoped to repo or `.planning/`

**Defer to v1.2:**
- Queue editor and reorder (999.3) — nice-to-have, not blocking v1.1
- Backlog promotion (phase creation) — needs deeper GSD integration; complex phase numbering
- Milestone plan editor with Claude launch (999.2) — blocked on session management maturity
- Plugin system / extensibility — architecture still stabilizing; premature abstraction

**Explicit anti-features:** No embedded Claude terminal inside TUI (terminal-in-terminal UX disaster), no real-time streaming of Claude output in TUI, no git commit/push from TUI (scope creep into gitui/lazygit territory), no auto-executing queue items without confirmation.

See `.planning/research/FEATURES.md` for full feature dependency graph and MVP phase recommendation.

### Architecture Approach

The v1.0 codebase is a clean TEA implementation with a monolithic `App` struct, a flat `Action` enum (5 variants), and stateless render functions dispatched by `InputMode`. v1.1 extends this without breaking it, adding ~13 new Action variants (reaching ~18 total — within comfortable single-match range if `update()` delegates to handler methods) and 4 new `InputMode` variants for full-screen navigations. Eight new source files are needed across `state_reader/` and `ui/`. Ten existing files require modification; seven remain untouched.

The critical architectural recommendation: refactor `App::update()` to delegate to feature-specific handlers (`handle_git()`, `handle_backlog()`, `handle_queue()`, `handle_claude()`) before adding new variants. Keep feature-specific state in dedicated structs in HashMaps on `App` rather than flat fields — `session_states: HashMap<String, Vec<ClaudeSession>>`, `git_history_states: HashMap<String, GitHistoryState>`, etc.

**Major components (new):**
1. `ClaudeSessionReader` (`src/state_reader/claude_sessions.rs`) — scans `~/.claude/projects/<encoded-path>/` for session JSONL files; detects active sessions via sysinfo process scan
2. `PhaseFlowReader` + `FlowGraphWidget` (`src/state_reader/phase_flow.rs`, `src/ui/flow_graph.rs`) — disk artifact scanning for pipeline stage detection; custom ratatui Widget for 4-column visualization
3. `GitLogReader` + `GitHistoryView` (`src/state_reader/git_log.rs`, `src/ui/git_history.rs`) — async `git log` invocation with null-byte format; scrollable commit list
4. `BacklogReader` + `BacklogView` (`src/state_reader/backlog.rs`, `src/ui/backlog_view.rs`) — parses `999.*` directories; scrollable list with preview panel
5. `QueueExecutor` (`src/queue_executor.rs`) — spawns `claude -p` with queue item as prompt; shares terminal-spawning infrastructure with session management
6. `claude_detector.rs` — cross-platform process detection via sysinfo; abstracted behind `trait SessionDetector` for future-proofing

**Data flow additions:**
- All new I/O goes through `spawn_blocking` or `tokio::process::Command` — results dispatched as Action variants through existing event bus
- Claude session scanning: on explicit navigation to Sessions view only (JSONL files can be megabytes); 30s cache TTL
- Git log: cached per project; invalidated only on `.git/` directory changes, not on every `.planning/` event
- Terminal handoff for interactive Claude launch: RAII guard calling `ratatui::restore()` on creation, `ratatui::init()` on Drop; new tmux pane if tmux is detected

See `.planning/research/ARCHITECTURE.md` for full component map, data flow diagrams, and build order table.

### Critical Pitfalls

1. **InputMode enum explosion** — 11 variants now; 5 new features could push past 20, making `handle_key` a 1000-line unmaintainable match. Avoid: refactor to `Screen` trait with stack-based navigation BEFORE adding new features. Phase 1 priority.

2. **Terminal state corruption on Claude spawn** — `ratatui::restore()` / `ratatui::init()` must be wrapped in a RAII guard covering all error paths. Avoid: build `struct TerminalHandoff` once, use it for both queue execution and session launching. Prefer `claude -p --output-format json` (non-interactive) for queue execution to minimize terminal handoff frequency.

3. **Synchronous file I/O blocking the render loop** — current `parse_project_state()` is sync on the main thread. git log (100-500ms), backlog scanning, and disk inference compound this. Avoid: move all I/O to `spawn_blocking` before building any new features on top. The `CreateProjectResult` pattern already exists — replicate it universally.

4. **Zombie/orphan processes from queue execution** — Claude sessions are Node.js processes (~200MB RSS each). TUI exit must offer "Detach or Kill" dialog; PIDs persisted to `~/.local/share/gsd-manager/sessions.json`; cap concurrent executions with `tokio::sync::Semaphore` (max 2-3).

5. **Claude session detection is inherently fragile** — Claude Code is a Node.js app; process name may be `node`, not `claude`. Session JSONL structure is undocumented and may change. Avoid: accept best-effort detection; use filesystem modification time (last 60s) as primary signal; process scan as secondary; abstract behind `trait SessionDetector`; show "possibly active" not "definitely running."

6. **File watcher self-triggering** — TUI writes to QUEUE.md trigger `FileChanged` events for the same file. Avoid: record a per-project self-write timestamp; skip re-parse for `FileChanged` events within 500ms of own writes.

7. **Disk inference ambiguity** — `disk_status` algorithm produces false positives/negatives (in-progress execution looks like "partial", archived phases create ghost signals). Avoid: STATE.md is primary source of truth; disk inference is fallback with explicit confidence levels shown in UI (fact vs. inference distinction).

See `.planning/research/PITFALLS.md` for the full pitfall list, phase mapping, security checklist, and "looks done but isn't" verification checklist.

---

## Implications for Roadmap

Feature dependencies establish a clear build order with two parallel tracks converging for the high-complexity features.

### Phase 1: Architecture Refactor + State Reader Fixes

**Rationale:** Every subsequent feature depends on accurate state reading and a scalable input handling architecture. Attempting new screens with the flat `InputMode` enum will cause technical debt that compounds immediately. The three parser bugs (999.4, 999.5, 999.6) undermine user trust in the entire dashboard — fix them first so all subsequent features render correct data. The async I/O pattern (`spawn_blocking` universally applied) must be established here before git history and backlog features are built on top.

**Delivers:** `InputMode` refactored to screen/component architecture; all existing functionality preserved; plan counting fixed; P5:Unknown bug resolved; disk-based phase inference implemented with confidence levels; async I/O pattern established.

**Addresses:** 999.4, 999.5, 999.6 (table stakes bugs)

**Avoids:** InputMode enum explosion (Pitfall 1), sync I/O blocking render loop (Pitfall 3), disk inference ambiguity (Pitfall 10), file watcher self-triggering (Pitfall 8)

**Research flag:** Standard patterns — well-documented ratatui component architecture; existing codebase analysis is the primary input. Skip research-phase.

### Phase 2: Read-Only Views (Backlog Browser + Git History Viewer)

**Rationale:** Both are standalone features with no write operations and no process management. They validate the new screen architecture pattern and async I/O pattern under real conditions before the higher-risk execution features are added. Backlog browser exercises the phase directory scanning pattern also needed by the execution flow graph.

**Delivers:** Backlog browser with scrollable list and preview (999.11); git history viewer scoped to repo or `.planning/` (999.8); lazy-loading patterns established; null-byte git log parser verified against edge cases.

**Addresses:** 999.11, 999.8

**Avoids:** Git log parsing edge cases (Pitfall 9 — use null-byte separators from day one), backlog markdown round-trip corruption (Pitfall 5 — read-only in this phase), DAG illegibility at small terminal sizes (Pitfall 7 — git history uses simple list, not graph)

**Research flag:** Standard patterns — git log subprocess invocation and ratatui scrollable list are well-documented. Skip research-phase.

### Phase 3: Execution Flow Graph + GSD Integration Hooks

**Rationale:** Execution flow graph depends on the disk-based inference from Phase 1 and the phase directory reading validated in Phase 2. GSD integration hooks (999.7) layer verified data on top of the now-accurate disk inference. Delivering these together completes the "visibility" half of v1.1 before tackling the "execution" half.

**Delivers:** Per-phase pipeline visualization (discuss/research/plan/execute/verify) as custom ratatui Widget with fixed 4-column layout (999.1); GSD integration hooks with fact/inference distinction and `[verified]` vs `[inferred]` badges (999.7); petgraph added as dependency.

**Addresses:** 999.1, 999.7

**Avoids:** DAG rendering failures (Pitfall 7 — fixed 4-column pipeline, not general DAG; graceful fallback below 100 columns), disk inference ambiguity (Pitfall 10 — confidence levels surfaced in UI).

**Research flag:** Custom widget rendering is well-documented in ratatui; the DAG-avoidance decision (fixed pipeline) eliminates the hard layout problem entirely. Skip research-phase.

### Phase 4: Queue Execution + Terminal Handoff Infrastructure

**Rationale:** Queue execution requires the terminal handoff abstraction (RAII guard) which is shared with Claude session management in Phase 5. Build and validate this infrastructure in Phase 4 where it's used for the simpler headless `claude -p` pattern before Phase 5 adds the complexity of interactive session launching.

**Delivers:** Queue items become executable from TUI with confirmation dialog (999.9); terminal handoff RAII guard built and tested; `ProcessManager` with PID persistence; concurrent execution limit (Semaphore); `QueueItemStatus` tracking (Pending/Running/Done/Failed); `QueueExecutor` module.

**Addresses:** 999.9

**Avoids:** Terminal state corruption (Pitfall 2 — RAII guard built here), zombie/orphan processes (Pitfall 4 — ProcessManager with PID persistence).

**Research flag:** Terminal spawning patterns and process lifecycle management warrant a focused research phase — specifically: tmux detection/pane creation API, platform differences in process group management, and Claude CLI headless mode edge cases. Recommend research-phase before planning.

### Phase 5: Claude Session Management

**Rationale:** Highest complexity and highest value feature. Depends on terminal handoff infrastructure from Phase 4. Claude session detection is inherently best-effort due to undocumented internals — implement with explicit abstraction layer (`trait SessionDetector`) and graceful degradation from the start.

**Delivers:** Session detection per project (filesystem modification time as primary signal, sysinfo process scan as secondary); session list view in detail sub-view; active session indicator on dashboard; launch/resume via tmux pane or detached terminal; sysinfo added as dependency (999.10).

**Addresses:** 999.10

**Avoids:** Claude detection fragility (Pitfall 6 — trait abstraction, best-effort UX), scanning JSONL on every tick (architecture anti-pattern — scan only on view navigation, 30s TTL), terminal corruption (Pitfall 2 — reuses Phase 4 RAII guard).

**Research flag:** Claude session detection warrants a focused research phase — specifically: sysinfo process enumeration for Node.js processes (Claude CLI is a Node.js app), cross-platform CWD extraction from process args, and Claude CLI version detection for JSONL format forward compatibility. Recommend research-phase before planning.

### Phase Ordering Rationale

- **Architecture before features:** The InputMode refactor cannot be done cheaply after new screens are added. It must come first.
- **State accuracy before visibility:** The execution flow graph and GSD integration hooks build directly on disk-based inference — wrong inference produces wrong graphs. Fix first.
- **Read-only before executable:** Read-only views (Phases 2-3) validate shared infrastructure under low risk before the high-complexity process management of Phases 4-5.
- **Terminal handoff before interactive sessions:** Queue execution (`claude -p`, non-interactive) is simpler than full session management. Build and validate the RAII guard in Phase 4 where a mistake is recoverable, not in Phase 5 where the interaction is more complex.
- **Two parallel delivery tracks:** The "visibility" track (Phases 1-3) delivers user-visible value — accurate state, backlog browsing, git history, flow graph — independent of the "execution" track (Phases 4-5). If execution features slip, v1.1 still ships meaningful improvements.

### Research Flags

Phases likely needing `/gsd:research-phase` during planning:
- **Phase 4 (Queue Execution):** tmux API for pane creation, platform differences in process group management (detach vs. orphan semantics), Claude CLI `--bare` headless mode edge cases
- **Phase 5 (Claude Session Management):** sysinfo Node.js process enumeration and CWD extraction cross-platform, Claude CLI version detection for session JSONL format forward compatibility, `.lock` file liveness check across platforms

Phases with standard patterns (skip research-phase):
- **Phase 1 (Architecture Refactor):** ratatui component architecture is well-documented; codebase is already known
- **Phase 2 (Read-Only Views):** git log subprocess and ratatui scrollable list are standard; null-byte format parsing is straightforward
- **Phase 3 (Execution Flow Graph):** fixed 4-column widget rendering follows existing `RoadmapWidget` pattern; petgraph is well-documented

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All existing deps verified on crates.io; two new deps (sysinfo 0.38.3, petgraph 0.8.2) verified stable; rejections (git2, ascii-petgraph) well-justified |
| Features | HIGH | GSD source code read directly (`roadmap.cjs`, `init.cjs`, `gsd-tools.cjs`); existing parsers read; Claude CLI flags verified locally; session JSONL verified on disk |
| Architecture | HIGH | Codebase analyzed directly (all `src/` files); session JSONL structure verified on live installation; Claude process detection verified via `ps aux`; integration anti-patterns identified from real code |
| Pitfalls | HIGH | Based on actual codebase state (11 InputMode variants, sync file I/O confirmed, existing spawn_blocking pattern confirmed); ratatui official docs for terminal handoff recipe |

**Overall confidence:** HIGH

### Gaps to Address

- **Claude session JSONL format stability:** The `~/.claude/projects/<encoded-path>/` structure and `.jsonl` field names are undocumented. Verified on Claude CLI v2.1.84 but may change. Mitigation: build `ClaudeSessionReader` with defensive parsing (skip unknown fields, handle missing fields) and abstract behind `trait SessionDetector`.
- **Process name for Claude CLI:** Research notes the process may show as `node` rather than `claude` in process listings since Claude Code is a Node.js application. `sysinfo.processes_by_name("claude")` may miss it. Phase 5 planning must verify the actual process tree structure and adjust detection strategy.
- **tmux pane creation API:** The recommended terminal launch path (new tmux pane) was not verified against all tmux versions. Phase 4 planning should verify the tmux command syntax and add fallback to `$TERMINAL -e` for non-tmux environments.
- **Archived phase paths:** Completed milestone phases are moved to `.planning/milestones/vX.X-phases/`. The disk inference algorithm must scan both `.planning/phases/` and the milestone archive path. Exact path convention verified on this project's own structure; should be confirmed against GSD source for the canonical pattern.

---

## Sources

### Primary (HIGH confidence)
- crates.io API — sysinfo 0.38.3, petgraph 0.8.2, git2 0.20.4 (evaluated and rejected) version verification
- [sysinfo docs.rs](https://docs.rs/sysinfo/latest/sysinfo/) — process enumeration API
- [ratatui 0.30.0 GitHub](https://github.com/ratatui/ratatui/releases) — latest stable confirmed
- [ratatui custom widget docs](https://ratatui.rs/recipes/widgets/custom/) — Widget trait implementation
- [ratatui component architecture docs](https://ratatui.rs/concepts/application-patterns/component-architecture/) — screen/component pattern
- [ratatui: Spawn External Editor recipe](https://ratatui.rs/recipes/apps/spawn-vim/) — terminal handoff RAII pattern
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — `--resume`, `--continue`, `-p`, `--output-format`, `--bare` flags
- [Claude Code headless docs](https://code.claude.com/docs/en/headless) — programmatic usage patterns
- [tokio::process::Command docs](https://docs.rs/tokio/latest/tokio/process/struct.Command.html) — async process spawning, `kill_on_drop` semantics
- GSD `roadmap.cjs` lines 127-177 — disk_status algorithm verified from source
- GSD `init.cjs` lines 956-985 — phase lifecycle (discuss/research/plan/execute/verify) verified from source
- GSD `gsd-tools.cjs` lines 1-80 — available CLI commands for integration verified from source
- Codebase: `src/app.rs`, `src/action.rs`, `src/state_reader/`, `src/ui/` — all source files read directly
- Local filesystem: `~/.claude/projects/-home-blk-projects-rust-gsd-manager/*.jsonl` — session JSONL structure verified
- Local filesystem: `~/.claude/ide/36473.lock` — lock file structure `{pid, workspaceFolders, ideName}` verified

### Secondary (MEDIUM confidence)
- [Claude Code session file format](https://databunny.medium.com/inside-claude-code-the-session-file-format-and-how-to-inspect-it-b9998e66d56b) — JSONL structure details
- [Claude Code session management](https://kentgigger.com/posts/claude-code-conversation-history) — directory structure and history.jsonl
- [claude-sessions-monitor](https://github.com/yepzdk/claude-sessions-monitor) — approach to scanning `~/.claude/projects/`
- [Session Files & Format - DeepWiki](https://deepwiki.com/affaan-m/everything-claude-code/7.1-session-files-and-format) — sessions-index.json schema reference
- [gitui](https://github.com/gitui-org/gitui) — UX patterns for commit list and keybindings
- [lazygit](https://github.com/jesseduffield/lazygit) — always-visible-panes, scope toggling UX patterns
- [ascii-dag crate](https://crates.io/crates/ascii-dag) — evaluated for DAG layout; rejected for fixed pipeline

### Tertiary (LOW confidence)
- [DEV.to: Go vs Rust TUI Deep Dive](https://dev.to/dev-tngsh/go-vs-rust-for-tui-development-a-deep-dive-into-bubbletea-and-ratatui-2b7) — 30-40% memory advantage for Rust (single benchmark article, not verified)

---
*Research completed: 2026-03-26*
*Ready for roadmap: yes*
