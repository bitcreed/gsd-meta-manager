# Quick Task 261103-f5y: One-off sandboxed task dispatch from the TUI - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended (human unavailable). Decisions stated by the user in the task brief are
marked **[user]**; decisions inferred from planning artifacts and code are marked
**[inferred — audit]** and must be audited later.

<domain>
## Task Boundary

From the TUI (experimental, behind `GSDMM_EXPERIMENTAL_FEATURES`, same gate as the Driver tab,
`src/experimental.rs` / `AppContext::experimental`), pick a pending todo
(`.planning/todos/pending/*.md`) or type a task, press a key, and a sandboxed headless agent runs
`/gsd-quick` (one key) or `/gsd-fast` (another key) on it in ONE `claude -p` session, then returns a
strict executive summary shown in the TUI, linked to the quick task's SUMMARY.md, with
merge / push / discard landing actions.

This is phase-sized. This quick task ships a **core slice** (below); everything listed under
"Deferred" becomes follow-up todos in `.planning/todos/pending/`, not silent omissions.

</domain>

<decisions>
## Implementation Decisions

### Execution model (no driver)
- **[user]** No driver, no nested subagent, no DIRECTION.md / LOG.md, no master prompt, no gap
  rules, no hook dispatch gate. ONE `claude -p` session whose prompt is the GSD command plus the
  reporting instruction. Caps: wall-clock + `--max-budget-usd`.
- **[user]** Reuse launcher infrastructure in `src/launcher/` (sandbox.rs argv builders, host.rs
  probe, auth.rs token, config_dir.rs seeding, egress, process.rs spawn/detach, stop.rs,
  outcome.rs stream parsing, push.rs). Compose them in a new module; do NOT route through
  `launcher::run::launch` (it is DIRECTION/RunStore-bound).
- **[inferred — audit]** New top-level module `src/oneoff/` (mod.rs + runtime.rs + report.rs +
  launch.rs + land.rs, names at planner's discretion) — runtime-neutral except `runtime.rs`.
- **[inferred — audit]** Synchronous std-threads code like the launcher (the launcher's
  "never from an async fn" rule and `tests/async_blocking_guard.rs` apply); every process spawn
  goes through `launcher::process` so `tests/spawn_seam_guard.rs` stays green (extend its
  allowlist only if unavoidable, and say so).
- **[inferred — audit]** `config_dir::seed` currently installs driver gate hooks/driverctl shim.
  Add a minimal option (or sibling fn) that seeds the per-run `CLAUDE_CONFIG_DIR` with the same
  host links (skills, agents, commands, gsd-core) and settings but WITHOUT the driver gate hooks /
  Stop-guard / driverctl shim. GSD skills must resolve inside the sandbox so `/gsd-quick` works.

### Where the agent works (isolation / "worktree")
- **[inferred — audit]** Host-side run directory, mirroring the driver's `driver_root()` layout:
  `dirs::data_local_dir()/gsd-meta-manager/oneoff/<hash8(canonical project)>/<run-id>/`. Keeping
  it outside the project keeps the user's checkout clean (a prerequisite for ff-merge).
- **[inferred — audit]** The agent works in a **local clone** `<run-dir>/work`
  (`git clone --local <project>`, then branch `gsd-oneoff/<run-id>` at the project's current HEAD
  sha, recorded as `base`), not a `git worktree`: a linked worktree's `.git` file points into the
  project's `.git`, which the sandbox would have to bind read-write. The clone is self-contained,
  so only `<run-dir>/work` is mounted read-write. Uncommitted changes in the project are NOT
  carried (the confirm popup says so).
- **[inferred — audit]** Sandbox mounts: `<run-dir>/work` rw as the project; `<run-dir>/report/`
  rw (holds only `report.json`); everything else in the run dir (stream log, run.json, launch log)
  is host-written and NOT mounted, so the agent cannot forge the store's records.
- **[inferred — audit]** Keep the launcher's sandbox posture: host probe refuses when the host
  cannot sandbox; an explicit per-launch `--unsandboxed` mirrors `driver launch` (CLI only; the TUI
  never offers it). `--setting-sources user` (project settings not loaded), egress proxy with token
  injection as the driver uses.

### Runtime seam (requirement)
- **[user]** ONE small internal seam (enum or trait, single Claude impl, one module, no config
  surface) holding all Claude specifics: argv construction, sandbox mounts/env/auth needs, stream
  parsing into a common outcome type (`Finished` / `Failed` / `RateLimited` / `OverBudget`, plus
  whatever "no report" synthesis needs), capability flags (supports budget cap, supports hooks).
- **[inferred — audit]** Shape: `enum AgentRuntime { Claude }` in `src/oneoff/runtime.rs`, with
  methods; the Claude impl delegates to `launcher::sandbox::claude_argv`-style building (a variant
  without driver specifics) and `launcher::outcome::StreamAccumulator`. Runtime-neutral code (run
  dir, report CLI, clone, land, TUI) must not import Claude-specific items — enforce with a small
  unit/guard test or at least by module boundaries.

### Raw stream persistence (forward-compat)
- **[user]** Persist the agent's raw stream-json events to a file in the run dir as they arrive
  (`<run-dir>/stream.jsonl`, append, line-flushed, bounded per line like the launcher's
  `MAX_STREAM_LINE`). Don't build any live view.

### Strict report CLI
- **[user]** A validated CLI the agent calls; rejects malformed input with a non-zero exit and a
  clear message so the agent retries; writes the report atomically (tmp + rename) into the run dir.
  Fields: `--status done|partial|blocked|failed`, `--headline <text>`, `--changed <text>` (repeatable,
  ≤5), `--verified "<cmd>: <result>"` (repeatable), `--followup <text>` (repeatable).
- **[inferred — audit]** Name: `gsd-meta-manager taskctl report …` — mirrors the existing
  `driver` (host verbs) / `driverctl` (agent-facing verbs) split. Host verbs live under
  `gsd-meta-manager task …` (`run`, `status`, `stop`, `land merge|push|discard`).
- **[inferred — audit]** Validation limits: headline 1..=120 chars, single line; each changed /
  followup item 1..=200 chars single line; `--changed` ≤5, `--verified` ≤5, `--followup` ≤5;
  `--verified` must contain `": "`; control characters rejected. Report target dir comes from an
  env var the launcher sets (e.g. `GSDMM_ONEOFF_REPORT_DIR`); a second report overwrites the first
  (last write wins, atomic).
- **[user]** The agent never types the commit range or SUMMARY path. Host-side, after the session
  ends, the store records `base..head` of the clone's branch and derives the SUMMARY path from files
  changed in that range matching `.planning/quick/<id>-<slug>/<id>-SUMMARY.md` (none for
  `/gsd-fast`, which writes no SUMMARY — link absent, not an error).
- **[user]** If the agent never reports, the result is a synthesized "no report" outcome from exit
  status + stream-json (via the runtime seam's outcome type).

### Prompt
- **[inferred — audit]** Prompt = `/gsd-quick <task>` or `/gsd-fast <task>`, then a fixed reporting
  instruction: the human is unavailable, decide from existing artifacts, mark inferred decisions;
  when finished run `gsd-meta-manager taskctl report …` (exact usage given), retry on validation
  error. For a todo: `<task>` = the todo's title plus "details: .planning/todos/pending/<file>";
  plus "if and only if status is done, `git mv` the todo to `.planning/todos/completed/` and commit
  it" — this realises "on done+merge, move the todo to completed/" without the host ever committing
  to the user's checkout (a discard drops the move with everything else). Typed task text is
  bounded (≤2000 chars) and passed verbatim after the command.

### Caps and model
- **[inferred — audit]** Defaults match `driver launch`: model `opus`, `--max-budget-usd 10`,
  wall-clock 60 min. CLI flags `--model`, `--max-budget-usd`, `--cap-minutes` reuse the existing
  clap value parsers in `src/cli.rs`. Wall clock enforced host-side (kill the scope) plus the
  systemd `RuntimeMaxSec` backstop, as the launcher does.
- **[inferred — audit]** TUI confirm popup shows model + spend ceiling + wall clock; `m` in the
  confirm popup cycles `opus` → `sonnet` → `opus` (model selectable without a config surface).

### Concurrency
- **[user, slice decision; inferred — audit]** One one-off per project at a time: an `flock`ed
  lock file at `<oneoff-root>/<hash8>/oneoff.lock` held by the detached daemon for the run's life; a
  second dispatch is refused with a clear message (STATE.md quick-task table would conflict).

### Landing
- **[user]** Result popup offers merge / push / discard.
- **[inferred — audit]** merge: only when the project checkout is clean (`git status --porcelain`
  empty), HEAD is on a branch, and the clone branch tip is a fast-forward of the project HEAD;
  then `git fetch <work> gsd-oneoff/<run-id>` + `git merge --ff-only FETCH_HEAD` in the project
  (host git, no hooks bypass). Otherwise refuse with the reason; nothing changes.
- **[user]** push: to `gsd-auto/<alias>/<run-id>` via the existing push path (mirror + SAFE-03
  tree scan + `dest_ref`). **[inferred — audit]** If `push_run`/`push_finished_run` are RunStore-
  bound, extract the minimal store-free core they share and call it from both; the driver path's
  behaviour must not change (its tests stay green). Never push anywhere else.
- **[inferred — audit]** discard: delete the run dir (clone included) after the run is not live;
  merged/pushed runs keep their run dir until discarded.

### TUI flow
- **[inferred — audit]** Lives on the Driver tab (already experimental-gated). Free Driver-tab
  keys (help.rs Driver section uses j k [ ] PgUp PgDn f G c a u e s R x o, Shift+D): `t` opens the
  one-off picker; `T` opens the one-off status/result popup for this project.
- **[inferred — audit]** Picker popup: list of pending todos (title from first heading/frontmatter
  `title`, file name fallback; all text through the codebase's `Untrusted` sanitisation) plus a
  "type a task…" entry with a single-line input; in the picker `q` = /gsd-quick, `f` = /gsd-fast,
  Esc cancels (the modal popup owns its keys; if the routing proves otherwise use `Q`/`F`).
  → confirm popup (model, spend ceiling, wall clock, "uncommitted changes are not included"),
  Enter launches detached (`task run` via the launcher's detach pattern; never blocks the render
  loop), Esc cancels.
- **[inferred — audit]** Running: the Driver tab shows a one-line one-off status (running /
  elapsed / mode) and the dashboard project row shows a small running badge if it can be added
  cheaply (else deferred); `x` inside the `T` popup kills a running one-off (asks first) via the
  stop path.
- **[user]** Finished: `T` shows ~6 lines: status, headline, changed, verified, followups, SUMMARY
  link; `e` opens SUMMARY.md in `$EDITOR` like other file links (from `<run-dir>/work`, the clone,
  since the SUMMARY only exists there until merged); `m` merge, `p` push, `d` discard (each asks
  first), Esc closes.
- **[inferred — audit]** Flag off: no `t`/`T` handling, no help rows, no badge — same gating
  pattern as every Driver surface.

### Docs
- **[user]** Update `docs/CONFIGURATION.md` (experimental section) and the help screen
  (`src/ui/screens/help.rs`, Driver section, with its tests) for the new keys and commands.

### Claude's Discretion
- Exact file/struct names inside `src/oneoff/`, run-id format (reuse the driver's `valid_run_id`
  shape), JSON schema of `run.json` / `report.json`, popup layout.
- Test strategy: pure functions (validation, argv, outcome synthesis, SUMMARY derivation from a
  file list, ff-ability decision) unit-tested; clone/land exercised against temp git repos;
  launching a real agent is NOT tested (no network/agent in CI).

</decisions>

<specifics>
## Core slice (this quick task) vs Deferred

**Core slice — must ship:**
1. `src/oneoff/` runtime seam (Claude only), run dir + `run.json`, raw `stream.jsonl`
   persistence, `taskctl report` with validation + atomic write, host-side commit-range /
   SUMMARY derivation, synthesized no-report outcome.
2. `task run` (clone, sandbox via launcher pieces, detached daemon, caps, per-project lock),
   `task status`, `task stop`, `task land merge|push|discard`.
3. Minimal TUI on the Driver tab (`t` picker incl. typed task, `q`/`f`, confirm popup with `m`
   model cycle, status line, `T` result popup with `e`/`m`/`p`/`d`/`x`), help rows, CONFIGURATION.md.

**Deferred (become todos if not reached; the executor must create them rather than drop them):**
- Dashboard row running badge (if not cheap).
- Live/attach view of `stream.jsonl` (explicitly out of scope).
- More than one concurrent one-off per project.
- A second runtime (Codex etc.) behind the seam.
- One-off run history list beyond the latest run per project.

</specifics>

<canonical_refs>
## Canonical References

- `src/launcher/mod.rs` (launcher facts 1-13), `src/launcher/run.rs` (launch order, detach,
  cap monitor), `src/launcher/sandbox.rs` (`claude_argv`, `bwrap_argv`, `systemd_run_argv`,
  `child_env`, `LaunchPaths`), `src/launcher/config_dir.rs` (`driver_root`, `seed`),
  `src/launcher/outcome.rs`, `src/launcher/push.rs`, `src/launcher/stop.rs`,
  `src/launcher/process.rs`.
- `src/experimental.rs`, `src/ui/screens/help.rs` (Driver section + tests),
  `src/ui/screens/driver.rs`, `src/ui/screens/driver_confirm.rs` (confirm popup with spend ceiling
  precedent), `src/cli.rs` (`driver launch` defaults and value parsers), `src/driverctl_cli.rs`
  (agent-facing CLI precedent).
- `docs/CONFIGURATION.md` experimental section.

</canonical_refs>
</content>
</invoke>
