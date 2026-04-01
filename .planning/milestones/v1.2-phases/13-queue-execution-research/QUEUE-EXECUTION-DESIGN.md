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

---

## 6. Strategy A: Per-Item Isolated Execution

Each QUEUE.md item spawns a separate headless LLM CLI process. Items execute sequentially. No session continuity between items.

### Trigger Mechanism

1. User selects a queue item in the TUI and presses the Execute key (or selects "Execute Next" to run the top item).
2. TUI performs a pre-flight check via `gsd-tools.cjs` (e.g., `state load`, `init phase-op`) to validate that the project is in a state where the command can run.
3. TUI spawns the executor process in the target project's directory with appropriate flags for headless, autonomous operation and output streaming.
4. TUI enters "executing" state for that project -- the dashboard shows an execution indicator.

### Session Lifecycle

- A **fresh session** is created for each queue item. No `--continue` or `--resume` flags are used.
- The session ID is captured from structured output but is only used for diagnostic purposes (logging, identifying the session in the LLM CLI's session list).
- The session ends when the process exits (success or failure).
- If the process is interrupted (TUI crash, user cancel), the session may be resumed later via `--resume <id>` as a manual recovery step, but this is not part of the automated flow.

### Artifact Flow

1. GSD writes STATE.md, ROADMAP.md, SUMMARY.md, and VERIFICATION.md as usual during execution.
2. The TUI's existing file watcher (notify-debouncer-full) detects these changes and refreshes the project state display in real time.
3. On process exit, the TUI performs a full re-read of the project state to update the dashboard.
4. The queue item is marked as "done" in QUEUE.md only after successful completion (non-zero exit leaves it in the queue with a failure annotation).

### Error Handling

| Condition | Detection | Response |
|-----------|-----------|----------|
| Process exits with code 0 | Exit code check | Mark item done, update dashboard, advance to next item if batch mode |
| Process exits non-zero | Exit code check | Mark item failed, show error details in TUI, pause queue |
| Process exceeds timeout | Timer (configurable, default 30 min) | Send SIGTERM, wait 10s grace, then SIGKILL; mark item timed out |
| WAITING.json appears | File watcher | Show checkpoint details in TUI, pause queue execution |
| No output for extended period | Idle timer (5 min no filesystem change and no stdout) | Warn user, offer option to kill |

### Pros

- **Simple to implement:** One process per item, clear lifecycle (start, run, done/failed).
- **Fault isolation:** A failed item does not affect subsequent items. Each starts with a clean session.
- **Easy retry:** Re-running a failed item is trivial -- just spawn another process.
- **LLM-agnostic:** Any CLI that accepts a prompt and exits works with this pattern.
- **Predictable resource usage:** One process at a time, bounded by timeout.

### Cons

- **No context continuity:** Each item starts fresh. The LLM does not remember what was done in previous items.
- **Higher startup overhead:** Each process must initialize the LLM session, load CLAUDE.md, resolve skills, and parse project state.
- **Cannot answer checkpoints from TUI:** A running headless session cannot receive user input injected after launch.

**Confidence:** HIGH -- based on verified CLI capabilities and standard process management patterns.

---

## 7. Strategy B: Persistent Session with Chaining

The first queue item starts a session. Subsequent items use `--resume <session_id>` to continue in the same session context, maintaining LLM memory across items.

### Trigger Mechanism

1. User initiates "Execute Queue" (batch mode) in the TUI.
2. First item is spawned as a fresh headless process with structured output to capture the session ID.
3. On successful completion, the TUI extracts `session_id` from the output.
4. Next item is spawned with `--resume <session_id>` to continue in the same session context.
5. This continues until the queue is empty or an error occurs.

### Session Lifecycle

- A **single session** spans multiple queue items through `--resume` chaining.
- Context accumulates -- the LLM remembers decisions, code changes, and project state from previous items.
- The session persists to disk (via the LLM CLI's session storage), so it can survive TUI restarts.
- If an item fails, the TUI can attempt to `--resume` with a diagnostic prompt ("The previous command failed with: ...").
- The session can be abandoned at any time and a fresh one started.

### Artifact Flow

- Same filesystem-based flow as Strategy A -- GSD writes artifacts, TUI watches them.
- Additionally, the session context itself contains accumulated knowledge about what was done, providing richer error messages and more informed execution in later items.

### Error Handling

| Condition | Detection | Response |
|-----------|-----------|----------|
| Item succeeds | Exit code 0 + session_id in output | Chain to next item via `--resume` |
| Item fails | Non-zero exit | Attempt `--resume` with diagnostic prompt (1 retry), then pause |
| Session corrupted | `--resume` fails with error | Abandon session, restart fresh (fallback to Strategy A behavior) |
| Context window exhausted | LLM reports context limit | Start new session for remaining items (implicit fallback to A) |
| Budget exceeded | `--max-budget-usd` enforcement | Pause queue, surface cost info to user |

### Pros

- **Context continuity:** The LLM remembers previous work, leading to more coherent multi-step execution.
- **Lower overhead after first item:** Reusing a warm session avoids repeated initialization.
- **Closer to /gsd:autonomous pattern:** Mirrors how GSD's internal autonomous mode chains phases.

### Cons

- **More complex state management:** Session ID tracking, recovery from corrupted sessions, fallback logic.
- **Context window pressure:** Accumulated context from multiple items may exhaust the context window, causing degraded performance or failures.
- **Failure cascade risk:** A corrupted session state can affect all subsequent items in the chain.
- **Recovery complexity:** Determining where to resume after a mid-chain failure requires understanding what the session has already done.
- **LLM-specific:** Not all LLM CLIs support session resumption. This strategy is less portable.

**Confidence:** HIGH -- based on verified `--resume` and `--continue` capabilities, with caveats about context window limits.

---

## 8. Strategy Comparison and Recommendation

### Side-by-Side Comparison

| Dimension | Strategy A (Per-Item Isolated) | Strategy B (Session Chaining) |
|-----------|-------------------------------|-------------------------------|
| Implementation complexity | Low | Medium-High |
| Fault isolation | Full -- each item independent | Partial -- session state shared |
| Context continuity | None | Full (within context window) |
| Startup overhead | Per-item (5-15s each) | Once (first item only) |
| Retry simplicity | Trivial (re-spawn) | Complex (resume or restart) |
| LLM-agnosticism | High (any CLI works) | Low (requires session resume) |
| Context window risk | None | High (accumulates per item) |
| Recovery from TUI crash | Simple (track PID) | Complex (track PID + session ID) |
| Checkpoint handling | Stop queue, user interacts directly | Same limitation |
| Cost predictability | High (per-item budget cap) | Lower (shared budget across items) |

### Recommendation

**Implement Strategy A for v1.3. Design Strategy B as a future enhancement (v1.4+).**

Rationale:

1. **Strategy A is dramatically simpler to implement and debug.** The lifecycle is start-run-done with no shared state between items.
2. **GSD commands are designed to be invoked independently.** Each command reads state from disk (STATE.md, ROADMAP.md, project files), not from session context. Context continuity provides marginal benefit.
3. **Each queue item is already a self-contained command.** Users write items like `/gsd:execute-phase 4` or `/gsd:quick "fix the login bug"` -- these are complete instructions that do not depend on previous session state.
4. **Isolation means a failed item does not corrupt the queue.** With Strategy B, a corrupted session can block all remaining items.
5. **Strategy B can be layered on top once Strategy A proves stable.** The executor interface (Section 11) supports both strategies. Adding `--resume` chaining is an incremental enhancement, not a redesign.

**Confidence:** HIGH -- the recommendation aligns with the principle of starting simple and adding complexity only when validated by usage.

---

## 9. Safety Requirements

### Timeout Limits

| Scope | Default | Configurable | Enforcement Mechanism |
|-------|---------|--------------|----------------------|
| Per queue item | 30 minutes | Yes (TUI settings) | SIGTERM to process, then SIGKILL after 10s grace period |
| Per session (Strategy B) | 2 hours | Yes (TUI settings) | Kill process, abandon session |
| Cost budget per item | $5 | Yes (TUI settings) | `--max-budget-usd` flag passed to LLM CLI |

### Error Handling

| Condition | Detection Method | TUI Response |
|-----------|-----------------|--------------|
| Process exits non-zero | Exit code from `tokio::process::Command` | Mark item failed, show error in status bar, offer retry |
| Process hangs (no progress) | Idle timer: 5 minutes with no filesystem change and no stdout output | Warn user with a notification, offer kill option |
| WAITING.json appears | File watcher event on `.planning/WAITING.json` | Show checkpoint question/options in TUI, pause queue, await user |
| Disk full / write error | GSD error output in stderr or stream-json | Abort item, surface error, pause queue |
| LLM API error (rate limit, auth failure) | stderr content or stream-json error events | Surface specific error to user, pause queue |
| Git conflict during execution | Non-zero exit with git error in output | Mark failed, show conflict details, do not retry automatically |

### Human Escalation Triggers

The following conditions **must** pause queue execution and require human attention:

- **`checkpoint:human-action`** in WAITING.json -- authentication gates, 2FA codes, or other actions that cannot be automated.
- **`checkpoint:decision`** in WAITING.json -- architectural or implementation decisions, unless `workflow.auto_advance` is enabled in the project's GSD config.
- **Verification failures** with `gaps_found` status in VERIFICATION.md -- the phase did not pass verification and needs human judgment on whether to retry, skip, or fix.
- **Two consecutive item failures** -- suggests a systemic issue (environment problem, missing dependency, broken project state) rather than an isolated failure.
- **Cost budget exceeded** -- the `--max-budget-usd` cap was hit, indicating the item is more expensive than expected.

### Max Retry Counts

| Scope | Max Retries | Behavior After Exhaustion |
|-------|-------------|---------------------------|
| Per queue item | 1 | Retry once on failure, then mark as permanently failed |
| Per checkpoint | 0 | Never auto-retry checkpoints; always escalate to human |
| Queue-wide cumulative failures | 3 | Pause entire queue execution after 3 total failures across any items |

### Runaway Loop Prevention

These rules prevent the TUI from entering an uncontrolled execution loop:

1. **Consume-on-success only:** Queue items are removed from QUEUE.md (or marked done) only after successful completion. Failed items remain in the queue with a failure annotation.
2. **Stop-on-first-failure:** Queue execution stops on the first failure. The user must acknowledge the failure before execution can resume.
3. **No auto-generation:** The TUI must never auto-generate new queue items. Only user-created items execute. This prevents a feedback loop where execution creates more work that triggers more execution.
4. **Single-execution-per-project:** The TUI enforces that only one queue item can be executing per project at any time. A lock mechanism (tracked PID or state flag) prevents concurrent execution.
5. **Respect GSD's own loop prevention:** GSD's autonomous mode already has built-in loop prevention (1 gap-closure retry max). The TUI should not override this by retrying externally.

**Confidence:** MEDIUM -- timeout values and retry counts are reasonable defaults based on GSD's existing patterns, but may need tuning based on real-world usage. The escalation triggers are well-defined.

---

## 10. TUI Integration Points

### Existing Codebase Components

These components in the current TUI codebase are relevant to queue execution and would need modification or extension:

| Component | Location | Current Role | Queue Execution Extension |
|-----------|----------|-------------|---------------------------|
| `QueuedAction` struct | `src/state_reader/queue_md.rs` | Stores command string, description, priority | Add execution state (pending, running, done, failed), retry count, last error, session ID |
| Queue tab CRUD | `src/ui/screens/detail.rs` | Add, edit, delete, reorder, mark done | Add "Execute" action, execution status display, error detail view |
| `suggest_next_commands()` | `src/state_reader/queue_md.rs` | Context-aware command suggestions | Validate suggested commands against executable patterns |
| `ScreenAction::DispatchAction` | `src/ui/screens/mod.rs` | Dispatch async actions through event channel | Route execution requests from Queue tab to executor |
| `Action` enum | `src/action.rs` | TUI action types for event loop | Add variants: `ExecuteQueueItem`, `CancelExecution`, `RetryQueueItem`, `ExecutionCompleted`, `ExecutionFailed`, `CheckpointDetected` |
| File watcher | `src/watcher.rs` | Watches `.planning/` for state changes | Add WAITING.json to watched paths (likely already covered by directory watch) |
| `ProjectState` | `src/state_reader/mod.rs` | Parsed project state including `queued_actions` | Add `execution_state` field tracking active execution per project |

### New Components Needed

| Component | Purpose | Location (suggested) |
|-----------|---------|---------------------|
| `Executor` trait | LLM-agnostic execution interface | `src/executor/mod.rs` |
| `ClaudeExecutor` | Claude Code implementation of Executor | `src/executor/claude.rs` |
| `ExecutionState` | Per-project execution tracking (PID, status, session ID) | `src/executor/state.rs` |
| `ExecutionConfig` | Timeout, budget, retry settings | `src/config.rs` (extend existing) |
| Execution status panel | Show running item, progress, output | `src/ui/screens/detail.rs` (new tab or overlay) |

**Confidence:** HIGH -- based on direct reading of the existing codebase during prior phases.

---

## 11. LLM-Agnostic Abstraction Layer

The queue execution design must not assume a specific LLM backend. This section defines the abstract interfaces that decouple the execution logic from any particular CLI tool.

### Executor Interface

```
trait Executor {
    /// Spawn a headless execution of the given command in the project directory.
    /// Returns a handle for monitoring and controlling the execution.
    fn start(project_dir: PathBuf, command: String, options: ExecutionOptions) -> Result<ExecutionHandle>

    /// Request cancellation of a running execution.
    fn cancel(handle: &ExecutionHandle) -> Result<()>

    /// Check if the execution is still running.
    fn is_running(handle: &ExecutionHandle) -> bool
}
```

### ExecutionHandle

```
struct ExecutionHandle {
    /// Unique identifier for this execution (maps to OS process ID internally).
    id: ExecutionId,

    /// Session identifier from the LLM CLI, if available. Used for resume/chaining.
    session_id: Option<String>,

    /// Channel for receiving execution events (output lines, completion, errors).
    events: Receiver<ExecutionEvent>,
}

enum ExecutionEvent {
    Output(String),              // A line of output from the process
    Progress(ProgressUpdate),    // Parsed progress information (if available)
    Checkpoint(CheckpointInfo),  // WAITING.json detected
    Completed(ExitStatus),       // Process exited
    Error(String),               // Error detected in output
}
```

### ExecutionOptions

```
struct ExecutionOptions {
    /// Maximum time the execution can run before being killed.
    timeout: Duration,

    /// Maximum cost budget (passed to LLM CLI if supported).
    budget_usd: Option<f64>,

    /// Whether to auto-approve tool usage and checkpoints.
    auto_approve: bool,

    /// Model to use (if the LLM CLI supports model selection).
    model: Option<String>,

    /// Whether to use session chaining (Strategy B). If Some, contains the session ID to resume.
    resume_session: Option<String>,

    /// Human-readable name for the execution session.
    name: Option<String>,
}
```

### Implementation Notes

The `Executor` trait can be implemented for different backends:

- **Claude Code:** Uses `claude -p` with `--output-format stream-json`, `--permission-mode auto`, `--allowedTools`, `--max-budget-usd`, `--name`. Parses stream-json for real-time events.
- **Future Codex CLI:** Would use the equivalent headless invocation with codex-specific flags.
- **Aider:** Uses `aider --yes` with appropriate flags for non-interactive mode.
- **Generic CLI:** Any tool that accepts a prompt on the command line and produces output on stdout.

The key constraint is that the executor interface is **process-based**: it spawns a subprocess, monitors its output and filesystem side effects, and reports completion. This is the lowest common denominator across all LLM CLI tools.

**Confidence:** MEDIUM -- the interface design is sound and based on real CLI capabilities, but has not been validated against non-Claude backends. The generic process model should work for any CLI tool, but specific implementations may need adjustments.

---

## 12. Open Questions and Future Work

### 12.1. Bidirectional Checkpoint Communication

**What we know:** GSD writes WAITING.json when it needs user input. The TUI can detect this via its file watcher and display the question to the user.

**What is unclear:** How to feed the user's answer back into a running `claude -p` session. There is no stdin injection mechanism for a headless process after it has launched. The `--input-format stream-json` flag requires stdin to be piped from launch, not injected mid-execution.

**Recommendation:** For v1.3, stop queue execution when WAITING.json appears and let the user interact with the Claude session directly (via terminal). Future work could explore: (a) a response file that GSD watches (`.planning/WAITING-RESPONSE.json`), (b) a named pipe or socket for IPC, or (c) enhancements to `gsd-tools.cjs` to support a `signal-answer` command.

### 12.2. Process Management Across TUI Restarts

**What we know:** `claude -p` processes are independent OS processes. Session IDs persist in the LLM CLI's session storage. The TUI can track PIDs.

**What is unclear:** How to re-attach monitoring to a running process if the TUI crashes and restarts. The TUI would need to discover orphaned execution processes and resume monitoring their output and filesystem changes.

**Recommendation:** Track execution state in a `.planning/queue-state.json` file containing PID, session ID, start time, and the command being executed. On TUI restart, check if the PID is still alive (via `/proc/<pid>/` on Linux). If alive, resume filesystem monitoring (output stream is lost but artifacts still update). If dead, check exit status and update queue item accordingly.

### 12.3. GSD Skill Resolution in Headless Mode

**What we know:** GSD skills resolve via `/skill-name` syntax even in non-bare headless mode. Non-bare mode loads CLAUDE.md, hooks, and all skill context.

**What is unclear:** Whether `/gsd:execute-phase 4` works reliably as a prompt to `claude -p`. GSD skills are typically invoked interactively within a Claude Code session. Headless invocation of complex multi-step workflows (which internally spawn subagents via `Task()`) may behave differently.

**Recommendation:** This must be tested empirically during v1.3 implementation. As a fallback, use `--append-system-prompt-file` to inject the workflow markdown directly, bypassing skill resolution. The simplest test: `claude -p "/gsd:next" --output-format json` in a GSD project directory.

### 12.4. Cost Visibility

**What we know:** `--max-budget-usd` caps spending per session. `--output-format json` returns usage metadata in the final response. The context monitor bridge file (`/tmp/claude-ctx-{session_id}.json`) contains some metrics.

**What is unclear:** Whether token usage and cost data are available in `stream-json` events for real-time cost display during execution, or only in the final output.

**Recommendation:** Parse `stream-json` output for usage/cost events during execution. Fall back to extracting cost from the final JSON response. Display cumulative cost per queue item and per session in the TUI's execution status panel.

---

## 13. Sources

### Primary (HIGH confidence)

| Source | What it covers |
|--------|---------------|
| `~/.claude/get-shit-done/workflows/autonomous.md` | Full autonomous mode lifecycle (860 lines) |
| `~/.claude/get-shit-done/workflows/execute-phase.md` | Phase execution with waves, subagents, verification |
| `~/.claude/get-shit-done/workflows/execute-plan.md` | Plan execution, checkpoints, deviation rules |
| `~/.claude/get-shit-done/references/checkpoints.md` | Checkpoint types and protocols |
| `~/.claude/get-shit-done/bin/gsd-tools.cjs` | CLI tool commands and state management |
| `~/.claude/get-shit-done/bin/lib/state.cjs` | WAITING.json signal-waiting/signal-resume implementation |
| `claude --help` output (v2.1.87) | All CLI flags and options verified |
| [Claude Code headless docs](https://code.claude.com/docs/en/headless) | `-p`, `--continue`, `--resume`, `--output-format`, `--bare` |
| `src/state_reader/queue_md.rs` | Existing QueuedAction struct and QUEUE.md parser |
| `src/ui/screens/detail.rs` | Existing Queue tab implementation |
| `src/action.rs` | Existing Action enum for TUI events |

### Secondary (MEDIUM confidence)

| Source | What it covers |
|--------|---------------|
| GSD hooks (`hooks/gsd-context-monitor.js`, `hooks/gsd-workflow-guard.js`) | Hook architecture patterns |
| `~/.claude/get-shit-done/workflows/quick.md`, `do.md`, `next.md`, `fast.md` | Additional workflow entry points |
| `~/.claude/get-shit-done/workflows/transition.md` | Internal phase transition workflow |
| `~/.claude/get-shit-done/workflows/pause-work.md` | HANDOFF.json structure |
