# Roadmap: GSD Meta Manager

## Milestones

- ✅ **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- ✅ **v1.1 Polish & Power Features** - Phases 05-09 (shipped 2026-03-27)
- ✅ **v1.2 Housekeeping & Archive Browser** - Phases 10-13 (shipped 2026-04-01)
- ✅ **v1.3 Configuration & Pipeline Visibility** - 10 quick tasks (shipped 2026-05-09)
- ✅ **v1.4 Live Sessions & Document Browsing** - 4 quick tasks (shipped 2026-05-12)
- ✅ **v1.5.0 Sub-phase Artifact Detection** - 2 quick tasks (shipped 2026-05-15)
- ✅ **v1.6.0 GSD 1.8.0 Catch-up** - 1 quick task + 2 fast tasks (shipped 2026-07-22)
- 🚧 **v2.0 Autonomous Orchestration** - Phases 14-22 (in progress)

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>v1.0 MVP (Phases 01-04) - SHIPPED 2026-03-26</summary>

See `.planning/milestones/v1.0-phases/` for archived phase artifacts.

Phase 01: Project Foundation (3 plans, complete)
Phase 02: Dashboard & Navigation (3 plans, complete)
Phase 03: Live State & Visualization (2 plans, complete)
Phase 04: Project Creation & Queue (2 plans, complete)

</details>

<details>
<summary>v1.1 Polish & Power Features (Phases 05-09) - SHIPPED 2026-03-27</summary>

See `.planning/milestones/v1.1-phases/` for archived phase artifacts.

Phase 05: State Reader Accuracy (5 plans, complete)
Phase 06: Read-Only Views (4 plans, complete)
Phase 07: Execution Flow & GSD Integration (3 plans, complete)
Phase 08: Queue Execution (2 plans, complete)
Phase 09: Claude Session Management (2 plans, complete)

</details>

<details>
<summary>v1.2 Housekeeping & Archive Browser (Phases 10-13) - SHIPPED 2026-04-01</summary>

See `.planning/milestones/v1.2-phases/` for archived phase artifacts.

Phase 10: Tech Debt Cleanup (1 plan, complete)
Phase 11: Paused Project Detection (1 plan, complete)
Phase 12: Milestone Archive Browser (3 plans, complete)
Phase 13: Queue Execution Research (1 plan, complete)

</details>

<details>
<summary>v1.3 Configuration & Pipeline Visibility (10 quick tasks) - SHIPPED 2026-05-09</summary>

No formal phases — the milestone shipped entirely via `/gsd-quick` tasks
listed in `STATE.md` "Quick Tasks Completed":

- 260401-t7y: tui-textarea + $EDITOR shell-out for archive/backlog markdown
- 260403-p84: PageUp/PageDown scrolling on the detail screen
- 260405-27p: fix folder appears empty after returning from markdown view
- 260405-oum: initial Defaults tab — display + edit .planning/config.json
- 260405-urb: GitHub-ready README.md
- 260509 (defaults dropdown): replace toggle-on-Enter with dropdown picker; include `adaptive` profile
- 260509-k9m: surface intel/graphify keys; text-input for String rows; `x`-to-clear shortcut
- 260509-zh2: layer ~/.gsd/defaults.json under project config; six-section layout matching `/gsd-settings`; `[d]` toggle to edit defaults; pipeline sub-stage drill-down
- 260509-t8m: Tab-to-switch into a tmux Claude session

</details>

<details>
<summary>v1.4 / v1.5.0 / v1.6.0 (quick tasks only) - SHIPPED 2026-05-12 → 2026-07-22</summary>

No formal phases — see `.planning/MILESTONES.md` and `STATE.md`
"Quick Tasks Completed" for the full task list.

</details>

### v2.0 Autonomous Orchestration (Phases 14-22)

- [x] **Phase 14: UI Fixes** - Four display defects that misreport project state (completed 2026-07-29)
- [x] **Phase 15: Transport Foundation** - Duplex `stream-json` executor with envelope-derived outcomes (completed 2026-07-29)
- [x] **Phase 16: Run Journal & State Substrate** - Durable, redacted, cheap-to-read run record (completed 2026-07-29)
- [x] **Phase 17: Supervisor** - Detach, kill switch, dry-run, single-run lock, opt-in gate (completed 2026-07-29)
- [ ] **Phase 18: Driver Tab, Live Watch & Durable Injection** - Manual ship point: run and steer from the TUI
- [ ] **Phase 19: GITSAFE — Git & Blast-Radius Envelope** - Mechanically enforced push boundary
- [ ] **Phase 20: Deterministic Decision Router & Run Bounds** - Rules pick the next command; runs stop themselves
- [ ] **Phase 21: LLM Goal Layer & Prompt-Injection Hardening** - One stated goal, model confined to two seams
- [ ] **Phase 22: Container Execution Target** - Docker/podman parity with the host path

**Parallelism:** Phase 14 has no dependencies and is parallel-safe throughout.
Phase 22 depends only on Phase 15 and may run alongside Phases 17-21, but must land
before Phase 20 closes so the router is never built against a stubbed target.

## Phase Details

### Phase 14: UI Fixes

**Goal**: Four long-standing display defects stop misreporting project state
**Depends on**: Nothing (no v2.0 dependencies — parallel-safe, can ship any time)
**Requirements**: UIFIX-01, UIFIX-02, UIFIX-03, UIFIX-04
**Success Criteria** (what must be TRUE):

  1. A project with a non-empty `HANDOFF.md` shows the pause badge on its dashboard row
  2. The D-R-P-E-V status display renders with no leading blank at any terminal width
  3. Pressing the markdown edit key enters edit mode on the first press
  4. PageDown at the end of a document leaves the last line on screen instead of scrolling past the content

**Research**: skip — established codebase idioms, no new external unknowns
**Plans**: 4/4 plans executed

Plans:
**Wave 1**

- [x] 14-01-PLAN.md — UIFIX-01 pause-badge verify-first tracer + regression matrix; UIFIX-02 D-R-P-E-V pad-cell removal at the construction site

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 14-02-PLAN.md — UIFIX-03 route `e` on the Docs tab to the existing SuspendAndEdit path; UIFIX-04 clamp the stored scroll offset to the renderer's own bound

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 14-03-PLAN.md — full project gate, clippy-delta audit, success-criterion traceability, and retirement of the four source todos

**Wave 4** *(gap closure — blocked on Wave 3 completion)*

- [x] 14-04-PLAN.md — close the two verification gaps: UIFIX-02 Status-column 13-cell floor + rendered-buffer regression test; UIFIX-04 clamp the four up-direction scroll sites and the generic fallback (CD-03)

**UI hint**: yes

### Phase 15: Transport Foundation — Duplex stream-json Executor

**Goal**: The tool can run a GSD command through `claude -p` over a structured two-way protocol and know exactly how it ended
**Depends on**: Nothing (v2.0 foundation — everything else stands on this)
**Requirements**: TRANS-01, TRANS-02, TRANS-03, TRANS-04
**Success Criteria** (what must be TRUE):

  1. A real multi-step GSD skill (e.g. `/gsd:execute-phase`) runs headlessly against a scratch project to completion, without hanging
  2. A run's outcome — succeeded, errored, permission-denied, killed — is reported from the `type:"result"` envelope, exit code, disk state, and git, and is correct even when the agent's own prose summary says otherwise
  3. The TUI keeps redrawing and accepting keypresses while a run streams output for minutes at a time
  4. Starting a run against a Claude CLI missing a required `system/init` capability is refused up front, naming the missing capability, rather than failing mid-run

**Phase risks** (carried from research, must resolve before this phase closes):

  - **MUST-SPIKE (OQ1)**: does `claude -p` reliably execute a multi-step GSD skill headlessly? The whole milestone rests on it — test this before any other Phase 15 work. A `PreToolUse` hook with no timeout reproduced a 180-240s hang on any tool-using prompt; `--setting-sources project` fixed the synthetic case, confirm it generalises
  - **MUST-SPIKE (OQ2)**: exact mid-turn stdin injection semantics on the installed CLI, and whether `control_request{subtype:"interrupt"}` behaves as community sources describe. Determines whether Phase 18's injection UI can steer mid-turn or must buffer to the turn boundary
  - **MUST-SPIKE (OQ3)**: does `--max-budget-usd` apply at all under subscription auth? If it is a no-op, the quota-floor mechanism in Phase 20 becomes the only cost control, not a supplement
  - MSRV rises 1.85 → 1.87 (process-wrap floor) — a real project-level change
  - `--bare` is incompatible with subscription auth and is slated to become the `-p` default; ship a version gate plus a regression guard. Parse `stream-json` tolerantly (never `deny_unknown_fields`)
  - `run_tui_loop`'s single-consumer `rx.recv().await` must become a `tokio::select!` here — a prerequisite, not an incidental cleanup

**Research**: yes — `/gsd-plan-phase --research-phase`. The stream-json protocol details are officially undocumented; needs a dedicated empirical spike, not just planning-time reading
**Plans**: 8/8 plans executed

Plans:
**Wave 1**

- [x] 15-01-PLAN.md — OQ1 multi-step spike (the phase gate, D-27/D-28), MSRV 1.87 + process-wrap/uuid, eight redacted golden transcripts

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 15-02-PLAN.md — tracer: one GSD command end-to-end from spawn to RunOutcome, plus the tolerant NDJSON model verified against all eight fixtures

**Wave 3** *(blocked on Wave 2 completion — three plans run in parallel)*

- [x] 15-03-PLAN.md — fail-closed capability/version/auth-path gate (TRANS-04) and the duplex `send`/`interrupt` control channel
- [x] 15-05-PLAN.md — four-source outcome derivation matrix with disk and git corroboration (TRANS-02)
- [x] 15-06-PLAN.md — `tokio::select!` event-loop restructure with `pump()` in the library and three deterministic responsiveness proofs (TRANS-03)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 15-04-PLAN.md — supervisor with dual deadlines and four-step process-group teardown, with unix-gated lifecycle tests

**Wave 5** *(gap closure — blocked on Wave 4 completion)*

- [x] 15-07-PLAN.md — close SC-2/CR-04 by deriving the outcome from the full `result` envelopes so a permission-blocked run reports `PermissionDenied`, and close CR-03 by bounding and draining the control-response wait (TRANS-01, TRANS-02)

**Wave 6** *(gap closure — blocked on Wave 5 completion)*

- [x] 15-08-PLAN.md — close SC-1/CR-01+CR-02 by evaluating every supervisor bound per loop pass, bounding the event forward, and bounding the post-exit drain so a descendant holding stdout can never hang the run (TRANS-01)

**UI hint**: no — this phase ships no visual surface. TRANS-03 restructures the
`run_tui_loop` event loop (input responsiveness), but adds no widget, screen, or layout.
The Driver tab and every rendered driver surface land in Phase 18.

### Phase 16: Run Journal & State Substrate

**Goal**: Every run leaves a durable, redacted, cheap-to-read record on disk that outlives the processes that wrote it
**Depends on**: Phase 15
**Requirements**: OBS-01, OBS-06, SAFE-04
**Success Criteria** (what must be TRUE):

  1. After a run is killed mid-flight or its host process dies, the on-disk journal still shows every step it completed and the last event before death
  2. A run appending events every few seconds for an hour leaves TUI navigation as responsive as when idle — journal writes do not trigger a full project re-parse
  3. A credential or token that appears in a run's output is already redacted in the journal file on disk, not merely in the rendered view

**Phase risks**:

  - **OQ9 — RESOLVED at planning (D-07/D-08)**: `run.json` is committed; `journal.jsonl`, `active` and every future per-run file are gitignored. Two refinements to the research default: the ignore entry is written at **run-directory creation**, not at opt-in time, because opt-in is Phase 17's and waiting would leave a window in which a journal exists unprotected; and `run.json` is written **exactly twice** (before spawn, after exit) so the driven agent's own `git add -A` can never sweep a mid-run snapshot into an unrelated commit
  - SAFE-04 (redact-at-capture) is bound to this phase rather than to Phase 19 deliberately: PITFALLS scopes redaction to the phase that builds the log-capture path, because every log written before the retrofit stays unredacted forever
  - Run-log growth must be bounded (per-run cap, rotation, retain last N runs) — a 4h run is tens of MB of JSON

**Research**: done — `16-RESEARCH.md` (empirically executed; every claim produced by running code)
**Plans**: 6/6 plans executed

Plans:
**Wave 1**

- [x] 16-01-PLAN.md — Tracer: `src/journal/` module, redact-at-capture behind the `RedactedLine` seam, append-only NDJSON writer, byte-offset tail, pure path classification
- [x] 16-02-PLAN.md — The bounded drain's dropped-event count becomes an observable stream event (D-33 handover from 15-08)

**Wave 2** *(blocked on Wave 1)*

- [x] 16-03-PLAN.md — Run record written exactly twice, the `runs/.gitignore` posture proved against a real repo, per-run growth cap and retention
- [x] 16-04-PLAN.md — The reader's tolerance contract: torn lines, sequence gaps, unknown kinds carried whole, oversize step-over
- [x] 16-05-PLAN.md — OBS-06: `changed_path`, per-`(root, kind)` watcher dedup, the forked `FileChanged` handler, and the counted re-parse measurement with its control arm

**Wave 3** *(blocked on Wave 2)*

- [x] 16-06-PLAN.md — `JournalRun` lifecycle and the `ExecutionEvent` consumer mapping, real-SIGKILL survival with on-disk redaction, phase gate and criteria traceability

**UI hint**: no — this phase ships no visual surface. It delivers the on-disk journal,
path classification in `watcher`/`app`, and redact-at-capture. Every rendered driver
surface (Driver tab, live stream, badges) lands in Phase 18. Planned with `--skip-ui`;
the repo-wide `ui-plan-gate` frontend detector fires on the ratatui codebase, not on this
phase's scope.

### Phase 17: Supervisor — Detach, Kill Switch, Dry-Run, Opt-In Gate

**Goal**: A run is stoppable, survivable, single-instance, previewable, and impossible to start against a project that did not opt in
**Depends on**: Phase 15, Phase 16
**Requirements**: CTRL-01, CTRL-02, CTRL-03, CTRL-04, CTRL-05
**Success Criteria** (what must be TRUE):

  1. Pressing stop during an active run leaves no `claude` process, no grandchild build or server process, and no zombie behind — verified 15s later
  2. Closing the TUI mid-run leaves the run going; reopening the TUI shows that run still live with its current step
  3. A dry-run reports the GSD commands it would issue plus the diffstat and push refspecs those commands would produce, and performs zero git writes
  4. A run cannot be started against a project that has not opted in, and a live run against one project provably performs no write, spawn, or git operation against any other registered project
  5. A second attempt to drive the same project reports which run holds the lock instead of starting a second one

**Phase risks**:

  - **MUST-SPIKE (OQ4)**: does the `--worktree` flag exist and behave as inferred? It was inferred from the statusline JSON schema, not a flag reference, and worktree isolation is load-bearing for driver-vs-human safety. Fallback: `git worktree add` manually and point the driver's cwd at it
  - **OQ8 (non-blocking)**: one driver process per project (research's chosen design) vs one for the fleet. Per-project is simpler to kill and reason about; layer a config-level global concurrency cap (default 1) on top
  - **OQ11**: `process_group(0)` is Unix-only. Driving is Unix-only in v2.0 — document this as an accepted limitation rather than silently dropping Windows
  - The lock must be `flock(2)`, never a PID file or in-memory flag. Opt-in must be a capability type (`DrivableProject`) constructible only from a validated opt-in record, never a bool checked at scattered call sites

**Research**: skip — process-group signal handling and `flock` are standard Unix patterns, corroborated across all four research documents
**Plans**: 8/8 plans complete

Plans:
**Wave 1**

- [x] 17-01-PLAN.md — Tracer: the `drive` subcommand runs one gated, journaled GSD command end-to-end; `DriverOptIn` record, `DrivableProject::from_registry`, and the mechanical spawn-seam guard

**Wave 2** *(blocked on Wave 1)*

- [x] 17-02-PLAN.md — CTRL-05: `flock(2)` on a held descriptor, holder metadata a loser can read, and the contention proof
- [x] 17-03-PLAN.md — CTRL-03: config schema v2, a migration that cannot silently enrol, `driver_max_concurrent`, and the cross-project isolation proof

**Wave 3** *(blocked on Wave 2)*

- [x] 17-04-PLAN.md — CTRL-02: the three-section dry-run (command sequence, working-tree diffstat, locally computed push refspecs) with the reflog-equality and tripwire proofs

**Wave 4** *(blocked on Wave 3)*

- [x] 17-05-PLAN.md — CTRL-04: detached spawn with `kill_on_drop(false)`, the `/proc` pid+cmdline liveness probe, zero-write reconciliation on the existing scan points, and the concurrency cap

**Wave 5** *(blocked on Wave 4)*

- [x] 17-06-PLAN.md — CTRL-01: the two-layer kill switch reusing Phase 15's group teardown, both reaping arms, and the 15-second grandchild-and-zombie proof

**Wave 6** *(blocked on Wave 5)*

- [x] 17-07-PLAN.md — Three dashboard keys and one confirmation, the two Phase 16 carry-forwards (cursor pruning, cross-batch sequence gaps), and the phase gate

**Wave 7** *(blocked on Wave 6 — gap closure from `17-REVIEW.md`)*

- [x] 17-08-PLAN.md — Close the six review blockers: a stop during agent startup is acted on (CR-01), the kill switch signals a kernel-confirmed group (CR-02), the detached spawn carries the TUI's config (CR-03), a run that cannot be identified is refused (CR-04), an undeterminable liveness never reads as "gone" or "crashed" (CR-05), and unregistering cannot abandon a live agent (CR-06)

### Phase 18: Driver Tab, Live Watch & Durable Injection

**Goal**: A human can run and steer a GSD project from the TUI and see exactly what it is doing — the manual ship point, before any autonomy exists
**Depends on**: Phase 16, Phase 17
**Requirements**: TRANS-05, OBS-02, OBS-03, OBS-04, OBS-05, OBS-07, STEER-01, STEER-02, STEER-03
**Success Criteria** (what must be TRUE):

  1. A user picks a GSD command for an opted-in project, runs it from the TUI, and watches its output, current step, and elapsed time stream live
  2. The project list marks driven projects and parked projects distinctly, and can be sorted or filtered down to the ones waiting on a human
  3. The originating goal prompt is readable for any driven project, and a finished or failed run can be reviewed afterwards — which commands ran and why it stopped
  4. A message typed into a running driver shows queued → delivered → acted-on, never a bare "sent"
  5. A message queued moments before the TUI restarts is still delivered to the run afterwards

**Phase risks**:

  - Depends on Phase 15's OQ2 spike result. If mid-turn stdin is dropped by the active turn, injection must buffer and flush at the `result` turn boundary; the three-state UI must reflect that honestly rather than claiming delivery
  - Delivery confirmation must come from correlating the echoed message in the output stream, never from "we wrote to the pipe"
  - Injection is a write to a running agent — route it through the same opt-in capability token as spawning
  - Reuse the existing D-R-P-E-V pipeline widget as the step timeline. A second progress display is an explicit anti-feature
  - Live re-streaming after a TUI restart is impossible by construction (the child's stdout pipe is gone); reattachment is journal-based and read-only

**Research**: optional — skipped by decision at planning time; every external unknown (mid-turn stdin semantics, the `isReplay` dequeue ack, `result` as a turn boundary) was settled empirically by Phase 15's OQ2 spike and is carried in 15-CONTEXT D-29..D-32 and 18-CONTEXT D-02/D-07
**Plans**: 9/11 plans executed
**UI hint**: yes

Plans:
**Wave 1**

- [x] 18-01-PLAN.md — Tracer: end-to-end durable injection spine + path-traversal fix (D-27/WR-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 18-02-PLAN.md — Driver honesty: acted-on, missed, and the blocking-call boundary (D-28/WR-10)
- [x] 18-03-PLAN.md — Journal read surfaces: readable output projection + run listing
- [x] 18-04-PLAN.md — State layer: bounded ring buffer, sanitiser, needs-human predicate, Action types

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 18-05-PLAN.md — Controller: stop-disposition fix (D-29/WR-15), schedulers, the redraw seam
- [x] 18-06-PLAN.md — Dashboard: driven/parked badges, sort toggle, needs-human filter
- [x] 18-07-PLAN.md — Injection input screen and the command + goal start flow
- [x] 18-08-PLAN.md — Release gate for the hidden agent-override flags (D-30/WR-16)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 18-09-PLAN.md — Driver tab: 11th tab, tab-bar tiering, two panes, header, pipeline row

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 18-10-PLAN.md — Live output pane, four-state injection display, after-the-fact run review

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 18-11-PLAN.md — Help screen, dry-run preview, mechanical close-out

### Phase 19: GITSAFE — Git & Blast-Radius Envelope

**Goal**: Autonomous git operations are bounded by mechanisms the agent cannot argue its way past
**Depends on**: Phase 17 (spawn seam for tool denylists and per-run credentials)
**Requirements**: SAFE-01, SAFE-02, SAFE-03, SAFE-05, SAFE-06
**Success Criteria** (what must be TRUE):

  1. A driven run configured to push to `main` is rejected by the envelope, with the model's cooperation removed from the equation
  2. `git push --force`, `+refs/…`, `--no-verify`, and `core.hooksPath` rewrites from a driven run are all blocked, and the attempt parks the run
  3. A push carrying a detectable secret is blocked before it leaves the machine, including a secret written to a gitignored path
  4. A driven run pushes using a per-run scoped credential and still works with the user's ambient credentials and SSH agent unavailable to it
  5. Exceeding the per-project 24-hour PR cap parks the run instead of opening another PR

**Phase risks**:

  - This phase is sequenced immediately before the decision layer per the hardest ordering constraint in the research: GITSAFE is co-resident with the driver, never a follow-up. Phase 20 must be built and tested against a real envelope, not a stub
  - A `pre-push` hook lives in agent-writable `.git/hooks/` — the meta-manager must own installation and re-assert it before each run, deny edits to `.git/hooks/**`, and pair it with a surfaced server-side branch-protection recommendation
  - Repo write and PR creation are two credentials with two scopes — a git token does not grant GitHub API access
  - `git stash` must be forbidden in the driver's allowlist outright

**Research**: skip — branch-protection and pre-push-hook patterns are well-established practice (GitHub Copilot's `copilot/`-prefix model is a direct precedent)
**Plans**: TBD

### Phase 20: Deterministic Decision Router & Run Bounds

**Goal**: The next GSD command is chosen by rules rather than a model call, and a run that stops making progress stops itself
**Depends on**: Phase 16, Phase 17, Phase 19 (and Phase 22 must land before this phase closes)
**Requirements**: CTRL-06, CTRL-07, DRIVE-02, DRIVE-05, DRIVE-06
**Success Criteria** (what must be TRUE):

  1. For every D-R-P-E-V state the rules cover, the next command is chosen with no model call, and the same project state always yields the same choice
  2. A run whose observed state hash is unchanged across consecutive iterations, or that re-selects the same command, halts and names which detector fired
  3. A run exceeding its step cap or its wall-clock cap halts and reports that as the reason
  4. A run that hits a Claude subscription rate limit parks, reports which quota window blocked it and when it resets, and does not retry
  5. Reaching a GSD gate that needs human judgement parks the run with the gate named — and every run ends classified as goal-met, parked, or halted with a reason, never as an unclassified "loop ended"

**Phase risks**:

  - **OQ7 (non-blocking)**: the park-vs-auto-answer taxonomy is a written design artifact owed during discuss/plan, not something discovered during execution. Research's recommendation is "always park", which may mean a fully autonomous run parks at every phase boundary by design — a value-proposition call to make explicitly
  - **OQ10 (non-blocking)**: the 5h/7d quota is shared across every Claude surface the user has, so N concurrent driven projects multiply burn N×. Validate a global concurrency cap default of 1
  - If Phase 15's OQ3 spike shows `--max-budget-usd` is a no-op under subscription auth, the quota floor is promoted from supplementary to the only cost control, and no dollar figure may be shown as "what this run cost"
  - The driver must be driven by an explicit tick or run-completion, never by the file watcher — the watcher-feedback loop is a genuine self-trigger that did not exist in the read-only architecture
  - Refuse to act on inferred-only state; v1.1's verified/inferred badge becomes a safety input here

**Research**: yes — `/gsd-plan-phase --research-phase`. GSD's own autonomous-mode semantics and the `WAITING.json`/checkpoint contract need re-verification: the Phase 13 queue-execution design self-dated "valid until 2026-04-30" and GSD has shipped 1.8.0 since
**Plans**: TBD

### Phase 21: LLM Goal Layer & Prompt-Injection Hardening

**Goal**: A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Depends on**: Phase 20
**Requirements**: DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08
**Success Criteria** (what must be TRUE):

  1. A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs
  2. An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input
  3. Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing
  4. A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes
  5. Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string

**Phase risks**:

  - **OQ6 (non-blocking)**: "done" must be a machine-checkable predicate (e.g. the milestone is marked shipped in `STATE.md` and the tag exists). A goal that cannot be turned into a predicate has no stopping condition and must be refused at start
  - The threat model is real, not theoretical: this tool drives other people's cloned repos, whose `CLAUDE.md` is third-party content. Structural delimiting plus `--strict-mcp-config` with an explicit `--mcp-config`, so a project-local `.mcp.json` cannot introduce tools
  - Opt-in disclosure belongs here in UX terms — list the files that will enter prompts, record a content hash, re-confirm if `CLAUDE.md` changes after opt-in
  - The driver sets its goal once, from a human. It may never enqueue itself more goals from artifacts the agent created during the run

**Research**: optional — not flagged by research; decide at planning time
**Plans**: TBD

### Phase 22: Container Execution Target

**Goal**: A driven project can run inside a container on either runtime, indistinguishably from the host path
**Depends on**: Phase 15 only — parallel-eligible with Phases 17-21, but must land before Phase 20 closes
**Requirements**: CTNR-01, CTNR-02, CTNR-03, CTNR-04, CTNR-05
**Success Criteria** (what must be TRUE):

  1. The same driven run works on a docker host and on a rootless podman host with no configuration change
  2. A containerized run authenticates against the Claude subscription with the host `~/.claude` never mounted into it, and stays authenticated across a container rebuild
  3. Start, stop, and resume of a containerized session are all driveable from the TUI
  4. A session started in a container resumes correctly, and the driver's code path is identical to the host path apart from target selection

**Phase risks**:

  - **MUST-SPIKE (OQ5)**: podman rootless is entirely unexercised — no podman was installed on any research machine. Volume permissions (`--userns=keep-id`), `--format json` shape (docker NDJSON vs podman array), and general Docker-CLI compatibility are all LOW confidence. Auto-detect is a user-locked hard requirement, so install podman and exercise `bollard::connect_with_podman_defaults()` end to end before this phase's planning finalizes
  - The two-file credential trap: `~/.claude.json` lives outside `~/.claude`, so a volume at `~/.claude` alone does not keep the session signed in — set `CLAUDE_CONFIG_DIR` to the same path
  - Mount-path parity with the host is required or `--resume` will not find the session (lookup is scoped to the project directory and its worktrees)
  - Container is an `ExecutionTarget` enum inside the one executor — argv prefix plus path map swap — not a second `Executor` implementation, or stream-json parsing, interjection, cost accounting, and session handling all get duplicated and drift
  - Pin the CLI version in the image with the auto-updater disabled; run as a non-root user; restrict egress with an allowlist

**Research**: yes — `/gsd-plan-phase --research-phase`. Podman rootless uid mapping and volume permissions are asserted from Docker/devcontainer-centric docs, not verified
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 14. UI Fixes | 4/4 | Complete    | 2026-07-29 |
| 15. Transport Foundation | 8/8 | Complete    | 2026-07-29 |
| 16. Run Journal & State Substrate | 6/6 | Complete    | 2026-07-29 |
| 17. Supervisor | 8/8 | Complete    | 2026-07-29 |
| 18. Driver Tab, Live Watch & Durable Injection | 9/11 | In Progress|  |
| 19. GITSAFE — Git & Blast-Radius Envelope | 0/? | Not started | - |
| 20. Deterministic Decision Router & Run Bounds | 0/? | Not started | - |
| 21. LLM Goal Layer & Prompt-Injection Hardening | 0/? | Not started | - |
| 22. Container Execution Target | 0/? | Not started | - |

## Backlog

### Phase 999.2: Container Support with Claude Command Injection (PROMOTED → v2.0)

**Promoted 2026-07-29 into milestone v2.0.** The injection/monitoring plumbing became
Phases 15-18 (transport, journal, supervisor, driver tab) and the container work became
Phase 22. Retained here for provenance; no longer actionable as a backlog item.

**Goal:** Start, stop, and resume Claude sessions inside containers mapped to project directories. Monitor container output and inject commands directly into running Claude instances from the TUI -- enabling remote/isolated execution without terminal switching.
**Requirements:** TRANS-01..05, CTRL-01..05, OBS-01..07, STEER-01..03, CTNR-01..05
**Plans:** see Phases 15, 16, 17, 18, 22

### Phase 999.3: LLM-Driven Autonomous Project Execution (PROMOTED → v2.0)

**Promoted 2026-07-29 into milestone v2.0.** The decision layer became Phase 20
(deterministic router) and Phase 21 (LLM goal layer), sequenced after the 999.2 plumbing
per the user-locked ordering. Retained here for provenance; no longer actionable as a
backlog item.

**Goal:** Let an LLM agent — not a human — drive the GSD pipeline for a registered project to completion. The user states a goal once ("Build milestones 1-3, then brainstorm the next milestone autonomously, plan it, and execute it"); the agent decides which GSD command to run at which point and injects it into a Claude session, including `/clear` between stages to reclaim context. Model is the user's choice. Uses the Claude subscription via `claude -p` / an interactive session, not the API.
**Requirements:** DRIVE-01..06, SAFE-01..08, CTRL-06, CTRL-07
**Plans:** see Phases 19, 20, 21

Captured 2026-07-28. Severity: minor (nothing is broken without it). Open question from
capture: is it doable? — resolved by the 2026-07-28/29 research pass: yes, via duplex
`stream-json` over stdio.

Scope sketch (from capture, not yet designed):

- A driver loop that maps project state → next GSD command. The state-reader already
  exposes exactly the signals a driver needs (phase/plan counts, pipeline sub-stages,
  DRPEV position), so the decision function has a real input surface today.

- Prompt injection into a Claude session. `src/session_detector.rs` already finds live
  `claude` PIDs with their TTY, and `src/terminal_switch.rs` already resolves a TTY to a
  tmux pane — `tmux send-keys` to that pane is the shortest path to injection and
  gives live user interjection for free.
  *Superseded by research:* `--input-format stream-json --replay-user-messages` is a
  supported stdin injection channel with echo-back delivery confirmation. tmux is
  demoted to optional human attach/watch only; using it as the control channel is now
  an explicit anti-requirement.

- `claude -p` is the alternative transport: simpler and headless, but one-shot per
  invocation, so the driver owns cross-invocation continuity rather than `/clear`.
  *Confirmed by research as the chosen transport.*

- Overview must mark a project as LLM-driven, expose the originating prompt (so the
  goal is legible later), show live driver state, and allow injecting messages mid-run.
  *Became OBS-02/03/04 and STEER-01/02/03 in Phase 18.*

Known risks to resolve before planning — all now bound to phases:

- Conflicts with the project's **Non-intrusive** constraint → narrowed to **Opt-in
  driving** in PROJECT.md; enforced as a capability type at the spawn seam in Phase 17
  (CTRL-03).

- `tmux send-keys` is screen-scraping with no delivery confirmation → replaced by duplex
  `stream-json` with three-state delivery in Phases 15 and 18 (TRANS-01, STEER-02).

- Unattended runs hit permission prompts, checkpoints, and AskUserQuestion gates →
  park taxonomy in Phase 20 (DRIVE-05).

- Blast radius: an autonomous driver commits, branches, and pushes with no human in the
  loop → kill switch and dry-run in Phase 17 (CTRL-01, CTRL-02), git envelope in
  Phase 19 (SAFE-01..06).

- Overlaps Phase 999.2 → sequenced: 999.2 plumbing (Phases 15-18, 22) before 999.3
  decision layer (Phases 20-21), per the user-locked ordering.

Note: Backlog 999.1 (Milestone Archive Browser) promoted to Phase 12 in v1.2.
