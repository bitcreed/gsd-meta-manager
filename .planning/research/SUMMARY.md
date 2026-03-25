# Project Research Summary

**Project:** GSD Manager — TUI multi-project orchestration dashboard
**Domain:** Terminal user interface (TUI) for managing AI-assisted development projects (GSD workflow)
**Researched:** 2026-03-24
**Confidence:** HIGH

## Executive Summary

GSD Manager is a developer-focused TUI dashboard that lets users monitor and manage multiple GSD (get-shit-done) projects from a single terminal interface, without switching between project directories or manually inspecting `.planning/` files. The product sits in a well-established category alongside lazygit, k9s, and lazydocker — tools that have defined what "a good TUI" looks like for the target audience. That audience expects keyboard-driven navigation, vim-modal keybindings, status-at-a-glance panels, and drill-down detail views. All of these are table stakes, not differentiators.

The recommended approach is Rust with ratatui 0.30 + crossterm + tokio, using The Elm Architecture (TEA). The TEA pattern — single App struct mutated only through an Action enum via a tokio mpsc EventBus — is the explicitly recommended pattern in ratatui's official documentation and directly prevents the two most dangerous architectural pitfalls: the god App struct and shared-mutable-state race conditions. File-system state from `.planning/` directories is read by a dedicated StateReader module and cached in memory; file-watcher events (via notify-debouncer-full) trigger selective cache invalidation rather than inline reads. This architecture keeps the render loop non-blocking and CPU usage near-zero at idle.

The primary risk category is "invisible until it bites": terminal not restored on panic, file watcher event storms without debounce, and partially-written state files silently corrupting cached state. All three must be addressed at the foundation — before any feature work — and cost less than an hour each to set up correctly. The second risk is scope: embedded terminals, real-time Claude session streaming, and plugin systems are tempting but explicitly out of scope for v1; the project's own PROJECT.md defers these. Ship the read-only dashboard first, validate that users find value, then add write actions.

---

## Key Findings

### Recommended Stack

Rust with ratatui is the correct choice, decided by the distribution constraint. GSD Manager targets arbitrary users' machines, so a single zero-dependency static binary via `cargo install` or release artifact is the only viable install story. Python + Textual is a legitimate alternative for internal tools in controlled environments but requires managing a Python runtime and virtualenv — unacceptable here.

The supporting library selection is straightforward: tokio for async (three concurrent producers: keyboard, file watcher, tick timer), crossterm for cross-platform terminal backend, notify-debouncer-full for debounced file watching, and serde/toml/serde_json for configuration and state file parsing. All library versions have been verified on crates.io as of 2026-03-24.

**Core technologies:**
- **Rust 1.85+**: Language — single-binary distribution, zero runtime dependencies, MSRV matches notify 8.x
- **ratatui 0.30**: TUI rendering — de facto standard (11.9M+ downloads), active maintenance, modular architecture
- **crossterm 0.29**: Terminal backend — cross-platform (macOS/Linux/Windows), event-stream support for async
- **tokio 1.50**: Async runtime — required for concurrent file watching + keyboard events without blocking the render loop
- **notify-debouncer-full 8.x**: Filesystem watching — debounced inotify/kqueue; do NOT use raw notify or the 9.x rc
- **serde + serde_json + toml 1.x**: Serialization — config.json, STATE.md JSON sections, registry.toml

See `.planning/research/STACK.md` for full version table, dev tooling, and compatibility matrix.

### Expected Features

The feature set splits cleanly into three tiers. P1 (launch) features are what users assume exist based on comparable TUIs (lazygit, k9s, lazydocker). P2 (v1.x) features are GSD-specific differentiators that provide the real reason to choose this tool over grepping through project directories. P3 features involve write-back to GSD state, which requires coordination with the GSD queue format and should not be designed in isolation.

**Must have (table stakes):**
- Project list panel with name, status, current phase, last-updated — the core view
- Project registration (add by path, validate .planning/ exists, persist to config)
- File-based state polling via notify-rs — makes the dashboard live rather than a static snapshot
- Keyboard navigation: j/k, Enter, Esc, q, vim-modal — non-negotiable for the target audience
- Help overlay (? key) — immediate trust signal; users check this first
- Detail view: project path, roadmap phases, current phase, task completion count
- Multi-project status bar: "N projects: X active, Y blocked, Z idle"
- Search/filter on project list (/ inline filter)
- Responsive terminal layout with minimum-size protection

**Should have (differentiators for v1.x, post-validation):**
- Roadmap ASCII visualization — parse ROADMAP.md and render phase boxes with progress markers
- Phase progress indicators — count checked vs. unchecked tasks in current phase PLAN.md
- Backlog item count per project — count 999.* dirs in .planning/
- Roadmap diff / last-change summary — "Phase 3 completed 2h ago" via mtime comparison
- Create new GSD project from TUI — shell out to git init + GSD setup

**Defer (v2+):**
- Enqueue next action (write to GSD queue) — GSD queue format must be defined first
- Claude session hooking / embedded terminal — platform-specific, high complexity, explicitly deferred in PROJECT.md
- Plugin system — design plugin boundaries only after 3+ real usage patterns emerge
- Remote project management — local-only is correct for v1

See `.planning/research/FEATURES.md` for full prioritization matrix and feature dependency graph.

### Architecture Approach

The architecture follows a strict layered TEA pattern: Infrastructure (crossterm, notify-rs, TOML config) feeds into an EventBus (tokio mpsc), which feeds into App::update() (the single state-mutation point), which feeds into stateless Component renderers. The critical design decision is that StateReader is the only module that knows the `.planning/` schema — all components receive typed `ProjectState` structs, never raw file paths or unparsed content. This boundary makes StateReader independently testable and prevents schema knowledge from leaking across the codebase.

**Major components:**
1. **StateReader** — pure file-in/struct-out; parses STATE.md, ROADMAP.md, config.json into ProjectState; no TUI dependency; build and test first
2. **App + Action enum** — root state struct; App::update() is the sole state-mutation function; actions cover Navigate, SelectProject, RefreshProject, EnqueuePhase, Quit
3. **EventBus (tokio mpsc)** — merges keyboard events + notify-rs file events + tick intervals into one channel; single consumer on the main loop
4. **FileWatcher (watcher.rs)** — notify-debouncer-full watching each registered .planning/ dir; debounce 200ms; sends Action::RefreshProject to EventBus
5. **RegistryStore (config.rs)** — persists registered project paths to ~/.config/gsd-manager/registry.toml; atomic writes via temp-file rename
6. **Components (dashboard, project_detail, workflow_queue, new_project)** — render from immutable &App; emit Option<Action>; no side effects
7. **TUI module (tui.rs)** — terminal lifecycle: raw mode, alternate screen, panic hook, graceful exit

See `.planning/research/ARCHITECTURE.md` for full data flow diagrams and build order table.

### Critical Pitfalls

1. **Terminal not restored on panic** — Use ratatui::init() / ratatui::restore() (available since 0.28.1); these install panic hooks automatically. Set up before writing any application logic. Cost: 5 minutes. Skipping it costs corrupt terminals throughout all of development.

2. **Blocking the event loop with file I/O** — Never call std::fs::read or any file I/O inside a render function or event handler. All file reads run on a tokio task; results flow back via mpsc channel. StateReader is only invoked on Action::RefreshProject, not on every tick.

3. **File watcher event storms** — Use notify-debouncer-full (or notify-debouncer-mini), not the raw notify watcher. GSD writes multiple files per save; without a 100–500ms debounce window, one save triggers 5–20 events and spikes CPU. One line to set up. Never skip.

4. **Spinning render loop** — Do not render at 60 FPS unconditionally. Drive terminal.draw() only when state has changed (Action received) or on a low-frequency background tick (200–500ms). Confirmed by ratatui GitHub Issue #1338: unconditional draw at 60 FPS causes 10–40% idle CPU.

5. **God App struct** — Adopt TEA from day one. Separate domain state (project registry, parsed file state) from UI state (selected row, scroll offset, modal flags). App::update() delegates to handler methods; a monolithic 500-line match arm is untestable and grows to be impossible to split later.

6. **Registry corruption on write** — Always write registry.toml via atomic rename (write to .tmp, then fs::rename). Include a schema version field from day one. Validate on load with actionable error messages.

7. **Partially-written state files** — GSD writes are not atomic. Treat parse failures as transient: keep last-good state in cache, log the failure, retry after 1–2 seconds. Never surface a parse error as "project broken."

See `.planning/research/PITFALLS.md` for the full pitfall list, phase mapping, and "looks done but isn't" checklist.

---

## Implications for Roadmap

Based on the combined research, the architecture's build-order dependencies and pitfall phase assignments converge on a clear 5-phase structure.

### Phase 1: Foundation and Event Loop

**Rationale:** Three critical pitfalls (terminal restore, blocking event loop, spinning render loop, god App struct) must be addressed before any feature work. These are architectural decisions that cannot be retrofitted cheaply. The TEA skeleton — App struct, Action enum, EventBus, tui.rs lifecycle — establishes the pattern that all subsequent phases follow.

**Delivers:** Working TUI shell with no visible UI, but with correct event loop, panic hook, terminal lifecycle, and async architecture proven out. The "wire frame" on which everything else is built.

**Addresses (pitfalls):** Terminal not restored on panic; blocking event loop; spinning render loop; god App struct.

**Stack used:** ratatui 0.30, crossterm 0.29, tokio 1.50, color-eyre 0.6, tracing + tracing-subscriber.

### Phase 2: State Reading and Project Registry

**Rationale:** StateReader and RegistryStore are pure infrastructure with no UI dependency — they can be built and tested independently. Everything visible in the UI reads from ProjectState structs, so these must be solid before any view code is written. This phase also addresses registry corruption and partial-write pitfalls.

**Delivers:** Typed ProjectState structs parsed from .planning/ files; RegistryStore persisting registered paths to ~/.config/gsd-manager/registry.toml with atomic writes and schema versioning; full test coverage for parser edge cases.

**Addresses (pitfalls):** Partially-written state file reads; registry corruption on write; parsing Markdown/JSON per-render.

**Architecture components:** state_reader/ module, config.rs + RegistryStore.

### Phase 3: Dashboard and Core TUI

**Rationale:** With the event loop proven and state reading solid, this phase delivers the first visible product. It wires the read→render pipeline end-to-end: project list panel, status indicators, keyboard navigation, help overlay, search/filter, and status bar. This is the MVP that validates whether the read-model concept is useful.

**Delivers:** Fully functional dashboard with project list, keyboard navigation, search/filter, status bar, and help overlay. Users can register projects and see their status at a glance.

**Addresses (features):** All P1 table-stakes features (project list, registration, keyboard nav, help, status bar, search/filter, responsive layout).

**Avoids:** Unicode width breakage — use Unicode 9.0 single-codepoint symbols only; test in tmux and two terminal emulators.

**Architecture components:** components/dashboard.rs, widgets/status_badge.rs, widgets/scrollable_list.rs.

### Phase 4: Live State Sync and Detail View

**Rationale:** File watching and the project detail view are the features that differentiate this tool from a static config viewer. They depend on StateReader (Phase 2) being solid and on the dashboard rendering correctly (Phase 3). File watcher event storms must be handled with debounce from the start.

**Delivers:** Live dashboard that auto-updates when GSD modifies .planning/ files; project detail view showing roadmap phases, current phase, task completion count; file watcher with 200ms debounce.

**Addresses (features):** File-based state polling (makes dashboard live); detail view / drill-down; phase progress indicators (P2).

**Avoids:** File watcher event storms — use notify-debouncer-full, not raw notify.

**Architecture components:** watcher.rs, components/project_detail.rs, widgets/workflow_graph.rs (ASCII roadmap sketch).

### Phase 5: Roadmap Visualization and Project Creation

**Rationale:** These are the highest-complexity features and the GSD-specific differentiators. Roadmap ASCII rendering requires ROADMAP.md parsing to be solid (built in Phase 2) and is the highest-value v1.x feature. New project creation shells out to git init and GSD tooling — isolate this complexity to its own component so it can be deferred if needed.

**Delivers:** Roadmap ASCII visualization (phase boxes with progress markers); backlog item count and last-change summary; create new GSD project workflow.

**Addresses (features):** GSD roadmap ASCII visualization (P2); backlog item count (P2); roadmap diff / last-change summary (P2); create new GSD project (P2).

**Note:** Enqueue next action (write to GSD queue) is explicitly deferred to v2 — GSD queue format must be defined externally before this can be designed correctly.

**Architecture components:** widgets/workflow_graph.rs (full implementation), components/new_project.rs.

### Phase Ordering Rationale

- **Infrastructure before UI:** StateReader and RegistryStore have no TUI dependency and are the most testable units. Building them first means all UI code builds on a tested foundation.
- **Foundation before features:** Three of the five critical pitfalls are architectural — they cannot be patched onto an existing codebase without significant refactoring. They must come first.
- **Read-model before write-model:** The dashboard delivers value purely as a read-model. Enqueue/write features require coordination with GSD queue format and should not block launch.
- **TEA enforced throughout:** Every phase adds components that conform to the Action→App::update→render pattern. No component owns state; no component does file I/O.

### Research Flags

Phases likely needing deeper research during planning:

- **Phase 5 (Roadmap Visualization):** ROADMAP.md is Markdown with a specific GSD schema. The parser needs to handle all current GSD roadmap formats robustly. Inspect actual GSD-generated ROADMAP.md files before designing the parser to avoid false assumptions.
- **Phase 5 (New Project Creation):** Shell-out to GSD initialization is underspecified. The exact CLI invocation and expected project scaffolding needs to be verified against current GSD tooling before implementation.

Phases with standard patterns (skip research-phase):

- **Phase 1 (Foundation):** ratatui's own documentation covers panic hooks and TEA event loop setup exhaustively. Follow the official async template.
- **Phase 2 (State Reading):** Standard serde + file I/O patterns. No novel research needed.
- **Phase 3 (Dashboard):** lazygit/lazydocker patterns are well-documented. Standard ratatui component architecture.
- **Phase 4 (Live State Sync):** notify-debouncer-full has clear documentation. The architecture is established.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified directly on crates.io and GitHub releases as of 2026-03-24. ratatui 0.30.0 and notify 8.x confirmed stable. |
| Features | HIGH | Cross-referenced against lazygit, k9s, lazydocker, taskwarrior-tui, btop, and GSD file structure docs. Feature priorities grounded in comparable TUI patterns. |
| Architecture | HIGH | Directly from ratatui official documentation (TEA, Component Architecture, Flux, Async Event Stream). All patterns verified against official sources. |
| Pitfalls | HIGH (Rust/ratatui); MEDIUM (Textual-specific) | Rust pitfalls sourced from ratatui official docs, GitHub issues, and inotify man page. Textual pitfalls are MEDIUM confidence but Rust is the recommended stack, so Textual pitfalls are informational only. |

**Overall confidence:** HIGH

### Gaps to Address

- **GSD ROADMAP.md schema:** The exact format of GSD-generated ROADMAP.md files is not fully specified in research. The roadmap parser (Phase 5) must handle all variants in the wild. Inspect 3–5 real GSD projects before designing the parser.
- **GSD queue format for enqueue feature:** The write-back format for enqueueing next actions is explicitly TBD. Do not design the enqueue feature (v2) until GSD defines this contract.
- **Minimum terminal size requirements:** The precise minimum terminal dimensions for the dashboard layout are not pinned. Define as a constant early in Phase 3 and enforce with a "terminal too small" message rather than discovering breakage via user reports.
- **notify 9.x maturity timeline:** notify 9.0 is in rc as of research date. If it reaches stable before Phase 4 is implemented, re-evaluate — but do not use rc software in production paths.

---

## Sources

### Primary (HIGH confidence)
- [ratatui official docs](https://ratatui.rs/) — TEA architecture, component patterns, panic hooks, async event stream tutorial
- [crates.io API](https://crates.io/) — ratatui 0.30.0, crossterm 0.29.0, tokio 1.50.0, notify 8.0.0, serde 1.0.228, anyhow 1.0.102, clap 4.6.0, toml 1.1.0, serde_json 1.0.149, tracing 0.1.44 (all verified)
- [notify-rs GitHub](https://github.com/notify-rs/notify) — cross-platform filesystem watching, MSRV 1.85, debouncer crates
- [ratatui GitHub releases](https://github.com/ratatui/ratatui/releases) — 0.30.0 confirmed as latest stable
- [inotify(7) Linux man page](https://man7.org/linux/man-pages/man7/inotify.7) — watch limits, event queue overflow
- [Ratatui GitHub Issue #1338](https://github.com/ratatui/ratatui/issues/1338) — confirmed 60 FPS unconditional draw causes high idle CPU

### Secondary (MEDIUM confidence)
- [lazygit GitHub](https://github.com/jesseduffield/lazygit) — keybinding patterns, panel layout
- [k9s official site](https://k9scli.io/) — multi-resource dashboard patterns
- [lazydocker GitHub](https://github.com/jesseduffield/lazydocker) — multi-container overview patterns
- [taskwarrior-tui GitHub](https://github.com/kdheepak/taskwarrior-tui) — project/task TUI patterns
- [Textual Workers Guide](https://textual.textualize.io/guide/workers/) — thread-safe UI update patterns (informational; Textual not selected)
- [7 Things Learned Building a Modern TUI Framework](https://www.textualize.io/blog/7-things-ive-learned-building-a-modern-tui-framework/) — unicode width, emoji unpredictability

### Tertiary (LOW confidence)
- [DEV.to: Go vs Rust TUI Deep Dive](https://dev.to/dev-tngsh/go-vs-rust-for-tui-development-a-deep-dive-into-bubbletea-and-ratatui-2b7) — 30–40% memory advantage for Rust (single benchmark article, not verified)
- [LibHunt: ratatui vs textual comparison](https://www.libhunt.com/compare-ratatui-vs-textual) — performance comparison data (single source)

---
*Research completed: 2026-03-24*
*Ready for roadmap: yes*
