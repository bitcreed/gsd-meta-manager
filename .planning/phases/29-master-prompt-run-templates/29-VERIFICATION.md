---
phase: 29-master-prompt-run-templates
verified: 2026-10-30T00:15:00Z
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
covered_digest: "v4:sha256:9728eb291995b19524696ff0cc2ee27afa559692851b755915e24b912656aed8"
behavior_unverified: 0
gaps: []
re_verification:
  previous_status: passed
  previous_score: 5/5
  previous_verified: 2026-10-29T05:20:00Z
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "29-REVIEW WR-14 (multi-line PEM printed as plain text is not redacted by the per-line capture pump), WR-15 (pump-thread spawn failure leaves the child unwatched), IN-07..IN-11 (SAFE-04 pump, quick 261029-1dp)"
    category: security
    reason: "New in the f98080d re-run review, recorded open in 29-REVIEW-DISPOSITION.md (59952a3). They concern the master's captured stdout/stderr in `launch-N.jsonl` (src/launcher/process.rs), which is SAFE-04, a Phase 16 requirement (REQUIREMENTS.md:341), not a Phase 29 requirement or success criterion; no Phase 29 PLAN lists SAFE-04. The pump strictly adds redaction that did not exist before; driverctl records (the Phase 29 surface) do not pass through it. Judged non-blocking for Phase 29; WR-14 is tracked on the Phase 28/16 side. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code; pump covered by 11 passing launcher::process unit tests (Phase 28 report)"
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
**Verified:** 2026-10-30T00:15:00Z at HEAD 59952a3 (source changes since e734d6e: quick 261029-1dp f4cfa01 and ce86206 only)
**Status:** passed
**Re-verification:** Yes. The report of ca515dd went stale: after it, quick batches 261028-t3c/t3d/t3e/t3f and 261028-v2z changed `src/driver/direction.rs`, `src/driver/lock.rs`, `src/driverctl/serve.rs`, `src/driverctl/store.rs`, `src/launcher/config_dir.rs`, `src/launcher/run.rs`, `tests/driverctl_store.rs` and others. The diff of the three Phase 29 files named in the request (direction.rs, serve.rs, store.rs) was read: it is hardening only (guarded lock/`state.json` opens per 27-REVIEW WR-03, a shared `exchange_or_restore` helper per 30-REVIEW WR-12), with no change to the splice contract, the `control` gate or the log grammar. Every must-have was re-checked at HEAD e734d6e with fresh CLI runs and the named suites.

**Re-verification (2026-10-30):** The report above (2026-10-29T05:20Z, passed 5/5, HEAD e734d6e) read `stale` again because quick 261029-1dp changed covered inputs. f4cfa01 adds a line-redacting capture pump in `src/launcher/process.rs` and edits `src/launcher/run.rs` (imports `CAPTURE_FINISH_BUDGET`/`MAX_STREAM_LINE` from `process.rs`; `supervise` takes the capture handle and calls `capture.finish(CAPTURE_FINISH_BUDGET)` before the stream file is read; 8 insertions, 5 deletions). ce86206 removes the dead `AgentRuntime` config key (`src/config.rs`, `src/executor/runtime.rs` deleted, `registry.rs`, doc comments). Neither touches the master prompt, the templates, `driver new/check`, the `control` gate, the driverctl store/serve, the log grammar or `next`; `grep AgentRuntime src tests` now hits only explanatory comments. Every must-have was re-checked at HEAD 59952a3 with fresh CLI runs on a scratch registered project and the named suites; the covered-input fingerprint was regenerated with `verification.fingerprint`. Pattern: Phase 28's c2f2083.

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
| phase suites | `cargo test --no-fail-fast --test driver_prompt --test driverctl_reconcile --test launcher_reconcile --test prompt_acceptance --test driver_new --test driverctl_store --test driverctl_serve --test next_cli --test launcher_e2e --test launcher_config_dir` | 14, 7, 7, 20, 13, 61, 16, 12, 41, 28 passed, 0 failed (re-run at HEAD 59952a3; prompt_acceptance also 2 ignored paid entry points) | PASS |
| regression gate | `rtk proxy cargo test --no-fail-fast` (orchestrator, this run, HEAD pre-review commits) | 54 test binaries + doctests, 3216 passed, 0 failed, 17 ignored, exit 0 | PASS |
| re-verification live CLI (this session) | `add`, `driver new proj my-slug`, repeat, `.bad`, `a/b`, `driver check`, `driver launch`, `next proj --json` | created `20261030-my-slug` rc 0 (dir: self-ignoring `.gitignore` = `*`, DIRECTION.md, LOG.md); duplicate rc 1 naming the dir; bad slugs rc 1 with regex message; `check` rc 1 with line-numbered placeholders; `launch` refuses the unfilled template before spend; `next --json` `{"advisory":true,"decision":"none","reason":"no-target","source":"phase-20-router"}` rc 0; template has 0 `isolation:` lines | PASS |

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

No TBD/FIXME/XXX in the covered implementation files without a follow-up reference (grep). Debt-marker grep (`TBD|FIXME|XXX`) over the Phase 29 implementation files, `process.rs`, `run.rs`, `driverctl/`, `driver/`, `driver_templates/` and the acceptance script: none. Code review re-run (f98080d, disposition 59952a3): 15 findings, 0 critical, 7 warning, 8 info; 7 new (WR-14, WR-15, IN-07..IN-11), 15 open, 7 fixed, 1 skipped; all advisory (see frontmatter). WR-14 (per-line pump cannot redact a multi-line plain-text PEM) is security-relevant but is SAFE-04, a Phase 16 requirement, outside every Phase 29 success criterion and PLAN `requirements`. Security: secure-phase re-run 5383f6e, `threats_open` 0 -> 0, 29/29 closed, T-29-20/21/22 `run.rs` citations refreshed. UI review hook skipped: no 29-UI-SPEC.md and 261029-1dp has no TUI surface. [inferred — cleanup 2026-10-28]

### Human Verification Required

None. The sole prior item was signed off by the operator (29-UAT.md) and is not re-raised.

### Gaps Summary

No gaps. All five success criteria hold at HEAD 59952a3; the SAFE-04 capture pump and the dead-key removal (261029-1dp) leave every behavior this phase delivers intact. Open review warnings, including the new WR-14 and WR-15, are advisories outside the Phase 29 requirement set.

---

_Verified: 2026-10-29T05:20:00Z_
_Verifier: Claude (gsd-verifier)_
