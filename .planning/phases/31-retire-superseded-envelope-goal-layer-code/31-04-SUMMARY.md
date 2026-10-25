---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 04
subsystem: retirement
tags: [rust, retirement, drive, goal-layer, inbox, journal, preferences, docs]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-01..31-03 removed the callers (router refusal, injection UI, legacy runs half of the TUI), so the drive core could go
provides:
  - no hidden `drive` subcommand; `driver/mod.rs` holds only direction, steer, router, rate_limit, liveness and (unix) kill/lock
  - no Phase 21 goal layer (goal, escalate, untrusted, bounds), no `DriveError`, no stream-json inbox core
  - a journal trimmed to what the envelope park path, the watcher, the registry and the Driver tab still read
  - a Preferences screen without the dead runtime/concurrency rows, and user docs that name only real surfaces
affects: [31-05, 31-06, 31-07]

actuals:
  tokens: 12770
  tasks: 3
  commits: 3

plan_head_before: baa7c8ababf6cb815c691fce67812f79d9223ac2
plan_head_after: 232decbf3a1d98c95102fd8e82f25cc04bf82c5d
commits: 3

tech-stack:
  added: []
  patterns:
    - "delete the entry point, let cargo check --all-targets name the next layer, then grep pub items by hand (pub lib items raise no dead_code)"
    - "a scripted item-remover that tracks bracket depth: a naive `;` end-of-item rule stops inside `[T; N]` and leaves the body behind"
    - "a retired field survives in a flattened `extra` map, so an old run.json still loads and round-trips (pinned by a byte-literal test)"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs
    - src/driver/mod.rs
    - src/driver/kill.rs
    - src/driver/lock.rs
    - src/driver/rate_limit.rs
    - src/driver/router.rs
    - src/driver/liveness.rs
    - src/error.rs
    - src/registry.rs
    - src/experimental.rs
    - src/executor/mod.rs
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/envelope/mod.rs
    - src/envelope/ledger.rs
    - src/envelope/advisory.rs
    - src/ui/screens/preferences.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/help.rs
    - src/ui/screens/mod.rs
    - tests/spawn_seam_guard.rs
    - tests/async_blocking_guard.rs
    - tests/journal_gitignore.rs
    - README.md
    - docs/CONFIGURATION.md
    - docs/GSD-CORE-SYNC.md
    - Cargo.toml
  deleted:
    - src/driver/run.rs
    - src/driver/spawn.rs
    - src/driver/dry_run.rs
    - src/driver/reconcile.rs
    - src/driver/goal.rs
    - src/driver/escalate.rs
    - src/driver/untrusted.rs
    - src/driver/bounds.rs
    - src/journal/inbox.rs
    - "19 tests/*.rs binaries, 6 fake-claude-*.sh fixtures, tests/fixtures/injection-corpus/"

key-decisions:
  - "The Preferences screen keeps its header, file path and GSD-template route, drops both editable rows, and says nothing manager-wide is editable; the config fields still parse and round-trip [inferred — audit]"
  - "RunRecord loses bounds, approved_plan, escalation_cap and escalations_used (types or goals that no longer exist); the other drive-era fields stay because 31-05 deletes RunRecord whole [inferred — audit]"
  - "driver_model_seam's argv tests are not moved: the seam-profile one already lives, retargeted to an inline schema, in spawn_seam_guard, and the executor-profile one is pinned by claude.rs's baseline test [inferred — audit]"
  - "The registry untrusted-field census keeps UnrecognizedRuntime as a named subject (its Display echoes a user-authored config value) and drops the four DriveError variants: 12 -> 9 subjects [inferred — audit]"
  - "The apple-target clippy break in envelope/ledger.rs (31-01's FIFO helper) is fixed with a Linux-only cfg on the FIFO rows rather than a new dependency [inferred — audit]"

patterns-established:
  - "a retired-field regression test feeds a byte literal of the old shape and asserts the field lands in `extra` and survives a rewrite"

requirements-completed: []  # RETIRE-01 and RETIRE-02 are shared with 31-01..31-03 and 31-05..31-07; the shared-ID gate (#2388) keeps them open until every declaring plan has a SUMMARY

coverage:
  - id: D1
    description: "The hidden drive subcommand and everything only it used are gone; launching, stopping and steering a launcher run and `next --json` still work with every census re-measured"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs (18 pass) and tests/async_blocking_guard.rs (9 pass)"
        status: pass
      - kind: integration
        ref: "tests/{next_cli,driver_router_conformance,driver_tab_launch_e2e,launcher_stop,driverctl_lock_race} pass; driver_router_conformance has no commit since 036426d"
        status: pass
    human_judgment: false
  - id: D2
    description: "The goal layer, DriveError, the inbox core, the approval-digest family and the legacy kill/lock entry points are gone; the router, signal_group and the acquire_at family stay and test green"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "rtk proxy cargo test --lib -- journal:: driver:: error:: registry:: launcher:: (500 pass)"
        status: pass
      - kind: unit
        ref: "src/driver/kill.rs#a_signal_to_a_process_group_reaches_a_member_of_it"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#a_record_carrying_the_retired_goal_layer_fields_still_loads_and_keeps_them"
        status: pass
    human_judgment: false
  - id: D3
    description: "Preferences, README, CONFIGURATION and GSD-CORE-SYNC describe only surfaces that exist, and old config.json files still load"
    requirement: RETIRE-02
    verification:
      - kind: unit
        ref: "src/ui/screens/preferences.rs#render_labels_both_scopes_and_offers_no_dead_rows"
        status: pass
      - kind: unit
        ref: "src/ui/screens/preferences.rs#old_preference_keys_still_load_from_a_config_file"
        status: pass
      - kind: other
        ref: "acceptance greps in Self-Check (all pass); clippy --locked --target x86_64-pc-windows-gnu, aarch64-apple-darwin, x86_64-apple-darwin"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 04: Retire the drive Subcommand, the Goal Layer and the Inbox Core Summary

**The hidden Phase 17 `drive` subcommand, its run/spawn/dry-run/reconcile code, the Phase 21 goal layer, `DriveError`, the stream-json inbox core and the legacy kill/lock entry points are deleted (about 39,000 lines, 19 test binaries); the Preferences screen and the user docs now describe only surfaces that exist.**

## Performance

- **Duration:** about 30 min (2026-10-25T05:44Z to 06:14Z)
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files:** 77 changed over the three code commits, +864 / -39,080 lines

## Phase baseline and post-plan suite

Before (31-03 SUMMARY, HEAD `baa7c8a`):

```
results=94 passed=4300 failed=1 ignored=30
```

After (FULLSUITE, HEAD `232decb`, raw log read, no pipe counting):

```
results=75 passed=3941 failed=1 ignored=17
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
expected results=75
```

`results=75` equals `expected results=75` (72 `tests/*.rs` plus lib, bin and doc-tests). The one failure is the known git-version witness, deleted by 31-05. `editor_race_smoke` passed. Passed delta against 31-03: **-359** (the plan estimated -330..-400). Ignored dropped 30 -> 17: thirteen `#[ignore]`d live-agent arms went with `driver_injection_corpus` and `driver_model_seam`. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` exit 0, and so does `cargo clippy --locked --target {x86_64-pc-windows-gnu, aarch64-apple-darwin, x86_64-apple-darwin} --all-targets -- -D warnings`.

## Census pins: old and new

| Pin | Old | New | Why |
|---|---|---|---|
| `spawn_seam_guard` tests | 41 | 18 | 23 tests whose subject died: drive args/raw-argv scanners, `CommandSource`, goal decomposition, terminal-write stamping, debug-only override fields, `NonBlank` census, model-seam and ambiguity-seam sites, arrival-evidence, untrusted field enumeration |
| `SPAWN_ALLOWLIST` | had `src/driver/spawn.rs` | entry removed | the `vanished` check named only that one; `kill.rs` still spawns in its own tests, so its entry stays |
| argv alias census (`ARGV_ALIAS_ENTRY_POINTS`) | 8 | 7 | `Commands::Drive` row removed; the planted-extra control, the exact-count test and the row-existence test all re-run green; `NonBlank` is no longer an accepted judge |
| `MIN_ASYNC_BODY_LINES` | 800 | **800 (unchanged)** | measured **2380** async body lines after the deletion, far above the floor, so there was nothing to lower; the stale "roughly 1700" sentence now says roughly 2400 and records 2380 |
| `REQUIRED_ASYNC_FILES` | `src/driver/{mod,run}.rs` | `src/main.rs`, `src/main_loop.rs`, `src/launcher/egress/mod.rs` | all three contain `async fn` bodies the walk finds (the plan's suggested `src/app.rs` has none) |
| `ASYNC_BLOCKING_ALLOWLIST` | 7 entries | 4 (`.status()` in main.rs, `.spawn()` in claude.rs, `.spawn()` in codex.rs, `RunSnapshot::capture(` in claude.rs) | the `preview_text(` entry and the two `run.rs` entries removed |
| `BLOCKING_HELPERS` | 17 markers | 7 | the stale-marker guard refused ten names that only the deleted callers carried |
| registry untrusted-field census subjects per corpus value | 12 | 9 | four `DriveError` variants -> `UnrecognizedRuntime` as one named subject |
| `ui::MEASURED_REACH`, `UI_SOURCE_FLOOR`, `driver::CENSUS_FILES` | unchanged | unchanged | this plan deletes no file under `src/ui/` and no needle call site |

## D-10 lock/kill exception: the kept tests

- **A launcher run lock is released when its holder dies:** `tests/launcher_e2e.rs::sigkill_then_relaunch_logs_crash_detected` SIGKILLs the launcher and asserts `probe_at(run_lock_path) == LockProbe::Free`, then relaunches and acquires the same lock. Contention is pinned by `tests/driverctl_lock_race.rs` (5 tests, including `real_holder_is_still_refused_within_budget`). The three holder-record unit tests in `lock.rs` were retargeted from the deleted `acquire`/`read_holder` onto `acquire_at`/`read_holder_at`.
- **A process group receives the signal:** no kept test did this, so one was added: `driver::kill::tests::a_signal_to_a_process_group_reaches_a_member_of_it` (an `sh` leads a fresh group, `signal_group(pgid, TERM)` kills it). The refusal half stays in `a_zero_process_group_is_refused_rather_than_signalled`. The launcher's `stop_group` is exercised through `launcher_stop` and `launcher_e2e`.

## Executor-direct tests found in the deleted binaries (held 31-07)

Because 31-07 is held, the executor code stays in the tree. Each deleted binary was checked for tests that touch it without `drive`:

- `driver_rate_limit`: three fixture-integrity tests (synthesised transcript parses through the shipped parser, matches the NDJSON format, README inventory agrees with disk) never entered through `drive`. **Moved** to `src/driver/rate_limit.rs` (22 tests there now).
- `driver_model_seam`: the seam-profile argv test is already present, retargeted to an inline schema instead of `goal::escalation_schema()`, as `spawn_seam_guard::the_seam_profile_couples_the_empty_tool_set_to_the_schema`; the executor-profile argv test duplicates `claude.rs::the_argv_baseline_carries_every_required_flag`; the MCP-flag test duplicates `the_permitted_mcp_set_stays_empty_on_both_profiles`. The schema-vs-router-alphabet test and the live arms depended on `goal.rs` and are gone.
- `driver_injection_corpus`, `envelope_wiring`: their `build_argv` uses are guard/corpus tests (31-05 territory) or live-agent arms; not moved.
- `driver_codex_runtime`, `driver_dry_run`, `driver_*`: every test enters through `drive()` or `dry_run`.

The fixtures `fake-codex.sh`, `tests/fixtures/codex/`, `fake-claude{,-deaf,-echo,-late-init,-orphan,-silent,-slow,-spawner}.sh` stay for 31-07.

## Journal items kept for 31-05

Kept because `src/envelope/{mod,hooks,policy}.rs` (the park path and its tests), `tests/journal_crash.rs`, `tests/journal_gitignore.rs` or another live module still reads them: `JournalWriter` and its `writer.rs` helpers (`create_run_dir`, `write_run_record`, `prune_runs`, the active pointer, `ensure_runs_root`, `RUNS_GITIGNORE_BODY`, `crash_test_writer_loop`), `run_paths` / `RunPaths` (now five paths, no `inbox`), `runs_root`, `RUNS_SUBDIR`, `RETAIN_RUNS` (used by `JournalRun::start`'s prune), `JournalEvent`, `RunRecord`, `JournalRun` (`start`, `record`, `finish`, `record_writes`, `paths`), `MAX_RUN_JOURNAL_BYTES`, `MAX_EVENT_PAYLOAD_BYTES`, `MAX_TAIL_BYTES`. Always kept and live: `is_plain_path_component`, `classify_change` + `ChangeKind`, `sha256_digest`, `argv_digest`, `redact`, `reader::{tail_file, TailCursor}`.

Deleted from the journal: the approval-digest family (`approval_digest`, `render/parse_approval_token`, `ApprovalTokenError`, `ApprovedPlan`, `ApprovalRefusal`, `recheck_approval`, `ABSENT_INPUT_DIGEST`, `APPROVAL_TOKEN_SEPARATOR`), `RunSummary`, `list_runs`, `last_ended_outcome`, `read_run_summary`, `sort_run_summaries_newest_first`, `new_run_id`, `RecordedBounds`, `EMITTED_KINDS`, `RESERVED_KINDS`, `MISSED_*`, `from_exec_event` with its projection helpers, `JournalRun::{record_exec, set_claude_pgid, set_escalations_used}`, `RunPaths::inbox`, and the `RunRecord` fields `bounds`, `approved_plan`, `escalation_cap`, `escalations_used`.

## Surviving `ctx.experimental` production reads

Every one fronts a Driver/launcher surface that exists:

| Site | Gates |
|---|---|
| `ui/screens/mod.rs::needs_attention_for`, `attention_rank_for`, `refresh_launcher_marker_from_cache`, `schedule_launcher_marker` | launcher marker badge, attention rank, `h` filter, the per-project marker scan |
| `ui/screens/normal.rs::row_badge` | the dashboard's driven badge (launcher marker class) |
| `ui/screens/detail.rs::driver_live_for` | the "driver live" indicator |
| `ui/screens/detail.rs` `handle_key` (`D`, left/right tab stepping), `switch_to_tab`, `render`, `handle_mouse`, footers | the Driver tab: visibility, `Shift+D`, tab stepping, tab-bar and sub-view footers, mouse hit-testing |
| `ui/screens/help.rs::render` -> `help_lines(experimental)` | the help screen's Driver section |
| `app.rs::stop_launcher_run`, `launch_target`, `request_new_run`, `steering_target` | launcher stop, launch, new-run form, steering |
| `app.rs` 20-tick block and `Action::LauncherMarkerScanned` | the launcher marker rescan and its apply |

No read remains for `start_driver_run` / `stop_driver_run` / `r` / `x` / `o` (gone in 31-02) or for `drive`.

## Accomplishments

- **Tracer (Task 1).** `Commands::Drive`, its `main.rs` arm, `driver::drive` and every item only it used, `run/spawn/dry_run/reconcile.rs`, 19 test binaries (16 `driver_*`, `journal_run_paths`, `envelope_wiring`, `envelope_advisory`), 6 fixtures and `injection-corpus/` are gone; `driver/mod.rs` is a 60-line module root with a new module doc. `TerminalState` (the UI label parser kept by 31-03) and the `experimental` test over `driver::spawn::admit` went with their subjects. The tracer `<verify>` was automated-only, so it was re-run before expansion (7 integration binaries + 585 lib tests + clippy): tracer verified end-to-end, expanding.
- **Goal layer and journal (Task 2).** `goal/escalate/untrusted/bounds.rs`, `journal/inbox.rs`, `DriveError` and the journal items listed above are gone. Old `run.json` files still load (pinned by a byte-literal test). `lock::{lock_path, acquire, read_holder}` and `kill::{stop_run, ReapArm, StopOutcome}` are gone; `signal_group`, `DRIVER_TEARDOWN_GRACE`, `RunLock`, `acquire_at`, `acquire_at_with_retry`, `probe_at`, `read_holder_at`, `LockProbe`, `LockHolder`, `ACQUIRE_RETRY_BUDGET` stay.
- **Preferences and docs (Task 3).** Preferences shows a header, the file path, one dim "(none: ...)" line and the GSD-template route; `j/k` and `</>` are no longer advertised. README, CONFIGURATION and GSD-CORE-SYNC lose the `drive` subcommand, `r`/`x`/`o`, startup reconciliation, the concurrency cap, the opt-in launch gate and `claude -p` / `codex exec` driving. Zero source files name `driver::{goal,escalate,untrusted,run,spawn,dry_run,bounds,reconcile}`.

## Task Commits

1. **Task 1 (tracer): the drive subcommand, its run/spawn/dry-run/reconcile code and 19 test binaries** - `2a23d9e` (refactor)
2. **Task 2: the goal layer, DriveError, the inbox core, the legacy kill/lock entry points and the journal trim** - `9caf162` (refactor)
3. **Task 3: Preferences, user docs, doc-link sweep, cross-target clippy** - `232decb` (docs)

## Dead-code allows added (held plan 31-07)

One: `DrivableProject::with_runtime` in `src/executor/mod.rs` carries `#[allow(dead_code)]` with a comment pointing at held plan 31-07 (D-04). Its only caller was `drive`. Nothing else warned: the rest of the executor is `pub` library surface, which raises no `dead_code`. The executor, `fake-codex.sh`, `tests/fixtures/codex/`, `OptInError`/`SpawnError`/`SendError`/`CapabilityError` and `process-wrap` were not touched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Apple-target clippy broken by 31-01's FIFO test helper**
- **Found during:** Task 3 (the plan's cross-target clippy verify)
- **Issue:** `src/envelope/ledger.rs::make_fifo` calls `rustix::fs::mknodat`, which rustix does not provide on Apple targets, so `cargo clippy --target aarch64-apple-darwin` / `x86_64-apple-darwin` failed with E0425 (introduced by 31-01 commit `46901b5`, not by this plan).
- **Fix:** `make_fifo`, `attempt_within_twelve_seconds` and the two FIFO rows are `#[cfg(all(unix, not(target_vendor = "apple")))]` (the FIFO block inside the four-kinds test is a `#[cfg(not(target_vendor = "apple"))]` block). The directory, symlink-to-device and fresh-root rows still run on every Unix. No new dependency; Linux runs every row, so local coverage is unchanged. CI's `cross-target-check` only compiles (clippy), so no macOS row ran before either.
- **Files modified:** src/envelope/ledger.rs
- **Commit:** `232decb`

**2. [Rule 3 - Blocking] A dead function in goal.rs broke the Task 1 clippy gate**
- **Found during:** Task 1 verify
- **Issue:** `GoalRefusal::empty_plan` (`pub(super)`, called only by the deleted `driver::approve_plan`) raised `dead_code` while `goal.rs` still existed for Task 2.
- **Fix:** removed the function; the whole file went in Task 2.
- **Commit:** `2a23d9e`

**3. [Rule 2 - Honest text] Doc comments claimed deleted guards and censuses**
- **Found during:** Task 2 and the Task 3 sweep
- **Issue:** comments in `state_reader/*`, `journal/mod.rs` (the `Parked.reason` taxonomy table named `BoundsReason` and `EscalationReason`), `error.rs`, `router.rs`, `lock.rs`, `kill.rs`, `ui/screens/mod.rs` and `envelope/advisory.rs` named deleted modules, a deleted census (`THIRD_PARTY_STRINGS`) or deleted injection/dry-run render sites as live.
- **Fix:** reworded to state the surviving fact; the `Parked` table now lists three taxonomies, not five.
- **Commits:** `9caf162`, `232decb`

**4. [Rule 3 - Process] Scripted item removal left fragments**
- **Found during:** Task 1 (`spawn_seam_guard.rs`)
- **Issue:** the item-remover ended a `const X: [T; N] = [...]` at the `;` inside the type, leaving the array body and orphan section comments behind.
- **Fix:** the leftover fragments were removed by hand before the first compile, and the tool now tracks bracket depth. No effect on committed history.

**Total deviations:** 3 auto-fixed (1 Rule 2, 2 Rule 3), 1 process note. **Impact:** no scope growth; the Apple fix repairs a pre-existing break the plan's own verify command exposes.

## Decisions Made (inferred, for audit)

- Preferences: both rows removed, header/path/`g` route kept, an honest "(none: ...)" line added; config fields untouched (`src/config.rs` has no 31-04 commit). `[inferred — audit]`
- `RunRecord` keeps `goal`, `gsd_command`, `target_phase` and the rest (plain strings) for 31-05 to delete whole; only the four fields tied to deleted types/goals went, and old records keep them in `extra`. `[inferred — audit]`
- `kill.rs` keeps a test-only mirror of the 2 s SIGKILL reap window (it was `KILL_REAP_BOUND` in the deleted `stop_run`) so the grace-plus-reap budget test keeps its second term without dead production code. `[inferred — audit]`
- The README now says "the app does not drive Codex" and drops the `.git/info/exclude` claim (only `envelope/hooks.rs`, now unreachable, wrote that block); both statements are about behaviour, not about whether executor code is in the tree. `[inferred — audit]`
- Task 2 deleted `JournalRun::{record_exec, set_claude_pgid}` and its `started`/`last_cost_usd`/`claude_pgid` state: their only caller was `run.rs`. `[inferred — audit]`

## Observed, not changed

- The opt-in confirm popup title for granting is still `Allow driver` (`driver_confirm.rs:358`, pinned by a 31-02 test). The prompt text beneath it already says the record only feeds the PR caps and branch namespace; the title reads slightly stronger than that. Left for the owner of 31-02's wording.
- `envelope::advisory::not_probed()` has no caller now that `dry_run` is gone; 31-05 deletes `advisory.rs`.
- `tests/journal_crash.rs` still passes (3 tests) and exists only for `JournalWriter`; 31-05 decides it with the writer.

## Issues Encountered

None beyond the deviations above.

## Known Stubs

None.

## Threat Flags

None. The plan added no endpoint, auth path or file-access surface. T-31-10 is mitigated as planned (only entries naming deleted files or reported by `vanished` were removed; floors re-measured with old/new recorded above). T-31-12: `tests/driver_router_conformance.rs` has no commit since `036426d` and passes (6 tests); `RouterAction`/`SAFE_COMMAND_ALPHABET` untouched. T-31-13: the doc acceptance greps below are all zero.

## Operator note: a legacy `drive` process still running at upgrade (T-31-11, accepted)

Nothing can start a `drive` process any more, and the TUI cannot see or stop one (31-03). If a `drive` process started by an older build is still running when this build is installed, **kill it by hand from a shell** (it leads its own process group). Its run directory under `.planning/meta-manager/runs/` stays on disk, untouched and ignored; the per-project `runs/run.lock` it holds is no longer read by anything in this build.

## Next Phase Readiness

Ready for 31-05: `driver::run` no longer consumes the guard, so `envelope/{hooks,cred,advisory}.rs`, the classifier and the corpora can go with the git-version witness and `pre-tag-check.sh` in one commit. Hand-offs: `envelope::park` and its tests are the remaining users of `JournalWriter`/`JournalRun`/`RunRecord`/`run_paths`/`read_active_run`, the `EnvelopeAction::{PrePush, PreCommit, Askpass, Guard}` arms still carry `Alias::new` rows in the argv alias census (seven rows now), `ExecutionOptions::envelope_env` is still the executor's last tie to `cred.rs`, and `tests/journal_crash.rs` rides the writer. 31-07 (held) finds `DrivableProject`, the Codex executor, the in-process Claude transport, `process-wrap` and their fixtures untouched, with the one `dead_code` allow above.

## Self-Check: PASSED

- Commits `2a23d9e`, `9caf162`, `232decb` are ancestors of HEAD; `git rev-list --count` from the recorded `plan_head_before` is 3
- Task 1: `run/spawn/dry_run/reconcile.rs` absent; `Drive {` in cli.rs 0 (baseline 3); `tests/*.rs` = 72 (baseline 91); `injection-corpus` and `fake-claude-seam.sh` absent; `injection-corpus` in Cargo.toml 0; `"src/driver/spawn.rs"` in spawn_seam_guard 0; `"src/driver/(mod|run).rs"` in async_blocking_guard 0; all three verify commands exit 0
- Task 2: `goal/escalate/untrusted/bounds.rs` and `journal/inbox.rs` absent; `DriveError` in src+tests 0 (baseline 149 in src); 7 surviving `pub mod` lines, 0 deleted ones; `fn (stop_run|lock_path|acquire|read_holder)` kill.rs 0 / lock.rs 0; `fn (signal_group|acquire_at|probe_at|read_holder_at)` kill.rs 1 / lock.rs 3; `approval_digest|parse_approval_token|ApprovedPlan` in journal/mod.rs 0; no commit touches `tests/driver_router_conformance.rs` since `036426d`; all three verify commands exit 0
- Task 3: `driver_max_concurrent` in preferences.rs 0 (baseline 12); `GSD GLOBAL TEMPLATE` 2; `driver::(goal|...)` in src 0 (baseline 81); stale CONFIGURATION phrases 0 (baseline 3); `experimental driver (can run|runs either)` in README 0 (baseline 2); `driver_codex_runtime` in GSD-CORE-SYNC 0 (baseline 1); no 31-04 commit touches `src/config.rs`; lib filter 49 pass; clippy host + three targets clean; FULLSUITE `results=75` equals expected, only the witness fails
- Plan verification: `spawn_seam_guard`, `async_blocking_guard`, `next_cli`, `driver_router_conformance`, `driver_tab_launch_e2e`, `launcher_e2e`, `launcher_stop` all pass

---
*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Completed: 2026-10-25*
