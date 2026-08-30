---
schema_version: 1
open_count: 18
waived_count: 0
fixed_count: 0
total_count: 18
last_updated: 2026-08-30T00:22:46.812Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 18 | deviation | src/ui/screens/driver.rs |  | 18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record | open |  | 2026-07-30T03:39:40.539Z |  |
| 2 | 19 | deviation | src/envelope/mod.rs |  | GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented | open |  | 2026-08-18T17:21:17.313Z |  |
| 3 | 19 | unmet-truth | .planning/phases/19-gitsafe-git-blast-radius-envelope/19-06-SUMMARY.md |  | Phase 19 honesty statement SECTION_ENVELOPE: its three parts are test-pinned but its tone is human_judgment only — no test can assert it reads as candour rather than a hedge | open |  | 2026-08-18T22:36:01.082Z |  |
| 4 | 19 | unmet-truth | src/envelope/hooks.rs |  | The pull-request cap has no git-hook second carrier: a settings file the agent CLI silently ignores leaves SAFE-06 unenforced while every push boundary stays standing (19-05 recorded degradation) | open |  | 2026-08-18T22:36:06.541Z |  |
| 5 | 19 | unrun-verify | src/envelope/advisory.rs |  | probe_protection's 10s budget/kill-at-deadline path has no test driving a real stall — D-35 forbids the network fixture; correctness under a real hang is reasoned, not observed | open |  | 2026-08-18T22:36:07.883Z |  |
| 6 | 20 | stub | src/driver/router.rs |  | 20-01: Decision::GoalMet has no producer until DiskInference records the VERIFICATION frontmatter status (research Pitfall 2) — closed by 20-03 + 20-04 | open |  | 2026-08-19T19:47:27.966Z |  |
| 7 | 20 | stub | src/driver/router.rs |  | 20-01: RouterReason::DependencyUnsatisfied has no producer; belongs to the plan that widens the rule table (20-04) | open |  | 2026-08-19T19:47:28.080Z |  |
| 8 | 20 | stub | src/driver/run.rs |  | 20-01: GOAL_MET_LABEL is unreachable — the one terminal label sourced from neither outcome_label nor the parked: prefix; needs a GoalMet producer (20-04) | open |  | 2026-08-19T19:47:28.196Z |  |
| 9 | 20 | deviation | src/driver/dry_run.rs |  | 20-01: SECTION_COMMANDS and its sibling prose at dry_run.rs:14 and :107-111 still claim the single --command is the complete honest sequence, which the routed loop makes false (research Pitfall 6); a routed preview reports '(routed: chosen per iteration)' so nothing lies about a specific command — 20-02 owns the pinned-text fix | open |  | 2026-08-19T19:47:28.307Z |  |
| 10 | 20 | deviation | tests/driver_reattach.rs |  | Wave-2 post-merge gate: 2 failures, both in driver_reattach. Bisected — FAILS at b6c1ae7 (docs-only, zero phase-20 source), so NOT a phase-20 regression; matches phase 19 deferred-items 'proved pre-existing'. New observation worth acting on: it now fails 3/3 in ISOLATION (0.53s), whereas phase 19 recorded it passing in isolation and failing only under parallel load. The failure rate has increased and the isolation-passes assumption in deferred-items.md is now stale. 930/932 tests pass; failures confined to this one target. | open |  | 2026-08-19T20:32:38.102Z |  |
| 11 | 20 | deviation | src/journal/mod.rs |  | 20-05: the Parked.reason taxonomy table gained a fourth sibling taxonomy (quota_) as a doc-only amendment. journal/mod.rs was outside 20-05's declared files_modified, but its own doc said 'Three sanctioned taxonomies' and its text says naming only some of them would be the same quiet lie the record exists to prevent — so leaving it stale was not an option. Quota is a sibling taxonomy, not a fifth BoundsReason arm: bounds.rs documents CTRL-06 as exactly four detectors, and router.rs belonged to a parallel worktree. | open |  | 2026-08-19T23:26:50.738Z |  |
| 12 | 20 | deviation | src/driver/rate_limit.rs |  | 20-05: research assumption A2 recommended detecting quota exhaustion via terminal_reason.starts_with('api_error'). That is wrong — it would report api_error_overloaded and every future api_error_* as quota exhaustion, telling the user to wait out a 7-day window for a fault a retry clears in seconds. Shipped predicate is contains('rate_limit'), with a test pinning seven non-quota terminal reasons including budget_exhausted (the --max-budget-usd post-turn breaker, a different constraint per D-16). 20-RESEARCH.md A2 is stale. | open |  | 2026-08-19T23:26:50.849Z |  |
| 13 | 21 | deviation | src/driver/escalate.rs |  | 21-02: plan acceptance criterion required escalate::resolve(None, 2) to REFUSE. Implementing it literally makes --max-steps 1 unrunnable — no legal cap exists (0 refused by the zero rule, 1 refused by the step-cap rule), so the refusal could name no remedy and a CTRL-06 test could only be 'fixed' by destroying the property it proves. Implemented instead: a SUPPLIED cap is refused exactly as specified; an UNSUPPLIED default is put through the same comparison and reduced to min(3, max_steps-1). Asymmetry is deliberate and documented in three places plus a test: supplied zero is a seam that looks configured but can never fire; derived zero is the honest consequence of a one-step run. Verified by the orchestrator. | open |  | 2026-08-20T02:45:53.478Z |  |
| 14 | 21 | stub | src/driver/goal.rs |  | 21-03: research Q4 (approval bound to plan AND files) is HALF discharged. The files half is now enforced at the spawn gate with SHA-256; the plan half still rests on goal::plan_digest, which remains FNV-1a and is documented in-tree as not a security control. Binding the two belongs with the loop wiring in 21-04/21-06. Recorded so it is not read as closed. | open |  | 2026-08-20T03:25:51.109Z |  |
| 15 | 21 | deviation | src/config.rs |  | 21-03: added #[serde(flatten)] extra to DriverOptIn. Empirically proven necessary — a probe against the real load_config/save_config showed entry-level unknown keys survive a round-trip but keys nested inside driver_opt_in are SILENTLY DELETED, which made the plan's own round-trip acceptance criterion unpassable and T-21-20's mitigation a paper one at that depth. Fail-safe: a dropped prompt_inputs reads as absent, absent means re-confirm, so the failure mode is a spurious re-confirmation after a downgrade, not a silent approval. | open |  | 2026-08-20T03:25:51.228Z |  |
| 16 | 21 | stub | src/driver/router.rs |  | 21-04 STRUCTURAL FINDING affecting ROADMAP criterion 3: router::decide cannot return NoRule for ANY project state this tree's reader produces from disk. The rule table covers five statuses, gate_for intercepts partial and every executed verification status, and complete requires passed which is_goal_met answers first; the one uncovered state violates the reader's own invariant (src/state_reader/disk_status.rs:670). Consequences: (1) the ambiguity seam is wired, order-pinned and unit-tested but has NO reachable production path; (2) since goal decomposition is the only escalation that can occur, a run makes at most 1 escalation, so escalation_cap_reached is UNREACHABLE end-to-end and no run can park under it on disk. This is arguably a good property — it means the rule table is complete — but criterion 3's cap must be verified as 'mechanism correct and unit-proven, no production path reaches it', NOT as end-to-end proven. 21-06 cannot discharge 21-02's E7 deferral without deliberately widening the reader, removing a rule-table row, or accepting a fixture that writes an inference the reader would never produce. Do not let a fixture of that third kind be mistaken for end-to-end proof. | open |  | 2026-08-20T04:50:50.557Z |  |
| 17 | 19 | deviation | tests/envelope_wrapper_class.rs |  | MIN_UNREADABLE_FORGE_SLOT_CASES was 50 against a stated arithmetic of 68 while the loop generates 40 — unreachable by construction; corrected to 40 by plan 19-17 | open |  | 2026-08-30T00:22:46.676Z |  |
| 18 | 19 | deviation | tests/envelope_literal_decision.rs |  | 19-16's the_marked_payload_splice_... pinned exit 0 despite name/comment/SUMMARY saying it asserts nothing post-fix; guard discharged and re-pinned by 19-17 without weakening clause 2(a) | open |  | 2026-08-30T00:22:46.812Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "18",
    "file": "src/ui/screens/driver.rs",
    "line": null,
    "description": "18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-30T03:39:40.539Z",
    "resolved_at": null
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "19",
    "file": "src/envelope/mod.rs",
    "line": null,
    "description": "GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T17:21:17.313Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "unmet-truth",
    "phase": "19",
    "file": ".planning/phases/19-gitsafe-git-blast-radius-envelope/19-06-SUMMARY.md",
    "line": null,
    "description": "Phase 19 honesty statement SECTION_ENVELOPE: its three parts are test-pinned but its tone is human_judgment only — no test can assert it reads as candour rather than a hedge",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T22:36:01.082Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "unmet-truth",
    "phase": "19",
    "file": "src/envelope/hooks.rs",
    "line": null,
    "description": "The pull-request cap has no git-hook second carrier: a settings file the agent CLI silently ignores leaves SAFE-06 unenforced while every push boundary stays standing (19-05 recorded degradation)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T22:36:06.541Z",
    "resolved_at": null
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "19",
    "file": "src/envelope/advisory.rs",
    "line": null,
    "description": "probe_protection's 10s budget/kill-at-deadline path has no test driving a real stall — D-35 forbids the network fixture; correctness under a real hang is reasoned, not observed",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-18T22:36:07.883Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "20-01: Decision::GoalMet has no producer until DiskInference records the VERIFICATION frontmatter status (research Pitfall 2) — closed by 20-03 + 20-04",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:27.966Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "20-01: RouterReason::DependencyUnsatisfied has no producer; belongs to the plan that widens the rule table (20-04)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:28.080Z",
    "resolved_at": null
  },
  {
    "id": 8,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/run.rs",
    "line": null,
    "description": "20-01: GOAL_MET_LABEL is unreachable — the one terminal label sourced from neither outcome_label nor the parked: prefix; needs a GoalMet producer (20-04)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:28.196Z",
    "resolved_at": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "20",
    "file": "src/driver/dry_run.rs",
    "line": null,
    "description": "20-01: SECTION_COMMANDS and its sibling prose at dry_run.rs:14 and :107-111 still claim the single --command is the complete honest sequence, which the routed loop makes false (research Pitfall 6); a routed preview reports '(routed: chosen per iteration)' so nothing lies about a specific command — 20-02 owns the pinned-text fix",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:28.307Z",
    "resolved_at": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "20",
    "file": "tests/driver_reattach.rs",
    "line": null,
    "description": "Wave-2 post-merge gate: 2 failures, both in driver_reattach. Bisected — FAILS at b6c1ae7 (docs-only, zero phase-20 source), so NOT a phase-20 regression; matches phase 19 deferred-items 'proved pre-existing'. New observation worth acting on: it now fails 3/3 in ISOLATION (0.53s), whereas phase 19 recorded it passing in isolation and failing only under parallel load. The failure rate has increased and the isolation-passes assumption in deferred-items.md is now stale. 930/932 tests pass; failures confined to this one target.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T20:32:38.102Z",
    "resolved_at": null
  },
  {
    "id": 11,
    "kind": "deviation",
    "phase": "20",
    "file": "src/journal/mod.rs",
    "line": null,
    "description": "20-05: the Parked.reason taxonomy table gained a fourth sibling taxonomy (quota_) as a doc-only amendment. journal/mod.rs was outside 20-05's declared files_modified, but its own doc said 'Three sanctioned taxonomies' and its text says naming only some of them would be the same quiet lie the record exists to prevent — so leaving it stale was not an option. Quota is a sibling taxonomy, not a fifth BoundsReason arm: bounds.rs documents CTRL-06 as exactly four detectors, and router.rs belonged to a parallel worktree.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T23:26:50.738Z",
    "resolved_at": null
  },
  {
    "id": 12,
    "kind": "deviation",
    "phase": "20",
    "file": "src/driver/rate_limit.rs",
    "line": null,
    "description": "20-05: research assumption A2 recommended detecting quota exhaustion via terminal_reason.starts_with('api_error'). That is wrong — it would report api_error_overloaded and every future api_error_* as quota exhaustion, telling the user to wait out a 7-day window for a fault a retry clears in seconds. Shipped predicate is contains('rate_limit'), with a test pinning seven non-quota terminal reasons including budget_exhausted (the --max-budget-usd post-turn breaker, a different constraint per D-16). 20-RESEARCH.md A2 is stale.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-19T23:26:50.849Z",
    "resolved_at": null
  },
  {
    "id": 13,
    "kind": "deviation",
    "phase": "21",
    "file": "src/driver/escalate.rs",
    "line": null,
    "description": "21-02: plan acceptance criterion required escalate::resolve(None, 2) to REFUSE. Implementing it literally makes --max-steps 1 unrunnable — no legal cap exists (0 refused by the zero rule, 1 refused by the step-cap rule), so the refusal could name no remedy and a CTRL-06 test could only be 'fixed' by destroying the property it proves. Implemented instead: a SUPPLIED cap is refused exactly as specified; an UNSUPPLIED default is put through the same comparison and reduced to min(3, max_steps-1). Asymmetry is deliberate and documented in three places plus a test: supplied zero is a seam that looks configured but can never fire; derived zero is the honest consequence of a one-step run. Verified by the orchestrator.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-20T02:45:53.478Z",
    "resolved_at": null
  },
  {
    "id": 14,
    "kind": "stub",
    "phase": "21",
    "file": "src/driver/goal.rs",
    "line": null,
    "description": "21-03: research Q4 (approval bound to plan AND files) is HALF discharged. The files half is now enforced at the spawn gate with SHA-256; the plan half still rests on goal::plan_digest, which remains FNV-1a and is documented in-tree as not a security control. Binding the two belongs with the loop wiring in 21-04/21-06. Recorded so it is not read as closed.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-20T03:25:51.109Z",
    "resolved_at": null
  },
  {
    "id": 15,
    "kind": "deviation",
    "phase": "21",
    "file": "src/config.rs",
    "line": null,
    "description": "21-03: added #[serde(flatten)] extra to DriverOptIn. Empirically proven necessary — a probe against the real load_config/save_config showed entry-level unknown keys survive a round-trip but keys nested inside driver_opt_in are SILENTLY DELETED, which made the plan's own round-trip acceptance criterion unpassable and T-21-20's mitigation a paper one at that depth. Fail-safe: a dropped prompt_inputs reads as absent, absent means re-confirm, so the failure mode is a spurious re-confirmation after a downgrade, not a silent approval.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-20T03:25:51.228Z",
    "resolved_at": null
  },
  {
    "id": 16,
    "kind": "stub",
    "phase": "21",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "21-04 STRUCTURAL FINDING affecting ROADMAP criterion 3: router::decide cannot return NoRule for ANY project state this tree's reader produces from disk. The rule table covers five statuses, gate_for intercepts partial and every executed verification status, and complete requires passed which is_goal_met answers first; the one uncovered state violates the reader's own invariant (src/state_reader/disk_status.rs:670). Consequences: (1) the ambiguity seam is wired, order-pinned and unit-tested but has NO reachable production path; (2) since goal decomposition is the only escalation that can occur, a run makes at most 1 escalation, so escalation_cap_reached is UNREACHABLE end-to-end and no run can park under it on disk. This is arguably a good property — it means the rule table is complete — but criterion 3's cap must be verified as 'mechanism correct and unit-proven, no production path reaches it', NOT as end-to-end proven. 21-06 cannot discharge 21-02's E7 deferral without deliberately widening the reader, removing a rule-table row, or accepting a fixture that writes an inference the reader would never produce. Do not let a fixture of that third kind be mistaken for end-to-end proof.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-20T04:50:50.557Z",
    "resolved_at": null
  },
  {
    "id": 17,
    "kind": "deviation",
    "phase": "19",
    "file": "tests/envelope_wrapper_class.rs",
    "line": null,
    "description": "MIN_UNREADABLE_FORGE_SLOT_CASES was 50 against a stated arithmetic of 68 while the loop generates 40 — unreachable by construction; corrected to 40 by plan 19-17",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-30T00:22:46.676Z",
    "resolved_at": null
  },
  {
    "id": 18,
    "kind": "deviation",
    "phase": "19",
    "file": "tests/envelope_literal_decision.rs",
    "line": null,
    "description": "19-16's the_marked_payload_splice_... pinned exit 0 despite name/comment/SUMMARY saying it asserts nothing post-fix; guard discharged and re-pinned by 19-17 without weakening clause 2(a)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-30T00:22:46.812Z",
    "resolved_at": null
  }
]
````
