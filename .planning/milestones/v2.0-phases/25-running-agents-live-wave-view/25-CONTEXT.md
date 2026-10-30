# Phase 25: Running Agents & Live Wave View - Context

**Gathered:** 2026-09-25
**Status:** Ready for planning
**Mode:** discuss-phase `--auto`, unattended (human unavailable). Decisions under "Locked" come from the user: the ROADMAP.md Phase 25 seed evidence plus architecture requirements relayed by the coordinator for this session. They are not open for re-litigation. Items tagged **[inferred]** were decided by the agent from planning artifacts, the codebase, and live inspection of `~/.claude/projects`; review them in the audit.

<domain>
## Phase Boundary

For each registered project, show which GSD agents are running in parallel right now and, for executor runs, which wave and which plans are running, queued or done. Everything is read from files and git. Nothing invokes Claude, Codex or GSD, and nothing writes to or locks anything the running GSD session uses.

Delivers:
1. A **runtime-agnostic core** that models agent worktrees and waves from git and `.planning/`.
2. A pluggable **enrichment-adapter seam**, plus one adapter for **Claude Code**.
3. **UI:** a compact summary on the dashboard row, and a scrollable per-agent and per-wave view in the project detail view.

Out of scope: acting on agents (kill, remove a worktree, merge), the Codex adapter (deferred, see D-A08), and any runtime other than Claude Code.

This phase is TUI-only and read-only. It does not depend on Phases 15-24.

</domain>

<decisions>
## Implementation Decisions

### A. Architecture (LOCKED, user-supplied)

- **D-A01: Two layers.**
  - (a) A runtime-agnostic **core**:
    - git worktree enumeration with `git worktree list --porcelain`;
    - per-worktree commit count with `rev-list --count <base>..HEAD`, and a dirty-file count;
    - the wave model, built from the PLAN.md `wave:` frontmatter plus SUMMARY.md presence.
  - (b) Pluggable per-runtime **enrichment adapters**. Each adapter attaches an agent type, a description and liveness to a worktree the core found.
- **D-A02: Group by `wave:`, never by plan number.** Plan numbers do not follow wave order: in ttbook, 13-33 is in wave 2, 13-26 is in wave 5 and 13-27 is in wave 4. A SUMMARY.md being present means the plan is done.
- **D-A03: The adapter trait tolerates missing or unparseable data.** The formats it reads are undocumented. Any adapter failure, including a missing directory, bad JSON, unknown fields or missing fields, degrades the row to what the core knows (worktree path plus branch). An adapter failure never drops the row and never fails the scan.
- **D-A04: The Claude Code adapter is IN SCOPE.**
  - Metadata comes from `~/.claude/projects/<project path, encoded>/*/subagents/agent-<id>.meta.json`, using `agentType`, `description` and `worktreePath`.
  - Liveness is the mtime of the sibling `agent-<id>.jsonl`.
- **D-A05: The lock-line pid is not per-agent liveness.** The `claude agent agent-<id> (pid N start T)` lock reason carries the pid of the top-level session, which every agent shares. A live pid does not mean a live agent.
- **D-A06: The core always parses the Codex-style branch pattern.** For `worktree-agent-p{plan}-{unix_ts}` it recovers the plan id and the spawn timestamp from the branch name, whatever the runtime. It costs nothing, and under Codex it is the only attribution available without an adapter.
- **D-A07: The adding-an-adapter test.** Adding a runtime adapter must be a self-contained change: one new module that implements the trait, plus one registration line. The core, the wave model and the UI must not change to accept it.
- **D-A08: The Codex adapter is DEFERRED by default.** Its scope is pending a user answer. The design notes are kept under Deferred Ideas. If the user pulls it in, D-A07 guarantees it stays a separate plan.
- **D-A09: `WAVE_WORKTREE_MANIFEST` is explicitly NOT a data source.** It lives at a random `mktemp` path (`$TMPDIR/gsd-worktree-wave-XXXXXX.json`) and could not be found on disk mid-wave. Do not scan `$TMPDIR` for it.
- **D-A10: Also not a data source: GSD 1.8.0's `waves.json`.** It exists only under the claude-orchestration backend (see the `src/state_reader/plan_waves.rs` module docs). Frontmatter is the authority.

### B. Project constraints (LOCKED, user-supplied)

- **D-B01: Read-only, and no Claude or GSD invocation.** Data comes from file reads, stats and read-only git.
- **D-B02: Non-intrusive git.**
  - Every git call uses `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0`.
  - Plain `git status` refreshes the index, which takes `index.lock` inside a live agent's worktree. `--no-optional-locks` prevents that.
  - Use `git status --porcelain` with `--no-optional-locks` for the dirty count.
  - No `fetch`, no `gc`, no ref writes, no `worktree prune`.
- **D-B03: Portable.**
  - Nothing is hard-coded to this user's paths.
  - Home and config roots are resolved at runtime.
  - The feature works without `/proc`: liveness is based on mtime only.
- **D-B04: Follow the existing polling, watching and caching patterns** (see D-C01).

### C. Agent-inferred detail (audit these)

- **D-C01 [inferred]: Cadence.**
  - The scan rides the existing ~5s session-poll counter in `Action::Tick` (`src/app.rs` ~1130). It gets no timer of its own; that code's comments explicitly forbid a second timer.
  - The scan runs in `tokio::task::spawn_blocking` and returns through a new `Action` variant, for example `AgentsScanned { per_project }`. Results are cached per alias in `AppContext`.
  - No `notify` watchers go on `~/.claude` or on worktree directories. They would cost a lot for large session trees, and polling already bounds staleness at about 5s.
- **D-C02 [inferred]: Git helper.** Reuse `git_read_raw` in `src/state_reader/git_ops.rs` (~198), which already passes `--no-optional-locks`, and add `GIT_OPTIONAL_LOCKS=0` to the environment. Run it once per agent worktree with `-C <worktree>`. `git_last_commit_time` in the same file lacks the flag; fixing it is out of scope, but do not copy it.
- **D-C03 [inferred]: Which worktrees count as agent worktrees.** A worktree counts if any of these holds:
  - its path is under `<project>/.claude/worktrees/`;
  - its branch matches `worktree-agent-*`;
  - an adapter claims it.

  Other user worktrees, and the main worktree, are ignored.
- **D-C04 [inferred]: The commit base.**
  - `<base>` is the HEAD commit of the main worktree, which is the first entry of `git worktree list --porcelain`, read on the same scan.
  - GSD branches agent worktrees from the checked-out branch. That is `master` in ttbook, but it is not always `master`, so the name is never hard-coded.
  - Right after spawn, both the commit count and the dirty count are 0.
- **D-C05 [inferred]: Encoding the Claude projects path.**
  - The directory name encodes the registered project path forward by replacing every non-alphanumeric character with `-`.
  - Observed examples:
    - `/home/blk/.local/state` becomes `-home-blk--local-state`;
    - a torrent-style name with `.` and `-` also collapses to `-`.
  - The encoding is lossy, so never decode it.
  - Try the registered path first, then its canonicalized form.
  - The config root is `$CLAUDE_CONFIG_DIR` if set, else `~/.claude`.
- **D-C06 [inferred]: Joining metadata to a worktree.**
  - Primary: `worktreePath` equals the worktree path.
  - Nested agents (`spawnDepth` 2) have `inheritedWorktreePath` instead. They attach to that worktree as children.
  - Fallback: the agent id in `.claude/worktrees/agent-<id>` or `worktree-agent-<id>` equals the `agent-<id>` meta stem.
  - Fields observed across 2470 local meta files:
    - always present: `agentType`, `description`, `spawnDepth`, `toolUseId`;
    - usually present: `parentAgentId`, `model`;
    - sometimes present: `worktreePath`, `worktreeBranch`, `inheritedWorktreePath`, `spawnedWithWorktree`, `worktreeCleanlyRemoved`, `stoppedByUser`, `name`, `isFork`, `requestShape`.
  - Deserialize every field as optional and ignore unknown fields.
- **D-C07 [inferred]: Subagents without a worktree are shown.**
  - Researchers, planners, and fixers that run without isolation have a meta file but no `worktreePath`.
  - The goal says "which GSD agents are running". So the Claude adapter also reports *live* subagents of this project's sessions that have no worktree, in their own group below the worktree agents.
  - `agentType` is shown verbatim, with no filtering to `gsd-*`.
  - Worktree-less agents are listed only while they are live. A worktree-less agent with a stale transcript is just history.
- **D-C08 [inferred]: Liveness states.**

  | State | Rule |
  |---|---|
  | `live` | the transcript mtime is 120s old or less (live agents were observed within ~40s) |
  | `idle` | older than 120s but no more than 10 min |
  | `stalled` | stale beyond 10 min while the worktree is still present |
  | `ended` | the meta has `worktreeCleanlyRemoved` or `stoppedByUser` |
  | `unknown` | there is a worktree but no adapter data |

  - The thresholds are named constants in one place.
  - A 0-commit, 0-dirty worktree counts as "stalled" only together with a stale transcript. It is never stalled on its own, because 0/0 is the normal state right after spawn.
- **D-C09 [inferred]: Bound the scan cost.**
  - Only stat the transcripts; never read their content.
  - For worktree agents, look the meta up by agent id.
  - For worktree-less agents, only consider session directories whose `subagents/` mtime is within the `idle` window.
  - Scan only the registered projects.
- **D-C10 [inferred]: Attributing plans and deriving wave state.** A plan's executor is attributed from these sources, in priority order:
  - the adapter description, matched by `plan (\S+) of phase (\S+)` (for example "Execute plan 13-13 of phase 13", or "Execute plan 22 of phase 21", where a bare plan number means `<phase>-<nn>`);
  - the `p{plan}` component of the branch (D-A06);
  - the `.git/worktrees/<name>/gsd-plan-head-before-<phase>-<plan>` ledger. This file is written before the executor's first commit only, so it is used as confirmation and never required.

  Each plan in the active phase then has one state:
  - `done` if its SUMMARY.md exists;
  - `running` if a live or idle agent is attributed to it;
  - `stalled` if the only agent attributed to it is stalled;
  - `queued` otherwise.

  The phase and wave:
  - The **active phase** is the phase named by the attributed agents. If there are none, it falls back to the STATE.md current phase.
  - The **current wave** is the lowest wave number that still has plans that are not done.
- **D-C11 [inferred]: Reuse `plan_waves.rs`.** Wave grouping reuses `src/state_reader/plan_waves.rs` (`plan_wave_number`, `PlanWave`, and the `w?` unknown bucket) and the disk-status scan that already reads the plan files. Do not add a second frontmatter reader.
- **D-C12 [inferred]: Fix runs from code review.**
  - Where no plan is attributed and `agentType` is `gsd-code-fixer`, show a "fixed/total" estimate:
    - the total comes from `findings.total` in the `NN-REVIEW.md` frontmatter;
    - the fixed count comes from distinct finding ids (`WR-08`, `CR-01`, …) in commit subjects `fix(NN): …` on the agent branches plus the base.
  - Label it as an estimate (`~5/12 fixed`).
  - This is lowest priority. The planner may split it into the final plan.
- **D-C13 [inferred]: Untrusted text.** The meta `description`, `agentType` and branch names are text controlled by agents or other external sources.
  - Route them through the existing `Untrusted` type and escape path in `src/text.rs`.
  - Add the new view to the `render_escape_guard` coverage.
- **D-C14 [inferred]: Dashboard placement.**
  - There is no new dashboard column: the 80-column layout in `src/ui/screens/normal.rs` (`dashboard_columns` ~332) is already at its budget.
  - While agents are running, the compact summary replaces the Status cell text, for example `P13 · w2/11 · 13 run · 8/35 done`, or `3 fixers · ~5/12` for fix runs.
  - It degrades by width, dropping segments from the right.
  - With no agents, the row is unchanged.
  - It does not add a badge; `alias_badge` keeps its single-badge rule.
- **D-C15 [inferred]: Detail placement.**
  - The Phase 24 consolidation to eight tabs is kept, so there is no ninth tab.
  - The Sessions tab (6:Ss) gets sub-views `Sessions | Agents`, following the pattern that Docs uses for `Files | Milestones` (D-B04 in Phase 24).
  - The Agents sub-view shows:
    - a header line with the same summary as the dashboard;
    - per-wave rows (`w2  running 13 · done 8 · queued 14`, one row per wave, with the current wave highlighted);
    - a scrollable per-agent list with glyph/state, plan or description, agentType, commits, dirty count, and the age of the last activity;
    - the worktree-less live agents as a separate group.
  - The toggle key and the scroll keys are left to the planner. They must not conflict with the `n` key already bound on the Sessions tab.
- **D-C16 [inferred]: Degraded and empty states.**
  - "No running agents" when there are no agent worktrees and no live subagents.
  - Rows without metadata render the branch and path with "(no agent metadata)".
  - A git failure on one worktree shows `?` for its counts and never hides the row.
- **D-C17 [inferred]: Testability.** Adapters and the core take their roots as parameters (the Claude config root and the project path). Tests build fixture git repos with real worktrees and a fake `~/.claude` tree in a tempdir. No test reads the real home directory.
- **D-C18 [inferred]: Requirements.** ROADMAP says "Requirements: TBD (to be derived in discuss)". The IDs proposed for the planner to adopt into REQUIREMENTS.md and ROADMAP.md are listed in the table below.

  | ID | Requirement | Decisions |
  |---|---|---|
  | AGENT-01 | Enumerate agent worktrees with commit and dirty counts, non-intrusively | D-A01, D-B02, D-C03, D-C04 |
  | AGENT-02 | Wave model: `wave:` grouping and SUMMARY-based done state | D-A02, D-C10, D-C11 |
  | AGENT-03 | Adapter seam that degrades gracefully | D-A03, D-A07 |
  | AGENT-04 | Claude Code adapter: metadata and liveness | D-A04, D-A05, D-C05 to D-C08 |
  | AGENT-05 | Dashboard summary | D-C14 |
  | AGENT-06 | Detail Agents view | D-C15, D-C16 |
  | AGENT-07 | Fixed/total estimate for code-review fix runs | D-C12 |

### Claude's Discretion

- Module layout. A suggestion is `src/agents/{mod,core,waves,adapter,claude}.rs` or a `state_reader` submodule.
- Trait shape: a sync trait over plain data, where `enrich(&CoreSnapshot) -> Vec<Enrichment>` is the obvious form.
- Exact glyphs and colours, provided they reuse the conventions for glyph width (`width_of` and `Span::width`) and the rule that no meaning is carried by colour alone.
- How the plans are split into waves.

### Folded Todos

None. The todo matcher scored six todos ≥ 0.4 on generic keywords (ui, row, phase). None of them concerns agents or worktrees, so none was folded. **[inferred]**

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` § "Phase 25: Running Agents & Live Wave View": the goal, the seven verified data sources, the observed run shapes and the UI implication.
- `.planning/PROJECT.md`: the Constraints section (state reading, non-intrusive, portability).

### GSD behaviour (external, read-only)
- `~/.claude/gsd-core/agents/gsd-executor.md` (~549): where the `gsd-plan-head-before-<phase>-<plan>` ledger is written.
- `~/.claude/gsd-core/workflows/execute-phase` and `steps/*-isolation-dispatch.md`: how GSD creates agent worktrees and waves. Use them to confirm naming only; they are not a runtime data source.

### Code
- `src/state_reader/plan_waves.rs`: `plan_wave_number` and `PlanWave`. This is the wave model to reuse.
- `src/state_reader/disk_status.rs`: `leading_frontmatter_value`, and the scan that already reads the plan files.
- `src/state_reader/git_ops.rs`: `git_read_raw` (~173-220) and why `--no-optional-locks` matters, plus `working_tree_stat` (~291).
- `src/app.rs` ~1103-1170: the `Action::Tick` 5s session-poll counter and the `spawn_blocking` → `Action` idiom. The new scan rides this counter.
- `src/session_detector.rs`: the existing Claude and Codex session detection (pgrep, `/proc`, `$CODEX_HOME` handling). It is not reused for per-agent liveness (D-A05), but it shows the portability conventions.
- `src/ui/screens/normal.rs`: `dashboard_columns` (~332-370), and `alias_badge` (~179) with its single-badge rule.
- `src/ui/screens/detail.rs`: `render_sessions_tab` (~4911), `DetailSubView::Sessions` (index 5), and the Docs `Files | Milestones` sub-view pattern. **Line numbers drift; re-read them at execution time.**
- `src/app.rs:16`: the `DetailSubView` enum.
- `src/text.rs`: `Untrusted`. `src/ui/screens/render_escape_guard.rs`: coverage for every sub-view.

### Real test data (live inspection)
- `/home/blk/projects/python/ttbook`: three observed runs (1 executor; 3 code-fixers; 13 executors in wave 2 of 11).
- `~/.claude/projects/-home-blk-projects-rust-gsd-meta-manager/*/subagents/*.meta.json`: real meta samples, including executors with `worktreePath` and `worktreeBranch` from Phase 21.

### Codebase maps
- `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/CONVENTIONS.md`, `.planning/codebase/TESTING.md`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `plan_waves.rs`: the wave grouping and the `w?` unknown bucket.
- `disk_status.rs`: `leading_frontmatter_value`, and SUMMARY/PLAN discovery.
- `git_ops::git_read_raw`: a read-only git wrapper that already passes `--no-optional-locks`.
- The Docs sub-view mechanism is the template for `Sessions | Agents`.
- `Untrusted`, `width_of` and `Span::width` handle safe, width-correct rendering.

### Established Patterns
- **Polling:** one shared tick counter, `spawn_blocking`, and results returned as an `Action` message. A second timer is forbidden.
- **Watching:** `notify-debouncer-full` covers `.planning/` only (`src/watcher.rs`). Nothing outside `.planning/` is watched.
- Failure is treated as data: readers return `Option` or defaults and never propagate errors into the render loop.
- Per-project view state is in memory only (`detail_sub_view_per_project`).

### Integration Points
- A new `Action` variant is added and handled in `App::update`, with a per-alias cache in `AppContext`.
- The dashboard Status cell rendering (`normal.rs`).
- The Sessions tab render and its key handling (`detail.rs`), plus `render_escape_guard`.

</code_context>

<specifics>
## Specific Ideas

- The dashboard summary shape comes from ROADMAP: `P13 · wave 2/11 · 13 running · 8/35 done`. A compact form at narrow widths is fine (D-C14).
- The detail list's columns come from ROADMAP: plan/description, commits, dirty count, last activity. A per-wave done/running/queued summary sits above the list.
- The three observed shapes must all render sensibly:
  - a single executor in a single-plan wave;
  - 3 `gsd-code-fixer` agents in parallel after review;
  - 13 executors in parallel (wave 2 of 11).

</specifics>

<deferred>
## Deferred Ideas

- **Codex enrichment adapter (DEFERRED, pending a user answer; not verified against a live Codex GSD run).** Design notes:
  - **Worktrees and branches:** under Codex, GSD itself creates the worktrees with `gsd_run query worktree.create`. The AGENT_ID is `agent-p{plan}-{unix_ts}` and the branch is `worktree-agent-p{plan}-{ts}`. The core already recovers the plan id from the branch (D-A06).
  - **Processes:** executors are separate `codex exec` processes.
  - **Session files:** sessions are stored at `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (honour `$CODEX_HOME`).
  - **Matching:** in a session file, the first line's `payload.cwd` is the working directory, and `payload.originator == "codex_exec"` identifies an executor. Match the cwd to the worktree path; the file's mtime gives liveness.
  - **Code-review fixers** on Codex do not use worktrees, so they could only be matched by cwd equal to the project root.
  - **Scope:** under D-A07, adding this adapter is one new module plus a registration line.
- **Actions on agents:** kill, prune an orphaned worktree, open a terminal in a worktree. These write, so they are a separate phase.
- **Watching `~/.claude` or worktrees with `notify`** for liveness below 5s: rejected for now (D-C01).
- **Adapters for other runtimes** (Gemini, OpenCode, …).

### Reviewed Todos (not folded)
- `2026-07-29-driver-tab-layout-at-medium-terminal-heights.md`: layout of the Driver tab; unrelated.
- `2026-07-29-invalidate-browser-cache-after-editor-exit.md`: the Docs cache; unrelated.
- `2026-09-23-add-a-global-settings-editor-with-unambiguous-scope.md`: the Cfg tab; a separate phase.
- `2026-08-19-verify-work-gate-policy-configurable.md`: belongs to Phase 23.
- `2026-09-22-codex-runtime-remainder-after-mvp.md`: the Codex driver runtime, not agent observation. It is relevant background if the Codex adapter is pulled in.
- `2026-07-29-badge-glyph-display-width-alignment.md`: not folded. New glyphs must still be measured with `width_of` or `Span::width`, never `str::len`.

</deferred>

---

*Phase: 25-running-agents-live-wave-view*
*Context gathered: 2026-09-25*
