---
phase: 15-transport-foundation
verified: 2026-07-29T00:00:00Z
status: gaps_found
score: 2/4 roadmap success criteria verified (2 failed)
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "A run's outcome — succeeded, errored, permission-denied, killed — is reported from the type:\"result\" envelope, exit code, disk state, and git, and is correct even when the agent's own prose summary says otherwise (roadmap SC-2)"
    status: failed
    reason: >
      Confirmed by direct code inspection (matches code review CR-04). The
      production coordinator (src/executor/claude.rs:962) calls
      `derive_run_outcome(&turns, ...)`, and `derive_run_outcome` (outcome.rs:165-172)
      hard-codes `Vec::new()` for the permission-denials source. The only entry
      point that reads `permission_denials[]` is
      `derive_run_outcome_from_envelopes` (outcome.rs:181), and a full-tree grep
      (`grep -rn "derive_run_outcome_from_envelopes" src/`) shows it is referenced
      only from outcome.rs's own `#[cfg(test)]` module — never from
      src/executor/claude.rs. Consequently a run that `--permission-mode dontAsk`
      blocked (a `success` envelope with a populated `permission_denials[]` and no
      disk delta) falls through to `RunOutcome::SucceededNoChanges` in production —
      reported as a success. This is not a missing feature; it is a wrong answer
      for exactly the untrusted-workspace case the phase's own spike (P2 finding
      in 15-SPIKE-OQ1.md) surfaced as a real, silent failure mode.
    artifacts:
      - path: "src/executor/claude.rs"
        issue: "Line 962 calls derive_run_outcome (envelope-discarding path) instead of derive_run_outcome_from_envelopes; TurnOutcome::from_result (line 1057) is built directly from the result envelope and the envelope itself is dropped, so permission_denials[] never survives to the derivation."
      - path: "src/executor/outcome.rs"
        issue: "derive_run_outcome (lines 165-172) is a real production entry point that silently discards the denials source; only its sibling derive_run_outcome_from_envelopes reads permission_denials[], and it is unreachable from src/."
    missing:
      - "Coordinator::run must collect Vec<ResultMessage> (the full envelopes) rather than only Vec<TurnOutcome>, and call derive_run_outcome_from_envelopes at claude.rs:962."
      - "An end-to-end regression test driving a stand-in whose terminal envelope carries a non-empty permission_denials[] through the real Coordinator (not just the outcome.rs unit tests), asserting RunOutcome::PermissionDenied is reported."
  - truth: "The tool can run a GSD command through claude -p ... and know exactly how it ended, without hanging (roadmap SC-1, phase goal)"
    status: failed
    reason: >
      Confirmed by direct code inspection (matches code review CR-01/CR-02/CR-03).
      The phase's own doc comment on Coordinator::run claims "no path in this file
      awaits process exit unbounded" (D-13), but three independent, reachable code
      paths contradict that:
      (1) The supervisor's tokio::select! at claude.rs:835-893 is `biased;` with
      the reader arm first (line 838), and that arm's body `.await`s
      `handle_item(...)` — itself calling `events_tx.send(...).await` on a
      *bounded* 8192-capacity channel — OUTSIDE the select. While that await is
      pending, no other arm (wall-clock deadline, idle deadline, or cancel) is
      polled at all, so a fast-emitting child or a stalled TUI consumer disables
      every cap and the cancel signal simultaneously.
      (2) Once `exited` flips true (claude.rs:864-867), every deadline/grace arm
      is guarded `if !exited` (lines 869, 874, 889) and the post-loop dispatch at
      line 927 (`if exited { exit_status }`) skips tear_down_group entirely. The
      only remaining live arms are reader_rx.recv() (which returns only on stdout
      EOF — i.e. only once every process holding the write end of that pipe,
      including any backgrounded Bash grandchild, is gone) and cancel_rx (which,
      per its guard chain, can no longer trigger any actual signal or bounded
      wait once exited is true). A live descendant holding stdout after the
      leader exits therefore hangs the coordinator, `wait_outcome()`, and
      `Executor::cancel()` for the process lifetime — never sending outcome_tx.
      (3) `interrupt()` (claude.rs:462-489) inserts a oneshot into
      `pending_control` and awaits it with no timeout; a full-tree grep confirms
      no call anywhere clears `pending_control` at run end (only the two
      per-request removal sites at lines 484 and 1080 exist), so an unanswered
      interrupt hangs the caller for the process lifetime.
      15-SPIKE-OQ1.md's real 774.6s /gsd-execute-phase run is genuine evidence
      that the happy path completes cleanly, but it does not exercise any of
      these three conditions (fast stream + blocked consumer, an orphaned
      descendant surviving the leader, or an unanswered interrupt), so it cannot
      stand in for them. The phase goal's own wording — "know exactly how it
      ended" — is precisely what these three paths defeat: the coordinator can
      reach a state where outcome_tx is never sent and the caller has no way to
      learn anything.
    artifacts:
      - path: "src/executor/claude.rs"
        issue: "Coordinator::run's supervisor loop (789-972): biased select prioritizes the reader arm ahead of both deadline arms and cancel, and the reader arm's body performs an unbounded send outside the select; post-exit guards (`if !exited`) disable every remaining cap/cancel escalation, and the reader-EOF-only escape can be defeated by a surviving descendant holding stdout."
        issue2: "interrupt() (462-489) awaits an un-timeout-bounded oneshot with no run-end drain of pending_control."
    missing:
      - "Deadlines and cancel must be evaluated unconditionally each loop pass (not only when select! reaches them), and the forward to events_tx must be bounded (timeout or try_send-with-overflow) so a stalled consumer cannot park the supervisor with every cap disabled."
      - "An absolute post-exit drain bound (not guarded by !exited) that still calls tear_down_group / terminate_group when the group has not been proven reaped, so a descendant holding stdout cannot hang the run forever."
      - "pending_control cleared at run end (dropping every waiter) and/or a bounded timeout on interrupt()'s rx.await, so an unanswered interrupt cannot hang the caller for the process lifetime."
      - "A regression test exercising each of the three conditions (fast stream/blocked consumer, exited-but-descendant-alive, interrupt with no control_response) — none of the existing tests reach any of them (tests/executor_lifecycle.rs's idle-cap test paces heartbeats at 50ms with an immediately-drained channel)."
deferred: []
---

# Phase 15: Transport Foundation Verification Report

**Phase Goal:** The tool can run a GSD command through `claude -p` over a structured
two-way protocol and know exactly how it ended
**Verified:** 2026-07-29
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A real multi-step GSD skill runs headlessly against a scratch project to completion, without hanging | ✗ FAILED | 15-SPIKE-OQ1.md proves the **happy path** completes (exit 0, 774.6s, real disk/git changes) — genuine positive evidence. But direct code inspection of `src/executor/claude.rs` confirms three reachable, unbounded-hang code paths the phase's own doc comments claim do not exist (CR-01/CR-02/CR-03, verified below). The general "without hanging" guarantee is not held by the shipped code. |
| 2 | A run's outcome — succeeded, errored, permission-denied, killed — is reported from the `type:"result"` envelope, exit code, disk state, and git, and is correct even when the agent's own prose summary says otherwise | ✗ FAILED | Prose-immunity is real and tested (`the_agents_prose_summary_changes_nothing_about_the_outcome`, outcome.rs:1026). But the production call site (`claude.rs:962`) never reaches the denials-aware derivation (`derive_run_outcome_from_envelopes`) — confirmed by grep across `src/`. A `dontAsk`-blocked run is reported as `SucceededNoChanges` (success), directly contradicting the "permission-denied" case this criterion explicitly names. |
| 3 | The TUI keeps redrawing and accepting keypresses while a run streams output for minutes at a time | ✓ VERIFIED | `src/main_loop.rs`'s `pump()` is a biased `select!` with input first, a bounded (`EXEC_BATCH=64`) executor-event drain second, and a 16ms redraw tick third. Three deterministic property tests confirm: a keypress is serviced ahead of 10,000 queued executor events on the very first `pump` call (`keypress_is_handled_before_a_flood_of_executor_events`), a burst drains in bounded batches (`executor_burst_is_drained_in_bounded_batches`), and frames keep rendering (>20 in 2s) while the channel stays saturated under a real multi-threaded flooder (`tui_renders_repeatedly_while_the_executor_channel_is_saturated`). |
| 4 | Starting a run against a Claude CLI missing a required `system/init` capability is refused up front, naming the missing capability, rather than failing mid-run | ✓ VERIFIED | `src/executor/gate.rs`'s `validate_first_init` fails closed on missing/empty capabilities, unreadable/absent version, and non-subscription/absent auth source, naming every missing capability (`CapabilityError::MissingCapabilities`). Wired: `claude.rs`'s `Coordinator` blocks `start_run`'s `gate_rx.await` until the first `system/init` is validated, and only then releases the first message to stdin (`handle_item`, claude.rs:1005-1041). `a_refused_run_writes_zero_bytes_to_the_child_stdin` (claude.rs:1459) proves the refusal costs zero stdin bytes end-to-end through a real spawned stand-in, not just a unit-level gate check. |

**Score:** 2/4 roadmap success criteria verified as met by the shipped code.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| TRANS-01 | 15-01, 15-02, 15-03 | Spawn `claude -p` with duplex stream-json, parse structured envelope not text | ✓ SATISFIED | `build_argv` emits the duplex stream-json baseline; `stream_json.rs` is a tolerant serde model verified line-by-line against 8 golden transcripts; parsing happens only in the reader task, never the render thread. |
| TRANS-02 | 15-01, 15-02, 15-04, 15-05 | Outcome derived from result envelope (incl. `permission_denials[]`), exit code, disk, git — never prose | ✗ BLOCKED | See gap above (CR-04): the requirement's own text names `permission_denials[]` as a required source, and it is unreachable from the shipped production path. |
| TRANS-03 | 15-02, 15-06 | TUI event loop stays responsive while a run streams output | ✓ SATISFIED | See Truth 3. |
| TRANS-04 | 15-02, 15-03 | Runtime capability detection via `system/init`, refuses unsupported CLI with clear message | ✓ SATISFIED | See Truth 4. |

No orphaned requirements: all four IDs declared across the phase's plan frontmatter (`requirements:` fields in 15-01 through 15-06) match exactly the four TRANS IDs REQUIREMENTS.md maps to Phase 15. TRANS-05 is correctly out of scope (mapped to Phase 18 in REQUIREMENTS.md).

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `src/executor/claude.rs` (Coordinator) | `src/executor/gate.rs` | `validate_first_init` called before first message released to stdin | ✓ WIRED | Confirmed at claude.rs:1005-1041: gate runs on the first `system/init`, prompt released only in the `Ok` branch. |
| `src/executor/claude.rs` (Coordinator) | `ExecutionHandle.pending_control` | interrupt registers oneshot under request id, reader resolves it | ⚠️ PARTIAL | Registration/resolution wiring exists and is tested (`a_second_system_init_...`, handle_item's `ControlResponse` arm at 1078-1089), but the map is never drained at run end (CR-03) — the *link* is wired, the *lifecycle* around it is not bounded. |
| `src/executor/claude.rs` (Coordinator) | `derive_run_outcome*` in `src/executor/outcome.rs` | Coordinator calls the outcome derivation with full envelopes | ✗ NOT WIRED (partial function) | Coordinator calls `derive_run_outcome` (envelope-discarding), never `derive_run_outcome_from_envelopes` (denials-aware). See CR-04 gap above. |
| `src/main_loop.rs` (`pump`) | `src/app.rs` (`apply_exec_event`) | executor events routed to app state | ✓ WIRED | Confirmed and property-tested. |
| `src/executor/git_ops.rs` (`head_sha`, `is_dirty`) | `src/executor/outcome.rs` (`RunSnapshot::capture`) | git half of the disk/git delta | ✓ WIRED | `RunSnapshot::capture` calls both helpers; tested end-to-end in `capture_fills_the_git_half_in_a_real_repository`. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| — | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/placeholder markers found in any phase-15 file (`src/executor/*`, `src/main_loop.rs`, `src/app.rs`, `src/lib.rs`, `src/error.rs`) | — | None — clean on this axis. |

The two BLOCKER-class defects found are not debt markers or stubs; they are logic errors in otherwise substantive, well-tested, well-documented code (a `biased select!` ordering choice with an unbounded await inside the winning arm's body, and a hard-coded `Vec::new()` in a function that has a fully-correct sibling). This is the "artifact present, substantive, wired — but the invariant it's supposed to hold does not hold" failure mode the goal-backward methodology is built to catch, and it was caught here by cross-referencing the code review's specific line citations against the actual file.

### Independently confirmed code-review findings (BLOCKER)

I read `src/executor/claude.rs` and `src/executor/outcome.rs` directly (not merely trusting 15-REVIEW.md's prose) and independently confirm:

- **CR-01** (starvation/suspension of deadlines and cancel): confirmed. `biased;` at claude.rs:836, reader arm first at 838, `handle_item(...).await` inside that arm's body (not inside the `select!`), `events_tx.send(...).await` inside `handle_item` on the 8192-capacity bounded channel (claude.rs:990-1088).
- **CR-02** (post-exit hang, no teardown): confirmed. Every deadline/grace arm guarded `if !exited` (claude.rs:869, 874, 889); post-loop dispatch (`if exited { exit_status }`, line 927) never calls `tear_down_group` when `exited` is true.
- **CR-03** (interrupt can hang forever): confirmed. No call site anywhere clears `pending_control` at run end; `rx.await` at claude.rs:481 has no timeout. Full grep of `pending_control` across `src/executor/claude.rs` and `src/executor/mod.rs` shows only per-request insert/remove, no run-end drain.
- **CR-04** (permission-denied misreported as success): confirmed. `claude.rs:962` calls `derive_run_outcome` (outcome.rs:165-172, hard-codes `Vec::new()` for denials); `derive_run_outcome_from_envelopes` (outcome.rs:181-196, the denials-aware entry point) is referenced only by its own test module — grep across `src/` and `tests/` confirms zero non-test call sites.
- **WR-15** (dash-encoded host paths in fixtures): I did not re-scan this in depth per the orchestrator's note that it was already remediated in commit `0a9b6d8` and re-scanned clean; treating as resolved per that instruction.

I did not independently re-verify every one of the 17 WARNING items in 15-REVIEW.md line-by-line (WR-01 through WR-14, WR-16, WR-17); they describe secondary robustness/hygiene gaps (grace-period race, breach-vs-cancel ordering, dropped turn accounting on TimedOut/Stalled, unbounded stdin-writer leak on clean runs, stderr-reader keeping `handle.events` open forever, `#[ignore]`d SIGKILL test, blocking I/O on join failure, unreachable safety mitigations beyond CR-04, predictable `/tmp` paths in two unit tests, unvalidated `budget_usd` reaching argv, over-broad `pub` visibility on `pending_control`/`with_program`, stale README section, and `RunState` stuck at `Stopping` forever) — none of these independently changes the pass/fail verdict on the four roadmap success criteria, but they represent real follow-up work and are consistent with (not contradicted by) my own reading of the files I did inspect.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full test suite passes (already verified by orchestrator; not re-run in full per spot-check constraints) | `cargo test` | 355 passed, 0 failed, 1 ignored | ✓ PASS (relied on orchestrator's gate) |
| The ignored SIGKILL-escalation test passes | `cargo test -- --ignored` | passes in 10.01s | ✓ PASS (relied on orchestrator's gate) |
| `derive_run_outcome_from_envelopes` is reachable only from tests | `grep -rn "derive_run_outcome_from_envelopes" src/` | 3 hits, all in `src/executor/outcome.rs` (1 definition + 2 in its own `#[cfg(test)] mod tests`) | ✓ CONFIRMS CR-04 |
| `pending_control` is never cleared at run end | `grep -n "pending_control" src/executor/claude.rs src/executor/mod.rs` | Only insert (342, 355, 387, 473), per-request remove (484, 1080), and the field declaration (330, 716) — no `.clear()` anywhere | ✓ CONFIRMS CR-03 |
| No debt markers in phase-15 files | `grep -rniE "TBD\|FIXME\|XXX\|TODO\|HACK\|placeholder"` across `src/executor/`, `src/main_loop.rs`, `src/app.rs`, `src/lib.rs`, `src/error.rs` | No matches | ✓ PASS |

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` convention; the phase's own gating artifact is `15-SPIKE-OQ1.md`, a manually-run, narratively-documented spike (not a scripted probe), and it was read and its evidence weighed above rather than re-executed (re-running it would require spawning a real headless `claude -p` run against a fresh scratch project for ~13 minutes, which is out of scope for a verification pass and would not change the code-level findings).

### Human Verification Required

None. Every truth resolved to VERIFIED or FAILED by direct code inspection cross-referenced against the code review; nothing here requires a human to observe runtime/visual behavior that grep cannot see.

### Gaps Summary

Two of the four roadmap success criteria are not actually met by the shipped code, and both gaps trace to confirmed logic defects (not missing tests, not stubs) in the supervisor loop and the outcome derivation call site:

1. **SC-1 ("...without hanging")** is true for the demonstrated happy path (15-SPIKE-OQ1.md is genuine, valuable evidence) but false as a general guarantee: the supervisor's `biased select!` can starve every deadline and the cancel signal while draining a fast stream into a blocked consumer, and once the child is observed exited, every remaining cap/teardown path is disabled — leaving only stdout EOF as the escape, which a surviving descendant can hold open forever. `interrupt()` has the same shape of problem: an unanswered control response hangs the caller for the process's whole lifetime, with no drain at run end. These are exactly the situations the phase goal's "know exactly how it ended" promises to prevent, and the doc comments in `claude.rs` assert (incorrectly, per this reading) that they cannot occur.

2. **SC-2 ("...correct even when the agent's own prose summary says otherwise")** is true for the prose-immunity half (tested, verified) but false for the permission-denied half, which the criterion explicitly names: the production coordinator discards each `result` envelope's `permission_denials[]` and calls the outcome function that cannot see it, so a `dontAsk`-blocked run — the exact untrusted-workspace scenario the phase's own spike (P2) surfaced as a real, silently-misleading failure mode — is reported as a plain success.

Both defects are narrow, mechanical fixes (swap one call site for its already-implemented, already-tested sibling for CR-04; restructure the supervisor's deadline evaluation to not depend on `select!` reaching an arm, and bound the post-exit drain and the interrupt wait, for CR-01/02/03) rather than architectural rework — the correct logic already exists in the codebase in each case (`derive_run_outcome_from_envelopes`, the documented four-step teardown, the cap-racing design) and simply is not reached from every path it needs to be reached from.

SC-3 and SC-4 are both genuinely, substantively met: the TUI responsiveness guarantee and the capability gate are implemented, wired end-to-end (not just unit-tested in isolation), and proven under deliberately adversarial test conditions (10,000-event floods, a real spawned refusing stand-in).

---

_Verified: 2026-07-29_
_Verifier: Claude (gsd-verifier)_
