---
status: testing
phase: 19-gitsafe-git-blast-radius-envelope
source: [19-VERIFICATION.md]
started: 2026-08-18T23:27:41Z
updated: 2026-08-29
resumed: 2026-08-29
previously_deferred_by: user
previously_deferred_reason: >-
  Explicitly deferred on 2026-08-19 so Phase 20 could start. All four items are reading
  judgements or a risk call; none block Phase 20's implementation. Resume with
  /gsd-verify-work 19.
resume_note: >-
  Reopened 2026-08-28 by /gsd-verify-work 19. The four items below were recorded as
  `skipped` with a reason, which the completion predicate would have scored as
  `status: complete` — clearing the phase without anyone judging them. They are
  unresolved debt, not clearances, so they are restored to `[pending]` and presented.
gap_closure_round:
  ran: 2026-08-29
  command: /gsd-execute-phase 19 --gaps-only
  plans: [19-09, 19-10]
  closed:
    - "G-19-1 — closed by 19-09 (395d0bb, 4cc54bb, 45fe2e4, fbc5670). SECTION_ENVELOPE reshaped to one ceiling sentence, then the guaranteed case with indented examples, then the not-guaranteed case with indented examples, then the branch-protection recommendation as the conclusion. 250 -> 200 whitespace tokens; all seven pinned substrings verbatim; the pin test byte-unchanged; every residual disclosure preserved; a new legibility cap observed RED before the rewrite."
    - "G-19-4 — closed by 19-10 (bfa3fee, 0447272, 04fb2bf, d7a9011). Six per-test aliases replace the shared ALIAS in tests/driver_lock.rs, so no two tests write the same envelope settings path. A source-scanning alias-uniqueness gate was observed RED against the pre-fix file and committed in that state (0447272) before the fix. A child that dies before taking the lock is now reported with its exit status and captured stderr instead of as a 30s lock timeout. Nothing under src/ was modified."
  new_pending_item: 5
---

## Current Test

number: 5
name: The shipped SECTION_ENVELOPE text reads as legible candour
expected: |
  One opening sentence stating the ceiling, then the guaranteed case with visibly
  indented examples, then the not-guaranteed case with visibly indented examples, then
  the server-side branch-protection recommendation as the closing conclusion — the shape
  asked for verbatim in test 1 ("One sentence, then an explanation for both cases and
  examples?"). All nine residual disclosures still present and un-softened.
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
result: issue
reported: "it reads like it's too long.. it's hard to follow. Can we shorten it
  significantly. One sentence, then an explanation for both cases and examples? Doable?"
severity: minor
previously: skipped — deferred by user 2026-08-19, never resolved; restored to pending on 2026-08-28
note: >-
  Not a candour failure — the three structural parts are present and correctly ordered, and
  no claim was found overstated. The defect is legibility: at ~230 words in three dense
  paragraphs the statement is not read, and an honesty statement nobody finishes is not
  doing its job. Requested shape: one sentence, then an explanation of each case
  (guaranteed / not guaranteed) with concrete examples.

### 2. The composed proofs are faithful decompositions of the criteria as written
expected: The `[C]`-marked rows in 19-08-SUMMARY.md's traceability table (criteria 1, 2, 4
  and 5) each read as a faithful decomposition of the criterion sentence, not as a
  substitution of a weaker claim. Two clauses matter most — criterion 2's "and the attempt
  parks the run" is proved once per park reason rather than once per force-push spelling,
  and criterion 5's "instead of opening another PR" is proved at the PreToolUse guard's deny
  and never by observing a forge (per D-35's fence).
why_human: 19-08 coverage D6 records this `human_judgment: true` explicitly — "whether a
  composed proof is an adequate proof of the sentence as written is a reader's judgement".
result: pass
previously: skipped — deferred by user 2026-08-19, never resolved; restored to pending on 2026-08-28
judged: 2026-08-28
note: >-
  Criterion 2 resolved by code inspection, not by judgement. The orchestrator first raised a
  diagonal-coverage objection — that `--no-verify` is push-shaped yet classifies to
  `hook_bypass_blocked`, a cell neither fixture appeared to cover — and a read-only check
  refuted it: the `hook_bypass_blocked` fixture drives
  `git -c core.hooksPath=/tmp/nowhere push origin HEAD:...`, which is itself push-shaped
  (refused by `scan_leading`, policy.rs:321-333). More decisively, the refuse-to-park path
  is a single reason-agnostic funnel — every classifier returns the identical
  `GitVerdict::Refuse` (policy.rs:292), consumed by one arm (hooks.rs:1019), funnelled to
  one `deny` (hooks.rs:900), appended by one writer (mod.rs:351) — with no match on reason,
  tool name, verb or refspec anywhere between. No spelling-specific hole can exist.
  Criterion 5 passed as a user judgement AFTER an objection was raised and restated: the
  sentence "parks the run instead of opening another PR" claims a system property while the
  proof covers a single control, the PreToolUse guard, which has no git-hook second carrier
  and which the phase's own honesty statement admits goes silently unenforced if the agent's
  CLI ignores the settings file. The user accepted the criterion as written. The gap remains
  disclosed and tracked as accepted risk T-19-35.
open_items_recorded_not_blocking:
  - >-
    hooks.rs:1144-1145 — `writeln!` to stdout/stderr uses `?` and returns before
    `park_refusal` at hooks.rs:1156, so a closed pipe skips the park. Uniform across all
    reasons and command shapes, so it creates no spelling-specific hole, but it is a narrow
    case where "the attempt parks the run" does not hold, and no test covers it.
  - >-
    There is no parameterized "every park reason lands a park" control. Four reasons have
    individual fixtures; the shared-sink property that makes criterion 2's composition sound
    is established by reading the code, not by a test. It would rot silently if someone later
    introduced a match on reason in that path.

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
result: pass
previously: skipped — deferred by user 2026-08-19, never resolved; restored to pending on 2026-08-28
judged: 2026-08-28
note: >-
  Passed on the verbatim text of all three sites. mod.rs:51-62 carries the unset-
  GIT_CONFIG_COUNT escape as the closing sentence of layer 3's own bullet, not in a separate
  caveats section; cred.rs:478-496 places the token-readable admission in the paragraph
  immediately after the mechanism it qualifies, names the limit as uncloseable, and points
  at credential scope and server-side branch protection rather than claiming a fix.
  Recorded for a future reader: the disclosure this item names for scan.rs — that the
  gitleaks Blocked and Failed arms are unexercised on this machine — is NOT in scan.rs. It
  lives only in 19-03-SUMMARY.md:179 and 19-08-SUMMARY.md:312. What scan.rs itself carries
  (scan.rs:15-26 and the ExternalScanner enum docs) is a different and candid admission:
  that gitleaks is absent and must never fail open. Someone reading scan.rs to decide
  whether to trust the scanner will not find the coverage fact there. Surfaced before the
  pass and accepted by the user as a documentation-placement observation rather than a
  candour failure.

### 4. Decide how to treat the `driver_lock` one-off
expected: A decision on whether the observed one-off
  `tests/driver_lock.rs::the_lock_is_released_when_the_holding_process_dies` failure ("the
  child driver never took the lock within 30s") is acceptable background flakiness or needs
  a fix before Phase 20 builds on this envelope.
why_human: Needs a judgement call on risk tolerance, not more evidence.
result: issue
reported: "fix"
severity: minor
judged: 2026-08-28
note: >-
  The user chose `fix` over `accept` and `defer`, after being shown that the item's original
  framing no longer holds. This is not product flakiness and not acceptable noise: it is a
  defect in the test harness that produces a misleading symptom, and the misleading symptom
  is the reason to fix it — the current failure mode points a future reader at run startup
  when the cause is a shared settings path.
orchestrator_evidence: Did NOT reproduce in three further full-suite runs under load — two
  fully green at 993 passing, the third showing only the two known pre-existing
  `driver_reattach` flakes. Recorded in `deferred-items.md` with the shared mechanism
  hypothesis: 19-07's executor and the verifier independently proposed that
  `establish_envelope()`'s synchronous I/O now sits on the run-startup path *before*
  `lock::acquire`. This is the one item in the phase where Phase 19's own changes are the
  plausible cause — unlike the `driver_reattach` pair, which is proved pre-existing (4/4 red
  at `0a84023`).
orchestrator_evidence_2026_08_28: >-
  Re-investigated 2026-08-28. The ordering claim is TRUE — `establish_envelope()` is
  invoked at src/driver/run.rs:2493, `lock::acquire` at src/driver/run.rs:2609, so 116
  lines of envelope work do precede the lock. But the SLOWNESS mechanism does not hold:
  100 reproduction attempts produced 0 failures (30x isolated under 48 busy loops on 16
  cores; 30x isolated under 64 busy loops plus a concurrent full `cargo test`, load avg
  30; 40x the whole `driver_lock` binary with all 5 tests concurrent, load avg 55). Worst
  measured wall time 1.09s against a 600x50ms = 30s deadline (driver_lock.rs:416-423),
  i.e. ~28x margin. On this test's path `advisory::probe_protection` short-circuits at
  `remote_url` (advisory.rs:511, no `origin`) so no `gh` subprocess runs, and gitleaks
  (scan.rs:415) is not on the path at all — net cost is ~4 `git config` spawns and ~5
  small file writes.
  A BETTER mechanism was found on the same pre-lock path. `write_settings_in` persists
  then read-back-verifies (hooks.rs:1340-1341), and the compared value embeds
  `std::env::current_exe()` via `guard_command(binary, alias)` (hooks.rs:1281). All five
  `driver_lock` tests share one process-wide envelope root (driver_lock.rs:90-97) and one
  ALIAS = "locked" (driver_lock.rs:56). The four sibling tests drive in-process so their
  `current_exe()` is the TEST binary; this test's holder is the REAL binary. They write
  different settings JSON to the same path. A sibling's atomic rename landing between the
  child's `persist_settings` and `verify_settings` makes the child fail
  `EnvelopeAssertionFailed` and exit BEFORE `lock::acquire` — and because stdout/stderr
  are null'd (driver_lock.rs:410-411), the only visible symptom is exactly "the child
  driver never took the lock within 30s". A microsecond window, hence one-off.
  Shape of a fix (not applied, not recommended — the risk call is the user's): give each
  `driver_lock` test its own alias or envelope root so no two writers share the settings
  path, or make `write_settings_in` verify from the handle/content it wrote rather than
  re-reading the shared path. Both are test-side or narrow; neither touches the pre-lock
  ordering.
previously: skipped — deferred by user 2026-08-19, never resolved; restored to pending on 2026-08-28

### 5. The shipped SECTION_ENVELOPE text reads as legible candour
expected: One opening sentence stating the ceiling, then the guaranteed case with visibly
  indented examples, then the not-guaranteed case with visibly indented examples, then the
  server-side branch-protection recommendation as the closing conclusion. All nine residual
  disclosures still present and un-softened.
why_human: 19-09-SUMMARY.md's coverage table (D1) records this `human_judgment: true`. The
  token count and the seven-phrase pin list prove the parts are present and ordered; they do
  not prove the layout is legible or that the text still reads as candour rather than a
  hedge. This is the same axis PITFALLS names as most dangerous — an overstated or
  under-communicated safety claim is worse than a stated limitation, because it gets trusted.
raised_by: gap closure of G-19-1. Test 1 reported the original text as too long and hard to
  follow; 19-09 rewrote it. The user reviewed a ~161-word draft during the 2026-08-28 UAT
  conversation, but the DELIVERED text is a different 200-token version (the 190-token target
  proved unsatisfiable without dropping required disclosures, so the cap was corrected to
  215). The exact shipped wording has therefore not itself been read by the user.
where: the full rendered text is quoted in 19-VERIFICATION.md so it can be judged without
  opening the source.
result: [pending]

## Summary

total: 5
passed: 2
issues: 2
pending: 1
skipped: 0
blocked: 0

Items 1-4 are the 2026-08-28 reading pass: 2 passed, and 2 (items 1 and 4) reported issues
that became G-19-1 and G-19-4. Both gaps are now mechanically closed by the 2026-08-29
gap-closure round (plans 19-09 and 19-10) — see `gap_closure_round` in the frontmatter.

Item 5 is new and is the only thing still pending. It exists because closing G-19-1 changed
the very text item 1 was judging, and the delivered wording differs from the draft the user
saw. It is a reading judgement, not a mechanical check.

## Gaps

None from automated verification — 5/5 must-haves verified against real enforcing code and
independently re-run tests, no stubs, no debt markers, no unwired key links. Every item above
is a judgement the phase deliberately reserved for a human reader.

The entries below come from the human reading pass resumed on 2026-08-28.

- gap_id: G-19-1
  truth: "SECTION_ENVELOPE reads as candour rather than as a hedge, and is legible enough to
    actually be read"
  status: closed
  closed_by: "19-09-PLAN.md — commits 395d0bb, 4cc54bb, 45fe2e4, fbc5670 (2026-08-29)"
  closed_note: >-
    Mechanically closed: 250 -> 200 whitespace tokens, reshaped to one ceiling sentence then
    both explained cases with indented examples then the branch-protection conclusion, all
    seven pinned substrings verbatim, the pin test byte-unchanged, every residual disclosure
    preserved, and a legibility cap observed RED before the rewrite. The human READING
    judgement on the delivered wording is carried forward as new UAT item 5 — the shipped
    200-token text is not the ~161-word draft the user reviewed, so it has not been read yet.
  reason: "User reported: it reads like it's too long.. it's hard to follow. Can we shorten
    it significantly. One sentence, then an explanation for both cases and examples? Doable?"
  severity: minor
  test: 1
  root_cause: "Legibility, not accuracy. SECTION_ENVELOPE (src/envelope/advisory.rs:192-214)
    is ~230 words across three dense paragraphs that each fuse a claim with its several
    qualifications. Every substantive claim checks out and the three required parts are
    present and correctly ordered — the failure is that the density defeats the purpose: an
    honesty statement that is not finished is not read, and PITFALLS' concern is precisely
    that safety claims get trusted without being understood."
  artifacts:
    - path: "src/envelope/advisory.rs"
      issue: "SECTION_ENVELOPE at lines 192-214 — three ~75-word paragraphs, each carrying a
        claim plus multiple embedded qualifications; no visual separation between the
        guaranteed case, the not-guaranteed case, and their examples."
  missing:
    - "Open with a single sentence stating the ceiling."
    - "Then the guaranteed case, explained, with concrete examples."
    - "Then the not-guaranteed case, explained, with concrete examples."
    - "Keep the server-side branch-protection recommendation as the closing conclusion."
    - "Preserve every substantive residual disclosure currently carried — the unset
      GIT_CONFIG_COUNT escape, the agent running the askpass responder to read the token,
      and the PR cap degrading to unenforced when the settings file is ignored. Shortening
      must not become softening; the specifics are what make it candour rather than a hedge."
    - "Keep green whichever tests assert the three parts are present and ordered
      (19-06 coverage D5, 19-08 coverage D2)."
  debug_session: ""
  control_located: >-
    tests/envelope_advisory.rs:175 `the_honesty_statement_carries_each_of_its_three_required_parts`
    pins seven substrings and asserts byte-offset ordering guaranteed < not_guaranteed <
    therefore. Each phrase must also sit wholly within one rendered line — a phrase
    straddling a newline cannot match. Four further tests reference SECTION_ENVELOPE only by
    identity, not content: tests/envelope_advisory.rs:224, src/driver/dry_run.rs:536,
    src/driver/dry_run.rs:699, src/driver/dry_run.rs:782.
  draft_available: >-
    A 161-word replacement draft (down from 239) already exists and was reviewed with the
    user on 2026-08-28. It keeps all seven pinned substrings verbatim and preserves all seven
    residual disclosures, so it lands with NO test edits. It did not reach the requested
    120-140 words: the pinned phrases alone are ~48 words, and going lower requires rewording
    pins and updating the test's phrase list in the same commit — judged the wrong trade for
    the phase's honesty statement. Two secondary clauses were dropped as redundant rather
    than softened: "so the run cannot reset its own limit by deleting a file it can see"
    (implied by a ledger the repo does not contain) and "it is this envelope's conclusion
    rather than its footnote" (the new layout demonstrates it instead of asserting it).
    IMPLEMENTATION GOTCHA, verified with rustc: Rust's backslash-continuation strips ALL
    leading whitespace on the next source line, so indented bullet and continuation lines
    must be written with \x20 escapes or every indent silently vanishes and the list
    collapses flush-left.

- gap_id: G-19-4
  truth: "`tests/driver_lock.rs::the_lock_is_released_when_the_holding_process_dies` passes
    reliably, and when it does fail the symptom points at the actual cause"
  status: closed
  closed_by: "19-10-PLAN.md — commits bfa3fee, 0447272, 04fb2bf, d7a9011 (2026-08-29)"
  closed_note: >-
    Six per-test `ALIAS_*` consts replace the shared alias, so no two tests write the same
    `<envelope_root>/<alias>/settings.json` and the persist-then-verify window that failed
    the child with EnvelopeAssertionFailed no longer exists. A source-scanning
    alias-uniqueness gate was observed RED against the pre-fix file and COMMITTED in that
    state (0447272) before the fix landed. The child's stdout/stderr now go to files in a
    TempDir (never pipes) and an early-dying child is reported with its exit status and
    stderr rather than as a 30s lock timeout. Nothing under src/ was modified; the refuted
    establish_envelope()/lock::acquire ordering hypothesis was not revived.
  follow_up: >-
    tests/driver_reattach.rs shares the old structural precondition (a single shared
    `const ALIAS: &str = "detached"` at :128 plus its own process-wide isolate_envelope_root
    at :181). Verification found it has no in-process `drive()` calls — only real-binary
    spawns — so the specific current_exe() mismatch does not directly transfer. Recorded as a
    candidate follow-up, deliberately NOT actioned in this round.
  reason: "User chose `fix` over `accept` and `defer` when shown that the item's original
    framing no longer holds."
  severity: minor
  test: 4
  root_cause: "A race in the test harness — not product flakiness, and not slow startup. All
    five `driver_lock` tests share one process-wide envelope root (driver_lock.rs:90-97) and
    one ALIAS = 'locked' (driver_lock.rs:56). `write_settings_in` persists then
    read-back-verifies (hooks.rs:1340-1341), and the compared value embeds
    `std::env::current_exe()` via `guard_command(binary, alias)` (hooks.rs:1281). The four
    sibling tests drive in-process so their `current_exe()` is the TEST binary; this test's
    holder is the REAL binary. They write different settings JSON to the same path. A
    sibling's atomic rename landing between the child's `persist_settings` and
    `verify_settings` makes the child fail `EnvelopeAssertionFailed` and exit BEFORE
    `lock::acquire`. Because stdout/stderr are null'd (driver_lock.rs:410-411), the only
    visible symptom is exactly 'the child driver never took the lock within 30s'.
    The originally recorded hypothesis — that `establish_envelope()`'s synchronous I/O on the
    pre-lock path made the child miss the deadline — is REFUTED as a slowness explanation.
    The ordering is real (run.rs:2493 before run.rs:2609) but 100 reproduction attempts under
    deliberate load (to load average 55) produced 0 failures, worst wall time 1.09s against a
    600x50ms = 30s deadline (driver_lock.rs:416-423) — roughly 28x margin."
  artifacts:
    - path: "tests/driver_lock.rs"
      issue: "Lines 56 and 90-97 — all five tests share one ALIAS and one process-wide
        envelope root, so four in-process siblings and one real-binary child write different
        settings JSON to the same path."
    - path: "src/envelope/hooks.rs"
      issue: "Lines 1281 and 1340-1341 — `write_settings_in` verifies by re-reading the
        shared path, and the compared value embeds `current_exe()`, which differs between the
        test binary and the real binary."
    - path: "tests/driver_lock.rs"
      issue: "Lines 410-411 — the child's stdout and stderr are null'd, so an
        `EnvelopeAssertionFailed` exit is invisible and surfaces only as the 30s timeout.
        This is what makes the failure misleading rather than merely rare."
  missing:
    - "Give each `driver_lock` test its own alias (or its own envelope root) so no two
      writers share the settings path. Preferred: fixes the race at its source, confined to
      the test file."
    - "Alternatively, make `write_settings_in` verify from the handle or content it wrote
      rather than re-reading the shared path. Narrower on the tests but touches production
      code."
    - "Surface the child's failure instead of swallowing it — capture stdout/stderr, or
      assert on the child's exit status, so a future `EnvelopeAssertionFailed` does not
      masquerade as a lock timeout. This has standalone value even if the race never recurs."
    - "Do NOT change the pre-lock ordering of `establish_envelope()` and `lock::acquire`. The
      measured margin is ~28x; the ordering is not the problem."
  debug_session: ""
