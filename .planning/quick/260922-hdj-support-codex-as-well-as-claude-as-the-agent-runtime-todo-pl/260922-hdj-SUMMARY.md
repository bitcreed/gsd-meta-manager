---
phase: quick-260922-hdj
plan: 01
subsystem: executor/driver
status: complete
tags: [codex, runtime, executor, driver, journal, security]
requires: []
provides:
  - AgentRuntime {Claude, Codex} + manager-config resolution (entry runtime > preference default_runtime > Claude)
  - CodexExecutor (codex exec --json) on the shared claude.rs Coordinator
  - Codex JSONL parser mapped onto ResultMessage / ExecutionEvent
  - AgentExecutor enum as the driver's executor type
affects: [src/executor, src/driver, src/journal, src/config.rs, src/error.rs, src/registry.rs (test module)]
tech-stack:
  added: []
  patterns: [runtime rides the DrivableProject capability token, synthesized ResultMessage turns reuse derive_run_outcome_from_envelopes, runtime-assembled forbidden needles]
key-files:
  created:
    - src/executor/runtime.rs
    - src/executor/codex_json.rs
    - src/executor/codex.rs
    - tests/driver_codex_runtime.rs
    - tests/fixtures/fake-codex.sh
    - tests/fixtures/codex/README.md
    - tests/fixtures/codex/01-exec-success.jsonl
    - tests/fixtures/codex/02-exec-failure.jsonl
    - .planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md
  modified:
    - src/executor/claude.rs
    - src/executor/mod.rs
    - src/executor/stream_json.rs
    - src/error.rs
    - src/config.rs
    - src/journal/mod.rs
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/registry.rs
    - tests/spawn_seam_guard.rs
    - tests/async_blocking_guard.rs
decisions:
  - "ID-1..ID-10 as planned (see Inferred decisions); all [AUDIT]"
  - "UnrecognizedRuntime carries the config key, so from_config_value takes (key, value)"
  - "turn.failed error kept as serde_json::Value so an unexpected shape still classifies the turn as failed"
metrics:
  duration: ~21m
  completed: 2026-09-22
actuals:
  tokens: 33334
  tasks: 3
  commits: 4
plan_head_before: b90ef326870e9a554bb33523e7e266bec31e507e
---

# Quick 260922-hdj Plan 01: Codex runtime MVP slice Summary

A project whose manager registry entry says `runtime: codex` is now driven by
`codex exec --json -s workspace-write --add-dir <root>/.git -C <root> [-m <model>] -- $gsd-<cmd>`
with stdin at `/dev/null` and a scrubbed environment. The executor reuses the
Claude supervisor through `pub(super)` visibility, and codex turns are
synthesized into `ResultMessage`s so the existing outcome derivation classifies
them. Projects with no runtime key still spawn the unchanged Claude argv.

## Commits

| Task | Commit | Subject |
|---|---|---|
| 1 (tracer) | 5eb7d2a | feat(quick-260922-hdj): drive a runtime=codex project through codex exec end to end |
| 2 (RED) | 0e5a1a8 | test(quick-260922-hdj): pin Claude byte-identity, Codex hardening and pre-spawn refusals, RED |
| 2 (GREEN) | a8ef9d6 | feat(quick-260922-hdj): journal runtime_envelope_partial before every codex run's first exec record |
| 3 | 862c038 | docs(quick-260922-hdj): file the Codex runtime remainder as a pending todo [AUDIT] |

The tracer gate after Task 1 passed: `<verify>` was re-run end to end and was
green before the expansion started.

## Tests added (30)

- `src/executor/runtime.rs` (4): exact-lowercase config values only; resolve precedence (`resolve(None, None)` is Claude); `codex_command` table (`/gsd-progress`→`$gsd-progress`, `/GSD:Plan-Phase 3 --X`→`$gsd-plan-phase 3 --X`, `$gsd-progress` unchanged, `hello world` and `/gsd-` verbatim); the argument tail is kept byte for byte.
- `src/executor/codex_json.rs` (7): the success fixture gives ThreadStarted, then outputs, then TurnEnded(success) with the right session id; the failure fixture gives TurnEnded(error_during_execution) with the message; the pre-turn error item is a warning; an unknown type is Unknown and a torn line is Err; a hostile item.type labels as `other`; compact-JSON fallback; the gate outcome carries no Claude-only fields.
- `src/executor/codex.rs` (8): exact argv shape; a `-`-leading prompt is one element after `--`; `-m` only when a model is set; `--add-dir` is `<root>/.git`; the model seam, resume and budget are refused; no option combination widens the sandbox or emits a bypass needle; the scrub predicate; send and interrupt return `SendError::UnsupportedByRuntime` through `AgentExecutor::Codex`.
- `tests/driver_codex_runtime.rs` (8, end to end): the tracer; `runtime_envelope_partial` is journaled before `exec_started`; the Claude argv pin (no runtime key, with GSD's `.planning/config.json` `runtime: codex` planted, exact positions 0-13, then `--disallowedTools`/`--settings`, length 18, no disclosure); preference fallback and entry precedence; an unrecognized runtime is refused (real and dry run) with nothing created or spawned; the failure transcript ends `failed` with a `codex:error` event; a goal-only codex drive is refused (`GoalSeamUnusable`) with zero spawns; the env log keeps `CODEX_HOME` and drops `CODEX_THREAD_ID`, `CODEX_SANDBOX_NETWORK_DISABLED`, `CLAUDECODE` and every `CLAUDE*`.
- `tests/spawn_seam_guard.rs` (3): `codex.rs` takes `project: &DrivableProject`; no executable src line carries the codex bypass or full-access needles; a planted positive control shows the scan fires on code and not on a comment.

## Test and clippy counts

| | Baseline (b90ef32) | Final (862c038) |
|---|---|---|
| `cargo test --no-fail-fast` passed | 2140 | 2170 (+30, all new tests) |
| failed | 1 (git-version witness in `src/envelope/policy.rs`) | 1 (the same witness) |
| ignored | 15 | 15 |
| `cargo clippy --all-targets` warnings | 11 (src/browser.rs ×3, src/project_creator.rs ×1, tests/envelope_*.rs ×7) | 11, the same sites; none in a touched file |

The first final full run showed a second failure:
`tests/envelope_interior_path.rs::the_t_19_119_replacement_takes_layer_2_as_well_and_that_is_measured_separately`
with `Text file busy` (ETXTBSY: a copied binary exec'd while a parallel fork
still held its write fd). That file is untouched by this plan. It passed 3/3
isolated reruns and the second full run (the counts above). It is the known
flake class and was not absorbed.

## A1 live probe: VERIFIED

`timeout 180 codex exec --json --ephemeral -s read-only -C <scratch git repo> -- '-reply with OK' < /dev/null`
ran against codex-cli 0.155.1 and exited 0. It emitted `thread.started`,
`turn.started`, an `agent_message` of `OK`, and `turn.completed`. The
`-`-leading prompt after `--` was taken as a prompt. stderr still printed
"Reading additional input from stdin...", but with `/dev/null` that read hits
EOF immediately.

## claude.rs diff

Beyond making items `pub(super)`, the diff has four additive pieces:

- **Visibility (`pub(super)`):** `EVENT_CHANNEL_CAPACITY`, `WRITER_CHANNEL_CAPACITY`, `capture_snapshot`, `ReaderItem`, `BoundedLine`, `read_bounded_line`, `read_stderr`, `Coordinator`, every one of its fields, and `Coordinator::run`.
- **`StreamProtocol` enum and `protocol` field:** adds `StreamProtocol { ClaudeStreamJson, CodexExecJson }`, a `protocol` field on `Coordinator` (destructured in `run`), and one `protocol: StreamProtocol::ClaudeStreamJson,` line in Claude's `start_run`.
- **`prompt_released` initialiser:** `matches!(protocol, StreamProtocol::CodexExecJson)`, which is false for Claude, exactly as before.
- **Codex reader item:** a `ReaderItem::Codex { raw, step }` variant, its `handle_item` arm, and one `use crate::executor::codex_json::{self, CodexStep};` import.

`build_argv`, the spawn closure, `read_stdout` and every existing arm are untouched.

## Deviations from Plan

1. **[Rule 3 - Blocking] `src/registry.rs` (not in `files_modified`).** Its test module has a wildcard-free `drive_untrusted_fields` match over `DriveError`, so the new `RuntimeUnrecognized` variant did not compile without an arm. The module's rule is that a variant echoing an untrusted value returns 1 and joins the subject list of `a_refusal_never_carries_a_raw_control_or_invisible_character`. The arm was added (returning 1), the variant was added to the subject list, and the hard-coded subject count went from 11 to 12 ("ELEVEN" to "TWELVE"). This is the one edit to an existing test's assertion body. It is forced by the enum extension and widens coverage. **[AUDIT]**
2. **Scrub predicate landed in Task 1 rather than Task 2.** `scrubbed_from_codex_child` was written with the codex spawn closure, so the closure never had the interim CLAUDE-only loop. Task 2 added its unit test and the end-to-end env test. Both passed on first run, so only the disclosure test was genuinely RED.
3. **Journal enumeration test left unchanged.** `an_executor_stream_becomes_a_journal_a_reader_can_replay` asserts an exact kind sequence over `every_execution_event()`. Adding `AgentOutput` there would change an existing assertion, so the mapping is covered end to end instead (the `codex:*` exec_event assertions in `tests/driver_codex_runtime.rs`).
4. **Capability-type guard added as a sibling test.** It is `the_codex_spawn_seam_takes_the_capability_type`, not an edit to `every_process_spawn_site_in_src_is_on_the_allowlist`, so that existing test stays byte-identical.
5. **`AgentRuntime::from_config_value(key, value)`** takes the config key, so `UnrecognizedRuntime` can name it. The plan listed only the value parameter.
6. **`turn.failed.error` is modelled as `Option<serde_json::Value>`** rather than `{message: Option<String>}`, so an unexpected error shape still ends the turn as failed instead of becoming an unparseable line.
7. **Bypass needles.** The unit test and the src guard use runtime-assembled halves of the bypass flag family, the full-access sandbox value, and codex's `--yolo` alias.
8. **The combinatorial builder test varies `model`, `envelope_disallowed_tools` and `envelope_settings`.** It does not vary `envelope_env`, which has no public constructor outside `envelope::cred` and is never read by the pure builder.

## Inferred decisions (for audit)

- ID-1: Runtime is stored under the existing `#[serde(flatten)] extra` maps (entry key `runtime`, preferences key `default_runtime`) and read through typed accessors. It is not a new typed struct field. Reasons: a typed field breaks about 30 `RegisteredProject { .. }` literals across 28 files (several owned by sibling items), and an unrecognized string in a typed field would make config.json fail to load, so the TUI would not start. With the accessor, an unrecognized value is a typed refusal at drive time and round-trips untouched.
- ID-2: Resolution is entry > preference > Claude. GSD's `runtime` key is never read. On this machine `~/.gsd/defaults.json` says codex, so auto-adoption would silently move Claude projects off Claude (research A5).
- ID-3: Codex session/liveness detection in src/session_detector.rs is DEFERRED to the remainder. Adding codex processes to `detect_sessions()` would make the Sessions tab offer `claude --resume=<codex thread id>`, because detail.rs launch/resume is Claude-only and detail.rs belongs to 260922-hdi. It would also change auto-registration. Driven-run liveness and the kill switch are runtime-agnostic (pgid published by observing_spawn, run-id liveness), so stopping a driven Codex run already works.
- ID-4: `<root>/.git` must be a directory. A worktree or submodule (`.git` is a file) and a missing `.git` are refused before spawn, so commits never fail silently.
- ID-5: Under Codex, resume_session, budget_usd and SpawnProfile::ModelSeam are refused (typed, zero spawns). name, setting_sources, permission_mode and bg_wait_ceiling_ms are ignored because they are Claude-only. The envelope deny list and the settings path have no Codex carrier, and each run discloses this in a journal diagnostic.
- ID-6: The model-seam refusal reaches the user through the existing `SeamAnswer::Unusable` → goal-seam refusal path, with no new pre-dry-run refusal.
- ID-7: Only prefixed commands are translated (case-insensitive `/gsd-`, `/gsd:`, `gsd:`, `$gsd-`). Unprefixed text passes verbatim. This deviates from GSD's formatGsdSlash, which treats a bare word as a command name, because the executor's input is a prompt.
- ID-8: The Codex child scrubs `CLAUDE*` and `CODEX_*` except `CODEX_HOME` and `CODEX_CA_CERTIFICATE` (research A4).
- ID-9: The resolved runtime rides on the DrivableProject token (`with_runtime`). It is not threaded through the dispatch/execute_run/decompose/consult_model_seam signatures, which keeps the spawn_seam_guard ordering scanners untouched.
- ID-10: This item does not move the original todo to completed/. The remainder todo names it as superseded, and the coordinator decides whether to close it. **The original `.planning/todos/pending/2026-09-22-support-codex-as-well-as-claude-as-the-agent-runtime.md` was left untouched for the coordinator.**

## Deferred remainder

`.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md`
(area: driver, [AUDIT]) lists 14 items:

1. Session/liveness detection
2. TUI launch/resume
3. Steering
4. Model seam
5. AGENTS.md disclosure
6. Envelope parity
7. Worktree git dir
8. Runtime picker
9. gsd-tools resolution
10. Version gate
11. Usage journaling
12. Resume and budget support
13. Agent-neutral renames
14. A1 status (verified)

Its Audit section carries ID-1 through ID-10 and A1 through A5 with their post-execution status.

## Known Stubs

None. The Claude-only fields on a Codex `ExecutionHandle` / `SessionStarted`
(`claude_code_version` empty, `capabilities` empty) are honest absences
documented in `codex_json::gate_outcome_for_thread`, not stubs.

## Threat Flags

None beyond the plan's threat register. T-hdj-05 (no PreToolUse hook or deny
list under Codex) and T-hdj-06 (AGENTS.md is not covered by the opt-in digest)
remain accepted residuals, flagged [AUDIT] and tracked as remainder items 6
and 5.

## Self-Check: PASSED

- All created files exist: runtime.rs, codex_json.rs, codex.rs, driver_codex_runtime.rs, fake-codex.sh (100755), the codex fixtures and README, and the remainder todo.
- Commits 5eb7d2a, 0e5a1a8, a8ef9d6 and 862c038 are present on master (`git rev-list --count b90ef32..HEAD` = 4).
