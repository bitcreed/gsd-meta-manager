# Quick Task 261103-pd7: Fix one-off sandbox-sensitive tests, already-landed runs, stale preservation, land.lock churn, TESTING.md fixture note - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning

Human unavailable; every decision below was taken by the orchestrating agent from the
existing planning artifacts (quick tasks hx7, klf, mnr, o2r) and is marked
`[inferred — audit]`.

<domain>
## Task Boundary

Findings from the second real paid one-off test (runs `oneoff-20261104-000606` = A,
`oneoff-20261104-000607` = B, under `~/.local/share/gsd-meta-manager/oneoff/<hash8>/`):

1. **Sandbox-sensitive test blocks every resolver landing.** Inside the one-off sandbox
   the hostname is `gsdmm-sandbox` (`src/launcher/sandbox.rs:92`). The test
   `a_launch_without_a_posture_writes_neither_key_and_clears_a_stale_one`
   (`tests/driverctl_store.rs:1659`) asserts `!text.contains("sandbox")` on DIRECTION.md,
   which contains the host line, so it fails inside the sandbox; the resolver then reports
   `partial` and landing is refused.
2. **A run whose head is already contained in the target cannot be recorded as landed.**
   `task land merge` prints `the run's head is already in master; nothing to merge
   (discard the run)` and exits 1 (`src/oneoff/land.rs:~839`; same message at
   `src/oneoff/resolve.rs:~607`).
3. **Stale preservation message.** `task list` keeps reporting a failed resolver clone as
   "commit 4f38f18 ... is not in any branch" although master is at 4f38f18.
4. **Recurring `land.lock`.** `task list` removes a stale `land.lock`, then it reappears.
5. **Docs inaccuracy.** `docs/TESTING.md` fixtures table, `oneoff/` row (and the prose near
   line 246), says the meter test loads its fixture with `include_str!`; it reads it by
   path (`src/oneoff/stream.rs:~1058`).

</domain>

<decisions>
## Implementation Decisions

### D-01 Sandbox test fix and sweep
- Fix the failing test to assert on the frontmatter key, not any substring: no line
  starting with `sandbox:` / `sandbox_missing:` in the frontmatter (the parsed
  frontmatter check already in the test stays). `[inferred — audit]`
- Sweep every test (`tests/**`, `#[cfg(test)]` modules in `src/**`) for assertions that
  depend on the host environment and would fail inside the one-off sandbox: hostname
  (`gsdmm-sandbox`), `$HOME`/user name, `/tmp` vs `$TMPDIR`, network access, ambient
  git config (user.name/email, init.defaultBranch, signing), locale/TZ. Fix each one by
  making the test hermetic (assert the specific key/field, set the env/git config the
  test needs explicitly) — never by skipping it in the sandbox. `[inferred — audit]`
- The sandbox's environment is defined by `src/launcher/sandbox.rs`; the sweep reads it
  to know what differs (hostname, home, network, mounts).
- Guard against regression with a **doc note** in `docs/TESTING.md` (the suite must pass
  inside the one-off sandbox; list what differs there and the rule "assert keys, not
  substrings that a hostname/path can contain"). No new CI job: a CI-in-sandbox job needs
  bwrap on the runner and is out of scope for a quick task. `[inferred — audit]`
- Optional cheap hardening, executor's call: a test that renders DIRECTION.md with a host
  name containing `sandbox` and checks the no-posture path still writes no `sandbox:` key
  (pins the fix without needing the real sandbox). `[inferred — audit]`

### D-02 Already-landed is a successful landing
- Add a third `LandingKind` variant, `AlreadyLanded`, serialized `already-landed`
  (explicit `#[serde(rename = "already-landed")]`, since `rename_all = "lowercase"` would
  give `alreadylanded`). Do not overload `Merged`. `[inferred — audit]`
- Record what contains it: the target branch, the target tip at detection, and,
  best-effort, the first commit on the target's first-parent history whose ancestry
  includes the head (e.g. the merge 4f38f18 for run A) as the landing's sha/detail. If
  that lookup fails, record the tip alone. `[inferred — audit]`
- Every place that reads "this run's work is on the target" (today `kind == Merged`:
  `store.rs` `branch_state`, the `landed`/`is_landed`-style checks around store.rs:397,
  408, 691, `resume.rs:90`, `land.rs:706`, UI screens) must treat `AlreadyLanded` the same
  as `Merged` for landed-ness, branch-gone display and resume refusal. Introduce one
  helper (e.g. `LandingKind::puts_work_on_target()` / `Record::is_landed()`) instead of
  sprinkling `||`. `last_merge_landing` (the `d` diff) stays Merged-only — an
  already-landed record has no merge commit of its own. `[inferred — audit]`
- `task land merge` on a contained head: under the land lock, record `AlreadyLanded`,
  then slim the run dir and delete the tracked branch exactly as after any merge landing;
  print a one-line success (`the run's head is already in <branch> (<sha7>); recorded as
  landed`) and exit 0. The resolver path (`resolve.rs`) does the same instead of
  `fail(...)`. `[inferred — audit]`
- Display: `task list`/`task status`/TUI show it as landed with how=`already` (or the
  kind word `already-landed`); exact wording is the executor's call, consistent with
  existing `ff`/`merge`/`resolved` words. `[inferred — audit]`
- Backward compatibility: older binaries cannot parse the new variant. Acceptable — the
  store is local to one user and only this binary writes it; note it in the SUMMARY.
  `[inferred — audit]`

### D-03 Scan auto-records contained runs
- The crash/reconcile scan (the one `task list` and TUI load run) detects runs that are:
  not running/paused, not already landed, have a recorded head and a recorded target
  branch that still exists, and whose head is an ancestor of that branch's tip. It
  records them `AlreadyLanded` and slims + deletes the tracked branch, same path as D-02.
  This also covers a manual ff/merge done by the user outside the tool. `[inferred — audit]`
- The scan takes the land lock **non-blockingly**; if it is held (a landing in
  progress), skip that run this time. It must release and remove the lock it took (D-05).
  `[inferred — audit]`
- Detached/legacy targets (no branch name) are not auto-recorded. Runs whose target
  branch is gone are left alone. `[inferred — audit]`
- Scan is read-mostly today; this makes it write. Keep it cheap: one `merge-base
  --is-ancestor` per candidate run, only for candidates (not every run every frame in the
  TUI — follow wherever the existing reconcile already writes, e.g. stale-lock removal).
  `[inferred — audit]`

### D-04 Preserved resolver clone re-evaluated at scan time
- Whatever writes "commit X ... is not in any branch" must re-evaluate containment on
  every scan instead of reading a stored verdict: if the preserved commit is now
  reachable from any local branch of the project (or is already contained in the run's
  target), the preservation is obsolete — remove the preserved clone and the message.
  If still unreachable, keep it (never delete the only copy of a commit).
  `[inferred — audit]`

### D-05 land.lock churn
- Root-cause first (likely suspects: the scan/probe opens the lock path with create, or
  a lock holder releases the flock without unlinking the file, or the scan's own
  reconcile takes the land lock). Fix so that: probing never creates the file; any code
  that takes the land lock removes the file when it releases it; after a landing (and
  after a scan) no `land.lock` remains in the run dir. Record the actual root cause in
  the SUMMARY with file:line. `[inferred — audit]`
- Same rule checked for the sibling `run.lock`/`lock` files only insofar as the same
  helper is shared; no broader lock redesign. `[inferred — audit]`

### D-06 TESTING.md fixture row
- Correct both the fixtures-table `oneoff/` row and the prose near line 246: the
  session-lifecycle tests use `include_str!`, the meter test reads
  `tests/fixtures/oneoff/real-bg-subagents.stream.jsonl` by path via
  `env!("CARGO_MANIFEST_DIR")`. `[inferred — audit]`

### Claude's Discretion
- Exact names of the helper(s), message wording, how the containing commit is found,
  and test layout. Tests for D-02..D-05 use real temp git repos like the existing land
  tests (no paid `claude` runs, no network).

</decisions>

<specifics>
## Specific Ideas

- After the code lands, operationally (no paid run): build `target/release/gsd-meta-manager`
  and run `task list .` / `task land merge . oneoff-20261104-000606` in the project so
  run A (work already on master via merge 4f38f18) is recorded landed and slimmed; its
  dir should be < ~1 MB, branch `gsd-oneoff/oneoff-20261104-000606` gone, no `land.lock`
  left. Do not discard records. This is done by the orchestrator after verification, not
  by the plan's automated checks (it touches the user's real data dir).
- Testing rule: `cargo test --no-fail-fast`; read raw output via `rtk proxy`, never
  piped through grep.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/quick/261103-o2r-one-off-run-cleanup-crash-recovery-and-l/*SUMMARY.md` (cleanup, crash recovery, land-to-dispatch-branch)
- `.planning/quick/261103-mnr-fix-one-off-session-lifecycle-pause-deli/*SUMMARY.md`
- `.planning/quick/261103-hx7-parallel-one-offs-per-project-tracked-br/` (C-21 MergeHow, tracked branches)
- `.planning/quick/261103-klf-cancel-pause-one-off-runs-and-resolvers-/` (I-05 branch state, I-13 land lock, I-18)
- `docs/TESTING.md`
- `src/launcher/sandbox.rs` (sandbox environment)

</canonical_refs>
