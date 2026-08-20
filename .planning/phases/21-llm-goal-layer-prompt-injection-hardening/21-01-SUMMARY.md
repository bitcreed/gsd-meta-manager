---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 01
subsystem: infra
tags: [claude-cli, structured-output, json-schema, prompt-injection, spawn-profile, uuid, serde_json]

requires:
  - phase: 20-deterministic-decision-router-and-run-bounds
    provides: "router::SAFE_COMMAND_ALPHABET, RouterAction + ALL + verb, is_goal_met, bounds::resolve, the four sibling reason taxonomies"
  - phase: 19-gitsafe-git-and-blast-radius-envelope
    provides: "the PreToolUse envelope hook the seam profile must not disarm, and the one spawn closure that builds the child environment"
  - phase: 15-transport-foundation-duplex-stream-json-executor
    provides: "build_argv, ExecutionOptions, the single audited spawn seam, ResultMessage and its tolerant-parse posture"
provides:
  - "executor::SpawnProfile — a second argv/env profile on the ONE existing spawn seam, matched exhaustively with no wildcard"
  - "The empirically answered OQ1: goal decomposition does NOT need file bodies, so the empty-tool seam survives"
  - "driver::untrusted — the nonce-suffixed, JSON-encoded untrusted-content boundary and the enumerated third-party string census"
  - "driver::goal — the JSON Schema built from the router alphabet, the RouterAction re-parse, the plan type, the legality predicate, the goal_ refusal taxonomy and the plan digest"
  - "The arrival-assertion pattern: the first tests in this tree that assert positively about what reached the model"
  - "Six coupling guards in tests/spawn_seam_guard.rs, each observed red against the unfixed behaviour"
affects: [21-02, 21-03, 21-04, 21-05, 21-06, 22-container-execution-target]

actuals:
  tokens: 33518
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "SpawnProfile as an exhaustive discriminant on the one spawn seam, mirroring ExecutionTarget"
    - "Model-side arrival assertion before any negative injection claim"
    - "Third-party string census as data, guarded by a both-directions source scan"
    - "Machine-checkable terminal state as a wire enum rather than prose"

key-files:
  created:
    - src/driver/goal.rs
    - src/driver/untrusted.rs
    - tests/driver_model_seam.rs
  modified:
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/executor/stream_json.rs
    - src/driver/mod.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "OQ1 answered NO: goal decomposition does not need file bodies. Arm A succeeded on typed state alone, so the empty-tool seam survives and the untrusted surface stays at six enumerated strings."
  - "The goal's target phase is the LAST step's phase, not the first's — both live arms correctly opened on a prerequisite phase, and the first version of the assertion was wrong, not the seam."
  - "terminal_state became a wire ENUM built from TerminalState::ALL, with the model's prose moved to a separate `rationale` field no predicate reads. Reducing a stopping condition by pattern-matching English would be the prose-scraping D-10 forbids, wearing a schema."
  - "The third-party string enumeration is the WHOLE String-field census with a per-field Disposition, not just the six prose fields — omission and 'we decided it is safe' are indistinguishable to a later reader, and only one of them is a decision."
  - "plan_digest reuses journal::argv_digest rather than adding a third hasher, and is explicitly drift detection rather than a security control; the fnv1a64: prefix is what makes 21-03's sha256: upgrade free."
  - "The spawn-closure comment was rewritten in the Task 1 commit rather than Task 3, because that is the commit whose code falsified it (the dry_run.rs:78-83 precedent)."

patterns-established:
  - "Model-side arrival assertion: assert what reached the model (tools == [StructuredOutput], mcp_servers == []) BEFORE concluding anything from what it said. No prior test in this tree asserted model-side arrival."
  - "Live-arm loud failure: an unavailable binary or session FAILS the ignored test with a diagnostic rather than skipping, because an availability check that silently passes is the vacuity class the phase exists to prevent."
  - "Guard-of-the-guard over synthetic text: each new source-scanning guard has a control arm proving the scanner fires on code and stays silent on a doc comment, so the declined alternative remains documentable."
  - "Both-directions census diff: a String field with no enumeration entry and an enumeration entry with no field are both failures."

requirements-completed: [DRIVE-01, DRIVE-03, SAFE-07, SAFE-08]

coverage:
  - id: D1
    description: "A plain-language goal handed to the model seam comes back as a schema-conformant payload whose every named command re-parses to a router::RouterAction, proven live against the real claude binary"
    requirement: "DRIVE-01"
    verification:
      - kind: e2e
        ref: "tests/driver_model_seam.rs#oq1_arm_a_typed_state_only (cargo test --test driver_model_seam -- --ignored)"
        status: pass
      - kind: e2e
        ref: "tests/driver_model_seam.rs#oq1_arm_b_typed_state_plus_roadmap_prose_in_the_untrusted_boundary"
        status: pass
    human_judgment: false
  - id: D2
    description: "OQ1 answered empirically by two live arms recorded in-tree; the answer decides whether the empty-tool seam survives"
    verification:
      - kind: e2e
        ref: "tests/driver_model_seam.rs#oq1_arm_a_typed_state_only — recorded verbatim in src/driver/goal.rs head doc"
        status: pass
    human_judgment: true
    rationale: "Both arms are single samples of a non-deterministic system. They establish capability, not a success rate, and a human should decide whether one sample per arm is enough evidence to rest the phase's shape on."
  - id: D3
    description: "Untrusted content containing the boundary's own closing form does not terminate the boundary: the body is JSON-encoded and the tag carries a per-call CSPRNG nonce"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/driver/untrusted.rs#content_carrying_the_closing_form_does_not_terminate_the_boundary"
        status: pass
      - kind: unit
        ref: "src/driver/untrusted.rs#two_calls_with_identical_inputs_produce_different_nonces"
        status: pass
    human_judgment: false
  - id: D4
    description: "A third-party string rendered into a record is length-bounded and control-character-stripped, truncates on a UTF-8 character boundary, and is marked as truncated"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/driver/untrusted.rs#truncation_inside_a_multibyte_character_lands_on_a_character_boundary"
        status: pass
      - kind: unit
        ref: "src/driver/untrusted.rs#control_characters_are_stripped_so_one_value_cannot_become_two_record_lines"
        status: pass
    human_judgment: false
  - id: D5
    description: "The seam's spawn carries an empty tool set, an explicit CLAUDE.md suppression and a pinned structured-output retry count, and never the hook-disabling safe-mode flag — asserted against the value build_argv and the env closure actually produce"
    requirement: "SAFE-07"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_seam_profile_couples_the_empty_tool_set_to_the_schema"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#no_executable_line_in_src_passes_the_hook_disabling_flag"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_permitted_mcp_set_stays_empty_on_both_profiles"
        status: pass
      - kind: e2e
        ref: "tests/driver_model_seam.rs#the_structured_output_retry_pin_is_honoured_rather_than_assumed"
        status: pass
    human_judgment: true
    rationale: "The CLAUDE.md suppression half has NO on-the-wire signal — no init-envelope field reports whether it took effect. The argv/env shape is asserted, but the behavioural proof is deferred to plan 21-05's corpus fixture. A human should confirm that deferral is acceptable rather than treating SAFE-07's suppression control as already proven."
  - id: D6
    description: "Every step of a decomposed plan names a command in SAFE_COMMAND_ALPHABET, a target phase that is a plain path component present in the roadmap, and a terminal state that reduces to router::is_goal_met; a goal that cannot be so reduced is refused, naming the part that could not be reduced"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/driver/goal.rs#a_terminal_state_that_does_not_reduce_to_the_goal_met_predicate_is_refused"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#a_target_phase_shaped_like_a_traversal_is_refused_by_the_shared_checker"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#a_plan_longer_than_the_resolved_step_cap_is_refused"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#every_refusal_arm_is_prefixed_and_the_constants_and_arms_match_both_ways"
        status: pass
    human_judgment: false
  - id: D7
    description: "The model's chosen action is constrained to the fixed enum; the schema is defence in depth and the Rust re-parse is the control; a named-but-refused string is recorded verbatim and never rendered as a command line"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/driver/goal.rs#the_schema_command_enum_and_the_safe_alphabet_match_in_both_directions"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#a_command_outside_the_alphabet_is_refused_and_names_the_string_verbatim"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#a_near_miss_is_refused_rather_than_repaired"
        status: pass
    human_judgment: false
  - id: D8
    description: "Every String-typed field on ProjectState and RoadmapPhase is classified in the untrusted enumeration, failing in both directions"
    requirement: "SAFE-07"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#every_free_string_field_that_could_reach_a_prompt_is_enumerated"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_free_string_field_parser_distinguishes_payloads_from_map_keys"
        status: pass
    human_judgment: false

duration: 78min
completed: 2026-08-19
status: complete
---

# Phase 21 Plan 01: The Model Seam and the OQ1 Gate Summary

**A plain-language goal now reaches a model through a second argv/env profile on the one audited spawn seam and comes back as a validated `RouterAction` plan — proven live against `claude` 2.1.236, with OQ1 answered empirically: goal decomposition does NOT need file bodies, so the empty-tool seam survives.**

## Performance

- **Duration:** ~78 min
- **Tasks:** 3 of 3
- **Files created:** 3
- **Files modified:** 5
- **Net:** +2,890 lines

## Accomplishments

- **OQ1 answered, and the phase gate cleared.** Both live arms produced schema-conformant four-step plans that the *shipped* `legality` predicate accepted, both terminating on phase 22 — the human choice. Arm A did it on typed state alone, which is the finding the rest of the phase rests on.
- **The retry pin was exercised rather than assumed.** Research read `MAX_STRUCTURED_OUTPUT_RETRIES` out of the binary's registry and never ran it. An unsatisfiable schema forced the validation loop and produced exactly **1** structured-output call. The pin is honoured; DRIVE-04's escalation count will not under-report by fivefold.
- **The seam profile landed on the ONE existing spawn seam**, with no second spawn site, and the executor profile's argv is asserted byte-identical to what it produced before.
- **The `--safe-mode` trap was avoided and then mechanically closed.** The env var is set directly; a source-scanning guard now fails on any executable line under `src/` that passes the hook-disabling flag, with a control arm proving the scanner still permits documenting the declined alternative in a comment.
- **The arrival-assertion pattern is established.** PATTERNS.md searched all 27 integration test files and found none asserting model-side arrival. Both OQ1 arms assert `tools == ["StructuredOutput"]` and `mcp_servers == []` *before* concluding anything from what the model said.

## Task Commits

1. **Task 1 (tracer): The model seam, wired through every layer, and the OQ1 gate** — `fdb050b` (feat)
2. **Task 2: The plan type, its legality predicate, and the refusal taxonomy** — `93f93b1` (feat)
3. **Task 3: The coupling guards** — `5f0b880` (test)

## Files Created/Modified

- `src/executor/mod.rs` — `SpawnProfile` (`Executor` / `ModelSeam { json_schema }`) and the `ExecutionOptions::profile` field, defaulting to `Executor`.
- `src/executor/claude.rs` — `build_argv`'s exhaustive seam arm (`--tools ""`, inline `--json-schema`); the spawn closure's two seam-only env vars; the falsified comment rewritten.
- `src/executor/stream_json.rs` — `ResultMessage::structured_output: Option<Value>`, following `permission_denials`.
- `src/driver/untrusted.rs` (NEW) — the boundary constructor, the third-party string census, `MAX_UNTRUSTED_FIELD_CHARS` and character-boundary truncation.
- `src/driver/goal.rs` (NEW) — the schema, the re-parse, `TerminalState`, `PlanStep`/`GoalPlan`, `legality`, `GoalReason`/`GoalRefusal`, `plan_digest`, and the OQ1 record in its head doc.
- `src/driver/mod.rs` — both module declarations, with the purity rationale the sibling modules use.
- `tests/driver_model_seam.rs` (NEW) — 4 pure argv assertions + 3 `#[ignore]`d live arms.
- `tests/spawn_seam_guard.rs` — 6 new guards (7 → 14 tests).

## Guards, and what each did against the UNFIXED behaviour

Every guard was run red before being trusted. This is the Phase 20 lesson applied: its review found a Critical hiding behind a guard that compared two constants instead of the resolved value.

| Guard | Injected defect | Observed |
|---|---|---|
| `the_seam_profile_couples_the_empty_tool_set_to_the_schema` | removed `push(&mut argv, "")` after `--tools` | FAILED — `--tools` absent from the returned argv |
| same, byte-identity half | renamed `--permission-mode` in the executor arm | FAILED — executor argv changed |
| `no_executable_line_in_src_passes_the_hook_disabling_flag` | added the assembled flag to a `push` in the seam arm | FAILED, naming the file and line |
| `the_hook_disabling_scan_fires_on_code_and_not_on_a_comment` | — (control arm, both directions in one test) | fires on a synthetic executable line, silent on a synthetic doc comment |
| `the_permitted_mcp_set_stays_empty_on_both_profiles` | swapped `--strict-mcp-config` for `--mcp-config /tmp/x.json` | FAILED on both profiles |
| `every_free_string_field_that_could_reach_a_prompt_is_enumerated` | added `ProjectState::newly_added_prose` | FAILED — `Unenumerated: ["ProjectState::newly_added_prose"]` |
| same, stale direction | added a `removed_last_year` entry to the enumeration | FAILED — `Stale: ["ProjectState::removed_last_year"]` |
| `the_free_string_field_parser_distinguishes_payloads_from_map_keys` | — (control arm over synthetic struct text) | finds `String`/`Option<String>`/`Vec<String>`, not a `HashMap` key, a comment, or a field past the closing brace |
| `a_terminal_state_that_does_not_reduce_to_the_goal_met_predicate_is_refused` | made the predicate accept any terminal state | FAILED — `"when the maintainer is happy with it"` accepted, no refusal produced |
| `a_plan_longer_than_the_resolved_step_cap_is_refused` | compared against `bounds::DEFAULT_MAX_STEPS` | FAILED — a 3-step plan accepted under `bounds::resolve(Some(2), None)`. **The Phase 20 Critical, reproduced exactly.** |
| `the_spawn_closure_comment_no_longer_claims_one_variable_is_set` | (pinned-honesty; stale text spelled out verbatim so it cannot compare the constant with itself) | passes only while the plural replacement and the no-on-the-wire-signal statement are both present |

Guard-one's coupling assertion and guard-three's MCP assertion are additionally duplicated in `tests/driver_model_seam.rs`, deliberately: the seam file is where a reader looks for the seam's shape, and the guard file is where a reader looks for what must not drift.

## Decisions Made

1. **`terminal_state` is a wire enum, not prose.** The plan required the terminal state to reduce to `router::is_goal_met`. Reducing free prose to a predicate would mean pattern-matching English — the screen-scraping D-10 forbids, wearing a schema. The enum is built from `TerminalState::ALL`, whose single arm delegates to `router::is_goal_met` rather than re-deriving the comparison. The model's prose moved to a separate `rationale` field that no predicate reads and that `plan_digest` excludes, so an approval does not expire on a rewording.

2. **The enumeration is the whole `String`-field census, not just the six prose fields.** Research named six free-text fields; the structs declare eleven `String`-shaped fields between them. Naming only six would have made guard four fail or forced a scope carve-out to argue about. Each entry now carries a `Disposition` — `UntrustedProse` (the six, shown only inside a boundary) or `TypedIdentifier` (phase numbers, plan ids). Omitting a field and deciding it is safe are indistinguishable to a later reader; only one of them is a decision.

3. **`plan_digest` reuses `journal::argv_digest`.** No third hasher. It is FNV-1a and therefore drift detection, not a security control — stated plainly in its doc, because plan 21-03 binds an approval to it and C-4's whole point is that a security affordance backed by a non-security hash is dishonest. The `fnv1a64:` prefix makes 21-03's `sha256:` upgrade cost no migration.

4. **The falsified comment was rewritten in Task 1's commit, not Task 3's.** The plan placed it in Task 3, but the `dry_run.rs:78-83` precedent requires the rewrite to ride with the code that falsified it — which was Task 1. Task 3 added the pinned-honesty *test* that keeps it rewritten.

## Deviations from Plan

### 1. [Rule 1 — Bug] The OQ1 arms asserted on the wrong step

- **Found during:** Task 1, on the first live run.
- **Issue:** `assert_plan_is_legal_and_return_target` returned the **first** step's phase and compared it to the human's choice. Both arms failed with `left: "21", right: "22"`.
- **Diagnosis:** the seam was right and the assertion was wrong. Both plans correctly opened on phase 21 — `planned`, unfinished, and standing between the run and 22 — before turning to the phase the goal named. The goal's target phase is the phase whose terminal state satisfies the goal: the **last** step's.
- **Fix:** return the last step's phase; document why in the helper's doc and in `goal.rs`'s head doc, because anything reading "which phase is this goal about" off step zero will be wrong the moment a prerequisite exists.
- **Committed in:** `fdb050b`.
- **Note:** this is recorded rather than quietly corrected because moving a failing assertion is exactly the shape of goalpost-moving that should be visible.

### 2. [Rule 1 — Bug] The truncation fixture was vacuous

- **Found during:** Task 1, caught by the fixture's own precondition assertion.
- **Issue:** the multi-byte truncation test used a 4-byte emoji. `MAX_UNTRUSTED_FIELD_CHARS` is 200, a multiple of 4, so byte index 200 landed cleanly on a character boundary — the test would have passed against a byte-slicing implementation and proved nothing.
- **Fix:** switched to a 3-byte character (`€`), where 200 lands mid-character. The precondition assertion that caught it is retained and its comment now records the near-miss.
- **Committed in:** `fdb050b`.

### 3. [Rule 3 — Blocking] Two new `--all-targets` clippy lints

- **Found during:** Task 3 verification.
- **Issue:** an in-source test used `field_reassign_with_default`, taking the `--all-targets` baseline from 5 to 7.
- **Fix:** struct-update syntax. Baseline restored to exactly the 5 pre-existing lints in their recorded locations.
- **Committed in:** `5f0b880`.

### 4. [Planned scope] The schema changed in Task 2, so the OQ1 arms were re-run

Task 2's terminal-state decision changed the wire schema, which would have made Task 1's recorded evidence describe a shape that no longer exists. Both arms were re-run against the new schema and the head doc's verbatim record updated. The arms now also feed their payload to the shipped `legality` predicate rather than re-implementing its checks, so they assert the driver would actually accept what came back — a strictly stronger claim than the plan asked for.

---

**Total deviations:** 3 auto-fixed (2 × Rule 1, 1 × Rule 3) + 1 in-scope rework.
**Impact:** no scope creep. Two of the three were vacuity or correctness defects in the tests themselves, which is the class this phase exists to catch.

## Issues Encountered

- **`tests/driver_reattach.rs` intermittency** — pre-existing and out of scope, as the plan states. Not touched, not investigated. The full suite ran green on every verification pass here.
- **`git clean`/blanket-reset avoidance during red-checks** — every injected defect was reverted from an explicit per-file backup copy rather than through a working-tree reset, per the worktree prohibition.

## Known Stubs

None. Every symbol this plan created is implemented and exercised.

Two honest limits, recorded because they are limits rather than stubs:

1. **`CLAUDE.md` suppression has no behavioural proof yet.** There is no field on this transport reporting whether it took effect — the init envelope covers the tool set and the MCP list and says nothing about it. This plan asserts the argv/env shape and states the absence explicitly in the spawn closure and in a pinned-honesty test. The behavioural proof is plan **21-05**'s corpus fixture. Nothing here claims SAFE-07's suppression control is already proven.
2. **All three live arms are single samples** of a non-deterministic system. They establish capability, not a success rate. The design deliberately does not rest on model behaviour — `parse_action` and `legality` refuse regardless — but the OQ1 *finding* rests on one sample per arm.

## Next Phase Readiness

Ready for 21-02 onward. What the next plans inherit:

- `SpawnProfile::ModelSeam` is the only way to reach a model, and a third profile is a compile error at two sites.
- `goal::escalation_schema`, `goal::parse_action`, `goal::legality`, `goal::plan_digest` and the `goal_` taxonomy are complete and tested.
- `untrusted::untrusted_block`, `untrusted::bounded` and the census are available; the seam does **not** require the boundary for decomposition (OQ1), so 21-05 can use it as a hostile-content carrier rather than as a load-bearing input.
- **For 21-03:** `plan_digest` is FNV-1a. Binding an approval to it is drift detection only. C-4's `sha256:` upgrade lands there and the prefix convention makes it migration-free.
- **For 21-05:** the arrival-assertion pattern and the loud-failure posture for live arms are established in `tests/driver_model_seam.rs`; the corpus fixture should follow both. A corpus test with no arrival proof is not evidence.

## Self-Check: PASSED

- Files claimed created, verified present: `src/driver/goal.rs`, `src/driver/untrusted.rs`, `tests/driver_model_seam.rs`, `21-01-SUMMARY.md`.
- Commits claimed, verified in `git log`: `fdb050b`, `93f93b1`, `5f0b880`.
- `cargo build`, `cargo test`, `cargo clippy -- -D warnings`: clean.
- `cargo clippy --all-targets -- -D warnings`: exactly the 5 known pre-existing lints, in their recorded locations.
- `cargo test --test driver_model_seam -- --ignored`: 3 passed (both OQ1 arms and the retry pin).
- `cargo test --test spawn_seam_guard`: 14 passed, up from 7.
- `rtk proxy grep -c 'safe-mode' tests/spawn_seam_guard.rs`: 0.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-19*
