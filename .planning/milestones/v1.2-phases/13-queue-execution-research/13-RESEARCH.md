# Phase 13: Queue Execution Research - Research

**Researched:** 2026-03-31
**Domain:** GSD workflow integration, CLI orchestration, process management
**Confidence:** HIGH

## Summary

This phase produces a design document (not code) that enables v1.3 queue execution implementation. The research investigates GSD's internal architecture -- autonomous mode lifecycle, workflow hooks, CLI capabilities (`claude -p`, `--continue`, `--resume`), and the WAITING.json signal mechanism -- to identify integration points where the TUI can dispatch and monitor GSD workflow execution.

GSD workflows are prompt-driven skill files (markdown) that chain through `Skill()` invocations inside a Claude Code session. The TUI cannot call `Skill()` directly -- it must spawn `claude -p` as an external process. The key insight is that GSD already has a `WAITING.json` signal file mechanism for external watchers, and the `state signal-waiting` / `state signal-resume` CLI commands provide the bidirectional communication channel the TUI needs. The autonomous workflow (`autonomous.md`) demonstrates the full discuss-plan-execute chain and is the closest existing pattern to queue execution.

**Primary recommendation:** Design queue execution around `claude -p` headless invocations with `--continue` for session continuity, `WAITING.json` polling for checkpoint detection, and filesystem watching (already implemented) for progress updates. The design document should specify two strategies: (A) per-item `claude -p` invocations (simpler, isolated), and (B) a persistent session with `--continue` chaining (lower overhead, maintains context).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Research GSD's workflow hooks, planning artifact lifecycle, and session management by scanning the GSD source at `~/projects/node/get-shit-done`
- Focus on how QUEUE.md items get executed through GSD commands -- the TUI dispatches GSD workflows, not raw LLM calls
- Scan GSD prompts, hooks, workflows, and bin/ scripts to understand integration points
- The design must be LLM-agnostic: GSD currently uses Claude but the queue execution design should not assume any specific LLM backend
- Single design document: `.planning/phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md`
- Sections must map to success criteria: GSD autonomous mode lifecycle, hook points, integration strategies, trade-offs, safety requirements
- Confidence levels (HIGH/MEDIUM/LOW) on each design element per QRES-02
- At least 2 strategies for auto-continue from QUEUE.md, with a recommended pick
- Each strategy must identify: trigger mechanism, session lifecycle, artifact flow, error handling
- Practical safety rules: timeout limits, error handling, human escalation triggers, max retry counts

### Claude's Discretion
- Internal document organization beyond the required sections
- How deep to go on each GSD subsystem
- Whether to include sequence diagrams or architecture sketches

### Deferred Ideas (OUT OF SCOPE)
- Actual implementation of queue execution (v1.3+)
- Container-based execution (backlog 999.2)
- Remote project execution
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QRES-01 | Research document covers GSD autonomous mode lifecycle, hook points, and CLI capabilities (`claude -p`, `--continue`, `--resume`) | Autonomous workflow fully analyzed (860+ lines), hooks system documented (context-monitor, workflow-guard, statusline), CLI capabilities verified against `claude --help` output and official docs |
| QRES-02 | Research document includes a design for auto-continue from QUEUE.md with identified integration points and trade-offs | Two strategies identified (per-item isolation vs. session chaining), integration points mapped (WAITING.json, filesystem events, STATE.md, gsd-tools.cjs), safety requirements enumerated |
</phase_requirements>

## GSD Architecture Deep Dive

### GSD Autonomous Mode Lifecycle (QRES-01)

**Confidence: HIGH** -- sourced from direct reading of `~/.claude/get-shit-done/workflows/autonomous.md` (860 lines)

The autonomous workflow is the closest existing pattern to what queue execution needs. Its lifecycle:

1. **Initialize:** Parse `--from N` flag, bootstrap via `gsd-tools.cjs init milestone-op`, validate ROADMAP.md and STATE.md exist
2. **Discover Phases:** `gsd-tools.cjs roadmap analyze` returns JSON with all phases, filter to incomplete, sort by number
3. **Execute Each Phase:** For each incomplete phase, sequentially:
   - **Smart Discuss** (3a): Check `has_context` via `init phase-op`. If false and `skip_discuss` not set, run interactive grey-area resolution. If true, skip. Can auto-generate minimal CONTEXT.md.
   - **UI Phase** (3a.5): If frontend phase detected and `workflow.ui_phase` enabled, run `gsd:ui-phase`
   - **Plan** (3b): `Skill(skill="gsd:plan-phase", args="${PHASE_NUM}")`
   - **Execute** (3c): `Skill(skill="gsd:execute-phase", args="${PHASE_NUM} --no-transition")`
   - **Post-Execution Routing** (3d): Read VERIFICATION.md status (`passed`/`human_needed`/`gaps_found`), route accordingly
   - **UI Review** (3d.5): If UI-SPEC exists, advisory review
4. **Iterate:** Re-read ROADMAP.md after each phase (catches inserted decimal phases), check STATE.md for blockers
5. **Lifecycle:** After all phases: `gsd:audit-milestone` -> `gsd:complete-milestone` -> `gsd:cleanup`
6. **Handle Blocker:** User choices: fix and retry, skip phase, or stop autonomous mode

**Key observations for queue execution design:**
- Autonomous mode runs entirely within a single Claude Code session
- It uses `Skill()` invocations (not `claude -p` subprocesses) to chain workflows
- The TUI cannot call `Skill()` -- it exists only inside Claude Code's prompt context
- Progress is tracked through filesystem artifacts (STATE.md, ROADMAP.md, VERIFICATION.md, SUMMARY.md)

### Execute-Phase Workflow

**Confidence: HIGH** -- sourced from `execute-phase.md`

Key details relevant to queue execution:
- Plans are grouped into waves; within a wave, parallel execution via `Task()` subagents
- Each plan produces a `SUMMARY.md` with completion status
- Phase verification spawns a `gsd-verifier` subagent
- `WAITING.json` signal file written at decision points (see below)
- STATE.md updated via `gsd-tools.cjs state advance-plan`, `state update-progress`, `state record-metric`

### GSD Hook System

**Confidence: HIGH** -- sourced from direct reading of hooks source

GSD installs three hooks into Claude Code's hook system:

| Hook | Type | Purpose |
|------|------|---------|
| `gsd-context-monitor.js` | PostToolUse | Reads context usage from statusline bridge file, injects WARNING (<=35%) or CRITICAL (<=25%) when context is low |
| `gsd-workflow-guard.js` | PreToolUse | Soft guard: warns when edits happen outside GSD workflow. Checks Write/Edit tool calls to non-.planning/ files |
| `gsd-statusline.js` | (statusline) | Writes metrics to `/tmp/claude-ctx-{session_id}.json` for the context monitor |

**Relevance to queue execution:** These hooks operate inside Claude Code sessions, not externally. The TUI does not need to interact with them directly. However, the context monitor's bridge file (`/tmp/claude-ctx-{session_id}.json`) could be read by the TUI to show session context usage.

### WAITING.json Signal Mechanism

**Confidence: HIGH** -- sourced from `gsd-tools.cjs` source and `state.cjs`

GSD has a built-in signal file mechanism for external watchers:

```
gsd-tools.cjs state signal-waiting --type T --question Q --options "A|B" --phase P
```

Writes `.planning/WAITING.json` (or `.gsd/WAITING.json`) with:
```json
{
  "status": "waiting",
  "type": "decision_point",
  "question": "...",
  "options": ["A", "B"],
  "phase": "...",
  "timestamp": "..."
}
```

And to clear it:
```
gsd-tools.cjs state signal-resume
```

**This is the primary bidirectional communication channel for external orchestrators.** The TUI can:
1. Watch for WAITING.json creation (via existing notify watcher)
2. Read the question/options
3. Present to user in TUI
4. Write an answer somehow (or the user can answer in the Claude session)

**Limitation:** The current signal mechanism is write-only from GSD's perspective. There is no `signal-answer` command that feeds a response back. The user must answer in the Claude Code session directly. This is a gap that v1.3 implementation would need to address.

### CLI Capabilities (`claude -p`, `--continue`, `--resume`)

**Confidence: HIGH** -- verified against `claude --help` output (v2.1.87) and official docs at code.claude.com

| Flag | Purpose | Queue Execution Use |
|------|---------|---------------------|
| `-p` / `--print` | Non-interactive (headless) mode, prints response and exits | Primary execution mode for TUI-spawned GSD commands |
| `-c` / `--continue` | Continue most recent conversation in current directory | Chain multiple queue items in same session for context continuity |
| `-r` / `--resume <id>` | Resume specific session by ID | Resume a specific queue execution session after interruption |
| `--output-format json` | Structured JSON output with session_id, result, metadata | Capture session_id for `--resume`, parse results |
| `--output-format stream-json` | Newline-delimited JSON streaming | Real-time progress monitoring |
| `--allowedTools` | Auto-approve specific tools | `"Bash,Read,Edit,Write"` for autonomous execution |
| `--permission-mode` | Permission handling mode | `auto` or `bypassPermissions` for unattended execution |
| `--model <model>` | Select model | LLM-agnostic: user configures model |
| `--bare` | Skip hooks, LSP, plugins, MCP, auto-memory, CLAUDE.md | Faster startup for scripted calls (but loses GSD hooks/skills) |
| `--append-system-prompt` | Add to system prompt | Inject GSD skill context for headless runs |
| `--max-budget-usd` | Spending cap | Safety: prevent runaway costs |
| `--worktree` | Create git worktree for session | Isolation: each queue item gets its own worktree |
| `--no-session-persistence` | Don't save session to disk | Ephemeral runs that don't clutter session list |
| `--name <name>` | Display name for session | Label queue execution sessions for identification |

**Critical finding:** `--bare` mode skips CLAUDE.md auto-discovery and hooks, which means GSD's skill resolution (`/gsd:*`) still works (skills resolve via `/skill-name` according to docs), but hooks and MCP servers won't load. For GSD queue execution, the TUI should NOT use `--bare` mode because GSD's hooks and CLAUDE.md context are needed.

**Session ID flow for chaining:**
```bash
# Start first queue item, capture session_id
session_id=$(claude -p "/gsd:execute-phase 4" --output-format json | jq -r '.session_id')

# Continue with next item in same session
claude -p "/gsd:plan-phase 5" --resume "$session_id" --output-format json
```

### GSD CLI Tools (`gsd-tools.cjs`)

**Confidence: HIGH** -- sourced from reading gsd-tools.cjs header (100 lines of command documentation)

Key commands the TUI can call directly (without Claude Code):

| Command | Purpose | Queue Execution Use |
|---------|---------|---------------------|
| `init phase-op <N>` | Get phase state (has_context, has_plans, etc.) | Pre-flight check before executing queue item |
| `roadmap analyze` | Full roadmap parse with disk status | Determine what's executable |
| `state json` | STATE.md frontmatter as JSON | Current project position |
| `phase-plan-index <N>` | Plans with waves and completion status | Check if execution is needed |
| `state signal-waiting` | Write WAITING.json | (GSD writes this, TUI reads it) |
| `state signal-resume` | Remove WAITING.json | TUI could call this after user answers |
| `state load` | Load project config + state | Pre-flight validation |

**Key insight:** The TUI can call `gsd-tools.cjs` directly (it's just Node.js) to check project state before and after queue execution, without needing a Claude Code session. This enables pre-flight validation and post-execution verification.

### GSD Workflow Entry Points

**Confidence: HIGH** -- sourced from workflow files

The GSD commands that map to QUEUE.md items:

| Command | Workflow File | Typical Queue Use |
|---------|---------------|-------------------|
| `/gsd:discuss-phase N` | `discuss-phase.md` | Gather context before planning |
| `/gsd:plan-phase N` | `plan-phase.md` | Create execution plans |
| `/gsd:execute-phase N` | `execute-phase.md` | Execute plans (spawns subagents) |
| `/gsd:verify-work N` | `verify-phase.md` | Verify phase completion |
| `/gsd:autonomous` | `autonomous.md` | Run all remaining phases |
| `/gsd:quick <desc>` | `quick.md` | Small ad-hoc tasks |
| `/gsd:fast <desc>` | `fast.md` | Trivial inline fixes |
| `/gsd:do <desc>` | `do.md` | Intent dispatcher (routes to best command) |
| `/gsd:next` | `next.md` | Auto-detect and advance to next step |

**For queue execution, `/gsd:next` is particularly interesting** -- it auto-detects project state and routes to the appropriate next action. A simple queue could just repeatedly invoke `/gsd:next`.

## Integration Points for Queue Execution

### Existing TUI Infrastructure

**Confidence: HIGH** -- sourced from codebase reading

| Component | Location | Relevance |
|-----------|----------|-----------|
| `QueuedAction` struct | `src/state_reader/queue_md.rs` | Current queue data model (command string only) |
| Queue tab CRUD | `src/ui/screens/detail.rs` | Add, edit, delete, reorder, mark done |
| `suggest_next_commands()` | `src/state_reader/queue_md.rs` | Context-aware command suggestions |
| `ScreenAction::DispatchAction` | `src/ui/screens/mod.rs` | Dispatch async actions through event channel |
| `Action` enum | `src/action.rs` | TUI action types (would need new variants for execution) |
| File watcher | `src/watcher.rs` | Already watches `.planning/` -- will detect STATE.md, WAITING.json changes |
| `ProjectState` | `src/state_reader/mod.rs` | Parsed project state including `queued_actions` |

### Process Spawning Architecture

The TUI needs to spawn `claude -p` as an external process. Key considerations:

1. **tokio::process::Command** -- async process spawning, fits existing tokio runtime
2. **stdout/stderr capture** -- `--output-format stream-json` for real-time monitoring
3. **Process lifecycle** -- track PID, support cancellation (SIGTERM/SIGKILL)
4. **Working directory** -- must `cd` to the target project directory
5. **Environment** -- inherit user's environment (for API keys, PATH with `claude`)

### Communication Channels (TUI <-> GSD Session)

| Channel | Direction | Mechanism | Latency |
|---------|-----------|-----------|---------|
| Progress updates | GSD -> TUI | Filesystem watch on STATE.md, ROADMAP.md | ~1-2s (debounced) |
| Checkpoint detection | GSD -> TUI | Filesystem watch on WAITING.json | ~1-2s (debounced) |
| Execution output | GSD -> TUI | stdout stream-json parsing | Real-time |
| Completion detection | GSD -> TUI | Process exit + SUMMARY.md existence check | Immediate |
| Error detection | GSD -> TUI | Process exit code + stderr | Immediate |
| User answers | TUI -> GSD | Cannot currently inject into running session | N/A (gap) |

**Critical gap:** There is no mechanism to inject user input into a running `claude -p` session. The `--input-format stream-json` flag exists but requires piping stdin at launch. This means:
- Checkpoints that need user input (`human-verify`, `decision`) cannot be answered from the TUI
- The session will block at checkpoints until manually answered
- **Mitigation:** Use `--permission-mode auto` and `workflow._auto_chain_active: true` to auto-approve most checkpoints

## Integration Strategies

### Strategy A: Per-Item Isolated Execution

**How it works:** Each QUEUE.md item spawns a separate `claude -p` process. Items execute sequentially. No session continuity between items.

**Trigger mechanism:**
1. User selects queue item and presses Execute key
2. TUI spawns: `claude -p "<command>" --output-format stream-json --allowedTools "Bash,Read,Edit,Write" --permission-mode auto`
3. Process runs in project directory
4. TUI monitors stdout stream + filesystem changes

**Session lifecycle:**
- Fresh session per item
- No `--continue` or `--resume`
- Session ID captured but not reused

**Artifact flow:**
- GSD writes STATE.md, ROADMAP.md, SUMMARY.md, VERIFICATION.md as usual
- TUI's existing file watcher detects changes and refreshes display
- On process exit, TUI re-reads project state to update dashboard

**Error handling:**
- Non-zero exit code -> mark item failed, show error in TUI
- Timeout (configurable, e.g., 30 min) -> SIGTERM, then SIGKILL after 10s
- Parse stderr for error details

**Pros:**
- Simple to implement
- Each item is isolated -- failure doesn't cascade
- Easy to retry individual items
- Clear lifecycle: start, run, done/failed
- Works with any LLM backend that has a CLI

**Cons:**
- No context continuity between items -- each starts fresh
- Higher overhead (Claude Code startup per item)
- Cannot answer checkpoints from TUI (session blocks)
- GSD skill resolution requires non-bare mode (slower startup)

### Strategy B: Persistent Session with Chaining

**How it works:** First queue item starts a session. Subsequent items use `--continue` or `--resume <session_id>` to maintain context.

**Trigger mechanism:**
1. User initiates "Execute Queue" (batch mode)
2. First item: `claude -p "<command>" --output-format json --name "queue-<project>" --allowedTools "..."`
3. Capture `session_id` from JSON output
4. Next item: `claude -p "<command>" --resume "$session_id" --output-format json`

**Session lifecycle:**
- Single session spans multiple queue items
- Context accumulates (LLM remembers previous work)
- Session persists to disk (can resume after TUI restart)

**Artifact flow:**
- Same filesystem-based flow as Strategy A
- Additionally: session context includes previous decisions and work

**Error handling:**
- If item fails, TUI can `--resume` with a diagnostic prompt
- Session can be abandoned and restarted fresh
- Budget cap via `--max-budget-usd`

**Pros:**
- Context continuity -- GSD session knows what was done in previous items
- Lower overhead after first item (reuse warm session)
- Closer to how `/gsd:autonomous` works internally

**Cons:**
- More complex state management (session ID tracking, recovery)
- Context window fills up over multiple items -- may hit limits
- Single failure can corrupt session state for subsequent items
- `--resume` may not work if session was corrupted
- Recovery from mid-queue failure is more complex

### Recommended Strategy: A (Per-Item Isolated) with Optional B (Chaining)

**Recommendation: Start with Strategy A for v1.3, with Strategy B as a future enhancement.**

Rationale:
1. Strategy A is dramatically simpler to implement and debug
2. Each queue item is already a self-contained GSD command (e.g., `/gsd:execute-phase 4`)
3. GSD commands are designed to be invoked independently -- they read state from disk, not from session context
4. Isolation means a failed item doesn't corrupt the queue
5. Strategy B can be layered on top as an optimization once A proves stable

## Safety Requirements

### Timeout Limits
| Scope | Default | Configurable | Enforcement |
|-------|---------|--------------|-------------|
| Per queue item | 30 minutes | Yes (TUI settings) | SIGTERM + SIGKILL after 10s grace |
| Per session (Strategy B) | 2 hours | Yes | Kill process |
| Cost budget | $5 per item | Yes | `--max-budget-usd` flag |

### Error Handling
| Condition | Detection | Response |
|-----------|-----------|----------|
| Process exits non-zero | Exit code | Mark item failed, show error, offer retry |
| Process hangs (no output) | Idle timer (5 min no filesystem change + no stdout) | Warn user, offer kill |
| WAITING.json appears | File watcher | Show checkpoint in TUI, pause queue |
| Disk full / write error | GSD error in stderr | Abort, surface error |
| Claude API error (rate limit, auth) | stderr / stream-json retry events | Surface to user, pause queue |

### Human Escalation Triggers
- Any `checkpoint:human-action` (auth gates) -- cannot be automated
- Any `checkpoint:decision` -- unless auto-advance is enabled
- Verification failures with `gaps_found` status
- Two consecutive item failures
- Cost budget exceeded

### Max Retry Counts
| Scope | Max Retries | Behavior |
|-------|-------------|----------|
| Per queue item | 1 | Retry once on failure, then mark failed |
| Per checkpoint | 0 | Never auto-retry checkpoints, always escalate |
| Queue-wide | 3 total failures | Pause queue execution after 3 failures |

### Runaway Loop Prevention
- Queue items are consumed (removed from QUEUE.md) only after successful completion
- Failed items stay in queue but are marked with failure state
- Queue execution stops on first failure (user must acknowledge)
- `/gsd:autonomous` already has built-in loop prevention (1 gap-closure retry max)
- The TUI should never auto-generate new queue items -- only user-created items execute

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GSD state parsing | Custom STATE.md parser | `gsd-tools.cjs state json`, `init phase-op`, `roadmap analyze` | GSD tools handle format changes; the TUI already uses the Rust parser but should validate against gsd-tools output |
| Session management | Custom session tracking | `claude --output-format json` session_id + `--resume` | Claude Code manages sessions natively |
| Process supervision | Custom daemon/supervisor | `tokio::process::Command` with timeout | Standard async process management |
| Checkpoint communication | Custom IPC protocol | `WAITING.json` file watcher + `gsd-tools.cjs state signal-resume` | GSD's existing external-watcher mechanism |

## Common Pitfalls

### Pitfall 1: Trying to Inject Input into Running Claude Sessions
**What goes wrong:** Attempting to pipe user answers into a running `claude -p` process via stdin
**Why it happens:** Natural assumption that stdin stays open for interaction
**How to avoid:** Accept that `claude -p` is fire-and-forget. Use `--permission-mode auto` for unattended execution. For checkpoints requiring human input, stop queue execution and let user interact directly.
**Warning signs:** Exploring `--input-format stream-json` for bidirectional communication

### Pitfall 2: Using `--bare` Mode for GSD Commands
**What goes wrong:** GSD commands fail because CLAUDE.md, hooks, and skills don't load
**Why it happens:** `--bare` is recommended for CI scripts, seems like a good fit
**How to avoid:** Never use `--bare` for GSD command execution. GSD skills, hooks, and CLAUDE.md context are all required.
**Warning signs:** "Skill not found" or "Unknown command" errors

### Pitfall 3: Assuming LLM-Specific CLI Flags
**What goes wrong:** Design hardcodes `claude -p` flags, breaking LLM-agnosticism
**Why it happens:** Claude Code is the only current backend
**How to avoid:** Abstract CLI invocation behind a configurable "executor" interface. The design document should specify the interface, not the implementation.
**Warning signs:** Direct references to `claude` binary in core execution logic

### Pitfall 4: Not Handling Concurrent Execution on Same Project
**What goes wrong:** Two queue items modify the same project simultaneously, causing git conflicts
**Why it happens:** User triggers execution while a previous item is still running
**How to avoid:** Enforce single-execution-per-project in the TUI. Use a lock file or process tracking.
**Warning signs:** Git merge conflicts, corrupted STATE.md

### Pitfall 5: Ignoring GSD's Own Auto-Advance
**What goes wrong:** Queue execution conflicts with GSD's internal `workflow.auto_advance` or autonomous mode
**Why it happens:** GSD may try to advance to the next phase internally while the TUI is about to dispatch the next queue item
**How to avoid:** Use `--no-transition` flag when executing phases, or ensure `workflow.auto_advance` is false during queue execution
**Warning signs:** Duplicate phase execution, skipped queue items

## Architecture Patterns

### Recommended Design Document Structure
```
QUEUE-EXECUTION-DESIGN.md
  1. Executive Summary
  2. GSD Autonomous Mode Lifecycle (QRES-01)
  3. Hook Points and Integration Surfaces
  4. CLI Capabilities Matrix
  5. Strategy A: Per-Item Isolated Execution (detailed)
  6. Strategy B: Session Chaining (detailed)
  7. Strategy Comparison and Recommendation
  8. Safety Requirements
  9. TUI Integration Points (existing code mapping)
  10. LLM-Agnostic Abstraction Layer
  11. Open Questions and Future Work
```

### LLM-Agnostic Executor Interface
The design should define an abstract executor interface:

```
Executor {
  fn start(project_dir, command, options) -> ExecutionHandle
  fn cancel(handle) -> Result
  fn is_running(handle) -> bool
}

ExecutionHandle {
  fn session_id() -> Option<String>
  fn exit_status() -> Option<ExitStatus>
  fn stdout_stream() -> Stream<Line>
}

ExecutionOptions {
  timeout: Duration,
  budget_usd: Option<f64>,
  auto_approve: bool,
  model: Option<String>,
}
```

This interface can be implemented for `claude -p`, future `codex -p`, `aider`, or any LLM CLI.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Claude Code headless mode | Agent SDK (same CLI, rebranded) | 2026 Q1 | `-p` flag unchanged, SDK adds Python/TypeScript wrappers |
| No external watcher support | WAITING.json signal mechanism | GSD recent (#1034) | Enables external orchestrators (like this TUI) to detect checkpoints |
| Manual session management | `--resume <id>` + `--output-format json` | Claude Code 2.x | Programmatic session continuity |

## Open Questions

1. **Bidirectional checkpoint communication**
   - What we know: GSD writes WAITING.json, TUI can detect it via file watcher
   - What's unclear: How to feed user's answer back into the running Claude session without `--input-format stream-json` piped from launch
   - Recommendation: For v1.3, stop queue on checkpoint and let user interact with Claude directly. Future: explore stdin streaming or a response file GSD could watch.

2. **Process management across TUI restarts**
   - What we know: `claude -p` processes are independent; session IDs persist
   - What's unclear: How to re-attach to a running claude process if TUI crashes and restarts
   - Recommendation: Track PIDs in a `.planning/queue-state.json` file. On TUI restart, check if PID still alive, and if so, resume monitoring.

3. **GSD skill resolution in headless mode**
   - What we know: Skills resolve via `/skill-name` even in bare mode (per official docs). Non-bare mode loads all context.
   - What's unclear: Whether `/gsd:execute-phase 4` works reliably as a prompt to `claude -p` (skills are typically invoked interactively)
   - Recommendation: Test this empirically during v1.3 implementation. Fallback: use `--append-system-prompt-file` to inject the workflow markdown directly.

4. **Cost visibility**
   - What we know: `--max-budget-usd` caps spending; `--output-format json` returns usage metadata
   - What's unclear: Whether usage data is available in stream-json events for real-time cost display
   - Recommendation: Parse `stream-json` output for usage events; fall back to final JSON response.

## Sources

### Primary (HIGH confidence)
- `~/.claude/get-shit-done/workflows/autonomous.md` -- full autonomous mode lifecycle (860 lines, read completely)
- `~/.claude/get-shit-done/workflows/execute-phase.md` -- phase execution with waves, subagents, verification
- `~/.claude/get-shit-done/workflows/execute-plan.md` -- plan execution, checkpoints, deviation rules
- `~/.claude/get-shit-done/references/checkpoints.md` -- checkpoint types and protocols
- `~/.claude/get-shit-done/bin/gsd-tools.cjs` -- CLI tool commands and state management
- `~/.claude/get-shit-done/bin/lib/state.cjs` -- WAITING.json signal-waiting/signal-resume implementation
- `claude --help` output (v2.1.87) -- verified all CLI flags and options
- [Claude Code headless docs](https://code.claude.com/docs/en/headless) -- `-p`, `--continue`, `--resume`, `--output-format`, `--bare` (verified 2026-03-31)
- `src/state_reader/queue_md.rs` -- existing QueuedAction struct and QUEUE.md parser
- `src/ui/screens/detail.rs` -- existing Queue tab implementation
- `src/action.rs` -- existing Action enum for TUI events

### Secondary (MEDIUM confidence)
- GSD hooks source (`hooks/gsd-context-monitor.js`, `hooks/gsd-workflow-guard.js`) -- hook architecture patterns
- `~/.claude/get-shit-done/workflows/quick.md`, `do.md`, `next.md`, `fast.md` -- additional workflow entry points
- `~/.claude/get-shit-done/workflows/transition.md` -- internal phase transition workflow
- `~/.claude/get-shit-done/workflows/pause-work.md` -- HANDOFF.json structure

## Metadata

**Confidence breakdown:**
- GSD autonomous lifecycle: HIGH -- direct source reading, complete coverage
- CLI capabilities: HIGH -- verified against `claude --help` and official docs
- Hook points: HIGH -- direct source reading of hooks and gsd-tools
- Integration strategies: HIGH -- based on verified capabilities, proven patterns
- Safety requirements: MEDIUM -- based on GSD's existing patterns, some assumptions about timeout behavior
- LLM-agnostic abstraction: MEDIUM -- conceptual design, not yet validated

**Research date:** 2026-03-31
**Valid until:** 2026-04-30 (stable -- GSD and Claude Code CLIs change incrementally)
