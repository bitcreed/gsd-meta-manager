---
phase: quick-261111-h9f
plan: 01
subsystem: multiplexer, sessions-tab
tags: [multiplexer, tmux, herdr, sessions, readme]
requires: []
provides:
  - Sessions-tab location column (tmux target, herdr pane with tmux via, dash, pending glyph)
  - MultiplexerBackend::locate_session / locate_tty (focus's joins without the act)
  - README "Terminal multiplexers" section
affects: [src/multiplexer, src/app.rs, src/ui/screens/detail.rs, README.md]
tech-stack:
  added: []
  patterns: [read-only twin of focus, PID-keyed cache with start-time reuse guard, in-flight flag + spawn_blocking + catch_unwind]
key-files:
  created: [src/multiplexer/location.rs]
  modified:
    - src/multiplexer/mod.rs
    - src/multiplexer/tmux.rs
    - src/multiplexer/herdr.rs
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/render_escape_guard.rs
    - tests/focus_entry_guard.rs
    - README.md
key-decisions:
  - "Location is resolved with the focus routing (ProcFs::session_context + plan_focus); focus is rebuilt on the same locate joins"
  - "Cache is PID-keyed and carries start_time; a mismatch reads as pending"
status: complete
actuals:
  tokens: 15600
  tasks: 3
  commits: 4
plan_head_before: 71f6a8e63a1c6e485287371e9cd9d71788de2fcd
plan_head_after: 029f69dde78a87b5969430d8e9f470b784281437
commits: 4
completed: 2026-11-11
---

# Phase quick-261111-h9f Plan 01: Sessions panel multiplexer location column Summary

Every Sessions-tab row now ends with where its session lives (`tmux 0:1.2`, `herdr w2:p1 (via tmux 0:0.0)`, a dash, or a pending glyph), resolved by the same per-session routing focus uses and read from a PID-keyed cache; the README gained a Terminal multiplexers section.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 (tracer) | d29e5dd | feat(quick-261111-h9f): show each session's tmux location in the Sessions tab |
| 2 | d3cafec | feat(quick-261111-h9f): locate herdr sessions and the tmux pane that shows their client |
| 3a | 575bb93 | test(quick-261111-h9f): guard the locate seam and probe the location column for escapes |
| 3b | 029f69d | docs(quick-261111-h9f): README terminal multiplexers section |

## What the column shows

- tmux session: `tmux <session:window.pane>`, e.g. `tmux 0:1.2`.
- herdr session whose client is in a tmux pane: `herdr w2:p1 (via tmux 0:0.0)`.
- herdr session whose client is in no tmux pane: `herdr w2:p1`.
- herdr match by working directory only: `herdr w2:p1?`.
- No multiplexer finds it (including ambiguous or failed lookups, reasons at `debug`): the em dash (U+2014).
- No resolution for this PID and start time yet: MIDLINE_ELLIPSIS (U+22EF).

The two herdr entry tests asserted exactly `herdr w2:p1 (via tmux 0:0.0)` (call log exactly `herdr pane list` then tmux `list-panes`) and `herdr w2:p1`.

## Gates

- Full suite (`cargo test --no-fail-fast`, raw output to file): `full-suite exit=0 passed=3822 failed=0 ignored=19` (baseline 3808 + 14 new tests).
- Clippy `--all-targets -D warnings`: host exit 0; `cargo check --locked` and `cargo clippy --locked` exit 0 for aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu (release.yml's matrix, all installed).
- `cargo fmt --all -- --check` clean. No `allow(dead_code)` added. Nothing pushed. Tree clean after the final task.
- `CLI_VERBS` unchanged (four entries). Locate tests assert call logs contain no `agent focus`, `tab focus`, `select-*`.
- The escape guard was checked red by temporarily rendering a raw target, then restored.

## Deviations from Plan

None - plan executed as written. Tests were written together with the implementation rather than strictly red-first (the new types did not exist, so a red run was a compile error).

## Inferred decisions (carried from the plan)

- [inferred — audit] Format `<backend> <target>`; tmux target is `session:window.pane`, herdr target is the pane id; ` (via <backend> <target>)` appended when the multiplexer's client is shown in another multiplexer's pane.
- [inferred — audit] The location cell is LAST on the row, after the status word.
- [inferred — audit] Three non-location states: dash (all backends answered, none found it), MIDLINE_ELLIPSIS (pending), trailing `?` for a herdr working-directory-only match.
- [inferred — audit] Every session is re-resolved on every `SessionsDetected` (~5 s poll) and once at startup; an in-flight flag prevents stacking.
- [inferred — audit] Cache key is the PID, with the session's `start_time` stored; a mismatch reads as pending (PID reuse guard).
- [inferred — audit] No per-batch memoization of the `/proc` scan for herdr clients.
- [inferred — audit] Location is resolved for all detected sessions, not only the open project's.
- [inferred — audit] focus_entry_guard extended to the locate internals; `locate_sessions(` pinned to `src/app.rs`.
- [inferred — audit] Executor choice: the `locate_with_procs` winner's `via` uses the first other reachable backend (plan order) whose `locate_tty` answers, mirroring `reveal_client`'s loop.
- [inferred — audit] Executor choice: commits were made on `master` directly, as the orchestrator's sequential-execution instruction directed (not pushed).

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model (new surface is a periodic read-only `list-panes` / `pane list` / `pane process-info` per live session).

## Self-Check: PASSED

- src/multiplexer/location.rs: FOUND
- Commits d29e5dd, d3cafec, 575bb93, 029f69d: ancestors of HEAD
