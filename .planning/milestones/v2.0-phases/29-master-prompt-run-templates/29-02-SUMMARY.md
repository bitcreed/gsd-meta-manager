---
phase: 29-master-prompt-run-templates
plan: 02
subsystem: driverctl
tags: [driverctl, store, launcher-gate, reconcile, control-pause, prompt-version, frontmatter]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: "RunStore, RunState, handle_log/handle_hook, DriverFrontmatter, the sandbox set-or-remove posture mechanics copied here"
  - phase: 28-sandboxed-driver-launcher
    provides: "launcher::gate::decide_dispatch + CapGate, the Gate hook the control word is threaded through"
provides:
  - "D-17: begin_launch records awaiting_recovery; handle_log rejects dispatched until each running item has one recovered; a second recovered per launch is rejected"
  - "D-18 / D-19(OQ5): decide_dispatch(.., tripped, control) denies pause|stop; the store rejects dispatched under pause|stop"
  - "D-03 store half: LaunchSpec.prompt_version, state.json prompt_version, frontmatter prompt_version, prompt=<v> on RUN | launch"
  - "D-09 store half: driverctl::store::frontmatter_for(&RunState, &DirectionInfo) -> DriverFrontmatter"
  - "tests/driverctl_reconcile.rs: binary-level proof of D-17 through driverctl serve + driverctl log"
affects: [29-04 driver new (frontmatter_for), 29-05 (matches the rejection and deny strings), 29-06 (sets LaunchSpec.prompt_version), 29-07 acceptance]

actuals:
  tokens: 14000
  tasks: 3
  commits: 7

plan_head_before: 3b04825c5ccb293285161c525df2dea031908c98
plan_head_after: 383a49cd97f7ce6bf99b71ff39ac71fd53b0aca2
commits: 7

tech-stack:
  added: []
  patterns:
    - "store-level acceptance rules checked in handle_log after validate_log and before apply_event (Response::rejected, rc 2), persisted in the same commit as the event"
    - "per-launch state keys set-or-removed in begin_launch (sandbox, prompt_version) so a launch never inherits its predecessor's value"
    - "behaviour-neutral plumbing refactor commit before the RED commit when a signature change would otherwise make RED a compile error"

key-files:
  created:
    - tests/driverctl_reconcile.rs
  modified:
    - src/driverctl/store.rs
    - src/launcher/gate.rs
    - src/launcher/run.rs
    - src/state_reader/driver_frontmatter.rs
    - tests/driverctl_store.rs
    - tests/driverctl_frontmatter.rs

key-decisions:
  - "D-18 pause semantics: control is honoured before the next dispatch [operator-confirmed 2026-10-19] (gate module docs carry the confirmed tag)."
  - "D-19 / RESEARCH OQ5 store half [inferred - audit]: the store also rejects dispatched under control pause|stop (D-19's live assertion is `no further dispatched`); the check precedes the D-17 check so a paused relaunch names the control word first."
  - "An awaiting id absent from the current DIRECTION items is skipped at check time, not pruned from state, so it blocks again if the human re-lists it."
  - "The Gate type's third argument is Option<&str> (the DIRECTION control word); decide_dispatch stays pure and the store reads DIRECTION on every driver-level dispatch hook."

patterns-established:
  - "Rejection strings are the contract plans 29-05/29-07 match on (quoted below)."

requirements-completed: [PRMT-04, PRMT-01]

duration: 12min
completed: 2026-10-19
status: complete
---

# Phase 29 Plan 02: Store and Gate Enforcement Summary

**The driverctl store now refuses `dispatched` until each item a crashed launch left running has exactly one `recovered`, the launcher gate and the store both refuse new dispatches under `control: pause|stop`, and every launch records its prompt version (`prompt=` on the launch line plus a `prompt_version` frontmatter key) through a pure `frontmatter_for`.**

## Performance

- **Duration:** 12 min (03:03 to 03:15 UTC, 2026-10-20)
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files:** 1 created, 6 modified; 7 commits (1117 insertions, 124 deletions)

## Exact strings (matched by plans 29-05 and 29-07)

Store rejections (rc 2 through the existing `Response::rejected` path; the CLI prints them as `driverctl: REJECTED: <message>`):

- `reconcile first: I-01 was running at launch start; record driverctl log recovered --item I-01 --action done|resume|reset|park` (the id is the lowest awaiting id in sorted order; the action list is `RECOVER_ACTIONS` joined with `|`)
- `already recovered in this launch: I-01`
- `control=pause: dispatch nothing new; record the in-flight report, then end --reason pause` (and the `stop` analogue)

Gate deny (the hook client prefixes `LAUNCHER CAP: `; logged as `RUN | dispatch-denied | reason=control=pause; ...` and counted in `denied_dispatches`):

- `control=pause; dispatch nothing new; record the in-flight worker's report, then driverctl log end --reason pause` (and the `stop` analogue)

Launch line: `RUN | launch | n=1 | mode=standalone | model=... | prompt=driver-prompt/1.0.0`; `prompt=` is absent when the launch carries no version. Frontmatter: `prompt_version: "driver-prompt/1.0.0"` right after the sandbox keys, only when set.

## Accomplishments

- **Tracer (D-17):** `RunState.awaiting_recovery` / `recovered_this_launch` (`serde(default)`, added to `RUN_STATE_FIELDS`, 16 to 18). `begin_launch` sets the sorted running ids and clears the recovered set; `handle_log` rejects `dispatched` while a still-listed awaiting id remains and rejects a second `recovered` for one item; an accepted `recovered` moves the id out of the awaiting set in the same committed state, so a store restart keeps it. A first `recovered` for an item that was not running is still accepted (`verified_needs_human_recovered_and_reconcile_use_the_spike_keys` unchanged). `tests/driverctl_reconcile.rs` (7 tests) drives `driverctl serve` + `driverctl log` as real processes and SIGKILLs serve #1: the main flow, the exact rejection text, no running item, a relaunch after a finished item, reverse-order recoveries naming `I-01` first, an awaiting item deleted from DIRECTION.md not wedging the run, and launch 3 accepting one `recovered` again.
- **Control (D-18, D-19):** `Gate` now takes the DIRECTION control word; `decide_dispatch` order is tripped, control, gap rule, dispatch cap. Store `handle_log` rejects `dispatched` under pause/stop before the D-17 check; `report`, `verified`, `end` and `recovered` stay accepted (tests cover a paused run recovering its running item).
- **Prompt version (D-03, D-09):** `LaunchSpec.prompt_version`, set-or-remove `prompt_version` extra key through `header_safe`, `prompt=` launch kv, additive `DriverFrontmatter.prompt_version` (no schema bump, byte-identical when unset), and `frontmatter_for` as the single derivation (`RunStore::frontmatter()` delegates). `src/launcher/run.rs` passes `prompt_version: None` until plan 29-06.

## Task Commits

1. **Task 1 (tracer): reconcile-first rule + real-binary tests** - `4a86684` (feat)
2. **Task 2:** plumbing `0d56048` (refactor), RED `15dbb75` (test), GREEN `16b5d6c` (feat)
3. **Task 3:** plumbing `2ce90d8` (refactor), RED `5d0571b` (test), GREEN `383a49c` (feat)

## TDD notes

- Both TDD tasks changed a signature or struct that every caller and exhaustive literal touches, so a RED commit holding only tests would not have compiled. Each therefore starts with a behaviour-neutral `refactor` commit (ignored parameter / unset field), after which RED fails on assertions: Task 2 RED failed 3 gate unit tests and 3 store tests on the planned assertions; Task 3 RED failed 1 lib, 1 frontmatter and 3 store tests on the planned assertions. `gsd_run check tdd-red-evidence` has no adapter for `cargo test` console output, so RED was assessed semantically (each target test executed and failed on its assertion).
- The tracer needed no RED commit (tracer, production-quality); its tests were written with the code and proved the rule through the real binary.

## Deviations from Plan

None - plan executed exactly as written, with two process additions: the two behaviour-neutral `refactor` plumbing commits described above, and a one-line doc renumbering in `src/launcher/gate.rs` (the `1b.` list item tripped clippy `doc_lazy_continuation`; the list now reads tripped, control, gap rule, dispatch cap).

**Total deviations:** 0 auto-fixed.

## Verification

- Plan `<verification>`: `--test driverctl_reconcile --test driverctl_store --test driverctl_replay --test driverctl_frontmatter --test launcher_caps --test launcher_e2e` green; `--lib launcher::gate` (17 passed) and `--lib driverctl::store` (9 passed) green; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- Acceptance greps: `awaiting_recovery` 6 and `recovered_this_launch` 7 in store.rs; `reconcile first` 5 in tests/driverctl_reconcile.rs; `control: Option<&str>` 1 and `control=` 5 in gate.rs; `Option<&str>) -> GateDecision` 1 in store.rs; `pub fn frontmatter_for` 1; `pub prompt_version: Option<String>` 1; `"prompt"` 1 in store.rs; `prompt_version` 1 in run.rs.
- Full suite (`cargo test --no-fail-fast`): 3573 passed, 1 failed, 22 ignored across 82 targets. The single failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` git-version witness (green on the CI runner). Every Phase 27 replay fixture passes unchanged.

## Known Stubs

None. `prompt_version: None` in `src/launcher/run.rs` is intentional: plan 29-06 sets the resolved version there (its tracer asserts `prompt=driver-prompt/1.0.0`).

## Threat Flags

None - no new endpoint, auth path or file access; the changes tighten acceptance rules on existing socket requests. Threats T-29-05 through T-29-09 are mitigated and each is covered by a test (reconcile-first and second-recovered; control gate and store; three-launch version replacement; `header_safe` on `prompt=`; removed-item no-wedge).

## Next Phase Readiness

Plan 29-04 can call `frontmatter_for(&RunState::new(..), &parse_direction(body)).emit()`; plan 29-05 can assert the rejection/deny strings above; plan 29-06 can set `LaunchSpec { prompt_version: Some(..), .. }`.

## Self-Check: PASSED

`tests/driverctl_reconcile.rs` exists and all seven task commits (`4a86684`, `0d56048`, `15dbb75`, `16b5d6c`, `2ce90d8`, `5d0571b`, `383a49c`) are ancestors of HEAD.
