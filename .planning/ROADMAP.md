# Roadmap: GSD Meta Manager

## Milestones

- ✅ **v1.0 MVP** - Phases 01-04 (shipped 2026-03-26)
- ✅ **v1.1 Polish & Power Features** - Phases 05-09 (shipped 2026-03-27)
- ✅ **v1.2 Housekeeping & Archive Browser** - Phases 10-13 (shipped 2026-04-01)
- ✅ **v1.3 Configuration & Pipeline Visibility** - 10 quick tasks (shipped 2026-05-09)
- ✅ **v1.4 Live Sessions & Document Browsing** - 4 quick tasks (shipped 2026-05-12)
- ✅ **v1.5.0 Sub-phase Artifact Detection** - 2 quick tasks (shipped 2026-05-15)
- ✅ **v1.6.0 GSD 1.8.0 Catch-up** - 1 quick task + 2 fast tasks (shipped 2026-07-22)
- ✅ **v2.0 Autonomous Orchestration** - Phases 14-31 (shipped 2026-10-29 as v2.0.0; Phases 19, 21, 22, 23 closed by supersession; Phase 32 deferred to backlog 999.8)
- 🚧 **v2.3.0 Multiplexer Backends** - Phase 33 (in progress; number 32 skipped, it names the deferred v2.0 phase now backlog 999.8, started 2026-11-10)

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

<details>
<summary>✅ v2.0 Autonomous Orchestration (Phases 14-31) - SHIPPED 2026-10-29 (v2.0.0)</summary>

Full phase details: `.planning/milestones/v2.0-ROADMAP.md`. Phase artifacts: `.planning/milestones/v2.0-phases/`.
Requirements: `.planning/milestones/v2.0-REQUIREMENTS.md`. Audit: `.planning/milestones/v2.0-MILESTONE-AUDIT.md`.

- [x] **Phase 14: UI Fixes** - Four display defects that misreport project state (completed 2026-07-29)
- [x] **Phase 15: Transport Foundation** - Duplex `stream-json` executor with envelope-derived outcomes (completed 2026-07-29; retired in part by Phase 31)
- [x] **Phase 16: Run Journal & State Substrate** - Durable, redacted, cheap-to-read run record (completed 2026-07-29)
- [x] **Phase 17: Supervisor** - Detach, kill switch, dry-run, single-run lock, opt-in gate (completed 2026-07-29; retired in part by Phase 31)
- [x] **Phase 18: Driver Tab, Live Watch & Durable Injection** - Manual ship point: run and steer from the TUI (completed 2026-07-29; retired in part by Phase 31)
- [x] ~~**Phase 19: GITSAFE — Git & Blast-Radius Envelope**~~ - SUPERSEDED 2026-10-14 by spikes 001/002 (sandbox without push credentials, plus a gsd-mm push step outside it). Gap loop closed by supersession, not resumed; Phase 31 deleted the code (closed 2026-10-28)
- [x] **Phase 20: Deterministic Decision Router & Run Bounds** - Rules pick the next command; runs stop themselves (verified 5/5 on 2026-08-19; kept as the advisory `next --json` hint) [inferred: completion marked at re-plan]
- [x] ~~**Phase 21: LLM Goal Layer & Prompt-Injection Hardening**~~ - SUPERSEDED 2026-10-14. The master driver is the goal layer, DIRECTION.md is the approved plan, and injection is contained by the sandbox. Paused at 4/5 (hard ceiling); gap loop closed by supersession, not resumed; Phase 31 deleted the code (closed 2026-10-28)
- [x] ~~**Phase 22: Container Execution Target**~~ - SUPERSEDED 2026-10-14 by bwrap + `systemd-run --user --scope` (Phase 28). 0/4 plans executed, no code landed (closed 2026-10-28)
- [x] ~~**Phase 23: Gate Policy & Auto-Validation**~~ - SUPERSEDED 2026-10-14 by per-item done-when plus standing rules in DIRECTION.md (Phase 29). Never planned (closed 2026-10-28)
- [x] **Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation** - Master/detail roadmap with a selection cursor; eight detail tabs instead of ten (completed 2026-09-23)
- [x] **Phase 25: Running Agents & Live Wave View** - Per-project view of running GSD agents and executor wave progress, read from worktrees and Claude subagent metadata (completed 2026-09-25)
- [x] **Phase 26: Driver Validation Spike 003** - Prove gap rounds 1-2, divergence parking and no-respawn on a real GSD phase before any Rust (completed 2026-10-16; superseded by 26.1, closed 2026-10-28 without re-verification)
- [x] **Phase 26.1: Driver Validation Spike 003 Re-run** (INSERTED) - Fix gap-round counting, headless-driver liveness and T-26-11, then re-run Opus to prove VALID-01/02 (completed 2026-10-18)
- [x] **Phase 27: driverctl & Driver Frontmatter** - Rust single writer of run state; exchange-splice frontmatter on DIRECTION.md (completed 2026-10-19)
- [x] **Phase 28: Sandboxed Driver Launcher** - bwrap + cgroup scope, outside lock and hard caps, per-run config and auth, push outside the sandbox, macOS degraded mode (completed 2026-10-24)
- [x] **Phase 28.1: Sandbox Egress & Agent-Channel Isolation** (INSERTED) - `--unshare-net` + allowlisting egress proxy, `$HOME` allowlist, planted-file guard, agent-tool denies, nested-agent cap check, degraded-mode gap reporting (completed 2026-10-28)
- [x] **Phase 29: Master Prompt & Run Templates** - Shipped, versioned master prompt; DIRECTION/LOG templates; explicit stop-rule precedence; reconciliation (completed 2026-10-20)
- [x] **Phase 30: Driver Tab on DIRECTION.md** - TUI reads the frontmatter (status, caps, heartbeat, crash) and steers by editing `control` (completed 2026-10-24)
- [x] **Phase 31: Retire Superseded Envelope & Goal-Layer Code** - Remove or rewire the Phase 19/21/22 code; close their threat ids as superseded (completed 2026-10-28)

</details>

### 🚧 v2.3.0 Multiplexer Backends (In Progress)

Requirements: `.planning/REQUIREMENTS.md` (MUX-01..MUX-07).

### Phase 33: Terminal multiplexer backend seam + herdr support

**Goal:** Replace the tmux-hardcoded `src/terminal_switch.rs` with a reusable multiplexer backend trait (detect, focus a Claude session, open a new session in a cwd with an argv), migrate tmux onto it, and add a herdr backend, so focus and launch work inside herdr (also nested in or around tmux). The trait is designed against zellij, WezTerm mux, kitty remote control, GNU screen, Rex (Superlogical) and herdr-core/`hc`; those implementations are deferred (MUX-F01..F06).
**Requirements**: MUX-01, MUX-02, MUX-03, MUX-04, MUX-05, MUX-06, MUX-07
**Depends on:** Nothing (first phase of v2.3.0)
**Plans:** 0 plans

**Success Criteria** (what must be TRUE):
1. Inside a herdr pane (`HERDR_ENV=1`, `$TMUX` unset), focusing a running Claude session lands in its herdr pane, matched on `agent_session.value` from `herdr pane list`; launch/resume opens a new herdr tab in the project cwd instead of a GUI terminal.
2. Inside tmux, focus and launch behave exactly as before, and the direct-exec refusals (<2-element argv, `-`-leading head, cwd as an argv element) are enforced on the shared sink for every backend, with the existing tests green.
3. With nested multiplexers the innermost one is chosen, and with none (or on backend failure) the GUI-terminal fallback chain runs with the reason surfaced. Both are covered by env-fixture unit tests that need no live multiplexer.
4. The trait's design note maps each surveyed tool (zellij, WezTerm, kitty, screen, Rex, herdr-core) onto its methods, and the five current call sites (`normal.rs:840`, `detail.rs:2407/4938/5448/5485`) all go through the seam.

**Open questions for research** [inferred]: does `herdr pane run <PANE_ID> <COMMAND>...` exec directly or type into a shell (MUX-05 refuses rather than regress the no-interpreter invariant); how herdr focus composes workspace → tab → pane; whether the herdr server under systemd needs `HERDR_SOCKET_PATH` passed explicitly.

Plans:
- [ ] TBD (run /gsd-plan-phase 33 to break down)

---

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

### Phase 999.5: Capability lifecycle: outdated, update, remove, install-by-URL (BACKLOG)

**Goal:** From the TUI, list outdated GSD capabilities and update, remove or install them
(install by URL) through `gsd-tools capability … --json`, showing GSD's consent disclosure and
requiring explicit confirmation before anything is run with `--yes`.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-10-19, deferred out of Phase 32 (now backlog 999.8; which covers only see / toggle / configure).
Builds on Phase 32's (now backlog 999.8) capability model. Global vs project scope, the per-scope
`.gsd-capabilities.json` ledgers and the `~/.gsd/consent.json` record all apply; a project install
cannot shadow a global one.

### Phase 999.6: Capability registry browsing (BACKLOG)

**Goal:** Browse the GSD capability registry (`docs/registries/capabilities.json`, fetched from
GitHub) from the TUI, as the discovery front end for capability install (Phase 999.5).
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-10-19, deferred out of Phase 32 (now backlog 999.8). **Trigger:** worth doing once the registry ships
with GSD or has more entries; until then a network fetch buys little.

### Phase 999.7: Master prompt 1.1 — reconcile-first refusal routing + IN-01 (BACKLOG)

**Goal:** Next master-prompt version (prompt bump from `driver-prompt/1.0.0`). (1) A "reconcile
first" refusal (D-17) reaches the driver as `LAUNCHER CAP: ...`, and stop rule 2 (launcher denial)
then makes it park the item instead of running reconcile and continuing. Only a driver that skips
its startup reconcile hits this. Make the refusal distinguishable in the prompt and route it to
"reconcile, then continue". (2) Bundle 29-REVIEW IN-01: the prompt uses `$RUN_DIR`/`$PROJECT_ROOT`
as env vars, but the launcher only substitutes `{RUN_DIR}`/`{PROJECT_ROOT}` textually.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-10-20 at the Phase 29 UAT sign-off. Both need a prompt version bump, so ship together.

Note: Backlog 999.1 (Milestone Archive Browser) promoted to Phase 12 in v1.2.


### Phase 999.8: GSD Capabilities: Visibility & Per-Project Control (BACKLOG)

> **Deferred from v2.0 Phase 32 by user decision (2026-10-28 cleanup).** Moved out of the milestone to the
> next milestone/backlog; nothing was planned (0 plans, no CONTEXT.md), so this is the full original section.
> Musk ledger: logged as a `milestone-phase` cut.

**Goal**: From the TUI, users can see which GSD capabilities exist for each project and across
all projects, and switch them on or off and configure them per project, without leaving the TUI.

**Prerequisite**: hard prerequisite is the separate quick bug-fix that makes
`config.json` writes atomic and `.planning/.lock`-aware and fixes the
`capabilities.strict_known_registries` type; Tier 2 must not start before it lands. TUI-only and
independent of the driver work (Phases 28-31), so parallel-safe like Phases 24 and 25
[inferred — the add-phase tool defaulted this to "Phase 31"].

**Requirements**: TBD (to be derived in discuss/plan; no REQUIREMENTS.md ids minted yet [inferred])

**Scope**:

- *Tier 1 — see.* Per project: capability id, version, kind (built-in / global / project),
  state (active / off in this project / incompatible / inactive with no consent) and the reason.
  Plus a dashboard matrix of capabilities × projects.
- *Tier 2 — toggle and configure.*
  - Flip the per-project switch, i.e. the manifest's `activationKey` (e.g. `musk.enabled`), in
    `.planning/config.json`.
  - Turn capability-contributed config keys (`ponytail.*`, `musk.*`, `mempalace.*`,
    `refactor.*`, …) from read-only pass-through rows into editable typed rows, driven
    generically by each manifest's declared config keys, defaults and descriptions, with no
    per-capability code.
  - Machine-wide enable/disable (`.gsd-surface.json` `disabledClusters`), if exposed at all, is
    labelled "affects all projects".

**Background (gsd-core 1.18.0)**:

- The 46 built-ins are frozen in `bin/lib/capability-registry.cjs` and switched by config keys.
- Third-party capabilities live in `$GSD_HOME/.gsd/capabilities/<id>/` (global) or
  `<repo>/.gsd/capabilities/` (project); each scope has a ledger `.gsd-capabilities.json`.
  Project scope needs a consent record in `~/.gsd/consent.json`.
- Precedence: first-party, then global, then project. A project install cannot shadow a global one.
- `gsd-tools capability list|state|set|enable|disable --json` exist; `set --gate` writes
  `config.json` under `.planning/.lock`.

**Decision to make in discuss/research (open)**:

- *Reads:* parse ledgers, manifests and config directly (fast across N projects; fits the
  "read from files" constraint), or call `gsd-tools capability list --json` (authoritative status
  semantics, but a Node startup per project)? A hybrid is possible.
- *Writes:* direct `config.json` edits through the app's own locked atomic write path, or
  `gsd-tools capability set` / `config-set`?
- The user has authorised running mutating GSD commands from the app.

**Out of scope (backlog)**: capability lifecycle — outdated/update/remove/install-by-URL
(Phase 999.5); registry browsing (Phase 999.6).

**Success Criteria** (draft, to be firmed up in discuss/plan [inferred]):

  1. For any registered project, a capability view lists every capability with id, version,
     kind, state and reason, and the dashboard shows a capabilities × projects matrix.
  2. Toggling a capability's `activationKey` from the TUI changes `.planning/config.json` safely
     (locked, atomic) and the new state is reflected without restarting.
  3. Capability-contributed config keys render as editable typed rows generated from the
     manifest, with no per-capability code.
  4. Any machine-wide control is visibly labelled "affects all projects".

**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.9: One-off dashboard running badge (BACKLOG)

**Goal:** The dashboard row shows that a sandboxed one-off agent is working on a project (a
count of live runs), not only the project's Driver tab (`t` / `T` and the one-line status).
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-11-03 from todo `2026-11-03-oneoff-dashboard-running-badge` (source: quick 261103-f5y
CONTEXT.md, deferred as I-13; CONTEXT asked for a running badge "if cheap"). The dashboard has no
per-project read of the one-off root today: it needs a lock probe of each listed run's
`<data>/gsd-meta-manager/oneoff/<hash8>/<run-id>/run.lock` (via `oneoff::store::live_seen`, which
falls back to the legacy per-project `oneoff.lock` for a run dir without one) for every registered
project on the dashboard tick. Any number of runs may be live per project since quick 261103-hx7,
so the badge can count them. **Approach:** fold the probe into the existing 20-tick launcher-marker
scan (`Action::LauncherMarkersRescanned`): `live_seen` over `read_runs` per project, off the render
thread, stored beside `launcher_marker`; badge in the Status cell only while one reads `HeldBy`.
Gate on `GSDMM_EXPERIMENTAL_FEATURES` like every driver surface; add render-escape and flag-off
tests. Files: `src/ui/screens/normal.rs`, `src/oneoff/store.rs`, `src/ui/screens/mod.rs`.

### Phase 999.10: One-off hard ceiling raise via session resume (BACKLOG)

**Goal:** A "raise the ceiling" action for a live one-off: stop the session, then relaunch the
agent with `claude --resume <session-id>` and a higher `--max-budget-usd` in the SAME clone and run
dir, so the work and the conversation carry on.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-11-03 from todo `2026-11-03-oneoff-raise-hard-ceiling-via-resume` (source: quick
261103-gt1 CONTEXT.md, C-14, C-19; out of scope for gt1 by decision). **Problem:** `--max-budget-usd`
is the agent's own hard ceiling, fixed at launch. The soft cap and spend meter only make it visible
and let the operator ask the agent to wrap up earlier; a run on track but near the ceiling can only
end as `over_budget` or be restarted from scratch in a new clone. **Needs:** the session id from the
stream's `system` init event (classified as `StreamEvent::System`; the classifier must keep the id,
runtime-side); decisions on the run record (one run, several sessions: `run.json` keeps the ceiling
history), the report contract (the agent has not reported yet) and the wall clock (keeps counting
from the first spawn). Shares the session-id plumbing with Phase 999.11. Files:
`src/oneoff/launch.rs`, `src/oneoff/runtime.rs`, `src/oneoff/session.rs`.

### Phase 999.11: One-off resume through the agent session (BACKLOG)

**Goal:** `task resume` continues a paused, stopped or killed one-off through the agent's own
session (resume flag + session id) instead of a fresh session primed with the old task, so the
agent keeps its conversation: what it had read, tried and decided.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-11-03 from todo `2026-11-03-oneoff-resume-with-the-agent-session` (source: quick
261103-klf CONTEXT.md, C-09, C-10). Today resume starts a NEW run with a FRESH session primed with
the old task, the old report's headline and follow-ups and the commit log. klf chose that on
purpose: the transcript lives in the per-run config dir inside the old sandbox, resumability could
not be verified without a paid run, and the fresh path is fully testable on the fake harness.
**Approach:** behind the `AgentRuntime` seam, keep the session id from the stream's `system` init
event (classified runtime-side, never in a neutral file) and start the new session with the agent's
resume flag in the SAME per-run config dir, copied or re-bound into the new run. **Needs:** a
decision on where the old config dir goes (copy the transcript in vs re-bind the config dir); a
fallback to the primed fresh session when the transcript cannot be resumed; a paid-run check that
the resumed agent really continues from where it paused. Shares session-id plumbing with Phase
999.10. Files: `src/oneoff/resume.rs`, `src/oneoff/launch.rs`, `src/oneoff/runtime.rs`.

### Phase 999.12: Second agent runtime behind the one-off seam (BACKLOG)

**Goal:** Add a second `oneoff::runtime::AgentRuntime` variant (Codex, ...) so the seam's shape is
tested against a real second implementation.
**Requirements:** TBD
**Plans:** 0 plans

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

Captured 2026-11-03 from todo `2026-11-03-oneoff-second-agent-runtime` (source: quick 261103-f5y
CONTEXT.md, C-10). Today the only variant is Claude; every agent-specific fact (argv, env, config
seeding, credential, egress upstream, stream parsing, models) lives in the runtime and the seam
guard keeps the rest neutral. **Approach:** the new variant implements the same methods: its argv
(with or without a budget cap, `caps().budget_cap`), its config dir and seeding, its credential and
egress upstream, and a stream parser into `AgentOutcome`. Chosen per run (CLI flag first, then the
confirm popup), never through a config surface unless one is asked for. Extend the seam guard's
positive control. Files: `src/oneoff/runtime.rs`, `tests/oneoff_runtime_seam_guard.rs`.
