# Phase 38: Inbox digest and triage hygiene - Research

**Researched:** 2026-11-16
**Domain:** In-repo Rust (CLI text renderer, Layer B owner rules, ratatui inbox screen); no new dependencies
**Confidence:** HIGH (every claim below was read from the worktree this session, unless tagged otherwise)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (not re-decided)
- PROJECT.md "GSD files are read-only" (2026-11-16) and report D7: the app writes nothing into a
  project. Phase 38 is reader and renderer work only.
- Layer A / Layer B split (34-CONTEXT): Layer A must keep matching `audit-uat` (oracle test). Every
  Phase 38 change is Layer B or rendering; Layer A and the oracle fixtures stay untouched.
- D3 strict dedup: a UAT whose `source:` does not name the VERIFICATION does not supersede it.
  The D-08 merge below is display grouping, not supersede: no item is dropped.
- 36 D-17 vocabulary (`ph N`, `ws` prefix, `N checks`, `closes phase`, `stale`, lane tags) and 37
  D-04 (one DECIDE row per debug session, not aggregated).
- 37 D-09: `y` copies exactly what the CLI prints for that group, through `group_checklist`.
- `inbox --json` schema `gsd-meta-manager/inbox` v1, deterministic, `status`-style exit codes;
  additive keys only (SC6).
- `stale` marker at 30 days (`preferences.owner.stale_days`, D9) is unchanged. The 90-day fold is a
  separate threshold (D-09).

#### CLI views (OWN-24, OWN-25)
- **D-01 [user]:** Plain `inbox` prints a short digest: per-project counts per lane plus the top
  few items. `inbox <alias>` shows one project; `--lane <test|decide|fix|blocked>` one lane;
  `--all` the full list; `--json` keeps carrying everything.
- **D-02:** View selection, in order **[inferred]**:
  1. `--json`: unchanged document (plus the additive keys of D-12). `--all` and the view choice do
     not change it; `--lane`/`--include` keep filtering as today.
  2. `--all`: the legacy `render_text` exactly as on master today, with `--lane`/`--include`
     honored as today. No merge, no fold, no labels, so `--all` equals the pre-phase output byte
     for byte on every fixture (SC2). The current `tests/fixtures/owner_golden/inbox.txt` and
     `inbox-include-all.txt` become the `--all` goldens unchanged.
  3. Exactly one project argument (alias or path): the full checklist for that project (D-04).
  4. `--lane` without a single project: the lane view (D-05).
  5. Otherwise (no argument, or two or more): the digest (D-03) over the selected projects.
  `--include` given without `--lane`, a single project or `--all` implies `--all`, so existing
  `inbox --include archived` invocations still print the full list **[inferred: back-compat]**.
- **D-03: Digest layout** (target 15-25 lines, asserted by a line-budget test on the fixtures and
  a synthetic 20-project fleet) **[inferred: exact layout]**:
  1. The existing totals line (`Owner inbox: N projects · test T · decide D · fix F phases ·
     blocked B`), numbers unchanged so they keep equaling `summary` and `status --json` `owner`.
  2. Per-project block: one line per project with owner work: alias, milestone version, then the
     `closes after` clauses in `ClauseForm::PhasesCapped(4)` (lane counts with their phases), and,
     for a project whose work is not in `closes_after` (for example root-scoped debug sessions),
     its lane counts alone. At most 10 lines; ordered by closeable phases (most first), then
     alias; the rest summarized as `+N more projects (inbox --all)`.
  3. `NEXT` section: the top 5 next actions (D-07), one line each: alias, `ph N` (with `ws`
     prefix), name capped, `N checks`, `→ closes phase` when it does, labels count (D-11), age,
     command.
  4. The fold lines (Old debt, Stale (D-09), Acknowledged, Deferred, Triage) as today, one line
     each.
  5. A hint line: `inbox <alias> · inbox --lane test|decide|fix|blocked · inbox --all`.
- **D-04: `inbox <alias>`** prints the full checklist for that project with the hygiene applied:
  groups and their items as today, D-08 merged entries, D-11 labels on item lines, and stale groups
  (D-09) listed in a trailing `STALE (> 90 d)` section instead of in their lane. Nothing is hidden
  for a single project **[inferred]**. A 51-step scripted UAT (gnome-victron ph 04 today) lists its
  51 items here; that is the point of the full view.
- **D-05: `inbox --lane L`** (any number of projects) prints the lane's sections exactly as the
  full output orders them, but collapsed: one line per phase entry (D-08 merged), no item lines,
  stale entries folded into one line at the end (D-09). Several `--lane` values print several
  lanes in the usual section order.

#### Collapse (OWN-26)
- **D-06 [user: B]:** In multi-project views a phase is one entry (`ph 43 · 3 checks` plus the
  action), never its individual checks. Applies to the digest, the lane view and the TUI inbox
  `i`. The TUI already shows one row per group with items behind `Enter` (37 D-04/D-06); Phase 38
  adds the merge (D-08), the stale fold row (D-09) and labels (D-11) there. The detail Owner block
  is a single-project view and keeps its 36 layout (frozen for the pending 36 UAT).

#### Ranking (OWN-24)
- **D-07:** Next actions are chosen from current-milestone, non-acknowledged, non-folded entries in
  Test, Decide and Fix (Blocked is not actionable by the owner, as in 3.1). Sort key: entries that
  close a phase first; then fewest checks (the merged entry's sum) first; then **newer first**
  (older items rank lower, per OWN-24; deliberately the opposite of the TUI's oldest-first triage
  order, which stays as 37 D-05 locked it); then alias, phase key, file for a total order. Bookkeeping-only entries
  ("all checks passed, run /gsd-verify-work N to finalize") count as 0 checks and close a phase,
  so they rank first. One pure, unit-tested function shared by the text digest and `--json`
  (D-12) **[inferred: candidate set, tie order, bookkeeping rank]**. Ages are as R9 computes them.

#### Hygiene (OWN-27) — data cleanups (allowed to change what is shown)
- **D-08 [user: D]: Same-phase merge.** Groups with the same project, scope, workstream, phase and
  lane are shown as one entry. Count = the sum of the groups' checks; the entry names its files
  (`2 files`); age = the newest group's age; command and `closes_phase` are per phase and equal.
  Live case: sentriq `v0-15-buyers-mode` ph 43 lists `43-UAT.md` (2 checks, `source:` names only
  SUMMARYs) and `43-VERIFICATION.md` (`human_needed`, 1 check) as two rows today; after the merge
  one entry `ph v0-15-buyers-mode/43 · 3 checks · 2 files`. No item-level dedup: the two
  "real-truck crank" items look alike, but matching them is a heuristic, so both stay (the user
  allows only labels for heuristics) **[inferred]**. Merge happens in Layer B or the view model;
  the JSON `groups` array keeps one group per file (key paths and the oracle unchanged) and gains
  the additive link of D-12. Expanding a merged entry in the TUI lists the items per file, with the
  file name as a sub-heading **[inferred]**. `v` and `y` act on the merged entry (same phase, same
  command; `y` copies all its items). `A` stays file-level: on a merged entry it asks which file
  to acknowledge, or is refused with a message telling the user to expand and pick a file. The
  planner chooses whichever is simpler **[inferred]**.
- **D-09 [user: D]: Stale fold.** Current-milestone entries whose age exceeds the fold threshold
  leave their lane in every multi-project view and are counted in one line `Stale: N checks in P
  phases older than 90 d`, which sits next to the other folds. In the TUI it is a fold row that
  `Enter` expands, like Old debt. Threshold: `preferences.owner.fold_days`, default 90, clamped
  like `stale_days` **[inferred: key name and clamping]**. Entries with no age never fold. Totals,
  `closes_after`, `status --json` `owner` and the dashboard cell keep counting folded entries.
  The work still blocks the milestone; the fold only takes it out of the triage list.
- **D-10 [user: D]: `1 check`/`2 checks` alignment.** The count column renders the number
  right-aligned and the unit left-aligned (`1 check ` / `2 checks`), so the digits line up. Applies
  to the TUI inbox and the CLI collapsed lines. The detail Owner block has the same right-aligned
  `N check(s)` cell, so it also uses the shared helper. The change is alignment only, not layout,
  and its goldens move by at most one cell **[inferred]**. Buffer snapshot asserts the alignment
  (SC4).

#### Heuristic labels (OWN-28) — label only
- **D-11 [user: D]:** Labels never move, hide, fold, re-rank or re-count a row. They change no
  lane, no total, no `closes_phase`, no ranking key and no fold.
  - `decision?`: an item's name or `expected` matches, case-insensitively and on word boundaries,
    `owner decision`, `decide`/`decision`, `ratify`/`ratification`, `approve`/`approval`,
    `accept`, `sign-off`/`sign off`, `rule on`, `review`; and `OK` matched case-sensitively as a
    whole word (lowercase "ok" is too common) **[inferred: exact list and forms]**. Not applied to
    Decide-lane items: those are already decisions.
  - `looks resolved`: name, `expected` or `reason` matches `resolved`, `fixed by`, `fixed in`,
    `closed by`, `addressed by`, `addressed in`, `superseded by` **[inferred: exact list]**.
  - One pure matcher module in Layer B with a table-driven test, including false-positive guards
    such as "unresolved", "review" inside another word and "okay".
  - Display: on an item line, a dim suffix (`decision?`, `looks resolved`). On a collapsed entry,
    a dim count (`2 decision?`) only when non-zero. CLI and TUI share the wording.

#### JSON (SC6)
- **D-12:** Additive keys only; every key path pinned in `tests/inbox_cli.rs` stays. Names are the
  planner's choice; the data is fixed **[inferred]**:
  - per item: `labels` (array of strings, `[]` when none);
  - per group: `folded_stale` (bool) and a merge link, so a consumer can rebuild the D-08 entry
    (for example `entry_files`: the basenames of every file in the same entry, sorted);
  - `summary`: `next_actions` (the D-07 list: alias, workstream, phase, lane, checks,
    closes_phase, command, age_days) and `stale_folded` counts.
  README `inbox` schema, `status --skill` paragraph (`inbox` now prints a digest; agents use
  `--json` or `--all`) and the help overlay (TUI stale fold row) are updated.

#### Tests and UAT
- **D-13:** Fixtures under `tests/fixtures/owner/` with synthetic text:
  - `same-phase-merge` modeled on sentriq ph 43 (UAT whose `source:` names SUMMARYs + a
    `human_needed` VERIFICATION, in a workstream): JSON two groups with the link, one text entry
    with 3 checks;
  - `stale-fold` (an `updated:` more than 90 days back, next to a fresh group);
  - `scripted-checklist` (one UAT with 51 numbered tests): one collapsed line in the digest, lane
    view and TUI, 51 items in `inbox <alias>`;
  - `heuristic-labels` (keyword hits, near misses, a Decide-lane item that is not labeled).
  Goldens: `inbox-digest.txt` (new, line budget asserted), `inbox-lane-test.txt`,
  `inbox-alias.txt`. The existing `inbox.txt` and `inbox-include-all.txt` stay byte-identical as
  `--all` goldens. TUI buffer snapshots: merged entry, stale fold row, aligned count column,
  labels. The oracle conformance test is unchanged and green. `render_escape_guard` covers the new
  strings. Ages in goldens use the existing clock seam (the planner confirms one exists, else
  adds one) **[inferred]**.
- **D-14:** Human UAT (ROADMAP): one readability check. Does `inbox` on the real fleet read as a
  useful one-screen answer? Nothing else goes to a human.

#### Musk pass
- Phase pass, no cuts. OWN-24..28 come from the owner's own feedback after the first real use, and
  decisions A, B and D settle them. Every item has an owner. Lean lane not used: the work spans
  three subsystems (CLI renderer, Layer B rules, TUI inbox) **[inferred]**.

### Claude's Discretion
- Exact digest wording, column spacing and the 10-line project cap (within the 15-25 line budget).
- Names of the additive JSON keys (D-12) and of the fold config key (D-09).
- Whether the merge (D-08) lives in Layer B (`owner_items/rules.rs`) or in a shared view model
  used by `inbox.rs` and `ui/screens/inbox.rs`, as long as both views share one implementation.
- `A` behavior on a merged entry (D-08).
- Number and grouping of plans (coarse granularity).

### Deferred Ideas (OUT OF SCOPE)
- Context tags (desk / phone / truck) on checks **[user: deferred]**.
- Moving or hiding rows by heuristic (OWN-F02 beyond labels).
- Item-level dedup of near-identical checks across a merged UAT/VERIFICATION pair (needs text
  matching, a heuristic).
- A `NEXT` / digest view inside the TUI (the TUI inbox already is the triage view; not required by
  OWN-26).
- 37 UAT item 6 (section counts or a lane strip so DECIDE/FIX show on the first screen): the owner
  decides in the 37 UAT.
- Check actions through GSD: Phase 39.

#### Reviewed Todos (not folded)
`todo.match-phase 38` returned the same keyword-only matches Phases 34-37 reviewed (TUI badge for
release maps, help popup at 80x24, launcher-marker in-flight flag, unregister guard, multi-line
queue input, ...). None touches the inbox digest, the views or the hygiene rules, so none is
folded despite the ≥ 0.4 auto-fold rule; folding them would be scope creep **[inferred]**.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description (REQUIREMENTS.md:115-132) | Research Support |
|----|-------------|------------------|
| OWN-24 | Plain `inbox` defaults to a short digest (~15-25 lines): totals, per-project `closes after`, top ~5 next actions ranked by payoff | View dispatch in `inbox_cli::run` (Pattern 1); `closes_after_clauses(.., ClauseForm::PhasesCapped(4))` reuse; ranking function (Pattern 4); line-budget arithmetic (Pitfall 9) |
| OWN-25 | `inbox <alias>` full checklist; `--lane` one lane; `--all` pre-phase output; `--json` additive only | `render_text` stays untouched as the `--all` path (Pattern 1); new `--all` clap flag; key-path exactness test mechanics (Pitfall 2) |
| OWN-26 | Multi-project views show each phase collapsed (count + action); a 51-step scripted UAT reads as one entry | Shared triage view model (Pattern 2); `group_line` already is the collapsed line; TUI already one row per group |
| OWN-27 | ~90-day stale fold; same-phase duplicate merge; aligned `1 check`/`2 checks` column in the TUI | Merge key and kind restriction (Pitfalls 4, 5, 6); fold computed at view time from `age_days` (Pattern 3); `checks_cell` helper (Pattern 6) |
| OWN-28 | `decision?` / `looks resolved` labels, label only, CLI + TUI + additive `--json` | `regex` (already a dependency) matcher module in Layer B (Pattern 5); false-positive table |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

No `AGENTS.md` exists in the worktree; `./CLAUDE.md` is the project instruction file. No `.claude/skills/` or `.agents/skills/` directory exists in the worktree. Actionable directives:

- **State reading:** must not require running Claude/GSD to check status; read from files or cached state. Render reads caches only (OWN-12; enforced by `tests/async_blocking_guard.rs`).
- **Non-intrusive:** must not interfere with running GSD instances. Phase 38 writes nothing (PROJECT.md "GSD files are read-only").
- **Portability:** works for any GSD user; nothing hardcoded to one user's setup. Fixtures must hold synthetic text only (public crate).
- Logs go to a file, never stdout (`inbox` is dispatched before `init_tracing`, so it logs nothing).
- `std::sync::Mutex` must not be held across `.await` (not relevant: all new code is sync and pure).
- MSRV `rust-version = "1.89"` [VERIFIED: Cargo.toml:18 `rust-version = "1.89"`]; do not use std APIs newer than 1.89.
- GSD workflow enforcement: edits only through `/gsd:execute-phase`.
- Release process (tagging, `pre-tag-check.sh --container`) is not part of this phase.
- User global rules that bind the executor: US spelling; conventional commits with path-scoped `git commit -- <paths>`; run `rtk bypass <cmd>` whenever a check depends on raw output (RTK strips `test result:` lines).

## Summary

Phase 38 is pure in-repo work over code Phases 34-37 already built. No new crate is needed: `regex = "1"` is already a dependency for the label matcher, and every renderer piece the digest needs exists (`closes_after_clauses` with `ClauseForm::PhasesCapped`, `group_line`, `group_checklist`, `plural`, the fold lines of `render_text`). The safest structure is **additive**: keep `inbox::render_text` byte-for-byte untouched as the `--all` path, and build the digest, lane and single-project views as new functions over one new pure **triage view model** that computes merged entries, the stale fold flag, labels and the ranking from an `OwnerDigest` plus `fold_days`. The TUI inbox consumes the same model through a new inbox-only source function, so the detail Owner block (frozen for the 36 UAT) keeps calling the unchanged `owner_block::current_sources`.

The three most expensive traps found in the code are: (1) `tests/inbox_cli.rs::the_inbox_key_paths_are_exactly_schema_v1` asserts **exact** key-path equality over two fixtures whose `updated:` dates are 2026-01, so at today's wall clock every group is older than 90 days, the stale fold empties `next_actions`, and the new `summary.next_actions[].*` paths never appear. The test registry needs a fresh project. (2) Every directory under `tests/fixtures/owner/` that holds `.planning/` is fed to the real `audit-uat` oracle and must carry an `oracle.expect` file, so the four new D-13 fixtures must be valid Layer A input. (3) The existing goldens' fixtures are 307 days old at the lib-level `now()`, so a digest golden built over them would show every group in the Stale fold; the digest golden needs fresh fixtures or its own fixed `now`.

Adding fields to `OwnerGroup`/`OwnerItem` is possible but costly: there are 13+ struct-literal construction sites in `src/` (no `Default` derive). Computing merge, fold and labels at view time from existing fields avoids touching any of them and avoids changing `read_owner_digest`'s signature (called from `app.rs`, `ui/screens/mod.rs`, `status_cli.rs`, `inbox_cli.rs` and tests).

**Primary recommendation:** Add one pure module `src/state_reader/owner_items/triage.rs` (entries, fold, ranking) plus `labels.rs` (regex matcher) in Layer B. Add new renderers beside `render_text` in `src/inbox.rs` (`render_digest`, `render_lanes`, `render_project`, `entry_line`, `entry_checklist`). Route views in `inbox_cli::run` by the D-02 order. Feed the TUI inbox from a new `inbox_sources(digest, fold_days)` and leave `render_text`, `current_sources`, Layer A and the oracle fixtures untouched.

## Architectural Responsibility Map

The "tiers" of this single-binary TUI/CLI app:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Same-phase merge, stale fold flag, ranking (D-07/08/09) | Layer B pure model (`state_reader/owner_items/triage.rs`) | — | Pure functions over `OwnerDigest` + `fold_days`; shared by CLI and TUI so they cannot disagree (37 D-09 pattern) |
| `decision?` / `looks resolved` labels (D-11) | Layer B pure matcher (`owner_items/labels.rs`) | — | CONTEXT says "one pure matcher module in Layer B"; no I/O |
| Fold threshold config (`preferences.owner.fold_days`) | Config (`src/config.rs` `Preferences`) | Callers (`inbox_cli`, TUI) | Same pattern as `owner_stale_days` |
| View selection (`--all`, alias, `--lane`, digest) | CLI binary (`src/inbox_cli.rs`, `src/cli.rs`) | — | clap flag + dispatch; the lib renderers stay pure |
| Text rendering (digest, lane, single project, legacy) | Lib renderer (`src/inbox.rs`) | — | Pure, golden-testable with a fixed `now` at read time |
| JSON additive keys | Lib (`inbox::build_document`) | README schema + `tests/inbox_cli.rs` pins | Schema v1 additive |
| TUI merged rows, Stale fold row, labels, aligned checks | TUI (`src/ui/screens/inbox.rs`, `owner_block.rs` helpers) | — | Render reads caches only; consumes the Layer B model |
| `v`/`y`/`A`/`o`/`g` on a merged entry | TUI actions (`inbox.rs` key handlers, `owner_actions.rs`) | — | `A` and `o` are file-level, `v`/`y` entry-level |

## Standard Stack

### Core (all already in the tree; nothing to install)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| regex | `regex = "1"` [VERIFIED: Cargo.toml dependencies] | `decision?` / `looks resolved` word-boundary matching | Already a dependency (used in `src/agents/worktrees.rs`); `\b` is Unicode-aware by default |
| ratatui | `0.30` with `crossterm` [VERIFIED: Cargo.toml] | TUI inbox rows, `TestBackend` buffer snapshots | Existing inbox screen and its tests use `ratatui::backend::TestBackend` |
| clap | `4` derive [VERIFIED: Cargo.toml] | new `--all` flag on `Commands::Inbox` | Existing CLI |
| serde_json | `1` [VERIFIED: Cargo.toml] | additive JSON keys via `json!` | Existing `build_document` |
| chrono | `0.4` [VERIFIED: Cargo.toml] | ages are already computed at read time (R9) | No new clock reads needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `regex` for labels | hand-rolled word-boundary scanner (like `starts_with_answered_word` in rules.rs) | Hand-rolled is fine for a fixed list but more code; `regex` already compiled in, one `LazyLock<Regex>` per label |
| View-time fold flag | new `OwnerGroup.folded` field filled in Layer B | Field needs `fold_days` in `read_owner_digest` (signature change at 4+ call sites) and edits at 13+ struct literals |
| Synthetic merged `OwnerGroup` per entry | Render merged entries field by field | The synthetic group lets `group_line`, `detail`, `row_cells` and `run_plan` work unchanged |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed by this phase. Every library used is already locked in `Cargo.lock`.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
argv ──> clap Commands::Inbox {projects, json, lane, include, all(new)}
           │
           v
     inbox_cli::run ── slots(config,cwd,targets) ──> per project:
           │              parse_project_state + read_owner_digest(now, stale_days)   (unchanged)
           │                         │
           │                         v
           │                  Vec<ProjectEntry> (OwnerDigest per project, ages already computed)
           │                         │
           │        fold_days = config.preferences.owner_fold_days()   (new, clamped)
           │                         │
           v                         v
   ┌── view choice (D-02 order) ─────────────────────────────────────────────┐
   │ --json ──> build_document(+ labels, folded_stale, entry_files,          │
   │            summary.next_actions, summary.stale_folded)  ──> 1 JSON line │
   │ --all / implied by --include ──> render_text (UNCHANGED, byte-exact)    │
   │ 1 project ──> render_project (entries + items + labels + STALE section) │
   │ --lane L ──> render_lanes (collapsed entry lines + Stale fold line)     │
   │ else ──> render_digest (totals, closes-after block, NEXT 5, folds, hint)│
   └─────────────────────────────────────────────────────────────────────────┘
                     ^ all four non-legacy paths call ^
          owner_items::triage::{entries, next_actions, stale_totals}
          owner_items::labels::{labels_of}
                     v also called by v
   TUI inbox (`i`): inbox_sources(digest, fold_days) -> merged RowSrc rows,
                    Fold::Stale rows; row_line uses checks_cell(); labels count
   Detail Owner block: current_sources (UNCHANGED) + checks_cell() alignment only
```

### Recommended Project Structure
```
src/
├── state_reader/owner_items/
│   ├── triage.rs      # NEW: TriageEntry, entries(), is_folded(), next_actions(), stale totals
│   ├── labels.rs      # NEW: Label enum, labels_of(item, lane) with LazyLock<Regex>
│   └── mod.rs         # `mod triage; mod labels;` + pub re-exports
├── inbox.rs           # render_text UNCHANGED; + render_digest, render_lanes, render_project,
│                      #   entry_line, entry_checklist (shared with TUI `y`), build_document keys
├── inbox_cli.rs       # view dispatch (D-02), passes fold_days
├── cli.rs             # `--all` flag on Commands::Inbox
├── config.rs          # Preferences::owner_fold_days()
└── ui/screens/
    ├── inbox.rs       # inbox_sources-based rows, Fold::Stale, labels count, checks_cell
    ├── owner_block.rs # Fold::Stale variant, RowSrc.parts (or similar), checks_cell helper
    ├── owner_actions.rs # fold_copy arm for Fold::Stale
    └── help.rs        # Stale fold row text
tests/
├── fixtures/owner/{same-phase-merge,stale-fold,scripted-checklist,heuristic-labels}/  (+ oracle.expect)
├── fixtures/owner_golden/{inbox-digest.txt,inbox-lane-test.txt,inbox-alias.txt}       (new)
├── owner_digest.rs    # lib-level goldens at a fixed now (pattern exists)
└── inbox_cli.rs       # INBOX_PATHS appended; plain-text tests retargeted to --all
```

### Pattern 1: Keep the legacy renderer as the `--all` path; dispatch in `inbox_cli`
**What:** Do not refactor `render_text`. Add the `--all` clap flag and choose the renderer in `inbox_cli::run`.
**Today's dispatch** [VERIFIED: src/inbox_cli.rs:141-147]:
```rust
    if json {
        out_line(&build_document(&entries, &filters).to_string());
    } else {
        for line in render_text(&entries, &filters).lines() {
            out_line(line);
        }
    }
```
**Today's flags** [VERIFIED: src/cli.rs:266-279]: `projects: Vec<String>`, `#[arg(long)] json: bool`, `#[arg(long, value_delimiter = ',')] lane: Vec<InboxLane>`, `#[arg(long, value_delimiter = ',')] include: Vec<InboxInclude>`. `main.rs` destructures them [VERIFIED: src/main.rs:115-126 `Some(Commands::Inbox { projects, json, lane, include, }) => std::process::exit(inbox_cli::run( &projects, json, &lane, &include, cli.config.clone(), ))`], so adding `all` touches `cli.rs`, `main.rs`, `inbox_cli::run` and the two clap unit tests at `src/cli.rs:1188-1226`.
**Sketch (new names are proposals):**
```rust
enum View { Json, Legacy, Project, Lanes, Digest }
fn view_of(json: bool, all: bool, targets: &[String], lanes: &[InboxLane], includes: &[InboxInclude]) -> View {
    if json { return View::Json; }
    if all { return View::Legacy; }
    if targets.len() == 1 { return View::Project; }
    if !lanes.is_empty() { return View::Lanes; }
    if !includes.is_empty() { return View::Legacy; } // D-02 back-compat
    View::Digest
}
```
Make `view_of` a pure function with a table test (every D-02 row). Note the D-02 wording: `--include` implies `--all` only "without `--lane`, a single project or `--all`".

### Pattern 2: One triage model, computed at view time
**What:** `entries(digest, fold_days) -> Vec<TriageEntry>` groups current-milestone owner groups into entries and flags fold. Each entry carries a **synthetic `OwnerGroup`** (summed counts, concatenated items, newest age, shared command) plus its parts, so `group_line`, `detail`, `row_cells` and `run_plan` work on it unchanged.
**Merge key:** `(scope == Current, workstream, phase_num::phase_key(phase), lane)` and **kind ∈ {Uat, Verification}** only (see Pitfall 4). Acknowledged groups never merge (they are not owner rows).
**Existing precedent:** the BLOCKED aggregate already merges groups per (workstream, phase) into one row [VERIFIED: src/ui/screens/owner_block.rs:441-447]:
```rust
pub(crate) struct RowSrc {
    pub(crate) group: OwnerGroup,
    pub(crate) merged: Option<OwnerGroup>,
    pub(crate) files: Vec<String>,
    pub(crate) key: GroupKey,
}
```
Do **not** put the same-phase merge into `merged`: `row_cells` reads `merged` as a BLOCKED tally (Pitfall 7).
```rust
// Proposed (src/state_reader/owner_items/triage.rs)
pub struct TriageEntry<'a> {
    pub group: OwnerGroup,          // synthetic: counts summed, items concatenated in part order
    pub parts: Vec<&'a OwnerGroup>, // one per file, digest order
    pub files: Vec<String>,         // rel paths of the parts
    pub folded: bool,               // age (newest part) > fold_days; None age never folds
}
```
Age of the entry is the minimum `age_days` over its parts (the "newest group's age"). The `stale` flag is recomputed from that age or taken as all-parts-stale. Do not OR it, or a fresh entry shows `stale`.

### Pattern 3: Fold threshold as a clamped preference
[VERIFIED: src/config.rs:546-553]:
```rust
    pub fn owner_stale_days(&self) -> u32 {
        const DEFAULT: u32 = 30;
        self.extra
            .get("owner")
            .and_then(|owner| owner.get("stale_days"))
            .and_then(Value::as_i64)
            .map_or(DEFAULT, |days| days.clamp(1, 3650) as u32)
    }
```
Copy this shape for `owner_fold_days()` with key `fold_days` and default 90, and mirror the test `owner_stale_days_defaults_to_thirty_and_clamps` (src/config.rs:1341). The fold compares the same `age_days` R9 computes [VERIFIED: src/state_reader/owner_items/rules.rs:241-247 `fn days_since(moment: DateTime<Utc>, now: DateTime<Utc>) -> i64 { (now - moment).num_days().max(0) }` and `fn age_of(file: &AuditFile, now: DateTime<Utc>) -> Option<i64> { file.updated.or(file.modified).map(|m| days_since(m, now)) }`], using `>` like the stale marker [VERIFIED: rules.rs:439 `stale: age_days.is_some_and(|days| days > i64::from(ctx.stale_days)),`].

### Pattern 4: The ranking is one pure function (D-07)
```rust
// Proposed
pub fn next_actions<'a>(entries: &'a [(String /*alias*/, TriageEntry<'a>)], limit: usize) -> Vec<&'a ...> {
    // candidates: !folded, lane in {Test, Decide, Fix}, scope Current, command.is_some() (see Open Q1)
    // key: (!closes, checks, age (None last), alias, phase_key, file)
    //  closes = group.closes_phase || group.bookkeeping_only
    //  checks = if bookkeeping_only { 0 } else { group.counts.checks }
}
```
Bookkeeping groups are created with `closes_phase: false` [VERIFIED: rules.rs:435 `closes_phase: false,` in `new_group`, then rules.rs:589-590 `let mut group = new_group(&group_ctx, Lane::Test, Vec::new(), 0); group.bookkeeping_only = true;`], so the ranking must treat `bookkeeping_only` as closing. D-07 says it closes a phase. The JSON `next_actions[].closes_phase` should use the same rule, which makes it differ from that group's own `groups[].closes_phase`. Document the difference in the README row.

### Pattern 5: Labels: two `LazyLock<Regex>`, one function
```rust
// Proposed (src/state_reader/owner_items/labels.rs)
static DECISION_CI: LazyLock<Regex> = LazyLock::new(|| Regex::new(
  r"(?i)\b(?:owner\s+decision|decide|decision|ratify|ratification|approve|approval|accept|sign[-\s]?off|rule\s+on|review)\b").unwrap());
static DECISION_OK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bOK\b").unwrap()); // case-sensitive
static RESOLVED: LazyLock<Regex> = LazyLock::new(|| Regex::new(
  r"(?i)\b(?:resolved|fixed\s+by|fixed\s+in|closed\s+by|addressed\s+by|addressed\s+in|superseded\s+by)\b").unwrap());
pub fn labels_of(item: &OwnerItem, lane: Lane) -> Vec<&'static str> { /* decision? (not Decide lane) on name+expected; looks resolved on name+expected+reason */ }
```
`LazyLock` is already used in this module tree [VERIFIED: src/state_reader/owner_items/mod.rs:40 `use std::sync::{Arc, LazyLock, Mutex, PoisonError};`]. Table test rows: hits for each keyword, and near misses `unresolved`, `preview`, `reviewer`?, `okay`, `ok`, `Bookkeeping`, `acceptance` (whole-word rule means `acceptance`, `approved`, `reviewed` do **not** match; see Open Q2), a Decide-lane item containing `decide` → no label.

### Pattern 6: Aligned checks cell, one helper for inbox and Owner block
Today both screens right-align the whole string [VERIFIED: src/ui/screens/inbox.rs:519 `const CHECKS_W: usize = 9;` and :716 `spans.push(Span::styled(pad(&c.cells.checks, CHECKS_W, true), plain));`; src/ui/screens/owner_block.rs:37 `const CHECKS_W: usize = 9;` and :781 `spans.push(Span::styled(pad(&row.checks, CHECKS_W, true), plain));`], so ` 1 check` and `2 checks` put the digit in different columns. The cell text comes from `inbox::plural(group.counts.checks, "check", "checks")` [VERIFIED: owner_block.rs:388-389]. A helper such as `checks_cell(n) -> String` = `format!("{n:>2} {unit:<6}")` (9 cells for n ≤ 99) gives ` 1 check ` / `12 checks`. The helper must grow for n ≥ 100 rather than let `pad` cut the unit (see Pitfall 10). The inbox title head `pad("checks", CHECKS_W, true)` (inbox.rs:777) and the test `assert_eq!(checks_at, grid.checks_x() + 3, "right-aligned over Checks")` (inbox.rs:3046) move with it.

### Anti-Patterns to Avoid
- **Editing `render_text` to add a "mode" flag:** risks the byte-for-byte `--all` guarantee. Add new functions and leave it alone.
- **Changing `owner_block::current_sources`:** it feeds the frozen detail Owner block (D-06). The inbox needs its own source function.
- **Merging Debug/Handoff/Checkpoint/ContextQuestions groups:** debug sessions all have `phase: String::new()` and `workstream: None` [VERIFIED: rules.rs:680-681 `workstream: None, phase: String::new(),`], so a naive merge key collapses every debug session into one row, which breaks 37 D-04.
- **Filling `folded_stale` per file group independently:** must be the entry's value, or a JSON consumer sees one half of a merged entry folded and the other not.
- **Labels influencing anything:** no label may enter `next_actions` sort keys, lanes, counts or the fold (D-11). Assert this in a test (rank with and without labels equal).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-project `closes after` clause text | new formatter | `inbox::closes_after_clauses(gate, ClauseForm::PhasesCapped(4))` [VERIFIED: src/inbox.rs:528-535 enum `Full`, `PhasesCapped(usize)`, `CountsOnly`] | Already escapes each phase and caps with ` …` |
| Collapsed entry line | new line builder | `group_line(alias, &entry.group)` on the synthetic group, plus ` · 2 files` and a label count | Keeps 36 D-17 vocabulary and escaping |
| Copy text for `y` and the `inbox <alias>` entry block | two formatters | one `entry_checklist(alias, &entry)` in `inbox.rs` called by both (37 D-09 rule; today `group_checklist` at src/inbox.rs:472) | `y` must equal what the CLI prints |
| Word-boundary keyword matching | char scanner | `regex` `\b` | Unicode boundaries, multi-word gaps (`fixed\s+by`) |
| Phase equality (`7.1` vs `07.1`) | string compare | `state_reader::phase_num::phase_key` [VERIFIED: src/state_reader/phase_num.rs:169-179 `pub fn phase_key(id: &str) -> String` and `pub fn same_phase(a: &str, b: &str) -> bool { phase_key(a) == phase_key(b) }`] | Same normalization `apply_r7` uses |
| Escaping project strings | anything new | `text::terminal_capped` (CLI) / `shown` (TUI) | Enforced by `render_escape_guard` and `hostile_strings_are_escaped_in_json_and_text` |
| Refusing `o` on a multi-file row | new check | `owner_actions::open_target` already refuses [VERIFIED: src/ui/screens/owner_actions.rs:205-211 `n => Err(format!("{n} files behind this row: expand it with Enter, or press g"))`] | Merged entries pass `files` with 2 entries |

**Key insight:** every renderer primitive exists. The phase is mostly composition. The risk sits in test pins and identity plumbing, not in algorithms.

## Common Pitfalls

### Pitfall 1: Plain-text binary tests assert the legacy checklist on plain `inbox`
**What goes wrong:** the digest becomes the default, and these tests fail:
- `plain_inbox_prints_the_checklist_and_exits_zero` asserts `text.contains("\nTEST — phase stays open\n")` and `text.contains("    - [ ] ")` [VERIFIED: tests/inbox_cli.rs:565-576].
- `hostile_strings_are_escaped_in_json_and_text` asserts `assert!(text.contains("- [ ] "), "{text}");` on plain `inbox` [VERIFIED: tests/inbox_cli.rs:971-981].
- `human_verification_names_are_the_test_text` asserts `plain.lines().any(|l| l == "    - [ ] Weekly summary sends")` on plain `inbox` [VERIFIED: tests/inbox_cli.rs:1164-1169].
**How to avoid:** retarget these three to `--all` (legacy coverage kept). Add new tests that run the same hostile tree through the digest, `--lane` and `<alias>` views (escape and line-bound assertions). The other plain-text tests survive: `--lane test` keeps `TEST — ` headings (tests/inbox_cli.rs:579-593), `--lane decide` keeps `DECIDE` and no line starting with `TEST`/`FIX`/`BLOCKED` (tests/inbox_cli.rs:1213-1221), and the digest keeps `Triage: 1 pending todos in 1 projects` (tests/inbox_cli.rs:617-621), **provided** the lane view keeps the legacy section headings and the digest keeps the legacy fold wording.

### Pitfall 2: The key-path exactness test sees no `next_actions` element
**What goes wrong:** `the_inbox_key_paths_are_exactly_schema_v1` asserts `assert_eq!(key_paths(&doc), inbox_set());` over fixtures `uat-gaps-merge` and `supersede-six-phases` with the comment `// Every array the two fixtures fill is non-empty, so no path is excused.` [VERIFIED: tests/inbox_cli.rs:790-820]. `key_paths` walks array elements only when present (tests/inbox_cli.rs:726-751). Those fixtures carry `updated: 2026-01-10T12:00:00Z` / `2026-01-09T12:00:00Z` (grep of tests/fixtures/owner), so at the real clock every group is > 90 days old. Every entry folds, `summary.next_actions` is `[]`, and `summary.next_actions[].alias` and the like never appear.
**How to avoid:** append the new paths to `INBOX_PATHS` (existing entries untouched, as Phase 35 did with `summary.fix_debug` and `projects[].milestone.closes_after*` at tests/inbox_cli.rs:712-719), **and** add a fresh project to that test's registry: a copied fixture whose UAT has **no `updated:`** (age then comes from the copy's mtime, which `std::fs::copy` sets to now) or a file written with today's date. The README fence (`the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths`) needs a non-empty `next_actions` example. The table test (`the_readme_inbox_table_documents_every_path`) requires **exactly** one `| \`path\` |` row per path.
**Warning signs:** a path list that passes locally on a fresh checkout and fails months later. Prefer mtime-based freshness over a hard-coded recent date.

### Pitfall 3: New fixtures are run through the real `audit-uat` oracle
**What goes wrong:** `discover()` takes every directory under `tests/fixtures/owner/` that holds `.planning/` [VERIFIED: tests/owner_audit_conformance.rs:292-302], and `expectation()` panics `fixture {name} has no oracle.expect` [VERIFIED: tests/owner_audit_conformance.rs:312-315]. Allowed values are `["same", "no-phases-dir", "out-of-set-verification"]` [VERIFIED: tests/owner_audit_conformance.rs:55].
**How to avoid:** each new D-13 fixture gets `oracle.expect` = `same` and must be valid Layer A input. Run `rtk bypass cargo test --test owner_audit_conformance` (the oracle is installed here: `~/.claude/gsd-core/bin/gsd-tools.cjs`, node v26.13.1). The test code stays unchanged, which is what D-13 requires.

### Pitfall 4: Merge key too wide (debug sessions) or too narrow (phase spelling)
**What goes wrong:** see the Anti-Patterns entry on debug sessions. Handoff/Checkpoint/ContextQuestions are Decide-lane, one per file, and have their own `detail` wording. Merging them mixes "paused" with "N open questions".
**How to avoid:** restrict the merge to `Kind::Uat | Kind::Verification` (D-08's case and the only kinds that share a phase). Compare phases with `phase_num::phase_key`.

### Pitfall 5: Commands of parts can differ (Fix lane, unreadable)
**What goes wrong:** `run_for` returns `None` when `unreadable > 0` [VERIFIED: rules.rs:124-126 `if !matches!(file.scope, Scope::Current) || unreadable > 0 { return None; }`], so a Fix-lane UAT that partly failed to parse has no command while its VERIFICATION sibling has `/gsd-plan-phase N --gaps`. D-08's "command ... per phase and equal" holds only for the commands that exist.
**How to avoid:** synthetic `command`/`run` = the first `Some` among parts, and `counts.unreadable` = sum. `run_plan` then refuses `v` with "No command: the source file is unreadable" [VERIFIED: owner_actions.rs:68-70], which is fail-safe. Pin this in a unit test.

### Pitfall 6: Stale fold and merge interact
**What goes wrong:** a merged entry with one 120-day file and one 3-day file. Folding per group splits the entry.
**How to avoid:** merge first, then fold on the entry's age (the newest part). The JSON `folded_stale` of each part = the entry's flag.

### Pitfall 7: Reusing `RowSrc.merged` mislabels merged rows
**What goes wrong:** `row_cells` takes the name from `merged.map(inbox::blocked_tally)` when non-empty [VERIFIED: owner_block.rs:380-383 `let name = match merged.map(inbox::blocked_tally).filter(|t| !t.is_empty()) { Some(tally) => shown(&tally), None => shown(&group.phase_name), };`]. `blocked_tally` counts non-blocked categories as `other` (src/inbox.rs:282-289), so a merged TEST row would be named `3 other`.
**How to avoid:** carry the synthetic group in `RowSrc.group` with `merged: None`, and add a separate field (e.g. `parts: Vec<OwnerGroup>`) for the per-file expansion.

### Pitfall 8: Identity plumbing for `A`, `g`, `v`, `y` on a merged row
**What goes wrong:** `ack_key` and `run_key` resolve `owner_block::group_for(digest, &row.src.key)` and fall back to `row.src.group.clone()` [VERIFIED: src/ui/screens/inbox.rs:1591-1594 and 1682-1685]. For a merged row, either the key resolves to one part (acknowledging only that file silently) or the fallback hands a synthetic group to `ack_open`. `goto_key` passes `&row.src.key` to `DetailScreen::opened_on_owner` (inbox.rs:1657-1661), and the unmerged Owner block cannot find a synthetic key.
**How to avoid:** (a) `A` on a row with `files.len() > 1` is refused with a message in the `open_target` style, pointing to `g` (the inbox has no per-file cursor stops; expansion lines are `BodyItem::Extra`, inbox.rs:791-797). (b) `g` passes the first part's `GroupKey::of`. (c) `v` uses the synthetic group (shared command). (d) `y` copies `entry_checklist` (extend `groups_for_copy` or bypass it for merged rows). (e) Give the merged row a key that stays stable when the merge appears or disappears. Using the first part's `GroupKey::of` keeps the cursor on the row when the VERIFICATION resolves. A distinct key (`kind: "entry"`) makes misuse fail loudly. Pick one and test the refresh reanchor (`reanchor`, inbox.rs:461-474).

### Pitfall 9: The digest line budget is exceeded by worst-case folds and errors
**What goes wrong:** totals (1) + blank (1) + projects (cap 10 + `+N more` = 11) + blank (1) + `NEXT` heading + 5 (6) + blank (1) + folds (Old debt, Stale, Acknowledged, Deferred, Triage, Triage-unreadable = up to 6) + blank (1) + hint (1) = **29** lines, plus one line per failed project (render_text prints each, src/inbox.rs:785-800).
**How to avoid:** within Claude's discretion, lower the project cap (about 7 lines incl. `+N more` fits 25 in the worst case) or drop blank separators. Summarize failed projects in one line (`2 projects unreadable: a, b (inbox --all)`). Assert ≤ 25 lines on the worst-case synthetic 20-project fleet and 15-25 on the realistic fixture set.

### Pitfall 10: Counts of 100 or more break a fixed 9-cell checks column
**What goes wrong:** `pad` cuts to width [VERIFIED: owner_block.rs:206-214 `let cut = fit_cells(text, w);`], so `100 checks` becomes `100 check`.
**How to avoid:** size the digit field as `max(2, digits(n))` over the visible rows. Or keep 9 and widen only when a row needs it. Test with a 120-check group.

### Pitfall 11: Existing TUI tests use ages > 90 and will fold
**What goes wrong:** the inbox fixture fleet uses ages 203 (birch-cli, `test_group("18", "visual-parity-gate", 3, true, 203, true)`) and 182 [VERIFIED: src/ui/screens/inbox.rs:2073-2081 and :2136-2137], and `layout_tiers_140_120_100_80` asserts `text.iter().any(|l| l.contains("203 d stale"))` (inbox.rs:2824). With a 90-day fold those rows leave the lanes.
**How to avoid:** decide up front. Either set those fixture ages to e.g. 75/60 (still `stale` at > 30, not folded) and add separate > 90 rows for the fold tests, or update the assertions to look in the opened Stale fold. Both change 37 snapshots. Record the choice in the plan, since the 37 UAT is still pending against these layouts.

### Pitfall 12: The existing goldens' fixtures are 307 days old at the lib-level `now`
**What goes wrong:** lib tests read fixtures at `fn now() -> DateTime<Utc> { "2026-11-14T00:00:00Z".parse().unwrap() }` [VERIFIED: tests/owner_digest.rs:16-18], and the golden shows `six  ph 21  Export  2 checks  307 d stale  /gsd-verify-work 21` [VERIFIED: tests/fixtures/owner_golden/inbox.txt line 11]. A digest golden over `golden_entries()` (tests/owner_digest.rs:344-356) would fold everything into `Stale:` and show an empty `NEXT`.
**How to avoid:** build the digest/lane/alias goldens over the new fixtures (dated relative to `now()`, e.g. `updated: 2026-11-04`) plus the existing ones, or read the existing six with a second fixed instant (e.g. `2026-01-20T00:00:00Z`). The lib-level `now` parameter of `read_owner_digest` **is** the clock seam D-13 asks about. No binary-level seam exists (no env override in `src/`), and none should be added.

### Pitfall 13: `--all` byte-for-byte at binary level vs wall-clock ages
**What goes wrong:** SC2 says `inbox --all` reproduces the pre-phase output byte for byte. The binary prints wall-clock ages (`N d stale`), so a binary test cannot compare with `inbox.txt` directly (tests/inbox_cli.rs:5 `Ages depend on the wall clock, so no test pins age_days.`).
**How to avoid:** prove it at two levels. (1) `render_text` is untouched and `the_default_checklist_matches_the_golden` / `the_include_all_checklist_matches_the_golden` (tests/owner_digest.rs:373-388) stay green. (2) Add a binary test that runs `inbox --all` and `inbox --all --include archived,deferred,acknowledged,todos` over copies of the six golden fixtures, normalizes `\b\d+ d\b` on both sides, and compares with the goldens. Also unit-test `view_of` so `--all` always routes to `render_text`.

### Pitfall 14: `closes_after` is root-scope only
**What goes wrong:** `/// The gate of the root scope (workstream scopes are not exposed, [INFERRED] D-12).` [VERIFIED: src/state_reader/owner_items/gate.rs:27-28], with the filter `&& group.workstream.is_none()` (gate.rs:52). The live sentriq case (workstream `v0-15-buyers-mode`) and the `ws` golden project (`ws  ph alpha/03 ...` in inbox.txt) have owner work that `closes after` never mentions.
**How to avoid:** the D-03 rule "its lane counts alone" must also cover workstream work. For example, append a counts tail when the project's lane totals exceed `closes_after`'s, or show counts when `closes_after` has no lane clause. Golden-test both the `ws` and the `same-phase-merge` (workstream) projects.

## Code Examples

### The collapsed line already exists (reuse for entries)
[VERIFIED: src/inbox.rs:386-442 `fn group_line(alias: &str, group: &OwnerGroup) -> String`] builds `alias  ph ws/N  name  detail  [(archived)]  N d[ stale]  command` and omits the command for `bookkeeping_only`. Make it `pub(crate)` and call it on the synthetic group, then add ` · 2 files` and the dim/plain label count. The CLI has no dim, so print `2 decision?` as plain text.

### Lib-level golden pattern (copy for the three new goldens)
[VERIFIED: tests/owner_digest.rs:358-378]
```rust
fn assert_golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner_golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    ...
}
#[test]
fn the_default_checklist_matches_the_golden() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    assert_golden("inbox.txt", &render_text(&entries, &Filters::default()));
}
```
`entry_of(name, alias)` reads with `read_owner_digest(&planning, &state, now(), 30)` (tests/owner_digest.rs:45-63). New renderers take `fold_days` explicitly (pass 90).

### Same-phase fixture shape (synthetic, modeled on the live pair's metadata only)
The live UAT is `status: partial` with a comma-separated scalar `source: 43-01-SUMMARY.md, 43-02-SUMMARY.md, 43-03-SUMMARY.md`, and the VERIFICATION is `status: human_needed` (read-only peek this session; no text copied). The existing `source-mismatch` fixture is already a root-scope same-phase pair: `11-UAT.md` (`status: testing`, `source: [11-03-PLAN.md]`, 3 pending) plus `11-VERIFICATION.md` (`status: "human_needed"`, 2 `human_verification` entries) [VERIFIED: tests/fixtures/owner/source-mismatch/.planning/phases/11-intake/*]. Model the new `same-phase-merge` fixture on it, but place the phase under `.planning/workstreams/<ws>/phases/43-*/` with its own `ROADMAP.md`/`STATE.md` and a `43-01-PLAN.md` (layout of `tests/fixtures/owner/workstreams/.planning/workstreams/alpha/`). Expected text: one entry with 3 checks, `2 files`.

The 51-step live file is `04-HUMAN-UAT.md` with `status: testing`, 51 `### N.` headings, all `result: pending` (read-only count this session). The `scripted-checklist` fixture can be generated text (`### 1. Step 1` … `### 51. Step 51`).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Plain `inbox` = full checklist (361 lines on the owner's fleet) | Digest by default; `--all` keeps the old output | Phase 38 | Agents and scripts that parse plain text must add `--all` (README + `src/agent_skill.md` note); `--json` is unaffected |
| `--lane` = filtered full checklist | `--lane` = collapsed lane view (legacy with `--all --lane`) | Phase 38 | Item lines disappear from `--lane` output |
| Two rows for UAT + VERIFICATION of one phase | One merged entry in multi-project views and `<alias>`; JSON still per file + `entry_files` | Phase 38 | — |

**Deprecated/outdated:** none.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Next-action candidates should require a command (`group.command.is_some()`), which excludes CONTEXT-question groups (no command) and unreadable Fix groups | Pattern 4 / Open Q1 | NEXT could list an entry with no command, violating SC1 "each naming project, phase and command", or hide a Decide entry the owner wanted |
| A2 | `std::fs::copy` gives the new file a fresh mtime on Linux, so a fixture without `updated:` reads as age ~0 at the binary level [ASSUMED] | Pitfall 2 | Exactness test flaky; fall back to writing `updated:` from `chrono::Utc::now()` in the test |
| A3 | Regex `\b` treats `-` as a boundary, so `pre-review` matches `review` [ASSUMED from regex semantics, not run] | Pattern 5 | One more false-positive label; add a table row to pin the chosen behavior |
| A4 | Worst-case digest line count of 29 (Pitfall 9) assumes all six fold kinds fire at once | Pitfall 9 | Budget test on the synthetic fleet catches it |

## Open Questions (RESOLVED)

1. **Decide entries without a command in `NEXT`.**
   - What we know: CONTEXT-question groups have `command: None` (rules.rs:875-876 `command: None, run: None,`). SC1 wants each NEXT line to name a command.
   - Recommendation: exclude command-less entries from `next_actions` [inferred]. Alternatively print `(no command)`. Record as inferred in the plan.
   - RESOLVED: excluded. `next_actions` takes only entries with `group.command.is_some()` (38-01-PLAN.md Task 1, action step 1, and the D-07 must_haves truth); recorded as an inferred decision in the 38-01 SUMMARY.
2. **Whole-word forms and high-frequency words.** `review` and `accept` are common verbs in UAT step names ("Review the export screen"). The strict whole-word list does not match `approved`/`accepted`/`reviewed`/`acceptance`. That cuts false positives but may miss real rulings.
   - Recommendation: implement the D-11 list exactly as locked. During execution, count label hits on the owner's fleet read-only (`./target/release/gsd-meta-manager inbox --json | jq`) and report the counts in the SUMMARY for the D-14 readability UAT. Do not change the list without the owner.
   - RESOLVED: the D-11 list is implemented exactly as locked, with `approved`, `reviewed` and `acceptance` pinned as misses in `labels_read_the_d11_words_only` (38-01-PLAN.md Task 3, action step 1); the read-only label counts on the owner's fleet go into the 38-03 SUMMARY for the D-14 check (38-03-PLAN.md Task 2, action step 7). Any change to the list waits for the owner.
3. **`looks resolved` negations** ("not resolved yet"). `\bresolved\b` matches it.
   - Recommendation: keep the D-11 list and pin the behavior in the table test as a known false positive (labels only, low cost). Add a negation guard only if the owner asks.
   - RESOLVED: kept. `not resolved yet` is a pinned hit in `labels_read_the_d11_words_only` (38-01-PLAN.md Task 3 behavior and the D-11 must_haves truth); no negation guard.
4. **Lane view header.** D-05 says "sections exactly as the full output orders them". It does not say whether the totals line and the `closes after` block print.
   - Recommendation: print the totals line, the collapsed lane sections, the fold lines (with Stale), and no `closes after` block (it belongs to the digest). Existing tests stay green under this.
   - RESOLVED: `render_lanes` prints the totals line, the collapsed lane sections, the fold lines with Stale and the failed lines; no `closes after` block and no hint line (38-03-PLAN.md Task 1, action step 1, and its D-05 must_haves truth).
5. **`stale_folded` units.** Decide-lane entries (questions, sessions) are not "checks".
   - Recommendation: mirror the Old debt rule (src/inbox.rs:740-749): say `items` once any non-check entry is folded, else `checks`. The JSON carries `{checks, entries}` counts.
   - RESOLVED: the text says `items` once a folded entry is not a check, else `checks` (`stale_fold_text`, 38-01-PLAN.md Task 2, action step 3); the JSON carries `summary.stale_folded` as `{entries, items}` (38-03-PLAN.md Task 2, action step 1).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | build, tests | ✓ | 1.99.1 (MSRV floor 1.89) | — |
| node | `owner_audit_conformance` oracle | ✓ | v26.13.1 | `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` (opt-out, not for this phase) |
| gsd-core `gsd-tools.cjs` | oracle | ✓ | `~/.claude/gsd-core/bin/gsd-tools.cjs` | — |
| cargo-nextest | optional runner | ✗ | — | `cargo test` (all proven commands use it) |

Baseline run this session: `rtk bypass cargo test --no-fail-fast --test owner_digest --test inbox_cli` → `test result: ok. 25 passed` and `test result: ok. 30 passed`.

## Test Strategy (informational; `workflow.nyquist_validation` is false)

| Req | Proof | Command |
|-----|-------|---------|
| OWN-24 | `inbox-digest.txt` golden + line budget (fixtures, synthetic 20-project worst case); `next_actions` ranking table test (closes first, fewest checks, newer first, bookkeeping first, total order under permutation) | `rtk bypass cargo test --no-fail-fast --test owner_digest` / `rtk bypass cargo test --lib triage` |
| OWN-25 | `view_of` table test; `inbox-alias.txt`, `inbox-lane-test.txt` goldens; `inbox.txt` + `inbox-include-all.txt` unchanged; binary `--all` age-normalized compare; `INBOX_PATHS` appended + README rows | `rtk bypass cargo test --no-fail-fast --test inbox_cli` |
| OWN-26 | scripted-checklist: one line in digest/lane/TUI, 51 items in `<alias>` | owner_digest + TUI unit tests |
| OWN-27 | merge fixture (JSON two groups + `entry_files`, text one entry 3 checks); stale-fold fixture (folded line, totals unchanged equal to `summary`); TUI buffer snapshot of aligned checks column | `rtk bypass cargo test --lib ui::screens::inbox` |
| OWN-28 | labels table test incl. near misses; "labels never change rank/lane/count" test; JSON `items[].labels` | `rtk bypass cargo test --lib labels` |
| All | oracle unchanged and green on new fixtures; render escape guard; clippy | `rtk bypass cargo test --test owner_audit_conformance`; `rtk bypass cargo test --lib render_escape_guard`; `rtk bypass cargo clippy --all-targets -- -D warnings` |

Phase gate: `rtk bypass cargo test --no-fail-fast` (full) + clippy.

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | read-only; no writes (PROJECT.md constraint) |
| V5 Input Validation / Output Encoding | yes | every project-authored string through `terminal_capped` (CLI/JSON) or `shown` (TUI); labels are fixed strings; matching runs on raw text and emits only constants |
| V6 Cryptography | no | — |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal escape injection via phase names, item names, workstream names in new lines (digest NEXT, entry `2 files`, STALE section, file sub-headings in TUI expansion) | Tampering | `terminal_capped`/`shown` per piece; extend `hostile_strings_are_escaped_in_json_and_text` to digest/lane/alias views and `render_escape_guard`'s `hostile_owner_digest` with a same-phase pair and an age > 90 group |
| Command injection through a merged entry's command | Elevation | the synthetic `run` is a part's `RunCommand`, which was already validated by `is_command_token`/`is_workstream_name`; `run_plan` re-validates (owner_actions.rs:60-90) |
| Unbounded output (51+ items, huge fleets) | DoS | digest line cap; `<alias>` view is intentionally full; existing per-line `≤ 1000` chars assertion |
| ReDoS in label regexes | DoS | Rust `regex` is linear-time by construction |

## Sources

### Primary (HIGH confidence, read this session)
- `src/inbox.rs` (render_text, group_line, group_checklist, closes_after_clauses, build_document), `src/inbox_cli.rs`, `src/cli.rs:261-306`, `src/main.rs:105-127`, `src/config.rs:538-554`
- `src/state_reader/owner_items/{mod.rs, rules.rs, gate.rs}`, `src/state_reader/phase_num.rs:169-179`
- `src/ui/screens/{inbox.rs, owner_block.rs, owner_actions.rs, help.rs, render_escape_guard.rs}`
- `tests/{inbox_cli.rs, owner_digest.rs, owner_audit_conformance.rs}`, `tests/fixtures/owner/*`, `tests/fixtures/owner_golden/*`
- `.planning/phases/38-*/38-CONTEXT.md`, `.planning/REQUIREMENTS.md:110-132`, `.planning/phases/37-*/37-UAT.md`, `README.md:624-826`, `src/agent_skill.md`
- Read-only metadata of the two live cases (frontmatter keys and heading counts only)

### Secondary / Tertiary
- None. No web research was needed: the phase adds no dependency and touches no external API.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new dependencies; versions read from Cargo.toml)
- Architecture: HIGH (all integration points read; the recommended module split is a design choice within Claude's discretion)
- Pitfalls: HIGH (each tied to a quoted test or code line); A2/A3 are assumptions to confirm by test

**Research date:** 2026-11-16
**Valid until:** until Phase 38 execution starts (in-repo facts; re-check if Phase 37 fixes land on the branch first)
