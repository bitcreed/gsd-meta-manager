---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 08
subsystem: security
tags: [sha256, fnv1a, digest, approval, prompt-injection, ansi-escape, c1-controls, tdd]

# Dependency graph
requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-01)
    provides: "goal::legality, untrusted::bounded, and the third-party string census that target_phase escaped"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-03)
    provides: "journal::sha256_digest and the sha256:/fnv1a64: prefix discipline that makes this upgrade migration-free"
  - phase: 18-driver-tui
    provides: "ui::screens::sanitize_render_line, the shared ESC/C0/C1 render gate (T-18-38)"
provides:
  - "goal::plan_digest returns a sha256:-prefixed digest produced by journal::sha256_digest, never by argv_digest"
  - "Three corrected doc paragraphs that state why the FNV-then-SHA-256 composition was NOT safe"
  - "A test proving a legacy fnv1a64: ApprovedPlan re-checks as PlanChanged (fails closed, no migration)"
  - "goal::legality refuses a target_phase that untrusted::bounded would alter, by name rather than truncating it"
  - "PlanStep::target_phase bounded at construction, byte-equal to the roadmap-declared token"
  - "DriveError::PlanApprovalRequired renders its steps through sanitize_render_line"
affects: [21-verification, 21-secure-phase, driver-approval-flow, goal-layer]

actuals:
  tokens: 76551
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A digest that is an input to a security control is computed with the collision-resistant hasher, never composed from a weak one under a strong outer hash"
    - "Refuse-rather-than-repair for any third-party token that is also a map key: bounding that would alter the value is a refusal, not a truncation"
    - "Two layers over the same bytes (bound at construction + sanitize at render), each documented as not-the-only-one"

key-files:
  created: []
  modified:
    - src/driver/goal.rs
    - src/journal/mod.rs
    - src/error.rs
    - tests/driver_goal_seam.rs

key-decisions:
  - "plan_digest moves to sha256_digest rather than approval_digest absorbing the weakness: hashing a weak digest under a strong one preserves the weak collision class exactly"
  - "No GoalReason arm added for control characters — PhaseNotPlainComponent is reused, keeping as_str/ALL/the both-directions guard untouched"
  - "target_phase is REFUSED when bounding would alter it, not stored bounded, because the value is the router's map key"
  - "error.rs takes an in-crate dependency on ui::screens::sanitize_render_line rather than restating the rule, so the two cannot disagree about C1"
  - "No migration for legacy fnv1a64: records — the goal layer is unreleased and the prefix makes the upgrade fail closed"

patterns-established:
  - "Premise assertions in hostile-input tests: each fixture first asserts the OLD checker accepts it, so the test cannot pass via an arm it never reaches"
  - "Digest verified against an independent SHA-256 computation, not only against itself"

requirements-completed: [DRIVE-03, SAFE-07]

coverage:
  - id: D1
    description: "The plan half of an approval is collision-resistant: goal::plan_digest returns a sha256:-prefixed digest from journal::sha256_digest, so a hostile ROADMAP.md phase name cannot be used to construct a second plan sharing an approval digest"
    requirement: DRIVE-03
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#the_plan_half_of_an_approval_is_collision_resistant_rather_than_a_non_cryptographic_hash"
        status: pass
      - kind: other
        ref: "python3 sha256 of the unit-separator-joined token stream matched the runtime value byte for byte"
        status: pass
    human_judgment: false
  - id: D2
    description: "A recorded approval carrying a legacy fnv1a64: plan_digest re-checks as ApprovalRefusal::PlanChanged rather than comparing equal — the format change fails closed and needs no migration"
    requirement: DRIVE-03
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_recorded_approval_carrying_a_legacy_fnv1a64_plan_digest_re_checks_as_stale"
        status: pass
    human_judgment: false
  - id: D3
    description: "Three doc paragraphs that asserted the FNV-then-SHA-256 composition was already safe are corrected, each stating why the old reasoning was wrong rather than deleting the claim"
    requirement: DRIVE-03
    verification: []
    human_judgment: true
    rationale: "Whether a corrected security doc is now TRUE and legible to the next reader is a judgment about prose, not a property a test can assert. The before/after text is quoted verbatim below for exactly this read-back."
  - id: D4
    description: "A model-selected target_phase carrying a control character (ESC, newline, carriage return, or the one-codepoint C1 CSI) is refused by name as PhaseNotPlainComponent, and a legal token still reaches PlanStep::target_phase byte-identical to what the roadmap declared"
    requirement: SAFE-07
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_phase_token_carrying_a_control_character_is_refused_by_name_rather_than_stored"
        status: pass
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_hostile_roadmap_phase_token_is_refused_at_the_seam_the_run_actually_reaches"
        status: pass
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_legal_phase_token_reaches_the_step_byte_identical_to_what_the_roadmap_declared"
        status: pass
    human_judgment: false
  - id: D5
    description: "DriveError::PlanApprovalRequired's Display carries neither ESC nor U+009B to the reviewer's terminal, while still naming every step, the numbering, and the --approved-plan flag"
    requirement: SAFE-07
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#the_approval_refusal_cannot_repaint_the_terminal_of_the_person_about_to_approve"
        status: pass
    human_judgment: false

duration: 26min
completed: 2026-08-20
status: complete
---

# Phase 21 Plan 08: Collision-Resistant Plan Digest and a Bounded Phase Token Summary

**The approval a user reviews is now identified by SHA-256 instead of FNV-1a-64, and the model-selected phase token inside it is refused rather than repaired when it carries an escape sequence — closing CR-02 and WR-05, the two defects that made a reviewed plan untrustworthy in its identity and in its rendering.**

## Performance

- **Duration:** ~26 min
- **Started:** 2026-08-21T00:25Z
- **Completed:** 2026-08-21T00:51Z
- **Tasks:** 2/2
- **Files modified:** 4

## Accomplishments

- **CR-02 closed.** `goal::plan_digest` no longer calls `journal::argv_digest`. The token stream is unchanged (same `\u{1f}` separator, same per-step `verb`/`target_phase`/`terminal_state`, same rationale exclusion); only the hasher moved, to `journal::sha256_digest`.
- **The false security claim is corrected in three places, not two.** The code review named `approval_digest`'s doc; a third paragraph at `ApprovedPlan::plan_digest` made the same false assertion and is corrected in the same commit as the code.
- **WR-05 closed with both layers.** `goal::legality` refuses a `target_phase` that `untrusted::bounded` would alter, and `DriveError::PlanApprovalRequired`'s `Display` routes each step through `ui::screens::sanitize_render_line`.
- **Test count on the seam file rose from 10 to 16**, all green, with the whole 35-binary suite at 0 failures.

## Task Commits

Both tasks were TDD (`tdd="true"`), so each has a RED and a GREEN commit:

1. **Task 1: The approval's plan half becomes collision-resistant** — `2268c0e` (test, RED) → `dc06128` (feat, GREEN)
2. **Task 2: The phase token is bounded at construction and sanitized before the terminal** — `65959a3` (test, RED) → `02b1658` (fix, GREEN)

No REFACTOR commit was needed for either: the GREEN implementations are each a hasher swap or a single added guard, with nothing left to clean up.

## Files Created/Modified

- `src/driver/goal.rs` — `plan_digest` hashes through `sha256_digest`; `legality` gains the bounded-inequality refusal; `PlanStep::target_phase` stored bounded and its field doc rewritten
- `src/journal/mod.rs` — `approval_digest`'s and `ApprovedPlan::plan_digest`'s doc paragraphs corrected. **`argv_digest` itself is untouched**, as the plan required: its sibling rationale remains true and this change is its first real consumer of the distinction
- `src/error.rs` — `PlanApprovalRequired`'s `Display` sanitizes its steps; head comment records the deliberate in-crate `ui` dependency
- `tests/driver_goal_seam.rs` — 6 new tests plus three helpers (`step_with_rationale`, `resolved_cap`, `plan_from`)

## The three corrected doc paragraphs, verbatim

The plan's `<output>` requires these quoted before and after, so a reader can check that a false security claim was **corrected** rather than deleted.

### 1. `goal::plan_digest` — `src/driver/goal.rs`

**BEFORE:**

```
/// A stable identity for one plan value.
///
/// Rendered in the `{prefix}:{hex}` shape [`crate::journal::argv_digest`]
/// establishes, and computed **by** that function rather than by a third hasher
/// written here — the tree has one identity digest and gains no second.
///
/// Plan 21-03 binds a recorded approval to this value, so it is declared here,
/// beside the type whose identity it is, rather than at the approval site.
///
/// **It is not a security control, and 21-03 is where that becomes a problem.**
/// `argv_digest` is FNV-1a 64 and its own doc says so in as many words. Against
/// an adversary who wants an approved plan swapped for a different one, FNV-1a
/// detects nothing. Binding an approval to it is honest only as drift detection.
/// The `fnv1a64:` prefix is what makes the upgrade free when 21-03 takes it: a
/// `sha256:`-prefixed sibling reads old values as legacy with no migration.
```

**AFTER:**

```
/// The identity of one plan value, as the **plan half of an approval**.
///
/// Rendered in the `{prefix}:{hex}` shape the journal's digests establish, and
/// computed through [`crate::journal::sha256_digest`].
///
/// # Why this is not [`crate::journal::argv_digest`]
///
/// **This value is an input to [`crate::journal::approval_digest`], so it is
/// part of a security control rather than an identity fingerprint.** That is the
/// whole difference between it and `argv_digest`, which answers "same command
/// line?" for a human reading a `run.json` and has no adversary.
///
/// Composing them would not have worked. Hashing an FNV-1a-64 digest under
/// `approval_digest`'s outer SHA-256 preserves FNV's collision class **exactly**:
/// two colliding inner values produce byte-identical input to the outer hash, so
/// the outer hash cannot tell them apart either. FNV-1a-64 second preimages are
/// *constructed* rather than searched — multiplication by the FNV prime is
/// invertible mod 2^64 — and one of the tokens hashed below is `target_phase`,
/// which is a phase name authored by whoever wrote the cloned repository's
/// `ROADMAP.md` and is therefore attacker-controlled under this phase's own
/// threat model. An attacker who can author a phase name could construct a
/// second plan sharing an approval digest with the one the user reviewed.
```

The "21-03 is where that becomes a problem" sentence is gone: the problem is closed here, and a doc still naming it open is the next reader's confusion.

### 2. `journal::approval_digest` — `src/journal/mod.rs`

**BEFORE (the paragraph the review showed to be mathematically wrong):**

```
/// **SHA-256, and that matters here specifically.** `argv_digest` is FNV-1a and
/// its own doc says it is not a security control; an approval is exactly the
/// affordance an adversary wants to defeat, so the outer digest is the
/// collision-resistant one. The inner `plan_digest` remains FNV-1a drift
/// detection, which is honest as long as it is not the only thing standing
/// between an approval and a substituted plan — and it is not, because it is one
/// of the inputs hashed here.
```

**AFTER:**

```
/// **SHA-256 on BOTH halves, and the "both" is the load-bearing word.** An
/// approval is exactly the affordance an adversary wants to defeat, so the outer
/// digest is the collision-resistant one — but an outer SHA-256 does not rescue a
/// weak inner digest, and an earlier version of this doc claimed it did.
///
/// That claim was wrong, and the reason it was wrong is worth keeping rather than
/// deleting: **hashing a weak digest under a strong one preserves the weak
/// digest's collision class exactly.** Two colliding inner values produce
/// byte-identical input to the outer hash, so the outer hash cannot distinguish
/// what the inner one already conflated. An inner FNV-1a-64 would have been a
/// second-preimage the attacker *constructs* rather than searches — the FNV round
/// is invertible mod 2^64 — over a token stream containing a `target_phase` that
/// a hostile `ROADMAP.md` authors.
///
/// So all three digests here are SHA-256: the plan half via
/// [`crate::driver::goal::plan_digest`], the file half via each
/// [`crate::config::PromptInput`]'s recorded digest from
/// `registry::current_prompt_inputs`, and the composition via this function.
/// [`argv_digest`] is not in this composition at all.
```

### 3. `ApprovedPlan::plan_digest` — `src/journal/mod.rs`

This is the **third** paragraph — the one the code review did not name. It asserted the field was FNV-1a, which is now false in fact as well as in reasoning.

**BEFORE:**

```
    /// `driver::goal::plan_digest` of the approved plan.
    ///
    /// FNV-1a, and therefore drift detection rather than a control on its own.
    /// It is recorded so a failed re-check can say *which half* changed; the
    /// control is [`Self::approval_digest`], which hashes this value together
    /// with the disclosed files under SHA-256.
```

**AFTER:**

```
    /// [`crate::driver::goal::plan_digest`] of the approved plan: a
    /// `sha256:`-prefixed digest of its step tokens in plan order.
    ///
    /// **A control rather than a hint.** It is SHA-256, so an attacker who
    /// authors a project's `ROADMAP.md` — and therefore the `target_phase`
    /// tokens hashed into it — cannot construct a second plan that shares this
    /// value with the one the user reviewed. It is recorded *separately* from
    /// [`Self::approval_digest`] so a failed re-check can name **which half**
    /// moved, the plan or the disclosed files, rather than only that something
    /// did.
    ///
    /// A record written before this became SHA-256 carries the legacy
    /// `fnv1a64:` prefix. Nothing migrates it and nothing needs to:
    /// [`recheck_approval`] compares the whole prefixed string, so the prefixes
    /// differ, the record re-checks as [`ApprovalRefusal::PlanChanged`], and the
    /// upgrade fails closed. That is what the prefixes are for, and
    /// `tests/driver_goal_seam.rs::a_recorded_approval_carrying_a_legacy_
    /// fnv1a64_plan_digest_re_checks_as_stale` proves it rather than leaving it
    /// as a claim in this paragraph.
```

## The digest values, before and after

The plan's `<output>` asks for one example `plan_digest` value showing the `sha256:` prefix. For the one-step plan `command=/gsd-plan-phase phase=21 terminal=verification_passed`:

| Build | `goal::plan_digest` value |
|---|---|
| Before (FNV-1a-64) | `fnv1a64:609568bf120b4c8d` |
| After (SHA-256) | `sha256:b763c07159a6538ed33c76f8e35740a8df2b355856e802a6eef9a2177626742e` |

For the two-step fixture used in the shape test, the old value was `fnv1a64:ddaab04189000ddd`.

**The `sha256:` value was verified against an independent computation**, not only against itself: `python3 -c 'hashlib.sha256(chr(31).join(["/gsd-plan-phase","21","verification_passed"]))'` produces exactly `b763c07159a6538ed33c76f8e35740a8df2b355856e802a6eef9a2177626742e`. That is what makes "it is really SHA-256 over the unit-separator-joined token stream" a fact rather than a claim that the function is stable against itself.

## Decisions Made

1. **`plan_digest` moved to `sha256_digest` rather than `approval_digest` compensating.** There is no compensating for it — that was the review's whole point and the correction the docs now carry.
2. **No new `GoalReason` arm.** A token carrying a control character is not a plain path component in any useful sense, the existing message reads correctly for it, and a new arm would mean touching `as_str`, `ALL` and the both-directions guard for a distinction no reader of the refusal needs. Followed the plan's explicit instruction.
3. **Refuse rather than store-bounded for `target_phase`, and do BOTH.** The refusal keeps the stored value byte-equal to the roadmap's token, which matters because it is the map key into `ProjectState::phase_disk_statuses` and the input to `RouterAction::command_for`. The bounded store makes the bound a construction property, so a future author who loosens the refusal does not thereby unbound the field. Both are commented as non-redundant so neither reads as dead code.
4. **`error.rs` reaches into `crate::ui::screens`** rather than restating the sanitizer. Recorded in the file's head comment as the single deliberate exception to its narrow dependency surface, with the reason: two implementations of "make these bytes safe to paint" are two things that can disagree about what a C1 introducer is.
5. **No migration path for legacy `fnv1a64:` records.** Verified the plan's reversibility premise independently — `src/driver/goal.rs` does not exist at tag `v1.6.0`, so the goal layer is unreleased and no shipped build ever wrote an `fnv1a64:` plan digest. The prefix makes a dev-tree record fail closed, and a test now pins that instead of a doc comment claiming it.

## Deviations from Plan

None — plan executed exactly as written. Every acceptance criterion, including the three-paragraph doc correction the plan flagged as easily under-counted, was met as specified.

## Issues Encountered

**A note on the RED phase, for honesty about the TDD gates.** Of the six new tests, five failed against the pre-change build as intended. One — `a_legal_phase_token_reaches_the_step_byte_identical_to_what_the_roadmap_declared` — passed at RED, and that is by design rather than a skipped gate: it is the guard against fixing WR-05 the wrong way, by bounding `target_phase` without refusing what bounding would alter. Its job is to fail against a *future* naive implementation, not against the current one. The behavior-adding tests in each task did fail first, so both RED gates are genuine.

The RED output also captured the defects concretely, which is worth recording:

- Task 1 RED: `plan_digest` returned `fnv1a64:ddaab04189000ddd` where a `sha256:` prefix was required.
- Task 2 RED: the hostile ESC token was refused as `goal_phase_absent_from_roadmap` rather than `goal_phase_not_plain_component` — it survived the path-component check entirely and was only stopped one arm later by roadmap membership. That refusal would have stopped being reached the moment a hostile `ROADMAP.md` declared the same token it planted, which is precisely the WR-05 scenario.
- Task 2 RED: `PlanApprovalRequired`'s rendered `Display` contained the literal `\u{1b}[2K\u{9b}1;32m` sequence, i.e. an erase-line plus a forged green attribute, in the message shown at the approval moment.

## Verification Performed

All six of the plan's `<verification>` items, using `rtk proxy` throughout because `rtk` filters `warning:` and `test result:` lines and a bare grep would have passed vacuously:

| # | Check | Result |
|---|---|---|
| 1 | `rtk proxy cargo build --all-targets` | clean |
| 2 | `rtk proxy cargo clippy -- -D warnings` (lib target, not `--all-targets`) | clean |
| 3 | `rtk proxy cargo test --test driver_goal_seam` | **16 passed, 0 failed** (was 10 in `21-VERIFICATION.md`) |
| 4 | `rtk proxy cargo test --lib journal::` / `--lib driver::goal` | 90 passed / 20 passed, 0 failed |
| 5 | `rtk proxy cargo test` (whole suite) | 35 binaries, **0 failures**, 0 `FAILED` lines |
| 6 | Manual read-back of the three doc paragraphs | quoted above, before and after |

Grep criteria: `sha256_digest` appears in `src/driver/goal.rs` (3 matches, one inside `plan_digest`'s body); `untrusted::bounded` appears 7 times in `src/driver/goal.rs` (≥3 required: the refusal comparison, `target_phase`, `rationale`); `sanitize_render_line` appears twice in `src/error.rs` (≥1 required).

`rtk proxy cargo doc --no-deps` was also run to confirm the new intra-doc links resolve. The two `unresolved link to \`tests\`` warnings it reports are pre-existing, in `src/driver/reconcile.rs:5` and `:7`, and are out of scope for this plan.

## Threat Mitigations Applied

All five rows of the plan's `<threat_model>` carried `mitigate` and all five are now implemented and tested:

| Threat ID | Category | Status |
|---|---|---|
| T-21-08-01 | Tampering (constructed FNV second preimage via a chosen roadmap phase name) | mitigated — `sha256_digest` |
| T-21-08-02 | Spoofing (terminal repaint at the approval moment) | mitigated — two layers, `bounded` + `sanitize_render_line` |
| T-21-08-03 | Repudiation (three in-tree false security claims) | mitigated — all three corrected with the reasoning kept |
| T-21-08-04 | EoP (a legacy `fnv1a64:` record comparing equal) | mitigated — fails closed, pinned by test |
| T-21-08-05 | DoS (a truncated phase driving a different phase) | mitigated — refused by name, byte-equality pinned by test |

No dependency was added; `sha2` was already present from plan 21-03 and was not re-audited.

## Known Stubs

None. No placeholder, no `TODO`/`FIXME`, no `#[ignore]`d or skipped test was introduced by this plan.

## Next Phase Readiness

- **ROADMAP criterion 1** (recorded ✗ FAILED) had CR-02 as one of its two causes; that cause is now closed. The criterion should be re-evaluated once the other cause is addressed.
- **Recorded gap 2** (`partial`) had WR-05 as half of it; that half is closed.
- **`DRIVE-03` and `SAFE-07`** are advanced by this plan but should not be marked Complete on this plan alone — `21-07-PLAN.md` owns six of the seven surfaced probe edges and this plan owns only item 6 (SAFE-07 / precision). The prior premature-Complete revert (`828d7cc`) is the reason to be conservative here.
- **No blockers.** The whole suite is green and nothing is left half-applied.

## Self-Check: PASSED

- `21-08-SUMMARY.md` exists on disk at the path the plan's `<output>` specifies.
- All four task commits plus the metadata commit are present in `git log` (`2268c0e`, `dc06128`, `65959a3`, `02b1658`, `c3d0998`), on branch `worktree-agent-aaad50d6e721c943c` from base `9ca1521`.
- All four modified files exist and are tracked.
- `git diff --diff-filter=D 9ca1521 HEAD` reports **no deletions** — nothing was removed.
- Working tree is clean; nothing uncommitted.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-20*
