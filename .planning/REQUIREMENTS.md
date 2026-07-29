# Requirements: v2.0 Autonomous Orchestration

**Milestone:** v2.0
**Defined:** 2026-07-28
**Source:** `.planning/research/SUMMARY.md` + backlog 999.2 / 999.3 + four pending UI todos

> **Scoping note.** The user directed that this milestone be scoped and run without
> further questions ("best well-reasoned guess is good enough"). Every scoping call
> below was made by the orchestrator from the research findings and the four locked
> decisions. Assumptions are marked **[ASSUMED]** so they can be corrected at the end.

## Locked Decisions

| Decision | Value | Source |
|---|---|---|
| Driver transport | `claude -p` drives; tmux is attach/watch only | User-locked; **rationale revised** — see below |
| Autonomy level | Fully autonomous, including `git push` and PR creation | User-locked |
| Hard requirements on autonomy | Kill switch + dry-run mode | User-locked |
| Container runtime | Auto-detect docker or podman | User-locked |
| Sequencing | 999.2 plumbing before 999.3 decision layer | User-locked |
| Version | v2.0 | User-locked |

**Revised rationale (not a reversal).** The locked decision was *"`claude -p` drives; tmux
pane for human watch/interject"* — chosen because `tmux send-keys` was believed to be the
only mid-run injection channel, at the cost of no delivery confirmation. All four
researchers independently found `--input-format stream-json` + `--replay-user-messages`,
a supported stdin injection channel **with** echo-back confirmation. The conclusion
(`-p` drives) is unchanged; the tmux dependency for *control* is obsolete. tmux is
retained as optional human attach ergonomics only. Using tmux as the control channel is
now an explicit anti-requirement — FEATURES.md notes driver and human keystrokes
interleave and corrupt each other.

## v2.0 Requirements

### Transport & Execution (TRANS)

- [ ] **TRANS-01**: The driver runs a GSD command by spawning `claude -p` with duplex `stream-json` over stdio, and parses the structured result envelope rather than scraping text
- [ ] **TRANS-02**: Run outcome is derived from the `type:"result"` envelope (`subtype`, `is_error`, `terminal_reason`, `permission_denials[]`), exit code, disk state, and git — never from the agent's prose summary
- [ ] **TRANS-03**: The TUI event loop reads input while a run is streaming output, so control keys stay responsive throughout a multi-hour run
- [ ] **TRANS-04**: The tool detects Claude CLI capabilities at runtime via `system/init` feature detection and refuses to start a run against an unsupported CLI, with a clear message
- [ ] **TRANS-05**: A user can execute a single GSD command against an opted-in project from the TUI and watch its output stream live (no autonomy yet — the manual ship point)

### Run Control & Safety Envelope (CTRL)

- [ ] **CTRL-01**: A user can stop any run at any time from the TUI, and stopping terminates the entire process tree including `claude`'s Bash grandchildren
- [ ] **CTRL-02**: A user can start any run in dry-run mode, which reports the GSD commands it would issue plus the resulting diffstat and push refspecs, without executing them
- [ ] **CTRL-03**: A project is driven only when explicitly opted in; non-opted-in projects are never spawned against, enforced at the process-spawn seam
- [ ] **CTRL-04**: An autonomous run survives the TUI being closed, and the TUI reconciles live runs on restart
- [ ] **CTRL-05**: Only one driver may execute against a given project at a time, enforced by an OS-level lock
- [ ] **CTRL-06**: A run halts itself when it stops making progress, repeats a command, exceeds a step cap, or exceeds a wall-clock cap
- [ ] **CTRL-07**: A run parks rather than retrying when it hits a Claude subscription rate limit, and reports which quota window blocked it

### Git & Blast Radius (SAFE)

- [ ] **SAFE-01**: The driver pushes only to a reserved branch namespace; pushes outside it are blocked by a mechanism the agent cannot talk its way past
- [ ] **SAFE-02**: Force-push and hook-bypass are blocked for driver-initiated git operations
- [ ] **SAFE-03**: Driver-initiated pushes are scanned for secrets and blocked on detection
- [ ] **SAFE-04**: Credentials and tokens are redacted when captured into the run log, not when rendered
- [ ] **SAFE-05**: A driven run uses a scoped git credential rather than inheriting the user's ambient credentials or SSH agent
- [ ] **SAFE-06**: PR creation is rate-capped per project per day
- [ ] **SAFE-07**: `.planning/` content read by the driver is passed to the model inside an explicit untrusted-content boundary, never concatenated into instructions
- [ ] **SAFE-08**: The model's chosen action is constrained to a fixed enum of GSD commands; free-form shell strings are never executed

### Run State & Observability (OBS)

- [ ] **OBS-01**: Every run writes an append-only journal to disk that survives process death and is the source of truth for run state
- [ ] **OBS-02**: The project list marks which projects are LLM-driven, and which are parked awaiting a human
- [ ] **OBS-03**: A user can read the originating goal prompt for any driven project, to understand what it was asked to do
- [ ] **OBS-04**: A user can watch a run's live output, current step, elapsed time, and step history from a TUI tab
- [ ] **OBS-05**: A user can review what a completed or failed run did, after the fact, including which commands ran and why it stopped
- [ ] **OBS-06**: Driver journal writes do not trigger full project re-parses, so a multi-hour run does not degrade TUI responsiveness
- [ ] **OBS-07**: A user can sort or filter the dashboard to surface projects that need human attention

### Steering (STEER)

- [ ] **STEER-01**: A user can inject a message into a running driver from the TUI
- [ ] **STEER-02**: An injected message shows its delivery state — queued, delivered, and acted upon — rather than being fire-and-forget
- [ ] **STEER-03**: Injected messages survive a TUI restart between queueing and delivery

### Autonomous Driver (DRIVE)

- [ ] **DRIVE-01**: A user states a goal once in natural language and the driver pursues it across multiple GSD commands without further input
- [ ] **DRIVE-02**: The driver chooses the next GSD command from observed project state using deterministic rules, not a model call, for every case the rules cover
- [ ] **DRIVE-03**: The driver's goal is decomposed into a structured, machine-checkable plan that the user can review before the run starts
- [ ] **DRIVE-04**: The driver escalates to a model only for goal decomposition and for ambiguity the rules cannot resolve, with a per-run cap on such escalations
- [ ] **DRIVE-05**: The driver parks and flags for a human when it encounters a GSD gate that requires human judgement
- [ ] **DRIVE-06**: The driver reports a terminal outcome — goal met, parked, or halted — with the reason

### Containerized Sessions (CTNR)

- [ ] **CTNR-01**: A user can run a driven project inside a container instead of on the host
- [ ] **CTNR-02**: The container runtime is auto-detected; both docker and podman work without configuration
- [ ] **CTNR-03**: Claude subscription authentication works inside the container without bind-mounting the host credential store
- [ ] **CTNR-04**: A user can start, stop, and resume containerized sessions from the TUI
- [ ] **CTNR-05**: Container and host execution behave identically from the driver's perspective, including session resume

### UI Fixes (UIFIX)

- [ ] **UIFIX-01**: Paused projects show a pause badge derived from HANDOFF.md
- [ ] **UIFIX-02**: The DRPEV status display has no leading blank
- [ ] **UIFIX-03**: Markdown edit mode activates on key press
- [ ] **UIFIX-04**: PageDown scroll offset is clamped to the end of content

## Future Requirements

Deferred to a later milestone — deliberately out of v2.0 scope:

- Socket-based fast-path for injection (the durable file inbox covers the requirement; the socket is a latency optimization) **[ASSUMED]**
- Fleet-level driving — one driver goal spanning multiple projects at once. v2.0 is one driver per project. **[ASSUMED]**
- Replacing `session_detector.rs`'s pgrep+`/proc` scraping with `claude agents --json`. Research found this is strictly better (yields `sessionId` and busy/idle), but it is a v1.x subsystem refactor, not a v2.0 capability. Captured as a follow-up. **[ASSUMED]**
- Live re-streaming of a run's output after TUI restart. Research established this is impossible by construction — once the TUI exits, the child's stdout pipe is gone. Reattachment is journal-based and read-only; OBS-01/CTRL-04 are scoped accordingly.
- Windows support for the driver. `process_group(0)` is Unix-only; the TUI remains cross-platform but driving is Unix-only in v2.0. **[ASSUMED]**

## Out of Scope

- **tmux as the control channel** — superseded by duplex `stream-json`. Driver and human keystrokes interleave on a shared pane and corrupt each other. tmux stays as optional human attach only.
- **`--bare` flag** — recommended by Anthropic for scripted calls and slated to become the `-p` default, but it skips OAuth/keychain reads and requires an API key, breaking the subscription-auth constraint. Excluded deliberately; flagged as a forward-compatibility risk to monitor.
- **`--max-budget-usd` as the cost control** — on subscription auth the dollar figure is a notional API-equivalent price, not a charge. The real constraint is the 5-hour/7-day quota. Retained only as an anomaly circuit-breaker (CTRL-06), never as the primary cap.
- **Prompt-text guardrails** — any safety rule that lives only in prompt wording. The Replit incident deleted a production database during an explicit code freeze precisely because the freeze was prompt-only. All SAFE-* requirements must be mechanically enforced.
- **Trusting agent self-reports** — run status derived from the model's prose. Same incident: the agent hid the deletion, fabricated data, and falsely claimed rollback was impossible.
- **A second progress display for driven projects** — the existing D-R-P-E-V pipeline widget already is the step timeline. Reuse it.
- **Plugin system, Telegram bridge, remote/SSH project management, offline mode** — unchanged from v1.x out-of-scope.

## Traceability

Every v2.0 requirement maps to exactly one phase. Phase numbering continues from
the previous milestone (last shipped phase was 13), so v2.0 spans Phases 14-22.

| Requirement | Phase | Status |
|-------------|-------|--------|
| TRANS-01 | Phase 15: Transport Foundation | Pending |
| TRANS-02 | Phase 15: Transport Foundation | Pending |
| TRANS-03 | Phase 15: Transport Foundation | Pending |
| TRANS-04 | Phase 15: Transport Foundation | Pending |
| TRANS-05 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| CTRL-01 | Phase 17: Supervisor | Pending |
| CTRL-02 | Phase 17: Supervisor | Pending |
| CTRL-03 | Phase 17: Supervisor | Pending |
| CTRL-04 | Phase 17: Supervisor | Pending |
| CTRL-05 | Phase 17: Supervisor | Pending |
| CTRL-06 | Phase 20: Deterministic Decision Router & Run Bounds | Pending |
| CTRL-07 | Phase 20: Deterministic Decision Router & Run Bounds | Pending |
| SAFE-01 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Pending |
| SAFE-02 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Pending |
| SAFE-03 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Pending |
| SAFE-04 | Phase 16: Run Journal & State Substrate | Pending |
| SAFE-05 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Pending |
| SAFE-06 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Pending |
| SAFE-07 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |
| SAFE-08 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |
| OBS-01 | Phase 16: Run Journal & State Substrate | Pending |
| OBS-02 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| OBS-03 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| OBS-04 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| OBS-05 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| OBS-06 | Phase 16: Run Journal & State Substrate | Pending |
| OBS-07 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| STEER-01 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| STEER-02 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| STEER-03 | Phase 18: Driver Tab, Live Watch & Durable Injection | Pending |
| DRIVE-01 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |
| DRIVE-02 | Phase 20: Deterministic Decision Router & Run Bounds | Pending |
| DRIVE-03 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |
| DRIVE-04 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |
| DRIVE-05 | Phase 20: Deterministic Decision Router & Run Bounds | Pending |
| DRIVE-06 | Phase 20: Deterministic Decision Router & Run Bounds | Pending |
| CTNR-01 | Phase 22: Container Execution Target | Pending |
| CTNR-02 | Phase 22: Container Execution Target | Pending |
| CTNR-03 | Phase 22: Container Execution Target | Pending |
| CTNR-04 | Phase 22: Container Execution Target | Pending |
| CTNR-05 | Phase 22: Container Execution Target | Pending |
| UIFIX-01 | Phase 14: UI Fixes | Pending |
| UIFIX-02 | Phase 14: UI Fixes | Pending |
| UIFIX-03 | Phase 14: UI Fixes | Pending |
| UIFIX-04 | Phase 14: UI Fixes | Pending |
