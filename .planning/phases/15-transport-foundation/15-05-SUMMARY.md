---
phase: 15-transport-foundation
plan: 05
subsystem: executor-outcome
tags: [outcome-derivation, trans-02, git, disk-delta, stream-json, tdd, matrix]
status: complete

requires:
  - phase: 15-01
    provides: "OQ1 PASS verdict, eight redacted golden stream-json transcripts, the settled A3 sub-probe finding"
  - phase: 15-02
    provides: "RunSnapshot with its two uncaptured git fields, derive_run_outcome's fixed four-argument shape, RunOutcome/TurnOutcome, the tolerant ResultMessage model"
provides:
  - "head_sha + is_dirty in src/state_reader/git_ops.rs — the run delta's git half, beside the existing helpers"
  - "DiskDelta — three independent corroboration signals (artifacts / HEAD / dirty) plus a made_changes roll-up"
  - "RunSnapshot::capture completed — artifact fingerprint AND git half, synchronous end to end"
  - "The full D-26 derivation matrix: every distinguishable combination of subtype x is_error x terminal_reason x exit code x disk-changed maps to a named outcome under its own test"
  - "derive_run_outcome_from_envelopes — the same matrix with permission_denials[] connected"
  - "run_turn_count (summed) and run_cost_usd (last envelope) — the D-29 per-turn vs cumulative split, as functions"
affects:
  - "Phase 16 (journals the outcome; the notional-cost caveat is on run_cost_usd's doc)"
  - "Phase 18 (renders RunOutcome — see the matrix table below for what each variant means)"
  - "Phase 20 (routes on RunOutcome)"
  - "Whoever next owns src/executor/claude.rs (one-line call-site switch, see Known Stubs)"

tech-stack:
  added: []
  patterns:
    - "Three independent delta signals kept separate at capture time, rolled up only at the decision point — which signal fired is the diagnostic"
    - "Unknown git half is INCONCLUSIVE, not false: a half-captured comparison contributes nothing rather than asserting no-change"
    - "A richer entry point beside a fixed one, instead of widening a signature that concurrent plans build against"
    - "Fixture-driven matrix tests via compile-time include_str! — zero filesystem I/O, zero quota, real captures"
    - "Mutate one field of a real captured envelope to build a test case, rather than inventing a whole envelope"

key-files:
  created: []
  modified:
    - "src/executor/outcome.rs"
    - "src/state_reader/git_ops.rs"

key-decisions:
  - "permission_denials[] reaches the derivation through a second entry point over ResultMessage, NOT by widening TurnOutcome in src/executor/mod.rs — that file is outside this plan's declared scope and two sibling executors were live"
  - "aborted_tools classifies as TimedOut on the terminal reason alone; the exit code corroborates but never decides"
  - "TimedOut carries Duration::ZERO from the derivation, meaning 'externally bounded, duration unknown to the four sources'"
  - "Denials outrank the envelope's own verdict: a success envelope alongside denials describes a blocked run"
  - "Untracked files count as dirty — an uncommitted new file is unambiguously a change the agent made"
  - "An unobserved exit status is unknown, not a disagreement: a failed wait() must not manufacture a contradiction"

requirements-completed: [TRANS-02]

coverage:
  - id: D10
    description: "A run's outcome is derived from the last result envelope, the process exit code, the .planning artifact fingerprint and git HEAD plus dirty status — and never from the agent's prose summary"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/executor/outcome.rs#the_agents_prose_summary_changes_nothing_about_the_outcome"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#a_failure_envelope_with_exit_zero_is_still_a_failure"
        status: pass
    human_judgment: false
  - id: D11
    description: "A result envelope reporting success while nothing changed on disk and nothing changed in git is reported as a distinct no-op outcome, not as a success"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/executor/outcome.rs#success_without_changes_is_noop"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#a_moved_head_alone_is_enough_for_a_success_with_changes"
        status: pass
    human_judgment: false
  - id: D29
    description: "A transcript carrying two result envelopes yields exactly one run-level outcome, derived from the LAST, with the turn count summed and the cumulative cost taken from the last envelope"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/executor/outcome.rs#multi_turn_yields_exactly_one_run_outcome_derived_from_the_last_envelope"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#the_run_turn_count_is_summed_while_the_run_cost_comes_from_the_last_envelope"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#a_later_failing_turn_overrides_an_earlier_succeeding_one"
        status: pass
    human_judgment: false
  - id: D26
    description: "Every distinguishable combination of subtype, error flag, terminal reason, exit code and disk-changed has its own named test, including the budget-exhausted and streaming-aborted states the original decision list did not contain"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "cargo test --lib executor::outcome (30 tests)"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#fixture_02_budget_exhausted_is_a_failure_classified_as_the_budget_ceiling"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#fixture_06_aborted_streaming_with_exit_1_is_a_kill_classified_as_interrupted"
        status: pass
    human_judgment: false
  - id: D11b
    description: "The artifact fingerprint reuses the existing project-state reader and the two new git helpers live beside the existing ones in git_ops.rs — no parallel reader module exists anywhere in the tree"
    requirement: TRANS-02
    verification:
      - kind: other
        ref: "! test -f src/executor/git.rs && ! test -f src/executor/disk.rs; grep -q parse_project_state src/executor/outcome.rs"
        status: pass
    human_judgment: false
  - id: D10b
    description: "The exit code is read as a liveness and crash signal only, never as the authoritative verdict"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/executor/outcome.rs#fixture_04_aborted_tools_with_exit_124_is_a_timeout_not_a_claude_verdict"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#a_success_envelope_with_a_non_zero_exit_code_surfaces_the_disagreement"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#an_unobserved_exit_status_is_not_read_as_a_disagreement"
        status: pass
    human_judgment: false
  - id: D24
    description: "Shelling out to git in a non-repository project never propagates an error (T-15-24)"
    requirement: TRANS-02
    verification:
      - kind: unit
        ref: "src/state_reader/git_ops.rs#both_new_helpers_return_none_for_a_non_git_dir_without_erroring"
        status: pass
    human_judgment: false

metrics:
  duration: "~35 min"
  completed: "2026-07-29"
  tasks: 2
  commits: 3
  files_created: 0
  files_modified: 2
---

# Phase 15 Plan 05: The Outcome Derivation Matrix Summary

**TRANS-02 is now a testable correctness claim rather than an assertion: a run's verdict comes
from the last terminal envelope, the exit status, a three-signal disk/git delta and nothing
else — and 30 named tests prove it stays right when those sources disagree, including the case
where the agent's own prose says the opposite.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2 (1 auto, 1 TDD)
- **Files modified:** 2 (no new files, no new modules, no Cargo change)
- **Tests added:** 24 (3 in `git_ops`, 21 in `outcome`) — `executor::outcome` went 6 → 30

## Task Commits

1. **Task 1: the disk and git delta** — `7fec8ae` (feat)
2. **Task 2: the matrix (RED)** — `35ce2b4` (test) — 16 failing
3. **Task 2: the matrix (GREEN)** — `91f10eb` (feat) — 30 passing

No REFACTOR commit: the GREEN implementation needed no cleanup pass.

## The outcome matrix

**This table is the contract Phase 18 renders and Phase 20 routes on.** Arms are tried top to
bottom; the first match wins.

| # | `subtype` | `is_error` | `terminal_reason` | exit | delta | → `RunOutcome` | Pinned by |
|---|-----------|-----------|-------------------|------|-------|----------------|-----------|
| 1 | — (no envelope at all) | — | — | any, incl. 0 | any | `Failed` naming the missing terminal envelope | synthetic |
| 2 | any | any | any | any | any | `PermissionDenied { denials }` **when `permission_denials[]` is non-empty on ANY envelope** | fixture 01, denials injected |
| 3 | `success` | `false` | `completed` \| absent | 0 | any signal fired | `SucceededWithChanges` | fixture 01 |
| 4 | `success` | `false` | `completed` \| absent | 0 | **no signal** | `SucceededNoChanges` — the no-op | fixture 01 |
| 5 | `success` | `false` | `completed` \| absent | 0 | HEAD moved only | `SucceededWithChanges` | fixture 01 |
| 6 | `success` | `false` | `completed` \| absent | **non-zero / signal** | any | `Failed`, reason names the **disagreement** | fixture 01 |
| 7 | `success` | `false` | `completed` \| absent | **not observed** | any | derived normally — unknown ≠ disagreement | fixture 01 |
| 8 | any | any | `aborted_tools` | 124 (external) | any | `TimedOut { after: ZERO }` | fixture 04 |
| 9 | any | any | `aborted_streaming` | 1 | any | `Killed { turns }` | fixture 06 |
| 10 | `error_max_budget_usd` | `true` | `budget_exhausted` | 1 | any | `Failed` — "budget ceiling" | fixture 02 |
| 11 | `error_max_turns` | any | any | any | any | `Failed` — "turn ceiling" | fixture 01, subtype swapped |
| 12 | **anything unrecognised** | any | any | any | any | `Failed` carrying both observed strings **verbatim** | synthetic |

Multi-turn, all from fixture 05 (two envelopes, one process):

| Property | Value | Why |
|----------|-------|-----|
| run outcomes produced | **1** | `result` closes a turn; the run ends at stdin EOF → exit (D-29) |
| verdict source | the **last** envelope | an executor reading the first truncates every steered run (Pitfall A) |
| run turn count | **sum** (fixture 05: 1+1=2; fixture 08: 1+2=3) | `num_turns` resets per envelope |
| run cost | the **last** envelope's `total_cost_usd` | that field accumulates |

## Decisions made

### `permission_denials[]` reaches the derivation through a second entry point

`TurnOutcome` — the projection `derive_run_outcome` consumes — **drops
`permission_denials[]`**. `TurnOutcome::from_result` copies six fields and the denials array is
not one of them. So the run-level derivation, as shaped by 15-02, structurally could not report
a permission refusal, while D-10 explicitly names that array as part of the envelope source.

Two ways to fix it. Add the field to `TurnOutcome` in `src/executor/mod.rs` — outside this
plan's declared `files_modified`, with two sibling executors live in their own worktrees and
`claude.rs` (theirs) potentially constructing the struct. Or read the envelope directly.

**Taken:** `derive_run_outcome_from_envelopes(&[ResultMessage], …)` joins the existing
`derive_run_outcome(&[TurnOutcome], …)`. Both delegate to one private `derive`; the envelope
entry point supplies the denials, the projection entry point supplies an empty vec. This is
strictly more faithful to D-10, which describes the derivation as reading *the envelope* —
`ResultMessage` carries the denials, the `errors[]` array and the absent-on-error prose field,
where the projection carries a lossy six-field subset. The fixed four-argument signature
15-02 pinned is untouched, so nothing downstream rebuilds.

Denials are collected across **all** envelopes rather than just the last: a refusal three turns
back still explains why the run produced nothing. They **outrank** the envelope's own verdict,
because a `success` envelope alongside denials describes a run that was blocked from doing what
it was asked — which is exactly the untrusted-workspace failure the 15-01 spike found (P2),
where an ignored project allow-list under `dontAsk` denies every write and reads as a
capability problem.

### `aborted_tools` → `TimedOut { after: Duration::ZERO }`

The plan maps fixture 04 (`error_during_execution` / `aborted_tools` / exit 124) to "timed
out", and `RunOutcome::TimedOut` is the variant for it. Its `after: Duration` field is a
problem: **the cap that was breached is not knowable from the four derivation sources.** Exit
124 is the GNU `timeout` convention and the bound lives in the external supervisor, not in
anything on the wire.

`Duration::ZERO` is used as an explicit "externally bounded, duration unknown to the
derivation" sentinel, documented in place. The alternative — widening `TimedOut` — means
editing `src/executor/mod.rs`, ruled out for the same reason as above. **Flagged for Phase 18:**
do not render `TimedOut { after: 0s }` as "timed out after 0 seconds". The supervisor
(`ExecutionOptions::wall_clock_cap`, 15-04's) constructs this variant with a real cap when *it*
enforces the deadline; the derivation's copy is a classification, not a measurement.

The classification keys on the **terminal reason**, not the exit code: D-10 is explicit that
exit 124 came from `timeout` and never from Claude, so making it the discriminator would be the
exact inversion the decision forbids. The same holds for `aborted_streaming` → `Killed`.

### Exit-code disagreement is surfaced in one direction only, deliberately

A `success` envelope with a non-zero exit becomes a `Failed` whose reason names the
disagreement and whose `subtype` is still `"success"` — both sources are preserved for the
reader. The converse (a failure envelope with exit 0) stays a failure: the exit code is a
liveness signal and cannot promote anything. And an **unobserved** status (`wait()` failed) is
*unknown*, not a contradiction — `describe_exit_disagreement` returns `None` for it, so a run
whose exit could not be read is still derived normally rather than manufactured into a
disagreement. Each of the three is its own test.

### Untracked files count as dirty

`is_dirty` uses `git status --porcelain`, which reports untracked files by default. An agent
that writes a new file without committing it has unambiguously changed the project, which is
precisely the corroboration signal the delta wants. Stated on the function's doc so nobody
"fixes" it to `--untracked-files=no` later.

### An unknown git half is inconclusive, not false

When a project is not a git repository, both git fields are `None` on both snapshots and the
two git signals stay `false` — they contribute nothing rather than asserting "nothing moved".
`DiskDelta::between` matches on `(Some, Some)` and falls through otherwise, so a half-captured
pair never fabricates a delta either. A project with no git therefore still reports its
artifact changes honestly. Two tests pin both halves of this.

## Files Modified

- **`src/state_reader/git_ops.rs`** — `head_sha` and `is_dirty`, placed between
  `project_last_activity` and the `GitLogEntry` block, following the file's *synchronous*
  shape (`std::process::Command`, `-C <root>`, lossy UTF-8, trimmed, `Option`-returning,
  never propagating an error for a non-repository). The existing file is inconsistent about
  `-C` versus `.current_dir()`; the two new helpers are consistent with each other on `-C`,
  as the plan asked. Plus 3 tests.
- **`src/executor/outcome.rs`** — `RunSnapshot::capture` completed; `DiskDelta` added;
  `changed_since` reduced to a roll-up over it; `derive` (the matrix) plus
  `derive_run_outcome_from_envelopes`, `run_turn_count`, `run_cost_usd`,
  `describe_exit_disagreement`; `describe_failure` trimmed of its two now-unreachable arms.
  Plus 21 tests.

## Deviations from Plan

### Interpretations recorded

**1. A second derivation entry point instead of a widened `TurnOutcome`**

Covered in full above. The plan's acceptance criterion
`grep -q 'permission_denials' src/executor/outcome.rs` is satisfied, the behaviour it asks for
is implemented and tested, and no file outside this plan's `files_modified` was touched. The
cost is one extra public function and a call-site switch left for whoever next owns
`claude.rs` (see Known Stubs).

**2. `TimedOut` carries a sentinel duration**

Covered in full above. Recorded because it is a rendering hazard for Phase 18, not because it
changes any classification.

**3. One 15-02 test's expected variant was updated**

`the_last_envelope_decides_a_multi_turn_run` (written in 15-02) asserted `Failed` for a last
envelope carrying `aborted_streaming`. D-32 and this plan's `<behavior>` list reclassify that
state as `Killed`. The test's *intent* — the verdict comes from the last envelope, not the
first — is unchanged and still proven; only the expected variant moved, with a comment saying
why. This is the matrix refining a placeholder classification, not a regression.

**4. `DiskDelta` exposes `between` as an associated function rather than a method on
`RunSnapshot`**

`DiskDelta::between(before, after)` reads in the argument order the domain uses. `changed_since`
survives as a thin roll-up because 15-02's call sites and one of its tests use it.

---

**Total deviations:** 0 auto-fixed bugs, 4 interpretations recorded.
**Impact on plan:** None on scope. Every `<behavior>` item and every acceptance criterion is
implemented and tested. Two interpretations (1 and 2) exist solely because the plan's declared
`files_modified` excludes `src/executor/mod.rs`, where the ideal shape of both would live.

## Issues Encountered

None requiring problem-solving beyond the two shape constraints above. The RED step failed
cleanly (16 tests), the GREEN step passed everything but the one 15-02 expectation, and the
project gate was green on the first attempt after that.

## Known Stubs

One, and it is a call-site wiring gap rather than missing behaviour:

1. **`src/executor/claude.rs`'s coordinator still calls `derive_run_outcome`**, the
   `TurnOutcome` entry point, so a production run cannot currently report
   `RunOutcome::PermissionDenied` even though the derivation implements and tests it. Fixing it
   is a one-line switch to `derive_run_outcome_from_envelopes` wherever the coordinator already
   holds the `ResultMessage` values (it collects them today to build the `TurnOutcome` vec).
   That file is owned by a concurrently-running plan in this wave, so it was deliberately not
   touched. **Recommended owner: whichever of 15-03 / 15-04 last edits `claude.rs`, or
   Phase 16.**

This does not prevent this plan's goal: TRANS-02's claim is about the derivation's correctness,
and the derivation is correct and proven. It does mean the denial path is unreachable in
production until the call site moves.

## Flagged upward

- **The TRANS-02 `unclassified` edge-shape assumption the plan surfaced is left open.** The
  planner's reading — that TRANS-02's edge shape is the multi-source *disagreement* axis rather
  than a data-shape axis — is what the matrix is written to, and this plan did not find
  evidence against it. Rows 6, 7 and the failure-with-exit-0 case are the disagreement axis
  made concrete. If a reviewer prefers the data-shape reading, the matrix is the thing to
  extend, and adding a row is additive.
- **Carried assumption A3 is SETTLED, not merely unblocked.** `15-SPIKE-OQ1.md` Step 4 records
  two `system/init` and two `result` envelopes for a **tool-using** queued turn, with a real
  `Read` `tool_use` block, and `num_turns` of 1 then 2. D-29 holds with no tool-use carve-out,
  so the multi-turn half of this matrix needed no revisiting. Fixture 08's 1+2=3 sum is
  asserted directly.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema at a trust boundary.
Every `mitigate` disposition in this plan's register is implemented:

| Threat | Status |
|--------|--------|
| T-15-20 — prose trusted as the verdict | Mitigated. No derivation branch reads the prose field; `the_agents_prose_summary_changes_nothing_about_the_outcome` proves two envelopes differing only in prose derive identically, and `success_without_changes_is_noop` proves a success envelope with no disk change is a no-op. |
| T-15-21 — a steered run truncated at the first envelope | Mitigated. The verdict comes from `turns.last()`; fixture 05 yields exactly one outcome from two envelopes, and a synthetic success-then-failure pair proves the later turn overrides. |
| T-15-22 — a panicking accessor on an absent field | Mitigated. Zero `unwrap()` in the non-test region (asserted mechanically); `fixture_02_absent_result_field_never_aborts_the_derivation` drives the real budget envelope, whose prose field is absent entirely. |
| T-15-23 — the notional cost presented as a real charge | Mitigated. `run_cost_usd`'s doc states it prices work that is not billed per call under subscription auth and must never be presented as what the run cost. |
| T-15-24 — shelling out to git in a non-repository | Mitigated. Both helpers return `Option` and never propagate; `both_new_helpers_return_none_for_a_non_git_dir_without_erroring` covers it. |

## Verification

| Gate | Result |
|------|--------|
| Task 1 `<verify>` | PASS |
| Task 2 `<verify>` | PASS |
| `cargo build` | PASS |
| `cargo test` | PASS — 325 tests across 6 suites (was 298 / 6 before this plan) |
| `cargo clippy -- -D warnings` | PASS |
| `cargo clippy --all-targets --message-format=short` | **exactly 5** warning lines — the frozen count did not grow |
| `cargo test --lib executor::outcome` | PASS — 30 tests (was 6) |
| `cargo test --lib state_reader::git_ops` | PASS — 8 tests (was 5) |

Every grep-shaped acceptance criterion was executed individually and passes:
`pub fn head_sha`, `pub fn is_dirty`, `parse_project_state`, `permission_denials`, `DiskDelta`
with its three signals and `made_changes`; and the negatives `! test -f src/executor/git.rs`,
`! test -f src/executor/disk.rs`, `! grep -q 'enum TerminalReason'`, and zero `unwrap()` in the
non-test region of `outcome.rs`.

## Next Phase Readiness

No blockers. The types Phases 16, 18 and 20 build on are final:

- **Phase 16** journals `RunOutcome` and reads `run_cost_usd` with its notional-cost caveat
  already documented at the source.
- **Phase 18** renders the matrix table above. Two rendering notes: `TimedOut { after: ZERO }`
  from the derivation means "externally bounded, duration unknown" and must not be printed as a
  measured duration; and `SucceededNoChanges` is a *distinct* state from `SucceededWithChanges`
  that deserves its own affordance — it is the whole point of ROADMAP success criterion 2.
- **Phase 20** routes on the variant set unchanged.
- **Whoever next edits `claude.rs`** switches one call to
  `derive_run_outcome_from_envelopes` to make the denial path reachable in production.

## Self-Check: PASSED

Both modified files exist on disk and carry their changes; all three commit hashes
(`7fec8ae`, `35ce2b4`, `91f10eb`) resolve in `git log`; no file was deleted across the plan's
commits; no file outside this plan's declared `files_modified` was touched.

---
*Phase: 15-transport-foundation*
*Completed: 2026-07-29*
