---
schema_version: 1
open_count: 0
waived_count: 16
fixed_count: 3
total_count: 19
last_updated: 2026-10-28T21:21:27.245Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 18 | deviation | src/ui/screens/driver.rs |  | 18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record | waived | obsolete (closed, not fixed): the drive-loop injection path in src/ui/screens/driver.rs was deleted in Phase 31; Phase 30 steers via DIRECTION.md control instead [inferred — cleanup 2026-10-28] | 2026-07-30T03:39:40.539Z | 2026-10-28T21:21:24.706Z |
| 2 | 19 | deviation | src/envelope/mod.rs |  | GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented | waived | waived, intentional: GSD_MM_ENVELOPE_ROOT test override is fail-closed by construction and still present (src/envelope/mod.rs:113-120); residual limit documented in-tree [inferred — cleanup 2026-10-28] | 2026-08-18T17:21:17.313Z | 2026-10-28T21:21:26.951Z |
| 3 | 19 | unmet-truth | .planning/phases/19-gitsafe-git-blast-radius-envelope/19-06-SUMMARY.md |  | Phase 19 honesty statement SECTION_ENVELOPE: its three parts are test-pinned but its tone is human_judgment only — no test can assert it reads as candour rather than a hedge | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (SECTION_ENVELOPE / src/envelope/advisory.rs) [inferred — cleanup 2026-10-28] | 2026-08-18T22:36:01.082Z | 2026-10-28T21:21:24.854Z |
| 4 | 19 | unmet-truth | src/envelope/hooks.rs |  | The pull-request cap has no git-hook second carrier: a settings file the agent CLI silently ignores leaves SAFE-06 unenforced while every push boundary stays standing (19-05 recorded degradation) | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/envelope/hooks.rs; push is now outside the sandbox, Phase 28 LNCH-06) [inferred — cleanup 2026-10-28] | 2026-08-18T22:36:06.541Z | 2026-10-28T21:21:25.007Z |
| 5 | 19 | unrun-verify | src/envelope/advisory.rs |  | probe_protection's 10s budget/kill-at-deadline path has no test driving a real stall — D-35 forbids the network fixture; correctness under a real hang is reasoned, not observed | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (probe_protection in src/envelope/advisory.rs) [inferred — cleanup 2026-10-28] | 2026-08-18T22:36:07.883Z | 2026-10-28T21:21:25.156Z |
| 6 | 20 | stub | src/driver/router.rs |  | 20-01: Decision::GoalMet has no producer until DiskInference records the VERIFICATION frontmatter status (research Pitfall 2) — closed by 20-03 + 20-04 | fixed |  | 2026-08-19T19:47:27.966Z | 2026-10-28T21:21:26.649Z |
| 7 | 20 | stub | src/driver/router.rs |  | 20-01: RouterReason::DependencyUnsatisfied has no producer; belongs to the plan that widens the rule table (20-04) | fixed |  | 2026-08-19T19:47:28.080Z | 2026-10-28T21:21:26.799Z |
| 8 | 20 | stub | src/driver/run.rs |  | 20-01: GOAL_MET_LABEL is unreachable — the one terminal label sourced from neither outcome_label nor the parked: prefix; needs a GoalMet producer (20-04) | waived | obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop run.rs GOAL_MET_LABEL) [inferred — cleanup 2026-10-28] | 2026-08-19T19:47:28.196Z | 2026-10-28T21:21:25.302Z |
| 9 | 20 | deviation | src/driver/dry_run.rs |  | 20-01: SECTION_COMMANDS and its sibling prose at dry_run.rs:14 and :107-111 still claim the single --command is the complete honest sequence, which the routed loop makes false (research Pitfall 6); a routed preview reports '(routed: chosen per iteration)' so nothing lies about a specific command — 20-02 owns the pinned-text fix | waived | obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop dry_run.rs SECTION_COMMANDS) [inferred — cleanup 2026-10-28] | 2026-08-19T19:47:28.307Z | 2026-10-28T21:21:25.455Z |
| 10 | 20 | deviation | tests/driver_reattach.rs |  | Wave-2 post-merge gate: 2 failures, both in driver_reattach. Bisected — FAILS at b6c1ae7 (docs-only, zero phase-20 source), so NOT a phase-20 regression; matches phase 19 deferred-items 'proved pre-existing'. New observation worth acting on: it now fails 3/3 in ISOLATION (0.53s), whereas phase 19 recorded it passing in isolation and failing only under parallel load. The failure rate has increased and the isolation-passes assumption in deferred-items.md is now stale. 930/932 tests pass; failures confined to this one target. | waived | obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (tests/driver_reattach.rs targeted the deleted drive loop) [inferred — cleanup 2026-10-28] | 2026-08-19T20:32:38.102Z | 2026-10-28T21:21:25.607Z |
| 11 | 20 | deviation | src/journal/mod.rs |  | 20-05: the Parked.reason taxonomy table gained a fourth sibling taxonomy (quota_) as a doc-only amendment. journal/mod.rs was outside 20-05's declared files_modified, but its own doc said 'Three sanctioned taxonomies' and its text says naming only some of them would be the same quiet lie the record exists to prevent — so leaving it stale was not an option. Quota is a sibling taxonomy, not a fifth BoundsReason arm: bounds.rs documents CTRL-06 as exactly four detectors, and router.rs belonged to a parallel worktree. | waived | obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop Parked.reason taxonomy in src/journal) [inferred — cleanup 2026-10-28] | 2026-08-19T23:26:50.738Z | 2026-10-28T21:21:25.754Z |
| 12 | 20 | deviation | src/driver/rate_limit.rs |  | 20-05: research assumption A2 recommended detecting quota exhaustion via terminal_reason.starts_with('api_error'). That is wrong — it would report api_error_overloaded and every future api_error_* as quota exhaustion, telling the user to wait out a 7-day window for a fault a retry clears in seconds. Shipped predicate is contains('rate_limit'), with a test pinning seven non-quota terminal reasons including budget_exhausted (the --max-budget-usd post-turn breaker, a different constraint per D-16). 20-RESEARCH.md A2 is stale. | waived | waived, intentional: contains('rate_limit') quota predicate is the correct shipped behaviour and still used via src/launcher/outcome.rs:27; 20-RESEARCH A2 is the stale side [inferred — cleanup 2026-10-28] | 2026-08-19T23:26:50.849Z | 2026-10-28T21:21:27.095Z |
| 13 | 21 | deviation | src/driver/escalate.rs |  | 21-02: plan acceptance criterion required escalate::resolve(None, 2) to REFUSE. Implementing it literally makes --max-steps 1 unrunnable — no legal cap exists (0 refused by the zero rule, 1 refused by the step-cap rule), so the refusal could name no remedy and a CTRL-06 test could only be 'fixed' by destroying the property it proves. Implemented instead: a SUPPLIED cap is refused exactly as specified; an UNSUPPLIED default is put through the same comparison and reduced to min(3, max_steps-1). Asymmetry is deliberate and documented in three places plus a test: supplied zero is a seam that looks configured but can never fire; derived zero is the honest consequence of a one-step run. Verified by the orchestrator. | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/driver/escalate.rs, Phase 21 goal layer) [inferred — cleanup 2026-10-28] | 2026-08-20T02:45:53.478Z | 2026-10-28T21:21:25.901Z |
| 14 | 21 | stub | src/driver/goal.rs |  | 21-03: research Q4 (approval bound to plan AND files) is HALF discharged. The files half is now enforced at the spawn gate with SHA-256; the plan half still rests on goal::plan_digest, which remains FNV-1a and is documented in-tree as not a security control. Binding the two belongs with the loop wiring in 21-04/21-06. Recorded so it is not read as closed. | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/driver/goal.rs plan_digest, Phase 21 goal layer) [inferred — cleanup 2026-10-28] | 2026-08-20T03:25:51.109Z | 2026-10-28T21:21:26.048Z |
| 15 | 21 | deviation | src/config.rs |  | 21-03: added #[serde(flatten)] extra to DriverOptIn. Empirically proven necessary — a probe against the real load_config/save_config showed entry-level unknown keys survive a round-trip but keys nested inside driver_opt_in are SILENTLY DELETED, which made the plan's own round-trip acceptance criterion unpassable and T-21-20's mitigation a paper one at that depth. Fail-safe: a dropped prompt_inputs reads as absent, absent means re-confirm, so the failure mode is a spurious re-confirmation after a downgrade, not a silent approval. | waived | waived, intentional: DriverOptIn #[serde(flatten)] extra (src/config.rs:347; triage cited :70, which is RegisteredProject's flatten) was proven necessary for lossless round-trip; fail-safe on downgrade [inferred — cleanup 2026-10-28] | 2026-08-20T03:25:51.228Z | 2026-10-28T21:21:27.245Z |
| 16 | 21 | stub | src/driver/router.rs |  | 21-04 STRUCTURAL FINDING affecting ROADMAP criterion 3: router::decide cannot return NoRule for ANY project state this tree's reader produces from disk. The rule table covers five statuses, gate_for intercepts partial and every executed verification status, and complete requires passed which is_goal_met answers first; the one uncovered state violates the reader's own invariant (src/state_reader/disk_status.rs:670). Consequences: (1) the ambiguity seam is wired, order-pinned and unit-tested but has NO reachable production path; (2) since goal decomposition is the only escalation that can occur, a run makes at most 1 escalation, so escalation_cap_reached is UNREACHABLE end-to-end and no run can park under it on disk. This is arguably a good property — it means the rule table is complete — but criterion 3's cap must be verified as 'mechanism correct and unit-proven, no production path reaches it', NOT as end-to-end proven. 21-06 cannot discharge 21-02's E7 deferral without deliberately widening the reader, removing a rule-table row, or accepting a fixture that writes an inference the reader would never produce. Do not let a fixture of that third kind be mistaken for end-to-end proof. | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (router NoRule/escalation reachability belonged to the deleted Phase 21 goal loop; router survives only as the advisory next --json hint) [inferred — cleanup 2026-10-28] | 2026-08-20T04:50:50.557Z | 2026-10-28T21:21:26.202Z |
| 17 | 19 | deviation | tests/envelope_wrapper_class.rs |  | MIN_UNREADABLE_FORGE_SLOT_CASES was 50 against a stated arithmetic of 68 while the loop generates 40 — unreachable by construction; corrected to 40 by plan 19-17 | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (tests/envelope_wrapper_class.rs) [inferred — cleanup 2026-10-28] | 2026-08-30T00:22:46.676Z | 2026-10-28T21:21:26.349Z |
| 18 | 19 | deviation | tests/envelope_literal_decision.rs |  | 19-16's the_marked_payload_splice_... pinned exit 0 despite name/comment/SUMMARY saying it asserts nothing post-fix; guard discharged and re-pinned by 19-17 without weakening clause 2(a) | waived | obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (tests/envelope_literal_decision.rs) [inferred — cleanup 2026-10-28] | 2026-08-30T00:22:46.812Z | 2026-10-28T21:21:26.497Z |
| 19 | 19 | unrun-verify | tests/envelope_callee_grammar.rs | 937 | 19-21 blocker: `git --super-prefix x push --force origin main` pinned at force_push_blocked is incompatible with `git --super-prefix x status` pinned at envelope_assertion_failed; row left RED | fixed | Corpus-labelling correction, no src/ change. Verified by measurement against the built binary (fresh GSD_MM_ENVELOPE_ROOT per row, root walked after): BOTH spellings refuse at exit 2 with an EMPTY walk; only the identifier differs. force_push_blocked was a PRE-fix observation mislabelled as post-fix — it held only because --super-prefix sat in GIT_GLOBAL_VALUE_OPTS and consumed x, and removing that entry was part of 19-21's fix (git 2.43.0 rejects --super-prefix bare, separate and attached). Row 937 corrected to envelope_assertion_failed; exit code and empty walk still asserted; no second reading site added. Commit 989f21a. tests/envelope_callee_grammar.rs is 19/19 green; all thirteen envelope_* binaries green; passed+failed = 1639. | 2026-09-04T02:50:39.202Z | 2026-09-04T02:58:45.101Z |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "18",
    "file": "src/ui/screens/driver.rs",
    "line": null,
    "description": "18-10: injection state refuses to promote an interjected record with delivered:false — a plan-rule deviation toward the prohibition; revisit if the driver ever stops writing the failed-write record",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): the drive-loop injection path in src/ui/screens/driver.rs was deleted in Phase 31; Phase 30 steers via DIRECTION.md control instead [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-07-30T03:39:40.539Z",
    "resolved_at": "2026-10-28T21:21:24.706Z"
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "19",
    "file": "src/envelope/mod.rs",
    "line": null,
    "description": "GSD_MM_ENVELOPE_ROOT override added so the tracer fixture is hermetic; fail-closed by construction, residual limit documented",
    "status": "waived",
    "reason": "waived, intentional: GSD_MM_ENVELOPE_ROOT test override is fail-closed by construction and still present (src/envelope/mod.rs:113-120); residual limit documented in-tree [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-18T17:21:17.313Z",
    "resolved_at": "2026-10-28T21:21:26.951Z"
  },
  {
    "id": 3,
    "kind": "unmet-truth",
    "phase": "19",
    "file": ".planning/phases/19-gitsafe-git-blast-radius-envelope/19-06-SUMMARY.md",
    "line": null,
    "description": "Phase 19 honesty statement SECTION_ENVELOPE: its three parts are test-pinned but its tone is human_judgment only — no test can assert it reads as candour rather than a hedge",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (SECTION_ENVELOPE / src/envelope/advisory.rs) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-18T22:36:01.082Z",
    "resolved_at": "2026-10-28T21:21:24.854Z"
  },
  {
    "id": 4,
    "kind": "unmet-truth",
    "phase": "19",
    "file": "src/envelope/hooks.rs",
    "line": null,
    "description": "The pull-request cap has no git-hook second carrier: a settings file the agent CLI silently ignores leaves SAFE-06 unenforced while every push boundary stays standing (19-05 recorded degradation)",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/envelope/hooks.rs; push is now outside the sandbox, Phase 28 LNCH-06) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-18T22:36:06.541Z",
    "resolved_at": "2026-10-28T21:21:25.007Z"
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "19",
    "file": "src/envelope/advisory.rs",
    "line": null,
    "description": "probe_protection's 10s budget/kill-at-deadline path has no test driving a real stall — D-35 forbids the network fixture; correctness under a real hang is reasoned, not observed",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (probe_protection in src/envelope/advisory.rs) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-18T22:36:07.883Z",
    "resolved_at": "2026-10-28T21:21:25.156Z"
  },
  {
    "id": 6,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "20-01: Decision::GoalMet has no producer until DiskInference records the VERIFICATION frontmatter status (research Pitfall 2) — closed by 20-03 + 20-04",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:27.966Z",
    "resolved_at": "2026-10-28T21:21:26.649Z"
  },
  {
    "id": 7,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "20-01: RouterReason::DependencyUnsatisfied has no producer; belongs to the plan that widens the rule table (20-04)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-19T19:47:28.080Z",
    "resolved_at": "2026-10-28T21:21:26.799Z"
  },
  {
    "id": 8,
    "kind": "stub",
    "phase": "20",
    "file": "src/driver/run.rs",
    "line": null,
    "description": "20-01: GOAL_MET_LABEL is unreachable — the one terminal label sourced from neither outcome_label nor the parked: prefix; needs a GoalMet producer (20-04)",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop run.rs GOAL_MET_LABEL) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-19T19:47:28.196Z",
    "resolved_at": "2026-10-28T21:21:25.302Z"
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "20",
    "file": "src/driver/dry_run.rs",
    "line": null,
    "description": "20-01: SECTION_COMMANDS and its sibling prose at dry_run.rs:14 and :107-111 still claim the single --command is the complete honest sequence, which the routed loop makes false (research Pitfall 6); a routed preview reports '(routed: chosen per iteration)' so nothing lies about a specific command — 20-02 owns the pinned-text fix",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop dry_run.rs SECTION_COMMANDS) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-19T19:47:28.307Z",
    "resolved_at": "2026-10-28T21:21:25.455Z"
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "20",
    "file": "tests/driver_reattach.rs",
    "line": null,
    "description": "Wave-2 post-merge gate: 2 failures, both in driver_reattach. Bisected — FAILS at b6c1ae7 (docs-only, zero phase-20 source), so NOT a phase-20 regression; matches phase 19 deferred-items 'proved pre-existing'. New observation worth acting on: it now fails 3/3 in ISOLATION (0.53s), whereas phase 19 recorded it passing in isolation and failing only under parallel load. The failure rate has increased and the isolation-passes assumption in deferred-items.md is now stale. 930/932 tests pass; failures confined to this one target.",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (tests/driver_reattach.rs targeted the deleted drive loop) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-19T20:32:38.102Z",
    "resolved_at": "2026-10-28T21:21:25.607Z"
  },
  {
    "id": 11,
    "kind": "deviation",
    "phase": "20",
    "file": "src/journal/mod.rs",
    "line": null,
    "description": "20-05: the Parked.reason taxonomy table gained a fourth sibling taxonomy (quota_) as a doc-only amendment. journal/mod.rs was outside 20-05's declared files_modified, but its own doc said 'Three sanctioned taxonomies' and its text says naming only some of them would be the same quiet lie the record exists to prevent — so leaving it stale was not an option. Quota is a sibling taxonomy, not a fifth BoundsReason arm: bounds.rs documents CTRL-06 as exactly four detectors, and router.rs belonged to a parallel worktree.",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): Phase 31 (31-01..31-09) deleted the referenced code (drive-loop Parked.reason taxonomy in src/journal) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-19T23:26:50.738Z",
    "resolved_at": "2026-10-28T21:21:25.754Z"
  },
  {
    "id": 12,
    "kind": "deviation",
    "phase": "20",
    "file": "src/driver/rate_limit.rs",
    "line": null,
    "description": "20-05: research assumption A2 recommended detecting quota exhaustion via terminal_reason.starts_with('api_error'). That is wrong — it would report api_error_overloaded and every future api_error_* as quota exhaustion, telling the user to wait out a 7-day window for a fault a retry clears in seconds. Shipped predicate is contains('rate_limit'), with a test pinning seven non-quota terminal reasons including budget_exhausted (the --max-budget-usd post-turn breaker, a different constraint per D-16). 20-RESEARCH.md A2 is stale.",
    "status": "waived",
    "reason": "waived, intentional: contains('rate_limit') quota predicate is the correct shipped behaviour and still used via src/launcher/outcome.rs:27; 20-RESEARCH A2 is the stale side [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-19T23:26:50.849Z",
    "resolved_at": "2026-10-28T21:21:27.095Z"
  },
  {
    "id": 13,
    "kind": "deviation",
    "phase": "21",
    "file": "src/driver/escalate.rs",
    "line": null,
    "description": "21-02: plan acceptance criterion required escalate::resolve(None, 2) to REFUSE. Implementing it literally makes --max-steps 1 unrunnable — no legal cap exists (0 refused by the zero rule, 1 refused by the step-cap rule), so the refusal could name no remedy and a CTRL-06 test could only be 'fixed' by destroying the property it proves. Implemented instead: a SUPPLIED cap is refused exactly as specified; an UNSUPPLIED default is put through the same comparison and reduced to min(3, max_steps-1). Asymmetry is deliberate and documented in three places plus a test: supplied zero is a seam that looks configured but can never fire; derived zero is the honest consequence of a one-step run. Verified by the orchestrator.",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/driver/escalate.rs, Phase 21 goal layer) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-20T02:45:53.478Z",
    "resolved_at": "2026-10-28T21:21:25.901Z"
  },
  {
    "id": 14,
    "kind": "stub",
    "phase": "21",
    "file": "src/driver/goal.rs",
    "line": null,
    "description": "21-03: research Q4 (approval bound to plan AND files) is HALF discharged. The files half is now enforced at the spawn gate with SHA-256; the plan half still rests on goal::plan_digest, which remains FNV-1a and is documented in-tree as not a security control. Binding the two belongs with the loop wiring in 21-04/21-06. Recorded so it is not read as closed.",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (src/driver/goal.rs plan_digest, Phase 21 goal layer) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-20T03:25:51.109Z",
    "resolved_at": "2026-10-28T21:21:26.048Z"
  },
  {
    "id": 15,
    "kind": "deviation",
    "phase": "21",
    "file": "src/config.rs",
    "line": null,
    "description": "21-03: added #[serde(flatten)] extra to DriverOptIn. Empirically proven necessary — a probe against the real load_config/save_config showed entry-level unknown keys survive a round-trip but keys nested inside driver_opt_in are SILENTLY DELETED, which made the plan's own round-trip acceptance criterion unpassable and T-21-20's mitigation a paper one at that depth. Fail-safe: a dropped prompt_inputs reads as absent, absent means re-confirm, so the failure mode is a spurious re-confirmation after a downgrade, not a silent approval.",
    "status": "waived",
    "reason": "waived, intentional: DriverOptIn #[serde(flatten)] extra (src/config.rs:347; triage cited :70, which is RegisteredProject's flatten) was proven necessary for lossless round-trip; fail-safe on downgrade [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-20T03:25:51.228Z",
    "resolved_at": "2026-10-28T21:21:27.245Z"
  },
  {
    "id": 16,
    "kind": "stub",
    "phase": "21",
    "file": "src/driver/router.rs",
    "line": null,
    "description": "21-04 STRUCTURAL FINDING affecting ROADMAP criterion 3: router::decide cannot return NoRule for ANY project state this tree's reader produces from disk. The rule table covers five statuses, gate_for intercepts partial and every executed verification status, and complete requires passed which is_goal_met answers first; the one uncovered state violates the reader's own invariant (src/state_reader/disk_status.rs:670). Consequences: (1) the ambiguity seam is wired, order-pinned and unit-tested but has NO reachable production path; (2) since goal decomposition is the only escalation that can occur, a run makes at most 1 escalation, so escalation_cap_reached is UNREACHABLE end-to-end and no run can park under it on disk. This is arguably a good property — it means the rule table is complete — but criterion 3's cap must be verified as 'mechanism correct and unit-proven, no production path reaches it', NOT as end-to-end proven. 21-06 cannot discharge 21-02's E7 deferral without deliberately widening the reader, removing a rule-table row, or accepting a fixture that writes an inference the reader would never produce. Do not let a fixture of that third kind be mistaken for end-to-end proof.",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (router NoRule/escalation reachability belonged to the deleted Phase 21 goal loop; router survives only as the advisory next --json hint) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-20T04:50:50.557Z",
    "resolved_at": "2026-10-28T21:21:26.202Z"
  },
  {
    "id": 17,
    "kind": "deviation",
    "phase": "19",
    "file": "tests/envelope_wrapper_class.rs",
    "line": null,
    "description": "MIN_UNREADABLE_FORGE_SLOT_CASES was 50 against a stated arithmetic of 68 while the loop generates 40 — unreachable by construction; corrected to 40 by plan 19-17",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (tests/envelope_wrapper_class.rs) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-30T00:22:46.676Z",
    "resolved_at": "2026-10-28T21:21:26.349Z"
  },
  {
    "id": 18,
    "kind": "deviation",
    "phase": "19",
    "file": "tests/envelope_literal_decision.rs",
    "line": null,
    "description": "19-16's the_marked_payload_splice_... pinned exit 0 despite name/comment/SUMMARY saying it asserts nothing post-fix; guard discharged and re-pinned by 19-17 without weakening clause 2(a)",
    "status": "waived",
    "reason": "obsolete (closed, not fixed): owned by a superseded phase (19/21, superseded 2026-10-14 by spikes 001/002 and Phases 26-31); Phase 31 deleted the code (tests/envelope_literal_decision.rs) [inferred — cleanup 2026-10-28]",
    "recorded_at": "2026-08-30T00:22:46.812Z",
    "resolved_at": "2026-10-28T21:21:26.497Z"
  },
  {
    "id": 19,
    "kind": "unrun-verify",
    "phase": "19",
    "file": "tests/envelope_callee_grammar.rs",
    "line": 937,
    "description": "19-21 blocker: `git --super-prefix x push --force origin main` pinned at force_push_blocked is incompatible with `git --super-prefix x status` pinned at envelope_assertion_failed; row left RED",
    "status": "fixed",
    "reason": "Corpus-labelling correction, no src/ change. Verified by measurement against the built binary (fresh GSD_MM_ENVELOPE_ROOT per row, root walked after): BOTH spellings refuse at exit 2 with an EMPTY walk; only the identifier differs. force_push_blocked was a PRE-fix observation mislabelled as post-fix — it held only because --super-prefix sat in GIT_GLOBAL_VALUE_OPTS and consumed x, and removing that entry was part of 19-21's fix (git 2.43.0 rejects --super-prefix bare, separate and attached). Row 937 corrected to envelope_assertion_failed; exit code and empty walk still asserted; no second reading site added. Commit 989f21a. tests/envelope_callee_grammar.rs is 19/19 green; all thirteen envelope_* binaries green; passed+failed = 1639.",
    "recorded_at": "2026-09-04T02:50:39.202Z",
    "resolved_at": "2026-09-04T02:58:45.101Z"
  }
]
````
