# Quick Task 261103-klf: Cancel/pause one-off runs and resolvers, accurate run.json git refs, bounded merge diff - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended. The human is unavailable. The "Locked by the user" items come from the user's task brief. Every item marked `[inferred — audit]` was decided by the orchestrator from the brief, the prior quick tasks (261103-f5y, 261103-gt1, 261103-hx7) and the code, and is recorded for a later audit.

<domain>
## Task Boundary

Builds on the one-off subsystem (`src/oneoff/`, quick 261103-f5y + gt1 + hx7). Closes the hx7 SUMMARY's
open audit items: "the merge diff reads git's whole output before capping", "a landing's
`tracked_branch` is not cleared in `run.json`", "the TUI cannot cancel a running resolver".

1. Two ways to cancel — **hard cut** and **soft pause** — for a running one-off agent AND a running
   conflict resolver, from the TUI and the CLI; pause is **resumable** (`task resume`).
2. `run.json` git references that reflect reality after landing, pause and discard.
3. A bounded, streamed merge diff with a summary, a truncation marker and a way to open it in full.
4. docs/CONFIGURATION.md and help.rs.

If it proves phase-sized, ship in this priority order and record the rest as follow-up todos in
`.planning/todos/pending/`: (a) run.json refs, (b) bounded diff, (c) hard cut runs + resolvers CLI+TUI,
(d) soft pause of runs, (e) `task resume --from branch`, (f) `--from master` with resolver, (g) resolver
soft pause. [inferred — audit]

</domain>

<decisions>
## Implementation Decisions

### Locked by the user (do not revisit)
L-1. Hard cut = immediate stop of the systemd scope / process group, SIGTERM then SIGKILL after a short
     grace; nothing lands. For a resolver: abort the merge attempt, nothing lands, the run returns to its
     unlanded state with its branch intact.
L-2. Soft pause = inject a message via the existing inbox asking the agent to stop at a safe point: commit
     WIP on its branch, report via `taskctl report`, end. Host fetches the branch as `gsd-oneoff/<id>`,
     outcome `paused`. A grace timeout escalates to hard cut.
L-3. Resumable two ways: `task resume <run> --from branch` and `--from master` (merge current master into
     the run branch first; LLM resolver on conflict). Resume starts a new sandboxed session.
L-4. Resolver soft pause = finish/abort the current attempt cleanly and leave the run unlanded; resuming =
     retry landing.
L-5. run.json: after a merge landing record the merge commit (ff: the landed range `base..head`), not a
     deleted branch; on pause record `gsd-oneoff/<id>` and its head; on discard no stale branch ref. Old
     run.json files stay readable. UI/CLI display what is recorded.
L-6. Bounded diff: never read the whole `git diff` into memory; stream and stop at the cap (kill/close the
     child); name-status/stat summary for everything, full hunks up to a byte/line limit, a
     "truncated — N more files/lines" marker, a way to open the full diff externally.
L-7. No rebases, no pushes, no tags, no real paid `claude` runs (fake claude/bwrap/systemd-run only).

### Cancel surface (keys and verbs)
C-01. TUI keys, same meaning in the run list, the run summary and both viewers (run viewer and resolver
      viewer `r`) [inferred — audit]:
      - `x` = hard cut (existing key, existing confirm; now TERM → KILL escalation after a grace).
      - `X` = immediate SIGKILL of the whole scope/process group (confirm).
      - `z` = soft pause ("take a break", z = snooze) (confirm).
      - `u` = resume a paused run; asks `b` = from its branch / `M` = merge master first / Esc.
        For a run whose latest land failure is `paused` (resolver pause), `u` = retry landing (same as `m`).
      Planner must grep help.rs + oneoff.rs + oneoff_viewer.rs + detail.rs Driver-tab handlers to confirm
      `X`, `z`, `u` are free in those contexts; if one collides, pick a free key and record it.
      The viewer's composer/inject mode must not swallow these when it is not in text-entry mode only.
C-02. CLI [inferred — audit]: `task stop <project> [--run-id] [--wait]` keeps meaning hard cut (TERM→KILL);
      new flags `--kill` (immediate SIGKILL) and `--pause` (soft pause; `--wait` then bounds waiting for
      the pause to complete, escalation is the daemon's job, not the CLI's). `--resolver` targets the run's
      in-progress landing instead of the agent (else auto: if the run is not live but its land lock is
      held, `task stop` targets the resolver and says so). New verb
      `task resume <project> [--run-id] --from branch|master [--detach]`.
C-03. Hard-cut grace before SIGKILL: 10 s default, overridable for tests by an env var in the same style as
      `GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS` (1..=600). [inferred — audit]
C-04. Who escalates: the run's own launcher daemon (it already owns the scope and the child). The CLI/TUI
      only *request* (signal or request file); the daemon does TERM → wait grace → `systemctl --user kill
      --signal=KILL <scope>` / killpg KILL. For `--kill` the daemon goes straight to KILL. If no daemon is
      alive (orphan scope), the requester does it itself as today's `OrphanScopeStopped` path does.
      Mechanism to carry the kind from requester to daemon: a small host-written request file in the run
      dir (`stop.request` with `hard|kill|pause` + requested_at), then SIGTERM for hard/kill as today
      (the daemon reads the file on signal; absent file = hard, keeps old CLIs working); pause sends no
      signal — the daemon polls the request file on its existing supervisor tick. [inferred — audit]

### Outcomes and records
C-05. Outcomes: hard cut → `stopped` (existing `AgentOutcome::Stopped`); SIGKILL (immediate or escalated)
      → new `killed`; soft pause that ended within grace → new `paused`; pause whose grace ran out → `killed`
      with a `stop` record saying it was an escalated pause. New enum variants are additive so old run.json
      still deserializes; unknown future words are not required to parse. [inferred — audit]
C-06. Agent-side status on pause: reuse `partial` in `taskctl report` (no taskctl schema change, the pause
      prompt tells the agent to report `partial` with what is left); the host — not the agent — decides
      outcome `paused` from the pause request + clean exit. [inferred — audit]
C-07. New `run.json` field `stop: Option<StopRecord { kind: hard|kill|pause, requested_at, ended_at,
      escalated: bool, by: tui|cli }>` (serde default). [inferred — audit]
C-08. On pause the branch is tracked exactly by hx7's finalize rules (commits > 0, clean planted gate,
      never overwritten). A pause with zero commits is still `paused` and resumable from its base with a
      tracking_note saying there was nothing to track. [inferred — audit]

### Resume
C-09. Resume creates a **new run id** (new run dir, lock, stream, budget) whose clone starts at the paused
      run's head (`--from branch`) or at a host-made merge of master into that head (`--from master`).
      New fields `resumed_from: Option<String>` on the new record and `resumed_by: Option<String>` on the
      old one. The old run stays readable; it is excluded from the "to land" count once `resumed_by` is
      set; its tracked branch is left in place (ancestor of the new one) and deleted only by its own
      discard. [inferred — audit]
C-10. Session strategy: a **fresh** sandboxed session primed with the old run's task, its report (headline,
      what's left), its commit log `base..head` and a note that it is a continuation — NOT claude
      `--resume <session-id>`. Reason: the session transcript lives in the per-run config dir inside the
      old sandbox, its recoverability is unverifiable without a paid run, and the fresh path is fully
      testable on the fake harness. Record `claude --resume` behind the AgentRuntime seam as a follow-up
      todo. [inferred — audit]
C-11. `--from master` = in a private `--no-hardlinks` clone, `git merge --no-ff <master-tip>` into the run
      head (parents: run head first, master second — the opposite of landing). Clean → host-made merge
      commit. Conflicts → the existing hx7 resolver path (same budget rule: new run's budget, floor $0.25,
      same host re-checks). The resulting commit becomes the new run's base; the merge commit is recorded
      on the new run (`resume_merge: Option<Landing-like {how, merge_sha, conflicted, attempts}>`). Budget
      for the resumed run = the old run's max budget minus its recorded spend, floor as hx7, overridable
      by `--budget`. [inferred — audit]
C-12. Resume refuses: a run that is live, not paused/stopped/killed-with-head, already resumed
      (`resumed_by` set), or already landed. Hard-cut (`stopped`/`killed`) runs with a tracked head are
      resumable too with `--from branch|master` (same code path). [inferred — audit]

### Resolver cancel
C-13. Resolver hard cut: the landing process (TUI in-process or `task land merge` in a terminal) owns
      `LandOptions.user_stop`. The TUI keeps a per-run `Arc<AtomicBool>` it can set (replacing the
      never-set flag); the CLI landing writes a holder record (pid) into the land lock like the run lock
      does, so `task stop --resolver` can SIGTERM it (its signal handlers set `user_stop`). Nothing lands;
      `land_failure` reason `stopped by user`; the run's branch stays; the land clone is kept as today.
      `X` on a resolver = KILL of the resolver scope `<run-id>-land<n>` directly. [inferred — audit]
C-14. Resolver soft pause: a `land/pause.request` file; the current attempt is allowed to finish within the
      same grace (C-03 env, default 300 s like the soft cap) but its result is NOT landed, no further
      attempts start, `land_failure` reason `paused`; grace expiry → hard cut. Resume (`u` / `task resume`)
      on such a run = retry landing (`task land merge`). [inferred — audit]

### run.json git references
C-15. Landing: `ff` records `range: "<old target>..<head>"` (new `Landing.range: Option<String>`) and
      `merge_sha` = head; merge/resolved keep `merge_sha` and also record `range`
      `<old target>..<merge_sha>`. After the landing deletes the branch, set `tracked_branch = None` and
      `tracking_note = "deleted after landing"`. [inferred — audit]
C-16. Discard: `tracked_branch = None` (the record survives only if the discard keeps the run dir; if the
      run dir is removed, nothing to do). Pause: `tracked_branch = Some("refs/heads/gsd-oneoff/<id>")`,
      `head = <sha>`. [inferred — audit]
C-17. Backward compatibility: all new fields `#[serde(default)]`; a reader-side normaliser treats an old
      record with a `Merged` landing and a still-set `tracked_branch` as "branch gone" (the existing display
      rule), so no migration rewrite of old files. A test deserialises a pre-klf fixture. [inferred — audit]
C-18. Display: `task status` / `task list` / the TUI summary show `Branch: gsd-oneoff/<id> @ <sha12>` while
      tracked, `Landed: <range>` / `Merge: <sha12>` after landing, `Paused/Stopped/Killed (<kind>)` with the
      stop record, and `Resumed as <id>` / `Resumed from <id>`. [inferred — audit]

### Bounded merge diff
C-19. A new bounded-read helper lives in the process seam (whatever module `spawn_seam_guard` allows to
      spawn): spawn hardened git, read stdout up to N bytes / L lines, then kill + reap the child; return
      `(text, truncated: bool)`. Caps: keep hx7's 5000 lines / 512 KiB for hunks. [inferred — audit]
C-20. View = header (`conflicted: ...`) + `git diff --numstat` summary for ALL files (itself bounded, e.g.
      2000 files, with its own "N more files" line) + hunks to the cap + marker
      `truncated — N more files, ~M more lines` computed from numstat totals minus what was shown.
      [inferred — audit]
C-21. Full diff externally: `o` in the diff view streams the full `git diff` straight to a file
      `<run-dir>/land/merge-<sha12>.diff` (child stdout → file, never through memory) and opens it with the
      existing `$EDITOR` suspend path (src/ui/screens/mod.rs "open a file in $EDITOR"). CLI: `task status`
      prints the exact `git diff <sha>^1 <sha>` command; no new `task diff` verb. [inferred — audit]

### Claude's Discretion
- Exact struct/enum names, module placement, the test split, wording of prompts and messages.
- Whether `u`'s two-way choice is an Ask popup or a sub-mode, as long as Esc cancels.
- How the daemon's supervisor tick polls `stop.request` (reuse the soft-cap tick).

</decisions>

<specifics>
## Specific Ideas

- Reuse: `session::SoftCap`-style grace timer and `grace_from_env`; `inbox` injection for the pause message;
  `runtime::HostEnd` gains `Pause`/`Kill` variants mapping to the new outcomes; `resolve.rs` already maps
  `HostEnd::UserStop` to "the landing was stopped; nothing was merged".
- The pause message text lives in `runtime.rs` (seam) next to `wrap_up_message`, so no agent name leaks.
- Tests: fake claude must be able to (a) honour a pause message by committing and running `taskctl report
  partial`, and (b) ignore it (to exercise escalation); fake systemd-run/bwrap already exist in
  tests/oneoff_e2e.rs. Use short grace env overrides. `cargo test --no-fail-fast`; clippy `-D warnings`
  for host + the two cross targets hx7 checked; `cargo fmt --check`.
- Golden/guard files from hx7 (`tests/launcher_sandbox.rs`, `launcher_config_dir.rs`, `launcher_push.rs`,
  `spawn_seam_guard.rs`) must stay byte-identical unless the bounded-read helper must be registered in
  the seam guard's allowlist — then record it as a deviation.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/quick/261103-f5y-one-off-sandboxed-task-dispatch-from-the/` (dispatch, clones, runtime seam, taskctl, landing)
- `.planning/quick/261103-gt1-live-viewer-with-message-injection-and-s/` (stream-json stdin, inbox, viewer, soft cap)
- `.planning/quick/261103-hx7-parallel-one-offs-per-project-tracked-br/` (parallel runs, tracked branches, resolver, multi-run UI; SUMMARY "Audit items worth a human look")
- `docs/CONFIGURATION.md` (one-off sections), `src/ui/screens/help.rs` (three identical copies of the one-off rows)

</canonical_refs>
</content>
</invoke>
