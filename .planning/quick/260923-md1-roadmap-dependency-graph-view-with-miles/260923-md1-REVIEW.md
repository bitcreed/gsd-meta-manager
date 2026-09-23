---
phase: 260923-md1-roadmap-dependency-graph-view-with-miles
reviewed: 2026-09-23T00:00:00Z
depth: quick
files_reviewed: 8
files_reviewed_list:
  - src/ui/roadmap_graph.rs
  - src/ui/mod.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
  - src/ui/screens/render_escape_guard.rs
  - src/state_reader/roadmap_md.rs
  - src/state_reader/mod.rs
findings:
  critical: 0
  warning: 3
  info: 7
  total: 10
status: issues_found
---

# Quick 260923-md1: Code Review Report

**Reviewed:** 2026-09-23T00:00:00Z
**Depth:** quick (pattern scan plus targeted reads for the requested focus areas)
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Scope: diff `7cf6910..76c1fd3`. The review covered the focus areas the orchestrator named:

- **Termination:** Layout terminates on cycles and pathological input. `break_cycles` is an iterative DFS that drops back edges, which leaves a DAG. Longest-path layering therefore processes every node. Every non-root node has exactly one primary parent exactly one layer back, so `place_chain` walks strictly increasing layers and each node is placed once. `pending` is therefore finite.
- **Panics:** None found. Every index is bounded by construction (`layer < layers`, `gap = layer[p] < layers-1` whenever `p` has a child, `row_of` set for every node). u16 conversions use `try_from(..).unwrap_or(u16::MAX)` or `saturating_sub`. The widget draws through `Paragraph` into split `Rect`s. `Margin::new(2,0)` on a narrow area saturates to an empty rect, and the widget returns early on it.
- **Escaping:** No unescaped third-party text found. Ids, external dep ids, the current phase name, cycle-note paths and milestone labels are all escaped (`esc()` / `Untrusted::shown()`) before they are stored in the layout. `split_milestone_label` runs on the escaped label.
- **Phase-id matching:** Ids are matched through `phase_key` / `PhaseNum` throughout. No raw string equality was found.

The findings below are about behaviour at the edges: the footer can take the whole graph body, content is unreachable horizontally, milestone assignment is wrong when ranges overlap, and milestone detection is heuristic.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: The notes/detail footer can use all of the graph body's height

**File:** `src/ui/roadmap_graph.rs:952-962` (and `169`, `208-211`)
**Issue:** `split_areas` gives the footer `Constraint::Length(notes + detail)` and the body only `Min(0)`. `break_cycles` emits one note per back edge, and `resolve_deps` emits one per self-cycle, with no cap. A malformed roadmap can produce O(n²) cycle notes, for example phases that depend on each other densely. Even a handful of notes plus the detail line takes the whole body on a short terminal. When that happens the graph is not drawn at all, and `detail.rs:3887-3890` records `visible_height: 0`. The same happens when the terminal is merely short (height ≤ header + notes + detail). The bounded, low-value part of the view hides the primary content.
**Fix:** Cap the footer, and give the body priority:
```rust
let footer_h = u16::try_from(footer_lines).unwrap_or(u16::MAX)
    .min(area.height.saturating_sub(header_h) / 3); // or a fixed cap like 3
```
Also collapse cycle notes into one line, e.g. `dependency cycles: N (edges ignored): a, b, c; …`, truncated with a `+K more` suffix.

### WR-02: A graph wider than the pane has no horizontal scroll, so nodes and row-end milestone labels cannot be reached

**File:** `src/ui/roadmap_graph.rs:1077-1083`, `src/ui/screens/detail.rs:3883-3900`
**Issue:** The only horizontal offset is derived from the current node's span. When there is no `Current` marker (all phases done, or no active phase), `h = 0`. When the current node is on the left of a wide graph, `h` is also small. In both cases every column past `graph.width` is clipped and no key reaches it. The row-end milestone labels sit at `widest + 2`, so they are the first thing lost on a normal-width terminal. The key handlers only scroll vertically (`generic_viewport`).
**Fix:** Add a horizontal offset to `RoadmapGraphWidget`, for example `h_offset: u16` stored in `ProjectViewCache`. Bind it to `h`/`l` or Left/Right on the Roadmap tab, clamp it to `max_row_width - graph.width`, and use the current-node offset only as the initial value. Alternatively, when the graph is wider than the pane, move the milestone labels into a separate column that is not scrolled.

### WR-03: `milestone_index_of` prefers the decimal-major rule over an exact range match

**File:** `src/state_reader/roadmap_md.rs:95-101, 112-119`
**Issue:** `range_contains` accepts `p` whenever `first <= p && (p <= last || p.major() == last.major())`, and `milestone_index_of` returns the first milestone whose range matches. Take milestone A `Phases 1-7` and milestone B `Phases 7.1-12`, which happens when an urgent decimal phase opens the next milestone. Phase `7.1` (and `7.2`, …) is assigned to A even though B's declared range covers it exactly. The row-end label and header membership are then wrong.
**Fix:** Search twice: strict inclusive range first, then the decimal-major extension:
```rust
ms.iter().position(|m| m.strict_range_contains(phase_id))
    .or_else(|| ms.iter().position(|m| m.range_contains(phase_id)))
    .or_else(|| /* heading scope */)
```

## Info

### IN-01: Milestone detection accepts entries that are not milestones

**File:** `src/state_reader/roadmap_md.rs:274-325`
**Issue:** The scan has four false-positive sources:
- Every `<summary>` line becomes a milestone regardless of content, e.g. `<summary>Phase 3 details</summary>`.
- Every bold span in any `-`/`*` bullet under `## Milestones` becomes a milestone, including indented sub-bullets such as `  - **Goal:** …`, because of `trim_start`.
- Any level 2-4 heading containing `\bv\d+` becomes a milestone and opens a phase scope, e.g. `### Upgrade to v2 API`.
- `any_heading` does not skip fenced code blocks, so a `# comment` inside a fenced block closes an open scope.

These entries appear in the header only if they gain members or match the in-progress fallback, so the impact is limited.
**Fix:** Require `<summary>` text to contain a version or `Milestone N` token. Accept only unindented bullets in `## Milestones`. Track a fenced-code-block flag and ignore lines inside fences.

### IN-02: `active_milestone_index` does not match `Milestone N` labels by short id

**File:** `src/state_reader/roadmap_md.rs:123-139`
**Issue:** The first-token fallback compares STATE's `milestone:` value with the first whitespace token of the label. For `Milestone 1: Foo` that token is `Milestone`, so a STATE value of `M1` or `Milestone 1` never matches. Activity then falls back to the in-progress marker, or to none.
**Fix:** Also compare `split_milestone_label(label).0` with `split_milestone_label(wanted).0`, reusing `same_milestone`.

### IN-03: Column arithmetic counts chars, but `Paragraph` scrolls by display width

**File:** `src/ui/roadmap_graph.rs:118-120, 1077-1080`
**Issue:** `width_of` uses `chars().count()`. When a label or milestone name contains wide characters (CJK, emoji that survive escaping), the current-node horizontal offset and the row-end label padding (`904`) are misaligned. This is the documented IN-02/IN-03 deferral, noted here because it now also affects the scroll target.
**Fix:** Use `unicode-width` (already a ratatui dependency) for `width_of` when that deferral is lifted.

### IN-04: Prefixed phase ids (`M-1`) never get range membership

**File:** `src/state_reader/roadmap_md.rs:229`
**Issue:** `endpoint` uses `PhaseNum::parse`, which rejects a `PHASE_ID` with a project-code prefix. Projects with prefixed ids therefore have `first`/`last` set to `None` and rely on heading scope only.
**Fix:** Strip the alphabetic prefix, the same way `is_sentinel_phase` does, before `PhaseNum::parse`. Alternatively, document that ranges are numeric-only.

### IN-05: Layout is recomputed on every frame, and branch placement is superlinear

**File:** `src/ui/screens/detail.rs:3883-3884`, `src/ui/roadmap_graph.rs:450-471`
**Issue:** `layout_for_state` runs in `render`, and `branch_row` × `run_claimable` is O(rows²·layers) per branch. Roadmaps of normal size are unaffected. A pathological roadmap with thousands of phases could stall the render loop. Performance is out of v1 scope; this is noted only against the "pathological input" focus.
**Fix:** Cache the `GraphLayout` in `ProjectViewCache` and invalidate it when `ProjectState` changes.

### IN-06: `roadmap_milestones` compiles about 12 regexes on every call

**File:** `src/state_reader/roadmap_md.rs:205-227`
**Issue:** This is inconsistent with the `OnceLock` pattern used elsewhere in the same file (`split_milestone_label`, `extract_phase_id`). It runs once per state parse, so the cost is small.
**Fix:** Hoist the regexes into `OnceLock` statics.

### IN-07: The box-view scroll estimate keeps an unchecked `as u16` cast and multiplication

**File:** `src/ui/screens/detail.rs:3862`
**Issue:** `3 + (state.phases.len() as u16 - 1) * phase_block_h` truncates past 65535 phases and overflows (panicking in debug builds) past about 13107 phases. The code is pre-existing but was moved in this diff.
**Fix:** `u16::try_from(len - 1).unwrap_or(u16::MAX).saturating_mul(phase_block_h).saturating_add(3)`.

---

_Reviewed: 2026-09-23T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
