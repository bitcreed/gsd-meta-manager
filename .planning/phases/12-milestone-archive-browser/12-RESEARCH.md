# Phase 12: Milestone Archive Browser - Research

**Researched:** 2026-03-31
**Domain:** Rust TUI - ratatui tab extension, async file I/O, markdown rendering
**Confidence:** HIGH

## Summary

This phase adds an 8th tab ("Archive") to the detail view, enabling users to browse completed milestones, drill into their phases and artifact files, and view file content with basic styled markdown rendering. The work builds entirely on established patterns already in the codebase: `ListState`-based selection (Backlog/Queue tabs), async data loading via `spawn_blocking` + `mpsc` channel (Git/Sessions), and the existing `DetailSubView` enum extension point.

The primary complexity lies in (1) the multi-level drill-down navigation (milestone -> phase -> file -> content) with Esc-to-go-back, which is a new pattern not exactly matching any existing tab, and (2) the custom markdown line renderer. Both are bounded in scope. The tab bar overflow at 80 columns (81+ chars rendered with the current label scheme) requires abbreviating some tab labels or using shorter names.

**Primary recommendation:** Structure as a new module `src/archive.rs` for data types and loading logic, with rendering integrated into `detail.rs` following existing tab patterns. Use an enum-based depth tracker for the 4-level drill-down.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Sequential list drill-down model (back with Esc) -- matches existing Backlog/Queue patterns using `ListState`
- Breadcrumb header bar showing current depth: `Archive > v1.0 > Phase 01 > PLAN.md`
- `Esc` returns to parent level (consistent with existing detail view escape behavior)
- Archive tab is the 8th tab (after Sessions) -- index 7 in `DetailSubView` enum
- Custom parser using line-by-line regex (no pulldown-cmark dependency) -- `#` headers get bold, triple-backtick code blocks get DarkGray background, `**bold**` gets bold, `- ` lists get bullet prefix
- Code blocks: DarkGray background with no syntax highlighting -- distinguishable from prose
- Full file content in a scrollable Paragraph widget (up/down to scroll)
- Header tiers: Bold + Cyan for `#`, Bold for `##`, Bold + dim underline for `###`
- Discover archived milestones by scanning `.planning/milestones/` for `v*-ROADMAP.md` files -- extract version from filename
- In-memory `HashMap<String, MilestoneArchive>` in `AppContext` -- populated on first access per milestone, never invalidated (archived milestones are immutable)
- `tokio::spawn_blocking` for file I/O, send results via existing `mpsc` channel -- same pattern as session detection
- "Loading..." text in the archive pane while data loads

### Claude's Discretion
- Internal data structures for `MilestoneArchive`, `PhaseArchive`, etc.
- How to extract phase list from archived ROADMAP.md (parse or filename scan)
- Scroll position tracking per drill-down level
- Whether to show file sizes or modification dates in the artifact list
- Tab bar overflow handling at 80 columns (noted as concern in STATE.md -- resolve or defer)

### Deferred Ideas (OUT OF SCOPE)
- Styled markdown in existing tabs (Backlog, etc.) -- deferred per REQUIREMENTS.md (MKDN-01 is future)
- Search within archive files
- File diffing between milestones
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ARCH-01 | User can see a list of completed milestones in an Archive tab within the detail view | Tab extension pattern (DetailSubView::Archive at index 7), milestone discovery via `v*-ROADMAP.md` glob, async loading pattern |
| ARCH-02 | User can drill into a milestone to see its phases, then into a phase to see its artifact files | 4-level ArchiveDepth enum, directory structure documented (v{X}-phases/{NN}-{slug}/), ListState per level |
| ARCH-03 | User can view a selected artifact file with styled markdown rendering (headers, bold, lists, code blocks) | Line-by-line regex parser, ratatui Span/Line styling, header/bold/code-block/list rules documented |
| ARCH-04 | Archive data is loaded asynchronously and cached (completed milestones are immutable) | spawn_blocking + mpsc pattern (same as SessionsDetected), DispatchAction already wired, HashMap cache in AppContext |
</phase_requirements>

## Architecture Patterns

### Recommended Project Structure

```
src/
├── archive.rs           # NEW: MilestoneArchive types, discovery, loading, markdown parser
├── action.rs            # MODIFY: Add ArchiveLoaded action variant
├── app.rs               # MODIFY: Add DetailSubView::Archive, handle ArchiveLoaded action
├── ui/screens/
│   ├── mod.rs           # MODIFY: Add archive cache field to AppContext
│   └── detail.rs        # MODIFY: Add Archive tab rendering, key handling, breadcrumb
```

### Pattern 1: Four-Level Drill-Down Navigation

**What:** An enum tracks the current depth within the archive browser. Each level has its own list selection state. Esc pops up one level; at the top level, Esc pops the detail screen (existing behavior).

**When to use:** When the Archive tab is active.

**Example:**
```rust
/// Depth levels for archive drill-down navigation.
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveDepth {
    /// Level 0: List of milestones (v1.0, v1.1, ...)
    MilestoneList,
    /// Level 1: Phases within a selected milestone
    PhaseList { milestone: String },
    /// Level 2: Artifact files within a selected phase
    FileList { milestone: String, phase_idx: usize },
    /// Level 3: Viewing file content with markdown styling
    FileView { milestone: String, phase_idx: usize, file_idx: usize },
}
```

**Key integration:** In the Esc handler in `detail.rs`, check if current view is `DetailSubView::Archive` and if depth > MilestoneList. If so, pop depth instead of popping the screen.

### Pattern 2: Archive State in ProjectViewCache

**What:** Extend `ProjectViewCache` with archive-specific fields rather than using a separate cache. This follows the existing pattern where each tab's state lives in the per-project view cache.

**Example:**
```rust
// Added to ProjectViewCache in mod.rs
pub archive_depth: ArchiveDepth,
pub archive_milestones: Vec<String>,         // ["v1.0", "v1.1"]
pub archive_selected: [usize; 4],            // Selection index per depth level
pub archive_scroll_offset: u16,              // For file content scrolling at Level 3
pub archive_loading: bool,
```

The milestone data cache (`HashMap<String, MilestoneArchive>`) goes in `AppContext` since it's shared across projects (a milestone belongs to the project, but the cache pattern is global).

### Pattern 3: Async Loading via Existing Channel

**What:** When the user first opens the Archive tab or drills into a milestone, fire a `spawn_blocking` task that reads the filesystem and sends results back via the existing `mpsc` channel. This matches the Git log and Sessions detection patterns.

**Example:**
```rust
// In action.rs - new variant
Action::ArchiveLoaded {
    alias: String,
    milestone: String,
    data: MilestoneArchive,
}

// In detail.rs - when user enters Archive tab or drills into a milestone
if let Some(tx) = &ctx.event_tx {
    let tx = tx.clone();
    let milestones_dir = project.path.join(".planning/milestones");
    let alias = self.alias.clone();
    tokio::task::spawn_blocking(move || {
        let data = load_milestone_archive(&milestones_dir, &milestone_version);
        let _ = tx.send(Action::ArchiveLoaded { alias, milestone: milestone_version, data });
    });
}
```

**Note:** `DispatchAction` variant on `ScreenAction` is already wired in `app.rs` (line 373) and `mod.rs` (line 45) with `#[allow(dead_code)]` -- this phase activates it.

### Pattern 4: Line-by-Line Markdown Renderer

**What:** A function that takes raw file content (String) and returns `Vec<Line<'_>>` with ratatui styling. No dependency on pulldown-cmark.

**Rules (from locked decisions):**
- `# heading` -> Bold + Cyan
- `## heading` -> Bold
- `### heading` -> Bold + dim underline
- `**bold text**` -> Bold spans (inline, may mix with normal text)
- `` ``` `` (triple backtick) toggles code block mode -> DarkGray background for all lines inside
- `- item` -> bullet prefix (render as-is or replace dash with bullet character)
- Everything else -> default style

**Example:**
```rust
pub fn render_markdown_lines(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for raw_line in content.lines() {
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            lines.push(Line::from(Span::styled(raw_line.to_string(),
                Style::default().bg(Color::DarkGray))));
            continue;
        }
        if in_code_block {
            lines.push(Line::from(Span::styled(raw_line.to_string(),
                Style::default().bg(Color::DarkGray))));
            continue;
        }
        if raw_line.starts_with("### ") {
            lines.push(Line::from(Span::styled(raw_line[4..].to_string(),
                Style::default().add_modifier(Modifier::BOLD | Modifier::DIM | Modifier::UNDERLINED))));
        } else if raw_line.starts_with("## ") {
            lines.push(Line::from(Span::styled(raw_line[3..].to_string(),
                Style::default().add_modifier(Modifier::BOLD))));
        } else if raw_line.starts_with("# ") {
            lines.push(Line::from(Span::styled(raw_line[2..].to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
        } else {
            // Handle inline **bold** and list items
            lines.push(parse_inline_styles(raw_line));
        }
    }
    lines
}
```

### Anti-Patterns to Avoid

- **Blocking the render loop with file I/O:** All file reads for archive content MUST go through `spawn_blocking`. Even though archived files are small, consistency with the established async pattern prevents future issues.
- **Storing raw file content in AppContext permanently:** Archive files can be large in aggregate. Cache the parsed `MilestoneArchive` (with file list and metadata) eagerly, but load individual file content on demand when the user drills into a specific file.
- **Custom Widget for drill-down:** Don't create a new ratatui Widget trait impl for the archive. Use the existing `List` + `Paragraph` pattern from Backlog/GitHistory tabs. The drill-down is state management, not rendering.

## Existing Codebase Integration Points

### Files to Modify

| File | Change | Complexity |
|------|--------|-----------|
| `src/app.rs` | Add `DetailSubView::Archive` variant, handle `Action::ArchiveLoaded` in `update()` | LOW |
| `src/action.rs` | Add `ArchiveLoaded` and `ArchiveMilestonesDiscovered` action variants | LOW |
| `src/ui/screens/mod.rs` | Add archive cache fields to `AppContext` and `ProjectViewCache` | LOW |
| `src/ui/screens/detail.rs` | Add tab label, tab_index/sub_view_from_index mappings, Esc/Enter/j/k handlers for archive, render_archive_tab method | MEDIUM-HIGH (bulk of work) |
| `src/archive.rs` | NEW: data types, discovery, loading, markdown parser | MEDIUM |

### Existing Patterns to Reuse

1. **Tab extension:** `TAB_TITLES` array, `tab_index()`, `sub_view_from_index()`, `switch_to_tab()` -- add index 7 mappings
2. **List selection:** `ListState` with highlight_style Cyan+Bold and `> ` highlight_symbol (from Backlog tab)
3. **Async loading:** `spawn_blocking` + `event_tx.send(Action::...)` (from Sessions/Git)
4. **Loading indicator:** `"Loading..."` paragraph in DarkGray (from Git tab: `cache.loading_git`)
5. **Esc interception:** GitHistory tab already intercepts Esc to dismiss diff pane before popping screen -- Archive does the same for depth > 0

### Tab Bar Overflow Resolution

Current 7-tab rendering at 80 columns works fine. Adding an 8th tab causes overflow because ratatui's `Tabs` widget renders with spacing around dividers.

**Recommended fix (Claude's discretion area):** Abbreviate select tab labels to fit:

```rust
const TAB_TITLES: [&str; 8] = [
    "1:Phases",
    "2:Roadmap",
    "3:Backlog",
    "4:Git",
    "5:Pipe",       // was "5:Pipeline" (saves 4 chars)
    "6:Queue",
    "7:Sess",       // was "7:Sessions" (saves 4 chars)
    "8:Archive",
];
```

This brings total text to 59 chars + 7 dividers with spacing ~= 73 chars, well within 80 columns. Alternative: use number-only labels for less-used tabs. The abbreviation approach is simpler and preserves readability.

## Milestone Directory Structure (Verified)

Examined the actual `.planning/milestones/` directory. Structure is consistent:

```
.planning/milestones/
├── v1.0-MILESTONE-AUDIT.md      # Top-level milestone docs
├── v1.0-REQUIREMENTS.md
├── v1.0-ROADMAP.md              # Discovery marker: glob for v*-ROADMAP.md
├── v1.0-phases/                 # Phase artifacts directory
│   ├── 01-core-infrastructure/
│   │   ├── 01-01-PLAN.md
│   │   ├── 01-01-SUMMARY.md
│   │   ├── 01-CONTEXT.md
│   │   ├── 01-RESEARCH.md
│   │   └── 01-VERIFICATION.md
│   ├── 02-dashboard-and-navigation/
│   │   ├── 02-01-PLAN.md
│   │   └── ...
│   └── ...
├── v1.1-MILESTONE-AUDIT.md
├── v1.1-REQUIREMENTS.md
├── v1.1-ROADMAP.md
└── v1.1-phases/
    ├── 05-state-reader-accuracy/
    └── ...
```

**Discovery algorithm:**
1. Glob `v*-ROADMAP.md` in `.planning/milestones/` -> extract version strings ("v1.0", "v1.1")
2. For each version, the phases directory is `v{version}-phases/`
3. Phase subdirectories are `{NN}-{slug}/` format
4. Artifact files within each phase are `{NN}-{padded}-{TYPE}.md` or `{NN}-{TYPE}.md`

**File counts:** 88 files across 2 milestones (v1.0: 4 phases, v1.1: 5 phases). Loading is lightweight.

**Top-level milestone files** (ROADMAP.md, REQUIREMENTS.md, MILESTONE-AUDIT.md) should also be browsable at the milestone level, not just phases. Recommend showing them as a "Milestone Docs" section above the phase list when drilling into a milestone.

## Data Structure Recommendations

```rust
/// Cached archive data for a single milestone
#[derive(Debug, Clone)]
pub struct MilestoneArchive {
    pub version: String,                    // "v1.0"
    pub top_level_files: Vec<ArchiveFile>,  // ROADMAP.md, REQUIREMENTS.md, etc.
    pub phases: Vec<PhaseArchive>,
}

#[derive(Debug, Clone)]
pub struct PhaseArchive {
    pub number: u32,                        // 1, 2, 3...
    pub name: String,                       // "core-infrastructure"
    pub display_name: String,               // "01 - Core Infrastructure"
    pub files: Vec<ArchiveFile>,
}

#[derive(Debug, Clone)]
pub struct ArchiveFile {
    pub name: String,                       // "01-01-PLAN.md"
    pub path: std::path::PathBuf,           // Absolute path for loading
    pub size: Option<u64>,                  // Optional, from metadata
}
```

**Phase list extraction:** Scanning the `v{version}-phases/` directory for subdirectories is simpler and more reliable than parsing ROADMAP.md. The directory names contain the phase number and slug. Recommend filesystem scan.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown AST parsing | Full CommonMark parser | Line-by-line regex (per locked decision) | Locked decision; no pulldown-cmark dependency. Line-by-line is sufficient for the rendering rules specified |
| File watching for archive | notify watcher on milestones dir | No watcher needed | Archives are immutable; cache never invalidates |
| Tree widget | Custom tree traversal widget | Sequential lists with depth enum | Locked decision; matches existing UX patterns |
| Breadcrumb rendering | Custom widget | `Line::from(vec![Span, ...])` inline | Simple span concatenation; no widget needed |

## Common Pitfalls

### Pitfall 1: Esc Key Conflict with Screen Pop
**What goes wrong:** Pressing Esc while in a drill-down level pops the entire detail screen instead of going up one archive level.
**Why it happens:** The existing Esc handler in `detail.rs` calls `ScreenAction::Pop` unconditionally (with one exception for GitHistory diff pane).
**How to avoid:** Add an Archive-specific check at the top of the Esc handler, before the `ScreenAction::Pop` return. If `archive_depth != MilestoneList`, set depth to parent level and return `ScreenAction::None`.
**Warning signs:** Pressing Esc while viewing a file takes you back to the dashboard instead of the file list.

### Pitfall 2: Stale Selection Indices After Drill-Down
**What goes wrong:** Selection index from a previous drill-down level persists and points to an out-of-bounds item in a new context.
**Why it happens:** Reusing a single `archive_selected` field across levels without resetting.
**How to avoid:** Use a per-level selection array `[usize; 4]` and reset the child level's selection to 0 when entering it. Keep parent selections stable for when user goes back.
**Warning signs:** Panic on index out of bounds or selecting the wrong item after navigating back and forth.

### Pitfall 3: Rendering Paragraph Without Scroll Clamping
**What goes wrong:** Scroll offset exceeds file content length, showing a blank pane.
**Why it happens:** `Paragraph::scroll((offset, 0))` accepts any value without clamping.
**How to avoid:** Clamp scroll offset to `content_lines.saturating_sub(visible_height)` before rendering. Reset scroll to 0 when entering a new file.
**Warning signs:** Blank archive pane after scrolling down in a short file.

### Pitfall 4: Tab Index Off-By-One in Number Key Handler
**What goes wrong:** Pressing `8` does not switch to the Archive tab.
**Why it happens:** The number key handler (`KeyCode::Char('1')..='7'`) needs to include `'8'`.
**How to avoid:** Check where number-key tab switching is handled in `detail.rs` and extend the range.
**Warning signs:** Pressing 8 does nothing while left/right arrows work.

### Pitfall 5: detail.rs Becoming Unmaintainable (1863 lines)
**What goes wrong:** Adding 200+ lines of archive rendering and key handling to an already large file makes it hard to navigate.
**Why it happens:** All tab logic lives in one file.
**How to avoid:** Put the archive rendering method and markdown parser in `src/archive.rs`. Keep only the match arm dispatch and minimal key handling in `detail.rs`. The rendering function takes `(&Frame, Rect, &AppContext, &str)` and `detail.rs` calls it.
**Warning signs:** 2000+ line detail.rs file.

## Code Examples

### Breadcrumb Header Rendering

```rust
fn archive_breadcrumb(depth: &ArchiveDepth, milestones: &[String], phases: &[PhaseArchive], files: &[ArchiveFile]) -> Line<'static> {
    let mut spans = vec![Span::styled("Archive", Style::default().fg(Color::Cyan))];

    match depth {
        ArchiveDepth::MilestoneList => {}
        ArchiveDepth::PhaseList { milestone } => {
            spans.push(Span::raw(" > "));
            spans.push(Span::styled(milestone.clone(), Style::default().fg(Color::Yellow)));
        }
        ArchiveDepth::FileList { milestone, phase_idx } => {
            spans.push(Span::raw(" > "));
            spans.push(Span::styled(milestone.clone(), Style::default().fg(Color::Yellow)));
            if let Some(phase) = phases.get(*phase_idx) {
                spans.push(Span::raw(" > "));
                spans.push(Span::raw(phase.display_name.clone()));
            }
        }
        ArchiveDepth::FileView { milestone, phase_idx, file_idx } => {
            spans.push(Span::raw(" > "));
            spans.push(Span::styled(milestone.clone(), Style::default().fg(Color::Yellow)));
            if let Some(phase) = phases.get(*phase_idx) {
                spans.push(Span::raw(" > "));
                spans.push(Span::raw(phase.display_name.clone()));
            }
            if let Some(file) = files.get(*file_idx) {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(file.name.clone(), Style::default().add_modifier(Modifier::BOLD)));
            }
        }
    }

    Line::from(spans)
}
```

### Milestone Discovery

```rust
use std::path::Path;

pub fn discover_milestones(milestones_dir: &Path) -> Vec<String> {
    let mut versions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(milestones_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(version) = name.strip_suffix("-ROADMAP.md") {
                if version.starts_with('v') {
                    versions.push(version.to_string());
                }
            }
        }
    }
    versions.sort_by(|a, b| {
        // Natural version sort: v1.0 < v1.1 < v2.0
        human_sort(a, b)
    });
    versions
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| pulldown-cmark for markdown | Line-by-line regex | Locked decision | No new dependency; simpler but less accurate for edge cases |
| tui-tree-widget for hierarchy | Sequential ListState drill-down | Locked decision | Consistent with existing UX; no new dependency |
| Eager load all archive files | Lazy load per-milestone on first access | Design decision | Fast initial load; pay cost only when browsing |

## Open Questions

1. **Top-level milestone files in drill-down**
   - What we know: Each milestone has ROADMAP.md, REQUIREMENTS.md, MILESTONE-AUDIT.md at the top level alongside the phases directory
   - What's unclear: Should these appear as browsable items when drilling into a milestone (alongside the phase list), or only the phases?
   - Recommendation: Show them in a "Docs" section above the phases list. They provide useful context and users would expect to find them. This is Claude's discretion per CONTEXT.md.

2. **File content caching strategy**
   - What we know: MilestoneArchive caches the directory structure. Individual file content could be cached too.
   - What's unclear: Should file content be cached after first read, or re-read each time?
   - Recommendation: Cache file content in the MilestoneArchive struct after first read. Archives are immutable so there's no staleness concern, and the total size across all milestones is small (88 files, mostly small markdown).

3. **Version sorting**
   - What we know: Milestones are named "v1.0", "v1.1", etc.
   - Recommendation: Simple string sort works for semver-like versions. If needed, split on `.` and compare numeric parts. Not critical for 2 milestones.

## Project Constraints (from CLAUDE.md)

- **No new Cargo dependencies** -- the line-by-line markdown parser and all archive logic uses only existing crate dependencies (std, ratatui, tokio, serde)
- **Non-intrusive** -- archive browsing is read-only on immutable files; no interference with running GSD instances
- **State reading from files** -- archive data read directly from `.planning/milestones/` filesystem; no need to invoke Claude/GSD
- **Stack patterns** -- use `tokio::spawn_blocking` for file I/O, send results via `mpsc` channel; never block the render loop
- **No tui-rs, no ncurses** -- use ratatui 0.30 with crossterm backend (already in use)

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection: `src/app.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/mod.rs`, `src/action.rs`, `src/session_detector.rs` -- all integration points verified
- Direct filesystem inspection: `.planning/milestones/` -- 88 files across 2 milestones, directory structure verified
- CONTEXT.md locked decisions -- all rendering rules and architecture choices specified by user

### Secondary (MEDIUM confidence)
- ratatui `Tabs` widget width calculation -- inferred from ratatui default rendering behavior with `|` divider; exact padding depends on ratatui 0.30 internals

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies, all existing crate capabilities verified in codebase
- Architecture: HIGH - all patterns directly observed in existing tabs (Backlog, Git, Sessions)
- Pitfalls: HIGH - identified from direct code reading of current Esc handling, selection state, and file size
- Markdown rendering: MEDIUM - regex approach is straightforward but inline `**bold**` parsing across mixed content needs careful implementation

**Research date:** 2026-03-31
**Valid until:** 2026-04-30 (stable -- no external dependency changes expected)
