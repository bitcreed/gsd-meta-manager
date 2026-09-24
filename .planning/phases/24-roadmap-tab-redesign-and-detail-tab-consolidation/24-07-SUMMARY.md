---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 07
subsystem: ui
tags: [ratatui, detail-view, tabs, archive, docs, keybindings]
status: complete

requires:
  - phase: 24-03
    provides: the new tab order, count-agnostic tab tests, PhaseList removal
  - phase: 24-05
    provides: the Roadmap `Enter` branch on the shipped-milestones row (re-routed here)
  - phase: 24-06
    provides: the final Roadmap render and `a_disk_complete_phase_with_an_unticked_box_draws_done`
provides:
  - "Final eight-tab detail view plus Driver: TAB_COUNT 9, DRIVER_TAB_INDEX 8, widths 89/63/78/55"
  - "Archive folded into Docs as its Milestones sub-tab (shares Docs' index 7)"
  - "switch_to_sub_view as the single arrival rule; switch_to_tab an index adapter; opened_on calls switch_to_sub_view"
  - "Docs `m` switch and docs_sub_tab_strip on both Docs renders"
  - "Roadmap Enter on the shipped row opens Docs › Milestones"
  - "Digits 1-8 only; 9 and 0 inert; footer [1-8/D] / [1-8]; Docs footers advertise [m]"
  - "App test helper open_docs_milestones; the five debug-fix archive tests ported"
affects: [phase-24 verification, README, docs/ARCHITECTURE.md, help popup]

actuals:
  tokens: 14000
  tasks: 3
  commits: 4
plan_head_before: 2bfb9e8bd32e9e4cb026ef0dc5f96897398ad7d1

tech-stack:
  added: []
  patterns:
    - "A tab index names a tab, never a sub-tab: sub-tabs are entered by sub-view through switch_to_sub_view"
    - "Sub-tab strip drawn as the first row of each sub-view render, shrinking the body by one row"

key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/app.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/ARCHITECTURE.md
    - .planning/todos/completed/2026-08-22-phase-list-grey-marker-disagrees-with-disk-inferred-stage.md

key-decisions:
  - "DetailSubView::Archive kept as the Docs › Milestones sub-view sharing index 7 with Browse (RESEARCH Pattern 6, A5), not removed"
  - "`m` is the Docs sub-tab switch; footer hints `[m]ilestones` on Files and `[m] files` on Milestones"
  - "The archive breadcrumb keeps its `Archive` literal so `Archive > v1.2` is unchanged (no title string changes)"
  - "The Roadmap footer stays at 91 cols (flag on) / 87 (flag off): bringing it under 80 would drop or rename hints, a scope change"

patterns-established:
  - "switch_to_sub_view is the one arrival rule for every way onto a tab or sub-tab"

requirements-completed: [D-B01, D-B04, D-B05, D-B06, D-B09, D-B10, D-B11]

coverage:
  - id: D1
    description: "Final tab set: 1:Roadmap .. 8:Docs + D:Drive, TAB_COUNT 9, DRIVER_TAB_INDEX 8, widths re-derived"
    requirement: D-B01
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_tab_bar_widths_are_the_label_arrays_own_arithmetic"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#every_tab_index_round_trips_through_its_sub_view"
        status: pass
      - kind: unit
        ref: "src/app.rs#the_driver_sub_view_is_the_last_tab_index_in_both_directions"
        status: pass
    human_judgment: false
  - id: D2
    description: "Archive is Docs › Milestones: shares Docs' index, `m` switches Files/Milestones, the strip marks the active sub-tab, opened_on(.., Archive, ..) lands on Milestones"
    requirement: D-B04
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#opened_on_archive_lands_on_docs_milestones"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#m_switches_docs_between_files_and_milestones"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#m_is_inert_outside_docs"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_stored_archive_view_is_the_docs_tab_with_milestones_active"
        status: pass
    human_judgment: false
  - id: D3
    description: "The five debug-fix archive regression tests pass unchanged through Docs then `m`"
    requirement: D-B06
    verification:
      - kind: integration
        ref: "src/app.rs#archive_milestone_view_keeps_its_content_across_the_periodic_prune"
        status: pass
      - kind: integration
        ref: "src/app.rs#the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry"
        status: pass
      - kind: integration
        ref: "src/app.rs#archive_milestone_view_reloads_in_place_when_planning_files_change"
        status: pass
      - kind: integration
        ref: "src/app.rs#the_archive_milestone_list_picks_up_a_newly_archived_milestone_in_place"
        status: pass
      - kind: integration
        ref: "src/app.rs#an_in_place_reload_that_shrinks_the_listing_keeps_the_cursor_on_a_row"
        status: pass
    human_judgment: false
  - id: D4
    description: "Roadmap Enter on the shipped-milestones row lands on Docs › Milestones with discovery scheduled"
    requirement: D-B04
    verification:
      - kind: integration
        ref: "src/app.rs#enter_on_the_shipped_milestones_row_opens_docs_milestones"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#roadmap_enter_on_the_shipped_row_opens_the_archive_view"
        status: pass
    human_judgment: false
  - id: D5
    description: "Digits 9/0 inert; footer [1-8/D] / [1-8]; Docs footers advertise [m]; Right from Docs reaches Driver only with the flag on"
    requirement: D-B10
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#nine_and_zero_are_inert_in_the_detail_view"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_docs_footers_advertise_the_milestones_switch"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_tabs_hint_names_shift_d_on_every_tab"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#right_from_docs_reaches_the_driver_tab_only_with_the_flag_on"
        status: pass
    human_judgment: false
  - id: D6
    description: "Help, README, ARCHITECTURE and escape-guard prose agree with the eight-tab layout; stale-literal sweep empty"
    requirement: D-B05
    verification:
      - kind: unit
        ref: "src/ui/screens/help.rs#the_docs_milestones_switch_is_documented"
        status: pass
      - kind: other
        ref: "rtk proxy git grep -n -F -e '[1-0' -e '[1-9/D]' -e '1:Phases' -e '5:Pipe' -e '8:Arch' -e '9:Arch' -e '0:Docs' -e 'DetailSubView::PhaseList' -e '10-tab' -e 'ten tabs' -e 'eleven tabs' -- src tests README.md docs (no output)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The Docs strip reads well on screen (bracket plus reverse video, dim `m switch`)"
    verification: []
    human_judgment: true
    rationale: "Visual quality of the sub-tab strip is a subjective call; the tests prove only its text and placement"

duration: 13min
completed: 2026-09-24
---

# Phase 24 Plan 07: Docs › Milestones and the Final Eight-Tab Layout Summary

**The milestone archive now lives in the Docs tab as a `Milestones` sub-tab (with `m` to switch between Files and Milestones), which leaves the detail view at eight digit-addressed tabs plus the Driver tab. The five debug-fix archive regression tests pass unchanged through the new path.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-09-24T04:32:00Z
- **Completed:** 2026-09-24T04:45:22Z
- **Tasks:** 3 (all complete)
- **Files modified:** 7

## Accomplishments

- **Final tabs (D-B01, D-B11):** `1:Roadmap 2:Phases 3:Backlog 4:Git 5:Queue 6:Sess 7:Cfg 8:Docs` + `D:Drive` (compact `1:Rd … 8:Dc`, `D:Dr`). `TAB_COUNT` 9, `DRIVER_TAB_INDEX` 8. Bar widths are 89/63/78/55, and the anti-drift test re-derives them. The flag-off full bar (78) now fits 80 columns whole.
- **Docs › Milestones (D-B04):** `tab_index(&Archive) == tab_index(&Browse) == 7`, and `sub_view_from_index(7) == Browse`. `switch_to_sub_view` holds the one arrival rule (the Archive discovery block is unchanged). `switch_to_tab` resolves an index and delegates. `opened_on` calls `switch_to_sub_view` directly. A guarded `m` arm switches Browse and Archive. `docs_sub_tab_strip` (`[Files] │ Milestones   m switch` / `Files │ [Milestones]   m switch`, active entry reversed) draws in the first row of both Docs renders.
- **Five archive tests ported (D-B06):** `open_archive_tab` became `open_docs_milestones`, which does `opened_on(.., Browse, ..)` and then a real `m` key through `App::update`. No assertion line in the five tests changed, and all five pass by exact name (`5 passed`).
- **Roadmap shipped row:** `Enter` calls `switch_to_sub_view(.., Archive, ..)`. An App-level test proves milestone discovery arrives on the real channel.
- **Digits and footer (D-B10):** the `9` arm is gone, so `9` and `0` fall through to the no-op arm. The footer reads `[1-8/D]` or `[1-8]`. The Browse footer adds `[m]ilestones` and the Archive footer adds `[m] files`.
- **Every consumer agrees (D-B05, D-B09):** the enum docs, escape-guard prose and arrival reasons, adjudication prose, help row `m`, README and ARCHITECTURE all match. A stored `Archive` (in-memory view state) renders as Docs with Milestones active, so no migration is needed.
- The folded phase-list marker todo is closed with a `## Resolution` section.

## Task Commits

1. **Task 1: Docs › Milestones end to end (tracer)**: `2d89a22` (feat)
2. **Task 2: inert 9/0, final footer, Docs hints, enum and probe prose (TDD)**: RED `28a51cd` (test), GREEN `c17b8e4` (feat). No refactor was needed.
3. **Task 3: help, README, architecture doc, folded todo, gates**: `4e10df7` (docs)

**Plan metadata:** see the final `docs(24-07)` commit.

## Files Created/Modified

- `src/ui/screens/detail.rs`: final constants, labels and mappings; `switch_to_sub_view`; `opened_on`; the `m` arm; `docs_sub_tab_strip` / `docs_sub_tab_row`; Roadmap shipped-row routing; the `9` arm deleted; footer `[1-8/D]` and `[m]` hints; 8 new tests plus the updated footer, round-trip and shipped-row tests.
- `src/app.rs`: `Archive` / `Browse` / `Driver` enum docs; `open_docs_milestones`; the Driver-index literal test at 8; `enter_on_the_shipped_milestones_row_opens_docs_milestones`.
- `src/ui/screens/render_escape_guard.rs`: tab and sub-view count prose; the Archive arrival reasons now name Docs › Milestones (labels and `ALL_SUB_VIEWS` are unchanged).
- `src/ui/screens/help.rs`: the `m` row, plus `the_docs_milestones_switch_is_documented`.
- `README.md`: the 8-tab bullet, both archive bullets, and the key table.
- `docs/ARCHITECTURE.md`: the `detail.rs` sub-view list.
- `.planning/todos/{pending → completed}/2026-08-22-phase-list-grey-marker-disagrees-with-disk-inferred-stage.md`: moved with `git mv`; `completed` / `resolved_by` fields and a `## Resolution` section added.

## Decisions Made

- The `Archive` variant stays as the sub-view rather than being removed (RESEARCH Pattern 6 / A5). This keeps the debug fix's code and tests untouched.
- `m` is the Docs sub-tab key.
- No title strings changed: the breadcrumb stays `Archive > v1.2`.

## Inferred decisions (for audit)

1. **`DetailSubView::Archive` kept as Docs › Milestones** instead of being removed. This follows RESEARCH Pattern 6 and deviates from the agent-inferred D-B09. It was already marked `[INFERRED — audit]` in the plan.
2. **`m` as the switch key and the strip wording** `[Files] │ Milestones   m switch`. The active entry is cyan, bold and reversed; `m switch` is dim. The plan marked this `[INFERRED — audit]`.
3. **The strip is drawn before the cache lookup** in both Docs renders, so even the no-cache `Loading...` state shows which sub-tab is active.
4. **README key table.** The stale `Tab` / `S-Tab` "Switch detail tabs" row (Tab is the tmux/session switch, not the tab switch) was replaced with `1`-`8` / `←` `→` plus an `m` row. The plan listed only the Features and highlights bullets. I inferred that the key table is a tab-index consumer under D-B05.
5. **Folded todo resolution text.** It records that `PhaseMarker::decide` treats disk `Executed` or later as done, which goes further than the todo's own narrow "fix 3". I verified this against `src/state_reader/mod.rs` rather than restating the plan's wording. The todo's data and process fixes (1 and 2) are noted as outside this resolution.
6. **Optional 80-column Roadmap footer: not done.** The footer is still 91 cols flag-on and 87 flag-off (`[1-8/D]` is the same length as `[1-9/D]`), so `[?]help` is clipped at 80. Getting under 80 would mean dropping or renaming Roadmap hints (for example `[e]nqueue` or `[Space] fold`). That is a copy and scope change, not a natural part of the tab consolidation. It is left for a follow-up.
7. **Todo `completed:` date is 2026-09-24**, which is the UTC date of execution.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The Roadmap shipped-row `Enter` was re-routed in Task 1, not Task 2**
- **Found during:** Task 1
- **Issue:** Once `tab_index(&Archive)` became 7, 24-05's `switch_to_tab(tab_index(&Archive))` resolved to Docs › Files. That broke `roadmap_enter_on_the_shipped_row_opens_the_archive_view`, which is in Task 1's own verify (`ui::screens::detail::tests` with no FAILED).
- **Fix:** The branch now calls `switch_to_sub_view(.., DetailSubView::Archive, ..)` in the Task 1 commit. Task 2 still added the App-level test and the `[Milestones]` assertion. Because of this, those two Task 2 tests were green at RED time; the RED gate rests on the other six target tests.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `2d89a22`

**2. [Rule 3 - Blocking] The app.rs Driver-index literal test was updated in Task 1**
- **Found during:** Task 1
- **Issue:** `the_driver_sub_view_is_the_last_tab_index_in_both_directions` pinned index 9. It fails as soon as `DRIVER_TAB_INDEX` becomes 8, and the plan assigned the update to Task 2.
- **Fix:** It now asserts 8, and 9 goes to `RoadmapViz`, in the same commit as the constant change, so no commit carries a known-red test other than the TDD RED commit.
- **Files modified:** `src/app.rs`
- **Committed in:** `2d89a22`

---

**Total deviations:** 2 auto-fixed (2 blocking, both task-boundary shifts; the planned end state is unchanged)
**Impact on plan:** None on scope. Both changes moved planned edits earlier so every commit's own verify stays green.

## Issues Encountered

- **The full-suite run hit `Disk quota exceeded` (os error 122) in 6 envelope integration tests** (`envelope_carrier_reach`, `envelope_interior_path`) while copying the 162 MB product binary. `/tmp` is a 16G tmpfs mounted with `usrquota`, and an 11G scratch crate from phase-24 research (`/tmp/gmm-probe`) sits on it. I didn't create that crate, so I left it alone. With `TMPDIR` pointed at `/home` the same suites pass (39/39 and 41/41), and the full suite is clean. This is environmental, not caused by this plan. The operator may want to delete `/tmp/gmm-probe`.

## TDD Gate Compliance

- RED: `28a51cd` `test(24-07): …`. `check tdd-red-evidence` returned `RED_EVIDENCE_OK` (target `nine_and_zero_are_inert_in_the_detail_view` failed on its assertion: "`9` moved the view"). The cargo results were converted line for line to TAP for the checker.
- GREEN: `c17b8e4` `feat(24-07): …`, with all targets passing.
- REFACTOR: none needed.

## Verification

- The five archive regression tests by exact name: `test result: ok. 5 passed` (re-run at HEAD `4e10df7`).
- `cargo test --no-fail-fast` (TMPDIR on /home): 49 `test result:` lines; **2342 passed, 1 failed, 15 ignored**. The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local witness. The baseline was 2333 passed, and this plan added 9 tests.
- `cargo clippy -- -D warnings`: clean.
- `cargo clippy --keep-going --all-targets -- -D warnings`: exactly the 11 pre-existing locations (browser.rs:156/157/158, project_creator.rs:146, envelope_carrier_reach.rs:1711, envelope_config_resolution.rs:2206, envelope_control_carrier.rs:978, envelope_wrapper_class.rs:6127/6213/10795/11232). No new ones.
- Stale-literal sweep: no output. `"9:Arch"` / `"9:Ar"` grep: no output. `KeyCode::Char('9') =>` grep: no output. `tab_index(&DetailSubView::Archive)` appears only in the round-trip test.
- `git diff 2bfb9e8 -- src/app.rs`: no `assert` line inside the five tests changed. The only assert diffs are in the Driver-index test.

## User Setup Required

None. No external service configuration is required.

## Next Phase Readiness

- This is the last plan of phase 24, so the phase is ready for verification (SC-5 complete).
- A follow-up candidate is bringing the Roadmap footer under 80 columns (see inferred decision 6).

## Self-Check: PASSED

- FOUND: src/ui/screens/detail.rs, src/app.rs, src/ui/screens/render_escape_guard.rs, src/ui/screens/help.rs, README.md, docs/ARCHITECTURE.md, .planning/todos/completed/2026-08-22-phase-list-grey-marker-disagrees-with-disk-inferred-stage.md (pending copy gone)
- FOUND commits: 2d89a22, 28a51cd, c17b8e4, 4e10df7

---
*Phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation*
*Completed: 2026-09-24*
