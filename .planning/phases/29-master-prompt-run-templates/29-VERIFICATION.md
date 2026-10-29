---
phase: 29-master-prompt-run-templates
verified: 2026-10-29T05:20:00Z
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
  - "src/driver/lock.rs"
  - "src/driver_cli.rs"
  - "src/driver_templates/assets/DIRECTION.md"
  - "src/driver_templates/assets/LOG.md"
  - "src/driver_templates/lint.rs"
  - "src/driver_templates/mod.rs"
  - "src/driverctl/serve.rs"
  - "src/driverctl/store.rs"
  - "src/launcher/assets/MASTER-PROMPT.md"
  - "src/launcher/config_dir.rs"
  - "src/launcher/gate.rs"
  - "src/launcher/prompt.rs"
  - "src/launcher/run.rs"
  - "src/next_cli.rs"
  - "tests/driverctl_serve.rs"
  - "tests/driverctl_store.rs"
  - "tests/launcher_config_dir.rs"
  - "tests/prompt_acceptance.rs"
covered_digest: "v4:sha256:ef63eb7c793a63f99d3bb83eed80fed5b09e1a36d01c44b81f95cd64ea9e5f0c"
behavior_unverified: 0
gaps: []
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "29-REVIEW WR-01: lint's `- control:` matcher and the store's still disagree on which line counts (only half closed)"
    category: other
    reason: "Open in 29-REVIEW-DISPOSITION.md, advisory by instruction. Failure direction is safe (the run holds). PRMT-04 'control edit honoured before next dispatch' is still met: store tests `a_pause_or_stop_is_honoured_in_any_case_and_with_stray_whitespace` and `a_pause_on_any_duplicate_control_line_holds_the_run` pass, and `driver launch` refuses `control: pause` (live run below)."
    evidence_status: "reviewed; behavior exercised by passing named tests and a live CLI run"
  - finding: "29-REVIEW WR-03, WR-08 (link-count refusal reports a held run lock as Unknown), WR-09 (managed `permissions` scalars first-wins), WR-10 (dropped host policy has no diagnostic), IN-01, IN-03, IN-06"
    category: other
    reason: "Open in the disposition; all sit in lock/config_dir/launcher-hardening code or diagnostics, none touches a Phase 29 success criterion. 0 critical in the re-run review (9eb4dff, e734d6e); 29-SECURITY.md threats_open 0 (46276f4). WR-02 and IN-02 are recorded fixed (t3f, v2z). Tracked for follow-up, not fixed here. [inferred — cleanup 2026-10-28]"
    evidence_status: "per 29-REVIEW.md and 29-REVIEW-DISPOSITION.md"
---

# Phase 29: Master Prompt & Run Templates Verification Report

**Phase Goal:** The driver's contract ships with gsd-mm as a versioned prompt and file templates. A new run is one command from a ready DIRECTION.md, and stop-rule precedence is stated explicitly instead of left implicit.
**Verified:** 2026-10-29T05:20:00Z
**Status:** passed
**Re-verification:** Yes. The report of ca515dd went stale: after it, quick batches 261028-t3c/t3d/t3e/t3f and 261028-v2z changed `src/driver/direction.rs`, `src/driver/lock.rs`, `src/driverctl/serve.rs`, `src/driverctl/store.rs`, `src/launcher/config_dir.rs`, `src/launcher/run.rs`, `tests/driverctl_store.rs` and others. The diff of the three Phase 29 files named in the request (direction.rs, serve.rs, store.rs) was read: it is hardening only (guarded lock/`state.json` opens per 27-REVIEW WR-03, a shared `exchange_or_restore` helper per 30-REVIEW WR-12), with no change to the splice contract, the `control` gate or the log grammar. Every must-have was re-checked at HEAD e734d6e with fresh CLI runs and the named suites.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master prompt embedded, versioned, version recorded on the run; requires a fresh subagent per step, 11-field report <= 15 lines, nested `isolation: "worktree"`, async no-respawn, all records via driverctl | VERIFIED | `src/launcher/assets/MASTER-PROMPT.md:1` `<!-- driver-prompt/1.0.0`; `DRIVER_PROMPT_VERSION = "driver-prompt/1.0.0"` at `src/launcher/prompt.rs:25`. Hard rules: 1 fresh subagent (:50), 2 report <= 15 lines recorded verbatim (:54), 3 `isolation: "worktree"` (:58, :196), 4 never re-spawn (:62, :200), 5 every record via `driverctl` (:66, command block :31-40). Version reaches the run through `run.rs` -> `store.rs`; `tests/launcher_e2e.rs` (41 pass) asserts `prompt=driver-prompt/1.0.0` and `prompt=custom:`. `tests/driver_prompt.rs` 14 pass |
| 2 | `driver new <project> <slug>` mints `<yyyymmdd>-<slug>`, creates a run dir with a self-ignoring `.gitignore`, DIRECTION.md and LOG.md with the grammar inline; a ready DIRECTION.md launches in one command | VERIFIED | Live run at HEAD on a scratch registered project: `driver new p my-slug` -> `created run 20261029-my-slug`, rc 0; dir holds `.gitignore` (content `*`), DIRECTION.md, LOG.md (grammar comment inline), prints `check:` and `launch:` one-liners. Duplicate -> rc 1 naming the existing dir; slugs `.bad`, `a/b`, `a b`, empty -> rc 1 with the regex message. Template has no `isolation:` line (grep). `driver check` rc 1 with line-numbered placeholders on the template, rc 0 once filled. `driver launch` on the filled run with `control: pause` refuses rc 1 `DIRECTION.md:40: control is pause; set - control: run to launch` before any spend. `tests/driver_new.rs` 13 pass |
| 3 | Stop rules explicit in prompt and template standing rules with written precedence; stop rules outrank `answer:`, only a budget edit lifts | VERIFIED | `MASTER-PROMPT.md:86-99` `stop-rules:begin/end` with ordered rules 1-8 (control, launcher denial, run budget, divergence, gap rounds, item budget, no progress, needs-human); "Divergence outranks gap rounds" (:93); "A human `answer:` lifts only a `needs-human` park (rule 8) ... lifted only by a budget edit ... An empty `answer:` lifts nothing." Generated DIRECTION.md carries the same block (`DIRECTION.md:53-65`). `tests/driver_prompt.rs` goldens pass |
| 4 | Crash-and-relaunch reconciles every `running` item to exactly one of done/resume/reset/park; `control: pause` mid-dispatch ends the run before the next dispatch | VERIFIED | Prompt reconcile section (`MASTER-PROMPT.md:130-142`, `recovered --action <done|resume|reset|park>` at :39). `driverctl_reconcile` 7, `launcher_reconcile` 7, `driverctl_store` 61, `driverctl_serve` 16 pass, including the pause/stop-honoured and duplicate-control-line tests named in the advisory above |
| 5 | `next <project> --json` exposes the Phase 20 router as an advisory hint, cited as advisory only by the prompt | VERIFIED | Live: `next p --json` -> `{"advisory":true,"decision":"none","reason":"no-target","source":"phase-20-router",...}` rc 0. Prompt `MASTER-PROMPT.md:159` "is advisory", `SUGGESTED_NEXT: <advisory only>` (:214). `tests/next_cli.rs` 12 pass |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified). Plan-level must-haves (D-01..D-25 incl. prohibitions on `next` writes and acceptance spend) were spot-read in the PLAN frontmatter and are covered by the suites above; `tests/prompt_acceptance.rs` 20 pass, 2 ignored (paid live entry points, gated by consent/prereg/ledger).

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/launcher/assets/MASTER-PROMPT.md`, `src/launcher/prompt.rs` | VERIFIED | embedded via `include_str!`; header version agrees with the constant (driver_prompt test) |
| `src/driver_templates/{mod,lint}.rs`, `assets/DIRECTION.md`, `assets/LOG.md` | VERIFIED | exercised end to end by the live CLI run |
| `src/driver/direction.rs` | VERIFIED | shared reader/splice; new WR-12 helper tests present and passing in the lib suite |
| `src/next_cli.rs`, `src/launcher/gate.rs`, `src/driverctl/store.rs`, `src/driverctl/serve.rs` | VERIFIED | suites green |
| `scripts/prompt-acceptance.sh`, `29-P29-RUBRIC.md`, `acceptance/*` | VERIFIED | `tests/prompt_acceptance.rs` green |

### Key Links

| From | To | Status |
|------|----|--------|
| `driver launch` | embedded prompt / `prompt_version` | WIRED (launcher_e2e asserts) |
| `driver launch` | `preflight_direction` lint + control refusal | WIRED (live refusal above) |
| PreToolUse gate / store | DIRECTION `control`, fail closed | WIRED (store/gate tests) |
| `next` | Phase 20 router, advisory | WIRED (live run) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mint / dup / bad slug / lint / launch refusal | `driver new`, `driver check`, `driver launch` on scratch project | as SC2 | PASS |
| advisory hint | `next p --json` | JSON advisory:true, rc 0 | PASS |
| phase suites | `cargo test --no-fail-fast --test driver_prompt --test driverctl_reconcile --test launcher_reconcile --test prompt_acceptance --test driver_new --test driverctl_store --test driverctl_serve --test next_cli --test launcher_e2e --test launcher_config_dir` | 14, 7, 7, 20, 13, 61, 16, 12, 41, 28 passed, 0 failed | PASS |
| regression gate | `rtk proxy cargo test --no-fail-fast` (orchestrator, this run) | 55 suites, 3211 passed, 0 failed | PASS |

Live paid acceptance entry points were not run; the operator sign-off of 2026-10-20 (29-UAT.md, complete 1/1) is carried forward and the code it depended on is covered by green suites.

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
| DRIVE-03 (re-mapped) | 29-04, 06, 07, 08 | SATISFIED | `driver check` lint over done-when/budget items (live rc 1 -> rc 0) |
| DRIVE-01 (prompt half, also claimed by 29-08) | 29-08 | SATISFIED (operator acceptance, 29-UAT.md) | launcher half is Phase 28 |

All IDs appear in PLAN `requirements` and are `[x]` / Complete in REQUIREMENTS.md (lines 86, 239-261, 356, 400-404). No orphaned IDs.

### Anti-Patterns Found

No TBD/FIXME/XXX in the covered implementation files without a follow-up reference (grep). Code review re-run (9eb4dff, e734d6e): 0 critical, 3 new warnings, 1 new info; all advisory (see frontmatter). Security: 29-SECURITY.md `threats_open: 0` (46276f4).

### Human Verification Required

None. The sole prior item was signed off by the operator (29-UAT.md) and is not re-raised.

### Gaps Summary

No gaps. All five success criteria hold at HEAD e734d6e; the later hardening commits did not remove or weaken any behavior this phase delivers. Open review warnings are advisories.

---

_Verified: 2026-10-29T05:20:00Z_
_Verifier: Claude (gsd-verifier)_
