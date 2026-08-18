---
created: 2026-08-18
source: phase-19 wave-4 post-merge gate
resolves_phase:
severity: high
---

# `tests/driver_reattach.rs` — liveness probe races the `run.json` write

Two tests fail non-deterministically:

- `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` —
  `exactly one project has a run to observe`, `left: 0`.
- `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` —
  `the run record is on disk: Os { code: 2, kind: NotFound }`.

**Cause.** `live_within(pid, RUN_ID, …)` waits for the spawned driver's *process* (it matches
the cmdline), then the test immediately asserts on artifacts the driver has not necessarily
written yet. A failing run finishes the whole file in ~0.5s instead of ~6.1s — the tell that
the driver never got far enough to write `run.json` or its `run_started` journal record.

**Fix direction.** Synchronise on the artifact rather than the process: poll for `run.json`
(and, for the journal assertion, for a non-empty journal) with the same bounded-wait helper
style `live_within` already uses. Do not add a fixed `sleep`, and do not serialise the tests
with `--test-threads=1` — that hides the race rather than closing it.

**Proved pre-existing.** Reproduced 4/4 at `0a84023`, the commit immediately before Phase 19
began, from a clean `git archive` tree with no Phase 19 code present. Also reproduced by
plan 19-04's executor at the wave base `5e1170b`. Owner is the Phase 17/18 reattachment path,
not Phase 19.

**Why it matters.** While this is live, no phase can get a clean binary pass/fail from the
post-merge or project test gate; Phase 19 accepted its waves on a bounded-failure-set rule
instead. Phase 19's plan 19-08 runs a full project gate and will trip on this.

See `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` for the full
evidence trail.
