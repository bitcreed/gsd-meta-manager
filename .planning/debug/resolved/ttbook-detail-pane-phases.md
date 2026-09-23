---
status: resolved
trigger: "Detail pane for /home/blk/projects/python/ttbook: P5 marked current while [Complete]; P7.1 and P07.1 duplicated (phase number normalization); empty Milestone field; 0/? plans with Executed/Complete."
created: 2026-09-23
updated: 2026-09-23
---

# Debug: ttbook detail pane — wrong current phase, duplicated decimal phase, empty milestone, 0/? plans

## Symptoms

- **Expected:** one row per phase; `*` on the phase actually in progress (7.1, which STATE.md
  names and whose directory holds 16 PLANs and no SUMMARYs); plan counts that agree with the
  disk badge; a milestone name.
- **Actual (reported render):**
  ```
  Status: executing    Milestone:
  + P1: WAF and session/token acquisition (R1)  0/? plans [Executed]
  * P5: Chat module redesign (R5)  0/? plans [Complete]
  o P7.1: Apply owner rulings and booking-HAR findings (INSERTED)  0/? plans [Not started]
  o P07.1: Apply owner rulings and booking-HAR findings (INSERTED)  0/? plans [Planned (16 plans)]
  ```
- **Errors:** none — a silently wrong render.
- **Reproduction:** register `/home/blk/projects/python/ttbook` (READ-ONLY fixture; never
  modified) and open its Detail pane.

## Evidence

- ttbook STATE.md frontmatter: `current_phase: "7.1"`, no `milestone:` key. state.json
  `milestone: null`.
- ttbook ROADMAP.md: checklist `- [ ] **Phase 7.1: …**`, details heading `### Phase 07.1: …`,
  directory `phases/07.1-apply-owner-rulings-and-booking-har-findings/` (16 PLANs, 0 SUMMARYs).
  Phases 1, 5 and 6 carry `**Plans**: TBD` with no plan checklist; their directories hold 6, 9
  and 8 PLAN/SUMMARY pairs. `## Milestones` lists
  `- 🚧 **Milestone 1: Reassessment and decision records** - Phases 1-7 (in progress)`.
  `## Progress` marks 3, 5, 6, 7 `Complete` → `completed_phases = 4`.
- Probe of the real `parse_project_state` on ttbook, before the fix:
  ```
  milestone="" state_md=None frontier=None active=5 completed=4
  5    Current 0/0 disk=Complete 9/9
  7.1  Future  0/0 disk=NoDirectory
  07.1 Future 0/0 disk=Planned 0/16
  ```
  After the fix:
  ```
  milestone="Milestone 1: Reassessment and decision records" state_md=7.1 frontier=7.1 active=7.1
  5    Done
  7.1  Current 0/16 disk=Planned
  ```

## Root Cause

1. **P5 current** — `src/state_reader/mod.rs` parsed both clamp sources with
   `parse::<u32>()` (STATE.md `current_phase` at the frontmatter read, the frontier in the disk
   loop), and `PhaseMarker::decide` compared with `parse::<u32>()` too. `"7.1"` is not a `u32`,
   so both sources abstained and `active_phase_number()` fell through to
   `completed_phases + 1` = 4 + 1 = 5 — a count from the Progress table, not a phase.
2. **P7.1 / P07.1 twice** — `roadmap_md::merge_duplicate_phases` keyed on the raw `number`
   string, so `7.1` (checklist) and `07.1` (details heading) stayed separate rows; and
   `disk_status::phase_dir_matches` only generated pad variants for all-digit ids, so `7.1`
   never found `07.1-…` (→ `[Not started]`) while `07.1` did.
3. **0/? plans** — two gaps. (a) `parse_roadmap_phases`'s plan regex
   `(?:\d+-\d+-)?PLAN\.md` could not match a decimal phase's `07.1-01-PLAN.md`, so a listed
   plan checklist counted 0. (b) Both render sites showed only the roadmap's plan checklist;
   ttbook's phases 1/5/6 have `**Plans**: TBD` and no list, while their directories hold the
   plans. The data existed on disk; the display did not consult it.
4. **Empty Milestone** — accurate to STATE.md (no `milestone:` key), but the roadmap names
   the in-progress milestone; there was no fallback.

## Resolution

root_cause: see above — `u32`-only phase numbers (1, 2), a decimal-blind plan regex and a
  roadmap-only plan count (3), no milestone fallback (4).

fix:
  - New `src/state_reader/phase_num.rs`: `PhaseNum` (dot-separated integer segments; `07.1`
    == `7.1`; ordered `7 < 7.1 < 8`; `M-2`/`4a` still abstain), `phase_key`, `same_phase`.
  - `ProjectState::{state_md_phase_number, current_phase_number}` are `Option<PhaseNum>`;
    `active_phase_number()` returns `PhaseNum` (same clamp, now decimal-aware);
    `PhaseMarker::decide`, `RoadmapWidget::current_phase_num`, `browser::resolve_active_phase_dir`
    and `app::phase_display_label` compare via `PhaseNum`.
  - `merge_duplicate_phases` keys on `phase_key` (first-seen spelling kept).
  - `phase_dir_matches` adds canonical + integer-part-padded candidates for decimal ids.
  - Plan regex accepts a decimal phase prefix.
  - `state_reader::phase_plan_counts`: roadmap checklist first, disk PLAN/SUMMARY counts when
    the roadmap lists none; both render sites use it. `None` still renders `0/?`.
  - `roadmap_md::active_milestone`: the `🚧` / `(in progress)` entry of `## Milestones`, used
    only when STATE.md has no `milestone:`.

verification: `cargo test --no-fail-fast` → 2183 passed / 1 failed (the known local
  `envelope::policy` git-version constants test). `cargo clippy -- -D warnings` clean;
  `--all-targets` shows only pre-existing lints in untouched code.

## Decisions [AUDIT]

- [AUDIT] "Current" semantics kept as the existing clamp `max(STATE.md, disk frontier)`, now
  over `PhaseNum`. For ttbook both sources say 7.1, so no disagreement needs flagging; no new
  "disagreement" indicator was added.
- [AUDIT] `disk_status.rs` was touched only in `phase_dir_matches` (directory-name matching),
  not in status inference — the earlier session's "do not touch disk_status.rs" constraint was
  about the inference.
- [AUDIT] Plan counts: roadmap checklist wins when present; disk counts only fill a missing
  list. Completed = `min(summary_count, plan_count)`.
- [AUDIT] Milestone fallback reads ROADMAP's `## Milestones` in-progress entry; STATE.md wins
  whenever it has a `milestone:` key.
- [AUDIT] Debugged directly in the dispatched subagent rather than via a nested
  `gsd-debug-session-manager` → `gsd-debugger` chain (no Agent() for GSD types was spawned, so
  no isolation dispatch applied).

## Regression coverage

- `tests/state_reader_test.rs` (`ttbook_shaped_planning_dir` fixture):
  `a_decimal_phase_written_padded_and_unpadded_is_one_row_with_its_disk_data`,
  `a_decimal_current_phase_is_the_active_phase_not_the_count_fallback`,
  `the_clamp_orders_decimal_phases_between_their_neighbours`,
  `the_rendered_roadmap_marks_the_decimal_phase_current_once`,
  `plan_counts_fall_back_to_disk_when_the_roadmap_lists_no_plans`,
  `milestone_falls_back_to_the_roadmaps_in_progress_milestone`.
- Unit: `phase_num::tests` (4), `roadmap_md::tests::{active_milestone_is_the_in_progress_entry_of_the_milestones_list, a_zero_padded_detail_heading_merges_with_its_checklist_entry}`,
  `disk_status::tests::test_phase_dir_matches_decimal_pad_insensitive`.

## Left open

- Other string-keyed phase lookups (`phase_disk_statuses` keyed by the roadmap's spelling,
  `deferred_verification_phases`, driver gates) were not audited for pad-insensitivity; they
  are internally consistent for the row that renders.
