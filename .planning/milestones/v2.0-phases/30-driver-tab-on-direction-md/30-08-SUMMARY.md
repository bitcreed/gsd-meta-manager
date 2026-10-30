---
phase: 30-driver-tab-on-direction-md
plan: 08
subsystem: editor-handoff
tags: [rust, editor, process-spawn, test-hygiene, gap-closure]
status: complete
gap_closure: true

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-05 Driver tab; D-17 made the $EDITOR handoff the only DIRECTION.md edit path
provides:
  - "app::editor_command: the (program, argv) that opens a path in the user's $VISUAL/$EDITOR string; an editor with arguments runs as sh -c '<editor> \"$@\"' '<editor>' <args...>, the path only ever a separate argv element"
  - "tests/editor_command.rs: real sh runs a stub editor; a hostile run-dir path stays one literal argument"
  - "tests/launcher_config_dir.rs: EXEC_LOCK + run_shim, no more Text file busy flake"
  - "next_matching gated to Linux with its callers"
affects: [30-09]

actuals:
  tokens: 5400
  tasks: 2
  commits: 3
plan_head_before: 2ccd8617841cf3fe5da617a81c961cd3680b19c3
plan_head_after: 8d6a763318da9fa4e21c4e73b12829044cb84718
commits: 3

tech-stack:
  added: []
  patterns:
    - "spawn-free argv builder in the library; the binary builds the one Command"
    - "lock across write -> spawn -> wait for freshly written test scripts, plus bounded ETXTBSY retry"

key-files:
  created: [tests/editor_command.rs]
  modified: [src/app.rs, src/main.rs, tests/launcher_config_dir.rs]

key-decisions:
  - "WR-06: the sh -c '\"$EDITOR\" \"$@\"' form supersedes RESEARCH Item 1's no-split spawn [inferred - audit]"
  - "An editor string that names one existing file (Path::is_file) spawns directly, exempt from the sh route [inferred - audit]"
  - "The 200-run loop is bounded by timeout 480 [inferred - audit]"

requirements-completed: [DTAB-03]

coverage:
  - id: D1
    description: "A $VISUAL/$EDITOR value with arguments (code -w, emacs -nw, nvim -u NONE) opens the edit temp; quoted paths with spaces work; an existing editor file with a space and a bare program keep the direct spawn"
    requirement: DTAB-03
    verification:
      - kind: integration
        ref: "tests/editor_command.rs#an_editor_with_arguments_receives_them_and_the_path"
        status: pass
      - kind: integration
        ref: "tests/editor_command.rs#an_editor_path_with_a_space_is_quoted_and_runs"
        status: pass
      - kind: integration
        ref: "tests/editor_command.rs#an_existing_editor_file_with_a_space_runs_directly"
        status: pass
      - kind: integration
        ref: "tests/editor_command.rs#a_bare_editor_program_runs_directly"
        status: pass
      - kind: unit
        ref: "src/app.rs#editor_command_wraps_an_editor_with_arguments_in_sh"
        status: pass
      - kind: unit
        ref: "src/app.rs#editor_command_positions_the_line_for_the_first_word"
        status: pass
    human_judgment: false
  - id: D2
    description: "A run-directory path holding shell syntax reaches the editor as one literal argument and runs nothing (T-30-27)"
    verification:
      - kind: integration
        ref: "tests/editor_command.rs#a_hostile_path_reaches_the_editor_as_one_literal_argument"
        status: pass
      - kind: unit
        ref: "src/app.rs#editor_command_never_puts_the_path_in_the_shell_script"
        status: pass
    human_judgment: false
  - id: D3
    description: "tests/launcher_config_dir.rs no longer flakes with Text file busy (200/200 runs; 0/500 on the bare binary)"
    verification:
      - kind: other
        ref: "timeout 480 loop of cargo test --test launcher_config_dir --test editor_command -- --test-threads=16 x200"
        status: pass
    human_judgment: false
  - id: D4
    description: "next_matching is compiled only on Linux, where its callers are (macOS clippy not run: blocked by the deferred sandbox.rs/scan.rs breaks)"
    verification:
      - kind: other
        ref: "grep -B3 'async fn next_matching' src/app.rs shows #[cfg(target_os = \"linux\")]; cargo clippy --all-targets -- -D warnings"
        status: pass
    human_judgment: true
    rationale: "The macOS dead-code claim cannot be confirmed here; the macOS clippy run is blocked by separately owned build breaks"

duration: 35min
completed: 2026-10-21
---

# Phase 30 Plan 08: Editor with arguments and the ETXTBSY flake Summary

**`e` now works with `$EDITOR=code -w` / `emacs -nw` via git's `sh -c '<editor> "$@"'` rule with the run-dir path only ever a separate argv element, and the `launcher_config_dir` shim tests no longer flake with `Text file busy` (27/200 before, 200/200 after).**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-10-21T04:47Z
- **Tasks:** 2 (Task 1 a tracer, TDD)
- **Files modified:** 4 (1 created)

## Accomplishments

- WR-06 fixed with no new dependency. `app::editor_command(editor, path, line)` returns the program and argv; the library builds no `Command` (`grep -c 'Command::new' src/app.rs` is 0, baseline 0). `src/main.rs` builds its one process (still the single `.status()` site) from that pair; the `$VISUAL` > `$EDITOR` > `vi` order, status messages and `EditorExit` mapping are unchanged.
- Unix routing: a string that is one existing file (`Path::is_file`) or holds no git shell-special character spawns directly as before; otherwise `sh -c '<editor> "$@"' '<editor>' <positioned args>`. Positioning is keyed on the first word (quotes stripped), so `nvim -u NONE` still gets `+N` and `code -w` gets `-g path:N`. Empty string is `vi`. Non-unix keeps the shipped pair.
- T-30-27 mitigated and tested twice: the unit test asserts the `-c` script is exactly `{editor} "$@"` with no path byte; the real-sh test runs four hostile names (`$(touch PWNED);x.md`, backtick, single- and double-quote breakouts) and asserts each is recorded verbatim and nothing is created.
- ETXTBSY: `EXEC_LOCK` holds write -> spawn -> wait across both shim tests, `run_shim` retries a residual `ETXTBSY` (spawn error or exit 126 with `Text file busy`) up to 5 times, 20 ms apart.
- `next_matching` gated `#[cfg(target_os = "linux")]`.

## Task Commits

1. **Task 1 RED: failing tests (stub keeps the shipped spawn)** - `11b0eea` (test)
2. **Task 1 GREEN: editor_command + main.rs** - `168d03e` (feat)
3. **Task 2: serialised shim spawns, ETXTBSY retry, next_matching gate** - `8d6a763` (fix)

**Plan metadata:** the docs(30-08) commit that carries this file.

## TDD record (Task 1)

- **RED:** `editor_command` existed as a stub returning the shipped pair, so the targets compiled. `cargo test --test editor_command`: 3 failed / 2 passed; the three failures are exactly the cases the bug breaks (`an_editor_with_arguments_receives_them_and_the_path`, `an_editor_path_with_a_space_is_quoted_and_runs`, `a_hostile_path_reaches_the_editor_as_one_literal_argument`), each on the `status.is_ok_and(success)` assertion with `Os { code: 2, NotFound }` (the whole `code -w` string tried as one program name). `app::tests::editor_` : 4 new tests failed on their planned assertions (`("code -w", [path])` vs `("sh", ["-c", ...])`, empty string vs `vi`, index out of range because no `sh` wrapper existed). The two passing integration cases (`a_bare_editor_program_runs_directly`, `an_existing_editor_file_with_a_space_runs_directly`) are the unchanged-behaviour pins and are expected green before and after. Semantic assessment: the targets executed and failed on the intended behaviour, not on compile, fixture or unrelated faults. The cargo test report is not one of the formats `gsd check tdd-red-evidence` parses and the plan is `type: execute`, so the classifier was not run `[inferred - audit]`.
- **GREEN:** all 5 integration and 7 `app::tests::editor_*` tests pass; `spawn_seam_guard` (41) and `async_blocking_guard` (9) pass; clippy clean.
- **REFACTOR:** none (rustfmt only, folded into GREEN).

## Pre-change loop result (Task 2 RED evidence)

On the unmodified tree, `for i in 1..200: cargo test --no-fail-fast --test launcher_config_dir -- --test-threads=16` failed **27 of 200** runs (35 s wall), and a captured failure was `Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }` at the `.output().unwrap()` of `the_next_shim_refuses_any_other_verb_without_running_the_binary`. After the change: the plan's 200-run loop over `--test launcher_config_dir --test editor_command` printed `200/200 runs passed` in 38 s (well inside the 480 s bound), and 500 runs of the bare test executable at `--test-threads=32` had 0 failures.

## Files Created/Modified

- `src/app.rs` - `editor_command`, `is_shell_special`, `first_word`; four unit tests; `next_matching` cfg gate
- `src/main.rs` - the editor handoff builds its Command from `editor_command`'s pair
- `tests/editor_command.rs` - new; stub-editor tests under `EXEC_LOCK`
- `tests/launcher_config_dir.rs` - `EXEC_LOCK`, `run_shim`

## Decisions Made

- `[inferred - audit]` The `sh -c '"$EDITOR" "$@"'` form supersedes RESEARCH Item 1's recommendation to keep the no-split spawn; it needs no dependency and keeps quoted paths with spaces working.
- `[inferred - audit]` Existing-file exemption: an unquoted value that names one existing file spawns directly, so a previously-working `/Applications/Sublime Text.app/.../subl` keeps working.
- `[inferred - audit]` A leading `~` now goes through `sh` and expands (it never spawned before, since exec does no tilde expansion).
- `[inferred - audit]` The 200-run loop is bounded by `timeout 480`; it finished in 38 s so the bound was not approached.
- `[inferred - audit]` The surrounding-quote stripping for the first word handles a leading `"` or `'` only; backslash-escaped spaces in the first word (`/a\ b/code -w`) key on the text before the first space and so skip line positioning (the editor still opens the file).

## Deviations from Plan

None - plan executed exactly as written. (Task 1's RED needed a stub `editor_command` in the RED commit so the tests compile and fail on assertions rather than on a missing symbol; this is the planned tracer shape, not a deviation.)

## Issues Encountered

- None blocking. `cargo fmt` reflowed two lines of my own new unit test before the GREEN commit; no other file changed.
- macOS confirmation of the `next_matching` gate (`cargo clippy --target x86_64-apple-darwin --all-targets -- -D warnings`) was not run: it is blocked by the deferred `src/launcher/sandbox.rs` / `src/driverctl/scan.rs` breaks, which another task owns and which this plan did not touch.

## Verification

- Full suite `cargo test --no-fail-fast`: **4057 passed, 1 failed, 24 ignored** (baseline after 30-07: 4048 passed; +4 unit +5 integration). The single failure is the expected environment-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The `launcher_new_run_the_footer...` flake did not trip.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `Command::new` count in `src/app.rs`: 0 (baseline 0).

## Known Stubs

None.

## Threat Flags

None. The new `sh` route is the planned T-30-27 surface and is covered by its mitigation tests; T-30-28 (the user's own `$EDITOR` interpreted by `sh`) is accepted as planned.

## Next Phase Readiness

Ready for 30-09. The `cargo test --no-fail-fast` gate no longer fails intermittently on `Text file busy`.

## Self-Check: PASSED

- `tests/editor_command.rs`, `src/app.rs`, `src/main.rs`, `tests/launcher_config_dir.rs` exist; commits `11b0eea`, `168d03e`, `8d6a763` are ancestors of HEAD.
- Acceptance greps: `pub fn editor_command` x1, `editor_command(` in main.rs x1, `EXEC_LOCK` x3, `fn run_shim` x1, `#[cfg(target_os = "linux")]` above `async fn next_matching`.
