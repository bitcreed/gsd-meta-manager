---
phase: 19-gitsafe-git-blast-radius-envelope
verified: 2026-08-29T00:00:00Z
status: human_needed
score: 6/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 5/5
  gaps_closed:
    - "G-19-1: SECTION_ENVELOPE reshaped to one-sentence-then-two-cases-then-conclusion (250 -> 200 whitespace tokens), all seven pinned substrings verbatim, pin test byte-unchanged, every residual disclosure preserved, legibility control observed RED before the rewrite and GREEN after"
    - "G-19-4: tests/driver_lock.rs shared-alias race closed — six per-test aliases replace the shared ALIAS, a source-scanning gate observed RED against the pre-fix file and GREEN after, an early-dying child is now reported with its exit status and captured stderr instead of a 30s lock timeout"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Read the shipped SECTION_ENVELOPE (src/envelope/advisory.rs:239-262, quoted in full below) and confirm it now reads as legible candour — the specific defect the UAT raised was legibility, not accuracy, and the delivered text (200 tokens against a corrected 215 cap) differs from the ~161-word draft the user reviewed during the UAT conversation, so the final shipped wording has not itself been shown to the user."
    expected: "One opening sentence stating the ceiling, then the guaranteed case with visibly indented examples, then the not-guaranteed case with visibly indented examples, then the server-side branch-protection recommendation as the closing conclusion — matching the shape the user asked for verbatim ('One sentence, then an explanation for both cases and examples?'). All nine residual disclosures must read as still present and un-softened."
    why_human: "19-09-SUMMARY.md's own coverage table (D1) records this as human_judgment: true — a token count and a pin list prove the parts are present, not that the layout is now actually legible or that the text still reads as candour rather than a hedge. This is the identical axis PITFALLS names as most dangerous: an overstated or under-communicated safety claim is worse than a stated limitation, because it gets trusted."
---

# Phase 19: GITSAFE — Git & Blast-Radius Envelope Verification Report

**Phase Goal:** Autonomous git operations are bounded by mechanisms the agent cannot argue its way past
**Verified:** 2026-08-29T00:00:00Z
**Status:** human_needed
**Re-verification:** Yes — gap-closure round (`/gsd-execute-phase 19 --gaps-only`, plans 19-09 and 19-10) following the 19-UAT.md pass that produced G-19-1 and G-19-4.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria — regression check)

None of these criteria's enforcing code (`policy.rs`, `hooks.rs`, `cred.rs`, `scan.rs`, `ledger.rs`) changed between the prior verification (`3ba7950`, pre-gap-closure) and `HEAD` — confirmed via `git diff --stat 3ba7950..HEAD -- src/envelope/{mod,cred,scan,policy,hooks,ledger}.rs`, empty. All five remain verified by direct re-run of their named test suites.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A driven run configured to push to `main` is rejected, with the model's cooperation removed | ✓ VERIFIED | `cargo test --test envelope_tracer` — 6/6 pass (re-run directly). No source change since prior verification. |
| 2 | `--force`, `+refs/…`, `--no-verify`, `core.hooksPath` rewrites are blocked and park the run | ✓ VERIFIED | `cargo test --test envelope_wiring` — 14/14 pass (re-run directly). |
| 3 | A push carrying a detectable secret is blocked, including on a gitignored path | ✓ VERIFIED | `cargo test --test envelope_hook_refusals` — 7/7 pass (re-run directly). |
| 4 | A driven run uses a per-run scoped credential; ambient credentials/SSH agent unreachable | ✓ VERIFIED | `cargo test --test envelope_credential` — 6/6 pass (re-run directly). |
| 5 | Exceeding the per-project 24h PR cap parks the run instead of opening another PR | ✓ VERIFIED | `cargo test --test envelope_pr_cap` — 11/11 pass (re-run directly). |

### Gap-Closure Truths (this round)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 6 | G-19-4: `tests/driver_lock.rs` no longer has a shared-settings-path race, and an early-dying child is reported with its exit status and stderr instead of a misleading 30s timeout | ✓ VERIFIED | Behavior-dependent truth (a state-transition/reporting invariant), confirmed by a passing named test, not presence alone: `a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr` (re-run directly, PASS). Six distinct `ALIAS_*` consts confirmed in source (`grep -c 'const ALIAS_'` = 6, `grep -c '#\[tokio::test'` = 6). `no_two_driving_tests_share_an_envelope_settings_path` (the regression gate) and `two_writers_sharing_one_alias_break_each_others_settings_verification` (the mechanism demonstration) both re-run directly and pass. `git diff --stat fbc5670..04fb2bf -- src/` confirmed empty — no production code touched, matching the plan's prohibition. |
| 7 | G-19-1: `SECTION_ENVELOPE` is legible (one sentence, two explained cases, conclusion) without softening any claim | ⚠️ Mechanically VERIFIED / candour read UNCERTAIN | The mechanical contract is fully verified: 200 whitespace tokens (was 250), all seven pinned substrings present verbatim in the pinned order, `the_honesty_statement_carries_each_of_its_three_required_parts` byte-identical to `3ba7950` (confirmed via `git diff 3ba7950..HEAD -- tests/envelope_advisory.rs` — the only hunk is the new legibility test, an insertion-only diff), all nine residual disclosures independently located in the rendered constant (read directly from `src/envelope/advisory.rs:239-262`). The legibility control (`the_honesty_statement_stays_short_enough_that_a_reader_finishes_it`) re-run directly and passes. **What remains unverifiable by this agent:** whether the text, as actually shipped, reads as candid and finished to a human — this is the exact judgment the original UAT gap was raised from, and the shipped wording differs from the ~161-word draft the user reviewed mid-UAT (the achieved floor was 200 tokens, not ~161, per 19-09-SUMMARY.md's Deviations section). Routed to human verification below. |

**Score:** 6/7 truths mechanically verified; 1 routed to human judgment (0 present-behavior-unverified in the state-transition sense — truth 7's open item is a subjective readability/candour judgment, not an unexercised state transition).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/envelope/advisory.rs` | `SECTION_ENVELOPE` rewritten to the 4-movement shape; doc comment extended with the three editor traps | ✓ VERIFIED | Read directly at lines 156-262; four movements present, `\x20` escapes used correctly (rendered text confirmed indented, not flush-left), ordering-trap and line-break-trap documentation present |
| `tests/envelope_advisory.rs` | New legibility cap test; pin test byte-unchanged | ✓ VERIFIED | 10/10 tests pass (re-run directly); `git diff 3ba7950..HEAD` shows a single insertion-only hunk, confined to the new test |
| `tests/driver_lock.rs` | Six per-test `ALIAS_*` consts; `wait_for_lock_or_report`; `ChildCapture` (files, not pipes); two new controls | ✓ VERIFIED | 8/8 tests pass (re-run directly); all claimed symbols present and confirmed via direct source read |
| `.planning/REQUIREMENTS.md` | SAFE-01/02/03/05/06 marked Complete | ✓ VERIFIED | Confirmed directly — all five `[x]` and "Complete" in the traceability table; no orphaned Phase 19 requirement IDs (SAFE-04 belongs to Phase 16, already Complete; SAFE-07/08 belong to Phase 21, correctly "Gaps Found") |

### Key Link Verification

No key links changed in this round — 19-09 touched only a constant and a test file; 19-10 touched only a test file (`git diff --stat` confirms zero `src/` changes for 19-10, and 19-09's `src/` change is confined to one constant + doc comment in `advisory.rs`, whose wiring into `dry_run.rs`/`run.rs` via `envelope_notice` was already verified in the prior report and re-confirmed unchanged: `the_journal_notice_and_the_rendered_preview_carry_the_same_claim_text` passes in the re-run of `envelope_advisory`).

### Behavioral Spot-Checks / Independent Reproduction

All checks below were re-run directly by this verifier, not taken from SUMMARY.md claims.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Clean build | `rtk proxy cargo build` (n/a — full suite build implies this) | via full-suite run below | ✓ PASS |
| Lint gate | `rtk proxy cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| All-targets lint delta | `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly 4 lints, all in `src/browser.rs` and `src/project_creator.rs` — none in any file this round touched | ✓ PASS |
| `envelope_advisory` + `driver_lock` suites | `cargo test --test envelope_advisory --test driver_lock` | 10 passed / 8 passed, 0 failed in both | ✓ PASS |
| Early-exit reporting invariant (behavior-dependent truth) | `a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr` (within the run above) | ok | ✓ PASS |
| Alias-uniqueness regression gate | `no_two_driving_tests_share_an_envelope_settings_path` (within the run above) | ok | ✓ PASS |
| Full workspace suite (run once) | `rtk proxy cargo test` | 1436 passed, 0 failed (aggregated across all binaries; `driver_reattach` green on this run) | ✓ PASS |
| Debt markers | `grep -n -E "TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER"` over `src/envelope/advisory.rs`, `tests/envelope_advisory.rs`, `tests/driver_lock.rs` | no matches | ✓ PASS |
| Commit existence | `395d0bb`, `4cc54bb`, `45fe2e4`, `bfa3fee`, `0447272`, `04fb2bf` | all present in `git log` | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|---|---|---|---|
| SAFE-01 | 19-01, 19-06, 19-07, 19-09 | ✓ SATISFIED | Unchanged enforcing code; `SECTION_ENVELOPE` rewrite carries the same claims (D1 of 19-09) |
| SAFE-02 | 19-02, 19-03, 19-05, 19-06, 19-07, 19-09 | ✓ SATISFIED | Unchanged enforcing code; pin test byte-unchanged |
| SAFE-03 | 19-03, 19-07 | ✓ SATISFIED | Unchanged; `scan.rs` untouched this round |
| SAFE-05 | 19-04, 19-07, 19-10 | ✓ SATISFIED | Unchanged production credential-scoping code; `driver_lock.rs` test-harness fix does not touch `src/` |
| SAFE-06 | 19-05, 19-07, 19-10 | ✓ SATISFIED (single-layer caveat disclosed, unchanged) | `ledger.rs` untouched; `driver_lock.rs` race fix closes a test-harness defect on the SAFE-06 test path, not a production gap |

No orphaned requirements — same finding as the prior verification, unaffected by this round.

### Anti-Patterns Found

None. `grep -n -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` over the three files this round modified returns no matches. `cargo clippy -- -D warnings` exits 0. `cargo clippy --all-targets` shows only the pre-existing 4 lints in `browser.rs`/`project_creator.rs`, neither of which this round touched.

### Non-Blocking Observations (recorded, not gaps)

Per the scope note governing this round, the following are explicitly out of scope for this gap-closure pass and were confirmed still present, unresolved, and non-blocking:

- `hooks.rs:1144-1145` — the broken-pipe park skip (recorded in 19-UAT.md's test 2 `open_items_recorded_not_blocking`).
- The missing shared-sink "every park reason lands a park" control (same source).
- `scan.rs` documentation placement — the gitleaks-coverage fact lives only in SUMMARY files, not in `scan.rs` itself (19-UAT.md test 3).

**One new observation from this round, investigated further here.** `tests/driver_reattach.rs` shares the structural precondition 19-10's fix removed from `driver_lock.rs` — a single `const ALIAS: &str = "detached"` (line 128) under one process-wide `isolate_envelope_root()` (line 181). 19-10-SUMMARY.md flagged this as a candidate follow-up but left open whether the file actually spawns the real binary. This verifier confirmed it does (`env!("CARGO_BIN_EXE_gsd-meta-manager")` at three call sites) — **but** found no in-process `drive()` calls in the file (`grep` for `drive(` inline calls returned nothing). This means `driver_reattach.rs`'s three real-binary spawns would all share the same `current_exe()` value, so the specific G-19-4 mechanism (a settings-JSON mismatch between an in-process test binary and a real-binary child) does **not** directly transfer — any race here would have to be a different one (e.g., two real-binary writers racing the same atomic rename with otherwise-identical settings content, or the previously-documented artifact-vs-liveness-probe race already recorded in `deferred-items.md`). Recommend this nuance be carried into any follow-up investigation rather than assuming the G-19-4 fix pattern applies unchanged. Confirmed pre-existing and independent of this round's changes (`driver_reattach` ran clean, 0 failures, in this verifier's own full-suite run).

### Code Review Cross-Check

`19-REVIEW.md` (standard depth, 3 files reviewed: `advisory.rs`, `driver_lock.rs`, `envelope_advisory.rs`) found 0 critical, 3 warning, 1 info. All three warnings concern the pre-existing `probe_protection`/branch-protection-probe logic (a lowercase-HTTP-verb gap in a source-scanning test, a 404-vs-403 ambiguity in the read-only GitHub probe, and a cumulative-latency doc-accuracy gap) — none of which are in the code this round's two plans changed (`SECTION_ENVELOPE`'s text and `driver_lock.rs`'s test harness). None are BLOCKER-tier and none contradict a ROADMAP success criterion; they are legitimate advisory-probe robustness improvements for a future plan, not gaps in this phase's goal.

### Human Verification Required

See frontmatter `human_verification`. One item:

1. **Final read of the shipped `SECTION_ENVELOPE` text** — the mechanical contract (shape, pins, disclosures, length control) is fully verified by this agent, but whether the delivered 200-token text actually reads as candid and finished — the specific complaint that opened G-19-1 — is a judgment this phase's own plans reserve for a human reader, and the exact shipped wording has not yet been shown to the user (only an earlier, shorter draft was). The full rendered text, for reference:

```
== What this envelope guarantees, and what it does not ==

Mechanism bounds this run's reach and pushes; a determined agent defeats
everything below the remote.

Guaranteed:
    This run cannot reach your ambient git credentials or SSH agent: the
    socket is removed, not emptied, and global and system git config is a
    generated file naming no credential helper.
    A push through the driven process tree passes the pre-push hook,
    judging the refs git hands it, not the command line asked about.
    Pull-request cap: an append-only ledger this repository does not contain.

Not guaranteed: client-side hooks, tool denies and env-injected git config
are all defeatable by an agent that can spawn an unsupervised shell.
    An agent that unsets GIT_CONFIG_COUNT in a subshell is past the last layer.
    An agent that runs the askpass responder itself reads the token.
    A settings file the agent's own CLI silently ignores leaves that cap
    unenforced: no git hook observes a pull request.
    Only the remote's own ruleset and the scope of the credential this
    run was given do not depend on the agent's cooperation.

Therefore: enable server-side branch protection on this repository. It is
the one control here an agent cannot talk its way past.
```

### Gaps Summary

Both gaps from the 2026-08-28 UAT are closed with real, independently re-run evidence: G-19-1's mechanical contract (shape, seven verbatim pins, byte-unchanged pin test, all nine residual disclosures, a legibility control observed RED then GREEN) and G-19-4's engineering fix (per-test aliases, a source-scanning regression gate observed RED then GREEN, a behavior-dependent early-exit-reporting test that passes, zero `src/` changes). No new blocking gap was found. Status is `human_needed`, not `passed`, solely because G-19-1's underlying complaint was about how the text *reads* to a person, and that judgment — on the text as actually shipped, which differs from the draft the user saw mid-UAT — has not yet been closed by the user. A secondary, non-blocking observation (`driver_reattach.rs`'s shared-alias structure) is recorded for a future investigation but does not block this phase.

---

_Verified: 2026-08-29T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
