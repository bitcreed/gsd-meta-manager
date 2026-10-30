---
phase: 29-master-prompt-run-templates
plan: 06
subsystem: launcher
tags: [launcher, embedded-prompt, prompt-version, direction-lint, preflight, e2e]

requires:
  - phase: 29-master-prompt-run-templates
    provides: "29-01 launcher::prompt (embedded/custom/ResolvedPrompt), 29-02 LaunchSpec.prompt_version, 29-04 driver_templates::lint::lint_direction, 29-05 launcher_reconcile suite"
  - phase: 28-sandboxed-driver-launcher
    provides: "driver launch, launch_detached, tests/launcher_support Fixture"
provides:
  - "driver launch <project> <run-id> with no --prompt-file runs the embedded driver-prompt/1.0.0 and records it per launch (LOG prompt= and frontmatter prompt_version)"
  - "--prompt-file is an optional hidden override recorded as custom:<sha8>, also when its bytes equal the embedded asset"
  - "LauncherError::DirectionInvalid + preflight_direction: lint and control=run enforced before any write, lock, token or prompt resolution, in both launch paths"
  - "lint::control_line: the file line and word of the Direction control"
  - "Fixture::launch_command_embedded and a lint-clean fixture BODY"
affects: [29-07 harness (launches by run id alone), 29-08 paid runs]

actuals:
  tokens: 9200
  tasks: 3
  commits: 4

plan_head_before: 8793ba3d059b8c9d955c4ba59ed4dd7d8bba2e33
plan_head_after: acfd1402ecbf2d1cf6f857ca4b9b3373460f5f4e
commits: 4

tech-stack:
  added: []
  patterns:
    - "prompt source decides the recorded version (embedded -> driver-prompt/X.Y.Z, file -> custom:<sha8>), never the content"
    - "preflight refusals sit ahead of the lock, token and prompt so a refused launch leaves no trace, asserted by one assert_nothing_was_written helper"
    - "DirectionInvalid prints bare file:line: message lines (driver_cli::report skips the verb prefix for it)"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/driver_cli.rs
    - src/driver_templates/lint.rs
    - src/launcher/mod.rs
    - src/launcher/run.rs
    - tests/launcher_e2e.rs
    - tests/launcher_support/mod.rs

key-decisions:
  - "A paused or stopped DIRECTION control is refused at launch with `control is <word>; set - control: run to launch` [inferred - audit] (RESEARCH OQ2)."
  - "DirectionInvalid is printed without the `driver launch:` prefix so every stderr line has the same DIRECTION.md:<line>: shape (clickable in editors)."
  - "Lint warnings are not printed at launch; `driver check` is where they are shown. Launch only refuses on problems."
  - "The two-launch version-overwrite test relaunches after a SIGKILL crash, the existing reconcile route, rather than after a normal stop."

requirements-completed: [PRMT-01, PRMT-02, DRIVE-03]

duration: 30min
completed: 2026-10-20
status: complete
---

# Phase 29 Plan 06: Launcher Wiring Summary

**`driver launch <project> <run-id>` now runs the embedded `driver-prompt/1.0.0` by default, records the version it actually used on every launch (`custom:<sha8>` for the hidden `--prompt-file` override), and refuses a failing, paused or stopped DIRECTION.md in both launch paths before any lock, token, write or `claude` call.**

## Performance

- **Duration:** about 30 min
- **Completed:** 2026-10-20
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 7

## Accomplishments

- **Tracer (D-04, D-10):** `LaunchArgs.prompt_file` and `LaunchOptions.prompt_file` are `Option<PathBuf>`, the flag is hidden. `resolve_prompt` returns `prompt::embedded()` or `prompt::custom(text)` plus a source label; `check_prompt` takes that label. The version rides in `LaunchContext.prompt_version` into the `LaunchSpec` of every launch. `launch_detached` canonicalizes and reads a given file in the parent and passes the canonical path to the daemon; with none, both sides resolve the embedded asset. `launch_argv` pushes `--prompt-file` only when given. E2E `a_launch_without_a_prompt_file_runs_the_embedded_versioned_prompt` checks the recorded prompt starts `<!-- driver-prompt/1.0.0`, carries the rendered run dir and project root, the `RUN | launch | n=1` line ends `prompt=driver-prompt/1.0.0`, and the frontmatter reads the same.
- **Override semantics (T-29-20):** e2e tests pin `custom:<hash of PROMPT_TEMPLATE>`, a file holding the exact `MASTER_PROMPT` bytes still `custom:` (verified to bite by mutation: making `custom()` return the embedded version for equal bytes fails that test), crash-then-relaunch without a file leaving frontmatter `driver-prompt/1.0.0` while LOG holds both `prompt=custom:` (n=1) and `prompt=driver-prompt/1.0.0` (n=2), empty and whitespace-only files refused before a `launch` line, and a detached launch without a file. CLI tests: `a_hidden_prompt_override_is_not_in_launch_help` (with a positive control) and `the_default_gap_cap_is_two` (D-14).
- **Preflight (D-11, DRIVE-03, T-29-21):** `preflight_direction(run_dir)` runs `lint_direction`, then requires `control: run`, and is called right after `validate_run_dir` in `launch()` and `launch_detached()`. Refusals exit 1 with one `DIRECTION.md:<line>: <message>` line per problem and leave no lock record, no `state.json`, no LOG.md change, no per-run state directory (so no `launcher.log`) and an empty fake-claude rec dir. A lint warning (`gap-rounds=3`) still launches.
- **Phase 28 regression:** all ten launcher suites and `spawn_seam_guard`, `driverctl_serve`, `async_blocking_guard` pass with the lint-clean fixture body.

## Task Commits

1. **Task 1 (tracer):** `f916a71` feat - launch the embedded versioned driver prompt by default
2. **Task 2:** `ab2b33d` test - custom:<sha8> per launch, hidden override, gap cap of two
3. **Task 3 RED:** `0951db5` test - failing launch-preflight tests and lint-clean fixture body
4. **Task 3 GREEN:** `acfd140` feat - refuse a failing or paused DIRECTION.md before any launch spends anything

## TDD Gate Compliance

- **Task 3:** RED `0951db5` (three refusal tests failed on the planned exit-code assertion `left: Some(0) right: Some(1)`: the launch succeeded), GREEN `acfd140`. No REFACTOR commit.
- **Task 2:** the behaviours were already delivered by the Task 1 tracer, so the tests passed on first run (unexpected GREEN, investigated: expected, because the tracer built the whole resolve/record path). They are characterization tests; the key one was mutation-checked (above). Recorded here rather than as a RED/GREEN pair.

## Files Created/Modified

- `src/cli.rs` - hidden optional `--prompt-file`, help and gap-cap tests
- `src/driver_cli.rs` - `report` prints `DirectionInvalid` as bare lines
- `src/driver_templates/lint.rs` - `control_line` and its three unit tests
- `src/launcher/mod.rs` - `LauncherError::DirectionInvalid` (exit 1, one line per problem)
- `src/launcher/run.rs` - `resolve_prompt`, `check_prompt(source)`, `preflight_direction`, `prompt_version` threading, optional argv flag, detached prompt handling
- `tests/launcher_e2e.rs` - 10 new e2e tests (embedded default, detached embedded, custom sha, byte-equal custom, version overwrite, empty files, lint refusal, detached refusal, pause/stop refusal, warning launches)
- `tests/launcher_support/mod.rs` - `launch_command_embedded`, lint-clean `BODY`

## Phase 28 fixture bodies changed for the lint

- `tests/launcher_support/mod.rs` `BODY` (the only body the Phase 28 suites write): each item now has `done-when`, `budget: dispatches=3`, `after: none`, `hold: false` (I-02 gained `budget` as well). Item ids and `- control: run` are unchanged. `launcher_reconcile.rs` already wrote a lint-clean `RECONCILE_BODY`. The lint was not weakened.

## Decisions Made

See `key-decisions`. The D-18 pause reading is not touched by this plan; the OQ2 launch-refusal of `pause`/`stop` is tagged `[inferred - audit]`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The preflight needs the control line number, which the lint does not expose**
- **Found during:** Task 3
- **Issue:** `lint_direction` returns findings but not where `control` sits, and the OQ2 message must name that line. `blank_comments` is private to `lint.rs`.
- **Fix:** added `pub fn control_line(text)` to `src/driver_templates/lint.rs` (same comment-blanking and frontmatter-offset rules as the lint) with unit tests; `src/driver_templates/lint.rs` is outside the plan's `files_modified`.
- **Commit:** `acfd140`

### Process notes

- The plan's `driver_cli.rs` edit is the `report` special case so stderr is exactly the problem lines (no `driver launch:` prefix on the first one).
- Task 3's refusal tests are red-then-green as above; Task 2's are not (see TDD Gate Compliance).

**Total deviations:** 1 auto-fixed (1 blocking). **Impact:** none on scope.

## Verification

- `cargo test --no-fail-fast`: 3662 passed, 1 failed, 22 ignored; the single failure is the expected local `envelope::policy` git-version witness.
- Launcher suites (launcher_e2e 29, caps 17, sandbox 5+1 ignored, stop 5, degraded 16, auth 11, push 10, config_dir 11, acceptance 7, reconcile 7), `spawn_seam_guard` 41, `driverctl_serve` 12, `async_blocking_guard` 9: all pass.
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- Acceptance greps: `pub prompt_file: Option<PathBuf>` 1, `prompt_version: Some` 1, `prompt::(embedded|custom)` 2, `custom:` in launcher_e2e 2, `fn the_default_gap_cap_is_two` 1, `DirectionInvalid` in launcher/mod.rs 2 (plus test uses), `preflight_direction` in run.rs 3, `^- done-when:` in launcher_support 2.
- No paid or live model run was launched (29-08 owns those).

## Known Stubs

None.

## Threat Flags

None. T-29-20 (custom: for every file, byte-equal included), T-29-21 (preflight ahead of every write in both paths, e2e asserts nothing written and no claude call) and T-29-22 (parent canonicalizes and reads, the daemon re-reads the same canonical path; the embedded default has no file) are mitigated as planned.

## Self-Check: PASSED

Commits `f916a71`, `ab2b33d`, `0951db5`, `acfd140` are ancestors of HEAD; all seven modified files exist.
