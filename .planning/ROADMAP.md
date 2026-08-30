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
**Plans**: 8/8 executed, plus 2 UAT gap-closure plans (19-09, 19-10) and 9 security gap-closure plans (19-11, 19-12, 19-13, 19-14, 19-15, 19-16, 19-17, and 19-18/19-19 pending) across five audit rounds

Plans:

- [x] 19-18-PLAN-CHECK.md

- [x] 19-16-PLAN-CHECK.md

- [x] 19-13-PLAN-CHECK.md
- [x] 19-13-PLAN.md
- [x] 19-14-PLAN-CHECK.md
- [x] 19-14-PLAN.md
- [x] 19-15-PLAN.md
- [x] 19-16-PLAN.md
- [x] 19-17-PLAN.md
- [x] 19-18-PLAN.md
- [ ] 19-19-PLAN.md

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

**Wave 9** *(gap closure from the human UAT pass of 2026-08-28 — both plans independent, no shared files, run in parallel)*

- [x] 19-09-PLAN.md — G-19-1: `SECTION_ENVELOPE` rewritten to one sentence + both cases explained with examples + the branch-protection conclusion, with all seven pinned substrings verbatim and a new legibility cap observed RED at 250 tokens
- [x] 19-10-PLAN.md — G-19-4: per-test aliases in `tests/driver_lock.rs` so no two writers share one `settings.json`, the shared-path mechanism demonstrated at the real seam, and an early-dying child reported with its exit status and stderr instead of as a 30s lock timeout

**Wave 10** *(gap closure from `/gsd-secure-phase 19`, 2026-08-29 — the one high-severity threat blocking the gate; 19-12 depends on 19-11)*

- [x] 19-11-PLAN.md — T-19-60: the `PreToolUse` guard classifies on `words[0]`, so a wrapper or a `NAME=VALUE` prefix walks a force push and a PR past layers 1, 2 and 3. Closed structurally by `policy::resolve_program` — consume assignment words, then find the first token whose basename is a program the envelope already governs — with the six measured lines committed RED first and the `NESTED_SHELLS` list deleted rather than extended
- [x] 19-12-PLAN.md — The control over the class rather than the instances: a fixed-seed generator asserting the verdict is invariant under any wrapper chain, an alphabet whose names are mechanically proved absent from `src/`, a paired allow corpus so the fix cannot be "deny everything", and the one disclosed residual bounded on both sides

**Wave 11** *(gap closure from audit 2 of `/gsd-secure-phase 19`, 2026-08-29)*

- [x] 19-13-PLAN.md — T-19-60's wrapper-operand sub-class closed by establishing COMMAND POSITION structurally (head shortcut, refusal on two candidates behind a wrapper prefix), T-19-81 on both halves, T-19-82's key-set gap with an unfiltered drift pin, and T-19-83's corpus vacuity — with T-19-86 registered, pinned at its current permitted verdict and left open

**Wave 12** *(gap closure from audit 3 of `/gsd-secure-phase 19`, 2026-08-29 — three findings, one root cause; split into Rule A and Rule B so each is shown separately load-bearing, 19-15 blocked on 19-14)*

- [x] 19-14-PLAN.md — Rule A. T-19-88 and T-19-90, plus six of T-19-87's eight rows: `Token.expansion` is unavailable to `classify_git` and `pr_command_label`, so the words every classifier decision turns on are exempt from every expansion rule. Closed by governing the DECISION REGION — exactly the words each matched arm reads: the git verb plus `config`'s key operand, the forge's first TWO subcommand words plus the `api` method, endpoint and both flag-ness spellings — with operands left free so commit messages and PR titles carrying `$` keep working. Delivers the Rule A half of the corpus widening with a fresh-root forge-slot property, registers T-19-91, and **ends with two severed-prefix rows deliberately RED** for 19-15
- [x] 19-15-PLAN.md — Rule B. T-19-87's remaining two rows and T-19-89: a segment whose immediately preceding operator is a word-splitting CLOSER continues an enclosing word, so its first token is not a command position and a governed program found there is refused. POSITIONAL — reads no name, no substring and no length; `SEPARATORS` unchanged, the flush flag set for `( ) { }` only, the opener excluded so command substitutions keep being classified. Confirms 19-14's carry-forward rows still red before fixing them, completes the alphabets, corrects the false T-19-87 pin by deleting it and re-homing its rows

**Wave 13** *(gap closure from audit 4 of `/gsd-secure-phase 19`, 2026-08-29 — four findings; the rule is INVERTED rather than extended, and the corpus is widened FIRST so the round cannot be the fifth certified by alphabets that could not fail on its class; 19-17 blocked on 19-16)*

- [x] 19-16-PLAN.md — The corpus, first, and RED. T-19-95: `EXPANSION_METACHARACTERS` is exactly `['$', '`', '{', '(']` and no entry of any alphabet contains a `{a,b}`, a `*`, a `?` or a `[`, so four rounds running the corpus could not fail on the class the next audit walked through. Widens seven alphabets across the four classes that make a word UNREADABLE — brace expansion, LITERAL brace pair, pathname expansion, tilde — split by whether each alphabet's property asserts refusal or invariance, with per-alphabet AND per-class floors. Writes audit 4's T-19-92 / T-19-93 / T-19-94 reproducers and five cells found while planning as a fourth evidence file, each measured against the built binary and confirmed under bash shims BEFORE being asserted, with T-19-93's rows written as COUNTED (a ledger line plus a second creation under `pr_cap_exceeded`) rather than refused. **Zero `src/` hunks; ends deliberately RED**, and reports a finding instead of proceeding if any axis is green
- [x] 19-17-PLAN.md — The rule, INVERTED. A governed segment's DECISION WORDS must be LITERAL — the shell hands the word to the program byte-identically to how it is written — established by positive evidence gathered by `tokenize` as the word is consumed, so brace expansion, pathname expansion, tilde expansion and `$IFS` re-splitting close in one rule. The region does not move and operands stay free. `{` gets bash's own three-way classification (parameter expansion / reserved word / brace pair), with `SEPARATORS` unchanged and the parameter-expansion case preserved so Rule B stays load-bearing; a comma-free brace pair is absorbed as one word, which is what makes `gh api repos/{owner}/{repo}/pulls` COUNTED without touching either forge scan. Clause 2 refuses a brace-spliced simple command on both halves, folded into a post-filter made EXHAUSTIVE. Corrects the T-19-91 `git push $REF` record without closing it. **T-19-86, T-19-91 and T-19-96 stay open at `high` — `/gsd-secure-phase 19` is NOT cleared**

**Wave 14** *(gap closure from audit 5 of `/gsd-secure-phase 19`, 2026-08-29 — audit 5 verified round 4's inversion is genuine and CLOSED the whole word-ASSEMBLY class, then found the gap had moved AXIS rather than one slot over: from* how a word is written *to* which words arrive*. Corpus first again, on the seam that worked twice; 19-19 blocked on 19-18)*

- [ ] 19-18-PLAN.md — The corpus, first, and RED, on a SECOND AXIS. T-19-99: all seven `UNREADABLE_CLASSES` are word-ASSEMBLY classes, `grep -rnE '"(git|gh|glab)[^"]*[<>][^"]*"' tests/ src/` returns nothing and neither corpus file holds a backslash-newline, so the corpus is structurally incapable of failing on T-19-97 or T-19-98. Adds `DELETION_CLASSES` **beside** a byte-identical `UNREADABLE_CLASSES` — five degenerate-proof predicates spanning a deleted word, an ATTACHED operator, a multi-character or fd-carrying operator and both positions of a line continuation — with a displacement alphabet spliced where the hole actually is (between the governed program and its decision words; the prefix and trailing positions are already refused and are pinned as controls). Writes audit 5's sixteen T-19-97 and eight T-19-98 reproducers plus seven cells found while planning as a FIFTH evidence file, each measured against the built binary and confirmed under bash shims — the continuation rows from a file whose bytes `od -c` verified — and pins the corpus to fail in BOTH directions, including the over-deletion control `git x2>/tmp/o push --force origin main` (bash gives git `[x2] [push] …`) and the two pre-existing FALSE REFUSALS this round removes. **Zero `src/` hunks; ends deliberately RED**
- [ ] 19-19-PLAN.md — The rule, COMPLETED rather than replaced: **the words the guard classifies must be exactly the words the program receives, in the same order.** Round 4's `Token.literal` is one half and audit 5 verified it complete; word SURVIVAL is the other. Design option (a) — `tokenize` MODELS deletion — taken over option (b) on a measured cost: refusing governed commands that carry a deletable token would refuse `git log > out`, `gh pr list 2>/dev/null` and, decisively, `gh pr create --title x > /tmp/o`, which is COUNTED with a ledger line today. Deletion happens in the ONE walk, so a deleted word never becomes a `Token`, every decision index is over the surviving argv, and `first_unreadable_decision_word`'s one closure needs **no second reading site**. `SEPARATORS` is byte-identical and `is_separator(">")` stays false — the doc's reasoning is right and only its unexamined consequence is corrected. The production is `[IO_NUMBER|{varname}] OPERATOR WORD` with `&>` recognised before `&` reaches the separator arm, and anything it cannot complete marks the command UNRESOLVABLE through the arms the exhaustive post-filter already has. `\`+newline is consumed producing no character in the unquoted arm and the double-quote loop, with `literal` deliberately left TRUE because a deletion is not a rewrite. **Net cost is NEGATIVE: two measured over-refusals removed. T-19-86 and T-19-91 stay open at `high` — `/gsd-secure-phase 19` is NOT cleared**

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
**Plans**: 37/37 plans executed — 35 executed, **2 pending (round 13: `21-36` wave 1, `21-37` wave 2)**. Verification pass 13 scored **98/100 must-haves, 4/5 ROADMAP criteria**, status `gaps_found`, with **three new gaps and one root cause**: for the third consecutive round a fix was certified by a corpus structurally incapable of failing on the class at issue. Round 12's headline claim — byte-identity between what the producer emits and what the consumer reads back — is FALSE (`trim()` at `session_detector.rs:285` rewrites the id, and a whitespace-only id reads back `None`), the invariant covers ONE of this build's TWO `claude` argv producers (`executor::claude::build_argv`'s `--session-id <uuid>` reads back `None`, as does the CLI's short `-r`), and `from_utf8_lossy` fabricates an id from non-UTF-8 wire bytes that the `&str`-typed harness cannot even express. **Round 13 attacks the corpus's SHAPE, not the three instances:** the claim becomes a total property with two named refusal classes, the enumerated corpus is demoted to a seed for a fixed-seed mixture generator over `Vec<u8>` with a committed non-vacuity floor, the harness is retyped to bytes, and the producer set becomes a source-derived adjudicated census. `proptest` was weighed against the project's own `icu_properties` precedent and DECLINED with the graph cost MEASURED — 14 newly locked packages, `Cargo.lock` 354 → 368 — because that precedent turns on a crate supplying external reference data, and no external source defines the set of session-id classes. Earlier history: 34/34 plans executed — 26/26 executed (6/6 original in 6 waves, then 12 gap-closure plans in 11 further waves, then 2 round-7, 2 round-8 and 4 round-9 plans in 7 more waves); verification pass 10 scored **4/5** ROADMAP criteria, **30/33** must-haves, status `gaps_found` — criteria 1, 2, 3 and 5 VERIFIED, criterion 4 PRESENT_BEHAVIOR_UNVERIFIED and **permanently agent-unclosable by construction** (it needs an authenticated `claude` subscription; tracked as a standing item in `deferred-items.md` by explicit user decision). Pass 10 confirmed three Criticals by reading the code directly rather than trusting the review: an `sh -c` shell injection from a `/proc`-scraped session id (CR-01), the invisible-formatting class reaching the driver output pane that shows the LLM's own prose (CR-02), and an escape laundered through the Defaults edit buffer (CR-03). **CR-01 does NOT falsify criterion 5** — it is a human-triggered TUI action, not the model's chosen action, and that scoping is settled. Round 10 (`21-27`..`21-30`) is now **executed** in 2 waves (wave 1 three-wide in parallel worktrees), and **verification pass 11** scored **4/5** ROADMAP criteria and **40/45** must-haves, status `gaps_found`. Pass 11 confirmed all three pass-10 Criticals CLOSED — CR-02 and CR-03 closed *at the type* (`DriverOutputLine::text: Untrusted` with `push_record` the sole construction site; `EditBuffer` over `Untrusted`) — and criterion 4 remains the expected, permanently agent-unclosable item. It opened **4 new gaps, all one species: a completeness claim wider than the control that certifies it**, and three of the four sit in mechanisms round 10 built to end that species. The blocker: `21-27` replaced the `sh -c` shell injection with an **argument injection** — `resume_terminal_argv` emits `[sep, "claude", "--resume", <untrusted>]` with **no `--` end-of-options separator**, `read_session_id` validates only non-emptiness, and no fixture in `hostile_session_ids()` begins with a hyphen, so the committed control passes against a build shipping it. Blast radius is bounded to a same-user process named `claude`. The other three: the interpreter census misses the CR-01 shape at >=12 intervening comment lines; `driver.rs`'s composition census over-joins sibling `match` arms; and `every_render_site_under_ui_composes_both_classes` is driven by 6 executable lines in 2 of 16 files (hand-tracing 63 sites outside the needle found no live leak — a false completeness claim, not a leak). **Round 11 is now PLANNED: 4 plans (`21-31`..`21-34`) in 2 waves, wave 1 three-wide with a computed-empty pairwise `files_modified` intersection — 34/34 plans, 30 executed and 4 pending.** Planning it produced one finding of its own: the planner measured the `claude` CLI's option parser rather than assuming it, and **pass 11's own recommended fix for gaps[0] is wrong** — a `--` end-of-options separator closes the injection and silently DELETES the resume (`claude --resume -- <valid-uuid>` returns the same `requires a valid session ID` error as a hostile input), so round 11 fuses instead, `--resume=<id>`. The pattern call is recorded per gap and never defaulted: bounded mechanism fixes for gaps[1] and gaps[2] because each is terminal (a two-sided boundary test; a verdict with no window left to over-join), and a narrower claim plus explicit disclosure for gaps[3] because a needle census's coverage shrinks as the conversion succeeds and the mechanism that could carry its name already exists. Next action: `/gsd-execute-phase 21 --gaps-only`.

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

- [x] 21-26-PLAN.md — The two mechanism defects that CANNOT fall out of a carrier, said plainly, plus the record. **CR-05** is closed by making an unadjudicated screen FAIL TO BUILD: `Screen` gains a sealed supertrait `RenderAdjudicated` with two object-safe methods, implemented only through an `adjudicate_screen!` macro, so `impl Screen for X` on an unadjudicated `X` is `error[E0277]` — verified by the planner in a scratch crate, along with `Box<dyn Screen>` still working (methods, never associated consts, because an associated const would make `Screen` dyn-incompatible and break every screen transition). **Both of the round-8 reviewer's plants are covered by that one bound**, including the macro-generated implementor, because a trait bound has no spelling to be short of. The disposition is PROMOTED onto the screen and read off the instance by the probe, so the checked and the declared disposition cannot drift; `SCREEN_IDENTITY_DISPOSITIONS` is demoted to a name-to-fixture map with its narrowed job documented in the same commit. The source walk is kept and its floor raised anyway — joined logical lines, an unnameable `impl` reported as an offence rather than skipped, and `census_offences` keyed on `(name, path)` (**IN-01**) — each observed red against a planted defect. **WR-05**: the alphabet needle is normalized so arm order and spacing stop mattering, red-first against a planted reordered copy, with the `default_branch_of` exclusion asserted since over-matching is the new failure direction. Records in `deferred-items.md`: the six carrier types round 9 did NOT retype with the measurement and failure direction of each; the per-widget ratatui obligation; **IN-04**'s process lesson (a prohibition against inheriting numbers that carries one named exemption will be falsified at the exemption); **IN-02**/**IN-03** deferred with reasons; and criterion 4 re-surfaced verbatim with no work claimed against it. **Corrects a count by measuring: the tree holds ELEVEN `Screen` implementors, not thirteen — the thirteen was eleven plus the reviewer's own two plants** (SAFE-07, SAFE-08, DRIVE-04)

Gap closure, round 10 (pass 10 scored 4/5 ROADMAP criteria, 30/33 must-haves, `gaps_found`. Criterion 4 stays **permanently agent-unclosable by construction** and tracked in `deferred-items.md` by explicit user decision, so **no plan here chases a criterion**; criterion 5's scoping is settled — CR-01 is a human-triggered TUI action, not the model's chosen action — and **no plan re-litigates it**. Round 10 closes the three Criticals pass 10 confirmed by reading the code itself, and folds in nine of the ten WR/IN items, with the tenth recorded as already closed. **The shape of the round is set by what round 9's mechanism structurally could not see:** a value that never entered a carrier, so the compiler named nothing for it, on a surface the probe's own fixture never rendered. Round 10's rule: **where a value can be typed, type it; where the fence forbids typing it, close it with a call AND a probe fixture AND say which is which.** And the second rule, which is why CR-02 and CR-03 were findings rather than known limitations: **a residual disclosed selectively has not been disclosed** — every carrier not retyped and every WR/IN item not folded in carries a written reason and a failure direction):

**Wave 1** *(three plans, verified-empty pairwise `files_modified` intersection, run in parallel worktrees)*

- [x] 21-27-PLAN.md — TRACER: **CR-01**, closed by DELETING the interpreter rather than learning to quote for it. The Sessions-tab resume hands `sh -c` a program string built by interpolating a session id scraped verbatim from another process's `/proc/<pid>/cmdline` and a cwd from `read_link("/proc/<pid>/cwd")` — a single quote in either is arbitrary code execution with the operator's privileges. It becomes an argv vector plus `Command::current_dir`, proven at the argv by a pure `resume_terminal_argv` whose control is observed RED against the pre-fix construction over every metacharacter class. **Round 9's own diff touched these lines and added a comment calling the value "A SUBPROCESS ARGUMENT"** — it is a shell-command fragment, and that misclassification is corrected with the falsified sentence quoted. The vocabulary gains the third question it lacked (`as_raw_for_logic_only`'s doc collapsed an argv element and a program fragment into one phrase, which is exactly why compiler-named-sites could not see this), and the rule — argv, never a program string — is CHECKED by a census over `src/` observed red by planting. Handles the defect the fix would otherwise introduce: the emulator/program separator is not uniform across `find_terminal`'s candidates, so a table and a test keep the security fix from silently deleting the feature. **WR-01**: the carrier's certificate stops being three-sixths of its claim — `Deref<Target = str>`, `Borrow<str>` and `Serialize` absences asserted with control arms, each observed red by planting the impl (SAFE-08, SAFE-07)
- [x] 21-28-PLAN.md — TRACER: **CR-02**, the pane that shows the LLM's own words was the one pane that was not escaped. `shown_capped` was introduced in round 9 for exactly this defect and applied to six sites; three on the same path were left behind, the largest being the live driver output pane, carrying the `U+E0000..U+E007F` tag block this phase itself names as the ASCII-smuggling carrier — and it SURVIVES a `Paragraph` by this tree's own per-widget measurement. **It was invisible because `probe_ctx` populates `driver_runs` and leaves `driver_output`/`driver_journal`/`driver_inbox` empty, so nine rounds of green were looking at `no_runs_lines` — the one line on the path that was already right.** `DriverOutputLine::text` becomes `crate::text::Untrusted`, so `output_line`'s `Span::styled` stops compiling, with the composition pinned EQUAL to `shown_capped` in both directions and the ring's drop accounting proven unmoved. The injection rows and dry-run preview are closed by a CALL because their carriers reach files outside the wave fence, and **that difference is disclosed with its direction and its promote trigger, not smoothed**. `sanitize_render_line`'s doc asserted these very files compose both classes — false for four sites, corrected in the commit that makes it true. **WR-06** (the opt-in disclosure an operator reads before an irreversible act, probed only with authored `&'static str` defaults so no control could ever go red) and **IN-01** (a completeness claim replaced by the census that checks it) (SAFE-07, DRIVE-01)
- [x] 21-29-PLAN.md — TRACER: **WR-05**, `render_for_terminal`'s ONE-composition claim made TRUE OF THE TREE rather than softened. Nine files under `src/ui/` still call `display_identity` alone on the same `ProjectState` fields and registry keys the CLI half was moved for. **What this plan is careful NOT to claim:** ratatui 0.30 filters `char::is_control` graphemes before a cell exists, so this is a false CLAIM plus a residual resting on dependency behaviour this tree asserts nowhere and that does not travel to non-TUI sinks — stated with its direction, not inflated into a live leak. The claim becomes a census in `src/ui/mod.rs` with exactly two exemptions named with their reasons, a **non-ban control arm** so the next author cannot satisfy it with a rename, a self-match control and a stale-exemption report. **WR-07**: the backlog sort's non-transitive comparator becomes `f64::total_cmp` over a finite-filtered key — tested on ORDER, because **pass 10 built a standalone program and measured the reviewer's panic claim FALSE on this toolchain**, and that refutation goes in the record rather than being inherited. **WR-08**: a control this phase depends on goes red for a CORRECT implementation the moment a fixture carries two invisible characters; the assertion becomes per character and the two-character fixture that proves it joins the shared list, making every consumer a re-run (SAFE-07, DRIVE-03)

**Wave 2** *(blocked on Wave 1 completion; `detail.rs` is 21-27's, `screens/mod.rs` and `render_escape_guard.rs` are 21-28's, and the record is assembled from the wave-1 SUMMARYs' own measurements)*

- [x] 21-30-PLAN.md — **CR-03**, which is not an unescaped render but an escape LAUNDERED by a round trip: `entry.value` is escaped at the list render and the same value is copied raw into `defaults_text_buffer` and drawn raw in the edit popup — the surface where the operator decides what to write back to disk. Escaping the second render would fix this field and leave the mechanism for the next, so the buffer gets a type: `EditBuffer` over `crate::text::Untrusted`, with character-wise push/pop and **exactly one deliberately-named raw take**. The thing this fix could make worse is closed in the same commit — a display escape reaching the persisted value would silently rewrite the operator's `.planning/config.json` with a rendering of itself, so a round-trip byte-identity assertion pins what is typed to what is stored. **WR-02**: the sealed doc's vocabulary claim is made TRUE by a crate-private two-variant `RenderDisposition` enum — a third value stops being expressible — designed so the two constants keep their names and **all eleven `adjudicate_screen!` invocations stay byte-identical**; the half that stays false (an in-crate hand-write can skip the macro) is narrowed to convention and given a census. **WR-03**: `adjudication_reason` finally gets readers. **IN-02** without a dependency. And the record, which is the half this phase keeps getting wrong: the CR-02 disclosure round 9 omitted while listing five others, framed as an **under-disclosure rather than a discovery**; a corrected carrier table naming the two carriers round 10 deliberately did not retype with their promote triggers; a **complete ten-row WR/IN triage with zero silent drops**, including WR-04 recorded as ALREADY CLOSED by `c9345a1` with evidence; WR-07's refutation; both DRIVE-04 backstops re-run; criterion 4 re-surfaced verbatim; and `COVERAGE.md` (SAFE-07, SAFE-08, DRIVE-01, DRIVE-03, DRIVE-04)

Gap closure, round 11 (pass 11 scored 4/5 ROADMAP criteria, 40/45 must-haves, `gaps_found`. Criterion 4 stays **permanently agent-unclosable by construction** and tracked in `deferred-items.md` by explicit user decision, so **no plan here chases a criterion and 4/5 is the expected, correct ceiling**. All four gaps are one species — *a completeness claim wider than the control that certifies it* — and three of the four sit in mechanisms round 10 built to end that species, which is why the round's default answer is **narrow the claim and disclose the remainder**, with a mechanism change admissible only where it is bounded and terminal. **The pattern call is made and recorded per gap, never defaulted:** gaps[1] and gaps[2] get bounded mechanism FIXES — gaps[1] because a misplaced increment is the mechanism failing its own documented constant and a two-sided boundary test at N and N+1 IS its certificate; gaps[2] because over-joining is a *laundering* defect and the fix DELETES the window from the decision rather than widening it — while gaps[3] gets a **narrower claim plus disclosure**, because a needle census's coverage SHRINKS as the conversion succeeds, so the structurally different mechanism that could carry its name already exists in this tree and a fourth census would move the over-claim rather than end it. **The planner MEASURED the receiving parser and refuted the verification pass's own recommended fix**: `claude --resume -- <valid-uuid>` returns the same `requires a valid session ID` error as a hostile input, so the recommended `--` separator closes the injection and *deletes the resume*; `--resume=<id>` binds the value regardless of its first byte and keeps it):

**Wave 1** *(three plans, pairwise `files_modified` intersection COMPUTED empty — union 8 distinct files, none owned twice — run in parallel worktrees)*

- [x] 21-31-PLAN.md — **gaps[0], the BLOCKER and the only live leak in the phase.** Round 10 traded CWE-78 for **CWE-88**: `resume_terminal_argv` emits the `/proc`-scraped session id as the trailing element after `-r, --resume [value]`, an OPTIONAL-value option, so an id beginning with a hyphen is read by `claude` as a new option — reproduced at the binary, `claude --resume --version` prints `2.1.248 (Claude Code)`. **And the committed control is complicit:** not one of `hostile_session_ids()`'s eighteen fixtures begins with a hyphen, so it passes today and would pass unchanged against a build shipping the defect. The fix is **FUSION**, `--resume=<id>` as one element, not the `--` separator the pass recommended — measured, because `--` makes the id a positional operand and the resume silently stops working, the exact feature-deletion-in-a-security-fix's-clothes shape `terminal_program_separator`'s own doc exists to prevent. The corpus half is proven by a **three-way measurement no single run can fake**: the pre-existing control captured still GREEN with the widened corpus against unfixed code (the complicity, shown not asserted), the new content-independence property captured RED with the new corpus against unfixed code, and the same property GREEN with the OLD corpus — so the widening is measured load-bearing. The structural assertion is a SHAPE property over the whole vector (the count of `-`-leading elements is constant for every id), never a list of forbidden first bytes, so it cannot be one fixture short. `launch_terminal_argv` is examined and PINNED unchanged by an equality; `read_session_id` deliberately keeps its pass-through and the reason is measured — the CLI resumes by session **title** too, so a rule tight enough to refuse `-h` deletes legitimate sessions silently; the three sibling `--option value` pairs in `src/executor/claude.rs` are measured inert (all three fields `None` at the only default, set by no caller) and deferred with a promote condition; and the doc clause claiming *"there is no interpreter left in the path to parse anything"* is corrected with its own text quoted (SAFE-08, SAFE-07)
- [x] 21-32-PLAN.md — **gaps[1] and gaps[2], both bounded mechanism fixes with their claims narrowed in the same commit.** gaps[1]: `taken += 1` executes before the comment check, so a skipped comment spends join budget — the planner independently reproduced pass 11's measurement (REPORTED at N = 0, 4, 10, 11; **MISSED at N = 12, 13, 20, 40, 100**) and the fix reports at every N up to 100 while the **executable-line window is measured unchanged at 11 reported / 12 missed**, which is the two-sided boundary that certifies it and resolves the SAFE-07 boundary probe row EXPLICITLY rather than by backstop. The claim is narrowed in the same commit: the word *any* comes out, the three real bounds go in, the method-chain residual is finally written where `has_an_unclosed_delimiter`'s doc has been promising it, and **IN-05** is closed by narrowing the doc rather than widening a substring detector, because widening adds false positives to a control whose whole value is that its zero is trustworthy. gaps[2]: the composition verdict moves off the joined text and onto the **innermost call** — each occurrence judged by the characters immediately preceding it — so there is no window left to over-join; measured green on all five real production sites and RED on the verifier's two-arm `match` fixture the committed rule calls clean. Its two secondary defects close in the same commit: NON-VACUITY 2 recomputed over the slice the census actually scans (measured 3 vs 2 in `driver.rs`), and the truncation moved from the first `#[cfg(test)]` LINE to a column-zero test MODULE with exactly one asserted per file — **the silent-kill-switch hazard is live in this tree at `detail.rs:5342`**. The residual's direction flips to over-detection, which is LOUD (SAFE-07, SAFE-08)
- [x] 21-33-PLAN.md — **gaps[3], narrowed and disclosed rather than re-mechanised, plus the three carried items delivered.** `every_render_site_under_ui_composes_both_classes` claims every render site and is driven by **7 executable needle occurrences in 3 of 16 files** (`driver_confirm.rs` 4, `driver.rs` 2, the exempt probe 1 — so 6 non-exempt in 2 files), independently re-measured. The decisive property is that its **coverage shrinks as the conversion succeeds**: a site escaping nothing carries no needle, and every conversion removes one. So it is renamed to what it checks, its reach is disclosed file by file, the shrinking property is stated with its direction, and the completeness claim is **handed by name** to the two mechanisms that structurally carry it — the sealed `RenderAdjudicated` supertrait, which makes an unadjudicated screen a compile error, and the per-screen behavioural probes. The one bounded mechanism addition is a **reach PIN**: the disclosed distribution as a two-direction equality, so the shrink the disclosure describes can go red; its failure is a bookkeeping red with a one-line repair and that direction is stated. Pass 11's hand-trace of 63 sites outside the needle is recorded with attribution — this is a false completeness claim, **not a live leak**. **WR-04**: `EditBuffer`'s prose-only trait-absence claim gets the certificate `Untrusted` got in the same round, built LOCALLY because `src/text.rs`'s probe module is private and fenced to 21-32, each absence observed red by planting its impl, and the doc's `buffer.clone()` example — which names a method the type does not have — replaced by a measured compiler error. **WR-05**: one appended tiebreak makes the backlog order total over ELEMENTS, not just keys. **21-29's S1 delivered whole** — a two-direction spot-check on an input-echo screen through the real `Screen::render`, arrival before property, widget family named because `Paragraph` drops zero-width graphemes and `ListItem` does not — and **F3 CLOSED AS UNNECESSARY by measurement**: `render_escape_guard.rs` is a child of `ui::screens` and already calls `super::tests::ctx_with_aliases` at `:982` and `:1191`, so the visibility widening 21-30 deferred is not needed and nothing is loosened (SAFE-07, DRIVE-03)

**Wave 2** *(blocked on Wave 1; ordering dependency, not a file conflict — `21-34 ∩ wave 1 = ∅` — because every residual must be quoted from what the three plans delivered and every gate figure measured on the MERGED tree)*

- [x] 21-34-PLAN.md — The record, corrected completely or not at all. `deferred-items.md` still opens with *"the same binary with `-- --test-threads=1` returned ok three times out of three"* — a mitigation round 10 proved does not work, standing FIRST above three later corrections, so a reader who stops there acts on the wrong one. The correction goes **at** the stale sentence, and its completeness is established by a **committed grep inventory** in which every `test-threads` / `driver_reattach` hit across `.planning/`, `tests/` and `src/` is classified as a STANDING claim (corrected in place) or a DATED observation of one run (left unedited, named with the reason) — because correcting only some locations recreates this round's own species inside the record itself. It also goes where the reader of the failing test looks: `tests/driver_reattach.rs` carries no note at all today, and gains a **comment-only** one, asserted comment-only so the next verifier's "no file under `tests/` in the diff" shortcut is replaced by something narrower and checkable. Plus: round 11's four closures with their residuals and DIRECTIONS; a new **`claude` CLI staleness obligation** in the shape of the ratatui one, carrying the six probe commands and the measured version, because gaps[0]'s fix rests on a dependency behaviour nothing in this tree goes red for; three deliberate non-closures with promote conditions (the executor argv pairs, `IN-01`..`IN-04`, `WR-06`); a **Record-corrections table** for the four round-10 truths pass 11 measured FALSE as shipped (`21-27` t1, `21-27` t5, `21-28` t7, `21-29` t2) naming what was claimed, what was measured and where each is now closed; both DRIVE-04 backstops re-run as explicit evidence; `COVERAGE.md` amended append-only with its existing declaration byte-identical; and **criterion 4 re-surfaced verbatim for the fourth consecutive round with the sentence, in words, that it is permanently agent-unclosable and that 4/5 is the correct ceiling — so the next verifier does not re-litigate it** (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 12 (verification pass 12 scored **90/91 must-haves, 4/5 ROADMAP criteria** and recorded `gaps_remaining: []`. **The verifier was wrong.** The independent code review found one real defect and the orchestrator re-confirmed it in the source: round 11's fusion at the producer changed the wire format the consumer parses, and nothing coupled the two. `resume_terminal_argv` emits ONE fused element (`detail.rs:713`, prefix at `:585`) while `read_session_id` still scans `args.windows(2)` for a standalone `--resume` (`session_detector.rs:159-169`), so **a session the TUI itself resumes can no longer be read back out of `/proc/<pid>/cmdline`** — it becomes undetectable in the Sessions tab and un-resumable, **silently**, with every test in the repository green. The fix goes on the **consumer only**: round 11 *measured* that the producer-side alternative deletes the resume outright (probe C == probe D at `claude` 2.1.248), so fusion stays and the parser learns both wire forms — the fused one this TUI emits and the split one a human typing `claude --resume <id>` by hand still produces. **This is a deliberately SMALL round: one gap, one plan, one wave, no new mechanism and no new census.** Its substance is the control and the lesson, not the four-line parser change):

**Wave 1**

- [x] 21-35-PLAN.md — **The one gap, and the two-sided control that would have caught it.** The parse is split out of the `/proc` I/O as `pub(crate) fn session_id_in_cmdline(cmdline: &[u8]) -> Option<Untrusted>` — the wrap moving to the innermost point so no unwrapped `String` escapes — and taught **both** wire forms with a leftmost-wins index scan; the non-validating pass-through, the non-empty-after-`trim()` check and the keep-scanning behaviour all survive, because the CLI resumes by session **title** too and a rule tight enough to refuse `-h` would delete legitimate sessions silently. **The control spans the pair:** what `resume_terminal_argv` emits is NUL-joined into the `/proc` encoding and driven through the real parser over the shared 28-fixture `hostile_session_ids()` corpus, in both forms, with the split arm constructed explicitly so "both are legitimate" is asserted rather than asserted-about — and it is **observed RED against a parser first proven behaviour-identical to HEAD** (green at the unchanged 1110), so the red is attributable to the missing wire form and not to the move. **And the lesson is corrected where it misled:** `session_detector.rs`'s `# No control is added in this file, deliberately` argued the property that matters is a property of the SINK — right about *security*, and precisely what hid the *functional* coupling, since every control lived on one side and none spanned both. The correction goes **at** the sentence, quotes the falsified reasoning verbatim, names what it got right and what it missed, and **leaves the paragraph standing**. Plus the durable phase-21 `ui.safety-gate` rationale (UI *files* change, no *visual-design* change; round 11 overrode, **round 12's deterministic gate returned `block: false` on its own**) with a measured falsification condition, the seven unresolved edge-probe rows surfaced as flagged assumptions with no silent drops, and criterion 4 re-surfaced verbatim for the fifth consecutive round with no work claimed against it — **4/5 remains the correct ceiling** (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08)

Gap closure, round 13 (verification pass 13 scored **98/100 must-haves, 4/5 ROADMAP criteria**, `gaps_found`, with three new gaps — and one root cause that is the actual subject of this round. **This is the third consecutive round in which a fix was certified by a corpus structurally incapable of failing on the very class at issue:** round 10 shipped a hostile-session-id corpus with no leading-hyphen fixture, so it passed against a build vulnerable to option injection; round 12 shipped the SAME corpus with no whitespace fixture, so it passed against a parser whose `trim()` falsifies the round's headline truth; and G3 is the same shape at the TYPE level, since a `&str`-typed harness cannot even express non-UTF-8 input. Adding a whitespace fixture and a `-r` fixture would fix these two instances and leave the mechanism fully intact — round 14 would find the next unenumerated class. **So the round changes the SHAPE of the certificate, on all three axes at once:** the VALUE axis (the claim becomes a total property with exactly two named refusal classes, and the input space becomes a fixed-seed mixture generator over `Vec<u8>` with a committed non-vacuity floor asserting six class minima, ≥200 distinct byte values, and that BOTH branches of the disjunction were exercised); the ENCODING axis (the harness is retyped to `&[&[u8]]`, so the class G3 lives in is expressible rather than unrepresentable — note `session_id_in_cmdline` has taken `&[u8]` all along and it was the harness that was narrower); and the PRODUCER axis (a source-derived, adjudicated census over `src/`, so "every `claude` argv producer in this build" is a MEASURED set rather than a remembered one). **The `proptest`-vs-hand-rolled call was made with the graph cost measured, not asserted** — `cargo add --dev proptest` locks 14 new packages and takes `Cargo.lock` from 354 to 368 entries — and DECLINED, because the project's own `icu_properties` precedent turns on a crate supplying Unicode's REFERENCE DATA, whereas a property engine supplies a SEARCH STRATEGY and no external source defines the set of session-id classes; `proptest`'s default strategies would not have found G1 either. The honest limit is written into the source beside the generator: a generator does not abolish enumeration, it moves it from values to a grammar and a distribution, and the committed floor is what stops that grammar degrading into a shorter list. **The mechanism-vs-narrower-claim call is recorded per gap and never defaulted**, and criterion 4 stays permanently agent-unclosable — **no plan here chases a criterion and 4/5 is the correct ceiling**):

**Wave 1**

- [x] 21-36-PLAN.md — **G1 (the VALUE axis) and G3 (the ENCODING axis), closed by changing what the certificate IS.** The round-12 claim is replaced by a property with no third outcome: for every generated byte string and every registered wire form, the parse is byte-identical OR it refuses, and `s` is in exactly one of two named classes — **R1** not valid UTF-8, **R2** empty after `trim`. Both directions are asserted, and the oracle is `std::str::from_utf8`/`str::trim` spelled independently in the test rather than a helper exported from the parser, because *an oracle that consumes the predicate it checks can only ever agree with it* — this repository's own recorded lesson. The input space becomes `generated_session_id_bytes(GENERATOR_SEED, 4096)`: a uniform-random-bytes arm that reaches classes nobody named, plus a combinator grammar (padding × prefix × core) whose `core` may be one of the 28 fixtures, which are RETAINED as a seed corpus and demoted from being the certificate. `nul_join_cmdline` is retyped to `&[&[u8]]`. Then the parser changes, narrowly and in one branch: the emptiness test moves to a trimmed COPY so the returned value is the wire bytes untrimmed (`--resume=" abc "` reads back `" abc "`), and ill-formed UTF-8 is REFUSED with the scan continuing rather than substituted into an id no process carries — the measured `--resume=\xff\xfe` → `Some("\u{fffd}\u{fffd}")` is a fabrication, and refusing it costs nothing because both of this build's producers take `String`. Three verbatim REDs are captured against the unchanged parser before it is touched, and a fourth against the FLOOR itself by suppressing a generated shape. Plus the dated corrections beside the two sentences this round falsifies, the `proptest` decision with its measured figure recorded in the source, and the IN-07 precedence reading pinned — `["claude","--resume","--resume=abc"]` returns `Some("--resume=abc")`, the code is right and the doc is what changes (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08)

**Wave 2** *(blocked on Wave 1 — the `files_modified` intersection with 21-36 is `{src/session_detector.rs, src/ui/screens/detail.rs}`, cardinality 2, computed pairwise and NOT empty)*

- [x] 21-37-PLAN.md — **G2 (the PRODUCER axis), plus WR-05 and the record.** `every_claude_argv_option_site_under_src_is_adjudicated` walks `src/` with an anti-self-match needle assembly (the `INTERPRETER_STEMS` idiom, split on the last character) and requires every non-comment quoted option-literal site to appear in `CLAUDE_ARGV_SITES` with a disposition — `Producer`, `Consumer`, `NotClaude` or `Excluded` with a reason AND a direction — plus three anti-rubber-stamp guards: ≥1 `Producer`, both known producers `Producer`, every non-`Producer` reason non-empty. The pre-wave-1 inventory is 18 non-comment sites across 4 files, including `git_ops.rs`'s `git diff-tree -r` as a false positive that proves the needle does real work. The census is observed RED naming `src/executor/claude.rs` as a `Producer` with no round trip, and only then is the gap closed: four spellings measured at `claude` 2.1.250 — bare `-r`, attached `-r<value>` (safe because the complete short-option inventory is eight and `-r` is the only `r`-initial one; `-r=abc` yields `=abc`, which is what the receiving parser binds), bare `--session-id` and fused `--session-id=` — under a rank rule derived from `--fork-session`'s own help text (*"When resuming, create a new session ID instead of reusing the original"*), so `--resume` outranks `--session-id` regardless of index while leftmost-within-rank is preserved. The premise is ENFORCED rather than remembered: `no_source_line_under_src_requests_a_forked_session`. Then the real `build_argv` output is byte-encoded through `OsStrExt::as_bytes` — never `to_string_lossy`, the exact substitution the previous plan removed — and driven through the real parser in both the resuming and non-resuming shapes. **WR-05's duplicate control is deleted only after its removability is OBSERVED**: the fused branch is planted broken, BOTH survivors go red, the output is captured, the defect restored — because a deletion argued rather than measured is this phase's failure with the sign flipped. Plus the durable record: round 13's closure with residuals and directions, the corpus-SHAPE record naming all three rounds, the pass-13 `WR-04` focus-stealing defect (`terminal_switch.rs:57`'s unanchored `contains` matching `/dev/pts/31` for `pts/3`) recorded in the BACKLOG with its reproduction and explicitly NOT folded in, the five info rows dispositioned, the three unclassified edge-probe rows kept flagged with the no-silent-drop equality stated (7 == 4 + 3), and criterion 4 re-surfaced verbatim for the sixth consecutive round with zero work claimed against it (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08)

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
| 19. GITSAFE — Git & Blast-Radius Envelope | 22/23 | In Progress|  |
| 20. Deterministic Decision Router & Run Bounds | 5/5 | In Progress|  |
| 21. LLM Goal Layer & Prompt-Injection Hardening | 37/37 | In Progress|  |
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

### Phase 999.4: Coverage sub-stage may need frontmatter status, not a presence bit (BACKLOG)

**Goal:** Decide whether the `COVERAGE.md` sub-stage should be read as a *state* parsed
from frontmatter rather than as a *presence bit*, and change the Pipeline drill-down
rendering if so.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-08-28. **Not for the current milestone** — this is an open question to be
decided later, not a defect with a known fix. Nothing is known to be broken today.

**What exists now.** The meta-manager detects GSD's `COVERAGE.md` sub-phase artifact and
renders it in the TUI:

- *Detection* — `src/state_reader/disk_status.rs:575` matches bare `COVERAGE.md` and
  prefixed `*-COVERAGE.md`, setting `has_coverage`. It is classified with the GSD 1.8.0
  informational artifacts (`WINDOWS.md`, `deferred-items.md`, `SKELETON.md`): flagged
  only, never counted, so it cannot skew `plan_count` / `summary_count` or phase status.

- *Rendering* — `src/ui/screens/detail.rs:4364` renders it as the last row of the
  **Execute sub-stages** drill-down on the Pipeline tab. `push_substage`
  (`detail.rs:4370-4386`) renders presence as `✓ done` and absence as `○ not run`. It is
  also part of the `exec_touched` predicate (`detail.rs:4351`), so a phase whose only
  execute-side artifact is a `COVERAGE.md` still opens the section.

**The open question.** A report from another GSD project states that its api-coverage
gate's *"detector runs only when COVERAGE.md is absent."* If that generalises, then the
artifact's **presence** means "the gate is suppressed / will not re-run" — not "coverage
was checked and passed". Our `✓ done` / `○ not run` labels would then be describing the
artifact's existence while *reading* to the user as a statement about the coverage
outcome. For a write-once artifact the two coincide and the current rendering is correct;
they diverge only if a `COVERAGE.md` can exist while recording a failing or partial
result.

**Precedent if a richer read is wanted.** `*-UAT.md` already goes beyond a presence bit —
`disk_status.rs:123` and `:212` parse `status` out of its frontmatter. That is the
established pattern in this codebase for promoting a sub-stage report from an existence
check to a state.

**First thing to check before changing anything:** whether GSD's `COVERAGE.md` actually
carries a frontmatter `status` key, and what values it takes.

Partial evidence, one local sample (2026-08-28): phase 21's own `21-30` wrote a
`COVERAGE.md` into this repo's `.planning/` tree, at
`.planning/phases/21-llm-goal-layer-prompt-injection-hardening/COVERAGE.md`. That file has
**no YAML frontmatter at all** — it opens with an `# …` heading and carries its verdict as
a bold prose line, `**Status: NO EXTERNAL API INTEGRATION.**`, followed by a reasoned
declaration. So on this sample there is no `status` key to parse, and a frontmatter-based
read would find nothing. Caveat: this is a single sample and it is a *declaration*-style
COVERAGE.md (an opt-out in place of a coverage matrix), which may not be representative of
a COVERAGE.md that records an actual coverage result. Confirm against GSD's own template
and against a matrix-style sample before deciding.

Possible outcomes, none chosen: (a) leave the presence bit as-is and reword the labels so
they describe the artifact rather than the outcome; (b) parse a frontmatter `status` the
way `*-UAT.md` does, if one exists; (c) parse the prose verdict line, if that is what GSD
actually emits.

Note: Backlog 999.1 (Milestone Archive Browser) promoted to Phase 12 in v1.2.
