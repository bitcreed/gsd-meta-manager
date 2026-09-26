# GSD Meta Manager

## What This Is

A Rust TUI command center for managing multiple GSD-run projects from a single interface. Users register their GSD projects and get a unified dashboard showing phase status, workflow progress, and pending actions across all of them — with live filesystem watching, ASCII roadmap visualization, project creation, work enqueueing, paused project detection, milestone archive browsing, and a read-only view of running GSD agents with live executor-wave progress.

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

### Active

**Milestone v2.0 — Autonomous Orchestration.** Requirements are defined in
`.planning/REQUIREMENTS.md` and mapped to phases in `.planning/ROADMAP.md`.

- Autonomous driver — LLM agent drives the GSD pipeline toward a user-stated goal
- Live visibility and interjection — watch driver state, read the originating prompt, inject messages mid-run
- Container support — Claude sessions in docker/podman containers with command injection
- UI fixes — HANDOFF pause badge, DRPEV leading blank, markdown edit mode, PageDown clamp

### Out of Scope

- Plugin system / extensibility — v2+, needs architecture once core stabilizes
- Telegram bridge or external integrations — v2+
- Remote project management (projects on different machines) — local first, SSH research in backlog
- Offline mode — real-time file watching is core value

## Current State

Shipped v1.6.0. 13 phases + 20 quick tasks across 7 milestones. Milestones v1.3
through v1.6.0 all shipped via `/gsd-quick` rather than formal phases: v1.3
(Defaults tab, pipeline sub-stage drill-down, tmux Tab-to-switch), v1.4 (session
auto-discovery, Docs browser tab), v1.5.0 (UAT/SPEC/EVAL-REVIEW artifact
detection, crates.io publish), and v1.6.0 (GSD 1.8.0 on-disk format catch-up
across state readers, config schema, and UI).

## Current Milestone: v2.0 Autonomous Orchestration

**Goal:** Turn the meta-manager from a dashboard that watches GSD projects into
one that runs them — an LLM agent drives the pipeline toward a user-stated goal,
in containers or on the host, watchable and interruptible from the TUI.

**Target features:**

- Autonomous driver — user states a goal once; a driver loop reads project state, picks the next GSD command, and runs it via `claude -p` until the goal is met or it parks
- Live visibility and interjection — LLM-driven projects marked in the overview, originating goal prompt viewable, driver state streamed live, messages injectable mid-run via an attached tmux pane
- Container support — start/stop/resume Claude sessions in containers mapped to project dirs, runtime auto-detected (docker or podman), with injection and output monitoring from the TUI
- UI fixes — HANDOFF.md pause badge, DRPEV leading blank, markdown edit mode activation, PageDown scroll clamp

## Context

- Shipped v1.6.0; ~7,000 LOC Rust across 7 milestones, distributed via crates.io
- Tech stack: Rust, ratatui 0.30, crossterm 0.29, tokio, notify-debouncer-full
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
- **Stoppable**: Any autonomous run must be interruptible from the TUI at any point, and
  must support a dry-run mode that reports the commands it would issue without issuing them.
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
| v2.0 driver transport: `claude -p` drives, tmux pane watches | `-p` gives real exit codes and reliable completion signals; `tmux send-keys` is screen-scraping with no delivery confirmation. Splitting the roles keeps reliability *and* live watch/interject | — Pending v2.0 |
| v2.0 autonomy: fully autonomous including push and PR | User's explicit choice for maximum leverage; kill switch and dry-run made hard requirements to bound the blast radius | — Pending v2.0 |
| Container runtime auto-detected (docker or podman) | Hardcoding either one contradicts the portability constraint; probing costs little | — Pending v2.0 |
| 999.2 injection plumbing sequenced before 999.3 driver | The driver is a decision layer built on top of injection/monitoring — building it first would mean stubbing the transport twice | — Pending v2.0 |
| Major version bump to v2.0 | The tool's category changes from passive dashboard to active orchestrator, and it narrows a stated constraint | — Pending v2.0 |
| ROADMAP `Build phase` headings kept in `planned_phases`, never `phases` (Phase 24) | GSD's own heading grammar does not count them; keeping them out keeps the driver router/frontier GSD-conformant | ✓ Good — conformance test green, build phases still drawn |
| Roadmap list as a pure model (no ratatui types) with key-based cursor targets (Phase 24) | Layout/lanes testable as exact strings; folding can't strand a row-index cursor | ✓ Good — mockups pinned by unit tests on real-roadmap fixtures |
| Archive kept as a Docs sub-view sharing Docs' tab index; `switch_to_sub_view` is the single arrival rule (Phase 24) | Removing the variant would churn archive discovery; one arrival path covers digits, Enter and `m` | — Inferred during autonomous run, audit |
| Roadmap footer left at 91/87 cols (Phase 24) | Fitting 80 cols means dropping or renaming hints — a scope change | ⚠️ Revisit — `[?]help` clips at 80 cols |
| Agent observation via a facts-only `AgentAdapter` seam with `catch_unwind` per adapter and per project (Phase 25) | Runtime metadata formats are third-party and unstable; a failing adapter must degrade a row to git facts, never drop it or fail the scan | ✓ Good — Claude Code adapter added as one module + registry entry |
| Agent-worktree git reads use `--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0` (Phase 25) | Scanning must never take a lock inside a live agent's worktree (non-intrusive constraint) | ✓ Good — real-worktree test proves the live index is untouched |
| One running predicate, `AgentLiveness::is_running` (Live/Idle); agents silent > `MAX_AGENT_AGE_SECS` read `Ended` (Phase 25, 25-07) | Orphaned worktrees from aborted runs hijacked the Status cell (CR-01); one predicate stops activation, phase vote and counts from disagreeing | ✓ Good — CR-01/WR-01 resolved; WR-06 shows `fixers.rs` still counts Finished orphans [inferred: follow-up] |
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
*Last updated: 2026-09-25 after Phase 25 (Running Agents & Live Wave View)*
