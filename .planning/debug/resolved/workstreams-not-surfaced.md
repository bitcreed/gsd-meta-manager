---
status: resolved
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

## Human Verification Result (2026-10-23)

- Dashboard fix VERIFIED: the overview now lists sentriq's workstreams.
- NEW GAP: the Detail screen's `1:Roadmap` and `2:Phases` tabs still read only the root
  `.planning/`. The user expects the v0.13 and v0.14 milestones and their phases there,
  including Phase 26 and its plans 26-01..26-14, each marked with the workstream/milestone it
  belongs to. Session stays open (do not mark resolved). Presentation choice must fit the
  existing tab UI and UI-SPEC conventions and be marked `[INFERRED — audit]`.

## Final Human Verification (2026-10-23) — SESSION CLOSED

- Round 1 (dashboard, fix 0fd27fa): CONFIRMED FIXED by the owner.
- Round 2 (Detail 1:Roadmap and 2:Phases, fix d1b1457): CONFIRMED FIXED by the owner
  ("Looks great").
- Both rounds verified end-to-end against the real sentriq project. The "Session stays open"
  note in the section above is superseded: the session is resolved and archived.

## Current Focus

- status (2026-10-23): RESOLVED. Both rounds verified by the owner ("Looks great"); nothing
  further to investigate. The `unresolved:` list below records the known, deliberate limits.
- (history) next_action, round 2: human verification of the Detail screen's 1:Roadmap and
  2:Phases tabs against the real sentriq project (fix commit d1b1457) — done, confirmed.

- round 2 (Detail tabs) hypothesis: CONFIRMED (see Resolution.root_cause, round 2). Both tabs read
  only the project's own `ProjectState` (`state.phases`, `state.milestones`,
  `state.phase_disk_statuses`); `ProjectState.workstreams` was never consulted, and phase numbers
  were treated as globally unique keys (they are not: root and v0.13 both have a Phase 16).
- round 2 reasoning_checkpoint:
    hypothesis: "The Detail Roadmap and Phases tabs omit workstreams because every list, key and
      lookup they use is built from the project's own roadmap alone (roadmap_model_for / render_roadmap
      read state.phases + state.milestones; render_pipeline_tab / selected_waves / pipeline_selected
      index state.phases), and a bare phase number is their only phase identity."
    confirming_evidence:
      - "RED on unmodified production code: 5 end-to-end tests (Roadmap lists v0.13/v0.14 bands and phases at 80/100/120/200 cols; Phases lists ws rows; Phase 26 + 14 plans; Enter opens the ws phase; `e` edits the ws plan) all fail; the printed frames show only `v0.12` with phases 15 and 16"
      - "sentriq fixture and the real tree both have two roadmaps with a Phase 16 (root + v0.13), so a number-keyed merge would silently collide"
      - "the real tree through the fixed code (read-only): bands `v0.12 8/9`, `ws v0.13 7/7`, `ws v0.14 4/6`, 22 listed phases, Phase 26 `[E 14/14]` with all 14 plans under w1..w6"
    falsification_test: "a hand-seeded mutant at each of: scoped key, dependency scope, parallel scope, agent-view scope, edit path, selection sharing, waves key, live-claim focus must fail at least one test; the 5 end-to-end tests must fail on the unmodified production code"
    fix_rationale: "make a phase's identity (workstream, number) instead of the number alone, and read each phase against the state that owns it: one scoped band per workstream in the single Roadmap model, one scoped phase list behind the Phases tab's index. The alternative (a scope switcher that swaps the whole tab to one workstream) was rejected because the owner asked to SEE v0.13 and v0.14 and their phases, each marked, not to switch to them."
    blind_spots: "the box view (`v`) still draws the project's own phases only; a workstream with no roadmap phases draws no band (layout skips member-less bands); the agents/worktree view is not applied to workstream phases; the Docs tab and Docs>Milestones are still project-only; dashboard counts/sort/filter still read only the root"
    candidate_causes:
      - "code: tabs read only the root ProjectState (CONFIRMED)"
      - "code: phase number used as a global identity key (CONFIRMED, fixed by scoped keys)"
      - "data: the state reader did not load the workstreams (ELIMINATED: ProjectState.workstreams holds both, each with phases, disk statuses and goals)"
      - "config/environment: no `.planning/active-workstream` (CONFIRMED benign: ws markers show `ws`, not `ws*`)"
    and_gate: "yes — omission needed BOTH the root-only lists AND number-only identity: listing workstream phases without scoped keys would have collided root 16 with v0.13 16 (wrong selection, wrong Enter target, shared fold state), so both had to change together"

- round 1 hypothesis: CONFIRMED (see Resolution.root_cause, round 1). The workstream DATA layer is correct, and two
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
- round 1 next_action (done, verified by the owner): human verification of the dashboard against
  the real sentriq project.

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

### Round 2 evidence (Detail 1:Roadmap and 2:Phases)

- timestamp: 2026-10-23 — code read: `render_roadmap` (detail.rs, pre-fix 6595) builds its model with
  `roadmap_model_for(state, ..)` (pre-fix 2852), whose nodes are `state.shipped_phases`,
  `state.roadmap_entries()` and ghosts, and whose bands are `state.milestones` (+ a synthetic one);
  `render_pipeline_tab` (pre-fix 7180) lists `state.phases`, clamps `pipeline_selected` against
  `state.phases.len()` and reads `state.phase_disk_statuses`; `selected_waves` (pre-fix 1785),
  `roadmap_activate` (pre-fix 1737), `share_pipeline_selection` (pre-fix 3139) and the j/k/PageDown
  arms (pre-fix 3719, 4051) all index `state.phases` and match phases by bare `phase_key(number)`.
  `ProjectState.workstreams` appears in none of them.
- timestamp: 2026-10-23 — RED on UNMODIFIED production code (only the fixture's Phase 26 grown from 3
  to 14 plans): `sentriq_roadmap_tab_lists_each_workstream_milestone_and_its_phases`,
  `sentriq_phases_tab_lists_workstream_phases_marked_with_their_milestone`,
  `sentriq_phases_tab_shows_phase_26_with_its_workstream_and_fourteen_plans`,
  `sentriq_roadmap_enter_opens_the_workstream_phase_not_a_same_numbered_one`,
  `sentriq_waves_pane_e_edits_the_plan_inside_the_workstream` — 5 failed, 13 other `sentriq_` tests
  passed. The failing frames showed the Roadmap with only `v0.12 — Actuation Routines` (phases 15, 16)
  and the Phases list with only `P15` / `P16`; pressing `j` 20-40 times never reached anything else.
- timestamp: 2026-10-23 — GREEN after the fix (commit d1b1457): the same 5 pass. Full suite
  `rtk proxy cargo test --no-fail-fast` (raw, to a log file): 88 targets, 4136 passed, 1 failed
  (the expected `envelope::policy` git-version witness: installed git 2.53.0 vs derived 2.55.0), 24
  ignored. `cargo clippy --all-targets`: 0 warnings. `cargo fmt --check`: clean.
- timestamp: 2026-10-23 — hand-seeded mutants, each killed: agent view applied to workstream phases
  (agent-view test); scoped key ignoring the workstream (11 tests); dependency index ignoring scope
  (2 graph tests); `parallel` without the scope filter (1 graph test); `e` edit path without the
  workstream directory (e-edit test); `share_pipeline_selection` with a bare key (shared-selection
  test); Waves model key not scoped (waves-key test); workstream focus ignoring the live claim
  (model test). All reverted; the suite was re-run green after.
- timestamp: 2026-10-23 — the REAL sentriq tree, READ-ONLY (a throwaway in-crate probe, deleted before
  commit, called only `parse_project_state` and rendered the Detail screen into a `TestBackend`; nothing
  under /home/blk/projects/flutter/sentriq was written): `workstreams = [v0-13-public-beta,
  v0-14-voice-assistant]`, 22 listed phases (9 root, 7 v0.13, 6 v0.14); bands
  `v0.12 Actuation Routines 8/9`, `ws v0.13 Public Beta 7/7`, `ws v0.14 — Voice Mechanic 4/6`.
  Roadmap tab at 150 columns (excerpt; the v0.14 band shows its REAL dependency lanes, W1..W4):
  ```
  ▾ ws v0.13 Public Beta  7/7 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◍ 16  Beta Build Channel            0/3  W1   ...   ◉ 22  Play Release Readiness   0/3  W1
  ▾ ws v0.14 — Voice Mechanic  4/6 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◍ 23  Mechanic Mode Foundation and Cost Guardrails   11/11  W1
  ├─╮
  ◍ │ 24  Live Read-Only Queries and Online Lookup      11/11  W2
  │ ◍ 25  Conversation Memory                            4/4  W2
  ├─╯
  ◍ 26  Push-to-Talk Voice                             14/14  W3
  ├─╮
  ◦ │ 27  Parity, Designs and Wake-Word Verdict            —  W4
    ◉ 28  Mechanic Toolbox, Full Data Access and Voice-Started Actions  7/9  W4
  ```
  Phases tab, Phase 26 selected: `ws v0.14 P26: Push-to-Talk Voice  6w`, right pane
  `Phase 26: Push-to-Talk Voice` / `ws v0.14 Voice Mechanic · workstream v0-14-voice-assistant` /
  `[E 14/14]` / `Verify: human_needed — /gsd-verify-work 26 --ws v0-14-voice-assistant`, and the Waves
  pane `w1–w6 ✓ 14/14 done` which, opened (`Enter` into the pane, `Enter` on the row), lists
  `26-01 … 26-14` under w1 (6 plans), w2 (4), w3..w6 (1 each), each with title, state and tokens.
  (Phase 28 is `◉` active in the Roadmap here because the live claim now names 28, not 26: the sentriq
  tree moved on since round 1; the display follows the claim by design.)

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
- hypothesis (round 2): the Detail tabs omit workstreams because the state reader never loaded their
  phases, disk statuses or goals
  evidence: every `WorkstreamState.state` is a full `ProjectState` from the same `parse_project_state`
  (phases, `milestones`, `phase_disk_statuses`, `phase_goals`, plan counts); the real tree yields 22
  listed phases once the tabs read them. The data was there; the tabs did not read it. (2026-10-23)
- hypothesis (round 2): phase numbers are a safe identity across the project's and the workstreams'
  roadmaps
  evidence: sentriq's root and v0-13-public-beta both have a Phase 16 (and the real tree has the same
  collision). A number-keyed merge would open/select/fold/edit the wrong phase; the end-to-end tests
  `..._enter_opens_the_workstream_phase_not_a_same_numbered_one` and
  `..._phases_selection_is_shared_with_the_roadmap_by_scoped_key` pin it. (2026-10-23)

## Resolution

root_cause: >
  ROUND 2 (Detail 1:Roadmap and 2:Phases, fixed in d1b1457). Both tabs read only the project's own
  `ProjectState`, and identified a phase by its bare number. Pre-fix locations in
  `src/ui/screens/detail.rs` (at 0bab7d4): `roadmap_model_for` 2852 (nodes from `state.shipped_phases` /
  `state.roadmap_entries()`, bands from `state.milestones`), `render_roadmap` 6595,
  `render_pipeline_tab` 7180 (list from `state.phases`, `pipeline_selected` clamped to
  `state.phases.len()`, disk data from `state.phase_disk_statuses`), `selected_waves` 1785,
  `roadmap_activate` 1737, `share_pipeline_selection` 3139 and the j/k/PageDown arms 3719 / 4051 (all
  index `state.phases` and match `phase_key(number)`). `ProjectState.workstreams` was never read, and
  a bare number is not a safe key: sentriq's root and v0-13-public-beta both have a Phase 16.
  ROUND 1 (dashboard, fixed in 0fd27fa).
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
  ROUND 2 (d1b1457). A phase's identity is now (workstream, number): `scoped_phase_key`
  (`state_reader/workstreams.rs:170`) keeps the project's bare `phase_key` and prefixes a workstream's
  with `<dir>/`. `ProjectState::scopes` / `scoped_phases` / `scoped_phase_count`
  (`state_reader/mod.rs:356-382`) give the project's phases then each workstream's, each read against
  the state that owns it (`ScopedPhase.owner`). Roadmap: `roadmap_graph.rs` gains `Scope` on
  `ListNode` / `BandInput` (dependencies resolve only inside a scope, `resolve_deps` :57; band and phase
  keys carry it, `band_key` :1119 / `scoped_key` :1131; `parallel`, `wave_step` and `max_wave_of` :1376
  are per roadmap) and `PhaseFacts.scope` / `BandFacts.scope`; `roadmap_model_at` (`detail.rs:2901`;
  `roadmap_model_for` is its `Utc::now()` wrapper) appends ONE band per workstream
  (`workstream_band_label` :3201 -> `ws v0.14 — Voice Mechanic`) with the workstream's live-claim focus
  phase as the current one; `roadmap_view.rs` names a workstream phase in Start-now
  (`ws v0.14 P26 …`), adds a `workstream <dir>` line to the band / phase detail panes and uses the
  phase's own roadmap for "wave N of M". Phases: `pipeline_selected` indexes `scoped_phases`
  (`render_pipeline_tab` :7326, `selected_waves` :1786, the j/k/PageDown arms, `share_pipeline_selection`
  :3280, `roadmap_activate` :1681); rows read `ws v0.14 P26: Push-to-Talk Voice` (`workstream_prefix`
  :8928, workstream colour), the selected workstream phase shows `ws v0.14 Voice Mechanic · workstream
  <dir>`, the Waves model carries `workstream` and a scoped `phase_key` (fold toggles never collide),
  `e` opens the plan under `workstreams/<dir>/phases/` (`waves_edit` :2037), every printed `/gsd-…`
  remedy carries `--ws <dir>` (`verification_hint_spans` :9216 and the `No plans yet` hint), and the
  agents scan's view (the PROJECT's worktrees, keyed by bare number) is never applied to a workstream
  phase (`agent_view_of` :9512). `app.rs` gains `workstream_marker` / `workstream_milestone_label` /
  `workstream_tag`, shared with the dashboard's `format_workstream_display`.
  ROUND 1 (0fd27fa).
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
human_verification: { result: confirmed, date: 2026-10-23, round_1: "dashboard (0fd27fa) confirmed fixed", round_2: "Detail Roadmap/Phases tabs (d1b1457) confirmed fixed — owner: 'Looks great'" }
verification_round_2:
  target_test: { result: pass, test: "five end-to-end tests in ui::screens::detail::tests (sentriq_roadmap_tab_lists_each_workstream_milestone_and_its_phases, sentriq_phases_tab_lists_workstream_phases_marked_with_their_milestone, sentriq_phases_tab_shows_phase_26_with_its_workstream_and_fourteen_plans, sentriq_roadmap_enter_opens_the_workstream_phase_not_a_same_numbered_one, sentriq_waves_pane_e_edits_the_plan_inside_the_workstream): RED on the unmodified production code (5 failed), GREEN after" }
  more_tests: "detail.rs: model (bands/keys/focus/lapsed claim), Space folds one band, j/PageDown bounds, scoped selection sharing, --ws remedies, agent view not applied, waves model keys, finished phase opens its waves to 26-01..26-14 (waved fixture), band-label fallbacks; roadmap_graph.rs: same-number phases, dependency scope, wave/parallel scope, scoped band label + independent fold, cursor across scopes, escaped scope text; workstreams.rs: scoped keys, scoped_phases order/identity/flat-project parity. +22 tests in total."
  mutation_check: { result: pass, reason: "Stryker is not applicable to this Rust crate; mutants hand-seeded at the fix sites and killed", mutant_killed: "agent view applied to workstream phases; scoped key ignoring the workstream (11 tests); dependency index ignoring scope; parallel without scope filter; edit path without the workstream dir; share_pipeline_selection with a bare key; unscoped waves key; focus ignoring the live claim" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: true, note: "nothing was deleted: the project's own keys, rows and panes are byte-for-byte unchanged (scope None); all ~2900 pre-existing lib tests pass unmodified apart from `scope: None` / the verification_hint_spans signature" }
  adjacent_tests: { result: pass, suites_run: ["rtk proxy cargo test --no-fail-fast (raw, to a log): 88 targets, 4136 passed, 1 failed (envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against: installed git 2.53.0 vs derived 2.55.0, the expected machine-local witness), 24 ignored", "cargo clippy --all-targets: 0 warnings", "cargo fmt --check: clean"] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, note: "the 5 end-to-end tests were run RED on the unmodified production code before any production edit (only the fixture grew Phase 26 to 14 plans), then GREEN after" }
  read_only_real_check: "the real sentriq tree through the fixed Detail screen (parse + render only, nothing written under it): see Evidence, Round 2"
  guardrail_verdict: accepted
  oracle_type: "specified (the owner's stated expectation: v0.13 and v0.14 and their phases, Phase 26 and plans 26-01..26-14, each marked with its workstream/milestone) with metamorphic checks (a project without workstreams renders exactly as before; a same-numbered phase in two roadmaps stays two phases); boundary neighbours: same number in root and workstream, workstream with and without a live claim, claim lapse, folded vs open band, first/last/over-the-end selection index, flat vs waved plans"
verification_round_1:
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
  - "[INFERRED — audit] Round-1 commits 0fd27fa / 0bab7d4 carry a Sonnet 5.5 trailer (left as-is per operator); round-2 commits were reworded by the coordinator to the briefed 'Opus 5.5' trailer; the Claude-Session line is as briefed."
  - "[INFERRED — audit, round 2] PRESENTATION: ONE combined list per tab, not a scope switcher. Roadmap: after the project's own band(s), one band per workstream labelled `ws v0.14 — Voice Mechanic` (the dashboard's `ws` marker + the workstream's own milestone heading), its phases listed under it like any milestone's. Phases: one flat list, the project's phases then each workstream's, each workstream row led by `ws v0.14 P26:` in Magenta (UI-SPEC: Magenta is the workstream cue; the words carry the meaning, never the colour alone). Chosen because the owner asked to SEE v0.13 and v0.14 and their phases, each marked, and because both existing tabs already group by band / list by row, so no new key, tab or screen is needed (the Roadmap/Phases/help key tables are unchanged)."
  - "[INFERRED — audit, round 2] A phase's identity is (workstream, number): the project's keys are unchanged, a workstream's is `<dir>/<phase key>`. Dependencies, waves, `parallel`, `[`/`]`, folds, the Waves fold toggles, Enter, `e` and the Roadmap<->Phases selection sharing all use it. Rejected: renumbering or de-duplicating same-numbered phases (it would hide real phases)."
  - "[INFERRED — audit, round 2] A workstream's `◉` (current) phase is `WorkstreamState::focus_phase_number` (live milestone.lock claim, else the established rule) — the SAME phase the dashboard names — rather than the established rule alone. The Waves pane's `active` flag follows the same phase."
  - "[INFERRED — audit, round 2] The agents scan's view (the project's worktrees; plans keyed by bare phase number) is not applied to workstream phases: no worktree badge, no worktree-held plan counts, no unmerged post-pass, no `Enter`-to-agent for them. Applying it would attribute the project's Phase N worktree plans to a workstream's Phase N."
  - "[INFERRED — audit, round 2] Every `/gsd-…` remedy printed for a workstream phase (`Verify:` line, `No plans yet`) carries `--ws <dir>`, GSD's own workstream flag (`gsd-core/references/workstream-flag.md`), because a bare command would resolve against whichever planning directory the session picks. `e` opens the plan from `<project>/.planning/workstreams/<dir>/phases/`."
  - "[INFERRED — audit, round 2] A finished workstream phase keeps the existing Waves default (`w1–w6 ✓ 14/14 done`, merged and collapsed, design I-4) — the plans 26-01..26-14 are one `Enter` into the pane and one `Enter` on that row away, exactly as for the project's own finished phases. Not changed: consistency over a special case."
  - "[INFERRED — audit, round 2] Only the workstream's active milestone band is drawn (its `phases` + planned entries); a workstream's shipped/closed milestones and its `shipped_phases` are not. A workstream whose roadmap lists no phase draws no band (the layout skips member-less bands)."
files_changed:
  - src/state_reader/workstreams.rs
  - src/app.rs
  - src/ui/screens/normal.rs
  - "round 2: src/state_reader/workstreams.rs, src/state_reader/mod.rs, src/app.rs, src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/mod.rs (test literal), src/ui/screens/detail.rs"
unresolved:
  - "The `ws*` active marker needs `.planning/active-workstream`; GSD 1.18 mostly keeps the active pointer per session in a temp dir, so it is usually absent (sentriq has none)."
  - "The Roadmap `Start now:` line now also lists active/ready workstream phases (e.g. `ws v0.14 P27 …`), so on a project with several workstreams it truncates earlier."
  - "A workstream's closed (shipped) milestones are still not drawn as Roadmap bands (Docs > Milestones lists them since quick 261023-05h)."
closed_by_quick_batch_261023_05f:
  - "Lapsed-claim fallback to a directory-less phase (P27): 261023-05m — the fallback now prefers a real, unfinished phase, then the established rule."
  - "Dashboard row colour, badge, footer counts and attention sort read only the root: 261023-05j — aggregate status over root + workstreams."
  - "Dashboard `/` filter: 261023-05k. Docs tab and Docs > Milestones: 261023-05h. Roadmap header: 261023-05g. Box view (`v`): removed by quick 261022-wtw."
  - "No agents/worktree integration for workstream phases: 261023-05i."
  - "A workstream with no roadmap phases is not drawn: 261023-05l — band header plus a dim placeholder row (Roadmap) and a placeholder line (Phases)."
