# Requirements: v2.0 Autonomous Orchestration

**Milestone:** v2.0
**Defined:** 2026-07-28
**Source:** `.planning/research/SUMMARY.md` + backlog 999.2 / 999.3 + four pending UI todos
**Re-planned:** 2026-10-14, after spikes 001 and 002 (`.planning/spikes/`). See "Re-plan 2026-10-14" below.

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

- [x] **TRANS-01**: The driver runs a GSD command by spawning `claude -p` with duplex `stream-json` over stdio, and parses the structured result envelope rather than scraping text
- [x] **TRANS-02**: Run outcome is derived from the `type:"result"` envelope (`subtype`, `is_error`, `terminal_reason`, `permission_denials[]`), exit code, disk state, and git — never from the agent's prose summary
- [x] **TRANS-03**: The TUI event loop reads input while a run is streaming output, so control keys stay responsive throughout a multi-hour run
- [x] **TRANS-04**: The tool detects Claude CLI capabilities at runtime via `system/init` feature detection and refuses to start a run against an unsupported CLI, with a clear message
- [x] **TRANS-05**: A user can execute a single GSD command against an opted-in project from the TUI and watch its output stream live (no autonomy yet — the manual ship point)

### Run Control & Safety Envelope (CTRL)

- [x] **CTRL-01**: A user can stop any run at any time from the TUI, and stopping terminates the entire process tree including `claude`'s Bash grandchildren
- [x] **CTRL-02**: A user can start any run in dry-run mode, which reports the GSD commands it would issue plus the resulting diffstat and push refspecs, without executing them
- [x] **CTRL-03**: A project is driven only when explicitly opted in; non-opted-in projects are never spawned against, enforced at the process-spawn seam
- [x] **CTRL-04**: An autonomous run survives the TUI being closed, and the TUI reconciles live runs on restart
- [x] **CTRL-05**: Only one driver may execute against a given project at a time, enforced by an OS-level lock
- [x] **CTRL-06**: A run halts itself when it stops making progress, repeats a command, exceeds a step cap, or exceeds a wall-clock cap
- [x] **CTRL-07**: A run parks rather than retrying when it hits a Claude subscription rate limit, and reports which quota window blocked it
- [ ] **CTRL-08**: The policy for a human-judgement verification gate is user-configurable to skip, defer, or auto-validate — settable both per-run and per-project/session, with `defer` the default and every non-default choice visible in the run record *(superseded 2026-10-14; see Traceability)*

### Git & Blast Radius (SAFE)

- [x] **SAFE-01**: The driver pushes only to a reserved branch namespace; pushes outside it are blocked by a mechanism the agent cannot talk its way past
- [x] **SAFE-02**: Force-push and hook-bypass are blocked for driver-initiated git operations
- [x] **SAFE-03**: Driver-initiated pushes are scanned for secrets and blocked on detection
- [x] **SAFE-04**: Credentials and tokens are redacted when captured into the run log, not when rendered
- [x] **SAFE-05**: A driven run uses a scoped git credential rather than inheriting the user's ambient credentials or SSH agent
- [x] **SAFE-06**: PR creation is rate-capped per project per day
- [ ] **SAFE-07**: `.planning/` content read by the driver is passed to the model inside an explicit untrusted-content boundary, never concatenated into instructions *(superseded 2026-10-14; see Traceability)*
- [ ] **SAFE-08**: The model's chosen action is constrained to a fixed enum of GSD commands; free-form shell strings are never executed *(superseded 2026-10-14; see Traceability)*

### Run State & Observability (OBS)

- [x] **OBS-01**: Every run writes an append-only journal to disk that survives process death and is the source of truth for run state
- [x] **OBS-02**: The project list marks which projects are LLM-driven, and which are parked awaiting a human
- [x] **OBS-03**: A user can read the originating goal prompt for any driven project, to understand what it was asked to do
- [x] **OBS-04**: A user can watch a run's live output, current step, elapsed time, and step history from a TUI tab
- [x] **OBS-05**: A user can review what a completed or failed run did, after the fact, including which commands ran and why it stopped
- [x] **OBS-06**: Driver journal writes do not trigger full project re-parses, so a multi-hour run does not degrade TUI responsiveness
- [x] **OBS-07**: A user can sort or filter the dashboard to surface projects that need human attention

### Steering (STEER)

- [x] **STEER-01**: A user can inject a message into a running driver from the TUI
- [x] **STEER-02**: An injected message shows its delivery state — queued, delivered, and acted upon — rather than being fire-and-forget
- [x] **STEER-03**: Injected messages survive a TUI restart between queueing and delivery

### Autonomous Driver (DRIVE)

- [ ] **DRIVE-01**: A user states a goal once in natural language and the driver pursues it across multiple GSD commands without further input
- [x] **DRIVE-02**: The driver chooses the next GSD command from observed project state using deterministic rules, not a model call, for every case the rules cover
- [ ] **DRIVE-03**: The driver's goal is decomposed into a structured, machine-checkable plan that the user can review before the run starts
- [ ] **DRIVE-04**: The driver escalates to a model only for goal decomposition and for ambiguity the rules cannot resolve, with a per-run cap on such escalations *(superseded 2026-10-14; see Traceability)*
- [x] **DRIVE-05**: The driver parks and flags for a human when it encounters a GSD gate that requires human judgement
- [x] **DRIVE-06**: The driver reports a terminal outcome — goal met, parked, or halted — with the reason
- [ ] **DRIVE-07**: Under the `auto` gate policy the driver attempts the verification a human would perform — driving the real surface (TUI, browser, or app) rather than asserting from source — and an auto-validated result is permanently distinguishable on disk from a human-verified one *(superseded 2026-10-14; see Traceability)*

### Containerized Sessions (CTNR)

- [ ] **CTNR-01**: A user can run a driven project inside a container instead of on the host *(superseded 2026-10-14; see Traceability)*
- [ ] **CTNR-02**: The container runtime is auto-detected; both docker and podman work without configuration *(superseded 2026-10-14; see Traceability)*
- [ ] **CTNR-03**: Claude subscription authentication works inside the container without bind-mounting the host credential store *(superseded 2026-10-14; see Traceability)*
- [ ] **CTNR-04**: A user can start, stop, and resume containerized sessions from the TUI *(superseded 2026-10-14; see Traceability)*
- [ ] **CTNR-05**: Container and host execution behave identically from the driver's perspective, including session resume *(superseded 2026-10-14; see Traceability)*

### UI Fixes (UIFIX)

- [x] **UIFIX-01**: Paused projects show a pause badge derived from HANDOFF.md
- [x] **UIFIX-02**: The DRPEV status display has no leading blank
- [x] **UIFIX-03**: Markdown edit mode activates on key press
- [x] **UIFIX-04**: PageDown scroll offset is clamped to the end of content

### Agent Observation (AGENT)

- [x] **AGENT-01**: The TUI enumerates each registered project's agent worktrees (under `.claude/worktrees/`, on an agent branch, or claimed by an adapter) with commits ahead of the main worktree's HEAD and a dirty-file count, using read-only git that takes no lock in a live agent's worktree
- [x] **AGENT-02**: For the active phase, plans are grouped by their PLAN.md `wave:` frontmatter (never by plan number) and each reads done (SUMMARY in main), finished (SUMMARY or lock release in its agent's worktree), running, stalled or queued, with the current wave identified
- [x] **AGENT-03**: Agent metadata arrives through a pluggable runtime-adapter seam; any adapter failure (missing directory, bad or truncated JSON, unknown or missing fields, a panic) degrades a row to what git knows and never drops the row or fails the scan
- [x] **AGENT-04**: A Claude Code adapter attaches agent type, description and liveness (transcript mtime; lock release as finished) from `<config>/projects/<encoded path>/*/subagents/` metadata, honours `$CLAUDE_CONFIG_DIR`, and lists live worktree-less subagents
- [x] **AGENT-05**: While agents are active, the dashboard row's Status cell shows a compact summary (phase, wave x/y, running, done/total) that fits the measured column width; with no active agents the row is unchanged
- [x] **AGENT-06**: The Sessions tab of the detail view has an Agents sub-view with the summary, per-wave running/done/queued rows, a scrollable per-agent list and a worktree-less group, with explicit empty and degraded states
- [x] **AGENT-07**: For code-review fix runs, the summary shows an estimated fixed/total count from REVIEW.md `findings.total` and finding ids in `fix(NN):` commit subjects, labelled as an estimate

## Re-plan 2026-10-14: capability-removal driver

Phases 19 (GITSAFE), 21 (LLM goal layer), 22 (containers) and 23 (gate policy) are **superseded**. The
replacement direction comes from spikes 001 and 002, both PARTIAL. Safety comes from **capability
removal and hard caps in the launcher**, not from parsing bash or auditing without end:

- An Opus master driver reads a human-owned
  `.planning/meta-manager/drivers/<run-id>/DIRECTION.md`.
- It dispatches one fresh subagent per GSD step.
- The launcher owns a bwrap sandbox under `systemd-run --user --scope`, the run lock and the caps.

The requirement IDs below are new. Superseded IDs are not deleted. Each is re-marked in Traceability
with what supersedes it. Inferred calls are marked `[inferred]`.

### Driver Validation (VALID): Phase 26, re-run in 26.1 (VALID-01/02)

- [x] **VALID-01**: On a real GSD scratch project seeded with a gap that looks closable but keeps
  producing new findings, the driver runs gap-closure rounds 1 and 2. It parks `needs-human` no later
  than the third `gaps_found`, and driverctl records the rule that fired.
- [x] **VALID-02**: Divergence detection (two consecutive rounds with new findings) parks the run on
  its own. With the prompt rule disabled, the launcher gap cap trips instead: deny at limit+1, kill at
  limit+2.
- [x] **VALID-03**: After an async Agent launch, the driver and its workers wait for the notification
  and never re-spawn. The transcripts show zero duplicate planner or executor runs.
- [x] **VALID-04**: The spike records the measured per-phase cost of an Opus driver and gives an
  explicit go / adjust / no-go for Phases 27-31.

### driverctl & Frontmatter (DCTL): Phase 27

- [x] **DCTL-01**: `gsd-meta-manager driverctl serve|log|hook|show` is the single writer of a run's
  `state.json`, `LOG.md` and DIRECTION.md frontmatter. It speaks the spike-002 socket protocol and
  runs on the launcher side, outside the sandbox.
- [x] **DCTL-02**: The frontmatter is rewritten by exchange-verify splice: `renameat2(RENAME_EXCHANGE)`,
  a check of the displaced file's body bytes (27-CONTEXT D-23; inode alone misses in-place saves),
  and a swap back on mismatch.
  - The human body is preserved byte-for-byte.
  - A body missing `## Direction` or `## Items` is never written back.
  - gsd-mm's own DIRECTION writes share a flock with driverctl.
- [x] **DCTL-03**: Frontmatter schema v1 is a typed `DriverFrontmatter` in `state_reader`. Liveness is
  derived from a non-blocking probe of the launcher's run flock, so `running` with a free lock reads
  as crashed.
- [x] **DCTL-04**: Counters come only from tool-observed state:
  - dispatches from hook payloads, where a missing `agent_id` means a driver-level call;
  - gap rounds from `gaps_found` VERIFICATION.md writes on disk, coalesced per the 26.1 rule
    (`COALESCE_S = 45`, strict, mtime-anchored). driverctl owns the counter and its VERIFICATION.md
    scan.

  Malformed or unknown events are rejected with rc=2. Heartbeat-only rewrites are coalesced to at
  most one every 120 s.

### Launcher (LNCH): Phase 28

- [x] **LNCH-01**: `gsd-meta-manager driver launch` runs the master headless inside bwrap under
  `systemd-run --user --scope`. Stop kills the whole scope, including descendants.
- [x] **LNCH-02**: The run lock is an flock held by the launcher outside the sandbox. It reuses
  Phase 17 CTRL-05 and is never a pid file. A second launcher is refused, and a relaunch after a
  crash logs `crash-detected`.
- [x] **LNCH-03**: Hard caps are enforced from outside the sandbox:
  - dispatch count: denied at the limit;
  - active wall-clock minutes: kill the scope, with `RuntimeMaxSec` as backstop;
  - gap rounds: deny at limit+1, kill at limit+2, reading driverctl's `caps.gap_rounds.per_phase`
    (the counter is owned by Phase 27, DCTL-04).

  A rate-limit result parks the run without retry and names the quota window.
- [x] **LNCH-04**: Each run gets its own `CLAUDE_CONFIG_DIR`.
  - GSD agents, skills and hooks are linked read-only.
  - The user's GSD and rtk hooks are kept.
  - The global `CLAUDE.md` is not linked (OQ-R2 default) `[inferred]`.

  Inside the sandbox:
  - host `~/.claude` is read-only;
  - `.credentials.json` and `~/.claude.json` are masked;
  - `~/.ssh` is empty;
  - `SSH_AUTH_SOCK`, `GH_TOKEN` and `ANTHROPIC_API_KEY` are stripped;
  - the run dir is read-only.
- [x] **LNCH-05**: Auth is passed as `CLAUDE_CODE_OAUTH_TOKEN` from a stored long-lived
  `claude setup-token` token, set up by a guided onboarding step. A host-token fallback detects
  `OAuth token revoked` and relaunches. No refresh token ever enters the sandbox.
- [x] **LNCH-06**: A driven run's commits leave the machine only through gsd-mm's push step outside
  the sandbox. That step uses a fixed refspec into the reserved namespace and reuses the namespace
  refusal, secret scan and PR cap (SAFE-01/03/06).
- [x] **LNCH-07**: Without bwrap or systemd-run (macOS), the launcher refuses by default. An explicit
  opt-in degraded mode runs unsandboxed, with caps enforced by process supervision, and the posture
  is recorded on the run and shown in the TUI `[inferred]`.
- [x] **LNCH-08**: Headless operation needs no tmux. An interactive or tmux attach is optional and
  never on the control path.

### Master Prompt & Templates (PRMT): Phase 29

- [ ] **PRMT-01**: The master prompt ships embedded and versioned, and its version is recorded on the
  run. It requires:
  - one fresh subagent per GSD step;
  - the 11-field report of at most 15 lines, copied verbatim;
  - nested `isolation: "worktree"` pass-through;
  - the async no-respawn rule;
  - every record going through driverctl.
- [ ] **PRMT-02**: `gsd-meta-manager driver new` mints a `<yyyymmdd>-<slug>` run id. It creates
  `.planning/meta-manager/drivers/<run-id>/`, with a self-ignoring `.gitignore`, DIRECTION.md, and
  LOG.md with its grammar inline. DIRECTION.md holds `control: run|pause|stop`, standing rules, and
  items with intent, done-when, budget, after, hold and answer.
- [ ] **PRMT-03**: The prompt and the template state these stop rules, and their precedence:
  - item budget;
  - at most 2 gap rounds per phase;
  - divergence;
  - no progress;
  - run budget.

  Stop rules outrank a human `answer:`, and only a budget edit lifts one (OQ-R1 default)
  `[inferred]`.
- [ ] **PRMT-04**: Startup reconciliation resolves every `running` item to exactly one of done,
  resume, reset or park. A `control` edit is honoured before the next dispatch.
- [x] **PRMT-05**: `gsd-meta-manager next <project> --json` exposes the Phase 20 router as an advisory
  hint that never binds the driver.

### Driver Tab (DTAB): Phase 30

- [ ] **DTAB-01**: The Driver tab lists a project's run dirs. For the selected run it shows, from
  frontmatter only:
  - status;
  - per-item state;
  - caps used/limit;
  - the dispatch audit;
  - heartbeat age;
  - `connected[]`.
- [ ] **DTAB-02**: Crashed runs and stale heartbeats are flagged. The dashboard marks projects that
  are driven, parked or crashed.
- [ ] **DTAB-03**: A user can set `control`, answer or un-hold a parked item, and open DIRECTION.md
  in `$EDITOR` from the tab. Every edit takes the shared flock and never touches the frontmatter.
- [ ] **DTAB-04**: A user can start a new run, relaunch a crashed run, and tail LOG.md live from the
  tab.

### Retirement (RETIRE): Phase 31

- [ ] **RETIRE-01**: The following superseded paths are removed:
  - the Phase 19 argv/bash-classifying `PreToolUse` guard and its carrier corpora;
  - the Phase 21 goal-layer seams;
  - stream-json durable-injection steering.

  The parts that survive are rewired into the launcher or the push step: namespace refusal, secret
  scan, PR ledger, SAFE-04 redaction, the `Untrusted` display type, and the router.
- [ ] **RETIRE-02**: The Phase 22 plans are archived as superseded, and `bollard` is never added.
- [ ] **RETIRE-03**: The open Phase 19/21 threat ids and criterion 4 are closed as superseded, each
  with a pointer. `./scripts/pre-tag-check.sh --container` passes on the reduced tree.

## Future Requirements

Deferred to a later milestone — deliberately out of v2.0 scope:

- Socket-based fast-path for injection (the durable file inbox covers the requirement; the socket is a latency optimization) **[ASSUMED]**
- Fleet-level driving — one driver goal spanning multiple projects at once. v2.0 is one driver per project. **[ASSUMED]**
- Replacing `session_detector.rs`'s pgrep+`/proc` scraping with `claude agents --json`. Research found this is strictly better (yields `sessionId` and busy/idle), but it is a v1.x subsystem refactor, not a v2.0 capability. Captured as a follow-up. **[ASSUMED]**
- Live re-streaming of a run's output after TUI restart. Research established this is impossible by construction — once the TUI exits, the child's stdout pipe is gone. Reattachment is journal-based and read-only; OBS-01/CTRL-04 are scoped accordingly.
- Auto-validation by driving a real surface (DRIVE-07, from superseded Phase 23). Deferred by the 2026-10-14 re-plan `[inferred]`.
- Container execution target (CTNR-01..05, from superseded Phase 22). Deferred; Linux isolation comes from bwrap + cgroup scope `[inferred]`.
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
the previous milestone (last shipped phase was 13), so v2.0 spans Phases 14-31 (re-plan 2026-10-14 added 26-31).

| Requirement | Phase | Status |
|-------------|-------|--------|
| TRANS-01 | Phase 15: Transport Foundation | Complete |
| TRANS-02 | Phase 15: Transport Foundation | Complete |
| TRANS-03 | Phase 15: Transport Foundation | Complete |
| TRANS-04 | Phase 15: Transport Foundation | Complete |
| TRANS-05 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| CTRL-01 | Phase 17: Supervisor | Complete |
| CTRL-02 | Phase 17: Supervisor | Complete |
| CTRL-03 | Phase 17: Supervisor | Complete |
| CTRL-04 | Phase 17: Supervisor | Complete |
| CTRL-05 | Phase 17: Supervisor | Complete |
| CTRL-06 | Phase 20: Deterministic Decision Router & Run Bounds | Complete (20-VERIFICATION 5/5) [inferred]; hard caps re-homed to Phase 28 (LNCH-03) |
| CTRL-07 | Phase 20: Deterministic Decision Router & Run Bounds | Complete (20-VERIFICATION 5/5) [inferred]; launcher-side park in Phase 28 (LNCH-03) |
| CTRL-08 | Phase 23 (superseded) | Superseded by per-item done-when + standing rules, Phase 29 (PRMT-02/03) |
| SAFE-01 | Phase 19 (superseded) | Complete in code; enforcement moves to push step outside sandbox, Phase 28 (LNCH-06) |
| SAFE-02 | Phase 19 (superseded) | Superseded by capability removal (no push creds in sandbox, LNCH-04/06); guard retired in Phase 31 |
| SAFE-03 | Phase 19 (superseded) | Complete in code; reused by the Phase 28 push step (LNCH-06) |
| SAFE-04 | Phase 16: Run Journal & State Substrate | Complete |
| SAFE-05 | Phase 19 (superseded) | Moot: sandbox holds no credentials (LNCH-04) |
| SAFE-06 | Phase 19 (superseded) | Complete in code; reused by the Phase 28 push step (LNCH-06) |
| SAFE-07 | Phase 21 (superseded) | Superseded: injection contained by sandbox (LNCH-04), not prevented |
| SAFE-08 | Phase 21 (superseded) | Superseded: blast radius bounded by sandbox + launcher caps (LNCH-03/04) |
| OBS-01 | Phase 16: Run Journal & State Substrate | Complete |
| OBS-02 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| OBS-03 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| OBS-04 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| OBS-05 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| OBS-06 | Phase 16: Run Journal & State Substrate | Complete |
| OBS-07 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| STEER-01 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| STEER-02 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| STEER-03 | Phase 18: Driver Tab, Live Watch & Durable Injection | Complete |
| DRIVE-01 | Phase 28/29 (re-mapped) | Pending: Phase 28 launcher half evidenced live (28-09: d33 real-Opus driver, organic deny then scope kill); Phase 29 master prompt half open [inferred - audit] |
| DRIVE-02 | Phase 20: Deterministic Decision Router & Run Bounds | Complete (20-VERIFICATION 5/5) [inferred]; now an advisory hint (PRMT-05) |
| DRIVE-03 | Phase 29 (re-mapped) | Pending: DIRECTION.md items with done-when/budget are the reviewable plan |
| DRIVE-04 | Phase 21 (superseded) | Superseded: the master is the model; dispatch cap replaces escalation cap (LNCH-03) |
| DRIVE-05 | Phase 20: Deterministic Decision Router & Run Bounds | Complete (20-VERIFICATION 5/5) [inferred] |
| DRIVE-06 | Phase 20: Deterministic Decision Router & Run Bounds | Complete (20-VERIFICATION 5/5) [inferred] |
| DRIVE-07 | Phase 23 (superseded) | Deferred to Future Requirements [inferred] |
| CTNR-01 | Phase 22 (superseded) | Superseded by bwrap + systemd-run scope (Phase 28); deferred [inferred] |
| CTNR-02 | Phase 22 (superseded) | Superseded by bwrap + systemd-run scope (Phase 28); deferred [inferred] |
| CTNR-03 | Phase 22 (superseded) | Superseded by bwrap + systemd-run scope (Phase 28); deferred [inferred] |
| CTNR-04 | Phase 22 (superseded) | Superseded by bwrap + systemd-run scope (Phase 28); deferred [inferred] |
| CTNR-05 | Phase 22 (superseded) | Superseded by bwrap + systemd-run scope (Phase 28); deferred [inferred] |
| UIFIX-01 | Phase 14: UI Fixes | Complete |
| UIFIX-02 | Phase 14: UI Fixes | Complete |
| UIFIX-03 | Phase 14: UI Fixes | Complete |
| UIFIX-04 | Phase 14: UI Fixes | Complete |
| AGENT-01 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-02 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-03 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-04 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-05 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-06 | Phase 25: Running Agents & Live Wave View | Complete |
| AGENT-07 | Phase 25: Running Agents & Live Wave View | Complete |
| VALID-01 | Phase 26.1 | Complete — Phase 26.1 (results/r261a/acceptance.txt A1 A2 A3 A6; I-01 needs-human in results/r261a/LOG.md) |
| VALID-02 | Phase 26.1 | Complete (ADJUST, operator-accepted 2026-10-18) — divergence parking proven on Opus (results/r261a/acceptance.txt A3 A4 PASS); launcher deny/kill proven by Haiku-scripted evidence only (results/r261-l7/acceptance.txt L7-1..L7-6 PASS); Opus NOT-REACHED (results/r261-run2b/acceptance.txt A5-deny, A5-kill, A11 NOT-REACHED: driver parked needs-human at gap-observed count=2, LOG.md:100-101, J1). Carried risk: Phases 28/29 must exercise deny/kill under the real driver |
| VALID-03 | Phase 26 | Complete |
| VALID-04 | Phase 26 | Complete |
| DCTL-01 | Phase 27 | Complete |
| DCTL-02 | Phase 27 | Complete |
| DCTL-03 | Phase 27 | Complete |
| DCTL-04 | Phase 27 | Complete |
| LNCH-01 | Phase 28 | Complete |
| LNCH-02 | Phase 28 | Complete |
| LNCH-03 | Phase 28 | Complete |
| LNCH-04 | Phase 28 | Complete |
| LNCH-05 | Phase 28 | Complete |
| LNCH-06 | Phase 28 | Complete |
| LNCH-07 | Phase 28 | Complete |
| LNCH-08 | Phase 28 | Complete |
| PRMT-01 | Phase 29 | Pending |
| PRMT-02 | Phase 29 | Pending |
| PRMT-03 | Phase 29 | Pending |
| PRMT-04 | Phase 29 | Pending |
| PRMT-05 | Phase 29 | Complete |
| DTAB-01 | Phase 30 | Pending |
| DTAB-02 | Phase 30 | Pending |
| DTAB-03 | Phase 30 | Pending |
| DTAB-04 | Phase 30 | Pending |
| RETIRE-01 | Phase 31 | Pending |
| RETIRE-02 | Phase 31 | Pending |
| RETIRE-03 | Phase 31 | Pending |
