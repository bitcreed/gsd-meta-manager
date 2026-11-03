---
quick_id: 261103-klf
phase: quick-261103-klf
plan: 01
subsystem: oneoff
tags: [oneoff, cancel, pause, resume, run-json, bounded-diff, tui, cli]
requires: [261103-f5y, 261103-gt1, 261103-hx7]
provides:
  - accurate run.json git refs (Landing.range, cleared tracked_branch, BranchState normaliser)
  - bounded, streamed, exportable merge diff (process::run_capped / run_to_file, diff_view)
  - stop / kill / pause of runs and conflict resolvers (CLI + TUI x / X / z)
  - task resume --from branch|master (TUI u) and retry of a paused landing
affects: [src/oneoff, src/launcher/process.rs, src/launcher/push.rs, src/cli.rs, src/task_cli.rs, src/app.rs, src/ui/screens, docs/CONFIGURATION.md]
key-files:
  created:
    - src/oneoff/cancel.rs
    - src/oneoff/resume.rs
    - .planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md
  modified:
    - src/launcher/process.rs
    - src/launcher/push.rs
    - src/oneoff/{land,launch,merge,resolve,store,runtime,session,inbox,prompt,mod,stream}.rs
    - src/{cli,task_cli,action,app}.rs
    - src/ui/screens/{oneoff,oneoff_viewer,help,render_escape_guard}.rs
    - tests/{oneoff_e2e,oneoff_runtime_seam_guard}.rs
    - tests/launcher_support/mod.rs
    - docs/CONFIGURATION.md
metrics:
  tasks: 3
  commits: 3
  files_changed: 28
  insertions: 5627
  deletions: 265
status: complete
commits: 3
plan_head_before: 634e9416da491a7843c6afc75514963115c4b30b
plan_head_after: ccf48c0ed59cd5970fdb159805682167120c4a1a
actuals:
  tokens: 58700
  tasks: 3
  commits: 3
---

# Quick 261103-klf: Cancel / pause one-off runs and resolvers, accurate run.json refs, bounded merge diff

All three tasks shipped; nothing was deferred, so no fallback todo was needed. The C-10 follow-up
todo (resume through the agent's own session) is written.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | `7e534a9` | feat(quick-261103-klf): accurate run.json git refs and a bounded, exportable merge diff |
| 2 | `483c61c` | feat(quick-261103-klf): stop, kill and pause one-off runs and their conflict resolvers |
| 3 | `ccf48c0` | feat(quick-261103-klf): resume a paused or stopped one-off from its branch or from master |

`git rev-list --count 634e941..HEAD` = 3 (measured from the plan ledger).

## Shipped vs deferred

Shipped (CONTEXT priority a..g): (a) run.json refs, (b) bounded diff + export, (c) hard cut and
kill for runs and resolvers, CLI + TUI, (d) soft pause of runs, (g) resolver soft pause, (e) `task
resume --from branch`, (f) `--from master` with the resolver. Docs and the three identical help copies
cover all of it. Deferred: nothing from the plan. Not built, as CONTEXT says: resume through the agent
session id (todo written).

## Tests

- Baseline (`target/klf-t0.log`, 59 binaries): 3433 passed + 1 failed, 17 ignored. The one failure is
  `registry_test::end_to_end_add_then_list_via_cli`, contaminated because I started editing while the
  baseline compiled it (it spawns cargo); that binary's 15 tests are all green in t1..t3, so the
  equivalent baseline is 3434 passed, hx7's number.
- Final (`target/klf-t3.log`, 59 binaries, none missing, none failed): **3492 passed, 0 failed, 17
  ignored**. Lib 2760 -> 2806, `oneoff_e2e` 26 -> 38, `registry_test` 14+1 -> 15 ok; every other
  binary line is identical to the baseline.
- `cargo clippy --all-targets -- -D warnings`: clean on the host, `x86_64-pc-windows-gnu` and
  `aarch64-apple-darwin`. `cargo fmt --check`: clean.
- `git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs
  tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs`: empty. The
  spawn seam needed no registration (the new reads live in `src/launcher/process.rs`).

## hx7 audit items

- "the merge diff reads git's whole output before capping" - CLOSED (`run_capped` stops at 5000
  lines / 512 KiB, numstat 2000 files / 1 MiB, kills and reaps git; full diff only child stdout -> file).
- "a landing's `tracked_branch` is not cleared in `run.json`" - CLOSED (ff and merge write once after
  the delete; discard writes before `remove_dir_all`; pre-klf files normalise through `branch_state()`).
- "the TUI cannot cancel a running resolver" - CLOSED (`x` / `X` / `z` in the list, summary and both
  viewers; per-run stop flag for TUI-owned landings; `land/stop.request`).

## Deviations from Plan

None needing a rule. Two departures worth a line:

- **I-12 realised as a non-blocking state machine.** Instead of one blocking `cut(containment, child,
  kill_now, grace)`, the supervised wait loop drives the pure `cancel::Cancel` and sends TERM / KILL as
  separate steps while it keeps ticking (wall clock and soft cap still run). Same observable timing;
  the unit tests drive injected instants.
- **`a_failed_discard_leaves_no_stale_branch_ref` is tolerant of removal order.** `remove_dir_all`
  may unlink `run.json` before it reaches the stuck directory (filesystem order), so the test always
  asserts exit 1 and the branch gone, and asserts the `run.json` fields only when the file survived.

## Decisions `[inferred — audit]`

Planner (all carried): C-01..C-21, I-01..I-25 as numbered in the plan / CONTEXT. New ones made while
executing, one line each:

1. [inferred — audit] `CappedOutput` also carries `stderr_line`; `run_to_file` returns `(status, last stderr line)` so `git_capped` / `git_to_file` can report `last_error` like `git`.
2. [inferred — audit] `run_capped` reports `status: None` when it killed the child at the cap, and a cap met exactly (no more output) is NOT truncated (one extra `read` probes).
3. [inferred — audit] `git_call` is the one builder of the hardened args + env for `git`, `git_capped`, `git_to_file`, so the existing "hardened_git_args(args) once" source guard still holds.
4. [inferred — audit] The diff marker reads `truncated — N more file(s), ~M more line(s)` with singular / plural; counts exclude `---` / `+++` header lines by tracking the `diff --git` .. `@@` header state.
5. [inferred — audit] `branch_state()` treats ANY Merged landing + still-set `tracked_branch` as gone; a `Pushed` landing leaves the branch tracked.
6. [inferred — audit] Writing a stop request merges with a pending one (stronger kind wins) and renames a pid-named temp file; reading is `O_NOFOLLOW | O_NONBLOCK`, regular files only, 4 KiB.
7. [inferred — audit] A TERM / Ctrl-C with no request file (or with a pause file) is a hard stop by `signal`; a request file seen only AFTER a natural exit maps Hard -> `stopped`, Kill -> `killed`, Pause + clean exit -> `paused` (runs only; a resolver is never `Paused`).
8. [inferred — audit] A landing treats its caller's stop flag (Ctrl-C of `task land merge`, the TUI's per-run flag) as a hard stop, so `land_failure.stop = hard` is recorded for it too.
9. [inferred — audit] `StopOutcome` gains `PauseRequested` (exit 0), `LandingStopped`, `LandingRequested`; `task stop` prints that it acted on the landing when it picked it automatically.
10. [inferred — audit] The TUI waits 0 s for a pause or any landing request and 60 s for a hard / kill of a run; a hard / kill of a landing also sets the owned flag, a pause does not.
11. [inferred — audit] `X` on a resolver rebuilds the scope unit name from run id, newest attempt and project hash and requires `-land<n>-` in it before `systemctl --user kill --signal=KILL`.
12. [inferred — audit] In the viewers, the notice for a refused `x`/`X`/`z`/`u` is shown in the footer; the `[following]` marker moved to the front of the footer so the longer key list cannot push it off.
13. [inferred — audit] `ResumeMerge.how` is `Option<MergeHow>`; `failure` is set on a failed resume merge and its run ends `failed` with the launcher-failure synthesized report.
14. [inferred — audit] `resume::start` takes a `land_opts` closure so a CLI's signal handlers are installed only when a paused landing is actually retried; the TUI never retries from `start` (its `u` asks the merge question instead).
15. [inferred — audit] Under its own old-run land lock, `start` clears the view's `landing` flag (the flag only reflected our own lock) before applying `resume_refusal`.
16. [inferred — audit] `key_plan` in the TUI blocks on any `resumed_by`, because the UI cannot read the successor; resuming again after a failed resume merge is done with the CLI.
17. [inferred — audit] A resume's resolver attempt is treated as live in the resolver viewer while the new run is `preparing` and has `resumed_from` (the merge record only exists after it).
18. [inferred — audit] The resume-merge resolver's prompt uses an empty headline (the new record has no report yet) and a new `into_run` direction; parents are `[old head, checkout tip]`.
19. [inferred — audit] The exact pause message is "Pause requested: stop at a safe point, commit your work in progress on your branch, then call `gsd-meta-manager taskctl report --status partial` with a headline and one `--followup` per thing still left to do, then stop."
20. [inferred — audit] `task stop --kill` conflicts with `--pause` at the clap level; `--resolver` combines with either.

## Audit items for a human

- The TUI resolver `X` and the daemon's TERM / KILL go through `systemctl --user kill --signal=...`
  by unit name; only the fake `systemctl` (which signals the unit's process group) exercised it.
- `task stop --pause` on an orphaned scope is refused ("no live launcher to pause it"); hard / kill
  stop the leftover scope themselves. Judge whether a refusal is the right UX.
- A resumed run's `commits` counts only its own commits (`base` = old head / merge commit); its
  landing still carries the whole chain because the branch history does.
- Old `stop.request` files in a finished run dir are inert but never cleaned (a landing clears its own
  stale one on lock take).
- The app-level test of a TUI stop proves the per-run flag and the target selection; the
  `StopBy::Tui` value reaching `run.json` is by construction (`stop_with`), not by a test with a real
  run dir.

## Unverified without a paid run

- A real agent honouring a mid-turn pause message (commit, `taskctl report --status partial`, exit).
- Real `systemctl --user kill --signal=TERM|KILL` on a real scope (bwrap as the namespace init).
- A real resumed session continuing well from the primed context (task, report, commit log, master
  note); this is why the C-10 todo records the agent-session variant.

## Known Stubs

None.

## Self-Check: PASSED

- Files: `src/oneoff/cancel.rs`, `src/oneoff/resume.rs`, the C-10 todo and this SUMMARY exist.
- Commits `7e534a9`, `483c61c`, `ccf48c0` are ancestors of HEAD; the working tree is clean apart from
  the orchestrator's own artifacts.
