---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 01
subsystem: infra
tags: [driver, supervisor, opt-in, capability-type, journal, rustix, clap, process-group]

requires:
  - phase: 15-transport-foundation
    provides: "ClaudeExecutor, DrivableProject, ExecutionHandle.pgid, the fake-claude shell fixtures and golden transcripts"
  - phase: 16-run-journal-state-substrate
    provides: "JournalRun, RunRecord (with the reserved opt_in field), run_paths, new_run_id, argv_digest"
provides:
  - "`gsd-meta-manager drive <alias> --command <c>`: the Drive subcommand, dispatched before tui::init()"
  - "`src/driver/`: the portable module root (DriveArgs, drive()) and the #[cfg(unix)] run body"
  - "DrivableProject::from_registry — the only production constructor of the capability token"
  - "DriverOptIn + RegisteredProject.driver_opt_in — the opt-in record on the registry schema"
  - "OptInError and DriveError — typed, pre-spawn refusals"
  - "JournalEvent::ExecStarted.claude_pgid + JournalRun::set_claude_pgid — the teardown handle for a SIGKILLed driver"
  - "The first production caller of Phase 15's ClaudeExecutor and Phase 16's JournalRun"
  - "tests/spawn_seam_guard.rs — the repository's first mechanical source-tree audit"
  - "tests/fixtures/fake-claude-cwd.sh — a stand-in that records its own $PWD"
affects: [17-02 lock, 17-03 migration, 17-04 dry-run, 17-05 detached spawn and reattach, 17-06 kill switch, 17-07 UI]

tech-stack:
  added: ["rustix 1.1 (features: process, fs)"]
  patterns:
    - "Capability token with exactly two constructors, one production and one #[doc(hidden)] escape hatch, fenced by a mechanical test"
    - "Mechanical source-tree audit: walk src/**, filter comment lines, assert against a declared const allowlist"
    - "Portable CLI surface + #[cfg]-gated implementation, with a typed unsupported-platform refusal instead of a missing subcommand"
    - "select! loop with a single arm today, so a later plan adds a variant rather than restructuring the body"

key-files:
  created:
    - src/driver/mod.rs
    - src/driver/run.rs
    - tests/driver_tracer.rs
    - tests/spawn_seam_guard.rs
    - tests/fixtures/fake-claude-cwd.sh
  modified:
    - Cargo.toml
    - src/lib.rs
    - src/cli.rs
    - src/main.rs
    - src/config.rs
    - src/error.rs
    - src/registry.rs
    - src/app.rs
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/journal/mod.rs
    - src/journal/reader.rs
    - tests/executor_lifecycle.rs
    - tests/executor_transport.rs

key-decisions:
  - "execute_run takes the RegisteredProject as a third parameter, because RunRecord.opt_in needs the opt-in timestamp and the capability token carries only alias and root by construction"
  - "--dry-run refuses with a typed error rather than silently performing a real run; plan 17-04 replaces that arm with the real preview"
  - "close_input() is called immediately after spawn: one command means one message, and without EOF a real claude waits for a turn this phase never sends"
  - "The spawn allowlist has 10 entries, not the 6 the plan estimated — main.rs, executor/outcome.rs, state_reader/queue_md.rs and ui/screens/detail.rs also spawn processes"
  - "Two in-source tests that built a token through the escape hatch were moved or retired rather than exempted, keeping the D-17 fence absolute"
  - "CLI rationale lives in `//` comments, not `///` docs, because clap renders doc comments verbatim into --help"

patterns-established:
  - "Mechanical guard test: a comment is not a guard; walk the tree, filter comments, assert a const allowlist"
  - "A guard must be observed RED on a planted violation before it is trusted green"
  - "Runtime-assembled search literal (two halves concatenated) so a guard cannot match its own source"

requirements-completed: [CTRL-03, CTRL-04]

coverage:
  - id: D1
    description: "A drive against an opted-in project runs one GSD command to completion and leaves a complete run directory: run.json with ended_at and outcome, journal.jsonl from run_started to run_ended, active pointer cleared"
    requirement: "CTRL-04"
    verification:
      - kind: integration
        ref: "tests/driver_tracer.rs#a_drive_against_an_opted_in_project_leaves_a_complete_run_on_disk"
        status: pass
    human_judgment: false
  - id: D2
    description: "The same invocation against a project with no driver_opt_in record is refused before anything is spawned and writes nothing at all on disk"
    requirement: "CTRL-03"
    verification:
      - kind: integration
        ref: "tests/driver_tracer.rs#a_drive_against_a_project_with_no_opt_in_record_is_refused_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "src/executor/mod.rs#from_registry_refuses_a_project_with_no_opt_in_record"
        status: pass
      - kind: unit
        ref: "src/driver/mod.rs#drive_refuses_an_unknown_alias_without_touching_disk"
        status: pass
    human_judgment: false
  - id: D3
    description: "RunRecord.pid and RunRecord.pgid are equal and non-zero in every launch mode, established by setpgid(0, 0) at run entry rather than assumed"
    verification:
      - kind: integration
        ref: "tests/driver_tracer.rs#the_run_record_carries_the_drivers_own_pid_and_an_equal_pgid"
        status: pass
    human_judgment: false
  - id: D4
    description: "The claude process group id is journaled on the exec_started event, so a SIGKILLed driver does not orphan an untraceable tree"
    verification:
      - kind: integration
        ref: "tests/driver_tracer.rs#the_exec_started_event_carries_the_claude_process_group_id"
        status: pass
    human_judgment: false
  - id: D5
    description: "The opt-in escape hatch is renamed to read as an alarm, is #[doc(hidden)], and has zero non-comment occurrences under src/ outside its own definition"
    requirement: "CTRL-03"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_escape_hatch_has_no_call_site_in_src"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#drivable_project_has_exactly_two_constructors_and_private_fields"
        status: pass
    human_judgment: false
  - id: D6
    description: "The set of process-spawn sites under src/ equals a declared const allowlist, and the agent spawn seam still takes the capability type"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#every_process_spawn_site_in_src_is_on_the_allowlist"
        status: pass
    human_judgment: false
  - id: D7
    description: "No executable line under src/ opts into strict unknown-field rejection — the guard src/journal/ has claimed since Phase 16 now exists and both passages name it"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#no_executable_line_in_src_opts_into_strict_unknown_field_rejection"
        status: pass
    human_judgment: false
  - id: D8
    description: "The Drive subcommand parses with --command, --run-id, --dry-run and --goal visible and both development flags hidden; an unknown alias exits non-zero with `Error: ` on stderr"
    verification:
      - kind: manual_procedural
        ref: "cargo run -- drive --help; cargo run -- drive nosuchalias --command x"
        status: pass
    human_judgment: false

duration: 22 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 01: Tracer — One Gated, Journaled GSD Command Summary

**`gsd-meta-manager drive <alias> --command <c>` travels argv → opt-in gate → journal start → real child process → event drain → journal finish → exit, and the same invocation against a non-opted-in project is refused before anything spawns and leaves nothing on disk.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-07-29T16:38:00Z
- **Completed:** 2026-07-29T17:00:00Z
- **Tasks:** 2
- **Files modified:** 20 (5 created, 15 modified)

## Accomplishments

- **The phase's architectural bet is proved on the first commit.** A `drive` invocation now touches every layer the phase will modify — CLI surface, pre-`tui::init()` dispatch, driver module root and run body, capability type, opt-in record, typed refusals and journal — and an integration test closes the loop by reading the finished run directory off disk.
- **Two Phase-15/16 components stopped being dead code.** `src/journal/mod.rs` said in as many words that *"nothing in this repository spawns a `ClaudeExecutor` yet; wiring a real one is Phase 17's"*. `src/driver/run.rs` is that caller, and it is also the first production caller of `JournalRun`. Success criterion #2 ("reopening the TUI shows that run still live") now has a journal to read.
- **The opt-in gate is structural, not procedural.** `DrivableProject::from_registry` is the only production constructor, the `Drive` handler is its only production caller, and because the gate lives in the *driver process* a hand-typed `drive` is refused by the same code as a TUI-initiated one.
- **`pid == pgid` is established by a call, not assumed by a comment.** `setpgid(0, 0)` at run entry makes D-04's arithmetic honest for a hand-typed `drive`, which would otherwise inherit the shell's job group and record a pgid naming a group it does not lead.
- **The repository has its first mechanical source-tree audit**, and it was observed failing on a planted violation before it was trusted passing. It also retired a guard `src/journal/` had been *claiming* since Phase 16 but never had.

## Task Commits

1. **Task 1: End-to-end "one gated, journaled GSD command"** — `05bb039` (feat)
2. **Task 2: The mechanical spawn-seam audit** — `65653e1` (test)

## Files Created/Modified

**Created**

- `src/driver/mod.rs` — Portable module root: `DriveArgs`, `drive()`, the `#[cfg(unix)] pub mod run;` declaration, and the non-Unix typed refusal.
- `src/driver/run.rs` — The run body: `establish_own_group`, `make_run_record`, `outcome_label`, `DriverRun`, `execute_run`.
- `tests/driver_tracer.rs` — The four end-to-end proofs, against the checked-in shell stand-in with no subscription.
- `tests/spawn_seam_guard.rs` — The four mechanical guards D-17 prescribes.
- `tests/fixtures/fake-claude-cwd.sh` — A transcript-replaying stand-in that records its own `$PWD` for plan 17-03's cross-project isolation proof.

**Modified**

- `Cargo.toml` — `rustix 1.1` with `process` and `fs`, with the reasoning recorded in the house dependency-comment style.
- `src/lib.rs` — `pub mod driver;`, alphabetically between `config` and `error`.
- `src/cli.rs` — The `Drive` struct variant; two hidden development flags.
- `src/main.rs` — The `Some(Commands::Drive { .. })` arm, by construction before `tui::init()`.
- `src/config.rs` — `DriverOptIn` and `RegisteredProject.driver_opt_in`.
- `src/error.rs` — `OptInError` and `DriveError`, hand-written `Display`/`Error`.
- `src/registry.rs` — Two struct literals now write `driver_opt_in: None` explicitly.
- `src/app.rs` — One test struct literal, same field.
- `src/executor/mod.rs` — `from_registry`; `for_testing` → `for_testing_bypassing_opt_in` + `#[doc(hidden)]`; both stale doc comments rewritten; three new unit tests.
- `src/executor/claude.rs` — Escape-hatch rename sweep; one in-source test relocated.
- `src/journal/mod.rs` — `ExecStarted.claude_pgid`, `JournalRun.claude_pgid`, `set_claude_pgid`, the `record_exec` stamping arm; the strict-unknown-field claim now names its guard.
- `src/journal/reader.rs` — Same claim, same correction.
- `tests/executor_lifecycle.rs`, `tests/executor_transport.rs` — Escape-hatch rename sweep; the relocated refusal test.

## Decisions Made

- **`execute_run` takes a third parameter.** The plan specified `execute_run(project, args)` *and* required `RunRecord.opt_in` to carry `DriverOptIn.opted_in_at`. Those are incompatible: `DrivableProject` carries only alias and root, deliberately. Passing `&RegisteredProject` resolves it without a panic, without widening the capability token, and without a second gate check.
- **`--dry-run` refuses rather than executing.** The plan parks the behaviour in 17-04 and says only that the flag is parsed here. A parsed-but-unimplemented preview flag that performs a *real* run is precisely the failure CTRL-02 exists to prevent, so `drive()` returns `DriveError::DryRunUnavailable`. 17-04 replaces that arm — which is exactly the "one arm added to `drive()`" the plan anticipates.
- **`close_input()` immediately after spawn.** The plan's step list omitted it. Without EOF a real `claude` waits for a second turn this phase never sends, so the run would never terminate. The fixture would have masked this entirely: it exits on its own.
- **CLI rationale moved out of doc comments.** clap renders `///` verbatim into `--help`, so the first draft's decision-id commentary became user-facing help text. Rationale now lives in `//` comments directly above each field; the `///` line is the one sentence a user needs.
- **`--claude-program` is a hidden flag, not an environment variable.** An env var is inherited by children, so a stray value in the user's shell would silently reach a TUI-spawned driver — the same leak class `claude.rs` already scrubs `CLAUDE*` for.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `execute_run`'s specified signature cannot populate `RunRecord.opt_in`**

- **Found during:** Task 1
- **Issue:** The plan gives `execute_run(project: DrivableProject, args: &DriveArgs)` and simultaneously requires `RunRecord.opt_in` to be "set from the project's `DriverOptIn.opted_in_at`". `DrivableProject` carries only `alias` and `root` by construction, and widening it would defeat the guard test asserting it has no public fields.
- **Fix:** Added a third parameter, `entry: &RegisteredProject`. `make_run_record` reads `entry.driver_opt_in.as_ref().map(...)`, which is non-panicking and needs no second gate check.
- **Files modified:** `src/driver/run.rs`, `src/driver/mod.rs`
- **Verification:** `tests/driver_tracer.rs` asserts `run.json`'s `opt_in` equals the registered timestamp.
- **Committed in:** `05bb039`

**2. [Rule 2 - Missing Critical] `--dry-run` would have silently performed a real run**

- **Found during:** Task 1
- **Issue:** The plan parses `--dry-run` here and implements it in 17-04. As specified, `gsd-meta-manager drive foo --command /gsd-execute-phase --dry-run` would have spawned a real agent — the "looks done but isn't" failure PITFALLS:63 names, and the one CTRL-02 exists to prevent.
- **Fix:** Added `DriveError::DryRunUnavailable`; `drive()` refuses **after** the opt-in gate and before dispatch. Documented on both the variant and the arm that 17-04 replaces it.
- **Files modified:** `src/error.rs`, `src/driver/mod.rs`
- **Verification:** `cargo run -- drive <alias> --command x --dry-run` exits non-zero and spawns nothing.
- **Committed in:** `05bb039`

**3. [Rule 2 - Missing Critical] The run body never signalled end-of-input**

- **Found during:** Task 1
- **Issue:** The plan's ten-step run body goes from `set_claude_pgid` straight to the drain. A real `claude` reads stdin for the whole run and only exits on EOF; without `close_input()` the drain would never end. Every checked-in fixture exits on its own, so no test in this plan could have caught it.
- **Fix:** `handle.close_input()` immediately after `set_claude_pgid`, with the failure downgraded to a `tracing::warn!` (the process may already have exited).
- **Files modified:** `src/driver/run.rs`
- **Verification:** Matches `tests/executor_transport.rs::tracer_runs_one_command_end_to_end`, which does the same for the same reason.
- **Committed in:** `05bb039`

**4. [Rule 1 - Bug] The plan's spawn allowlist was incomplete (6 of 10 files)**

- **Found during:** Task 2
- **Issue:** The plan named `executor/claude.rs`, `state_reader/git_ops.rs`, `journal/writer.rs`, `project_creator.rs`, `session_detector.rs` and `terminal_switch.rs`. A grep for the three spawn markers over non-comment lines also finds `src/main.rs` (the `$EDITOR` shell-out), `src/executor/outcome.rs` (git, in its own test helper), `src/state_reader/queue_md.rs` (the `gsd-tools` launcher) and `src/ui/screens/detail.rs` (`which` + terminal launch). The allowlist as specified would have failed the test.
- **Fix:** Allowlist carries all 10 entries with one comment per entry. None takes a capability type and none spawns an agent, which is what the audit is actually asking.
- **Files modified:** `tests/spawn_seam_guard.rs`
- **Verification:** `every_process_spawn_site_in_src_is_on_the_allowlist` passes, and asserts set *equality* — a stale entry fails just as loudly as a missing one.
- **Committed in:** `65653e1`

**5. [Rule 1 - Bug] Two in-source tests built the capability token through the escape hatch**

- **Found during:** Task 2 (the guard found them on its first run)
- **Issue:** `src/executor/claude.rs::a_refused_run_writes_zero_bytes_to_the_child_stdin` and `src/executor/mod.rs::a_drivable_project_carries_its_alias_and_root` both called the escape hatch. Both are under `src/`, so D-17's fence would have had to be weakened to accommodate them.
- **Fix:** The first spawns a real child process, which is this repository's own stated criterion for an integration test, so it moved to `tests/executor_transport.rs` — where the hatch is legitimate — unchanged. The second asserted the two accessors; `from_registry_accepts_a_project_carrying_an_opt_in_record` asserts the identical accessors off a production-built token, so it was retired as redundant with a comment recording where its coverage went.
- **Files modified:** `src/executor/claude.rs`, `src/executor/mod.rs`, `tests/executor_transport.rs`
- **Verification:** `the_escape_hatch_has_no_call_site_in_src` passes with exactly one occurrence, the `pub fn` definition. Suite total rose from 427 to 439.
- **Committed in:** `65653e1`

**6. [Rule 1 - Bug] clap rendered decision-id rationale into user-facing `--help`**

- **Found during:** Task 1
- **Issue:** The plan asks for a `///` doc on the variant and every field because clap renders them. The first draft put the full rationale there, so `drive --help` printed paragraphs about D-05 and Phase 20 to the user.
- **Fix:** One-line `///` per field; rationale moved to `//` comments directly above. The `hide = true` acceptance criterion is unaffected — its grep filters comment lines.
- **Files modified:** `src/cli.rs`
- **Verification:** `cargo run -- drive --help` now fits on one screen and shows exactly the four intended flags.
- **Committed in:** `05bb039`

---

**Total deviations:** 6 auto-fixed (3 missing-critical/blocking, 3 bugs). **Impact:** every one was necessary for correctness or for an acceptance criterion to be satisfiable. No scope creep — the only new public symbol beyond the plan's list is one `DriveError` variant, which plan 17-04 replaces.

## Issues Encountered

**`rtk`'s `grep` wrapper drops piped stdin.** The plan already warns that `rtk` filters raw `cargo` output and that criteria greping for `warning:` or `test result:` pass vacuously. A second instance of the same hazard surfaced here: acceptance criteria of the form `grep -v '…' FILE | grep -c '…'` returned **0** under the hook, because the second `grep` in the pipeline receives no stdin. Both affected criteria (`std::process::id()` × 2, `hide = true` × 2) actually pass and were verified with `rtk proxy sh -c '<pipeline>'`. Worth recording for later plans: **wrap the whole pipeline in `rtk proxy sh -c`, not just the `cargo` invocation**, whenever a criterion pipes one filter into another.

**The guard was observed red before green (recorded per acceptance criterion).** A line containing the escape-hatch identifier was appended to `src/driver/run.rs`; `cargo test --test spawn_seam_guard` failed with:

```
`for_testing_bypassing_opt_in` bypasses the user's opt-in. A production call site means the
compiler is no longer what enforces the gate, and CTRL-03's "never" stops being literally
true (D-17, PITFALLS:521). Offending lines:
  src/driver/run.rs:236: fn _planted() { let _ = DrivableProject::for_testing_bypassing_opt_in("x", "/tmp/x"); }
```

The line was then reverted with `git checkout -- src/driver/run.rs` and all four guards pass. The guard had already proved itself independently by finding two *real* violations on its very first run (deviation 5).

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **439 passed, 0 failed** (baseline 427) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/driver/`, `src/cli.rs`, `src/config.rs`, `src/error.rs`, `src/executor/mod.rs` or `src/journal/` |
| `git diff Cargo.lock \| grep -c '^+name = '` | `0` — `rustix` was already in the lockfile transitively at 1.1.4 |
| `cargo add --dry-run rustix --features process,fs` | resolves 1.1.4, adds no other crate |
| `cargo run -- drive --help` | exits 0; shows `--command`, `--run-id`, `--dry-run`, `--goal`; hides both development flags |
| `cargo run -- drive nosuchalias --command x` | exits 1, stderr begins `Error: ` |

## Known Stubs

None. `--dry-run` is not a stub: it is a typed refusal that names its owner plan, and it executes nothing.

## User Setup Required

None — no external service configuration required. The integration tests run against checked-in shell fixtures with no subscription, network or quota dependency.

## Next Phase Readiness

Every later plan in the phase is downstream of this one and each is now a field or variant addition rather than a layer move:

- **17-02 (`flock`)** — adds one field to `DriverRun`, which exists and is documented as the place for it.
- **17-03 (migration, write-side hardening)** — `DriverOptIn` and `RegisteredProject.driver_opt_in` exist; `tests/fixtures/fake-claude-cwd.sh` is checked in and executable for the cross-project isolation proof.
- **17-04 (dry-run)** — replaces one arm in `drive()`, which is already gated ahead of it.
- **17-05 (detached spawn, reattach)** — is a *caller* of this CLI surface. It must add `src/driver/spawn.rs` to `SPAWN_ALLOWLIST` in the same commit; the guard fails otherwise, which is the intent.
- **17-06 (kill switch)** — adds a SIGTERM arm to the `select!` loop in `execute_run`, which is a single-arm loop today for exactly that reason, and reuses `outcome_label` for the killed path.

One forward note for 17-05: `RunRecord.claude_code_version` is written empty and never overwritten, because `run.json` is written exactly twice and the version is only known after the first `system/init`. If the TUI needs it, the value is on the `exec_started` journal record, not the run record.

## Requirements Traceability

`CTRL-03` and `CTRL-04` are both **declared by sibling plans that have not finished yet**
(`CTRL-03` by 17-03 and 17-07; `CTRL-04` by 17-05 and 17-07), so neither is marked complete in
`REQUIREMENTS.md` by this plan. Marking them now would flip them to `Complete` while their
siblings are still unwritten, which is exactly the gap phase verification exists to catch.
`REQUIREMENTS.md` is therefore deliberately untouched.

## Self-Check: PASSED

- All five created files verified present on disk.
- All three commits verified present in `git log`: `05bb039`, `65653e1`, `4b38ba6`.
- All Task 1 and Task 2 acceptance criteria re-run after the final commit; all pass.
- Plan-level verification re-run: build clean, 439 tests passing (baseline 427), zero
  failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still
  exactly 5 and all pre-existing.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
