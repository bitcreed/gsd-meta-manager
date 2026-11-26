# Phase 36: Dashboard and detail - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

Gathered unattended (`--auto`). Every decision follows `.planning/research/OWNER-INBOX-UX.md`
(adopted as written by REQUIREMENTS.md, report §3.1, §3.2, §7) and the Phase 34/35 decisions,
unless marked **[inferred]**: decided by the orchestrator from the report, REQUIREMENTS.md,
ROADMAP.md, the Phase 34/35 artifacts and a scout of the dashboard (`src/ui/screens/normal.rs`,
`mod.rs`), the detail Phases tab (`detail.rs`), the reload path (`src/app.rs`) and the owner
reader (`src/state_reader/owner_items/`).

<domain>
## Phase Boundary

The TUI starts consuming the owner digest that Phases 34-35 built (`read_owner_digest`); no new
reader rule, signal or JSON key is added.

- **Dashboard:** a quiet `You` column (report §3.1), a one-line `You:` summary with `[i] inbox`,
  a `has owner work` rank in AttentionFirst, and a `/y [lane]` filter.
- **Detail:** an `Owner` block on the existing Phases tab (D5: no new tab) with the
  `closes after` header, expandable groups and collapsed Old debt / Acknowledged / Todos folds.
- **Docs:** help overlay and README for the parts above (OWN-23, TUI half).

Requirements: OWN-16, OWN-17, OWN-18, OWN-19, OWN-23 (help overlay and README part).

Not in this phase: the inbox screen and the `i` key's real target, `v` run command, `A`
acknowledge, `y` copy checklist, `o` open source file (Phase 37, OWN-20..22); any change to the
`❢` badge or `needs_human` (D1); todo triage beyond a count (report §6 deferred).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (not re-decided)
- D1: `needs_human`, the `❢` badge, `/h` and AttentionFirst rank 0 are **not widened**. Owner work
  is a quiet text cell plus a separate sort rank. Existing `needs_human` tests stay untouched.
- D4: archived debt on the dashboard is a dim `(+N old)` only. D5: block on the Phases tab, no
  new tab. D10: `/y` filter, `i` inbox.
- The digest is computed only by `read_owner_digest` (34 D-15), never inside
  `parse_project_state`; per-file (path, mtime, len) parse caches; `evict_project` on
  `Action::FileChanged` is already wired (`src/app.rs` ~2736). No parse on the render path
  (34 D-16, OWN-12).
- Every project-authored string is already capped/escaped by the reader (`terminal_capped`);
  the TUI still renders through the existing `shown`/`fit_cells` helpers and the
  `render_escape_guard` coverage is extended to the new cells and block (OWN-11).
- Counts mean what they mean in `status --json` `owner` (34 D-22, 35 D-11): `test` = Test items,
  `decide` = Decide items, `fix_phases` + `fix_debug`, `blocked` items, `closeable_phases`; current
  milestone only, after Layer B; all scopes (root and workstreams) of the project.

### Loading and cost
- **D-01:** The digest is computed in the existing reparse task: `schedule_reparse`'s
  `spawn_blocking` calls `read_owner_digest(&planning_dir, &state, Utc::now(), stale_days)` right
  after `parse_project_state` and sends it in the same `Action::ProjectStateLoaded` (new field).
  No second dispatch site, so `reparse_dispatches` and the OBS-06 test keep their meaning
  **[inferred]**. `stale_days` from `Preferences::owner_stale_days()`.
- **D-02:** The digest is stored on `AppContext` in a new map keyed by alias (e.g.
  `owner_digests: HashMap<String, OwnerDigest>`), **not** as a `ProjectState` field, so the driver
  loop, `next` and the router (other `parse_project_state` callers) never carry or compare it
  **[inferred; narrows 34 D-15's "`ProjectState` gets `owner`"]**. The `changed` check in the
  `ProjectStateLoaded` handler also compares the old and new digest, so a digest-only change
  re-sorts, re-filters and redraws.
- **D-03:** Archived scan cost: the full digest (current plus archived) is computed on every
  reparse because the dashboard's `(+N old)` and the detail Old debt fold need archived counts; a
  warm reparse costs one `stat` per archived file thanks to the per-file caches, and cold cost is
  bounded by OWN-12 (< 1 s for 20 projects). No timer-driven recompute; only watcher-driven
  reparses and the startup load **[inferred]**. Planner checks that the startup fan-out does not
  regress first paint (render the rows first, cells fill when the digest arrives).
- **D-04:** Fold the Phase 34 VERIFICATION follow-up: a test that `App::update(FileChanged)` evicts
  the project's cached owner files and that the following `ProjectStateLoaded` carries a fresh
  digest (e.g. a UAT row flipped to `pass` disappears from the cell) **[inferred]**.

### Dashboard `You` cell (OWN-16)
- **D-05:** Column placement: `Alias, Phase, Status, You, Progress, Backlog` (report §3.1 order;
  Backlog stays last) **[inferred: Backlog position]**. A new width tier at `>= 100` columns adds
  `You`; the `80..99`, `60..79` and `< 60` tiers are **byte-identical** to today, so no existing
  snapshot below 100 columns changes (SC3). Column width: a fixed length that fits the widest
  normal form (`decide 1 · fix 2 ph`, `test 10 → closes 4`, about 20 cells); exact constraint is
  Claude's discretion as long as the Status column keeps `STATUS_COLUMN_MIN_CELLS`.
- **D-06:** Lanes in fixed order `decide`, `test`, `fix`; **at most two** are shown; `blocked`
  never appears. With three lanes non-zero, `fix` is the one dropped (the inbox shows it). This
  follows OWN-16 over the report mockup's `mailbot` row (`fix 4 ph · test 6`), which contradicts
  the fixed order **[inferred]**.
- **D-07:** Wording:
  - `decide N` (N = `decide`);
  - `test N` (N = `test`), with ` → closes M` appended only when `test` is the only lane shown and
    M = `closeable_phases` > 0;
  - `fix N ph` (N = `fix_phases`) when `fix_debug` is 0, else `fix N` with N = `fix_phases` +
    `fix_debug` (a debug investigation is not a phase) **[inferred]**;
  - separator ` · `; the cell is plain default-color text, never red/yellow (quiet cell, badge
    fatigue principle) **[inferred]**.
  Final wording is a human UAT item ("cell wording").
- **D-08:** Clean project (no `decide`/`test`/fix work in the current milestone, blocked-only
  included): a dim `·`. Archived actionable debt is appended dim as ` (+N old)` with N =
  `archived.test + archived.fix + archived.decide` (blocked excluded, as in the cell)
  **[inferred]**; `(+N old)` is shown only when it fits after the lanes, lanes win the space.
  A project with only blocked or archived items therefore shows no lane text (SC2).
- **D-09:** No digest yet (startup, reload pending) → empty cell, never `·`: absent data must not
  read as clean (fail-safe, 34 D-17) **[inferred]**. Unreadable files are already counted as Test
  work by the reader (parse gap = "unreadable checks"), so they show as lane text.

### Summary line (OWN-17)
- **D-10:** A separate one-row `You:` line between the table and the existing footer, shown in the
  `>= 100` tier only (with the column); below 100 columns the layout is unchanged **[inferred]**.
  Form (report §3.1): `You: test T in P phases (C close) · decide D · fix F phases · K of N projects`
  then right-aligned `[i] inbox`. Lanes with 0 are omitted; a fleet with no owner work renders a dim
  `You: ·` plus `[i] inbox` so the layout does not jump **[inferred]**. Plain colors, no red.
- **D-11:** "Totals equal the sum of the visible cells" is defined over the **visible rows** (the
  current filtered, sorted alias list): each total is the sum of the same per-project numbers the
  cell is built from, by one shared function, including lanes the two-lane cap hides
  **[inferred]**. `P` = distinct (scope, phase) pairs with current Test groups, `C` =
  Σ `closeable_phases`, `F` = Σ the D-07 fix number, `K` = visible rows with owner work, `N` =
  visible rows. Unit test: for a fixture fleet, every summary total equals the Σ over visible rows.
- **D-12:** `[i] inbox` is rendered now (SC2). Until Phase 37 binds the screen, `i` on the
  dashboard sets a status message pointing at `gsd-meta-manager inbox` (no screen push, no dead
  key) **[inferred]**; Phase 37 replaces the handler. Both phases ship in v2.5.0, so the interim
  never reaches a release.

### Sort and filter (OWN-18)
- **D-13:** `attention_rank` grows to four ranks: 0 needs a human, **1 has owner work**, 2 driven
  and live, 3 everything else. `has owner work` = `decide + test + fix_phases + fix_debug > 0`
  (current milestone; blocked and archived excluded, the same predicate that makes the cell
  non-`·`). Alphabetical stays the default; the stable sort and alias-pinned selection are
  unchanged. Rank-table unit test covers all combinations; existing `needs_human` tests untouched.
- **D-14:** Filter grammar extends the existing `term/x` suffix form in `app::parse_filter`: a new
  `FilterColumn::OwnerWork(Option<Lane>)` for suffixes `/y`, `/y test`, `/y decide`, `/y fix`
  (also `/yt`, `/yd`, `/yf` short forms is Claude's discretion). `/y` alone = rows with owner work;
  `/y <lane>` = rows whose lane count is > 0 (fix = `fix_phases + fix_debug`). `blocked` is not a
  filter lane (it is not owner work, D-08) **[inferred]**. An unrecognized lane word falls through
  to a plain `All` term, as unknown suffixes do today. `/h` is unchanged.

### Detail: Owner block on the Phases tab (OWN-19)
- **D-15:** Placement: a full-width block at the **top** of the Phases tab, above the existing
  40/60 phase-list/pipeline split, because its header and rows are project-wide (report §3.2)
  **[inferred]**. Height = header + visible rows + fold line, capped at about 40% of the tab
  height, scrolling inside when longer. When the project has no current owner groups **and** no
  fold has content, the block takes **zero rows** and the tab renders exactly as today (keeps
  existing Phases-tab snapshots unchanged) **[inferred]**.
- **D-16:** Header: `Owner ── <alias> · <milestone> closes after: test N (ph …) · decide N (ph …)
  · unblock N (ph …) · fix N (ph …)` from the root scope's `closes_after` and `lane_phases`
  (35 D-12); zero lanes omitted; `not_built` phases appended as `· not built: ph …` when non-empty
  **[inferred]**. Blocked is worded `unblock` as in the mockup.
- **D-17:** Rows: one per current-scope group, lane order TEST (closes-a-phase first), DECIDE,
  FIX, BLOCKED as in the §3.2 mockup; within a lane by phase number. Columns: lane tag, `ph N` (or
  `debug`/`handoff` for root signals) with a `ws <name>` prefix for workstream groups (the Phases
  list's own vocabulary and Magenta cue), name, `N checks`, status phrase (`closes phase`,
  `+ fix N`, `+ unblock N`, `awaiting your confirmation`, bookkeeping-only "all checks passed, run
  … to finalize"), age (`2 d`, `stale` marker past `stale_days`), command. BLOCKED rows aggregate
  per phase by category (`2 device · 1 third party · 1 other`). Narrow widths drop command, then
  age, before truncating the name (Claude's discretion on exact breakpoints).
- **D-18:** Expansion: `Enter` on a group row toggles its items (test number, name, expected,
  why_human, reason/blocked_by, merged gaps, source file path), collapsed by default
  **[inferred]**. Expansion state is per project, in memory only (like other `ProjectViewCache`
  view state).
- **D-19:** Folds, collapsed by default, on one line: `▸ Old debt (N): …` (archived groups,
  expandable to rows grouped by milestone), `▸ Acknowledged (N files)` (expands to the
  acknowledged groups when the digest carries them, else count only), `▸ Todos N (oldest D d)`
  (count only, never expandable: todo triage is deferred; `todos_unreadable` shows `Todos ?`)
  **[inferred]**. A fold with zero content is omitted.
- **D-20:** Focus: the Owner block becomes a focus target on the Phases tab (e.g. a new
  `DetailFocus` variant). Default entry: `↑`/`k` on the first phase row moves into the block's last
  row, `↓`/`j` past its last row returns to the phase list, `Esc` returns to the phase list
  **[inferred]**; the planner confirms this does not collide with today's top-row `k` behavior and
  picks another unused key if it does. `Enter` inside the block expands (D-18), never launches:
  `v`/`o`/`A`/`y` are Phase 37. The existing verification remedy line
  (`verification_hint_spans`, detail.rs ~9455) stays unchanged.

### Help, README (OWN-23 part)
- **D-21:** Help overlay documents the `You` column (lanes, `→ closes`, `·`, `(+N old)`), the
  `has owner work` rank under `s`, `/y [lane]`, and the Owner block keys. README gains an "Owner
  inbox" section covering the dashboard and detail surfaces (the `inbox` CLI and schema are
  already documented by Phases 34-35); Phase 37 adds the inbox screen and actions to the same
  section.

### Tests and UAT
- **D-22:** Buffer snapshots at 80, 100, 120 and 140 columns (OWN-16) over a fixture fleet modeled
  on report §3.1 with synthetic names: at 80 the table is unchanged; at 100/140 the column and the
  summary line appear. SC1 test at 120: the three projects with the most closeable Test work show
  `test N → closes M` legibly. SC2 tests: blocked-only and archived-only projects show `·` /
  `· (+N old)`; summary totals equal Σ visible cells (D-11). SC3: rank-table test, `/y` filter
  tests, `needs_human` tests untouched, no snapshot under 100 columns changes. SC4: Owner block
  snapshot with header, a collapsed and an expanded group, and the three folds.
- **D-23:** Human UAT (ROADMAP, report §6): cell wording; the column at 100 and 140 columns. Kept
  to these two checks.

### Musk pass
- Milestone pass done (`.planning/musk/MILESTONE-v2.5.0.md`, `pass_due: false`). Phase pass: no
  cuts. OWN-16..19 and OWN-23 trace to the report adopted as written at the owner's request;
  tests guarding them are never cut **[inferred]**. Lean lane not used: several subsystems (reload
  path, dashboard, detail, help, README) and more than 3 tasks.

### Claude's Discretion
- Exact `You` column constraint and the Owner block row layout breakpoints.
- Whether `/yt`/`/yd`/`/yf` short forms exist alongside `/y <lane>`.
- Module split (e.g. a new `src/ui/screens/owner_block.rs` and a shared `owner_cell` formatter used
  by both the cell and the summary).
- Number and grouping of plans (coarse granularity).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and contract
- `.planning/research/OWNER-INBOX-UX.md` — §3 principle (quiet cell, no new badge), §3.1 dashboard
  mockup and rules, §3.2 Owner block mockup, §6 Phase 36 success criteria and human UAT, §7 D1,
  D4, D5, D10
- `.planning/REQUIREMENTS.md` — OWN-16..19, OWN-23
- `.planning/ROADMAP.md` §"Phase 36" — goal, success criteria 1-4, human UAT
- `README.md` — existing `inbox` section and dashboard docs

### Prior phases
- `.planning/phases/34-owner-items-reader-inbox-cli/34-CONTEXT.md` — D-15 (separate entry point),
  D-16 (cache, eviction), D-17 (safety), D-22 (counts)
- `.planning/phases/34-owner-items-reader-inbox-cli/34-VERIFICATION.md` — the FileChanged eviction
  follow-up folded here as D-04
- `.planning/phases/35-decisions-code-needed-and-gates/35-CONTEXT.md` — D-11 (decide/fix_debug
  units), D-12 (`closes_after`, `lane_phases`, `not_built`)

### Code
- `src/state_reader/owner_items/mod.rs` — `OwnerDigest`, `OwnerGroup`, `OwnerItem`, `Hidden`,
  `ClosesAfter`, `LanePhases`, `ArchivedCounts`, `read_owner_digest`, `evict_project`
- `src/state_reader/owner_items/rules.rs` — count derivation (~1086), lane/scope semantics
- `src/app.rs` — `schedule_reparse` (~1000), `Action::ProjectStateLoaded` handler (~2832),
  `FileChanged` eviction (~2736), `parse_filter` / `FilterColumn` (~63-104)
- `src/ui/screens/mod.rs` — `SortMode`, `needs_human`, `attention_rank` (~1427-1490),
  `attention_rank_for` (~1870), filter application (~1909)
- `src/ui/screens/normal.rs` — `dashboard_columns` (~426), `status_column_cells`, `summary_spans`
  / `render_normal_footer` (~1332), `s` sort key (~730)
- `src/ui/screens/detail.rs` — `render_pipeline_tab` (~7456), `DetailFocus` (~1162),
  `verification_hint_spans` (~9455), `workstream_prefix`, `shown`, `fit_cells`
- `src/ui/screens/help.rs`, `src/ui/screens/render_escape_guard.rs`
- `src/config.rs` — `Preferences::owner_stale_days`
- `src/status_cli.rs` — `owner` counts (the dashboard numbers must agree with them)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `read_owner_digest` with per-file caches and `evict_project`; counts already aggregated over
  all current-scope groups including workstreams.
- `dashboard_columns` width tiers (single source of truth for render and tests);
  `summary_spans` as an assertable span builder.
- `attention_rank` / `attention_rank_for` and the stable, alias-pinned sort.
- `parse_filter` suffix grammar and `FilterColumn::NeedsHuman` as the predicate-filter model.
- Phases tab focus model (`DetailFocus::Pane` for Waves) as the model for an Owner focus.

### Established Patterns
- Background `spawn_blocking` loads deliver via `Action`s; the render path reads caches only
  (`async_blocking_guard`).
- Non-default states announce themselves; defaults are silent (sort indicator rule) — matches the
  quiet `·` cell.
- Snapshot tests per width tier; `render_escape_guard` for every surface showing project text.

### Integration Points
- `ProjectStateLoaded` gains the digest; `AppContext` gains the digest map; dashboard cell,
  summary, rank, filter and the Owner block all read that map.

</code_context>

<specifics>
## Specific Ideas

- Report §3.1 mockup rows are the target look (`test 3 → closes 10`, `decide 1 · fix 2 ph`,
  `· (+3 old)`, summary `You: test 43 in 20 phases (17 close) · decide 9 · fix 9 phases · 11 of 20
  projects   [i] inbox`), with the fixed lane order of OWN-16 where the mockup deviates.
- Report §3.2 mockup is the Owner block target, including `unblock` wording in the header.

</specifics>

<deferred>
## Deferred Ideas

- Phase 37: inbox screen behind `i`, `v` run, `A` acknowledge, `y` copy checklist, `o` open file
  (also from the Owner block), launching the row command from the detail view.
- Report §6 deferred list (OWN-F01..): todo triage UI beyond a count, notifications, daily digest,
  and the rest.
- 34 VERIFICATION follow-ups still out of scope: IN-02 id collision after truncation, IN-04
  byte-bounded cache.

### Reviewed Todos (not folded)
`todo.match-phase 36` returned the same 17 keyword-only matches Phases 34 and 35 reviewed (score
0.2-0.9: multi-line queue input, ambiguous-wide typography, TUI badge for release maps, help popup
at 80x24, launcher-marker in-flight flag, unregister guard, registry glyph overflow, D-17 recovery
block, executor remnants, driverctl LOG.md redaction, capture-pump redaction, run-lock hard link,
managed-policy shapes, run-file hardening, unsupervised child, driverctl `fm_status` IN-06,
sibling-run tmpfs). None touches the owner cell, summary, rank, filter or Owner block, so none is
folded despite the ≥ 0.4 auto-fold rule; folding them would be scope creep **[inferred]**.

</deferred>

---

*Phase: 36-dashboard-and-detail*
*Context gathered: 2026-11-14*
