---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 06
subsystem: infra
tags: [driver, kill-switch, process-group, signal, rustix, sigterm, reaping, zombie, ctrl-01]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 01
    provides: "src/driver/{mod,run}.rs, execute_run's single-arm select! loop, outcome_label, the two hidden development flags, JournalEvent::ExecStarted.claude_pgid, tests/spawn_seam_guard.rs"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 02
    provides: "src/driver/lock.rs — RunLock held on DriverRun, released by the descriptor close after the terminal write"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 05
    provides: "liveness::is_run_alive / process_state / is_zombie, reconcile::ObservedRun (the pgid to signal), spawn::spawn_detached's reaping task, App::start_driver_run, AppContext.observed_runs"
  - phase: 15-transport-foundation
    provides: "Executor::cancel — the already-built SIGTERM → 10s grace → SIGKILL → unconditional wait() on the claude group; tests/fixtures/fake-claude-spawner.sh"
provides:
  - "`src/driver/kill.rs`: DRIVER_TEARDOWN_GRACE, ReapArm, StopOutcome, signal_group, stop_run — D-06 layers 1 and 4 and both of D-07's reaping arms"
  - "The driver's terminate arm: a biased select! whose first arm calls Executor::cancel ONCE, journals the reason, and writes the killed terminal record"
  - "`Action::DriverStopRequested` / `Action::DriverStopped`"
  - "`AppContext.session_spawned_runs` — the set that decides the reap arm, deliberately not a flag on the scan-replaced observed map"
  - "`App::stop_driver_run` — the TUI's stop seam, dispatched off the render thread"
  - "tests/driver_kill.rs — ROADMAP success criterion #1 over three real process levels"
  - "A left word boundary on the spawn-seam guard's `process_group(` marker, so signalling a group is no longer reported as spawning one"
affects: [17-07 UI key binding, 18 driver tab and orphan sweep, 20 decision router]

tech-stack:
  added: []
  patterns:
    - "biased select! with the signal arm first, so a stop never loses the race to a busy event queue"
    - "Layer-2-by-call: a teardown that already exists is invoked once, never re-implemented, with the reason for the prohibition written at the call site"
    - "A constant wedged between two neighbours has BOTH bounds unit-asserted rather than commented"
    - "A marker-based source guard matches on a word boundary, not a substring, or an unrelated call forces a file onto an allowlist and silently changes what the audit means"
    - "Observable-constant convention applied in-module: the private neighbour a bound is asserted against is mirrored inside the test module"

key-files:
  created:
    - src/driver/kill.rs
    - tests/driver_kill.rs
  modified:
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/error.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "A driver that cannot install its terminate handler REFUSES the run, before anything touches disk, rather than warning and continuing: without the handler the kill switch is a silent no-op and SIGTERM would kill the driver by default disposition while orphaning the whole claude group"
  - "The refusal is DriveError::UnsupportedPlatform rather than a new variant — tokio only rejects signals the kernel will not let a process catch, so a build that errors on SIGTERM is a platform that cannot support driving at all"
  - "The terminal label comes from the outcome Executor::cancel actually returned, not a hard-coded Killed: the coordinator maps a cancelled run onto Killed by construction, and using the real value reports a wall-clock or idle breach truthfully instead of overwriting it"
  - "The reap arm is decided by a separate `session_spawned_runs` set, never a flag on ObservedRun: the observed map is REPLACED wholesale by every scan, so a flag there survives at most five seconds, and the disk cannot supply the answer because run.json records the driver's pid and never its parent"
  - "tests/driver_kill.rs drops the spawned Child immediately, so the test process genuinely holds no handle to wait() on — which is what makes ReapArm::Adopted the honest arm and the /proc re-probe the only confirmation available"
  - "Group survivors are probed by scanning /proc for a matching pgrp rather than by a zero-signal group probe, because the zero signal reports a zombie as alive and a surviving zombie is one of the four things criterion #1 forbids"

patterns-established:
  - "Plant the documented failure itself, not an approximation: `std::process::exit(143)` in place of layer 2 models a default-disposition SIGTERM exactly, because it runs no destructors — and it proved that process-wrap's KillOnDrop backstop does NOT save the tree"
  - "A negative grep over a test file must be evaluated comment-filtered, and where a literal appears in a panic message the message is reworded rather than the criterion excused"

requirements-completed: []

coverage:
  - id: K1
    description: "Stopping a run leaves no claude process, no grandchild build or server process, and no zombie behind — verified 15 seconds later, with the grandchild observed alive before the stop"
    requirement: "CTRL-01"
    verification:
      - kind: integration
        ref: "tests/driver_kill.rs#stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie"
        status: pass
      - kind: manual_procedural
        ref: "planted `std::process::exit(143)` in place of layer 2; the test failed naming the two surviving members of the agent's process group, then the plant was reverted"
        status: pass
    human_judgment: false
  - id: K2
    description: "Teardown is two-layer and layer 2 is a single CALL into Phase 15's existing sequence, never a second implementation"
    requirement: "CTRL-01"
    verification:
      - kind: manual_procedural
        ref: "comment-filtered greps over src/driver/run.rs: cancel(=1, start_kill|SIGKILL|signal(15)|kill_process_group=0; cancel at line 180 precedes the terminate-path finish at 196"
        status: pass
      - kind: integration
        ref: "tests/driver_kill.rs#stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie — the agent pgid is asserted DISTINCT from the driver's, so the test is about the two-group case"
        status: pass
    human_judgment: false
  - id: K3
    description: "Both reaping arms exist and the adopted arm is exercised by a test that would hang without it"
    verification:
      - kind: integration
        ref: "tests/driver_kill.rs#the_adopted_arm_confirms_death_by_probing_proc_rather_than_by_wait"
        status: pass
      - kind: unit
        ref: "src/driver/kill.rs#both_reaping_arms_name_who_performs_the_wait"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_run_this_session_did_not_spawn_is_never_recorded_as_our_child"
        status: pass
    human_judgment: false
  - id: K4
    description: "Group signals go through rustix::process::kill_process_group; a zero or oversized pgid is refused rather than signalling the caller's own group; no shell-out to kill exists on any executable line"
    verification:
      - kind: unit
        ref: "src/driver/kill.rs#a_zero_process_group_is_refused_rather_than_signalled"
        status: pass
      - kind: manual_procedural
        ref: "comment-filtered greps: kill_process_group=1 and from_raw=1 in kill.rs; the repository-wide shell-out grep is 0 on executable lines"
        status: pass
    human_judgment: false
  - id: K5
    description: "The grace exceeds the claude group's own grace plus slack and is strictly below criterion #1's 15-second verification point, both asserted"
    verification:
      - kind: unit
        ref: "src/driver/kill.rs#the_driver_grace_exceeds_the_claude_group_grace_plus_slack"
        status: pass
      - kind: unit
        ref: "src/driver/kill.rs#the_driver_grace_is_strictly_below_the_fifteen_second_verification_point"
        status: pass
      - kind: integration
        ref: "tests/driver_kill.rs — VERIFY_AFTER > DRIVER_TEARDOWN_GRACE asserted in the criterion test itself"
        status: pass
    human_judgment: false
  - id: K6
    description: "A stopped run is distinguishable on disk from a crashed one — ended_at present, killed outcome, terminate diagnostic, cleared active pointer — and its lock is released"
    requirement: "CTRL-01"
    verification:
      - kind: integration
        ref: "tests/driver_kill.rs#a_stopped_run_leaves_a_terminal_record_and_a_released_lock"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#the_outcome_label_for_a_stopped_run_is_killed"
        status: pass
      - kind: manual_procedural
        ref: "under the layer-2 plant the same test failed on the missing ended_at, then the plant was reverted"
        status: pass
    human_judgment: false
  - id: K7
    description: "The 12-second stop never blocks the render thread; it is dispatched on a task and returns as an Action"
    verification:
      - kind: unit
        ref: "src/app.rs#a_stop_returns_its_outcome_as_an_action_rather_than_blocking"
        status: pass
      - kind: unit
        ref: "src/app.rs#stopping_an_alias_with_no_observed_run_is_refused_visibly"
        status: pass
      - kind: manual_procedural
        ref: "comment-filtered grep: exactly one `stop_run` occurrence in src/app.rs, at line 914, inside the tokio::spawn at 912"
        status: pass
    human_judgment: false
  - id: K8
    description: "The spawn-seam audit still means what it says: signalling an existing process group is not reported as spawning one"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#a_signal_to_a_process_group_is_not_mistaken_for_a_spawn"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#every_process_spawn_site_in_src_is_on_the_allowlist"
        status: pass
    human_judgment: false

duration: 31 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 06: The Kill Switch Summary

**Pressing stop now removes the whole tree: fifteen seconds after `stop_run`, a real detached driver, the fake `claude` leading its own second process group, and that agent's real backgrounded grandchild are all gone with none of them in the zombie state — and planting a driver that stops without tearing `claude` down made the test fail naming the two survivors, which is the failure this design exists to prevent.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-07-29T18:22:00Z
- **Completed:** 2026-07-29T18:53:00Z
- **Tasks:** 3
- **Files modified:** 10 (2 created, 8 modified)

## Accomplishments

- **ROADMAP success criterion #1 is proved at full depth, and the proof was observed failing on the exact documented failure first.** The test runs three real process levels and asserts the grandchild alive *before* the stop and gone after. Planting `std::process::exit(143)` in place of layer 2 — which models a default-disposition SIGTERM precisely, because it runs no destructors — failed the test with *"the agent's process group 4102479 still has members: [(4102479, 'S'), (4102481, 'S')]"*. That plant also settled a real question: `process-wrap`'s `KillOnDrop` backstop does **not** save the tree, because a driver that dies without unwinding never drops it. Layer 2 is load-bearing, not belt-and-braces.
- **Layer 2 is a single call and the greps prove it.** `cancel(` appears exactly once on an executable line of `src/driver/run.rs`, and `start_kill|SIGKILL|signal(15)|kill_process_group` appears zero times — the driver's own body contains no teardown. The call sits at line 180, the terminate-path `finish` at 196, so the journal is closed *after* the agent is gone and never before.
- **The most dangerous primitive in the phase refuses its most dangerous input.** `kill(0, sig)` signals the caller's own process group, which under a TUI is the user's whole terminal session. `signal_group` turns a zero (or oversized) pgid into a typed refusal, and `a_zero_process_group_is_refused_rather_than_signalled` is what keeps that true.
- **Both of D-07's arms are implemented and the adopted arm is exercised by a test that would hang without it.** `tests/driver_kill.rs` drops the spawned `Child` immediately, so the test process genuinely holds nothing to `wait()` on — the `/proc` re-probe is the only confirmation available — and the call is wrapped in a bounded timeout that must not fire.
- **The zombie half is answered by the helper that can answer it.** `liveness::is_zombie` reads `/proc/<pid>/stat`; there is no `kill -0` on any executable line of the new test, because the zero signal succeeds for a zombie and would report the criterion satisfied while the process table filled up. Group survivors are found the same way, by scanning `/proc` for a matching `pgrp`, so a surviving zombie is visible rather than invisible.
- **The spawn-seam audit was quietly about to stop meaning what it says, and it was fixed rather than worked around.** The guard's `process_group(` marker matches inside `rustix::process::kill_process_group(`, so a module that only *signals* a group was reported as a process-spawn site. The path of least resistance — put a file that spawns nothing onto a **spawn** allowlist — would have converted an audit into a list of files somebody once had to add. A left word boundary plus its own test fixes it.

## Task Commits

1. **Task 1: The driver's SIGTERM arm reuses the teardown that already exists** — `0079cdb` (feat)
2. **Task 2: Signal the driver's group, escalate, and reap through both arms** — `65f198c` (feat)
3. **Task 3: Prove the tree is gone — the grandchild, the zombie, and the 15 seconds** — `7b0ab59` (test)

## Files Created/Modified

**Created**

- `src/driver/kill.rs` — `DRIVER_TEARDOWN_GRACE`, `ReapArm` (+ `reaper`), `StopOutcome` (+ `Display`), `signal_group`, `gone_within`, `stop_run`, and five unit tests. The module doc opens with the fact that makes the whole design necessary — two process groups, not one — and numbers D-06's four steps against who owns each.
- `tests/driver_kill.rs` — the three criterion-#1 proofs plus `Fixture`, `spawn_detached_driver`, `journal_yields`, `announced_grandchild`, `journaled_claude_pgid`, `group_members`, `present`, and the poll helpers. Its banner explains why three real process levels are required and why the existing liveness helper cannot be used for the zombie half.

**Modified**

- `src/driver/run.rs` — `TERMINATE_DIAGNOSTIC_CODE`; `shutdown_on_terminate` (layers 2 and 3); the handler installed as the first statement of `execute_run`; the drain loop converted to a `biased` `select!` with the terminate arm first; two unit tests.
- `src/driver/mod.rs` — `#[cfg(unix)] pub mod kill;` plus a fifth numbered fact in the module-root doc stating D-06's shape once, at the level a reader arriving at `src/driver/` needs.
- `src/error.rs` — a paragraph on `DriveError`'s doc recording that 17-06 added **no** variant and why the absence is deliberate.
- `src/action.rs` — `DriverStopRequested` and `DriverStopped`, the second carrying an already-rendered outcome because `StopOutcome` is `#[cfg(unix)]` and `Action` is not.
- `src/app.rs` — the `session_spawned_runs` field in the exhaustive `AppContext` literal; the two new `Action` arms; the run id recorded on a successful spawn; `App::stop_driver_run`; four new unit tests.
- `src/ui/screens/mod.rs` — the `session_spawned_runs` sibling set with a five-bullet doc naming why it is not a flag on `ObservedRun`.
- `src/ui/screens/detail.rs` — one test-helper `AppContext` literal the new field broke.
- `tests/spawn_seam_guard.rs` — `calls_marker` with its left word boundary, the `src/driver/kill.rs` allowlist entry, and `a_signal_to_a_process_group_is_not_mistaken_for_a_spawn`.

## Decisions Made

- **A driver that cannot install its terminate handler refuses the run.** The plan left the choice open ("decide and record which, in a comment"). Warning and continuing was rejected: without the handler, SIGTERM takes the default disposition, the driver dies *instantly* with no unwinding, and the `claude` group — a different process group that never received anything — is orphaned along with its grandchildren. That is precisely the failure CTRL-01 exists to prevent, so it refuses; and refusing costs nothing because the handler is installed **before** the process group is established, before the lock, and before a byte lands on disk.
- **The refusal is `UnsupportedPlatform`, not a new variant.** `tokio::signal::unix::signal` errors only for signals the kernel does not let a process catch, so a build where this fails on SIGTERM is a platform that cannot support driving at all — which is what that variant already says (D-05). A variant of its own would widen an enum that 17-07 also matches on, to describe a branch no supported platform reaches.
- **The terminal label is the outcome `cancel` returned, not a hard-coded `Killed`.** The coordinator maps a cancelled run onto `RunOutcome::Killed` at `claude.rs:1223`, so this reports `killed` by construction — and in the rare pass where a wall-clock or idle breach was classified alongside the cancel it reports the breach truthfully instead of overwriting it with a label that is merely expected. `the_outcome_label_for_a_stopped_run_is_killed` pins the mapping.
- **The reap arm lives in a separate set, not on `ObservedRun`.** The plan offered either. `observed_runs` is *replaced wholesale* by every reconciliation scan — that is deliberate, and it means any flag stored there survives at most five seconds before reverting to whatever the disk implies. The disk cannot imply this: `run.json` records the driver's own pid and never who its parent was. `AppContext.session_spawned_runs` is the only place the answer can live.
- **Group survivors are found by scanning `/proc` for a matching `pgrp`, not by a group signal probe.** A zero-signal probe reports a zombie as alive, and a surviving zombie is one of the four things the criterion forbids — so a probe that cannot see one is useless here. The scan also names *which* pids survived and in what state, which a yes/no cannot.
- **The `/proc/<pid>/stat` parse skips past the LAST `)`**, matching `liveness::process_state` rather than `session_detector::read_start_time`'s first-`)` form. This helper is pointed at process names it does not choose.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's task split does not compile**

- **Found during:** Task 1
- **Issue:** Task 1's action adds `#[cfg(unix)] pub mod kill;` to `src/driver/mod.rs` while Task 2 creates `src/driver/kill.rs`. A module declaration without its file is a hard compile error, so Task 1's commit could not have built — and every task commit in this phase is required to be independently green.
- **Fix:** The whole `src/driver/mod.rs` edit — the declaration *and* the fifth numbered doc fact that links to `[`kill`]` — moved into Task 2's commit, alongside the file it describes. Task 1 commits `src/driver/run.rs` only. Splitting them differently was considered and rejected: leaving the doc in Task 1 would have put a sentence claiming `kill` "landed in 17-06" two lines above nothing, which is the stale-claim class waves 1, 2 and 4 each had to retire.
- **Files modified:** `src/driver/mod.rs` (in `65f198c` rather than `0079cdb`)
- **Verification:** `cargo build` clean after each of the three commits.
- **Committed in:** `65f198c`

**2. [Rule 1 - Bug] The spawn-seam guard reports a group signal as a process spawn**

- **Found during:** Task 2
- **Issue:** `SPAWN_MARKERS` contains `process_group(` and the guard matched it with a plain `contains`. `rustix::process::kill_process_group(` contains that substring, so `src/driver/kill.rs` — which spawns nothing in production and only *signals* two groups that already exist — was reported as an unallowlisted process-spawn site and the guard went red. The path of least resistance is to add the file to the allowlist and move on, and that is the trap: it converts an audit of *"every spawn site takes a capability type"* into a list of files somebody once had to add, silently, without anyone noticing the meaning changed. `test_kill_process_group` and any future `…_process_group` helper are the same case.
- **Fix:** `calls_marker` requires a left word boundary — the character before the match must not continue an identifier — so `.process_group(` and `::process_group(` still match and `kill_process_group(` does not. The right side needs no boundary because every marker ends in `(`.
- **Files modified:** `tests/spawn_seam_guard.rs`
- **Verification:** `a_signal_to_a_process_group_is_not_mistaken_for_a_spawn` asserts both directions — the two `…_kill_process_group(` forms are rejected **and** the three genuine spawn forms are still found, because a boundary check that rejected everything would pass the first loop while disabling the audit outright.
- **Committed in:** `65f198c`

**3. [Rule 3 - Blocking] `src/driver/kill.rs` needed a spawn-allowlist entry**

- **Found during:** Task 2
- **Issue:** Independently of deviation 2, `stopping_a_pid_that_is_already_gone_reports_already_gone` spawns a real `sleep` child so it can reap it and hand `stop_run` a pid that has genuinely been recycled out of existence — which puts `Command::new(` in the module and makes the guard's set-equality assertion fail. The plan anticipated this ("if it does, the entry must be added deliberately with its comment").
- **Fix:** The entry landed in the same commit as the code that created the spawn site, with a comment saying it is an in-source test helper and that the module's production surface signals two existing groups and spawns nothing. Exactly the `src/driver/liveness.rs` precedent from wave 4.
- **Files modified:** `tests/spawn_seam_guard.rs`
- **Verification:** all five guards pass after the Task 2 commit.
- **Committed in:** `65f198c`

**4. [Rule 3 - Blocking] The `session_spawned_runs` field broke a struct literal the plan did not name**

- **Found during:** Task 2
- **Issue:** Adding the field to `AppContext` broke `src/ui/screens/detail.rs:4949`, a test helper that builds an exhaustive literal. `detail.rs` is not in this plan's `files_modified`. This is the third consecutive wave to hit the same literal.
- **Fix:** One explicit `session_spawned_runs: std::collections::HashSet::new()` line — never a struct-update shorthand, which would absorb the next field silently and destroy the compile-time proof the breakage exists to provide.
- **Files modified:** `src/ui/screens/detail.rs`
- **Verification:** `cargo build` clean; full suite green.
- **Committed in:** `65f198c`

**5. [Rule 1 - Bug] A real startup race in the new test, exposed by the plant**

- **Found during:** Task 3, while red-checking against the planted layer-2 removal
- **Issue:** `a_stopped_run_leaves_a_terminal_record_and_a_released_lock` read `run.json` immediately after `live_within` returned. But `liveness::is_run_alive` answers off `/proc/<pid>/cmdline`, which carries `--run-id` from the instant of `exec` — so a driver reports **live** while it is still establishing its process group, taking its lock and starting its journal, all of which precede the first `run.json` write. Under the plant's altered timing the read hit `NotFound`, and the test would have failed intermittently under load for a reason that has nothing to do with what it tests. Note this also weakens the assumption for any future test: "live" is not "recorded".
- **Fix:** A `recorded_within` helper polls for the record, and the two tests that read disk after startup gate on it. Its doc states the gap explicitly so the next test author does not re-derive it from a flake.
- **Files modified:** `tests/driver_kill.rs`
- **Verification:** with the fix in place the plant was re-applied and the same test failed on the property it exists to check — *"a stopped run must carry an ended_at"* — rather than on the race. Plant then reverted.
- **Committed in:** `7b0ab59`

**6. [Rule 1 - Bug] A `kill -0` literal in a panic message defeats the plan's own negative grep**

- **Found during:** Task 3 verification
- **Issue:** The acceptance criterion requires the test body to contain no `kill -0` **and** a comment naming the reason it is not used. The banner comments are filtered, but the zombie assertion's failure *message* also spelled the literal — so the comment-filtered count was 1, not 0, on a line that invokes nothing.
- **Fix:** The message now says "the zero-signal helper" instead. The property the criterion reaches for is unchanged and now mechanically checkable rather than requiring a judgement call about which occurrences are invocations.
- **Files modified:** `tests/driver_kill.rs`
- **Verification:** `grep -v '^[[:space:]]*//' tests/driver_kill.rs | grep -c 'kill -0'` outputs `0`.
- **Committed in:** `7b0ab59`

**7. [Rule 2 - Missing Critical] Four tests beyond the plan's named list**

- **Found during:** Tasks 1, 2 and 3
- **Issue:** Four gaps where a plausible wrong implementation passes every named test. Nothing exercised the zero-pgid refusal — the plan's single most dangerous named threat (T-17-37) — and reading the branch is not a test. Nothing distinguished the two `ReapArm` values, so an implementation that ignored the arm entirely would pass. Nothing asserted that a stop is dispatched off the render thread and returns as an `Action`, which is TRANS-03's whole content. And the diagnostic code that makes a stop traceable in the journal was unpinned.
- **Fix:** `a_zero_process_group_is_refused_rather_than_signalled`, `both_reaping_arms_name_who_performs_the_wait`, `a_stop_returns_its_outcome_as_an_action_rather_than_blocking`, `stopping_an_alias_with_no_observed_run_is_refused_visibly`, `a_run_this_session_did_not_spawn_is_never_recorded_as_our_child`, `the_terminate_diagnostic_code_is_a_stable_grep_target`, and `a_signal_to_a_process_group_is_not_mistaken_for_a_spawn`.
- **Files modified:** `src/driver/kill.rs`, `src/driver/run.rs`, `src/app.rs`, `tests/spawn_seam_guard.rs`
- **Verification:** all pass; suite total 508.
- **Committed in:** `0079cdb`, `65f198c`, `7b0ab59`

**8. [Rule 1 - Bug] `DRIVER_TEARDOWN_GRACE`'s observable neighbour was dead code in a non-test build**

- **Found during:** Task 2
- **Issue:** `CLAUDE_GROUP_GRACE` — the mirror of the executor's private ten seconds — was written at module scope beside the constant whose bound it justifies, and is read only by a test. `cargo build` warned `constant is never used`, which would have grown the warning count.
- **Fix:** Moved into the `#[cfg(test)] mod tests` block with its doc intact, alongside `CRITERION_VERIFICATION_POINT`. That is also where `tests/executor_lifecycle.rs:43-46` keeps its own mirror, so the convention is followed rather than diverged from; `DRIVER_TEARDOWN_GRACE`'s doc still states both bounds in prose.
- **Files modified:** `src/driver/kill.rs`
- **Verification:** `cargo build` clean; `cargo clippy --all-targets` warning count still exactly 5.
- **Committed in:** `65f198c`

---

**Total deviations:** 8 (2 blocking compile/guard breakages, 1 audit weakness that would have silently changed what the guard means, 1 real test race, 1 self-defeating negative grep, 1 dead-code warning, 1 unbuildable task split, 1 coverage addition). **Impact:** deviations 2 and 5 are the substantive ones — without 2 the spawn-seam audit would have quietly gained a member that spawns nothing, and without 5 the terminal-record test would have failed intermittently for a reason unrelated to what it checks. No scope creep: the public surface is the plan's symbol list plus `AppContext.session_spawned_runs` and nothing else. No dependency added.

## Issues Encountered

**`process-wrap`'s `KillOnDrop` does not save the tree, and the plant is what settled it.** Before planting, it was an open question whether removing layer 2 would actually orphan `claude` — the executor wraps its child with `KillOnDrop` as a backstop, and if the driver's runtime were dropped on the way out that backstop would fire and the test would pass against a broken implementation. Modelling the failure faithfully answered it: `std::process::exit(143)` runs no destructors, exactly as a default-disposition SIGTERM does not, and the agent's group survived with two members. An approximation that returned cleanly from the arm would have exercised the drop path and proved the opposite of what was intended.

**"Live" is not "recorded", and the difference is a real window.** `is_run_alive` is true from `exec` onward because it matches on the cmdline, while `run.json` is not written until after the process group is established and the lock is taken. Recorded as deviation 5 and worth carrying forward: any future test that waits for liveness and then reads the run directory is racing the driver's own startup.

**`pgrep -f <pattern>` matches the shell that is running it.** The acceptance criterion `pgrep -f fake-claude-spawner` returns nothing after the suite kept reporting one hit, with a *different* pid each time — the wrapping shell's own command line contains the pattern. Evaluated correctly as `pgrep -af 'fake-claude-spawn[e]r'`, which cannot match itself. A criterion of this shape passes vacuously in the other direction too, so it is worth stating: a self-matching `pgrep` reports a leak that does not exist and would have been "fixed" by weakening something real.

**`rtk proxy sh -c` for every pipeline, as all five previous waves recorded.** Every criterion here that pipes one filter into another was run as `rtk proxy sh -c '<whole pipeline>'`, and every cargo invocation needing raw `warning:`/`test result:` lines as `rtk proxy cargo …`. No criterion in this plan passed vacuously.

**Wave 2's `tokio::time::timeout` carry-forward did not bite, and the absence is a choice.** `the_adopted_arm_confirms_death_by_probing_proc_rather_than_by_wait` bounds `stop_run` with a plain inline `timeout` and no task boundary. That is safe here precisely because the thing being bounded is genuinely async — `stop_run` yields at a `tokio::time::sleep` between every `/proc` read — rather than a blocking syscall, which is what parked the runtime in 17-02. The distinction is recorded in a comment on the test so nobody "hardens" it into the shape wave 2 needed for a different reason.

**The environment's Bash guard refuses compound commands from a worktree agent**, so several verification steps were issued as separate plain commands. No workaround was needed for anything the tests do.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **508 passed, 0 failed** (baseline 494; +7 unit, +4 app unit, +3 integration) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/driver/`, `src/app.rs`, `src/action.rs`, `src/error.rs` or `src/ui/screens/` |
| `rtk proxy cargo test --lib driver::kill` | 5 passed — all three named tests present |
| `rtk proxy cargo test --lib driver::run` | 2 passed — the named test present |
| `rtk proxy cargo test --lib app::` | 20 passed — all 16 pre-existing plus 4 new |
| `rtk proxy cargo test --test driver_kill` | 3 passed in 15.16s — all three named tests present |
| `rtk proxy cargo test --test spawn_seam_guard` | 5 passed |
| `rtk proxy cargo test --test driver_tracer` | 4 passed, unchanged |
| `rtk proxy cargo test --test driver_lock` | 4 passed, unchanged |
| `grep -c 'SignalKind::terminate'` in `run.rs`, comments filtered | `1` |
| `grep -c 'biased'` in `run.rs`, comments filtered | `1`, and the signal arm is the first arm |
| `grep -c 'cancel('` in `run.rs`, comments filtered | `1` |
| `grep -c 'start_kill\|SIGKILL\|signal(15)\|kill_process_group'` in `run.rs`, comments filtered | `0` |
| `grep -n 'cancel(\|finish('` in `run.rs` | cancel at **180**, terminate-path finish at **196** — the journal closes after the agent is gone |
| `grep -c 'kill_process_group'` in `kill.rs`, comments filtered | `1` |
| `grep -c 'from_raw'` in `kill.rs`, comments filtered | `1`, and the `None` branch returns an error |
| `grep -c 'ReapArm'` in `kill.rs` | `7` |
| `grep -c 'stop_run'` in `app.rs`, comments filtered | `1`, at line 914, inside the `tokio::spawn` at 912 |
| `grep -rn '"/bin/kill"\|Command::new("kill")\|kill -' src/`, comments filtered | `0` (raw count 2, both wave-4 doc lines in `liveness.rs` explaining why `kill -0` is unusable) |
| `grep -c 'kill -0'` in `tests/driver_kill.rs`, comments filtered | `0` |
| `grep -c 'is_zombie'` in `tests/driver_kill.rs` | `2` |
| `pgrep -af 'fake-claude-spawn[e]r'` after the suite | **NONE** — no leaked stand-in; no leaked `sleep 600` grandchild |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no dependency added |

### The plant, verbatim

`std::process::exit(143)` substituted for the `shutdown_on_terminate` call — a faithful model of a default-disposition SIGTERM, because it runs no destructors:

```
---- stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie stdout ----
the agent's process group 4102479 still has members: [(4102479, 'S'), (4102481, 'S')].
This is the failure D-06 exists to prevent — the driver was stopped and `claude` was
not, because they are two different process groups and a signal to one does not reach
the other
```

```
---- a_stopped_run_leaves_a_terminal_record_and_a_released_lock stdout ----
a stopped run must carry an ended_at — without it the stop is indistinguishable on
disk from a crash (D-06.3, D-12)
```

Two members: the `sh` stand-in and its `sleep 600` grandchild. Both plants were reverted and `git status` confirmed `src/driver/run.rs` byte-identical to its commit before proceeding; the orphans the plant created were cleaned up by hand and a clean re-run leaks nothing.

## Known Stubs

None. `Action::DriverStopRequested` has no production emitter yet — the key binding that sends it is plan 17-07's, and `src/app.rs` handles the variant by calling `stop_driver_run`, so the path is complete and reachable rather than stubbed. `StopOutcome::SignalFailed` is the only variant with no dedicated test; it is an errno passthrough with no behaviour of its own beyond carrying an error kind, and both of its construction sites are on the same two lines as the calls that can produce it.

## User Setup Required

None — no external service configuration required. Every test runs against checked-in shell fixtures and temporary directories with no subscription, network or quota dependency.

## Next Phase Readiness

- **17-07 (UI)** — `App::stop_driver_run` is the seam a key binding calls, and `Action::DriverStopRequested { alias }` is the message to send; the handler already routes one to the other. The stop refuses visibly through `ctx.error_message` when no run is observed, and reports its result as a status message built from `StopOutcome`'s `Display`. Two constraints for the binding: it must carry **only the alias**, because the pid and pgid are read at dispatch and a value captured at key-press time may already name a different run's group; and any status text must say **observed**, never "reattached" (D-11).
- **Phase 18 (driver tab, orphan sweep)** — `StopOutcome::ExitedAfterKill` is the signal the sweep exists for: an escalated stop means the driver skipped its own layer 2, so its `claude` group may be orphaned. The journaled `claude_pgid` on `exec_started` (D-09) is the handle, and `tests/driver_kill.rs::group_members` is a working `/proc` group scan the sweep can reuse — including its comm-field-safe `stat` parse.
- **Phase 20 (router)** — unaffected. The terminate arm sits in the drain loop and returns from `execute_run`; a multi-command sequence wraps the loop rather than changing it, and `shutdown_on_terminate` is already the single place a stop closes a run out.

One forward note, and it is the one the plan rated "costly": `DRIVER_TEARDOWN_GRACE` is wedged between the executor's ten-second group grace below and criterion #1's fifteen seconds above. Both bounds are asserted rather than commented, so moving either neighbour fails a unit test naming the reason instead of silently making a stop escalate mid-teardown. The assessment stands.

## Requirements Traceability

`CTRL-01` is declared by this plan alone in its frontmatter, and every clause of it is proved above — two-layer teardown, both reaping arms, `rustix` signalling, the terminal record, and criterion #1 at fifteen seconds including the grandchild and the zombie. `REQUIREMENTS.md` is nonetheless **deliberately untouched**, consistently with 17-01 through 17-05: this executor runs in a worktree and the orchestrator owns all post-wave shared-file writes. The TUI-side key binding that makes the stop reachable by a user is 17-07's, which is not yet written.

## Self-Check: PASSED

- Both created files verified present on disk: `src/driver/kill.rs`, `tests/driver_kill.rs`.
- All three commits verified present in `git log`: `0079cdb`, `65f198c`, `7b0ab59`.
- All Task 1, 2 and 3 acceptance criteria re-run after the final task commit; all pass, with the two unsatisfiable grep criteria evaluated in their corrected form and both corrections documented above.
- Plan-level verification re-run: build clean, **508 tests passing** (baseline 494), zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml` and `Cargo.lock` unchanged.
- Both plants verified reverted: `grep -c '_planted' src/driver/run.rs tests/driver_kill.rs` is `0` for both, `git status` reported `src/driver/run.rs` clean against its commit, and the full suite was re-run green afterwards.
- Fixture-leak check re-run after a clean suite: `pgrep -af 'fake-claude-spawn[e]r'` returns nothing.
- No modification to `STATE.md` or `ROADMAP.md` — the orchestrator owns those writes.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
