---
created: 2026-09-22T18:30:02.000Z
title: "Codex runtime: remainder after the MVP slice (260922-hdj) [AUDIT]"
area: driver
severity: minor
files:
  - src/session_detector.rs
  - src/registry.rs
  - src/app.rs
  - src/main.rs
  - src/ui/screens/detail.rs
  - src/executor/codex.rs
  - src/executor/codex_json.rs
  - src/executor/runtime.rs
  - src/executor/claude.rs
  - src/executor/mod.rs
  - src/driver/run.rs
  - src/driver/mod.rs
  - src/config.rs
  - src/cli.rs
  - src/journal/mod.rs
  - src/state_reader/queue_md.rs
  - src/ui/screens/mod.rs
---

## Problem

Quick task 260922-hdj shipped the minimum viable Codex runtime slice: an
`AgentRuntime {Claude, Codex}` abstraction (`src/executor/runtime.rs`), a
`CodexExecutor` driving `codex exec --json` on the shared Claude coordinator
(`src/executor/codex.rs`), a Codex JSONL parser mapped onto the existing
turn/outcome types (`src/executor/codex_json.rs`), driver dispatch through the
`AgentExecutor` enum, and per-project (`runtime`) / global (`default_runtime`)
selection in the manager's `config.json`. Claude behaviour is unchanged and
pinned by an end-to-end argv test. Everything below did not fit and is still
open. This todo supersedes the unshipped remainder of
`2026-09-22-support-codex-as-well-as-claude-as-the-agent-runtime.md`; per
ID-10 that original was left in `pending/`, and the coordinator decides
whether to close it.

## Solution

1. **Codex session/liveness detection** in `src/session_detector.rs`: a
   `pgrep -x codex` pass filtered on the argv[0] basename, skipping the
   `codex-linux-sandbox` helper (same `comm`) and the `app-server` /
   `exec-server` / `mcp` subcommands; the session id taken from the open
   rollout fd under `~/.codex/sessions/**/rollout-*-<thread_id>.jsonl`,
   wrapped in `Untrusted`; an `AgentSession` with a `runtime` field, keeping
   `ClaudeSession` as an alias. Consumers: `src/registry.rs`
   `auto_register_from_sessions`, `src/app.rs`, `src/main.rs`, and the
   Sessions UI in `src/ui/screens/detail.rs`. Deferred (ID-3) because the
   Sessions tab would then offer `claude --resume=<codex thread id>` (detail.rs
   launch/resume is Claude-only and belonged to sibling item 260922-hdi) and
   because it changes auto-registration. Driven-run liveness and the kill
   switch are already runtime-agnostic. **Census impact:** a codex argv parser
   adds option literals to `session_detector.rs`; re-measure its row in
   `CLAUDE_ARGV_SITES` and adjudicate any codex-only literal as `NotClaude`.
2. **TUI launch/resume** in `src/ui/screens/detail.rs`:
   `claude_launch_args` / `claude_resume_args` → `codex` / `codex resume <id>`.
   Needs its own CWE-88 fusion analysis: `codex resume` takes a positional id,
   not a fused `--resume=<id>`, so a `--` terminator (or an id-alphabet check)
   is required.
3. **Steering/inbox** for Codex, via `codex exec resume <thread_id> -- <msg>`
   between turns or the `app-server` protocol, replacing
   `SendError::UnsupportedByRuntime` in `src/executor/codex.rs` (today the
   driver journals injected messages as undelivered + missed).
4. **Model seam** via `--output-schema <tempfile>` + the terminal
   `agent_message`, lifting the `SpawnProfile::ModelSeam` refusal in
   `build_codex_argv` (`src/executor/codex.rs`). Until then `--goal` and
   escalation on a Codex project refuse with `GoalSeamUnusable`.
5. **AGENTS.md opt-in disclosure**: `src/registry.rs` prompt-input disclosure
   (`current_prompt_inputs`, the `("CLAUDE.md", PromptProfile::Executor)` row)
   and the `DrivableProject::from_registry` drift check cover `CLAUDE.md` only,
   while Codex loads `AGENTS.md` itself (T-hdj-06).
6. **Envelope parity**: Codex hooks (`features.hooks`, `~/.codex/hooks.json`)
   or a `-c` sandbox policy to carry the `PreToolUse` guard and the tool deny
   list, then retire the `runtime_envelope_partial` diagnostic
   (`src/driver/run.rs` `journal_runtime_disclosure`) (T-hdj-05).
7. **Worktree-aware `--add-dir`** via `git rev-parse --git-common-dir`,
   lifting the ID-4 refusal in `CodexExecutor::start_run` for worktrees and
   submodules (`.git` is a file).
8. **Runtime picker** in the TUI config screen (`src/ui/screens/mod.rs` /
   `detail.rs` Config tab) writing the entry `runtime` / preference
   `default_runtime` keys, plus a mismatch hint when GSD's own `runtime`
   (`.planning/config.json`, `~/.gsd/defaults.json`) disagrees with the
   manager's.
9. `src/state_reader/queue_md.rs` `resolve_gsd_tools` also resolving
   `~/.codex/gsd-core/bin/gsd-tools.cjs`.
10. A **codex version gate** (minimum tested: codex-cli 0.155.1), the analogue
    of the Claude capability/version gate in `src/executor/gate.rs`.
11. Journal `turn.completed.usage` token counts (`src/executor/codex_json.rs`
    drops them today; `src/journal/mod.rs` has only a USD `cost` record).
12. `resume_session` and `budget_usd` support under Codex (both are typed
    pre-spawn refusals today), including verifying the A2 `-c sandbox_mode`
    key, since `codex exec resume` rejects `-s` and `-C`.
13. **Agent-neutral renames** (the assumption-delta accepted debt): the
    `ClaudeSession` type, `DriveArgs::claude_program` / `claude_args` (and the
    `--claude-program` debug flag in `src/cli.rs`), `ExecutionHandle` /
    `SessionStarted::claude_code_version`, and the journal's `claude_pgid`.
    They were ADD-ALONGSIDE in the MVP because renaming serialized/journaled
    field names would break Claude byte-identity. Codex session detection
    (item 1) is what forces the promotion.
14. **A1 status: VERIFIED** on 2026-09-22 against codex-cli 0.155.1:
    `codex exec --json --ephemeral -s read-only -C <scratch git repo> -- '-reply with OK'`
    with stdin from `/dev/null` exited 0 with `thread.started`, `turn.started`,
    an `agent_message` of `OK`, and `turn.completed`, so a `-`-leading prompt
    after `--` is taken as the prompt, not a flag. Re-probe on a codex version
    change.

## Audit

Decisions inferred with the human unavailable, copied from the 260922-hdj plan.
Each needs a human look.

- [AUDIT] ID-1: Runtime is stored under the existing `#[serde(flatten)] extra`
  maps (entry key `runtime`, preferences key `default_runtime`) and read
  through typed accessors. It is not a new typed struct field. Reasons: a typed
  field breaks about 30 `RegisteredProject { .. }` literals across 28 files
  (several owned by sibling items), and an unrecognized string in a typed field
  would make config.json fail to load, so the TUI would not start. With the
  accessor, an unrecognized value is a typed refusal at drive time and
  round-trips untouched.
- [AUDIT] ID-2: Resolution is entry > preference > Claude. GSD's `runtime` key
  is never read. On this machine `~/.gsd/defaults.json` says codex, so
  auto-adoption would silently move Claude projects off Claude (research A5).
- [AUDIT] ID-3: Codex session/liveness detection in src/session_detector.rs is
  DEFERRED to the remainder. Adding codex processes to `detect_sessions()`
  would make the Sessions tab offer `claude --resume=<codex thread id>`,
  because detail.rs launch/resume is Claude-only and detail.rs belongs to
  260922-hdi. It would also change auto-registration. Driven-run liveness and
  the kill switch are runtime-agnostic (pgid published by observing_spawn,
  run-id liveness), so stopping a driven Codex run already works.
- [AUDIT] ID-4: `<root>/.git` must be a directory. A worktree or submodule
  (`.git` is a file) and a missing `.git` are refused before spawn, so commits
  never fail silently.
- [AUDIT] ID-5: Under Codex, resume_session, budget_usd and
  SpawnProfile::ModelSeam are refused (typed, zero spawns). name,
  setting_sources, permission_mode and bg_wait_ceiling_ms are ignored because
  they are Claude-only. The envelope deny list and the settings path have no
  Codex carrier, and each run discloses this in a journal diagnostic.
- [AUDIT] ID-6: The model-seam refusal reaches the user through the existing
  `SeamAnswer::Unusable` → goal-seam refusal path, with no new pre-dry-run
  refusal.
- [AUDIT] ID-7: Only prefixed commands are translated (case-insensitive
  `/gsd-`, `/gsd:`, `gsd:`, `$gsd-`). Unprefixed text passes verbatim. This
  deviates from GSD's formatGsdSlash, which treats a bare word as a command
  name, because the executor's input is a prompt.
- [AUDIT] ID-8: The Codex child scrubs `CLAUDE*` and `CODEX_*` except
  `CODEX_HOME` and `CODEX_CA_CERTIFICATE` (research A4).
- [AUDIT] ID-9: The resolved runtime rides on the DrivableProject token
  (`with_runtime`). It is not threaded through the
  dispatch/execute_run/decompose/consult_model_seam signatures, which keeps the
  spawn_seam_guard ordering scanners untouched.
- [AUDIT] ID-10: This item does not move the original todo to completed/. The
  remainder todo names it as superseded, and the coordinator decides whether to
  close it.

Research assumptions, status after execution:

- [AUDIT] A1 (`--` before the positional prompt is honoured by `codex exec`):
  **VERIFIED** by the live probe in Solution item 14, and pinned in unit tests
  (`a_dash_leading_prompt_is_the_one_element_after_the_terminator`).
- [AUDIT] A2 (`-c sandbox_mode="workspace-write"` is the right key for
  `exec resume`): **UNVERIFIED**, not exercised: resume is refused under Codex
  in the MVP (Solution item 12).
- [AUDIT] A3 (the interactive codex TUI holds its rollout fd itself, not via
  the app-server daemon): **UNVERIFIED**; only matters for session detection
  (Solution item 1).
- [AUDIT] A4 (the `CODEX_*` scrub list): **IMPLEMENTED** as ID-8 (keeps
  `CODEX_HOME` and `CODEX_CA_CERTIFICATE`), pinned by a predicate unit test and
  an end-to-end env-name log. Whether codex needs any other `CODEX_*` variable
  from its parent is unverified.
- [AUDIT] A5 (resolution registry > preference > Claude, ignoring GSD's
  `runtime` key): **IMPLEMENTED** as ID-2 and pinned end to end
  (`a_project_with_no_runtime_key_spawns_the_unchanged_claude_argv` plants
  GSD's `runtime: codex` and asserts the Claude argv).
