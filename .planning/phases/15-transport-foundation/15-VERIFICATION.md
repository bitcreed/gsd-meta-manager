---
phase: 15-transport-foundation
verified: 2026-07-29T00:00:00Z
status: passed
score: 4/4 roadmap success criteria verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 2/4 roadmap success criteria verified
  gaps_closed:
    - "SC-2 / TRANS-02 / CR-04 — a permission-blocked run reported as success"
    - "SC-1 / TRANS-01 / CR-01, CR-02, CR-03 — three reachable unbounded-hang paths in the supervisor and in interrupt()"
  gaps_remaining: []
  regressions: []
deferred: []
---

# Phase 15: Transport Foundation Verification Report

**Phase Goal:** The tool can run a GSD command through `claude -p` over a structured
two-way protocol and know exactly how it ended
**Verified:** 2026-07-29
**Status:** passed
**Re-verification:** Yes — after gap closure (plans 15-07 and 15-08, closing the two gaps
recorded in this file's prior version at commit `d2d8761`)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A real multi-step GSD skill runs headlessly against a scratch project to completion, without hanging | ✓ VERIFIED | The three CR-01/CR-02 defects are closed and independently re-derived from source, not from the SUMMARYs. `Coordinator::run` (`src/executor/claude.rs:943-1141`) now evaluates the wall-clock cap, the idle cap, the grace deadline, the post-exit drain deadline and the cancel signal in an **unconditional block at the top of every loop pass** (lines 963-1025), before `select!` runs — confirmed by direct reading, not by trusting the doc comment. The hand-off to the caller goes through a single `forward()` funnel (line 1266) bounded by `tokio::time::timeout_at(forward_deadline, ...)`, where `forward_deadline` is the earliest of `EVENT_FORWARD_TIMEOUT` (5s) and every armed bound (lines 1031-1040) — `grep -cE "events_tx$" src/executor/claude.rs` returns `0` (all eight direct sends inside `handle_item` now route through `forward`), confirmed live. The post-exit drain (`POST_EXIT_DRAIN_CAP`, 5s) is armed by the exit arm and read by a `select!` arm gated **only** on `drain_deadline.is_some()` (line 1117), explicitly not on `exited` — and the post-loop dispatch (lines 1166-1196) runs the full four-step teardown whenever `exited && drain_expired`, discarding the teardown's own status and reporting the leader's already-observed one. I independently ran the three regression tests by name (not trusting the SUMMARY's numbers): `a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked` (8.02s, PASS, never reads `handle.events`), `a_cancel_is_still_honoured_while_the_event_consumer_is_blocked` (20.12s, PASS), `a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run` (5.01s, PASS, asserts the descendant is gone within 5s). All three carry their own hard `tokio::time::timeout` (60s), so a regression fails rather than hangs. The previously `#[ignore]`d SIGKILL-escalation test (`a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped`) is un-ignored and passes in 10.01s, independently confirmed; `grep -rnE "^[[:space:]]*#\[ignore" tests/ src/` returns zero hits tree-wide. |
| 2 | A run's outcome — succeeded, errored, permission-denied, killed — is reported from the `type:"result"` envelope, exit code, disk state, and git, and is correct even when the agent's own prose summary says otherwise | ✓ VERIFIED | CR-04 is closed. `src/executor/outcome.rs` now exposes exactly **one** run-outcome entry point — `grep -cE "^pub fn derive_run_outcome" src/executor/outcome.rs` returns `1`, and `grep -rn "derive_run_outcome" src/ | grep -v "_from_envelopes"` returns **zero hits** across the whole `src/` tree: the envelope-discarding sibling is deleted, not deprecated. The production call site is `src/executor/claude.rs:1221` (`None => derive_run_outcome_from_envelopes(&envelopes, status, &before, &after)`), reached from a coordinator that collects `Vec<ResultMessage>` (the local `envelopes`, line 917) rather than a `TurnOutcome` projection — `handle_item`'s `StreamMessage::Result` arm (line 1397) pushes the full envelope, including `permission_denials[]`, before the box moves into `ExecutionEvent::TurnCompleted`. I independently ran `a_permission_blocked_run_is_reported_as_permission_denied_not_success` (`tests/executor_transport.rs:274`) by name: PASS in 0.01s, driving a real spawned stand-in (`fake-claude-slow.sh`'s new `denied` ending, verified to emit a populated `permission_denials` array with no host path) through the real `Coordinator`, asserting `RunOutcome::PermissionDenied { denials }` with `denials.len() == 1`. `outcome.rs`'s own unit test `the_agents_prose_summary_changes_nothing_about_the_outcome` (prose-immunity) is unregressed. |
| 3 | The TUI keeps redrawing and accepting keypresses while a run streams output for minutes at a time | ✓ VERIFIED (unregressed) | `src/main_loop.rs` was fenced out of both gap-closure plans and is byte-identical to its pre-replan state — `git diff --name-only 5dbfb63..HEAD -- src/main_loop.rs src/executor/gate.rs` returns empty, confirmed directly, and `git status --porcelain src/main_loop.rs src/executor/gate.rs` is empty on the current tree. The three `pump()` responsiveness tests (`keypress_is_handled_before_a_flood_of_executor_events`, `executor_burst_is_drained_in_bounded_batches`, `tui_renders_repeatedly_while_the_executor_channel_is_saturated`) still pass — `cargo test --lib main_loop` reports 3 passed, independently re-run. Light regression confirmation only, per the re-verification instructions; not re-derived from scratch. |
| 4 | Starting a run against a Claude CLI missing a required `system/init` capability is refused up front, naming the missing capability, rather than failing mid-run | ✓ VERIFIED (unregressed) | `src/executor/gate.rs` was likewise fenced out and untouched (same `git diff` above). `a_refused_run_writes_zero_bytes_to_the_child_stdin` (now living in `src/executor/claude.rs`'s own `#[cfg(test)] mod tests`, not `tests/executor_transport.rs` — a harmless relocation, not a regression) was independently re-run by name: PASS. The gate's wiring into `Coordinator::run`'s first-`system/init` arm (lines 1334-1373) is unchanged by either gap-closure plan, and the later-init informational arm beside it (D-30) still forwards verbatim without re-running the gate. Light regression confirmation only, per the re-verification instructions; not re-derived from scratch. |

**Score:** 4/4 roadmap success criteria verified as met by the shipped code.

### Gap Closure Verification (this pass's primary job)

**GAP 1 (SC-2 / TRANS-02 / CR-04) — both `missing:` bullets closed:**

1. *"Coordinator::run must collect Vec<ResultMessage> ... and call derive_run_outcome_from_envelopes"* — **CLOSED.** Verified directly against source: `envelopes: Vec<ResultMessage>` at `claude.rs:917`, pushed at `claude.rs:1397`, consumed at the call site `claude.rs:1221`. The envelope-discarding entry point is deleted from `outcome.rs`, not merely unreferenced (`grep -rn "derive_run_outcome" src/ | grep -v "_from_envelopes"` = 0 hits).
2. *"An end-to-end regression test driving a stand-in whose terminal envelope carries a non-empty permission_denials[] through the real Coordinator"* — **CLOSED.** `a_permission_blocked_run_is_reported_as_permission_denied_not_success` independently re-run: PASS, 0.01s, real spawned child, real `Coordinator`.

**GAP 2 (SC-1 / TRANS-01 / CR-01, CR-02, CR-03) — all four `missing:` bullets closed:**

1. *"Deadlines and cancel must be evaluated unconditionally each loop pass ... and the forward to events_tx must be bounded"* — **CLOSED.** Verified directly: the unconditional block at `claude.rs:963-1025`, and the bounded `forward()` funnel at `claude.rs:1266-1286` used by all eight prior direct sends (`grep -cE "events_tx$" src/executor/claude.rs` = 0; the lone remaining direct send, the post-loop `Exited` send, is itself wrapped in `tokio::time::timeout` at line 1232 — a deviation the SUMMARY flagged and I independently confirmed in the source, since an unbounded send there would have been a third way to lose `outcome_tx`).
2. *"An absolute post-exit drain bound (not guarded by !exited) that still calls tear_down_group / terminate_group when the group has not been proven reaped"* — **CLOSED.** `POST_EXIT_DRAIN_CAP` (line 171) is armed unconditionally in the exit arm (line 1081) and its `select!` arm is gated only on `drain_deadline.is_some()` (line 1117), verified to have no `exited` conjunct in that guard by direct reading. The post-loop dispatch (lines 1166-1196) runs the four-step teardown on `exited && drain_expired`.
3. *"pending_control cleared at run end ... and/or a bounded timeout on interrupt()'s rx.await"* — **CLOSED, both, not either.** `pending_control.lock().await.clear()` appears exactly once (`claude.rs:1245`), immediately before `outcome_tx` is sent; `interrupt()` wraps its `rx.await` in `tokio::time::timeout(handle.control_response_cap, rx)` (`claude.rs:540`), default 30s. Both release paths independently re-run by name: `an_unanswered_interrupt_is_released_by_the_run_end_drain` (0.61s) and `an_unanswered_interrupt_on_a_live_child_is_released_by_the_control_response_cap` (0.41s), both PASS.
4. *"A regression test exercising each of the three conditions"* — **CLOSED**, one test per condition, all independently re-run by name and all PASS: fast-stream/blocked-consumer → `a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked` + `a_cancel_is_still_honoured_while_the_event_consumer_is_blocked`; exited-but-descendant-alive → `a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run`; interrupt-with-no-response → the two tests in bullet 3. Every one of these five tests carries its own hard `tokio::time::timeout`, confirmed by direct reading of each test body, not by trusting the SUMMARY's claim.

No `missing:` bullet from the prior verification is left open.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/executor/claude.rs` | coordinator collects `Vec<ResultMessage>`; per-pass unconditional bound evaluation; bounded `forward()`; `interrupt` cap-bounded; run-end `pending_control` drain; post-exit drain always reaps | ✓ VERIFIED | All confirmed by direct line-level reading, not SUMMARY trust. |
| `src/executor/mod.rs` | `ExecutionOptions::control_response_cap` and matching `ExecutionHandle` field | ✓ VERIFIED | `mod.rs:272` (option, default 30s at `:291`), `:349` (handle field), unit test at `:666`. |
| `src/executor/outcome.rs` | single, denials-aware run-outcome entry point | ✓ VERIFIED | Exactly one `pub fn derive_run_outcome*`; module doc rewritten to state this. |
| `tests/fixtures/fake-claude-slow.sh` | `denied` ending emitting populated `permission_denials[]`; zero-interval flood fast path | ✓ VERIFIED | Both present; no host path (grep for literal and dash-encoded forms returns 0). |
| `tests/fixtures/fake-claude-orphan.sh` | descendant that outlives the leader and holds stdout | ✓ VERIFIED | New file, executable, no `wait`, fully synthetic, read in full. |
| `tests/executor_transport.rs` | permission-denied end-to-end regression test | ✓ VERIFIED | Read in full; real `Coordinator`, real spawned stand-in. |
| `tests/executor_lifecycle.rs` | five new regression tests (2 interrupt, 2 flood, 1 orphan) + un-ignored escalation test | ✓ VERIFIED | All six read in full and independently re-run by name; all PASS with the measured durations matching the SUMMARY's claims. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `Coordinator::run` | `derive_run_outcome_from_envelopes` | production call site, full envelopes | ✓ WIRED | Was "NOT WIRED (partial function)" in the prior report; now confirmed wired at `claude.rs:1221`, the only entry point in existence. |
| `handle_item`'s `StreamMessage::Result` arm | the coordinator's envelope vector | `envelopes.push((*result).clone())` before the box moves | ✓ WIRED | `claude.rs:1397`. |
| `ClaudeExecutor::interrupt` | `ExecutionHandle::pending_control` | registration, cap, run-end drain | ✓ WIRED (closed lifecycle) | Was "⚠️ PARTIAL" (wired but unbounded) in the prior report; now both the cap (`timeout(handle.control_response_cap, rx)`) and the run-end drain (`pending_control.lock().await.clear()`) close the lifecycle at both ends, confirmed by direct reading and by two independently-passing regression tests. |
| supervisor loop head | `wall_deadline` / `idle_deadline` / `grace_deadline` / `drain_deadline` / `cancel_rx` | unconditional per-pass evaluation | ✓ WIRED | `claude.rs:963-1025`, confirmed to run before `select!` on every pass. |
| `src/main_loop.rs` (`pump`) | `src/app.rs` (`apply_exec_event`) | executor events routed to app state | ✓ WIRED (unregressed) | File untouched by either gap-closure plan; not re-derived. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| — | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/placeholder markers found in any file touched by 15-07 or 15-08 (`src/executor/claude.rs`, `src/executor/mod.rs`, `src/executor/outcome.rs`, the four test files, the two new/modified fixtures) | — | None — clean, independently re-swept. |

### Behavioral Spot-Checks (independently run, not trusted from SUMMARY)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace build | `cargo build` | exit 0 | ✓ PASS |
| Full test suite | `cargo test` | `363 passed (7 suites, 22.60s)` — matches the measured baseline exactly | ✓ PASS |
| Lib clippy gate | `cargo clippy -- -D warnings` | `No issues found` | ✓ PASS |
| `--all-targets` clippy delta | `cargo clippy --all-targets` (after touching a source file to bust the cache) | `0 errors, 5 warnings` — 3x `browser.rs` literal-bool `assert_eq!`, 1x `project_creator.rs` owned-instance, 1x `state_reader/mod.rs` items-after-test-module — none in an executor or test file | ✓ PASS |
| Permission-denied regression (named) | `cargo test --test executor_transport a_permission_blocked_run_is_reported_as_permission_denied_not_success` | 1 passed, 0.01s | ✓ PASS |
| Interrupt regressions (named) | `cargo test --test executor_lifecycle an_unanswered_interrupt` | 2 passed, 0.61s | ✓ PASS |
| Wall-clock-under-flood regression (named) | `cargo test --test executor_lifecycle a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked` | 1 passed, 8.02s | ✓ PASS |
| Cancel-under-flood regression (named) | `cargo test --test executor_lifecycle a_cancel_is_still_honoured_while_the_event_consumer_is_blocked` | 1 passed, 20.12s | ✓ PASS |
| Orphan-descendant regression (named) | `cargo test --test executor_lifecycle a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run` | 1 passed, 5.01s | ✓ PASS |
| Un-ignored SIGKILL escalation (named) | `cargo test --test executor_lifecycle a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped` | 1 passed, 10.01s | ✓ PASS |
| Full lifecycle suite | `cargo test --test executor_lifecycle` | 11 passed, 0 failed, 0 ignored | ✓ PASS |
| Full transport suite | `cargo test --test executor_transport` | 7 passed, 0 failed | ✓ PASS |
| No ignored test anywhere | `grep -rnE "^[[:space:]]*#\[ignore" tests/ src/` | 0 hits | ✓ PASS |
| No host-path leak in new/modified fixtures | `grep -rnE "/home/[a-zA-Z]|home-blk" tests/fixtures/fake-claude-slow.sh tests/fixtures/fake-claude-orphan.sh` | 0 hits | ✓ PASS |
| Gate/main-loop scope fence held | `git diff --name-only 5dbfb63..HEAD -- src/main_loop.rs src/executor/gate.rs` | empty | ✓ PASS |
| SC-4 unregressed (named, relocated) | `cargo test --lib a_refused_run_writes_zero_bytes_to_the_child_stdin` | 1 passed | ✓ PASS |
| SC-3 unregressed | `cargo test --lib main_loop` | 3 passed | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| TRANS-01 | 15-01, 15-02, 15-03, 15-04, 15-07, 15-08 | Spawn `claude -p` with duplex stream-json, parse structured envelope not text; process never awaits exit unbounded | ✓ SATISFIED | Transport shape verified in the initial pass, unregressed; the CR-01/CR-02/CR-03 hang paths (also TRANS-01, since they defeat "know exactly how it ended" without hanging) are now closed and independently re-verified above. |
| TRANS-02 | 15-01, 15-02, 15-04, 15-05, 15-07 | Outcome derived from result envelope (incl. `permission_denials[]`), exit code, disk, git — never prose | ✓ SATISFIED | CR-04 closed; the production call site now reaches the denials-aware derivation, independently confirmed by source reading and a real end-to-end test. |
| TRANS-03 | 15-02, 15-06 | TUI event loop stays responsive while a run streams output | ✓ SATISFIED (unregressed) | File untouched by gap closure; tests independently re-run. |
| TRANS-04 | 15-02, 15-03 | Runtime capability detection via `system/init`, refuses unsupported CLI with clear message | ✓ SATISFIED (unregressed) | File untouched by gap closure; test independently re-run. |

No orphaned requirements: all four IDs declared across the phase's eight plans (15-01 through 15-08) match exactly the four TRANS IDs REQUIREMENTS.md maps to Phase 15. TRANS-05 is correctly out of scope (mapped to Phase 18 in REQUIREMENTS.md).

**Note:** `.planning/REQUIREMENTS.md` lines 38-40 still show `[ ]` (unchecked) for TRANS-02/03/04 as of this verification pass — only TRANS-01 is checked. This is a requirements-tracking-document staleness issue, not a code gap: the shipped code satisfies all four, as detailed above. Flagged here as an administrative follow-up (updating the checkboxes), not a phase gap.

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` convention. The gap-closure verification instead ran the specific named regression tests the two gap-closure plans introduced, plus the full project gate, all independently re-executed in this verification pass rather than trusted from either SUMMARY.

### Human Verification Required

None. Every truth resolved to VERIFIED by direct code inspection cross-referenced against the prior verification's specific line citations, corroborated by independently re-running every named regression test (not merely reading the SUMMARY's reported numbers). Nothing here requires a human to observe runtime/visual behavior that grep and a real test run cannot see.

### Gaps Summary

None. Both gaps recorded in the prior verification (`d2d8761`) are closed:

- **GAP 1 (SC-2 / TRANS-02 / CR-04)**: the production coordinator now collects full `ResultMessage` envelopes and calls the single, denials-aware `derive_run_outcome_from_envelopes` — the envelope-discarding sibling is deleted from the codebase, not merely unreferenced, confirmed by a zero-hit grep across all of `src/`. An end-to-end test drives a real spawned stand-in through the real `Coordinator` and asserts `RunOutcome::PermissionDenied`.
- **GAP 2 (SC-1 / TRANS-01 / CR-01, CR-02, CR-03)**: the supervisor now evaluates every deadline and the cancel signal unconditionally at the top of every loop pass, independent of `select!` arm ordering; the hand-off to the caller is bounded by the earliest armed deadline so a stalled consumer cannot disable the caps or cancel; the post-exit drain is bounded and explicitly not guarded by the `exited` flag, and the post-loop dispatch always tears the group down when it has not been proven reaped; `interrupt()`'s wait is bounded by a configurable cap and `pending_control` is cleared at run end, closing the control-response lifecycle at both ends. Five new regression tests, each carrying its own hard timeout, independently confirmed passing at the durations the SUMMARY reported.

SC-3 and SC-4 were confirmed unregressed via a light check (file-untouched confirmation plus an independent re-run of their existing tests), per the re-verification scope — not re-derived from scratch.

One administrative note, not a gap: `.planning/REQUIREMENTS.md`'s checkboxes for TRANS-02/03/04 remain unchecked and should be updated to reflect that all four Phase 15 requirements are now satisfied.

---

_Verified: 2026-07-29_
_Verifier: Claude (gsd-verifier)_
