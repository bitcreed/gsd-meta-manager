---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 01
subsystem: retirement
tags: [rust, retirement, push-step, safe-04-redaction, pr-ledger, router, rate-limit]

requires:
  - phase: 28.1-sandbox-egress-agent-channel-isolation
    provides: the launcher push step (push.rs) whose child stderr this plan redacts
provides:
  - redact_captured, the one capture-time redactor both persisted push-step stderr sites call
  - a PR ledger (ledger.rs) that names no item of the hooks module, with its T-19-117 / T-19-120 bound pins in its own unit tests
  - tests/envelope_pr_cap.rs driving the cap through ledger::record_and_check_in
  - a next_cli.rs target-phase refusal that does not name DriveError
  - the quota-event classification test inside driver::rate_limit's own tests
affects: [31-02, 31-03, 31-04, 31-05, 31-06]

actuals:
  tokens: 14100
  tasks: 3
  commits: 4

plan_head_before: 17e0b5feb8b0bdab0b27c9544b1dab841eaf4359
plan_head_after: 46901b55c3365e08adb934d7459094169967b9fe
commits: 4

tech-stack:
  added: []
  patterns:
    - "capture-time redaction helper wrapping redact_urls then journal::redact::redact"
    - "blocking-FIFO fixtures run on a spawned thread with recv_timeout(12s), the FIFO made by rustix::fs::mknodat (no spawn in src/)"

key-files:
  created: []
  modified:
    - src/launcher/push.rs
    - tests/launcher_push.rs
    - src/envelope/ledger.rs
    - tests/envelope_pr_cap.rs
    - src/next_cli.rs
    - src/driver/rate_limit.rs
    - tests/driver_rate_limit.rs

key-decisions:
  - "redact_captured is the single redactor for every child-stderr line the push step persists (git_with_env last_error, pr-failed detail); the push body's existing redact_urls pass stays as a harmless idempotent second pass"
  - "The ledger owns COUNT_DEADLINE_SECS = 5 and the size-bound message stays byte-identical; MAX_LEDGER_BYTES unchanged"
  - "The 202 MB T-19-117 row uses a sparse file (set_len) because the bound is a stat-only check [inferred - audit]"
  - "next --json refusal text stays byte-identical to the old DriveError rendering (verified by diffing stderr for three inputs)"

patterns-established:
  - "Test fixtures in src/ must not spawn processes: tests/spawn_seam_guard.rs censuses every spawn site in src/, test modules included"

requirements-completed: []  # RETIRE-01 is shared with 31-02..31-07; the shared-ID gate (#2388) keeps it open until every declaring plan has a SUMMARY, and PATTERNS says flip RETIRE-01..03 only at verification

coverage:
  - id: D1
    description: "A gh pr create that fails with a GitHub token on its last stderr line logs a redacted pr-failed detail; no token byte reaches LOG.md"
    requirement: RETIRE-01
    verification:
      - kind: integration
        ref: "tests/launcher_push.rs#a_failed_pr_create_logs_its_stderr_redacted"
        status: pass
      - kind: unit
        ref: "src/launcher/push.rs#redact_captured_masks_a_token_and_url_userinfo"
        status: pass
    human_judgment: false
  - id: D2
    description: "ledger.rs names no hooks item; the size-bound refusal text is unchanged; the seven T-19-117/T-19-120 bound pins and the link-non-following stat slice pin live in kept tests"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "src/envelope/ledger.rs#tests (23 pass, 7 new)"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_link_non_following_stat_variant_is_absent_from_ledger_rs_production_half"
        status: pass
    human_judgment: false
  - id: D3
    description: "tests/envelope_pr_cap.rs reaches the cap only through record_and_check_in; window, boundary, per-run, record-before-permit and the hostile-alias ledger half keep their assertions"
    requirement: RETIRE-01
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs (7 pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "next --json --target-phase refuses a non-plain component with the same stderr text, without naming DriveError; the quota-event test lives in rate_limit.rs and the file no longer names driver::bounds"
    requirement: RETIRE-01
    verification:
      - kind: integration
        ref: "tests/next_cli.rs#a_traversing_target_phase_is_refused"
        status: pass
      - kind: unit
        ref: "src/driver/rate_limit.rs#the_synthesised_fixtures_quota_event_classifies_as_a_rejection"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 01: Rewire the Survivors Summary

**SAFE-04 redaction on the push step's persisted child stderr (`redact_captured`), a PR ledger that stands alone with its seven T-19-117/T-19-120 bound pins retargeted onto `record_and_check_in`, and a router `next` refusal and rate-limit test that no longer depend on the legacy drive layer.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-10-25T04:31:47Z
- **Completed:** 2026-10-25T04:49Z
- **Tasks:** 3 (1 tracer, 2 auto) plus one in-plan fix commit
- **Files modified:** 7

## Phase baseline (D-11)

Measured with the FULLSUITE command before the first edit, at HEAD `17e0b5feb8b0bdab0b27c9544b1dab841eaf4359`
(the plan text cites 036426d; HEAD had moved by the docs commits listed in the dispatch). 31-02 had not merged.

```
results=94 passed=4413 failed=1 ignored=30
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
expected results=94
```

The lone failure is the known environmental git-version witness (local git 2.53.0 against the constant's 2.55.0).
`editor_race_smoke` passed. The research baseline's second failure (28.1 WR-09 render probe) is fixed at this HEAD.

## Post-plan suite

```
results=94 passed=4418 failed=1 ignored=30
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
expected results=94
```

+5 passed = +1 push unit test, +1 launcher_push integration test, +7 ledger unit tests, -4 net in `envelope_pr_cap.rs`
(11 tests before; 7 after: five settings/argv tests deleted, hostile-alias test renamed, slice pin moved in).
`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both exit 0.

## Accomplishments

- **D-09 redaction.** `redact_captured` (`redact_urls`, then `journal::redact::redact`) now wraps both persisted capture
  sites. A negative control confirmed the integration test is not vacuous: with the `pr-failed` site temporarily reverted
  to `redact_urls`, `a_failed_pr_create_logs_its_stderr_redacted` fails showing the raw `ghp_...` token in LOG.md.
- **Ledger decoupled (D-05 prerequisite).** `COUNT_DEADLINE_SECS = 5` lives in `ledger.rs`; `grep -c 'hooks::'` is 0
  (baseline 3); the size-bound refusal renders byte-identically; `MAX_LEDGER_BYTES` is unchanged.
- **Bound pins retargeted (C5, D-10).** Seven `envelope::ledger::tests` call `record_and_check_in` directly; FIFO rows
  run on a thread with `recv_timeout(12s)`. The slice pin moved into `tests/envelope_pr_cap.rs` unchanged.
- **C8 / C9.** `next_cli.rs` has its own refusal (stderr verified byte-identical for `../1`, an ESC-bearing value and
  `x/y`); `rate_limit.rs` owns the quota-event test with a local `T09` and no longer names the bounds module.

## Task Commits

1. **Task 1 (tracer): SAFE-04 redaction on the push step's captured stderr** - `8ed63d2` (feat)
2. **Task 2: PR ledger stands alone, cap suite over `record_and_check_in`** - `9aaaa8f` (test)
3. **Task 3: `next` refusal and rate-limit test stop depending on the drive layer** - `3dbea42` (refactor)
4. **In-plan fix: ledger test FIFO without spawning a process** - `46901b5` (fix)

Tracer gate: `HUMAN_VERIFY_MODE=end-of-phase` and the tracer `<verify>` is automated-only, so it was re-run
(`launcher_push` 15 pass, `launcher::push::` lib 36 pass) before expansion. Tracer verified end-to-end, expanding.

## Decisions Made

- `redact_captured` placed beside `redact_urls`; the existing `redact_urls(&pushed.last_error)` before the `push-failed`
  log stays (idempotent second pass, per the plan).
- T-19-117 headline row: the ledger half only (the guard's forge/non-forge split is out of subject). The 202 MB ledger is a
  sparse file via `set_len`, because the bound is a `stat` and never reads it; asserts: `Err` containing `bytes, past the`,
  the message names `COUNT_DEADLINE_SECS`, refusal in under a quarter of the deadline, and the file is neither read nor
  appended. `[inferred - audit]`
- Module docs in `rate_limit.rs` that linked `super::bounds` / `BoundsReason` were reworded so the file no longer names a
  module 31-04 deletes. Doc-only. `[inferred - audit]`
- No test-first RED commit for Task 2 (`tdd="true"`): it retargets assertions onto behaviour that already exists, so every
  new test is green on first run by construction. `workflow.tdd_mode` was not passed and this is not a `type: tdd` plan.
  The Task 1 test was shown non-vacuous by the negative control above.

## Duplicated assertions (Task 2 merge rule)

No assertion was dropped. Two assertions appear in two tests each, because both test names are acceptance-pinned and
they come from different corpus files: the FIFO-is-refused row (`a_ledger_that_is_not_a_regular_file_...` and the FIFO row of
`the_ledger_kind_bound_is_non_vacuous_...`) and the fresh-root-permits row (`a_fresh_envelope_root_...` and the last row of
`the_ledger_kind_bound_...`). The code is shared through `make_fifo`, `attempt_within_twelve_seconds` and
`assert_not_a_regular_file`, so nothing is copy-pasted. The seven originals still exist in the old corpora until 31-05.

## D-09 stdio finding (recorded, no call added)

`eprintln!("driver launch: push: {outcome}")` in `src/launcher/run.rs` (line 833) and `out_line`/`err_line` in
`src/driver_cli.rs` (`push_finished_run` match) write to stdio, not to LOG.md. They are not routed through
`redact_captured`, per D-09: record the finding rather than adding a call for its own sake. The `PushOutcome::Failed
{ detail }` they print is already built from `last_error`, which is now `redact_captured`, so it is redacted anyway.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Ledger test fixture spawned `mkfifo` from `src/`, tripping the spawn-seam census**
- **Found during:** Task 3 closing FULLSUITE (first run: 2 failures, the second being
  `every_process_spawn_site_in_src_is_on_the_allowlist`, "Unexpected: [src/envelope/ledger.rs]")
- **Issue:** the plan prescribed `std::process::Command::new("mkfifo")` for FIFO fixtures, but `tests/spawn_seam_guard.rs`
  censuses `Command` use across `src/` including `#[cfg(test)]` modules; widening the allowlist for a test fixture would
  weaken a security census.
- **Fix:** `make_fifo` uses `rustix::fs::mknodat(..., FileType::Fifo, ...)`, the same call
  `state_reader::driver_frontmatter`'s FIFO tests already use. No allowlist change.
- **Files modified:** `src/envelope/ledger.rs`
- **Verification:** `spawn_seam_guard` 41 pass; ledger tests 23 pass; FULLSUITE back to the baseline failure only
- **Committed in:** `46901b5`

**Total deviations:** 1 auto-fixed (1 bug). **Impact:** none on scope; the plan's `mkfifo` hint was incompatible with a
standing census.

## Issues Encountered

None beyond the deviation above.

## Known Stubs

None.

## Threat Flags

None. The plan added no network endpoint, auth path or file-access surface.

## Next Phase Readiness

Ready for 31-02..31-06. Left in place on purpose: the original guard corpora (`envelope_carrier_reach`,
`envelope_interior_path`, `envelope_word_set`) still hold their copies of the bound rows until 31-05 deletes them. The
git-version witness stays red locally until 31-05. Plan 31-07 (D-04, held) was not touched. `RETIRE-01` is left open by the
shared-ID gate until every plan declaring it has a SUMMARY.

## Self-Check: PASSED

- Files: `src/launcher/push.rs`, `tests/launcher_push.rs`, `src/envelope/ledger.rs`, `tests/envelope_pr_cap.rs`,
  `src/next_cli.rs`, `src/driver/rate_limit.rs`, `tests/driver_rate_limit.rs` all present and modified
- Commits `8ed63d2`, `9aaaa8f`, `3dbea42`, `46901b5` are ancestors of HEAD; `git rev-list --count` from the recorded
  `plan_head_before` is 4
- Acceptance greps: `redact_captured(` x4, `journal::redact::redact` x2, `hooks::` 0 in ledger.rs and envelope_pr_cap.rs,
  `record_and_check_in` x6 in envelope_pr_cap.rs, `COUNT_DEADLINE_SECS` const present, `MAX_LEDGER_BYTES` line unchanged,
  `DriveError` 0 in next_cli.rs, `driver::bounds` 0 in rate_limit.rs, moved test 1 / 0
- Plan verification: launcher_push, envelope_pr_cap, next_cli, driver_router_conformance, driver_router_table, lib
  `launcher::push::` / `envelope::ledger::` / `driver::rate_limit::` all exit 0; clippy and fmt clean

---
*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Completed: 2026-10-25*
