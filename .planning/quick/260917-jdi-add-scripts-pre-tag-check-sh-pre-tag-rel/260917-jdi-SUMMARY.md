---
phase: quick-260917-jdi
plan: 01
subsystem: release-tooling
status: complete
tags: [release, ci-parity, shell, tooling]

requires: []
provides:
  - "scripts/pre-tag-check.sh — local dry-run of .github/workflows/release.yml's tag-push gates"
  - "CLAUDE.md Release Process step 3 routed through that script"
affects:
  - "the release procedure (steps 1-6 in CLAUDE.md); no runtime code path"

tech-stack:
  added: []
  patterns:
    - "runtime extraction of a Rust source constant into a shell gate, so no version literal is duplicated"
    - "advisory-vs-failure separation: a reconciliation mismatch shouts twice but never moves the exit code"
    - "read-only rustup toolchain detection (never `cargo +<floor>`, which auto-installs)"

key-files:
  created:
    - scripts/pre-tag-check.sh
  modified:
    - CLAUDE.md

decisions:
  - "Gate order inverted vs the CI job graph (DEC-1): the tag check costs one cargo metadata call, so it runs first and hard-stops before minutes of compilation"
  - "Only gate 1 hard-stops; gates 2-5 run to completion and collect statuses, so one invocation yields the whole picture (DEC-2)"
  - "The MSRV floor is derived from cargo metadata's rust_version and matched as floor-followed-by-[.-]-or-end, which is what made the two-component `1.88` declaration select the installed `1.88-x86_64-unknown-linux-gnu` (DEC-3)"
  - "A git/constant mismatch is a prominent advisory, never a script-level failure (DEC-4)"
  - "Gate 4's expected witness failure is explained in the summary, never filtered or skipped (DEC-5)"

metrics:
  duration: 10min
  completed: 2026-09-17

actuals:
  tokens: 4706
  tasks: 3
  commits: 2
plan_head_before: b157b1b2bbdaaf43e64500e509fe973e76205219
---

# Quick Task 260917-jdi: Pre-Tag Release Dry-Run Summary

`scripts/pre-tag-check.sh` reproduces release.yml's five tag-push gates locally before the tag
exists, extracting the derived-against git constant from `src/envelope/policy.rs` at runtime so the
CI-vs-local git divergence is impossible to miss but never fakes a red run; CLAUDE.md release step 3
now invokes it and carries the caveat.

## What Was Built

**`scripts/pre-tag-check.sh`** (new, mode 100755, 406 lines). `#!/usr/bin/env bash`, `set -u` only —
deliberately no `set -e` (gates 2-5 must all run) and no `pipefail` (the extraction pipelines end in
`head -1`, whose SIGPIPE would read as a failure). Structure: usage parsing, status initialisation,
`print_summary` + `trap ... EXIT`, constant extraction, reconciliation banner, `mktemp -d` log
directory, then the five gates.

- **Constant extraction** reads the declaration line only (an ERE anchored at line start through
  `pub const <NAME> : &str =`), so the several doc-comment lines that also name the constant cannot
  match. A missing file, a non-matching declaration, or an empty captured literal is a **hard error,
  exit 1** — the one constant-related condition that does fail the script, since without its subject
  it cannot honestly report a reconciliation at all.
- **Reconciliation banner** prints on every run before gate 1, headed `!!! GIT VERSION
  RECONCILIATION !!!`, and the mismatch arm states one claim per line: this run cannot validate that
  test the way CI will; the `ubuntu-latest` runner's ambient git is the authority; a green local gate
  does not guarantee publish. Repeated verbatim in the summary as an `ADVISORY:` block.
- **Gate 1** mirrors the publish job's tag check with the same `${ARG#v}` strip. No argument →
  reports the crate version, marked SKIPPED. Mismatch → prints both values and exits 1 immediately.
- **Gate 2** derives the floor from `cargo metadata`'s `rust_version`, then selects a toolchain by
  name from `rustup toolchain list` (dots escaped, floor followed by `.`, `-`, or end-of-name) and
  invokes the **full matched name**. No `rustup` → SKIPPED; no `rust-version` → FAILED, matching CI's
  own reasoning. The script never probes with `cargo +<floor> --version`, which would auto-install
  the toolchain it claims to be testing for (T-jdi-01); a comment says exactly that.
- **Gates 3-5** run `cargo build --release`, `cargo test --no-fail-fast`, `cargo clippy -- -D
  warnings`, each tee'd to a file in the log directory, each taking its status from
  `${PIPESTATUS[0]}` rather than from tee.
- **Summary/exit trap** is installed before extraction, so it fires on the hard error and on gate 1's
  hard stop too. All five statuses initialise to the literal `NOT REACHED`, so an early exit reports
  honestly instead of claiming a skip it never judged. Exit 0 when no gate FAILED, 1 when any did.

**`CLAUDE.md` step 3** replaces the inline `cargo build && cargo test && cargo clippy -- -D warnings`
chain with `./scripts/pre-tag-check.sh vX.Y.Z`, and names the `ubuntu-latest` runner as the authority
for the witness test. Still item 3 of an untouched 6-item list.

## Task 3 — the end-to-end run against the real tree

Run as `./scripts/pre-tag-check.sh v1.7.0` (tag derived from the manifest, not hardcoded).
**Observed exit code: 1.** No cargo-lock contention; the run completed in roughly four minutes on
warm caches, so the plan's deferral branch was not taken.

| Gate | Name | Status |
|------|------|--------|
| 1 | tag / Cargo.toml version | PASS (`v1.7.0` → `1.7.0` = crate `1.7.0`) |
| 2 | MSRV check | **PASS — it RAN, it did not skip** |
| 3 | `cargo build --release` | PASS |
| 4 | `cargo test --no-fail-fast` | FAILED |
| 5 | `cargo clippy -- -D warnings` | PASS |

Zero occurrences of `NOT REACHED` in the transcript: every gate was reached and classified.

**Gate 2 ran**, selecting `1.88-x86_64-unknown-linux-gnu` from the declared two-component floor
`1.88` and running `cargo +1.88-x86_64-unknown-linux-gnu check --all-targets --locked` to a clean
exit. This is the DEC-3 case working as measured at planning time: a naive `^1\.88\.0` match would
have reported SKIPPED on a machine that has the floor installed.

**Gate 4's failing set** was exactly the three tests documented as pre-existing on this tree —
2118 passed / 3 failed / 15 ignored across 48 suites:

1. `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
   — the version witness, expected to fail while the advisory stands (installed git `2.53.0`,
   constant derived against `2.55.0`).
2. `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` (`tests/driver_reattach.rs`).
3. `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
   (same file) — the long-standing per-run-flaky pair documented in that module's header.

None of the three is a defect in this script, and none was filtered, skipped or carved out. Per
DEC-5 the script detected the witness name in gate 4's captured output and printed, in the summary,
that the witness is among the failures and is expected while the advisory stands, so the operator
should read the gate-4 log for any OTHER failure before concluding a regression. **A non-zero exit
here is the correct result and the verification that the script works**, not a task failure.

The reconciliation block printed verbatim (both in the banner and repeated in the summary):

```
!!! GIT VERSION RECONCILIATION !!!
  installed git (this machine) : git version 2.53.0
  derived-against constant     : git version 2.55.0
    from src/envelope/policy.rs
  MISMATCH — read this:
    1. THIS LOCAL RUN CANNOT VALIDATE THAT TEST THE WAY CI WILL.
    2. The GitHub ubuntu-latest runner's ambient git is the AUTHORITY for it.
    3. A GREEN LOCAL GATE THEREFORE DOES NOT GUARANTEE THE PUBLISH JOB PASSES.
```

## Verification

- `bash -n scripts/pre-tag-check.sh` parses clean; `git ls-files -s` reports mode **100755**.
- `shellcheck` is not installed and was **not** installed (per the plan's explicit prohibition); its
  absence is not a gate failure.
- No version literal is baked in: `grep -v '^[[:space:]]*#' | grep -Ec '"git version [0-9]'` = **0**,
  and the runtime-extracted constant appears in every transcript.
- Wrong-tag run (`v0.0.0-nope`) exits non-zero **before any compilation**, having printed the banner
  and a summary in which gates 2-5 read `NOT REACHED`.
- Additional edge paths exercised beyond the plan's verify block: a second positional argument is
  rejected with usage and **exit 2**; running from outside the repository root (so `policy.rs` is
  absent) produces the **hard error, exit 1**, and still prints the summary via the trap.
- CLAUDE.md step 3 contains `scripts/pre-tag-check.sh`, contains `runner`, no longer contains the
  `cargo build &&` chain, and the list is still exactly 6 items.
- Scope confirmed: `git diff b157b1b..HEAD --name-only` = `CLAUDE.md`, `scripts/pre-tag-check.sh`.
  **Exactly two files.** Nothing under `.github/`, nothing in `Cargo.toml`, nothing in `src/`, no
  tag, no release step. The two untracked entries in the working tree (`.gsd/`, `.planning/state.json`)
  are pre-existing and were not touched.

## Deviations from Plan

None material — the plan was executed as written, with three small implementation choices the plan
left unspecified (listed below for audit).

## Inferred Decisions (human unavailable — audit these)

1. **Committed on `master`.** The generic five-name protected-branch fallback flags `master`, but the
   orchestrator's dispatch specified sequential execution on the primary checkout with
   `isolation: none` (the repo's known `worktree.base-check` false negative), and this repo's own
   convention puts working commits on `master` with `dev` as the PR target — every recent commit
   including `b157b1b` sits on `master`. Proceeded rather than halting on the generic rule.
2. **Gate 1 gained an unspecified defensive branch.** If `cargo metadata` yields no
   `.packages[0].version`, the gate is marked FAILED with that reason and the script **continues** to
   gate 2 rather than hard-stopping. Only the tag-vs-version mismatch hard-stops, which keeps DEC-2's
   letter (gate 1 hard-stops on the mismatch; everything else collects statuses).
3. **`-h`/`--help` and unknown `-*` options are handled before the EXIT trap is installed**, so
   `--help` prints clean usage and exits 0 without a misleading all-`NOT REACHED` summary; an unknown
   dash-leading option exits 2 with usage. The plan specified only the second-positional rejection.
4. **Task 3 carries no commit.** It is an exercise task whose action is to run the script; it found no
   defect of the script's own, so `scripts/pre-tag-check.sh` was not modified after Task 1's commit.
5. **Task 3's verify was split** into a backgrounded run plus the assertion block evaluated against
   the captured transcript, because a full five-gate run exceeds the foreground tool-call ceiling.
   Same commands, same assertions, same transcript — only the invocation shape differs.

## Threat Flags

None. No new network endpoint, auth path, or schema surface; no dependency was added. The
mitigations the plan's threat register assigned are all present in the delivered script: T-jdi-01
(no `cargo +<floor>` probe; read-only `rustup toolchain list`), T-jdi-02 (banner + summary advisory
naming the runner as authority), T-jdi-04 (the tag argument is never `eval`'d, never interpolated
into a path, and every expansion is quoted), T-jdi-05 (gate 4's verdict is untouched; only an
explanatory line is added).

## Known Stubs

None.

## Self-Check: PASSED

- `scripts/pre-tag-check.sh` — FOUND (mode 100755)
- `CLAUDE.md` — FOUND (step 3 rewritten)
- Commit `cf2219c` — FOUND
- Commit `b0cb5be` — FOUND
- `git rev-list --count b157b1b..HEAD` = 2, matching the `commits:` frontmatter
