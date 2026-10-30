# Driver log: lacc-l1

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

- 2026-10-19T22:08:11Z | RUN | launch | n=1 | mode=bwrap-scope | model=haiku
- 2026-10-19T22:08:12Z | RUN | connect | session=f8bd9c29-c62b-4cb7-82af-4dbb4519bff6 | source=startup
- 2026-10-19T22:08:24Z | RUN | dispatch-denied | reason=dispatch cap reached (3/3). Park the item with --rule launcher-cap and end the run.
- 2026-10-19T22:08:25Z | RUN | dispatch-denied | reason=dispatch cap reached (3/3). Park the item with --rule launcher-cap and end the run.
- 2026-10-19T22:08:34Z | RUN | end | reason=stop
- 2026-10-19T22:08:39Z | RUN | disconnect | session=f8bd9c29-c62b-4cb7-82af-4dbb4519bff6 | reason=other
- 2026-10-19T22:08:39Z | RUN | launch-end | rc=0 | status=stopped
