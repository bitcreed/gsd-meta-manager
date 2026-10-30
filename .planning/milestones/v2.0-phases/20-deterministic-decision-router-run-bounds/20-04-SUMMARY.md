---
phase: 20-deterministic-decision-router-run-bounds
plan: "04"
subsystem: driver
status: complete
tags: [driver, router, rule-table, gates, goal-met, conformance, drive-02, drive-05, drive-06]
requires:
  - src/driver/router::decide
  - src/state_reader/disk_status::VerificationStatus
  - src/state_reader/disk_status::UatStatus
  - src/state_reader/roadmap_md::RoadmapPhase::depends_on
  - src/state_reader/state_md::is_error_status
  - "ProjectState: continue_here_present, deferred_verification_phases"
provides:
  - src/driver/router::RULE_TABLE
  - src/driver/router::SAFE_COMMAND_ALPHABET
  - src/driver/router::RouterAction
  - src/driver/router::is_goal_met
  - src/driver/router::status_token
  - "RouterReason: twelve gate arms under the `gate_` prefix"
  - "Decision::GoalMet has a producer"
  - "RouterReason::DependencyUnsatisfied has a producer"
affects:
  - src/driver/router.rs
  - src/envelope/mod.rs
  - src/journal/mod.rs
  - src/driver/dry_run.rs
  - tests/driver_router_table.rs
  - tests/driver_router_conformance.rs
tech-stack:
  added: []
  patterns:
    - "declared const rule table driven by both the implementation and its guard"
    - "command alphabet made constructive: row -> action -> verb, no other path to a string"
    - "source-scanning guard with a non-vacuity floor and a matcher control arm"
    - "declared-divergence list that fails when a divergence stops diverging"
    - "include_str! of a module's own source to pin a doc paragraph"
key-files:
  created:
    - tests/driver_router_table.rs
    - tests/driver_router_conformance.rs
  modified:
    - src/driver/router.rs
    - src/envelope/mod.rs
    - src/journal/mod.rs
    - src/driver/dry_run.rs
decisions:
  - "/gsd-verify-work is NOT in the safe alphabet; it is the command a human runs to answer a gate"
  - "gates are evaluated BEFORE goal-met, so a passing status can never route past an open gate"
  - "verification gates are consulted only for an `executed` target; `missing` earlier means 'not yet'"
  - "G6 stale-check-indeterminate is recovered from has_verification && status Missing"
  - "an absent map entry parks under `no_disk_inference`, not `no_directory`"
  - "the collision filter models the clock-free `partial` half only; `is_active` needs a clock the pure fn does not have"
  - "the router reason enum carries two prefixes, `router_` and `gate_`"
metrics:
  duration: ~2h
  completed: 2026-08-19
actuals:
  tokens: 35000
  tasks: 3
  commits: 3
---

# Phase 20 Plan 04: The Complete Rule Table, the Gate Taxonomy and the Conformance Oracle Summary

The router now covers every forward-motion state with a table transcribed from
GSD's own, parks at twelve separately-greppable human-judgement gates, declares
goal-met from a fact rather than a claim, and proves the whole thing against the
runtime the driver is actually driving.

## What Was Built

**The rule table as data.** `RULE_TABLE` is a `const` slice of five `RuleRow`
values — `no_directory` and `empty` → discuss, `discussed` and `researched` →
plan, `planned` → execute — each carrying the observed status, the action, the
dependency condition and its rationale. `decide` drives from the slice rather
than from match arms, which is what makes "a rule with no row, or a row with no
rule, fails the build" a property of one list instead of a property of whoever
remembered to edit two.

**The alphabet is constructive, not documented.** `SAFE_COMMAND_ALPHABET` holds
three bare verbs; `RouterAction` has three arms and each returns one of them;
every emitted command is `format!("{verb} {phase}")`. There is no path from a
row to a string that does not pass through `RouterAction::verb`, so
`/gsd-complete-milestone`, `/gsd-cleanup`, `/gsd-autonomous`, `/gsd-progress`
and the rest of research §6's never-auto-select list are unreachable by
construction rather than by a denylist somebody maintains (T-20-17).

**Twelve gate arms, each with its own reason string.** `gate_verification_*`
(five), `gate_stale_check_indeterminate`, `gate_uat_outstanding`,
`gate_continue_here_project`, `gate_continue_here_phase_blocking`,
`gate_state_error`, `gate_deferred_verification`, `gate_phase_partial`. Every
one has a producer reachable from an observable state, asserted by
`every_gate_reason_is_reachable_from_some_observable_state` — a reason with no
producer is a stub wearing a constant's clothes, and it makes a grep of the
journals silently answer zero.

**The dependency conditions, both halves.** `deps_satisfied` reads the roadmap
entry's declared `**Depends on**:` line and requires each named phase to be
complete by either of upstream's two sources (inferred completion or a `- [x]`
roadmap checkbox). The collision filter withholds an execute action while any
phase related to the target by a declared dependency path — **in either
direction, transitively, with a cycle guard** — is `partial`.

**`tests/driver_router_table.rs`** — twelve guards. The both-directions rule/row
guard with a control arm that truncates the table and proves the check fires;
the disjointness proof; the alphabet source scan with a non-vacuity floor and a
matcher control arm on a synthetic offending and a synthetic clean snippet; the
three empty-input shapes; the hundred-iteration determinism property over two
insertion orders; the never-auto-select disjointness; the waiting-signal
non-goal; and the assertion that no non-comment line under `src/` names the
oracle verb.

**`tests/driver_router_conformance.rs`** — nine fixture project trees, one per
covered state, each built by *writing the artifact files both readers look for*
rather than by constructing reader structs. GSD's own router runs over each tree
and its recommendation is compared with `decide`'s. Result on this machine:
**5 real command-to-command comparisons, 2 declared divergences, 2
upstream-silent states.**

## Key Decisions

**`/gsd-verify-work` is deliberately absent from the alphabet, and the
conformance test declares the resulting divergence rather than hiding it.**
Upstream routes an `executed` phase to `verify` with a concrete command
(`/gsd-verify-work <N>`, or `/gsd-plan-phase <N> --gaps` for `gaps_found`). The
verification statuses that put a phase in that state *are* the DRIVE-05 gate
set, and `verify-work` is the command a human runs to answer them — so
auto-selecting it would be the router answering the question the gate exists to
ask. This is a choice against the upstream default (research A4 flags it), so
the oracle carries it as an `Upstream::VerifyGate` declaration whose arm fails
in **both** directions: if upstream stops emitting `verify` for that state, or
if this router ever stops parking, the build goes red naming the fixture. An
undeclared divergence is indistinguishable from a bug.

**Gates are evaluated before goal-met, and that ordering is the fail-closed
one.** A target whose verification passed while a human-judgement gate is still
open is a contradiction on disk; reporting victory over the open gate is
precisely the failure DRIVE-05 exists to prevent. The consequence is stated
rather than discovered: a project carrying an unrelated root `.continue-here.md`
parks instead of reporting goal-met.

**The verification gates are consulted only for an `executed` target, and
getting this wrong would have bricked the router.** `VerificationStatus::Missing`
is the status of every phase that has not been executed yet — an `empty` or
`discussed` phase has no verification artifact and legitimately should not. A
gate keyed on `missing` without that scope would park every phase in every
project forever, before a single plan was ever written. Upstream consults its
verification routing table exactly when `disk_status == 'executed'`
(`init.cjs:2028-2036`), and this router does the same.

**G6 `stale_check_indeterminate` was recovered from a fact the reader already
records, rather than invented or stubbed.** Upstream raises its own
`staleCheckIndeterminate` flag when the staleness scan fails on an fs, scan or
clock error; this repository's reader has no such flag and degrades any
unreadable or unparseable verification artifact to `Missing`. The two cases are
split on `has_verification`: **no artifact at all** is G4 `missing` (nothing was
checked because there was nothing to check), and **an artifact that yielded no
status** is G6 (the scan ran and produced no conclusion) — which is precisely
the distinction `verification.cjs:343-357` insists on. Both park, so nothing
turns on the split being wrong; the split exists so a journal reader can tell an
unwritten verification from an unreadable one without opening the tree.

**An absent map entry now parks under `no_disk_inference` instead of defaulting
to `no_directory`.** The old `unwrap_or_default()` conflated two opposite facts.
`no_directory` is an *observation* — the reader looked and found no phase
directory — and GSD's own table routes forward from it. An absent key is the
*absence of an observation*: `parse_project_state` inserts one inference per
roadmap phase, so a missing key means the value did not come from that reader.
Routing forward from it would be a command chosen from nothing, and the plan's
own truth requires those three empty-input shapes to park.

**The collision filter models the clock-free half only, and the omission is
argued rather than silent.** Upstream's filter also withholds actions while a
related phase is *active*, where active means "a file in its directory was
modified in the last five minutes". `decide` is a pure function with no clock by
design — the property that makes determinism a value property rather than a
filesystem experiment — and worse, the signal would be **self-referential**: the
driven agent is the thing writing into those directories, so a driver reacting
to five-minute mtimes would be reacting to its own output. The `partial` half
needs no clock and catches the collision that matters.

**The reason enum carries two prefixes.** `router_` marks a refusal about the
table; `gate_` marks a human-judgement gate the run refused to answer. Research
OQ2 and CONTEXT OQ2 both name `gate_verification_gaps_found` specifically, and
the split is what makes "how often does the always-park posture stop a run, and
at which gate?" a grep over the journals rather than a memory.

## Wave 1's three stubs

All three are closed, and none needed a line of `src/driver/run.rs`:

| Wave-1 stub | Status | How |
|---|---|---|
| `Decision::GoalMet` has no producer | **closed** | `is_goal_met` returns `verification_status.is_passed()`; `decide` returns `GoalMet` for a target whose verification passed. `a_target_phase_whose_verification_passed_is_goal_met` and the eight-status × five-state negative sweep pin both directions. |
| `RouterReason::DependencyUnsatisfied` has no producer | **closed** | Two producers: an unsatisfied declared dependency, and the partial-phase collision filter. Five tests cover both, plus the cycle guard and the "roadmap does not declare the dependency" fail-closed arm. |
| `GOAL_MET_LABEL` is unreachable in `src/driver/run.rs` | **closed, with no edit to that file** | `run.rs:2247-2253` already routes `Terminal::Completed` with no spawn to `GOAL_MET_LABEL`, and `run.rs:1823` already consumes `Decision::GoalMet`. Giving the decision a producer made the constant reachable with zero changes to a file this wave's sibling plan owns. |

`DiskStatus::Executed routes to NoRule`, 20-03's fourth stub, is also closed: it
now routes to the verification gates.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2's router half landed in Task 1's commit**
- **Found during:** Task 1
- **Issue:** `decide`'s contract is that gates are consulted *before* any
  forward-motion rule. A Task 1 commit that shipped the table without the gates
  would have had to route `Executed` and `Partial` to `NoRule` and then re-route
  them one commit later — churning exactly the reason vocabulary Task 1's own
  `<reversibility rating="costly">` warns is durable the moment it is journalled.
- **Fix:** The table, the alphabet, the gates and the goal-met predicate landed
  together in `f06c610`. Task 2's separable half — the residual-exposure
  disclosure and the written non-goals with their pinning tests — is `02cfeb7`.
- **Commit:** f06c610

**2. [Rule 1 - Now-false prose] `journal/mod.rs`'s `Parked` reason table said
`RouterReason` carries the `router_` prefix**
- **Issue:** The table names three sanctioned taxonomies and their on-disk
  prefixes. The gate arms make the `RouterReason` row false, and a reader
  greppping by that table would find no `gate_*` producer.
- **Fix:** The row now documents both prefixes and why the split exists.
  `src/journal/mod.rs` is outside this plan's declared file list; it is not in
  plan 20-05's either, so the merge stays clean.
- **Commit:** 02cfeb7

**3. [Rule 1 - Now-false prose] `dry_run.rs`'s `GoalMet` arm claimed the
decision had no producer**
- **Fix:** The comment now says what the arm means. The behaviour is unchanged —
  `PreviewScope::Complete` with an empty command list was already the right
  answer and is now reachable.
- **Commit:** 02cfeb7

**4. [Rule 1 - Guard-driven] An assertion message in `router.rs` spelled a
forbidden command as a literal**
- **Found during:** Task 1, by the alphabet guard going red on its first run
- **Issue:** A test's failure message read *"defaulted to /gsd-progress"*. That
  is an executable line, so the source scan counted it as the router naming a
  never-auto-select command — correctly.
- **Fix:** The message names the progress command in prose instead, with an
  in-source comment explaining why. The alternative was excluding the in-source
  test module from the scan, which would have weakened the guard for the arm
  that mattered in order to spare the arm that did not.
- **Commit:** f06c610

### Judgement Calls Worth Review

**5. The tasks were committed atomically rather than as RED/GREEN pairs**, for
the reason 20-03 recorded in this same phase: a test-only commit referencing
`RULE_TABLE`, `RouterAction` or the new `RouterReason` arms does not compile
before the types carry them, so the RED commit would be a broken commit in
history rather than a failing test. In each task the tests were written from the
plan's `<behavior>` block before the implementation, so the specification order
held even though the commit granularity did not. The one genuine RED happened
anyway and is deviation 4 above: the alphabet guard failed on its first run and
found a real violation.

**6. Two in-source tests that pinned the old behaviour were rewritten, not
deleted.** `an_uncovered_status_parks_as_no_rule_and_names_what_was_observed`
asserted that six of the eight statuses reach `NoRule` — true when the table had
one row and false the moment it had five. It is replaced by
`a_state_the_table_does_not_cover_parks_as_no_rule_naming_it`, which exercises
the one state that genuinely has no rule (`Complete` with a non-passing
verification: a violation of the reader's own invariant, expressible by a
hand-edited or foreign tree). `a_declared_phase_with_no_disk_entry_at_all_parks_
rather_than_panicking` kept its name and its intent and gained the second shape
and the new observed token. Both moved in the same commit as the code, per
CONVENTIONS.md:75.

**7. The repo self-check was run out-of-band rather than pinned as a test**,
following 20-03's precedent. Pinning "phase 19 parks as `human_needed`" would
turn a future `/gsd-verify-work 19` into a build failure for an unrelated
reason. The values are recorded under Verification below.

**8. The `gaps_found`-versus-`stale` precedence test asserts the pair that can
actually co-occur.** The plan asks for a state where both are observable. In
this build they cannot be: `stale` and `gaps_found` are two values of one
frontmatter field, where upstream derives staleness from mtimes independently of
the field. The documented precedence is proved on the pairs that do co-occur
(project-level hard stop over phase-level gate; verification over UAT and
deferred-verification; `gaps_found` over deferred-verification), and the
impossibility is written into the test rather than left as a silent gap.

## Known Stubs

| Stub | File | Reason / who resolves it |
|---|---|---|
| `src/driver/run.rs:1317-1321`'s `GOAL_MET_LABEL` doc still says the constant is *"Unreachable in this plan"* | `src/driver/run.rs` | **The constant is now reachable and the code needs no change; only the prose is stale.** `src/driver/run.rs` belongs to plan 20-05 in this wave and this plan's brief forbids editing across that boundary — a parallel edit to the same file is the one thing that breaks the wave's merge. One paragraph, no behaviour. Owed to whoever touches `run.rs` next. |
| `is_active` half of the dependency-collision filter is not modelled | `src/driver/router.rs` | Argued in `colliding_partial_phase`'s doc: a pure function has no clock, and the recency signal would be self-referential because the driven agent writes into the directories it would measure. The conformance oracle is what surfaces a divergence that matters. |
| `VerificationStatus::Stale` is still never *derived* | `src/state_reader/disk_status.rs` (20-03's stub, unchanged) | The router recognises and parks on it; the mtime comparison that produces it is unimplemented. Named again here because it is the partial mitigation for T-20-19, and this plan's residual-exposure disclosure says so in as many words. |
| G8, G12, G14, G16–G20 remain unmodelled | reader | 20-03's deferred list. G16–G20 leave no disk trace by construction; the others need per-item parsing nothing has scoped. An unobservable gate leaves the target in a state the table either covers or parks on, so none of them can produce a route *past* a gate. |

None prevents this plan's goal.

## Deferred Items

- **`run.json` still does not record the bounds in force**, and `dry_run.rs`'s
  `SECTION_COMMANDS` paragraph is still stale — both carried from 20-01, both
  in files this plan does not own.
- **The conformance fixtures do not cover a dependency-collision state.** The
  oracle's `is_active` computation makes every freshly-written fixture phase
  active, which would exercise upstream's clock-dependent half against a router
  that deliberately does not model it. The dependency rules are proved by value
  tests instead; a fixture that could age its own mtimes would close this.
- **No fixture exercises `stale`, `unknown` or the indeterminate split against
  the oracle**, because upstream derives `stale` from mtimes and constructs
  `unknown`/`missing` internally. All three are covered by value tests.

## Threat Flags

None. Every file touched is either a pure function over typed state, a test, or
a documentation paragraph. No network surface, no auth path, no schema at a
trust boundary. T-20-17, T-20-18, T-20-20, T-20-21 and T-20-22 are mitigated as
the register requires — the alphabet guard, the typed-fields-only rule (no
routing arm reads artifact body text; every `detail` is a phase number or a
token chosen by explicit comparison), the goal-met predicate, the absence of a
default arm, and the oracle source scan respectively. T-20-19 remains accepted
and is now disclosed in `src/envelope/mod.rs` with the staleness detector named
as the partial mitigation it is.

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean, no warnings |
| `rtk proxy cargo test --lib router` | **30 passed**, 0 failed |
| `rtk proxy cargo test --lib envelope` | 167 passed, 0 failed |
| `rtk proxy cargo test --test driver_router_table` | **12 passed**, 0 failed |
| `rtk proxy cargo test --test driver_router_conformance` | **2 passed** — 5 command comparisons, 2 declared divergences, 2 upstream-silent states over 9 fixtures |
| `rtk proxy cargo test` | **29 targets, 0 failures** |
| `cargo clippy -- -D warnings` (documented gate) | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly the **5** pre-existing lints, at the same locations recorded in `TESTING.md` (`browser.rs:131-133`, `project_creator.rs:146`, `state_reader/mod.rs:279`) |

**The router run against this repository's own planning directory**, out-of-band
via a temporary probe (deleted; tree clean):

```
target 19 -> Park { reason: GateVerificationHumanNeeded, detail: "19" }
target 20 -> Park { reason: GatePhasePartial,            detail: "20" }
target 21 -> Park { reason: DependencyUnsatisfied,       detail: "20" }
target 22 -> Park { reason: DependencyUnsatisfied,       detail: "20" }
```

Phase 19 parks naming the human-needed verification gate — the concrete case
that motivated 20-03's reader fix, and the exact line the plan's
`<verification>` block asks for. Phase 20 reads `partial` mid-execution and
parks as such. Phases 21 and 22 park on phase 20 being an unsatisfied declared
dependency.

**Flaky-test advisory checked deliberately** (research Pitfall 7): one full run
showed `driver_reattach::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_
journal_step` red at **0.53s**; the same target in isolation immediately after
passed 3 of 3 at **6.14s**. That order-of-magnitude timing anomaly is the
signature phase 19's anti-pattern table describes — the work never happened, the
test did not race. Three other full runs were green across all 29 targets.
Nothing in this plan touches the driver's lock, spawn or reattach paths.

Every `rtk proxy` above is deliberate: `rtk` strips cargo's `warning:` and `test
result:` lines, so a grep of those against bare `cargo` succeeds vacuously.

## Self-Check: PASSED

Created files verified present and non-empty: `tests/driver_router_table.rs`,
`tests/driver_router_conformance.rs`.
Modified files verified present: `src/driver/router.rs`, `src/envelope/mod.rs`,
`src/journal/mod.rs`, `src/driver/dry_run.rs`.
Commits verified in `git log`: `f06c610`, `02cfeb7`, `2abffa4`.
`git diff --diff-filter=D` over the whole plan range is empty — no file was
deleted. `src/driver/run.rs` and `src/driver/mod.rs` are untouched, as the
wave's parallel-execution boundary requires.

## Notes on `actuals`

`tokens: 35000` is chars/4 over the realized diff (138,985 characters across 6
files, measured with `rtk proxy git diff` — the rtk-filtered `git diff` reports
27,770 and would understate it by 5×, the same trap 20-03 recorded). The
estimate was 46,000 at `confidence: low`, so the plan came in about 25% under.
The gap is that Tasks 1 and 2 turned out to be one change to one file rather
than two, and that the conformance oracle's fixtures reduced to one skeleton
plus four three-line writers once the minimal tree GSD's router accepts was
established empirically rather than guessed.
