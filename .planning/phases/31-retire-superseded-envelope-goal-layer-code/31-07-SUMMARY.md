---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 07
subsystem: retirement
tags: [rust, retirement, executor, codex, claude-transport, exec-channel, process-wrap, D-04]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-04 removed `drive` (the only caller of either executor); 31-05 removed envelope_env; USER DECISION 2026-10-27 lifted the D-04 hold
provides:
  - no Codex executor and no in-process Claude transport: src/executor is now runtime.rs (AgentRuntime config value) and stream_json.rs (wire parser) only
  - no TUI exec-event channel: pump has two arms (actions, redraw timer)
  - no process-wrap or direct uuid dependency; Cargo.lock regenerated with removals only
  - SC4 re-certified on the final tree: FULLSUITE green, four clippy targets clean, container dry run exit 0 with six gates PASS
affects: [31-VERIFICATION]

actuals:
  tokens: 141154
  tasks: 3
  commits: 2

plan_head_before: 1aca88d201ce91b1627b124c39baac74080d84ed
plan_head_after: 2e2e3355e3319472ae21bc7a96f9c6d4ef19ed79
commits: 2

tech-stack:
  added: []
  patterns:
    - "a census row / allowlist entry is deleted in the same commit as the file it names; the bidirectional vanished checks and the async floor are re-measured, never relaxed"
    - "tests whose subject was a deleted type are retargeted to the surviving gate (registry::is_opted_in) instead of dropped, so the opt-in assertion survives"

key-files:
  created: []
  modified:
    - src/executor/mod.rs
    - src/executor/runtime.rs
    - src/error.rs
    - src/main.rs
    - src/main_loop.rs
    - src/app.rs
    - src/registry.rs
    - src/session_detector.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - tests/spawn_seam_guard.rs
    - tests/async_blocking_guard.rs
    - Cargo.toml
    - Cargo.lock
    - docs/TESTING.md
  deleted:
    - src/executor/{codex, codex_json, claude, gate, outcome}.rs
    - tests/executor_lifecycle.rs
    - tests/executor_transport.rs
    - tests/fixtures/codex/ and tests/fixtures/fake-codex.sh
    - tests/fixtures/fake-claude{, -deaf, -echo, -late-init, -orphan, -silent, -slow, -spawner}.sh

key-decisions:
  - "DriverOutputLine / DriverOutput KEPT: live via src/ui/screens/driver_run.rs (launcher tail pushes into it), so the plan's inferred-delete conditional resolved to keep, together with its render-escape census row"
  - "uuid REMOVED as a direct dependency (no `uuid::` user left); it stays in Cargo.lock transitively through wezterm-blob-leases, so the MSRV floor measured by cargo metadata is unchanged at 1.89.0"
  - "MIN_ASYNC_BODY_LINES unchanged at 800: measured 2380 -> 1229, still above the floor, so the plan's lower-only-if-below rule did not fire"
  - "[inferred - audit] runtime::claude_command (plus gsd_command_parts and GSD_COMMAND_PREFIXES) deleted: its only caller was ClaudeExecutor::start_run; not named in the plan's list but it satisfies D-04b's last-caller rule"
  - "[inferred - audit] main_loop keeps three new pump tests (keypress applied ahead of the timer, idle returns RedrawTick, closed action channel still progresses) because all three original tests were exec-channel tests and pump would otherwise lose its coverage"

requirements-completed: [RETIRE-01, RETIRE-03]

coverage:
  - id: D1
    description: "The Codex executor is gone; interactive codex session detection and the AgentRuntime config parse still pass"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "cargo test --lib -- session_detector:: executor::runtime:: config:: registry:: (81 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The in-process Claude transport, its error types, the TUI exec channel and process-wrap are gone"
    requirement: RETIRE-01
    verification:
      - kind: integration
        ref: "FULLSUITE results=55 passed=3158 failed=0 ignored=17"
        status: pass
    human_judgment: false
  - id: D3
    description: "SC4 re-certified: container dry run exits 0 with six gates PASS on the final tree"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "./scripts/pre-tag-check.sh --container v1.15.0 (exit 0)"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-28
status: complete
---

# Phase 31 Plan 07: Retire the executor (D-04) Summary

**The experimental Codex executor, the in-process Claude transport, the dead TUI exec channel and the process-wrap dependency are deleted; the container dry run is green on the final tree (11,398 lines out, 227 in).**

## Performance

- **Duration:** about 45 min (start time not recorded; first task commit 04:48Z)
- **Completed:** 2026-10-28
- **Tasks:** 3 (task 3 is verification only, no commit)
- **Files:** 42 changed, 227 insertions, 11,398 deletions

## Accomplishments

- Task 1 (tracer): `executor::{codex, codex_json}`, `runtime::{AgentExecutor, codex_command}`, `tests/fixtures/{codex/, fake-codex.sh}`, the session-detector codex-argv test and the Codex rows in both census files are gone. `AgentRuntime::{Claude, Codex}`, `from_config_value`, `resolve` and interactive `pgrep -x codex` detection stay and pass. The Claude supervisor needed its Codex arms (`ReaderItem::Codex`, `StreamProtocol`) removed so the tree compiled between the two commits.
- Task 2: `executor::{claude, gate, outcome}`, every transport item in `executor/mod.rs` (module doc rewritten), `SpawnError`/`SendError`/`CapabilityError`/`OptInError` (and their tests, `LockError` kept), `exec_tx`/`exec_rx`, `ExecEvent`/`EXEC_BATCH`/`EXEC_CHANNEL_CAPACITY`/`PumpOutcome::ExecEvents` and the exec arm of `pump`, `App::apply_exec_event`, `AppContext::{run_states, exec_tx}` with every test-context literal, two test binaries, eight `fake-claude*.sh` stand-ins, and the escape-hatch / `DrivableProject` / seam-profile / MCP-set / spawn-closure-comment census tests.
- `Cargo.toml`: `process-wrap` and `uuid` removed; the regenerated `Cargo.lock` diff is 68 deleted lines and nothing else (process-wrap, the `windows` 0.62 family it pulled in, uuid's direct edge).
- `docs/TESTING.md`: the executor fixtures, process stand-ins, Unix-only lifecycle rationale and teardown-grace ignored-tests sections are replaced by a short Transcripts section (still used by `stream_json` and `rate_limit`) and a generic "Ignored tests" note.

## Verification

FULLSUITE (`cargo test --no-fail-fast`, final tree):

| Run | results | passed | failed | ignored |
|-----|---------|--------|--------|---------|
| 31-09 baseline (pre-plan) | 57 | 3294 | 0 | 17 |
| 31-07 (this plan) | 55 | 3158 | 0 | 17 |

`expected results=55` (52 `tests/*.rs` + 3). Delta: 2 fewer result lines (the two deleted test binaries) and 136 fewer passes, inside the plan's 130-160 estimate: executor_lifecycle and executor_transport binaries, the unit tests that lived in executor/{claude, codex, codex_json, gate, outcome, mod}.rs and the Codex runtime tests, the session-detector codex-argv and claude `build_argv` round-trip tests, the three exec-channel `pump` tests (replaced by three smaller ones), the exec-event app tests, the `OptInError` Display tests in error.rs, and the escape-hatch / DrivableProject / seam-profile / MCP / spawn-closure census tests. Ignored stays 17 (no ignored test lived in the deleted binaries at HEAD).

Clippy: `cargo clippy --locked --all-targets -- -D warnings` clean; cross-target `--locked --target {x86_64-pc-windows-gnu, aarch64-apple-darwin, x86_64-apple-darwin} --all-targets -- -D warnings` all exit 0.

Container dry run `./scripts/pre-tag-check.sh --container v1.15.0`, exit 0 (inner run exit 0):

```
gate 1  tag / Cargo.toml version   PASS
gate 2  MSRV check                 PASS
gate 3  cargo build --release      PASS
gate 4  cargo test                 PASS
gate 5  cargo clippy -D warnings   PASS
gate 6  cargo package --locked     PASS
```

No tag created (`git tag --points-at HEAD` empty); nothing pushed or published.

## Task Commits

1. **Task 1: retire the Codex executor (tracer)** - `a23d1d4` (refactor)
2. **Task 2: retire the in-process Claude transport, exec channel, process-wrap** - `2e2e335` (refactor)
3. **Task 3: re-certify the final tree** - no commit (changes no source; Cargo.lock needed no fix)

## Census and floor values

- `MIN_ASYNC_BODY_LINES`: old 800, new 800. Real async body lines measured 2380 (earlier Phase 31 figure) -> 1229. Headroom over the floor is now 429 lines; flagged for the verifier, not lowered because the rule only lowers when the measurement falls below the floor.
- `BLOCKING_HELPERS` loses `RunSnapshot::capture(` (its only `async fn` caller was claude.rs; `every_named_blocking_helper_still_exists_in_the_tree` would have refused the stale name); `ASYNC_BLOCKING_ALLOWLIST` loses both claude.rs entries.
- `SPAWN_ALLOWLIST` loses the `claude.rs`, `codex.rs` and `outcome.rs` entries. `CLAUDE_ARGV_SITES` in session_detector loses the claude.rs Producer row (4 rows -> 3) and `CLAUDE_ARGV_ROUND_TRIPS` loses its executor entry (2 -> 1); `registry` look-alike subject count 9 -> 5.

## Retargeted tests

- `detail::tests::opting_in_from_the_driver_tab_flips_the_spawn_seam` (name kept, cited by historical quick-task docs): the before/after `DrivableProject::from_registry` / `OptInError::NotOptedIn` assertions now use `crate::registry::is_opted_in` on the live config and on the reloaded config from disk.
- `app::tests::removing_a_project_interactively_clears_its_driver_maps` and `every_per_alias_driver_map_is_pruned`: `run_states` assertions replaced by `last_refresh`, which the same removal path clears.
- 31-08's WR-03 unregister-guard tests in delete_confirm.rs untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Codex arms inside claude.rs**
- **Found during:** Task 1
- **Issue:** `executor/claude.rs` imported `codex_json` and carried `ReaderItem::Codex` and `StreamProtocol::CodexExecJson`, so deleting codex_json broke the build before Task 2 removes claude.rs.
- **Fix:** removed those arms (and the single-variant `StreamProtocol`) in the Task 1 commit so every commit compiles.
- **Files modified:** src/executor/claude.rs (deleted in Task 2)
- **Commit:** a23d1d4

**2. [Rule 3 - Blocking] Items outside the plan's file list that named deleted types**
- **Found during:** Task 2
- **Issue:** `src/registry.rs` look-alike control iterated `OptInError`; `src/app.rs` and `detail.rs` tests used `RunState`/`DrivableProject`; spawn_seam_guard's `every_process_spawn_site...` test ended with a claude.rs `DrivableProject` assertion; `tests/driver_router_conformance.rs`, `src/config.rs`, `src/state_reader/{disk_status, queue_md}.rs` carried doc references.
- **Fix:** removed or retargeted as described above; doc-only references re-worded.
- **Commit:** 2e2e335

**3. [Rule 2 - Missing critical] pump kept test coverage**
- **Issue:** all three `main_loop` tests were exec-channel tests, so deleting them would leave `pump` untested.
- **Fix:** added three small deterministic tests (keypress ahead of the timer, idle -> `RedrawTick`, closed channel does not panic). The saturated-channel frame-rate test was dropped with its subject.
- **Commit:** 2e2e335

**Total deviations:** 3 auto-fixed (2 Rule 3, 1 Rule 2). No scope creep beyond the inferred `claude_command` deletion recorded under decisions.

## Known Stubs

None.

## Deferred Issues

- `registry::check_prompt_input_drift` and `OptInDrift` now have no production caller at all (their one caller was `DrivableProject::from_registry`); they are `pub` library items with live registry tests, so nothing warns. Out of scope here; candidate for a later retirement pass.
- `tui-textarea` is still unused (pre-existing, noted per plan, not touched).
- `src/journal/redact.rs` `RedactedLine` remains `#[allow(dead_code)]` as set by 31-05.
- `cargo fmt --check` reports one pre-existing diff in src/ui/screens/detail.rs (~14890, `repo_tagging`), unchanged by this plan.

## Threat Flags

None. No new endpoint, auth path or trust-boundary file access; this plan only removes surface.

## Self-Check: PASSED

- a23d1d4 and 2e2e335 are ancestors of HEAD.
- Deleted files absent; `grep -c process-wrap Cargo.toml` 0 and `name = "process-wrap"` in Cargo.lock 0; `-w DrivableProject|ExecutionHandle|apply_exec_event|ExecEvent` over src and tests 0; `find tests -maxdepth 1 -name '*.rs'` 52; `tests/fixtures/fake-*` 0; `pub mod runtime` and `pub mod stream_json` present.
