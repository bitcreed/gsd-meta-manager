---
status: resolved
trigger: "Phase 3 renders with the future marker (o) instead of the done marker (+) in the roadmap display for picsync — regression from 3ffabfc, which added ProjectState::active_phase_number()"
created: 2026-09-10
updated: 2026-09-10
---

# Debug: roadmap phase marker reads "future" for a phase that is complete on disk

## Symptoms

- **Expected:** in the Detail screen's Phases list and in the Roadmap tab, picsync's
  phase 3 shows `+` (done), phase 4 shows `*` (current), phases 5-8 show `o` (future).
- **Actual:** phase 3 shows `o` (future). Phase 4 correctly shows `*`.
- **Errors:** none — wrong glyph, no failure.
- **Timeline:** introduced by `3ffabfc fix(phase): the active phase is what STATE.md
  says, then the disk, then arithmetic`. Before it, `current_phase_num` was
  `completed_phases + 1` = 3, so phase 3 caught the `*` arm by accident.
- **Reproduction:** open picsync in the TUI, Detail screen → Phases / Roadmap tab.

## Current Focus

hypothesis: CONFIRMED — the glyph was a two-source decision, "done" from ROADMAP's
`- [x]` checkbox (`RoadmapPhase::completed`), "current" from `active_phase_number()`.
test: `PhaseMarker::decide` against picsync's real `.planning/`
expecting: P1-P3 `+`, P4 `*`, P5-P8 `o`
next_action: none — resolved in 1a10482

## Evidence

- timestamp: 2026-09-10 — `src/ui/roadmap_widget.rs:31-39` `phase_icon(phase, is_current)`
  returns `+` when `phase.completed`, `*` when `is_current`, else `o`. `phase.completed`
  is the ROADMAP `- [x]` checkbox (`roadmap_md.rs:186`), NOT `completed_phases`.
- timestamp: 2026-09-10 — `src/ui/screens/detail.rs:3231-3237` repeats the same three-arm
  decision, same two sources.
- timestamp: 2026-09-10 — picsync `.planning/ROADMAP.md:30` is `- [ ] **Phase 3: ...**`,
  a stale unchecked box. Its `## Progress` Status cell reads
  `In Progress — verified, awaiting 2 live checks…`, which is not the exact
  `complete`/`done` match at `roadmap_md.rs:405-411`, so `completed_phases` is 2 as well.
- timestamp: 2026-09-10 — `ProjectState::phase_disk_statuses` already holds a per-phase
  `DiskInference` for every roadmap phase (`state_reader/mod.rs:296-317`), and picsync's
  phase 3 infers `Complete` / `verification_status: Passed`. The truth was already in
  memory; the glyph just never asked it.
- timestamp: 2026-09-10 — the "which phase is active" question was unified by 3ffabfc
  into `active_phase_number()`. The "is this phase done" question was left on the
  checkbox. Two questions, two sources, one of them stale.
- timestamp: 2026-09-10 — the same bug was live on THIS repository and unnoticed:
  phases 20 (`Complete`/`passed`) and 21 (`Executed`) have unchecked boxes and are not
  the current phase, so both drew `o`. Only picsync's phase 3 was reported.
- timestamp: 2026-09-10 — a first attempt ordered done ahead of current and cost this
  repository its `*` entirely: phase 19 is `Executed`/`human_needed`, so it read `+` and
  the roadmap named no current phase at all. Current must outrank done.
- timestamp: 2026-09-10 — verified against both real projects after the fix.
  picsync: `+ + + * o o o o` for phases 1-8. This repository: `+ + + + + * + + o o`
  for phases 14-23.

## Eliminated

- hypothesis: `active_phase_number()` returns the wrong number for picsync.
  Eliminated — the operator confirms the label reads phase 4, which is correct.
- hypothesis: the `## Progress` Status-cell matcher at `roadmap_md.rs:405-411` is too
  strict and should be loosened. Eliminated — declared out of scope; picsync's wording is
  deliberate, and loosening it would fix the count without fixing the checkbox, which is
  the actual input to the glyph.

## Resolution

root_cause: the per-phase marker asked two different sources — "done" from the ROADMAP
`- [x]` checkbox, "current" from `active_phase_number()`. picsync's phase 3 is complete on
disk but its checkbox is stale, so after 3ffabfc moved "current" from 3 to 4 the phase
matched neither arm and fell through to `o`. The `*` it used to show was the arithmetic
being wrong in a way that happened to look right.

fix: one `phase_marker()` decision in `state_reader`, consulted by both glyph sites.
"Done" now comes from that phase's own `DiskInference` (`status >= Executed`, the same
threshold the frontier uses), with the ROADMAP checkbox as the fallback for a phase with
no directory on disk. "Current" stays `active_phase_number()` — one notion of current.

verification: `cargo build` clean; `cargo clippy -- -D warnings` clean; `cargo test
--no-fail-fast` 1965 passed / 4 failed against a measured baseline of 1957 passed / 4
failed on the same machine — the same four, all pre-existing and environmental (the
git-version pin, and three driver/envelope tests that pass in isolation). Eight new
tests: six in `tests/state_reader_test.rs` (picsync's exact shape end to end, plus the
degrade path and the precedence) and two in `src/ui/roadmap_widget.rs` that read the
glyph off rendered cells rather than off the decision that produced them.

files_changed: src/state_reader/mod.rs, src/ui/roadmap_widget.rs,
src/ui/screens/detail.rs, src/ui/mod.rs, tests/state_reader_test.rs
