# Driver log: s003-smoke2

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

- 2026-10-17T02:22:42Z | RUN | launch | n=1 | model=haiku | loop_mode=loop | sandbox=bwrap | unit=gsdmm-s003-smoke2-1792203762.scope | caps=dispatches=0,minutes=5,gap_rounds=2
- 2026-10-17T02:22:43Z | RUN | connect | session=59258af2-4bbe-44ec-b7d2-1fafe9396351 | source=startup
- 2026-10-17T02:22:48Z | RUN | start | mode=headless | loop_mode=loop | session=59258af2-4bbe-44ec-b7d2-1fafe9396351
- 2026-10-17T02:22:48Z | RUN | reconcile | found=0
- 2026-10-17T02:22:51Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=69da1a5 | round=0 | dispatch=1/8 | phase=1
- 2026-10-17T02:22:55Z | RUN | dispatch-denied | reason=dispatch cap reached (0/0). Park the item with --rule launcher-cap and end the run.
- 2026-10-17T02:22:58Z | I-01 | status | state=parked | rule=launcher-cap | outcome=Dispatch denied by launcher: dispatch cap 0/0; no worker ran
- 2026-10-17T02:22:58Z | RUN | end | reason=all-parked
- 2026-10-17T02:23:00Z | RUN | gap-observed | phase=1 | count=1 | file=.planning/phases/01-slugify/01-VERIFICATION.md | source=launcher-scan
- 2026-10-17T02:23:01Z | RUN | disconnect | session=59258af2-4bbe-44ec-b7d2-1fafe9396351 | reason=other
- 2026-10-17T02:23:02Z | RUN | launch-end | rc=0 | cost_usd=0.00826581 | status=parked
