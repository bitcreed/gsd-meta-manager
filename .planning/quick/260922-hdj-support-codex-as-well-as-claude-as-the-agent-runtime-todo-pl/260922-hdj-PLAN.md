---
phase: quick-260922-hdj
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/executor/runtime.rs
  - src/executor/codex_json.rs
  - src/executor/codex.rs
  - src/executor/mod.rs
  - src/executor/claude.rs
  - src/executor/stream_json.rs
  - src/error.rs
  - src/config.rs
  - src/journal/mod.rs
  - src/driver/run.rs
  - src/driver/mod.rs
  - tests/spawn_seam_guard.rs
  - tests/async_blocking_guard.rs
  - tests/driver_codex_runtime.rs
  - tests/fixtures/fake-codex.sh
  - tests/fixtures/codex/README.md
  - tests/fixtures/codex/01-exec-success.jsonl
  - tests/fixtures/codex/02-exec-failure.jsonl
  - .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md
autonomous: true
requirements: [QUICK-260922-hdj]

must_haves:
  truths:
    - "A project whose manager registry entry carries the key runtime=codex is driven by `codex exec`: the child argv is exactly `exec --json -s workspace-write --add-dir <root>/.git -C <root> [-m <model>] -- <prompt>` with the canonical `/gsd-x args` rewritten to `$gsd-x args` ONLY at the executor boundary, and the child's stdin is /dev/null."
    - "Codex JSONL reaches the existing pipeline: the first thread.started passes the start gate and becomes SessionStarted (session id = thread_id), turn.completed / turn.failed become synthesized ResultMessage turns consumed by the UNCHANGED derive_run_outcome_from_envelopes, and item.* / error lines become ExecutionEvent::AgentOutput journaled as exec_event records with codex:* stream labels."
    - "Runtime resolution is registry entry `runtime` > preferences `default_runtime` > Claude. GSD's own runtime keys (.planning/config.json, ~/.gsd/defaults.json) are never read. An unrecognized value is a typed refusal raised before anything is created."
    - "A project with no runtime key spawns byte-for-byte the pre-change Claude argv, env handling and parsing: every pre-existing test passes unmodified and a new end-to-end test pins the Claude argv shape spawned through `drive`."
    - "Under Codex, the model seam, resume_session and budget_usd are typed pre-spawn refusals (zero processes launched), and send/interrupt return SendError::UnsupportedByRuntime so injected messages are journaled undelivered rather than lost."
    - "The Codex child never receives a permission-bypass flag or a full-access sandbox, never inherits CLAUDE* or CODEX_* variables (except CODEX_HOME and CODEX_CA_CERTIFICATE), and every Codex run journals a runtime_envelope_partial diagnostic before its first exec record."
    - "The unshipped remainder (session/liveness detection, TUI launch/resume, steering, model seam, AGENTS.md disclosure, envelope parity, worktree git dir, runtime picker UI, and the rest) is filed as a new pending todo flagged [AUDIT]."
  artifacts:
    - path: "src/executor/runtime.rs"
      provides: "AgentRuntime {Claude, Codex}, resolution, codex_command translation, #[cfg(unix)] AgentExecutor dispatch enum"
      contains: "pub enum AgentExecutor"
    - path: "src/executor/codex_json.rs"
      provides: "CodexEvent serde model, parse_codex_line, CodexStreamState::step -> CodexStep"
      contains: "pub fn parse_codex_line"
    - path: "src/executor/codex.rs"
      provides: "build_codex_argv (pure), CodexExecutor implementing Executor over the shared Coordinator"
      contains: "impl Executor for CodexExecutor"
    - path: "tests/driver_codex_runtime.rs"
      provides: "end-to-end drive() tests through the fake-codex fixture, including the Claude argv pin"
      min_lines: 150
    - path: "tests/fixtures/fake-codex.sh"
      provides: "argv/stdin/env-logging stand-in that replays a codex JSONL transcript"
    - path: ".planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md"
      provides: "the remainder todo, area: driver, flagged [AUDIT]"
      contains: "area: driver"
  key_links:
    - from: "src/driver/mod.rs"
      to: "src/executor/runtime.rs"
      via: "drive() resolves the runtime after DrivableProject::from_registry and stamps it onto the token with with_runtime"
      pattern: "with_runtime\\("
    - from: "src/driver/run.rs"
      to: "src/executor/runtime.rs"
      via: "build_executor(args, project.runtime()) returns AgentExecutor at both call sites (iteration spawn and consult_model_seam)"
      pattern: "build_executor\\(args, project\\.runtime\\(\\)\\)"
    - from: "src/executor/codex.rs"
      to: "src/executor/claude.rs"
      via: "CodexExecutor constructs the shared Coordinator with protocol CodexExecJson and feeds it ReaderItem::Codex items"
      pattern: "ReaderItem::Codex"
    - from: "src/executor/claude.rs"
      to: "src/executor/outcome.rs"
      via: "synthesized codex ResultMessage turns are pushed onto the same envelopes Vec that derive_run_outcome_from_envelopes reads"
      pattern: "derive_run_outcome_from_envelopes\\(&envelopes"
    - from: "tests/spawn_seam_guard.rs"
      to: "src/executor/codex.rs"
      via: "SPAWN_ALLOWLIST entry plus the capability-type assertion"
      pattern: "src/executor/codex.rs"
---

<objective>
Ship the minimum viable Codex runtime slice defined by the research and the batch constraints: an `AgentRuntime` abstraction behind `src/executor/`, a working `CodexExecutor` (`codex exec --json`), a Codex JSONL parser mapped onto the existing turn/outcome types, driver dispatch through an `AgentExecutor` enum, and per-project / global runtime selection in the manager config. Claude stays byte-identical. Everything that does not fit is filed as a new pending todo flagged for audit.

Purpose: the manager can drive projects whose user runs GSD under OpenAI Codex, which the CLAUDE.md "portable, any GSD user" constraint calls for. Claude users must see no change at all.

Output: three new executor modules, the driver wiring, a JSONL fixture set with a logging stand-in, end-to-end tests (including a Claude argv pin), guard-list updates and a remainder todo.

Sibling overlap: none. 260922-hdh (state_reader/mod.rs, app.rs, ui/screens/normal.rs, delete_confirm.rs) and 260922-hdi (ui/screens/mod.rs, detail.rs) touch no file in `files_modified`. `src/app.rs`'s `apply_exec_event` has a catch-all arm, so the new event variant needs no edit there.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/STATE.md
@.planning/quick/260922-hdj-support-codex-as-well-as-claude-as-the-agent-runtime-todo-pl/260922-hdj-RESEARCH.md
@.planning/todos/pending/2026-09-22-support-codex-as-well-as-claude-as-the-agent-runtime.md

Seam anchors (read these ranges, not whole files — claude.rs is 2501 lines, run.rs is 5417):
- src/executor/mod.rs:88-132 (Executor trait), 160-257 (DrivableProject), 381-547 (ExecutionOptions), 565-669 (WriterCommand, ExecutionHandle), 673-754 (ExecutionEvent)
- src/executor/claude.rs:239-325 (build_argv, the Claude pin), 331-450 (ClaudeExecutor builders), 451-711 (start_run — the spawn closure and Coordinator construction to mirror), 848-1016 (ReaderItem, BoundedLine, read_bounded_line, read_stdout, read_stderr), 1042-1186 (Coordinator fields and the head of run), 1697-1886 (handle_item)
- src/executor/gate.rs:135-156 (GateOutcome fields)
- src/executor/stream_json.rs:282-362 (ResultMessage), 478-495 (Envelope)
- src/error.rs:43-227 (SpawnError, SendError), 588-870 (DriveError)
- src/config.rs:31-64 (RegisteredProject), 281-310 (Preferences)
- src/journal/mod.rs:2153-2208 (from_exec_event, the only exhaustive ExecutionEvent match)
- src/driver/mod.rs:830-841 (drive head, from_registry), 1172-1180 (goal decomposition call)
- src/driver/run.rs:214 (DEFAULT_AGENT_PROGRAM), 509-575 (spawn_failure_label / spawn_failure_diagnostic — exhaustive SpawnError matches), 1248-1256 and 1515-1522 (&ClaudeExecutor params), 1661-1774 (agent_program, journal_agent_program_override, build_executor), 1990-2013 (consult_model_seam), 2724-2727 (argv digest), 3106-3165 (per-iteration spawn), 4836 and 4935 (in-module tests that pass ClaudeExecutor::new())
- tests/driver_tracer.rs (the end-to-end harness shape to copy), tests/fixtures/fake-claude-echo.sh (fixture style)
- tests/spawn_seam_guard.rs:45-105 (SPAWN_ALLOWLIST), 479-487 (capability-type assertion); tests/async_blocking_guard.rs:211-258 (ASYNC_BLOCKING_ALLOWLIST)
- src/session_detector.rs:1177-1300 (option-literal census: needles `--resume`, `-r`, `--session-id` counted per file under src/)
</context>

<assumption_delta_decision>
Signal: a second agent runtime (Codex) appears where the design had exactly one (Claude). Primary noun: the agent runtime.
- Driver layer: PROMOTE. The driver stops naming the concrete `ClaudeExecutor` type and programs against `AgentExecutor` (an enum over both backends that implements `Executor`), and the `DrivableProject` token carries the resolved `AgentRuntime`. Claude becomes one variant of the general noun.
- Names outside the driver's executor type (`ClaudeSession`, `claude_program`/`claude_args` on DriveArgs, `claude_code_version`, the journal's `claude_pgid`): ADD-ALONGSIDE, accepted debt. Renaming them would break "Claude byte-identical" on serialized/journaled field names and touch files owned by sibling items. What forces the promotion: Codex session detection landing in session_detector.rs (remainder item 1). The remainder todo records this.
- Suggested invariant test (adopted in Task 2): "a project with no runtime key round-trips through `drive` onto the unchanged Claude argv", which goes red the moment a future change makes Codex (or GSD's own runtime key) the silent default.
</assumption_delta_decision>

<api_coverage>
The external surface is the installed `codex` CLI (0.155.1). The default is full coverage. Every opt-out below carries its reason and is carried into the remainder todo.

| capability | decision | reason |
|---|---|---|
| `codex exec <prompt>` one-shot run | INTEGRATE | |
| `--json` JSONL event stream | INTEGRATE | |
| `-C/--cd <root>` | INTEGRATE | |
| `-s workspace-write` sandbox | INTEGRATE | read-only default cannot write GSD artifacts; the full-access value is forbidden by the D-15 house rule |
| `--add-dir <root>/.git` | INTEGRATE | without it every GSD commit fails read-only (research Pitfall 2) |
| `-m/--model` | INTEGRATE | plumbed from ExecutionOptions.model |
| `codex exec resume <id>` / `--last` | OPT-OUT | not needed yet: no driver caller sets resume_session, and the `-c sandbox_mode` key is unverified (A2). This is a typed refusal. Remainder item |
| `--output-schema <file>` | OPT-OUT | not needed yet: the model seam is refused under Codex in the MVP. Remainder item |
| `-o/--output-last-message` | OPT-OUT | not needed: agent_message items already carry the text |
| `--ephemeral` | OPT-OUT | not needed: session files are kept for the future detection/resume work |
| `--skip-git-repo-check` | OPT-OUT | explicitly out of scope: GSD projects are git repos and a `.git` directory is required |
| `-c` overrides, `-p` profile, `--ignore-user-config` | OPT-OUT | not needed yet: the user's own codex config (auth, provider) must apply |
| `-i` images | OPT-OUT | not needed |
| interactive TUI launch / `codex resume` | OPT-OUT | not needed yet: remainder item (detail.rs launch/resume, the hdi-owned file) |
| app-server protocol (mid-run steering) | OPT-OUT | not needed yet: remainder item |
| codex hooks (`features.hooks`) | OPT-OUT | not needed yet: envelope parity remainder item |
| on-disk sessions / session_index / sqlite | OPT-OUT | not needed yet: session detection remainder item |
| `turn.completed.usage` token counts | OPT-OUT | not needed yet: journaling usage is a remainder item |
| `mcp` / `mcp-server` / `app-server` subcommands | OPT-OUT | explicitly out of scope |
| permission-bypass flags, full-access sandbox | OPT-OUT | explicitly out of scope and forbidden (D-15 house rule, guarded by tests) |
</api_coverage>

<inferred_decisions>
The human is unavailable. Each decision below was inferred from the artifacts and MUST be copied into the SUMMARY under "Inferred decisions (for audit)".
- ID-1: Runtime is stored under the existing `#[serde(flatten)] extra` maps (entry key `runtime`, preferences key `default_runtime`) and read through typed accessors. It is not a new typed struct field. Reasons: a typed field breaks about 30 `RegisteredProject { .. }` literals across 28 files (several owned by sibling items), and an unrecognized string in a typed field would make config.json fail to load, so the TUI would not start. With the accessor, an unrecognized value is a typed refusal at drive time and round-trips untouched.
- ID-2: Resolution is entry > preference > Claude. GSD's `runtime` key is never read. On this machine `~/.gsd/defaults.json` says codex, so auto-adoption would silently move Claude projects off Claude (research A5).
- ID-3: Codex session/liveness detection in src/session_detector.rs is DEFERRED to the remainder. Adding codex processes to `detect_sessions()` would make the Sessions tab offer `claude --resume=<codex thread id>`, because detail.rs launch/resume is Claude-only and detail.rs belongs to 260922-hdi. It would also change auto-registration. Driven-run liveness and the kill switch are runtime-agnostic (pgid published by observing_spawn, run-id liveness), so stopping a driven Codex run already works.
- ID-4: `<root>/.git` must be a directory. A worktree or submodule (`.git` is a file) and a missing `.git` are refused before spawn, so commits never fail silently.
- ID-5: Under Codex, resume_session, budget_usd and SpawnProfile::ModelSeam are refused (typed, zero spawns). name, setting_sources, permission_mode and bg_wait_ceiling_ms are ignored because they are Claude-only. The envelope deny list and the settings path have no Codex carrier, and each run discloses this in a journal diagnostic.
- ID-6: The model-seam refusal reaches the user through the existing `SeamAnswer::Unusable` → goal-seam refusal path, with no new pre-dry-run refusal.
- ID-7: Only prefixed commands are translated (case-insensitive `/gsd-`, `/gsd:`, `gsd:`, `$gsd-`). Unprefixed text passes verbatim. This deviates from GSD's formatGsdSlash, which treats a bare word as a command name, because the executor's input is a prompt.
- ID-8: The Codex child scrubs `CLAUDE*` and `CODEX_*` except `CODEX_HOME` and `CODEX_CA_CERTIFICATE` (research A4).
- ID-9: The resolved runtime rides on the DrivableProject token (`with_runtime`). It is not threaded through the dispatch/execute_run/decompose/consult_model_seam signatures, which keeps the spawn_seam_guard ordering scanners untouched.
- ID-10: This item does not move the original todo to completed/. The remainder todo names it as superseded, and the coordinator decides whether to close it.
</inferred_decisions>

<tasks>

<task type="tracer">
  <name>Task 1: End-to-end "drive a runtime=codex project through codex exec" — registry key to journal, one path</name>
  <files>src/executor/runtime.rs, src/executor/codex_json.rs, src/executor/codex.rs, src/executor/mod.rs, src/executor/claude.rs, src/executor/stream_json.rs, src/error.rs, src/config.rs, src/journal/mod.rs, src/driver/run.rs, src/driver/mod.rs, tests/spawn_seam_guard.rs, tests/async_blocking_guard.rs, tests/fixtures/fake-codex.sh, tests/fixtures/codex/README.md, tests/fixtures/codex/01-exec-success.jsonl, tests/fixtures/codex/02-exec-failure.jsonl, tests/driver_codex_runtime.rs</files>
  <read_first>The context anchors above. Also RESEARCH.md sections "Codex CLI facts", "Recommended seam (MVP)" and "Common Pitfalls" (1, 2, 4, 5, 6, 7). Before editing, capture the baseline by running `rtk proxy cargo test --no-fail-fast` once and recording the passed/failed/ignored totals. Expect exactly one local failure, the git-version witness in src/envelope/policy.rs. Also record the `rtk proxy cargo clippy --all-targets` warning count.</read_first>
  <action>
Wire ONE path end to end: a registry entry with runtime=codex, then `drive`, then `AgentExecutor::Codex`, then `codex exec` (the fixture), then the shared Coordinator, then the journal and outcome. Real error handling on that path. Nothing on the Claude path changes behaviour.

1. `src/executor/runtime.rs` (new, portable). Define `AgentRuntime { #[default] Claude, Codex }` deriving Debug, Clone, Copy, PartialEq, Eq, Default, with `as_str()` returning `claude`/`codex` and `program()` returning the default program name, which is the same pair. Add `from_config_value(&serde_json::Value) -> Result<AgentRuntime, UnrecognizedRuntime>`: exact lowercase `claude` or `codex` strings only, and any other string or non-string is refused. `UnrecognizedRuntime` carries `value: crate::text::Untrusted` (the rendered value) and `key: &'static str` (which config key it came from), and implements Display naming both the key and the known values. Add `resolve(project: Option<AgentRuntime>, global: Option<AgentRuntime>) -> AgentRuntime` = project, else global, else Claude (per ID-2). Its doc must state that GSD's own runtime keys are deliberately never consulted. Add `codex_command(canonical: &str) -> String` per ID-7: when the input starts with a case-insensitive `/gsd-`, `/gsd:`, `gsd:` or `$gsd-` prefix, split the remainder at its first whitespace, lowercase ONLY the command token, keep the tail byte-for-byte, and emit `$gsd-` + token + tail. Anything else, including a prefix-only degenerate input, is returned verbatim. Under `#[cfg(unix)]`, add `pub enum AgentExecutor { Claude(ClaudeExecutor), Codex(CodexExecutor) }` with `new(runtime)`, `with_program(runtime, program, leading_args)`, and `observing_spawn(tx)` / `observing_replay_echoes(tx)` builders delegating to the variant; the Codex arm of the echo builder drops the sender, since Codex has no replay echo. It also implements `Executor` by pure delegation (all six methods) and gets a `runtime()` accessor.

2. `src/executor/codex_json.rs` (new, portable). Define `CodexEvent` as a serde internally tagged enum on `type` with renamed variants `thread.started {thread_id: String}`, `turn.started`, `turn.completed {usage: Option<Value>}`, `turn.failed {error: Option<{message: Option<String>}>}`, `item.started` / `item.updated` / `item.completed {item: serde_json::Value}`, `error {message: Option<String>}`, and a `#[serde(other)] Unknown` unit variant. `parse_codex_line(&str) -> Result<CodexEvent, serde_json::Error>`. `CodexStreamState { thread_id: Option<String> }` with `step(&mut self, CodexEvent) -> CodexStep` where `CodexStep` is `ThreadStarted { thread_id }` (records it), `TurnEnded(Box<ResultMessage>)`, `Output { stream: String, text: String }` or `Unknown`. The mappings are:
   - turn.completed becomes a ResultMessage with subtype `success`, is_error false, terminal_reason `completed` and session_id set to the recorded thread id.
   - turn.failed becomes subtype `error_during_execution`, is_error true, `errors` set to the failure message (when present) and the same session_id.
   - turn.started becomes Output with stream `codex:turn.started` and empty text.
   - A top-level error becomes Output with stream `codex:error` and text set to message.
   - An item event becomes Output with stream `codex:<phase>:<item_type>`, where phase is started, updated or completed. item_type is the item's `type` string only when it matches `[a-z0-9_]{1,32}`, and is `other` otherwise, because it is wire data and becomes a journal label. text is the item's `text` string, else its `message` string, else its `command` string followed by ` (exit N)` when exit_code is a number, else the compact JSON of the item.
   - Unknown becomes Unknown.

   An `item.completed` whose item.type is `error` is a warning and never classifies the run (research Pitfall 7). Build the synthesized ResultMessage with `..Default::default()`: add `#[derive(Default)]` to `ResultMessage` in src/executor/stream_json.rs, a one-line change that leaves deserialization byte-identical. If a field type lacks Default, fall back to `serde_json::from_value` of a literal inside `step`, returning `CodexStep::Unknown` on the (unreachable) error rather than panicking. Add `gate_outcome_for_thread(thread_id: &str) -> GateOutcome` with session_id Some(thread id), empty capabilities, and None/false for every Claude-only field. Unit tests in this module (TDD, written first) run over the two fixture files through `include_str!`. They cover: success yields ThreadStarted, then outputs, then TurnEnded(success) with the right session id; failure yields a TurnEnded(error_during_execution, is_error true) carrying the message; the pre-turn `item.type=error` warning maps to Output and not to a failure; an unknown type is Unknown; a torn line is a parse Err; and a hostile item.type such as a string with ESC or 200 chars labels as `other`.

3. `src/executor/claude.rs`: VISIBILITY-ONLY, plus exactly two additive pieces. Make these `pub(super)`: `ReaderItem`, `BoundedLine`, `read_bounded_line`, `read_stderr`, `capture_snapshot`, `EVENT_CHANNEL_CAPACITY`, `WRITER_CHANNEL_CAPACITY`, `Coordinator` and every one of its fields, and `Coordinator::run`. Additive piece (a): a `pub(super) enum StreamProtocol { ClaudeStreamJson, CodexExecJson }` plus a `protocol` field on Coordinator. Claude's start_run sets ClaudeStreamJson. In `run`, `prompt_released` starts as `matches!(protocol, StreamProtocol::CodexExecJson)`: the Codex prompt rides argv, so there is nothing to release and the grace timer never arms. It is therefore false for Claude exactly as today. Additive piece (b): a `ReaderItem::Codex { raw: String, step: CodexStep }` variant and its arm in `handle_item`:
   - ThreadStarted while not yet gated: set gated, send `Ok(codex_json::gate_outcome_for_thread(..))` on gate_tx, and forward SessionStarted (session_id = thread id, empty capabilities, empty claude_code_version, None auth/mode). It writes NOTHING to the writer.
   - ThreadStarted after gating: forward Unknown{raw}.
   - TurnEnded: push the clone onto `envelopes`, then forward TurnCompleted. There is no Cost event, because Codex reports no cost.
   - Output: forward the new AgentOutput.
   - Unknown: forward Unknown{raw}.

   Do not touch build_argv, the spawn closure, read_stdout or any existing arm. Claude never produces ReaderItem::Codex.

4. `src/executor/mod.rs`:
   - Declare `pub mod codex_json; pub mod runtime;` and `#[cfg(unix)] pub mod codex;`.
   - Add `ExecutionEvent::AgentOutput { stream: String, text: String }`, documented as a non-Claude runtime's line that is neither a turn boundary nor the start gate.
   - Add a private `runtime: AgentRuntime` field to `DrivableProject`. `from_registry` and `for_testing_bypassing_opt_in` initialise it to `AgentRuntime::default()`. Add `pub fn runtime(&self)` and `pub(crate) fn with_runtime(self, runtime) -> Self` per ID-9.
   - Update the Executor trait doc sentence that says there is exactly one implementor.

   `src/journal/mod.rs` from_exec_event: AgentOutput maps to `JournalEvent::ExecEvent { stream, text }`. If a journal test enumerates every ExecutionEvent variant, add AgentOutput to it.

5. `src/error.rs`: add `SpawnError::UnsupportedByRuntime { runtime: &'static str, feature: &'static str }`, whose Display says the runtime cannot drive the feature and that nothing was launched. Add `SendError::UnsupportedByRuntime { runtime: &'static str, operation: &'static str }`. Add `DriveError::RuntimeUnrecognized(UnrecognizedRuntime)`. Extend every exhaustive match the compiler flags. In src/driver/run.rs, `spawn_failure_label` maps the new SpawnError to `spawn_failed`, since nothing launched and the closed label set stays unchanged. `spawn_failure_diagnostic` maps it to the code `spawn_unsupported_by_runtime` with a fixed driver-authored sentence, never the rendered error.

6. `src/config.rs`: add `RegisteredProject::runtime(&self) -> Result<Option<AgentRuntime>, UnrecognizedRuntime>`, which reads `self.extra["runtime"]`, and `Preferences::default_runtime(&self)`, which reads `self.extra["default_runtime"]`. Absent means Ok(None). Amend both `extra` field docs to name the modelled key, citing ID-1. Do NOT add struct fields.

7. `src/executor/codex.rs` (new, `#[cfg(unix)]`):
   - `build_codex_argv(options: &ExecutionOptions, root: &Path, prompt: &str) -> Result<Vec<OsString>, SpawnError>` is PURE and does no fs. It makes exhaustive matches on `options.target` and `options.profile` with no wildcard. ModelSeam, `resume_session: Some`, and `budget_usd: Some` return `SpawnError::UnsupportedByRuntime` (ID-5). Otherwise it emits, in this order: `exec`, `--json`, `-s`, `workspace-write`, `--add-dir`, `<root>/.git`, `-C`, `<root>`, then `-m <model>` only when set, then `--`, then the prompt as ONE element (CWE-88).
   - `CodexExecutor { program, leading_args, spawn_observer }` has `new()` (program `codex`), `with_program`, and `observing_spawn`, mirroring ClaudeExecutor's doc'd take-once Arc<Mutex<Option<..>>> shape.
   - `start_run(&self, project: &DrivableProject, command, options)` mirrors claude.rs start_run:
     - Check that the root is a directory.
     - Refuse with UnsupportedByRuntime unless `root.join(".git").is_dir()` (ID-4).
     - Translate `runtime::codex_command(&command)` HERE and nowhere else.
     - Build the argv as leading_args followed by the built argv.
     - Take the `before` snapshot through `capture_snapshot`.
     - Spawn with `CommandWrap::with_new`, applying args, current_dir(root), stdin `Stdio::null()` (research Pitfall 1: inherited stdin hangs codex forever), and piped stdout/stderr.
     - Scrub every inherited `CLAUDE*` variable in the closure, then apply `options.envelope_env` entries exactly as claude.rs does (Some sets, None removes). Do NOT set CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS.
     - Wrap in ProcessGroup::leader() and KillOnDrop.
     - Read the pgid, publish it to the spawn observer before awaiting anything, and take stdout/stderr.
     - Create a writer channel whose receiver a trivial spawned task drains and discards until closed. The Coordinator's Close and `close_input()` must succeed, with no stdin to write to.
     - Spawn `read_codex_stdout`: read_bounded_line, stamp last_line_at, skip blank lines, parse with parse_codex_line, and feed a per-reader `CodexStreamState`. Parse Ok sends ReaderItem::Codex{raw, step}. Parse Err sends `ReaderItem::Envelope(Envelope::Unparseable{raw, error})`. Truncation sends ReaderItem::Truncated. Log counts only and never content.
     - Spawn the shared `read_stderr`.
     - Spawn the Coordinator with protocol CodexExecJson, an empty first_message, options.permission_mode, the caps, `before`, project_root, and replay_observer None.
     - Await gate_rx exactly as Claude does. A closed channel maps to InitNeverObserved.
     - Build the ExecutionHandle with session_id set to the gate's thread id, empty capabilities and an empty claude_code_version.
   - `impl Executor for CodexExecutor`: `start` boxes start_run. `send` and `interrupt` return `SendError::UnsupportedByRuntime`, so the driver journals injected messages as undelivered plus missed (run.rs:1515-1571 already does this). `cancel`, `is_running` and `capabilities` are identical to Claude's.
   - Keep option-literal hygiene. No non-comment line in codex.rs, runtime.rs or codex_json.rs may contain the substrings `--resume`, `--session-id` or `-r`. That includes test literals such as a `/gsd-` command whose token after a dash starts with r; use progress, plan-phase or execute-phase instead. If one is unavoidable, adjudicate it honestly as a new NotClaude row in session_detector.rs `CLAUDE_ARGV_SITES` with the measured count.
   - NEVER call `DrivableProject::for_testing_bypassing_opt_in` anywhere under src/, test modules included (`the_escape_hatch_has_no_call_site_in_src` scans every non-comment line). Unit tests here exercise only the pure builder.

8. Driver wiring:
   - `src/driver/mod.rs` drive(): immediately after `DrivableProject::from_registry(..)?` (the gate stays first), resolve with `AgentRuntime::resolve`. Read the entry accessor first; only when it is Ok(None), read the preference accessor. Either Err becomes `DriveError::RuntimeUnrecognized` (a pure refusal above the dry-run branch that creates nothing, WR-09 symmetry). Rebind `project = project.with_runtime(runtime)`.
   - `src/driver/run.rs`: `build_executor(args, runtime) -> AgentExecutor`, keeping both `#[cfg]` bodies. Debug with `claude_program` Some uses `AgentExecutor::with_program(runtime, program, args.claude_args.clone())`, otherwise `AgentExecutor::new(runtime)`. Both call sites become `build_executor(args, project.runtime())`.
   - `shutdown_on_terminate` and `deliver_pending_inbox` take `&AgentExecutor`. The two in-module tests wrap `ClaudeExecutor::new()` in `AgentExecutor::Claude(..)`.
   - Replace `DEFAULT_AGENT_PROGRAM` with `runtime.program()`: `agent_program(args, runtime)` in both cfg bodies, called as `agent_program(args, project.runtime())` at the argv-digest site. The Claude digest input stays exactly `claude`.
   - Update the build_executor and agent_program docs.

9. Guards:
   - tests/spawn_seam_guard.rs `SPAWN_ALLOWLIST`: add `src/executor/codex.rs` with a one-line justification (the Codex agent spawn, which takes the capability type).
   - tests/async_blocking_guard.rs `ASYNC_BLOCKING_ALLOWLIST`: add `("src/executor/codex.rs", ".spawn()")` with the same justification the Claude entry carries.

10. Fixtures and tracer test:
   - `tests/fixtures/codex/01-exec-success.jsonl`: the research's success transcript verbatim, 7 lines.
   - `02-exec-failure.jsonl`: the research's failure transcript. The research abbreviated the final `turn.failed` line with `...`, so reconstruct it as valid JSON whose error.message equals the preceding top-level error's message string.
   - `README.md`: states the capture source (codex-cli 0.155.1, 2026-09-22, research probes) and marks the one reconstructed line as reconstructed. These files live OUTSIDE tests/fixtures/transcripts/, whose README inventory is enforced by driver_rate_limit.rs.
   - `tests/fixtures/fake-codex.sh`: POSIX sh, executable (chmod +x so git records 100755). Leading args are `<transcript> <exit-code> <log-dir>`, followed by the executor's argv. After shifting 3, it writes each remaining arg on its own line to `<log-dir>/argv`; writes `readlink /proc/$$/fd/0` (or `unknown`) to `<log-dir>/stdin`, which never blocks on stdin; and writes the sorted NAMES only of env vars matching `^(CODEX_|CLAUDE)` to `<log-dir>/env`. Then it cats the transcript and exits with the given code. Header comment in the fake-claude-echo.sh style.
   - `tests/driver_codex_runtime.rs` (`#![cfg(unix)]`) copies driver_tracer.rs's harness: nonblank, the envelope-root OnceLock isolation, config_for with an opt-in built from `current_prompt_inputs(root)`, run_record and journal_records. The root is a TempDir with `.planning/`, initialised as a git repo (`git init -q`). Tracer test `a_codex_registered_project_is_driven_through_codex_exec_end_to_end`: registry entry `extra` = {"runtime": "codex"}; args are `--command /gsd-progress` with a fixed run id, `claude_program` pointing to fake-codex.sh, and claude_args [01 transcript, "0", log dir]; no goal. It asserts:
     - drive returns Ok.
     - The argv log equals exactly [exec, --json, -s, workspace-write, --add-dir, <root>/.git, -C, <root>, --, $gsd-progress].
     - The stdin log is `/dev/null`.
     - run.json gsd_command is still the canonical `/gsd-progress` and its outcome is `succeeded_no_changes` or `succeeded_with_changes`.
     - The journal has an exec_started whose session id is `01a0ca2f-8a0c-72c0-87fb-11c28739d140`, an exec_event with stream `codex:completed:agent_message` and text `DONE`, and an exec_event on the turn_completed stream.

     Read JournalEvent's serde attributes for the exact field names before writing the assertions.

Doc comments: follow the repo's register. State the WHY and the measured fact (research transcripts), and never claim a property a test does not check.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --lib executor:: && rtk proxy cargo test --no-fail-fast --test driver_codex_runtime --test driver_tracer --test executor_transport --test executor_lifecycle --test spawn_seam_guard --test async_blocking_guard</automated>
  </verify>
  <done>
- The tracer test passes: a runtime=codex project runs one iteration through fake-codex with the exact argv, /dev/null stdin, a thread-id session, codex:* journal lines and a success outcome.
- The codex_json fixture unit tests pass.
- Every pre-existing test in the listed suites passes WITHOUT modification, apart from the two in-module run.rs tests whose executor argument is wrapped in `AgentExecutor::Claude` (a type-only edit).
- spawn_seam_guard and async_blocking_guard are green, including `every_claude_argv_option_site_under_src_is_adjudicated` in the lib run.
- `git diff src/executor/claude.rs` shows only visibility changes, the StreamProtocol enum and field, the `prompt_released` initialiser, the ReaderItem::Codex variant and its handle_item arm, and the Claude start_run's one added `protocol:` field line.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Expand — pin Claude byte-identity, harden the Codex child, and prove every refusal spawns nothing</name>
  <files>src/executor/codex.rs, src/executor/runtime.rs, src/driver/run.rs, tests/driver_codex_runtime.rs, tests/spawn_seam_guard.rs</files>
  <read_first>Task 1's diff. src/driver/run.rs around `journal_agent_program_override` (1715-1742) and its call site in execute_run. `GoalDecomposition::decompose` (run.rs:2250-2310), for how `SeamAnswer::Unusable` becomes a DriveError. tests/spawn_seam_guard.rs:479-487 and the runtime-assembled needle idiom (REJECT_HEAD, and FORK_OPTION_HEAD/TAIL in session_detector.rs tests).</read_first>
  <behavior>
    - Claude pin, end to end: a project with NO runtime key, whose `.planning/config.json` carries {"runtime": "codex"} (GSD's own key, which must be ignored per ID-2), is driven with fake-codex.sh as the program and logs a Claude argv. Positions 0-13 equal exactly -p, --input-format, stream-json, --output-format, stream-json, --verbose, --replay-user-messages, --session-id, <a v4 UUID>, --setting-sources, project, --permission-mode, dontAsk, --strict-mcp-config. They are followed by --disallowedTools <non-empty> and --settings <path>, and no other element. The drive result itself is not asserted, since the fixture emits no Claude init; the journal carries no runtime_envelope_partial diagnostic.
    - Preference fallback: no entry key and preferences extra {"default_runtime": "codex"} produces a Codex argv (first logged element `exec`). Entry `claude` plus preference `codex` produces Claude.
    - An unrecognized entry value (for example "gemini") returns DriveError::RuntimeUnrecognized. No run directory, no argv log and no journal is created, and a `--dry-run` of the same config is refused identically.
    - Failure path: the 02 transcript with exit code 1 ends with run.json outcome `failed`, and the journal contains a `codex:error` exec_event.
    - Model seam refusal: a goal-only drive (no --command or --target-phase) of a codex project is refused, and the fake-codex argv log never exists (zero spawns).
    - Env hardening: the test binary's OnceLock sets CODEX_THREAD_ID, CODEX_SANDBOX_NETWORK_DISABLED, CODEX_HOME and CLAUDECODE. A Codex run's env log then contains CODEX_HOME and none of the other three. A unit test of the scrub predicate covers the keep list (CODEX_HOME, CODEX_CA_CERTIFICATE), the removals (CODEX_THREAD_ID, CODEX_SESSION_ID, CODEX_CI, CLAUDE_CODE_ENTRYPOINT), and a non-matching key left alone (PATH).
    - Pure builder unit tests: a prompt beginning with `-` is the element after `--` and the last element. `-m` appears only when the model is set. `--add-dir` is always followed by `<root>/.git`. ModelSeam, resume_session and budget_usd each return UnsupportedByRuntime. Across every combination of model set/unset and envelope fields set/unset, the sandbox value is exactly workspace-write, and no element contains the runtime-assembled needles for the bypass-flag family or the full-access sandbox value.
    - send and interrupt on an AgentExecutor::Codex return SendError::UnsupportedByRuntime, tested through a minimally constructed in-crate ExecutionHandle with no DrivableProject needed.
    - codex_command: `/gsd-progress` becomes `$gsd-progress`; `/GSD:Plan-Phase 3 --X` becomes `$gsd-plan-phase 3 --X`; `$gsd-progress` stays as is; `hello world` is verbatim; `/gsd-` alone is verbatim. `AgentRuntime::resolve(None, None)` is Claude.
    - Every Codex run journals a Diagnostic with code `runtime_envelope_partial` BEFORE its exec_started. The Claude tracer run in tests/driver_tracer.rs is unchanged and carries no such record.
  </behavior>
  <action>
Write the tests above first (RED), then make them pass.

- src/executor/codex.rs: add the scrub predicate `scrubbed_from_codex_child(key: &OsStr) -> bool`, which is true for any `CLAUDE` prefix and for any `CODEX_` prefix except CODEX_HOME and CODEX_CA_CERTIFICATE (ID-8). Use it in the spawn closure in place of the CLAUDE-only loop from Task 1. Unit tests assemble the forbidden needles at runtime from two halves, so no source line carries them and comment-text discipline holds.
- src/driver/run.rs: add a `RUNTIME_ENVELOPE_PARTIAL` code constant and a `journal_runtime_disclosure(journal, runtime)` helper, called beside `journal_agent_program_override` so it lands before the first exec record. For Codex, record a Diagnostic whose fixed driver-authored detail says the run executes under codex exec in the workspace-write sandbox (network off, writes limited to the project root and its .git), that the git and credential environment envelope applies, and that the PreToolUse hook and the tool deny list have NO Codex carrier. For Claude it writes nothing, keeping Claude byte-identical. A failed write is warned about by error kind only and swallowed, matching the sibling helper.
- tests/spawn_seam_guard.rs: extend the capability-type assertion so `src/executor/codex.rs` must also carry `project: &DrivableProject` on an executable line. Add a test that no executable line under src/ contains the runtime-assembled Codex bypass-flag and full-access-sandbox needles, with a planted positive control proving the scan fires on code and not on a comment. Reuse this file's existing executable_lines/source_files helpers.
- tests/driver_codex_runtime.rs: the end-to-end tests from the behavior list, sharing Task 1's harness. Merge the env-var setup into the same OnceLock that isolates the envelope root, so set_var happens once per binary.
- A1 live probe (optional evidence, zero code impact). If `codex` is on PATH and authenticated, run one `codex exec --json --ephemeral -s read-only -C <scratch git repo> -- '-reply with OK'` with stdin from /dev/null and `timeout 180`. Record in the SUMMARY whether thread.started and turn.completed arrived, which shows that the `-`-leading prompt was taken as a prompt. If it shows the prompt parsed as a flag, STOP and report; do not ship. If codex is unavailable, record A1 as unverified; Task 3 carries it.

Final gate for the whole item, after this task:
- `rtk proxy cargo test --no-fail-fast` must show passed = baseline + new tests and failed = baseline (only the src/envelope/policy.rs git-version witness). Any other failure is investigated, not re-run until green; see memory note on the known BrokenPipe/ETXTBSY flakes, and report rather than absorb.
- `rtk proxy cargo clippy --all-targets` must show no warning in any file this plan touched, and a total no higher than the baseline.
- Do not pipe either command through grep/awk (rtk filters downstream of proxy).
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --lib executor:: && rtk proxy cargo test --no-fail-fast --test driver_codex_runtime --test driver_tracer --test spawn_seam_guard && rtk proxy cargo test --no-fail-fast && rtk proxy cargo clippy --all-targets</automated>
  </verify>
  <done>
- Every behavior above is a passing test.
- The full suite's failures equal the recorded baseline (the git-version witness only).
- clippy adds no warnings.
- The SUMMARY records the baseline versus final pass/fail/ignored counts, the A1 probe result (or "unverified"), and the Inferred decisions (for audit) section with ID-1 through ID-10.
  </done>
</task>

<task type="auto">
  <name>Task 3: File the remainder as a new pending todo flagged for audit</name>
  <files>.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md</files>
  <read_first>.planning/todos/pending/2026-09-22-support-codex-as-well-as-claude-as-the-agent-runtime.md (frontmatter shape to copy), RESEARCH.md "MVP vs remainder", and this plan's api_coverage and inferred_decisions blocks.</read_first>
  <action>
Create the todo with the same frontmatter keys as the original: `created` (current UTC ISO-8601), `title: "Codex runtime: remainder after the MVP slice (260922-hdj) [AUDIT]"`, `area: driver`, `severity: minor`, and a `files:` list naming every path cited below. Body sections:
- `## Problem`: one paragraph saying what 260922-hdj shipped and that this todo supersedes the unshipped remainder of 2026-09-22-support-codex-as-well-as-claude-as-the-agent-runtime.md (per ID-10, the coordinator decides whether to close that original).
- `## Solution`: a numbered list with file-level notes:
  1. Codex session/liveness detection in src/session_detector.rs: a `pgrep -x codex` pass filtered on argv[0] basename, skipping the codex-linux-sandbox helper and the app-server/exec-server/mcp subcommands; the session id taken from the open rollout fd under ~/.codex/sessions, wrapped in Untrusted; an AgentSession runtime field with ClaudeSession kept as an alias. Consumers: registry.rs auto_register_from_sessions, app.rs, main.rs, and the Sessions UI. Include the ID-3 reason it was deferred, and the CLAUDE_ARGV_SITES census impact.
  2. TUI launch/resume in src/ui/screens/detail.rs (claude_launch_args/claude_resume_args to codex / `codex resume <id>`), with its own CWE-88 fusion analysis.
  3. Steering/inbox via `codex exec resume <thread_id>` between turns or the app-server protocol, replacing SendError::UnsupportedByRuntime.
  4. Model seam via `--output-schema <tempfile>` (lifts the ModelSeam refusal in src/executor/codex.rs).
  5. AGENTS.md opt-in disclosure: registry.rs prompt-input disclosure and the from_registry drift check currently cover CLAUDE.md only.
  6. Envelope parity: codex hooks or a sandbox policy to carry the PreToolUse guard and tool deny list, retiring the runtime_envelope_partial diagnostic.
  7. Worktree-aware `--add-dir` via `git rev-parse --git-common-dir`, lifting the ID-4 refusal.
  8. A runtime picker in the TUI config screen, plus a mismatch hint against GSD's own `runtime` key.
  9. src/state_reader/queue_md.rs resolving ~/.codex/gsd-core/bin/gsd-tools.cjs.
  10. A codex version gate (minimum tested 0.155.1).
  11. Journaling turn.completed usage token counts.
  12. resume_session and budget_usd support (and verifying the A2 `-c sandbox_mode` key).
  13. Agent-neutral renames: ClaudeSession, DriveArgs claude_program/claude_args, claude_code_version, claude_pgid (the assumption-delta accepted debt).
  14. The A1 status copied from Task 2's SUMMARY (verified or unverified).
- `## Audit`: copy ID-1 through ID-10 verbatim, plus research assumptions A1-A5 with their status after execution, each marked `[AUDIT]`.
  </action>
  <verify>
    <automated>test -f .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md && grep -q '^area: driver$' .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md && grep -q '^## Audit' .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md && grep -q 'session_detector.rs' .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md && grep -q 'ID-10' .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md</automated>
  </verify>
  <done>
- The new pending todo exists with the original's frontmatter keys and `area: driver`.
- It lists all 14 remainder items with concrete file paths.
- Its Audit section reproduces ID-1 through ID-10 and A1-A5 with post-execution status.
- The original todo file is left untouched.
  </done>
</task>

</tasks>

<threat_model>
ASVS level 1; block on high.

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| driver → codex child (argv, env, cwd) | The driver hands a user-supplied or router-derived command, and its own environment, to an autonomous agent process |
| codex stdout JSONL → reader task | Every line is untrusted wire data from a process that reads third-party repository content |
| manager config.json → runtime resolution | User-authored values select which agent binary is executed |
| driven repository (AGENTS.md) → codex model context | Codex loads the repository instructions itself, outside any boundary the driver builds |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-hdj-01 | Tampering / EoP | build_codex_argv prompt position (CWE-88) | high | mitigate | Prompt is one argv element after `--`, with no shell (CommandWrap). A unit test covers a `-`-leading prompt, plus the optional A1 live probe (Task 2) |
| T-hdj-02 | Elevation of Privilege | build_codex_argv sandbox/flags | high | mitigate | The builder emits only `-s workspace-write` and never the bypass family or full-access value. The unit combinatorial scan plus the spawn_seam_guard src scan use runtime-assembled needles with a positive control (Task 2) |
| T-hdj-03 | Information Disclosure | codex spawn closure env | medium | mitigate | Scrub CLAUDE* and CODEX_* except CODEX_HOME/CODEX_CA_CERTIFICATE, then apply EnvelopeEnv (SSH_AUTH_SOCK/GIT_CONFIG_GLOBAL) in the one closure. Covered by the predicate unit test and the e2e env-name log (Task 2) |
| T-hdj-04 | Tampering / DoS | read_codex_stdout / CodexStreamState | medium | mitigate | 4 MiB read_bounded_line bound, parsing in the reader task, unknown types to Unknown, torn lines to Unparseable. item.type labels are restricted to `[a-z0-9_]{1,32}`, else `other`. Journal text passes the existing redactor. Only counts are logged, never content |
| T-hdj-05 | Elevation of Privilege | Codex run without the PreToolUse hook and tool deny list | medium | mitigate (partial) + accept residual [AUDIT] | The network-off workspace-write sandbox limits writes to the root and .git. EnvelopeEnv applies. Per-project opt-in plus an explicit runtime key are required (Claude default), and the driver is hidden behind GSDMM_EXPERIMENTAL_FEATURES. Every Codex run journals `runtime_envelope_partial`. Parity is remainder item 6 |
| T-hdj-06 | Spoofing (consent) | AGENTS.md read by codex, not covered by the opt-in digest | medium | accept [AUDIT] | Codex is reachable only by editing config to runtime=codex on an opted-in project. Disclosure is remainder item 5 |
| T-hdj-07 | Tampering | runtime key in config.json | low | mitigate | Exact lowercase values only. Anything else is a typed pre-dry-run refusal echoing an Untrusted value. GSD's runtime keys are never auto-adopted (ID-2) |
| T-hdj-08 | Repudiation | journal of a Codex run | low | mitigate | The argv digest is computed over the `codex` program name. exec_started carries the codex thread id, and the runtime_envelope_partial diagnostic names the runtime |
| T-hdj-09 | Denial of Service | inherited stdin | medium | mitigate | `Stdio::null()`. The fixture logs `/proc/$$/fd/0` and the tracer asserts `/dev/null` |
| T-hdj-10 | Elevation of Privilege | model seam under Codex | low | mitigate | Typed refusal inside the pure builder before spawn. The e2e test proves zero spawns and no silent fallback to Claude auth |
| T-hdj-SC | Tampering | package installs | low | accept | No new crates or packages. The package legitimacy gate is not applicable |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: failures equal the pre-change baseline (only the src/envelope/policy.rs git-version witness, which fails locally by design and passes on the runner), and passed = baseline + new tests.
- `rtk proxy cargo clippy --all-targets`: no warning in a touched file, and the total does not exceed the baseline.
- `git diff --stat` touches only `files_modified`. src/executor/claude.rs changes are limited to the enumerated visibility and additive pieces. No existing test body was edited except the two type-only `AgentExecutor::Claude(..)` wraps in run.rs.
- tests/driver_codex_runtime.rs proves the Codex path end to end and pins the Claude argv end to end.
</verification>

<success_criteria>
- A registry entry with runtime=codex drives `codex exec --json` with the exact argv, /dev/null stdin, and a scrubbed env. Its thread id, turns, items and outcome land in the existing journal and run.json.
- A project without the key behaves exactly as before, proven by the unchanged suite and the new Claude argv pin.
- The model seam, resume, budget cap, send and interrupt are typed refusals under Codex, and none spawns a process.
- The remainder todo exists, is flagged [AUDIT], and carries ID-1 through ID-10.
</success_criteria>

<output>
Create `.planning/quick/260922-hdj-support-codex-as-well-as-claude-as-the-agent-runtime-todo-pl/260922-hdj-SUMMARY.md`. It must include: baseline versus final test counts; the clippy baseline versus final; the A1 probe result; the claude.rs diff summary; and an "Inferred decisions (for audit)" section reproducing ID-1 through ID-10 and noting that the original todo was left for the coordinator (ID-10).
</output>
