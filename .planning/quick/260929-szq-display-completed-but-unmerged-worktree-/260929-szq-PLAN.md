---
quick_id: 260929-szq
mode: quick
phase: quick-260929-szq
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/agents/unmerged.rs
  - src/agents/mod.rs
  - src/agents/waves.rs
  - src/app.rs
  - tests/agents_unmerged.rs
  - src/ui/screens/detail.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/help.rs
  - .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh
autonomous: true
requirements: [QUICK-260929-szq]

estimate:
  tokens: 210000
  raw_tokens: 210000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "A plan counts as unmerged when its SUMMARY exists only in a linked worktree, whether committed or untracked. 'Only' means absent from main's `.planning/phases/*/` and `.planning/milestones/*-phases/*/`. Such a plan appears in `ProjectAgents.unmerged` with its worktree path, a short branch ref, its +N/~N counts and an `UnmergedState`. Once the SUMMARY is in main it is no longer listed, however it got there (squash, rebase or merge). The worktree's commit ancestry is never the done-signal (I-1)"
    - "A worktree holding only a PLAN (no SUMMARY) never flags, including a stale worktree whose PLAN main has since deleted. Non-SUMMARY phase docs (REVIEW, REVIEW-FIX, VERIFICATION, VALIDATION) never flag (I-3)"
    - "A quick task (`YYMMDD-xxx`) whose SUMMARY exists in a worktree's `.planning/quick/<id>-*/` but in no main quick dir is listed as `Quick(id)` (I-5)"
    - "Unmerged work has two states, and both are distinct from merged:"
    - "  - `InProgress`: its holder worktree is LOCKED by an agent whose liveness is not Ended, or the agent there is Live/Idle."
    - "  - `AwaitingMerge`: the worktree is unlocked or its lock is stale (Ended), with no live agent."
    - "  Both render as `◐ unmerged`; InProgress adds a yellow `▶ active` marker (I-4, I-13)"
    - "The dashboard Status cell of a project with unmerged work (either state) shows `◐N unmerged` (or `◐N` when narrower) even when no agent runs. The executor ladder's `{d}/{t} done` counts main-only SUMMARYs and appends `· ◐N`. A project with no worktrees renders byte-identically to today (I-7)"
    - "In the Phases Waves pane, a plan finished on a worktree reads `◐ unmerged` with a dim `@<ref> +N ~D` tail (plus `▶ active` when InProgress), not `leftover`. This holds on non-active phases too. In the Agents sub-view, a row whose worktree holds AwaitingMerge work reads `unmerged` instead of `done`/`ended`/`?`, while an InProgress row keeps its liveness word (I-8, I-9, I-10)"
    - "On the Roadmap, a phase not done in main whose remaining plans are all unmerged draws a cyan, undimmed `◐ unmerged`. Its dependents keep their Blocked/Ready status. The detail status line of any phase with unmerged plans ends in `◐k` (I-11)"
    - "Help shows a `Work State` block. Each glyph in it is interpolated from the same constants the Waves pane, the Roadmap and the dashboard render (I-12)"
    - "Real-world check, run read-only against `~/projects/python/mailbot` with a temp `--config` and temp XDG dirs, while its phase-05 SUMMARYs are still worktree-only:"
    - "  - the mailbot dashboard row shows `◐`;"
    - "  - the Roadmap's phase 05 row shows `◐`;"
    - "  - phase 05's Waves rows read `unmerged` with `@a486395`."
    - "  If mailbot's state has changed, the check reports SKIP, and the mailbot-modelled fixture (h) carries the proof (I-14)"
    - "`rtk proxy cargo test --no-fail-fast` fails only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, which is expected locally. `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0. `tests/agents_scan.rs::no_file_under_src_agents_writes_reads_proc_or_spawns` stays green"
  artifacts:
    - path: src/agents/unmerged.rs
      provides: "`UnmergedKey {Plan(PlanRef), Quick(String)}`, `UnmergedState {AwaitingMerge, InProgress}`, `UnmergedItem`, `GLYPH_UNMERGED`/`LABEL_UNMERGED`/`LABEL_ACTIVE`, `main_done_keys`, `worktree_done_keys`, `short_ref`, `detect`, and pure unit tests"
    - path: tests/agents_unmerged.rs
      provides: "tempdir-git fixtures covering these cases: (a) flagged, (b) squash-merged cleared, (c) untracked flagged, (d) stale main-deleted PLAN not flagged, (e) quick flagged/cleared, (f) archived not flagged, (g) non-agent worktree flagged, (h) mailbot-modelled locked phase 05 → InProgress, then unlocked → AwaitingMerge"
    - path: src/agents/waves.rs
      provides: "`AgentView.unmerged`, `pending_unmerged()`, `unmerged_plan(stem)`, `unmerged_in_phase(key)`, `awaiting_merge_at(path)`, the ◐N ladder, main-only `d`"
    - path: src/ui/screens/detail.rs
      provides: "`PaneState::Unmerged` + hint tail with `▶ active`, the Agents `unmerged` word, `roadmap_model_for(.., view)` post-pass, `WAVES_GLYPH_*` constants"
    - path: src/ui/roadmap_graph.rs
      provides: "`PhaseStatus::Unmerged`, `PhaseFacts.unmerged`"
    - path: src/ui/screens/help.rs
      provides: "the `Work State` legend block and its test"
    - path: .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh
      provides: "the read-only tmux smoke against the real mailbot repo (I-14)"
  key_links:
    - from: "`scan_project_with_probe` (src/agents/mod.rs ~377), after `rows` are built"
      to: "`unmerged::detect(&core, &rows, project_root)` → `ProjectAgents.unmerged`"
      via: "the existing ~5 s Tick scan (app.rs ~1269) in spawn_blocking; no new timer or watcher, and no new git helper"
    - from: "`Action::AgentsScanned` handler empty-skip (src/app.rs ~1786)"
      to: "`waves::derive` → `ctx.agent_views[alias]` → `agent_summary_line` (normal.rs ~437)"
      via: "the skip lets a project with `!agents.unmerged.is_empty()` through, so an unmerged-only project gets a view"
    - from: "`AgentView::unmerged_plan` / `unmerged_in_phase` / `awaiting_merge_at`"
      to: "`waves_model` state_of + PanePlan hint, `agent_list_lines` word, `roadmap_model_for` post-pass"
      via: "every UI surface reads the one scan-time list and never touches the filesystem at render time"
---

# Quick 260929-szq: show completed-but-unmerged worktree work distinctly from merged work

<objective>
Today a plan (or quick task) that finished only on a git worktree/branch is shown as "done"/"executed", exactly as if it had merged into main. For example, the dashboard's `d = done + finished` counts it and the Agents view calls it `done`. This task adds path-level detection of such work: a SUMMARY that exists in a worktree but not in main. The detection records which worktree and branch holds the work, and whether that worktree is still in use. The task also adds a distinct `◐ unmerged` visualization on the dashboard, in the Waves and Agents views, on the Roadmap and in the help legend. The visualization follows `260929-szq-UX-SPEC.md`, amended by I-13. The detection follows `260929-szq-RESEARCH.md`. The real-world acceptance case is mailbot:
- Phase 05 (05-01..05-05 SUMMARY) exists only in the LOCKED worktree `agent-a486395e992a58454`, which is 34 commits ahead with a dirty `REQUIREMENTS.md`.
- Main's STATE is at phase 09.

Purpose: the core value is "see the state of every GSD project at a glance". Work waiting to be merged is a pending action and must not pass as finished.
Output: `src/agents/unmerged.rs`, `tests/agents_unmerged.rs`, edits to the agents view model and four UI surfaces, and a read-only mailbot smoke script.

## Decisions (all [inferred]: the human was unavailable. Carry each I-n into the SUMMARY for audit)

- **I-1 Detection is path-level.** A plan is unmerged-done when the worktree holds a `*-SUMMARY.md` whose key is not among main's SUMMARY keys. Commit ancestry is REJECTED: RESEARCH found 13 worktrees "ahead" after squash merges. Main is `core.main_worktree` (the first porcelain block), falling back to `project_root`, not origin/HEAD. A main SUMMARY always wins, so squash and rebase merges clear correctly.
- **I-2 Scope.** Every non-prunable non-main worktree is scanned, agent-pattern or not, because a human feature worktree can hold unmerged work too. When a project has no such worktree, main's keys are not read at all. The existing `.git/worktrees` prefilter already returns early for most projects.
- **I-3 Step (d) is deferred** (the PLAN-only `git diff --diff-filter=A` guard). PLAN-only keys never flag, so no new git read and no new `git_ops` helper is needed. Counts for a worktree that is not already an agent row reuse the existing `worktrees::worktree_counts`. It is called only for a worktree that holds at least one unmerged key.
- **I-4 One item per distinct key**, with the representative worktree = the first holder by path.
  - `state = InProgress` iff some holder worktree has an agent row that either (a) has `liveness.is_running()`, or (b) has `row.locked` and `liveness != Ended`. A lock whose owner is gone, or which has been silent for more than a day, is stale. Rows exist only for agent-pattern or adapter-claimed worktrees.
  - Otherwise `state = AwaitingMerge`. A human worktree (no agent row) is AwaitingMerge even when git-locked.
  - Dirty state never suppresses an item, so an untracked SUMMARY flags.
- **I-5 Quick keys.** The key is the `YYMMDD-xxx` prefix of the quick dir name, validated as 6 digits, a `-`, then 3 `[a-z0-9]`. Main's quick set = ids whose main `.planning/quick/<id>-*/` dir holds a `*-SUMMARY.md`. A worktree quick dir whose id is already in main's set is skipped without a `read_dir`, as a cost prune. `.planning/quick-batches/` is ignored.
- **I-6 Known limits, accepted.** A SUMMARY that main deleted outright, rather than archiving it, still flags in a stale worktree. RESEARCH's ttbook snapshot found none. A renumbered phase whose key collides with an archived one reads as merged, which is a false negative. Main's phase set includes `milestones/*-phases/*/` so that stale worktrees predating an archive do not flag.
- **I-7 Dashboard ladder.** `d` = `self.done` only, i.e. main. N = `pending_unmerged()` counts EVERY item, in BOTH states. With N > 0:
  - Every active ladder (executor, fixer and `N agents`) becomes: `forms[0] + " · ◐N unmerged"`, then each form + `" · ◐N"`, then the last bare form.
  - An idle project yields `◐N unmerged`, then `◐N`.
  - An idle project with S stalled rows yields `◐N unmerged · S stalled`, then `◐N · S stalled`, then `S stalled`. This keeps normal.rs's `ends_with("stalled")` yellow rule untouched.
  - This replaces the Status cell, as the stalled form already does (UX-SPEC (d): "also emitted when no agent active").
- **I-8 Waves pane.** `PlanState::Finished` keeps its meaning and gains one more trigger: the plan's key is in the unmerged list, in either state. Its word becomes `unmerged`. `PaneState::Leftover` is renamed `PaneState::Unmerged`, and its glyph comes from `GLYPH_UNMERGED`. Non-active phases also show `Unmerged` for a plan the view lists. This covers mailbot, whose phase 05 is not main's active phase.
- **I-9 Agents sub-view.** A row reads `unmerged` (Cyan) when its worktree holds an AwaitingMerge item (`view.awaiting_merge_at(&row.path)`), whatever its liveness (Finished, Ended or Unknown). A row whose worktree holds InProgress work keeps its liveness word, because the agent is still there. Any other row is unchanged. The state column widens from 5 to 8 cells through a named const.
- **I-10 short_ref.**
  - Strip `refs/heads/`, `worktree-` and `gsd/`.
  - `agent-<id>` with a valid id becomes the first 7 chars of the id, extended until unique among the scan's agent ids. For example, mailbot's `worktree-agent-a486395e992a58454` becomes `a486395`.
  - Any other branch keeps its stripped name.
  - A worktree with no branch (detached HEAD) uses its directory name.
  - The result is stored as `Untrusted`. The UI prefixes `@`, draws it through `shown()` and fits it to 14 cells with `fit_cells`.
- **I-11 Roadmap.** `roadmap_model_for` post-processes the `layout_list` output and sets `PhaseStatus::Unmerged` when all of these hold: the phase is not Done, not a shipped (archived) node, has k>0 unmerged plans (either state), and `plans == Some((d,t))` with `t>0 && d+k>=t`. Unmerged wins over Active/Ready/Blocked. Because dependents are computed from markers beforehand, they stay blocked (UX: unmerged does not satisfy dependents). The `◐k` tail goes only in the detail-pane status line. The list row's 5-cell plans column is unchanged.
- **I-12 Help.** A `Work State` block goes right after `Dashboard Badges`. Its glyphs are interpolated from constants (rule T-18-63), never retyped.
- **I-13 A locked worktree with complete SUMMARYs gets a COMBINED state, not `▶ running`** (the coordinator's fork, decided here).
  - Both unmerged states render `◐ unmerged` (cyan) and both count in ◐N and toward the Roadmap `k`.
  - `InProgress` adds a yellow `▶ active` marker after the `@ref +N ~D` hint in Waves rows. The glyph is interpolated from `WAVES_GLYPH_RUNNING`; the word is `LABEL_ACTIVE = "active"`.
  - This deliberately deviates from UX-SPEC (a)'s "running > unmerged" precedence, for three reasons:
    1. `plan_state` already ranks a worktree SUMMARY above Running.
    2. A lock is often held by a parent session long after the plan's own work finished, as in mailbot, where review-fix, verification or merge may still run. `▶ running` would hide a complete phase there.
    3. The real-world acceptance requires mailbot's phase 05 to render `◐` while its worktree is locked.
  - A plan with NO SUMMARY and a live agent still renders `▶ running` exactly as today.
- **I-14 Real-world smoke is read-only.**
  - It runs against `~/projects/python/mailbot` through the release binary, with `--config <tmp>/config.json` and `XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_STATE_HOME/XDG_CACHE_HOME` pointed at a `mktemp -d` dir. `HOME` stays untouched, so the Claude adapter can read `~/.claude` transcripts read-only.
  - It never writes into mailbot. It runs no git command there itself: the app's own git reads go through `git_ops`, with `--no-optional-locks`.
  - Its precondition check is `test -e`/`ls` only.
  - It sends navigation keys only.
  - The scratchpad snapshot is never used as a live repo. Fixture (h) models it in a tempdir instead.
- **Deferred** (optional per coordinator, not built):
  - the `//u` filter
  - the roadmap detail "Worktrees" block
  - the ASCII box-view `^` marker / `PhaseMarker::Unmerged` (PhaseMarker is state_reader's single done/current source, so the variant ripples wide)
  - step (d)
  - a per-worktree (HEAD sha, dir mtime) scan cache
  - the `~N stray` Agents line
  - full branch+path of the selected row in the status line
  - rendering quick tasks anywhere other than the dashboard count
  - splitting the dashboard ◐N by state
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/quick/260929-szq-display-completed-but-unmerged-worktree-/260929-szq-UX-SPEC.md
@.planning/quick/260929-szq-display-completed-but-unmerged-worktree-/260929-szq-RESEARCH.md

<interfaces>
Existing seams. Read them with grep plus offset reads. `src/ui/screens/detail.rs` is 25k lines (1.2 MB) and `src/app.rs` is 5.8k lines: never read either whole.

src/agents/mod.rs:
- `pub struct AgentRow { path, branch: Option<Untrusted>, agent_id: Option<String>, locked: bool, prunable, commits_ahead: Option<u32>, dirty: Option<u32>, liveness: AgentLiveness, plan: Option<waves::PlanRef>, summary_in_worktree: bool, .. }` (~242)
- `pub struct ProjectAgents { rows, worktreeless, main_worktree, base_sha, scanned_at, fixer_estimate }` (~284), which derives Default/PartialEq/Eq
- `fn worktree_holds_summary(phase_dir, plan)` (~326): the FIX/GAPCLOSURE exclusion and the `plan_index(stem)` pairing to mirror
- `pub fn scan_project_with_probe(project_root, adapters, probe, now) -> ProjectAgents` (~377): `core = worktrees::scan_worktrees(..)`, then rows are built only for `wt.agent_pattern || claim.is_some()`
- `AgentLiveness::is_running()` = Live | Idle
- `Ended` = the owner is gone, or silent for more than `MAX_AGENT_AGE_SECS`
- The Claude adapter sets `lock_released = worktree.locked.is_none()`, so a locked worktree is never `Finished`

src/agents/worktrees.rs:
- `pub struct CoreWorktree { path, branch: Option<Untrusted>, locked: Option<Option<Untrusted>>, prunable, agent_pattern, agent_id: Option<String>, .. }` (~66)
- `pub struct CoreScan { main_worktree, base_sha, worktrees }` (~92)
- `pub(crate) fn worktree_counts(&CoreWorktree, Option<&str>) -> (Option<u32>, Option<u32>)` (~490)
- `pub fn valid_agent_id(id)` (~335)

src/agents/waves.rs:
- `pub struct PlanRef { pub phase: PhaseNum, pub plan: u32 }` with `from_id` and `label()`
- `pub enum PlanState { Done, Finished, Running, Stalled, Queued }` (~103)
- `fn plan_state(done_in_main, rows)` (~291)
- `pub fn derive(&ProjectAgents, &ProjectState) -> AgentView` (~340)
- `AgentView::summary_forms()` (~518), where `let d = self.done + self.finished;` is at ~565. `fixer_forms()` (~592)

src/state_reader/disk_status.rs: `pub(crate) fn plan_index(stem) -> Option<(PhaseNum, u32)>` (~628)
src/state_reader/phase_num.rs: `phase_key`, `same_phase`
src/text.rs: `Untrusted::from_untrusted_source(String)`, `.as_raw_for_logic_only()`, `.shown()`
src/cli.rs: the global `--config <PATH>` flag
src/config.rs: `Config { version: 2, projects: {alias: {path, added}}, preferences }` as JSON. `default_path()` uses `dirs::config_dir()`, which honours `XDG_CONFIG_HOME`.

src/app.rs:
- `Action::AgentsScanned` handler (~1771), with the empty-skip at ~1786
- test helpers `obs_app` (~2885), `render_top_screen` (~5146), `live_executor_scan` (~5467), and the tracer test `an_agents_scan_reaches_the_dashboard_status_cell` (~5491)

src/ui/screens/normal.rs: `agent_summary_line(view, cells)` (~437) picks the first form that fits. It is yellow iff `form.ends_with("stalled")`. NOT edited.

src/ui/screens/detail.rs:
- `TAB_LABELS_FULL` (~475): `1:Roadmap`, `2:Phases`, ...
- `enum PaneState` + `glyph()/word()/style()` (~8556-8620)
- `fn waves_model(phase_number, inf, view, phase_is_active)` (~8726), whose `state_of`/`make` closures are at ~8745-8765
- `struct PanePlan` (~8625)
- the Waves plan-row renderer `WavesRowKind::Plan` arm (~9174-9226)
- `agent_state_word` (~9605), `agent_state_style` (~9618), `fit_cells` (~9632), `agents_wave_strip` (~9683), `agent_count` (~9815), `agent_line` with its `{:<5}` pad (~9824), `agent_list_lines` (~9846)
- `pub(crate) fn roadmap_model_for(state, cache, show_badges)` (~2772), called at ~1588 and ~6127 (both have `self.alias`/`alias` + `ctx`) and at ~21145-22040 in tests

src/ui/roadmap_graph.rs:
- `GLYPH_*` (~215)
- `pub enum PhaseStatus` + `glyph()` (~332)
- `pub struct PhaseFacts` (~384), built in `layout_list` (~840-860)

src/ui/roadmap_view.rs: `glyph_style` (~265), `status_word` (~276), `phase_spans` (~808), `status_spans` (~959), the `parallel_rows` tail (~1114)
src/ui/screens/help.rs: `BADGE_LEGEND`, `row()`, and the body builder with its `heading("Dashboard Badges")` at ~362

tests/agents_scan.rs:
- `plain_repo()` / `add_worktree()` / `commit_file()` (~43-97): the tempdir-git pattern, which returns `None` when sandboxed
- `FORBIDDEN_HALVES` (~590-613): no `remove_*`, `ren`+`ame(`, `write_all(`, `/pr`+`oc`, `Command::new(` under src/agents production code

Snapshot, READ-ONLY and for modelling only: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/068882e0-2580-4769-b5db-96f153b1fce5/scratchpad/mailbot-snapshot/` (`worktrees.txt`, `unmerged.txt`, `planning-<worktree>/`).
- Main `phases/05-attachment-export/` holds 05-01..05-05 PLAN only.
- `agent-a486395e992a58454`'s copy adds `05-0{1..5}-SUMMARY.md`, `05-REVIEW.md`, `05-REVIEW-FIX.md`, `05-VALIDATION.md` and `05-VERIFICATION.md`.
</interfaces>
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): detect unmerged SUMMARYs in worktrees (with InProgress/AwaitingMerge) → ProjectAgents → AgentView → dashboard `◐N unmerged`</name>
  <files>src/agents/unmerged.rs, src/agents/mod.rs, src/agents/waves.rs, src/app.rs, tests/agents_unmerged.rs</files>
  <behavior>
    - Integration (tests/agents_unmerged.rs, real git, skip when `plain_repo()` is None). Each fixture's main commits `.planning/phases/13-demo/13-01-PLAN.md` before the worktree is added.
      - (a) The agent worktree `.claude/worktrees/agent-a0123456789abcdef` commits `13-01-SUMMARY.md` into its `.planning/phases/13-demo/`. Expect `scan_project_with(root, &[], now).unmerged` to be exactly one item: key `Plan(13-01)`, worktree = agent path, `short_ref` raw `a012345`, `commits_ahead == Some(1)`, `state == AwaitingMerge`.
      - (b) As (a), then the same SUMMARY is committed on main by copying the file, which simulates a squash. `unmerged` is empty although the worktree is still 1 ahead.
      - (c) The SUMMARY is written untracked in the worktree: flagged, `dirty >= Some(1)`.
      - (d) Main commits `15-10-PLAN.md` in `.planning/phases/15-x/`, the worktree is added, then main `git rm`s the PLAN and commits. The worktree holds only the PLAN, so `unmerged` is empty.
      - (e) The worktree adds `.planning/quick/260929-abc-slug/260929-abc-SUMMARY.md`: `Quick("260929-abc")` is flagged. Once main holds a `260929-abc-*` dir with a SUMMARY, the item clears.
      - (f) Main holds `13-01-SUMMARY.md` only under `.planning/milestones/v1.0-phases/13-demo/`: not flagged.
      - (g) A non-agent worktree on branch `feature/x` at a sibling path holds the SUMMARY: flagged, `short_ref` raw `feature/x`, `rows` empty, `AwaitingMerge` even after `git worktree lock`.
      - (h) mailbot-modelled.
        - Main commits `.planning/REQUIREMENTS.md` and `.planning/phases/05-attachment-export/05-0{1..5}-PLAN.md`.
        - The worktree `.claude/worktrees/agent-a486395e992a58454` is added on branch `worktree-agent-a486395e992a58454`. It commits `05-0{1..5}-SUMMARY.md`, `05-REVIEW.md`, `05-REVIEW-FIX.md`, `05-VALIDATION.md` and `05-VERIFICATION.md`, then modifies `REQUIREMENTS.md` without committing.
        - `git worktree lock --reason "claude agent agent-a486395e992a58454 (pid 1 start 1)"`.
        - Expect exactly five `Plan(05-0N)` items, all `InProgress`, with `short_ref` `a486395` and `dirty == Some(1)`.
        - After `git worktree unlock`, all five are `AwaitingMerge`.
        - In BOTH states, `waves::derive(.., &ProjectState::default()).summary_forms() == ["◐5 unmerged", "◐5"]`.
      - End-to-end: `scan_projects_guarded(&[(alias, root)], &NoProcessProbe, now)` over (a), then `derive(..)`. `summary_forms()` must equal `["◐1 unmerged", "◐1"]`.
    - Unit (src/agents/unmerged.rs `#[cfg(test)]`; tempdir `std::fs` fixtures are allowed in test code, but no spawn):
      - `short_ref` handles `worktree-agent-<id>`, `refs/heads/gsd/foo` (→ `foo`), a detached HEAD (→ the dir name), and the 7→8 extension when two ids share 7 chars.
      - The quick-id validator rejects `2609-abc` and `260929-ABC`.
      - FIX and GAPCLOSURE SUMMARYs are never keys.
      - The state rule (I-4) as a pure fn over (is_running, locked, liveness): Live → InProgress; locked+Stalled → InProgress; locked+Unknown → InProgress; locked+Ended → AwaitingMerge; unlocked+Finished → AwaitingMerge; no row → AwaitingMerge.
    - Unit (src/agents/waves.rs):
      - Executor mode with done=4, finished=2, total=9, one item has the widest form ending `4/9 done · ◐1 unmerged`, and every later form ends `· ◐1` except the last bare form.
      - Idle with 2 items + 1 stalled row gives `["◐2 unmerged · 1 stalled", "◐2 · 1 stalled", "1 stalled"]`.
      - InProgress and AwaitingMerge items both count in N.
      - `plan_state` gives Finished for a listed plan (either state), and Done when it is summarized in main.
    - Unit (src/app.rs): an `AgentsScanned` whose `ProjectAgents` has empty rows/worktreeless and one item gives the alias a view, and `render_top_screen` contains `◐1`.
  </behavior>
  <action>
    RED first: write tests/agents_unmerged.rs and the unit tests above, run them, and see them fail to compile or fail. Commit `test(quick-260929-szq): add failing unmerged-detection tests`.
    - In the new integration file, copy the `plain_repo`/`add_worktree`/`commit_file` helpers from tests/agents_scan.rs. Add one helper that creates parent dirs, writes a nested path and optionally commits it.
    - Fixture (h) is MODELLED on the snapshot's file names only: it never reads from or points at the snapshot or at mailbot.

    Then implement:

    1. **src/agents/unmerged.rs.** Start with a module doc that states it is read-only: names only via `std::fs::read_dir`, no writes, no spawns, no contents read. Respect the `FORBIDDEN_HALVES` tokens in production code. Add `pub mod unmerged;` in src/agents/mod.rs. Define:
       - `pub const GLYPH_UNMERGED: &str = "\u{25D0}"`, `pub const LABEL_UNMERGED: &str = "unmerged"` and `pub const LABEL_ACTIVE: &str = "active"`.
       - `pub enum UnmergedKey { Plan(waves::PlanRef), Quick(String) }`, deriving Debug, Clone, PartialEq, Eq, PartialOrd, Ord and Hash.
       - `pub enum UnmergedState { AwaitingMerge, InProgress }`.
       - `pub struct UnmergedItem { pub key, pub worktree: PathBuf, pub branch: Option<Untrusted>, pub short_ref: Untrusted, pub commits_ahead: Option<u32>, pub dirty: Option<u32>, pub state: UnmergedState }`.
       - `pub fn main_done_keys(main_planning: &Path) -> BTreeSet<UnmergedKey>`. It reads SUMMARY names from `phases/*/` and `milestones/*-phases/*/`, keyed via `disk_status::plan_index` on the `-SUMMARY.md` stem, with the same FIX/GAPCLOSURE exclusion as `worktree_holds_summary`. It also reads quick ids from `quick/*/` dirs holding any `*-SUMMARY.md` (I-1, I-5, I-6).
       - `pub fn worktree_done_keys(wt_planning: &Path, main: &BTreeSet<UnmergedKey>) -> Vec<UnmergedKey>`. It covers `phases/*/` and `quick/*/` only, and skips a quick dir whose id main already holds (I-5).
       - `pub fn short_ref(..) -> Untrusted`, per I-10.
       - A pure `fn item_state(row: Option<&AgentRow>) -> UnmergedState`, per I-4.
       - `pub fn detect(core: &worktrees::CoreScan, rows: &[AgentRow], project_root: &Path) -> Vec<UnmergedItem>`:
         - Return empty when no non-prunable worktree exists (I-2).
         - Main = `core.main_worktree` or `project_root`.
         - Collect each worktree's keys ∉ main set, and dedupe per key with the first holder by path. The state is InProgress if ANY holder's `item_state` is InProgress (I-4).
         - Counts come from the matching row, else `worktrees::worktree_counts(wt, core.base_sha.as_deref())`, only for holders (I-3).
         - Sort by key.
         - Any tracing is counts only (D-C13).
    2. **src/agents/mod.rs.** Add `pub unmerged: Vec<unmerged::UnmergedItem>` to `ProjectAgents` and fill it in `scan_project_with_probe` after `rows` are sorted.
    3. **src/agents/waves.rs.**
       - Add `pub unmerged: Vec<UnmergedItem>` to `AgentView`, copied in `derive`.
       - `plan_state` takes an extra `listed: bool`, true when the plan's `PlanRef` is a `Plan` item in either state. It returns Finished right after Done (I-8).
       - Add the pub helpers `pending_unmerged() -> u32` (all items, plans + quick), `unmerged_plan(stem) -> Option<&UnmergedItem>` (matched by `plan_index`), `unmerged_in_phase(phase_key: &str) -> u32` and `awaiting_merge_at(path) -> bool`.
       - Rewrite `summary_forms`: `d = self.done`; wrap every active ladder with one private `with_unmerged(forms, n)` helper; add the idle forms, all per I-7. The `◐` comes from `GLYPH_UNMERGED` and ` · ` stays U+00B7.
       - Update the existing waves tests that encoded `done + finished` to the main-only count. This change is intended, per UX-SPEC (d).
    4. **src/app.rs.** Widen the empty-skip at ~1786 so it also requires `agents.unmerged.is_empty()`. Update the comment. Add the unit test next to `an_agents_scan_reaches_the_dashboard_status_cell`.

    GREEN: all new and existing agents/app tests pass. Commit `feat(quick-260929-szq): detect unmerged worktree work and surface ◐N on the dashboard`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --test agents_unmerged --test agents_scan --test agents_waves</automated>
    <automated>rtk proxy cargo test --lib --no-fail-fast agents::</automated>
    <automated>rtk proxy cargo test --lib --no-fail-fast app::</automated>
  </verify>
  <done>
    Fixtures (a)-(h) pass, including (h): the locked mailbot-modelled phase 05 is five InProgress items, and after unlock they are five AwaitingMerge items, with ◐5 in both states. The end-to-end `["◐1 unmerged", "◐1"]` holds. An unmerged-only project gets a dashboard view showing `◐1`. The `agents_scan` guard is green, and `d` counts main only.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Waves pane `◐ unmerged @ref +N ~D [▶ active]` and Agents sub-view `unmerged` word</name>
  <files>src/ui/screens/detail.rs</files>
  <behavior>
    - `PaneState::Unmerged.word() == LABEL_UNMERGED` and `glyph() == GLYPH_UNMERGED`. The existing uniqueness/ASCII/width test (~23604) still passes.
    - The mailbot-shaped `waves_model` test:
      - Setup: the view's `active_phase` is 09. Its `unmerged` list holds `Plan(05-01..05-05)`, short_ref `a486395`, `commits_ahead Some(34)`, `dirty Some(1)`, `InProgress`. `inf` for phase 05 has 5 plans and none summarized.
      - Every plan is `PaneState::Unmerged`, even though phase 05 is NOT active, with hint `@a486395 +34 ~1` and the active marker.
      - The same test with `AwaitingMerge` gives the same hint without the marker.
      - A summarized plan stays Done even if listed.
    - The Waves plan-row line at 100 cells contains `unmerged`, `@a486395 +34 ~1` and (InProgress) `▶ active`. At 30 cells the line's width never exceeds 30.
    - `agents_wave_strip` reads `1 unmerged` where it used to read `1 leftover`.
    - `agent_list_lines`:
      - a Finished or Ended row whose path holds AwaitingMerge work reads `unmerged`;
      - a Live row whose path holds InProgress work reads `live`;
      - a Finished row without listed work reads `done`;
      - every state word is padded to the same width.
  </behavior>
  <action>
    Work only through grep plus offset reads in src/ui/screens/detail.rs (see interfaces).
    1. Rename `PaneState::Leftover` to `PaneState::Unmerged` everywhere, including `PANE_STATES`, `from_plan_state`, the `moving` set and `agents_wave_strip`. Its `glyph()` returns `crate::agents::unmerged::GLYPH_UNMERGED` and its `word()` returns `LABEL_UNMERGED`. Its style stays Cyan (I-8).
       - Extract the Done/Running/Stalled glyph literals (`\u{2713}`, `\u{25b6}`, `!`) into `pub(super) const WAVES_GLYPH_DONE`, `WAVES_GLYPH_RUNNING` and `WAVES_GLYPH_STALLED`, and have `glyph()` return them. Task 3's help legend and the active marker interpolate these (I-12, I-13).
       - Update any test literal that spelled the old word.
    2. **`waves_model`.** Keep the unfiltered view before the active-phase filter. In `state_of`:
       - the active-phase `plan_state` lookup stays first;
       - then a summarized plan is Done;
       - then the unfiltered view's `unmerged_plan(id)` → `PaneState::Unmerged`;
       - else `not_done`.
    3. Add `hint: Option<Vec<(String, Style)>>` (or a small struct) to `PanePlan`. For an Unmerged plan it is built from `unmerged_plan(id)`:
       - a DarkGray segment made of `@` + `fit_cells(&item.short_ref.shown().to_string(), 14)`, then a space and `agent_count("+", item.commits_ahead)`, then ` ~D` when `dirty` is `Some(D)` with D>0;
       - for `InProgress`, a Yellow segment made of a space, `WAVES_GLYPH_RUNNING`, a space and `LABEL_ACTIVE` (I-13).
       - It is already escaped: it never contains raw branch bytes (T-szq-01).
    4. **Waves plan-row renderer (`WavesRowKind::Plan` arm).** Draw the hint segments after the label and before the title. Reserve their total width the same way `tokens_w` is reserved: show the hint only when `cells >= base + word_w + 1 + hint_w + min_title.min(8)`. If the hint does not fit, drop the active segment first, then the whole hint. Count it in `used`/`room` so the title truncates around it and the line never exceeds `cells`.
    5. **Agents sub-view (I-9).**
       - `agent_line` gains a `pending: bool`, computed in `agent_list_lines` as `view.awaiting_merge_at(&row.path)`. When it is true the word is `LABEL_UNMERGED` in Cyan; otherwise the word is `agent_state_word(liveness)` as today.
       - Replace the `{:<5}` pad with `const AGENT_STATE_CELLS: usize = 8` and use it in the `fixed` width computation too. Children rows pass `false`.
       - Update the few tests that depend on the 5-cell pad.
    6. Add the behavior tests above in detail.rs's test module. Commit `feat(quick-260929-szq): unmerged state in the Waves pane and Agents sub-view`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast ui::screens::detail</automated>
  </verify>
  <done>
    In the Waves pane, a plan finished on a worktree shows `◐ unmerged` plus a dim `@<ref> +N ~D` tail, and a yellow `▶ active` when its worktree is still locked or live. This holds on non-active phases, such as mailbot's phase 05. The wave strip counts `N unmerged`. In the Agents sub-view, AwaitingMerge rows read `unmerged` and InProgress rows keep their liveness word. All detail tests pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Roadmap `PhaseStatus::Unmerged` + `◐k`, help "Work State" legend, full gates, read-only mailbot smoke</name>
  <files>src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/ui/screens/detail.rs, src/ui/screens/help.rs, .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh</files>
  <behavior>
    - `PhaseStatus::Unmerged.glyph() == GLYPH_UNMERGED`. `status_word` returns `unmerged`, and `glyph_style` is Cyan and not dim.
    - A `roadmap_model_for(state, None, false, Some(&view))` test:
      - the mailbot shape (phase 05 has plans (0, 5) in main, not Done, and the view lists 5 InProgress `05-*` plans) → phase 05 status `Unmerged`, `unmerged == 5`;
      - with only 1 of 3 listed, the status is unchanged (Active/Ready/Blocked) and `unmerged == 1`;
      - a phase that depends on 05 keeps its pre-post-pass status;
      - passing `None` for the view keeps the existing roadmap tests byte-identical.
    - `status_spans` for a phase with `unmerged == 3` ends with a `◐3` span.
    - The help body contains a `Work State` heading. Its rows interpolate `WAVES_GLYPH_DONE`, `roadmap_graph::GLYPH_DONE`, `GLYPH_UNMERGED`, `WAVES_GLYPH_RUNNING` (for both `active` and `running`), `WAVES_GLYPH_STALLED` and `LABEL_ACTIVE`, plus the `@a0973c1` and `Dashboard ◐N` explanation rows.
  </behavior>
  <action>
    1. **src/ui/roadmap_graph.rs.**
       - Add `pub const GLYPH_UNMERGED: &str = crate::agents::unmerged::GLYPH_UNMERGED;`.
       - Add the `PhaseStatus::Unmerged` variant (doc: "not done in main; every remaining plan finished on a worktree, unmerged — never satisfies dependents") and its `glyph()` arm.
       - Add `pub unmerged: u32` to `PhaseFacts`, initialised to 0 in `layout_list`, and fix any test constructor the compiler flags.
    2. **src/ui/roadmap_view.rs.**
       - `glyph_style`: Unmerged → Cyan. `status_word`: Unmerged → `unmerged`. `phase_spans` still dims only Done.
       - `status_spans`: after the plans span, when `p.unmerged > 0`, push a Cyan span with a leading space, `GLYPH_UNMERGED` and k (I-11).
       - In the `parallel_rows` tail, an Unmerged phase reads `unmerged`.
       - Fix every other exhaustive `PhaseStatus` match the compiler flags.
    3. **src/ui/screens/detail.rs.**
       - Add a 4th parameter `view: Option<&AgentView>` to `roadmap_model_for`. After the existing shipped-node post-pass, for each non-shipped phase `u`: set `k = view.map_or(0, |v| v.unmerged_in_phase(&facts.key))` and `facts.unmerged = k`. When `k > 0`, the status is not Done, and `facts.plans` is `Some((d, t))` with `t > 0 && d + k >= t`, set the status to `PhaseStatus::Unmerged` (I-11). Document the rule in the fn doc.
       - Update the two production callers (~1588 and ~6127) to pass `ctx.agent_views.get(&self.alias)` / `ctx.agent_views.get(alias)`, and the test callers to pass `None`.
       - The pass stays in-memory, with no I/O.
    4. **src/ui/screens/help.rs.**
       - Import `WAVES_GLYPH_*` from `super::detail`, `GLYPH_DONE` from `crate::ui::roadmap_graph`, and `GLYPH_UNMERGED`/`LABEL_UNMERGED`/`LABEL_ACTIVE` from `crate::agents::unmerged`.
       - After the Dashboard Badges loop's trailing blank, push `heading("Work State")` and a blank line.
       - Then push these rows, each built with `format!` from the constants, never retyped:
         - `✓ ● done`: merged into the main branch
         - `◐ unmerged`: finished on a worktree branch, not yet in main
         - `▶ active` (after a ◐ hint): its worktree is still locked or has a live agent
         - `▶ running`: a live agent is working on a worktree
         - `! stalled`: locked/dirty worktree, no recent activity
         - `@a0973c1`: worktree branch (agent id, 7 chars); +N commits ahead, ~N dirty paths
         - `Dashboard ◐N`: N plans or quick tasks waiting to be merged
       - End with a trailing blank. Keep every description unique in the body.
       - Add the test `the_work_state_legend_is_built_from_the_rendered_glyphs`.
    5. Run the full gates. If a test outside these files breaks because `d` is now main-only or because of the word rename, fix it to the new intended value and record it in the SUMMARY. Commit `feat(quick-260929-szq): roadmap unmerged status and Work State help legend`.
    6. **Read-only real-world smoke (I-14).** Write `.planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh`, a bash script with `set -euo pipefail`, run from the repo root. It does the following, in order:
       - (i) Precondition, read-only with `test -e`/`ls` only. `~/projects/python/mailbot/.claude/worktrees/agent-a486395e992a58454/.planning/phases/05-*/05-01-SUMMARY.md` must exist AND no `~/projects/python/mailbot/.planning/phases/05-*/05-0?-SUMMARY.md` may exist; `tmux` must be on PATH. If any of this fails, print `SKIP: <reason>` and exit 0.
       - (ii) `cargo build --release`.
       - (iii) `T=$(mktemp -d)`. Write `$T/config.json` = `{"version":2,"projects":{"mailbot":{"path":"<abs mailbot path>","added":"2026-09-29T00:00:00Z"}},"preferences":{}}` and create `$T/{config,data,state,cache}`.
       - (iv) `tmux new-session -d -s szq-smoke -x 200 -y 50` running `env XDG_CONFIG_HOME=$T/config XDG_DATA_HOME=$T/data XDG_STATE_HOME=$T/state XDG_CACHE_HOME=$T/cache target/release/gsd-meta-manager --config $T/config.json`. Install a `trap` that always kills the session and removes `$T`.
       - (v) Poll up to 30 s, with `tmux capture-pane -p -t szq-smoke > $T/dash.txt`, until the line containing `mailbot` also contains `◐`. Auto-registration may add other rows: select the mailbot row with the `/mailbot` filter + Enter before opening it.
       - (vi) Open the mailbot detail view and press `1` (Roadmap). Capture, and assert that a line holding phase id `05` contains `◐`.
       - (vii) Press `2` (Phases) and move the phase cursor to 05 (`g`, then `j` until the capture's selected row shows 05, max 20 presses). Capture, and assert the capture contains both `unmerged` and `@a486395`.
       - (viii) Press `q` (then `q` again if a confirm appears), and exit non-zero with the failing capture printed if any assertion failed.
       - Send only navigation keys (Enter, `/`, letters of `mailbot`, `1`, `2`, `g`, `j`, `k`, Right, Esc, `q`). Never run git against mailbot. Never touch the scratchpad snapshot.
       - Record whether `▶ active` appeared in the Phases capture in the SUMMARY; it depends on the live lock owner, so the script does not assert it.
       - Run it. On SKIP, record the reason: fixture (h) plus Task 2/3's mailbot-shaped unit tests are then the proof. If a capture shows the TUI cannot be driven headless, do the same and state that explicitly. Commit the script with the docs commit.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast roadmap</automated>
    <automated>rtk proxy cargo test --lib --no-fail-fast help</automated>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings</automated>
    <automated>bash .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh</automated>
  </verify>
  <done>
    - The Roadmap draws a phase whose remaining plans all sit unmerged on worktrees (mailbot's 05) as cyan `◐ unmerged`, and its dependents stay blocked. A partially unmerged phase ends `◐k`.
    - Help documents the Work State vocabulary, including `▶ active`, from shared constants.
    - In the full `--no-fail-fast` run, the only failure is the git-version constants test. Clippy is clean.
    - The mailbot smoke either passes (dashboard `◐`, Roadmap 05 `◐`, Waves `unmerged @a486395`) or reports a recorded SKIP reason.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| worktree filesystem → scan | Branch names, directory names and file names in linked worktrees are clone-authored (untrusted). |
| scan → render | Untrusted ref text reaches terminal cells. |
| meta-manager → running GSD agents | The scan must never disturb a live agent's worktree (non-intrusive constraint). mailbot's phase-05 worktree is locked by a live session. |
| smoke script → the user's real repo and config | The verification touches a real project and must leave it and the user's config untouched. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-szq-01 | Tampering (terminal injection) | `short_ref` → Waves hint / Agents label | high | mitigate | `short_ref` is stored as `Untrusted`, and the UI draws it only through `.shown()` + `fit_cells(..,14)`. Agent ids pass `valid_agent_id`. The quick id is regex-validated before it becomes a key, and plan keys go through `plan_index` (numbers only). |
| T-szq-02 | Denial of Service | `unmerged::detect` read_dir fan-out on many worktrees | medium | mitigate | Names only, one directory level per phase/quick dir, and quick dirs already merged are pruned (I-5). The detection is skipped with no non-prunable worktree (I-2). It runs inside the existing spawn_blocking + catch_unwind scan behind `agents_scan_in_flight`, so it never blocks the render loop. |
| T-szq-03 | Tampering / non-intrusion | git reads against live agent worktrees | high | mitigate | There is no new git invocation: only the existing `git_ops` helpers (`--no-optional-locks`, `GIT_OPTIONAL_LOCKS=0`, 10 s budget), called only for holder worktrees (I-3). The `no_file_under_src_agents_writes_reads_proc_or_spawns` guard stays green. |
| T-szq-04 | Information disclosure | tracing in detect | low | mitigate | Logs carry counts only, never branch, path or dir names (D-C13). |
| T-szq-05 | Spoofing | a symlinked `.planning` in a hostile worktree | low | accept | Only names are read, nothing is written or opened for content, and the worst outcome is a wrong `◐N` count. |
| T-szq-06 | Tampering | mailbot-smoke.sh against the real mailbot + user config | high | mitigate | The script uses a temp `--config`, temp `XDG_*` dirs, a `test -e`-only precondition and navigation-only keys, and runs no git command against mailbot. A `trap` kills the tmux session and removes the temp dir. The snapshot is never used as a live repo (I-14). |
| T-szq-SC | Tampering | cargo dependencies | low | accept | No new crate. `Cargo.toml`/`Cargo.lock` are untouched. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`. The one expected local failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. Do not pipe the output through grep, because rtk truncates downstream of the proxy.
- `rtk proxy cargo clippy --all-targets -- -D warnings`.
- `tests/agents_unmerged.rs` fixtures must actually run, not be skipped. If `plain_repo()` returns `None` in the sandbox, say so in the SUMMARY.
- `bash .planning/quick/260929-szq-display-completed-but-unmerged-worktree-/mailbot-smoke.sh` passes or prints a SKIP reason, which is recorded in the SUMMARY.
</verification>

<success_criteria>
- Work finished only on a worktree is visibly distinct (`◐ unmerged`, cyan) from merged work (`✓`/`●`) on the dashboard, in the Waves pane, in the Agents sub-view, on the Roadmap and in help.
- It is possible to tell which worktree/branch holds it (`@ref +N ~D`), and whether that worktree is still in use (`▶ active`) or simply awaiting merge.
- Merged work, including squash merges, is never reported as unmerged.
- The real mailbot phase 05 renders `◐` with `@a486395`, or the SKIP reason is recorded.
- No new timer, watcher, git helper, write or lock.
- Every [inferred] decision I-1..I-14 and the Deferred list are copied into the SUMMARY.
</success_criteria>

<output>
Create `.planning/quick/260929-szq-display-completed-but-unmerged-worktree-/260929-szq-SUMMARY.md` when done.
</output>
