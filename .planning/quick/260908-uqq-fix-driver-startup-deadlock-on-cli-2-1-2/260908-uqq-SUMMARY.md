---
phase: quick-260908-uqq
plan: 01
subsystem: executor
tags: [claude-cli, stream-json, startup-handshake, capability-gate, deadlock, journal, tokio]

requires:
  - phase: 15 (executor transport)
    provides: the `system/init` capability gate, the Coordinator's two-layer supervisor loop, the stdin writer task
  - phase: 17 (driver run journal)
    provides: `JournalEvent::Diagnostic`, `finish_run`, `outcome_label`'s terminal-label vocabulary
provides:
  - "`ExecutionOptions::prompt_release_grace` — a startup-specific bound that releases the first user message when the child has not announced itself"
  - "A grace-release arm in `Coordinator::run`'s unconditional enforcement block, with `prompt_released` threaded into `handle_item`"
  - "`tests/fixtures/fake-claude-late-init.sh` — a CLI-2.1.266-shaped stand-in that announces only after its first stdin line"
  - "Both capability-refusal arms pinned by tests: zero bytes on the eager arm, prompt-already-written on the late arm"
  - "`SpawnError::StalledBeforeInit` and `SpawnError::TimedOutBeforeInit` — a pre-gate breach is no longer spelled `spawn_failed`"
  - "`driver::run::spawn_failure_label` and `spawn_failure_diagnostic` — a derived terminal label and a closed journal vocabulary"
affects: [executor, driver, ui-driver-screen, any future retune of the startup bounds]

actuals:
  tokens: 18000
  tasks: 2
  commits: 2
plan_head_before: 0dde2b40c17bb12bfd75cdf418902993ce30d078

tech-stack:
  added: []
  patterns:
    - "Startup bound as its own knob rather than a reused run-wide cap"
    - "`try_send` + set-the-flag-before-inspecting-the-result inside the unconditional enforcement block (CR-01 discipline)"
    - "Empty `select!` arm as a pure unpark for a bound the enforcement block owns"
    - "Two-arm honesty: a guarantee that survives conditionally is documented per arm and pinned by one test per arm"
    - "Closed driver-authored vocabulary for journal `code`/`detail`, never the rendered error"

key-files:
  created:
    - tests/fixtures/fake-claude-late-init.sh
  modified:
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/executor/gate.rs
    - src/error.rs
    - src/driver/run.rs
    - src/ui/screens/driver.rs
    - tests/executor_lifecycle.rs
    - tests/executor_transport.rs
    - tests/fixtures/fake-claude-silent.sh

key-decisions:
  - "Bounded grace, then release — not release-at-spawn. The eager arm's pre-prompt refusal is retained beside a new post-hoc abort arm (`add-alongside`), so the D-06 zero-cost guarantee lost its monopoly rather than its existence."
  - "The gate is neither widened nor deleted: both arms refuse, both stop the run loop, both tear the process group down. Only the ordering relative to the prompt differs, and the CLI's announce timing decides which arm runs."
  - "Every site asserting the zero-cost claim unconditionally was corrected in the same commit as the behaviour change that falsified it. An honest weaker claim beats a false strong one."
  - "`idle_cap` deliberately NOT shrunk. 'Has the child introduced itself' and 'has this run gone silent' are different questions and one number cannot mean both."
  - "The grace release uses `try_send`, never an awaited send, and sets `prompt_released` before inspecting the result — an awaited send would park the one place every cap and the cancel are evaluated (CR-01), and a flag set only on success would busy-spin against a deadline already in the past."
  - "The spawn-failure label mapping reuses `outcome_label`'s four existing words and invents none, because a new word would reach disk and paint as `Unrecorded`."
  - "Journal `code` and `detail` come from a closed driver-authored vocabulary, never from the error's `Display` — `CapabilityError` interpolates CLI-supplied strings into a surface that is read back and painted."

patterns-established:
  - "Per-arm test pinning: `a_refused_run_writes_zero_bytes_to_the_child_stdin_on_the_eager_arm` and `a_refused_run_on_a_late_announcing_cli_has_already_written_the_prompt` are siblings differing only in which stand-in they point at."
  - "Fixture as a versioned CLI stand-in: `fake-claude-late-init.sh` differs from `fake-claude-echo.sh` in exactly one thing — when the init arrives — so tests can swap them to isolate announce timing."

requirements-completed: [260908-uqq]

coverage:
  - id: D1
    description: "A run against a CLI that emits `system/init` only after the first stdin user message gets past startup, and its command reaches the child"
    requirement: "260908-uqq"
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command"
        status: pass
    human_judgment: false
  - id: D2
    description: "On a CLI that announces before the grace expires, a capability refusal still writes ZERO bytes to the child's stdin"
    requirement: "260908-uqq"
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#a_refused_run_writes_zero_bytes_to_the_child_stdin_on_the_eager_arm"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#a_refused_first_init_never_releases_the_prompt_on_the_eager_arm"
        status: pass
    human_judgment: false
  - id: D3
    description: "On a CLI that announces late, a capability refusal still refuses with `SpawnError::Capability` and the prompt has already been written"
    requirement: "260908-uqq"
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#a_refused_run_on_a_late_announcing_cli_has_already_written_the_prompt"
        status: pass
    human_judgment: false
  - id: D4
    description: "The prompt is written exactly once whichever arm released it"
    requirement: "260908-uqq"
    verification:
      - kind: integration
        ref: "tests/executor_lifecycle.rs#a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command (one-copy assertion)"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#a_second_system_init_does_not_re_run_the_gate_or_abort_the_run"
        status: pass
    human_judgment: false
  - id: D5
    description: "The default grace is greater than zero and strictly less than the default idle cap, and `idle_cap` is unchanged at 15 minutes"
    requirement: "260908-uqq"
    verification:
      - kind: unit
        ref: "tests/executor_lifecycle.rs#the_default_prompt_release_grace_sits_strictly_between_zero_and_the_idle_cap"
        status: pass
    human_judgment: false
  - id: D6
    description: "CR-01's pgid publication and the `biased` terminate-arm ordering are unchanged"
    requirement: "260908-uqq"
    verification:
      - kind: integration
        ref: "tests/driver_kill_startup.rs (1 passed), tests/spawn_seam_guard.rs (38 passed)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every comment in `src/` and `tests/` that asserted the refusal's cost unconditionally now states the condition under which it holds"
    requirement: "260908-uqq"
    verification:
      - kind: other
        ref: "rtk proxy grep -rn 'zero tokens|zero quota|no turn ever|never releases the prompt' src/ tests/ — every surviving hit is inside a per-arm conditional statement"
        status: pass
    human_judgment: true
    rationale: "Whether a corrected sentence is genuinely honest, rather than merely containing the word 'grace', is a reading judgment no grep can make. The plan deliberately specified no negative grep, because an honest correction may quote the claim it corrects."
  - id: D8
    description: "A pre-gate breach yields `stalled` or `timed_out`, not `spawn_failed`, and a `Diagnostic` naming the reason precedes `run_ended` on disk"
    requirement: "260908-uqq"
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_pre_gate_stall_is_labelled_stalled_and_never_spawn_failed"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#a_spawn_failure_puts_its_reason_on_disk_before_the_ending"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#every_label_the_spawn_failure_mapping_emits_is_one_the_render_layer_reads"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-09-09
status: complete
---

# Quick 260908-uqq: Fix the driver startup deadlock on CLI 2.1.266

**A grace-bounded prompt release breaks the driver/CLI startup deadlock against 2.1.266, and the D-06 "a capability refusal costs zero tokens" guarantee is rewritten as a per-arm conditional at every one of the thirteen sites that asserted it unconditionally.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 2 of 2 (Task 2 was droppable and was NOT dropped)
- **Files modified:** 10 (1 created, 9 modified)
- **Commits:** 2

## The crux, answered

**Yes — after this change the prompt CAN reach the child before the gate verdict.** It reaches the child before the verdict on any CLI that does not emit `system/init` within `prompt_release_grace` of spawn. CLI 2.1.266 is exactly such a CLI, so on 2.1.266 the gate can only ever judge an init that the prompt itself produced.

There was no alternative shape. Eliciting `system/init` from 2.1.266 without starting a turn is impossible — the measured behaviour is init, assistant message and result all arriving within one second of the first stdin write — and a probe message would itself be a turn and would spend quota.

## Accomplishments

1. **The deadlock is broken and proved broken.** `ExecutionOptions::prompt_release_grace` (default 5s) releases the first user message when the grace expires with nothing announced. The acceptance test drives the new stand-in end to end and asserts the command text is in the child's stdin log.
2. **Both refusal arms are pinned by tests**, against checked-in shell stand-ins. No real `claude` binary was invoked and no quota was spent.
3. **Thirteen stale claim sites corrected in the same commit as the behaviour change** that falsified them — the plan's inventory of eleven, plus two more found during the sweep.
4. **A pre-gate stall is no longer called `spawn_failed`**, and the reason it ended now exists on disk.

## Task Commits

1. **Task 1 (tracer): Release the prompt on a grace, prove both arms, correct every claim** — `114de68` (fix)
2. **Task 2: Stop calling a pre-gate stall `spawn_failed`, and put the reason on disk** — `2fb6592` (fix)

## RED-then-GREEN, measured

The acceptance test `a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command` was written and run **against the pre-change executor**, with the new fixture and the new `prompt_release_grace` field present but nothing wired to it.

**RED (observed, before the release was wired):**

```
test a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command ... FAILED

panicked at tests/executor_lifecycle.rs:846:6:
the stand-in advertises every required capability, so the gate must pass: InitNeverObserved

test result: FAILED. 0 passed; 1 failed; 0 ignored; 13 filtered out; finished in 5.01s
```

That is the reported production failure in miniature: `SpawnError::InitNeverObserved` after the test's 5-second idle cap, exactly as production produced it after its 900-second one.

**GREEN (after wiring the grace release):**

```
test a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command ... ok
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.15s
```

Task 2's label mapping was also RED-first — `spawn_failure_label` did not exist, so the test failed to compile (`E0425: cannot find function`) before the helper was written.

## Design decision: the gate, and what the zero-token claim now says

**Shape chosen: a bounded grace, then release.** Not "release at spawn, always".

| Shape | Gate becomes | Cost |
|---|---|---|
| Release at spawn, unconditionally | post-hoc validation everywhere | the pre-prompt refusal disappears; the zero-bytes test must be deleted or inverted; every eagerly-announcing CLI loses a guarantee it could still honour |
| **Release after a grace, if init has not arrived** | **pre-prompt refusal when the CLI announces in time, post-hoc abort otherwise** | **~40 lines, one flag, one knob, two pinned arms** |

**Assumption delta, recorded deliberately: `add-alongside`.** The pre-prompt refusal is retained as one arm; a post-hoc abort arm is added beside it. **The guarantee lost its monopoly, not its existence.**

The two arms, and what each costs:

- **Eager arm** — the CLI announced within the grace (CLI 2.1.220's measured shape). The gate rules before a single byte reaches the child's stdin. A refusal here still costs **zero tokens and zero quota**, exactly as D-06 always claimed.
- **Late arm** — the CLI announced only after reading a user message (CLI 2.1.266's measured shape). The grace released the prompt first, so the init the gate judges is the one the prompt provoked. A refusal here **aborts a turn that has already begun**. It is not free.

**The gate was neither widened nor deleted.** Both arms still refuse, both still return `SpawnError::Capability`, both still stop the run loop, both still tear the agent's process group down. Only the ordering relative to the prompt differs, and which arm runs is decided by the CLI's own announce timing — not by anything this driver chooses.

### The corrected claim sites

All thirteen landed in `114de68`, the same commit as the behaviour change.

| # | File | Site | What it says now |
|---|---|---|---|
| 1 | `src/executor/mod.rs` | module doc point 2 | The prompt is withheld until the gate passes **or** the grace expires; the zero-cost half is stated per arm |
| 2 | `src/executor/mod.rs` | `Executor::start` doc | Names the grace and says the late arm's start-time error is about a turn that has already begun |
| 3 | `src/executor/mod.rs` | `RunOutcome::CapabilityRefused` | "**No turn ever started**" → whether a turn had started depends on which arm ran |
| 4 | `src/error.rs` | `SpawnError` enum doc | Every *other* variant is genuinely free; `Capability` is the sole conditional one |
| 5 | `src/error.rs` | `SpawnError::Capability` | "No user message was ever written to stdin" → per-arm, with the 2.1.266 measurement cited |
| 6 | `src/executor/claude.rs` | the `gate_rx.await` comment | The refusal is unconditional; its cost is not — both arms spelled out |
| 7 | `src/executor/claude.rs` | "A refused run never had its prompt released" | → either its prompt was never released, or the grace released it into a child whose init then failed |
| 8 | `src/executor/claude.rs` | "Only now is the prompt released" | Labelled **the eager arm**, "and only on this arm" |
| 9 | `src/executor/claude.rs` | `feed()` helper doc | States it drives the eager arm and only the eager arm, and why (no grace timer exists there) |
| 10 | `src/executor/claude.rs` | two in-source assertion messages | Both now say they pin the **eager arm**, and point at where the late arm is pinned |
| 11 | `src/ui/screens/driver.rs` | `TerminalState::CapabilityRefused` | "no turn ever started" → "**not necessarily** no turn ever started" |
| 12 | `tests/executor_transport.rs` | section header + zero-bytes assertion | Header restructured around the two arms; the assertion scopes itself to the eager one |
| 13 | `tests/fixtures/fake-claude-silent.sh` | header | "The prompt is never released — the gate never opens" → the prompt **is** released now, and this drain is what swallows it |

**Two sites beyond the plan's inventory** were found during the sweep and corrected (deviation Rule 2 — a stale guarantee is a correctness defect in this codebase's own terms):

- `src/executor/gate.rs` — `REQUIRED_CAPABILITIES`' doc opened with "Capabilities a run requires **before any turn may begin**". Now: "requires, **not necessarily** before any turn may begin", with the reason.
- `src/executor/claude.rs` — the `feed()` helper doc (row 9 above), which the inventory did not list but which would otherwise read as a universal law.

**Where the honest weaker claim is written down as a test assertion**, per the plan:

> "THIS IS THE HONEST WEAKER CLAIM, WRITTEN DOWN. On a CLI that announces only after reading a user message, the `system/init` the gate judges is the one the PROMPT PROVOKED […] So on this arm the refusal aborts a turn that has already begun; it does not prevent one, and it does not cost zero tokens."
> — `tests/executor_transport.rs#a_refused_run_on_a_late_announcing_cli_has_already_written_the_prompt`

## Which tests prove the startup handshake

| Test | File | Proves |
|---|---|---|
| `a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command` | `tests/executor_lifecycle.rs` | **The acceptance test.** `start()` returns `Ok` against a late-announcing CLI, the command text is in the stdin log, exactly one copy of it, the provoked init still reaches the caller as `SessionStarted`, and the run ends on the handshake rather than on a bound |
| `the_default_prompt_release_grace_sits_strictly_between_zero_and_the_idle_cap` | `tests/executor_lifecycle.rs` | `0 < grace < idle_cap`, and `idle_cap` is still exactly 15 minutes |
| `a_refused_run_writes_zero_bytes_to_the_child_stdin_on_the_eager_arm` | `tests/executor_transport.rs` | The **eager** refusal arm: zero bytes on stdin, typed `SpawnError::Capability` |
| `a_refused_run_on_a_late_announcing_cli_has_already_written_the_prompt` | `tests/executor_transport.rs` | The **late** refusal arm: typed `SpawnError::Capability` **and** a non-empty stdin log containing the prompt |
| `a_refused_first_init_never_releases_the_prompt_on_the_eager_arm` | `src/executor/claude.rs` (in-source) | The eager arm at the `handle_item` level, renamed to say so |
| `a_second_system_init_does_not_re_run_the_gate_or_abort_the_run` | `src/executor/claude.rs` (in-source) | Written-exactly-once on the eager arm (D-30 unchanged) |
| `the_spawn_observer_publishes_the_agent_pgid_even_when_the_gate_never_opens` | `tests/executor_lifecycle.rs` | **Unchanged and still green** — CR-01's pre-gate pgid publication survives the longer startup window |
| `tests/driver_kill_startup.rs` | — | **Unchanged and still green** — the three-process startup-teardown proof |
| `a_pre_gate_stall_is_labelled_stalled_and_never_spawn_failed` | `src/driver/run.rs` (in-source) | Task 2's label mapping for all four outcomes, plus the `spawn_failed` fallback for all five remaining variants |
| `every_label_the_spawn_failure_mapping_emits_is_one_the_render_layer_reads` | `src/driver/run.rs` (in-source) | No invented vocabulary: every emitted label round-trips through `TerminalState::from_label` without landing on `Unrecorded` |
| `a_spawn_failure_puts_its_reason_on_disk_before_the_ending` | `src/driver/run.rs` (in-source) | The `Diagnostic` precedes `run_ended` in `journal.jsonl`, and the init-never-observed sentence is on disk |

The new fixture, `tests/fixtures/fake-claude-late-init.sh`, is a shell stand-in reached through the hidden `--claude-program` / `--claude-args` dev seam. It is identical to `fake-claude-echo.sh` except that it prints nothing at startup and emits its init only after reading its first stdin line. **No real `claude` binary was invoked at any point; no quota was spent.**

## Exact verification results

Full gate, run from the repo root with `rtk proxy` (per CLAUDE.md — rtk strips `warning:` and `test result:` lines, so any check reading raw output is vacuous without it):

| Command | Result |
|---|---|
| `rtk proxy cargo build` | **exit 0**, `Finished dev profile` |
| `rtk proxy cargo test --no-fail-fast` | **1929 passed, 1 failed, 13 ignored**, across 48 test binaries |
| `rtk proxy cargo clippy -- -D warnings` | **exit 0** |

**Baseline for comparison**, measured at `0dde2b4` before any edit: **1923 passed, 1 failed, 13 ignored**.

So **+6 passed** — exactly the six tests added (3 in task 1, 3 in task 2) — with the failure count unchanged at 1 and the ignored count unchanged at 13.

**`--no-fail-fast` was used throughout**, as required: a plain `cargo test` stops at the first failing binary and reports an unchanged count no matter what was added.

**The single failure is pre-existing and unrelated:** `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. It is a deliberate *schedule* assertion that fires when the installed git moves off the version its constants were derived against — `installed "git version 2.53.0"` vs `derived against "git version 2.43.0"`. It fails identically at the base commit.

**A correction to the plan's expectation:** the plan stated the pre-existing failure set was "the two documented `driver_reattach` failures". That is stale. The actual pre-existing failure set at `0dde2b4` is the single `envelope::policy` git-version pin; `driver_reattach` is green.

Per-binary results at the task gates:

```
tests/executor_lifecycle.rs   ok. 14 passed;   0 failed;  0 ignored
tests/executor_transport.rs   ok.  9 passed;   0 failed;  0 ignored
tests/driver_kill_startup.rs  ok.  1 passed;   0 failed;  0 ignored
tests/spawn_seam_guard.rs     ok. 38 passed;   0 failed;  0 ignored
--lib executor::              ok. 109 passed;  0 failed;  0 ignored
--lib driver::run             ok. 34 passed;   0 failed;  0 ignored
```

## Reported, not fixed

### 1. The stderr / `events_rx` pre-gate backpressure hazard — UNFIXED

Pre-gate, nothing drains `events_rx` — it is local to `start_run` until the handle is returned — so `read_stderr`'s `tx.send().await` on the 8192-slot channel can park. Because `last_line_at` is stamped *before* that send, a child that floods stderr before announcing itself can freeze the idle clock. Buffered stderr is also dropped when `events_rx` is dropped on the error path.

**`prompt_release_grace` shrinks the exposure window from "until the idle cap" (15 minutes) to "until the grace expires plus init latency" (~5 seconds). That is the only relief this change gives it; it does not close it.** Closing it needs a sink that survives the error path — the journal, in `run.rs` — which is not cheap and was explicitly out of scope.

### 2. The 15-minute residual for a genuinely mute child — UNFIXED, deliberately

`ExecutionOptions::default().idle_cap` is **unchanged at `Duration::from_secs(15 * 60)`**, asserted by `the_default_prompt_release_grace_sits_strictly_between_zero_and_the_idle_cap`.

After this fix, a child that is genuinely mute — one that neither announces itself nor responds to the released prompt — still takes the full 15 minutes to trip. **The idle cap was deliberately not shrunk and no second startup bound was added beyond the grace.** The two answer different questions: the grace asks "has the child introduced itself", the idle cap asks "has this run gone silent". A single number cannot mean both, and `iteration_options` inherits the idle cap from `Default` on purpose.

What the fix *does* change for that case is legibility: the failure is now labelled `stalled` with a journalled reason, rather than `spawn_failed` with nothing on disk.

### 3. Two out-of-scope test findings

Logged in `deferred-items.md` in this directory, **not fixed**:

- `driver::run::tests::the_current_group_agrees_with_the_proc_parse` **flaked once in five full runs** (`getpgrp()` returned 2712294, the `/proc/<pid>/stat` parse returned 2716792). It did not reproduce in three consecutive full `--lib` runs or in a second full `cargo test --no-fail-fast` run. Not attributable to this change — it cross-checks two readings of the *test harness's own* process group and has no dependency on the executor or the label mapping. Worth investigating on its own terms, because the parse it checks is what `kill::resolve_signal_target` uses.
- `cargo clippy --all-targets -- -D warnings` reports **4 pre-existing errors** in `src/project_creator.rs`'s test module (`bool_assert_comparison`, `cmp_owned`). The gate this task specifies is `cargo clippy -- -D warnings`, which does not compile test targets and exits 0.

## Task 2: shipped, not dropped

Task 2 was marked explicitly droppable. **It shipped.** Task 1 did not balloon — it landed inside its estimate — so the secondary work was done in full:

- `SpawnError::StalledBeforeInit { idle_for }` and `SpawnError::TimedOutBeforeInit { after }` added beside `InitNeverObserved`, with `Display` arms; `source()` returns `None` for both via the existing wildcard.
- `Coordinator::run`'s post-loop gate answer now chooses among the three from the `breach` local already in scope. The comment explains why: the supervisor sends the correct `RunOutcome` on `outcome_tx`, but that receiver lives in an `ExecutionHandle` this path never returns, so the gate channel is the caller's only surface and must carry the same fact.
- `driver::run::spawn_failure_label` maps `&SpawnError` onto a terminal label, replacing the hard-coded `"spawn_failed"`. It reuses `outcome_label`'s four existing words and invents none; the render layer already reads all four.
- `driver::run::spawn_failure_diagnostic` supplies a closed `(code, detail)` pair per variant, journalled as `JournalEvent::Diagnostic` **before** `finish_run`, following the terminate-shutdown site's pattern (log the error KIND on failure, still attempt the terminal record). **The rendered error is never journalled** — `CapabilityError` interpolates CLI-supplied strings into a surface the TUI paints. The mechanical gate confirming this (`err.to_string()` on no non-comment line of `run.rs`) passes.

**Nothing from task 2 stays open.** The one thing worth noting for a future reader: `spawn_failure_label` re-labels only the two supervisor breaches and the capability refusal, because only those three describe a child that actually launched. The other five `SpawnError` variants keep `spawn_failed`, and that fallback is asserted rather than assumed.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 2 — Missing correctness] Two stale claim sites beyond the plan's inventory**
- **Found during:** Task 1, § Corrections sweep
- **Issue:** The plan's inventory listed eleven sites. A grep sweep for the claim's wording across `src/` and `tests/` found two more asserting the pre-gate ordering unconditionally: `src/executor/gate.rs`'s `REQUIRED_CAPABILITIES` doc ("Capabilities a run requires **before any turn may begin**") and `src/executor/claude.rs`'s `feed()` test-helper doc.
- **Fix:** Both corrected to name the condition. In this codebase a stale guarantee is a correctness defect by its own stated standard, and the task's constraint was "leave nothing asserting the unconditional form anywhere in `src/` or `tests/`".
- **Files modified:** `src/executor/gate.rs`, `src/executor/claude.rs`
- **Verification:** `rtk proxy grep -rn 'zero tokens|zero quota|no turn ever|before any turn' src/ tests/` — every surviving hit is inside a per-arm conditional
- **Committed in:** `114de68`

**2. [Rule 3 — Blocking] Test written against variant names that do not exist**
- **Found during:** Task 2
- **Issue:** The label-mapping test was drafted against `CapabilityError::VersionTooOld { reported, minimum }` and `TerminalState::from_label(label)`. The real API is `CapabilityError::VersionBelowFloor { observed, floor }` and `from_label(Option<&str>)`.
- **Fix:** Corrected to the real signatures before the RED observation was taken.
- **Files modified:** `src/driver/run.rs`
- **Committed in:** `2fb6592`

**3. [Rule 3 — Blocking] Doc comment on a function parameter**
- **Found during:** Task 1
- **Issue:** `///` on `handle_item`'s new `prompt_released` parameter — `error: documentation comments cannot be applied to function parameters`.
- **Fix:** Changed to `//`. Content unchanged.
- **Files modified:** `src/executor/claude.rs`
- **Committed in:** `114de68`

---

**Total deviations:** 3 auto-fixed (1 × Rule 2, 2 × Rule 3)
**Impact on plan:** None on scope. The Rule 2 fix widens the correction sweep by two sites, which is the plan's own stated intent rather than scope creep. No architectural decision was needed, so no Rule 4 checkpoint was raised.

## Issues Encountered

**The plan's stated pre-existing failure set was wrong.** It expected "two documented `driver_reattach` failures"; the actual baseline at `0dde2b4` is one failure, in `envelope::policy`, driven by the installed git version. Resolved by measuring the baseline directly before making any edit, rather than trusting the plan's number — which is also what makes the "+6 passed, failures unchanged" claim above verifiable.

**One test flake.** See "Reported, not fixed" §3. Characterised across five full runs rather than fixed, per the scope boundary.

## Known Stubs

None. No hardcoded empty values, placeholder text, TODOs or FIXMEs were introduced, and no component was left without a data source.

## Threat Flags

None. The change introduces no new network endpoint, auth path, file access pattern or schema at a trust boundary beyond what the plan's `<threat_model>` already registered.

The registered mitigations were applied:

- **T-UQQ-01** (information disclosure — the command reaches a child the D-07/D-08 guards have not yet judged): bounded three ways as planned. The pre-prompt refusal is preserved wherever the CLI announces in time; the refusal still fires and still tears the group down on the late arm; and the residual is written into the corrected doc comments rather than left implicit. Pinned by `a_refused_run_on_a_late_announcing_cli_has_already_written_the_prompt`.
- **T-UQQ-02** (denial of service — pre-gate teardown during the now-longer-lived startup window): the pre-gate `pgid` publication and the `biased` terminate-arm ordering are byte-unchanged; `tests/driver_kill_startup.rs` and `the_spawn_observer_publishes_the_agent_pgid_even_when_the_gate_never_opens` are both green.
- **T-UQQ-03** (undrained `events_rx`): accepted and disclosed, as planned. See "Reported, not fixed" §1.
- **T-UQQ-04** (tampering — journal `Diagnostic` detail): closed driver-authored vocabulary; the rendered error is never journalled, confirmed by the mechanical non-comment gate.
- **T-UQQ-SC**: no package installs, no new dependencies. `git diff --name-only 0dde2b4..HEAD -- Cargo.toml Cargo.lock` is empty.

## Self-Check: PASSED

- `tests/fixtures/fake-claude-late-init.sh` — FOUND, mode 775 (executable)
- `src/executor/mod.rs`, `src/executor/claude.rs`, `src/executor/gate.rs`, `src/error.rs`, `src/driver/run.rs`, `src/ui/screens/driver.rs`, `tests/executor_lifecycle.rs`, `tests/executor_transport.rs`, `tests/fixtures/fake-claude-silent.sh` — all FOUND and modified
- Commit `114de68` — FOUND in `git log`
- Commit `2fb6592` — FOUND in `git log`
- `git rev-list --count 0dde2b40c17bb12bfd75cdf418902993ce30d078..HEAD` = **2**, matching the `commits: 2` recorded in frontmatter
- No files deleted by either commit (`git diff --diff-filter=D` empty for both)
