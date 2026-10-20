---
phase: 29-master-prompt-run-templates
verified: 2026-10-20T16:10:00Z
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
covered_digest: "v3:sha256:fbfddf35e305df46095346aa6120dee644de26a6d527bf8b2bbc758fda40cc70"
behavior_unverified: 0
gaps: []
re_verification:
  previous_status: human_needed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "IN-01 (prompt uses $RUN_DIR / $PROJECT_ROOT as if env vars), IN-03 (preflight ordering masks 'already launched'), IN-05 (lossy shell_quote for non-UTF-8 paths): skipped by the fixer as non-trivial contract changes"
    category: other
    reason: "Recorded `skipped` (not open) in 29-REVIEW-DISPOSITION.md; IN-01 is captured as backlog 999.7 with the reconcile-first/LAUNCHER CAP prompt follow-up. None touches a success criterion."
    evidence_status: "per 29-REVIEW-FIX.md; backlog f33f5f5"
---

# Phase 29: Master Prompt & Run Templates Verification Report

**Phase Goal:** The driver's contract ships with gsd-mm as a versioned prompt and file templates. A new run is one command from a ready DIRECTION.md, and stop-rule precedence is stated explicitly instead of left implicit.
**Verified:** 2026-10-20T16:10:00Z
**Status:** passed
**Re-verification:** Yes. The prior report (status human_needed, 2026-10-20T14:55Z) went stale when the code-review fixes (1fb4c24..3dfa13e, ledger 7923976) changed covered files. Every success criterion was re-checked against the post-fix tree; the human item was settled by the operator (see Operator Sign-off).

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master prompt embedded, versioned, version recorded on the run; carries fresh subagent per step, 11-field report <= 15 lines, nested `isolation: "worktree"`, async no-respawn, all records via driverctl | VERIFIED | `src/launcher/assets/MASTER-PROMPT.md` line 1 `driver-prompt/1.0.0` (unchanged by the review fixes: the version the live evidence was measured against). `tests/driver_prompt.rs` 14 pass (version agreement + goldens). Live evidence unchanged: LOG `prompt=driver-prompt/1.0.0`, `custom:<sha8>` for an override (h4-4) |
| 2 | `driver new <project> <slug>` mints `<yyyymmdd>-<slug>`, creates run dir with self-ignoring `.gitignore`, DIRECTION.md template, LOG.md with grammar inline; a ready DIRECTION.md launches in one command | VERIFIED | Re-ran on the post-fix binary against a registered scratch project: `driver new proj my-slug` -> `20261020-my-slug`, dir holds `.gitignore`, DIRECTION.md, LOG.md; prints the `check:` and `launch:` lines; `driver check` rc=1 on the unfilled template with line-numbered placeholders, rc=0 once filled; duplicate and `.bad` slug refused. `tests/driver_new.rs` 13 pass. WR-05 fix confirmed: the template no longer ships an `isolation:` line (grep empty; guard test at `src/driver_templates/mod.rs:362`) |
| 3 | Stop rules explicit in prompt and template standing rules with written precedence; stop rules outrank `answer:`, only a budget edit lifts | VERIFIED | Generated DIRECTION.md line 64 states "A human `answer:` lifts only a `needs-human` park (rule 8) ... lifted only by a budget edit"; same precedence block in the prompt (`stop-rules:begin/end`, covered by driver_prompt goldens). Live h3 PASS, h4 PASS (unchanged evidence). WR-04 fix confirmed in `src/launcher/gate.rs:49-64` (`mentions_phase` now treats `26.1` and `26` as distinct keys, tests cover 26/26.1/26.10/260) |
| 4 | Scripted crash-and-relaunch reconciles every `running` item to exactly one of done/resume/reset/park; `control: pause` mid-dispatch ends the run before the next dispatch | VERIFIED | `driverctl_reconcile` 7 pass, `launcher_reconcile` 7 pass, `driverctl_store` 53 pass (post-fix: includes pause/stop case variants, duplicate control lines, unreadable DIRECTION.md fail-closed, reconcile-first denial at the dispatch itself). Live h1 PASS, h2 H2-1..H2-4 PASS (H2-5: see Operator Sign-off). `scripts/prompt-acceptance.sh check h2` still re-derives the committed FAIL byte-for-byte; `selftest` OK |
| 5 | `next <project> --json` exposes the Phase 20 router as an advisory hint, cited as advisory only by the prompt | VERIFIED | `next proj --json` -> `{"advisory":true,...,"source":"phase-20-router"}` rc=0 (decision `none`, reason `no-target` for an empty scratch project); `tests/next_cli.rs` 12 pass; prompt line 159 "is advisory" and `SUGGESTED_NEXT: <advisory only>` |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Code-review fix re-check (29-REVIEW-FIX.md / 29-REVIEW-DISPOSITION.md, ledger 7923976: open 0 / total 12)

| Finding | Disposition | Re-check in the tree |
|---------|-------------|----------------------|
| WR-01 lint/store read different plans | fixed (1fb4c24) | Reproduced the old defect scenario: a multi-line commented-out `### I-09` now lints rc=0 because both readers blank comments and count items only under `## Items`; a stray `### I-07` under `## Direction` is flagged ("outside `## Items`; the driver ignores it", rc=1) |
| WR-02 pause/stop fails open | fixed (e4b8125) | `control_hold` shared by `decide_dispatch` (gate.rs:81) and the store; `direction_read()` separates NotFound from other read errors (store.rs:648-651) and denies on unreadable (store.rs:1050-1055, 1172-1175); `driver check` rejects `Pause` spelling at launch; store tests green |
| WR-03 terminal escape in lint findings | fixed (9258e33) | `Display`/`as_warning` run through `render_for_terminal` (lint.rs:81,92); `shell_word` in driver_cli.rs:41; lib tests green |
| WR-04 decimal phase vs integer parent | fixed (35a7264) | gate.rs:59-61 regex; tests green |
| WR-05 unread `isolation:` knob | fixed (4fd39cb) | removed from template; prompt version unchanged |
| WR-06 reconcile-first only on log record | fixed (6f45357) | `blocking_recovery()` used by both the record rule and the hook path (store.rs:1001, 1034, 1164); tests green |
| WR-07 H2 harness cannot stage premise | fixed (04a7a40) | `pa_stage_item_running`/`h2_harness=2` in script; `tests/prompt_acceptance.rs` 20 pass (2 ignored = paid live entry points, not run); committed h2 evidence untouched |
| IN-02, IN-04 | fixed | selftest OK; single `cli::DEFAULT_CAP_GAP_ROUNDS` (cli.rs:526) used by clap and lint |
| IN-01, IN-03, IN-05 | skipped (reasons recorded) | Contract changes needing a prompt bump or ordering owner; IN-01 + reconcile-first LAUNCHER CAP routing captured as backlog 999.7 |

The prior report's five advisories (WR-01, WR-02, WR-04, WR-05, WR-06) are closed by these fixes and are no longer advisories.

### Operator Sign-off

The sole prior human_verification item (o1 ADJUST x2 and h2 H2-5 FAIL) was signed off by the operator on 2026-10-20 and recorded in `29-UAT.md` (status complete, 1/1 pass). Decision, verbatim: "Accept both pre-registered outcomes. (1) o1's two ADJUSTs are the accepted outcome. (2) h2's H2-5 FAIL is a rubric setup flaw: the item had already finished, so no recovery was logged, and the pause was honoured. The harness precondition is since fixed (WR-07, 04a7a40). No re-run is authorized."

Consequences recorded here: DRIVE-01 (prompt half) is SATISFIED by operator acceptance; the real-Opus third-gaps_found park remains unobserved live, with carried-risk coverage from the 28-09 d33 reuse, h4, and the deterministic suite. No third paid o1 attempt and no h2 re-run. This is a settled operator decision, not a pending human item.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/launcher/assets/MASTER-PROMPT.md` | VERIFIED | embedded via `launcher::prompt`; version `driver-prompt/1.0.0` |
| `src/driver_templates/{mod,lint}.rs`, `assets/DIRECTION.md`, `assets/LOG.md` | VERIFIED | exercised end to end via the CLI runs above |
| `src/driver/direction.rs` (shared reader) | VERIFIED | used by lint and store; WR-01 repro now consistent |
| `src/next_cli.rs`, `src/launcher/gate.rs`, `src/driverctl/store.rs` | VERIFIED | suites green |
| `scripts/prompt-acceptance.sh`, `29-P29-RUBRIC.md`, `acceptance/*` | VERIFIED | selftest OK; `check h2` re-derives committed verdict |

### Key Links

| From | To | Status |
|------|----|--------|
| `driver launch` | embedded prompt / `prompt_version` | WIRED |
| `driver launch` | `preflight_direction` lint + control refusal | WIRED (shared reader now) |
| `PreToolUse` gate / store hook | DIRECTION `control` (case-insensitive, fail closed on unreadable) | WIRED |
| store hook | reconcile-first denial on the dispatch itself | WIRED |
| `next` | Phase 20 router, advisory | WIRED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mint + lint + WR-01/WR-02 lint behavior | `driver new` / `driver check` (scratch project) | as in SC2 and fix table | PASS |
| advisory hint | `next proj --json` | JSON, rc 0 | PASS |
| phase suites | `cargo test --no-fail-fast --test driver_prompt --test driverctl_reconcile --test launcher_reconcile --test prompt_acceptance --test driver_new --test driverctl_store --test next_cli` | 14 + 7 + 7 + 20 + 13 + 53 + 12 passed, 0 failed (2 ignored paid live entry points) | PASS |
| lib units | `cargo test --lib -- driver_templates launcher::gate driver::direction launcher::config_dir` | 77 passed | PASS |
| lint | `cargo clippy --all-targets -- -D warnings` | clean | PASS |
| acceptance harness | `prompt-acceptance.sh selftest`; `check h2` | OK; re-derives committed FAIL (H2-5) | PASS |
| full suite (per 29-REVIEW-FIX / orchestrator) | `cargo test --no-fail-fast` | 3700 passed, 1 failed: the known local-only git-version witness in `src/envelope/policy.rs` (green on CI) | PASS (known env failure) |

Live paid acceptance entry points were not run (by instruction).

### Probe Execution

No phase-declared probe scripts (`probe-*.sh`); the acceptance harness checks above are the equivalent. N/A.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| PRMT-01 | 29-01, 02, 06, 07, 08 | SATISFIED | SC1 |
| PRMT-02 | 29-04, 06 | SATISFIED | SC2 |
| PRMT-03 | 29-01, 04, 07, 08 | SATISFIED | SC3; h3/h4 live; o1 third-round park accepted by operator |
| PRMT-04 | 29-02, 05, 07, 08 | SATISFIED | SC4; deterministic + h1 + H2-1..4; H2-5 accepted as rubric flaw |
| PRMT-05 | 29-01, 03 | SATISFIED | SC5 |
| DRIVE-03 | 29-04, 06, 07, 08 | SATISFIED | lint + `driver check` over done-when/budget items; WR-01 reader divergence now fixed |
| DRIVE-01 (prompt half) | 29-08 | SATISFIED (operator acceptance) | Operator Sign-off above; launcher half is Phase 28 |

No orphaned IDs: every PRMT-01..05, DRIVE-03 and DRIVE-01 appears in at least one PLAN's `requirements`. REQUIREMENTS.md now reads `[x]` for PRMT-01..05, DRIVE-01 and DRIVE-03 with Complete traceability rows (working-tree edit, not yet committed at verification time; uncommitted `M .planning/REQUIREMENTS.md`).

### Anti-Patterns Found

No TBD/FIXME/XXX in the files changed by the phase or the review fixes (grep over the covered implementation files). Review: 0 critical; 7 warnings fixed; 2 info fixed; 3 info skipped with recorded reasons; 0 open. Security: 29-SECURITY.md `threats_open: 0`.

### Human Verification Required

None. The prior item is settled (Operator Sign-off).

### Gaps Summary

No gaps. All five success criteria hold on the post-review-fix tree, verified by fresh CLI runs and green deterministic suites; the review findings are all resolved or deliberately skipped with recorded reasons; the operator accepted the pre-registered o1/h2 acceptance outcomes. Residual carried risk (real-Opus third-gaps_found park not observed live; prompt-level LAUNCHER CAP handling of a reconcile-first denial, backlog 999.7) is accepted and tracked, not a gap.

---

_Verified: 2026-10-20T16:10:00Z_
_Verifier: Claude (gsd-verifier)_
