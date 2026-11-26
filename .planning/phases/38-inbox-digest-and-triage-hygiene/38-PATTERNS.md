# Phase 38: Inbox digest and triage hygiene - Pattern Map

**Mapped:** 2026-11-16
**Files analyzed:** 26 (2 new modules, 4 new fixture dirs, 3 new goldens, 17 modified files)
**Analogs found:** 26 / 26 (every analog is git-tracked source under `src/` or `tests/`; no mirror paths)

All line numbers are from the worktree `/home/blk/projects/rust/gsd-meta-manager-owner-inbox` at the time of mapping.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/state_reader/owner_items/triage.rs` (NEW) | model / pure Layer B | transform | `src/state_reader/owner_items/gate.rs` (pure fn over `&[OwnerGroup]`) + `src/ui/screens/owner_block.rs::current_sources` (per-key merge) | exact (role + flow) |
| `src/state_reader/owner_items/labels.rs` (NEW) | utility / matcher | transform | `src/launcher/push.rs:186-193` (`LazyLock<Regex>`), `src/agents/fixers.rs:74-104` (word-boundary regexes); test: `rules.rs:2169-2219` | exact |
| `src/state_reader/owner_items/mod.rs` | module root | — | itself, lines 25-48 | exact |
| `src/inbox.rs` (new renderers + JSON keys) | renderer / document builder | transform (pure) | itself: `render_text` 633-804, `group_checklist` 472-481, `push_closes_after` 600-628, `group_json`/`item_json`/`build_document` 102-246 | exact |
| `src/inbox_cli.rs` | CLI controller | request-response | itself, `run` 94-149 | exact |
| `src/cli.rs` | config (clap) | — | itself, `Commands::Inbox` 261-279, tests 1184-1230 | exact |
| `src/main.rs` | entrypoint dispatch | — | itself, 113-126 | exact |
| `src/config.rs` | config | — | `Preferences::owner_stale_days` 538-554, test 1340-1361 | exact |
| `src/ui/screens/inbox.rs` | TUI screen | event-driven + render | itself: `inbox_rows` 204-233, `fold_rows` 319-354, `fleet_folds` 301-314, `fold_line_at` 1274-1381, `row_line` 680-761, key handlers 1557-1689 | exact |
| `src/ui/screens/owner_block.rs` | TUI component / view model | transform | itself: `Fold` 129-134, `RowSrc` 439-467, `row_cells` 369-437, `pad` 204-214, `expansion_lines` 1019-1052 | exact |
| `src/ui/screens/owner_actions.rs` | TUI action helpers | request-response | itself: `copy_text` 504-513, `fold_copy` 548-566, `open_target` 200-213 | exact |
| `src/ui/screens/help.rs` | TUI static content | — | itself, inbox block 528-543 | exact |
| `src/ui/screens/render_escape_guard.rs` | test (escape guard) | — | `hostile_owner_digest` 1014-1100 | exact |
| `README.md` | docs | — | itself, `#### Owner work: \`inbox\`` 624-826 | exact |
| `src/agent_skill.md` | docs | — | itself, lines 18, 24, 91 | exact |
| `tests/owner_digest.rs` | test (lib goldens) | file-I/O | itself: `now` 16-18, `entry_of` 45-63, `golden_entries` 344-356, `assert_golden` 358-371, golden tests 373-388 | exact |
| `tests/inbox_cli.rs` | test (binary) | request-response | itself: helpers 14-106, `INBOX_PATHS` 636-720, `key_paths` 726-751, exactness test 789-820, plain-text tests 564-622 | exact |
| `tests/fixtures/owner/same-phase-merge/` (NEW) | fixture | file-I/O | `tests/fixtures/owner/source-mismatch/` + `tests/fixtures/owner/workstreams/.planning/workstreams/alpha/` | exact |
| `tests/fixtures/owner/stale-fold/` (NEW) | fixture | file-I/O | `tests/fixtures/owner/uat-pending/` | exact |
| `tests/fixtures/owner/scripted-checklist/` (NEW) | fixture | file-I/O | `tests/fixtures/owner/uat-pending/` | exact |
| `tests/fixtures/owner/heuristic-labels/` (NEW) | fixture | file-I/O | `tests/fixtures/owner/uat-pending/` (+ `context-questions/` or `debug-awaiting/` for the Decide-lane item) | exact |
| `tests/fixtures/owner_golden/inbox-digest.txt` (NEW) | golden | — | `tests/fixtures/owner_golden/inbox.txt` | exact |
| `tests/fixtures/owner_golden/inbox-lane-test.txt` (NEW) | golden | — | `tests/fixtures/owner_golden/inbox.txt` | exact |
| `tests/fixtures/owner_golden/inbox-alias.txt` (NEW) | golden | — | `tests/fixtures/owner_golden/inbox.txt` | exact |
| `tests/owner_audit_conformance.rs` | test (oracle) | — | UNCHANGED; new fixtures must satisfy `discover`/`expectation` 291-320 | n/a |
| `tests/fixtures/owner_golden/inbox.txt`, `inbox-include-all.txt` | golden | — | UNCHANGED (become the `--all` goldens, byte-identical) | n/a |

## Pattern Assignments

### `src/state_reader/owner_items/triage.rs` (NEW; model, transform)

**Analog A (pure Layer B module shape):** `src/state_reader/owner_items/gate.rs`

Module doc + imports (lines 1-14): `//!` doc stating the rule and its source decision, `use super::{...}` for owner types, `crate::state_reader::phase_num::{phase_key, same_phase}` for phase equality:
```rust
//! R8, the milestone gate: which open phases of the current milestone close after which
//! owner work (Phase 35, D-12, OWN-08).
//! ...
use std::collections::BTreeSet;

use super::{ClosesAfter, Kind, Lane, LanePhases, OwnerGroup, Scope};
use crate::state_reader::phase_num::{phase_key, same_phase, PhaseNum};
use crate::state_reader::ProjectState;
```

Group filter idiom (lines 49-55) — copy for the merge candidate filter (current, not acknowledged, kind restricted):
```rust
for group in groups.iter().filter(|group| {
    matches!(group.scope, Scope::Current)
        && !group.acknowledged
        && group.workstream.is_none()
        && !matches!(group.kind, Kind::Debug | Kind::Handoff)
        && same_phase(&group.phase, &phase.number)
}) {
```
For D-08 the predicate is `matches!(g.scope, Scope::Current) && !g.acknowledged && matches!(g.kind, Kind::Uat | Kind::Verification)`; the merge key is `(g.workstream.clone(), phase_key(&g.phase), g.lane)` (RESEARCH Pitfall 4). Note `gate.rs` exposes `pub(super) fn`; triage must be `pub` (it is called from `crate::inbox` and the TUI).

Test-helper builders (lines 104-170): `#[cfg(test)] mod tests { use super::*; fn item() -> OwnerItem {..} fn group(lane, kind, phase, items, unreadable) -> OwnerGroup {..} }` — copy these literal builders; `OwnerGroup` has no `Default`.

**Analog B (merge into one row with parts + files):** `src/ui/screens/owner_block.rs::current_sources` lines 472-519. The BLOCKED aggregate is the existing "collect parts by key, concatenate items, dedup files" algorithm:
```rust
let parts: Vec<&OwnerGroup> = digest.groups.iter()
    .filter(|g| is_owner_row(g) && g.lane == Lane::Blocked
        && g.workstream == key.0 && g.phase == key.1)
    .collect();
let mut merged = group.clone();
merged.items = parts.iter().flat_map(|g| g.items.iter().cloned()).collect();
let mut files: Vec<String> = Vec::new();
for part in &parts {
    if !files.contains(&part.file) { files.push(part.file.clone()); }
}
```
For the synthetic group also sum `counts.{checks,failed,gaps,unreadable}`, take `age_days = min` over parts (newest), recompute `stale` from that age (do NOT OR it), and take `command`/`run` = first `Some` (Pitfall 5). Fold on the entry's age, after the merge (Pitfall 6). Age semantics: `rules.rs:240-247` (`days_since`, `age_of`), stale comparison `rules.rs:439` `age_days.is_some_and(|days| days > i64::from(ctx.stale_days))` — use the same `>` with `fold_days`; `None` never folds.

**Analog C (ranking, total order):** two sorts to mirror.
- `src/inbox.rs:674-682` (CLI TEST sort: closes first, then tie-breakers via `then_with(|| by_location(a, b))`).
- `src/ui/screens/inbox.rs:260-286` `sort_groups` (doc comment states "the order is **total**: every permutation of the same rows sorts to the same sequence"; chained `.then_with` ending in alias, `phase_key`, kind, file, workstream). Its `by_age` (236-243) is oldest-first; D-07 needs the inverse (newer first, `None` last). Write a new comparator; do not reuse `by_age`.
- Ranking rule from RESEARCH Pattern 4: `closes = group.closes_phase || group.bookkeeping_only`, `checks = if bookkeeping_only {0} else {counts.checks}`; candidates: not folded, lane in {Test, Decide, Fix}, `Scope::Current`, `command.is_some()` (Open Q1, inferred).
- Permutation test pattern: `src/ui/screens/inbox.rs` test `sort_groups_breaks_every_tie_the_same_way` (line 2309).

---

### `src/state_reader/owner_items/labels.rs` (NEW; utility, transform)

**Regex static pattern:** `src/launcher/push.rs:186-193` (function-local `LazyLock<Regex>`, `.expect("static regex")`):
```rust
pub fn github_repo(url: &str) -> Option<String> {
    static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:git@github\.com:|...)$")
            .expect("static regex")
    });
    let caps = GITHUB.captures(url.trim())?;
```
Word-boundary + case-insensitive convention: `src/agents/fixers.rs:100-104` `Regex::new(r"(?i)\bphase (\d+(?:\.\d+)?)\b").expect("static regex")`; one doc comment per regex naming what it matches (fixers.rs 74, 80, 86, 100). Import line: `use regex::Regex;` (fixers.rs:33). `LazyLock` is already imported in `owner_items/mod.rs:40`.

**Table-driven test with false-positive guards:** `src/state_reader/owner_items/rules.rs:2169-2219` `is_answered_reads_explicit_markers_only` — a `for line in [...] { assert!(f(line), "{line}"); }` block of hits, then a block of misses with `// WR-01: ...` comments explaining each guard. Copy this shape: hits per keyword, misses `unresolved`, `preview`, `okay`, `ok`, `acceptance`, `approved`, `reviewed`; a Decide-lane item containing `decide` → no label; pin the known false positive `not resolved yet` (Open Q3).

**Hand-rolled alternative (do not use, but the boundary rules are documented here):** `rules.rs:708-726` `starts_with_answered_word`.

Label strings are constants (`"decision?"`, `"looks resolved"`), so they need no escaping; matching runs on raw text.

---

### `src/state_reader/owner_items/mod.rs` (modify)

Module list + crate re-export idiom (lines 25-48):
```rust
mod context;
mod debug;
...
mod gate;
...
// The launch path re-validates the tokens it builds an argv from (D-11).
pub(crate) use rules::{is_command_token, is_workstream_name};
```
Add `mod labels; mod triage;` in alphabetical order and `pub use triage::{...}; pub use labels::{...};` (they are consumed by `crate::inbox`, the TUI, and lib-level tests in `tests/owner_digest.rs`, so `pub`, not `pub(crate)`). Keep `read_owner_digest` (621-644) signature unchanged (RESEARCH summary: 4+ call sites).

---

### `src/inbox.rs` (modify; renderer, transform)

**Rule: `render_text` (633-804) stays byte-for-byte untouched.** New functions go beside it.

**Imports** (13-21): `use crate::state_reader::owner_items::{Category, ClosesAfter, Kind, Lane, OwnerDigest, OwnerGroup, OwnerItem, Scope};` + `use crate::text::terminal_capped;` — extend the list with the triage/labels exports.

**Totals line to reuse verbatim in digest/lane/alias views** (643-650):
```rust
let mut out = vec![format!(
    "Owner inbox: {} projects · test {} · decide {} · fix {} phases{debug_tail} · blocked {}",
    entries.len(), sum(|d| d.test), sum(|d| d.decide), sum(|d| d.fix_phases), sum(|d| d.blocked),
)];
```
Extract into a private helper only if `render_text`'s output stays identical (simplest: duplicate the 15 lines; the golden tests `the_default_checklist_matches_the_golden` guard it).

**Per-project `closes after` line** (600-628 `push_closes_after`): copy its alias/version formatting and call `closes_after_clauses(gate, ClauseForm::PhasesCapped(4))` (549-592) instead of `ClauseForm::Full`. Note gate is root-scope only (`gate.rs:27-28`); for workstream work append lane counts (Pitfall 14).

**Collapsed entry line**: `group_line` (386-442) — make `pub(crate)`, call on the synthetic group, then append ` · 2 files` / label count. Vocabulary already there: `ph ws/N`, `N d stale`, bookkeeping omits command (435-439).

**Full checklist for an entry** (shared with TUI `y`, 37 D-09): copy `group_checklist` (469-481):
```rust
pub(crate) fn group_checklist(alias: &str, group: &OwnerGroup) -> Vec<String> {
    let mut out = vec![format!("  {}", group_line(alias, group))];
    for item in &group.items {
        out.push(format!("    - [ ] {}", terminal_capped(&item.name)));
        for gap in &item.gaps {
            out.push(format!("        - {}", terminal_capped(&gap.name)));
        }
    }
    out
}
```
New `entry_checklist(alias, &entry)` = same, with per-item label suffix and (for merged entries) per-file grouping. Keep `group_checklist` unchanged (used by `render_text` via `push_section` 458-467 and `old_debt_checklist` 486-506).

**Section headings / order for the lane view**: `render_text` 669-697 — same headings (`"TEST — closes a phase"`, `"TEST — phase stays open"`, `"DECIDE"`, `"FIX — code needed"`, `"BLOCKED"`) and same sort (674-682 TEST, `by_location` 509-515 for the others), since `tests/inbox_cli.rs:591` and `:1213-1221` assert those headings survive under `--lane`.

**Fold lines** (734-783): copy wording exactly (`Old debt: ... (archived milestones)`, `Acknowledged: N file(s)`, `Deferred: N item(s)`, `Triage: N pending todos in P projects`), insert `Stale: N checks in P phases older than 90 d` beside them. Unit rule (`items` vs `checks`) mirrors 740-746 (Open Q5).

**Failed entries** (785-800): one line per failed project today; the digest should summarize to one line (Pitfall 9).

**JSON additive keys**: `item_json` (102-119) gains `"labels": [...]`; `group_json` (121-151) gains `"folded_stale"` and the merge link (e.g. `"entry_files"`). Both take only `&OwnerItem`/`&OwnerGroup` today, so pass the computed triage data in (e.g. `group_json(group, &EntryInfo)`); `project_json` (153-214) iterates `entry.digest.groups` with `filters.lists_group` (176-182) — compute `triage::entries(&entry.digest, fold_days)` there. `build_document` (217-246) `summary` gains `next_actions` and `stale_folded`; signature grows a `fold_days: u32` param (callers: `inbox_cli.rs:142`, `tests/owner_digest.rs`, unit tests 865-878). Every string through `terminal_capped`/`opt_text` (80-82).

**Unit-test builders** (806-863): `fn group(lane) -> OwnerGroup` with hostile `phase_name: "Alpha\u{1b}[31m"` and `fn entry(groups) -> ProjectEntry` — reuse for new renderer unit tests.

---

### `src/inbox_cli.rs` (modify; controller, request-response)

**Analog:** itself. Current dispatch (141-147):
```rust
if json {
    out_line(&build_document(&entries, &filters).to_string());
} else {
    for line in render_text(&entries, &filters).lines() {
        out_line(line);
    }
}
```
Add a pure `view_of(json, all, targets, lanes, includes) -> View` (RESEARCH Pattern 1 sketch) and match on it. `fold_days` comes from `config.preferences.owner_fold_days()` next to `owner_stale_days()` (68-73). `run` signature (94-100) grows `all: bool`.

**Pure helper + mapping style:** `lane_of` (39-46), `filters_of` (48-56). The file has no `#[cfg(test)]` yet; add one for the `view_of` table test, following `src/status_cli.rs:369-409` (binary-crate `mod tests { use super::*; #[test] fn ... }` with plain `assert_eq!`).

Module doc numbered facts (1-25): fact 2 ("Without `--json`, the plain checklist of `inbox::render_text` is printed") must be updated to describe the views.

---

### `src/cli.rs` (modify; config)

**Flag pattern** (266-279):
```rust
Inbox {
    projects: Vec<String>,
    /// Print the inbox as one JSON document on stdout
    #[arg(long)]
    json: bool,
    /// List only these lanes (comma-separated; default: all four)
    #[arg(long, value_delimiter = ',')]
    lane: Vec<InboxLane>,
    ...
},
```
Add `/// Print the full checklist (the pre-digest output)` `#[arg(long)] all: bool,`. Update the doc line on 265 (it says "The checks you have to run").

**Tests** (1184-1230): both `inbox_parses_without_targets` and `inbox_parses_comma_separated_lanes_and_includes` destructure every field of `Commands::Inbox { projects, json, lane, include }` — add `all` to both patterns and assert `!all` / `all` (add `--all` to the second argv).

---

### `src/main.rs` (modify)

Lines 113-126: add `all` to the destructure and pass it to `inbox_cli::run(&projects, json, all, &lane, &include, cli.config.clone())`.

---

### `src/config.rs` (modify; config)

**Analog:** `owner_stale_days` (538-554), copy shape exactly with key `fold_days`, default 90:
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
Doc comment pattern (539-545) names the key path and the clamp range. **Test:** copy `owner_stale_days_defaults_to_thirty_and_clamps` (1340-1361): absent → default, 45 → 45, 0 → 1, 99999 → 3650, `"45"` string → default, `"owner": "x"` → default.

---

### `src/ui/screens/owner_block.rs` (modify; component / view model)

**`Fold` enum** (128-134): add `Stale`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fold { OldDebt, Acknowledged, Todos }
```
`Fold` is matched exhaustively in `inbox.rs:327-331` (`fold_rows`), `owner_actions.rs:555-565` (`fold_copy`), and the Owner block's `selectable_folds` (580-589, which must NOT offer `Stale`: the detail block is frozen, D-06). Grep `Fold::` for every arm.

**`RowSrc`** (439-467): add a separate field (e.g. `parts: Vec<OwnerGroup>`) for the per-file expansion; keep `merged: None` for same-phase entries — `row_cells` (380-383) reads `merged` as a BLOCKED tally (Pitfall 7):
```rust
let name = match merged.map(inbox::blocked_tally).filter(|t| !t.is_empty()) {
    Some(tally) => shown(&tally),
    None => shown(&group.phase_name),
};
```
`RowSrc::single` (450-458) is the constructor to extend; `current_sources` (472-519) stays unchanged for the detail block — add a new `inbox_sources(digest, fold_days)` beside it.

**Checks cell helper (D-10):** today `row_cells` (388-392) builds `inbox::plural(group.counts.checks, "check", "checks")` and `row_line` (781) right-pads the whole string with `pad(&row.checks, CHECKS_W, true)`. `pad` (204-214) cuts with `fit_cells`, so a fixed width truncates `100 checks` (Pitfall 10). Add `pub(crate) fn checks_cell(n: usize) -> String` = digits right-aligned, unit left-aligned (`format!("{n:>2} {unit:<6}")`, wider for n ≥ 100) and use it in both `row_cells` and the inbox screen. `CHECKS_W` exists in both files (owner_block.rs:37 and inbox.rs:519).

**Expansion with per-file sub-headings:** `expansion_lines` (1019-1052) — items capped at `MAX_ITEMS`, gap lines, `… N more`, then `file {path}` lines via `dim_line(4 + shift, ...)`. For merged entries emit a `dim_line` file sub-heading before each part's items instead of the trailing `file` lines. Labels go as a dim suffix on the item line (`item_text` 987).

---

### `src/ui/screens/inbox.rs` (modify; screen, event-driven)

**Row source:** `inbox_rows` (199-233) calls `owner_block::current_sources(digest)`; switch to `owner_block::inbox_sources(digest, fold_days)` and skip folded entries. `fold_days` must come from the cached config on `ctx` (render reads caches only, `tests/async_blocking_guard.rs`).

**Stale fold row:** mirror the Old debt fold end to end:
- `fleet_folds` (298-314): push `Fold::Stale` when the stale total > 0.
- `fold_rows` (316-354): add a `Fold::Stale => <folded inbox sources>` arm.
- `stop_rows` (358-367) and `prune_view` (436-453) iterate `[Fold::OldDebt, Fold::Acknowledged]` — add `Fold::Stale`.
- `FoldTotals` + `fold_totals` (1220-1262) and `fold_line_at` rungs (1294-1350): add a stale part with three rungs, e.g. `Stale: N checks in P phases older than 90 d` / `Stale: N checks` / `Stale (N)`, `false` (not dim), `Fold::Stale`.
- `build_plan` (799-827) `Fold::OldDebt` sub-header logic shows how fold rows are inserted.

**Aligned checks column:** `row_line` 716 `pad(&c.cells.checks, CHECKS_W, true)` and title head 777 `pad("checks", CHECKS_W, true)`; test assertion `sc2_legible_at_100x30` line 3046 `assert_eq!(checks_at, grid.checks_x() + 3, "right-aligned over Checks")` moves with it.

**Key handlers on a merged row** (1557-1689):
- `open_key` → `owner_actions::open_target(ctx, &row.alias, &row.src.files)` already refuses `n > 1` files (owner_actions.rs:205-212). No change.
- `ack_key` (1580-1605): add an early refusal when `row.src.files.len() > 1`, copying the `open_target` message style: `format!("{n} files behind this row: expand it with Enter, or press g")`, then `Self::say(ctx, message)`.
- `copy_key` (1610-1636): uses `owner_block::groups_for_copy` + `owner_actions::copy_text`; for merged rows copy `inbox::entry_checklist` instead.
- `goto_key` (1644-1662): pass the first part's `GroupKey::of` to `DetailScreen::opened_on_owner`.
- `run_key` (1666-1689) / `expansion_for` (838-859): `group_for(...).unwrap_or_else(|| row.src.group.clone())` — falls back to the synthetic group, whose command is shared. Pin with a test.

**Test builders + snapshots:** `fn group(g: G)`, `fn digest(groups)`, `fn test_group(phase, name, checks, closes, age, stale)` (1918-2029), `inbox_fleet_ctx` (2031+), `draw`/`render_inbox_text` with `TestBackend` (2377-2399), column checks in `sc2_legible_at_100x30` (3015-3063). Fixture ages 203 and 182 (birch-cli, ~2073-2137) exceed 90 and will fold (Pitfall 11): decide in the plan.

---

### `src/ui/screens/owner_actions.rs` (modify)

`fold_copy` (548-566) — add a `Some(Fold::Stale)` arm; either copy the stale entries' checklists (like `old_debt_text` 515-531 does for Old debt) or refuse with a message in the existing wording style (`"Nothing to copy: ..."`). `copy_text` (504-513) maps `group_checklist` over groups; add an entry variant that maps `inbox::entry_checklist` so `y` equals the CLI (37 D-09). `finish_copy_text` (533-546) unchanged.

---

### `src/ui/screens/help.rs` (modify)

Inbox block 528-543 uses `row(key, description)`, every description unique in the body. Line 538 `row("Left / Right", "On the fold line: move between Old debt, Acknowledged and Triage (inbox)")` gains `Stale`; the same string is duplicated in the test expectations at line 924 — update both. Add a row describing the Stale fold (> 90 d) and the `decision?` / `looks resolved` labels if they need a legend.

---

### `src/ui/screens/render_escape_guard.rs` (modify; test)

`hostile_owner_digest(identity)` (1014-1100) builds groups via a `group(lane, scope, workstream)` closure with `age_days: Some(3)`. Add: a same-phase pair (a `Kind::Uat` + `Kind::Verification` group with equal phase/lane, both hostile) and a group with `age_days: Some(120)` so the merged row, the stale fold row and file sub-headings are drawn with hostile text.

---

### `tests/owner_digest.rs` (modify; lib goldens)

Copy `assert_golden` (358-371, `UPDATE_GOLDEN` env rewrites) and the golden test shape (373-378):
```rust
#[test]
fn the_default_checklist_matches_the_golden() {
    let held = golden_entries();
    let entries: Vec<ProjectEntry> = held.iter().map(|(_, e)| e.clone()).collect();
    assert_golden("inbox.txt", &render_text(&entries, &Filters::default()));
}
```
`entry_of` (45-63) reads at fixed `now()` = `2026-11-14T00:00:00Z` (16-18) with `read_owner_digest(&planning, &state, now(), 30)` — this is the clock seam. New goldens use a new fixture list (the four D-13 fixtures, dated e.g. `updated: 2026-11-04`) plus the six existing ones only if read at a second fixed instant (Pitfall 12); the existing six are 307 days old at `now()`. Keep `the_default_checklist_matches_the_golden` and `the_include_all_checklist_matches_the_golden` unchanged.

---

### `tests/inbox_cli.rs` (modify; binary tests)

- Helpers to reuse: `fixture` (14-18), `registry` (20-38), `inbox_with` (40-49), `json` (58-67), `write` (82-85), `copy_tree` (822-833).
- `INBOX_PATHS` (636-720): **append** new paths at the end, like Phase 35 did at 712-719; never edit existing entries.
- `the_inbox_key_paths_are_exactly_schema_v1` (789-820): uses `uat-gaps-merge` + `supersede-six-phases` dated 2026-01, so everything folds and `summary.next_actions[]` paths never appear (Pitfall 2). Add a fresh project to its registry (a copied fixture without `updated:` so mtime = now, or written with today's date).
- README tests (835-853): `the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths` and `the_readme_inbox_table_documents_every_path` require a fence with non-empty `next_actions` and exactly one `| \`path\` |` row per path.
- Retarget to `--all`: `plain_inbox_prints_the_checklist_and_exits_zero` (564-576), the plain-text half of `hostile_strings_are_escaped_in_json_and_text` (~971-981), `human_verification_names_are_the_test_text` (~1164-1169) (Pitfall 1). Then add digest/lane/alias variants of the hostile test.
- Survivors that constrain the new views: `a_lane_filter_lists_only_that_lane_in_both_modes` (578-593: `TEST — ` heading present, no `BLOCKED\n`), `include_todos_fills_the_todo_list_and_only_then` (617-621: digest must keep `Triage: 1 pending todos in 1 projects`).
- New `--all` byte test: run `--all` and `--all --include archived,deferred,acknowledged,todos` over copies of the six golden fixtures, normalize `\b\d+ d\b` on both sides, compare with `inbox.txt` / `inbox-include-all.txt` (Pitfall 13).
- 20-project fleet generator for the line budget: `generate_project` (984) + `a_cold_inbox_over_twenty_projects_is_under_a_second` (1061).

---

### `tests/fixtures/owner/same-phase-merge/` (NEW; fixture)

**Analog:** `tests/fixtures/owner/source-mismatch/` (root-scope same-phase pair) placed in the workstream layout of `tests/fixtures/owner/workstreams/.planning/workstreams/alpha/`.

`source-mismatch/.planning/phases/11-intake/11-UAT.md`:
```
---
status: testing
source: [11-03-PLAN.md]
updated: 2026-01-10T12:00:00Z
---

# UAT

## Tests

### 1. Form fields
expected: form fields behaves as described
result: pending
```
`source-mismatch/.planning/phases/11-intake/11-VERIFICATION.md`:
```
---
status: "human_needed"
updated: 2026-01-09T12:00:00Z
human_verification:
  - "check the intake form on a phone"
  - "check the confirmation mail"
---

# Verification
```
Workstream files: `workstreams/.planning/workstreams/alpha/{ROADMAP.md,STATE.md,phases/03-stream/03-01-PLAN.md,03-01-SUMMARY.md,03-UAT.md}`; root `ROADMAP.md`/`STATE.md` as in `workstreams/.planning/`. Target: UAT `status: partial`, `source:` naming only SUMMARYs (comma-separated scalar), 2 checks; VERIFICATION `human_needed`, 1 entry; recent `updated:` dates. Plus `oracle.expect` containing `same`.

### `tests/fixtures/owner/stale-fold/`, `scripted-checklist/`, `heuristic-labels/` (NEW; fixtures)

**Analog:** `tests/fixtures/owner/uat-pending/` — `oracle.expect` (`same`), `.planning/ROADMAP.md`:
```
# Roadmap

## Phases

- [ ] **Phase 01: Alpha** - the one phase of this fixture

### Phase 01: Alpha

**Goal**: exercise the owner inbox
**Depends on**: Nothing
```
`.planning/STATE.md`: `---\nstatus: in_progress\n---\n\n# State\n`; `.planning/phases/01-alpha/01-UAT.md` with `### N. Name` / `expected:` / `result: pending` blocks. Add `01-01-PLAN.md` where the phase must count as built (`gate.rs:40-46`, `not built` otherwise). `stale-fold`: two phases, one `updated:` > 90 days before the golden `now`, one fresh. `scripted-checklist`: 51 generated `### N. Step N` blocks, `status: testing`. `heuristic-labels`: keyword hits and near misses in names/`expected:`; for the Decide-lane unlabeled case use the shape of `tests/fixtures/owner/context-questions/` (CONTEXT open questions) or `debug-awaiting/`. Every new dir is fed to the oracle by `tests/owner_audit_conformance.rs:291-320` (`discover` + `expectation` panics without `oracle.expect`).

### `tests/fixtures/owner_golden/inbox-digest.txt`, `inbox-lane-test.txt`, `inbox-alias.txt` (NEW; goldens)

**Analog:** `tests/fixtures/owner_golden/inbox.txt` — format reference (two-space indent for entry lines, `    - [ ] ` items, `        - ` gaps, blank line between blocks, fold lines unindented, trailing newline). Generated with `UPDATE_GOLDEN=1` via `assert_golden`.

---

## Shared Patterns

### Escaping project text
**Source:** `src/text.rs` `terminal_capped` (CLI/JSON), `src/ui/screens/detail.rs` `shown` (TUI)
**Apply to:** every new string in `inbox.rs` renderers, `group_json`/`item_json` additions, TUI rows, file sub-headings
Every project-derived piece is escaped on its own, the generated structure is not (`inbox.rs:594-599` WR-03 comment; `push_closes_after` 617-621):
```rust
Some(format!("  {}{version} closes after: {}", terminal_capped(&alias_of(entry)), parts.join(" · ")))
```
Guarded by `render_escape_guard.rs` and `tests/inbox_cli.rs::hostile_strings_are_escaped_in_json_and_text` (870+).

### One shared formatter CLI ↔ TUI (37 D-09)
**Source:** `src/inbox.rs:469-481` `group_checklist`, consumed by `owner_actions.rs:507-513` `copy_text`
**Apply to:** `entry_checklist` (CLI `inbox <alias>` + TUI `y`), label wording, `checks_cell`, Stale fold wording (CLI fold line + TUI fold line)

### Pure functions, caller supplies `now` / thresholds
**Source:** `owner_items/mod.rs:19-20` ("No function here reads the clock"), `read_owner_digest(.., now, stale_days)` (621-626)
**Apply to:** `triage::entries(digest, fold_days)`, `next_actions`, `labels_of`; no clock read, no I/O; config threshold via `Preferences::owner_fold_days()`

### Clamped owner preference
**Source:** `src/config.rs:538-554`
**Apply to:** `owner_fold_days` (key `preferences.owner.fold_days`, default 90, clamp `1..=3650`)

### Total-order sorts with permutation tests
**Source:** `src/ui/screens/inbox.rs:260-286` (`sort_groups`), test `sort_groups_breaks_every_tie_the_same_way` (2309)
**Apply to:** `next_actions` ranking (D-07), digest project ordering (closeable phases desc, then alias)

### Phase equality
**Source:** `src/state_reader/phase_num.rs:169-179` `phase_key` / `same_phase` (used by `gate.rs:13`)
**Apply to:** merge key in `triage.rs`. Do not confuse with `crate::inbox::phase_key` (253-256), which is a sort key `(u64, String)`, not a normalizer.

### Refusal messages
**Source:** `owner_actions.rs:205-212` (`"{n} files behind this row: expand it with Enter, or press g"`), `fold_copy` 560-564 (`"Nothing to copy: ..."`)
**Apply to:** `A` on a merged row, `y` on the Stale fold (if refused)

### Golden + env-rewrite tests
**Source:** `tests/owner_digest.rs:358-371`
**Apply to:** the three new goldens

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| (none) | | | Every file has an in-repo analog. Closest gaps: the D-07 "newer first" comparator inverts `by_age` (inbox.rs:236-243) rather than copying it, and `inbox_cli.rs` has no test module yet (use `status_cli.rs:369+` as the model). |

## Metadata

**Analog search scope:** `src/inbox.rs`, `src/inbox_cli.rs`, `src/cli.rs`, `src/main.rs`, `src/config.rs`, `src/cli_targets.rs`, `src/status_cli.rs`, `src/state_reader/owner_items/{mod,gate,rules}.rs`, `src/agents/fixers.rs`, `src/launcher/push.rs`, `src/ui/screens/{inbox,owner_block,owner_actions,help,render_escape_guard}.rs`, `tests/{owner_digest,inbox_cli,owner_audit_conformance}.rs`, `tests/fixtures/owner/{source-mismatch,workstreams,uat-pending,context-questions}`, `tests/fixtures/owner_golden/inbox.txt`, `README.md`, `src/agent_skill.md`
**Files scanned:** 24
**Pattern extraction date:** 2026-11-16
