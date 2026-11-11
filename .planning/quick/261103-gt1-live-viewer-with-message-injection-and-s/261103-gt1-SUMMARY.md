---
quick_id: 261103-gt1
phase: quick-261103-gt1
plan: 01
subsystem: oneoff
tags: [oneoff, stream-json, inbox, live-viewer, spend-meter, soft-cap, tui, cli]
requires: [quick-261103-f5y]
provides:
  - stream-json stdin for one-off agents (prompt = first message, end-of-turn lifecycle)
  - host-only inbox + injections.jsonl, `task send`
  - runtime-neutral StreamEvent classifier, tail, renderer, spend estimate, follower
  - "`v` live viewer with meter, rate limit, composer (`i`) and soft-cap input (`b`)"
  - soft spend cap (wrap-up + grace + over_budget stop), `task budget`, `task attach`
key-files:
  created:
    - src/oneoff/stream.rs
    - src/oneoff/session.rs
    - src/oneoff/inbox.rs
    - src/ui/screens/oneoff_viewer.rs
    - .planning/todos/pending/2026-11-03-oneoff-raise-hard-ceiling-via-resume.md
  modified:
    - src/oneoff/runtime.rs
    - src/oneoff/launch.rs
    - src/oneoff/store.rs
    - src/oneoff/mod.rs
    - src/launcher/process.rs
    - src/cli.rs
    - src/task_cli.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/{mod,oneoff,help,render_escape_guard}.rs
    - tests/{oneoff_e2e,oneoff_runtime_seam_guard}.rs
    - tests/launcher_support/mod.rs
    - docs/CONFIGURATION.md
  moved:
    - .planning/todos/pending/2026-11-03-oneoff-live-stream-view.md -> completed/
decisions: see "Inferred decisions" below
metrics:
  tasks: 3
  completed: 2026-11-03
status: complete
estimate_actuals_note: no `estimate` frontmatter actuals recorded by the executor (start time was not captured); the plan estimated 380k tokens, 3 tasks
actuals:
  tasks: 3
  commits: 3
plan_head_before: 41ce9f80690ccb67a99af400ef398d79eb859d7f
plan_head_after: a65de2d9afa1fccff069757c1e296ba1d074639b
audit_acknowledged:
  milestone: v2.3.0
  at: 2026-11-10
  status: complete
---

# Quick 261103-gt1: live viewer, message injection and spend meter for one-off runs

A one-off agent now reads stream-json on stdin (prompt = first message), accepts operator
messages through a host-only inbox, can be watched live in a full-screen viewer with a
notional spend meter and rate-limit line, and can be given a live-adjustable soft cap.
Everything shipped, nothing deferred.

## Commits (master, nothing pushed)

| Task | SHA | Message |
|------|-----|---------|
| 1 (tracer) | `7e608a8` | feat(quick-261103-gt1): stream-json stdin, inbox injection and session lifecycle for one-offs |
| 2 | `13fa5a7` | feat(quick-261103-gt1): one-off live viewer, spend meter and message composer |
| 3 | `a65de2d` | feat(quick-261103-gt1): one-off soft spend cap, task budget and task attach |

`commits: 3` is the executor's own. `git rev-list --count plan_head_before..HEAD` reports 4
because `f0d50be chore(release): v2.1.0` (not this plan; another actor, mid-run) sits between
Task 2 and Task 3. I never staged `Cargo.toml`, `Cargo.lock` or `.planning/STATE.md`, which
that release touched in the shared working tree.

## Shipped vs deferred

Shipped (all of it): A (stdin mode, prompt as first message, lifecycle), B (inbox,
`injections.jsonl`, `task send`), D (viewer: replay + follow, scroll, `f`/`G`, composer `i`),
the meter (notional estimate + rate limit), F (docs, help), the soft cap (enforcement,
`task budget`, viewer `b`), `task attach`, the hard-ceiling follow-up todo, and the starting
todo moved to `completed/`.

Deferred: none. No `2026-11-03-oneoff-soft-cap.md` or `...-task-attach.md` todo was needed.
Out of scope by decision (C-19, todo created): raising the hard ceiling via stop + `--resume`.
Follow-up idea, no todo (C-02): `--include-partial-messages` for live token-level typing.

## Verification

- Final raw suite, `rtk proxy cargo test --no-fail-fast` (redirected to a file, not piped):
  **3378 passed; 0 failed; 17 ignored** (58 result lines, exit 0).
- `cargo clippy --all-targets -- -D warnings`: clean for the host, `x86_64-pc-windows-gnu` and
  `aarch64-apple-darwin`. `cargo fmt --check`: clean.
- `git diff a52eda7 -- tests/launcher_sandbox.rs tests/launcher_config_dir.rs
  tests/launcher_e2e.rs tests/spawn_seam_guard.rs src/launcher/sandbox.rs` is empty: no driver
  golden, allowlist or argv edited. `launcher_e2e` (driver stdin `/dev/null`),
  `spawn_seam_guard`, `async_blocking_guard`, `oneoff_runtime_seam_guard` (now listing
  `stream.rs`, `session.rs`, `inbox.rs`, `oneoff_viewer.rs`) and the render-escape guard (new
  `OneoffViewerScreen` adjudication with a hostile-run-id/line/composer fixture) are green.
- e2e on the fake harness (`tests/oneoff_e2e.rs`, 14 tests): prompt on a `pipe:` stdin through
  systemd-run + bwrap shims and the launcher closing stdin at end of turn; `task send`
  delivered, recorded (0600, inbox 0700), replayed, and refused with `not live` afterwards; no
  bwrap bind covers the inbox; soft cap (`task budget --soft 20` refused naming the hard
  ceiling, `--soft 1` delivered, exactly one `wrap_up` record, run ends `over_budget`,
  `run.json` has `soft_cap_usd: 1.0` and `soft_cap_hit_at`); `task attach` on a finished run
  and on a live one with a stdin line.

## Deviations from the plan

None of Rules 1-4 beyond the shape changes below (all within the plan's intent):

- `StreamEvent::Usage` carries `tokens: UsageTokens` rather than four loose fields;
  `notional_usd(model, tokens)` takes the model explicitly.
- `OneoffStreamBatch` carries `epoch` (a per-`v`-press counter; stale batches of an earlier
  viewer's loop are ignored) and `info: RunInfo` (live state, hard ceiling, start/end, soft
  cap, hit time) instead of just `live`.
- `AgentRuntime::wrap_up_message()` landed in Task 1 (the renderer needed it), not Task 3.
- The e2e "no bind covers the inbox" test exempts the read-only root bind `--ro-bind / /`
  and instead asserts a `--tmpfs` masks an ancestor of the run dir (see audit item below).

## Inferred decisions [inferred — audit]

1. `classify` returns `Vec<StreamEvent>`; thinking blocks not kept; a replay is `type:user` +
   `isReplay:true` + no parent + text; replays match writes by COUNT (I-1, I-2).
2. Stdin is written by a `launch-input` writer thread fed by a channel; EPIPE marks the input
   broken and later sends are refused (I-3).
3. The daemon follows its own `stream.jsonl` with the same `StreamTail` the viewer uses; the
   capture pump and `claude_argv` are untouched (I-4, C-03).
4. `StreamTail` reads at most 8 MiB per poll; the viewer loop re-polls without sleeping while
   batches are non-empty. An overlong line yields one fixed note and the cursor resumes after
   its newline.
5. Inbox: `take` handles at most 64 files per tick; names that are not request names are left
   alone; client stamps are monotonic per process; `wait_outcome` returns the first record with
   the request id.
6. `SessionLifecycle.wrote(now)` restarts a waiting `result`'s 30 s settle clock, so a message
   written after a `result` is not lost to the close.
7. `SpendEstimate.usd()` returns a high-water mark so it never decreases even when a reported
   `total_cost_usd` is below the notional pending sum; on overflow (>10,000 messages) pending
   folds into the baseline.
8. Price table values and fallbacks exactly as the plan (I-8); `utilization <= 1.0` is shown
   as a fraction x100, larger values as given.
9. Viewer: scroll anchored by absolute line index, so a scrolled-up view stays put while lines
   arrive; PgDn to the bottom does not re-enable follow (only `f`/`G` do); the soft-cap input
   accepts only digits and `.` (max 12); `b`/`i` on a non-live run show `run is not live`.
10. `task budget` validates against the hard ceiling client-side (so it can name it) and the
    daemon validates again; `--clear` wins over a stray `--soft`.
11. `task attach` refreshes the run state about once a second, so it exits up to ~1 s after the
    run ends, and prints one closing `attach: <run-id> <state> - <meter line>` line.
12. The wrap-up message is delivered even past the 200-injection limit (it is the launcher's
    own and counts against nothing); it is refused only when the session is ending.

## Audit items worth a human look

- **Sandbox reads the run dir on some hosts.** The bwrap argv ro-binds `/`, then masks `/tmp`
  and `$HOME` with tmpfs. The inbox is never *writable* from the sandbox (no writable bind
  covers it), but if `dirs::data_local_dir()` resolved outside `$HOME` and `/tmp` (an unusual
  `XDG_DATA_HOME`), the run dir would be readable (not writable) inside the sandbox through
  the read-only root. This predates this task (same for `run.json`); the e2e test documents it.
- The injected-message record is written BEFORE the stdin write; if the later send fails a
  warning is recorded in `run.json` but the `delivered` record stays (it cannot be amended).

## Unverified without a paid run (verified against the fake agent only)

- The stdin message shape (C-04), the replay semantics and `--replay-user-messages` count
  behaviour (I-2), and that the real agent exits on stdin EOF after its last turn (C-06).
  The shape comes from the SDK typings; no real `claude` run was made.
- That a message queued mid-turn is replayed only at the next turn boundary (the lifecycle
  relies on it; if the real agent replays earlier the lifecycle still closes correctly).
- Real `systemd-run --scope` and the real in-sandbox bridge inheriting stdin: the fake bwrap
  shim drops the bridge, so that hop is proved by reading `spawn_bridged_child` /
  `bridge.rs` (inherits all stdio, never reads stdin), not by running it.
- Real `rate_limit_event` shapes (`utilization` scale, `resetsAt` units) and
  `--forward-subagent-text` event shapes; the price table against real `usage` numbers.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model; the new surfaces (inbox, `injections.jsonl`, viewer,
attach) are the ones T-gt1-01..11 cover.

## Self-Check: PASSED

- Files exist: `src/oneoff/{stream,session,inbox}.rs`, `src/ui/screens/oneoff_viewer.rs`,
  `.planning/todos/pending/2026-11-03-oneoff-raise-hard-ceiling-via-resume.md`,
  `.planning/todos/completed/2026-11-03-oneoff-live-stream-view.md`; the pending copy of the
  starting todo is gone.
- Commits `7e608a8`, `13fa5a7`, `a65de2d` are on master (ancestors of HEAD).
