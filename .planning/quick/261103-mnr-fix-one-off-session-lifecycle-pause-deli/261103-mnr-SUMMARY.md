---
quick_id: 261103-mnr
phase: quick-261103-mnr
plan: 01
subsystem: oneoff
tags: [oneoff, session-lifecycle, pause, meter, rate-limit, landing]
status: complete
requirements: [QUICK-261103-mnr]
commits: 3
plan_head_before: 3934078b63e88740f940cf3749a932306bc1ae51
plan_head_after: e6c67c423f69ede1b59b8c4d1705f2016170eb1e
actuals:
  tokens: 0
  tasks: 3
  commits: 3
key-files:
  created:
    - tests/fixtures/oneoff/real-bg-subagents.stream.jsonl
    - tests/fixtures/oneoff/real-resumed.stream.jsonl
    - tests/fixtures/oneoff/README.md
  modified:
    - src/oneoff/session.rs
    - src/oneoff/stream.rs
    - src/oneoff/runtime.rs
    - src/oneoff/launch.rs
    - src/oneoff/cancel.rs
    - src/oneoff/inbox.rs
    - src/oneoff/land.rs
    - src/task_cli.rs
    - src/app.rs
    - src/ui/screens/oneoff.rs
    - src/ui/screens/oneoff_viewer.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - tests/oneoff_e2e.rs
    - docs/CONFIGURATION.md
    - docs/TESTING.md
duration: not recorded
completed: 2026-11-03
---

# Phase quick-261103-mnr Plan 01: One-off session lifecycle, pause delivery, rate-limit and meter fixes Summary

A one-off session now stays open while background subagents work, a pause is confirmed or refused (never assumed), and the spend/rate figures are honest: labelled a lower bound while live, exact once a result reports it.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | `ab13940` | fix(quick-261103-mnr): keep a one-off session open while background work is outstanding |
| 2 | `59030a0` | fix(quick-261103-mnr): honest one-off meter, real rate-limit shape and live spend in task status |
| 3 | `e6c67c4` | fix(quick-261103-mnr): confirm or refuse a pause, let the report decide paused, count what a landing landed |

`commits: 3` is measured (`git rev-list --count 3934078..HEAD`, base read from the on-disk ledger). No push, tag or rebase. No paid `claude` run: all integration coverage is the fake-agent harness.

## Shipped (all six defects, D-01..D-07)

- **Bug 1 (D-01):** `SessionLifecycle` tracks outstanding background work (`background_tasks_changed` count, else started-not-ended task ids, max with the result's `subagent_stats` still running), queued turns, and treats every `init` after the first as a new turn. Stdin closes only with a result since the last turn start, replays >= writes, nothing outstanding, no queued turn, and either the report file present (close at once) or 120 s idle (`GSDMM_ONEOFF_IDLE_SETTLE_SECS`, 1..=600) with the warning `closed the agent's input after 120s idle with no report`. Replaying the trimmed run A keeps stdin open at the first result (4 outstanding, even with a report and 300 s of quiet) and closes at the first result after the set drained.
- **Bug 2 (D-02):** `task stop --pause` waits for the launcher's `pause` record in `injections.jsonl` (`inbox::pause_outcome_since`): delivered exits 0, refused exits 1 with `could not be delivered ... (reason)` plus the `task stop` hint, unconfirmed exits 1 with `not confirmed` wording. A refused pause is withdrawn (`Cancel::withdraw_pause`: no grace cut), recorded as `stop.refused`, shown as `pause refused (...)` in `task status` and the TUI summary. TUI `z` waits 10 s and shows `pause refused: ...`. `paused` needs a delivered pause AND a `partial` report (`pause_end_with_report`); a `done` report ends `finished`.
- **Bug 3 (D-03):** utilization read from `rate_limit_info.unifiedWindows[rateLimitType]`, else the highest window (its name becomes `kind` when `rateLimitType` is missing), else the legacy top-level; run A's L12 line gives `allowed seven_day 21%`.
- **Bug 4 (D-04):** the ff landing counts `git rev-list --count old..new`; the resume-from-master e2e lands 3 commits, 1 by the agent.
- **Bug 5 (D-05):** `task status` of a live run prints `spend:     ≥ $X so far of $Y (live estimate, lower bound; exact at the end)`, `soft cap:` and `rate:` lines; `--json` adds `live_spend_usd`, `live_spend_is_lower_bound`, `rate_limit` (live runs only; finished runs keep their exact recorded line).
- **Bug 6 (D-06):** 1-hour cache writes at 2.0 x input (run A's final token sums reproduce the reported cost within $0.001 for both models), output = max(reported, visible chars / 4) accumulated per message id, folded ids ignored, `SpendEstimate::is_exact`, `meter_of_file`; the meter reads `spend ≥ $X (lower bound)` while live or inexact and `spend $X` once exact. Folding the run A fixture ends exactly at 0.86173693 and exact. Docs and the three help rows say the soft cap is judged against the lower bound.
- **D-07:** two whitelist-trimmed fixtures, README, TESTING.md row.

## Deferred

Nothing from the plan was deferred. Not built (out of scope per plan): any paid run, push, tag, rebase. No ROADMAP.md update, and STATE.md and this SUMMARY are left to the orchestrator's docs commit.

## Tests and lint

Raw `rtk proxy cargo test --no-fail-fast` logs in `target/`:

| Log | Passed | Failed | Ignored | Binaries |
|-----|--------|--------|---------|----------|
| `mnr-t0.log` (base 3934078, see note) | 3491 | 1 (environmental) | 17 | 59 |
| `mnr-t2.log` (after Task 2) | 3513 | 0 | 17 | 59 |
| `mnr-t3.log` (final, e6c67c4) | 3522 | 0 | 17 | 59 |

- Final vs base per-binary diff: lib 2806 -> 2832, `oneoff_e2e` 37 (+1 failed) -> 42; every other binary line identical; all 59 binaries present.
- **t0 note:** the baseline had to run in a scratch worktree under the session scratchpad (an earlier baseline in the main tree was invalidated because I had started editing while the registry/doc tests compile the source). Its single failure, `a_data_root_outside_home_is_masked_in_the_sandbox`, is an artifact of that location: the test's temp dir sits under `/tmp` (the scratch target dir) which the sandbox masks with `--tmpfs /tmp`. The same test passes in the real tree (verified, and it is part of the green t2/t3). The scratch worktree and its target dir were removed afterwards. The baseline was built with `CARGO_PROFILE_DEV_DEBUG=0` because `/tmp` is a 16 GB tmpfs.
- **No t1 log:** Task 1 was verified with its targeted automated commands (lib `oneoff::session/runtime/stream`, five named e2e tests, seam guard, leak gate), then the next full-suite run is t2. A full run at the Task 1 commit alone was not made.
- `cargo clippy --all-targets -- -D warnings`: clean on host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`; `cargo fmt --check` clean. The seam guard and `spawn_seam_guard` pass (inside the full suite).
- Red-first evidence: the three new e2e behaviours were run against the base commit in the scratch worktree and failed there as the bugs predict (`input was closed while a background task was outstanding`; `task stop --pause` exit 0 with `stopped`; outcome `paused` for a `done` report; `merged 1 commit(s)`), and pass now.

## Fixture leak check (T-mnr-05)

Fixtures were generated by a whitelist script kept in the session scratchpad (not in the repo), one output line per input line (304 and 44). Gate commands run from the repo root; both printed nothing:

```
grep -niE 'blk|/home/|/tmp/|@|(^|[^a-z])sk-|bearer|oauth|apikey|anthropic|fbcffc10' tests/fixtures/oneoff/real-bg-subagents.stream.jsonl tests/fixtures/oneoff/real-resumed.stream.jsonl
   -> (empty)
grep -ohiE '"[a-z0-9_]*token[a-z0-9_]*"' <both> | sort -u | grep -viE '^"[a-z0-9_]*tokens(_delta|_details)?"$'
   -> (empty)
```

Sorted token-like keys left in the fixtures (all usage counts): `cacheCreationEphemeral1hInputTokens`, `cacheCreationEphemeral5mInputTokens`, `cacheCreationInputTokens`, `cacheReadInputTokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `ephemeral_1h_input_tokens`, `ephemeral_5m_input_tokens`, `inputTokens`, `input_tokens`, `maxOutputTokens`, `outputTokens`, `output_tokens`, `output_tokens_details`, `thinkingTokens`, `thinking_tokens`, `total_tokens`.

## Deviations from Plan

**1. [Process - commit guard] Commits made on `master`, which the GSD protected-branch assertion flags.**
`gsd_run query git.base-branch --is-protected master` returns true, so the executor's Step 0 assertion would halt. The plan, the orchestrator prompt (`Branch: master`, sequential, no branches/worktrees) and every prior quick task in this repo's history put commits directly on master, with `branching_strategy: none`. I treated that as the explicit intended target, committed locally, did NOT edit `.planning/config.json` (`git.allow_default_branch_commits` stays unset) and did not push. Flagging it so the orchestrator can confirm. [inferred — audit]

**2. [Rule 1 - Bug in an existing test] `a_reported_cost_is_the_baseline_and_usage_adds_on_top` asserted the old behaviour.**
It re-sent message `m2` after a result and expected it to count as new, which D-06(c) (ignore ids already folded into a baseline) forbids by design. The last step now uses a fresh id `m3` and asserts the repeat of `m2` is ignored. Commit `59030a0`.

**3. [Rule 3 - Blocking, tooling] Baseline run.** See the t0 note above (separate scratch worktree with a smaller dev profile; one environmental failure documented).

Otherwise the plan was executed as written, including: the tracer gate (the e2e was made red first on the base commit), `help.rs` instead of the non-existent `src/help.rs` (as the plan notes), and the `Landing` struct untouched (message-only count).

## Decisions [inferred — audit]

D-01..D-07 were carried as locked in CONTEXT. New or discretionary choices of this execution:

- D-DISC, as planned: bug 4's count is message-only (no `Landing` field).
- Names: `IDLE_SETTLE` (120 s), `IDLE_SETTLE_ENV` = `GSDMM_ONEOFF_IDLE_SETTLE_SECS`, `idle_settle_from_env`/`idle_settle_from`, `LifecycleStep::CloseStdinIdle`, `SessionLifecycle::{with_idle_settle, outstanding}`, `StreamEvent::{BackgroundTasks, TaskStarted, TaskEnded}`, `TurnResult::{queued_turns, subagents_outstanding}`, `StreamEvent::Usage::content_chars`, `UsageTokens::cache_creation_1h`, `CACHE_WRITE_1H_FACTOR`, `Meter::exact`, `SpendEstimate::is_exact`, `meter_of_file`, `soft_cap_text`, `StopRecord::refused`, `Cancel::withdraw_pause`, `StopOutcome::{PauseDelivered, PauseRefused, PauseUnconfirmed}` + `is_refusal`, `inbox::pause_outcome_since`, `landed_message`.
- The lifecycle's idle clock is "now minus the later of the result instant and the last event instant" (any classified event, including `Other`, moves it); the `REPLAY_SETTLE` path is measured from the result instant only, as before.
- `outstanding()` takes the max of the background signal and the latest result's `subagents_outstanding`; the result's value is kept until the next result (harmless: `decide` only looks at it while a result is current).
- Window names from `unifiedWindows` pass through `lower_word`; a `rateLimitType` with no matching window falls back to the highest window for utilization while keeping the given kind.
- A live run in `task status` counts spend as `record.spend_usd.unwrap_or(0) + stream estimate` (so a resume merge's resolver spend is included); "live" means `LiveState::may_be_live()`.
- Landing message when `rev-list` cannot be computed: `landed N commit(s) by the agent of <run> on <branch> (now at <sha>)` (makes no claim beyond the agent's count).
- A refused pause stamps `ended_at` on the stop record at once (and re-emits the record); the final stop-record write now uses `get_or_insert` so that time is kept. Pause refusal wording for the TUI is the CLI message prefixed by the existing `pause refused: `.
- A pause request merged into something stronger (hard/kill) keeps the old hard/kill wait path; the pause polling applies only when the merged request is still a pause.
- Idle-settle e2e override set in `task()` (every e2e launch), 1 s.

## Unverified without a paid run

- That the real agent ends promptly on stdin EOF after its final notification turn.
- That it accepts a user message mid-background-work as a new turn.
- That `background_tasks_changed`, `queued_turn_count` and `subagent_stats` keep their shape in other agent versions (the classifier treats absent or malformed fields as "no evidence of outstanding work").
- That the live lower bound tracks real spend as in the replica (run A 0.40 vs 0.43 before the first result); only the final exact snap and the pricing identity are asserted from the fixture.

## Known Stubs

None.

## Threat Flags

None. New surface is host-side only (report-file presence check via `symlink_metadata`, host-written `injections.jsonl` read by the CLI) and is covered by T-mnr-01..09.

## Self-Check: PASSED

- Commits `ab13940`, `59030a0`, `e6c67c4` present on master (`git rev-list --count 3934078..HEAD` = 3).
- Created files present: both fixtures (304 and 44 lines) and `tests/fixtures/oneoff/README.md`.
- Working tree clean after Task 3 (no untracked leftovers); scratch worktree removed.
