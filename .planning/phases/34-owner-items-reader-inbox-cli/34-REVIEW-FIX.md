---
phase: 34-owner-items-reader-inbox-cli
fixed_at: 2026-11-14T15:00:00Z
review_path: .planning/phases/34-owner-items-reader-inbox-cli/34-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 34: Code Review Fix Report

**Fixed at:** 2026-11-14T15:00:00Z
**Source review:** .planning/phases/34-owner-items-reader-inbox-cli/34-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (CR-01, WR-01..WR-05)
- Fixed: 6
- Skipped: 0
- Info items also fixed (trivial and clearly correct): IN-01, IN-03

Verification ran in the feature worktree `/home/blk/projects/rust/gsd-meta-manager-owner-inbox`
(the isolated checkout the orchestrator provided), not in the main checkout. No nested
worktree was created. Oracle conformance (`tests/owner_audit_conformance.rs`) is unchanged:
16 compared, 2 declared divergences, 0 unexpected, over 18 fixtures.

## Fixed Issues

### CR-01: Quadratic fence rescan per heading in `deferred-items.md`

**Files modified:** `src/state_reader/owner_items/deferred.rs`, `src/state_reader/owner_items/scan.rs`
**Commit:** db29f46f
**Applied fix:** A fence scan is a forward state machine, so a scan begun earlier that is
closed at the restart line continues exactly as a fresh scan from that line would. The
per-heading and per-item rescans now reuse the running scan when it is closed there
(`FenceScan::closed_before`) and scan afresh only otherwise; the de-indented lines are
computed once. The remaining fresh scans draw on a shared work budget (8 line visits per
line plus a floor); an exhausted budget makes `parse_deferred_items` return `None` and the
file is marked `Unreadable::TooLarge`, never clean. Rather than the reviewer's
segment-bounded scan (which changes `unterminated_from` semantics), reuse keeps the output
identical to the old behavior on every honest file. Tests: a 1 MiB file of headings and two
1 MiB fence-opener patterns finish well under a 15 s hang bound, and
`reusing_a_closed_scan_matches_a_fresh_one` proves reuse equals a fresh scan at every start.
Status: fixed (logic change; conformance unchanged, so human review of the budget constant
is optional).

### WR-01: Containment is vacuous above the final path component

**Files modified:** `src/state_reader/owner_items/scan.rs`
**Commit:** 5436c6f9
**Applied fix:** `contained` now bounds the canonical phase directory to the canonical
planning tree (computed once in `scan_planning`) instead of to its own parent. A symlinked
`phases/` or `workstreams/` reads nothing; test
`a_symlinked_phases_or_workstreams_directory_cannot_lead_out_of_the_project` (confirmed to
fail with the old check). Module header updated.

### WR-02: A VERIFICATION with unparseable frontmatter reads as clean

**Files modified:** `src/state_reader/owner_items/frontmatter.rs`, `src/state_reader/owner_items/scan.rs`
**Commit:** 1873189b
**Applied fix:** New `frontmatter::extract_checked` separates "absent, unterminated or not a
mapping" (empty `Ok`) from "closed block whose YAML failed to parse, or carries an anchor or
alias" (`Err`). `parse_document` turns the latter, for a VERIFICATION, into
`Unreadable::Io` so it lands in the Fix lane. `extract` keeps its old contract. A parsed
block without a status still contributes nothing (upstream behavior).
Status: fixed: requires human verification (it makes a previously silent case visible; the
anchor-refusal case is also reported as unreadable, which the reviewer did not list).

### WR-03: `read_text_file` has no size cap

**Files modified:** `src/state_reader/disk_status.rs`
**Commit:** 9868af85
**Applied fix:** `read_text_file` rejects a file whose length exceeds 1 MiB and bounds the
read itself with `take(cap + 1)`; every caller already treats an error as absent/unreadable.
Test covers exactly 1 MiB (read), 1 MiB + 1 (refused) and a 4 GiB sparse file (refused
without reading).

### WR-04: Impossible-state guard `file_unreadable` in `apply_r7`

**Files modified:** `src/state_reader/owner_items/rules.rs`
**Commit:** 02693555
**Applied fix:** Verified the invariant (every current, unacknowledged unreadable or
partially parsed file yields a Fix group, so `has_fix` already excludes the phase), removed
the guard, and recorded the invariant at the `has_fix || has_blocked` check. The existing
"parse gap" row of `r7_closeable_table` covers it.
Status: fixed: requires human verification (code removal).

### WR-05: Flaky wall-clock assertions in the cold-run test

**Files modified:** `tests/inbox_cli.rs`
**Commit:** fb6255c0
**Applied fix:** The default bound is now a 10 s hang detector; `GSD_MM_STRICT_TIMING=1`
restores the one-second target for a quiet machine. The test, its name and its
`summary.projects` / `test > 0` assertions are kept. The pathological-input proof for the
real performance hazard is the CR-01 unit tests.

### IN-01: Stale `allow(dead_code)` attributes

**Files modified:** `src/state_reader/owner_items/markdown.rs`, `src/state_reader/owner_items/frontmatter.rs`
**Commit:** ffc7611e
**Applied fix:** Removed the module-wide `#![allow(dead_code)]` and `fm_list`'s attribute;
`collect_section_stopping_at` (no non-test caller) is `#[cfg(test)]`.

### IN-03: `fix_phases` compared raw phase tokens

**Files modified:** `src/state_reader/owner_items/rules.rs`
**Commit:** 572ef92d
**Applied fix:** The set key uses `phase_key` (the canonical spelling, equal to
`same_phase`'s identity). Test `two_spellings_of_one_phase_are_one_fix_phase`.

## Left Alone (Info, not trivial)

- IN-02 (id truncation collision), IN-04 (cache bound by weight), IN-05 (todo list error
  and per-project clock) change schema or behavior beyond a trivial fix; they remain
  open in REVIEW.md.

---

_Fixed: 2026-11-14T15:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
