# Feature Landscape

**Domain:** GSD TUI meta-manager v1.1 power features
**Researched:** 2026-03-26
**Scope:** Claude session management, queue execution, git history viewer, backlog browser, execution flow graph, state reader fixes, GSD integration hooks
**Overall confidence:** MEDIUM-HIGH (verified against actual Claude CLI, GSD source code, and local filesystem; Claude session JSONL format is undocumented)

---

## Table Stakes

Features v1.1 must deliver. Missing any = release feels half-baked.

| Feature | Why Expected | Complexity | Depends On | Notes |
|---------|--------------|------------|------------|-------|
| Fix state reader plan counting (999.4) | Dashboard shows wrong progress — users notice immediately and lose trust | Low | Existing `roadmap_md.rs` | Regex misses standalone `PLAN.md` and indented plan lists |
| Fix P5:Unknown on completed milestones (999.5) | Visible bug for any finished milestone | Low | Existing `state_reader/mod.rs` | When `completed_phases == total_phases`, code computes phase N+1 which doesn't exist |
| Disk-based phase completion inference (999.6) | ROADMAP.md checkboxes are often stale; disk files are ground truth | Med | state_reader refactor | Replicate GSD's own disk_status algorithm from `roadmap.cjs` |
| GSD integration hooks (999.7) | Users need "fact vs assumption" distinction — is status verified or just inferred from files? | Med | state_reader, async subprocess | Optional enrichment via `gsd-tools.cjs` JSON output |
| Backlog browser (999.11) | Dashboard already shows backlog count; users expect to drill in and see items | Med | Existing detail_view | Parse `999.*` directories, display in scrollable list |

## Differentiators

Features that make v1.1 a meaningful upgrade. High value, not universally expected.

| Feature | Value Proposition | Complexity | Depends On | Notes |
|---------|-------------------|------------|------------|-------|
| Claude session management (999.10) | See which projects have active Claude sessions, resume/launch from TUI — the killer feature that justifies the meta-manager over terminal tabs | High | Claude CLI, process detection, terminal spawning | Highest value, highest risk |
| Queue execution (999.9) | Turn QUEUE.md from passive reminder into actionable launcher — execute GSD commands from the TUI | High | Queue parser (exists), terminal spawning | Shares terminal-spawning infra with session management |
| Execution flow graph (999.1) | Per-phase pipeline visualization (discuss/research/plan/execute/verify) — makes GSD workflow visible and navigable | High | disk_status inference (999.6) | Custom ratatui Widget, maps files to pipeline stages |
| Git history viewer (999.8) | Scrollable git log scoped to repo or `.planning/` — see what changed without leaving TUI | Med | git CLI subprocess | Modeled after gitui/lazygit commit list UX |

## Anti-Features

Features to explicitly NOT build in v1.1.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Embedded Claude terminal inside TUI | Terminal-in-terminal is a UX disaster: keypress conflicts, rendering glitches, impossible scrollback. Every TUI that tries this regrets it. | Launch Claude in a separate tmux pane or terminal tab; show session status in the TUI |
| Real-time streaming of Claude output | Requires parsing Claude's ANSI output stream, rate-limiting renders — massive complexity for marginal value | Show session metadata (active/idle/completed, last activity timestamp, duration) instead |
| Full text editor for QUEUE.md | Text editing in TUI is a solved-but-painful problem; users already have preferred editors | Support add/remove/reorder operations on queue items; `$EDITOR` for complex edits |
| Auto-executing queued commands without confirmation | Dangerous: queued items may be stale, context-dependent, or destructive | Always show confirmation dialog before execution |
| Git commit/push/branch from TUI | Scope creep into gitui/lazygit territory; those tools exist and excel | Read-only git history; launch gitui/lazygit for mutations |
| Plugin system / extensibility | Architecture still stabilizing; premature abstraction creates maintenance burden | Defer to v2+ per PROJECT.md |
| Backlog promotion (creating phases) | Needs deeper GSD integration research; phase numbering and roadmap editing is complex | Read-only in v1.1; promote by enqueueing `/gsd:add-phase` command |

---

## Feature Details

### 1. Fix State Reader Accuracy (Table Stakes, Low-Med)

Three parser bugs that undermine dashboard reliability.

**999.4 — Plan counting:** The `roadmap_md.rs` regex `^\s*- \[([ xX])\] \d+-\d+-PLAN\.md` requires format `NN-NN-PLAN.md`. GSD also uses standalone `PLAN.md` files and the plan regex doesn't account for all indentation patterns. Fix: broaden regex, add `PLAN.md` standalone match.

**999.5 — P5:Unknown:** In `state_reader/mod.rs`, when `completed_phases == total_phases`, the code computes `current_phase = format!("Phase {}", completed_phases + 1)`. That phase doesn't exist. Fix: when all phases complete, set `current_phase` to "Complete" or the milestone name.

**999.6 — Disk-based inference:** Replicate GSD's disk_status algorithm (verified from `roadmap.cjs` lines 127-153):

| disk_status | Detection |
|-------------|-----------|
| `no_directory` | Phase directory doesn't exist in `.planning/phases/` |
| `empty` | Directory exists but no CONTEXT/RESEARCH/PLAN/SUMMARY files |
| `discussed` | `CONTEXT.md` or `*-CONTEXT.md` present |
| `researched` | `RESEARCH.md` or `*-RESEARCH.md` present |
| `planned` | `PLAN.md` or `*-PLAN.md` present, no SUMMARY files |
| `partial` | Some SUMMARY files but count < PLAN count |
| `complete` | SUMMARY count >= PLAN count (all plans executed) |

This is the foundation the execution flow graph depends on.

### 2. GSD Integration Hooks (Table Stakes, Med)

Optional enrichment of file-based state with authoritative GSD tool JSON output.

**Integration points (verified from `gsd-tools.cjs` source):**
- `gsd-tools.cjs state json` — STATE.md frontmatter as structured JSON
- `gsd-tools.cjs roadmap analyze` — full roadmap with disk_status per phase, plan/summary counts, dependency resolution
- `gsd-tools.cjs progress json` — formatted progress data

**Design constraint:** Must be opt-in and async. The TUI must work perfectly with file-only reading (project constraint: "must not require running Claude/GSD to check status"). GSD enrichment is a "trust boost" overlay.

**Implementation:**
- `tokio::process::Command` to spawn gsd-tools, capture stdout JSON
- Deserialize into enriched state struct
- Cache per-project with TTL, invalidate when file watcher fires
- Show `[verified]` vs `[inferred]` badges on status fields
- Graceful degradation: if gsd-tools.cjs not found or errors, silently fall back to file-only

### 3. Claude Session Management (Differentiator, High)

Detect, display, resume, and launch Claude Code sessions for registered projects.

**Detection approach (verified against Claude CLI v2.1.84 and local filesystem):**

**Session file discovery:**
- Path: `~/.claude/projects/{encoded-cwd}/{uuid}.jsonl`
- Encoded CWD format: absolute path with non-alphanumeric chars replaced by `-` (e.g., `/home/blk/projects/rust/gsd-manager` becomes `-home-blk-projects-rust-gsd-manager`)
- Each `.jsonl` file is one session

**Session metadata extraction (from JSONL tail):**
- `type` field: `user`, `assistant`, `progress`, `file-history-snapshot`
- `sessionId` field: UUID of the session
- `timestamp` field: when the message was recorded
- `message.content` for user messages: check for `/gsd:` prefix to identify GSD workflow sessions
- Session name/slug: stored in records, used by `claude --resume`

**Active session detection:**
- Process check: scan `/proc/*/cmdline` for `claude` processes whose CWD matches project path
- IDE lock files: `~/.claude/ide/*.lock` — JSON with `{pid, workspaceFolders, ideName}` — check if PID is alive via `kill -0`
- Heuristic: if most recent JSONL was modified within the last 60 seconds, session is likely active

**Launch capabilities:**
- `claude --resume {session-id}` — resume specific session in new terminal
- `claude --continue` — continue most recent session for project dir
- `claude -p "/gsd:resume-work"` — headless: new GSD session that auto-resumes
- Terminal spawning: `tmux new-window -c {project_path} "claude --resume {id}"` or `$TERMINAL -e "cd {path} && claude --continue"`

**UX design (following lazygit's always-visible-panes pattern):**
- New sub-view tab in detail view: `PhaseList | RoadmapViz | Sessions | FlowGraph | GitLog`
- Session list showing: name/slug, last activity (relative time), status icon (active/recent/stale), GSD workflow type
- Active session indicator on dashboard project row (a small dot or icon in status column)
- Keybindings: `Enter` to resume in new terminal, `n` to start new session, `c` to continue latest

**Confidence:** MEDIUM — Session JSONL structure verified from actual files, but it's undocumented and could change between Claude Code versions. Build defensively with graceful fallback. Process detection is platform-specific (Linux `/proc` vs macOS `ps`).

### 4. Queue Execution (Differentiator, High)

Make QUEUE.md items actionable by executing them in Claude sessions.

**Current state:** Queue is append-only list of strings like `/gsd:plan-phase 4`. The `QueuedAction` struct only has a `command` field. No execution.

**Enhanced QueuedAction struct:**
```rust
pub struct QueuedAction {
    pub command: String,
    pub status: QueueStatus,       // Pending | InProgress | Done | Failed
    pub enqueued_at: Option<String>, // ISO timestamp
    pub started_at: Option<String>,
    pub session_id: Option<String>,  // Claude session UUID when executed
}
```

**Execution flow:**
1. User selects queued item, presses `x` (execute)
2. TUI shows confirmation dialog: "Execute `/gsd:plan-phase 4` in project X? [y/n]"
3. On confirm, determine execution mode:
   - **Interactive:** `tmux new-window -c {path} "claude"` then the user types the command (safest)
   - **Headless:** `claude -p "{command}" --output-format json` in project dir (for commands that don't need interaction)
   - **Session-attached:** If an active session exists, show option to queue it for that session
4. Update queue item status to `in-progress`
5. File watcher detects `.planning/` changes when GSD completes -> dashboard updates
6. Mark item `done` or `failed` based on outcome (or manual user action)

**Queue view UX:**
- Sub-view in detail view alongside phases, sessions, etc.
- Vim navigation: `j`/`k` to move, `x` to execute, `d` to delete, `e` to edit, `J`/`K` to reorder
- Color coding: pending (white), in-progress (yellow), done (green), failed (red)
- Bottom bar shows suggested next commands from `suggest_next_commands()` (already implemented)
- `a` to add new item (opens inline input, same as current enqueue modal)

**Shared infrastructure with session management:** Both features need terminal spawning. Build a `terminal_launcher` module that handles tmux detection, fallback to `$TERMINAL`, and process lifecycle tracking. Implement this once, use it for both queue execution and session launching.

### 5. Execution Flow Graph (Differentiator, High)

Per-phase pipeline visualization of the GSD workflow stages.

**Pipeline stages (verified from GSD source code `init.cjs` lines 956-985):**
```
[Discuss] -> [Research] -> [Plan] -> [Execute] -> [Verify]
```

**Stage-to-file mapping (from `roadmap.cjs` disk_status algorithm):**

| Stage | Detected By | Notes |
|-------|-------------|-------|
| Not started | No directory or empty directory | Gray box |
| Discuss | CONTEXT.md exists | discuss-phase creates this |
| Research | RESEARCH.md exists | Optional; some phases skip |
| Plan | PLAN.md files exist | plan-phase creates these |
| Execute | SUMMARY.md files exist (partial or complete) | execute-phase creates these |
| Verify | VERIFICATION.md or UAT.md exists | verify-work creates these |

**Visualization (ASCII in terminal):**
```
Phase 3: Live State
  [DISC] --> [RSCH] --> [PLAN] --> [EXEC] --> [VRFY]
    ok        ok        ok       2/3 done      --

Phase 4: Visualization
  [DISC] --> [RSCH] --> [PLAN] --> [EXEC] --> [VRFY]
    ok       skip      >>> NOW       --         --
```

**Implementation approach:**
- Custom `Widget` implementation (per CLAUDE.md: "implement custom ratatui Widget trait" for graph rendering)
- Each stage is a small bordered cell with status text/color
- Connecting arrows `-->` between stages
- Color scheme: dim gray (not started), yellow/bold (current), green (complete), dark gray (skipped)
- Show plan progress as fraction (e.g., "2/3") in the Execute stage

**View modes:**
- **Stacked view:** All phases shown vertically, scrollable (using ratatui's built-in List or tui-widget-list)
- **Single-phase view:** One phase at a time, up/down to switch between phases
- **Dashboard compact:** Single-line pipeline indicator per project on the main list (e.g., `D-R-P-E-V` with colors)

**Complexity notes:** The widget rendering is the hard part. Each phase row needs 5 boxes + 4 arrows, with dynamic widths based on terminal size. Use `Constraint::Ratio` or `Constraint::Min` for responsive layout. The data model is straightforward once disk_status inference works.

### 6. Git History Viewer (Differentiator, Med)

Scrollable git log scoped to the whole repo or `.planning/` directory.

**Approach:** Shell out to `git log` via `tokio::process::Command`:
```bash
# Full repo history
git -C {project_path} log --oneline --format="%h|%ad|%an|%s" --date=short -n 100

# .planning/ scoped
git -C {project_path} log --oneline --format="%h|%ad|%an|%s" --date=short -n 100 -- .planning/
```

**Why `git` CLI over `git2` crate:** The git2 crate links libgit2 (C library), adds significant compile time, and introduces a native dependency. The `git` CLI is guaranteed available for any GSD user (GSD requires git). Cost/benefit strongly favors `Command::new("git")`.

**UX design (modeled after gitui/lazygit):**
- New sub-view tab in detail view
- Two scopes toggled by keybinding: `a` for all commits, `p` for `.planning/` only
- Columnar display: hash (dim cyan), date (dim), author (dim), message (default)
- Vim navigation: `j`/`k` to scroll, `G`/`gg` for top/bottom
- `Enter` on a commit shows diff stat: `git show --stat {hash}` in a popup or split pane
- Lazy loading: fetch first 50 commits, load more on scroll (avoids blocking on large repos)

**Parse format:** Use `|` delimiter in `--format` string, split in Rust. Handle edge cases: commit messages containing `|` (use last-occurrence split for the message field).

### 7. Backlog Browser (Table Stakes, Med)

View and browse backlog items from within the TUI.

**Data source:** Parse `999.*` directories in `.planning/phases/`:
- Directory pattern: `999.N-slug-name` (e.g., `999.8-git-history-viewer`)
- Parse: split on first `-` after `999.N` to get number and slug
- Content: check for any `.md` files inside the directory for description text
- Humanize slug: replace `-` with spaces, title case

**UX design:**
- Sub-view in detail view, or section at the bottom of PhaseList
- Scrollable list: item number, humanized name, file count
- `Enter` to view item details (show any .md file contents in a read-only panel)
- `q` to queue promotion: adds `/gsd:add-phase {description}` to QUEUE.md
- Sort by item number (999.1, 999.2, etc.)

**Edge cases:**
- Empty directories (as in this project currently) — show name only, no description
- Malformed directory names (e.g., `999.1-{\n "slug": ...}` — yes, this exists in the current project) — handle gracefully, extract what we can
- No `.planning/phases/` directory — show "No backlog items" message

---

## Feature Dependencies

```
Fix plan counting (999.4) --------+
Fix P5:Unknown (999.5) ----------+---> Accurate state reader (foundation)
Disk-based inference (999.6) ----+
                                  |
                                  v
                         GSD integration hooks (999.7)
                                  |
                                  v
                         Execution flow graph (999.1)
                         (needs disk_status per phase for pipeline stages)

Terminal spawning module --------+---> Queue execution (999.9)
(shared infrastructure)          |
                                 +---> Claude session management (999.10)
                                       (resume/launch needs terminal spawning)

Backlog browser (999.11) ------------ independent, no blockers
Git history viewer (999.8) ---------- independent, no blockers
```

## MVP Recommendation

### Phase 1 — Fix the foundation (bugs + state accuracy)
Prioritize because everything else depends on accurate state reading.

1. **Fix plan counting regex** (999.4) — Low effort, high trust impact
2. **Fix P5:Unknown bug** (999.5) — Low effort, visible improvement
3. **Disk-based phase completion inference** (999.6) — Medium effort, foundation for flow graph

### Phase 2 — New read-only views (low risk, high utility)
Independent features that add value without complex infrastructure.

4. **Backlog browser** (999.11) — Simplest new feature, data already partially parsed
5. **Git history viewer** (999.8) — Medium complexity, high utility, git CLI is simple
6. **Execution flow graph** (999.1) — Depends on disk inference from phase 1; custom widget work

### Phase 3 — GSD integration + execution (high complexity, high value)
Requires terminal spawning infrastructure and careful error handling.

7. **GSD integration hooks** (999.7) — Enriches all views with verified data
8. **Queue execution** (999.9) — Needs terminal spawning module
9. **Claude session management** (999.10) — Highest complexity + highest value; reuses terminal spawning from 999.9

### Defer to v1.2
- Queue editor and reorder (999.3) — nice-to-have, not blocking v1.1
- Backlog promotion (phase creation from backlog) — needs deeper GSD integration
- Milestone plan editor with Claude launch (999.2) — blocked on session management maturity

---

## Sources

- Claude CLI v2.1.84 `--help` output — session flags: `--resume`, `--continue`, `--name`, `--session-id`, `--worktree`, `--tmux` (HIGH confidence, verified locally)
- `~/.claude/projects/-home-blk-projects-rust-gsd-manager/` — session JSONL files with `type`, `sessionId`, `timestamp`, `message.content` fields (HIGH confidence, verified locally)
- `~/.claude/ide/36473.lock` — lock file structure: `{pid, workspaceFolders, ideName, transport, authToken}` (HIGH confidence, verified locally)
- GSD `roadmap.cjs` lines 127-177 — disk_status algorithm: no_directory/empty/discussed/researched/planned/partial/complete (HIGH confidence, read from source)
- GSD `init.cjs` lines 956-985 — phase lifecycle: discuss/research/plan/execute/verify with dependency-aware recommendations (HIGH confidence, read from source)
- GSD `gsd-tools.cjs` lines 1-80 — available CLI commands for integration (HIGH confidence, read from source)
- Existing `queue_md.rs` — current queue parser with `QueuedAction`, `suggest_next_commands` (HIGH confidence, read from source)
- Existing `roadmap_md.rs` — current roadmap parser with plan counting regex (HIGH confidence, read from source)
- Existing `state_reader/mod.rs` — current state parsing with ProjectState struct (HIGH confidence, read from source)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — session management documentation (MEDIUM confidence, WebSearch)
- [Claude Code session file format](https://databunny.medium.com/inside-claude-code-the-session-file-format-and-how-to-inspect-it-b9998e66d56b) — JSONL structure: parentUuid, sessionId, slug for session chaining (MEDIUM confidence, single source blog post)
- [gitui](https://github.com/gitui-org/gitui) — Rust TUI git client; UX patterns for commit list, keybindings (HIGH confidence, well-known project)
- [lazygit](https://github.com/jesseduffield/lazygit) — TUI UX patterns: always-visible panes, consistent keybinding model, scope toggling (HIGH confidence, well-known project)
- [tui-widget-list](https://github.com/preiter93/tui-widget-list) — scrollable list widget for ratatui with padding and infinite scrolling (HIGH confidence, crates.io)
- [tui-scrollview](https://github.com/joshka/tui-scrollview) — generic scrollable view widget for ratatui (HIGH confidence, crates.io)
