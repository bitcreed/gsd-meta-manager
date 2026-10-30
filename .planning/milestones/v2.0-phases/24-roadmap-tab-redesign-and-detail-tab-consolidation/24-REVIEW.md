---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - docs/CONFIGURATION.md
  - docs/DRIVER-SANDBOX.md
  - docs/GSD-CORE-SYNC.md
  - src/agents/adapters/mod.rs
  - src/config.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/executor/mod.rs
  - src/launcher/config_dir.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/launcher/stop.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 7
  info: 7
  total: 14
new_findings:
  critical: 0
  warning: 0
  info: 2
status: issues_found
---

# Phase 24: Code Review Report (incremental re-review at HEAD)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 22 (this pass); the 19 files of the prior pass are listed under "Prior review (e16a201)"
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This file is the union of two passes. The first, the HEAD re-verification at `e16a201`, is preserved verbatim below under "Prior review (e16a201)", and its twelve findings (WR-01..WR-07, IN-01..IN-05) are carried into the sections that follow, unchanged and still open. The second is an incremental pass over `git diff e16a201 HEAD` for the 22 files in the frontmatter list. `git log` shows no Phase 24 task commit since the prior review, so this delta is not Phase 24 work. It is the quick batches 261028-t3c / 261028-v2z (driver lock and direction hardening, `driverctl` store and serve, the fail-closed managed-settings merge) and quick 261029-1dp (`f4cfa01`, the SAFE-04 redacting capture pump; `ce86206`, removal of the dead `AgentRuntime` config key and the doc-comment corrections).

**What touches Phase 24's own surface.** Only two hunks do. `src/ui/screens/detail.rs:21260-21277` is a doc comment and an assertion message inside a test module, and `src/ui/screens/mod.rs:1389-1393` is the doc comment on `AppContext::experimental`. Neither changes the Roadmap tab, the Detail tab strip, the roadmap model, or any key handler. I read both against the code they describe. The claim that `driver_opt_in` "gates no launch; it only shapes the push policy" holds: `is_opted_in` has no caller outside `registry.rs` and the confirm popup, and the only readers of the record are `launcher::push::policy_for` (via `app.rs:1057`, `driver_cli.rs:292,523` and `driver_launch.rs:342`). The record is also documented as superseded in `REQUIREMENTS.md:24` (CTRL-03). The `GsdConfig::runtime` doc in `config_json.rs:207-217`, which the Defaults tab renders, agrees with the code.

**What I traced in the rest of the delta.**
- The shared guarded lock-file open (`lock.rs` `open_lock_file`, `ensure_lock_file`, and the read-side `probe_at` / `read_holder_at`). It is applied consistently, and all three refusal kinds (symlink, non-regular, hard link) reach `LockProbe::Unknown` or `LockError::Unavailable`.
- `exchange_or_restore` and its two callers in `direction.rs`. The retry is bounded by `GIVE_BACK_ROUNDS` in the helper and by `SPLICE_TRIES` in `run_loop`.
- The guarded `state.json` reads in `store.rs`, `stop.rs` and `run.rs`.
- The pump, `pump_redacted` (`process.rs:96-209`), with its caller in `run.rs:1629-1703`. I found no live reader of `launch-N.jsonl` before the pump has finished, and `capture.finish` runs before `summarize_stream_file`.
- The `well_formed_hooks` / `well_formed_permissions` merge in `config_dir.rs`, the new integration tests, and the `AgentRuntime` removal.

No critical or security defect was found. Two new info items are recorded; no new warnings.

**Already recorded elsewhere, not re-raised here.**
- Multi-line PEM left unredacted by the line pump: Phase 29 WR-14, Phase 31 WR-01.
- A pump-thread spawn failure leaves the child unsupervised: Phase 29 WR-15, Phase 31 WR-02.
- The pump's launcher-death rationale does not hold for the degraded posture: Phase 30 WR-16.
- The read-side link-count refusal turns a held run lock into `Unknown`: Phase 29 WR-08.
- First-doc-wins for managed `permissions` scalars: Phase 29 WR-09, Phase 30 WR-14.
- Hook-item validation stops at `type`: Phase 30 WR-15.
- Silent drop of wrong-shape host policy: Phase 29 WR-10, Phase 30 IN-18, Phase 31 WR-03.
- Dead branch and shadowed `key` in the managed merge: Phase 30 IN-19.
- A stale test name, `opting_in_from_the_driver_tab_flips_the_spawn_seam` (`detail.rs:21271`, a Phase 24 file): Phase 30 IN-20.
- A stale `driver_max_concurrent` enforcement claim: Phase 30 IN-21.
- An over-cap line skipped by the reader: Phase 29 IN-07.
- The ignored capture-finish timeout: Phase 29 IN-08.
- The stale redaction-corpus assertion message: Phase 29 IN-09.
- The dangling `runtime()` doc reference: Phase 29 IN-10.
- The unpinned per-project `runtime` round trip: Phase 29 IN-11.
- The `InvalidData` versus `EISDIR` doc claim for a directory at the lock name: Phase 31 IN-01.
- The refused or link-flipped run lock surfacing as "Invalid argument": Phase 29 IN-06.

The Phase 24 findings from the prior pass are not affected by this delta. None of the files that carry them (`roadmap_md.rs`, `roadmap_graph.rs`, `roadmap_view.rs`, `render_escape_guard.rs`, `help.rs`, `state_reader/mod.rs`, the roadmap fixtures, `docs/ARCHITECTURE.md`) changed between `e16a201` and HEAD. In `detail.rs` and `screens/mod.rs`, the `roadmap_nav` and `roadmap_list_viewport` code behind WR-07 and IN-01 is untouched.

### Prior review (e16a201)

This re-reviews Phase 24 as the code stands now, after the month of later quick tasks (workstream bands, lane corners, gsd-core 1.15 parsing parity). The sources read were `roadmap_md.rs`, `roadmap_graph.rs` and `roadmap_view.rs` in full outside their test modules. In `detail.rs` and `state_reader/mod.rs` the roadmap adapter, key handlers, render path and `phase_progress` were read. In `app.rs`, `ui/screens/mod.rs` and `ui/mod.rs` the roadmap-related code paths were read. The docs and fixtures were read too.

Files reviewed in that pass (19): `README.md`, `docs/ARCHITECTURE.md`, `src/app.rs`, `src/state_reader/mod.rs`, `src/state_reader/roadmap_md.rs`, `src/ui/mod.rs`, `src/ui/roadmap_graph.rs`, `src/ui/roadmap_view.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/help.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/render_escape_guard.rs`, `tests/fixtures/roadmaps/README.md`, and the daily-vow, sentriq and ttbook ROADMAP/STATE fixtures.

No security issues were found. Escaping discipline still holds for every new string: phase ids, names, goals, badges, band labels and workstream tags are escaped before they are stored in the model, and raw keys are used only for matching.

**All five warnings from the 2026-09-24 review are still present at HEAD.** The old Info item IN-02 (the stale `v` row in help) is resolved. Four of the old warnings were re-confirmed this time with a throwaway probe crate. The probe lives in the scratchpad and links the repo by path, so the source tree was not touched. Two new warnings were found, both also probe-confirmed or directly provable:

- **GSD phase entries bleed into the build-phase heading that follows them.** This is a correctness bug in the reader.
- **The list pane title advertises a `v list` key that does nothing.**

Probe results (debug build):

| Probe | Result |
|---|---|
| `Phases 0-4294967295` | `declared_phase_count` panics: "attempt to add with overflow" at `roadmap_md.rs:1562` |
| Build phases 3 (range 1-4) and 4 (needs 3) | 3 → `["1","3","4"]` (self-dependency), 4 → `["3"]` |
| `### Phase 13` with no Depends line, then `#### Build phase 14 … Depends on: Build phases 8-13` and two plan lines | Phase 13 reads `depends_on = ["8"]` and plans `0/2` |
| Bands `Über` and `über` | `j` cycles `Band(über), Phase 1, Band(über), Phase 1, …`; phases 2 and 3 are unreachable |

## Critical Issues

None.

## Warnings

No new warnings in the 2026-10-29 pass. WR-01..WR-07 below are carried from the e16a201 pass, unchanged.

### WR-01: Colliding `BandKey`s deadlock `j`/`k` navigation and fold two bands at once (still present, probe-confirmed)

**File:** `src/ui/roadmap_graph.rs:1157-1163` (`band_key`), `:1379-1398` (`row_of` / `step`)
**Issue:** `band_key` is `to_lowercase()` of the raw label, which is full Unicode case folding. `roadmap_milestones` dedupes with `same_milestone`, which uses `eq_ignore_ascii_case` (`roadmap_md.rs:1582-1589`). Two labels that differ only in non-ASCII case (`Über` / `über`) therefore survive as two bands with the SAME key, and the same collision is possible between a synthetic STATE.md band and a milestone. Cursor targets are keys, and `step` finds the current target with `position()`, which returns the first match. The probe showed the cursor cycling `Band, 1, Band, 1, …`, so phases 2 and 3 can never be reached with `j`. `Space` also folds both bands, and `row_of` highlights the first band row only.
**Fix:** Make the key unique per band. Either key by band index and keep the label only for display, or disambiguate duplicate keys inside `layout_list` after computing them:
```rust
let mut seen = HashMap::<BandKey, usize>::new();
for b in &mut bands {
    let n = seen.entry(b.key.clone()).or_insert(0);
    if *n > 0 {
        if let BandKey::Named(k) = &mut b.key { k.push_str(&format!("#{n}")); }
    }
    *n += 1;
}
```
At minimum, make the key and the dedupe use the same fold (`to_ascii_lowercase`).

### WR-02: `declared_phase_count` overflows u32 on third-party input (still present, probe-confirmed; narrower than first reported)

**File:** `src/state_reader/roadmap_md.rs:1559-1566`
**Issue:** `last.major() - first.major() + 1` overflows when `first` is 0 and `last` is `u32::MAX`, as in a `## Milestones` bullet reading `Phases 0-4294967295`. `PhaseNum::parse` accepts any u32, and `roadmap_milestones` records both endpoints. The probe panicked at `:1562` ("attempt to add with overflow"). The function runs from `roadmap_model_at` (`detail.rs:2880`) on every Roadmap render and keypress, so a debug build crashes the TUI from a hostile ROADMAP.md, and a release build wraps to 0. The later sum already uses `saturating_add` (`roadmap_graph.rs:1095`); this site was missed.
**Fix:**
```rust
(Some(first), Some(last)) if last.major() >= first.major() => {
    (last.major() - first.major()).saturating_add(1)
}
```

### WR-03: Build-phase range expansion includes the entry's own id, which creates spurious cycles and can invert a real dependency (still present, probe-confirmed)

**File:** `src/state_reader/roadmap_md.rs:886-936` (`parse_build_depends_on`, range arm at `:917-929`); caller `:1005-1013`; pinned by the test at `:2768-2789`
**Issue:** A range expands to every known id inside it, including the declaring build phase itself and any later phases. The existing test pins phase 6 depending on `["1","2","4","5","6"]`, a self-dependency, and `resolve_deps` then reports `dependency cycle: 6 (edges ignored)` for a roadmap with no cycle. It can also invert a real dependency. With `Build phase 3` declaring `Build phases 1-4` and `Build phase 4` declaring `Build phase 3`, the probe yielded `3 → [1,3,4]` and `4 → [3]`. `break_cycles` then drops the legitimate edge `4 → 3` and keeps the bogus `3 needs 4`, so phase 3 is drawn blocked by 4 and 4 as ready.
**Fix:** Pass the entry's own id into `parse_build_depends_on` and exclude it from range expansion. Also consider bounding the expansion to ids strictly below it:
```rust
.filter(|(n, k)| lo <= *n && *n <= hi && phase_key(k) != own_key)
```
Update the pinned test to expect `["1","2","4","5"]`.

### WR-04: A GSD phase entry does not end at a `#### Build phase` heading, so it inherits that heading's dependencies and plan lines (new, probe-confirmed)

**File:** `src/state_reader/roadmap_md.rs:317-323` (`is_entry_line`), `:462-486` (entry scan in `parse_phases_in_region`)
**Issue:** An entry runs "to the next entry line or the edge of the region". `is_entry_line` does not recognise `build_heading_re` headings (`#### Build phase N …`), and no other heading ends the scan either. ttbook-style roadmaps place build-phase placeholders directly after the last real phase's entry. The probe, with `### Phase 13` carrying no `**Depends on**:` line followed by `#### Build phase 14 … **Depends on**: Build phases 8-13` and two plan lines, gave:
- Phase 13 reads `depends_on = ["8"]`. The "first Depends line wins" rule takes the build phase's line, and the `8-13` endpoints survive self-skip.
- Phase 13 reads plans `0/2`, which are build phase 14's plan lines.

That changes status, waves, the driver's `deps_satisfied` gate and plan counts for a real GSD phase from a section GSD itself does not attribute to it. The goal parser (`parse_phase_goals`) and `parse_planned_build_phases` both already stop at any heading, so the three readers disagree about where an entry ends. The bleed is not limited to build headings: any later non-phase section, such as `## Progress` or `## Backlog`, is also swallowed until the next recognised entry line.
**Fix:** End a Current-region entry at the first heading that is not itself an entry line. At minimum:
```rust
if !in_region(j) || is_entry_line(l, region) || build_heading_re().is_match(l) { break; }
```
Prefer `any_heading_re().is_match(l)` after checking that the `<details>`/`<summary>` wrappers (which are not headings) stay transparent, as the doc comment requires.

### WR-05: The list pane title advertises a `v` key that is unbound

**File:** `src/ui/roadmap_view.rs:691`
**Issue:** `.title_top(Line::from(" v list ").right_aligned())` draws a "v list" toggle hint in the top-right of the Roadmap pane. The graph/box view was removed in 24-06. The test `roadmap_tab_draws_the_list_and_v_is_unbound` (`detail.rs:~26214`) asserts `v` does nothing, and nothing in `src`, `README.md` or `docs` mentions the string. Users are shown a control that does not exist. The old IN-02 fixed the help row for the same reason, but this pane-title remnant was missed.
**Fix:** Drop the right-aligned title, or replace it with something that is bound, such as `" ? help "`:
```rust
let block = pane_block().title_top(Line::from(" Roadmap "));
```

### WR-06: The Roadmap measures widths in chars, so wide-glyph names push the plans and wave columns out of the pane (still present)

**File:** `src/ui/roadmap_view.rs:204-270` (`cells` / `truncate` / `pad_*` / `fit`), `:962-1007` (`phase_spans`)
**Issue:** Every width is `chars().count()`, but CJK characters and most emoji take two terminal cells. A row with a wide-glyph name therefore overflows `cols.name`. `Paragraph` clips to the rect, so the `1/3` plans and `W1` wave columns are silently lost, and `fit` never adds its `…` because it believes the line fits. The Start-now line and the detail-pane lines have the same defect. This was a recorded decision in Phase 21 (`D-21-53`: "`unicode-width` is NOT added", matching `section_rule`), so treat it as known debt, but it is still a visible defect on realistic phase names. `ratatui`'s `Span::width()` / `Line::width()` (already used in `detail.rs:3466`) measure display width.
**Fix:** Measure with `unicode_width::UnicodeWidthStr::width` (already a transitive dependency of ratatui) or `Span::width()`, and truncate by accumulated display width rather than char count.

### WR-07: The Roadmap cursor does not write back to the Phases selection, so the "shared selection" (D-B03) only works one way (still present)

**File:** `src/ui/screens/detail.rs:1595-1633` (`roadmap_nav`); doc claim `src/ui/screens/mod.rs:1035-1036` and `detail.rs:3537-3540`
**Issue:** The Phases tab's keys call `share_pipeline_selection`, so Phases → Roadmap works. `roadmap_nav` stores only `roadmap_cursor`. Moving the Roadmap cursor to phase 12 and pressing `2` (or `→`) opens Phases on the stale `pipeline_selected`. The next `j` on Phases then calls `share_pipeline_selection`, which snaps the Roadmap cursor to that stale phase's key, so the Roadmap selection is silently lost. Only `Enter` (`roadmap_activate`) and the mouse path set `pipeline_selected`. The cache doc says "the two tabs share one selected phase", which overstates the behaviour.
**Fix:** In `roadmap_nav`, when `next` is a `CursorTarget::Phase(key)`, also set `cache.pipeline_selected` to the matching index in `state.scoped_phases()` (as `roadmap_activate` does, `detail.rs:1702-1705`), and reset `waves_cursor` as the Phases `j` arm does. Alternatively, sync once in `switch_to_sub_view` when arriving on `Pipeline`.

## Info

IN-01..IN-05 are carried from the e16a201 pass, unchanged. IN-06 and IN-07 are new in the 2026-10-29 pass.

### IN-01: PageUp/PageDown steps targets, but the page size counts rows that include connectors (still present)

**File:** `src/ui/screens/detail.rs:1605-1612`
**Issue:** `roadmap_list_viewport` is the number of model rows on screen, and fork/merge connector rows are counted. `model.step` moves over targets, which exclude connectors. On a graph with many connectors, one PageDown can move past a full screen and skip phases from view.
**Fix:** Step until the resolved row index has moved by `rows - 1`, or count visible targets inside the window.

### IN-02: ARCHITECTURE.md source tree omits `roadmap_graph.rs`

**File:** `docs/ARCHITECTURE.md:167`
**Issue:** Only `roadmap_view.rs` is listed under `ui/`. The list model, `ui/roadmap_graph.rs` (layout, cursor navigation, lane text), is missing, as are `glyphs.rs` and `mouse.rs`. The stale `roadmap_widget.rs` entry has been removed.
**Fix:** Add `roadmap_graph.rs  Roadmap list model: dependency pipeline, lanes, cursor navigation` beside `roadmap_view.rs`.

### IN-03: `PROBE_PLANNED_PHASE_ID` sits inside `hostile_project_state`'s doc comment (still present)

**File:** `src/ui/screens/render_escape_guard.rs:859-871`
**Issue:** The const and its two `///` lines are inserted between the `///` block of `hostile_project_state` and the function. rustdoc attaches the whole block, including "This is what widens the probe past aliases…", to the const, and the function loses its documentation.
**Fix:** Move the const, with its own doc, above the `hostile_project_state` doc block.

### IN-04: The fixture sanitisation guard is narrower than the parser and the README contract

**File:** `src/state_reader/roadmap_md.rs:3151-3162`; `tests/fixtures/roadmaps/README.md` ("Replaced" section and rule 1)
**Issue:** The guard checks only that a line containing the case-sensitive `**Goal` also contains `(sanitised)`, while `parse_phase_goals` accepts `**goal**:` case-insensitively. The path check covers only `/home/`, so a macOS `/Users/` path would pass. More importantly, the README promises "`(sanitised) ` followed by neutral filler", but the guard enforces only the marker. The ttbook goal bodies, for example `ttbook-ROADMAP.md:83` "One local database holds the migrations, an append-only log and the cached tokens, …", read as coherent project-specific prose, and the ttbook phase titles read the same way. They may be rewritten rather than copied, but the guard cannot tell, and these private-project excerpts ship in the public crate (`tests/` is not excluded in `Cargo.toml`). Provenance could not be verified from this tree.
**Fix:** Confirm the ttbook and sentriq goal and title text is authored filler. Tighten the guard to `to_ascii_lowercase().contains("**goal")`, reject `/Users/` and `C:\Users`, and consider asserting that goal bodies come from a short allow-list of filler phrases.

### IN-05: Regexes are compiled on every call beside `OnceLock` statics

**File:** `src/state_reader/roadmap_md.rs:907` (`parse_build_depends_on`), `:952` (`parse_planned_build_phases`), `:1246-1248` and `:1287` (`roadmap_progress`), `:1339-1341` (`active_milestone`), `:1601-1629` (`roadmap_milestones`, 11 patterns)
**Issue:** These patterns are rebuilt on every ROADMAP reload, which `parse_project_state` triggers for the project and each workstream. The file otherwise uses `OnceLock<Regex>` consistently. This is flagged for consistency only, not performance.
**Fix:** Hoist the patterns into `static OnceLock<Regex>` helpers, as `checklist_re()` and the others do.

### IN-06: `state.json` is read under DIRECTION.md's 1 MiB cap but written with no cap, so a run whose state outgrows it becomes permanently unopenable (new, 2026-10-29)

**File:** `src/driverctl/store.rs:645` (read) against `:742-763` (`write_state`); the cap is `DIRECTION_READ_MAX_BYTES` at `src/state_reader/driver_frontmatter.rs:814` through `read_run_file_capped` (`:882`); other users `src/launcher/stop.rs:200-206`, `src/launcher/run.rs:1216-1223`
**Issue:**
- Quick 261028-t3c / 261028-v2z switched every `state.json` read to `read_run_file_capped`, which enforces the limit chosen for a hand-edited DIRECTION.md (`DIRECTION_READ_MAX_BYTES = 1 << 20`).
- `RunStore::write_state` has no matching bound. It persists the pretty-printed `RunState` as is: the `items` map, a `findings: Vec<i64>` per item, and two flattened `extra` maps that keep any key the launcher adds.
- A `state.json` that crosses 1 MiB therefore saves without complaint, and the next `RunStore::open` returns `StoreError::CorruptState` ("larger than the 1048576-byte limit") for good.
- The orphan path in `stop_orphaned_scope` then returns an I/O error instead of stopping the recorded scope. `check_wall_clock_budget` reads the same file as "no recorded time", so the wall-clock pre-check passes with zero used time (the store's own refusal at open backstops the launch itself, as Phase 29 recorded).
- Reaching 1 MiB takes a pathological run (thousands of items or findings), so this is latent rather than live. The defect is that the write side and the read side do not agree on a bound, and the failure mode is unrecoverable by the tool.

**Fix:** Give `state.json` its own constant instead of borrowing DIRECTION.md's, and make the writer respect it, so that anything the store saves it can read back:
```rust
// in store.rs
if json.len() as u64 > STATE_READ_MAX_BYTES {
    return Err(StoreError::Io { detail: format!("state.json would exceed {STATE_READ_MAX_BYTES} bytes") });
}
```
Pass that constant into the guarded read (a `read_run_file_capped_at(path, max)` variant), and pin both halves with a test that saves a state of exactly the cap and one byte over.

### IN-07: The legacy-key wording is imprecise in two places: "since the Phase 28 launcher", and "written back unchanged" for a typed field (new, 2026-10-29)

**File:** `src/config.rs:63-66` (the `RegisteredProject::extra` doc); `docs/CONFIGURATION.md:84`; contrast `docs/CONFIGURATION.md:84` / `docs/GSD-CORE-SYNC.md:400-405` and `src/config.rs:499-500`
**Issue:**
- Commit `ce86206` rewrote these comments to say what the retained keys do now. `src/config.rs:63-66` says the per-project `runtime` key "selects nothing since the Phase 28 launcher (Claude only)". `docs/CONFIGURATION.md:84` and `docs/GSD-CORE-SYNC.md:400-405` both say it stopped selecting an agent "since Phase 31". The docs are the accurate ones: the legacy `drive` path, which consumed the `AgentRuntime` value, lived until Phase 31 (per the new `executor/mod.rs` header). The in-code comment therefore misdates the behaviour change by three phases.
- `docs/CONFIGURATION.md:84` now says all three legacy keys are "kept and written back unchanged". That is exact for `default_runtime` and the per-project `runtime`, which live in the flattened `extra` maps. It is not exact for `preferences.driver_max_concurrent`, which is a typed `usize` with `#[serde(default = "default_driver_max_concurrent")]` and no `skip_serializing_if` (`config.rs:499-500`). A config that never carried the key gains `"driver_max_concurrent": 1` on its next save, and a value that is not a non-negative integer fails the whole config load rather than being preserved. Phase 30 IN-21 covers the field's stale enforcement doc but not this claim.

**Fix:** In `config.rs`, change "since the Phase 28 launcher" to "since Phase 31 retired the `drive` path" (matching the two docs). In `CONFIGURATION.md`, split the sentence: `default_runtime` and `runtime` are preserved verbatim, while `driver_max_concurrent` is still read and written as a number (default 1) and caps nothing.

## Resolved since 2026-09-24

- **Old IN-02** (help row for `v` still read "graph / box view"): the row is gone from `help.rs` (see WR-05 for the pane-title remnant).
- **`PhaseList`, `layout_graph`, `RoadmapGraphWidget` and `roadmap_widget.rs`:** no live references remain outside `archive::ArchiveDepth::PhaseList`, comments and test strings.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
