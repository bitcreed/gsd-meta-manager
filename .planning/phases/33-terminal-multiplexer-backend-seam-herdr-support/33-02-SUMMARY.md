---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
plan: 02
subsystem: ui
tags: [multiplexer, tmux, herdr, nesting, focus, proc, rust]

requires:
  - phase: 33-01
    provides: "MultiplexerBackend trait, REGISTRY, EnvSnapshot, ExecArgv sink, herdr launch over layout.apply"
provides:
  - "Innermost-multiplexer ordering from env plus /proc ancestry (order_innermost with an ancestry argument, proc_chain, parse_stat_ppid, parse_stat_comm, is_server_comm on the trait)"
  - "herdr focus: pane list, three-tier join (session id, pid, unique cwd), agent focus with tab focus fallback, CLI verb allowlist"
  - "D-14 design note as module rustdoc of src/multiplexer/mod.rs (eight tools mapped onto detect / focus / open)"
  - "README and docs describe tmux and herdr truthfully and cite src/multiplexer"
  - "Two ignored live herdr tests pinning the exec-direct boundary and the pane-list / process-info JSON names"
affects: [UAT for SC1 (Tab on fresh and resumed sessions inside herdr), requirements MUX-01 MUX-02 MUX-04]

actuals:
  tokens: 17000
  tasks: 3
  commits: 3
plan_head_before: ea6d9b4b5fa3def6652f80440d6feb6a2e60ef2c
plan_head_after: 2dd6128180059b5845a4b1dc925af6913dfa6a75

tech-stack:
  added: []
  patterns:
    - "Pure ordering rule over (EnvSnapshot, nearest-first ancestry); the only /proc reader is one cfg(linux) function whose non-Linux twin returns an empty chain"
    - "Herdr-private CLI edge with a (group, verb) allowlist checked before spawn; the program path is an instance field so a fixture script stands in for herdr"
    - "Join tiers return One / None / Ambiguous, so list order can never decide a match and ambiguity is a refusal"
    - "Registry invariant test: a fixture table must equal REGISTRY by name in both directions"

key-files:
  created: []
  modified:
    - src/multiplexer/mod.rs
    - src/multiplexer/tmux.rs
    - src/multiplexer/herdr.rs
    - tests/spawn_seam_guard.rs
    - README.md
    - docs/CONFIGURATION.md
    - docs/ARCHITECTURE.md
    - docs/GETTING-STARTED.md
    - docs/DEVELOPMENT.md

key-decisions:
  - "[inferred - audit] D-11 corrected by research: herdr exposes no tty, so the pid tier replaces the tty step; agent focus <pane_id> composes workspace, tab and pane in one call, tab focus is the fallback. Exact-match and refuse-on-ambiguity unchanged."
  - "[inferred - audit] D-07 off Linux: proc_chain is empty, so the static order (tmux first) is the only rule there."
  - "[inferred - audit] Cwd tier candidates are not restricted by agent kind, so a Codex session is focusable in herdr too. Residual: a plain shell pane in the same directory is a cwd candidate; the last-resort tier then focuses it (agent focus reports agent_not_found, tab focus lands on its tab). Pid and session-id tiers run first."
  - "[inferred - audit] A failing process-info for one candidate counts as no match, so one dead pane cannot block focus."
  - "[inferred - audit] An empty session id skips the session-id tier, and an empty agent_session.value never matches."
  - "[inferred - audit] The pid tier's chain pids include the session pid itself plus its /proc ancestors, so the join still works where /proc cannot be read."
  - "[inferred - audit] If HERDR_SOCKET_PATH is absent from the snapshot, herdr_cli fails (Failed) rather than inheriting the process environment."

requirements-completed: [MUX-01, MUX-02, MUX-04]

duration: 1 session
completed: 2026-11-10
status: complete
---

# Phase 33 Plan 02: Nesting precedence, herdr focus and the design note Summary

**The innermost multiplexer now wins when tmux and herdr are nested (env gate decides candidacy, the nearest server in the TUI's own /proc ancestry decides order), `Tab` inside herdr lands in the session's pane through a never-guess three-tier join, and the trait's design is recorded against eight tools next to the code.**

## Performance

- **Tasks:** 3 of 3 (all `auto`; Tasks 1 and 2 `tdd`)
- **Commits:** 3 task commits (plus this SUMMARY commit)
- **Files:** 0 created, 9 modified, 0 deleted

## Accomplishments

- **MUX-02 nesting:** `order_innermost(registry, env, ancestry)` takes the first ancestry entry whose comm is some candidate's `is_server_comm` (tmux: exactly `tmux: server` or `tmux`; herdr: exactly `herdr`) and moves it to the front. An ancestor whose env gate fails is ignored, a `tmux: client` never counts, an empty or inconclusive chain keeps the static order. The six measured shapes are table-tested from `(pid, comm)` plus env fixtures. `every_registered_backend_is_selected_by_its_own_fixtures` fails when a backend is registered without its env and server-comm fixture row (and when a fixture row names no backend).
- **MUX-04 herdr focus:** `pane list`, then session id (kind `"id"`, exact), then pid (`pane process-info` among the cwd candidates; session pid among foreground pids, or `shell_pid` among the session's `/proc` chain), then a unique cwd. Two or more matches on any tier is `RefusedForSafety("refusing to guess: ...")`. Then `agent focus <pane_id>`, falling back to `tab focus <tab_id>` on `agent_not_found`, with a `tab only: ... N panes` note when the tab holds more than one pane. Reversing the pane list changes nothing (tested on every tier).
- **CLI edge:** `herdr_cli` refuses any `(group, verb)` outside `CLI_VERBS` (`pane list`, `pane process-info`, `agent focus`, `tab focus`) before spawning, passes `HERDR_SOCKET_PATH` from the snapshot, and ids from herdr output that are empty or start with `-` are refused before they reach argv (CWE-88).
- **MUX-01 design note:** the `//!` rustdoc of `src/multiplexer/mod.rs` has the eight-tool table with confidence tags, the method-to-column mapping, the five call sites, `TERM_PROGRAM` as a recorded-but-unused signal, and the D-04 / D-11 supersession notes. Unsupported is marked for hc and Rex detection and for GNU screen `open`.
- **Docs:** README, `docs/CONFIGURATION.md` (HERDR_ENV, HERDR_SOCKET_PATH, HERDR_WORKSPACE_ID rows; corrected fallback chain), `docs/ARCHITECTURE.md`, `docs/GETTING-STARTED.md` and `docs/DEVELOPMENT.md` no longer reference the deleted `terminal_switch` or claim tmux-only behaviour.

## Verification

| Check | Result |
|---|---|
| `cargo test --no-fail-fast` (all suites) | 3714 passed, 0 failed, 19 ignored (baseline on master: 3674 passed, 17 ignored; +40 tests, +2 ignored live tests) |
| `cargo test --lib multiplexer` | 76 passed, 0 failed, 3 ignored |
| `cargo test --test spawn_seam_guard` | 10 passed (herdr.rs on the allowlist, `CLAUDE_ARGV_SITES` census untouched) |
| `driver_router_conformance` (oracle installed locally) | passed |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --target aarch64-apple-darwin --all-targets -- -D warnings` | clean (non-Linux `proc_chain` stub compiles) |
| `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings` | clean |
| Live herdr tests (`cargo test --lib multiplexer::herdr -- --ignored --nocapture`, run inside herdr 0.9.5 on this machine) | `test result: ok. 2 passed; 0 failed`; no skip message printed, so both really ran |
| Leftover tabs after the live run | `herdr tab list` shows only `wA:t1` |

Live test output, verbatim:

```
running 2 tests
test multiplexer::herdr::tests::the_live_herdr_pane_list_and_process_info_parse ... ok
test multiplexer::herdr::tests::the_herdr_layout_apply_exec_direct_boundary_holds_against_a_live_server ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3000 filtered out; finished in 0.05s
```

The exec-direct test sent a real `layout.apply` with `env touch -- <dir>/lit a b;touch INJECTED $HOME *`: the literal-named file appeared and no `INJECTED` file did.

## Task Commits

1. **Task 1: the innermost multiplexer wins when nested** - `8f33690` (feat)
2. **Task 2: focus inside herdr lands in the session's pane** - `a283da8` (feat)
3. **Task 3: design note, corrected docs, live herdr tests** - `2dd6128` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Dead-code on non-Linux for the stat parsers**
- **Found during:** Task 1 (`clippy --target aarch64-apple-darwin` / `x86_64-pc-windows-gnu`)
- **Issue:** `parse_stat_comm` / `parse_stat_ppid` are only called by the Linux `proc_chain`, so non-Linux builds failed `-D warnings`.
- **Fix:** gated both with `#[cfg(any(target_os = "linux", test))]` (tests still run on every OS); `PROC_CHAIN_MAX` is Linux-only.
- **Commit:** `8f33690`

**2. [Rule 2 - Stale reference] `docs/DEVELOPMENT.md` still listed `terminal_switch.rs`**
- **Found during:** Task 3 acceptance grep over the repo
- **Issue:** outside the plan's file list, but the plan's intent is that docs cite only `src/multiplexer`.
- **Fix:** replaced the entry with a `multiplexer/` line.
- **Commit:** `2dd6128`

**3. Rustdoc link hygiene.** New doc text uses `\[inferred\]` / backticks instead of unresolvable intra-doc links; the repo's pre-existing `cargo doc` warnings (including older `[inferred]` headings from 33-01) were not touched.

No architectural deviations. Plan steps otherwise executed as written.

## Known Stubs

None.

## Threat Flags

None. All new surface is the planned T-33-09 to T-33-11 surface (herdr CLI argv, ids from herdr output, `/proc` comm), mitigated as specified: exact equality and ambiguity refusal on every tier (T-33-09), `validate_id` (T-33-10), `CLI_VERBS` checked before spawn with a test asserting the fixture log stays empty (T-33-11).

## Notes for the orchestrator

- STATE.md and ROADMAP.md were not touched, per the dispatch instruction. REQUIREMENTS.md was also left alone (as in 33-01), so MUX-01, MUX-02 and MUX-04 are not yet checked off there; this plan completes them.
- Not pushed (origin is public).
- No `gsd-plan-head-before` ledger file could be written (the worktree git dir is outside the allowed write path), so `plan_head_before` is the worktree base `ea6d9b4` and `commits: 3` is `git rev-list --count ea6d9b4..HEAD` measured before this SUMMARY commit.
- Open UAT (cannot be settled by tests): Tab on a freshly started `claude` (session_id None, pid tier) and on a resumed one (session-id tier) inside herdr lands in its pane, and `n` opens a herdr tab. RESEARCH assumptions A4 (same id under resume) and A6 (`foreground_processes` lists the claude pid while tools run) remain unconfirmed; if either is wrong the pid tier's shell_pid-in-chain clause still matches.
- Residual (accepted, T-33-13): herdr CLI calls are blocking on the UI thread, bounded in practice by the measured under-10 ms latency but not by a timeout.

## Self-Check: PASSED

- Modified files present: `src/multiplexer/mod.rs`, `tmux.rs`, `herdr.rs`, `tests/spawn_seam_guard.rs`, README and four docs files.
- Commits `8f33690`, `a283da8`, `2dd6128` are ancestors of HEAD.
- `grep -rn "crate::terminal_switch" src tests` prints nothing; `grep -c terminal_switch` is 0 in README and all four docs files the plan names.
