---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-21T03:35:00Z
depth: standard
diff_base: 9ca152173c4177476bd506ddf49509607ec9c331
files_reviewed: 13
files_reviewed_list:
  - src/cli.rs
  - src/driver/dry_run.rs
  - src/driver/goal.rs
  - src/driver/mod.rs
  - src/driver/run.rs
  - src/error.rs
  - src/journal/mod.rs
  - src/ui/screens/driver_confirm.rs
  - tests/async_blocking_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
  - tests/driver_refusal_record.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 2
  warning: 5
  info: 4
  total: 11
status: issues_found
---

# Phase 21 (gap closure): Code Review Report

**Reviewed:** 2026-08-21T03:35:00Z
**Depth:** standard
**Diff base:** `9ca1521..HEAD`
**Files Reviewed:** 13
**Status:** issues_found

## Summary

The four gap-closure changes were traced against the code rather than the commit
messages, and three of the four hold up under attack:

* `CommandSource` is genuinely exhaustive — `preview_text` has three arms, no
  fall-through, and `--goal X --dry-run` now renders honestly (verified against
  the built binary, not only against the tests). The TUI's second preview path
  (`AppContext::schedule_dry_run_report`) is not a residue, because
  `spawn::drive_argv` always emits `--command` and the TUI cannot produce a
  goal-only invocation at all.
* `plan_digest`'s move to `journal::sha256_digest` is real, and the `+` split in
  `parse_approval_token` is genuinely unambiguous: both halves are
  `sha256:`/`fnv1a64:` + lowercase hex, `+` is in neither alphabet, and all four
  malformed shapes (absent / repeated / empty-left / empty-right) return a named
  error rather than a half-approval.
* `ApprovalRefusal::PlanChanged` is now reachable, and a files-only change still
  lands on `DisclosedFilesChanged` — the recorded plan half genuinely arrives
  from the caller's token.
* `do_toggle_opt_in`'s restore is correct in **both** directions, not only the
  one the fix targeted: `registry::is_opted_in` is exactly
  `driver_opt_in.is_some()`, so `was_opted_in == false` implies `previous ==
  None`, and `clear_opt_in` in that branch is equivalent to putting `previous`
  back. The three new tests pin the withdrawal direction against a re-baseline.
* `finish_run` is the single terminal-write site inside `src/driver/run.rs`, and
  the spawn-failure path really does stamp `escalations_used` now — confirmed by
  reading a `run.json` produced by a live spawn failure.

Two defects remain that the phase's own stated invariants claim to have closed,
both reproduced against the built binary. Both are in `src/driver/mod.rs`'s
ordered refusal chain — the file whose doc says an invocation malformed *as an
invocation* is refused before anything else and identically for a preview and a
real run.

`cargo test` is green (1018 unit + all integration suites). `cargo clippy
--all-targets -- -D warnings` fails on five pre-existing lints in files this diff
does not touch (`src/browser.rs`, `src/project_creator.rs`,
`src/state_reader/mod.rs`); not reported as a finding.

## Critical Issues

### CR-01: A malformed `--approved-plan` token spawns an agent and spends a model consultation before it is refused, and `--dry-run` never refuses it at all

**File:** `src/driver/mod.rs:730-760`, `src/driver/mod.rs:825-830`, `src/error.rs:619-628`

**Issue:** `error.rs:626-627` states that `PlanApprovalMalformed` "is raised
before any other approval work happens", and `approve_plan`'s body comment
(`src/driver/mod.rs:824-826`) states "**Parsed first, before any of the work
below.** A value that is not a token cannot approve anything, so nothing is
computed on the strength of it." Both claims are true only *within*
`approve_plan`. At the invocation level the order in `drive` is:

```
730  let decomposed = ... decomposition.decompose(...).await?;   // spawns `claude`, spends 1 escalation
757  let approved_plan = ... approve_plan(&project, &args, plan)?; // parse_approval_token happens HERE
```

So a token with a typo costs a full model consultation and a real process spawn
into the driven repository before a pure string check refuses it. Reproduced
against the built binary:

```
$ gsd-meta-manager --config cfg.json drive demo --goal 'g' --run-id r1 \
    --approved-plan 'sha256:aa'
Error: the goal-decomposition seam produced nothing usable ... (reason:
escalation_output_unusable): the seam could not be spawned ...
```

The refusal reported is the *seam*, not the malformed token — the parse never
ran. The same value passed with `--dry-run` exits **0** with a clean preview and
no mention of the token at all:

```
$ gsd-meta-manager ... drive demo --goal 'g' --dry-run \
    --approved-plan 'total-garbage-no-separator'
DRY RUN — nothing below was executed. ...   (exit 0)
```

That is precisely the asymmetry `drive`'s own doc (step 3, and the WR-09 comment
at `src/driver/mod.rs:470-477`) forbids: "each is answered identically whether or
not the run is real and a preview that answered them differently would be
previewing something the user cannot run." `tests/driver_dry_run.rs`'s
`a_preview_refuses_exactly_what_the_real_run_would_refuse` covers
`--target-phase` and `--max-steps` but not this flag.

`tests/driver_goal_seam.rs::a_half_supplied_approval_token_is_refused_by_name_
and_never_treated_as_an_approval` passes only because its harness plants a
payload the seam returns, so the decomposition succeeds and the parse is reached.
It cannot detect the ordering.

**Fix:** Parse at the seam in `drive`, in the pure-refusal group above the
dry-run branch, and pass the parsed halves down:

```rust
// beside the --target-phase and bounds refusals, above `if args.dry_run`
let recorded_approval = match args.approved_plan.as_deref() {
    Some(raw) => Some(journal::parse_approval_token(raw).map_err(DriveError::PlanApprovalMalformed)?),
    None => None,
};
...
#[cfg(unix)]
let approved_plan = match decomposed.as_ref() {
    None => None,
    Some(plan) => Some(approve_plan(&project, recorded_approval.as_ref(), &project_inputs, plan)?),
};
```

and delete the now-duplicated parse from `approve_plan` (its doc comment moves
with it). Add an arm to
`a_preview_refuses_exactly_what_the_real_run_would_refuse` for a malformed token,
so the preview/real symmetry is pinned rather than asserted.

### CR-02: An empty or whitespace-only `--command` still renders `1 command in the sequence:` and a blank numbered entry, and records `gsd_command: ""` on a real run

**File:** `src/driver/mod.rs:360-377`, `src/driver/dry_run.rs:254-264`, `src/driver/dry_run.rs:395-401`, `src/driver/mod.rs:1459-1518`

**Issue:** `command_source` trims and rejects a blank `--goal`
(`src/driver/mod.rs:370-374`, with a comment explaining exactly why), but applies
no such rule to `--command`: `Some("")` matches the `(Some(command), None)` arm
and becomes `CommandSource::Command("")`. `preview_text` then routes it to
`build_report(project, "")`, which pushes `vec![""]`
(`src/driver/dry_run.rs:256`), and the `Complete` arm prints a total of 1
(`src/driver/dry_run.rs:396-400`).

That is byte-for-byte the CR-01 output shape this gap closure exists to
eliminate. Reproduced against the built binary:

```
== GSD commands this run would issue ==
... For a supplied command, the line below is the complete and honest sequence. ...
  1 command in the sequence:
    1.
```

and with `--command '   '`:

```
  1 command in the sequence:
    1.
```

The new invariant test at `src/driver/mod.rs:1459-1518`
(`every_command_source_renders_a_preview_with_no_empty_numbered_command`) asserts
"**no** preview may render an empty numbered entry", but its `sources` array
carries only non-empty payloads, so the property it names is not the property it
checks. Adding `CommandSource::Command(String::new())` to that array makes it
fail today.

The real-run consequence is worse than the preview one. Reproduced:

```
$ gsd-meta-manager ... drive demo --command '' --run-id r-empty --claude-program /bin/true
$ cat .planning/meta-manager/runs/r-empty/run.json
  "gsd_command": "",
  "escalations_used": 0,
  "outcome": "spawn_failed"
```

An agent spawn is attempted with no instruction, a run directory is created, and
`gsd_command` is written as `""` — which
`ROUTED_RECORD_MARKER`'s own doc (`src/driver/mod.rs:393-397`) declares must
never happen: *"`\"\"` already means 'field absent' on the tolerant read path
(D-30)"*. `run_summary_from_value` reads it back through
`string_field("gsd_command")`, which returns `""` for both absent and empty, so
this run is indistinguishable in the run list from one written by a build with no
such field.

**Fix:** Refuse a blank command at the same seam and in the same register as the
blank goal:

```rust
match (command, target_phase) {
    (Some(_), Some(_)) => Err(DriveError::AmbiguousCommandSource),
    // A command made of nothing is nothing to do, exactly as `--goal ' '` is.
    (Some(command), None) if !command.trim().is_empty() => {
        Ok(CommandSource::Command(command.to_string()))
    }
    (Some(_), None) => Err(DriveError::NoCommandSource),
    (None, Some(target_phase)) => Ok(CommandSource::Routed(target_phase.to_string())),
    (None, None) => match goal { ... }
}
```

and add `CommandSource::Command(String::new())` plus
`CommandSource::Command("   ".into())` to the enumeration in
`every_command_source_renders_a_preview_with_no_empty_numbered_command`, so the
invariant the test names is the invariant it holds. Note the `matches!` sweep in
that test keys on index, so it needs re-shaping to a per-variant count rather
than a per-position one.

## Warnings

### WR-01: Guard six is scoped to one file while `JournalRun::finish` is `pub`, and its stated limits claim a failure direction it does not have

**File:** `tests/spawn_seam_guard.rs:1673-1700`, `tests/spawn_seam_guard.rs:1707-1713`, `tests/spawn_seam_guard.rs:1723-1814`

**Issue:** The guard header (lines 1687-1699) states two over-approximations and
concludes "**Both fail in the over-detection direction — loud, not silent** —
which is the direction a guard may err in." That characterisation is not true of
the guard as a whole. `TERMINAL_WRITE_HOME` pins the scan to
`src/driver/run.rs`, and `JournalRun::finish` is `pub`
(`src/journal/mod.rs:1771`) on a `pub struct`. A terminal write added from
`src/driver/kill.rs`, `src/driver/reconcile.rs`, `src/app.rs` or any UI screen is
invisible to the guard — a *silent* under-detection, which is the direction the
header says it does not have.

A second unstated under-approximation: `TERMINAL_WRITE_CALL` is `".finish("`, so
the UFCS form `JournalRun::finish(&mut journal, label)` is not matched.

The control-arm test (`the_terminal_write_scanner_reports_a_bare_call_and_not_
the_helpers_own`) verifies attribution inside one synthetic file; it cannot see
either gap.

**Fix:** Either widen the scan to every file under `src/` with an explicit
`(file, fn)` allowlist for `finish_run` — the shape guard one already uses for
`from_registry` — or, at minimum, correct the header so the file scoping and the
UFCS spelling are named as silent under-detections rather than folded into a
"loud, not silent" claim. If the scan stays scoped, adding an assertion that
`.finish(` appears in no other `src/` file at all is one line and closes the
larger of the two.

### WR-02: `registry::current_prompt_inputs` does blocking file I/O inside two `async fn` bodies and is not in `BLOCKING_HELPERS`

**File:** `src/driver/mod.rs:833`, `src/driver/run.rs:2325`, `tests/async_blocking_guard.rs:124-161`

**Issue:** `current_prompt_inputs` calls `file_digest`, which is
`std::fs::read(...)` plus SHA-256 per disclosed file
(`src/registry.rs:195-214`), for each of the five entries in
`DISCLOSED_PROMPT_INPUTS`. It is called synchronously from `approve_plan`, which
`async fn drive` calls inline (`src/driver/mod.rs:759`), and directly inside
`async fn execute_run` at `src/driver/run.rs:2325` — neither behind
`spawn_blocking`.

`tests/async_blocking_guard.rs` reports green because the `fs::read(` sits
one call deep inside `registry::file_digest`, and `current_prompt_inputs(` is not
listed in `BLOCKING_HELPERS`. This is exactly the residue that list's own doc
names ("A new synchronous seam that nobody adds here is still invisible"), and
the same commit remembered to add `build_goal_report(` for the identical reason.
The `execute_run` call predates this diff; the `approve_plan` call is on the path
this gap closure reworked.

Practical impact is small (five small reads on a foreground CLI invocation), but
the guard reporting green on a seam it cannot see is the failure mode the file
argues is worse than no guard.

**Fix:** Add `"current_prompt_inputs("` to `BLOCKING_HELPERS` in the same commit,
then either wrap both call sites in `spawn_blocking` or add the two
`(file, marker)` entries to `ASYNC_BLOCKING_ALLOWLIST` with the one-line reason
that convention requires.

### WR-03: The "a legacy `fnv1a64:` record fails closed as `PlanChanged`" claim describes a path production cannot take

**File:** `src/driver/goal.rs:750-757`, `src/journal/mod.rs:836-844`, `tests/driver_goal_seam.rs:1133-1181`

**Issue:** Both docs assert that a **record** carrying a legacy `fnv1a64:` plan
digest "re-checks as `ApprovalRefusal::PlanChanged` and fails closed", and name
`a_recorded_approval_carrying_a_legacy_fnv1a64_plan_digest_re_checks_as_stale`
as proof. Neither is a production path:

* `ApprovedPlan` is only ever *constructed* in-process by `approve_plan` and
  serialised into `run.json`. It is never deserialised and re-checked — grep
  confirms the only reads of `RunRecord::approved_plan` are the round-trip test
  at `src/journal/mod.rs:2560` and the `None` fixtures. So no recorded legacy
  plan digest is ever fed to `recheck_approval`.
* The value a user could actually still be holding is a legacy **token** — the
  single-digest `--approved-plan` value an earlier build printed. That fails at
  `parse_approval_token` with `ApprovalTokenError::SeparatorAbsent`, not with
  `PlanChanged`.
* The named test constructs an `ApprovedPlan` by hand and calls
  `recheck_approval` directly, so it exercises the predicate rather than any
  route into it.

Both directions fail closed, so there is no security gap. The problem is that
two docs claim a mechanism and a pinning test that do not correspond to reality,
in a codebase whose stated rule is that a doc describing a build other than the
current one "is worse than no doc — it is read as current."

**Fix:** Reword both docs to say what actually fails closed: a *legacy single-half
token* on argv is refused by `parse_approval_token` as `SeparatorAbsent`, and no
recorded `ApprovedPlan` is ever re-read, so no migration is needed for a
different reason than the one stated. Keep the `recheck_approval` unit test but
rename it to say it pins the predicate rather than a record path.

### WR-04: The spawn-gate re-check passes the same digest on both sides — a latent re-run of the defect WR-01 named

**File:** `src/driver/run.rs:2304-2328`

**Issue:** The gate calls

```rust
journal::recheck_approval(
    Some((&approved.plan_digest, &approved.approval_digest)),
    &approved.plan_digest,
    &crate::registry::current_prompt_inputs(project.root()),
)
```

The recorded plan half and the observed plan half are the same `String`, so the
first comparison in `recheck_approval` is a value against itself — structurally
identical to the defect this cycle removed one call site up. The 17-line comment
argues this is sound because the goal is decomposed exactly once above the run,
which is true of the code today. But nothing enforces it: the argument lives in a
comment, and `recheck_approval` now takes two loose `&str`s that make the
tautological call trivially expressible. If any future change re-derives a plan
between `drive` and `execute_run`, this silently reverts to a check that cannot
fail while looking like one that can.

**Fix:** Make the intent structural rather than commentary. Either split the
predicate — a `recheck_disclosed_files(recorded_approval_digest, plan_digest,
inputs)` for the spawn gate and the full two-half `recheck_approval` for
`approve_plan` — or thread the token halves down into `execute_run` so the spawn
gate compares the *caller's* recorded plan digest against the record's, and the
comparison becomes real on both sides.

### WR-05: `PlanStep::rationale` is bounded, stored, and read by nothing — the review surface it exists for prints only the triples it calls unreviewable

**File:** `src/driver/goal.rs:143-150`, `src/driver/goal.rs:452-455`, `src/driver/goal.rs:697-721`, `src/driver/mod.rs:839-850`

**Issue:** `FIELD_RATIONALE`'s doc says the field exists because "DRIVE-03
requires the user to *review* the plan before it runs, and a list of
`verb phase verification_passed` triples is reviewable only by someone who
already knows the answer." The only review surface —
`DriveError::PlanApprovalRequired` — is built from
`format!("command={} phase={} terminal={}", ...)` at `src/driver/mod.rs:842-848`,
which is exactly that list of triples. `ApprovedPlan::steps`' doc explicitly
excludes the rationale from `run.json` too.

Grep confirms `PlanStep::rationale` has no production reader anywhere: the only
non-test reference is the write at `src/driver/goal.rs:720`. So the field is
effectively dead, and the DRIVE-03 review requirement is served by the surface
its own doc calls insufficient.

**Fix:** Either render the rationale in the refusal (already sanitized end-to-end
via `sanitize_render_line` on the step string — extending the `format!` is a
one-line change and the bound at construction already covers it), or delete the
field and the doc paragraph that justifies it. Keeping both is a stored,
attacker-influenced string that nothing reads and a stated requirement nothing
satisfies.

## Info

### IN-01: The goal preview still says `--approved-plan` takes a "digest" after the flag became a two-half token

**File:** `src/driver/dry_run.rs:426-435`

**Issue:** The `GoalNotDecomposed` prose ends "...prints the plan together with
the `--approved-plan` **digest** that authorises exactly it." `src/cli.rs:136`,
`src/error.rs:606-614` and `journal::render_approval_token` were all updated to
say *token*; this string, which is inside a pinned-contract section, was not.
Verified in the rendered output.

**Fix:** `--approved-plan token that authorises exactly it`. The pinned-header
test at `src/driver/dry_run.rs:758-776` is the natural place to assert the word.

### IN-02: `ScopedPreview` lets `GoalNotDecomposed` carry a non-empty command list

**File:** `src/driver/dry_run.rs:186-242`, `src/driver/dry_run.rs:320-358`

**Issue:** `ScopedPreview.report` and `.scope` are both `pub`, so
`PreviewScope::GoalNotDecomposed { .. }` beside a non-empty `commands` vector is
representable. `render_with_scope` would then print numbered command entries
directly beneath the text "no command can be shown, and none is withheld." The
emptiness is a property of `build_goal_report` only, not of the type — which is
the same "add beside rather than through" shape the `CommandSource` promotion was
made to remove.

**Fix:** Either move the scope inside `DryRunReport` behind a constructor that
enforces the pairing, or add a `debug_assert!(report.commands.is_empty())` in the
`GoalNotDecomposed` arm of `render_with_scope`.

### IN-03: `plan_target_phase(plan).unwrap_or_default()` silently yields an empty target phase

**File:** `src/driver/mod.rs:878-881`

**Issue:** The comment correctly explains why this is not `unwrap()`, but
`unwrap_or_default()` turns an empty plan into `target_phase: ""`, which becomes
`args.target_phase = Some("")` and then a map key into
`phase_disk_statuses` that matches nothing — a run that drives toward nothing
while looking configured. The only thing preventing it is `legality`'s empty-plan
refusal in a different module.

**Fix:** Return a typed refusal instead:
`run::plan_target_phase(plan).ok_or(DriveError::…)?`, or reuse
`DriveError::TargetPhaseInvalid { target_phase: String::new() }`.

### IN-04: The zero-budget refusal says a budget of 0 "was already spent"

**File:** `src/driver/run.rs:2001-2010`

**Issue:** With `--max-steps 1`, `escalate::resolve` reduces the default cap to
`min(3, 0) = 0`, and the decomposition then refuses with (verified against the
binary):

```
Error: ... (reason: escalation_cap_reached): the run's model-consultation budget
of 0 was already spent before the goal could be decomposed.
```

Nothing was spent — the budget was zero from the start, because the step cap left
no room for one that could bind. A reader chasing "already spent" will look for a
consultation that never happened.

**Fix:** Branch the detail on `budget.cap() == 0`, e.g. *"this run's step cap of
1 leaves no room for a model consultation that could bind, so the goal cannot be
decomposed — raise `--max-steps`"*, which also names an action.

---

_Reviewed: 2026-08-21T03:35:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
