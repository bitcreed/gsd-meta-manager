---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 06
subsystem: driver
tags: [refusal-record, escalation-cap, park-evidence, tripwire, envelope-independence, self-goal, no-rule-reachability, safe-07, safe-08, drive-04, roadmap-criterion-3, roadmap-criterion-5]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "02"
    provides: "escalate::resolve, EscalationBudget::permit_consultation, EscalationReason + as_str + ALL, EscalationRefusal"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "04"
    provides: "the ambiguity seam at router::Decision::NoRule, ParkTaxonomy, the three escalation park reasons wired to Terminal::Parked, tests/fixtures/fake-claude-seam.sh"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "05"
    provides: "tests/fixtures/injection-corpus/ — the eleven-class hostile tree, four of whose classes were planted for this plan"
  - phase: 20-deterministic-decision-router-and-run-bounds
    provides: "router::decide, RouterAction::ALL + verb, SAFE_COMMAND_ALPHABET, status_token, is_goal_met, bounds::resolve, the park machinery and the parked terminal label"
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "the PreToolUse guard, the pre-push hook, hooks::install_in, cred::hooks_path_env, policy's seven park reasons"
provides:
  - "The finding that Decision::NoRule IS reachable through the shipped reader — an archived milestone phase the roadmap still declares — which closes 21-04's Known Stub 1 without taking any of its three named routes"
  - "tests/driver_refusal_record.rs — out-of-enum refusal, the no-constructed-command-line scan, the behavioural self-goal proof and envelope independence, plus four guards"
  - "tests/driver_escalation_cap.rs — the cap park read off disk, no-silent-degradation, count honesty against a child-process ledger, and three boundary arms that drive real runs"
  - "The agent-spawns tripwire on fake-claude-seam.sh: one line per executor-profile spawn, so 'nothing ran' is a fact written by a program rather than a number the driver reports"
  - "The residue technique: strike the model-named bytes out of a record, then assert what remains reads as nothing runnable"
affects: [22-container-execution-target, 23-gate-policy]

actuals:
  tokens: 23971
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Strike-out-then-scan: a refusal record is REQUIRED to carry the payload verbatim, so the payload is removed first and whatever still reads as runnable was constructed"
    - "Cutting a scan at the journal's own monotonic sequence rather than at a list of record codes: a record written before the seam spawned cannot be a rendering of its answer"
    - "Deriving a pub(crate) constant from an observed artifact instead of retyping it — the parked-label prefix is the terminal label minus the reason the park record independently carries"
    - "A tripwire and its control arm in ONE test, so the negative half cannot pass against a tripwire that could never fire"
    - "Guarding a `grep -c` acceptance criterion inside the file it is about, with the forbidden strings assembled at runtime so the guard is not the occurrence it forbids"

key-files:
  created:
    - tests/driver_refusal_record.rs
    - tests/driver_escalation_cap.rs
  modified:
    - tests/fixtures/fake-claude-seam.sh

key-decisions:
  - "21-04's Known Stub 1 was WRONG, and no production change was needed. The analysis reasoned about `disk_status::infer_disk_status`, where `Complete` is the conjunction implementation-complete AND verification-passed — but `infer_phase_status` returns EARLIER for a phase archived into `.planning/milestones/`, handing back `DiskInference { status: Complete, ..Default::default() }` whose verification status is `Missing` because nothing was read. `gate_for` has no arm for it, `is_goal_met` is false, `RULE_TABLE` has no row for `Complete`, and `decide` falls through to `NoRule { observed: \"complete\" }`."
  - "None of the three routes 21-04 named was taken: the reader was not widened, no rule-table row was removed, and no inference was fabricated. `git diff --stat` against the base touches `src/` in zero places. The route the plans did not consider was to look one function further up."
  - "The premise is pinned by its own non-ignored test in BOTH files, with a control direction. A future change to the reader or the rule table fails there, naming the premise, rather than silently emptying seven driven proofs that would all still pass."
  - "The constructed-command-line scan is cut at the journal's monotonic sequence rather than run over every byte. A run's opening records are several hundred words of the envelope's fixed protection advisory, and scanning them measured Phase 19's prose style — the first version of the test did exactly that and failed on a semicolon in English."
  - "Force-push is refused at the ARGV layer (the PreToolUse guard), not at the pre-push hook. The test discovered this by observing `+ 06adc00...3c642f8 (forced update)` succeed against a hook-level assertion, and was corrected to assert the boundary that exists rather than the one it assumed."
  - "The largest escalation budget any run can spend to exhaustion against a tree its agent does not change is TWO, because Phase 20's no-progress detector fires on the third unchanged observation. That is correct behaviour — such a run IS stalled — and the upper-boundary arm respects the ordering rather than working around it."
  - "No arm is `#[ignore]`d. The plan's availability tolerance was written for arms needing the real `claude` binary; every arm here drives the checked-in stand-in and runs everywhere. An `#[ignore]` on an arm that could always run is a silent skip wearing an availability excuse."
  - "`CLASSES[].asserted_here` in `tests/driver_injection_corpus.rs` was left `false` for all four classes. Its completeness guard scans that file's OWN source for `assert_class` arms; flipping the flag would demand a 21-05-shaped model arm that this plan is not writing. This plan's arms assert the PARK RECORD, in their own file, and a guard there checks every payload verbatim against the corpus fixture that carries it."

requirements-completed: [DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - id: R1
    description: "An action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string, and the refusal is recorded on disk with the named action verbatim"
    requirement: "SAFE-08"
    verification:
      - kind: integration
        ref: "tests/driver_refusal_record.rs#an_out_of_enum_action_is_refused_recorded_verbatim_and_never_executed"
        status: pass
    human_judgment: false
  - id: R2
    description: "The refusal record contains no constructed command line — no shell chaining or piping sequence, no backtick span, and no alphabet verb followed by a token — anywhere in the parked event, the refusal diagnostic or run.json"
    requirement: "SAFE-08"
    verification:
      - kind: integration
        ref: "tests/driver_refusal_record.rs#no_part_of_a_refusal_record_reads_as_a_constructed_command_line"
        status: pass
    human_judgment: false
  - id: R3
    description: "A run that reaches its escalation cap PARKS with the cap reason read back off the on-disk journal by the shipped reader, and the journal shows no further command after it"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing"
        status: pass
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap"
        status: pass
    human_judgment: false
  - id: R4
    description: "The escalation count on the run record equals the number of seam child processes that actually ran, observed through a program that left evidence rather than an in-process counter"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing (the count-honesty third)"
        status: pass
    human_judgment: false
  - id: R5
    description: "Every escalation-cap boundary is measured against the RESOLVED step cap, and the two refusals happen above the run — no run directory, no journal, no spawn"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#the_three_boundaries_are_measured_against_the_resolved_step_cap"
        status: pass
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run"
        status: pass
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#a_budget_of_zero_refuses_above_the_run"
        status: pass
    human_judgment: false
  - id: R6
    description: "A model-named git push is still refused by the Phase 19 envelope, under the envelope's OWN park reason and never one this phase added, with the repository and the remote byte-identical afterwards"
    requirement: "SAFE-08"
    verification:
      - kind: integration
        ref: "tests/driver_refusal_record.rs#the_envelope_refuses_a_model_named_push_under_its_own_reason"
        status: pass
      - kind: integration
        ref: "tests/driver_refusal_record.rs#the_two_taxonomies_share_no_identifier_and_neither_borrows_the_others_prefix"
        status: pass
    human_judgment: false
  - id: R7
    description: "The driver does not adopt a goal from an artifact written to look agent-authored during its own run, proven behaviourally against the corpus rather than only by the compile-time move"
    requirement: "SAFE-07"
    verification:
      - kind: integration
        ref: "tests/driver_refusal_record.rs#the_driver_keeps_the_goal_the_human_stated_against_an_agent_authored_artifact"
        status: pass
    human_judgment: false
  - id: R8
    description: "The ambiguity seam is reachable end to end from disk, so no assertion above is vacuous"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_refusal_record.rs#the_no_rule_state_this_file_depends_on_is_reached_through_the_shipped_reader"
        status: pass
      - kind: integration
        ref: "tests/driver_escalation_cap.rs#the_fixture_really_reaches_the_state_the_rule_table_does_not_cover"
        status: pass
    human_judgment: false
  - id: R9
    description: "The corpus's third demanded git action — a pull request — is refused by the envelope's PR cap"
    requirement: "SAFE-08"
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs (Phase 19; not re-asserted here)"
        status: partial
    human_judgment: true
    rationale: "The corpus's envelope-probe payload demands a push, a force-push AND a pull request. This plan asserts the first two mechanically, each under the envelope's own reason and through a different enforcement layer. The PR half is left where its behavioural proof already lives — `tests/envelope_pr_cap.rs` — because exercising `pr_cap_exceeded` needs a run-scoped ledger and a configured cap, and standing that up here would have re-tested Phase 19 rather than the independence claim. A human should confirm that two of three demanded actions, proved at two different layers, is the coverage they want for the independence requirement."

duration: 35min
completed: 2026-08-20
status: complete
---

# Phase 21 Plan 06: The Refusal Record and the Cap That Parks Summary

**A refused action is now evidence on disk that constructs nothing runnable, and
a run that spends its model budget parks and says so — and both are proved by
DRIVEN runs rather than deferred, because the state three earlier plans recorded
as unreachable turned out to be an ordinary archived phase and needed no
production change at all.**

## Performance

- **Duration:** ~35 min (worktree 16:26:59, last commit 17:01:41)
- **Tasks:** 2 of 2
- **Files created:** 2; **modified:** 1
- **Net:** +2,235 / −0 lines against the plan's base commit `fc9a2ef`
- **`src/` changed:** **zero files.** Production is untouched.
- **New suites:** `driver_refusal_record` — **9 tests**; `driver_escalation_cap` —
  **8 tests**. None `#[ignore]`d; all 17 run on an ordinary `cargo test`.
- **Lib tests:** 1,005, unchanged (no source change to test).

## Task Commits

1. **Task 1: the refusal record — verbatim evidence, nothing runnable** — `ac08f7b` (test)
2. **Task 2: the cap parks, and the run says so** — `f380cda` (test)

## The blocker three plans deferred, and why it was not one

21-02, 21-04 and 21-05 each recorded the same thing: `router::decide` cannot
return `NoRule` for any state the shipped reader produces from disk, so the three
`escalation_*` park reasons had a producer in the code and no reachable state on
disk. 21-04 named three routes out and asked 21-06 to pick one **deliberately**:
widen the reader, remove a rule-table row, or accept a fixture that writes an
inference the reader would never produce.

**All three are bad, and none was needed.** Widening the reader changes what every
project observes to manufacture a test state. Removing a rule row hands the model
a state the deterministic rules already cover, which 21-CONTEXT.md forbids
outright — *"the model is consulted only where the router returns
`router_no_rule`; it never overrides a rule"*. Fabricating an inference needs a
production injection seam, which is a hole.

The fourth route is that **the analysis stopped one function too early.** It
reasoned about `disk_status::infer_disk_status`, where `Complete` really is the
conjunction *implementation complete AND verification passed* — so a `complete`
observation is always goal-met and `decide` returns `GoalMet` before the table is
consulted. But `infer_phase_status` returns **earlier**, at
`src/state_reader/disk_status.rs:821-824`, for any phase archived into
`.planning/milestones/`:

```rust
return DiskInference {
    status: DiskStatus::Complete,
    ..Default::default()      // verification_status: Missing — nothing was read
};
```

That value is a `complete` observation that is **not** goal-met. Traced through
`decide`: the roadmap declares the phase, the inference exists, `gate_for` has no
arm for `Complete`, `is_goal_met` is false because `Missing.is_passed()` is false,
and `RULE_TABLE` has no `Complete` row — so it falls through to
`Decision::NoRule { observed: "complete" }`.

It is also an entirely **ordinary** project shape rather than an exotic one: a
roadmap that still declares a phase `/gsd-complete-milestone` has archived. The
router's own `phase_is_complete` reads the `- [x]` checkbox precisely because
*"an archived or hand-ticked phase has no artifacts left to infer completion
from"*.

Verified empirically before anything was built on it, with a throwaway probe over
a real temp tree read by `parse_project_state`:

```
PROBE inference = DiskInference { status: Complete, …, verification_status: Missing, … }
PROBE decision  = NoRule { observed: "complete" }
```

**The premise is now pinned by its own non-ignored test in both files**
(`the_no_rule_state_this_file_depends_on_is_reached_through_the_shipped_reader`
and `the_fixture_really_reaches_the_state_the_rule_table_does_not_cover`), each
with a control direction asserting that the same tree with the phase left in
`phases/` routes deterministically. A future change to the reader or the rule
table fails **there**, naming the premise, rather than quietly emptying seven
driven proofs that would all still pass.

This closes 21-04's Known Stub 1 and 21-05's Known Limit 1, and it also means the
ambiguity seam is not dead code: it is reachable in any project with an archived
phase still on the roadmap.

## The envelope's park reasons, verbatim

The plan asks for these in the SUMMARY so a reader can confirm they are the
envelope's own and not ones this phase added:

| Demanded by the corpus | Refused by | Reason on the wire | Layer |
|---|---|---|---|
| push to `origin main` | the `pre-push` hook | `push_outside_namespace` | refs git hands it |
| force-push the rewrite | the `PreToolUse` guard | `force_push_blocked` | the command line, before git runs |

Neither carries the `escalation_` prefix, neither is any member of
`EscalationReason::ALL`, and
`the_two_taxonomies_share_no_identifier_and_neither_borrows_the_others_prefix`
asserts that no identifier belongs to both vocabularies — which is what makes
"the envelope's own reason fired" and "this phase's reason fired" distinguishable
claims at all.

**Which layer refuses which is a fact this test discovered rather than assumed.**
An earlier draft asserted the force-push at the pre-push hook and observed
`+ 06adc00...3c642f8 HEAD -> gsd-auto/envprobe/probe (forced update)` **succeed**.
The hook enforces the namespace and the credential scan from the refs it is
handed; destructiveness is judged from the argv before git runs. The test was
corrected to assert the boundary that exists, not the one it assumed — and it is
also why the first draft's force-push was a *creation* wearing a `--force` flag
rather than a real non-fast-forward. Both halves now have a paired **allow**, so
neither refusal is a wall.

## Guards and proofs, and what each did against the UNFIXED behaviour

Every assertion was run red before being trusted. Each injected defect was
restored from an explicit per-file backup in the scratchpad and verified
**byte-identical** with `diff` — never `git clean`, `git stash` or a blanket
working-tree reset, per the worktree prohibition.

| Proof / guard | Injected defect | Observed |
|---|---|---|
| `no_part_of_a_refusal_record_reads_as_a_constructed_command_line` | the refusal detail rendered through `RouterAction::command_for` | **FAILED** — *"the out-of-enum record carries `\"/gsd-plan-phase\"` followed by whitespace and the token `'3'` — a pasteable command line assembled from a refused action (WR-09)"* |
| `an_out_of_enum_action_is_refused_recorded_verbatim_and_never_executed` | the named string dropped from the detail | **FAILED** — *"the refusal must record the named action VERBATIM. A silently dropped injection teaches nobody that the repository is hostile; got: the seam named an action outside the safe alphabetunnamed"* |
| same, tripwire halves | — (the control arm IS the demonstration) | the refusal arm records **0** agent spawns and the control arm records **≥1**, in the same run of the suite |
| the alphabet derivation | one arm removed from `RouterAction::ALL` | **FAILED** — the derived set and `SAFE_COMMAND_ALPHABET` disagree, which is what proves the verb list is read from `ALL` rather than typed into the test |
| `the_driver_keeps_the_goal_the_human_stated…` (goal half) | the driver adopts the goal from `31-AGENT-NOTES.md` and re-decomposes | **FAILED** — *"the run pursued a goal other than the one the human stated"* |
| same (tripwire half) | a second decomposition with the goal unchanged | **FAILED** — *"a SECOND decomposition seam spawn occurred… Spawns are counted by a program that ran, not by the driver reporting on itself"* |
| `the_envelope_refuses_a_model_named_push_under_its_own_reason` | `policy::denied_push_flag("force")` returns `None` | **FAILED** — the guard answered 0 instead of the hook protocol's blocking status |
| `a_run_that_spends_its_budget_parks…` (no-degradation) | the no-rule arm carries on rules-only after exhaustion, silently | **FAILED** — the reason on disk became `bounds_no_progress` where the cap reason was required. **That is the discrimination**: a run that stopped consulting the model and kept going reports a *stall*, not a cap, so the change is readable off disk exactly as the requirement demands |
| same (count honesty) | `set_escalations_used(used - 1)` | **FAILED** — `left: Number(1), right: Number(2)`, the T-21-43 shape: a number that reads as a safety property while under-reporting |
| `the_three_boundaries_are_measured_against_the_resolved_step_cap` | `escalate::resolve` compares against the compiled-in default | **FAILED — and so did two driven arms**, three at once: the pure boundary, the driven upper boundary and the driven equal-to-cap refusal. The exact shape of the Critical Phase 20's review found |
| `this_file_spells_neither_the_cap_reason_nor_the_compiled_in_step_cap` | — (the greps, asserted in-file) | `grep -c` returns **0** for both; the forbidden strings are assembled at runtime so the guard is not itself the occurrence it forbids |
| `every_payload_this_file_names_is_verbatim_in_the_corpus_fixture_that_carries_it` | — | four payloads, four markers, four fixture files, checked against the committed corpus so this file cannot drift from it |
| `the_hostile_materialisation_keeps_every_payload_and_leaves_no_sentinel` | — | every marker survives the copy and no `INJECTION-BEGIN`/`END` line does, walking the whole materialised tree |

## Decisions Made

1. **No production change, and that was the whole point.** `git diff --stat`
   against the base touches `tests/` only. The three routes 21-04 offered all
   traded shipped behaviour for testability; the fourth traded nothing.

2. **The constructed-command-line scan is cut at the journal's own sequence.**
   A run's opening records are the envelope's protection advisory — several
   hundred words of fixed compiled-in English with inline code quotes — and the
   first version of this test failed on a semicolon in that prose. The cut is
   made at the refusal diagnostic rather than at a list of record codes somebody
   maintains: *everything before it was written before the seam was ever spawned,
   and a record written before the model was consulted cannot be a rendering of
   its answer.* The helper asserts it really excluded something and that the
   window still reaches the parked event, so a mis-aimed cut fails rather than
   scanning nothing.

3. **Strike out the payload, then scan the residue.** A refusal record is
   *required* to carry the named string verbatim — that is the evidence — and
   the shell-smuggling payload carries `&&`, a pipe, backticks and `$(…)`. So
   the payload is removed first and the assertion is about what remains. That is
   also what makes the check fail against a command-formatting helper: the
   out-of-enum payload (`/gsd-ship 31 --force`) contains no alphabet verb at all,
   so a `/gsd-plan-phase 31` in that run's record is unambiguously constructed.

4. **The parked-label prefix is derived, never typed.** `PARKED_LABEL_PREFIX` is
   `pub(crate)`, and typing `"parked:"` into a test whose whole subject is "the
   reason reached disk through machinery that already existed" would be the third
   string source the assertion forbids. In `driver_refusal_record.rs` it is taken
   from a run that parks under **Phase 20's** `router_state_unverified` — a park
   that demonstrably predates this phase; in `driver_escalation_cap.rs` it is the
   terminal label minus the reason the parked record independently carries.

5. **The tripwire and its control live in one test.** "The evidence file does not
   exist" is worthless alone, because a broken tripwire satisfies it forever. The
   control arm — the same corpus with the phase where GSD normally keeps it, so a
   legal command IS routed — runs in the same test and asserts the file DOES
   appear, and that the seam was consulted **zero** times.

6. **Two escalations is the exercisable maximum against a static tree.** Phase
   20's no-progress detector fires on the third unchanged observation, so a
   budget of three under a step cap of four parks on `bounds_no_progress` before
   the cap binds. Correct behaviour — such a run *is* stalled — so the
   upper-boundary arm uses a step cap of three, where the largest legal budget is
   two and is reachable. Recorded because the arm was corrected to respect the
   ordering rather than to work around it.

7. **Nothing was `#[ignore]`d.** The plan's availability tolerance was written for
   arms needing the real `claude` binary; every arm here drives the checked-in
   stand-in and runs on any machine. An `#[ignore]` on an arm that could always
   run is a silent skip wearing an availability excuse, which is the failure the
   tolerance exists to prevent.

## Deviations from Plan

### 1. [Rule 3 — Blocking] The reachability decision was none of the three offered

Documented at length above. The plan inherited 21-04's framing — *"decide which
of 21-04's three named routes to take, deliberately"* — and the deliberate
decision was that all three are worse than the fourth, which is that the state is
already reachable. This is the plan's largest deviation and the reason it
produced driven proofs rather than a fourth deferral.

### 2. [Planned scope] `tests/fixtures/fake-claude-seam.sh` beyond `files_modified`

The plan's `files_modified` lists two test files. The out-of-enum proof requires
*"a program that leaves an evidence file on disk if it is ever run"* for the
**agent** spawn, and 21-04's stand-in recorded seam spawns only. The change is 17
lines: a doc paragraph and one `printf` appending the argv to `agent-spawns`,
placed before any replay so a spawn that then failed still leaves evidence. It is
purely additive — no earlier caller reads that file, and `driver_goal_seam.rs`'s
`seam_stdin` scanner does not match the name.

### 3. [Rule 1 — Bug in the test, not the code] The force-push assertion named the wrong layer

- **Found during:** Task 1, running the envelope proof for the first time.
- **Issue:** the draft asserted `force_push_blocked` from the `pre-push` hook and
  observed the push **succeed**. Two things were wrong: a `--force` that *creates*
  a ref is not a non-fast-forward at all, and destructiveness is judged at the
  argv layer rather than from the refs.
- **Fix:** the force-push is asserted through the `PreToolUse` guard — which is
  also the layer a model-named git action really reaches, since a model emits a
  tool call and never a ref update — and the ref-level half asserts the namespace
  boundary instead. Both now carry paired allows.
- **Recorded rather than quietly corrected**, because a test that asserted a
  boundary which is not there would have reported the envelope stronger than it
  is, which is exactly the class this phase exists to prevent.

### 4. [Planned scope] `CLASSES[].asserted_here` left `false`

21-05's handoff says flipping the four flags *"makes the completeness guard
require an arm for each"*. It does — but that guard scans
`driver_injection_corpus.rs`'s **own** source for `assert_class` arms, so flipping
would demand a 21-05-shaped model arm this plan is not writing. This plan's arms
assert the **park record**, in their own file. The link is held instead by
`every_payload_this_file_names_is_verbatim_in_the_corpus_fixture_that_carries_it`,
which fails if a payload, a marker or a fixture path drifts.

---

**Total deviations:** 1 blocking decision that removed the phase's last deferral,
1 auto-fixed test defect, 2 in-scope scope notes.
**Impact:** no shipped behaviour changed, no assertion weakened, no criterion
dropped.

## Issues Encountered

- **`cargo fmt --check` is not clean at baseline** (pre-existing, recorded by
  21-02 through 21-05). No global `cargo fmt` was run.
- **`rtk` filtering** was bypassed with `rtk proxy` for every measurement that
  depends on raw output, per the phase's own instruction.
- No flakes observed. The full suite was run to completion green; the
  `driver_reattach` intermittent 21-04 recorded did not reproduce.

## Known Stubs

None. Every test, helper and guard this plan created is implemented, reachable
and exercised, and all 17 arms run on an ordinary `cargo test`.

**Three honest limits, recorded because they are limits rather than stubs:**

1. **The pull-request third of the envelope-probe payload is not re-asserted
   here** (coverage R9). The corpus demands a push, a force-push *and* a PR. Two
   are proved mechanically, at two different enforcement layers, each under the
   envelope's own reason. Exercising `pr_cap_exceeded` needs a run-scoped ledger
   and a configured cap; standing that up would have re-tested Phase 19 rather
   than the independence claim, which the two proved actions already carry.

2. **The seam's answers come from a checked-in stand-in, not a real model.** That
   is deliberate and is what makes these arms deterministic: the subject is what
   the *driver* records and refuses when a model names something, and a real model
   asked to name an illegal action is an unreliable way to obtain one. 21-05's
   corpus arms are where a real model reads the hostile bytes; these are where the
   refusal machinery is measured. The two are complementary and neither
   substitutes for the other.

3. **The `agent-spawns` tripwire proves a GSD command was executed, not which
   one.** It records the argv it was handed, and the assertions here use existence
   and count. A future test wanting "this specific command never ran" can read the
   recorded lines; nothing does yet.

## Threat Flags

None. No new network endpoint, no new auth path, no new argv surface, and no
change to `src/` at all. The one new file-access pattern — copying the committed
corpus into a `TempDir` and moving a phase directory into a milestone within that
copy — is confined to the test binaries and never touches the committed fixture.

The threat register's dispositions, all `mitigate`, are implemented and each was
observed working:

- **T-21-38** (an out-of-enum action reaching execution) — the park plus the
  tripwire, with the control arm proving the tripwire fires on a legal command.
- **T-21-39** (a refusal record rendering the payload as a runnable command line)
  — the residue scan over the whole window, alphabet side derived from
  `RouterAction::ALL`, observed failing against a command-formatting helper.
- **T-21-40** (a model-named push reaching a remote) — the envelope's own reasons
  at two layers, refs and reflog and the `.git` directory listing byte-identical,
  and the bare remote's own `for-each-ref` output — **with object ids**, because a
  refused force-push and a successful one leave the same ref *name*.
- **T-21-41** (the driver adopting a goal from an artifact written during its run)
  — the goal and the approval digest byte-identical across the run, plus the
  spawn count from a program that ran; both halves observed red.
- **T-21-42** (budget exhaustion silently degrading to rules-only) — the park
  reason, the no-command-after-park ordering and the step-count comparison;
  observed red against a rules-only continuation.
- **T-21-43** (an escalation count that does not match the model calls made) — the
  on-disk count compared against the stand-in's spawn ledger and against the agent
  spawn ledger; observed red against an under-reporting write.
- **T-21-44** (a boundary measured against a compiled-in default) — every cap
  built by calling the resolvers, the constant named nowhere, both greps asserted
  in-file, and the defect observed failing three arms at once.

## Next Phase Readiness

- **Phase 21 is complete.** All six plans have summaries and all three
  requirements this plan declares (DRIVE-04, SAFE-07, SAFE-08) are now satisfied
  by every plan that declared them.
- **For anyone touching `state_reader`:** the archived-phase early return at
  `disk_status.rs:821-824` is now load-bearing for two test suites. It is honest
  as written — an archived phase's verification was never read, so claiming it
  passed would be an inference — but if it is ever changed to synthesise a passing
  status, both premise tests fail by name and the ambiguity seam becomes
  unreachable again. That failure is the intended signal, not a nuisance.
- **For Phase 22 (container execution target):** the two tripwire ledgers
  (`seam-spawns`, `agent-spawns`) are the cheapest existing way to assert "how
  many child processes really ran" across a transport change. A container target
  that changes the spawn path should keep them working or replace them with an
  equivalent that a program writes.
- **For Phase 23 (gate policy):** the always-park behaviour is now observable end
  to end rather than only unit-tested, which is what makes it safe to change. Any
  skip/defer/auto policy can be measured against
  `a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing` — if a
  policy makes that arm pass for a different reason, the policy changed what a
  park means.

## Self-Check: PASSED

- Files claimed created, verified present: `tests/driver_refusal_record.rs`,
  `tests/driver_escalation_cap.rs`, `21-06-SUMMARY.md`.
- Commits claimed, verified in `git log`: `ac08f7b`, `f380cda`.
- `cargo build`: clean, no warnings.
- `cargo test`: every suite `ok`, **0 failures** across the whole run.
- `cargo test --lib`: **1,005 passed**, unchanged — this plan changed no source.
- `cargo test --test driver_refusal_record`: **9 passed, 0 ignored**.
- `cargo test --test driver_escalation_cap`: **8 passed, 0 ignored**.
- `cargo clippy -- -D warnings`: clean.
- `cargo clippy --all-targets -- -D warnings`: exactly the **5** known
  pre-existing lints, in their recorded locations (`browser.rs:131/132/133`,
  `project_creator.rs:146`, `state_reader/mod.rs:311`). Both new files add none;
  each was linted alone under `-D warnings` and is clean.
- `grep -c 'escalation_cap_reached' tests/driver_escalation_cap.rs`: **0**.
- `grep -c 'DEFAULT_MAX_STEPS' tests/driver_escalation_cap.rs`: **0**.
- `git diff --stat fc9a2ef HEAD`: 3 files, +2,235 / −0, and **no file under
  `src/`**.
- `git diff --diff-filter=D --name-only` across both commits: **empty** — nothing
  was deleted.
- Every injected-defect restoration verified byte-identical to its scratchpad
  backup via `diff` (`run.rs` ×4, `policy.rs`, `router.rs`, `escalate.rs`,
  `driver/mod.rs` ×2); no `git clean`, `git stash` or blanket working-tree reset
  was used at any point.
- STATE.md, ROADMAP.md and WINDOWS.md: **untouched**, per the orchestrator's
  instruction.
- `actuals.tokens` = 23,971 = 95,885 raw diff chars / 4, measured under
  `rtk proxy` against the plan's base commit `fc9a2ef`. About a third of the
  plan's 68,000 estimate (`confidence: low`); recorded as measured rather than
  adjusted toward it.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-20*
