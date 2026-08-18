# External Integrations

**Analysis Date:** 2026-08-18

This project has no network-service integrations (no HTTP APIs, no databases, no cloud SDKs). Its "external integrations" are subprocess contracts with local CLI tools and a set of on-disk file formats owned by another project (GSD). Each is described below with the actual protocol/format observed in the code.

## APIs & External Services

**Agent driving (the primary integration):**
- `claude` CLI, driven as a long-lived subprocess over a duplex NDJSON stream — `src/executor/claude.rs`, protocol model in `src/executor/stream_json.rs`
  - Invocation: `claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages --session-id <uuid> --setting-sources <value> --permission-mode <mode> --strict-mcp-config [--model ...] [--resume ...] [--max-budget-usd ...] [--name ...]` — argv built by `build_argv()` in `src/executor/claude.rs`
  - Transport: three independent Tokio tasks (stdout reader, stderr reader, stdin writer) plus a supervisor/coordinator task; stdin/stdout/stderr are `Stdio::piped()`, never merged
  - Wire protocol: NDJSON, one JSON object per line, framed with `tokio::io::BufReader` plus an explicit `MAX_LINE_BYTES` (4 MiB) bound — no codec crate (`tokio-util`) is used by design
  - Message shapes modeled in `src/executor/stream_json.rs`: `system` (`init`, plus absorbed catch-all subtypes like `hook_started`, `thinking_tokens`), `assistant`/`user` turn messages, `result` (closes a **turn**, not the run), `control_response` (interrupt acknowledgement), `rate_limit_event` (carried unmodeled)
  - Outbound messages: `UserMessage` (`{"type":"user","message":{...}}`) written to the child's stdin; `ControlRequest::interrupt` for turn interrupts (request/response correlated by `request_id`)
  - Teardown: SIGTERM to the whole process group first, then a grace period, then SIGKILL — never plain `kill()` (which would skip Claude's clean-shutdown path)
  - Environment: every inherited `CLAUDE*` variable is scrubbed before spawn; `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` is set explicitly afterward
  - Auth: none handled by this binary — the CLI use its own already-configured auth (OAuth/subscription or API key); `system/init`'s `apiKeySource` field is read only to detect `"none"` (subscription auth) for a regression guard

**Detached driver spawn (a separate, adjacent integration):**
- `gsd-meta-manager drive ...` — this same binary re-invoked as its own subcommand (`std::env::current_exe()`), spawned detached from the TUI process — `src/driver/spawn.rs`
  - Argv always carries `--config <path>`, `drive <alias> --command <cmd> --run-id <id> [--goal <text>]`
  - Detached via: new process group (`process_group(0)`), three null stdio handles, `kill_on_drop(false)` (explicit, load-bearing — closing the TUI must not kill the run), plus a reaping Tokio task to avoid zombies
  - Never spawns via a worktree; always the project root as cwd

## Data Storage

**Databases:**
- None. All persistent state is plain files.

**File Storage:**
- Local filesystem only. Two categories:
  - **GSD project state, read-only**: `.planning/config.json`, `.planning/STATE.md` (YAML frontmatter + Markdown), `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, `QUEUE.md`/backlog files — parsed by `src/state_reader/*` submodules (`config_json.rs`, `state_md.rs`, `roadmap_md.rs`, `queue_md.rs`, `backlog.rs`, `workstreams.rs`, `disk_status.rs`, `git_ops.rs`)
  - **This tool's own state, read/write**: user config at a platform-resolved path (`dirs::config_dir()` + `gsd-meta-manager/config.json`, e.g. `~/.config/gsd-meta-manager/config.json` on Linux) written atomically via `tempfile::NamedTempFile` + rename (`src/config.rs`); the append-only run journal `journal.jsonl` (NDJSON, one record per line, monotonic `seq`, no per-event fsync) — `src/journal/writer.rs`, `reader.rs`, `redact.rs`, `inbox.rs`

**Caching:**
- In-process only: parsed project state is cached and invalidated on filesystem-watch events (per `CLAUDE.md`'s intended `Arc<RwLock<HashMap<...>>>` pattern); no external cache service.

## Authentication & Identity

**Auth Provider:**
- None owned by this binary. The `claude` CLI's own authentication (subscription/OAuth or API key) is used as-is; this tool only reads `system/init.apiKeySource` to confirm no `--dangerously-skip-permissions`/bypass path silently activated (D-08 regression guard in `src/executor/claude.rs`).

## Monitoring & Observability

**Error Tracking:**
- None (no Sentry/Bugsnag/etc.)

**Logs:**
- `tracing` + `tracing-subscriber` + `tracing-appender`, routed to a file under the user's local data directory (not stdout, since ratatui owns the terminal)
- Explicit discipline: raw subprocess stream content and journal event content are never logged — only counts/shapes (documented at `src/executor/claude.rs` and `src/journal/mod.rs`, "D-28"/"D-33")

## CI/CD & Deployment

**Hosting:**
- Distributed as a compiled binary; no hosted service component. `README.md`/`CLAUDE.md` describe cargo-based release builds (`cargo build --release --target ...`) and mention `cargo-dist` as a candidate for release artifacts.

**CI Pipeline:**
- GitHub Actions present under `.github/workflows/` (build/test/lint on push/PR — inspect that directory directly for exact jobs)

## Environment Configuration

**Required env vars:**
- None required for basic operation
- Consulted when relevant: `$VISUAL` / `$EDITOR` (config-edit key, falls back to `vi`), `$TMUX` (gates tab-switching support), inherited `CLAUDE*` vars are actively stripped rather than consulted

**Secrets location:**
- None stored by this binary. No `.env`, no credentials file, no keychain integration observed.

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None (no network calls of any kind found in `src/`)

## Local CLI Tool Contracts (the real integration surface)

Beyond `claude` itself, several other local commands are shelled out to synchronously or via Tokio's `process` module — these are the project's actual "integrations":

- **`git`** — read-only repository introspection: last-commit timestamp (`src/state_reader/git_ops.rs:12` and others), disk/branch status, and a diff-based snapshot before/after a driven run (`src/executor/outcome.rs:661`, `src/journal/writer.rs:365`); also `git init` when scaffolding a new project (`src/project_creator.rs:47`)
- **`tmux`** — `list-panes`, `select-window`, `select-pane` to switch terminal focus onto a live Claude session's pane (`src/terminal_switch.rs`); no-ops with a clear error when `$TMUX` is unset (i.e., not running inside tmux); other terminals (Kitty, Ghostty, WezTerm) are explicitly not yet supported
- **`pgrep`** — detects already-running `claude` processes and their controlling TTY for session discovery (`src/session_detector.rs`)
- **`sh -c <cmd>`** — executes user-configured `hooks.pre_create` / `hooks.post_create` shell commands during project creation, with `GSD_PROJECT_NAME`/`GSD_PROJECT_PATH`/`GSD_PROJECT_ALIAS` in the environment (`src/project_creator.rs`, `src/config.rs`)
- **`$EDITOR`/`$VISUAL` or `vi`** — opens files for editing, suspending the TUI (`src/main.rs`, `src/app.rs`)
- **A queue-defined external launcher** — `queue_md.rs` builds a `Command` for `<launcher> <args...>` from data read out of `QUEUE.md`, i.e., the launcher program itself is configuration-driven, not hardcoded

---

*Integration audit: 2026-08-18*
