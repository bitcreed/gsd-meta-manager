---
last_mapped_commit: d7381ab615e5181d33f3b6a5e02cf4d2a42594af
last_mapped_at: 2026-11-03
---
<!-- refreshed: 2026-11-03 -->
# Codebase Concerns

**Analysis Date:** 2026-11-03

## Tech Debt

**Oversized UI screen module:**
- Issue: `src/ui/screens/detail.rs` is about 35,200 lines, by far the largest file in the crate. Rendering, input handling, escaping helpers and a very large inline test block all live in one file.
- Files: `src/ui/screens/detail.rs`, `src/app.rs` (about 9,100 lines), `src/ui/screens/driver_run.rs` (about 7,000 lines)
- Impact: Hard to review, slow to compile incrementally, and changes to one screen risk side effects in unrelated code paths. Contributors have to load a huge context to change anything.
- Fix approach: Split `detail.rs` into submodules by panel (for example `detail/render.rs`, `detail/input.rs`, `detail/escape.rs`) and move its tests next to the code they cover. Do this as a behaviour-preserving refactor with the existing test suite as the guard.

**Deferred review findings carried across milestones:**
- Issue: 123 open or deferred review entries (0 critical, 41 warning, 82 info) are recorded across Phases 24-31 and never closed.
- Files: `.planning/MILESTONES.md`, `.planning/STATE.md` (Review advisories section), `.planning/milestones/v2.0-phases/*/*-REVIEW-DISPOSITION.md`
- Impact: Known warnings accumulate without an owner, so real defects can hide inside the backlog.
- Fix approach: Triage the 41 warnings first. Fix the small ones, convert each remaining one into a todo under `.planning/todos/pending/`, and close the dispositions append-only.

**Executor code kept without a caller:**
- Issue: The Codex executor and in-process Claude transport were held pending a user decision (plan 31-07, D-04). Executor code remains in the tree with a narrow `dead_code` allow on `DrivableProject::with_runtime`.
- Files: `.planning/STATE.md` (Phase 31 entry), `src/executor/mod.rs`, `src/executor/stream_json.rs`
- Impact: Dead code that is not exercised by the binary can rot silently and still needs maintenance during refactors.
- Fix approach: Make the D-04 decision. Either delete the unused executor path or wire it up behind the `experimental` gate in `src/experimental.rs` with a test that exercises it.

**Test-side setup relies on sleeps and polling:**
- Issue: Many tests synchronise with `std::thread::sleep` polling loops (the `*_within` helpers) rather than explicit signals. The planning history records several flake fixes of this kind; the tests that needed them (`tests/driver_reattach.rs`, `tests/envelope_tracer.rs`, the `driver::run` tests) were deleted in Phase 31 (commits `2a23d9e` and `13ba5ac`), but the polling style remains.
- Files: `src/bounded_output.rs:111`, `src/launcher/run.rs:546`, `src/launcher/run.rs:593`, `src/launcher/stop.rs:143`, `src/launcher/stop.rs:168`, `src/launcher/process.rs:643`, `src/launcher/process.rs:688`
- Impact: Timing-dependent tests fail only under CPU contention, which makes CI results non-deterministic.
- Fix approach: Replace artifact polling with channel or condition-variable signals where the code under test can emit them.

**Large number of `unwrap`/`expect` calls:**
- Issue: About 3,370 `.unwrap()` or `.expect(` occurrences exist under `src/`, mostly in `#[cfg(test)]` modules. The one-off modules are clean outside tests: `src/oneoff/{recycle,land,session,runtime,stream}.rs` have no non-test unwrap/expect (test modules begin at `recycle.rs:1112`, `land.rs:1310`, `session.rs:638`, `runtime.rs:804`, `stream.rs:672`); `src/oneoff/launch.rs:95` has one `expect("static regex")`. Hits in `src/oneoff/mod.rs` and `merge.rs` are test-only.
- Files: `src/driverctl/serve.rs`, `src/driverctl_cli.rs`, `src/ui/screens/detail.rs`, `src/app.rs`
- Impact: A malformed file or unexpected state in a production path can panic the TUI or a driver process instead of returning a recoverable error.
- Fix approach: Run `cargo clippy` with `unwrap_used` and `expect_used` enabled for non-test targets, and convert production call sites to `?` with contextual errors.

**Large one-off lifecycle modules:**
- Issue: The one-off lifecycle code grew by about 7,000 lines since the last map. Non-test bulk sits beside inline test blocks in single files: `src/oneoff/launch.rs` (1,881 lines), `src/oneoff/store.rs` (1,657), `src/oneoff/land.rs` (1,649), `src/oneoff/recycle.rs` (1,585), `src/oneoff/runtime.rs` (1,351), `src/oneoff/resolve.rs` (1,236), `src/oneoff/session.rs` (1,157), `src/oneoff/stream.rs` (1,078). `tests/oneoff_e2e.rs` adds 1,087 lines.
- Impact: Landing, recycling, crash recovery and session control interact through run directories and lock files; changes need the whole context.
- Fix approach: Split `recycle.rs` (slimming at `slim_locked`, liveness at `probe_scope`, tending at `tend_run`/`recover_dead`) and `land.rs` into submodules when next touched.

**Unknown stream events are dropped silently:**
- Issue: `src/oneoff/stream.rs:295` has a `_ => {}` arm, and the event renderer yields no lines for unknown events (`src/oneoff/stream.rs:458`). A schema change in `claude` stream-json shows as missing output with no diagnostic.
- Files: `src/oneoff/stream.rs`, `tests/fixtures/oneoff/*.stream.jsonl`
- Fix approach: Count and surface unknown event types in the run report.

## Known Bugs

**Live-session detection is Linux only:**
- Symptoms: On macOS no running sessions are detected. Windows builds compile but are not exercised at runtime.
- Files: `src/session_detector.rs`, `src/driver/liveness.rs`
- Trigger: Run the TUI on macOS or Windows and expect running sessions to appear.
- Workaround: None. README states the feature is Linux only (`README.md` lines 35 and 414-420).

**Environment-pinned test failure:**
- Symptoms: One git-version-constants test fails on any machine whose installed git differs from the version the constants were derived against (git 2.55.0 is the expected version, see `scripts/pre-tag-check.Dockerfile:22`). Not re-verified in this refresh.
- Files: `scripts/pre-tag-check.sh` (gate 4), `scripts/pre-tag-check.Dockerfile`
- Trigger: Run `cargo test` on a machine with a different git version.
- Workaround: Accept the single known failure locally, or re-derive the constants on the target git version.

## Security Considerations

**Shell execution of user-configured hooks:**
- Risk: Project hook commands are passed to `sh -c` with the project name, path and alias exported as environment variables. Any value in the hook string is interpreted by the shell, and the hook runs with the user's full privileges.
- Files: `src/project_creator.rs` (`execute_hook`, around line 69), `src/terminal_switch.rs` (around line 212)
- Current mitigation: Hook stdout and stderr are discarded. The hook string comes from the user's own config, not from remote input.
- Recommendations: Document that hook strings are trusted shell code. Do not interpolate `name`, `path` or `alias` into the hook string; keep them only in environment variables, which the code already does. Add a test that a project name containing shell metacharacters is not executed.

**Process termination by recorded process group:**
- Risk: Kill and cancel paths signal a process group id read back from `run.json`. If the recorded pgid is stale and the id has been reused, the signal can hit an unrelated process group.
- Files: `src/driver/kill.rs` (pgid validation, refusing zero and out-of-range values), `src/oneoff/cancel.rs` (`signal_scope`, around line 388), `src/driver/liveness.rs` (reads `/proc/<pid>/cmdline` to decide liveness)
- Current mitigation: pgid zero and values that do not fit `pid_t` are refused. The liveness check reads `/proc`.
- Recommendations: Before signalling a recorded pgid, confirm the process group still belongs to the run (for example by matching the process start time or the command line recorded at launch), not only that the pid exists.

**Bridge listens on a caller-supplied loopback port:**
- Risk: The in-sandbox bridge binds `127.0.0.1:<port>` from its command line. Any local process can connect to that port while the bridge is up.
- Files: `src/launcher/bridge.rs` (line 64)
- Current mitigation: The bridge only relays bytes. Policy and run tokens are enforced by the launcher-side egress server, so bypassing the bridge gains nothing (fact 3 in the module header).
- Recommendations: Confirm that the egress server rejects connections without a valid run token even when they arrive through the bridge, and add a test for that path.

**Sensitive file permissions depend on per-call `mode` settings:**
- Risk: Files holding run state, sentinels and steering data are created with explicit modes (`0o600` for most, `0o644` for `src/driver/lock.rs:342` and `src/driverctl/log.rs:97`). A new write path that forgets the mode would create world-readable files.
- Files: `src/driver/direction.rs` (lines 728-731, 1537), `src/driver/lock.rs` (342, 560), `src/driver/steer.rs` (674, 1885), `src/driverctl/log.rs` (97)
- Current mitigation: Tests assert `0o600` on sentinels and steering files (`src/driver/steer.rs` around lines 1995-1997).
- Recommendations: Centralise file creation for run-state files in one helper that always applies `0o600`, and review whether `0o644` at `src/driver/lock.rs:342` and `src/driverctl/log.rs:97` must be world-readable.

**Two `unsafe` blocks in the tree are text-only:**
- Risk: The `unsafe` matches under `src/` are string literals and comments, not code. No real `unsafe` block was found, so the crate's safety depends on its dependencies.
- Files: `src/ui/glyphs.rs` (line 205), `src/text.rs` (line 630)
- Current mitigation: Not applicable.
- Recommendations: Add `#![forbid(unsafe_code)]` to `src/lib.rs` to keep it that way.

## Performance Bottlenecks

**Full-document rendering in the detail screen:**
- Problem: The detail screen rebuilds large rendered line sets from planning markdown on each change.
- Files: `src/ui/screens/detail.rs`, `src/archive.rs` (`render_markdown_lines`, `read_archive_file`), `src/state_reader/roadmap_md.rs` (about 4,160 lines)
- Cause: Rendering is done on the render path rather than cached per file version. `read_archive_file` reads whole archive files.
- Improvement path: Cache parsed roadmap and archive output keyed by file mtime and size, and re-render only when the source changes.

**Polling loops with short sleeps in the launcher:**
- Problem: Child-process supervision polls with `try_wait` and short sleeps, for example 2 ms and 5 ms in `src/launcher/process.rs` and 50 ms in `src/launcher/stop.rs`.
- Files: `src/launcher/process.rs:643`, `src/launcher/process.rs:688`, `src/launcher/run.rs:546`, `src/launcher/run.rs:593`, `src/launcher/stop.rs:143`
- Cause: No blocking wait on the child is used, so each supervised process costs a wakeup every few milliseconds.
- Improvement path: Use `wait` on a dedicated thread, or a pidfd where available, to avoid periodic wakeups.

**Fork-based process spawns for every git query:**
- Problem: Git state is read by spawning `git` subprocesses for each query.
- Files: `src/state_reader/git_ops.rs` (about 2,060 lines), `src/oneoff/merge.rs`
- Cause: Each status or diff call starts a new process.
- Improvement path: Batch queries into a single `git status --porcelain=v2` or `git for-each-ref` call per refresh, and cache results for the length of one refresh cycle.

## Fragile Areas

**Process group and pid handling:**
- Files: `src/driver/kill.rs` (`signal_group`), `src/launcher/run.rs` (`stop_group`), `src/launcher/process.rs` (`process_group(0)` around line 213), `src/driver/lock.rs` (`LockHolder::pgid`)
- Why fragile: There are two process groups, the driver's and the agent's own, so a signal aimed at one does not reach the other (`src/driver/kill.rs` module doc). The lock records `pgid == pid` on the assumption that the driver became a group leader at run entry. A refusal to signal group zero lives only in `signal_group`.
- Safe modification: Route every group signal through `signal_group`. Keep the `process_group(0)` spawn settings and the deliberate exceptions documented in `src/launcher/process.rs` (the bridge must not be a group leader) in sync with `stop_group`'s teardown grace.
- Test coverage: The earlier `driver::run` process-group tests and their race retry helper were removed with the drive subcommand in Phase 31. Coverage now comes from the launcher stop tests (`src/launcher/stop.rs`, `tests/launcher_*.rs`). [inferred — audit]

**Socket and bind path handling:**
- Files: `src/driverctl/serve.rs` (`bind_private`, `replace_and_bind`, lines 521-580)
- Why fragile: Socket binding stages a file, changes its mode and renames it. The code avoids `umask` swaps because `umask` is per-process and races with other threads. Path-length limits for Unix sockets also apply.
- Safe modification: Keep the staging-then-rename sequence. Do not replace it with a `umask` change.
- Test coverage: `tests/driverctl_serve.rs` and `tests/driverctl_race.rs` cover the main cases.

**One-off land lock and recycle liveness:**
- Files: `src/oneoff/land.rs` (`land_lock_path:95`, `take_land_lock:134`), `src/oneoff/recycle.rs` (`is_dead:517`, `lock_is_free:684`, `already_landed_candidate:763`, `recover_dead:910`)
- Why fragile: Recycling decides a run is dead from process liveness, scope state and whether the land lock is free. A wrong verdict removes a run directory (`remove_run_dir:663`) that a concurrent landing still uses. Earlier defects (leftover `land.lock` files, runs whose head was already in the target) were fixed in quick-261103-pd7; the lock now unlinks on release (`land.rs:107`).
- Safe modification: Take the land lock before any destructive step and keep `is_dead` the single liveness predicate.
- Test coverage: Unit tests in both files and `tests/oneoff_e2e.rs`.

**Cancel and stop request state machine:**
- Files: `src/oneoff/cancel.rs` (`tick`, `outranks`, `merge_request`, `read_request`, `write_request`)
- Why fragile: Stop, kill and pause requests are persisted to disk and merged by rank. A request written during an escalation can be lost or downgraded if the ordering rules change.
- Safe modification: Keep `outranks` as the single source of truth for precedence and add a test for any new `StopKind`.
- Test coverage: Unit tests in `src/oneoff/cancel.rs` cover precedence and round-tripping.

**Configuration writes:**
- Files: `src/state_reader/config_write.rs`, `src/state_reader/config_json.rs`, `src/ui/screens/mod.rs` (`U+XXXX` handling near line 876)
- Why fragile: Writes to the user's `config.json` must preserve bytes that were not changed and avoid writing escaped spellings such as `U+XXXX` into values. Mistakes corrupt the user's configuration.
- Safe modification: Use the atomic temp-file-and-rename helper in `config_write.rs`. Add a round-trip test for any new field.
- Test coverage: Covered by unit tests in `config_write.rs` and `detail.rs`.

## Scaling Limits

**One-off run and todo limits:**
- Current capacity: `MAX_TODOS` is 200, and each todo is read for its first 8 KiB only (`src/oneoff/todos.rs`, line 26 and line 104).
- Limit: Todo items beyond 200 are ignored, and content beyond 8 KiB is invisible to the listing.
- Scaling path: Make the cap configurable and read the title from the first line only, so the limit does not affect valid long backlogs.

**Roadmap and state parsing:**
- Current capacity: The roadmap and state parsers read the full `ROADMAP.md` and `STATE.md` (`.planning/STATE.md` is about 121 KB, `.planning/STATE-ARCHIVE.md` about 67 KB) on each refresh.
- Limit: Parse time and memory grow linearly with planning history. STATE.md is already large because each quick task adds a long table row.
- Scaling path: Move quick-task history out of STATE.md into per-task files, and keep STATE.md as a short index.

## Dependencies at Risk

**Dependency on external `gsd-core` and `claude` CLIs:**
- Risk: The launcher and driver shell out to `claude` and the gsd-core tooling. Output format changes in those tools (for example the stream-json schema in `src/executor/stream_json.rs` and `tests/fixtures/transcripts/`) break parsing.
- Impact: Runs can stall or misreport state after an upstream upgrade.
- Migration plan: Pin the expected schema version in `docs/GSD-CORE-SYNC.md` and fail with a clear error on unknown event types instead of silently ignoring them.

**`systemctl --user` for run scopes:**
- Risk: Cancel and kill of one-off runs call `systemctl --user kill` (`src/oneoff/cancel.rs`, `signal_scope`). This is unavailable on systems without a user systemd instance.
- Impact: Stop requests on such systems cannot signal the scope by name.
- Migration plan: Fall back to pgid signalling with the validation described under Security Considerations when `systemctl` is absent.

## Missing Critical Features

**No macOS or Windows runtime support for session detection:**
- Problem: Running-session detection and the sandbox launcher are Linux only (`README.md` lines 35 and 414-420; `src/launcher/egress/upstream.rs` TLS path is Linux only).
- Blocks: Cross-platform use of the launcher, and reliable running-session status on macOS and Windows.

## Test Coverage Gaps

**Real Claude process behaviour:**
- What's not tested: Recorded real streams now exist in `tests/fixtures/oneoff/` (`real-bg-subagents.stream.jsonl`, `real-resumed.stream.jsonl`), but live `claude` runs are gated behind `tests/launcher_live_*.rs`, which need credentials and are not part of the default suite.
- Files: `tests/launcher_live_egress.rs`, `tests/launcher_live_isolation.rs`, `tests/launcher_live_managed.rs`, `tests/launcher_live_scope.rs`, `tests/launcher_live_token_reach.rs`
- Risk: Changes to the real CLI behaviour (stream-json, rate limits, interrupts) can pass the suite and fail in use. Fixtures in `tests/fixtures/transcripts/` and `tests/fixtures/oneoff/` may drift from the real format.
- Priority: High

**Windows and macOS code paths:**
- What's not tested: Windows builds are compile-checked only, and macOS session detection has no tests.
- Files: `src/session_detector.rs`, `src/driver/liveness.rs`
- Risk: Platform-specific regressions reach users unnoticed.
- Priority: Medium

**Terminal rendering edge cases:**
- What's not tested: Escaping of invisible and control characters is tested with hand-written cases in `src/ui/screens/render_escape_guard.rs` and `src/text.rs`, but not with broad generated input.
- Files: `src/text.rs`, `src/ui/screens/render_escape_guard.rs`, `src/state_reader/driver_frontmatter.rs`
- Risk: A character class that is missed in the escape table can corrupt the terminal or inject control sequences into the display.
- Priority: Medium

---

*Concerns audit: 2026-11-03*
