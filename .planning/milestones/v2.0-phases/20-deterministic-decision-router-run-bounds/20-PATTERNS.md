# Phase 20: Deterministic Decision Router & Run Bounds - Pattern Map

**Mapped:** 2026-08-19
**Files analyzed:** 10 (2 created, 6 modified, 2+ test targets)
**Analogs found:** 10 / 10

Every file this phase touches has a close in-tree analog. This phase adds **zero**
dependencies and **zero** new subsystems: the pattern work is copying four
established shapes (pure classifier + reason constants, exhaustive no-wildcard
match, declared-allowlist guard, on-disk journal assertion) into new positions.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/driver/router.rs` (CREATE) | pure classifier / policy | transform (state → command) | `src/envelope/policy.rs` | exact |
| `src/driver/bounds.rs` (CREATE, or fold into `router.rs`) | pure classifier / detector | transform (snapshot pair → verdict) | `src/envelope/policy.rs` + `src/executor/outcome.rs::DiskDelta` | exact |
| `src/state_reader/disk_status.rs` (MODIFY) | model / reader | file-I/O (dir scan + frontmatter parse) | `disk_status.rs::plan_frontmatter_superseded` (same file) | exact (self-analog) |
| `src/driver/run.rs` (MODIFY — iteration loop) | service / orchestrator | event-driven (async select loop) | `src/driver/run.rs:1518` drain loop (same file) | exact (self-analog) |
| `src/driver/run.rs` (MODIFY — rate-limit observation) | service | event-driven | the `event = handle.events.recv()` arm at `:1531-1593` | exact |
| `src/driver/mod.rs` (MODIFY — `DriveArgs` cap overrides + target phase) | config / DTO | request-response (argv → value) | `DriveArgs` existing fields (`:98-135`) | exact |
| `src/cli.rs` (MODIFY — new flags) | config / CLI | request-response | `Commands::Drive` arm (`:47-75`) | exact |
| `src/journal/mod.rs` (MODIFY — emit `Observed`/`Decided`) | model / durable record | file-I/O (append-only) | `JournalEvent::Parked` (`:815-833`) + `ExecStarted` | exact |
| `src/driver/dry_run.rs` (MODIFY — pinned text) | presenter | transform | `SECTION_COMMANDS`/`SECTION_REFSPECS` (same file `:69-95`) | exact (self-analog) |
| `tests/driver_router_*.rs` (CREATE — table + oracle) | test (integration) | batch | `tests/envelope_wiring.rs`, `tests/spawn_seam_guard.rs` | exact |
| `tests/async_blocking_guard.rs` (MODIFY — allowlist entries) | test (lint) | batch | `ASYNC_BLOCKING_ALLOWLIST` (`:188-211`) | exact (self-analog) |

## Pattern Assignments

### `src/driver/router.rs` (CREATE — pure classifier, transform)

**Analog:** `src/envelope/policy.rs`

**Module-doc pattern** (`policy.rs:1-13`) — state the no-I/O property first, because
it is what makes exhaustive unit tests possible:

```rust
//! Pure decision functions over refspecs and the reserved branch namespace.
//!
//! **No I/O, no processes, no network.** Everything here is a total function of
//! its arguments, which is what makes the envelope's decisions exhaustively
//! unit-testable without a repository, a child process or a credential — …
//! a decision that needs the world to answer cannot be tested against
//! the world's hostile cases.
```

The router's version of this sentence is CONTEXT.md's lock: the router takes
`ProjectState` by value and does no I/O; the caller reads state and hands it in
(D-11).

**Reason-constant + enum + `as_str` pattern** (`policy.rs:33-53`, `154-189`) — copy
this shape verbatim for the *sibling* bounds/router reason enum. Do **not** extend
`ParkReason`; its seven arms stay untouched:

```rust
/// D-24's park reason for a push whose destination is outside the namespace.
pub const REASON_PUSH_OUTSIDE_NAMESPACE: &str = "push_outside_namespace";
// … one `pub const` per arm, each with its own doc comment naming the decision id

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkReason {
    /// A push whose destination is outside the reserved namespace (D-05).
    PushOutsideNamespace,
    // …
}

impl ParkReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// Every arm returns one of the `REASON_*` constants above rather than a
    /// fresh literal, so the constant and the enum can never disagree and
    /// `grep REASON_FORCE_PUSH_BLOCKED` finds every producer.
    pub fn as_str(&self) -> &'static str {
        match self {
            ParkReason::PushOutsideNamespace => REASON_PUSH_OUTSIDE_NAMESPACE,
            // … one arm per variant, no wildcard
        }
    }
}
```

Reason names this phase's docs already fix: `router_no_rule`,
`gate_verification_gaps_found`. The rest follow the same snake_case shape.

**Closed-taxonomy doc pattern** (`policy.rs:136-153`) — the paragraph that says the
set is closed on purpose and where an un-named case goes. The router's equivalent
is CONTEXT.md's rule that an uncovered state parks under `router_no_rule` and never
falls back to a model or to `/gsd-progress`:

```rust
/// The set is closed on purpose. When a refusal does not have its own reason —
/// `git stash`, `update-ref`, `filter-branch` — it is reported under
/// [`ParkReason::ForcePushBlocked`] … That is stated here rather than left for a
/// reader to infer from a surprising string, because inventing an eighth reason
/// at a call site is how a taxonomy stops being one.
```

**Verdict-enum pattern** (`policy.rs:101-117`, `191-207`) — the decision type. Note
`reason` is `&'static str` / a typed enum, never a caller-authored string, and the
`detail` field carries the one token, never the whole input:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitVerdict {
    Allow,
    Refuse {
        /// The member of D-24's taxonomy this refusal parks under.
        reason: ParkReason,
        /// One line naming the exact token that caused the refusal. Never the
        /// whole argv: a detail that quotes the command back is a detail that
        /// can carry a secret into the journal (SAFE-04).
        detail: String,
    },
}
```

The router's `Decision` is this shape with four arms — `Run(command)`, `Park(reason)`,
`GoalMet`, `NoRule(state)` — and the same SAFE-04 constraint on any detail field
(only enum values and command names reach the journal; never agent free text).

**Context-passed-in pattern** (`policy.rs:209-220`) — the justification for taking
inputs rather than resolving them:

```rust
/// The two facts [`classify_git`] cannot compute from argv alone.
///
/// Passed in rather than resolved inside, and that is the whole reason the
/// classifier is a pure function: the namespace comes from configuration and
/// the destinations come from a repository, so a classifier that fetched them
/// itself could only be tested against a real repository.
```

**Unit-test pattern** (`policy.rs:1388+`) — in-source `#[cfg(test)] mod tests`, a
small fixture constructor, sentence-length test names, and an assertion message
that states the failure this test exists to prevent:

```rust
assert_eq!(
    policy.pr_cap_per_24h, DEFAULT_PR_CAP_PER_24H,
    "a derived `Default` would make this 0, which means no PR may ever be opened \
     (config.rs:109-117's lesson, applied to a cap)"
);
```

---

### `src/driver/bounds.rs` (CREATE — pure detector, transform)

**Analog:** `src/executor/outcome.rs:88-130` (`DiskDelta`) — **use it, do not
re-derive it.** RESEARCH "Don't Hand-Roll" and CONTEXT's correction both say the
no-progress detector compares deltas, never digests.

**The comparison to call** (`outcome.rs:105-129`, verbatim):

```rust
impl DiskDelta {
    /// Compute the delta between a before and an after snapshot.
    pub fn between(before: &RunSnapshot, after: &RunSnapshot) -> Self {
        let head_moved = match (&before.head_sha, &after.head_sha) {
            (Some(before_sha), Some(after_sha)) => before_sha != after_sha,
            // Unknown on either side: inconclusive, so the signal stays quiet.
            _ => false,
        };
        let dirty_flipped = match (before.dirty, after.dirty) {
            (Some(before_dirty), Some(after_dirty)) => before_dirty != after_dirty,
            _ => false,
        };

        Self {
            artifacts_changed: before.project_state != after.project_state,
            head_moved,
            dirty_flipped,
        }
    }

    /// Whether **any** of the three signals fired.
    pub fn made_changes(&self) -> bool {
        self.artifacts_changed || self.head_moved || self.dirty_flipped
    }
}
```

The unknown-vs-unchanged doc that must be preserved as the detector's own
justification (`outcome.rs:83-87`):

```rust
/// **An unknown git half is inconclusive, not false.** When a project is not a
/// git repository, `head_sha` and `dirty` are `None` on both snapshots and the
/// two git signals stay `false` — they contribute nothing rather than asserting
/// "nothing moved".
```

Consequence for the bounds detector: an unknown fingerprint is **not** evidence of
no progress. Do not count an inconclusive comparison toward the 2-iteration
threshold without saying so explicitly in the code.

**Named-invariant test pattern for the cap collision** (Pitfall 3). The precedent is
`the_startup_stop_budget_fits_inside_the_driver_teardown_grace`, whose const-doc
shape at `src/driver/run.rs:156-177` is the model — the doc states the arithmetic
and then names the test that asserts it rather than claiming it:

```rust
/// The upper bound is not taste. The whole startup teardown has to fit inside
/// [`crate::driver::kill::DRIVER_TEARDOWN_GRACE`] … Five plus
/// [`STARTUP_REAP_BOUND`] plus the journal's two is nine, which fits with three
/// to spare, and
/// `the_startup_stop_budget_fits_inside_the_driver_teardown_grace` asserts that
/// rather than this comment claiming it.
const STARTUP_AGENT_GRACE: Duration = Duration::from_secs(5);
```

Copy this for `RUN_WALL_CLOCK_CAP` (4h) vs the per-iteration executor
`wall_clock_cap`, with a test asserting the **strict** inequality.

---

### `src/state_reader/disk_status.rs` (MODIFY — reader, file-I/O)

**Analog:** the same file. Two existing shapes cover the whole change.

**Frontmatter parse pattern** (`disk_status.rs:46-73`) — the byte-0-anchored line
scan, no YAML dependency, fail-safe on absence. Mirror it for the VERIFICATION
`status` field (research: `verification.cjs`'s `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP`
records that a broad grep false-matched `status:` inside fenced code blocks, so the
anchor is load-bearing):

```rust
fn plan_frontmatter_superseded(content: &str) -> bool {
    let mut lines = content.lines();
    // Frontmatter must open on the very first line with a bare `---`.
    if lines.next().map(str::trim) != Some("---") {
        return false;
    }
    for line in lines {
        if line.trim() == "---" {
            // End of frontmatter block without a superseded marker.
            return false;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "status" && value.trim().eq_ignore_ascii_case("superseded") {
                return true;
            }
        }
    }
    false
}
```

**Filename-match arm to extend** (`:254-257`) — today it records presence only:

```rust
// Match VERIFICATION.md or *-VERIFICATION.md
if name == "VERIFICATION.md" || name.ends_with("-VERIFICATION.md") {
    has_verification = true;
}
```

Collect names (as the summary pass at `:236-241` already does with `summary_names`),
sort, take the first (mirroring `verification.cjs:302-303`), then parse.

**Status-derivation arm to extend** (`:275-288`) — the `Executed` variant slots
between `Complete` and `Partial`; `Complete` must gain the `verification == passed`
conjunct so it means what GSD's `complete` means:

```rust
// Determine status following GSD's priority order
let status = if summary_count >= plan_count && plan_count > 0 {
    DiskStatus::Complete           // ← today this IS GSD's `executed`
} else if summary_count > 0 {
    DiskStatus::Partial
} else if plan_count > 0 {
    DiskStatus::Planned
} else if has_research {
    DiskStatus::Researched
} else if has_context {
    DiskStatus::Discussed
} else {
    DiskStatus::Empty
};
```

**Enum-extension pattern** (`:4-14`) — `DiskStatus` derives
`Copy, PartialEq, Eq, PartialOrd, Ord, Default` and **`Ord` is derived from
declaration order**, so a new `Executed` variant must be inserted between `Partial`
and `Complete`, not appended, or every ordering comparison silently changes meaning.
Adding the variant is the stronger move precisely because every `match` on it
(e.g. `src/ui/screens/detail.rs:465-474`) is exhaustive with no wildcard and becomes
a compile error.

**Struct-field pattern** (`:16-44`) — `DiskInference` is a flat `pub` struct deriving
`Default`; a new typed `verification_status` field follows the existing
`has_*: bool` neighbours and must be threaded through the literal at `:290-315`.

---

### `src/driver/run.rs` (MODIFY — iteration loop, event-driven)

**Analog:** the existing drain loop in the same file. **Hoist boundary is `:1401`
(the `select!` racing `executor.start`) to `:1673` (`handle.wait_outcome()`).
Nothing above or below moves** (Pitfall 4: `RunLock` has no `Drop` impl, `run.json`
is written exactly twice, `establish_envelope` writes four files and probes).

**`biased` select with terminate first** (`:1401-1413`) — every new `select!` the
outer loop introduces copies this arm order and this justification:

```rust
let started = tokio::select! {
    biased;

    _ = term.recv() => {
        shutdown_during_startup(&mut pgid_rx, &mut run.journal).await;
        // Returning drops `run` and with it the `RunLock` — the descriptor
        // close IS the release (D-20.2). The terminal record was written by
        // the call above, so nothing below runs and no second one follows.
        return Ok(());
    }

    result = executor.start(&project, args.command.clone(), options) => result,
};
```

with the rationale from `:1387-1394` (*"There the cost of losing the race is a
delayed stop; here it is a **swallowed** one"* — CR-01) and `:1478-1490` (*"a stop
that loses to a busy event queue is a stop the user experiences as ignored
(D-06.1)"*). Any new await between iterations — snapshot capture, router call,
journal write — is a new window a SIGTERM can arrive in.

**Event-arm pattern for rate-limit observation** (`:1531-1541`) — extend this arm;
do not add a second consumer of the stream:

```rust
event = handle.events.recv() => {
    match event {
        Some(event) => {
            let turn_boundary = matches!(event, ExecutionEvent::TurnCompleted(_));

            if let Err(err) = run.journal.record_exec(&event) {
                // The error KIND only. Never a message body, which
                // could carry agent output (T-17-05).
                tracing::warn!(kind = ?err.kind(), "journal write failed");
            }
            // … NEW: retain the latest rate_limit_info here …
        }
        None => break,   // ← the iteration boundary is the stream CLOSING
    }
}
```

Pitfall 8: `None => break` at `:1591` is the iteration boundary. A `result` is a
*turn* boundary (`:1642-1644`, *"Deliberately no `break` here"*).

**Blocking hand-off pattern** (`:1675-1694`) — the shape every per-iteration
`RunSnapshot::capture` must adopt, including the inline join-failure fallback:

```rust
let journal_path = run.journal.paths().journal.clone();
let task_outcome = outcome.clone();
let task_path = journal_path.clone();
let label = match tokio::task::spawn_blocking(move || {
    terminal_label(&task_outcome, &task_path)
})
.await
{
    Ok(label) => label,
    Err(err) => {
        tracing::warn!(
            panicked = err.is_panic(),
            "the terminal-label task did not run to completion",
        );
        terminal_label(&outcome, &journal_path)
    }
};
```

**Terminal-record pattern** (`:417-463`) — the carrier CONTEXT.md locks for reuse.
A bounds halt or a router park writes a `Parked`-shaped journal record with a
sibling reason and this function picks it up **with no change**:

```rust
pub(crate) const PARKED_LABEL_PREFIX: &str = "parked:";

pub(crate) fn terminal_label(outcome: &RunOutcome, journal: &Path) -> String {
    let Ok((records, _diagnostics)) = journal::reader::read_all(journal) else {
        // A journal that cannot be read is a park that was not observed.
        return outcome_label(outcome).to_string();
    };

    match records
        .iter()
        .rev()
        .find(|record| record.kind == "parked")
        .and_then(|record| record.rest["reason"].as_str())
    {
        Some(reason) => format!("{PARKED_LABEL_PREFIX}{reason}"),
        None => outcome_label(outcome).to_string(),
    }
}
```

**The LAST park wins** (`:433-436`). `outcome_label` (`:403-415`) is the only other
string source and is `pub(crate)` so the render layer proves against it — a new
terminal vocabulary goes through `outcome_label` or through the `parked:` prefix,
**never as a third string source**:

```rust
pub(crate) fn outcome_label(outcome: &RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::SucceededWithChanges { .. } => "succeeded_with_changes",
        // … nine arms, no wildcard
    }
}
```

**`make_run_record` field pattern** (`:482-518`) — where cap overrides land if they
are recorded on `run.json`. Note `gsd_command: args.command.clone()` at `:494`
becomes the *first* command or the goal under a loop; per-iteration commands belong
on `Decided` events, not on a widened `RunRecord` field. `claude_code_version` at
`:513` carries the "record what is known; nothing overwrites it, because `run.json`
is written exactly twice" comment — the same constraint binds any new field.

---

### `src/journal/mod.rs` (MODIFY — emit reserved variants)

**Analog:** the reserved variants themselves (`:650-670`) — **no schema change, no
migration (D-36).** Emit them as written:

```rust
/// What the driver observed about a project's state before deciding.
///
/// **Schema only in this phase — Phase 20 emits it.** It is present now so
/// that phase adds no schema migration (D-36).
Observed {
    /// The phase number the observation is about.
    phase: String,
    /// The five D-R-P-E-V stage statuses, in order.
    drpev: Vec<String>,
},
/// What the driver decided to run next, and why.
///
/// **Schema only in this phase — Phase 20 emits it** (D-36).
Decided {
    /// `"policy"`, `"llm"`, or `"human"`.
    by: String,
    /// The GSD command the decision selected.
    command: String,
    /// Why this command, in one line.
    rationale: String,
},
```

`Decided.by` is a closed three-value vocabulary; Phase 20 emits `"policy"` and
nothing else. `Observed.drpev` is length 5, in order (Assumption A5).

**Reason-field pattern to reuse for bounds parks** (`:815-833`):

```rust
Parked {
    /// Why the run parked.
    ///
    /// A short stable identifier, in the same convention
    /// [`JournalEvent::Diagnostic`]'s `code` field documents. The taxonomy
    /// is [`crate::envelope::policy::ParkReason`] and this string is always
    /// its `as_str()` — **one list rather than two**, so a reader who greps
    /// for `force_push_blocked` finds the producer and the record together.
    reason: String,
    /// What would unpark it, in the same register as [`Self::reason`]: a
    /// short phrase naming the actor, not a sentence of advice.
    needs: String,
},
```

The `reason` doc must be widened to name the sibling enum as a second sanctioned
taxonomy — otherwise the doc says "the taxonomy is `ParkReason`" while a bounds
reason rides the same field.

---

### `src/driver/mod.rs` + `src/cli.rs` (MODIFY — config/DTO, request-response)

**Analog:** the existing `DriveArgs` fields and their paired CLI arm.

**`DriveArgs` field pattern** (`mod.rs:98-135`) — a plain owned record so the TUI
spawn path and a hand-typed invocation build the same value; each field's doc
carries the decision id and the refusal that guards it:

```rust
/// A plain owned record rather than a borrow of the CLI enum, so the TUI's
/// spawn path (plan 17-05) and a hand-typed invocation build the same value.
#[derive(Debug, Clone)]
pub struct DriveArgs {
    pub alias: String,
    /// The single GSD command to run.
    ///
    /// **Exactly one, and that is the whole of this phase's execution model.**
    /// Phase 20 owns the decision router that turns a goal into a sequence; a
    /// field that accepted a sequence now would imply a loop that does not
    /// exist.
    pub command: String,
    // …
    /// Free text recorded into `RunRecord.goal` and never interpreted.
    pub goal: Option<String>,
}
```

This doc is now false and must change in the same commit as the loop, along with
the module-doc claim at `mod.rs:34-36` and the test
`drive_args_carry_the_single_command_the_router_phase_will_replace` (`:538-547`).

**Validate-at-the-seam pattern** (`mod.rs:280-293`) — the precedent for where the
new `--target-phase` value is checked. Not in a `value_parser`, but in `drive`,
because the value arrives from three paths:

```rust
let Some(run_id) = args.run_id.as_deref() else {
    return Err(DriveError::RunIdRequired);
};
if !journal::is_plain_path_component(run_id) {
    return Err(DriveError::RunIdInvalid {
        run_id: run_id.to_string(),
    });
}
```

**CLI arm pattern** (`cli.rs:47-75`) — `//` comments carry the rationale, doc
comments carry only the one line a user needs:

```rust
// Exactly one command. The decision router is Phase 20's, so there is
// deliberately no way to pass a sequence.
/// The single GSD command to run, e.g. `/gsd-progress`
#[arg(long)]
command: String,
```

---

### `src/driver/dry_run.rs` (MODIFY — pinned contract text)

**Analog:** the sibling constants in the same file. The rule is stated at `:80-95`:
changing this text is a user-visible break and the paired test moves in the same
commit (CONVENTIONS.md:75, TESTING.md:74).

**The now-false text** (`:69-72`):

```rust
/// The commands header (**pinned contract** — see [`SECTION_REFSPECS`]).
pub const SECTION_COMMANDS: &str = "== GSD commands this run would issue ==\n\
    The decision router is Phase 20, so the single --command argument below is\n\
    the complete and honest sequence for this build — not a truncated one.";
```

**The paired-test rule to obey** (`:82-90`):

```rust
/// **These four constants are a contract, not decoration.**
/// `dry_run::tests::the_rendered_report_carries_all_three_section_headers_in_order`
/// asserts all four appear, in this order, by comparing byte offsets … Changing
/// their text is a user-visible output change and breaks anyone scripting
/// against the preview.
```

**The field doc that also becomes false** (`:105-111`):

```rust
/// The GSD commands the run would issue, in order.
///
/// **One element, because this phase runs exactly one command** — not
/// because the builder is a stub. Phase 20's decision router is what makes
/// this longer; until it exists, a single element is the truth.
pub commands: Vec<String>,
```

Type is already `Vec<String>` — no type change. Sibling sites carrying the same
now-false claim: `src/driver/mod.rs:34-36`, `:98-102`, `:538-547`, `src/cli.rs:51-55`,
`src/driver/dry_run.rs:107-111`.

---

### Tests (CREATE — integration, batch)

**Analog A — the conformance oracle and the rule/test-table guard:**
`tests/spawn_seam_guard.rs`'s `SPAWN_ALLOWLIST` shape. It fails in **both**
directions: an unlisted item is a violation, and a listed item that matches nothing
is *also* a violation (*"the allowlist is now wider than the truth it describes"*,
TESTING.md:64). Apply the identical shape so a rule with no row, or a row with no
rule, fails the build. Add the non-vacuity floor and the control arm that both
`spawn_seam_guard.rs` and `async_blocking_guard.rs` carry:

```rust
/// The floor below which this audit is examining too little to mean anything.
///
/// Non-vacuity in the register `tests/spawn_seam_guard.rs` already uses: an
/// audit that walked nothing passes for the wrong reason …
const MIN_ASYNC_BODY_LINES: usize = 800;
```

Live at `tests/`, not in-source: *"a test that walks src/ has no business living
inside it"* (TESTING.md:48). Skip-with-a-loud-reason if `gsd-tools` is absent, and
assert at least one fixture produced a comparison.

**Analog B — asserting an on-disk journal event read back by a separate process:**
`tests/envelope_wiring.rs:58-82`. Copy the helper and its justification verbatim
for the bounds/router park assertions:

```rust
/// Every `Parked` event in the journal **at `journal_path` on disk**, as
/// `(reason, needs)` pairs.
///
/// **Reading the file rather than an in-memory handle is the entire point.** …
/// A helper that inspected a `JournalRun` this process was holding would prove
/// that this process can write what it just wrote.
///
/// `reader::read_all` is the shipped reader, tolerant parse and all, so a record
/// of a kind this build did not model still arrives rather than being dropped.
fn parked_events(journal_path: &Path) -> Vec<(String, String)> {
    let (records, _diagnostics) = reader::read_all(journal_path).expect("the journal is readable");
    records
        .iter()
        .filter(|record| record.kind == "parked")
        .map(|record| {
            (
                record.rest["reason"].as_str().unwrap_or_default().to_string(),
                record.rest["needs"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}
```

Also copy the file-header rule (`envelope_wiring.rs:1-23`): **no assertion may read
a model's summary.** Every fact is an exit code, a file on disk, a git ref, or a
journal event. That rule is D-10 restated and it binds this phase's goal-met and
terminal-classification tests directly.

Fixture-struct pattern: `RunFixture` (`envelope_wiring.rs:90-141`) — a `TempDir`
held for its `Drop`, the run built through the production `JournalRun::start` path
rather than by writing the files by hand, *"because the thing under test is that a
separate process can find this run through the same `active` pointer the driver
writes."*

**Analog C — the async-blocking lint allowlist** (`tests/async_blocking_guard.rs`).
Two edits, both in the same commit as the code they cover.

`BLOCKING_HELPERS` (`:124-138`) gains `"RunSnapshot::capture("` and
`"capture_snapshot("` — the pre-existing hole Pitfall 5 identifies:

```rust
const BLOCKING_HELPERS: &[&str] = &[
    "build_report(",
    "establish_envelope(",
    "terminal_label(",
    "lock::acquire(",
    "JournalRun::start(",
    "inbox::tail(",
    // …
];
```

`ASYNC_BLOCKING_ALLOWLIST` (`:188-211`) gains
`("src/executor/claude.rs", "RunSnapshot::capture(")` with a join-failure-fallback
reason in the register the two existing entries of that shape already use:

```rust
// The terminal record's two join-failure fallbacks, the same shape and the
// same reason: the label is read inside `spawn_blocking`, and the inline
// re-run keeps a park reason on `run.json` rather than losing it merely
// because a task failed to join. A run that ends with no record at all is
// the one failure OBS-01 cannot tolerate.
("src/driver/run.rs", "terminal_label("),
```

The constant's own doc states the standard the entry must meet (`:167-172`):
*"**This is a declared allowlist, not a habit.** Whoever adds an entry adds it in
the same commit as the code it covers."* `no_allowlist_entry_is_stale` (`:638`)
refuses an entry that suppresses nothing, so the entry itself proves the hole was
real.

## Shared Patterns

### Exhaustive match, no wildcard, as a compile-time gate
**Source:** `src/error.rs:530-547` (`DriveError::source()`), `src/driver/reconcile.rs:284-286`
(`RunVerdict`), `src/driver/run.rs:403-415` (`outcome_label`), `src/envelope/policy.rs:178-189`
(`ParkReason::as_str`)
**Apply to:** the router's rule table, the bounds detector ordering, the terminal
classification enum, and every `match` on the extended `DiskStatus`.
Spell every arm out, join identical bodies with `|`, never a `_` arm. Criterion 5's
*"never as an unclassified 'loop ended'"* becomes a type-level property this way:
if the terminal enum has no unclassified arm and every match is exhaustive, the
failure mode is unrepresentable.

### Stable snake_case reason string, one constant per arm
**Source:** `src/envelope/policy.rs:33-53, 172-189`
**Apply to:** the router reason enum, the bounds reason enum, any new journal
`Diagnostic` code. One list, not two: the `REASON_*` const is what a later reader
greps, and the enum arm returns it rather than a fresh literal.

### Log the error KIND only, never a message body
**Source:** `src/driver/run.rs:1536-1540`
**Apply to:** every `tracing::warn!`/`debug!` the loop, the router and the
rate-limit observer add.
```rust
tracing::warn!(kind = ?err.kind(), "journal write failed");
```
The journal lands in the driven project's `.planning/`, a directory users commit
(T-17-05, T-18-03, SAFE-04). New journal records this phase adds — `Observed`,
`Decided`, bounds `Parked` — carry only enum values and command names.

### Tolerant `&str` matching with an explicit fallback carrying the observed value
**Source:** `src/executor/outcome.rs:9-12` and `:203-208`; `src/executor/stream_json.rs:14-18`
**Apply to:** `rate_limit_info.rateLimitType`, `rate_limit_info.status`, and the
VERIFICATION frontmatter `status`.
```rust
//! `subtype` and `terminal_reason` are matched as `&str` with an explicit
//! fallback arm, never as typed enums: this CLI shipped three new values on one
//! version line, and a typed enum would need a catch-all on each and would
//! still lose the actual string.
```
No `deny_unknown_fields`, no panicking accessor, no `unwrap` on a wire field
(V5, security domain).

### Blocking work handed to `spawn_blocking` with an inline join-failure fallback
**Source:** `src/driver/run.rs:1675-1694`; `src/executor/outcome.rs:50-59`
(`RunSnapshot::capture`'s own doc: *"Does full-tree file I/O **and shells out to git
twice**, so callers must run the whole capture on a blocking thread"*)
**Apply to:** the per-iteration observe step and any journal read inside the loop.

### One entry point, never a second envelope-discarding sibling
**Source:** `src/executor/outcome.rs:14-20` and `:167-171`
**Apply to:** the rate-limit path — observe it in the drain loop; do **not** widen
`derive_run_outcome_from_envelopes`'s signature. That is the shape of the CR-04 bug:
*"Two entry points onto one matrix is what made that possible, so there is now one."*

## No Analog Found

None. Every file in this phase has a same-role, same-data-flow analog in the tree.

The nearest thing to a gap is the **synthesised `status: "rejected"` rate-limit
fixture** — `tests/fixtures/transcripts/` holds eight real 2.1.220 captures and two
carry `rate_limit_event`, but no `rejected` capture exists and one cannot be
produced without burning quota. There is no in-tree precedent for a *synthesised*
transcript line; the plan should follow the existing fixture format exactly, take
field names and enum values verbatim from RESEARCH's binary-verified inventory, and
label it as synthesised in `tests/fixtures/transcripts/README.md` so no future
reader mistakes it for a capture.

## Metadata

**Analog search scope:** `src/driver/`, `src/envelope/`, `src/executor/`,
`src/state_reader/`, `src/journal/`, `src/cli.rs`, `tests/`
**Files read:** `src/envelope/policy.rs`, `src/executor/outcome.rs`,
`src/driver/run.rs`, `src/driver/mod.rs`, `src/driver/dry_run.rs`, `src/cli.rs`,
`src/journal/mod.rs`, `src/state_reader/disk_status.rs`,
`tests/async_blocking_guard.rs`, `tests/envelope_wiring.rs`
**Pattern extraction date:** 2026-08-19
