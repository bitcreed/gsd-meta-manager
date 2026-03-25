# Architecture Research

**Domain:** TUI multi-project management dashboard (file-based state, interactive workflows)
**Researched:** 2026-03-24
**Confidence:** HIGH (ratatui patterns from official docs + verified async patterns)

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Presentation Layer                        │
├───────────────┬──────────────────┬──────────────────────────────┤
│  DashboardView│  ProjectDetailView│  WorkflowView (enqueue/new) │
│  (list all)   │  (single project) │  (phase/task queue)         │
└───────┬───────┴────────┬─────────┴──────────────┬───────────────┘
        │                │                         │
        └────────────────┼─────────────────────────┘
                         │  (render from)
┌────────────────────────▼────────────────────────────────────────┐
│                       Application Layer                          │
├──────────────────────────────────────────────────────────────────┤
│  App (root state)   │  Action enum   │  Router (active view)    │
│  - ProjectRegistry  │  - Navigate    │  - tracks focus/screen   │
│  - SelectedProject  │  - LoadProject │  - dispatches focus keys  │
│  - UIMode           │  - EnqueueWork │                           │
└────────────────────────┬────────────────────────────────────────┘
                         │
        ┌────────────────┼──────────────────────┐
        │                │                      │
┌───────▼──────┐ ┌───────▼──────┐  ┌────────────▼────────┐
│  EventBus    │ │  StateReader  │  │  RegistryStore      │
│  (tokio mpsc)│ │  (file I/O)  │  │  (~/.config TOML)   │
└───────┬──────┘ └───────┬──────┘  └────────────┬────────┘
        │                │                       │
┌───────▼────────────────▼───────────────────────▼───────────────┐
│                       Infrastructure Layer                       │
├──────────────────────────────────────────────────────────────────┤
│  TUI (crossterm)  │  FileWatcher (notify-rs)  │  Config (TOML)  │
│  - raw mode       │  - inotify/kqueue/poll    │  - serde        │
│  - event stream   │  - per-project .planning/ │  - dirs crate   │
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| `App` (root state) | Owns all application state: registry, selected project, UI mode, scroll positions | Receives `Action`s from EventBus; passes slices to Views for render |
| `Action` enum | Enumerates every meaningful state transition (Navigate, SelectProject, EnqueuePhase, RefreshState, Quit) | EventBus sends Actions; App.update() consumes them |
| `Router` / `UIMode` | Tracks which screen is active (Dashboard, ProjectDetail, WorkflowQueue, NewProject) and which pane has keyboard focus | App reads it on each frame to decide which View to render |
| `DashboardView` | Renders scrollable list of all registered projects with status summary per project | Reads from App.registry; emits SelectProject action on enter |
| `ProjectDetailView` | Renders ASCII workflow graph, phase list, current milestone for one project | Reads from App.selected_project state |
| `WorkflowView` | Renders enqueue UI — choose project, choose next phase/task | Reads registry; emits EnqueueWork action |
| `EventBus` | Tokio mpsc channel that merges keyboard events + file-watcher events + tick intervals into a single stream | tui.rs feeds keyboard; FileWatcher feeds changes; tick drives refresh |
| `StateReader` | Parses `.planning/` files (STATE.md, ROADMAP.md, config.json) into typed `ProjectState` structs — no GSD process invocation | Called by FileWatcher handler and on-demand refresh |
| `FileWatcher` | Uses `notify-rs` to watch each registered `.planning/` directory for changes and send a `RefreshProject(path)` action | Sends to EventBus; reads paths from RegistryStore |
| `RegistryStore` | Persists the list of registered project paths to `~/.config/gsd-manager/registry.toml` | Read on startup; written on add/remove |
| `TUI` module | Terminal lifecycle: enter raw mode, alternate screen, crossterm event stream, graceful exit on panic | Called by main; provides the draw surface |

## Recommended Project Structure

```
src/
├── main.rs                   # entry point — wires TUI, App, event loop
├── tui.rs                    # terminal lifecycle (raw mode, alt screen, crossterm)
├── app.rs                    # App struct (root state), App::update(action), App::new()
├── action.rs                 # Action enum — every possible state transition
├── router.rs                 # UIMode enum, focus state, screen transitions
├── config.rs                 # config dir resolution, RegistryStore read/write
├── state_reader/
│   ├── mod.rs                # ProjectState struct, parse() entry point
│   ├── planning_dir.rs       # locate and validate .planning/ folder
│   ├── state_md.rs           # parse STATE.md → CurrentState
│   ├── roadmap_md.rs         # parse ROADMAP.md → RoadmapState
│   └── config_json.rs        # parse .planning/config.json → ProjectConfig
├── watcher.rs                # notify-rs setup, maps fs events to Action::RefreshProject
├── components/
│   ├── mod.rs                # Component trait definition
│   ├── dashboard.rs          # DashboardView — project list with status badges
│   ├── project_detail.rs     # ProjectDetailView — workflow graph + phase list
│   ├── workflow_queue.rs     # WorkflowView — enqueue next phase/task
│   └── new_project.rs        # NewProjectView — form to spin up a GSD project
└── widgets/
    ├── status_badge.rs       # colored phase/status indicator
    ├── workflow_graph.rs     # ASCII roadmap renderer (box-drawing chars)
    └── scrollable_list.rs    # list with selection highlight, scrolling
```

### Structure Rationale

- **`state_reader/`** is a separate module because it is the most testable unit — pure file-in / struct-out, no TUI dependency. Test it independently before touching the UI.
- **`components/`** owns all view logic. The ratatui component template explicitly says "you shouldn't need to change anything outside components/" once the framework is wired. Keep business views here.
- **`widgets/`** holds primitive re-usable rendering building blocks (badge, graph, list) that multiple components share. Separating them avoids duplication across DashboardView and ProjectDetailView.
- **`watcher.rs`** is isolated so it can be replaced with a simple polling fallback without touching any UI code — important because notify-rs inotify has known edge cases on network filesystems.
- **`router.rs`** is explicit rather than embedded in `app.rs` because screen routing and focus logic grows quickly; keeping it separate prevents app.rs from becoming a god module.

## Architectural Patterns

### Pattern 1: Elm Architecture (TEA) with Async Actions

**What:** Model (App state struct) → Update (App::update consumes Actions) → View (components render from App) cycle. Actions flow through a tokio mpsc channel so background tasks (file watcher, tick timer) post to the same bus as keyboard input.

**When to use:** This project. Single-user local app, deterministic state, multiple async producers (keyboard + file watcher + timer).

**Trade-offs:** Simple to reason about; all state changes go through one path. Slight verbosity in Action enum as features grow — acceptable cost.

**Example (Rust):**
```rust
// action.rs
pub enum Action {
    Navigate(Screen),
    SelectProject(PathBuf),
    RefreshProject(PathBuf),   // sent by FileWatcher
    EnqueuePhase { project: PathBuf, phase_id: String },
    Tick,
    Quit,
}

// app.rs
impl App {
    pub fn update(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::SelectProject(path) => {
                self.selected = Some(self.read_project(&path));
                Some(Action::Navigate(Screen::ProjectDetail))
            }
            Action::RefreshProject(path) => {
                self.refresh_project_state(&path);
                None
            }
            // ...
        }
    }
}
```

### Pattern 2: Single EventBus via tokio mpsc + select!

**What:** One `tokio::sync::mpsc::UnboundedSender<Action>` is cloned and distributed to every async source (keyboard handler, file watcher, tick interval). The main loop calls `rx.recv().await` and feeds the received Action into `App::update`.

**When to use:** Any time you have 2+ async producers feeding state updates (this project has keyboard, file watcher, and tick timer as three producers).

**Trade-offs:** Simple, no deadlock risk. Unbounded channel can buffer during slow renders — acceptable for local dashboard with no burst load. If tick spam is a concern, use a bounded channel with `try_send` and drop ticks.

**Example (Rust):**
```rust
// main event loop
loop {
    tokio::select! {
        Some(action) = rx.recv() => {
            if let Some(followup) = app.update(action) {
                tx.send(followup).ok();
            }
        }
    }
    terminal.draw(|f| ui::render(f, &app))?;
    if app.should_quit { break; }
}
```

### Pattern 3: Component Trait with Render + Handle

**What:** Each view implements a `Component` trait with `render(&self, frame, area)` and `handle(&mut self, action) -> Option<Action>`. The root render function delegates to the active component based on `app.router.active_screen`.

**When to use:** When you have 4+ distinct screens (Dashboard, Detail, Workflow, NewProject) that each own their own scroll/selection state and key bindings.

**Trade-offs:** Enables "once you set up routing, only add components/" discipline. Slight indirection when tracing key → action across component boundary. Worth it at this scale.

## Data Flow

### User Interaction Flow

```
Key Press (crossterm)
    │
    ▼
EventHandler task (tokio::select!)
    │  converts crossterm::Event → Action
    ▼
EventBus rx (mpsc channel)
    │
    ▼
App::update(action)
    │  mutates App state
    ▼
terminal.draw()
    │  calls active Component::render(&app)
    ▼
Terminal output (crossterm backend)
```

### File-Change Flow

```
.planning/STATE.md written by GSD
    │
    ▼
notify-rs FileWatcher (inotify/kqueue)
    │  debounced: 200ms
    ▼
watcher sends Action::RefreshProject(path) → EventBus tx
    │
    ▼
App::update → StateReader::parse(.planning/)
    │  updates App.registry[project].state
    ▼
Next terminal.draw() picks up new state automatically
```

### Startup Flow

```
main()
    │
    ├── RegistryStore::load() → Vec<RegisteredProject>
    ├── StateReader::parse() for each registered project → ProjectState
    ├── FileWatcher::watch() for each .planning/ dir
    ├── TUI::enter() → raw mode + alt screen
    └── event loop starts
```

### Key Data Flows

1. **Project registration:** User adds path → RegistryStore writes `~/.config/gsd-manager/registry.toml` → FileWatcher starts watching new path → StateReader does initial parse → App.registry updated → Dashboard re-renders.

2. **Background state refresh:** GSD modifies `.planning/STATE.md` → notify-rs fires → debounced 200ms → Action::RefreshProject sent → StateReader re-parses that one project → App state patched → next draw frame reflects changes.

3. **Enqueue action:** User navigates to WorkflowView, selects a phase → Action::EnqueuePhase dispatched → App writes a queue entry file into `.planning/` (exact format TBD during implementation) → non-intrusive: only appends, never modifies active GSD state.

## Build Order Implications

Dependencies between components determine the order in which phases should be implemented:

| Order | Component | Depends On | Rationale |
|-------|-----------|------------|-----------|
| 1 | `state_reader/` | Nothing (pure file I/O) | Everything else reads ProjectState; test this first before any UI |
| 2 | `config.rs` + RegistryStore | state_reader | Need project list before anything can be displayed |
| 3 | `tui.rs` + `action.rs` + `app.rs` | Nothing (skeleton) | Wire the event loop with dummy state so the terminal renders |
| 4 | `components/dashboard.rs` | App state, state_reader | First visible screen; proves the read → render pipeline works end-to-end |
| 5 | `watcher.rs` | state_reader, EventBus | Adds live refresh; not needed for basic display but needed for "always current" |
| 6 | `components/project_detail.rs` + `widgets/workflow_graph.rs` | state_reader | Requires roadmap parsing to be solid |
| 7 | `components/workflow_queue.rs` (enqueue) | app state, config | Requires knowing what phases exist (from roadmap parser) |
| 8 | `components/new_project.rs` (create project) | config, shell invocation | Most complex: needs git init, directory scaffolding, GSD initialization |

## Anti-Patterns

### Anti-Pattern 1: Parsing .planning/ Files Inline in Render

**What people do:** Call `fs::read_to_string(".planning/STATE.md")` inside a component's `render()` function.

**Why it's wrong:** `render()` is called every frame (up to 60Hz). Filesystem reads in the hot render path cause visible stutter and are wasteful. On slow filesystems (network mounts, WSL) this is a hard bug.

**Do this instead:** StateReader runs only on `Action::RefreshProject` — triggered by file watcher or explicit user action. Render reads only from the already-parsed `App.registry` in memory.

### Anti-Pattern 2: Separate Event Loops for Keyboard and File Watcher

**What people do:** Spawn independent threads with their own `std::sync::Mutex<AppState>` that both mutate shared state.

**Why it's wrong:** Mutex contention, subtle race conditions between render and mutation, complex shutdown logic. Becomes a debugging nightmare.

**Do this instead:** Single mpsc channel as EventBus. Both keyboard and file-watcher tasks are senders; the main loop is the sole consumer. All state mutation happens on one thread in `App::update`. Zero shared-state concurrency.

### Anti-Pattern 3: Monolithic App::update() Match

**What people do:** Put all business logic inside one `match action { ... }` in `app.rs` that grows to 500+ lines.

**Why it's wrong:** Untestable, hard to navigate. Adding a feature means touching the same giant match.

**Do this instead:** Delegate to methods — `app.handle_navigation(action)`, `app.handle_project_actions(action)`, `app.handle_registry_actions(action)`. Keep `update()` as a router that calls the appropriate handler.

### Anti-Pattern 4: Tightly Coupling Views to File Paths

**What people do:** Pass raw `PathBuf` values to components and let them resolve `.planning/` structure themselves.

**Why it's wrong:** Every component needs to know the .planning/ schema. Changes to the schema require touching every component.

**Do this instead:** StateReader is the only code that knows about `.planning/` layout. Components receive `ProjectState` structs — typed, already-parsed data. File paths stay in the infrastructure layer.

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| FileWatcher → EventBus | `mpsc::UnboundedSender<Action>` | Watcher holds a sender clone; fires `Action::RefreshProject(PathBuf)` |
| EventBus → App | `mpsc::UnboundedReceiver<Action>` | Main loop awaits; single consumer, no locking |
| App → Components | Immutable `&App` reference passed to `render()` | Components are read-only observers of App state |
| Components → EventBus | Return `Option<Action>` from `handle()` | Components don't mutate state; they emit actions |
| StateReader → App | Returns `ProjectState` value | Pure function: path in, typed struct out |
| RegistryStore → App | Returns `Vec<RegisteredProject>` | Called on startup and after add/remove |

### External Integration

| Integration | Pattern | Notes |
|-------------|---------|-------|
| `.planning/` files | Read-only file I/O via `std::fs` | Never write into existing GSD-managed files; only read |
| `~/.config/gsd-manager/` | Read/write TOML via `serde` + `toml` crates; path resolved with `dirs` crate | Create on first run |
| GSD initialization (new project) | `std::process::Command` to invoke shell/claude | Scoped to `new_project.rs` component only; isolate this complexity |
| notify-rs (file watcher) | `RecommendedWatcher` (inotify on Linux) with fallback to `PollWatcher` | Watch `.planning/` dirs recursively; debounce 200ms to avoid duplicate refreshes on rapid writes |

## Scaling Considerations

This is a local single-user TUI — traditional scaling concerns don't apply. The relevant growth dimension is "number of registered projects."

| Projects | Concern | Approach |
|----------|---------|----------|
| 1-20 | None | Parse all on startup; hold all in memory |
| 20-100 | Startup time if parsing all eagerly | Lazy-parse on first view; cache in memory after first access |
| 100+ | Memory + startup time | Unlikely use case; pagination in DashboardView is sufficient |

**First real bottleneck:** file descriptor limits from notify-rs inotify watches. Each watched `.planning/` directory consumes an inotify watch. Linux default limit is 8192 watches. At 100 projects this is fine; at 1000+ projects, switch from inotify to PollWatcher and consolidate watches.

## Sources

- [The Elm Architecture — ratatui official docs](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
- [Component Architecture — ratatui official docs](https://ratatui.rs/concepts/application-patterns/component-architecture/)
- [Flux Architecture — ratatui official docs](https://ratatui.rs/concepts/application-patterns/flux-architecture/)
- [Async Event Stream tutorial — ratatui official docs](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/)
- [Async Template project structure — ratatui](https://ratatui.github.io/async-template/02-structure.html)
- [Component Template project structure — ratatui](https://ratatui.rs/templates/component/project-structure/)
- [notify-rs cross-platform filesystem notification library](https://github.com/notify-rs/notify)
- [tokio mpsc channels](https://tokio.rs/tokio/tutorial/channels)
- [rat-focus: focus handling for ratatui applications](https://github.com/thscharler/rat-focus)

---
*Architecture research for: TUI multi-project management dashboard*
*Researched: 2026-03-24*
