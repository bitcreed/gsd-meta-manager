---
phase: quick-260908-uqq
plan: 01
type: execute
wave: 1
depends_on: []
autonomous: true
requirements: [260908-uqq]
files_modified:
  - src/executor/claude.rs
  - src/executor/mod.rs
  - src/error.rs
  - src/ui/screens/driver.rs
  - src/driver/run.rs
  - tests/executor_lifecycle.rs
  - tests/executor_transport.rs
  - tests/fixtures/fake-claude-late-init.sh
  - tests/fixtures/fake-claude-silent.sh

estimate:
  tokens: 95000
  raw_tokens: 95000
  tasks: 2
  confidence: low          # zero calibration samples for this repo; unadjusted projection

must_haves:
  truths:
    - "A run against a CLI that emits `system/init` only after the first stdin user message gets past startup, and its command reaches the child."
    - "On a CLI that announces `system/init` before the grace expires, a capability refusal still writes ZERO bytes to the child's stdin."
    - "On a CLI that announces late, a capability refusal still refuses with `SpawnError::Capability` and still tears the agent's process group down — after the prompt has been written."
    - "Every comment in `src/` and `tests/` that asserted the refusal's cost unconditionally now states the condition under which it holds."
    - "The pre-gate `pgid` publication (CR-01) and the `biased` terminate-arm ordering are unchanged; `tests/driver_kill_startup.rs` stays green."
    - "The run-wide `idle_cap` default is unchanged at 15 minutes."
  artifacts:
    - tests/fixtures/fake-claude-late-init.sh
    - src/executor/claude.rs
    - src/executor/mod.rs
    - tests/executor_lifecycle.rs
    - tests/executor_transport.rs
  key_links:
    - "Coordinator enforcement block → `writer_tx`: the SECOND release path, the one that is not `handle_item`."
    - "`prompt_released` flag threaded into `handle_item`: the prompt is written exactly once whichever arm fires."
    - "`ExecutionOptions::prompt_release_grace` → `Coordinator.prompt_release_grace`: the knob is wired through `start_run`, never defaulted twice."
    - "`SpawnError` breach variants → `driver::run` terminal label: a pre-gate stall is no longer spelled `spawn_failed` (task 2)."
---

<objective>
Break the startup deadlock against CLI 2.1.266 and leave no stale guarantee behind.

**The crux, answered up front: YES — after this change the prompt CAN reach the child
before the gate verdict.** Specifically: it reaches the child before the verdict on any
CLI that does not emit `system/init` within `prompt_release_grace` of spawn. 2.1.266 is
exactly such a CLI — the brief measured `system/init` arriving *because of* the first
stdin user message, with the assistant turn one second behind it. So on 2.1.266 the gate
can only ever judge an init that the prompt itself produced, and by then the turn has
begun.

Therefore the D-06 claim that a capability refusal "costs zero tokens and zero quota"
**loses its monopoly** and every site asserting it unconditionally is corrected in the
same commit (task 1, § Stale-claim inventory).

Purpose: a driver run must get past startup and deliver its command against 2.1.266.
Output: a grace-window prompt release, a 2.1.266-shaped test stand-in, both arms of the
refusal pinned by tests, the corrected claims, and (droppable) a legible failure record.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/260908-uqq-CONTEXT.md
@CLAUDE.md
@src/executor/claude.rs
@src/executor/mod.rs
@src/error.rs
@src/driver/run.rs
@tests/executor_lifecycle.rs
@tests/executor_transport.rs
@tests/fixtures/fake-claude-echo.sh
</context>

## Design decision (read before task 1)

**Shape chosen: a bounded grace, then release. Not "release at spawn, always".**

There is no way to elicit `system/init` from 2.1.266 without starting a turn — the brief
measured init, assistant message and result all arriving within one second of the stdin
write. A probe message would be a turn and would spend quota, so it is not an option. The
prompt must go out before the verdict on such a CLI. The only question is *when*.

Two candidate shapes were weighed:

| Shape | Gate becomes | Cost |
|---|---|---|
| Release at spawn, unconditionally | post-hoc validation everywhere | the pre-prompt refusal disappears from the codebase; the existing zero-bytes test must be deleted or inverted; every CLI that *does* announce eagerly loses a guarantee it could still honour |
| Release after a grace, if init has not arrived | pre-prompt refusal when the CLI announces in time, post-hoc abort otherwise | ~20 extra lines, one extra flag, one extra knob, two pinned arms instead of one |

The grace shape is chosen. Writing bytes into the child's stdin pipe is not itself a
spend — the spend happens when the CLI reads them — so an early release is cheap, and
keeping the eager arm costs almost nothing while preserving a guarantee that is still
*true* on the CLIs where it is true.

**Assumption delta (advisory capability, recorded deliberately): `add-alongside`.**
The pre-prompt refusal is retained as one arm; a post-hoc abort arm is added beside it.
The guarantee lost its monopoly, not its existence. Rationale: an honest conditional beats
both a false unconditional and a deleted capability. This is what forces the doc
corrections below — no comment may keep asserting the unconditional form.

**The gate is not widened and not deleted.** Both arms still refuse, both still stop the
run loop, both still tear the process group down. Only the *ordering relative to the
prompt* differs, and which arm ran is decided by the CLI's own announce timing.

## Stale-claim inventory (the sites task 1 corrects)

Verified present at planning time by direct read. Line numbers are where they were then —
find them by their text, not by line.

| # | File | ~Line | What is stale |
|---|---|---|---|
| 1 | `src/executor/mod.rs` | 17-20 | Module doc point 2: the first user message is withheld until the gate passes, therefore no turn ever begins before validation |
| 2 | `src/executor/mod.rs` | 782-783 | `RunOutcome::CapabilityRefused` doc: "**No turn ever started**" |
| 3 | `src/error.rs` | 28-31 | `SpawnError` enum doc: every variant is reachable before any turn begins |
| 4 | `src/error.rs` | 58-59 | `SpawnError::Capability` doc: "No user message was ever written to stdin" |
| 5 | `src/executor/claude.rs` | 664-666 | The `gate_rx.await` comment: the prompt "has not been released yet" |
| 6 | `src/executor/claude.rs` | 1388-1390 | "A refused run never had its prompt released" |
| 7 | `src/executor/claude.rs` | 1643 | "Only now is the prompt released (D-02, D-06)." |
| 8 | `src/executor/claude.rs` | 2280-2314 | Two in-source assertion messages. `feed()` drives `handle_item` with no grace timer, so both stay TRUE — their messages must say they are pinning the **eager arm** rather than a universal law |
| 9 | `src/ui/screens/driver.rs` | 411 | `TerminalState::CapabilityRefused` doc: "no turn ever started" |
| 10 | `tests/executor_transport.rs` | 524-532, 568-577 | Section header and assertion message for the zero-bytes test: scope both to the eager arm |
| 11 | `tests/fixtures/fake-claude-silent.sh` | header | "The prompt is never released — the gate never opens" — false once the grace fires |

**No negative grep gates this inventory, and that is deliberate.** An honest correction may
legitimately quote the claim it is correcting, so a negative grep on the old wording would
forbid the very sentence that makes the correction legible. The gate is a positive grep for
the new conditional wording plus — the way this repo actually proves things — two tests, one
per arm.

## Not in scope

- **The stderr / `events_rx` backpressure hazard is NOT fixed here.** Pre-gate nothing
  drains `events_rx`, so `read_stderr`'s `tx.send().await` can park, and buffered stderr is
  dropped when `events_rx` is dropped on the error path. Fixing it needs a sink that
  survives (the journal, in `run.rs`), which is not cheap. **The grace shrinks the exposure
  window from "until the idle cap" to "until the grace expires plus init latency", and that
  is the only relief this change gives it.** Report it in the SUMMARY, unfixed.
- **`idle_cap` is not shrunk and no second startup bound is added.** After the fix, a
  genuinely mute child still takes the full 15 minutes to trip. That residual is disclosed
  in the SUMMARY, not papered over.
- No new dependencies. No `COVERAGE.md`: this integrates no new external API surface, it
  fixes the stdin/stdout handshake with an already-integrated CLI subprocess.

<tasks>

<task type="tracer">
  <name>Task 1: Release the prompt on a grace, prove both arms, and correct every claim the change falsifies</name>
  <files>
tests/fixtures/fake-claude-late-init.sh (new),
tests/fixtures/fake-claude-silent.sh,
src/executor/mod.rs,
src/executor/claude.rs,
src/error.rs,
src/ui/screens/driver.rs,
tests/executor_lifecycle.rs,
tests/executor_transport.rs
  </files>
  <behavior>
    - A stand-in that stays silent until its first stdin line, then announces and answers,
      is driven to completion: `start()` returns `Ok`, and the command text is in the
      stand-in's stdin log.
    - The same stand-in, one capability short: `start()` returns
      `Err(SpawnError::Capability(CapabilityError::MissingCapabilities { .. }))`, AND its
      stdin log is non-empty — the refusal happened after the prompt was written.
    - A stand-in that announces immediately, one capability short: `start()` refuses and
      the stdin log is still zero bytes.
    - The prompt is written exactly once, whichever arm released it.
    - The default grace is greater than zero and strictly less than the default idle cap.
  </behavior>
  <action>
This is one commit. The behaviour change and the comment corrections land together —
splitting them leaves a window in which the tree ships a guarantee it no longer honours.

**Order the work: fixture first, then the knob, then the release, then the tests, then the
corrections.**

1. **`tests/fixtures/fake-claude-late-init.sh`** (new, `chmod +x`, `#!/bin/sh`, `set -u`).
   Model it on `tests/fixtures/fake-claude-echo.sh` — same leading-argument contract
   (`<capabilities-csv> <version> <api-key-source> <stdin-log> [ignored...]`), same
   tolerance of the appended real claude argv, same truncate-the-log-at-startup discipline.
   The ONE difference is the entire fixture and its header comment must say so: it prints
   NOTHING at startup. It emits its `system/init` envelope only after reading its first
   stdin line, then behaves exactly as the echo stand-in does (replay echo with
   `"isReplay":true`, a terminal `result` at stdin EOF, exit 0). Cite the brief's measured
   2.1.266 behaviour in the header as what the fixture stands in for.

2. **`src/executor/mod.rs` — the knob.** Add `pub prompt_release_grace: Duration` to
   `ExecutionOptions`, beside `idle_cap`. Default it to `Duration::from_secs(5)` in
   `impl Default`. Doc it in the register the neighbouring caps use: it is how long the
   supervisor waits for the child to announce itself before writing the prompt anyway; it
   is a frank number with no tuning data behind it, bounded above by the latency every
   iteration pays on a CLI that announces late and below by the time a healthy eager CLI
   needs to print its init; and it is a deliberate startup-specific bound added rather than
   a shrunk `idle_cap`, because those two answer different questions.

3. **`src/executor/claude.rs` — the release.**
   - Thread `prompt_release_grace: options.prompt_release_grace` from `start_run` into the
     `Coordinator` struct literal and add the matching field + doc.
   - In `Coordinator::run`, destructure it, and add two locals beside `gated`:
     `prompt_released = false` and `prompt_deadline = Instant::now() + prompt_release_grace`
     (computed once, before the loop, alongside `wall_deadline`).
   - In the UNCONDITIONAL enforcement block at the top of each pass — the block whose whole
     doc explains that every bound this supervisor owns is evaluated there — add: when
     `!prompt_released && now >= prompt_deadline`, set `prompt_released = true` and
     `try_send` the first message on `writer_tx` as a `WriterCommand::Line`.
     **Use `try_send`, never an awaited send, and set the flag before inspecting the
     result.** An awaited send here would park the enforcement block and disable every cap
     and the cancel — precisely the CR-01 class of defect the block's own doc warns about —
     and a flag set only on success turns a `Full` channel into a busy spin against a
     deadline already in the past. `Full` is unreachable in practice because nothing writes
     to stdin before the gate; log a `tracing::warn!` on either error and let the run fail
     on its own terms. Say all of that in the comment.
   - Add a `select!` arm `_ = tokio::time::sleep_until(prompt_deadline), if !prompt_released`
     with an empty body: it exists to unpark the loop at the grace so the enforcement block
     above can act, exactly as the cap arms do. Add `prompt_deadline` to the
     `forward_deadline` minimum while `!prompt_released`, for the same reason the caps are
     in it.
   - Pass `&mut prompt_released` into `handle_item` and use it there: the successful-gate
     branch releases the prompt only when it has not already gone, and sets the flag when it
     does. The existing "written exactly once" assertion is what this protects.
   - Leave `SpawnError::InitNeverObserved`, the pre-gate `pgid` publication, the `biased`
     ordering, the refusal branch and the teardown untouched.

4. **Tests.**
   - `tests/executor_lifecycle.rs`: add a `FAKE_LATE_INIT` const beside the others, and a
     test that drives the new stand-in with `prompt_release_grace` set explicitly short
     (~200ms) and the full required capability set, asserting `start()` returns `Ok` and the
     stdin log contains the command text. Name it so a reader sees the bug it pins — the
     handshake against a CLI that announces only after the first user message. This is the
     acceptance test; it must fail against the current tree.
   - `tests/executor_lifecycle.rs`: add a cheap assertion that the DEFAULT grace is
     `> Duration::ZERO` and `< ExecutionOptions::default().idle_cap`, in the same spirit as
     the existing default-ordering assertions in `src/executor/claude.rs`.
   - `tests/executor_transport.rs`: add the post-release refusal arm as a sibling of
     `a_refused_run_writes_zero_bytes_to_the_child_stdin`, using the late-init stand-in one
     capability short. Assert the typed refusal AND that the stdin log is non-empty. Its
     assertion message is where the honest weaker claim is written down: on a CLI that
     announces only after reading a user message, the init the gate judges is the one the
     prompt produced, so the refusal aborts a turn that has already begun.
   - Do NOT weaken the existing zero-bytes test. It drives the eager stand-in and stays
     green; only its wording changes, per item 10 of the inventory.

5. **Corrections.** Work the § Stale-claim inventory table above, every row. Each corrected
   site must name the condition rather than the outcome: which arm holds the strong property
   and which one does not, and why the CLI's announce timing is what decides. Where a site is
   an assertion message pinning the eager arm, say that it pins the eager arm. Leave nothing
   asserting the unconditional form anywhere in `src/` or `tests/`.
  </action>
  <verify>
    <automated>BASE=$(git log --format=%H -n 1 -- .planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/260908-uqq-PLAN.md); test -n "$BASE" || { echo "FAIL: cannot resolve this plan's own commit, so the diff range this gate compares over is unknown"; exit 1; }; echo "base=$BASE"; rtk proxy cargo build &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: build"; exit 1; }; test -x tests/fixtures/fake-claude-late-init.sh || { echo "FAIL: the 2.1.266-shaped stand-in is missing or not executable"; exit 1; }; rtk proxy grep -qF 'idle_cap: Duration::from_secs(15 * 60)' src/executor/mod.rs || { echo "FAIL: the run-wide idle_cap default moved — it was required to stay at fifteen minutes and the new bound to be its own knob"; exit 1; }; for f in src/executor/mod.rs src/error.rs src/executor/claude.rs src/ui/screens/driver.rs tests/executor_transport.rs tests/fixtures/fake-claude-silent.sh; do test "$(rtk proxy grep -cF 'prompt_release_grace' $f || true)" -ge 1 || { echo "FAIL: $f does not name the knob its corrected claim is now conditional on — a claim site that cites no condition has not been corrected"; exit 1; }; done; rtk proxy cargo test --test executor_lifecycle --no-fail-fast &gt;/tmp/uqq-t1-life.log 2&gt;&amp;1 || { echo "FAIL: executor_lifecycle is RED — the late-init delivery arm or a pre-existing lifecycle bound"; rtk proxy grep -A4 'FAILED|panicked' /tmp/uqq-t1-life.log | head -40; exit 1; }; rtk proxy cargo test --test executor_transport --no-fail-fast &gt;/tmp/uqq-t1-tx.log 2&gt;&amp;1 || { echo "FAIL: executor_transport is RED — one of the two refusal arms"; rtk proxy grep -A4 'FAILED|panicked' /tmp/uqq-t1-tx.log | head -40; exit 1; }; rtk proxy cargo test --test driver_kill_startup &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: CR-01's three-process startup-teardown proof is RED — the pgid publication or the biased terminate arm moved"; exit 1; }; rtk proxy cargo test --test spawn_seam_guard &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: the spawn seam guard is RED"; exit 1; }; rtk proxy cargo test --lib executor:: --no-fail-fast &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: the in-source executor tests are RED — the written-exactly-once pin most likely"; exit 1; }; rtk proxy cargo clippy -- -D warnings &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: clippy"; exit 1; }; test -z "$(git diff --name-only "$BASE"..HEAD -- Cargo.toml Cargo.lock)" || { echo "FAIL: a manifest moved — this change adds no dependency"; exit 1; }; rtk proxy grep 'test result:' /tmp/uqq-t1-life.log /tmp/uqq-t1-tx.log; echo OK</automated>
  </verify>
  <done>
- `rtk proxy cargo test --test executor_lifecycle` passes, including the new late-init
  delivery test, and that test is proved fail-first (run it once against the pre-change
  executor, or `git stash` the `src/` half, and record the RED observation in the SUMMARY).
- `rtk proxy cargo test --test executor_transport` passes with BOTH refusal arms: zero bytes
  on the eager stand-in, non-empty stdin log plus a typed `SpawnError::Capability` on the
  late-init stand-in.
- `tests/driver_kill_startup.rs` and `tests/spawn_seam_guard.rs` are green — CR-01's pgid
  publication and the `biased` terminate arm are intact.
- `cargo clippy -- -D warnings` exits 0.
- Every row of § Stale-claim inventory is edited; no site in `src/` or `tests/` still states
  the refusal's cost without its condition.
- `ExecutionOptions::default().idle_cap` is still `Duration::from_secs(15 * 60)`.
  </done>
  <reversibility rating="costly">Changes the ordering guarantee the capability gate is documented on; reverting restores the deadlock against 2.1.266.</reversibility>
</task>

<task type="auto" tdd="true">
  <name>Task 2 (SECONDARY — DROPPABLE): stop calling a pre-gate stall `spawn_failed`, and put the reason on disk</name>
  <files>src/error.rs, src/executor/claude.rs, src/driver/run.rs</files>
  <behavior>
    - A run whose gate is answered by an idle breach yields a terminal label of `stalled`,
      not `spawn_failed`; a wall-clock breach yields `timed_out`; a capability refusal
      yields `capability_refused`. Every other spawn failure stays `spawn_failed`.
    - `journal.jsonl` carries a `Diagnostic` record naming the reason BEFORE the terminal
      `run_ended` record, on every spawn-failure path.
    - The diagnostic's `code` and `detail` come from a closed, driver-authored vocabulary.
  </behavior>
  <action>
**DROP THIS TASK AND REPORT IT if task 1 ballooned.** The brief marks it explicitly
droppable. If dropped, say so in the SUMMARY with what remains open.

1. **`src/error.rs`.** Add two `SpawnError` variants beside `InitNeverObserved`, named
   exactly `StalledBeforeInit { idle_for: Duration }` and
   `TimedOutBeforeInit { after: Duration }`. Two rather than one, on the same grounds the
   supervisor already separates its two breaches: "went silent" and "took too long" have
   different remedies. Add their `Display` arms; `source()` returns `None` for both. Doc
   each as what it actually is — a supervisor bound that expired while `start` was still
   parked, which is a *stall*, not a launch failure. (No external site matches `SpawnError`
   exhaustively — only `Display` and `source` in this file — so adding variants is cheap.)
2. **`src/executor/claude.rs`.** At the post-loop site that answers an unanswered gate,
   choose the variant from the `breach` local already in scope: idle → the stall variant,
   wall-clock → the timeout variant, neither → `InitNeverObserved` unchanged. Comment why:
   the supervisor computed the right verdict and sent it on `outcome_tx`, whose receiver
   lives in an `ExecutionHandle` this path never returns — so the gate channel is the only
   surface the caller has, and it must carry the same fact.
3. **`src/driver/run.rs`.** Add a private helper mapping `&SpawnError` onto a terminal
   label, and use it in place of the hard-coded `"spawn_failed"` at the spawn-failure arm.
   **Reuse `outcome_label`'s existing words — `capability_refused`, `stalled`, `timed_out`,
   `spawn_failed` — and invent none.** The TUI's label reader already recognises all four,
   so no render change is needed; a new word would read as `Unrecorded`. Note that in the
   helper's doc.
4. **`src/driver/run.rs` — the record.** In the same arm, before `finish_run`, write a
   `JournalEvent::Diagnostic` whose `code` is a closed-vocabulary identifier derived from
   the variant and whose `detail` is a driver-authored sentence — for the never-observed
   case, the exact sentence the `Display` impl already carries about the stream ending
   before any init was seen. Follow the established pattern at the terminate-shutdown site:
   log the error KIND on failure and still attempt the terminal record, which is the more
   important of the two. **Never journal the rendered error — do not reach for the
   `SpawnError`'s `Display`.** `CapabilityError` interpolates strings the CLI supplied (its
   reported version, its advertised capability names) and this journal is read back and
   painted in the TUI; a closed vocabulary keeps that surface shut. Say so in the comment so
   a later reader does not "simplify" it — and phrase that comment without naming the
   rendering call, because a mechanical gate on this task counts non-comment occurrences of
   it.
5. Add a test in `src/driver/run.rs`'s in-source test module asserting the label mapping for
   each variant — including that the fallback is still `spawn_failed`.
  </action>
  <verify>
    <automated>if ! rtk proxy grep -qF 'StalledBeforeInit' src/error.rs; then echo "DROPPED: task 2 was not attempted — the SUMMARY must say so and state that a pre-gate stall is still reported as spawn_failed"; exit 0; fi; BASE=$(git log --format=%H -n 1 -- .planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/260908-uqq-PLAN.md); test -n "$BASE" || { echo "FAIL: cannot resolve this plan's own commit, so the diff range this gate compares over is unknown"; exit 1; }; rtk proxy cargo build &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: build"; exit 1; }; rtk proxy cargo test --lib driver::run --no-fail-fast &gt;/tmp/uqq-t2-run.log 2&gt;&amp;1 || { echo "FAIL: the driver::run unit tests are RED — the label mapping most likely"; rtk proxy grep -A4 'FAILED|panicked' /tmp/uqq-t2-run.log | head -40; exit 1; }; rtk proxy cargo test --lib executor:: --no-fail-fast &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: the in-source executor tests are RED"; exit 1; }; for t in executor_lifecycle executor_transport driver_kill_startup driver_dry_run journal_run_paths; do rtk proxy cargo test --test $t &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: UNEXPECTED RED in $t"; exit 1; }; done; test "$(rtk proxy grep -vE '^[[:space:]]*(//|///)' src/driver/run.rs | rtk proxy grep -cF 'err.to_string()' || true)" -eq 0 || { echo "FAIL: run.rs journals a rendered SpawnError on a NON-comment line — CapabilityError interpolates CLI-supplied strings into a surface that is read back and painted; code and detail must come from the closed driver-authored vocabulary"; exit 1; }; rtk proxy cargo clippy -- -D warnings &gt;/dev/null 2&gt;&amp;1 || { echo "FAIL: clippy"; exit 1; }; test -z "$(git diff --name-only "$BASE"..HEAD -- Cargo.toml Cargo.lock)" || { echo "FAIL: a manifest moved"; exit 1; }; rtk proxy grep 'test result:' /tmp/uqq-t2-run.log; echo OK</automated>
  </verify>
  <done>
- The label helper is tested for all four outcomes and the mapping test fails if any variant
  falls back to `spawn_failed` incorrectly.
- A spawn failure writes a `Diagnostic` before `run_ended`; the sentence about the stream
  ending before any `system/init` was observed now exists on disk.
- `cargo clippy -- -D warnings` exits 0.
- OR: the task is dropped, and the SUMMARY says so, names what stays open, and states that
  a pre-gate stall is still reported as `spawn_failed`.
  </done>
  <reversibility rating="reversible">Additive variants and a label mapping; revertible without touching the handshake.</reversibility>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| driver → `claude` child stdin | the operator's command text crosses into a subprocess whose identity/auth the gate has not yet judged on the late-announce arm |
| `claude` child stdout/stderr → driver | untrusted CLI-authored text crosses in; already framed and bounded by the existing readers |
| driver → agent process group | teardown signals cross out during the widened pre-gate window |

## STRIDE Threat Register (ASVS L1; block on high)

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-UQQ-01 | Information disclosure | grace release in `Coordinator::run` | medium | mitigate | The command reaches a child whose `apiKeySource`/version the D-07/D-08 guards have not yet judged. Bounded three ways: the grace preserves the pre-prompt refusal wherever the CLI announces in time; the refusal still fires and still tears the group down on the late arm; and the residual is written into the corrected doc comments rather than left implicit. Pinned by the task-1 post-release refusal test. |
| T-UQQ-02 | Denial of service | pre-gate teardown path | high | mitigate | The pre-gate `pgid` publication (CR-01) and the `biased` terminate-arm ordering are explicitly unchanged, and `tests/driver_kill_startup.rs` is a required-green gate on task 1. A stop landing during the now-longer-lived startup window must still reach the agent's group. |
| T-UQQ-03 | Denial of service | `read_stderr` → undrained `events_rx` | low | accept | Pre-existing, out of scope, disclosed in the SUMMARY. The grace shrinks the exposure window rather than closing it; no new exposure is added by this change. |
| T-UQQ-04 | Tampering | journal `Diagnostic` detail (task 2) | medium | mitigate | Closed driver-authored vocabulary for `code` and `detail`; `err.to_string()` is explicitly forbidden because `CapabilityError` interpolates CLI-supplied strings into a surface that is read back and rendered. |
| T-UQQ-SC | Tampering | npm/pip/cargo installs | n/a | accept | No package installs and no new dependencies in this change; the package-legitimacy gate has nothing to audit. |
</threat_model>

<verification>
Final gate, run from the repo root:

```
rtk proxy cargo build
rtk proxy cargo test --no-fail-fast
rtk proxy cargo clippy -- -D warnings
```

**`--no-fail-fast` is mandatory** (STATE.md, 19-12): a plain `cargo test` stops at the
failing `driver_reattach` binary and never reaches the later test binaries, reporting an
unchanged count no matter what was added. The two `driver_reattach` failures are the
documented pre-existing pair; the total passed count must be strictly greater than the
pre-change total by the number of tests added, and the failure count must still be 2.

`rtk proxy` rather than bare `cargo` throughout, per CLAUDE.md: rtk strips `warning:` and
`test result:` lines, so any check that reads raw output is vacuous without it.
</verification>

<success_criteria>
- A run against a `system/init`-after-first-stdin-message CLI gets past startup and its
  command reaches the child, proved by a test using the checked-in stand-in — never by
  spending real quota.
- Both refusal arms are pinned by tests: zero bytes on the eager arm, prompt-already-written
  on the late arm, and a typed `SpawnError::Capability` on both.
- No comment in `src/` or `tests/` asserts the refusal's cost without stating the condition.
- CR-01's pgid publication and the `biased` terminate-arm ordering are untouched.
- `ExecutionOptions::default().idle_cap` is unchanged; the new bound is its own named knob.
- Full gate clean apart from the two pre-existing `driver_reattach` failures.
</success_criteria>

<output>
Create `.planning/quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/260908-uqq-SUMMARY.md` when done.

It must record, explicitly:
1. The measured RED-then-GREEN observation for the acceptance test.
2. The assumption delta (`add-alongside`) and the list of corrected claim sites.
3. The unfixed stderr/`events_rx` backpressure hazard, and that the grace only shrinks its
   window.
4. The unfixed 15-minute residual for a genuinely mute child, and that `idle_cap` was
   deliberately not shrunk.
5. Whether task 2 shipped or was dropped, and what stays open either way.
</output>
