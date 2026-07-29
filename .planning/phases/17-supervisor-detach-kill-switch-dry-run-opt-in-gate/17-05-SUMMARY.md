---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 05
subsystem: infra
tags: [driver, detach, process-group, kill-on-drop, proc, liveness, reconciliation, concurrency-cap, ctrl-04]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 01
    provides: "the `drive` CLI surface, DriveArgs, the opt-in gate in the child, the two hidden development flags, tests/spawn_seam_guard.rs and its allowlist"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 03
    provides: "Preferences.driver_max_concurrent (default 1), RegisteredProject.extra, schema v2"
  - phase: 16-run-journal-state-substrate
    provides: "RunRecord and its `ended_at is None` crash contract, run_paths, runs_root, read_active_run, create_run_dir, write_run_record, write_active_pointer, new_run_id, argv_digest, reader::tail_lines and parse_line"
  - phase: 15-transport-foundation
    provides: "DrivableProject, the fake-claude-slow.sh paced stand-in"
provides:
  - "`src/driver/liveness.rs`: is_run_alive (pid AND cmdline double-check), process_state, is_zombie — the zombie read tests/executor_lifecycle.rs's `kill -0` helper cannot perform"
  - "`src/driver/reconcile.rs`: ObservedRun, RunVerdict, reconcile_one, reconcile_all — a scan that performs zero disk writes"
  - "`src/driver/spawn.rs`: drive_argv (pure), spawn_detached (own process group, three null stdio, kill_on_drop(false), reaping task), admit + ConcurrencyRefusal"
  - "`Action::RunsReconciled` — the whole scan result, never a delta"
  - "`AppContext.observed_runs` — the per-alias sibling map"
  - "`App::start_driver_run` — the TUI's spawn seam, with the D-18 admission check and deliberately no second opt-in check"
  - "The startup reconciliation scan in main.rs and the tick probe on the EXISTING 20-tick block"
  - "tests/driver_reattach.rs — ROADMAP success criterion #2"
affects: [17-06 kill switch, 17-07 UI, 18 driver tab, 20 global cap policy]

tech-stack:
  added: []
  patterns:
    - "Detached spawn: tokio::process::Command + as_std_mut().process_group(0) + three separate Stdio::null() lines + explicit kill_on_drop(false) + a detached reaping task"
    - "In-module source guard via include_str! with runtime-assembled search literals, so the guard cannot match its own table"
    - "Test fixtures routed through the production writers so a negative grep over the whole file stays literally satisfiable"
    - "Equality-guarded redraw on a result that lands every five seconds forever"

key-files:
  created:
    - src/driver/liveness.rs
    - src/driver/reconcile.rs
    - src/driver/spawn.rs
    - tests/driver_reattach.rs
  modified:
    - src/driver/mod.rs
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "process_state finds the LAST ')' in /proc/<pid>/stat, not the first as session_detector::read_start_time does: the comm field is unescaped and may itself contain parentheses, and this module is pointed at process names it does not choose"
  - "reconcile.rs's own test fixtures are built through writer::create_run_dir / write_run_record / write_active_pointer rather than hand-rolled fs writes, which is both more faithful and what keeps the file's zero-write negative grep literally satisfiable over the whole file including tests"
  - "src/driver/liveness.rs needed a SPAWN_ALLOWLIST entry of its own — its pid-reuse test spawns a real `sleep` child — and it landed in the same commit as the module, so the guard never went red"
  - "A crashed run does not consume a concurrency slot: counting it would leave a project permanently unstartable after one crash with no way out but editing config.json"
  - "tests/driver_reattach.rs reproduces spawn_detached's configuration rather than calling it, because spawn_detached spawns std::env::current_exe() and inside an integration test that resolves to the test harness. The reproduction's fidelity is pinned by an assertion over src/driver/spawn.rs's own source"

patterns-established:
  - "A negative grep whose scope includes a test module constrains the test module too — route fixtures through production APIs rather than exempting the tests"
  - "A guard and a discriminating test are both planted-and-observed-red before they are trusted green"

requirements-completed: []

coverage:
  - id: A1
    description: "Liveness is a pid AND cmdline double-check: a live pid whose /proc cmdline does not carry the matching --run-id is reported dead, proved against a real spawned process"
    requirement: "CTRL-04"
    verification:
      - kind: unit
        ref: "src/driver/liveness.rs#a_live_pid_whose_cmdline_lacks_the_run_id_is_reported_dead"
        status: pass
      - kind: unit
        ref: "src/driver/liveness.rs#a_pid_that_does_not_exist_is_reported_dead"
        status: pass
    human_judgment: false
  - id: A2
    description: "A zombie-state read exists, because the repository's existing alive() helper uses kill -0 and reports a zombie as alive"
    verification:
      - kind: unit
        ref: "src/driver/liveness.rs#the_process_state_of_a_running_child_is_not_the_zombie_state"
        status: pass
      - kind: unit
        ref: "src/driver/liveness.rs#this_process_reports_its_own_state_as_running_or_sleeping"
        status: pass
    human_judgment: false
  - id: A3
    description: "Crash reconciliation performs zero disk writes — no run.json repair, no cleared active pointer, no prune — enforced by an in-module source guard, a shell-checkable negative grep, and a before/after fingerprint of the whole .planning tree"
    verification:
      - kind: unit
        ref: "src/driver/reconcile.rs#the_reconcile_module_contains_no_write_call"
        status: pass
      - kind: unit
        ref: "src/driver/reconcile.rs#the_scan_leaves_the_planning_tree_byte_identical"
        status: pass
      - kind: unit
        ref: "src/driver/reconcile.rs#a_stale_active_pointer_is_not_cleared_by_the_scan"
        status: pass
      - kind: integration
        ref: "tests/driver_reattach.rs#a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired"
        status: pass
      - kind: manual_procedural
        ref: "planted `std::fs::write` in reconcile.rs; the guard failed naming the offending line and verb, then the plant was reverted"
        status: pass
    human_judgment: false
  - id: A4
    description: "A run with an ended_at is not reported live; a run without one and with a dead pid is reported crashed; an absent or unreadable record is never reported as a crash"
    verification:
      - kind: unit
        ref: "src/driver/reconcile.rs#a_run_with_an_end_timestamp_is_not_reported_live"
        status: pass
      - kind: unit
        ref: "src/driver/reconcile.rs#a_run_without_an_end_timestamp_and_a_dead_pid_is_reported_crashed"
        status: pass
      - kind: unit
        ref: "src/driver/reconcile.rs#an_unreadable_record_is_not_reported_as_a_crash"
        status: pass
    human_judgment: false
  - id: A5
    description: "kill_on_drop is explicitly false at the detached spawn, the process-wrap kill-on-drop backstop is absent, and all three stdio handles are null"
    requirement: "CTRL-04"
    verification:
      - kind: manual_procedural
        ref: "comment-filtered greps over src/driver/spawn.rs: kill_on_drop(false)=1, kill_on_drop(true)|KillOnDrop=0, Stdio::null()=3, process_group(0)=1"
        status: pass
      - kind: integration
        ref: "tests/driver_reattach.rs#a_run_outlives_the_process_that_spawned_it"
        status: pass
      - kind: manual_procedural
        ref: "planted kill_on_drop(true) in the test's spawn; the post-drop liveness assertion failed, then the plant was reverted"
        status: pass
    human_judgment: false
  - id: A6
    description: "The TUI's drive argv provably carries no development flag, so a spawned driver can never be pointed at a fixture"
    verification:
      - kind: unit
        ref: "src/driver/spawn.rs#the_drive_argv_carries_no_development_flag"
        status: pass
      - kind: unit
        ref: "src/driver/spawn.rs#the_drive_argv_omits_the_goal_flag_entirely_when_there_is_no_goal"
        status: pass
    human_judgment: false
  - id: A7
    description: "A spawn is refused when the live-run count across every registered project already meets driver_max_concurrent, checked by a pure function and surfaced through ctx.error_message"
    verification:
      - kind: unit
        ref: "src/driver/spawn.rs#admit_refuses_when_the_live_count_already_meets_the_cap"
        status: pass
      - kind: unit
        ref: "src/driver/spawn.rs#admit_allows_the_first_run_under_the_default_cap_of_one"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_driver_run_is_refused_when_the_live_count_already_meets_the_cap"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_crashed_run_does_not_consume_a_concurrency_slot"
        status: pass
    human_judgment: false
  - id: A8
    description: "The scan attaches to the existing startup scan and the existing 20-tick block, with no second timer; its result replaces rather than merges the observed map and is equality-guarded for redraw"
    verification:
      - kind: unit
        ref: "src/app.rs#a_reconciliation_result_replaces_the_observed_map_rather_than_merging_it"
        status: pass
      - kind: unit
        ref: "src/app.rs#an_unchanged_reconciliation_result_does_not_request_a_redraw"
        status: pass
      - kind: manual_procedural
        ref: "grep -c '_poll_counter|_tick_counter|interval(' src/app.rs is 5, unchanged, and all five are session_poll_counter; the dispatch sits inside the existing >= 20 block"
        status: pass
    human_judgment: false
  - id: A9
    description: "ROADMAP success criterion #2: a run survives the process that spawned it, and a fresh Config that knows nothing but the registry finds it live with its goal and command off run.json and its current step off the journal tail"
    requirement: "CTRL-04"
    verification:
      - kind: integration
        ref: "tests/driver_reattach.rs#a_run_outlives_the_process_that_spawned_it"
        status: pass
      - kind: integration
        ref: "tests/driver_reattach.rs#a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step"
        status: pass
    human_judgment: false

duration: 26 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 05: Detached Spawn and Read-Only Reattachment Summary

**A driver spawned into its own process group with null stdio keeps running after its parent handle is dropped — proved by dropping the handle and watching the run continue — and a fresh `Config` that knows nothing but the registry finds it again off disk, live, with its current step read from the journal and not from a stream that no longer exists.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-07-29T17:50:00Z
- **Completed:** 2026-07-29T18:16:00Z
- **Tasks:** 3
- **Files modified:** 11 (4 created, 7 modified)

## Accomplishments

- **ROADMAP success criterion #2 is met and the load-bearing half is proved by the one event that would break it.** `a_run_outlives_the_process_that_spawned_it` spawns the real binary configured exactly as `spawn_detached` configures it, waits until the run is observably live, and then **drops the parent handle**. A `tokio::process::Child` with `kill_on_drop(true)` sends SIGKILL on that drop; with `false` it does not. A clean TUI shutdown drops precisely this handle, so an accidental `true` would kill the run at the exact moment the phase exists to survive. The plant was made and the test was observed failing on it before it was trusted passing.
- **Liveness is a pid *and* cmdline double-check, and the test that matters uses a real process.** `a_live_pid_whose_cmdline_lacks_the_run_id_is_reported_dead` spawns a genuine `sleep`, takes its genuine live pid, and asserts the probe says dead. Every other test in that module passes against an implementation that only checks existence; this one does not. Without it the phase would ship a kill switch that eventually SIGTERMs a stranger's process group on any host that has been up long enough to recycle a pid.
- **"Zero disk writes" is enforced three ways and was observed detecting a real write.** An in-module `include_str!` guard, a shell-checkable negative grep, and a before/after fingerprint of the whole `.planning` tree — the last one both at unit scale over a fabricated crashed run and at integration scale over a genuinely SIGKILLed driver. A planted `std::fs::write` made the guard fail naming the line and the verb.
- **The `is_zombie` gap 17-06 needs is closed, and the reason it exists is written where it will be read.** `tests/executor_lifecycle.rs`'s `alive()` uses `kill -0`, which reports a zombie as **alive**, so criterion #1's *"no zombie behind"* half is untestable with it. `process_state` parses `/proc/<pid>/stat` past the parenthesised `comm` field using the **last** `)` rather than the first, which is the difference between correct-for-`claude` and correct-for-any-process-name.
- **The concurrency cap Phase 20 inherits is a working cap, not a preference with no enforcement.** `admit` is pure and tested, the count comes from the scan that already enumerates every project, and a crashed run deliberately does not consume a slot — counting it would leave a project permanently unstartable after one crash.
- **The spawn-seam guard did its job twice in this plan.** `src/driver/liveness.rs` and `src/driver/spawn.rs` each had to be added to the allowlist in the same commit as the code that made them spawn sites, which is the mechanism working exactly as designed.

## Task Commits

1. **Task 1: The `/proc` probe and a reconciliation scan that writes nothing** — `ca82772` (feat)
2. **Task 2: Spawn detached, admit against the cap, attach the scan to both existing points** — `b7220ea` (feat)
3. **Task 3: Prove a run outlives the process that started it, and is found again** — `aa3c487` (test)

## Files Created/Modified

**Created**

- `src/driver/liveness.rs` — `cmdline_args`, `is_run_alive`, `process_state`, `is_zombie`, and four unit tests. No `cfg` guard: the module carries `session_detector.rs`'s honest-failure posture rather than diverging from the module it copies.
- `src/driver/reconcile.rs` — `RunVerdict`, `ObservedRun` (+ `verdict()`), `classify`, `RunFacts`, `read_run_facts`, `reconcile_one`, `reconcile_all`, and six unit tests including the source guard. Module doc opens with the zero-write rule in bold and names each of the three destructive "repairs" it forbids.
- `src/driver/spawn.rs` — `drive_argv`, `spawn_detached`, `ConcurrencyRefusal`, `admit`, and four unit tests. `#[cfg(unix)]`.
- `tests/driver_reattach.rs` — the three criterion-#2 proofs plus `fingerprint_tree`, `process_group_of`, `kill_group`, and the poll helpers. Its banner states what it deliberately does not attempt and why.

**Modified**

- `src/driver/mod.rs` — three module declarations (`liveness` and `reconcile` portable, `spawn` Unix-gated), each with the reason for its placement, plus the stale "later plans add" sentence corrected.
- `src/action.rs` — `Action::RunsReconciled`, documented as carrying the whole scan and never a delta.
- `src/app.rs` — the `observed_runs` field in the exhaustive `AppContext` literal; the reconciliation dispatch inside the **existing** 20-tick block with a comment forbidding a second timer; the `RunsReconciled` arm; `App::start_driver_run`; and five new unit tests.
- `src/main.rs` — the synchronous startup scan, beside the existing session scan.
- `src/ui/screens/mod.rs` — the `observed_runs` sibling map with a four-bullet doc naming `ProjectState`, `Action`'s `Clone` derive, and D-11's "observed, not reattached".
- `src/ui/screens/detail.rs` — one test-helper `AppContext` literal the new field broke.
- `tests/spawn_seam_guard.rs` — two new allowlist entries, each in the commit that created its spawn site.

## Decisions Made

- **`process_state` uses `rfind(')')`, not `find(')')`.** `session_detector::read_start_time` uses the first `)` and has been correct in practice only because `claude` has no parenthesis in its name. `/proc/<pid>/stat`'s `comm` field is unescaped and may contain both spaces and parentheses, and this module is pointed at process names it does not choose. The divergence is documented at the function so the two are not "harmonised" back later.
- **`reconcile.rs`'s test fixtures go through the production writers.** `writer::create_run_dir`, `write_run_record` and `write_active_pointer` build the crashed run rather than hand-rolled `fs::write` calls. This is more faithful — the fixture is the real layout — and it is what keeps the zero-write negative grep satisfiable over the **whole file**, tests included. A guard whose scope stops at the test module is a guard with a documented hole.
- **The in-module guard's verb table is assembled at runtime from halves.** Spelled out whole, the table would be a non-comment line containing every string the guard searches for, and the guard would report itself. This is wave 1's technique from `tests/spawn_seam_guard.rs`, reused rather than reinvented.
- **A crashed run does not count against `driver_max_concurrent`.** It is still surfaced — a run that died without an ending is the thing a user most needs to be told about — but it burns no quota. Counting it would make one crash permanently unstartable with no remedy but editing `config.json`.
- **`start_driver_run` is `#[cfg(unix)]` rather than returning a platform error.** The TUI stays cross-platform and simply has no way to start a run off Unix, which is what D-05 says. The typed platform refusal already exists one layer down, at `driver::drive`, where a hand-typed invocation reaches it.
- **The optimistic `ObservedRun` inserted on a successful spawn is deliberate and self-correcting.** Without it the dashboard would wait up to five seconds for the first scan; the next scan replaces the map wholesale from disk, which is what corrects the entry if the driver refused at its own gate and exited immediately.
- **`tests/driver_reattach.rs` reproduces `spawn_detached`'s configuration rather than calling it**, because `spawn_detached` spawns `std::env::current_exe()` and inside an integration test that resolves to the test harness, not `gsd-meta-manager`. The reproduction's fidelity is not left as a comment: the test reads `src/driver/spawn.rs` and asserts the four properties it claims to mirror.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `src/driver/liveness.rs` needed a spawn-allowlist entry the plan did not anticipate**

- **Found during:** Task 1
- **Issue:** The plan assigns the `tests/spawn_seam_guard.rs` edit to Task 2 and names only `src/driver/spawn.rs`. But `a_live_pid_whose_cmdline_lacks_the_run_id_is_reported_dead` must spawn a real child — the plan's own acceptance criterion requires it — so `liveness.rs` contains `Command::new(`, which is one of the guard's three spawn markers. The guard asserts set *equality*, so it would have gone red on Task 1's commit.
- **Fix:** The allowlist entry landed in Task 1's commit, with a comment saying it is an in-source test helper and that nothing in the module's production surface spawns anything. `src/driver/spawn.rs`'s entry landed in Task 2's, as planned.
- **Files modified:** `tests/spawn_seam_guard.rs`
- **Verification:** `cargo test --test spawn_seam_guard` passes after each of the three commits.
- **Committed in:** `ca82772`, `b7220ea`

**2. [Rule 3 - Blocking] The `observed_runs` field broke a struct literal the plan did not name**

- **Found during:** Task 2
- **Issue:** The plan names `src/app.rs`'s `AppContext` literal in `App::from_config`. Adding the field also broke `src/ui/screens/detail.rs:4949`, a test helper that builds an exhaustive `AppContext`. `detail.rs` is not in this plan's `files_modified`.
- **Fix:** One explicit `observed_runs: HashMap::new()` line — never a struct-update shorthand, which would absorb the next field silently and destroy the compile-time proof the breakage exists to provide. This is the same call the wave-3 executor made for `extra`.
- **Files modified:** `src/ui/screens/detail.rs`
- **Verification:** `cargo build` clean; the full suite green.
- **Committed in:** `b7220ea`

**3. [Rule 1 - Bug] The allowlist acceptance grep counts a pre-existing doc line**

- **Found during:** Task 2 verification
- **Issue:** `grep -c 'src/driver/spawn.rs' tests/spawn_seam_guard.rs | == 1` cannot hold. Plan 17-01 wrote a doc comment on the allowlist constant that names `src/driver/spawn.rs` in advance — *"adds `src/driver/spawn.rs`; whoever adds it must add the entry here in the same commit"* — so the raw count is 2 and one of the two is wave 1's foresight.
- **Fix:** No code change; the property the criterion reaches for holds. Verified with the comment-filtering form this phase already uses elsewhere: `grep -v '^[[:space:]]*//' tests/spawn_seam_guard.rs | grep -c 'src/driver/spawn.rs'` outputs `1`, the allowlist entry itself.
- **Files modified:** none
- **Verification:** the comment-filtered grep, plus `every_process_spawn_site_in_src_is_on_the_allowlist` passing with set equality.
- **Committed in:** n/a (verification-only)

**4. [Rule 2 - Missing Critical] The plan's `spawn_detached` cannot be called from an integration test**

- **Found during:** Task 3
- **Issue:** `spawn_detached` spawns `std::env::current_exe()`. Inside an integration test that is `target/debug/deps/driver_reattach-<hash>`, not `gsd-meta-manager`, so calling the production function from `tests/` would spawn the test harness with `drive` arguments it does not understand. The plan already anticipates using `std::process::Command` on `CARGO_BIN_EXE_gsd-meta-manager` — but a `std::process::Child` does not kill on drop under **any** configuration, so `drop(child)` on one would assert nothing at all and the plan's own note that *"the test looks trivial and is not"* would stop being true.
- **Fix:** The test builds a `tokio::process::Command` configured exactly as `spawn_detached` configures one — including `kill_on_drop(false)` — so the drop is genuinely the discriminating event. To stop that reproduction drifting from the thing it reproduces, the test also reads `src/driver/spawn.rs`, filters comments, and asserts the four properties it claims to mirror.
- **Files modified:** `tests/driver_reattach.rs`
- **Verification:** planted `kill_on_drop(true)` in the test's own spawn; `a_run_outlives_the_process_that_spawned_it` failed on the post-drop liveness assertion in 0.53s. Reverted, and `grep -c 'cmd.kill_on_drop' tests/driver_reattach.rs` confirms one occurrence, `false`.
- **Committed in:** `aa3c487`

**5. [Rule 2 - Missing Critical] Four tests beyond the plan's named list**

- **Found during:** Tasks 1, 2 and 3
- **Issue:** Four gaps where a plausible wrong implementation would pass every named test. `process_state` returning `None` or a digit — the classic comm-field bug — is invisible to the three liveness tests as specified, all of which only compare against `'Z'`. An absent `run.json` being reported as a crash would manufacture crash reports out of permission errors and nothing checked it. An empty `--goal` passed instead of an omitted flag would be recorded verbatim into `RunRecord.goal`. And an unconditional redraw on every scan would repaint twelve times a minute forever, which no equality assertion in the plan's list would have caught.
- **Fix:** `this_process_reports_its_own_state_as_running_or_sleeping`, `an_unreadable_record_is_not_reported_as_a_crash`, `the_drive_argv_omits_the_goal_flag_entirely_when_there_is_no_goal`, and `an_unchanged_reconciliation_result_does_not_request_a_redraw` — the last with a control arm asserting a real change still does request one.
- **Files modified:** `src/driver/liveness.rs`, `src/driver/reconcile.rs`, `src/driver/spawn.rs`, `src/app.rs`
- **Verification:** all pass; suite total 494.
- **Committed in:** `ca82772`, `b7220ea`

---

**Total deviations:** 5 (2 blocking compile/guard breakages the plan did not anticipate, 1 unsatisfiable plan criterion, 1 test that as specified could not detect what it exists to detect, 1 coverage addition). **Impact:** deviation 4 is the substantive one — without it the phase's headline property would have had a test that passes on an implementation that kills the run when the TUI closes. No scope creep: the public surface is the plan's symbol list plus `ObservedRun::verdict` and nothing else. No dependency added.

## Issues Encountered

**A negative grep whose scope includes a test module constrains the test module too.** The plan's zero-write criterion greps the *whole* of `src/driver/reconcile.rs`, while its named test `the_scan_leaves_the_planning_tree_byte_identical` requires a fabricated crashed run on disk — and fabricating one needs writes. Waves 3 and 4 each resolved a conflict of this shape by evaluating the criterion in a corrected form; here it was resolvable without weakening anything, by routing the fixtures through `journal::writer`'s production functions. That is worth carrying forward as the preferred resolution: a fixture built from production APIs is more faithful *and* keeps the guard's scope whole.

**`rtk proxy sh -c` for every pipeline, as all four previous waves recorded.** Every criterion here that pipes one filter into another was run as `rtk proxy sh -c '<whole pipeline>'`, and every cargo invocation needing raw `warning:`/`test result:` lines as `rtk proxy cargo …`. No criterion in this plan passed vacuously.

**Wave 2's carry-forward did not bite, because the shape it warns about was avoided.** 17-02 recorded that `tokio::time::timeout` cannot bound a synchronous syscall on the runtime it shares. This plan's process-lifecycle tests use plain `std::thread::sleep` poll loops with wall-clock deadlines rather than `timeout`, so there is no task-boundary hazard to get wrong. Recorded because the absence is a choice.

**The environment's Bash guard refuses compound commands from a worktree agent.** Several verification steps had to be issued as separate plain commands rather than as one `&&` chain. No workaround was needed for anything the tests do.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **494 passed, 0 failed** (baseline 472; +14 unit, +8 app/driver unit, +3 integration) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/driver/`, `src/app.rs`, `src/main.rs`, `src/action.rs` or `src/ui/screens/mod.rs` |
| `rtk proxy cargo test --lib driver::liveness` | 4 passed — all three named tests present |
| `rtk proxy cargo test --lib driver::reconcile` | 6 passed — all five named tests present |
| `rtk proxy cargo test --lib driver::spawn` | 4 passed — all three named tests present |
| `rtk proxy cargo test --lib app::` | 17 passed — all 12 pre-existing plus 5 new |
| `rtk proxy cargo test --test spawn_seam_guard` | 4 passed |
| `rtk proxy cargo test --test driver_reattach` | 3 passed in 6.10s — all three named tests present |
| `grep -c 'fs::write\|File::create\|OpenOptions\|remove_file\|remove_dir\|create_dir\|rename\|persist\|clear_active_pointer\|prune_runs'` in `reconcile.rs`, comments filtered | `0` |
| `grep -c '#\[cfg(unix)\]\|#\[cfg(target_os'` in `liveness.rs`, comments filtered | `0` |
| `grep -c 'is_zombie' src/driver/liveness.rs` | `5` |
| `grep -c 'kill_on_drop(false)'` in `spawn.rs`, comments filtered | `1` |
| `grep -c 'kill_on_drop(true)\|KillOnDrop'` in `spawn.rs`, comments filtered | `0` |
| `grep -c 'Stdio::null()'` in `spawn.rs`, comments filtered | `3` |
| `grep -c 'process_group(0)'` in `spawn.rs`, comments filtered | `1` |
| `grep -c 'src/driver/spawn.rs' tests/spawn_seam_guard.rs`, comments filtered | `1` |
| `grep -c '_poll_counter\|_tick_counter\|interval(' src/app.rs` | `5`, unchanged, and all five are `session_poll_counter` — no second timer |
| `grep -c 'observed_runs' src/ui/screens/mod.rs` | `1`; the field's doc contains `sibling map` and names `ProjectState` |
| `grep -c '#\[ignore\]' tests/driver_reattach.rs` | `1`, the banner sentence explaining why there is none |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no dependency added |

## Known Stubs

None. `RunVerdict::Ended` is returned only by the pure `classify` function and never travels in an `ObservedRun`, and that is stated at `ObservedRun::verdict` rather than left to be inferred: an ended run yields no observation because there is nothing left to observe. `App::start_driver_run` has no production caller yet — the key binding that calls it is plan 17-07's, and the plan says so — but the function is complete, tested, and reachable.

## User Setup Required

None — no external service configuration required. Every test runs against checked-in shell fixtures and temporary directories with no subscription, network or quota dependency.

## Next Phase Readiness

- **17-06 (kill switch)** — `liveness::is_zombie` exists and is the helper criterion #1's *"no zombie behind"* half needs; `tests/executor_lifecycle.rs`'s `alive()` cannot answer that question and must not be extended to try. `ObservedRun.pgid` is the group to signal, and D-07's second arm — a run the TUI adopted rather than spawned, where `wait()` returns `ECHILD` — is exactly what `reconcile_all` now produces. `tests/driver_reattach.rs::kill_group` is the shell-based group-signal idiom, and the reason it shells out rather than using `rustix` (a normal dependency an integration test cannot see) is recorded at the helper.
- **17-07 (UI)** — `AppContext.observed_runs` is populated at startup and every ~5s, and `App::start_driver_run` is the seam a key binding calls. It refuses visibly through `ctx.error_message` for both an unknown alias and a full concurrency cap, and it deliberately performs **no** opt-in check: the gate is in the child, which is what makes CTRL-03's "never" true for both entry points. Any status text must say **observed**, never "reattached" and never "streaming" (D-11).
- **Phase 18 (driver tab)** — the journal tail read `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` performs is exactly the "current step" read a rendered surface needs, and it goes through `reader::tail_lines` + `parse_line` with no new computation.
- **Phase 20 (router, global cap policy)** — `admit` is the working cap; the policy around the count (a queue, a park, a priority) is Phase 20's and none of it is here.

One forward note, and it is the same one the plan rated "costly": the `drive` argv `drive_argv` emits is a compatibility surface between a running TUI and a driver binary that may be a **different build** during an upgrade. `liveness::is_run_alive` matches on `--run-id` specifically, so changing that flag's name later means a window in which a TUI cannot recognise its own running drivers. No data moves; the assessment stands.

## Requirements Traceability

`CTRL-04` is declared by this plan **and** by 17-07, which is not yet written, so it is not marked complete here — marking it while a sibling that also claims it is unwritten is exactly the gap phase verification exists to catch. `REQUIREMENTS.md` is therefore deliberately untouched, consistently with 17-01 through 17-04. This executor runs in a worktree and the orchestrator owns all post-wave shared-file writes.

## Self-Check: PASSED

- All four created files verified present on disk: `src/driver/liveness.rs`, `src/driver/reconcile.rs`, `src/driver/spawn.rs`, `tests/driver_reattach.rs`.
- All three commits verified present in `git log`: `ca82772`, `b7220ea`, `aa3c487`.
- All Task 1, 2 and 3 acceptance criteria re-run after the final task commit; all pass, with the one unsatisfiable grep criterion evaluated in its corrected form and the correction documented above.
- Plan-level verification re-run: build clean, **494 tests passing** (baseline 472), zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml` and `Cargo.lock` unchanged.
- Both planted checks verified reverted: `grep -c '_planted' src/driver/reconcile.rs` is `0` and `grep -c 'cmd.kill_on_drop' tests/driver_reattach.rs` is `1` (`false`), and the full suite was re-run green afterwards.
- No modification to `STATE.md` or `ROADMAP.md` — the orchestrator owns those writes.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
