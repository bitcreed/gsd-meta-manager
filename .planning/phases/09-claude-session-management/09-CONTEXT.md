# Phase 09: Claude Session Management - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Detect active Claude Code sessions across registered projects, show indicators on the dashboard, provide a session list in the detail view, and enable launching/resuming sessions in new terminal tabs. Linux-only for now (uses /proc filesystem).

</domain>

<decisions>
## Implementation Decisions

### Session Detection
- **D-01:** Detect active sessions via `pgrep -x claude` + read `/proc/PID/cwd` to match against registered project paths
- **D-02:** Parse `/proc/PID/cmdline` for session ID — `--resume UUID` gives session ID, plain `claude` = new session
- **D-03:** Poll every 5 seconds for session status updates (integrate with existing tick interval)
- **D-04:** Dashboard indicator: `▶` (play icon, Unicode U+25B6) in green next to project name when Claude is running in that directory

### Session List View
- **D-05:** Session list lives as tab 7 ("Sessions") in the detail view, extending the tab system
- **D-06:** Per-session info: PID, session ID (truncated to 8 chars), start time from `/proc/PID/stat`
- **D-07:** Show only active sessions (running processes) — no historical session log
- **D-08:** Selectable list with j/k navigation, Enter to act on selected session

### Session Actions
- **D-09:** Launch new session: open new terminal tab with `$TERMINAL -e claude` in the project's working directory
- **D-10:** Resume existing session: open new terminal tab with `$TERMINAL -e claude --resume {session_id}` in project's cwd
- **D-11:** Terminal detection: try `$TERMINAL`, fall back to common terminals (kitty, alacritty, gnome-terminal, xterm)
- **D-12:** No kill/stop functionality in v1.1 — view and launch only

### Claude's Discretion
- Session list layout proportions
- How to handle stale PIDs (process died between polls)
- Terminal fallback order
- Whether to show session count in tab header

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/ui/screens/detail.rs` — Tab system with 6 tabs, well-established pattern
- `src/ui/screens/mod.rs` — ProjectViewCache for per-project state
- `src/ui/screens/normal.rs` — Dashboard row rendering (for indicator placement)
- `src/state_reader/mod.rs` — ProjectState with project paths

### Established Patterns
- Tab system: DetailSubView enum + number-key switching + Tabs widget
- Polling: existing tick interval in event loop (5s matches well)
- Process spawning: tokio::process::Command already used for git operations

### Integration Points
- ProjectState needs `active_sessions: Vec<ClaudeSession>` field
- Dashboard row rendering needs `▶` indicator check
- Event loop tick handler needs session refresh logic
- DetailSubView needs Sessions variant (tab 7)

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Real-time session activity indicators (SESS-04) — streaming status
- Attach to running session output stream (SESS-05)
- Kill/stop sessions from TUI
- Historical session browsing from .jsonl files
- Windows/macOS session detection (non-/proc approaches)

</deferred>

---

*Phase: 09-claude-session-management*
*Context gathered: 2026-03-27 via Smart Discuss (autonomous mode)*
