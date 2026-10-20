---
phase: 29-master-prompt-run-templates
plan: 03
subsystem: driver-router-hint
tags: [next-cli, router, read-only, sandbox-shim, clap, prmt-05]

requires:
  - phase: 20-deterministic-decision-router-run-bounds
    provides: "router::decide, router::is_goal_met, Decision / RouterReason, RULE_TABLE rationale consts"
  - phase: 28-sandboxed-driver-launcher
    provides: "launcher::config_dir::seed and the bin/driverctl shim this one sits beside; the ro-bound sandbox"
provides:
  - "D-20: `gsd-meta-manager next <project> [--target-phase N] --json` prints one advisory JSON object built only from typed router tokens"
  - "D-20 / OQ3: default target is the first current-milestone roadmap phase that is not goal-met; none left -> decision none, reason no-target, exit 0"
  - "D-21: dispatched before init_tracing, writes nothing; accepts an unregistered directory with .planning/ or a registered alias (read-only)"
  - "D-21: <config_dir>/bin/gsd-meta-manager 0755 shim that runs `next` and refuses every other first argument with exit 2"
affects: [29-01 master prompt cites this command (D-22), 29-07/29-08 acceptance runs]

actuals:
  tokens: 8200
  tasks: 3
  commits: 5

plan_head_before: f7c538c5ee876493207c191bb3a09a127210c1ad
plan_head_after: 0046571f6ca27a0f27bd1096b7537a32e8dd4223

tech-stack:
  added: []
  patterns:
    - "read-only CLI verb dispatched in main before init_tracing (same slot as Commands::Driverctl)"
    - "Decision -> JSON mapping with one arm per variant and no wildcard"
    - "bin shim as a `case \"$1\" in` allow-list, documented as convenience-not-boundary"

key-files:
  created:
    - src/next_cli.rs
    - tests/next_cli.rs
  modified:
    - src/cli.rs
    - src/main.rs
    - src/launcher/config_dir.rs
    - tests/launcher_config_dir.rs

key-decisions:
  - "D-20 / RESEARCH OQ3 [inferred - audit]: the default target uses router::is_goal_met, not phase_is_done, so a phase stuck at gaps_found is reported (router park) instead of skipped."
  - "`--json` is required [inferred - audit]; without it clap exits 2."
  - "Reason strings follow the plan: no_rule -> `no-rule`, goal_met -> `goal-met`, none -> `no-target` (hyphenated, distinct from the router's snake_case RouterReason tokens)."
  - "Path form wins over registry lookup: a directory containing .planning/ is used as-is (sandbox has no registry); otherwise an alias or a path inside a registered root resolves through launch_target."
  - "serde_json (no preserve_order) emits keys alphabetically; the object is compact one-line JSON and consumers must not rely on key order."

patterns-established:
  - "Refusals are one `next: ...` stderr line, exit 1, empty stdout; usage errors are clap's exit 2."

requirements-completed: [PRMT-05]

coverage:
  - id: D1
    description: "`next <project> --json` prints the router's decision as one advisory JSON object (run, park, goal_met, none shapes)"
    requirement: PRMT-05
    verification:
      - kind: e2e
        ref: "tests/next_cli.rs#a_discussed_phase_is_routed_to_planning_as_an_advisory_run"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#a_phase_with_gaps_found_parks_with_the_routers_own_reason"
        status: pass
    human_judgment: false
  - id: D2
    description: "Default target selection (first current-milestone phase not goal-met) and the no-target answer"
    requirement: PRMT-05
    verification:
      - kind: unit
        ref: "src/next_cli.rs#the_default_target_* (4 tests)"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#without_a_target_phase_the_first_unfinished_phase_is_routed"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#when_every_phase_passed_there_is_no_target_and_the_exit_is_zero"
        status: pass
    human_judgment: false
  - id: D3
    description: "Alias and path resolution, plus clean refusals (unknown alias, bad --target-phase, no .planning/, missing --json)"
    requirement: PRMT-05
    verification:
      - kind: e2e
        ref: "tests/next_cli.rs#a_registered_alias_resolves_through_the_global_config"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#an_unknown_alias_is_refused_with_one_stderr_line"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#a_traversing_target_phase_is_refused"
        status: pass
    human_judgment: false
  - id: D4
    description: "`next` writes no file anywhere (empty HOME/XDG dirs, unchanged .planning mtimes) and carries no artifact prose"
    requirement: PRMT-05
    verification:
      - kind: e2e
        ref: "tests/next_cli.rs#next_writes_nothing_anywhere_and_is_not_feature_gated"
        status: pass
      - kind: e2e
        ref: "tests/next_cli.rs#artifact_prose_never_reaches_the_output"
        status: pass
    human_judgment: false
  - id: D5
    description: "bin/gsd-meta-manager shim: forwards `next ...`, refuses everything else with exit 2 without running the binary"
    requirement: PRMT-05
    verification:
      - kind: unit
        ref: "src/launcher/config_dir.rs#the_gsd_meta_manager_shim_execs_next_and_refuses_everything_else"
        status: pass
      - kind: integration
        ref: "tests/launcher_config_dir.rs#the_next_shim_forwards_next_and_its_arguments_to_the_binary"
        status: pass
      - kind: integration
        ref: "tests/launcher_config_dir.rs#the_next_shim_refuses_any_other_verb_without_running_the_binary"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-10-19
status: complete
---

# Phase 29 Plan 03: Router hint (`next`) and sandbox shim Summary

**`gsd-meta-manager next <project> --json` exposes the Phase 20 router as a strictly read-only advisory JSON hint, dispatched before tracing, with a `next`-only shim seeded into the driver sandbox's PATH.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-20T03:16:00Z (approx.)
- **Completed:** 2026-10-20T03:28:00Z
- **Tasks:** 3 (tracer + 2 TDD)
- **Files modified:** 6 (2 new)

## Accomplishments

- `Commands::Next` (required `--json`, optional `--target-phase`) dispatched in `main` ahead of `init_tracing`; the later exhaustive match returns an error arm like `Driverctl`/`Driver`.
- `src/next_cli.rs`: `resolve_root` (path form, else read-only alias lookup), `default_target` (router's own `is_goal_met`, current milestone only), `hint_json` (one arm per `Decision`, no wildcard), `run` (exit 0 hint, exit 1 refusal).
- `bin/gsd-meta-manager` shim seeded beside `bin/driverctl`; idempotent; comment in source states it is a convenience, not a boundary.

Sample output, one line per decision kind (keys are alphabetical because `serde_json` does not preserve order):

```
run:      {"advisory":true,"command":"/gsd-plan-phase 1","decision":"run","detail":null,"project":"/tmp/samp","reason":"the phase has context or research on disk and no plans, so planning is the next step","source":"phase-20-router","target_phase":"1"}
park:     {"advisory":true,"command":null,"decision":"park","detail":"1","project":"/tmp/samp","reason":"gate_verification_gaps_found","source":"phase-20-router","target_phase":"1"}
goal_met: {"advisory":true,"command":null,"decision":"goal_met","detail":null,"project":"/tmp/samp","reason":"goal-met","source":"phase-20-router","target_phase":"1"}
none:     {"advisory":true,"command":null,"decision":"none","detail":null,"project":"/tmp/samp","reason":"no-target","source":"phase-20-router","target_phase":null}
no_rule:  {"advisory":true,"command":null,"decision":"no_rule","detail":"no_disk_inference","project":"/tmp/samp","reason":"no-rule","source":"phase-20-router","target_phase":"1"}
```

The `run`, `park`, `goal_met` and `none` lines were captured from the built binary against a real `.planning/` tree. The `no_rule` line is the shape `hint_json` produces for `Decision::NoRule`; no on-disk fixture reaches that arm today (every roadmap phase has a disk inference), so it is covered by the exhaustive match rather than by a captured run.

## Task Commits

1. **Task 1 (tracer): `next` reads a real project and prints the router decision** - `08502c7` (feat)
2. **Task 2 (TDD): default target, alias, refusals, read-only proof** - `246304a` (test, RED) then `c0656d5` (feat, GREEN)
3. **Task 3 (TDD): next-only shim in the per-run config dir** - `44caa68` (test, RED) then `0046571` (feat, GREEN)

**Plan metadata:** recorded in the following docs commit.

## TDD Gate Compliance

- **Task 2 RED:** 3 of the 4 `the_default_target_*` unit tests and 5 e2e tests failed on their planned assertions against a `None`-returning `default_target` stub (and a `--target-phase`-required `run`). Semantic assessment: the stub was added so the test targets compile, so each failure is an assertion on the planned behaviour, not a build error. The refusal and clap cases already passed (they pin behaviour that Task 1 had already made true) and are guards, not RED evidence.
- **Task 3 RED:** the four new shim tests failed because `bin/gsd-meta-manager` did not exist (read of the missing shim / spawn of a missing file). That is the planned absence, not a fixture or syntax fault.
- GREEN commits follow each RED commit; no REFACTOR commits were needed.

## Files Created/Modified

- `src/next_cli.rs` - the `next` command and its unit tests
- `tests/next_cli.rs` - binary e2e: shape, default target, alias/path, refusals, read-only proof, no-prose proof
- `src/cli.rs` - `Commands::Next` plus three clap tests
- `src/main.rs` - `mod next_cli;`, early dispatch before `init_tracing`, error arm in the async match
- `src/launcher/config_dir.rs` - `bin/gsd-meta-manager` shim and tests, module doc updated
- `tests/launcher_config_dir.rs` - exec tests with a fake binary

## Decisions Made

See `key-decisions` above. The two `[inferred - audit]` items (default target via `is_goal_met`; `--json` required) follow the plan text.

## Deviations from Plan

None - plan executed exactly as written. Two small points inside the plan's latitude: unit-test names use a `the_default_target_` prefix so the acceptance grep `fn default_target` matches the single function; and `rustfmt` was applied to the touched files.

**Total deviations:** 0
**Impact on plan:** none.

## Issues Encountered

None. Full suite: 3596 passed, 1 failed - the expected local-only `envelope::policy` git-version witness. `cargo clippy --all-targets -- -D warnings` is clean.

## Flagged assumption (carried from the plan, still UNRESOLVED)

A project whose ROADMAP has no current milestone or no phases yields `decision: "none"`, exit 0, rather than an error. Implemented that way; the verifier should confirm the reading with the operator.

## Threat Flags

None - `next` adds no network, auth or write surface; the shim adds no capability the ro-bound `/` did not already give the sandbox (T-29-12, accepted).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 29-01's master prompt can cite `gsd-meta-manager next {PROJECT_ROOT} --json`; the shim makes it resolve on `PATH` in the sandbox.
- No blockers.

## Self-Check: PASSED

- Files: `src/next_cli.rs`, `tests/next_cli.rs` exist; the other four were modified in place.
- Commits `08502c7`, `246304a`, `c0656d5`, `44caa68`, `0046571` are all ancestors of HEAD.
- Acceptance greps: `Commands::Next` x2 and `^mod next_cli;` x1 in main.rs; `"advisory"` in next_cli.rs; `fn default_target` x1, `is_goal_met`, `no-target`; both shim strings present.
- Verification: `--test next_cli` 12 passed, `--bin` next_cli 4 passed, `--lib launcher::config_dir` 8 passed, launcher/guard targets green, clippy clean.

---
*Phase: 29-master-prompt-run-templates*
*Completed: 2026-10-19*
