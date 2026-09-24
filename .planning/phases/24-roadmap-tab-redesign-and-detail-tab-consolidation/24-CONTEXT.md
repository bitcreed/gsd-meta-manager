# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation - Context

**Gathered:** 2026-09-23
**Status:** Ready for planning
**Mode:** discuss-phase `--auto`, unattended. Every decision under "Locked decisions (user-approved)" was supplied by the user and approved before this session; they are recorded verbatim-in-substance and are NOT open for re-litigation. Items tagged **[INFERRED — audit]** were decided by the agent from the codebase and should be reviewed.

<domain>
## Phase Boundary

Two coupled TUI changes to the per-project detail view, with no dependency on the v2.0 orchestration phases (15-23):

1. **Roadmap tab redesign** — replace the current graph layout (quick 260923-md1, `src/ui/roadmap_graph.rs`) with a master/detail view: a phase list with a narrow git-log-style lane column on the left and a detail pane for the selected phase on the right (stacked below at narrow widths).
2. **Detail-tab consolidation** — ten tabs (plus Driver) become eight (plus Driver): the old PhaseList tab is removed (Roadmap supersedes it), Pipeline is renamed "Phases" and moved to position 2, and Archive is folded into Docs as a "Milestones" sub-tab.

Out of scope: mouse click-to-select (deferred), any change to Phase 21 or other orchestration phases, any horizontal-scroll mechanism (the list design removes the need).

</domain>

<decisions>
## Implementation Decisions

### A. Roadmap tab — master/detail (LOCKED, user-approved)

- **D-A01: Left pane is a phase list** with a narrow git-log-style graph column on the left. One row per phase: status glyph, lane lines, phase number, full name (truncated with `…`), plans done/total, wave `W<n>`.
- **D-A02: Right pane is a detail pane** for the selected phase: name, milestone, wave, status, a **Goal** paragraph, **Needs** (deps, each with its status glyph), **Unblocks**, and **Parallel** (other phases in the same wave).
- **D-A03: Goal parsing is new** — parse `**Goal**:` in `src/state_reader/roadmap_md.rs`. **[INFERRED — audit]** Also accept the `**Goal:**` form (colon inside the bold), because `gsd phase.add` writes that form (see this repo's own ROADMAP history) while hand-written phases use `**Goal**:`.
- **D-A04: Status glyphs:**

  | Glyph | Meaning | Colour |
  |---|---|---|
  | ● | done | dim |
  | ◉ | active | yellow, bold |
  | ○ | ready (not done and all deps done) | green |
  | ◌ | blocked | default |

- **D-A05: Selection markers** — the selected row is reverse video with `▶`. Deps of the selection get `↑` (cyan); phases it unblocks get `↓` (magenta).
- **D-A06: "Start now" line** at the top lists ready and active phases, with `║` meaning they can run in parallel.
- **D-A07: Milestones** appear exactly once each, as a foldable header band (`▾ name done/total ━━━`). Shipped milestones start collapsed into one row (`▸ v1.0 … v1.4  N milestones · M phases shipped`).
- **D-A08: Graph rules:**
  - Transitive reduction before layout. An implied dep is shown as a dim `·` marker, never as a duplicated node row.
  - Never place an unrelated node under a lane; give it a new lane.
  - Wave = longest dep path + 1.
  - No node is ever rendered twice; no milestone label repeats per row.
- **D-A09: Width** — side-by-side at ≥ ~100 cols; the detail pane stacks below the list when narrower. 80-col support is required. — **Reversibility:** reversible.
- **D-A10: Keys** — `j`/`k` and `↑`/`↓` move the selection; `g`/`G` top/bottom; `h` jumps to a dep (repeat to cycle); `l` jumps to a phase this one unblocks; `[`/`]` step through phases in the same wave; `Space` folds/unfolds a milestone; `Enter` opens the selected phase in the Phases tab (D-B03); `v` keeps toggling the old box view (`src/ui/roadmap_widget.rs`); `←`/`→` still switch tabs.
- **D-A11: Mouse click-to-select is OUT of scope** — deferred to a later follow-up.
- **D-A12: Parser fix in scope** — the parser must also recognise `#### Build phase N` style headings. **[INFERRED — audit]** The brief placed ttbook at `/home/blk/projects/flutter/ttbook` using them for phases 8–18; the repo actually lives at `/home/blk/projects/python/ttbook`, where phases 8–13 use ordinary `### Phase N:` headings and phases **14–18** use `#### Build phase N (Milestone M): Title` under `### 📋 Milestone M "..." (planned)` headers. The parser must handle the `(Milestone M)` parenthetical between the number and the colon, and those phases belong to the milestone named by the enclosing `### 📋 Milestone` header.
- **D-A13: No separate horizontal scrolling** — the list design covers the horizontal-scroll problem.
- **D-A14: The redesign must eliminate these current bugs:**
  - duplicated source nodes on long-edge rows (daily-vow `20` twice; sentriq `9 ─► 11` dim row);
  - the milestone label repeated on every row;
  - no selection cursor (the current highlight only marks the current phase);
  - no explanation when a node has no deps (sentriq phase 12 says "Nothing in this milestone"); the detail pane must say e.g. "nothing declared" and, under Parallel, that it can run any time (see MOCKUPS.md mockup C).
- **D-A15: Keep, rewrite** — keep `roadmap_graph.rs`'s parsing, cycle-breaking and wave logic; rewrite the layout. — **Reversibility:** reversible.

### B. Detail-tab consolidation (LOCKED, user-approved)

- **D-B01: New tab order** — `1:Roadmap · 2:Phases · 3:Backlog · 4:Git · 5:Queue · 6:Sess · 7:Cfg · 8:Docs`, plus the Driver tab (currently `DRIVER_TAB_INDEX`, reached via `Shift+D` and `←`/`→`). — **Reversibility:** costly — undo touches every tab-index consumer (label arrays, `tab_index`/`sub_view_from_index`, digit-key handlers, round-trip tests, help text, escape-guard tests, README).
- **D-B02: Remove the old 1:Phases (PhaseList) tab.** The Roadmap supersedes it. Move its header (Path / Status / Milestone, plus the unreadable-state and recovered-state lines) to the top of the Roadmap tab. Audit PhaseList for anything else unique and preserve it (see D-B08 for the audit result).
- **D-B03: Rename 5:Pipe (Pipeline) to "Phases"** and put it at position 2. Roadmap and Phases share the selected phase: `Enter` on a Roadmap row opens that phase in Phases.
- **D-B04: Fold 8:Arch (Archive) into 0:Docs (Browse) as a sub-tab.** Docs gets sub-tabs "Files | Milestones"; Milestones is the existing archive drill-down. In the Roadmap, `Enter` on the collapsed shipped-milestones row jumps to Docs › Milestones.
- **D-B05: Update every tab-index/number dependent:** the `tab_index` / `sub_view_from_index` round-trip tests; help text in `src/ui/screens/help.rs`; `render_escape_guard` tests; README (it lists tabs — `README.md:47-48` "10-tab detail view: Phases, Roadmap (ASCII DAG), Backlog, Git History, Pipeline, Queue, Sessions, Archive, Config, Docs"); persisted per-project view state (see D-B09).
- **D-B06: Sequencing** — the Archive→Docs move must land AFTER the debug fix for the Archive "Loading..." bug (a concurrent `/gsd-debug` session is committing it to master; see `.planning/debug/archive-milestone-view-loading.md`). Plans must sequence that work **last** and **re-read `src/ui/screens/detail.rs` at execution time** rather than trusting line numbers captured during planning.

### Agent-inferred detail (audit these)

- **D-B07 [INFERRED — audit]: Default tab becomes Roadmap.** `DetailSubView::PhaseList` is `#[default]` in `src/app.rs:16-19` and is the fallback in `sub_view_from_index` for out-of-range indices and for a Driver index with the experimental flag off. With PhaseList removed, `RoadmapViz` takes all three roles.
- **D-B08 [INFERRED — audit]: PhaseList audit — content unique to it that must be preserved.** Besides the header named in D-B02, `render_phase_list` (`detail.rs` ~3607-3790) is the only place that renders:
  - the **Paused** line (`state.paused` + `state.pause_context`, or "Paused (HANDOFF file present)") → move into the Roadmap header block;
  - the **change-tracker banner** (`ctx.change_tracker.latest_change(alias)`, "[ description -- elapsed ]") → move into the Roadmap header block;
  - the per-phase **disk-inferred `[stage]` badge** (e.g. `[Executing 16/18]`) → show in the detail pane's status line for the selected phase;
  - "No state data available for this project." / "No roadmap data available" empty states → Roadmap must render equivalent explanatory empty states.
  - The `Backlog: N items` and `Queued:` summaries duplicate the Backlog and Queue tabs and may be dropped.
- **D-B09 [INFERRED — audit]: Persisted view state is in-memory only.** `AppContext.detail_sub_view_per_project: HashMap<String, DetailSubView>` (`src/ui/screens/mod.rs:1247`) stores the enum, not an index, and is not written to config. No on-disk migration is needed; removing the `PhaseList` and `Archive` variants only requires updating the code paths that construct/match them (`app.rs`, `delete_confirm.rs`, `normal.rs`, `driver.rs`, `driver_confirm.rs`, `executor/mod.rs`, `render_escape_guard.rs`).
- **D-B10 [INFERRED — audit]: Digit keys `9` and `0` become unbound** in the detail view (today `detail.rs` ~2220-2229 maps `1`-`0` to indices 0-9). They must be no-ops, not stale jumps. Driver stays at the end of the label arrays with `Shift+D`; `visible_tab_count` keeps the experimental-flag gating (`TAB_COUNT` 11 → 9, `DRIVER_TAB_INDEX` 10 → 8).
- **D-B11 [INFERRED — audit]: Compact labels** follow the same order with two-letter mnemonics (e.g. `1:Rd 2:Ph 3:Bk 4:Gt 5:Qu 6:Ss 7:Cf 8:Dc D:Dr`); the tab-bar width constants derived from the label render must be recomputed, not hand-edited.
- **D-B12 [INFERRED — audit]: Status-glyph source of truth.** The ●/◉/○/◌ decision must derive from the same `PhaseMarker` / disk-inferred stage logic the rest of the UI uses (`state.phase_marker(phase)`), not from the ROADMAP checkbox alone — see folded todo below.

### Claude's Discretion

- Internal module structure for the new list/detail layout (e.g. splitting `roadmap_graph.rs` into model vs layout vs widget).
- Exact lane-column width budget and truncation rules, as long as 80 columns works and MOCKUPS.md's shapes are honoured.
- How the Docs sub-tab switch is keyed (the brief fixes only the "Files | Milestones" naming).
- Plan split and wave structure, subject to D-B06 (Archive→Docs last).

### Folded Todos

- **`2026-08-22-phase-list-grey-marker-disagrees-with-disk-inferred-stage.md`** — "Phases panel marker/grey disagrees with the disk-inferred stage". The panel it describes (PhaseList) is being removed; the Roadmap's status glyphs and dimming must not reproduce the disagreement (D-B12). Folded as a constraint, not as extra scope. **[INFERRED — audit]**

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design
- `.planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/MOCKUPS.md` — the three user-verified mockups (A: invented `bookly` data side-by-side; B: daily-vow with collapsed shipped milestones and an implied dep; C: sentriq at 80 cols, stacked detail pane). The mockups show the old tab bar; D-B01 supersedes it.
- `.planning/ROADMAP.md` § "Phase 24" — goal and success criteria.

### Code (current behaviour to replace or preserve)
- `src/ui/roadmap_graph.rs` — current graph from quick 260923-md1; keep parsing, cycle-breaking, wave logic; rewrite layout.
- `src/ui/roadmap_widget.rs` — old box view, still reachable with `v`.
- `src/state_reader/roadmap_md.rs` — ROADMAP parser (`parse_roadmap_phases` ~156, `parse_depends_on` ~133); add Goal and `#### Build phase N` support.
- `src/ui/screens/detail.rs` — `TAB_COUNT`/labels/`DRIVER_TAB_INDEX` ~355-395; `tab_index` ~782; `sub_view_from_index` ~807; digit keys ~2220; ←/→ switching ~2244; `v` toggle ~3397; `render_phase_list` ~3607; `render_roadmap` ~3794; `render_pipeline_tab` ~4300; `render_archive_tab` ~4618; `render_browser_tab` ~4803. **Line numbers drift — re-read at execution time.**
- `src/app.rs:16` — `DetailSubView` enum.
- `src/ui/screens/mod.rs:1247` — `detail_sub_view_per_project`.
- `src/ui/screens/help.rs` — help text listing tabs.
- `src/ui/screens/render_escape_guard.rs` — `states_over_sub_views` iterates every sub-view.
- `README.md:47-48` — tab list.
- `.planning/debug/archive-milestone-view-loading.md` — the concurrent Archive "Loading..." fix that must land before D-B04 work.
- `.planning/todos/pending/2026-08-22-phase-list-grey-marker-disagrees-with-disk-inferred-stage.md` — folded todo.

### Real test data
- `/home/blk/projects/flutter/daily-vow/.planning/ROADMAP.md` — duplicated-`20` bug; implied dep 20 via 21.
- `/home/blk/projects/flutter/sentriq/.planning/ROADMAP.md` — `9 ─► 11` dim-row bug; phase 12 no-deps explanation.
- `/home/blk/projects/python/ttbook/.planning/ROADMAP.md` — `#### Build phase N (Milestone M):` headings (phases 14-18).

### Codebase maps
- `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/CONVENTIONS.md`, `.planning/codebase/TESTING.md`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `roadmap_graph.rs` dependency resolution (`resolve_deps` ~135), cycle-breaking (`break_cycles` ~182, `cycle_note` ~239) and wave assignment (`longest_path_layers` ~266, Kahn's order — already "longest dep path") — keep (D-A15). `width_of` (~118) and `esc` (~113) are the width/escape helpers.
- `state.phase_marker(phase)` / `PhaseMarker` — shared done/current/future decision; basis for D-B12.
- `crate::state_reader::phase_plan_counts` — plans done/total per phase (used by PhaseList today).
- `unreadable_state_line` / `recovered_state_line` helpers in `detail.rs` — reuse for the Roadmap header (D-B02).
- The existing archive drill-down (`render_archive_tab` + its key handling) becomes Docs › Milestones unchanged in behaviour.

### Established Patterns
- `visible_tab_count(experimental)` is the single bound for tab navigation; Driver exists only with `AppContext.experimental` (quick 260917-fko). Keep that gating.
- Tab labels have full and compact variants and a derived-from-render width constant.
- Every sub-view is exercised by `render_escape_guard` tests; a removed variant must be removed there, a new sub-tab state must be added.

### Integration Points
- `DetailSubView` is matched in `app.rs`, `ui/screens/{mod,normal,delete_confirm,detail,driver,driver_confirm,render_escape_guard}.rs` and `executor/mod.rs`.
- Roadmap → Phases handoff needs a shared "selected phase" in per-project view state so `Enter` can open the Phases (Pipeline) tab on that phase.
- Roadmap → Docs › Milestones handoff needs Docs to accept an initial sub-tab.

</code_context>

<specifics>
## Specific Ideas

- Follow MOCKUPS.md closely: header line (`alias · milestone · phase N state · k of n phases done`), "Start now" line, `lanes # Phase plans wave` column header, milestone bands with `━` fill, detail-pane footer hint (`⏎ open in Phases   h/l follow edge`).
- Mockup B shows an implied dep in the detail pane as `(implied via 21)` under Needs, and `·` in the list's marker column.
- Mockup C shows the 80-col stacked layout and the no-deps wording: "Needs nothing declared", "Parallel … (no edge either way; can run any time)".

</specifics>

<deferred>
## Deferred Ideas

- **Mouse click-to-select** in the Roadmap list — later follow-up (D-A11).

### Reviewed Todos (not folded)
The todo matcher scored nine todos ≥ 0.4 on generic keywords; auto-mode would fold them all, but that would be scope creep. Only the phase-list marker todo was folded. **[INFERRED — audit]**
- `2026-07-29-driver-tab-layout-at-medium-terminal-heights.md` — Driver tab layout; unrelated.
- `2026-07-29-invalidate-browser-cache-after-editor-exit.md` — Docs file cache; unrelated to tab consolidation.
- `2026-08-19-verify-work-gate-policy-configurable.md` — belongs to Phase 23.
- `2026-09-22-codex-runtime-remainder-after-mvp.md` — runtime work; unrelated.
- `2026-09-23-add-a-global-settings-editor-with-unambiguous-scope.md` — Cfg tab feature; separate phase.
- `2026-08-18-surface-destructive-confirmations-in-a-modal-popup-with-yes.md` — unrelated.
- `2026-09-15-make-queue-input-truly-multi-line.md` — unrelated.
- `2026-07-29-badge-glyph-display-width-alignment.md` — related in spirit (glyph widths), not folded; the new glyphs ●◉○◌ are all single-width in common terminals, but planners should measure widths with the existing helper (`width_of` in `src/ui/roadmap_graph.rs`, or ratatui `Span::width`), never `str::len`.

</deferred>

---

*Phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation*
*Context gathered: 2026-09-23*
