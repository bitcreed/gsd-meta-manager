# Quick Task 261103-gt1: Live viewer with message injection and spend meter for one-off runs - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended (human unavailable). Decisions stated in the task brief are **[user]**;
decisions inferred from the code, the prior quick task (261103-f5y) and the installed CLI are
**[inferred — audit]** and must be audited later.

<domain>
## Task Boundary

Experimental (behind `GSDMM_EXPERIMENTAL_FEATURES`, Driver tab, same gating as 261103-f5y),
TUI-first. For a one-off run (`src/oneoff/`):

1. A live viewer that replays `<run-dir>/stream.jsonl` from the start and then follows it.
2. Operator message injection into the RUNNING agent (stdin in stream-json mode).
3. A spend meter (notional) + rate-limit status, and a launcher-enforced, live-adjustable soft
   cap below the fixed `--max-budget-usd` hard ceiling.
4. `task attach` (plain-text follow + stdin injection) if it falls out of the same renderer.
5. Docs (`docs/CONFIGURATION.md`) and help (`src/ui/screens/help.rs`).

Starting point: `.planning/todos/pending/2026-11-03-oneoff-live-stream-view.md` — move it to
`.planning/todos/completed/` when done.

</domain>

<decisions>
## Implementation Decisions

### A. Agent stdin / stream protocol (Claude-specific → `src/oneoff/runtime.rs` only)
- **[user]** The one-off Claude argv switches to `--input-format stream-json` +
  `--replay-user-messages` + `--forward-subagent-text`, keeping `-p`, `--output-format
  stream-json --verbose`, `--setting-sources user`, `--permission-mode bypassPermissions`,
  `--model`, `--max-budget-usd`. Flags verified present in `claude --help` of the installed
  v2.1.317 (`--replay-user-messages` "only works with --input-format=stream-json and
  --output-format=stream-json"; `--forward-subagent-text` "Forward subagent text and thinking
  blocks as assistant/user messages with parent_tool_use_id set").
- **[inferred — audit]** `--include-partial-messages`: NOT used. Token-level deltas would bloat
  `stream.jsonl` (host-written, bounded) and the viewer renders whole messages; live typing is a
  nicety. Recorded as a follow-up idea only (no todo needed unless the executor finds it cheap).
- **[inferred — audit]** The driver's `launcher::sandbox::claude_argv` and every driver golden stay
  byte-identical. The one-off argv is built by `AgentRuntime` (a sibling builder, e.g. in
  `runtime.rs` or a new `sandbox::claude_stream_input_argv`), with NO positional prompt and no
  trailing `--` prompt. The existing `the_argv_is_the_launchers_headless_command` test is
  replaced by one pinning the new argv.
- **[inferred — audit, shape verified from SDK typings, not from a live run]** One stdin line per
  user message, JSON, newline-terminated:
  `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<text>"}]},"parent_tool_use_id":null}`.
  Source: `SDKUserMessage` in `@anthropic-ai/claude-agent-sdk` 0.2.141 `sdk.d.ts` (the SDK drives
  the same CLI over `--input-format stream-json`); local copy at
  `/home/blk/projects/node/gsd-core/node_modules/@anthropic-ai/claude-agent-sdk/sdk.d.ts` L3657.
  The optional `priority: 'now'|'next'|'later'` field is NOT set (CLI default). No paid run was
  made to confirm; the e2e harness's fake claude implements this contract. Flag in SUMMARY.
- **[user]** The initial prompt becomes the FIRST stdin message, written right after spawn. Keep a
  size bound on it (reuse `MAX_PROMPT_BYTES` or an equal stdin bound) so behaviour for huge tasks
  is unchanged.
- **[inferred — audit] Session lifecycle.** In stream-json input mode `claude -p` stays alive until
  stdin EOF. The launcher must therefore close stdin to end the session the way it ends today
  (when the agent finishes its turn). Rule: on a `result` event, close stdin iff every message the
  launcher has written has already been seen replayed (a `type:"user"` replay line carrying our
  text, counted in order) BEFORE that result. A message replayed after the result means a new
  turn started → wait for the next `result`. Injections arriving after stdin is closed are refused
  and recorded as refused ("the session is ending"). Wall clock / user stop unchanged.
  The planner must make this robust against: no replay ever arriving (fallback: if a `result` is
  seen and no replay for pending writes arrives within a short settle window, e.g. 30 s, close
  stdin anyway and record a warning).
- **[inferred — audit]** Stdin plumbing: `launcher::process::spawn_launch_child` uses
  `Stdio::null()`; add an option or sibling fn that returns a piped `ChildStdin` (keep the driver's
  call byte-identical in behaviour; keep `tests/spawn_seam_guard.rs` green — extend its allowlist
  only if unavoidable and say so). `bridge` (`spawn_bridged_child`) inherits stdin, `systemd-run
  --scope` and `bwrap` inherit stdin — the executor must prove the pipe reaches the agent through
  the fake systemd-run/bwrap harness in `tests/oneoff_e2e.rs`.
- **[inferred — audit]** Stream event classification lives in the runtime seam: `AgentRuntime::
  classify(line: &str) -> StreamEvent` (a runtime-NEUTRAL enum defined outside runtime.rs, e.g.
  in `src/oneoff/stream.rs`): `AssistantText{sub,text}`, `ToolCall{sub,name,summary}`,
  `ToolResult{sub,is_error,first_line,lines}`, `UserReplay{text}` (operator or initial prompt),
  `TurnResult{subtype,cost_usd,turns}`, `RateLimit{status,kind,utilization,resets_at}`,
  `Usage{message_id,model,tokens…}`, `System{subtype}`, `Other`. `sub` = parent_tool_use_id
  present. Tool arg summary: Bash→command, Read/Edit/Write/NotebookEdit→file_path, Grep/Glob→
  pattern, Task/Agent→description, Skill→skill, else first short string field; ≤100 chars.
  One classifier drives the daemon (lifecycle + soft cap) and the viewer (rendering).

### B. Injection transport (runtime-neutral) and security
- **[inferred — audit]** Transport = a host-owned **inbox directory** `<run-dir>/inbox/` (0700),
  NOT a socket. Reasons: the driverctl socket is bound INTO the sandbox (`LaunchPaths.socket`), so
  reusing it would let the agent inject/raise its own cap; a new unix socket under the run dir
  risks the 108-byte `sun_path` limit; the daemon's wait loop already polls every 100 ms
  (`WAIT_POLL`), so an inbox needs no listener thread. Clients (TUI, CLI) write
  `inbox/<unix-nanos>-<pid>.json` atomically (tmp in `inbox/.tmp-*` + rename). The daemon,
  each tick, takes files in name order, refuses symlinks / non-regular files / files not owned by
  its uid / files > 16 KiB, processes, then deletes them.
- **[user]** Only the local operator: the run dir and `inbox/` are 0700 under the 0700 one-off
  project dir; the run dir root is NOT mounted in the sandbox (only `work/`, `report/`,
  `claude-config/`). The executor must add a test asserting no bwrap bind covers `<run-dir>/inbox`
  (the `$HOME` allowlist included). No network backends.
- **[user]** Every injection is a dispatch-like event recorded in the run dir:
  `<run-dir>/injections.jsonl` (0600, append; host-written, not mounted): `{at, kind, text|usd,
  outcome: "delivered"|"refused", reason?}`. Written BEFORE the stdin write for delivered
  messages. The launcher's own wrap-up injection is recorded with `kind:"wrap_up"`.
- **[inferred — audit]** Inbox message kinds: `{"kind":"message","text":...}` and
  `{"kind":"soft_cap","usd":<f64>|null}`. Message text: 1..=4000 chars, control chars other than
  `\n`/`\t` refused (validated by the client AND re-validated by the daemon).
- **[inferred — audit]** Client API in a runtime-neutral module (e.g. `src/oneoff/inbox.rs`):
  `send_message(run_dir, text)`, `set_soft_cap(run_dir, Option<f64>)` — refuse when the run is not
  live (lock not held by that run id).

### C. Spend meter and soft cap
- **[user]** `--max-budget-usd` stays the hard ceiling fixed at launch.
- **[inferred — audit] Estimate.** `result.total_cost_usd` is authoritative but arrives only at
  turn end, and a `/gsd-quick` run is mostly ONE long turn. Running estimate = last
  `total_cost_usd` seen + notional cost of assistant `usage` seen since (input, output,
  cache_creation, cache_read tokens × a per-model price table in `runtime.rs`, Claude-specific),
  deduplicated by `message.id` (a message's usage repeats per content block). Subagent usage is
  only partly visible → the estimate can undercount; label it "≈ est., notional" in the UI and
  docs (subscription tokens → dollars are notional). The estimator is a pure, unit-tested fold
  over `StreamEvent`s, shared by daemon and viewer.
- **[user]** Rate limits: surface the latest `rate_limit_event` (`status`, `rateLimitType`,
  `utilization` if present, `resetsAt` as local time) in the meter line.
- **[inferred — audit] Soft cap.** Default: off (none). Operator sets/changes/clears it live from
  the viewer (and `task budget`, see E). Valid: finite, > 0, < hard ceiling. The daemon holds the
  current value and persists it to `run.json` (new optional field, serde default, e.g.
  `soft_cap_usd`) plus a `soft_cap_hit_at` once reached, so the viewer shows the effective value.
- **[user, mechanism inferred — audit]** When estimate ≥ soft cap: the launcher injects ONE
  wrap-up message — "Budget reached: commit your work now, call `gsd-meta-manager taskctl report`
  with your status, then stop." — records it, and starts a grace timer (default 5 min,
  constant). If the session is still alive at grace end → stop it like a user stop but with a new
  `HostEnd::SoftCap` mapping to the existing `AgentOutcome::OverBudget` (synthesis already
  handles it). Raising/clearing the cap above the estimate during grace cancels the pending stop
  (the wrap-up message stays sent). If the soft cap makes the task too big, the executor may defer
  ONLY the soft cap to a todo and must say so.
- **[user]** Raising above the hard ceiling (stop + `--resume` with a higher ceiling) is OUT of
  scope → follow-up todo.

### D. TUI viewer
- **[inferred — audit]** Entry: `v` inside the one-off `T` popup (running or finished) opens a
  full-screen viewer overlay; also usable on a finished run (read-only). Close: `Esc` / `q`.
- **[user]** Keys consistent with the Driver tab LOG tail: `PgUp`/`PgDn` scroll, `f` follow
  toggle, `G` jump to end + follow; plus `j`/`k` line scroll **[inferred — audit]**.
- **[inferred — audit]** `i` opens a message composer (single-line input, Enter sends via the
  inbox, Esc cancels); `b` opens a soft-cap input (USD; empty clears). Both only while the run is
  live; otherwise a one-line "run is not live" notice. Inside the composer the input owns all keys.
- **[inferred — audit]** Header/meter line: run id, live/finished, elapsed, `spend ≈ $X.XX /
  soft $Y|off / hard $Z (notional)`, rate-limit status. Body: one or more lines per event, markers
  ASCII-safe, `[sub]` prefix for subagent events, `you:` for operator injections, `prompt:` for the
  first replayed message, tool results truncated (first line ≤120 chars + "(N lines)"). Every
  file-derived string through `crate::text::render_for_terminal` [user]; add the new screen to
  `src/ui/screens/render_escape_guard.rs` dispositions like 261103-f5y did.
- **[inferred — audit]** Reading: a background tail (std thread / `spawn_blocking`, never in the
  render loop; `tests/async_blocking_guard.rs` applies) with a byte cursor, bounded per line like
  the launcher's `MAX_STREAM_LINE`; replays from offset 0 then polls (~250 ms). Ring bounded by a
  named constant (e.g. 10_000 rendered lines; oldest dropped with a "… N earlier lines dropped"
  marker). The viewer's tail stops when the overlay closes.

### E. CLI
- **[inferred — audit]** `task send <project> <text>` and `task budget <project> --soft <usd>|--clear`
  (thin wrappers over the inbox client; they make the transport e2e-testable without the TUI).
- **[user, scope inferred — audit]** `task attach <project> [--run-id <id>]`: prints the same
  rendered lines (plain text, sanitized) from the start, follows until the run is finished and the
  stream drained; each stdin line becomes an injected message; `--no-input` disables stdin.
  Ship it if it falls out of the shared renderer cheaply; otherwise create a todo and say so.
  NOTE: the brief's argument is `<run-id>`; one-off commands key on the project (one live run per
  project), so `attach` takes the project like `task status` and optionally `--run-id`.

### F. Docs
- **[user]** `docs/CONFIGURATION.md` experimental one-off section: viewer keys, injection,
  `injections.jsonl`, spend meter caveats (notional, undercounts subagents), soft cap, new `task`
  subcommands. `help.rs` Driver section rows for `v` (in `T`) and the viewer keys, with its tests.

### Claude's Discretion
- Exact module/struct names, event marker glyphs, colours, the price table values (document as
  notional list prices with a date), constants (settle window, grace, ring size, poll interval).
- Test strategy: pure functions (classify, summarize args, estimator, renderer, lifecycle decision,
  inbox validation) unit-tested; the fake claude in `tests/launcher_support` extended to speak
  stream-json input (read stdin lines, replay each, emit a `result` per turn with a
  `total_cost_usd`, exit at stdin EOF) — the driver's fake behaviour (positional prompt) must keep
  working; e2e: prompt delivered via stdin, `task send` delivered + recorded + replayed in
  `stream.jsonl`, session ends on result, soft cap wrap-up + grace stop (short test grace via a
  hidden env override or option), injection refused when not live.
  No real `claude` run (no paid calls).

</decisions>

<specifics>
## Core slice vs deferred

Must ship: A (stdin mode + prompt as first message + lifecycle), B (inbox + record), D (viewer
with follow/scroll/compose), meter display (C estimate + rate limit), F.
Ship if it fits (else todo, stated in SUMMARY): soft cap enforcement (C), `task attach` (E).
Always a todo: raise above hard ceiling via stop + `--resume`.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/261103-f5y-CONTEXT.md`,
  `-PLAN.md`, `-SUMMARY.md` (seam guard, run-dir layout, sandbox mounts, e2e harness).
- `.planning/todos/pending/2026-11-03-oneoff-live-stream-view.md` (starting todo).
- `src/oneoff/runtime.rs`, `src/oneoff/launch.rs` (wait loop, `run_session`), `src/oneoff/store.rs`,
  `src/launcher/process.rs` (`spawn_launch_child`, `pump_redacted`), `src/launcher/sandbox.rs`
  (`claude_argv`, `bwrap_argv_with`, `RunDirMode::ReportWritable`), `src/launcher/bridge.rs`,
  `src/ui/screens/oneoff.rs`, `src/ui/screens/driver.rs` + `driver_run.rs` (LOG tail keys, ring),
  `src/ui/screens/mod.rs` (`DRIVER_OUTPUT_RING_LINES`), `src/ui/screens/help.rs`,
  `src/ui/screens/render_escape_guard.rs`, `src/text.rs` (`render_for_terminal`),
  `tests/oneoff_e2e.rs`, `tests/launcher_support/`, `tests/oneoff_runtime_seam_guard.rs`,
  `tests/spawn_seam_guard.rs`, `tests/async_blocking_guard.rs`.
- `claude --help` (v2.1.317): `--input-format`, `--replay-user-messages`,
  `--forward-subagent-text`, `--include-partial-messages`.
- `@anthropic-ai/claude-agent-sdk` 0.2.141 `sdk.d.ts`: `SDKUserMessage` (L3657),
  `SDKUserMessageReplay` (L3678), `SDKRateLimitEvent`/`SDKRateLimitInfo` (L3267),
  `SDKResultMessage` (`total_cost_usd`), `SDKAssistantMessage` (`parent_tool_use_id`).

</canonical_refs>
