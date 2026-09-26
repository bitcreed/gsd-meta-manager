# Phase 25: Running Agents & Live Wave View - Research

**Researched:** 2026-09-25
**Domain:** Read-only observation of GSD agent worktrees (git), Claude Code subagent metadata (undocumented on-disk format), PLAN/SUMMARY wave model, ratatui dashboard/detail rendering
**Confidence:** HIGH for the codebase seams and git behaviour; MEDIUM for Claude Code's on-disk format (undocumented, verified against the installed 2.1.283 binary and a live 13-agent run, but it can change without notice)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### A. Architecture (LOCKED, user-supplied)

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

#### B. Project constraints (LOCKED, user-supplied)

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

#### C. Agent-inferred detail (audit these)

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

### Deferred Ideas (OUT OF SCOPE)

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
</user_constraints>

<phase_requirements>
## Phase Requirements

The IDs are the ones CONTEXT.md D-C18 proposes. They are adopted here; the planner must add them to REQUIREMENTS.md (new `### Agent Observation (AGENT)` section plus traceability rows) and replace ROADMAP Phase 25's `**Requirements**: TBD` line. **[inferred]**

| ID | Description | Research Support |
|----|-------------|------------------|
| AGENT-01 | Enumerate agent worktrees with commit and dirty counts, non-intrusively | §Pattern 1 (porcelain `-z` grammar, measured), §Pattern 2 (git helpers through `git_read_raw` + env), Pitfall 1 (spawn allowlist), Pitfall 8 (non-intrusion proof test), 170ms measured cost for 13 worktrees |
| AGENT-02 | Wave model: `wave:` grouping and SUMMARY-based done state | §Pattern 5 (reuse `DiskInference.plan_waves`, add `summarized_plans`, expose `plan_index`), Pitfall 3 (SUMMARY lives in the worktree until merge), attribution tiers measured over 690 executor metas |
| AGENT-03 | Adapter seam that degrades gracefully | §Pattern 3 (trait returns facts, core classifies; `catch_unwind` per adapter), D-A07 proof test with a test-only adapter |
| AGENT-04 | Claude Code adapter: metadata and liveness | §Pattern 4 (encoding verified in the Claude binary incl. the 200-char hash suffix), Pitfall 2 (`subagents/` dir mtime does NOT advance on append), Pitfall 4 (worktree lock release = agent finished) |
| AGENT-05 | Dashboard summary | §Pattern 6: the Status column is **13 cells at every width up to 120** (measured), so the width ladder must fit 13 |
| AGENT-06 | Detail Agents view | §Pattern 7 (`DetailSubView::Agents` sharing index 5, `m` toggle, `docs_sub_tab_strip` pattern, escape-guard obligations) |
| AGENT-07 | Fixed/total estimate for code-review fix runs | §Pattern 8 (REVIEW.md `findings:` nested frontmatter via the existing nested reader, `fix(NN): WR-08` subjects verified in ttbook, REVIEW-FIX.md = run over) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading:** must not require running Claude/GSD to check status; read files or cached state. (Project Constraints)
- **Non-intrusive:** must not interfere with running GSD instances. (Project Constraints) — here this concretely means no `index.lock`, no index rewrite, no ref writes in a live agent worktree.
- **Portability:** works for any GSD user; nothing hard-coded to one user's setup.
- **Stack:** Rust (MSRV `rust-version = "1.88"`, edition 2021), ratatui 0.30 + crossterm 0.29, tokio. `std::sync::Mutex` must not be held across `.await`; logs go to a file via tracing, never stdout.
- **Pattern:** `tokio::select!` loop, never block the render loop, all I/O through channels; state mutated only by message handlers.
- **GSD workflow enforcement:** repo edits only inside a GSD command (execute-phase for this phase).
- **Tests:** `rtk proxy cargo test --no-fail-fast` for any check that depends on raw output (rtk strips `test result:` lines; user memory: a fail-fast run can hide the envelope suites).
- **User global rules (relevant to execution):** executors are dispatched with `isolation: "worktree"` (`.planning/config.json` has `"use_worktrees": true`, `"parallelization": true`); never rebase `dev`.

## Summary

Every data source this phase needs was verified on disk today, including against a **live 13-executor wave-2 run in ttbook** that was in progress during research. The core is simple: `git worktree list --porcelain -z` (main worktree always first, lock reason on a `locked <reason>` line), then per agent worktree `rev-list --count <mainHEAD>..HEAD` and `status --porcelain`, which together took **~170 ms for 13 worktrees**. The Claude Code adapter reads `<config>/projects/<encoded>/<session>/subagents/agent-<id>.meta.json` and stats the sibling `.jsonl`. All 400 sampled metas had a sibling transcript. The encoding is `path.replace(/[^a-zA-Z0-9]/g,"-")` with a **200-char cap plus a `-<base36 hash>` suffix**; both were extracted from the installed Claude Code 2.1.283 binary.

The research overturns or sharpens four CONTEXT assumptions, and the planner must fold these in:

1. **D-C09's prefilter is wrong.** The `subagents/` directory mtime does NOT move when a transcript is appended; it only moves when an agent is spawned. Measured on this very session: the dir was ~190 s stale while the transcript was 0 s old. Filtering sessions by `subagents/` mtime within the 10-min idle window would hide every long-running agent. Stat all `subagents/*.jsonl` of the project instead; it is ~10 ms for 329 files.
2. **Claude Code releases the worktree lock when the agent finishes, and the worktree stays until the orchestrator merges.** Across all 13 live agents, `unlocked` ⇔ the plan-completion commit is present (13/13). Without a `finished` state, every early finisher in a long wave would decay to `idle` and then `stalled` while it waits for the merge.
3. **SUMMARY.md lives in the agent's worktree until merge.** At the time of the check, main had 8 SUMMARYs and each finished worktree had 9. "Done" from main alone lags by one wave; the wave model needs a `finished (unmerged)` plan state.
4. **The dashboard Status column is exactly 13 cells at every terminal width ≤ 120.** This was measured with ratatui 0.30's own layout solver. The ROADMAP summary `P13 · w2/11 · 13 run · 8/35 done` is 32 cells and would only fit at ~200 columns, so the width ladder must have a ≤13-cell form as its common case.

Two codebase guards shape the plan structure. `tests/spawn_seam_guard.rs` fails if any file outside its allowlist contains `Command::new(`, and that includes in-source test helpers. So all new git calls must live in `src/state_reader/git_ops.rs`, and git-fixture tests belong in `tests/*.rs`. `render_escape_guard` enumerates every `DetailSubView`, so adding an `Agents` variant forces fixture rows there.

**Primary recommendation:** Build a new top-level `src/agents/` module with these files:
- `worktrees.rs`: the core;
- `adapter.rs`: the trait and registry;
- `claude.rs`: the Claude Code adapter;
- `waves.rs`: a pure wave and summary model.

Put every git call behind new `git_ops` helpers that route through `git_read_raw`, with `GIT_OPTIONAL_LOCKS=0` added there. Adapters return *facts*: last activity, lock released, ended flags, text. The core classifies liveness with the named thresholds. Cache the result as an `AppContext` sibling map that is replaced wholesale each scan. Add no new crates.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Worktree enumeration, commit/dirty counts | Storage read layer (`src/agents/worktrees.rs` + `git_ops`) | — | Runtime-agnostic git facts; spawn sites must stay in `git_ops.rs` (allowlist) |
| Agent metadata + liveness facts | Adapter layer (`src/agents/claude.rs`) | — | Runtime-specific, undocumented format; isolated so D-A07 holds |
| Liveness classification (live/idle/stalled/finished/ended/unknown) | Core (`src/agents/mod.rs`) | — | Thresholds must be named constants "in one place" (D-C08); adapters report facts only |
| Plan attribution + wave/plan state | Pure model (`src/agents/waves.rs`) | `state_reader::disk_status` (plan list, summaries, waves) | Needs `ProjectState` wave data already parsed; no second frontmatter reader (D-C11) |
| Scan scheduling | App (`App::update` / `Action::Tick`) | tokio `spawn_blocking` | Existing 20-tick counter; no second timer |
| Cache | `AppContext` sibling map | — | Must NOT live on `ProjectState` (its `PartialEq` drives "Updated" suppression) |
| Dashboard summary | UI (`normal.rs` Status cell) | — | Render-only; string built from the cached model |
| Agents sub-view | UI (`detail.rs`) | `render_escape_guard.rs` | Sub-view pattern of Docs `Files | Milestones` |

## Standard Stack

### Core (all already in `Cargo.toml`; **no new dependencies**)

| Library | Version (Cargo.toml) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde / serde_json | `"1"` | Tolerant `meta.json` deserialization (`Option<T>` fields, no `deny_unknown_fields`) | Already used; `deny_unknown_fields` is banned tree-wide by `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection` [VERIFIED: tests/spawn_seam_guard.rs:675] |
| regex | `"1"` | Description attribution, branch pattern, finding ids | Already used (`roadmap_md.rs`); `std::sync::LazyLock` for statics (MSRV 1.88 ≥ 1.80) |
| dirs | `"7"` | `home_dir()` for the `~/.claude` default | Existing convention (`project_creator.rs`, `queue_md.rs`, `journal/redact.rs`) |
| ratatui | `0.30` | `Layout`/`Flex` to derive the Status column width; `List`/`ListState` for the agents list | Existing |
| tempfile / assert_fs | `"3"` / `"1"` | Fixtures | Existing |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `git worktree list --porcelain -z` | Reading `.git/worktrees/*/{gitdir,HEAD,locked}` directly | Faster, no subprocess, but reimplements git's layout (packed refs, `commondir`, `$GIT_DIR` indirection). Use git; read `.git/worktrees/<name>/` only for the optional ledger stat. |
| `notify` on `~/.claude` | — | Rejected by D-C01 |
| `git2` crate | — | New C dependency; subprocess git is the codebase convention |

**Installation:** none.

## Package Legitimacy Audit

This phase installs **no external packages**; every library it uses is already locked in `Cargo.lock`. `gsd-tools package-legitimacy check` was therefore not needed.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none new) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Action::Tick (250ms) ──► session_poll_counter hits 20 (~5s)
        │                      │ (existing SessionsDetected + RunsReconciled tasks)
        │                      ▼
        │         if !agents_scan_in_flight: set flag; spawn_blocking(move || …)
        │                      │  inputs: registered {alias→path}, claude_root (resolved once), now
        │                      ▼
        │   for each registered project (catch_unwind per project):
        │     ┌──────────────── CORE (runtime-agnostic) ────────────────┐
        │     │ git worktree list --porcelain -z  ──► [main, wt1, wt2…] │
        │     │   base = main.HEAD                                      │
        │     │   agent worktree? (.claude/worktrees/ | branch pattern) │
        │     │   per agent wt: rev-list --count base..HEAD  (or ?)     │
        │     │                 status --porcelain → dirty (or ?)       │
        │     │                 parse branch agent-p{plan}-{ts} (D-A06) │
        │     │                 stat ledger gsd-plan-head-before-*      │
        │     └────────────────────────┬────────────────────────────────┘
        │                              ▼ CoreSnapshot
        │     ┌──────── ADAPTERS (registry order; catch_unwind each) ────┐
        │     │ ClaudeCodeAdapter: <root>/projects/<enc(path)>/*/subagents│
        │     │   by id: agent-<id>.meta.json  → type, description, flags │
        │     │   stat agent-<id>.jsonl        → last_activity            │
        │     │   worktree `locked` absent     → lock_released            │
        │     │   live worktree-less metas     → extra agents             │
        │     └────────────────────────┬─────────────────────────────────┘
        │                              ▼ Enrichments (facts only)
        │           classify liveness (named thresholds, one place)
        │                              ▼ ProjectAgents (rows, no wave math yet)
        └──◄ tx.send(Action::AgentsScanned { per_project })
                               ▼
App::update: clear in-flight flag; for each alias:
   AgentView = waves::derive(&ProjectAgents, &ProjectState)   (pure: attribution,
               plan states, current wave, summary strings, fixer estimate)
   ctx.agent_views = new map  (replaced wholesale → no prune needed)
                               ▼
render: normal.rs Status cell ← AgentView.summary(width ladder)  |  detail.rs Agents sub-view
```

### Recommended Project Structure

```
src/agents/
├── mod.rs        # pub types (AgentRow, Liveness, ProjectAgents, AgentView), thresholds,
│                 # scan_projects(projects, &adapters, now) — the spawn_blocking entry
├── worktrees.rs  # CORE: parse_porcelain_z(&str), is_agent_worktree, parse_agent_branch,
│                 # scan_worktrees(project_root) using git_ops helpers; ledger + worktree-SUMMARY stat
├── adapter.rs    # trait AgentAdapter, Enrichment, AdapterReport, registered_adapters()
├── claude.rs     # ClaudeCodeAdapter { config_root }, encode_project_dir, tolerant Meta
├── waves.rs      # pure: attribute(), plan_states(), current_wave(), summary ladder
└── fixers.rs     # AGENT-07 (last plan): review total + fixed-id estimate
src/state_reader/git_ops.rs   # + worktree_list_porcelain, commits_ahead, dirty_count, log_subjects
tests/agents_scan.rs          # real git worktrees + fake claude root (integration; not spawn-audited)
```

It is a new **top-level** module rather than a `state_reader` submodule **[inferred]**. The result must stay off `ProjectState` (sibling-map rule, `src/ui/screens/mod.rs` AppContext docs), and it has its own cadence, which is not the `.planning/` watcher's. Register it in `src/lib.rs` as `pub mod agents;`, since integration tests use `gsd_meta_manager::agents::…`.

### Pattern 1: Porcelain parsing — pure function over git's `-z` output

**What:** Records are NUL-terminated lines, and a worktree block ends with an empty record. The first block is always the main worktree. Measured line kinds [VERIFIED: probe in scratch repo, git 2.53.0]:

```
worktree <abs path>          (paths with spaces arrive verbatim under -z)
HEAD <sha>
branch refs/heads/<name>     | detached
locked                       | locked <reason>        e.g. "locked claude agent agent-aaa (pid 1 start 2)"
prunable <reason>            e.g. "prunable gitdir file points to non-existent location"
bare                         (main only, bare repos)
```

**Rules:**
- Unknown keys are ignored.
- A `prunable` worktree is kept as a row with `?` counts (D-C16) and never pruned (D-B02).
- `-z` needs git ≥ 2.36. On a non-zero exit, retry without `-z` and parse newline records. Without `-z` git quotes unusual paths C-style, so strip the surrounding `"` and unescape `\\`, `\"`, `\t`, `\n`, or leave such a path as a best-effort raw string. **[inferred]**

The agent-worktree predicate follows D-C03, widened to GSD's own branch regex. GSD's `bin/lib/worktree-safety.cjs:26` declares `const WORKTREE_AGENT_BRANCH_RE = /^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$/;` [VERIFIED: ~/.claude/gsd-core/bin/lib/worktree-safety.cjs:26, GSD 1.14.0]. Match `agent-*`, `worktree-agent-*` and `worktree-wf_*` branches, plus any path under `<project>/.claude/worktrees/`. **[inferred widening]**

### Pattern 2: Git helpers — every call through `git_read_raw`

`git_read_raw` is the one wrapper [VERIFIED: src/state_reader/git_ops.rs:198-212]:

```rust
pub(crate) fn git_read_raw(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
```

- Add `.env("GIT_OPTIONAL_LOCKS", "0")` **inside `git_read_raw` itself**. It is idempotent with the flag, and the envelope's readers inherit the same guarantee.
- Add new `pub(crate)` helpers in `git_ops.rs`, not in `src/agents/`:
  - `worktree_list_porcelain(root) -> Option<String>`: `["worktree","list","--porcelain","-z"]`, with the non-`-z` retry;
  - `commits_ahead(worktree, base_sha) -> Option<u32>`: `["rev-list","--count", &format!("{base}..HEAD")]`;
  - `dirty_count(worktree) -> Option<u32>`: `["status","--porcelain"]`, counting non-empty lines;
  - `log_subjects(dir, range, max) -> Vec<String>`: `["log","--format=%s", &format!("-n{max}"), range]`, for AGENT-07.
- Validate `base_sha` as `[0-9a-f]{40}` or `{64}` before formatting it into an argument.
- Never put a branch name into argv. Run in the worktree and use `HEAD`, so a hostile branch named `--output=x` cannot become an option.

### Pattern 3: Adapter seam — facts in, classification in the core

```rust
// src/agents/adapter.rs  [inferred shape; names are recommendations]
pub struct CoreWorktree {            // what the core found; read-only to adapters
    pub path: PathBuf,               // as git reported it
    pub branch: Option<Untrusted>,   // short name
    pub locked: Option<Option<Untrusted>>, // None = not locked; Some(None) = locked w/o reason
    pub agent_id: Option<String>,    // from path/branch, validated [A-Za-z0-9_.-]{1,64}
    pub branch_plan: Option<BranchPlan>, // D-A06: { plan: String, spawned_unix: u64 }
}
pub struct CoreSnapshot<'a> { pub project_root: &'a Path, pub main_worktree: &'a Path,
                              pub worktrees: &'a [CoreWorktree], pub now: SystemTime }

#[derive(Default)]
pub struct Enrichment {              // FACTS only — no Liveness here
    pub agent_type: Option<Untrusted>,
    pub description: Option<Untrusted>,
    pub last_activity: Option<SystemTime>,
    pub ended: bool,                 // worktreeCleanlyRemoved | stoppedByUser
    pub lock_released: Option<bool>, // runtime's own lock semantics (Claude: Some(locked.is_none()))
    pub children: Vec<ChildAgent>,   // inheritedWorktreePath agents
}
pub struct AdapterReport {
    pub per_worktree: Vec<(usize /*index into worktrees*/, Enrichment)>,
    pub worktreeless: Vec<ChildAgent>, // live subagents without a worktree (D-C07)
}
pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport; // never panics by contract…
}
pub fn registered_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(crate::agents::claude::ClaudeCodeAdapter::from_env()),
        // Codex (deferred, D-A08): one line here + src/agents/codex.rs
    ]
}
```

- **…and the core enforces it anyway.** Wrap each `enrich` call in `std::panic::catch_unwind(AssertUnwindSafe(..))`. A panic becomes an empty report plus a `tracing::warn!`. There is no `[profile]` in `Cargo.toml`, so the default `panic = "unwind"` applies and `catch_unwind` works [VERIFIED: Cargo.toml has no `[profile` section].
- Wrap the per-project scan as well. A panicking `spawn_blocking` closure sends nothing, and the in-flight flag would then stay set forever.
- The first adapter to claim a worktree wins, in registration order. **[inferred]**
- **D-A07 proof:** a `#[cfg(test)]` adapter defined in a test module, fed through `scan_project_with(root, &[Box::new(Fake)], now)`, must show its `agent_type` on the row with zero edits to core, waves or UI. Keep `scan_project_with` public so the Codex plan can reuse the test shape.

### Pattern 4: Claude Code adapter

**Root:** `$CLAUDE_CONFIG_DIR` if it is set and non-empty, else `dirs::home_dir()?/.claude`. Resolve it once, in `ClaudeCodeAdapter::from_env()`. Tests call `ClaudeCodeAdapter::new(root)`. The Claude binary resolves `projects` as `join(configHome,"projects")` and refuses a non-absolute `CLAUDE_CONFIG_DIR`. If it is relative, ignore it and fall back. [CITED: Claude Code 2.1.283 binary strings: `function zu(){return S(Se(),"projects")}`, message "the configuration home (CLAUDE_CONFIG_DIR) is not an absolute path"]

**Directory encoding** [VERIFIED: extracted from the installed Claude Code 2.1.283 binary]:

```js
function k(e){return e.replace(/[^a-zA-Z0-9]/g,"-")}
function qx(e){let n=k(e);if(n.length<=Kne)return n;return`${n.slice(0,Kne)}-${Le(e)}`}   // Kne=200
function Le(e){return Math.abs(KJ(e)).toString(36)}
function KJ(t){let e=0;for(let n=0;n<t.length;n++)e=(e<<5)-e+t.charCodeAt(n)|0;return e}
```

Rust notes:
- The replacement is **ASCII-only**, so use `is_ascii_alphanumeric`, not `char::is_alphanumeric`.
- JS iterates **UTF-16 code units**, so a non-BMP char yields **two** dashes: emit `"-".repeat(c.len_utf16())`. `length`/`slice(0,200)` also count UTF-16 units; the encoded string is pure ASCII, so slicing bytes is fine after encoding.
- For `KJ`, use `i32` wrapping arithmetic over `encode_utf16()`, then `(h as i64).abs()` (`Math.abs(-2^31)` = 2147483648), then base-36 lowercase.
- Candidates, in order: `enc(registered path)`, `enc(canonicalize(registered))`, `enc(main worktree path from porcelain)`.
- If a >200 encoding misses, fall back to a unique prefix match on `slice(0,200) + "-"`.
- Claude files sessions under the *canonical working-copy root* (`canonicalWcRootForProject` in the same binary), which is why there are no per-worktree project dirs. This was observed: only 1 of 72 local project dirs contains `claude-worktrees`, and that one was a non-GSD scratch probe [VERIFIED: `ls ~/.claude/projects`].

**Layout** [VERIFIED: filesystem inspection]: `<root>/projects/<enc>/<session-uuid>/subagents/agent-<id>.meta.json` sits beside `agent-<id>.jsonl`. The tree is **flat**: depth-2 and depth-3 agents live in the top-level session's `subagents/` too. 400 of 400 sampled metas had the sibling `.jsonl`.

**Meta fields**, re-measured over 2475 local metas [VERIFIED: python census]:

| Field | Count |
|---|---|
| `agentType` | 2475 |
| `spawnDepth` | 2475 |
| `description` | 2474 (**not** always present; CONTEXT's "always" is off by one) |
| `toolUseId` | 2474 |
| `requestShape` | 1560 |
| `parentAgentId` | 1487 |
| `model` | 1195 |
| `spawnedWithWorktree` | 587 |
| `worktreePath` | 399 |
| `worktreeBranch` | 399 |
| `inheritedWorktreePath` | 188 |
| `worktreeCleanlyRemoved` | 21 |
| `name` | 20 |
| `stoppedByUser` | 3 |

- `worktreePath` appears at **every** depth (1: 133, 2: 257, 3: 9). Depth-2 executors spawned by a depth-1 coordinator carry `worktreePath` directly (all 13 ttbook executors are `spawnDepth: 2`). D-C06's "depth 2 ⇒ inheritedWorktreePath" is not a rule: join on whichever field is present.
- `inheritedWorktreePath` agents have `parentAgentId` equal to the worktree's own agent id, which confirms the child-attach rule.
- **Metas are rewritten.** `worktreeCleanlyRemoved: true` metas carry **no** `worktreePath`. Re-read the meta each scan, and treat a parse failure (a torn write mid-rewrite) as "no data this tick" (D-A03). Cap the read at 64 KiB. **[inferred]**

**Lookup cost:**
- Per worktree agent: for each session dir, one stat of `subagents/agent-<id>.meta.json`. 19 sessions in this project.
- Worktree-less live agents: read_dir each session's `subagents/`, stat every `*.jsonl`, and read the meta only when the mtime is within `LIVE`. Measured: 329 transcripts statted in ~10 ms [VERIFIED: `time stat`].

### Pattern 5: Wave model — reuse, add one field, expose two helpers

- `DiskInference.plan_waves: Vec<PlanWave>` already exists per phase in `ProjectState.phase_disk_statuses`. The field is documented as `/// Per-phase disk inference keyed by phase number (e.g., "01", "05")` [VERIFIED: src/state_reader/mod.rs:94-95]. Look up with `phase_num::same_phase`, never by string equality.
- **Add** `pub summarized_plans: Vec<String>` (sorted plan stems whose summary paired) to `DiskInference`, filled in the existing "Pass 2 (pairing)" loop. That costs zero extra I/O and needs no second reader (D-C11). It must be sorted, because `DiskInference`'s `PartialEq` drives the dashboard's unchanged-state suppression (see `plan_tokens`' doc).
- **Expose** `disk_status::plan_index` as `pub(crate)`. It is currently private at `src/state_reader/disk_status.rs:399`. It maps both `13-13` (from a description) and `13-13-slug` (a plan stem) to `(PhaseNum, u32)`, the only safe join key (`13-1` ≡ `13-01`).
- **Expose** `leading_frontmatter_nested_value` as `pub(crate)` for AGENT-07.

**Attribution tiers**, measured over 690 local `gsd-executor` descriptions [VERIFIED: python census]:

| Tier | Pattern | Matches |
|---|---|---|
| 1 | `\bplan (\S+) of phase (\S+)` (GSD's own template: `description="Execute plan {plan_number} of phase {phase_number}"` [VERIFIED: ~/.claude/gsd-core/workflows/execute-phase.md:634]) | 355 |
| 2 (**add**) | `\bplans? (\d+(?:\.\d+)?-\d+)\b` (e.g. "Execute plan 04-06", "Close out plan 04-03") | 132 |
| none | quick tasks (`Execute 260923-e9d: …`, `Execute: …`) | 203 |

- For unattributed rows, show the description verbatim.
- Tier 1's plan capture is either `13-13` or a bare `22`, since the orchestrator fills the template inconsistently. Bare means `<phase>-<nn>`.
- Tier 3 is the D-A06 branch grammar `^(?:worktree-)?agent-p(?P<plan>[0-9][0-9A-Za-z.]*(?:-[0-9]+)?)-(?P<ts>[0-9]{9,11})$`. It must not match Claude's `agent-a<16 hex>` ids, which start with `a`. It is also bare-or-dashed, because the dispatch step substitutes `{plan_number}` as the LLM chose. The step itself reads `AGENT_ID="agent-p{plan_number}-$(date -u +%s)"` [VERIFIED: ~/.claude/gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md:290].
- Tier 4, the ledger, is confirmation only. It was **absent on 9 of 13 live agents, including agents with 4–5 commits**, so it is not reliably "written before the first commit".
- **Tier 2b (optional, strong):** worktree commit subjects carry the scope, e.g. `feat(13-17): …` and `test(13-17): …`. It is available from the same `log_subjects` helper if tiers 1–3 fail. **[inferred]**
- Validate every captured id against `^[0-9]+(\.[0-9]+)?(-[0-9]+)?$` before it becomes a join key or display text.

**Plan states:** CONTEXT's list amended with `finished` **[inferred — required by Pitfall 3/4]**:

| State | Rule |
|---|---|
| `done` | SUMMARY paired in **main** (`summarized_plans`) |
| `finished` | not done in main, but `<worktree>/.planning/phases/<same phase dir name>/<NN-MM>-SUMMARY.md` exists (one stat), **or** its agent is `finished` (lock released) |
| `running` | a live or idle agent is attributed |
| `stalled` | the only attributed agent is stalled |
| `queued` | otherwise |

- The "done" count shown to users is `done + finished`, with `finished` also shown separately in the wave row (`done 8 (+2 unmerged)`).
- **Current wave** = the lowest numbered wave with a plan not `done` in main.
- The denominator of `w2/11` = the max numbered wave; the `w?` bucket is excluded.
- Plan total = `DiskInference.plan_count`, which already excludes superseded plans; ttbook phase 13 has 35.

**Liveness states:** CONTEXT's table amended **[inferred]**. The constants live in one `const` block in `src/agents/mod.rs`:

| State | Rule |
|---|---|
| `ended` | meta `worktreeCleanlyRemoved` or `stoppedByUser` |
| `finished` | `lock_released == Some(true)` **and** (worktree SUMMARY present **or** transcript age > `LIVE_SECS`). The second clause protects against a spawn-time window before the lock is taken, and against a Claude version that never locks. |
| `live` | age ≤ `LIVE_SECS` = 120 |
| `idle` | age ≤ `IDLE_SECS` = 600 |
| `stalled` | older, worktree present, not finished |
| `unknown` | no adapter data |

Clamp a future mtime (clock skew) to age 0.

### Pattern 6: Dashboard Status cell — a ladder that fits 13 cells

The Status column width was measured with ratatui 0.30's `Layout::horizontal(widths).flex(Flex::Start).spacing(1)`. That is exactly what `Table::get_column_widths` does (`ratatui-widgets-0.3.2/src/table.rs:1041-1065`, defaults `column_spacing: 1`, `flex: Flex::Start`). Inputs were `dashboard_columns` and the 2-cell `"> "` selection [VERIFIED: scratch program against ratatui 0.30.0]:

| terminal width | 60 | 70 | 80 | 100 | 120 | 160 | 200 |
|---|---|---|---|---|---|---|---|
| Status cells | 13 | 13 | 13 | 13 | 13 | 19 | 25 |

The floor constant is `const STATUS_COLUMN_MIN_CELLS: u16 = 13;` [VERIFIED: src/ui/screens/normal.rs:322].

- Add `fn status_column_cells(terminal_width: u16) -> u16`. It re-runs that exact `Layout` over `dashboard_columns(terminal_width).1`, the single source of truth, so the budget can never drift from the render.
- Pick the first form whose `Line::width()` fits. **[inferred forms; widths counted]**

| Form | Cells |
|---|---|
| `P13 · w2/11 · 13 run · 8/35 done` | 32 |
| `w2/11 · 13 run · 8/35 done` | 26 |
| `w2/11 · 13 run · 8/35` | 21 |
| `w2/11 · 13 run` | 14 |
| `w2/11 13run` | 11 (**the common case**) |
| `13run` | 5 |
| Fixers: `3 fixers · ~5/48` | 16 |
| Fixers: `3fix ~5/48` | 10 |
| Generic, no attribution: `2 agents` | 8 |
| Stalled-only: `2 stalled` | 9 |

- `·` is U+00B7, a single-cell glyph.
- Dropping the `P13` segment first is a deliberate deviation from "drop from the right". The Phase column beside it already shows the phase, and at 13 cells the right-drop rule would leave `P13 · w2/11` and hide the running count, which is the point of the feature. **[inferred; flag in audit]**
- Show the summary when ≥1 agent is `live`, `idle` or `finished`. If agents exist but all are stalled, use the stalled form. Otherwise leave the cell unchanged (D-C14).
- No badge change.

### Pattern 7: Agents sub-view — the Docs sub-view pattern, one index

- Add `DetailSubView::Agents`. `tab_index` maps it to `5`, which it shares with `Sessions`, the same way Docs maps `DetailSubView::Browse | DetailSubView::Archive => 7` [VERIFIED: src/ui/screens/detail.rs:1008]. `sub_view_from_index(5)` stays `Sessions`.
- `tab_index` is "Exhaustive and wildcard-free on purpose" (detail.rs:995), so the compiler names every site: 29 `Archive` mentions in `detail.rs`, 1 in `app.rs`, 2 in `render_escape_guard.rs`.
- **Toggle key: `m` [inferred].** It is the established "switch sub-tab" key. Today its arm is guarded `if matches!(current_view, DetailSubView::Browse | DetailSubView::Archive)` (detail.rs:3248-3249), so a second guarded arm for `Sessions | Agents` does not collide.
  - `n` stays bound only for `Sessions` (detail.rs:3412).
  - `j/k`, `Down/Up` and `PgUp/PgDn` scroll a new `ProjectViewCache.agents_selected: usize`, beside the existing `pub sessions_selected: usize` [VERIFIED: src/ui/screens/mod.rs:965].
  - `Tab` keeps its current behaviour: it jumps to the first matching session from any non-Sessions view.
- A strip helper mirrors `docs_sub_tab_strip`: `[Sessions] │ Agents   m switch`, bracketed and reversed so it reads in monochrome (detail.rs:6457-6484). Draw it on **both** Sessions sub-views, so `render_sessions_tab` shrinks by one row.
- Footer hints get an `Agents` arm, and Sessions gains `[m] agents`.
- The help row `row("m", "Docs tab: switch Files / Milestones (detail view)")` [VERIFIED: src/ui/screens/help.rs:244] becomes the shared `m` description. It is pinned by a test at help.rs:941, which must be updated in the same commit.
- **Body layout:**
  - the summary line (the same ladder, full width);
  - one row per wave (`w2  running 13 · done 8 (+2 unmerged) · queued 14`), with the current wave bold and marked `▸` so the meaning is not colour-only;
  - a `List` of agent rows: `state-word  plan|description  agentType  +commits  ~dirty  age`;
  - a `Worktree-less (live)` group;
  - the empty state "No running agents".
- Use ASCII state words (`live`, `idle`, `done`, `stall`, `ended`, `?`) rather than `●`/`◐`. Those are East-Asian-Width "Ambiguous" and render 2 cells in CJK locales, the class of bug in todo `2026-07-29-badge-glyph-display-width-alignment.md`. **[inferred]**
- Measure widths with `Span::width()` / `Line::width()` (existing use at normal.rs:1075, detail.rs:4181), never `str::len`.
- **Escape-guard obligations** (render_escape_guard.rs):
  - `ALL_SUB_VIEWS: [DetailSubView; 10]` becomes 11 with `Agents`;
  - `sub_view_label` gains `Agents => "Agents sub-view"`;
  - `DETAIL_TAB_ARRIVAL` gains a row with `arrives: true`;
  - `probe_ctx` populates `ctx.agent_views[identity]`, with the hostile identity as `description`, `agent_type`, branch and child description;
  - add a `NormalScreen` state with an `AgentView` present so the dashboard Status cell is probed too.

### Pattern 8 (AGENT-07): fixer estimate

- **Total:** `leading_frontmatter_nested_value(review, "findings", "total")`. The ttbook 12-REVIEW.md frontmatter reads `findings:` / `critical: 2` / `warning: 22` / `info: 24` / `total: 48` [VERIFIED: ttbook .planning/phases/12-*/12-REVIEW.md].
- **Fixed:** distinct `\b(?:CR|WR|IN)-\d+\b` found in subjects matching `^fix\((<phase>)(?:-[^)]*)?\):`. Read them via `log_subjects` on each fixer worktree over `base..HEAD`, plus the main worktree `-n 300`. Observed subjects: `fix(12): WR-15 …`, `fix(12): IN-01 …`, `fix(12-sec): …` [VERIFIED: ttbook git log].
- **Run over:** `NN-REVIEW-FIX.md` exists (ttbook: `status: all_fixed`, `fixed: 30`). Hide the estimate then.
- Phase comes from the fixer description (`Fix phase 05 review findings`, via `\bphase (\d+(?:\.\d+)?)\b`, case-insensitive), else the active phase.

### Anti-Patterns to Avoid

- **Any `Command::new(` in `src/agents/`**, even in `#[cfg(test)]`. It fails `every_process_spawn_site_in_src_is_on_the_allowlist`.
- **Putting agent state on `ProjectState`.** It floods "Updated: alias" and breaks the sibling-map rule.
- **Deriving the wave model at render time.** The codebase derives on the refresh cadence ("never at render time", plan_waves.rs:11-14). Derive in the `AgentsScanned` handler.
- **Reading transcript contents.** Stat only (D-C09).
- **`deny_unknown_fields`** anywhere. It is banned tree-wide.
- **Joining plan ids as strings.** Use `plan_index` / `PhaseNum`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Worktree discovery | Walking `.git/worktrees/` | `git worktree list --porcelain -z` | commondir/gitdir indirection, bare repos, prunable detection |
| Frontmatter reads | A new YAML/line scanner | `leading_frontmatter_value`, `leading_frontmatter_nested_value`, `plan_wave_number` | WR-05 column-zero and byte-zero anchoring defects are already fixed there |
| Plan↔summary pairing | Stem string compare | `disk_status::plan_index` + the existing Pass 2 | Slugged plan vs bare summary; `07.1` phases |
| Terminal-safe text | Ad-hoc sanitising | `Untrusted::from_untrusted_source` + `.shown()` / `render_for_terminal` | Two escape classes, Debug-safe |
| Column width | Guessing percentages | `Layout` with `dashboard_columns` constraints | Measured: 13 cells, not a percentage |
| Git wrapper | A new `Command` builder | `git_ops::git_read_raw` | Spawn allowlist + `--no-optional-locks` + failure-as-data |

**Key insight:** every hard sub-problem here (frontmatter anchoring, plan pairing, escaping, the git read discipline) already has a hardened single implementation with a doc trail of defects. The phase is mostly new wiring over them plus one undocumented-format adapter.

## Common Pitfalls

### Pitfall 1: A new spawn site fails the allowlist, including in test helpers
**What goes wrong:** `tests/spawn_seam_guard.rs` walks `src/` for `Command::new(` and allows only listed files. `src/executor/outcome.rs` is listed *only* because of its in-source test helper.
**How to avoid:** production git goes through `git_ops.rs` helpers. Tests that need real repos and worktrees live in `tests/agents_scan.rs`, which may reuse `tests/common/mod.rs::git`, because `tests/` is not walked for spawns.
**Warning signs:** `a process-spawn site appeared in a file that is not on the allowlist`.

### Pitfall 2: The `subagents/` dir mtime is a spawn clock, not an activity clock
**What goes wrong:** D-C09's prefilter would drop live long-running worktree-less agents after 10 min.
**Evidence:** this session: `subagents/` mtime 1790386422, newest transcript 1790386612 = now; the parent `<session>.jsonl` was also ~237 s stale [VERIFIED: `stat`].
**How to avoid:** stat every `subagents/*.jsonl`, which is cheap. Only as an optional coarse bound, skip session dirs whose `subagents/` mtime is older than `MAX_AGENT_AGE` = 24h **[inferred]**.

### Pitfall 3: "Done" read only from main lags a whole wave
**What goes wrong:** finished executors commit SUMMARY in their worktree; main gets it only at the wave merge. Observed: main 8 SUMMARYs, each finished worktree 9.
**How to avoid:** use the `finished` plan state (one stat per attributed agent).

### Pitfall 4: Early finishers decay to "stalled" while the wave's slowest agent runs
**What goes wrong:** the lock is released at finish (13/13 correlation on the live run), the transcript goes stale, and the worktree persists until the orchestrator's `git worktree unlock`/`remove` in cleanup [CITED: ~/.claude/gsd-core/workflows/execute-phase.md:863-864].
**How to avoid:** the `finished` liveness state from `lock_released`. Pid in the lock reason is still ignored (D-A05).
**Confidence:** MEDIUM. Observed on one run of Claude Code 2.1.283; the second clause of the rule protects against a version that never locks.

### Pitfall 5: The summary never fits
**What goes wrong:** a 32-cell summary in a 13-cell column gets clipped mid-segment.
**How to avoid:** the Pattern 6 ladder plus a `TestBackend` render test at widths 60/80/120/160/200 asserting the rendered Status cell equals a whole ladder form.

### Pitfall 6: Stuck in-flight flag or overlapping scans
**What goes wrong:** a scan that runs longer than 5 s (network FS, huge repo) gets spawned again every tick; a panic inside `spawn_blocking` never sends.
**How to avoid:** `AppContext.agents_scan_in_flight: bool`, set before the spawn and cleared in the `AgentsScanned` handler. The closure body is wrapped in `catch_unwind` so it **always** sends, possibly with an empty map. **[inferred]**

### Pitfall 7: Path identity mismatches
**What goes wrong:** `worktreePath` in the meta vs the porcelain path vs the registered path can differ by symlinks (e.g. `/home` → `/var/home`) or a trailing slash.
**How to avoid:** compare raw first, then `canonicalize()` both sides. Use the agent-id fallback (D-C06).

### Pitfall 8: Non-intrusion is claimed but never proven
**How to avoid:** in `tests/agents_scan.rs`:
1. Make a tracked file stat-dirty in an agent worktree (rewrite the same bytes, bump its mtime with `File::set_modified`).
2. Record the worktree's `index` file (`git rev-parse --git-path index`) bytes and mtime.
3. Run the scan.
4. Assert that the index is unchanged and that no `index.lock` exists.

A plain `git status` would refresh (rewrite) that index; `--no-optional-locks` must not. The test would go red if the flag or env were removed from `git_read_raw`, so it is the committed control for D-B02.

### Pitfall 9: Edition 2021
No `if let … && let …` chains (they are an edition-2024 feature); use nested `if let` / `let … else`.

### Pitfall 10: Tests touching the real home
**How to avoid:** `ClaudeCodeAdapter::new(root)` in every test, with the clock injected as `now: SystemTime`. Set transcript mtimes with `std::fs::File::set_modified` (stable since 1.75) rather than sleeping. Never call `from_env()` in a test.

## Code Examples

### Tolerant meta model
```rust
// src/agents/claude.rs — every field optional, unknown fields ignored (serde default)
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Meta {
    agent_type: Option<String>,
    description: Option<String>,
    worktree_path: Option<PathBuf>,
    inherited_worktree_path: Option<PathBuf>,
    parent_agent_id: Option<String>,
    spawn_depth: Option<u32>,
    worktree_cleanly_removed: Option<bool>,
    stopped_by_user: Option<bool>,
}
// Parse: read ≤64 KiB; serde_json::from_slice::<Meta>(..).ok() — None degrades (D-A03).
// Wrap text immediately: Untrusted::from_untrusted_source(s).
```

### Claude project-dir encoding (verified algorithm)
```rust
fn encode_project_dir(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c.is_ascii_alphanumeric() { out.push(c) } else { out.extend(std::iter::repeat_n('-', c.len_utf16())) }
    }
    if out.len() <= 200 { return out; }                       // ASCII ⇒ len == UTF-16 length
    let mut h: i32 = 0;
    for u in path.encode_utf16() { h = (h << 5).wrapping_sub(h).wrapping_add(u as i32); }
    format!("{}-{}", &out[..200], to_base36((h as i64).unsigned_abs()))
}
```
`std::iter::repeat_n` is stable since Rust 1.82, which is within MSRV 1.88.

### Tick wiring (inside the existing 20-tick block, beside `SessionsDetected`)
```rust
if !self.ctx.agents_scan_in_flight {
    if let Some(ref tx) = self.ctx.event_tx {
        self.ctx.agents_scan_in_flight = true;
        let tx = tx.clone();
        let projects: Vec<(String, PathBuf)> =
            self.ctx.config.projects.iter().map(|(a, p)| (a.clone(), p.path.clone())).collect();
        tokio::task::spawn_blocking(move || {
            let per_project = crate::agents::scan_projects_guarded(&projects, std::time::SystemTime::now());
            let _ = tx.send(Action::AgentsScanned { per_project });
        });
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `WAVE_WORKTREE_MANIFEST` / `waves.json` | `wave:` frontmatter | D-A09/D-A10 | Frontmatter is the only reliable source |
| GSD branch shapes `worktree-agent-*` only | `agent-*`, `worktree-agent-*`, `worktree-wf_*` | GSD #3021 (`worktree-safety.cjs:23-26`) | Widen the predicate |
| Ledger as a first-commit marker | Ledger present on only 4/13 live agents | observed 2026-09-25 | Confirmation only, never required |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Claude Code releases the worktree lock exactly when an agent finishes (13/13 on one run, v2.1.283) | Pattern 5, Pitfall 4 | A finished agent shows `live`/`idle` or a running one `finished`; mitigated by the two-clause rule |
| A2 | Encoding/hash algorithm stays as extracted from 2.1.283 | Pattern 4 | Adapter finds no sessions and rows degrade to "(no agent metadata)" (D-A03); prefix fallback covers hash drift |
| A3 | Dropping `P13` before other segments is acceptable despite D-C14's "drop from the right" | Pattern 6 | UX preference only |
| A4 | `m` as the Sessions/Agents toggle | Pattern 7 | Key-binding preference only |
| A5 | `MAX_AGENT_AGE` 24h coarse bound; 64 KiB meta cap | Pitfall 2, Pattern 4 | An agent running >24h is hidden from the worktree-less group only |
| A6 | Done count = main-done + finished-unmerged | Pattern 5 | Display semantics only |
| A7 | `findings.total` includes `info` findings fixers often skip (ttbook: 30 fixed of 48 total, `fix_scope: critical_warning plus selected info`) | Pattern 8 | Estimate reads low (`~30/48`) though the run is `all_fixed`; consider `critical+warning` as the denominator |
| A8 | Base = main worktree HEAD (D-C04) overcounts when a lane orchestrator runs in a non-main worktree (GSD #630) | Pattern 2 | Commit counts inflated in that rare mode; acceptable for v1 |

## Open Questions

1. **The AGENT-07 denominator** (A7). What we know: ttbook's run fixed 30, and `findings.total` is 48. Recommendation: show `~fixed/(critical+warning)` when the fixed count ≤ that, else `~fixed/total`; the planner may keep D-C12's `total` verbatim and mark it for audit.
2. **Codex adapter** (D-A08, pending user answer). The trait above takes `CoreSnapshot` (paths + branch plan) and returns per-worktree enrichment plus worktree-less agents. That is exactly the shape a cwd-matching Codex reader needs, so D-A07 holds without research beyond this note.
3. **Git < 2.36 users** lack `-z`. The retry path is recommended; no environment to test against locally (git 2.53).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | Core scan, tests | ✓ | 2.53.0 local (CI runner 2.55.0 per memory) | Non-`-z` retry for < 2.36 |
| Rust toolchain | Build | ✓ | rustc 1.98.1 (MSRV floor 1.88) | — |
| rtk | Raw test output | ✓ | 0.48.0 | `rtk proxy` |
| Claude Code (for manual smoke only) | Live observation | ✓ | 2.1.283 | Tests never need it |
| `~/.claude/projects` | Manual smoke only | ✓ | 72 project dirs | Tests use a tempdir root |

**Missing dependencies with no fallback:** none.

## Validation Architecture

`workflow.nyquist_validation` is `false` in `.planning/config.json`, but the coordinator explicitly required this section, so it is included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `#[test]` + `cargo test`; `tempfile`, `assert_fs` fixtures; ratatui `TestBackend` for renders |
| Config file | none |
| Quick run command | `rtk proxy cargo test --lib agents -- --nocapture` |
| Full suite command | `rtk proxy cargo test --no-fail-fast` |
| Lint gate | `rtk proxy cargo clippy -- -D warnings` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AGENT-01 | porcelain `-z` parser: main-first, locked±reason, prunable, detached, spaces; branch predicate incl. `worktree-wf_` | unit (pure strings) | `rtk proxy cargo test --lib agents::worktrees` | ❌ Wave 0 |
| AGENT-01 | real repo + 3 worktrees: commits/dirty counts, deleted worktree → `?` row kept, non-intrusion (index unchanged, no `index.lock`) | integration | `rtk proxy cargo test --test agents_scan -- --nocapture` | ❌ Wave 0 |
| AGENT-01 | no new spawn site / no blocking call in async fn | guard | `rtk proxy cargo test --test spawn_seam_guard --test async_blocking_guard` | ✅ |
| AGENT-02 | three observed shapes (1 executor/1-plan wave; 3 fixers; 13 executors w2/11); attribution tiers 1/2/3; finished-unmerged; current wave; `w?` excluded | unit (pure) | `rtk proxy cargo test --lib agents::waves` | ❌ Wave 0 |
| AGENT-02 | `summarized_plans` filled + sorted by Pass 2 | unit | `rtk proxy cargo test --lib state_reader::disk_status` | ✅ (extend) |
| AGENT-03 | missing root / garbage JSON / truncated JSON / unknown fields / panicking adapter → row survives with core data; D-A07 fake adapter enriches with zero core/UI edits | unit + integration | `rtk proxy cargo test --lib agents::adapter` and `--test agents_scan` | ❌ Wave 0 |
| AGENT-04 | encoding (ASCII, `.`/space, non-BMP → 2 dashes, >200 → hash suffix, prefix fallback); `CLAUDE_CONFIG_DIR` relative ignored; liveness via `set_modified`; `worktreePath` / `inheritedWorktreePath` / id fallback joins; `finished` two-clause rule; worktree-less live-only listing | unit (tempdir, no git) | `rtk proxy cargo test --lib agents::claude` | ❌ Wave 0 |
| AGENT-05 | ladder forms at widths 60/80/120/160/200; `status_column_cells` equals rendered column; no-agent row byte-identical to today | render (TestBackend) | `rtk proxy cargo test --lib ui::screens::normal` | ✅ (extend) |
| AGENT-06 | `m` toggles Sessions↔Agents, `n` still launches only on Sessions, tab 6 highlighted on both, scroll clamps, empty/degraded states | render + key | `rtk proxy cargo test --lib ui::screens::detail` | ✅ (extend) |
| AGENT-06 | hostile identity escaped in Agents sub-view + dashboard summary; census 11 sub-views | guard | `rtk proxy cargo test --lib render_escape_guard` | ✅ (extend) |
| AGENT-07 | nested `findings.total`; id dedupe across worktree+main subjects; REVIEW-FIX.md hides estimate | unit + integration | `rtk proxy cargo test --lib agents::fixers` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the module-filtered command above plus `rtk proxy cargo clippy -- -D warnings`.
- **Per wave merge:** `rtk proxy cargo test --no-fail-fast`.
  - Baseline: 49 suites, or 50 once `tests/agents_scan.rs` lands.
  - Locally, expect exactly one failure: the git-version witness in `envelope/policy.rs` (local git 2.53.0 vs constants pinned to 2.55.0; user memory).
  - Compare the suite count, not just "green". A fail-fast run hides the envelope suites.
- **Phase gate:** the full suite plus clippy, then `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/agents_scan.rs` — real-worktree fixture builder (reuse `tests/common/mod.rs::git`; `git worktree add -b worktree-agent-<id> .claude/worktrees/agent-<id>`, `git worktree lock --reason "claude agent agent-<id> (pid 1 start 2)"`), fake `<tmp>/claude/projects/<enc>/<uuid>/subagents/` tree
- [ ] In-source test modules for `src/agents/{worktrees,adapter,claude,waves,fixers}.rs` — pure or tempdir-only, **no git spawns**
- [ ] `render_escape_guard.rs` fixture rows (`ALL_SUB_VIEWS`, `sub_view_label`, `DETAIL_TAB_ARRIVAL`, `probe_ctx`)
- Framework install: none

## Security Domain

`security_enforcement` is absent from the config, so it is treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | Read-only; no writes anywhere |
| V5 Input Validation | yes | `Untrusted` carrier; id charset validation; tolerant serde; 64 KiB read cap |
| V6 Cryptography | no | (the path hash is Claude's non-crypto string hash, used for lookup only) |
| V12 Files/Resources | yes | Never join an unvalidated agent id into a path; stat/read only under the resolved root |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal escape / bidi injection via `description`, `agentType`, branch, path | Tampering / Spoofing | `Untrusted::from_untrusted_source` at parse; `.shown()` at render; `render_escape_guard` probe (D-C13) |
| Path traversal via an agent id taken from a worktree dir or branch (`agent-../../x`) | Tampering / Info disclosure | Validate `^[A-Za-z0-9_-]{1,64}$` before building `agent-<id>.meta.json` |
| git option injection via a branch name in argv | Tampering | Never pass branch names to git; use `-C <worktree> … HEAD` and a hex-validated base sha |
| Oversized or hostile JSON (DoS) | DoS | Read cap + `from_slice(..).ok()` |
| Index/lock interference with a live agent | Tampering (availability) | `--no-optional-locks` + `GIT_OPTIONAL_LOCKS=0`; Pitfall 8 control test |
| A spoofed "finished"/"live" signal from a crafted meta in a cloned repo's `.claude/` | Spoofing | Metas are read only from the user's own config root, never from the project tree; display-only impact |

## Sources

### Primary (HIGH confidence)
- Codebase, read this session:
  - `src/state_reader/git_ops.rs:100-317`;
  - `src/state_reader/plan_waves.rs` (whole);
  - `src/state_reader/disk_status.rs:230-450, 669-905`;
  - `src/app.rs:1-120, 1095-1205`;
  - `src/ui/screens/normal.rs:300-560, 704-913`;
  - `src/ui/screens/detail.rs:362-373, 920-1045, 1690-1760, 3235-3275, 3400-3430, 4905-5010, 6440-6520, 6590-6640`;
  - `src/ui/screens/mod.rs:958-967, 1255-1443`;
  - `src/text.rs:400-660`;
  - `src/ui/screens/render_escape_guard.rs:1-120, 1068-1110, 1385-1410, 1610-1680`;
  - `tests/spawn_seam_guard.rs:1-150`; `tests/async_blocking_guard.rs:1-80`; `tests/common/mod.rs`;
  - `Cargo.toml`; `src/action.rs:44-51`; `src/ui/screens/help.rs:240-245`.
- Git behaviour: scratch-repo probe of `worktree list --porcelain [-z]` (git 2.53.0); live ttbook run (13 worktrees: lock state, commits, dirty, ledger, SUMMARY placement, timing).
- ratatui 0.30.0 `Layout` measurement program; `ratatui-widgets-0.3.2/src/table.rs:1041-1065`, 281, 289.
- GSD 1.14.0: `workflows/execute-phase.md:634, 863-864`, `workflows/execute-phase/steps/executor-isolation-dispatch.md:290-292`, `agents/gsd-executor.md:544-551`, `bin/lib/worktree-safety.cjs:23-27`.

### Secondary (MEDIUM confidence)
- Claude Code 2.1.283 binary strings: `k`/`qx`/`Le`/`KJ` encoding, `zu()` projects root, `canonicalWcRootForProject`, `CLAUDE_CONFIG_DIR` absolute-path message. Authoritative for this version, undocumented, may change.
- Meta-field census over 2475 local metas; description census over 690 executor metas; `subagents/` dir-mtime measurement on this live session.

### Tertiary (LOW confidence)
- None. No web research was needed; every claim was checked against local artifacts or the installed binaries.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. There are no new crates; every seam was read.
- Architecture: HIGH. Allowlist, escape-guard and sub-view mechanics were read directly.
- Claude adapter format: MEDIUM. It is undocumented; verified against the binary and live data, and degradation is designed in.
- Pitfalls: HIGH for 1, 2, 3, 5, 8 (measured); MEDIUM for 4 (one live run).

**Research date:** 2026-09-25
**Valid until:** ~2026-10-25 for the codebase and git. The Claude Code format is only as valid as the installed version (re-check if `claude --version` moves past 2.1.283 and the adapter shows "(no agent metadata)" on known-live agents).
