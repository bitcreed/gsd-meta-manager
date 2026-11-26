# Phase 36: Dashboard and detail - Research

**Researched:** 2026-11-14
**Domain:** Rust ratatui 0.30 TUI: dashboard table column/summary, sort/filter predicates, detail-view focus level, background digest loading
**Confidence:** HIGH (every claim below was checked against live source on `feat/v2.5.0-owner-inbox`, HEAD `d36f6733`; no external library work)

## Summary

Phase 36 is pure in-repo integration work: no new crate, no new reader rule. The owner digest
(`read_owner_digest`, Phases 34-35) already carries every number the dashboard needs
(`test`, `decide`, `fix_phases`, `fix_debug`, `blocked`, `closeable_phases`, `archived.*`) and every
group the Owner block lists. The work is (1) getting that digest into `AppContext` off the render
path, (2) a `>= 100` dashboard tier with a `You` cell and a `You:` summary row, (3) a 4-rank
`attention_rank` and a `/y [lane]` filter predicate, (4) an Owner block plus a new `DetailFocus`
level on the Phases tab, and (5) help/README copy. `36-UI-SPEC.md` is an approved, very detailed
contract; this research verifies its assumptions against the code and records where the code
disagrees with CONTEXT/UI-SPEC.

Five findings change the plan materially. **First**, at startup no digest would ever arrive: the
initial `load_project_states()` parses synchronously on the main thread and never goes through
`schedule_reparse`/`ProjectStateLoaded`, and `event_tx` is set only after it; the same is true of
auto-register, create and add-project. D-01's "reparse task only" therefore needs a startup and
registration dispatch next to it, or every `You` cell stays blank until a file changes. **Second**,
unreadable UAT/VERIFICATION files are counted as **Fix** work (`fix_phases`), not Test work as D-09
and the UI-SPEC state; the fail-safe still holds, only the lane word differs. **Third**, digest
strings are **raw**: escaping happens at emit time in `inbox.rs` (`terminal_capped`), not in the
reader, so the TUI must escape every digest string itself. **Fourth**, the existing test
`attention_first_is_stable_across_projects_of_equal_rank` hard-codes ranks 1 and 2 and must be
edited (it is a rank test, not a `needs_human` test). **Fifth**, a new alias-keyed `AppContext` map
is caught by six struct literals and one exhaustive destructure, and must be pruned.

**Primary recommendation:** Build one pure presentation module (`OwnerNumbers` + cell/summary/rank/
filter predicates) consumed by both dashboard and summary, deliver digests through the reparse task
plus a first-fill-only startup/registration dispatcher, and reuse `inbox.rs`'s wording helpers
(made `pub(crate)`) for the Owner block so CLI and TUI never diverge.

## User Constraints (from CONTEXT.md)

<user_constraints>

### Locked Decisions

#### Carried forward (not re-decided)
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

#### Loading and cost
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

#### Dashboard `You` cell (OWN-16)
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

#### Summary line (OWN-17)
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

#### Sort and filter (OWN-18)
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

#### Detail: Owner block on the Phases tab (OWN-19)
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

#### Help, README (OWN-23 part)
- **D-21:** Help overlay documents the `You` column (lanes, `→ closes`, `·`, `(+N old)`), the
  `has owner work` rank under `s`, `/y [lane]`, and the Owner block keys. README gains an "Owner
  inbox" section covering the dashboard and detail surfaces (the `inbox` CLI and schema are
  already documented by Phases 34-35); Phase 37 adds the inbox screen and actions to the same
  section.

#### Tests and UAT
- **D-22:** Buffer snapshots at 80, 100, 120 and 140 columns (OWN-16) over a fixture fleet modeled
  on report §3.1 with synthetic names: at 80 the table is unchanged; at 100/140 the column and the
  summary line appear. SC1 test at 120: the three projects with the most closeable Test work show
  `test N → closes M` legibly. SC2 tests: blocked-only and archived-only projects show `·` /
  `· (+N old)`; summary totals equal Σ visible cells (D-11). SC3: rank-table test, `/y` filter
  tests, `needs_human` tests untouched, no snapshot under 100 columns changes. SC4: Owner block
  snapshot with header, a collapsed and an expanded group, and the three folds.
- **D-23:** Human UAT (ROADMAP, report §6): cell wording; the column at 100 and 140 columns. Kept
  to these two checks.

#### Musk pass
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

### Deferred Ideas (OUT OF SCOPE)
- Phase 37: inbox screen behind `i`, `v` run, `A` acknowledge, `y` copy checklist, `o` open file
  (also from the Owner block), launching the row command from the detail view.
- Report §6 deferred list (OWN-F01..): todo triage UI beyond a count, notifications, daily digest,
  and the rest.
- 34 VERIFICATION follow-ups still out of scope: IN-02 id collision after truncation, IN-04
  byte-bounded cache.

#### Reviewed Todos (not folded)
`todo.match-phase 36` returned the same 17 keyword-only matches Phases 34 and 35 reviewed (score
0.2-0.9: multi-line queue input, ambiguous-wide typography, TUI badge for release maps, help popup
at 80x24, launcher-marker in-flight flag, unregister guard, registry glyph overflow, D-17 recovery
block, executor remnants, driverctl LOG.md redaction, capture-pump redaction, run-lock hard link,
managed-policy shapes, run-file hardening, unsupervised child, driverctl `fm_status` IN-06,
sibling-run tmpfs). None touches the owner cell, summary, rank, filter or Owner block, so none is
folded despite the ≥ 0.4 auto-fold rule; folding them would be scope creep **[inferred]**.

</user_constraints>

**The UI design contract `36-UI-SPEC.md` (approved 2026-11-14) is binding for copy, geometry, colors
and keys.** This research does not restate it; it only corrects or sharpens it where the code says
otherwise (see "Corrections to CONTEXT / UI-SPEC").

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OWN-16 | Dashboard `You` column: ≤ 2 lanes in fixed order `decide`, `test`, `fix`; never `blocked`; current milestone only, archived debt as dim `(+N old)`; hidden below 100 columns; dim `·` when clean. Test: buffer snapshots at 80/100/140 | Pattern 2 (tier), Pattern 3 (cell), Pitfalls 1, 6, 7, 8; startup delivery (Pattern 1) is what makes the cell non-blank |
| OWN-17 | Summary line shows totals and `[i] inbox`; totals equal the sum of the visible cells | Pattern 3 (`OwnerNumbers` shared), Pattern 4 (summary row layout), Pitfall 9 (height gate) |
| OWN-18 | AttentionFirst gains `has owner work` rank between "needs a human" and "driven"; `/y [lane]` filters; `needs_human` and `❢` unchanged. Test: rank table; existing needs_human tests untouched | Pattern 5, Pitfall 4 (the one existing rank test that must change) |
| OWN-19 | Detail Phases tab Owner block (no new tab) with expandable groups, `closes after` header, collapsed Old debt / Acknowledged / Todos folds | Patterns 6-7, Pitfalls 10-12, "Don't Hand-Roll" (reuse `inbox.rs` wording) |
| OWN-23 (TUI half) | Help overlay and README "Owner inbox" section | Pattern 8; help test conventions |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

No `AGENTS.md` exists in the worktree (checked: `cat AGENTS.md` failed). Directives from
`./CLAUDE.md` and the user's global instructions that bind this phase:

- **State reading must not require running Claude/GSD**; **non-intrusive** (never write into a
  project). The Owner block and dashboard only read the digest; no write, no launch (D-20).
- **Stack is fixed:** Rust 1.89+ MSRV (`Cargo.toml` `rust-version = "1.89"`), ratatui 0.30
  (`features = ["crossterm", "unstable-rendered-line-info"]`), crossterm 0.29, tokio. No new crate.
- **Never block the render loop; all I/O goes through tokio channels.** Enforced for
  `read_owner_digest(` by `tests/async_blocking_guard.rs` (`BLOCKING_HELPERS`).
- **Render is driven by state; state is mutated only by message handlers** (`Action` handlers).
- **Logging goes to a file** (`tracing`), never stdout.
- **GSD workflow enforcement:** edits happen inside `/gsd:execute-phase`.
- **US spelling; never singular "they" for a person** (the owner is "he"; README copy addresses "you").
- **Conventional commits, path-scoped `git commit -- <paths>`, never `git add -A`.** (Researcher does
  not commit; orchestrator does.)
- **Respect env vars over hardcoded paths** (`XDG_*`, `CLAUDE_CONFIG_DIR`): nothing new here touches paths.
- **Release process** (`./scripts/pre-tag-check.sh --container`) is milestone-end only, not this phase.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Computing the owner digest | Background blocking task (`spawn_blocking`) | Reader (`state_reader::owner_items`) | File I/O and parsing; banned from render path and `async fn` (OWN-12, guard test) |
| Holding digests | `AppContext` (in-memory state, alias-keyed map) | — | D-02; must not live on `ProjectState` (its `PartialEq` drives the "Updated:" message and is compared by driver/next/router) |
| Per-project numbers, has-owner-work, lane predicates | Pure presentation module (new) | — | One function feeds cell, summary, rank and filter (D-11, D-13, D-14) |
| `You` cell + `You:` summary row | Dashboard screen (`normal.rs`) | Pure module | Rendering only, reads the map per frame |
| Rank and filter | `AppContext::sorted_aliases` / `recompute_filtered_aliases` (`screens/mod.rs`), grammar in `app::parse_filter` | Pure module | Existing single sites for sort/filter |
| Owner block render, focus, expansion | Detail screen (`detail.rs` or a new sibling module) | `ProjectViewCache` (view state) | Mirrors the Waves pane (`DetailFocus::Pane` + `waves_toggles`) |
| Group wording (status phrases, blocked tally, closes-after clauses) | `crate::inbox` (lib module, already the CLI's) | — | One wording source for CLI, Owner block and Phase 37 |

## Standard Stack

No new dependencies. Everything is already in `Cargo.toml` and in use.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 (`Cargo.toml:38`) | `Table`, `Layout`, `Paragraph`, `Line`/`Span`, `TestBackend` | The project's TUI library; every dashboard/detail test uses `TestBackend` [VERIFIED: Cargo.toml:38] |
| crossterm | 0.29 (`Cargo.toml:39`) | `KeyCode` for the new key arms | Existing backend [VERIFIED: Cargo.toml:39] |
| tokio | 1.x | `spawn_blocking` for the digest | Existing reparse pattern [VERIFIED: src/app.rs:1009] |
| chrono | (existing) | `Utc::now()` passed into `read_owner_digest` | Reader takes `now` from the caller (34 D-14) [VERIFIED: src/state_reader/owner_items/mod.rs:559-564] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled buffer assertions | `insta` snapshots | Not a dependency (UI-SPEC: "no `insta` in `Cargo.toml`"); the codebase convention is `TestBackend` + cell/text helpers. Do not add it. |

**Installation:** none.

## Package Legitimacy Audit

Not applicable: this phase installs no external package (no `Cargo.toml` change). No
`package-legitimacy check` was run because there is nothing to check.

## Architecture Patterns

### System Architecture Diagram

```
 startup (main.rs)                       watcher (.planning/ change)
   load_project_states() [sync]            Action::FileChanged
   event_tx = Some(..)                       ├─ owner_items::evict_project(path)   (already wired)
   dispatch_owner_digests()  ──┐             └─ ChangeKind::Planning + 500 ms dedup
 add / auto-register / create ─┤                  └─ schedule_reparse(alias, path)
                               │                        spawn_blocking:
                               │                          state  = parse_project_state()
                               │                          digest = read_owner_digest(state, now, stale_days)
                               ▼                          send ProjectStateLoaded{alias, state, digest}
              spawn_blocking: read_owner_digest(clone of state)
              send OwnerDigestLoaded{alias, digest}  (first fill only)
                               │                                  │
                               └──────────────┬───────────────────┘
                                              ▼
                         App::update handler: ctx.owner_digests[alias] = digest
                         if state or digest changed → recompute_filtered_aliases(); needs_redraw
                         ("Updated: alias" only when the STATE changed)
                                              │
                ┌─────────────────────────────┼────────────────────────────────┐
                ▼                             ▼                                ▼
   sorted_aliases(): attention_rank   recompute_filtered_aliases():   render (per frame, read-only)
   (needs_human, owner, driven)       FilterColumn::OwnerWork(lane)    ├─ dashboard >=100: You cell,
                                                                       │   You: summary over filtered_aliases
                                                                       └─ detail Phases tab: Owner block
                                                                           (header, group rows, folds)
                                                                           focus = DetailFocus::Owner
```

### Recommended Module Layout

```
src/ui/screens/owner.rs        # NEW, pure: OwnerNumbers, owner_numbers(&OwnerDigest), has_owner_work,
                               #   OwnerLane {Test, Decide, Fix}, you_cell_line(nums, cells) -> Line,
                               #   summary_line_spans(ctx/visible digests, width) -> (left, right)
src/ui/screens/owner_block.rs  # NEW: Owner block model (rows/stops from a digest), render fn,
                               #   key handling helper, OwnerBlockState (lives in ProjectViewCache)
src/inbox.rs                   # make wording helpers pub(crate): plural, blocked_tally, detail,
                               #   closes-after clause builder (extracted from push_closes_after)
src/app.rs                     # FilterColumn::OwnerWork, parse_filter, schedule_reparse, handlers,
                               #   dispatch_owner_digests, prune
src/action.rs                  # ProjectStateLoaded gains `digest`; new OwnerDigestLoaded
src/ui/screens/mod.rs          # AppContext.owner_digests, attention_rank(3 args), filter arm,
                               #   ProjectViewCache owner state
src/ui/screens/normal.rs       # >=100 tier, You cell, You: row, `i` key
src/ui/screens/detail.rs       # DetailFocus::Owner, ↑ entry, block in render_pipeline_tab, footer
src/ui/screens/help.rs, README.md
```

Registering the new files: `src/ui/screens/mod.rs` declares screen submodules; add `mod owner;` and
`mod owner_block;` there (neither implements `Screen`, so `render_escape_guard`'s screen census is
unaffected).

### Pattern 1: Digest delivery (reparse path + first-fill startup path)

**What:** The watcher path follows D-01 exactly; a second, digest-only dispatcher covers startup and
registration. It never calls `parse_project_state`, so it never touches `reparse_dispatches`.

The current reparse body [VERIFIED: src/app.rs:1000-1015]:
```rust
    fn schedule_reparse(&mut self, alias: &str, project_path: &Path) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let planning_dir = project_path.join(".planning");

        self.ctx.reparse_dispatches += 1;
        tokio::task::spawn_blocking(move || {
            let state = state_reader::parse_project_state(&planning_dir);
            let _ = tx.send(Action::ProjectStateLoaded {
                alias: alias_for_task,
                state: Box::new(state),
            });
        });
```
The variant today [VERIFIED: src/action.rs:45-49]:
```rust
    ProjectStateLoaded {
        alias: String,
        // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
        state: Box<ProjectState>,
    },
```
The staleness preference [VERIFIED: src/config.rs:546-553]: `pub fn owner_stale_days(&self) -> u32`
reading `extra["owner"]["stale_days"]`, default `30`, clamped `1..=3650`.

Recommended shape [ASSUMED: design, consistent with D-01/D-02]:
```rust
// app.rs, schedule_reparse: read stale_days BEFORE the spawn (config is not moved in)
let stale_days = self.ctx.config.preferences.owner_stale_days();
tokio::task::spawn_blocking(move || {
    let state = state_reader::parse_project_state(&planning_dir);
    let digest = state_reader::owner_items::read_owner_digest(
        &planning_dir, &state, chrono::Utc::now(), stale_days);
    let _ = tx.send(Action::ProjectStateLoaded {
        alias: alias_for_task,
        state: Box::new(state),
        digest: Box::new(digest),          // boxed for clippy::large_enum_variant
    });
});

// AppContext (callable from screens such as add_project.rs, which only hold ctx):
pub fn schedule_owner_digest(&self, alias: &str, project_path: &Path) {
    let (Some(tx), Some(state)) = (&self.event_tx, self.project_states.get(alias)) else { return };
    let (tx, alias, state) = (tx.clone(), alias.to_string(), state.clone());
    let planning_dir = project_path.join(".planning");
    let stale_days = self.config.preferences.owner_stale_days();
    tokio::task::spawn_blocking(move || {
        let digest = read_owner_digest(&planning_dir, &state, chrono::Utc::now(), stale_days);
        let _ = tx.send(Action::OwnerDigestLoaded { alias, digest: Box::new(digest) });
    });
}
// Handler: insert ONLY if the alias has no digest yet (a reparse result is always newer).
```
**Call sites for the first-fill dispatcher** (all verified this session):
- startup, in `main.rs` right after `app.ctx.event_tx = Some(event_bus.tx.clone());`
  [VERIFIED: src/main.rs:527-538: `app.load_project_states();` … `app.ctx.event_tx = Some(event_bus.tx.clone());`];
- auto-register [VERIFIED: src/app.rs:2567-2569: `let state = state_reader::parse_project_state(&planning_dir);` … `self.ctx.project_states.insert(alias.clone(), state);`];
- `Action::CreateProjectResult` success [VERIFIED: src/app.rs:2960-2961];
- add-project screen [VERIFIED: src/ui/screens/add_project.rs:205-206].
- Queue mutations (`queue_delete_confirm.rs:60-61`, `detail.rs:3761-3762`) re-insert state but write
  `QUEUE.md` under `.planning/`, so the watcher's reparse refreshes the digest; no call needed there.

**When to use:** every place a `ProjectState` is inserted without a following reparse.

### Pattern 2: The `>= 100` dashboard tier

Today's top tier [VERIFIED: src/ui/screens/normal.rs:439-450]:
```rust
fn dashboard_columns(terminal_width: u16) -> (Vec<&'static str>, Vec<Constraint>) {
    if terminal_width >= 80 {
        (
            vec!["Alias", "Phase", "Status", "Progress", "Backlog"],
            vec![
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Min(STATUS_COLUMN_MIN_CELLS),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
```
`const STATUS_COLUMN_MIN_CELLS: u16 = 13;` [VERIFIED: normal.rs:423];
`const DASHBOARD_STATUS_COLUMN: usize = 2;` [VERIFIED: normal.rs:500].

Add a `terminal_width >= 100` branch **before** it with UI-SPEC's vector
(`Percentage(20), Percentage(26), Min(13), Length(21), Percentage(12), Percentage(9)`, one
`YOU_COLUMN_CELLS = 21` constant). The resolved widths (Status 13 at 100 and 120, 39 at 200) come
from the UI-SPEC's probe [CITED: 36-UI-SPEC.md "Resolved column widths"]; the planner re-asserts
them with `Layout::horizontal` exactly as `status_column_cells` does (normal.rs:486-497).

**The cell vector is a second, separate site.** `render_main` builds cells with its own
`if terminal_width >= 80 { …5 cells… } else if >= 60 …` [VERIFIED: normal.rs:1200-1221], independent
of `dashboard_columns`. Both must gain the `>= 100` branch; the `You` cell goes at index 3.

### Pattern 3: One `OwnerNumbers` function for cell, summary, rank, filter

Digest fields [VERIFIED: src/state_reader/owner_items/mod.rs:537-553]:
```rust
pub struct OwnerDigest {
    pub groups: Vec<OwnerGroup>,
    pub test: usize,
    pub decide: usize,
    pub fix_phases: usize,
    pub fix_debug: usize,
    pub blocked: usize,
    pub closeable_phases: usize,
    pub archived: ArchivedCounts,
    pub superseded: usize,
    pub hidden: Hidden,
    pub closes_after: ClosesAfter,
}
```
`ArchivedCounts { test, blocked, fix, decide }` [VERIFIED: mod.rs:504-510];
`Hidden { deferred, acknowledged_files, noise, todos, todos_oldest_days, todos_unreadable }`
[VERIFIED: mod.rs:491-501]. `OwnerDigest` derives `Debug, Clone, Default, PartialEq`
[VERIFIED: mod.rs:536], so D-02's "compare old and new digest" is a plain `!=`.

Recommended pure core [ASSUMED: design]:
```rust
pub(crate) struct OwnerNumbers { decide: usize, test: usize, fix: usize, fix_is_phases: bool,
                                 closeable: usize, old: usize, test_phases: BTreeSet<(Option<String>, String)> }
pub(crate) fn owner_numbers(d: &OwnerDigest) -> OwnerNumbers {
    // fix = fix_phases + fix_debug; fix_is_phases = fix_debug == 0
    // old = archived.test + archived.fix + archived.decide            (D-08; blocked excluded)
    // test_phases = {(workstream, phase_key(phase)) | current, !acknowledged, lane Test, !items.is_empty()}
}
pub(crate) fn has_owner_work(n: &OwnerNumbers) -> bool { n.decide + n.test + n.fix > 0 }
```
`phase_key` is `pub fn phase_key(id: &str) -> String` [VERIFIED: src/state_reader/phase_num.rs:169].
The cell ladder follows `agent_summary_line` (normal.rs:519-530: first form whose
`Line::from(form).width() <= cells`).

### Pattern 4: The `You:` summary row

`NormalScreen::render` splits `[Constraint::Min(0), Constraint::Length(1)]` [VERIFIED:
normal.rs:866]. At `area.width >= 100 && area.height >= 12` use `[Min(0), Length(1), Length(1)]`
and draw the summary into the middle row; otherwise keep the old split byte-identical. The footer's
status-message branch (normal.rs:1268-1320) is untouched, because the summary is its own row. The
right part `[i] inbox` uses the `render_normal_footer` idiom (normal.rs:1423-1432:
`Layout::horizontal([Constraint::Min(0), Constraint::Length(right_len + 1)])`).

"Visible rows" = `ctx.filtered_aliases` (the same list `render_main` iterates, normal.rs:1037-1040).

### Pattern 5: Rank and filter

Today [VERIFIED: src/ui/screens/mod.rs:1483-1491]:
```rust
pub fn attention_rank(needs_human: bool, driven_and_live: bool) -> u8 {
    if needs_human {
        0
    } else if driven_and_live {
        1
    } else {
        2
    }
}
```
Its only production caller is `attention_rank_for` (mod.rs:1870-1878), used by `sorted_aliases`
(mod.rs:1576-1583, `aliases.sort_by_key(|a| self.attention_rank_for(a))`, a stable sort after the
alphabetical one). New: `attention_rank(needs_human, has_owner_work, driven_and_live) -> u8` returning
0/1/2/3. The owner rank is **not** gated on `ctx.experimental` (the driven rank is).

Filter grammar today [VERIFIED: src/app.rs:62-75, 92-104]:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FilterColumn {
    All,
    Name,
    Phase,
    Status,
    NeedsHuman,
}
pub fn parse_filter(input: &str) -> (String, FilterColumn) {
    if let Some(term) = input.strip_suffix("/p") {
        (term.to_string(), FilterColumn::Phase)
    } else if let Some(term) = input.strip_suffix("/n") {
        (term.to_string(), FilterColumn::Name)
    } else if let Some(term) = input.strip_suffix("/s") {
        (term.to_string(), FilterColumn::Status)
    } else if let Some(term) = input.strip_suffix("/h") {
        (term.to_string(), FilterColumn::NeedsHuman)
    } else {
        (input.to_string(), FilterColumn::All)
    }
}
```
Add `OwnerWork(Option<OwnerLane>)` with `OwnerLane { Test, Decide, Fix }` (a **new** 3-variant enum,
not `owner_items::Lane`, whose variants are `Test, Decide, Fix, Blocked, Deferred` [VERIFIED:
owner_items/mod.rs:373-379]; reusing it would make `/y blocked` representable). Check the long suffixes
first: `"/y test"`, `"/y decide"`, `"/y fix"`, `"/yt"`, `"/yd"`, `"/yf"`, then `"/y"`. The filter arm
mirrors the `NeedsHuman` arm: `matches_any_column() && owner_lane_matches(alias, lane)` (mod.rs:1955-1957).
A project with no digest never matches.

### Pattern 6: `DetailFocus::Owner`, modeled on `Pane`

Today [VERIFIED: src/ui/screens/detail.rs:1161-1181]: `enum DetailFocus { TabBar, Content (#[default]), Pane, ScopeStrip }`.
Dispatch site [VERIFIED: detail.rs:3939-3945]:
```rust
        if self.focus == DetailFocus::Pane {
            if current_view != DetailSubView::Pipeline {
                self.focus = DetailFocus::Content;
            } else if let Some(action) = self.handle_waves_pane_key(code, ctx) {
                return action;
            }
        }
```
Add an identical block for `Owner` calling `handle_owner_block_key` (same `Option<ScreenAction>`
contract as `handle_waves_pane_key`, detail.rs:2060-2098).

Entry point [VERIFIED: detail.rs:4255-4266]:
```rust
            KeyCode::Char('k') | KeyCode::Up => {
                if self.content_at_first_row(&current_view, ctx) {
                    self.focus = if current_view == DetailSubView::Defaults {
                        DetailFocus::ScopeStrip
                    } else {
                        DetailFocus::TabBar
                    };
```
Insert: `Pipeline` and the last frame drew a non-empty Owner block → `DetailFocus::Owner` with the
cursor on the last stop. `content_at_first_row`'s Pipeline arm is
`DetailSubView::Pipeline => cache.is_none_or(|c| c.pipeline_selected == 0),` [VERIFIED: detail.rs:2187].

"Did the last frame draw a block" must be a `Cell` on `DetailScreen` written by the render (the
`waves_viewport_rows: Cell<..>` precedent, detail.rs:1205). Key tests that never render see `0` and
keep today's `↑ → TabBar` behavior, which is exactly the UI-SPEC "no block" test.

The wheel cannot accidentally enter the block: `if !down && self.content_at_first_row(current_view, ctx) { return ScreenAction::None; }`
("The wheel never climbs to the tab bar") [VERIFIED: detail.rs:6431-6434] returns before `handle_key`.

### Pattern 7: Owner block placement and state

`render_pipeline_tab(&self, frame, area, ctx)` [VERIFIED: detail.rs:7456] has two early returns
(no state; `entries.is_empty()` → "No phases found", detail.rs:7461-7490) and a Waves-pane full-tab
branch under 100 columns (detail.rs:7498-7522), then the 40/60 split (detail.rs:7525-7526). Compute
the block height first, split `area` into `[Length(h), Min(0)]`, draw the block, and pass the remainder
as `area` to the unchanged code (including the "No phases found" branch, per UI-SPEC). With `h == 0`
the remainder is `area` itself, so the existing Phases-tab tests stay byte-identical.

State model: `ProjectViewCache` already holds `waves_cursor: Option<WavesCursor>` (an identity, not an
index) and `waves_toggles: HashSet<(String, Option<u32>)>` [VERIFIED: src/ui/screens/mod.rs:1022-1032].
Add an `OwnerBlockState { expanded: HashSet<GroupKey>, open_folds, cursor: Option<StopKey>, scroll }`
the same way; `ProjectViewCache` is `#[derive(Default)]` (mod.rs:986-987) and its test literals use
`..ProjectViewCache::default()` (e.g. driver_run.rs:5409-5419), so a new field breaks nothing.

Footer: the content footer's lead is chosen here [VERIFIED: detail.rs:11335-11339]:
```rust
    let up = if matches!(sub_view, DetailSubView::Defaults) {
        "scope  "
    } else {
        "tab bar  "
    };
```
`footer_spans` has three test call sites (detail.rs:18233, 26921, 26932); add a parameter (or a
wrapper) for "Owner block present" → `owner  `. The focus-dependent footer chain is at
detail.rs:6305-6318; add an `Owner` arm calling a new `owner_block_footer_spans()` modeled on
`waves_pane_footer_spans` (detail.rs:11466-11483).

### Pattern 8: Help rows and README

`fn row(key: &str, description: &str) -> Line<'static> { Line::from(format!("  {key:<13} {description}")) }`
[VERIFIED: src/ui/screens/help.rs:196-198]. The current `s` row is
`row("s", "Toggle sort: alphabetical / attention first"),` [VERIFIED: help.rs:228] and README
line 258 carries the same text. Help tests assert **whole rows** built with `row()` and require each
description to be unique (help.rs:678-690). The "Filter Syntax" block ends with `row("//h", …)`
(help.rs:449-457) followed by "Dashboard Badges" (help.rs:460); a test checks only that both headings
exist and that "Work State" follows "Dashboard Badges" (help.rs:816-829), so inserting an "Owner Work"
section between Filter Syntax and Dashboard Badges is safe.

### Anti-Patterns to Avoid
- **Computing the digest (or `phase_key`-ing groups) in `render`.** The render reads
  `ctx.owner_digests` only; `OwnerNumbers` may be derived per frame from the in-memory digest (cheap,
  no I/O), but nothing on the render path may stat or read a file.
- **Putting the digest on `ProjectState`.** D-02; also `ProjectState`'s `PartialEq` drives the
  "Updated: alias" message (mod.rs:1348-1353 doc on `agent_views` explains the flood this causes).
- **Spelling `▸`/`▾` as literals under `src/ui/`.** Use `glyphs::TRIANGLE_RIGHT_SMALL` /
  `TRIANGLE_DOWN_SMALL` (glyphs.rs:88, 91); the registry source scan fails otherwise.
- **A bare `Span::raw` in the `You` cell.** The row carries `Style::default().fg(row_color)`
  (normal.rs:1223-1225); unset span fg inherits Red/Yellow/Green. Set `fg(Color::Reset)` explicitly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Group status phrase (`closes phase`, `awaiting your confirmation`, `N failed checks, M gaps`, `N unreadable checks`, handoff/checkpoint/question phrases) | New TUI wording | `inbox::detail(group)` (inbox.rs:290-381), made `pub(crate)` | CLI golden output already pins it; Phase 37 needs the same |
| Blocked tally `2 device · 1 third party · 1 other` | New tally | `inbox::blocked_tally` (inbox.rs:260-288) | Same category order (`server, device, build, third party, blocked`, then `other`) |
| `1 check` / `N checks` | `if n == 1` everywhere | `inbox::plural` (inbox.rs:255-257) | One pluralizer |
| `closes after:` lane clauses (`unblock` wording, `no owner work (ph …)`) | New header builder | Extract the clause list from `inbox::push_closes_after` (inbox.rs:504-525) into a `pub(crate)` fn returning `Vec<String>` | Keeps lane order test/decide/unblock/fix identical to the CLI |
| Which groups are "Old debt" / "Acknowledged" | New predicates | `inbox::Filters::lists_group` (inbox.rs:69-77) plus `Scope::Archived` / `acknowledged`, as `render_text` does (inbox.rs:619-629) | Deferred-lane and acknowledged rules already decided there |
| Row order inside a lane | New comparator | `by_location` semantics (inbox.rs:473-479) / the digest's own `sort_key` order | Digest groups are already sorted by `sort_key` (rules.rs:1080) |
| Escaping | Custom filtering | `shown()` (detail.rs:81-83) / `crate::text::render_for_terminal` | Census and `render_escape_guard` know these |
| Truncation | `&s[..n]`, `len()` | `fit_cells(text, cols)` (detail.rs:10797, `pub(super)`) and `Line::width()` | Multi-byte `·` `→` `▸` |
| Stale computation | `age > stale_days` in the UI | `group.stale`, `group.age_days` | Reader already applied `stale_days` (rules.rs:428) |

**Key insight:** every wording and predicate the Owner block needs already exists in `inbox.rs` for
the CLI. Re-spelling any of it creates a second truth that Phase 37's inbox screen would have to pick
between.

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted.

## Corrections to CONTEXT / UI-SPEC (verified against code)

1. **Unreadable files are Fix work, not Test work** (contradicts D-09 and UI-SPEC "Unreadable files:
   already counted as Test work … appear as `test N`"). The reader puts the unreadable count only on
   the Fix group: `let group_unreadable = if lane == Lane::Fix { unreadable } else { 0 };`
   [VERIFIED: src/state_reader/owner_items/rules.rs:1062-1068], pinned by the test
   `unparsed_blocks_and_unreadable_files_make_a_fix_group_without_a_command` (rules.rs:1255-1276).
   Any current Fix group counts into `fix_phases` (rules.rs:1100-1102). An unreadable CONTEXT file is
   Decide work (`group.items.len() + group.counts.unreadable`, rules.rs:1093-1097); an unreadable debug
   session is `fix_debug` (rules.rs:1099). **Consequence:** the fail-safe (non-`·`) holds, but the cell
   reads `fix N ph`, the Owner block row is FIX with `N unreadable check(s)` (inbox.rs:367-372), and the
   help/README must not say unreadable files show as `test`.
2. **Digest strings are not escaped by the reader** (contradicts the carried-forward bullet "already
   capped/escaped by the reader"). `terminal_capped` appears nowhere under `src/state_reader/owner_items/`
   (only `sanitize_for_display` for CONTEXT questions, context.rs:118); `inbox.rs` escapes at emit time
   (e.g. `"name": terminal_capped(&item.name)`, inbox.rs:106). The TUI must pass every digest string
   (phase, phase_name, workstream, file, file_status, command, item fields, archived label) through
   `shown()`. Escaping is idempotent on already-escaped text (the escape output is printable ASCII such as
   `U+202E`, text.rs:305-315), so feeding `inbox::detail()` output (already `terminal_capped`) through
   `shown()` again is harmless.
3. **No digest arrives at startup under D-01 alone.** See Pattern 1. `load_project_states` is
   synchronous and sends no action [VERIFIED: src/app.rs:2458-2465]; it is the only caller pattern at
   startup [VERIFIED: src/main.rs:527].
4. **`not built` wording differs.** The CLI prints `not built ph {list}` without a colon
   [VERIFIED: src/inbox.rs:523-525: `parts.push(format!("not built ph {}", phase_list(&gate.not_built)));`];
   D-16/UI-SPEC say `· not built: ph …`. Recommendation [ASSUMED]: when extracting the shared clause
   builder, keep the CLI's text (its golden tests constrain it) and let the TUI use the same string; this
   is a one-colon UAT-level difference, not a contract change.
5. **Workstream vocabulary.** The Phases list prefix is the milestone tag (`ws v0.14 `), built by
   `workstream_prefix` from `crate::app::workstream_tag(ws)` [VERIFIED: detail.rs:9140-9144], while
   `OwnerGroup.workstream` is the **directory name** (`/// The workstream directory name`, mod.rs:158 on
   `AuditFile`). UI-SPEC's `ws {name}` uses the directory name, which is what the row's `--ws` command
   carries; keep it, but expect the same workstream to read `ws v0.14` in the list below and
   `ws v0-14-voice` in the block above. Flag for UAT, not a blocker.
6. **Bookkeeping-only groups count zero.** `apply_r7` adds Test groups with no items and
   `bookkeeping_only = true` (rules.rs:563-579); `digest.test` sums `items.len()` (rules.rs:1086). A
   project whose only owner item is "all checks passed, run … to finalize" therefore shows `·` and has
   no owner-work rank, while its Owner block shows a TEST row. Consistent with `status --json`; record
   it in help as-is. `P` must count only Test groups with at least one item (otherwise a bookkeeping
   phase inflates "in P phases").

## Common Pitfalls

### Pitfall 1: "Updated: alias" flood on digest-only changes
**What goes wrong:** D-02 makes a digest-only change "changed"; reusing the existing branch also sets
`status_message = "Updated: {alias}"` [VERIFIED: src/app.rs:2848-2852]. With the startup fan-out this
fires for every project and overwrites the one startup status line (`announce_gsd_install`).
**How to avoid:** compute `state_changed` and `digest_changed` separately; re-filter and redraw on
either; set the status message only on `state_changed`.

### Pitfall 2: Stale startup digest overwriting a newer one
**What goes wrong:** a first-fill task started at launch can finish after a watcher reparse and
replace a newer digest. **How to avoid:** the `OwnerDigestLoaded` handler inserts only when the alias
has no digest (`entry(alias).or_insert`); `ProjectStateLoaded` always replaces.

### Pitfall 3: Every `AppContext` literal breaks
**What goes wrong:** a new field fails to compile at six struct literals: app.rs:932, detail.rs:20036,
driver_confirm.rs:~409, mod.rs:~2410, delete_confirm.rs:~240, normal.rs:~1660 (all found via
`reparse_dispatches: 0,` at app.rs:945, detail.rs:20049, driver_confirm.rs:421, mod.rs:2427,
delete_confirm.rs:256, normal.rs:1676), plus the exhaustive destructure in
`pruning_drops_the_view_cache_for_an_unregistered_alias` (app.rs:5783-5822), which says "Destructured
exhaustively so a new `AppContext` field is a compile error here until it has been classified."
**How to avoid:** add `owner_digests: HashMap::new()` to each literal; classify it in the destructure
as **alias-keyed and pruned**; add `self.ctx.owner_digests.retain(|alias, _| registered.contains_key(alias));`
to `prune_driver_maps` (app.rs:3574-3605) and an assertion in that test.

### Pitfall 4: The one existing rank test that must change
**What goes wrong:** `attention_first_is_stable_across_projects_of_equal_rank` ends with
`assert_eq!(attention_rank(false, true), 1);` and `assert_eq!(attention_rank(false, false), 2);`
[VERIFIED: src/ui/screens/mod.rs:2872-2879]. A 3-argument, 4-rank function cannot keep these.
**How to avoid:** update only those three `attention_rank(..)` calls (new signature, new values);
its `sorted_aliases()` assertion (`["bravo", "delta", "charlie", "alpha"]`, mod.rs:2864-2870) has no
digests and stays byte-identical. This is a rank test, not a `needs_human` test, so SC3 / D1 still
hold; name it in the plan so the verifier does not read it as scope drift.

### Pitfall 5: Ambiguous-width glyphs in a fixed table cell
**What goes wrong:** U+00B7 `·` and U+2192 `→` are East-Asian-Ambiguous; the glyph registry rule
exists to keep them out of fixed cells (glyphs.rs:1-30), and
`the_status_cell_source_draws_no_typography` enforces it for the Status cell's source (glyphs.rs:707-728,
scanning `src/agents/waves.rs` only). The `You` cell is locked to `·`/`→` by D-07/D-08.
**How to avoid:** keep the locked glyphs, but build the separator from one `OWNER_SEP` constant (swap
target `glyphs::BULLET_OPERATOR` U+2219, glyphs.rs:98), keep the cell's formatter out of
`src/agents/waves.rs`, and leave the alignment check to human UAT item 2. Both glyphs are in the
TYPOGRAPHY allow-list (`0x00B7 … 0x2192`, glyphs.rs:321-323), so the source scan passes.

### Pitfall 6: Two width-tier sites drift
`dashboard_columns` and the per-row cell vector are separate (Pattern 2). Guard: extend
`test_dashboard_columns_header_and_width_counts_match` (normal.rs:2586-2604, already iterates
`[200, 120, 80, …]`) and assert the row cell count equals the header count at 99/100.

### Pitfall 7: Existing tests that render at ≥ 100 columns
These now get the `You` column (and, at height ≥ 12, the summary row): normal.rs tests at
widths 100/120/160/200 (e.g. lines 3148, 3211, 3323, 3371, 3406, 3511-3539 `status_column_cells_matches_the_rendered_status_column`,
3576-3654, 3709-3820) and the 5-cell fixture rows of `render_dashboard_rows` swept 44..=200
(normal.rs:2606-2622; a 5-cell row in a 6-column table still puts Status at index 2, so it passes).
Baseline: `cargo test --lib -- ui::screens::normal` = **76 passed, 0.84 s** (run this session); 3349
lib tests in total. Phase column at 120 shrinks from today's width to 30 cells; texts asserted there
(`v0.14 P26`, `5/6 phases`, `(missing)`) are short enough, but the full suite must be re-run after the
tier lands. SC3's rule: only the added column may change; a test that changes for any other reason is a
regression.

### Pitfall 8: Summary-row height gate collides with 12-row tests
Many dashboard tests render at height 12 and width 100/120 (`render_selected` uses 12, normal.rs:3476-3479).
The UI-SPEC gate (`height >= 12`) shows the row there, shrinking the table by one row. With ≤ 7
projects nothing visible changes; a test with more rows would lose its last row. If such a test
breaks, raise the gate (e.g. `>= 14`) rather than edit the test.

### Pitfall 9: `↑` into the block when the block was not drawn
Decide block presence from a `Cell` the last render wrote, not from the digest alone: the block is
also hidden below 12 tab rows and when a focused Waves pane takes the tab under 100 columns. A
digest-based check would move focus into an invisible block.

### Pitfall 10: Cursor and expansion after a refresh
A watcher refresh can shrink the group list under an open cursor. Keys must be identities
(`(lane, kind, scope label, workstream, phase, file)`), the cursor clamps to the new stop count, and
vanished groups are dropped from `expanded` (UI-SPEC backstop test). `OwnerGroup` does not derive
`Hash` (mod.rs:468), so build an owned key struct.

### Pitfall 11: `e` falling through to enqueue
Content-level `e` enqueues (help.rs:240 "Enqueue next action (detail view)"). In `Owner` focus,
swallow `e`, `v`, `o`, `A`, `y` (`Some(ScreenAction::None)`); let `?`, `Tab`, digits, `D`, `[`, `]`
fall through, resetting focus for digits/`D`/`[`/`]` as `handle_waves_pane_key` does (detail.rs:2090-2095).

### Pitfall 12: D-04 test trips the 500 ms dedup
`FileChanged` → `schedule_reparse` is skipped when `last_refresh[alias]` is under 500 ms old
[VERIFIED: src/app.rs:2810-2817]. The D-04 test sends two `FileChanged` events (before/after flipping a
UAT row to `pass`); remove the `last_refresh` entry between them. It needs `#[tokio::test]`, an
`event_tx` with a receiver, a tempdir project registered in `config.projects` with the same path, and
`App::update` of the received `ProjectStateLoaded`. No existing test constructs `ProjectStateLoaded`
(grep: only action.rs and the two app.rs sites), so this is the first.

## Code Examples

### Existing ladder pattern to copy for the `You` cell
```rust
// Source: src/ui/screens/normal.rs:519-530
fn agent_summary_line(view: &crate::agents::waves::AgentView, cells: u16) -> Option<Line<'static>> {
    let form = view
        .summary_forms()
        .into_iter()
        .find(|form| Line::from(form.as_str()).width() <= usize::from(cells))?;
    // … color pick …
    Some(Line::from(Span::styled(form, Style::default().fg(color))))
}
```

### Existing gate counts the header reuses
```rust
// Source: src/inbox.rs:504-509 (lane order and `unblock` wording)
            let lanes = [
                ("test", gate.test, &gate.lane_phases.test),
                ("decide", gate.decide, &gate.lane_phases.decide),
                ("unblock", gate.blocked, &gate.lane_phases.blocked),
                ("fix", gate.fix, &gate.lane_phases.fix),
            ];
```

### Muted style and truncation helpers
```rust
// Source: src/ui/screens/driver.rs:120-122
pub(super) fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}
// Source: src/ui/screens/detail.rs:10797 — pub(super) fn fit_cells(text: &str, cols: usize) -> String
// Source: src/ui/screens/detail.rs:81-83 — fn shown(value: &str) -> String (private; bump to pub(super))
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 3-rank attention sort | 4 ranks (needs human, owner work, driven, rest) | This phase | One existing assertion pair updates (Pitfall 4) |
| Owner work visible only via `inbox`/`status --json` | Dashboard cell + summary + detail block | This phase | Same numbers as `status --json` `owner` |

**Deprecated/outdated:** `.planning/codebase/` maps are stale (84 structural changes); every line
number above was read live this session instead.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A digest-only `OwnerDigestLoaded` action plus a first-fill-only handler is the right complement to D-01 for startup/registration | Pattern 1 | Low: alternative is calling `schedule_reparse` for every project at startup (doubles startup parse cost and moves `reparse_dispatches`) |
| A2 | Module split `screens/owner.rs` (pure) + `screens/owner_block.rs` | Recommended layout | None functionally (CONTEXT discretion) |
| A3 | Keep the CLI's `not built ph …` wording in the shared clause builder | Correction 4 | One-colon copy difference vs UI-SPEC; UAT-visible only |
| A4 | `P` counts only Test groups with ≥ 1 item | Correction 6, Pattern 3 | Summary "in P phases" off by bookkeeping phases |
| A5 | Tab bar `↓` keeps descending to the phase list (Content), not into the Owner block | Pattern 6 | Asymmetric with `↑`; changing it would alter existing tab-bar tests |
| A6 | Raise the summary-row height gate only if a 12-row test breaks | Pitfall 8 | Minor layout difference at 12 rows |
| A7 | UI-SPEC resolved column widths (Status 13 at 100/120, 39 at 200) | Pattern 2 | Cited from UI-SPEC's probe, not re-run here; planner's unit test re-asserts them |

## Open Questions

1. **Startup dispatch vs D-01's "no second dispatch site".**
   - What we know: D-01's intent is that `reparse_dispatches` keeps meaning "full re-parses"; a
     digest-only task does not re-parse and does not touch the counter.
   - What's unclear: whether the orchestrator reads "no second dispatch site" literally.
   - Recommendation: implement the first-fill dispatcher (A1) and mark it **[inferred]** in the plan;
     without it SC1 cannot pass on a freshly started TUI.
2. **Help/README wording for unreadable files.** Use "unreadable checks count as fix work" instead of
   CONTEXT's "count as test work" (Correction 1).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | build, tests | ✓ | 1.99.1 (MSRV floor 1.89) | — |
| node + gsd-tools | planning tooling only | ✓ | — | — |

No external services. Step 2.6 otherwise not applicable.

## Validation Architecture

Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json`. Test expectations are in
36-UI-SPEC.md "Snapshot-test Expectations" and in Pitfalls 4, 7, 8, 12 above. Quick commands:
`cargo test --lib -- ui::screens::normal` (0.84 s), `cargo test --lib -- ui::screens::detail`,
`cargo test --lib -- ui::screens::help`, `cargo test --lib -- render_escape_guard`,
`cargo test --test async_blocking_guard`; full: `cargo test --locked` and
`cargo clippy --all-targets -- -D warnings`.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | read-only views; no write, no launch (D-20) |
| V5 Input Validation / Output Encoding | yes | Every project-authored digest string through `shown()` / `render_for_terminal` at render (Correction 2); `fit_cells` for width; `render_escape_guard` fixtures extended |
| V6 Cryptography | no | — |
| V12 Files and Resources | indirect | Reader already enforces the 1 MiB cap and symlink containment (OWN-11); the TUI adds no file access |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal escape / bidi injection via a UAT name, phase name, command or file path in a `.planning/` file | Tampering / Spoofing | `render_for_terminal` on every cell; hostile digest fixtures in `render_escape_guard` (dashboard and Phases tab states) |
| A row command shown verbatim that the owner copies and runs | Spoofing | Commands come from the reader's token-checked builder (`command_for`, `is_command_token`, rules.rs:74, 113); still escape at render |
| Huge digest (many groups/items) slowing every frame | Denial of service | Block height capped (~40% of tab), items capped at 5 per group with `… N more`; per-frame work is in-memory only |
| Blocking I/O on the render thread | Denial of service | `spawn_blocking` only; `tests/async_blocking_guard.rs` lists `read_owner_digest(` |

## Sources

### Primary (HIGH confidence, read this session)
- `src/state_reader/owner_items/mod.rs` (digest types, `read_owner_digest`, `evict_project`)
- `src/state_reader/owner_items/rules.rs:370-584, 1010-1133` (lane placement, counts, R7) and `gate.rs:1-80`
- `src/inbox.rs:1-720` (wording helpers, filters, closes-after builder)
- `src/app.rs` (filter grammar, `schedule_reparse`, `FileChanged`, `ProjectStateLoaded`, insert sites, prune + destructure test)
- `src/main.rs:495-600` (startup order), `src/action.rs:35-60`, `src/config.rs:538-553`
- `src/ui/screens/mod.rs` (AppContext, SortMode, `attention_rank`, sort/filter, ProjectViewCache, rank test)
- `src/ui/screens/normal.rs:405-1473` and its test helpers (tiers, cells, footer, keys)
- `src/ui/screens/detail.rs` (`shown`, `DetailFocus`, Waves-pane handler, `↑` arm, render dispatch/footer, `render_pipeline_tab`, `fit_cells`, footers)
- `src/ui/screens/help.rs:185-484, 655-700`, `src/ui/glyphs.rs`, `src/ui/screens/render_escape_guard.rs` (structure), `tests/async_blocking_guard.rs`
- `cargo test --lib -- ui::screens::normal`: 76 passed, 0.84 s

### Secondary
- `36-CONTEXT.md`, `36-UI-SPEC.md`, `REQUIREMENTS.md` OWN-11..23, `OWNER-INBOX-UX.md` §3.1-3.2, `34-VERIFICATION.md` (D-04 follow-up)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new dependency.
- Architecture: HIGH, every integration point read live; A1 is the only structural inference.
- Pitfalls: HIGH, each tied to a cited line; Pitfall 8's impact depends on test fleet sizes (MEDIUM).

**Research date:** 2026-11-14
**Valid until:** until the next change to `app.rs`/`normal.rs`/`detail.rs` on this branch (line numbers drift; identifiers stay valid ~30 days)
