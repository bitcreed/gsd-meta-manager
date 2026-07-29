# Architecture Research

**Domain:** Autonomous LLM driver integrated into an existing Rust/ratatui TEA dashboard (v2.0 Autonomous Orchestration)
**Researched:** 2026-07-28
**Confidence:** HIGH for all claims about the existing codebase (every one cites a `file:line` I read) and for Claude CLI capabilities (verified by running `claude --help` / `claude agents --json` against the locally installed **v2.1.220**). MEDIUM/LOW for ecosystem-pattern claims sourced from web search — tagged inline.

> Supersedes the v1.2-era `ARCHITECTURE.md` (commit `0912a54`). Recover the old one with
> `git show 0912a54:.planning/research/ARCHITECTURE.md` if the archive browser needs it.

---

## 0. Executive Answer

Six questions were asked. Short answers first; evidence and detail follow.

| Question | Answer |
|----------|--------|
| Where does the driver loop live? | **Detached child of the same binary** (`gsd-meta-manager drive <alias>`), re-parented away from the TUI. Not an in-process tokio task (dies with the TUI), not a system daemon (portability). Re-attachment is by reading the on-disk run journal + a PID liveness probe that reuses `session_detector.rs`'s `/proc` technique. |
| Shape of "decide next command"? | **Hybrid, deterministic-first.** A pure function `decide(&ProjectState, &RunGoal, &RunHistory) -> Decision` implemented as a rule-based state machine over D-R-P-E-V. The LLM is called exactly twice per run in the common case: once to decompose the user's goal into a bounded plan, and once *per ambiguity* when the router returns `Ambiguous`. This matches the production consensus. |
| Long-lived driver state? | **Append-only JSONL run journal** under `.planning/meta-manager/runs/<run-id>/`. It gets live updates for free from the existing watcher — `extract_project_root` already walks arbitrarily-deep `.planning/` paths (`watcher.rs:22-35`, test at `:105-110`). But *free* is not *free of cost*: three concrete changes in `app.rs` are required or the journal will DoS the state reader. Detailed in §4.4. |
| New vs modified components? | 13 new modules, 11 modified files. Explicit tables in §8. |
| Container behind the same interface? | Yes — and **not as a second `Executor` impl**. Container is an `ExecutionTarget` enum *inside* `ClaudeExecutor`: it changes the argv prefix and the host↔container path mapping, nothing else. |
| Does the v1.2 `Executor` trait still fit? | **Mostly yes — with one required signature change.** The process-based model is still right, but its fire-and-forget `start/cancel/is_running` shape predates duplex streaming. It needs `send()` and `interrupt()`. Full verdict in §7.1. |

---

## 1. What the Existing Architecture Actually Is

Everything in this section is read from source, not assumed.

### 1.1 The loop

`main.rs:134-205` is the whole event loop. It is **not** a `tokio::select!` — it is a single `rx.recv().await` over one unbounded mpsc channel:

```
main.rs:151   if let Some(action) = rx.recv().await { app.update(action); }
```

The multiplexing happens *upstream*, in `EventBus` (`event.rs:7-44`), which spawns independent tokio tasks that all clone the same `tx`:

- `spawn_crossterm_reader` (`event.rs:18-30`) — crossterm `EventStream` → `Action::RawKey` / `Action::Resize`
- `spawn_tick(250)` (`event.rs:32-43`, called at `main.rs:119`) — `Action::Tick` every 250ms
- `FileWatcher` (`watcher.rs:40-71`) — notify-debouncer-full callback → `Action::FileChanged`

**This matters for the driver:** adding a fourth event source is a *pure addition* — clone `tx`, spawn a task, send `Action` variants. No change to the loop shape at `main.rs:139-202`. This is the single most important integration fact in this document.

### 1.2 The Action contract and its two hidden constraints

```rust
// action.rs:5
#[derive(Debug, Clone)]
pub enum Action { ... }
```

Two constraints fall out, both of which shape the driver design:

1. **`Clone` is derived.** A `tokio::process::Child`, a `ChildStdin`, or a `JoinHandle` is not `Clone`. **No process handle can ride inside an `Action`.** Driver handles must live in a side table; Actions carry only IDs and owned data.
2. **Large payloads must be boxed.** `action.rs:21-22` boxes `ProjectState` with the comment *"Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)"*. Any new driver-state variant of comparable size must follow suit or clippy fails the build (the project holds a zero-clippy-warning bar — PROJECT.md:107).

### 1.3 The mutation contract

`App::update` (`app.rs:235-443`) is one `match action` with 10 arms. Every arm ends by setting `self.needs_redraw = true` when it changes anything visible. Screens never mutate app state directly; they return a `ScreenAction` (`ui/screens/mod.rs:37-48`) which `process_screen_action` (`app.rs:458-487`) interprets. `ScreenAction::DispatchAction(Box<Action>)` (`mod.rs:47`) is the existing escape hatch for a screen to fire async work — `app.rs:481-485` just forwards it to `event_tx`.

**The driver needs no new `ScreenAction` variant.** `DispatchAction` already covers "user pressed a key, kick off async work."

### 1.4 The I/O contract

Two patterns, both already in use:

- **`spawn_blocking` for sync fs/process work.** `app.rs:252-256` (session detection), `app.rs:289-295` (project state re-parse). Result returns as an `Action` on the cloned `tx`.
- **`tokio::spawn` for async work.** `detail.rs:1490` (git log), `app.rs:385-398` (polling for a `.planning/` dir to appear).

`session_detector.rs:19-20` documents the discipline explicitly: *"Uses std::process::Command (not tokio) — called from spawn_blocking."*

### 1.5 The state-reading contract

`parse_project_state(&planning_dir) -> ProjectState` (`state_reader/mod.rs:103-240`) is a single synchronous full re-parse: STATE.md, ROADMAP.md, per-phase disk inference, backlog count, QUEUE.md, HANDOFF, async-jobs, workstreams. It is idempotent and never panics (`:102`).

The signals a driver needs already exist and are already parsed:

| Driver needs | Already in `ProjectState` |
|---|---|
| Where are we in the milestone? | `current_phase`, `current_phase_name`, `current_plan`, `completed_phases`/`total_phases` (`mod.rs:15-28`) |
| Where are we in D-R-P-E-V for this phase? | `current_phase_status: Option<DiskInference>` (`mod.rs:34`), built at `mod.rs:190-209` |
| Which sub-artifacts exist? | 22 booleans on `DiskInference` (`disk_status.rs:17-44`) |
| Are we parked? | `paused` + `pause_context` (`mod.rs:35-38`), `external_job_waiting` (`mod.rs:41`) |
| Is a phase actually complete? | `plan_count` vs `summary_count` (`disk_status.rs:19-20`) |

And the D-R-P-E-V position is **already computed** by `derive_all_stage_statuses(&DiskInference) -> [StageStatus; 5]` at `detail.rs:3281-3328`. It returns `Complete | Current | Skipped | NotStarted` per stage.

> **This is the single biggest architectural gift in the codebase.** `derive_all_stage_statuses` is 90% of the driver's "where are we" logic, already written, already eyeball-validated against real projects across four milestones. It is currently private inside a 216KB UI file. **Lift it into `state_reader/` and the driver's decision function becomes a thin policy layer over it.**

### 1.6 The session/terminal contract

- `session_detector.rs:21-28` — `pgrep -x claude` → `/proc/<pid>/{cwd,cmdline,stat,fd/0}`. Linux-only by construction.
- `session_detector.rs:83-98` — session_id is recovered **only** by scanning cmdline for `--resume <id>`. A freshly-started `claude` has no `--resume` in argv, so **the current detector cannot learn the session ID of most live sessions.**
- `terminal_switch.rs:19-74` — `tmux list-panes -a -F "#{pane_tty} …"`, substring-match `pane_tty` against the session's TTY, then `select-window` + `select-pane`. Returns a human-readable `Err(String)` for the status bar. Hard-fails outside tmux (`:20-25`).

---

## 2. The Capability That Changes the Design

I ran the installed CLI rather than trusting the v1.2 design doc, which was written against v2.1.87. **v2.1.220 has capabilities that invalidate three of that document's conclusions.**

### 2.1 Verified flags (source: `claude --help`, v2.1.220 — HIGH confidence, direct observation)

```
--input-format <format>      Input format (only works with --print): "text" (default),
                             or "stream-json" (realtime streaming input)
--replay-user-messages       Re-emit user messages from stdin back on stdout for
                             acknowledgment (only works with --input-format=stream-json
                             and --output-format=stream-json)
--session-id <uuid>          Use a specific session ID for the conversation
--bg, --background           Start the session as a background agent and return
                             immediately (manage with `claude agents`)
--permission-mode <mode>     acceptEdits | auto | bypassPermissions | manual | dontAsk | plan
--max-budget-usd <amount>    Maximum dollar amount to spend on API calls
--include-hook-events        Include all hook lifecycle events in the output stream
--fork-session               When resuming, create a new session ID instead of reusing
```

And `claude agents --json` — *"Print active sessions (interactive and background) as a JSON array and exit (for scripting; does not require a TTY)"* — which I ran on this machine:

```json
[ { "pid": 711810,
    "cwd": "/home/blk/projects/rust/gsd-meta-manager",
    "kind": "interactive",
    "startedAt": 1785274901878,
    "sessionId": "4661fdcd-7717-459f-9c14-d7e5e713b2e9",
    "name": "gsd-meta-manager-77",
    "status": "busy" } ]
```

### 2.2 What this overturns

| v1.2 design doc claim | 2026-07 reality |
|---|---|
| §12.1: *"There is no stdin injection mechanism for a headless process after it has launched."* | **False now.** `--input-format stream-json` is exactly that mechanism, and `--replay-user-messages` supplies the delivery confirmation the backlog note said `tmux send-keys` lacked. |
| §6: session_id is *"captured from structured output… only used for diagnostic purposes."* | **Inverted.** `--session-id <uuid>` lets the caller **choose** the ID before launch. The driver generates a UUID, journals it, *then* spawns. Re-attachment after a crash no longer depends on having successfully parsed stdout. |
| §12.2: *"unclear how to re-attach monitoring to a running process."* | Largely solved: `claude agents --json` enumerates every live session with `sessionId`, `cwd`, and `busy`/`idle` — no `/proc` parsing, no TTY heuristics. |

### 2.3 What it does *not* solve — the pitfalls that survive

MEDIUM confidence (community protocol doc + three corroborating issues; `--input-format stream-json` is officially undocumented — anthropics/claude-code#24594):

- **Mid-turn stdin is dropped.** A user message written while a turn is in flight is ignored by that turn *and* is not persisted to the session's history JSONL (#41230), so it also vanishes on `--resume`. **Consequence: the driver must queue interjections in its own buffer and flush them only on the `result` event**, or precede them with a `control_request` interrupt.
- **Interrupt is a control request, not a message.** Shape: `{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}` → `{"type":"control_response","response":{"subtype":"success","request_id":"req_1"}}`. Feature-detect via the `capabilities` array on `system/init` (e.g. `interrupt_receipt_v1`, v2.1.205+) rather than version-string comparison. A plain `{"type":"interrupt"}` stdin message was requested and rejected (#41665).
- **`--bare` is a trap, twice over.** Its help text states auth is *"strictly ANTHROPIC_API_KEY or apiKeyHelper… OAuth and keychain are never read"* — which directly contradicts the ROADMAP's *"Uses the Claude subscription… not the API."* And the headless docs note `--bare` *"will become the default for `-p` in a future release."* **The driver must pass its context explicitly and pin non-bare behavior**, or it will silently lose CLAUDE.md, hooks, and GSD skills on a routine `claude` update. A dated, tracked, time-bomb dependency.
- **`--permission-mode dontAsk` is the right unattended mode.** Per the headless docs it denies `AskUserQuestion` *even when an allow rule matches*. That converts the backlog's "unattended runs hang on AskUserQuestion" risk from a **hang** into a **fast, detectable failure** the driver can park on. Prefer it over `bypassPermissions`.
- **SIGTERM semantics are defined:** aborts the turn, kills the bash process tree, runs `SessionEnd` hooks, exits **143**. That is the kill switch — no SIGKILL needed in the normal path.

---

## 3. System Overview

```
┌───────────────────────────────────────────────────────────────────────────┐
│  TUI PROCESS  (gsd-meta-manager)          — unchanged loop, new sources    │
├───────────────────────────────────────────────────────────────────────────┤
│  EventBus (event.rs) ── one mpsc<Action> ──▶ App::update (app.rs:235)      │
│    ▲          ▲              ▲                        │                   │
│    │crossterm │tick(250ms)   │FileWatcher             ▼                   │
│    │          │              │(watcher.rs)      screen_stack              │
│    │          │              │                  ├ NormalScreen            │
│    │          │              │                  └ DetailScreen (11 tabs)  │
│    │          │              │                       └ ★ Driver tab       │
│    │          │              │                                            │
│    │   ★ DriverTailer  ★ ControlClient (optional unix socket)             │
│    │   (tails journal)  (interject / kill — fast path only)               │
└────┼──────────┼──────────────┼────────────────────────────────────────────┘
     │          │              │  reads                    writes │
     │          │       ┌──────┴──────────────────────────────────▼───────┐
     │          │       │ ★ RUN JOURNAL  (durable, single source of truth) │
     │          │       │  .planning/meta-manager/runs/<run-id>/           │
     │          │       │     run.json       ← goal, opt-in, status (small)│
     │          │       │     journal.jsonl  ← append-only event log       │
     │          │       │     inbox.jsonl    ← interjections (durable path)│
     │          │       │     control.sock   ← optional fast path          │
     │          │       └──────▲──────────────────────────────────────────┘
     │          │              │ appends
┌────┴──────────┴──────────────┴────────────────────────────────────────────┐
│  ★ DRIVER PROCESS  (gsd-meta-manager drive <alias>)  — detached, no TTY    │
├───────────────────────────────────────────────────────────────────────────┤
│   driver/mod.rs   run loop:  observe → decide → act → journal → repeat     │
│        │                                                                  │
│        ├─ state_reader::parse_project_state()   (SHARED, unmodified)       │
│        ├─ driver/policy.rs   decide()  ← deterministic D-R-P-E-V router    │
│        ├─ driver/goal.rs     LLM: goal decomposition + ambiguity adjudge   │
│        └─ executor::Executor                                              │
│              └─ ClaudeExecutor  ── ExecutionTarget ──┬─ Host              │
│                 duplex stream-json                   └─ Container         │
└──────────────────────────────────────────┬────────────────────────────────┘
                                           │ spawns
                    ┌──────────────────────┴────────────────────────────┐
                    │ claude -p --input-format stream-json               │
                    │        --output-format stream-json --verbose       │
                    │        --session-id <uuid> --permission-mode       │
                    │        dontAsk --max-budget-usd N                  │
                    │  [ docker|podman exec -i <ctr> … for Container ]    │
                    └──────────────────────┬────────────────────────────┘
                                           │ writes
                              .planning/**  (STATE.md, ROADMAP.md, SUMMARY.md…)
                                           │
                                           └──▶ FileWatcher ──▶ TUI refresh (already works)
```

★ = new.

**The load-bearing property:** the TUI and the driver share exactly one primitive — the filesystem. The TUI never holds a handle on the driver. This is what makes "TUI closes, run continues" and "TUI restarts, run reappears" fall out for free rather than needing recovery machinery.

---

## 4. Question 3 First: The Journal, and Why "Free Watching" Has a Bill

Answering the state-modeling question before the placement question, because placement depends on it.

### 4.1 Layout

```
.planning/meta-manager/                      ← app namespace already exists (queue_md.rs:178)
└── runs/
    ├── active                               ← one line: <run-id>, or absent. Cheap to poll.
    └── 2026-07-28T14-03-11Z-a3f9/
        ├── run.json                         ← COMMITTED. goal prompt, opt-in record,
        │                                       started_at, status, pid, session ids.
        │                                       Small; rewritten atomically (tempfile +
        │                                       persist — same idiom as config.rs:72-87).
        ├── journal.jsonl                     ← GITIGNORED. append-only, one event per line.
        ├── inbox.jsonl                       ← GITIGNORED. TUI→driver interjections.
        └── control.sock                      ← optional; absent = journal-only mode.
```

`run.json` is committed because the requirement (PROJECT.md:93) is *"the originating goal prompt viewable (so the goal is legible later)."* `journal.jsonl` is gitignored because it carries LLM output volume.

### 4.2 Why JSONL and not a socket or shared memory

| Option | Verdict |
|---|---|
| **Append-only JSONL** | ✅ **Chosen.** Crash-safe by construction (a torn final line is discardable — the reader skips unparseable lines). Survives both processes dying. Human-readable when debugging at 3am. Replayable. Free change notification (§4.3). |
| Unix socket only | ❌ As the *sole* channel. Nothing survives a driver crash; the TUI has no way to answer "what happened while I was closed?" Fine as an **optional latency fast-path** layered on top. |
| Shared memory | ❌ Wrong tool. Needs both processes alive simultaneously, needs a lock protocol, unreadable post-mortem, and buys nothing at a ~1 event/sec rate. |
| SQLite | ⚠️ Defensible but overkill: a dependency, a schema-migration story, and a lock story, to serve a single-writer/single-reader append log. Revisit only if querying run history becomes a feature. |

The stateless-session + file-journal + circuit-breaker shape is the convergent pattern for long-horizon agent runs (LOW confidence — web synthesis, no single authority, but consistent across every orchestrator surveyed).

### 4.3 Yes, it gets live updates for free — mechanically

`watcher.rs:22-35`:

```rust
fn extract_project_root(path: &Path) -> Option<&Path> {
    let mut current = path;
    loop {
        if let Some(name) = current.file_name() {
            if name == ".planning" { return current.parent(); }
        }
        …
    }
}
```

It walks *up* from any changed path until it hits `.planning`, at arbitrary depth — the existing test at `watcher.rs:105-110` proves this for `.planning/phases/01/plan.md`. And `watch()` uses `RecursiveMode::Recursive` (`watcher.rs:76`). So a write to `.planning/meta-manager/runs/<id>/journal.jsonl` fires `Action::FileChanged { project_path }` with **zero new watcher code**.

### 4.4 …and here is the bill. Three required `app.rs` changes.

This is the part that will bite if it isn't planned for.

**(a) Every journal append currently triggers a full project re-parse.**
`app.rs:265-307` handles `FileChanged` by calling `parse_project_state` on a `spawn_blocking` (`:289-295`). That function reads STATE.md, ROADMAP.md, QUEUE.md, HANDOFF, every phase directory's disk inference, and every workstream (`state_reader/mod.rs:117-237`). A driver appending an event every few seconds triggers that repeatedly, for the entire duration of a multi-hour run.

The 500ms dedup at `app.rs:278-283` blunts but does not fix it — it caps the rate at 2/sec of a *full* re-parse, indefinitely.

> **Required:** classify the changed path *before* dispatching. If the change is under `meta-manager/runs/`, emit a lightweight `Action::DriverJournalAppended { alias, run_id }` that tails from the last-read byte offset. Only non-driver paths take the `parse_project_state` route. This is a small change inside the existing `FileChanged` arm — **plus a `changed_path` field on the `FileChanged` variant**, because today it carries only `project_path` (`action.rs:10-12`) and the handler therefore has *no way* to tell driver writes from GSD writes.

**(b) Driver state must stay out of `ProjectState`'s `PartialEq`.**
`app.rs:314-322` suppresses the "Updated: {alias}" status message by comparing `*old_state != *state`. That was a deliberate v1.4 feature — *"Refresh noise suppression — no 'Updated' status when project state is unchanged"* (PROJECT.md:45). `ProjectState` derives `PartialEq` (`state_reader/mod.rs:13`).

> **Required:** do **not** add a live driver field to `ProjectState`. Keep driver run state in a sibling `HashMap<String, DriverRunState>` on `AppContext`. If a pointer on `ProjectState` proves unavoidable it must be excluded from equality — which means hand-writing `PartialEq`, which is worse. Prefer the sibling map.

**(c) The tick handler is the right place for the liveness probe, and the pattern already exists.**
`app.rs:246-257` polls Claude sessions every 20 ticks (~5s) via `spawn_blocking`. The driver liveness probe (is the recorded PID still alive and still ours?) and the `claude agents --json` read slot into the same counter block. Reuse `session_poll_counter`; don't add a second timer.

### 4.5 Event schema (sketch)

```jsonc
{"ts":"…","seq":1,"kind":"run_started","goal":"…","dry_run":false,"target":"host"}
{"ts":"…","seq":2,"kind":"observed","phase":"14","drpev":["Complete","Skipped","Current","NotStarted","NotStarted"],"plans":3,"summaries":0}
{"ts":"…","seq":3,"kind":"decided","by":"policy","command":"/gsd:execute-phase 14","rationale":"P complete, E not started"}
{"ts":"…","seq":4,"kind":"exec_started","session_id":"<uuid>","argv_digest":"sha256:…"}
{"ts":"…","seq":5,"kind":"exec_event","stream":"assistant","text":"…"}
{"ts":"…","seq":6,"kind":"interjected","text":"skip the UI review","delivered":true}   // delivered ← --replay-user-messages ack
{"ts":"…","seq":7,"kind":"exec_finished","exit":0,"cost_usd":1.83,"duration_s":420}
{"ts":"…","seq":8,"kind":"parked","reason":"verification_gaps_found","needs":"human"}
```

`seq` is monotonic so a tailing reader can detect gaps. `kind` is a closed set — but the TUI must tolerate unknown kinds (forward compat) by rendering them as raw text rather than erroring.

---

## 5. Question 1: Where the Driver Loop Lives

### 5.1 The options, scored against the real requirement

The requirement is explicit: *"An autonomous run may take hours and should survive the TUI being closed"*, and PROJECT.md:118-119 makes stoppability and dry-run hard requirements.

| Option | Survives TUI close | Re-attach story | Portability | Verdict |
|---|---|---|---|---|
| **A. In-process tokio task** | ❌ No | N/A | ✅ | **Rejected.** Fails the stated requirement outright. Would also park non-`Clone` process handles adjacent to the `Action` path (§1.2). |
| **B. Detached child of the same binary** | ✅ | Journal + PID probe | ✅ Unix; Windows needs `CREATE_NEW_PROCESS_GROUP` | ✅ **Chosen.** |
| **C. `systemd-run --user` transient unit** | ✅ | `systemctl --user status` + journal | ❌ Linux + systemd only | **Optional wrapper on B**, not an alternative. |
| **D. tmux-hosted** | ✅ | `tmux ls` + journal | ❌ requires tmux running | **Optional wrapper on B.** |
| **E. `claude --bg` background agents** | ✅ (per session) | `claude agents --json` | ✅ | **Not applicable to the loop.** Supervises a *session*, not the driver. Useful for §5.4's liveness data. |

Ecosystem synthesis backs the split (MEDIUM confidence): `setsid`/double-fork gives detachment but no reattach channel; tmux is excellent for *observing* but a poor *supervisor* (systemd+tmux needs `Type=forking` and loses journald capture); `systemd-run --user` is the strongest supervisor but is Linux-only and conflicts with the portability constraint (PROJECT.md:120). The recurring advice is to keep the worker a plain foreground binary and expose a control channel separately — which is exactly option B plus §4.1's optional socket.

### 5.2 Why B specifically

- **One binary, one build, one version.** `cli.rs:19-35` already has a `Commands` enum with three subcommands; a fourth (`Drive`) costs ~10 lines. `main.rs:30-129` already dispatches subcommands *before* TUI init (`main.rs:78-80` is the `None =>` TUI arm), so the driver path never touches ratatui.
- **The driver reuses `state_reader` verbatim.** Same crate, same `parse_project_state`, guaranteed-identical interpretation between what the dashboard shows and what the driver decides on. A separate program would make that a permanent drift hazard.
- **No new IPC is *required*.** The journal is the contract. The socket is an optimization deferrable to a later phase without redesign.
- **Testable without a TUI.** `gsd-meta-manager drive <alias> --dry-run` is a plain CLI invocation — unit-testable, CI-runnable, and it satisfies the dry-run hard requirement *by construction* rather than as a UI mode.

### 5.3 Detachment mechanics

```rust
// in the TUI, on user confirm:
let mut cmd = std::process::Command::new(std::env::current_exe()?);
cmd.args(["drive", alias, "--run-id", &run_id])
   .stdin(Stdio::null())
   .stdout(Stdio::null())     // driver logs to the journal + the tracing file appender
   .stderr(Stdio::null());
#[cfg(unix)]
{ use std::os::unix::process::CommandExt; cmd.process_group(0); }  // new pgid → no SIGHUP/SIGINT from the TUI's terminal
let child = cmd.spawn()?;
```

`process_group(0)` (stable since Rust 1.64) is sufficient here and avoids a `daemonize`/`nix` dependency. The driver reuses the existing `tracing` file-appender setup (`main.rs:15-23`) so debug logs land in `~/.local/share/gsd-meta-manager/` exactly as they do today.

**Critically: write `run.json` with `status: "starting"` and the run-id *before* spawning.** If the spawn fails, or power is lost between spawn and the first journal write, the TUI still finds a record and can reconcile. Never let the process be the only evidence the run exists.

### 5.4 Re-attachment on TUI restart

At startup (`main.rs:81-116`, alongside the existing startup session scan at `:113-116`):

```
for each registered project:
  read .planning/meta-manager/runs/active  → run_id, or skip
  read run.json                            → pid, session_ids, status
  probe liveness:
     /proc/<pid>/cmdline contains "gsd-meta-manager" AND "--run-id <run_id>"
       → LIVE:  tail journal.jsonl from byte 0, mark project as driven
       → DEAD:  read the journal's last event
                 · last kind == run_finished / parked  → terminal, archive it
                 · otherwise                            → CRASHED, surface to user,
                   offer resume (the recorded session_id makes `--resume` viable)
```

The PID+cmdline double-check is the standard defense against PID reuse and is the *same technique already proven* in `session_detector.rs:46-69` (read `/proc/<pid>/cwd`, `/cmdline`, `/stat`). No new capability — just a second consumer of it.

**This is why `--session-id <uuid>` matters so much.** The driver generates the UUID and journals it *before* spawning `claude`. A crash mid-turn leaves a resumable session ID on disk regardless of whether any stdout was ever parsed. Under the v1.2 design (read session_id back from output) a crash before the first flush lost the session permanently.

### 5.5 Interjection: two paths, one durable

- **Fast path (optional):** TUI connects to `control.sock`, writes `{"kind":"interject","text":"…"}`, gets an ack. Sub-100ms.
- **Durable path (required):** TUI appends to `runs/<id>/inbox.jsonl`; the driver watches/polls it. Slower (~1 tick), but works when the socket is gone and after either process restarts.

**Build the durable path first.** The socket is a latency optimization for a workflow whose unit of work is measured in minutes.

Then, regardless of path, the driver **must** buffer the interjection and flush it at the `result` boundary, per §2.3. Injecting mid-turn silently loses the message *and* loses it from history. This is the highest-severity foot-gun in the design and deserves a named test case.

---

## 6. Question 2: The Shape of `decide()`

### 6.1 The consensus, and why it applies here unusually cleanly

The production consensus (MEDIUM confidence — web synthesis across several orchestrator write-ups and two arXiv papers) is: **deterministic orchestration at the workflow level, LLM at the task level, narrow LLM routing only at genuinely semantic decision points.** The argument is structural, not a model-capability gap: control-flow decisions sampled from a distribution have per-step accuracy < 1, so error compounds over a long horizon. Observed failure modes are re-doing completed work, skipping steps, and terminating early. Reference architectures (Bernstein; a five-phase verdict-gated phase machine) spend **zero LLM tokens on coordination** — the LLM runs once, at goal decomposition.

**GSD is an unusually good fit for the deterministic side of that split**, for a reason specific to this project: the pipeline has a fixed, known, five-stage structure (D-R-P-E-V), the transitions are gated by *artifacts on disk*, and this repo **already has the function that reads that position** — `derive_all_stage_statuses` at `detail.rs:3281-3328`. The decision function is not a research problem. It is a lookup table over a `[StageStatus; 5]` the codebase already computes correctly.

### 6.2 The recommended shape

```rust
// driver/policy.rs — pure, synchronous, no I/O, no LLM. Trivially unit-testable.
pub fn decide(state: &ProjectState, goal: &RunGoal, hist: &RunHistory) -> Decision;

pub enum Decision {
    Run { command: String, rationale: String },   // "/gsd:execute-phase 14"
    Park { reason: ParkReason },                  // human needed; run stays resumable
    Ambiguous { question: String, candidates: Vec<String> },  // ← the ONLY door to the LLM
    Done,
}
```

**Ordering inside `decide` — hard gates first, then the pipeline:**

| Order | Guard | Source signal | Action |
|---|---|---|---|
| 1 | `state.paused` | `mod.rs:35` (HANDOFF present) | `Park(HandoffPresent)` — never drive a paused project |
| 2 | `state.external_job_waiting` | `mod.rs:41` (`async-jobs/*.json`) | `Park(WaitingOnExternalJob)` — legitimately blocked, not stuck |
| 3 | `classify_status(&state.status) == Blocked` | `app.rs:59-83` | `Park(Blocked)` |
| 4 | circuit breaker: same command N times, or no artifact change in T | `RunHistory` | `Park(NoProgress)` |
| 5 | budget / step-count cap exceeded | `RunHistory` | `Park(BudgetExhausted)` |
| 6 | milestone terminal | `state_md::is_milestone_terminal` (`app.rs:65`) | `Done`, or advance the goal |
| 7 | `is_all_phases_complete` | `app.rs:69` | `Run("/gsd:complete-milestone")` — note this is deliberately **not** Complete, per the ADR-2207 comment at `app.rs:60-64` |
| 8 | D-R-P-E-V position | `derive_all_stage_statuses` | table below |

D-R-P-E-V mapping (reusing the existing semantics at `detail.rs:3281-3328`):

| Observed | Command |
|---|---|
| D `NotStarted` / `Current` | `/gsd:discuss-phase <n>` |
| D done, P `NotStarted` | `/gsd:plan-phase <n>` |
| P done, E `Current` (`summary_count < plan_count`) | `/gsd:execute-phase <n>` |
| E `Complete`, V `NotStarted` | `/gsd:verify-work <n>` |
| V `Complete`, more phases remain | advance phase, loop |
| R `Skipped` on a phase the goal flags research-heavy | `Ambiguous` → LLM |

### 6.3 Where the LLM goes — exactly two places

**(a) Goal decomposition, once per run.** The user's free-text goal ("build milestones 1-3, then brainstorm the next one, plan it, execute it") is compiled *once* into a structured `RunGoal`:

```rust
pub struct RunGoal {
    pub raw_prompt: String,           // verbatim — this is what run.json exposes to the UI
    pub milestones: Vec<String>,
    pub stop_after: StopCondition,
    pub allow: AllowSet,              // push? open PR? create milestones?
    pub max_steps: u32,
    pub max_budget_usd: f64,
}
```

This is a `claude -p --output-format json --json-schema '<RunGoal schema>'` call — the `--json-schema` flag (verified in `--help`; populates a `structured_output` field) makes it a typed function call rather than prose parsing. **Do this at run start, journal the result, and show the user the compiled goal before the run begins.** That review step is a cheap and very effective blast-radius control: the human approves an explicit machine-readable plan rather than a vibe.

**(b) Ambiguity adjudication, only when `decide` returns `Ambiguous`.** Bounded: hand the LLM the observed state, the goal, and a *closed set* of candidate commands; require it to pick one or answer `park`. Journal the question and the answer. **Cap adjudications per run** — if the router keeps hitting ambiguity, that is a signal the router is wrong, not that the LLM should take over.

Everything else — every ordinary D→P→E→V advance — is a table lookup. The LLM never chooses the next GSD command on the common path.

### 6.4 The anti-pattern to avoid by name

> **"Just give an LLM the state dump and ask what to run next, in a loop."**

It looks like it works for the first three iterations. It fails on hour four of an unattended run, and it fails in the specific ways this project cannot tolerate: re-running `/gsd:execute-phase` on an already-complete phase (wasted spend, churned commits), skipping `/gsd:verify-work` (the exact gate the pipeline exists to enforce), and declaring victory early. **And it is untestable** — you cannot unit-test a prompt. A pure `decide()` gets a test per D-R-P-E-V permutation, which bounds to maybe 20 meaningful cases.

### 6.5 One structural nuance: `/gsd:next` is a tempting shortcut

The v1.2 design doc §5 noted `/gsd:next` auto-detects state and routes. A driver could simply loop `claude -p "/gsd:next"`. **Don't make that the architecture** — it moves the decision inside an opaque LLM turn, so the journal records "ran /gsd:next" rather than *why*, destroying the auditability the Driver tab exists to provide. It is, however, a perfectly good **fallback** for the `Ambiguous` branch, and an excellent smoke test for the first band.

---

## 7. Question 5: The Container Target, and the v1.2 `Executor` Verdict

### 7.1 Verdict on the prior decision (PROJECT.md:134 — *"LLM-agnostic queue execution design… ✓ Good — Executor trait interface designed"*)

**It still fits, and it should be revived rather than redesigned — but the trait signature needs one change, and one of its stated rationales is now weaker.**

| Aspect of the v1.2 design | Verdict |
|---|---|
| Process-based model (*"spawns a subprocess, monitors its output and filesystem side effects, and reports completion"*, §11) | ✅ **Holds.** Still the lowest common denominator across LLM CLIs, and it is precisely what makes the container target a trivial variation. |
| `ExecutionEvent` enum (`Output`/`Progress`/`Checkpoint`/`Completed`/`Error`) | ✅ **Holds**; maps cleanly onto stream-json event types. Add `Cost` and `SessionStarted`. |
| `ExecutionOptions` (timeout, budget, auto_approve, model, resume_session, name) | ✅ **Holds nearly verbatim** — every field has a real flag: `--max-budget-usd`, `--permission-mode`, `--model`, `--resume`, `--name`. Add `session_id` (pre-chosen) and `target`. |
| `trait Executor { start; cancel; is_running }` | ⚠️ **Insufficient.** Fire-and-forget; predates duplex streaming. **Needs `fn send(&self, h: &mut ExecutionHandle, msg: UserMessage) -> Result<()>` and `fn interrupt(&self, h: &mut ExecutionHandle) -> Result<()>`.** Without them there is no interjection — a named v2.0 requirement (PROJECT.md:93). |
| `ExecutionHandle { id, session_id, events: Receiver }` | ⚠️ Add a `stdin` sink and `capabilities: Vec<String>` (from `system/init`) for feature detection. |
| Rationale *"GSD could use any LLM backend"* | ⚠️ **Weaker than in v1.2.** The v2.0 design leans on Claude-specific protocol details — duplex stream-json, `--replay-user-messages`, `--session-id`, control_request/interrupt. A hypothetical Codex/aider backend would satisfy `start`/`cancel` but not `send`/`interrupt`. **Recommendation: keep the trait** — it is the right seam and costs almost nothing — **but be honest that interjection is a Claude capability tier, not universal parity.** Model it with a `capabilities()` method so the UI can grey out "Interject" for a backend that lacks it, rather than pretending. |

### 7.2 Container is a *target*, not a second `Executor`

This is the key structural call, and it is a smaller change than it first appears.

```rust
pub enum ExecutionTarget {
    Host,
    Container { runtime: Runtime, container: String, workdir_map: PathMap },
}
pub enum Runtime { Docker, Podman }
```

`ClaudeExecutor::start` builds the same argv either way; the target supplies only a **prefix** and a **path mapping**:

```
Host:      claude -p --input-format stream-json …
Container: docker exec -i -w /workspace <ctr> claude -p --input-format stream-json …
           podman exec -i -w /workspace <ctr> claude -p …
```

Podman is a deliberate Docker-CLI drop-in for `run`/`ps`/`exec`/`logs`/`build`, so the runtime is genuinely an argv-prefix swap (LOW confidence on the finer points — web synthesis; the coarse claim is well-established, and the real divergences are enumerated below).

**Why not a second `Executor` impl:** everything downstream — stream-json parsing, event mapping, interjection, interrupt, cost accounting, session-id handling — is *identical*. A second impl would duplicate all of it to vary two strings, and the copies would drift. The v1.2 trait's own framing supports this: it abstracts *LLM backends*, and Claude-in-a-container is the same backend over a different transport.

### 7.3 The four things that are genuinely different, and must be handled

1. **Path translation is mandatory and pervasive.** The driver reasons in host paths (`state.project_root`, `state_reader/mod.rs:45`) but the container sees `/workspace`. Every path in a prompt, every `--add-dir`, every artifact path echoed back in stream-json output crosses this boundary. **Make `PathMap` a real type with `to_container()` / `to_host()` and route every path through it.** Ad-hoc `format!` string surgery here produces bugs that appear only in container mode and are miserable to debug.
2. **Auth.** Container `claude` needs credentials, and the ROADMAP requires *subscription*, not API key. `claude setup-token` (verified present in `claude --help` subcommands — *"Set up a long-lived authentication token (requires Claude subscription)"*) is the clean answer: provision once, inject as an env var. **Do not bind-mount `~/.claude` wholesale** — that drags the user's entire config, session history, and keychain state into the container.
3. **Rootless podman file ownership.** Use `-v "$HOST":/workspace:Z` for SELinux relabeling and `--userns=keep-id` for UID mapping. Without `keep-id`, files the agent writes are owned by a subuid the host user cannot read — and since the whole product is a *filesystem watcher*, the failure mode presents as "the TUI stopped seeing the project's own commits." Very confusing; very avoidable.
4. **`--format json` output differs between runtimes.** For `ps`/`inspect`: docker emits line-delimited JSON objects, podman emits a JSON array; label shapes differ (string vs map); inspect diverges on timestamps and `null` vs `[]`. **The runtime-probe module must normalize both shapes**, not assume docker's.

Runtime detection: probe `docker` then `podman` on `PATH` (`detail.rs:89` already uses `Command::new("which")` for exactly this job in `find_terminal()` — reuse the pattern), honoring an explicit config override. This satisfies the auto-detection decision at PROJECT.md:138. *(Note: only `docker` 29.1.3 is installed on this machine — podman paths need a container or CI to exercise.)*

---

## 8. Question 4: New vs Modified — Explicit

### 8.1 New components

| # | Path | Responsibility |
|---|---|---|
| N1 | `src/executor/mod.rs` | `Executor` trait (+`send`/`interrupt`/`capabilities`), `ExecutionOptions`, `ExecutionHandle`, `ExecutionEvent`. Revived from the v1.2 design with §7.1 amendments. |
| N2 | `src/executor/claude.rs` | `ClaudeExecutor`. Builds argv, spawns via `tokio::process`, owns the stdin sink + stdout reader tasks. |
| N3 | `src/executor/stream_json.rs` | serde types for the NDJSON protocol: `system/init` (+`capabilities`), `assistant`, `user`, `stream_event`, `system/api_retry`, `result`, `control_request`/`control_response`. Must tolerate unknown `type`/`subtype`. |
| N4 | `src/executor/target.rs` | `ExecutionTarget`, `Runtime`, `PathMap` with `to_container` / `to_host`. |
| N5 | `src/container/mod.rs` | Runtime probe; container lifecycle (create/start/stop/rm/status); `ps --format json` shape normalization (docker NDJSON vs podman array). |
| N6 | `src/driver/mod.rs` | The run loop: observe → decide → act → journal → repeat. Owns the interjection buffer and the turn-boundary flush. |
| N7 | `src/driver/policy.rs` | **`decide()`** — pure, sync, no I/O. The deterministic D-R-P-E-V router (§6.2). Highest test density in the milestone. |
| N8 | `src/driver/goal.rs` | LLM goal decomposition via `--json-schema`; `Ambiguous` adjudication over a closed candidate set with a per-run cap. |
| N9 | `src/driver/journal.rs` | Append-only JSONL writer (single writer, fsync policy) + offset-tracking tail reader that skips torn/unparseable lines. |
| N10 | `src/driver/run.rs` | `RunGoal`, `RunState`, `RunStatus`, `ParkReason`, `RunHistory`; atomic `run.json` read/write (mirror `config.rs:72-87` tempfile+persist). |
| N11 | `src/driver/supervisor.rs` | Detached spawn (§5.3); PID+cmdline liveness probe; kill switch (SIGTERM → exit 143 → SIGKILL fallback); crash reconciliation on TUI startup. |
| N12 | `src/agents.rs` | `claude agents --json` reader → `sessionId` + `busy`/`idle` for live sessions. Complements (does not replace) `session_detector.rs`. |
| N13 | `src/ui/screens/driver.rs` | The Driver tab: goal prompt, decision trace, live stream, interject input, Stop/kill. |

### 8.2 Modified components

| # | File | Change | Anchor |
|---|---|---|---|
| M1 | `src/action.rs` | Add `DriverJournalAppended`, `DriverRunStateLoaded` (**boxed** — follow the `:21-22` precedent), `DriverStarted`, `DriverFinished`, `ContainerStatusLoaded`, `AgentsDetected`. **Extend `FileChanged` with `changed_path`** — today it carries only `project_path` (`:10-12`), so the handler cannot distinguish driver writes from GSD writes. All variants must be `Clone` (§1.2). | `action.rs:5-45` |
| M2 | `src/app.rs` | (a) Route `FileChanged` on driver paths to the cheap tail instead of `parse_project_state` — §4.4(a). (b) New `update()` arms. (c) Extend the 20-tick block with the driver liveness probe + `claude agents --json`. (d) Startup reconciliation call. | `:265-307`, `:236-442`, `:246-257` |
| M3 | `src/ui/screens/mod.rs` | Driver fields on `ProjectViewCache`; `driver_runs: HashMap<String, DriverRunState>` on `AppContext` (**deliberately not on `ProjectState`** — §4.4b). No new `ScreenAction` needed. | `:57-109`, `:111-132`, `:37-48` |
| M4 | `src/app.rs` (`DetailSubView`) | Add the `Driver` variant (11th tab). | `app.rs:16-28` |
| M5 | `src/ui/screens/detail.rs` | Extend both tab-index maps (0-9 → 0-10; `Driver` = 10), add render + key dispatch. **Also lift `derive_all_stage_statuses` (`:3281-3328`) out into `state_reader/`** so the driver and the UI share one definition of D-R-P-E-V position. | `:51-76`, `:3278-3328` |
| M6 | `src/ui/screens/normal.rs` + `src/ui/project_list.rs` | LLM-driven badge + run-status column on the dashboard (PROJECT.md:93). Follows the established badge pattern (pause / workstream / external-job). | — |
| M7 | `src/config.rs` | `Preferences` gains driver defaults (budget, step cap, permission mode, container runtime override). **`RegisteredProject` gains `driver_opt_in: bool`** — the enforcement point for the opt-in constraint (PROJECT.md:112-117). Both `#[serde(default)]` for back-compat with existing config files. | `:16-20`, `:28-34` |
| M8 | `src/cli.rs` | `Commands::Drive { alias, run_id, dry_run, goal }`. | `:19-35` |
| M9 | `src/main.rs` | Dispatch `Drive` **before** `tui::init()` — the driver must never touch ratatui. Add startup run-reconciliation next to the existing session scan. | `:30-129`, `:113-116` |
| M10 | `src/session_detector.rs` | Augment `ClaudeSession` with data from N12: `session_id` for sessions started *without* `--resume` (which `:83-98` cannot see today) and `busy`/`idle`. Keep the `/proc/<pid>/fd/0` TTY read (`:71-81`) — `claude agents --json` has no TTY field, and `terminal_switch.rs` needs it. | `:4-14`, `:83-98` |
| M11 | `src/watcher.rs` | No functional change required — `extract_project_root` (`:22-35`) already resolves nested journal paths. Add a test asserting `…/meta-manager/runs/<id>/journal.jsonl` resolves correctly, to lock the behavior the driver now depends on. | `:22-35`, `:97-116` |

**Explicitly unchanged:** `event.rs` (new sources are additive), `state_reader/mod.rs`'s `parse_project_state` (the driver is a second caller, not a modifier), `terminal_switch.rs` (already generic over `ClaudeSession`), `change_tracker.rs`, `archive.rs`, `browser.rs`, `registry.rs`, `project_creator.rs`.

---

## 9. Question 6: Build Order

Honors PROJECT.md:139 — *"999.2 injection plumbing sequenced before 999.3 driver."* Safety is **not** a trailing band; it is woven in where it is cheapest to enforce.

```
   ┌── B0 UI fixes ──────────────────────────────── (independent, parallel)
   │
   B1 Executor + stream-json (host)
   ├──▶ B2 Journal + run model
   │      └──▶ B3 Supervisor / detach / reattach / kill switch
   │             └──▶ B4 Driver tab + interjection UI
   │                    └──▶ B6 Deterministic policy router + dry-run
   │                           └──▶ B7 LLM goal layer
   └──▶ B5 Container target                     ──────┘
                                     (B5 ∥ B3/B4; both before B6)
```

| # | Band | Deliverable | Why here |
|---|---|---|---|
| **B0** | independent | The four UI fixes (HANDOFF pause badge, DRPEV leading blank, markdown edit-mode activation, PageDown clamp). | Zero dependencies. Ship first for momentum, or run in parallel throughout. |
| **B1** | 999.2 | `Executor` trait + `ClaudeExecutor` duplex stream-json, host only. Verify `/gsd:next` actually works as a `-p` prompt (v1.2 open question §12.3). Prove `--replay-user-messages` acks. **No UI.** | Everything stands on this. If duplex stream-json misbehaves, that must surface in week one, not month two. |
| **B2** | 999.2 | Journal writer/tailer, `run.json`, path classification in `watcher`/`app` (§4.4a), noise-suppression exclusion (§4.4b). | The state substrate. Without it, B3 has nothing to reattach *to*. |
| **B3** | 999.2 | Detached spawn, PID liveness, crash reconciliation, **kill switch**. | Placed before the driver deliberately: it *is* the stoppability hard requirement (PROJECT.md:118), and it is far cheaper to build against a hand-run `claude` step than to retrofit around a live decision loop. |
| **B4** | 999.2 | Driver tab, live stream render, interjection (durable inbox path only; socket deferred), Stop. Dashboard badge. | Completes 999.2's *"monitor container output and inject commands from the TUI."* At this point a human can drive a project step-by-step from the TUI — **useful on its own, and a natural ship point.** |
| **B5** | 999.2 | `ExecutionTarget::Container`, runtime probe, `PathMap`, `setup-token` auth, rootless mount flags, `ps --format json` normalization. | Depends only on B1; can run parallel with B3/B4. Must land before B6 so the driver never needs a stubbed target. |
| **B6** | 999.3 | `decide()` — deterministic D-R-P-E-V router, circuit breaker, park reasons, **dry-run** (a `Decision` that is journaled but not executed). | The decision layer, on proven transport. Dry-run is nearly free here (`decide()` is already pure) and expensive later. |
| **B7** | 999.3 | LLM goal decomposition (`--json-schema`), the compiled-goal review gate, `Ambiguous` adjudication with a per-run cap. | Last. By now the run is fully observable and stoppable, so the first genuinely autonomous run has a safety net beneath it. |

**Two ordering points worth defending:**

- **B3 before B4** (supervisor before UI), not the reverse. Building the Driver tab against an in-process task would bake in the assumption that the TUI owns the run — and then every screen, cache field, and Action would need rework when detachment lands. Detachment is not a feature you add; it is a constraint you design under.
- **B6 before B7** (rules before LLM). If the LLM layer lands first there is enormous pressure to let it paper over gaps in the router, and the router never gets written properly. Rules-first forces every ambiguity to be *named* before an LLM is allowed near it.

---

## 10. Anti-Patterns

### AP1: Putting live driver state on `ProjectState`
**What people do:** add a `driver: Option<DriverRunState>` field next to `paused` and `external_job_waiting`.
**Why it's wrong:** `ProjectState` derives `PartialEq` (`state_reader/mod.rs:13`) and `app.rs:314-322` uses that equality to suppress "Updated:" status spam — a deliberate v1.4 feature (PROJECT.md:45). Driver state changes every few seconds; this would defeat the suppression and flood the status bar for the entire multi-hour run.
**Instead:** a sibling `HashMap<String, DriverRunState>` on `AppContext`.

### AP2: Treating a journal write as an ordinary `.planning/` change
**Why it's wrong:** `app.rs:289-295` answers `FileChanged` with a full `parse_project_state` — STATE.md, ROADMAP.md, every phase's disk inference, every workstream. Doing that twice a second (the `:278-283` dedup floor) for hours is a self-inflicted load, and it makes the tool slowest exactly when it is doing the most.
**Instead:** classify the path first; tail from a byte offset.

### AP3: Injecting a message mid-turn
**Why it's wrong:** the in-flight turn ignores it *and* it isn't persisted to history, so it's also lost on `--resume`. Meanwhile the user sees their message echoed in the TUI and reasonably concludes it landed.
**Instead:** buffer, flush on `result`, and mark `delivered` in the journal only after `--replay-user-messages` acks. If it must land *now*, send `control_request{subtype:"interrupt"}` first, then resubmit.

### AP4: Letting the LLM pick the next GSD command every iteration
**Why it's wrong:** compounding per-step error over a long horizon; the observed failure modes (re-execute a complete phase, skip verify, finish early) are precisely the ones that cost money and corrupt project state. And it is untestable.
**Instead:** §6.2's router; LLM only at `Ambiguous`.

### AP5: A second `Executor` impl for containers
**Why it's wrong:** stream-json parsing, interjection, interrupt, cost, and session handling are all identical. You'd duplicate everything to vary an argv prefix, and the two copies would drift.
**Instead:** `ExecutionTarget` inside the one executor.

### AP6: `tmux send-keys` as the injection transport
**Why it's wrong:** the backlog already named it — screen-scraping, no delivery confirmation, breaks when the pane is mid-prompt or on an `AskUserQuestion`. As of v2.1.220 there is a real duplex channel with acknowledgment.
**Instead:** stream-json stdin. Keep tmux for what it's genuinely good at — letting a human *attach and watch* — via the existing `terminal_switch.rs`.
**This refines PROJECT.md:136.** That decision's *conclusion* (`-p` drives) is right and stands. Its *rationale* — that tmux is needed because `-p` can't be interjected — is now obsolete. The driver should own a single duplex `claude -p` per step; the tmux pane becomes optional ergonomics, not required plumbing.

### AP7: Relying on `--bare` defaults
**Why it's wrong:** `--bare` skips CLAUDE.md, hooks, and project settings, and forces API-key auth (contradicting the subscription requirement). The docs state it *"will become the default for `-p` in a future release"* — so a driver that works today can silently lose all GSD context on a routine `claude` update.
**Instead:** pass context explicitly, and treat this as a dated, tracked assumption guarded by a smoke test that asserts GSD skills resolved.

### AP8: Driving a project that never opted in
**Why it's wrong:** violates the narrowed constraint at PROJECT.md:112-117, which explicitly preserves *"the default remains no interference."*
**Instead:** `RegisteredProject.driver_opt_in` checked in `supervisor.rs` at spawn **and** re-checked in the driver loop before every `Run` decision. Two gates, because a config edit mid-run should stop the run.

---

## 11. Integration Points

### External services

| Service | Integration pattern | Gotchas |
|---|---|---|
| `claude` CLI (v2.1.220+) | subprocess; duplex NDJSON over stdin/stdout | Version-gated: `--input-format stream-json` and `--session-id` are required. Feature-detect via `system/init.capabilities` (v2.1.205+), not version strings. `--input-format stream-json` is officially undocumented — treat the protocol as MEDIUM confidence and pin a smoke test. |
| `claude agents --json` | `spawn_blocking` + serde | No TTY field → does **not** replace `session_detector.rs`'s `/proc/<pid>/fd/0` read (`:71-81`), which `terminal_switch.rs` depends on. |
| `docker` / `podman` | subprocess; argv-prefix swap | `--format json` shape differs (NDJSON vs array). Rootless podman needs `:Z` and `--userns=keep-id`. Only docker 29.1.3 is on this dev machine. |
| `tmux` | already integrated (`terminal_switch.rs`) | Optional. Hard-fails outside tmux by design (`:20-25`). |
| `gsd-tools.cjs` | already resolved (`queue_md.rs:11-31`, `GsdToolsCmd`) | Reuse for pre-flight validation without spawning an LLM (v1.2 design §3). |

### Internal boundaries

| Boundary | Communication | Notes |
|---|---|---|
| TUI ↔ Driver | **Filesystem journal** (required) + unix socket (optional fast path) | The only contract. No shared memory, no handles. This is what makes detach/reattach fall out for free. |
| Driver ↔ `state_reader` | direct call, same crate | Zero drift between what the dashboard shows and what the driver decides on — the whole reason the driver is the same binary. |
| Driver ↔ Executor | trait object + mpsc of `ExecutionEvent` | Mirrors the existing `Action`/mpsc idiom, so it reads as native to this codebase. |
| Executor ↔ target | `ExecutionTarget` (argv prefix + `PathMap`) | Deliberately **not** a trait boundary. |
| Watcher → App | `Action::FileChanged` (**+ `changed_path`**) | The one signature change that unblocks §4.4(a). |

### Legal / distribution note

The Agent SDK docs state Anthropic does not permit third-party developers to *offer* claude.ai login or rate limits in their products. This tool does not: it invokes the user's own locally-installed `claude` CLI with the user's own credentials — what the user does by hand today. Worth a README line nonetheless, since v2.0 changes the tool's category from "watches Claude" to "runs Claude." LOW confidence on the legal reading — surface it to the user; don't settle it in a phase plan.

---

## 12. Confidence Summary

| Claim class | Confidence | Basis |
|---|---|---|
| Every statement about existing code | **HIGH** | Read the file; cited `file:line`. |
| `claude` CLI flags and `agents --json` shape | **HIGH** | Ran `claude --help` and `claude agents --json` against locally installed v2.1.220. Direct observation, not a doc claim. |
| Headless docs behavior (`--bare`, SIGTERM/143, `dontAsk`, skill resolution in `-p`) | **MEDIUM-HIGH** | Official `code.claude.com/docs/en/headless`, fetched today. |
| stream-json wire shapes, mid-turn drop, interrupt as control_request | **MEDIUM** | Community protocol doc + three corroborating claude-code issues. Officially undocumented — **verify empirically in B1 before building on it.** |
| Orchestration hybrid consensus | **MEDIUM** | Multi-source web synthesis (2 arXiv, 2 practitioner architectures). Directionally strong, no single authority. |
| Supervisor tradeoffs (setsid / tmux / systemd) | **MEDIUM** | Web synthesis; the conclusions are standard Unix practice. |
| podman/docker divergences | **LOW** | Web synthesis only; podman is not installed here. **Verify in B5.** |

### Gaps to resolve during implementation

1. **Does `claude -p` reliably execute a multi-step GSD skill** (`/gsd:execute-phase 14`, which itself spawns subagent waves)? Open since the v1.2 doc §12.3, still unverified. **First thing to test in B1** — the entire milestone rests on it.
2. **Exact behavior of a stdin user message during an active turn** in v2.1.220 specifically. Community reports span older versions.
3. **Does `--max-budget-usd` apply on subscription auth**, or only to API-key billing? The flag's help says *"API calls."* Cost-cap safety depends on the answer.
4. **The podman path is entirely unexercised** on this machine.
5. **Windows detachment** — `process_group(0)` is Unix-only. The portability constraint (PROJECT.md:120) is already softened in practice by `session_detector.rs` being Linux-only (`/proc`); make this an explicit accepted limitation rather than an accidental one.

---

## Sources

**Primary — direct observation (HIGH)**
- `claude --help`, `claude agents --help`, `claude agents --json` — locally installed Claude Code **v2.1.220**, run 2026-07-28
- Repository source, read in full or in the cited ranges: `src/main.rs`, `src/app.rs`, `src/action.rs`, `src/event.rs`, `src/watcher.rs`, `src/config.rs`, `src/cli.rs`, `src/session_detector.rs`, `src/terminal_switch.rs`, `src/state_reader/mod.rs`, `src/ui/screens/mod.rs`, and cited ranges of `src/ui/screens/detail.rs`, `src/state_reader/disk_status.rs`, `src/state_reader/queue_md.rs`
- `.planning/PROJECT.md`, `.planning/ROADMAP.md` (backlog 999.2 / 999.3)
- `.planning/milestones/v1.2-phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md` — the prior `Executor` design, re-evaluated in §7.1

**Official documentation (MEDIUM-HIGH)**
- [Run Claude Code programmatically (headless)](https://code.claude.com/docs/en/headless)
- [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)

**Community / ecosystem (MEDIUM–LOW, tagged inline)**
- [claude-agent-sdk-go CLI protocol doc](https://github.com/Roasbeef/claude-agent-sdk-go/blob/main/docs/cli-protocol.md) — NDJSON shapes, lifecycle
- claude-code issues [#24594](https://github.com/anthropics/claude-code/issues/24594) (undocumented `--input-format`), [#41230](https://github.com/anthropics/claude-code/issues/41230) (mid-turn stdin not persisted), [#41665](https://github.com/anthropics/claude-code/issues/41665) (interrupt-on-stdin, closed)
- [AI agent orchestration: LLM vs code-driven patterns](https://genta.dev/resources/ai-agent-orchestration-patterns-llm-vs-code-driven)
- [Deterministic AI orchestration (Praetorian)](https://www.praetorian.com/blog/deterministic-ai-orchestration-a-platform-architecture-for-autonomous-development/)
- [LLM-as-Code: agentic programming for agent harness (arXiv 2606.15874)](https://arxiv.org/pdf/2606.15874)
- [awesome-agent-orchestrators](https://github.com/andyrewlee/awesome-agent-orchestrators)
- [Podman Docker compatibility](https://deepwiki.com/containers/podman/1.2-docker-compatibility) · [podman#21847](https://github.com/containers/podman/issues/21847) · [Rootless podman volumes](https://www.tutorialworks.com/podman-rootless-volumes/)
- [Building a daemon using Rust](https://tuttlem.github.io/2024/11/16/building-a-daemon-using-rust.html) · [devenv-systemd-run](https://github.com/jkxyz/devenv-systemd-run)

---
*Architecture research for: v2.0 Autonomous Orchestration — driver integration into the existing GSD Meta Manager TUI*
*Researched: 2026-07-28*
