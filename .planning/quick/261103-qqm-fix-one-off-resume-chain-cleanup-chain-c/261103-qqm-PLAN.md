---
quick_id: 261103-qqm
mode: quick
phase: quick-261103-qqm
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/oneoff/store.rs
  - src/oneoff/recycle.rs
  - src/oneoff/land.rs
  - src/oneoff/launch.rs
  - tests/oneoff_e2e.rs
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-261103-qqm]

estimate:
  tokens: 300000
  raw_tokens: 300000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Run `task land merge <alias> <successor>` on a run that was resumed from an earlier run. Afterwards every predecessor in its resume chain whose head is in the branch the successor landed in is closed out, as I-01, I-02 and I-04 specify: it has a landing of kind `already-landed` whose detail is that branch, `gsd-oneoff/<pred>` is deleted, and `pruned` is set. Its run dir has no `work/`, no `run.lock` and no `land.lock`. The landing output names each such predecessor id followed by `recorded as landed`."
    - "The next scan (`task list`, `task run`, TUI start) closes out the same way any predecessor that an earlier landing left untouched. This is the state of `oneoff-20261104-005114`: the successor is recorded `merged`, and the predecessor is paused, still has its clone and branch, and has `resumed_by` set. `task list` no longer prints `the run has not landed` for any resumed run (I-02, I-03)."
    - "A predecessor keeps its clone, its `gsd-oneoff/<id>` branch and its unlanded record, and no scan slims it or warns about it, when its successor was discarded, failed its resume merge, was only pushed, or has not landed (I-02, I-03)."
    - "The fast-forward landing message counts agent commits across the whole resume chain (I-05). A `--from branch` chain of one agent commit per run prints `landed 2 commit(s) of <successor> on <branch> (now at <sha>)`. The `--from master` e2e prints `landed 3 commit(s) of <successor> ... (2 by the agent; ...)`, because the host merge commit is the only commit not made by the agent."
    - "After a run ends, its run dir has no `run.lock` (I-06). The launcher unlinks the file after the final record write, while it still holds the lock, and only when the path still names the inode it locked. The scan removes a free `run.lock` from every finished record that has no landing in progress, before it does anything else with that run."
    - "Clones are deleted only through `clone_preserved`. Branches are deleted only through the safe `git branch --delete` at the recorded head. Every chain link is confirmed in both directions (`resumed_by` and `resumed_from`), and every walk is bounded (I-07). `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings` are clean."
  artifacts:
    - path: src/oneoff/store.rs
      provides: "resume_successors / resume_predecessors: bounded, link-checked walks over the resume chain (MAX_RESUME_CHAIN)"
    - path: src/oneoff/recycle.rs
      provides: "successor_landing_branch (pure), contained_candidate, close_out_predecessors, clear_stale_run_lock; tend_run and tend_contained use contained_candidate; the o2r I-17 pre-landing slim of resumed runs is removed"
    - path: src/oneoff/land.rs
      provides: "merge() closes out the chain after the successor's slim; chain_agent_commits feeds landed_message; unlink_if_held is shared by LandLock::drop and the launcher"
    - path: src/oneoff/launch.rs
      provides: "run_foreground unlinks run.lock while it holds it, before it releases"
    - path: tests/oneoff_e2e.rs
      provides: "resume_chain_* and run_lock_* e2e tests on the fake harness; the --from master expectation updated to (2 by the agent"
    - path: docs/CONFIGURATION.md
      provides: "Resume, ff landing message, Several at once and Cleanup and recovery text updated"
  key_links:
    - from: src/oneoff/land.rs
      to: src/oneoff/recycle.rs
      via: "merge() calls recycle::close_out_predecessors(project, run_id) after recycle::slim_run, which calls tend_contained for each predecessor"
      pattern: "close_out_predecessors"
    - from: src/oneoff/recycle.rs
      to: src/oneoff/store.rs
      via: "contained_candidate = already_landed_candidate, or else successor_landing_branch(record, &resume_successors(project_dir, record))"
      pattern: "resume_successors"
    - from: src/oneoff/recycle.rs
      to: src/oneoff/land.rs
      via: "tend_contained records each predecessor through land::record_already_landed (branch delete included) under try_land_lock, then slim_locked"
      pattern: "record_already_landed"
    - from: src/oneoff/launch.rs
      to: src/driver/lock.rs
      via: "run_foreground calls land::unlink_if_held(&lock, &run_lock_path) before drop(lock), which checks RunLock::is_at"
      pattern: "unlink_if_held"
---

<objective>
Fix the three defects that the paid one-off test of 2026-11-04 exposed (prior context: quick 261103-o2r and 261103-pd7 SUMMARYs).

1. **Resume-chain predecessors are never cleaned up.** Run `oneoff-20261104-005114` (paused, 119 MB with its clone, `resumed_by` = `oneoff-20261104-010937`) kept its clone and its `gsd-oneoff/<id>` branch after its successor landed. Every `task list` warns `...: the run has not landed`. The cause has three parts:
   - `leftover_slim_note` (src/oneoff/recycle.rs:596) asks to slim a resumed run whose branch is at its head.
   - `tend_run` calls `slim_run`, which uses `require_landing = true` (recycle.rs:338, 742-743), and `slim_locked` refuses a run with no landing (recycle.rs:359-360).
   - `already_landed_candidate` skips resumed and paused runs (recycle.rs:767-768), so nothing deletes the branch either.
2. **The ff landing message misattributes commits.** It prints `(1 by the agent)` because `merge_locked` passes only the successor's own `commits` (land.rs:804, 969-975), although the predecessor's commit was made by the agent too.
3. **`run.lock` is left in a finished run dir.** `run_foreground` drops the lock without unlinking it (launch.rs:325-326). The scan's free-`run.lock` removal sits in `tidy_finished` (recycle.rs:888-890), and `tend_run` never reaches it when a slim is Kept or Skipped.

Purpose: when the work of a resume chain lands, every run in the chain closes out the way a single landed run already does. The landing names what the agent made. A finished run leaves no lock files.

Output:
- code and unit tests in src/oneoff/{store,recycle,land,launch}.rs;
- e2e tests in tests/oneoff_e2e.rs;
- docs/CONFIGURATION.md updates;
- `261103-qqm-SUMMARY.md`.

Out of scope: the real-data cleanup of `oneoff-20261104-005114`, which the orchestrator runs after execution. No paid `claude` runs: use the fake harness and temp git repos only.

## Inferred decisions (human unavailable) `[inferred — audit]`

- **I-01** `[inferred — audit]` Record each predecessor with the existing `LandingKind::AlreadyLanded`, through `land::record_already_landed` (detail = the branch, merge_sha from `containing_commit`, target_tip). Do not add a new kind. The predecessor's head IS in that branch, which is what this kind records. `task list` and `task status` already show the successor link (`resumed as <id>`). No new serde variant means no forward-compat break, and no new list or TUI vocabulary is needed.
- **I-02** `[inferred — audit]` A predecessor is closed out only when both of these hold:
  - A run later in its resume chain has a landing that puts work on the target (`puts_work_on_target`: Merged or AlreadyLanded). The chain is followed through `resumed_by`, and each link is confirmed by the successor's `resumed_from`.
  - The predecessor's head is an ancestor of that landing's branch tip (`merge-base --is-ancestor` on the project's own objects).

  A successor that was only pushed does not close out its predecessors: a push puts nothing on a project branch. The predecessor's own outcome (Paused) and commit count (0) do not disqualify it: once the chain has landed, a 0-commit predecessor's clone is just as dead.
- **I-03** `[inferred — audit]` Remove o2r's I-17 rule ("slim a resumed run when `gsd-oneoff/<id>` is at its head"). A predecessor then keeps its clone and branch until a successor lands, so a discarded or failed successor leaves it whole and resumable, as the brief asks. This also ends the per-scan `the run has not landed` warning: since the rule never took effect, removing it changes nothing else that can be observed.
- **I-04** `[inferred — audit]` At landing time, `land::merge` closes out the chain right after the successor's own slim. It uses the same routine as the scan (`tend_contained`), with the predecessor's land lock taken without waiting. A held lock skips that predecessor and the next scan finishes it. This also covers the resolver path, the already-landed path, the TUI and `task resume`'s retried landing, because all of them go through `land::merge`. `land::push` does not trigger it.
- **I-05** `[inferred — audit]` The agent count is the sum, over the landed run and its predecessors, of `git rev-list --count <head> ^<base> ^<old_tip>` in the project. This counts only the commits this landing brought in, so commits already on the target are not counted twice. Host merge commits are excluded, because a `--from master` successor's `base` is the merge commit (launch.rs:646-649). A member whose count fails, or that has no base or head, contributes its recorded `commits`.
- **I-06** `[inferred — audit]` `run.lock` handling:
  - The launcher unlinks `run.lock` in `run_foreground` after the final record write, while it still holds the lock, and only when `RunLock::is_at(path)` holds. This is the same helper that `LandLock::drop` uses (pd7 D-05).
  - Early returns between acquisition and `run_session` leave no `run.json`. The orphan rule handles those dirs.
  - The scan removes a free `run.lock` of every finished record with no landing in progress, before it does anything else in `tend_run`. The removal moves out of `tidy_finished`.
  - Without the file, liveness falls back to the legacy per-project lock path. Every slimmed run already reads that way today (slim_locked recycle.rs:479-482), so this changes no reader.
- **I-07** `[inferred — audit]` Chain walks:
  - Each walk stops after `MAX_RESUME_CHAIN = 32` links and keeps a visited set.
  - Ids come from `run.json` and reach the filesystem only through `read_view`, which checks `valid_oneoff_run_id`.
  - A landing detail is used as a branch only when `push::plausible_branch` accepts it and `refs/heads/<detail>` resolves.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/quick/261103-pd7-fix-one-off-sandbox-sensitive-tests-alre/261103-pd7-SUMMARY.md
@.planning/quick/261103-o2r-one-off-run-cleanup-crash-recovery-and-l/261103-o2r-SUMMARY.md

Interfaces the executor needs (read these regions, not whole files):
- src/oneoff/recycle.rs:321-487: `slim_run(project, run_id, note) -> SlimOutcome` and `slim_locked(project, view, note, require_landing)`. The free run.lock unlink is at 479-482.
- src/oneoff/recycle.rs:577-598: `leftover_slim_note(record, clone_present, tracked_at_head)`. Lines 596-597 hold the I-17 clause.
- src/oneoff/recycle.rs:716-850:
  - `tend_run`;
  - `already_landed_candidate(record) -> Option<String>`;
  - `tend_contained(project, project_dir, id, branch, report) -> bool`, which re-checks under `try_land_lock` at 811-820.
- src/oneoff/recycle.rs:852-906: `tidy_finished`, whose run.lock removal is at 888-890.
- src/oneoff/recycle.rs tests:
  - `fixture()` at 1170 and `landed_view` at 1368;
  - the leftover test at 1315-1341 (its I-17 lines are 1336-1340);
  - the candidate test at 1527-1584.
- src/oneoff/land.rs:
  - 107-159: `LandLock`, whose Drop unlinks when `lock.is_at(path)`, and `take_land_lock` / `try_land_lock`;
  - 775-785: `merge()`;
  - 788-996: `merge_locked`. The ff message is built at 959-975, with `old_head` at 925;
  - 1024-1065: `record_already_landed`;
  - 1067-1089: `landed_message(landed, agent, run_id, branch, short)`;
  - 1129-1150: `clear_tracking_after_landing`;
  - private `git_in(dir, args)` at 358.
- src/oneoff/store.rs:
  - 625-639: `derive_live`;
  - 801-808: `live_seen`, the run.lock-or-legacy choice;
  - 913-934: `read_view(project_dir, run_id) -> Option<OneoffView>`, which validates the id;
  - `RunRecord.resumed_from` and `resumed_by` at 313-316;
  - the `tests::record()` helper.
- src/oneoff/launch.rs:263-277 (run.lock acquire in `run_foreground`) and 325-326 (`drop(lock)` after the final write); 362-396 (a resume's base is the old head); 646-649 (a `--from master` resume moves base to the merge sha).
- src/driver/lock.rs:126-153: `RunLock` and `is_at(&self, path) -> bool`. `acquire_at(path, run_id, pid)` is public to integration tests as `gsd_meta_manager::driver::lock::acquire_at`.
- src/launcher/push.rs:217: `plausible_branch(&str) -> bool`. `push::git_in(dir, args, budget)` is used by recycle.
- tests/oneoff_e2e.rs helpers:
  - `paused_run` 1851, `resumed_scenario` 2138, `resumed_id` 2159;
  - `wait_run_finished` 366, `status_of_run` 359, `run_dir` 85, `branch_head` 380;
  - `texts` 2439, `dir_size` 2426, `lock_files` 3106.
  - Pattern tests: `a_paused_run_resumes_from_its_branch_as_a_new_run` (2173-2257), `resume_from_master_merges_the_checkout_tip_into_the_run_first` (2260-2306; its `(1 by the agent` assertion is at 2305) and `already_landed_runs_are_recorded_by_the_scan` (3187-3222).

Baselines measured at HEAD 69c1c4f:
- `oneoff_e2e` lists 61 tests, and 0 of them match `resume_chain` or `run_lock`.
- No identifier introduced by this plan exists yet in src/ or tests/.
- The last full run (pd7) was 3577 passed, 0 failed, 17 ignored, over 59 result lines.

Project rules:
- Use `rtk proxy` for raw cargo output, and run `--no-fail-fast` so the envelope suites run (see memory notes). `tests/spawn_seam_guard.rs` counts `Command::new` under src/, so unit tests drive git through `launcher::push::git_in` (o2r deviation).
- Do not rebase, and do not touch the real data dir under `~/.local/share/gsd-meta-manager`.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: A landed successor closes out its resume chain and counts the chain's agent commits</name>
  <files>src/oneoff/store.rs, src/oneoff/recycle.rs, src/oneoff/land.rs, tests/oneoff_e2e.rs</files>
  <action>
Wire the one landing path end to end (I-01, I-02, I-04, I-05, I-07). Start red: first write the e2e test below and watch it fail at HEAD (the predecessor keeps `work/` and its branch, and the message says `(1 by the agent`).

1. **Walks, in src/oneoff/store.rs next to `read_view`.**
   - Add `pub const MAX_RESUME_CHAIN: usize = 32`.
   - Add `pub fn resume_successors(project_dir: &Path, record: &RunRecord) -> Vec<RunRecord>`. It follows `resumed_by` from `record` and returns the records in chain order. It stops at:
     - a missing or unreadable view (a discarded successor);
     - a successor whose `resumed_from` is not the id it was reached from;
     - an id already visited;
     - `MAX_RESUME_CHAIN` links.
   - Add `pub fn resume_predecessors(project_dir: &Path, record: &RunRecord) -> Vec<OneoffView>`. It does the same walk through `resumed_from`, immediate predecessor first, and requires `pred.record.resumed_by == Some(child id)`.
   - Ids reach the filesystem only through `read_view`, which validates them.

2. **Candidates, in src/oneoff/recycle.rs.**
   - Add pure `pub(crate) fn successor_landing_branch(record: &RunRecord, successors: &[RunRecord]) -> Option<String>`.
     - It returns `None` unless all of these hold for `record`: status Finished, `head` is some, `resumed_by` is some, and no landing has `puts_work_on_target`.
     - It then walks `successors` and re-checks each link in both directions (`run_id` against the expected `resumed_by`, `resumed_from` against the previous id). It stops at the first mismatch.
     - For the first successor that has a landing with `puts_work_on_target` (take the latest such landing), it returns that landing's `detail`, but only when `push::plausible_branch(detail)` holds. Otherwise it returns `None`.
     - Outcome Paused and `commits` of 0 or `None` do NOT disqualify the record (I-02).
   - Add `pub(crate) fn contained_candidate(project_dir: &Path, record: &RunRecord) -> Option<String>`, defined as `already_landed_candidate(record)` or, failing that, `successor_landing_branch(record, &resume_successors(project_dir, record))`.
   - Leave `already_landed_candidate` unchanged. It still excludes resumed and paused runs for the plain path.
   - In `tend_contained`, replace the under-lock re-check `already_landed_candidate(&view.record).as_deref() != Some(branch)` with `contained_candidate(project_dir, &view.record).as_deref() != Some(branch)`. Everything else in `tend_contained` stays as it is: the ancestry test, `try_land_lock`, `record_already_landed` (which deletes the tracked branch) and `slim_locked(.., true)`.

3. **Close-out, in recycle.rs.** Add `pub fn close_out_predecessors(project: &Path, run_id: &str) -> Vec<String>`:
   - Resolve `project_dir_for(project)` and `read_view(run_id)`.
   - For each view in `resume_predecessors`, when `contained_candidate(project_dir, &pred.record)` gives a branch, call `tend_contained(project, project_dir, &pred_id, &branch, &mut report)`.
   - Return `report.warnings`. These already read `<id>: its head is already in <branch>; recorded as landed`, plus any slim note or kept-clone line. Add one line `<id>: removed its clone` for each predecessor that was slimmed. You may append `(resumed as <successor>)` to the recorded line.
   - Never return an error.

4. **Landing hook, in src/oneoff/land.rs `merge()`.** After `slim_run`, append the lines from `super::recycle::close_out_predecessors(&project, run_id)` to the returned text, newline-joined.

5. **Agent count (I-05), in land.rs.**
   - Add `fn chain_agent_commits(project: &Path, record: &RunRecord, old_tip: &str) -> u32`. The members are `record` plus `resume_predecessors(project_dir_for(project), record)`.
   - For each member with `head` and `base`, run private `git_in(project, ["rev-list", "--count", head, "^base", "^old_tip"])` and parse the count. On any failure, or when `head` or `base` is missing, use `commits.unwrap_or(0)`. Add the counts with saturation.
   - In `merge_locked`'s ff path, pass `chain_agent_commits(project, record, &tip)` as `landed_message`'s `agent` argument instead of `facts.commits`. `tip` is the pre-ff target tip that is already used as `old_head`.
   - `facts.commits`, and therefore `merge_decision`, stays the run's own count.
   - Update the comment at land.rs:959-961.

6. **Tests.**
   - Add e2e `resume_chain_landing_closes_out_the_predecessor` in tests/oneoff_e2e.rs, modelled on `a_paused_run_resumes_from_its_branch_as_a_new_run`:
     - `paused_run`, then `resumed_scenario("  :")`, then `resume --from branch --detach`, then release, then `wait_run_finished(new)`.
     - `land merge ALIAS <new>` exits 0. Its stdout contains `landed 2 commit(s) of <new>` and does not contain `by the agent`. The output also contains `<old>` and `recorded as landed`.
     - The old run's record has `landed[0].kind == "already-landed"`, `landed[0].detail == "main"` (or the fixture's branch name), and `pruned.at` is a string.
     - The old run dir has no `work/`, no `run.lock` and no `land.lock`, and `dir_size` is under 1 MB. `branch_head(project, old)` is `None`.
     - `list ALIAS` exits 0, and its stderr does not contain `has not landed`.
     - A second `list` leaves the old run.json byte-identical, and `lock_files` stays empty.
   - In `resume_from_master_merges_the_checkout_tip_into_the_run_first`, change the assertion `(1 by the agent` to `(2 by the agent`, and update the comment above it: the old run's wip commit is the agent's, and the host merge commit is not.
   - Add store.rs unit tests named `resume_chain_walk_*`. Use temp project dirs and `write_record`. Cover a forward walk across three runs, stopping at a missing dir, stopping at a `resumed_from` mismatch, and a cycle that stays bounded.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e resume_chain_landing_closes_out_the_predecessor</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e resume</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e already_landed</automated>
    <automated>rtk proxy cargo test --no-fail-fast --lib -- oneoff::</automated>
    <automated>rtk proxy cargo test --test oneoff_e2e resume_chain -- --list 2>/dev/null | rtk proxy grep -c ': test$'</automated>
  </verify>
  <done>
- The tracer e2e test passes. It failed at HEAD; record the failing line in the SUMMARY.
- All `resume` and `already_landed` e2e tests pass, including the updated `(2 by the agent` expectation.
- `oneoff::` lib tests pass, and the `resume_chain_walk_*` tests exist and pass.
- The `--list` gate prints at least 1 (baseline 0).
- Commit: `fix(quick-261103-qqm): close out a resume chain when its successor lands and count the chain's agent commits`.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: The scan closes out stranded predecessors, keeps unlanded chains, and no run.lock outlives its run</name>
  <files>src/oneoff/recycle.rs, src/oneoff/land.rs, src/oneoff/launch.rs, tests/oneoff_e2e.rs</files>
  <behavior>
    - successor_landing_branch: a Merged successor (detail main) gives Some("main"); an AlreadyLanded successor gives Some; a Pushed-only or unlanded successor gives None; a landing two hops away (P1, then unlanded P2, then landed S) gives Some; a successor whose resumed_from names another run gives None; a record already landed, not Finished, without head or without resumed_by gives None; a Paused record with commits 0 still gives Some; an implausible detail gives None.
    - leftover_slim_note: a resumed, unlanded run with its clone gives None (the I-17 clause is gone); landed and pruned cases are unchanged.
    - clear_stale_run_lock: a free run.lock of a finished run is removed and counted in stale_files; a held one (taken with acquire_at in the test) stays.
    - e2e: the scan closes out a predecessor that a landing skipped; a discarded successor leaves its predecessor whole; a finished run has no run.lock before any scan runs.
  </behavior>
  <action>
Write the unit tests from `<behavior>` first and run them red. Then implement the following (I-02, I-03, I-06).

1. **Scan wiring, in src/oneoff/recycle.rs `tend_run`.**
   - Replace `already_landed_candidate(&view.record)` with `contained_candidate(project_dir, &view.record)`. The scan then closes out stranded predecessors through `tend_contained`: under `try_land_lock`, it records them already-landed, deletes their branch and slims them.
   - Remove the I-17 clause from `leftover_slim_note` (recycle.rs:596-597) together with its `tracked_at_head` parameter, and drop the `tracked_at_head` ref read from `tend_run` (recycle.rs:737-741).
   - Update the doc comments on `leftover_slim_note` and `tend_run` to say: a predecessor waits for its successor's landing (I-03).
   - Update the test at recycle.rs:1315-1341, whose resumed case now asserts `None`.

2. **Stale run.lock in the scan (I-06).**
   - Add `fn clear_stale_run_lock(run_dir: &Path, report: &mut RecycleReport)`. When `probe_at(run.lock) == LockProbe::Free`, it removes the file with `fs::remove_file` (a symlink is removed as a link) and adds 1 to `report.stale_files`. It leaves a held, unknown or absent lock alone.
   - Call it in `tend_run` right after the `live != Finished || landing` gate and before any other branch.
   - Delete the duplicate removal from `tidy_finished` (recycle.rs:888-890). The unlink in `slim_locked` (recycle.rs:479-482) stays.

3. **Run-end unlink (I-06), in src/oneoff/land.rs and src/oneoff/launch.rs.**
   - Extract `pub(crate) fn unlink_if_held(lock: &RunLock, path: &Path)` from `LandLock::drop`: when `lock.is_at(path)`, it removes the file and ignores the error. `LandLock::drop` then calls it.
   - In `run_foreground` (launch.rs), keep the run.lock path in a local. Call `super::land::unlink_if_held(&lock, &path)` immediately before `drop(lock)`, after the failure-record write.
   - Update the comment `Lock last: ...` to say the file is unlinked while held, then released.
   - Do not change `run_detached`'s poll. A run that ends before the poll sees the lock is still reported as started by the successful-exit branch, launch.rs:1324.

4. **E2e tests in tests/oneoff_e2e.rs.** Names must contain `resume_chain` or `run_lock`.
   - `resume_chain_predecessor_left_by_an_older_landing_is_closed_by_the_scan` reproduces the real `005114` state:
     - Paused run, resume, release, `wait_run_finished(new)`.
     - Hold the old run's `land.lock` with `gsd_meta_manager::driver::lock::acquire_at(&run_dir(&fx, &old).join("land.lock"), &old, std::process::id())`, then `land merge ALIAS <new>` (exit 0).
     - Assert the old run is untouched: `landed` is empty, `work/` exists, and `branch_head(old)` is some.
     - Drop the lock, then run `list ALIAS`. Its stderr contains `<old>` and `recorded as landed`, and does not contain `has not landed`.
     - The old run is now `already-landed`: pruned, no `work/`, no `run.lock`, branch gone.
     - A second `list` leaves its run.json identical, and `lock_files` is empty.
   - `resume_chain_discarded_successor_keeps_the_predecessor`:
     - Paused run, resume, release, `wait_run_finished(new)`.
     - `land discard ALIAS <new>`. Find the exact argv in `a_finished_run_is_tracked_as_a_branch_and_discard_deletes_it`.
     - `list ALIAS` exits 0. Its stderr does not contain `has not landed` and contains no `recorded as landed` line for `<old>`.
     - The old run's `landed` is empty, `pruned` is null, `work/` exists, and `branch_head(old)` equals the old head.
   - `run_lock_is_removed_when_a_run_ends`:
     - `quick_run(&fx)`, then assert `run_dir(&fx, &id).join("run.lock")` does not exist. No `list` has run since the run finished.
     - `paused_run(&fx)`, then assert the same for the paused run.
   - The `<behavior>` unit tests go in the recycle.rs test module, named `resume_chain_*` and `stale_run_lock_*`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e resume_chain</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e run_lock</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e recycle_</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e the_scan_leaves_an_unlanded_run_alone</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test oneoff_e2e no_land_lock_is_left</automated>
    <automated>rtk proxy cargo test --no-fail-fast --lib -- oneoff::</automated>
    <automated>rtk proxy cargo test --test oneoff_e2e -- --list 2>/dev/null | rtk proxy grep -cE '(resume_chain|run_lock).*: test$'</automated>
    <automated>rtk proxy cargo test --lib -- --list 2>/dev/null | rtk proxy grep -cE '(resume_chain|stale_run_lock).*: test$'</automated>
  </verify>
  <done>
- The new unit tests were red before the implementation and are green after it.
- The e2e filters `resume_chain`, `run_lock`, `recycle_`, `the_scan_leaves_an_unlanded_run_alone` and `no_land_lock_is_left` all pass.
- The e2e `--list` gate prints at least 4 (baseline 0), and the lib gate prints at least 3 (baseline 0).
- Commit: `fix(quick-261103-qqm): scan closes out stranded resume predecessors and run.lock goes with its run`.
  </done>
</task>

<task type="auto">
  <name>Task 3: Document the chain close-out and run.lock removal, then gate on the full suite</name>
  <files>docs/CONFIGURATION.md</files>
  <action>
1. **Docs, in docs/CONFIGURATION.md.**
   - **Resume paragraph** (around line 272): the old run's tracked branch stays "until a run later in its resume chain lands, or until its own discard". When the successor lands (`task land merge`, or the next scan for a landing made earlier), every predecessor whose head is in that branch is recorded `already-landed`, its `gsd-oneoff/<id>` is deleted and its clone is removed. If the successor is discarded, fails its resume merge or is only pushed, the predecessor keeps its clone and branch and can be resumed again (I-02, I-03).
   - **ff landing bullet** (around lines 433-437): the agent count now spans the resume chain, and only host merge commits are not the agent's. Change the example to `landed 3 commit(s) of <run> on <branch> (2 by the agent; now at <sha>)` (I-05).
   - **"Several at once"** (around line 521): the launcher takes `run.lock` before `run.json` exists, and removes and releases it last, once the record is final (I-06).
   - **"Cleanup and recovery" bullets** (around lines 563-575):
     - A free `run.lock` of any finished run is removed by the scan.
     - The "Runs already in their target" bullet now also covers resumed predecessors whose successor landed, as defined in I-02.
     - Replace "a pause or a successor are left alone" with: a paused run and a run whose successors have not landed are left alone.
     - The "Landed runs from before" bullet no longer implies that a resumed run is slimmed before its successor lands.
   - Run every file:line claim you add against the tree.

2. **Full gate.** Run the whole suite into `target/qqm-host.log`, then clippy and fmt. If a test fails, fix the cause in the task it belongs to; do not weaken a test. Then write `261103-qqm-SUMMARY.md`. It must cover:
   - the failing tracer line at HEAD;
   - the result-line, passed, failed and ignored counts;
   - each I-0x decision as applied;
   - any deviation;
   - a note that the real-data cleanup is left to the orchestrator: one `task list` on the project should close out `oneoff-20261104-005114` through the scan path that `resume_chain_predecessor_left_by_an_older_landing_is_closed_by_the_scan` reproduces.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast > target/qqm-host.log 2>&1 && echo SUITE-OK; rtk proxy grep -E '^test result:|FAILED|panicked at' target/qqm-host.log</automated>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings && rtk proxy cargo fmt --check && echo LINT-OK</automated>
    <automated>rtk proxy grep -c 'resume chain' docs/CONFIGURATION.md; rtk proxy grep -c '2 by the agent' docs/CONFIGURATION.md</automated>
  </verify>
  <done>
- `SUITE-OK` is printed.
- 59 `test result: ok` lines, 0 failed, 17 ignored, and at least 3577 passed plus the new tests.
- `LINT-OK` is printed.
- CONFIGURATION.md describes the chain close-out, the chain-wide agent count and the run.lock removal.
- Both doc grep counts print at least 1. At HEAD both were 0; `run.lock` is not used as a gate because it already appears 5 times at HEAD.
- The SUMMARY is written, and the commit `docs(quick-261103-qqm): document resume-chain close-out and run.lock removal` is made.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| run.json chain links -> scan/landing writes | `resumed_by` and `resumed_from` from records now decide which other runs get recorded as landed, lose their branch and lose their clone. This happens unattended, on `task list` and at TUI start. |
| agent clone -> host git | A predecessor's clone is agent-written. It is deleted only behind `clone_preserved` (planted gate, then containment). |
| concurrent processes -> run dir locks | The daemon, the CLI and the TUI race on `run.lock` (now unlinked at run end and by the scan) and on each predecessor's `land.lock`. |
| record fields -> terminal | Predecessor ids and branch names reach the landing output and stderr. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-qqm-01 | Tampering | chain close-out (recycle.rs `successor_landing_branch`, `contained_candidate`, `close_out_predecessors`) | high | mitigate | Each link must hold in both directions (`resumed_by` = next id AND next `resumed_from` = this id). A landing counts only when it puts work on target and its detail passes `plausible_branch` and resolves as `refs/heads/<detail>`. The predecessor's head must pass `merge-base --is-ancestor` against that tip, on the project's own objects. Everything is judged again under `try_land_lock`. The clone goes only through `clone_preserved`, and the branch only through the safe `git branch --delete` at the recorded head. |
| T-qqm-02 | Denial of service | chain walks (store.rs `resume_successors`/`resume_predecessors`) | medium | mitigate | `MAX_RESUME_CHAIN` = 32 and a visited set, so a cyclic or forged chain ends. Ids go through `read_view`, which checks `valid_oneoff_run_id` (no path traversal). |
| T-qqm-03 | Tampering | `run.lock` unlink (launch.rs `run_foreground`, recycle.rs `clear_stale_run_lock`) | medium | mitigate | The launcher unlinks only while it holds the lock and only when `RunLock::is_at(path)`, after the record is final. The scan removes only a lock that `probe_at` reports Free. A held or unprobeable lock stays. Nothing ever re-takes the `run.lock` of a run dir that has a record. |
| T-qqm-04 | Repudiation | predecessor record | low | mitigate | The close-out writes a normal `already-landed` landing with `at`, `detail`, `merge_sha` and `target_tip`, plus `pruned {at, note}`, so the record says when, where and why the predecessor was closed. |
| T-qqm-05 | Spoofing | landing output and scan stderr lines | low | mitigate | Lines are printed through the existing `shown()` / `render_for_terminal` paths. Ids are validated run ids, and branches pass `plausible_branch`. |
| T-qqm-06 | Information disclosure | e2e tests | low | accept | They use the fake harness and temp git repos only. They make no paid runs and never touch `~/.local/share/gsd-meta-manager`. |
| T-qqm-SC | Tampering | npm/pip/cargo installs | low | accept | No package installs or dependency changes in this plan. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: 59 result lines, 0 failed, 17 ignored, at least 3577 passed plus the new tests (raw log `target/qqm-host.log`).
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean.
- The e2e tests prove all of the following:
  - a landed successor closes out its predecessor (recorded, slimmed, branch gone, no locks) and prints a chain-wide agent count;
  - a predecessor stranded by an earlier landing is closed out by the scan;
  - a discarded successor leaves its predecessor whole and the scan stays silent;
  - no `run.lock` is left when a run ends;
  - the `--from master` message names 2 agent commits out of 3.
- Unit tests cover the pure chain predicate, the bounded walks, the removed I-17 clause and stale run.lock removal.
</verification>

<success_criteria>
- All three defects in the brief are fixed, each with fixture-based regression tests and no paid runs.
- The real-data state is reproduced on the fake harness: the successor is recorded `merged`, and the paused predecessor still has its clone and branch. The scan closes it out, so the orchestrator's later cleanup needs one `task list` and no new code.
- Every inferred decision (I-01..I-07) is applied as written and listed in the SUMMARY for audit.
</success_criteria>

<output>
Create `.planning/quick/261103-qqm-fix-one-off-resume-chain-cleanup-chain-c/261103-qqm-SUMMARY.md` when done
</output>
