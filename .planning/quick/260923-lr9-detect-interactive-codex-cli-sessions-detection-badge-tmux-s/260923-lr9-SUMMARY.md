---
phase: quick-260923-lr9
plan: 01
subsystem: session-detection
status: complete
tags: [codex, sessions, badge, tmux, resume-gate]
requires: ["260923-lr8"]
provides: [SessionKind, codex-session-detection, codex-resume-gate]
affects: [dashboard-badge, sessions-tab, help-legend, auto-registration]
tech-stack:
  added: []
  patterns: ["pure /proc parsers + thin I/O glue", "exhaustive-match resume gate"]
key-files:
  created: []
  modified:
    - src/session_detector.rs
    - src/registry.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/help.rs
    - src/app.rs
    - tests/spawn_seam_guard.rs
decisions:
  - "Codex id = lowest lowercase canonical UUID among OPEN thread-writer-locks fds (readlink only)"
  - "Interactive = argv[0] basename exactly codex + no exec/e/app-server/exec-server/mcp/mcp-server before `--` + fd/0 is pts/* or tty*"
  - "Resume gate is an exhaustive match on SessionKind before resolve_launch_plan; Codex always refused"
metrics:
  completed: 2026-09-23
actuals:
  tokens: 12450
  tasks: 3
  commits: 3
plan_head_before: 7cf69101b198e95214f9f6a9d5374f86506dbf3d
---

# Quick 260923-lr9: Detect interactive Codex CLI sessions Summary

Interactive `codex` TUIs are now detected next to `claude` sessions. They get the same dashboard badge, Sessions-tab row (labelled `Codex`), tmux Tab-switch and auto-registration. Resume is refused by an exhaustive `SessionKind` gate placed before any launch planning, so a Codex thread id can never reach `claude --resume=`.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | 2054f81 | feat(quick-260923-lr9): detect interactive Codex sessions alongside Claude |
| 2 | 5e49ba3 | feat(quick-260923-lr9): never resume Codex sessions; label agent in Sessions tab |
| 3 | 45c2138 | docs(quick-260923-lr9): agent-neutral session legend and logs |

## What was built

- **src/session_detector.rs**: `SessionKind { Claude, Codex }` with `label()`. `ClaudeSession.kind` is placed directly after `pid`, and lr8's `tty` doc is untouched. Pure functions: `codex_cmdline_is_interactive`, `is_terminal_device`, `codex_thread_id_from_fd_targets` and `is_lowercase_canonical_uuid`. I/O glue: `pgrep_exact` (still the only `Command::new("pgrep")` site), `read_fd_targets` (capped at 4096) and `build_codex_session`. `detect_sessions` returns the Claude sessions followed by the Codex sessions. There are 9 new unit tests, including a coupling test that runs the real `build_codex_argv` output through the classifier.
- **src/ui/screens/detail.rs**: adds `CODEX_RESUME_UNSUPPORTED`, `NO_SESSION_ID_TO_RESUME` and `resumable_session_id`. The Enter/Sessions branch now early-returns on `Err` before `resolve_launch_plan`, and the Claude body is otherwise byte-identical (`git diff -w`). Rows use the format `PID n | Claude|Codex | Session: … | active`. A Codex row with no id shows `unknown`. The empty state now reads "No active Claude or Codex sessions". There are 4 new tests: the pure gate, Enter on a Codex row, row labels, and the empty state.
- **src/ui/screens/normal.rs**: `no_active_session_status(alias)` is now the single place the Tab-miss text is built, and the detail screen uses it too. The badge docs are agent-neutral. New tests cover the helper and check that a Codex session lights `BADGE_SESSION`.
- **help.rs / app.rs / registry.rs / render_escape_guard.rs / spawn_seam_guard.rs**: wording and docs only. The help legend now reads "an active Claude or Codex session in this directory", with a new test for it.

## Verification

- `rtk proxy cargo test --no-fail-fast`: **2210 passed, 1 failed, 15 ignored** across 49 result blocks. The baseline was 2194/1/15, and the 16 new tests account for the difference. The only failure is the known git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Census tests `every_claude_argv_option_site_under_src_is_adjudicated` and `no_source_line_under_src_requests_a_forked_session` pass, and `CLAUDE_ARGV_SITES` is unchanged.
- `rtk proxy cargo clippy -- -D warnings` exits 0.
- `cargo clippy --keep-going --all-targets -- -D warnings`: the diagnostics are only in the pre-existing baseline files (src/browser.rs, src/project_creator.rs, tests/envelope_*.rs). None are in the files this item touched.
- `grep -rn "No active Claude session" src` and `grep -rn "an active Claude session" src` both find nothing.
- There are no changes to src/terminal_switch.rs, README.md, Cargo.toml or Cargo.lock.
- The code was not tested against a live codex process, because none was running (`pgrep -x codex` returned nothing) and the constraints forbid launching codex. Detection logic is certified by the pure-function tests and the executor-argv coupling test.

## Inferred decisions (audit)

1. **[INFERRED 1]** `kind` sits directly after `pid`, and lr8's `tty` field doc was not edited.
2. **[INFERRED 2]** The Codex session id is the lexicographically lowest stem among OPEN `thread-writer-locks/*.lock` fds. For lowercase UUIDv7 stems that is creation order, so the main thread wins over a later review thread. This is a heuristic.
3. **[INFERRED 3]** A stem counts only if it is a lowercase canonical UUID: 36 bytes, hyphens at offsets 8/13/18/23, everything else `[0-9a-f]`, version nibble unchecked. Anything else gives `None` rather than a fabricated id. This does not reverse D-21-48, because a Codex id feeds no parser.
4. **[INFERRED 4]** The lock directory is matched by its name `thread-writer-locks`, not anchored to `~/.codex`, so `$CODEX_HOME` works.
5. **[INFERRED 5]** Interactive means all three of these hold. (a) The argv[0] basename is exactly `codex`, which excludes `codex-linux-sandbox`. (b) No element before the first `--` is one of exec/e/app-server/exec-server/mcp/mcp-server; this applies anywhere, not only at argv[1]. (c) fd/0 is `pts/*` or `tty*`. The `e` alias comes from the codex-rs CLI definition and was not probed at 0.155.1. The tty rule applies to Codex only.
6. **[INFERRED 6]** A Codex row with no id renders `unknown`, not `new session`.
7. **[INFERRED 7]** The neutral wording is "Claude or Codex". The refusal text is "Resume is not supported for Codex sessions — press Tab to switch to it".
8. **[INFERRED 8]** The fd scan is capped at `CODEX_FD_SCAN_LIMIT = 4096` per process. A process holding more fds is under-detected.
9. **[EXECUTOR]** The Enter/Sessions gate uses an early-return `let sid = match resumable_session_id(session) { Ok(sid) => sid, Err(msg) => return … }` instead of nesting the body in `Ok(sid) => { … }`. This keeps the Claude body verbatim at one less indent level. Two `match tmux_err` / `None =>` arms in that body were reflowed to rustfmt's output for the new indentation; this is whitespace and braces only.
10. **[EXECUTOR]** The Sessions row id display matches `(Some(sid), _)` for both kinds: a Codex id, when one exists, is shown truncated through `shorten_session_id` just like a Claude id. The `None` arms are exhaustive over `SessionKind`.
11. **[EXECUTOR]** The app.rs tracing message now reads "Auto-registered GSD project from active agent session", as the plan specified.
12. **Residual, not addressed:** an npm-wrapper Codex install whose native binary's comm is not exactly `codex` is invisible to `pgrep -x codex`. That is silent under-detection. The macOS `/proc` limitation is documented by sibling 260923-lra.

## Deviations from Plan

- **TDD order, Task 1:** the RED tests and the implementation were written in the same pass, so a separate RED run was not observed for Task 1. The tests assert the plan's behaviour rows exactly and pass. Task 2's RED was observed: a compile failure with 4 unresolved `resumable_session_id` references, 3 for `CODEX_RESUME_UNSUPPORTED` and 1 for `NO_SESSION_ID_TO_RESUME`.
- The Enter-on-Codex test destructures with `let … else panic!` because `ScreenAction` does not implement `Debug`.

Otherwise the plan was executed as written.

## Known Stubs

None.

## Threat Flags

None. The new /proc reads (cmdline, fd readlinks) are covered by T-lr9-02, T-lr9-03, T-lr9-05 and T-lr9-06.

## Self-Check: PASSED

- The files exist: src/session_detector.rs, src/ui/screens/detail.rs, src/ui/screens/normal.rs, src/ui/screens/help.rs.
- These commits are in `git log`: 2054f81, 5e49ba3, 45c2138.
