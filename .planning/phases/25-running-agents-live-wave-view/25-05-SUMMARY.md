---
phase: 25-running-agents-live-wave-view
plan: 05
subsystem: detail-view
tags: [agents, detail-view, sub-view, ratatui, escape-guard, D-C13, D-C15, D-C16]

requires:
  - phase: 25-03
    provides: "AgentView, WaveRow, PlanRef, AgentView::summary_forms, waves::derive"
  - phase: 25-04
    provides: "AppContext.agent_views, filled by the AgentsScanned handler"
provides:
  - "DetailSubView::Agents: the Sessions tab's Agents sub-view, sharing tab index 5"
  - "ProjectViewCache.agents_selected"
  - "detail.rs: sessions_sub_tab_strip / sessions_sub_tab_row, render_agents_tab, agent_list_len, agents_list_max, agent_age"
  - "`m` toggles Sessions / Agents; Enter and n do nothing on Agents"
  - "render_escape_guard: 11 sub-views, Agents arrival row, hostile_agent_view, the_dashboard_agent_summary_draws_no_identity"
affects: [25-06, phase-25-verification]

actuals:
  tokens: 16900
  tasks: 3
  commits: 5
plan_head_before: 4db357e42c9510de7b033eec3b6adb768252b06a

tech-stack:
  added: []
  patterns:
    - "Two-sub-view tab strip shared by Docs and Sessions through two_sub_tab_strip"
    - "One line-count helper (agent_list_len) that both the render and the scroll keys clamp against"
    - "List text widths are measured with Span::width per char (fit_cells), never str::len"

key-files:
  created: []
  modified:
    - src/app.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/ARCHITECTURE.md

key-decisions:
  - "The Agents footer adds only `[m] sessions`. The shared footer prefix already shows `[j/k]scroll` on every tab, so a second `[j/k] scroll` would duplicate it [inferred]"
  - "switch_to_sub_view does not reset sessions_selected on arrival, so agents_selected is not reset either. Agents arrival does no work, because the tick already fills agent_views [inferred, following the plan's conditional]"
  - "Docs and Sessions strips share one private two_sub_tab_strip helper. docs_sub_tab_strip's output is unchanged [inferred]"
  - "Rows with no adapter metadata show plan (if any), branch and full path as the label, with `(no agent metadata)` in the fixed tail. When the line is narrow, the path is truncated before the marker or counts [inferred]"
  - "The variable label is truncated with `…` to what remains after the fixed tail, so counts and age stay visible at 80 columns [inferred]"
  - "The wave window shows min(waves, inner.height/3) rows. It starts one wave before the current wave, so the current wave is always visible [inferred]"
  - "agent_age returns `-` when the view has no scanned_at as well as when last_activity is missing [inferred]"
  - "DetailScreen's adjudication_reason now counts eleven sub-views and describes what the Agents sub-view draws [inferred, Rule 2: the claim is quoted in escape-guard failures]"

patterns-established:
  - "A sub-view that shares its tab's index is a separate ALL_SUB_VIEWS probe state with its own DETAIL_TAB_ARRIVAL row"

requirements-completed: [AGENT-06]

coverage:
  - id: D1
    description: "Sessions › Agents sub-view. `m` toggles it through switch_to_sub_view, it shares tab index 5, the strip reads in monochrome, and `m` is inert on every other tab"
    requirement: AGENT-06
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#m_switches_sessions_between_sessions_and_agents"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#m_is_inert_outside_docs_and_sessions"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#every_tab_index_round_trips_through_its_sub_view"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Agents sub-view draws the summary ladder, wave rows (the current wave marked ▸ and bold), agent rows with state words, children indented below their rows, and the worktree-less group"
    requirement: AGENT-06
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_agents_sub_view_draws_the_summary_waves_and_agent_rows"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_current_wave_is_marked_in_text_not_only_colour"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_nested_agent_renders_indented_under_its_worktree"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#equal_rows_order_by_path_and_render_identically"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_three_observed_shapes_render_at_80_by_24"
        status: pass
    human_judgment: false
  - id: D3
    description: "Scrolling clamps at both ends. Enter and n do nothing on Agents. Both footers and the help popup advertise m"
    requirement: AGENT-06
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#agents_selection_clamps_at_both_ends"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#agents_paging_clamps_at_both_ends"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#enter_and_n_do_nothing_on_the_agents_sub_view"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_sessions_and_agents_footers_advertise_m"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#the_m_sub_view_switch_is_documented"
        status: pass
    human_judgment: false
  - id: D4
    description: "Degraded and empty states (D-C16): no-metadata rows, ? counts, zero counts, floor-rounded ages, No running agents, ended-only views, tiny areas"
    requirement: AGENT-06
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_row_without_metadata_shows_branch_path_and_marker"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#zero_counts_render_as_zero_not_question_marks"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#ages_floor_round_and_clamp_future_to_zero"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#no_running_agents_is_the_empty_state"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_tiny_area_renders_without_panicking"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every agent-authored string in the sub-view is escaped (D-C13), and the dashboard summary draws no agent-authored text"
    requirement: AGENT-06
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_dashboard_agent_summary_draws_no_identity"
        status: pass
    human_judgment: false
  - id: D6
    description: "README Features bullet and ARCHITECTURE agents/ entry and data-flow sentence"
    verification:
      - kind: other
        ref: "grep: README.md contains `Agents sub-view` and `w2/11 13run`; docs/ARCHITECTURE.md contains `agents/` and `AgentsScanned`"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 05: Sessions › Agents Sub-view Summary

**Pressing `m` on the detail view's Sessions tab now opens an Agents sub-view. It shows the project's running agents under the dashboard's summary ladder and per-wave rows (the current wave marked `▸ w2` and bold), in a scrollable list: state word, plan or description, agentType, `+commits`, `~dirty` and a floor-rounded age. Children appear indented under their rows, followed by a `Worktree-less (live)` group. There are explicit `No running agents` and `(no agent metadata)` states, and every agent-authored string is escaped and probed by the render escape guard.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-09-26T03:25:05Z
- **Completed:** 2026-09-26T03:38:18Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 7

## Accomplishments

- **Variant and toggle (D-C15).**
  - `DetailSubView::Agents` shares Sessions' index 5, and `sub_view_from_index(5)` still returns `Sessions`.
  - A second guarded `m` arm goes through `switch_to_sub_view`. Its comment cites why `m` is safe there: neither sub-view has a text-input mode (T-25-23).
  - `n` is still guarded on `== DetailSubView::Sessions`.
- **Strip.** `sessions_sub_tab_strip` / `sessions_sub_tab_row` draw `[Sessions] │ Agents   m switch` above both sub-views. They share `two_sub_tab_strip` with the Docs strip: the active label is bracketed plus cyan, bold and reversed.
- **`render_agents_tab`.** The block is titled ` Agents `, with an early return below 3×10. From top to bottom it draws:
  - the widest summary form that fits;
  - the wave rows (`running · done (+N unmerged) · queued · stalled`), in a window that keeps the current wave in sight;
  - the list, with the `> ` highlight and the selection clamped against `agent_list_len`.
  - Label widths are measured with `Span::width`.
- **Keys.** `j/k/Down/Up/PageDown/PageUp` clamp `agents_selected` through `agents_list_max`, and a missing view counts as zero lines. `Enter` on Agents is an explicit `ScreenAction::None` (T-25-24).
- **Footer and help.** The Sessions footer gains `[m] agents` and the Agents footer shows `[m] sessions`. A single help row, `m  Docs / Sessions tab: switch sub-view (detail view)`, covers both tabs.
- **Degraded states (D-C16).**
  - A row with no adapter metadata shows its branch, its path and `(no agent metadata)`.
  - A missing count reads `?`.
  - `agent_age` gives `45s` / `3m` / `2h` / `1d`, `0s` for a future time and `-` when there is no activity.
  - An ended-only view shows its rows under `no active agents`.
- **Escape guard (D-C13, T-25-22).**
  - `ALL_SUB_VIEWS` now has 11 entries and includes an `Agents sub-view` arrival row.
  - `hostile_agent_view` puts the identity into the description, agentType, branch, path, child description and worktree-less description.
  - `the_dashboard_agent_summary_draws_no_identity` pins that the Status-cell summary is numbers and authored words only, and a prose note beside the NormalScreen states records this.
- **Docs.** README gains a Features bullet and `Sessions (with an Agents sub-view)`. ARCHITECTURE gains the `agents/` tree entry and the `Tick → scan_projects_guarded → AgentsScanned → derive → agent_views` flow.

## Task Commits

1. **Task 1 (tracer): Sessions › Agents end to end:** `be45408` (feat)
   - Tracer gate (automated-only verify, end-of-phase mode): `ui::screens::detail` (183 passed) and `render_escape_guard` (15 passed) re-ran green before expanding.
   - Mutation check: rendering `description` raw turned `the_screen_renders_identity_escaped` red at `[Agents sub-view]`. The change was restored immediately.
2. **Task 2: scrolling, footer, help:** `ced5d68` (test, RED), then `590973e` (feat, GREEN)
3. **Task 3: degraded states, ages, shapes, dashboard escape record, docs:** `49dc624` (test, RED), then `56340f8` (feat, GREEN)

## Files Created/Modified

- `src/app.rs`: `DetailSubView::Agents` with its doc
- `src/ui/screens/mod.rs`: `ProjectViewCache.agents_selected`
- `src/ui/screens/detail.rs`:
  - the index arm, the `m` arm, the Enter no-op and the four scroll arms;
  - both render dispatches and the strip into `render_sessions_tab`;
  - `render_agents_tab` and its helpers (`two_sub_tab_strip`, `sessions_sub_tab_*`, `agent_state_word/style`, `fit_cells`, `agents_summary_line`, `wave_line`, `agent_list_len`, `agents_list_max`, `agent_count`, `agent_line`, `agent_list_lines`, `agent_age`, `child_line`);
  - the footer arms and the adjudication reason;
  - 15 new tests plus one renamed test.
- `src/ui/screens/render_escape_guard.rs`: `ALL_SUB_VIEWS[11]`, the arrival row, the label, `hostile_agent_view` in `probe_ctx`, the dashboard test and the prose note
- `src/ui/screens/help.rs`: the shared `m` row and the renamed test
- `README.md`, `docs/ARCHITECTURE.md`: the feature docs

## Test Results

- `cargo test --lib ui::screens`: 404 passed.
- `cargo clippy -- -D warnings`: clean (exit 0, no error or warning lines).
- `cargo test --no-fail-fast`:
  - 52 `test result:` lines: 2469 passed, 1 failed, 15 ignored.
  - The only failure is the known local git-version witness, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
  - 2469 = 2453 + 16 new tests.
- Acceptance greps:
  - `KeyCode::Char('n') if current_view == DetailSubView::Sessions` appears once.
  - The help row appears twice: the row itself and its test.
  - README contains `Agents sub-view` and `w2/11 13run`, and ARCHITECTURE contains `agents/` and `AgentsScanned`.
- **Manual render review at 80×24.** All three shapes plus a scrolled list were printed from a temporary test edit, then reverted.
  - Shape 3 shows the summary, w1–w5 with `▸ w2`, and 11 visible `live 13-NN gsd-executor +2 ~0 0s` rows.
  - A no-metadata row truncates its path with `…` and keeps `(no agent metadata)  +?  ~?  -` visible.

## TDD Gate Compliance

- **Task 2:** a genuine RED commit (`ced5d68`) came first. All 5 target tests failed on behaviour assertions:
  - selection stayed 0;
  - `[m] agents` was missing from the footer;
  - the help row was absent.
  - GREEN followed in `590973e`.
- **Task 3:** RED commit `49dc624` came first. Three target tests failed on behaviour assertions:
  - no path and no `(no agent metadata)` marker;
  - no age column (`45s`);
  - the shapes test failed on `+2  ~0  0s`.
  - The other seven Task 3 tests passed at RED because the Task 1 tracer already rendered those behaviours (nesting, ordering, empty state, tiny areas, the `▸` marker, zero counts, and the summary-forms property). This is the same tracer-first pattern 25-01..25-04 recorded.
  - The tracer's escape coverage was shown to be non-vacuous by the mutation check above.
- **`gsd-tools check tdd-red-evidence`:** not run. It parses node TAP output, and `workflow.tdd_mode` is not enabled. [inferred: proceeded on cargo's output]

## Decisions Made

See `key-decisions` in the frontmatter. Every entry is marked [inferred] for audit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Correctness of a checked claim] DetailScreen's `adjudication_reason` updated**
- **Found during:** Task 1
- **Issue:** the reason text said "ten sub-views" and did not mention agent text. The escape guard quotes this claim in its failure messages, so it had to stay accurate.
- **Fix:** it now reads eleven sub-views and describes the Agents sub-view's agent-authored values.
- **Files modified:** src/ui/screens/detail.rs
- **Commit:** be45408

**2. [Convention] Agents footer omits a duplicate `[j/k] scroll`**
- The plan asked for `[j/k]` + `scroll  ` in the Agents arm, but the shared prefix already draws `[j/k]scroll` on every non-Driver tab. The test asserts that `[j/k]` is present.
- **Commit:** 590973e

**3. [Test harness] `line_with` name collision**
- An existing test helper already had that name, so the new helper is `agent_line_with`. Production code was not affected.
- **Commit:** 49dc624

---

**Total deviations:** 1 correctness fix to a checked claim, 1 footer convention choice, 1 test-harness rename. **Impact:** none on the `<interfaces>` contract.

## Issues Encountered

None.

## Known Stubs

None. The README line mentions the `~5/12 fixed` fixer estimate, which 25-06 implements (D-C12), as the plan's coupling note records.

## Threat Flags

None. No new surface. The threat mitigations are covered as follows:
- **T-25-22:** covered by the Agents probe state, the mutation check and `the_dashboard_agent_summary_draws_no_identity`.
- **T-25-23:** covered by the guarded `m` arm and the comment citing why it is safe.
- **T-25-24:** covered by the `n` guard and the Enter no-op, pinned by `enter_and_n_do_nothing_on_the_agents_sub_view`.

## User Setup Required

None.

## Next Phase Readiness

- **25-06:** can add `AgentView.fixers` and its fixer summary forms. This plan builds views only with `..Default::default()` and renders `summary_forms()` generically, so the fixer forms will appear in the Agents summary line with no change here.

## Self-Check: PASSED

- All seven modified files exist.
- Commits be45408, ced5d68, 590973e, 49dc624 and 56340f8 are present.
- `git rev-list --count 4db357e..HEAD` = 5.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*
