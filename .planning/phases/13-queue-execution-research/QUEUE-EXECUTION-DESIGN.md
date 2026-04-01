# Queue Execution Design Document

**Phase 13 Deliverable** | Research date: 2026-03-31 | Valid until: 2026-04-30

---

## 1. Executive Summary

This document captures the complete design for enabling the GSD Meta Manager TUI to execute queued workflow items from QUEUE.md. It is the sole deliverable of Phase 13 and is intended to be self-contained: a v1.3 implementer should need no further research to begin building queue execution.

The TUI currently manages a queue of GSD commands per project (add, edit, delete, reorder, mark done) but cannot execute them. Queue execution bridges this gap by spawning headless LLM CLI processes that run GSD workflows, monitoring progress through filesystem artifacts, and surfacing results back to the TUI dashboard.

The recommended approach is **Strategy A: Per-Item Isolated Execution**, where each QUEUE.md item spawns a separate headless process. This strategy is simpler to implement, provides natural fault isolation, and aligns with how GSD commands are designed -- each command reads state from disk and operates independently. Strategy B (persistent session chaining) is documented as a future optimization once Strategy A proves stable.

A key architectural constraint is **LLM-agnosticism**. While GSD currently runs on Claude Code, this design abstracts the execution interface behind an `Executor` trait so that future backends (Codex, aider, or any LLM CLI) can be substituted without changing the queue execution core. The design specifies interfaces and integration points, not Claude-specific implementation details.

The document covers: GSD's autonomous mode lifecycle and how it relates to queue execution, the hook and signal mechanisms available for external orchestration, CLI capabilities for headless invocation, two integration strategies with full trigger/lifecycle/artifact/error analysis, safety requirements with concrete limits, TUI integration points mapped to the existing codebase, and open questions for future work.

---

## 2. GSD Autonomous Mode Lifecycle

GSD's autonomous workflow (`autonomous.md`, 860+ lines) is the closest existing pattern to what queue execution needs. Understanding its lifecycle reveals how GSD chains phases and where external orchestration can integrate.

### The 6-Step Lifecycle

**Step 1 -- Initialize:** Parse the `--from N` flag, bootstrap via `gsd-tools.cjs init milestone-op`, validate that ROADMAP.md and STATE.md exist in the project's `.planning/` directory.

**Step 2 -- Discover Phases:** Call `gsd-tools.cjs roadmap analyze` which returns JSON with all phases. Filter to incomplete phases, sort by phase number.

**Step 3 -- Execute Each Phase:** For each incomplete phase, execute sequentially:
- **3a. Smart Discuss:** Check `has_context` via `init phase-op`. If false and `skip_discuss` is not set, run interactive grey-area resolution to gather context. If true, skip. Can auto-generate a minimal CONTEXT.md.
- **3a.5. UI Phase:** If a frontend phase is detected and `workflow.ui_phase` is enabled, run the UI phase workflow.
- **3b. Plan:** Invoke `Skill(skill="gsd:plan-phase", args="${PHASE_NUM}")` to create execution plans.
- **3c. Execute:** Invoke `Skill(skill="gsd:execute-phase", args="${PHASE_NUM} --no-transition")` to execute all plans.
- **3d. Post-Execution Routing:** Read VERIFICATION.md status (`passed`, `human_needed`, or `gaps_found`) and route accordingly.
- **3d.5. UI Review:** If a UI-SPEC exists, perform an advisory review.

**Step 4 -- Iterate:** Re-read ROADMAP.md after each phase (catches dynamically inserted decimal phases). Check STATE.md for blockers.

**Step 5 -- Lifecycle Completion:** After all phases complete: run `gsd:audit-milestone`, then `gsd:complete-milestone`, then `gsd:cleanup`.

**Step 6 -- Handle Blocker:** If a blocker is detected, present user choices: fix and retry, skip phase, or stop autonomous mode.

### Key Observations for Queue Execution

- Autonomous mode runs entirely within a **single Claude Code session** using internal `Skill()` invocations to chain workflows.
- The TUI **cannot call `Skill()`** -- it exists only inside Claude Code's prompt context as an internal mechanism.
- The TUI must use the **headless CLI** (`claude -p` or equivalent) to spawn GSD commands as external processes.
- Progress is tracked through **filesystem artifacts** (STATE.md, ROADMAP.md, VERIFICATION.md, SUMMARY.md) which the TUI already watches.
- The execute-phase workflow groups plans into **waves** with parallel execution via subagents. Each plan produces a SUMMARY.md upon completion.

**Confidence:** HIGH -- sourced from direct reading of the complete autonomous.md workflow (860 lines) and execute-phase.md.

---

## 3. Hook Points and Integration Surfaces

### GSD Hook System

GSD installs three hooks into Claude Code's hook system. These operate **inside** Claude Code sessions, not externally:

| Hook | Type | Purpose |
|------|------|---------|
| `gsd-context-monitor.js` | PostToolUse | Monitors context usage via a statusline bridge file; injects WARNING (<=35%) or CRITICAL (<=25%) alerts when context is low |
| `gsd-workflow-guard.js` | PreToolUse | Soft guard that warns when file edits occur outside a GSD workflow; checks Write/Edit tool calls to non-`.planning/` files |
| `gsd-statusline.js` | Statusline | Writes metrics to `/tmp/claude-ctx-{session_id}.json` for the context monitor to read |

**Relevance to queue execution:** The TUI does not interact with hooks directly. However, the context monitor's bridge file (`/tmp/claude-ctx-{session_id}.json`) could be read by the TUI to display session context usage during execution. This is an optional enhancement, not a requirement.

### WAITING.json Signal Mechanism

GSD has a built-in signal file mechanism designed for external watchers. This is the **primary bidirectional communication channel** for external orchestrators like the TUI.

**Writing the signal (GSD -> external):**
```
gsd-tools.cjs state signal-waiting --type T --question Q --options "A|B" --phase P
```

This creates `.planning/WAITING.json` (or `.gsd/WAITING.json`) with the following structure:
```json
{
  "status": "waiting",
  "type": "decision_point",
  "question": "How should authentication be implemented?",
  "options": ["JWT tokens", "Session cookies"],
  "phase": "04-auth",
  "timestamp": "2026-03-31T12:00:00Z"
}
```

**Clearing the signal (external -> GSD):**
```
gsd-tools.cjs state signal-resume
```

This removes the WAITING.json file, signaling the GSD session to continue.

**Communication gap:** The current mechanism is **write-only from GSD's perspective**. There is no `signal-answer` command that feeds a user's chosen option back into the running session. The user must answer checkpoints in the Claude Code session directly. For headless `claude -p` execution, this means:

1. The TUI detects WAITING.json via its existing file watcher (notify-debouncer-full).
2. The TUI reads the question and options, presenting them to the user.
3. The running `claude -p` session **cannot receive the answer** -- there is no stdin injection mechanism for running headless sessions.
4. **Mitigation for v1.3:** Use `--permission-mode auto` and GSD's `workflow._auto_chain_active: true` to auto-approve most checkpoints. Stop queue execution on checkpoints that require genuine human input.

### GSD CLI Tools (gsd-tools.cjs)

The TUI can call `gsd-tools.cjs` **directly** (it is a Node.js CLI) to check project state without needing a Claude Code session:

| Command | Purpose | Queue Execution Use |
|---------|---------|---------------------|
| `init phase-op <N>` | Get phase state (has_context, has_plans, etc.) | Pre-flight check before executing a queue item |
| `roadmap analyze` | Full roadmap parse with disk status | Determine what is executable |
| `state json` | STATE.md frontmatter as JSON | Read current project position |
| `phase-plan-index <N>` | Plans with waves and completion status | Check if execution is needed |
| `state signal-waiting` | Write WAITING.json | GSD writes this; TUI reads it |
| `state signal-resume` | Remove WAITING.json | TUI calls this after user acknowledges |
| `state load` | Load project config + state | Pre-flight validation |

**Key insight:** Pre-flight validation and post-execution verification can be done entirely through `gsd-tools.cjs` without spawning an LLM session. This enables the TUI to validate queue items before execution and confirm results after.

**Confidence:** HIGH -- sourced from direct reading of gsd-tools.cjs command header and state.cjs implementation.

---

## 4. CLI Capabilities Matrix

The following CLI flags are relevant to queue execution. These are documented for the Claude Code CLI but the design abstracts them behind an executor interface (see Section 11).

| Flag | Purpose | Queue Execution Use |
|------|---------|---------------------|
| `-p` / `--print` | Non-interactive (headless) mode; prints response and exits | Primary execution mode for TUI-spawned GSD commands |
| `-c` / `--continue` | Continue most recent conversation in the current directory | Chain multiple queue items in the same session (Strategy B) |
| `-r` / `--resume <id>` | Resume a specific session by ID | Resume a queue execution session after interruption |
| `--output-format json` | Structured JSON output with session_id, result, metadata | Capture session_id for `--resume`; parse completion results |
| `--output-format stream-json` | Newline-delimited JSON streaming | Real-time progress monitoring during execution |
| `--allowedTools` | Auto-approve specific tools | `"Bash,Read,Edit,Write"` for autonomous execution |
| `--permission-mode` | Permission handling mode | `auto` for unattended execution |
| `--model <model>` | Select model | LLM-agnostic: user configures which model to use |
| `--bare` | Skip hooks, LSP, plugins, MCP, auto-memory, CLAUDE.md | Faster startup but **must not be used** for GSD commands (see below) |
| `--append-system-prompt` | Add text to system prompt | Inject GSD skill context for headless runs if needed |
| `--max-budget-usd` | Spending cap per session | Safety: prevent runaway costs |
| `--worktree` | Create a git worktree for the session | Isolation: each queue item gets its own worktree |
| `--no-session-persistence` | Do not save session to disk | Ephemeral runs that do not clutter the session list |
| `--name <name>` | Display name for the session | Label queue execution sessions for identification in logs |

### Critical Finding: Do Not Use --bare Mode

The `--bare` flag skips CLAUDE.md auto-discovery and hooks. GSD's skill resolution (`/gsd:*` commands) still works in bare mode, but hooks and CLAUDE.md context -- which contain project-specific constraints and workflow enforcement -- will not load. **For GSD queue execution, the TUI must not use `--bare` mode.**

### Session ID Chaining Example (Strategy B)

```bash
# Start first queue item, capture session_id from JSON output
session_id=$(claude -p "/gsd:execute-phase 4" --output-format json | jq -r '.session_id')

# Continue with next item in the same session context
claude -p "/gsd:plan-phase 5" --resume "$session_id" --output-format json
```

This enables Strategy B (persistent session chaining) where context accumulates across queue items. See Section 7 for the full strategy analysis.

**Confidence:** HIGH -- verified against `claude --help` output (v2.1.87) and official documentation at code.claude.com/docs/en/headless.

---

## 5. GSD Workflow Entry Points

These are the GSD commands that map to items users would place in QUEUE.md:

| Command | Workflow File | Typical Queue Use | Notes |
|---------|---------------|-------------------|-------|
| `/gsd:discuss-phase N` | `discuss-phase.md` | Gather context before planning | Interactive; may need user input |
| `/gsd:plan-phase N` | `plan-phase.md` | Create execution plans for a phase | Produces PLAN.md files |
| `/gsd:execute-phase N` | `execute-phase.md` | Execute all plans in a phase | Spawns subagents for parallel waves |
| `/gsd:verify-work N` | `verify-phase.md` | Verify phase completion | Produces VERIFICATION.md |
| `/gsd:autonomous` | `autonomous.md` | Run all remaining phases end-to-end | Full lifecycle; highest autonomy |
| `/gsd:quick <desc>` | `quick.md` | Small ad-hoc tasks | Self-contained; good queue candidate |
| `/gsd:fast <desc>` | `fast.md` | Trivial inline fixes | Minimal overhead |
| `/gsd:do <desc>` | `do.md` | Intent dispatcher (routes to best command) | GSD decides which workflow to use |
| `/gsd:next` | `next.md` | Auto-detect and advance to next step | Reads project state, routes accordingly |

### Highlight: /gsd:next for Simple Queue Execution

The `/gsd:next` command is particularly interesting for queue execution. It auto-detects the current project state and routes to the appropriate next action (discuss, plan, execute, or verify). A simple queue execution strategy could repeatedly invoke `/gsd:next` and let GSD determine what to do, rather than requiring the user to specify exact commands. However, this reduces user control over execution order and may not map cleanly to user-curated QUEUE.md items.

**Confidence:** HIGH -- sourced from direct reading of all referenced workflow files.
