---
quick_id: 261103-hx7
phase: quick-261103-hx7
plan: 01
subsystem: oneoff
tags: [oneoff, parallel-runs, per-run-lock, tracked-branch, merge-landing, conflict-resolver, tui, sandbox-hardening]
requires: [quick-261103-f5y, quick-261103-gt1]
provides:
  - any number of concurrent one-offs per project (per-run `run.lock`, atomic run-id claim, `--run-id` on every verb, `task list`)
  - the run's work tracked in the user's repo as `gsd-oneoff/<run-id>` (deleted by landing and by discard, never overwritten)
  - merge landing: fast-forward, host-made merge commit in a private clone, or one sandboxed resolver session, with host re-checks, a final user-run `--ff-only`, at most 3 attempts, spend and failure recording
  - multi-run Driver-tab UI: counts line, run list (`T`), per-run actions, `resolved by agent` marker, merge diff (`d`), resolver viewer (`r`)
  - data-root and config-root masks in the one-off and driver bwrap profiles (D-09)
key-files:
  created:
    - src/oneoff/merge.rs
    - src/oneoff/resolve.rs
    - tests/launcher_sandbox_data_mask.rs
  modified:
    - src/oneoff/{store,launch,land,inbox,session,stream,prompt,mod}.rs
    - src/launcher/{sandbox,config_dir,run}.rs
    - src/{cli,task_cli,action,app}.rs
    - src/ui/screens/{mod,oneoff,oneoff_viewer,detail,help,render_escape_guard}.rs
    - tests/{oneoff_e2e,oneoff_runtime_seam_guard}.rs
    - docs/CONFIGURATION.md
    - README.md
  moved:
    - .planning/todos/pending/2026-11-03-oneoff-concurrent-runs-per-project.md -> completed/
    - .planning/todos/pending/2026-11-03-oneoff-run-history-list.md -> completed/
decisions: see "Inferred decisions" below
metrics:
  tasks: 3
  completed: 2026-11-03
status: complete
actuals:
  tasks: 3
  commits: 3
plan_head_before: 940ff0c62a708505e1bb3d30a76fc1a08a51c069
plan_head_after: 01846deb27d9e7093de7133627736ece895739a8
---

# Quick 261103-hx7: parallel one-offs per project, tracked branches, LLM-resolved merges

A project can now run any number of one-offs at once. Each finished run is visible in the
user's repo as `gsd-oneoff/<run-id>`. Landing never rebases: it fast-forwards, makes a merge
commit in a private clone, or hands conflicts to one short sandboxed resolver session whose
output the host re-derives before the user's own `git merge --ff-only` touches the checkout.
The Driver tab lists the runs and acts on the selected one. The bwrap profiles mask the
app's data and config roots wherever XDG puts them. Everything planned shipped; nothing is
deferred and no fallback todo was needed.

## Commits (master, nothing pushed, nothing tagged)

| Task | SHA | Message |
|------|-----|---------|
| 1 (tracer) | `02954ff` | feat(quick-261103-hx7): parallel one-offs with per-run locks, tracked branches and a data-root mask |
| 2 | `5e8de7b` | feat(quick-261103-hx7): merge landing with a sandboxed conflict resolver and bounded retries |
| 3 | `01846de` | feat(quick-261103-hx7): multi-run one-off TUI with a run list, merge diff and resolver view |

`git rev-list --count 940ff0c..HEAD` is 3 at the Task 3 commit (`commits: 3` is measured,
not narrated). The quick dir's pre-dispatch artifacts (`940ff0c`) are the base and the
orchestrator's.

## Shipped vs deferred

Shipped (all of it):
- **Task 1:** per-run lock with legacy fallback, atomic run-id claim, `--run-id` on
  status/stop/send/budget/attach, `task list`, finalize-time branch tracking, discard and
  landing branch deletion, the data/config-root mask (goldens kept), docs.
- **Task 2:** shared sandboxed-session runner, `merge.rs` (pure) and `resolve.rs` (I/O), the
  non-ff landing with resolver, host checks, final ff, 3-attempt bound, spend and failure
  recording, land lock, docs.
- **Task 3:** counts line, run list, per-run actions, `resolved by agent` marker and
  conflicted files, landing/land-failure display, merge diff (`d`) and resolver viewer (`r`),
  `D` for discard, help rows, docs, the two todo moves.

Todos: `oneoff-concurrent-runs-per-project` moved to `completed/` (Task 1 delivered
concurrency; the TUI half landed in Task 3). `oneoff-run-history-list` moved to
`completed/`: the list really shows finished runs with their outcome (unit test
`the_list_shows_runs_newest_first_with_their_outcome_and_landing`) and `task list` is the CLI
half; both carry `resolved_by: quick 261103-hx7`. `oneoff-dashboard-running-badge` stays
pending: it needs a dashboard-wide scan, a new cache field and its own render-escape and
flag-off tests, not "a few lines" (C-31); its text was corrected in Task 1 (I-23).

Deferred: none. New follow-up todos: none written.

## Verification

- Final raw suite, `rtk proxy cargo test --no-fail-fast` (redirected to a file, not piped):
  **3434 passed; 0 failed; 17 ignored** (59 result lines, exit 0; was 3415 after Task 2). `oneoff_e2e`: 26 passed. The run's per-binary lines show no missing suite versus `t2-full.log`.
- `cargo clippy --all-targets -- -D warnings`: clean for the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`. `cargo fmt --check`: clean.
- `git diff --exit-code 8aa41ee -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs
  tests/launcher_push.rs tests/spawn_seam_guard.rs` is empty (S-03): no driver golden or
  guard edited.
- `oneoff_runtime_seam_guard` (lists `merge.rs`, `resolve.rs`), `spawn_seam_guard`,
  `async_blocking_guard`, the render-escape guard (new List, Result-with-resolved-landing,
  Diff and Diff-error states with hostile ids, headlines, conflicted names, failure reasons
  and diff lines) and `driver_router_conformance` are green.
- e2e on the fake harness (`tests/oneoff_e2e.rs`): two concurrent runs, tracked branch and
  discard, a second run while one is live, the data-root mask outside `$HOME`, a clean merge
  commit, a resolver-resolved merge (budget 8.5, spend 2.0), a failed resolution, leftover
  markers, a moving target and a target that keeps moving, no budget left, a concurrent
  landing refused. Task 3 added `land::merge_diff` assertions to the clean-merge and the
  resolved-merge cases (real git, header, diff body, a non-sha refused).
- Task 3 unit tests: `run_counts` (store), `diff_lines` (merge), the run list, selection by
  id across a reorder, Enter/Esc walk, per-run actions naming the selected run, `r` and `d`
  routing, diff scrolling and error, hostile run escaping (oneoff.rs); resolver stream
  refuses `i`/`b` and names the attempt (viewer); diff request, flag-off inertness, viewer
  open refusals and the resolver attempt dir (app.rs); counts line (detail.rs).

## Deviations from the plan

- **[Rule 1 - flaky test] `store::fs_store::tests::a_run_lock_is_preferred_and_the_legacy_project_lock_is_the_fallback`**
  (a Task 1 test) failed once in a full run: right after `drop(project_lock)` a read still
  saw the lock held, because a process another test forks at that moment holds a copy of the
  flock's open file description until its exec. It polls up to 5 s now instead of asserting
  once. Passed 6 of 6 alone before the change. Files: `src/oneoff/store.rs`.
- `List { selected }` is a unit `List` mode; the selection is the screen's `run_id` (so a
  reordering refresh keeps it by id without extra state). `Ask` carries the asked run id and
  where `n`/Esc return to, so `y` can never act on a different run than the one asked about.
- `OneoffStatusRefreshed` sends `runs: Vec<OneoffView>`; `read_latest` was removed (no
  reader left). `RunRecord::last_merge_landing()` is shared by the screen and the app.
- `merge::diff_lines` (pure, unit-tested) does the capping; `land::merge_diff` is the thin
  hardened-git read. `OneoffStream::with_attempt` (builder) instead of a fifth `new`
  parameter, so no existing caller changed. The tail loop takes a `TailSpec` struct.
- The Task 1 `t`-while-live test inversion had already been done in Task 1 (C-28 pulled
  forward), so Task 3 did not need to touch it beyond the cache field rename.
- README untouched in Task 3 (its one-off lines make no single-run or key claim).

## Inferred decisions [inferred — audit]

Binding ones from CONTEXT (all `[inferred — audit]` there, carried as C-01..C-33) and the
planner's I-01..I-24 stand as written in `261103-hx7-CONTEXT.md` / `-PLAN.md`; the ones
worth auditing, one line each:

**CONTEXT / PLAN (as shipped)**
1. C-01/C-02: liveness is `<run-dir>/run.lock`; a legacy run is judged by the project lock;
   no id means the newest run, several live means refuse and list.
2. C-03/C-04: no concurrency cap; quick-id collisions accepted, the resolver unions STATE.md rows.
3. C-05/C-06: the branch is fetched at finalize only with commits > 0 and a clean planted gate
   (gate first), hardened fetch, verified against `record.head`, never overwritten.
4. C-07/C-08: landing runs a safe branch delete; discard deletes only while the ref equals the head.
5. C-11..C-19: fresh `--no-hardlinks` clone at the target tip, `merge --no-ff`, host-made
   commit on a clean merge, one resolver for conflicts, `taskctl report` unchanged with
   `--verified` required, host re-derives parents / clean tree / markers / planted gate, the
   user's own `--ff-only` last, at most 3 attempts, dirty means refuse and keep the clone.
6. C-22..C-25: resolver budget = run budget minus recorded spend, floor $0.25; 15 min per
   attempt; stream, log and report under `<run-dir>/land/<n>/`.
7. C-26..C-31: counts line; `T` list; `t` while live; `resolved by agent` marker; diff and
   resolver view; history todo moved only because the list shows finished runs.
8. C-32/C-33, I-06, I-07: mask both the data root and the config root (stored OAuth token) only
   when outside `$HOME` and `/tmp`; `bwrap_argv_masking`, no new `SandboxSpec` field.
9. I-01..I-05: atomic run-id claim, `live_seen` chooser, `--run-id` everywhere, `task list`,
   `task run` no longer exits 3.
10. I-09..I-20, I-24: `update-ref -d <ref> <head>` for discard; land git is `git_mirror_only`
    with an explicit identity; the resolver is never unsandboxed; in-process land with a
    land lock; marker rule (`<<<<<<<` / `>>>>>>>` at line start); `spend_usd` and
    `MIN_RESOLVER_BUDGET_USD`; test command read from the land clone's config; "head already
    in the target" refuses.

**Task 1 (progress notes)**
11. With no id and exactly one live run, verbs pick that run even if a newer one finished;
    zero live picks the newest; two or more that may be live refuse.
12. `task stop` on a project with no runs says so and exits 0; an unknown `--run-id` exits 1;
    a legacy project lock held by another run never gets signalled.
13. Tracking-note texts (no commits / planted / no head / mismatch / fetch failure, redacted,
    capped at 400); landing deletes the tracked branch only if it still resolves to the head.
14. `task list` line format; `store::live_lock_path`; `attach` follows the picked run by id.

**Task 2 (progress notes)**
15. The merge commit is pinned as `refs/gsdmm/land` in the land clone and fetched by that ref;
    FETCH_HEAD must equal the checked sha. The run head is merged by sha.
16. Conflicted files come from `git ls-files -u -z` (blanks in paths survive); more than 1000
    refuses. Parents check and final fetch/ff also run for clean merges.
17. `Landing.attempts` only for `resolved`; `land_failure.attempts` lists every attempt dir;
    a session's spend is `max(supervisor estimate, whole-stream fold)`; a resolver that fails
    to start still adds the spend found in its stream.
18. `LandOptions::with_signal_handlers` for the CLI, a never-set flag for the TUI;
    `landing_in_progress` treats an unprobeable lock as in progress.
19. Pre-resolver refusals write no `land_failure`; the resolver prompt says "the project's
    agent instructions" (seam guard); the budget flag is `f64::to_string` of the remainder.
20. README: "never commits" became "never commits in your checkout" plus one sentence on the
    private-clone merge.

**Task 3 (new)**
21. The list's selection is the screen's `run_id` (a unit `List` mode); a vanished selected run
    falls back to the newest in the list, but never in a run's own summary.
22. `Ask` remembers the asked run id and its origin (list or summary); `m` is refused while the
    run's land lock is held ("already in progress") instead of asking.
23. `d` works from the list and from a summary, on the latest `merge`/`resolved` landing that
    recorded a sha (a later push does not hide it); Esc in the diff returns to the summary.
24. The counts line: `landing` is its own count (a run being landed is not "to land"); `to
    land` is not live, head present, commits > 0, no `Merged` landing (a pushed-only run still
    counts); `N finished` is the total listed when nothing else is left to say.
25. The summary shows `Branch:` only while no merge landed (the record keeps the name after a
    landing deleted the branch), a `Spend:` line from `spend_usd`, and for a landing the
    merge sha (12), `(merge commit)` / `(fast-forward)` and, when resolved, the marker,
    conflicted files and attempts.
26. The diff is a full-width view with a windowed body (no wrap), lines cut at 1000 chars,
    coloured by prefix, headed by `conflicted: ...` (`none (a clean merge)` otherwise);
    `g`/`G`/Home/End jump; `q` or Esc leaves.
27. `r` opens `land/<newest>`; the resolver viewer has no soft cap and no composer, hides the
    run's elapsed clock (it is not the resolver's), shows `resolver land/<n>`, and is live only
    while the run's land lock is held and it is the newest attempt. Its meter denominator is
    the run's whole budget, not the resolver's remainder.
28. The Driver tab line is only the counts (the old per-run headline line is gone); the list
    window is sized to the screen and scrolls around the selection.
29. Help rows: `T` lists the new keys, `v` mentions `r`, `t` says it starts another run even
    while one is live; the three copies are identical.

## Audit items worth a human look

- **The merge diff reads git's whole output before capping.** `git_in` collects stdout in
  memory (60 s budget) and `diff_lines` then cuts at 5000 lines / 512 KiB. A merge that
  changes a huge generated file costs that memory on a `spawn_blocking` thread once per `d`.
  A streaming cap would need a new bounded-read helper in the process seam.
- **Resolved merges deserve a human diff review** (D-06): the host proves parents, tree
  cleanliness, absence of conflict markers and a clean planted gate, not that the resolution
  is semantically right. The tests claim lives in the agent's `--verified` text only (T-hx7-05).
- A landing's `tracked_branch` is not cleared in `run.json` after the branch is deleted; the
  UI and `task list` read it as "branch exists" only while no `Merged` landing is recorded.
- The unreliable-by-design `launcher_bridge::the_bridge_pumps_tcp_to_the_egress_socket`
  (port race) is the only known flake; one Task 1 lock test was hardened (above).
- The TUI cannot cancel a running resolver: quitting the TUI ends it through bwrap's
  `--die-with-parent` and nothing lands; `task land merge` in a terminal honours Ctrl-C.
- Concurrent first-ever runs can race in `push::prepare_mirror`; the loser records a harmless
  "no push mirror" warning (from Task 1).

## Unverified without a paid run (verified against the fake agent only)

- The real resolver's merge behaviour: that a real agent resolves a real conflict, commits a
  two-parent merge with `git commit --no-edit`, runs the project's tests and reports through
  `taskctl report --verified`; and its STATE.md table-row union.
- Real `--max-budget-usd` enforcement on a resolver session, and the 15-minute wall clock
  against a stuck real agent.
- Real systemd scope and bwrap behaviour for the resolver's `<run-id>-land<n>` unit and for the
  data/config-root `--tmpfs` masks on a host whose XDG roots are outside `$HOME` (proved on
  the fake bwrap's argv log and the goldens, not by a real sandbox escape attempt).
- The TUI screens were exercised through the unit/render tests and the render-escape guard,
  not by a person at a real terminal (key feel, resize behaviour of the windowed list and the
  full-screen diff).

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model (T-hx7-01..11). The new Task 3 surface is the merge-diff
read (hardened `git diff` in the project, `--no-ext-diff --no-textconv`, a validated sha) and
the TUI drawing run-derived text, which T-hx7-10 covers and the render-escape guard fixture
now exercises.

## Self-Check: PASSED

- Files exist: `src/oneoff/{merge,resolve}.rs`, `tests/launcher_sandbox_data_mask.rs`,
  `src/ui/screens/oneoff.rs`, both todos under `.planning/todos/completed/` (the pending
  copies are gone), and the badge todo is still pending.
- Commits `02954ff`, `5e8de7b`, `01846de` are ancestors of HEAD (master).
- The four driver golden/guard files are byte-identical to `8aa41ee`.
