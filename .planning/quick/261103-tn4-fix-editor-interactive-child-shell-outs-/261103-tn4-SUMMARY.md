---
quick_id: 261103-tn4
phase: quick-261103-tn4
plan: 01
subsystem: tui-input
tags: [input, editor, crossterm, tty, regression-test]
status: complete
requirements: [QUICK-261103-tn4]
key-files:
  created: [src/input.rs]
  modified: [src/lib.rs, src/event.rs, src/tui.rs, src/main.rs, .planning/codebase/ARCHITECTURE.md]
commits: 2
plan_head_before: 5c2c2e0dba6eaf33567e2a2fb94db5698603cce6
plan_head_after: 6d55fb7b51fe0f133f34983ff7ad17f2c63d4b46
actuals:
  tasks: 2
  commits: 2
---

# Quick 261103-tn4: Editor shell-outs no longer race the TUI input reader

**Root cause:** crossterm's `EventStream` runs a never-joined background thread that polls and reads tty bytes, and `main.rs` never stopped it around the editor, so the TUI swallowed about every other keystroke meant for vim.

**Fix:** an app-owned std-thread reader (`src/input.rs`, `InputReader`: poll 50ms + read, `suspend()` blocks until parked or exited, exit guard, Drop-stop) plus one helper, `input::run_with_terminal_suspended` (suspend -> release terminal -> child -> reclaim terminal -> resume). `tui::Handoff` is the production `TerminalHandoff` (mouse capture remembered, off during the child, back on only if it was on). The `pending_editor` block in `main.rs` uses it; `tui::init()` now appears once in main.rs (startup).

## Commits
- `0f88ef3` fix: reader, helper, Handoff, rewiring, regression + control tests
- `6d55fb7` test: three edge-path tests (in-flight poll, dead reader, drop)

## RED proof
Regression test run against a helper body of release -> child -> reclaim (old shape):
```
assertion `left == right` failed: tty poll/read calls while the child owned the terminal
  left: 20
 right: 171
```
Control test passed on the same body (harness sees reads when the reader is not suspended). After adding suspend first / resume last, both pass. The 5 `input::tests` were run 3 times in a row, stable.

## Gates
- `rtk proxy cargo test --no-fail-fast`: 59 suites, 3592 passed, 0 failed, 17 ignored (lib: 2879 passed, 1 ignored).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean. Cargo.toml / Cargo.lock unchanged.

## Shell-out inventory (live grep)
Only `main.rs` editor child hands the tty to a child while the TUI runs. Others: `launcher::process::run_interactive` is called only from `driver_cli.rs` (TUI not running); the rest of `launcher/process.rs` uses null/piped stdin; `project_creator`, `session_detector`, `envelope/scan`, `state_reader/git_ops`, `bounded_output`, `text`, `driver/*`, `terminal_switch` (tmux/which) capture output or null stdio; `ui/screens/detail.rs:4989/5511` `.spawn()` a separate terminal emulator without waiting. No other site routed through the helper.

## Deviations
None for behavior. Note: `App::after_editor` now runs after the terminal is reclaimed (previously before `tui::init()`); it does no terminal I/O, so there is no behavior change.

## Inferred decisions (audit)
- Module `src/input.rs` in the library (so `cargo test --lib` reaches the helper main.rs calls); POLL_TIMEOUT 50ms.
- No drain of crossterm's buffer on resume.
- `futures` dependency is now unused in src but Cargo.toml was left untouched per the plan.
- `.planning/codebase/ARCHITECTURE.md` edits are left uncommitted for the orchestrator's docs commit (task commits were code-only).

## Known Stubs
None.

## Self-Check: PASSED
