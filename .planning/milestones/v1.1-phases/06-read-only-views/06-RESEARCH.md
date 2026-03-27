# Phase 06: Read-Only Views - Research

**Researched:** 2026-03-26
**Domain:** TUI tab navigation, backlog file parsing, git subprocess integration
**Confidence:** HIGH

## Summary

Phase 06 adds two read-only browsing views to the existing detail screen: a backlog browser for 999.* items and a scrollable git history viewer. Both integrate as tabs alongside the existing phase list and roadmap views, using the Screen trait architecture from Phase 05.

The primary challenge is extending the DetailScreen from its current two-view toggle (`PhaseList`/`RoadmapViz`) into a multi-tab system with 4+ tabs, while keeping the existing rendering logic intact. Git data comes from `tokio::process::Command` shell-outs to `git log`/`git diff-tree`, and backlog data from filesystem reads of `.planning/phases/999.*` directories. Both are async operations that should show loading states.

**Primary recommendation:** Extend `DetailSubView` enum to include `Backlog` and `GitHistory` variants, add a tab bar header widget, implement async data fetching via `spawn_blocking` + `Action` dispatch (matching the existing `ProjectStateLoaded` pattern), and introduce two new data structs (`BacklogItem`, `GitLogEntry`) in the state_reader module.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Backlog browser lives as a tab/section in the detail view, consistent with how phases are already shown
- **D-02:** Backlog items displayed as a scrollable list showing `999.N-slug` with one-line description parsed from first markdown heading
- **D-03:** Backlog item content shown in an inline expandable panel below the list (split pane style, no screen switch)
- **D-04:** "Queue promotion" pre-fills the enqueue screen with `/gsd:review-backlog` command for the selected item
- **D-05:** Git history lives as a tab/section in the detail view, same pattern as backlog
- **D-06:** One-line commit format: `hash (7) — date — message` with author dimmed
- **D-07:** Keybind toggle (e.g., `p`) switches between full-repo and .planning/-only history, with indicator in header
- **D-08:** Selecting a commit shows inline diff stat summary (files changed, +/- lines) below the log
- **D-09:** Tab switching via number keys `1`/`2`/`3` or left/right arrows, with a tab bar header showing current view
- **D-10:** Git data accessed via shell-out to `git log`/`git diff` using `tokio::process::Command` -- no git2 crate dependency
- **D-11:** Backlog item content read from `.planning/phases/999.N-*/` directories, parsing first `.md` file found
- **D-12:** Async loading shows dim "Loading..." text in the view area while data fetches

### Claude's Discretion
- Exact tab bar styling and layout proportions
- Split pane sizing for backlog content preview
- Git log page size and scrolling behavior
- Diff stat formatting details

### Deferred Ideas (OUT OF SCOPE)
- Backlog item editing (BLOG-05) -- write operations are v2+
- Direct backlog promotion to phase (BLOG-04) -- needs more UX design
- Full diff view screen -- inline stats sufficient for read-only
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BLOG-01 | User can browse backlog items (999.*) in a scrollable list within detail view | Tab system + backlog parsing from filesystem; `count_backlog_items` already exists, extend to return full metadata |
| BLOG-02 | User can view backlog item details (markdown content) | Split pane in backlog tab; read first `.md` file from 999.N-* directory; fallback to slug display if no markdown |
| BLOG-03 | User can queue a promotion command for a backlog item | Pre-fill EnqueueScreen with `/gsd:review-backlog <item>` command; reuse existing enqueue push pattern |
| GIT-01 | User can view scrollable git log for a project | `git log --oneline --format` via `tokio::process::Command`; rendered as scrollable list in git tab |
| GIT-02 | User can toggle between full repo and .planning/-scoped history | `p` keybind toggles `-- .planning/` path filter on git log command; header indicator shows current mode |
| GIT-03 | User can view commit diff stats by selecting a commit | `git diff-tree --stat -r <hash>` via subprocess; inline panel below log list |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI rendering -- List, Paragraph, Layout for tabs and split panes | Already in use |
| crossterm | 0.29 | Terminal backend + key events | Already in use |
| tokio | 1.x | Async runtime for `process::Command` and `spawn_blocking` | Already in use |

### No New Dependencies Required

This phase uses only existing dependencies. Git interaction is via `tokio::process::Command` (part of tokio). Filesystem reads use `std::fs`. No new crates needed.

## Architecture Patterns

### Extended DetailSubView Enum

The current `DetailSubView` enum in `src/app.rs` has two variants:
```rust
pub enum DetailSubView {
    PhaseList,   // default
    RoadmapViz,
}
```

Extend to:
```rust
pub enum DetailSubView {
    PhaseList,   // Tab 1
    RoadmapViz,  // Tab 2
    Backlog,     // Tab 3  (new)
    GitHistory,  // Tab 4  (new)
}
```

### Tab Bar Header Pattern

Currently, the `r` key toggles between PhaseList and RoadmapViz. Replace this with a proper tab bar:

```rust
// Tab bar rendering pattern
fn render_tab_bar(frame: &mut Frame, area: Rect, active: &DetailSubView) {
    let tabs = vec!["1:Phases", "2:Roadmap", "3:Backlog", "4:Git"];
    // Render as horizontal spans, highlight active tab
    // Use bold+underline for active, dimmed for inactive
}
```

Key bindings per D-09:
- `1`, `2`, `3`, `4` -- direct tab selection
- Left/Right arrows -- cycle through tabs (overrides scroll when at tab bar level)
- `j`/`k` -- scroll within the active tab content

**Navigation conflict resolution:** Currently `j`/`k` scroll the detail view and Left/Right are unused. The new tab bar uses Left/Right for tab switching. The `r` key should be removed (replaced by tab `2`).

### Backlog Data Model

New struct in `src/state_reader/`:
```rust
pub struct BacklogItem {
    pub dir_name: String,      // e.g., "999.3-queue-editor-and-reorder"
    pub number: String,        // e.g., "999.3"
    pub slug: String,          // e.g., "queue-editor-and-reorder"
    pub description: String,   // First markdown heading, or slug humanized
    pub content: Option<String>, // Full markdown content (loaded on demand)
}
```

Parsing approach:
1. Read `.planning/phases/` directory entries matching `999*` prefix and `is_dir()`
2. Extract number and slug from directory name: split on first `-` after the `999.N` prefix
3. For description: look for first `.md` file in directory, parse first `# heading`
4. Fallback: humanize slug (replace `-` with spaces, title case)

**Current state observation:** Existing 999.* directories in this project contain only `.gitkeep` files -- no markdown content yet. The code must handle this gracefully (show slug as description, show "No content" for detail view).

### Git History Data Model

```rust
pub struct GitLogEntry {
    pub hash: String,          // 7-char abbreviated hash
    pub date: String,          // ISO or relative date
    pub author: String,        // Author name
    pub message: String,       // First line of commit message
}

pub struct GitDiffStat {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub file_stats: Vec<String>, // Per-file stat lines
}
```

### Git Subprocess Pattern

Use `tokio::process::Command` (not `std::process::Command`) since we need async:

```rust
use tokio::process::Command;

async fn git_log(project_path: &Path, planning_only: bool, limit: usize) -> anyhow::Result<Vec<GitLogEntry>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(project_path)
       .arg("log")
       .arg(format!("--max-count={}", limit))
       .arg("--format=%h\x1f%ad\x1f%an\x1f%s")
       .arg("--date=short");

    if planning_only {
        cmd.arg("--").arg(".planning/");
    }

    let output = cmd.output().await?;
    // Parse \x1f-delimited fields per line
    Ok(parse_git_log_output(&output.stdout))
}

async fn git_diff_stat(project_path: &Path, hash: &str) -> anyhow::Result<GitDiffStat> {
    let output = Command::new("git")
        .current_dir(project_path)
        .arg("diff-tree")
        .arg("--stat")
        .arg("-r")
        .arg(hash)
        .output()
        .await?;
    Ok(parse_diff_stat_output(&output.stdout))
}
```

**Key detail:** Use `\x1f` (unit separator) as field delimiter in `--format` to avoid ambiguity with commit messages containing `--` or `|`. This is a standard git formatting trick.

### Async Loading Pattern

Follow the existing `ProjectStateLoaded` pattern. Add new Action variants:

```rust
pub enum Action {
    // ... existing variants ...
    BacklogLoaded {
        alias: String,
        items: Vec<BacklogItem>,
    },
    BacklogContentLoaded {
        alias: String,
        item_number: String,
        content: String,
    },
    GitLogLoaded {
        alias: String,
        entries: Vec<GitLogEntry>,
        planning_only: bool,
    },
    GitDiffStatLoaded {
        alias: String,
        hash: String,
        stat: GitDiffStat,
    },
}
```

Trigger loading when switching to a tab for the first time (or when data is stale). Show dim "Loading..." text until the action arrives.

### State Storage on AppContext

Add per-project cached data to AppContext (or a new struct):

```rust
// In AppContext or a new DetailViewState struct
pub struct ProjectViewCache {
    pub backlog_items: Vec<BacklogItem>,
    pub backlog_selected: usize,
    pub backlog_expanded: bool,         // Is content panel visible?
    pub git_entries: Vec<GitLogEntry>,
    pub git_selected: usize,
    pub git_planning_only: bool,        // Toggle state for D-07
    pub git_diff_stat: Option<GitDiffStat>,
    pub loading_backlog: bool,
    pub loading_git: bool,
    pub loading_diff: bool,
}

// On AppContext:
pub view_cache: HashMap<String, ProjectViewCache>,
```

### Split Pane for Content Preview (D-03, D-08)

Both backlog content and git diff stat use an inline expandable panel below the list:

```rust
// Layout pattern for split pane
let chunks = if expanded {
    Layout::vertical([
        Constraint::Percentage(50),  // List
        Constraint::Percentage(50),  // Content/diff
    ]).split(content_area)
} else {
    Layout::vertical([
        Constraint::Min(0),  // List only
    ]).split(content_area)
};
```

### Recommended Project Structure Changes

```
src/
  state_reader/
    mod.rs              # Extended: load_backlog_items() function
    backlog.rs          # NEW: BacklogItem struct + parsing
    git_ops.rs          # NEW: GitLogEntry, GitDiffStat, git subprocess functions
  ui/
    screens/
      detail.rs         # MODIFIED: tab system, delegates to tab renderers
      detail/           # NEW directory (optional: could keep in detail.rs)
        mod.rs          # Tab dispatch
        backlog_tab.rs  # Backlog list + content rendering
        git_tab.rs      # Git log + diff stat rendering
  action.rs             # MODIFIED: new Action variants
  app.rs                # MODIFIED: DetailSubView extended, handle new actions
```

**Alternative (simpler):** Keep all tab rendering in `detail.rs` with helper functions. The file is currently ~595 lines -- adding ~200 lines for each tab keeps it under 1000 lines, which is manageable. Splitting into a `detail/` module directory is a judgment call based on final size.

### Anti-Patterns to Avoid

- **Blocking git subprocess in render loop:** Always use `tokio::process::Command` with async `.output().await` or `spawn_blocking` for `std::process::Command`. Never call synchronous `Command::new("git")...status()` from the render path.
- **Unbounded git log output:** Always pass `--max-count=N` to `git log`. A project with 10000 commits would freeze the TUI without a limit. Start with 50-100 entries, add pagination later if needed.
- **Parsing git output with split on spaces/dashes:** Use `\x1f` unit separator in `--format` to reliably delimit fields. Commit messages and author names can contain arbitrary characters.
- **Holding the entire backlog content in memory for all items:** Load content on demand when an item is selected/expanded, not all at once.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git log parsing | Custom regex on `git log` default output | `--format=%h\x1f%ad\x1f%an\x1f%s` with unit separator | Default output format is ambiguous; structured format avoids edge cases |
| Git diff stats | Parse `git diff` full output | `git diff-tree --stat -r <hash>` | `--stat` gives pre-formatted summary; `diff-tree` works on single commits without needing parent ref |
| Tab bar widget | Custom character-by-character drawing | ratatui `Tabs` widget | Built-in widget handles highlighting, spacing, styling |

**Key insight:** ratatui 0.30 includes a `Tabs` widget that handles tab bar rendering natively. Use it instead of manually composing `Span` elements for the tab bar. The `Tabs` widget accepts titles and a selected index.

## Common Pitfalls

### Pitfall 1: Git not initialized in project directory
**What goes wrong:** `git log` returns error exit code, stderr says "not a git repository"
**Why it happens:** Registered projects may not have git initialized
**How to avoid:** Check for `.git` directory before running git commands; show "No git repository" message in the git tab
**Warning signs:** `output.status.success()` returns false

### Pitfall 2: Empty backlog directories
**What goes wrong:** Backlog items show but content is empty
**Why it happens:** Some 999.* directories contain only `.gitkeep` (no markdown files)
**How to avoid:** Gracefully handle missing `.md` files; show "No description available" text; use humanized slug as title
**Warning signs:** `fs::read_dir` returns entries but none match `*.md`

### Pitfall 3: Corrupted 999.* directory names
**What goes wrong:** The existing `999.1-{` and `999.2-{` directories have JSON-like slugs with newlines in the directory name
**Why it happens:** A bug in earlier tooling created directories with JSON objects as names
**How to avoid:** Robust parsing that handles any characters in directory names; filter out entries where the slug doesn't look valid (contains `{`, newlines, etc.)
**Warning signs:** Directory names containing `{`, `}`, or newline characters

### Pitfall 4: Navigation key conflicts
**What goes wrong:** Left/Right arrows used for tab switching conflict with potential future horizontal scrolling
**Why it happens:** Limited key space in TUI
**How to avoid:** Use number keys (`1`-`4`) as primary tab selectors; Left/Right as secondary. Within tabs, `j`/`k`/Up/Down for vertical movement. `Enter` to expand/collapse content panels. `p` for git planning-only toggle.
**Warning signs:** User reports "keys don't work as expected"

### Pitfall 5: tokio::process::Command vs std::process::Command
**What goes wrong:** Using `std::process::Command` from async context blocks the tokio runtime
**Why it happens:** Easy to import the wrong `Command`
**How to avoid:** For git operations called from async context, use `tokio::process::Command`. For operations in `spawn_blocking`, `std::process::Command` is fine. The existing `project_creator.rs` uses `std::process::Command` inside `spawn_blocking` -- both approaches work.
**Warning signs:** TUI freezes briefly when switching to git tab

### Pitfall 6: Git log for projects without commits
**What goes wrong:** `git log` on a fresh repo (no commits) returns exit code 128
**Why it happens:** `git log` on an empty repo with no HEAD fails
**How to avoid:** Check `git rev-parse HEAD` first, or handle the error gracefully and show "No commits yet"
**Warning signs:** Error on newly created projects

## Code Examples

### Tab Bar with ratatui Tabs Widget
```rust
use ratatui::widgets::Tabs;
use ratatui::text::Line;

let titles = vec!["Phases", "Roadmap", "Backlog", "Git"];
let tab_index = match sub_view {
    DetailSubView::PhaseList => 0,
    DetailSubView::RoadmapViz => 1,
    DetailSubView::Backlog => 2,
    DetailSubView::GitHistory => 3,
};

let tabs = Tabs::new(titles)
    .select(tab_index)
    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
    .divider("|");

frame.render_widget(tabs, tab_area);
```

### Parsing Backlog Directory Names
```rust
pub fn parse_backlog_dir_name(name: &str) -> Option<(String, String)> {
    // Skip invalid entries (JSON-named dirs, etc.)
    if name.contains('{') || name.contains('\n') {
        return None;
    }
    // Expected: "999.N-slug-text"
    let after_999 = name.strip_prefix("999.")?;
    let dot_pos = after_999.find('-')?;
    let number = format!("999.{}", &after_999[..dot_pos]);
    let slug = after_999[dot_pos + 1..].to_string();
    Some((number, slug))
}
```

### Git Log with Unit Separator
```rust
async fn load_git_log(
    project_path: &Path,
    planning_only: bool,
    limit: usize,
) -> anyhow::Result<Vec<GitLogEntry>> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(project_path)
        .arg("log")
        .arg(format!("--max-count={}", limit))
        .arg("--format=%h\x1f%ad\x1f%an\x1f%s")
        .arg("--date=short");

    if planning_only {
        cmd.arg("--").arg(".planning/");
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        return Ok(Vec::new()); // Graceful degradation
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            if parts.len() == 4 {
                Some(GitLogEntry {
                    hash: parts[0].to_string(),
                    date: parts[1].to_string(),
                    author: parts[2].to_string(),
                    message: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(entries)
}
```

### Diff Stat via git diff-tree
```rust
async fn load_diff_stat(project_path: &Path, hash: &str) -> anyhow::Result<GitDiffStat> {
    let output = tokio::process::Command::new("git")
        .current_dir(project_path)
        .arg("diff-tree")
        .arg("--stat")
        .arg("--no-commit-id")
        .arg("-r")
        .arg(hash)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout.lines().map(|l| l.to_string()).collect();

    // Last line is summary: " N files changed, M insertions(+), K deletions(-)"
    // All other lines are per-file stats
    let (file_stats, summary) = if lines.len() > 1 {
        (lines[..lines.len()-1].to_vec(), lines.last().cloned())
    } else {
        (Vec::new(), lines.first().cloned())
    };

    // Parse summary line for counts
    // ... regex or manual parse of "N files changed, M insertions(+), K deletions(-)"

    Ok(GitDiffStat { files_changed, insertions, deletions, file_stats })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| git2 crate for Rust git access | Shell-out to git CLI | Decision D-10 | Avoids libgit2 C dependency, simpler error handling, matches user's git config |
| DetailSubView as bool toggle | Multi-variant enum with tab bar | This phase | Enables N-tab detail view extensibility |

## Open Questions

1. **Git log page size**
   - What we know: Must limit `--max-count` to avoid loading all commits
   - What's unclear: Optimal initial page size (50? 100?)
   - Recommendation: Start with 50, show "Load more..." at bottom. Good enough for read-only browsing.

2. **Tab persistence per project**
   - What we know: `detail_sub_view_per_project` already stores view per project
   - What's unclear: Should the selected backlog item / git entry also persist per project?
   - Recommendation: Yes, store `ProjectViewCache` in a HashMap keyed by alias. Reset on project state reload.

3. **Backlog items without markdown content**
   - What we know: Current backlog dirs are empty (only `.gitkeep`)
   - What's unclear: Whether this is the permanent state or will be populated later
   - Recommendation: Handle gracefully -- show slug as title, "No content" as body. When `.md` files appear later, they'll be picked up.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | Git history viewer (GIT-01, GIT-02, GIT-03) | Yes | 2.43.0 | Show "git not found" message in git tab |

No other external dependencies required. All libraries already in Cargo.toml.

## Project Constraints (from CLAUDE.md)

- **State reading:** Must not require running Claude/GSD to check status -- read from files or cached state. Git data via `git` CLI is acceptable (it reads local state).
- **Non-intrusive:** Must not interfere with running GSD instances. Read-only views satisfy this by definition.
- **Stack:** Rust + ratatui 0.30 + crossterm 0.29 + tokio 1.x (all already in use)
- **Pattern:** TEA pattern with Action enum, mpsc EventBus, stateless components
- **No git2 crate:** D-10 locks this -- use `tokio::process::Command` for git operations
- **GSD Workflow Enforcement:** Changes must go through GSD workflow commands

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `src/ui/screens/detail.rs` (595 lines), `src/ui/screens/mod.rs`, `src/state_reader/mod.rs`, `src/app.rs` -- current architecture patterns
- Codebase analysis: `src/project_creator.rs` -- existing `std::process::Command` usage for git
- Codebase analysis: `src/action.rs` -- Action enum pattern for async results
- Filesystem inspection: `.planning/phases/999.*` directories -- actual backlog structure (empty dirs with `.gitkeep`)
- ratatui `Tabs` widget -- built-in tab bar rendering (part of ratatui 0.30 standard widget set)

### Secondary (MEDIUM confidence)
- `git diff-tree --stat` output format -- standard git CLI behavior, widely documented
- `git log --format` with `\x1f` separator -- common pattern for machine-readable git output

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all patterns already established in codebase
- Architecture: HIGH -- extending existing patterns (DetailSubView enum, Action dispatch, Screen trait)
- Pitfalls: HIGH -- identified from direct codebase inspection (corrupted 999.* dirs, empty repos, async Command)

**Research date:** 2026-03-26
**Valid until:** 2026-04-26 (stable -- no external dependency changes expected)
