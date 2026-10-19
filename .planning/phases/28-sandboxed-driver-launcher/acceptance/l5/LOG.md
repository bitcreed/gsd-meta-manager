# Driver log: lacc-l5

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

- 2026-10-19T22:07:47Z | RUN | launch | n=1 | mode=bwrap-scope | model=haiku
- 2026-10-19T22:07:48Z | RUN | connect | session=8d4bc25e-9d8f-4d69-9f1d-3c48946aeefd | source=startup
- 2026-10-19T22:07:52Z | RUN | start | mode=headless | loop_mode=loop | session=8d4bc25e-9d8f-4d69-9f1d-3c48946aeefd
- 2026-10-19T22:07:53Z | RUN | crash-detected | previous_launch=1 | evidence=status=running and no RUN end; lock was free
- 2026-10-19T22:07:53Z | RUN | launch | n=2 | mode=bwrap-scope | model=haiku
- 2026-10-19T22:07:54Z | RUN | connect | session=ca5dc671-5305-47c8-acc2-7b5345952b89 | source=startup
- 2026-10-19T22:07:57Z | RUN | start | mode=headless | loop_mode=loop | session=ca5dc671-5305-47c8-acc2-7b5345952b89
- 2026-10-19T22:07:58Z | RUN | end | reason=stop
- 2026-10-19T22:08:00Z | RUN | disconnect | session=ca5dc671-5305-47c8-acc2-7b5345952b89 | reason=other
- 2026-10-19T22:08:01Z | RUN | launch-end | rc=0 | status=stopped
