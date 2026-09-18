---
phase: quick-260917-nhc
plan: 01
subsystem: tests
status: complete
tags: [flake, determinism, setpgid, process-group, test-harness, diagnosis-correction]
requires: []
provides:
  - a bounded seqlock read at the two group observations in src/driver/run.rs's test module
  - the cross-test setpgid case record in src/driver/run.rs's module header
  - phase 21's current_group/proc-parse rows, closed append-only
  - the correction of 260908-uqq section 2's two false sentences
affects:
  - src/driver/run.rs
tech-stack:
  added: []
  patterns:
    - "the `*_within` bounded-poll idiom (Instant deadline + 25ms in-loop sleep, Duration::from_secs(30) inline at the call site) extended from tests/driver_reattach.rs to an OBSERVATION site rather than a spawn site"
    - "a seqlock-style read — observe, sample, observe again, discard if the two observations disagree — as the way a test cross-checks two readers of a value a sibling thread can move"
key-files:
  created: []
  modified:
    - src/driver/run.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
decisions:
  - "The recorded diagnosis was WRONG and correcting it is the headline: the /proc parse was the suspect, and it is exonerated — the group genuinely moves between the two reads"
  - "The mover is production code reached TRANSITIVELY by another lib test (src/driver/mod.rs:1834 -> drive -> execute_run -> establish_own_group, 7x per run), which is why the direct-call search that produced the false diagnosis returned a false negative"
  - "The fix is a synchronisation fix at the observation site, not a tolerance: the assert_eq! stays exact and byte-identical, and the .expect keeps its verbatim wording"
  - "The ONLY retry condition is 'the group moved inside the observation window'; an unreadable /proc still fires the .expect immediately, and expiry panics rather than returning a fabricated pair"
  - "Each discarded observation emits one eprintln!, so the branch is COUNTABLE rather than merely absent (inferred decision, see audit section)"
  - "execute_run's establish_own_group() ordering was considered and deliberately NOT changed — T-nhc-05, disposition accept, FLAGGED FOR LATER AUDIT"
metrics:
  duration: ~1 session
  completed: 2026-09-17
actuals:
  tokens: 34000
  tasks: 3
  commits: 3
plan_head_before: 6c2445ff8860ba7e952b26e5bd8dedd11adef03e
---

# Quick 260917-nhc: the current_group / proc-parse flake Summary

A bounded seqlock read at the two observation statements makes
`driver::run::tests::the_current_group_agrees_with_the_proc_parse` deterministic against a
cross-test `setpgid` race — proven in effect, not merely absent: across 200 contended runs
the retry branch fired 7 times, every straddle was absorbed on attempt 1, and all 200 runs
reported the test green against a measured pre-fix rate of 5 failures in 140 contended runs.

## The headline: the recorded diagnosis was wrong

Two prior investigations blamed `liveness::process_group`'s `/proc/<pid>/stat` field parse.
**The parse is correct. So is `getpgrp()`.** They disagree only because the process group
genuinely MOVES between the two reads.

The hazard the test's own comment frames as hypothetical — *"`setpgid` in a shared test
binary **would** move the harness's own process group"* — is ACTUAL, and it is caused by a
different test in the same binary. On Linux `setpgid(0, 0)` resolves against the
**thread-group leader**, not the calling thread, so a libtest worker thread that reaches it
moves the whole shared binary's group. The mover is `src/driver/mod.rs:1834`, the
"visible twin, driven end to end" arm of
`driver::tests::a_target_phase_that_renders_as_another_is_refused_at_the_seam`, which calls
`drive` once per `LOOK_ALIKE_PAIRS` entry and reaches `execute_run`'s
`establish_own_group()` **seven times per run**. No test calls it DIRECTLY — which is
precisely why the direct-call search that produced the false diagnosis came back negative.

Both false sentences in `260908-uqq/deferred-items.md` section 2 are now quoted verbatim
and corrected in phase 21's `deferred-items.md`. `260908-uqq` itself was not edited; it is
superseded from the newer file.

## What shipped

**Task 1 — `0e603eb` — the seqlock read** (`src/driver/run.rs`, test module only).
`group_observations_without_a_move_within(limit) -> (u32, u32)` in the `*_within` idiom:
`current_group()`, the `/proc` parse, `current_group()` again; agree and return, disagree
and discard the observation, sleep 25ms, retry until an `Instant::now() + limit` deadline.
`setpgid(0, 0)` is one-way and idempotent (it always sets the group to the caller's own
pid), which is what makes `leading == trailing` sufficient rather than merely suggestive —
the helper's doc records that a second, different `setpgid` target in this binary would
invalidate the reasoning.

**Task 2 — `9ea3204` — the module header case record** (`//!` prose only, 66 added lines, 0
removed). The correction as the finding, the thread-group-leader mechanism, the mover and
its 7 iterations, the strace and `/proc`-watch evidence, the 0/39-versus-1/20 rates and why
launch context decides reproducibility, both exonerations, the `/proc`-is-larger signature,
and the two documented non-goals — closing with a pointer to `deferred-items.md` rather
than duplicating the tables.

**Task 3 — `dbab7f9` — the append-only paper record** (277 insertions, **0 deletions**).
`# QUICK 260917-nhc` in phase 21's `deferred-items.md`, plus two dated pointers placed
adjacent to the `260917-lkg` lines that named this test (run-7 table row, and the
`What this does NOT close` sentence), each quoting the line it points from.

## Nothing was weakened

| Property | How it is held |
|---|---|
| the equality stays exact | `assert_eq!(from_syscall, from_proc, …)` and its message are byte-identical — the diff removes not one character, and the binding names were kept so it did not have to be retyped |
| an unreadable `/proc` is never retried | the `.expect("this process's own /proc/<pid>/stat is readable")` fires on the spot under verbatim wording. The ONLY retry condition is "the group moved" |
| expiry is loud | the loop falls out to a `panic!` naming the moving-group condition, the limit and the attempt count — never a fabricated pair |
| no tolerance | no `#[ignore]`, no `--test-threads`, no pre-assertion `sleep`, no approximate comparison, no `establish_own_group()` call in the test region (verified with comment-excluding greps) |
| production untouched | `src/driver/liveness.rs`, `src/driver/mod.rs`, `Cargo.toml`, `.github/` all at 0 changed lines; `current_group`, `establish_own_group` and `execute_run`'s call ordering byte-identical |

## The measurements

**Before** (untouched HEAD binary, non-job-leader Python parent):

| Condition | Runs | Failures |
|---|---|---|
| launched directly from an interactive shell | 39 | 0 |
| contended, pre-planning batch 1 | 20 | 1 |
| contended, pre-planning batch 2 | 60 | 2 |
| contended, re-confirmed at execution time on a snapshot of the HEAD binary | 60 | **2 (3.3%)** |

Three contended samples agreeing: **5 in 140 (3.6%)** against **0 in 39** direct.

**After — 200 contended runs on the rebuilt binary: `TOTAL 0/200`.** The retry branch fired
**7 times in 7 distinct runs (3.5%)**, always succeeding on the next window, and all 200
runs reported the test green. The seven straddle lines carry the predicted signature: the
leading value is CONSTANT at `3290125` — confirmed with `ps -o pid,pgid` to be the pgid of
the reproducer's own non-leader ancestor, the inherited group — while the trailing value
differs every time and equals that run's own binary pid (`3336491`, `3346180`, `3351057`,
`3518925`, `3608008`, `3610492`, `3656884`).

**Anti-vacuity (three controls).** The binary was re-resolved through
`cargo test --lib --no-run --message-format=json` after the rebuild; its mtime (17:06:59)
post-dates the source edit (17:06:35); and the retry-branch string is present in the
rebuilt binary and absent from the HEAD snapshot the "before" column was measured against
(`grep -ac` → 1 versus 0).

**Ten full-suite `rtk proxy cargo test --no-fail-fast` runs**, captured unfiltered to files
and parsed from the files with Python (never through a pipe — the shell hook rewrites
`grep` through rtk, which strips `test result:` lines):

| Run | passed / failed / ignored | Failing tests |
|---|---|---|
| 1–10 | 2120 / 1 / 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` |

This test reports `... ok` in all ten and appears in no run's `failures:` block.
`BrokenPipe` appears in none of the ten.

`./scripts/pre-tag-check.sh` → exit 1: gate 1 SKIPPED (no tag argument), gate 2 (MSRV) PASS,
gate 3 (`build --release`) PASS, gate 4 (`cargo test`) FAILED on exactly one test — the
version witness — gate 5 (`clippy -D warnings`) PASS, with the git-version mismatch reported
as the script's designed loud ADVISORY. `rtk proxy cargo clippy -- -D warnings` clean;
`cargo clippy --all-targets` produces zero diagnostics naming `src/driver/run.rs`.

## Deviations from Plan

**1. [Rule 2 — missing critical evidence] An `eprintln!` on the discard branch.** The plan's
action described the loop without one. Added so the branch is COUNTABLE: a fix that never
fires is indistinguishable from a fix that was not needed, and the coordinator's
verification discipline required proof in effect. It is what turned this from "the flake did
not recur" into "the branch fired 7 times and all 7 runs passed". Follows
`tests/envelope_tracer.rs`'s own convention for its bounded retry. Committed in `0e603eb`.

**2. [Scope, reported not absorbed] The plan's Task 1 diff gate**
`grep '^-[^-]' t1.diff | grep -c 'is readable'  # 0` **returns 1, and the constraint it
encodes still holds.** The `.expect` line MOVED from the test body into the helper, so git
pairs it as one removal and one addition. Its text is byte-identical
(`.expect("this process's own /proc/<pid>/stat is readable");`, confirmed with `cat -A`);
only its indentation changed, by the 4 spaces the enclosing `while` adds. The gate is a
heuristic for "the wording was not retyped or weakened", and the wording was not. The
`assert_eq!` gate — the one the hard constraints make byte-exact — returns 0 as specified.

**3. [Scope, reported not absorbed] The plan's Task 3 gate**
`grep -l 'the_current_group…' full-*.txt | wc -l  # 0` **cannot return 0 on a passing run.**
libtest prints `test driver::run::tests::the_current_group_agrees_with_the_proc_parse ... ok`
for every green run, so the name is in all ten captures by construction. The meaningful
check was run instead, per capture: the test's `... ok` / `... FAILED` line, and whether the
name appears inside a `failures:` block. Result: `ok=1, FAILED=0, in-failures-block=0` for
all ten.

**4. [Method] 200 post-fix contended runs rather than the planned 60**, on the coordinator's
instruction. At a ~3.5% base rate, 60 runs cannot distinguish a fix from luck.

## Deliberate non-goals

- **`execute_run`'s `establish_own_group()` ordering was NOT changed** — T-nhc-05,
  disposition `accept`, **FLAGGED FOR LATER AUDIT**. Stopping the mover means either a
  test-only seam in the production path that owns the kill switch's group identity, or
  deleting a real end-to-end control arm. Production `drive` is a dedicated process where
  the group move is correct and intended.
- **`establish_own_group()` is still not called from this test.** The decision stands; only
  its premise strengthened, from hypothetical to observed.
- **The BrokenPipe race in `tests/envelope_tracer.rs`'s `run_stub` remains OPEN.** It did
  not surface in any of the ten full-suite runs. Untouched, nothing claimed about it.
- **The git-version-constants witness is environmental** (local git 2.53.0 against constants
  re-derived at 2.55.0) and out of scope. Untouched.
- **`Cargo.toml`, the crate version, `.github/` and all tags are untouched.** v1.7.1 is a
  separate step.

## Coordinator inferred decisions (for audit)

The human was unavailable for the whole run; every fork below was decided from the planning
artifacts and is recorded here rather than escalated.

1. **The `eprintln!` on the discard branch** is an addition beyond the plan's literal change
   description. Taken because the coordinator's verification discipline demanded the fix be
   proven IN EFFECT, and because `260917-lkg` set the precedent in this very repo ("the
   branch is countable, not merely absent"). Load-bearing evidence, not decoration.
2. **Isolation mode: `none`.** Ran SEQUENTIALLY and unisolated on the primary checkout,
   branch `master`, after harness-worktree isolation was auto-degraded by the known
   `worktree.base-check` false negative (#1941/#48) on this repo's `master`. No worktree
   created, no branch switched. `gsd-tools query git.base-branch --is-protected master`
   returns `false` (this repo's default branch is `dev`), so the executor's protected-branch
   assertion passed rather than being overridden.
3. **T-nhc-05 (`execute_run`'s `establish_own_group()` ordering) dispositioned `accept`** and
   recorded as a deliberate non-goal in both the module header and `deferred-items.md`,
   flagged for later audit. This is the one call in this task a reviewer might reasonably
   make the other way; the premises for either choice are written down so the decision can
   be revisited without re-derivation.
4. **The fail-first baseline was re-measured rather than taken on trust** — 2 failures in 60
   runs against a snapshot of the untouched HEAD binary, executed before the first edit
   landed and agreeing with the two pre-planning samples. The snapshot was taken so the
   "before" leg could not be silently invalidated by a later rebuild.
5. **Two plan verification gates were reported as unsatisfiable-as-written rather than
   massaged into passing** (deviations 2 and 3 above). Neither hard constraint they encode
   was violated; both were re-verified by a check that measures the thing the gate was
   reaching for.
6. **Task 1 and Task 2 were committed separately despite touching the same file**, using
   `git apply --cached` on a hunk-filtered patch. The alternative — one combined commit —
   would have blurred the code change and the prose record into a single reviewable unit.

## Self-Check: PASSED

- `src/driver/run.rs` — present, modified, contains `group_observations_without_a_move_within`
- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` — present,
  `# QUICK 260917-nhc` present, `git diff --numstat` 277/**0**
- commits `0e603eb`, `9ea3204`, `dbab7f9` — all present in `git log`
- `git rev-list --count 6c2445f..HEAD` = **3**, matching `actuals.commits`
