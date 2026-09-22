# Quick 260922-hdj: Support Codex as well as Claude as the agent runtime - Research

**Researched:** 2026-09-22
**Domain:** Rust executor/driver seam; OpenAI Codex CLI 0.155.1 non-interactive (`codex exec --json`) protocol
**Confidence:** HIGH for the Codex CLI facts (every one was probed live against the installed binary), MEDIUM for the seam design (it is inferred from the code)

<user_constraints>
## User Constraints (no CONTEXT.md; taken from the quick-batch task and todo)

- Add a runtime abstraction behind `src/executor/` that covers the spawn argv, the output-stream parser and session/liveness detection (`src/session_detector.rs`), and use it from `src/driver/`.
- **Claude behavior must stay byte-identical**: the same argv, env, parsing and outcomes.
- The runtime can be chosen per project or globally in config.
- If the work is too large, ship a **minimum viable slice** (the abstraction plus a working Codex executor) and file the rest as a new pending todo flagged for audit.
- The human is unavailable. Decisions are inferred and marked `[INFERRED]` for later audit.
- Project constraints from CLAUDE.md: work read-only against GSD state, stay non-intrusive and portable, use a Rust 1.88 MSRV, avoid `std::sync::Mutex` in async code, send logs to a file only, and don't use `--dangerously-*` flags (D-15 house rule: `PermissionMode` has no bypass variant).
</user_constraints>

## Summary

Codex's non-interactive mode is **one prompt, one turn per process, NDJSON on stdout**: `codex exec --json [flags] "<prompt>"`. There is no stdin message protocol, no interrupt control channel and no USD cost. The Claude executor's extra features (the duplex `send` for steering, `interrupt`, replay-echo acks, the capability gate, `--settings`/PreToolUse envelope, `--json-schema` model seam) **have no Codex equivalent**. So the Codex executor is a strict subset: `start` / `cancel` / `is_running`. The existing `Executor` trait doc already anticipates this (`src/executor/mod.rs:82-87`: *"A hypothetical Codex or aider backend satisfies `start`, `cancel` and `is_running` and nothing else"*).

The cheapest seam that leaves Claude untouched has four parts:
1. an `AgentRuntime { Claude, Codex }` enum;
2. a new `src/executor/codex.rs` with its own argv builder, spawn closure and line parser;
3. reuse of the existing supervisor/teardown helpers in `claude.rs` through **visibility-only** changes, plus one new `ReaderItem` arm;
4. an `AgentExecutor` enum in the driver that replaces the concrete `ClaudeExecutor` type.

Codex `turn.completed` / `turn.failed` are mapped onto the existing `ResultMessage` / `TurnCompleted` path, so `derive_run_outcome_from_envelopes` (disk delta, exit code) is reused unchanged.

**Primary recommendation:** Ship the MVP as a runtime enum, a `CodexExecutor` (single-command iterations, no steering or interrupt), a per-project `runtime` field on the registry entry with Claude as the default, `/gsd-x` → `$gsd-x` translation at the executor boundary, and `pgrep -x codex` session detection. File the remainder (TUI launch/resume, steering, model seam, AGENTS.md opt-in disclosure, network/envelope parity, runtime picker UI) as a new pending todo.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary | Rationale |
|---|---|---|---|
| Runtime selection | `config.rs` (registry entry) | `driver/run.rs` reads it | The per-project opt-in record already lives on `RegisteredProject` |
| Argv + env for the child | `executor/codex.rs` (new) | — | The one-spawn-seam rule (`tests/spawn_seam_guard.rs` allowlist) |
| Stream parsing → `ExecutionEvent` | `executor/codex_json.rs` (new) | `executor/claude.rs` supervisor | Parse in the reader task, never the render thread |
| Command-syntax translation | `executor/codex.rs::start` | — | The driver, bounds, journal and router all key on canonical `/gsd-…` strings; translate only at the last hop |
| Liveness / sessions | `session_detector.rs` | `registry.rs` auto-register | The `/proc` scan is already there |

## Codex CLI facts (probed live, codex-cli 0.155.1)

All runs used a scratchpad temp dir with a trivial prompt and `--ephemeral` where possible. Transcripts are quoted verbatim.

**Invocation:** `codex exec [OPTIONS] [PROMPT]`. The prompt is a positional argument. `-C/--cd <DIR>` sets the working root. `--json` means *"Print events to stdout as JSONL"*. `-m/--model`, `-s/--sandbox <read-only|workspace-write|danger-full-access>`, `--add-dir <DIR>` (*"Additional directories that should be writable alongside the primary workspace"*), `--skip-git-repo-check`, `--ephemeral` (*"Run without persisting session files to disk"*), `--ignore-user-config`, `--output-schema <FILE>` (file path only), `-o/--output-last-message <FILE>`. **`codex exec` has no `-a/--ask-for-approval` flag**; that flag exists only on the top-level interactive command. [VERIFIED: `codex exec --help`]

**Resume:** `codex exec resume [OPTIONS] [SESSION_ID] [PROMPT]`, where *"Conversation/session id (UUID) or thread name. UUIDs take precedence"*, or `--last`. Resume does **not** accept `-s/--sandbox` or `-C` (they are absent from its help), so the sandbox is whatever `-c`/config supplies. [VERIFIED: `codex exec resume --help`]

**Stdin pitfall:** with stdin inherited (not a TTY, not closed), `codex exec` printed `Reading additional input from stdin...` to stderr and **blocked until the 180 s timeout** (exit 124, zero stdout lines). Help: *"If stdin is piped and a prompt is also provided, stdin is appended as a `<stdin>` block"*. With `< /dev/null` it ran normally. **The child must be spawned with `Stdio::null()`.** [VERIFIED: probe]

**Event schema, success run (`-s read-only`, prompt: run `cat note.txt`, reply DONE), exit 0:**
```
{"type":"thread.started","thread_id":"01a0ca2f-8a0c-72c0-87fb-11c28739d140"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"I’ll read the requested file now."}}
{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"/usr/bin/bash -lc 'cat note.txt'","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"/usr/bin/bash -lc 'cat note.txt'","aggregated_output":"hello\n","exit_code":0,"status":"completed"}}
{"type":"item.completed","item":{"id":"item_2","type":"agent_message","text":"DONE"}}
{"type":"turn.completed","usage":{"input_tokens":36912,"cached_input_tokens":18176,"cache_write_input_tokens":0,"output_tokens":116,"reasoning_output_tokens":0}}
```
**Failure run (`-m no-such-model-xyz`), exit 1:**
```
{"type":"thread.started","thread_id":"01a0ca2f-c735-7202-936b-f2ffba031bdb"}
{"type":"item.completed","item":{"id":"item_0","type":"error","message":"Model metadata for `no-such-model-xyz` not found. Defaulting to fallback metadata; this can degrade performance and cause issues."}}
{"type":"turn.started"}
{"type":"error","message":"{\"type\":\"error\",\"status\":400,\"error\":{\"type\":\"invalid_request_error\",\"message\":\"The 'no-such-model-xyz' model is not supported when using Codex with a ChatGPT account.\"}}"}
{"type":"turn.failed","error":{"message":"{\"type\":\"error\",\"status\":400, ...}"}}
```
[VERIFIED: probe] Notes:
- An `item.completed` with `item.type == "error"` is a **non-fatal warning**. It arrives *before* `turn.started`.
- The top-level `error` is followed by `turn.failed`.
- `usage` has **no cost field**.
- The docs list the item families as *"agent messages, reasoning, command executions, file changes, MCP tool calls, web searches, and plan updates"*. [CITED: learn.chatgpt.com/docs/non-interactive-mode]
- The item stream is **not a complete tool log**: in one `workspace-write` probe a sandbox-denied `git commit` produced no `command_execution` item at all, only the agent's text quoting the failure. [VERIFIED: probe] Nothing may be derived from the absence of items.

**Exit codes observed:** 0 for success; 1 for `turn.failed`; 1 for a bad `-C` path (`Error: No such file or directory (os error 2)`, nothing on stdout); 1 for a non-git dir without `--skip-git-repo-check` (`Not inside a trusted directory and --skip-git-repo-check was not specified.`, nothing on stdout). No `thread.started` means the error is only on stderr. [VERIFIED: probe]

**Sandbox vs GSD commits (critical):** `-s workspace-write` makes **`.git` read-only**. Probe output was `fatal: Unable to create '…/.git/index.lock': Read-only file system`. Adding **`--add-dir <root>/.git`** made `git commit` succeed (`[master fa4e24f] probe`). Inside `workspace-write`, child env has `CODEX_SANDBOX_NETWORK_DISABLED=1`, so network is off (no `git push`/`gh`), which works as an envelope for free. The default `codex exec` sandbox is read-only. [VERIFIED: probe; CITED: learn.chatgpt.com/docs/non-interactive-mode]

**Child env set by codex:** `CODEX_VERSION`, `CODEX_SESSION_ID`, `CODEX_THREAD_ID`, `CODEX_CI=1`, `CODEX_SANDBOX_NETWORK_DISABLED=1`. If the TUI itself runs inside a codex shell, these leak into a driven child. So scrub `CODEX_*` **except `CODEX_HOME`**. [VERIFIED: probe; the scrub list is `[INFERRED]`]

**GSD skills under Codex:** commands are skills at `~/.agents/skills/gsd-<cmd>/SKILL.md`, *"invoked by mentioning `$gsd-execute-phase`"*. A live `codex exec --json -s read-only '$gsd-help'` loaded `/home/blk/.agents/skills/gsd-help/SKILL.md` and `~/.codex/gsd-core/workflows/help.md` and printed GSD help, so **`$gsd-…` mentions resolve in exec mode**. [VERIFIED: probe] GSD's own translator (`~/.codex/gsd-core/bin/lib/runtime-slash.cjs` `formatGsdSlash`) strips `^[/$]?gsd[-:]`, **lowercases only the command token**, keeps the argument tail verbatim, and emits `$gsd-${token.toLowerCase()}${tail}` for codex. [VERIFIED: read runtime-slash.cjs]

**Sessions on disk:** `~/.codex/sessions/YYYY/MM/DD/rollout-<YYYY-MM-DDTHH-MM-SS>-<thread_id>.jsonl`. The first line is `{"type":"session_meta","payload":{"id":…,"cwd":…,"cli_version":"0.155.1","originator":…,"thread_source":…,"git":{…}}}`. There is also a `~/.codex/session_index.jsonl` (`{"id","thread_name","updated_at"}`) and sqlite stores (`state_5.sqlite`, `thread_history_1.sqlite`). [VERIFIED: read files]

**Liveness:** a live `codex exec` has `comm == "codex"` and `/proc/<pid>/cwd` equal to the `-C` dir. It **holds its rollout file open**: `/proc/<pid>/fd/*` → `…/sessions/2026/09/22/rollout-2026-09-22T12-39-37-01a0ca33-a532-7022-8768-84a2cf77932f.jsonl`, whose suffix equals the `thread.started` `thread_id`. That fd is the robust session-id source; argv carries none for new sessions. **Pitfall:** the sandbox helper *also* has `comm == "codex"` (pstree: `codex(321347)-+-bwrap(322574)---codex(322579)`, with cmdline `codex-linux-sandbox --sandbox-policy-cwd …`), so `pgrep -x codex` over-matches. Filter on `argv[0]` basename `== "codex"`. [VERIFIED: probe] Whether the **interactive** TUI holds the rollout fd itself or delegates to the `app-server` daemon (`~/.codex/app-server-daemon/` exists) was not probed. [ASSUMED]

**Runtime keys GSD already has:** `~/.gsd/defaults.json` on this machine contains `"runtime": "codex"`. This repo's `.planning/config.json` has no `runtime` key. GSD resolves `env.GSD_RUNTIME > <project>/.planning/config.json "runtime" > install marker (<gsd-core>/.gsd-runtime) > 'claude'` (`runtime-slash.cjs` `resolveRuntime`/`resolveExplicitRuntime`). `~/.codex/gsd-core/.gsd-runtime` = `codex`, `~/.claude/gsd-core/.gsd-runtime` = `claude`. [VERIFIED: read files]

## How Claude is wired today (the seam points)

- **Argv:** `src/executor/claude.rs:239-325` `build_argv` emits `-p --input-format stream-json --output-format stream-json --verbose --replay-user-messages --session-id <uuid> --setting-sources <…> --permission-mode <…> --strict-mcp-config` plus optional `--model/--resume/--max-budget-usd/--name/--disallowedTools/--settings`. It also emits `--tools "" --json-schema` for `SpawnProfile::ModelSeam`. [VERIFIED: claude.rs:248-261]
- **Spawn:** `ClaudeExecutor::start_run` (`claude.rs:451-711`) uses `CommandWrap` + `ProcessGroup::leader()` + `KillOnDrop`, scrubs `CLAUDE*`, applies `EnvelopeEnv`, sends the prompt over **stdin** after the gate or `prompt_release_grace`, and publishes the pgid to `observing_spawn`.
- **Parse:** `read_stdout` → `parse_line` (`stream_json.rs:498`) → `ReaderItem::Envelope` → `handle_item` (`claude.rs:1697-1886`), which maps onto `ExecutionEvent` (`mod.rs:673-754`). `StreamMessage::Result` is pushed to `envelopes: Vec<ResultMessage>` and forwarded as `TurnCompleted`. The outcome is `derive_run_outcome_from_envelopes(&envelopes, exit, before, after)` (`outcome.rs:185`).
- **Driver:** `build_executor(args) -> ClaudeExecutor` (`driver/run.rs:1763-1774`) is constructed per iteration, then `.observing_spawn(pgid_tx).observing_replay_echoes(echo_tx)` (`run.rs:3114-3116`). `deliver_pending_inbox(executor: &ClaudeExecutor, …)` (`run.rs:1515`) and the shutdown helpers take the **concrete** type. Options come from `iteration_options` (`run.rs:1826-1839`: the envelope deny-list and settings path are always filled).
- **Router commands:** these are canonical slash strings, e.g. `pub const COMMAND_EXECUTE_PHASE: &str = "/gsd-execute-phase";` (`driver/router.rs:294`). They are also consumed by `bounds.rs`, `goal.rs`, `journal`, and the UI.
- **Sessions:** `session_detector.rs:43-44` `Command::new("pgrep").args(["-x", "claude"])`, then `/proc/<pid>/{cwd,cmdline,stat,fd/0}`. The session id comes only from `--resume`/`--session-id` argv (`:219-286`). The struct is `ClaudeSession { pid, session_id: Option<Untrusted>, working_dir, start_time, tty }` (`:18-27`). Consumers are `registry.rs:877` `auto_register_from_sessions`, `app.rs:1056`, `main.rs:544`, and the detail/Sessions UI.
- **Config:** `RegisteredProject { path, added, #[serde(default)] driver_opt_in, #[serde(flatten)] extra }` (`config.rs:31-64`). `Preferences { hooks, gsd_integration, driver_max_concurrent, #[serde(flatten)] extra }` (`config.rs:281-310`). `CONFIG_SCHEMA_VERSION: u32 = 2`, and the test at `config.rs:609` asserts that adding `#[serde(default)]` fields is not a schema change.

## Recommended seam (MVP)

```
src/executor/runtime.rs   NEW  AgentRuntime { #[default] Claude, Codex } (serde lowercase)
                               + fn codex_command(cmd:&str)->String  ("/gsd-x a" -> "$gsd-x a")
src/executor/codex_json.rs NEW  CodexEvent serde enum (tag="type"): thread.started{thread_id},
                               turn.started, item.started/updated/completed{item:Value},
                               turn.completed{usage:Value}, turn.failed{error:{message}},
                               error{message}, #[serde(other)] Unknown; parse_codex_line(&str)
src/executor/codex.rs     NEW  #[cfg(unix)] CodexExecutor + build_codex_argv(options, prompt)
src/executor/claude.rs    EDIT visibility only (pub(super)) on: read_bounded_line/BoundedLine,
                               read_stderr, forward, report_dropped, terminate_group,
                               tear_down_group, finish_teardown, await_clean_exit,
                               capture_snapshot, Coordinator (+ one ReaderItem::Codex arm
                               and a `protocol` field so prompt_released starts true for Codex)
src/executor/mod.rs       EDIT pub mod codex/codex_json/runtime; ExecutionEvent::AgentOutput
                               {stream:String,text:String}; SendError::UnsupportedByRuntime
src/driver/run.rs         EDIT enum AgentExecutor{Claude(ClaudeExecutor),Codex(CodexExecutor)}
                               implementing Executor by delegation + observing_* builders;
                               build_executor picks by resolved runtime; &ClaudeExecutor
                               params -> &AgentExecutor
src/config.rs             EDIT RegisteredProject.runtime: Option<AgentRuntime>
                               (#[serde(default, skip_serializing_if="Option::is_none")]);
                               Preferences.default_runtime: Option<AgentRuntime> (same attrs)
src/session_detector.rs   EDIT AgentSession{runtime,…} (keep ClaudeSession as type alias);
                               add pgrep -x codex pass with argv0-basename filter +
                               rollout-fd session id
```

**Codex argv (MVP), in this order** [INFERRED from the probes]:
`exec --json -s workspace-write --add-dir <root>/.git -C <root> [-m <model>] -- <translated prompt>`.
- Use `Stdio::null()` for stdin, keep `ProcessGroup::leader()`, and use the same SIGTERM → grace → SIGKILL teardown.
- Put `--` before the prompt, because a prompt that starts with `-` (e.g. a user goal) must not parse as a flag. [ASSUMED: clap honours `--` here; verify in a unit/fixture test]
- For `resume_session: Some(id)` emit `exec resume --json … <id> -- <prompt>`, but resume rejects `-s`/`-C`, so use `-c sandbox_mode="workspace-write"` and rely on `current_dir(root)`. [ASSUMED key name `sandbox_mode`, matching the `config.toml` schema; verify with `--strict-config`]
- The MVP never emits `--dangerously-bypass-approvals-and-sandbox`, `--dangerously-bypass-hook-trust` or `danger-full-access`. Guard this with a test like `the_argv_never_carries_a_permission_bypass`.

**Event mapping (codex → existing types):**

| Codex line | ExecutionEvent | Notes |
|---|---|---|
| first `thread.started` | gate passes → `GateOutcome{session_id:Some(thread_id), capabilities:vec![], claude_code_version:None, api_key_source:None, permission_mode:None, permission_mode_confirmed:false}` + `SessionStarted` | `GateOutcome` fields quoted from `gate.rs:135-156`; no capability check exists for Codex |
| `turn.completed` | push synthesized `ResultMessage{subtype:"success", is_error:false, terminal_reason:Some("completed"), session_id, ..}` then `TurnCompleted` | `ResultMessage` is `Deserialize` with every field `#[serde(default)]` (`stream_json.rs:283-362`), so build it via `serde_json::from_value(json!{…})` or add a `pub(crate)` constructor. No `Cost` event |
| `turn.failed` | same, with `subtype:"error_during_execution"`, `is_error:true`, `errors:[error.message]` | `outcome.rs` keeps unknown strings verbatim |
| `item.*`, `error` | new `AgentOutput{stream:"codex:<item.type>"|"codex:error", text}` | +1 arm in `journal/mod.rs:2153` `from_exec_event`, +1 in `app.rs:569` |
| unparseable / oversize | the existing `Unparseable` / `LineTruncated` | reuse `read_bounded_line` (4 MiB bound) |

With no `thread.started` before exit, the gate channel drops and the run ends as `SpawnError::InitNeverObserved`; stderr lines are journaled as they are today.

**Driver behaviour under Codex (MVP):**
- Each iteration is one `codex exec`, and the process exits after its turn.
- `send` returns `SendError::UnsupportedByRuntime`, so the inbox is not delivered and is journaled as undelivered.
- `interrupt` returns the same error. `close_input` is a no-op `Ok`: keep a writer channel whose receiver a trivial task drains.
- `SpawnProfile::ModelSeam` (`--goal` decomposition, escalation) under Codex returns a typed refusal before spawn.
- The envelope's `--settings`/`--disallowedTools` have no Codex carrier. The network-off `workspace-write` sandbox plus `EnvelopeEnv` (SSH_AUTH_SOCK/GIT_CONFIG_GLOBAL scrub) are the MVP envelope, and this gap is **flagged for audit**.

**Runtime resolution (MVP)** [INFERRED]: `registry entry .runtime` > `preferences.default_runtime` > **Claude**. Do **not** auto-adopt `.planning/config.json` `runtime` or `~/.gsd/defaults.json` `runtime` in the MVP. On this machine `defaults.json` says `codex`, so auto-adoption would silently flip projects created after it off Claude, which breaks "Claude byte-identical". Showing a mismatch hint (GSD says codex, the manager says claude) belongs to the remainder.

### MVP vs remainder

**MVP (this quick task):** `runtime.rs`, `codex_json.rs` (plus fixture tests built from the transcripts above), `codex.rs` (argv builder tests plus a `fake-codex.sh` fixture replaying the JSONL through `with_program`), the claude.rs visibility changes and the codex reader arm, `AgentExecutor` in the driver, the config fields, the `/gsd-` → `$gsd-` translation, the codex process detection in `session_detector` (with an `AgentSession.runtime` field), and the `SPAWN_ALLOWLIST` entry for `src/executor/codex.rs`.

**Remainder → new pending todo, `[AUDIT]`:**
1. TUI launch/resume: `detail.rs` `claude_launch_args`/`claude_resume_args` would become `codex` / `codex resume <id>`, and would need its own CWE-88 fusion analysis, since `codex resume` takes a positional id, not a fused `--resume=`.
2. Steering/inbox for Codex, via `codex exec resume <thread_id> "<msg>"` between turns or the `app-server` protocol.
3. The model seam via `--output-schema <tempfile>`.
4. Opt-in disclosure of `AGENTS.md` (Codex reads it the way Claude reads `CLAUDE.md`; `registry.rs:565` discloses only `CLAUDE.md`), because consent drift is a security gap.
5. Envelope parity through Codex hooks (`features.hooks = true`, `~/.codex/hooks.json`) or `-c` sandbox policy.
6. Worktree-aware `--add-dir` (`git rev-parse --git-common-dir`).
7. A runtime picker in the TUI config screen, plus a mismatch hint against the GSD `runtime` key.
8. `state_reader/queue_md.rs:80-91` also resolving `~/.codex/gsd-core/bin/gsd-tools.cjs`.
9. A Codex version gate (the minimum tested version is 0.155.1).
10. Codex token usage in the journal.

## Common Pitfalls

1. **Inherited stdin blocks `codex exec` forever.** Always use `Stdio::null()` (the probe shows the hang).
2. **`.git` is read-only under `workspace-write`.** Without `--add-dir <root>/.git`, every GSD commit fails while the agent reports prose "success". The disk delta then shows changes but no commits. Worktrees keep their real git dir elsewhere.
3. **`pgrep -x codex` matches the sandbox helper** (`codex-linux-sandbox`, same comm) and possibly `codex app-server` daemons. Filter on argv[0] basename, and skip the `app-server`/`exec-server`/`mcp` subcommands.
4. **Guard tests scan `src/` text.** `tests/spawn_seam_guard.rs::every_process_spawn_site_in_src_is_on_the_allowlist` requires `codex.rs` in `SPAWN_ALLOWLIST`. The same file has a no-bypass-flag scan and a strict-unknown-field scan. `session_detector.rs::CLAUDE_ARGV_SITES` counts `--resume`/`-r`/`--session-id` literals **per file**, so don't put those option literals in `codex.rs`, or adjudicate them. Run `cargo test --no-fail-fast` (see memory: fail-fast hides the envelope suites).
5. **Don't rename `ClaudeSession`/`claude_code_version`** in the MVP. They are referenced from `registry.rs`, `app.rs`, `action.rs`, the UI and the tests. Use a type alias, or add a `runtime` field.
6. **Translate commands only at the executor boundary.** `bounds.rs` repeat detection, `goal.rs` validation and the journal all compare canonical `/gsd-…` strings. Lowercase only the command token, as `formatGsdSlash` does.
7. **An `item.type == "error"` item is a warning, not a failure.** Only `turn.failed` or the exit code classify the run.
8. **`codex exec resume` rejects `-s` and `-C`.** Sandbox must come via `-c`.
9. **Untrusted/non-git dirs exit 1 with nothing on stdout.** Surface stderr in the spawn-failure diagnostic (the existing `InitNeverObserved` path journals stderr lines).

## Security Domain

| ASVS | Applies | Control |
|---|---|---|
| V5 Input validation | yes | The prompt goes as one argv element after `--`, with no shell (`CommandWrap`). The thread id from the rollout path is wrapped in `Untrusted`, like the Claude session id |
| V4 Access control | yes | `DrivableProject` capability token unchanged. `workspace-write` plus `.git` only, network off, and no `danger-*` flags |
| V6 Crypto | no | — |

| Threat | STRIDE | Mitigation |
|---|---|---|
| Argument injection via prompt/goal (CWE-88) | Tampering | `--` terminator + a test with a `-`-leading prompt |
| Env leak of `CODEX_THREAD_ID`/`CODEX_SANDBOX_*` from a parent codex | Info disclosure / Tampering | Scrub `CODEX_*` except `CODEX_HOME`, keep the `CLAUDE*` scrub, apply `EnvelopeEnv` |
| Undisclosed `AGENTS.md` prompt input | Spoofing (consent) | Remainder item 4. Flag the MVP as not covering it |
| Envelope parity gap (no PreToolUse hook) | Elevation | Network-off sandbox. Flag for audit |

## Environment Availability

| Dependency | Available | Version |
|---|---|---|
| codex | yes, `/home/blk/.local/bin/codex` → `~/.codex/packages/standalone/current/bin/codex` (static ELF) | 0.155.1 |
| claude | yes | 2.1.280 |
| pgrep, git, node | yes | — |

No new crates are needed (serde_json, tokio, process-wrap and uuid are already dependencies), so there is no package legitimacy audit. Tests must not call the real `codex`: use a `tests/fixtures/fake-codex-*.sh` that replays the JSONL above, following the pattern of `fake-claude-echo.sh`.

## Assumptions Log

| # | Claim | Risk if wrong |
|---|---|---|
| A1 | `--` before the positional prompt is honoured by `codex exec` (clap) | A prompt starting with `-` would be parsed as a flag. Needs a unit/fixture test, plus one real probe during execution |
| A2 | `-c sandbox_mode="workspace-write"` is the right key for `exec resume` | Resume would run read-only. Remainder only |
| A3 | The interactive codex TUI holds its rollout fd itself (not via the daemon) | TUI-session ids would come back `None`; liveness by cwd still works |
| A4 | The `CODEX_*` scrub list (keep only `CODEX_HOME`) | Could drop a var codex needs (e.g. `CODEX_CA_CERTIFICATE`), so keep that one too |
| A5 | [INFERRED] Resolution order: registry > preference > Claude, ignoring the GSD `runtime` key | Users expecting GSD's key to drive selection must set it in the manager as well |

## Open Questions

1. Should the MVP expose runtime selection in the TUI, or only through `config.json` editing? Recommendation: config-file only plus a `--runtime` override on `drive` (debug-gated like `--claude-program`?). `[INFERRED]` Use config-file only; do the UI in the remainder.
2. Should goal-driven runs be allowed under Codex by keeping the model seam on Claude? Recommendation: refuse under Codex in the MVP, so that one Codex project never silently needs Claude auth.

## Sources

- Live probes of `/home/blk/.local/bin/codex` 0.155.1: `--help`, `exec --help`, `exec resume --help`, `features list`, 6 `exec --json` runs in a scratchpad dir, and a `/proc` inspection of a live run (HIGH).
- `~/.codex/gsd-core/bin/lib/runtime-slash.cjs`, `~/.agents/skills/gsd-execute-phase/SKILL.md`, `~/.gsd/defaults.json`, `~/.codex/sessions/**/rollout-*.jsonl` (HIGH).
- Repo: `src/executor/{mod,claude,stream_json,gate,outcome}.rs`, `src/driver/run.rs`, `src/session_detector.rs`, `src/config.rs`, `src/journal/mod.rs`, `tests/spawn_seam_guard.rs` (HIGH).
- [learn.chatgpt.com/docs/non-interactive-mode](https://learn.chatgpt.com/docs/non-interactive-mode): event/item families, default read-only sandbox, resume (MEDIUM).

**Valid until:** about 14 days (Codex CLI ships frequently; re-probe on version change).
