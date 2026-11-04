---
quick_id: 261103-o2r
phase: quick-261103-o2r
plan: 01
subsystem: oneoff
tags: [oneoff, landing, cleanup, crash-recovery, retention]
status: complete
requirements: [QUICK-261103-o2r]
key-files:
  created:
    - src/oneoff/recycle.rs
  modified:
    - src/oneoff/mod.rs
    - src/oneoff/store.rs
    - src/oneoff/launch.rs
    - src/oneoff/land.rs
    - src/oneoff/resolve.rs
    - src/oneoff/resume.rs
    - src/cli.rs
    - src/task_cli.rs
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/oneoff.rs
    - src/ui/screens/render_escape_guard.rs
    - tests/oneoff_e2e.rs
    - tests/oneoff_runtime_seam_guard.rs
    - docs/CONFIGURATION.md
    - README.md
commits: 3
plan_head_before: 3a23e213a018243753e837c31fcfad413b3f594f
plan_head_after: 88aa4f3696d4032622ae7b3b165d88ae87d6080e
actuals:
  tokens: 43750   # chars/4 over the realized diff (175007 chars, plan_head_before..HEAD)
  tasks: 3
  commits: 3
---

# Quick 261103-o2r: One-off run cleanup, crash recovery and land-to-dispatch-branch

A one-off run now lands into the branch HEAD pointed at when it was dispatched, leaves
only a small record behind once it landed or was pushed, and a run whose launcher died
is recovered as `crashed` with its work kept as `gsd-oneoff/<id>`.

## Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 (tracer) | `4bcbd74` | feat: land one-offs into their dispatch branch and slim the run after landing |
| 2 | `bb731fa` | feat: land slimmed one-offs from their tracked branch and show the land target in the TUI |
| 3 | `88aa4f3` | feat: recover crashed one-offs, recycle orphans and age out finished records |

## Shipped

- **D-C land target.** `dispatch_branch` / `dispatch_tip` / `dispatch_detached` recorded in
  `run_session` (a resume inherits them); `RunRecord::land_target()` (Branch / Detached /
  Legacy). `task land merge --into <branch>`. A target that is not checked out is moved by
  `update-ref` with the expected old value (fast-forward, or the merge made in the land
  clone), working tree and index untouched; refusals for a detached dispatch without
  `--into`, a deleted branch, a branch checked out in another worktree and a tip that
  moved. A checked-out target runs the old flow unchanged; a legacy record lands into the
  checked-out branch as before.
- **D-A slim.** `recycle::slim_run` after every successful merge landing and push (CLI
  and TUI): `pruned` first, preservation predicate, `summary.md`, stream/launch tails,
  allowlist deletion; the landed resolver attempts keep their small files. The e2e
  slimmed run dir is under 1 MB. `verified_summary` falls back to `summary.md`.
- **D-A run source.** `RunSource::{Clone, Tracked}`: a pruned run lands (ff and resolver)
  and pushes from `refs/heads/gsd-oneoff/<id>`; a push-only run with no tracked branch
  keeps its clone; a push of a merge-slimmed run says where the work is.
- **D-B recycle.** `recycle_project` on TUI start (`spawn_blocking`, one
  `Action::OneoffRecycled`, experimental flag), `task list` and `task run` (stderr
  summary): dead runs finished as `crashed` (new `AgentOutcome::Crashed`, resumable),
  `settle_work` extracted from `finalize`, tracked through the planted gate and slimmed;
  leftover slims; stale files; orphan dirs (> 1 h, work preserved first); retention (30
  days / newest 50, terminal-and-clean only).
- **Invariant.** Every non-discard clone deletion goes through `clone_preserved`; tests
  prove the refusal for slim (unit), a crashed run with a planted file (e2e), an orphan
  with a planted config (e2e), an unpreserved resolver clone (unit) and retention (pure,
  never a record with a clone or branch).
- **Display and docs.** `→ <target>` and `clone removed` in `task list`; `target:` and
  `clone:` in `task status`; `Target:` / `Clone:` and the target-naming merge question in
  the TUI (a detached dispatch refuses with the `task land merge --into` hint);
  render-escape guard covers a hostile `dispatch_branch` / `pruned.note`;
  `task land merge --help`, docs/CONFIGURATION.md (Where runs live, Landing, push,
  Several at once, new "Cleanup and recovery") and README.md updated.

## Deferred

Nothing from the plan. Known quirks left as they are:

- After a push-slim of a run with failed resolver attempts, `land_failure.attempts` still
  names attempt dirs whose clones are gone (their small files remain).
- A landed run whose clone the predicate keeps is retried (and reported as "kept N
  clone(s)") on every scan until the user discards it.

## Verification

- Tests: `rtk proxy cargo test --no-fail-fast` (raw log `target/o2r-t3.log`):
  **3561 passed, 0 failed, 17 ignored** across 59 result lines (baseline 3522 passed,
  17 ignored). `oneoff_e2e`: 56 tests (42 baseline + 14 new: 5 tracer-level `land_target` /
  `dispatch_branch`, 4 `slimmed_`, 5 `recycle_`).
- `cargo clippy --all-targets -- -D warnings`: clean on the host, on
  `x86_64-pc-windows-gnu` and on `aarch64-apple-darwin`. `cargo fmt --check` clean.
- `git diff --exit-code 0432f6a -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs
  tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs`
  is empty. No paid `claude` run: fake harness and temp git repos only.

## Deviations from Plan

- **[Rule 3 - Blocking]** `tests/spawn_seam_guard.rs` counts any `Command::new` under
  `src/` as a spawn site and the plan forbids editing the guard. The recycle unit tests
  therefore drive git through `launcher::push::git_in` instead of `std::process::Command`.
- **[Rule 1 - Bug in my own test]** The first orphan-with-planted-file e2e committed a
  `.gitattributes`; strict mode (no baseline) only flags program-running git config and
  escaping pointers, so the gate was correctly clean and the orphan was tracked. The test
  now plants `core.fsmonitor` in the clone's `.git/config`, which is what the gate guards.
- A `RunSource` / slim hook was also wired into `resume::start`'s paused-landing retry
  without code of its own (it calls `land::merge`).

## Inferred decisions (new, beyond the plan's I-01..I-16) `[inferred — audit]`

- I-17 A finished run that a successor continues (`resumed_by`) is slimmed by the scan
  when `gsd-oneoff/<id>` is at its head (its clone is not needed to resume it).
- I-18 After a merge landing, every resolver attempt dir the landing does not name is
  deleted wholesale (superseded, no per-clone predicate); after a push-only slim, an
  attempt clone is removed only when `clone_preserved` (against the run's head) passes,
  and its small files stay.
- I-19 `git worktree list --porcelain -z` is tried first and the plain form is the
  fallback; the parser splits on both NUL and newline. A listing that fails refuses.
- I-20 A slim removes `work/` before the rest, so `planted-baseline.json` survives an
  interrupted slim and a retry judges the clone with its baseline.
- I-21 `update_target` also refuses when the target became the project's checked-out
  branch during the landing (an `update-ref` would desync the index).
- I-22 Crash recovery keeps a valid agent `report.json` (source `agent`) and only
  synthesizes a `failed` report otherwise; recorded spend is `previous + stream estimate`.
- I-23 Retention rank counts all finished records (clean or not), newest first.
- I-24 A tracked-source landing fetches `gsd-oneoff/<id>` from the project into itself
  (FETCH_HEAD check kept) so the checked-out and tracked paths share one code path.

## Open questions

None blocking.

## Self-Check: PASSED

Commits `4bcbd74`, `bb731fa`, `88aa4f3` are ancestors of HEAD; `src/oneoff/recycle.rs`
exists; the summary is the only uncommitted artifact besides the orchestrator's.
