---
quick_id: 261103-klf
mode: quick
phase: quick-261103-klf
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/launcher/process.rs
  - src/launcher/push.rs
  - src/oneoff/mod.rs
  - src/oneoff/cancel.rs
  - src/oneoff/resume.rs
  - src/oneoff/store.rs
  - src/oneoff/runtime.rs
  - src/oneoff/session.rs
  - src/oneoff/inbox.rs
  - src/oneoff/launch.rs
  - src/oneoff/land.rs
  - src/oneoff/merge.rs
  - src/oneoff/resolve.rs
  - src/oneoff/prompt.rs
  - src/cli.rs
  - src/task_cli.rs
  - src/action.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/oneoff.rs
  - src/ui/screens/oneoff_viewer.rs
  - src/ui/screens/help.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/launcher_support/mod.rs
  - tests/oneoff_e2e.rs
  - tests/oneoff_runtime_seam_guard.rs
  - docs/CONFIGURATION.md
  - .planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md
autonomous: true
requirements: [QUICK-261103-klf]

estimate:
  tokens: 520000
  raw_tokens: 520000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "After `task land merge` fast-forwards, the run's latest landing in run.json has `how: ff`, `merge_sha` = the head and `range` = `<checkout HEAD before the ff>..<head>`; after a merge or resolved landing, `range` = `<target tip>..<merge_sha>`. Once the landing deleted `gsd-oneoff/<id>`, `tracked_branch` is null and `tracking_note` is `deleted after landing`. A discard that deletes the branch writes `tracked_branch: null` before it removes the run dir. A pre-klf run.json (no `range`/`stop`/`resumed_*`, a Merged landing with `tracked_branch` still set) deserializes, and `task status`, `task list` and the TUI summary all show it as landed with no live branch."
    - "`task status` prints `branch: gsd-oneoff/<id> @ <sha12>` while the branch is tracked, the landed range and merge sha after a landing, and for a merge/resolved landing the exact `git diff <sha>^1 <sha>` command."
    - "The TUI merge diff (`d`) never holds more than the hunk cap of git's output in memory. The capped reader stops at 5000 lines / 512 KiB, then kills and reaps git. The view is the `conflicted: ...` header, a numstat summary of every file (up to 2000, then `... N more files`), the hunks, and, when cut, a marker starting `truncated — ` that gives the remaining files and approximate lines. `o` streams the full diff to `<run-dir>/land/merge-<sha12>.diff` (0600, no symlink follow) and opens it in $EDITOR."
    - "`task stop <p> [--run-id]` (hard cut) sends TERM to the run's scope and, after `GSDMM_ONEOFF_STOP_GRACE_SECS` (default 10), KILL. A scenario that ignores TERM ends `killed` with `stop {kind: hard, escalated: true, by: cli}`, and a cooperative one ends `stopped`. `--kill` ends it `killed` at once (`escalated: false`). The run is finalized as usual, branch tracking included."
    - "`task stop --pause` (TUI `z`) injects the pause message through the agent's stdin; it is recorded in `injections.jsonl` with kind `pause`. An agent that commits its WIP, reports `partial` and exits within `GSDMM_ONEOFF_PAUSE_GRACE_SECS` (default 300) ends `paused`, with `gsd-oneoff/<id>` tracked at its head. An agent that ignores the message is hard-cut after the grace and ends `killed` with `stop {kind: pause, escalated: true}`."
    - "`task stop` on a run that is not live but whose land lock is held targets the landing and says so (`--resolver` forces it). Hard and kill end the resolver; nothing lands, the branch stays, and `land_failure` has the reason `stopped by user` or `killed by user` and the matching `stop` kind. With `--pause`, the current attempt runs on within the pause grace, nothing lands, no further attempt starts, and the record is `land_failure {reason: paused, stop: pause}`. No landing holder pid is ever signalled."
    - "The run list, the run summary, the run viewer and the resolver viewer all take `x` stop, `X` kill now, `z` pause and `u` resume, each confirmed first. `u` asks `b` (from its branch) or `M` (merge master first), and Esc cancels. On a run whose landing was paused, `u` retries the landing. While the viewer's composer or soft-cap input is open, these keys are typed text."
    - "`task resume <p> [--run-id] --from branch` on a paused, stopped or killed run with a head starts a NEW run id. The new run's work branch starts at the old head (its `base`), and its fresh session is primed with the old task, the old report and the old commit log. The old record gets `resumed_by` = the new id and the new one `resumed_from` = the old id; the old run stops counting as `to land`. Resume refuses (exit 1) a second resume of the same run, a live run, a landed run and a run that finished on its own."
    - "`task resume --from master` merges the checkout branch's tip into the old head inside the new run's private clone, with parents [old head, tip]. A clean merge becomes a host-made merge commit, and conflicts go to one sandboxed resolver whose result the host re-checks. `resume_merge` records how, merge_sha, conflicted and attempts, and the new run's `base` is that merge commit."
    - "`rtk proxy cargo test --no-fail-fast` reports 0 failed, with no per-binary result line missing versus the Task 0 baseline. `cargo clippy --all-targets -- -D warnings` passes for the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`, and `cargo fmt --check` passes. `git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs` is empty."
  artifacts:
    - path: src/launcher/process.rs
      provides: "run_capped (stdout read up to N bytes / L lines, then the child is killed and reaped) and run_to_file (stdout straight into a file)"
      contains: "fn run_capped"
    - path: src/launcher/push.rs
      provides: "git_capped and git_to_file: hardened git through the two new process helpers"
      contains: "fn git_capped"
    - path: src/oneoff/merge.rs
      provides: "pure numstat/shortstat parsing and diff_view (header, summary, hunks, truncation marker)"
      contains: "fn diff_view"
    - path: src/oneoff/store.rs
      provides: "Landing.range, RunRecord.stop/resumed_from/resumed_by/resume_merge, LandFailure.stop, branch_state() normaliser"
      contains: "fn branch_state"
    - path: src/oneoff/cancel.rs
      provides: "StopKind/StopBy/StopRequest/StopRecord, the request file, both graces, the pure cancel state machine, the TERM->KILL cut"
      contains: "STOP_REQUEST_FILE"
    - path: src/oneoff/resume.rs
      provides: "resume refusals, resume budget, the serialized start (resumed_by under the old run's land lock), the --from master merge in the new clone"
      contains: "fn resume_refusal"
    - path: tests/oneoff_e2e.rs
      provides: "e2e on the fake harness: refs after ff/merge/discard, capped + exported diff, hard cut with escalation, kill, pause honoured and ignored, resolver stop and pause, resume from branch and from master (clean and resolved), resolver-paused landing resumed"
    - path: .planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md
      provides: "the C-10 follow-up: resume through the agent's own session behind the AgentRuntime seam"
  key_links:
    - from: "oneoff::land::merge_diff"
      to: "push::git_capped -> process::run_capped"
      via: "shortstat + capped numstat + capped hunks, assembled by merge::diff_view"
      pattern: 'run_capped\('
    - from: "OneoffScreen Diff mode `o`"
      to: "land::export_merge_diff -> push::git_to_file -> App.pending_editor"
      via: "an export action on spawn_blocking, answered by an exported action that opens the editor"
      pattern: 'export_merge_diff'
    - from: "launch::stop(kind, by) and the TUI x/X/z"
      to: "<run-dir>/stop.request read by run_sandboxed_session's wait loop"
      via: "request file + SIGTERM for hard/kill, file only for pause; the daemon escalates"
      pattern: 'STOP_REQUEST_FILE'
    - from: "task stop on a landing, the TUI on a resolver"
      to: "<run-dir>/land/stop.request read by the resolver session loop and by land_by_merge before fast_forward"
      via: "the landing never reaches the final ff once a request exists"
      pattern: 'LandFailure'
    - from: "task resume / TUI u"
      to: "resume::start -> launch::run_detached|run_foreground with a resume spec -> run_session starts the clone at the old head (and merges master)"
      via: "hidden --resume-from/--resume-start daemon args"
      pattern: 'resumed_from'
---

<objective>
This builds on the one-off subsystem: quick 261103-f5y (dispatch), 261103-gt1 (stream, inbox, viewer, soft cap) and 261103-hx7 (parallel runs, tracked branches, resolver, multi-run UI). It closes three hx7 audit items:
- "the merge diff reads git's whole output before capping",
- "a landing's `tracked_branch` is not cleared in `run.json`",
- "the TUI cannot cancel a running resolver".

It adds:
- a hard cut, an immediate kill and a soft pause for runs and resolvers, from the CLI and the TUI;
- `task resume` with `--from branch|master`;
- accurate git references in run.json;
- a bounded, streamed merge diff that can be opened in full;
- the docs and help.

Purpose: a run cannot be stopped cleanly, paused or picked up again today. A runaway merge diff can cost memory, and run.json can claim a branch that landing already deleted.

Output: run.json refs and the bounded diff (Task 1); stop, kill and pause for runs and resolvers in the CLI and TUI (Task 2); resume, final docs and help, and the follow-up todo (Task 3).

Task shape. Each task ends green and committed. The order follows CONTEXT's priority list a..g, except I-01:
- Task 1 (tracer) = (a) run.json refs + (b) bounded diff.
- Task 2 = (c) hard cut + kill for runs and resolvers, CLI + TUI, (d) soft pause of runs, (g) resolver soft pause.
- Task 3 = (e) `task resume --from branch`, (f) `--from master` with the resolver, the docs/help finalisation and the todo.

Fallback if a task cannot be reached. Never leave a half task uncommitted; record the fallback in the SUMMARY.
- Task 2 not reached: write `.planning/todos/pending/2026-11-03-oneoff-stop-kill-and-pause.md`, covering L-1, L-2, L-4, C-01..C-08, C-13, C-14 and I-09..I-16 with the Task 2 e2e list.
- Task 3 not reached: write `.planning/todos/pending/2026-11-03-oneoff-resume-paused-runs.md`, covering L-3, C-09..C-12, I-17..I-24 and the Task 3 e2e list, AND the C-10 follow-up todo that Task 3 would write.

Decision ids. CONTEXT's "Locked by the user" items are cited as L-1..L-7 and its `[inferred — audit]` items as C-01..C-21, all binding. Its "Specific Ideas" are cited as S-01..S-04, in order: reuse (grace timer, inbox, HostEnd, resolve), the pause text in `runtime.rs`, the fake-harness tests, the goldens.
- L-1 Hard cut = scope/process group TERM, then KILL after a short grace; nothing lands. For a resolver: abort, nothing lands, the branch stays.
- L-2 Soft pause = an inbox-style message asking the agent to commit WIP, `taskctl report`, and end. The host tracks `gsd-oneoff/<id>` and records outcome `paused`. The grace escalates to a hard cut.
- L-3 Resumable two ways, `--from branch` and `--from master` (master merged into the run first, LLM resolver on conflict), each in a new sandboxed session.
- L-4 Resolver soft pause = finish or abort the attempt cleanly and leave the run unlanded; resume = retry landing.
- L-5 run.json: merge landing → the merge commit (ff → the landed range); pause → `gsd-oneoff/<id>` + head; discard → no stale ref. Old files stay readable, and the UI/CLI show what is recorded.
- L-6 Bounded diff: stream, stop at the cap and kill the child; numstat/stat summary for everything, hunks to the cap, a truncation marker, a way to open the full diff externally.
- L-7 No rebase, push or tag, and no real paid agent runs (fake claude/bwrap/systemd-run only).
- C-01..C-21: as numbered in `261103-klf-CONTEXT.md` (keys; CLI verbs; grace; who escalates; outcomes; agent-side partial; the stop record; pause tracking; resume run ids; fresh session; master merge; resume refusals; resolver hard cut; resolver pause; landing refs; discard/pause refs; backward compatibility; display; bounded reader; diff view; full diff externally).

Planner-inferred decisions. The human is unavailable. Carry each one into the SUMMARY tagged `[inferred — audit]`.
- I-01 Task order: (g) resolver pause ships in Task 2 with (c)/(d). It shares the request file, the grace and the session-loop code with the run's pause, and building it later would reopen the same files. That is the only departure from CONTEXT's a..g fallback order: (g) lands before (e)/(f).
- I-02 The bounded reads live in `src/launcher/process.rs`, already `tests/spawn_seam_guard.rs`'s single launcher allowlist entry, so S-04 holds with no guard edit:
  - `run_capped(program, args, cwd, extra_env, max_bytes, max_lines, budget)` returns `Option<CappedOutput { status: Option<ExitStatus>, stdout: Vec<u8>, truncated: bool }>`. A reader thread stops at either cap; the caller then kills and reaps the child. stderr goes to a temp file, as in `run_bounded_with_stderr`.
  - `run_to_file(program, args, cwd, extra_env, out: File, budget)` returns `Option<ExitStatus>`; stdout goes straight to `out`.
  - `src/launcher/push.rs` wraps them as `git_capped` and `git_to_file`, with `GIT_HARDENING` and exactly `git`'s env (`GIT_TERMINAL_PROMPT`, `GIT_ALLOW_PROTOCOL`, repository-env scrub), so `push::git*` stays the only path that starts a launcher git.
- I-03 The diff view is built from three reads, all `--no-ext-diff --no-textconv --no-color --no-renames <sha>^1 <sha>`, so the three agree:
  - `git diff --shortstat`: totals, one line, always complete;
  - `git diff --numstat -z`, capped at 2000 entries / 1 MiB: the per-file summary, binary `-\t-` rows counting as files with 0 lines;
  - the hunks, capped at hx7's 5000 lines / 512 KiB.
  The marker is `truncated — N more file(s), ~M more line(s)`:
  - N = the shortstat files minus the files whose `diff --git` header was shown;
  - M = the shortstat insertions + deletions minus the `+`/`-` body lines shown, floored at 0.
  The pure `merge::diff_view` replaces `merge::diff_lines`, and its callers are updated.
- I-04 `o` in the diff view:
  - Export to `<run-dir>/land/merge-<sha12>.diff`. `land/` is created 0700 if missing. The file is opened `O_NOFOLLOW | O_CREAT | O_TRUNC`, 0600, and a symlink there fails. The sha must pass `push::is_hex_sha`. Budget 120 s.
  - On success the app sets `pending_editor`, the precedent being the action handler that sets it at `src/app.rs:2904`. On failure a status message is shown and no editor opens.
- I-05 `RunRecord::branch_state()` is the reader-side normaliser (C-17). It returns `BranchState::{Tracked { name, head }, Gone { why }, Untracked { note }}`.
  - `Gone` when `tracked_branch` is None and the note is one of the landing/discard notes (I-06/I-07).
  - `Gone` also for a legacy record with a `Merged` landing and a still-set `tracked_branch`.
  - Every display (CLI status/list, TUI summary/list, counts) uses only this, never the raw field. The display name strips `refs/heads/`.
- I-06 `land::delete_tracked_branch_after_landing` returns `Deleted | Absent | Left(line)` instead of an Option.
  - Both landing paths (`land::merge` ff and `resolve::landed`) write run.json ONCE, after the delete, with the landing appended and `land_failure` cleared.
  - `Deleted` → `tracked_branch = None`, `tracking_note = "deleted after landing"`.
  - `Absent` with a set `tracked_branch` → None + `"already gone at landing"`.
  - `Left` keeps the ref and prints the line.
- I-07 Discard: after a successful `update-ref -d` and BEFORE `remove_dir_all`, write `tracked_branch = None`, `tracking_note = "deleted by discard"`. A branch left in place (it moved) gets `tracking_note = "left by discard: it no longer points at the run's head"`. A failed removal then leaves no stale ref (C-16).
- I-08 Ranges:
  - ff: `<old>..<head>`, where `<old>` is `rev-parse --verify HEAD^{commit}` in the project, read after the dirty/branch checks and before the user's `--ff-only`.
  - merge/resolved: `<tip>..<merge_sha>`, where `<tip>` is the target tip the successful attempt merged onto, carried in `Step::Landed`.
- I-09 Request files:
  - `<run-dir>/stop.request` (the run's agent) and `<run-dir>/land/stop.request` (the landing). Both are host-only and sit outside every sandbox bind: the run's binds are the clone, `report/`, the config dir and sockets; a resolver's are `land/<n>/...`.
  - Format: JSON `{kind: hard|kill|pause, requested_at, by: tui|cli}`, at most 4 KiB. Writes go to a temp file in the same dir, then rename (0600). Reads use `O_NOFOLLOW`.
  - The stronger kind wins (kill > hard > pause), and a requester never downgrades a pending request.
  - A landing deletes a stale `land/stop.request` right after it takes the land lock.
- I-10 A request is written only while its target's lock is held: `run.lock` for a run, `land.lock` for a landing.
  - The land lock's holder pid is NEVER signalled, because the holder of a TUI-owned landing is the TUI itself. The landing polls its request file on the resolver session's tick instead. This refines C-13's "SIGTERM it".
  - The TUI also sets its own per-run `Arc<AtomicBool>` for landings it started (C-13), and `task land merge` keeps its Ctrl-C/TERM handlers.
  - `X` on a resolver also kills the resolver scope directly by name (C-13): `systemctl --user kill --signal=KILL <unit>`, only when the name passes the one-off scope plausibility check and ends in `-land<n>-<hash8>.scope`.
- I-11 Two graces, both parsed like `session::grace_from` (an integer 1..=600, else the default) and documented as tests-only env rows:
  - `GSDMM_ONEOFF_STOP_GRACE_SECS`: TERM → KILL, default 10 s.
  - `GSDMM_ONEOFF_PAUSE_GRACE_SECS`: pause → hard cut, for runs and resolvers, default 300 s.
  C-03 asks for one env "in the same style" and C-14 for a 300 s pause grace; two variables keep the two defaults apart.
- I-12 The one-off cut does not use `Containment::stop`; driver code in `src/launcher/run.rs` is untouched.
  - Scope: `systemctl --user kill --signal=TERM <unit>`, poll the child up to the stop grace, then `systemctl --user kill --signal=KILL <unit>` plus a `child.kill()` backstop, reap, then `stop_leftovers()`.
  - Group: `kill_process_group` TERM/KILL with the same timing.
  - kill skips TERM.
  - The wall-clock and soft-cap stops keep `Containment::stop`.
  - The fake `systemctl` in `tests/launcher_support/mod.rs` gains a `kill` verb (additive).
- I-13 Ends and outcomes:
  - `HostEnd` gains `Paused` and `Killed { escalated: bool }`.
  - After the child exits, the loop reads the request file once more, and a request found then classifies the end. A pause request plus a clean exit is `Paused` (C-06). A kill request, which `X` on a resolver makes before killing the scope directly, is `Killed`.
  - A pause whose grace ran out ends `Killed { escalated: true }` even if TERM sufficed (C-05).
  - `StopRecord.by` is `tui|cli|signal`, with `signal` when a TERM or Ctrl-C arrived and no request file existed (an older CLI or a terminal). This is additive to C-07.
  - The daemon writes `record.stop` with `ended_at: None` when it first observes a request (so the UI can say "pausing"/"stopping") and completes it at finalize.
- I-14 `--wait-secs` (the existing flag) is C-02's `--wait`.
  - With `--pause`, a run still winding down at the end of the wait exits 0 and says the launcher cuts it after its grace.
  - Hard/kill keep today's exit 1 for "still running".
  - With no live daemon, an orphan scope is handled by the requester: hard → `systemctl --user stop`, as today; kill → `systemctl --user kill --signal=KILL` and then stop. A pause is refused: "no live launcher to pause it; use stop or kill".
- I-15 `LandFailure` gains `stop: Option<StopKind>` (serde default).
  - Reasons: `stopped by user`, `killed by user`, `paused` (C-13/C-14).
  - The TUI's `u` and `task resume` detect a paused landing by `stop == Some(Pause)`, never by text.
  - `land_by_merge` re-reads `land/stop.request` after every resolver session, before every `fast_forward` and between attempts. Any request ends the landing there: no landing, no further attempt.
- I-16 Keys: at e9ebe29, `X`, `z` and `u` are unbound in `src/ui/screens/oneoff.rs` and `src/ui/screens/oneoff_viewer.rs`, and the viewer has no `x` yet.
  - `u` is bound on the Driver tab itself (`src/ui/screens/detail.rs`, `KeyCode::Char('u') if current_view == DetailSubView::Driver`, unhold), but the popup and the viewers are pushed screens that get keys first, so nothing clashes. The help rows scope the new keys "In T" / "In the live view".
  - The viewers push a `OneoffScreen` in an Ask mode for the viewed run; its `n`/Esc/`y` pop back to the viewer.
  - On a run that is not live but whose land lock is held, `x`/`X`/`z` target the landing and the Ask prompt says so. In the resolver viewer they always target the landing.
- I-17 `task resume`:
  - `--from` is optional to clap. It is required to resume the agent ("pass --from branch or --from master") and refused for a run whose landing was paused; for that run `task resume` retries `task land merge`.
  - Default foreground; `--detach` detaches, as C-02 has it (the opposite of `task run`); the TUI always detaches.
  - `--budget <usd>` overrides the budget. `--unsandboxed` is accepted explicitly and never inherited.
- I-18 The resume start is serialized:
  1. The requester takes the OLD run's land lock, re-reads its record and applies the refusals.
  2. It claims the new run dir and writes `resumed_by` on the old record.
  3. It releases the lock and starts the new run with `run_id = Some(new)`.
  If the start fails, it clears `resumed_by` again (best effort). The refusal also ignores a `resumed_by` whose run dir is gone, or whose run ended with `resume_merge.failure` set (a failed resume never strands its predecessor).
- I-19 A run with commits > 0 but no `tracked_branch` (planted findings, fetch failure, or a discard) is refused: its work is not in the project. A zero-commit run resumes from its head, which is its base.
  - The new clone is the usual project clone; `git_read` (host config off) then switches it to `gsd-oneoff/<new>` at the old head.
  - The committed-todo check is skipped for a resume, since the old run passed it and may have moved the todo.
  - Model, mode, task and cap_minutes are the old run's.
- I-20 "master" in `--from master` is the branch the project's checkout is on, at its tip (`push::current_branch_and_sha`), the same target a landing uses. A detached HEAD is refused.
  - The merge happens in the new run's own clone, which is C-11's private `--no-hardlinks` clone: the daemon does it in `run_session` before the agent session, with `git_read` (`merge --no-ff --no-edit`) and `-c user.name/-c user.email` from `resolve::merge_identity`.
  - A clean merge becomes a host commit, `Merge <branch> into one-off <old-id> (resume <new-id>)`. A tip already contained in the old head means no merge, plus a warning.
  - Conflicts go to ONE resolver attempt (there is no moving target to retry) in `<new-run-dir>/land/<n>`. The resolver uses `resolve::claim_attempt` and a `resolve_conflicts` generalized to take explicit (first, second) parents and a prompt direction. Its budget is `resolver_budget(new max, 0)`.
  - The resolver's spend is added to the new run's `spend_usd`. The agent session gets the remainder as `--max-budget-usd`; below $0.25 the run ends `failed` ("no budget left after the resume merge").
  - Any failure ends the run `failed` before the agent session, with `resume_merge.failure` set. A stop request during the resume merge ends the run with that request's outcome, before the agent session.
  - The resolver viewer treats such an attempt as live while the run is live, still `preparing`, and the attempt is the newest.
- I-21 `prompt::build_resume` = the old task's normal prompt plus a continuation block:
  - "this continues one-off <old-id> (<outcome word>)";
  - the old report's headline and followups (at most 20, each control-cleaned and at most 200 chars);
  - `git log --format='%h %s' <old base>..<old head>` read in the new clone with `git_read` (at most 50 lines, each at most 200 chars);
  - the master-merge note.
  `MAX_PROMPT_BYTES` still applies.
- I-22 `resume::budget(old_max, old_spend, override)`: the override when given (positive and finite); otherwise `old_max − old_spend`, refused below `MIN_RESOLVER_BUDGET_USD` with "no budget left to resume; pass --budget".
- I-23 The C-10 follow-up todo is a NEW file, `.planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md`. It cross-references the existing `2026-11-03-oneoff-raise-hard-ceiling-via-resume.md`, which stays pending and unedited.
- I-24 The daemon handoff for a resume uses hidden `task run` args `--resume-from <old-id>` and `--resume-start branch|master`. With `--resume-from`, the daemon takes the task, mode, model and cap from the old record, and the `task` ArgGroup accepts it in place of `--todo`/`--text`. Finalize adds the session's spend to any resume-merge spend instead of overwriting it.
- I-25 All new run.json fields are `#[serde(default)]`:
  - `Landing.range`, `LandFailure.stop`;
  - `RunRecord.stop`, `resumed_from`, `resumed_by`;
  - `resume_merge: Option<ResumeMerge { how, branch, merge_sha, conflicted, attempts, failure }>`.
  `AgentOutcome` gains `Paused` and `Killed` (kind words `paused`, `killed`). `synthesize` maps Paused → `partial` ("The run was paused before the agent reported") and Killed → `failed` ("The run was killed before the agent reported").
</objective>

<gates>
- musk: a quick task with no ROADMAP phase. Every task traces to L-/C-/S-/I- ids. No omissions were made, so none are logged.
- api-coverage: does not fire. No external API/SDK; local git, systemd/bwrap and the existing agent CLI seam.
- assumption-delta: skipped (no ROADMAP phase). One transition is real: "a run ends once" becomes "a run can be continued as another run". Primary noun: the run id. Resume makes a NEW run and links both records (`resumed_from`/`resumed_by`) instead of reopening an old one, so no verb's single-run assumption changes. The invariant test is Task 3's e2e: a second resume of the same run is refused.
- schema gate: does not fire (no ORM). run.json compatibility is pinned by a pre-klf fixture test (C-17).
- security: `<threat_model>` below at ASVS L1, blocking on high. Every high threat has a mitigation implemented in a task.
</gates>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/261103-klf-cancel-pause-one-off-runs-and-resolvers-/261103-klf-CONTEXT.md
@.planning/quick/261103-hx7-parallel-one-offs-per-project-tracked-br/261103-hx7-SUMMARY.md

Large files: NEVER read `src/app.rs` (8.8K lines), `src/ui/screens/detail.rs` (35K lines) or `src/ui/screens/render_escape_guard.rs` whole. Grep for a symbol, then Read with offset/limit around it. If an anchor below has moved, find it by its symbol name.

Measured at HEAD e9ebe29.

`src/oneoff/mod.rs`:
- File-name constants at :55-90 (`RUN_LOCK_FILE` :74, `LAND_LOCK_FILE` :76, `LAND_DIR` :78, `BRANCH_PREFIX` :80, `INBOX_DIR` :82).
- `AgentOutcome` with `word()` at :133-160.
- `OneoffCommand` (Merge/Push/Discard/Stop, `verb()`) at :164-181.
- The round-trip test at :194.
- Gated modules `land`, `launch` and `resolve` near :48-52.

`src/oneoff/store.rs`:
- `MergeHow` :60, `Landing` :82-117 (`Landing::plain` :104), `LandFailure` :120.
- `RunRecord` :140-205 (`tracked_branch` :192, `tracking_note` :195, `spend_usd` :200, `land_failure` :203), `RunRecord::new` :209, `last_merge_landing` :261.
- `synthesize` :340, `derive_live` :419, `OneoffView` :437 (`landing` :443, `land_attempts` :445), `RunCounts`/`run_counts` :457/:491, `RunPick` :519.
- `fs_store` :552: `live_seen` :591, `live_lock_path` :602, `write_record` :612, `read_record` :627, `read_view` :690, `land_attempts_of` :724, `read_runs_in` :753, `resolve_run` :775.
- Test helper `tests::record()` :968.

`src/oneoff/launch.rs`:
- `OneoffOptions` :92-107.
- `claim_run_dir` ~:190.
- `run_foreground` :223 (signal handlers :259-260; the record is final before the lock drops :306-307).
- `Ctx` :312, `git_read` :326 (hardened, host config off).
- `run_session` :333: clone + switch :343-367, the committed-todo check :369-379, planted baseline :382, prompt :396, `SessionSpec` literal :398-416, `on_event` :417, `record.spend_usd = Some(session.spend_usd)` :438.
- `SessionSpec` :455, `SessionEvent` :484, `SessionRun` :498.
- `run_sandboxed_session` :512 (Supervisor :619, containment :621, the wait loop :630-663: user_stop :631, wall clock :636, soft cap :648, exit :654).
- `finalize` :697 (tracking :782-795).
- `daemon_argv` :808, `run_detached` :843.
- `StopOutcome` :936, `plausible_scope` :964, `stop(project, run_id, wait)` :977-1066 (signal while held :1017-1027; the orphan scope :1045-1062).

`src/launcher/run.rs`:
- `Containment` :482-575 (driver code; untouched): `stop` :505 (`systemctl --user stop`), `stop_leftovers` :557.
- `TimeoutStopSec=15` comes from the shared `sandbox::systemd_run_argv` (golden-pinned), so the one-off grace cannot ride on it.

`src/launcher/process.rs`:
- The module header lists the spawn shapes at :1-25 (add the two new ones there).
- `run_bounded` :325, `run_bounded_with_stderr` :440.
- The reader-thread + deadline pattern is `crate::bounded_output::stdout_within_with_stderr` (`src/bounded_output.rs:71`).
- Its tests already spawn `sh` (:521, :674).

`src/launcher/push.rs`: `GIT_HARDENING` :281, `MIRROR_ONLY_ENV` :308, `git` :319, `git_mirror_only` :324, `git_with_env` :328, `git_in` :357, `is_hex_sha`, `current_branch_and_sha`.

`src/oneoff/session.rs`:
- `SOFT_CAP_GRACE` :109, `SOFT_CAP_GRACE_ENV` :113, `grace_from_env` :117, `grace_from` :122, `SoftCap` :155.
- `Supervisor` :263: `deliver` :349, `handle` :370, `tick` :412 (wrap-up delivery :427-444), `close_input` :466, `finish` :473.

`src/oneoff/runtime.rs`: `HostEnd` :37-46, `wrap_up_message` :155, `WRAP_UP_MESSAGE` :246, `outcome_at` :526-570 (HostEnd mapping :533-537), the outcome tests :667-760.

`src/oneoff/inbox.rs`: `InjectionKind` :28 (Message/SoftCap/WrapUp).

`src/oneoff/land.rs`:
- `LandOptions` :63-87 (`new` = never-set flag; `with_signal_handlers` for the CLI).
- `land_lock_path` :90, `landing_in_progress` :95, `take_land_lock` :103 (the holder pid is the landing process: the TUI when the TUI lands).
- `git_relevant_planted` :212, `tracked_ref` :232, `ref_commit` :237, `track_branch` :256.
- `DIFF_BUDGET` :307, `merge_diff` :313-342, `record_landing` :345.
- `merge` :358-479: the ff path :432-478, the record :461-468, the delete :474.
- `delete_tracked_branch_after_landing` :484-501, `push` :504, `discard` :587-627 (branch delete :608-623, `remove_dir_all` :624).

`src/oneoff/merge.rs`: constants :11-38 (`MIN_RESOLVER_BUDGET_USD` :17, `DIFF_MAX_LINES` :29, `DIFF_MAX_BYTES` :32), `merge_parents_ok` :69, `diff_lines` :143-173 (its tests near :311), `landed_line` :176.

`src/oneoff/resolve.rs`:
- `Step` :75, `claim_attempt` :119, `merge_identity` :138, `persist` :204, `Land` :219.
- `land_by_merge` :232-292 (the failure record :278-291), `landed` :295-332, `attempt` :335.
- `resolve_conflicts` :537-699: the budget :547-550, the spec :602-620 with `user_stop` :618, the spend :629-634, the HostEnd match :643-654, the parents check :688-693.
- `fast_forward` :705.

`src/oneoff/prompt.rs`: `REPORT_USAGE` :19 (taskctl uses `--followup`), `build` :43, `ResolverPrompt` :86, `resolver` :110.

`src/cli.rs`: `TaskAction` :487-570 (`Stop` :511-520 with `--run-id` and `--wait-secs`), `LandAction` :574-597, `TaskRunArgs` :602-635 (the `task` ArgGroup :601; hidden `daemon_child`/`run_id` :629-634).

`src/task_cli.rs`:
- `pick` :74, the dispatch :92-133, `run_main` :158-241 (the egress allowlist :173-185 is the pattern).
- `summary_lines` :244-366 (`branch:` :288-292, `landed:` :308-346, `land failed:` :347-364), `status_main` :368, `list_line` :400-425, `stop_main` :453-505.
- `land_main` :698-753.

`src/app.rs`:
- `pending_editor` :536 (set from an action handler at :2904; `ScreenAction::SuspendAndEdit` at :3404).
- `open_oneoff_viewer` :1270, `request_oneoff_diff` :1376-1410, `request_oneoff_run` :1524 (`OneoffOptions` literal :1559).
- `request_oneoff_command` :1601-1690: Merge with `LandOptions::new` :1647-1651, Stop :1659-1668.
- The action dispatch :3049-3120. The diff tests are at :8715+, the viewer-open tests at :8790+.

`src/action.rs`: the `Oneoff*` actions :340-420.

`src/ui/screens/mod.rs`: `ScreenAction::Push` :326, `SuspendAndEdit` :354, `ProjectViewCache.oneoff_runs` :1233, `oneoff_diff` :1236.

`src/ui/screens/oneoff.rs`:
- `OneoffMode` :77-107 (List, Result, Ask{prompt, command, run_id, from_list}, Diff{scroll}), `OneoffScreen` :110.
- `list_key` :296, `result_key` :324, `run_action_key` :333-451 (`x` :433), `ask_key` :453.
- Diff: `diff_lines` :486, `diff_key` :496, `diff_body` :527.
- `merge_warning` :770, `result_lines` :837, `status_line` :952, tests :1038 (`finished_view` :1053).

`src/ui/screens/oneoff_viewer.rs`: `OneoffStream::new` :67, `OneoffViewerScreen` :103, `new` :121, `live` :166, `RESOLVER_NOTICE` :280, `handle_key` (composer/soft input first, then q/Esc/PgUp/PgDn/j/k/f/G/i/b) ~:304-360, tests :497.

`src/ui/screens/help.rs`: the one-off rows :371-390, repeated in tests at :648-656 and :1018-1026. They are asserted as WHOLE rows, and the three copies must stay identical.

`src/ui/screens/render_escape_guard.rs`: dispositions :378-379, the `OneoffScreen` fixture :2277+.

`tests/launcher_support/mod.rs`:
- `SYSTEMD_RUN_SHIM` :79 (records `<unit>.pid`), `SYSTEMCTL_SHIM` :135-162 (verbs `is-system-running`, `is-active`, `stop`; the unit is `$2`), `CLAUDE_SHIM` :164-194.
- `hermetic_command` :370.

`tests/oneoff_e2e.rs`:
- Helpers: `fixture` :43, `task`/`run` :64/:70, `run_dir` :82, `status` :86, `wait_finished` :93, `QUICK_SCENARIO` :107, `quick_run` :123.
- `PARALLEL_SCENARIO` :302, `status_of_run` :334, `wait_run_finished` :341, `branch_head` :355.
- `a_second_run_starts_while_one_is_live_and_stop_takes_a_run_id` :522 (asserts outcome `stopped`).
- `SOFT_CAP_SCENARIO` :838 (env override :851).
- `with_identity` :973, `conflict_fixture` :980, `merge_scenario` :989, `RESOLVES` :1007, `conflicting_run` :1017, `head_parents` :1032.
- `a_non_conflicting_merge_lands_as_a_merge_commit` :1040, the resolved merge :1102, `assert_nothing_landed` :1180, `a_second_landing_of_the_same_run_is_refused_while_one_is_in_progress` :1387.

`tests/oneoff_runtime_seam_guard.rs`: `NEUTRAL_FILES` :11-26 (add `src/oneoff/cancel.rs` and `src/oneoff/resume.rs`).

`docs/CONFIGURATION.md`: the env table :119-121 (`GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS` row :121), `#### One-off tasks` :159, **Keys** :166, **CLI.** :199, **Live view** :212, **Soft cap.** :242, **Landing.** :288, **Several at once.** :338, **Tracked branches.** :355.

Testing notes:
- `rtk` filters test output. Read raw output with `rtk proxy <cmd> > target/<name>.log 2>&1`, then Read the file. Never pipe test output through grep or awk, and always pass `--no-fail-fast`. A green run with an unchanged pass count can mean a suite never ran, so compare the per-binary result lines against the Task 0 baseline.
- No test starts a real agent or reaches the network, and no paid agent run is made (L-7).
- In-source `#[cfg(test)]` code under `src/` must not spawn `git` (`sh` is fine, as in `process.rs`); git-touching tests live in `tests/`.
- Every process starts through `launcher::process` or `push::git*`. There is no new `Command::new(` in `src/` (`tests/spawn_seam_guard.rs`).
- Nothing in `src/oneoff/` other than `runtime.rs` names the agent (`tests/oneoff_runtime_seam_guard.rs`); the pause message lives in `runtime.rs` (S-02).
- Use short grace overrides in e2e (`GSDMM_ONEOFF_STOP_GRACE_SECS=1`, `GSDMM_ONEOFF_PAUSE_GRACE_SECS=1..3`), set on the `task run` command so the daemon inherits them.
- The known flake is `launcher_bridge::the_bridge_pumps_tcp_to_the_egress_socket` (port race). Re-run it alone before calling anything a regression.

Commit hygiene:
- One commit per task on `master`, with files staged by name. Never push, tag or rebase (L-7).
- Do not stage `.planning/STATE.md`, `.planning/musk/ledger.jsonl`, `Cargo.toml`/`Cargo.lock`, or this quick dir's pre-dispatch artifacts (CONTEXT/PLAN); the orchestrator commits those.
- No new crate dependency (`rustix`, `serde_json`, `tempfile` and `chrono` are already in the tree).
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): a landing records its real git refs and clears the deleted branch, and the merge diff is streamed, capped, summarized and exportable in full (CLI + TUI)</name>
  <files>src/launcher/process.rs, src/launcher/push.rs, src/oneoff/store.rs, src/oneoff/land.rs, src/oneoff/resolve.rs, src/oneoff/merge.rs, src/task_cli.rs, src/action.rs, src/app.rs, src/ui/screens/mod.rs, src/ui/screens/oneoff.rs, src/ui/screens/help.rs, src/ui/screens/render_escape_guard.rs, tests/oneoff_e2e.rs, docs/CONFIGURATION.md</files>
  <behavior>
    - process (unit; spawns `sh` only):
      - `run_capped` on an endless writer (e.g. `sh -c 'while :; do echo xxxxxxxx; done'`) with `max_bytes = 4096` returns within 5 s with `truncated = true` and at most 4096 bytes, and the child has been reaped.
      - `max_lines = 10` stops at 10 lines.
      - A short command returns all of its output, `truncated = false` and a success status.
      - `run_to_file` writes ~2 MB of `sh` output to a file byte-exactly and returns the status.
    - merge (unit, pure):
      - The numstat `-z` parser handles text rows and binary `-\t-` rows. The shortstat parser handles "1 file changed, 3 insertions(+)" and the forms with no insertions or no deletions.
      - `diff_view` line 1 is the `conflicted: ...` header, then the summary (one line per file up to 2000, then `... N more files`), a blank line, then the hunks.
      - With the hunks cut, the last line starts `truncated — ` with the counts from I-03. Uncut hunks get no marker, and an empty diff keeps the "changed nothing" line.
    - store (unit):
      - A literal pre-klf `run.json` deserializes: hx7-shaped, a Merged `merge` landing with `merge_sha` and no `range`, `tracked_branch` still set, no `stop`/`resumed_*`. Its `branch_state()` is `Gone` (C-17).
      - A record with `tracked_branch = refs/heads/gsd-oneoff/<id>` and no landing is `Tracked { name: "gsd-oneoff/<id>", head }`.
      - A record with no ref and note `deleted after landing` is `Gone`.
    - e2e (`tests/oneoff_e2e.rs`, fake harness):
      - The ff landing in `two_concurrent_runs_on_one_project_run_and_are_tracked` (or `a_detached_quick_run_on_a_todo_reports_records_and_merges`) asserts `landed[-1].range == "<pre-ff HEAD>..<head>"`, `merge_sha == head`, `tracked_branch == null` and `tracking_note == "deleted after landing"`.
      - `a_non_conflicting_merge_lands_as_a_merge_commit` additionally asserts `range == "<target tip>..<merge_sha>"` (tip = HEAD^1), the cleared ref, and the new `land::merge_diff` view shape (header, summary, hunk).
      - `task status ALIAS --run-id <id>` (text) contains `git diff <merge_sha>^1 <merge_sha>`.
      - New `a_huge_merge_diff_is_capped_summarized_and_exported_in_full`: the run commits `a-big.txt` (20000 lines) and `z-small.txt`, the target moves on an unrelated file, and `land merge` makes a merge commit. `land::merge_diff` then:
        - returns at most `2 + summary + 5000 + 1` lines;
        - lists both files in the summary;
        - ends with a `truncated — ` marker naming 1 more file and ~15000 more lines.
        `land::export_merge_diff` writes `<run-dir>/land/merge-<sha12>.diff` (mode 0600), byte-equal to the test's own `git diff --no-ext-diff --no-textconv --no-color --no-renames <sha>^1 <sha>`. A symlink planted at that path makes the export fail without following it.
      - New `a_failed_discard_leaves_no_stale_branch_ref`: after `quick_run`, make a subdir of the run dir unremovable (chmod 0500 on a dir with entries, restored at the end). `land discard` exits 1, the branch is gone, and run.json has `tracked_branch: null` and `tracking_note: "deleted by discard"` (C-16, I-07).
    - TUI (unit): `o` in Diff mode dispatches the export action for the selected run (and only when a merge diff is loaded). The Diff render shows the summary and marker lines escaped. The render-escape guard gains a Diff state with hostile file names in the summary.
    - app (unit): the exported result `Ok(path)` sets `pending_editor` to that path; `Err` sets a status message and leaves `pending_editor` unset.
  </behavior>
  <action>
Step 0, before any edit: record the baseline. Run `rtk proxy cargo test --no-fail-fast > target/klf-t0.log 2>&1` and note the per-binary result lines and the totals for the SUMMARY (hx7 ended at 3434 passed, 17 ignored).

Write the failing tests first (behavior list), then implement. This task covers L-5, L-6, C-15..C-21, S-04 and I-02..I-08.

1. `src/launcher/process.rs` (C-19, I-02):
   - Add `CappedOutput`, `run_capped` and `run_to_file`, and list both in the module header's shapes.
   - `run_capped` follows `stdout_within_with_stderr`'s thread + deadline pattern. The reader thread reads in chunks and counts newlines. At `max_bytes` (the excess is truncated at the byte cap) or `max_lines` it stops reading, drops the pipe and reports `truncated`. The caller thread then kills the child (`Child::kill`), waits for it, and returns.
   - The deadline also kills and reaps, and returns `None` like `run_bounded`. Remove the repository `GIT_*` env for `git` exactly as `run_bounded_with_stderr` does.
   - `run_to_file`: stdout = `Stdio::from(out)` and stderr to a temp file. On the budget, kill and reap and return `None`.
2. `src/launcher/push.rs`:
   - `pub(crate) fn git_capped(args, max_bytes, max_lines, budget)` returns a `CappedGitRun { success, stdout: String, truncated, last_error }`.
   - `pub(crate) fn git_to_file(args, out, budget)` returns `GitRun`-like success/last_error.
   - Both carry `GIT_HARDENING` and the same env as `git` (host config ON, since the diff reads the user's project as `merge_diff` does today), via one shared env builder so the two cannot drift from `git_with_env`.
3. `src/oneoff/merge.rs` (C-20, I-03):
   - Add `NumstatRow`, `parse_numstat_z`, `Shortstat` + `parse_shortstat`, and `diff_view(conflicted, shortstat, rows, rows_truncated, hunks, hunks_truncated) -> Vec<String>` with the I-03 marker.
   - Keep `DIFF_MAX_LINES`/`DIFF_MAX_BYTES`/`DIFF_MAX_LINE_CHARS`. Add `DIFF_MAX_FILES = 2000` and `NUMSTAT_MAX_BYTES = 1 MiB`.
   - Summary lines look like `+12 -3  path` (`bin  path` for binary rows). Paths are control-cleaned with `clean_text`, like the recorded names. Hunk lines are cut by length only (the screen escapes).
   - Remove `diff_lines` and move its tests onto `diff_view`.
4. `src/oneoff/land.rs`:
   - `merge_diff(project, merge_sha, conflicted)` runs the three I-03 reads through `git_capped` (shortstat with small caps) and returns `diff_view(...)`. Its signature is unchanged for `src/app.rs`.
   - Add `export_merge_diff(project, run_dir, merge_sha) -> Result<PathBuf, String>` (I-04, C-21).
   - Ranges and the single write (C-15, I-06, I-08):
     - in `merge`'s ff path, read the pre-ff HEAD sha before the user's `--ff-only`, then build the `Landing` with `range`;
     - change `delete_tracked_branch_after_landing` to return the I-06 enum;
     - replace `record_landing` + delete with one write after the delete that also sets `tracked_branch`/`tracking_note`. `push` keeps appending its `Pushed` landing through a write that does not touch the branch fields.
   - `discard` (C-16, I-07): write the cleared or left ref and note into run.json before `remove_dir_all`.
5. `src/oneoff/resolve.rs`: `Step::Landed` carries the attempt's target tip. `landed()` sets `range = "<tip>..<merge_sha>"`, then calls the I-06 delete and writes run.json once with the landing and the branch fields.
6. `src/oneoff/store.rs` (C-15, C-17, I-05, I-25 part):
   - Add `Landing.range: Option<String>` (serde default; `Landing::plain` sets None) and `BranchState` + `RunRecord::branch_state()`.
   - `run_counts`' `to_land` uses `branch_state` only where it reads the branch; the hx7 rule "no Merged landing" stays.
   - Add the pre-klf fixture test.
7. `src/task_cli.rs` (C-18, C-21):
   - `summary_lines`' branch line comes from `branch_state()`: `branch:    gsd-oneoff/<id> @ <sha12>` / `branch:    none (deleted after landing)` / `branch:    none (<note>)`.
   - Each landing line adds `range <range>` when recorded.
   - A merge/resolved landing adds `diff:      git -C <project> diff <sha>^1 <sha>` (the project path escaped through `shown`); an ff landing with a range adds `diff:      git -C <project> diff <range>`.
   - `list_line` says `branch yes|gone|no` from `branch_state()`.
8. TUI (C-18, C-21, I-04):
   - `src/action.rs`: `OneoffDiffExportRequested { alias, run_id }` and `OneoffDiffExported { alias, run_id, result: Result<PathBuf, String> }`.
   - `src/app.rs`:
     - the request handler mirrors `request_oneoff_diff`: the cache gives the merge landing, the run dir is `view.run_dir`, and the work runs on `spawn_blocking`, cfg-gated like `request_oneoff_diff`;
     - the exported handler sets `pending_editor` for Ok and a status message for Err;
     - the merge diff stays `Result<Vec<String>, String>`, so `ProjectViewCache.oneoff_diff` is unchanged.
   - `src/ui/screens/oneoff.rs`:
     - Diff mode `o` dispatches the export, and the diff title/footer mentions `o full diff`;
     - `result_lines` shows `Branch: gsd-oneoff/<id> @ <sha12>` from `branch_state()` while tracked, `Landed: <range>` and `Merge: <sha12>` for a landing (replacing the hx7 "Branch only while no merge landed" rule with the normaliser);
     - update the unit tests;
     - extend the render-escape guard's Diff fixture (summary lines with hostile names, the marker).
   - `src/ui/screens/help.rs`: the `T` row's `d merge diff` becomes `d merge diff (o: full diff in $EDITOR)` in all three copies.
9. `docs/CONFIGURATION.md` **Landing.** and **Tracked branches.**:
   - the recorded `range`/`merge_sha`;
   - the branch cleared after landing and on discard;
   - the bounded diff (summary of every file up to 2000, hunks up to 5000 lines / 512 KiB, the marker);
   - `o` writes `<run-dir>/land/merge-<sha12>.diff` and opens `$EDITOR`;
   - `task status` prints the full-diff command.
10. Run the gates in this order:
    1. `cargo test --lib -- oneoff launcher::process`;
    2. `cargo test --test oneoff_e2e`;
    3. the full suite: `rtk proxy cargo test --no-fail-fast > target/klf-t1.log 2>&1`, then Read it and compare the per-binary lines with `target/klf-t0.log`;
    4. clippy for the host, `--target x86_64-pc-windows-gnu` and `--target aarch64-apple-darwin`;
    5. `cargo fmt --check`;
    6. the golden diff.
    Then commit `feat(quick-261103-klf): accurate run.json git refs and a bounded, exportable merge diff`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff launcher::process && cargo test --test oneoff_e2e && cargo clippy --all-targets -- -D warnings && cargo fmt --check && git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs</automated>
  </verify>
  <done>
- After an ff landing, run.json has `range` and `merge_sha`. After a merge landing it has `range` = tip..merge. After either, `tracked_branch` is null with the note `deleted after landing`.
- A discard whose dir removal fails leaves no stale ref.
- The pre-klf fixture reads back as "landed, branch gone".
- `land::merge_diff` caps at the hunk limit through `run_capped` and shows the summary and the marker; `o` exports the byte-exact full diff and opens the editor.
- `task status` prints the diff command.
- `target/klf-t1.log` shows 0 failed and no missing per-binary line versus `target/klf-t0.log`. Clippy is clean on 3 targets, fmt is clean, the goldens are unchanged, and there is one commit.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: hard cut (TERM then KILL), immediate kill and soft pause for runs and resolvers, from `task stop` and from x / X / z in the run list, the summary and both viewers</name>
  <files>src/oneoff/cancel.rs, src/oneoff/mod.rs, src/oneoff/store.rs, src/oneoff/runtime.rs, src/oneoff/session.rs, src/oneoff/inbox.rs, src/oneoff/launch.rs, src/oneoff/land.rs, src/oneoff/resolve.rs, src/cli.rs, src/task_cli.rs, src/action.rs, src/app.rs, src/ui/screens/oneoff.rs, src/ui/screens/oneoff_viewer.rs, src/ui/screens/help.rs, src/ui/screens/render_escape_guard.rs, tests/launcher_support/mod.rs, tests/oneoff_e2e.rs, tests/oneoff_runtime_seam_guard.rs, docs/CONFIGURATION.md</files>
  <behavior>
    - cancel (unit, pure):
      - kill > hard > pause, and `merge_request` never downgrades.
      - A request round-trips through JSON. More than 4 KiB, garbage or an unknown kind is an error.
      - Both graces parse: defaults 10 s / 300 s, 1..=600 accepted, 0 / 601 / text fall back.
      - The state machine, driven with injected instants:
        - pause: `InjectPause` once, `Wait` until the pause deadline, then `Term`;
        - `Term`: `Wait` for the stop grace, then `Kill`;
        - kill: `Kill` at once;
        - a hard request during a pause: `Term` at once;
        - signalled with no request: hard, by `signal`.
    - runtime (unit): `outcome_at` maps `Paused` → `Paused`, `Killed{..}` → `Killed` and `UserStop` → `Stopped`. `pause_message()` names `taskctl report --status partial` and `--followup`.
    - mod/store (unit): `Paused`/`Killed` round-trip with the words `paused`/`killed`. `synthesize` covers both. A record without `stop` and a `LandFailure` without `stop` read back as None.
    - e2e (fake harness; graces shortened on the `task run` command):
      - `a_hard_stop_escalates_to_kill_when_the_agent_ignores_term`: the scenario does `trap '' TERM`, touches `started` and loops on `sleep 0.1`; `GSDMM_ONEOFF_STOP_GRACE_SECS=1`. `task stop ALIAS` exits 0. The outcome is `killed`, with `stop.kind == hard`, `escalated == true` and `by == cli`. The shim log has `systemctl --user kill --signal=TERM <unit>` before `--signal=KILL <unit>`.
      - `task stop --kill` on a cooperative scenario: outcome `killed`, `escalated == false`, and the shim log has no TERM kill line for that unit.
      - The existing `a_second_run_starts_while_one_is_live_and_stop_takes_a_run_id` stays `stopped` and now also asserts `stop == {kind: hard, by: cli, escalated: false}`.
      - `a_paused_run_commits_its_wip_reports_partial_and_is_tracked`: the scenario replays stdin lines. On a line containing the pause text it writes and commits `wip.txt`, runs `taskctl report --status partial --headline "Paused" --followup "finish it"`, prints a success result and exits. `task stop ALIAS --pause --wait-secs 30` exits 0. Then:
        - the outcome is `paused` and `report.status == partial` (agent);
        - `tracked_branch == refs/heads/gsd-oneoff/<id>` at `head`;
        - `injections.jsonl` has one delivered `pause`, and `stream.jsonl` has the replayed pause text.
      - `a_pause_the_agent_ignores_is_cut_after_its_grace`: `SOFT_CAP_SCENARIO`-style endless replay, `GSDMM_ONEOFF_PAUSE_GRACE_SECS=1`, `GSDMM_ONEOFF_STOP_GRACE_SECS=1`. The outcome is `killed` with `stop {kind: pause, escalated: true}`.
      - `stopping_a_resolver_lands_nothing_and_keeps_the_branch`: a conflict fixture, and a resolver scenario that touches `resolver-started` and then waits for `$FAKE_CLAUDE_DIR/release`. `task land merge` is spawned as a child process. After `resolver-started`, `task stop ALIAS --run-id <id>` exits 0 and its stdout says it stopped the landing. Then:
        - the land child exits 1, and `assert_nothing_landed` holds;
        - `land_failure.reason == "stopped by user"` and `land_failure.stop == hard`;
        - the branch still points at the head.
      - `pausing_a_resolver_lands_nothing_even_when_it_resolves`: the resolver waits for `release`, then resolves, commits and reports done with `--verified`. `task stop ALIAS --run-id <id> --pause` exits 0, then the test creates `release`. Then:
        - the land child exits 1 and nothing landed;
        - `land_failure.reason == "paused"` and `stop == pause`;
        - exactly one `land/<n>` attempt dir exists (no further attempt);
        - a stale `land/stop.request` does not stop the next `task land merge`, which lands (the land lock start clears it).
    - TUI (unit):
      - In the list and the summary: `X` on a live run gives Ask Kill and `z` gives Ask Pause; `z` on a finished run gives a refusal message.
      - On a not-live run with `landing: true`, `x`/`X`/`z` ask about "its landing".
      - `y` dispatches the command naming the asked run and the right target.
      - Run viewer: `x`/`X`/`z` return a Push of an Ask screen for the viewed run. With the composer open, `x` lands in the buffer.
      - Resolver viewer: the Ask targets the landing.
    - app (unit): a confirmed landing stop, on a run whose landing this App started, sets that run's stop flag and writes the request. A Pause request reaches the stop entry with kind pause and `by: tui`. Test through the existing test seams; do not invent a new process spawn.
  </behavior>
  <action>
Write the failing tests first, then implement. This task covers L-1, L-2, L-4, C-01..C-08, C-13, C-14, S-01..S-03 and I-09..I-16 (I-25's `stop`/`Paused`/`Killed` part).

1. New `src/oneoff/cancel.rs`, registered in `src/oneoff/mod.rs` and added to `tests/oneoff_runtime_seam_guard.rs` `NEUTRAL_FILES`. The pure part is ungated, because store uses its types; the I/O is gated `#[cfg(any(target_os = "linux", target_os = "macos"))]` like `land`.
   - Types: `StopKind {Hard, Kill, Pause}` (serde lowercase) with `stronger`; `StopBy {Tui, Cli, Signal}`; `StopRequest {kind, requested_at, by}`; `StopRecord {kind, requested_at, ended_at: Option<String>, escalated: bool, by}` (C-07, I-13).
   - Constants: `STOP_REQUEST_FILE = "stop.request"`.
   - Graces: `STOP_GRACE_ENV`/`PAUSE_GRACE_ENV` with `stop_grace_from_env`/`pause_grace_from_env`. Generalize `session::grace_from` to take the default and keep the soft cap's behaviour byte-identical.
   - `write_request(dir, request)` and `read_request(dir)` per I-09.
   - The pure `Cancel` state machine (behavior list). It is SoftCap's shape, and the soft-cap tick hosts it (S-01, C-04, Discretion).
   - The I/O cut `cut(containment, child, kill_now, grace) -> bool killed` per I-12, with `systemctl` through `run_bounded` and groups through `rustix` `kill_process_group`.
2. `src/oneoff/runtime.rs` (S-02, I-13):
   - `HostEnd::{Paused, Killed { escalated }}`;
   - `outcome_at` mapping;
   - `pause_message()` next to `wrap_up_message`: "Pause requested: stop at a safe point, commit your work in progress on your branch, then call `gsd-meta-manager taskctl report --status partial` with a headline and one `--followup` per thing left, then stop." (exact wording is the executor's).
   Also update every `HostEnd` match (`resolve.rs`).
3. `src/oneoff/mod.rs` + `src/oneoff/store.rs` (C-05, I-15, I-25):
   - `AgentOutcome::{Paused, Killed}` with their words and the round-trip test;
   - `RunRecord.stop`;
   - `LandFailure.stop`;
   - the `synthesize` arms;
   - `OneoffCommand` gains kill and pause, plus a landing target (names are the executor's; `verb()` covers them).
4. `src/oneoff/inbox.rs` + `src/oneoff/session.rs`:
   - `InjectionKind::Pause` (snake_case `pause`);
   - `Supervisor::inject_host(kind, text, now)`, the wrap-up delivery path generalized: recorded before the write, not counted against `MAX_INJECTIONS`, refused and recorded when the input is closed.
5. `src/oneoff/launch.rs`:
   - `SessionSpec` gains `control: Option<&Path>` (the dir holding the request file) and `pause_message: Option<&str>`. The run passes its run dir and the runtime's pause message; the resolver passes `land/` and None (C-14: no message, the attempt may finish).
   - The wait loop:
     - on `user_stop` (a signal) or a request read from `control` on the tick, feed `Cancel`;
     - on `InjectPause`, call `inject_host`;
     - on `Term`/`Kill`, call `cancel::cut`;
     - after the child exits, read the request once more and classify (I-13).
     The `SessionEvent` stream gains a `Stop(&StopRecord)` event so the run persists `record.stop` on first observation. `SessionRun` returns `stop: Option<StopRecord>`, and `finalize` stores it.
   - `stop()` becomes `stop(project, run_id, kind, by, target, wait)` (C-02, C-04, I-10, I-14):
     - it resolves the run as today;
     - a run target writes the request while `run.lock` is held, then sends TERM to the daemon for hard/kill (no signal for pause);
     - a landing target (explicit, or auto when the run is not live and `landing_in_progress`) writes `land/stop.request` while `land.lock` is held, never signals the holder, and for kill also kills the resolver scope by name (I-10);
     - it waits up to `wait` for the targeted lock to go free;
     - the orphan path follows I-14.
     `StopOutcome` gains the landing and pause outcomes.
6. `src/oneoff/land.rs` + `src/oneoff/resolve.rs` (C-13, C-14, I-09, I-15):
   - `merge` deletes a stale `land/stop.request` right after `take_land_lock`.
   - `resolve_conflicts` maps the new HostEnds to a `Step::Stopped(kind)`.
   - `land_by_merge` checks the request between attempts and before every `fast_forward`.
   - A stop records `land_failure { reason, stop, conflicted, attempts }` and returns Err (`"the landing was <paused|stopped|killed>; nothing was merged"`).
   - `LandOptions` stays as is; the TUI now passes a flag it keeps.
7. CLI (C-02, I-14):
   - `src/cli.rs` `TaskAction::Stop` gains `--kill`, `--pause` (mutually exclusive) and `--resolver`, with doc comments.
   - `src/task_cli.rs` `stop_main` maps them to `stop(...)` with `StopBy::Cli`, prints which target it acted on, and applies I-14's exit codes.
   - `summary_lines`/`list_line` show `stop: <kind> by <by> at <requested_at>[, escalated]` and, while a request is pending (`ended_at` None and live), `pausing`/`stopping` (C-18).
8. TUI (C-01, C-13, I-16):
   - `src/ui/screens/oneoff.rs` `run_action_key`:
     - `x` asks "Stop … TERM, then KILL after 10 s" (the existing Stop); `X` asks Kill and `z` asks Pause;
     - an auto landing target uses `view.landing && !live`;
     - add a constructor that opens Ask for one run with a "return to viewer" origin.
   - `result_lines` shows the stop record ("Paused (pause)", "Killed (hard, escalated)", "Pausing…").
   - `src/ui/screens/oneoff_viewer.rs`: `x`/`X`/`z` outside the composer and soft input push that Ask, with the landing target in the resolver viewer.
   - `src/app.rs` `request_oneoff_command`:
     - keep a per-run `Arc<AtomicBool>` map for landings this App started (replacing `LandOptions::new`'s never-set flag) and remove the entry when the landing ends;
     - a landing-target stop sets the flag when owned and always goes through `launch::stop(...)` with `StopBy::Tui`;
     - pause and landing requests use wait 0; a run hard/kill keeps the 60 s wait.
   - `src/ui/screens/help.rs`: in all three copies, the `T` row becomes `... x stop (TERM, then KILL), X kill now, z pause, e SUMMARY, d merge diff (o: full diff in $EDITOR)`, and the live-view rows mention `x`/`X`/`z`.
   - Render-escape guard: add an Ask state with a hostile run id and a Result state with a stop record.
9. `tests/launcher_support/mod.rs`: the fake `systemctl` gains a `kill` verb. It reads `--signal=<SIG>` and the unit from the remaining args, then `kill -s <SIG> -- -<pid>` from `<unit>.pid`; the other verbs are unchanged. Write the e2e scenarios listed above (the resolver variants extend `merge_scenario`).
10. `docs/CONFIGURATION.md`:
    - a new **Stop, kill and pause.** paragraph: the three kinds, both graces, the outcomes, the stop record, the landing target and what a paused landing means;
    - env table rows for `GSDMM_ONEOFF_STOP_GRACE_SECS` and `GSDMM_ONEOFF_PAUSE_GRACE_SECS`;
    - **Keys** (`x`/`X`/`z`) and **CLI** (`task stop --kill|--pause|--resolver`).
11. Run the gates as in Task 1, writing the full log to `target/klf-t2.log`, then commit `feat(quick-261103-klf): stop, kill and pause one-off runs and their conflict resolvers`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff launcher::process && cargo test --test oneoff_e2e && cargo test --test oneoff_runtime_seam_guard && cargo clippy --all-targets -- -D warnings && cargo fmt --check && git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs</automated>
  </verify>
  <done>
- A hard stop escalates TERM to KILL after the stop grace (`killed`, escalated) or ends `stopped`; `--kill` is immediate.
- A pause is injected, recorded, honoured (`paused`, partial report, branch tracked) or escalated (`killed`, pause escalated).
- A resolver stop or pause lands nothing and records `land_failure.stop`; no holder pid is signalled.
- x/X/z work in the list, the summary and both viewers, behind a confirmation.
- Docs and help are updated in all three copies.
- `target/klf-t2.log`: 0 failed, no missing binaries. Clippy is clean on 3 targets, fmt is clean, the goldens are unchanged, and there is one commit.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: `task resume --from branch|master` (and TUI `u`) starts a new primed run from the paused/stopped/killed head, merging master first with the resolver on conflict; a paused landing resumes by retrying it; docs, help and the follow-up todo</name>
  <files>src/oneoff/resume.rs, src/oneoff/mod.rs, src/oneoff/store.rs, src/oneoff/launch.rs, src/oneoff/resolve.rs, src/oneoff/prompt.rs, src/cli.rs, src/task_cli.rs, src/app.rs, src/ui/screens/oneoff.rs, src/ui/screens/oneoff_viewer.rs, src/ui/screens/help.rs, src/ui/screens/render_escape_guard.rs, tests/oneoff_e2e.rs, tests/oneoff_runtime_seam_guard.rs, docs/CONFIGURATION.md, .planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md</files>
  <behavior>
    - resume (unit, pure):
      - `resume_refusal(view, now_resumed_dir_exists, ...)` refuses: live; landed (a Merged landing); finished on its own (outcome other than paused/stopped/killed); no head; commits > 0 with `branch_state` not Tracked; `resumed_by` set while that dir exists and its run did not fail its resume merge.
      - It allows paused, stopped and killed runs with a head, and a zero-commit pause.
      - `resume_mode` returns RetryLanding when `land_failure.stop == Some(Pause)` (`--from` given → refusal), and requires `--from` otherwise.
      - `budget` follows I-22: the override, the remainder, the floor refusal.
    - prompt (unit): `build_resume` contains the old task's prompt, the old id and outcome word, the headline, followups capped at 20, commit lines capped at 50, and for master the merge note. Control characters are cleaned, and the result stays within `MAX_PROMPT_BYTES`.
    - cli (unit): `task resume p --from branch` and `task resume p --run-id x --from master --budget 2.5 --detach` parse. `task run` accepts hidden `--resume-from oneoff-x --resume-start master` without `--todo`/`--text`, and `daemon_argv` emits them for a resume.
    - e2e (fake harness; reuse Task 2's pause scenario for the old run):
      - `a_paused_run_resumes_from_its_branch_as_a_new_run`: pause, then `task resume ALIAS --run-id <old> --from branch --detach` exits 0 and prints a new id. Then:
        - the new record has `resumed_from == old` and `base == old head`, and the old record has `resumed_by == new`;
        - the new session's scenario records `git log --format=%s` with the WIP subject, and its prompt (fake `prompt.txt`) contains the old id and the old headline;
        - a second `task resume` of the old run exits 1 ("already resumed"), and a resume of the live new run exits 1;
        - `task list` shows `resumed as <new>`;
        - the new run finishes and `land merge` fast-forwards with both runs' commits.
      - `resume_from_master_merges_the_checkout_tip_into_the_run_first`: after the pause, commit a non-conflicting file on the project's `main`; resume `--from master`. The new `base` has parents [old head, main tip] (`head_parents`-style check in the run's tracked branch history), and `resume_merge.how == merge`.
      - `a_conflicting_resume_from_master_is_resolved_by_the_agent`:
        - a conflicting commit on `main`, and a resolver scenario that resolves, commits with `git commit --no-edit`, and reports done with `--verified`;
        - `resume_merge.how == resolved`, `conflicted` lists the file, and `attempts == ["land/1"]`;
        - the agent session then ran (scenario marker present).
        A failing resolver variant ends the new run `failed` with `resume_merge.failure` set and no agent-session marker, and the old run can be resumed again (I-18).
      - `pausing_a_resolver_lands_nothing_even_when_it_resolves` (from Task 2) is extended: `task resume ALIAS --run-id <id>` (no `--from`) retries the landing and lands, and `task resume ... --from branch` on that run exits 1 naming the landing retry.
    - TUI (unit):
      - `u` on a paused run opens the resume ask; `b`/`M` dispatch the branch/master resume naming the run, and Esc returns.
      - `u` on a run whose landing was paused opens the merge Ask; `u` on a run that finished on its own gives a refusal message.
      - The viewers' `u` pushes the same ask. The summary shows `Resumed as <id>` / `Resumed from <id>` and the resume merge.
      - `run_counts` does not count a run with `resumed_by` as `to land`.
  </behavior>
  <action>
Write the failing tests first, then implement. This task covers L-3, L-4 (resume side), C-09..C-12, C-18 (resume lines) and I-17..I-25.

1. New `src/oneoff/resume.rs`, registered in `src/oneoff/mod.rs` and added to `NEUTRAL_FILES`. It holds the pure refusal/mode/budget functions (ungated) and the gated I/O.
   - `start(project, alias, run_id: Option<&str>, from: Option<ResumeFrom>, budget: Option<f64>, unsandboxed, egress_allow, by, detach, config) -> Result<ResumeStarted, String>` runs I-18's serialized sequence.
     - RetryLanding delegates to `land::merge`: the CLI's caller passes `LandOptions::with_signal_handlers`, and the TUI passes its owned flag.
     - Otherwise it builds `OneoffOptions` from the old record (I-19) with `resume: Some(ResumeSpec { from_run, start: Branch|Master })`, then calls `run_detached` (detach / TUI) or `run_foreground`.
   - Add `ResumeFrom` (clap `ValueEnum`: `branch`, `master`).
2. `src/oneoff/launch.rs` (C-09, C-11, I-19, I-20, I-24):
   - `OneoffOptions.resume: Option<ResumeSpec>`; update every literal (`task_cli.rs`, `app.rs`, `launch.rs` and its tests).
   - `daemon_argv` emits `--resume-from`/`--resume-start` instead of `--todo`/`--text` for a resume.
   - In `run_session`:
     - for a resume, after the clone, `git_read` switches `-c gsd-oneoff/<new> <old head>` and skips the todo check;
     - for master, it calls `resume::merge_master(...)` before the planted baseline (clean commit, or the generalized resolver per I-20), then sets `record.base` and `record.resume_merge`;
     - the prompt is `prompt::build_resume(...)` (I-21);
     - the session's `budget_usd` is the remainder after any resolver spend;
     - `finalize` adds the session's spend instead of overwriting it (I-24).
   - The new record gets `resumed_from`.
3. `src/oneoff/resolve.rs`: generalize `resolve_conflicts` to a parent pair (first, second), a prompt direction and the session dir/target names. The landing keeps [tip, head]; resume uses [old head, master tip]. `merge_identity` and `claim_attempt` become `pub(crate)`. The landing's behaviour and tests do not change.
4. `src/oneoff/prompt.rs`: `build_resume` (I-21), plus the resolver prompt's resume direction ("merge `<branch>` into this one-off's branch; first parent = the run head"), with the same never-rebase and both-sides rules.
5. `src/oneoff/store.rs` (C-09, C-18, I-25): `resumed_from`, `resumed_by`, `resume_merge` (all serde default). `run_counts` excludes `resumed_by` runs from `to_land`.
6. CLI (C-02, I-17, I-24):
   - `src/cli.rs`: `TaskAction::Resume { project, run_id, from: Option<ResumeFrom>, budget: Option<f64> (positive_finite_usd), detach, unsandboxed }`, plus hidden `TaskRunArgs` `resume_from`/`resume_start` with the ArgGroup change.
   - `src/task_cli.rs`:
     - `resume_main` resolves the project and its egress allowlist like `run_main`/`land_main` and calls `resume::start`;
     - it prints `task resume: <old> resumed as <new> (from <branch|master>)`, or the landing result for a retry;
     - `run_main` builds the options from the old record when `--resume-from` is given;
     - `summary_lines`/`list_line` show `resumed as`/`resumed from` and the resume merge (`resume:    merged <branch> as <sha12> [resolved: files]` or its failure).
7. TUI (C-01 `u`, C-12):
   - `src/ui/screens/oneoff.rs`: a resume-ask mode (`b`/`M`/Esc); `u` per the behavior list (RetryLanding → the merge Ask); `result_lines` resume lines.
   - `src/ui/screens/oneoff_viewer.rs`: `u` outside text entry pushes it; the resolver viewer's liveness for a resume-merge attempt follows I-20.
   - `src/app.rs`: the resume commands call `resume::start` on `spawn_blocking` with detach, `StopBy::Tui` semantics and the project's egress allowlist (the Merge pattern). A RetryLanding goes through the owned-flag landing path from Task 2.
   - `src/ui/screens/help.rs`: all three copies add `u resume (b from its branch, M merge master first)` to the `T` row and `u` to the live-view row.
   - Render-escape guard: a resume-ask state and a Result state with hostile resumed ids and resume-merge files.
8. Docs and todo:
   - `docs/CONFIGURATION.md`: a **Resume.** paragraph (C-09..C-12, I-17..I-22); **CLI** gains `task resume`; **Keys** gains `u`. Do a final consistency pass over every one-off paragraph touched by this plan, and check that `README.md`'s one-off lines make no claim this plan falsifies (edit only if one does).
   - Write `.planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md` (I-23). Use the same frontmatter shape as its sibling todos: `area: launcher`, `severity: minor`, source = this CONTEXT. It describes resuming through the agent's own session id behind the `AgentRuntime` seam, records why klf chose a fresh session (C-10), and links `2026-11-03-oneoff-raise-hard-ceiling-via-resume.md`.
9. Run the gates as in Task 1, writing the full log to `target/klf-t3.log`, and stage the todo by name. Commit `feat(quick-261103-klf): resume a paused or stopped one-off from its branch or from master`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff cli && cargo test --test oneoff_e2e && cargo test --test oneoff_runtime_seam_guard && cargo clippy --all-targets -- -D warnings && cargo fmt --check && git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs && test -f .planning/todos/pending/2026-11-03-oneoff-resume-with-the-agent-session.md</automated>
  </verify>
  <done>
- `task resume --from branch` and `--from master` (clean and resolved) start a new run at the right base, primed with the old task, report and log.
- The run links (`resumed_from`/`resumed_by`) are recorded, and refusals hold (second resume, live, landed, finished, untracked).
- A failed resume merge never strands its predecessor.
- A paused landing resumes by retrying it.
- `u` works in all four contexts.
- Docs, help and the follow-up todo are written.
- `target/klf-t3.log`: 0 failed, no missing binaries. Clippy is clean on 3 targets, fmt is clean, the goldens are unchanged, and there is one commit.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| operator (CLI/TUI) → run dir request files → launcher daemon / landing process | Host-only files, same user. Their kind decides whether the host signals or kills processes. |
| sandboxed agent / resolver → host | Agent-written commits, report text and files reach host git operations, the resume prompt and the UI. |
| git subprocess → host memory and files | The merge diff output size is agent-controlled through the commits. |
| requester → lock holder pids and scope names | Signals and `systemctl kill` hit pids or units derived from records. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-klf-01 | Denial of service | `land::merge_diff` (TUI `d`) | high | mitigate | `process::run_capped` stops reading at 5000 lines / 512 KiB (numstat at 2000 rows / 1 MiB), then kills and reaps git (Task 1, I-02/I-03). The full diff only ever goes child stdout → file (`run_to_file`). |
| T-klf-02 | Tampering | `export_merge_diff` target path | medium | mitigate | The sha is validated by `is_hex_sha`. The path is fixed under the host-only `<run-dir>/land/`. Opened `O_NOFOLLOW`, 0600; a planted symlink fails (Task 1 e2e). |
| T-klf-03 | Elevation of privilege | stop/pause request files | high | mitigate | The files live directly in `<run-dir>/` and `<run-dir>/land/`, outside every sandbox bind (I-09), so neither the agent nor the resolver can write them. Reads are bounded to 4 KiB, `O_NOFOLLOW` and strict-enum; garbage with no signal is ignored, and garbage with a signal falls back to hard. Writes are atomic and 0600. |
| T-klf-04 | Denial of service | signalling the land-lock holder | high | mitigate | The holder pid is never signalled (I-10): a TUI-owned landing's holder is the TUI. Landings poll their request file instead; the run daemon is signalled only while `run.lock` is held, by the existing `signal_target`. |
| T-klf-05 | Elevation of privilege | `systemctl --user kill` by unit name | medium | mitigate | Unit names come from run.json or are rebuilt from the run id and project hash. They pass the one-off scope plausibility check (and the `-land<n>-` shape for a resolver) before they reach `systemctl` (Task 2). |
| T-klf-06 | Tampering | resume from an agent-made head | high | mitigate | Resume requires the head to be tracked in the project, which means the clean planted gate passed at finalize (I-19). The new clone switches to the head with `git_read` (host global/system config off, hardened), so `.gitattributes` filters in agent commits cannot reach host drivers. The master merge uses hardened `git_read` with an explicit identity (I-20). |
| T-klf-07 | Tampering | resume-merge resolver output | high | mitigate | The hx7 host re-checks: planted gate on the clone, a valid `done` report with `--verified`, exact parents [old head, master tip], a clean tree and no conflict markers in previously conflicted files. The resolver is never unsandboxed (Task 3, I-20). |
| T-klf-08 | Spoofing | agent text in the resume prompt | low | accept | The headline, followups and commit subjects come from the same agent at the same trust level. They are capped (20 × 200, 50 × 200), control-cleaned and bounded by `MAX_PROMPT_BYTES` (I-21). |
| T-klf-09 | Repudiation | who stopped or paused a run | low | mitigate | `run.json` `stop {kind, requested_at, ended_at, escalated, by}` (C-07, I-13) and the `pause` record in `injections.jsonl`. |
| T-klf-10 | Information disclosure | TUI rendering of numstat paths, stop/resume fields | medium | mitigate | Everything drawn goes through `render_for_terminal`. The render-escape guard gains hostile-name fixtures for the diff summary, the Ask, the stop and resume lines (Tasks 1-3). |
| T-klf-11 | Denial of service | the grace env vars | low | accept | Read by the host at session start, bounded 1..=600 s, documented as tests-only. The agent cannot set the daemon's environment. |
| T-klf-12 | Tampering | concurrent resumes of one run | medium | mitigate | `resumed_by` is checked and written under the old run's land lock (I-18). Task 3 e2e asserts that a second resume is refused. |
| T-klf-SC | Tampering | npm/pip/cargo installs | high | mitigate | No package is installed or added: the plan forbids a new crate dependency, and `Cargo.toml`/`Cargo.lock` are not staged. No legitimacy checkpoint is needed. |
</threat_model>

<verification>
- Per task: the `<automated>` command, then the full suite raw: `rtk proxy cargo test --no-fail-fast > target/klf-tN.log 2>&1`. Read the log, and compare the per-binary result lines with `target/klf-t0.log` (no binary missing, 0 failed).
- `cargo clippy --all-targets -- -D warnings`, plus the same with `--target x86_64-pc-windows-gnu` and `--target aarch64-apple-darwin` (both installed). `cargo fmt --check`.
- `git diff --exit-code e9ebe29 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs tests/launcher_push.rs tests/spawn_seam_guard.rs tests/launcher_sandbox_data_mask.rs` is empty (S-04: the bounded reader needed no seam-guard registration).
- `cargo test --test oneoff_runtime_seam_guard` lists `src/oneoff/cancel.rs` and `src/oneoff/resume.rs` and passes; `cargo test --test spawn_seam_guard` passes.
- `git rev-list --count <plan base>..HEAD` equals the number of tasks committed (measured, not narrated, in the SUMMARY).
</verification>

<success_criteria>
- (a) run.json refs: the ff range, merge range, cleared branch after landing and discard, and readable pre-klf files.
- (b) The bounded diff: streamed with a cap, a summary of every file, the truncation marker, a full export opened in $EDITOR, and the CLI command.
- (c) Hard cut (TERM → KILL after the grace) and immediate kill, for runs and resolvers, from the CLI and from x/X in all four TUI contexts.
- (d) Soft pause of runs: the injected message, the `paused` outcome, the tracked branch, escalation to `killed`.
- (g) Resolver pause: nothing lands, no further attempt, and `u`/`task resume` retries the landing.
- (e)/(f) `task resume --from branch|master` (clean and resolver-resolved) makes a new, primed run linked both ways.
- `docs/CONFIGURATION.md` and `src/ui/screens/help.rs` (three identical copies) describe all of it. The C-10 follow-up todo exists, and no Deferred item from CONTEXT was built.
- No paid agent run, no push, no tag, no rebase. Three commits on master.
</success_criteria>

<output>
Create `.planning/quick/261103-klf-cancel-pause-one-off-runs-and-resolvers-/261103-klf-SUMMARY.md` when done. Include:
- the commits table;
- shipped vs deferred, with any fallback todos;
- the baseline and final per-binary test counts;
- every `[inferred — audit]` decision (C-01..C-21, I-01..I-25, and any new executor ones);
- the three hx7 audit items marked closed;
- "Unverified without a paid run": a real agent honouring a mid-turn pause message, real `systemctl kill` on a real scope, and a real resume session continuing well from the primed context.
</output>
