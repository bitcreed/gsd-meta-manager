---
phase: 20-deterministic-decision-router-run-bounds
reviewed: 2026-08-19T00:00:00Z
depth: standard
files_reviewed: 40
files_reviewed_list:
  - src/app.rs
  - src/browser.rs
  - src/change_tracker.rs
  - src/cli.rs
  - src/driver/bounds.rs
  - src/driver/dry_run.rs
  - src/driver/mod.rs
  - src/driver/rate_limit.rs
  - src/driver/reconcile.rs
  - src/driver/router.rs
  - src/driver/run.rs
  - src/driver/spawn.rs
  - src/envelope/mod.rs
  - src/error.rs
  - src/journal/mod.rs
  - src/journal/writer.rs
  - src/main.rs
  - src/state_reader/disk_status.rs
  - src/state_reader/mod.rs
  - src/state_reader/roadmap_md.rs
  - src/state_reader/state_md.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_start.rs
  - src/ui/screens/normal.rs
  - tests/async_blocking_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_inbox.rs
  - tests/driver_iteration_loop.rs
  - tests/driver_lock.rs
  - tests/driver_optin.rs
  - tests/driver_rate_limit.rs
  - tests/driver_router_conformance.rs
  - tests/driver_router_table.rs
  - tests/driver_tracer.rs
  - tests/envelope_wiring.rs
  - tests/journal_gitignore.rs
  - tests/journal_run_paths.rs
  - tests/state_reader_test.rs
  - tests/fixtures/transcripts/09-rate-limit-rejected.ndjson
  - tests/fixtures/transcripts/README.md
findings:
  critical: 1
  warning: 9
  info: 8
  total: 18
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-08-19
**Depth:** standard
**Files Reviewed:** 40 (24 source, 16 test/fixture)
**Status:** issues_found

## Summary

The phase's central claims mostly hold under adversarial reading. **Determinism (axis 3) is
clean**: `phase_disk_statuses` is indexed and never iterated anywhere in a decision path
(`src/driver/router.rs:686, 789, 844`; `src/driver/run.rs:1414`), and `DiskDelta::between`
compares `ProjectState` by value equality, which is order-independent for a `HashMap`.
**The safe alphabet (axis 2) is structurally closed**: `RouterAction` has three arms, every
emitted string is `format!("{verb} {phase}")`, and there is no default arm in `decide` — an
uncovered state genuinely parks. **The `DiskStatus::Ord` insertion (axis 4)** was traced to
every consumer; one behavioural consumer was missed by 20-03 (WR-04).

The serious finding is on axis 1. **The run-level wall-clock cap is not a bound on the run.**
It is evaluated only between iterations, and the per-iteration cap handed to the executor is a
hardcoded three-hour constant that is never reduced to the run's remaining budget. A run
launched with `--wall-clock-cap-secs 60` will happily run for three hours while `run.json`
records `wall_clock_cap_secs: 60` as "the cap in force". Every existing test passes because
none of them exercises a cap smaller than one iteration.

Secondary concerns cluster on **fail-open paths where the run continues or reports success
when it should stop or report a halt**: the terminal label round-trips through a disk read
that can silently discard the in-memory halt reason (WR-01); a `rejected` quota event can be
overwritten by a later `allowed` one (WR-02); the G15 deferred-verification gate matches by
raw string equality and silently never fires if STATE.md writes the phase cell in any other
form (WR-06).

On **test honesty (axis 7)** the guards are, with one exception, genuinely load-bearing: the
both-directions rule/row guard has a live control arm, the digest scanner has a fires/spares
pair, the signature scanner has a three-way control arm, the async-blocking allowlist grew by
exactly two entries each with a written reason and is defended by `no_allowlist_entry_is_stale`.
The exception is the conformance oracle (WR-07), which returns early and passes silently on
any machine without `gsd-tools` — its own comment claims a companion test prevents this, and
the named companion checks nothing about the oracle.

---

## Critical Issues

### CR-01: The run-level wall-clock cap does not bound the run; it is only checked between iterations

**File:** `src/driver/run.rs:1889-1897`, `src/driver/run.rs:1381-1392`, `src/driver/bounds.rs:102-117`

**Issue:**
`bounds::evaluate` is called at exactly one site — immediately before each spawn
(`run.rs:1889`). `elapsed` is `run_started_at.elapsed()`. Nothing evaluates the wall-clock
detector *during* an iteration, and nothing derives the executor's per-iteration cap from the
run's remaining budget:

```rust
// src/driver/run.rs:1385-1391
ExecutionOptions {
    ...
    wall_clock_cap: bounds::ITERATION_WALL_CLOCK_CAP,   // a fixed 3h constant
    ..Default::default()
}
```

`iteration_options` takes only `(&Path, &EnvelopeEnv)` — `run_bounds` is not in scope there
and is never consulted.

Concrete failure scenario:

1. `gsd-meta-manager drive alias --target-phase 20 --wall-clock-cap-secs 60`.
2. `bounds::resolve` accepts 60 (it is neither 0 nor above the 24h ceiling).
3. `RunRecord::bounds.wall_clock_cap_secs` is written as `60`, and
   `the_iteration_wall_clock_cap_is_strictly_inside_the_run_level_cap` still passes because it
   compares two *constants*, not the resolved value.
4. Iteration 1 spawns with a **3-hour** executor cap. The agent runs for three hours.
5. Only at the top of iteration 2 does `elapsed >= 60s` fire.

The run overruns its explicitly requested cap by a factor of 180, spends three hours of a
shared 5h/7d quota, and holds the project lock for the duration. Even at the default caps the
overrun is up to +3h on a stated 4h bound (a run of two 3h iterations halts at ~6h). The
`ITERATION_WALL_CLOCK_CAP < DEFAULT_RUN_WALL_CLOCK_CAP` assertion in `bounds.rs:390-400` proves
only that the *reason* is reportable, not that the cap is enforced — the doc at
`bounds.rs:110-112` ("the run-level cap always fires first for an accumulation across several")
is true about which reason is reported and false about when the run stops.

**Fix:**
Derive the per-iteration cap from the remaining run budget, and pass the resolved bounds into
`iteration_options`:

```rust
fn iteration_options(
    envelope_settings: &Path,
    envelope_env: &EnvelopeEnv,
    remaining: Duration,
) -> ExecutionOptions {
    ExecutionOptions {
        ...
        // Never longer than what is left of the run's own budget, and never
        // longer than the per-iteration ceiling. A cap smaller than one
        // iteration must bound that iteration, not the gap after it.
        wall_clock_cap: remaining.min(bounds::ITERATION_WALL_CLOCK_CAP),
        ..Default::default()
    }
}
```

with the call site computing
`run_bounds.wall_clock_cap.saturating_sub(run_started_at.elapsed())`.

Add a test that is not satisfiable by two constants — e.g. resolve a 2-second run cap, assert
the `ExecutionOptions` the loop would build carries `wall_clock_cap <= 2s`, and an end-to-end
test with `--wall-clock-cap-secs 1` against the transcript stand-in asserting the run ends
under the cap plus teardown. Also consider refusing a run-level cap below some floor rather
than silently accepting one the loop cannot honour.

---

## Warnings

### WR-01: A failed journal write or read silently converts a halt into a reported success

**File:** `src/driver/run.rs:2341`, `src/driver/run.rs:2357-2377`, `src/driver/run.rs:583-600`,
`src/driver/run.rs:2461-2482`

**Issue:**
`record_terminal` swallows every journal error with a `tracing::warn!` (lines 2472, 2480). The
terminal label is then derived by *re-reading the journal from disk*:

```rust
let label = match last_outcome {
    Some(outcome) => { /* spawn_blocking(terminal_label(&outcome, &journal_path)) */ }
    None => match terminal.park_reason() { ... }
};
```

and `terminal_label` (line 584) returns `outcome_label(outcome)` whenever `read_all` fails.

Failure scenario: a routed run halts on `bounds_step_cap` after a successful iteration. The
`Parked` record write fails (disk full, ENOSPC, a truncated line), or the journal is
unreadable at the end of the run. `terminal.park_reason()` is `Some("bounds_step_cap")` **in
memory and unused on this branch**. `run.json` records `outcome: "succeeded_with_changes"`. A
supervising process reading `run.json` concludes the run completed its work.

The authoritative value is already in hand; the code prefers a disk round-trip that can lose it.

**Fix:** prefer the in-memory terminal over the journal read on both branches:

```rust
let label = match terminal.park_reason() {
    // This run's OWN halt/park is authoritative and needs no round trip.
    Some(reason) => format!("{PARKED_LABEL_PREFIX}{reason}"),
    None => match last_outcome {
        Some(outcome) => /* terminal_label, which lets an EARLIER envelope park win */,
        None => GOAL_MET_LABEL.to_string(),
    },
};
```

Add a test that makes the `Parked` write fail (read-only journal file) and asserts `run.json`
still carries `parked:bounds_step_cap`.

### WR-02: A `rejected` quota event is discarded if any later `rate_limit_event` arrives in the same iteration

**File:** `src/driver/run.rs:2107-2111`

**Issue:**

```rust
if let ExecutionEvent::Message(message) = &event {
    if let StreamMessage::RateLimitEvent(payload) = message.as_ref() {
        latest_quota_event = Some(payload.clone());   // unconditional overwrite
    }
}
```

The retained value is the *last* event, not the *worst*. `classify` is then run once, at the
end of the iteration (line 2265). A stream that emits
`rate_limit_event{status:"rejected", five_hour}` followed by
`rate_limit_event{status:"allowed", seven_day}` — one event per window, or one per turn on a
steered run — leaves `latest_quota_event` holding the `allowed` payload. `classify` returns
`Allowed`, the second detector only fires if the run also *failed* with a rate-limit terminal
reason, and a run that succeeded continues to the next iteration. CTRL-07's "parks rather than
retrying" is bypassed and the next command spends more of the shared quota.

The retained-uninspected design is otherwise good; the flaw is only in the retention rule.

**Fix:** latch the rejection rather than overwriting:

```rust
// A rejection is a fact about the whole iteration and must not be overwritten
// by a later event about a different window: `latest` is the wrong rule when
// one of the values is terminal.
if !matches!(
    rate_limit::classify(latest_quota_event.as_ref(), now),
    rate_limit::QuotaVerdict::Rejected { .. }
) {
    latest_quota_event = Some(payload.clone());
}
```

or keep two slots (`latest` and `first_rejection`) and classify the rejection first. Add a
fixture with `rejected` followed by `allowed` and assert the run still parks.

### WR-03: `resetsAt` in the past is accepted and rendered to the user as when the quota resets

**File:** `src/driver/rate_limit.rs:265-277`, `src/driver/rate_limit.rs:90-104`,
`src/driver/rate_limit.rs:616-647`

**Issue:** the sanity bound is symmetric and deliberately so:

```rust
let skew = resets_at.timestamp().saturating_sub(now.timestamp()).saturating_abs();
(skew <= RESET_SANITY_WINDOW_SECS).then_some(resets_at)
```

`the_sanity_bound_accepts_one_second_inside_and_refuses_one_second_outside_on_both_sides`
pins the symmetry. A reset time up to 30 days *in the past* therefore passes validation and is
formatted into `park_detail` as `resets_at=<past instant>`.

This is demonstrable today, not hypothetical: the committed fixture
`tests/fixtures/transcripts/09-rate-limit-rejected.ndjson` carries
`"resetsAt":1785859200` (≈ 2026-08-02), so
`the_committed_rejection_fixture_parks_a_real_run` currently produces a park detail naming a
reset roughly seventeen days in the past. Nothing asserts on that portion of the detail, so
the test passes.

A quota window that "resets" before now is nonsense by construction. Axis 6's requirement is
that the bound reject nonsense rather than propagate it into a user-facing claim; a past
timestamp is exactly the corrupted/stale value the bound exists to catch, and it is the one
shape it lets through.

**Fix:** make the bound asymmetric — allow a small backwards allowance for clock skew and the
full window forwards:

```rust
/// Clock skew only. A reset time meaningfully in the PAST is not a reset time.
pub const RESET_MAX_SKEW_BEHIND_SECS: i64 = 5 * 60;

let ahead = resets_at.timestamp() - now.timestamp();
(ahead >= -RESET_MAX_SKEW_BEHIND_SECS && ahead <= RESET_SANITY_WINDOW_SECS)
    .then_some(resets_at)
```

Update the both-sides test to assert the *rejection* of a past value, and give fixture 09's
assertion a `now` on the correct side.

### WR-04: The redefinition of `Complete` silently moves the dashboard's "current phase" for every project

**File:** `src/state_reader/mod.rs:211-230`

**Issue:**

```rust
// Track first non-complete phase as the current phase status
if current_phase_inference.is_none()
    && inference.status != disk_status::DiskStatus::Complete
{
    current_phase_inference = Some(inference.clone());
}
```

`Complete` now means *implementation complete **and** verification passed*
(`src/state_reader/disk_status.rs:646-649`). Every phase that previously read `Complete` on
artifact presence alone now reads `Executed`, so this loop stops at the first phase awaiting
verification instead of walking past it.

Against this repository: phase 19 is `Executed` with `status: human_needed`, so
`current_phase_status` — which feeds the dashboard's compact D-R-P-E-V cell
(`src/ui/screens/normal.rs:723`) and the driver screen (`src/ui/screens/driver.rs:1096`) — now
points at phase 19 rather than phase 20. The dashboard shows a *different phase's* pipeline
than it did before this change, for every project with an unverified completed phase.

20-03's summary states "the D-R-P-E-V stage thresholds did NOT move; only `prev_status` did"
and lists only `src/ui/screens/normal.rs::prev_status` as the affected consumer. This consumer
is in a file 20-03 declares as modified but is not mentioned in the deviations, the key
decisions, or the deferred list, and there is no test covering it. Whether the new reading is
*better* is arguable; that it is an unreviewed user-visible change is not.

**Fix:** decide explicitly which semantics `current_phase_status` wants and say so in a comment
naming the change, then pin it:

```rust
// `Complete` is a conjunction since Phase 20 (implementation AND verification),
// so "first non-complete" now stops at a phase awaiting verification. That is
// deliberate: the dashboard's current phase is the phase that still needs
// something, and a phase whose verification is human_needed does.
```

Add a test over a two-phase fixture where phase 1 is `Executed` and phase 2 is `Planned`,
asserting which one `current_phase_status` names.

### WR-05: `leading_frontmatter_value` ignores indentation, so a nested `status:` is read as the document's status

**File:** `src/state_reader/disk_status.rs:260-278`

**Issue:** the byte-zero anchor correctly prevents the fenced-code-block false match
(T-20-12), but the key comparison trims leading whitespace off the key:

```rust
if let Some((found, value)) = line.split_once(':') {
    if found.trim() == key {          // "  status" trims to "status"
        return Some(value.trim().to_string());
    }
}
```

A leading frontmatter block of the shape

```yaml
---
phase: 20
verification:
  status: passed
status: human_needed
---
```

returns `passed`. The **first** match wins, and a nested key at any indentation is
indistinguishable from a top-level one. (List items are safe by accident — `- status:` trims
to `- status` — but a plain indented mapping key is not.)

This function is the single input to `VerificationStatus`, which is the goal-met predicate
(`router::is_goal_met`) and the entire DRIVE-05 verification gate set. A nested
`status: passed` produces a false `Decision::GoalMet`; a nested `status: human_needed` produces
a spurious park. The same helper also drives `plan_frontmatter_superseded`
(`disk_status.rs:288-291`), where a false positive silently removes a plan from the counts and
changes the derived `DiskStatus`.

GSD's current VERIFICATION.md template does not produce this shape, which is why this is a
warning rather than a blocker — but the whole point of the byte-zero anchor was that a
document *describing* a status must not be read as *having* one, and this is the same class of
defect one level in.

**Fix:** require the key at column zero within the block:

```rust
// Column zero, because a nested mapping key is a different key. GSD's own
// DEFECT.FRONTMATTER-SCALAR-BROAD-GREP is the same mistake one level up.
if line.starts_with(char::is_whitespace) {
    continue;
}
if let Some((found, value)) = line.split_once(':') {
    if found == key { ... }
}
```

Add two tests: a nested `status` under a parent key must not be returned, and a top-level
`status` after a nested one must be.

### WR-06: The G15 deferred-verification gate matches by raw string equality and fails open on any other cell format

**File:** `src/state_reader/state_md.rs:149-171`, `src/driver/router.rs:607-617`

**Issue:** `deferred_verification_phases` pushes the first table cell verbatim after trimming
`*` (line 164, 170). The router then compares with exact equality:

```rust
if state.deferred_verification_phases.iter().any(|phase| phase == target_phase)
```

`target_phase` is whatever arrived on argv (`"20"`). If STATE.md's table writes the cell as
`Phase 19`, `19-gitsafe-git-blast-radius-envelope`, or `#19`, the gate **never fires** and the
run routes forward past a phase that explicitly owes a verification. This repository's own
STATE.md happens to use bare numbers, which is why every test passes.

Nothing else in the reader is this literal: `parse_depends_on`
(`src/state_reader/roadmap_md.rs:79-93`) deliberately accepts `Phase <id>` and the four
identifier forms `PHASE_ID` recognises. The G15 path accepts none of them.

A gate that silently never fires is worse than an absent one — the journal grep that
CONTEXT.md OQ2 asks for ("how often does always-park stop a run, and where?") answers zero for
a reason unrelated to the posture.

**Fix:** normalise the cell through the same identifier extraction `parse_depends_on` uses, or
at minimum strip a leading `Phase` keyword and a trailing `-<slug>`:

```rust
// The same identifier forms the roadmap parser accepts, for the same reason:
// a gate keyed on one spelling is a gate that fails open on every other.
let phase = extract_phase_id(first).unwrap_or_else(|| first.to_string());
```

Add tests over `| Phase 19 |`, `| 19-gitsafe |` and `| **19** |` asserting the same gate fires.

### WR-07: The conformance oracle passes silently on any machine without `gsd-tools`

**File:** `tests/driver_router_conformance.rs:341-348`, `tests/driver_router_conformance.rs:476-504`

**Issue:**

```rust
let Some(oracle) = Oracle::resolve() else {
    // The reason was printed by `resolve`. Returning here is a skip, and the
    // companion test below is what stops a skip becoming a silent pass in an
    // environment where the oracle IS present.
    return;
};
```

The named companion, `every_fixture_reaches_the_state_it_is_named_for`, checks only that each
fixture tree produces the disk status it is named for — it never resolves or consults the
oracle. So on any machine or CI runner without `~/.claude/gsd-core/bin/gsd-tools.cjs` and
without `gsd-tools` on `PATH`, *both* tests pass while nothing checked that the rule table is
still a faithful transcription of GSD's router. `eprintln!` output from a passing test is
captured by libtest and invisible without `--nocapture`.

The file's own header states the standard it fails: *"A test that passes because it checked
nothing is worse than no test."* The `compared >= 1` floor inside the test is correct but is
never reached on the skip path.

(It does resolve and run 5 comparisons on this machine — verified.)

**Fix:** make the skip explicit and detectable. Either:
- gate on an opt-out env var and `panic!` when the oracle is absent and the var is unset, so
  the default is fail-loud; or
- keep the skip but add a genuine companion that asserts the oracle *was* resolvable whenever
  a marker file / env var says the environment is expected to have it, and record the skip in a
  way CI can see (e.g. write a sentinel the build checks).

At minimum, change the misleading comment: no companion test currently guards this.

### WR-08: A routed run that meets its goal reports two different terminal labels depending on whether any command ran

**File:** `src/driver/run.rs:1861-1864`, `src/driver/run.rs:2357-2390`,
`src/driver/run.rs:1344-1360`

**Issue:** `Decision::GoalMet` sets `terminal = Terminal::Completed` and breaks. At the
terminal write:

- if **no** iteration ever spawned, `last_outcome` is `None` and the label is
  `GOAL_MET_LABEL` (`"goal_met"`);
- if **any** iteration spawned, `last_outcome` is `Some(...)` and the label is
  `outcome_label` of the *previous* iteration — `"succeeded_with_changes"` or
  `"succeeded_no_changes"`.

So the same terminal condition — the declared target's verification passed — writes
`goal_met` on disk when the target was already met at launch and `succeeded_with_changes` when
the run actually achieved it. Criterion 5 asks that every run end classified as goal-met,
parked or halted; the more interesting of the two goal-met cases is the one that does not say
so. A reader of `run.json` must infer goal-met from the absence of a `parked:` prefix.

**Fix:** carry the distinction on the terminal rather than on whether an outcome exists:

```rust
Terminal::GoalMet,          // the declared target was reached
Terminal::Completed,        // the run performed everything asked of it
```

and label `Terminal::GoalMet` as `GOAL_MET_LABEL` regardless of `last_outcome`. Both arms stay
exhaustive and wildcard-free. Add an end-to-end test where iteration 1 writes a passing
verification and iteration 2's router reports goal-met, asserting `run.json` carries
`goal_met`.

### WR-09: `--dry-run` skips the `--target-phase` validation and the bounds refusal it is previewing

**File:** `src/driver/mod.rs:349`, `src/driver/mod.rs:456-473`

**Issue:** `drive` returns from the `if args.dry_run { ... }` block at line 349, before both

- `journal::is_plain_path_component(target_phase)` (line 460), and
- `bounds::resolve(args.max_steps, args.wall_clock_cap_secs)` (line 473).

Two consequences:

1. `--dry-run --target-phase '../../../escaped'` renders a preview. The value reaches
   `Decision::Park { detail: target_phase }` (`router.rs:834-839`) and is printed verbatim, and
   `RouterAction::command_for`'s doc (`router.rs:362-366`) asserts *"`phase` arrived on argv and
   was validated at the seam"* — a statement that is false on this path. Nothing composes a path
   from it today, so the impact is a preview that prints an unvalidated token as a pasteable
   command line; the doc claim is the part that is wrong.
2. `--dry-run --max-steps 0 --target-phase 20` prints a clean preview of an invocation that
   would be *refused* if run for real. A preview whose whole purpose is "what would happen"
   answers the wrong question.

**Fix:** move both refusals above the dry-run branch, beside `command_source_refusal`, which is
already positioned there for exactly this reason ("This one is about **what was asked for at
all**"). Both are pure and neither creates anything on disk.

---

## Info

### IN-01: Two doc comments in `run.rs` state a reader limitation that 20-03 removed

**File:** `src/driver/run.rs:1403-1409`, `src/driver/run.rs:450-456`

`drpev_stages`' doc says *"this repository's `DiskInference` records only whether a
`*-VERIFICATION.md` exists, never its frontmatter `status`, which is the whole DRIVE-05 gate
set (research Pitfall 2)"*, and `Terminal::Completed`'s doc says goal-met *"needs the
verification frontmatter status that this repository's one reader does not record yet"*. Both
are false since 20-03 added `DiskInference::verification_status`. `GOAL_MET_LABEL`'s doc
(line 1353) was correctly updated; these two were not.

**Fix:** update both. `drpev_stages` still genuinely reports presence for the `V` stage, so say
that it *chooses* presence (and why) rather than that status is unavailable.

### IN-02: `prev_status` keeps a `_ =>` wildcard, so the next `DiskStatus` insertion will not be a compile error

**File:** `src/ui/screens/normal.rs:52-62`

The doc added by 20-03 says *"this function has to move whenever a variant is inserted into
`DiskStatus`"* — and the wildcard is precisely what stops the compiler from saying so. The
`Executed` insertion was caught by a human, not by the build. CONVENTIONS.md:51-53 names
exhaustive-match-as-compile-gate as the house pattern for correctness-relevant matches.

**Fix:** spell out `NoDirectory` and `Empty` and delete the wildcard.

### IN-03: The digest scanner audits only `src/driver/bounds.rs`

**File:** `tests/driver_iteration_loop.rs:454-541`

`the_no_progress_path_uses_the_shipped_delta_and_computes_no_digest` reads one file. A digest
over `ProjectState` planted in `src/driver/run.rs` or `src/driver/router.rs` — the two other
files on the no-progress path — is invisible to it. The `MIN`-floor and control-arm shapes are
correct; the scope is narrower than the property claimed.

**Fix:** walk the three files, or `src/driver/` entirely, with the same floor.

### IN-04: A documentation guard is pinned on exact line wrapping

**File:** `src/envelope/mod.rs` (`the_residual_exposure_is_disclosed_as_open_and_its_mitigation_named_partial`)

`OWN_SOURCE.contains("partial in\n//!    two directions")` embeds the current line break and
indentation. Any rewrap of that paragraph — including an innocuous one — turns the disclosure
guard red for a reason unrelated to the disclosure being softened.

**Fix:** normalise whitespace before the `contains`, or assert on a short phrase that cannot
straddle a wrap ("partial in two directions" after collapsing whitespace).

### IN-05: `spawn_failed` is a terminal state outside the three DRIVE-06 classes

**File:** `src/driver/run.rs:1957-1970`

A spawn failure writes `run.json` with `outcome: "spawn_failed"` and returns
`Err(DriveError::Spawn)`, bypassing `Terminal` and `record_terminal` entirely. It is a stated
reason and pre-dates this phase (T-17-06), but criterion 5's enumeration is
goal-met / parked / halted and this is none of them.

**Fix:** either document `spawn_failed` in `Terminal`'s doc as the fourth, pre-`Terminal`
terminal reason, or route it through a `Terminal` arm so the type really is exhaustive over
"how a run ended".

### IN-06: Dead `unwrap_or` on an infallible `split().next()`

**File:** `tests/driver_router_conformance.rs:491-496`

```rust
let expected = fixture.name.split("_gaps").next()
    .and_then(|n| n.split("_human").next())
    .unwrap_or(fixture.name);
```

`str::split(..).next()` always yields `Some`, so both the `and_then` and the `unwrap_or` are
unreachable-fallback code that reads as if the parse could fail.

**Fix:** `let expected = fixture.name.split("_gaps").next().unwrap().split("_human").next().unwrap();`
or store the expected status token as a field on `Fixture` rather than deriving it from the name.

### IN-07: Two self-referential tests return early rather than failing when their own repo data is missing

**File:** `src/state_reader/roadmap_md.rs:461-464`, `tests/state_reader_test.rs:637-640`

`test_depends_on_is_read_from_this_repositorys_own_roadmap` and
`test_this_repositorys_own_planning_dir_reads_without_panicking` both `return` silently when
`.planning/` or `ROADMAP.md` cannot be read. Both files are committed and always present, so
the escape hatch buys nothing and matches the vacuous-pass shape the rest of the phase's guards
are careful to avoid. (`disk_status.rs:1284-1291` has a *justified* early return — the phase-19
directory can legitimately be archived away.)

**Fix:** `expect("this repository ships its own .planning/ROADMAP.md")`.

### IN-08: `parse_depends_on` recompiles two regexes on every invocation

**File:** `src/state_reader/roadmap_md.rs:80-93`

`Regex::new(r"\([^)]*\)")` and the `Phase\s+(...)` regex are built inside the function, which
is called once per roadmap entry inside `parse_roadmap_phases`'s line loop. The rest of the
module hoists its regexes to the top of `parse_roadmap_phases`.

**Fix:** hoist both alongside `checklist_re`/`heading_re`/`depends_re`, or use a `OnceLock`.
(Noted as a consistency defect, not a performance finding.)

---

## What was checked and found sound

Recorded so a later reviewer does not re-derive it:

- **Determinism (criterion 1).** No decision path iterates `phase_disk_statuses`;
  `unsatisfied_dependency`, `colliding_partial_phase` and `reaches` all walk
  `ProjectState::phases` (a `Vec` in roadmap order) and index the map.
  `DiskDelta::between` compares `ProjectState` by `PartialEq`, which is order-independent for
  a `HashMap`. `read_verification_status` / `read_uat_status` sort names before reading
  (`disk_status.rs:304, 319`).
- **Safe alphabet (criterion 4).** Three-arm `RouterAction`, one `format!` construction site,
  no default arm in `decide`, and `NEVER_AUTO_SELECT`/`SAFE_COMMAND_ALPHABET` proven disjoint
  as declarations rather than only as observations.
- **Step-cap boundary.** `completed_steps >= max_steps` with N-1/N/N+1 asserted in one body;
  `saturating_add` on both counters; `u32` throughout with no cast. No off-by-one.
- **Evaluation order.** Single `evaluate` call site, `return`-on-first-hit,
  `BoundVerdict` structurally unable to hold two reasons, and the order walked down arm by arm
  in `the_documented_evaluation_order_decides_when_every_condition_holds_at_once`.
- **Rate limit: no retry.** No `sleep`, no backoff, nothing scheduled against `resets_at`
  anywhere on the quota path; `derive_run_outcome_from_envelopes`' signature is unchanged and
  the scanner guarding it has a genuine three-way control arm.
- **Async-blocking allowlist.** Grew by exactly two entries, both with written reasons, both
  join-failure fallbacks; the marker rename from `build_report(` to `preview_text(` was forced
  by `no_allowlist_entry_is_stale` rather than volunteered; `capture_snapshot(` is asserted
  *not* to be a marker, which is the correct call.
- **`DiskStatus::Ord` consumers.** `src/ui/screens/normal.rs:288` is the only ordering
  comparison in the tree; `detail.rs:470-478` and `normal.rs:1475-1483` were forced by
  exhaustive matches. `state_reader/mod.rs:216` is the missed one (WR-04).
- Build is clean; `driver_router_conformance` runs 5 real comparisons on this machine.

---

_Reviewed: 2026-08-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
