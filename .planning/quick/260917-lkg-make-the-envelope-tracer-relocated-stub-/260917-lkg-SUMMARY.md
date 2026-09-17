---
phase: quick-260917-lkg
plan: 01
subsystem: tests
status: complete
tags: [flake, determinism, etxtbsy, envelope, test-harness]
requires: []
provides:
  - a bounded ETXTBSY-only exec retry at tests/envelope_tracer.rs's single spawn site
  - the mechanism record in the envelope_tracer module header
  - phase 21's two-binary deferred item, fully closed
affects:
  - tests/envelope_tracer.rs
tech-stack:
  added: []
  patterns:
    - "the `*_within` bounded-poll idiom (Instant deadline + 25ms in-loop sleep, Duration::from_secs(30) inline at the call site) extended from tests/driver_reattach.rs to the exec site in tests/envelope_tracer.rs"
key-files:
  created: []
  modified:
    - tests/envelope_tracer.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
decisions:
  - "The retry discriminates on ErrorKind::ExecutableFileBusy ONLY; every other spawn error fails on the spot under the verbatim pre-existing wording, so a genuinely non-executable stub is never masked"
  - "Deadline expiry panics naming path, limit and attempt count and stating the exec never happened — ETXTBSY is never accepted as the refusal the test asserts"
  - "Each retry emits one eprintln!, so the branch is COUNTABLE rather than merely absent (inferred decision, see audit section)"
  - "A second, distinct BrokenPipe race discovered by this task's own reproducer is registered OPEN and measured rather than fixed, because the plausible repair touches an assertion this task was forbidden from loosening"
metrics:
  duration: ~1 session
  completed: 2026-09-17
actuals:
  tokens: 21000
  tasks: 3
  commits: 3
plan_head_before: 8226c0ff27e1253b739666f9efd4985553b0680a
---

# Quick 260917-lkg: envelope_tracer ETXTBSY determinism Summary

A bounded, ETXTBSY-only exec retry at `run_stub`'s single spawn site makes
`a_relocated_copy_of_the_stub_refuses_instead_of_acting` deterministic against the
`Text file busy` race — proven in effect, not merely absent: across 800 contended runs the
retry branch fired 5 times and all 5 of those runs still reported 6 passed.

## What shipped

| Task | Commit | Files |
|---|---|---|
| 1 — bounded ETXTBSY-only exec retry in `run_stub` | `59108b1` | `tests/envelope_tracer.rs` |
| 2 — module header records the mechanism | `e505eac` | `tests/envelope_tracer.rs` |
| 3 — phase 21 `deferred-items.md` closed append-only | `41c0b5d` | `.planning/phases/21-.../deferred-items.md` |

`src/envelope/hooks.rs`, `Cargo.toml`, the crate version, `.github/` and all tags are
untouched. `git diff --numstat -- src/ Cargo.toml .github/` is empty.

## The finding

The table row phase 21 logged calls this "the classic write-then-exec race", and it is not
one. `strace -f -e trace=execve` caught the failing syscall three times and **every one was
the SANCTIONED stub**, never the relocated copy — so `std::fs::copy` is a bystander and the
leg that flaked is the CONTROL leg, which runs before the copy exists. The writer is a
`NamedTempFile` descriptor from `src/envelope/hooks.rs::write_stub`, inherited into a
SIBLING libtest thread's forked child and closed by `O_CLOEXEC` at that child's own
`execve` — but not one instruction before. ETXTBSY is per-INODE, which is what rules out
closing the file before the rename (`rename(2)` does not change the inode, and the exposure
window predates it).

## The fix, and what it does not do

`fn run_stub`'s signature and both call sites are unchanged. The `Command` is built once and
bound mutably (`spawn` takes `&mut self`), then spawned inside a loop with an
`Instant::now() + Duration::from_secs(30)` deadline and a 25ms in-loop sleep — the
`*_within` idiom `tests/driver_reattach.rs` already uses, at the same inline duration, with
no new named constant.

- Only `ErrorKind::ExecutableFileBusy` retries. Every other `Err` panics immediately under
  the **verbatim** pre-existing message `the generated stub is executable`, so a bad mode
  (`PermissionDenied`) or bad shebang (`ENOEXEC`) still fails for its own reason.
- Expiry `panic!`s naming the stub path, the limit and the attempt count, and states the
  exec never happened. It never returns a synthesised `Output` and never skips.
- No assertion deleted or loosened (`grep -c '^-.*assert'` on the task diff = 0); no
  `#[ignore]` (0 on non-comment lines); no `--test-threads` (0 on non-comment lines); the
  only `thread::sleep` is the one INSIDE the poll loop.
- `ErrorKind::ExecutableFileBusy` compiles at the declared MSRV floor (1.88); pre-tag-check
  gate 2 (`cargo +1.88 check --all-targets --locked`) passes.

## Measurements

### The contended reproducer (8 concurrent binaries × 25 rounds = 200 runs)

| Sample | Runs | ETXTBSY failures | Retries absorbed | Runs reporting 6 passed | Wall band/round |
|---|---|---|---|---|---|
| baseline on file (planning) | 200 | **10 (5%)** | n/a | n/a | n/a |
| pre-change re-confirmation, this machine | 200 | **0** | 0 | 200 | 210–305ms |
| after, pass 1 | 200 | 0 | **1** | 199 | 181–276ms |
| after, pass 2 | 200 | 0 | **2** | 200 | same band |
| after, pass 3 | 200 | 0 | **0** | 200 | same band |
| after, pass 4 | 200 | 0 | **2** | 200 | same band |
| **after, total** | **800** | **0** | **5** | **799** | same band as pre-change |

**The fix is proven IN EFFECT.** `RETRY_LINES` is the evidence the plan asked to be reported
whatever its value: 5 retries fired, each naming the sanctioned stub path and attempt
number, and every run that retried still reported `6 passed`. Had the count been 0 the
correct report would have been "the sample did not reproduce the race"; it is not 0.

**The pre-change sample came back 0/200 red and is reported rather than suppressed.** At a
5% per-run rate one clean 200-run sample is ordinary and neither refutes the 10/200 baseline
nor shows there was nothing to fix. It does mean this particular pair of samples supplies no
before/after contrast — the contrast that carries the claim is the 5 absorbed retries.

**Anti-vacuity.** `fixture()` returns `None` on git failure and the tests then return early,
so `6 passed` can be vacuous. Three controls: the per-run `6 passed` count (799/800, the one
shortfall being the unrelated failure below), the wall-time band (post-change band sits
inside the pre-change band — no suspiciously fast greens), and a direct
`strace -f -e trace=execve` of a single `--exact` run counting **2** `pre-push` `execve`
calls, proving both the control and relocated legs actually exec.

### Nine full-suite `rtk proxy cargo test --no-fail-fast` runs

Output redirected to files and read from the files — never piped through `grep` live.

| Run | suites | passed | failed | ignored | Failing tests |
|---|---|---|---|---|---|
| 1 | 48 | 2120 | 1 | 15 | version witness |
| 2 | 48 | 2120 | 1 | 15 | version witness |
| 3 | 48 | 2120 | 1 | 15 | version witness |
| 4 | 48 | 2120 | 1 | 15 | version witness |
| 5 | 48 | 2120 | 1 | 15 | version witness |
| 6 | 48 | 2120 | 1 | 15 | version witness |
| 7 | 48 | 2119 | **2** | 15 | version witness **+ `driver::run::tests::the_current_group_agrees_with_the_proc_parse`** |
| 8 | 48 | 2120 | 1 | 15 | version witness |
| 9 | 48 | 2120 | 1 | 15 | version witness |

"version witness" =
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— environmental (local git 2.53.0 against constants re-derived at 2.55.0), expected green on
CI, untouched as instructed.

`a_relocated_copy_of_the_stub_refuses_instead_of_acting` is green in all nine;
`Text file busy` appears in none of the nine.

**Run 7's extra failure is reported, not absorbed.** It is a separate, previously-recorded
per-run-flaky process test (quick `260917-ii4` measured it as a per-run-flaky process test
across three re-runs each way), it lives in `src/driver/run.rs` — a file this task did not
open — and nothing here claims anything about it.

### `./scripts/pre-tag-check.sh` — gate by gate

| Gate | Result |
|---|---|
| 1 — tag vs `Cargo.toml` version | **SKIPPED** (no tag argument given; correct, v1.7.1 is out of scope) |
| 2 — MSRV `cargo +1.88 check --all-targets --locked` | **PASS** |
| 3 — `cargo build --release` | **PASS** |
| 4 — `cargo test --no-fail-fast` | **FAILED on exactly ONE test** — the version witness. 2120 passed / 1 failed / 15 ignored |
| 5 — `cargo clippy -- -D warnings` | **PASS** |

Script exit 1 (gate 4). The git-version mismatch is surfaced as the script's designed loud
ADVISORY (installed 2.53.0 vs derived-against 2.55.0), never as a gate failure. Zero gates
`NOT REACHED`.

### Clippy

`rtk proxy cargo clippy -- -D warnings` → exit 0, clean. Additionally
`cargo clippy --test envelope_tracer -- -D warnings` → exit 0, so the edited target adds no
lint of its own.

## Known Stubs

None.

## Deviations from Plan

None to the plan's own actions. One discovery outside its scope, handled under the
plan's own reporting discipline rather than fixed — see below.

## A second, distinct race — found, measured, registered OPEN

This task's own reproducer surfaced a **different** failure in the same helper:

```
the stub reads its stdin: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
```

at the `write_all` in `run_stub` — **1 in 800** contended runs (0.125%), 0 in the nine
full-suite runs, and absent from the 200-run pre-change sample (so no before/after rate is
claimed).

**Not introduced by this change**, structurally rather than by argument: the `write_all` and
its `.expect("the stub reads its stdin")` are byte-identical before and after; the task
commit's diff touches the `spawn` and nothing between it and `wait_with_output`.

**Likely mechanism, stated as hypothesis and NOT as measurement:** the stub `exec`s the
binary's `envelope pre-push` handler; on the RELOCATED leg `assert_provenance` refuses —
which is precisely what the test exists to prove — and the process can exit before reading
stdin, so the parent's write races the child's exit. Not confirmed by strace.

**Why it was not fixed here.** The plausible repair is to stop treating `BrokenPipe` on that
write as fatal, and that touches an assertion — which this task was explicitly forbidden
from loosening, and which is a judgement about what the test is entitled to require of a
refusing stub rather than a synchronisation detail. Registered OPEN with its measurements in
`deferred-items.md`, following this project's round discipline (cf. T-19-91: measured,
pinned and registered OPEN rather than closed). A note there records what a fixer should
settle first — whether `.expect("the stub reads its stdin")` asserts anything real, given
that a successful write into a 64KB pipe buffer proves the buffer accepted the bytes, not
that the stub read them.

**Consequence for the plan's success criteria, stated plainly:** three of the four
post-change contended passes were 200/200 green; pass 1 carried this one unrelated failure.
The plan's C-1 truth — zero runs failing with `Text file busy` — holds across all 800.

## Coordinator inferred decisions (for audit)

1. **The `eprintln!` on each retry** is an addition beyond the literal change description,
   flagged by the plan itself for audit. Taken so the ETXTBSY branch is COUNTABLE rather
   than merely absent — libtest captures it by default and the reproducer runs with
   `--nocapture`. It is what turned this from "the flake did not recur" into "the branch
   fired 5 times and every one of those runs still passed", so it is load-bearing evidence
   rather than decoration.
2. **Isolation mode: `none`.** Ran unisolated on the primary checkout, branch `master`,
   after the harness-worktree isolation was auto-degraded by `worktree.base-check`
   (`baseref-head-ignored-by-harness`, the known #1941/#48 degrade on this repo's `master`).
   No worktree created, no branch switched.
3. **Nine full-suite runs, not eight.** The contract asked for at least eight; a ninth was
   run because run 7 carried an unrelated extra failure and one more sample makes the
   "8 of 9 identical" shape legible rather than leaving the odd run at the boundary.
4. **Four contended passes, not one.** The plan required one post-change reproducer run. The
   pre-change sample came back 0/200, which would have left a single post-change 0/200
   proving nothing by itself. Three extra passes were run to reach a retry count that is
   evidence — 800 runs, 5 absorbed retries.
5. **The BrokenPipe race was NOT fixed**, though it is in an in-scope file and in the very
   function this task edited. Judged to fall on the far side of the "no assertion loosened"
   prohibition, and registered OPEN instead. This is the one call in this task a reviewer
   might reasonably have made the other way; the evidence for either choice is recorded in
   `deferred-items.md` so the decision can be revisited without re-derivation.

## Self-Check: PASSED

- `tests/envelope_tracer.rs` — FOUND
- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` — FOUND
- Commit `59108b1` — FOUND
- Commit `e505eac` — FOUND
- Commit `41c0b5d` — FOUND
- `git diff --numstat` on `deferred-items.md`: `253  0` — append-only, 0 deletions
- `git diff --numstat -- src/ Cargo.toml .github/`: empty
