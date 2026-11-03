---
quick_id: 261103-f5i
phase: quick-261103-f5i
plan: 01
subsystem: gsd-core-sync
tags: [gsd-core, sync, config, defaults-tab, planner]
status: complete
requirements: [QUICK-261103-f5i]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
commits: 4
plan_head_before: 6933d8dedd7c054643af571da572be40204d3ecc
plan_head_after: ce4d2fd2f17facc3d45da48fc218f5151f35d633
branch: quick/261103-f5i
completed: 2026-11-03
actuals:
  tasks: 3
  commits: 3
---

# Quick 261103-f5i: gsd-core v1.20.0 sync Summary

Baseline moved from v1.19.0 to v1.20.0 (trio 1.20.0 / v1.20.0 / 1.20.0, README note, sync record), the two planner stall knobs promoted from pass-through to typed Integer rows, and three Defaults help texts corrected.

## Commits (branch `quick/261103-f5i`, master refused by the pre-commit guard)

| Task | Commit | Files |
|---|---|---|
| 1 feat: planner stall rows | `891a472` | config_json.rs, detail.rs, docs/GSD-CORE-SYNC.md |
| 2 fix: three help texts | `2ee44e8` | detail.rs |
| 3 chore(sync): baseline v1.20.0 | `ce4d2fd` | config_json.rs, README.md, docs/GSD-CORE-SYNC.md |

`git rev-list --count 6933d8d..HEAD` is 4, not 3: commit `7b25355 "Update deps"` (Cargo.lock only, authored by the user at 11:10:00) landed on this branch between tasks 1 and 2. It is not mine; see Deviations.

## What changed

- **PlannerConfig** now types all three planner keys (`stall_threshold_minutes`, `stall_detect_interval_minutes`: `Option<u32>`, skipped when unset so a clear removes the key). Doc comment records the reversal of 260926-gtk I-3.
- **Defaults rows**: two Integer rows after `planner.stall_detection_enabled` (threshold first), `since v1.10.0`, built-in defaults 10 and 5, cycle +1 wrapping to 1 after 30 / 15 (never 0), clear arms. `DEFAULTS_OPTION_COUNT` 136 -> 138. New `PROMOTED_PLANNER_KEYS_1_20_0` (len 2) iterated by both per-kind resync loops.
- **Help texts**: `workflow.inline_plan_threshold` (inline execution, 0 always spawns, default 2), `phase_naming` (sequential default, custom case-insensitive, other values keep numeric allocation), `phase_id_convention` (adds bracket). Kinds, `since` and built-in defaults unchanged.
- **Baseline**: constants, doc comments (incl. full tag sha b14172e3...), README note, sync record header/table/history, `REF=v1.21.0`/`PREV=v1.20.0`, new `## The v1.20.0 delta` section, Pass-through row for `review.context_window_tokens.<slug>`, Next-sync item 3 sentence and new item 8.

## Measurements (Task 3 step 0, all against tags)

- `git describe v1.20.0` = `v1.20.0`; `rev-parse v1.20.0^{commit}` = `b14172e38d02aeab483856949453264228886517`; `npm view @opengsd/gsd-core version` = 1.20.0; `~/.claude/gsd-core/VERSION` = 1.20.0; tree `package.json` 1.20.0; tag date 2026-11-03.
- Key cells 262 -> 263; `comm` added = `review.context_window_tokens` only, removed = none. Planner stall keys no longer appear in `comm -23 core gmm`; `review.context_window_tokens` does (pass-through).
- No diff in `src/config.cts`, `src/secrets.cts`, both manifests, `templates/config.json`, `gsd-run-resolver.md`. `src/config-loader.cts`: 3 ins / 3 del (`env` param on `isGitIgnored`).
- Slugs registering `review.context_window_tokens.<slug>`: 10 (antigravity, claude, codex, cursor, kimi-code, llama_cpp, lm_studio, ollama, opencode, qwen) - matches the plan. 45 capability files changed.
- `since` for both planner keys: `git log v1.20.0 --reverse -S<key>` = 067a4d1c6; `tag --contains` = v1.10.0. `review.context_window_tokens`: a032f9fed, first tag v1.20.0.
- `agents/` 5 files changed; `commands/gsd` only `update.md`; `agents/*.compact*` count 29; codex install-tree fixture adds `cicd-review-patterns.md` and `necessity-review.md`; `grep -rnE 'compact\.md|agents/gsd-' src` hits only the `workflow.compact_content` help string (detail.rs:12349 now).
- Upstream checkout HEAD: branch `next`, `v1.18.0-151-g3730c6704`, package.json 1.19.0 (recipe note updated from 119/be164db40).
- Differences from the plan context: none material. The ~12337 line anchor for the compact_content help is now 12349 (shifted by this work).

## Gates

- `rtk proxy cargo test --no-fail-fast`: exit 0. lib 2600 passed / 0 failed / 1 ignored; main 10 passed; 42 integration suites all `ok`, totals 0 failed; ignored counts are the pre-existing `#[ignore]` tests (live-egress etc.). `tests/driver_router_conformance.rs`: `6 passed; 0 failed; 0 ignored` against installed gsd-core 1.20.0 (ran, not skipped).
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo fmt --check`: clean.
- Each commit was green on its own (lib suite run before commits 1 and 2; full suite before commit 3).

## Inferred decisions [inferred - audit]

- **I-1** Promote both stall knobs to typed `Option<u32>` rows, reversing 260926-gtk I-3. Outcome: done; `since v1.10.0` measured; `executor.stall_*` stays pass-through.
- **I-2** New `PROMOTED_PLANNER_KEYS_1_20_0` instead of a `RESYNCED_KEYS_1_20_0`. Outcome: done; no 1.20 key-cell delta key is typed.
- **I-3** Cycle: unset = 0, +1, wrap to 1 after 30 (threshold) / 15 (interval). Outcome: done; caps are planner-chosen; never writes 0.
- **I-4** Non-integer hand edit = whole-file parse failure (accepted, same as PlannerConfig I-5); `0` is shown verbatim. Outcome: documented in the struct doc comment and the sync record.
- **I-5** Row order threshold then interval directly after the bool. Outcome: asserted by test.
- **I-6** `review.context_window_tokens` stays pass-through (templated row). Outcome: slug count measured 10 (coordinator brief said 9 / listed 10); recorded 10.
- **I-7** `phase_naming` and `phase_id_convention` stay String rows; only help text changed. Outcome: done.
- **I-8** Custom and bracket phase identity deferred; owner next-milestone planning (dedicated phase). Outcome: Next-sync item 8.
- **I-9** Oracle pin moved to 1.20.0 without a `--container` pre-tag run (belongs to the next release). Outcome: local conformance test green against installed 1.20.0.
- **I-10** Three commits (feat, fix, chore(sync)). Outcome: done.
- **Added**: the threshold help was shortened to 149 chars because the existing `every_summary_is_within_the_pane_budget...` test caps summaries at 160 chars (the plan's paraphrase was 204). The two new help summaries for the phase rows are 154 chars.
- **Added**: the "executor writes ROADMAP progress before STATE" and "bracket write paths are new in 1.20" statements were verified from `git diff v1.19.0 v1.20.0 -- agents/gsd-executor.md` and `git log v1.19.0..v1.20.0` (57942161d #5528, 3befe44bd #5186), and #4304 from `src/smart-entry.cts`.
- **Added**: no `set_config_value` arm for the two Integer rows, as the plan directed (Integer rows have none in this build; an arm would be dead code).

## Deviations from Plan

**1. [Rule 1 - Bug] Help summary exceeded the 160-char pane budget.** Found during Task 1: `every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key` failed on the 204-char threshold summary. Fix: shortened to 149 chars, same meaning. Files: src/ui/screens/detail.rs. Commit 891a472.

**2. Pre-commit guard refused master.** `git.base-branch --is-protected master` = true, so work was committed on `quick/261103-f5i` branched from master HEAD 6933d8d (as 261103-em6 did). The tree is left on that branch for the coordinator's `git merge --ff-only`. No rebase, no push, no tag.

**3. Foreign commit on the branch.** `7b25355 "Update deps"` (Cargo.lock only, author Andreas Brauchli, 11:10:00) appeared in the reflog between my tasks 1 and 2. I did not make it and did not touch it. It will ride along with the branch on ff-merge; if it was not intended for this branch, the coordinator should decide before merging. My gate runs after it (full suite at task 3) were green with it applied.

**4. Untracked/modified files not mine.** `.planning/musk/ledger.jsonl` is modified (a hook appends to it) and `.planning/quick/261103-f5y-...` is untracked (pre-existing). Neither was staged.

## Known Stubs

None.

## Threat Flags

None. The planner promotion only adds typed numeric fields on an existing parse/save path (T-f5i-01/02/03 mitigations asserted by tests); the oracle pin was verified against npm before moving (T-f5i-04).

## Deferred Issues

Custom and bracket phase identity (Next sync item 8). No other issues.

## Self-Check: PASSED

- Files: src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md, README.md all exist and are committed.
- Commits 891a472, 2ee44e8, ce4d2fd are ancestors of HEAD (HEAD = ce4d2fd on quick/261103-f5i).
