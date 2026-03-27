# Phase 07: Execution Flow & GSD Integration - Research

**Researched:** 2026-03-26
**Domain:** TUI pipeline visualization, disk-inferred state badges, config extension
**Confidence:** HIGH

## Summary

This phase adds a Pipeline tab to the detail view and [verified]/[inferred] badges to status fields. The implementation is straightforward because it builds entirely on existing infrastructure: the `DiskInference` struct already tracks most stage artifacts, the tab system from Phase 06 supports adding a 5th tab, and the `compact_pipeline()` function in `normal.rs` already renders D-R-P-E-V with color logic.

The primary work involves: (1) extending `DiskInference` with a `has_plans` boolean (it already has `has_context`, `has_research`, `has_verification` but lacks explicit plan tracking as a boolean), (2) adding a `Pipeline` variant to `DetailSubView` with tab key `5`, (3) rendering horizontal box-drawing pipeline `[D]-[R]-[P]-[E]-[V]` with per-stage coloring, and (4) adding `[verified]`/`[inferred]` badge spans to status display lines.

**Primary recommendation:** Reuse the existing `compact_pipeline()` color logic as the foundation for the detailed pipeline renderer, extending it with box-drawing characters and the plan fraction display for the Execute stage.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- D-01: Pipeline visualization lives as a new tab ("Pipeline") in the detail view, reusing the tab system from Phase 06
- D-02: Horizontal box format: `[D]-[R]-[P]-[E]-[V]` with color per stage
- D-03: Plan progress shown inside Execute box: `[E 2/3]` (completed/total plans)
- D-04: Reuse and extend `phase_disk_statuses` / `DiskInference` from Phase 05 rather than creating a new data model
- D-05: Badges appear next to status fields in the detail view: `Executing [inferred]` or `Complete [verified]`
- D-06: Verified = has SUMMARY.md and/or VERIFICATION.md artifacts; Inferred = disk heuristic only (no authoritative GSD artifacts)
- D-07: Badge styling: dim text -- `[verified]` in green, `[inferred]` in dark gray
- D-08: GSD integration opt-in via config toggle in `~/.config/gsd-manager/config.toml`: `gsd_integration = true` (default false)
- D-09: Extend `DiskInference` with per-stage booleans: `has_context`, `has_research`, `has_plans`, `has_summaries`, `has_verification`
- D-10: Skipped stages shown as dimmed `[--]` -- a stage is "skipped" if a later stage is complete but this one has no artifacts
- D-11: Stage colors: Green = complete, Yellow = current/active, DarkGray = not started, Magenta = skipped
- D-12: Per-phase pipeline in the Pipeline tab -- user selects a phase to see its detailed stage breakdown

### Claude's Discretion
- Pipeline tab layout proportions (phase list vs pipeline detail split)
- Exact box-drawing characters and spacing
- Whether to show pipeline summary in the Phases tab as well
- Config file creation on first toggle

### Deferred Ideas (OUT OF SCOPE)
- Cached gsd-tools.cjs JSON output for richer state (GSD-01 advanced) -- beyond file existence checks
- Interactive pipeline stage drilling (click a stage to see artifacts) -- future enhancement
- Pipeline animation for active stages -- cosmetic, defer
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FLOW-01 | User sees per-phase pipeline visualization (discuss/research/plan/execute/verify) | Pipeline tab with `[D]-[R]-[P]-[E]-[V]` rendering; extend DetailSubView enum; reuse compact_pipeline color logic |
| FLOW-02 | User sees color-coded status per stage (not started, current, complete, skipped) | D-11 color scheme applied via stage booleans on DiskInference; skipped detection via "later stage complete but this one missing" |
| FLOW-03 | User sees plan execution progress as fraction in the execute stage | `[E 2/3]` format using existing `summary_count`/`plan_count` from DiskInference |
| GSD-01 | User can opt into enriching state with cached gsd-tools.cjs JSON output | Config toggle `gsd_integration = true` in config.toml; this phase implements the toggle and basic file-based enrichment only (advanced caching deferred) |
| GSD-02 | User sees [verified] vs [inferred] badges on status fields | Badge spans appended to status lines in phase list; verified = has SUMMARY.md/VERIFICATION.md; inferred = heuristic only |
</phase_requirements>

## Standard Stack

No new dependencies needed. This phase uses only existing crate features.

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI rendering (Tabs, Span, Line, Layout, Block, List) | Already in use; all widgets needed exist |
| crossterm | 0.29.0 | Terminal backend | Already in use |
| serde | 1.0.228 | Config deserialization | Already in use |
| toml | 1.1.0 | TOML config parsing | Already in Cargo.toml for user config |

### No New Dependencies
This phase is entirely UI + data model extension. No new crates required.

## Architecture Patterns

### Recommended Project Structure Changes
```
src/
  state_reader/
    disk_status.rs      # Extend DiskInference with has_plans, rename summary_count->has_summaries
  config.rs             # Add gsd_integration field to Preferences
  app.rs                # Add Pipeline variant to DetailSubView
  ui/screens/
    detail.rs           # Add Pipeline tab (tab 5), badge rendering, phase selection
```

### Pattern 1: Extending DiskInference (Data Model)

**What:** Add missing per-stage booleans to DiskInference so the pipeline can query each stage independently.

**Current state of DiskInference:**
```rust
pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_context: bool,
    pub has_research: bool,
    pub has_verification: bool,
}
```

**Required additions (D-09):**
```rust
pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_context: bool,      // already exists
    pub has_research: bool,     // already exists
    pub has_plans: bool,        // NEW: true if plan_count > 0
    pub has_summaries: bool,    // NEW: true if summary_count > 0
    pub has_verification: bool, // already exists
}
```

**Key insight:** `has_plans` and `has_summaries` are derived from counts that already exist. The booleans are convenience fields for the pipeline renderer. Set them in `infer_disk_status()` alongside the count assignments.

### Pattern 2: Pipeline Tab with Phase Selection (Split-Pane)

**What:** A new tab showing a selectable phase list on the left and a detailed pipeline visualization for the selected phase on the right.

**When to use:** Follows the exact pattern from Git tab (Phase 06) -- ListState for selection, Layout::horizontal for split.

**Layout recommendation:**
```
+--[Pipeline]-------------------------------------------+
| Phase List (40%)       | Pipeline Detail (60%)        |
| > P05: State Reader    | [D]--[R]--[P]--[E 2/3]--[V] |
|   P06: Detail Tabs     |                              |
|   P07: Exec Flow       | Stage Details:               |
|   P08: Queue Exec      |   Discuss:  Complete         |
|   P09: Sessions        |   Research: Complete          |
|                        |   Plan:     3 plans           |
|                        |   Execute:  2/3 summaries     |
|                        |   Verify:   Not started       |
+--------------------------------------------------------+
```

**Selection state:** Add `pipeline_selected: usize` to `ProjectViewCache`. Reuse `j/k` navigation pattern from backlog/git tabs.

### Pattern 3: Stage Status Derivation

**What:** Determine per-stage color from DiskInference booleans.

**Algorithm:**
```rust
enum StageStatus { Complete, Current, Skipped, NotStarted }

fn derive_stage_status(inference: &DiskInference, stage_index: usize) -> StageStatus {
    let stages = [
        inference.has_context,       // D
        inference.has_research,      // R
        inference.has_plans,         // P
        inference.summary_count > 0, // E (partial execution)
        inference.has_verification,  // V
    ];

    let this_complete = stages[stage_index];
    let any_later_complete = stages[stage_index + 1..].iter().any(|&s| s);

    if this_complete {
        StageStatus::Complete   // Green
    } else if any_later_complete {
        StageStatus::Skipped    // Magenta
    } else {
        // Check if this is the "current" stage (first incomplete after last complete)
        let last_complete_idx = stages[..stage_index].iter().rposition(|&s| s);
        let is_next_after_complete = match last_complete_idx {
            Some(idx) => stage_index == idx + 1,
            None => stage_index == 0,
        };
        if is_next_after_complete && inference.status != DiskStatus::NoDirectory && inference.status != DiskStatus::Empty {
            StageStatus::Current    // Yellow
        } else {
            StageStatus::NotStarted // DarkGray
        }
    }
}
```

**Edge case -- Execute stage:** The Execute stage is "complete" when `summary_count >= plan_count && plan_count > 0`. It is "partial/current" when `summary_count > 0 && summary_count < plan_count`. This maps to the existing `DiskStatus::Partial` vs `DiskStatus::Complete` logic.

### Pattern 4: Badge Rendering

**What:** Append `[verified]` or `[inferred]` as styled Span to status lines.

**Verification rule (D-06):**
```rust
fn is_verified(inference: &DiskInference) -> bool {
    inference.has_summaries || inference.has_verification
}
```

**Rendering:**
```rust
fn badge_span(inference: &DiskInference) -> Span<'static> {
    if is_verified(inference) {
        Span::styled(" [verified]", Style::default().fg(Color::Green).add_modifier(Modifier::DIM))
    } else {
        Span::styled(" [inferred]", Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM))
    }
}
```

**Where badges appear:** In `disk_suffix()` function and in the Pipeline tab's stage detail view. The existing `disk_suffix` function in `detail.rs` (line 65) already computes status labels -- badges get appended here.

### Pattern 5: Config Toggle (D-08)

**What:** Add `gsd_integration` boolean to Preferences in config.

**Current config format:** `~/.config/gsd-manager/config.json` (JSON, not TOML as D-08 suggests).

**Important discrepancy:** D-08 says "config toggle in `~/.config/gsd-manager/config.toml`" but the actual config is `config.json`. The implementation should add the field to the existing JSON config, not create a second TOML file. The user intent is clear: a toggle in the app's config. Use the existing `Preferences` struct.

```rust
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Preferences {
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub gsd_integration: bool,  // NEW: opt-in for enriched state
}
```

**Behavior when false (default):** Pipeline tab still works (uses disk inference). Badges do not appear. When true: badges appear on status fields.

### Anti-Patterns to Avoid
- **Creating a separate PipelineState struct:** Reuse DiskInference -- it already has all the data. Adding a parallel struct creates sync bugs.
- **Blocking I/O for pipeline data:** Phase disk statuses are already computed in `parse_project_state()` and cached. Pipeline tab should read from `ProjectState.phase_disk_statuses`, not re-scan the filesystem.
- **Hardcoding stage count:** Use a const array `STAGES` to define the 5 stages, so if GSD adds stages later the code is easy to extend.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Box-drawing pipeline | Manual string concatenation | Compose with ratatui `Span` + `Line` | Proper color per-character, no ANSI escape codes |
| Phase selection list | Custom scroll logic | ratatui `List` + `ListState` | Already proven in git/backlog tabs |
| Split-pane layout | Manual Rect math | `Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])` | Established pattern in git tab |

## Common Pitfalls

### Pitfall 1: Tab Index Off-By-One After Adding Pipeline
**What goes wrong:** Adding `Pipeline` as tab 5 (index 4) without updating `sub_view_from_index` match default arm causes fallback to PhaseList.
**Why it happens:** The current code has `_ => DetailSubView::PhaseList` as catchall.
**How to avoid:** Update both `tab_index()` and `sub_view_from_index()` functions, add `'5'` key handler in `handle_key`.
**Warning signs:** Pressing `5` does nothing or switches to wrong tab.

### Pitfall 2: Execute Stage "Complete" vs "Partial" Logic
**What goes wrong:** Execute stage shows green (complete) when only some summaries exist.
**Why it happens:** Using `has_summaries` boolean (true if any summary exists) instead of checking `summary_count >= plan_count`.
**How to avoid:** Execute stage completeness must check the count comparison, not just the boolean.
**Warning signs:** Phase with 1/3 summaries shows Execute as green.

### Pitfall 3: Skipped Stage Detection False Positives
**What goes wrong:** A phase with only VERIFICATION.md (no other artifacts) shows D, R, P, E all as "skipped" (Magenta).
**Why it happens:** Verification exists but nothing before it, so all earlier stages appear skipped.
**How to avoid:** This edge case is actually correct behavior per D-10 ("a stage is skipped if a later stage is complete but this one has no artifacts"). However, it may look odd. Consider whether a phase with only VERIFICATION.md is realistic.
**Warning signs:** Unrealistic phase states showing all-magenta pipeline.

### Pitfall 4: Config.json vs Config.toml Mismatch
**What goes wrong:** Creating a separate TOML config file when JSON config already exists.
**Why it happens:** D-08 mentions `config.toml` but actual config is `config.json`.
**How to avoid:** Add the toggle to existing `Preferences` in `config.rs`. Single config file.
**Warning signs:** Two config files, user confusion.

### Pitfall 5: Badge Visibility When gsd_integration is Off
**What goes wrong:** Badges show even when `gsd_integration = false`.
**Why it happens:** Forgetting to check the config toggle before rendering badges.
**How to avoid:** Check `ctx.config.preferences.gsd_integration` before appending badge spans.
**Warning signs:** Badges appear on fresh install without user opt-in.

## Code Examples

### Pipeline Box Rendering
```rust
// Render a single stage box with color
fn stage_box(label: &str, status: StageStatus, extra: Option<&str>) -> Vec<Span<'static>> {
    let color = match status {
        StageStatus::Complete => Color::Green,
        StageStatus::Current => Color::Yellow,
        StageStatus::Skipped => Color::Magenta,
        StageStatus::NotStarted => Color::DarkGray,
    };

    let content = match extra {
        Some(e) => format!("[{} {}]", label, e),
        None => format!("[{}]", label),
    };

    vec![Span::styled(content, Style::default().fg(color))]
}

// Full pipeline for a phase
fn render_pipeline_line(inference: &DiskInference) -> Line<'static> {
    let stages = ["D", "R", "P", "E", "V"];
    let mut spans: Vec<Span> = Vec::new();

    for (i, label) in stages.iter().enumerate() {
        let status = derive_stage_status(inference, i);
        let extra = if *label == "E" && inference.plan_count > 0 {
            Some(format!("{}/{}", inference.summary_count, inference.plan_count))
        } else {
            None
        };

        if i > 0 {
            spans.push(Span::styled("---", Style::default().fg(Color::DarkGray)));
        }
        spans.extend(stage_box(label, status, extra.as_deref()));
    }

    Line::from(spans)
}
```

### Tab Extension Pattern
```rust
// In app.rs
pub enum DetailSubView {
    #[default]
    PhaseList,
    RoadmapViz,
    Backlog,
    GitHistory,
    Pipeline,  // NEW
}

// In detail.rs
const TAB_TITLES: [&str; 5] = ["1:Phases", "2:Roadmap", "3:Backlog", "4:Git", "5:Pipeline"];

fn tab_index(sub_view: &DetailSubView) -> usize {
    match sub_view {
        DetailSubView::PhaseList => 0,
        DetailSubView::RoadmapViz => 1,
        DetailSubView::Backlog => 2,
        DetailSubView::GitHistory => 3,
        DetailSubView::Pipeline => 4,
    }
}

fn sub_view_from_index(index: usize) -> DetailSubView {
    match index {
        0 => DetailSubView::PhaseList,
        1 => DetailSubView::RoadmapViz,
        2 => DetailSubView::Backlog,
        3 => DetailSubView::GitHistory,
        4 => DetailSubView::Pipeline,
        _ => DetailSubView::PhaseList,
    }
}
```

### Badge in Phase List
```rust
// Extend disk_suffix to include badge when gsd_integration is enabled
fn disk_suffix_with_badge(
    phase_number: &str,
    phase_disk_statuses: &HashMap<String, DiskInference>,
    show_badges: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    if let Some(inf) = phase_disk_statuses.get(phase_number) {
        let label = match inf.status {
            DiskStatus::Partial => {
                if inf.plan_count > 0 {
                    format!(" [Executing {}/{}]", inf.summary_count, inf.plan_count)
                } else {
                    " [Executing]".to_string()
                }
            }
            DiskStatus::Planned => {
                if inf.plan_count > 0 {
                    format!(" [Planned ({} plans)]", inf.plan_count)
                } else {
                    " [Planned]".to_string()
                }
            }
            other => format!(" [{:?}]", other),
        };
        spans.push(Span::raw(label));

        if show_badges {
            if inf.has_summaries || inf.has_verification {
                spans.push(Span::styled(
                    " [verified]",
                    Style::default().fg(Color::Green).add_modifier(Modifier::DIM),
                ));
            } else {
                spans.push(Span::styled(
                    " [inferred]",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                ));
            }
        }
    }

    spans
}
```

## Project Constraints (from CLAUDE.md)

- **Architecture:** TEA pattern -- single App struct, Action enum, mpsc EventBus, stateless components
- **State reading:** Parse `.planning/` files directly; StateReader is the only module that knows the schema
- **Config format:** JSON at `~/.config/gsd-manager/config.json` (not TOML despite D-08 mention)
- **Rendering:** Never block the render loop; all I/O through tokio channels
- **Error handling:** anyhow for internal, color-eyre for top-level panic handler
- **Widget pattern:** Custom Widget trait impl with direct Buffer writes (Phase 04 pattern)
- **Tab pattern:** DetailSubView enum + number-key switching + Tabs widget header (Phase 06 pattern)
- **Split-pane pattern:** outer Block, inner area, Layout::vertical/horizontal split (Phase 06 pattern)
- **No std::sync::Mutex in async code** -- use tokio::sync::Mutex if needed

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Expanded status labels | Compact D-R-P-E-V pipeline | Phase 05 | Normal screen already renders pipeline; detail view catches up |
| Single detail sub-view | 4-tab system | Phase 06 | Adding tab 5 is a well-established pattern now |

## Open Questions

1. **Config.json vs config.toml for gsd_integration toggle**
   - What we know: D-08 says config.toml, but actual config is config.json
   - What's unclear: Whether user specifically wanted a separate TOML file
   - Recommendation: Add to existing JSON config (Preferences struct). Single config is simpler and the codebase already handles it. If user wanted TOML specifically, they can flag it during planning review.

2. **GSD-01 scope in this phase**
   - What we know: GSD-01 says "enriching state with cached gsd-tools.cjs JSON output" but the deferred section says this is out of scope for this phase
   - What's unclear: What minimal GSD-01 work belongs here vs deferred
   - Recommendation: Implement the config toggle (opt-in mechanism) and file-existence-based verification logic. The toggle enables future enrichment. Mark GSD-01 as partially addressed.

3. **Pipeline tab data loading**
   - What we know: phase_disk_statuses are already computed in parse_project_state()
   - Recommendation: No async loading needed. Pipeline tab reads directly from ProjectState.phase_disk_statuses (already cached). This makes it the simplest tab implementation.

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `src/state_reader/disk_status.rs` -- DiskInference struct, infer_disk_status() function
- Codebase analysis: `src/ui/screens/detail.rs` -- tab system, disk_suffix(), phase list rendering
- Codebase analysis: `src/ui/screens/normal.rs` -- compact_pipeline() function with D-R-P-E-V color logic
- Codebase analysis: `src/config.rs` -- Config/Preferences struct, JSON-based config
- Codebase analysis: `src/app.rs` -- DetailSubView enum
- Codebase analysis: `src/ui/screens/mod.rs` -- ProjectViewCache, AppContext

### Secondary (MEDIUM confidence)
- CONTEXT.md decisions D-01 through D-12 -- user decisions from discuss phase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all existing crates
- Architecture: HIGH -- extends well-established patterns (tab system, DiskInference, split-pane)
- Pitfalls: HIGH -- identified from direct code analysis of edge cases

**Research date:** 2026-03-26
**Valid until:** 2026-04-26 (stable -- internal codebase patterns, no external API changes)
