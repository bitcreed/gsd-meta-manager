---
phase: 15-transport-foundation
plan: 03
subsystem: executor-transport
tags: [capability-gate, version-floor, auth-guard, interrupt, control-request, duplex, fail-closed]
status: complete

requires:
  - phase: 15-01
    provides: "OQ1 PASS verdict; the eight redacted golden 2.1.220 transcripts"
  - phase: 15-02
    provides: "src/executor/ surface, the thin first-init gate, the tolerant NDJSON model, ClaudeExecutor and its coordinator"
provides:
  - "validate_first_init + GateOutcome — the fail-closed capability, version-floor and auth-path gate (D-06, D-07, D-08)"
  - "Version: three-component CLI version parsed and compared componentwise, never as a string"
  - "MINIMUM_CLAUDE_VERSION (2.1.214), TESTED_MAXIMUM_CLAUDE_VERSION (2.1.220), SUBSCRIPTION_API_KEY_SOURCE"
  - "CapabilityError::{VersionBelowFloor, VersionUnreadable, AuthPathChanged} + unmet_requirements()"
  - "First-init-only enforcement in the coordinator, proven against a hostile second init (D-30)"
  - "interrupt_stopped_a_turn + ABORTED_STREAMING — the only honest answer to 'did my interrupt work?' (D-31, T-15-18)"
  - "Doc contracts on send/interrupt recording dequeue-time echo semantics and request-id correlation"
  - "tests/fixtures/fake-claude-echo.sh (mode 100755) — a stdin-reactive stand-in with a stdin-recording log"
affects:
  - "15-04 (owns claude.rs lifecycle; the coordinator's permission_mode field and Coordinator destructuring changed)"
  - "15-05 (owns outcome.rs; RunOutcome::CapabilityRefused is now reachable from four refusal shapes, not one)"
  - "15-06 (consumes ExecutionEvent; unchanged)"
  - "Phase 16 (journals claude_code_version off GateOutcome), Phase 17 (owns the skills[] pre-flight gate this leaves open), Phase 18 (send/interrupt UI — see ## OQ2 finding)"

tech-stack:
  added: []
  patterns:
    - "Fail-closed on ABSENCE, not just on disagreement: an absent version and an absent apiKeySource each refuse, because absence is the shape a silent upstream regression takes"
    - "Componentwise version comparison via derived Ord on a three-field struct — field-declaration order IS the comparison order, and string comparison would rank 2.1.99 above 2.1.214"
    - "A typed error is the fidelity surface; a lossy projection (unmet_requirements) feeds the coarse UI state, so the two never drift"
    - "Mechanical grep guards force forbidden tokens to be assembled with concat! even inside the test that asserts their absence"
    - "A test fixture that TRUNCATES its observation log at startup, so an empty log proves the driver wrote nothing rather than that the child never ran"

key-files:
  created:
    - "tests/fixtures/fake-claude-echo.sh"
  modified:
    - "src/executor/gate.rs"
    - "src/executor/claude.rs"
    - "src/error.rs"
    - "tests/executor_transport.rs"

key-decisions:
  - "Gate order is version -> capabilities -> auth, because 'your CLI is too old' diagnoses a genuinely old CLI better than the capability list it would also fail"
  - "The permission-mode check is a recorded confirmation plus a warning, never a refusal — it indicts the driver's argv, not the CLI's fitness"
  - "check_init -> validate_first_init and GateFacts -> GateOutcome, matching the plan's declared exports"
  - "REQUIRED_CAPABILITIES keeps its screaming-snake name; a const named RequiredCapabilities is a non_upper_case_globals warning and therefore a -D warnings build failure"
  - "CapabilityError::unmet_requirements() rather than widening RunOutcome::CapabilityRefused, because mod.rs belongs to no plan in this wave"
  - "interrupt_stopped_a_turn reads the terminal envelope, not the ack — both golden interrupt transcripts produce an identical ack"

requirements-completed: [TRANS-01, TRANS-04]

coverage:
  - id: D1
    description: "A CLI missing a required capability is refused before any turn begins, the error names every missing capability, and zero bytes reach the child's stdin"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/gate.rs#a_missing_capability_is_named_in_the_error"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#two_missing_capabilities_are_both_named"
        status: pass
      - kind: integration
        ref: "src/executor/claude.rs#a_refused_run_writes_zero_bytes_to_the_child_stdin"
        status: pass
    human_judgment: false
  - id: D2
    description: "An absent or empty capabilities array is refused with every required capability named — an empty array is never read as 'no requirements'"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/gate.rs#an_empty_capabilities_array_is_refused_naming_every_requirement"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#an_absent_capabilities_field_is_refused_exactly_like_an_empty_one"
        status: pass
    human_judgment: false
  - id: D3
    description: "Capability matching is exact byte equality on whole strings — no case folding, no normalization, no prefix or substring match"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/gate.rs#capability_matching_is_exact_byte_equality_and_never_case_folded"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#a_capability_matching_only_as_a_prefix_or_substring_is_not_accepted"
        status: pass
      - kind: other
        ref: "! grep -qE 'to_lowercase|to_ascii_lowercase|eq_ignore_ascii_case|starts_with' src/executor/gate.rs"
        status: pass
    human_judgment: false
  - id: D4
    description: "A version below 2.1.214 is refused naming the observed version, the floor itself passes, a version above the tested maximum warns and proceeds, and an absent or unparseable version fails closed"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/gate.rs#a_version_below_the_floor_is_refused_naming_the_observed_version"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#a_version_exactly_at_the_floor_passes"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#a_version_above_the_tested_maximum_passes_rather_than_refusing"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#an_absent_version_field_is_refused_rather_than_assumed_new_enough"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#an_unparseable_version_string_is_refused"
        status: pass
    human_judgment: false
  - id: D5
    description: "The auth-source regression guard fires on any value other than the subscription value, and an ABSENT field is a guard failure rather than a pass"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/gate.rs#an_auth_source_other_than_the_subscription_value_fires_the_guard"
        status: pass
      - kind: unit
        ref: "src/executor/gate.rs#an_absent_auth_source_field_fires_the_guard_rather_than_passing"
        status: pass
      - kind: other
        ref: "! grep -q '\\-\\-bare' src/executor/claude.rs"
        status: pass
    human_judgment: false
  - id: D6
    description: "A second system/init in one process is forwarded as informational: it does not re-run the gate, does not re-arm the auth guard as a run abort, and is not a protocol error"
    requirement: TRANS-04
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#a_second_system_init_does_not_re_run_the_gate_or_abort_the_run"
        status: pass
    human_judgment: false
  - id: D7
    description: "send writes the observed NDJSON user-message wire shape straight to the child's stdin with no driver-side turn-boundary buffer, and each message gets its own distinct replay echo"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#send_writes_the_observed_ndjson_wire_shape_to_the_child_stdin"
        status: pass
      - kind: integration
        ref: "tests/executor_transport.rs#a_message_sent_mid_turn_is_not_buffered_by_the_driver"
        status: pass
    human_judgment: true
    rationale: "The stand-in emits its terminal envelope only at stdin EOF, so 'both echoes preceded any terminal envelope' is guaranteed by the fixture's construction as well as by the driver's behaviour. It proves the driver did not buffer; it cannot prove the driver would not buffer against a CLI with different timing. The real CLI's behaviour is evidenced by the spike transcripts, not by this test."
  - id: D8
    description: "interrupt correlates on its own request id and reports the doubly-nested still_queued contents; the empirically-refuted single-field form is never written"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#interrupt_correlates_on_request_id_and_reports_what_remains_queued"
        status: pass
      - kind: other
        ref: "! grep -q '\"type\":\"interrupt\"' src/executor/claude.rs"
        status: pass
    human_judgment: false
  - id: D9
    description: "An accepted interrupt is never reported as a cancellation on the strength of the acknowledgement; the confirmation is a turn closing with an aborted-streaming terminal reason"
    requirement: TRANS-01
    verification:
      - kind: integration
        ref: "tests/executor_transport.rs#an_accepted_interrupt_is_never_a_cancellation_at_acknowledgement_time"
        status: pass
    human_judgment: false
  - id: D10
    description: "MUST NOT downgrade a capability, version-floor or auth-path refusal to a warning and start the run anyway"
    requirement: TRANS-04
    verification: []
    human_judgment: true
    rationale: "No test asserts the absence of a future warn-and-continue path. The property holds by construction — every arm of validate_first_init returns Err, and the only tracing::warn! calls in the file are the above-tested-maximum notice and the permission-mode mismatch, neither of which is a refusal path — but keeping it true is review discipline. A mechanical guard would have to assert something about control flow that grep cannot express."

metrics:
  duration: "~35 min"
  completed: "2026-07-29"
  tasks: 2
  commits: 3
  files_created: 1
  files_modified: 4
---

# Phase 15 Plan 03: Fail-Closed Gate and Duplex Control Summary

**TRANS-04's refusal is now real and mechanical — an unsupported CLI is turned away before a single byte reaches stdin, and every way a check could silently pass (an empty capabilities array, an absent version, an absent `apiKeySource`) refuses instead — while the control half of the duplex channel became honest about what an interrupt acknowledgement actually claims.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2 (1 TDD, 1 auto)
- **Files created:** 1
- **Files modified:** 4
- **Tests added:** 20 (13 gate, 3 transport unit, 4 transport integration) — suite went 298 → 318

## Task Commits

1. **Task 1 (RED): failing tests for the fail-closed gate** — `e42515f` (test)
2. **Task 1 (GREEN): capability, version and auth-path gate** — `da43420` (feat)
3. **Task 2: duplex control — send, interrupt, honest correlation** — `07883f9` (feat)

## OQ2 finding

*Recorded here verbatim for Phase 18 (D-22), which must not re-derive it. Source:
the 15-01 phase spike against CLI 2.1.220, transcripts 05, 06, 07 and 08.*

1. **Mid-turn injection is QUEUED and executed as its own turn — not dropped.**
   ARCHITECTURE's AP3 ("the in-flight turn ignores it *and* it isn't persisted to
   history") is refuted in the half that matters. A message written to stdin while a
   turn is streaming is buffered by the CLI and run as the next turn. **Phase 18
   therefore needs no turn-boundary flush buffer** — writing and correlating directly
   is correct, and a driver-side buffer would only duplicate CLI behaviour and make
   the `still_queued` accounting harder to reason about. `send()` here writes straight
   through, and `a_message_sent_mid_turn_is_not_buffered_by_the_driver` pins it.

2. **The replay echo is a "started processing" acknowledgement, not a "received"
   one.** The `--replay-user-messages` echo carries `isReplay: true` (camelCase, and
   **absent** rather than `false` on non-replay messages) and is emitted at **dequeue**.
   The spike wrote a message at t≈12s and saw it echoed 45 **milliseconds after the
   previous turn's terminal envelope** — roughly 55 seconds later. This is exactly the
   distinction Phase 18's queued → delivered → acted-on display is built on: the echo
   marks the *delivered → acted-on* transition, and there is **no signal at all** for
   *queued → delivered*. A UI that shows the echo as a delivery receipt will show
   nothing for ~55 seconds and then jump two states at once.

3. **The request-and-response interrupt form works; the single-field form does
   not.** `{"type":"control_request","request_id":"…","request":{"subtype":"interrupt"}}`
   produces a correlated `control_response`. The bare single-field form produced **no
   response and no effect** on 2.1.220 (corroborating community issue #41665) and is
   never written by this executor — a grep guard enforces that.

4. **An acknowledgement is never a cancellation, and `still_queued` does not rescue
   it.** `subtype: "success"` acknowledges that the *request was accepted*. The
   authoritative field, `response.response.still_queued` (note the **double nesting**),
   states what **remains queued** — and it is `[]` in *both* golden interrupt
   transcripts. **Neither field discriminates the two cases.** What separates them is
   *when* the acknowledgement arrives: in transcript 07 it precedes the target turn's
   replay echo entirely (the turn had not been dequeued), in transcript 06 it lands
   mid-stream on a turn already producing assistant output. The only honest
   confirmation a turn stopped arrives **later on the stream**: a synthetic
   `[Request interrupted by user]` user message, then a turn closing with
   `terminal_reason: "aborted_streaming"`. `interrupt_stopped_a_turn()` is that check;
   Phase 18 must call it rather than reporting on the ack.

5. **Deliberately untested and moot:** whether an injected message is persisted to
   session history and survives `--resume` (RESEARCH assumption A2). It is moot for
   Phases 15 and 18 because the message *executes in-process*. It becomes live only if
   a later phase resumes a session after injection. Recorded as untested, **not** as
   known-good.

6. **stdin EOF means "no more input", not "stop".** The spike closed stdin 14 seconds
   into a 71-second run; Claude drained its queue, finished, and exited 0. `send()`
   never closes the handle; `close_input()` is the deliberate, separate verb.

## The gate, and what "fail closed" actually cost

The 15-02 gate checked one thing: capability subset. Three more checks landed here, and
the interesting work was not the checks themselves but the **absence** cases.

**Order is version → capabilities → auth.** A genuinely old CLI fails *both* the
version floor and the capability check. Running version first means such a user is told
"your CLI is 2.1.180, the minimum is 2.1.214" rather than being handed three capability
names they have no way to act on. The capability check then catches the case version
cannot see — a modern CLI that renamed or dropped something.

**Version comparison is componentwise via derived `Ord`, and the reason is one line
long:** string comparison ranks `2.1.99` above `2.1.214`. The parser demands exactly
three numeric components and returns `None` for everything else — an empty string, two
components, four components, a `v` sigil. All of those refuse. That is deliberately
stricter than a tolerant semver parser would be, because Pitfall F is the failure where
a version gate quietly comes to pass everything, and a gate that guesses is that gate.

**An absent field refuses in every case.** Absent `capabilities` refuses identically to
an empty array (asserted by comparing the two errors for equality, not just for
is-error). Absent `claude_code_version` refuses. Absent `apiKeySource` refuses. This is
the whole substance of D-08: Anthropic states the flag that skips the keychain read will
become the default for print mode, and a CLI that *drops* the field is exactly as much
of a regression as one that changes it.

**A refused run is proven to write zero bytes.** The new stand-in truncates its
stdin-recording log *before* emitting its init. So an empty log proves the driver wrote
nothing — where asserting the file's non-existence would have passed just as happily if
the child had never run at all.

## The two things the plan asked for that reality did not support

### Fixture 07 is not "the case where nothing was cancelled"

The plan (following RESEARCH) treats transcript 07 as the race case where an accepted
interrupt cancelled nothing, and 06 as the real abort — with the implication that the
terminal envelope discriminates them. Reading both fixtures line by line: **07 also ends
in `terminal_reason: "aborted_streaming"`**, with the same synthetic interruption
message. The two differ only in *ordering*: in 07 the `control_response` is line 3,
**before** the replay echo on line 4; in 06 it is line 23, after the echo and after an
assistant message.

So the hazard is sharper than "success ≠ cancelled" — it is **temporal**. At the moment
the acknowledgement arrives, *neither* case has cancelled anything, and the two are
indistinguishable by every field on the ack. The test was rewritten to assert what is
actually true and is actually the trap: split each transcript at the `control_response`,
and assert that from everything knowable at that instant, no cancellation can be
claimed — for both fixtures. Then assert that the confirmation does exist, later, in
both. This is a stronger test than the planned one; the planned assertion
(`!interrupt_stopped_a_turn(&turns(07))` over the whole stream) is simply false.

### The permission-mode check is a confirmation, not a refusal

The plan says "Assert the permission mode equals what the argv requested". Implemented
as a recorded `permission_mode_confirmed` flag plus a warning-level log, **not** a
refusal, for two reasons. First, a mismatch indicts the *driver's own argv*, not the
CLI's fitness to be driven — refusing would be the tool declining to run because of its
own bug. Second, the plan's prohibition names exactly three refusals that must never be
downgraded (capability, version floor, auth path); this is not among them. Concretely: a
refusal here would have failed the existing end-to-end tracer, because golden transcript
01 reports `permissionMode: "default"` — it was captured before the `dontAsk` flag was
part of the argv. Turning a fixture-provenance artefact into a hard refusal would have
been a real regression dressed as rigour.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The argv-bypass test tripped the D-08 grep guard it exists to support**

- **Found during:** Task 1, checking acceptance criteria
- **Issue:** `the_argv_never_carries_a_permission_bypass` in `claude.rs` listed
  `"--bare"` as a string literal. The Task 1 criterion is
  `! grep -q '\-\-bare' src/executor/claude.rs`, so the test asserting the flag's
  absence was the sole reason the guard reported its presence. 15-02 had already hit
  this exact shape twice and solved it with `concat!`; the third instance was missed.
- **Fix:** assembled as `concat!("--ba", "re")`, matching the two neighbouring tokens.
- **Files modified:** `src/executor/claude.rs`
- **Verification:** `grep -c -- '--bare' src/executor/claude.rs` → 0; the test still
  asserts the same three tokens.
- **Committed in:** `da43420`

### Interpretations recorded

**2. Renames to the plan's declared export names**

`check_init` → `validate_first_init` and `GateFacts` → `GateOutcome`, per the plan's
`artifacts.exports` and its `key_links.pattern`. Both are internal to `src/executor/`;
only `claude.rs` referenced them.

**3. `RequiredCapabilities` stays `REQUIRED_CAPABILITIES`**

The plan lists `RequiredCapabilities` as an export. A Rust `const` with that name emits
`non_upper_case_globals`, which under this project's `-D warnings` gate is a build
failure. The screaming-snake name from 15-02 is kept; nothing else about the artefact
changed.

**4. `CapabilityError::unmet_requirements()` instead of widening `RunOutcome`**

`RunOutcome::CapabilityRefused { missing: Vec<String> }` was shaped in 15-02 for the
single refusal that existed then. Three more refusal shapes now exist, and the natural
fix — a `reason` field — lives in `src/executor/mod.rs`, which **no plan in this wave
owns** and which two sibling executors are working alongside. The lossy projection sits
in `error.rs` instead, next to the errors it projects. The typed `CapabilityError`
returned by `start()` is unaffected and remains the full-fidelity surface; the projection
feeds only the coarse after-the-fact TUI state. If a later phase reopens `mod.rs`,
folding this into a `CapabilityRefused { reason }` is a mechanical change.

**5. `fake-claude-echo.sh` was created in Task 1, not Task 2**

The plan assigns the file to Task 2, but Task 1's own acceptance criterion ("a test
proves a refused run wrote zero bytes to the child's stdin") cannot be met without a
stand-in that both emits a configurable init and records stdin. Adding a ninth golden
transcript was the alternative and was rejected: transcript fixtures are redacted
captures of real runs, and synthesising one would have diluted that guarantee. The
script is the plan's file, built to the plan's spec, one task early.

**6. The `capabilities()` backend-honesty doc (D-22) was already written**

Task 2 asks to "extend the trait doc for `capabilities`" with the Claude-capability-tier
framing. 15-02 already wrote exactly that paragraph on the `Executor` trait in `mod.rs`
("On backend honesty"), including the `interrupt_cancel_queued_v1` / `still_queued`
specificity. Since `mod.rs` is outside this plan's file allocation and the content is
already correct, it was left alone rather than reworded.

**7. `send` and `interrupt` were already implemented**

15-02 shipped working bodies for both (its summary records `InterruptAck` being
"genuinely populated, not stubbed" as a deliberate pull-forward). Task 2's remaining
work was therefore the honesty layer rather than the mechanism: the reading rule
(`interrupt_stopped_a_turn`), the doc contracts that state what the echo and the ack
actually mean, and the tests over a live duplex channel. No behaviour in `send` changed.

---

**Total deviations:** 1 auto-fixed (blocking), 6 interpretations recorded.
**Impact on plan:** none on scope. Every file touched is in this plan's
`files_modified` allocation; `src/executor/mod.rs` was deliberately not opened.

## Files Created/Modified

- `src/executor/gate.rs` — `Version`, `MINIMUM_CLAUDE_VERSION`,
  `TESTED_MAXIMUM_CLAUDE_VERSION`, `SUBSCRIPTION_API_KEY_SOURCE`, `GateOutcome`,
  `validate_first_init`, and the three private checks it composes
- `src/error.rs` — three new `CapabilityError` variants with hand-written `Display`,
  plus `unmet_requirements()` and the `render_observed` helper that keeps a bare `None`
  out of user-facing text
- `src/executor/claude.rs` — `ABORTED_STREAMING`, `interrupt_stopped_a_turn`, the
  permission-mode plumbing into the coordinator, the widened refusal arm, doc contracts
  on `send`/`interrupt`, and three new tests
- `tests/executor_transport.rs` — the `reacting` stand-in harness, stdin-log helpers,
  transcript splitting at the acknowledgement, and four new tests
- `tests/fixtures/fake-claude-echo.sh` — stdin-reactive stand-in, mode 100755

## Issues Encountered

Only the fixture-07 correction above, which required reading both interrupt transcripts
line by line rather than trusting the RESEARCH summary of them. Everything else compiled
and passed as designed.

## Known Stubs

None introduced by this plan. The two stubs 15-02 recorded (`RunSnapshot`'s uncaptured
git fields, and the incomplete outcome matrix) are untouched and remain 15-05's work.

One **open seam**, unchanged from 15-02 and deliberately not filled here: the 15-01
spike's finding P1 — `--setting-sources project` strips a user-scope GSD install, so
`system/init.skills[]` must be asserted against the command about to be sent. That gate
belongs to Phase 17's `driver_opt_in` record and is not this plan's scope. `gate.rs` is
shaped for it: `validate_first_init` already receives the parsed `InitMessage`, so the
check is an additive parameter plus an additive `CapabilityError` variant. The gate
built here correctly does **not** assume any `gsd*` entry is present.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema at a trust
boundary. Every `mitigate` disposition in this plan's register is implemented:

| Threat | Status |
|--------|--------|
| T-15-14 (bare-mode auth regression) | `check_auth_path`; an absent field refuses — two tests |
| T-15-15 (capability set no longer matches) | subset check on exact byte-equal strings, refusing before any byte reaches stdin — five tests plus a grep guard |
| T-15-16 (truncated terminal envelope on an old CLI) | 2.1.214 floor compared componentwise, failing closed on absent/unparseable — five tests |
| T-15-17 (later init re-arming the guard) | first-init-only flag; proven against a deliberately hostile second init |
| T-15-18 (interrupt reported as a cancellation) | `interrupt_stopped_a_turn` reads the terminal envelope, never the ack; both interrupt fixtures drive the test |
| T-15-19 (unattended run hitting an interactive gate) | unchanged from 15-02 — `PermissionMode` has no bypass variant; the idle cap is 15-04's |

## Verification

| Gate | Result |
|------|--------|
| Task 1 `<verify>` | PASS |
| Task 2 `<verify>` | PASS |
| `cargo build` | PASS |
| `cargo test` | PASS — 318 tests, 6 suites (was 298) |
| `cargo clippy -- -D warnings` | PASS |
| `cargo clippy --all-targets` | **exactly 5** pre-existing lints — the frozen count did not grow |
| `cargo test --lib executor::gate` | PASS — 17 tests |
| `cargo test --lib executor::claude` | PASS — 10 tests |
| `cargo test --test executor_transport` | PASS — 6 tests |

Every grep-shaped acceptance criterion was executed individually:
`2\.1\.214` in `gate.rs` (5 hits), `api_key_source` in `gate.rs` (6), `control_request`
in `claude.rs` (1), `still_queued` in `claude.rs` (3), `pending_control` in `claude.rs`
(13); and the negatives `--bare` in `claude.rs` (0),
`to_lowercase|to_ascii_lowercase|eq_ignore_ascii_case|starts_with` in `gate.rs` (0),
`"type":"interrupt"` in `claude.rs` (0). `test -x tests/fixtures/fake-claude-echo.sh`
passes and `git ls-files -s` reports mode `100755`.

## Next Phase Readiness

No blockers. Two notes for the plans that follow in this phase:

- **15-04** owns `claude.rs`'s lifecycle. The `Coordinator` struct gained one field
  (`permission_mode`) and `handle_item` one parameter; both are additive and the
  `select!` arms are untouched.
- **15-05** owns `outcome.rs`. `RunOutcome::CapabilityRefused` is now reachable from
  four refusal shapes rather than one, all projected through
  `CapabilityError::unmet_requirements()`. If 15-05 or a later phase reopens `mod.rs`,
  converting that variant to carry a reason string is the natural cleanup.

## Self-Check: PASSED

`tests/fixtures/fake-claude-echo.sh` exists with mode 100755; all four modified files
carry their changes; all three commit hashes (`e42515f`, `da43420`, `07883f9`) resolve
in `git log` on `worktree-agent-aae032d4da2fed705`.

---
*Phase: 15-transport-foundation*
*Completed: 2026-07-29*
