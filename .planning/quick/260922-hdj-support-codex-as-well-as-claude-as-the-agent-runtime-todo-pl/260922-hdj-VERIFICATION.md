---
item: quick-260922-hdj
verified: 2026-09-22T00:00:00Z
status: passed
score: 7/7 must-haves verified
---

# Quick 260922-hdj: Codex Runtime MVP Slice — Verification Report

**Item goal:** Support Codex as well as Claude as the agent runtime (MVP slice
plus remainder todo), verified by code inspection (cargo test/clippy run
concurrently by the orchestrator; not re-run here to avoid target-dir
contention).

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | codex=runtime project spawns exact `codex exec` argv with `/dev/null` stdin | ✓ VERIFIED | `src/executor/codex.rs:71-119,205-211` `build_codex_argv`/spawn closure; pinned by `the_argv_is_exactly_the_documented_shape` and e2e test `a_codex_registered_project_is_driven_through_codex_exec_end_to_end` in `tests/driver_codex_runtime.rs:182-244` |
| 2 | Codex JSONL reaches existing outcome pipeline unchanged | ✓ VERIFIED | `src/executor/codex_json.rs` maps `turn.completed`/`turn.failed` to `ResultMessage` (`Default` added to `stream_json.rs`, all fields `#[serde(default)]`); `claude.rs:1744-1798` `ReaderItem::Codex` arm pushes onto same `envelopes` Vec `derive_run_outcome_from_envelopes` reads; item events become `AgentOutput` → journaled `codex:*` exec_events |
| 3 | Runtime resolution: entry > preference > Claude; GSD's own keys never read | ✓ VERIFIED | `src/executor/runtime.rs:112-114` `resolve()`; `src/driver/mod.rs:843-862` reads `entry.runtime()` then `config.preferences.default_runtime()`, never `.planning/config.json` or `~/.gsd/defaults.json`; proven by `a_project_with_no_runtime_key_spawns_the_unchanged_claude_argv` which plants GSD's own `runtime: codex` and asserts the Claude argv still spawns |
| 4 | Claude stays byte-identical | ✓ VERIFIED | `git diff src/executor/claude.rs` (b90ef32..862c038) is visibility-only plus `StreamProtocol` enum/field and the additive `ReaderItem::Codex` arm — `build_argv`, spawn closure, `read_stdout` and all existing arms untouched; `driver/run.rs` `agent_program`/`build_executor` for Claude produce byte-identical argv (digest input stays `claude`); pinned end-to-end by the Claude-argv-pin test (positions 0-13 exact, length 18) |
| 5 | Model seam, resume, budget, send, interrupt are typed pre-spawn refusals under Codex, zero processes | ✓ VERIFIED | `codex.rs:78-101` exhaustive match refuses ModelSeam/resume_session/budget_usd via `SpawnError::UnsupportedByRuntime` before any argv is built; `send`/`interrupt` return `SendError::UnsupportedByRuntime` (`codex.rs:418-443`); e2e tests `a_goal_only_drive_of_a_codex_project_is_refused_without_spawning` and `an_unrecognized_runtime_is_refused_before_anything_is_created` assert zero argv/journal artifacts |
| 6 | Codex child never gets bypass/full-access flags; scrubs CLAUDE*/CODEX_* except CODEX_HOME/CA; journals `runtime_envelope_partial` before first exec record | ✓ VERIFIED | `codex.rs:346-352` `scrubbed_from_codex_child`; `driver/run.rs` `journal_runtime_disclosure` called before `exec_started`; guarded by `tests/spawn_seam_guard.rs` new tests with a **planted positive control** proving the bypass-flag/full-access-sandbox scan actually fires (`no_executable_line_in_src_carries_a_codex_bypass_flag_or_the_full_access_sandbox`, `the_codex_bypass_scan_fires_on_code_and_not_on_a_comment`); no literal dangerous flags found anywhere under `src/` (grep) |
| 7 | Remainder filed as new pending todo flagged [AUDIT] | ✓ VERIFIED | `.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md` exists, `area: driver`, title ends `[AUDIT]`, 14 concrete numbered items with file paths, `## Audit` section reproduces ID-1..ID-10 and A1-A5 with post-execution status; original todo left untouched per ID-10 |

**Score:** 7/7 truths verified.

## Required Artifacts

| Artifact | Expected | Status |
|----------|----------|--------|
| `src/executor/runtime.rs` | AgentRuntime, resolve, codex_command, AgentExecutor enum | ✓ VERIFIED — all present, matches spec exactly |
| `src/executor/codex_json.rs` | CodexEvent, parse_codex_line, CodexStreamState::step | ✓ VERIFIED — matches spec, 8 unit tests over both fixtures |
| `src/executor/codex.rs` | build_codex_argv (pure), CodexExecutor | ✓ VERIFIED — 691 lines, pure builder + Executor impl, 8 unit tests |
| `tests/driver_codex_runtime.rs` | e2e drive() tests incl. Claude argv pin | ✓ VERIFIED — 468 lines, 8 end-to-end tests |
| `tests/fixtures/fake-codex.sh` | argv/stdin/env-logging stand-in | ✓ VERIFIED — executable (100755), logs argv/stdin/env names only |
| `.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md` | remainder todo, area: driver, [AUDIT] | ✓ VERIFIED |

## Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `src/driver/mod.rs` | `src/executor/runtime.rs` | `drive()` resolves runtime after `from_registry`, stamps via `with_runtime` | ✓ WIRED (`driver/mod.rs:843-862`) |
| `src/driver/run.rs` | `src/executor/runtime.rs` | `build_executor(args, project.runtime())` at both call sites | ✓ WIRED (iteration spawn `run.rs:3179` and `consult_model_seam` `run.rs:2067`) |
| `src/executor/codex.rs` | `src/executor/claude.rs` | shared Coordinator, `ReaderItem::Codex` | ✓ WIRED |
| `src/executor/claude.rs` | `src/executor/outcome.rs` | synthesized turns pushed onto `envelopes` | ✓ WIRED |
| `tests/spawn_seam_guard.rs` | `src/executor/codex.rs` | SPAWN_ALLOWLIST + capability-type assertion | ✓ WIRED |

## Anti-Patterns Found

None. No TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER markers in the new or modified files. No literal permission-bypass or full-access-sandbox strings anywhere under `src/` (only runtime-assembled halves in tests, by design, to keep the guard's own scan honest).

## Deviations Review

The SUMMARY documents 8 deviations from plan; the only one touching an existing test's assertion body is `src/registry.rs`'s `drive_untrusted_fields` corpus count (11→12, "ELEVEN"→"TWELVE"). This is judged **legitimate**: it is forced by making `DriveError::RuntimeUnrecognized` participate in an exhaustive, wildcard-free match, and it *widens* an untrusted-value redaction test rather than weakening any assertion — the new variant is added to the subject list that must prove it never leaks raw control/invisible characters. No other pre-existing test body was edited (the two `run.rs` in-module tests are type-only wraps in `AgentExecutor::Claude(..)`).

## Session Detection Deferral

Sanctioned by the goal's MVP clause. `src/session_detector.rs` is untouched in this diff; the remainder todo's item 1 records the deferral with a concrete plan (pgrep pass, rollout-fd session id, `AgentSession` type) and names ID-3's reasoning (avoiding a Claude-only resume UI wrongly offering a Codex thread id, and sibling-item file ownership of `detail.rs`).

## Human Verification Required

None. Every must-have was settled by direct code inspection (argv shape, env scrub, refusal paths, wiring, remainder todo content) plus the codebase's own test suite, which exercises the exact behaviors claimed (fixture-replayed end-to-end drive() calls, not mocked). The orchestrator's separately running `cargo test`/`clippy` gate is the remaining confirmation that the code compiles and every test actually passes; nothing here is a subjective/UX call that needs a human.

## Gaps Summary

None found blocking the item goal. The MVP scope boundary (session/liveness detection, TUI launch/resume, steering, model seam lift, etc.) is explicitly and traceably deferred to the new remainder todo, exactly as the item's goal instructed for an over-large task.

---

_Verified: 2026-09-22_
_Verifier: Claude (gsd-verifier)_
