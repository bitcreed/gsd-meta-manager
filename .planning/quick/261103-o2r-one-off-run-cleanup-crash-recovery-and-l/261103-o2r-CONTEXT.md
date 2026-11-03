# Quick Task 261103-o2r: One-off run cleanup, crash recovery and land-to-dispatch-branch - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended — the human was unavailable; every decision below was inferred by the
orchestrator from the task brief, the prior one-off quick tasks (f5y, gt1, hx7, klf, mnr) and
the code (`src/oneoff/*`, `src/task_cli.rs`). Items marked `[inferred — audit]` are for later audit.

<domain>
## Task Boundary

No leftover one-off run dirs; land back to the branch the run was dispatched from.

1. **Cleanup after landing.** After a successful merge landing (ff, clean merge, resolver merge)
   or a successful `land push`, delete the clone (`<run-dir>/work`) and all bulky/temporary data
   (resolver `land/<n>/` clones, temp diff files, inbox leftovers). Keep a small record so
   `task list` and TUI history still show the run. `discard` removes everything (unchanged).
   Failed/unlanded runs keep their clone until landed or discarded.
2. **Crash recovery / recycling.** A run whose launcher died (crash, reboot, OOM, killed TUI)
   must not leave orphaned clones/scopes/locks forever. Detect dead runs and reconcile them;
   clean orphan dirs; retention policy for finished-run records. Never delete a clone whose
   commits are not preserved in a branch.
3. **Land target = where HEAD pointed at dispatch.** Record the branch (symbolic HEAD) and its
   tip at dispatch; landing merges into that recorded branch, not whatever is checked out.
   `task land merge --into <branch>` override. Legacy run.json without the field: current
   behaviour. Target branch shown in TUI/CLI.
4. Docs/help updates.

Run dirs: `~/.local/share/gsd-meta-manager/oneoff/<hash8>/<run-id>/` (run.json, run.lock,
land.lock, work/, report/, inbox/, injections.jsonl, stream.jsonl, launch.log, land/<n>/).

</domain>

<decisions>
## Implementation Decisions

### A. What the slim record keeps after a landing / push / crash cleanup
- Keep: `run.json`, `report/` (report.json), `injections.jsonl`, the run's SUMMARY.md copied out
  of the clone into the run dir before the clone is deleted (so the summary view still works
  — `verified_summary` must fall back to the kept copy), and a **tail-truncated**
  `stream.jsonl`: the last 256 KiB cut at a line boundary, with a first line marking the
  truncation. `launch.log` likewise tail-truncated to 64 KiB. Total target < 1 MB.
  `[inferred — audit]` (no compression dependency is in Cargo.toml; truncation keeps the
  viewer's tail-oriented reading working without a new crate).
- Fold anything derived from the full stream (spend_usd, rate-limit/meter facts) into run.json
  BEFORE truncating, so nothing that reads it later depends on the dropped head. `[inferred — audit]`
- Delete: `work/`, every `land/<n>/` resolver clone, `inbox/` leftovers, temp/diff files,
  stale request files. Locks (`run.lock`, `land.lock`) are removed only after being released.
- Record the pruning in run.json (e.g. `pruned_at` + a short note) so the CLI/TUI can say
  "clone removed after landing" instead of showing missing files as an error. `[inferred — audit]`
- Cleanup failures after a successful landing/push never turn the landing into a failure:
  they are a warning, and the crash/recycle scan retries later. `[inferred — audit]`
- After `land push` the local tracked branch `gsd-oneoff/<id>` is kept (the work is not merged
  locally); the clone is still deleted because the commits live in that branch and on the
  remote. If there is no tracked branch for a push-only run, the clone is kept. `[inferred — audit]`

### B. Crash recovery / recycling scan
- **Dead run** = record not `Finished` (Preparing/Running) and its `run.lock` is free (the existing
  `LiveState` "stale"/daemon-gone derivation) and its systemd scope unit, if recorded, is not
  active. Lock probe `Unknown` = never touched. A held `land.lock` = never touched.
- Reconcile: set `status: Finished`, `ended_at`, an outcome meaning crashed (new
  `AgentOutcome::Crashed { detail }` variant, serde-compatible with old records), and a warning.
  Then preserve: if the clone's work branch has commits beyond `base`, run the existing
  `track_branch` (with its planted-file gate) to make `refs/heads/gsd-oneoff/<id>`; only when that
  ref exists and equals the clone head is the clone deleted. If tracking is refused (planted
  findings, git error) the clone is KEPT and the reason is recorded in `tracking_note`.
  A clone with no commits beyond base is deleted. `[inferred — audit]`
- Also remove: stale `land/<n>/` clones whenever `land.lock` is free and no landing is in
  progress; stale lock files whose lock is free; temp/request files of finished runs.
- Finished runs that still carry a clone **and** are already landed (landed merge/push recorded)
  — i.e. leftovers from before this change — get the same slimming. Finished unlanded runs keep
  their clone (that is the unmerged work).
- **Orphan dirs** (no valid run.json) under a project dir: removed only when no lock in them is
  held and the dir is older than 1 hour (avoids racing a run being created). If it holds a
  `work/` clone with commits and its dir name is a valid run id, preserve its HEAD as
  `gsd-oneoff/<dirname>` first (same planted-file gate); if that is refused, keep the dir and warn.
  `[inferred — audit]`
- **When:** on TUI start (background thread, never blocking the render loop; results surfaced
  via the existing message/refresh path), and on `task list` / `task run` for the project in
  question (cheap: directory listing + lock probes; git work only for runs that need it).
  `[inferred — audit]`
- **Retention of finished-run records:** default keep 30 days AND at most the newest 50 finished
  records per project; only *terminal-and-clean* records age out (landed, or finished with no
  clone and no unlanded tracked branch). A record whose work is unlanded (clone present or
  `gsd-oneoff/<id>` branch exists) is never aged out. Configurable via config.toml keys if a
  natural config place exists; otherwise constants. `[inferred — audit]`
- Hard invariant (from the brief): **never delete a clone whose commits are not preserved in a
  branch** of the project. Every deletion path checks this, with a test.

### C. Land target = branch at dispatch
- At dispatch record `dispatch_branch: Option<String>` (short name from `symbolic-ref`) and
  `dispatch_tip: Option<String>` (the commit the clone starts from — already `base`) in run.json,
  `#[serde(default)]`. A resume inherits the old run's dispatch branch. `[inferred — audit]`
- **Detached HEAD at dispatch:** allowed; record `dispatch_tip`, `dispatch_branch = None`, and
  `dispatch_detached = true`; `task land merge` then refuses without `--into <branch>` with a
  clear message. `[inferred — audit]` (refusing dispatch would block a legitimate workflow; the
  choice can be deferred to land time without loss.)
- **Legacy record** (no dispatch fields): current behaviour — land into the checked-out branch.
- **Target resolution order:** `--into <branch>` > recorded `dispatch_branch` > (legacy) current
  checked-out branch. `--into` must name an existing local branch (refs/heads), validated as a
  ref name; no branch creation. `[inferred — audit]`
- **Target is the checked-out branch of the project checkout:** existing flow unchanged (clean
  checkout required, `git merge --ff-only` in the working tree, resolver for conflicts).
- **Target is NOT checked out:** never touch the working tree or the index; no clean-checkout
  requirement. Fast-forward = `git update-ref refs/heads/<b> <new> <old>` (expected-old guard).
  Non-ff = merge made in the private land clone (existing merge/resolver flow, based on the
  target branch's tip instead of the checkout's HEAD), then fetched into the project and
  `update-ref` with the expected old value. A changed old value = refuse, nothing changed.
- **Target checked out in another linked worktree** of the project: refuse (update-ref would
  desync that worktree's index), telling the user to land from that worktree or check it out.
  `[inferred — audit]`
- Target branch deleted since dispatch: refuse with a hint to pass `--into`.
- The `Landing` record and land output name the target branch. `task list`/`task show` and the
  TUI one-off screen show "→ <branch>" (or "detached @<sha7>" / "checked-out branch (legacy)").
- `land push` is unaffected by the target (it pushes to `gsd-auto/<alias>/<id>`).
- `--from master` resume (klf) keeps its current semantics; out of scope to retarget it.

### Claude's Discretion
- Exact struct/field names, module split (a new `oneoff/recycle.rs` is reasonable), message
  wording, and how the TUI start scan reports results.
- Tests: use fake tooling and temp git repos only — no paid `claude` runs.

</decisions>

<specifics>
## Specific Ideas

- Observed today: after `task land merge` the run dir + ~115 MB `--no-hardlinks` clone stay on
  disk until `task land discard` is run by hand — this is the primary bug to kill.
- Reuse existing seams: `track_branch`, `land_lock_path`/`take_land_lock`, `probe_at`,
  `derive_live`, `read_view`, `discard`, `push::current_branch_and_sha`.
- Never rebase; commits land on master; `cargo test --no-fail-fast` must pass; clippy clean.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/quick/261103-f5y-*/` — one-off dispatch design (SUMMARY.md)
- `.planning/quick/261103-gt1-*/` — live viewer + injection
- `.planning/quick/261103-hx7-*/` — parallel runs, per-run locks, tracked branches, resolver landing
- `.planning/quick/261103-klf-*/` — cancel/pause/resume
- `.planning/quick/261103-mnr-*/` — session lifecycle / meter fixes
- `src/oneoff/mod.rs` module doc (trust split)

</canonical_refs>
