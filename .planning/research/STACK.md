# Stack Research: v1.1 New Feature Dependencies

**Domain:** TUI project manager -- new capabilities for Claude session management, queue execution, git history, backlog browsing, execution flow graph
**Researched:** 2026-03-26
**Confidence:** HIGH (versions verified via crates.io and official docs)

## Context

This research covers ONLY new dependencies needed for v1.1 features. The existing stack (Rust 1.85+, ratatui 0.30, crossterm 0.29, tokio 1.50, notify 8, serde/serde_json/toml, anyhow/color-eyre, clap 4, tracing) is validated and not re-evaluated here.

## Recommended Stack Additions

### New Dependencies

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| sysinfo | 0.38.3 | Process detection (Claude sessions) | Cross-platform process enumeration by name/PID/cmdline; 38M+ downloads; the standard Rust crate for process inspection. Needed to detect running `claude` processes and map them to project directories via their command-line arguments |
| petgraph | 0.8.2 | Graph data structure (execution flow) | Standard Rust graph library; models the discuss/plan/execute/verify pipeline as a directed graph with status tracking. Provides topological sort, traversal, and dependency queries. Pure Rust, zero transitive bloat |

### Libraries Explicitly NOT Needed

| Library | Why Not | What to Do Instead |
|---------|---------|-------------------|
| git2 (0.20.4) | Adds libgit2 C dependency (~200MB build dep), requires cmake/pkg-config/openssl-dev on build host. Git history viewer only needs `git log` output | Shell out via `tokio::process::Command::new("git")` -- git is guaranteed present (GSD requires it); output parsing is trivial |
| gix / gitoxide | Pure Rust git but pulls 50+ transitive crates. Only need log output | `tokio::process::Command::new("git")` |
| ascii-petgraph (0.1) | Physics-based force-directed layout is wrong for a fixed 4-stage pipeline (discuss/plan/execute/verify). Only v0.1, immature | Custom ratatui `Widget` impl -- same pattern as existing `RoadmapWidget` |
| ratatui-graph | Unclear maintenance, general-purpose graph widget for arbitrary graphs | Custom Widget impl -- execution flow is a known-shape horizontal pipeline |
| tui-textarea / tui-input | For backlog text editing in TUI | Use ratatui's `Paragraph` with manual key handling for inline edits. Revisit only if editing grows complex |
| Claude Agent SDK (Python/TS) | Would require bundling Node.js or Python runtime, violating single-binary constraint | `claude -p` CLI invocation via `tokio::process` |
| nix / procfs | Low-level Unix-only process APIs | sysinfo 0.38 (cross-platform) |

## Feature-by-Feature Stack Analysis

### 1. Claude Session Detection and Management

**Approach:** Two-layer detection (filesystem + process)

**Layer 1 -- Filesystem detection (no new deps):**
- Parse `~/.claude/history.jsonl` -- each line is JSON with `project`, `sessionId`, `timestamp` fields (already have serde_json)
- Check `~/.claude/tasks/*/.lock` files for active task sessions (presence = running)
- Check `~/.claude/ide/*.lock` for IDE-attached sessions
- Read `~/.claude/projects/{encoded-path}/` for project-specific session data and subagent metadata
- Watch `~/.claude/history.jsonl` via existing notify infrastructure for real-time updates

**Layer 2 -- Process detection (sysinfo 0.38.3):**
- Enumerate processes named `claude` via `System::new_all()` + `system.processes_by_name()`
- Extract command-line args to determine: project directory (cwd), session ID (`--resume` arg), mode (interactive vs `-p` headless)
- Map running sessions to registered GSD projects by matching working directory to project paths

**Session launching (no new deps):**
- Spawn via `tokio::process::Command::new("claude")` with appropriate flags
- Parse `--output-format stream-json` output line-by-line with serde_json for real-time progress

**Key CLI flags for programmatic use (verified from official docs):**

| Flag | Purpose | Use Case |
|------|---------|----------|
| `-p "prompt"` | Headless/print mode, non-interactive | Queue execution |
| `--bare` | Skip auto-discovery (hooks, MCP, CLAUDE.md) | Queue execution (faster, deterministic) |
| `--resume <session-id>` | Continue specific session | Attaching to existing session |
| `--continue` | Continue most recent session | Quick resume |
| `--output-format json` | Structured JSON with session_id, result | Capturing completion results |
| `--output-format stream-json` | Newline-delimited JSON streaming | Real-time progress display in TUI |
| `--allowedTools "Read,Edit,Bash"` | Auto-approve specific tools | Unattended queue execution |
| `--verbose` | Include all events in stream output | Progress monitoring |

**Session data locations (verified on local filesystem):**

| Path | Contains | Format |
|------|----------|--------|
| `~/.claude/history.jsonl` | All prompts with project path, sessionId, timestamp | JSONL (one JSON object per line) |
| `~/.claude/projects/{encoded-path}/` | Per-project session data, subagent metadata | Mixed JSON files |
| `~/.claude/tasks/*/.lock` | Active task session indicators | Lock files (presence = running) |
| `~/.claude/ide/*.lock` | IDE-attached session indicators | Lock files |

### 2. Queue Execution

**Approach:** Spawn `claude -p` via tokio::process (no new deps)

**Implementation:**
- `tokio::process::Command` to spawn Claude CLI in headless mode with `kill_on_drop(true)`
- Stream `--output-format stream-json` output via `BufReader::new(child.stdout)` reading lines async
- Send parsed events through existing `mpsc` channel to TUI event loop
- Track `Child` process handle for cancellation

**Queue item lifecycle:**
```
Pending -> Running (spawn claude -p --bare) -> Complete / Failed
                                            -> Cancelled (child.kill())
```

**No new dependencies.** tokio already provides `process::Command`, `sync::mpsc`, and `select!` for racing process output with TUI events.

### 3. Git History Viewer

**Approach:** Shell out to `git log` via tokio::process (no new deps)

**Why not git2:** git2 pulls in the libgit2 C library, requiring cmake, pkg-config, and openssl-dev at build time. The only operation needed is `git log --format`. git is already a hard runtime dependency (GSD requires a git repo). Parsing pipe-delimited output is trivial.

**Implementation:**
```rust
let output = tokio::process::Command::new("git")
    .args(["log", "--format=%h|%s|%an|%ar", "-n", "100"])
    .current_dir(&project_path)
    .output()
    .await?;
```

For `.planning/`-scoped view: append `-- .planning/` to args.

**No new dependencies.**

### 4. Backlog Browser

**Approach:** Parse `.planning/phases/999.*` directories (no new deps)

**What's needed:**
- Read `.planning/phases/` directory entries matching `999.*` pattern (backlog convention)
- Parse description files within each directory -- already have regex and serde
- Display in ratatui's `List` or `Table` widget with scrolling
- For promotion (backlog to active phase): file operations via `tokio::fs`

**No new dependencies.** Existing stack handles file I/O, parsing, and rendering.

### 5. Execution Flow Graph

**Approach:** petgraph for data model, custom ratatui Widget for rendering

**Why petgraph:**
- Models the discuss -> plan -> execute -> verify pipeline as a `DiGraph<FlowStep, ()>`
- Provides dependency semantics: "can't execute until plan is complete"
- Topological ordering and traversal come free
- Pure Rust, tiny footprint (single crate, no transitive bloat)
- Future-proofs for more complex flows (parallel tracks, conditional steps)

**Why custom Widget (not ascii-petgraph):**
- Execution flow is a fixed 4-node horizontal pipeline, not an arbitrary graph
- ascii-petgraph uses physics simulation with jitter/settling -- inappropriate for a static known layout
- The codebase already has `RoadmapWidget` implementing custom box drawing -- same proven pattern
- Rendering: `[Discuss] --> [Plan] --> [Execute] --> [Verify]` with status-colored boxes

**Rendering approach:**
- Horizontal layout: 4 boxes connected by Unicode arrows
- Each box colored by status: gray (pending), yellow (active), green (complete), red (blocked)
- Direct `Buffer` writes (existing pattern from `RoadmapWidget`)

## Updated Cargo.toml Additions

```toml
[dependencies]
# NEW for v1.1
sysinfo = "0.38"        # Claude session process detection
petgraph = "0.8"        # Execution flow graph data model
```

Two new crates total. Everything else uses existing dependencies or the standard library.

## Alternatives Considered

| Recommended | Alternative | Why Not Alternative |
|-------------|-------------|---------------------|
| sysinfo for process detection | `/proc` parsing (Linux only) | Not cross-platform; sysinfo abstracts this cleanly |
| sysinfo for process detection | `Command::new("ps")` | Parsing ps output is fragile and platform-dependent; sysinfo gives typed data |
| `Command::new("git")` for history | git2 crate (libgit2 bindings) | C FFI build complexity for a simple log query |
| `Command::new("git")` for history | gix (gitoxide, pure Rust) | 50+ transitive crates for one command |
| petgraph for flow graph | `Vec<FlowStep>` with manual ordering | Loses dependency semantics and traversal; petgraph is tiny |
| petgraph for flow graph | ascii-petgraph for rendering | v0.1, physics layout wrong for fixed pipeline |
| Custom ratatui Widget for flow | tui-realm component framework | Adds React/Elm overhead for a simple horizontal pipeline |
| Manual key handling for backlog edit | tui-textarea crate | Extra dep for single-line input; revisit if editing grows complex |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| git2 / libgit2 | C FFI, cmake/openssl build requirements, ~200MB build overhead for a log query | `tokio::process::Command::new("git")` |
| gix (gitoxide) | 50+ transitive crates, still stabilizing API | `tokio::process::Command::new("git")` |
| ascii-petgraph | v0.1, physics-based force-directed layout wrong for fixed pipeline | Custom Widget impl |
| Claude Agent SDK (Python/TS) | Requires bundling a runtime, violates single-binary constraint | `claude -p` CLI invocation |
| nix crate | Low-level Unix API, not cross-platform | sysinfo 0.38 |
| procfs crate | Linux-only, no macOS support | sysinfo 0.38 |
| std::sync::Mutex with sysinfo in async | sysinfo calls are blocking; holding std mutex across await = deadlock | Call sysinfo in `spawn_blocking` or on a timer, not in async context |

## Stack Patterns for v1.1

**Pattern: Claude session detection as a periodic scan**
- Do NOT continuously poll processes (expensive)
- Scan every 5-10 seconds via `tokio::time::interval` + `spawn_blocking` for the sysinfo call
- Cache results in `Arc<RwLock<Vec<ClaudeSession>>>`
- Supplement with filesystem watching on `~/.claude/history.jsonl` via existing notify

**Pattern: Queue execution as supervised child processes**
- Spawn via `tokio::process::Command` with `kill_on_drop(true)`
- Stream `--output-format stream-json` via async `BufReader` on child stdout
- Send parsed events through existing `mpsc` channel to TUI event loop
- Never block the render loop -- all process I/O goes through tokio channels

**Pattern: Git commands as async one-shots**
- Run `git log` via `Command::output()` (collect all stdout at once)
- Parse in `spawn_blocking` if output is large
- Cache results; invalidate on filesystem changes (existing notify watcher covers `.planning/`)

**Pattern: Execution flow as data-driven rendering**
- `petgraph::DiGraph<FlowStep, ()>` models the pipeline per phase
- Custom Widget reads graph state and renders boxes + arrows
- State mutations go through message handlers (existing architecture pattern)

## Version Compatibility

| New Package | Compatible With | Notes |
|-------------|-----------------|-------|
| sysinfo 0.38.3 | Rust 1.85+ | MSRV 1.74; no conflicts with existing deps |
| sysinfo 0.38.3 | tokio 1.x | sysinfo is synchronous; wrap in `spawn_blocking` or use on interval timer |
| petgraph 0.8.2 | Rust 1.85+ | Pure Rust, zero transitive conflicts |
| petgraph 0.8.2 | ratatui 0.30 | No interaction -- petgraph is data model only; rendering is a separate custom Widget |

## Sources

- [sysinfo 0.38.3 on crates.io](https://crates.io/crates/sysinfo) -- latest stable version verified 2026-03-02 release (HIGH confidence)
- [sysinfo docs.rs](https://docs.rs/sysinfo/latest/sysinfo/) -- process enumeration API (HIGH confidence)
- [petgraph 0.8.2 on crates.io](https://crates.io/crates/petgraph) -- latest stable version verified (HIGH confidence)
- [git2 0.20.4 on crates.io](https://crates.io/crates/git2) -- evaluated and rejected due to C dependency (HIGH confidence)
- [git2-rs log example](https://github.com/rust-lang/git2-rs/blob/master/examples/log.rs) -- complexity assessment (HIGH confidence)
- [ascii-petgraph 0.1 on crates.io](https://crates.io/crates/ascii-petgraph) -- evaluated and rejected, physics layout wrong for use case (HIGH confidence)
- [Claude Code headless/programmatic docs](https://code.claude.com/docs/en/headless) -- CLI flags, session management, output formats (HIGH confidence)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) -- --resume, --continue, --output-format flags (HIGH confidence)
- [tokio::process::Command docs](https://docs.rs/tokio/latest/tokio/process/struct.Command.html) -- async process spawning API (HIGH confidence)
- Local filesystem inspection of `~/.claude/` -- session storage structure (history.jsonl format, tasks/.lock pattern, projects/ structure) verified on live installation (HIGH confidence)

---
*Stack research for: gsd-meta-manager v1.1 new feature dependencies*
*Researched: 2026-03-26*
