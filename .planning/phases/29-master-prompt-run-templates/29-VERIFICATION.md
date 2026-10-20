---
phase: 29-master-prompt-run-templates
verified: 2026-10-20T14:55:00Z
status: human_needed
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
  - ".planning/phases/29-master-prompt-run-templates/29-P29-RUBRIC.md"
  - "scripts/prompt-acceptance.sh"
  - "src/driver_cli.rs"
  - "src/driver_templates/assets/DIRECTION.md"
  - "src/driver_templates/assets/LOG.md"
  - "src/driver_templates/lint.rs"
  - "src/driver_templates/mod.rs"
  - "src/driverctl/store.rs"
  - "src/launcher/assets/MASTER-PROMPT.md"
  - "src/launcher/gate.rs"
  - "src/launcher/prompt.rs"
  - "src/next_cli.rs"
covered_digest: "v3:sha256:3b1cfbfea165f98134f2d920b129c4415471590186806327056552f2a327ca9e"
behavior_unverified: 0
gaps: []
advisory:
  - finding: "WR-01: driver check / launch preflight lint reads a different plan than the store parser (a multi-line commented-out '### I-09' item passes `driver check` rc=0 but is still counted by the store)"
    category: other
    reason: "Reproduced here. Edge case (hand-commented items); the normal template path lints correctly. Make one reader authoritative."
    evidence_status: "reproduced"
  - finding: "WR-02: gate fails open on an unreadable DIRECTION.md and matches `pause`/`stop` case-sensitively (mid-run `control: Pause` is not gate-enforced)"
    category: other
    reason: "Confirmed in src/launcher/gate.rs:72 and src/driverctl/store.rs:645. The documented lowercase word works (H2-3/H2-4 live PASS, unit tests); lint rejects other spellings at launch only. The prompt also tells the model to check stop rule 1 itself. Hardening, not a goal gap."
    evidence_status: "code-read"
  - finding: "WR-04: mentions_phase matches a decimal phase (26.1) against its integer parent (26), so the gap-round gate can deny the wrong dispatch"
    category: other
    reason: "Reproduced with the equivalent regex. Phase 28 backstop code; false-deny only, never a false-allow."
    evidence_status: "reproduced"
  - finding: "WR-05: template ships an `isolation:` knob the prompt never reads"
    category: other
    reason: "Prompt hard rule 3 carries worktree pass-through unconditionally (SC1 satisfied); the knob is cosmetic."
    evidence_status: "code-read"
  - finding: "WR-06: 'reconcile first' refusal is enforced on the `dispatched` log record, not on the Agent dispatch itself"
    category: other
    reason: "PRMT-04 'exactly one recovery per running item' is store-enforced and live-evidenced (h1); an un-logged dispatch is caught only by the dispatch audit (observed > logged). Decide whether D-17 should also gate the hook."
    evidence_status: "code-read"
  - finding: "WR-03, IN-01..IN-05 (terminal-escape in lint findings, prompt $RUN_DIR wording, selftest precedence, preflight ordering, duplicate constants, lossy shell_quote)"
    category: other
    reason: "Hardening/cleanup; none touch a success criterion. 12 findings remain `open` in 29-REVIEW-DISPOSITION.md."
    evidence_status: "per 29-REVIEW.md"
human_verification:
  - test: "Sign the pre-registered unmet acceptance outcomes: o1 ADJUST twice (attempt + one retry; no third attempt) and h2 FAIL (H2-5)"
    expected: "Operator accepts that (a) the real-Opus third-gaps_found park (rubric o1: 'closes SC3 and DRIVE-01') was never reached live, the carried-risk coverage resting on d33 reuse + h4 + the deterministic suite, and (b) h2's H2-5 is a rubric-premise flaw (evidence: I-01 was already `done` so `reconcile found=0` and no `recovered` is correct), live reconciliation being evidenced by h1 PASS instead"
    why_human: "Accepting a pre-registered ADJUST/FAIL and closing DRIVE-01's prompt half is a risk acceptance that is the operator's to sign; the verifier cannot buy a third Opus attempt."
---

# Phase 29: Master Prompt & Run Templates Verification Report

**Phase Goal:** The driver's contract ships with gsd-mm as a versioned prompt and file templates. A new run is one command from a ready DIRECTION.md, and stop-rule precedence is stated explicitly instead of left implicit.
**Verified:** 2026-10-20T14:55:00Z
**Status:** human_needed
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master prompt embedded, versioned, version recorded on the run; carries fresh subagent per step, 11-field report <= 15 lines verbatim, nested `isolation: "worktree"`, async no-respawn, all records via driverctl | VERIFIED | `src/launcher/assets/MASTER-PROMPT.md` header `driver-prompt/1.0.0`; hard rules 1-5 (lines ~52-70) plus worker brief with the 11 fields (STATUS..SUGGESTED_NEXT) and isolation clause; `tests/driver_prompt.rs` 14 pass (version agreement + goldens). Version recorded live: LOG `RUN \| launch \| prompt=driver-prompt/1.0.0` and frontmatter `prompt_version` in h2/o1 evidence; `custom:<sha8>` for override (h4-4 PASS) |
| 2 | `driver new <project> <slug>` mints `<yyyymmdd>-<slug>`, creates run dir with self-ignoring `.gitignore`, DIRECTION.md template (control run/pause/stop, items with done-when and budget), LOG.md with grammar inline | VERIFIED | Ran `driver new p my-slug` -> `20261020-my-slug`; dir holds `.gitignore` (`*`), DIRECTION.md (control, standing rules, I-01 intent/done-when/budget/start-with/after/hold/answer), LOG.md with grammar comment. Bad slug rc=1, duplicate rc=1. `driver check` rc=1 on the unfilled template (6 placeholder findings with line numbers), rc=0 once filled. Launch preflight refuses lint/pause (29-06; `tests/driver_new.rs`, `prompt_acceptance.rs` 18 pass) |
| 3 | Stop rules explicit in prompt and template standing rules, with written precedence: item budget, <= 2 gap rounds, divergence, no progress, run budget; stop rules outrank `answer:`, only a budget edit lifts | VERIFIED | Same 8-rule precedence block (control > launcher denial > run budget > divergence > gap rounds > item budget > no progress > needs-human) in prompt (`stop-rules:begin/end`) and in the generated DIRECTION.md (diffed visually in the `driver new` output); "A human `answer:` lifts only a `needs-human` park" paragraph present in both. Live: h3 PASS (answer alone does not lift a gap-rounds park; budget edit does), h4 PASS (stripped-prompt launcher deny, park `launcher-cap`, no retry). Live real-Opus third-gaps park: NOT reached (see human item) |
| 4 | Scripted crash-and-relaunch reconciles every `running` item to exactly one of done/resume/reset/park; `control: pause` mid-dispatch ends the run before the next dispatch | VERIFIED | `driverctl_reconcile` 7 pass, `launcher_reconcile` 7 pass (SIGKILL mid-dispatch, matrix done/reset/park, pause mid-dispatch). Live: h1 PASS (crash-detected, one `recovered` before first new dispatch). h2: H2-1..H2-4 PASS (report recorded, `end reason=pause`, no dispatch after the pause save); H2-5 FAIL is a rubric-premise flaw, see below. D-18 "before the next dispatch" reading is operator-confirmed |
| 5 | `next <project> --json` exposes the Phase 20 router as advisory hint, cited as advisory only by the prompt | VERIFIED | Ran `next p --json` (alias), `next <path> --json --target-phase 1`: `{"advisory":true,"command":"/gsd-discuss-phase 1","decision":"run",...,"source":"phase-20-router"}` rc=0, no files written; unknown target rc=1. Prompt Loop step 2 cites the hint "advisory ... never binds". `next_cli` suite in full run green |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### H2-5 assessment (settled with evidence)

`acceptance/h2/LOG.md`: pause saved 13:29:09 while the worker slept; the driver recorded I-01 `report`, `verified done-when=met`, `status state=done` (13:30:54) and ended `reason=pause` (13:30:59). On launch 2 `reconcile found=0` is correct: I-01 was `done`, not `running`, so no `recovered` is owed. The rubric premise (an I-01 still `running` at relaunch) was unstageable by a well-behaved driver; code review WR-07 reaches the same conclusion independently. This is not a prompt or product defect. It remains a pre-registered FAIL on record and is folded into the single human item.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/launcher/assets/MASTER-PROMPT.md` | VERIFIED | 215 lines, embedded via `launcher::prompt` (`MASTER_PROMPT`, `DRIVER_PROMPT_VERSION`) |
| `src/driver_templates/{mod,lint}.rs`, `assets/DIRECTION.md`, `assets/LOG.md` | VERIFIED | exercised end to end by the CLI runs above |
| `src/next_cli.rs`, `src/launcher/config_dir.rs` shim | VERIFIED | `next` runs; shim covered by `launcher_config_dir` |
| `src/launcher/gate.rs`, `src/driverctl/store.rs` (pause/stop deny, recovery bookkeeping, `prompt_version`) | VERIFIED | reconcile suites pass; caveats WR-02/04/06 advisory |
| `scripts/prompt-acceptance.sh`, `29-P29-RUBRIC.md`, `acceptance/*` | VERIFIED | consent.txt (approve-all, $25), prereg.txt `acceb35` exists, ledger sums to $12.7557 <= $25; verdicts re-read above |

### Key Links

| From | To | Status |
|------|----|--------|
| `driver launch` | embedded prompt / `prompt_version` in LOG + frontmatter | WIRED (live evidence h2, o1; `custom:<sha8>` h4) |
| `driver launch` | `preflight_direction` lint + control refusal | WIRED (29-06 tests, `launcher_e2e`) |
| `PreToolUse` gate | DIRECTION `control` word | WIRED (lowercase exact match only, WR-02) |
| sandbox | `gsd-meta-manager next` shim | WIRED (`launcher_config_dir`) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mint + lint | `driver new` / `driver check` | as in SC2 | PASS |
| advisory hint | `next p --json` | JSON, rc 0, read-only | PASS |
| prompt/reconcile/acceptance suites | `cargo test --no-fail-fast --test driver_prompt --test driverctl_reconcile --test launcher_reconcile --test prompt_acceptance` | 14 + 7 + 7 + 18 passed, 0 failed (2 ignored = paid live entry points) | PASS |
| full suite (orchestrator) | `cargo test --no-fail-fast` | 3680 pass, 1 fail (known local git-version witness, green on CI); clippy `-D warnings` exit 0 | PASS |
| WR-01 repro | commented-out `### I-09` in DIRECTION.md | `driver check` rc=0 | advisory confirmed |
| WR-04 repro | key `26` vs `/gsd-plan-phase 26.1 --gaps` | matches | advisory confirmed |

### Live acceptance (paid, operator consent approve-all, $25 ceiling)

h1 PASS, h3 PASS, h4 PASS, h2 FAIL (H2-5, rubric premise), o1 ADJUST x2 (rounds 0/1/2 reached with plan/execute/gap commands; no deny, no cap-trip; 5 reports all <= 15 lines starting `STATUS:`; version PASS; the Opus driver self-parked `needs-human` under stop rule 8 at the second gaps_found closure instead of reaching count 3). Ledger $12.7557.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| PRMT-01 | 29-01, 02, 06, 07, 08 | SATISFIED | SC1 |
| PRMT-02 | 29-04, 06 | SATISFIED | SC2 |
| PRMT-03 | 29-01, 04, 07, 08 | SATISFIED (text + h3/h4 live; o1 third-round park not reached) | SC3 |
| PRMT-04 | 29-02, 05, 07, 08 | SATISFIED (deterministic + h1 + H2-1..4 live) | SC4 |
| PRMT-05 | 29-01, 03 | SATISFIED | SC5 |
| DRIVE-03 | 29-04, 06, 07, 08 | SATISFIED | lint + `driver check` over done-when/budget items; WR-01 edge case advisory |
| DRIVE-01 (prompt half) | 29-08 | PENDING operator sign-off | o1 ADJUST x2; rubric reserves closure for o1 PASS |

No orphaned IDs: every PRMT-01..05, DRIVE-03 and DRIVE-01 appears in at least one PLAN. REQUIREMENTS.md checkboxes/traceability for PRMT-03, PRMT-04, DRIVE-03 still read Pending and should be updated after sign-off.

### Anti-Patterns Found

No TBD/FIXME/XXX in the 34 files changed since 6ffda4c. Review: 0 critical, 7 warnings, 5 info, all `open`. Weighed against the goal: none defeats a success criterion. WR-02 (fail-open/case-sensitive gate pause), WR-06 (gate vs log recovery enforcement) and WR-01 (lint/parser divergence) are hardening of a mechanism whose documented contract (lowercase word, log-level recovery rule, ordinary templates) works and is live-evidenced; they are recommended follow-ups (suggest folding WR-02 into Phase 30/31 or a quick fix before relying on mid-run `control` edits from the TUI). Security: 29-SECURITY.md `threats_open: 0`.

### Human Verification Required

#### 1. Sign the pre-registered unmet outcomes (o1 ADJUST x2, h2 FAIL)

**Test:** Review `acceptance/o1/verdict.txt`, `acceptance/o1/retry/verdict.txt`, `acceptance/h2/verdict.txt` and the H2-5 assessment above.
**Expected:** Operator accepts closing Phase 29 with the real-Opus third-gaps_found park unobserved (carried risk covered by d33 reuse + h4 + deterministic suites) and h2 H2-5 recorded as a rubric flaw; this also decides whether DRIVE-01's prompt half is marked complete.
**Why human:** A risk acceptance on a pre-registered outcome; no further paid attempt may be made (rubric: never a third).

### Gaps Summary

No goal-blocking gaps. All five success criteria are met in the codebase and by deterministic plus live evidence. The sole open item is the operator's risk acceptance of the o1/h2 acceptance outcomes.

---

_Verified: 2026-10-20T14:55:00Z_
_Verifier: Claude (gsd-verifier)_
