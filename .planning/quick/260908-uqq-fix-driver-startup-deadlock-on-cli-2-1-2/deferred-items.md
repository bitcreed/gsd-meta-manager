# Deferred items — 260908-uqq

Out-of-scope discoveries made while executing this quick task. Logged, **not
fixed**: none of them is caused by this change, and each would widen the diff
past the deadlock it exists to close.

## 1. `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` — FAILING (pre-existing)

Present at the base commit `0dde2b4`, before any edit in this task. The
assertion is a deliberate **schedule**, not a control: it fires when the
installed git moves off the version the two config-section constants were
derived against.

```
derived against : "git version 2.43.0"
installed       : "git version 2.53.0"
```

Its own panic message states the correct response — re-derive
`INDIRECTION_SECTIONS` and `REPARSED_COMMAND_SECTIONS` against git 2.53.0 and
update `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION`, never delete the
assertion. That is a research task against git 2.53's release notes, not a code
fix, and it is unrelated to the startup handshake.

**Note for future runs:** the plan for this task expected "two pre-existing
`driver_reattach` failures". That is stale. The actual pre-existing failure set
at `0dde2b4` is this one test, and `driver_reattach` is green.

## 2. `driver::run::tests::the_current_group_agrees_with_the_proc_parse` — FLAKY (pre-existing, 1 occurrence in 5 runs)

Failed exactly once, during a full `cargo test --no-fail-fast`:

```
left:  2712294   (getpgrp())
right: 2716792   (liveness::process_group's /proc/<pid>/stat parse)
```

Did not reproduce in three consecutive full `cargo test --lib` runs or in a
second full `cargo test --no-fail-fast` run. `getpgrp()` cannot return a stale
value, so the suspect is the `/proc/<pid>/stat` field parse under the load of
many test binaries running concurrently.

Not attributable to this change: the test cross-checks two readings of the
*test harness's own* process group and has no dependency on the executor's
prompt release, the capability gate or the spawn-failure label mapping. No test
in the tree calls `establish_own_group()`, so the cross-test `setpgid` hazard
the test's own doc names is not the mechanism.

Worth a look on its own terms, because the parse it cross-checks is what
`kill::resolve_signal_target` uses — a parse that can misread would refuse a
stop, which is D-04's failure mode. Out of scope here.

## 3. `cargo clippy --all-targets -- -D warnings` — 4 errors (pre-existing)

All in `src/project_creator.rs`'s in-source test module
(`clippy::bool_assert_comparison`, `clippy::cmp_owned`). Untouched by this
change.

The gate this task's plan specifies is `cargo clippy -- -D warnings`, which
does not compile test targets and **exits 0**. Flagged only so a later reader
who reaches for `--all-targets` is not surprised.

## 4. The stderr / `events_rx` pre-gate backpressure hazard — REPORTED, NOT FIXED

Carried forward from the task's CONTEXT as an explicit non-goal. See the
SUMMARY's "Reported, not fixed" section for the full statement: nothing drains
`events_rx` before the gate, so `read_stderr`'s `tx.send().await` can park, and
`last_line_at` is stamped *before* that send — so a child that floods stderr
before announcing itself can freeze the idle clock. Buffered stderr is also
dropped when `events_rx` is dropped on the error path.

`prompt_release_grace` shrinks the exposure window from "until the idle cap"
(15 minutes) to "until the grace expires plus init latency" (~5 seconds). It
does not close it. Closing it needs a sink that survives the error path — the
journal, in `run.rs` — which is not cheap.
