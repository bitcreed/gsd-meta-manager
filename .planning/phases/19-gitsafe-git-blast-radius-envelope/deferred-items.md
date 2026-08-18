# Deferred Items — Phase 19

Out-of-scope discoveries logged during execution. Not fixed, by the scope
boundary rule: they are not caused by the current plan's changes.

## `tests/driver_reattach.rs` is intermittently flaky (found during 19-04)

**Symptom.** Two tests fail non-deterministically:

- `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` —
  `assertion left == right failed: exactly one project has a run to observe`,
  `left: 0`.
- `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
  — `the run record is on disk: Os { code: 2, kind: NotFound }`.

Both look like the same race: the spawned driver's process is live (the liveness
probe finds its cmdline) before it has written `run.json`, so the reconcile scan
that follows finds nothing on disk.

**Proved pre-existing.** The wave base `5e1170b` was extracted with `git archive`
into a clean directory and `cargo test --test driver_reattach` reproduces the
same two failures there, with plan 19-04's changes absent entirely. Observed
passing 3/3 on some runs and failing 2/3 on others in both trees.

**Why it is not fixed here.** Nothing in 19-04 touches `src/driver/`, and the
envelope is not wired into the driver until plan 19-06. Fixing a spawn/write race
in the reattachment path is a change to Phase 17's code with its own verification
needs, not a deviation of this plan.

**Suggested owner.** A follow-up quick task, or plan 19-06 if it finds the same
race while wiring the envelope into the driver's spawn path.
