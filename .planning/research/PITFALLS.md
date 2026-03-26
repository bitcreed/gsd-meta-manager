# Pitfalls Research

**Domain:** Adding process management, git integration, file editing, and graph visualization to an existing Rust TUI (ratatui) application
**Researched:** 2026-03-26
**Confidence:** HIGH (based on codebase analysis, official docs, and community patterns)

## Critical Pitfalls

### Pitfall 1: InputMode Enum Explosion Collapses the App Architecture

**What goes wrong:**
The current `InputMode` enum has 11 variants. Adding Claude session management, backlog browser, git history viewer, queue execution, and execution flow graph could push this to 20+ variants. Every new feature adds 2-3 modes (viewing, editing, confirming). The `handle_key` match arm becomes a 1000+ line function. Key binding conflicts emerge across modes. Testing becomes combinatorial.

**Why it happens:**
v1.0 grew organically from 3 modes to 11 in two days. The flat enum pattern works for small apps but collapses when every feature needs its own input handling. The `App` struct already holds 18 fields -- each new feature adds more per-feature state (scroll offsets, selection indices, buffers).

**How to avoid:**
Refactor to a screen/component architecture before adding new features. Each feature (git history, backlog browser, etc.) gets its own struct implementing a `Screen` trait with `handle_key()`, `render()`, and `update()` methods. The `App` holds a `Vec<Box<dyn Screen>>` stack. Push screens on entry, pop on Escape. This is the ratatui community's recommended pattern for apps with 5+ screens.

**Warning signs:**
- `handle_key` exceeds 200 lines
- Adding a keybinding requires checking 5+ other modes for conflicts
- New features require adding fields to `App` that are only used in one mode
- `.clone()` on `InputMode` in match arms (already happening at line 363 of app.rs)

**Phase to address:**
Phase 1 -- refactor architecture BEFORE adding any new screens. This is the single highest-leverage change for v1.1.

---

### Pitfall 2: Terminal State Corruption When Spawning Claude Sessions

**What goes wrong:**
Spawning a Claude CLI process (`claude --resume <id>`) from inside a ratatui app requires leaving alternate screen and disabling raw mode first. If the spawn fails, panics, or the child process is killed, the terminal stays in a broken state -- no cursor, garbled input, alternate screen stuck. Users must `reset` their terminal.

**Why it happens:**
ratatui owns the terminal (alternate screen + raw mode). Spawning an interactive child process (Claude CLI) that also needs terminal control creates a double-ownership problem. The `ratatui::restore()` / `ratatui::init()` dance must be airtight, including on all error paths. The ratatui docs have an explicit recipe for this (spawning vim), but it is easy to miss error paths.

**How to avoid:**
1. Wrap terminal handoff in a RAII guard: `struct TerminalHandoff` that calls `ratatui::restore()` on creation and `ratatui::init()` on Drop.
2. Use `std::panic::catch_unwind` around the spawn to ensure restoration.
3. For Claude specifically: prefer `claude -p "command" --print` (non-interactive output mode) for status checks and queue execution, and only do full terminal handoff for interactive attach.
4. Set `kill_on_drop(true)` on spawned `tokio::process::Command` to prevent orphan processes when the TUI is the parent.

**Warning signs:**
- Manual `restore()` / `init()` calls without error handling between them
- Testing only the happy path (successful spawn + graceful exit)
- Forgetting to restore on `Ctrl+C` during child process execution

**Phase to address:**
Queue execution and Claude session management phases. Build the terminal handoff abstraction first, then use it for both features.

---

### Pitfall 3: Synchronous File I/O Blocking the Render Loop Gets Worse

**What goes wrong:**
The current `parse_project_state()` does synchronous `std::fs::read_to_string()` on the main thread (called from `load_project_states()` and the `FileChanged` handler in `app.rs` line 248-251). With v1.1 adding git log parsing (`git log` can take 100ms+ on large repos), backlog file reading (scanning all `999.*` directories and reading their content), and disk-based phase completion inference (stat-ing multiple files per phase), a single `FileChanged` event could block rendering for 500ms+.

**Why it happens:**
v1.0's synchronous approach was pragmatic for parsing 3 small files. But git history shells out to `git log` (which can be slow on large repos), disk-based inference requires stat-ing N * M files (phases times expected artifacts), and backlog browsing reads N markdown files. All of these currently run in the event handler on the main thread.

**How to avoid:**
Move all file I/O and `git` invocations to `tokio::task::spawn_blocking()`. Send results back through the event bus as a new `Action` variant like `Action::ProjectStateUpdated { alias, state }`. The render loop stays responsive. This pattern already exists in the codebase -- `CreateProjectResult` is dispatched from a `spawn_blocking` task (app.rs line 820).

For git: use `tokio::process::Command` for `git log --format=<custom> -n 50` rather than the git2 crate (avoids C FFI dependency, matches the single-binary distribution constraint).

**Warning signs:**
- UI freezes when a project with a large git history is selected
- Typing feels laggy after adding backlog browsing
- Multiple `FileChanged` events queue up and process one-by-one, causing a visible "catching up" delay

**Phase to address:**
State reader accuracy fix phase (earliest v1.1 phase). Establish the async I/O pattern before git history and backlog features are built on top.

---

### Pitfall 4: Zombie and Orphan Processes from Queue Execution

**What goes wrong:**
Queue execution spawns Claude sessions (`claude -p "command" --cwd /path`) that can run for minutes to hours. If the TUI exits (quit, crash, SIGTERM), child processes either become orphans (still running but untracked) or zombies (exited but not reaped). If child handles are dropped without awaiting, tokio attempts best-effort reaping but makes no guarantees. If the TUI spawns multiple queue items concurrently, resource exhaustion is possible since Claude sessions are memory-heavy (each is a Node.js process with 200MB+ RSS).

**Why it happens:**
Tokio's `Child` handle does not kill the child on drop by default -- the process continues running. The TUI has no graceful shutdown sequence that waits for or detaches children. Users expect to quit the TUI without killing their long-running Claude sessions, but the TUI also needs to track them for status display.

**How to avoid:**
1. Build a `ProcessManager` that tracks all spawned children with their PIDs, start times, and associated project/queue-item.
2. On TUI exit, give users a choice: "2 Claude sessions running. Detach (keep running) or Kill?"
3. Use `kill_on_drop(false)` intentionally for Claude sessions that should survive TUI restart.
4. Store PIDs in a file (`~/.local/share/gsd-manager/sessions.json`) so the TUI can re-discover them on restart via `kill(pid, 0)` (signal 0 = existence check).
5. Limit concurrent executions (max 2-3 Claude sessions) with a `tokio::sync::Semaphore`.
6. Ensure `child.wait()` is always awaited for processes the TUI explicitly kills, to prevent zombies.

**Warning signs:**
- `ps aux | grep claude` shows dozens of orphan processes after TUI crashes
- System memory climbs after repeated queue executions without explicit cleanup
- PID file has stale entries that never get cleaned up

**Phase to address:**
Queue execution phase. This is the core complexity of making QUEUE.md actionable.

---

### Pitfall 5: Markdown Round-Trip Corruption in Backlog and Queue Editing

**What goes wrong:**
The backlog browser needs to read, edit, and write back `.planning/` markdown files (QUEUE.md, backlog item descriptions, potentially ROADMAP.md). Naive parse-modify-serialize loses formatting: extra blank lines vanish, indentation changes, frontmatter (the YAML-like header in STATE.md) gets mangled, emoji/Unicode in phase names breaks, and list markers change style. GSD reads these files too -- if the TUI's writes produce subtly different formatting, GSD may parse them differently or show unexpected diffs.

**Why it happens:**
Markdown has no canonical serialization. The same semantic content can be formatted many ways. Most markdown parsers produce an AST that loses whitespace details. The current `queue_md` module does line-by-line string manipulation and reconstruction -- workable for a single file format, but fragile when extended to editing arbitrary markdown sections.

**How to avoid:**
1. For structured files (QUEUE.md, STATE.md frontmatter): continue the line-by-line approach. Parse only the specific section being edited. Preserve everything else byte-for-byte by tracking byte offsets of the edited section.
2. For backlog items: treat the TUI as "insert/append only." The TUI can add new items and mark items as promoted, but full editing should launch `$EDITOR` (using the terminal handoff pattern from Pitfall 2).
3. Write integration tests that round-trip real GSD files: `read -> parse -> serialize -> diff` must produce zero changes when no edits were made.
4. Never parse STATE.md's frontmatter with a full YAML/TOML parser for write-back -- use regex replacement on the specific field being updated.

**Warning signs:**
- `git diff` shows whitespace-only changes in files the TUI "didn't edit"
- GSD fails to parse a file after the TUI touched it
- Tests pass on synthetic files but fail on real project files with Unicode or unusual formatting

**Phase to address:**
Backlog browser phase. Build round-trip tests before implementing any write operations.

---

### Pitfall 6: Claude Session Detection is Inherently Fragile

**What goes wrong:**
Detecting running Claude sessions via `pgrep -x claude` is unreliable: the Claude CLI process name is actually `node` (it is a Node.js application), multiple Claude instances may exist for different projects, and the process tree may include wrapper scripts. Matching PIDs to projects requires heuristics (checking `/proc/<pid>/cwd` or parsing command-line args from `/proc/<pid>/cmdline`) that break across OS updates, Claude CLI version changes, or when Claude is run through tmux/screen.

**Why it happens:**
Claude Code has no official API for external process discovery. The CLI stores sessions in `~/.claude/projects/<path-hash>/` as `.jsonl` files, but there is no lock file or PID file indicating "this session is currently active." Any detection method is reverse-engineering an undocumented internal. The session file structure may change between Claude CLI versions.

**How to avoid:**
1. Accept that detection is best-effort. Show "possibly active" with a visual confidence indicator, never "definitely running."
2. Primary signal: check if `~/.claude/projects/<project-path-hash>/` has `.jsonl` files modified in the last 60 seconds. File modification time is more reliable than process detection.
3. Secondary signal: `pgrep -f "claude.*<project-path>"` to find Claude processes associated with a specific project directory.
4. For attaching: do not try to attach to a running terminal session programmatically. Instead, use `claude --resume <session-id> --cwd <path>` which creates a new terminal session that continues the conversation. This is the officially supported mechanism.
5. Build an abstraction layer (`trait SessionDetector`) so the detection method can be swapped when Claude adds official tooling (likely).

**Warning signs:**
- Detection works on the developer's machine but fails for other users (different shell, different Claude install method)
- False positives when multiple projects are active
- Hardcoded assumptions about Claude CLI internals (process names, file paths, hashing algorithms)

**Phase to address:**
Claude session management phase. Mark this as a "best-effort feature" in the milestone plan, not a hard requirement.

---

### Pitfall 7: DAG Rendering That Fails at Real Terminal Sizes

**What goes wrong:**
The execution flow graph (discuss -> plan -> execute -> verify pipeline per phase) looks great at 120x40 terminal but is illegible at 80x24 (a common minimum). Node labels overlap, edges cross through boxes, and the layout algorithm picks a placement that wastes space or clips content. The current `RoadmapWidget` (a simple vertical stack) does not generalize to a multi-column DAG.

**Why it happens:**
DAG layout is a fundamentally hard problem. The Sugiyama layered layout algorithm handles most cases but has pathological performance on "fan" topologies (one node connecting to many). Terminal rendering adds constraints: fixed-width characters, no sub-pixel positioning, and box-drawing characters that require exact alignment. Developers prototype with a 5-node graph and declare success, then the 15-phase project breaks the layout.

**How to avoid:**
1. For the execution flow graph specifically (discuss/plan/execute/verify per phase), this is NOT a general DAG. It is a fixed 4-stage pipeline. Render it as a horizontal 4-column layout, not a general graph. This sidesteps the entire DAG layout problem.
2. If a general DAG is needed later (cross-phase dependencies), consider the `ascii-dag` crate which implements Sugiyama layout for terminals, or implement a simplified version for the limited topology.
3. Detect terminal size and gracefully degrade: below 100 columns, switch from graphical boxes to a compact list view with indented stages.
4. Set a maximum number of simultaneously visible nodes (e.g., 8 phases) with scrolling for the rest.

**Warning signs:**
- Graph looks correct only at the developer's terminal size
- Edge routing breaks when two phases have the same pipeline stage active
- No graceful fallback for small terminals -- just garbled output
- Spending more than 2 days on layout algorithm for a 4-stage pipeline

**Phase to address:**
Execution flow graph phase. Start with the simple 4-column pipeline; do not build a general DAG renderer unless actually needed.

---

### Pitfall 8: File Watcher Self-Triggering on TUI Writes

**What goes wrong:**
The TUI writes to QUEUE.md (enqueue action, line 582 of app.rs) and potentially to backlog files. The file watcher detects these writes and fires a `FileChanged` event. The TUI re-parses the file it just wrote, which is wasted work. Worse, if the write and re-parse have slightly different timing, the TUI could read its own partially-written file and display incorrect state momentarily.

**Why it happens:**
The current `notify` watcher watches the entire `.planning/` directory recursively. It cannot distinguish between writes from the TUI and writes from GSD or other tools. The 200ms debounce helps but does not eliminate the issue.

**How to avoid:**
1. Set a per-project "self-write" timestamp. When the TUI writes a file, record `(project_alias, Instant::now())`. In the `FileChanged` handler, if a self-write happened within the last 500ms for that project, skip the re-parse (the TUI already has the correct state).
2. After the self-write cooldown expires, do re-parse once to pick up any external changes that occurred during the window.
3. For queue execution: this is especially important because Claude sessions may write to `.planning/` files while the TUI is also writing queue status updates.

**Warning signs:**
- `tracing` logs show redundant re-parses immediately after TUI writes
- Flicker in the UI after enqueueing an action (brief "old state" then "new state")
- In extreme cases, infinite loop: write triggers parse, parse triggers UI update, UI update triggers write

**Phase to address:**
State reader fix phase, because this affects the foundation that all other features build on.

---

### Pitfall 9: Git Log Output Parsing Breaks on Edge Cases

**What goes wrong:**
Parsing `git log` output with the default format is locale-dependent (date formats change), encoding-dependent (non-UTF8 commit messages), and ambiguous (commit messages can contain any characters including format separators). Using `--format=%H %s` and splitting on space breaks when the subject contains the separator. Merge commits, empty commits, and detached HEAD states produce unexpected output.

**Why it happens:**
Developers test with their own clean git history, which has short ASCII commit messages and linear history. Real projects have merge commits, rebase artifacts, co-author trailers, signed commits, and multi-line subjects.

**How to avoid:**
1. Use `git log --format=%x00%H%x00%an%x00%aI%x00%s` with null byte (`%x00`) separators. Null bytes cannot appear in any git field, making parsing unambiguous.
2. Use `--no-merges` for the initial view; add merge commit display as an enhancement later.
3. Handle `git log` returning non-zero exit code (empty repo, detached HEAD, corrupted repo) by showing "Git history unavailable" rather than crashing.
4. Limit output with `-n 100` and implement pagination rather than loading entire history.
5. Use `--no-walk` with specific refs when scoping to `.planning/` changes: `git log -- .planning/`.

**Warning signs:**
- Git history shows garbled entries after a merge commit
- Non-ASCII author names or commit messages cause panics in the parser
- Empty repos (no commits yet) crash the git history view

**Phase to address:**
Git history viewer phase. Build the parser with null-byte separators from the start.

---

### Pitfall 10: Disk-Based Phase Completion Inference Has Ambiguous Signals

**What goes wrong:**
Inferring phase completion from disk artifacts (presence of PLAN files, execution markers, verify results) produces false positives and false negatives. A phase directory might have plan files but no execution marker because execution is in progress. Archived phases (v1.0 phases were moved to `.planning/archive/`) create ghost signals. Phases with `autonomous: false` plans may be partially complete but waiting for human verification.

**Why it happens:**
GSD's `.planning/` directory structure is an implicit state machine, not an explicit one. STATE.md has the authoritative progress counts, but they may be stale (written at session boundaries, not continuously). The disk-based approach tries to derive truth from artifacts, but the mapping from "files that exist" to "phase completion status" has many edge cases that depend on GSD workflow conventions that may evolve.

**How to avoid:**
1. Make STATE.md the primary source of truth for phase counts. Only use disk-based inference as a fallback when STATE.md data is missing or clearly stale.
2. Define explicit rules with confidence levels: "PLAN.md exists + all plans have `-DONE` markers = HIGH confidence complete", "PLAN.md exists + some done markers = MEDIUM confidence in-progress", "no PLAN.md = LOW confidence not-started."
3. Display the confidence level to the user (fact vs. inference distinction mentioned in PROJECT.md).
4. Write the inference rules as a separate, testable module with fixtures from real GSD projects (including the current project's own `.planning/` history).
5. Handle archived phases: check `.planning/archive/` in addition to `.planning/phases/` for completed milestone phases.

**Warning signs:**
- Dashboard shows Phase 3 as "complete" when it is still executing
- Archived phases from previous milestones appear as current work
- Phase count disagrees between STATE.md and disk inference without any explanation

**Phase to address:**
State reader accuracy fix phase (first v1.1 phase). This is the foundational fix that the PROJECT.md explicitly calls out.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keep flat `InputMode` enum, add more variants | Fast to implement new screens | Unmaintainable past 15 variants; key conflicts; untestable combinatorial state | Never for v1.1 -- refactor first |
| Sync `git log` in event handler | Simple implementation | UI freeze on large repos, up to 500ms+ on repos with 1000+ commits | Only if capped to `git log -n 10` and repos are small |
| Shell out to `git` instead of using git2/gix | No C FFI, simpler build, single binary preserved | Slower than native, depends on git being installed, output parsing fragility | Acceptable -- `git` is a reasonable dependency for a dev tool; all GSD users have it |
| Store process PIDs in memory only | No persistence code needed | Lose track of spawned sessions on TUI restart or crash | Never for queue execution -- must persist to disk |
| Parse markdown with regex | No parser dependency | Breaks on edge cases (code blocks containing markdown syntax, nested lists) | Acceptable for known-format files (QUEUE.md checklist, STATE.md frontmatter) |
| Hardcode Claude session paths (~/.claude/projects/) | Quick detection implementation | Breaks on Claude CLI updates, XDG compliance changes, non-standard installs | Only with version-detection fallback and trait abstraction |
| General DAG layout for 4-stage pipeline | Looks impressive, future-proof | Weeks of layout work for a problem that doesn't exist yet | Never for v1.1 -- use fixed 4-column layout |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Claude CLI process spawning | Spawning `claude` without restoring terminal first | Use RAII terminal handoff guard; `ratatui::restore()` before spawn, `ratatui::init()` on Drop |
| Git log parsing | Parsing default format (locale-dependent, ambiguous separators) | Use `git log --format=%x00%H%x00%an%x00%aI%x00%s` with null-byte separators |
| File watcher + git operations | Git checkout/rebase triggers hundreds of file change events in a burst | Debounce at 200ms (already done), add burst detection: if 10+ events in 1s, wait 2s before re-parse |
| STATE.md frontmatter | Using a TOML/YAML parser that normalizes whitespace on write-back | Parse with regex, modify specific fields, preserve surrounding bytes |
| QUEUE.md concurrent write | Writing while GSD is also writing to the same file (race condition) | Use atomic rename (`write to .tmp` then `fs::rename()`) or advisory file locking (`flock`) |
| Notify watcher + own writes | TUI writes to QUEUE.md, triggering its own FileChanged event, causing redundant re-parse | Set a self-write timestamp per project; ignore FileChanged events within 500ms of own writes |
| Claude session JSONL files | Reading `.jsonl` session files while Claude is actively writing them | Read-only access only; use modification time for freshness check; never parse partially written last line |
| Process stdout streaming | Collecting all of `git log` output before parsing | Stream line-by-line; use `BufReader::lines()` on the child's stdout to avoid buffering entire output |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Parsing all backlog items on every FileChanged | UI lag proportional to backlog item count | Lazy-load backlog only when backlog browser is open; cache otherwise | 15+ backlog items with complex markdown content |
| Running `git log` on every FileChanged in `.planning/` | Noticeable 100-500ms delay on each file change event | Cache git log; only re-run when `.git/` directory events fire (not `.planning/`) | Repos with 500+ commits |
| Re-rendering full DAG on every tick | CPU spike when execution flow graph view is active | Only re-render DAG when underlying state changes, not on tick; cache the rendered `Buffer` | DAGs with 10+ phase nodes visible simultaneously |
| Unbounded channel backpressure from process output | Memory grows if a spawned process writes faster than the TUI processes | Use bounded channel for process output; drop oldest entries if full; show "output truncated" | Process producing 1000+ lines/second |
| Scanning `/proc/` for Claude detection on every tick (250ms) | 1-5ms per scan adds up, visible CPU at scale | Cache process detection results; re-scan on 5-10s interval or on explicit user refresh | Systems with 500+ running processes |
| Re-parsing entire ROADMAP.md to get single phase status | Wasted work when only one phase's status is needed | Parse once, cache `Vec<RoadmapPhase>`, update individual entries on targeted changes | ROADMAP.md files with 15+ phases |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Executing queue items without user confirmation | Arbitrary command execution from QUEUE.md (any process with file access can modify it) | Always display the full command and require explicit y/n confirmation before spawning |
| Passing unsanitized project paths to shell commands | Command injection via project names or paths containing shell metacharacters | Use `Command::new("git").arg(...)` (never `format!("git log {}", path)`); validate paths on registration |
| Storing Claude API keys or session tokens in gsd-manager config | Credential exposure in plaintext config file | Never store credentials; rely on Claude CLI's own auth mechanism. Config should only contain project paths |
| Backlog browser path traversal | If backlog item paths aren't validated, reading files outside `.planning/` | Restrict all file operations to within the project's `.planning/` directory; canonicalize paths and reject any containing `..` |
| Running spawned Claude sessions with TUI's full environment | Environment variables (API keys, tokens) leak to child processes | Sanitize environment before spawning; only pass necessary vars |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Launching Claude session without feedback | User thinks nothing happened, launches again, gets duplicate sessions | Show spinner in project row immediately; transition to "Claude active" when process confirms start |
| Git history viewer blocks on initial load | User enters git view, sees blank screen for 2+ seconds on large repos | Show "Loading git history..." immediately; stream results as they arrive, render incrementally |
| Backlog edit loses unsaved changes on accidental Escape | User accidentally hits Esc, loses typed content | Require confirmation ("Discard changes? y/n") if input buffer is non-empty |
| Execution flow graph illegible at small terminal sizes | Graph overlaps, text truncates, becomes useless below 80x24 | Detect terminal size; switch to simplified list view below 100 columns for graph mode |
| Queue execution gives no progress feedback | User queues 3 items, sees no output for minutes | Stream Claude's last output line to a status area; show elapsed time per running session |
| "Fact vs assumption" distinction is invisible | User cannot tell if "Phase 3: executing" comes from STATE.md or is inferred from disk | Use visual markers: solid color for facts (STATE.md), dimmed/italic for inferred state (disk heuristics) |
| Too many new keybindings without discoverability | User cannot remember how to access git history vs backlog vs graph | Context-sensitive footer showing available actions for current screen; consistent navigation (Esc always goes back) |

## "Looks Done But Isn't" Checklist

- [ ] **State reader fix:** Verify against completed milestones (not just active ones) -- the "P5: Unknown" bug specifically happens when all phases are done and `completed_phases + 1` exceeds `total_phases`
- [ ] **Disk-based inference:** Test with archived phases (v1.0 phases were moved), missing PLAN files, partially completed phases, and phases with `autonomous: false` markers
- [ ] **Git history:** Test with repos that have zero commits, merge commits, detached HEAD, non-ASCII commit messages, and repos where `.planning/` was added mid-project
- [ ] **Claude session detection:** Test when Claude CLI is not installed, when multiple versions exist (nvm/volta), when running through tmux, and when another user's Claude session is running
- [ ] **Queue execution:** Test graceful shutdown (Ctrl+C during execution), TUI crash recovery (SIGKILL), and re-discovering sessions from PID file after restart
- [ ] **Backlog browser:** Round-trip test with real GSD project files from at least 3 different projects -- `parse -> serialize (no edits) -> diff` must be empty
- [ ] **Execution flow graph:** Test with 1 phase, 10+ phases, phases with very long names (40+ chars), and terminal sizes from 80x24 to 200x50
- [ ] **Terminal handoff:** Test spawning Claude, killing Claude mid-session (SIGTERM, SIGKILL), and verifying TUI terminal state is restored correctly in both cases
- [ ] **File watcher self-triggering:** Write to QUEUE.md from TUI, verify exactly one re-parse happens (not zero, not infinite loop)
- [ ] **Concurrent access:** Have GSD actively running on a project while the TUI is displaying it -- verify no file corruption or parse errors

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Terminal corruption from bad spawn | LOW | `reset` command restores terminal; add `gsd-manager --recover` CLI flag that just calls `ratatui::restore()` and exits |
| Zombie/orphan Claude processes | LOW | `pkill -f "claude.*--cwd"` clears orphans; PID file allows TUI to clean up on next launch |
| Markdown corruption in .planning/ files | MEDIUM | Files are in git; `git checkout -- .planning/QUEUE.md` recovers. If committed, requires `git revert` |
| InputMode enum becomes unmaintainable | HIGH | Requires architecture refactor to screen/component system. 2-3 day effort once app is at 20+ modes. Better to prevent |
| State reader inaccuracy cascading to UI | LOW | All state is derived from files; re-parse on demand with `r` to refresh. No persistent mutation means no data loss |
| Git log parsing breaks on unusual commit | LOW | Graceful degradation to "git history unavailable" message; log the parsing error via tracing for debugging |
| Stale PID file after crash | LOW | On startup, validate all PIDs with `kill(pid, 0)`; remove entries where process no longer exists |
| DAG rendering garbled at small size | LOW | Automatic fallback to list view; user can resize terminal to restore graph view |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| InputMode enum explosion | Phase 1: Architecture refactor | New features add screens via trait impl, not new enum variants; `App` field count stays stable |
| Terminal state corruption | Queue execution / Claude session phases | Integration test: spawn child, kill it, verify terminal is usable afterward |
| Sync I/O blocking render | State reader fix phase | No function in render path calls `std::fs::*` or `std::process::Command`; all I/O through `spawn_blocking` |
| Zombie/orphan processes | Queue execution phase | TUI exit with running processes shows confirmation dialog; PIDs persisted; restart re-discovers them |
| Markdown round-trip corruption | Backlog browser phase | Automated round-trip test on 3+ real GSD `.planning/` directories passes with zero diff |
| Claude detection fragility | Claude session management phase | Feature degrades gracefully to "unknown"; no crash when Claude is not installed |
| DAG illegibility at small sizes | Execution flow graph phase | Render test at 80x24 produces readable list fallback; 120x40 shows full graph |
| File watcher self-triggering | State reader fix phase | Write test: TUI writes file, verify one re-parse (not zero, not infinite loop) |
| Git log parsing edge cases | Git history viewer phase | Parser tested with null-byte format on: empty repo, merge commits, non-ASCII messages, detached HEAD |
| Disk inference ambiguity | State reader fix phase | Inference results carry confidence level; UI shows fact-vs-inference distinction; archived phases excluded |

## Sources

- [Ratatui component architecture docs](https://ratatui.rs/concepts/application-patterns/component-architecture/) -- application pattern guidance for scaling beyond 5 screens (HIGH confidence)
- [Ratatui: Spawn External Editor recipe](https://ratatui.rs/recipes/apps/spawn-vim/) -- terminal handoff pattern for spawning interactive child processes (HIGH confidence)
- [Ratatui Elm Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) -- TEA state management for growing apps (HIGH confidence)
- [Ratatui forum: ergonomic application state](https://forum.ratatui.rs/t/how-do-i-represent-application-state-ergonomically/54) -- community discussion on InputMode scaling (MEDIUM confidence)
- [Tokio process::Child docs](https://docs.rs/tokio/latest/tokio/process/struct.Child.html) -- zombie process behavior, kill_on_drop semantics (HIGH confidence)
- [Tokio issue #6797](https://github.com/tokio-rs/tokio/issues/6797) -- child process can spawn even when Command::spawn returns error (HIGH confidence)
- [Rust forum: spawn process with timeout](https://users.rust-lang.org/t/spawn-process-with-timeout-and-capture-output-in-tokio/128305) -- timeout and output capture patterns (MEDIUM confidence)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) -- session commands: --continue, --resume, --fork-session, -p for non-interactive (HIGH confidence)
- [Claude Code session extractor gist](https://gist.github.com/gchamon/0949dc427086d387e9164b229271288b) -- ~/.claude/projects/<hash>/ session storage structure (MEDIUM confidence)
- [Claude Code tmux integration](https://www.devas.life/how-to-run-claude-code-in-a-tmux-popup-window-with-persistent-sessions/) -- session persistence with tmux, pgrep detection pattern (MEDIUM confidence)
- [ascii-dag crate](https://crates.io/crates/ascii-dag) -- Sugiyama layout for terminal DAG rendering, fan topology limitations (MEDIUM confidence)
- [ascii-petgraph crate](https://github.com/elefthei/ascii-petgraph) -- force-directed graph rendering in terminal with ratatui (MEDIUM confidence)
- Codebase analysis: `src/app.rs` -- InputMode enum (11 variants), App struct (18 fields), sync state parsing, existing spawn_blocking pattern (HIGH confidence, direct observation)
- Codebase analysis: `src/watcher.rs` -- 200ms debounce, recursive watch, project root extraction (HIGH confidence, direct observation)
- Codebase analysis: `src/state_reader/mod.rs` -- sync file I/O in parse_project_state, backlog counting pattern (HIGH confidence, direct observation)

---
*Pitfalls research for: GSD Meta Manager v1.1 -- process management, git integration, file editing, graph visualization in Rust TUI*
*Researched: 2026-03-26*
