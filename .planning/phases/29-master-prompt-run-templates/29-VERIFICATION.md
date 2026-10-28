---
phase: 29-master-prompt-run-templates
verified: 2026-10-28T12:00:00Z
status: passed
score: 5/5 must-haves verified
covered_files:
  - ".planning/phases/29-master-prompt-run-templates/29-01-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-01-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-02-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-02-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-03-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-03-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-04-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-04-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-05-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-05-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-06-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-06-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-07-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-07-SUMMARY.md"
  - ".planning/phases/29-master-prompt-run-templates/29-08-PLAN.md"
  - ".planning/phases/29-master-prompt-run-templates/29-08-SUMMARY.md"
  - "scripts/prompt-acceptance.sh"
  - "src/cli.rs"
  - "src/driver/direction.rs"
  - "src/driver_cli.rs"
  - "src/driver_templates/assets/DIRECTION.md"
  - "src/driver_templates/assets/LOG.md"
  - "src/driver_templates/lint.rs"
  - "src/driver_templates/mod.rs"
  - "src/driverctl/serve.rs"
  - "src/driverctl/store.rs"
  - "src/launcher/assets/MASTER-PROMPT.md"
  - "src/launcher/gate.rs"
  - "src/launcher/prompt.rs"
  - "src/next_cli.rs"
  - "tests/driverctl_store.rs"
  - "tests/prompt_acceptance.rs"
covered_digest: "v4:sha256:5c871487be44e9730c3d2ca015593f0cda6dab836823f93fdd782eb2d98c5693"
behavior_unverified: 0
gaps: []
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "29-REVIEW WR-01: lint's KEY_VALUE matcher is stricter than the store's control_re, so an indented or differently cased `- control: pause` holds the run while `driver check` reports clean"
    category: other
    reason: "Open in 29-REVIEW-DISPOSITION.md, advisory by instruction. Failure direction is safe (the run holds); PRMT-04 'control edit honoured before next dispatch' is still met (store tests a_pause_or_stop_is_honoured_in_any_case_and_with_stray_whitespace and the gate tests pass)."
    evidence_status: "reviewed; behavior exercised by passing named tests"
  - finding: "29-REVIEW WR-02 (non-object host managed hooks/permissions replace launcher gate), WR-03 (hook_is_kept over-broad), IN-01..IN-03"
    category: security
    reason: "Open in the disposition, located in src/launcher/config_dir.rs and store comments; none touches a Phase 29 success criterion. 29-SECURITY.md threats_open 0. Tracked for a follow-up, not fixed here by instruction. [inferred — cleanup 2026-10-28]"
    evidence_status: "per 29-REVIEW.md"
---

# Phase 29: Master Prompt & Run Templates Verification Report

**Phase Goal:** The driver's contract ships with gsd-mm as a versioned prompt and file templates. A new run is one command from a ready DIRECTION.md, and stop-rule precedence is stated explicitly instead of left implicit.
**Verified:** 2026-10-28T12:00:00Z
**Status:** passed
**Re-verification:** Yes. The 2026-10-20 report went stale: covered files changed in Phase 28.1, quick tasks, Phase 31 deletions and cleanup batch 261028-n0u. Every must-have was re-checked at HEAD (e720238 plus docs-only commits) with fresh CLI runs and targeted suites.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master prompt embedded, versioned, version recorded on the run; requires fresh subagent per step, 11-field report <= 15 lines, nested `isolation: "worktree"`, async no-respawn, all records via driverctl | VERIFIED | `src/launcher/assets/MASTER-PROMPT.md` line 1 `driver-prompt/1.0.0`; `DRIVER_PROMPT_VERSION` at `src/launcher/prompt.rs:25`, `custom:<hash8>` for overrides (prompt.rs:81-95). Prompt rule 2 (15-line report), rule 3 (`isolation: "worktree"`), async no-respawn (line 8), every record via `driverctl log ...` (lines 27-37). Version carried into the run: `run.rs:775,933` to `store.rs:603,981`; `tests/launcher_e2e.rs:129-172` assert `prompt=driver-prompt/1.0.0` / `prompt=custom:`. `tests/driver_prompt.rs` 14 pass |
| 2 | `driver new <project> <slug>` mints `<yyyymmdd>-<slug>`, creates run dir with self-ignoring `.gitignore`, DIRECTION.md, LOG.md with grammar inline; ready DIRECTION.md launches in one command | VERIFIED | Live run at HEAD on a scratch registered project: `driver new <p> my-slug` -> `created run 20261028-my-slug`, rc 0, dir holds `.gitignore` (`*`), DIRECTION.md, LOG.md (grammar comment inline); prints `check:` and `launch:` one-liners. `driver check` rc 1 with line-numbered placeholders on the unfilled template, rc 0 once filled. Duplicate refused (rc 1); slugs `.bad`, `a/b`, `a b` refused with the regex message. The template carries no `isolation:` line. `tests/driver_new.rs` 13 pass |
| 3 | Stop rules explicit in prompt and template standing rules with written precedence; stop rules outrank `answer:`, only a budget edit lifts | VERIFIED | Generated DIRECTION.md holds the `stop-rules:begin/end` block (ordered rules 1-8) and "A human `answer:` lifts only a `needs-human` park (rule 8) ... lifted only by a budget edit ... An empty `answer:` lifts nothing." Same block in MASTER-PROMPT.md line 86-99. `tests/driver_prompt.rs` goldens pass. `mentions_phase` (gate.rs:49-64) keeps 26 and 26.1 distinct |
| 4 | Crash-and-relaunch reconciles every `running` item to exactly one of done/resume/reset/park; `control: pause` mid-dispatch ends the run before the next dispatch | VERIFIED | Prompt reconcile section (done/resume/reset/park, lines 137-142). `driverctl_reconcile` 7, `launcher_reconcile` 7, `driverctl_store` 60 pass, including `the_cap_gate_denies_a_driver_dispatch_while_control_is_pause`, `dispatched_is_rejected_while_control_is_pause_or_stop...`, `a_pause_or_stop_is_honoured_in_any_case_and_with_stray_whitespace`, `a_pause_on_any_duplicate_control_line_holds_the_run`, `a_control_hold_is_named_before_the_reconcile_rule...` |
| 5 | `next <project> --json` exposes the Phase 20 router as an advisory hint, cited as advisory only by the prompt | VERIFIED | `next <p> --json` -> `{"advisory":true,"decision":"none","reason":"no-target","source":"phase-20-router",...}` rc 0. Prompt line 159 "is advisory"; `SUGGESTED_NEXT: <advisory only>`. `tests/next_cli.rs` 12 pass |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified).

### Renamed or moved symbols since the last report

No must-have cited a deleted symbol. The prior test counts moved (`driverctl_store` 53 -> 60, new tests from later phases); every previously cited behavior has a passing current test. The prior "known local-only git-version witness" failure no longer exists (retired in Phase 31): the full run has 0 failures.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/launcher/assets/MASTER-PROMPT.md` | VERIFIED | embedded; version line 1 agrees with `DRIVER_PROMPT_VERSION` (driver_prompt test) |
| `src/driver_templates/{mod,lint}.rs`, `assets/DIRECTION.md`, `assets/LOG.md` | VERIFIED | exercised end to end by CLI runs |
| `src/driver/direction.rs` | VERIFIED | shared reader (store and lint) |
| `src/next_cli.rs`, `src/launcher/gate.rs`, `src/driverctl/store.rs`, `src/driverctl/serve.rs` | VERIFIED | suites green |
| `scripts/prompt-acceptance.sh`, `29-P29-RUBRIC.md`, `acceptance/*` | VERIFIED | `tests/prompt_acceptance.rs` 20 pass, 2 ignored (paid live entry points) |

### Key Links

| From | To | Status |
|------|----|--------|
| `driver launch` | embedded prompt / `prompt_version` | WIRED (run.rs:775 -> store.rs:981, e2e asserts) |
| `driver launch` | `preflight_direction` lint + control refusal | WIRED |
| PreToolUse gate / store hook | DIRECTION `control`, fail closed | WIRED |
| `next` | Phase 20 router, advisory | WIRED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mint / dup / bad slug / lint | `driver new`, `driver check` on scratch project | as SC2 | PASS |
| advisory hint | `next <p> --json` | JSON advisory:true, rc 0 | PASS |
| phase suites | `cargo test --no-fail-fast --test driver_prompt --test driverctl_reconcile --test launcher_reconcile --test prompt_acceptance --test driver_new --test driverctl_store --test next_cli` | 14, 7, 7, 20, 13, 60, 12 passed, 0 failed | PASS |
| regression gate | `cargo test --no-fail-fast` at e720238 (orchestrator log) | 55 binaries, 3183 passed, 0 failed, 17 ignored | PASS |

Live paid acceptance entry points were not run.

### Probe Execution

No phase-declared `probe-*.sh`; the acceptance harness tests above are the equivalent. N/A.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| PRMT-01 | 29-01, 02, 06, 07, 08 | SATISFIED | SC1 |
| PRMT-02 | 29-04, 06 | SATISFIED | SC2 |
| PRMT-03 | 29-01, 04, 07, 08 | SATISFIED | SC3 |
| PRMT-04 | 29-02, 05, 07, 08 | SATISFIED | SC4 |
| PRMT-05 | 29-01, 03 | SATISFIED | SC5 |
| DRIVE-03 | 29-04, 06, 07, 08 | SATISFIED | lint + `driver check` over done-when/budget items |
| DRIVE-01 (prompt half, also claimed by 29-08) | 29-08 | SATISFIED (operator acceptance, 29-UAT.md) | launcher half is Phase 28 |

All IDs appear in at least one PLAN's `requirements` and are `[x]` / Complete in REQUIREMENTS.md (lines 86, 239-261, 400-404). No orphaned IDs.

### Anti-Patterns Found

No TBD/FIXME/XXX in the covered implementation files (grep). Code review re-run (29-REVIEW.md, e6c0d9e): 0 critical, 3 warning, 3 info, all `open` in the disposition and advisory only. None breaks a must-have (see advisory frontmatter). Security: 29-SECURITY.md `threats_open: 0` (795480c).

### Human Verification Required

None. The sole prior item (o1 ADJUST x2, h2 H2-5) was signed off by the operator on 2026-10-20 (29-UAT.md, complete 1/1) and is carried forward. The code it depended on (prompt, reconcile, gate, store) has not regressed: the suites that cover it pass at HEAD. Not re-raised.

### Gaps Summary

No gaps. All five success criteria hold at HEAD, and the later changes (Phase 28.1, quick tasks, Phase 31 deletions, cleanup batch) did not remove or weaken any behavior this phase delivers. Open review warnings are recorded as advisories.

---

_Verified: 2026-10-28T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
