---
phase: quick-260917-nhc
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/driver/run.rs
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
autonomous: true
requirements:
  - DEFER-21-CURRENT-GROUP-PROC-PARSE-FLAKE

estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Under the already-built contended reproducer (a non-job-leader Python parent spawning the compiled lib-test binary), which produced 1 failure in 20 runs on the untouched tree, at least 60 runs complete with ZERO occurrences of `driver::run::tests::the_current_group_agrees_with_the_proc_parse` in the failures list (C-1)."
    - "The `assert_eq!(from_syscall, from_proc, ...)` and its failure message survive BYTE-IDENTICAL — `git diff` on `src/driver/run.rs` removes not one character of that assertion (C-1)."
    - "A `None` from `kernel_process_group` is still an immediate, loud failure under the verbatim `this process's own /proc/<pid>/stat is readable` wording. It is never a retry condition; the ONLY retry condition is `the group moved inside the observation window` (C-1)."
    - "Deadline expiry PANICS naming this condition by name — it never returns a fabricated pair, never skips, and never lets a moving group stand in for the agreement being asserted (C-1)."
    - "No `#[ignore]`, no thread-count flag, no fixed pre-assertion sleep, no loosened or approximate comparison, and no call to `establish_own_group` anywhere in the `mod tests` region (C-1)."
    - "`src/driver/liveness.rs`, `src/driver/mod.rs` and every non-test line of `src/driver/run.rs` are byte-identical before and after — the production `/proc` parse and `getpgrp` wrapper are EXONERATED and untouched (C-1, C-2)."
    - "The module header records the mechanism: `setpgid(0, 0)` resolves against the THREAD-GROUP LEADER, so a libtest worker thread calling it through `drive` -> `execute_run` moves the whole shared binary's process group; names `src/driver/mod.rs:1834` and its 7 `LOOK_ALIKE_PAIRS` iterations as the mover; records how it was established (strace + the live `/proc` pgrp-transition watch) and the measured rates (0/39 direct, 1/20 under a non-leader parent) (C-2)."
    - "The module header states that the hazard `current_group`'s doc (src/driver/run.rs:1053-1059) and the test's own comment call HYPOTHETICAL is in fact ACTUAL, and records the two things deliberately NOT done: not stopping the mover, and not calling `establish_own_group` from this test (C-2)."
    - "Phase 21's deferred-items.md closes the rows naming this test append-only, quoting VERBATIM the two superseded sentences from `260908-uqq/deferred-items.md` section 2 before correcting each, with `git diff --numstat` proving ZERO deletions (C-3)."
  artifacts:
    - src/driver/run.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
  key_links:
    - "The seqlock read is the whole fix: `current_group()` BEFORE, the `/proc` parse BETWEEN, `current_group()` AFTER. If the trailing read is dropped or reordered the helper proves nothing, because the straddle it exists to detect happens precisely in that interval."
    - "`setpgid(0, 0)` is one-way and idempotent — it always sets the group to the caller's own pid — so the group cannot move away and back inside one window. That is what makes `before == after` sufficient rather than merely suggestive. If a future change introduces a second, different `setpgid` target in this binary, this reasoning has to be re-derived."
    - "`tests/spawn_seam_guard.rs` treats the bare token `process_group(` as a process-SPAWN marker with a left word boundary that accepts `::` and identifier characters. The existing import alias `kernel_process_group` is what keeps this module off the spawn allowlist. Any new helper name must NOT reintroduce that bare token."
    - "The deferred-items closure is the only place a future reader learns that `260908-uqq` section 2's `No test in the tree calls establish_own_group()` is a false negative. If the correction is not written down, the next investigator re-derives the whole chain — or worse, trusts the wrong sentence and stops."
---

<objective>
Make `driver::run::tests::the_current_group_agrees_with_the_proc_parse` deterministic
against a cross-test `setpgid` race, and correct the recorded diagnosis, which is wrong.

Purpose: this is the last flake standing between HEAD and the v1.7.1 publish gate. The
`driver_reattach` pair was closed by quick `260917-k6y`; the `envelope_tracer` ETXTBSY race by
quick `260917-lkg`; this one is what run 7 of `260917-lkg`'s nine-run sweep reported rather than
absorbed. The fix is a SYNCHRONISATION fix at the observation site, not a tolerance: the equality
the test asserts is exact, is correct, and stays exact.

The finding is that the hazard the test's own comment names as hypothetical is ACTUAL. A sibling
libtest thread reaches `establish_own_group()` transitively through `drive` -> `execute_run`,
seven times per run, and on Linux `setpgid(0, 0)` resolves against the thread-group leader — so
that worker thread moves the process group of the ENTIRE shared test binary. The two observations
in this test straddle the move.

Output: a bounded, condition-specific seqlock-style read in `src/driver/run.rs`'s test module; a
module-header case record; and an append-only closure in phase 21's `deferred-items.md` that
supersedes the two false sentences in `260908-uqq/deferred-items.md` section 2 by quoting them.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

@src/driver/run.rs
@src/driver/liveness.rs
@tests/driver_reattach.rs
@tests/envelope_tracer.rs
@.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
@.planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/deferred-items.md
</context>

<diagnosis_is_closed>
**The diagnosis is complete, evidence-backed, and NOT to be re-investigated.** It was established
before planning with `strace` and a live `/proc` watcher. Re-deriving it burns the context this
plan needs for the fix. The eight findings, each with the evidence that established it:

1. **Test-side race, not a production defect.** `liveness::process_group`'s `/proc/<pid>/stat`
   parse (`src/driver/liveness.rs:234-241`) is correct. `current_group()`
   (`src/driver/run.rs:1060`) is a bare `getpgrp()`. Neither misreads. They disagree only because
   the group genuinely MOVES between the two reads.

2. **The mover is production code reached by another lib test.** `src/driver/mod.rs:1834`, the
   "visible twin, driven end to end" arm of
   `driver::tests::a_target_phase_that_renders_as_another_is_refused_at_the_seam`, calls
   `drive(visible_args, &visible_config)` once per `crate::test_support::LOOK_ALIKE_PAIRS` entry.
   Each call reaches `src/driver/run.rs:2553` `let pgid = establish_own_group();`, which is
   `src/driver/run.rs:1085` `rustix::process::setpgid(None, None)` — `setpgid(0, 0)`.

3. **Linux resolves `setpgid(0, 0)` against the THREAD-GROUP LEADER**, not the calling thread. A
   libtest worker thread calling it therefore moves the WHOLE shared binary's process group.
   Evidence: `strace -f -e trace=setpgid,execve` on the lib-test binary caught exactly 7
   `setpgid(0, 0) = 0` calls from one TID that never `execve`s (a thread, not a forked child),
   and the main pid's own strace stream contains no `setpgid` at all. Correlating the trace
   against libtest's fd-1 writes under a single-threaded run put all 7 inside
   `driver::tests::a_target_pha…` — the 7 `LOOK_ALIKE_PAIRS` iterations.

4. **The move was observed directly.** A Python parent that is not its own job leader spawned the
   lib-test binary and polled `/proc/<child>/stat`; the child's pgrp transitioned
   `[2751890, 2751905]` — from the parent's inherited group to the child's own pid — mid-run.

5. **The test straddles that move.** `src/driver/run.rs:4483-4485` reads `current_group()` and
   then `kernel_process_group(std::process::id())`. A move landing between those two statements
   leaves `from_syscall` on the old inherited group and `from_proc` on the process's own pid.
   Both historical failure records match that signature exactly: `left: 2759114 / right: 2772348`
   (reproduced by the coordinator; right == the test binary's own pid) and the archived
   `left: 2712294 / right: 2716792` and `getpgrp() 300296 vs the /proc pgrp field 300318`. In
   every case the `/proc` value is the LARGER one, because a freshly-created group id is the
   process's own pid.

6. **Why it never reproduces from an interactive shell.** Launched directly, shell job control has
   already made the binary its own group leader, so `setpgid(0, 0)` is a no-op and there is no
   window. Launched by cargo — or any non-job-leader parent — the binary inherits its parent's
   group and the window is real. MEASURED: **0 failures in 39 direct runs**, versus **1 failure
   in 20 runs** under a non-leader parent.

7. **The recorded diagnosis in `.planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/deferred-items.md`
   section 2 is WRONG, and correcting it is part of the finding.** Two sentences are false. The
   first: *"No test in the tree calls `establish_own_group()`, so the cross-test `setpgid` hazard
   the test's own doc names is not the mechanism."* — a false negative; no test calls it
   DIRECTLY, but at least one reaches it TRANSITIVELY through `drive` -> `execute_run`, seven
   times per run. The second: *"`getpgrp()` cannot return a stale value, so the suspect is the
   `/proc/<pid>/stat` field parse under the load of many test binaries running concurrently."* —
   the parse is EXONERATED; the first clause is true and the conclusion drawn from it is wrong.

8. **The test's own comment understates it.** `src/driver/run.rs:4480-4482` names the hazard as
   hypothetical — *"Deliberately NOT `establish_own_group()`: `setpgid` in a shared test binary
   would move the harness's own process group, and with it every other test in this process."* —
   and `current_group`'s doc at `src/driver/run.rs:1053-1059` says the same in the conditional.
   It is in fact ACTUAL, caused by a different test. **That correction is the headline of the
   write-up.**
</diagnosis_is_closed>

<hard_constraints>
Non-negotiable. A violation of any one of these makes the change wrong even if every gate is
green.

- The `assert_eq!(from_syscall, from_proc, "…")` and its verbatim failure message survive
  BYTE-IDENTICAL. The equality is exact and stays exact.
- No `#[ignore]`. No `--test-threads` flag. No deleted or loosened assertion. No widening of the
  equality into an approximate or "close enough" comparison. No bare `sleep` before the assertion.
- The `.expect("this process's own /proc/<pid>/stat is readable")` stays. A `None` from the parse
  is an immediate failure, NEVER a retry condition. The ONLY retry condition is "the group moved
  inside the observation window".
- Do NOT call `establish_own_group()` from this test.
- No change to `src/driver/liveness.rs`. No change to `src/driver/mod.rs`. No change to any
  non-test line of any production file. The `/proc` parse and the `getpgrp` wrapper are correct.
- Do not touch
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Do NOT bump `Cargo.toml`, run a release, or create a tag. v1.7.1 is a separate step.
- Do NOT touch `.github/`.
- Do NOT fix or absorb the known, deliberately-open BrokenPipe race in `tests/envelope_tracer.rs`'s
  `run_stub` (~1 in 800 contended runs). If it appears in a verification run it is REPORTED,
  never absorbed.
- Do NOT change the ordering of `establish_own_group()` inside `execute_run`. Considered and
  rejected: production `drive` is a dedicated process where the group move is correct and
  intended, and re-ordering production code to accommodate a shared test binary is the wrong
  trade. This is a deliberate, documented non-goal and is flagged for later audit.
</hard_constraints>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: the seqlock read — make the two observations provably straddle no move</name>
  <files>src/driver/run.rs</files>
  <read_first>
    `src/driver/run.rs:1049-1092` (`current_group`, `establish_own_group` — READ ONLY, neither is
    edited), `src/driver/run.rs:3777-3792` (the test module opening and the load-bearing
    `kernel_process_group` import alias, whose doc explains why the alias exists),
    `src/driver/run.rs:4472-4493` (the test), and `tests/driver_reattach.rs:318-340`
    (`live_within` / `gone_within` — the deadline-plus-25ms poll shape this must reuse exactly).
  </read_first>
  <behavior>
    Fail-first is ALREADY MEASURED on the untouched tree and must be reproduced once, not
    reinvented — see `<verification>`. Baseline at HEAD: **1 failure in 20 runs** under the
    non-job-leader parent.

    - Common case (the group is not moving): the helper returns on its FIRST iteration, adds no
      measurable wall time, and the test behaves exactly as it does today.
    - Straddle case (a sibling thread's `setpgid(0, 0)` lands between the two reads): the trailing
      `current_group()` disagrees with the leading one, the observation is DISCARDED, and the loop
      retries after a 25ms sleep. Nothing is asserted on a discarded observation.
    - Unreadable `/proc` case: the `.expect(…)` fires immediately, under its verbatim wording. It
      is not caught, not retried, not converted into a loop-continue.
    - Expiry case: the deadline passes with every window straddled, and the helper PANICS naming
      the condition — e.g. `the process group kept moving across every observation window` —
      together with the limit and the attempt count. It never returns a fabricated pair.
  </behavior>
  <action>
    Edit the `mod tests` region of `src/driver/run.rs` ONLY (line 3778 to EOF). Add a small named
    helper immediately above `the_current_group_agrees_with_the_proc_parse`, with a `///` doc
    comment, because this file's convention is a named function with a doc rather than an inline
    block — `JOURNAL_BUDGET` at `src/driver/run.rs:4495-4502` is the local precedent for
    "mirror the reasoning where it happens".

    Name the helper in the `*_within` idiom `tests/driver_reattach.rs` established and
    `tests/envelope_tracer.rs` reused — something of the shape
    `group_observations_without_a_move_within(limit: Duration) -> (u32, u32)`. The name MUST NOT
    contain the bare token that `tests/spawn_seam_guard.rs` scans for as a process-spawn marker;
    the import alias `kernel_process_group` at `src/driver/run.rs:3791` exists for exactly that
    reason and its doc says so. Take `limit` as a parameter rather than baking a constant, and
    pass `Duration::from_secs(30)` inline at the single call site — the same inline duration
    `tests/driver_reattach.rs` uses at its five `live_within` call sites, with no new named
    constant. `Duration` is already imported at `src/driver/run.rs:14`; reach `Instant` through
    the fully-qualified `std::time::Instant::now()`, as `tests/driver_reattach.rs:320` does,
    rather than adding an import.

    The body is a seqlock-style read. Compute a deadline as `std::time::Instant::now() + limit`.
    While `std::time::Instant::now() < deadline`: read `current_group()` into a leading binding;
    read `kernel_process_group(std::process::id())` and apply the `.expect(…)` with its message
    UNCHANGED; read `current_group()` again into a trailing binding. If leading equals trailing,
    return the leading value paired with the `/proc` value — no move can have straddled the parse,
    because the parse happened strictly between the two reads. Otherwise sleep
    `Duration::from_millis(25)` and loop, matching the house polling shape; a 25ms sleep INSIDE a
    poll loop is the idiom, a fixed pre-assertion sleep is the thing forbidden.

    Falling out of the loop is a `panic!` that names the condition, the limit and the attempt
    count, and says plainly that no window was ever free of a move. Expiry must be loud: a
    fabricated pair would make the surviving equality assert something the process never
    observed, which is exactly the failure mode the assertion exists to catch.

    The doc comment on the helper carries four facts and no more: what a straddle is; that
    `setpgid(0, 0)` always sets the group to the caller's own pid and is therefore one-way and
    idempotent, which is what makes `leading == trailing` sufficient rather than merely
    suggestive; that an unreadable `/proc` is NOT a retry condition and why; and a pointer to the
    module header section Task 2 writes, rather than duplicating the mechanism here — the
    `tests/envelope_tracer.rs` header's closing pointer to `deferred-items.md` is the convention.

    In the test body, replace the two observation statements with one destructuring call to the
    helper, keeping the binding names `from_syscall` and `from_proc` so the surviving `assert_eq!`
    is untouched down to its identifiers. The `assert_eq!` and its message are not retyped; leave
    those lines alone. Update the test's existing comment block so that the sentence calling the
    `setpgid` hazard hypothetical now states it is real and points at the header section and at
    `src/driver/mod.rs:1834`; keep the "Deliberately NOT `establish_own_group()`" decision itself,
    because the decision stands and only its premise strengthens.

    Change nothing outside the `mod tests` region. `current_group`, `establish_own_group`,
    `execute_run` and `src/driver/liveness.rs` are correct and are not opened.
  </action>
  <verify>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; mkdir -p "$SCRATCH"; rtk proxy cargo build --tests 2>&1 | tail -5</automated>
    <automated>rtk proxy cargo test --lib driver::run::tests::the_current_group_agrees_with_the_proc_parse -- --exact 2>&1 | tail -5   # 1 passed</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; git diff -U0 -- src/driver/run.rs > "$SCRATCH/t1.diff"; echo "git exit=$?"; grep '^-[^-]' "$SCRATCH/t1.diff" | grep -c 'must name the same group'   # git exit=0 and 0 — the assertion message was not removed or retyped</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; grep '^-[^-]' "$SCRATCH/t1.diff" | grep -c 'is readable'   # 0 — the expect wording was not removed or retyped</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; git diff --numstat -- src/driver/liveness.rs src/driver/mod.rs Cargo.toml .github/ > "$SCRATCH/t1.numstat"; echo "git exit=$?"; wc -l < "$SCRATCH/t1.numstat"   # git exit=0 and count 0 — production parse, mover and release surface untouched</automated>
    <automated>awk 'NR>=3778' src/driver/run.rs | grep -v '^[[:space:]]*//' | grep -c 'establish_own_group('   # 0 — no test calls it, comments excluded so the retained decision note does not self-invalidate</automated>
    <automated>awk 'NR>=3778' src/driver/run.rs | grep -v '^[[:space:]]*//' | grep -cE '#\[ignore\]|test-threads'   # 0</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; grep -E '^[-+][^-+]' "$SCRATCH/t1.diff" | grep -cE 'fn current_group|fn establish_own_group|rustix::process::setpgid'   # 0 — neither production fn was edited</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; LIB=$(rtk proxy cargo test --lib --no-run --message-format=json 2>/dev/null | jq -r 'select(.profile.test == true) | .executable' | tail -1); echo "LIB=$LIB"; python3 "$SCRATCH/repro.py" "$LIB" 60 2>&1 | tail -3   # TOTAL 0/60</automated>
  </verify>
  <done>
    A doc-commented helper in the `*_within` idiom returns a `(getpgrp, /proc)` pair that provably
    straddles no `setpgid`; the surviving `assert_eq!` and the `.expect` are byte-identical in the
    diff; no `#[ignore]`, thread-count flag, `establish_own_group` call or pre-assertion sleep
    exists in the test region; `src/driver/liveness.rs`, `src/driver/mod.rs`, `Cargo.toml` and
    `.github/` are untouched; and at least 60 contended runs under the non-job-leader parent
    report `TOTAL 0/60` against a measured HEAD baseline of 1/20.
  </done>
</task>

<task type="auto">
  <name>Task 2: the module header case record</name>
  <files>src/driver/run.rs</files>
  <read_first>
    `src/driver/run.rs:1-11` (the existing header, which is short and `//!`-shaped),
    `tests/envelope_tracer.rs:1-54` (the ETXTBSY header section — the documentation VOICE and
    structure to match: a ruled separator, a named finding, the evidence, the eliminations, and a
    closing pointer to `deferred-items.md` for measurements deliberately not duplicated).
  </read_first>
  <action>
    Append a new `//!` section to `src/driver/run.rs`'s module header, after the existing
    `ClaudeExecutor` / `JournalRun` paragraph at line 10 and before the `use` block. Match the
    `tests/envelope_tracer.rs` header's voice: the correction stated as the finding, then the
    evidence that established it, then what was deliberately not done.

    Record, in this order:

    The headline correction — the cross-test `setpgid` hazard that `current_group`'s own doc at
    `src/driver/run.rs:1053-1059` and the test comment both frame in the conditional is ACTUAL,
    and it is caused by a different test in this same binary.

    The mechanism — on Linux `setpgid(0, 0)` resolves against the THREAD-GROUP LEADER, not the
    calling thread, so a libtest worker thread that reaches it moves the process group of the
    entire shared test binary, and every other test in that process with it.

    The mover, named — `src/driver/mod.rs:1834`, the "visible twin, driven end to end" arm of
    `driver::tests::a_target_phase_that_renders_as_another_is_refused_at_the_seam`, calls `drive`
    once per `crate::test_support::LOOK_ALIKE_PAIRS` entry, and each call reaches
    `execute_run`'s `establish_own_group()` — seven times per run. No test calls it directly,
    which is why a direct-call search returns a false negative.

    How it was established — `strace -f -e trace=setpgid,execve` on the lib-test binary caught
    exactly 7 `setpgid(0, 0) = 0` calls from a single TID that never `execve`s, proving a thread
    rather than a forked child, with no `setpgid` at all in the main pid's own stream; correlation
    against libtest's fd-1 writes under a single-threaded run put all 7 inside the look-alike
    test. Separately, a parent that is not its own job leader polled `/proc/<child>/stat` on a
    live run and watched the pgrp transition from the inherited group to the child's own pid
    mid-run.

    Why the launch context decides whether it reproduces — launched from an interactive shell, job
    control has already made the binary its own group leader and `setpgid(0, 0)` is a no-op, so
    there is no window; launched by cargo, or by any non-job-leader parent, the binary inherits
    its parent's group and the window is real. MEASURED: 0 failures in 39 direct runs against
    1 failure in 20 under a non-leader parent. Record this explicitly, because "it passes when I
    run it by hand" is the exact observation that sent the previous two investigations wrong.

    What is EXONERATED — `liveness::process_group`'s `/proc/<pid>/stat` field parse and the
    `getpgrp()` wrapper both read correctly. They disagreed only because the group genuinely moved
    between the two reads. Say this in as many words, because the previously recorded suspicion
    fell on the parse.

    The failure signature, so a future reader recognises it instantly — the `/proc` value is
    always the LARGER of the two, because a freshly-created group id is the process's own pid.

    What was deliberately NOT done, and why. Not stopping the mover: that would mean either a
    test-only seam in production `execute_run` or deleting a real end-to-end control arm, and
    production `drive` is a dedicated process where the group move is correct and intended —
    re-ordering production code to accommodate a shared test binary is the wrong trade. Not
    calling `establish_own_group()` from this test: the original decision stands, and only its
    premise strengthened from hypothetical to observed.

    Close with a pointer to the `# QUICK 260917-nhc` section of
    `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` for the full
    measurement tables, and state that they are deliberately not duplicated here — the same
    closing move `tests/envelope_tracer.rs:49-53` makes.

    Header prose only. Do not edit any code line in this task.
  </action>
  <verify>
    <automated>rtk proxy cargo build 2>&1 | tail -3</automated>
    <automated>awk 'NR<=80' src/driver/run.rs | grep -c 'thread-group leader'   # >= 1</automated>
    <automated>awk 'NR<=80' src/driver/run.rs | grep -cE 'LOOK_ALIKE_PAIRS|mod\.rs:1834'   # >= 1, the mover is named</automated>
    <automated>awk 'NR<=80' src/driver/run.rs | grep -cE '0 (failures )?in 39|39 direct|1 (failure )?in 20|1/20'   # >= 1, the measured rates are on the record</automated>
    <automated>awk 'NR<=80' src/driver/run.rs | grep -c 'strace'   # >= 1</automated>
    <automated>awk 'NR<=80' src/driver/run.rs | grep -c '260917-nhc'   # >= 1, the pointer to deferred-items</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; git diff -U0 -- src/driver/run.rs > "$SCRATCH/t2.diff"; echo "git exit=$?"; grep '^+[^+]' "$SCRATCH/t1.diff" | grep -v '^+//!' > "$SCRATCH/t1.code"; grep '^+[^+]' "$SCRATCH/t2.diff" | grep -v '^+//!' > "$SCRATCH/t2.code"; diff "$SCRATCH/t1.code" "$SCRATCH/t2.code" && echo "NON-DOC ADDITIONS UNCHANGED"   # this task added header prose only; every non-`//!` added line is byte-identical to what Task 1 left</automated>
    <automated>rtk proxy cargo clippy -- -D warnings 2>&1 | tail -3</automated>
  </verify>
  <done>
    The module header carries a case-record section in the file's documentation voice naming the
    thread-group-leader mechanism, the mover and its 7 iterations, the strace and `/proc`-watch
    evidence, the 0/39-versus-1/20 rates and why launch context matters, the exoneration of both
    readers, the larger-value failure signature, and the two documented non-goals — closing with a
    pointer to `deferred-items.md` rather than duplicating the tables. No code line changed.
  </done>
</task>

<task type="auto">
  <name>Task 3: the gate sweep, then the append-only paper record</name>
  <files>.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md</files>
  <read_first>
    `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md:2462-2520`
    (the `# QUICK 260917-lkg` section opening — the append-only convention, the italic
    "canonical entry" note, and the verbatim-quote-then-correct move) and `:2580-2635` (the
    nine-run table whose row 7 reported this test, and the closing sentence that named it out of
    scope). Also
    `.planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/deferred-items.md:29-51`
    — the section being superseded. That file is NOT edited.
  </read_first>
  <action>
    Run the gates FIRST, then write them down. The write-up records measurements, so it cannot be
    drafted before they exist.

    Gate sweep, in order, with every result recorded verbatim as it comes back:
    ten `rtk proxy cargo test --no-fail-fast` runs, recording passed / failed / ignored and the
    failing test NAMES for each run (~75 seconds per run); `./scripts/pre-tag-check.sh`, reported
    gate by gate; `rtk proxy cargo clippy -- -D warnings`. Never plain `cargo test`, and never
    pipe rtk output through `grep` — rtk's filtering strips `test result:` lines, so a filtered
    run lies about the counts. The ONLY acceptable recurring failure is
    `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
    which is environmental (local git 2.53.0 against constants re-derived at 2.55.0, green on
    CI's runner). If the known BrokenPipe race in `tests/envelope_tracer.rs`'s `run_stub` appears,
    report it with its run number and absorb nothing.

    Then append a new top-level section `# QUICK 260917-nhc — appended 2026-09-17, append-only`
    at the END of
    `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md`, following
    the `# QUICK 260917-lkg` section's structure exactly. ZERO DELETIONS is a hard requirement and
    is checked with `git diff --numstat`.

    The section carries, in this order:

    An italic canonical-entry note saying that the dated pointers placed above point here, that no
    line above was edited or deleted, and that where a pointer corrects an earlier one it quotes
    the earlier text verbatim and says what changed.

    The mechanism, headed by the correction: the hazard is real, `setpgid(0, 0)` resolves against
    the thread-group leader, the mover is `src/driver/mod.rs:1834` at 7 `LOOK_ALIKE_PAIRS`
    iterations per run, and both readers are exonerated.

    **The supersession, which is the load-bearing part.** Quote VERBATIM, as block quotes, each of
    the two false sentences from `260908-uqq/deferred-items.md` section 2 — the one beginning
    *"No test in the tree calls `establish_own_group()`…"* and the one beginning *"`getpgrp()`
    cannot return a stale value…"* — attributed to that file and section, and put the correction
    directly beneath each. For the first: no test calls it DIRECTLY, but one reaches it
    TRANSITIVELY through `drive` -> `execute_run`, seven times per run, so the search that
    produced that sentence returned a false negative. For the second: the first clause is true and
    the conclusion drawn from it is wrong — the parse is correct and the group moved. Do NOT edit
    `260908-uqq/deferred-items.md` itself; it is superseded from here.

    A third supersession for the test's own comment and `current_group`'s doc, both of which named
    the hazard in the conditional: quote the *"Deliberately NOT `establish_own_group()`…"*
    sentence and record that the decision stands while its premise moved from hypothetical to
    observed.

    A pointer placed at the two dated rows above that name this test — line 2585's run-7 table row
    and line 2632's *"as is run 7's `the_current_group_agrees_with_the_proc_parse`"* — each
    quoting the line it points from and saying it is now closed here. These are ADDITIONS
    adjacent to those lines, not edits of them.

    The measurements: the fail-first baseline (1 failure in 20 runs under the non-job-leader
    parent, against 0 in 39 direct runs), the post-fix contended result from Task 1 with its run
    count, the ten full-suite runs as a table of run / passed / failed / ignored / failing test
    names, the `pre-tag-check.sh` result gate by gate, and the clippy result.

    A `### What this does NOT close` subsection: `src/envelope/hooks.rs`-style disclaimers do not
    apply here, but three things do — the BrokenPipe race in `tests/envelope_tracer.rs`'s
    `run_stub` remains OPEN and untouched; the environmental git-version-constants failure remains
    out of scope; and the `establish_own_group()` ordering inside `execute_run` was considered and
    deliberately NOT changed, with the reason (production `drive` is a dedicated process where the
    move is correct and intended) recorded as a deliberate non-goal and flagged for later audit.

    A closing line stating that `Cargo.toml`, the crate version, `.github/` and all tags are
    untouched, and that v1.7.1 is a separate step.
  </action>
  <verify>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; for i in $(seq 1 10); do rtk proxy cargo test --no-fail-fast > "$SCRATCH/full-$i.txt" 2>&1; echo "run $i exit=$?"; done   # never plain cargo test, never piped through grep</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; for i in $(seq 1 10); do echo "== run $i =="; awk '/^test result:/ {print} /^failures:$/,0 {print}' "$SCRATCH/full-$i.txt" | sort -u | tail -40; done   # read from the unfiltered captures, not from a live pipe</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; grep -l 'the_current_group_agrees_with_the_proc_parse' "$SCRATCH"/full-*.txt | wc -l   # 0 across all ten runs</automated>
    <automated>./scripts/pre-tag-check.sh; echo "pre-tag-check exit=$?"</automated>
    <automated>rtk proxy cargo clippy -- -D warnings 2>&1 | tail -3</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; git diff --numstat -- .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md > "$SCRATCH/t3.numstat"; echo "git exit=$?"; cat "$SCRATCH/t3.numstat"   # git exit=0; column 2 (deletions) MUST be 0</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; awk '{exit ($2==0)?0:1}' "$SCRATCH/t3.numstat"; echo "zero-deletions exit=$?"   # exit=0</automated>
    <automated>grep -c 'QUICK 260917-nhc' .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # >= 1</automated>
    <automated>grep -c 'No test in the tree calls' .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # >= 1, the first superseded sentence is quoted verbatim</automated>
    <automated>grep -c 'cannot return a stale value' .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # >= 1, the second superseded sentence is quoted verbatim</automated>
    <automated>SCRATCH=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad; git diff --numstat -- .planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/ src/ Cargo.toml .github/ > "$SCRATCH/t3-scope.numstat"; echo "git exit=$?"; grep -v 'src/driver/run.rs' "$SCRATCH/t3-scope.numstat" | wc -l   # git exit=0 and count 0 — 260908-uqq, other src files, Cargo.toml and .github untouched</automated>
  </verify>
  <done>
    Ten unfiltered full-suite runs are captured with per-run passed/failed/ignored and failing
    names, and none names this test; `pre-tag-check.sh` is reported gate by gate; clippy is clean;
    and a new append-only `# QUICK 260917-nhc` section closes the rows naming this test, quoting
    verbatim both superseded sentences from `260908-uqq` section 2 plus the test's own
    hypothetical-hazard comment before correcting each, recording the fail-first baseline and
    every post-fix measurement, naming the three things this does not close, and leaving
    `git diff --numstat` at ZERO deletions with `260908-uqq`, `Cargo.toml` and `.github/`
    untouched.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| libtest worker thread -> the shared test process's group (`setpgid`) | The only boundary this change touches, and both sides are test-owned. No untrusted input crosses it. |
| the recorded `pgid` in `run.json` -> `kill::resolve_signal_target` | NOT touched by this change, but it is what the asserted equality protects. Named so the register states what is being defended. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-nhc-01 | Tampering | the retry loop, `mod tests` in src/driver/run.rs | high | mitigate | The retry condition is EXCLUSIVELY "the leading and trailing `getpgrp()` disagree". An unreadable `/proc` still fires the unchanged `.expect`, and a genuine disagreement between a stable group and the parse still reaches the unchanged `assert_eq!`. The loop therefore cannot become a swallow-all that hides a real parse bug. Pinned by the diff gates in Task 1 and by the `<behavior>` block's unreadable-`/proc` case. |
| T-nhc-02 | Repudiation | the D-04 kill-switch witness | high | mitigate | Deadline expiry PANICS naming the moving-group condition rather than returning a fabricated pair. The test can never report an agreement it did not observe, which is the only way this change could weaken the witness that `kill::resolve_signal_target` depends on. |
| T-nhc-03 | Information disclosure | the module-header case record | low | accept | The header records pids and group ids from local reproduction runs. They are ephemeral local integers with no bearing outside the machine that produced them, and their value is that a future reader can recognise the `/proc`-is-larger signature. Accepted. |
| T-nhc-04 | Denial of service | the 30s bounded wait | low | accept | A permanently moving group costs 30s of wall time on a genuine failure and nothing on a passing run — the observed window closes after the mover's last of 7 iterations and never reopens, because `setpgid(0, 0)` is idempotent. Accepted, matching `tests/driver_reattach.rs`'s existing `Duration::from_secs(30)` call sites. |
| T-nhc-05 | Elevation of privilege | `execute_run`'s `establish_own_group()` | medium | accept | The production call stays exactly where it is. Moving or gating it would be a test-only seam in the code path that owns the kill switch's group identity, which is a larger risk than the flake. Recorded as a deliberate non-goal in Tasks 2 and 3 and FLAGGED FOR LATER AUDIT. |
| T-nhc-SC | Tampering | npm/pip/cargo installs | n/a | n/a | No package is installed or added by this plan. `Cargo.toml` is explicitly out of scope, so the package-legitimacy gate has no subject here. |
</threat_model>

<verification>

## The fail-first reproducer is ALREADY BUILT — reuse it, do not reinvent it

```
/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/2bcaed4e-c188-49b8-9c63-d99619e05302/scratchpad/repro.py
```

Usage: `python3 repro.py <lib-test-binary-path> <N>`. It spawns the binary from a Python parent
that is **not a job leader**, which is what holds the `setpgid` window open; without that the
window does not exist and the test cannot fail. It reports per-run `ok` / `FAIL` and a final
`TOTAL <fails>/<N>`.

Resolve the binary path with:

```
rtk proxy cargo test --lib --no-run --message-format=json 2>/dev/null \
  | jq -r 'select(.profile.test == true) | .executable' | tail -1
```

Re-resolve it after every rebuild — the hash in the filename changes.

**Baseline, measured at HEAD before any edit: `TOTAL 1/20`.** Reproduce it once on the untouched
tree to confirm the window is open on this machine, then edit.

**After the fix, require at least 60 contended runs with ZERO occurrences of this test.** Say this
plainly in the write-up: a 1-in-20 base rate means fewer runs cannot distinguish a fix from luck,
and 60 runs is the minimum at which "0 failures" carries information. More is better; report the
actual count rather than the target.

## The control that keeps the post-fix run from being vacuous

The reproducer only proves something if the binary it runs is the REBUILT one. Confirm the
resolved path's mtime is newer than the edit, or rebuild immediately before the run in the same
command. A stale binary produces a clean `TOTAL 0/60` that means nothing.

## Full-suite gates

- **`rtk proxy cargo test --no-fail-fast`, ten times**, capturing each run to a file in the
  scratchpad. Record passed / failed / ignored and the failing test NAMES per run. Each run takes
  ~75 seconds.
- **NEVER plain `cargo test`** — rtk's filtering is not the issue there; fail-fast is, and it
  stops at the lib binary so the later suites never run. That is exactly how the v1.7.0 breakage
  stayed invisible.
- **NEVER pipe rtk output through `grep`** — rtk strips `test result:` lines downstream of
  `proxy`, so a filtered run reports counts that are not real. Capture to a file, then read the
  file.
- The ONLY acceptable recurring failure is
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
  — environmental, local git 2.53.0 against constants re-derived at 2.55.0, expected green on
  CI's runner.
- The BrokenPipe race in `tests/envelope_tracer.rs`'s `run_stub` (~1 in 800 contended runs) is
  REPORTED with its run number if it appears. It is never absorbed and never fixed here.
- **`./scripts/pre-tag-check.sh`**, reported gate by gate. Expected: exit 1, gate 1 SKIPPED (no
  tag argument), gates 2/3/5 PASS, gate 4 failing on exactly one test — the version witness alone.
- **`rtk proxy cargo clippy -- -D warnings`** — clean.
- **`git diff --numstat`** on `deferred-items.md` — deletions column MUST be 0.

</verification>

<success_criteria>

- `TOTAL 0/60` or better from the contended reproducer against a measured HEAD baseline of 1/20,
  with the binary provably rebuilt.
- Ten `rtk proxy cargo test --no-fail-fast` runs with this test green in ALL ten, counts recorded
  per run from unfiltered captures.
- `./scripts/pre-tag-check.sh` reported gate by gate, with gate 4's failure set containing no
  entry for this test.
- `rtk proxy cargo clippy -- -D warnings` clean.
- `git diff` shows the `assert_eq!` and the `.expect` byte-identical; no `#[ignore]`, no
  thread-count flag, no pre-assertion sleep, no `establish_own_group` call in the test region.
- `src/driver/liveness.rs`, `src/driver/mod.rs`, `Cargo.toml`, `.github/` and every non-test line
  of `src/driver/run.rs` untouched.
- The module header records the mechanism, the mover, the evidence, the rates, the exonerations,
  the hypothetical-is-actual correction and both non-goals.
- `deferred-items.md` closes the rows append-only with ZERO deletions, quoting verbatim both
  superseded sentences before correcting them.

</success_criteria>

<output>
Create `.planning/quick/260917-nhc-close-the-last-v1-7-1-publish-gate-flake/260917-nhc-SUMMARY.md` when done.

The summary must record, in the voice the previous two quick tasks used:
- the corrected diagnosis as the headline, including that the previously recorded one was wrong
  and in what way;
- the fail-first baseline and the post-fix contended count, with the run counts stated;
- the ten-run gate table, `pre-tag-check.sh` gate by gate, and clippy;
- the deliberate non-goals, with the `execute_run` ordering decision **flagged for later audit**;
- a "Coordinator inferred decisions (for audit)" section noting that the human was unavailable and
  that this ran unisolated on the primary checkout after the #1941/#48 `worktree.base-check`
  false-negative auto-degrade.
</output>
