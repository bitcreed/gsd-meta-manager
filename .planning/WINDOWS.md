---
schema_version: 1
open_count: 9
waived_count: 0
fixed_count: 0
total_count: 9
last_updated: 2026-08-19T19:47:28.307Z
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
  }
]
````
