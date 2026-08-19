# Phase 20: Deterministic Decision Router & Run Bounds — Research

**Researched:** 2026-08-19
**Domain:** Rust async control flow (tokio), GSD 1.10.0 runtime contract archaeology, Claude Code `stream-json` rate-limit protocol
**Confidence:** HIGH on every load-bearing claim — this phase's unknowns were all resolvable by reading files on this machine, not by web search.

<user_constraints>
## User Constraints (from CONTEXT.md)

> **Provenance warning, copied from CONTEXT.md's own header.** These were gathered in *smart discuss* mode during an autonomous run: "Recommendations were auto-accepted per an explicit user instruction to run fully autonomously; every answer below is a proposal the user did not individually confirm. Treat them as decisions of record, but flag any that the research step contradicts rather than silently honouring them." Two are flagged below.

### Locked Decisions

**Router Surface & Determinism**

- The router lives in a new `src/driver/router.rs`, beside `run.rs`, as pure functions. It is not a new subsystem and not an extension of `state_reader`.
- Its input is the `ProjectState` produced by `state_reader::parse_project_state` — the same reader the dashboard and the driver already share. The decision function performs **no I/O itself**; the caller reads state and hands it in. D-11 forbids a parallel reader, and a pure function is what makes criterion 1's "the same project state always yields the same choice" testable as a value property rather than a filesystem experiment.
- Determinism is proved by an exhaustive table-driven test over every D-R-P-E-V state the rules cover, plus a same-input-same-output property test. "For every state the rules cover" is the criterion's own wording, so the test table and the rule table must be derived from one another — a rule with no row, or a row with no rule, should fail the build.
- A state the rules do **not** cover parks the run with a `router_no_rule` reason that names the observed state. It never falls back to a model call, never defaults to `/gsd-progress`, and never guesses. An uncovered state is a gap in the rule table, and parking is how that gap becomes visible.

**Progress Detectors & Run Bounds**

- The observed-state hash is a stable hash over the same `ProjectState` value the router reads, plus the git `HEAD` sha and dirty flag already captured by `executor::outcome::RunSnapshot`. It is not a scan of `.planning/` file contents or mtimes — the reader is already the single source of truth and `ProjectState` already derives `PartialEq`.
- Two detectors, both required by criterion 2: **unchanged state hash across 2 consecutive iterations** halts, and **the same command re-selected twice in a row** halts. The halt names which of the two fired.
- Default step cap: 20 iterations. Default wall-clock cap: 4 hours. Both are overridable per run on argv and are recorded in `run.json` so the bound in force is readable after the fact rather than inferred from the binary's defaults.
- When several bounds could trip on the same iteration, evaluation order is fixed and documented, the first detector wins, and the halt reports exactly one detector. Criterion 2 says "names which detector fired" — a list of detectors is not that.

**Quota & Rate-Limit Handling (CTRL-07)**

- A rate limit is detected from the `result`/error envelope of the stream-json transport, never from the agent's prose. This is the same rule `executor/outcome.rs` already enforces (D-10) and the same reason "Trusting agent self-reports" is in REQUIREMENTS' out-of-scope list.
- On a rate limit the run **parks immediately**: no retry, no backoff, no sleep-until-reset. CTRL-07's wording is "parks rather than retrying"; a backoff is a retry with a delay.
- The park reports which quota window blocked it (5-hour vs 7-day) and when it resets, taken from the API-provided reset field. If that field is absent, the run reports the window as unknown and says so. It does not estimate, and it does not present a dollar figure as "what this run cost" (D-16).
- Concurrency: a global default cap of one concurrent driven run, validating OQ10's concern that the 5h/7d quota is shared across every Claude surface the user has, so N driven projects burn it N×. No synthetic quota accounting.

**Terminal Classification & GSD-Gate Parking**

- One exhaustive terminal type — goal-met, parked-with-reason, halted-with-reason — with no unclassified arm. Criterion 5's "never as an unclassified 'loop ended'" is a type-level requirement, not a logging convention: if the type cannot express "ended for no stated reason", the failure mode is unrepresentable.
- **OQ7 resolved: always park.** Reaching a GSD gate that requires human judgement parks the run with the gate named, from an explicit enumerated gate set (verification `human_needed`, verification `gaps_found`, outstanding UAT, an execution checkpoint awaiting a decision, milestone completion, and a blocker prompt). This is the research recommendation and it is accepted with its consequence stated plainly: a fully autonomous run parks at every phase boundary by design. That is the value-proposition call OQ7 asked for, made explicitly. If the phase's own dogfooding shows this makes the driver useless in practice, that is a finding to record — not grounds to auto-answer a judgement gate.
- Reason plumbing: `envelope::policy::ParkReason` keeps its existing seven safety arms untouched; the router/bounds reasons live in a sibling enum. Both flow through the one terminal record and the one journal event, so a reader greps one place for "why did this run end". Phase 19 established the `Parked` journal event carrying a reason string read back off disk by a different process — reuse that carrier rather than inventing a second.
- Goal-met is derived from deterministic project state (the declared target reached — e.g. the target phase's verification passed, or the milestone closed), never from the agent saying it is done.

### Claude's Discretion

- Module/file decomposition below `src/driver/router.rs`, naming of the bounds reason enum and its `as_str` constants, and the exact shape of the rule table (match arms vs a data table) are open — follow `CONVENTIONS.md` and the pattern `envelope/policy.rs` already set for pure classification functions with a stable snake_case reason string per arm.
- Whether the step/wall-clock caps also appear in the Defaults tab is discretionary; if it is cheap, prefer surfacing them, since v1.x already exposes `.planning/config.json` keys there.

### Deferred Ideas (OUT OF SCOPE)

- Fleet-level driving (one goal across multiple projects) — out of v2.0 scope; the global concurrency cap of 1 chosen above is the v2.0 posture, not a step toward fleet mode.
- Socket-based fast-path for injection — the durable file inbox already covers the requirement.
- Any model-assisted routing for states the rules do not cover — that is Phase 21's goal layer, and this phase deliberately parks instead.

### ⚠ Two locked decisions this research contradicts

CONTEXT.md's own header instructs: *"flag any that the research step contradicts rather than silently honouring them."* Two qualify. Neither is a reversal request — both are corrections of a factual premise, and the planner should decide.

1. **"A rate limit is detected from the `result`/error envelope."** It cannot be, as written. The rate-limit signal is a **separate top-level stream type** (`rate_limit_event`), not a field on the `result` envelope, and it is structurally unreachable from `derive_run_outcome_from_envelopes`, which takes `&[ResultMessage]` (`src/executor/outcome.rs:173-178`). The *principle* the decision encodes — read the structured transport, never the agent's prose (D-10) — is preserved exactly by reading `rate_limit_event` in the driver's drain loop. See **The Claude Rate-Limit Wire Protocol** below. Also note the decision's mention of "the same rule `executor/outcome.rs` already enforces… (CR-04 is exactly that bug)" from the Code Context section: CR-04 was about *discarding* envelopes, and adding a second envelope-discarding path is precisely what widening that signature would risk.

2. **"The observed-state hash is a stable hash over the same `ProjectState` value."** A hash is not needed and, implemented naively, would be *nondeterministic* — `ProjectState.phase_disk_statuses` is a `HashMap` (`src/state_reader/mod.rs:32`) with no defined iteration order, so an unsorted hash would falsify criterion 1's determinism claim in a way no single test run would reveal. `DiskDelta::between` already performs exactly the comparison the decision describes, including the unknown-vs-unchanged distinction the decision requires, by value equality (`src/executor/outcome.rs:105-129`). See **Don't Hand-Roll** and **Pitfall** notes below.

One further correction, smaller: the enumerated gate set names *"an execution checkpoint awaiting a decision"* inside execute-phase. `execute-phase.md` contains zero `AskUserQuestion` calls; the checkpoints live in generated plan files and in `execute-plan.md`. The gate is real, its stated location is not — see §4 of the GSD Runtime Contract.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **CTRL-06** | A run halts itself when it stops making progress, repeats a command, exceeds a step cap, or exceeds a wall-clock cap | **The Loop the Router Plugs Into** establishes that the iteration loop must be built (`src/driver/mod.rs:34-36`), gives the exact hoist boundary (`run.rs:1401`–`:1673`) and the per-run/per-iteration split table. **Don't Hand-Roll** supplies `DiskDelta::between` as the no-progress detector with unknown-vs-unchanged already correct. **Pitfall 3** identifies the 4h/4h cap collision that would make the wall-clock reason unreportable, and **Pitfall 4** the setup steps that must not be hoisted. |
| **CTRL-07** | A run parks rather than retrying when it hits a Claude subscription rate limit, and reports which quota window blocked it | **The Claude Rate-Limit Wire Protocol** gives the verbatim on-wire envelope from two committed captures, the full `rateLimitType` and `status` enums from the installed 2.1.235 binary, `resetsAt` as epoch seconds, the confirmed answer that the 5h/7d window **is** distinguishable, the exact plumbing gap (event never reaches `derive_run_outcome_from_envelopes`), the honest seam (`run.rs:1531`), and working classification code. Assumptions **A1** and **A2** carry the two residual risks with mitigations. |
| **DRIVE-02** | The driver chooses the next GSD command from observed project state using deterministic rules, not a model call, for every case the rules cover | **GSD Runtime Contract §1–§2** gives the verbatim `disk_status` derivation and GSD's own routing table, confirmed live against this repository. **§6** gives the safe output alphabet and the never-auto-select list. **Pitfalls 1 and 2** identify the two reader defects (`DiskStatus` has no `Executed`; `DiskInference` never reads verification status) that must be closed before any rule is written. **Pattern 3** gives the both-directions rule-table/test-table guard, and **Code Examples** the conformance-oracle shape that keeps the Rust table honest against GSD's without a runtime dependency. |
| **DRIVE-05** | The driver parks and flags for a human when it encounters a GSD gate that requires human judgement | **GSD Runtime Contract §3–§4** enumerates all twenty gates with exact file:line and exact condition text, including the six verification statuses, the four UAT/verification blocking sets verbatim from `uat-predicate.cjs:28-43`, the three `next.md` hard stops, `.continue-here.md` at both locations, and the milestone/cleanup prompts. **§5** resolves STATE.md's standing `WAITING.json` concern: the contract exists and is inert; do not model it. **§7** flags the `--research-phase` reading trap that would otherwise cause a spurious command-repeat halt. |
| **DRIVE-06** | The driver reports a terminal outcome — goal met, parked, or halted — with the reason | **The Loop** documents the existing terminal machinery: `terminal_label` reading the last `Parked` off the journal with the `parked:` prefix (`run.rs:417-463`), `outcome_label`'s nine `pub(crate)` values, and the `Observed`/`Decided` journal variants Phase 16 reserved for this phase with no migration (`journal/mod.rs:650-670`, `Decided.by` ∈ `policy`\|`llm`\|`human`). **Pattern 1** gives the reason-constant shape to mirror; **Pattern 2** makes the no-unclassified-arm requirement a compile-time property. **Open Question 4** proposes the deterministic goal-met predicate that satisfies criterion 5 without borrowing from Phase 21. |
</phase_requirements>

## Summary

Three things dominate the plan, and all three were established by reading the real files rather than by inference.

**First: Phase 20 must BUILD the iteration loop, not bound an existing one.** `DriveArgs::command` is a single `String`, and `src/driver/mod.rs:34-36`, `src/driver/mod.rs:98-102` and `src/cli.rs:51-55` each say in as many words that there is no loop and that Phase 20 owns the one that would exist. The `loop { tokio::select! { … } }` at `src/driver/run.rs:1518` is a *stream-event* loop inside one `claude` process, not a command loop. The plan's shape therefore depends on a structural change: an outer iteration loop wrapped around spawn→drain→outcome, with the lock, the journal and the terminate handler held across the whole run rather than re-established per iteration.

**Second: the router's rule table already exists, authoritatively, in GSD's own runtime — and this repo's state reader disagrees with it in one specific, dangerous way.** `gsd-tools query init.manager` computes `recommended_actions[]` with no model call; run live against this repository it returned `verify → /gsd-verify-work 19` and `plan → /gsd-plan-phase 20`. Its `disk_status` vocabulary has an `executed` state between `partial` and `complete`, and its `complete` requires `verification_status == 'passed'`. This repository's `DiskStatus` (`src/state_reader/disk_status.rs:4-14`) has no `Executed` variant, and its `Complete` (`:276-277`) means only *implementation complete* — GSD's `executed`. A router built on the current `DiskInference` as-is would step past a phase whose verification is `human_needed`, which is precisely the DRIVE-05 gate it exists to park at. `DiskInference` also never reads the VERIFICATION.md frontmatter `status`, and that field is the whole gate set.

**Third: the rate-limit signal is fully observable and the 5h/7d window IS distinguishable.** `rate_limit_event` is a top-level stream type carrying `rate_limit_info.rateLimitType` (`five_hour` | `seven_day` | `seven_day_opus` | `seven_day_sonnet` | `seven_day_overage_included`), `status` (`allowed` | `allowed_warning` | `rejected`) and `resetsAt` as unix epoch seconds. Two of these are already in this repository's committed golden transcripts. The gap is plumbing, not protocol: the event is forwarded to the driver as `ExecutionEvent::Message` and journaled, but it is never collected into the `Vec<ResultMessage>` that `derive_run_outcome_from_envelopes` consumes, so the outcome derivation can never see it. CONTEXT.md's "report the window as unknown" fallback remains correct for a malformed or absent payload — but it is the exception, not the expected case.

**Primary recommendation:** Build the outer iteration loop in `src/driver/run.rs` around the existing single-command body; derive the router's rule table from `gsd-tools query init.manager`'s logic and prove agreement with a golden-fixture conformance test rather than calling it at runtime; extend `DiskInference` with the VERIFICATION frontmatter status (one reader, D-11) before writing a single routing rule; observe `rate_limit_event` in the drain loop where it already arrives.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rule table: state → next GSD command | Pure library (`src/driver/router.rs`) | — | CONTEXT.md locks it as a pure function; determinism is a value property, not a filesystem experiment |
| Reading verification frontmatter status | State reader (`src/state_reader/disk_status.rs`) | — | D-11: one reader. The router receives it on `ProjectState`, never reads it |
| Iteration loop, bounds evaluation, terminal classification | Driver run body (`src/driver/run.rs`) | — | Owns the lock, the journal, the terminate handler and the executor handle |
| Rate-limit observation | Driver drain loop (`src/driver/run.rs:1531`) | Executor (`src/executor/claude.rs:1592-1621`) | The event is already forwarded there; the outcome path structurally cannot carry it |
| Terminal record + reason | Journal (`src/journal/`) + `run.json` | — | Existing durable carrier proven readable by a separate process (Phase 19) |
| Per-run cap overrides on argv | CLI (`src/cli.rs`) → `DriveArgs` (`src/driver/mod.rs`) | — | Existing seam; the TUI spawn path and a hand-typed invocation build the same value |
| Concurrency cap of 1 | Config (`src/config.rs:203-212`) enforced at spawn seam | — | Already shipped; Phase 20 confirms, does not rebuild |
| Preview of the chosen sequence | `src/driver/dry_run.rs` | — | `DryRunReport::commands` is already `Vec<String>`; only the pinned honesty text changes |

## Runtime State Inventory

> Phase 20 is not a rename/refactor phase, but it does change a **pinned on-disk and on-screen contract**, so the equivalent audit is recorded here.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `run.json` terminal record + `journal.jsonl` under `.planning/meta-manager/runs/<run-id>/`. `JournalEvent::Observed` and `JournalEvent::Decided` are **already schema'd** for this phase (`src/journal/mod.rs:650-670`) — no migration. New bounds/rate-limit reasons ride the existing `Parked { reason, needs }` variant (`:821-833`). | Code edit only — no data migration |
| Live service config | None. The driver contacts no external service that stores configuration. | None |
| OS-registered state | None new. The existing process-group + `flock` contract is unchanged by an outer loop, but the loop must not re-acquire the lock per iteration (see Pitfall 4). | None |
| Secrets / env vars | None. The envelope environment (`cred::build_env`) is established once per run and is unaffected by iteration count. | None |
| Build artifacts | None. Pure Rust, no generated code, no `egg-info` equivalent. | None |
| **Pinned contracts (this phase's real inventory item)** | `dry_run::SECTION_COMMANDS` (`src/driver/dry_run.rs:70-72`) states verbatim: *"The decision router is Phase 20, so the single --command argument below is the complete and honest sequence for this build — not a truncated one."* Phase 20 makes that sentence false. Its paired test is `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs`. Sibling prose making the same now-false claim: `src/driver/mod.rs:34-36`, `src/driver/mod.rs:98-102`, `src/cli.rs:51-55`, `src/driver/dry_run.rs:107-111`, and `src/driver/mod.rs:538-547` (`drive_args_carry_the_single_command_the_router_phase_will_replace`). | Update text **and** its paired test in the same commit (CONVENTIONS.md:75, TESTING.md:74) |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| *(none new)* | — | — | This phase adds **zero dependencies** |

**Installation:**

```bash
# No new crates. Verified against Cargo.toml [VERIFIED: Cargo.toml:15-49].
```

Everything Phase 20 needs is already in the tree:

| Existing facility | Location | What it gives Phase 20 |
|---|---|---|
| `tokio::select!` with `biased` | `src/driver/run.rs:1519-1520` | The arm-ordering discipline the outer loop must preserve |
| `tokio::time::Instant` / `interval` | `src/driver/run.rs:1442-1443` | Wall-clock deadline arithmetic without a new time crate |
| FNV-1a64 digest idiom | `src/journal/mod.rs:543-554` | A short greppable state identifier with the "this phase adds zero dependencies" rationale already written |
| `ProjectState: PartialEq` | `src/state_reader/mod.rs:13-14` | No-progress detection by value comparison, no hash needed |
| `DiskDelta::between` | `src/executor/outcome.rs:105-121` | Unknown-vs-unchanged already correct and tested |
| `Preferences::driver_max_concurrent` (default 1) | `src/config.rs:172-212` | OQ10's concurrency cap, already shipped |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Deriving the rule table in Rust | Shelling out to `gsd-tools query init.manager` per iteration | **Declined.** It is a second project-state reader (D-11), a blocking `Command` on the driver's async path (Phase 19's lint), and it puts a Node process inside the run loop. Use it as the *specification* and as a conformance oracle in tests — not at runtime. |
| A hand-rolled `Hash` over `ProjectState` | `PartialEq` on the retained previous `RunSnapshot` | **Prefer `PartialEq`.** `ProjectState.phase_disk_statuses` is a `HashMap` (`src/state_reader/mod.rs:32`); a hash that iterates it unsorted is nondeterministic and would make criterion 1's determinism claim literally false. |
| A new `BoundsReason` module | A sibling enum beside `ParkReason` in a new `src/driver/router.rs` or `src/driver/bounds.rs` | CONTEXT.md locks "sibling enum, do not extend `ParkReason`". Mirror the `REASON_*` const + `as_str()` shape at `src/envelope/policy.rs:34-53, 178-189`. |
| Adding a hashing crate (`blake3`, `sha2`) | FNV-1a64 inline | The digest is an identity comparison, not a security control — `src/journal/mod.rs:537-542` already argues this case and settled it. |

## Package Legitimacy Audit

**Not applicable — this phase installs no external packages.** Verified against `Cargo.toml` (`[VERIFIED: Cargo.toml:15-52]`): the `[dependencies]` block is unchanged by every recommendation in this document, and `[dev-dependencies]` is `assert_fs` + `tempfile` only.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## GSD Runtime Contract — Re-Verified 2026-08-19

> This section answers STATE.md's standing open concern: *"Phase 13's queue-execution design self-dated 'valid until 2026-04-30'; re-verify GSD's autonomous-mode / checkpoint contract during Phase 20 research."*

**Installed GSD version: `1.10.0`** `[VERIFIED: ~/.claude/gsd-core/VERSION]`
**Installed runtime marker: `claude`** `[VERIFIED: ~/.claude/gsd-core/.gsd-runtime]`

### 1. The D-R-P-E-V state machine as it exists today

`disk_status` is derived from artifact presence in `~/.claude/gsd-core/bin/lib/init.cjs:1875-1888`, verbatim:

```js
if (completion.phase_complete)        diskStatus = 'complete';
else if (completion.implementation_complete) diskStatus = 'executed';
else if (summaryCount > 0)            diskStatus = 'partial';
else if (planCount > 0)               diskStatus = 'planned';
else if (hasResearch)                 diskStatus = 'researched';
else if (hasContext)                  diskStatus = 'discussed';
else                                  diskStatus = 'empty';
```

with `'no_directory'` as the initial value at `:1855` `[VERIFIED: ~/.claude/gsd-core/bin/lib/init.cjs:1855-1888]`.

`phase_complete` is **implementation complete AND verification passed** `[VERIFIED: ~/.claude/gsd-core/bin/lib/init.cjs:194-195]`:

```js
const verificationPassed = projectedVerificationStatus === 'passed';
const phaseComplete = implementationComplete && verificationPassed;
```

`implementation_complete` is `planCount > 0 && summaryCount >= planCount` `[VERIFIED: ~/.claude/gsd-core/bin/lib/init.cjs:181]`.

### 2. The routing table — GSD's own, computed with no model call

`~/.claude/gsd-core/bin/lib/init.cjs:2022-2079`, verbatim structure:

| `disk_status` | Extra condition | `action` | `command` |
|---|---|---|---|
| `complete` | — | *(none — `continue`)* | — |
| `executed` | — | `verify` | `phase['verification_next_command']` |
| `planned` | `deps_satisfied` | `execute` | `/gsd-execute-phase <N>` |
| `discussed` \| `researched` | — | `plan` | `/gsd-plan-phase <N>` |
| `empty` \| `no_directory` | `is_next_to_discuss` (i.e. `deps_satisfied`) | `discuss` | `/gsd-discuss-phase <N>` |
| `partial` | — | *(no action emitted)* | — |

`[VERIFIED: ~/.claude/gsd-core/bin/lib/init.cjs:2022-2079]`

A dependency-collision filter then drops an `execute` action while any phase is `partial`-or-active-`planned`, and a `plan` action while any phase is active-`discussed`/`researched`, when a dependency relationship exists `[VERIFIED: ~/.claude/gsd-core/bin/lib/init.cjs:2067-2079]`.

**Live confirmation against this repository** (`node ~/.claude/gsd-core/bin/gsd-tools.cjs query init.manager`, 2026-08-19):

```json
"recommended_actions": [
  {"phase":"19","action":"verify","reason":"Implementation complete; verification human_needed","command":"/gsd-verify-work 19"},
  {"phase":"20","action":"plan","reason":"Context gathered, ready for planning","command":"/gsd-plan-phase 20"}
]
```

`[VERIFIED: gsd-tools query init.manager, executed 2026-08-19]`

**Note the verb split:** `query init.progress` and `query init.manager` are **different commands with different shapes**. `init.progress` emits `status` (`complete` | `executed` | `pending` | `not_started`) and carries **no** `recommended_actions` and **no** `waiting_signal`. Only `init.manager` carries the router table. `[VERIFIED: both verbs executed 2026-08-19]`

### 3. The verification routing table — the gate set's ground truth

`~/.claude/gsd-core/bin/lib/verification.cjs:72-113`, `VERIFICATION_ROUTING_TABLE`, verbatim keys and `next_command` values:

| `status` | `next_command` (bare) | Emitted by verifier? | Router disposition |
|---|---|---|---|
| `passed` | `''` | yes | **PROCEED** |
| `gaps_found` | `plan-phase <N> --gaps` (built at call time, `:335-342`) | yes | **PARK** (CONTEXT.md OQ7: always park) |
| `human_needed` | `verify-work <N>` | yes | **PARK** — definitionally |
| `stale` | `verify-work <N>` (built at call time, `:344-351`) | no — internal | **PARK** |
| `missing` | `execute-phase <N>` | no — internal sentinel | **PARK** |
| `unknown` | `execute-phase <N>` | no — internal sentinel | **PARK** |

`[VERIFIED: ~/.claude/gsd-core/bin/lib/verification.cjs:72-113, 282-381]`

The verifier emits only three values: `VERIFIER_STATUSES = ['passed', 'gaps_found', 'human_needed']` `[VERIFIED: ~/.claude/gsd-core/bin/lib/verification.cjs:50]`. `missing` and `unknown` are constructed internally and are excluded from raw-status lookup `[VERIFIED: verification.cjs:360-364]`.

**Precedence, verbatim from the source:** `gaps_found` is checked **before** the staleness check and short-circuits it (`:333-342`, comment: *"gaps_found takes priority over stale — gap closure is the correct next step regardless of whether summaries are newer than the verification file"*). `stale` is checked before ordinary table routing (`:343-351`).

**A seventh state the router must handle: `staleCheckIndeterminate`.** The staleness check can itself fail (fs / scan / clock error), in which case routing proceeds *as if nothing were stale* but the result carries `staleCheckIndeterminate: true`, surfaced by init as `verification_stale_check_indeterminate` `[VERIFIED: verification.cjs:343-357, 370, 379; init.cjs:204-208]`. The source comment is explicit that this is *"checked; nothing is stale"* versus *"could not check"*. CONTEXT.md's "refuse to act on inferred-only state" makes this a **PARK**, not a proceed.

### 4. The complete human-judgement gate set — enumerated from the real workflow files

| # | Gate | Exact condition | File:line |
|---|---|---|---|
| G1 | Verification `human_needed` | `verification.status` frontmatter | `verification.cjs:84-93`; `autonomous.md:463-469` |
| G2 | Verification `gaps_found` | frontmatter, precedence over stale | `verification.cjs:78-83, 335-342` |
| G3 | Verification `stale` | a `*-SUMMARY.md` newer than `*-VERIFICATION.md` | `verification.cjs:94-98, 344-351`; `autonomous.md:461` → `handle_blocker` |
| G4 | Verification `missing` | no `*-VERIFICATION.md`, or no parseable frontmatter `status` | `verification.cjs:99-105, 309-311, 330-332` |
| G5 | Verification `unknown` | frontmatter `status` not in `VERIFIER_STATUSES` | `verification.cjs:106-112, 373-380` |
| G6 | Stale-check indeterminate | `verification_stale_check_indeterminate == true` | `verification.cjs:357`; `init.cjs:204-208` |
| G7 | Outstanding UAT | UAT frontmatter `status` ∈ `partial, diagnosed, pending, blocked, in_progress, failed` | `uat-predicate.cjs:30-32` |
| G8 | UAT item result | `result` ∈ `pending, blocked, failed`; passing only `passed, pass` | `uat-predicate.cjs:34, 43` |
| G9 | Blocking VERIFICATION frontmatter | `status` ∈ `human_needed, gaps_found, pending, blocked, partial, failed, in_progress` | `uat-predicate.cjs:38-41` |
| G10 | `.planning/.continue-here.md` exists | hard stop, `--force` only bypass | `next.md:46-58` |
| G11 | STATE.md `status: error` \| `status: failed` | hard stop | `next.md:60-69` |
| G12 | VERIFICATION.md unresolved `FAIL` items without overrides | hard stop | `next.md:71-84` |
| G13 | Phase-dir `.continue-here.md` with `severity: blocking` rows | *"stop and ask the user for clarification"* | `execute-phase.md:217-235`; `discuss-phase.md:162-177` |
| G14 | Incomplete prior phase (`plans.length > summaries.length`) | Route 0 resume, then the 3-option `[C]/[S]/[F]` prompt, default **Stop** | `next.md:88-...` (`resume_incomplete_phase`, `prior_phase_completeness`) |
| G15 | `verification_deferred_human` | STATE.md Deferred Verification table row → `handle_blocker` | `autonomous.md:478-481` |
| G16 | Milestone audit `gaps_found` | `AskUserQuestion`: "Continue anyway" / "Stop" | `autonomous.md:691-704` |
| G17 | Milestone audit `tech_debt` | `AskUserQuestion`: "Continue with tech debt" / "Stop" | `autonomous.md:706-716` |
| G18 | Any `handle_blocker` entry | 3-option `AskUserQuestion` | `autonomous.md:761-769` |
| G19 | Cleanup confirmation | `AskUserQuestion`: "Proceed with archiving and pruning?" | `cleanup.md:126` |
| G20 | Milestone merge strategy | `AskUserQuestion`: squash / merge / delete / keep | `complete-milestone.md:641` |

`[VERIFIED: each file:line opened and read this session]`

**One correction to a CONTEXT.md assumption.** CONTEXT.md's gate set includes *"an execution checkpoint awaiting a decision"* inside execute-phase. `execute-phase.md` contains **zero** `AskUserQuestion` occurrences `[VERIFIED: grep -c AskUserQuestion ~/.claude/gsd-core/workflows/execute-phase.md → 0]`. The checkpoints exist in generated **plan files** (`checkpoint:human-verify` tasks) and in `execute-plan.md`, not in the phase workflow. The gate is real; its location in CONTEXT.md is wrong. Plan against G13 and G18 instead.

### 5. WAITING.json — the contract still exists and is INERT

`state signal-waiting` / `state signal-resume` are live CLI verbs `[VERIFIED: ~/.claude/gsd-core/bin/gsd-tools.cjs:18-19]`. The written schema, verbatim `[VERIFIED: ~/.claude/gsd-core/bin/lib/state.cjs:2296-2314]`:

```js
const signal = {
    status: 'waiting',
    type: type || 'decision_point',
    question: question || null,
    options: options ? options.split('|').map(o => o.trim()) : [],
    since: clock.nowIso(),
    phase: phase || null,
};
```

It is written to `.gsd/WAITING.json` if `.gsd/` exists, else `.planning/WAITING.json` `[VERIFIED: state.cjs:2297-2298]`. `init.manager` reads it back as `waiting_signal` `[VERIFIED: init.cjs:2011-2021, 2110]`.

**Two facts that kill it as a driver signal:**

1. **Nothing writes it.** A grep for `signal-waiting`, `signal-resume` and `WAITING` across `~/.claude/gsd-core/workflows/`, `~/.claude/gsd-core/references/` and `~/.claude/skills/gsd-*/` returns **zero hits** `[VERIFIED: grep executed 2026-08-19]`. Every occurrence in the runtime is inside `gsd-tools.cjs`, `init.cjs` and `state.cjs` themselves. A driver that waits on `WAITING.json` waits forever.
2. **The writer and the reader disagree about location.** The writer prefers `.gsd/WAITING.json` when `.gsd/` exists; `init.manager` hardcodes `.planning/WAITING.json` (`init.cjs:2013`). Even if a workflow started writing it, a project with a `.gsd/` directory would file the signal where the only reader never looks. *(Note: this repository has a `.gsd/` directory — untracked, present in `git status` — so it would be affected.)*

**What replaced it:** nothing single-purpose. The observable substitute is the composite of §4's gate set, read through `query verification.status`, `query audit-uat`, and file-existence checks for `.continue-here.md`. **Recommendation: do not model `WAITING.json` at all in Phase 20.** Reading an always-absent file to decide nothing is a rule with no state behind it, and CONTEXT.md's "a state the rules do not cover parks" already covers the case where GSD revives it.

### 6. The router's output alphabet — safe and forbidden

**Safe to auto-select** (each is the `command` GSD's own router emits, or its direct read-only analogue):

| Command | Selected when | Source |
|---|---|---|
| `/gsd-discuss-phase <N>` | `empty` \| `no_directory`, deps satisfied | `init.cjs:2056-2064` |
| `/gsd-plan-phase <N>` | `discussed` \| `researched` | `init.cjs:2046-2054` |
| `/gsd-execute-phase <N>` | `planned`, deps satisfied | `init.cjs:2037-2045` |
| `/gsd-verify-work <N>` | `executed` with verification `human_needed` (as the *unpark* command a human runs, **not** as an auto-selected step) | `verification.cjs:92`; `init.cjs:2028-2036` |

**Never auto-select:**

| Command | Why |
|---|---|
| `/gsd-complete-milestone` | Ends in an interactive merge-strategy prompt (G20) and rewrites ROADMAP.md/PROJECT.md by judgement |
| `/gsd-cleanup` | Archives and prunes branches behind a confirmation (G19) |
| `/gsd-audit-milestone` | Its outcomes are themselves gates (G16, G17) |
| `/gsd-autonomous` | Recursive self-invocation. The driver would be driving a driver |
| `/gsd-progress`, `/gsd-next` | Routers themselves. Selecting a router from a router is how a run re-selects the same command forever — and it is exactly what CONTEXT.md forbids as a default fallback |
| `/gsd-undo`, `/gsd-quick`, `/gsd-fast`, `/gsd-debug` | Unbounded scope; no deterministic state predicate selects them |
| `/gsd-ship`, `/gsd-pr-branch` | Push/PR side effects belong to the Phase 19 envelope's explicit paths, not to a router branch |

### 7. `--research-phase` — a reading trap this phase inherits

ROADMAP's Phase 20 entry says `Research: yes — /gsd-plan-phase --research-phase`. STATE.md:112-116 records that this flag is research-**only** mode: it exits before the planner runs and produces no plans, and that the same wording appears on Phases 20 and 22. `[VERIFIED: .planning/STATE.md:112-116]` The router must not treat `--research-phase` as a plan-producing command; a rule that selected it would satisfy no `has_plans` postcondition and would re-select forever on the next iteration — tripping the command-repeat detector for a reason that is a bug rather than a stall.

## The Claude Rate-Limit Wire Protocol (CTRL-07)

**CLI on this machine: `2.1.235`** `[VERIFIED: claude --version, /home/blk/.local/share/claude/versions/2.1.235]`. Note the drift: this repository's committed golden transcripts and every code comment reference **2.1.220**. Fifteen patch versions have shipped. No quota was burned reproducing a limit — every fact below comes from committed fixtures and from strings extracted from the installed binary.

### The envelope, verbatim from a committed real capture

`[VERIFIED: tests/fixtures/transcripts/01-success-textonly.ndjson]`

```json
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1785327000,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"out_of_credits","isUsingOverage":false},"uuid":"11111111-1111-4111-8111-000000000002","session_id":"00000000-0000-4000-8000-000000000001"}
```

A second shape, also from a committed capture `[VERIFIED: tests/fixtures/transcripts/*.ndjson, extracted 2026-08-19]`:

```json
"rate_limit_info":{"status":"allowed_warning","resetsAt":1785859200,"rateLimitType":"seven_day","utilization":0.25,"isUsingOverage":false}
```

### Field inventory, from the installed 2.1.235 binary

Adjacent string-table run at offset 308456-308470 of the binary, in emission order `[VERIFIED: strings(2.1.235):308452-308470]`:

```
includeOverageInUse, resetsAt, rateLimitType, utilization, overageStatus,
overageResetsAt, overageDisabledReason, isUsingOverage, overageInUse,
surpassedThreshold?, rateLimitGraceActive, overagePeriodMonthly,
overagePeriodChannel, canUserPurchaseCredits, hasChargeableSavedPaymentMethod,
rate_limit_event, rate_limit_info
```

Two further strings from the same table: `org_spend_cap_reached`, `org_level_disabled_until` `[VERIFIED: strings(2.1.235):308449-308450]`.

### The enums

**`rate_limit_info.rateLimitType`** — appears as a five-member set in six separate switch tables `[VERIFIED: strings(2.1.235):310638-310642, 310675-310679, 310680-310684, 310710-310714, 310731-310736]`:

```
five_hour
seven_day
seven_day_opus
seven_day_sonnet
seven_day_overage_included
```

**`rate_limit_info.status`** — three values `[VERIFIED: strings(2.1.235):310627, 310631, 310647 and the transcript-renderer at 322950-322960]`:

```
allowed          (observed in fixture 01)
allowed_warning  (observed in a second fixture)
rejected         (the binary's transcript renderer prints "rate_limit: rejected (" then rateLimitType then resetsAt)
```

The renderer's field order at `strings(2.1.235):322950-322960` is verbatim:

```
rejected
status
rate_limit: rejected (
rateLimitType
resetsAt
 resets_at=
```

**`resetsAt` format:** unix epoch **seconds** — `1785327000` and `1785859200` in the fixtures are 10-digit values, and the delta between the `five_hour` and `seven_day` samples is 532200 s ≈ 6.16 days, consistent with two independent windows rather than one. `[VERIFIED: tests/fixtures/transcripts/*.ndjson]`

**Print-mode emission path exists:** `[print] rate_limit listener failed: ` `[VERIFIED: strings(2.1.235):381945]` — headless `-p` subscribes to a rate-limit listener, which is what publishes `rate_limit_event` onto the stream.

### Answer to the research question

**Is the 5-hour vs 7-day window distinguishable? YES, unambiguously**, via `rate_limit_info.rateLimitType`. **Is a reset timestamp provided? YES**, as `resetsAt` (unix epoch seconds), with `overageResetsAt` as a separate field for the overage window.

CONTEXT.md's commitment to report "window unknown" rather than guessing remains the correct fallback — but it applies only when `rate_limit_info` is absent, is not an object, or carries a `rateLimitType` outside the five known values. It is the exception arm, not the expected outcome. Match `rateLimitType` and `status` as `&str` with an explicit fallback arm carrying the observed string verbatim, following the posture at `src/executor/stream_json.rs:14-18` and `src/executor/outcome.rs:203-208` — this CLI ships new enum values on a patch line, and `seven_day_opus` / `seven_day_sonnet` / `seven_day_overage_included` are three that a 2.1.220-era typed enum would already have lost.

### The plumbing gap — where the signal currently dies

| Step | Location | Status |
|---|---|---|
| Parsed off the wire | `StreamMessage::RateLimitEvent(serde_json::Value)` | ✅ exists, carried unmodelled `[VERIFIED: src/executor/stream_json.rs:53-57]` |
| Forwarded to the driver | grouped with `System`/`Assistant`/`User` into `ExecutionEvent::Message(Box<StreamMessage>)` | ✅ exists `[VERIFIED: src/executor/claude.rs:1592-1621]` |
| Journaled | kind `"rate_limit_event"`, rendered as `format!("rate_limit_event: {value}")` | ✅ exists `[VERIFIED: src/journal/mod.rs:1189, 1250]` |
| Reaches the outcome derivation | **NO** | ❌ `envelopes: Vec<ResultMessage>` (`src/executor/claude.rs:1086`) is pushed only at `StreamMessage::Result` (`:1627`); `derive_run_outcome_from_envelopes` takes `&[ResultMessage]` (`src/executor/outcome.rs:173-178`) |

**Consequence for the plan:** the rate-limit park **cannot** be derived inside `derive_run_outcome_from_envelopes`, and attempting it would require widening that function's signature — the one entry point Phase 15's CR-04 gap-closure deliberately narrowed to *"the full `result` envelopes"* precisely so no envelope-discarding sibling path could exist. The honest seam is the driver's own drain loop at `src/driver/run.rs:1531`, which already receives every `ExecutionEvent` and already inspects them (`turn_boundary` at `:1534`). Observe `ExecutionEvent::Message(StreamMessage::RateLimitEvent(v))` there, retain the latest `rate_limit_info`, and let the iteration loop's post-outcome classification read it. This mirrors exactly how `PendingAcks` correlation is done — parsed protocol read in the driver because the driver is the party D-08 makes responsible.

Also worth carrying forward: **an `allowed_warning` is not a park.** Fixture 01 carries `status: "allowed"` on a completely successful run, and `allowed_warning` at 25% utilization is informational. Only `rejected` parks. A rule keyed on the *presence* of a `rate_limit_event` would park every healthy run.

## The Loop the Router Plugs Into

### Answer: Phase 20 must BUILD the iteration loop

Three independent, explicit statements in the tree say so:

> **4. This phase runs exactly one GSD command, supplied on the command line.** There is no loop and no sequence. The decision router is Phase 20's, which is why [`DriveArgs::command`] is a single `String` and not a `Vec`.
> `[VERIFIED: src/driver/mod.rs:34-36]`

> **Exactly one, and that is the whole of this phase's execution model.** Phase 20 owns the decision router that turns a goal into a sequence; a field that accepted a sequence now would imply a loop that does not exist.
> `[VERIFIED: src/driver/mod.rs:98-101]`

> `// Exactly one command. The decision router is Phase 20's, so there is` `// deliberately no way to pass a sequence.`
> `[VERIFIED: src/cli.rs:51-52]`

And a test that will fail the moment the field changes shape:

```rust
fn drive_args_carry_the_single_command_the_router_phase_will_replace() {
    let args = args("demo");
    // A `String`, never a `Vec<String>`: this phase issues exactly one GSD
    // command and Phase 20 owns the router that would issue a sequence.
    assert_eq!(args.command, "/gsd-progress");
```

`[VERIFIED: src/driver/mod.rs:538-547]`

### What `execute_run` does today, in order

`[VERIFIED: src/driver/run.rs:1152-1701]`

| Step | Line | Per-run or per-iteration under Phase 20? |
|---|---|---|
| Install SIGTERM handler | `:1177-1184` | **per-run** — must not be re-installed |
| `establish_own_group()` | `:1186` | **per-run** |
| Read `run_id` | `:1203` | **per-run** |
| `establish_envelope` in `spawn_blocking` | `:1225-1238` | **per-run** — four files + a network-free probe; re-running per iteration would re-probe and re-write |
| Build `ExecutionOptions` | `:1241-1246` | **per-iteration** (`session_id`, and the command changes) |
| `argv_digest` | `:1253-1256` | per-iteration if the digest is to stay honest; `run.json` writes it once (`:513`) |
| `lock::acquire` in `spawn_blocking` | `:1295-1301` | **per-run** — held for the run's duration (D-20.2) |
| `JournalRun::start` in `spawn_blocking` | `:1312-1319` | **per-run** — `run.json` is written exactly twice (`:512`) |
| Envelope notice diagnostic | `:1336-1344` | **per-run** |
| `executor.start(...)` raced against terminate | `:1401-1413` | **per-iteration** |
| Drain loop `loop { select! { … } }` | `:1518-1646` | **per-iteration** — this is a *stream-event* loop, not a command loop |
| `handle.wait_outcome()` | `:1673` | **per-iteration** |
| `terminal_label` + `journal.finish` | `:1678-1698` | **per-run** — exactly one terminal record |

**The smallest honest change:** hoist lines `:1401`–`:1673` into a bounded outer loop, leaving everything above `:1401` and everything below `:1673` untouched. Concretely:

```
per-run setup  (:1177 .. :1351, unchanged)
loop {                              // NEW — the iteration loop
    observe    -> RunSnapshot::capture on spawn_blocking
    route      -> router::next_command(&project_state) -> Decision
    bound      -> evaluate detectors in fixed order; break on first hit
    journal    -> Observed { phase, drpev } ; Decided { by:"policy", command, rationale }
    execute    -> executor.start(...) .. drain .. wait_outcome    (:1401 .. :1673)
    classify   -> RunOutcome + latest rate_limit_info -> continue | park | halt
}
terminal record  (:1678 .. :1698, one call, carrying the classification)
```

### What `run.json` and the journal already record about termination

`RunRecord` `[VERIFIED: src/journal/mod.rs — constructed at src/driver/run.rs:482-518]`: `run_id, goal, gsd_command, target, opt_in, started_at, session_id, pid, pgid, claude_code_version, argv_digest, ended_at, outcome`.

`gsd_command` is a single `String` (`:494`) and `run.json` is written **exactly twice** (`:512`). Under a multi-command loop, `gsd_command` becomes either the *first* command or the goal; the per-iteration sequence belongs in the journal's `Decided` events, not in a widened `run.json` field.

`terminal_label` `[VERIFIED: src/driver/run.rs:446-463]` returns `outcome_label(outcome)` **unless** the journal's last `parked` record exists, in which case `format!("{PARKED_LABEL_PREFIX}{reason}")` with `PARKED_LABEL_PREFIX = "parked:"` (`:422`). **The LAST park wins** (`:433-436`). This is exactly the carrier CONTEXT.md wants reused — a bounds halt writes a `Parked`-shaped record with a sibling reason and `terminal_label` picks it up with no change.

`outcome_label`'s nine values `[VERIFIED: src/driver/run.rs:403-415]`: `succeeded_with_changes, succeeded_no_changes, failed, permission_denied, killed, timed_out, stalled, capability_refused, spawn_failed`. `driver::run::outcome_label` is `pub(crate)` so the render layer proves against the string the driver writes (`:399-402`) — a new terminal vocabulary must go through it or through the `parked:` prefix, never as a third string source.

### The schema Phase 16 already reserved for this phase

```rust
/// What the driver observed about a project's state before deciding.
///
/// **Schema only in this phase — Phase 20 emits it.** It is present now so
/// that phase adds no schema migration (D-36).
Observed {
    /// The phase number the observation is about.
    phase: String,
    /// The five D-R-P-E-V stage statuses, in order.
    drpev: Vec<String>,
},
/// What the driver decided to run next, and why.
///
/// **Schema only in this phase — Phase 20 emits it** (D-36).
Decided {
    /// `"policy"`, `"llm"`, or `"human"`.
    by: String,
    /// The GSD command the decision selected.
    command: String,
    /// Why this command, in one line.
    rationale: String,
},
```

`[VERIFIED: src/journal/mod.rs:650-670]`

**`Decided.by` is a three-value vocabulary: `"policy"`, `"llm"`, `"human"`.** Phase 20 emits `"policy"` and nothing else; `"llm"` is Phase 21's. Do not mint a fourth value.

`Observed.drpev` is documented as *"the five D-R-P-E-V stage statuses, in order"* — a `Vec<String>` of length 5, not a free-form list. The five stages map onto GSD's artifact presence: Discuss (`has_context`), Research (`has_research`), Plan (`plan_count`), Execute (`summary_count`), Verify (verification status).

## Architecture Patterns

### System Architecture Diagram

```
                     TUI process                       detached driver process
                  (gsd-meta-manager)                    (gsd-meta-manager drive)
                          │                                       │
       user presses "drive"                     SIGTERM ──────────┤ (raced first, biased)
                          │                                       │
                          ▼                                       ▼
              spawn.rs: argv + pgid                    ┌── PER-RUN SETUP (once) ─────────┐
                          │                            │ signal handler → own pgroup     │
                          └──── argv ─────────────────▶│ establish_envelope (blocking)   │
                                                       │ lock::acquire   (blocking)      │
                                                       │ JournalRun::start (blocking)    │
                                                       └────────────────┬────────────────┘
                                                                        │
                              ┌─────────────────────────────────────────▼──────────────┐
                              │            ITERATION LOOP  ◀── NEW IN PHASE 20          │
                              │                                                         │
   .planning/ on disk ───────▶│  1. OBSERVE   RunSnapshot::capture (spawn_blocking)      │
   (single reader:            │               → head_sha, dirty, ProjectState            │
    parse_project_state)      │                        │                                │
                              │                        ▼                                │
                              │  2. BOUND     detectors, fixed order, first wins:        │
                              │               a. wall-clock cap breached?  ─────┐        │
                              │               b. step cap reached?          ────┤        │
                              │               c. state unchanged 2x?         ───┤ HALT   │
                              │               d. same command 2x?           ────┘        │
                              │                        │ none fired                      │
                              │                        ▼                                │
                              │  3. ROUTE     router::decide(&ProjectState) -> Decision  │
                              │               pure fn, no I/O, no model call             │
                              │                 ├─ Park(gate)      ──────────▶ PARK      │
                              │                 ├─ GoalMet         ──────────▶ GOAL-MET  │
                              │                 ├─ NoRule(state)   ──────────▶ PARK      │
                              │                 └─ Run(command)                          │
                              │                        │                                │
                              │                        ▼                                │
                              │  4. JOURNAL   Observed{phase,drpev} + Decided{policy,..} │
                              │                        │                                │
                              │                        ▼                                │
                              │  5. EXECUTE   executor.start(cmd) ──▶ claude -p          │
                              │               drain stream-json events:                  │
                              │                 · TurnCompleted  → inbox drain           │
                              │                 · RateLimitEvent → retain rate_limit_info│
                              │                 · replay echo    → InterjectionActedOn   │
                              │                        │                                │
                              │                        ▼                                │
                              │  6. CLASSIFY  wait_outcome() + retained rate_limit_info  │
                              │                 ├─ rejected quota ─────────▶ PARK        │
                              │                 ├─ Failed/Denied  ─────────▶ HALT        │
                              │                 └─ Succeeded      ── loop back to 1      │
                              └─────────────────────────┬───────────────────────────────┘
                                                        │
                                                        ▼
                                          ┌── TERMINAL (exactly one) ──┐
                                          │ journal Parked{reason,..}  │
                                          │ terminal_label() reads it  │
                                          │ journal.finish(label)      │
                                          │ run.json write two of two  │
                                          └────────────┬───────────────┘
                                                       │
   TUI reads back ◀────── journal.jsonl / run.json ────┘   (filesystem-mediated, never a socket)
```

### Recommended Project Structure

```
src/driver/
├── mod.rs         # DriveArgs gains cap overrides; the single-command doc changes
├── router.rs      # NEW — pure decision function + rule table (CONTEXT.md locks this path)
├── bounds.rs      # NEW (or fold into router.rs) — detectors + BoundsReason sibling enum
├── run.rs         # execute_run gains the outer iteration loop
└── dry_run.rs     # SECTION_COMMANDS honesty text updated; build_report renders the sequence

src/state_reader/
└── disk_status.rs # DiskInference gains verification frontmatter status (the ONE reader, D-11)
```

### Pattern 1: Pure classification function with a stable snake_case reason per arm

**What:** A `&self`-free function over an owned value returning a typed verdict; each variant maps through `as_str()` to a `const &str` that a later reader greps for.
**When to use:** Every routing and bounds decision in this phase.
**Example** (the established shape to mirror, verbatim):

```rust
// Source: src/envelope/policy.rs:34-53, 155-189
pub const REASON_PUSH_OUTSIDE_NAMESPACE: &str = "push_outside_namespace";
pub const REASON_FORCE_PUSH_BLOCKED: &str = "force_push_blocked";
pub const REASON_HOOK_BYPASS_BLOCKED: &str = "hook_bypass_blocked";
pub const REASON_SECRET_DETECTED: &str = "secret_detected";
pub const REASON_PR_CAP_EXCEEDED: &str = "pr_cap_exceeded";
pub const REASON_CREDENTIAL_UNAVAILABLE: &str = "credential_unavailable";
pub const REASON_ENVELOPE_ASSERTION_FAILED: &str = "envelope_assertion_failed";

pub enum ParkReason {
    PushOutsideNamespace,
    ForcePushBlocked,
    HookBypassBlocked,
    SecretDetected,
    PrCapExceeded,
    CredentialUnavailable,
    EnvelopeAssertionFailed,
}

impl ParkReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParkReason::PushOutsideNamespace => REASON_PUSH_OUTSIDE_NAMESPACE,
            // … one arm per variant, no wildcard
        }
    }
}
```

CONTEXT.md locks: **these seven arms stay untouched; the router/bounds reasons live in a sibling enum.** Both flow through `JournalEvent::Parked { reason, needs }` and through `terminal_label`, so one grep answers "why did this run end".

### Pattern 2: Exhaustive match, no wildcard, as a compile-time gate

**What:** Spell every arm out, joining identical bodies with `|`, so an upstream variant addition is a compile error rather than a silent fallthrough.
**When to use:** The router's rule table, the bounds detector ordering, and the terminal classification.
**Why it is house style:** `DriveError::source()` at `src/error.rs:530-547` does this deliberately, and `RunVerdict` at `src/driver/reconcile.rs:284-286` repeats it `[CITED: .planning/codebase/CONVENTIONS.md:51-53]`. Criterion 5's *"never as an unclassified 'loop ended'"* is a type-level requirement — if the terminal enum has no unclassified arm and every match on it is exhaustive, the failure mode is unrepresentable.

### Pattern 3: A rule table and its test table derived from one another

**What:** CONTEXT.md requires that "a rule with no row, or a row with no rule, should fail the build."
**How, following this repo's own guard mechanism:** the `SPAWN_ALLOWLIST` shape at `tests/spawn_seam_guard.rs` fails in **both** directions — an unlisted item is a violation, and a listed item that no longer matches anything is *also* a violation (*"the allowlist is now wider than the truth it describes"*) `[CITED: .planning/codebase/TESTING.md:64]`. Apply the identical shape: declare the rule table as a `const` slice of `(observed-state, expected-command)` pairs, drive both the router and the test from it, and add a companion test that fails if any router arm is unreachable from the table or any table row routes to an arm that does not exist.

**Add a non-vacuity assertion and a control arm** — TESTING.md:70 and :113 require both: assert the scan examined *something*, and prove the matcher fires on a synthetic bad example and spares a synthetic good one.

### Pattern 4: `biased` select with the terminate arm first

**What:** Preserve `biased;` with `term.recv()` as arm one at every `select!` the outer loop introduces.
**Why:** `src/driver/run.rs:1478-1494` explains at length that without `biased` the macro picks a ready arm at random, and *"a stop that loses to a busy event queue is a stop the user experiences as ignored (D-06.1)"*. `src/driver/run.rs:1387-1400` makes the sharper point for the startup path: there the cost of losing is a **swallowed** stop, which is CR-01. Any new await introduced between iterations — the snapshot capture, the router call, a journal write — is a new window where a SIGTERM can arrive.

### Anti-Patterns to Avoid

- **Calling `gsd-tools query init.manager` from the run loop.** It is a parallel project-state reader (D-11), a blocking `Command` on an `async fn` path (Phase 19's lint), and it puts a Node process inside a loop whose whole value is surviving its parent. Use it as a **test-time oracle** against fixture projects instead.
- **Re-acquiring the lock or restarting the journal per iteration.** `RunLock` has **no `Drop` impl** and its descriptor *is* the lock (`src/driver/run.rs:1278-1286`); dropping or re-acquiring it opens the concurrency window CTRL-05 exists to close. `run.json` is written exactly twice (`:512`).
- **Hashing `ProjectState` by iterating `phase_disk_statuses` unsorted.** `HashMap` iteration order is nondeterministic; criterion 1's determinism claim would be false in a way no single test run would reveal.
- **Defaulting an uncovered state to `/gsd-progress`.** CONTEXT.md forbids it explicitly, and `/gsd-progress` is itself a router — selecting it guarantees the same-command-twice detector fires for a reason that is a bug.
- **Treating any `rate_limit_event` as a park.** Fixture 01 carries one on a fully successful run with `status: "allowed"`. Only `rejected` parks.
- **Parking on `DiskStatus::Complete` as "phase done".** In this repository that value means *implementation complete*, not verified. See Pitfall 1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| "Did anything change since last iteration?" | A field-by-field `ProjectState` differ, or a hand-rolled hash | `DiskDelta::between(&before, &after)` | Already built, already handles unknown-vs-unchanged correctly (`src/executor/outcome.rs:105-121`), and D-11 says outright *"reusing the existing reader rather than hand-rolling a field-by-field diff is the whole point"* (`:92-94`) |
| Unknown vs unchanged for git state | `head_sha.unwrap_or_default()` | The existing `match (Some, Some) => …, _ => false` arms | `src/executor/outcome.rs:106-114`: unknown on either side is *inconclusive, so the signal stays quiet*. An unknown fingerprint is not evidence of no progress — CONTEXT.md requires exactly this and it is already correct |
| A short stable identifier for a state | A new hashing crate | `journal::argv_digest`'s FNV-1a64 shape | `src/journal/mod.rs:532-554` already argues the "identity, not security; zero dependencies" case and settled it |
| Reading `.planning/` state | A second reader for the router | `state_reader::parse_project_state` | D-11, locked. The router takes `ProjectState` by value and does no I/O |
| The GSD routing rules | Inventing a rule table from the README | The verbatim table at `~/.claude/gsd-core/bin/lib/init.cjs:2022-2079` | Any hand-written table drifts from the runtime the driver is actually driving |
| Verification status semantics | A regex over `*-VERIFICATION.md` | The frontmatter-anchored parse, mirroring `verification.cjs:312-332` | `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP` (verification.cjs:13-17) records that a broad grep false-matched `status:` inside fenced code blocks. The parser must anchor at byte 0 |
| Concurrency capping | A new counter | `Preferences::driver_max_concurrent`, default 1 | `src/config.rs:203-212` says verbatim: *"Phase 20 inherits a working cap instead of building one"* |
| Terminal-record park plumbing | A second reason carrier | `JournalEvent::Parked` + `terminal_label` | `src/driver/run.rs:417-463` already reads the last park and prefixes the label |

**Key insight:** almost every "new" mechanism this phase appears to need was deliberately built and left dormant by Phases 15–19. The plan's risk is not that these pieces are missing — it is that a plan written from the ROADMAP text alone will re-implement them beside the originals.

## Common Pitfalls

### Pitfall 1: `DiskStatus::Complete` does not mean the phase is complete

**What goes wrong:** The router treats a phase as done and advances, while its verification is `human_needed` — walking straight past the DRIVE-05 gate.
**Why it happens:** The Rust enum has seven variants and GSD has eight. This repository's derivation `[VERIFIED: src/state_reader/disk_status.rs:275-288]`:

```rust
let status = if summary_count >= plan_count && plan_count > 0 {
    DiskStatus::Complete
} else if summary_count > 0 {
    DiskStatus::Partial
} else if plan_count > 0 {
    DiskStatus::Planned
} else if has_research {
    DiskStatus::Researched
} else if has_context {
    DiskStatus::Discussed
} else {
    DiskStatus::Empty
};
```

That first arm is GSD's `implementation_complete` predicate (`init.cjs:181`) and therefore GSD's **`executed`**, not GSD's `complete`. The enum has no `Executed` variant at all `[VERIFIED: src/state_reader/disk_status.rs:4-14]`:

```rust
pub enum DiskStatus {
    #[default]
    NoDirectory,
    Empty,
    Discussed,
    Researched,
    Planned,
    Partial,
    Complete,
}
```

**How to avoid:** Read the VERIFICATION frontmatter `status` in `disk_status.rs` (the one reader) and either add an `Executed` variant or carry the verification status as a separate field on `DiskInference`. Adding a variant is the stronger move: every `match` on `DiskStatus` in `src/ui/screens/detail.rs:465-474` and elsewhere is exhaustive with no wildcard, so a new variant becomes a compile error at every site that has to classify it.
**Warning signs:** A router rule that maps `DiskStatus::Complete` to "advance to next phase"; a test fixture whose phase has summaries but no `*-VERIFICATION.md`.

### Pitfall 2: `DiskInference` never reads the verification status

**What goes wrong:** Six of the twenty gates in §4 are unobservable, so the router silently cannot park at them.
**Why it happens:** `DiskInference` carries `has_verification: bool` — *presence*, not *status* `[VERIFIED: src/state_reader/disk_status.rs:16-42]`. The file scan sets it from a filename match only `[VERIFIED: src/state_reader/disk_status.rs:255-257]`:

```rust
if name == "VERIFICATION.md" || name.ends_with("-VERIFICATION.md") {
    has_verification = true;
}
```

**How to avoid:** Extend the same scan to open the first `*-VERIFICATION.md` (sorted, take first — mirroring `verification.cjs:302-303`) and parse the leading frontmatter block for `status`, anchored at byte 0. Map the six values into a typed field. Do this **before** writing any routing rule; a rule table built on the current `DiskInference` cannot express the gate set.
**Warning signs:** A gate-set test that passes with an empty `*-VERIFICATION.md`.

### Pitfall 3: the wall-clock cap collides with the executor's existing one

**What goes wrong:** Criterion 3 requires a run that exceeds *its* wall-clock cap to halt and report that as the reason — but the reason reported is the executor's `TimedOut`, not the run's.
**Why it happens:** `ExecutionOptions` already defaults `wall_clock_cap` to **exactly four hours**, the same value CONTEXT.md proposes for the run-level cap `[VERIFIED: src/executor/mod.rs:381-382]`:

```rust
wall_clock_cap: Duration::from_secs(4 * 60 * 60),
idle_cap: Duration::from_secs(15 * 60),
```

and `RunOutcome::TimedOut { after }` / `RunOutcome::Stalled { idle_for }` already exist with labels `timed_out` / `stalled` `[VERIFIED: src/executor/mod.rs:682-692; src/driver/run.rs:410-411]`. With identical values, a single-iteration run can never distinguish the two caps, and a multi-iteration run trips the run cap only as an accumulation artefact.
**How to avoid:** Make the relationship an explicit, tested invariant — the per-iteration executor cap must be strictly less than the run-level cap, with a named test asserting the inequality (the precedent is `the_startup_stop_budget_fits_inside_the_driver_teardown_grace` at `src/driver/run.rs:167-176`). Either lower the per-iteration `wall_clock_cap` the driver passes, or raise the run-level default. Do not leave them equal by coincidence.
**Warning signs:** A halt reporting `timed_out` when the plan expected a run-bounds reason.

### Pitfall 4: hoisting the loop past the per-run setup

**What goes wrong:** The lock is released mid-run, or a second `run.json` is written, or the envelope is re-established and re-probed every iteration.
**Why it happens:** The setup block reads as preamble but four of its steps are one-shot by contract: `RunLock` has no `Drop` impl and its descriptor *is* the lock (`src/driver/run.rs:1278-1286`); `run.json` is written exactly twice (`:512`); `establish_envelope` writes four files and runs an external-client probe (`:110-134`); the SIGTERM handler must be installed before a single byte lands on disk (`:1157-1176`).
**How to avoid:** Draw the loop boundary at `:1401` (the `select!` that races `executor.start`) and `:1673` (`handle.wait_outcome()`). Nothing above or below moves.
**Warning signs:** A second `drive` against the same project starting alongside a live one; two `run_started` records in one journal.

### Pitfall 5: `RunSnapshot::capture` is invisible to the async-blocking lint

**What goes wrong:** The per-iteration observe step blocks the driver's only poll thread — the thread whose whole job is keeping the terminate arm reachable — and the guard reports green.
**Why it happens:** `tests/async_blocking_guard.rs` is a textual per-line scanner. `RunSnapshot::capture` does full-tree I/O and shells out to git twice and says so `[VERIFIED: src/executor/outcome.rs:51-59]`, but neither `RunSnapshot::capture(` nor `capture_snapshot(` appears in `BLOCKING_HELPERS` `[VERIFIED: tests/async_blocking_guard.rs:124-138]`:

```rust
const BLOCKING_HELPERS: &[&str] = &[
    "build_report(", "establish_envelope(", "terminal_label(", "lock::acquire(",
    "JournalRun::start(", "inbox::tail(", "probe_protection(", "scan_worktree(",
    "cred::build_env(", "hooks::install(", "write_settings(", "record_and_check(",
    "git_read_raw(",
];
```

This is a **pre-existing hole**, not one Phase 20 creates: `capture_snapshot` at `src/executor/claude.rs:747-751` is an `async fn` whose join-failure fallback at `:751` calls `RunSnapshot::capture(&fallback)` inline, unallowlisted and unreported. The scanner's own doc admits the ceiling: *"A new synchronous seam that nobody adds here is still invisible, and that residue is the honest ceiling"* `[VERIFIED: tests/async_blocking_guard.rs:109-111]`.
**How to avoid:** In the same plan that adds the per-iteration capture, add `"RunSnapshot::capture("` and `"capture_snapshot("` to `BLOCKING_HELPERS`, and add `("src/executor/claude.rs", "RunSnapshot::capture(")` to `ASYNC_BLOCKING_ALLOWLIST` with the join-failure-fallback reason the two existing entries of that shape already carry (`:200-210`). CONTEXT.md's prohibition is that the allowlist must not grow *silently* — a declared entry with a written reason, added in the same commit as the code it covers, is exactly what the constant's doc asks for (`:167-172`). `no_allowlist_entry_is_stale` (`:638`) will refuse it if it suppresses nothing, so the entry proves the hole was real.
**Warning signs:** A green `every_blocking_call_inside_an_async_fn_is_handed_off_or_allowlisted` alongside a driver that stops responding to the kill switch during a long observe.

### Pitfall 6: the pinned dry-run contract becomes a lie

**What goes wrong:** The preview keeps telling the user *"the single --command argument below is the complete and honest sequence for this build — not a truncated one"* while the router now issues many.
**Why it happens:** `SECTION_COMMANDS` is a `pub const` whose text is asserted verbatim by a paired test `[VERIFIED: src/driver/dry_run.rs:69-72; .planning/codebase/CONVENTIONS.md:75; .planning/codebase/TESTING.md:74]`. CONVENTIONS.md: *"Treat changing pinned-contract text as a breaking, user-visible change — update the paired test in the same commit."*
**How to avoid:** Update `SECTION_COMMANDS`, `build_report` and `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs` in one commit. `DryRunReport::commands` is already `Vec<String>` (`:105-111`), so no type changes. Grep for the other four sites carrying the same now-false claim (see Runtime State Inventory).
**Warning signs:** A dry-run preview showing one command for a goal-driven run.

### Pitfall 7: `tests/driver_reattach.rs` is flaky under a full parallel test run

**What goes wrong:** The phase gate reports red for a reason Phase 20 did not cause, and someone "fixes" unrelated code.
**Evidence, measured this session:**

```
---- a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired stdout ----
thread '…' panicked at tests/driver_reattach.rs:459:50:
the run record is on disk: Os { code: 2, kind: NotFound, message: "No such file or directory" }
test result: FAILED. 2 passed; 1 failed
```

`[VERIFIED: rtk proxy cargo test, run 1 of 3, 2026-08-19]`

An isolated `cargo test --test driver_reattach` passed 3/3, and a second full `cargo test` passed with zero failures across every target `[VERIFIED: runs 2 and 3, 2026-08-19]`. The test races the detached driver writing `run.json`; under parallel load the read wins.
**How to avoid:** Treat a single red on this test as inconclusive; re-run in isolation before investigating. If Phase 20's outer loop delays the first `run.json` write at all, the race widens — worth a deliberate check, and worth considering a bounded poll in the test rather than a bare read.

### Pitfall 8: `result` is a turn boundary, not a run terminator — and the outer loop makes this worse

**What goes wrong:** The iteration loop breaks on the first `TurnCompleted` and truncates every steered iteration while reporting success.
**Why it happens:** Phase 15's D-29 finding, recorded in STATE.md:96-99 and enforced at `src/driver/run.rs:1642-1644` (*"Deliberately no `break` here"*). A single `claude` process can emit two `system/init` and two `result` envelopes when a message is injected mid-turn. The outer loop's boundary is `handle.wait_outcome()` at `:1673` — reached only after `handle.events.recv()` returns `None` at `:1591` — never a `result`.
**How to avoid:** The iteration boundary is the **event stream closing**, not a `result`. `tests/executor_transport.rs::a_message_sent_mid_turn_is_not_buffered_by_the_driver` is the existing regression guard for the adjacent mistake.

### Pitfall 9: a driver-tick fed by the file watcher is a self-trigger

**What goes wrong:** The driver observes its own journal writes as project change, re-routes, and never converges.
**Why it happens:** The driver writes into `.planning/meta-manager/runs/…`, which is under the watched tree. `ChangeKind::DriverJournal` exists precisely to route such writes to a byte-offset tail rather than a full re-parse `[VERIFIED: src/journal/mod.rs:561-568]` — but that is a TUI-side optimisation, not a driver-side guard.
**How to avoid:** ROADMAP's own phase risk states it: *"The driver must be driven by an explicit tick or run-completion, never by the file watcher."* The detached driver has no watcher today (`src/driver/run.rs:196-208` argues polling over `notify` deliberately) — keep it that way. The iteration tick is the previous iteration's `wait_outcome()` returning, full stop.

## Code Examples

### Reading the rate-limit info off the drain loop

```rust
// Source: shape derived from src/driver/run.rs:1531-1593 (the existing event arm)
// and src/executor/stream_json.rs:53-57 (the carried, unmodelled payload).
//
// The payload is a serde_json::Value by design — modelling a shape we did not
// consume would add a second thing to keep in sync with the CLI's patch cadence
// (stream_json.rs:53-57). Phase 20 IS a consumer, so it reads the three fields
// it needs and carries the rest verbatim.

event = handle.events.recv() => {
    match event {
        Some(event) => {
            // … existing turn_boundary / record_exec handling, unchanged …

            if let ExecutionEvent::Message(msg) = &event {
                if let StreamMessage::RateLimitEvent(value) = msg.as_ref() {
                    // Absent / non-object / unknown status all degrade to "carry
                    // it and do nothing", never to a parse failure (D-09).
                    if let Some(info) = value.get("rate_limit_info") {
                        latest_rate_limit = Some(info.clone());
                    }
                }
            }
        }
        None => break,
    }
}
```

### Classifying the retained info — string matching with an explicit fallback

```rust
// Source: posture from src/executor/outcome.rs:203-208 and
// src/executor/stream_json.rs:14-18 — `subtype` and `terminal_reason` are
// matched as &str with an explicit fallback arm that carries the observed value
// verbatim, because this CLI shipped three new values on one version line.
//
// Enum values VERIFIED against strings(claude 2.1.235):310638-310642 and the
// two committed golden transcripts.

fn quota_window(info: &serde_json::Value) -> QuotaWindow {
    match info.get("rateLimitType").and_then(serde_json::Value::as_str) {
        Some("five_hour") => QuotaWindow::FiveHour,
        Some("seven_day")
        | Some("seven_day_opus")
        | Some("seven_day_sonnet")
        | Some("seven_day_overage_included") => QuotaWindow::SevenDay,
        // CONTEXT.md: report the window as unknown and say so. Never estimate.
        Some(other) => QuotaWindow::Unknown { observed: other.to_string() },
        None => QuotaWindow::Unknown { observed: String::new() },
    }
}

/// Only `rejected` parks. `allowed` appears on a completely healthy run
/// (tests/fixtures/transcripts/01-success-textonly.ndjson) and
/// `allowed_warning` is informational at 25% utilization.
fn is_rejection(info: &serde_json::Value) -> bool {
    info.get("status").and_then(serde_json::Value::as_str) == Some("rejected")
}

/// Unix epoch SECONDS. 1785327000 and 1785859200 are the two observed values.
fn resets_at(info: &serde_json::Value) -> Option<i64> {
    info.get("resetsAt").and_then(serde_json::Value::as_i64)
}
```

### No-progress detection without a hash

```rust
// Source: src/executor/outcome.rs:105-129, used as-is.
//
// CONTEXT.md asks for "a stable hash over the same ProjectState value the router
// reads, plus the git HEAD sha and dirty flag already captured by RunSnapshot".
// DiskDelta::between IS that comparison, already written, already preserving the
// unknown-vs-unchanged distinction CONTEXT.md requires — and it needs no hash,
// which matters because ProjectState.phase_disk_statuses is a HashMap whose
// iteration order is nondeterministic.

let delta = DiskDelta::between(&previous_snapshot, &current_snapshot);
if !delta.made_changes() {
    unchanged_iterations += 1;
} else {
    unchanged_iterations = 0;
}
// Criterion 2: "unchanged across consecutive iterations" — CONTEXT.md fixes the
// threshold at 2.
if unchanged_iterations >= 2 {
    return halt(BoundsReason::NoProgress);
}
```

If a short greppable identifier is also wanted on the `Observed` journal record, follow the existing digest idiom over a **sorted** canonical rendering:

```rust
// Source: src/journal/mod.rs:543-554 — FNV-1a 64, `fnv1a64:` prefix, implemented
// inline because "this phase adds zero dependencies" and the digest is an
// identity comparison, not a security control.
pub fn argv_digest(argv: &[String]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let joined = argv.join("\u{1f}");
    let mut hash = OFFSET_BASIS;
    for byte in joined.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}
```

### The conformance oracle — proving the Rust table agrees with GSD's

```rust
// An integration test, not a runtime call. It runs GSD's own router against
// fixture project trees and asserts the pure Rust router picks the same command.
// This is the only honest way to keep the two in step without making a Node
// subprocess part of the run loop (D-11, and Phase 19's async-blocking lint).
//
// Follows tests/spawn_seam_guard.rs's shape: an integration test, because it
// shells out and reads trees, and "a test that walks src/ has no business living
// inside it" (TESTING.md:48).

// for each fixture project:
//   let expected = run_gsd_tools(&["query", "init.manager"], fixture_root);
//   let observed = router::decide(&parse_project_state(&fixture_root.join(".planning")));
//   assert_eq!(observed.command(), expected.recommended_actions[0].command);
//
// Skip-with-a-loud-reason if gsd-tools is not on PATH, and assert non-vacuity:
// at least one fixture must have produced a comparison, or the test passed for
// the wrong reason (TESTING.md:113).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| `WAITING.json` as the driver's checkpoint signal (Phase 13 design, self-dated valid-until 2026-04-30) | The verb exists but **no workflow writes it**; the observable gate set is the composite in §4 | Unknown — the verb is present in 1.10.0 but has zero call sites | **Do not model it.** STATE.md's standing concern is hereby resolved: the contract is inert |
| A five-command D-R-P-E-V pipeline (`discuss`→`research`→`plan`→`execute`→`verify`) | Four commands. **Research is not a separate command** — `plan-phase` performs it; and `verify` runs *inside* `execute-phase`, with `verification.status` read afterwards | GSD ≤1.10.0 `[VERIFIED: autonomous.md:378-469]` | The router's output alphabet is `discuss`/`plan`/`execute` for forward motion, plus `verify-work` as an unpark command a human runs |
| `--max-budget-usd` as a supplementary cost control | Post-turn circuit breaker only; bounds the next turn, never the current one | Phase 15 OQ3 spike, 2026-07-29 | The quota floor is the **only** real cost control. No dollar figure may be presented as "what this run cost" (D-16) |
| CLI 2.1.220 | CLI **2.1.235** installed | between 2026-07-29 and 2026-08-19 | Fixtures and every code comment reference 2.1.220. `error_max_structured_output_retries` is a `result` subtype present in 2.1.235 `[VERIFIED: strings(2.1.235):381618-381621]` and absent from `src/executor/stream_json.rs:283-287`'s observed list — harmless, because that field is a `String` with a fallback arm, which is exactly why it was made one |
| `ObservedRun.live: bool` | `ObservedRun.liveness: Liveness` + `is_live()` | Phase 17 CR-05 | Phase 20 must read the tri-state, never reintroduce a boolean `[CITED: .planning/STATE.md:71-74]` |
| `--run-id` generated when absent | **Required** for a real run | Phase 17 CR-04 | An iteration loop must not mint per-iteration ids; one run id spans every iteration |

**Deprecated / outdated:**

- `.planning/phases/19-gitsafe-git-blast-radius-envelope/.continue-here.md` is a **stale** mid-execution checkpoint claiming task 3 of 8, while all 8 plans have summaries `[CITED: .planning/STATE.md:269-270]`. Under gate G13, a naive gate implementation would park every run on this file forever. The gate must be at `.planning/.continue-here.md` (project root, G10) *and* the phase directory (G13) — and the phase-dir gate is specifically about rows with `severity: blocking`, not mere existence `[VERIFIED: execute-phase.md:223]`.

## Project Constraints (from CLAUDE.md)

| Directive | Source | Consequence for this phase |
|---|---|---|
| GSD workflow enforcement — no direct edits outside a GSD workflow | CLAUDE.md §GSD Workflow Enforcement | Plans are executed through `/gsd-execute-phase`, not by hand |
| `rtk` filters build/test output; a grep for `warning:` or `test result:` passes vacuously | CLAUDE.md §RTK; TESTING.md:20-23 | Every gate command in the plan that depends on raw output must be written as `rtk proxy cargo …` |
| No new Cargo dependencies unless justified | CLAUDE.md §What NOT to Use; the tree's own "this phase adds zero dependencies" precedent | Confirmed: this phase needs none |
| No `std::sync::Mutex` in async code — use `tokio::sync::Mutex` | CLAUDE.md §What NOT to Use | The iteration loop holds no shared state across `.await` that would need one; keep it that way |
| Never block the render loop; all I/O through channels | CLAUDE.md §Stack Patterns | Reinforced by Phase 19's `tests/async_blocking_guard.rs` |
| Parse `.planning/` lazily, cache in the shared reader | CLAUDE.md §Stack Patterns | D-11's single-reader rule, already locked in CONTEXT.md |
| Release process: bump `Cargo.toml`, `cargo update`, verify `cargo build && cargo test && cargo clippy -- -D warnings` | CLAUDE.md §Release Process | Not triggered by this phase (no milestone tag), but the three-command gate is the phase gate |
| Documented pre-PR gate is `cargo clippy -- -D warnings` (lib target); `--all-targets` has **5 known pre-existing lints** | CONVENTIONS.md:33-39; TESTING.md:29-41 | The phase gate asserts the lib gate clean and the `--all-targets` delta unchanged at 5 — the same shape Phase 19's 19-08 used |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `claude` CLI | Real driven runs; rate-limit protocol verification | ✓ | **2.1.235** (`~/.local/bin/claude` → `~/.local/share/claude/versions/2.1.235`) | Debug-only `--claude-program` stand-ins in `tests/fixtures/fake-claude*.sh` |
| `node` + `gsd-tools.cjs` | The conformance oracle test (test-time only) | ✓ | GSD **1.10.0** (`~/.claude/gsd-core/VERSION`) | Skip-with-loud-reason if absent; must not be a runtime dependency |
| `cargo` / rustc | Build and gate | ✓ | MSRV floor 1.87 (`Cargo.toml:6`) | — |
| `git` | `RunSnapshot::capture`, envelope, dry-run | ✓ | — | `Option` returns already model absence |
| `rtk` | Unfiltered build/test output | ✓ | `rtk proxy` required for any raw-output check | — |
| Committed golden transcripts | Rate-limit fixture work | ✓ | 8 files in `tests/fixtures/transcripts/` at 2.1.220; two carry `rate_limit_event` | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

**One environment note worth planning around:** the committed transcripts are 2.1.220 captures and the installed CLI is 2.1.235. A ninth fixture capturing a **`status: "rejected"`** `rate_limit_event` does not exist and cannot be produced without burning quota. The plan should build the rejection path against a **synthesised** fixture line whose field names and enum values are taken verbatim from this document's binary-verified inventory, and label it as synthesised in `tests/fixtures/transcripts/README.md` so no future reader mistakes it for a capture.

## Security Domain

> `security_enforcement` is absent from `.planning/config.json` `[VERIFIED: .planning/config.json]`, so it is treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard control |
|---|---|---|
| V2 Authentication | no | The driver authenticates nothing; the CLI holds the subscription session |
| V3 Session Management | no | `session_id` is a correlation id, not a credential |
| V4 Access Control | **yes** | `DrivableProject` capability token with private fields and exactly two constructors; single production call site enforced by `tests/spawn_seam_guard.rs`. The iteration loop must not create a second `from_registry` call site |
| V5 Input Validation | **yes** | The `rate_limit_info` payload is untrusted input from a process that itself consumed untrusted repository content. Tolerant parsing only — no `deny_unknown_fields` (mechanically forbidden under `src/`), no panicking accessor, no `unwrap` on a wire field |
| V6 Cryptography | no | FNV-1a64 is explicitly not a security control (`src/journal/mod.rs:537-539`) |
| V7 Error Handling & Logging | **yes** | Log the error **kind** only, never a message body — the journal lands in the driven project's `.planning/`, a directory users commit. Every `tracing::warn!` in `src/driver/run.rs` follows this (T-17-05, T-18-03) |
| V12 File & Resource | **yes** | `run_id` must remain a single plain path component (`journal::is_plain_path_component`, `src/driver/mod.rs:289-293`). An iteration loop must not derive per-iteration directory names from anything the agent can influence |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard mitigation |
|---|---|---|
| Agent-authored `.planning/` content steers the router | Tampering / Elevation | The router is rules-only and reads a **typed** `ProjectState`, never prose. Phase 21 owns prompt-injection hardening; Phase 20's contribution is that no free text reaches a routing decision |
| Agent writes a `*-VERIFICATION.md` with `status: passed` to unpark itself | Tampering | Real and unmitigated by this phase alone. GSD's verifier is the only sanctioned writer, and the driver runs it. Worth recording as an open question rather than papering over — note that `verification.cjs`'s staleness check (a summary newer than the verification) is a partial detector for exactly this |
| `run_id` path traversal into another project | Elevation | Already closed at `src/driver/mod.rs:289-293` (WR-02 reproduced `--run-id '../../../../escaped'` writing outside the project with exit 0). The loop must not introduce a second path-composition site |
| Agent self-report drives the terminal classification | Spoofing | D-10 and REQUIREMENTS' out-of-scope list. Goal-met must be derived from deterministic project state, never from prose. The `result` envelope's `result` field is read by **no** branch of the derivation (`src/executor/outcome.rs:172`) — keep it that way |
| A rate-limit park that retries | DoS (self-inflicted, against a shared quota) | CTRL-07: no retry, no backoff, no sleep-until-reset. A backoff is a retry with a delay |
| N concurrent driven projects multiplying quota burn | DoS | `driver_max_concurrent` default 1, enforced at the spawn seam |
| Journal disclosure of agent-read secrets | Information Disclosure | `redact::RedactedLine` newtype at the capture path (SAFE-04). New journal records this phase adds — `Observed`, `Decided`, bounds `Parked` — must carry only enum values and command names, never free text from the agent |

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|---|---|---|
| A1 | `resetsAt` is unix epoch **seconds**, not milliseconds | Rate-Limit Protocol | Two observed 10-digit values consistent with seconds, and their delta ≈ 6.16 days matches a 7-day window — but no unit is stated anywhere in the binary's strings. Wrong units would render a reset time ~56,000 years out. **Mitigation: sanity-bound the parsed value** (reject anything more than ~30 days from now) and report "window unknown" rather than a nonsense timestamp |
| A2 | A `status: "rejected"` `rate_limit_event` is emitted on the `-p` stream, not only in the interactive TUI | Rate-Limit Protocol | The `[print] rate_limit listener failed:` string and the transcript renderer's `rate_limit: rejected (` both exist in 2.1.235, and `allowed`/`allowed_warning` are both observed on `-p` streams — but no `rejected` capture exists. If it turns out `rejected` arrives only as a `result` `terminal_reason: api_error_rate_limit`, CTRL-07 needs a second detector on that path. **Mitigation: implement both detectors** — the `rate_limit_event` reader and a `terminal_reason.starts_with("api_error")` + `errors[]` check — since the second is nearly free given `derive_run_outcome_from_envelopes` already carries the field |
| A3 | GSD 1.10.0's routing table will remain stable across the phase's life | GSD Runtime Contract | GSD shipped 1.8.0→1.10.0 in under a month. **Mitigation: the conformance oracle test is the whole answer** — it fails loudly when GSD's table moves, which is the outcome Phase 13's undated design lacked |
| A4 | `gaps_found` should PARK rather than auto-route to `/gsd-plan-phase N --gaps` | GSD Runtime Contract / §4 | GSD's own router emits a concrete next command for this state and autonomous.md follows it. CONTEXT.md's OQ7 resolution ("always park") is stricter. Parking here is defensible and safe, but it is a *choice against the upstream default* — worth naming in the plan rather than presenting as forced |
| A5 | The five-element `Observed.drpev` vector maps onto (has_context, has_research, plan_count, summary_count, verification_status) | The Loop | The field's only documentation is *"the five D-R-P-E-V stage statuses, in order"* (`src/journal/mod.rs:657`). No writer exists to compare against. The mapping is the obvious one but is not stated |
| A6 | Phase 22's `ExecutionTarget` will gain a `Container` variant that the router must not branch on | Scope fence | `ExecutionTarget` currently has one variant, `Host` (`src/executor/mod.rs:219-223`), and ROADMAP says container is *"an `ExecutionTarget` enum inside the one executor — argv prefix plus path map swap"*. If Phase 22 instead introduces a second `Executor`, the loop's assumption that one `executor` value spans all iterations may need revisiting |

## Open Questions

1. **Does the run-level wall-clock cap or the per-iteration executor cap win, and by how much?**
   - What we know: both currently default to exactly 4 hours (`src/executor/mod.rs:381`); CONTEXT.md proposes 4 hours for the run.
   - What's unclear: which value moves.
   - Recommendation: keep the run cap at CONTEXT.md's 4 hours (it is the user-facing number) and reduce the per-iteration executor `wall_clock_cap` the driver passes — a single GSD command taking four hours is already pathological, and the `idle_cap` of 15 minutes is the real stuck-detector. Assert the strict inequality in a named test.

2. **Is `gaps_found` a park or an auto-route?**
   - What we know: GSD emits `/gsd-plan-phase N --gaps` for it; CONTEXT.md's OQ7 resolution says always park.
   - What's unclear: whether parking here makes a fully autonomous run park at nearly every phase, which CONTEXT.md itself flags as the accepted consequence to record if dogfooding shows it.
   - Recommendation: park, per CONTEXT.md, but make it a **named, separately-greppable reason** (`gate_verification_gaps_found`) rather than folding it into a generic gate reason, so the dogfooding finding is measurable from the journals rather than remembered.

3. **Can a driven run's own agent write `status: passed` into `*-VERIFICATION.md` and thereby unpark itself?**
   - What we know: nothing mechanically prevents it. The `--disallowedTools` envelope covers git, not file writes into `.planning/`.
   - What's unclear: whether this is in scope for Phase 20 at all, or belongs to Phase 21's hardening.
   - Recommendation: out of scope for Phase 20 — but record it explicitly in the phase's residual-exposure disclosure, in the register `src/envelope/mod.rs` already established. The staleness detector is a partial mitigation worth naming.

4. **What is the goal-met predicate for a run with no Phase 21 goal layer?**
   - What we know: criterion 5 requires goal-met as a terminal class; DRIVE-01/03 (the goal itself) is Phase 21. `DriveArgs::goal` today is *"free text recorded into `RunRecord.goal` and never interpreted"* (`src/driver/mod.rs:116-117`).
   - What's unclear: what Phase 20 can honestly declare goal-met against.
   - Recommendation: a **target phase** supplied on argv, with goal-met = that phase's `verification_status == 'passed'`. It is deterministic, machine-checkable, satisfies criterion 5 without borrowing from Phase 21, and it is exactly the shape Phase 21's OQ6 says a goal must reduce to.

5. **Does the outer loop reuse one `claude` session across iterations, or start fresh each time?**
   - What we know: `ExecutionOptions.session_id` is a `Uuid` generated pre-spawn, and `resume_session: Option<String>` exists (`src/executor/mod.rs:296`). `run.json` carries a single `session_id` (`src/driver/run.rs:504`).
   - What's unclear: whether one GSD command per fresh session, or a resumed session across the run.
   - Recommendation: **one fresh session per iteration.** Each GSD command is a discrete unit with its own context budget, and a resumed session accumulates the previous command's transcript into the next command's window — which is how a multi-hour run hits a context limit for reasons unrelated to the work. It also keeps `run.json`'s single `session_id` field from becoming a lie; if the sequence needs recording, it belongs on the per-iteration `Decided`/`ExecStarted` journal events, which already carry `session_id` (`src/journal/mod.rs:672-676`).

## Sources

### Primary (HIGH confidence — read directly on this machine, this session)

- `~/.claude/gsd-core/VERSION`, `.gsd-runtime` — GSD 1.10.0, runtime `claude`
- `~/.claude/gsd-core/bin/lib/init.cjs` — `:181`, `:189-208`, `:1840-1937`, `:2000-2118` (disk_status derivation, verification projection, routing table, waiting_signal)
- `~/.claude/gsd-core/bin/lib/verification.cjs` — `:50`, `:72-113`, `:114-127`, `:214-258`, `:282-406` (routing table, staleness, frontmatter anchoring)
- `~/.claude/gsd-core/bin/lib/uat-predicate.cjs` — `:28-43` (the four blocking-status sets, verbatim)
- `~/.claude/gsd-core/bin/lib/state.cjs` — `:2290-2335` (WAITING.json write/remove)
- `~/.claude/gsd-core/bin/gsd-tools.cjs` — `:18-19` (signal-waiting verbs)
- `~/.claude/gsd-core/workflows/next.md` — `:40-175` (three hard-stop gates, Route 0, prior-phase prompt)
- `~/.claude/gsd-core/workflows/autonomous.md` — `:300-382`, `:380-481`, `:690-716`, `:757-769`
- `~/.claude/gsd-core/workflows/execute-phase.md` `:217-235`, `discuss-phase.md` `:162-177`, `cleanup.md` `:126`, `complete-milestone.md` `:641`
- Live command output: `gsd-tools query init.manager`, `query init.progress`, `query verification.status <dir>`, `query audit-uat`, `query list-todos`, `query roadmap.analyze`, `query windows status`, `check auto-mode` — all executed against this repository 2026-08-19
- `claude --version` → 2.1.235; `strings` over `~/.local/share/claude/versions/2.1.235` at offsets 307000, 308440-308490, 310600-310790, 316150-316270, 322930-322990, 381600-381660, 381920-381980
- `tests/fixtures/transcripts/01-success-textonly.ndjson` and siblings — the two committed `rate_limit_event` captures
- `src/driver/run.rs`, `src/driver/mod.rs`, `src/driver/dry_run.rs`, `src/executor/mod.rs`, `src/executor/outcome.rs`, `src/executor/claude.rs`, `src/executor/stream_json.rs`, `src/journal/mod.rs`, `src/state_reader/mod.rs`, `src/state_reader/disk_status.rs`, `src/envelope/policy.rs`, `src/config.rs`, `src/cli.rs`, `src/ui/screens/detail.rs`, `tests/async_blocking_guard.rs`, `Cargo.toml`
- `.planning/phases/20-…/20-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md`, `.planning/config.json`, `.planning/codebase/{CONVENTIONS,TESTING,ARCHITECTURE}.md`, `./CLAUDE.md`
- Measured: `rtk proxy cargo test` × 3 (one intermittent `driver_reattach` failure, two clean)

### Secondary (MEDIUM confidence)

- None. No web search was performed and none was warranted: this phase has no external package selection, no ecosystem question, and every unknown it carried was answerable from files on this machine. A web result about the Claude rate-limit protocol would have been strictly weaker evidence than the installed binary's own string table and this repository's committed captures.

### Tertiary (LOW confidence)

- Items A1–A6 in the Assumptions Log, each with its mitigation stated inline.

## Metadata

**Confidence breakdown:**

- GSD runtime contract (routing table, gate set, WAITING.json): **HIGH** — every claim is a line read from the installed 1.10.0 runtime this session, and the routing table was additionally confirmed by executing it against this repository.
- Rate-limit protocol (fields, enums, distinguishability): **HIGH** for field names, enum members and the two observed statuses (committed captures + binary string table); **MEDIUM** for the `rejected` emission path on `-p` specifically (A2) and for the `resetsAt` unit (A1), both with mitigations.
- The loop's shape and the build-vs-bound answer: **HIGH** — three explicit source statements plus a test that enforces the current single-command shape.
- Existing-asset inventory (journal schema, `DiskDelta`, `ParkReason`, concurrency cap, pinned contracts): **HIGH** — all read directly.
- Standard stack: **HIGH** — zero dependencies, verified against `Cargo.toml`.
- Pitfalls: **HIGH** — 1, 2, 3, 5, 6, 7 were each measured or read rather than inferred; 4, 8, 9 are restatements of decisions already written in the tree.

**Research date:** 2026-08-19
**Valid until:** 2026-09-18 for the Rust-side findings (stable, in-repo). **2026-09-02 for the GSD-runtime findings** — GSD shipped 1.8.0 → 1.10.0 in roughly one month, which is the exact staleness that made Phase 13's design obsolete. The conformance oracle test recommended above is the durable answer: it converts this expiry date from a note in a document into a failing build.
