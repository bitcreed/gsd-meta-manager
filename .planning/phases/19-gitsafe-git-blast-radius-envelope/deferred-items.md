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

### Ownership settled by the orchestrator (wave 4 post-merge gate)

19-04's executor proved the flake reproduces at the wave base `5e1170b` (waves 1-3 merged).
The orchestrator extended that check to `0a84023` — the commit **immediately before Phase 19
began** — via `git archive` into a clean tree:

```
run1 FAILED. 1 passed; 2 failed  (0.53s)
run2 FAILED. 1 passed; 2 failed  (0.52s)
run3 FAILED. 1 passed; 2 failed  (0.54s)
run4 FAILED. 1 passed; 2 failed  (0.52s)
```

4/4 red with **no Phase 19 code present at all**. At Phase 19 HEAD the same file is
intermittently green (1 of 3 full-suite runs clean, failure set never larger than these two
tests). The defect therefore predates Phase 19 entirely and belongs to the Phase 17/18
reattachment path — no Phase 19 plan introduced or worsened it.

**Signature of a failing run:** the file completes in ~0.5s instead of ~6.1s. The driver
process is reported live by the cmdline probe before it has written `run.json`, so the
reconcile scan that follows finds nothing on disk. The fix is to wait on the artifact, not
the process — `live_within` is the wrong synchronisation primitive for these two assertions.

**Consequence for this phase:** the post-merge gate cannot be relied on for a clean binary
pass/fail while this flake is live. Waves are accepted on the bounded-failure-set rule
instead: a wave passes if the only failures are these two tests and the total passing count
advances as expected.

## `tests/envelope_tracer.rs` — ETXTBSY when a just-copied stub is exec'd (found during 19-07)

**Symptom.** `a_relocated_copy_of_the_stub_refuses_instead_of_acting` fails intermittently
under a full parallel `cargo test` with:

```
the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }
```

Observed once in ~6 full-suite runs during 19-07; green in every run of the file alone.

**Cause (likely).** The fixture copies the generated stub and executes the copy. ETXTBSY is
the kernel refusing to `exec` a file that still has an open writer descriptor somewhere — the
classic write-then-exec race. Nothing about it is about the policy under test.

**Not caused by 19-07.** `git diff c107e04 -- tests/envelope_tracer.rs` is empty: this plan
did not touch the file, the stub generator, or `assert_provenance`. The file belongs to
19-01/19-03.

**Fix direction.** Drop or `sync_all` the copy's handle before exec, or retry a bounded number
of times on `ExecutableFileBusy`. Do **not** serialise the suite — that hides the race.

## `tests/driver_lock.rs` — one-off lock-acquisition timeout (found during 19-VERIFICATION)

**Symptom.** `the_lock_is_released_when_the_holding_process_dies` failed once under full-suite
parallel load with "the child driver never took the lock within 30s". Passed cleanly in
isolation twice.

**Orchestrator follow-up.** Three further full-suite runs under load did NOT reproduce it
(two fully green at 993 passing; the third showed only the two known `driver_reattach`
flakes). It is therefore rarer than the documented `driver_reattach` pair.

**Why it is recorded anyway.** Two independent observers reached the same hypothesis from
different directions. 19-07's executor flagged, unprompted, that moving envelope
establishment ahead of the lock "plausibly widens that race without changing its cause"; the
verifier then independently proposed that `establish_envelope()` — hook stub writes,
settings-file generation plus round-trip readback, `.git/info/exclude` write, cred config
generation, all inside one `spawn_blocking` **before** `lock::acquire` — is new I/O on the
run-startup critical path that did not exist before Phase 19.

**Assessment.** A test-timing observation, not a mechanism defect: no SAFE-0x success
criterion depends on lock-acquisition latency. But it is the one item in this phase where
Phase 19's own changes are the plausible cause, unlike the `driver_reattach` pair which is
proved pre-existing.

**Suggested owner.** Whoever fixes the `driver_reattach` race — the fix direction is the
same (wait on the artifact/state, not a fixed budget), and Phase 20 builds on this envelope
and will add further startup work.
