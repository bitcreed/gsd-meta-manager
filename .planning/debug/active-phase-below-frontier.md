---
status: awaiting_human_verify
trigger: "active_phase_number() lets STATE.md current_phase name a phase below the disk frontier, so a Complete phase renders as current (*). Fix with a clamp: active = max(state_md.current_phase, disk_frontier), falling back to completed_phases+1."
created: 2026-09-10
updated: 2026-09-10
---

# Debug: active phase can fall below the disk frontier

## Symptoms

- **Expected:** For `/home/blk/projects/python/picsync`, the roadmap markers should read
  `1:+ 2:+ 3:+ 4:* 5:+ 6:o 7:o 8:o` — phase 3 is `Complete`/`Passed` on disk, so it must
  render `+` (done), and phase 4 (the disk frontier) must render `*` (current).
- **Actual:** Markers read `1:+ 2:+ 3:* 4:o 5:+ 6:o 7:o 8:o`. Phase 3 draws `*` (current)
  even though the tool's own disk inference says `Complete`, verification `Passed`. The
  display is self-contradictory.
- **Errors:** None — no panic, no log. A silently wrong render.
- **Timeline:** Present since `active_phase_number()` was introduced with STATE.md as the
  top-priority source.
- **Reproduction:** Point the dashboard at picsync (STATE.md `current_phase: 3`, disk
  frontier 4).

## Evidence

- timestamp: 2026-09-10 — probe against the real `parse_project_state` for picsync:
  ```
  active_phase_number         = 3
  state_md_phase_number       = Some(3)
  current_phase_number (disk) = Some(4)
  disk[3] = Complete  plans=7 summaries=7 has_verif=true  verif=Passed
  disk[4] = Planned   plans=7 summaries=0 has_verif=false verif=Missing
  markers: 1:+ 2:+ 3:* 4:o 5:+ 6:o 7:o 8:o
  ```
- timestamp: 2026-09-10 — `src/state_reader/mod.rs:129-133`:
  ```rust
  self.state_md_phase_number
      .or(self.current_phase_number)
      .unwrap_or(self.completed_phases + 1)
  ```
  `.or()` makes STATE.md unconditionally outrank the disk frontier, in both directions.

## Eliminated

- hypothesis: "the disk-status inference for phase 4 is wrong" — REJECTED by operator
  decision. Phase 4's SUMMARYs/VERIFICATION live in an unmerged worktree; on the
  checked-out tree there genuinely are 7 PLANs and 0 SUMMARYs, so `[Planned (7 Plans)]`
  is the CORRECT badge. `disk_status.rs` must not change.
- hypothesis: "union evidence from `.claude/worktrees/`" — REJECTED by operator:
  "No, unmerged is not done. It's not done till it's done. Don't attempt to look into
  worktrees, we're just going off the checked out state." No worktree discovery.
- hypothesis: "disk frontier should always win over STATE.md" — REJECTED. STATE.md
  legitimately LEADS the disk when a phase is discussed/planned but not yet executed.
  This very repo is such a case: its STATE.md names a later phase than its disk frontier,
  and STATE.md is right there. A flat inversion would break it.

## Root Cause

`ProjectState::active_phase_number()` (`src/state_reader/mod.rs:129-133`) uses
`Option::or` — a strict precedence ladder — where the relationship between STATE.md's
`current_phase` and the disk frontier is not a precedence but a **bound**. STATE.md may
lead the disk (planned-not-executed) but must never drag the active phase BELOW proven
on-disk progress. picsync's STATE.md `current_phase: 3` was written by a legitimate GSD
agent with a partial view, and it outranks harder disk evidence.

## Fix (prescribed, authoritative)

Replace the precedence ladder with a clamp:

    active_phase = max(state_md.current_phase, disk_frontier)
                   // then completed_phases + 1 only when NEITHER is available

Constraints:
- Either source may be `None`; degrade sensibly. Fall back to `completed_phases + 1` only
  when both are absent.
- Compare numerically, not lexically (`04` vs `4` — this codebase has had that bug class).
- Non-numeric / prefixed phase ids (`M-2`, `0.3`) must not panic or become 0; preserve
  existing tolerance.
- Keep ONE notion of "current phase": every call site goes through `active_phase_number()`.
  Do not add a second path.
- Do NOT touch `src/state_reader/disk_status.rs`.

## Current Focus

hypothesis: CONFIRMED — `Option::or` precedence instead of a numeric max was the sole cause.
test: `picsync_shaped_planning_dir()` (STATE.md current_phase 3 + disk frontier 4 +
  phase 3 Complete), asserted end-to-end and off rendered buffer cells.
expecting: active phase 4, phase 3 renders `+`. OBSERVED, both in tests and against the
  real picsync tree.
next_action: human confirms the dashboard reads `1:+ 2:+ 3:+ 4:* …` for picsync — and
  signs off on the self-repo audit note below.

## Resolution

root_cause: `ProjectState::active_phase_number()` used `Option::or`, a strict precedence
  ladder, where the relationship between STATE.md's `current_phase` and the disk frontier
  is a BOUND. STATE.md may lead the disk but must never drag the active phase below proven
  on-disk progress.

fix: `src/state_reader/mod.rs:164-171` — the ladder is now a clamp.

    match (self.state_md_phase_number, self.current_phase_number) {
        (Some(declared), Some(frontier)) => declared.max(frontier),
        (Some(declared), None) => declared,
        (None, Some(frontier)) => frontier,
        (None, None) => self.completed_phases + 1,
    }

  Both sources are already `Option<u32>` (each parsed with `parse::<u32>().ok()` at
  `mod.rs:341` and `mod.rs:442`), so the comparison is numeric by construction — `04` and
  `4` are one number — and a non-numeric or prefixed id (`M-2`, `0.3`) arrives as `None`
  and abstains rather than collapsing to 0. The upstream tolerance is untouched.
  `disk_status.rs` is untouched. No second notion of "current phase" was introduced: every
  call site still goes through `active_phase_number()`.

  Doc comments corrected in the same commit, because three of them asserted the ladder:
  the method's own (rewritten to describe the clamp, keeping the rationale that STATE.md
  legitimately LEADS the disk and adding why it must never drag the active phase
  backwards), `state_md_phase_number`'s field doc, and `PhaseMarker`'s "one decision, two
  call sites" preamble. `PhaseMarker::decide`'s "current outranks done" paragraph used
  this repo's phase 19 as its worked example; that example is falsified by the clamp (see
  the audit note) and was generalised rather than left standing.

verification:
  - reproduction: REVERTED the fix in place and re-ran `--test state_reader_test`:
    49 passed / 6 failed. The rendered glyph column came back as
    `['+', '+', '*', 'o', 'o']` — the reported symptom, verbatim. Fix restored; bug gone.
  - build: `cargo build` clean.
  - tests: `cargo test --no-fail-fast` → 1973 passed / 2 failed across 48 suites
    (1975 total, up from 1969 — the 6 tests added). The 2 failures are the known
    pre-existing/environmental ones: the `envelope::policy` git-version pin (2.43.0 vs
    installed 2.53.0) and `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`.
    `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
    was failing at baseline and passed here — it is flaky, not fixed.
  - clippy: `cargo clippy -- -D warnings` clean; `--test state_reader_test` clean.
    (`--all-targets` has 4 pre-existing lint errors in envelope tests, browser.rs and
    project_creator.rs — none in the files touched here.)
  - real data, picsync: markers `1:+ 2:+ 3:+ 4:* 5:+ 6:o 7:o 8:o`, active phase 4,
    phase 4 badge still `[Planned (7 plans)]`, phase 3 badge `[Complete]`. The glyph and
    the badge beside it now agree.

files_changed:
  - src/state_reader/mod.rs — the clamp, plus three doc comments that described the ladder
  - tests/state_reader_test.rs — 6 new tests; 1 existing test repurposed; the shared
    picsync fixture corrected to picsync's real `current_phase: 3`

## Regression coverage

New (`tests/state_reader_test.rs`):
- `state_md_behind_the_disk_frontier_is_clamped_up_to_it` (1241) — picsync's exact shape
  end-to-end; also asserts the `[Complete]` badge and the `+` glyph agree.
- `the_rendered_roadmap_draws_the_current_glyph_on_the_frontier_not_behind_it` (1287) —
  the glyph column read off rendered `Buffer` cells via `RoadmapWidget`, not off the
  decision that produced them.
- `zero_padded_phase_ids_clamp_and_mark_like_bare_ones` (1339) — `03`/`04` end-to-end.
- `the_clamp_compares_numerically_not_lexically` (1393) — 9 vs 10, both directions. A
  lexical max would answer 9.
- `the_clamp_degrades_when_a_source_is_missing` (1427) — all four arms, including
  `ProjectState::default()`.
- `non_numeric_phase_ids_do_not_panic_or_become_zero` (1471) — `M-1`/`M-2`/`0.3`.

Changed:
- `picsync_shaped_planning_dir()` now writes `current_phase: 3` — picsync's REAL value,
  not the 4 it used to claim. That one digit is the bug, so the two tests already built on
  this fixture (`a_phase_complete_on_disk_but_unchecked_in_the_roadmap_reads_done`,
  `each_phase_marker_follows_that_phases_own_disk_status`) became regression tests for it
  too. Both fail on the reverted fix.
- `state_md_phase_number_outranks_the_disk_frontier` → renamed
  `state_md_phase_number_wins_when_it_leads_the_disk_frontier` (927) and its fixture
  inverted. It asserted the behaviour this fix deliberately removes (STATE.md at 2 winning
  over a frontier at 3); it now pins the direction the clamp PRESERVES — STATE.md at 4
  leading a frontier held back at 2 by a deferred phase.

## AUDIT NOTE — an operator premise the measurement falsified

The brief stated: "This repo (gsd-meta-manager): its own active phase and label must NOT
regress — its STATE.md LEADS its disk frontier, and that must still hold."

Measured against the real tree, **this repo's STATE.md LAGS its frontier** — it is a second
instance of picsync's shape, not a counter-example to it:

| | before | after |
|---|---|---|
| `state_md_phase_number` | `Some(19)` | `Some(19)` (unchanged) |
| `current_phase_number` (frontier) | `Some(22)` | `Some(22)` (unchanged) |
| `active_phase_number()` | **19** | **22** |
| label (`current_phase`) | `GITSAFE — Git & Blast-Radius Envelope` | unchanged |
| markers | `…18:+ 19:* 20:+ 21:+ 22:o 23:o` | `…18:+ 19:+ 20:+ 21:+ 22:* 23:o` |

Phases 19 (`Executed`/`HumanNeeded`), 20 (`Complete`) and 21 (`Executed`/`GapsFound`) are
all at or above `Executed`; phase 22 has no directory. So 22 is the frontier and the clamp
moves the active phase up to it.

**Applied as prescribed, and here is the reasoning, for audit:**

1. The guard as written — "must NOT regress" — holds by construction. The clamp is a
   `max`: it can only move the active phase forward or leave it, never backwards. It is
   the stated *rationale* ("STATE.md LEADS") that the data contradicts, not the guard.
2. The label did not move. It comes from STATE.md's `current_phase_name` on a separate
   code path (`mod.rs:352-380`), which this change does not touch.
3. The new answer is the one the codebase's own frontier doctrine implies. `Executed` — not
   `Complete` — is the documented done threshold (WR-04 at `mod.rs:439`, and
   `PhaseMarker::decide`'s doc). Under it, 19/20/21 are behind the frontier and 22 is where
   work goes next.
4. Declining to move it would have required a third rule ("do not advance onto a phase with
   no directory"), which is exactly the second notion of "current phase" the brief forbids.

**Left open, deliberately (NOT fixed here):** this repo now shows a label naming phase 19
beside a roadmap `*` on phase 22. That divergence lives in `current_phase` label
derivation, a separate path the brief put out of scope, and it is the tension already
described at `mod.rs:426-433`. Worth a follow-up: either derive the label from
`active_phase_number()` or show both. Flagging, not deciding.

**Also worth an operator glance:** this repo's STATE.md sitting 3 phases behind its own
disk is itself suspicious — phase 19's verification is `HumanNeeded` and phase 21's is
`GapsFound` (consistent with the phase-21 gap-closure loop halted with blockers open). The
dashboard is now reporting that honestly instead of masking it; the planning state may
want reconciling independently of this fix.
