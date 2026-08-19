# Phase 20: Deterministic Decision Router & Run Bounds - Context

**Gathered:** 2026-08-19
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run). Recommendations were auto-accepted per an explicit
user instruction to run fully autonomously; every answer below is a proposal the user did not
individually confirm. Treat them as decisions of record, but flag any that the research step
contradicts rather than silently honouring them.

<domain>
## Phase Boundary

This phase makes the driver's *choice of next GSD command* a rule, and makes a run that stops
making progress stop itself.

**In scope:**
- A deterministic decision router: observed project state → next GSD command, with no model
  call for any state the rules cover (DRIVE-02).
- Self-halting bounds: no-progress detection, command-repeat detection, a step cap, and a
  wall-clock cap (CTRL-06).
- Rate-limit parking: a run that hits a Claude subscription quota parks, names the window that
  blocked it and when it resets, and does not retry (CTRL-07).
- GSD-gate parking: reaching a gate that needs human judgement parks the run with the gate
  named (DRIVE-05).
- Terminal classification: every run ends as goal-met, parked, or halted, each with a reason
  (DRIVE-06).

**Out of scope (explicitly):**
- The LLM goal layer — deciding *what* the goal is, and prompt-injection hardening around it,
  is Phase 21. This phase's router is rules-only; where rules do not cover a state it parks,
  it does not escalate to a model.
- Container targeting (Phase 22). The ROADMAP records that Phase 22 must land before Phase 20
  *closes*; that is a closing condition on this phase, not an input to its implementation. The
  router must not grow a container-shaped branch here.
- Any new progress display. The existing D-R-P-E-V pipeline widget is the step timeline
  (REQUIREMENTS "Out of Scope").
- `--max-budget-usd` as a cost control. It is an anomaly circuit breaker only; the real
  constraint is the 5h/7d quota (D-16, confirmed empirically in Phase 15's OQ3 spike: it is a
  post-turn breaker that bounds the next turn, never the current one).

</domain>

<decisions>
## Implementation Decisions

### Router Surface & Determinism

- The router lives in a new `src/driver/router.rs`, beside `run.rs`, as pure functions. It is
  not a new subsystem and not an extension of `state_reader`.
- Its input is the `ProjectState` produced by `state_reader::parse_project_state` — the same
  reader the dashboard and the driver already share. The decision function performs **no I/O
  itself**; the caller reads state and hands it in. D-11 forbids a parallel reader, and a pure
  function is what makes criterion 1's "the same project state always yields the same choice"
  testable as a value property rather than a filesystem experiment.
- Determinism is proved by an exhaustive table-driven test over every D-R-P-E-V state the
  rules cover, plus a same-input-same-output property test. "For every state the rules cover"
  is the criterion's own wording, so the test table and the rule table must be derived from
  one another — a rule with no row, or a row with no rule, should fail the build.
- A state the rules do **not** cover parks the run with a `router_no_rule` reason that names
  the observed state. It never falls back to a model call, never defaults to `/gsd-progress`,
  and never guesses. An uncovered state is a gap in the rule table, and parking is how that
  gap becomes visible.

### Progress Detectors & Run Bounds

- The observed-state hash is a stable hash over the same `ProjectState` value the router reads,
  plus the git `HEAD` sha and dirty flag already captured by `executor::outcome::RunSnapshot`.
  It is not a scan of `.planning/` file contents or mtimes — the reader is already the single
  source of truth and `ProjectState` already derives `PartialEq`.
- Two detectors, both required by criterion 2: **unchanged state hash across 2 consecutive
  iterations** halts, and **the same command re-selected twice in a row** halts. The halt
  names which of the two fired.
- Default step cap: 20 iterations. Default wall-clock cap: 4 hours. Both are overridable per
  run on argv and are recorded in `run.json` so the bound in force is readable after the fact
  rather than inferred from the binary's defaults.
- When several bounds could trip on the same iteration, evaluation order is fixed and
  documented, the first detector wins, and the halt reports exactly one detector. Criterion 2
  says "names which detector fired" — a list of detectors is not that.

### Quota & Rate-Limit Handling (CTRL-07)

- A rate limit is detected from the `result`/error envelope of the stream-json transport,
  never from the agent's prose. This is the same rule `executor/outcome.rs` already enforces
  (D-10) and the same reason "Trusting agent self-reports" is in REQUIREMENTS' out-of-scope
  list.
- On a rate limit the run **parks immediately**: no retry, no backoff, no sleep-until-reset.
  CTRL-07's wording is "parks rather than retrying"; a backoff is a retry with a delay.
- The park reports which quota window blocked it (5-hour vs 7-day) and when it resets, taken
  from the API-provided reset field. If that field is absent, the run reports the window as
  unknown and says so. It does not estimate, and it does not present a dollar figure as "what
  this run cost" (D-16).
- Concurrency: a global default cap of one concurrent driven run, validating OQ10's concern
  that the 5h/7d quota is shared across every Claude surface the user has, so N driven
  projects burn it N×. No synthetic quota accounting.

### Terminal Classification & GSD-Gate Parking

- One exhaustive terminal type — goal-met, parked-with-reason, halted-with-reason — with no
  unclassified arm. Criterion 5's "never as an unclassified 'loop ended'" is a type-level
  requirement, not a logging convention: if the type cannot express "ended for no stated
  reason", the failure mode is unrepresentable.
- **OQ7 resolved: always park.** Reaching a GSD gate that requires human judgement parks the
  run with the gate named, from an explicit enumerated gate set (verification `human_needed`,
  verification `gaps_found`, outstanding UAT, an execution checkpoint awaiting a decision,
  milestone completion, and a blocker prompt). This is the research recommendation and it is
  accepted with its consequence stated plainly: a fully autonomous run parks at every phase
  boundary by design. That is the value-proposition call OQ7 asked for, made explicitly. If
  the phase's own dogfooding shows this makes the driver useless in practice, that is a
  finding to record — not grounds to auto-answer a judgement gate.
- Reason plumbing: `envelope::policy::ParkReason` keeps its existing seven safety arms
  untouched; the router/bounds reasons live in a sibling enum. Both flow through the one
  terminal record and the one journal event, so a reader greps one place for "why did this run
  end". Phase 19 established the `Parked` journal event carrying a reason string read back off
  disk by a different process — reuse that carrier rather than inventing a second.
- Goal-met is derived from deterministic project state (the declared target reached — e.g. the
  target phase's verification passed, or the milestone closed), never from the agent saying it
  is done.

### Claude's Discretion

- Module/file decomposition below `src/driver/router.rs`, naming of the bounds reason enum and
  its `as_str` constants, and the exact shape of the rule table (match arms vs a data table)
  are open — follow `CONVENTIONS.md` and the pattern `envelope/policy.rs` already set for pure
  classification functions with a stable snake_case reason string per arm.
- Whether the step/wall-clock caps also appear in the Defaults tab is discretionary; if it is
  cheap, prefer surfacing them, since v1.x already exposes `.planning/config.json` keys there.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `state_reader::parse_project_state` — the single project-state reader shared by the
  dashboard and the driver (D-01, D-11). The router's input type, not a new reader.
- `executor::outcome::RunSnapshot` — already fingerprints a project as `head_sha` + `dirty` +
  `ProjectState`, with `None` meaning *unknown* rather than "unchanged". The state hash should
  be built from this, and must preserve that unknown-vs-unchanged distinction: an unknown
  fingerprint is not evidence of no progress.
- `executor::outcome::derive_run_outcome_from_envelopes` — the one run-outcome entry point,
  deliberately taking full `result` envelopes. The rate-limit signal should be read here or
  beside it, not from a second envelope-discarding path (CR-04 is exactly that bug).
- `envelope::policy::ParkReason` + its `REASON_*` constants — the established pattern for a
  typed reason with a stable greppable string. Mirror it; do not extend it.
- `journal::` `Parked` event + `run.json` terminal record — the existing durable carrier for
  "this run stopped, here is why", already proven readable by a separate process (Phase 19).
- `driver::run::execute_run` and `driver::dry_run` — the loop the router plugs into, and the
  preview surface that should be able to show the chosen command without running it.

### Established Patterns

- Pure classification functions with exhaustive unit tests (`envelope/policy.rs`: 47 tests
  over `classify_git`, `classify_push_ref`).
- `subtype`/`terminal_reason` matched as `&str` with an explicit fallback arm rather than a
  typed enum, because the CLI ships new values on a version line. Apply the same posture to
  any rate-limit subtype matching.
- No blocking calls inside `async fn` — Phase 19's 19-08 added a lint with a justified
  allowlist that fails when a blocking `std::process::Command` is planted in `drive`. New
  router/bounds code must stay on the right side of it, and its allowlist must not grow
  silently.

### Integration Points

- `src/driver/run.rs::execute_run` — where an iteration ends and the next command is chosen.
- `src/driver/mod.rs::DriveArgs` — where per-run cap overrides arrive on argv.
- `src/journal/` — where the terminal classification is durably recorded.
- The D-R-P-E-V pipeline widget — reused as the step timeline; no new display.

</code_context>

<specifics>
## Specific Ideas

- Phase 13's queue-execution design self-dated "valid until 2026-04-30" and GSD has shipped
  1.8.0 since. STATE.md carries this as an open concern: **re-verify GSD's autonomous-mode
  semantics and the `WAITING.json`/checkpoint contract during this phase's research.** The
  router's gate set is only as correct as that contract.
- Phase 15's OQ3 result is a hard input: `--max-budget-usd` does apply under subscription auth
  but only as a post-turn circuit breaker. The quota floor is the real cost control here.
- The driver must be driven by an explicit tick or run-completion, never by the file watcher —
  a watcher-fed loop is a genuine self-trigger that did not exist in the read-only
  architecture (ROADMAP phase risk).
- Refuse to act on inferred-only state: v1.1's verified/inferred badge is a safety input to
  the router, not decoration.

</specifics>

<deferred>
## Deferred Ideas

- Fleet-level driving (one goal across multiple projects) — out of v2.0 scope; the global
  concurrency cap of 1 chosen above is the v2.0 posture, not a step toward fleet mode.
- Socket-based fast-path for injection — the durable file inbox already covers the
  requirement.
- Any model-assisted routing for states the rules do not cover — that is Phase 21's goal
  layer, and this phase deliberately parks instead.

</deferred>
