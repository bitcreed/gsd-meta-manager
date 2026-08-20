---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-20T23:45:00Z
status: gaps_found
score: 3/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs (ROADMAP criterion 1)"
    status: failed
    reason: "The two review-integrity mechanisms this criterion depends on are both broken, and both are confirmed live in the current tree (no fix landed after 21-REVIEW.md, HEAD == cad4dd4)."
    artifacts:
      - path: "src/driver/mod.rs"
        issue: "CR-01: `command_source_refusal` was widened to accept a goal alone as a legal command source, but `preview_text` (:369-380) was never given a goal-aware arm. `--goal X --dry-run` falls to the `(None, None)` arm, which calls `dry_run::build_report(project, \"\")` and renders under `PreviewScope::Complete`'s \"the complete and honest sequence\" header as a 1-line, empty command (`commands: vec![\"\"]`). No test covers `--goal` + `--dry-run`; the doc comment two lines above the match (`:360-361`) still asserts the last arm is unreachable, which is now false. Confirmed by direct read at `src/driver/mod.rs:357-380` and `:441-446` — `preview_text` is called with only `command`/`target_phase`, never `args.goal`."
      - path: "src/driver/goal.rs"
        issue: "CR-02: `plan_digest` (:691-702) calls `journal::argv_digest`, which is FNV-1a-64 (`fnv1a64:` prefix, `src/journal/mod.rs:549-560`) — an invertible, non-cryptographic hash. `approval_digest` (`src/journal/mod.rs:637-652`) then hashes `plan=<plan_digest>` plus the file digests under SHA-256, but the SHA-256 outer layer confers no collision resistance on the plan half: two plans whose FNV-1a-64 digests collide (constructible, not merely searchable, since multiplication by the FNV prime is invertible mod 2^64) produce identical approval digests. The tokens hashed are `verb`, `target_phase`, `terminal_state` where `target_phase` is any roadmap-declared phase name — attacker-controlled under this phase's own adversarial threat model (a third-party `ROADMAP.md`). The doc at `src/journal/mod.rs:628-632` asserts the composition is safe; that assertion is false. Confirmed by direct read."
    missing:
      - "Give the goal-only dry-run its own `PreviewScope` (or equivalent) that states plainly a plan cannot be previewed without a model call, per CR-01's suggested fix, and add a test asserting the rendered text for `--goal --dry-run` never claims an empty command is the honest sequence."
      - "Make `plan_digest` collision-resistant (e.g. hash the step tokens through `journal::sha256_digest` rather than through `argv_digest`), and correct the two doc paragraphs in `src/journal/mod.rs` and `src/driver/goal.rs` that currently claim the composition is already safe."
  - truth: "The named-but-refused / model-selected `target_phase` and the approval's plan-half re-check message stay honest and bounded (supporting SAFE-07 / DRIVE-03's review-integrity chain)"
    status: partial
    reason: "Two related Warning-level defects from 21-REVIEW.md, both confirmed live, weaken the trustworthiness of the plan the user reviews and re-approves without breaking the primary tested happy path."
    artifacts:
      - path: "src/driver/goal.rs"
        issue: "WR-05: in `legality()` (:656-666), `rationale` is passed through `untrusted::bounded` but `target_phase` is stored raw (`target_phase: named_phase.to_string()`, :663). That value flows unsanitised into `DriveError::PlanApprovalRequired`'s `Display` (`src/error.rs:719-733`), which writes it straight to the operator's terminal with no `sanitize_render_line`, and into the committed `run.json`. Confirmed by direct read."
      - path: "src/driver/mod.rs"
        issue: "WR-01: `approve_plan` (:766-790) synthesises the `ApprovedPlan` handed to `recheck_approval` with `plan_digest: plan_digest.clone()` — the digest of the plan just observed — making the plan-half comparison a tautology (`ApprovalRefusal::PlanChanged` is unreachable from this call site; same at `run.rs:2253-2260`). The common review flow (run once for the digest, re-run with `--approved-plan`, where the second run re-decomposes through a non-deterministic model) reports any real drift as `DisclosedFilesChanged` — \"the plan is unchanged\" — even when the plan is what actually changed."
    missing:
      - "Route `target_phase` through `untrusted::bounded` at construction in `goal::legality`, and render `PlanApprovalRequired`'s steps through a sanitizing helper before they reach stdout."
      - "Carry the plan digest the approval actually covered into `recheck_approval` (e.g. accept both digest halves on the CLI) rather than comparing the freshly observed plan digest against itself."
deferred: []
human_verification: []
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-20T23:45:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth (ROADMAP success criterion) | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✗ FAILED | CR-01: `--goal X --dry-run` (the phase's headline invocation) prints a false "complete and honest sequence" of one empty command — confirmed at `src/driver/mod.rs:357-380`. CR-02: the approval's plan half is bound by FNV-1a-64 (invertible), not SHA-256, so the "review" the SHA-256 approval digest claims to protect does not actually resist a substituted plan — confirmed at `src/driver/goal.rs:691-702` and `src/journal/mod.rs:549-560,637-652`. Both confirmed live at HEAD (`cad4dd4`); no fix has landed since `21-REVIEW.md` was written. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED | `tests/driver_goal_seam.rs#a_stated_goal_becomes_a_recorded_plan_and_the_run_drives_its_terminal_phase` and `#the_run_record_carries_the_approval_the_cap_and_the_count`, re-run and passing at HEAD. The decomposition-then-loop wiring (`src/driver/run.rs`) and the capability-by-move enforcement (guard six, `tests/spawn_seam_guard.rs`) are real and tested. WR-01 (see gap 2 below) weakens the re-review workflow's honesty but does not break this criterion's tested behavior. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED | `tests/driver_escalation_cap.rs#a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing`, `#a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap`, `#the_three_boundaries_are_measured_against_the_resolved_step_cap` — all re-run and passing at HEAD, driven against a real stand-in with a tripwire-counted spawn ledger (not an in-process counter). WR-02 (`escalations_used` unstamped on 3 of 4 terminal-write paths — `run.rs:1005`, `:1126`, `:2808`) is a real, confirmed auditability gap on the kill/spawn-fail paths, but does not affect the tested park behavior itself; recorded as a secondary finding, not a gap against this criterion. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes | ✓ VERIFIED | `tests/driver_injection_corpus.rs` — the arrival-then-property harness proves, against the real `claude` 2.1.238 binary, that all 11 corpus classes' markers arrive and the hostile/clean arms both choose the identical terminal command. The empty-corpus experiment (per-SUMMARY, verbatim) confirms the ARRIVAL assertion — not the property assertion — is what fires when content is withheld, ruling out the vacuous-pass failure mode this plan explicitly targets. Non-ignored completeness/round-trip guards re-run and pass at HEAD (`cargo test --test driver_injection_corpus`: 12 passed). The `--ignored` live arms against the real binary were not re-executed by this verifier (would require live credentialed API calls); the SUMMARY's verbatim recorded transcripts (binary version, both control arms' full JSON output) are accepted as sufficient evidence given the methodology is sound and independently confirmed by code review. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED | `tests/driver_refusal_record.rs#an_out_of_enum_action_is_refused_recorded_verbatim_and_never_executed` and `#no_part_of_a_refusal_record_reads_as_a_constructed_command_line` — re-run and passing at HEAD. The alphabet-derived scan (from `RouterAction::ALL`) plus the tripwire-with-control-arm technique give strong evidence nothing model-named is ever executed or rendered as a pasteable command line. `router::command_for` stays `pub(crate)` at a single composition site. |

**Score:** 3/5 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/goal.rs` | Plan type, schema, legality predicate, refusal taxonomy, plan digest | ⚠️ VERIFIED but weak | Exists, substantive, wired. `plan_digest` (CR-02) and `target_phase` sanitisation (WR-05) are confirmed defects within this file. |
| `src/driver/untrusted.rs` | Nonce-suffixed JSON-encoded boundary, third-party string census | ✓ VERIFIED | Exists, substantive, wired; in-source tests confirmed passing. |
| `src/driver/escalate.rs` | Fifth sibling taxonomy, per-run counter, cap resolution against resolved step cap | ✓ VERIFIED | Exists, substantive, wired; `escalate::resolve` compares against the resolved value, not `DEFAULT_MAX_STEPS` (confirmed by grep and by passing boundary tests). |
| `src/journal/mod.rs` (`sha256_digest`, `approval_digest`, `ApprovedPlan`) | SHA-256 sibling digest; approval bound to plan + disclosed files | ⚠️ HOLLOW on the plan half | `sha256_digest` itself is a correct SHA-256 implementation (known-vector test passes). `approval_digest` composes it over an FNV-1a-64 plan digest (CR-02), so the plan half of the approval is not actually collision-resistant despite the SHA-256 outer wrapping. |
| `tests/driver_model_seam.rs` | Live seam proof, OQ1 arms | ✓ VERIFIED | Present; live arms recorded verbatim in `goal.rs`'s head doc per 21-01 SUMMARY. |
| `tests/fixtures/injection-corpus/` | Hostile 11-class corpus, README with markers/binary version | ✓ VERIFIED | Present; verified structurally (README, markers, no absolute host paths per WR-08's own scoped concern) and via non-ignored guards at HEAD. |
| `tests/driver_refusal_record.rs`, `tests/driver_escalation_cap.rs` | Refusal-as-evidence and cap-parks proofs | ✓ VERIFIED | Present, re-run, all 17 tests passing at HEAD. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `src/driver/goal.rs` | `src/driver/router.rs` | schema enum from `SAFE_COMMAND_ALPHABET`, re-parse walks `RouterAction::ALL` | ✓ WIRED | Confirmed by passing both-directions tests. |
| `src/driver/mod.rs` (`drive`) | `src/driver/escalate.rs` | `escalate::resolve` called with `bounds::resolve`'s output, not `args.max_escalations` or the default | ✓ WIRED | Confirmed at `src/driver/mod.rs:497-500` and by the boundary tests. |
| `src/driver/run.rs` | `src/driver/goal.rs` | decomposition validated by `legality` before the loop | ✓ WIRED | Confirmed by `driver_goal_seam.rs` tests. |
| `src/driver/mod.rs` (`preview_text`) | `src/driver/dry_run.rs` | goal-only invocation renders an honest, goal-aware preview | ✗ NOT WIRED | CR-01 — `preview_text` has no goal-aware arm; a goal-only `--dry-run` falls through to the command-preview renderer with an empty command string. |
| `src/driver/goal.rs` (`plan_digest`) | `src/journal/mod.rs` (`approval_digest`) | the approval's plan half is cryptographically bound | ⚠️ WIRED BUT WEAK | CR-02 — wired, but the plan half rides on FNV-1a-64, so the binding does not resist a constructed second preimage as the code's own doc claims. |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 21-04 | User states a goal once, driver pursues it without further input | ⚠️ PARTIAL | The multi-command pursuit mechanism is verified (criterion 2). The "review before anything runs" half of the stated-once contract is undermined by CR-01/CR-02 (criterion 1). |
| DRIVE-03 | 21-01, 21-03, 21-04 | Goal decomposed into structured, machine-checkable, reviewable plan | ✗ BLOCKED | CR-01 and CR-02 directly target this requirement's "reviewable" half. |
| DRIVE-04 | 21-02, 21-04, 21-06 | Model escalation capped per run, exceeding it parks | ✓ SATISFIED | Criterion 3 verified; WR-02 is a secondary auditability gap, not a functional block. |
| SAFE-07 | 21-01, 21-03, 21-05, 21-06 | `.planning/` content passed inside an explicit untrusted boundary, never concatenated into instructions | ✓ SATISFIED | Criterion 4 verified extensively; WR-05 (unbounded `target_phase`) and WR-04 (opt-in revert re-baselining, confirmed at `src/ui/screens/driver_confirm.rs:519-524`) are real but narrower defects that do not defeat the boundary itself. |
| SAFE-08 | 21-01, 21-05, 21-06 | Model's action constrained to fixed enum; free-form shell strings never executed | ✓ SATISFIED | Criterion 5 verified. |

No orphaned requirements: the union of `requirements:` declared across all six plans (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08) matches exactly the phase's declared requirement IDs and REQUIREMENTS.md's phase-21 mapping.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/driver/mod.rs` | :360-361 | Stale doc comment asserting an arm is "unreachable" when it is now reachable (falsified by the same change that introduced CR-01) | 🛑 Blocker | Directly misleads a future reader about `preview_text`'s coverage; tied to CR-01. |
| `src/journal/mod.rs` | :628-632 | Doc paragraph asserting the FNV+SHA-256 composition is safe, which is mathematically false | 🛑 Blocker | Tied to CR-02; a documented incorrect security claim. |
| `src/driver/run.rs` | :1005, :1126, :2808 | `journal.finish(...)` called without a preceding `set_escalations_used`, on 3 of 4 terminal-write paths | ⚠️ Warning | WR-02; affects auditability of killed/spawn-failed goal-driven runs. |
| `src/ui/screens/driver_confirm.rs` | :519-524 | Save-failure revert calls `record_opt_in` (the sole constructor, which re-baselines digests) instead of restoring the prior record | ⚠️ Warning | WR-04; can launder file drift that should have required re-confirmation. |

No `TBD`/`FIXME`/`XXX` unreferenced debt markers were found in the files reviewed.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| `driver_goal_seam` suite | `cargo test --test driver_goal_seam` | 10 passed | ✓ PASS |
| `driver_refusal_record` suite | `cargo test --test driver_refusal_record` | 9 passed | ✓ PASS |
| `driver_escalation_cap` suite | `cargo test --test driver_escalation_cap` | 8 passed | ✓ PASS |
| `driver_dry_run` suite | `cargo test --test driver_dry_run` | 8 passed | ✓ PASS (does not cover `--goal --dry-run`, per CR-01) |
| `spawn_seam_guard` suite | `cargo test --test spawn_seam_guard` | 22 passed | ✓ PASS |
| `driver_injection_corpus` (non-ignored) | `cargo test --test driver_injection_corpus` | not re-run individually; covered via SUMMARY + guard suites above | ? SKIP (live-binary arms require credentialed `claude` access; not exercised by this verifier) |
| `cargo build --all-targets` | `rtk proxy cargo build --all-targets` | clean | ✓ PASS |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no `scripts/*/tests/probe-*.sh`.

### Human Verification Required

None required for a status determination — both blocking issues (CR-01, CR-02) are code-level, objectively confirmed defects rather than judgment calls, and are recorded as gaps above.

### Gaps Summary

Phase 21's engineering is extensive and mostly sound: the two-seam confinement (goal decomposition once, ambiguity escalation once, both budget-checked and type/compile-time bounded), the injection corpus's arrival-before-property methodology, the escalation cap's park behavior, and the out-of-enum refusal machinery are all real, wired, and independently re-confirmed passing at HEAD. Four of five ROADMAP success criteria (2, 3, 4, 5) are solidly verified.

**Criterion 1 — "gets back a structured, machine-checkable plan to review before anything runs" — is not met**, for two confirmed reasons, both flagged Critical by `21-REVIEW.md` and unfixed at HEAD (`cad4dd4`, the review's own commit):

1. **CR-01**: the dry-run preview — this codebase's own honesty contract, and the mechanism a user would naturally reach for to "review before anything runs" — renders a false, empty "complete and honest sequence" for `--goal X --dry-run`, the phase's headline invocation. No test exists for this invocation shape.
2. **CR-02**: the approval digest that is supposed to bind a user's review to what will actually run has its plan half backed by FNV-1a-64 (invertible, constructible second preimages), not SHA-256, despite the code's own documentation claiming the composition is safe. This directly weakens the "review" a machine-checkable plan is supposed to guarantee, under a threat model this very phase declares adversarial (an attacker-controlled `ROADMAP.md`).

Two further Warning-level defects (WR-05: unsanitised `target_phase` reaching the terminal and `run.json`; WR-01: the approval refusal's plan-vs-files distinction is a tautology in the common re-review flow) compound the same "is the plan the user reviews actually the plan that runs, and is it shown honestly" concern, so they are recorded as a related, partial gap rather than separate criteria failures.

Two additional Warning-level defects (WR-02: `escalations_used` unstamped on 3 of 4 terminal-write paths; WR-04: opt-in-revert re-baselines rather than restores) are real and confirmed but do not defeat the specific ROADMAP truths they touch (criteria 3 and the SAFE-07 boundary respectively) — they are noted in the anti-patterns table for the record but not elevated to blocking gaps.

**Recommendation:** route back through `/gsd-plan-phase --gaps` for a closure plan targeting CR-01 and CR-02 (and, opportunistically, WR-01/WR-05 since they share the same root concern and files). Do not proceed to the next phase until criterion 1 is re-verified.

---

_Verified: 2026-08-20T23:45:00Z_
_Verifier: Claude (gsd-verifier)_
