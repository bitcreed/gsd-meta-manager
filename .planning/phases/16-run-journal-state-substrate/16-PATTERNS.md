# Phase 16: Run Journal & State Substrate - Pattern Map

**Mapped:** 2026-07-29
**Files analyzed:** 12 (4 new source, 2 new tests, 6 modified)
**Analogs found:** 12 / 12 (all exact or role-match — this phase adds no unprecedented shape)

Every excerpt below was read directly from the current tree. Line numbers are as of this date.

---

## File Classification

| New/Modified File | New? | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `src/journal/mod.rs` | new | module root / domain model | event-driven (schema + pure fns) | `src/executor/mod.rs` | exact |
| `src/journal/writer.rs` | new | service (single-writer task) | streaming append + file-I/O | `src/executor/claude.rs` writer task + `src/config.rs::save_config` | role-match (two analogs, split by concern) |
| `src/journal/reader.rs` | new | service (tolerant parser) | file-I/O / streaming tail | `src/executor/stream_json.rs` (`Envelope` + `parse_line`) | exact (parse shape) / none (byte-offset tail — RESEARCH §2 is the source) |
| `src/journal/redact.rs` | new | utility (pure transform) | transform | `src/executor/mod.rs::DrivableProject` (the newtype-enforces-a-seam idiom) | role-match (idiom only; no regex-redactor exists yet) |
| `src/action.rs` | mod | model (message enum) | event-driven | itself — `ProjectStateLoaded` / `ArchiveLoaded` | exact |
| `src/watcher.rs` | mod | adapter (fs → Action) | event-driven | itself — `extract_project_root` + the debounced callback | exact |
| `src/app.rs` | mod | controller (reducer) | request-response | itself — `FileChanged` arm + `apply_exec_event` | exact |
| `src/ui/screens/mod.rs` | mod | state container | — | itself — `run_states` / `last_refresh` / `archive_cache` | exact |
| `src/main_loop.rs` | mod (maybe) | config constants | — | itself — `EXEC_BATCH` / `EXEC_CHANNEL_CAPACITY` | exact |
| `src/lib.rs` | mod | module registry | — | itself | exact |
| `tests/journal_crash.rs` | new | integration test | process-lifecycle | `tests/executor_lifecycle.rs` | exact (header + fixture-path idiom); the child is a re-exec, not a shell fixture |
| `tests/journal_gitignore.rs` | new | integration test | file-I/O | `tests/executor_lifecycle.rs` (shape) + `tempfile` | role-match |

---

## Pattern Assignments

### `src/journal/mod.rs` (module root, domain model)

**Analog:** `src/executor/mod.rs` — same job: a module root that carries the whole domain
surface later phases build against, with submodules that divide cleanly.

**Module-root doc-comment pattern** (`src/executor/mod.rs:1-26`). Note the shape: what the
module root is *for*, then the numbered *facts that govern every type below*, then a
`#[cfg(unix)]` justification written as rationale not as a note. Copy this structure:

```rust
//! Drive a `claude` CLI process over the duplex `stream-json` protocol.
//!
//! This module root carries the whole domain surface every later plan builds
//! against, so nothing downstream has to reopen it (D-21). The four submodules
//! divide cleanly: `claude` owns the process transport, `gate` owns the
//! `system/init` capability check, `outcome` owns run-outcome derivation, and
//! `stream_json` owns the wire model.
//!
//! Two facts govern every type below, both measured directly against CLI
//! 2.1.220 rather than inferred:
//!
//! 1. **`type:"result"` is a TURN boundary, not a run terminator (D-29).** ...

#[cfg(unix)]
pub mod claude;
pub mod gate;
pub mod outcome;
pub mod stream_json;
```

`src/journal/mod.rs` takes the same four-submodule declaration block
(`writer`, `reader`, `redact`) with no `cfg` gate — nothing in the journal is Unix-only
(only `tests/journal_crash.rs` is, via `#![cfg(unix)]`).

**Enum-with-rationale-per-variant pattern** (`src/executor/mod.rs:418-474`) — the model for
`JournalEvent`. Each variant's doc says *why the variant exists*, and the forward-compat vs
malformed distinction is spelled out on the variants themselves:

```rust
/// One observed thing on the stream.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// The first `system/init` passed the gate. Emitted exactly once per run.
    SessionStarted { session_id: String, capabilities: Vec<String>, /* ... */ },
    /// A parsed message of a known type.
    Message(Box<StreamMessage>),
    /// A well-formed line of a type no version we know emits. Forward-compat:
    /// carried, never fatal. Deliberately distinct from `Unparseable` (D-09).
    Unknown { raw: String },
    /// A line that did not parse — a torn write or invalid JSON. A real
    /// diagnostic, but still never fails the run.
    Unparseable { raw: String, error: String },
    /// A line exceeded `MAX_LINE_BYTES` and was not parsed. The run continues:
    /// an oversize line from the subprocess must not be able to exhaust memory
    /// or abort a run (T-15-07, D-05).
    LineTruncated { bytes: usize, prefix: String },
    // ...
}
```

`LineTruncated { bytes, prefix }` is the direct precedent RESEARCH §10 names for the
per-event payload cap marker — mirror its field shape.

**`Box` for large variants** (`src/executor/mod.rs:464-466`, `src/action.rs:19-23`) — carried
into `RunState::Finished(Box<RunOutcome>)` at `mod.rs:598`. Applies to any `JournalEvent`
variant that grows.

**Enforce-with-a-type idiom** (`src/executor/mod.rs:109-140`) — this is D-22's stated model
for `Redacted`. Read the doc comment carefully; the *argument* is the pattern, not just the
struct:

```rust
/// Proof that the user designated a project as drivable.
///
/// This is the D-23 capability token, and it is a *type* on purpose: the
/// compiler, not a code review, is what enforces the single spawn seam.
/// [`Executor::start`] takes this and never a bare path and never a `bool`,
/// so a call site cannot spawn an agent against a directory the user never
/// opted in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivableProject {
    alias: String,   // private fields — no construction outside the module
    root: PathBuf,
}

impl DrivableProject {
    /// Construct a token without a validated opt-in record.
    ///
    /// **Test and development only.** ... the name is deliberately loud.
    pub fn for_testing(alias: impl Into<String>, root: impl Into<PathBuf>) -> Self { /* ... */ }
    pub fn alias(&self) -> &str { &self.alias }
}
```

Load-bearing details to copy for `RedactedLine`: **private fields**, an accessor rather than
a `Deref`, and no `From`/`Display`. RESEARCH §5.4 adds: `as_line()` is `pub(crate)`.
Note the *deviation* from `DrivableProject`: `RedactedLine` must have **no** `for_testing`
escape hatch — that would reopen exactly the seam D-22 closes.

**Where classification lives:** RESEARCH Open Question 1 recommends `journal/mod.rs` with
`watcher.rs` importing it (dependency direction journal→nothing). Copy the pure-function +
in-file-test shape from `watcher.rs:20-35` (below) regardless of which file it lands in.

---

### `src/journal/writer.rs` (service, streaming append + file-I/O)

**Analog A — atomic `run.json`:** `src/config.rs:71-87` (`save_config`). D-05 names this
exactly. Copy verbatim in shape, including the `anyhow::Context` on every fallible step:

```rust
/// Save config atomically using tempfile + rename.
pub fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir).context("Failed to create temp file for config")?;
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    tmp.write_all(json.as_bytes())
        .context("Failed to write config to temp file")?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;

    Ok(())
}
```

Imports to copy (`src/config.rs:1-6`):

```rust
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
```

Two deliberate deviations, both already decided:
- `to_string_pretty` is fine for `run.json` (it is a document, not NDJSON) but **never** for
  `journal.jsonl` lines (RESEARCH §9.1 — pretty output breaks NDJSON framing).
- Add `set_permissions(0o644)` before `persist` under `cfg(unix)` — RESEARCH §1.4 measured
  `persist` preserving 0600.

**Analog B — writing into a third-party repo's `.planning/meta-manager/`:**
`src/state_reader/queue_md.rs:170-230`. The **path-helper + rationale doc-comment** is the
part to copy; the `.tmp` + `rename` write is explicitly the part **not** to copy (D-05).

```rust
/// The queue lives in an app-namespaced subdirectory (`meta-manager/`) rather
/// than at the `.planning/` root to satisfy GSD 1.8.0's `/gsd-health` W019
/// check. That check flags any non-canonical `*.md` FILE at the `.planning/`
/// root ... but skips subdirectories entirely ...
/// Placing the queue under a subdir therefore produces zero health findings.
fn queue_paths(planning_dir: &Path) -> (PathBuf, PathBuf) {
    let canonical = planning_dir.join("meta-manager").join("QUEUE.md");
    let legacy = planning_dir.join("QUEUE.md");
    (canonical, legacy)
}
```

```rust
    let meta_dir = canonical
        .parent()
        .expect("canonical queue path always has a parent");
    std::fs::create_dir_all(meta_dir)?;
    // NOT THIS PART (D-05: fixed .tmp name collides between writers):
    // let tmp_path = meta_dir.join("QUEUE.md.tmp");
    // std::fs::write(&tmp_path, &content)?;
    // std::fs::rename(&tmp_path, &canonical)?;
```

Journal equivalent: a `run_paths(planning_dir, run_id) -> RunPaths { dir, run_json, journal,
gitignore, active }` helper carrying the D-01/D-07/D-08 rationale in its doc comment.

**Analog C — the single-writer task with an ordered command channel:**
`src/executor/mod.rs:306-318`. D-29's model. Note the enum-of-commands shape and that the
doc explains *why the task is separate*:

```rust
/// A command for the dedicated stdin writer task.
///
/// The writer is a task of its own, never the task awaiting process exit: the
/// two-pipe deadlock is live here, because the child blocks writing stdout
/// while a parent that owns both blocks writing stdin (D-04).
#[derive(Debug)]
pub(crate) enum WriterCommand {
    /// Write one NDJSON line plus its newline.
    Line(String),
    /// Drop the stdin handle. EOF means "no more input", **not** "stop": the
    /// CLI drains its queued turns, finishes them, and exits 0.
    Close,
}
```

The journal writer's command enum carries `RedactedLine`, never `String` — that is the D-22
seam made concrete at the channel boundary.

**Tracing discipline** (`src/executor/claude.rs:1248-1266`) — D-28's precedent, and it names
this phase:

```rust
/// The warning carries the running **count** and the channel capacity and
/// nothing else — no event, no raw line, no message body. Redact-at-capture
/// does not land until Phase 16, so anything logged here stays unredacted
/// forever (T-15-53).
```

Phase 16's journal modules must state the same rule in their own docs and honour it.
`tracing::warn!` for non-fatal degradation — the idiom is at `src/watcher.rs:63, 78, 86`.

---

### `src/journal/reader.rs` (service, tolerant parse + byte-offset tail)

**Analog:** `src/executor/stream_json.rs` — the tolerant-parsing shape D-30 mirrors, and the
module that already documents the `#[serde(other)]` constraint RESEARCH §9 re-verified.

**Module doc stating the tolerance contract** (`stream_json.rs:1-32`), including the
grep-as-mechanical-guard rule that D-30/§9.1 says to repeat in `src/journal/`:

```rust
//! Parsing is **tolerant by construction and never fails a run** (D-09). Every
//! line arriving here is untrusted input from a process that itself consumed
//! untrusted repository content, so:
//!
//! - Serde's strict unknown-field rejection attribute is never opted into
//!   anywhere in this file, and its absence is grepped for as a mechanical
//!   guard. ...
//! - Unknown message `type`s and unknown `subtype`s absorb into catch-all
//!   variants and are carried, never fatal.
//! - `subtype` and `terminal_reason` are plain `String`s, never enums. This CLI
//!   shipped three new values on one version line; a typed enum would need a
//!   catch-all on each and would still lose the actual string.
//! - No panicking accessor exists in the non-test region of this file.
//!
//! `#[serde(other)]` is only accepted on a **unit** variant of an
//! internally-tagged enum, so the enum itself cannot carry the raw body. The
//! raw line is therefore preserved outside the enum, by [`Envelope`], ...
```

**The `Envelope` + `parse_line` pattern** (`stream_json.rs:293-329`) — RESEARCH §9's
"Option A", to be copied structurally as `JournalRecord`:

```rust
/// What the reader task yields for one observed line.
///
/// Parsing **never** fails a run: a line either parsed into a carried message
/// or is reported as unparseable, and both keep the run going (D-09).
#[derive(Debug, Clone)]
pub enum Envelope {
    /// The line parsed. `msg` may still be a forward-compat `Unknown`.
    Parsed { raw: String, msg: StreamMessage },
    /// The line did not parse — a torn write, a truncation, or invalid JSON.
    /// Distinct from `StreamMessage::Unknown`, which is a *known-good* line of
    /// an unknown type.
    Unparseable { raw: String, error: String },
}

/// Parse one NDJSON line. Never panics, never fails a run.
pub fn parse_line(raw: &str) -> Envelope {
    match serde_json::from_str::<StreamMessage>(raw) {
        Ok(msg) => Envelope::Parsed { raw: raw.to_string(), msg },
        Err(e) => Envelope::Unparseable { raw: raw.to_string(), error: e.to_string() },
    }
}
```

**The internally-tagged enum** (`stream_json.rs:36-60`) for the writer's emission surface:

```rust
/// One message off the `stream-json` stream, dispatched on the `type` field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum StreamMessage {
    System(SystemMessage),
    /// `result` — closes a **turn**, not the run (D-29). Boxed because the
    /// payload dwarfs every other variant (`clippy::large_enum_variant` is a
    /// `-D warnings` build gate here, per the `src/action.rs` precedent).
    Result(Box<ResultMessage>),
    /// `rate_limit_event`. The payload is carried unmodelled, following the
    /// escape-hatch idiom in `state_reader/config_json.rs` ...
    RateLimitEvent(serde_json::Value),
    #[serde(other)]
    // ...
}
```

Writer side becomes `#[serde(tag = "kind")]`, **struct variants only** (§9.1: tuple variants
are a compile error, newtype-around-a-scalar a runtime error).

**No analog for the byte-offset tail.** `BufReader::lines()` is what the repo does elsewhere
and RESEARCH §2.1 demonstrates it is *wrong* here. Use the compiling sketch at RESEARCH
§2.2 (`tail_lines` / `TailCursor` / `TailRead`) verbatim — it is the closest thing to an
analog and it was executed against a real concurrently-appended file.

---

### `src/journal/redact.rs` (utility, pure transform)

**No existing redactor in `src/`.** Use RESEARCH §4.2 (the verified `PARTS` table +
`LazyLock` alternation) and §5.1 (`redact_value` tree walk) as the source — both are
executed code, not sketches.

**Repo-side analogs to honour:**

- **The `Redacted` newtype** takes its shape from `DrivableProject` (excerpt above,
  `src/executor/mod.rs:109-140`) — private field, named accessor, loud docs.
- **The documented-honest-limit convention.** The precedent is
  `tests/fixtures/transcripts/README.md` "Redaction record (D-25)", which records the WR-15
  miss (slash form matched, dash-encoded form missed in seven of eight fixtures; commit
  `0a9b6d8`) and concludes *"Any future redaction sweep must scan both encodings."*
  D-25 requires `redact.rs`'s module docs to carry the equivalent statement: a pattern
  redactor cannot catch an arbitrary high-entropy secret, and a bare username outside a path
  context is deliberately not redacted (RESEARCH §4.5).
- **`dirs` for the runtime home prefix** — already used at `src/config.rs:44`
  (`dirs::config_dir()`); `dirs::home_dir()` is the same crate, no new dependency.

---

### `src/action.rs` (model, event-driven)

**Analog:** itself. The whole file is 45 lines; the two variants that matter:

```rust
#[derive(Debug, Clone)]
pub enum Action {
    FileChanged {
        project_path: std::path::PathBuf,
    },
    ProjectStateLoaded {
        alias: String,
        // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
        state: Box<ProjectState>,
    },
    ArchiveLoaded {
        alias: String,
        milestone: String,
        data: crate::archive::MilestoneArchive,
    },
}
```

Changes: add `changed_path: std::path::PathBuf` to `FileChanged` (D-09), and add the new
`DriverJournalAppended { alias, run_id, events: Vec<_>, cursor: u64 }` variant. RESEARCH §8.3
measured both as inside the `large_enum_variant` budget (enum is 120 B; threshold is a
200 B *difference*) — carry the events in a `Vec`, never by value. The one-line
`// Boxed: ...` comment above is the convention for recording why indirection exists.

---

### `src/watcher.rs` (adapter, event-driven)

**Analog:** itself. All three touched regions:

**Pure path function + its exhaustive in-file tests** (`watcher.rs:20-35, 92-117`) — D-11
says `classify_change` is tested "the way `extract_project_root` already is":

```rust
/// Walk up from a changed file path to find the parent of `.planning/`.
/// Returns the project root (the directory containing `.planning/`).
fn extract_project_root(path: &Path) -> Option<&Path> {
    let mut current = path;
    loop {
        if let Some(name) = current.file_name() {
            if name == ".planning" { return current.parent(); }
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}
```

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_project_root_from_nested() {
        let path = PathBuf::from("/home/user/projects/myapp/.planning/phases/01/plan.md");
        let root = extract_project_root(&path);
        assert_eq!(root, Some(Path::new("/home/user/projects/myapp")));
    }

    #[test]
    fn test_extract_project_root_no_planning() {
        let path = PathBuf::from("/home/user/projects/myapp/src/main.rs");
        let root = extract_project_root(&path);
        assert_eq!(root, None);
    }
}
```

Note the naming convention: `test_<fn>_<case>`, one assertion, no fixtures. Extend this
module rather than adding a new one.

**The debounced callback — the D-09 + D-10 change site** (`watcher.rs:41-68`):

```rust
let debouncer = new_debouncer(
    Duration::from_millis(200),
    None,
    move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                // Collect unique project roots from all changed paths
                let mut seen = std::collections::HashSet::new();
                for event in &events {
                    for path in &event.paths {
                        if let Some(root) = extract_project_root(path) {
                            if seen.insert(root.to_path_buf()) {          // <-- D-10: per-root
                                let _ = tx.send(Action::FileChanged {
                                    project_path: root.to_path_buf(),
                                    // <-- D-09: `path` is already in scope here
                                });
                            }
                        }
                    }
                }
            }
            Err(errors) => {
                for error in errors {
                    tracing::warn!("File watcher error: {}", error);
                }
            }
        }
    },
)?;
```

D-10 changes `seen` to a `HashSet<(PathBuf, ChangeKind-discriminant)>`. RESEARCH Pitfall 6
requires a test that puts the journal path **first** in the batch.

**`watch()` is already recursive** (`watcher.rs:74-81`) — `RecursiveMode::Recursive`, so no
new registration is needed for `.planning/meta-manager/runs/`. The `map_err` +
`tracing::warn!` + `anyhow::anyhow!` pairing there is the error convention.

---

### `src/app.rs` (controller, request-response)

**Analog:** itself, two arms.

**The `FileChanged` arm — what classification forks** (`app.rs:316-358`):

```rust
Action::FileChanged { project_path } => {
    // Find the alias matching this project path
    let alias = self.ctx.config.projects.iter()
        .find(|(_, proj)| proj.path == project_path)
        .map(|(alias, _)| alias.clone());

    if let Some(alias) = alias {
        // Dedup: skip if last refresh was less than 500ms ago
        let now = std::time::Instant::now();
        if let Some(last) = self.ctx.last_refresh.get(&alias) {
            if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                return;
            }
        }

        // Use spawn_blocking for async file I/O
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
            self.ctx.last_refresh.insert(alias, now);
        }

        // Auto-start watcher if not yet watching
        let planning_dir_check = project_path.join(".planning");
        if planning_dir_check.is_dir() {
            if let Some(ref mut watcher) = self.ctx.watcher {
                let _ = watcher.watch(&planning_dir_check);
            }
        }
    }
}
```

D-14: classification must happen **before** the `last_refresh` check, and the driver arm
returns without touching it. RESEARCH §7.3 gives the recommended extraction into
`schedule_reparse()` plus the `reparse_dispatches: u64` counter.

**The `spawn_blocking` + result-as-`Action` idiom, second instance** (`app.rs:301-308`) —
confirms it is a convention, not a one-off:

```rust
if let Some(ref tx) = self.ctx.event_tx {
    let tx: UnboundedSender<Action> = tx.clone();
    tokio::task::spawn_blocking(move || {
        let sessions = crate::session_detector::detect_sessions();
        let _ = tx.send(Action::SessionsDetected { sessions });
    });
}
```

The tail handler for `DriverJournalAppended` copies this exactly.

**The sibling-map reducer** (`app.rs:177-209`, `apply_exec_event`) — the closest analog for a
`DriverJournalAppended` handler. Note it touches only the sibling map and `needs_redraw`,
and the doc comment records what it deliberately does *not* do:

```rust
/// Apply one executor event to the per-alias driver state (D-17, D-19).
///
/// This is the executor arm's whole handler. It touches only the sibling
/// map on `AppContext` and the redraw flag — never `ProjectState`, whose
/// derived equality suppresses status-bar spam.
///
/// Note what it deliberately does *not* do: `ExecutionEvent::Exited` moves
/// the alias to `Stopping`, **not** to `Finished(outcome)`. ...
pub fn apply_exec_event(&mut self, event: crate::main_loop::ExecEvent) {
    let crate::main_loop::ExecEvent { alias, event } = event;
    let state = self.ctx.run_states.entry(alias).or_default();
    match event { /* ... */ }
    self.needs_redraw = true;
}
```

**The equality D-18 must not break** (`app.rs:359-374`):

```rust
let changed = match self.ctx.project_states.get(&alias) {
    Some(old_state) => {
        self.ctx.change_tracker.detect_changes(&alias, old_state, &state);
        *old_state != *state
    }
    None => true,
};
```

**Test constructor** (`app.rs:130-132`) — `App::new_for_test()` sets `event_tx: None`
(`app.rs:152`). RESEARCH §7.3's OBS-06 test replaces it with a real sender.

---

### `src/ui/screens/mod.rs` (state container)

**Analog:** itself — `run_states` at `:138-147` is both the map D-19 extends and the doc-comment
template for the new offset map:

```rust
    /// Per-alias live driver state (D-19).
    ///
    /// A **sibling map**, shaped exactly like `last_refresh` and
    /// `archive_cache` above. Driver state deliberately does NOT live on
    /// `ProjectState`: that type derives `PartialEq`, and `app.rs` uses the
    /// derived equality to suppress the "Updated: {alias}" status message.
    /// Driver state changes every few seconds — one 68-second spike turn
    /// emitted 22 `thinking_tokens` events — so putting it there would flood
    /// the status bar for an entire multi-hour run.
    pub run_states: HashMap<String, RunState>,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    // ...
    pub archive_cache: HashMap<String, crate::archive::MilestoneArchive>,
```

The `exec_tx` doc at `:126-137` is the other template — a field whose *invariants* are spelled
out as bullets ("Two things about this field are load-bearing, and neither is stylistic").
Use that shape for `reparse_dispatches` (RESEARCH §7.3 supplies the wording) and for the
`(alias, run_id) -> TailCursor` map.

**Every new field must also be initialised in `App::from_config`** (`app.rs:140-163`) — the
struct literal there is exhaustive and is where the compiler will point.

---

### `src/main_loop.rs` (config constants)

**Analog:** itself, `:41-64`. D-31/D-32's constants copy this exact doc shape — value,
rationale, and an explicit admission when there is no tuning data:

```rust
/// The maximum number of executor events a single [`pump`] iteration applies.
///
/// This bound is what stops a burst starving a redraw: once `EXEC_BATCH` events
/// have been applied, control returns to the loop head, which draws a frame and
/// re-polls the input arm.
///
/// 64 is a defensible starting value with no tuning data behind it (RESEARCH
/// assumption A6, D-13). It is a named constant so tuning is a one-line change.
pub const EXEC_BATCH: usize = 64;

/// Capacity of the process-lifetime executor channel.
///
/// Generous on purpose. The channel is bounded so that a render loop blocked in
/// the editor shell-out applies **backpressure** to the reader task ...
pub const EXEC_CHANNEL_CAPACITY: usize = 8192;
```

Apply to `MAX_EVENT_PAYLOAD_BYTES` (8 KiB), `MAX_RUN_JOURNAL_BYTES` (64 MiB), `RETAIN_RUNS`
(10), `MAX_TAIL_BYTES` (4 MiB) — RESEARCH §10 supplies the anchoring evidence for each.

**`main_loop.rs`'s test module (`:199-221`) is also the precedent RESEARCH §7.6 cites** for
*declining* a wall-clock assertion. Read it before writing any timing test.

**The dropped-event count D-33 journals** originates in `pump`'s bounded drain
(`main_loop.rs:156-160` and `claude.rs:1248-1266`).

---

### `src/lib.rs` (module registry)

Alphabetical `pub mod` list, 17 lines. Insert `pub mod journal;` between `executor` and
`main_loop`.

---

### `tests/journal_crash.rs` / `tests/journal_gitignore.rs` (integration tests)

**Analog:** `tests/executor_lifecycle.rs:1-46`. The header block is the pattern — a banner
comment saying *why this test needs to be an integration test*, `#![cfg(unix)]`, then
`concat!(env!("CARGO_MANIFEST_DIR"), "…")` constants:

```rust
// ============================================================================
// Process lifecycle: the two deadlines (D-13) and the group teardown (D-14)
//
// Everything here needs a real child process, which is why it is not in-source
// with the parsing tests. Three shell stand-ins supply the behaviours a
// transcript cannot express: going silent, spawning a grandchild, and ignoring
// the terminate signal.
//
// Unix-only by construction. ...
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::time::{Duration, Instant};

use gsd_meta_manager::executor::claude::ClaudeExecutor;
use gsd_meta_manager::executor::{DrivableProject, ExecutionEvent, /* ... */};
use tempfile::TempDir;

const FAKE_SLOW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-slow.sh"
);
```

**Named helper functions with rationale docs** (`:48-99`) — the convention for test-local
helpers, including the process-liveness probe:

```rust
/// Whether `pid` still exists, via the zero signal.
///
/// The shell builtin rather than `/bin/kill` so this works wherever `sh` does,
/// and `pid` is an integer so the interpolation cannot carry anything else.
fn alive(pid: u32) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
```

**Deliberate deviation from the analog** (RESEARCH §6.3): do **not** use a
`tests/fixtures/fake-*.sh` stand-in or `process-wrap`'s `ProcessGroup`. The child must run
the real `JournalWriter`, which a shell script cannot; use the `current_exe()` re-exec
harness at RESEARCH §6.2 with `std::process::Child::kill()` (already SIGKILL on unix,
verified `unix_wait_status(9)`).

---

## Shared Patterns

### `#[cfg(test)] mod tests` at the bottom of every module
**Source:** `src/watcher.rs:92-117`, `src/config.rs` (none — a file with no logic has none),
`src/executor/stream_json.rs`, `src/main_loop.rs:199+`
**Apply to:** all four `src/journal/*.rs` files, plus the extended `watcher.rs` and `app.rs`
modules.
Convention: `use super::*;` first line, `test_<subject>_<case>` naming in `watcher.rs`,
sentence-style names (`a_sigkilled_writer_leaves_every_flushed_line_readable`) in the newer
executor-era modules. Prefer the newer style for `src/journal/`.

### Doc comments state the rationale, not the what
**Source:** `src/executor/mod.rs:1-26`, `:109-122`, `:579-585`; `src/ui/screens/mod.rs:126-147`;
`src/main_loop.rs:41-64`, `:110-129`; `src/state_reader/queue_md.rs:170-176`
**Apply to:** every new public item.
The house style: bold the load-bearing claim (`**Test and development only.**`), cite the
decision id (`(D-19)`), and record what the code deliberately does *not* do. Several docs name
the alternative that was rejected and why — copy that habit; the planner's acceptance criteria
can then point at a doc comment.

### `anyhow::Context` on fallible IO
**Source:** `src/config.rs:60-87`
**Apply to:** `writer.rs` (`run.json`, `create_dir_all`, `.gitignore`), `reader.rs` where it
returns `anyhow::Result`.
```rust
std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read config: {}", path.display()))?;
```
Note `reader.rs`'s tail returns `io::Result` in the RESEARCH sketch — that is fine at the
leaf; add `Context` at the call boundary.

### `tracing::warn!` for non-fatal degradation, never `error!` or a panic
**Source:** `src/watcher.rs:63, 78, 86`
```rust
self.debouncer.watch(planning_dir, RecursiveMode::Recursive).map_err(|e| {
    tracing::warn!("Failed to watch {}: {}", planning_dir.display(), e);
    anyhow::anyhow!("Failed to watch {}: {}", planning_dir.display(), e)
})
```
**Apply to:** the D-08 parent-`.gitignore` diagnostic (RESEARCH §1.3), `restarted` /
`skipped_oversize` / `seq`-gap diagnostics, and the retention pruner.
**Constraint (D-28):** in `src/journal/**` these must carry counts/paths-already-redacted or
no event content at all.

### `Box` large enum payloads with a one-line reason
**Source:** `src/action.rs:21-22`, `src/executor/mod.rs:464-466`, `:598`,
`src/executor/stream_json.rs:47-50`
```rust
// Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
state: Box<ProjectState>,
```
**Apply to:** any `JournalEvent` or `Action` variant that grows. RESEARCH §8 measured the
exact budget (fires at a >200 B *difference*; `Action` is 120 B today).

### Sibling maps on `AppContext`, never fields on `ProjectState`
**Source:** `src/ui/screens/mod.rs:138-155`, rationale duplicated at
`src/executor/mod.rs:579-585`
**Apply to:** the tail-cursor map and `reparse_dispatches`. Also: every new field needs a line
in the `AppContext` literal at `src/app.rs:140-163`.

### `serde` tolerance: no `deny_unknown_fields`, plain `String` for open vocabularies
**Source:** `src/executor/stream_json.rs:6-18`
**Apply to:** all of `src/journal/`. §9.1 says to repeat the "absence is grepped for as a
mechanical guard" sentence in the journal's module docs so the same check applies.

---

## No Analog Found

| File / concern | Role | Data Flow | Reason | What to use instead |
|---|---|---|---|---|
| `journal/reader.rs` byte-offset tail | service | streaming file-I/O | Nothing in the repo reads a growing file from a stored offset; every existing reader is `read_to_string` or `BufReader::lines()`, and RESEARCH §2.1 demonstrates `lines()` is actively wrong here | RESEARCH §2.2 `tail_lines` sketch (executed) |
| `journal/redact.rs` pattern set | utility | transform | No redactor exists in `src/`; the only precedent is the *fixture* redaction recorded in `tests/fixtures/transcripts/README.md` | RESEARCH §4.2 `PARTS` + §5.1 `redact_value` (both executed, 27 cases, 0 idempotence failures) |
| `.gitignore` emission + verification | config | file-I/O | The repo has never written a `.gitignore` into a third-party repo; root `.gitignore` is two hand-written lines | RESEARCH §1 (verified transcript, plus the two ways to verify it wrong) |
| Retention pruning by directory listing | service | batch | No pruner exists anywhere in the tree | D-32 + RESEARCH Open Question 2 (listing is authoritative, `active` file is a hint) |
| `current_exe()` re-exec test harness | test | process-lifecycle | `tests/executor_lifecycle.rs` spawns *shell fixtures*, never itself | RESEARCH §6.2 (built and run 8× this session) |

---

## Notes for the Planner

1. **`src/journal/mod.rs` should be written first and read like `src/executor/mod.rs`** — the
   domain surface Phases 17/18/20 build against. The four-submodule split is already decided
   (D-35) and mirrors an existing, working precedent.
2. **The redactor cannot trail the writer.** CONTEXT's own wave note: "(2) should not trail (1)
   by much — the writer must not exist unredacted even for one commit." The cheapest way to
   honour this mechanically is to give the writer's command channel a `RedactedLine` payload
   from its very first commit, so the seam is closed by construction rather than by sequencing.
3. **Three excerpts here are anti-patterns, clearly marked:** `queue_md.rs`'s `.tmp` + `rename`
   (use `config.rs` instead, D-05), `BufReader::lines()` for the tail (RESEARCH §2.1), and
   `tests/fixtures/fake-*.sh` for the crash child (RESEARCH §6.3). Each is included because a
   pattern-matching implementer would otherwise reach for it.
4. **`cargo clippy --all-targets` has exactly 5 pre-existing lints** in `browser.rs` (×3),
   `project_creator.rs` (×1), `state_reader/mod.rs` (×1). None of those files appear in this
   pattern map, and none should be opened.

---

## Metadata

**Analog search scope:** `src/` (all 17 root modules + `executor/`, `ui/screens/`,
`state_reader/`), `tests/`, `Cargo.toml`
**Files read this pass:** 11 (`watcher.rs`, `action.rs`, `config.rs`, `lib.rs`,
`ui/screens/mod.rs` in full; `app.rs`, `executor/mod.rs`, `executor/stream_json.rs`,
`executor/claude.rs`, `state_reader/queue_md.rs`, `main_loop.rs`, `tests/executor_lifecycle.rs`
in targeted ranges)
**Pattern extraction date:** 2026-07-29
