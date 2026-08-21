---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-21T04:30:00Z
status: gaps_found
score: 4/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/5
  gaps_closed:
    - "CR-01 (original): --goal X --dry-run rendered a false 'complete and honest sequence' of one empty command"
    - "CR-02 (original): the approval's plan half was bound by invertible FNV-1a-64, not SHA-256"
    - "WR-01 (original, partial-gap half): ApprovalRefusal::PlanChanged was unreachable from production (a value compared against itself)"
    - "WR-05 (original, partial-gap half): target_phase was stored and rendered unbounded, reaching the terminal and run.json unsanitized"
  gaps_remaining:
    - "Criterion 1's honesty/review-integrity chain is broken again, by two NEW Critical defects the code review (21-REVIEW.md) found in the gap-closure diff itself and which I independently reproduced against the built binary at HEAD (92a4b4f)"
  regressions: []
gaps:
  - truth: "A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs (ROADMAP criterion 1)"
    status: failed
    reason: "The two mechanisms this criterion depends on — an honest preview and a validated approval token — are each broken again by defects introduced as a side effect of this cycle's own gap-closure work. Both are Critical per 21-REVIEW.md and both are independently reproduced by this verifier against the built binary at HEAD (commit 92a4b4f), not merely read in the review or SUMMARYs."
    artifacts:
      - path: "src/driver/mod.rs"
        issue: "review-CR-02 (new; distinct from the original CR-01 this cycle closed): `command_source` (:360-377) trims and rejects a blank `--goal` (:370-374) but applies no such check to `--command` — `(Some(command), None) => Ok(CommandSource::Command(command.to_string()))` (:367) accepts `Some(\"\")` and `Some(\"   \")` unchanged. `preview_text` then routes it to `dry_run::build_report(project, \"\")`, which renders `vec![\"\"]` under the SAME 'complete and honest sequence' header the whole 21-07..21-10 cycle exists to make honest for the Goal source. I reproduced this independently: `drive()` with `command: Some(\"\".into())`, `dry_run: true` returns `Ok(())` and renders `  1 command in the sequence:\\n    1. ` — byte-for-byte the defect class the original CR-01 named, just via a command source the fix's own regression test never enumerated. `every_command_source_renders_a_preview_with_no_empty_numbered_command` (:1460-1520-ish) only constructs `CommandSource::Command(\"/gsd:progress\".to_string())` — a non-empty payload — so the enumerated invariant test the plan wrote specifically to stop a future source repeating CR-01 passes vacuously against the one source that already had the bug. On a real (non-dry-run) invocation this also records `gsd_command: \"\"\"` in `run.json`, which `ROUTED_RECORD_MARKER`'s own doc (:393-397) says must never happen because `\"\"` already means 'field absent' on the tolerant read path (D-30)."
      - path: "src/driver/mod.rs"
        issue: "review-CR-01 (new; distinct from the original WR-01 this cycle closed): `drive` (:460-778) calls `run::GoalDecomposition::decompose` at :731-738 — a real process spawn and a real model consultation — BEFORE `approve_plan` is called at :757-760, and `approve_plan`'s own body is where `journal::parse_approval_token` first runs (:825-830), despite its doc comment (:822-824) claiming the parse happens 'before any of the work below.' That claim is true only inside `approve_plan`; at the `drive`-level ordering it is false. I reproduced this independently against the real seam fixture (`tests/fixtures/fake-claude-seam.sh`): a real, non-dry-run invocation with `--goal 'x' --approved-plan 'total-garbage-no-separator'` returns `Err(PlanApprovalMalformed(SeparatorAbsent))` as expected, but only AFTER exactly one seam spawn is recorded on disk (`seam-spawns` shows 1 line) — a full model consultation was spent on a token a pure string check could have refused for free. Separately, `--dry-run` returns from `drive` at line 638, well above the decompose/approve_plan region (:731-830), so a goal-only `--dry-run` invocation never reaches `parse_approval_token` at all: I reproduced `--goal 'x' --dry-run --approved-plan 'total-garbage-no-separator'` exiting `Ok(())` with a clean preview and zero mention of the malformed token. That is precisely the preview/real asymmetry `drive`'s own doc forbids at :438-446 and :470-477 (WR-09): 'each is answered identically whether or not the run is real.'"
    missing:
      - "Reject a blank or whitespace-only `--command` at the same seam and in the same register `--goal` already is refused, per 21-REVIEW.md's suggested fix, and extend `every_command_source_renders_a_preview_with_no_empty_numbered_command`'s enumeration to include `CommandSource::Command(String::new())` and `CommandSource::Command(\"   \".into())` so the invariant the test's name asserts is the invariant it actually checks."
      - "Move `parse_approval_token` to the pure-refusal group in `drive`, above the dry-run branch, alongside the other invocation-shape refusals (target-phase validity, bounds, escalation cap) — all of which are already positioned there for the documented reason that they must be 'answered identically whether or not the run is real.' A malformed token should be refused before decomposition spends a consultation, and should be refused identically in preview mode."
  - truth: "Two Warning-level accuracy defects in the gap-closure diff itself, both flagged by 21-REVIEW.md, both independently confirmed by this verifier; recorded because they weaken confidence in this phase's own self-checking machinery even though neither is independently exploitable today"
    status: partial
    reason: "Neither defeats a ROADMAP truth on its own (no live call site currently reaches either hole), but both are doc/guard-accuracy defects this verifier confirmed by direct code reading rather than accepting on the review's word, and 21-secure-phase or the next review pass should close them."
    artifacts:
      - path: "tests/spawn_seam_guard.rs"
        issue: "review-WR-01: guard six's header (:1691-1702) states 'Two stated over-approximations' and concludes 'Both fail in the over-detection direction — loud, not silent — which is the direction a guard may err in,' as a characterisation of the guard as a whole. That is not true of the guard: `TERMINAL_WRITE_HOME` (:1711) pins the scan to `src/driver/run.rs` only, and `JournalRun::finish` is `pub fn` (confirmed at `src/journal/mod.rs:1771` on a `pub struct`), so a terminal write added from any other file — `src/driver/kill.rs`, `src/app.rs`, a UI screen — would be invisible to the guard: a silent under-detection, the opposite of what the header claims for the guard overall. I confirmed via `grep -rn '\\.finish(' src/` that no other call site currently exists outside `src/driver/run.rs` and `src/journal` itself, so there is no live exploit today — but the header's claim is inaccurate as written, and a fifth terminal-write path added anywhere else in the tree next year would inherit exactly the ambiguity guard six exists to remove, without the guard ever firing."
      - path: "src/driver/goal.rs"
        issue: "review-WR-03: the doc paragraph at `plan_digest` (~:750-756) states 'A record carrying a legacy `fnv1a64:` plan digest is not migrated and needs no migration: the whole prefixed string is compared, so it re-checks as ApprovalRefusal::PlanChanged and fails closed,' naming `tests/driver_goal_seam.rs::a_recorded_approval_carrying_a_legacy_fnv1a64_plan_digest_re_checks_as_stale` as proof. I confirmed by grep that `RunRecord::approved_plan` (`src/journal/mod.rs:1422`) is written in exactly one production place and never read back and re-checked: `recheck_approval`'s two production call sites are `src/driver/mod.rs:868` (the token parsed off argv inside `approve_plan`, same invocation) and `src/driver/run.rs:2322` (the spawn-gate re-check, using the in-memory `ApprovedPlan` from the same run) — neither deserializes a `run.json` from disk. So no recorded legacy **record** is ever fed back into `recheck_approval`; the doc describes a path production cannot take. The value a user could actually hold from an earlier dev build is a legacy single-half **token**, which fails at `parse_approval_token` with `ApprovalTokenError::SeparatorAbsent`, not with `PlanChanged`. The named test constructs an `ApprovedPlan` by hand and calls `recheck_approval` directly, which pins the predicate but not any route into it."
    missing:
      - "Widen guard six's scan (or add an explicit `(file, fn)` allowlist for `finish_run`, the shape guard one already uses for `from_registry`) to cover every file under `src/`, OR correct the guard's own header to name the file-scoping as a third, silent under-detection rather than folding it into the 'loud, not silent' claim."
      - "Reword the `plan_digest` doc paragraph to describe what actually fails closed: a legacy single-half token on argv is refused by `parse_approval_token` as `SeparatorAbsent`; no recorded `ApprovedPlan` is ever re-read. Keep the `recheck_approval` unit test but rename it or its doc to say it pins the predicate rather than a production record path."
deferred: []
human_verification: []
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-21T04:30:00Z
**Status:** gaps_found
**Re-verification:** Yes — after gap closure (second cycle)

## Goal Achievement

This is a **second** re-verification cycle. The first `21-VERIFICATION.md` scored
3/5 and named CR-01 (original), CR-02 (original), WR-01 (original) and WR-05
(original) as the causes. Plans 21-07…21-10 closed all four — confirmed below,
by re-running the relevant test suites myself rather than trusting the
SUMMARYs. A code review of that gap-closure diff (`21-REVIEW.md`, committed
`92a4b4f`) then found **two NEW Critical defects**, introduced as a side effect
of the fixes themselves, plus two Warning-level accuracy defects in the new
guard/doc machinery. This report independently reproduces both Criticals
against the built binary — not by reading the review, but by writing and
running my own reproduction tests directly against `drive()` at HEAD — and
independently confirms both Warnings by direct code/grep inspection.

**Both new Criticals are confirmed live at HEAD (`92a4b4f`).** They are a
different pair of bugs than the ones the first verification cycle named
(CR-01/CR-02 original are genuinely closed), but they land in the same place —
the honesty of the dry-run preview and the integrity of the approval-token
check — so **criterion 1 remains not met**, for a new reason.

### Observable Truths

| # | Truth (ROADMAP success criterion) | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✗ FAILED | The original CR-01/CR-02 are genuinely closed (see below), but two NEW Critical defects reintroduce the same class of dishonesty this criterion requires be absent. **review-CR-02 (new):** `--command '' --dry-run` (and `--command '   '`) renders `1 command in the sequence:` and a bare numbered entry under the "complete and honest sequence" header — I reproduced this directly by calling `drive()` with an empty `command` and `dry_run: true`, which returned `Ok(())` and printed exactly that shape. `command_source` (`src/driver/mod.rs:360-377`) trims-checks `--goal` but not `--command`, and the new invariant test `every_command_source_renders_a_preview_with_no_empty_numbered_command` only enumerates a non-empty `Command` payload, so it passes vacuously against the one source that already had the bug. **review-CR-01 (new):** I reproduced, against the real seam fixture, that a real run with a malformed `--approved-plan` token spends exactly one live model consultation (confirmed via the seam's on-disk spawn ledger, `seam-spawns` = 1) before being refused — `drive` decomposes the goal at `src/driver/mod.rs:731-738` before `approve_plan` parses the token at `:825-830` — and that `--dry-run` with the same malformed token exits `Ok(0)` with a clean preview, never validating the token at all. Both reproductions are recorded below with the actual commands and output, not merely cited from `21-REVIEW.md`. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `tests/driver_goal_seam.rs` — 19/19 passing, re-run at HEAD. The original WR-01 gap ("`ApprovalRefusal::PlanChanged` unreachable from production, a value compared against itself") is genuinely closed: `a_re_decomposition_that_changed_the_plan_is_refused_as_a_changed_plan_not_as_changed_files` and `a_disclosed_file_rewritten_under_an_approval_is_refused_as_changed_files_not_a_changed_plan` both exist and pass, and `approve_plan`'s recorded plan digest now arrives from the caller's parsed token (`src/driver/mod.rs:825-830`) rather than from the plan just observed. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | `tests/driver_escalation_cap.rs` — 8/8 passing, re-run at HEAD. The original WR-02 anti-pattern ("`escalations_used` unstamped on 3 of 4 terminal-write paths") is genuinely closed: `tests/spawn_seam_guard.rs` guard six (`every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper`) passes, and all four production terminal writes in `src/driver/run.rs` now route through `finish_run`, confirmed by `grep -c 'fn finish_run' src/driver/run.rs` = 1. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes | ✓ VERIFIED (reconfirmed) | `tests/driver_injection_corpus.rs` — 12/12 non-ignored tests passing, re-run at HEAD (10 `--ignored` live-binary arms not re-executed by this verifier, same as the prior cycle — they require credentialed API calls). Untouched by any 21-07..21-10 plan. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED (reconfirmed) | `tests/driver_refusal_record.rs` — 9/9 passing, re-run at HEAD. Untouched by any 21-07..21-10 plan. |

**Score:** 4/5 truths verified (0 present-but-behavior-unverified)

### Independent Reproduction (not taken from 21-REVIEW.md)

I wrote and ran two standalone reproduction tests directly against `drive()`
at HEAD (`92a4b4f`), deleted afterward and not committed, to confirm the
review's two Critical findings myself rather than accept them on the review's
word.

**review-CR-02 (empty `--command --dry-run`):**

```
RESULT (whitespace --command, dry-run): Ok(())
...
== GSD commands this run would issue ==
...
  1 command in the sequence:
    1.
...
```

**review-CR-01 (malformed token spends a consultation, and dry-run skips validation):**

```
$ drive() with --goal 'get phase 21 verified' --approved-plan 'total-garbage-no-separator' (real run, seam stand-in)
RESULT: Err(PlanApprovalMalformed(SeparatorAbsent))
SEAM SPAWNS: 1
CONFIRMED: a model consultation (seam spawn) happened BEFORE the malformed token was refused.

$ drive() with the same goal/token, dry_run: true
DRY-RUN RESULT with malformed token: Ok(())
```

Both reproductions used the project's own test harness patterns
(`tests/driver_dry_run.rs`'s `config_for`/`args`, `tests/driver_goal_seam.rs`'s
seam fixture and `fake-claude-seam.sh`) so the setup is representative of the
project's own established test methodology, not a synthetic shortcut.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/dry_run.rs` (`PreviewScope::GoalNotDecomposed`, `build_goal_report`) | An honest goal-only preview | ✓ VERIFIED | Present, wired, tested; `tests/driver_dry_run.rs` 12/12 passing including the goal-only regression tests. The Goal arm of the fix is genuinely closed. |
| `src/driver/mod.rs` (`CommandSource`, `command_source`) | Resolved-once command source, exhaustive `preview_text` match | ⚠️ VERIFIED but incomplete | The type is exhaustive (compiles, no fall-through arm) and closes the ORIGINAL CR-01 for the Goal variant — but `command_source` (the function computing the resolved value) still admits an empty/blank `Command` variant unchecked, which is the review-CR-02 (new) root cause. The exhaustiveness fix and the emptiness-validation fix are two different properties; only the first was actually delivered. |
| `src/driver/goal.rs` (`plan_digest` via `sha256_digest`) | Collision-resistant plan digest | ✓ VERIFIED | `sha256_digest` used, confirmed by grep and by an independent Python SHA-256 computation recorded in 21-08's SUMMARY. Original CR-02 genuinely closed. |
| `src/journal/mod.rs` (`APPROVAL_TOKEN_SEPARATOR`, `render_approval_token`, `parse_approval_token`, `recheck_approval`) | Two-half approval token, real plan-half comparison | ✓ VERIFIED for the comparison itself; ⚠️ the CALLER's ordering is the new defect | The token vocabulary and the comparison logic are real and tested (19/19 `driver_goal_seam` tests). The new CR-01 defect is not in this module — it is that `drive` (the caller) invokes decomposition before calling into the code that parses the token. |
| `tests/spawn_seam_guard.rs` (guard six) | Single stamped terminal-write call site | ⚠️ VERIFIED but overclaims its own limits | The guard passes and the property (one call site in its scanned region) holds. Its own header text overclaims: it names two over-approximations and calls both "loud, not silent," but the file-scoping (`TERMINAL_WRITE_HOME`) is a third, unstated, silent under-detection risk given `JournalRun::finish` is `pub`. No live exploit today (grep confirms no other `.finish(` call site exists), but the guard's self-description is inaccurate. |
| `src/ui/screens/driver_confirm.rs` (`do_toggle_opt_in` revert) | Restore, not re-mint, on a failed save | ✓ VERIFIED | 18/18 `ui::screens::driver_confirm` lib tests passing; the drift-not-laundered test (`a_failed_save_after_a_withdrawal_does_not_rebaseline_a_drifted_disclosure`) passes. Original WR-04 genuinely closed. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `src/driver/mod.rs` (`command_source`) | `src/driver/dry_run.rs` (`preview_text`) | An empty/blank command source is refused, never rendered | ✗ NOT WIRED | `command_source` accepts `Command("")`/`Command("   ")` unchecked; `preview_text` renders them through `dry_run::build_report`, which pushes the empty string as a numbered command entry. review-CR-02 (new). |
| `src/driver/mod.rs` (`drive`) | `src/journal/mod.rs` (`parse_approval_token`) | The token is validated as a pure invocation-shape refusal, above decomposition and above the dry-run branch, alongside the other WR-09-grouped refusals | ✗ NOT WIRED | The parse happens inside `approve_plan`, called AFTER `decompose` (`src/driver/mod.rs:731` vs `:825`), and `approve_plan` is never called at all on the `--dry-run` path (which returns at `:638`). review-CR-01 (new). |
| `src/driver/goal.rs` (`plan_digest`) | `src/journal/mod.rs` (`approval_digest`) | The approval's plan half is cryptographically bound | ✓ WIRED | Confirmed; original CR-02 closed. |
| `src/driver/mod.rs` (`approve_plan`) | `src/journal/mod.rs` (`recheck_approval`) | The recorded plan digest arrives from the caller's token, not from the plan just observed | ✓ WIRED | Confirmed; original WR-01 closed, `PlanChanged` reachable and tested end to end. |
| `tests/spawn_seam_guard.rs` (guard six) | every `src/` file's terminal writes | A single call site is provable, not merely asserted for one file | ⚠️ PARTIAL | Scoped to `src/driver/run.rs` only; `JournalRun::finish`'s `pub` visibility means a future terminal write elsewhere is invisible to the guard, contrary to its own "loud, not silent" self-description (review-WR-01). |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 21-04, 21-07, 21-09 | User states a goal once, driver pursues it without further input | ⚠️ PARTIAL (unchanged disposition from prior cycle, new cause) | The multi-command pursuit mechanism (criterion 2) is verified. The "review before anything runs" half is again undermined — by review-CR-01/review-CR-02 (new) rather than by the original defects, which are closed. |
| DRIVE-03 | 21-01, 21-03, 21-04, 21-07, 21-08, 21-09 | Goal decomposed into structured, machine-checkable, reviewable plan | ✗ BLOCKED (unchanged disposition from prior cycle, new cause) | review-CR-02 directly targets the honest-preview half of "reviewable"; review-CR-01 targets the "machine-checkable and validated before anything runs" half, since a malformed approval spends a live consultation before refusal and is not checked at all in preview mode. |
| DRIVE-04 | 21-02, 21-04, 21-06, 21-10 | Model escalation capped per run, exceeding it parks | ✓ SATISFIED | Criterion 3 verified; original WR-02 (unstamped `escalations_used` on 3/4 terminal paths) genuinely closed by 21-10 and reconfirmed by this verifier (guard six passes, tests pass). |
| SAFE-07 | 21-01, 21-03, 21-05, 21-06, 21-08, 21-10 | `.planning/` content passed inside an explicit untrusted boundary, never concatenated into instructions | ✓ SATISFIED | Criterion 4 verified. Original WR-05 (unbounded `target_phase`) and WR-04 (opt-in revert re-baselining) both genuinely closed and reconfirmed. review-WR-03 (new; the stale "legacy record fails closed" doc paragraph in `src/driver/goal.rs`) is a documentation-accuracy defect, not a live boundary break, and does not affect this disposition. |
| SAFE-08 | 21-01, 21-05, 21-06 | Model's action constrained to fixed enum; free-form shell strings never executed | ✓ SATISFIED | Criterion 5 verified, untouched by this cycle's plans. |

No orphaned requirements: the union of `requirements:` declared across all ten
plans (21-01 through 21-10) matches DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07,
SAFE-08 exactly, and REQUIREMENTS.md's phase-21 mapping names the same five.

**REQUIREMENTS.md was checked and left untouched, deliberately.** It still
reads Pending (DRIVE-01, DRIVE-03) / Gaps Found (DRIVE-04, SAFE-07, SAFE-08),
consistent with commit `828d7cc`'s revert of a premature Complete marking and
with all four gap-closure executors' explicit decision to leave that call to
verification. DRIVE-04, SAFE-07 and SAFE-08 now have strong, reconfirmed
evidence and could reasonably be marked Complete on their own merits, but this
phase's overall status remains `gaps_found` (criterion 1 still fails), so I am
not editing per-requirement status mid-phase — that call belongs to whichever
step next processes a `passed` verification for this phase as a whole.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/driver/mod.rs` | `:367` | `command_source`'s `Command` arm has no emptiness check, unlike the adjacent `Goal` arm two lines below it (`:372-374`) | 🛑 Blocker | Direct cause of review-CR-02 (new); reproduced independently. |
| `src/driver/mod.rs` | `:731` vs `:825` | `decompose` (a real spawn) is called 94 lines before `parse_approval_token` (a pure string check) runs inside `approve_plan` | 🛑 Blocker | Direct cause of review-CR-01 (new); reproduced independently — a typo'd token costs a live model consultation before refusal, and is never checked at all on the `--dry-run` path. |
| `src/driver/mod.rs` (`every_command_source_renders_a_preview_with_no_empty_numbered_command`) | `~:1465-1467` | The array meant to guard against exactly this defect class enumerates only non-empty payloads | ⚠️ Warning | The regression test 21-07 wrote specifically to stop a future command source repeating CR-01 does not cover the one existing source (`Command`) that already had the bug, so it currently passes for the wrong reason. |
| `tests/spawn_seam_guard.rs` | `:1691-1702` | Guard six's header claims both its stated over-approximations "fail in the over-detection direction — loud, not silent," which is not true of the (unstated) file-scoping limitation | ⚠️ Warning | review-WR-01 (new); confirmed by grep that `JournalRun::finish` is `pub` and the scan covers only `src/driver/run.rs`. No live exploit today, but the guard's self-description is inaccurate. |
| `src/driver/goal.rs` | `~:750-756` | The `plan_digest` doc's "a legacy `fnv1a64:` record fails closed as `PlanChanged`" claim describes a path production cannot take (`ApprovedPlan` is written but never deserialized and re-checked) | ⚠️ Warning | review-WR-03 (new); confirmed by grep across `recheck_approval`'s two production call sites. Not exploitable — both directions genuinely fail closed — but the doc names the wrong mechanism. |

No `TBD`/`FIXME`/`XXX` unreferenced debt markers were found in the files
reviewed.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| `driver_goal_seam` suite | `rtk proxy cargo test --test driver_goal_seam` | 19 passed | ✓ PASS |
| `driver_refusal_record` suite | `rtk proxy cargo test --test driver_refusal_record` | 9 passed | ✓ PASS |
| `driver_escalation_cap` suite | `rtk proxy cargo test --test driver_escalation_cap` | 8 passed | ✓ PASS |
| `driver_dry_run` suite | `rtk proxy cargo test --test driver_dry_run` | 12 passed | ✓ PASS (does not cover empty `--command`, per review-CR-02) |
| `spawn_seam_guard` suite | `rtk proxy cargo test --test spawn_seam_guard` | 24 passed | ✓ PASS |
| `async_blocking_guard` suite | `rtk proxy cargo test --test async_blocking_guard` | 9 passed | ✓ PASS |
| `driver_optin`, `driver_kill`, `driver_kill_startup` suites | `rtk proxy cargo test --test driver_optin --test driver_kill --test driver_kill_startup` | 3 / 3 / 1 passed | ✓ PASS |
| `driver_injection_corpus` (non-ignored) | `rtk proxy cargo test --test driver_injection_corpus` | 12 passed, 10 ignored | ✓ PASS (live-binary arms not re-run, same as prior cycle) |
| Independent reproduction: empty `--command --dry-run` | standalone `drive()` call, non-committed test | renders `1 command in the sequence:` / `1.` under the honest-sequence header | ✗ CONFIRMS review-CR-02 |
| Independent reproduction: malformed `--approved-plan` token | standalone `drive()` call against the seam fixture | 1 seam spawn before refusal (real run); `Ok(0)` with no validation (`--dry-run`) | ✗ CONFIRMS review-CR-01 |
| `rtk proxy cargo build --all-targets` | — | clean | ✓ PASS |
| `rtk proxy cargo clippy --lib -- -D warnings` | — | clean | ✓ PASS |
| Whole-suite regression | `rtk proxy cargo test --workspace -- --test-threads=4` | 1018 lib + all integration binaries; one pre-existing flake (`envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting`, a documented write-then-exec race, `deferred-items.md`) | ✓ PASS (flake is pre-existing, documented, not caused by any `21-*` plan) |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Human Verification Required

None required for a status determination. The two new blocking issues
(review-CR-01, review-CR-02) are code-level, objectively confirmed defects —
independently reproduced against the built binary by this verifier — rather
than judgment calls.

### Gaps Summary

**The good news first, because it is real and hard-won:** every gap the prior
verification cycle named is genuinely closed. `--goal X --dry-run` — the
phase's headline invocation — now renders an honest, empty command list with a
scope that says why. The approval's plan half is SHA-256, not invertible
FNV-1a-64. `ApprovalRefusal::PlanChanged` is reachable from production instead
of comparing a value against itself. `target_phase` is bounded at construction
and sanitized at render. `escalations_used` is stamped on all four terminal
paths. A failed opt-in-withdrawal save restores the granted record instead of
minting a fresh, re-baselined one. All of this is reconfirmed by this
verifier re-running the relevant suites myself, not by trusting the SUMMARYs.

**But criterion 1 fails again, for a different reason.** The fixes themselves
introduced two new Critical defects, both flagged by `21-REVIEW.md` and both
independently reproduced by me against the built binary rather than accepted
on the review's word:

1. **review-CR-02**: `command_source` validates `--goal` for emptiness but not
   `--command`, so `--command '' --dry-run` renders the exact "empty command
   as the honest sequence" defect class the entire 21-07..21-10 cycle exists to
   eliminate — just through an input shape the new regression test's
   enumeration never included. This is not a new *kind* of bug; it is the
   *same* bug, on a sibling input the fix's own guard was specifically
   designed to make impossible for any command source, and missed for the one
   source (`Command`) that could already reach it.
2. **review-CR-01**: a malformed `--approved-plan` token is parsed inside
   `approve_plan`, which `drive` calls after decomposition has already spawned
   a process and spent a model consultation — and `--dry-run` never reaches
   `approve_plan` at all, so a malformed token is never validated in preview
   mode. This breaks the same invariant `drive`'s own doc states for every
   other invocation-shape refusal: "each is answered identically whether or
   not the run is real."

Both are Critical, both are reproduced independently in this report (not
merely cited from the review), and both land squarely on criterion 1's
"review before anything runs" guarantee. **Criterion 1 is therefore recorded
✗ FAILED again**, with a different root cause than the first cycle.

Two further Warning-level findings — the spawn-seam guard's own header
overclaiming that its known limits "fail loud, not silent" when its file
scoping is actually a silent gap (review-WR-01), and a doc paragraph in
`src/driver/goal.rs` describing a legacy-record re-check path production
cannot take (review-WR-03) — are confirmed accurate by direct code
inspection but are not independently exploitable today (no other `.finish(`
call site exists yet; both directions of the legacy-approval check genuinely
fail closed either way). They are recorded as anti-patterns rather than
blocking gaps, matching how the prior verification handled the analogous
WR-02/WR-04 findings before they were closed.

**Recommendation:** route back through `/gsd-plan-phase --gaps` for a third,
narrowly-scoped closure plan targeting review-CR-01 and review-CR-02 only —
both are small, well-understood fixes (an emptiness check mirroring the one
`--goal` already has; moving one function call above another). Given this is
the second time a gap-closure cycle for this phase has introduced a new
Critical in the process of fixing the last one, a plan for this round should
also add a regression test whose enumeration is proved exhaustive over the
resolved type's variants (not a hand-picked payload per variant), so a third
recurrence of this specific failure mode is a compile-time or test-time
certainty rather than something the next reviewer has to notice by hand. The
two Warning-level accuracy findings (review-WR-01, review-WR-03) can ride the
same plan cheaply or be deferred to the next scheduled review pass — neither
blocks re-verification of criterion 1.

---

_Verified: 2026-08-21T04:30:00Z_
_Verifier: Claude (gsd-verifier)_
