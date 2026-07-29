# Stack Research: v2.0 Autonomous Orchestration

**Domain:** TUI orchestrator — autonomous `claude -p` driver, live stream visibility + mid-run injection, container session management (docker/podman)
**Researched:** 2026-07-28
**Confidence:** HIGH (CLI behavior verified by executing the local `claude` 2.1.220 binary; crate versions verified via `cargo search`/`cargo info` against the live crates.io index; bollard API verified against docs.rs)

## Context

This research covers ONLY new dependencies and external-tool behavior needed for v2.0. The existing stack is validated and not re-evaluated:

- Core: Rust, ratatui 0.30 (`crossterm` feature), crossterm 0.29 (`event-stream`), tokio 1 (`full`)
- Data: serde 1, serde_json 1, serde_yml 0.0.13, chrono 0.4
- App: anyhow 1, color-eyre 0.6, clap 4, tracing 0.1 + tracing-subscriber 0.3 + tracing-appender 0.2, regex 1, dirs 6, tempfile 3, futures 0.3
- Watch/UI: notify 8, notify-debouncer-full 0.7, tui-textarea 0.7

Already-validated capabilities reused as-is: pgrep + `/proc` session detection (`src/session_detector.rs`), tmux pane switching via `tmux list-panes -a -F` + `select-window`/`select-pane` (`src/terminal_switch.rs`), `.planning/` disk-state readers.

**Headline finding:** `tokio::process` alone is not sufficient, `--bare` is disqualified, and one crate (`bollard`) covers both docker and podman. Details below.

---

## Ground Truth: What `claude -p` Actually Supports

All claims in this section were verified by **executing the local `claude` binary (v2.1.220)** and/or reading the official docs at `code.claude.com`. Nothing here is assumed. Where the two disagree, the empirical result is noted.

### Verified flags (present in `claude --help` on 2.1.220)

| Flag | Behavior | Confidence |
|------|----------|-----------|
| `-p, --print` | Non-interactive. Workspace-trust dialog is skipped. Settings files that fail validation are **silently ignored** in this mode. | HIGH — in `--help` |
| `--output-format <text\|json\|stream-json>` | Only works with `--print`. `stream-json` = NDJSON realtime. | HIGH — in `--help`, executed |
| `--input-format <text\|stream-json>` | Only with `--print`. `stream-json` = realtime streaming **input** — this is the mid-run message injection channel. | HIGH — in `--help` |
| `--verbose` | **Required** for `stream-json` output to emit events. | HIGH — executed |
| `--include-partial-messages` | Token-level deltas; only with `--print` + `--output-format=stream-json`. | HIGH — in `--help` |
| `--replay-user-messages` | Re-emits stdin user messages on stdout for **acknowledgment**. Only with `--input-format=stream-json` + `--output-format=stream-json`. Directly solves "did my injected message land?". | HIGH — in `--help` |
| `--include-hook-events` | Emits `hook_started`/`hook_progress`/`hook_response` into the stream. | HIGH — in `--help`, observed |
| `-c, --continue` | Continue most recent conversation **in the current directory**. | HIGH — in `--help` |
| `-r, --resume [value]` | Resume by session ID or name. Without a value it opens an interactive picker — **always pass a value** in headless use. | HIGH — in `--help` |
| `--session-id <uuid>` | Caller-chosen session ID; must be a valid UUID. Lets the driver own the ID instead of scraping it. | HIGH — in `--help` |
| `--fork-session` | With `--resume`/`--continue`, branches to a new session ID instead of mutating the original. | HIGH — in `--help` |
| `--permission-mode` | Choices are exactly: `acceptEdits`, `auto`, `bypassPermissions`, `manual`, `dontAsk`, `plan`. (`manual` is an alias of `default`.) | HIGH — enumerated by `--help` |
| `--dangerously-skip-permissions` | Equivalent to `--permission-mode bypassPermissions`. | HIGH — in `--help` |
| `--allowedTools` / `--disallowedTools` | Comma/space-separated permission rules, e.g. `"Bash(git diff *) Edit"`. Space before `*` is significant. | HIGH — in `--help` + docs |
| `--setting-sources <user,project,local>` | Selects which settings tiers load. **Key mitigation** — see hook hazard below. | HIGH — in `--help` |
| `--add-dir`, `--mcp-config`, `--strict-mcp-config`, `--settings`, `--append-system-prompt`, `--system-prompt`, `--model`, `--fallback-model`, `--max-budget-usd`, `--no-session-persistence` | All present and `-p`-compatible. `--fallback-model` and `--max-budget-usd` are explicitly `--print`-only. | HIGH — in `--help` |

### Flags that work but are NOT in `--help`

| Flag | Status | Confidence |
|------|--------|-----------|
| `--max-turns <n>` | **Absent from `claude --help` on 2.1.220** but functional — executing with `--max-turns 1` produced `subtype: "error_max_turns"`. Documented in the official CLI reference. | HIGH that it works (executed); treat as **unstable/hidden API** |
| `--permission-prompt-tool <mcp_tool>` | Absent from `--help` on 2.1.220. Documented officially as the headless permission-callback mechanism. **Not verified working.** | LOW — do not build on it without a spike |

> **Roadmap implication:** don't hard-depend on `--max-turns` for loop bounding. Bound turns in the driver's own state machine and treat `--max-turns` as a belt-and-braces secondary guard.

### The `result` envelope (executed, captured verbatim)

The last NDJSON line of a `stream-json` run — and the whole body of a `--output-format json` run — is the authoritative completion record:

```json
{"type":"result","subtype":"success","is_error":false,"session_id":"...","num_turns":1,
 "terminal_reason":"completed","permission_denials":[],"result":"OK",
 "total_cost_usd":0.131564,"duration_ms":2341,"duration_api_ms":2166,
 "usage":{...},"modelUsage":{...},"api_error_status":null}
```

Observed `subtype` values: `success`, `error_max_turns`, `error_during_execution`.
Observed `terminal_reason` values: `completed`, `aborted_tools`.

Other observed stream message types, all of which the driver should model:

| `type` | Notes |
|--------|-------|
| `system` / `init` | First event (after hook events). Carries `session_id`, `model`, `tools`, `mcp_servers`, `permissionMode`, `claude_code_version`, `apiKeySource`, `capabilities[]`, `plugins[]`, and optionally `plugin_errors[]`. |
| `system` / `hook_started`, `hook_response` | Precede `init`. Carry `hook_name`, `exit_code`, `outcome`, `stderr`. |
| `system` / `api_retry` | `attempt`, `max_retries`, `retry_delay_ms`, `error_status`, and an `error` category — including `rate_limit`, `overloaded`, `authentication_failed`, `billing_error`. |
| `assistant` / `user` | Turn content. `parent_tool_use_id` is non-null for subagent messages, null for the main thread. |
| `rate_limit_event` | Observed live. Carries `rate_limit_info{status, resetsAt, rateLimitType:"five_hour", overageStatus}`. **The driver should consume this for backoff** rather than blindly retrying. |
| `stream_event` | Only with `--include-partial-messages`; contains `event.delta.text_delta`. |

Use `capabilities[]` from `system/init` for feature detection instead of parsing version strings. Observed on 2.1.220: `interrupt_receipt_v1`, `interrupt_cancel_queued_v1`, `msg_lifecycle_v1`.

### Exit codes

| Code | Meaning | Confidence |
|------|---------|-----------|
| `0` | Clean completion. Observed on a text-only run. | HIGH — executed |
| `143` | SIGTERM received. Docs: Claude aborts the in-progress turn, **terminates the process tree of any running Bash command**, runs `SessionEnd` hooks, and exits 143. This is the correct kill-switch signal. | HIGH — official docs; not independently executed |
| non-zero | Piped stdin over 10MB; invalid `--json-schema`; `--max-turns` reached. | MEDIUM — official docs |

**Exit codes beyond these are not officially documented.** The `result` envelope's `is_error` + `subtype` is a strictly richer and more reliable signal than the exit status. **Design the driver to key off the `result` message, and treat the exit code only as a liveness/crash signal.**

### ⚠ The headless hook hazard (empirically reproduced, 3/3)

This is the single most important operational finding for the v2.0 driver.

Running `claude -p` with **any tool-using prompt** on this machine hung indefinitely:

| Run | `duration_api_ms` | `duration_ms` | `subtype` | `terminal_reason` |
|-----|------------------|--------------|-----------|-------------------|
| tool + `--max-turns 1`, 240s cap | 2117 | **239008** | `error_max_turns` | `aborted_tools` |
| tool + `--max-turns 1`, 180s cap | 2414 | **179043** | `error_max_turns` | `aborted_tools` |
| tool, no turn limit, 180s cap | 2716 | **179017** | `error_during_execution` | `aborted_tools` |

`duration_ms` equals the wall-clock cap in every case while `duration_api_ms` is ~2s. The model finished in seconds; **the process then hung until an external SIGTERM arrived**, at which point it emitted the `result` line. A text-only prompt (no tools) exited cleanly in 2.3s with code 0.

Root cause identified: `~/.claude/settings.json` defines `PreToolUse` hooks, including `claudear.hooks.permission` and `rtk-rewrite.sh`, **with no `timeout` field**. Synchronous hooks run on the agent's critical path and Claude Code waits for them; hooks in `-p` mode have no controlling terminal, so a hook that expects an interactive decision blocks forever.

**Consequences for the roadmap — these are requirements, not suggestions:**

1. **The supervisor MUST enforce its own wall-clock timeout per invocation.** Never `child.wait().await` unbounded.
2. **The kill MUST be a process-group kill.** `claude` spawns Bash subprocesses; killing only the direct child orphans them. This is what forces `process-wrap` over bare `tokio::process` (see below).
3. **Escalate SIGTERM → grace period → SIGKILL.** SIGTERM is the documented clean path (exit 143, `SessionEnd` hooks run, Bash tree torn down); SIGKILL is the backstop.
4. **Constrain what the driven session loads.** `--setting-sources project` (omitting `user`) keeps OAuth working while skipping the user's global hooks. This is the recommended default for driver-launched sessions, with a config toggle to re-enable `user`.
5. The driver's "is it stuck?" heuristic should be **time since last stream event**, not time since spawn — a long legitimate `/gsd:execute-phase` emits events continuously, a hook-hung run emits nothing.

### Auth: subscription works, and one flag would break it

`system/init` reported `"apiKeySource":"none"` with no `ANTHROPIC_API_KEY` in the environment, and the run succeeded — **subscription/OAuth auth works fine under `-p`** (HIGH, executed).

The `--bare` flag is a trap. Official docs: *"Bare mode skips OAuth and keychain reads. Anthropic authentication must come from `ANTHROPIC_API_KEY` or an `apiKeyHelper`."* It is otherwise attractive (it skips exactly the hooks that caused the hang above, and the docs call it "the recommended mode for scripted and SDK calls"), which makes it an easy wrong turn. **`--bare` is disqualified by the subscription-auth constraint.** Use `--setting-sources` instead.

### Other headless behaviors worth designing around

- **`--resume` scope is directory-bound.** Session ID lookup only covers the current project directory and its git worktrees. The driver must invoke from the project root; a resume from the wrong cwd silently fails to find the session.
- **Background Bash tasks** are terminated ~5s after the final result. Background **subagents/workflows** are waited for, capped at 10 minutes by default (`CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`).
- **Slow stream consumers**: Claude waits for queued output to drain before exiting, scaled to backlog, capped at 30s. The TUI reader must not be the bottleneck — read the pipe on a dedicated task and forward via `mpsc`, never parse on the render thread.
- **Piped stdin is capped at 10MB.** Goal prompts are small; a naive "paste the whole roadmap in" would not be.
- **`dontAsk` mode** denies anything not in `permissions.allow` or the read-only command set — the right choice for dry-run and for cautious first runs. `acceptEdits` auto-approves writes plus `mkdir`/`touch`/`mv`/`cp` but **aborts the run** on an unapproved shell command.
- **Skills work in `-p`**: `/gsd:execute-phase` style invocations can be embedded directly in the prompt string and are expanded before running. This is what makes the driver viable at all.

---

## Recommended Stack Additions

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **process-wrap** | 9.1.0 | Spawn and supervise `claude -p` in a process **group**; kill the whole tree | The explicit successor to `command-group`, from the watchexec org (README states this outright). `tokio::process` can only kill the direct child — `claude` spawns Bash subprocesses, so a bare kill orphans them, which directly violates the v2.0 "Stoppable" constraint. `ProcessGroup::leader()` makes one kill reap everything. 9.7M downloads, 4.1M recent, updated 2026-03-08. MSRV 1.87. |
| **bollard** | 0.21.0 | Docker **and** Podman control | Verified on docs.rs: `Docker::connect_with_podman_defaults()` exists alongside `connect_with_local_defaults`/`connect_with_unix_defaults`/`connect_with_socket`. One typed, async client covers both runtimes, so the "auto-detect docker or podman" decision collapses into picking a constructor rather than maintaining two code paths. 42.2M downloads, 13.2M recent, updated 2026-05-04; crate keywords literally include `podman`. |
| **tokio-util** | 0.7.19 | `CancellationToken` for run cancellation; `LinesCodec` + `FramedRead` for NDJSON framing | Already a transitive dep of the tokio stack, so it is nearly free. `CancellationToken` is the canonical way to propagate "stop this run" from a TUI keypress through the supervisor, the stdout reader, and the journal writer without inventing a bespoke flag. `FramedRead<ChildStdout, LinesCodec>` gives a `Stream<Item = Result<String>>` that drops straight into the existing `tokio::select!` loop. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **uuid** | 1.24.0 | Generate the `--session-id` UUID before spawning | Pass `--session-id` rather than scraping `session_id` out of `system/init`. The driver then knows the resume key *before* the process starts, so a crash between spawn and first event is still recoverable. Use feature `v4` + `serde`. |
| **fs4** | 1.1.0 | Advisory file lock on each project's run journal | Prevents two meta-manager instances (or a stale one) from driving the same project concurrently — a real hazard given the tool is explicitly multi-project. Pure Rust, no libc, async-capable. 56M downloads. |
| **serde_json** | 1 *(already present)* | Deserialize the stream-json envelope; write the run journal | No new dep. Model the envelope as an `#[serde(tag = "type")]` enum with a `#[serde(other)]`-style catch-all so unknown future message types don't fail the run. |
| **chrono** | 0.4 *(already present)* | Journal timestamps | No new dep. Already used with the `serde` feature. |
| **tokio** | 1 *(already present, `full`)* | `process`, `io-util`, `sync::mpsc`, `time::timeout` | The `full` feature already includes `process`. No Cargo.toml change. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `podman` + `podman-docker` | Test the non-Docker path | This machine has `/usr/bin/docker` and `/var/run/docker.sock` (mode 660) but **no podman** — the podman path is currently unexercisable locally. Install `podman`, `systemctl --user enable --now podman.socket`, and verify with `curl --unix-socket /run/user/$UID/podman/podman.sock http://localhost/v1.40/version`. |
| `jq` | Manual inspection of captured NDJSON journals | Invaluable when reverse-engineering stream shapes during phase work. |
| `cargo-nextest` | Parallel test runs | Supervisor tests involve real timeouts; serial `cargo test` gets slow fast. |

## Installation

```toml
# Cargo.toml [dependencies] — additions only
process-wrap = { version = "9.1.0", features = ["tokio1"] }   # tokio1 frontend is NOT default
bollard = "0.21"
tokio-util = { version = "0.7", features = ["codec"] }
uuid = { version = "1.24", features = ["v4", "serde"] }
fs4 = { version = "1.1", features = ["tokio"] }
```

> `process-wrap` does nothing without a frontend feature — `tokio1` (or `std`) **must** be enabled explicitly. The default feature set (`process-group`, `kill-on-drop`, `job-object`, `process-session`, `creation-flags`, `tracing`) is otherwise what you want; `tracing` integrates with the existing subscriber for free.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `process-wrap` 9.1.0 | Bare `tokio::process::Command` + `kill_on_drop(true)` | Only if the child were guaranteed leaf-level. `claude` is not — it spawns Bash. If a future refactor runs `claude` inside a container for *every* case, the container boundary provides tree teardown and `tokio::process` would suffice for the `docker`/`podman` CLI invocation itself. |
| `process-wrap` | `command-group` 5.0.1 | Never for new code — same author, explicitly superseded, last released **2023-11-18**. Only relevant if you need its single unified cross-platform API and can accept the staleness. |
| `process-wrap` | `processkit` 3.0.2 | It advertises exactly the right feature set ("whole-tree kill-on-drop (no orphans), plus streaming, pipelines, timeouts, and supervision") — but it had **10,853 total downloads** and was first published **2026-07-25**, three days before this research. Revisit in a year. |
| `bollard` (API) | Shelling out to `docker`/`podman` CLI | Use the CLI for the narrow subset where it is genuinely simpler: `docker exec -it` semantics for attaching a human, and anything involving TTY allocation for the tmux pane. Recommendation is a **hybrid**: bollard for lifecycle + programmatic exec + log streaming (typed, no output parsing, no fork-per-poll), CLI shell-out only for the interactive attach path where the existing `terminal_switch.rs` pattern already applies. |
| `bollard` | `docker_credential` / `shiplift` / `docker-api` | `shiplift` is unmaintained; `docker-api` has a fraction of bollard's adoption. No reason. |
| `tokio-util` `LinesCodec` | `tokio::io::BufReader::lines()` | `BufReader::lines()` is fine and dependency-free if you don't want the codec. Use `LinesCodec` when you want a `Stream` you can `select!` over and apply `.max_length()` backpressure to; use `BufReader::lines()` if a plain `while let Some(line) = ...` loop reads better. Either is acceptable — do not add `tokio-util` solely for this if `CancellationToken` isn't also used. |
| JSON run journal | `rmp-serde` / sled / SQLite | A newline-delimited JSON journal under `.planning/meta-manager/runs/<run-id>/` matches this project's entire existing idiom (disk-readable `.planning/` state, already-working `notify` watching, human-inspectable with `jq`) and lets the *file watcher already in the codebase* drive live TUI updates for free. A binary format or embedded DB buys nothing at this scale and breaks the "read state from files" constraint. |
| `--setting-sources project` | `--settings <json>` with an empty hooks block | `--settings` is additive rather than exclusive, so it can't reliably *remove* a user hook. `--setting-sources` is the exclusion mechanism. Use `--settings` only to *add* driver-specific config. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Any Rust Anthropic API SDK** (`anthropic-sdk`, `async-anthropic`, `misanthropy`, `clust`, `anthropic-rs`, …) | Every one of these talks to `api.anthropic.com` with an `ANTHROPIC_API_KEY`. The project requires **subscription auth**, which lives in Claude Code's OAuth/keychain state and is not exposed as an API key. Adding one would silently produce a second, separately-billed code path. | Shell out to the `claude` binary. It inherits the user's subscription credentials (verified: `apiKeySource: "none"`, run succeeded). |
| **`claude --bare`** | Skips OAuth and keychain reads — auth must then come from `ANTHROPIC_API_KEY` or `apiKeyHelper`. Directly breaks subscription auth. Dangerously attractive because the docs recommend it for scripted use and it would sidestep the hook hang. | `--setting-sources project` (+ `--strict-mcp-config` if MCP noise is a problem) to get hermeticity while keeping OAuth. |
| **`command-group` 5.0.1** | Superseded by `process-wrap` by its own author; last release 2023-11-18. | `process-wrap` 9.1.0 |
| **`processkit` 3.0.2** | 10.8k downloads, published 2026-07-25. Unproven at the time of writing; wrong risk profile for the component that owns the kill switch. | `process-wrap` 9.1.0 |
| **`shared_child` 1.1.1** | Excellent crate (44M downloads) but it is a **sync/threads** primitive for sharing a `std::process::Child` across threads. It has no process-group support and no async integration — it would fight the existing tokio loop. | `process-wrap` with the `tokio1` frontend |
| **`tokio-process-stream` 0.4.1** | Only 83k downloads, thin wrapper. Merges stdout/stderr into one stream, which is the opposite of what a driver wants — stderr must stay separable for diagnostics. | `FramedRead` + `LinesCodec`, or `BufReader::lines()`, one task per pipe |
| **`tmux send-keys` as the driver transport** | Screen-scraping with no delivery confirmation, no exit code, no structured completion signal. (Already recorded as a v2.0 key decision.) | `claude -p --input-format stream-json` for injection (with `--replay-user-messages` for acknowledgment); reserve tmux for the human-attached watch pane only. |
| **Relying on the child's exit code for completion** | Empirically the process hung for 180–240s after the model finished, and only emitted `result` on SIGTERM. Exit status was `124` (from the external `timeout`), never a meaningful Claude code. | Treat the `type:"result"` NDJSON line as the completion event; use exit status only to detect crashes. |
| **`--max-turns` as the primary loop bound** | Works, but is **absent from `claude --help` on 2.1.220** — an undocumented surface that can change without notice. | Bound iterations in the driver's own state machine; keep `--max-turns` as a secondary guard. |
| **`--resume` / `-r` with no value in headless mode** | Opens an interactive picker. In `-p` with no TTY this is at best an error and at worst a hang. | Always `--resume "$SESSION_ID"`, with the ID generated by the driver via `--session-id`. |
| **`std::sync::Mutex` around driver state** | Existing project rule; still applies. Held across `.await` in a supervisor loop it deadlocks the runtime. | `tokio::sync::Mutex` / `RwLock`, or better, message passing over `mpsc` |
| **Parsing stream JSON on the render thread** | Violates the project's "never block the render loop" rule, and Claude Code caps its exit-drain wait at 30s — a slow consumer can truncate the tail of a run. | Dedicated `tokio::spawn` reader per child; parsed `DriverEvent`s forwarded over `mpsc` into the existing `tokio::select!` loop |
| **`testcontainers`** | Built for ephemeral test-fixture container lifecycles, not long-lived user-owned dev containers. (It is itself built on bollard.) | `bollard` directly |
| **`bollard-next` / `pangu-bollard` / `bollard-containerd`** | Third-party forks trailing upstream (0.18.1 / 0.14.x vs 0.21.0). | `bollard` 0.21.0 |

## Stack Patterns by Variant

**Supervising a driver run (the core loop):**
- Generate a UUID; spawn `claude -p <prompt> --session-id <uuid> --output-format stream-json --verbose --input-format stream-json --replay-user-messages --setting-sources project` via `CommandWrap::with_new(...).wrap(ProcessGroup::leader()).wrap(KillOnDrop)`.
- Keep the child's **stdin open** for the whole run — that pipe is the injection channel.
- One `tokio::spawn` per pipe (stdout NDJSON, stderr raw). Parse in the reader task, forward `DriverEvent` over `mpsc` to the app loop.
- The app's existing `tokio::select!` gains one more arm for the driver channel alongside the crossterm event stream, notify events, and the tick interval. **No change to the render architecture** — this is purely an additional message source, which is exactly what the existing design anticipated.
- Enforce two independent deadlines: a total wall-clock cap, and an **idle cap measured from the last stream event**. On breach: `CancellationToken::cancel()` → SIGTERM the group → 5–10s grace → SIGKILL the group.

**Injecting a user message mid-run:**
- Write one NDJSON user message to the child's stdin. Confirm delivery by matching the echoed message from `--replay-user-messages` — do not assume a write succeeded.
- Per the CLI reference, a message sent while Claude is working stays **queued** and runs as its own turn (v2.1.205+); on earlier versions it was discarded. Feature-detect via `capabilities[]` in `system/init` rather than version-string comparison.

**Container sessions:**
- Detect at startup: try `Docker::connect_with_local_defaults()`, then `Docker::connect_with_podman_defaults()`; cache the winner. Call `negotiate_version()` in both cases — Podman's Docker-compat endpoint reports a lower API version than bollard's generated moby 1.52 schema.
- Podman probe order (handled by bollard): `$DOCKER_HOST` (only if `unix://`) → `$XDG_RUNTIME_DIR/podman/podman.sock` → `/run/user/$UID/podman/podman.sock` → `/run/podman/podman.sock` → `/var/run/docker.sock`.
- Surface "no runtime detected" as a **disabled feature with a clear reason**, never a startup failure — the tool must stay useful on machines with neither runtime.
- Note that `session_detector.rs`'s pgrep + `/proc` approach **does not see inside containers** (different PID namespace). Container sessions need a separate discovery path via bollard's container list + exec inspect. This is a real seam the roadmap must account for, not a detail.

**Durable state across meta-manager restarts:**
- Journal per run at `.planning/meta-manager/runs/<run-id>/` with `run.json` (goal prompt, session UUID, PID, PGID, container ID, started-at, status) + `events.ndjson` (append-only raw stream).
- Take an `fs4` advisory lock on `run.json` while driving; a lock held by a live PID means "another instance owns this".
- **Re-attachment is read-only, by design.** Once the meta-manager exits, the child's stdout pipe is gone — a new instance cannot recover the live stream. On restart: read the journal to reconstruct history, check liveness via `/proc/<pid>` (reusing the existing `session_detector` primitives) plus a start-time match to defeat PID reuse, and offer either "adopt as observed" (tail the journal / attach the tmux pane) or "kill the orphan by PGID". Do not promise live re-streaming into a fresh process — plan the UX around this limit rather than discovering it mid-phase.
- Because the journal is a file under `.planning/`, the **already-working `notify-debouncer-full` watcher gives live TUI updates with no new mechanism** — including for runs started by a *different* meta-manager instance.

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| process-wrap 9.1.0 | tokio 1.x | Requires the `tokio1` feature explicitly; `std` and `tokio1` frontends are mutually exclusive in practice. MSRV **1.87.0** — highest new MSRV in this set; project currently states 1.85+, so **the project MSRV must rise to 1.87**. |
| bollard 0.21.0 | tokio 1.x, hyper-util | Default features `http` + `pipe`. Generated from moby schema 1.52; call `negotiate_version()` for Podman and older Docker daemons. |
| tokio-util 0.7.19 | tokio 1.x | Same major line as the rest of the tokio ecosystem; `codec` feature needed for `LinesCodec`. |
| uuid 1.24.0 | serde 1.x | `v4` needs `getrandom`; already satisfied transitively. |
| fs4 1.1.0 | tokio 1.x | Pure Rust, no libc. Enable the `tokio` feature for async locking; sync API is also fine here since locking happens off the render path. |
| Claude Code CLI | **≥ 2.1.211** recommended | `--forward-subagent-text` needs 2.1.211; `capabilities[]` in `system/init` needs 2.1.205; queued-message-as-own-turn needs 2.1.205; full stream drain fix needs 2.1.214. Verified against **2.1.220** locally. The driver should read `claude_code_version` from `system/init` and degrade explicitly on older versions. |
| Rust toolchain | **1.87+** | Driven by process-wrap's MSRV (processkit would have required 1.88). |

## Sources

- **`claude --version` / `claude --help` / live `claude -p` executions on 2.1.220** — flag inventory, `stream-json` envelope shape, `result` subtypes and `terminal_reason` values, `rate_limit_event`, `apiKeySource: "none"` under subscription auth, the tool-use hang (3/3 reproductions with `duration_api_ms` ≈ 2s vs `duration_ms` = wall-clock cap), `--max-turns` working while absent from `--help`. **HIGH — primary source, directly executed.**
- [Run Claude Code programmatically (headless)](https://code.claude.com/docs/en/headless) — SIGTERM → exit 143 + Bash process-tree teardown + `SessionEnd` hooks, `--bare` requiring `ANTHROPIC_API_KEY`, `stream-json` requiring `--verbose`, `system/api_retry` schema, `system/init` `capabilities`/`plugin_errors`, 10MB stdin cap, background-task grace periods, 30s exit-drain cap, `--resume` scoping to cwd + worktrees. **HIGH — official documentation.**
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — permission-mode enumeration and semantics, `--max-turns` behavior, `--fork-session`, `--permission-prompt-tool`, `--input-format` prerequisites. **HIGH — official documentation** (except `--permission-prompt-tool`, unverified locally → LOW).
- `~/.claude/settings.json` inspection — confirmed `PreToolUse` hooks (`claudear.hooks.permission`, `rtk-rewrite.sh`) registered **without `timeout`**, the root cause of the observed hang. **HIGH — direct file inspection.**
- `cargo search` / `cargo info` against the live crates.io index, plus crates.io API download counts — process-wrap 9.1.0 (9.7M/4.1M, 2026-03-08, MSRV 1.87), bollard 0.21.0 (42.2M/13.2M, 2026-05-04), command-group 5.0.1 (2023-11-18), processkit 3.0.2 (10,853 dl, 2026-07-25), shared_child 1.1.1, tokio-util 0.7.19, uuid 1.24.0, fs4 1.1.0, tokio-process-stream 0.4.1. **HIGH — primary registry data.**
- [docs.rs/bollard/0.21.0 `struct.Docker`](https://docs.rs/bollard/latest/bollard/struct.Docker.html) — `connect_with_podman_defaults` confirmed present in the generated API surface. **HIGH — generated from the published crate.**
- [process-wrap README](https://github.com/watchexec/process-wrap) — "Successor to command-group", MSRV 1.87.0, wrapper composition API. **HIGH — official repo.**
- [Podman Docker-socket compatibility](https://oneuptime.com/blog/post/2026-03-18-configure-docker-socket-compatibility-podman/view) and [testcontainers-rs configuration](https://rust.testcontainers.org/features/configuration/) — rootless/rootful podman socket setup, `DOCKER_HOST` conventions. **MEDIUM — secondary sources, but corroborated by bollard's own probe order.**
- [claude-code issue #40506](https://github.com/anthropics/claude-code/issues/40506) and community hook guides — hooks run without a controlling terminal, synchronous hooks block the critical path, hook exit-code semantics (0 allow / 2 block / 1 blocks nothing). **LOW–MEDIUM — community/issue tracker; used only as corroboration for the root cause already established empirically.**
- Local environment probe — `/usr/bin/docker` present, `/var/run/docker.sock` mode 660, **podman absent**. **HIGH — direct inspection.**

---
*Stack research for: v2.0 Autonomous Orchestration*
*Researched: 2026-07-28*
