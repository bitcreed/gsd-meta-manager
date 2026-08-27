# Roadmap: GSD Meta Manager

## Milestones

- ✅ **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- ✅ **v1.1 Polish & Power Features** - Phases 05-09 (shipped 2026-03-27)
- ✅ **v1.2 Housekeeping & Archive Browser** - Phases 10-13 (shipped 2026-04-01)
- ✅ **v1.3 Configuration & Pipeline Visibility** - 10 quick tasks (shipped 2026-05-09)
- ✅ **v1.4 Live Sessions & Document Browsing** - 4 quick tasks (shipped 2026-05-12)
- ✅ **v1.5.0 Sub-phase Artifact Detection** - 2 quick tasks (shipped 2026-05-15)
- ✅ **v1.6.0 GSD 1.8.0 Catch-up** - 1 quick task + 2 fast tasks (shipped 2026-07-22)
- 🚧 **v2.0 Autonomous Orchestration** - Phases 14-23 (in progress)

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

### v2.0 Autonomous Orchestration (Phases 14-23)

- [x] **Phase 14: UI Fixes** - Four display defects that misreport project state (completed 2026-07-29)
- [x] **Phase 15: Transport Foundation** - Duplex `stream-json` executor with envelope-derived outcomes (completed 2026-07-29)
- [x] **Phase 16: Run Journal & State Substrate** - Durable, redacted, cheap-to-read run record (completed 2026-07-29)
- [x] **Phase 17: Supervisor** - Detach, kill switch, dry-run, single-run lock, opt-in gate (completed 2026-07-29)
- [x] **Phase 18: Driver Tab, Live Watch & Durable Injection** - Manual ship point: run and steer from the TUI
- [ ] **Phase 19: GITSAFE — Git & Blast-Radius Envelope** - Mechanically enforced push boundary
- [ ] **Phase 20: Deterministic Decision Router & Run Bounds** - Rules pick the next command; runs stop themselves
- [ ] **Phase 21: LLM Goal Layer & Prompt-Injection Hardening** - One stated goal, model confined to two seams
- [ ] **Phase 22: Container Execution Target** - Docker/podman parity with the host path
- [ ] **Phase 23: Gate Policy & Auto-Validation** - The verify gate is a choice, not a law: skip, defer, or auto-validate

**Parallelism:** Phase 14 has no dependencies and is parallel-safe throughout.
Phase 22 depends only on Phase 15 and may run alongside Phases 17-21, but must land
before Phase 20 closes so the router is never built against a stubbed target.
Phase 23 depends on Phase 20's gate taxonomy and on Phase 22 (a containerized surface is
one of the surfaces `auto` must be able to drive), so it runs last.

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
**Plans**: 11/11 plans executed
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

- [x] 18-10-PLAN.md — Live output pane, four-state injection display, after-the-fact run review

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 18-11-PLAN.md — Help screen, dry-run preview, mechanical close-out

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
**Plans**: 8/8 plans executed

Plans:
**Wave 1**

- [x] 19-01-PLAN.md — Tracer: SAFE-01 end-to-end namespace refusal proved against a `file://` bare remote, with `src/envelope/`, the hidden `envelope` subcommand, the out-of-repo artifact directory and env-injected `core.hooksPath`

**Wave 2** *(blocked on Wave 1)*

- [x] 19-02-PLAN.md — SAFE-02: `classify_git` as one pure function over argv covering the whole denied set, the park-reason taxonomy, and the four `#[serde(default)]` `DriverOptIn` envelope fields with their migration literal

**Wave 3** *(blocked on Wave 2)*

- [x] 19-03-PLAN.md — SAFE-03: the `SecretClass` split of the shared pattern table, the full-worktree scan with its reported skip list, and the `git add -A` worktree-sweep guard (pre-commit, pre-push backstop, `.git/info/exclude`)

**Wave 4** *(blocked on Wave 3)*

- [x] 19-04-PLAN.md — SAFE-05: the child-environment scrub-and-rebuild as a pure value, the host-scoped askpass responder, and the empty-`HOME` `file://` push that proves pushing still works

**Wave 5** *(blocked on Wave 4)*

- [x] 19-05-PLAN.md — SAFE-06: the append-only PR ledger with an inclusive rolling 24h boundary, the network-free `PreToolUse` guard with an explicit timeout, and the round-tripped settings file

**Wave 6** *(blocked on Wave 5)*

- [x] 19-06-PLAN.md — The read-only remote-protection probe (`protected | unprotected | unknown`) and the pinned honesty statement as a fourth dry-run section constant

**Wave 7** *(blocked on Wave 6)*

- [x] 19-07-PLAN.md — Wiring: `--disallowedTools`/`--settings` on argv, the envelope environment in the one spawn closure, envelope establishment at the single options site, the positioned refusal, and `Parked`'s first emission

**Wave 8** *(blocked on Wave 7)*

- [x] 19-08-PLAN.md — `tests/async_blocking_guard.rs`, the project gate with the unchanged 5-lint delta, and criterion-by-criterion traceability

**UI hint**: no — this phase ships no visual surface. It delivers a policy module, two git hook
stubs, a `PreToolUse` guard, an environment envelope and a secret scanner. The one user-visible
string it adds is a pinned honesty paragraph in the existing dry-run preview and the existing run
journal. The repo-wide `ui-plan-gate` frontend detector fires on the ratatui codebase, not on this
phase's scope — the same reasoning Phases 15 and 16 recorded.

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
**Plans**: 5/5 plans executed in 3 waves

Plans:

- [x] 20-01-PLAN.md — Tracer: build the iteration loop and prove one routed command, bounded, ending with the detector named on disk (CTRL-06, DRIVE-02, DRIVE-06)
- [x] 20-02-PLAN.md — Terminal-record honesty: per-field scope classification on `run.json`, the caps in force on disk, the pinned dry-run contract, the async-blocking lint hole (DRIVE-06, CTRL-06)
- [x] 20-03-PLAN.md — The one reader: GSD's `executed` vocabulary, the verification frontmatter status, the disk-observable gate set, declared roadmap dependencies (DRIVE-05, DRIVE-02)
- [x] 20-04-PLAN.md — The complete rule table, the closed gate taxonomy, the goal-met predicate, and the conformance oracle against GSD's own router (DRIVE-02, DRIVE-05, DRIVE-06)
- [x] 20-05-PLAN.md — Quota park: observe the rate-limit signal on the transport, name the window and reset, and stop without retrying (CTRL-07, DRIVE-06)

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
  - The threat model is real, not theoretical: this tool drives other people's cloned repos, whose `CLAUDE.md` is third-party content. **CORRECTED by research (C-1/C-2):** structural delimiting alone does not achieve SAFE-07 — the CLI auto-loads `CLAUDE.md` itself, outside any boundary the driver controls, so the control is `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1` and explicitly **not** `--safe-mode`, which also disables the hooks Phase 19's envelope is enforced by. `--strict-mcp-config` is already emitted with **no** `--mcp-config`; supplying one would widen the permitted set from empty
  - Opt-in disclosure belongs here in UX terms — list the files that will enter prompts, record a content hash, re-confirm if `CLAUDE.md` changes after opt-in. **CORRECTED by research (C-4):** the existing hash is FNV-1a and its own doc says it is not a security control, so the digest is upgraded to SHA-256 behind a `sha256:` prefix
  - The driver sets its goal once, from a human. It may never enqueue itself more goals from artifacts the agent created during the run
  - **The vacuity hazard is the phase's sharpest risk (C-3):** the officially-recommended untrusted-content channel is accepted by this transport with exit 0 and never reaches the model, so an injection-corpus test built on it passes *vacuously*, forever. Every corpus assertion must prove arrival before it proves the property

**Research**: done — `21-RESEARCH.md` (five live spikes; four CONTEXT.md decisions contradicted and corrected in `21-CONTEXT.md` § "Research Corrections")
**Plans**: 25/26 plans executed (22/22 executed: 6/6 original in 6 waves, then 12 gap-closure plans in 11 further waves, then 2 round-7 and 2 round-8 plans in 4 more waves); verification pass 9 scored **4/5** ROADMAP criteria, **24/26** must-haves, status `gaps_found` — criteria 1, 2, 3 and 5 VERIFIED (criterion 1 now swept over all 170 `Cf` and all 4174 `Default_Ignorable` code points against Perl `Unicode::UCD` cross-checked with OpenJDK, zero escapes), criterion 4 PRESENT_BEHAVIOR_UNVERIFIED and **permanently agent-unclosable** (it needs an authenticated `claude` subscription; tracked as a standing item in `deferred-items.md` by explicit user decision). **No remaining item moves a criterion.** 4 further gap-closure plans (round 9, `21-23`..`21-26`) close the round-8 review's 5 Criticals and 5 Warnings plus pass 9's own items, and are not yet executed.

Plans:
**Wave 1**

- [x] 21-01-PLAN.md — Tracer: the seam argv/env profile, the untrusted-content boundary, the structured plan type, and the OQ1 gate answered live (DRIVE-01, DRIVE-03, SAFE-07, SAFE-08)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-02-PLAN.md — The fifth sibling taxonomy, an escalation cap resolved against the *resolved* step cap, and the journal table's fifth row (DRIVE-04)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 21-03-PLAN.md — SHA-256 digests, the prompt-input disclosure with its residual-exposure statement, and drift re-confirmation at the gate (SAFE-07, DRIVE-03)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 21-04-PLAN.md — The two seams wired: decomposition once above the loop as a moved capability, ambiguity at the router's no-rule branch, approval bound to plan *and* files (DRIVE-01, DRIVE-03, DRIVE-04)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 21-05-PLAN.md — The injection corpus: arrival proven before property, class by class, plus the matched `CLAUDE.md` suppression control pair (SAFE-07, SAFE-08)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 21-06-PLAN.md — The refusal record as evidence, no constructed command line, the cap park read off disk, and Phase 19's envelope shown firing independently (DRIVE-04, SAFE-08, SAFE-07)

Gap closure (from `21-VERIFICATION.md`):

- [x] 21-07-PLAN.md — Gap 1 / CR-01, tracer: an honest goal-only dry-run — a fourth `PreviewScope`, a promoted `CommandSource` with no unreachable arm, and the regression tests the invocation never had (DRIVE-01, DRIVE-03)
- [x] 21-08-PLAN.md — Gap 1 / CR-02 + WR-05: the approval's plan half becomes SHA-256, three false safety docs corrected, and the model-selected `target_phase` bounded at construction and sanitized before the terminal (DRIVE-03, SAFE-07)
- [x] 21-09-PLAN.md — Gap 2 / WR-01: `--approved-plan` carries both digest halves, so `PlanChanged` is reachable and the refusal stops reporting a file change that did not happen (DRIVE-01, DRIVE-03, SAFE-07)
- [x] 21-10-PLAN.md — Secondary findings WR-02 + WR-04: one stamped terminal write behind a guard, and an opt-in revert that restores rather than re-baselines (DRIVE-04, SAFE-07)

Gap closure, round 3 (from the re-verification in `21-VERIFICATION.md`, which found two NEW Criticals in the round-2 diff):

**Wave 1**

- [x] 21-11-PLAN.md — Tracer: review-CR-02 (a blank `--command` refused at the seam that already refuses a blank `--goal`) and review-CR-01 (the approval token parsed in the pure-refusal group, above the dry-run branch and above the seam, so a typo costs zero consultations), guarded by a degenerate-payload × argv-position matrix driven through the production resolver with a compile-forced variant classifier (DRIVE-01, DRIVE-03)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-12-PLAN.md — review-WR-01 (guard six scans every `src/` file and its header names each remaining limit with the direction it fails in), the `CommandSource` single-construction-site guard 21-11's design depends on, and review-WR-03 (`plan_digest`'s doc names the refusal production actually produces for a legacy value) (DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 4 (third consecutive cycle to introduce a new Critical while closing the last; premise validation in `21-PREMISES.md` found per-arm blankness validation BROKEN and the class is closed at the type level rather than patched a fourth time):

**Wave 1**

- [x] 21-13-PLAN.md — Tracer: a blank `CommandSource` payload becomes unrepresentable — a `NonBlank` newtype in a nested `mod payload` (so even sibling code in `driver/mod.rs` cannot construct one), the degenerate matrix rebuilt uniform and exemption-free with its column axis tied to `command_source`'s arity, the `trim` tautology broken by an independent detector, `is_plain_path_component` tightened to refuse whitespace-only, and the blank `target_phase` route through the model seam refused typed (DRIVE-01, DRIVE-03)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-14-PLAN.md — The guard layer made honest: guard six's marker-to-EOF blind region converted from silent to loud by a tree-wide boundary self-check, guard eight given a limits block and `Self::`-qualified needles, `run.rs`'s colliding second `CommandSource` renamed `IterationSource` with a single-declaration assertion replacing the collision watchdog, and its `(None, None) => Fixed(String::new())` arm — which manufactured the exact blank value this phase refuses — returned as a typed `Err` (DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 5 (round 4's `NonBlank` mechanism survived direct attack and regressed nothing — the first clean cycle — but was scoped to 3 of `DriveArgs`' 6 argv-derived string fields, and all three new Criticals landed in the other 3; the enumeration moves from human to compiler):

**Wave 1**

- [x] 21-15-PLAN.md — Tracer: `DriveArgs` itself becomes the domain — all six argv string fields typed `payload::NonBlank` behind one parse boundary whose exhaustive destructure with no `..` makes a seventh field a compile error until classified; both `(None,None) => String::new()` arms in `run.rs` become unrepresentable and the source resolves above every disk write (CR-01); one production predicate `text::carries_visible_content` replaces the two disagreeing definitions of blank, composed with — not replacing — the traversal and control-char checks (CR-02); `RunRecord.goal` written from `Option<NonBlank>` so `""` provably means absent (IN-01) (DRIVE-01, DRIVE-03)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-16-PLAN.md — The type-reading guard four cycles never had: guard nine asserts `DriveArgs` declares no bare `String` argv field, self-tested by a planted synthetic defect; `ITEM_OPENERS` gains `"pub("` with the backwards header bound deleted and planted-offender controls, the boundary scan extracted as a shared fn both the live assertion and its control call while keeping the `scanned_files >= 10` non-vacuity floor; and round-4's two violated prohibitions become standing assertions, with both plans required to audit their own prohibitions against their own diff (DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 6 (pass 6 verified criterion 1 for the first time — 5/5 ROADMAP criteria — so the goal-layer domain is closed; the remaining work is the *other* argv entry points and two anti-recurrence mechanisms measured weaker than their docs claimed):

**Wave 1**

- [x] 21-17-PLAN.md — The look-alike class, closed structurally rather than by enumeration: `text::carries_invisible_formatting` (one shared char class with `carries_visible_content`) and `test_support::LOOK_ALIKE_PAIRS` — the fixture shape the tree provably lacked, since every `DEGENERATE` payload is wholly invisible and none can express "renders identically, differs in bytes"; then the `Commands` domain — a `registry::Alias` newtype deleting the fourth blankness predicate, look-alike refusal at registration, `AliasNotVisible` replacing the borrowed `UnknownAlias`, six envelope re-entry arms fail-closed, and guard ten's 8-row alias census with a planted-ninth control (DRIVE-01, DRIVE-03, SAFE-08)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-18-PLAN.md — Both anti-recurrence mechanisms made to say what they deliver, certified only by planted-defect controls: guard nine's nine silent spellings (`pub(`, `Box<str>`, `Cow`, `&'static str`, `OsString`, trailing-`//`) fixed via one shared `is_field_opener`, with `OsString` deny-by-default over the whole body behind a two-entry suppress-only allowlist; `one_of_each`'s false compile-error claim deleted and its residual named; the `--alias`/`--run-id` matrix exemption removed; and the SUMMARY carrying Record corrections naming 21-16 truths 1 and 4 as measured-false (DRIVE-04, SAFE-07)

Gap closure, round 7 (pass 7 CORRECTED pass 6's claim that criterion 1 was verified — `git show 5b24022:src/text.rs` is byte-identical to HEAD, so the round-6 diff regressed nothing and closed nothing; pass 6 had attacked the class with U+200B and U+FEFF, two characters from INSIDE the implementation's own list, and so measured the implementation against itself, while U+202E, U+00AD, U+E0041 and U+FE0F still pass every seam. The round's shape follows from that: stop enumerating the class and DERIVE it, and invert identity from a deny-list that can always be one code point short to an allow-list that cannot):

**Wave 1**

- [x] 21-19-PLAN.md — The invisible class derived instead of hand-enumerated a seventh time: `text::is_invisible_formatting_char` answers `General_Category=Cf` ∪ `Default_Ignorable_Code_Point` from `icu_properties` compiled data with no literal range surviving in the function, consumed by BOTH judgments — the emptiness half (`carries_visible_content`, criterion 1's own question, the half no round had named) and the identity half; the falsifying corpus stops sharing a source with the implementation, becoming an exhaustive all-codepoints sweep oracled by the independently maintained `unicode-properties` dev-dep with its `general-category` feature pinned explicitly, carrying a COMMITTED `format_seen >= 150` non-vacuity floor (measured against 170 Cf code points at Unicode 15.0) so the sweep cannot pass green forever over an empty set, and over-detection bounded PAST ASCII by ten named visible non-ASCII members spanning Latin-accented, CJK, Hangul, Arabic, Devanagari, Hebrew, Greek, Cyrillic and Thai, so a derivation bug that made real script invisible goes red in the tree instead of shipping as a narrowing D-19-1 never disclosed; and identity inverted to the allow-list `[A-Za-z0-9._-]` — one `text::is_identity_char` spelling as a clause of `journal::is_plain_path_component` — closing bidi (Trojan Source, CVE-2021-42574), the U+E0000–E007F tag block, variation selectors and homoglyphs at every run-directory, envelope-root, credential-scope, phase-token and run-id seam in a single clause, with `Alias::new`'s recorded product trade, escaped `display_identity` rendering for legacy entries, and the remove-and-re-add recovery route D-19-2's "not one-way" rating rests on certified by `a_legacy_alias_the_alphabet_refuses_is_still_removable` rather than by prose (DRIVE-01, DRIVE-03, SAFE-08)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-20-PLAN.md — The anti-recurrence machinery made honest for the fifth consecutive round, every claim moved down to what a planted-defect control committed red-first actually measures: guard nine's `is_field_opener` widened to bare (no-`pub`) field declarations — pass 6's exact silent signature `field_lines=12 protected=6 offenders=[]` reproduced as a plant — and `judge_declaration` judging `names_string_payload` INSIDE the `OsString` branch instead of returning early, with the `OSSTRING_ALLOWED` integrity pin becoming an EQUALITY on the parsed declared type text rather than a `contains` that a compound `(Vec<OsString>, String)` inherits suppression through; the `DEGENERATE` uniqueness scan drawing at least two witnesses from different members (one from 21-19's four new outside-the-old-ranges values) with its under-detection direction named in words; guard ten's census rows asserted to name variants that still exist in `src/cli.rs`; the SAFE-07 ignored-set census made ACTIVE and anchored to line-start `fn` declarations with its pattern assembled at runtime so it cannot match its own source (seven arm declarations, ten line-anchored `#[ignore]` attributes over a raw substring count of fourteen), sharing `is_comment_line`/`is_ignore_attribute_line` with the file's pre-existing self-scan so two scanners cannot drift about what an ignore attribute is; the boundary-never-executed fact moved into `deferred-items.md` as a standing tracked item with its exact command rather than a SUMMARY qualification read once; and every artifact pinning an assertion identifier instead of a prose grep, plus a Record-corrections table naming 21-17 truths 1/2/6/8 and 21-18 truths 1/2/5 false as shipped (DRIVE-04, SAFE-07)

Gap closure, round 8 (pass 8 scored 4/5 with 0 FAILED — the first pass in this phase to find no gap. Criterion 4 is permanently agent-unclosable and stays tracked in `deferred-items.md` by explicit user decision, so **no plan here chases a criterion**. Round 8 exists to close round 7's 1 Critical and 4 Warnings plus pass 8's own 4 Warnings. CR-01: the TUI half of D-19-5 shipped into `src/ui/project_list.rs`, a file the module tree has not contained since `c297631` — three `display_identity` call sites in `src/`, one of them dead, and the live render surface never enumerated. Round 8's rule: **the render surface closes BY DERIVATION, not by a longer hand-list** — a site added tomorrow, in a file these plans never touch, must go red with nobody editing a list):

**Wave 1**

- [x] 21-21-PLAN.md — The render surface closed by three mechanisms whose reach is MEASURED rather than assumed, none claimed to do another's work: (1) the **compiler** — `impl Display for Alias` withdrawn from `src/registry.rs:168`, measured by planting the withdrawal and reverting it at exactly FOUR sites (`registry.rs:640`, `main.rs:103`/`:111`/`:371`) and stated to reach no further, because the `Remove` arm binds a raw argv `String` and the TUI carries `Vec<String>`; (2) the **one producer** — `Display for AliasRefusal` escapes the alias it embeds, so `main.rs:91` and `:359` become correct WITHOUT appearing in the diff, which is the criterion rather than a nicety; (3) a **source-derived, deny-by-default census over the `Screen` trait** — a recursive `read_dir` walk of `src/` collects every `impl Screen for` (eleven at HEAD) and `assert_eq!`s that derived set against a disposition table in BOTH directions, so a twelfth screen added tomorrow is unadjudicated and red, proven by really adding one and quoting the red. Each `renders-identity` row is then checked BEHAVIOURALLY — the screen renders through its real `Screen::render` into a ratatui `Buffer` over hostile/clean twins imported from `test_support::LOOK_ALIKE_PAIRS`, asserting arrival (the clean twin present) BEFORE the escaped form, and zero characters satisfying `is_invisible_formatting_char` — so no sink spelling is invisible, where a syntactic scan would face 598 sink calls and 77 candidate identity-to-sink lines. Corrects an inherited claim neither the review nor pass 8 measured: **ratatui 0.30's buffer DROPS zero-width graphemes** (`gsd-\u{202e}nur` renders `gsd-nur`) while the tag block SURVIVES, so the TUI silently deletes rather than reorders and an assertion on the raw form's absence would be vacuous forever. Deletes `src/ui/project_list.rs` with the `c297631` evidence and fixes its two dangling citations; splits `remove`'s ACCEPT from its ECHO structurally via a no-`Display` `LegacyRegistryKey` so D-17-3's raw acceptance cannot regress while the echo cannot compile unescaped; pins `AliasRefusal::NotPlainComponent` for `".."`/`"."` (WR-04, keep-and-pin — it is also the certificate that the alphabet clause did not subsume the traversal check); and corrects 21-19 truth 5, `21-19-SUMMARY.md:180` and named-shape row 8 append-only in the commit that falsifies them (DRIVE-01, DRIVE-03, SAFE-08)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 21-22-PLAN.md — The sentences that would get a correct mechanism removed or a correct fix reverted, ordered by that harm: the `narrow_visible` assertion catches a bare field but DIAGNOSES the opposite cause, so an executor repairing under it would narrow `is_field_opener` back and undo round 7's fix — **treated as the highest-value item and ordered first**, rewritten to name both causes with the likelier first and certified by re-planting a bare field in the real `DriveArgs` body and capturing what the executor reads; `WITNESS_ALLOWED_ELSEWHERE`'s doc replaced (the TABLE explicitly KEPT — a both-ways `assert_eq!` over a hygiene scan, so a stale row breaks loudly) with both residuals named WITH their directions and **each MEASURED by a plant** rather than argued, because a doc asserting an untested residual is the same overclaim pointing the other way; `hooks.rs`'s security rationale corrected from the post-D-19-2 falsehood that `is_plain_path_component` accepts a quote character — the sentence a maintainer reads before deleting `sh_quote` — with the defence-in-depth reason the quoting survives an alphabet widening; WR-03 closed by making `src/text.rs:155`'s one-spelling claim TRUE through delegation of `advisory.rs`'s second alphabet to `text::is_identity_char`, certified by a spelling census committed RED at two executable occurrences and green at one; `carries_visible_content`'s free-text residual DISCLOSED with its direction and its two completeness bounds (the pinned Unicode version and the hand-named default-ignorable members with no second machine oracle). Records without fixing: the four PRE-EXISTING `--all-targets` clippy lints with the empty `git log 2074595..HEAD` evidence so the gate's failure is never mistaken for round-8 breakage, a new standing staleness obligation that invisible-character rendering is a property of the ratatui VERSION, and criterion 4 re-surfaced unchanged (DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 9 (pass 9 scored 4/5 ROADMAP criteria, 24/26 must-haves, `gaps_found`. Criterion 4 stays permanently agent-unclosable and tracked in `deferred-items.md` by explicit user decision, so **no plan here chases a criterion**. Round 9 closes the round-8 review's 5 Criticals and 5 Warnings plus pass 9's own items. **The user's explicit decision drives the shape: type-level inversion, not a ninth site sweep.** Eight rounds show that site-sweeping does not converge and that type-level inversion does — identities were closed permanently in round 7 by making a bad identity *unconstructable*, while the display surface was still being fixed the old way, and round 8's review found four more classes of site the round could not see. Round 9's rule: **make an unescaped untrusted string UNRENDERABLE, so CR-01..CR-05 fall out as consequences rather than as a list of patches.** The lever's reach is measured and its uncovered remainder is named, carrier by carrier, because an unmeasured claim of reach is what produced round 8):

**Wave 1**

- [x] 21-23-PLAN.md — TRACER: the carrier, the one composition, and the git-history pane end to end. `crate::text::Untrusted` generalises round 8's `LegacyRegistryKey` shape from ONE call site to the DATA CARRIERS — no `Display`, no `AsRef<str>`, no `Deref`, no `Borrow<str>`, no `Into<Cow<'_, str>>`, a hand-written `Debug` that prints the escaped form, and two accessors named after the questions (`as_raw_for_logic_only`, `shown() -> Rendered`); `Rendered` is the ONLY escaped type that IS `Into<Cow<'static, str>>`, so at a render site the escaped path is the short one. **The constraint is named honestly:** `Span::raw` takes `Into<Cow<'_, str>>` and `String: Into<Cow>` is a std impl, so there is no global forbid — measured at HEAD, 129 `Span::raw`, 198 `Span::styled`, 502 sink calls under `src/ui/`, and the lever gates NONE of them. What it gates is 21 fields across 9 of 15 carrier types, and the SIX it does not (`ProjectState` 5 fields/135 mentions/a `HashMap` key and router input; `RoadmapPhase`; `QueuedAction`; `filtered_aliases` at 93 mentions and D-21-4's measured 25-error cascade; `status_message`, whose boundary is inside a `format!`; `GsdConfig`) are tabulated with the measurement that excludes each. Wires `GitLogEntry`'s four fields end to end (producer → `Action` → cache → render → probe), closing **CR-04 — live Trojan Source in the git-history pane of a tool whose stated threat model is driving other people's cloned repos**. Makes CR-04 REPRODUCIBLE ON DEMAND by populating `git_entries` in `probe_ctx`, which settles pass 9's "flake" as a defect rather than nondeterminism — and notes honestly that the offered reconciliation ("the cache happened to be populated") does not survive reading the fixture, since `probe_ctx` performs no I/O. Corrects **CR-03** in the commit that falsifies it: zero-width dropping is a `Paragraph` property, NOT a `Buffer` property — re-derived in a scratch crate against ratatui 0.30.2, `U+202E`/`U+200B`/`U+00AD`/`U+2062`/`U+2065`/`U+FEFF` are dropped by `Paragraph` and ALL survive `Block::title` and `ListItem` — so LIMIT 4's declined raw-absence assertion is REINSTATED where it is now known to be non-vacuous. Resolves **WR-01/WR-02** once, in one place, by moving the ESC/C0/DEL/C1 class into `text::strip_terminal_controls` and composing it with `display_identity` as `text::render_for_terminal` — and records that the review's proposed one-liner was wrong as written, because `sanitize_render_line` truncates to 512 characters. Pins the `TAG_PAIR` teeth by RENDERING rather than by pinning the index (pass-9 Warning). Certifies the carrier's three absent conversions with an autoref-specialization control asserting BOTH directions, observed red by planting — the certificate **WR-04** says `LegacyRegistryKey` never had (SAFE-07, SAFE-08, DRIVE-01)

**Wave 2** *(blocked on Wave 1 completion; 21-24 and 21-25 have zero `files_modified` overlap and run in parallel)*

- [x] 21-24-PLAN.md — The error and lookup layers, closed by the SAME carrier so **CR-01** and **CR-02** arrive as consequences. `OptInError`'s four alias variants and `DriveError`'s `AliasNotVisible`/`RunIdInvalid`/`TargetPhaseInvalid` carry `Untrusted`, so both `Display` impls stop compiling until every interpolation is `shown()` and `src/main.rs`'s four `eprintln!` sites become correct **without appearing in the diff** — which is the criterion, and is an acceptance check. `remove_project` takes `&LegacyRegistryKey`, making `bail!("Project not found: {}", alias)` unwritable, with the verbatim compile error captured to replace the remembered one at `main.rs:162-173`; all three spellings of that sentence collapse into one escaping producer, `registry::project_not_found`. Lands the **promote**: `LegacyRegistryKey` is demoted to the argv-lookup variant of `Untrusted` and its `#[derive(Debug)]` is removed (**WR-04**), measured unused first. Closes **WR-01**'s ESC half at the CLI — measured at the binary, `ev\u{1b}[31mil` printed a live ANSI sequence through `list` and `remove` before the fix. `driver_confirm.rs`'s `do_start_run` stops re-spelling a refusal `OptInError::NotOptedIn` already owns. One shared control drives BOTH error types and BOTH halves of `remove` over imported fixtures plus ESC and C1 witnesses, asserting non-vacuity then arrival then the property, with its subject list compile-forced by a wildcard-free `match`. **Two source-artifact corrections made by measuring:** the type the review calls `DriverError` does not exist in this tree (it is `OptInError`), and D-17-3's raw acceptance is re-measured at the binary rather than assumed (DRIVE-01, DRIVE-03, SAFE-08)
- [x] 21-25-PLAN.md — The `.planning/` layer, same carrier: `BacklogItem` (4 fields), `ArchiveFile::name`, `PhaseArchive::{name, display_name}`, `BrowserEntry::name`, `ClaudeSession::session_id` and three `ProjectViewCache` fields take `Untrusted`, so pass 9's five raw sites (`detail.rs:2882`, `:3349`, `:3416`, `:3438`, `:3444`) become compile errors — and the SUMMARY records how many MORE the compiler names, because that difference is the argument for the mechanism. Three of the five are `ListItem`s, which per CR-03's correction PRESERVE the whole invisible class. Closes the probe's disclosed coverage hole rather than re-disclosing it: `probe_ctx` populates every cache the eleven `DetailScreen` sub-views read, so each renders its POPULATED branch on every run, with **arrival recorded per tab** so a populated cache the render never reads is reported rather than counted — the vacuity hazard the ROADMAP names as this phase's sharpest risk, one level up. LIMIT 1 must come out strictly narrower than the wording it replaces. **WR-03**: the status footer at `normal.rs:811` is escaped at the RENDER site — the one deliberate inversion of the escape-at-the-producer rule in round 9, because the boundary runs through the middle of a `format!` in `app.rs` — argued at the site, given a fixture state that provably reaches the branch, and finally named in `NormalScreen`'s disposition row. Also rewrites the `sid[..8]` byte slice that panics on a multibyte session id. The four pre-existing clippy lints are protected by a CHECKED property (exactly four, same two kinds, same two files) rather than by "do not touch", since this plan must edit `src/browser.rs` (SAFE-07, DRIVE-03)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 21-26-PLAN.md — The two mechanism defects that CANNOT fall out of a carrier, said plainly, plus the record. **CR-05** is closed by making an unadjudicated screen FAIL TO BUILD: `Screen` gains a sealed supertrait `RenderAdjudicated` with two object-safe methods, implemented only through an `adjudicate_screen!` macro, so `impl Screen for X` on an unadjudicated `X` is `error[E0277]` — verified by the planner in a scratch crate, along with `Box<dyn Screen>` still working (methods, never associated consts, because an associated const would make `Screen` dyn-incompatible and break every screen transition). **Both of the round-8 reviewer's plants are covered by that one bound**, including the macro-generated implementor, because a trait bound has no spelling to be short of. The disposition is PROMOTED onto the screen and read off the instance by the probe, so the checked and the declared disposition cannot drift; `SCREEN_IDENTITY_DISPOSITIONS` is demoted to a name-to-fixture map with its narrowed job documented in the same commit. The source walk is kept and its floor raised anyway — joined logical lines, an unnameable `impl` reported as an offence rather than skipped, and `census_offences` keyed on `(name, path)` (**IN-01**) — each observed red against a planted defect. **WR-05**: the alphabet needle is normalized so arm order and spacing stop mattering, red-first against a planted reordered copy, with the `default_branch_of` exclusion asserted since over-matching is the new failure direction. Records in `deferred-items.md`: the six carrier types round 9 did NOT retype with the measurement and failure direction of each; the per-widget ratatui obligation; **IN-04**'s process lesson (a prohibition against inheriting numbers that carries one named exemption will be falsified at the exemption); **IN-02**/**IN-03** deferred with reasons; and criterion 4 re-surfaced verbatim with no work claimed against it. **Corrects a count by measuring: the tree holds ELEVEN `Screen` implementors, not thirteen — the thirteen was eleven plus the reviewer's own two plants** (SAFE-07, SAFE-08, DRIVE-04)

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

### Phase 23: Gate Policy & Auto-Validation

**Goal**: Whether a human-judgement verification gate stops the run is the user's configured choice, and under `auto` the driver earns the verification by driving the real surface rather than asserting it
**Depends on**: Phase 20 (the gate taxonomy and the router's safe alphabet), Phase 22 (a containerized surface is one of the surfaces `auto` must be able to drive)
**Requirements**: CTRL-08, DRIVE-07
**Success Criteria** (what must be TRUE):

  1. The gate policy is settable per-run on argv **and** per-project/session in config, with a documented precedence between them, and `defer` remains the default when neither is set
  2. Under `skip` a run continues past a human-judgement gate instead of parking, and the run record shows that a gate was skipped, which gate it was, and under whose configured choice — a skipped gate is never invisible
  3. Under `defer` behaviour is byte-identical to Phase 20's: the run parks with the gate named
  4. Under `auto` the driver attempts the verification by driving the real surface a human would use, and a surface it cannot drive is reported as un-attemptable rather than silently passed
  5. An auto-validated verification result is permanently distinguishable on disk from a human-verified one, by any later reader, including GSD's own tooling

**Phase risks**:

  - **The `auto` mode's whole value depends on criterion 5.** A machine-written `status: passed` that a later reader cannot tell apart from a human's does not automate the gate, it removes it. Design the record shape before the automation.
  - Phase 20's conformance oracle encodes the current `/gsd-verify-work` exclusion as a declared divergence from upstream that fails **in both directions**. It must be taught the modes, not weakened, or it goes red the first time `skip` is selected
  - `skip` is the mode most likely to be reached for casually and least likely to be reviewed. It must be a recorded, visible choice at every layer — never a default, never implicit, never inherited silently from a stale session config
  - Related open exposure, disclosed by 20-04 and explicitly NOT closed by Phase 20: nothing mechanically prevents a driven agent writing `status: passed` into a `*-VERIFICATION.md` itself. `auto` makes a legitimate machine-written pass a normal event, which removes the anomaly signal that would otherwise expose the illegitimate one. Resolve the two together or say plainly why not
  - Driving a real surface (TUI/browser/app simulator) is a genuinely new capability class for this codebase — no in-tree precedent. Expect a spike

**Research**: yes. No in-tree precedent for driving a UI surface; the recording-shape question in criterion 5 is a design artifact owed before implementation
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 14. UI Fixes | 4/4 | Complete    | 2026-07-29 |
| 15. Transport Foundation | 8/8 | Complete    | 2026-07-29 |
| 16. Run Journal & State Substrate | 6/6 | Complete    | 2026-07-29 |
| 17. Supervisor | 8/8 | Complete    | 2026-07-29 |
| 18. Driver Tab, Live Watch & Durable Injection | 11/11 | Complete    | 2026-07-29 |
| 19. GITSAFE — Git & Blast-Radius Envelope | 8/8 | In Progress|  |
| 20. Deterministic Decision Router & Run Bounds | 5/5 | In Progress|  |
| 21. LLM Goal Layer & Prompt-Injection Hardening | 25/26 | In Progress|  |
| 22. Container Execution Target | 0/? | Not started | - |
| 23. Gate Policy & Auto-Validation | 0/? | Not started | - |

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
