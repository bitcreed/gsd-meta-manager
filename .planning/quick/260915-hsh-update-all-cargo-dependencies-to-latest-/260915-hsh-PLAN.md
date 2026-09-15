---
phase: quick-260915-hsh
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.lock
  - Cargo.toml
  - src/executor/claude.rs
  - README.md
  - CONTRIBUTING.md
  - CLAUDE.md
  - docs/DEVELOPMENT.md
  - docs/GETTING-STARTED.md
  - docs/TESTING.md
  - src/journal/redact.rs
  - .github/workflows/release.yml
autonomous: true
requirements: [QUICK-260915-hsh]
user_setup: []

estimate:
  tokens: 50000
  raw_tokens: 50000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Every lockfile entry that can advance under semver-compatible resolution has advanced, and the tree still builds, lints clean at the project gate, and shows NO NEW test failure beyond a baseline captured from THIS tree."
    - "`dirs` and `process-wrap` each sit at their latest major version — OR the one that could not be reached is back at its prior version with a written reason, no partial edit, and no half-migrated call site."
    - "The declared `rust-version` equals the maximum `rust_version` across the resolved graph, MEASURED after the updates rather than inherited from the pre-update measurement."
    - "The baseline that `no new failures` is judged against was captured by running the suite on unmodified HEAD in this session, not copied from a prior task's summary."
    - "Each dependency change is a separate commit that reverts alone, without dragging the other two with it."
    - "`notify` stays on 8.x and `notify-debouncer-full` on 0.7.x; no release-candidate version is present in the lockfile."
  artifacts:
    - "Cargo.lock advanced to the latest semver-compatible versions of the whole graph"
    - "Cargo.toml with the `dirs` and `process-wrap` requirements at their reached versions (or a recorded, reasoned revert of one)"
    - "Gate logs for baseline and for each task, written under the scratchpad and quoted in the SUMMARY"
    - ".planning/quick/260915-hsh-update-all-cargo-dependencies-to-latest-/260915-hsh-SUMMARY.md"
  key_links:
    - "Cargo.toml requirement <-> the version ACTUALLY resolved in Cargo.lock. A caret-requirement edit that resolves to the version already locked is a no-op that reads as a bump — this is precisely how quick task 260915-f4n's titled `Cargo.toml` change turned out to have changed nothing. Every claimed bump is proved from the lockfile, never from the manifest."
    - "`process_wrap::tokio::ChildWrapper` accessors <-> the spawn path in `src/executor/claude.rs` AND `terminate_group` / `tear_down_group` / `finish_teardown` / `await_clean_exit`. That trait object is the kill switch's ONLY route to the driven agent's process group; a silently-changed accessor or kill semantic orphans a running agent."
    - "measured max `rust_version` across the resolved graph <-> `Cargo.toml` `rust-version` <-> the toolchain version pinned in the `msrv` job of `.github/workflows/release.yml` <-> the ten prose MSRV sites quick task 260915-hae established"
---

<objective>
Advance the entire Cargo dependency graph to latest — the ~60 semver-compatible lockfile moves
plus the two genuine major bumps (`dirs` 6→7, `process-wrap` 9.1→10.0) that the release process
normally defers and that the user has explicitly authorised for this task.

Purpose: the repository's release process (CLAUDE.md, step 2) runs `cargo update` at every
milestone and *lists* semver-breaking direct deps as deferred. That deferral has accumulated.
This task pays it down in three independently revertable commits so a regression traced later
points at exactly one dependency.

Output: an updated `Cargo.lock`, two updated requirements in `Cargo.toml`, whatever call-site
fixups the majors demand, a re-measured MSRV floor, and a SUMMARY quoting measured gate numbers
rather than asserted ones.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@Cargo.toml
@src/executor/claude.rs
</context>

<environment>

## Scratchpad

Every log this plan writes goes under:

```
S=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/99121bc3-2602-491c-9020-a4abc0ec3173/scratchpad
mkdir -p "$S"
```

Nothing in `$S` is committed.

## The `rtk` output-filtering hazard — READ THIS BEFORE RUNNING ANY GATE

This environment installs an `rtk` hook that rewrites cargo invocations and FILTERS the output,
stripping (among others) cargo's `warning:` lines and the `test result:` summary lines. Two
consequences, both of which have already produced false green results in this repository:

1. Any check whose verdict depends on raw cargo output MUST be invoked as `rtk proxy cargo ...`.
2. `rtk proxy` protects only the wrapped command. A `grep`, `awk`, or `tee` placed AFTER it in a
   pipeline is filtered again, so a grep for a stripped token succeeds vacuously.

Therefore: **redirect to a file, never pipe.** Write the raw output to `$S/<name>.log` with a
plain `>` redirect plus `2>&1`, then `Read` that file. Do not put a pipe between `rtk proxy`
and the consumer of its output.

Shell note: the interactive shell here is fish. `cmd > file 2>&1` works in both fish and bash;
`$?` does not (fish spells it `$status`). Prefer reading the exit status from the tool's own
report over interpolating a status variable.

## The test-runner hazard

`cargo test` stops at the first failing test BINARY. `driver_reattach` fails (known, pre-existing)
and sorts before the `envelope_*` binaries, so a plain `cargo test` never executes roughly a third
of the suite and reports a stable-looking number no matter what changed. **Every suite number in
this plan comes from `cargo test --no-fail-fast`.** A run without that flag is not a measurement.

## Known-failing tests (NOT regressions)

The last recorded gate, from quick task 260915-hae, was **2002 passed / 4 failed / 15 ignored**
across 48 test binaries. Two of those four are the pre-existing `driver_reattach` pair (a spawn-
artifact race, tracked as a pending todo). One is a git-version-constants test in the
`envelope/policy.rs` unit module that fails *environmentally* — this machine's installed git
differs from the version the test pins — and is not a code fault.

That number is CONTEXT, not the baseline. Task 1 measures the real baseline.

</environment>

<decisions_inferred_for_audit>

The human is unavailable for this task. These calls were made from the artifacts and are flagged
here so a later reader can audit them rather than discover them:

- **I-01 — `process-wrap` goes LAST.** Its `ChildWrapper` trait object is the kill switch's only
  handle on the driven agent's process group, and the 10.0 release notes name three breaking
  changes to exactly that surface. Ordering it last means the escape hatch reverts the newest
  commit, leaving the other two bumps landed and untouched.
- **I-02 — the CLAUDE.md prohibition is scoped to the dependency notes, not to the MSRV row.**
  The constraint "do not edit CLAUDE.md" exists so the `notify` 8.x-not-9.0-rc guidance survives
  (9.0 is still a release candidate — that note is still correct). `CLAUDE.md:25` is also one of
  the ten MSRV sites quick task 260915-hae established. If and only if the measured floor MOVES,
  that one table cell is updated with it; the `notify` notes and every other line stay untouched.
  If the floor does not move, CLAUDE.md is not opened at all.
- **I-03 — a "bump" is proved from `Cargo.lock`, never from `Cargo.toml`.** Quick task 260915-f4n
  halted after discovering its titled manifest change had been a no-op. Every version claim in the
  SUMMARY is read out of the lockfile.
- **I-04 — build breakage caused by the plain `cargo update` is in scope for Task 1.** If a
  semver-compatible move breaks the build, fix the call site if it is a small fixup; if it is not,
  pin that single crate back with `cargo update -p <crate> --precise <old>` and record it. Do not
  abandon the whole update over one crate.

</decisions_inferred_for_audit>

<tasks>

<task type="tracer">
  <name>Task 1: Baseline, then advance the whole lockfile semver-compatibly</name>
  <files>Cargo.lock</files>
  <precondition>`rtk` is on PATH and `jq` is installed; the working tree is clean apart from the two untracked/modified `.planning/quick/` SUMMARY files noted in git status.</precondition>
  <action>
This is the tracer slice: it drives the full pipeline — baseline capture, dependency change,
three-gate verification, atomic commit — end-to-end on the lowest-risk change, so the method is
proved before either major bump rides on it.

Step 1, BASELINE, on unmodified HEAD, before touching anything. Confirm the tree is clean of
source edits, then run each gate with output redirected to the scratchpad:

- `rtk proxy cargo build > "$S/base-build.log" 2>&1`
- `rtk proxy cargo clippy -- -D warnings > "$S/base-clippy.log" 2>&1`
- `rtk proxy cargo test --no-fail-fast > "$S/base-test.log" 2>&1`

Read `$S/base-test.log` and extract the aggregate passed / failed / ignored counts and the NAME of
every failing test. Record that list — it is the comparison set for all three tasks. Note the
clippy gate is the lib-target one (`cargo clippy -- -D warnings`); `--all-targets` carries five
pre-existing lints in this repository and is deliberately not the gate.

Step 2, capture the pre-update lockfile identity so the diff is readable afterwards:
`cp Cargo.lock "$S/Cargo.lock.before"`.

Step 3, run `cargo update` with NO manifest edit. This moves roughly sixty locked crates. The
orchestrator's survey names, among others: tokio to 1.53.x, serde to 1.0.229, serde_json to
1.0.151, anyhow to 1.0.104, uuid to 1.26.x, futures to 0.3.34, chrono to 0.4.45, tempfile to
3.27.0, icu_properties to 2.3.0, notify to 8.2.0, regex to 1.13.x, tracing-subscriber to 0.3.23,
thiserror to 2.0.20, time to 0.3.55, darling to 0.24.x, pest to 2.9.x, clap_derive to 4.6.7.
Treat that as expected shape, not as a checklist to enforce — the resolver decides.

Step 4, guard the two crates that must NOT advance. `notify` must stay on 8.x and
`notify-debouncer-full` on 0.7.x, because their next majors are still release candidates. Neither
has a manifest edit here, so a caret requirement cannot reach them — but verify it rather than
assume it, and verify no release-candidate version reached the lockfile at all (see verify).

Step 5, run the three gates again into `$S/t1-build.log`, `$S/t1-clippy.log`, `$S/t1-test.log`.
Compare the failing-test NAME SET against the baseline set from step 1. A different count with the
same names is acceptable only if the difference is in ignored/passed; a NEW name is a regression.

Step 6, if a semver-compatible move broke the build or introduced a new failure, apply I-04: fix
the call site if it is a small fixup, otherwise pin that one crate back with
`cargo update -p &lt;crate&gt; --precise &lt;previous&gt;` and record the pin and its reason in the SUMMARY.

Step 7, commit `Cargo.lock` alone, atomically:
`build(quick-260915-hsh): advance the lockfile to latest semver-compatible versions`.
Do not stage `Cargo.toml` in this commit — the manifest is untouched by this task.
  </action>
  <verify>
    <automated>rtk proxy cargo build > "$S/t1-build.log" 2>&1 &amp;&amp; rtk proxy cargo clippy -- -D warnings > "$S/t1-clippy.log" 2>&1 &amp;&amp; rtk proxy cargo test --no-fail-fast > "$S/t1-test.log" 2>&1; grep -v '^#' Cargo.lock > "$S/lock-nocomment.txt"; grep -c 'rc\.' "$S/lock-nocomment.txt"; grep -n '^name = "notify"' -A1 "$S/lock-nocomment.txt"; grep -n '^name = "notify-debouncer-full"' -A1 "$S/lock-nocomment.txt"</automated>
    <human-check>Read `$S/t1-test.log` and `$S/base-test.log`. The set of failing test NAMES in the former must be a subset of the set in the latter.</human-check>
  </verify>
  <done>
`cargo build` exits 0. `cargo clippy -- -D warnings` exits 0. `cargo test --no-fail-fast` reports
no failing test name that is absent from the baseline set captured in step 1. The comment-stripped
lockfile contains zero release-candidate version strings. `notify` resolves within 8.x and
`notify-debouncer-full` within 0.7.x. `Cargo.lock` differs from `$S/Cargo.lock.before`. One commit
exists touching `Cargo.lock` and nothing else.
  </done>
  <reversibility rating="reversible">A lockfile-only commit reverts with `git revert`; the manifest constrains resolution either way.</reversibility>
</task>

<task type="auto">
  <name>Task 2: dirs 6 → 7</name>
  <files>Cargo.toml, Cargo.lock, src/state_reader/queue_md.rs, src/project_creator.rs, src/journal/redact.rs, src/main.rs, src/envelope/mod.rs, src/config.rs</files>
  <action>
Before editing anything, do the supply-chain check that T-hsh-01 in the threat model requires.
The `dirs` crate's former repository (github.com/dirs-dev/dirs-rs) was ARCHIVED in February 2025
and 7.0.0's documented source now points at codeberg.org/dirs/dirs-rs. A repository move is also
what a crate hijack looks like from the outside, so confirm publisher continuity on crates.io —
the owners of `dirs` at 7.0.0 must be the same owners that published 6.0.0. If they are not, HALT
this task, leave Task 1 committed, and report it; do not proceed on a judgement call.

Then change the requirement in `Cargo.toml` from the 6 line to `dirs = "7"` and run
`cargo update -p dirs` (or plain `cargo build`, which will re-resolve) so the lockfile actually
moves. Per I-03, read the resolved version back out of `Cargo.lock` and confirm it starts with
`7.` — do not report a bump on the strength of the manifest edit alone.

The call sites are narrow. Production code uses exactly three free functions across six files:
`dirs::home_dir()` in `src/state_reader/queue_md.rs`, `src/project_creator.rs` and
`src/journal/redact.rs`; `dirs::data_local_dir()` in `src/main.rs` and `src/envelope/mod.rs`;
`dirs::config_dir()` in `src/config.rs`. Test modules and the `envelope_*` integration tests
mention these names in doc comments and assertions too. The 7.0.0 API surface still exports the
same eighteen functions, so the expected outcome is that nothing needs changing and the compiler
is the oracle. If a signature did change — for instance a return type that is no longer
`Option<PathBuf>` — adapt each call site to the new shape while preserving the existing
None-handling semantics at that site. Several of those sites make a deliberate choice on the None
branch (`project_creator.rs` falls back to a literal tilde path; `redact.rs` returns early rather
than redacting nothing); preserve those choices exactly, do not "improve" them.

Pay particular attention to `src/envelope/mod.rs:215` and `src/config.rs:331`. The envelope's
blast-radius root is derived from `data_local_dir()`, and `tests/envelope_control_carrier.rs`
pins the behaviour of the guard's registry when `config_dir()` is None. A change in what those
functions return under an unusual environment moves a security boundary, so the `envelope_*`
binaries passing under `--no-fail-fast` is a load-bearing part of this task's gate, not a
formality.

Run the three gates into `$S/t2-build.log`, `$S/t2-clippy.log`, `$S/t2-test.log` and compare the
failing-name set against Task 1's baseline set.

Commit atomically: `build(quick-260915-hsh): bump dirs 6 -> 7`. Stage `Cargo.toml`, `Cargo.lock`
and any source files the migration actually required. If no source file changed, say so in the
commit body rather than leaving it to inference.

ESCAPE HATCH — this is a decision point, act on it rather than pushing through. If reaching 7.0.0
requires substantial redesign or an API-surface rewrite rather than straightforward call-site
fixups, revert the `Cargo.toml` requirement and the lockfile entry for `dirs` alone, leave Task 1
committed, and record `dirs` as needing a dedicated planning pass. Report which specific API
forced the call, with the file and line. The signal is scope, not effort: mechanical adaptation of
the seven listed call sites is in scope however tedious; anything that changes what those call
sites MEAN, or requires a new abstraction to hold the difference, is out.
  </action>
  <verify>
    <automated>grep -n '^dirs' Cargo.toml; grep -A1 '^name = "dirs"$' Cargo.lock; rtk proxy cargo build > "$S/t2-build.log" 2>&1 &amp;&amp; rtk proxy cargo clippy -- -D warnings > "$S/t2-clippy.log" 2>&1 &amp;&amp; rtk proxy cargo test --no-fail-fast > "$S/t2-test.log" 2>&1</automated>
    <human-check>Read `$S/t2-test.log`. Every `envelope_*` binary must have executed (the `--no-fail-fast` flag is what makes this true), and the failing-name set must be a subset of Task 1's baseline set.</human-check>
  </verify>
  <done>
Either (a) `Cargo.lock` shows `dirs` resolved at a `7.` version, the manifest requirement reads
`7`, all three gates pass with no new failing test name, and one commit carries the change — or
(b) the escape hatch fired, `Cargo.toml` and `Cargo.lock` carry no `dirs` change at all, Task 1's
commit is intact, and the SUMMARY names the specific API that forced the revert with file and line.
Publisher continuity on crates.io was checked and recorded either way.
  </done>
  <reversibility rating="reversible">One commit touching a manifest requirement and up to six call sites; `git revert` restores 6.x exactly.</reversibility>
</task>

<task type="auto">
  <name>Task 3: process-wrap 9.1 → 10.0, then re-measure the MSRV floor</name>
  <files>Cargo.toml, Cargo.lock, src/executor/claude.rs, README.md, CONTRIBUTING.md, CLAUDE.md, docs/DEVELOPMENT.md, docs/GETTING-STARTED.md, docs/TESTING.md, src/journal/redact.rs, .github/workflows/release.yml</files>
  <action>
PART A — the bump. Change the `process-wrap` requirement in `Cargo.toml` from `"9.1.0"` to
`"10.0.0"`, keeping `features = ["tokio1"]` (the crate is inert without it — the existing comment
above that line says so and must survive the edit). Re-resolve and, per I-03, read the version back
out of `Cargo.lock` to confirm it moved.

`process-wrap` has exactly ONE consumer: `src/executor/claude.rs`. The 10.0.0 release notes name
three breaking changes, and each maps onto a known line in that file:

- "Replace panicking accessors with fallible ones." The spawn path at roughly lines 601-612 calls
  `child.stdin()`, `child.stdout()` and `child.stderr()`, each followed by `.take()` and
  `.ok_or(SpawnError::PipeUnavailable { pipe: ... })`. If those accessors now return a `Result`,
  thread the error into the existing `SpawnError` vocabulary — the `PipeUnavailable` variant with
  its `pipe` label already exists for exactly this condition, so reuse it rather than inventing a
  new variant or unwrapping.
- "Make stored wrapper operations type-safe" and "make wrapper extension typed." The builder at
  roughly lines 480-572 uses `CommandWrap::with_new(&program, |cmd| ...)` then
  `wrap.wrap(ProcessGroup::leader())` and `wrap.wrap(KillOnDrop)`. Adapt to whatever the typed
  form now is.

Then check the teardown surface, because it is the part that matters. `Box<dyn ChildWrapper>` is
stored as the `child` field at roughly line 1043 and is consumed by `terminate_group` (line 1897,
`child.signal(SIGTERM)`), `tear_down_group` (1928), `finish_teardown` (1938, `child.start_kill()`)
and `await_clean_exit` (1957). `child.id()` at line 584 is how the pgid is obtained, because the
concrete group type is not reachable through the trait object — a comment at 578-583 says so
explicitly. Read those comments before editing; they record WHY the code is shaped as it is, and
several of them encode findings from phase 17's kill-switch gap closure (CR-01 through CR-06).
Preserve the SIGTERM-then-grace-then-SIGKILL ordering and the "an observed exit is not a reaped
group" discipline exactly. `SIGTERM` is a local `i32` constant kept specifically to avoid a `nix`
or `libc` dependency; if the new signal API demands a typed signal, prefer whatever `process-wrap`
itself re-exports over adding a new crate.

ESCAPE HATCH, and this is the task most likely to trigger it. If "make wrapper extension typed"
has removed the `dyn ChildWrapper` trait object — so that the child can no longer be stored as
`Box<dyn ChildWrapper>` in a struct field and passed to four free functions — that is a redesign
of the executor's ownership model, not a call-site fixup. In that case: revert the `Cargo.toml`
requirement and the `Cargo.lock` entry for `process-wrap` alone, leave Tasks 1 and 2 committed,
and report `process-wrap` as needing a dedicated planning pass, naming the removed item. Same
scope-not-effort test as Task 2. Do NOT leave a half-migrated `claude.rs` behind — if the hatch
fires, `git checkout -- src/executor/claude.rs` so the file matches the last commit exactly.

Gate Part A into `$S/t3a-build.log`, `$S/t3a-clippy.log`, `$S/t3a-test.log`. Give the
`driver_*` binaries specific attention: they exercise the spawn, liveness and stop paths that this
crate owns. Their failing-name set must be a subset of Task 1's baseline set — the known
`driver_reattach` pair may still fail, a new `driver_*` name may not.

Commit: `build(quick-260915-hsh): bump process-wrap 9.1 -> 10.0`.

PART B — re-measure the MSRV floor. Do this AFTER Part A has settled, whether it landed or was
reverted, so the measurement describes the tree that actually exists.

`Cargo.toml`'s `rust-version` carries a FLOOR PROVENANCE comment stating that the value is the
maximum `rust_version` across the resolved graph and that it drifts under an ordinary `cargo
update` with no manifest edit. This task performed exactly that event. Measure:

```
cargo metadata --format-version 1 --locked \
  | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1
```

Write the measured value to `$S/msrv-measured.txt`.

If the measured floor is unchanged: change nothing. Record the measured value in the SUMMARY as
evidence that it was checked and held, with the command that produced it. This is a real result,
not a non-event — the provenance comment exists because the value moved silently once before.

If the measured floor ROSE: update `rust-version` in `Cargo.toml`, refresh the crate names in the
FLOOR PROVENANCE comment so it still names the dependencies that actually set the value, and bring
the ten MSRV sites quick task 260915-hae established to the new value. Those sites are:
`README.md` lines 61 and 186; `CONTRIBUTING.md` line 23; `CLAUDE.md` line 25 (the Rust row of the
tech-stack table — and ONLY that cell, per I-02); `docs/TESTING.md` line 14;
`docs/GETTING-STARTED.md` lines 13, 150 and 155; `docs/DEVELOPMENT.md` lines 10-11;
`src/journal/redact.rs` line 218; and the toolchain version in the `msrv` job of
`.github/workflows/release.yml` line 19. Verify the new floor by measurement, not assertion:
`cargo +<measured> check --all-targets --locked` must exit 0, and the version one minor below it
must not. Install the toolchain with `rustup toolchain install` if it is absent.

Under no circumstances edit the `notify` guidance in `CLAUDE.md` — 9.0 is still a release
candidate, so that note is correct as written. Do not change the crate's own `version = "1.6.0"`;
this is not a release. Do not touch the "Recent decisions" or "Pending Todos" sections of
`.planning/STATE.md`.

If Part B changes files, it is a SEPARATE commit:
`build(quick-260915-hsh): raise the measured MSRV floor to <measured>`.

PART C — record the transitive holds. `generic-array` 0.14.7 and `unicode-width` 0.2.0 are pinned
below their latest releases by upstream transitive requirements, not by anything in this manifest.
Confirm with `cargo tree -i generic-array` and `cargo tree -i unicode-width`, and note in the
SUMMARY which dependents hold them. No action — this is disclosure so the next person running
`cargo update` does not re-investigate it.
  </action>
  <verify>
    <automated>grep -n 'process-wrap' Cargo.toml; grep -A1 '^name = "process-wrap"$' Cargo.lock; rtk proxy cargo build > "$S/t3-build.log" 2>&1 &amp;&amp; rtk proxy cargo clippy -- -D warnings > "$S/t3-clippy.log" 2>&1 &amp;&amp; rtk proxy cargo test --no-fail-fast > "$S/t3-test.log" 2>&1; cargo metadata --format-version 1 --locked | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1 > "$S/msrv-measured.txt"; cat "$S/msrv-measured.txt"; grep -n '^rust-version' Cargo.toml</automated>
    <human-check>Read `$S/t3-test.log`. Confirm every `driver_*` binary executed and that no `driver_*` test name fails which was passing in `$S/base-test.log`.</human-check>
  </verify>
  <done>
Part A: either `Cargo.lock` shows `process-wrap` at a `10.` version with all three gates green and
no new failing test name, or the escape hatch fired and `src/executor/claude.rs`, `Cargo.toml` and
`Cargo.lock` carry no `process-wrap` change while Tasks 1 and 2 remain committed.

Part B: `$S/msrv-measured.txt` holds a measured value. `Cargo.toml`'s `rust-version` equals it. If
it rose, every one of the listed MSRV sites states the new value, `cargo +<measured> check
--all-targets --locked` exits 0, and a second commit carries the change. If it held, no file moved
and the SUMMARY records the measured value with its command.

Part C: the dependents holding `generic-array` and `unicode-width` are named in the SUMMARY.

Across all three tasks: `git log --oneline` shows one commit per dependency change, each
revertable without the others.
  </done>
  <reversibility rating="costly">`process-wrap` owns the kill switch's only route to the driven agent's process group. A silently-changed kill semantic is not caught by a compile and would surface as an orphaned agent process at runtime, so the revert is cheap but the detection is not — which is why the `driver_*` binaries are a named part of the gate rather than folded into the aggregate count.</reversibility>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| crates.io → local build | Third-party code enters the tree and EXECUTES at compile time via `build.rs` scripts and proc macros, before any test runs. `cargo update` crosses this boundary roughly sixty times in Task 1 alone. |
| `process-wrap` → OS process group | This crate is the only route from the supervisor to the driven agent's process group (signal, kill, reap). Its semantics are a security property of the kill switch, not an implementation detail. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-hsh-SC | Tampering | `cargo update` across the whole graph | high | mitigate | No NEW package NAME is introduced deliberately by this task — only version advances of already-vetted deps and their transitives. Task 1 keeps `$S/Cargo.lock.before`; diff it against the post-update lockfile and apply the package-legitimacy check to any package NAME that appears for the first time (not merely a changed version). A newly-appearing name is a blocking finding, not a routine diff line. |
| T-hsh-01 | Spoofing | `dirs` 7.0.0 | medium | mitigate | The `dirs` source repository moved: github.com/dirs-dev/dirs-rs was archived 2025-02-18 and 7.0.0 documents codeberg.org/dirs/dirs-rs. A repository move and a crate hijack are externally indistinguishable. Task 2 confirms crates.io publisher continuity between 6.0.0 and 7.0.0 BEFORE editing the requirement, and halts on a mismatch. |
| T-hsh-02 | Elevation of Privilege | `process-wrap` 10.0.0 → `src/executor/claude.rs` teardown path | high | mitigate | 10.0.0's breaking changes land directly on the accessors and wrapper types the kill switch uses. A changed signal or reap semantic orphans a driven agent process group — the exact class phase 17's CR-01..CR-06 closed. Task 3 requires the `driver_*` binaries to execute under `--no-fail-fast` with no new failing name, and requires the SIGTERM→grace→SIGKILL ordering and the "observed exit is not a reaped group" discipline to be preserved at the call sites rather than assumed. |
| T-hsh-03 | Denial of Service | MSRV floor drift | low | accept | A raised floor makes the crate unbuildable for users on an older toolchain. Accepted because the alternative — leaving a false floor declared — is what quick task 260915-f4n already measured and refuted. The floor is re-measured and declared honestly; the `msrv` CI job enforces it before publish. |
| T-hsh-04 | Tampering | release-candidate versions entering the lockfile | medium | mitigate | `notify` 9.0.0-rc.5 and `notify-debouncer-full` 0.8.0-rc.2 are pre-release and must not be resolved. Task 1 asserts zero release-candidate version strings in the comment-stripped lockfile and pins `notify` within 8.x and `notify-debouncer-full` within 0.7.x. |
</threat_model>

<verification>

Run after all three tasks:

1. `git log --oneline -5` — one commit per dependency change, each with the
   `build(quick-260915-hsh):` prefix (plus an optional MSRV commit).
2. `git status` — no unstaged source changes, no half-migrated file left behind.
3. `rtk proxy cargo build > "$S/final-build.log" 2>&1` — exit 0.
4. `rtk proxy cargo clippy -- -D warnings > "$S/final-clippy.log" 2>&1` — exit 0.
5. `rtk proxy cargo test --no-fail-fast > "$S/final-test.log" 2>&1` — read the log; the failing
   test NAME set must be a subset of `$S/base-test.log`'s.
6. `cargo +<measured-floor> check --all-targets --locked` — exit 0 against the declared floor.
7. Each version claim written into the SUMMARY is quoted from `Cargo.lock`, not from `Cargo.toml`.

</verification>

<success_criteria>

- The lockfile is at latest semver-compatible across the graph.
- `dirs` and `process-wrap` are each either at latest major, or reverted with a named blocking API
  and a recommendation for a dedicated planning pass. A partial migration of either is a failure.
- No new test failure beyond the baseline captured in this session on unmodified HEAD.
- `rust-version` equals the post-update measured maximum across the resolved graph.
- `notify` on 8.x, `notify-debouncer-full` on 0.7.x, no release candidate in the lockfile.
- `CLAUDE.md`'s `notify` guidance unchanged; crate `version` still `1.6.0`; `.planning/STATE.md`'s
  "Recent decisions" and "Pending Todos" sections unchanged.
- Every commit reverts independently.

</success_criteria>

<output>
Create `.planning/quick/260915-hsh-update-all-cargo-dependencies-to-latest-/260915-hsh-SUMMARY.md` when done.

The SUMMARY must carry, as MEASURED values rather than assertions: the baseline and final
passed/failed/ignored counts with the failing test names for each; the before/after version of
every crate whose version this task claims to have changed, read from `Cargo.lock`; the measured
MSRV floor and the command that produced it; the escape-hatch outcome for each major bump; the
crates.io publisher-continuity result for `dirs`; the dependents holding `generic-array` and
`unicode-width`; and any crate pinned back under I-04 with its reason.
</output>
