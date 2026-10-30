---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 05
subsystem: retirement
tags: [rust, retirement, envelope, guard, release-gate, pre-tag-check, journal]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-01 retargeted the ledger bound tests and routed push-step stderr through SAFE-04; 31-04 removed the drive core, so the guard's last callers were gone
provides:
  - no Phase 19 PreToolUse guard: envelope::{hooks, cred, advisory}, the classifier/tokenizer in policy.rs, park/park_at/ParkOutcome, the four hidden envelope CLI arms and the 16 carrier corpora are deleted
  - a reduced src/envelope (mod.rs, scan.rs, ledger.rs, a 554-line policy.rs) that is exactly what launcher/push.rs calls; `envelope scan` still answers 1 on a PEM key and 0 on a clean tree
  - a release gate without the git-version witness (one atomic commit across the constant, its test, pre-tag-check.sh, Dockerfile, release.yml, install-conformance-oracle.sh and CLAUDE.md)
  - a journal trimmed to its live users and a ParkReason pruned to the three reasons the push step produces
affects: [31-06, 31-07]

actuals:
  tokens: 8500
  tasks: 3
  commits: 3

plan_head_before: 235a912e54490b5bd6eed0aff9d749b5d5b87102
plan_head_after: f114b6612447434071c66394a66f297b9300ae63
commits: 3

tech-stack:
  added: []
  patterns:
    - "rebuild a 12k-line module from its kept regions by extracting ranges by item name into a new file, instead of editing in place"
    - "a census that is a count plus a row table (spawn_seam_guard argv aliases, async_blocking_guard markers) is re-measured in the same commit as the deletion, never relaxed"
    - "a temporarily retained constant (the witness) plus a temporary spawn-allowlist entry keep the release script runnable between two commits; both leave in the atomic D-12 commit"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs
    - src/envelope/mod.rs
    - src/envelope/policy.rs
    - src/envelope/ledger.rs
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/executor/codex.rs
    - src/journal/mod.rs
    - src/journal/reader.rs
    - src/journal/redact.rs
    - tests/common/mod.rs
    - tests/spawn_seam_guard.rs
    - tests/async_blocking_guard.rs
    - scripts/pre-tag-check.sh
    - scripts/pre-tag-check.Dockerfile
    - scripts/install-conformance-oracle.sh
    - .github/workflows/release.yml
    - CLAUDE.md
  deleted:
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - src/envelope/advisory.rs
    - src/journal/writer.rs
    - "16 tests/envelope_*.rs carrier corpora (envelope_pr_cap.rs stays)"
    - tests/journal_crash.rs
    - tests/journal_gitignore.rs

key-decisions:
  - "ParkReason is pruned to {PushOutsideNamespace, SecretDetected, PrCapExceeded} with their REASON_* constants, and gains `ParkReason::ALL` (the plan assumed it existed; it did not) so the distinctness test iterates one list [inferred — audit]"
  - "`journal::redact::RedactedLine` stays behind a narrow `#[allow(dead_code)]` with a comment: its only consumer (the writer) is gone, but its tests pin the redact-then-cap-then-serialise order and deleting a redactor test is not this plan's call. Cleanup deferred [inferred — audit]"
  - "reader keeps `tail_lines` and `TailRead` beside `tail_file`/`TailCursor` (the tail tests run through them); `JournalCursor` and the record-parsing family are deleted [inferred — audit]"
  - "`MAX_EVENT_PAYLOAD_BYTES` stays in journal/mod.rs because redact.rs imports it (the plan's 'whatever a kept module still imports')"
  - "the argv-alias census in spawn_seam_guard drops from 7 rows to 3 (Add, Remove, envelope Scan); the count was re-measured, not relaxed [inferred — audit]"
  - "the two container-only launcher failures are reported, not fixed: src/launcher/* is outside this plan's file set, belongs to the concurrent 28.1 follow-up, and the failures reproduce on the pre-plan tree [inferred — audit]"

patterns-established:
  - "a gate failure is attributed by re-running the failing tests in the same container image against `git archive <pre-plan sha>` extracted outside the repo (no worktree, no branch)"

requirements-completed: [RETIRE-01, RETIRE-03]  # copied verbatim from the plan; RETIRE-03's gate half (SC4) is OPEN, see "Open gate"

coverage:
  - id: D1
    description: "The Phase 19 guard (hooks, cred, advisory, classifier, tokenizer, park, the four hidden CLI arms, envelope_env) and its 16 carrier corpora are deleted; namespace refusal, secret scan, PR ledger and `envelope scan` survive and are exercised"
    requirement: RETIRE-01
    verification:
      - kind: integration
        ref: "rtk proxy cargo run -q -- envelope scan alpha <dir with a PEM key> => exit=1, `k.pem:1  rule=pem`; <clean dir> => exit=0, `findings: none`"
        status: pass
      - kind: unit
        ref: "rtk proxy cargo test --no-fail-fast --test launcher_push --test envelope_pr_cap --test agents_claude --test registry_worktree_guard --test spawn_seam_guard (13+7+15+8+18 pass)"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#only_refs_inside_the_namespace_are_allowed and src/envelope/ledger.rs#a_ledger_that_is_not_a_regular_file_is_refused_rather_than_read"
        status: pass
    human_judgment: false
  - id: D2
    description: "The git-version witness retires atomically with its release-gate scaffolding and the gate still runs all six gates"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "bash -n scripts/pre-tag-check.sh; the container run reached and printed all six gates, gates 1-3, 5, 6 PASS"
        status: pass
    human_judgment: false
  - id: D3
    description: "The journal is trimmed to its live users and ParkReason keeps only live reasons"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "rtk proxy cargo test --no-fail-fast --lib -- envelope:: journal:: driver::rate_limit:: classify_change (100 pass, includes every_park_reason_has_a_distinct_stable_snake_case_identifier)"
        status: pass
    human_judgment: false
  - id: D4
    description: "SC4: `./scripts/pre-tag-check.sh --container v1.15.0` exits 0 with all six gates PASS"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "./scripts/pre-tag-check.sh --container v1.15.0 => exit 1, gate 4 FAILED on two launcher tests that fail identically on the pre-plan tree 235a912"
        status: fail
    human_judgment: true
    rationale: "The gate is red for a reason outside this plan (28.1 launcher tests that depend on the host filesystem/runtime dir). Whether to fix them in a 28.1 follow-up and re-run, or to accept them, is the coordinator's scope call."

duration: 30min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 05: Retire the Phase 19 guard, the git-version witness and the legacy journal Summary

**The argv/bash-classifying PreToolUse guard, its hidden CLI arms and 16 carrier corpora are deleted (policy.rs 12654 -> 554 lines), the git-version witness retires atomically with its release scaffolding, the journal and ParkReason shrink to their live users, and the local suite is green (57 results, 0 failures); the container dry run passes 5 of 6 gates but gate 4 is red on two pre-existing 28.1 launcher tests.**

## Performance

- **Duration:** ~30 min of wall clock, plus the container run
- **Started:** 2026-10-25T06:16Z
- **Completed:** 2026-10-25T06:45Z
- **Tasks:** 3 (Tasks 1 and 2 done; Task 3's container gate ran and is red, see "Open gate")
- **Files modified:** 51 changed against the plan base (537 insertions, 58891 deletions)

## Accomplishments

- **Guard gone end to end.** `envelope::{hooks, cred, advisory}`, `park`/`park_at`/`ParkOutcome`, `EnvelopeAction::{PrePush, PreCommit, Askpass, Guard}`, `judged_alias_or_exit`, `ExecutionOptions::envelope_env` (and its two reads in `claude.rs` and `codex.rs`, the approved D-04 exception) and the 16 carrier corpora are deleted. `policy.rs` was rebuilt from its kept regions: namespace rules, `PushVerdict`/`classify_push_ref`, `ParkReason`, the `REASON_*` constants, the PR caps and `EnvelopePolicy::resolve`, with the ten named tests.
- **Survivors proven.** `envelope scan alpha <PEM tree>` exits 1 with `rule=pem` naming `k.pem`; a clean tree exits 0 with `findings: none`; `launcher_push`, `envelope_pr_cap`, the `agents_*` suites and `registry_worktree_guard` pass. Neither acceptance script was touched.
- **Witness retired in one commit** (`b99eb11`), including the constant, its test, every witness and banner line of `pre-tag-check.sh`, the Dockerfile rationale (`ppa:git-core/ppa` stays as runner fidelity), the `release.yml` parenthetical, the `install-conformance-oracle.sh` cross-reference and the CLAUDE.md Release Process text.
- **Journal trimmed (D-09).** `journal` is now `is_plain_path_component`, `classify_change`/`ChangeKind`, `sha256_digest`, `argv_digest`, `redact` and `reader::{tail_file, tail_lines, TailCursor, TailRead}` plus the two constants they import. `mod.rs` went 1950 -> 790 lines.
- **ParkReason pruned** to the three reasons the push step and ledger produce.
- **Ledger wording** names the push step and its `COUNT_DEADLINE_SECS`, not a hook or a registered deadline; `bytes, past the` is kept in the message and `registered` appears 0 times.

## Task Commits

1. **Task 1: Tracer - the guard is gone end to end** - `13ba5ac` (refactor)
2. **Task 2: witness retires atomically with its tooling** - `b99eb11` (refactor)
   **Task 2 (continued): journal trim + ParkReason prune** - `f114b66` (refactor)
3. **Task 3: the phase done gate** - no source change; FULLSUITE, cross-target clippy and the container run are recorded below.

**Witness commit `--stat`** (`git show --stat b99eb11`; it lists both policy.rs and pre-tag-check.sh as D-12 requires):

```
 .github/workflows/release.yml         |   4 +-
 CLAUDE.md                             |   4 +-
 scripts/install-conformance-oracle.sh |   3 +-
 scripts/pre-tag-check.Dockerfile      |  34 ++++----
 scripts/pre-tag-check.sh              | 143 +++-------------------------------
 src/envelope/policy.rs                |  73 -----------------
 tests/spawn_seam_guard.rs             |   3 -
 7 files changed, 34 insertions(+), 230 deletions(-)
```

## FULLSUITE (raw counts, read from the raw log, not through a pipe)

| Point | results | passed | failed | ignored | failures |
|---|---|---|---|---|---|
| 31-01 baseline | 94 | 4413 | 1 | 30 | witness |
| 31-04 end (plan base) | 75 | 3941 | 1 | 17 | witness |
| After Task 1, first run | 59 | 3327 | 2 | 17 | witness; `async_blocking_guard::every_named_blocking_helper_still_exists_in_the_tree` (stale markers, fixed in the same commit) |
| **End of plan (Task 3)** | **57** | **3290** | **0** | **17** | none; `editor_race_smoke` passed first time |

`results=57` equals `expected results=57` (54 `tests/*.rs` plus lib, bin and doc-tests).

**Passed delta, 3941 -> 3290 = -651 (-652 counting the witness):** Task 1 removed 613 (the 16 corpora about 457, policy.rs about 85 net of the 11 kept, hooks 35, cred 20, advisory 12, the `envelope/mod.rs` park tests 5); Task 2 removed 39 (the witness 1, `journal_crash` 3, `journal_gitignore` about 3, the writer, run-record, event and record-reader tests in `journal/`). Two binaries' worth of results fell (59 -> 57) for `journal_crash` and `journal_gitignore`. No surviving suite stopped running: every deleted count is explained by a deleted binary or module (D-11).

## Cross-target clippy, bollard, fmt

- `cargo clippy --locked --target {x86_64-pc-windows-gnu, aarch64-apple-darwin, x86_64-apple-darwin} --all-targets -- -D warnings`: all three exit 0. Host `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`: clean.
- RETIRE-02 (bollard): `grep -ci bollard Cargo.toml` = 0, `Cargo.lock` = 0.

## Container dry run (`./scripts/pre-tag-check.sh --container v1.15.0`)

- Runtime: docker (podman also available). Cache volume `gsdmm-pretag-target` finished at 23 GB, over the 20 GB cap, so the next run is cold.
- The reconciliation banner no longer exists (witness retired), so there is no MATCH/MISMATCH line to report.
- Inner run summary, quoted:

```
=== PRE-TAG SUMMARY ===
  gate 1  tag / Cargo.toml version   PASS
  gate 2  MSRV check                 PASS
  gate 3  cargo build --release      PASS
  gate 4  cargo test                 FAILED      (see gate4-test.log)
  gate 5  cargo clippy -D warnings   PASS
  gate 6  cargo package --locked     PASS
  The exit code reflects GATE FAILURES ONLY. SKIPPED is not a failure.
```

  Exit 1. Gate 6 (`cargo package --locked`) PASS: 282 files packaged, verified, no stranded `include_str!`. `driver_router_conformance` passed in the container (6 of 6) with the oracle provisioned.
- No tag was created (`git tag --points-at HEAD` is empty), nothing was published, nothing was pushed.

## Open gate (SC4 is NOT met)

Gate 4 failed on exactly two tests, both in `src/launcher`/`tests/launcher_*`, the 28.1 follow-up's area, which this plan does not edit:

1. `launcher::push::planted::tests::a_truncation_the_baseline_had_too_is_listed_but_does_not_block_the_push` (`src/launcher/push/planted.rs:1343`): left `["scan-truncated .", "removed .claude/worktrees/w/.git"]`, right `["scan-truncated ."]`. It walks a 10-file budget through `read_dir`, so which entries the truncated walk sees depends on the filesystem's directory order; it passes on the host and fails under the container's overlay. Introduced by `2d5ab57 fix(28.1): WR-08`.
2. `no_flag_names_a_credential_directory_and_the_home_block_precedes_the_run_binds` (`tests/launcher_sandbox.rs:305`): `--tmpfs /run/user/1000 missing`. `SandboxSpec::from_host` emits that flag only when the host user's runtime dir exists, and the container has no `/run/user/1000`.

**Attribution evidence.** Both tests fail identically when run in the same container image against `git archive 235a912` (the pre-plan tree) extracted outside the repo. They are not caused by this plan, and the gate was not made to pass by skipping, ignoring or weakening anything. The fix belongs in a 28.1 follow-up (make both tests independent of host directory order and of `/run/user/<uid>`), after which this plan's container run should be repeated. Everything else in the container passes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `async_blocking_guard` markers named deleted helpers**
- **Found during:** Task 1 FULLSUITE; Task 2 for the third marker
- **Issue:** `BLOCKING_HELPERS` named `probe_protection(`, `write_settings(` (deleted with `advisory`/`hooks`) and `JournalRun::start(` (deleted with the journal trim); `every_named_blocking_helper_still_exists_in_the_tree` correctly refused them.
- **Fix:** removed the three markers and extended the Phase 31 re-measure paragraph. The plan did not list this file.
- **Files modified:** tests/async_blocking_guard.rs
- **Commits:** 13ba5ac, f114b66

**2. [Rule 3 - Blocking] spawn_seam_guard censuses needed re-measuring**
- **Found during:** Task 1
- **Issue:** the argv-alias census counted 7 `alias:` fields and 7 rows, four of which were the deleted envelope arms; the spawn allowlist named `policy.rs`, `advisory.rs`, `cred.rs` (and later `journal/writer.rs`).
- **Fix:** 7 rows -> 3, planted-extra control and prose updated; allowlist entries removed with their comments. `policy.rs` was re-added with a comment in Task 1 because the temporarily kept witness test spawns `git --version`, and removed again in the D-12 commit.
- **Files modified:** tests/spawn_seam_guard.rs
- **Commits:** 13ba5ac, b99eb11, f114b66

**3. [Rule 1 - Bug] vacuous census assertion in text.rs**
- **Found during:** Task 1
- **Issue:** `!counted.iter().any(|site| site.contains("advisory.rs"))` now holds vacuously.
- **Fix:** kept, with a message that says so (the plan's first option); docs naming `envelope::advisory::*` marked "since deleted (Phase 31)".
- **Commit:** 13ba5ac

### Plan imprecisions (no code consequence)

- Task 1 acceptance `grep -cE 'PrePush|PreCommit|Askpass|Guard \{' src/cli.rs` is 0 could not hold literally: it also matches `DriverctlAction::StopGuard {` (3 matches), a surviving, unrelated Driverctl arm. The four envelope arms are gone (0 matches for `PrePush|PreCommit|Askpass`).
- The plan said `ParkReason::ALL` existed. It did not (the test listed the variants inline); one was added.

**Total deviations:** 3 auto-fixed (2 Rule 3, 1 Rule 1). **Impact:** none on scope; all are census or doc consequences of the deletions.

## Journal items deleted (D-09)

Deleted: `journal::writer` whole (`JournalWriter`, `read_active_run`, the active pointer, `parent_excludes_run_record`, `create_run_dir`, `write_run_record`, `ensure_runs_root`, `RUNS_GITIGNORE_BODY`, `prune_runs`, `crash_test_writer_loop`), `run_paths`/`RunPaths`, `JournalEvent`, `RunRecord`, `JournalRun`, `RUNS_SUBDIR`, `runs_root`, `MAX_RUN_JOURNAL_BYTES`, `RETAIN_RUNS`, and in the reader `JournalCursor`, `JournalRecord`, `ParsedLine`, `parse_line`, `ReadDiagnostics`, `seq_gaps`, `seq_gaps_from`, `read_all`; `tests/journal_crash.rs` and `tests/journal_gitignore.rs`. `driver_templates::create_run_dir` is a different function and stays. Kept: `is_plain_path_component`, `classify_change`/`ChangeKind`, `sha256_digest`, `argv_digest`, `redact` (with `RedactedLine` under a narrow `allow(dead_code)`), `reader::{tail_file, tail_lines, TailCursor, TailRead}`, `MAX_TAIL_BYTES`, `MAX_EVENT_PAYLOAD_BYTES`.

## Known Stubs

None. `RedactedLine` is dead code kept deliberately (see key-decisions), not a stub.

## Threat Flags

None. Threat register: T-31-14 (witness commit atomic, `bash -n` plus grep gate clean, container run proves the gate still runs all six gates), T-31-15 (`envelope scan` exits 1/0 end to end, acceptance scripts unchanged), T-31-17 (ten namespace/cap tests, scan and ledger unit tests, `envelope_pr_cap`, `launcher_push` pass; push.rs not edited), T-31-18 (gate 6 PASS) all hold; T-31-16 accepted as planned.

## For the coordinator (memory notes and runtime state)

- **Two memory notes are now obsolete or changed in subject** (not edited by this plan): `git-version-constants-test-env-failure.md` (obsolete, the witness is gone, there is no expected local failure any more) and `cargo-test-failfast-hides-envelope-suites.md` (its envelope subject is gone, the `--no-fail-fast` lesson stands).
- **New finding worth a note:** the two container-only 28.1 launcher failures above; the container image exposes host-dependent tests that a local run cannot.
- **Runtime state:** old envelope dirs under `~/.local/share/gsd-meta-manager/envelope/<alias>/` keep hook and askpass stubs that now fail closed with an unrecognised-subcommand error from clap (T-31-16); legacy run dirs under `.planning/meta-manager/runs/` stay on disk, ignored by `classify_change`.

## Next Phase Readiness

31-06 and 31-07 are unblocked by this plan. 31-07 (D-04 executor deletion) remains held by the user; its isolation exception (`envelope_env`) was taken here, so it can be dropped. SC4 needs a 28.1 follow-up for the two launcher tests and a repeat of the container run before the phase can claim its done gate.

## Self-Check: PASSED

- `src/envelope/{policy,mod,ledger,scan}.rs` exist; `src/envelope/{hooks,cred,advisory}.rs`, `src/journal/writer.rs`, `tests/journal_crash.rs`, `tests/journal_gitignore.rs` are absent.
- Commits `13ba5ac`, `b99eb11`, `f114b66` are ancestors of HEAD; `git rev-list --count 235a912..HEAD` = 3.
- Acceptance criteria re-run: all pass except the noted `StopGuard` literal and the SC4 container gate (open, documented above).
