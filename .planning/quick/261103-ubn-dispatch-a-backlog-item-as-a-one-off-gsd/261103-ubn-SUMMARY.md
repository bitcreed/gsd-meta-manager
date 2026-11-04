---
phase: quick-261103-ubn
plan: 01
subsystem: oneoff
tags: [oneoff, backlog, tui, experimental, cli]
requires: [261103-f5y, 261103-hx7]
provides:
  - "TaskSpec::Backlog { number, title, entry } carried TUI key -> OneoffRunRequested -> request_oneoff_run -> run_detached -> `task run --backlog` daemon -> prompt"
  - "`t` on the detail view's Backlog tab (experimental-gated) starts the selected item as a /gsd-quick one-off"
  - "`gsd-meta-manager task run <alias|path> --backlog 999.N` (detached and --foreground)"
affects: [src/oneoff, src/state_reader/backlog.rs, src/ui/screens/detail.rs, src/cli.rs, src/task_cli.rs, docs/CONFIGURATION.md]
tech-stack:
  added: []
  patterns: ["untrusted repository text cleaned + bounded + marker-delimited in the prompt", "daemon re-resolves the task from the checkout by a validated id (todo pattern)"]
key-files:
  created: [src/oneoff/backlog.rs]
  modified:
    - src/state_reader/backlog.rs
    - src/oneoff/mod.rs
    - src/oneoff/prompt.rs
    - src/oneoff/launch.rs
    - src/ui/screens/oneoff.rs
    - src/cli.rs
    - src/task_cli.rs
    - src/ui/screens/detail.rs
    - src/app.rs
    - src/ui/screens/help.rs
    - src/ui/screens/mod.rs
    - tests/oneoff_e2e.rs
    - tests/oneoff_runtime_seam_guard.rs
    - docs/CONFIGURATION.md
decisions:
  - "Key `t` on the Backlog tab only (no other view selects a backlog item) [inferred — audit]"
  - "Entry = ROADMAP.md section only; the directory .md files stand in only when there is no section; PROMOTED items are not special-cased [inferred — audit]"
  - "MAX_ENTRY_BYTES = 48 KiB so worst-case build_resume stays under MAX_PROMPT_BYTES [inferred — audit]"
  - "Title = the Backlog tab's own label (description via sanitize_title), falling back to the number [inferred — audit]"
  - "Daemon and parent both re-read the entry from the project checkout (TOCTOU-tolerant, as todos) [inferred — audit]"
  - "OneoffRunStarted text unchanged; the start status already names the backlog item [inferred — audit]"
metrics:
  duration: "~35 min"
  completed: 2026-11-03
status: complete
commits: 2
plan_head_before: 4805b1a67b877ae2a130d9a12732c4b1969f94aa
plan_head_after: 2ffc8db459f1de5da81fd5d07159590f9cd180bd
actuals:
  tokens: 16600
  tasks: 2
  commits: 2
---

# Phase quick-261103-ubn Plan 01: Dispatch a backlog item as a one-off with one key Summary

`t` on the Backlog tab starts the selected item as a sandboxed `/gsd-quick` one-off at once (experimental flag on), carrying the item's number, title and full cleaned ROADMAP.md entry via a new `TaskSpec::Backlog`, with `task run --backlog 999.N` as the CLI entry point and daemon transport.

## Commits

- `b79612f` feat(quick-261103-ubn): `task run --backlog` carries a backlog item end to end (Task 1, tracer)
- `2ffc8db` feat(quick-261103-ubn): `t` on the Backlog tab dispatches the selected item as a one-off (Task 2)

## What was built

- **Task 1 (tracer).** `state_reader::backlog::backlog_dispatch_entry` (ROADMAP section via `roadmap_md::phase_section`, else the labelled dir `.md` bodies through a helper shared with `load_backlog_content`). New `oneoff::backlog` (`valid_backlog_number`, `clean_entry`, `MAX_ENTRY_BYTES`, `backlog_spec`). `TaskSpec::Backlog` with arms in `prompt::build`, `launch::validate`, `launch::daemon_argv` (`--backlog <number>`) and the Confirm display. `TaskRunArgs.backlog` in the required `task` ArgGroup, resolved in `task_cli::run_main`. `ui::screens::oneoff::default_model()` extracted from `confirm()`.
- **Task 2.** Backlog-tab `t` arm in `detail.rs` (gated by `ctx.experimental`, `can_steer` refusal, direct `driver_confirm::dispatch`, no screen pushed), `App::request_oneoff_run` start status naming the backlog item, flag-gated help row under the Backlog `e` row, `AppContext::experimental` doc, `docs/CONFIGURATION.md` (key paragraph, `--backlog` synopsis, env-var row, flag-off list).

## Tracer RED line

Before any code, `rtk proxy cargo test --no-fail-fast --test oneoff_e2e backlog` failed for the expected reason:
`error: unexpected argument '--backlog' found` (clap), 2 failed. After Task 1: 2 passed.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 59 suites, **3623 passed, 0 failed, 17 ignored** (lib: 2908 passed / 1 ignored; envelope and e2e suites ran).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `git diff --stat master -- Cargo.toml Cargo.lock`: empty (no new crates).
- New tests: `oneoff::backlog` (7), `state_reader::backlog` (2), `oneoff::prompt` (6), `oneoff::launch` (2), `oneoff::mod` (compat), `detail` `backlog_one_off_*` (7), `app` (1), `help` (1 + flag-on assertion), e2e (2).

## Deviations from Plan

None - plan executed as written. Two test-harness details worth knowing: the fake harness records `prompt.txt` with newlines escaped (`\n` literal), so the e2e `starts_with` check omits the trailing newline; and the "other tabs" test uses tabs 1 and 2 only, because tab 4 (Git History) spawns a tokio task and needs a runtime.

## Inferred decisions (for later audit)

All from CONTEXT/PLAN, none confirmed by a human:
1. `t` is bound on the Backlog tab only; no other view selects a backlog item.
2. The entry is the ROADMAP section alone; dir `.md` files are only a fallback; PROMOTED items are dispatchable.
3. `MAX_ENTRY_BYTES = 48 KiB` (budget derived against `MAX_PROMPT_BYTES` and the largest `build_resume` block; a worst-case test pins it).
4. Title is the Backlog tab's label (`sanitize_title(description)`), else the number.
5. Parent and daemon both resolve the entry from the project checkout at their own time; the run's clone comes from the last commit (documented in CONFIGURATION.md).
6. The `OneoffRunStarted` text is unchanged ("One-off {run_id} started on {alias}"); naming the item there would need a new Action field (musk M-0042).
7. The END-marker rewrite loops until no `<<< END BACKLOG ENTRY` is left, so `<<<< END ...` cannot re-form a marker.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register (T-ubn-01..06 mitigations implemented: marker-delimited data, END-marker rewrite, `clean_entry`, `valid_backlog_number` before any spawn and in the daemon, `render_for_terminal` on every repo-derived status string, flag guard with an empty-channel test).

## Self-Check: PASSED

- src/oneoff/backlog.rs exists; commits b79612f and 2ffc8db are ancestors of HEAD.
