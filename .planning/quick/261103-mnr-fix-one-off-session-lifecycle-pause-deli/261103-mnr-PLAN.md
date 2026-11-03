---
quick_id: 261103-mnr
mode: quick
phase: quick-261103-mnr
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
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
  - tests/fixtures/oneoff/real-bg-subagents.stream.jsonl
  - tests/fixtures/oneoff/real-resumed.stream.jsonl
  - tests/fixtures/oneoff/README.md
  - docs/CONFIGURATION.md
  - docs/TESTING.md
autonomous: true
requirements: [QUICK-261103-mnr]

estimate:
  tokens: 420000
  raw_tokens: 420000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "A one-off whose agent still has background tasks outstanding at a `result` keeps its stdin open: `task send`, a pause and the soft-cap wrap-up are DELIVERED during that background work (never refused with `the session is ending`). Stdin closes only when a result was seen since the last turn start, replays >= writes, outstanding background tasks == 0, queued turns == 0, and either the agent's `report/report.json` exists (close at once) or the stream has been idle for the idle settle (120 s; `GSDMM_ONEOFF_IDLE_SETTLE_SECS` 1..=600 for tests), the latter recorded as a run.json warning `closed the agent's input after <N>s idle with no report` (D-01)."
    - "Fed the trimmed recording of run A, `SessionLifecycle` answers Keep at the first result (4 background tasks outstanding, even with a report present) and its first close is at the first result after the background set reached 0; fed run C (no background agents), it closes at the result when the report is present and only after the idle settle when it is not (D-01, D-07)."
    - "`task stop --pause` exits 0 only when the launcher's `injections.jsonl` holds a `pause` record with outcome `delivered` written at/after the request; a refused pause exits 1 with `the pause could not be delivered to <run> (<reason>)` and the hint to use `task stop` without `--pause`; a pause not confirmed within `--wait-secs` exits 1 with `not confirmed` wording. The TUI `z` on a run waits up to 10 s and shows a refusal or non-confirmation as `pause refused: ...`, not as success (D-02)."
    - "A refused pause is withdrawn: no pause-grace cut follows it (with a 1 s pause grace the run still ends `finished`, not `killed`), run.json `stop.refused` holds the reason, and `task status` / the TUI summary say the pause was refused instead of `pausing` (D-02)."
    - "On a clean natural exit the agent's report decides: a `done` report ends `finished` even after a delivered pause; `paused` only for a DELIVERED pause plus a `partial` report; a delivered pause with no report or a `blocked`/`failed` report falls back to the ordinary classification. Hard/kill precedence is unchanged (D-02)."
    - "The real rate-limit shape (`rate_limit_info.unifiedWindows.<window>.utilization`) is read: the L12 line of run A classifies as status `allowed`, kind `seven_day`, utilization 0.21, resets 1794326400, and the meter shows `seven_day 21%`; the old top-level shape still works (D-03)."
    - "A fast-forward landing names what actually landed: `landed <git rev-list --count old..new> commit(s) of <run> on <branch> (<agent count> by the agent; now at <sha12>)` when the counts differ, `landed N commit(s) of <run> on <branch> (now at <sha12>)` when equal; the resume-from-master e2e lands 3 commits with 1 by the agent (D-04)."
    - "`task status` on a live run prints `spend:     ≥ $X so far of $Y (live estimate, lower bound; exact at the end)`, a `soft cap:` line when a soft cap is set and a `rate:` line when a rate-limit event was seen; `task status --json` adds `live_spend_usd`, `live_spend_is_lower_bound: true` and `rate_limit {status, kind, utilization, resets_at}`; a finished run keeps its recorded `spend_usd` line with no `≥` (D-05)."
    - "The spend estimate prices `ephemeral_1h_input_tokens` cache writes at 2.0 x input (5-minute and unsplit writes stay 1.25 x), estimates each message's output as max(reported output_tokens, ceil(visible characters / 4)) accumulated over the message's lines, and ignores a message id already folded into a result baseline; run A's final token sums reproduce the reported costUSD within $0.001 for opus ($0.77382) and haiku ($0.08791), and folding the run A fixture ends exactly at the reported $0.86174 (D-06)."
    - "The meter line reads `spend ≥ $X (lower bound) / ...` while the run may be live or the figure is not a just-reported total, and `spend $X / ...` once it is exact; docs/CONFIGURATION.md states that the soft cap is judged against this lower bound, can fire late by roughly the unseen output/thinking of the current turn, snaps to the exact cumulative figure at each result, and that `--max-budget-usd` is exact (D-06)."
    - "The committed fixtures under `tests/fixtures/oneoff/` contain no user name, home or tmp path, `@`, key prefix, bearer/oauth/apiKey/ANTHROPIC string or the real session id, and every token-like string in them is a usage-count key (D-07, T-mnr-05)."
  artifacts:
    - path: tests/fixtures/oneoff/real-bg-subagents.stream.jsonl
      provides: "Trimmed, credential-free structure of run A (304 lines): background tasks, task events, five results, usage, the real rate-limit event"
    - path: tests/fixtures/oneoff/real-resumed.stream.jsonl
      provides: "Trimmed structure of run C (44 lines): a resumed docs run with no background agents"
    - path: tests/fixtures/oneoff/README.md
      provides: "Source, trimming rules and the leak check of the two fixtures"
    - path: src/oneoff/session.rs
      provides: "SessionLifecycle with outstanding-work tracking, turn starts, queued turns, report-present close and the idle settle; Supervisor fed the report path; inject_host returning the refusal"
    - path: src/oneoff/stream.rs
      provides: "BackgroundTasks/TaskStarted/TaskEnded events, TurnResult counters, SpendEstimate with char-based output, folded ids and exactness, meter_of_file, lower-bound meter_text"
    - path: src/oneoff/runtime.rs
      provides: "classify_claude for system task events, result counters, unifiedWindows utilization, 1-hour cache split and visible characters; 1h cache pricing"
    - path: src/oneoff/cancel.rs
      provides: "StopRecord.refused and Cancel::withdraw_pause"
    - path: src/oneoff/launch.rs
      provides: "pause delivery tracking, pause_end_with_report precedence, stop() pause outcomes"
  key_links:
    - from: src/oneoff/session.rs
      to: src/oneoff/launch.rs
      via: "Supervisor::new(..., report_file, idle_settle) called from run_sandboxed_session with spec.report_dir.join(REPORT_FILE) and idle_settle_from_env(); Supervisor::tick calls lifecycle.decide(now, report_present)"
      pattern: "idle_settle_from_env"
    - from: src/oneoff/runtime.rs
      to: src/oneoff/session.rs
      via: "classify_claude emits StreamEvent::BackgroundTasks / TaskStarted / TaskEnded and TurnResult { queued_turns, subagents_outstanding } that SessionLifecycle::saw consumes"
      pattern: "BackgroundTasks"
    - from: src/oneoff/launch.rs
      to: src/oneoff/cancel.rs
      via: "the wait loop's InjectPause arm calls cancel.withdraw_pause() when inject_host returns Err(reason)"
      pattern: "withdraw_pause"
    - from: src/oneoff/launch.rs
      to: src/oneoff/inbox.rs
      via: "stop() polls inbox::pause_outcome_since(run_dir, requested_at) to tell delivered, refused and unconfirmed apart"
      pattern: "pause_outcome_since"
    - from: src/task_cli.rs
      to: src/oneoff/stream.rs
      via: "status folds the live run's stream.jsonl through stream::meter_of_file"
      pattern: "meter_of_file"
---

<objective>
Fix the six defects the first real paid one-off run exposed (CONTEXT §domain), exactly as the locked decisions in 261103-mnr-CONTEXT.md say. Decision ids used in this plan (CONTEXT numbers its sections; the plan names them):

- D-01 = CONTEXT §1 Session-end rule (stdin close), bug 1.
- D-02 = CONTEXT §2 Pause delivery and outcome precedence, bug 2.
- D-03 = CONTEXT §3 Rate-limit utilization, bug 3.
- D-04 = CONTEXT §4 Landing count, bug 4.
- D-05 = CONTEXT §5 Live spend in `task status`, bug 5.
- D-06 = CONTEXT §6 Meter accuracy (a) 1-hour cache price, (b) char-based output, (c) repeated ids, labelling; bug 6.
- D-07 = CONTEXT §Fixtures.
- D-DISC = CONTEXT §Claude's Discretion (names, wording, test placement, report-path lookup, whether bug 4's count is persisted).

Purpose: a one-off must not end its own input while GSD background subagents still work, must never report an undelivered pause as success, and its spend/rate figures must be honest (labelled lower bound while live, exact at the end).

Output: three commits on master, one per task, each `fix(quick-261103-mnr): ...`:
- Task 1 (tracer): session lifecycle + fixtures (D-01, D-07).
- Task 2: meter, rate limit, live `task status` spend, labels (D-03, D-05, D-06).
- Task 3: pause delivery, outcome precedence, landing count (D-02, D-04).
Docs (docs/CONFIGURATION.md, the TUI help in `src/ui/screens/help.rs`, docs/TESTING.md) are updated in the task whose behaviour they describe. Note: CONTEXT says `src/help.rs`; the live tree has no such file, the help text lives in `src/ui/screens/help.rs` (three identical copies of each row, as klf found).

Execution context (human unavailable): run SEQUENTIALLY on master in the main tree, no worktree. No push, no tag, no rebase. No paid agent run: all integration coverage uses the fake-agent harness in `tests/launcher_support/mod.rs`. Mark every new judgement call `[inferred — audit]` in the SUMMARY.
</objective>

<gates>
- musk: `gsd-tools musk phase-check --phase quick` printed "no phase check for quick". Every task traces to D-01..D-07; nothing was omitted, so nothing is logged.
- api-coverage: does not fire. No external API or SDK is integrated; the work reads the existing agent CLI's stream-json through the existing runtime seam.
- assumption-delta: skipped (no ROADMAP phase). One real transition: "one result ends the session" becomes "a session ends when its outstanding work is done". Primary noun stays the session; the lifecycle gains facts, no identity changes. The invariant test is the run-A fixture replay in Task 1.
- schema gate: does not fire (no ORM). run.json gains only `stop.refused` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), so older files read unchanged.
- security: `<threat_model>` below at ASVS L1, blocking on high. The one high threat (fixture leakage, T-mnr-05) is mitigated by Task 1's leak gate.
- Discretion (D-DISC): bug 4's count is message-only (no `Landing` field), because `Landing` has six literal construction sites and the message is what was wrong. `[inferred — audit]`
</gates>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/261103-mnr-fix-one-off-session-lifecycle-pause-deli/261103-mnr-CONTEXT.md
@.planning/quick/261103-klf-cancel-pause-one-off-runs-and-resolvers-/261103-klf-SUMMARY.md
@.planning/quick/261103-gt1-live-viewer-with-message-injection-and-s/261103-gt1-SUMMARY.md

Large files: NEVER read `src/app.rs` or `src/ui/screens/render_escape_guard.rs` whole. Grep for the symbol, then Read with offset/limit. `src/oneoff/launch.rs` (1655 lines), `src/ui/screens/oneoff.rs` (2254) and `tests/oneoff_e2e.rs` (2173) are read by range. If an anchor below has moved, find it by its symbol name.

Measured at HEAD 9a93bdc (live observation at planning time):

`src/oneoff/session.rs` (644 lines, runtime-neutral: must never contain the agent's name; the seam guard scans non-comment lines):
- `REPLAY_SETTLE` :16, `LifecycleStep` :20-25, `SessionLifecycle` :31-106 (`wrote` :58, `saw` :66, `decide` :79, `closed` :98, `can_write` :103).
- `SOFT_CAP_GRACE_ENV` :113, `grace_from_env`/`grace_from` :117-124 (pattern for the new idle-settle env).
- `Supervisor` :259-499: `new` :274, `send_prompt` :301, `record_of`/`refused` :314-342, `deliver` :346, `inject_host` :370-378, `handle` :380, `tick` :422-473 (`self.lifecycle.decide(now)` :461), `finish` :483.
- Tests :502-644 (lifecycle tests :519-588).

`src/oneoff/stream.rs` (855 lines, runtime-neutral):
- `UsageTokens` :18-24, `StreamEvent` :28-72 (`TurnResult` :50-55, `RateLimit` :56-61, `Usage` :62-67, `System` :68-70).
- `MAX_TRACKED_MESSAGES` :180, `SpendEstimate` :190-234 (doc comment :182-189 claims subagent usage is only partly visible: wrong, fix it).
- `RateLimitSeen` :237, `Meter` :247, `rate_text` :262, `meter_text` :287-312 (`spend ≈` text :304).
- `Renderer::render` :372 (silent arm :437-440), `Follower` :475-531, `spend_of_file` :537-553.
- Tests :555-855 (`usage()` helper :610, `result()` :623, meter test :682, spend-of-file test :845).

`src/oneoff/runtime.rs` (1048 lines, the ONLY file that may name the agent):
- `HostEnd` :37-50. Price table: `Prices` :176, `PRICES` doc + rows :192-209, `notional_usd` :243-254.
- `PAUSE_MESSAGE` :261, `lower_word` :294, `one_line` :271.
- `classify_claude` :359-490: assistant usage :400-419, `result` arm :461-472, `rate_limit_event` arm :473-484, `system` arm :485-487.
- `outcome_at` :541 (HostEnd::Paused -> AgentOutcome::Paused :550).
- Tests :824-1048 (rate-limit asserts :952-973, price test :1003-1033).

`src/oneoff/launch.rs`:
- `WAIT_POLL` :83 (100 ms). `run_session` builds `SessionSpec` :457-477, `on_event` :478-499 (`SessionEvent::Stop` persists run.json :493-498), `finalize` call :511-519.
- `resume_master` :556-638 maps a pre-session pause to `HostEnd::Paused` :598 (NOT a natural exit: leave it as is).
- `run_sandboxed_session` :761: `Supervisor::new` :868, wait loop :883-972, `CancelStep::InjectPause` :909-913 (ignores `inject_host`'s result today), natural exit :939-963 (`(Some(StopKind::Pause), true) if status.success() => HostEnd::Paused` :956).
- `finalize` :1037 (reads the report :1051).
- `StopOutcome` :1282-1332, `stop()` :1366-1487 (`write_request` :1420, wait loop :1434-1456 returning `PauseRequested` :1443-1447), `stop_landing` :1490.
- Tests :1530.

`src/oneoff/cancel.rs` (525 lines): `StopRecord` :112-136 (`begun` :127), `grace_with_default` :157, `Cancel` :206-297 (`tick` :231-273), tests :404-525.
`src/oneoff/inbox.rs` (448 lines): `InjectionKind` :28, `InjectionRecord` :47-59, `wait_outcome` :305-323 (StreamTail-based pattern to copy), tests :325.
`src/oneoff/land.rs` (895 lines): `ref_commit` :238, ff path in `merge` :531-585 (`old_head` :532, message :562-567).
`src/oneoff/store.rs`: `rfc3339` :387 (seconds precision, `Z`), `LiveState::may_be_live` :515.
`src/oneoff/report.rs`: `ReportStatus` :36 (Done, Partial, Blocked, Failed), `read_validated` :211.
`src/oneoff/mod.rs`: `REPORT_FILE` :59, `STREAM_FILE` :63, `REPORT_SUBDIR` :69, `INJECTIONS_FILE` :86.
`src/task_cli.rs` (bin crate, no unit-test module; covered by e2e): `summary_lines` :312 (stop line :390-405, spend line :428-433), `status_main` :509, `view_json` :532 (shared with `list --json`), `stop_main` match :715-758, `report_request` :762, attach `meter_text` call :936-948.
`src/app.rs`: TUI `stop_with` closure :1799-1827 (pause wait `Duration::ZERO` :1811-1815), command dispatch :1845-1850.
`src/ui/screens/oneoff.rs`: summary Stop label :1123-1138, test `the_summary_names_a_pending_and_a_finished_stop` :1752 (two `StopRecord` literals :1755, :1762).
`src/ui/screens/oneoff_viewer.rs`: `RunInfo` :40 (`live: LiveState`), header `meter_text` call :471-476, test asserting `spend ≈ $1.50 / soft off / hard $10.00 (notional)` :649.
`src/ui/screens/render_escape_guard.rs`: `StopRecord` literal :2302.
`src/ui/screens/help.rs`: soft-cap row (three identical copies) :391, :669, :1043.
`tests/oneoff_e2e.rs`: `task()` builder :64, `STREAM_TURN_SCENARIO` :708, `INJECT_SCENARIO` :716, soft-cap test :881-946, klf section :1629-1801 (`start_with_graces` :1639, `PAUSE_HONOURED_SCENARIO` :1701, `paused_run` :1727), `resumed_scenario` :1901, `resume_from_master_merges_the_checkout_tip_into_the_run_first` :2023-2058.
`tests/launcher_support/mod.rs`: fake agent shim :175-205 (replays the prompt line, then `exec`s the scenario with stdin attached), `hermetic_command` :381, `release` :514, `scenario_started` :520, `wait_until` :532.
`tests/oneoff_runtime_seam_guard.rs`: the neutral-file list (session.rs, stream.rs, launch.rs, cancel.rs, inbox.rs, land.rs, ui/screens/oneoff*.rs are neutral). A fixture FILE NAME used from a neutral file must not contain the agent's name.
`Cargo.toml` :32 excludes only `/.planning/`, `/CLAUDE.md`, `/assets/`, so `include_str!("../../tests/fixtures/...")` survives `cargo package` (existing pattern in `src/state_reader/mod.rs:1100`).
docs/CONFIGURATION.md: env table :122-125, stop/pause :225-245, session end :303-312, spend meter :314-320, soft cap :322-332, ff landing bullet :374-375. docs/TESTING.md: fixtures table :252-262.

Evidence (read-only, leave on disk): `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-220759/stream.jsonl` (run A, 304 lines) and `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-221445/stream.jsonl` (run C, 44 lines). Measured on them at planning time: in run A no line carries more than one content block and no content block repeats under one message id; the L86/L92 pair shares a message id with DIFFERENT blocks around the L91 result. A replica of the D-06 algorithm gave run A 0.401 (was 0.255) just before the first result (reported 0.432) and 0.789 (was 0.687) before the final one (reported 0.862); run C 0.229 (was 0.150) vs 0.250. Run A's results carry `modelUsage` with `cacheCreationEphemeral1hInputTokens`.
</context>

<tasks>

<task type="tracer">
  <name>Task 1 (tracer): the session stays open while background work is outstanding, proved end to end and on the recorded run</name>
  <files>src/oneoff/stream.rs, src/oneoff/runtime.rs, src/oneoff/session.rs, src/oneoff/launch.rs, tests/oneoff_e2e.rs, tests/fixtures/oneoff/real-bg-subagents.stream.jsonl, tests/fixtures/oneoff/real-resumed.stream.jsonl, tests/fixtures/oneoff/README.md, docs/CONFIGURATION.md, docs/TESTING.md</files>
  <precondition>Both evidence streams exist and are non-empty: `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-220759/stream.jsonl` and `~/.local/share/gsd-meta-manager/oneoff/f4c69b74/oneoff-20261103-221445/stream.jsonl` (prior artifact; plan ordering does not create it). Halt if either is missing.</precondition>
  <read_first>
    - src/oneoff/session.rs (whole file, 644 lines)
    - src/oneoff/stream.rs :1-72 and :178-234 and the tests :555-680
    - src/oneoff/runtime.rs :347-490 and the tests :859-982
    - src/oneoff/launch.rs :855-875 (the Supervisor construction)
    - tests/oneoff_e2e.rs :1-130 and :704-830
    - tests/launcher_support/mod.rs :175-205 and :505-545
  </read_first>
  <action>
Start red: write the e2e test below first and watch it fail on the current tree (the session refuses the message), then implement.

1. Events (src/oneoff/stream.rs, per D-01). Add to `StreamEvent`: `BackgroundTasks { count: u32 }` (the outstanding background set changed; count = its size), `TaskStarted { task_id: String }`, `TaskEnded { task_id: String }`. Extend `TurnResult` with `queued_turns: u32` and `subagents_outstanding: u32` (0 when the line does not say). Add the three new variants to `Renderer::render`'s silent arm (they render nothing). Update every `TurnResult` literal (runtime.rs, session.rs, stream.rs tests) with the two new fields.

2. Classifier (src/oneoff/runtime.rs `classify_claude`, per D-01; tolerant of older agent versions and the fake tooling: absent or malformed fields mean "no evidence of outstanding work").
   - `system` arm: subtype `background_tasks_changed` with a `tasks` array -> `BackgroundTasks { count }` (array length, saturating into u32); `task_started` with a string `task_id` -> `TaskStarted`; `task_notification` whose `status` is terminal -> `TaskEnded`; `task_updated` whose `patch.status` is terminal -> `TaskEnded`. Terminal statuses: completed, failed, killed, stopped, error. Task ids go through `one_line(id, 80)`. Anything else (including `init`, non-terminal updates, missing ids) stays `System { subtype }` exactly as today.
   - `result` arm: `queued_turns` from `queued_turn_count` (u64 -> u32, absent = 0); `subagents_outstanding` = `subagent_stats.spawned` minus `completed` minus `failed` minus (`killed.parent` + `killed.system` + `killed.user`), every field optional (absent = 0), saturating at 0.
   - Unit test `system_task_events_and_result_counters_are_classified` with real-shape lines taken from run A (L30 four-task `background_tasks_changed` reduced to ids, L31 `task_started`, L179 `task_updated` with `patch.status` completed, L180 `task_notification` completed, a `task_updated` whose patch status is `running` -> `System`, the L91 result's `subagent_stats` -> outstanding 4 and queued 0, a result with no `subagent_stats` -> 0). Keep `results_rate_limits_and_system_lines_are_sanitised` passing (its `init` line is still `System`).

3. Lifecycle (src/oneoff/session.rs `SessionLifecycle`, per D-01; stays a pure function of events, `now` and a `report_present: bool`).
   - New constants next to `REPLAY_SETTLE`: `IDLE_SETTLE` = 120 s and `IDLE_SETTLE_ENV` = `GSDMM_ONEOFF_IDLE_SETTLE_SECS`; `idle_settle_from_env()` and `idle_settle_from(value: Option<&str>)` built on `super::cancel::grace_with_default` exactly like `grace_from` (integer 1..=600, else the default; read by the host daemon at session start, never settable by the sandboxed agent).
   - New state: the idle settle in force, the instant of the last event seen, the number of `System { subtype: "init" }` events seen, `Option<u32>` latest `BackgroundTasks` count, a `BTreeSet<String>` of started-not-ended task ids (ignore inserts once it holds 1024 ids, T-mnr-01), and the latest result's `queued_turns` and `subagents_outstanding`. `SessionLifecycle::new()` keeps the 120 s default; add `with_idle_settle(Duration)`.
   - `saw(event, now)`: every event refreshes the last-event instant. `UserReplay`: replayed += 1 and clears `result_since` (unchanged). `System { subtype == "init" }`: counts; every init AFTER the first clears `result_since` exactly like a replay (the agent starts a new turn on each task notification). `TurnResult`: sets `result_since = Some(now)` and stores its `queued_turns` and `subagents_outstanding`. `BackgroundTasks`: stores the count. `TaskStarted`/`TaskEnded`: insert/remove the id.
   - `pub fn outstanding(&self) -> u32`: the max of (the latest `BackgroundTasks` count when one was ever seen, otherwise the task-id set size) and the latest result's `subagents_outstanding`.
   - `decide(&mut self, now: Instant, report_present: bool) -> LifecycleStep`, in this order: input already closed -> Keep; no result since the last turn start -> Keep; `outstanding() > 0` or the latest result's `queued_turns > 0` -> Keep; replays < writes -> CloseStdinWithWarning once `REPLAY_SETTLE` passed since the result, else Keep (the existing path, now only reachable with nothing outstanding); `report_present` -> CloseStdin; idle (now minus the later of the result instant and the last-event instant) >= the idle settle -> the new `LifecycleStep::CloseStdinIdle`; else Keep. Every close marks the input closed, as today.
   - `Supervisor::new` gains `report_file: PathBuf` and `idle_settle: Duration`; it builds the lifecycle with `with_idle_settle`. `tick` computes `report_present` each tick as `self.report_file.symlink_metadata().is_ok()` (presence only, no symlink follow; the report is still re-validated by `read_validated` at finalize) and passes it to `decide`. On `CloseStdinIdle` push the warning `closed the agent's input after {secs}s idle with no report` and close the input. Update the module and `SessionLifecycle` doc comments to state the new end rule. Backstops (wall clock, `--max-budget-usd`, soft-cap grace stop, user stop/kill) stay untouched.
   - src/oneoff/launch.rs `run_sandboxed_session`: pass `spec.report_dir.join(REPORT_FILE)` and `idle_settle_from_env()` to `Supervisor::new` (resolver sessions use the same function with their own report dir, so they get the same rule).

4. Fixtures (per D-07). Write a trimming script in YOUR session scratchpad (never in the repo; run python with `-I`), reading the two evidence streams and writing `tests/fixtures/oneoff/real-bg-subagents.stream.jsonl` (run A) and `tests/fixtures/oneoff/real-resumed.stream.jsonl` (run C), one output line per input line so line indices stay comparable. Whitelist, do not blacklist:
   - every line: keep `type`, `subtype`, `parent_tool_use_id`, `isReplay`; drop `session_id`, `uuid` and everything else not listed below.
   - ids: map every message id, tool-use id, task id and agent id to a deterministic fake that preserves equality (for example `msg_fixture_001`, `toolu_fixture_001`, `task_01`, `agent_01`).
   - `assistant`: keep `message.id`, `message.model`, `message.role`, `message.usage` (whole object, it holds the 1-hour split); content blocks become `{type:text, text:"(text)"}` or `{type:tool_use, id, name, input:{}}`; thinking blocks keep only their type.
   - `user`: replay text becomes `(prompt)` or `(message)`; tool results keep `type`, `tool_use_id`, `is_error` and content `(result)`.
   - `system`: keep `task_id`, `tool_use_id`, `is_backgrounded`, `status`, `patch.status`, `usage` counts, and for `background_tasks_changed` a `tasks` array of `{task_id, task_type, subagent_type}`; `init` keeps only `type`, `subtype`, `model`.
   - `result`: keep `subtype`, `is_error`, `num_turns`, `total_cost_usd`, `result_index`, `queued_turn_count`, `subagent_stats`, `origin.kind`, `modelUsage`, `usage`; drop the result text and every other key.
   - `rate_limit_event`: keep `rate_limit_info` whole.
   Then run the leak gate in this task's `<verify>`; any hit means fix the script and regenerate, never hand-edit. Write `tests/fixtures/oneoff/README.md` (source runs, agent version 2.1.317, the whitelist above, that the files are regenerated not edited, and the leak check). Add a `oneoff/` row to the fixtures table of docs/TESTING.md.

5. Unit tests in src/oneoff/session.rs (update the five existing lifecycle tests to `decide(t, true)` where they expect an immediate close). New: `background_work_keeps_stdin_open_past_a_result` (count 2, result, decide with report present -> Keep; count 0 then a later init then a result -> CloseStdin); `task_events_count_when_no_background_event_was_seen`; `subagent_stats_and_queued_turns_hold_the_session_open`; `a_later_init_starts_a_new_turn`; `without_a_report_the_session_closes_after_the_idle_settle` (119 s Keep, 120 s CloseStdinIdle, an event at 60 s moves the idle clock); `the_replay_settle_waits_for_background_work`; `the_idle_settle_override_is_bounded` (mirror `the_grace_override_is_bounded`). Fixture replays through `crate::oneoff::runtime::AgentRuntime::default().classify` and `include_str!("../../tests/fixtures/oneoff/real-bg-subagents.stream.jsonl")`: `the_recorded_background_run_closes_only_at_its_final_result` — call `wrote` once for the prompt, feed every line with a synthetic clock, call `decide(now, true)` after every `TurnResult`; assert the first result answers Keep with `outstanding() == 4`, `decide(first_result + 300 s, false)` is still Keep, and the first non-Keep answer comes at the first result after the line whose `BackgroundTasks` count reached 0. `the_recorded_resumed_run_closes_at_its_result_once_reported` (run C fixture: with the report -> CloseStdin at the result; without it -> Keep, then CloseStdinIdle after the settle).

6. End to end (tests/oneoff_e2e.rs). In the `task()` builder add `.env("GSDMM_ONEOFF_IDLE_SETTLE_SECS", "1")` with a one-line comment (fake scenarios that end a turn without a report must not wait 120 s; every e2e launch goes through `task()`). Grep the file for other direct `hermetic_command()` launches of `task run`/`task resume`/`task land` and give them the same variable. New scenario `BACKGROUND_SCENARIO` and test `background_work_keeps_the_session_open_for_messages`: the scenario (after the shim's prompt replay) prints an `init`, a one-task `background_tasks_changed`, a `task_started`, and a result whose `subagent_stats` say 1 spawned 0 completed; touches `started`; reads ONE stdin line and replays it; prints a `task_notification` completed, an empty `background_tasks_changed`, an `init`; calls `gsd-meta-manager taskctl report --status done --headline "Background work done"`; prints a final result with 1 completed; then reads stdin to EOF and touches `eof`. The test starts the run detached, waits for `scenario_started()`, sleeps 3 s (three idle settles) and asserts `eof` does not exist yet, sends `task send <alias> "status check"` and asserts exit 0 and `delivered`, then `wait_finished` and asserts outcome `finished`, report `done`, the `eof` file, and exactly one `message` record in `injections.jsonl` with outcome `delivered` and none `refused`.

7. Docs (docs/CONFIGURATION.md): rewrite the session-end sentences (:303-306) to the D-01 rule in plain words (turn over, nothing pending, no background task outstanding, no queued turn, and the agent's report written, or 120 s with no new stream event and no report, which is recorded as a warning; injections, a pause and the soft-cap wrap-up are delivered during background work); add a `GSDMM_ONEOFF_IDLE_SETTLE_SECS` row to the env table (:122-125) in the style of the soft-cap grace row (Tests only, `src/oneoff/session.rs` `idle_settle_from_env`, integer 1-600, default 120, read by the daemon at session start).

Commit: `fix(quick-261103-mnr): keep a one-off session open while background work is outstanding`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff::session oneoff::runtime oneoff::stream && cargo test --test oneoff_e2e -- background_work_keeps_the_session_open_for_messages the_prompt_arrives_on_a_piped_stdin task_send_is_delivered attach_turns_stdin_lines a_soft_cap_wraps_up_once && cargo test --test oneoff_runtime_seam_guard && test -s tests/fixtures/oneoff/real-bg-subagents.stream.jsonl && test -s tests/fixtures/oneoff/real-resumed.stream.jsonl && ! grep -niE 'blk|/home/|/tmp/|@|(^|[^a-z])sk-|bearer|oauth|apikey|anthropic|fbcffc10' tests/fixtures/oneoff/real-bg-subagents.stream.jsonl tests/fixtures/oneoff/real-resumed.stream.jsonl && test -z "$(grep -ohiE '"[a-z0-9_]*token[a-z0-9_]*"' tests/fixtures/oneoff/real-bg-subagents.stream.jsonl tests/fixtures/oneoff/real-resumed.stream.jsonl | sort -u | grep -viE '^"[a-z0-9_]*tokens(_delta|_details)?"$')" && test "$(wc -l < tests/fixtures/oneoff/real-bg-subagents.stream.jsonl)" -eq 304 && test "$(grep -v '^\s*//' src/oneoff/session.rs | grep -c 'GSDMM_ONEOFF_IDLE_SETTLE_SECS')" -ge 1 && test "$(grep -c 'GSDMM_ONEOFF_IDLE_SETTLE_SECS' docs/CONFIGURATION.md)" -ge 1</automated>
  </verify>
<!-- planner-discipline-allow: blk -->
<!-- planner-discipline-allow: fbcffc10 -->
  <done>On the fake agent, a message sent while a background task is outstanding is delivered and the session ends finished only after the final result with a report; the run-A fixture replay keeps stdin open at the first result and closes at the final one; run C closes at its result when reported and after the idle settle when not; the fixtures pass the leak gate (baseline at HEAD: the files do not exist, so the gate cannot pass before this task); the idle-settle env var is documented. Full suite raw (`rtk proxy cargo test --no-fail-fast > target/mnr-t1.log 2>&1`) shows every binary present and 0 failed versus `target/mnr-t0.log`; clippy clean.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: honest meter: 1-hour cache price, visible-output estimate, no double count, real rate-limit shape, live spend in task status, lower-bound labels</name>
  <files>src/oneoff/runtime.rs, src/oneoff/stream.rs, src/task_cli.rs, src/ui/screens/oneoff_viewer.rs, src/ui/screens/help.rs, tests/oneoff_e2e.rs, docs/CONFIGURATION.md</files>
  <read_first>
    - src/oneoff/stream.rs :170-312 and :464-553 and the tests :605-706, :844-855
    - src/oneoff/runtime.rs :170-255, :359-490 and the tests :1003-1033
    - src/task_cli.rs :300-540 and :895-950
    - src/ui/screens/oneoff_viewer.rs :440-480 and :630-660
    - docs/CONFIGURATION.md :303-333
  </read_first>
  <behavior>
    - opus-5-5 {input 44, output 8140, cache_creation 54218 of which 1h 54218, cache_read 885513} prices within $0.001 of 0.7738226; haiku-5-5 {84, 60295, 203769 all 1h, 1700463} within $0.001 of 0.0879143; 1M unsplit cache-creation tokens on opus-5-5 still price at $5.00.
    - The run-A L12 rate-limit line (session_id/uuid removed) classifies as status allowed, kind seven_day, utilization 0.21, resets 1794326400; with no `rateLimitType` and windows five_hour 0.4 / seven_day 0.7, kind is seven_day and utilization 0.7 with that window's reset; the old top-level shape keeps its current result.
    - A usage of 1 output token whose message showed 4000 visible characters counts 1000 output tokens; characters accumulate over the lines of one message id; a message id seen again after a result is ignored (estimate stays at the reported figure).
    - Right after a result with a reported cost the estimate is exact; any later usage makes it not exact; folding the run-A fixture with `meter_of_file` ends at 0.86173693 (within 1e-6) and exact, with the latest rate limit seven_day 0.21.
    - `meter_text(.., lower_bound = true)` contains `spend ≥ $1.23 (lower bound) / soft off / hard $10.00 (notional)`; with `lower_bound = false` it contains `spend $1.23 / soft off / hard $10.00 (notional)`.
  </behavior>
  <action>
1. Pricing (src/oneoff/runtime.rs, per D-06 (a)). Add `cache_creation_1h: u64` to `UsageTokens` (in stream.rs; the 1-hour subset of `cache_creation`, 0 when the split is absent); `classify_claude` reads it from `usage.cache_creation.ephemeral_1h_input_tokens`. In `notional_usd`, clamp the 1-hour part to `cache_creation`, price it at a named constant factor 2.0 x the row's input price and the rest of `cache_creation` at the row's `cache_write` (1.25 x input, as every row already is). Update the `PRICES` doc comment to say both rates. Unit test `one_hour_cache_writes_reproduce_the_reported_cost` with the D-06 numbers above (CONTEXT §specifics); keep `the_price_table_is_notional_and_the_longest_prefix_wins` unchanged and passing.

2. Visible output (per D-06 (b)). `StreamEvent::Usage` gains `content_chars: u64`: the character count of the line's text blocks (raw text, BEFORE `bounded_text` cuts it) plus the length of each `tool_use` block's `input` serialized as compact JSON; thinking blocks add 0. Update the existing `an_assistant_line_yields_text_tool_calls_and_usage_in_order` expectation and every `Usage` literal (stream.rs `usage()` helper and tests).

3. Estimate (src/oneoff/stream.rs `SpendEstimate`, per D-06 (b)(c)). Per message id keep the model, the latest tokens, the accumulated `content_chars` and the cost; each `Usage` adds its chars and replaces the tokens with the latest copy, then prices it with `output = max(tokens.output, chars.div_ceil(4))`. Keep a bounded `BTreeSet<String>` of ids already folded into a baseline: a `TurnResult` moves every pending id into it before clearing; a `Usage` whose id is in it is ignored; the existing `MAX_TRACKED_MESSAGES` overflow also moves the pending ids into it, and the set is cleared when it exceeds `MAX_TRACKED_MESSAGES`. Track exactness: `is_exact()` is true only when the latest spend-changing event was a `TurnResult` with a reported cost and the high-water equals that baseline (within 1e-9). `usd()` keeps its never-decreasing high-water. Replace the doc comment's claim that subagent usage is only partly visible: subagent messages are forwarded and folded; the live figure undercounts because streamed output counts are message-start snapshots and thinking is not visible, so it is a lower bound until a result snaps it to the reported cumulative total.

4. Rate limit (src/oneoff/runtime.rs `rate_limit_event` arm, per D-03). Source order for utilization: `rate_limit_info.unifiedWindows[rateLimitType].utilization`; else the highest utilization across all `unifiedWindows` entries (that window's name, sanitized through `lower_word`, becomes `kind` when `rateLimitType` is missing); else the legacy top-level `utilization`. `resets_at` prefers the chosen window's `resetsAt`, then the top-level one. Keep the finite and >= 0 filter. Add `the_real_rate_limit_shape_reads_the_window_utilization` (the cases in `<behavior>`); keep the old-shape asserts.

5. Meter (src/oneoff/stream.rs). `Meter` gains `exact: bool` (from `SpendEstimate::is_exact`); add `pub fn meter_of_file(path: &Path) -> Meter` (the same `StreamTail` drain, classifier and fold as `spend_of_file`, plus the latest `RateLimit` as `RateLimitSeen`), and make `spend_of_file` return `meter_of_file(path).estimate`. `meter_text` gains a final `lower_bound: bool`: true renders `spend ≥ $X (lower bound)`, false renders `spend $X`; the rest of the line is unchanged; fix its doc comment. Callers pass `live || !meter.exact`: the viewer header (`stream.info.live.may_be_live()`), the `task attach` closing line (`view.live.may_be_live()`). Update the viewer test at oneoff_viewer.rs:649 and the stream.rs meter test to the new texts; grep `src`, `tests` and `docs` for the old `spend ≈` text and leave none behind. Add the stream.rs tests named in `<behavior>`, including `the_recorded_background_run_meters_to_its_reported_total` reading `Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/oneoff/real-bg-subagents.stream.jsonl")`.

6. `task status` (src/task_cli.rs, per D-05). A run is live here when `view.live.may_be_live()` and its record is not `Finished`. For a live run, `summary_lines` folds `<run_dir>/stream.jsonl` once with `meter_of_file` and, in place of the recorded spend line, prints `spend:     ≥ $X so far of $Y (live estimate, lower bound; exact at the end)` where X = `record.spend_usd.unwrap_or(0.0)` (a resume merge's resolver spend already on the record) + the estimate and Y = `record.max_budget_usd`; then `soft cap:  $S` (with ` (hit HH:MM)` when `soft_cap_hit_at` is set) when a soft cap is set, and `rate:      <rate_text>` when a rate limit was seen. Finished runs keep the existing `spend:` line unchanged (no `≥`). For `--json`, `status_main` prints `view_json` extended, for a live run only, with `live_spend_usd`, `live_spend_is_lower_bound: true` and `rate_limit` (`{status, kind, utilization, resets_at}` or null); `list --json` is unchanged. Every agent-influenced string goes through `shown()` as today.

7. e2e (tests/oneoff_e2e.rs): `task_status_shows_live_spend_and_the_rate_limit_of_a_running_run` with a scenario that prints the real-shape rate-limit line (no session_id/uuid), an assistant line pricing at $4 (1M opus-5-5 input tokens, like `SOFT_CAP_SCENARIO`), touches `started`, waits for `release`, then prints a result with `total_cost_usd` 4.5 and exits. While it waits: `task status` stdout contains `≥ $4.00 so far`, `lower bound` and `rate:      allowed seven_day 21%`; `task status --json` has `live_spend_usd` >= 4.0, `live_spend_is_lower_bound` true, `rate_limit.kind` `seven_day`, `rate_limit.utilization` 0.21. After `release` and `wait_finished`: stdout has `$4.50` on the `spend:` line and no `≥`, and `--json` has no `live_spend_usd`.

8. Help and docs. The soft-cap row in `src/ui/screens/help.rs` (all three identical copies) says the cap is judged against the lower-bound spend estimate, e.g. `In the live view: set or clear the soft spend cap (judged against the lower-bound estimate; asks the agent to wrap up, then stops it)`. docs/CONFIGURATION.md: rewrite the Spend meter paragraph (`spend ≥ $X (lower bound)` while live, exact `spend $X` once a result reported it; 1-hour cache writes at 2 x input; output estimated from visible text and tool input, thinking unseen; subagent usage IS included; the rate-limit window utilization; the `task status` live lines and the three `--json` fields) and the Soft cap paragraph's last sentence per D-06 labelling (judged against the lower bound, can fire late by roughly the unseen output/thinking of the current turn, snapped to the exact cumulative figure at each result; `--max-budget-usd`, enforced by the agent itself, is exact). Remove the now-wrong "subagent usage is only partly visible" and "can undercount subagents" claims.

Commit: `fix(quick-261103-mnr): honest one-off meter, real rate-limit shape and live spend in task status`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff::runtime oneoff::stream oneoff::session ui::screens::oneoff_viewer ui::screens::help && cargo test --test oneoff_e2e -- task_status_shows_live_spend a_soft_cap_wraps_up_once attach_prints_a_finished_run && cargo test --test oneoff_runtime_seam_guard && test "$(grep -v '^\s*//' src/oneoff/runtime.rs | grep -c 'unifiedWindows')" -ge 1 && test "$(grep -v '^\s*//' src/oneoff/runtime.rs | grep -c 'ephemeral_1h_input_tokens')" -ge 1 && test "$(grep -c 'live_spend_usd' src/task_cli.rs)" -ge 1 && test "$(grep -c 'lower bound' docs/CONFIGURATION.md)" -ge 1 && test "$(grep -cE 'lower.bound' src/ui/screens/help.rs)" -ge 3</automated>
  </verify>
  <done>The pricing check reproduces run A's reported costs; the run-A fixture meters to exactly $0.86174; the live meter is labelled a lower bound while live and exact after a reported result; `task status` shows live spend, soft cap and rate (`seven_day 21%`) for a running run and the exact recorded spend once finished; help and docs describe the lower bound and the soft cap's judgement. Baselines at HEAD 9a93bdc for the grep gates: `unifiedWindows` 0, `ephemeral_1h_input_tokens` 0, `live_spend_usd` 0, `lower bound` in docs 0, `lower.bound` in help 0. Full suite raw to `target/mnr-t2.log`: every binary present, 0 failed; clippy clean.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: a pause is confirmed or refused, never assumed; the report decides paused vs finished; the landing counts what landed</name>
  <files>src/oneoff/cancel.rs, src/oneoff/session.rs, src/oneoff/launch.rs, src/oneoff/inbox.rs, src/oneoff/land.rs, src/task_cli.rs, src/app.rs, src/ui/screens/oneoff.rs, src/ui/screens/render_escape_guard.rs, tests/oneoff_e2e.rs, docs/CONFIGURATION.md</files>
  <read_first>
    - src/oneoff/cancel.rs (whole file, 525 lines)
    - src/oneoff/launch.rs :440-520, :876-1000, :1282-1490 and the tests from :1530
    - src/oneoff/session.rs :344-378 (deliver, inject_host)
    - src/oneoff/inbox.rs :1-60 and :295-325
    - src/oneoff/land.rs :230-250 and :525-585
    - src/task_cli.rs :385-406 and :676-760
    - src/app.rs :1795-1860 (grep `stop_with` first)
    - src/ui/screens/oneoff.rs :1120-1140 and :1745-1830
    - tests/oneoff_e2e.rs :1629-1801 and :2020-2060
  </read_first>
  <behavior>
    - `Cancel`: Pause -> InjectPause; `withdraw_pause()`; later Pause ticks answer Wait forever (no Term at t+400 s, `cutting()` false, `end()` None); a Hard after withdrawal still answers Term.
    - `pause_end_with_report(Paused, Some(Partial))` = Paused; Paused with Some(Done) / Some(Blocked) / Some(Failed) / None = Exited; UserStop, Killed, WallClock, SoftCap and Exited pass through unchanged whatever the report.
    - `inbox::pause_outcome_since(dir, since)` returns the first `pause` record whose `at` >= `since`, ignores older pause records and other kinds, and is None for a missing file.
    - A `StopRecord` JSON without `refused` reads back with `refused: None`, and one with `refused: None` serializes without the key.
    - The landing message: counts (3, agent 1) -> `landed 3 commit(s) of r on main (1 by the agent; now at 0123456789ab)`; (2, 2) -> `landed 2 commit(s) of r on main (now at 0123456789ab)`; count unknown -> the agent's count only, with no claim beyond it.
    - `StopOutcome::PauseRefused` and `PauseUnconfirmed` report `is_refusal() == true` and name the run and the reason; `PauseDelivered` does not.
  </behavior>
  <action>
1. Cancel and record (src/oneoff/cancel.rs, per D-02). `StopRecord` gains `refused: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` (`begun` sets None; fix the literals in src/ui/screens/oneoff.rs tests and src/ui/screens/render_escape_guard.rs). `Cancel` gains a `pause_withdrawn` flag and `pub fn withdraw_pause(&mut self)`: from `Pausing`, back to `Idle` with the flag set; while the flag is set, `(Idle, Some(Pause))` answers Wait (match it before the existing Idle/Pause arm). Hard and Kill keep their arms. Add `a_refused_pause_is_withdrawn_and_never_cut` and `a_hard_stop_after_a_withdrawn_pause_still_cuts`.

2. Delivery result (src/oneoff/session.rs, per D-02). `inject_host` returns `Result<(), String>`: Ok when it recorded the pause `delivered` (the existing deliver path, recorded before the write as today); Err(reason) when it recorded a refusal (the existing `the session is ending` condition). Keep recording behaviour identical otherwise.

3. Wait loop (src/oneoff/launch.rs `run_sandboxed_session`, per D-02). Track `pause_delivered: bool`. In the `InjectPause` arm: Ok -> `pause_delivered = true`; Err(reason) -> `cancel.withdraw_pause()`, set the current stop record's `refused = Some(reason)` and `ended_at` to now, and emit `SessionEvent::Stop` with it so run.json shows the refusal at once. At the natural exit, `Paused` additionally requires `pause_delivered` (a stale or refused pause request falls through to `Exited`). Leave `resume_master`'s pre-session pause mapping alone (not a natural exit).

4. Outcome precedence (src/oneoff/launch.rs, per D-02). Add the pure `fn pause_end_with_report(end: HostEnd, report: Option<ReportStatus>) -> HostEnd` (Paused survives only with `Partial`; every other end passes through) and apply it in `run_session` to `session.end` right before `finalize`, using `read_validated(&report_dir.join(REPORT_FILE)).ok().map(|r| r.status)`. Hard and kill precedence is unchanged. Unit test `the_report_decides_whether_a_delivered_pause_is_paused` (cases in `<behavior>`).

5. Confirmed pause (src/oneoff/inbox.rs and launch.rs `stop()`, per D-02). Add `pub fn pause_outcome_since(run_dir: &Path, since: &str) -> Option<InjectionRecord>` (drain `injections.jsonl` with `StreamTail` like `wait_outcome`; first record of kind `Pause` with `at.as_str() >= since`; both sides are `rfc3339` seconds-precision UTC strings). Replace `StopOutcome::PauseRequested` with `PauseDelivered { run_id, pid }`, `PauseRefused { run_id, reason, ended: bool }` and `PauseUnconfirmed { run_id, pid }`, plus `pub fn is_refusal(&self) -> bool` (true for the last two). In `stop()`'s held-lock branch, keep the request `write_request` returns; when it is a pause, poll every `WAIT_POLL` until `wait` runs out: a delivered record -> `Stopped` if the run lock is already free, else `PauseDelivered`; a refused record -> `PauseRefused { reason, ended: lock free }`; the run lock found free with no record -> scan once more, then `PauseRefused { reason: "the run ended before the pause reached it", ended: true }`; the deadline -> `PauseUnconfirmed`. A merged request that is stronger than a pause keeps today's hard/kill waiting. Messages (wording per CONTEXT D-02): delivered `<run>: the pause was delivered; the agent is winding down (the launcher cuts it after its pause grace)`; refused and still running `the pause could not be delivered to <run> (<reason>); it keeps running - use 'task stop' (without --pause) to stop it`; refused and ended `the pause could not be delivered to <run> (<reason>); the run has ended on its own`; unconfirmed `the pause of <run> was requested but its delivery is not confirmed yet; check 'task status' (the launcher may still deliver it)`. Unit test `pause_refusals_are_failures_with_their_reason`.

6. Exit codes and UI (per D-02). src/task_cli.rs `stop_main`: `PauseDelivered` -> stdout `task stop: <message>`, exit 0; `PauseRefused` -> stderr `task stop: <message>`, exit 1; `PauseUnconfirmed` -> stderr `task stop: <message> (after <wait_secs>s)`, exit 1. src/app.rs `stop_with`: a pause of a run (`StopTarget::Run`) waits 10 s instead of zero (it runs in `spawn_blocking`; a landing pause stays at zero), and an `Ok(outcome)` with `outcome.is_refusal()` becomes `Err(outcome.message())`, so the status line reads `pause refused: ...`. The stop line in `task status` (`summary_lines`) and the TUI summary (src/ui/screens/oneoff.rs) show a stop with `refused` as `pause refused (<reason>)` / `Pause refused (<reason>)` instead of pausing/paused; extend `the_summary_names_a_pending_and_a_finished_stop` with a refused case.

7. Landing count (src/oneoff/land.rs ff path, per D-04; message-only per D-DISC). After the ff merge succeeds, count `git rev-list --count <old_head>..<head>` in the project with the existing `git_in` helper (old_head is the checkout HEAD captured before the merge). Build the line through a small pure `fn landed_message(landed: Option<u32>, agent: u32, run_id: &str, branch: &str, short: &str) -> String` (cases in `<behavior>`), replacing the old `merged N commit(s) ...` line. Unit test `the_landing_message_counts_what_landed`.

8. e2e (tests/oneoff_e2e.rs).
   - `a_pause_refused_because_the_session_is_ending_exits_non_zero_and_is_withdrawn`: scenario (after the shim's prompt replay) runs `gsd-meta-manager taskctl report --status done --headline "Done early"`, prints a result, reads stdin to EOF (the launcher closes it: report present, nothing outstanding), touches `eof` then `started`, and waits for `release`. Start it with `start_with_graces(&fx, SCENARIO, "1", "1")`. `task stop <alias> --run-id <id> --pause --wait-secs 10` exits 1 and its stderr contains `could not be delivered` and `the session is ending`; `injections.jsonl` holds exactly one `pause` record, outcome `refused`; `task status --json` shows `record.stop.refused` = `the session is ending`. Sleep 3 s (past the 1 s pause grace), release, `wait_run_finished`: outcome `finished` (NOT `killed`), report `done`, `stop.kind` `pause`, and the shim log has no `--signal=TERM`.
   - `a_done_report_after_a_pause_request_ends_finished_not_paused`: scenario replays stdin lines; on the pause message it reports `--status done --headline "Finished anyway"`, prints a result and exits 0. Start with graces "5"/"60"; the pause exits 0; the finished record has outcome `finished`, report `done`, `stop.kind` `pause`, and one `pause` record `delivered`.
   - Extend `resume_from_master_merges_the_checkout_tip_into_the_run_first`: after its existing asserts, `task land merge <alias> <new>` exits 0 and stdout contains `landed 3 commit(s) of <new>` and `(1 by the agent` (the old run's wip commit, the host merge commit and the resumed commit, as in run C).
   - The existing `a_paused_run_commits_its_wip_reports_partial_and_is_tracked` and `a_pause_the_agent_ignores_is_cut_after_its_grace` must stay green unchanged (delivered + partial -> paused; delivered + ignored -> cut, killed).

9. Docs (docs/CONFIGURATION.md): in the stop/pause paragraph (:225-245) state that `task stop --pause` waits up to `--wait-secs` for the launcher's record and exits 0 only when the pause was delivered, 1 when it was refused (for example the session is already ending after the agent reported) or not confirmed; that a refused pause is withdrawn (no grace cut, `stop.refused` in run.json, `task status` says refused); that `paused` needs a delivered pause AND a `partial` report, a `done` report wins (`finished`) and a missing or `blocked` report falls back to the normal outcome; and that `z` in the TUI shows a refusal as an error. Replace "a run still winding down exits 0" accordingly. In the `ff` landing bullet (:374-375) add that the message counts the commits the branch advanced by, which exceeds the run's own count for a resumed run, and names the agent's count when they differ.

Commit: `fix(quick-261103-mnr): confirm or refuse a pause, let the report decide paused, count what a landing landed`.
  </action>
  <verify>
    <automated>cargo test --lib -- oneoff::cancel oneoff::launch oneoff::inbox oneoff::land oneoff::session ui::screens::oneoff && cargo test --test oneoff_e2e -- a_pause_refused_because_the_session_is_ending a_done_report_after_a_pause_request a_paused_run_commits_its_wip a_pause_the_agent_ignores resume_from_master_merges_the_checkout_tip pausing_a_resolver && cargo test --test oneoff_runtime_seam_guard && test "$(grep -v '^\s*//' src/oneoff/cancel.rs | grep -c 'refused')" -ge 1 && test "$(grep -v '^\s*//' src/oneoff/launch.rs | grep -c 'pause_outcome_since')" -ge 1 && test "$(grep -v '^\s*//' src/oneoff/land.rs | grep -c 'rev-list')" -ge 1</automated>
  </verify>
  <done>A refused pause exits 1 with its reason, is withdrawn (no grace cut), is recorded in run.json and shown as refused; a delivered pause exits 0; a done report after a pause ends finished; delivered + partial still ends paused; the TUI shows refusals as errors; the resume-from-master landing reports 3 landed with 1 by the agent. Baselines at HEAD 9a93bdc: `refused` in cancel.rs 0, `pause_outcome_since` 0, `rev-list` in land.rs 0. Full suite raw to `target/mnr-t3.log`: every binary present, 0 failed; `cargo clippy --all-targets -- -D warnings` clean (host, `x86_64-pc-windows-gnu`, `aarch64-apple-darwin`); `cargo fmt --check` clean.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| sandboxed agent -> host daemon | `stream.jsonl` events (background tasks, results, usage, rate limit) and the writable `report/` dir are agent-influenced and drive the session-end decision and the meter |
| operator CLI/TUI -> daemon | `stop.request` (host-only) and the daemon-written `injections.jsonl` (0600, never mounted) carry the pause request and its delivery verdict |
| stream -> terminal | `task status` and the TUI render agent-influenced strings (rate-limit window names, refusal reasons) |
| real paid-run recordings -> git | the committed fixtures are derived from real streams that held paths, ids and text |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-mnr-01 | Denial of service | `SessionLifecycle` outstanding/idle tracking | medium | mitigate | An agent claiming endless background work or streaming forever keeps only its own session open: the wall-clock cap, `--max-budget-usd`, the soft-cap grace stop and user stop/kill are unchanged backstops (Task 1). The task-id set stops growing at 1024 ids; counts parse into u32 with saturation. |
| T-mnr-02 | Tampering | report-present close signal | low | accept | Creating `report/report.json` (or a symlink there) early only ends the agent's own input sooner; presence is checked with `symlink_metadata` (no follow) and the report is still re-validated by `read_validated` at finalize. |
| T-mnr-03 | Spoofing | pause delivery status (CLI exit code, TUI status) | medium | mitigate | Success comes only from the daemon-written `injections.jsonl` record (host-only, 0600, never mounted, so the agent cannot forge it); refused and unconfirmed exit 1 / show as errors (Task 3). |
| T-mnr-04 | Repudiation | a refused pause | low | mitigate | The refusal is recorded twice: `injections.jsonl` (refused + reason) and run.json `stop.refused`, and `task status` shows it (Task 3). |
| T-mnr-05 | Information disclosure | committed fixtures from real paid runs | high | mitigate | Whitelist trimming in a scratchpad script (drops session ids, uuids, init extras, every text body, tool input and tool result; fake ids preserving equality), then the leak gate in Task 1's `<verify>` (user name, home/tmp paths, `@`, key prefix, bearer/oauth/apiKey/ANTHROPIC, the real session id, and token-like strings limited to usage-count keys); regenerate, never hand-edit; the check is recorded in the SUMMARY. |
| T-mnr-06 | Tampering | rendering rate-limit window names and refusal reasons | low | mitigate | Window names pass `lower_word` (`[a-z_]`, else `other`); every printed string goes through `shown()` / `render_for_terminal` as today (Tasks 2, 3). |
| T-mnr-07 | Denial of service | `task status` folding a live `stream.jsonl` | low | mitigate | Only live runs are folded, once per call, through `StreamTail` (4 MiB per line, 8 MiB per poll); streams are under 1 MB in practice (Task 2). |
| T-mnr-08 | Tampering | report decides paused vs finished | low | accept | The report is already the agent's re-validated self-assessment; `paused` additionally needs a host-delivered pause, and hard/kill outcomes stay host-decided. |
| T-mnr-09 | Elevation of privilege | `GSDMM_ONEOFF_IDLE_SETTLE_SECS` | low | accept | Read by the host daemon at session start only, bounded 1..=600, not settable from the sandbox (same as the three existing grace variables). |
| T-mnr-SC | Tampering | npm/pip/cargo installs | high | mitigate | No package is installed or added: no new crate dependency, and `Cargo.toml`/`Cargo.lock` are not staged. No legitimacy checkpoint is needed. |
</threat_model>

<verification>
- Baseline before Task 1: `rtk proxy cargo test --no-fail-fast > target/mnr-t0.log 2>&1` (redirect to a file, never pipe through grep). After each task the same to `target/mnr-tN.log`; read the log and compare the per-binary result lines with t0 (no binary missing, 0 failed; the lib, `oneoff_e2e` and seam-guard counts grow, every other line identical).
- `cargo clippy --all-targets -- -D warnings`, plus `--target x86_64-pc-windows-gnu` and `--target aarch64-apple-darwin`; `cargo fmt --check`.
- `cargo test --test oneoff_runtime_seam_guard` and `cargo test --test spawn_seam_guard` pass (no new process spawn: the landing count uses the existing `git_in`).
- Coverage audit (GOAL / REQ / RESEARCH / CONTEXT): bug 1 + D-01 -> Task 1; bug 2 + D-02 -> Task 3; bug 3 + D-03 -> Task 2; bug 4 + D-04 -> Task 3; bug 5 + D-05 -> Task 2; bug 6 + D-06 -> Task 2; D-07 -> Task 1; docs/help -> Tasks 1-3; REQ: QUICK-261103-mnr (all tasks); RESEARCH: none (no research phase). Deferred/out of scope (not built): any paid run, push, tag, rebase. Open questions in CONTEXT keep their chosen defaults (120 s idle settle; unconfirmed pause exits 1).
- `git rev-list --count 9a93bdc..HEAD` equals the number of task commits plus any orchestrator commits made meanwhile (measured, not narrated, in the SUMMARY).
</verification>

<success_criteria>
- Bug 1: background work no longer ends the session; injections, pause and wrap-up are delivered during it; end = outstanding 0 + no queued turn + report (or 120 s idle, warned).
- Bug 2: an undelivered pause is never a success (CLI exit 1, TUI error), a refused pause is withdrawn and recorded, and a `done` report is never labelled `paused`.
- Bug 3: utilization is read from `unifiedWindows` (`seven_day 21%`).
- Bug 4: the landing message counts what landed and names the agent's count when different.
- Bug 5: `task status` shows live spend (lower bound), soft cap and rate; `--json` carries the three new fields.
- Bug 6: 1-hour cache writes priced at 2 x input, visible-output estimate, no double count after a result; lower-bound labelling in the TUI, CLI, help and docs.
- Fixtures committed and leak-checked; docs/CONFIGURATION.md, the TUI help and docs/TESTING.md describe the new behaviour. No paid run, no push, no tag, no rebase. Three task commits on master.
</success_criteria>

<output>
Create `.planning/quick/261103-mnr-fix-one-off-session-lifecycle-pause-deli/261103-mnr-SUMMARY.md` when done. Include:
- the commits table;
- shipped vs deferred;
- baseline and final per-binary test counts (t0..t3 logs);
- the fixture leak check: the exact gate commands and their (empty) output, and the sorted list of token-like keys left in the fixtures;
- every `[inferred — audit]` decision: D-01..D-07 as carried, D-DISC choices (message-only landing count; names of new types, fields and the env var; message wording), and any new executor ones;
- "Unverified without a paid run": that the real agent ends promptly on stdin EOF after its final notification turn; that it accepts a user message mid-background-work as a new turn; that `background_tasks_changed`, `queued_turn_count` and `subagent_stats` keep their shape in other agent versions; that the live lower bound tracks real spend as in the replica (run A 0.40 vs 0.43 before the first result).
</output>
