# Quick Task 261103-hx7: Parallel one-offs per project, tracked branches, LLM-resolved merges - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended. The human is unavailable. Decisions 1-10 under "Locked by the user" were made with the user before this run. Every item marked `[inferred — audit]` was decided by the orchestrator from the prior quick tasks (261103-f5y, 261103-gt1) and the code, and is recorded for a later audit.

<domain>
## Task Boundary

Builds on the one-off subsystem (`src/oneoff/`, quick 261103-f5y + 261103-gt1). It adds:
- any number of concurrent one-offs per project,
- run branches tracked in the user's repo,
- a merge-based landing whose conflicts a short sandboxed resolver agent session resolves,
- a multi-run Driver-tab UI,
- one sandbox hardening fix,
- docs and help.

It closes `.planning/todos/pending/2026-11-03-oneoff-concurrent-runs-per-project.md`; move that todo to `completed/`.

</domain>

<decisions>
## Implementation Decisions

### Locked by the user (do not revisit)
1. **Keep clones** (`--no-hardlinks`), not worktrees, for sandbox isolation.
2. **Tracking.** When a run finishes, the host fetches the clone's branch into the main repo as `refs/heads/gsd-oneoff/<run-id>`, so `git branch --no-merged` shows unmerged work. Discard deletes that branch as well as the clone and run dir.
3. **Parallel.** Remove the one-run-per-project lock. Any number of one-offs may run per project at once. The Driver-tab one-off UI handles multiple runs: list and select a run, and `T` and the viewer act on the selected run.
4. **Landing.** Fast-forward when possible, otherwise a normal merge commit. Never rebase (SUMMARY files cite shas).
5. **LLM conflict resolution, STATE.md included.** The merge happens in a fresh clone of the current target tip with the run branch merged in, never in the user's checkout.
   - On conflicts, a short sandboxed agent session resolves them. It uses the same launcher sandbox and the same `AgentRuntime` seam, and its own stream is logged in the run dir so the live viewer can show it.
   - The resolver runs the project's tests, commits the merge and reports through a strict validated call.
   - The host then fast-forwards the target branch to that merge commit. If the target moved meanwhile, the merge is redone on the new tip, with bounded retries.
   - The user's working tree changes only on that final ff, and only if it is clean. The existing dirty-checkout refusal stays.
6. **Review.** The result popup marks LLM-resolved merges and lets the user open the resolver's diff: the merge commit against its first parent, with the conflicted files listed.
7. **Failure.** If the resolver fails or the tests stay red, nothing lands. The branch stays visible as unmerged and the popup says why.
8. **Spend.** Resolver sessions count against the run's budget and are logged in the run dir.
9. **Hardening.** bwrap mounts `/` read-only and masks only `/tmp` and `$HOME`. With an `XDG_DATA_HOME` outside `$HOME`, the one-off run dirs (including the host-owned inbox) are readable inside the sandbox. Mask the one-off data root, and the driver's equivalents if they have the same issue, inside the sandbox wherever it lives.
10. Update `docs/CONFIGURATION.md` and `src/ui/screens/help.rs`.

### Liveness without the per-project lock
- Liveness moves from the per-project `oneoff.lock` to a per-run lock file inside the run dir (for example `<run-dir>/run.lock`). It is host-only, never mounted, and taken by the daemon for the run's life and released last, with the same semantics `derive_live` has today. `[inferred — audit]`
- A legacy run (no per-run lock file, created before this change) is judged by the old per-project lock exactly as before, so old run dirs still read correctly. `stop` and `task status` take a run id. When it is omitted, they mean the newest run; if several runs are live, they refuse and list them. `[inferred — audit]`
- There is no concurrency cap. Each run has its own wall clock and budget. `[inferred — audit]`
- Quick-id collisions between parallel `/gsd-quick` runs are accepted. Ids are 2-second base36 timestamps plus a slug, and the merge (with resolver) handles the STATE.md table conflicts. No host-assigned quick id. `[inferred — audit]`

### Tracked branch
- The branch is fetched at finalize, only when the run finished with commits > 0 and the planted-file gate found nothing git-relevant. The gate runs before any git touches the clone, as today. Otherwise no branch is created, and `run.json` records why. `[inferred — audit]`
- The fetch is the existing hardened fetch (`push::git`, `--no-tags --no-recurse-submodules`): `refs/heads/gsd-oneoff/<id>:refs/heads/gsd-oneoff/<id>`, verified to equal the recorded head. It never overwrites an existing branch that points elsewhere; a mismatch is refused and recorded. `[inferred — audit]`
- After a successful landing merge, the host deletes `gsd-oneoff/<id>` with a safe `git branch -d`, because it is merged and stays reachable from the target branch. The run dir is kept until discarded. `[inferred — audit]`
- Discard deletes the branch with a forced `branch -D`, but only when it still points at the recorded head; otherwise it leaves the branch and says so. It also deletes the run dir. `[inferred — audit]`

### Landing flow (`task land merge`)
- The target is the user's checkout's current branch, as today. The existing refusals stay: live run, no head, 0 commits, planted, dirty, detached. `[inferred — audit]`
- **FF case:** the target tip is an ancestor of the run head, so the existing `git merge --ff-only` runs in the user's checkout, with the user's hooks. Unchanged.
- **Non-FF case:** the host makes a fresh `--no-hardlinks` clone of the project at the target tip in the run dir, at `<run-dir>/land/<attempt>/work`, fetches the run branch into it, and tries `git merge --no-ff --no-edit` with hardened git (hooks off, as other host git on agent-touched repos).
  - Clean merge: no agent. The host commits the merge with a fixed message: `Merge one-off <run-id> (<headline>)`. `[inferred — audit]`
  - Conflicts: the resolver session runs in that clone, sandboxed with the one-off profile. Its prompt names the conflicted files (`git diff --name-only --diff-filter=U`), the merge's two sides, the never-rebase rule, the instruction to keep both sides' intent (STATE.md tables: union of rows) and the test command. It must commit the merge (two parents, first parent = target tip) and then run `taskctl report`.
- **Resolver report.** It reuses the existing `taskctl report` schema and validator unchanged. `--status done` means resolved and tests green; `--verified "<test cmd>: <result>"` is required for done. Anything else, or a missing or invalid report, is a failure. `[inferred — audit]`
- **Host checks after the resolver.** The host never trusts the report alone. It re-derives:
  - HEAD is a merge commit whose parents are exactly [target tip, run head],
  - the work tree is clean,
  - no conflict markers remain in the previously conflicted files,
  - the planted-file gate on the land clone is clean.
  Any failure means nothing lands. `[inferred — audit]`
- **Test command.** `workflow.test_command` from the project's `.planning/config.json` when present. Otherwise the prompt tells the resolver to find the project's own test command (CLAUDE.md, README, the build manifest) and to state the command it ran in `--verified`. The host does not run tests itself, because tests run in the sandbox. `[inferred — audit]`
- **Final ff.** The host fetches the merge commit from the land clone into the project (hardened fetch), re-checks that the checkout is clean and that the target tip is still the merge's first parent, then runs the user's own `git merge --ff-only <merge-sha>` (hooks included, as today).
  - If the target moved, the merge is redone from scratch on the new tip, a new attempt dir, up to 3 attempts in total. After that it refuses with "the target kept moving". `[inferred — audit]`
  - A dirty checkout at the final step refuses, and the resolved merge commit is kept in the land clone. No retry loop on dirtiness. `[inferred — audit]`
- **Async in the TUI.** A landing that needs the resolver can take minutes. The TUI merge runs it off the render thread, the way other landings run today, or as a detached `task land merge` daemon if the existing seam needs it. A per-run land lock refuses a second concurrent land of the same run. `[inferred — audit]`
- **Recording.** The landing record in `run.json` gains:
  - how it landed: `ff`, `merge` (clean) or `resolved` (LLM),
  - the merge sha,
  - the conflicted files,
  - the resolver attempt dirs,
  - the failure reason on failure.
  `task status` and the popup show these. `[inferred — audit]`

### Resolver spend and caps
- The resolver's `--max-budget-usd` is the run's remaining budget: `max_budget_usd` minus the run's recorded spend estimate, the high-water mark from gt1's `SpendEstimate`, persisted in `run.json`. Below $0.25 remaining, it refuses with "no budget left to resolve conflicts". `[inferred — audit]`
- The resolver's spend is added to the run's recorded spend.
- Resolver wall clock: 15 minutes per attempt. `[inferred — audit]`
- Its `stream.jsonl`, `launch.log` and `report/` live under `<run-dir>/land/<attempt>/`. `[inferred — audit]`

### Multi-run UI
- The Driver tab's one-off status line shows counts, for example `one-offs: 2 running, 1 to land`.
- `T` opens a run list for the project, newest first, live and finished, with j/k or arrow keys to select. Enter or the action keys act on the selected run: merge, push, discard, stop, `v` view, `e` summary.
- `t` still starts a new run, now also while others are live. `[inferred — audit]`
- The result popup for a resolved merge shows a `resolved by agent` marker and the conflicted files.
- A `d` key opens a scrollable diff: `git diff <merge>^1 <merge>`, read off the render thread and capped. A `v` key on a run with resolver attempts opens the viewer on the newest resolver stream. `[inferred — audit]`
- This run list also satisfies the bulk of `2026-11-03-oneoff-run-history-list.md`. Move it to `completed/` only if the list really shows past finished runs with their outcome; otherwise leave it pending. The dashboard badge todo stays pending unless it is a few lines. `[inferred — audit]`

### Hardening
- For the one-off and driver bwrap profiles, also mask the gsd-meta-manager data root (`dirs::data_local_dir()/gsd-meta-manager`, which covers the one-off root and the driver's per-run config dirs if they live there) with an empty `--tmpfs`, before the binds that re-establish this run's own paths (clone/work, report dir, config dir). Emit it only when that root is not already under `$HOME` or `/tmp`, so the default-layout driver goldens stay byte-identical. Add golden tests for the outside-`$HOME` case. `[inferred — audit]`
- The planner/executor should check where the driver's config dirs and run dirs actually live, and mask whatever host data root is affected, by the same rule.

### Claude's Discretion
- Exact module split (e.g. a new `src/oneoff/merge.rs` / `resolve.rs`), type names, and how much of the land flow is pure functions plus thin I/O.
- Exact key letters, provided they do not collide with existing Driver-tab or one-off screen bindings.
- If the work turns out phase-sized, ship it in this order:
  1. per-run lock, parallel runs, branch tracking and hardening,
  2. merge landing with the resolver,
  3. multi-run UI, docs and help.
  Record anything not reached as follow-up todos under `.planning/todos/pending/`.

</decisions>

<specifics>
## Specific Ideas

- Tests use only the existing fake `claude` / bwrap / systemd-run shim harness (`tests/oneoff_e2e.rs`, `tests/launcher_support/`). No real paid runs.
- The e2e cases to add:
  - two concurrent runs on one project,
  - the branch appears in the main repo after finish,
  - discard removes the branch,
  - a non-conflicting merge lands as a merge commit,
  - a conflicting merge where the fake resolver commits a resolution and reports done, so it lands and is marked resolved,
  - a fake resolver that reports failure, or leaves markers, so nothing lands and the branch stays,
  - the target moves between resolve and ff, so it retries,
  - sandbox argv masks a data root outside `$HOME`.
- Keep `tests/oneoff_runtime_seam_guard.rs` green. Any new agent-specific facts, such as the resolver prompt's agent flags, go through `AgentRuntime`.
- Keep the driver's goldens (`tests/launcher_sandbox.rs`, `tests/launcher_config_dir.rs`, `tests/launcher_push.rs`, `tests/spawn_seam_guard.rs`) unchanged unless the hardening fix intentionally changes the outside-`$HOME` case only.
- Run `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings`. For any check that reads raw output, use `rtk proxy`, redirected to a file rather than piped through grep.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/` (CONTEXT, PLAN, SUMMARY): the one-off design, C-/I- decisions
- `.planning/quick/261103-gt1-live-viewer-with-message-injection-and-s/` (CONTEXT, PLAN, SUMMARY): stream-json, inbox, viewer, spend estimate, soft cap; audit item "Sandbox reads the run dir on some hosts"
- `.planning/todos/pending/2026-11-03-oneoff-concurrent-runs-per-project.md` (closed by this task)
- `.planning/todos/pending/2026-11-03-oneoff-run-history-list.md`, `...-dashboard-running-badge.md` (only if they fall out cheaply)
- `src/oneoff/{land,launch,store,report,runtime,prompt}.rs`, `src/launcher/sandbox.rs`, `src/ui/screens/{oneoff,oneoff_viewer,help}.rs`, `docs/CONFIGURATION.md`

</canonical_refs>
