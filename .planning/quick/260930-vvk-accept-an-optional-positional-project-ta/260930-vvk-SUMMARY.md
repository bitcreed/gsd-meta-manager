---
quick_id: 260930-vvk
phase: quick-260930-vvk
plan: 01
subsystem: cli
status: complete
tags: [cli, clap, launch-target, tui-navigation]
requires: []
provides:
  - "positional [TARGET] on the top-level CLI (alias or path)"
  - "launch_target::resolve_launch_target + LaunchTargetError"
  - "App::open_project_view (drives the overview's Enter handler)"
  - "Cli::try_parse_checked_from / Cli::parse_checked"
affects: [src/main.rs startup ordering]
tech-stack:
  added: []
  patterns:
    - "post-parse clap conflict check instead of args_conflicts_with_subcommands"
    - "launch-time navigation by replaying the existing key handler"
key-files:
  created:
    - src/launch_target.rs
    - tests/launch_target_cli.rs
  modified:
    - src/lib.rs
    - src/cli.rs
    - src/app.rs
    - src/main.rs
    - README.md
decisions:
  - "No args_conflicts_with_subcommands; post-parse ArgumentConflict check preserves intent"
  - "Blank target refused as NotFound"
  - "Same-root alias tie broken by lexicographically smallest alias"
  - "App::new moved before tui::init()"
  - "open_project_view replays Enter on the overview rather than pushing DetailScreen directly"
metrics:
  duration: "~25 min"
  completed: 2026-09-30
actuals:
  tokens: 8300
  tasks: 3
  commits: 3
plan_head_before: 87e18cf296ea5d6059cffb5d5512f86b7b8e7e3e
plan_head_after: 333d1636ebb385f1ebf7d561d0a470e5ef94975e
---

# Quick 260930-vvk: Positional project target Summary

You can now run `gsd-meta-manager <alias|.|path|subdir>` to start the TUI directly on that project's detail view. It opens through the overview's own Enter handler, so Esc behaves exactly like a drilled-in view. An unknown or unregistered target exits 1 before any terminal setup, with an `add` hint. Subcommands, including the TUI's `--config X drive …` respawn, keep priority. A target combined with a subcommand is refused as a clap usage error (exit 2).

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 (tracer) | 9840afe | feat(quick-260930-vvk): gsd-meta-manager <alias> opens the project view at launch |
| 2 | 304e3d8 | feat(quick-260930-vvk): path targets resolve to the nearest registered root |
| 3 | 333d163 | feat(quick-260930-vvk): subcommands keep priority; target+subcommand is a usage conflict |

## What was built

- `src/launch_target.rs`: `resolve_launch_target(config, target, cwd)` follows D-02.
  1. An exact byte-equal alias wins first.
  2. Otherwise the target is read as a path: `cwd.join(target).canonicalize()` is matched component-wise (`Path::starts_with`) against canonicalized registered roots, and the deepest one wins.
  3. A stale root falls back to its stored path.
  4. An existing path under no registered root gives `Unregistered`. A nonexistent path gives `NotFound`.
  - `LaunchTargetError::Display` escapes the target and path, and so does `suggestion()`. Both go through `text::render_for_terminal`.
- `src/cli.rs`:
  - New `target: Option<String>` positional. Its short help and long help (the collision note) are `///` comments; the rationale is in `//` comments.
  - `try_parse_checked_from` and `parse_checked` refuse a target combined with a subcommand, returning `ErrorKind::ArgumentConflict`.
- `src/app.rs`: `App::open_project_view(alias)` works only at launch (the stack must have exactly 1 screen). It selects the alias's row and then replays `KeyCode::Enter` through `handle_key`.
- `src/main.rs`:
  - `Cli::parse_checked()` replaces `Cli::parse()`, and the unused `clap::Parser` import is gone.
  - `App::new` and the target resolution now run BEFORE `tui::init()`.
  - `open_project_view` is called immediately before `spawn_crossterm_reader`. If it fails, it logs a warning and shows a status message rather than falling back silently.
- `tests/launch_target_cli.rs`: three binary-level checks.
  - An unregistered existing path is refused before the TUI starts.
  - An unknown alias is refused before the TUI starts.
  - With a project registered as `list`, `list` still runs the subcommand.
  - The refusal checks assert exit code exactly 1, an `add` hint on stderr, and no `\x1b[?1049h` on stdout.
- `README.md`: the `## Usage` help block is replaced with the real `--help` output, and an "Open a project directly" example set is added with the Esc, refusal and collision notes.

## Verification (D-07 gate)

- `cargo build`: clean.
- `cargo clippy -- -D warnings`: clean. `--all-targets` is clean too.
- `cargo test --no-fail-fast`: **2881 passed, 1 failed, 15 ignored** across 58 test-result lines.
  - The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. This is the known environmental git-version witness failure on this machine.
- New tests:
  - 18 `launch_target` lib tests: 15 resolver tests plus 3 app-state tests.
  - 7 `the_project_target_` cli tests.
  - 3 binary tests in `tests/launch_target_cli.rs`.
  - `spawn_seam_guard` (41) and `async_blocking_guard` (9) stay green.
- Tracer gate: the Task 1 binary check was re-run end-to-end before expanding. It exited 1, the stderr named the target and carried the `add` hint, and there was no alt-screen sequence.
- `grep -c "gsd-meta-manager \." README.md` = 3.

## Inferred decisions (for audit)

1. **D-01 mechanism deviation (accepted by the orchestrator).** We did NOT use `args_conflicts_with_subcommands = true`.
   - In clap_builder 4.6.7, any long flag, including the global `--config`, sets `valid_arg_found` (parser.rs:821). Once that is set, `possible_subcommand` stops matching (parser.rs:592). With the attribute, `--config X list` would parse as target "list", and `--config X add p` would be an error.
   - That would also silently break the TUI's own `current_exe() --config X drive …` respawn (`driver::spawn::drive_argv`), whose stdio is /dev/null.
   - The intent (a target and a subcommand never combine) is kept by a post-parse `ArgumentConflict` check in `Cli::try_parse_checked_from`.
   - Regression tests pin `--config X list` and the real `drive_argv` output. The rationale is recorded in `//` comments on `Cli::target` and on the `impl Cli`.
2. **A blank or whitespace-only target is refused as NotFound.** Otherwise `cwd.join("")` would silently open the project at cwd.
3. **Deterministic tie-break.** When two aliases are registered at the same canonical root, the lexicographically smallest alias wins. `projects` is a HashMap, so without this the choice would vary between runs.
4. **`App::new` moved before `tui::init()`.** This is required so a refusal exits before terminal setup. As a side effect, a config-load error no longer leaves the terminal in raw mode on the alt screen.
5. **The launch reuses the Enter handler.** `open_project_view` replays `KeyCode::Enter` through the overview rather than pushing `DetailScreen` itself, so scroll reset, redraw and future Enter behaviour stay identical. The Esc parity test checks this side by side against a j+Enter twin.
6. **README help block uses the long `--help` rendering.** The previous block used the short form. The long form is the one that carries the D-04 collision note.
7. **Non-UTF-8 target argv.** `target` is a `String`, so clap rejects non-UTF-8 argv with its standard usage error. This was left as is: registered aliases are UTF-8, and paths with non-UTF-8 bytes remain a known gap.
8. Commits landed directly on `master`, as the orchestrator instructed (isolation none). The repo's default branch is `dev`.

## Deviations from Plan

- **TDD order, Task 1:** the resolver tests and the alias-only implementation were written together, and they passed on the first run. There was no observed red step for Task 1. Task 2 had a proper red run (9 failing) before the implementation.
- Added two resolver tests beyond the plan's list: `a_stale_registered_root_does_not_block_resolving_another` (T-vvk-06) and an `Unregistered` escape check inside the Display escaping test (T-vvk-01).

Otherwise the plan was executed as written.

## Known Stubs

None.

## Threat Flags

None. All new surface is covered by T-vvk-01..06 in the plan's threat model.

## Self-Check: PASSED

- FOUND: src/launch_target.rs, tests/launch_target_cli.rs
- FOUND commits: 9840afe, 304e3d8, 333d163
