<!-- GSD:project-start source:PROJECT.md -->
## Project

**GSD Meta Manager**

A TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — without opening separate terminals or running `/gsd:progress` in each directory.

**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.

### Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Language Decision: Rust
## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust | 1.85+ (stable) | Language | Zero-cost abstractions, single-binary distribution, no GC pauses, Cargo ecosystem |
| ratatui | 0.30.0 | TUI rendering | The de facto standard Rust TUI library, 11.9M+ downloads, active maintenance, modular since 0.30 |
| crossterm | 0.29.0 | Terminal backend | Cross-platform (macOS/Linux/Windows), event stream support, ratatui's recommended backend |
| tokio | 1.50.0 | Async runtime | Standard async runtime; required for concurrent file watching + event handling without blocking the render loop |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| notify | 8.0.0 | Filesystem watching | Watch `.planning/` directories for live state updates; use 8.x (stable), not 9.0 rc |
| serde | 1.0.228 | Serialization | Deserialize `config.json`, `STATE.md` (JSON sections), any structured files |
| serde_json | 1.0.149 | JSON parsing | Parse GSD's `config.json` and JSON-structured state files |
| toml | 1.1.0 | TOML parsing | User config (`~/.config/gsd-meta-manager/config.toml`) for registered project paths |
| anyhow | 1.0.102 | Error handling | Ergonomic error propagation through TUI event/render loops; avoids boilerplate |
| color-eyre | 0.6.5 | Error reporting | Rich terminal error reports with context; install as the panic/error handler at startup |
| clap | 4.6.0 | CLI argument parsing | Handle `--config`, `--projects-dir`, `--version` flags on launch |
| tracing | 0.1.44 | Structured logging | Log to a file (not stdout, which ratatui owns); critical for debugging render/state issues |
| tracing-subscriber | 0.3.x | Log output routing | Route logs to `~/.local/share/gsd-meta-manager/gsd-meta-manager.log` instead of terminal |
### Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-watch | Live rebuild during development | `cargo watch -x run` — avoids manual restart cycle |
| cargo-nextest | Faster test runner | Parallel test execution; better output than `cargo test` |
| bacon | Background checker | Runs `cargo check` on save; surfaces errors without rebuilding |
## Installation
# Core (Cargo.toml [dependencies])
# Dev dependencies (Cargo.toml [dev-dependencies])
# (no additional dev deps required for this stack)
# Dev tools (install once)
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Rust + ratatui | Python + Textual | Internal tool where distribution target is a controlled environment (e.g., team with enforced Python version); faster to prototype |
| ratatui | cursive | If you need an older retained-mode widget API; cursive is slower to update and has less community momentum in 2025 |
| ratatui | tui-realm (on top of ratatui) | If the app grows to 10+ distinct screens with complex component lifecycles; tui-realm adds React/Elm state machine overhead not justified for a focused dashboard |
| tokio (async) | std::thread + channels | If the project had no I/O concurrency; this project watches N directories + handles input simultaneously, so async is warranted |
| notify 8.x | notify 9.x rc | 9.0 is still in rc as of 2026-03-24; use 8.x for stable production behavior |
| crossterm backend | termion backend | termion is Unix-only; crossterm works on Windows too, better for a tool targeting "any GSD user" |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| tui-rs (fdehau/tui-rs) | Archived/unmaintained; ratatui is the official fork and continuation | ratatui 0.30 |
| Python + Textual | Requires users to manage Python runtime and virtualenv; unacceptable distribution burden for a tool targeting any user | Rust + ratatui |
| ncurses / pancurses | C FFI dependency, platform-specific, notoriously painful to install on macOS, poor Rust ergonomics | crossterm |
| termion | Unix-only; cross-platform portability matters for a tool distributed to arbitrary users | crossterm |
| std::sync::Mutex in async code | Holding across `.await` points causes deadlocks | tokio::sync::Mutex |
| env_logger | Only writes to stderr; ratatui owns the terminal — logs must go to a file | tracing + tracing-subscriber with file appender |
## Stack Patterns by Variant
- Use `tokio::select!` to race between: crossterm event stream, notify filesystem events, tick interval
- Never block the render loop — all I/O goes through tokio channels (mpsc)
- Render is always driven by state; state is mutated only by message handlers
- Use ratatui's `Canvas` widget with custom `Shape` implementations for boxes and arrows
- `Block` widget handles bordered panels without custom drawing
- For directed graphs/DAG rendering, implement custom ratatui `Widget` trait; no suitable third-party crate exists yet for this specific use case
- Parse `.planning/` files lazily on demand, cache parsed state in `Arc<RwLock<HashMap<PathBuf, ProjectState>>>`
- Invalidate cache entries when notify fires events for that project's directory
- Build with `cargo build --release --target x86_64-unknown-linux-gnu` (and `aarch64-apple-darwin`)
- Consider `cargo-dist` for automated release artifact generation
## Version Compatibility
| Package | Compatible With | Notes |
|---------|-----------------|-------|
| ratatui 0.30 | crossterm 0.29 | ratatui 0.30 ships its own `ratatui-crossterm` backend crate in the modular workspace; use the feature flag `crossterm` on ratatui |
| tokio 1.x | crossterm 0.29 event-stream | crossterm's `event-stream` feature requires tokio or async-std; use `crossterm = { version = "0.29", features = ["event-stream"] }` |
| notify 8.x | tokio 1.x | notify 8 supports async watcher via `notify-debouncer-mini` or `notify-debouncer-full` companion crates |
| serde 1.x | toml 1.x, serde_json 1.x | All use serde 1.0; no version conflicts |
| color-eyre 0.6 | anyhow 1.x | Both compatible; use color-eyre for the top-level error handler, anyhow for internal error propagation |
## Sources
- cargo search / crates.io API — ratatui 0.30.0, crossterm 0.29.0, tokio 1.50.0, serde 1.0.228, anyhow 1.0.102, clap 4.6.0, toml 1.1.0, serde_json 1.0.149, tracing 0.1.44 (HIGH confidence, verified directly)
- notify crates.io API — 8.0.0 stable confirmed, 9.0.0-rc.2 is pre-release (HIGH confidence)
- [ratatui GitHub releases](https://github.com/ratatui/ratatui/releases) — 0.30.0 confirmed as latest stable (HIGH confidence)
- [ratatui.rs official site](https://ratatui.rs/) — architecture patterns, backend recommendations (HIGH confidence)
- [Textual PyPI](https://pypi.org/project/textual/) — 6.5.0 current version (MEDIUM confidence, not verified with official changelog)
- [LibHunt: ratatui vs textual](https://www.libhunt.com/compare-ratatui-vs-textual) — performance comparison data (MEDIUM confidence, single source)
- [DEV.to: Go vs Rust TUI Deep Dive](https://dev.to/dev-tngsh/go-vs-rust-for-tui-development-a-deep-dive-into-bubbletea-and-ratatui-2b7) — 30-40% memory advantage for Rust (LOW confidence, single benchmark article)
- [tui-realm GitHub](https://github.com/veeso/tui-realm) — component architecture patterns, version 3.1.0 (MEDIUM confidence)
- [ratatui component architecture docs](https://ratatui.rs/concepts/application-patterns/component-architecture/) — application pattern guidance (HIGH confidence)
- [notify-rs GitHub](https://github.com/notify-rs/notify) — cross-platform filesystem watching, MSRV 1.85 (HIGH confidence)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
