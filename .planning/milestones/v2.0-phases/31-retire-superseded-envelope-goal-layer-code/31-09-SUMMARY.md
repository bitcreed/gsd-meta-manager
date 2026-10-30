---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 09
subsystem: launcher-release-gate
tags: [rust, gap-closure, release-gate, pre-tag-check, launcher, safe-04, determinism]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-08 fixed WR-01..03; 31-05 recorded the two container-only failures (open gate)
provides:
  - planted-file walks that visit entries in file-name order and truncate whole, so a snapshot is a function of the tree (SC4 gap)
  - a sandbox test that pins its own fixture's runtime dir instead of reading the host's XDG_RUNTIME_DIR (SC4 gap)
  - last_line_of_tail: the stderr capture never returns the partial first line of a cut 4 KiB window (WR-05, SAFE-04)
  - redact_captured caps a persisted line at 512 chars after redaction (WR-05)
  - a ledger refusal that states the size bound and names no time limit (WR-04)
  - a green six-gate `./scripts/pre-tag-check.sh --container v1.15.0` (exit 0), cold
affects: [31-07, verification]

actuals:
  tokens: 21000
  tasks: 3
  commits: 5

plan_head_before: 5078bf55097af07c7cb83715b1721be578971a03
plan_head_after: 69d371d66cecd0c3f990e31e2fdc7fd47a11047c
commits: 5

key-files:
  created: []
  modified:
    - src/launcher/push/planted.rs
    - tests/launcher_sandbox.rs
    - src/launcher/process.rs
    - src/launcher/push.rs
    - src/envelope/ledger.rs
    - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-REVIEW-DISPOSITION.md

key-decisions:
  - "Planted-order failure is a product property: fixed in production code (name-ordered, listing-bounded walks), not by making the test order-insensitive [inferred - audit]"
  - "Snapshot::VERSION is not bumped; the in-flight-baseline residual is accepted [inferred - audit]"
  - "A cut stderr window with no complete line after the first line break yields the fixed note, not an empty string [inferred - audit]"
  - "COUNT_BUDGET_SECS lives in the ledger test module: the envelope_pr_cap source-slice pin allows exactly one cfg(test) sentinel in ledger.rs [inferred - audit]"

requirements-completed: [RETIRE-01, RETIRE-03]

duration: about 2h (dominated by the cold container build and suite)
completed: 2026-10-27
status: complete
---

# Phase 31 Plan 09: SC4 container-gate gap closure Summary

**Both container-only launcher tests are fixed at the source (a name-ordered, bounded planted-file walk and a self-supplied runtime dir), the stderr capture can no longer persist a half-redacted token, and `./scripts/pre-tag-check.sh --container v1.15.0` exits 0 with gates 1-6 PASS.**

## Commits

| Commit | Subject |
|--------|---------|
| `6de4572` | fix(31-09): make both container-only launcher tests host-independent (SC4) |
| `21b94df` | fix(31-09): stderr capture cannot leak a window-cut token; ledger refusal states its real reason (WR-05, WR-04) |
| `832c58f` | docs(31-09): plain-code the private items in run_bounded_with_stderr's doc |
| `a51b39a` | fix(31-09): keep the ledger's sizing constant in its test module (one cfg(test) sentinel) |
| `69d371d` | docs(31-09): record WR-01..WR-05 fixed after the green container gate (open 11 -> 6) |

BASE for the sandbox/run.rs no-diff check: `5078bf5`. `git diff --quiet 5078bf5 -- src/launcher/sandbox.rs src/launcher/run.rs` exits 0.

## Task 1: both container-only failures (SC4 gap)

**Investigation outcome (planted ordering).** Confirmed as planned: the listing order reaches the baseline comparison, so it is a product property. `walk_tracked_dir` and `walk_nested_git` popped a LIFO stack over raw `read_dir` order, so under truncation the recorded set (and hence a push-blocking nested `.git` finding) depended on the file system. Fixed in production code: one private helper, `list_dir_bounded`, reads at most `room + 1` entries, reports `Unreadable` / `OverBudget` / name-sorted `Entries`; both walks use it and push subdirectories in reverse name order so the stack pops them ascending. `record_root_git`, the per-entry `budget.take()`, the `.git` / `SKIP_DIR_NAMES` / symlink rules and every blocking rule are untouched.

**Budget count for the fixture (limit 10), re-counted, matches the planner's:** tracked dirs 2 (`.git/hooks/pre-commit`, `.claude/agents/a.md`); root listing 3 (`.claude`, `.git`, `flood`) = 5; `.claude` 2 = 7; `.claude/agents` 1 = 8; `.claude/worktrees` 1 = 9; `.claude/worktrees/w` 1 = 10 (records `.claude/worktrees/w/.git`); `flood/` 40 entries > 0 left, so the walk truncates. The limit stayed 10 and no `assert_eq!` expectation changed; a comment with this count was added.

**RED results (quoted).**
- New `a_truncated_walk_records_the_same_entries_in_any_directory_order` against the unsorted walk (this machine's tmpfs):
  `assertion left == right failed: limit 13: the snapshot depends on listing order` with left `git_entries: {".git": Absent, "n00/.git": File(..)}` and right `{".git": Absent, "n11/.git": File(..)}`.
- `rtk proxy env -u XDG_RUNTIME_DIR cargo test --no-fail-fast --test launcher_sandbox` at HEAD: `no_flag_names_a_credential_directory_and_the_home_block_precedes_the_run_binds` panicked at `tests/launcher_sandbox.rs:305:32`: `--tmpfs /run/user/1000 missing: ["bwrap", ...]`.

**GREEN.** planted tests: 21 passed (includes the new test, `a_flood_of_files_is_a_truncation_finding_not_silence`, `a_truncation_the_baseline_had_too_is_listed_but_does_not_block_the_push`). `launcher_sandbox`: 15 passed, 1 ignored (the pre-existing opt-in live smoke) under both `env -u XDG_RUNTIME_DIR` and `XDG_RUNTIME_DIR=/nonexistent/run/user/4242`. The sandbox test now sets `spec.runtime_dir = Some(PathBuf::from(RUNTIME_DIR))` (a `const` naming `/run/user/1000` once) with a comment; `sandbox.rs` and `run.rs` are untouched.

## Task 2: WR-05 and WR-04

**RED results (quoted).**
- `run_bounded_with_stderr_never_returns_a_token_cut_by_the_window`: `a token fragment "GHIJKLMN" leaked: GHIJKLMNOPQRSTUVWXYZ012345yyyy...` (payload: 100 x, space, 36-byte token, 4070 y; the window starts 10 bytes into the token).
- `a_long_captured_line_is_redacted_before_it_is_capped`: failed on the `<= MAX_CAPTURED_LINE_CHARS` assertion (the line came back uncapped, 529 chars).
- `t_19_117_unbounded_work_is_bounded`: `the refusal must not name a time limit ("deadline"): ... past the 8388608-byte bound the push step can count inside its 5-second deadline ...`.

**GREEN.**
- process.rs: `last_line_of_tail(tail, window_cut)` drops everything up to and including the first `\n` of a cut window; no line break (or no complete line left) returns `OVERLONG_STDERR_NOTE`. `run_bounded_with_stderr` passes `len > STDERR_TAIL_BYTES`. New `mod tests` with the four named tests.
- push.rs: `MAX_CAPTURED_LINE_CHARS = 512`; `redact_captured` caps after `redact_urls` and `journal::redact::redact`. The cap test uses 500 x, a space and a runtime-assembled token, so a cap applied first would keep exactly 7 secret chars that no pattern matches; the assertion is that no 7-char prefix survives.
- ledger.rs: the refusal now says "past the N-byte bound up to which the push step counts a ledger" and names no time limit. `COUNT_DEADLINE_SECS` became `COUNT_BUDGET_SECS`; the stale "retired guard" and "journal::reader" docs were rewritten (the ledger is outside the sandbox `$HOME` allowlist under Phase 28.1, T-19-112 closure). `MAX_LEDGER_BYTES` is still `8 * 1024 * 1024` (line 135) and the kind bound is untouched.

Acceptance greps: `deadline` 0, `COUNT_DEADLINE_SECS` 0, `retired guard` 0, `journal::reader` 0 in ledger.rs; `fn last_line_of_tail` and `mod tests` present in process.rs; `MAX_CAPTURED_LINE_CHARS` 4 occurrences in push.rs; no new `#[ignore]` (planted.rs 0; launcher_sandbox.rs 1, the pre-existing live smoke).

## Task 3: the done gate

**FULLSUITE** (the plan's exact command):

    FULLSUITE results=57 passed=3294 failed=0 ignored=17
    expected results=57

31-05's end line was `results=57 passed=3290 failed=0 ignored=17`. Passed delta +4: 31-09 added 6 tests (1 planted, 4 process, 1 push; the ledger test was changed, not added), and 31-08 net -2 (removed the six disclosure tests; added `unregistering_a_project_with_a_live_launcher_run_is_refused`, `a_project_whose_launcher_runs_are_not_live_is_still_removed`, `the_grant_popup_shows_no_disclosure_of_retired_seams`, `the_opt_in_prompts_say_what_the_record_feeds_and_claim_no_gate`). No `editor_race_smoke` flake appeared locally.

A first FULLSUITE run (before the final commit) reported `passed=3293 failed=1`: `envelope_pr_cap::the_link_non_following_stat_variant_is_absent_from_ledger_rs_production_half` failed its "exactly one `#[cfg(test)]` sentinel" pin, because my first `COUNT_BUDGET_SECS` carried a `#[cfg(test)]` of its own. Fixed in `a51b39a` by moving the constant into the existing test module (Rule 1, caused by this task's change); the pin itself was not touched.

**Clippy.** `rtk proxy cargo clippy --all-targets --locked -- -D warnings` exit 0. Cross-target loop (`x86_64-pc-windows-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `--locked --all-targets -- -D warnings`): all three exit 0.

**Container dry run** `./scripts/pre-tag-check.sh --container v1.15.0`, exit code **0** (inner run exited 0):

    gate 1  tag / Cargo.toml version   PASS
    gate 2  MSRV check                 PASS
    gate 3  cargo build --release      PASS
    gate 4  cargo test                 PASS
    gate 5  cargo clippy -D warnings   PASS
    gate 6  cargo package --locked     PASS

The run was **cold**: the prep step measured `gsdmm-pretag-target` at 23 GB (cap 20 GB) and wiped it (`PRE_TAG_CONTAINER_CACHE_MAX_GB` was not raised). Post-run volumes: target 11 GB, cargo-home 213 MB. Gate 4's log shows 57 `test result: ok` lines and none with failures; it contains `a_truncation_the_baseline_had_too_is_listed_but_does_not_block_the_push`, `a_truncated_walk_records_the_same_entries_in_any_directory_order`, `no_flag_names_a_credential_directory_and_the_home_block_precedes_the_run_binds` and `run_bounded_with_stderr_never_returns_a_token_cut_by_the_window`, all `ok`; `editor_race_smoke` passed in-container, so no warm re-run was needed.

**No tag, publish or push.** `git tag --points-at HEAD` is empty; nothing was pushed or published; plan 31-07 (HELD, D-04) was not touched.

**Disposition record.** `31-REVIEW-DISPOSITION.md`: WR-01..WR-05 `fixed` (frontmatter and table, each with a fixing sha: WR-01 `12b8a1d` + `89f4b3b`, WR-02 `89f4b3b`, WR-03 `36e0805`, all from 31-08; WR-04 `21b94df` + `a51b39a`, WR-05 `21b94df`, from this plan), `open: 6`, IN-01..IN-06 still `open`.

## Report to the coordinator (no memory edited)

- The pre-tag container image exposed two host-dependent tests that a local run could not (one on directory listing order, one on a real `/run/user/<uid>`). Both are now fixed at the source, without skipping, `#[ignore]` or weakening an assertion.
- RETIRE-03's container clause is ready for the orchestrator to flip under D-17.
- **Accepted residual (planted baseline) `[inferred - audit]`:** a run launched before this change keeps a baseline written under the old walk order. If that checkout is larger than the 1,000,000-entry walk (both snapshots truncated), the first push after the upgrade may list created/removed entries the old order recorded; nested `.git` ones block until `--ack-planted`. `Snapshot::VERSION` was NOT bumped, because bumping it would send every in-flight run to strict mode, which is broader.

## Deviations from Plan

**1. [Rule 1 - Bug] Test-only constant carried a second `#[cfg(test)]` sentinel.**
- **Found during:** Task 3 FULLSUITE.
- **Issue:** `#[cfg(test)] const COUNT_BUDGET_SECS` made `envelope_pr_cap`'s exactly-one-sentinel production-slice pin fail.
- **Fix:** moved the constant into `ledger.rs`'s test module; its doc keeps the "sized against, not enforced" statement.
- **Commit:** `a51b39a`.

**2. [Rule 2 - small addition] A cut window with no complete line left returns the note.** `last_line_of_tail` returns `OVERLONG_STDERR_NOTE` not only when the cut tail has no `\n` but also when nothing non-empty follows the first one (the last line is then the cut one). Returning an empty string there would have been silent, not wrong, but the note says why. `[inferred - audit]`

**3. [Rule 3 - rustdoc hygiene]** `run_bounded_with_stderr`'s public doc referred to two private items with intra-doc links, which adds rustdoc private-link warnings; those two references are plain code spans (`832c58f`). The pre-existing `STDERR_TAIL_BYTES` link was left alone.

**Total deviations:** 3, none changing behaviour beyond the plan.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or schema change. T-31-29..33 are mitigated as planned (cut-token and cap-ordering tests observed RED first; order-independence test pins the walk; at most `room + 1` entries buffered per directory; both tests proven under unset and nonexistent `XDG_RUNTIME_DIR` and a reversed creation order; ledger message pinned).

## Self-Check: PASSED

- Files exist: `src/launcher/push/planted.rs`, `tests/launcher_sandbox.rs`, `src/launcher/process.rs`, `src/launcher/push.rs`, `src/envelope/ledger.rs`, `31-REVIEW-DISPOSITION.md`.
- Commits `6de4572`, `21b94df`, `832c58f`, `a51b39a`, `69d371d` are ancestors of HEAD.
- Acceptance greps hold (see Task 2 list; `grep -c 'disposition: fixed'` is 5 and `grep -c '^open: 6$'` is 1 in the disposition file).
- `git tag --points-at HEAD` is empty.
