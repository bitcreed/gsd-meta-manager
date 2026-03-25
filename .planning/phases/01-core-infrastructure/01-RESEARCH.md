# Phase 1: Core Infrastructure - Research

**Researched:** 2026-03-24
**Domain:** Rust TUI foundation — async event loop, state parser, project registry, terminal lifecycle
**Confidence:** HIGH

## Summary

Phase 1 builds the foundational skeleton: a working async TUI event loop with panic-safe terminal management, a state reader that parses GSD `.planning/` files into typed Rust structs, and a project registry that persists named project entries to disk as JSON. The TUI in this phase is a functional stub (basic project list, add/remove via keyboard and CLI subcommands) — visual polish comes in Phase 2.

The stack is locked: Rust 1.85+ with ratatui 0.30, crossterm 0.29, and tokio 1.50. The architecture follows The Elm Architecture (TEA): a single `App` struct mutated only through an `Action` enum dispatched via a tokio mpsc channel. This phase establishes the patterns every subsequent phase builds on — getting the event loop, state management, and file parsing right here prevents expensive refactors later.

**Primary recommendation:** Build in this order: (1) project scaffold with Cargo.toml and panic-safe terminal lifecycle, (2) state reader module with unit tests against real `.planning/` file formats, (3) registry/config persistence, (4) async event loop with mpsc EventBus wiring, (5) stub TUI with basic project list and CLI subcommands for add/remove.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Config format is JSON — consistent with GSD's own config.json, easier cross-tool parsing
- **D-02:** Config location is `~/.config/gsd-manager/` — XDG-compliant, standard for Linux CLI tools
- **D-03:** Projects are stored as named entries — user assigns an alias when registering (e.g., "myapp" -> /home/user/projects/myapp). Alias is displayed in TUI
- **D-04:** Single config file: `~/.config/gsd-manager/config.json` containing both registry and preferences
- **D-05:** Parse essential fields only in Phase 1 — STATE.md (current phase, status), ROADMAP.md (phase list), config.json (mode). Enough for dashboard. Deeper extraction (PLAN.md task counts, REQUIREMENTS.md completion) added in later phases
- **D-06:** Graceful degradation on missing/malformed files — show project with "unknown" status, log warning. Never crash, never auto-remove from registry
- **D-07:** Phase 1 delivers a functional stub — basic project list with add/remove via keyboard. Usable but unstyled. Phase 2 polishes it
- **D-08:** Both CLI and TUI interfaces — `gsd-manager add <alias> /path/to/project` works headless (scriptable). Same operations available inside the TUI
- **D-09:** Hybrid state update design — the state reader uses an event channel (tokio mpsc) that can accept both file-change events and push events. Phase 1 only implements file-based reads, but the channel architecture is ready for hooks
- **D-10:** Design the state update channel now, but no actual hook code in Phase 1. Hook research and implementation happens in Phase 3 alongside the file watcher

### Claude's Discretion
- Exact ProjectState struct field names and types
- Event loop tick rate and render strategy
- CLI argument parser choice (clap vs manual)
- Internal error types and logging setup
- Test strategy (unit tests for parser, integration tests for registry persistence)

### Deferred Ideas (OUT OF SCOPE)
- Hook research and implementation — Phase 3
- PLAN.md task count parsing — later phases (STATE-02 deferred deeper extraction)
- Color-coding and styling — Phase 2
- Search/filter — Phase 2
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REG-01 | User can add a GSD project by path (validates `.planning/` exists before registering) | Registry module with path validation; CLI `add` subcommand; config.json persistence |
| REG-02 | User can remove a tracked project from the manager | Registry module with `remove` method; CLI `remove` subcommand; TUI keybinding |
| REG-03 | Project registry persists across restarts (config file on disk) | JSON config at `~/.config/gsd-manager/config.json` with atomic writes via tempfile+rename |
| STATE-01 | Manager reads project state from `.planning/` files (STATE.md, ROADMAP.md, config.json) without running GSD commands | StateReader module with YAML frontmatter parser for STATE.md, markdown parser for ROADMAP.md, JSON parser for config.json |
| STATE-02 | Manager shows phase progress indicators (completed vs total tasks from PLAN.md files) | Per D-05, Phase 1 extracts progress from STATE.md YAML frontmatter (`progress.completed_plans` / `progress.total_plans`). Deeper PLAN.md extraction deferred |
| STATE-03 | Manager shows backlog item count per project (999.x directories in `.planning/`) | Filesystem scan: count directories matching `999*` pattern in `.planning/phases/` |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- GSD workflow enforcement: do not make direct repo edits outside a GSD workflow unless user explicitly asks to bypass
- Stack confirmed: Rust + ratatui 0.30 + crossterm 0.29 + tokio 1.50
- Architecture: TEA pattern — single App struct, Action enum, mpsc EventBus, stateless components
- State reading: Parse `.planning/` files directly; StateReader is the only module that knows the schema
- File watching: notify-debouncer-full 8.x with 200ms debounce (Phase 3, not Phase 1)

## Standard Stack

### Core (Phase 1 dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI rendering | De facto Rust TUI library; 0.30 confirmed latest stable on crates.io |
| crossterm | 0.29.0 | Terminal backend | Cross-platform; ratatui's recommended backend; event-stream feature for async |
| tokio | 1.50.0 | Async runtime | Required for mpsc EventBus, async event loop, non-blocking I/O |
| serde | 1.0.228 | Serialization | Derive macros for all config/state structs |
| serde_json | 1.0.149 | JSON parsing | Parse GSD `config.json` and gsd-manager's own `config.json` |
| serde_yml | 0.0.12 | YAML parsing | Parse STATE.md YAML frontmatter (serde_yaml is deprecated) |
| clap | 4.6.0 | CLI argument parsing | `add`, `remove`, `list`, and TUI launch subcommands; derive macro for ergonomics |
| anyhow | 1.0.102 | Error handling | Ergonomic `Result<T>` propagation throughout the app |
| color-eyre | 0.6.5 | Panic/error reporting | Installs panic hook; works with ratatui's terminal restore |
| tracing | 0.1.44 | Structured logging | Log to file, not stdout (ratatui owns the terminal) |
| tracing-subscriber | 0.3.23 | Log routing | File appender; env-filter for log levels |
| tracing-appender | 0.2.4 | File appender | Non-blocking file writer for logs |
| dirs | 6.0.0 | Platform directories | Resolve `~/.config/` and `~/.local/share/` paths portably |
| regex | 1.12.3 | Pattern matching | Parse structured fields from STATE.md and ROADMAP.md markdown |
| tempfile | 3.27.0 | Atomic file writes | Write config to tempfile then rename for crash safety |

### Not Needed in Phase 1

| Library | Phase | Reason |
|---------|-------|--------|
| notify / notify-debouncer-full | Phase 3 | File watching not in scope |
| toml | Removed | Config format is JSON (D-01), not TOML |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| serde_yml | yaml-rust2 (0.11.0) | Lower level, no serde integration; serde_yml wraps yaml-rust2 with serde derive support |
| clap derive | manual arg parsing | Clap adds compile time but saves significant boilerplate; worth it for 4+ subcommands |
| color-eyre | custom panic hook | color-eyre integrates with ratatui's restore flow and provides backtraces; don't hand-roll |

**Cargo.toml dependencies (Phase 1):**
```toml
[dependencies]
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
dirs = "6"
regex = "1"
tempfile = "3"

[dev-dependencies]
assert_fs = "1"  # Temporary directories for tests
```

## Architecture Patterns

### Recommended Project Structure (Phase 1)
```
src/
├── main.rs                   # Entry point — CLI dispatch or TUI launch
├── cli.rs                    # Clap derive structs, subcommand dispatch
├── tui.rs                    # Terminal lifecycle (raw mode, alt screen, panic hook)
├── app.rs                    # App struct (root state), App::update(action)
├── action.rs                 # Action enum — every state transition
├── event.rs                  # EventBus: tokio mpsc setup, crossterm event reader task
├── config.rs                 # Config struct, load/save to ~/.config/gsd-manager/config.json
├── registry.rs               # RegisteredProject, add/remove/list, path validation
├── state_reader/
│   ├── mod.rs                # ProjectState struct, top-level parse() function
│   ├── state_md.rs           # Parse STATE.md YAML frontmatter → StateInfo
│   ├── roadmap_md.rs         # Parse ROADMAP.md → phases list with completion status
│   └── config_json.rs        # Parse .planning/config.json → ProjectConfig
├── ui/
│   ├── mod.rs                # Root render dispatch based on app mode
│   └── project_list.rs       # Stub project list view (Phase 1 functional stub)
└── error.rs                  # App-level error types (optional, anyhow may suffice)
```

### Pattern 1: TEA (The Elm Architecture) Event Loop

**What:** Single `App` struct owns all state. `Action` enum represents every possible state change. `App::update(&mut self, action) -> Option<Action>` is the sole mutation path. Views render from `&App` immutably. Background tasks (keyboard reader, tick timer) send Actions through a tokio mpsc channel.

**When to use:** This entire project. All phases build on this pattern.

**Phase 1 event loop skeleton:**
```rust
// main.rs (TUI mode)
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Action>();

// Spawn crossterm event reader
let tx_keys = tx.clone();
tokio::spawn(async move {
    let mut reader = crossterm::event::EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        if let Some(action) = map_event_to_action(event) {
            tx_keys.send(action).ok();
        }
    }
});

// Spawn tick timer (low frequency for Phase 1 stub)
let tx_tick = tx.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    loop {
        interval.tick().await;
        tx_tick.send(Action::Tick).ok();
    }
});

// Main loop
loop {
    terminal.draw(|frame| ui::render(frame, &app))?;

    if let Some(action) = rx.recv().await {
        if let Some(followup) = app.update(action) {
            tx.send(followup).ok();
        }
    }

    if app.should_quit {
        break;
    }
}
```

### Pattern 2: Terminal Lifecycle with Panic Safety

**What:** `ratatui::init()` enters raw mode + alternate screen + installs panic hook. `ratatui::restore()` exits cleanly. color-eyre's panic hook must be installed BEFORE ratatui's init so ratatui's hook runs first (restoring terminal) then color-eyre prints the backtrace.

**Critical ordering:**
```rust
fn main() -> color_eyre::Result<()> {
    // 1. Install color-eyre FIRST
    color_eyre::install()?;

    // 2. Then init ratatui (installs its own panic hook that chains)
    let mut terminal = ratatui::init();

    // 3. Run app
    let result = run_app(&mut terminal);

    // 4. Restore terminal on normal exit
    ratatui::restore();

    result
}
```

### Pattern 3: CLI + TUI Dual Mode

**What:** `gsd-manager` with no subcommand launches the TUI. `gsd-manager add <alias> <path>` runs headlessly. Both paths share the same registry/config module.

```rust
#[derive(Parser)]
#[command(name = "gsd-manager")]
enum Cli {
    /// Launch the TUI dashboard (default)
    Tui,
    /// Add a GSD project to the registry
    Add {
        /// Alias for the project (displayed in TUI)
        alias: String,
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
    },
    /// Remove a project from the registry
    Remove {
        /// Alias of the project to remove
        alias: String,
    },
    /// List all registered projects
    List,
}
```

### Anti-Patterns to Avoid
- **File I/O in render():** Never read `.planning/` files inside the draw closure. Parse on startup and on refresh actions only.
- **God App struct:** Keep UI state (selected index, scroll offset) separate from domain state (project list, parsed state). Use nested structs.
- **Monolithic update():** Delegate `App::update()` to sub-methods by action category. Keep the top-level match as a router.
- **`std::sync::Mutex` in async code:** Use `tokio::sync::Mutex` if needed, but prefer message passing (mpsc) to avoid locking entirely.
- **Direct file writes without atomic rename:** Always write to tempfile first, then `std::fs::rename()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML frontmatter parsing | Custom line-by-line parser for `---` delimited blocks | Extract frontmatter between `---` markers, deserialize with serde_yml | YAML has subtle edge cases (quoted strings, multiline, type coercion) |
| CLI argument parsing | Manual `std::env::args` parsing | clap 4.6 with derive | Subcommands, help text, validation, error messages all handled |
| Config directory resolution | Hardcoded `~/.config/` | `dirs::config_dir()` | Handles XDG_CONFIG_HOME overrides, macOS ~/Library/Preferences, Windows AppData |
| Panic-safe terminal restore | Custom signal handler + Drop impl | `ratatui::init()` + `color_eyre::install()` | ratatui 0.28.1+ installs panic hooks automatically; color-eyre chains correctly |
| Atomic file writes | Manual open/write/close | `tempfile::NamedTempFile` + `persist()` | Handles platform differences; `persist()` calls `rename()` atomically on POSIX |
| Log file rotation/appending | Manual file open with append | `tracing-appender` | Non-blocking writes, rotation support, no manual file management |

**Key insight:** The STATE.md YAML frontmatter looks simple but has real edge cases (quoted ISO timestamps, optional fields like `last_activity` that may be absent in some projects, variable `milestone_name` values). Use serde with `Option<T>` fields and `#[serde(default)]` rather than assuming all fields are present.

## GSD `.planning/` File Format Analysis

Based on examination of 4 real GSD projects, here are the formats the state reader must handle:

### STATE.md Format

YAML frontmatter between `---` delimiters, followed by markdown body.

**Frontmatter fields (observed across projects):**
```yaml
---
gsd_state_version: 1.0          # Always present
milestone: v1.0                  # Always present (string)
milestone_name: milestone        # Always present (string, varies: "milestone", "App Store Submission")
status: planning                 # Always present ("planning", "unknown", other values)
stopped_at: Phase 1 context...  # Always present (free-form string)
last_updated: "2026-03-25..."   # Always present (quoted ISO 8601 timestamp)
last_activity: 2026-03-24 ...   # OPTIONAL - not always present
progress:
  total_phases: 4               # Always present (integer)
  completed_phases: 0           # Always present (integer)
  total_plans: 0                # Always present (integer)
  completed_plans: 0            # Always present (integer)
  percent: 0                    # OPTIONAL - not always present
---
```

**Markdown body contains:**
- `## Current Position` with `Phase: N of M (name)` and optional `— EXECUTING` suffix
- `## Performance Metrics` with velocity data

**Recommended struct:**
```rust
#[derive(Debug, Deserialize, Default)]
pub struct StateFrontmatter {
    pub gsd_state_version: f64,
    pub milestone: String,
    #[serde(default)]
    pub milestone_name: String,
    pub status: String,
    #[serde(default)]
    pub stopped_at: String,
    pub last_updated: String,
    #[serde(default)]
    pub last_activity: Option<String>,
    pub progress: ProgressInfo,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProgressInfo {
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
    #[serde(default)]
    pub percent: u32,
}
```

### ROADMAP.md Format

Markdown with a phase checklist:
```markdown
- [x] **Phase 1: Name** - Description (completed DATE)
- [ ] **Phase 2: Name** - Description
```

**Parsing strategy:** Regex on lines matching `- \[([ x])\] \*\*Phase (\d+): (.+?)\*\*` to extract completion status, phase number, and name.

### .planning/config.json Format

Standard JSON object. Key fields for Phase 1:
```json
{
  "mode": "yolo",
  "granularity": "coarse",
  ...
}
```

**Parsing strategy:** Deserialize with serde_json. Use `serde_json::Value` for unknown fields, extract only `mode` and `granularity` initially. Use `#[serde(flatten)]` with `HashMap<String, Value>` to preserve unknown fields.

### Backlog Directories (STATE-03)

Count directories matching `999*` pattern in `.planning/phases/`. Based on examination of 4 projects, none currently have backlog directories — this is a feature that may not be widely used yet. The parser should handle the common case (zero backlog items) gracefully.

```rust
pub fn count_backlog_items(planning_dir: &Path) -> u32 {
    let phases_dir = planning_dir.join("phases");
    std::fs::read_dir(&phases_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("999"))
                        .unwrap_or(false)
                        && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0)
}
```

## Common Pitfalls

### Pitfall 1: Terminal Not Restored on Panic
**What goes wrong:** Panic while in raw mode leaves terminal unusable (no echo, garbled output, cursor missing).
**Why it happens:** Raw mode and alternate screen must be explicitly restored. Panics bypass normal cleanup.
**How to avoid:** Install `color_eyre::install()` BEFORE `ratatui::init()`. ratatui 0.30's `init()` installs a panic hook that calls `restore()` automatically. The ordering matters: color-eyre first, then ratatui, so ratatui's hook restores the terminal before color-eyre prints the backtrace.
**Warning signs:** Terminal corruption during development when unwrap() panics.

### Pitfall 2: serde_yaml is Deprecated
**What goes wrong:** Using `serde_yaml` pulls in an unmaintained crate with known issues.
**Why it happens:** Many tutorials and examples still reference `serde_yaml`.
**How to avoid:** Use `serde_yml` (0.0.12) which is the maintained replacement. Same API surface, drop-in replacement.
**Warning signs:** Deprecation warning in Cargo output.

### Pitfall 3: YAML Frontmatter Extraction Edge Cases
**What goes wrong:** Naive "split on `---`" parser fails when markdown body contains `---` (horizontal rules) or when frontmatter contains multi-line strings.
**Why it happens:** `---` is valid markdown for horizontal rules. The frontmatter delimiter must be at line start and the first `---` must be line 1.
**How to avoid:** Parse frontmatter as: first line must be exactly `---`, scan for next line that is exactly `---`, extract the content between them. Do not split the entire file.
**Warning signs:** Parser fails on STATE.md files that have horizontal rules in the body.

### Pitfall 4: Blocking Event Loop with Startup File I/O
**What goes wrong:** Parsing all projects' `.planning/` files synchronously at startup delays TUI appearance.
**Why it happens:** Seems fast with 2 projects on SSD; becomes noticeable with 10+ projects or network filesystems.
**How to avoid:** Show the TUI immediately with "loading..." state. Spawn file reads as tokio tasks that send `Action::ProjectLoaded(alias, state)` through the EventBus. Use `tokio::task::spawn_blocking` for filesystem I/O.
**Warning signs:** Visible delay between launch and first render.

### Pitfall 5: Config File Corruption on Crash
**What goes wrong:** Writing config.json directly; crash mid-write leaves truncated/invalid JSON.
**Why it happens:** `std::fs::write` is not atomic — it truncates then writes.
**How to avoid:** Use `tempfile::NamedTempFile` in the same directory as config.json, write to it, then call `.persist(config_path)` which does an atomic `rename()`.
**Warning signs:** Config file is zero bytes or truncated JSON after a kill -9.

### Pitfall 6: Hardcoded Config Path Ignoring XDG
**What goes wrong:** Hardcoding `~/.config/gsd-manager/` breaks when `XDG_CONFIG_HOME` is set to a non-standard location.
**Why it happens:** Developer's own machine uses the default.
**How to avoid:** Use `dirs::config_dir()` which respects `XDG_CONFIG_HOME` on Linux, `~/Library/Application Support` on macOS, `AppData/Roaming` on Windows.
**Warning signs:** Tests that create files in `~/.config/` instead of a temp directory.

### Pitfall 7: Event-Driven Rendering Without Redraw Flag
**What goes wrong:** Drawing every event loop iteration wastes CPU. Or, drawing only on state change misses the initial render.
**Why it happens:** The render-on-every-tick pattern from tutorials wastes CPU. But pure event-driven rendering can miss edge cases.
**How to avoid:** Use a `needs_redraw: bool` flag on App. Set it true on any state change. The main loop checks it: if true, draw and reset flag. Also draw on Tick at a slow interval (250ms) as a safety net. This gives <2% idle CPU.
**Warning signs:** High CPU when idle; or stale display after state changes.

## Code Examples

### YAML Frontmatter Extraction
```rust
/// Extract YAML frontmatter from a GSD STATE.md file.
/// Returns None if the file doesn't start with `---`.
pub fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    // Skip the opening ---
    let after_open = &content[3..];
    let after_open = after_open.trim_start_matches(|c| c == '\r' || c == '\n');

    // Find closing ---
    if let Some(end) = after_open.find("\n---") {
        Some(&after_open[..end])
    } else {
        None
    }
}
```

### Atomic Config Write
```rust
use tempfile::NamedTempFile;
use std::io::Write;

pub fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to temp file in same directory, then atomic rename
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    let json = serde_json::to_string_pretty(config)?;
    tmp.write_all(json.as_bytes())?;
    tmp.persist(path)?;

    Ok(())
}
```

### ROADMAP.md Phase Parser
```rust
use regex::Regex;

#[derive(Debug)]
pub struct RoadmapPhase {
    pub number: String,    // "1", "2.1", etc.
    pub name: String,
    pub description: String,
    pub completed: bool,
}

pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    let re = Regex::new(
        r"- \[([ xX])\] \*\*Phase ([0-9.]+): (.+?)\*\*\s*[-—]\s*(.*)"
    ).unwrap();

    content
        .lines()
        .filter_map(|line| {
            re.captures(line).map(|caps| RoadmapPhase {
                completed: &caps[1] != " ",
                number: caps[2].to_string(),
                name: caps[3].to_string(),
                description: caps[4].trim().to_string(),
            })
        })
        .collect()
}
```

### Config Schema (gsd-manager's own config.json)
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Schema version for future migrations
    pub version: u32,
    /// Registered projects by alias
    pub projects: HashMap<String, RegisteredProject>,
    /// User preferences (placeholder for Phase 2+)
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredProject {
    /// Filesystem path to the project root
    pub path: PathBuf,
    /// When the project was registered
    pub added: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    // Placeholder — populated in later phases
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("gsd-manager")
            .join("config.json")
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| serde_yaml | serde_yml 0.0.12 | 2024 | serde_yaml is deprecated; serde_yml is the maintained fork |
| Manual panic hook for ratatui | `ratatui::init()` auto-installs panic hook | ratatui 0.28.1 (2024) | No more manual panic hook boilerplate |
| tui-rs | ratatui 0.30 | 2023 | tui-rs archived; ratatui is the official continuation |
| Manual raw mode/alt screen | `ratatui::init()` / `ratatui::restore()` | ratatui 0.28.1 | One-call terminal setup and teardown |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust (stable) | Everything | Yes | 1.94.0 | -- |
| Cargo | Build system | Yes | 1.94.0 | -- |
| git | Version control | Yes | (system) | -- |

**Missing dependencies:** None. Phase 1 has no external service dependencies.

## Open Questions

1. **serde_yml maturity**
   - What we know: serde_yml 0.0.12 is the maintained replacement for serde_yaml. Version number is low (0.0.x).
   - What's unclear: Whether 0.0.12 has any known issues with the YAML subset used in GSD frontmatter.
   - Recommendation: Use it but write thorough unit tests against real STATE.md files. If issues arise, fall back to manual frontmatter extraction + yaml-rust2 for the small YAML subset needed.

2. **STATE-02 scope ambiguity**
   - What we know: REQUIREMENTS.md says "phase progress indicators (completed vs total tasks from PLAN.md files)". CONTEXT.md D-05 says "Deeper extraction (PLAN.md task counts) added in later phases."
   - What's unclear: Whether Phase 1 must parse PLAN.md files or can use STATE.md's `progress.completed_plans`/`progress.total_plans`.
   - Recommendation: Use STATE.md frontmatter progress fields for Phase 1. They already contain the data. The success criteria says "Phase progress and backlog item counts are readable from the parsed state" which STATE.md satisfies.

3. **Event-driven vs tick-based rendering**
   - What we know: Unconditional 60fps rendering wastes CPU. Pure event-driven can miss edge cases.
   - What's unclear: Exact tick rate for the Phase 1 stub.
   - Recommendation: 250ms tick as a safety-net redraw, plus immediate redraw on any state-changing action. Target <2% idle CPU.

## Sources

### Primary (HIGH confidence)
- crates.io API via `cargo search` — verified ratatui 0.30.0, crossterm 0.29.0, tokio 1.50.0, serde 1.0.228, clap 4.6.0, serde_json 1.0.149, dirs 6.0.0, regex 1.12.3, tempfile 3.27.0, anyhow 1.0.102, color-eyre 0.6.5, tracing 0.1.44, tracing-subscriber 0.3.23, tracing-appender 0.2.4, serde_yml 0.0.12
- [ratatui panic hooks docs](https://ratatui.rs/recipes/apps/panic-hooks/) — `init()` auto-installs panic hook since 0.28.1
- [ratatui init() API docs](https://docs.rs/ratatui/latest/ratatui/fn.init.html) — terminal lifecycle functions
- [ratatui TEA pattern](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) — recommended architecture
- [ratatui async event stream tutorial](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/) — tokio + crossterm event pattern
- Real GSD project `.planning/` files — 4 projects examined for STATE.md, ROADMAP.md, config.json format consistency
- `rustc --version` — Rust 1.94.0 confirmed available on target machine

### Secondary (MEDIUM confidence)
- [ratatui component template structure](https://ratatui.rs/templates/component/project-structure/) — project layout recommendation
- Project STACK.md, ARCHITECTURE.md, PITFALLS.md research — pre-existing research validated against current crate versions

### Tertiary (LOW confidence)
- serde_yml stability assessment — version 0.0.12 suggests early development; needs validation through testing

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified against crates.io, Rust version confirmed
- Architecture: HIGH — TEA pattern well-documented by ratatui official docs; project structure follows official templates
- State reader: HIGH — examined 4 real GSD projects to understand format; fields and edge cases documented
- Pitfalls: HIGH — drawn from ratatui official docs + verified project-specific research

**Research date:** 2026-03-24
**Valid until:** 2026-04-24 (30 days — stable ecosystem, no expected breaking changes)
