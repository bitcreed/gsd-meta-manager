# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation - Research

**Researched:** 2026-09-23
**Domain:** Rust TUI (ratatui 0.30) — ROADMAP.md parsing, DAG layout (transitive reduction + git-log lanes), master/detail widget, tab-index refactor
**Confidence:** HIGH (codebase read this session; algorithm prototyped against all three mockups; defects reproduced on the real fixtures; test/lint baseline measured)

**Tree state at research time:** `master` = `46d7e82` ("docs(debug): resolve archive-milestone-view-loading session"). **The concurrent Archive "Loading..." debug fix has LANDED** (`3c0e38f` key the archive cache by alias then milestone; `581aa7d` reload the open archive in place; `46d7e82` resolve session). Working tree clean except untracked `.gsd/`, `.planning/state.json`. All `file:line` below are against `46d7e82` unless marked otherwise. **Line numbers drift — every executor must re-read before editing** (D-B06).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### A. Roadmap tab — master/detail (LOCKED, user-approved)

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

#### B. Detail-tab consolidation (LOCKED, user-approved)

- **D-B01: New tab order** — `1:Roadmap · 2:Phases · 3:Backlog · 4:Git · 5:Queue · 6:Sess · 7:Cfg · 8:Docs`, plus the Driver tab (currently `DRIVER_TAB_INDEX`, reached via `Shift+D` and `←`/`→`). — **Reversibility:** costly — undo touches every tab-index consumer (label arrays, `tab_index`/`sub_view_from_index`, digit-key handlers, round-trip tests, help text, escape-guard tests, README).
- **D-B02: Remove the old 1:Phases (PhaseList) tab.** The Roadmap supersedes it. Move its header (Path / Status / Milestone, plus the unreadable-state and recovered-state lines) to the top of the Roadmap tab. Audit PhaseList for anything else unique and preserve it (see D-B08 for the audit result).
- **D-B03: Rename 5:Pipe (Pipeline) to "Phases"** and put it at position 2. Roadmap and Phases share the selected phase: `Enter` on a Roadmap row opens that phase in Phases.
- **D-B04: Fold 8:Arch (Archive) into 0:Docs (Browse) as a sub-tab.** Docs gets sub-tabs "Files | Milestones"; Milestones is the existing archive drill-down. In the Roadmap, `Enter` on the collapsed shipped-milestones row jumps to Docs › Milestones.
- **D-B05: Update every tab-index/number dependent:** the `tab_index` / `sub_view_from_index` round-trip tests; help text in `src/ui/screens/help.rs`; `render_escape_guard` tests; README (it lists tabs — `README.md:47-48` "10-tab detail view: Phases, Roadmap (ASCII DAG), Backlog, Git History, Pipeline, Queue, Sessions, Archive, Config, Docs"); persisted per-project view state (see D-B09).
- **D-B06: Sequencing** — the Archive→Docs move must land AFTER the debug fix for the Archive "Loading..." bug (a concurrent `/gsd-debug` session is committing it to master; see `.planning/debug/archive-milestone-view-loading.md`). Plans must sequence that work **last** and **re-read `src/ui/screens/detail.rs` at execution time** rather than trusting line numbers captured during planning.

#### Agent-inferred detail (audit these)

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

### Deferred Ideas (OUT OF SCOPE)

- **Mouse click-to-select** in the Roadmap list — later follow-up (D-A11).
- Reviewed-but-not-folded todos: driver-tab layout at medium heights; browser cache invalidation after editor exit; verify-work gate policy (Phase 23); codex runtime remainder; global settings editor; destructive-confirmation modal; multi-line queue input; badge glyph display-width alignment (related in spirit only — measure glyph widths with `width_of` / `Span::width`, never `str::len`).
</user_constraints>

<phase_requirements>
## Phase Requirements

No REQ-IDs are assigned (ROADMAP.md: "Requirements: TBD (locked decisions in `24-CONTEXT.md`)"). The five ROADMAP success criteria are the acceptance contract; they map to research findings as follows.

| ID | Description (ROADMAP.md §Phase 24 success criteria) | Research Support |
|----|-------------|------------------|
| SC-1 | Every phase once; each milestone label once (foldable band); implied deps after transitive reduction as a dim marker — daily-vow `20`-twice and sentriq `9 ─► 11` gone | §Real Fixtures (defects reproduced), §Pattern 1 (transitive reduction), §Pattern 2 (lanes), prototype output matching mockups A/B/C |
| SC-2 | Cursor over every phase (j/k, g/G, h/l, [/], Space); detail pane Goal/Needs/Unblocks/Parallel; no-deps explanation | §Pattern 4 (selection model + key arms, collision check), §Pattern 3 (Goal parse), §Detail-pane content spec |
| SC-3 | 80 cols stacked, ~100+ side-by-side, no horizontal scroll | §Pattern 5 (width breakpoints + row column budget), glyph widths measured |
| SC-4 | `#### Build phase N` headings (ttbook) parse as phases | §Pattern 3b (display-only planned-phase list; GSD itself does NOT treat them as phases — verified) |
| SC-5 | PhaseList gone (unique content preserved), Pipeline → Phases at 2 sharing selection, Archive → Docs "Milestones" sub-tab, every tab-index consumer agrees | §Tab-Index Consumer Inventory, §Pattern 6 (keep `Archive` as the Docs›Milestones sub-view), §File Ownership & Wave Plan |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading must not require running Claude/GSD** — parse files only. Non-intrusive: never interfere with running GSD instances. Portability: not hardcoded to one user's setup (so fixtures/tests must not read `/home/blk/...` at test time).
- **Stack is fixed:** Rust 1.88+, ratatui 0.30, crossterm 0.29, tokio. No new crates are needed for this phase (see Standard Stack).
- **Logging goes to a file** (tracing), never stdout — ratatui owns the terminal.
- **Never block the render loop**: render is state-driven; the Roadmap model must be computed from in-memory `ProjectState` (no file I/O in `render`).
- **GSD workflow enforcement:** edits go through `/gsd:execute-phase` plans.
- **Release process** is out of scope for this phase (no version bump here).
- **User global rules:** development work runs in subagents; `rtk proxy cargo test --no-fail-fast` for any check that depends on raw output (rtk filters `test result:` lines); never rebase `dev` (docs cite shas); worktree dispatch must pass `isolation: "worktree"` when the project's dispatch-isolation config calls for it (`.planning/config.json` has `"use_worktrees": true`, `"parallelization": true`).
- **House conventions observed in code (not CLAUDE.md, but enforced by tests):** every third-party string reaches a cell through `shown()` / `render_for_terminal` / `Untrusted::shown()` (`render_escape_guard` probe); new `String`/`Option<String>`/`Vec<String>` fields on `ProjectState` or `RoadmapPhase` must be enumerated in `src/driver/untrusted.rs::THIRD_PARTY_STRINGS` (`tests/spawn_seam_guard.rs` diffs it in both directions); new badge-like glyphs are written as `\u{…}` escapes in `&'static str` consts (`normal.rs:65-70`, `detail.rs:460 DRIVER_LIVE_MARKER`); "a comment is not a guard; the test is".

## Summary

The phase is three separable engineering problems plus one mechanical refactor. (1) **Parser**: `parse_roadmap_phases` must keep its current (GSD-conformant) output because the driver router, dashboard frontier and counts consume it; the new capabilities — `**Goal**:`/`**Goal:**` text and ttbook's `#### Build phase N (Milestone M):` placeholder phases — should be added as *additional* parse outputs stored on `ProjectState` without changing `RoadmapPhase`'s shape. This is the single most important design call: GSD's own roadmap parser does **not** match `#### Build phase N` (its heading regex requires `#{2,4}\s*(?:\[..\]\s*)?Phase\s+`), and ttbook's roadmap says in prose that they "are not GSD phases", so putting them into `state.phases` would make the meta-manager's router and frontier diverge from GSD. (2) **Layout engine**: keep `resolve_deps` / `break_cycles` / `cycle_note` / `longest_path_layers`; add a transitive reduction (with an "implied via" witness) and replace the row-placement/painting half with a git-log-style lane assigner over a vertical, milestone-grouped list. A Python prototype of the recommended algorithm reproduces Mockups A, B and C exactly (§Pattern 2). (3) **View**: a new pure widget (list + detail pane, 80-col stacked / ≥100 side-by-side) plus selection state in `ProjectViewCache` (which is `#[derive(Default)]`, so new fields cost zero constructor churn) and new key arms on the Roadmap tab. (4) **Tab consolidation**: 11 → 9 label entries; every consumer is inventoried below with file:line.

Both current defects were reproduced on the real repos with a scratch probe against the committed crate: daily-vow renders a reference row `20 ─────────► 23` (so `20` appears twice) and sentriq renders `9 ────────► 11`; ttbook's build phases 14-18 are not parsed at all and are instead mis-detected as five spurious *milestones* named `Build phase 14`…`Build phase 18` (because their heading contains `(Milestone 3)`). The milestone label is appended to every row (`(v1.5: Closing the Loop)` on each line).

**Primary recommendation:** Wave 1 = parser (roadmap_md.rs + state_reader/mod.rs + fixtures) ∥ pure layout model + widget (roadmap_graph.rs + new roadmap_view.rs); Wave 2 = Roadmap tab wiring in detail.rs (render, keys, selection sharing, escape-guard/help rows); Wave 3 = tab consolidation incl. Archive→Docs (all tab-index consumers in one plan). Keep `DetailSubView::Archive` as the Docs›Milestones *sub-view* (not a tab) rather than deleting it — it preserves the just-landed debug fix and its regression tests unchanged.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Goal / build-phase / milestone parsing | State reader (`src/state_reader/roadmap_md.rs`, `mod.rs::parse_project_state`) | — | File parsing lives in the reader; render must do no I/O |
| Status glyph decision (●◉○◌) | State reader (`PhaseMarker::decide`, `state.phase_marker`) | Layout model (ready vs blocked from deps) | D-B12: one source of truth; the model only adds "all deps done?" |
| Transitive reduction, waves, lanes, bands, Start-now, Parallel sets | Pure layout model (`src/ui/roadmap_graph.rs`) | — | No ratatui types → text-pinnable unit tests (existing house pattern: `render_text`) |
| List + detail-pane drawing, 80/100-col layout, truncation | Widget (`src/ui/roadmap_view.rs`, new) | ratatui `List`/`Paragraph`/`Block` | Draws only escaped strings from the model; clips to its Rect |
| Cursor, folding, h/l/[/] navigation, Enter handoffs | Screen (`src/ui/screens/detail.rs` key arms) | View state (`ProjectViewCache` in `mod.rs`) | Key handling is the screen's job; state persists per project in the cache |
| Tab order / labels / digits / footer / help | Screen (`detail.rs` tab constants), `help.rs`, README | `app.rs` (`DetailSubView` enum) | The enum is the identity; the index mapping is `tab_index`/`sub_view_from_index` |
| Archive discovery / refresh | App/context (`AppContext::schedule_archive_refresh`, `app.rs` handlers) | — | Already sub-view-agnostic after the debug fix — unaffected by the move |

## Standard Stack

### Core (all already in `Cargo.toml`; no additions)
| Library | Version (locked) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.2 `[VERIFIED: Cargo.lock:1911-1912]` | `List`/`ListState`, `Paragraph` + `Wrap`, `Layout`, `Block`, `TestBackend` | Already used for every tab (`detail.rs:19-22` imports `Constraint, Layout, Margin, Rect`, `List, ListItem, ListState, Paragraph, Tabs, Wrap`) |
| unicode-width | 0.2.0 `[VERIFIED: Cargo.lock:2767-2768]` | What ratatui uses to measure `Span::width` | Measured this session: every glyph this phase needs is width 1 (table below) |
| regex | 1.x `[VERIFIED: Cargo.toml [dependencies] regex = "1"]` | Heading / Goal / Depends-on recognisers | Linear-time on untrusted text: "All regex searches in this crate have worst case `O(m * n)` time complexity" `[CITED: docs.rs/regex]` |

### Supporting
| Library | Purpose | When to Use |
|---------|---------|-------------|
| `crate::text::Untrusted` / `render_for_terminal` | Carrier + escape for third-party text | Goal text, milestone names, planned-phase names |
| `crate::state_reader::phase_num::{phase_key, same_phase, PhaseNum}` | Pad-insensitive phase identity (`07` == `7`) | Every dep lookup, goal map key, selection key |

### Glyph widths (measured with unicode-width 0.2.0 this session)
`● U+25CF`, `◉ U+25C9`, `○ U+25CB`, `◌ U+25CC`, `▶ U+25B6`, `↑ U+2191`, `↓ U+2193`, `· U+00B7`, `▾ U+25BE`, `▸ U+25B8`, `━ U+2501`, `║ U+2551`, `⏎ U+23CE`, `├ ┼ ┐ ┘ ┬ ┴ │ ─`, `… U+2026` — **all width=1** `[VERIFIED: scratch probe /tmp/gmm-w, unicode-width =0.2.0]`. (Terminals with East-Asian-ambiguous=wide may still draw some as 2 cells — that is the open todo `2026-07-29-badge-glyph-display-width-alignment.md`, deferred.)

**Installation:** none.

## Package Legitimacy Audit

This phase installs **no** external packages. No registry checks required.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Real Fixtures — shapes and reproduced defects

Probe: a scratch crate depending on a `git archive` of this repo, calling `parse_roadmap_phases`, `roadmap_milestones`, `parse_project_state`, `roadmap_graph::layout_for_state` + `render_text` on each project's real `.planning/` (read-only). `[VERIFIED: /tmp/gmm-probe run this session]`

### daily-vow — `/home/blk/projects/flutter/daily-vow/.planning/ROADMAP.md` (309 lines; repo HEAD `2dd223b`, file last changed `abc11af`; repo is **PRIVATE**)
- Milestones list (`## Milestones`): `- ✅ **v1.0 MVP** -- Phases 1-3 (shipped 2026-03-22)` … `- 🚧 **v1.5 Closing the Loop** -- Phases 18-23 (in progress)`. Separator is `--`.
- Shipped phases 1-17 appear only in a `## Complete Phase History` **table** (not parsed as phases — correct). Only 18-23 are phases.
- Checklist `- [x] **Phase 18: Calibration Foundation** - …`; details `### Phase 18: Calibration Foundation`, `**Goal**: …`, `**Depends on**: Phase 17 (v1.4 complete)`.
- Deps parsed: 18←17 (external), 19←18, 20←19, 21←20, 22←21, **23←[21, 20]** (`**Depends on**: Phase 21 (for real evidence), Phase 20 (for the reserved …)`).
- A spurious milestone `"Requirement Coverage"` is detected from `## Requirement Coverage (v1.5)` (no range, no members).
- STATE.md: `milestone: v1.5`, `milestone_name: Closing the Loop`, `current_phase: 23`.
- **Current render (defect reproduced):**
  ```
  Milestones: ◆ v1.5 Closing the Loop
  18 ─► 19 ─► 20 ─► 21 ─┬─► 22  (v1.5: Closing the Loop)
                        └─► 23  (v1.5: Closing the Loop)
              20 ─────────► 23            <- 20 rendered twice (reference row)
  external deps: 18 ◄ 17
  ▶ P23: Transparency, Reset & History
  ```
  After reduction, `20` is implied via `21` → Mockup B's `·` marker and "(implied via 21)".

### sentriq — `/home/blk/projects/flutter/sentriq/.planning/ROADMAP.md` (248 lines; HEAD `dfc6d2c`, file `49533e9`; **PRIVATE**)
- **No `## Milestones` list.** The active milestone is a bold line in `## Current Milestone`: `**v0.12 — Actuation Routines** (phases 9-12)` — not recognised by `roadmap_milestones`. Detected (spurious) milestones: `"v0.11 Phases"` (from `## v0.11 Phases (4-7) — archived, index only`; the `(4-7)` range is in parentheses so no range is read) and `"Scope Explicitly Excluded from v0.12"`. Neither has member phases.
- Phases 9-12 (checklist + `### Phase N:` details). Deps: 9 `Nothing within v0.12's GSD-tracked phases (…)` → [], 10←9, **11←[10, 9]**, 12 `Nothing in this milestone — …` → [].
- STATE.md: `milestone: v0.12`, `milestone_name: Actuation Routines` (note: `milestone_name` is AFTER `progress:` in this frontmatter), `current_phase: 9`.
- **Current render (defect reproduced):**
  ```
  9 ──► 10 ─► 11
  12
  9 ────────► 11                    <- 9 rendered twice
  ▶ P9: Routine Event Logging (Schema v18)
  ```
  Because no roadmap milestone contains 9-12, the new list needs a **synthetic band** named from STATE.md (`v0.12` + `milestone_name` → "v0.12 Actuation Routines", exactly Mockup C's band). `milestone_name` is parsed by `state_md` (`src/state_reader/state_md.rs:27,641`) but **not** copied onto `ProjectState` today.

### ttbook — `/home/blk/projects/python/ttbook/.planning/ROADMAP.md` (258 lines; HEAD `f420fd5`, file `d54fa75`; **PRIVATE**)
- Milestones list: `- ✅ **v1 Milestone 1: Reassessment and decision records** - Phases 1-7.1 (shipped …)`, `- 🚧 **v2 Core library and central service** - Phases 8-13 (in progress)`, `- 📋 **Milestone 3: Supervised live booking and unattended runs** - build phases 14-15 (planned)`, `…Milestone 4… - build phases 16-17`, `…Milestone 5: Web frontend** - build phase 18`.
- Shipped v1 phases are inside `<details>` as `- [x] Phase 1: WAF and … (6/6 plans)` — **no `**` bold**, so they do not parse as phases (acceptable: collapsed shipped band).
- v2 phases 8-13: checklist + `### Phase N: Title` details with `**Goal**:` and `**Depends on**: Phase 8, Phase 9`. Phase 8: `**Depends on**: Milestone 1 (Phase 7: …)` → [] (parenthetical stripped).
- **Build phases**: under `## Planned milestones (placeholders, not GSD phases)` → `### 📋 Milestone 3 "Supervised live booking and unattended runs" (planned)` → `#### Build phase 14 (Milestone 3): Supervised first live booking`, `**Goal**: …`, **`**Depends on**: Build phases 8-13`**, `Build phase 14`, `Build phases 12, 13`, `Build phases 12, 13, 16`, `Build phase 12`. The roadmap's own prose: "Build phases 14-18 are written as `####` headings and are not GSD phases of milestone v2".
- **Current parse (defect reproduced):** phases = 8..13 only; milestones include five spurious entries `"Build phase 14"` … `"Build phase 18"` (the `#### Build phase 14 (Milestone 3): …` heading matches the milestone detector's `(?i)\bMilestone\s+\d` rule and is not recognised as a phase heading). Also reference rows `8 ────────► 10`, `8 ────────► 11`, `9 ───────────► 12`.
- `parse_depends_on` requires the capitalised keyword `Phase\s+` and rejects plurals by design (`test_depends_on_accepts_every_phase_id_form_and_rejects_a_plural_range`), so **none** of the build-phase dependency lines parse today.

### GSD's own treatment of build-phase headings
GSD's roadmap heading recogniser is `^ {0,3}#{2,4}\s*` + `(?:\[[^\]]{1,200}\]\s*)?Phase\s+` + id `[VERIFIED: ~/.claude/gsd-core/bin/lib/roadmap.cjs:364 and phase-id.cjs:260 "const BASE_ANY_BRACKET_HEADING_PREFIX_SRC = '(?:\\[[^\\]]{1,200}\\]\\s*)?Phase\\s+';"]`. `#### Build phase 14` has `Build ` between the hashes and `phase`, so **GSD does not count build phases**. The meta-manager's router conformance test (`tests/driver_router_conformance.rs`) compares against that runtime.

### Fixture vendoring recommendation
- Directory: `tests/fixtures/roadmaps/` with a `README.md` giving provenance (source repo, commit sha, date, what was changed) — the house convention set by `tests/fixtures/codex/README.md` `[VERIFIED: read this session]`. Read from unit tests with `include_str!("../../tests/fixtures/roadmaps/<name>.md")` (precedent: `src/executor/codex_json.rs:242`, `src/executor/outcome.rs:385`) — never read `/home/blk/...` at test time (portability constraint).
- **Privacy:** this repo is **PUBLIC** on GitHub and published to crates.io with `tests/` included (`Cargo.toml` excludes only `.planning/` and `CLAUDE.md`); all three source repos are **PRIVATE** `[VERIFIED: gh repo view … visibility]`. **Do not vendor the files verbatim.** Vendor *structural excerpts*: the `## Milestones` list, the `## Phases` checklist lines, each phase/build-phase heading, its `**Goal**:` line and its `**Depends on**:` line, and the milestone headings — with sentences that carry product/private detail replaced by neutral text of similar length, while preserving every heading shape, id, separator, parenthetical and dependency line byte-for-byte. Phase names already appear in the public MOCKUPS.md and may be kept. **[INFERRED — needs operator sign-off: a risk acceptance on publishing private-repo content; planner should add a `checkpoint:human-verify` before committing fixtures, or default to the sanitised form.]**
- Assertion approach: two layers — (a) pure `render_text`-style line assertions on the layout model (existing pattern: `roadmap_graph.rs` tests assert `Vec<String>`), (b) a `TestBackend` buffer-to-string render of the whole detail screen (existing helper `render_detail_to_text` at `detail.rs:11631` renders 120×30; add an 80×24 variant) asserting e.g. `text.matches(" 20 ").count() == 1`, exactly one occurrence of each milestone label, presence of `·` on row 20 when 23 is selected, and `nothing declared` / `can run any time` for sentriq 12.

## Architecture Patterns

### System Architecture Diagram

```
 .planning/ROADMAP.md, STATE.md, phases/*   (third-party text, read-only)
            │  (watcher FileChanged → parse_project_state, off the render thread)
            ▼
 ┌─────────────────────────── state_reader ────────────────────────────┐
 │ parse_roadmap_phases ──► state.phases (GSD-conformant; router uses) │
 │ roadmap_milestones   ──► state.milestones (build-phase headings no  │
 │                          longer mis-detected as milestones)         │
 │ NEW parse_phase_goals ──► state.phase_goals  {phase_key → Untrusted}│
 │ NEW parse_planned_build_phases ──► state.planned_phases (display)   │
 │ NEW state.milestone_name (Option<Untrusted>, from STATE.md)         │
 │ disk inference ──► phase_disk_statuses ──► PhaseMarker (D-B12)      │
 └───────────────┬─────────────────────────────────────────────────────┘
                 │ ProjectState (in memory)
                 ▼
 ┌──────── detail.rs adapter (wave 2) ────────┐     ProjectViewCache
 │ ProjectState → Vec<ListNode> + bands input │◄──── roadmap_selected (phase key)
 └───────────────┬────────────────────────────┘      roadmap_folded, cursor-on-band,
                 ▼                                   h/l cycle index
 ┌──────── roadmap_graph.rs (pure model) ───────────────────────────────┐
 │ resolve_deps → break_cycles → longest_path_layers (KEPT)             │
 │ → transitive_reduction (+implied-via) → waves = layer+1              │
 │ → group into bands (shipped summary, milestone bands, synthetic band)│
 │ → assign_lanes (git-log) over list order → rows: Band|Connector|Phase│
 │ → per-phase facts: needs/implied/unblocks/parallel/ready/start-now   │
 └───────────────┬──────────────────────────────────────────────────────┘
                 ▼ RoadmapModel (escaped strings only)
 ┌──────── roadmap_view.rs (widget) ─────────┐
 │ width ≥ 100: [list | detail]; else stacked │──► ratatui Buffer
 │ selection → ▶ + REVERSED; deps ↑ cyan;     │
 │ unblocks ↓ magenta; implied · dim          │
 └────────────────────────────────────────────┘
 Keys (detail.rs): j/k/g/G/h/l/[/]/Space/Enter/v on RoadmapViz → mutate cache → redraw
 Enter(phase) → set shared selection → switch_to_tab(tab_index(&Pipeline))
 Enter(shipped row) → Docs › Milestones (wave 3)
```

### Recommended module structure
```
src/state_reader/roadmap_md.rs   # + parse_phase_goals, parse_planned_build_phases, build-dep grammar,
                                 #   milestone detector excludes build-phase headings (wave 1, plan A)
src/state_reader/mod.rs          # ProjectState: + phase_goals, planned_phases, milestone_name (plan A)
src/ui/roadmap_graph.rs          # model: kept dep/cycle/wave code + reduction + lanes + RoadmapModel;
                                 #   old row-placement/painting/RoadmapGraphWidget removed (plan B)
src/ui/roadmap_view.rs  (NEW)    # widget: list rows, detail pane, 80/100 layouts (plan B)
src/ui/mod.rs                    # + `pub mod roadmap_view;` (plan B; one line)
src/ui/screens/detail.rs         # render_roadmap rewrite, key arms, selection sharing (plan C);
                                 #   tab consolidation (plan D, last)
src/ui/screens/mod.rs            # ProjectViewCache fields (plan C), docs sub-tab field (plan D)
tests/fixtures/roadmaps/         # sanitised excerpts + README provenance (plan A)
```

### Pattern 1: Transitive reduction with an "implied via" witness
**What:** After `break_cycles`, for each node drop any parent that is an ancestor of another parent; remember which parent implied it.
**When:** Before lane assignment and before building Needs lists. Waves are unaffected (longest path is invariant under reduction), so keep computing waves from the unreduced acyclic parents.
**Example (Rust sketch; adapts the file's existing `ParentLists`):**
```rust
// Source: this research's prototype (/tmp/gmm-proto/lanes.py), ported; ParentLists is
// `type ParentLists = Vec<Vec<usize>>;` [VERIFIED: src/ui/roadmap_graph.rs:127]
/// (reduced parents, implied[u] = [(implied_parent, via_parent)])
fn transitive_reduction(parents: &ParentLists, layer: &[usize])
    -> (ParentLists, Vec<Vec<(usize, usize)>>)
{
    let n = parents.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&u| layer[u]);            // acyclic: parents have lower layers
    let mut anc: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for &u in &order {
        let mut s = HashSet::new();
        for &p in &parents[u] { s.insert(p); s.extend(anc[p].iter().copied()); }
        anc[u] = s;
    }
    let mut reduced = vec![Vec::new(); n];
    let mut implied = vec![Vec::new(); n];
    for u in 0..n {
        for &p in &parents[u] {
            // first OTHER declared parent whose ancestry already contains p
            match parents[u].iter().copied().find(|&q| q != p && anc[q].contains(&p)) {
                Some(via) => implied[u].push((p, via)),
                None => reduced[u].push(p),
            }
        }
    }
    (reduced, implied)
}
```
Prototype results `[VERIFIED: /tmp/gmm-proto/lanes.py run]`: daily-vow `23: [(20 via 21)]`; sentriq `11: [(9 via 10)]`; ttbook `10/11: [(8 via 9)]`, `12: [(9 via 10)]`. n is ≤ ~100 phases, so `HashSet` per node is fine (O(n·e)).

### Pattern 2: Git-log lane assignment over the vertical list
**Rules (prototype-validated against all three mockups):**
1. Rows are the visible list in order: `Band` rows and `Phase` rows (milestone-grouped, roadmap order inside a group). Edges drawn only from an earlier row to a later row using **reduced** parents; a dep that appears later (backward in list order) or externally is not drawn as a lane (it still appears under Needs).
2. `active: Vec<Option<target_node>>` — one lane per pending edge.
3. For phase `u`: `inc` = lanes whose target is `u`. If `inc` non-empty → `u` sits on `inc[0]`; if `inc.len() > 1` emit a **merge connector row** (`├─┘`, `├─┴─┘`, `┼` where an unrelated active lane is crossed) and free the others. If `inc` is empty (a root) → the lowest lane that is free **and is not the lane the previous phase row occupied** ("never place an unrelated node under a lane"); a Band row resets "previous".
4. After `u`: children = reduced children later in list order, sorted by row. First child continues `u`'s lane; each further child takes the lowest free lane → emit a **fork connector row** (`├─┐`, `├─┬─┐`, `├─┼─┐` crossing a busy lane). No children → lane freed.
5. A child on the very next row in the same lane needs no connector (vertical adjacency = edge). Band rows draw `│` for every active lane (Mockup A `│ │   M4 Support chat`).
6. Folding: compute lanes on the unfolded list, then drop hidden rows (and their connector rows). Deterministic; lanes that start/end inside a folded band simply appear/disappear at the band row.

Prototype output (lane cells shown as `o`; compare to MOCKUPS.md):
```
=== A bookly               === B daily-vow      === C sentriq       === no deps at all
o         8                o         18         o         9         o         1
o         9                o         19         o         10          o       2
├─┐                        o         20         o         11        o         3
o │       10               o         21           o       12          o       4
│ o       11               ├─┐
├─┘                        o │       22
o         12                 o       23
├─┐
o │       13
├─┼─┐
o │ │     14
o │ │     15
  │ │     [M4]
  │ o     16
  │ o     17
  │       [M5]
  o       18
```
A, B and C match MOCKUPS.md row-for-row `[VERIFIED: /tmp/gmm-proto/lanes.py]`. The "no deps" case zig-zags instead of stacking (stacking would falsely read as a chain). ttbook (once build-phase deps parse) produces Mockup A's shape exactly.

**Lane cap (discretion):** 2 cells per lane; cap visible lanes at 4 below 100 cols and 6 at ≥100; lanes beyond the cap collapse into one overflow column (`┆` or `…`). Keeps the name column ≥ 40 cells at 80 cols.

### Pattern 3: Goal parsing (D-A03)
- Recogniser: `(?i)^\s*\*\*Goal(?:\*\*\s*:|:\*\*)\s*(.*)$` — matches `**Goal**: text` and `**Goal:** text`. `[ASSUMED — regex authored here; must be unit-tested against both forms]`. Scope: the first Goal line inside a phase entry (same "first wins inside the entry, stop at next header" scan `parse_roadmap_phases` already does for `depends_re` — `roadmap_md.rs:175,239-247` `[VERIFIED: read]`). Multi-line goals: take the single line (all three fixtures put the goal on one line) `[VERIFIED: fixture greps above]`.
- **Storage (recommended): `ProjectState.phase_goals: HashMap<String /*phase_key*/, crate::text::Untrusted>`**, filled by a new `roadmap_md::parse_phase_goals(content)` called from `parse_project_state` (`src/state_reader/mod.rs:598-599` is where `parse_roadmap_phases`/`roadmap_milestones` are called `[VERIFIED: read]`). Rationale:
  - Adding a field to `RoadmapPhase` breaks **~20 struct literals in 10 files** (`src/app.rs` ×4, `browser.rs` ×3, `change_tracker.rs`, `driver/router.rs`, `ui/mod.rs`, `ui/roadmap_widget.rs`, `ui/screens/detail.rs` ×2, `render_escape_guard.rs`, `tests/driver_router_table.rs`, `tests/state_reader_test.rs`) because `RoadmapPhase` derives only `Debug, Clone, PartialEq` (`roadmap_md.rs:6`) — that would drag the parser plan into the hot shared files.
  - `ProjectState` derives `Default` (`state_reader/mod.rs:42`) and **every** `ProjectState { … }` literal in `src/` and `tests/` ends in a `..` struct update `[VERIFIED: scripted scan this session — zero full literals]`, so new `ProjectState` fields are zero-churn.
  - A `String`/`Option<String>`/`Vec<String>` field on `ProjectState` or `RoadmapPhase` would have to be added to `THIRD_PARTY_STRINGS` (`src/driver/untrusted.rs:136-203`) and would break `the_enumerated_prose_set_is_exactly_the_six_free_text_fields` (`untrusted.rs:439`). The census counts only `String | Option<String> | Vec<String>` and excludes `HashMap<String, T>` (`tests/spawn_seam_guard.rs:723-774`) `[VERIFIED: read]`. Using `Untrusted` values follows the precedent of `RoadmapMilestone.label` ("`ProjectState` gains no free-string field (`THIRD_PARTY_STRINGS`)", `roadmap_md.rs:486-491`).
  - **[INFERRED — audit]** This keeps goals out of the driver's model seam (they are display-only). If the operator later wants goals in the seam, that is a deliberate widening of the prose set.

### Pattern 3b: Build-phase headings as a display-only list (D-A12 / SC-4)
- Heading recogniser: `(?i)^\s*#{2,4}\s+Build\s+phase\s+(<PHASE_ID>)(?:\s*\([^)]*\))?:\s+(.+?)\s*$` where `PHASE_ID` is `r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?"` `[VERIFIED: roadmap_md.rs:49]`. The existing `heading_re` already tolerates the `(…)` parenthetical: `r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$"` `[VERIFIED: roadmap_md.rs:166-169]` — only the `Build ` prefix and lowercase `phase` are missing.
- **Recommended: new `roadmap_md::parse_planned_build_phases(content) -> Vec<RoadmapPhase>` feeding `ProjectState.planned_phases: Vec<RoadmapPhase>`**; `parse_roadmap_phases` output is unchanged. `Vec<RoadmapPhase>` is census-exempt. The Roadmap list shows `state.phases` followed by `state.planned_phases`; nothing else reads `planned_phases`, so the driver router (`router.rs:836` iterates `state.phases`), the frontier loop (`state_reader/mod.rs:620-665`), `format_phase_display` (`app.rs:466`) and `resolve_active_phase_dir` (`browser.rs:110`) stay GSD-conformant. **[INFERRED — audit]** (alternative — putting them in `state.phases` — would make the frontier advance to "phase 14" when 8-13 execute and let the driver target a phase GSD does not know exists).
- **Build-phase dependency grammar (apply ONLY inside build-phase entries):** accept `Build phase N`, `Build phases A, B[, C]`, `Build phases A-B` (expand the range to the ids that exist in `state.phases ∪ planned`), plus the ordinary `Phase N` form; strip parentheticals first like `parse_depends_on` does. Without this, every build phase shows "nothing declared" and a spurious zig-zag. **[INFERRED — audit]**: the CONTEXT locks heading recognition only; dependency parsing is the minimum needed for SC-1's "each phase once/graph rules" to hold on ttbook. Do **not** widen `parse_depends_on` globally (it would contradict its pinned "a range stays prose" test and GSD conformance).
- `roadmap_milestones` must (a) treat a build-phase heading as a phase heading (so `(Milestone 3)` no longer creates a spurious milestone) and (b) collect build phases into the enclosing `### 📋 Milestone 3 …` heading scope. Its private `phase_heading` regex at `roadmap_md.rs:639-643` needs the same `(?:Build\s+)?[Pp]hase` widening **[VERIFIED: read]**. Its range regex is already case-insensitive (`(?i)\bphases?\s+(id)\s*[-–—]\s*(id)`, `roadmap_md.rs:632-635`) so `build phases 14-15` ranges already parse (probe: `Milestone 3 … first=14 last=15`).
- Planned phases have no disk directory → `PhaseMarker::decide` returns `Future` (`completed=false`); ready/blocked then follows deps.

### Pattern 4: Selection model and key arms (D-A10, D-B03)
- **State (all in `ProjectViewCache`, which is `#[derive(Default)]` at `mod.rs:906-907` — "additive with zero constructor churn", its own doc at `mod.rs:1022-1024`):**
  - `roadmap_selected: Option<String>` — the selected phase's `phase_key` (survives ROADMAP reloads/reorders). `None` ⇒ default to the active (◉) phase, else first non-done, else first.
  - `roadmap_band_cursor: Option<usize>` — cursor resting on a band row (needed for `Enter` on the shipped-summary row and `Space` on a band).
  - `roadmap_folded: HashSet<String>` — folded milestone keys (shipped summary starts folded; the active band starts open).
  - `roadmap_edge_cycle: usize` — h/l repeat-to-cycle index, reset whenever the selection changes by any other key.
  - Viewport: list scroll offset in a `Cell` on `DetailScreen` like `generic_viewport` (`detail.rs:658`), because `render` takes `&self`.
- **Shared selection with Phases (Pipeline):** today `pipeline_selected: usize` indexes `state.phases` (`mod.rs:935`; clamped in `render_pipeline_tab` `detail.rs:4328-4333`). Recommended: on Roadmap `Enter`, resolve `roadmap_selected` → index in `state.phases`, write `pipeline_selected`, then `switch_to_tab(alias, tab_index(&DetailSubView::Pipeline), …)` (never a literal index). On Phases `j/k/PgUp/PgDn`, also write `roadmap_selected = phase_key(state.phases[pipeline_selected].number)` so the relationship is bidirectional. Enter on a planned (build) phase → `SetStatusMessage("Build phase 14 is a planned placeholder, not a GSD phase")` (it has no Phases entry).
- **Key collision check** `[VERIFIED: detail.rs top-level arms read this session]`: `j/Down`, `k/Up`, `PageUp/PageDown` currently fall to the `_ =>` generic-scroll arm for RoadmapViz (`detail.rs:1799-1808`, `1910-1918`, `2077-2086`, `2207-2215`) — add explicit `DetailSubView::RoadmapViz` arms that move the cursor when `!roadmap_box_view` and keep generic scroll for the box view. `g` is bound only as `KeyCode::Char('g') if current_view == DetailSubView::Browse` (`detail.rs:2721`), `G` only `if current_view == DetailSubView::Driver` (`3106`) → add guarded `RoadmapViz` arms. `h`, `l`, `[`, `]` are unbound anywhere in the detail screen. `Enter | Char(' ')` is one arm with a per-view match whose `_ => ScreenAction::None` covers RoadmapViz today (`2262`, `2717`) → add a RoadmapViz branch that distinguishes `code`. `v` stays (`3398`). `Left/Right` stay (tab switching).
- Help: `help.rs:229` has a **stale** row `row("r", "Toggle roadmap visualization (detail view)")` — `r` is bound only on the Config tab (`detail.rs:2811`). Replace with Roadmap rows (j/k, g/G, h/l, [/], Space, Enter, v).

### Pattern 5: Layout and width budget (D-A09, D-A13)
- **Breakpoint constant:** `ROADMAP_SIDE_BY_SIDE_MIN_COLS: u16 = 100` measured on the Roadmap content area width. Side-by-side: detail pane width `clamp(w * 2 / 5, 40, 56)`; list takes the rest (Mockups A/B at 120 cols: list 70, detail 50). Stacked (<100): detail pane height `min(9, content_h / 2)` rows incl. borders (Mockup C: 9), list gets the rest.
- **80×24 vertical budget:** detail screen = tab bar 3 + footer 1 → 20 content rows; Roadmap header block ≥ 1 summary line (+ Path / fault / paused / change-banner lines only when present) → list box ≥ 6 rows, detail box 9. Put the mockup summary line first; keep optional lines conditional so the common case costs 1-2 rows.
- **Row columns (inner width W):** `lanes (2·L, min 8)` · `marker (3: " ▶ " / " ↑ " / " ↓ " / " · ")` · `num (right-aligned to the widest id)` · `2` · `name (flex, …-truncated)` · `plans (5, right-aligned, "—" when unknown)` · `2` · `wave ("W<n>", 3)`. At 80 cols: 78 − (8+3+3+2+5+2+3) = 52 cells of name. Truncate on the **escaped** string by `chars().count()` (the existing `truncate_subject` rule, `detail.rs:307-317`) so an escape marker can never be half-emitted into something that looks like data.
- **Scrolling:** use ratatui `List` + `ListState::select(Some(row_of_selected_phase))` with the offset persisted in a `Cell` (so the view does not jump); connector/band rows are ordinary items. No horizontal scroll: the name column absorbs width; lanes cap (Pattern 2).
- **Header line** (Mockups): `{alias} · {milestone label} · phase {active} {status} · {k} of {n} phases done` where n counts every listed phase (GSD + planned; Mockup A counts 11 = M3+M4+M5). Milestone label = active roadmap milestone's label, else `state.milestone` + `milestone_name` (sentriq).
- **Start-now line:** ready (○) and active (◉) phases in list order, joined by ` ║ `, truncated with `…` at the box width.

### Pattern 6: Tab consolidation — keep `Archive` as the Docs › Milestones sub-view
- **Recommended over deleting the variant (supersedes part of D-B09, [INFERRED — audit]):** keep `DetailSubView::Archive` but make it a *sub-view of the Docs tab*: `tab_index(&Archive) == tab_index(&Browse) == 7`; `sub_view_from_index(7) == Browse`. A "Files | Milestones" strip renders at the top of both `render_browser_tab` and `render_archive_tab`; a Docs-only key toggles between the two stored sub-views. Consequences:
  - Every existing `DetailSubView::Archive` key arm (Esc depth-pop `1544`, j `1713`, k `1843`, PgDn `1993`, PgUp `2133`, Enter `2514`, e `3232`), the render arms (`3472`, `5083`), the arrival logic in `switch_to_tab` (`1408`), the escape-guard sub-states ("Archive tab, phase list/file list") and the **debug session's new regression tests** (`app.rs:4725-5000`, which call `DetailScreen::opened_on(alias, DetailSubView::Archive, …)` and assert rendered text `"Archive > v1.2"`) keep working unchanged.
  - `every_tab_index_round_trips_through_its_sub_view` iterates index → view → index (`detail.rs:9889-9902`) and still holds; add a separate assertion that `tab_index(&Archive) == tab_index(&Browse)`.
  - `switch_to_tab(alias, index, …)` resolves an index to one view; entering Milestones needs a sibling `switch_to_sub_view(alias, DetailSubView::Archive, …)` (extract the body of `switch_to_tab` to take a `DetailSubView`; `switch_to_tab` becomes a thin index adapter). `opened_on` already goes through `tab_index` → would land on Browse, so change it to call the sub-view form directly.
  - Breadcrumb: prefer keeping the text `Archive > v1.2` (or update the three `app.rs` assertions in the same commit — never weaken them).
- **Docs sub-tab key (discretion):** `m` toggles Files ↔ Milestones on the Docs tab (`m` is unbound in the detail view `[VERIFIED: no Char('m') arm]`); footer hint `[m]ilestones` / `[m] files`. Roadmap `Enter` on the shipped-summary row → `switch_to_sub_view(Archive)`.
- `PhaseList` **is** removed: variant, `render_phase_list` (`3608-3792`), both dispatch arms (`3465`, `5076`), escape-guard row/label, test usages (`detail.rs:9790, 9823, 11395, 11446, 14508`, `app.rs:4260`). `Pipeline` keeps its variant name (label/titles change to "Phases"; block titles `" Pipeline "` at `render_pipeline_tab` become `" Phases "`/`" Pipeline "` as appropriate) — renaming the variant is churn with no user-visible benefit (discretion).
- **New constants (derive, don't hand-edit — the anti-drift test `the_tab_bar_widths_are_the_label_arrays_own_arithmetic` re-derives them):**
  ```rust
  pub(crate) const TAB_COUNT: usize = 9;
  const TAB_LABELS_FULL: [&str; TAB_COUNT] =
      ["1:Roadmap", "2:Phases", "3:Backlog", "4:Git", "5:Queue", "6:Sess", "7:Cfg", "8:Docs", "D:Drive"];
  const TAB_LABELS_COMPACT: [&str; TAB_COUNT] =
      ["1:Rd", "2:Ph", "3:Bk", "4:Gt", "5:Qu", "6:Ss", "7:Cf", "8:Dc", "D:Dr"];
  pub(crate) const DRIVER_TAB_INDEX: usize = 8;
  // Σ(len+2) + (n−1) (+1 marker cell with Driver):
  pub(crate) const TAB_BAR_FULL_CELLS: u16 = 89;            // 62 + 18 + 8 + 1
  pub(crate) const TAB_BAR_FULL_CELLS_NO_DRIVER: u16 = 78;  // 55 + 16 + 7
  pub(crate) const TAB_BAR_COMPACT_CELLS: u16 = 63;         // 36 + 18 + 8 + 1
  pub(crate) const TAB_BAR_COMPACT_CELLS_NO_DRIVER: u16 = 55; // 32 + 16 + 7
  ```
  `[ASSUMED — arithmetic done here with the formula quoted from detail.rs:397-402 "Σ(len + 2) + (n − 1)" plus the Driver marker cell; the existing test will confirm or refute it]`. Consequence worth noting: the flag-off full bar (78) now fits an 80-col terminal whole.
- Digits: `'1'..='8'` → indices 0..7; `'9'` and `'0'` arms deleted (fall to `_ => None`, D-B10). Footer prefix `"[1-0/D]"`/`"[1-0]"` → `"[1-8/D]"`/`"[1-8]"` (`detail.rs:6105, 6155`).

### Anti-Patterns to Avoid
- **Reference rows for skip-layer edges** (the current `refs` mechanism, `roadmap_graph.rs:395-400, 832-844`) — this is the root cause of "node rendered twice". The new model must have no row type that repeats a node label.
- **Per-row milestone labels** (`milestone_decorations`, `roadmap_graph.rs:889-939`) — replaced by one band row per milestone.
- **Hand-written tab indices** at call sites — always `tab_index(&view)` (the `opened_on` doc explains why, `detail.rs:715-720`).
- **Deciding done/current from the checkbox** — use `PhaseMarker::decide` (D-B12).
- **`str::len` / byte slicing for widths** — panics on multibyte (see `shorten_session_id` doc, `detail.rs:167-196`).
- **Widening `parse_depends_on` or `parse_roadmap_phases` globally** — breaks GSD conformance and pinned tests.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal-safe text | custom escaping | `crate::text::render_for_terminal` / `shown()` / `Untrusted::shown()` | Covers Cf/Default_Ignorable AND Cc/C1; enforced by `render_escape_guard` |
| Phase id equality | `==` on strings | `phase_key` / `same_phase` / `PhaseNum` | `07` vs `7`, `07.1` vs `7.1` |
| done/current decision | checkbox logic | `state.phase_marker(phase)` / `PhaseMarker::decide` | Folded todo; picsync regression history |
| Plans done/total | re-count | `state_reader::phase_plan_counts` (`mod.rs:406-417`) | Roadmap-vs-disk precedence already decided |
| `[stage]` badge | new formatter | `disk_suffix_spans` (`detail.rs:1206-1280`) | D-B08 |
| Cycle handling | new DFS | existing `break_cycles` + `cycle_note` | Iterative, bounded footer (WR-01) |
| Scroll clamping | new math | `clamp_scroll` / `ViewportMetrics` (`detail.rs:212-225`) | UIFIX-04 lesson |
| Wrapping the Goal | manual word-wrap | `Paragraph::wrap(Wrap { trim: true })` in a right-hand column of a `Layout::horizontal([Length(label_w), Min(0)])` | Hanging indent for "Goal  …" (Mockup C) |
| Tab-bar tiers | new tiering | existing `tab_titles` / `windowed_tab_titles` | Only the arrays/constants change |

**Key insight:** almost everything except the reduction, the lane assigner and the widget already exists; the risk is in wiring and in the wide mechanical renumber, not in algorithms.

## Runtime State Inventory

(Tab renumber / variant removal is a refactor.)

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `detail_sub_view_per_project` is an in-memory `HashMap<String, DetailSubView>` (`mod.rs:1247`); `DetailSubView` derives only `Debug, Clone, PartialEq, Default` (`app.rs:15`), no serde. `config.rs`/`registry.rs`/`main*.rs`/`cli.rs` contain no sub-view/tab state `[VERIFIED: grep]` | none |
| Live service config | None — no external service holds tab state | none |
| OS-registered state | None | none |
| Secrets/env vars | None | none |
| Build artifacts | None — no generated code depends on tab indices | none |

## Tab-Index Consumer Inventory (at `46d7e82`; re-read before editing)

### `src/ui/screens/detail.rs`
| Line(s) | Item | Change |
|---|---|---|
| 347-368 | `TAB_COUNT = 11`, `visible_tab_count` doc "11 … 10" | → 9 / "9 … 8" |
| 373-385 | `TAB_LABELS_FULL` | new order (Pattern 6) |
| 390-392 | `TAB_LABELS_COMPACT` | new order |
| 395 | `DRIVER_TAB_INDEX = 10` | → 8 |
| 397-430 | `TAB_BAR_{FULL,COMPACT}_CELLS[_NO_DRIVER]` = 107/77/96/69 + docs | recompute (89/63/78/55) + docs |
| 504-507, 540-543 | docs "first ten", "ten-label" | text |
| 725-730 | `opened_on` | route through sub-view form (Pattern 6) |
| 782-798 | `tab_index` | new mapping; `Archive` → Docs index |
| 800-827 | `sub_view_from_index` + fallback `PhaseList` | new mapping; fallback `RoadmapViz` |
| 829-847 | `effective_sub_view` fallback `PhaseList` | → `RoadmapViz` |
| 1283-1428 | `switch_to_tab` (Archive arrival at 1408) | split into index adapter + `switch_to_sub_view` |
| 1451-1488 | `adjudicate_screen!` prose ("eleven tabs", PhaseList, Pipeline, Archive) | update prose |
| 1544, 1713, 1843, 1993, 2133, 2514, 3232 | `DetailSubView::Archive` key arms | unchanged if Pattern 6 adopted |
| 1674, 1828, 1954, 2118 | `DetailSubView::Pipeline` j/k/PgDn/PgUp arms | also write shared `roadmap_selected` (plan C) |
| 1799-1808, 1910-1918, 2077-2086, 2207-2215 | `_ =>` generic scroll (Roadmap reaches it) | add RoadmapViz arms (plan C) |
| 2220-2229 | digit arms `'1'..'0'` → 0..9 | `'1'..'8'` → 0..7; delete `'9'`,`'0'` |
| 2240-2242 | `Shift+D` → `DRIVER_TAB_INDEX` | unchanged (const moves) |
| 2244-2260 | Left/Right bounded by `visible_tab_count` | unchanged |
| 2262-2719 | `Enter | Space` per-view match | add RoadmapViz branch (plan C) |
| 2721, 3106 | `g` (Browse) / `G` (Driver) guarded arms | add RoadmapViz guards (plan C) |
| 3398-3404 | `v` on RoadmapViz | keep |
| 3463-3484, 5074-5097 | render dispatch ×2 (`render`, `render_main_only`) | drop PhaseList arms |
| 3608-3792 | `render_phase_list` | delete (content already in `render_roadmap` header; badge → detail pane) |
| 3795-3931 | `render_roadmap` | rewrite (plan C); header block at 3805-3875 already has Path/Status/Milestone/unreadable/recovered/Paused/change banner |
| 4301-4466 | `render_pipeline_tab` titles `" Pipeline "` | label "Phases" |
| 6093-6140 | `driver_footer_spans` `"[1-0/D]"` (6105) | `"[1-8/D]"` |
| 6145-6243 | `footer_spans` prefix (6155); RoadmapViz arm (6227-6232); Archive/Browse arms | new prefix; Roadmap hints; Docs `[m]` hint |
| 9770-9779 | `test_other_footers_unchanged_by_browse_edit_hint` (exact strings with `[1-0/D]`) | update |
| 9788-9810, 9821-9844 | `the_tabs_hint_*` (lists incl. PhaseList, Archive; `[1-0/D]`, `[1-0]`) | update lists + strings |
| 9848-9861 | `the_driver_footer_has_three_measured_width_forms` (`[1-0/D]`) | update |
| 9889-9902 | `every_tab_index_round_trips_through_its_sub_view` (fallback PhaseList) | fallback RoadmapViz; add Archive↔Browse same-index assertion |
| 9936-9939 | `the_visible_tab_count_drops_the_eleventh_tab…` asserts `10` | → 8 (rename test) |
| 9948-9973 | `no_width_tier_emits…` (active `[0,5,9]`) | active within 0..8 |
| 9996-10009 | `the_ten_tab_bar_renders_whole…` (`10`, index 9) | eight-tab version |
| 10018-10054 | anti-drift width test | passes after constants recomputed |
| 10058-10069 | `index_ten_is_not_a_tab…` (fallback PhaseList) | index 8 / RoadmapViz |
| 10073-10088 | `a_stored_driver_sub_view_reads_as_the_default_tab…` (PhaseList) | RoadmapViz |
| 10127-10156 | `right_from_the_last_visible_tab…`, `right_from_tab_nine…` (index 9 = Browse) | index 7 = Browse |
| 10171-10186 | `the_full/compact_tier_renders_eleven_labels…` | nine |
| 10193-10214 | windowed tier test (active `[0,5,DRIVER]`) | fine; re-check |
| 10282-10328 | `the_active_tab_label_is_always_present…` literal `("1:Phases","1:Ph")` | `("1:Roadmap","1:Rd")` |
| 10334-10360 | `shift_d_and_right_both_reach_the_driver_tab` (Browse → Right → Driver) | still valid (Browse is last before Driver) |
| 10504, 11395-11396, 11446, 14508 | test fixtures using `DetailSubView::Archive` / `PhaseList` | PhaseList → another view; Archive stays (Pattern 6) |
| 14451-14521 | `roadmap_graph_tab_*` tests (assert `"1 ─► 2 ─► 3"`) | rewrite for list view (plan C) |

### `src/app.rs`
| Line(s) | Item | Change |
|---|---|---|
| 15-40 | `DetailSubView` enum, `#[default] PhaseList`, Driver doc "Index 10 … TAB_COUNT is 11" | remove `PhaseList`, `#[default] RoadmapViz`, doc "Index 8 … TAB_COUNT is 9" |
| 3481, 3596 | tests using `DetailSubView::Pipeline` (driver redraw gate) | unchanged |
| 4249-4262 | `the_driver_sub_view_is_index_ten_in_both_directions` (10, 11 → PhaseList) | 8, 9 → RoadmapViz; rename |
| 4725-5000 | debug-session archive regression tests (`opened_on(…, Archive, …)`, assert `"Archive > v1.2"`) | unchanged under Pattern 6 — MUST stay green |

### `src/ui/screens/render_escape_guard.rs`
| Line(s) | Item | Change |
|---|---|---|
| 110-128, 838, 1012-1013, 1240-1255, 1590, 1617 | prose "eleven tabs", "fifteen states", tab lists | update prose |
| 859-948 | `hostile_project_state` | add `phase_goals` (identity), a `planned_phases` entry, `milestone_name` (plan C) |
| 1355-1361 | `ALL_SUB_VIEWS: [DetailSubView; 11]` | `[_; 10]` (drop PhaseList; keep Archive as a sub-view state) |
| 1382-1519 | `DETAIL_TAB_ARRIVAL` rows `"PhaseList tab"`, `"RoadmapViz tab"`, `"Pipeline tab"`, `"Archive tab…"` | drop PhaseList row; rewrite RoadmapViz reason (list + detail pane draws goal, milestone band label, planned phase names); optional relabel |
| 1558-1573 | `sub_view_label` | drop PhaseList |
| 1651-1752 | `DETAIL_SUB_STATES` (RoadmapViz box view, Archive phase/file list) | keep; optionally add "RoadmapViz tab, folded shipped band" |
| 2612 | "eleven adjudicated" — this is **screens**, not tabs | none |

### Elsewhere
| File:line | Item | Change |
|---|---|---|
| `src/ui/screens/help.rs:227-234` | detail-view rows incl. stale `r` row (229) and `v` row (230) | Roadmap key rows; help test at `help.rs:884-895` pins the `v` row wording |
| `src/ui/screens/normal.rs:582-594, 2405-2472` | `b` opens detail on Backlog; test presses `'3'` for Backlog | **unchanged** — Backlog stays at `3` |
| `src/ui/screens/mod.rs:906-1040` | `ProjectViewCache` | + roadmap fields (plan C), + docs sub-tab state if needed (plan D) |
| `src/ui/screens/driver.rs:16, 232, 975` | doc comments naming Driver/Pipeline | prose only |
| `src/executor/mod.rs:290` | doc comment mentioning `DetailSubView` derives | none |
| `README.md:47-49` | "10-tab detail view: Phases, Roadmap (ASCII DAG), …" | "8-tab detail view (plus the experimental Driver tab): Roadmap (master/detail dependency list), Phases, Backlog, Git, Queue, Sessions, Config, Docs (Files, Milestones)" |
| `README.md:28, 59` | "Milestone archive browsing/browser" | mention Docs › Milestones |
| `docs/ARCHITECTURE.md:165` | "sub-views (PhaseList, Roadmap, …" | update |
| `src/ui/screens/delete_confirm.rs:170`, `driver_confirm.rs:704`, `normal.rs:1225`, `app.rs:530`, `mod.rs:2351` | `detail_sub_view_per_project` construction/removal | none (enum-keyed map) |

## File Ownership & Wave Plan (recommended)

| Plan | Wave | Owns (writes) | Reads / depends on | Notes |
|---|---|---|---|---|
| **24-01 Parser** | 1 | `src/state_reader/roadmap_md.rs`, `src/state_reader/mod.rs` (3 `ProjectState` fields + assignments in `parse_project_state`), `tests/fixtures/roadmaps/**` (sanitised), optionally `tests/state_reader_test.rs` | — | No `RoadmapPhase` field change ⇒ touches no UI/test fixture elsewhere. `ProjectState` additions are zero-churn (all literals use `..`) |
| **24-02 Layout model + widget** | 1 | `src/ui/roadmap_graph.rs` (rewrite layout half; keep dep/cycle/wave code), `src/ui/roadmap_view.rs` (new), `src/ui/mod.rs` (one `pub mod` line) | — (pure inputs; no `ProjectState` coupling) | Define an input struct (`ListNode { id, name, deps, milestone, marker, plans, goal, stage_badge, planned }`) so the model compiles without plan 01's fields. Tests: synthetic topologies incl. the three fixtures' graphs spelled inline + `TestBackend` at 80×24 / 120×30 |
| **24-03 Roadmap tab wiring** | 2 | `src/ui/screens/detail.rs` (render_roadmap, adapter ProjectState→ListNode, RoadmapViz key arms, Pipeline-arm selection sync, footer RoadmapViz arm, tests), `src/ui/screens/mod.rs` (ProjectViewCache fields), `render_escape_guard.rs` (hostile goal/planned/milestone_name + RoadmapViz arrival reason), `help.rs` (Roadmap rows) | 24-01, 24-02 | End-to-end fixture tests (daily-vow `20` once, sentriq `9` once, ttbook build phases present) live here |
| **24-04 Tab consolidation + Archive→Docs** | 3 (last) | `detail.rs` (tab constants/labels/index/digits/footer/PhaseList removal/Docs sub-tab/`switch_to_sub_view`/shipped-row Enter), `app.rs` (enum + its test), `render_escape_guard.rs` (ALL_SUB_VIEWS/arrival/labels), `help.rs`, `README.md`, `docs/ARCHITECTURE.md`, `mod.rs` (if a docs sub-tab field is needed) | 24-03 | D-B06: precondition "debug fix commits `3c0e38f`/`581aa7d` are ancestors of HEAD" — already true at `46d7e82`; still re-read `detail.rs` first. Could be split into 04a (renumber + PhaseList removal) and 04b (Docs sub-tab UI) **sequentially** — same files |

**Why 03 and 04 are sequential:** both edit `detail.rs` (footer_spans, tests module tail), `render_escape_guard.rs` (adjacent `DETAIL_TAB_ARRIVAL` rows `"PhaseList tab"`/`"RoadmapViz tab"`), and `help.rs` (adjacent rows 228-230) — a shared-file conflict, the reason the user's rules accept for sequential execution.
**Why 01 ∥ 02 is safe:** disjoint files; 02 compiles against its own input struct, not against 01's new `ProjectState` fields.

## Common Pitfalls

### Pitfall 1: Build phases leaking into GSD-facing logic
**What goes wrong:** `state.phases` gains 14-18 → frontier/active phase, dashboard `P{n}` label and driver router start treating placeholders as real phases.
**Why:** GSD's parser excludes them (verified regex).
**Avoid:** separate `planned_phases`; only the Roadmap adapter merges them. **Warning sign:** any diff to `router.rs`, `browser.rs::resolve_active_phase_dir`, or the frontier loop in 24-01.

### Pitfall 2: Spurious milestones from build-phase headings
**What goes wrong:** `#### Build phase 14 (Milestone 3): …` matches the milestone detector (`(?i)\bMilestone\s+\d`) → five fake bands. **Avoid:** widen `roadmap_milestones`' `phase_heading` check (`roadmap_md.rs:639-643`, `713`). **Test:** ttbook fixture yields exactly the five real milestones.

### Pitfall 3: THIRD_PARTY_STRINGS census
**What goes wrong:** adding `goal: String` (or `milestone_name: String`) trips `tests/spawn_seam_guard.rs` and `untrusted.rs::the_enumerated_prose_set_is_exactly_the_six_free_text_fields`. **Avoid:** `Untrusted`/`HashMap` carriers (Pattern 3). **Warning:** a plan that edits `src/driver/untrusted.rs` — that is a security-surface widening needing explicit sign-off.

### Pitfall 4: Escape-guard arrival silently passing
**What goes wrong:** the probe asserts arrival per state against a chrome baseline (`render_escape_guard.rs:1521-1553`); if the hostile fixture does not populate `phase_goals`, the goal branch is never rendered and nothing goes red. **Avoid:** populate goal/planned/milestone_name with the identity and state in the RoadmapViz arrival reason what is drawn. Probe renders at 200×60 (`PROBE_WIDTH/HEIGHT`, `render_escape_guard.rs:2016-2017`) → side-by-side layout; make sure the identity arrives **whole** at least once (the detail pane name/goal), not only truncated in a list row.

### Pitfall 5: Hard-coded tab literals left behind
**What goes wrong:** a missed `[1-0/D]`, a `9`, a `"1:Phases"` literal → silent wrong-tab bug (a named phase risk). **Avoid:** work from the inventory above; after the edit, grep `\[1-0\|1:Phases\|5:Pipe\|8:Arch\|0:Docs\|PhaseList\|DRIVER_TAB_INDEX, 10\|, 10)` across `src tests README.md docs` — expect zero (except `ArchiveDepth::PhaseList`, a different enum).

### Pitfall 6: `cargo test` fail-fast hiding suites
**What goes wrong:** the lib test's known env failure stops later suites; a green-looking run can skip the envelope suites (user memory). **Avoid:** `rtk proxy cargo test --no-fail-fast`; compare suite count (49) and totals to the baseline below.

### Pitfall 7: Selection lost on refresh / fold
**What goes wrong:** an index-based cursor jumps when ROADMAP reloads or a band folds. **Avoid:** store `phase_key`; if the selected phase becomes hidden by a fold, move the cursor to that band row.

### Pitfall 8: Parallel set definition
**What goes wrong:** D-A02 says Parallel = "other phases in the same wave", but user-verified Mockup C lists `◉ 9 ◌ 10 ◌ 11` (waves 1, 2, 3) for phase 12 with "(no edge either way; can run any time)". **Resolution [INFERRED — audit]:** Parallel = phases with **no dependency path in either direction** (incomparable in the partial order); this satisfies all three mockups (A: none; B: 22; C: 9, 10, 11). `[`/`]` still step through the **same wave** per D-A10.

### Pitfall 9: Width math on raw strings
**Avoid:** measure the escaped string (`width_of`/`Span::width`), never `len()`; truncate escaped text by chars.

## Code Examples

### Build-phase heading (plan 01)
```rust
// PHASE_ID quoted verbatim from roadmap_md.rs:49:
// const PHASE_ID: &str = r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?";
let build_heading_re = Regex::new(&format!(
    r"(?i)^\s*#{{2,4}}\s+Build\s+phase\s+({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
    id = PHASE_ID
)).unwrap();
// "#### Build phase 14 (Milestone 3): Supervised first live booking" -> ("14", "Supervised first live booking")
```

### Goal line (plan 01)
```rust
let goal_re = Regex::new(r"(?i)^\s*\*\*Goal(?:\*\*\s*:|:\*\*)\s*(.*)$").unwrap();
// "**Goal**: The first real booking …"  and  "**Goal:** The first real booking …"
```

### Status glyph consts (plan 02; house `\u{…}` rule)
```rust
const GLYPH_DONE: &str = "\u{25CF}";    // ●  dim
const GLYPH_ACTIVE: &str = "\u{25C9}";  // ◉  yellow bold
const GLYPH_READY: &str = "\u{25CB}";   // ○  green
const GLYPH_BLOCKED: &str = "\u{25CC}"; // ◌  default
const MARK_SELECTED: &str = "\u{25B6}"; // ▶  (row REVERSED)
const MARK_DEP: &str = "\u{2191}";      // ↑  cyan
const MARK_UNBLOCKS: &str = "\u{2193}"; // ↓  magenta
const MARK_IMPLIED: &str = "\u{00B7}";  // ·  dim
const BAND_OPEN: &str = "\u{25BE}";     // ▾
const BAND_FOLDED: &str = "\u{25B8}";   // ▸
const BAND_FILL: &str = "\u{2501}";     // ━
const PARALLEL_SEP: &str = "\u{2551}";  // ║
// Glyph decision: marker = state.phase_marker(phase) (D-B12);
// Done -> ●, Current -> ◉, Future -> ○ if every in-graph dep is Done (external deps count as satisfied) else ◌.
```

### Detail-pane content (from MOCKUPS.md A/B/C)
```
<name>                                   (block title " Phase <id> ")
<milestone label> · wave <w> of <max>
<glyph> <active|ready|blocked|done> · <status if active> · <d>/<t> plans | plans TBD  <[stage] badge>
Goal  <wrapped goal, or "no goal in ROADMAP.md">
Needs     <glyph> <id> <name…> <d/t>         | "nothing declared"
          <glyph> <id> <name…>  (implied via <id>)
Unblocks  <glyph> <id> <name…> [<milestone short> if different] | "nothing (last in <milestone>)" | "nothing"
Parallel  <glyph> <id> …  | "none in wave <w>" | "… (no edge either way; can run any time)" when it has no edges
⏎ open in Phases   h/l follow edge   j/k move
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Left-to-right graph with junctions + reference rows + per-row milestone labels (quick 260923-md1) | Vertical list with git-log lanes, transitive reduction, milestone bands, detail pane | This phase | Removes duplicate nodes, horizontal scroll, repeated labels |
| 10 tabs + Driver (PhaseList, Pipe, Arch separate) | 8 tabs + Driver; Archive as Docs sub-view | This phase | Digits 9/0 unbound; full bar fits 80 cols with Driver hidden |

**Deprecated/outdated:** `RoadmapGraphWidget`, `split_areas`, `render_text` (row form), `milestone_decorations`, `Segment::Reference`, `GapCell`/`Placement` machinery in `roadmap_graph.rs` — delete with their tests once the new model's tests cover the same topologies (O1-O10 fixtures are worth porting as lane tests).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Store goals as `ProjectState.phase_goals: HashMap<String, Untrusted>` instead of a `RoadmapPhase` field | Pattern 3 | Low — alternative is a 20-literal mechanical change in hot files |
| A2 | Build phases go to a display-only `planned_phases` list, not `state.phases` | Pattern 3b | Medium — if the user wants them counted as real phases, frontier/router semantics change |
| A3 | Parse `Build phase(s)` dependency lines (incl. ranges) only inside build-phase entries | Pattern 3b | Low — without it ttbook shows no deps for 14-18 |
| A4 | Parallel = incomparable set (Mockup C), not strictly same wave (D-A02 text) | Pitfall 8 | Low — cosmetic; both are one function |
| A5 | Keep `DetailSubView::Archive` as the Docs›Milestones sub-view (deviates from D-B09's "remove Archive variant") | Pattern 6 | Low — preserves debug-fix tests; removal remains possible later |
| A6 | Docs sub-tab toggle key = `m` | Pattern 6 | Low (discretion) |
| A7 | Tab-bar widths 89/78/63/55 | Pattern 6 | None — anti-drift test recomputes |
| A8 | Goal regex `(?i)^\s*\*\*Goal(?:\*\*\s*:|:\*\*)\s*(.*)$` | Code Examples | Low — unit tests pin both forms |
| A9 | Fixtures vendored as sanitised structural excerpts (private repos → public crate) | Real Fixtures | **Operator risk acceptance** — genuine human escalation |
| A10 | "Shipped" milestones = those before the active milestone that have a range or member phases (no new struct field) | Pattern 5 / D-A07 | Low — daily-vow/ttbook correct; sentriq has no detected shipped milestones, so no summary row (Mockup C's "earlier" row is not reproduced) |
| A11 | Synthetic band from `state.milestone` + new `ProjectState.milestone_name: Option<Untrusted>` for phases no roadmap milestone contains | Real Fixtures (sentriq) | Low |
| A12 | External (not-in-roadmap) deps count as satisfied for ○/◌ | Code Examples | Low — daily-vow 18←17 would otherwise be "blocked" forever |
| A13 | `Pipeline` variant name kept; only labels/titles become "Phases" | Pattern 6 | None |

## Open Questions (RESOLVED)

1. **Publishing private-repo roadmap content as test fixtures** — the three sources are PRIVATE, this crate is PUBLIC. Recommendation: sanitised excerpts by default; `checkpoint:human-verify` before committing if verbatim content is wanted. RESOLVED: sanitised excerpts, no checkpoint (24-01, [INFERRED — audit]).
2. **Sentriq "earlier" row** (Mockup C: `▸ earlier  pre-GSD 1–3 · v0.11 4–7 · TASK-111 (quick)`) — needs index-table / parenthetical-range milestone parsing that is not in the locked scope. Recommendation: not reproduced; sentriq shows only its synthetic v0.12 band. Revisit as a follow-up if wanted. RESOLVED: shipped-milestones row built from real milestones only; pre-GSD/TASK-111 not reproduced (24-01 Task 3).
3. **Lane overflow glyph** at the cap — discretion; recommend a single `┆` column. RESOLVED: single `┆` overflow column (24-04).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc (MSRV 1.88) | build/test | ✓ | (built this session) | — |
| GSD oracle `~/.claude/gsd-core` | `tests/driver_router_conformance.rs` | ✓ | installed | container gate |
| git 2.53.0 local vs 2.55.0 constants | `envelope/policy.rs` witness test | ✗ mismatch (expected) | 2.53.0 | known-failing locally; green on runner |
| `gh` | fixture privacy check only | ✓ | — | — |
| Real fixture repos (daily-vow, sentriq, ttbook) | vendoring excerpts | ✓ | HEADs `2dd223b`, `dfc6d2c`, `f420fd5` | — |

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; included because the orchestrator requested it.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]`; `ratatui::backend::TestBackend` for rendered-text assertions |
| Config file | none |
| Quick run command | `rtk proxy cargo test --lib roadmap -- --nocapture` (module-filtered) |
| Full suite command | `rtk proxy cargo test --no-fail-fast` |
| Lint gate (CI) | `rtk proxy cargo clippy -- -D warnings` |

### Measured baseline at `46d7e82` (exported snapshot, own target dir; working tree untouched)
- `cargo test --no-fail-fast`: **49 suites, 2251 passed, 1 failed, 15 ignored**; the one failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (`src/envelope/policy.rs:10721`) — the known local git-version witness `[VERIFIED: /tmp/gmm-head2-test.log]`. (At `53b2159`, before the debug fix: 2246 passed / 1 failed, same test.)
- `cargo clippy -- -D warnings` (the CI/pre-tag gate): **clean** `[VERIFIED]`.
- `cargo clippy --keep-going --all-targets -- -D warnings`: **11 pre-existing errors**, not 6: `src/browser.rs:156,157,158` (bool_assert_comparison), `src/project_creator.rs:146` (cmp_owned), `tests/envelope_carrier_reach.rs:1711`, `tests/envelope_config_resolution.rs:2206`, `tests/envelope_control_carrier.rs:978`, `tests/envelope_wrapper_class.rs:6127, 6213, 10795, 11232` `[VERIFIED: /tmp/gmm-head2-clippy-all.log]`. Without `--keep-going` cargo stops early and shows fewer — which is likely where "6" came from. **Gate for this phase: no new location in that list, and the CI command stays clean.**

### Success Criteria → Test Map
| ID | Behavior | Test Type | Automated Command | File Exists? |
|----|----------|-----------|-------------------|-------------|
| SC-1 | reduction: daily-vow 23→20 implied via 21; sentriq 11→9 via 10; ttbook 10/11/12 | unit (model) | `cargo test --lib roadmap_graph::tests::reduction` | ❌ Wave 0 |
| SC-1 | lanes reproduce Mockups A/B/C shapes; "no deps" zig-zags | unit (model text) | `cargo test --lib roadmap_graph::tests::lanes` | ❌ |
| SC-1 | each phase id once, each milestone label once, on all 3 fixtures | render (TestBackend 80×24, 120×30) | `cargo test --lib detail::tests::roadmap_fixture` | ❌ |
| SC-2 | j/k/g/G/h(cycle)/l/[ ]/Space/Enter move/fold/open; Enter lands on Phases with `pipeline_selected` = phase | unit (handle_key) | `cargo test --lib detail::tests::roadmap_keys` | ❌ |
| SC-2 | detail pane: Goal, Needs (+implied via), Unblocks, Parallel; sentriq 12 "nothing declared" + "can run any time" | render | `cargo test --lib roadmap_view::tests` | ❌ |
| SC-3 | stacked < 100 cols, side-by-side ≥ 100, no row wider than area, no panic at tiny sizes (port `assert_no_panic_at_tiny_sizes`) | render | `cargo test --lib roadmap_view::tests::width` | ❌ |
| SC-4 | `#### Build phase N (Milestone M): Title` parsed to planned phases; no spurious milestones; goals both syntaxes; `parse_roadmap_phases` output unchanged on ttbook | unit (parser) | `cargo test --lib state_reader::roadmap_md::tests` | partial (existing module) |
| SC-5 | round-trip, width anti-drift, digit 9/0 inert, footer `[1-8/D]`, active label present at 40/60/80/120, Archive↔Browse same index, debug archive tests still green | unit + render | `cargo test --lib detail::tests && cargo test --lib app::tests` | ✅ existing (update) |
| SC-5 | escape guard: goal/planned/milestone_name arrive escaped; PhaseList state gone | probe | `cargo test --lib render_escape_guard` | ✅ (update) |
| — | census unchanged (no new free-string fields) | integration | `cargo test --test spawn_seam_guard` | ✅ |

### Sampling Rate
- **Per task commit:** module-filtered `rtk proxy cargo test --lib <module>` + `rtk proxy cargo clippy -- -D warnings`
- **Per wave merge:** `rtk proxy cargo test --no-fail-fast` → expect 49 suites, exactly one failure (git-version witness)
- **Phase gate:** full suite + `--keep-going --all-targets` clippy location set unchanged, before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/fixtures/roadmaps/{daily-vow,sentriq,ttbook}-ROADMAP.md` + `README.md` (sanitised, provenance) — plan 01
- [ ] model/lane/reduction test module in `roadmap_graph.rs` — plan 02
- [ ] `roadmap_view.rs` tests module + an 80×24 `render_detail_to_text` variant — plans 02/03

## Security Domain

(`security_enforcement` absent from config ⇒ enabled.)

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Validation, Sanitization & Encoding | **yes** | Output encoding of all ROADMAP/STATE text via `render_for_terminal`/`shown()`/`Untrusted::shown()`; enforced by `render_escape_guard` probe |
| V6 Cryptography | no | — |
| V12 Files/Resources (untrusted file content) | yes | regex crate linear-time matching `[CITED: docs.rs/regex]`; no byte slicing of untrusted strings; bounded model size (n phases) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| ANSI/C0/C1 escape or bidi (`U+202E`) in a phase name/goal/milestone repainting the terminal | Tampering/Spoofing | Escape before measuring; store only escaped strings in the model (existing `roadmap_graph.rs` doc, lines 16-22) |
| Goal text reaching the driver's model seam unlabelled | Elevation (prompt injection) | Keep goals in an `Untrusted` carrier outside `THIRD_PARTY_STRINGS`; any seam use must go through `untrusted_block` |
| Pathological ROADMAP (huge, deep, cyclic) freezing render | DoS | Iterative cycle breaking (existing), O(n·e) reduction, compute from in-memory state only; cap lanes |
| Multibyte truncation panic | DoS | char-based truncation (`truncate_subject` rule) |

## Sources

### Primary (HIGH confidence)
- Codebase read this session: `src/ui/roadmap_graph.rs` (full), `src/state_reader/roadmap_md.rs:1-330, 440-780`, `src/ui/screens/detail.rs` (tab machinery, key arms, render fns, tab tests), `src/app.rs:1-60, 4249-4262, 4725-4760`, `src/ui/screens/mod.rs:895-1040`, `src/ui/screens/render_escape_guard.rs:840-948, 1340-1752`, `src/ui/screens/help.rs:190-260`, `src/driver/untrusted.rs:1-260`, `src/state_reader/mod.rs:42-130, 185-420, 590-680`, `tests/spawn_seam_guard.rs` census section, `tests/fixtures/codex/README.md`, `.planning/debug/archive-milestone-view-loading.md`
- `~/.claude/gsd-core/bin/lib/roadmap.cjs:364`, `phase-id.cjs:260-310` — GSD phase-heading grammar
- Probes run this session: `/tmp/gmm-probe` (current graph on real fixtures), `/tmp/gmm-proto/lanes.py` (algorithm vs mockups), `/tmp/gmm-w` (glyph widths), baseline test/clippy logs `/tmp/gmm-head-test.log`, `/tmp/gmm-head2-*.log`

### Secondary (MEDIUM confidence)
- docs.rs/regex — worst-case `O(m * n)` search complexity

### Tertiary (LOW confidence)
- None relied upon.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; versions read from Cargo.lock.
- Architecture: HIGH — algorithm prototyped and matched against all three user-verified mockups; file ownership derived from actual struct-literal and consumer scans.
- Pitfalls: HIGH — defects reproduced on real data; census/escape-guard mechanics read from the tests that enforce them.
- Tab inventory: HIGH at `46d7e82`; line numbers will drift — re-read at execution.

**Research date:** 2026-09-23
**Valid until:** 2026-10-07 (fast-moving tree; re-verify line numbers per plan)
