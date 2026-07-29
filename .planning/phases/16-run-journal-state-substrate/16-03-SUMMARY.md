---
phase: 16-run-journal-state-substrate
plan: 03
subsystem: infra
tags: [gitignore, atomic-write, retention, byte-cap, run-record, obs-01, safe-04, d-05, d-06, d-07, d-08, d-31, d-32]

# Dependency graph
requires:
  - phase: 16-run-journal-state-substrate
    plan: 01
    provides: "RunPaths / run_paths / runs_root / RunRecord / MAX_RUN_JOURNAL_BYTES / RETAIN_RUNS, JournalWriter and its bytes_written accessor, JournalEvent::is_content, and the RedactedLine seam every write here still passes through"
provides:
  - "create_run_dir: the run directory with its ignore posture already in place before the first journal byte exists (D-08)"
  - "RUNS_GITIGNORE_BODY: the six-line ignore body, verified against git 2.43.0, treated as an interface because it is written into other people's repositories"
  - "parent_excludes_run_record: the RESEARCH §1.3 detection predicate, reading the reported pattern rather than the exit status because check-ignore inverts on negations"
  - "write_run_record: the config.rs atomic idiom, pretty-serialised, mode 0644 under unix (D-05, D-06, RESEARCH §1.4)"
  - "write_active_pointer / clear_active_pointer / read_active_run, with the directory listing documented and enforced as authoritative"
  - "JournalWriter per-run byte cap: one JournalTruncated notice, then content stops and lifecycle/decision/outcome events continue forever (D-31)"
  - "prune_runs: retention at run start that never touches the active run, an unfinished run, or a run with no record (D-32)"
  - "crash_test_writer_loop + CRASH_TEST_PLANTED_KEY/HOME: the library-side child plan 16-06's crash harness re-execs"
affects: [16-06, 17-detached-driver, 18-driver-tab, 20-drpev-router]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Verify a git property by staging the repository (git add -A + git ls-files), never by asking git whether a pattern matched"
    - "An ignore body written into third-party repositories is an interface: idempotent write, character-for-character test against a literal held in the test body"
    - "A diagnostic that shells out returns a bool predicate (assertable from an integration test) with the tracing::warn! as a thin public wrapper"
    - "A cap constant paired with a with_cap() builder, so the breach is reachable in a test without producing the production volume"

key-files:
  created:
    - tests/journal_gitignore.rs
  modified:
    - src/journal/writer.rs

key-decisions:
  - "JournalWriter::append returns SUPPRESSED_SEQ (0) for an event the cap suppressed — unambiguous because seq is monotonic from 1, and it keeps 16-01's Result<u64> signature intact"
  - "with_cap() is the configurability seam D-31/RESEARCH §10 already called for, and is how the cap tests avoid writing 64 MiB"
  - "prune_runs reads run.json as a serde_json::Value to answer only the ended_at question, so a later schema stays prunable (D-30's tolerance applied to a write path)"
  - "retain counts complete, inactive runs; unfinished runs sit outside the budget rather than consuming it"
  - "crash_test_writer_loop disables the cap, because a suppressed content event would spin the loop with no I/O and the crash harness needs bytes for its whole kill window"
  - "write_runs_gitignore and warn_if_parent_excludes are private per the plan's symbol list; parent_excludes_run_record is the public half, because a tracing::warn! cannot be asserted on from an integration test"

patterns-established:
  - "The interface-vs-implementation distinction is written above the constant it applies to, with the two facts a reader would otherwise get wrong"
  - "An integration test that shells out to git isolates GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM to a nonexistent path, so a developer's global config cannot change the measured answer"
  - "A skip prints its reason, so a green run that skipped is distinguishable from one that ran"

requirements-completed: [OBS-01, SAFE-04]

coverage:
  - id: D1
    description: "Staging a real repository tracks runs/.gitignore and both run records at the documented depth, and tracks none of the journals, inbox files, active pointer, or a record one level deeper"
    requirement: SAFE-04
    verification:
      - kind: integration
        ref: "tests/journal_gitignore.rs#staging_a_real_repo_tracks_run_json_and_ignores_the_journal"
        status: pass
    human_judgment: false
  - id: D2
    description: "A driven repository that has excluded .planning/meta-manager/ as a directory is detected by the same predicate the warning uses, and the detection agrees with what staging measures"
    requirement: OBS-01
    verification:
      - kind: integration
        ref: "tests/journal_gitignore.rs#a_parent_that_excludes_the_directory_is_detected"
        status: pass
      - kind: integration
        ref: "tests/journal_gitignore.rs#a_parent_that_excludes_by_file_glob_is_harmless"
        status: pass
    human_judgment: false
  - id: D3
    description: "The ignore file is written on first call and a second call leaves its bytes and its modification time untouched, so a user's edit survives and no run churns the file"
    requirement: SAFE-04
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#the_ignore_file_is_written_once_and_not_rewritten"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#the_ignore_body_is_exactly_the_four_verified_patterns"
        status: pass
    human_judgment: false
  - id: D4
    description: "run.json round-trips through an atomic write, leaves no temporary litter, and is mode 0644 under unix so persist's preserved 0600 cannot return"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#a_run_record_is_written_atomically_and_is_world_readable"
        status: pass
    human_judgment: false
  - id: D5
    description: "A journal past its per-run cap contains exactly one truncation record, no content event after it, and still writes its terminal record as the last line"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#the_per_run_cap_emits_one_truncation_notice_then_only_lifecycle_events"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#a_terminal_record_is_still_written_after_the_cap_is_reached"
        status: pass
    human_judgment: false
  - id: D6
    description: "Retention keeps the newest complete runs and removes the rest, never the active run, never a run with no end timestamp, never a run with no record at all, and never a directory that is not a run — asserted on both the returned ids and the filesystem"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#pruning_keeps_the_newest_runs_and_never_the_active_one"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#pruning_never_removes_a_run_with_no_end_timestamp"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#only_a_d02_shaped_directory_name_parses_as_a_run_id"
        status: pass
    human_judgment: false
  - id: D7
    description: "The active pointer is written with exactly the id and one newline, is ignored when it names a directory that does not exist, and clears idempotently"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#the_active_pointer_is_written_cleared_and_never_trusted_over_the_listing"
        status: pass
    human_judgment: false
  - id: D8
    description: "Whether 64 MiB and 10 retained runs are the right budget for real four-hour runs on real projects"
    requirement: OBS-01
    verification: []
    human_judgment: true
    rationale: "RESEARCH §10 states plainly that no tuning data exists. Both constants carry their reasoning and an explicit admission of it, and MAX_RUN_JOURNAL_BYTES now has a with_cap() override, so tuning is a one-line change once real runs produce evidence. Adequacy is a judgement against observed runs, not a test."

# Metrics
duration: 22 min
completed: 2026-07-29
status: complete
---

# Phase 16 Plan 03: Run Directory Durability, Growth Bounds & Retention Summary

**A run directory that carries its ignore posture from the moment it exists, a `run.json` written atomically exactly twice at mode 0644, a journal that announces its cap once and still writes its ending, retention that refuses to eat an unfinished run — and an integration test that proves the ignore posture by staging a real repository rather than by asking git a question it answers backwards.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-07-29T14:49:00Z
- **Completed:** 2026-07-29T15:11:00Z
- **Tasks:** 3
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- **The ignore posture is proved by ground truth, not by the question that inverts.** `staging_a_real_repo_tracks_run_json_and_ignores_the_journal` builds the layout through `create_run_dir` and `write_run_record`, runs `git add -A`, and asserts on the `git ls-files` set in **both directions**: 3 paths must be present (the runs ignore file, both records at the documented depth) and 6 must be absent (both journals, both inbox files, the active pointer, and a record one directory level deeper). A test that asserted only presence would pass against an ignore file that ignores nothing.
- **Both of RESEARCH §1's traps have a test that would catch them.** The `check-ignore` inversion is defended by construction — no executable line in either file asks git whether a pattern matched — and the parent-excludes-the-directory failure mode is reproduced against a real repository, with the detection predicate asserted to agree with what staging measured. The benign file-glob case is pinned too, so a future over-broad diagnostic cannot start crying wolf.
- **The 0600 surprise cannot return.** `write_run_record` mirrors `config.rs:71-87` with `anyhow::Context` on every step, sets 0o644 under `cfg(unix)` before `persist`, and the test asserts the persisted mode bits — plus that the run directory holds nothing but `run.json` after two writes, so no temporary litter survives either.
- **The cap's asymmetry is asserted as a count, not a presence.** Past the cap the journal holds **exactly one** truncation record, no `exec_event` after it, and the `run_ended` record is still the last line of the file. The truncation record carries both the bytes written and the cap, so a reader never has to guess the build's constant.
- **Retention refuses to destroy crash evidence.** `prune_runs` skips the active run, any run whose `run.json` has no `ended_at`, any run with no record at all, and any directory whose name is not a D-02 run id. Both tests assert on the returned ids *and* on the filesystem, so a prune that reported work it did not do would fail.
- **The crash-test child is real library code.** `crash_test_writer_loop` is `pub` and not `cfg(test)` — verified by a green `cargo build --release` — and writes through the ordinary `append` path, so it does not reopen the `RedactedLine` seam and plan 16-06 can assert redaction on the bytes that survive a `SIGKILL`.
- **Zero new dependencies, and the lint budget did not move.** `Cargo.toml`/`Cargo.lock` untouched; `cargo clippy --all-targets` still reports exactly the same 5 pre-existing lints.

## Task Commits

Each task was committed atomically:

1. **Task 1: The run directory, its ignore posture, and the twice-written run record** — `1eadc96` (feat)
2. **Task 2: Growth bounds and retention** — `9fee1e5` (feat)
3. **Task 3: Prove the ignore posture against a real repository** — `2264f7a` (test)
4. **Follow-up: plan-faithful visibility split + two doc links** — `f4beaa3` (refactor)

## Files Created/Modified

- `src/journal/writer.rs` (modified, +947 lines) — `RUNS_GITIGNORE_BODY`, `write_runs_gitignore`, `create_run_dir`, `parent_excludes_run_record`, `warn_if_parent_excludes`, `write_active_pointer`, `clear_active_pointer`, `read_active_run`, `write_run_record`, `prune_runs`, `parses_as_run_id`, `has_end_timestamp`, `crash_test_writer_loop`, the two planted constants, `SUPPRESSED_SEQ`, and `JournalWriter`'s `cap` / `truncated` / `suppressed_content` fields with `with_cap`, `truncated()`, `suppressed_content_events()` and `append_suppressed_diagnostic()`. Ten new unit tests.
- `tests/journal_gitignore.rs` (created, 280 lines) — the banner explaining why this cannot be in-source, the isolated-git helpers, and the three tests.

## Decisions Made

1. **`append` returns `SUPPRESSED_SEQ` (0) for a suppressed event.** Plan 16-01 fixed the signature as `anyhow::Result<u64>` returning the seq used, and a suppressed event consumes no seq. Zero is unambiguous because `seq` is monotonic from 1, so no written record can carry it, and it is a named public constant rather than a bare literal. The alternative — changing the signature to `Result<Option<u64>>` — would have churned 16-01's committed API for no additional information.

2. **`with_cap()` rather than a hard-wired `MAX_RUN_JOURNAL_BYTES`.** D-31 and RESEARCH §10 both say the cap is "a defensible starting value, made configurable"; this is that seam. It is also the only way to reach the breach in a unit test without writing 64 MiB, and it is what lets `crash_test_writer_loop` keep producing bytes for its whole kill window.

3. **`retain` counts complete, inactive runs.** D-32 says "keeping the last N complete runs", so unfinished and active runs sit outside the budget rather than consuming it. The alternative reading — counting every directory — would silently shrink the retained history each time a run crashed, which is the opposite of what a crash-reconciliation phase wants.

4. **`has_end_timestamp` reads `run.json` as a `Value`, not a `RunRecord`.** The only field the question needs is `ended_at`, and a record written by a later schema must stay prunable. This is D-30's tolerance discipline applied to a write path.

5. **`parses_as_run_id` parses rather than pattern-matches.** The stamp is fed back to the same calendar that produced it, so `2026-13-45T99-99-99Z-0000` is rejected and a directory a user dropped into `runs/` by hand is never mistaken for a run.

6. **The `active` pointer's authority rule is mechanical, not documentary.** `read_active_run` returns the id **only if** a directory of that name exists, and logs a `tracing::warn!` on disagreement. This is the resolution of CONTEXT's open discretion: the file is written because it is cheap to poll and useful to Phase 17, but D-02's sort property means a listing answers "which run" without reading anything, and a listing cannot go stale after a crash.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The per-run cap is unreachable in a test without a configurable cap**

- **Found during:** Task 2
- **Issue:** The plan's acceptance criterion requires driving the writer past `MAX_RUN_JOURNAL_BYTES` "by at least 10 further content events". That constant is 64 MiB; a unit test that reached it honestly would write 64 MB to a temp directory on every `cargo test` run — slow enough that it would be `#[ignore]`d within a week, which RESEARCH Pitfall 7 names as the failure mode that leaves a criterion with no verification at all.
- **Fix:** Added `JournalWriter::with_cap(cap)`, a consuming builder that overrides the per-run cap. This is not new scope — D-31 and RESEARCH §10 both specify the cap as "made configurable", and this is the seam that makes it so. The cap tests use 200 bytes; production still defaults to `MAX_RUN_JOURNAL_BYTES`.
- **Files modified:** `src/journal/writer.rs`
- **Verification:** `the_per_run_cap_emits_one_truncation_notice_then_only_lifecycle_events` and `a_terminal_record_is_still_written_after_the_cap_is_reached` both green in 0.02s.
- **Committed in:** `9fee1e5` (Task 2 commit)

**2. [Rule 2 - Missing critical functionality] The crash-test loop would have gone silent after 64 MiB**

- **Found during:** Task 2
- **Issue:** `crash_test_writer_loop` writes `ExecEvent` records, which are content events. Once the per-run cap fires, every subsequent append is suppressed — so the loop would spin at full CPU producing **no bytes at all**, and plan 16-06's crash harness would assert redaction against a file that stopped growing before the kill. The failure would look like a passing test.
- **Fix:** The loop opens its writer with `.with_cap(u64::MAX)`, and the reason is stated in its doc comment so nobody removes it as redundant.
- **Files modified:** `src/journal/writer.rs`
- **Verification:** `cargo build --release` green (the function is genuinely not `cfg(test)`); the loop's cap override is one line above the loop it protects.
- **Committed in:** `9fee1e5` (Task 2 commit)

**3. [Rule 1 - Bug] Two new intra-doc links did not resolve**

- **Found during:** post-Task-3 verification
- **Issue:** `cargo doc --no-deps` reported 5 warnings against a pre-existing baseline of 3: a bare `[`append_suppressed_diagnostic`]` inside an inherent-method doc does not resolve without a `Self::` qualifier, and `create_run_dir`'s public doc linked to `warn_if_parent_excludes` after that function was made private.
- **Fix:** Qualified the first, repointed the second at the public predicate. `cargo doc` is back to its 3 pre-existing warnings, none in `src/journal/`.
- **Files modified:** `src/journal/writer.rs`
- **Verification:** `rtk proxy cargo doc --no-deps` lists exactly the 3 pre-existing warnings (`claude.rs:158`, `claude.rs:507`, `ui/screens/mod.rs:83`).
- **Committed in:** `f4beaa3`

### Recorded, Not a Fix

**`write_runs_gitignore` and `warn_if_parent_excludes` are private.** The plan's symbol list writes these two without `pub` while every other new symbol carries it — a deliberate distinction, and the correct one: `create_run_dir` is the only moment at which writing the ignore file is correct (D-08), so there is no second caller to serve. Task 3's requirement to expose an assertable predicate is met by `parent_excludes_run_record`, which is `pub`; the `tracing::warn!` remains the user-facing surface. They were briefly `pub` between Task 1 and the follow-up commit; `f4beaa3` corrects that.

---

**Total deviations:** 3 auto-fixed (1 bug, 1 missing critical functionality, 1 blocking), 1 recorded
**Impact on plan:** None on scope or shape. Deviation 2 is the one that mattered: without it plan 16-06's crash assertion would have been silently vacuous, which is precisely the class of failure this phase's RESEARCH keeps flagging. `git diff --stat` against the base touches exactly the two declared files.

## Issues Encountered

None that required a fix cycle. Every test passed on its first run; the only iteration was the deliberate visibility correction and the two doc links that followed from it.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `cargo build --release` | clean (proves `crash_test_writer_loop` is not test-gated) |
| `rtk proxy cargo test --lib journal::writer` | 13 passed, 0 failed |
| `rtk proxy cargo test --test journal_gitignore` | 3 passed, 0 failed |
| `rtk proxy cargo test` (full suite) | **404 passed**, 0 failed (346 lib + 11 + 7 + 3 + 12 + 25) — base commit was 391; +13, none removed |
| `cargo clippy -- -D warnings` | exits 0 |
| `rtk proxy cargo clippy --all-targets` lint count | exactly **5**, all pre-existing, none in `src/journal/` or `tests/journal_gitignore.rs` |
| `cargo doc --no-deps` | 3 warnings, all pre-existing, none in `src/journal/` |
| `git diff --stat 62b8e32..HEAD` | `src/journal/writer.rs` + `tests/journal_gitignore.rs` only |
| `Cargo.toml` / `Cargo.lock` | unchanged — zero new dependencies |
| `grep -v '^[[:space:]]*//' src/journal/writer.rs \| grep -c 'check-ignore -q'` | `0` (RESEARCH §1.2, Pitfall 1) |
| `grep -c 'NamedTempFile' src/journal/writer.rs` | `3` (D-05) |
| `grep -v '^[[:space:]]*//' src/journal/writer.rs \| grep -c '\.tmp'` | `0` — the `queue_md.rs` fixed-temp-name idiom is not copied |
| `grep -c 'crash_test_writer_loop' src/journal/writer.rs` | `2` |
| `grep -B5 'pub fn crash_test_writer_loop' src/journal/writer.rs \| grep -c 'cfg(test)'` | `0` |
| `grep -v '^[[:space:]]*//' tests/journal_gitignore.rs \| grep -c 'check-ignore'` | `0` — no executable line asks git whether a pattern matched |
| `grep -c 'git add -A' tests/journal_gitignore.rs` | `2` |
| `grep -c 'ls-files' tests/journal_gitignore.rs` | `2` |
| `grep -c 'create_run_dir' tests/journal_gitignore.rs` | `2` — the fixture is built by the library, so the ignore body cannot drift from its test |
| `warn_if_parent_excludes` body | no `?`, no `panic!`/`unwrap`/`expect`; git absent or a non-repository is silent |

## Known Stubs

None. Every symbol this plan introduces has a caller or a test, and nothing is a placeholder awaiting a later phase. Two forward-looking surfaces are documented as such rather than stubbed: `crash_test_writer_loop` exists for plan 16-06's harness and has no production caller by design, and `append_suppressed_diagnostic` is exposed for whichever phase owns the terminal sequence (Phase 17) because the writer does not itself know when a run has ended.

## Threat Flags

None. Every file touched is inside the plan's declared `<threat_model>` scope, and the six registered threats are addressed as planned:

- **T-16-12** (journal reaching a commit) — the ignore file lands at run-directory creation, before `JournalWriter::open` is legal, and its effect is proved by staging.
- **T-16-13** (mid-run `run.json` swept into a commit) — D-06's exactly-twice contract is stated on `write_run_record`, carrying D-07's Aider-hazard reasoning and Phase 20's forward constraint verbatim.
- **T-16-14** (record readable by a second uid) — accepted and made deliberate: mode 0644 is asserted, and the journal, which holds agent content, is not mode-adjusted.
- **T-16-15** (unbounded growth) — the per-run cap plus retention, both tested.
- **T-16-16** (destroying crash evidence) — `prune_runs` skips the active run, an absent `ended_at`, and an absent record entirely.
- **T-16-17** (goal not legible after a clone) — the parent-exclusion diagnostic, reproduced against a real repository.

No package-manager installs occurred.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase 17's run-start sequence is fully served.** `create_run_dir` → `write_run_record` (write one) → `write_active_pointer` → `JournalWriter::open`, with `prune_runs` called at run start, is the whole shape; the ignore file is already in place by the time the writer can legally open.
- **Phase 17's crash reconciliation has its signal.** A run directory whose `run.json` has no `ended_at` is guaranteed to still be on disk, because retention refuses to remove it, and `read_active_run` will not hand back a pointer to a directory that is gone.
- **Plan 16-06 can re-exec `crash_test_writer_loop` immediately**, asserting against `CRASH_TEST_PLANTED_KEY` and `CRASH_TEST_PLANTED_HOME` rather than retyped copies.
- **One note for whoever wires the terminal sequence:** `append_suppressed_diagnostic()` must be called once before the `RunEnded` append if the count is to appear at all — the writer cannot know when a run ends.
- No blockers.

## Self-Check: PASSED

- `src/journal/writer.rs` and `tests/journal_gitignore.rs` — both present on disk.
- Commits `1eadc96`, `9fee1e5`, `2264f7a`, `f4beaa3` — all four found in `git log`.
- Every task `<acceptance_criteria>` re-run after the final code commit; all pass verbatim, with no substitutions needed.
- Plan-level `<verification>` re-run: full suite 404 green, `journal_gitignore` green (git was available, so nothing skipped), `clippy -- -D warnings` exits 0, `--all-targets` lint count exactly 5, `git diff --stat` touching only the two declared files.
- `STATE.md` and `ROADMAP.md` deliberately untouched: the orchestrator owns those writes after the wave merges.

---
*Phase: 16-run-journal-state-substrate*
*Completed: 2026-07-29*
