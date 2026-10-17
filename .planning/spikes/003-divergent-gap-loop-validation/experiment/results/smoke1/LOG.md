# Driver log: s003-smoke1

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

- 2026-10-17T02:22:16Z | RUN | launch | n=1 | model=haiku | loop_mode=loop | sandbox=bwrap | unit=gsdmm-s003-smoke1-1792203736.scope | caps=dispatches=0,minutes=5,gap_rounds=2
- 2026-10-17T02:22:16Z | RUN | connect | session=49d82cb6-99fc-40d0-b567-d16de8b1f888 | source=startup
- 2026-10-17T02:22:23Z | RUN | start | mode=headless | loop_mode=loop | session=49d82cb6-99fc-40d0-b567-d16de8b1f888
- 2026-10-17T02:22:23Z | RUN | reconcile | found=0
- 2026-10-17T02:22:26Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=8ad3628 | round=0 | dispatch=1/8 | phase=1
- 2026-10-17T02:22:30Z | RUN | dispatch-denied | reason=dispatch cap reached (0/0). Park the item with --rule launcher-cap and end the run.
- 2026-10-17T02:22:35Z | I-01 | status | state=parked | rule=launcher-cap | outcome=dispatch denied by launcher cap (dispatches 0/0); item budget says 8; no worker ran
- 2026-10-17T02:22:35Z | I-01 | needs-human | question=Launcher dispatch cap is 0/0 but DIRECTION.md sets dispatches=8 for I-01 and run-budget dispatches=10. Raise the launcher dispatches cap to match, then set I-01 to eligible again, to let /gsd-plan-phase 1 run.
- 2026-10-17T02:22:35Z | RUN | end | reason=all-parked
- 2026-10-17T02:22:38Z | RUN | disconnect | session=49d82cb6-99fc-40d0-b567-d16de8b1f888 | reason=other
- 2026-10-17T02:22:40Z | RUN | launch-end | rc=0 | cost_usd=0.008837010000000001 | status=parked
