---
phase: quick-260917-lkg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - tests/envelope_tracer.rs
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
autonomous: true
requirements:
  - DEFER-21-ENVELOPE-TRACER-ETXTBSY

estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Under the contended reproducer (8 concurrent instances of the compiled envelope_tracer binary x 25 rounds = 200 runs), which produced 10 ETXTBSY failures on the untouched tree, zero runs fail with `Text file busy` (C-1)."
    - "A spawn failure that is NOT ETXTBSY still fails immediately, under the unchanged `the generated stub is executable` wording, so a genuinely non-executable stub (PermissionDenied / ENOEXEC) is not masked (C-1)."
    - "An ETXTBSY that never clears inside the bounded window fails LOUDLY, naming the stub path, the limit and the attempt count, and saying the exec never happened — it is never skipped, never returned early, and never accepted as the refusal the test asserts (C-1)."
    - "Every assertion the file carried before this change is present after it, unweakened; no `#[ignore]`, no thread-count flag, no fixed pre-assertion sleep (C-1)."
    - "The module header records why the EXEC and not the copy flaked, whose descriptor pins the inode, that ETXTBSY is per-inode, and why expiry must be loud (C-2)."
    - "Phase 21's deferred-items.md closes the envelope_tracer row append-only, states the two-binary item is now fully closed, and points back at the 260917-k6y section for the half it did not touch (C-3)."
  artifacts:
    - tests/envelope_tracer.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
  key_links:
    - "`run_stub` (tests/envelope_tracer.rs:177-193) is the SINGLE spawn site both legs of `a_relocated_copy_of_the_stub_refuses_instead_of_acting` go through — the control leg at line 283 and the relocated leg at line 296. Fixing it there fixes both call sites and the other tests' future use of it, with no signature change and no caller edited."
    - "The retry's error discrimination: `ErrorKind::ExecutableFileBusy` retries, EVERYTHING else fails on the spot. If that discrimination is lost the test stops being able to tell a busy inode from a broken stub."
    - "The deferred-items closure is the only place a future reader learns that `std::fs::copy` was exonerated by strace. If the section does not say so, the next investigator re-derives it."
---

<objective>
Make `tests/envelope_tracer.rs::a_relocated_copy_of_the_stub_refuses_instead_of_acting`
deterministic against the ETXTBSY exec race, and close the surviving half of phase 21's
two-binary flake item.

Purpose: the flake is the last of the three failures `./scripts/pre-tag-check.sh` gate 4 carried;
quick `260917-k6y` closed the `driver_reattach` pair, this closes the `envelope_tracer` one. The
fix is a SYNCHRONISATION fix at the exec site, not a tolerance and not a weakening — the test
exists to prove a relocated stub REFUSES rather than acts, and an exec that never happened proves
nothing about refusal.

Output: a bounded, ETXTBSY-only exec retry inside `run_stub`; an extended module header that
records the mechanism; an append-only closure section in phase 21's `deferred-items.md`.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@tests/envelope_tracer.rs
@src/envelope/hooks.rs
@.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
</context>

<findings_do_not_re_derive>
This investigation is COMPLETE and MEASURED. Plan execution consumes it; it does not repeat it.

**The flake.** Intermittent failure at `tests/envelope_tracer.rs:185:10` —
`the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`.
Line 185 is the `.expect(...)` on `.spawn()` inside `fn run_stub` (lines 177-193).

**Measured baseline, this machine, untouched tree:**

| Condition | Runs | ETXTBSY failures |
|---|---|---|
| isolated back-to-back `envelope_tracer` binary | 70 | **0** |
| full-suite `rtk proxy cargo test --no-fail-fast` | 6 | **0** (each 2120 passed / 1 failed / 15 ignored; the 1 is the unrelated git-version-constants pin) |
| **8 concurrent instances x 25 rounds** | **200** | **10 (5%)**, every one this test, this line, this error, in ~12s |

**Mechanism, CONFIRMED by strace, not inferred.** `strace -f -e trace=execve` caught the failing
syscall three times. Every one is the **SANCTIONED** stub, never the relocated copy:
`execve("/tmp/.tmpXXXXXX/envelope/provenance/hooks/pre-push", ...) = -1 ETXTBSY`. So
**`std::fs::copy` at line 294 is NOT the cause** — the copy's destination never appears in a
failing `execve`. What fails is the control leg, `run_stub(&fx, &sanctioned, ...)` at line 283.

The writable descriptor comes from `src/envelope/hooks.rs::write_stub` (lines 126-149:
`NamedTempFile::new_in` -> `write_all` -> `set_permissions` -> `persist`). ETXTBSY is a
**per-inode** condition (`i_writecount`). `envelope_tracer` runs six `#[test]` fns on six libtest
threads that spawn subprocesses constantly. When one thread forks while another thread's
`NamedTempFile` write descriptor is open, the forked child **inherits** it; `O_CLOEXEC` closes it
at the child's own `execve`, but not before — so between the child's `fork` and its `execve` the
child pins the stub's inode as open-for-write, and any `execve` of that stub in that window is
ETXTBSY.

**Three eliminations were considered and each is provably useless here. Do NOT attempt them:**

1. **Closing/syncing the `File` before the rename in `write_stub`.** `rename(2)` does not change
   the inode, and the inherited descriptor pins the INODE — which was already exposed during the
   pre-rename `write_all`/`set_permissions` window. The hazard is not "the final path had a
   writer", it is "this inode had a writer while a sibling thread forked".
2. **`hard_link` instead of `fs::copy`.** The copy is not the failing exec (strace proves it), and
   a hard link SHARES the sanctioned inode, making it worse.
3. **A process-wide RwLock making writes exclusive against spawns.** Would require wrapping every
   `Command::spawn` in the binary and in `tests/common/mod.rs`; any site missed silently
   reinstates the flake while LOOKING deterministic. Rejected as a fix that overclaims.

The descriptor is **not ours to close** — by exec time it lives in a short-lived forked child of a
sibling test thread and clears on its own within microseconds. The only correct handle on it at
the exec site is to wait it out.
</findings_do_not_re_derive>

<!-- planner-discipline-allow: #[ignore] -->
<!-- planner-discipline-allow: --test-threads -->

<tasks>

<task type="tracer">
  <name>Task 1: a bounded, ETXTBSY-only exec retry inside run_stub (C-1)</name>
  <files>tests/envelope_tracer.rs</files>
  <precondition>`jq` is on PATH and `cargo test --test envelope_tracer --no-run` builds, since the contended reproducer locates the compiled binary through `--message-format=json`. Both verified present at planning time.</precondition>
  <behavior>
    - Contended reproducer, 200 runs: 0 failures (was 10/200 before the change).
    - The ETXTBSY branch is OBSERVABLE: retries emitted across the 200 contended runs are countable, and a run that retried still reports its 6 tests passed.
    - A non-ETXTBSY spawn error is not retried and not reworded — the existing `the generated stub is executable` message still carries it.
    - Deadline expiry panics naming the stub path, the limit and the attempt count.
    - Anti-vacuity: post-fix per-round wall times sit in the same band as the pre-fix baseline, so a green run cannot be a run whose fixtures silently skipped.
  </behavior>
  <read_first>
    tests/envelope_tracer.rs lines 170-193 (`run_stub`, the only spawn site) and lines 269-310
    (`a_relocated_copy_of_the_stub_refuses_instead_of_acting`, its two call sites).
    tests/driver_reattach.rs lines 350-452 — the house `*_within` idiom this wait must match:
    an `Instant::now() + limit` deadline, `Duration::from_millis(25)` between polls,
    `Duration::from_secs(30)` passed INLINE at the call site rather than as a new named constant,
    and a failure message that names exactly what never arrived.
  </read_first>
  <action>
Step 1, BEFORE editing a byte — reconfirm red on this tree. Write the contended reproducer from
the `<verification>` section below into the session scratchpad (NOT into the repository; repo file
scope is exactly the two files in `files_modified`) and run it against the untouched tree. Record
FAILED_RUNS, the ETXTBSY log count, and the per-round wall-time band. Expect roughly 10 of 200.
If this particular 200-run sample comes back 0 red, record that honestly and PROCEED — the 10/200
baseline is already on file and one clean sample at a 5% rate does not refute it; do not treat a
clean pre-run as evidence there is nothing to fix.

Step 2 — the retry. Keep `fn run_stub(fx: &Fixture, stub: &Path, ref_line: &str) -> Output`'s
signature and both call sites exactly as they are. Build the `Command` once before the loop (it is
the same builder chain that exists today: `current_dir(&fx.work)`, `env(ENVELOPE_ROOT_ENV,
&fx.envelope_root)`, the three piped stdio) and bind it mutably — `Command::spawn` takes
`&mut self`, so one builder can be spawned repeatedly and no closure or rebuild is needed.

Loop with an `Instant::now() + Duration::from_secs(30)` deadline, matching driver_reattach's
call-site convention; do not introduce a named constant for a single site. Count attempts.
Three arms, and the discrimination between them is the load-bearing part:
  - `Ok(child)` — break out with the child and carry on into the existing stdin write and
    `wait_with_output`, which are untouched.
  - `Err(e)` where `e.kind() == std::io::ErrorKind::ExecutableFileBusy` — if the deadline has
    passed, `panic!`; otherwise emit one `eprintln!` naming the stub path and the attempt number,
    sleep `Duration::from_millis(25)`, and retry.
  - any other `Err(e)` — fail immediately, preserving the existing message text verbatim so a
    genuinely non-executable stub still fails for its own reason. A bad mode yields
    `PermissionDenied`, a bad shebang `ENOEXEC`; neither is ETXTBSY, so nothing is masked.

Match on the `ErrorKind` variant, not on `raw_os_error() == Some(26)`.
`ErrorKind::ExecutableFileBusy` is stable since Rust 1.83 and `Cargo.toml` declares
`rust-version = "1.88"`, so the variant is available at this crate's MSRV floor. Confirm it
compiles; if it somehow does not, STOP and report rather than silently downgrading to the raw
errno — the downgrade would be a real finding about the toolchain, not a detail.

The expiry panic must name the stub path, the elapsed limit and the attempt count, and must say
plainly that the exec never happened so nothing about refusal was proven. It is a loud failure:
it never returns a synthesised `Output`, never skips, and never lets a busy inode stand in for
the refusal the caller is about to assert on.

Add `use std::time::{Duration, Instant};` to the imports; `std::io::Write` is already there.

The `eprintln!` on each retry is an addition beyond the literal change description, taken so the
ETXTBSY branch is COUNTABLE rather than merely absent: libtest captures it by default, and the
reproducer runs the binary with `--nocapture` so the lines land in the per-run logs. It is the
difference between "the flake did not recur" and "the branch fired N times and every run still
passed". Mark it in the SUMMARY as an inferred decision for audit.

Prohibited, non-negotiable: no attribute that skips the test, no thread-count flag, no bare
pre-assertion sleep (a 25ms sleep INSIDE a poll loop is the house style; a fixed sleep before an
assertion is what this project's test headers forbid), no assertion deleted or loosened, and
ETXTBSY never accepted as the refusal.

Step 3 — re-run the reproducer and record FAILED_RUNS, the retry-line count, the 6-passed count
and the wall-time band.
  </action>
  <verify>
    <automated>bash "$SCRATCH/etxtbsy-repro.sh" "$SCRATCH/after" | tail -6   # FAILED_RUNS=0 of 200, ETXTBSY_LOGS=0, VACUITY_CHECK=200</automated>
    <automated>rtk proxy cargo test --test envelope_tracer -- --nocapture 2>&1 | tail -5   # 6 passed</automated>
    <automated>grep -vE '^\s*//' tests/envelope_tracer.rs | grep -c 'ExecutableFileBusy'   # >= 1, counted on non-comment lines only</automated>
    <automated>grep -c 'the generated stub is executable' tests/envelope_tracer.rs   # >= 1, the original wording survives</automated>
    <automated>grep -vE '^\s*//' tests/envelope_tracer.rs | grep -cF '#[ignore]'   # 0, comment-stripped so header prose cannot self-invalidate</automated>
    <automated>grep -vE '^\s*//' tests/envelope_tracer.rs | grep -c 'test-threads'   # 0, comment-stripped</automated>
    <automated>git diff -U0 tests/envelope_tracer.rs > "$SCRATCH/t1.diff"; echo "git exit=$?"; grep -c '^-.*assert' "$SCRATCH/t1.diff"   # git exit=0 and count 0 — no assertion removed</automated>
    <automated>rtk proxy cargo clippy -- -D warnings</automated>
  </verify>
  <done>200 contended runs green where 10 were red; the retry count across those runs is recorded (a count of 0 is reported as "fix unproven in effect", not as success); non-ETXTBSY errors still fail under the original wording; expiry panics loudly; no assertion removed; clippy clean.</done>
</task>

<task type="auto">
  <name>Task 2: extend the module header with the mechanism (C-2)</name>
  <files>tests/envelope_tracer.rs</files>
  <read_first>
    tests/envelope_tracer.rs lines 1-14 — the existing header, for register.
    tests/driver_reattach.rs lines 340-373 — the voice quick 260917-k6y established for exactly
    this kind of closed-case note: it explains WHY the old synchronisation point was wrong, states
    what a reader would otherwise wrongly conclude, and justifies the shape of the wait.
  </read_first>
  <action>
Extend the existing `// ===` header block. Match the surrounding documentation voice: this file's
headers explain WHY, name the decision ids, and state what a reader would otherwise wrongly
conclude. Do not write a changelog entry, do not date-stamp it as a release note, and do not
rewrite the SAFE-01/D-31 paragraph that is already there — this is an addition below it.

Record, briefly and in this order:
  - the EXEC flaked, not the copy, and strace named the SANCTIONED stub: the relocated copy's path
    never appeared in a failing `execve`, so the obvious suspect (`std::fs::copy` immediately
    before the second exec) is exonerated by measurement rather than by argument;
  - the writer is an inherited `NamedTempFile` descriptor living in a forked child of a SIBLING
    test thread — cite `src/envelope/hooks.rs::write_stub` — closed by `O_CLOEXEC` at that child's
    own `execve`, but not before;
  - ETXTBSY is a per-INODE condition, which is why closing the file before the rename would not
    have helped: `rename(2)` does not change the inode and the exposure window predates the
    rename;
  - the wait is ETXTBSY-only and bounded, and expiry FAILS rather than skips, because this test
    exists to prove the relocated stub refuses and an exec that never happened proves nothing
    about refusal.

Keep it proportionate — the mechanism and the four claims above, not a transcript of the
investigation. The measurements and the three rejected eliminations belong in deferred-items.md
(Task 3), and the header should point there rather than duplicate them.
  </action>
  <verify>
    <automated>rtk proxy cargo test --test envelope_tracer 2>&1 | tail -3   # 6 passed; the header is a comment, this proves the file still compiles</automated>
    <automated>grep -c 'write_stub' tests/envelope_tracer.rs   # >= 1, the descriptor's origin is cited</automated>
    <automated>grep -ci 'inode' tests/envelope_tracer.rs   # >= 1, the per-inode fact is recorded</automated>
    <automated>git diff -U0 tests/envelope_tracer.rs > "$SCRATCH/t2.diff"; echo "git exit=$?"; grep -c '^-.*SAFE-01' "$SCRATCH/t2.diff"   # git exit=0 and count 0 — the existing header paragraph is added to, not replaced</automated>
  </verify>
  <done>The header carries all four claims in the file's own voice, cites `src/envelope/hooks.rs::write_stub`, and the pre-existing SAFE-01/D-31 paragraph is intact.</done>
</task>

<task type="auto">
  <name>Task 3: close the envelope_tracer row in phase 21's deferred-items.md, append-only (C-3)</name>
  <files>.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md</files>
  <read_first>
    deferred-items.md lines 1-45 — the two-binary table (the `tests/envelope_tracer.rs` row at
    line 15) and the 260917-k6y pointer at lines 18-35 that closed only the other row.
    deferred-items.md lines 2310-2429 — the `# QUICK 260917-k6y — appended 2026-09-17, append-only`
    section. That is the exact convention to follow: a horizontal rule, a `# QUICK <id>` banner, a
    dated `##` canonical entry, an italic note stating that no line above was edited or deleted and
    that corrections quote the earlier text verbatim, then mechanism / what changed / measurements /
    what this does NOT close.
  </read_first>
  <action>
Append a new dated section at the END of the file, after the 260917-k6y section, following that
section's shape exactly. Delete nothing. Rewrite no heading. Where a sentence above is superseded,
quote it VERBATIM and say what changed.

Two pointers must be placed above (as `>` blockquote insertions in the k6y style, quoting what
they supersede, not editing it):
  - at the two-binary table near line 15-35, noting that the `tests/envelope_tracer.rs` row is now
    RESOLVED too and quoting verbatim the k6y pointer's sentence *"The `tests/envelope_tracer.rs`
    row — `a_relocated_copy_of_the_stub_refuses_instead_of_acting`, the `Text file busy`
    write-then-exec race — **remains OPEN, unchanged, and untouched**"*, stating that it no longer
    holds as of this task and that the two-binary item is now FULLY closed;
  - at the k6y canonical section's `### What this does NOT close`, against its verbatim sentence
    *"The `tests/envelope_tracer.rs` half of the original two-binary item stays OPEN."* — pointing
    at the new section below. That sentence was TRUE when written and is left as written.

The new canonical section records:
  - **the mechanism**: the exec, not the copy; strace naming the sanctioned stub three times; the
    inherited `NamedTempFile` descriptor from `src/envelope/hooks.rs::write_stub` in a forked child
    of a sibling libtest thread; ETXTBSY as a per-inode (`i_writecount`) condition;
  - **what changed**: `tests/envelope_tracer.rs` only — a bounded ETXTBSY-only retry at the single
    spawn site, signature and call sites unchanged, layered as a wait BEFORE the assertions rather
    than as a change to any of them; nothing weakened, nothing skipped, expiry loud;
  - **the measurements**, as a table: 0 of 70 isolated, 0 of 6 full-suite, **10 of 200 contended**
    before; the post-fix contended re-run, the retry-line count and the wall-time band after; the
    full-suite runs with their per-run passed/failed/ignored counts and failing-test names;
  - **the three rejected eliminations** and the reason each fails — close-before-rename (rename
    does not change the inode; the exposure window predates it), `hard_link` instead of `fs::copy`
    (the copy is not the failing exec, and a hard link shares the sanctioned inode), a process-wide
    write/spawn RwLock (needs every `Command::spawn` in the binary and in `tests/common/mod.rs`
    wrapped; one missed site silently reinstates the flake while looking deterministic);
  - **what this does NOT close**: `src/envelope/hooks.rs` is untouched and no claim is made about
    the descriptor's lifetime there; the eliminations above are reasoned, not measured, and are
    recorded as reasoning; the git-version-constants failure is environmental and out of scope;
  - **the two-binary item is now FULLY closed**, with an explicit pointer back at the
    `# QUICK 260917-k6y` section for the `driver_reattach` half this task did not touch.

`src/`, `Cargo.toml`, `.github/`, the version and any tag stay untouched — v1.7.1 is a separate
step and is explicitly out of scope for this task.
  </action>
  <verify>
    <automated>git diff --numstat .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # deletions column MUST be 0 — append-only</automated>
    <automated>grep -c '260917-lkg' .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # >= 3: the banner, the table pointer, the k6y pointer</automated>
    <automated>grep -c '10 of 200\|10/200' .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md   # >= 1, the contended baseline is on the record</automated>
    <automated>git diff --numstat -- src/ Cargo.toml .github/ > "$SCRATCH/t3.numstat"; echo "git exit=$?"; wc -l < "$SCRATCH/t3.numstat"   # git exit=0 and count 0 — nothing outside the two in-scope files moved</automated>
  </verify>
  <done>A new append-only `# QUICK 260917-lkg` section closes the envelope_tracer row with mechanism, measurements and the three rejected eliminations; two verbatim-quoting pointers sit above; `git diff --numstat` shows 0 deletions on the file; src/, Cargo.toml, .github/ untouched.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| test process -> generated hook stub (`execve`) | The only boundary this change touches. Both sides are test-owned, inside a `TempDir`. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-lkg-01 | Tampering | `run_stub` retry loop, tests/envelope_tracer.rs | medium | mitigate | The retry is ETXTBSY-ONLY. A tampered or non-executable stub surfaces as `PermissionDenied`/`ENOEXEC` and fails immediately under the unchanged wording, so the loop cannot become a swallow-all that hides a broken stub. Pinned by the error-discrimination arm in Task 1. |
| T-lkg-02 | Repudiation | the D-10 provenance proof | high | mitigate | ETXTBSY is never accepted as the refusal. Deadline expiry panics naming the path, limit and attempt count and stating the exec never happened — the test cannot report a provenance refusal it did not observe. |
| T-lkg-03 | Denial of Service | the 30s bounded wait | low | accept | A permanently pinned inode costs 30s of wall time on a genuine failure and nothing on a passing run (the observed pin clears in microseconds). Accepted, matching driver_reattach's existing `Duration::from_secs(30)` call sites. |
| T-lkg-SC | Tampering | npm/pip/cargo installs | n/a | n/a | No package is installed or added by this plan. `Cargo.toml` is explicitly out of scope, so the package-legitimacy gate has no subject here. |
</threat_model>

<verification>

## The contended reproducer

Write this to the session scratchpad — NOT into the repository, whose file scope for this task is
exactly the two files in `files_modified`. Run it once against the untouched tree (Task 1 step 1)
and once after the change (Task 1 step 3), into two different output directories.

```bash
#!/usr/bin/env bash
# 8 concurrent instances of the compiled envelope_tracer binary x 25 rounds = 200 runs.
# Produced 10 failures on the untouched tree; runs in ~12s.
set -u
REPO=/home/blk/projects/rust/gsd-meta-manager
OUT=${1:?usage: etxtbsy-repro.sh <output-dir>}
mkdir -p "$OUT"
cd "$REPO" || exit 1

cargo test --test envelope_tracer --no-run --quiet || exit 1
BIN=$(cargo test --test envelope_tracer --no-run --message-format=json 2>/dev/null \
      | jq -r 'select(.executable != null and .target.name == "envelope_tracer") | .executable' \
      | tail -1)
[ -x "$BIN" ] || { echo "no envelope_tracer binary found"; exit 1; }
echo "binary: $BIN"

fails=0
for round in $(seq 1 25); do
  pids=""
  start=$(date +%s%N)
  for i in $(seq 1 8); do
    "$BIN" --nocapture > "$OUT/r${round}-i${i}.log" 2>&1 &
    pids="$pids $!"
  done
  for p in $pids; do wait "$p" || fails=$((fails + 1)); done
  end=$(date +%s%N)
  echo "round $round  wall $(( (end - start) / 1000000 ))ms"
done

echo "FAILED_RUNS=$fails of 200"
echo "ETXTBSY_LOGS=$(grep -l 'Text file busy' "$OUT"/*.log 2>/dev/null | wc -l)"
echo "RETRY_LINES=$(grep -h 'ETXTBSY' "$OUT"/*.log 2>/dev/null | wc -l)"
echo "VACUITY_CHECK=$(grep -l '6 passed' "$OUT"/*.log 2>/dev/null | wc -l) of 200 runs report 6 passed"
```

Run it as `bash "$SCRATCH/etxtbsy-repro.sh" "$SCRATCH/before"`. The inner `cargo` calls are NOT
rewritten by the rtk hook (the hook rewrites the tool-level command only), so the
`--message-format=json` output arrives unfiltered — which is what the binary lookup depends on.

**Anti-vacuity.** `fixture()` returns `None` on any git failure and the affected tests then return
early, so a run can report 6 passed having proved nothing. Two controls: `VACUITY_CHECK` must be
200, and the post-fix per-round wall times must sit in the same band as the pre-fix baseline. A
suspiciously fast green round is reportable as a failure regardless of its exit code — the
convention `260917-k6y` established.

**The retry count is evidence, and a zero is reported, not hidden.** `RETRY_LINES` is the count of
ETXTBSY retries the fix absorbed. Pre-fix contention hit that condition 10 times in 200 runs, so a
post-fix count of 0 alongside 0 failures means the sample did not reproduce the race at all — that
is "fix unproven in effect", and the SUMMARY must say so rather than claim a green run as proof.

## Full-suite gates

**At least 8 runs** of `rtk proxy cargo test --no-fail-fast`, each reporting passed / failed /
ignored and the name of every failing test.

- NEVER plain `cargo test` — it fail-fasts at the first failing binary and never reaches any
  `envelope_*` suite, reporting a stale count regardless of what changed.
- NEVER pipe rtk output through `grep` for `test result:` lines — rtk's filtering strips them and a
  filtered run lies about the counts. Redirect rtk's output to a file and read the file instead.

**The ONLY acceptable failure in any run** is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— environmental: local git is 2.53.0 while the re-derived constant records 2.55.0. It passes on
CI. **It must not be touched.** Any other failure in any of the 8 runs is a real result and is
reported, not absorbed.

Expected steady state per run: 2120 passed / 1 failed / 15 ignored.

## Remaining gates

- `./scripts/pre-tag-check.sh` — reported gate by gate. Gate 4 is expected to fail on exactly ONE
  test (the version witness), down from the two it carried before this task.
- `rtk proxy cargo clippy -- -D warnings` — clean.
- `git diff --numstat` shows exactly two files: `tests/envelope_tracer.rs` and phase 21's
  `deferred-items.md`, the latter with a deletions count of 0.

</verification>

<success_criteria>
- 200 contended runs green where 10 of 200 were red, with the retry count and wall-time band recorded.
- `RETRY_LINES` reported whatever its value; a 0 is stated as "fix unproven in effect", never as proof.
- `VACUITY_CHECK` = 200.
- 8+ `rtk proxy cargo test --no-fail-fast` runs, per-run counts and failing-test names reported, the only failure being the git-version-constants pin.
- `./scripts/pre-tag-check.sh` gate 4 down to exactly one failing test.
- `rtk proxy cargo clippy -- -D warnings` clean.
- No assertion deleted or loosened; no skip attribute, thread-count flag or fixed pre-assertion sleep anywhere in the file.
- `deferred-items.md` closed append-only with 0 deletions, and the two-binary item declared fully closed with a pointer back at the `260917-k6y` section.
- `src/envelope/hooks.rs`, `Cargo.toml`, `.github/`, the version and all tags untouched.
</success_criteria>

<output>
Create `.planning/quick/260917-lkg-make-the-envelope-tracer-relocated-stub-/260917-lkg-SUMMARY.md` when done.

It must carry a "Coordinator inferred decisions (for audit)" section covering at minimum: the
`eprintln!` retry-visibility addition (beyond the literal change description, taken so the ETXTBSY
branch is countable rather than merely absent), and the isolation mode the run used.
</output>
