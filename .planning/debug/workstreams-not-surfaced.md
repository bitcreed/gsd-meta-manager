---
status: awaiting_human_verify
trigger: "gsd-meta-manager doesn't show the parallel milestone (workstreams v0-13-public-beta, v0-14-voice-assistant) and active Phase 26 for registered project sentriq at /home/blk/projects/flutter/sentriq. Reproduce read-only, fix root cause with regression test."
created: 2026-10-22
updated: 2026-10-23
---

# Debug: sentriq workstreams / Phase 26 not surfaced

## Symptoms

- **Expected:** For the registered project `sentriq` (`/home/blk/projects/flutter/sentriq`,
  configured in `~/.config/gsd-meta-manager/config.json`), the TUI should surface the parallel
  workstreams under `.planning/workstreams/` — `v0-13-public-beta` (milestone v0.13, status
  human_needed, phases 16-22) and `v0-14-voice-assistant` (milestone v0.14 "Voice Mechanic",
  status executing, active Phase 26 = `26-push-to-talk-voice`, plans 26-01..26-14).
- **Actual:** Only the root milestone v0.12 "Actuation Routines" (status phase_complete, root
  phases top out at 16 and 999.x) is shown. Neither workstream nor Phase 26 appears.
- **Errors:** None reported — silent omission.
- **Timeline:** Workstream support exists (`src/state_reader/workstreams.rs`,
  `src/state_reader/mod.rs`, `src/app.rs`, `src/ui/screens/normal.rs`); the user believed it
  already covered this. Unknown whether it ever worked for this layout.
- **Reproduction:** Register sentriq and launch the TUI; or drive the state reader against
  the real sentriq tree READ-ONLY (never modify anything under /home/blk/projects/flutter/sentriq).

## Coordinator-established facts (verify, don't re-derive)

- Each workstream dir holds STATE.md, ROADMAP.md, REQUIREMENTS.md, HANDOFF.json,
  MILESTONE-CONTEXT.md, phases/, possibly milestones/ and quick/.
  - VERIFIED, with one correction: v0-14 has no HANDOFF.json (only v0-13 does).
- v0-14 phases/ holds 23,24,25,26,28 plus quick dirs and an `archived-20261022-phases/` dir.
  - CORRECTED: `phases/` holds exactly 23,24,25,26,28; `quick/` and the (empty)
    `milestones/archived-20261022-phases/` are siblings of `phases/`, not inside it.
- v0-14 STATE.md `stopped_at` is a ~1KB quoted string with semicolons, parentheses and commas
  (possible parse hazard).
  - ELIMINATED as a cause: it parses cleanly (see Eliminated).
- Workstream dir names use dashes (`v0-14-voice-assistant`); milestone ids use dots (v0.14).
- Root `.planning/milestone.lock` (phase 15) exists, plus 9 locked agent worktrees under
  `.claude/worktrees/` holding copies of root .planning — NOT where Phase 26 lives.

## Constraints

- Human unavailable: decide from planning artifacts and code; mark inferred decisions
  `[INFERRED — audit]`. Escalate only irreversible/scope-changing forks.
- Do not push (origin is public).
- Full suite: `rtk proxy cargo test --no-fail-fast` (raw output). A lone
  `envelope/policy.rs` git-version constants failure is expected on this machine.
- Also run `cargo clippy --all-targets`.
- sentriq tree is READ-ONLY.

## Current Focus

- hypothesis: CONFIRMED (see Resolution.root_cause). The workstream DATA layer is correct, and two
  downstream layers dropped it: the dashboard drew a workstream only as a trailing "[N ws]" count
  after the root's unbounded phase label (clipped away at every ordinary width), and the model's
  per-workstream active phase (declared 24 / frontier 27) never produced 26.
- reasoning_checkpoint:
    hypothesis: "Workstreams are loaded and parsed correctly but never surfaced: normal.rs drew only
      a count suffix appended AFTER a 54-cell root label in a ~33-cell Phase column (clipped), and no
      layer derived 26 for v0.14 because ProjectState::active_phase_number() = max(STATE.md declared
      24, disk frontier 27)."
    confirming_evidence:
      - "probe of the real parse_project_state on sentriq: n_workstreams=2, both STATE.md parse clean, v0.14 declared=24 frontier=27 active=27"
      - "RED regression test on unmodified code: dashboard text at 60/120/200 columns contains no workstream milestone/phase; at 120 cols the cue is absent, at 200 it is clipped to '[2'"
      - "gsd-core 1.18 milestone-lock.cjs: the workstream-scoped lock is GSD's structured claim of the phase a live session works; sentriq's v0.14 lock says 26 and is 53 min old"
    falsification_test: "a sentriq-shaped fixture rendered through the real NormalScreen must contain 'v0.14 P26' and 'v0.13 P22' at 60/80/120/200 columns; it must fail on the old code and when the claim rule is disabled"
    fix_rationale: "give each workstream its own dashboard line (milestone + focus phase + pipeline) so it cannot be clipped by the root label, and derive the focus phase from a live milestone.lock claim before the established rule; keep the established rule as the fallback"
    blind_spots: "the claim expires after 4 h (GSD's TTL), after which v0.14 falls back to P27; root row/badges/sort/summary counts still read only the root STATE.md; dashboard '/' filter does not search workstream text; Detail screen is still root-only"
    candidate_causes:
      - "code: UI draws count-only cue, appended after an unbounded label (CONFIRMED)"
      - "code: active-phase rule is wrong for parallel phases (CONFIRMED, fixed via live claim)"
      - "data: STATE.md current_phase is a single slot clobbered by parallel sessions (24 vs 26) (CONFIRMED, explains why the declared phase cannot be trusted)"
      - "config/environment: no .planning/active-workstream, session-scoped pointers live in tmp (CONFIRMED benign: workstreams still listed, just none 'active')"
      - "code: detection / STATE.md parse of the long stopped_at (ELIMINATED)"
    and_gate: "yes — the omission needed BOTH the clipped count-only UI AND a phase rule that cannot name 26; fixing either alone still leaves the owner without 'v0.14 · Phase 26' on screen"
- next_action: human verification of the dashboard against the real sentriq project.

## Evidence

- timestamp: 2026-10-23 — throwaway probe `tests/zz_debug_sentriq.rs` (deleted before commit)
  ran the REAL `parse_project_state` against the real sentriq `.planning` (read-only):
  `n_workstreams=2` (v0-13-public-beta, v0-14-voice-assistant), `active=false` for both (sentriq has
  no `.planning/active-workstream`; GSD 1.18 uses session-scoped tmp pointers + that shared marker).
  Detection layer: WORKS. Dashes in dir names are irrelevant to `load_workstreams`.
- timestamp: 2026-10-23 — same probe: both workstream STATE.md parse cleanly
  (`state_md_unreadable=false`, `state_md_recovered=false`; v0-14 status `executing`, milestone
  `v0.14`, 6 roadmap phases 23..28). The ~1 KB quoted `stopped_at` is NOT a parse hazard.
  Parse layer: WORKS.
- timestamp: 2026-10-23 — same probe, v0-14 model values: `state_md_phase_number=24` (STATE.md
  `current_phase: 24`, stale: GSD's single Current Position slot), `current_phase_number=27`
  (disk frontier: 23..26 are Executed, 27 has NO directory), so `active_phase_number()=27`.
  Per-phase disk: 23 Executed/HumanNeeded, 24 Executed/HumanNeeded, 25 Executed/HumanNeeded,
  26 Executed 14/14 plans verif=GapsFound, 27 NoDirectory, 28 Partial 5/9.
  Active-phase layer: yields 27 (and the label would be 24), never the 26 the owner is working.
- timestamp: 2026-10-23 — `.planning/workstreams/v0-14-voice-assistant/milestone.lock` =
  `{phase:"26", session, pid, updated_at: 1792718191629}` (2026-10-23T01:16:31Z; ~53 min old at
  probe time). Root lock is `{phase:"15", updated_at: 2026-10-20}`. gsd-core 1.18.0
  `bin/lib/milestone-lock.cjs`: "advisory (phase, session) claim over the single Current Position
  slot in STATE.md (#3311)", workstream-scoped via planningDir, live while
  `now - updated_at < MILESTONE_LOCK_TTL_MS` (4 h), conflict warns and never steals. This is GSD's own
  structured statement of which phase a live session is working in a parallel milestone.
  STATE.md `current_phase` flips (26 at ~20:16 local per state.json "Phase 4 of 6", 24 at 20:54).
- timestamp: 2026-10-23 — `grep workstream src`: `ProjectState.workstreams` is consumed in exactly
  ONE place, `src/ui/screens/normal.rs:1014-1026`, as `"  [ws:<active>]"` or `"  [N ws]"` appended to
  the Phase cell's `Line`. No Detail/Roadmap/Pipe screen, no classify_status, no filter reads it.
  Shipped as a "workstream badge" in v1.6 (quick 260722-emn plan 9). UI layer: count-only cue.
- timestamp: 2026-10-23 — `normal.rs` `dashboard_columns`: Phase column is Percentage(30) of the
  inner width (~33 cells at 120 cols, ~22 at 80). sentriq's root phase label is
  "Ram Catalog Procedures (experimental, from AlfaOBD RE)" (54 cells); the cue is appended after
  it, so ratatui clips the cue off. Even the count is invisible for this project.
- timestamp: 2026-10-23 — watcher is `RecursiveMode::Recursive`; `classify_change` treats
  `.planning/workstreams/**` (incl. `milestone.lock`) as `Planning`, so edits there do re-parse.
  Refresh layer: WORKS.
- timestamp: 2026-10-23 — dashboard mouse hit-testing (`ListRegion::row_at`) assumes one terminal
  row per project; any fix that draws more than one line per project must make it height-aware.
- timestamp: 2026-10-23 — RED on UNMODIFIED code (sentriq-shaped fixture through the real
  `NormalScreen::render`): 60 cols shows `Ram Catalog Proc` and nothing about workstreams; 120 cols
  shows `Ram Catalog Procedures (experimenta` and no cue; 200 cols shows `...AlfaOBD RE)  [2` (cue
  clipped mid-word). Test `sentriq_parallel_workstreams_are_drawn_with_their_milestone_and_active_phase`.
- timestamp: 2026-10-23 — GREEN after the fix; the real sentriq tree, driven through the new
  model API (read-only): `ws v0.13 P22: Play Release Readiness` (no claim, established rule) and
  `ws v0.14 P26: Push-to-Talk Voice` (live claim 26), both with an Executed focus pipeline.
  Rendered at 120 columns:
  ```
  > sentriq   Ram Catalog Procedures (experimenta   D  R  P  E  V  1/2 phases
              ws v0.13 P22: Play Release Readines   D  R  P  E  V
              ws v0.14 P26: Push-to-Talk Voice      D  R  P  E  V
  ```

## Eliminated

- hypothesis: workstream directories with dashed names are not detected
  evidence: `load_workstreams` takes every subdirectory; probe returned both. (2026-10-23)
- hypothesis: the long quoted `stopped_at` (`;` `(` `)` `,`) breaks workstream STATE.md parsing
  evidence: both workstream states parse with `state_md_unreadable=false`, `recovered=false`,
  status/milestone populated. (2026-10-23)
- hypothesis: `archived-20261022-phases/` dir breaks phase enumeration
  evidence: it sits under `workstreams/v0-14-voice-assistant/milestones/` (not `phases/`), is empty,
  and phases=23..28 enumerate from ROADMAP. The fixture also plants one inside `phases/`; it is not
  treated as a phase either. (2026-10-23)
- hypothesis: file watcher does not see workstream changes
  evidence: recursive watch + `classify_change` default Planning. (2026-10-23)
- hypothesis: the 823 KB root STATE.md (or its size) hides workstreams
  evidence: root parses fine (status phase_complete, milestone v0.12); workstreams loaded
  independently of it. (2026-10-23)
- hypothesis: the declared STATE.md `current_phase` is the right active phase for a workstream
  evidence: it reads 24 on disk now, while state.json (written 38 minutes earlier, which GSD
  derives from STATE.md's current_phase) says "Phase 4 of 6" = phase 26 — GSD's one Current
  Position slot is overwritten by whichever parallel session wrote last. (2026-10-23)

## Resolution

root_cause: >
  Two layers dropped data the reader had loaded correctly (detection `workstreams.rs:load_workstreams`
  and per-workstream parse were sound).
  (1) UI: `src/ui/screens/normal.rs` (the removed `ws_suffix`, formerly lines ~1014-1026) drew a
  workstream only as a `"  [N ws]"` count appended AFTER the project's phase label inside the Phase
  cell. sentriq's root label is 54 cells wide and the Phase column is `Percentage(30)` of the
  terminal, so the cue was clipped away at every ordinary width, and even drawn it named no
  milestone or phase; no other screen reads `ProjectState.workstreams`.
  (2) Model: `ProjectState::active_phase_number()` (`src/state_reader/mod.rs:341`) = max(STATE.md
  `current_phase`, disk frontier) gives 27 for v0.14 — STATE.md's single slot is stale (24) under
  parallel sessions and the frontier lands on phase 27, which has no directory — so no layer could
  ever name the phase being worked (26).
fix: >
  `WorkstreamState` now carries the workstream's `milestone.lock` as a typed `MilestoneClaim`
  (phase + heartbeat only; size-capped read) and exposes `live_claim_phase` / `focus_phase_number` /
  `focus_phase` / `focus_inference` (`src/state_reader/workstreams.rs`): a live claim (GSD's own 4 h
  window, phase must be on the roadmap) leads, else the established active-phase rule.
  `format_workstream_display` (`src/app.rs`) renders `ws v0.14 P26: Push-to-Talk Voice`.
  The dashboard (`src/ui/screens/normal.rs`) gives each workstream its own line under the project
  row, in the Phase cell (milestone and phase NUMBER first, so a narrow column clips the name) and
  the Status cell (the same D-R-P-E-V pipeline for the focus phase); rows are `height(1 + lines)`,
  capped at 4 lines with a `ws +N more` tail, and click hit-testing is height-aware
  (`dashboard_row_at`). The old `[N ws]` suffix is removed.
verification:
  target_test: { result: pass, test: "ui::screens::normal::tests::sentriq_parallel_workstreams_are_drawn_with_their_milestone_and_active_phase (RED on unmodified code at 60/120/200 columns, GREEN after)" }
  mutation_check: { result: pass, reason: "Stryker is not applicable to this Rust crate; mutants seeded by hand at both fix sites were killed", mutant_killed: "(a) live_claim_phase forced to None -> the_v014_focus_is_the_live_claim_26_not_declared_24_nor_frontier_27 and the dashboard test FAIL; (b) dashboard_row_at replaced by ListRegion::row_at -> mouse_click_below_a_project_with_workstreams_selects_the_project_under_the_pointer FAILS" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: true, note: "the only removed behaviour is the clipped [N ws] suffix, replaced by strictly more information" }
  adjacent_tests: { result: pass, suites_run: ["rtk proxy cargo test --no-fail-fast: 88 targets, 4114 passed, 1 failed (envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against - the expected machine-local git-version witness), 24 ignored", "cargo clippy --all-targets: clean, 0 warnings", "cargo fmt --check: clean"] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, note: "the regression test was run RED on the unmodified code before any production edit, then GREEN after" }
  guardrail_verdict: accepted
  oracle_type: "specified (the owner's stated expectation: v0.14 + Phase 26 visible) backed by a derived oracle (GSD's milestone-lock.cjs contract); boundary neighbours: claim at TTL-1min / TTL, claim 4 h+ in the future, claim for an unlisted phase, 6 workstreams vs the 4-line cap vs exactly 4"
inferred_decisions:
  - "[INFERRED — audit] Surface workstreams as extra LINES under the project row (multi-line table row) instead of new rows or a new Detail tab: keeps the selection/filter/sort model (one row per alias) intact, needs only a height-aware click test, and is the smallest change that puts milestone + phase on screen."
  - "[INFERRED — audit] A live `<workstream>/milestone.lock` claim outranks STATE.md `current_phase` and the disk frontier for the workstream's displayed phase: it is GSD's structured claim, STATE.md's slot is demonstrably clobbered (24 vs 26), and the claim is validated (4 h window mirrored from gsd-core, phase must be on the roadmap, size-capped read). The established rule stays the fallback."
  - "[INFERRED — audit] Root rows keep their existing phase rule and ignore the root milestone.lock (root claim is 2 days old; changing root behaviour is out of scope)."
  - "[INFERRED — audit] The workstream Status cell shows the focus phase's D-R-P-E-V pipeline (the dashboard's existing vocabulary); the workstream's own status text only appears as the line tint, and as the text fallback when it has no phase."
  - "[INFERRED — audit] Attribution trailer uses the harness-reported model (Sonnet 5.5) rather than the 'Opus 5.5' in the dispatch brief."
files_changed:
  - src/state_reader/workstreams.rs
  - src/app.rs
  - src/ui/screens/normal.rs
unresolved:
  - "After the v0.14 claim's 4 h heartbeat window lapses with no new heartbeat, the workstream falls back to the established rule and shows `P27` (a phase with no directory). Deliberate (GSD's own liveness rule), but visible."
  - "Dashboard row colour, attention badge, summary counts and sort still read only the root STATE.md: a project whose root is phase_complete but whose workstream is executing still counts as `done` in the footer."
  - "The dashboard `/` filter does not search workstream text; the Detail screen (Roadmap/Phases/Docs tabs) is still root-only."
  - "The `ws*` active marker needs `.planning/active-workstream`; GSD 1.18 mostly keeps the active pointer per session in a temp dir, so it is usually absent (sentriq has none)."
