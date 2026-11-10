# GSD Meta Manager

## What This Is

A Rust TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — with live filesystem watching, ASCII roadmap visualization, project creation, work enqueueing, paused project detection, milestone archive browsing, and a read-only view of running GSD agents with live executor-wave progress. Since v2.0 it can also drive a GSD project: an Opus master driver works through a human-owned DIRECTION.md inside a credential-free bwrap sandbox bounded by launcher hard caps, watched and steered from the TUI's Driver tab through the run's files.

## Core Value

See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## Requirements

### Validated

- ✓ Project registration — add/remove existing GSD projects to track — v1.0
- ✓ Status detection — read project state from `.planning/` files without running GSD commands — v1.0
- ✓ Unified dashboard — color-coded project list with rich columns, aggregate status bar, vim navigation, search/filter, help overlay — v1.0
- ✓ Live state and detail view — file watcher auto-refresh, drill-down detail panel with phase breakdown and change summary — v1.0
- ✓ Workflow visualization — ASCII roadmap with phase boxes, status icons, progress markers — v1.0
- ✓ Enqueue work — free-form command queue with suggestions, stored in .planning/QUEUE.md — v1.0
- ✓ Create new project — TUI modal for name/path, git init, hook support, auto-register — v1.0

- ✓ State reader accuracy — disk-based phase inference, plan counting fixes — v1.1
- ✓ GSD integration — verified/inferred badges, gsd_integration config toggle — v1.1
- ✓ Claude session management — detect active sessions, dashboard indicator, launch/resume — v1.1
- ✓ Queue management — full CRUD (add, edit, delete, reorder, mark done) in Queue tab — v1.1
- ✓ Git history viewer — scrollable log, planning-only toggle, diff stats — v1.1
- ✓ Backlog browser — list with content preview, queue promotion — v1.1
- ✓ Execution flow pipeline — per-phase D-R-P-E-V visualization with color-coded stages — v1.1

- ✓ Tech debt cleanup — zero warnings, zero clippy lints, clean formatting — v1.2
- ✓ Paused project detection — HANDOFF file detection with cyan pause badge on dashboard — v1.2
- ✓ Milestone archive browser — 8-tab detail view with drill-down, styled markdown, async loading — v1.2

- ✓ Defaults tab — 9-tab detail view exposing `.planning/config.json` with categorized rendering, dropdown picker for booleans/enums, inline text-input for string rows (`base_branch`, etc.), `x`-to-clear shortcut, `r`-to-reload, six sections mirroring `/gsd-settings` — v1.3
- ✓ Defaults layering — `~/.gsd/defaults.json` parsed and surfaced as fallback for unset rows with `*` marker; `[d]` toggle to edit the global defaults file directly — v1.3
- ✓ Pipeline sub-stage drill-down — Plan and Execute parents expand into per-toggle artifact rows (PATTERNS, PLAN-CHECK, VALIDATION, UI-SPEC, UI-CHECK, AI-SPEC, REVIEW, UI-REVIEW) with ✓/○ markers — v1.3
- ✓ Tmux session switching — Tab on the project list or Sessions tab jumps focus to the matching tmux pane (mirrors claudectl) — v1.3
- ✓ Markdown editing polish — `tui-textarea` + `$EDITOR` shell-out for archive/backlog markdown, PageUp/PageDown scrolling on detail screen — v1.3
- ✓ GitHub-ready README — landing-page README with quickstart, screenshots, and kid-friendly GSD explainer — v1.3
- ✓ Queue execution research — design document with 2 strategies, safety requirements, LLM-agnostic — v1.2

- ✓ Session auto-discovery — register GSD projects found in running `claude` sessions (startup scan + 5s poll) — v1.4
- ✓ Docs browser tab — 10th detail tab drill-down over `.planning/` markdown, rooted at the active phase with `g`/`p` quick-jumps — v1.4
- ✓ Refresh noise suppression — no "Updated" status when project state is unchanged — v1.4
- ✓ Version alignment — Cargo.toml tracks the milestone tag; `--version` wired through clap — v1.4

- ✓ Sub-phase artifact detection — UAT, SPEC, and EVAL-REVIEW surfaced in pipeline drill-down — v1.5
- ✓ crates.io distribution — publish metadata, `cargo install` path, semver `vX.Y.Z` tag convention — v1.5

- ✓ GSD 1.8.0 on-disk compatibility — matched-summary plan counting, roadmap heading variants and `## Progress` table, superseded plans, flexible phase-dir tokens — v1.6
- ✓ Config schema catch-up — `claude_orchestration`, `statusline`, `dynamic_routing`, `review`, `external_job`, `capabilities` blocks plus 12 workflow gates — v1.6
- ✓ Workstreams data layer — reads `.planning/workstreams/`, with workstream/external-job/artifact badges in the UI — v1.6
- ✓ Queue relocation — `.planning/meta-manager/QUEUE.md` with legacy migration — v1.6
- ✓ Git-based staleness — commit-time derived with mtime fallback — v1.6
- ✓ CI release workflow — crates.io publish on version tags — v1.6

- ✓ Roadmap tab redesign — dependency list with git-log lanes, transitive reduction (implied deps shown dim), milestone bands with folds, master-detail pane (side-by-side ≥100 cols, stacked below), build phases and goals read from ROADMAP.md — Phase 24
- ✓ Detail-tab consolidation — eight tabs plus Driver (`1:Roadmap 2:Phases … 8:Docs`, `D:Drive`); PhaseList removed, Archive folded into Docs › Milestones; Roadmap/Phases share the selected phase — Phase 24
- ✓ Running agents & live wave view — lock-free scan of agent worktrees (commits ahead, dirty count), pluggable runtime-adapter seam with a Claude Code adapter (subagent meta, transcript-mtime liveness, worktree-less subagents), plans grouped by PLAN `wave:` with done/finished/running/stalled/queued states, width-fitted dashboard Status summary, Sessions › Agents sub-view, and an estimated `~fixed/total` for code-fixer runs (AGENT-01..07) — Phase 25
- ✓ driverctl & driver frontmatter — `gsd-meta-manager driverctl serve|log|hook|show` is the single writer of a run's `state.json`, LOG.md and the DIRECTION.md frontmatter (exchange-verify splice via `renameat2(RENAME_EXCHANGE)`, 0 lost rename-saves under an editor race); typed `DriverFrontmatter` schema v1 with `stalled`/`crashed` terminal states set only by the launcher; counters (dispatches, coalesced 45 s gap rounds, heartbeat) from tool-observed state only; three recorded spike runs replay with identical LOG lines and counts (DCTL-01..04) — Phase 27
- ✓ Master prompt & run templates — embedded, versioned `driver-prompt/1.0.0` recorded on every run (`custom:<sha8>` for overrides); `driver new` mints `<yyyymmdd>-<slug>` with self-ignoring run dir, DIRECTION.md and LOG.md templates; `driver check` lints the reviewable plan; one 8-rule stop precedence stated identically in prompt and template; crash-and-relaunch reconciliation and gate-enforced `control: pause|stop`; `next <project> --json` advisory router hint (PRMT-01..05, DRIVE-03, DRIVE-01 prompt half by operator sign-off) — Phase 29
- ✓ Sandboxed driver launcher — `driver launch` runs the headless master in bwrap under `systemd-run --user --scope` (scope kill on stop, flock against a second launcher, `crash-detected` on relaunch); hard caps enforced from outside (dispatch, wall clock + RuntimeMaxSec, gap deny L+1 / kill L+2, rate-limit park); RO host `~/.claude`, masked credentials and scrubbed env, per-run `CLAUDE_CONFIG_DIR`; stored `claude setup-token` auth with `driver auth setup|status`; `driver push` outside the sandbox into `gsd-auto/<alias>/<run-id>` with secret scan, protected-branch refusal and PR cap; explicit `--unsandboxed` degraded mode (refused by default, posture recorded, `!`/UNSANDBOXED in the Driver tab), proven on a real macOS host (LNCH-01..08, DRIVE-01 launcher half). Network egress and agent-channel isolation gaps from the 2026-10-24 audit move to Phase 28.1 — Phase 28
- ✓ Driver tab on DIRECTION.md — the TUI observes runs only through the run's files (status, caps, heartbeat and crash from the tool-written frontmatter, lock-probe status rule) and steers by editing DIRECTION.md (`control`, answer, unhold, `$EDITOR` edit under `DIRECTION.lock`, polite stop; nothing is killed); launch form and relaunch from the tab; FULL tier legible at 100x30 (DTAB-01..04). Re-verified 2026-10-24 after quick 261023-v1s hardened the give-back exchange — Phase 30
- ✓ Sandbox egress & agent-channel isolation — `--unshare-net` plus an allowlisting egress proxy, a `$HOME` allowlist, the planted-file guard, agent-tool denies and the nested-agent cap check close the Phase 28 audit channels (LNCH-13, LNCH-14); the D-11 setup-token probe shows the token refused (401/403) on all four account endpoints — Phase 28.1
- ✓ UI fixes — HANDOFF pause badge, DRPEV leading blank, markdown edit mode, PageDown clamp (UIFIX-01..04) — Phase 14 / v2.0
- ✓ Run journal & redaction substrate — append-only redacted NDJSON journal, run dirs with an ignore posture, SAFE-04 capture-time redaction (journal half Phase 16, launcher pump half quick 261029-1dp) — v2.0
- ✓ Driver validation — spike 003 re-run proved gap rounds 1+2 and divergence parking on Opus (VALID-01; VALID-02 accepted as ADJUST) — Phase 26.1 / v2.0
- ✓ Advisory router hint — deterministic `next <project> --json` (Phase 20, kept advisory after the re-plan) — v2.0
- ✓ Retirement — superseded Phase 19/21/22 envelope and goal-layer code removed or rewired, their threat ids closed as superseded (RETIRE-01..03) — Phase 31 / v2.0
- Superseded in v2.0 (not shipped, by re-plan or user decision): CTRL-02/CTRL-03 (dry-run, opt-in gate), CTRL-08, SAFE-02/05/07/08, DRIVE-04, TRANS-01/04/05, STEER-02; moved to future: DRIVE-07, CTNR-01..05. CTRL-05 accepted partial (lock per run dir, not per project). See `.planning/milestones/v2.0-REQUIREMENTS.md`.

### Active

**v2.3.0 Multiplexer Backends** (started 2026-11-10) — requirements in `.planning/REQUIREMENTS.md`:

- [ ] **MUX-01..MUX-07** — reusable terminal-multiplexer backend trait, tmux migrated onto it,
  herdr backend, innermost-multiplexer detection for nesting, GUI-terminal fallback preserved.

Carried forward from v2.0 (not in v2.3.0 scope) `[inferred — release 2026-10-29]`:

- **Security-adjacent review findings from v2.0** — five clustered todos in `.planning/todos/pending/`
  (`2026-10-29-v2-carry-*`): capture-pump redaction edge cases, unsupervised child on capture/parent
  failure, lock hard-link denial of `driver stop`, managed-policy shape validation, and run-file
  open/size hardening. Full per-finding status lives in each archived phase's `*-REVIEW-DISPOSITION.md`
  under `.planning/milestones/v2.0-phases/` (123 entries: 0 critical, 41 warning, 82 info).
- **CTRL-05 per-project lock** — accepted partial in v2.0; the lock is per run dir.
- **Master prompt 1.1** — backlog 999.7. **GSD capabilities visibility** — backlog 999.8 (formerly Phase 32).
- **Open human calls** — OQ-R1, OQ-R2 and the Phase 26 INCONCLUSIVE reading (Key Decisions, ⚠️ Revisit).

<details>
<summary>v2.0 Active list as it stood at close (historical)</summary>


**Milestone v2.0 — Autonomous Orchestration.** Requirements are defined in
`.planning/REQUIREMENTS.md` and mapped to phases in `.planning/ROADMAP.md`.

*Re-planned 2026-10-14 after spikes 001 and 002. Phases 19, 21, 22 and 23 are superseded, and
Phases 26-31 replace them.*

- **Driver validation (spike 003).** Before any Rust is written, gap rounds 1-2, divergence parking
  and the async no-respawn rule must be shown on a real GSD phase.
  *Phase 26 (2026-10-16): NOT validated. Spike 003 verdict INCONCLUSIVE. Both Run 1 attempts were
  SEED STALLED, Run 2 was skipped under the pre-registered rule, and $7.81 of the $60 ceiling was
  spent. Rounds 1+2, divergence parking and the Opus-tripped launcher cap were never observed
  (VALID-01 Unproven, VALID-02 Partial: launcher deny L+1 / kill L+2 shown only by a scripted run).
  Zero duplicate worker dispatches were measured where dispatches occurred.*
  *Phase 26.1 (2026-10-18): validated with ADJUST. Seed and instruments repaired; VALID-01 proven on
  an organic Opus driver (r261a: rounds 1+2, divergence park needs-human at count 3, DUP=0). VALID-02
  accepted by the operator as ADJUST: divergence parking proven on Opus, launcher deny L+1 / kill L+2
  proven only by the scripted Haiku run r261-l7 (Opus driver self-parked at count 2, so deny/kill
  NOT-REACHED). Risk carried to Phases 28/29. Phase 27 unblocked.*
- **Spike 003 redesign or re-plan (emerged, Phase 26).** *Resolved by Phase 26.1 (redesign + re-run chosen) `[inferred — audit]`.* Before Phase 27 starts, either redesign
  the seed and instruments, or re-plan Phases 27-31. A redesign needs a divergence signal that
  survives a verifier re-writing VERIFICATION.md (one gap round was counted twice), and a way to
  wake the headless driver before a worker reports. See
  `.planning/spikes/003-divergent-gap-loop-validation/README.md`.
- **driverctl hardening (emerged, Phase 27).** 27-REVIEW.md left CR-01 (gap scan reads
  driver-plantable files with no type/size guard, under the store mutex) and WR-01..09 open
  (27-REVIEW-DISPOSITION.md). Close or explicitly defer them before Phase 28's launcher relies on
  `driverctl serve` `[inferred — audit]`.
- **Master prompt 1.1 (emerged, Phase 29).** Route a "reconcile first" refusal to reconcile-then-
  continue instead of a stop-rule-2 park, and fix IN-01 (`$RUN_DIR`/`$PROJECT_ROOT` used as env vars).
  Backlog 999.7.
- **DIRECTION.md write-path hardening (emerged, Phase 30 re-review 2026-10-24).** 30-REVIEW.md
  WR-10 (`give_back_newest` exchange error incl. ENOENT leaves the newest save at a dot-temp the
  60 s sweep can delete; fix: NOREPLACE rename on ENOENT), WR-08 (`DIRECTION.lock` opened following
  symlinks, `src/driver/lock.rs`) and WR-09 (`read_run_file` has no 1 MiB cap) are open in
  30-REVIEW-DISPOSITION.md; none blocked verification `[inferred — audit]`.
- **Retirement.** The superseded Phase 19, 21 and 22 code is removed or rewired. *Done — Phase 31.*

</details>

### Out of Scope

- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — local first, SSH research in backlog
- Offline mode — real-time file watching is core value

## Current State

Shipped v2.0.0 Autonomous Orchestration (2026-10-29): Phases 14-31 (20 phases incl. 26.1 and
28.1; 19/21/22/23 closed by supersession, 26 superseded by 26.1) plus 109 quick tasks and 1 fast task, with interim
releases v1.7.0 through v1.15.0 cut along the way. The tool now runs GSD projects as well as
watching them: `driver new|check|launch|push|auth`, `driverctl` as the single writer of run state,
a sandboxed launcher with hard caps, and a Driver tab that steers through DIRECTION.md. Audit
verdict tech_debt (70/71 requirements, 1 accepted partial; 123 open/deferred review entries, none
critical). Archive: `.planning/milestones/v2.0-ROADMAP.md`, `v2.0-REQUIREMENTS.md`,
`v2.0-MILESTONE-AUDIT.md`, `v2.0-phases/`.

<details>
<summary>Before v2.0</summary>

Shipped v1.6.0. 13 phases + 20 quick tasks across 7 milestones. Milestones v1.3
through v1.6.0 all shipped via `/gsd-quick` rather than formal phases: v1.3
(Defaults tab, pipeline sub-stage drill-down, tmux Tab-to-switch), v1.4 (session
auto-discovery, Docs browser tab), v1.5.0 (UAT/SPEC/EVAL-REVIEW artifact
detection, crates.io publish), and v1.6.0 (GSD 1.8.0 on-disk format catch-up
across state readers, config schema, and UI).

</details>

## Current Milestone: v2.3.0 Multiplexer Backends

**Goal:** Focus and launch Claude sessions from the TUI in whatever terminal workspace the user
runs (tmux today, herdr now, others later) through one reusable backend seam, instead of the
tmux-hardcoded `src/terminal_switch.rs`.

**Target features:**
- A `MultiplexerBackend` trait (detect, focus an existing session, open a new session in a cwd with
  an argv) designed against tmux, herdr, zellij, WezTerm mux, kitty remote control, GNU screen and
  Rex, with innermost-multiplexer precedence when nested (e.g. herdr inside tmux)
- tmux migrated onto the trait with its direct-exec safety invariants intact
- herdr backend joining on `agent_session` (Claude session id) from `herdr pane list`
- GUI-terminal fallback preserved; other backends recorded as future requirements

**Key context:** started unattended from a user request; the milestone holds a single phase.
Remaining carried-forward candidates (v2.0 review clusters, CTRL-05, backlog 999.7/999.8) stay
deferred `[inferred — milestone start 2026-11-10]`.

<details>
<summary>v2.0 Autonomous Orchestration — milestone goal (shipped 2026-10-29)</summary>

**Goal:** Turn the meta-manager from a dashboard that watches GSD projects into one that runs them.
An Opus master driver works through a human-owned DIRECTION.md, one fresh subagent per GSD step. It
runs inside a sandbox that has no credentials and is bounded by hard caps in the launcher. It is
watchable and steerable from the TUI through the run's files.

**Re-plan (2026-10-14):** the original target features below, as far as containers and the
LLM-goal-layer design, were superseded by spikes 001/002. See Active and Key Decisions.

**Target features:**

- Autonomous driver — user states a goal once; a driver loop reads project state, picks the next GSD command, and runs it via `claude -p` until the goal is met or it parks
- Live visibility and interjection — LLM-driven projects marked in the overview, originating goal prompt viewable, driver state streamed live, messages injectable mid-run via an attached tmux pane
- Container support — start/stop/resume Claude sessions in containers mapped to project dirs, runtime auto-detected (docker or podman), with injection and output monitoring from the TUI
- UI fixes — HANDOFF.md pause badge, DRPEV leading blank, markdown edit mode activation, PageDown scroll clamp

</details>

## Context

- Shipped v2.0.0; ~166K lines Rust in `src/` plus ~32K in `tests/`, across 8 milestones, distributed via crates.io
- v2.0 driver stack: `src/driverctl/` (single writer), `src/launcher/` (bwrap + `systemd-run --user --scope`, egress proxy, push step), `src/driver/` (DIRECTION.md, lock), embedded master prompt; Linux sandboxed, macOS degraded `--unsandboxed`
- Known technical debt: 123 open/deferred review entries across Phases 24-31 (see `*-REVIEW-DISPOSITION.md` in `.planning/milestones/v2.0-phases/`); security-adjacent clusters tracked as todos
- Tech stack: Rust, ratatui 0.30, crossterm 0.29, tokio, notify-debouncer-full
- `src/driverctl/` + `src/driver/direction.rs` (Phase 27): run-store, unix-socket server, client/hook, DIRECTION.md splice and run lock
- Screen trait architecture; detail view is 8 tabs + Driver since Phase 24 (Archive lives under Docs › Milestones; Sessions › Agents since Phase 25)
- Running-agent scan (`src/agents/`) rides the 5 s session poll in `spawn_blocking`; read-only, no locks taken in agent worktrees
- Disk-based phase inference via /proc-like directory scanning
- Queue is fully managed (CRUD) with execution design ready for v1.3
- Claude session detection via pgrep + /proc (Linux-only)
- Paused project detection via HANDOFF.md/HANDOFF.json with content validation
- Milestone archive browser with 4-level drill-down and styled markdown rendering
- Zero build warnings, zero clippy warnings, 25 integration tests pass

## Constraints

- **State reading**: Must not require running Claude/GSD to check status — read from files or cached state
- **Opt-in driving** *(supersedes "Non-intrusive" as of v2.0)*: The tool may drive a
  project's GSD pipeline only when that project is explicitly opted in. Projects that
  are not opted in must never be touched — observation stays strictly read-only, exactly
  as before. v1.x's blanket "must not interfere with running GSD instances" is
  deliberately narrowed here, not dropped: interference is now a per-project choice the
  user makes, and the default remains no interference.
  **Amended 2026-10-29 — user decision: CTRL-02/CTRL-03 superseded by sandboxed launcher + DIRECTION.md stop rules; Phase 31 retirement of drive dry-run/opt-in gate accepted.** The opt-in record now shapes push policy only; a run starts only through an explicit `driver launch` or a Driver-tab new run.
- **Stoppable**: Any autonomous run must be interruptible from the TUI at any point, and
  must support a dry-run mode that reports the commands it would issue without issuing them.
  **Amended 2026-10-29 — user decision: CTRL-02/CTRL-03 superseded by sandboxed launcher + DIRECTION.md stop rules; Phase 31 retirement of drive dry-run/opt-in gate accepted.** The dry-run clause is superseded; stoppability stands.
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| V1 scope: dashboard + visualization + enqueue + create | Focus on the core management loop before advanced features | ✓ Good — shipped all 22 requirements |
| Rust + ratatui over Python + Textual | Single binary, zero-cost abstractions, no runtime dependency | ✓ Good — 7.6K LOC, fast, portable |
| Claude session detection via pgrep + /proc | Reliable on Linux, no Claude API dependency | ✓ Good — works for local sessions, Linux-only |
| Queue as managed CRUD, not executable | Execution needs GSD hooks research, CRUD sufficient for v1.1 | ✓ Good — execution design ready for v1.3 |
| Screen trait refactor in Phase 05 | 11 InputMode variants would explode with new views | ✓ Good — enabled 8-tab detail view cleanly |
| File-watching over polling for live updates | notify-debouncer-full with 200ms debounce | ✓ Good — responsive without CPU overhead |
| Custom markdown renderer over pulldown-cmark | No new dependency, line-by-line regex sufficient for archive display | ✓ Good — tiered header styling, code blocks, bold |
| Per-item isolation for queue execution (v1.3) | Simpler than session chaining, each item runs independently from disk state | — Pending v1.3 implementation |
| LLM-agnostic queue execution design | GSD could use any LLM backend, not just Claude | ✓ Good — Executor trait interface designed |
| Ship v1.3–v1.6 via `/gsd-quick` instead of formal phases | Each was a bounded, well-understood change; phase overhead wasn't earning its keep | ✓ Good — 4 milestones shipped, but PROJECT.md/MILESTONES.md drifted 3 milestones behind (backfilled at v2.0 start) |
| v2.0 driver transport: `claude -p` drives, tmux pane watches | `-p` gives real exit codes and reliable completion signals; `tmux send-keys` is screen-scraping with no delivery confirmation. Splitting the roles keeps reliability *and* live watch/interject | ✓ Kept: headless `claude -p` and no tmux on the control path. Steering moved from stdin injection to DIRECTION.md edits (2026-10-14) |
| v2.0 autonomy: fully autonomous including push and PR | User's explicit choice for maximum leverage; kill switch and dry-run made hard requirements to bound the blast radius | Amended 2026-10-29 by user decision (dry-run and the opt-in gate superseded; see next row) |
| CTRL-02/CTRL-03 superseded (v2.0 audit closure) | 2026-10-29 — user decision: CTRL-02/CTRL-03 superseded by sandboxed launcher + DIRECTION.md stop rules; Phase 31 retirement of drive dry-run/opt-in gate accepted. The sandbox plus DIRECTION.md stop rules bound the blast radius instead | ✓ User decision 2026-10-29; doc-only |
| Container runtime auto-detected (docker or podman) | Hardcoding either one contradicts the portability constraint; probing costs little | ✗ Superseded 2026-10-14 (Phase 22 dropped; bwrap + scope instead) |
| 999.2 injection plumbing sequenced before 999.3 driver | The driver is a decision layer built on top of injection/monitoring — building it first would mean stubbing the transport twice | ✗ Superseded by the 2026-10-14 re-plan (stdin injection replaced by DIRECTION.md steering) |
| Major version bump to v2.0 | The tool's category changes from passive dashboard to active orchestrator, and it narrows a stated constraint | ✓ Shipped as v2.0.0 on 2026-10-29 |
| ROADMAP `Build phase` headings kept in `planned_phases`, never `phases` (Phase 24) | GSD's own heading grammar does not count them; keeping them out keeps the driver router/frontier GSD-conformant | ✓ Good — conformance test green, build phases still drawn |
| Roadmap list as a pure model (no ratatui types) with key-based cursor targets (Phase 24) | Layout/lanes testable as exact strings; folding can't strand a row-index cursor | ✓ Good — mockups pinned by unit tests on real-roadmap fixtures |
| Archive kept as a Docs sub-view sharing Docs' tab index; `switch_to_sub_view` is the single arrival rule (Phase 24) | Removing the variant would churn archive discovery; one arrival path covers digits, Enter and `m` | — Inferred during autonomous run, audit |
| Roadmap footer left at 91/87 cols (Phase 24) | Fitting 80 cols means dropping or renaming hints — a scope change | ⚠️ Revisit — `[?]help` clips at 80 cols |
| Agent observation via a facts-only `AgentAdapter` seam with `catch_unwind` per adapter and per project (Phase 25) | Runtime metadata formats are third-party and unstable; a failing adapter must degrade a row to git facts, never drop it or fail the scan | ✓ Good — Claude Code adapter added as one module + registry entry |
| Agent-worktree git reads use `--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0` (Phase 25) | Scanning must never take a lock inside a live agent's worktree (non-intrusive constraint) | ✓ Good — real-worktree test proves the live index is untouched |
| One running predicate, `AgentLiveness::is_running` (Live/Idle); agents silent > `MAX_AGENT_AGE_SECS` read `Ended` (Phase 25, 25-07) | Orphaned worktrees from aborted runs hijacked the Status cell (CR-01); one predicate stops activation, phase vote and counts from disagreeing | ✓ Good — CR-01/WR-01 resolved; WR-06 shows `fixers.rs` still counts Finished orphans [inferred: follow-up] |
| v2.0 re-plan (2026-10-14): safety by capability removal + launcher hard caps instead of parsing bash (Phase 19) or an LLM goal layer (Phase 21) | Both phases ran 13 gap rounds without converging. Each round found a new carrier class. Spikes 001/002 showed that a sandbox with no push credentials, a read-only run dir and caps counted from tool-written state bound the blast radius mechanically | — Pending. Phase 26 was INCONCLUSIVE; Phase 26.1 (2026-10-18) confirmed gap rounds 1+2 and divergence parking on Opus (VALID-01) and VALID-02 as ADJUST; Phase 27 unblocked, launcher-cap risk carried to 28/29 |
| Spike 003 verdict read as INCONCLUSIVE, not NO-GO (Phase 26) `[inferred]` | The rubric's NO-GO (d)/(e) conditions hold literally on the two STALLED runs. They are judged only on a Run 1 with a VALID seed, and none exists; the literal reading would leave the INCONCLUSIVE clauses unreachable | ⚠️ Revisit. Human to confirm; a literal reading gives NO-GO. Routing is the same either way: Phase 27 is blocked until a redesign or re-plan |
| VALID-02 accepted as ADJUST (Phase 26.1, operator decision 2026-10-18) | Launcher deny/kill is model-independent launcher code, proven by the scripted Haiku run r261-l7; the Opus driver self-limited at gap count 2, so a further paid run would test the model, not the launcher. The prereg terminal rule barred more Run 2s inside 26.1 | — Pending. Phases 28/29 must exercise launcher deny/kill under the real driver |
| Spike 003 rubric and instruments frozen before spend; post-hoc fixes recorded only as redesign inputs (Phase 26) | Pre-registration (PREREG_SHA 4ba6195) keeps the verdict mechanical. Adapting the instruments to the results would launder it | ✓ Good. The verdict is honest, and the gap double-count was identified as a redesign input |
| Phases 19/21/22/23 superseded, code kept until a dedicated retirement phase (31) `[inferred]` | Removing code before the launcher's push step owns SAFE-01/03/06 would leave a gap. An explicit phase keeps the removal reviewable | ✓ Good — Phase 31 removed it (13ba5ac) and closed the threat ids as superseded |
| v2.0 restructured in place, not closed and reopened `[inferred]` | Closing would need a release tag and an audit of two phases that cannot close. v2.0 never released, and the major-bump rationale still holds | ✓ Good — closed as one milestone, audit tech_debt with no blockers |
| Containers (Phase 22) replaced by bwrap + `systemd-run --user --scope` on Linux, with a recorded degraded mode on macOS `[inferred]` | Verified with auth in spikes 001/002, with no new dependency (bollard never added). Containers move to Future Requirements | ✓ Good — Phase 28 (live bwrap isolation + scope kill on Linux; `--unsandboxed` degraded run proven on macOS 26.6.2). Egress/agent-channel gaps → Phase 28.1 |
| Status frontmatter on DIRECTION.md is tool-written by driverctl (single writer, exchange-verify splice), never by the model | Spike 002 measured 0 lost rename-saves in 12,943. The driver made 18 driverctl calls with 0 hand-edits. The Rust reader already parses it | ✓ Good — Phase 27 shipped it (verification 4/4; ported editor race 0 lost saves; SC1 replay identical); CR-01 + WR-01..09 review findings open |
| driverctl Gate denies on transport error, unset socket, unreadable reply and server `ok:false` (Phase 27, D-25 deviation from the spike) | A sandboxed driver must fail closed when the single writer is unreachable | — Pending (Phase 28 wires the Gate) |
| `crashed` and `stalled` are set only by the launcher via `RunStore::set_terminal`; no driver op reaches them; `caps.dispatches.used` counts only hook-observed, Gate-allowed dispatches (Phase 27) | Terminal and cap state must come from tool-observed facts, never the model's own claims | ✓ Good — mutation-checked: each single-writer guard fails its test when removed |
| Auth through a long-lived `claude setup-token` token passed as an env var; no refresh token in the sandbox | Spike 002 measured that a host refresh rotates the refresh token and revokes the previous access token, so copied credentials would log the host out | ✓ Good — Phase 28 shipped it (launcher_auth 11/11; stored token is the only `CLAUDE_CODE_OAUTH_TOKEN`, host refresh token never enters the sandbox) |
| OQ-R1: driver stop rules outrank a human `answer:`; only a budget edit lifts them `[inferred]` | Spike 002 finding 5 default. Open human call | ⚠️ Revisit — human to confirm |
| OQ-R2: driven runs do not see the user's global CLAUDE.md; GSD + rtk hooks kept `[inferred]` | Spike 002 A2.1 default. The driver's rules come from its prompt | ⚠️ Revisit — human to confirm |
| Phase 29 closed on accepted acceptance outcomes (operator decision 2026-10-20): o1 ADJUST x2 accepted; h2 H2-5 FAIL ruled a rubric setup flaw (item already done at relaunch, so no recovery owed; harness precondition fixed in WR-07, 04a7a40); no re-run | The real-Opus third gaps_found park was never reached live; carried risk is covered by 28-09 d33, h4 and the deterministic reconcile suites. A further paid run would test the model, not the prompt | ✓ Accepted — DRIVE-01 prompt half closed |
| Fixer progress shown as an estimate (`~F/T`) from REVIEW.md `findings.total` + `fix(NN)` commit ids (Phase 25) | No authoritative fix ledger exists mid-run; labelling it approximate is honest | ⚠️ Revisit — WR-03/WR-04 (wrong REVIEW file, inflated count) open |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-11-10 after starting milestone v2.3.0 Multiplexer Backends*
