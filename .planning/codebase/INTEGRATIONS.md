---
last_mapped_commit: d7381ab615e5181d33f3b6a5e02cf4d2a42594af
last_mapped_at: 2026-11-03
---
# External Integrations

**Analysis Date:** 2026-11-03

## APIs & External Services

**Anthropic Inference API:**
- Host: `api.anthropic.com` (constant `INFERENCE_HOST` in `src/config.rs` and `src/launcher/egress/mod.rs`); path `/v1/messages`.
- Used by: the sandboxed headless driver (`claude -p`), which talks to the API only through the launcher's egress proxy.
- SDK/Client: none in-process. The `claude` CLI is the client; the crate reaches the host with its own TLS stack (`rustls` 0.23, `tokio-rustls` 0.26, `webpki-roots` 1) in `src/launcher/egress/upstream.rs`.
- Auth: `claude setup-token` token stored in the app config directory (`driver-oauth-token`, 0600), or the host Claude OAuth access token (`claudeAiOauth.accessToken`) with a minimum-lifetime check (`src/launcher/auth.rs`). The refresh token is never copied into a run.
- Egress policy: the sandbox has no network. The launcher terminates the driver's HTTP requests on a unix socket, allows only `api.anthropic.com`, injects the real token, and re-originates the request (`src/launcher/egress/policy.rs`, `src/launcher/egress/http1.rs`, `src/launcher/egress/log.rs`). Tests run against a plain-TCP stub.

**Claude Code CLI (local process):**
- Binary: `claude` on `PATH`.
- Invocations: `claude -p --output-format stream-json` for driver and one-off runs (`src/launcher/sandbox.rs`, `src/oneoff/`); `claude setup-token` for operator token setup (`src/driver_cli.rs`).
- Session detection: reads Claude session state under `CLAUDE_CONFIG_DIR` or `~/.claude` (`src/agents/adapters/claude.rs`, `src/session_detector.rs`).

**Codex CLI (local, best-effort):**
- Session detection under `$CODEX_HOME` or `~/.codex` (`src/session_detector.rs`). Codex GSD installs are read from `~/.codex/gsd-core` or `$CODEX_HOME/gsd-core`.

**GSD (Get Shit Done) planning system (local files):**
- The app reads `.planning/` directories of each registered project: `STATE.md`, `ROADMAP.md`, `config.json`, `QUEUE.md`, `HANDOFF.json`, phase and plan files, driver frontmatter (`src/state_reader/`, `src/registry.rs`).
- Reads the installed gsd-core per project and per runtime to show version and install status (`src/state_reader/gsd_install.rs`).
- No network calls; this is a file-based integration.

**Git (local process):**
- All repository operations go through the `git` binary: status, branch and worktree inspection, merge and diff for one-off runs, and resolver worktrees (`src/state_reader/git_ops.rs`, `src/oneoff/merge.rs`, `src/oneoff/land.rs`, `src/agents/worktrees.rs`). Remote is used only as a string in project metadata, e.g. `https://github.com/...` in tests.

**Gitleaks (optional local scanner):**
- Binary `gitleaks`, invoked from `src/envelope/scan.rs` (`GITLEAKS_BIN`) to scan external output before it is recorded. Rule id `gitleaks-external`.

**Terminal and multiplexer:**
- `tmux` for focusing and switching to running project sessions (`src/terminal_switch.rs`, `src/agents/processes.rs`). `TMUX` env var is consulted.
- `xdg-terminal-exec` (and `TERMINAL`, `VISUAL`, `EDITOR`) for opening external terminals and editors.

## Data Storage

**Databases:**
- None. No database server or ORM. All persistent state is JSON, YAML, Markdown, or plain text on disk.

**File-based persistence:**
- App config: `~/.config/gsd-meta-manager/config.json` (`src/config.rs`); schema-versioned, with read-side migration in memory.
- App data: `dirs::data_local_dir()/gsd-meta-manager/` - logs, envelope ledger (`src/envelope/mod.rs`), one-off run store (`src/oneoff/store.rs`) keyed by `hash8(canonical project path)`, and driver run directories with `launcher.lock`, `state.json`, and `LOG.md` (`src/launcher/`, `src/driverctl/store.rs`).
- Project state: each registered project's `.planning/` directory is read-only from the app's view except for the driver and one-off flows, which write run artifacts.
- Advisory locks: `flock` via `rustix` on run and store lock files (`src/driver/lock.rs`, `src/oneoff/store.rs` `oneoff.lock`, `src/oneoff/land.rs` `land.lock`, unlinked on every release).
- One-off run directory files: `stream.jsonl`, `launch.log`, `run.json`, `injections.jsonl`, a copy of `SUMMARY.md`, and an `inbox/` directory of `<nanos>-<pid>.json` injection requests (`src/oneoff/inbox.rs`, `src/oneoff/session.rs`). Slimmed (recycled) runs keep only tails of `stream.jsonl`/`launch.log` (`src/oneoff/recycle.rs`).

**File Storage:**
- Local filesystem only. No cloud object storage.

**Caching:**
- None detected. The UI re-reads state on filesystem change events.

**Sockets:**
- Unix domain socket for the driver control channel (`DRIVERCTL_SOCK`, `src/driverctl/serve.rs`) and the egress server socket (`src/launcher/egress/mod.rs`).

## Authentication & Identity

**Auth Provider:**
- No application-level auth provider and no user accounts.
- Driver inference auth: Anthropic OAuth token, obtained through `claude setup-token` or the host Claude credential file, as described above (`src/launcher/auth.rs`).
  - Token is passed only in the child process environment; it is never placed in argv, `state.json`, `LOG.md`, or launch logs (`src/launcher/auth.rs`, `child_env` in `src/launcher/sandbox.rs`).
  - `auth` posture is recorded as a word (e.g. `setup-token`), never the value.

**Sandbox identity:**
- Per-run `CLAUDE_CONFIG_DIR` seeded by the launcher (`src/launcher/config_dir.rs`), so the driver does not share the host's Claude config.

## Monitoring & Observability

**Error Tracking:**
- None. No Sentry or similar.

**Logs:**
- `tracing` with `tracing-subscriber` (`env-filter`) and a daily-rolling `tracing-appender` file named `gsd-meta-manager.log*` under `dirs::data_local_dir()/gsd-meta-manager/` (`src/main.rs`).
- Per-run logs: `LOG.md` and launch logs under the run directory; driver output captured as `stream-json` in a file (`src/launcher/sandbox.rs`).
- Egress request log: `src/launcher/egress/log.rs`.
- Redaction before writing: `src/journal/redact.rs` is the capture-path redaction seam applied before journal and run output is recorded.

## CI/CD & Deployment

**Hosting:**
- Source: GitHub, `https://github.com/bitcreed/gsd-meta-manager` (`Cargo.toml` `repository`).
- Distribution: crates.io (`cargo publish --locked`).

**CI Pipeline:**
- GitHub Actions, `.github/workflows/release.yml`, triggered on tags matching `v[0-9]+.[0-9]+.[0-9]+`.
  - `msrv`: compiles all targets on Rust 1.89.0.
  - `cross-target-check`: `cargo check` and `cargo clippy -D warnings` for `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-gnu`.
  - `publish`: `cargo test --no-fail-fast`, `cargo clippy -- -D warnings`, `cargo publish --locked`, with Node 24 installed via `actions/setup-node@v7` for the conformance oracle.
- Runners pinned to `ubuntu-24.04`.
- Actions used: `actions/checkout@v7`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache@v2`, `actions/setup-node@v7`.

**Secrets used by CI:**
- `CARGO_REGISTRY_TOKEN` (crates.io publish token), supplied via `secrets.CARGO_REGISTRY_TOKEN` in the publish step only.

## Environment Configuration

**Required for normal TUI use:**
- None. The app runs with defaults; `--config <PATH>` is optional.

**Required for driver runs (Linux):**
- A `claude` CLI on `PATH`, plus either a stored `claude setup-token` token or a logged-in host Claude session.
- `bwrap` and `systemd-run --user` for the sandbox; otherwise the run needs `--unsandboxed` and is recorded as `sandbox: degraded`.

**Optional:**
- `gitleaks` on `PATH` for external-output scanning.
- `tmux` for session focus.
- `CLAUDE_CONFIG_DIR`, `CODEX_HOME` to point at non-default agent config locations.
- `GSDMM_ONEOFF_IDLE_SETTLE_SECS` optionally overrides the one-off idle-settle wait (`IDLE_SETTLE_ENV` in `src/oneoff/runtime.rs`).

**Secrets location:**
- Driver OAuth token: app config directory, file `driver-oauth-token` (mode 0600, owner-checked). Host Claude credentials are read in place and never copied.
- CI: GitHub repository secret `CARGO_REGISTRY_TOKEN`.
- No `.env` files are tracked.

## Webhooks & Callbacks

**Incoming:**
- None. The app exposes no HTTP server. The only listeners are local unix sockets (driver control and egress), reachable only from the sandboxed child.

**Outgoing:**
- Anthropic inference requests to `api.anthropic.com:443` over TLS, issued only by the launcher egress server for sandboxed driver runs (`src/launcher/egress/upstream.rs`).
- No other outbound network calls were found in `src/`. Test fixtures reference hosts such as `example.invalid` and `github.com` only as strings.

---

*Integration audit: 2026-11-03*
