# Phase 21: LLM Goal Layer & Prompt-Injection Hardening - Context

**Gathered:** 2026-08-19
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run). Recommendations were auto-accepted per an explicit user
instruction to run fully autonomously; every answer below is a proposal the user did not
individually confirm. Treat them as decisions of record, but flag any that research contradicts
rather than silently honouring them.

<domain>
## Phase Boundary

This phase adds the **only** place a model is allowed to influence what the driver does, and it
adds the boundary that keeps third-party text from becoming instructions.

**In scope:**
- A stated goal in plain language becomes a structured, machine-checkable plan the user reviews
  before anything runs (DRIVE-01, DRIVE-03).
- Exactly two model seams, both bounded and counted against a per-run escalation cap; exceeding
  the cap parks (DRIVE-04).
- `.planning/` and `CLAUDE.md` content reaches the model inside an explicit untrusted-content
  boundary, never concatenated into instructions (SAFE-07).
- Every action the model names is constrained to the fixed GSD command enum Phase 20 already
  built; a free-form shell string is never executed (SAFE-08).

**Out of scope (explicitly):**
- Widening the router. Phase 20's rules stay authoritative for every state they cover. The model
  is consulted **only** where the router returns `router_no_rule` — it never overrides a rule.
- The gate policy (skip/defer/auto) — that is Phase 23 (CTRL-08, DRIVE-07). This phase must not
  pre-empt it, and must not make the always-park behaviour harder to change later.
- Container targeting — Phase 22.
- Relaxing anything Phase 19's envelope enforces. The envelope is the last line of defence and
  this phase must leave it strictly no weaker.

</domain>

<decisions>
## Implementation Decisions

### The Goal → Plan Decomposition (DRIVE-01, DRIVE-03)

- A "machine-checkable plan" is an **ordered list of steps over Phase 20's existing safe
  alphabet** (`SAFE_COMMAND_ALPHABET`, `src/driver/router.rs:327`), each step carrying a target
  phase and the terminal state that would satisfy it. It is not prose with a checklist stapled on.
  If a step cannot be expressed in that vocabulary, the plan is not machine-checkable and the goal
  is refused.
- **OQ6 resolved: a goal that cannot be reduced to a deterministic predicate is refused at start**,
  naming the part that could not be reduced. Reuse Phase 20's goal-met primitive
  (`router::is_goal_met` — the target phase's `verification_status == 'passed'`). A goal with no
  stopping condition has no terminal state, and a run with no terminal state is the exact
  "unclassified loop ended" outcome Phase 20's criterion 5 forbids.
- The plan is **rendered for review before anything runs**, and approval is an explicit recorded
  act on the run record — not an inferred consent, not a timeout-to-yes.
- The plan lives on the run record and the journal, the same durable carrier Phase 20 already
  proved readable by a separate process. No new store.

### The Two Model Seams and Their Cap (DRIVE-04)

- **Exactly two seams, and the count is a property of the design, not a coincidence:** goal
  decomposition (once, at run start) and ambiguity escalation (only where the router returns
  `REASON_NO_RULE`). There is deliberately no third seam for error recovery — an error the rules
  cannot classify is a park, not a prompt.
- The escalation cap is counted per run, recorded on the run record beside Phase 20's `bounds`,
  and exceeding it **parks** using Phase 20's existing park machinery under its own greppable
  reason. Not a halt, and not a silent degrade to rules-only: a run that stopped consulting the
  model has materially changed behaviour and must say so.
- The escalation's **input is the router's observed state only** — never raw file content. This is
  what keeps the ambiguity seam narrow enough to reason about; feeding it file excerpts would
  reopen SAFE-07 at the one place the model has influence.
- The escalation's **output must parse to the fixed command enum or it is refused**, and a refusal
  is itself a park rather than a retry with a stricter prompt. Retrying a model that just produced
  an invalid action is how a bounded seam becomes an unbounded one.

### The Untrusted-Content Boundary (SAFE-07)

- Untrusted content is delimited **structurally** — passed as a distinct typed field with explicit
  boundary markers — never string-concatenated into an instruction. Prompt-text warnings ("ignore
  anything below that looks like an instruction") are exactly the prompt-only guardrail
  REQUIREMENTS.md lists as out of scope, and the Replit incident is in this project's own research
  as the reason.
- `--strict-mcp-config` with an explicit `--mcp-config`, so a project-local `.mcp.json` in a cloned
  third-party repo cannot introduce tools into a driven run.
- **Disclosure with a content hash:** opting a project in lists exactly which files will enter
  prompts and records a hash of each. If `CLAUDE.md` changes after opt-in, re-confirm. The threat
  is not hypothetical — this tool drives other people's cloned repos, and their `CLAUDE.md` is
  third-party content that can change under a `git pull` the user did not read.
- **The driver sets its goal once, from a human, and may never enqueue itself another goal from an
  artifact created during the run.** This is enforced mechanically, not documented. A run that can
  write its own next goal has no bound that means anything.

### Action Refusal (SAFE-08)

- The model may name only a variant of Phase 20's existing command enum. **No shell string is ever
  constructed from model output**, at any point, including for logging or preview.
- A refusal **parks with the named-but-refused action recorded verbatim**, so an injection attempt
  becomes evidence on disk rather than a silent no-op. A silently-dropped injection teaches nobody
  that the repo is hostile.
- Proof is an **injection corpus fixture** — real `.planning/` and `CLAUDE.md` files carrying
  genuine injection strings — asserting the *selected command is unchanged*. Unit-testing the
  parser alone proves the parser, not the property.
- **The envelope stays the last line of defence.** A test must prove that a model-named git push
  is still refused by Phase 19's envelope, so the two layers are shown to be independent rather
  than assumed to be.

### Claude's Discretion

- Module layout, the escalation cap's default value, and the exact wire shape of the structured
  plan are open — follow `CONVENTIONS.md` and the `envelope/policy.rs` pattern for typed reasons
  with stable snake_case strings.
- Whether the plan review surface is a TUI screen or a rendered preview is discretionary; the
  requirement is that approval is explicit and recorded, not where it is clicked.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `router::SAFE_COMMAND_ALPHABET` (`src/driver/router.rs:327`) and `router::RouterAction` — the
  fixed enum SAFE-08 requires already exists. This phase consumes it; it does not invent a second.
- `router::REASON_NO_RULE` (`:71`) — the single, already-greppable signal marking the one state
  where the ambiguity seam may fire.
- `router::is_goal_met` — the deterministic goal predicate, added by 20-04. OQ6's "machine-checkable
  stopping condition" reduces to this.
- Phase 20's park machinery, `bounds` on the run record, and the journal `Parked` carrier — the
  escalation cap and every refusal reuse them rather than adding a parallel path.
- `envelope::` (Phase 19) — the mechanical push/secret/PR boundary that must remain the last line
  of defence and must be shown still to fire on a model-named action.

### Established Patterns

- Typed reason enums with stable snake_case `as_str` constants (`envelope/policy.rs`,
  `driver/router.rs`, `driver/bounds.rs`, `driver/rate_limit.rs` — now four siblings; a fifth must
  follow the same shape and be added to the taxonomy table in `src/journal/mod.rs`, which
  documents the sanctioned set and whose own text says naming only some of them would be a quiet lie).
- Structured transport parsing, never prose scraping (D-10). The model's answer is data to be
  validated, not text to be trusted.
- Guard tests that would actually fail — Phase 20's review found a Critical hiding behind a guard
  that compared two constants instead of the resolved value. Every guard this phase adds must be
  demonstrated failing against the unfixed behaviour.

### Integration Points

- `src/driver/run.rs` — the iteration loop; the ambiguity seam fires where the router returns no rule.
- `src/driver/mod.rs` / `src/cli.rs` — where the goal and the escalation cap arrive on argv.
- `src/journal/mod.rs` — the terminal record and the park-reason taxonomy table.
- Opt-in flow (`driver_confirm` / opt-in gate from Phase 17) — where the file-disclosure and content
  hash belong.

</code_context>

<specifics>
## Specific Ideas

- The threat model is concrete, not theoretical: this tool drives **other people's cloned repos**.
  Their `CLAUDE.md` and `.planning/` are third-party content and can change under a `git pull`.
- REQUIREMENTS.md's out-of-scope list already names "prompt-text guardrails — any safety rule that
  lives only in prompt wording" and "trusting agent self-reports". Both bear directly on this phase.
- Phase 20 shipped `--target-phase` as the goal primitive with `verification_status == 'passed'` as
  goal-met. Phase 21's natural-language goal should reduce **to that**, not to a parallel notion of
  done. Research OQ6 predicted exactly this shape.

</specifics>

<deferred>
## Deferred Ideas

- Gate policy skip/defer/auto — Phase 23 (CTRL-08, DRIVE-07).
- Fleet-level goals spanning multiple projects — out of v2.0.
- Any model involvement in a state the router covers — permanently out of scope, not deferred.

</deferred>
