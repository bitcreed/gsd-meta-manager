---
phase: 35-decisions-code-needed-and-gates
fixed_at: 2026-11-14T00:00:00Z
review_path: .planning/phases/35-decisions-code-needed-and-gates/35-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 7
skipped: 1
status: partial
---

# Phase 35: Code Review Fix Report

**Fixed at:** 2026-11-14
**Source review:** .planning/phases/35-decisions-code-needed-and-gates/35-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (fix_scope `all`: WR-01..03, IN-01..05)
- Fixed: 7
- Skipped: 1

**Verification:** run in the `feat/v2.5.0-owner-inbox` worktree checkout
(`/home/blk/projects/rust/gsd-meta-manager-owner-inbox`), which has a full build environment and
the gsd-core oracle. No separate review-fix worktree was created: the orchestrator directed the
work into this existing worktree. Baseline before the fixes: 65 test binaries, 0 failures. After
the fixes: `cargo test --no-fail-fast` gives 65 binaries, 0 failures (4168 passed, including the
oracle-backed `owner_open_conformance`, `owner_audit_conformance` and router conformance tests).
`cargo clippy --all-targets -- -D warnings` is clean. Golden fixtures under
`tests/fixtures/owner_golden/` are byte-identical.

## Fixed Issues

### WR-01: `is_answered` hides open questions that merely contain or start with RESOLVED, ANSWERED or CLOSED

**Status:** fixed: requires human verification (matcher heuristic)
**Files modified:** `src/state_reader/owner_items/rules.rs`
**Commit:** b0b8fa3c
**Applied fix:** A marker word now ends only at the end of the text, at whitespace, or at one of
`:.,;—–)]*~` and a backtick. A hyphen does not end it, so `Closed-loop` and
`Resolved-by-default` stay open. A bracketed tag now counts only when it opens the question text
or when no `?` follows its closing bracket. That keeps `(closed vs resolved)?` and
`[answered] polls ... ?` open, while `Which scheme? (CLOSED)` and `- (Resolved 2026-01-02) note`
still count as answered. `QUESTION_LABEL` now needs a real separator (`—`, `–`, `:`, or a hyphen
with whitespace on both sides), so `v2 closed beta` and `v2-closed beta` keep `v2` as the first
word. The reviewer's suggested bracket regex would still have matched `(closed vs resolved)`, so
the position and `?` rule was used instead. Negative cases (all three review examples plus
variants) and new positive cases were added to `is_answered_reads_explicit_markers_only`. All
existing plan-table cases still pass.

### WR-02: An unreadable CONTEXT file is listed but counted nowhere (summary `decide`, `closes_after.decide`)

**Status:** fixed: requires human verification (counting rule change)
**Files modified:** `src/state_reader/owner_items/rules.rs`, `src/state_reader/owner_items/gate.rs`, `README.md`
**Commit:** 9bcebd09
**Applied fix:** The planning artifacts were checked before the change. 35-02-PLAN only fixes
"Unreadable → Decide group with `counts.unreadable` 1 and no items (fail-safe)", and D-11 counts
open questions. Neither locks a `decide` of 0, and CONTEXT's fail-safe principle ("unreadable,
never clean") supports counting the file. Decide groups other than Handoff and Checkpoint now add
`items.len() + counts.unreadable` to `digest.decide`. The gate's Decide arm adds `counts.unreadable`
too, except for checkpoints, which already count their unreadable record as their one decision.
The pinned assertion now expects `decide == 1`, a new gate test
(`an_unreadable_context_file_is_a_decision_not_a_clean_phase`) covers `closes_after.decide`, and
the four README `decide` rows mention unreadable CONTEXT files.

### WR-03: The `closes after:` line is silently truncated by the 200-character cap, dropping the lane parts that matter most

**Status:** fixed
**Files modified:** `src/inbox.rs`
**Commit:** 6ad3aa23
**Applied fix:** The whole line no longer goes through `terminal_capped`. Each project-derived
piece is now escaped and capped on its own: the alias, the milestone version, and each phase
number (new `phase_list` helper). The generated structure is not capped, so the `fix` and
`not built` tail survives. A new unit test
(`a_long_closes_after_line_keeps_every_lane_and_escapes_each_piece`) builds 12 phases in four lanes
with a 60-character alias and hostile escape bytes in the alias, version and phase. It asserts that
the line is longer than 200 characters, is not cut, keeps the full `fix` part and `not built`, and
carries no raw ESC or BEL.

### IN-01: Stale and misleading doc comment on `OPEN_CACHE`

**Status:** fixed
**Files modified:** `src/state_reader/owner_items/mod.rs`
**Commit:** 1b3e19b3
**Applied fix:** `OPEN_CACHE` is now documented as "Parsed debug sessions by canonical path". The
`CONTEXT_CACHE` comment now reads "a separate map" instead of "a second map".

### IN-02: Generality without instantiation, `OpenKind` as a cache key discriminator

**Status:** fixed: requires human verification (necessity finding, code removed)
**Files modified:** `src/state_reader/owner_items/mod.rs`, `src/state_reader/owner_items/debug.rs`, `src/state_reader/owner_items/context.rs`
**Commit:** 742373f4
**Applied fix:** Removed `OpenKind`. `OPEN_CACHE` and `CONTEXT_CACHE` are now keyed by `()`, since
each holds one kind of document. `ParseCache<K, V>` stays generic for `AuditKind`, whose three kinds
share `PARSE_CACHE`. The plans name `OpenKind` only as an implementation detail, not as an
acceptance criterion.

### IN-03: Docs drift for item ids, `acknowledged_files`, and the Old debt fold

**Status:** fixed
**Files modified:** `README.md`, `src/inbox.rs`
**Commit:** cf81b028
**Applied fix:** The README `items[].id` row now names `#q<k>`, `#a<k>`/`#b<k>` and `#1`. Both
`acknowledged_files` rows now list UAT and VERIFICATION files, debug sessions and CONTEXT files.
The Old debt fold says "items" once archived open questions are part of its total. A total made
only of checks keeps the word "checks", because 35-02 requires the goldens to stay byte-identical
(the golden line `Old debt: 16 checks ...` is unchanged). A unit test pins both wordings.

### IN-04: `hidden.todos_unreadable` is JSON-only, so the plain checklist still shows no sign of an unreadable todo folder

**Status:** fixed
**Files modified:** `src/inbox.rs`
**Commit:** 8215266c
**Applied fix:** The plain checklist now adds the fold `Triage: pending todos unreadable in N projects`
when any project sets `hidden.todos_unreadable`. It appears with or without `--include todos`,
since no titles are listed for such a folder either way. A unit test covers both filter settings
and the clean case.

## Skipped Issues

### IN-05: A phase with owner work but no plan is reported as `not built` and contributes nothing to the gate

**File:** `src/state_reader/owner_items/gate.rs:33-39`
**Reason:** Skipped because the change needs a design decision. D-12 in 35-CONTEXT.md locks the
current behavior: "phases with no plans go to `not_built` and contribute no owner work (R8)".
Counting such phases as open would reverse that decision. The alternative, adding PLAN files to the
`gaps` and `ack` golden fixtures, would churn goldens that 35-02 requires to stay byte-identical,
only to hide a fixture artifact. This should be raised as a follow-up decision, not changed here.
**Original issue:** `closes_after` treats `plan_count == 0` as not built, so a real phase that has
open UAT or VERIFICATION work but no PLAN file shows open work in the listing and "not built, no
work" in the gate.

---

_Fixed: 2026-11-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
