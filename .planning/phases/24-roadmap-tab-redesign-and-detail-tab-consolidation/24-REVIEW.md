---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
reviewed: 2026-09-24T04:57:05Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - docs/ARCHITECTURE.md
  - README.md
  - src/app.rs
  - src/archive.rs
  - src/state_reader/mod.rs
  - src/state_reader/roadmap_md.rs
  - src/ui/mod.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/fixtures/roadmaps/daily-vow-ROADMAP.md
  - tests/fixtures/roadmaps/daily-vow-STATE.md
  - tests/fixtures/roadmaps/README.md
  - tests/fixtures/roadmaps/sentriq-ROADMAP.md
  - tests/fixtures/roadmaps/sentriq-STATE.md
  - tests/fixtures/roadmaps/ttbook-ROADMAP.md
  - tests/fixtures/roadmaps/ttbook-STATE.md
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-09-24T04:57:05Z
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This review covered the phase-24 diff (`53b2159^..HEAD`) and the code around it. That includes the new ROADMAP reader facts (build phases, goals, shipped milestones, declared counts) in `roadmap_md.rs` and `state_reader/mod.rs`, the rewritten list model in `roadmap_graph.rs`, the new `roadmap_view.rs` widget, the Roadmap adapter, keys and render in `detail.rs`, the alias-keyed `ArchiveCache` and in-place archive refresh, the tab renumbering and Docs sub-tabs, help and docs, and the vendored fixtures.

The escaping discipline holds. Every third-party string in the new model is escaped before storage, and the probe suite covers the new text paths. The archive-cache re-key is correct.

The defects are in edge-case logic, and three of them were confirmed with throwaway probe tests. Those tests ran against a scratch copy of HEAD, which was deleted afterwards. The source tree was not touched.

1. **Duplicate band keys lock the cursor.** Two bands whose keys collide make part of the list unreachable with `j`/`k`.
2. **Declared phase count can overflow.** A u32 overflow in `declared_phase_count` panics the render in debug builds.
3. **Build-phase ranges can include their own phase.** The range expansion can add the phase itself, which turns legitimate edges into cycles and can invert a dependency.
4. **Wide characters push columns off screen.** Width is measured in chars, so CJK or emoji names push the plans and wave columns out of the list pane.

`cargo test --lib`: 1550 passed and 1 failed. The failure is the known machine-local git-version witness in `envelope/policy.rs`, which is unrelated to this phase.

## Warnings

### WR-01: Colliding `BandKey`s deadlock `j`/`k` navigation and fold two bands at once

**File:** `src/ui/roadmap_graph.rs:908-910` (key), `:1064-1083` (`row_of` / `step`)
**Issue:** `band_key` is `to_lowercase()` of the raw label. `roadmap_milestones` dedupes labels with `eq_ignore_ascii_case` (`roadmap_md.rs:918-925`), so two labels that differ only in non-ASCII case (`Über` / `über`) survive as two bands with the SAME key. The synthetic STATE.md band can collide the same way. Cursor targets are keys, and `step` finds the current target with `position()`, which returns the first match. The result is that the cursor can never pass the second band row.

A probe on the current code with bands `Über`[1] and `über`[2, 3] showed this. The targets were `[Band(über), Phase 1, Band(über), Phase 2, Phase 3]`, and eight `j` presses cycled `Band, 1, Band, 1, …`. Phases 2 and 3 could not be reached with `j`. `row_of` also highlights the first band row when the cursor is on the second, and `Space` folds both bands.
**Fix:** Make the key unique per band. Either key by band index and keep the label only for display, or disambiguate duplicates when building `BandFacts`. For example:
```rust
// in layout_list, after computing keys:
let mut seen = HashMap::<BandKey, usize>::new();
for b in &mut bands {
    let n = seen.entry(b.key.clone()).or_insert(0);
    if *n > 0 { if let BandKey::Named(k) = &mut b.key { k.push_str(&format!("#{n}")); } }
    *n += 1;
}
```
At minimum, use the same case-folding the dedupe uses (`to_ascii_lowercase`) so that dedupe and key agree.

### WR-02: `declared_phase_count` overflows u32 on third-party input (panics the render in debug builds)

**File:** `src/state_reader/roadmap_md.rs:895-902`
**Issue:** `last.major() - first.major() + 1` overflows when a ROADMAP line reads `Phases 0-4294967295`. `PhaseNum::parse` accepts any u32. A probe confirmed the panic at `roadmap_md.rs:898` ("attempt to add with overflow"). The function runs from `roadmap_model_for` on every Roadmap render and keypress, so a debug build (`cargo run`) crashes the TUI. A release build wraps to 0 instead, and the milestone then reports a wrong declared count. The later sum already uses `saturating_add` (`roadmap_graph.rs:857`); this site was missed.
**Fix:**
```rust
(Some(first), Some(last)) if last.major() >= first.major() => {
    (last.major() - first.major()).saturating_add(1)
}
```

### WR-03: Build-phase range expansion includes the entry's own id, which creates spurious cycles and can invert a real dependency

**File:** `src/state_reader/roadmap_md.rs:392-404` (`parse_build_depends_on`, the range arm); `parse_planned_build_phases` at `:~470`
**Issue:** A range expands to every known id inside it, including the build phase that declares it and any later phases. The existing test `a_build_dependency_range_expands_only_to_known_ids` pins phase 6 depending on `["1","2","4","5","6"]`, a self-dependency. The model then shows `dependency cycle: 6 (edges ignored)` for a roadmap with no cycle.

It can also be worse than a note. With `Build phase 3` declaring `Build phases 1-4` and `Build phase 4` declaring `Build phase 3`, the reader yields `3 → [3, 4]` and `4 → [3]` (confirmed by probe). `break_cycles` then drops the edge `(4, 3)`, which is the legitimate one, and keeps the bogus `3 needs 4`. Phase 3 is then drawn as blocked by 4, and 4 as ready.
**Fix:** Pass the entry's own id into `parse_build_depends_on` and exclude it from range expansion. Also consider bounding the expansion to ids strictly below the entry's own:
```rust
.filter(|(n, k)| lo <= *n && *n <= hi && phase_key(k) != own_key)
```
Update the pinned test to expect `["1","2","4","5"]` or `["1","2","4"]`.

### WR-04: The Roadmap list measures width in chars, so wide-glyph names push the plans and wave columns out of the pane

**File:** `src/ui/roadmap_view.rs:172-204` (`cells` / `truncate` / `pad_*`), `:208-233` (`fit`), `:782-791` (`phase_spans`)
**Issue:** Every width is `chars().count()`, but CJK characters and most emoji take two terminal cells. A probe rendered a CJK phase name at 120 columns. The name column filled the row and the `1/3` plans and `W1` wave columns were not drawn at all. `fit` did not add its `…` because it believes the line fits. The Start-now line and the detail-pane `fit()` lines have the same defect.

A single emoji in a long name is enough to clip the wave column. Nothing bleeds into the detail pane because `Paragraph` clips to its rect, but the columns are wrong for realistic non-Latin or emoji phase names. The rest of the crate already measures display width through ratatui (`Span::width()` / `Line::width()` in `normal.rs:1075`, `detail.rs:4094`).
**Fix:** Measure with `unicode_width::UnicodeWidthStr::width` (already a transitive dependency through ratatui), or with `Span::width()`. Truncate by accumulated display width, not by char count, so a two-cell glyph is never split at the boundary.

### WR-05: The Roadmap cursor does not write back to the Phases selection, so D-B03's shared selection only works one way

**File:** `src/ui/screens/detail.rs:827-862` (`roadmap_nav`); doc claim at `src/ui/screens/mod.rs:940-945`
**Issue:** D-B03 says the two tabs share the selected phase. Phases → Roadmap works: `share_pipeline_selection` runs on `j`/`k`/`PgUp`/`PgDn`. Roadmap → Phases happens only on `Enter`. If the user moves the Roadmap cursor to phase 12 and presses `2` (or `→`), Phases opens on the stale `pipeline_selected`, not phase 12.
**Fix:** In `roadmap_nav`, when `next` is a `CursorTarget::Phase(key)` that indexes into `state.phases`, also set `cache.pipeline_selected` to that index, as `roadmap_activate` already does. Alternatively, sync it in `switch_to_sub_view` when arriving on `Pipeline` from a Roadmap cursor. If the one-way behaviour is intended, say so in the `roadmap_cursor` doc and in D-B03.

## Info

### IN-01: PageUp/PageDown pages over targets, but the page size counts rows that include connectors

**File:** `src/ui/screens/detail.rs:837-841`
**Issue:** `roadmap_list_viewport` is the number of model rows visible, and fork/merge connector rows are counted in it. `model.step` moves over targets, which exclude connectors. On a graph with many connectors, one PageDown can move past a full screen, and phases are skipped from view.
**Fix:** Page by the number of visible targets inside the window, or step until the resolved row moves by `rows - 1`.

### IN-02: The help row for `v` still says "graph / box view"

**File:** `src/ui/screens/help.rs:238`
**Issue:** The left-to-right graph was removed in 24-06. The default view is now the list.
**Fix:** Change the row to `row("v", "Roadmap tab: list / box view (detail view)")`.

### IN-03: The ARCHITECTURE.md source tree omits the two new Roadmap modules

**File:** `docs/ARCHITECTURE.md:159-161`
**Issue:** Only `roadmap_widget.rs` is listed, and it is now the secondary box view. `ui/roadmap_graph.rs` (the list model) and `ui/roadmap_view.rs` (the default widget) are missing.
**Fix:** Add both entries under `ui/`.

### IN-04: `PROBE_PLANNED_PHASE_ID` was inserted inside `hostile_project_state`'s doc comment

**File:** `src/ui/screens/render_escape_guard.rs:852-864`
**Issue:** The const sits between the tail of `hostile_project_state`'s `///` block and the function. rustdoc therefore attaches the whole doc block ("Putting the identity in all of them at once …") to the const, and the function loses its documentation.
**Fix:** Move the const, with its own doc line, above the `hostile_project_state` doc block.

### IN-05: The fixture sanitisation guard is narrower than the parser it protects

**File:** `src/state_reader/roadmap_md.rs:2144-2170`
**Issue:** The guard checks `line.contains("**Goal")`, which is case-sensitive, but `parse_phase_goals` accepts `**goal**:` case-insensitively. A future fixture with a lower-case goal line would be parsed as a goal yet escape the "(sanitised)" check. The path check only looks for `/home/`, so a fixture taken on macOS (`/Users/`) would pass.
**Fix:** Match `to_ascii_lowercase().contains("**goal")`, and also reject `/Users/` and `C:\\Users`.

### IN-06: `parse_build_depends_on` compiles a regex on every call despite the `OnceLock` statics beside it

**File:** `src/state_reader/roadmap_md.rs:~382` (`Regex::new(r"\([^)]*\)").unwrap()`)
**Issue:** This is inconsistent with the three `OnceLock` regexes directly above it. The same pattern appears in `parse_planned_build_phases` (`plan_re`, `depends_re`). These run on every ROADMAP reload. This is not flagged for performance, only for consistency.
**Fix:** Hoist the pattern into a `static OnceLock<Regex>` like its neighbours.

---

_Reviewed: 2026-09-24T04:57:05Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
