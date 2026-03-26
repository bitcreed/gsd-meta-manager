# Architecture: v1.1 Feature Integration

**Domain:** TUI multi-project management dashboard — v1.1 power features
**Researched:** 2026-03-26
**Confidence:** HIGH (based on actual codebase analysis, verified Claude CLI structure, and real session file inspection)

## Existing Architecture Summary

The v1.0 codebase follows TEA (The Elm Architecture):

- **App struct** (app.rs): Monolithic root state — config, project_states HashMap, input_mode, UI state
- **Action enum** (action.rs): 5 variants — Tick, RawKey, Resize, FileChanged, CreateProjectResult
- **EventBus** (event.rs): tokio mpsc unbounded channel, crossterm reader + tick timer as producers
- **InputMode enum**: 11 variants controlling modal navigation (Normal, DetailView, EnqueueInput, etc.)
- **StateReader** (state_reader/): Parses STATE.md (YAML frontmatter), ROADMAP.md (regex), QUEUE.md (line-based)
- **FileWatcher** (watcher.rs): notify-debouncer-full, sends Action::FileChanged with project_path
- **UI modules** (ui/): Stateless render functions dispatched by InputMode — project_list, detail_view, roadmap_widget, help_overlay

**Key characteristics:**
- App.update() is ~140 lines, handles all actions in a single match
- InputMode variants carry data (e.g., DetailView { alias }, EnqueueInput { alias })
- Per-project sub-view state lives in `HashMap<String, DetailSubView>` on App
- All state mutation flows through App::update(); UI is pure render

## New Features — Integration Analysis

### Feature 1: Claude Session Management

**What it does:** Detect active Claude Code sessions per project, show session status in dashboard and detail view, allow launching/resuming sessions from TUI.

**Data source (verified on disk):**
- `~/.claude/projects/<url-encoded-path>/` contains `<uuid>.jsonl` session files
- Path encoding: `/home/blk/projects/rust/gsd-manager` becomes `-home-blk-projects-rust-gsd-manager`
- Each JSONL entry has: `type` (user/assistant/progress/file-history-snapshot), `sessionId`, `timestamp`, `cwd`, `gitBranch`
- No `sessions-index.json` exists on this system — must parse JSONL directly or scan directory
- Running processes detectable via `ps` — process name is `claude` with args like `--resume <session-id>`

**New components:**

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| `ClaudeSessionReader` | Data reader | `src/state_reader/claude_sessions.rs` | Scan `~/.claude/projects/` for session files matching a project path |
| `ClaudeProcessDetector` | Infrastructure | `src/claude_detector.rs` | Check running `claude` processes via `/proc` or `ps` output |
| `SessionListView` | UI component | `src/ui/session_list.rs` | Render session list with status indicators |

**New data structures:**

```rust
// In state_reader/claude_sessions.rs
pub struct ClaudeSession {
    pub session_id: String,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub first_prompt: Option<String>,  // extracted from first "user" type entry
    pub message_count: u32,
    pub git_branch: Option<String>,
    pub is_active: bool,  // true if matching process found
}

// Added to ProjectState
pub struct ProjectState {
    // ... existing fields ...
    pub claude_sessions: Vec<ClaudeSession>,
    pub active_session_count: u32,
}
```

**New Action variants:**

```rust
Action::RefreshClaudeSessions { project_path: PathBuf },
Action::LaunchClaudeSession { alias: String, command: String },
Action::ResumeClaudeSession { alias: String, session_id: String },
Action::ClaudeSessionResult { alias: String, success: bool, error: Option<String> },
```

**New InputMode variants:**

```rust
InputMode::SessionList { alias: String },
InputMode::LaunchSession { alias: String },
```

**Integration points:**
- `parse_project_state()` gains optional claude session scanning (guarded by feature flag or lazy-loaded on detail view entry)
- Detail view gets new sub-view tab: PhaseList / RoadmapViz / **Sessions**
- Dashboard can show a session indicator column (active session count)
- Session launch uses `tokio::process::Command` to spawn `claude` in a new terminal (detached)

**Critical constraint:** Session scanning reads potentially large JSONL files. Must NOT scan on every file-watcher tick. Scan only on explicit navigation to session view, or on a slower timer (every 30s).

### Feature 2: Queue Execution

**What it does:** Make QUEUE.md items actionable — execute queued GSD commands by launching Claude sessions with the command as the initial prompt.

**Integration with existing code:**
- QUEUE.md parsing already exists in `state_reader/queue_md.rs`
- EnqueueInput mode already works for adding items
- Execution means: `claude -p "<command>"` or `claude "<command>"` in the project directory

**New components:**

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| `QueueExecutor` | Infrastructure | `src/queue_executor.rs` | Spawn Claude process with queue item as prompt |
| Queue editor enhancements | UI modification | `src/ui/detail_view.rs` | Add reorder, delete, execute actions to queue display |

**New Action variants:**

```rust
Action::ExecuteQueueItem { alias: String, index: usize },
Action::QueueExecutionResult { alias: String, index: usize, success: bool, error: Option<String> },
Action::RemoveQueueItem { alias: String, index: usize },
Action::ReorderQueueItem { alias: String, from: usize, to: usize },
```

**New InputMode variant:**

```rust
InputMode::QueueView { alias: String, selected: usize },
```

**Integration points:**
- `QueuedAction` struct gains `status: QueueItemStatus` (Pending/Running/Done/Failed)
- Detail view queue section becomes interactive (not just display)
- Execution spawns detached `claude` process — TUI does not wait for completion
- Queue item removal modifies QUEUE.md via existing `save_queue()` function

### Feature 3: Git History Viewer

**What it does:** Scrollable git log for a project, with ability to scope to `.planning/` changes only.

**Implementation approach:** Use `std::process::Command` to call `git log` rather than linking `libgit2`. Rationale:
- git2 crate adds ~3MB to binary and requires libgit2 C library build
- `git log --oneline --format=...` gives structured output trivially
- The viewer is read-only; no git write operations needed
- Every GSD project is a git repo by definition

**New components:**

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| `GitLogReader` | Data reader | `src/state_reader/git_log.rs` | Run `git log` and parse structured output |
| `GitHistoryView` | UI component | `src/ui/git_history.rs` | Scrollable commit list with filtering |

**New data structures:**

```rust
pub struct GitCommit {
    pub hash: String,       // short hash
    pub message: String,    // first line
    pub author: String,
    pub date: String,       // relative date
    pub files_changed: u32, // from --stat
}

pub struct GitHistoryState {
    pub commits: Vec<GitCommit>,
    pub scroll_offset: usize,
    pub scope: GitScope,       // All or PlanningOnly
    pub loading: bool,
}

pub enum GitScope {
    All,
    PlanningOnly,  // git log -- .planning/
}
```

**New Action variants:**

```rust
Action::LoadGitHistory { alias: String, scope: GitScope },
Action::GitHistoryLoaded { alias: String, commits: Vec<GitCommit> },
```

**New InputMode variant:**

```rust
InputMode::GitHistory { alias: String },
```

**Integration points:**
- Detail view gets new sub-view or keybinding (`g` for git history)
- Git log is loaded async via `tokio::task::spawn_blocking` (runs in thread pool)
- Per-project git history state stored in `HashMap<String, GitHistoryState>` on App
- Scope toggle (`Tab` key) switches between all commits and .planning/-only

### Feature 4: Backlog Browser

**What it does:** View, edit, and promote backlog items (999.x directories) from the TUI.

**Data source (verified on disk):**
- Backlog items are directories in `.planning/phases/` matching `999.*` pattern
- Directory names follow pattern: `999.N-slug-name`
- May contain `CONCEPT.md` or be empty
- Promotion means: renaming from `999.N-slug` to a real phase number directory

**New components:**

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| `BacklogReader` | Data reader | `src/state_reader/backlog.rs` | Scan 999.x directories, parse names and contents |
| `BacklogView` | UI component | `src/ui/backlog_view.rs` | Scrollable list with preview panel |

**New data structures:**

```rust
pub struct BacklogItem {
    pub number: String,     // "999.3"
    pub slug: String,       // "queue-editor-and-reorder"
    pub display_name: String, // derived from slug: "Queue Editor and Reorder"
    pub has_concept: bool,
    pub concept_preview: Option<String>,  // first ~200 chars of CONCEPT.md
    pub dir_path: PathBuf,
}

pub struct BacklogViewState {
    pub items: Vec<BacklogItem>,
    pub selected: usize,
    pub show_preview: bool,
}
```

**New Action variants:**

```rust
Action::LoadBacklog { alias: String },
Action::BacklogLoaded { alias: String, items: Vec<BacklogItem> },
Action::PromoteBacklogItem { alias: String, item_number: String, target_phase: u32 },
```

**New InputMode variant:**

```rust
InputMode::BacklogBrowser { alias: String },
```

**Integration points:**
- `count_backlog_items()` already exists in state_reader; enhance to return full BacklogItem structs
- Detail view gains `b` keybinding for backlog browser
- Promotion writes to filesystem (rename directory) — must be careful about active GSD instances
- Backlog count already shown in detail view; make it clickable/navigable

### Feature 5: Execution Flow Graph

**What it does:** Per-phase visualization of the GSD workflow pipeline: discuss -> plan -> execute -> verify, showing which steps are complete based on file presence.

**Data source (verified on disk):**
Phase directories contain specific files indicating completion of each workflow step:
- `NN-DISCUSSION-LOG.md` -> discuss step done
- `NN-CONTEXT.md` -> context gathered
- `NN-RESEARCH.md` -> research done (optional)
- `NN-MM-PLAN.md` files -> plan step done (per plan)
- `NN-MM-SUMMARY.md` files -> execute step done (per plan)
- `NN-VERIFICATION.md` -> verify step done

**New components:**

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| `PhaseFlowReader` | Data reader | `src/state_reader/phase_flow.rs` | Scan phase directory for workflow artifacts |
| `FlowGraphWidget` | Custom widget | `src/ui/flow_graph.rs` | Render pipeline visualization |

**New data structures:**

```rust
pub struct PhaseFlowState {
    pub phase_number: String,
    pub phase_name: String,
    pub steps: Vec<FlowStep>,
}

pub struct FlowStep {
    pub name: String,           // "Discuss", "Plan", "Execute", "Verify"
    pub status: FlowStepStatus,
    pub artifacts: Vec<String>, // file names that prove completion
}

pub enum FlowStepStatus {
    NotStarted,
    InProgress,  // some artifacts present but not all expected
    Complete,
    Skipped,     // optional step that was bypassed
}
```

**New InputMode variant:**

```rust
InputMode::FlowGraph { alias: String, phase: String },
```

**Integration points:**
- Accessible from detail view's phase list — select a phase, press `Enter` or `f` to see flow
- Requires reading the actual phase directory, not just ROADMAP.md
- Phase directory paths: `.planning/phases/NN-slug/` (active) or `.planning/milestones/vX.X-phases/NN-slug/` (archived)
- Widget renders a horizontal pipeline with box-drawing characters:

```
  ┌─────────┐    ┌──────┐    ┌─────────┐    ┌────────┐
  │ Discuss  │───>│ Plan │───>│ Execute │───>│ Verify │
  │    [+]   │    │ [3/3]│    │  [2/3]  │    │  [ ]   │
  └─────────┘    └──────┘    └─────────┘    └────────┘
```

## State Changes to App Struct

```rust
pub struct App {
    // ... existing fields unchanged ...

    // NEW: Per-project extended state
    pub session_states: HashMap<String, Vec<ClaudeSession>>,
    pub git_history_states: HashMap<String, GitHistoryState>,
    pub backlog_states: HashMap<String, BacklogViewState>,
    pub phase_flow_states: HashMap<String, PhaseFlowState>,

    // NEW: Detail view sub-views expanded
    // (detail_sub_view_per_project HashMap values change type)
}
```

**DetailSubView expansion:**

```rust
pub enum DetailSubView {
    PhaseList,      // existing
    RoadmapViz,     // existing
    Sessions,       // NEW
    GitHistory,     // NEW
    BacklogBrowser, // NEW
    FlowGraph { phase: String }, // NEW
}
```

Alternatively, keep the existing DetailSubView for the main toggle and use InputMode variants for the new full-screen views. This keeps the existing `r` key toggle intact and adds new keybindings. **Recommended approach:** Use InputMode variants for new views because they are full-screen navigations, not sub-tabs of the detail view.

## Action Enum Growth Plan

Current: 5 variants. After v1.1: ~18 variants. This is within the "comfortable" range for a single match in update(). However, the update() method should be refactored into delegated handlers:

```rust
impl App {
    pub fn update(&mut self, action: Action) {
        match &action {
            Action::Tick | Action::Resize | Action::Noop => self.handle_lifecycle(action),
            Action::RawKey(_) => self.handle_key_action(action),
            Action::FileChanged { .. } => self.handle_file_change(action),
            Action::CreateProjectResult { .. } => self.handle_create_result(action),
            // v1.1 additions
            Action::LoadGitHistory { .. } | Action::GitHistoryLoaded { .. } => self.handle_git(action),
            Action::LoadBacklog { .. } | Action::BacklogLoaded { .. } | Action::PromoteBacklogItem { .. } => self.handle_backlog(action),
            Action::ExecuteQueueItem { .. } | Action::QueueExecutionResult { .. } | Action::RemoveQueueItem { .. } | Action::ReorderQueueItem { .. } => self.handle_queue(action),
            Action::RefreshClaudeSessions { .. } | Action::LaunchClaudeSession { .. } | Action::ResumeClaudeSession { .. } | Action::ClaudeSessionResult { .. } => self.handle_claude(action),
        }
    }
}
```

## Data Flow: New Patterns

### Claude Session Detection Flow

```
Navigation to Sessions view
    |
    v
App dispatches Action::RefreshClaudeSessions { project_path }
    |
    v
spawn_blocking: ClaudeSessionReader::scan(~/.claude/projects/<encoded-path>/)
    |  reads directory listing + first/last lines of each .jsonl
    |  checks /proc or `ps` for matching claude process
    v
Action::ClaudeSessionsLoaded sent via event_tx
    |
    v
App::update() stores sessions in session_states HashMap
    |
    v
Next render picks up session data
```

### Async Command Execution Flow (Queue + Session Launch)

```
User triggers execute (queue item or session launch)
    |
    v
App::update() spawns detached process:
    tokio::process::Command::new("claude")
        .args(&["-r", session_id])  // or queue command
        .current_dir(project_path)
        .spawn()  // detached, TUI does NOT wait
    |
    v
Status message: "Launched Claude session in terminal"
    |
    v
User switches to that terminal manually
```

**Important:** The TUI cannot embed a Claude session within itself. Claude Code is a full TUI application that takes over the terminal. Launch must either:
1. Open a new terminal window/tab (platform-dependent)
2. Print instructions to switch to the spawned terminal
3. Use tmux/screen split if available

**Recommended:** Spawn `claude` in a new tmux pane if tmux is detected, otherwise spawn in background and show the session ID for manual resume.

### Git History Loading Flow

```
Navigation to GitHistory view
    |
    v
App dispatches Action::LoadGitHistory { alias, scope }
    |
    v
spawn_blocking: git log --format="<structured>" --max-count=100 -- [.planning/]
    |
    v
Action::GitHistoryLoaded { alias, commits } sent via event_tx
    |
    v
App::update() stores in git_history_states
    |
    v
Render shows scrollable commit list
```

## Build Order (Dependency-Based)

| Order | Feature | Depends On | Rationale |
|-------|---------|------------|-----------|
| 1 | **State reader fixes** (999.4, 999.5, 999.6) | Nothing | Foundation — all other features read from corrected state |
| 2 | **Backlog browser** (999.11) | State reader fixes (uses phase directory scanning) | Low complexity, extends existing `count_backlog_items()`, validates phase directory reading pattern used by flow graph |
| 3 | **Git history viewer** (999.8) | Nothing (standalone) | Self-contained, no writes, introduces async command pattern reused by queue execution and session launch |
| 4 | **Queue editor/reorder** (999.3) | Nothing (extends existing queue_md) | Builds on existing EnqueueInput; adds interactive list pattern reused by backlog and session views |
| 5 | **Execution flow graph** (999.1) | Backlog browser (shares phase directory reading) | Needs phase directory scanning; more complex widget but isolated rendering |
| 6 | **GSD integration hooks** (999.7) | State reader fixes | Enhances state reader with cached status and fact/assumption tracking |
| 7 | **Queue execution** (999.9) | Queue editor, git history (reuses async spawn pattern) | Needs both queue management and process spawning |
| 8 | **Claude session management** (999.10) | Queue execution (reuses spawn pattern), GSD integration | Most complex; needs process detection, JSONL parsing, terminal management |

**Critical path:** State reader fixes -> Backlog browser -> Flow graph (these share phase directory reading code). Git history and queue editor can be built in parallel since they are independent.

## New Module Map

```
src/
├── main.rs
├── tui.rs
├── app.rs                       # MODIFY: add new state HashMaps, refactor update()
├── action.rs                    # MODIFY: add ~13 new Action variants
├── event.rs                     # UNCHANGED
├── config.rs                    # UNCHANGED
├── error.rs                     # UNCHANGED
├── cli.rs                       # UNCHANGED
├── registry.rs                  # UNCHANGED
├── change_tracker.rs            # UNCHANGED
├── project_creator.rs           # UNCHANGED
├── watcher.rs                   # UNCHANGED
├── state_reader/
│   ├── mod.rs                   # MODIFY: add claude_sessions, backlog, phase_flow, git_log
│   ├── state_md.rs              # MODIFY: fix plan counting, completion inference
│   ├── roadmap_md.rs            # MODIFY: fix parser bugs
│   ├── config_json.rs           # UNCHANGED
│   ├── queue_md.rs              # MODIFY: add QueueItemStatus, reorder/remove helpers
│   ├── claude_sessions.rs       # NEW: scan ~/.claude/projects/ for session JSONL
│   ├── backlog.rs               # NEW: parse 999.x directories into BacklogItem structs
│   ├── phase_flow.rs            # NEW: scan phase dirs for workflow artifacts
│   └── git_log.rs               # NEW: run git log, parse structured output
├── claude_detector.rs           # NEW: check running claude processes
├── queue_executor.rs            # NEW: spawn claude with queue command
├── ui/
│   ├── mod.rs                   # MODIFY: dispatch to new views
│   ├── project_list.rs          # MODIFY: add session indicator column
│   ├── detail_view.rs           # MODIFY: add keybindings for new sub-views
│   ├── roadmap_widget.rs        # UNCHANGED
│   ├── help_overlay.rs          # MODIFY: add new keybindings to help
│   ├── session_list.rs          # NEW: Claude session list with status
│   ├── git_history.rs           # NEW: scrollable git log
│   ├── backlog_view.rs          # NEW: backlog item list with preview
│   └── flow_graph.rs            # NEW: per-phase pipeline visualization widget
└── (tests remain in-module)
```

**Files changed:** 10 modified, 8 new
**Files unchanged:** 7

## Anti-Patterns to Avoid in v1.1

### Anti-Pattern: Scanning Claude Sessions on Every Tick

Session JSONL files can be megabytes. Scanning on every 250ms tick or file-watcher event would cause visible lag. Scan only on explicit user navigation to session view, with a 30-second cache TTL.

### Anti-Pattern: Blocking on Process Spawn

When launching `claude` for queue execution or session resume, never `wait()` on the child process. The TUI must remain responsive. Spawn and forget; show status via process detection on next session scan.

### Anti-Pattern: Direct Filesystem Mutation in update()

Queue reorder, backlog promotion, and QUEUE.md saves should use `spawn_blocking` with a result action, not synchronous fs writes in `update()`. The existing `save_queue()` is synchronous — acceptable for small files, but promotion (directory rename) can be slow on network filesystems.

### Anti-Pattern: Putting All New State in App

With 5 new features, App is at risk of becoming a god struct. Keep feature-specific state in dedicated state structs stored in HashMaps. App orchestrates; feature modules own their state shape.

## Sources

- Codebase analysis: `/home/blk/projects/rust/gsd-manager/src/` (all source files read directly)
- Claude Code session format: inspected `~/.claude/projects/-home-blk-projects-rust-gsd-manager/*.jsonl` (verified JSONL structure with sessionId, type, timestamp, cwd, gitBranch fields)
- Running process detection: verified via `ps aux | grep claude` showing process name and args including `--resume <session-id>`
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — `--resume`, `--session-id`, `-c`, `-p` flags (HIGH confidence, official docs)
- [claude-sessions-monitor](https://github.com/yepzdk/claude-sessions-monitor) — approach of scanning `~/.claude/projects/` JSONL files (MEDIUM confidence, third-party)
- [Session Files & Format - DeepWiki](https://deepwiki.com/affaan-m/everything-claude-code/7.1-session-files-and-format) — sessions-index.json schema reference (MEDIUM confidence)
- [Claude Code session management article](https://kentgigger.com/posts/claude-code-conversation-history) — directory structure and history.jsonl (MEDIUM confidence)
- GSD phase directory structure: verified via `ls .planning/milestones/v1.0-phases/01-core-infrastructure/` showing DISCUSSION-LOG, CONTEXT, PLAN, SUMMARY, VERIFICATION files
- [ratatui custom widget docs](https://ratatui.rs/recipes/widgets/custom/) — Widget trait implementation (HIGH confidence, official docs)
- [git2-rs](https://crates.io/crates/git2) — considered and rejected in favor of git CLI for read-only log viewing (HIGH confidence assessment)

---
*Architecture research for: v1.1 feature integration into existing TEA architecture*
*Researched: 2026-03-26*
