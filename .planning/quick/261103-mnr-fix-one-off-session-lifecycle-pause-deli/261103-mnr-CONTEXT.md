# Quick Task 261103-mnr: Fix one-off session lifecycle, pause delivery, rate-limit and meter bugs from first real paid run - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning
**Mode:** unattended (human unavailable). Every decision below was taken by the orchestrator from the recorded evidence and is marked `[inferred — audit]`.

<domain>
## Task Boundary

Fix six defects found in the first real paid one-off test (Opus, real bwrap/systemd, claude 2.1.317, `--mode fast`, runs `/gsd-map-codebase` and `/gsd-docs-update`):

1. (High) stdin is closed at the first `result` while GSD background subagents still work, so later injections / pause / soft-cap wrap-up are refused ("the session is ending").
2. (Medium) An undelivered pause is reported as success by `task stop --pause` (exit 0), and the run is then recorded `outcome: paused` although it finished with a `done` report.
3. (Low) Rate-limit utilization is never read (real shape nests it under `unifiedWindows.<window>.utilization`).
4. (Low) Landing message miscounts commits ("merged 1 commit(s)" when master advanced by 3).
5. (Gap) `task status` shows no live spend until the run ends.
6. Live meter undercounts (A: $0.65 live vs $0.86 final; B: $2.04 vs $3.95).

Update docs/CONFIGURATION.md and src/help.rs where behaviour/wording changes. Real run dirs under `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/` stay on disk (read-only evidence).

Out of scope: any paid `claude` run; push; tags; rebase.
</domain>

<evidence>
## Evidence from the recorded streams (orchestrator analysis)

Run A = `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-220759/stream.jsonl` (304 lines, `/gsd-map-codebase`, 4 background `gsd-codebase-mapper` agents on haiku).
Run C = `.../oneoff-20261103-221445/stream.jsonl` (44 lines, resumed docs run, no background agents).
Run B (`oneoff-20261103-220759-2`, the one with the refused pause) no longer exists on disk; its symptoms are as reported in the task brief. Run A's `injections.jsonl` shows the same refusal shape for a message: `{"kind":"message",...,"outcome":"refused","reason":"the session is ending"}` at 22:09:47, after the first `result`.

Line numbers below are 0-based line indices in run A.

- Background work signals present in the stream:
  - `{"type":"system","subtype":"background_tasks_changed","tasks":[{task_id, run_id, description, subagent_type, task_type}...]}` — the full outstanding set each time it changes. Counts in run A: 1 (L30), 2 (L37), 3 (L46), 4 (L57), then 3 (L181), 2 (L204), 1 (L214), 0 (L235).
  - `system/task_started` (L31,38,47,58: `task_id`, `tool_use_id`, `is_backgrounded:true`), `system/task_progress` (with `usage:{duration_ms,tool_uses,total_tokens}`), `system/task_updated` (`patch.status:"completed"`), `system/task_notification` (L180,203,213,234: `status:"completed"`, `usage`).
  - Each notification is followed by a `system/init` (L182,205,215,236): the agent starts a NEW turn on the notification.
  - `result` carries `subagent_stats` (`spawned`, `completed`, `failed`, `killed:{parent,system,user}`, `started_in_background`), `queued_turn_count`, `result_index`, and for notification turns `origin:{kind:"task-notification",...}`.
  - First `result` is at L91 (~38 s in): `result_index 0`, `subagent_stats.spawned 4, completed 0`, text "All 4 mappers are running ... Once they finish I'll check the output, stamp it, scan for secrets and commit." — the work is clearly NOT done.
  - The final turn (L236–299) runs after the outstanding set reached 0 (L235); the agent's `taskctl report` was written in it (report `at` 22:11:19). Results L300–303 (`result_index` 1–4, `queued_turn_count 0`, `completed 4`) are flushed at the very end.
- Spend signals:
  - `result.total_cost_usd` is cumulative for the whole session and INCLUDES subagent usage (`modelUsage` lists both `claude-opus-5-5` and `claude-haiku-5-5`). L91: $0.4319; L300–303: $0.8617 (= run.json spend).
  - Subagent assistant messages ARE forwarded in the stream with `parent_tool_use_id` set (42 unique haiku message ids in run A). Their `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens` sum EXACTLY to the final `modelUsage` figures (haiku: 84 / 203 769 / 1 700 463; opus parent: 44 / 54 218 / 885 513).
  - Streamed `output_tokens` are only the message-start snapshot: opus 896 streamed vs 8 140 final; haiku 328 vs 60 295. Run C: 548 vs 2 243.
  - Cache writes are 1-hour ephemeral (`usage.cache_creation.ephemeral_1h_input_tokens`; 5m = 0). With 1h writes priced at 2.0 x input (not the table's 1.25 x), the existing price table reproduces the reported `costUSD` exactly: opus 44x4 + 8140x20 + 54218x8 + 885513x0.2 = $0.77382 (reported 0.7738226); haiku 84x0.1 + 60295x0.5 + 203769x0.2 + 1700463x0.01 = $0.08791 (reported 0.0879143).
  - Content size of streamed blocks (text + tool_use input JSON) / ~3.5-4 recovers roughly half of the real output tokens (opus ~4 055 of 8 140; haiku ~35 105 of 60 295); thinking text is not visible (thinking blocks are empty/signature only).
  - A subagent message seen before a `result` can be repeated after it (L86 and L92 are the same message, before/after the L91 result) — after the baseline snap it must not be counted again.
- Rate limit: L12 `{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"seven_day","resetsAt":1794326400,"isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"out_of_credits","unifiedWindows":{"seven_day":{"status":"allowed","resetsAt":1794326400,"utilization":0.21}}}}`. No top-level `utilization`.
- Landing: run C `landed[0].range` `21090c1..9a93bdc` spans 3 commits on master (9b8047d, ca323ad host merge, 9a93bdc) although `commits: 1`.
- The streams on disk are already partially redacted (`/home/[REDACTED:user]/...`, `/tmp/claude-[REDACTED:uid]/...`).
</evidence>

<decisions>
## Implementation Decisions

### 1. Session-end rule (stdin close) — `src/oneoff/session.rs` `SessionLifecycle`
- `[inferred — audit]` The lifecycle tracks **outstanding background tasks**. Authoritative source: the latest `system/background_tasks_changed` `tasks` array length. Fallbacks when that event is absent: `task_started` (+1 per new `task_id`) and `task_notification` / `task_updated` with a terminal status (completed / failed / killed / stopped / error) (−1 for that `task_id`), tracked as a set of task ids. A `result` whose `subagent_stats` shows `spawned − completed − failed − (killed.parent+killed.system+killed.user) > 0` also counts as outstanding work. Outstanding = the max of the known signals. Unknown/absent fields mean "no evidence of outstanding work" (tolerant of older claude versions and the fake tooling).
- `[inferred — audit]` A `system/init` event seen after the first one is a new turn start (claude starts a turn on each task notification, L182/205/215/236): it clears `result_since` exactly like a replay does.
- `[inferred — audit]` A `result` with `queued_turn_count > 0` is not an end.
- `[inferred — audit]` **Close stdin when ALL hold:** a `result` has been seen since the last turn start; replays ≥ writes (existing rule); outstanding background tasks == 0; `queued_turn_count` == 0; AND EITHER the agent's `taskctl report` exists (the run dir's report file) → close immediately, OR (no report) the stream has been idle (no new classified event) for **IDLE_SETTLE = 120 s** since that result → close, with a warning recorded ("closed the agent's input after 120s idle with no report").
  - Justification: run A's first result (L91) had 4 outstanding tasks and an explicit "once they finish I'll ... commit"; the real end was the result after the outstanding set hit 0 and the report was written. The report is the agent's own "I'm done" signal in this design (the prompt tells it to call `taskctl report` and stop); the idle fallback covers an agent that forgets to report without waiting for the wall clock.
  - `IDLE_SETTLE` is overridable on the host via an env var in the same style as `GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS` (e.g. `GSDMM_ONEOFF_IDLE_SETTLE_SECS`, 1..=600, read by the daemon at session start, not settable by the sandboxed agent) so fake-tooling tests that never write a report do not wait 120 s. `[inferred — audit]`
- The existing REPLAY_SETTLE (30 s) "written but never replayed" warning path stays, but only applies once outstanding == 0.
- Backstops unchanged: wall-clock cap, `--max-budget-usd` hard ceiling, soft-cap grace stop, user stop/kill.
- While stdin stays open, injections / pause / soft-cap wrap-up are delivered (no more "the session is ending" refusals during background work). Claude delivers them as a new user turn; that is acceptable.
- The decision stays a pure function of events + `now` + a `report_present: bool` fed by the supervisor (keep `SessionLifecycle` unit-testable); the supervisor checks the report file's existence each tick.
- Tests: unit tests for each rule, plus a replay test that feeds a trimmed fixture derived from run A through `SessionLifecycle` and asserts stdin stays open at the L91-equivalent result and closes only at the final result (with report present) — and an idle-fallback test with no report.

### 2. Pause delivery and outcome precedence — `src/oneoff/launch.rs`, `src/task_cli.rs`, TUI
- `[inferred — audit]` `stop()` with `StopKind::Pause` waits (within the existing `wait`) for the daemon's `injections.jsonl` record of kind `pause` written at/after the request, and returns distinct outcomes: delivered-and-winding-down, delivered-and-ended (lock freed), or **refused { reason }**. If neither a record appears nor the lock frees within the wait, report "pause requested; delivery not yet confirmed" (exit non-zero? → see next bullet).
- `[inferred — audit]` CLI exit codes: delivered → 0; refused → non-zero (1) with a clear message, e.g. `task stop: the pause could not be delivered to <run> (the session is ending); it keeps running — use 'task stop' (without --pause) to stop it`; unconfirmed within the wait → non-zero (1) with "not confirmed" wording. The TUI's pause action surfaces the same refused/unconfirmed text as an error toast/status line instead of success.
- `[inferred — audit]` When the daemon refuses a pause, it **withdraws the pause**: no pause-grace cut is scheduled for it and the stale request must not later turn a natural end into `Paused`. The refusal stays recorded in `injections.jsonl` and in `run.json.stop` (so `task status` shows the pause was refused, not "pausing").
- `[inferred — audit]` **Outcome precedence on a clean natural exit** (`launch.rs` ~L947-956): the agent's report wins. Report `status: done` → normal finished outcome, never `Paused`, even if a pause request exists. `Paused` only when a pause was **delivered** AND the report is `partial` (the pause message asks for `--status partial`). A delivered pause with no report or a `blocked` report falls back to the ordinary non-pause classification (Exited / finished-with-report). Hard/Kill requests keep current precedence.
- Tests: launch-level unit/integration tests with the fake agent covering: refused pause → CLI non-zero + message; done report after a pause request → not paused; delivered pause + partial report → paused.

### 3. Rate-limit utilization — `src/oneoff/runtime.rs` `classify_claude`
- `[inferred — audit]` Utilization source order: `rate_limit_info.unifiedWindows[rateLimitType].utilization`; else the highest `utilization` across all `unifiedWindows` entries (with that window's name as `kind` if `rateLimitType` is missing); else the legacy top-level `rate_limit_info.utilization`. `resets_at` likewise prefers the chosen window's `resetsAt`, then the top-level one. Keep the existing finite/≥0 filter and the old-shape test; add a real-shape test from the L12 line.

### 4. Landing count — `src/oneoff/land.rs` (and any non-ff merge path in `merge.rs` / resume merge that prints a count)
- `[inferred — audit]` Count what actually landed: `git rev-list --count <old_target_head>..<new_target_head>` after the landing (includes host merge commits and commits brought in by a resumed run). Message names both when they differ, e.g. `landed 3 commit(s) of <run> on master (1 by the agent; now at <sha>)`; when equal, just `landed N commit(s)`. If the count cannot be computed, fall back to the agent's commit count with no claim beyond it. Record the count in the landing entry if the schema allows an optional field (additive, `#[serde(default)]`); otherwise message-only.

### 5. Live spend in `task status` — `src/task_cli.rs`
- `[inferred — audit]` For a run that may be live (lock held / not finished), `task status` folds the run's `stream.jsonl` through the same `SpendEstimate` + rate-limit reading the viewer uses and prints e.g. `spend:     ≥ $0.78 so far of $10.00 (live estimate, lower bound; exact at the end)` plus `soft cap:` (if set) and `rate:      allowed seven_day 21% resets HH:MM`. `--json` gains additive fields (`live_spend_usd`, `live_spend_is_lower_bound: true`, `rate_limit {status, kind, utilization, resets_at}`). Finished runs keep the recorded final `spend_usd` (exact, no "≥"). Reading is bounded (stream files are < 1 MB in practice; reuse `StreamTail`/existing bounded readers).

### 6. Meter accuracy — `src/oneoff/runtime.rs` pricing + `src/oneoff/stream.rs` `SpendEstimate`
- Finding: subagent usage IS present (forwarded messages with `parent_tool_use_id`), already folded; it is not the cause. The undercount comes from (a) 1-hour cache writes priced at 1.25 x instead of 2 x input and (b) streamed `output_tokens` being a message-start snapshot. (A third, smaller issue: re-counting a message repeated after a `result`.)
- `[inferred — audit]` (a) Price cache writes from the split when present: `usage.cache_creation.ephemeral_1h_input_tokens` at 2.0 x input, `ephemeral_5m_input_tokens` at 1.25 x input; when the split is absent, keep 1.25 x for all of `cache_creation_input_tokens`. Extend `UsageTokens` additively. Update the price-table doc comment. Unit test: run A's final-turn token sums reproduce `modelUsage.costUSD` to within $0.001 for opus and haiku.
- `[inferred — audit]` (b) Live output estimate per message = max(reported `output_tokens`, ceil(chars of visible text + tool_use input JSON / 4)). Still misses hidden thinking, so the live figure remains a **lower bound** until a `result` snaps the baseline to `total_cost_usd` (cumulative, exact, includes subagents).
- `[inferred — audit]` (c) `SpendEstimate` remembers message ids already folded into a `result` baseline (bounded set, cleared with the existing MAX_TRACKED_MESSAGES rollover) and ignores later copies of them.
- `[inferred — audit]` Labelling: the live figure is shown as `≥ $X` / "lower bound" in the TUI meter (`meter_text`) and the CLI; the final recorded spend (from the last `result`) is shown without "≥". The soft cap documentation states plainly that it is judged against this lower-bound estimate, so it can fire late (by roughly the unseen output/thinking cost of the current turn), and that each `result` snaps it to the exact cumulative figure; the hard ceiling (`--max-budget-usd`, enforced by claude itself) is exact. Document in docs/CONFIGURATION.md and the help text in src/help.rs.

### Fixtures
- `[inferred — audit]` Add trimmed, credential-free excerpts of runs A and C under `tests/fixtures/oneoff/` (e.g. `real-bg-subagents.stream.jsonl`, `real-resumed.stream.jsonl`): keep event structure, ids, `type`/`subtype`, usage, `result` fields, `rate_limit_event`; replace/strip prompt bodies, tool inputs/results and assistant text with short placeholders (keep their character counts only where a test needs them, or adjust the test expectation); replace any home/tmp path with `/home/redacted/...`; drop `session_id`/`uuid` or replace with fixed fakes. Before committing, grep the fixtures for `blk`, `/home/`, `@`, `sk-`, `token`, `Bearer`, `oauth`, `apiKey`, `ANTHROPIC` and the real session id `fbcffc10` and confirm none remain (record the check in SUMMARY).

### Claude's Discretion
- Exact names of new types/fields/env vars, message wording (keeping the meaning above), test file placement, how the supervisor locates the report file (use the existing report path constant/helper).
- Whether bug 4's count is persisted in run.json or message-only.
</decisions>

<specifics>
## Specific Ideas

- Pricing check that must pass (run A final totals): opus-5-5 {in 44, out 8140, cw1h 54218, cr 885513} → $0.77382; haiku-5-5 {in 84, out 60295, cw1h 203769, cr 1700463} → $0.08791.
- The session-end replay test should show: at the first `result` (outstanding 4, report absent) → Keep; after L235 (outstanding 0) and the final `result` with report present → CloseStdin.
- Tests: `cargo test --no-fail-fast`; use `rtk proxy cargo test --no-fail-fast` for raw counts and do not pipe through grep. `cargo clippy --all-targets -- -D warnings` must be clean.
- No paid `claude` runs; use the existing fake-agent tooling for integration coverage.
</specifics>

<canonical_refs>
## Canonical References

- Prior one-off quick tasks (read their SUMMARY.md for the design vocabulary C-xx / I-xx / S-xx): `.planning/quick/261103-f5y-*`, `261103-gt1-*` (session lifecycle, meter, soft cap), `261103-hx7-*` (parallel runs, spend_usd), `261103-klf-*` (pause/stop/cancel, outcome precedence).
- Real evidence (read-only, leave on disk): `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-220759/{stream.jsonl,injections.jsonl,run.json}`, `.../oneoff-20261103-221445/{stream.jsonl,run.json}`.
- Meter replica used during the test: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/26d084ca-c51a-4925-971e-e01ece9f94ce/scratchpad/meter.py`.
- docs/CONFIGURATION.md, src/help.rs.
</canonical_refs>

## Open questions for the human (non-blocking; defaults chosen above)
- IDLE_SETTLE = 120 s for a no-report session end — longer keeps the session open for operator injections, shorter ends forgotten-report runs sooner.
- Whether an unconfirmed (not refused, just not yet acknowledged) pause should exit non-zero (chosen: yes, 1).
