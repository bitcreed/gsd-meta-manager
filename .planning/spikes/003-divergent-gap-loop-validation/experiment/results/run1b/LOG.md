# Driver log: s003-run1b

<!--
TOOL-WRITTEN, APPEND-ONLY. Only driverctl appends here (one O_APPEND write per event); the run dir is
read-only inside the sandbox, so neither the driver nor a worker can edit this file directly. The
driver records events with `driverctl log <event> ...`, which validates every field first.

One event = one header line, then optional indented detail lines (the worker report).
Header grammar (pipe-separated, first three fields fixed):
  - {ISO-8601 UTC} | {item-id or RUN} | {event} | key=value | key=value ...

Written on the driver's request (`driverctl log ...`):
  RUN   start        mode={interactive|headless} loop_mode={loop|fresh-per-item} session={id}
  RUN   reconcile    found={n}
  {id}  dispatched   step={...} base={sha: tool-computed} round={n} dispatch={k: tool-counted}/{budget} phase={N}
  {id}  report       status={done|partial|blocked|failed}   (+ the <=15-line report, verbatim)
  {id}  verified     done-when={met|not-met} evidence={short}
  {id}  status       state={pending|running|done|parked|failed} rule={budget|gap-rounds|divergence|no-progress|needs-human|scope|run-budget|launcher-cap} outcome={one line}
  {id}  needs-human  question={one line}
  {id}  recovered    action={done|resume|reset|park} reason={short}
  RUN   end          reason={all-done|all-parked|stop|pause|run-budget|fresh-exit}

Written by the launcher / hooks, never by the model:
  RUN   launch          n={k} model=... loop_mode=... sandbox=... unit=... caps=...
  RUN   connect         session={claude session id} source=startup      (SessionStart hook)
  RUN   disconnect      session=... reason=...                          (SessionEnd hook)
  RUN   gap-observed    phase={N} count={k} file=... source=launcher-scan
  RUN   dispatch-denied reason=...                                      (PreToolUse gate)
  RUN   cap-tripped     cap={dispatches|wall_clock_min|gap_rounds|operator-signal} used=.. limit=.. action=kill-scope
  RUN   crash-detected  previous_launch={k} evidence=...
  RUN   launch-end      rc=.. tripped=.. cost_usd=.. status=..

Current item status = the last `status` event for that id (absent = pending).
-->

- 2026-10-17T02:58:31Z | RUN | launch | n=1 | model=opus | loop_mode=loop | sandbox=bwrap | unit=gsdmm-s003-run1b-1792205911.scope | caps=dispatches=10,minutes=90,gap_rounds=2
- 2026-10-17T02:58:31Z | RUN | connect | session=1f83d5f6-8bee-4f7c-aae5-1d35893e7c87 | source=startup
- 2026-10-17T02:58:47Z | RUN | start | mode=headless | loop_mode=loop | session=1f83d5f6-8bee-4f7c-aae5-1d35893e7c87
- 2026-10-17T02:58:47Z | RUN | reconcile | found=0
- 2026-10-17T02:59:05Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=e1003bc | round=0 | dispatch=1/8 | phase=1
- 2026-10-17T03:07:39Z | RUN | disconnect | session=1f83d5f6-8bee-4f7c-aae5-1d35893e7c87 | reason=other
- 2026-10-17T03:07:39Z | RUN | launch-end | rc=0 | cost_usd=2.6119913999999995 | status=crashed
