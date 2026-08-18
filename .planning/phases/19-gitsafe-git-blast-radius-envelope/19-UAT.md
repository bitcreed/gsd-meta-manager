---
status: testing
phase: 19-gitsafe-git-blast-radius-envelope
source: [19-VERIFICATION.md]
started: 2026-08-18T23:27:41Z
updated: 2026-08-18T23:27:41Z
---

## Current Test

number: 1
name: The pinned honesty statement reads as candour, not as a hedge
expected: |
  `SECTION_ENVELOPE` (src/envelope/advisory.rs:192-214, rendered by the dry-run preview and
  the run journal) should plainly state what IS mechanically guaranteed (no ambient
  credentials or SSH agent reachable, the pre-push hook sees ground truth, the PR cap is
  enforced from an out-of-repo ledger), what is NOT (client-side hooks, tool denies and
  env-injected config are all defeatable by an agent that spawns an unsupervised shell and
  chooses to; the PR cap specifically has no git-hook second carrier and degrades to
  unenforced if the settings file is ignored), and conclude with the server-side
  branch-protection recommendation as the phase's honest conclusion rather than a footnote.
awaiting: user response

## Tests

### 1. The pinned honesty statement reads as candour, not as a hedge
expected: Reads as an honest account of the safety ceiling rather than as a hedge or a
  pre-excuse. States the guaranteed half, the not-guaranteed half, and ends on the
  server-side branch-protection recommendation as the conclusion.
why_human: 19-06 coverage D5 and 19-08 coverage D2 both record this `human_judgment: true`.
  A test can assert the three parts are present and ordered; whether the prose reads as
  candid rather than as marketing is a reading judgement. This is the exact axis PITFALLS
  names as most dangerous to get wrong — an overstated safety claim is worse than a stated
  limitation, because it gets trusted.
result: [pending]

### 2. The composed proofs are faithful decompositions of the criteria as written
expected: The `[C]`-marked rows in 19-08-SUMMARY.md's traceability table (criteria 1, 2, 4
  and 5) each read as a faithful decomposition of the criterion sentence, not as a
  substitution of a weaker claim. Two clauses matter most — criterion 2's "and the attempt
  parks the run" is proved once per park reason rather than once per force-push spelling,
  and criterion 5's "instead of opening another PR" is proved at the PreToolUse guard's deny
  and never by observing a forge (per D-35's fence).
why_human: 19-08 coverage D6 records this `human_judgment: true` explicitly — "whether a
  composed proof is an adequate proof of the sentence as written is a reader's judgement".
result: [pending]

### 3. The residual-exposure disclosures read as admissions, not rationalisations
expected: The paragraphs in `src/envelope/mod.rs`, `src/envelope/cred.rs` and
  `src/envelope/scan.rs` — covering the unset-`GIT_CONFIG_COUNT` escape, the admission that
  an agent inside the run can execute the askpass responder and read the token, and the
  gitleaks Blocked/Failed arms being unexercised on this machine — each state the residual
  gap plainly, in the same paragraph as the mechanism they qualify, without softening
  language.
why_human: Recorded `human_judgment: true` across 19-01 D9, 19-02 D8, 19-03's pre-commit
  rationale, 19-04 D11, 19-05 D11 and 19-07 D25 — a systemic, deliberate pattern across the
  phase rather than an isolated item.
result: [pending]

### 4. Decide how to treat the `driver_lock` one-off
expected: A decision on whether the observed one-off
  `tests/driver_lock.rs::the_lock_is_released_when_the_holding_process_dies` failure ("the
  child driver never took the lock within 30s") is acceptable background flakiness or needs
  a fix before Phase 20 builds on this envelope.
why_human: Needs a judgement call on risk tolerance, not more evidence.
orchestrator_evidence: Did NOT reproduce in three further full-suite runs under load — two
  fully green at 993 passing, the third showing only the two known pre-existing
  `driver_reattach` flakes. Recorded in `deferred-items.md` with the shared mechanism
  hypothesis: 19-07's executor and the verifier independently proposed that
  `establish_envelope()`'s synchronous I/O now sits on the run-startup path *before*
  `lock::acquire`. This is the one item in the phase where Phase 19's own changes are the
  plausible cause — unlike the `driver_reattach` pair, which is proved pre-existing (4/4 red
  at `0a84023`).
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps

None from automated verification — 5/5 must-haves verified against real enforcing code and
independently re-run tests, no stubs, no debt markers, no unwired key links. Every item above
is a judgement the phase deliberately reserved for a human reader.
