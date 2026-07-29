---
phase: 15-transport-foundation
reviewed: 2026-07-29T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - src/app.rs
  - src/error.rs
  - src/executor/claude.rs
  - src/executor/gate.rs
  - src/executor/mod.rs
  - src/executor/outcome.rs
  - src/executor/stream_json.rs
  - src/lib.rs
  - src/main_loop.rs
  - src/main.rs
  - src/state_reader/git_ops.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - tests/executor_lifecycle.rs
  - tests/executor_transport.rs
  - tests/fixtures/fake-claude-deaf.sh
  - tests/fixtures/fake-claude-echo.sh
  - tests/fixtures/fake-claude.sh
  - tests/fixtures/fake-claude-slow.sh
  - tests/fixtures/fake-claude-spawner.sh
  - tests/fixtures/transcripts/README.md
  - docs/DEVELOPMENT.md
  - docs/GETTING-STARTED.md
  - docs/TESTING.md
findings:
  critical: 4
  warning: 17
  info: 0
  total: 21
status: issues_found
---

# Phase 15: Code Review Report

**Reviewed:** 2026-07-29
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

The wire model (`stream_json.rs`), the capability gate (`gate.rs`) and the outcome
matrix (`outcome.rs`) are in good shape: parsing is genuinely tolerant (no
`deny_unknown_fields`, unit `#[serde(other)]` catch-alls with the raw line preserved
by `Envelope`), the gate fails closed on every absence, `result` is correctly treated
as a turn boundary with no early `break`, `--bare`/`bypassPermissions`/
`--dangerously-skip-permissions` never reach the argv, teardown really does start
with `signal(15)` and reserves `start_kill()` for the post-grace escalation, and no
raw stream body is written to the log file (only byte counts and error values).
`cargo clippy --lib --bins -- -D warnings` is clean and both integration binaries pass.

The defects are concentrated in the **supervisor loop** (`Coordinator::run`) and in
the **production wiring** of otherwise-correct logic:

* Both deadline caps and the cancel signal are only evaluated *between* reader items,
  and they are disabled outright once the child is observed exited. A stream that
  stays hot, a consumer that stops draining, or a descendant that holds the stdout
  pipe open each defeat D-13's "never unbounded" promise — one of them permanently.
* The `permission_denials[]` source is implemented, tested, and **unreachable from
  production**, so a run that `dontAsk` blocked is reported as a *success* variant.
* `interrupt()` awaits a oneshot that nothing ever drops, so an unanswered interrupt
  hangs the caller for the process lifetime.

Findings below are classified **BLOCKER** (must fix before this ships) and
**WARNING** (should fix).

No `<structural_findings>` block was supplied with this review, so the findings
below are entirely narrative.

## Narrative Findings (AI reviewer)

### BLOCKER

#### CR-01: Both deadline caps and cancel are unenforceable while the stream is hot or the consumer is blocked

**File:** `src/executor/claude.rs:829-912` (loop head, `biased;` at 836), `src/executor/claude.rs:838-857` (reader arm calling `handle_item(...).await`), `src/executor/claude.rs:976-1092` (`handle_item` awaits `events_tx.send`)

**Issue:** The supervisor's `select!` is `biased;` with `reader_rx.recv()` first, and
the winning arm's body then `.await`s inside `handle_item` on a *bounded* channel
(`EVENT_CHANNEL_CAPACITY = 8192`). Two consequences, both defeating D-13:

1. **Starvation.** With `biased;`, tokio polls arm 1 first and never polls the later
   arms when it is ready. A child that emits lines faster than the loop retires them
   keeps `reader_rx` permanently non-empty, so `wall_deadline`, `idle_deadline`, the
   grace arm **and `cancel_rx`** are never polled. The 4-hour wall-clock cap — the
   only bound on a runaway run's quota spend — never fires, and `Executor::cancel`
   never takes effect.
2. **Suspension.** Once `select!` has picked the reader arm, the loop is *inside*
   `handle_item`, awaiting `events_tx.send(...)`. Every timer future has already been
   dropped. When the TUI stops draining — precisely the state `main.rs:88-105` and
   `main_loop.rs:57-64` document as expected during the blocking `$EDITOR` shell-out —
   the channel fills and the coordinator parks there with **no cap armed and no cancel
   arm**. In steady state the child blocks on its own pipe writes and the run simply
   never ends.

`tests/executor_lifecycle.rs:311-350` (`a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap`)
paces its heartbeats at 50 ms with an immediately-drained channel, so it never
exercises either condition.

**Fix:** Do not let the stream arm monopolise the loop or hold the loop across an
unbounded send. Two changes:

```rust
// 1. Evaluate the deadlines unconditionally, not just when select! reaches them.
loop {
    if !exited && Instant::now() >= wall_deadline { breach = Some(Breach::WallClock); break; }
    if !exited && Instant::now() >= *last_line_rx.borrow() + idle_cap {
        breach = Some(Breach::Idle); break;
    }
    if !cancelled && cancel_rx.try_recv().is_ok() { cancelled = true; terminate = true; }
    // ... select! as before ...
}

// 2. Bound the forward, so a stalled consumer can never park the supervisor.
//    (Drop-oldest or a deadline; a dropped diagnostic beats an unbounded park.)
match tokio::time::timeout(FORWARD_CAP, events_tx.send(event)).await { ... }
```

Alternatively drop `biased;` for a round-robin `select!` and move the send behind a
`try_send`-with-overflow-counter, which also removes the need for the 8192 buffer.

---

#### CR-02: Once the child is observed exited the loop has no bound and no escape — a descendant holding stdout hangs the run forever

**File:** `src/executor/claude.rs:864-892` (every deadline arm is guarded `if !exited`), `src/executor/claude.rs:898-907` (`terminate_group` guarded `if !exited`), `src/executor/claude.rs:927-929` (`if exited { exit_status }`)

**Issue:** After the exit arm fires, `exited == true` disables the wall-clock arm, the
idle arm and the grace arm. The only remaining arms are `reader_rx.recv()` and
`cancel_rx`. `reader_rx` yields `None` only when `read_stdout` returns, i.e. only on
**stdout EOF** — which requires every process holding the write end of that pipe to
be gone. `claude` routinely backgrounds Bash grandchildren that inherit stdout; one
that outlives the leader (or that escaped the group via `setsid`, in which case
`waitpid(-pgid)` returns `ECHILD` and `wait()` resolves immediately) keeps the pipe
open forever. The coordinator then loops forever: `outcome_tx` is never sent, so
`ExecutionHandle::wait_outcome()` and `Executor::cancel()` hang for the process
lifetime, and the task, the child handle and the pipes leak.

Cancel cannot rescue it: the cancel arm sets `terminate`, but `terminate_group` is
guarded by `if !exited` so no signal goes out, and the grace arm is guarded by
`!exited` so `stop` is never set.

This is reachable even when the group *is* intact, because `ProcessGroupChild::wait`
caches the leader's status (`process-wrap-9.1.0/src/tokio/process_group.rs:181-183`):
once a partially-polled `wait()` future has cached it and been dropped by another
`select!` arm winning — which the tail of lines after the leader exits makes likely —
the next `child.wait()` returns `Ready` immediately **without reaping the group**, so
`exited` flips true while group members are still alive.

Related: line 927's `if exited { exit_status }` skips the teardown entirely, so those
surviving members are never signalled at all — the exact "background agent holding
files, ports and quota the user cannot see" outcome `tear_down_group`'s doc comment
says T-15-30 exists to prevent.

**Fix:** Keep an absolute bound on the post-exit drain and always tear the group down:

```rust
// When the exit arm fires:
exited = true;
exit_status = status.ok();
drain_deadline = Some(Instant::now() + EXIT_DRAIN_CAP);

// New arm, not guarded by !exited:
_ = tokio::time::sleep_until(drain_deadline.unwrap_or(wall_deadline)),
    if drain_deadline.is_some() => { stop = true; }
```

and after the loop, replace `if exited { exit_status }` with a path that still calls
`tear_down_group` (or at minimum `terminate_group` + a bounded `wait`) whenever the
group has not been proven reaped. A regression test needs a stand-in that backgrounds
a process inheriting stdout and then exits the leader.

---

#### CR-03: `interrupt()` awaits a oneshot nothing ever drops — an unanswered interrupt hangs the caller forever

**File:** `src/executor/claude.rs:462-489`, `src/executor/mod.rs:330` (`pub pending_control`), `src/executor/claude.rs:1078-1089` (only removal site)

**Issue:** `interrupt()` inserts a `oneshot::Sender` into the `pending_control` map
and then `rx.await`s it with **no timeout**. That await resolves only if the sender is
sent on or dropped. The sender lives inside the map, and the map is an `Arc` held by
the `ExecutionHandle` itself, so it is never dropped while the caller holds the
handle — and the coordinator never clears the map on any teardown path (`grep
pending_control` shows insert at 473, remove at 484 and 1080, and no drain at run
end). If the child dies, is killed, or simply never answers the `control_request`
(the 2.1.220 bare form was empirically observed to produce no response at all), the
caller hangs for the process lifetime.

Consequently `SendError::ControlResponseLost` (`src/error.rs:114-119`) is dead code:
the `Err(_)` branch at line 483 cannot be taken.

**Fix:** Clear the map when the run ends *and* bound the wait:

```rust
// end of Coordinator::run, before `let _ = outcome_tx.send(outcome);`
pending_control.lock().await.clear(); // drops every waiter → callers get Err

// in interrupt()
match tokio::time::timeout(CONTROL_RESPONSE_CAP, rx).await {
    Ok(Ok(response)) => Ok(InterruptAck::from_response(&response)),
    _ => {
        handle.pending_control.lock().await.remove(&request_id);
        Err(SendError::ControlResponseLost { request_id })
    }
}
```

---

#### CR-04: `RunOutcome::PermissionDenied` is unreachable in production — a permission-blocked run is reported as a success

**File:** `src/executor/claude.rs:962` (`derive_run_outcome(&turns, ...)`), `src/executor/claude.rs:1057` (`turns.push(TurnOutcome::from_result(&result))`), `src/executor/outcome.rs:165-196`

**Issue:** The coordinator projects each `result` envelope onto `TurnOutcome`
immediately and discards the envelope, then calls `derive_run_outcome`, which hard-codes
`Vec::new()` for denials (`outcome.rs:171`). `permission_denials[]` lives only on
`ResultMessage`, so the denials arm at `outcome.rs:240-242` can never fire in
production. `derive_run_outcome_from_envelopes` — the entry point the module doc calls
"the complete four-source derivation" — is referenced **only** by its own unit tests
(verified by grep across `src/` and `tests/`).

The failure is not a missing feature, it is a wrong answer: a run that `--permission-mode
dontAsk` blocked emits a `success` envelope with a populated `permission_denials[]` and
no disk delta, which falls through the first match arm to
`RunOutcome::SucceededNoChanges`. The driver reports **success** for the untrusted-workspace
case that D-10/P2 identify as the whole reason the denials source exists.

**Fix:** Keep the envelopes and call the denials-aware entry point:

```rust
// Coordinator
let mut envelopes: Vec<ResultMessage> = Vec::new();
// in handle_item's Result arm
envelopes.push((*result).clone());
// at the end
None => derive_run_outcome_from_envelopes(&envelopes, status, &before, &after),
```

(`turns` can then be derived from `envelopes` rather than tracked separately.) Add an
end-to-end test using a stand-in whose terminal envelope carries a non-empty
`permission_denials[]`.

---

### WARNING

#### WR-01: `finish_teardown` races the group `wait()` it documents as never raced, and the retry cannot reap

**File:** `src/executor/claude.rs:1144-1154`, doc contract at `src/executor/claude.rs:1120-1127`

**Issue:** Step 4's doc says the reap "is never raced against anything here", but step 2
is implemented as `tokio::time::timeout(grace, child.wait())`. When the grace expires,
that future is dropped — possibly after `ProcessGroupChild::wait` has already cached the
leader status and while the blocking group reap is pending. The subsequent `child.wait()`
at line 1153 then returns the cached status **immediately** and performs no reap, so the
"unconditional wait so no zombie survives" property does not hold on the escalation path.
(In practice the orphaned `spawn_blocking` reaper usually still runs, but the coordinator
neither knows nor waits for it.)

**Fix:** Use `try_wait()` polling for the grace instead of a cancelled `wait()`, e.g.
poll `child.try_wait()` every 100 ms until the grace elapses, then `start_kill()` and a
single uncancelled `child.wait().await`.

#### WR-02: A breached cap outranks an explicit cancel, misreporting the outcome and charging a second grace period

**File:** `src/executor/claude.rs:916-963`

**Issue:** After a cancel, `breach` arms stay enabled (`if !exited`). A cancelled child
that goes quiet while dying can trip the idle cap; then `tear_down = breach.is_some()`
is true, so the code takes the `tear_down_group` branch (line 930) — a **second**
SIGTERM plus a **fresh** full `TEARDOWN_GRACE` — even though step 1 already ran and its
grace has been elapsing since line 906. The outcome match (line 956) also puts `breach`
before `cancelled`, so a user-cancelled run is reported as `Stalled`/`TimedOut` rather
than `Killed`.

**Fix:** Suppress the breach arms once `cancelled` is set (`if !exited && !cancelled`),
and order the outcome match `cancelled` before `breach`.

#### WR-03: `TimedOut`/`Stalled` discard every turn observed, losing the run's cost and turn accounting

**File:** `src/executor/mod.rs:536-546`, `src/executor/claude.rs:956-960`

**Issue:** `RunOutcome::TimedOut` and `RunOutcome::Stalled` carry only a `Duration`. A
run that completed three turns and then breached a cap reports nothing about those turns —
no `total_cost_usd`, no `session_id`, no `terminal_reason` — while `Killed { turns }`
retains them. The collected `turns` vector is simply dropped at line 957/960.

**Fix:** Add `turns: Vec<TurnOutcome>` to both variants (or a shared
`observed: Vec<TurnOutcome>` field), and populate them from the coordinator.

#### WR-04: `TimedOut { after: Duration::ZERO }` is a sentinel with the same type as a real cap

**File:** `src/executor/outcome.rs:273-279`

**Issue:** Two producers now build `TimedOut` with incompatible semantics: the supervisor
passes the real breached cap, the `aborted_tools` arm passes `Duration::ZERO` meaning
"externally bounded, duration unknown". Any renderer formatting `after` will print
"timed out after 0s", which reads as "instantly" — a claim the derivation explicitly does
not make. Nothing renders it today, which is exactly why it will be wrong when Phase 18
does.

**Fix:** Make the field `after: Option<Duration>` (or add a distinct
`ExternallyBounded` variant) so "unknown" is representable rather than encoded as zero.

#### WR-05: `start()` has no start-up deadline of its own

**File:** `src/executor/claude.rs:374-378`

**Issue:** `gate_rx.await` is unbounded. A program that spawns, keeps stdout open and
never emits `system/init` (a wrong binary on `PATH`, a CLI awaiting a TTY prompt) is
caught only when the *idle cap* fires — 15 minutes by default — and `start()` blocks for
that whole period before returning `InitNeverObserved`. The caller is a TUI action.

**Fix:** Wrap the gate wait in its own short bound:

```rust
let facts = match tokio::time::timeout(INIT_CAP, gate_rx).await {
    Ok(Ok(Ok(facts))) => facts,
    Ok(Ok(Err(err))) => return Err(err),
    _ => return Err(SpawnError::InitNeverObserved),
};
```
(and signal the group before returning, so the timed-out child is not leaked — see WR-06).

#### WR-06: An abandoned `start()` leaves an unreachable, unkillable agent process running

**File:** `src/executor/claude.rs:349-378`

**Issue:** The `Coordinator` task is spawned before the gate is awaited. If the caller
drops the `start()` future (a cancelled TUI action, a `select!` losing arm), the
coordinator keeps running with the child; nothing holds `cancel_tx`, so the run cannot be
stopped and continues up to the 4-hour wall-clock cap. `KillOnDrop` does not help — the
coordinator, not the dropped future, owns the child.

**Fix:** Give the coordinator an abort/`Drop` guard held by the `start()` future, or
register the run in a process-wide registry keyed by `pgid` before spawning the task.

#### WR-07: The stdin writer task and the child's stdin pipe leak on every clean run

**File:** `src/executor/claude.rs:688-708`, `src/executor/claude.rs:917-920`, `src/executor/mod.rs:332` (`stdin_tx` on the handle)

**Issue:** `WriterCommand::Close` is sent only on the refused/breached/cancelled paths.
On a clean run the coordinator does `drop(writer_tx)` (line 920), but the
`ExecutionHandle` still holds a `stdin_tx` clone, so `rx.recv()` never returns `None`,
`write_stdin` never returns, and the task plus the `ChildStdin` fd stay alive until the
handle is dropped. Separately, `let _ = writer_tx.send(WriterCommand::Close).await` at
line 918 is itself an unbounded await: if the writer task is parked in `write_all` on a
child that stopped reading, and the 64-slot channel is full, the teardown parks there.

**Fix:** Send `Close` on every terminal path and use `try_send` (or a timeout) for it;
have the coordinator drop the writer explicitly rather than relying on sender counts.

#### WR-08: `handle.events` can never close while any process holds the child's stderr

**File:** `src/executor/claude.rs:346` (stderr reader gets an `events_tx` clone), `src/executor/claude.rs:969` (`drop(events_tx)`)

**Issue:** The documented consumer idiom is `while let Some(event) = handle.events.recv().await`
(used in `tests/executor_transport.rs:152` and `:320`). That loop ends only when every
`events_tx` clone is dropped — including the one owned by `read_stderr`, which returns
only on stderr EOF. A surviving descendant holding stderr therefore hangs every consumer,
even when the coordinator has already published the outcome.

**Fix:** Have the coordinator own the reader task handles and `abort()` them after the
final `Exited` event, or move the stderr reader's output through `reader_rx` so the
coordinator is the sole owner of `events_tx`.

#### WR-09: The SIGKILL escalation is proven only by an `#[ignore]`d test

**File:** `tests/executor_lifecycle.rs:223-269`, `docs/TESTING.md:81-101`

**Issue:** `a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped` is the
only coverage for step 3/step 4 of the teardown, and it is `#[ignore]`d, so `cargo test`
reports green on a regression that breaks escalation entirely. `docs/TESTING.md` documents
the risk honestly but "run it before tagging" is a manual gate with no enforcement, and
`CLAUDE.md`'s release checklist (`cargo build && cargo test && cargo clippy`) does not
include `--ignored`.

**Fix:** Make `TEARDOWN_GRACE` configurable through `ExecutionOptions` (as the caps
already are) so the escalation can be exercised with a 200 ms grace in the default suite;
keep the 10-second test as the `#[ignore]`d realism check. Failing that, add
`cargo test -- --ignored` to the release checklist in `CLAUDE.md`.

#### WR-10: `capture_snapshot`'s fallback runs full-tree I/O and two git subprocesses on the async reactor

**File:** `src/executor/claude.rs:510-515`

**Issue:** `spawn_blocking(...).await.unwrap_or_else(|_| RunSnapshot::capture(&fallback))`
re-runs the whole capture inline on a reactor thread when the blocking task fails or is
cancelled — the exact thing `RunSnapshot::capture`'s doc comment
(`src/executor/outcome.rs:50-57`) forbids. During runtime shutdown, when `spawn_blocking`
reliably fails, this stalls the reactor with two `git` subprocess spawns.

**Fix:** Return a default/`None` snapshot on join failure and record the degradation,
rather than silently doing the blocking work in the wrong place.

#### WR-11: Safety mitigations implemented and tested but not reachable from any production path

**File:** `src/executor/outcome.rs:181` (`derive_run_outcome_from_envelopes`), `src/executor/outcome.rs:136` (`run_turn_count`), `src/executor/claude.rs:147` (`interrupt_stopped_a_turn`), `src/executor/outcome.rs:86-128` (`DiskDelta` as a public type), `src/executor/outcome.rs:70` (`changed_since`)

**Issue:** Grep across `src/` and `tests/` shows every one of these is referenced only
from `#[cfg(test)]` modules or integration tests. `interrupt_stopped_a_turn` is the
documented mitigation for the T-15-18 repudiation threat, and nothing in the driver calls
it; `run_turn_count` is the D-29 "sum, don't read the last envelope" helper, unused. The
export surface therefore advertises guarantees the shipped code does not apply (CR-04 is
the case where that is actively harmful).

**Fix:** Wire them where they belong (CR-04 covers the denials path), or mark the
deferred ones explicitly — e.g. `#[doc(hidden)]` plus a `// Phase 17 wires this` note —
so a reader cannot mistake presence for coverage.

#### WR-12: New unit tests use predictable `/tmp` paths with `remove_dir_all` instead of `tempfile::TempDir`

**File:** `src/executor/outcome.rs:615-620`, `src/executor/outcome.rs:651-656`

**Issue:** `std::env::temp_dir().join(format!("gsd_outcome_test_artifacts_{}", process::id()))`
followed by `remove_dir_all` is a predictable-path pattern: on a shared `/tmp` another user
can pre-create or symlink the path, and `remove_dir_all` will follow a symlinked
subdirectory. It is also fragile — the pid is stable across a whole `cargo test` process,
so two tests sharing a prefix would collide. `tempfile` is already a dev-dependency and is
used correctly in the integration tests added by this same phase
(`tests/executor_transport.rs:128`).

**Fix:** Use `tempfile::TempDir::new()` in these two tests, matching the phase's own
integration-test convention.

#### WR-13: `budget_usd` reaches the argv unvalidated

**File:** `src/executor/claude.rs:201-204`

**Issue:** `budget.to_string()` is pushed straight after `--max-budget-usd`. A negative
value yields a flag-shaped token (`-5`), and `f64::NAN`/`INFINITY` yield `NaN`/`inf`. The
file's own T-15-06 argument ("a path containing a flag-shaped token cannot become a flag")
is about *paths*; nothing validates this numeric field, and it will be config-driven in
Phase 17.

**Fix:** Reject non-finite and non-positive budgets at the `ExecutionOptions` boundary
(or clamp and log), rather than forwarding them to the CLI.

#### WR-14: `ExecutionHandle::pending_control` is `pub`, and `with_program` is a public arbitrary-program seam

**File:** `src/executor/mod.rs:330`, `src/executor/claude.rs:245-250`

**Issue:** `pending_control` is the run's correlation state; exposing it `pub` lets any
caller insert or remove waiters and desynchronise `interrupt()`. Every other channel/state
field on the handle is `pub(crate)` for exactly that reason. Separately,
`ClaudeExecutor::with_program` is a public constructor that runs an arbitrary binary with
arbitrary leading argv — a deliberate test seam, but the D-23 `DrivableProject` token
guards only the *directory*, so the "single spawn seam" property is weaker than the module
doc claims.

**Fix:** Make `pending_control` `pub(crate)`; gate `with_program` behind
`#[cfg(any(test, feature = "test-support"))]` or `#[doc(hidden)]`.

#### WR-15: Transcript redaction is incomplete — the operator's username survives in dash-encoded paths

**File:** `tests/fixtures/transcripts/*.ndjson` (e.g. `01-success-textonly.ndjson:1`), redaction record at `tests/fixtures/transcripts/README.md:27-59`

**Issue:** The transform replaced the slash-form home prefix but not Claude's dash-encoded
project-directory form, so every fixture except `08` still contains
`-home-blk-projects-rust-gsd-meta-manager` inside `cwd` and
`/home/testuser/.claude/projects/-tmp-claude-1000--home-blk-...`. The README asserts "zero
hits for any absolute `/home/<user>` other than the placeholder", which is literally true
and materially misleading: the username is disclosed, and git history makes it permanent
(the README's own Pitfall G). No credential material was found — `sk-`, `ghp_`, `Bearer`,
`AKIA`, `xox*` and email patterns all return zero hits.

**Fix:** Extend the transform to the dash-encoded form (`-home-<user>-` → `-home-testuser-`,
and the `-tmp-claude-<uid>-` prefix), rewrite the eight fixtures, and correct the redaction
record to state which forms were covered.

#### WR-16: `tests/fixtures/transcripts/README.md` documents a staging directory that no longer exists

**File:** `tests/fixtures/transcripts/README.md:61-69`

**Issue:** "The unredacted captures **remain** in the gitignored
`.planning/phases/15-transport-foundation/transcripts-raw/`" — that directory has been
deleted (plan 15-02 shipped). A reader auditing for unredacted material is sent to a path
that is not there, and the paragraph also describes plan 15-02's deletion as still pending.

**Fix:** Rewrite the section in the past tense and state that the staging directory was
removed by plan 15-02.

#### WR-17: The per-alias run state can never leave `Stopping`, and entries are never removed

**File:** `src/app.rs:189-210`, `src/ui/screens/mod.rs:143-147`

**Issue:** `apply_exec_event` maps `Exited` → `RunState::Stopping` and has no transition to
`Finished`; the doc comment correctly explains why an outcome cannot be fabricated here, but
the consequence is that a completed run reads as "teardown in progress" forever, and
`run_states` entries are never cleared. No screen reads the map today, so nothing is
user-visible yet — which is precisely the window in which to fix it. `Stopping` also
double-books: a run being cancelled and a run that has already finished are indistinguishable.

**Fix:** Have Phase 17's driver (which owns `wait_outcome()`) write
`RunState::Finished(outcome)` and remove the alias when a new run starts; until then,
introduce a distinct `Exited` placeholder so `Stopping` keeps its documented meaning.

---

_Reviewed: 2026-07-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
