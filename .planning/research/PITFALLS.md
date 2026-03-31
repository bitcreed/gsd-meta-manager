# Pitfalls Research

**Domain:** Adding archive browsing, paused-project detection, tech debt cleanup, and queue execution research to an existing Rust TUI (gsd-meta-manager v1.2)
**Researched:** 2026-03-31
**Confidence:** HIGH (based on direct codebase analysis of 6,319 LOC, GSD workflow file inspection, and milestone archive structure audit)

## Critical Pitfalls

### Pitfall 1: Archive browser reads entire milestone tree synchronously on the render thread

**What goes wrong:**
The milestones directory contains nested structures (`milestones/v1.1-phases/05-state-reader-accuracy/05-01-PLAN.md`). A naive implementation scans the full tree with `std::fs::read_dir` recursively when the user opens the Archive tab. With 2+ milestones each containing 5-10 phases with 5+ artifacts each (observed: v1.0 and v1.1 archives already exist with this structure), this blocks the TUI render loop for 50-200ms -- enough to feel sluggish and trigger missed key events.

**Why it happens:**
The existing backlog browser (`src/state_reader/backlog.rs`) and disk status scanner (`src/state_reader/disk_status.rs`) both use synchronous `std::fs::read_dir`. Developers copy this pattern for archive browsing without realizing the archive tree is 3-4 levels deep instead of 1-2, and grows unboundedly with each completed milestone.

**How to avoid:**
Load milestone directory listings lazily: scan top-level milestone names on tab activation (fast -- just `read_dir` on `milestones/`), then scan phase dirs only when user expands a milestone. Read file contents (markdown preview) only when user selects a specific artifact. Send all I/O through `tokio::spawn_blocking` and deliver results via the existing `Action` channel, matching the async pattern already used for git log loading in the detail view.

**Warning signs:**
- Any `std::fs::read_to_string` call in a `render()` method
- Archive tab takes noticeably longer than other tabs to first paint
- Key events are dropped or delayed after switching to Archive tab

**Phase to address:**
Archive Browser implementation phase. Establish the lazy-load pattern in the first plan task, before building UI widgets.

---

### Pitfall 2: HANDOFF.json race conditions -- reading while GSD is writing

**What goes wrong:**
When `/gsd:pause-work` runs, it writes `HANDOFF.json` as a complete JSON blob via Claude's Write tool. The meta-manager's file watcher fires on the write event and `parse_project_state()` re-reads the planning directory. If the meta-manager reads `HANDOFF.json` mid-write (between `open` and `close`), it gets truncated JSON, causing a serde parse error. The watcher then either shows stale state or marks the project as "unknown."

**Why it happens:**
The GSD `pause-work` workflow writes `HANDOFF.json` with a simple file write -- not an atomic write-then-rename like the meta-manager's own `queue_md.rs` uses (`QUEUE.md.tmp` -> `QUEUE.md`). The notify-debouncer-full 200ms window helps but does not eliminate the race -- fast writes can complete within one debounce cycle while still being read mid-operation by another process.

**How to avoid:**
1. Treat `HANDOFF.json` parse failures as non-fatal: if serde fails, retain the previous known state for that project and set a "stale" flag.
2. On parse failure, schedule a re-read after 500ms (single retry with backoff) via the existing Action channel.
3. Do NOT write to `HANDOFF.json` yourself -- GSD owns this file. Read-only access prevents bidirectional races.
4. Consider checking file size stability: if file size changed between the watcher event and the read attempt, defer the read.

**Warning signs:**
- Intermittent "unknown" status flickers on projects being paused
- serde_json errors in the tracing log file during pause operations
- Tests that mock HANDOFF.json never fail, but real usage does

**Phase to address:**
Paused-project detection phase. The HANDOFF.json reader must be designed defensively from the start.

---

### Pitfall 3: Tech debt cleanup breaks existing tests and removes needed API surface

**What goes wrong:**
Removing `#[allow(dead_code)]` annotations (7 locations across the codebase: `change_tracker.rs:14`, `state_md.rs:4,25`, `roadmap_md.rs:4`, `backlog.rs:7`, `state_reader/mod.rs:3`, `ui/screens/mod.rs:33`) forces you to either use or remove the annotated items. Removing fields from `ProjectSnapshot` in `change_tracker.rs` could break `state_reader_test.rs`. Worse, removing `ScreenAction` variants from `ui/screens/mod.rs:33` eliminates dispatch capabilities that the new Archive Browser screen would need.

**Why it happens:**
Dead code warnings were suppressed during rapid v1.0/v1.1 development. The suppressed items fall into two categories: (a) genuinely unused code that should be removed (like `config_json` module), and (b) code that exists for API completeness or imminent v1.2 use (like `ScreenAction::DispatchAction`). Developers conflate the two and batch-delete everything.

**How to avoid:**
1. Audit each `#[allow(dead_code)]` individually. For each one, categorize as: (a) truly unused, (b) used only in tests, (c) part of Screen trait API that new screens need, or (d) planned for v1.2 use.
2. The `config_json` module (`src/state_reader/config_json.rs`) is `#[allow(dead_code)]` at the module level -- it was written for GSD integration but the codebase uses `state_md.rs` instead. Verify it has no consumers before removing.
3. `ScreenAction` variants: check which are used by the 8 existing screen implementations before removing any. The Archive Browser will likely need `DispatchAction`.
4. Run `cargo nextest run` after each individual removal, not in batch.

**Warning signs:**
- Batch-removing all `#[allow(dead_code)]` in one commit
- Tests pass but the archive browser implementation later needs a variant that was deleted
- `cargo check` warns about new dead code in areas adjacent to the removed annotations

**Phase to address:**
Tech debt cleanup phase, BEFORE Archive Browser phase. This ordering lets the archive browser use cleaned-up APIs without reintroducing suppressions.

---

### Pitfall 4: Queue execution research designs for stdin injection instead of CLI invocation

**What goes wrong:**
The research phase concludes that queue execution should work by: launching a Claude session, injecting QUEUE.md commands via stdin, and monitoring stdout. This misunderstands GSD's architecture. GSD workflows are Claude slash commands (e.g., `/gsd:execute-phase`), not shell commands. They require Claude Code's internal prompt routing, not process I/O. Designing for stdin injection produces a dead-end architecture.

**Why it happens:**
The existing `session_detector.rs` detects Claude sessions via `pgrep` and `/proc`, creating a mental model of Claude as "a process you interact with." The queue already stores items as strings like `/gsd:quick` and `/gsd:plan-phase 4`. Developers naturally assume these can be piped to a process.

**How to avoid:**
The research must explicitly investigate and document these actual GSD execution mechanisms:
1. **Claude Code CLI invocation**: `claude --prompt "/gsd:execute-phase 5"` launches a new session with a prompt. This is the primary programmatic entry point.
2. **GSD autonomous mode**: `workflows/autonomous.md` shows GSD can chain phases autonomously. Queue items could be batched into a single autonomous session prompt.
3. **QUEUE.md as prompt context**: The simplest integration is passing QUEUE.md content as part of a `--prompt` argument, letting Claude/GSD interpret it. No hook needed.
4. **File-based signaling**: Write to QUEUE.md, rely on GSD's `/gsd:resume-work` (which reads HANDOFF.json and project state) to discover queued items on session start.

The research deliverable should be a design document, not code. The goal is to map the integration surface.

**Warning signs:**
- Research output includes prototype code for stdin/stdout piping to Claude
- Research assumes Claude sessions accept runtime command injection after launch
- Research does not reference `autonomous.md`, `pause-work.md`, or `resume-project.md` from the GSD workflows directory

**Phase to address:**
Queue execution research phase. Must read GSD workflow files before designing.

---

### Pitfall 5: Archive browser renders raw markdown as unformatted text walls

**What goes wrong:**
Phase artifacts (PLAN.md, SUMMARY.md, VERIFICATION.md) are structured markdown with headers, lists, code blocks, and tables. Displaying them as raw text in a ratatui `Paragraph` widget produces dense, unformatted content. Users cannot distinguish headers from body text or code blocks from prose. Observed: the existing backlog browser already has this problem -- `backlog.rs` reads markdown content and the detail view displays it as plain `Paragraph` spans.

**Why it happens:**
It works acceptably for short backlog descriptions (1-5 lines). Archive artifacts are 50-200 lines with rich structure. The same rendering approach produces unusable output at this scale.

**How to avoid:**
1. Implement minimal markdown-to-styled-spans conversion: bold headers with color, code blocks with background highlight, list items with indentation. This is NOT a full markdown renderer.
2. Use `pulldown-cmark` (standard Rust markdown parser, ~4M crates.io downloads) to tokenize markdown into events, then map events to ratatui `Span` styles. This is approximately 50-100 lines of mapping code.
3. Limit initial display to visible terminal lines with scroll support, rather than loading entire file into a single `Paragraph`.

**Warning signs:**
- Archive file content displayed using `Paragraph::new(raw_content)` with no style mapping
- Headers in archived PLANs are indistinguishable from body text
- Users immediately ask "can I just open this in my editor?"

**Phase to address:**
Archive Browser implementation phase, as a rendering subtask after navigation tree works.

---

### Pitfall 6: Paused-project detection only checks file existence, missing content validation

**What goes wrong:**
GSD's pause workflow creates TWO files: `HANDOFF.json` (machine-readable, in `.planning/`) and `.continue-here.md` (human-readable, in `.planning/phases/XX-name/`). Simply checking `HANDOFF.json` exists is insufficient because: (a) a stale `HANDOFF.json` from a previous pause may never have been cleaned up after resume, and (b) the file could be partially written or corrupted.

**Why it happens:**
`resume-project.md` reads `HANDOFF.json` but does not guarantee deletion after resume. The two files have different lifecycles that are not documented as a contract. A developer who checks only existence will show "paused" on projects that are actively being worked on.

**How to avoid:**
1. Primary signal: `HANDOFF.json` exists AND contains `"status": "paused"` in its JSON content.
2. Staleness check: Compare `HANDOFF.json` timestamp against most recent git commit timestamp. If the last commit is significantly newer than the handoff file, the project was likely resumed without cleanup.
3. Secondary signal: `.continue-here.md` exists in an active phase directory. Use for "has handoff context" display but NOT as the sole "is paused" indicator.
4. Clear rule: If `HANDOFF.json` does not exist, the project is NOT paused, regardless of `.continue-here.md` presence.

**Warning signs:**
- Project shows "paused" badge after user has already resumed and is actively working
- Paused badge persists across multiple milestones because HANDOFF.json was never cleaned up
- False negative: project IS paused but HANDOFF.json was partially written

**Phase to address:**
Paused-project detection phase. Define the detection heuristic and staleness rules before writing code.

---

### Pitfall 7: Adding an 8th tab overflows the tab bar at standard terminal widths

**What goes wrong:**
The current detail view has 7 tabs (`1:Phases` through `7:Sessions`, defined in `detail.rs:20`) with number-key shortcuts. Adding an 8th "Archive" tab pushes the tab bar beyond 80 columns. The ratatui `Tabs` widget clips overflow without visual indication, making the rightmost tabs invisible on standard terminals.

**Why it happens:**
Current tab labels total approximately 85 characters including separators. An 8th label adds 10+ characters, reaching ~95. At 80 columns (the POSIX minimum), the last 1-2 tabs are clipped.

**How to avoid:**
1. Use shorter tab labels: abbreviate to fit 80 columns (e.g., `1:Ph 2:Rm 3:Bl 4:Git 5:Pl 6:Qu 7:Se 8:Ar`), with full names shown in the help overlay.
2. Or: make the Archive browser a separate screen (pushed onto `screen_stack` from the detail view) rather than an 8th tab. This is architecturally cleaner -- archive browsing is a deep-dive activity, not a quick-glance tab.
3. Test at 80x24 during development. Add a terminal size assertion in the rendering code.

**Warning signs:**
- Tab labels truncated at 80 columns with no visual cue
- Users cannot see or access the Archive tab without widening their terminal
- `sub_view_from_index()` returns wrong variant for index 7+ due to off-by-one

**Phase to address:**
Archive Browser implementation phase, during UI design before widget work.

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `#[allow(dead_code)]` on `config_json` module | Suppresses warnings for unused GSD integration code | Module takes up space, confuses contributors, inflates binary | Remove during tech debt phase -- module appears genuinely unused |
| `#[allow(dead_code)]` on `ScreenAction` variants | Enables adding new screen dispatch types without immediate callers | Hides which variants are genuinely needed vs. speculative | Keep with comment `// Used by: archive browser (v1.2)` for variants that v1.2 needs |
| Synchronous file reads in `parse_project_state()` | Simple code path, no async machinery | Blocks render loop when archive scanning adds deep directory traversal | Acceptable for STATE.md/ROADMAP.md (small, fast); NOT acceptable for archive tree |
| `active_sessions` duplicated on both `App` and `AppContext` | Avoided borrow checker issues during v1.1 | Two sources of truth for session state | Fix during tech debt phase -- consolidate to one location |
| `Instant::now()` for change timestamps | No wall-clock formatting dependency | Cannot show "paused at 2:30pm" for HANDOFF.json display | Must add wall-clock parsing for HANDOFF.json timestamps anyway |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| HANDOFF.json reading | Assuming the file is always valid JSON (partial writes during GSD pause) | Wrap serde parse in `Result`, fall back to previous state on error, retry after 500ms delay |
| HANDOFF.json lifecycle | Checking only file existence to determine "paused" status | Parse content for `"status": "paused"` AND check staleness against recent git activity |
| QUEUE.md concurrent access | Writing to QUEUE.md while GSD's `/gsd:resume-work` is reading it | Meta-manager already uses atomic write-rename (`QUEUE.md.tmp` -> `QUEUE.md`); ensure reads tolerate momentary file absence |
| Milestone directory structure | Assuming milestone archives are nested as `v1.0/phases/...` | Actual structure: `v1.0-phases/`, `v1.0-ROADMAP.md`, `v1.0-REQUIREMENTS.md` are siblings in `milestones/`. Phase dirs are inside `v1.0-phases/`, not a parent `v1.0/` dir |
| GSD `config.json` in `.planning/` | Reading it expecting project metadata | Contains GSD workflow configuration (model settings, feature flags), NOT project state. State comes from STATE.md and ROADMAP.md |
| Milestone naming convention | Hardcoding `v*` prefix pattern | Milestone names follow `{version}-phases`, `{version}-ROADMAP.md` pattern, but version format is user-defined. Parse dynamically |
| `.planning/` directory watching for archives | Adding `milestones/` to the watcher expecting change events | Milestone archives are static (written once during `/gsd:complete-milestone`). Watching them wastes inotify watches. Only watch active files |
| GSD workflow commands in queue | Treating `/gsd:execute-phase 5` as a shell command | These are Claude slash commands requiring Claude Code's prompt routing. Cannot be executed via `std::process::Command` |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Scanning all milestone phase directories on every file-change event | UI lag after any file save in any project | Only scan milestones on explicit user action (opening Archive tab), not on watcher events | >3 milestones with >8 phases each |
| Loading full markdown file content for all archive artifacts | Memory grows unboundedly with project history | Load file content only for the currently-viewed artifact, drop when user navigates away | Projects with 50+ archived phase artifacts (already approaching this with v1.0 + v1.1 archives) |
| Re-parsing HANDOFF.json on every watcher tick | Wasted I/O when project is not paused (file does not exist) | Check file existence with `Path::exists()` before attempting read+parse. Cache the "not present" state | >20 registered projects, each checked on every tick |
| Building full milestone tree structure eagerly on app startup | 1-3 second startup delay as milestones accumulate | Defer milestone tree construction until Archive tab is first opened | >5 completed milestones |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Archive browser shows flat file list without milestone grouping | User cannot tell which milestone a phase belongs to | Tree structure: Milestone -> Phase -> Artifacts, with expand/collapse |
| Paused badge shown without temporal context | User sees "paused" but cannot tell if it was 5 minutes or 5 days ago | Show relative time from HANDOFF.json `timestamp` field: "Paused 2h ago" |
| Archive markdown not searchable or filterable | User cannot find a specific decision across past milestones | Defer search to v1.3, but structure the data model to support it (store file paths, load content on demand) |
| Queue execution research produces no visible TUI change | User perceives v1.2 as having no queue improvements despite research effort | Add a "Queue execution: designed for v1.3" note in the Queue tab footer |
| Tech debt cleanup invisible to users | Compiler warnings fixed but no user-facing improvement | Pair at least one visible fix (e.g., removing a visual glitch from deferred UAT) with the cleanup work |

## "Looks Done But Isn't" Checklist

- [ ] **Archive browser:** Often missing scroll position persistence -- verify that switching away from Archive tab and back preserves the user's position in the milestone tree
- [ ] **Archive browser:** Often missing handling of milestones with no phases directory -- verify graceful display when `v1.0-phases/` does not exist but `v1.0-ROADMAP.md` does
- [ ] **Paused detection:** Often missing re-detection on resume -- verify that the paused badge disappears within one watcher cycle after HANDOFF.json is deleted
- [ ] **Paused detection:** Often missing delete event handling -- `notify` may report file deletion differently than modification. Verify the watcher handles `EventKind::Remove` for HANDOFF.json
- [ ] **Tech debt:** Often missing regression verification -- verify that removing each `#[allow(dead_code)]` does not break `cargo test` AND `cargo nextest run` (integration tests in `tests/`)
- [ ] **Tech debt:** Often missing the stale integration test fix -- `tests/registry_test.rs` or `tests/state_reader_test.rs` may have outdated assertions from v1.0 that silently pass but test wrong behavior
- [ ] **Archive markdown rendering:** Often missing terminal width handling -- verify that long lines in archived PLANs wrap correctly at 80 columns
- [ ] **HANDOFF.json parser:** Often missing forward compatibility -- verify the parser ignores unknown fields (serde `#[serde(deny_unknown_fields)]` would break on future HANDOFF.json versions)
- [ ] **Tab navigation:** Often missing help overlay update -- verify that the help screen (`?` key) documents whatever keybinding accesses the Archive browser

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Archive browser blocks render loop | MEDIUM | Extract file reads into `tokio::spawn_blocking`, add `Loading...` placeholder in render, deliver results via Action channel |
| HANDOFF.json parse race | LOW | Add `Option<HandoffState>` with fallback to previous state, single retry after 500ms |
| Tech debt removal breaks screen dispatch | LOW | Revert the specific `#[allow(dead_code)]` removal, add comment explaining why it stays, re-run tests |
| Wrong queue execution research direction | LOW | Research-only phase -- discard and rewrite the design doc. No code to undo |
| Tab bar overflow at 80 columns | LOW | Shorten tab labels OR switch Archive to a separate screen pushed onto screen_stack |
| Stale paused badge after resume | LOW | Add HANDOFF.json to watched files, handle both modify and delete events, add staleness heuristic |
| Milestone structure assumption wrong | LOW | Print actual `milestones/` listing in debug log, adjust parser to handle observed structure |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Archive blocks render (P1) | Archive Browser | `tokio::spawn_blocking` used for all archive I/O; no `std::fs` in `render()` |
| HANDOFF.json race (P2) | Paused Detection | Intentionally corrupt HANDOFF.json mid-write; verify no crash or permanent "unknown" |
| Tech debt breaks tests (P3) | Tech Debt Cleanup | `cargo nextest run` passes after every individual `#[allow]` removal |
| Wrong GSD hook design (P4) | Queue Execution Research | Research doc references `autonomous.md`, `pause-work.md`, `resume-project.md` by name |
| Raw markdown rendering (P5) | Archive Browser | Archived PLAN.md headers render in bold/color, code blocks visually distinct |
| HANDOFF existence-only check (P6) | Paused Detection | Test matrix: HANDOFF.json present+paused, present+stale, absent+continue-here present, neither |
| Tab bar overflow (P7) | Archive Browser (UI design) | Screenshot at 80x24 terminal shows all tabs readable; or Archive is a separate screen |

## Sources

- Direct codebase analysis: `src/state_reader/disk_status.rs` (lines 126-234 -- `find_phase_dir` and `infer_phase_status` already scan milestones), `src/state_reader/mod.rs` (sync I/O in `parse_project_state`), `src/state_reader/queue_md.rs` (atomic write pattern at line 44-59), `src/ui/screens/detail.rs` (7-tab layout at line 20, `DetailSubView` enum), `src/ui/screens/mod.rs` (7 `#[allow(dead_code)]` locations, `ScreenAction` variants, `AppContext` struct), `src/change_tracker.rs` (in-memory only tracking, `ProjectSnapshot` fields), `src/watcher.rs` (200ms debounce, recursive watch, `extract_project_root`)
- GSD workflow files: `~/.claude/get-shit-done/workflows/pause-work.md` (HANDOFF.json schema with `"status": "paused"`, `.continue-here.md` dual-file pattern), `~/.claude/get-shit-done/workflows/resume-project.md` (HANDOFF.json consumption on resume), `~/.claude/get-shit-done/workflows/autonomous.md` (phase chaining via `roadmap analyze`, `--from N` flag)
- Milestone archive structure: `.planning/milestones/` contains `v1.0-phases/`, `v1.0-ROADMAP.md`, `v1.1-phases/`, `v1.1-ROADMAP.md` as siblings (not nested). Phase dirs inside `v1.1-phases/` contain full artifact sets (PLAN, SUMMARY, CONTEXT, RESEARCH, VERIFICATION files observed in `05-state-reader-accuracy/`)
- Compiler warning audit: 7 `#[allow(dead_code)]` and `#[allow(unused)]` annotations identified across source tree via grep

---
*Pitfalls research for: gsd-meta-manager v1.2 -- archive browsing, paused-project detection, tech debt cleanup, queue execution research*
*Researched: 2026-03-31*
