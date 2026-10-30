---
phase: 18-driver-tab-live-watch-durable-injection
plan: 08
subsystem: infra
tags: [rust, clap, cfg-gate, supply-chain, journal, diagnostic, grep-guard, security]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "WR-16 itself, `tests/spawn_seam_guard.rs`, the `DriveArgs` override fields and the `hide = true` flags this plan closes"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "the nine `tests/fixtures/fake-claude*.sh` stand-ins that make the debug-only half of D-30 necessary"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 02
    provides: "`execute_run`'s post-`spawn_blocking` shape, the `DriverRun` binding the marker is written through, and `observing_replay_echoes` on the executor builder chain"
provides:
  - "A released binary with no parser entry for `--claude-program`/`--claude-args` — the flags are rejected with a clap `unexpected argument` error and exit 2"
  - "`Diagnostic { code: \"agent_program_overridden\" }`, journaled between `run_started` and the first `exec_started` whenever a debug build's override is used"
  - "`agent_program`/`agent_leading_args`/`agent_override_detail`/`journal_agent_program_override` — the cfg-split consumer surface in `src/driver/run.rs`"
  - "`the_agent_program_override_fields_are_debug_only` + `an_override_field_declared_without_the_debug_gate_is_reported` — the mechanical guard on the cfg and its control arm"
affects: [phase-19, phase-20]

tech-stack:
  added: []
  patterns:
    - "Two `#[cfg]` function bodies behind one name rather than `if cfg!(…)`, because `cfg!` compiles both arms and leaves the dangerous path in the shipped binary with only nothing able to select it"
    - "A security-relevant `#[cfg]` is paired with a grep guard in the same commit; the attribute is the mechanism, the guard is what keeps it one"
    - "An accepted consequence is written into the source it constrains, in the register of `journal/writer.rs:24-28` — as a decision that was made, not a limitation that was discovered"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs
    - src/driver/mod.rs
    - src/driver/run.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "The cfg propagates all the way to `DriveArgs`, not just to the clap fields: a field surviving into the release binary keeps `ClaudeExecutor::with_program` reachable from anything that can build a `DriveArgs`, and \"the only caller today is the CLI\" is a fact about today"
  - "`agent_program`/`agent_leading_args` are a `#[cfg]` pair, not an `if cfg!(…)` — `cfg!` would ship the arbitrary-program exec path in the release binary"
  - "The marker's `detail` carries the stand-in's file name only; a path with no file name degrades to a fixed word rather than falling back to the path"
  - "The marker is written immediately after the journal opens, before the executor is even constructed, so it precedes `exec_started` by construction rather than by ordering luck"
  - "The guard matches declaration shape (`claude_program:`), not the bare identifier: covering use sites would require a lookback window wide enough to stop proving anything"
  - "`is_debug_only_gate` accepts `all(debug_assertions, …)` and rejects `any(…)`/`not(…)` — narrowing is still debug-only, widening is the refactor being guarded against"

requirements-completed: [TRANS-05]

coverage:
  - id: D1
    description: "A released binary has no parser entry for the agent-override flags; passing one is a parse error, not a silent acceptance"
    requirement: TRANS-05
    verification:
      - kind: other
        ref: "`./target/release/gsd-meta-manager drive alias --command /gsd-progress --claude-program /bin/true --run-id x` → `error: unexpected argument '--claude-program' found`, exit 2"
        status: pass
      - kind: other
        ref: "`cargo test --release` fails to build the integration tests that set the fields (E0560, `DriveArgs` has no field named `claude_program`) — the gate propagated rather than stopping at clap"
        status: pass
    human_judgment: false
  - id: D2
    description: "A debug-build override is journaled as a diagnostic before the run's first exec record"
    requirement: TRANS-05
    verification:
      - kind: other
        ref: "end-to-end through the real CLI: journal kinds in order are `run_started`, `diagnostic`, `exec_started`, … with `{\"code\":\"agent_program_overridden\",…,\"seq\":2}`"
        status: pass
    human_judgment: false
  - id: D3
    description: "The marker names the stand-in and leaks neither its path nor its arguments"
    requirement: null
    verification:
      - kind: unit
        ref: "src/driver/run.rs#the_override_marker_names_the_stand_in_and_leaks_neither_its_path_nor_its_arguments"
        status: pass
    human_judgment: false
  - id: D4
    description: "The cfg attribute is enforced by a test that fails loudly if a later refactor removes or widens it"
    requirement: null
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_agent_program_override_fields_are_debug_only"
        status: pass
      - kind: other
        ref: "mutation check: removing `#[cfg(debug_assertions)]` from `src/cli.rs` makes it fail naming `src/cli.rs:123`; attribute restored, tree byte-identical"
        status: pass
    human_judgment: false
  - id: D5
    description: "The guard is non-vacuous: a declaration without the attribute is reported, one with it is not, and a rename cannot empty the audit silently"
    requirement: null
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#an_override_field_declared_without_the_debug_gate_is_reported"
        status: pass
    human_judgment: false
  - id: D6
    description: "The fixtures are still reachable in a debug build — securing the binary did not break the test suite"
    requirement: null
    verification:
      - kind: integration
        ref: "tests/driver_tracer.rs (4), tests/driver_kill.rs (3), tests/driver_inbox.rs (7), tests/driver_lock.rs, tests/driver_optin.rs, tests/driver_dry_run.rs — all pass"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 08: The Agent-Override Flags Leave the Released Binary Summary

**A released `gsd-meta-manager` can no longer be told to exec an arbitrary program inside an opted-in project root — the parser has no entry for `--claude-program` at all — and in a debug build, where the flag must keep working for the nine fake-`claude` stand-ins, a run driven by one is labelled on disk between `run_started` and the first `exec_started`.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-07-30T00:02:00Z
- **Completed:** 2026-07-30T00:22:00Z
- **Tasks:** 2
- **Files modified:** 5 (0 created, 5 modified)

## Accomplishments

- **The parser entry is gone, and it is gone all the way down.** `#[cfg(debug_assertions)]`
  sits on both clap fields, on the `Commands::Drive` pattern and `DriveArgs`
  literal in `main.rs`, on the `DriveArgs` fields themselves, and on both
  consumer paths in `execute_run` — the argv digest's program lookup and the
  `ClaudeExecutor::with_program` branch. A release binary answers
  `--claude-program` with `error: unexpected argument` and exit 2.
- **The gate is proved to have propagated rather than stopped at clap.**
  `cargo test --release` now fails with `E0560: struct DriveArgs has no field
  named claude_program`. That failure is the *evidence*: a cfg that stopped at
  the CLI struct would have left `DriveArgs` intact and the release test build
  green, with `with_program` still sitting in the shipped binary waiting for a
  caller.
- **`cfg!` was refused in favour of two `#[cfg]` bodies.** `agent_program` and
  `agent_leading_args` each have a debug body and a release body.
  `if cfg!(debug_assertions)` compiles **both** arms, so the released binary
  would still contain the code path that execs an arbitrary program — merely
  with nothing able to select it. "Unreachable today" is a fact about today.
- **The debug-build gap is closed by a record, not by a comment.** The cfg
  cannot help where the flag legitimately still works, and there a stand-in run
  writes `run_started`, `exec_started`, `run_ended` and an outcome —
  indistinguishable from a real agent's transcript. `execute_run` now journals
  `Diagnostic { code: "agent_program_overridden" }` immediately after the
  journal opens, **before the executor is even constructed**, so it precedes
  `exec_started` by construction rather than by ordering luck. Verified end to
  end through the real CLI: the record lands at `seq: 2`.
- **The marker names the stand-in and nothing else.** `detail` carries the file
  name only — never the path, which would publish where a developer's tree
  lives, and never the arguments, which in this repo's own fixtures already
  carry transcript paths. The journal is written inside the *driven* project's
  `.planning/`, a directory a user may well commit. A path with no file name
  degrades to a fixed word rather than falling back to the full path, because
  the fallback is the thing being avoided.
- **The cfg is now a mechanism.** `the_agent_program_override_fields_are_debug_only`
  walks `src/` through the existing `SRC_ROOT` constant and fails if any
  override-field declaration lacks a debug-only gate within three lines above
  it — with a message that names the consequence rather than the value. It was
  proved against the real tree, not only against synthetic input: removing the
  attribute from `src/cli.rs` made it fail naming `src/cli.rs:123`.
- **No new dependency, no clippy regression, no fmt drift added.**
  `Cargo.toml`/`Cargo.lock` untouched; `cargo clippy --all-targets` still reports
  exactly the 5 pre-existing lints. Test count 612 → 615.

## Task Commits

1. **Task 1: gate the flags out of release and mark an overridden run on disk** —
   `13d2ee2` (fix)
2. **Task 2: make the cfg a mechanism rather than a convention** — `6966dfd` (test)

## Files Created/Modified

**Modified**
- `src/cli.rs` — `#[cfg(debug_assertions)]` on both override fields, the gated
  `OsString` import, and the extended field docs carrying both the preserved
  env-var reasoning and the newly recorded release reasoning and accepted
  consequence.
- `src/main.rs` — the attribute on the `Commands::Drive` pattern field and on
  the `DriveArgs` struct-expression field, so the arm is not forked in two.
- `src/driver/mod.rs` — the attribute on the `DriveArgs` fields, their imports
  and the in-source test helper's literal.
- `src/driver/run.rs` — `DEFAULT_AGENT_PROGRAM`, `AGENT_PROGRAM_OVERRIDDEN`, the
  `agent_program`/`agent_leading_args` cfg pairs, `agent_override_detail`,
  `journal_agent_program_override` and its call site, the cfg-split executor
  construction, and
  `the_override_marker_names_the_stand_in_and_leaks_neither_its_path_nor_its_arguments`.
- `tests/spawn_seam_guard.rs` — `AGENT_OVERRIDE_FIELDS`, `OVERRIDE_PARSER_HOME`,
  `GATE_LOOKBACK`, `is_override_declaration`, `is_debug_only_gate`,
  `override_declarations`, and the two new tests.

## Decisions Made

- **The cfg propagates to `DriveArgs`, not just to the clap fields.** The
  narrower reading — gate the parser entry, leave the library struct alone —
  would have satisfied WR-16's literal words and left `with_program` reachable
  from anything that can build a `DriveArgs`. It also would have made D-30's
  recorded consequence *false*, since the integration tests construct `DriveArgs`
  directly and would have kept building in release. That the consequence
  materialised is how this plan knows the gate is real.
- **The guard keys on declaration shape rather than on the bare identifier.**
  A bare `claude_program` also matches `args.claude_program`, the shorthand
  `claude_program,` in a pattern, and every doc mention. Each of those sits an
  arbitrary distance below the `#[cfg]` that governs it, so covering them would
  mean widening `GATE_LOOKBACK` until the window stopped proving anything. The
  declarations and struct-literal initialisers are where the field either exists
  in release or does not; those are what the audit checks, and the test says so
  in an assertion rather than in a comment.
- **`is_debug_only_gate` accepts `all(debug_assertions, …)` and rejects
  `any(…)`/`not(…)`.** A narrower gate is still debug-only and should pass;
  `any(debug_assertions, feature = "dev-tools")` is exactly the shape a future
  refactor would use to hand the flag back to the release build while looking
  like it kept the attribute. Whitespace is stripped before matching so rustfmt
  cannot break the audit.
- **The audit refuses to pass when it finds no declaration in `src/cli.rs`.**
  An emptiness assertion over a matcher that recognises nothing passes forever.
  A rename must be re-pointed deliberately — the same argument `source_files`
  already makes with its "auditing nothing" assertion.
- **The accepted consequence lives in `src/cli.rs`, written as a decision.** The
  register is `journal/writer.rs:24-28` — *"one cosmetic consequence worth
  writing down so nobody 'fixes' it"*. Here the thing nobody should "fix" is the
  cfg, and the doc names the test that will stop them.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The gate cannot compile without touching `src/main.rs` and `src/driver/mod.rs`**

- **Found during:** Task 1
- **Issue:** The plan's artifact list names `src/cli.rs`, `src/driver/run.rs` and
  `tests/spawn_seam_guard.rs`. But `#[cfg(debug_assertions)]` on a clap field
  makes `src/main.rs:88-102` — which destructures `Commands::Drive` by name and
  rebuilds a `DriveArgs` from it — fail to compile in release. And the plan's own
  instruction to gate *"every consumer path that reads them"* is unsatisfiable
  without `DriveArgs` itself: gating `run.rs`'s reads while `src/driver/mod.rs`
  keeps ungated fields would be a gate on the reader of a value that still
  exists. D-30's recorded consequence — that `cargo test --release` stops
  building the integration tests — is only reachable if the `DriveArgs` fields
  are gated, since those tests construct `DriveArgs` directly and never touch
  clap. So the plan's under-enumerated file list, not its instruction, is what
  had to give.
- **Fix:** Both files gated. `src/main.rs` carries the attribute on the pattern
  field and the struct-expression field rather than forking the match arm in
  two — one arm drifting out of sync with the other is the bug a cfg exists to
  prevent. `src/driver/mod.rs` gates the fields, their `OsString`/`PathBuf`
  imports and its in-source test helper's literal.
- **Ownership check before editing:** neither file is in a wave-3 sibling's
  scope — 18-05 owns `src/app.rs` + `detail.rs`, 18-06 owns `normal.rs`, 18-07
  owns the new driver input screens + `screens/mod.rs` — so there is no hunk to
  clobber and no fence crossed.
- **Files modified:** `src/main.rs`, `src/driver/mod.rs`
- **Verification:** debug and release builds both clean; `cargo test --release`
  fails only with `E0560` on the integration tests that set the fields, which is
  the accepted consequence and the evidence the gate propagated.
- **Committed in:** `13d2ee2`

**2. [Rule 2 - Missing Critical] A unit test on the marker's detail, which the plan did not ask for**

- **Found during:** Task 1
- **Issue:** The plan's threat register carries T-18-45 — the override's full
  path or arguments leaking into the journal — as a `mitigate` row, but the
  acceptance criteria check only that the *code* string is present in
  `run.rs`. A `detail` that quietly used `program.display()` would satisfy every
  stated criterion and every stated criterion would still be met after a later
  refactor reintroduced the leak.
- **Fix:** `agent_override_detail` was factored out as a pure function and
  `the_override_marker_names_the_stand_in_and_leaks_neither_its_path_nor_its_arguments`
  asserts on it directly: the stand-in's name is present, no path separator or
  parent-directory component is, and the no-file-name case does not fall back to
  the path.
- **Files modified:** `src/driver/run.rs`
- **Committed in:** `13d2ee2`

**3. [Deliberate] Task 1's commit was amended for a rustfmt reflow**

- **Found during:** Task 2
- **Issue:** One `format!` line added by Task 1 was 101 characters, which
  `cargo fmt` wants to wrap. The tree is broadly not rustfmt-clean already — the
  drift is pre-existing across ~120 files and reformatting them is nobody's task
  here — but *adding* a new drifting line while that is true is how the backlog
  grows.
- **Fix:** The line was reflowed to rustfmt's own shape and folded into Task 1's
  commit by `--amend`, so the task-to-commit mapping stays one-to-one. Task 1's
  hash is therefore `13d2ee2`, not the pre-amend `d9f2ed7`. The branch was never
  pushed and no sibling had seen either hash.
- **Impact:** None beyond the hash.

---

**Total deviations:** 3 (1 blocking-mechanical with a scope consequence, 1
missing-critical test, 1 deliberate process). Deviation 1 is the only one that
widened the file set, and it widened it in the direction D-30's own recorded
consequence requires.

## Issues Encountered

- **A criterion that greps for a literal can miss an emission that uses a
  constant.** The plan asks that `rg -n 'agent_program_overridden'
  src/driver/run.rs` *"matches at the emission site"*. It matches once — at the
  `AGENT_PROGRAM_OVERRIDDEN` const — because the emission uses the identifier,
  which is exactly the shape `TERMINATE_DIAGNOSTIC_CODE` established two plans
  ago and the shape the plan's own artifact list asks for. The criterion's
  intent (the code is emitted from `run.rs`) is met and was checked the only way
  that is not vacuous: end to end, by reading the record off disk.
- **The end-to-end check found the runs root is `.planning/meta-manager/runs/`,
  not `.planning/runs/`.** Worth knowing for any later hand-written verification;
  `journal::run_paths` is the answer, and guessing the layout is not.
- **`cargo fmt --check` is not part of this project's gate and cannot become one
  incidentally.** ~120 files drift under the current rustfmt. Every drift this
  plan touched was confirmed pre-existing (18-02's `spawn_blocking` wrap, the
  guard file's pre-existing allowlist test) except the one line noted in
  deviation 3, which was fixed.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo build --release` | pass |
| `cargo test` | **615 passed, 0 failed** (base: 612; +1 unit, +2 guard) |
| `cargo clippy -- -D warnings` | pass (exit 0) |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat aef3143 HEAD -- Cargo.toml Cargo.lock` | empty — no new dependency |
| release binary + `--claude-program` | `error: unexpected argument '--claude-program' found`, **exit 2** |
| debug binary + `--claude-program` | parses; reaches the alias refusal — the fixtures are still reachable |
| journal of an overridden debug run | `run_started`, **`diagnostic`**, `exec_started`, … — marker at `seq: 2` |
| marker `detail` | ``this run execs the stand-in `fake-claude.sh` instead of the agent, so it is not a real agent run`` — file name only |
| `cargo test --release` | **fails to build the flag-passing integration tests (E0560)** — the accepted, recorded consequence of D-30, and the evidence the gate propagated past clap |
| `grep -c 'SPAWN_MARKERS' tests/spawn_seam_guard.rs` | 2 — unchanged; WR-17 was not folded in |
| `ls tests/ \| grep -c guard` | 1 — no second guard file |
| mutation check on the guard | removing the attribute from `src/cli.rs` fails the test naming `src/cli.rs:123`; restored, `git diff src/cli.rs` empty |

Every count above was taken through `rtk proxy`, because plain `cargo` output is
filtered by the `rtk` summarising wrapper and a criterion that greps for
`warning:` or `test result:` without it passes **vacuously**.

`tests/driver_reattach.rs` — the known flake under parallel-worktree load — passed
in the full-suite run; no re-run in isolation was needed.

## Known Stubs

None. Every artifact this plan declares has a producer and a test, and the one
property that could only be checked outside the suite (a release binary's parser
rejecting the flag) was checked against a real release binary.

## Threat Flags

None. The plan's `<threat_model>` rows are all addressed:

| Threat | Disposition | Where |
|---|---|---|
| T-18-43 (release-build elevation of privilege) | mitigated | no parser entry; release binary exits 2 on the flag; `the_agent_program_override_fields_are_debug_only` holds the source property |
| T-18-44 (stand-in run indistinguishable on disk) | mitigated | `agent_program_overridden` at `seq: 2`, before the first `exec_started`, verified end to end |
| T-18-45 (path or arguments leaking into the journal) | mitigated | file name only; asserted by a unit test on `agent_override_detail`, not by a comment |
| T-18-46 (a later refactor quietly widening the cfg) | mitigated | the grep guard, with a control arm and a real-tree mutation check; `any(…)`/`not(…)` are rejected spellings |
| T-18-47 (package-manager installs) | accepted | zero Cargo dependencies added |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready.** WR-16 is closed and D-30 is satisfied in both halves.

- **Phase 19** inherits one fewer hole to build its git blast-radius envelope
  around. This plan closed a single flag and deliberately did not start on push
  allowlists, `--disallowedTools`, pre-push hooks or worktree isolation, which
  remain Phase 19's.
- **Phase 20** gains a `DriveArgs` whose release surface is exactly the arguments
  a user can legitimately supply, which is the shape the decision router will
  extend.
- **WR-17** — the spawn-seam allowlist matching three spellings and missing
  several — keeps the owner `17-08-PLAN.md` assigned it. `SPAWN_MARKERS` is
  unchanged in this commit and the new assertion was added to the existing guard
  file rather than to a second one.

**Carried obligations**

- **The doc in `src/cli.rs` is load-bearing.** Anybody who "fixes" the
  release-test build by widening the cfg re-opens WR-16. The doc names the test
  that will stop them and the test's failure message names the consequence; both
  halves are needed, because a test whose message says only "assertion failed"
  invites deletion.
- The negative carry-forward from 18-01/18-02 still holds and this plan adds
  nothing to it: **every new per-alias or per-run map must be pruned in
  `App::prune_driver_maps`**. This plan introduces no map.

## Self-Check: PASSED

- `.planning/phases/18-driver-tab-live-watch-durable-injection/18-08-SUMMARY.md`
  written and committed.
- Both task commits present in `git log`: `13d2ee2` (fix), `6966dfd` (test).
- No deletions in either task commit — `git diff --diff-filter=D HEAD~1 HEAD`
  empty for both.
- No edits to `.planning/STATE.md` or `.planning/ROADMAP.md`.
- Every file claimed under "Files Created/Modified" appears in
  `git diff --name-only aef3143 HEAD`.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
