# Phase 15: Transport Foundation — Duplex stream-json Executor - Pattern Map

**Mapped:** 2026-07-29
**Files analyzed:** 14 (5 new source, 4 modified source, 1 manifest, 2 new test surfaces, 8 doc sites)
**Analogs found:** 10 / 14

> Read `15-CONTEXT.md` (D-01…D-32) and `15-RESEARCH.md` first. This document answers only
> one question: **for each file this phase creates or modifies, which existing file is the
> pattern to copy, and what exactly should be copied.**

---

## File Classification

| New/Modified File | New? | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `src/executor/mod.rs` | new | module root + domain types | (types only) | `src/state_reader/mod.rs:1-50` (mod decls + central struct) / `src/archive.rs:1-45` (pure domain enums+structs) | role-match |
| `src/executor/stream_json.rs` | new | model / parser | transform (line → typed) | `src/state_reader/config_json.rs:1-80` (serde-tolerant deserialize) | role-match |
| `src/executor/claude.rs` | new | service (process transport) | streaming / event-driven | `src/state_reader/git_ops.rs:114-208` (async `tokio::process` + output parse) + `src/event.rs:18-43` (`tokio::spawn` → `mpsc`) + `src/watcher.rs:37-90` (long-lived producer struct owning a channel sender) | **partial — no duplex/streaming subprocess exists today** |
| `src/executor/outcome.rs` (optional, D-12/D-26) | new | service (pure derivation) | transform | `src/state_reader/disk_status.rs` (pure inference fn + `PartialEq` struct) | role-match |
| `src/error.rs` | modify | error types | — | `src/registry.rs` / `anyhow::Context` idiom in `src/config.rs:60-87` | partial (file is a 3-line stub) |
| `src/main.rs` (`run_tui_loop` → `select!` + `pump()`) | modify | event loop | event-driven | `src/main.rs:134-205` **is its own analog** — preserve shape, add arms | exact (self) |
| `src/action.rs` | modify (maybe) | message contract | event-driven | `src/action.rs:19-23` (`ProjectStateLoaded` boxing precedent) | exact |
| `src/ui/screens/mod.rs` (`AppContext` sibling map, D-19) | modify | state container | — | `AppContext.archive_cache` / `.last_refresh` (`src/ui/screens/mod.rs:111-132`) | exact |
| `src/app.rs` (`apply_exec_event`, `App::new_for_test`) | modify | controller | event-driven | `App::update` + `App::new` (`src/app.rs:118-158`, `:235+`) | exact |
| `Cargo.toml` | modify | config | — | `Cargo.toml:14-37` | exact |
| `tests/fixtures/transcripts/*.ndjson` | new | test fixture | — | *(none — repo has no `tests/fixtures/` today)* | **no analog** |
| `tests/fixtures/fake-claude.sh` (+ variants) | new | test fixture (executable) | — | *(none)* | **no analog** |
| `tests/executor_transport.rs`, `tests/executor_lifecycle.rs` | new | integration test | — | `tests/state_reader_test.rs:1-60` (plain `#[test]` + `tempfile::TempDir`), `tests/registry_test.rs:1-20` (`assert_fs`) | role-match |
| README.md / CLAUDE.md / CONTRIBUTING.md / docs/*.md (MSRV) | modify | docs | — | the 8 sites themselves | exact |

**Two "no analog" facts the planner must internalize:**

1. **The repo contains zero `tokio::select!`, zero bounded `mpsc::channel(N)`, zero
   `#[serde(tag = ...)]`, and zero long-lived child-process handling.** Verified by grep.
   Every subprocess site in the tree is fire-and-forget `output()` (`git_ops.rs`,
   `session_detector.rs`, `terminal_switch.rs`) or blocking `status()` (`main.rs:166`).
   For `claude.rs`'s pipe topology and supervisor, **RESEARCH.md's Pattern 1 / Pattern 3 and
   the verified `process-wrap` API surface are the pattern source, not this codebase.**
2. **`tests/` has no `fixtures/` directory.** Both fixture surfaces are greenfield.

---

## Pattern Assignments

### `src/executor/stream_json.rs` (model/parser, transform)

**Analog:** `src/state_reader/config_json.rs`

The repo's *only* tolerant-serde precedent. Every field is `#[serde(default)]` +
`Option<T>`, unknown fields are silently ignored (no `deny_unknown_fields` anywhere in the
tree), and per-field `#[serde(rename)]` is already the idiom for wire-name mismatches.

**Derive + default pattern** (`config_json.rs:1-13`):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GsdConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub commit_docs: Option<bool>,
```

**Per-field rename pattern** (`config_json.rs:155`) — exactly what D-09 / Pitfall F needs
for `apiKeySource` and `isReplay` in a struct whose other fields are snake_case:

```rust
    #[serde(rename = "_auto_chain_active", default)]
```

**Escape hatch for unmodelled payloads** (`config_json.rs:57`) — the precedent for
`rate_limit_event` and `permission_denials[]` being `serde_json::Value`:

```rust
    pub sub_repos: Option<serde_json::Value>,
```

**Boxing large variants** — mandatory, from `src/action.rs:19-23`:

```rust
    ProjectStateLoaded {
        alias: String,
        // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
        state: Box<ProjectState>,
    },
```

Apply verbatim to `StreamMessage::Result(Box<ResultMessage>)` and
`SystemMessage::Init(Box<InitMessage>)`. `clippy::large_enum_variant` is a `-D warnings`
failure at the lib target, so this is a build gate, not style.

**Do NOT copy from `config_json.rs`:** it derives `Serialize` too. `StreamMessage` is
read-only — derive `Deserialize` only. The one outbound type (the stdin `UserMessage`
wire shape, `{"type":"user","message":{"role":"user","content":[{"type":"text",…}]}}`)
gets `Serialize` only.

**Test pattern:** in-file `#[cfg(test)] mod tests` (18 source files use it), feeding
`include_str!` fixtures. See `src/watcher.rs:92-117` for the minimal shape.

---

### `src/executor/claude.rs` (service, streaming/event-driven)

No single analog. Compose three.

**Analog A — async subprocess + arg construction:** `src/state_reader/git_ops.rs:114-166`

```rust
pub async fn load_git_log(
    project_path: &Path,
    planning_only: bool,
    limit: usize,
) -> anyhow::Result<Vec<GitLogEntry>> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(project_path);
    cmd.args([
        "log",
        &format!("--max-count={}", limit),
        "--format=%h\x1f%ad\x1f%an\x1f%s",
        "--date=short",
    ]);

    if planning_only {
        cmd.arg("--").arg(".planning/");
    }

    let output = cmd.output().await?;
```

Copy: `current_dir(project_path)`, incremental `args([...])` / conditional `.arg()` argv
building — **never a shell string** (matches the argument-injection mitigation in
RESEARCH's Security Domain). Diverge: `process-wrap`'s `CommandWrap::with_new("claude",
|cmd| …)` takes the closure instead of a `&mut Command`, and you need
`.stdin/.stdout/.stderr(Stdio::piped())` rather than `.output()`.

**Analog B — spawned task feeding an mpsc sender:** `src/event.rs:18-43`

```rust
    pub fn spawn_crossterm_reader(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                if let Some(action) = map_event_to_action(event) {
                    if tx.send(action).is_err() {
                        break;
                    }
                }
            }
        });
    }
```

Copy exactly for the stdout and stderr reader tasks: clone the sender into the task,
`while let Some(...) = …await`, and **`if tx.send(..).is_err() { break; }`** — the
established convention for "receiver dropped, stop producing". With a *bounded* channel
(RESEARCH Open Question 4) the analogous call is `tx.send(ev).await` and the same
`is_err() => break`; do **not** use `try_send` and silently drop, and do not block
indefinitely without a note — Claude's 30s exit-drain cap is on the other end.

**Analog C — long-lived producer struct that owns the sender:** `src/watcher.rs:16-90`

```rust
pub struct FileWatcher {
    debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl FileWatcher {
    pub fn new(tx: UnboundedSender<Action>) -> anyhow::Result<Self> { … }
    pub fn watch(&mut self, planning_dir: &Path) -> anyhow::Result<()> { … }
    pub fn unwatch(&mut self, planning_dir: &Path) -> anyhow::Result<()> { … }
}
```

`ExecutionHandle` should follow this ownership shape: a struct handed the sender at
construction, exposing `&mut self` verbs. `FileWatcher` also shows the error-logging
convention paired with a returned error:

```rust
            .map_err(|e| {
                tracing::warn!("Failed to watch {}: {}", planning_dir.display(), e);
                anyhow::anyhow!("Failed to watch {}: {}", planning_dir.display(), e)
            })
```

Use `tracing::warn!` the same way for teardown/reap failures. **Constraint from RESEARCH's
Security Domain (V7):** never `tracing::info!` a raw stream line — event type + counts at
`info`, raw at `trace` only.

**Analog D — process discipline comment:** `src/session_detector.rs:16-21`

```rust
/// Detect active Claude Code sessions by inspecting the Linux /proc filesystem.
///
/// Uses `pgrep -x claude` to find PIDs, then reads /proc entries for each.
/// Silently skips any PID where reads fail (stale/exited processes).
/// Uses std::process::Command (not tokio) — called from spawn_blocking.
```

This doc-comment convention (state the sync/async contract and the failure posture in the
doc comment) is the house style — `claude.rs`'s module doc must state the inverse:
"async `tokio::process` via `process-wrap`; never called from `spawn_blocking`; parsing
happens in the reader task, never on the render thread."

**Analog E — module-level design-rationale header:** `src/watcher.rs:1-5` and
`src/terminal_switch.rs:1-11` both open with a `//` / `//!` block explaining *why* the
design is what it is. `claude.rs` should carry the D-14 teardown rationale (`signal(15)`
before `start_kill()`, and why `kill()` is wrong) as such a header — Pitfall B is exactly
the mistake a future reader makes without it.

---

### `src/executor/outcome.rs` (service, pure transform)

**Analog:** `src/state_reader/disk_status.rs` + `src/state_reader/mod.rs:103-115`

The disk half of TRANS-02 is almost entirely reuse. `ProjectState` already derives
`PartialEq` (`src/state_reader/mod.rs:13`):

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectState {
```

so D-11's "made_changes signal" is `before != after` on a value produced twice by:

```rust
/// Parse a GSD project's .planning/ directory into a ProjectState.
/// Gracefully handles missing or malformed files -- never panics.
pub fn parse_project_state(planning_dir: &Path) -> ProjectState {
```

**Call it from `spawn_blocking`, per the established precedent** (`src/app.rs:284-296`):

```rust
                    if let Some(tx) = &self.ctx.event_tx {
                        let tx = tx.clone();
                        let alias_for_task = alias.clone();
                        let planning_dir = project_path.join(".planning");
                        tokio::task::spawn_blocking(move || {
                            let state = state_reader::parse_project_state(&planning_dir);
                            let _ = tx.send(Action::ProjectStateLoaded {
                                alias: alias_for_task,
                                state: Box::new(state),
                            });
                        });
```

`parse_project_state` is synchronous and does full-tree file I/O — the before/after
snapshots must not run on the async reactor thread.

**New git helpers (`head_sha()`, `is_dirty()`) go IN `src/state_reader/git_ops.rs`,
next to the existing ones (D-11: "do not write a parallel one").** Two shapes already
exist there; pick by caller:

*Sync shape* (`git_ops.rs:11-32`) — use this if called from the same `spawn_blocking`
closure as `parse_project_state`:

```rust
fn git_last_commit_time(project_root: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
```

*Async shape* (`git_ops.rs:161-170`) — `tokio::process::Command` + `.output().await`,
returning `anyhow::Result<T>` with a **non-failing default on non-zero exit**:

```rust
    if !output.status.success() {
        return Ok(GitDiffStat::default());
    }
```

Copy the `-C <root>` + `String::from_utf8_lossy` + `Option`/default-on-failure posture.
Note the file is inconsistent about `-C` vs `.current_dir()`; either is fine, but be
consistent within the new helpers.

**Outcome-matrix test pattern:** the exhaustive-combination style has a precedent in
`tests/state_reader_test.rs` (one `#[test]` per input shape, descriptive snake_case names,
no shared harness). D-26's matrix should be one `#[test]` per distinguishable
(`subtype` × `is_error` × `terminal_reason` × exit code × disk-changed) combination in
`#[cfg(test)] mod tests` inside `outcome.rs`, per RESEARCH's test map
(`cargo test --lib executor::outcome`).

---

### `src/executor/mod.rs` (module root + domain types)

**Analog for module-root shape:** `src/state_reader/mod.rs:1-11`

```rust
pub mod backlog;
pub mod config_json;
pub mod disk_status;
pub mod git_ops;
…

use std::collections::HashMap;
use std::path::{Path, PathBuf};
```

Submodule declarations first, then the central shared type. `src/executor/mod.rs` mirrors
this: `pub mod claude; pub mod stream_json; pub mod outcome;` then `Executor`,
`ExecutionOptions`, `ExecutionHandle`, `ExecutionEvent`, `RunOutcome`, `ExecutionTarget`,
`DrivableProject`.

**Analog for the pure domain enum/struct block:** `src/archive.rs:6-44`

```rust
/// Depth levels for archive drill-down navigation.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ArchiveDepth {
    /// Level 0: List of milestones (v1.0, v1.1, ...)
    #[default]
    MilestoneList,
    /// Level 1: Phases within a selected milestone
    PhaseList { milestone: String },
```

Copy: doc comment on every variant, struct-variants with named fields (not tuples) once a
variant carries more than one datum. `RunOutcome`'s eight-ish variants (D-12) should read
like this.

**`ExecutionTarget` (D-21, single `Host` variant today):** add
`#[non_exhaustive]` or accept that Phase 22 adds a variant — but note `DetailSubView`
(`src/app.rs:15-28`) is the repo's precedent for a plain grow-over-time enum with
`#[derive(Debug, Clone, PartialEq, Default)]` and no `#[non_exhaustive]`. Follow it;
the crate is the only consumer.

**Registering the module:** `src/lib.rs` is a flat alphabetical list of `pub mod` lines —
insert `pub mod executor;` between `pub mod error;` and `pub mod project_creator;`.

---

### `src/error.rs` (error types)

**Analog:** none in-tree — the file is 3 lines:

```rust
// Error types for gsd-meta-manager.
// Currently using anyhow for error handling throughout the application.
// This module is reserved for future custom error types if needed.
```

The whole codebase uses `anyhow::Result` + `.with_context()` (`src/config.rs:60-87`) or
`Result<(), String>` for user-facing status-bar messages (`src/terminal_switch.rs:19`).
There is **no `thiserror` dependency and no custom error enum anywhere.**

RESEARCH's Open Question 3 recommends: `RunOutcome` → `src/executor/mod.rs` (it is a
domain type with success variants), and the genuinely error-shaped `SpawnError` /
`SendError` / `CapabilityError` → `src/error.rs`. If the planner takes that split, note it
introduces the codebase's first hand-written error enums; either add `thiserror` (a real
Cargo change nobody has budgeted) or hand-write `Display` + `std::error::Error`. Prefer
hand-written `Display` — three small enums do not justify a dependency, and it keeps the
"no new deps beyond `process-wrap` + `uuid`" line clean.

The boundary to preserve: **public fns keep returning `anyhow::Result`** at the seams that
existing callers touch, per `config.rs`/`registry.rs`/`git_ops.rs`. Only the executor's
own surface uses typed errors.

---

### `src/main.rs` — `run_tui_loop` restructure (event loop, event-driven)

**Analog: itself.** `src/main.rs:134-205` is the code being changed, and D-18 says preserve
observable behavior. Copy the current body forward verbatim except for the `recv` arm.

**Current shape to preserve** (`main.rs:139-153` and `:199-201`):

```rust
    loop {
        // Sync needs_redraw from ctx (screens set ctx.needs_redraw)
        if app.ctx.needs_redraw {
            app.needs_redraw = true;
            app.ctx.needs_redraw = false;
        }

        if app.needs_redraw {
            terminal.draw(|frame| gsd_meta_manager::ui::render(frame, app))?;
            app.needs_redraw = false;
        }

        if let Some(action) = rx.recv().await {   // ← THIS is what becomes select!
            app.update(action);
        }
        …
        if app.should_quit {
            break;
        }
    }
```

**The `pending_editor` block (`main.rs:155-197`) moves unchanged** — including
`ratatui::restore()`, the `VISUAL`>`EDITOR`>`vi` resolution, the blocking
`std::process::Command::new(&editor).arg(&path).status()`, the three-arm `match status`
writing `app.ctx.status_message = Some((msg, std::time::Instant::now()))`, and
`*terminal = ratatui::init(); app.needs_redraw = true;`. D-18: do not touch it.

**Replacement `select!` shape:** RESEARCH.md "Architecture Patterns → Pattern 3" has the
full literal code. Non-negotiables from it:
- `biased;` with input first, exec events second, redraw tick third.
- The exec channel is **separate and bounded**; do not widen `Action`.
- Bounded drain (`EXEC_BATCH`) via `try_recv()` in the exec arm.
- **Pitfall C:** hold a long-lived `exec_tx` clone on `AppContext` so the arm never
  self-disables, or add `else => break`. This is the most likely concrete bug.
- **Keep `spawn_tick(250)`** (`main.rs:119`) — it drives the 20-tick session poll
  (`app.rs:246-257`) and the 3s status-message expiry. The 16ms redraw interval is
  additive, not a replacement.

**Wiring precedent for the new channel** (`main.rs:86-92`, `:118-123`):

```rust
            let event_bus = EventBus::new();

            // Store event_tx on App context so creation flow can send actions back
            app.ctx.event_tx = Some(event_bus.tx.clone());
            …
            event_bus.spawn_crossterm_reader();
            event_bus.spawn_tick(250);

            let mut rx = event_bus.rx;

            let result = run_tui_loop(&mut terminal, &mut app, &mut rx).await;
```

Create the exec channel here, stash the `tx` clone on `AppContext` (same line as
`event_tx`), and pass `&mut exec_rx` as `run_tui_loop`'s fourth parameter.

**Testability lever (RESEARCH Pattern 3):** extract the `select!` body into
`async fn pump(app, rx, exec_rx, batch) -> PumpOutcome`. Required for all three TRANS-03
tests. Note `pump` must live somewhere testable — `main.rs` is a binary, not the lib.
`src/main.rs` currently declares `mod event; mod tui;` locally and imports everything else
from `gsd_meta_manager::`. **Put `pump()` in the library** (e.g. `src/app.rs` or a new
`src/main_loop.rs` added to `src/lib.rs`) or the RESEARCH test map's
`cargo test --lib main_loop::keypress_priority` cannot resolve.

---

### `src/action.rs` (message contract)

**Analog: itself.** If any new `Action` variant is needed (likely not — D-17 says stream
traffic uses the separate channel), copy the boxing comment convention verbatim:

```rust
    ProjectStateLoaded {
        alias: String,
        // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
        state: Box<ProjectState>,
    },
```

**Hard constraint (D-19):** `#[derive(Debug, Clone)]` on line 5 means no `ChildStdin`,
`JoinHandle`, or `Box<dyn ChildWrapper>` may ever be placed in an `Action`. `FileChanged`
stays `{ project_path }` — adding `changed_path` is Phase 16's.

---

### `src/ui/screens/mod.rs` — `AppContext` sibling map (D-19)

**Analog:** `AppContext`'s existing per-alias sibling maps (`src/ui/screens/mod.rs:111-132`):

```rust
pub struct AppContext {
    …
    pub event_tx: Option<UnboundedSender<Action>>,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    …
    pub archive_cache: HashMap<String, crate::archive::MilestoneArchive>,
}
```

Copy exactly this shape for driver state: `pub run_states: HashMap<String, RunState>` keyed
by alias, plus `pub exec_tx: Option<mpsc::Sender<ExecutionEvent>>` alongside `event_tx`.

**Do NOT put driver state on `ProjectState`** — it derives `PartialEq`
(`src/state_reader/mod.rs:13`) and `app.rs` uses that equality to suppress
"Updated: {alias}" spam.

**Matching initializer** — every field added here needs a line in `App::new`
(`src/app.rs:126-153`), which constructs `AppContext` with an explicit exhaustive literal
(no `..Default::default()`). Adding a field there is also what makes the RESEARCH Wave-0
gap `App::new_for_test()` cheap: add a constructor next to `App::new` that skips
`load_config(&config_path)?` and starts from `Config::new()`.

---

### `Cargo.toml`

**Analog:** the existing manifest (`Cargo.toml:1-37`). Additions per D-03/D-20 and
RESEARCH's Standard Stack:

```toml
[package]
…
edition = "2021"
rust-version = "1.87"     # NEW — no rust-version key exists today
```

```toml
[dependencies]
…
process-wrap = { version = "9.1.0", features = ["tokio1"] }   # tokio1 is NOT default
uuid = { version = "1.24", features = ["v4", "serde"] }
```

Feature-flag style already matches the file (`ratatui = { version = "0.30", features =
["crossterm"] }`). `tokio = { version = "1", features = ["full"] }` already provides
`process`, `io-util`, `sync`, `time`, `macros` — no change. Per D-05/RESEARCH,
`tokio-util` is **not** added; use `BufReader::lines()` plus an explicit max-line-length
guard. `[dev-dependencies]` currently holds only `assert_fs = "1"`; nothing new is needed
(`tempfile` is already a runtime dep).

**Release-process note (CLAUDE.md):** `Cargo.toml` `version` is the milestone tag's job,
not this phase's — do not bump it here.

---

### `tests/executor_transport.rs`, `tests/executor_lifecycle.rs`

**Analog:** `tests/state_reader_test.rs:1-10`

```rust
use gsd_meta_manager::state_reader::config_json::parse_gsd_config;
use gsd_meta_manager::state_reader::{count_backlog_items, parse_project_state};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// STATE.md parsing tests
// ============================================================================

#[test]
fn test_parse_real_state_md() {
```

Copy: import from `gsd_meta_manager::…` (the lib, never `crate::`), `tempfile::TempDir` for
scratch dirs (the dominant choice — `assert_fs` appears only in `tests/registry_test.rs`),
banner comments to section the file, plain `#[test]` fns with descriptive snake_case names.

**Naming inconsistency to resolve deliberately:** `state_reader_test.rs` prefixes with
`test_`, `registry_test.rs` does not (`add_project_with_duplicate_alias_returns_error`).
The unprefixed, behavior-describing form reads better and matches RESEARCH's test-map names
(`refuses_missing_capability`, `success_without_changes_is_noop`); use it.

**Assertion-message convention** (`tests/registry_test.rs:16`, `:32-36`) — always carry the
observed value:

```rust
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    …
    assert!(
        err_msg.contains("already exists"),
        "Expected 'already exists' in error, got: {}",
        err_msg
    );
```

**Gate the process-spawning tests:** `#[cfg(unix)]`, and `#[ignore]` the ones with real
10s grace periods, documented in `docs/TESTING.md` (RESEARCH "Testing Without Spawning
`claude`"). **Integration tests count toward `cargo clippy --all-targets`** — the
pre-existing lint count is exactly 5 and must not grow.

---

### MSRV documentation surface (D-20)

**Verified sites** (all currently say 1.85, grep-confirmed this session):

| File:line | Current text |
|-----------|--------------|
| `README.md:61` | `Requires **Rust 1.85+**.` |
| `README.md:186` | `- **Rust 1.85+** (for building from source)` |
| `CLAUDE.md:25` | stack table row `\| Rust \| 1.85+ (stable) \| Language \| …` |
| `CONTRIBUTING.md:23` | `Prerequisites: Rust 1.85+ (stable) with \`cargo\` on your \`PATH\`.` |
| `docs/DEVELOPMENT.md:10` | `- **Rust 1.85+** (stable) -- required by the \`Cargo.toml\` \`edition = "2021"\` setup and` |
| `docs/GETTING-STARTED.md:13` | table row `\| Rust toolchain \| \`>=1.85\` (stable) \| … \`notify 8.x\` (MSRV 1.85). \|` |
| `docs/GETTING-STARTED.md:150` | `### "error: package requires rustc 1.85 or newer"` |
| `docs/TESTING.md:14` | `… working Rust toolchain (\`1.85+ stable\`, per the project's stack` |

**One extra site NOT in D-20's list, and it should stay 1.85:** `CLAUDE.md:99` —
`- [notify-rs GitHub](…) — cross-platform filesystem watching, MSRV 1.85` — that is a
citation about *notify's* MSRV, not the project's. Leave it.

**Also update the rationale, not just the number.** `docs/DEVELOPMENT.md:10` and
`docs/GETTING-STARTED.md:13` both attribute the floor to `edition = "2021"` / `notify 8.x`.
The new floor's cause is `process-wrap` 9.1.0 (`rust-version = "1.87.0"`, verified in its
published manifest). Copying "1.85 → 1.87" without fixing the attribution leaves two docs
stating a false reason.

---

## Shared Patterns

### Never-panic, degrade-gracefully parsing
**Source:** `src/state_reader/mod.rs:101-103`, `src/session_detector.rs:16-20`,
`src/watcher.rs:61-66`
**Apply to:** `stream_json.rs`, `outcome.rs`, both reader tasks

```rust
/// Parse a GSD project's .planning/ directory into a ProjectState.
/// Gracefully handles missing or malformed files -- never panics.
```

Every parser in this tree returns `Option`/default rather than erroring, and says so in its
doc comment. D-09's `Unparseable` variant is the same discipline. **No `unwrap()` on any
parsed stream field** (RESEARCH Security Domain V5).

### Silent-skip on failure, `tracing::warn!` on the interesting ones
**Source:** `src/watcher.rs:61-66`, `src/main.rs:96-102`
**Apply to:** reader tasks, teardown, capability gate

```rust
                    Err(errors) => {
                        for error in errors {
                            tracing::warn!("File watcher error: {}", error);
                        }
                    }
```

and

```rust
                    if let Err(e) = watcher.watch(&planning_dir) {
                        tracing::warn!(
                            "Could not watch {}: {}",
                            planning_dir.display(),
                            e
                        );
                    }
```

`tracing::warn!` is the only log level in production use today (grep: no `info!`/`debug!`
in `src/`). Follow it, and per V7 keep raw stream lines at `trace` if logged at all.

### `anyhow::Context` at fallible I/O boundaries
**Source:** `src/config.rs:60-87`
**Apply to:** spawn setup, fixture loading, any new `git_ops` helper returning `Result`

```rust
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config JSON: {}", path.display()))?;
```

`.with_context(|| format!("Failed to X: {}", path.display()))` — lazy closure, verb-first
message, `Path::display()`. Used uniformly across `config.rs`, `registry.rs`,
`project_creator.rs`.

### `spawn_blocking` for synchronous fs/process work
**Source:** `src/app.rs:252-256`, `:284-296`
**Apply to:** the before/after `parse_project_state` + git snapshots (D-11)

```rust
                        tokio::task::spawn_blocking(move || {
                            let sessions = crate::session_detector::detect_sessions();
                            let _ = tx.send(Action::SessionsDetected { sessions });
                        });
```

Clone the `tx`, move owned values in, `let _ = tx.send(...)` (send failure is not an error
at shutdown). Both existing sites use `let _ =`; keep that.

### `#[cfg(test)] mod tests` in-file
**Source:** `src/watcher.rs:92-117`, `src/session_detector.rs:111-133`
**Apply to:** `stream_json.rs`, `outcome.rs`, the gate, `pump()`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_project_root_from_state_md() {
```

18 source files use this; it is the dominant convention and the right home for everything
that does not need a real child process. Load transcripts with
`include_str!("../../tests/fixtures/transcripts/05-….ndjson")` — compile-time, no I/O.

**Clippy trap:** `src/state_reader/mod.rs` already carries one of the 5 pre-existing
`--all-targets` lints for *items-after-test-module*. Put `#[cfg(test)] mod tests` **last**
in every new file, or the count grows.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/executor/claude.rs` — pipe topology, supervisor, teardown | service | streaming/duplex | No long-lived child process exists in the tree. Every `Command` site is one-shot `output()` or blocking `status()`. No `tokio::select!`, no `Stdio::piped()`, no signal handling. **Use RESEARCH.md "Architecture Patterns → Pattern 1/2", the verified `process-wrap` 9.1.0 API surface, and the four documented gotchas.** |
| `src/executor/mod.rs` — the `Executor` trait | trait | — | No trait with async methods exists; `Screen` (`src/ui/screens/mod.rs`) is the only `dyn` trait and is fully synchronous. RESEARCH "The `Executor` Trait Revival" + the v1.2 design doc are the source. Note the boxed-future vs `async-trait` decision (Assumption A5) is a real Cargo consequence. |
| `tests/fixtures/transcripts/*.ndjson` | fixture | — | `tests/fixtures/` does not exist. Greenfield; redaction rules are D-25 + RESEARCH "Captured Transcripts". |
| `tests/fixtures/fake-claude.sh` | fixture (exec) | — | No shell fixtures, no executable test assets in the repo. Greenfield; needs the exec bit preserved in git (`git update-index --chmod=+x`). |

---

## Metadata

**Analog search scope:** `src/` (all 26 files, sizes checked), `tests/` (2 files),
`Cargo.toml`, `README.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `docs/*.md`
**Files scanned:** 36; read in full or targeted-range: `main.rs`, `event.rs`, `action.rs`,
`error.rs`, `session_detector.rs`, `terminal_switch.rs`, `watcher.rs`, `config.rs`,
`lib.rs`, `Cargo.toml`, `git_ops.rs:1-210`, `state_reader/mod.rs:1-150`,
`app.rs:1-60,100-162,240-305`, `ui/screens/mod.rs:100-182`, `archive.rs:1-80`,
`config_json.rs:1-80`, `disk_status.rs` (derives), `tests/*.rs`
**Verification greps:** `tokio::select` (0 hits), `mpsc::channel` (0 hits),
`#[serde(tag` (0 hits), `#[serde(other` (0 hits), `#[serde(rename` (1 hit),
`1.85` in docs (8 project sites + 1 citation)
**Pattern extraction date:** 2026-07-29
