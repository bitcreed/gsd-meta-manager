---
phase: quick-260923-lra
plan: 01
subsystem: docs
tags: [readme, platform, sessions, codex, macos]
requires: ["260923-lr9"]
provides: ["README Platform support note"]
affects: [README.md]
key-files:
  created: []
  modified: [README.md]
decisions:
  - "Platform support subsection lives under ## Requirements, before ### Compatibility"
  - "Features auto-registration bullet and Quick start step 4 left byte-identical although Codex sessions also auto-register"
metrics:
  completed: 2026-09-23
status: complete
actuals:
  tasks: 1
  commits: 1
plan_head_before: 45c2138
---

# Quick 260923-lra: README Platform support note Summary

README now says that live session detection, for both Claude and Codex, reads
Linux `/proc` and so works on Linux only. On macOS no sessions are detected
but the rest of the TUI works. It also says Codex detection is best-effort,
excludes `codex exec`, and cannot resume a session. Both headline session
bullets link to `#platform-support`.

## Commit

- `25dce1e` docs(quick-260923-lra): note Linux-only /proc session detection and best-effort Codex (README.md only, +20/-2)

## Verification

- The plan's `<automated>` check printed `LRA-OK` before the commit.
- No test pins the top-level README.md (grep for `include_str!`/`README.md` in `src/` and `tests/` found only fixture and temp-file uses), so `cargo test` was not needed.

## Code citations for README claims

| README claim | Backed by |
|---|---|
| Detection (Claude and Codex) reads `/proc`, Linux only | `src/session_detector.rs:60-79` (`detect_sessions`, "inspecting the Linux /proc filesystem"), `:125-129`, `:158-164` |
| macOS: no sessions, nothing crashes | `src/session_detector.rs:66-67`, `:129`, `:159`/`:164` (`.ok()?` silently skips pids) |
| Auto-registration consumes detected sessions (quiet on macOS) | `src/app.rs:1005-1007`, `:1255-1257`; `src/registry.rs:878-897` |
| Interactive `codex` shown alongside Claude | `src/session_detector.rs:74-78`, `:158-174` (`SessionKind::Codex`) |
| Non-interactive runs such as `codex exec` excluded | `src/session_detector.rs:88-97`, `:199-214`; stdin must be a terminal `:163`, `:218-220` |
| `Tab` under tmux reaches Codex sessions | `src/ui/screens/detail.rs:2826-2857` ("Claude or Codex session"), `src/ui/screens/normal.rs:607-620`, `src/terminal_switch.rs:271-285` (tty-based, kind-agnostic) |
| Codex cannot be resumed; resume is Claude-only | `src/ui/screens/detail.rs:139-140`, `:152-160` (`resumable_session_id`), `src/session_detector.rs:8-11` |
| Best-effort: relies on Codex CLI process details | `src/session_detector.rs:84-105` (argv subcommand list, `thread-writer-locks` dir name), `:225-227` ("inferred heuristic") |

## Deviations from Plan

None -- plan executed exactly as written.

## Inferred decisions (audit)

Carried over from the plan:

1. `depends_on: ["260923-lr9"]` for semantic ordering. lr9 was already on master (2054f81/5e49ba3/45c2138), so the precondition passed (`SessionKind` present in `src/`).
2. Placement: `### Platform support` under `## Requirements`, before `### Compatibility`. The headline bullets link to it instead of repeating the caveat.
3. The macOS wording stays conservative. It only says "the rest of the TUI -- everything read from `.planning/` -- works normally" and promises nothing else unverified on macOS.
4. One `type="auto"` task, with no tracer and no TDD, because this is a docs-only edit.

Made during execution:

5. Codex sessions also auto-register, because `auto_register_from_sessions` does not filter on `kind`. The Features auto-registration bullet and Quick start step 4 still say "`claude`" only. I left them unchanged because the plan requires Quick start to stay byte-identical and limits the edits to the two session bullets. A later docs pass may want to widen that wording.
6. The "best-effort" reason is limited to "relies on Codex CLI process details that may change between releases". That is grounded in the hardcoded subcommand list and the lock-dir name heuristic.

## Self-Check: PASSED

- README.md contains `### Platform support` (FOUND)
- Commit `25dce1e` present on master (FOUND)
